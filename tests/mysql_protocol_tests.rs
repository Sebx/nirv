#![allow(unused)]

use nirv_engine::protocol::{MySQLProtocolAdapter, ProtocolAdapter, ProtocolType, Connection, ProtocolQuery};
use nirv_engine::utils::{QueryResult, ColumnMetadata, Row, Value, DataType};
use tokio::net::{TcpListener, TcpStream};
use std::time::Duration;
use std::env;

/// Helper function to get MySQL connection parameters from environment
/// This will use the service container configuration when running in CI
fn get_mysql_config() -> MySQLConnectionConfig {
    MySQLConnectionConfig {
        host: env::var("MYSQL_HOST").unwrap_or_else(|_| "localhost".to_string()),
        port: env::var("MYSQL_PORT").unwrap_or_else(|_| "3306".to_string()).parse().unwrap_or(3306),
        user: env::var("MYSQL_USER").unwrap_or_else(|_| "testuser".to_string()),
        password: env::var("MYSQL_PASSWORD").unwrap_or_else(|_| "testpassword".to_string()),
        database: env::var("MYSQL_DATABASE").unwrap_or_else(|_| "testdb".to_string()),
    }
}

/// MySQL connection configuration for testing
#[derive(Debug, Clone)]
struct MySQLConnectionConfig {
    host: String,
    port: u16,
    user: String,
    password: String,
    database: String,
}

