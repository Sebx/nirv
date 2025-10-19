use async_trait::async_trait;
use std::collections::HashMap;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

use crate::protocol::{ProtocolAdapter, ProtocolType, Connection, Credentials, ProtocolQuery, ProtocolResponse, ResponseFormat};
use crate::utils::{NirvResult, ProtocolError, QueryResult, ColumnMetadata, Row, Value, DataType};

/// PostgreSQL protocol version 3.0
const POSTGRES_PROTOCOL_VERSION: u32 = 196608; // (3 << 16) | 0

/// PostgreSQL message types
#[derive(Debug, Clone, PartialEq)]
pub enum PostgresMessageType {
    StartupMessage = 0,
    Query = b'Q' as isize,
    Terminate = b'X' as isize,
    PasswordMessage = b'p' as isize,
}

/// PostgreSQL response message types
#[derive(Debug, Clone, PartialEq)]
pub enum PostgresResponseType {
    AuthenticationOk = b'R' as isize,
    ParameterStatus = b'S' as isize,
    ReadyForQuery = b'Z' as isize,
    RowDescription = b'T' as isize,
    DataRow = b'D' as isize,
    CommandComplete = b'C' as isize,
    ErrorResponse = b'E' as isize,
}

/// PostgreSQL protocol adapter implementation
#[derive(Debug)]
pub struct PostgresProtocol {
    // Configuration and state can be added here
}

impl PostgresProtocol {
    /// Create a new PostgreSQL protocol adapter
    pub fn new() -> Self {
        Self {}
    }
    
    /// Parse a startup message from the client
    async fn parse_startup_message(&self, data: &[u8]) -> NirvResult<(u32, HashMap<String, String>)> {
        if data.len() < 8 {
            return Err(ProtocolError::InvalidMessageFormat("Startup message too short".to_string()).into());
        }
        
        // Read protocol version (4 bytes, big-endian)
        let protocol_version = u32::from_be_bytes([data[4], data[5], data[6], data[7]]);
        
        if protocol_version != POSTGRES_PROTOCOL_VERSION {
            return Err(ProtocolError::UnsupportedVersion(format!("Protocol version {} not supported", protocol_version)).into());
        }
        
        // Parse parameters (null-terminated strings)
        let mut parameters = HashMap::new();
        let mut pos = 8;
        
        while pos < data.len() - 1 {
            // Find null terminator for key
            let key_end = data[pos..].iter().position(|&b| b == 0)
                .ok_or_else(|| ProtocolError::InvalidMessageFormat("Unterminated parameter key".to_string()))?;
            
            let key = String::from_utf8_lossy(&data[pos..pos + key_end]).to_string();
            pos += key_end + 1;
            
            if pos >= data.len() {
                break;
            }
            
            // Find null terminator for value
            let value_end = data[pos..].iter().position(|&b| b == 0)
                .ok_or_else(|| ProtocolError::InvalidMessageFormat("Unterminated parameter value".to_string()))?;
            
            let value = String::from_utf8_lossy(&data[pos..pos + value_end]).to_string();
            pos += value_end + 1;
            
            parameters.insert(key, value);
        }
        
        Ok((protocol_version, parameters))
    }
    
    /// Create an authentication OK response
    fn create_auth_ok_response(&self) -> Vec<u8> {
        let mut response = Vec::new();
        response.push(b'R'); // Authentication response
        response.extend_from_slice(&8u32.to_be_bytes()); // Message length
        response.extend_from_slice(&0u32.to_be_bytes()); // Authentication OK
        response
    }
    
    /// Create a parameter status message
    fn create_parameter_status(&self, name: &str, value: &str) -> Vec<u8> {
        let mut response = Vec::new();
        response.push(b'S'); // Parameter status
        
        let content_len = name.len() + value.len() + 2; // +2 for null terminators
        response.extend_from_slice(&(content_len as u32 + 4).to_be_bytes()); // Message length
        
        response.extend_from_slice(name.as_bytes());
        response.push(0); // Null terminator
        response.extend_from_slice(value.as_bytes());
        response.push(0); // Null terminator
        
        response
    }
    
    /// Create a ready for query message
    fn create_ready_for_query(&self) -> Vec<u8> {
        let mut response = Vec::new();
        response.push(b'Z'); // Ready for query
        response.extend_from_slice(&5u32.to_be_bytes()); // Message length
        response.push(b'I'); // Transaction status: Idle
        response
    }
    
