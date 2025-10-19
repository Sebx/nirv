use async_trait::async_trait;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

use crate::protocol::{ProtocolAdapter, ProtocolType, Connection, Credentials, ProtocolQuery, ProtocolResponse};
use crate::utils::{NirvResult, ProtocolError, QueryResult, ColumnMetadata, Row, Value, DataType};

/// MySQL protocol version
const MYSQL_PROTOCOL_VERSION: u8 = 10;

/// MySQL server capabilities flags
const CLIENT_LONG_PASSWORD: u32 = 0x00000001;
const CLIENT_FOUND_ROWS: u32 = 0x00000002;
const CLIENT_LONG_FLAG: u32 = 0x00000004;
const CLIENT_CONNECT_WITH_DB: u32 = 0x00000008;
const CLIENT_NO_SCHEMA: u32 = 0x00000010;
const CLIENT_COMPRESS: u32 = 0x00000020;
const CLIENT_ODBC: u32 = 0x00000040;
const CLIENT_LOCAL_FILES: u32 = 0x00000080;
const CLIENT_IGNORE_SPACE: u32 = 0x00000100;
const CLIENT_PROTOCOL_41: u32 = 0x00000200;
const CLIENT_INTERACTIVE: u32 = 0x00000400;
const CLIENT_SSL: u32 = 0x00000800;
const CLIENT_IGNORE_SIGPIPE: u32 = 0x00001000;
const CLIENT_TRANSACTIONS: u32 = 0x00002000;
const CLIENT_RESERVED: u32 = 0x00004000;
const CLIENT_SECURE_CONNECTION: u32 = 0x00008000;
const CLIENT_MULTI_STATEMENTS: u32 = 0x00010000;
const CLIENT_MULTI_RESULTS: u32 = 0x00020000;

/// MySQL command types
#[derive(Debug, Clone, PartialEq)]
pub enum MySQLCommand {
    Sleep = 0x00,
    Quit = 0x01,
    InitDB = 0x02,
    Query = 0x03,
    FieldList = 0x04,
    CreateDB = 0x05,
    DropDB = 0x06,
    Refresh = 0x07,
    Shutdown = 0x08,
    Statistics = 0x09,
    ProcessInfo = 0x0a,
    Connect = 0x0b,
    ProcessKill = 0x0c,
    Debug = 0x0d,
    Ping = 0x0e,
    Time = 0x0f,
    DelayedInsert = 0x10,
    ChangeUser = 0x11,
    BinlogDump = 0x12,
    TableDump = 0x13,
    ConnectOut = 0x14,
    RegisterSlave = 0x15,
    StmtPrepare = 0x16,
    StmtExecute = 0x17,
    StmtSendLongData = 0x18,
    StmtClose = 0x19,
    StmtReset = 0x1a,
    SetOption = 0x1b,
    StmtFetch = 0x1c,
}

/// MySQL field types
#[derive(Debug, Clone, PartialEq)]
pub enum MySQLFieldType {
    Decimal = 0x00,
    Tiny = 0x01,
    Short = 0x02,
    Long = 0x03,
    Float = 0x04,
    Double = 0x05,
    Null = 0x06,
    Timestamp = 0x07,
    LongLong = 0x08,
    Int24 = 0x09,
    Date = 0x0a,
    Time = 0x0b,
    DateTime = 0x0c,
    Year = 0x0d,
    NewDate = 0x0e,
    VarChar = 0x0f,
    Bit = 0x10,
    NewDecimal = 0xf6,
    Enum = 0xf7,
    Set = 0xf8,
    TinyBlob = 0xf9,
    MediumBlob = 0xfa,
    LongBlob = 0xfb,
    Blob = 0xfc,
    VarString = 0xfd,
    String = 0xfe,
    Geometry = 0xff,
}

/// MySQL protocol adapter implementation
#[derive(Debug)]
pub struct MySQLProtocolAdapter {
    server_version: String,
    connection_id: u32,
    capabilities: u32,
}

impl MySQLProtocolAdapter {
    /// Create a new MySQL protocol adapter
    pub fn new() -> Self {
        Self {
            server_version: "8.0.0-NIRV".to_string(),
            connection_id: 1,
            capabilities: CLIENT_LONG_PASSWORD
                | CLIENT_FOUND_ROWS
                | CLIENT_LONG_FLAG
                | CLIENT_CONNECT_WITH_DB
                | CLIENT_NO_SCHEMA
                | CLIENT_PROTOCOL_41
                | CLIENT_TRANSACTIONS
                | CLIENT_SECURE_CONNECTION
                | CLIENT_MULTI_STATEMENTS
                | CLIENT_MULTI_RESULTS,
        }
    }
    