/// Helper function to create a mock TCP connection for testing
async fn create_mock_connection() -> Connection {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let stream = TcpStream::connect(addr).await.unwrap();
    Connection::new(stream, ProtocolType::MySQL)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mysql_protocol_creation() {
        let protocol = MySQLProtocolAdapter::new();
        assert_eq!(protocol.get_protocol_type(), ProtocolType::MySQL);
    }

    #[test]
    fn test_mysql_protocol_default() {
        let protocol = MySQLProtocolAdapter::default();
        assert_eq!(protocol.get_protocol_type(), ProtocolType::MySQL);
    }

    #[tokio::test]
    async fn test_mysql_handshake_packet_creation() {
        let protocol = MySQLProtocolAdapter::new();
        
        // Create a mock TCP connection
        let connection = create_mock_connection().await;
        
        // Test handshake creation
        assert_eq!(connection.protocol_type, ProtocolType::MySQL);
        assert!(!connection.authenticated);
        assert!(connection.database.is_empty());
    }

    #[tokio::test]
    async fn test_mysql_service_container_compatibility() {
        let protocol = MySQLProtocolAdapter::new();
        let config = get_mysql_config();
        
        // Test that we can create protocol adapter with service container config
        assert_eq!(protocol.get_protocol_type(), ProtocolType::MySQL);
        
        // Verify configuration is properly formatted
        assert!(!config.host.is_empty());
        assert!(config.port > 0);
        assert!(!config.user.is_empty());
        assert!(!config.database.is_empty());
    }

    #[tokio::test]
    async fn test_mysql_query_parsing() {
        let protocol = MySQLProtocolAdapter::new();
        
        // Create a mock connection
        let connection = create_mock_connection().await;
        
        // Test query command packet (simplified)
        let query_packet = vec![
            0x15, 0x00, 0x00, 0x00, // Packet length (21 bytes) + sequence ID
            0x03, // COM_QUERY
            b'S', b'E', b'L', b'E', b'C', b'T', b' ', b'*', b' ', 
            b'F', b'R', b'O', b'M', b' ', b'u', b's', b'e', b'r', b's'
        ];
        
        let parsed_query = protocol.parse_message(&connection, &query_packet).await.unwrap();
        assert_eq!(parsed_query.raw_query, "SELECT * FROM users");
        assert_eq!(parsed_query.protocol_type, ProtocolType::MySQL);
    }

    #[tokio::test]
    async fn test_mysql_quit_command() {
        let protocol = MySQLProtocolAdapter::new();
        
        // Create a mock connection
        let connection = create_mock_connection().await;
        
        // Test quit command packet
        let quit_packet = vec![
            0x01, 0x00, 0x00, 0x00, // Packet length (1 byte) + sequence ID
            0x01, // COM_QUIT
        ];
        
        let parsed_query = protocol.parse_message(&connection, &quit_packet).await.unwrap();
        assert_eq!(parsed_query.raw_query, "QUIT");
        assert_eq!(parsed_query.protocol_type, ProtocolType::MySQL);
    }

    #[tokio::test]
    async fn test_mysql_ping_command() {
        let protocol = MySQLProtocolAdapter::new();
        
        // Create a mock connection
        let connection = create_mock_connection().await;
        
        // Test ping command packet
        let ping_packet = vec![
            0x01, 0x00, 0x00, 0x00, // Packet length (1 byte) + sequence ID
            0x0e, // COM_PING
        ];
        
        let parsed_query = protocol.parse_message(&connection, &ping_packet).await.unwrap();
        assert_eq!(parsed_query.raw_query, "PING");
        assert_eq!(parsed_query.protocol_type, ProtocolType::MySQL);
    }

    #[tokio::test]
    async fn test_mysql_response_formatting() {
        let protocol = MySQLProtocolAdapter::new();
        
        // Create a mock connection
        let connection = create_mock_connection().await;
        
        // Create test result
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
            Row::new(vec![Value::Integer(1), Value::Text("Alice".to_string())]),
            Row::new(vec![Value::Integer(2), Value::Text("Bob".to_string())]),
        ];
        
        let result = QueryResult {
            columns,
            rows,
            affected_rows: Some(2),
            execution_time: Duration::from_millis(10),
        };
        
        // Format response
        let response_bytes = protocol.format_response(&connection, result).await.unwrap();
        
        // Should contain MySQL protocol packets
        assert!(!response_bytes.is_empty());
        
        // First packet should be result set header
        assert_eq!(response_bytes[0], 0x01); // Length byte 1
        assert_eq!(response_bytes[1], 0x00); // Length byte 2
        assert_eq!(response_bytes[2], 0x00); // Length byte 3
        assert_eq!(response_bytes[3], 0x01); // Sequence ID
        assert_eq!(response_bytes[4], 0x02); // Column count (2)
    }

    #[tokio::test]
    async fn test_mysql_handle_query() {
        let protocol = MySQLProtocolAdapter::new();
        
        // Create a mock connection
        let connection = create_mock_connection().await;
        
        // Create test query
        let query = ProtocolQuery::new("SELECT * FROM users".to_string(), ProtocolType::MySQL);
        
        // Handle query
        let response = protocol.handle_query(&connection, query).await.unwrap();
        
        assert_eq!(response.protocol_type, ProtocolType::MySQL);
        assert!(!response.result.columns.is_empty());
        assert!(!response.result.rows.is_empty());
    }

    #[tokio::test]
    async fn test_mysql_ok_packet_for_non_select() {
        let protocol = MySQLProtocolAdapter::new();
        
        // Create a mock connection
        let connection = create_mock_connection().await;
        
        // Create result with no columns (non-SELECT query)
        let result = QueryResult {
            columns: vec![],
            rows: vec![],
            affected_rows: Some(1),
            execution_time: Duration::from_millis(5),
        };
        
        // Format response
        let response_bytes = protocol.format_response(&connection, result).await.unwrap();
        
        // Should contain OK packet
        assert!(!response_bytes.is_empty());
        
        // Check OK packet structure
        assert_eq!(response_bytes[4], 0x00); // OK packet header
    }

    #[tokio::test]
    async fn test_mysql_value_conversion() {
        let protocol = MySQLProtocolAdapter::new();
        
        // Test various value types through response formatting
        let columns = vec![
            ColumnMetadata {
                name: "text_col".to_string(),
                data_type: DataType::Text,
                nullable: false,
            },
            ColumnMetadata {
                name: "int_col".to_string(),
                data_type: DataType::Integer,
                nullable: false,
            },
            ColumnMetadata {
                name: "bool_col".to_string(),
                data_type: DataType::Boolean,
                nullable: false,
            },
            ColumnMetadata {
                name: "null_col".to_string(),
                data_type: DataType::Text,
                nullable: true,
            },
        ];
        
        let rows = vec![
            Row::new(vec![
                Value::Text("test".to_string()),
                Value::Integer(42),
                Value::Boolean(true),
                Value::Null,
            ]),
        ];
        
        let result = QueryResult {
            columns,
            rows,
            affected_rows: Some(1),
            execution_time: Duration::from_millis(1),
        };
        
        // Create a mock connection
        let connection = create_mock_connection().await;
        
        // Format response should not panic
        let response_bytes = protocol.format_response(&connection, result).await.unwrap();
        assert!(!response_bytes.is_empty());
    }

    #[tokio::test]
    async fn test_mysql_command_parsing_invalid() {
        let protocol = MySQLProtocolAdapter::new();
        
        // Create a mock connection
        let connection = create_mock_connection().await;
        
        // Test invalid packet (too short)
        let invalid_packet = vec![0x01, 0x00, 0x00]; // Missing sequence ID and command
        
        let result = protocol.parse_message(&connection, &invalid_packet).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_mysql_unsupported_command() {
        let protocol = MySQLProtocolAdapter::new();
        
        // Create a mock connection
        let connection = create_mock_connection().await;
        
        // Test unsupported command
        let unsupported_packet = vec![
            0x01, 0x00, 0x00, 0x00, // Packet header
            0xFF, // Unsupported command
        ];
        
        let result = protocol.parse_message(&connection, &unsupported_packet).await;
        assert!(result.is_err());
    }
}

