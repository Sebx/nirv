use async_trait::async_trait;
use std::collections::HashMap;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

use crate::protocol::{ProtocolAdapter, ProtocolType, Connection, Credentials, ProtocolQuery, ProtocolResponse, ResponseFormat};
use crate::utils::{NirvResult, ProtocolError, QueryResult, ColumnMetadata, Row, Value, DataType};

/// SQLite connection flags
const SQLITE_OPEN_READONLY: u32 = 0x00000001;
const SQLITE_OPEN_READWRITE: u32 = 0x00000002;
const SQLITE_OPEN_CREATE: u32 = 0x00000004;
const SQLITE_OPEN_URI: u32 = 0x00000040;
const SQLITE_OPEN_MEMORY: u32 = 0x00000080;

/// SQLite result codes
const SQLITE_OK: u32 = 0;
const SQLITE_ERROR: u32 = 1;
const SQLITE_BUSY: u32 = 5;
const SQLITE_NOMEM: u32 = 7;
const SQLITE_READONLY: u32 = 8;
const SQLITE_MISUSE: u32 = 21;

/// SQLite data types
#[derive(Debug, Clone, PartialEq)]
pub enum SQLiteDataType {
    Null = 0,
    Integer = 1,
    Real = 2,
    Text = 3,
    Blob = 4,
}

/// SQLite command types for the simplified protocol
#[derive(Debug, Clone, PartialEq)]
pub enum SQLiteCommand {
    Connect,
    Query,
    Prepare,
    Execute,
    Close,
}

/// SQLite protocol adapter implementation
/// 
/// Note: SQLite doesn't have a traditional network protocol like PostgreSQL or MySQL.
/// This implementation provides a simplified protocol interface that can work with
/// SQLite clients through file-based connections and basic query execution.
#[derive(Debug)]
pub struct SQLiteProtocolAdapter {
    database_path: String,
    connection_flags: u32,
    prepared_statements: HashMap<u32, String>,
    next_statement_id: u32,
}

impl SQLiteProtocolAdapter {
    /// Create a new SQLite protocol adapter
    pub fn new() -> Self {
        Self {
            database_path: ":memory:".to_string(),
            connection_flags: SQLITE_OPEN_READWRITE | SQLITE_OPEN_CREATE,
            prepared_statements: HashMap::new(),
            next_statement_id: 1,
        }
    }
    
    /// Create SQLite protocol adapter with specific database path
    pub fn with_database_path(database_path: String) -> Self {
        let flags = if database_path == ":memory:" || database_path.is_empty() {
            SQLITE_OPEN_READWRITE | SQLITE_OPEN_CREATE | SQLITE_OPEN_MEMORY
        } else {
            SQLITE_OPEN_READWRITE | SQLITE_OPEN_CREATE
        };
        
        Self {
            database_path,
            connection_flags: flags,
            prepared_statements: HashMap::new(),
            next_statement_id: 1,
        }
    }
    
    /// Parse SQLite connection request
    fn parse_connection_request(&self, data: &[u8]) -> NirvResult<(String, u32)> {
        if data.len() < 8 {
            return Err(ProtocolError::InvalidMessageFormat("Connection request too short".to_string()).into());
        }
        
        // Simple protocol: 4 bytes for flags, then null-terminated database path
        let flags = u32::from_le_bytes([data[0], data[1], data[2], data[3]]);
        
        // Find null terminator for database path
        let path_start = 4;
        let path_end = data[path_start..].iter().position(|&b| b == 0)
            .map(|pos| path_start + pos)
            .unwrap_or(data.len());
        
        let database_path = String::from_utf8_lossy(&data[path_start..path_end]).to_string();
        
        Ok((database_path, flags))
    }
    