    /// Create initial handshake packet
    fn create_handshake_packet(&self) -> Vec<u8> {
        let mut packet = Vec::new();
        
        // Protocol version
        packet.push(MYSQL_PROTOCOL_VERSION);
        
        // Server version (null-terminated)
        packet.extend_from_slice(self.server_version.as_bytes());
        packet.push(0);
        
        // Connection ID (4 bytes, little-endian)
        packet.extend_from_slice(&self.connection_id.to_le_bytes());
        
        // Auth plugin data part 1 (8 bytes)
        packet.extend_from_slice(b"12345678");
        
        // Filler (1 byte)
        packet.push(0);
        
        // Capability flags lower 2 bytes
        packet.extend_from_slice(&(self.capabilities as u16).to_le_bytes());
        
        // Character set (1 byte) - UTF-8
        packet.push(0x21);
        
        // Status flags (2 bytes)
        packet.extend_from_slice(&0u16.to_le_bytes());
        
        // Capability flags upper 2 bytes
        packet.extend_from_slice(&((self.capabilities >> 16) as u16).to_le_bytes());
        
        // Auth plugin data length (1 byte)
        packet.push(21);
        
        // Reserved (10 bytes)
        packet.extend_from_slice(&[0; 10]);
        
        // Auth plugin data part 2 (12 bytes + null terminator)
        packet.extend_from_slice(b"123456789012");
        packet.push(0);
        
        // Auth plugin name (null-terminated)
        packet.extend_from_slice(b"mysql_native_password");
        packet.push(0);
        
        self.wrap_packet(&packet, 0)
    }
    
    /// Wrap data in MySQL packet format
    fn wrap_packet(&self, data: &[u8], sequence_id: u8) -> Vec<u8> {
        let mut packet = Vec::new();
        
        // Packet length (3 bytes, little-endian)
        let length = data.len() as u32;
        packet.push((length & 0xff) as u8);
        packet.push(((length >> 8) & 0xff) as u8);
        packet.push(((length >> 16) & 0xff) as u8);
        
        // Sequence ID (1 byte)
        packet.push(sequence_id);
        
        // Packet data
        packet.extend_from_slice(data);
        
        packet
    }
    
    /// Parse handshake response from client
    fn parse_handshake_response(&self, data: &[u8]) -> NirvResult<(String, String, String)> {
        if data.len() < 32 {
            return Err(ProtocolError::InvalidMessageFormat("Handshake response too short".to_string()).into());
        }
        
        let mut pos = 4; // Skip packet header
        
        // Client capabilities (4 bytes)
        let _client_capabilities = u32::from_le_bytes([data[pos], data[pos + 1], data[pos + 2], data[pos + 3]]);
        pos += 4;
        
        // Max packet size (4 bytes)
        let _max_packet_size = u32::from_le_bytes([data[pos], data[pos + 1], data[pos + 2], data[pos + 3]]);
        pos += 4;
        
        // Character set (1 byte)
        let _charset = data[pos];
        pos += 1;
        
        // Reserved (23 bytes)
        pos += 23;
        
        // Username (null-terminated)
        let username_start = pos;
        while pos < data.len() && data[pos] != 0 {
            pos += 1;
        }
        let username = String::from_utf8_lossy(&data[username_start..pos]).to_string();
        pos += 1; // Skip null terminator
        
        // Password length (1 byte)
        if pos >= data.len() {
            return Err(ProtocolError::InvalidMessageFormat("Missing password length".to_string()).into());
        }
        let password_len = data[pos] as usize;
        pos += 1;
        
        // Password (password_len bytes)
        let password = if password_len > 0 {
            if pos + password_len > data.len() {
                return Err(ProtocolError::InvalidMessageFormat("Password data truncated".to_string()).into());
            }
            String::from_utf8_lossy(&data[pos..pos + password_len]).to_string()
        } else {
            String::new()
        };
        pos += password_len;
        
        // Database (null-terminated, optional)
        let database = if pos < data.len() {
            let db_start = pos;
            while pos < data.len() && data[pos] != 0 {
                pos += 1;
            }
            String::from_utf8_lossy(&data[db_start..pos]).to_string()
        } else {
            String::new()
        };
        
        Ok((username, password, database))
    }
    
