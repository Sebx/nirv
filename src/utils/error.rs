use thiserror::Error;

/// Main error type for NIRV Engine
#[derive(Debug, Error)]
pub enum NirvError {
    #[error("Protocol error: {0}")]
    Protocol(#[from] ProtocolError),
    
    #[error("Query parsing error: {0}")]
    QueryParsing(#[from] QueryParsingError),
    
    #[error("Connector error: {0}")]
    Connector(#[from] ConnectorError),
    
    #[error("Dispatcher error: {0}")]
    Dispatcher(#[from] DispatcherError),
    
    #[error("Configuration error: {0}")]
    Configuration(String),
    
    #[error("Internal error: {0}")]
    Internal(String),
}

/// Protocol-specific errors
#[derive(Debug, Error)]
pub enum ProtocolError {
    #[error("Connection failed: {0}")]
    ConnectionFailed(String),
    
    #[error("Authentication failed: {0}")]
    AuthenticationFailed(String),
    
    #[error("Invalid message format: {0}")]
    InvalidMessageFormat(String),
    
    #[error("Protocol version not supported: {0}")]
    UnsupportedVersion(String),
    
    #[error("Unsupported feature: {0}")]
    UnsupportedFeature(String),
    
    #[error("Connection closed unexpectedly")]
    ConnectionClosed,
}

/// Query parsing errors
#[derive(Debug, Error)]
pub enum QueryParsingError {
    #[error("Invalid SQL syntax: {0}")]
    InvalidSyntax(String),
    
    #[error("Unsupported SQL feature: {0}")]
    UnsupportedFeature(String),
    
    #[error("Missing source specification in query")]
    MissingSource,
    
    #[error("Invalid source format: {0}")]
    InvalidSourceFormat(String),
    
    #[error("Ambiguous column reference: {0}")]
    AmbiguousColumn(String),
}

/// Connector-specific errors
#[derive(Debug, Error)]
pub enum ConnectorError {
    #[error("Connection to backend failed: {0}")]
    ConnectionFailed(String),
    
    #[error("Query execution failed: {0}")]
    QueryExecutionFailed(String),
    
    #[error("Schema retrieval failed: {0}")]
    SchemaRetrievalFailed(String),
    
    #[error("Unsupported operation: {0}")]
    UnsupportedOperation(String),
    
    #[error("Timeout occurred: {0}")]
    Timeout(String),
    
    #[error("Authentication failed: {0}")]
    AuthenticationFailed(String),
}

/// Dispatcher errors
#[derive(Debug, Error)]
pub enum DispatcherError {
    #[error("Data object type not registered: {0}")]
    UnregisteredObjectType(String),
    
    #[error("No suitable connector found for query")]
    NoSuitableConnector,
    
    #[error("Query routing failed: {0}")]
    RoutingFailed(String),
    
    #[error("Cross-connector join not supported")]
    CrossConnectorJoinUnsupported,
    
    #[error("Connector registration failed: {0}")]
    RegistrationFailed(String),
}

/// Result type alias for NIRV operations
pub type NirvResult<T> = Result<T, NirvError>;#
[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_nirv_error_from_protocol_error() {
        let protocol_error = ProtocolError::ConnectionFailed("Test connection failed".to_string());
        let nirv_error: NirvError = protocol_error.into();
        
        match nirv_error {
            NirvError::Protocol(ProtocolError::ConnectionFailed(msg)) => {
                assert_eq!(msg, "Test connection failed");
            }
            _ => panic!("Expected Protocol error"),
        }
    }

    #[test]
    fn test_nirv_error_from_query_parsing_error() {
        let parsing_error = QueryParsingError::InvalidSyntax("Invalid SQL".to_string());
        let nirv_error: NirvError = parsing_error.into();
        
        match nirv_error {
            NirvError::QueryParsing(QueryParsingError::InvalidSyntax(msg)) => {
                assert_eq!(msg, "Invalid SQL");
            }
            _ => panic!("Expected QueryParsing error"),
        }
    }

    #[test]
    fn test_nirv_error_from_connector_error() {
        let connector_error = ConnectorError::QueryExecutionFailed("Query failed".to_string());
        let nirv_error: NirvError = connector_error.into();
        
        match nirv_error {
            NirvError::Connector(ConnectorError::QueryExecutionFailed(msg)) => {
                assert_eq!(msg, "Query failed");
            }
            _ => panic!("Expected Connector error"),
        }
    }

    #[test]
    fn test_nirv_error_from_dispatcher_error() {
        let dispatcher_error = DispatcherError::UnregisteredObjectType("unknown_type".to_string());
        let nirv_error: NirvError = dispatcher_error.into();
        
        match nirv_error {
            NirvError::Dispatcher(DispatcherError::UnregisteredObjectType(msg)) => {
                assert_eq!(msg, "unknown_type");
            }
            _ => panic!("Expected Dispatcher error"),
        }
    }

    #[test]
    fn test_error_display() {
        let error = NirvError::Configuration("Invalid config".to_string());
        let error_string = format!("{}", error);
        assert!(error_string.contains("Configuration error: Invalid config"));
    }

    #[test]
    fn test_nirv_result_type() {
        let success: NirvResult<String> = Ok("success".to_string());
        let failure: NirvResult<String> = Err(NirvError::Internal("test error".to_string()));
        
        assert!(success.is_ok());
        assert!(failure.is_err());
        
        match failure {
            Err(NirvError::Internal(msg)) => assert_eq!(msg, "test error"),
            _ => panic!("Expected Internal error"),
        }
    }
}