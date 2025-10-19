use tokio::net::{TcpListener, TcpStream};

use nirv_engine::protocol::{
    SqlServerProtocol, ProtocolAdapter, ProtocolType, Connection, Credentials,
    ProtocolQuery, ProtocolResponse, ResponseFormat
};
use nirv_engine::utils::types::{QueryResult, ColumnMetadata, Row, Value, DataType};

#[tokio::test]
async fn test_sqlserver_protocol_creation() {
    let protocol = SqlServerProtocol::new();
    assert_eq!(protocol.get_protocol_type(), ProtocolType::SqlServer);
}

#[tokio::test]
async fn test_sqlserver_protocol_type() {
    let protocol = SqlServerProtocol::new();
    assert_eq!(protocol.get_protocol_type(), ProtocolType::SqlServer);
}

#[tokio::test]
async fn test_sqlserver_connection_creation() {
    // Create a mock TCP stream for testing
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    
    tokio::spawn(async move {
        let (stream, _) = listener.accept().await.unwrap();
        // Just accept the connection for testing
        drop(stream);
    });
    
    let stream = TcpStream::connect(addr).await.unwrap();
    let connection = Connection::new(stream, ProtocolType::SqlServer);
    
    assert_eq!(connection.protocol_type, ProtocolType::SqlServer);
    assert!(!connection.authenticated);
    assert!(connection.database.is_empty());
}

#[tokio::test]
async fn test_sqlserver_credentials_creation() {
    let credentials = Credentials::new("sa".to_string(), "master".to_string())
        .with_password("password123".to_string())
        .with_parameter("ApplicationName".to_string(), "NIRV Engine".to_string())
        .with_parameter("Workstation".to_string(), "localhost".to_string());
    
    assert_eq!(credentials.username, "sa");
    assert_eq!(credentials.password, Some("password123".to_string()));
    assert_eq!(credentials.database, "master");
    assert_eq!(credentials.parameters.get("ApplicationName"), Some(&"NIRV Engine".to_string()));
    assert_eq!(credentials.parameters.get("Workstation"), Some(&"localhost".to_string()));
}

#[tokio::test]
async fn test_sqlserver_protocol_query_creation() {
    let query = ProtocolQuery::new(
        "SELECT * FROM users WHERE id = ?".to_string(),
        ProtocolType::SqlServer
    ).with_parameters(vec!["1".to_string()]);
    
    assert_eq!(query.raw_query, "SELECT * FROM users WHERE id = ?");
    assert_eq!(query.protocol_type, ProtocolType::SqlServer);
    assert_eq!(query.parameters, vec!["1".to_string()]);
}

#[tokio::test]
async fn test_sqlserver_protocol_response_creation() {
    let query_result = QueryResult {
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
            Row::new(vec![Value::Integer(1), Value::Text("John".to_string())]),
            Row::new(vec![Value::Integer(2), Value::Text("Jane".to_string())]),
        ],
        affected_rows: Some(2),
        execution_time: std::time::Duration::from_millis(10),
    };
    
    let response = ProtocolResponse::new(query_result.clone(), ProtocolType::SqlServer)
        .with_format(ResponseFormat::Binary);
    
    assert_eq!(response.protocol_type, ProtocolType::SqlServer);
    assert_eq!(response.format, ResponseFormat::Binary);
    assert_eq!(response.result.rows.len(), 2);
}