    /// Create OK packet
    fn create_ok_packet(&self, affected_rows: u64, last_insert_id: u64) -> Vec<u8> {
        let mut packet = Vec::new();
        
        // OK packet header
        packet.push(0x00);
        
        // Affected rows (length-encoded integer)
        self.write_length_encoded_integer(&mut packet, affected_rows);
        
        // Last insert ID (length-encoded integer)
        self.write_length_encoded_integer(&mut packet, last_insert_id);
        
        // Status flags (2 bytes)
        packet.extend_from_slice(&0u16.to_le_bytes());
        
        // Warnings (2 bytes)
        packet.extend_from_slice(&0u16.to_le_bytes());
        
        self.wrap_packet(&packet, 2)
    }
    
    /// Create error packet
    fn create_error_packet(&self, error_code: u16, message: &str) -> Vec<u8> {
        let mut packet = Vec::new();
        
        // Error packet header
        packet.push(0xff);
        
        // Error code (2 bytes, little-endian)
        packet.extend_from_slice(&error_code.to_le_bytes());
        
        // SQL state marker
        packet.push(b'#');
        
        // SQL state (5 bytes)
        packet.extend_from_slice(b"HY000");
        
        // Error message
        packet.extend_from_slice(message.as_bytes());
        
        self.wrap_packet(&packet, 1)
    }
    
    /// Create result set header
    fn create_result_set_header(&self, column_count: usize) -> Vec<u8> {
        let mut packet = Vec::new();
        
        // Column count (length-encoded integer)
        self.write_length_encoded_integer(&mut packet, column_count as u64);
        
        self.wrap_packet(&packet, 1)
    }
    
    /// Create column definition packet
    fn create_column_definition(&self, column: &ColumnMetadata, sequence_id: u8) -> Vec<u8> {
        let mut packet = Vec::new();
        
        // Catalog (length-encoded string)
        self.write_length_encoded_string(&mut packet, "def");
        
        // Schema (length-encoded string)
        self.write_length_encoded_string(&mut packet, "");
        
        // Table (length-encoded string)
        self.write_length_encoded_string(&mut packet, "");
        
        // Original table (length-encoded string)
        self.write_length_encoded_string(&mut packet, "");
        
        // Name (length-encoded string)
        self.write_length_encoded_string(&mut packet, &column.name);
        
        // Original name (length-encoded string)
        self.write_length_encoded_string(&mut packet, &column.name);
        
        // Length of fixed-length fields (1 byte)
        packet.push(0x0c);
        
        // Character set (2 bytes)
        packet.extend_from_slice(&0x21u16.to_le_bytes()); // UTF-8
        
        // Column length (4 bytes)
        packet.extend_from_slice(&0u32.to_le_bytes());
        
        // Column type
        let field_type = self.nirv_type_to_mysql_type(&column.data_type);
        packet.push(field_type as u8);
        
        // Flags (2 bytes)
        let flags: u16 = if column.nullable { 0 } else { 1 }; // NOT_NULL flag
        packet.extend_from_slice(&flags.to_le_bytes());
        
        // Decimals (1 byte)
        packet.push(0);
        
        // Reserved (2 bytes)
        packet.extend_from_slice(&0u16.to_le_bytes());
        
        self.wrap_packet(&packet, sequence_id)
    }
    
    /// Create EOF packet
    fn create_eof_packet(&self, sequence_id: u8) -> Vec<u8> {
        let mut packet = Vec::new();
        
        // EOF packet header
        packet.push(0xfe);
        
        // Warnings (2 bytes)
        packet.extend_from_slice(&0u16.to_le_bytes());
        
        // Status flags (2 bytes)
        packet.extend_from_slice(&0u16.to_le_bytes());
        
        self.wrap_packet(&packet, sequence_id)
    }
    
    /// Create row data packet
    fn create_row_packet(&self, row: &Row, sequence_id: u8) -> Vec<u8> {
        let mut packet = Vec::new();
        
        for value in &row.values {
            match value {
                Value::Null => {
                    packet.push(0xfb); // NULL value
                }
                _ => {
                    let value_str = self.value_to_string(value);
                    self.write_length_encoded_string(&mut packet, &value_str);
                }
            }
        }
        
        self.wrap_packet(&packet, sequence_id)
    }
    