    /// Create a row description message
    fn create_row_description(&self, columns: &[ColumnMetadata]) -> Vec<u8> {
        let mut response = Vec::new();
        response.push(b'T'); // Row description
        
        // Calculate message length
        let mut content_len = 2; // Field count (2 bytes)
        for col in columns {
            content_len += col.name.len() + 1; // Name + null terminator
            content_len += 18; // Table OID (4) + Column attr (2) + Type OID (4) + Type size (2) + Type modifier (4) + Format code (2)
        }
        
        response.extend_from_slice(&(content_len as u32 + 4).to_be_bytes());
        response.extend_from_slice(&(columns.len() as u16).to_be_bytes()); // Field count
        
        for col in columns {
            response.extend_from_slice(col.name.as_bytes());
            response.push(0); // Null terminator
            response.extend_from_slice(&0u32.to_be_bytes()); // Table OID
            response.extend_from_slice(&0u16.to_be_bytes()); // Column attribute number
            
            // Map NIRV data types to PostgreSQL OIDs
            let type_oid = match col.data_type {
                DataType::Text => 25u32,      // TEXT
                DataType::Integer => 23u32,   // INT4
                DataType::Float => 701u32,    // FLOAT8
                DataType::Boolean => 16u32,   // BOOL
                DataType::Date => 1082u32,    // DATE
                DataType::DateTime => 1114u32, // TIMESTAMP
                DataType::Json => 114u32,     // JSON
                DataType::Binary => 17u32,    // BYTEA
            };
            
            response.extend_from_slice(&type_oid.to_be_bytes()); // Type OID
            response.extend_from_slice(&(-1i16).to_be_bytes()); // Type size (-1 = variable)
            response.extend_from_slice(&(-1i32).to_be_bytes()); // Type modifier
            response.extend_from_slice(&0u16.to_be_bytes()); // Format code (0 = text)
        }
        
        response
    }
    
    /// Create a data row message
    fn create_data_row(&self, row: &Row) -> Vec<u8> {
        let mut response = Vec::new();
        response.push(b'D'); // Data row
        
        // Calculate message length
        let mut content_len = 2; // Field count (2 bytes)
        for value in &row.values {
            match value {
                Value::Null => content_len += 4, // Length field only
                _ => {
                    let value_str = self.value_to_string(value);
                    content_len += 4 + value_str.len(); // Length field + data
                }
            }
        }
        
        response.extend_from_slice(&(content_len as u32 + 4).to_be_bytes());
        response.extend_from_slice(&(row.values.len() as u16).to_be_bytes()); // Field count
        
        for value in &row.values {
            match value {
                Value::Null => {
                    response.extend_from_slice(&(-1i32).to_be_bytes()); // NULL value
                }
                _ => {
                    let value_str = self.value_to_string(value);
                    response.extend_from_slice(&(value_str.len() as u32).to_be_bytes());
                    response.extend_from_slice(value_str.as_bytes());
                }
            }
        }
        
        response
    }
    
    /// Create a command complete message
    fn create_command_complete(&self, tag: &str) -> Vec<u8> {
        let mut response = Vec::new();
        response.push(b'C'); // Command complete
        
        let content_len = tag.len() + 1; // +1 for null terminator
        response.extend_from_slice(&(content_len as u32 + 4).to_be_bytes());
        response.extend_from_slice(tag.as_bytes());
        response.push(0); // Null terminator
        
        response
    }
    
    /// Create an error response message
    fn create_error_response(&self, message: &str) -> Vec<u8> {
        let mut response = Vec::new();
        response.push(b'E'); // Error response
        
        let content_len = 1 + message.len() + 1 + 1; // Severity + message + null + terminator
        response.extend_from_slice(&(content_len as u32 + 4).to_be_bytes());
        
        response.push(b'S'); // Severity field
        response.extend_from_slice(b"ERROR");
        response.push(0); // Null terminator
        
        response.push(b'M'); // Message field
        response.extend_from_slice(message.as_bytes());
        response.push(0); // Null terminator
        
        response.push(0); // End of error message
        
        response
    }
    
    /// Convert a NIRV Value to PostgreSQL string representation
    fn value_to_string(&self, value: &Value) -> String {
        match value {
            Value::Text(s) => s.clone(),
            Value::Integer(i) => i.to_string(),
            Value::Float(f) => f.to_string(),
            Value::Boolean(b) => if *b { "t".to_string() } else { "f".to_string() },
            Value::Date(d) => d.clone(),
            Value::DateTime(dt) => dt.clone(),
            Value::Json(j) => j.clone(),
            Value::Binary(b) => {
                // Simple hex encoding without external dependency
                let mut hex_string = String::with_capacity(b.len() * 2 + 2);
                hex_string.push_str("\\x");
                for byte in b {
                    hex_string.push_str(&format!("{:02x}", byte));
                }
                hex_string
            },
            Value::Null => String::new(), // Should not be called for NULL values
        }
    }
}

impl Default for PostgresProtocol {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ProtocolAdapter for PostgresProtocol {
    async fn accept_connection(&self, stream: TcpStream) -> NirvResult<Connection> {
        let connection = Connection::new(stream, ProtocolType::PostgreSQL);
        Ok(connection)
    }
    
