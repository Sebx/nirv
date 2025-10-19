use nirv_engine::protocol::{ProtocolAdapter, PostgresProtocol, ProtocolType, Credentials};
use nirv_engine::engine::{DefaultQueryExecutor, QueryExecutor};
use nirv_engine::utils::{QueryResult, ColumnMetadata, DataType, Row, Value};
use std::collections::HashMap;

#[cfg(test)]
mod integration_tests {
    use super::*;

    #[tokio::test]
    async fn test_postgres_protocol_integration() {
        // Create PostgreSQL protocol adapter
        let protocol = PostgresProtocol::new();
        
        // Verify protocol type
        assert_eq!(protocol.get_protocol_type(), ProtocolType::PostgreSQL);
        
        // Test that the protocol can be used in the engine context
        // This demonstrates that the protocol adapter integrates with the system
        let executor = DefaultQueryExecutor::new();
        
        // Create a mock query result that the protocol would format
        let result = QueryResult {
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
                Row::new(vec![Value::Integer(1), Value::Text("Alice".to_string())]),
                Row::new(vec![Value::Integer(2), Value::Text("Bob".to_string())]),
            ],
            affected_rows: Some(2),
            execution_time: std::time::Duration::from_millis(5),
        };
        
        // Test that the protocol can format the result
        // In a real scenario, this would be called after query execution
        let mock_connection = create_mock_connection().await;
        let formatted_response = protocol.format_response(&mock_connection, result).await;
        
        assert!(formatted_response.is_ok());
        let response_bytes = formatted_response.unwrap();
        
        // Verify that we got some response data
        assert!(!response_bytes.is_empty());
        
        // The response should contain PostgreSQL protocol messages
        // First byte should be 'T' for RowDescription
        assert_eq!(response_bytes[0], b'T');
    }

    #[tokio::test]
    async fn test_postgres_message_parsing() {
        let protocol = PostgresProtocol::new();
        let mock_connection = create_mock_connection().await;
        
        // Test parsing a simple query message
        let query_message = create_query_message("SELECT * FROM source('postgres.users')");
        let parsed_query = protocol.parse_message(&mock_connection, &query_message).await;
        
        assert!(parsed_query.is_ok());
        let query = parsed_query.unwrap();
        assert_eq!(query.raw_query, "SELECT * FROM source('postgres.users')");
        assert_eq!(query.protocol_type, ProtocolType::PostgreSQL);
    }

    #[tokio::test]
    async fn test_postgres_credentials_validation() {
        let _protocol = PostgresProtocol::new();
        
        // Test credential creation
        let credentials = Credentials::new("testuser".to_string(), "testdb".to_string())
            .with_password("testpass".to_string())
            .with_parameter("client_encoding".to_string(), "UTF8".to_string());
        
        assert_eq!(credentials.username, "testuser");
        assert_eq!(credentials.database, "testdb");
        assert_eq!(credentials.password, Some("testpass".to_string()));
        assert_eq!(credentials.parameters.get("client_encoding"), Some(&"UTF8".to_string()));
    }

    // Helper function to create a mock connection for testing
    async fn create_mock_connection() -> nirv_engine::protocol::Connection {
        use tokio::net::{TcpListener, TcpStream};
        
        // Create a local TCP connection for testing
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        
        // Create a connection to ourselves
        let stream = TcpStream::connect(addr).await.unwrap();
        
        let mut connection = nirv_engine::protocol::Connection::new(stream, ProtocolType::PostgreSQL);
        connection.authenticated = true;
        connection.database = "testdb".to_string();
        connection.parameters.insert("client_encoding".to_string(), "UTF8".to_string());
        
        connection
    }

    // Helper function to create a PostgreSQL query message
    fn create_query_message(query: &str) -> Vec<u8> {
        let mut message = Vec::new();
        message.push(b'Q'); // Query message type
        
        let content_len = query.len() + 1; // +1 for null terminator
        message.extend_from_slice(&(content_len as u32 + 4).to_be_bytes()); // Message length
        
        message.extend_from_slice(query.as_bytes());
        message.push(0); // Null terminator
        
        message
    }
}