use std::collections::HashMap;
use std::sync::Arc;
use tokio::net::{TcpListener, TcpStream};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::sync::RwLock;

use nirv_engine::protocol::SqlServerProtocol;
use nirv_engine::utils::types::{QueryResult, ColumnMetadata, Row, Value, DataType};

/// Mock database with in-memory tables
#[derive(Debug, Clone)]
struct MockDatabase {
    tables: HashMap<String, MockTable>,
}

#[derive(Debug, Clone)]
struct MockTable {
    columns: Vec<ColumnMetadata>,
    rows: Vec<Row>,
}

impl MockDatabase {
    fn new() -> Self {
        let mut db = Self {
            tables: HashMap::new(),
        };
        
        // Create sample tables
        db.create_sample_data();
        db
    }
    
    fn create_sample_data(&mut self) {
        // Users table
        let users_table = MockTable {
            columns: vec![
                ColumnMetadata {
                    name: "id".to_string(),
                    data_type: DataType::Integer,
                    nullable: false,
                },
                ColumnMetadata {
                    name: "name".to_string(),
                    data_type: DataType::Text,
                    nullable: false,
                },
                ColumnMetadata {
                    name: "email".to_string(),
                    data_type: DataType::Text,
                    nullable: true,
                },
                ColumnMetadata {
                    name: "active".to_string(),
                    data_type: DataType::Boolean,
                    nullable: false,
                },
                ColumnMetadata {
                    name: "created_at".to_string(),
                    data_type: DataType::DateTime,
                    nullable: false,
                },
            ],
            rows: vec![
                Row::new(vec![
                    Value::Integer(1),
                    Value::Text("Alice Johnson".to_string()),
                    Value::Text("alice@example.com".to_string()),
                    Value::Boolean(true),
                    Value::DateTime("2024-01-15T10:30:00Z".to_string()),
                ]),
                Row::new(vec![
                    Value::Integer(2),
                    Value::Text("Bob Smith".to_string()),
                    Value::Text("bob@example.com".to_string()),
                    Value::Boolean(true),
                    Value::DateTime("2024-01-16T14:20:00Z".to_string()),
                ]),
                Row::new(vec![
                    Value::Integer(3),
                    Value::Text("Charlie Brown".to_string()),
                    Value::Text("charlie@example.com".to_string()),
                    Value::Boolean(false),
                    Value::DateTime("2024-01-17T09:15:00Z".to_string()),
                ]),
            ],
        };
        
        // Products table
        let products_table = MockTable {
            columns: vec![
                ColumnMetadata {
                    name: "id".to_string(),
                    data_type: DataType::Integer,
                    nullable: false,
                },
                ColumnMetadata {
                    name: "name".to_string(),
                    data_type: DataType::Text,
                    nullable: false,
                },
                ColumnMetadata {
                    name: "price".to_string(),
                    data_type: DataType::Float,
                    nullable: false,
                },
                ColumnMetadata {
                    name: "in_stock".to_string(),
                    data_type: DataType::Boolean,
                    nullable: false,
                },
            ],
            rows: vec![
                Row::new(vec![
                    Value::Integer(101),
                    Value::Text("Laptop".to_string()),
                    Value::Float(999.99),
                    Value::Boolean(true),
                ]),
                Row::new(vec![
                    Value::Integer(102),
                    Value::Text("Mouse".to_string()),
                    Value::Float(29.99),
                    Value::Boolean(true),
                ]),
                Row::new(vec![
                    Value::Integer(103),
                    Value::Text("Keyboard".to_string()),
                    Value::Float(79.99),
                    Value::Boolean(false),
                ]),
            ],
        };
        
        self.tables.insert("users".to_string(), users_table);
        self.tables.insert("products".to_string(), products_table);
    }
    
    fn execute_query(&self, sql: &str) -> QueryResult {
        let sql_lower = sql.to_lowercase();
        
        // Simple query parsing - in a real implementation, you'd use a proper SQL parser
        if sql_lower.contains("select") && sql_lower.contains("from") {
            if sql_lower.contains("users") {
                if let Some(table) = self.tables.get("users") {
                    return QueryResult {
                        columns: table.columns.clone(),
                        rows: table.rows.clone(),
                        affected_rows: Some(table.rows.len() as u64),
                        execution_time: std::time::Duration::from_millis(5),
                    };
                }
            } else if sql_lower.contains("products") {
                if let Some(table) = self.tables.get("products") {
                    return QueryResult {
                        columns: table.columns.clone(),
                        rows: table.rows.clone(),
                        affected_rows: Some(table.rows.len() as u64),
                        execution_time: std::time::Duration::from_millis(3),
                    };
                }
            }
        }
        
        // Default empty result
        QueryResult {
            columns: vec![],
            rows: vec![],
            affected_rows: Some(0),
            execution_time: std::time::Duration::from_millis(1),
        }
    }
}

