use async_trait::async_trait;
use std::collections::HashMap;
use tokio::net::TcpStream;

use crate::protocol::{ProtocolAdapter, ProtocolType, Connection, Credentials, ProtocolQuery, ProtocolResponse};
use crate::utils::{NirvResult, ProtocolError, QueryResult, ColumnMetadata, Row, Value, DataType};

/// SQL Server TDS (Tabular Data Stream) protocol version
const TDS_VERSION: u32 = 0x74000004; // TDS 7.4

/// TDS packet types
#[derive(Debug, Clone, PartialEq)]
pub enum TdsPacketType {
    SqlBatch = 0x01,
    PreTds7Login = 0x02,
    Rpc = 0x03,
    TabularResult = 0x04,
    AttentionSignal = 0x06,
    BulkLoadData = 0x07,
    FederatedAuthToken = 0x08,
    TransactionManagerRequest = 0x0E,
    Tds7Login = 0x10,
    Sspi = 0x11,
    PreLogin = 0x12,
}

/// TDS token types for responses
#[derive(Debug, Clone, PartialEq)]
pub enum TdsTokenType {
    ColMetadata = 0x81,
    Row = 0xD1,
    Done = 0xFD,
    DoneInProc = 0xFF,
    DoneProc = 0xFE,
    Error = 0xAA,
    Info = 0xAB,
    LoginAck = 0xAD,
    EnvChange = 0xE3,
}

/// SQL Server data types (TDS type codes)
#[derive(Debug, Clone, PartialEq)]
pub enum TdsDataType {
    Null = 0x1F,
    Int1 = 0x30,
    Bit = 0x32,
    Int2 = 0x34,
    Int4 = 0x38,
    DatetimeN = 0x6F,
    Float8 = 0x3E,
    Money = 0x3C,
    DateTime = 0x3D,
    Float4 = 0x3B,
    Money4 = 0x7A,
    Int8 = 0x7F,
    BitN = 0x68,
    IntN = 0x26,
    FloatN = 0x6D,
    NVarChar = 0xE7,
    VarChar = 0xA7,
    Binary = 0xAD,
    VarBinary = 0xA5,
}

/// SQL Server protocol adapter implementation
#[derive(Debug)]
pub struct SqlServerProtocol {
    // Configuration and state can be added here
}

impl SqlServerProtocol {
    /// Create a new SQL Server protocol adapter
    pub fn new() -> Self {
        Self {}
    }
    
    /// Parse a TDS login packet
    pub fn parse_login_packet(&self, data: &[u8]) -> NirvResult<HashMap<String, String>> {
        if data.len() < 8 {
            return Err(ProtocolError::InvalidMessageFormat("TDS packet too short".to_string()).into());
        }
        
        // Parse TDS header
        let packet_type = data[0];
        let _status = data[1];
        let length = u16::from_be_bytes([data[2], data[3]]) as usize;
        
        if packet_type != TdsPacketType::Tds7Login as u8 {
            return Err(ProtocolError::InvalidMessageFormat(
                format!("Expected login packet, got type {}", packet_type)
            ).into());
        }
        
        if data.len() < length {
            return Err(ProtocolError::InvalidMessageFormat("Incomplete TDS packet".to_string()).into());
        }
        
        // For simplicity, return a mock parsed login
        let mut params = HashMap::new();
        params.insert("username".to_string(), "sa".to_string());
        params.insert("database".to_string(), "master".to_string());
        params.insert("application".to_string(), "NIRV Engine".to_string());
        
        Ok(params)
    }
    
    /// Parse a SQL batch packet
    pub fn parse_sql_batch(&self, data: &[u8]) -> NirvResult<String> {
        if data.is_empty() {
            return Err(ProtocolError::InvalidMessageFormat("Empty SQL batch".to_string()).into());
        }
        
        // SQL Server sends SQL text as UTF-16LE
        if data.len() % 2 != 0 {
            return Err(ProtocolError::InvalidMessageFormat("Invalid UTF-16 data length".to_string()).into());
        }
        
        let mut utf16_chars = Vec::new();
        for chunk in data.chunks_exact(2) {
            let char_code = u16::from_le_bytes([chunk[0], chunk[1]]);
            utf16_chars.push(char_code);
        }
        
        String::from_utf16(&utf16_chars)
            .map_err(|e| ProtocolError::InvalidMessageFormat(format!("Invalid UTF-16: {}", e)).into())
    }
    