    /// Write length-encoded integer
    fn write_length_encoded_integer(&self, buffer: &mut Vec<u8>, value: u64) {
        if value < 251 {
            buffer.push(value as u8);
        } else if value < 65536 {
            buffer.push(0xfc);
            buffer.extend_from_slice(&(value as u16).to_le_bytes());
        } else if value < 16777216 {
            buffer.push(0xfd);
            buffer.push((value & 0xff) as u8);
            buffer.push(((value >> 8) & 0xff) as u8);
            buffer.push(((value >> 16) & 0xff) as u8);
        } else {
            buffer.push(0xfe);
            buffer.extend_from_slice(&value.to_le_bytes());
        }
    }
    
    /// Write length-encoded string
    fn write_length_encoded_string(&self, buffer: &mut Vec<u8>, value: &str) {
        let bytes = value.as_bytes();
        self.write_length_encoded_integer(buffer, bytes.len() as u64);
        buffer.extend_from_slice(bytes);
    }
    
    /// Convert NIRV data type to MySQL field type
    fn nirv_type_to_mysql_type(&self, data_type: &DataType) -> MySQLFieldType {
        match data_type {
            DataType::Text => MySQLFieldType::VarString,
            DataType::Integer => MySQLFieldType::LongLong,
            DataType::Float => MySQLFieldType::Double,
            DataType::Boolean => MySQLFieldType::Tiny,
            DataType::Date => MySQLFieldType::Date,
            DataType::DateTime => MySQLFieldType::DateTime,
            DataType::Json => MySQLFieldType::VarString,
            DataType::Binary => MySQLFieldType::Blob,
        }
    }
    
    /// Convert NIRV Value to MySQL string representation
    fn value_to_string(&self, value: &Value) -> String {
        match value {
            Value::Text(s) => s.clone(),
            Value::Integer(i) => i.to_string(),
            Value::Float(f) => f.to_string(),
            Value::Boolean(b) => if *b { "1".to_string() } else { "0".to_string() },
            Value::Date(d) => d.clone(),
            Value::DateTime(dt) => dt.clone(),
            Value::Json(j) => j.clone(),
            Value::Binary(b) => {
                // Simple hex encoding
                let mut hex_string = String::with_capacity(b.len() * 2);
                for byte in b {
                    hex_string.push_str(&format!("{:02x}", byte));
                }
                hex_string
            },
            Value::Null => String::new(), // Should not be called for NULL values
        }
    }
    
    /// Parse MySQL command from packet
    fn parse_command(&self, data: &[u8]) -> NirvResult<(MySQLCommand, Vec<u8>)> {
        if data.len() < 5 {
            return Err(ProtocolError::InvalidMessageFormat("Command packet too short".to_string()).into());
        }
        
        // Skip packet header (4 bytes)
        let command_byte = data[4];
        let command_data = &data[5..];
        
        let command = match command_byte {
            0x01 => MySQLCommand::Quit,
            0x02 => MySQLCommand::InitDB,
            0x03 => MySQLCommand::Query,
            0x0e => MySQLCommand::Ping,
            _ => return Err(ProtocolError::UnsupportedFeature(format!("Command {} not supported", command_byte)).into()),
        };
        
        Ok((command, command_data.to_vec()))
    }
}

impl Default for MySQLProtocolAdapter {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ProtocolAdapter for MySQLProtocolAdapter {
    async fn accept_connection(&self, stream: TcpStream) -> NirvResult<Connection> {
        let mut connection = Connection::new(stream, ProtocolType::MySQL);
        
        // Send initial handshake packet
        let handshake = self.create_handshake_packet();
        connection.stream.write_all(&handshake).await
            .map_err(|e| ProtocolError::ConnectionFailed(format!("Failed to send handshake: {}", e)))?;
        
        Ok(connection)
    }
    