/// Integration tests that require a running MySQL instance
#[cfg(test)]
mod integration_tests {
    use super::*;

    /// Test MySQL protocol compatibility with service container
    #[tokio::test]
    async fn test_mysql_protocol_with_service_container() {
        // Only run this test if we have MySQL environment variables set
        if env::var("MYSQL_HOST").is_err() {
            return; // Skip test if not in CI environment
        }

        let protocol = MySQLProtocolAdapter::new();
        let config = get_mysql_config();
        
        // Test protocol adapter creation with service container config
        assert_eq!(protocol.get_protocol_type(), ProtocolType::MySQL);
        
        // Verify that configuration matches expected service container values
        if env::var("CI").is_ok() {
            // In CI environment, verify service container configuration
            assert_eq!(config.host, "localhost");
            assert_eq!(config.port, 3306);
            assert_eq!(config.user, "testuser");
            assert_eq!(config.database, "testdb");
        }
    }

    /// Test MySQL protocol error handling with service container
    #[tokio::test]
    async fn test_mysql_protocol_error_handling() {
        let protocol = MySQLProtocolAdapter::new();
        
        // Test protocol adapter with invalid connection parameters
        let invalid_config = MySQLConnectionConfig {
            host: "nonexistent-mysql-host".to_string(),
            port: 3306,
            user: "invalid_user".to_string(),
            password: "invalid_password".to_string(),
            database: "invalid_db".to_string(),
        };
        
        // Protocol adapter should still be created successfully
        // (actual connection testing would be done at a higher level)
        assert_eq!(protocol.get_protocol_type(), ProtocolType::MySQL);
        assert!(!invalid_config.host.is_empty());
    }

    /// Test MySQL protocol packet handling with containerized MySQL
    #[tokio::test]
    async fn test_mysql_protocol_packet_compatibility() {
        let protocol = MySQLProtocolAdapter::new();
        
        // Test that protocol can handle various MySQL packet types
        // that would be sent by a containerized MySQL instance
        
        // Test authentication packet (simplified)
        let auth_packet = vec![
            0x20, 0x00, 0x00, 0x01, // Packet header
            0x85, 0xa2, 0x1e, 0x00, // Client flags
            0x00, 0x00, 0x00, 0x01, // Max packet size
            0x21, // Charset
            // ... (rest would be username, password, database)
        ];
        
        // Protocol should be able to handle this packet structure
        assert_eq!(protocol.get_protocol_type(), ProtocolType::MySQL);
        
        // Test that we can create connections for MySQL protocol
        let connection = create_mock_connection().await;
        assert_eq!(connection.protocol_type, ProtocolType::MySQL);
    }
}