use tokio::net::TcpStream;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

/// Simple SQL Server client to test our NIRV simulation
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🔌 NIRV SQL Server Client Test");
    println!("==============================");
    
    // Connect to our NIRV SQL Server simulation
    println!("🔗 Connecting to NIRV SQL Server at localhost:1433...");
    let mut stream = TcpStream::connect("127.0.0.1:1433").await?;
    println!("✅ Connected successfully!");
    
    // Send a mock login packet
    println!("\n🔐 Sending login packet...");
    let login_packet = create_mock_login_packet();
    stream.write_all(&login_packet).await?;
    
    // Read login response
    let mut buffer = vec![0u8; 1024];
    let bytes_read = stream.read(&mut buffer).await?;
    buffer.truncate(bytes_read);
    
    if bytes_read > 0 {
        println!("✅ Received login response: {} bytes", bytes_read);
        println!("📦 Response header: {:02X} {:02X} {:04X}", 
                 buffer[0], buffer[1], 
                 u16::from_be_bytes([buffer[2], buffer[3]]));
    }
    
    // Send SQL queries
    let queries = vec![
        "SELECT * FROM users",
        "SELECT * FROM products", 
        "SELECT name, email FROM users WHERE active = 1",
    ];
    
    for (i, query) in queries.iter().enumerate() {
        println!("\n📝 Sending query {}: {}", i + 1, query);
        
        let query_packet = create_sql_batch_packet(query);
        stream.write_all(&query_packet).await?;
        
        // Read response
        let mut buffer = vec![0u8; 4096];
        let bytes_read = stream.read(&mut buffer).await?;
        buffer.truncate(bytes_read);
        
        if bytes_read > 0 {
            println!("✅ Received query response: {} bytes", bytes_read);
            println!("📦 Response header: {:02X} {:02X} {:04X}", 
                     buffer[0], buffer[1], 
                     u16::from_be_bytes([buffer[2], buffer[3]]));
            
            // Try to parse some basic info from the response
            if buffer.len() > 8 {
                println!("📊 Response data preview: {:02X?}", &buffer[8..std::cmp::min(buffer.len(), 24)]);
            }
        } else {
            println!("❌ No response received");
        }
        
        // Small delay between queries
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }
    
    println!("\n👋 Closing connection...");
    drop(stream);
    println!("✅ Client test completed!");
    
    Ok(())
}

fn create_mock_login_packet() -> Vec<u8> {
    let mut packet = Vec::new();
    
    // TDS Header (8 bytes)
    packet.push(0x10); // Type: Login
    packet.push(0x01); // Status: End of message
    packet.extend_from_slice(&100u16.to_be_bytes()); // Length (will be updated)
    packet.extend_from_slice(&0u16.to_be_bytes()); // SPID
    packet.push(0x01); // Packet ID
    packet.push(0x00); // Window
    
    // Login packet data (simplified mock)
    // In a real implementation, this would be a proper TDS login packet
    let login_data = vec![0u8; 92]; // Minimum login packet size
    packet.extend_from_slice(&login_data);
    
    // Update length in header
    let total_length = packet.len() as u16;
    packet[2..4].copy_from_slice(&total_length.to_be_bytes());
    
    packet
}

fn create_sql_batch_packet(sql: &str) -> Vec<u8> {
    let mut packet = Vec::new();
    
    // TDS Header
    packet.push(0x01); // Type: SQL Batch
    packet.push(0x01); // Status: End of message
    
    // Calculate length (header + UTF-16 SQL text)
    let sql_utf16_len = sql.len() * 2;
    let total_length = 8 + sql_utf16_len;
    packet.extend_from_slice(&(total_length as u16).to_be_bytes());
    
    packet.extend_from_slice(&0u16.to_be_bytes()); // SPID
    packet.push(0x01); // Packet ID
    packet.push(0x00); // Window
    
    // SQL text as UTF-16LE
    for ch in sql.chars() {
        let utf16_bytes = (ch as u16).to_le_bytes();
        packet.extend_from_slice(&utf16_bytes);
    }
    
    packet
}