    async fn authenticate(&self, conn: &mut Connection, credentials: Credentials) -> NirvResult<()> {
        // Read startup message
        let mut buffer = vec![0u8; 8192];
        let bytes_read = conn.stream.read(&mut buffer).await
            .map_err(|e| ProtocolError::ConnectionFailed(format!("Failed to read startup message: {}", e)))?;
        
        if bytes_read < 8 {
            return Err(ProtocolError::InvalidMessageFormat("Startup message too short".to_string()).into());
        }
        
        // Parse startup message
        let (_protocol_version, parameters) = self.parse_startup_message(&buffer[..bytes_read]).await?;
        
        // Validate credentials match startup parameters
        if let Some(user) = parameters.get("user") {
            if user != &credentials.username {
                return Err(ProtocolError::AuthenticationFailed("Username mismatch".to_string()).into());
            }
        }
        
        if let Some(database) = parameters.get("database") {
            if database != &credentials.database {
                return Err(ProtocolError::AuthenticationFailed("Database mismatch".to_string()).into());
            }
        }
        
        // Send authentication OK
        let auth_response = self.create_auth_ok_response();
        conn.stream.write_all(&auth_response).await
            .map_err(|e| ProtocolError::ConnectionFailed(format!("Failed to send auth response: {}", e)))?;
        
        // Send parameter status messages
        let param_status = self.create_parameter_status("server_version", "13.0 (NIRV Engine)");
        conn.stream.write_all(&param_status).await
            .map_err(|e| ProtocolError::ConnectionFailed(format!("Failed to send parameter status: {}", e)))?;
        
        let encoding_status = self.create_parameter_status("client_encoding", "UTF8");
        conn.stream.write_all(&encoding_status).await
            .map_err(|e| ProtocolError::ConnectionFailed(format!("Failed to send encoding status: {}", e)))?;
        
        // Send ready for query
        let ready_response = self.create_ready_for_query();
        conn.stream.write_all(&ready_response).await
            .map_err(|e| ProtocolError::ConnectionFailed(format!("Failed to send ready response: {}", e)))?;
        
        // Update connection state
        conn.authenticated = true;
        conn.database = credentials.database;
        conn.parameters = parameters;
        
        Ok(())
    }
    
    async fn handle_query(&self, _conn: &Connection, _query: ProtocolQuery) -> NirvResult<ProtocolResponse> {
        // For now, create a mock response
        // In the full implementation, this would parse the query and execute it
        let columns = vec![
            ColumnMetadata {
                name: "id".to_string(),
                data_type: DataType::Integer,
                nullable: false,
            },
            ColumnMetadata {
                name: "name".to_string(),
                data_type: DataType::Text,
                nullable: true,
            },
        ];
        
        let rows = vec![
            Row::new(vec![Value::Integer(1), Value::Text("Test User".to_string())]),
            Row::new(vec![Value::Integer(2), Value::Text("Another User".to_string())]),
        ];
        
        let result = QueryResult {
            columns,
            rows,
            affected_rows: Some(2),
            execution_time: std::time::Duration::from_millis(10),
        };
        
        Ok(ProtocolResponse::new(result, ProtocolType::PostgreSQL))
    }
    
    fn get_protocol_type(&self) -> ProtocolType {
        ProtocolType::PostgreSQL
    }
    
    async fn parse_message(&self, _conn: &Connection, data: &[u8]) -> NirvResult<ProtocolQuery> {
        if data.is_empty() {
            return Err(ProtocolError::InvalidMessageFormat("Empty message".to_string()).into());
        }
        
        let message_type = data[0];
        
        match message_type {
            b'Q' => {
                // Query message
                if data.len() < 5 {
                    return Err(ProtocolError::InvalidMessageFormat("Query message too short".to_string()).into());
                }
                
                // Skip message type (1 byte) and length (4 bytes)
                let query_data = &data[5..];
                
                // Find null terminator
                let query_end = query_data.iter().position(|&b| b == 0)
                    .unwrap_or(query_data.len());
                
                let query_string = String::from_utf8_lossy(&query_data[..query_end]).to_string();
                
                Ok(ProtocolQuery::new(query_string, ProtocolType::PostgreSQL))
            }
            b'X' => {
                // Terminate message
                Ok(ProtocolQuery::new("TERMINATE".to_string(), ProtocolType::PostgreSQL))
            }
            _ => {
                Err(ProtocolError::InvalidMessageFormat(format!("Unknown message type: {}", message_type)).into())
            }
        }
    }
    
    async fn format_response(&self, _conn: &Connection, result: QueryResult) -> NirvResult<Vec<u8>> {
        let mut response = Vec::new();
        
        // Send row description
        let row_desc = self.create_row_description(&result.columns);
        response.extend_from_slice(&row_desc);
        
        // Send data rows
        for row in &result.rows {
            let data_row = self.create_data_row(row);
            response.extend_from_slice(&data_row);
        }
        
        // Send command complete
        let tag = format!("SELECT {}", result.rows.len());
        let cmd_complete = self.create_command_complete(&tag);
        response.extend_from_slice(&cmd_complete);
        
        // Send ready for query
        let ready = self.create_ready_for_query();
        response.extend_from_slice(&ready);
        
        Ok(response)
    }
    
    async fn terminate_connection(&self, conn: &mut Connection) -> NirvResult<()> {
        conn.stream.shutdown().await
            .map_err(|_e| ProtocolError::ConnectionClosed)?;
        Ok(())
    }
}