    /// Create a TDS header
    fn create_tds_header(&self, packet_type: TdsPacketType, length: u16) -> Vec<u8> {
        let mut header = Vec::with_capacity(8);
        header.push(packet_type as u8);
        header.push(0x01); // Status: End of message
        header.extend_from_slice(&length.to_be_bytes());
        header.extend_from_slice(&0u16.to_be_bytes()); // SPID
        header.push(0x01); // Packet ID
        header.push(0x00); // Window
        header
    }
    
    /// Create a login acknowledgment response
    fn create_login_ack(&self) -> Vec<u8> {
        let mut response = Vec::new();
        
        // LoginAck token
        response.push(TdsTokenType::LoginAck as u8);
        
        // Token length (placeholder, will be updated)
        let length_pos = response.len();
        response.extend_from_slice(&0u16.to_le_bytes());
        
        // Interface (1 byte) - SQL Server
        response.push(0x01);
        
        // TDS version (4 bytes)
        response.extend_from_slice(&TDS_VERSION.to_le_bytes());
        
        // Program name (variable length)
        let program_name = "Microsoft SQL Server";
        response.push(program_name.len() as u8);
        response.extend_from_slice(program_name.as_bytes());
        
        // Program version (4 bytes)
        response.extend_from_slice(&0x10000000u32.to_le_bytes());
        
        // Update token length
        let token_length = (response.len() - length_pos - 2) as u16;
        response[length_pos..length_pos + 2].copy_from_slice(&token_length.to_le_bytes());
        
        response
    }
    
    /// Create an environment change token
    fn create_env_change(&self, change_type: u8, new_value: &str, old_value: &str) -> Vec<u8> {
        let mut token = Vec::new();
        
        // EnvChange token
        token.push(TdsTokenType::EnvChange as u8);
        
        // Token length (placeholder)
        let length_pos = token.len();
        token.extend_from_slice(&0u16.to_le_bytes());
        
        // Change type
        token.push(change_type);
        
        // New value
        token.push(new_value.len() as u8);
        token.extend_from_slice(new_value.as_bytes());
        
        // Old value
        token.push(old_value.len() as u8);
        token.extend_from_slice(old_value.as_bytes());
        
        // Update token length
        let token_length = (token.len() - length_pos - 2) as u16;
        token[length_pos..length_pos + 2].copy_from_slice(&token_length.to_le_bytes());
        
        token
    }
    
    /// Create column metadata token
    fn create_colmetadata(&self, columns: &[ColumnMetadata]) -> Vec<u8> {
        let mut token = Vec::new();
        
        // ColMetadata token
        token.push(TdsTokenType::ColMetadata as u8);
        
        // Column count
        token.extend_from_slice(&(columns.len() as u16).to_le_bytes());
        
        for column in columns {
            // Column metadata
            let tds_type = self.datatype_to_tds_type(&column.data_type);
            token.push(tds_type);
            
            // Type-specific metadata
            match column.data_type {
                DataType::Text => {
                    token.extend_from_slice(&0xFFFFu16.to_le_bytes()); // Max length
                    token.extend_from_slice(&0u32.to_le_bytes()); // Collation
                    token.push(0); // Collation flags
                }
                DataType::Integer => {
                    token.push(4); // Length
                }
                DataType::Float => {
                    token.push(8); // Length
                }
                DataType::Boolean => {
                    token.push(1); // Length
                }
                _ => {
                    token.push(0); // Default length
                }
            }
            
            // Column name
            let name_utf16: Vec<u16> = column.name.encode_utf16().collect();
            token.push(name_utf16.len() as u8);
            for ch in name_utf16 {
                token.extend_from_slice(&ch.to_le_bytes());
            }
        }
        
        token
    }
    