    /// Create SQLite OK response
    fn create_ok_response(&self, changes: u32, last_insert_rowid: i64) -> Vec<u8> {
        let mut response = Vec::new();
        
        // Response type (1 byte): 0 = OK
        response.push(0);
        
        // Result code (4 bytes)
        response.extend_from_slice(&SQLITE_OK.to_le_bytes());
        
        // Changes (4 bytes)
        response.extend_from_slice(&changes.to_le_bytes());
        
        // Last insert rowid (8 bytes)
        response.extend_from_slice(&last_insert_rowid.to_le_bytes());
        
        response
    }
    
    /// Create SQLite error response
    fn create_error_response(&self, error_code: u32, message: &str) -> Vec<u8> {
        let mut response = Vec::new();
        
        // Response type (1 byte): 1 = Error
        response.push(1);
        
        // Error code (4 bytes)
        response.extend_from_slice(&error_code.to_le_bytes());
        
        // Message length (4 bytes)
        response.extend_from_slice(&(message.len() as u32).to_le_bytes());
        
        // Message
        response.extend_from_slice(message.as_bytes());
        
        response
    }
    
    /// Create SQLite row response
    fn create_row_response(&self, columns: &[ColumnMetadata], rows: &[Row]) -> Vec<u8> {
        let mut response = Vec::new();
        
        // Response type (1 byte): 2 = Rows
        response.push(2);
        
        // Column count (4 bytes)
        response.extend_from_slice(&(columns.len() as u32).to_le_bytes());
        
        // Column definitions
        for column in columns {
            // Column name length (4 bytes)
            response.extend_from_slice(&(column.name.len() as u32).to_le_bytes());
            
            // Column name
            response.extend_from_slice(column.name.as_bytes());
            
            // Column type
            let sqlite_type = self.nirv_type_to_sqlite_type(&column.data_type);
            response.push(sqlite_type as u8);
            
            // Nullable flag
            response.push(if column.nullable { 1 } else { 0 });
        }
        
        // Row count (4 bytes)
        response.extend_from_slice(&(rows.len() as u32).to_le_bytes());
        
        // Row data
        for row in rows {
            for value in &row.values {
                match value {
                    Value::Null => {
                        response.push(SQLiteDataType::Null as u8);
                        response.extend_from_slice(&0u32.to_le_bytes()); // No data length
                    }
                    Value::Integer(i) => {
                        response.push(SQLiteDataType::Integer as u8);
                        response.extend_from_slice(&8u32.to_le_bytes()); // 8 bytes for i64
                        response.extend_from_slice(&i.to_le_bytes());
                    }
                    Value::Float(f) => {
                        response.push(SQLiteDataType::Real as u8);
                        response.extend_from_slice(&8u32.to_le_bytes()); // 8 bytes for f64
                        response.extend_from_slice(&f.to_le_bytes());
                    }
                    Value::Text(s) => {
                        response.push(SQLiteDataType::Text as u8);
                        response.extend_from_slice(&(s.len() as u32).to_le_bytes());
                        response.extend_from_slice(s.as_bytes());
                    }
                    Value::Binary(b) => {
                        response.push(SQLiteDataType::Blob as u8);
                        response.extend_from_slice(&(b.len() as u32).to_le_bytes());
                        response.extend_from_slice(b);
                    }
                    Value::Boolean(b) => {
                        response.push(SQLiteDataType::Integer as u8);
                        response.extend_from_slice(&8u32.to_le_bytes());
                        let int_val = if *b { 1i64 } else { 0i64 };
                        response.extend_from_slice(&int_val.to_le_bytes());
                    }
                    Value::Date(d) | Value::DateTime(d) => {
                        response.push(SQLiteDataType::Text as u8);
                        response.extend_from_slice(&(d.len() as u32).to_le_bytes());
                        response.extend_from_slice(d.as_bytes());
                    }
                    Value::Json(j) => {
                        response.push(SQLiteDataType::Text as u8);
                        response.extend_from_slice(&(j.len() as u32).to_le_bytes());
                        response.extend_from_slice(j.as_bytes());
                    }
                }
            }
        }
        
        response
    }
    
