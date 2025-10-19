use tokio::net::{TcpListener, TcpStream};

use nirv_engine::connectors::{SqlServerConnector, Connector, ConnectorInitConfig};
use nirv_engine::protocol::{SqlServerProtocol, ProtocolAdapter, ProtocolType, Connection, Credentials};
use nirv_engine::utils::types::{QueryResult, ColumnMetadata, Row, Value, DataType};

/// Example demonstrating SQL Server protocol simulation
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 NIRV Engine - SQL Server Simulation Example");
    println!("===============================================");
    
    // 1. Create SQL Server connector
    println!("\n1. Creating SQL Server connector...");
    let connector = SqlServerConnector::new();
    
    // Configure connection
    let _config = ConnectorInitConfig::new()
        .with_param("server", "localhost")
        .with_param("port", "1433")
        .with_param("database", "testdb")
        .with_param("username", "sa")
        .with_param("password", "password123")
        .with_param("trust_cert", "true");
    
    println!("   ✓ Connector type: {:?}", connector.get_connector_type());
    println!("   ✓ Supports transactions: {}", connector.supports_transactions());
    
    // 2. Create SQL Server protocol adapter
    println!("\n2. Creating SQL Server protocol adapter...");
    let protocol = SqlServerProtocol::new();
    println!("   ✓ Protocol type: {:?}", protocol.get_protocol_type());
    
    // 3. Simulate connection establishment
    println!("\n3. Simulating connection establishment...");
    
    // Create a mock TCP connection for demonstration
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let addr = listener.local_addr()?;
    
    tokio::spawn(async move {
        let (stream, _) = listener.accept().await.unwrap();
        // Simulate server accepting connection
        drop(stream);
    });
    
    let stream = TcpStream::connect(addr).await?;
    let mut connection = Connection::new(stream, ProtocolType::SqlServer);
    
    println!("   ✓ Connection established");
    println!("   ✓ Protocol: {:?}", connection.protocol_type);
    
    // 4. Simulate authentication
    println!("\n4. Simulating authentication...");
    let credentials = Credentials::new("sa".to_string(), "testdb".to_string())
        .with_password("password123".to_string())
        .with_parameter("ApplicationName".to_string(), "NIRV Engine".to_string());
    
    protocol.authenticate(&mut connection, credentials).await?;
    println!("   ✓ Authentication successful");
    println!("   ✓ Database: {}", connection.database);
    
    // 5. Demonstrate TDS packet parsing
    println!("\n5. Demonstrating TDS packet parsing...");
    
    // Create a mock SQL batch packet
    let query_text = "SELECT id, name FROM users WHERE active = 1";
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
    
    let parsed_query = protocol.parse_message(&connection, &query_packet).await?;
    println!("   ✓ Parsed SQL: {}", parsed_query.raw_query);
    
    // 6. Demonstrate response formatting
    println!("\n6. Demonstrating response formatting...");
    
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
            Row::new(vec![Value::Integer(1), Value::Text("Alice".to_string())]),
            Row::new(vec![Value::Integer(2), Value::Text("Bob".to_string())]),
        ],
        affected_rows: Some(2),
        execution_time: std::time::Duration::from_millis(15),
    };
    
    let response_bytes = protocol.format_response(&connection, mock_result).await?;
    println!("   ✓ Response formatted: {} bytes", response_bytes.len());
    println!("   ✓ TDS packet type: 0x{:02X}", response_bytes[0]);
    
    // 7. Demonstrate data type conversions
    println!("\n7. Demonstrating data type conversions...");
    
    let test_values = vec![
        Value::Integer(42),
        Value::Text("Hello SQL Server".to_string()),
        Value::Boolean(true),
        Value::Float(3.14159),
        Value::Null,
    ];
    
    for value in &test_values {
        let tds_type = protocol.value_to_tds_type(value);
        println!("   ✓ {:?} -> TDS type: 0x{:02X}", value, tds_type);
    }
    
    // 8. Demonstrate error handling
    println!("\n8. Demonstrating error handling...");
    
    let error_response = protocol.create_error_response(
        208, 
        "Invalid object name 'nonexistent_table'", 
        16
    );
    println!("   ✓ Error response created: {} bytes", error_response.len());
    
    // 9. Show connector capabilities
    println!("\n9. SQL Server connector capabilities:");
    let capabilities = connector.get_capabilities();
    println!("   ✓ Supports joins: {}", capabilities.supports_joins);
    println!("   ✓ Supports aggregations: {}", capabilities.supports_aggregations);
    println!("   ✓ Supports subqueries: {}", capabilities.supports_subqueries);
    println!("   ✓ Supports transactions: {}", capabilities.supports_transactions);
    println!("   ✓ Supports schema introspection: {}", capabilities.supports_schema_introspection);
    println!("   ✓ Max concurrent queries: {:?}", capabilities.max_concurrent_queries);
    
    // 10. Cleanup
    println!("\n10. Cleaning up...");
    protocol.terminate_connection(&mut connection).await?;
    println!("   ✓ Connection terminated");
    
    println!("\n🎉 SQL Server simulation completed successfully!");
    println!("\nThis example demonstrates:");
    println!("  • SQL Server connector creation and configuration");
    println!("  • TDS protocol packet parsing and formatting");
    println!("  • Authentication simulation");
    println!("  • Data type conversions");
    println!("  • Error handling");
    println!("  • Connection lifecycle management");
    
    Ok(())
}