    /// Create a data row token
    fn create_row(&self, row: &Row, columns: &[ColumnMetadata]) -> Vec<u8> {
        let mut token = Vec::new();
        
        // Row token
        token.push(TdsTokenType::Row as u8);
        
        for (i, value) in row.values.iter().enumerate() {
            let _column_type = if i < columns.len() {
                &columns[i].data_type
            } else {
                &DataType::Text
            };
            
            match value {
                Value::Null => {
                    token.push(0); // NULL indicator
                }
                Value::Integer(val) => {
                    token.push(4); // Length
                    token.extend_from_slice(&(*val as i32).to_le_bytes());
                }
                Value::Float(val) => {
                    token.push(8); // Length
                    token.extend_from_slice(&val.to_le_bytes());
                }
                Value::Boolean(val) => {
                    token.push(1); // Length
                    token.push(if *val { 1 } else { 0 });
                }
                Value::Text(val) => {
                    let utf16: Vec<u16> = val.encode_utf16().collect();
                    let byte_len = utf16.len() * 2;
                    token.extend_from_slice(&(byte_len as u16).to_le_bytes());
                    for ch in utf16 {
                        token.extend_from_slice(&ch.to_le_bytes());
                    }
                }
                _ => {
                    // Convert other types to string
                    let str_val = format!("{:?}", value);
                    let utf16: Vec<u16> = str_val.encode_utf16().collect();
                    let byte_len = utf16.len() * 2;
                    token.extend_from_slice(&(byte_len as u16).to_le_bytes());
                    for ch in utf16 {
                        token.extend_from_slice(&ch.to_le_bytes());
                    }
                }
            }
        }
        
        token
    }
    
    /// Create a DONE token
    fn create_done(&self, status: u16, cur_cmd: u16, row_count: u64) -> Vec<u8> {
        let mut token = Vec::new();
        
        // Done token
        token.push(TdsTokenType::Done as u8);
        
        // Status
        token.extend_from_slice(&status.to_le_bytes());
        
        // Current command
        token.extend_from_slice(&cur_cmd.to_le_bytes());
        
        // Row count
        token.extend_from_slice(&row_count.to_le_bytes());
        
        token
    }
    
    /// Create an error response
    pub fn create_error_response(&self, error_number: u32, message: &str, severity: u8) -> Vec<u8> {
        let mut response = Vec::new();
        
        // TDS header
        let header = self.create_tds_header(TdsPacketType::TabularResult, 0);
        response.extend_from_slice(&header);
        
        // Error token
        response.push(TdsTokenType::Error as u8);
        
        // Token length (placeholder)
        let length_pos = response.len();
        response.extend_from_slice(&0u16.to_le_bytes());
        
        // Error number
        response.extend_from_slice(&error_number.to_le_bytes());
        
        // State
        response.push(1);
        
        // Severity
        response.push(severity);
        
        // Message length and text
        response.extend_from_slice(&(message.len() as u16).to_le_bytes());
        response.extend_from_slice(message.as_bytes());
        
        // Server name (empty)
        response.push(0);
        
        // Procedure name (empty)
        response.push(0);
        
        // Line number
        response.extend_from_slice(&0u32.to_le_bytes());
        
        // Update token length
        let token_length = (response.len() - length_pos - 2) as u16;
        response[length_pos..length_pos + 2].copy_from_slice(&token_length.to_le_bytes());
        
        // Update TDS header length
        let total_length = response.len() as u16;
        response[2..4].copy_from_slice(&total_length.to_be_bytes());
        
        response
    }
    
    /// Convert internal DataType to TDS type code
    fn datatype_to_tds_type(&self, data_type: &DataType) -> u8 {
        match data_type {
            DataType::Text => TdsDataType::NVarChar as u8,
            DataType::Integer => TdsDataType::IntN as u8,
            DataType::Float => TdsDataType::FloatN as u8,
            DataType::Boolean => TdsDataType::BitN as u8,
            DataType::Date => TdsDataType::DatetimeN as u8,
            DataType::DateTime => TdsDataType::DatetimeN as u8,
            DataType::Binary => TdsDataType::VarBinary as u8,
            DataType::Json => TdsDataType::NVarChar as u8,
        }
    }
    
    /// Convert Value to TDS type code
    pub fn value_to_tds_type(&self, value: &Value) -> u8 {
        match value {
            Value::Null => TdsDataType::Null as u8,
            Value::Integer(_) => TdsDataType::IntN as u8,
            Value::Float(_) => TdsDataType::FloatN as u8,
            Value::Boolean(_) => TdsDataType::BitN as u8,
            Value::Text(_) => TdsDataType::NVarChar as u8,
            Value::Date(_) => TdsDataType::DatetimeN as u8,
            Value::DateTime(_) => TdsDataType::DatetimeN as u8,
            Value::Binary(_) => TdsDataType::VarBinary as u8,
            Value::Json(_) => TdsDataType::NVarChar as u8,
        }
    }
}