    /// Convert NIRV data type to SQLite data type
    fn nirv_type_to_sqlite_type(&self, data_type: &DataType) -> SQLiteDataType {
        match data_type {
            DataType::Text => SQLiteDataType::Text,
            DataType::Integer => SQLiteDataType::Integer,
            DataType::Float => SQLiteDataType::Real,
            DataType::Boolean => SQLiteDataType::Integer,
            DataType::Date => SQLiteDataType::Text,
            DataType::DateTime => SQLiteDataType::Text,
            DataType::Json => SQLiteDataType::Text,
            DataType::Binary => SQLiteDataType::Blob,
        }
    }
    
    /// Parse SQLite command from message
    fn parse_command(&self, data: &[u8]) -> NirvResult<(SQLiteCommand, Vec<u8>)> {
        if data.is_empty() {
            return Err(ProtocolError::InvalidMessageFormat("Empty command".to_string()).into());
        }
        
        let command_byte = data[0];
        let command_data = if data.len() > 1 { &data[1..] } else { &[] };
        
        let command = match command_byte {
            0 => SQLiteCommand::Connect,
            1 => SQLiteCommand::Query,
            2 => SQLiteCommand::Prepare,
            3 => SQLiteCommand::Execute,
            4 => SQLiteCommand::Close,
            _ => return Err(ProtocolError::UnsupportedFeature(format!("Unknown SQLite command: {}", command_byte)).into()),
        };
        
        Ok((command, command_data.to_vec()))
    }
    
    /// Handle SQLite-specific SQL functions and syntax
    fn process_sqlite_sql(&self, sql: &str) -> String {
        let mut processed_sql = sql.to_string();
        
        // Handle SQLite-specific functions that might need translation
        // For now, we'll pass through most SQL as-is since NIRV handles the source() function
        
        // Handle common SQLite functions
        processed_sql = processed_sql.replace("datetime('now')", "CURRENT_TIMESTAMP");
        processed_sql = processed_sql.replace("date('now')", "CURRENT_DATE");
        processed_sql = processed_sql.replace("time('now')", "CURRENT_TIME");
        
        // SQLite uses different syntax for some operations, but we'll keep it compatible
        processed_sql
    }
    
    /// Validate SQLite connection flags
    fn validate_connection_flags(&self, flags: u32) -> NirvResult<()> {
        // Check for conflicting flags
        if (flags & SQLITE_OPEN_READONLY) != 0 && (flags & SQLITE_OPEN_READWRITE) != 0 {
            return Err(ProtocolError::InvalidMessageFormat("Cannot specify both READONLY and READWRITE flags".to_string()).into());
        }
        
        // Ensure at least one access mode is specified
        if (flags & (SQLITE_OPEN_READONLY | SQLITE_OPEN_READWRITE)) == 0 {
            return Err(ProtocolError::InvalidMessageFormat("Must specify either READONLY or READWRITE flag".to_string()).into());
        }
        
        Ok(())
    }
}

impl Default for SQLiteProtocolAdapter {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ProtocolAdapter for SQLiteProtocolAdapter {
    async fn accept_connection(&self, stream: TcpStream) -> NirvResult<Connection> {
        let connection = Connection::new(stream, ProtocolType::SQLite);
        Ok(connection)
    }
    