/// NIRV SQL Server simulation server
struct NirvSqlServerSimulator {
    database: Arc<RwLock<MockDatabase>>,
    protocol: SqlServerProtocol,
}

impl NirvSqlServerSimulator {
    fn new() -> Self {
        Self {
            database: Arc::new(RwLock::new(MockDatabase::new())),
            protocol: SqlServerProtocol::new(),
        }
    }
    
    async fn handle_client(&self, mut stream: TcpStream, client_addr: std::net::SocketAddr) -> Result<(), Box<dyn std::error::Error>> {
        println!("📱 New client connected: {}", client_addr);
        
        // Read initial packet (should be login)
        let mut buffer = vec![0u8; 4096];
        let bytes_read = stream.read(&mut buffer).await?;
        
        if bytes_read == 0 {
            println!("❌ Client disconnected immediately");
            return Ok(());
        }
        
        buffer.truncate(bytes_read);
        println!("📦 Received packet: {} bytes", bytes_read);
        
        // Check packet type and handle accordingly
        if self.is_login_packet(&buffer) {
            println!("🔐 Processing login request...");
            
            // Send login acknowledgment
            let login_response = self.create_login_response();
            stream.write_all(&login_response).await?;
            println!("📤 Sent login acknowledgment");
        } else if self.is_sql_batch_packet(&buffer) {
            // Parse SQL from the packet
            if let Ok(sql) = self.parse_sql_from_packet(&buffer) {
                println!("📝 Processing SQL query: {}", sql);
                
                // Execute query against mock database
                let db = self.database.read().await;
                let result = db.execute_query(&sql);
                drop(db);
                
                println!("📊 Query result: {} rows, {} columns", 
                         result.rows.len(), result.columns.len());
                
                // Create and send response
                let response_bytes = self.create_query_response(result).await?;
                stream.write_all(&response_bytes).await?;
                println!("📤 Sent query response: {} bytes", response_bytes.len());
            } else {
                println!("❌ Failed to parse SQL from packet");
                let error_response = self.protocol.create_error_response(
                    50001, 
                    "Invalid SQL packet format", 
                    16
                );
                stream.write_all(&error_response).await?;
            }
        } else {
            println!("❌ Unknown packet type");
            let error_response = self.protocol.create_error_response(
                50002, 
                "Unknown packet type", 
                16
            );
            stream.write_all(&error_response).await?;
        }
        
        // Keep connection alive for more queries
        loop {
            let mut buffer = vec![0u8; 4096];
            match stream.read(&mut buffer).await {
                Ok(0) => {
                    println!("👋 Client {} disconnected", client_addr);
                    break;
                }
                Ok(bytes_read) => {
                    buffer.truncate(bytes_read);
                    println!("📦 Received additional packet: {} bytes", bytes_read);
                    
                    if self.is_sql_batch_packet(&buffer) {
                        if let Ok(sql) = self.parse_sql_from_packet(&buffer) {
                            println!("📝 Processing SQL query: {}", sql);
                            
                            let db = self.database.read().await;
                            let result = db.execute_query(&sql);
                            drop(db);
                            
                            let response_bytes = self.create_query_response(result).await?;
                            stream.write_all(&response_bytes).await?;
                            println!("📤 Sent query response: {} bytes", response_bytes.len());
                        }
                    }
                }
                Err(e) => {
                    println!("❌ Error reading from client {}: {}", client_addr, e);
                    break;
                }
            }
        }
        
        Ok(())
    }
    
    fn is_login_packet(&self, buffer: &[u8]) -> bool {
        buffer.len() >= 8 && buffer[0] == 0x10 // TDS Login packet type
    }
    
    fn is_sql_batch_packet(&self, buffer: &[u8]) -> bool {
        buffer.len() >= 8 && buffer[0] == 0x01 // TDS SQL Batch packet type
    }
    
    fn parse_sql_from_packet(&self, buffer: &[u8]) -> Result<String, String> {
        if buffer.len() < 8 {
            return Err("Packet too short".to_string());
        }
        
        // Skip TDS header (8 bytes) and parse UTF-16LE SQL text
        let sql_data = &buffer[8..];
        
        if sql_data.len() % 2 != 0 {
            return Err("Invalid UTF-16 data length".to_string());
        }
        
        let mut utf16_chars = Vec::new();
        for chunk in sql_data.chunks_exact(2) {
            let char_code = u16::from_le_bytes([chunk[0], chunk[1]]);
            utf16_chars.push(char_code);
        }
        
        String::from_utf16(&utf16_chars).map_err(|e| e.to_string())
    }
    
