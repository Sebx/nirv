use async_trait::async_trait;
use std::collections::HashMap;
use tokio::net::TcpStream;
use crate::utils::{NirvResult, ProtocolError, InternalQuery, QueryResult};

/// Protocol types supported by NIRV Engine
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ProtocolType {
    PostgreSQL,
    MySQL,
    SQLite,
}

/// Connection state for protocol adapters
#[derive(Debug)]
pub struct Connection {
    pub stream: TcpStream,
    pub authenticated: bool,
    pub database: String,
    pub parameters: HashMap<String, String>,
    pub protocol_type: ProtocolType,
}

impl Connection {
    pub fn new(stream: TcpStream, protocol_type: ProtocolType) -> Self {
        Self {
            stream,
            authenticated: false,
            database: String::new(),
            parameters: HashMap::new(),
            protocol_type,
        }
    }
}

/// Authentication credentials
#[derive(Debug, Clone)]
pub struct Credentials {
    pub username: String,
    pub password: Option<String>,
    pub database: String,
    pub parameters: HashMap<String, String>,
}

impl Credentials {
    pub fn new(username: String, database: String) -> Self {
        Self {
            username,
            password: None,
            database,
            parameters: HashMap::new(),
        }
    }
    
    pub fn with_password(mut self, password: String) -> Self {
        self.password = Some(password);
        self
    }
    
    pub fn with_parameter(mut self, key: String, value: String) -> Self {
        self.parameters.insert(key, value);
        self
    }
}

/// Protocol-specific query representation
#[derive(Debug, Clone)]
pub struct ProtocolQuery {
    pub raw_query: String,
    pub parameters: Vec<String>,
    pub protocol_type: ProtocolType,
}

impl ProtocolQuery {
    pub fn new(raw_query: String, protocol_type: ProtocolType) -> Self {
        Self {
            raw_query,
            parameters: Vec::new(),
            protocol_type,
        }
    }
    
    pub fn with_parameters(mut self, parameters: Vec<String>) -> Self {
        self.parameters = parameters;
        self
    }
}

/// Protocol-specific response representation
#[derive(Debug, Clone)]
pub struct ProtocolResponse {
    pub result: QueryResult,
    pub protocol_type: ProtocolType,
    pub format: ResponseFormat,
}

/// Response format options
#[derive(Debug, Clone, PartialEq)]
pub enum ResponseFormat {
    Text,
    Binary,
}

impl ProtocolResponse {
    pub fn new(result: QueryResult, protocol_type: ProtocolType) -> Self {
        Self {
            result,
            protocol_type,
            format: ResponseFormat::Text,
        }
    }
    
    pub fn with_format(mut self, format: ResponseFormat) -> Self {
        self.format = format;
        self
    }
}

/// Main trait for database protocol adapters
#[async_trait]
pub trait ProtocolAdapter: Send + Sync {
    /// Accept a new connection and perform initial handshake
    async fn accept_connection(&self, stream: TcpStream) -> NirvResult<Connection>;
    
    /// Authenticate a connection with provided credentials
    async fn authenticate(&self, conn: &mut Connection, credentials: Credentials) -> NirvResult<()>;
    
    /// Handle a query from the client and return a response
    async fn handle_query(&self, conn: &Connection, query: ProtocolQuery) -> NirvResult<ProtocolResponse>;
    
    /// Get the protocol type this adapter handles
    fn get_protocol_type(&self) -> ProtocolType;
    
    /// Parse protocol-specific message into internal representation
    async fn parse_message(&self, conn: &Connection, data: &[u8]) -> NirvResult<ProtocolQuery>;
    
    /// Format internal query result into protocol-specific response
    async fn format_response(&self, conn: &Connection, result: QueryResult) -> NirvResult<Vec<u8>>;
    
    /// Handle connection termination
    async fn terminate_connection(&self, conn: &mut Connection) -> NirvResult<()>;
}