    async fn authenticate(&self, conn: &mut Connection, credentials: Credentials) -> NirvResult<()> {
        // SQLite doesn't have traditional authentication, but we can simulate it
        // for compatibility with the NIRV protocol interface
        
        // Read connection request if present
        let mut buffer = vec![0u8; 1024];
        let bytes_read = match conn.stream.read(&mut buffer).await {
            Ok(n) => n,
            Err(_) => {
                // No connection request, use default settings
                conn.authenticated = true;
                conn.database = credentials.database.clone();
                return Ok(());
            }
        };
        
        if bytes_read > 0 {
            // Parse connection request
            let (database_path, flags) = self.parse_connection_request(&buffer[..bytes_read])?;
            
            // Validate flags
            self.validate_connection_flags(flags)?;
            
            // Set connection parameters
            conn.database = if database_path.is_empty() { 
                credentials.database 
            } else { 
                database_path 
            };
            
            conn.parameters.insert("flags".to_string(), flags.to_string());
            
            // Send OK response
            let ok_response = self.create_ok_response(0, 0);
            conn.stream.write_all(&ok_response).await
                .map_err(|e| ProtocolError::ConnectionFailed(format!("Failed to send OK response: {}", e)))?;
        }
        
        conn.authenticated = true;
        Ok(())
    }
    
    async fn handle_query(&self, _conn: &Connection, _query: ProtocolQuery) -> NirvResult<ProtocolResponse> {
        // Create a mock response for now
        // In the full implementation, this would execute the query through the engine
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
            Row::new(vec![Value::Integer(1), Value::Text("SQLite Test User".to_string())]),
            Row::new(vec![Value::Integer(2), Value::Text("Another SQLite User".to_string())]),
        ];
        
        let result = QueryResult {
            columns,
            rows,
            affected_rows: Some(2),
            execution_time: std::time::Duration::from_millis(5),
        };
        
        Ok(ProtocolResponse::new(result, ProtocolType::SQLite))
    }
    
    fn get_protocol_type(&self) -> ProtocolType {
        ProtocolType::SQLite
    }
    
    async fn parse_message(&self, _conn: &Connection, data: &[u8]) -> NirvResult<ProtocolQuery> {
        let (command, command_data) = self.parse_command(data)?;
        
        match command {
            SQLiteCommand::Connect => {
                Ok(ProtocolQuery::new("CONNECT".to_string(), ProtocolType::SQLite))
            }
            SQLiteCommand::Query => {
                let sql = String::from_utf8_lossy(&command_data).to_string();
                let processed_sql = self.process_sqlite_sql(&sql);
                Ok(ProtocolQuery::new(processed_sql, ProtocolType::SQLite))
            }
            SQLiteCommand::Prepare => {
                let sql = String::from_utf8_lossy(&command_data).to_string();
                let processed_sql = self.process_sqlite_sql(&sql);
                Ok(ProtocolQuery::new(format!("PREPARE {}", processed_sql), ProtocolType::SQLite))
            }
            SQLiteCommand::Execute => {
                // Parse statement ID and parameters
                if command_data.len() < 4 {
                    return Err(ProtocolError::InvalidMessageFormat("Execute command missing statement ID".to_string()).into());
                }
                
                let statement_id = u32::from_le_bytes([command_data[0], command_data[1], command_data[2], command_data[3]]);
                Ok(ProtocolQuery::new(format!("EXECUTE {}", statement_id), ProtocolType::SQLite))
            }
            SQLiteCommand::Close => {
                Ok(ProtocolQuery::new("CLOSE".to_string(), ProtocolType::SQLite))
            }
        }
    }
    
    async fn format_response(&self, _conn: &Connection, result: QueryResult) -> NirvResult<Vec<u8>> {
        if result.columns.is_empty() {
            // Non-SELECT query - return OK response
            let ok_response = self.create_ok_response(result.affected_rows.unwrap_or(0) as u32, 0);
            Ok(ok_response)
        } else {
            // SELECT query - return row data
            let row_response = self.create_row_response(&result.columns, &result.rows);
            Ok(row_response)
        }
    }
    
    async fn terminate_connection(&self, conn: &mut Connection) -> NirvResult<()> {
        // Send close acknowledgment if possible
        let close_response = self.create_ok_response(0, 0);
        let _ = conn.stream.write_all(&close_response).await;
        
        conn.stream.shutdown().await
            .map_err(|_e| ProtocolError::ConnectionClosed)?;
        Ok(())
    }
}