    async fn create_query_response(&self, result: QueryResult) -> Result<Vec<u8>, String> {
        let mut response = Vec::new();
        
        // Create column metadata
        let colmetadata = self.protocol.create_colmetadata(&result.columns);
        
        // Create data rows
        let mut rows_data = Vec::new();
        for row in &result.rows {
            let row_data = self.protocol.create_row(row, &result.columns);
            rows_data.extend_from_slice(&row_data);
        }
        
        // Create DONE token
        let done = self.protocol.create_done(0x0010, 0xC1, result.rows.len() as u64);
        
        // Combine all tokens
        let mut tokens = Vec::new();
        tokens.extend_from_slice(&colmetadata);
        tokens.extend_from_slice(&rows_data);
        tokens.extend_from_slice(&done);
        
        // Create TDS header
        let header = self.create_tds_header(0x04, (tokens.len() + 8) as u16); // 0x04 = Tabular result
        
        response.extend_from_slice(&header);
        response.extend_from_slice(&tokens);
        
        Ok(response)
    }
    
    fn create_tds_header(&self, packet_type: u8, length: u16) -> Vec<u8> {
        let mut header = Vec::with_capacity(8);
        header.push(packet_type);
        header.push(0x01); // Status: End of message
        header.extend_from_slice(&length.to_be_bytes());
        header.extend_from_slice(&0u16.to_be_bytes()); // SPID
        header.push(0x01); // Packet ID
        header.push(0x00); // Window
        header
    }
    
    fn create_login_response(&self) -> Vec<u8> {
        // Create a simple login acknowledgment response
        let mut response = Vec::new();
        
        // TDS header
        response.push(0x04); // Type: Tabular result
        response.push(0x01); // Status: End of message
        response.extend_from_slice(&50u16.to_be_bytes()); // Length
        response.extend_from_slice(&0u16.to_be_bytes()); // SPID
        response.push(0x01); // Packet ID
        response.push(0x00); // Window
        
        // LoginAck token
        response.push(0xAD); // LoginAck token type
        response.extend_from_slice(&36u16.to_le_bytes()); // Token length
        response.push(0x01); // Interface (SQL Server)
        response.extend_from_slice(&0x74000004u32.to_le_bytes()); // TDS version
        
        // Program name
        let program_name = "NIRV Engine";
        response.push(program_name.len() as u8);
        response.extend_from_slice(program_name.as_bytes());
        
        // Program version
        response.extend_from_slice(&0x01000000u32.to_le_bytes());
        
        // DONE token
        response.push(0xFD); // DONE token
        response.extend_from_slice(&0x0000u16.to_le_bytes()); // Status
        response.extend_from_slice(&0x0000u16.to_le_bytes()); // CurCmd
        response.extend_from_slice(&0u64.to_le_bytes()); // RowCount
        
        response
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("� NIRV Enginee - SQL Server Simulation Server");
    println!("==============================================");
    
    let simulator = NirvSqlServerSimulator::new();
    
    // Bind to SQL Server default port
    let listener = TcpListener::bind("127.0.0.1:1433").await?;
    println!("🎯 Server listening on 127.0.0.1:1433 (SQL Server port)");
    
    println!("\n� Aovailable sample data:");
    println!("  • users table: id, name, email, active, created_at");
    println!("  • products table: id, name, price, in_stock");
    
    println!("\n💡 Try connecting with a SQL Server client and run:");
    println!("  SELECT * FROM users");
    println!("  SELECT * FROM products");
    println!("  SELECT name, email FROM users WHERE active = 1");
    
    println!("\n🔧 Connection details:");
    println!("  Server: localhost,1433");
    println!("  Username: sa");
    println!("  Password: password123");
    println!("  Database: master");
    
    println!("\n⏳ Waiting for connections...\n");
    
    loop {
        match listener.accept().await {
            Ok((stream, addr)) => {
                println!("🔗 Accepted connection from: {}", addr);
                
                let simulator_clone = simulator.clone();
                tokio::spawn(async move {
                    if let Err(e) = simulator_clone.handle_client(stream, addr).await {
                        println!("❌ Error handling client {}: {}", addr, e);
                    }
                });
            }
            Err(e) => {
                println!("❌ Failed to accept connection: {}", e);
            }
        }
    }
}

impl Clone for NirvSqlServerSimulator {
    fn clone(&self) -> Self {
        Self {
            database: Arc::clone(&self.database),
            protocol: SqlServerProtocol::new(),
        }
    }
}