#[tokio::test]
async fn test_sqlserver_login_packet_parsing() {
    let protocol = SqlServerProtocol::new();
    
    // Create a mock SQL Server login packet
    let mut login_packet = Vec::new();
    
    // TDS Header (8 bytes)
    login_packet.push(0x10); // Type: Login
    login_packet.push(0x01); // Status: End of message
    login_packet.extend_from_slice(&100u16.to_be_bytes()); // Length
    login_packet.extend_from_slice(&0u16.to_be_bytes()); // SPID
    login_packet.push(0x01); // Packet ID
    login_packet.push(0x00); // Window
    
    // Login packet data (simplified)
    login_packet.extend_from_slice(&[0; 92]); // Pad to minimum size
    
    let result = protocol.parse_login_packet(&login_packet);
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_sqlserver_query_packet_parsing() {
    let protocol = SqlServerProtocol::new();
    
    // Create a mock SQL Server query packet
    let query_text = "SELECT * FROM users";
    let mut query_packet = Vec::new();
    
    // TDS Header
    query_packet.push(0x01); // Type: SQL Batch
    query_packet.push(0x01); // Status: End of message
    query_packet.extend_from_slice(&((8 + query_text.len() * 2) as u16).to_be_bytes()); // Length
    query_packet.extend_from_slice(&0u16.to_be_bytes()); // SPID
    query_packet.push(0x01); // Packet ID
    query_packet.push(0x00); // Window
    
    // Query text (UTF-16LE)
    for ch in query_text.chars() {
        let utf16_bytes = (ch as u16).to_le_bytes();
        query_packet.extend_from_slice(&utf16_bytes);
    }
    
    let result = protocol.parse_sql_batch(&query_packet[8..]);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), query_text);
}

#[tokio::test]
async fn test_sqlserver_response_formatting() {
    let protocol = SqlServerProtocol::new();
    
    let query_result = QueryResult {
        columns: vec![
            ColumnMetadata {
                name: "id".to_string(),
                data_type: DataType::Integer,
                nullable: false,
            },
        ],
        rows: vec![
            Row::new(vec![Value::Integer(42)]),
        ],
        affected_rows: Some(1),
        execution_time: std::time::Duration::from_millis(5),
    };
    
    // Create a mock connection
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    
    tokio::spawn(async move {
        let (stream, _) = listener.accept().await.unwrap();
        drop(stream);
    });
    
    let stream = TcpStream::connect(addr).await.unwrap();
    let connection = Connection::new(stream, ProtocolType::SqlServer);
    
    let response_bytes = protocol.format_response(&connection, query_result).await;
    assert!(response_bytes.is_ok());
    
    let bytes = response_bytes.unwrap();
    assert!(!bytes.is_empty());
    
    // Check TDS header
    assert_eq!(bytes[0], 0x04); // Type: Tabular result
    assert_eq!(bytes[1], 0x01); // Status: End of message
}

#[tokio::test]
async fn test_sqlserver_error_response_formatting() {
    let protocol = SqlServerProtocol::new();
    
    let error_message = "Invalid object name 'nonexistent_table'";
    let error_response = protocol.create_error_response(208, error_message, 1);
    
    assert!(!error_response.is_empty());
    assert_eq!(error_response[0], 0x04); // Type: Tabular result
    assert_eq!(error_response[1], 0x01); // Status: End of message
}

#[tokio::test]
async fn test_sqlserver_data_type_conversion() {
    let protocol = SqlServerProtocol::new();
    
    // Test various data type conversions
    assert_eq!(protocol.value_to_tds_type(&Value::Integer(42)), 0x26); // INTN
    assert_eq!(protocol.value_to_tds_type(&Value::Text("test".to_string())), 0xE7); // NVARCHAR
    assert_eq!(protocol.value_to_tds_type(&Value::Boolean(true)), 0x68); // BITN
    assert_eq!(protocol.value_to_tds_type(&Value::Float(3.14)), 0x6D); // FLOATN
    assert_eq!(protocol.value_to_tds_type(&Value::Null), 0x1F); // NULL
}

#[tokio::test]
async fn test_sqlserver_authentication_flow() {
    let protocol = SqlServerProtocol::new();
    
    // Create mock connection
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    
    tokio::spawn(async move {
        let (stream, _) = listener.accept().await.unwrap();
        drop(stream);
    });
    
    let stream = TcpStream::connect(addr).await.unwrap();
    let mut connection = Connection::new(stream, ProtocolType::SqlServer);
    
    let credentials = Credentials::new("sa".to_string(), "master".to_string())
        .with_password("password123".to_string());
    
    // Test authentication (should succeed with mock implementation)
    let auth_result = protocol.authenticate(&mut connection, credentials).await;
    assert!(auth_result.is_ok());
    assert!(connection.authenticated);
}