    async fn authenticate(&self, conn: &mut Connection, credentials: Credentials) -> NirvResult<()> {
        // Read handshake response
        let mut buffer = vec![0u8; 8192];
        let bytes_read = conn.stream.read(&mut buffer).await
            .map_err(|e| ProtocolError::ConnectionFailed(format!("Failed to read handshake response: {}", e)))?;
        
        if bytes_read < 32 {
            return Err(ProtocolError::InvalidMessageFormat("Handshake response too short".to_string()).into());
        }
        
        // Parse handshake response
        let (username, _password, database) = self.parse_handshake_response(&buffer[..bytes_read])?;
        
        // Validate credentials
        if username != credentials.username {
            let error_packet = self.create_error_packet(1045, "Access denied for user");
            conn.stream.write_all(&error_packet).await
                .map_err(|e| ProtocolError::ConnectionFailed(format!("Failed to send error: {}", e)))?;
            return Err(ProtocolError::AuthenticationFailed("Username mismatch".to_string()).into());
        }
        
        if !database.is_empty() && database != credentials.database {
            let error_packet = self.create_error_packet(1049, "Unknown database");
            conn.stream.write_all(&error_packet).await
                .map_err(|e| ProtocolError::ConnectionFailed(format!("Failed to send error: {}", e)))?;
            return Err(ProtocolError::AuthenticationFailed("Database mismatch".to_string()).into());
        }
        
        // Send OK packet
        let ok_packet = self.create_ok_packet(0, 0);
        conn.stream.write_all(&ok_packet).await
            .map_err(|e| ProtocolError::ConnectionFailed(format!("Failed to send OK packet: {}", e)))?;
        
        // Update connection state
        conn.authenticated = true;
        conn.database = if database.is_empty() { credentials.database } else { database };
        conn.parameters.insert("user".to_string(), username);
        
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
        
        Ok(ProtocolResponse::new(result, ProtocolType::MySQL))
    }
    
    fn get_protocol_type(&self) -> ProtocolType {
        ProtocolType::MySQL
    }
    
    async fn parse_message(&self, _conn: &Connection, data: &[u8]) -> NirvResult<ProtocolQuery> {
        let (command, command_data) = self.parse_command(data)?;
        
        match command {
            MySQLCommand::Query => {
                let query_string = String::from_utf8_lossy(&command_data).to_string();
                Ok(ProtocolQuery::new(query_string, ProtocolType::MySQL))
            }
            MySQLCommand::Quit => {
                Ok(ProtocolQuery::new("QUIT".to_string(), ProtocolType::MySQL))
            }
            MySQLCommand::Ping => {
                Ok(ProtocolQuery::new("PING".to_string(), ProtocolType::MySQL))
            }
            MySQLCommand::InitDB => {
                let db_name = String::from_utf8_lossy(&command_data).to_string();
                Ok(ProtocolQuery::new(format!("USE {}", db_name), ProtocolType::MySQL))
            }
            _ => {
                Err(ProtocolError::UnsupportedFeature(format!("Command {:?} not supported", command)).into())
            }
        }
    }
    
    async fn format_response(&self, _conn: &Connection, result: QueryResult) -> NirvResult<Vec<u8>> {
        let mut response = Vec::new();
        
        if result.columns.is_empty() {
            // OK packet for non-SELECT queries
            let ok_packet = self.create_ok_packet(result.affected_rows.unwrap_or(0), 0);
            response.extend_from_slice(&ok_packet);
        } else {
            // Result set for SELECT queries
            
            // Result set header
            let header = self.create_result_set_header(result.columns.len());
            response.extend_from_slice(&header);
            
            // Column definitions
            for (i, column) in result.columns.iter().enumerate() {
                let col_def = self.create_column_definition(column, (i + 2) as u8);
                response.extend_from_slice(&col_def);
            }
            
            // EOF packet after column definitions
            let eof1 = self.create_eof_packet((result.columns.len() + 2) as u8);
            response.extend_from_slice(&eof1);
            
            // Row data
            for (i, row) in result.rows.iter().enumerate() {
                let row_packet = self.create_row_packet(row, (result.columns.len() + 3 + i) as u8);
                response.extend_from_slice(&row_packet);
            }
            
            // EOF packet after rows
            let eof2 = self.create_eof_packet((result.columns.len() + 3 + result.rows.len()) as u8);
            response.extend_from_slice(&eof2);
        }
        
        Ok(response)
    }
    
    async fn terminate_connection(&self, conn: &mut Connection) -> NirvResult<()> {
        conn.stream.shutdown().await
            .map_err(|_e| ProtocolError::ConnectionClosed)?;
        Ok(())
    }
}