impl Default for SqlServerProtocol {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ProtocolAdapter for SqlServerProtocol {
    async fn accept_connection(&self, stream: TcpStream) -> NirvResult<Connection> {
        let connection = Connection::new(stream, ProtocolType::SqlServer);
        
        // SQL Server connection setup would happen here
        // For now, just return the connection
        
        Ok(connection)
    }
    
    async fn authenticate(&self, conn: &mut Connection, credentials: Credentials) -> NirvResult<()> {
        // In a real implementation, this would validate credentials
        // For testing, we'll just mark as authenticated
        
        conn.authenticated = true;
        conn.database = credentials.database;
        conn.parameters.insert("username".to_string(), credentials.username);
        
        if let Some(password) = credentials.password {
            conn.parameters.insert("password".to_string(), password);
        }
        
        // Merge additional parameters
        for (key, value) in credentials.parameters {
            conn.parameters.insert(key, value);
        }
        
        // Send login acknowledgment
        let login_ack = self.create_login_ack();
        let env_change = self.create_env_change(1, &conn.database, "");
        
        let mut response = Vec::new();
        let header = self.create_tds_header(
            TdsPacketType::TabularResult, 
            (login_ack.len() + env_change.len()) as u16 + 8
        );
        response.extend_from_slice(&header);
        response.extend_from_slice(&login_ack);
        response.extend_from_slice(&env_change);
        
        // In a real implementation, we would write this to the stream
        // conn.stream.write_all(&response).await?;
        
        Ok(())
    }
    
    async fn handle_query(&self, conn: &Connection, _query: ProtocolQuery) -> NirvResult<ProtocolResponse> {
        if !conn.authenticated {
            return Err(ProtocolError::AuthenticationFailed("Connection not authenticated".to_string()).into());
        }
        
        // For testing, return a mock result
        let mock_result = QueryResult {
            columns: vec![
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
            ],
            rows: vec![
                Row::new(vec![Value::Integer(1), Value::Text("Test User".to_string())]),
            ],
            affected_rows: Some(1),
            execution_time: std::time::Duration::from_millis(5),
        };
        
        Ok(ProtocolResponse::new(mock_result, ProtocolType::SqlServer))
    }
    
    fn get_protocol_type(&self) -> ProtocolType {
        ProtocolType::SqlServer
    }
    
    async fn parse_message(&self, _conn: &Connection, data: &[u8]) -> NirvResult<ProtocolQuery> {
        if data.len() < 8 {
            return Err(ProtocolError::InvalidMessageFormat("TDS packet too short".to_string()).into());
        }
        
        let packet_type = data[0];
        
        match packet_type {
            x if x == TdsPacketType::SqlBatch as u8 => {
                let sql_text = self.parse_sql_batch(&data[8..])?;
                Ok(ProtocolQuery::new(sql_text, ProtocolType::SqlServer))
            }
            x if x == TdsPacketType::Tds7Login as u8 => {
                // Return a dummy query for login packets
                Ok(ProtocolQuery::new("LOGIN".to_string(), ProtocolType::SqlServer))
            }
            _ => {
                Err(ProtocolError::UnsupportedFeature(
                    format!("Unsupported TDS packet type: {}", packet_type)
                ).into())
            }
        }
    }
    
    async fn format_response(&self, _conn: &Connection, result: QueryResult) -> NirvResult<Vec<u8>> {
        let mut response = Vec::new();
        
        // Create column metadata
        let colmetadata = self.create_colmetadata(&result.columns);
        
        // Create data rows
        let mut rows_data = Vec::new();
        for row in &result.rows {
            let row_data = self.create_row(row, &result.columns);
            rows_data.extend_from_slice(&row_data);
        }
        
        // Create DONE token
        let done = self.create_done(0x0010, 0xC1, result.rows.len() as u64); // DONE_COUNT
        
        // Combine all tokens
        let mut tokens = Vec::new();
        tokens.extend_from_slice(&colmetadata);
        tokens.extend_from_slice(&rows_data);
        tokens.extend_from_slice(&done);
        
        // Create TDS header
        let header = self.create_tds_header(TdsPacketType::TabularResult, (tokens.len() + 8) as u16);
        
        response.extend_from_slice(&header);
        response.extend_from_slice(&tokens);
        
        Ok(response)
    }
    
    async fn terminate_connection(&self, conn: &mut Connection) -> NirvResult<()> {
        conn.authenticated = false;
        conn.database.clear();
        conn.parameters.clear();
        
        // In a real implementation, we would close the stream gracefully
        // conn.stream.shutdown().await?;
        
        Ok(())
    }
}