use nirv_engine::protocol::{MySQLProtocolAdapter, ProtocolAdapter, ProtocolType, Connection, ProtocolQuery};
use nirv_engine::utils::{QueryResult, ColumnMetadata, Row, Value, DataType};
use tokio::net::{TcpListener, TcpStream};
use std::time::Duration;

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
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        
        // Connect to the listener
        let stream = TcpStream::connect(addr).await.unwrap();
        
        // Accept connection should send handshake
        let connection = protocol.accept_connection(stream).await.unwrap();
        
        assert_eq!(connection.protocol_type, ProtocolType::MySQL);
        assert!(!connection.authenticated);
        assert!(connection.database.is_empty());
    }

    #[tokio::test]
    async fn test_mysql_query_parsing() {
        let protocol = MySQLProtocolAdapter::new();
        
        // Create a mock connection
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let stream = TcpStream::connect(addr).await.unwrap();
        let connection = Connection::new(stream, ProtocolType::MySQL);
        
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
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let stream = TcpStream::connect(addr).await.unwrap();
        let connection = Connection::new(stream, ProtocolType::MySQL);
        
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
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let stream = TcpStream::connect(addr).await.unwrap();
        let connection = Connection::new(stream, ProtocolType::MySQL);
        
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
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let stream = TcpStream::connect(addr).await.unwrap();
        let connection = Connection::new(stream, ProtocolType::MySQL);
        
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
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let stream = TcpStream::connect(addr).await.unwrap();
        let connection = Connection::new(stream, ProtocolType::MySQL);
        
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
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let stream = TcpStream::connect(addr).await.unwrap();
        let connection = Connection::new(stream, ProtocolType::MySQL);
        
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
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let stream = TcpStream::connect(addr).await.unwrap();
        let connection = Connection::new(stream, ProtocolType::MySQL);
        
        // Format response should not panic
        let response_bytes = protocol.format_response(&connection, result).await.unwrap();
        assert!(!response_bytes.is_empty());
    }

    #[tokio::test]
    async fn test_mysql_command_parsing_invalid() {
        let protocol = MySQLProtocolAdapter::new();
        
        // Create a mock connection
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let stream = TcpStream::connect(addr).await.unwrap();
        let connection = Connection::new(stream, ProtocolType::MySQL);
        
        // Test invalid packet (too short)
        let invalid_packet = vec![0x01, 0x00, 0x00]; // Missing sequence ID and command
        
        let result = protocol.parse_message(&connection, &invalid_packet).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_mysql_unsupported_command() {
        let protocol = MySQLProtocolAdapter::new();
        
        // Create a mock connection
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let stream = TcpStream::connect(addr).await.unwrap();
        let connection = Connection::new(stream, ProtocolType::MySQL);
        
        // Test unsupported command
        let unsupported_packet = vec![
            0x01, 0x00, 0x00, 0x00, // Packet header
            0xFF, // Unsupported command
        ];
        
        let result = protocol.parse_message(&connection, &unsupported_packet).await;
        assert!(result.is_err());
    }
}