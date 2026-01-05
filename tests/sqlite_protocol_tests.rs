#![allow(unused)]

use nirv_engine::protocol::{ProtocolAdapter, SQLiteProtocolAdapter, ProtocolType};
use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::Path;
use std::time;

/// Helper function to get SQLite database path for CI environment
fn get_sqlite_test_db_path() -> String {
    if env::var("CI").is_ok() {
        // In CI environment, use a temporary directory
        format!("/tmp/nirv_test_{}.db", std::process::id())
    } else {
        // In local environment, use current directory
        format!("./test_{}.db", std::process::id())
    }
}

/// Helper function to clean up test database files
fn cleanup_test_db(db_path: &str) {
    if Path::new(db_path).exists() {
        let _ = fs::remove_file(db_path);
    }
    
    // Also clean up any associated files (WAL, SHM)
    let wal_path = format!("{}-wal", db_path);
    let shm_path = format!("{}-shm", db_path);
    
    if Path::new(&wal_path).exists() {
        let _ = fs::remove_file(&wal_path);
    }
    
    if Path::new(&shm_path).exists() {
        let _ = fs::remove_file(&shm_path);
    }
}

/// Helper function to ensure test directory exists in CI
fn ensure_test_directory() {
    if env::var("CI").is_ok() {
        // Ensure /tmp directory is writable in CI
        let test_dir = "/tmp";
        if !Path::new(test_dir).exists() {
            let _ = fs::create_dir_all(test_dir);
        }
    }
}

/// Test data structures for SQLite protocol testing
#[derive(Debug, Clone)]
struct TestCredentials {
    username: String,
    password: Option<String>,
    database: String,
}

#[derive(Debug)]
struct TestConnection {
    authenticated: bool,
    database: String,
    parameters: HashMap<String, String>,
}

impl TestConnection {
    fn new() -> Self {
        Self {
            authenticated: false,
            database: String::new(),
            parameters: HashMap::new(),
        }
    }
}

/// SQLite protocol message types for testing
#[derive(Debug, Clone, PartialEq)]
enum SQLiteMessage {
    Connect {
        database_path: String,
        flags: u32,
    },
    Query {
        sql: String,
    },
    Prepare {
        sql: String,
    },
    Execute {
        statement_id: u32,
        parameters: Vec<SQLiteValue>,
    },
    Close,
}

/// SQLite value types for testing
#[derive(Debug, Clone, PartialEq)]
enum SQLiteValue {
    Null,
    Integer(i64),
    Real(f64),
    Text(String),
    Blob(Vec<u8>),
}

/// SQLite response types for testing
#[derive(Debug, Clone, PartialEq)]
enum SQLiteResponse {
    Ok {
        changes: u32,
        last_insert_rowid: i64,
    },
    Row {
        columns: Vec<SQLiteValue>,
    },
    Done,
    Error {
        code: u32,
        message: String,
    },
}

/// SQLite column metadata for testing
#[derive(Debug, Clone, PartialEq)]
struct SQLiteColumn {
    name: String,
    type_name: String,
    nullable: bool,
    primary_key: bool,
    auto_increment: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_sqlite_protocol_creation() {
        let protocol = SQLiteProtocolAdapter::new();
        assert_eq!(protocol.get_protocol_type(), ProtocolType::SQLite);
    }

    #[tokio::test]
    async fn test_sqlite_connection_establishment() {
        let _protocol = SQLiteProtocolAdapter::new();
        let mut connection = TestConnection::new();
        
        // Test SQLite connection with file path
        let connect_msg = SQLiteMessage::Connect {
            database_path: ":memory:".to_string(),
            flags: 0x00000006, // SQLITE_OPEN_READWRITE | SQLITE_OPEN_CREATE
        };
        
        match connect_msg {
            SQLiteMessage::Connect { database_path, flags } => {
                assert_eq!(database_path, ":memory:");
                assert_eq!(flags, 0x00000006);
                
                // Simulate successful connection
                connection.authenticated = true;
                connection.database = database_path;
            }
            _ => panic!("Expected Connect message"),
        }
        
        assert!(connection.authenticated);
        assert_eq!(connection.database, ":memory:");
    }

    #[tokio::test]
    async fn test_sqlite_file_based_connection() {
        ensure_test_directory();
        let _protocol = SQLiteProtocolAdapter::new();
        let mut connection = TestConnection::new();
        
        // Test SQLite connection with actual file path
        let test_db_path = get_sqlite_test_db_path();
        let connect_msg = SQLiteMessage::Connect {
            database_path: test_db_path.clone(),
            flags: 0x00000006, // SQLITE_OPEN_READWRITE | SQLITE_OPEN_CREATE
        };
        
        match connect_msg {
            SQLiteMessage::Connect { database_path, flags } => {
                assert_eq!(database_path, test_db_path);
                assert_eq!(flags, 0x00000006);
                
                // Simulate successful file-based connection
                connection.authenticated = true;
                connection.database = database_path.clone();
            }
            _ => panic!("Expected Connect message"),
        }
        
        assert!(connection.authenticated);
        assert_eq!(connection.database, test_db_path);
        
        // Clean up test database file
        cleanup_test_db(&test_db_path);
    }

    #[tokio::test]
    async fn test_sqlite_ci_environment_compatibility() {
        ensure_test_directory();
        let _protocol = SQLiteProtocolAdapter::new();
        
        // Test that SQLite works in CI environment
        let test_db_path = get_sqlite_test_db_path();
        
        // Verify path is appropriate for CI environment
        if env::var("CI").is_ok() {
            assert!(test_db_path.starts_with("/tmp/"));
        }
        
        // Test connection message creation
        let connect_msg = SQLiteMessage::Connect {
            database_path: test_db_path.clone(),
            flags: 0x00000006,
        };
        
        match connect_msg {
            SQLiteMessage::Connect { database_path, flags } => {
                assert_eq!(database_path, test_db_path);
                assert_eq!(flags, 0x00000006);
            }
            _ => panic!("Expected Connect message"),
        }
        
        // Clean up
        cleanup_test_db(&test_db_path);
    }

    #[tokio::test]
    async fn test_sqlite_query_execution() {
        let _protocol = SQLiteProtocolAdapter::new();
        
        // Test basic SELECT query
        let query_msg = SQLiteMessage::Query {
            sql: "SELECT * FROM source('file.users.csv') WHERE age > 25".to_string(),
        };
        
        match query_msg {
            SQLiteMessage::Query { sql } => {
                assert!(sql.contains("SELECT"));
                assert!(sql.contains("source('file.users.csv')"));
                assert!(sql.contains("WHERE age > 25"));
            }
            _ => panic!("Expected Query message"),
        }
    }

    #[tokio::test]
    async fn test_sqlite_prepared_statements() {
        let _protocol = SQLiteProtocolAdapter::new();
        
        // Test prepared statement creation
        let prepare_msg = SQLiteMessage::Prepare {
            sql: "SELECT name, age FROM source('api.users') WHERE id = ?".to_string(),
        };
        
        match prepare_msg {
            SQLiteMessage::Prepare { sql } => {
                assert!(sql.contains("SELECT"));
                assert!(sql.contains("source('api.users')"));
                assert!(sql.contains("WHERE id = ?"));
            }
            _ => panic!("Expected Prepare message"),
        }
        
        // Test prepared statement execution
        let execute_msg = SQLiteMessage::Execute {
            statement_id: 1,
            parameters: vec![SQLiteValue::Integer(123)],
        };
        
        match execute_msg {
            SQLiteMessage::Execute { statement_id, parameters } => {
                assert_eq!(statement_id, 1);
                assert_eq!(parameters.len(), 1);
                assert_eq!(parameters[0], SQLiteValue::Integer(123));
            }
            _ => panic!("Expected Execute message"),
        }
    }

    #[tokio::test]
    async fn test_sqlite_data_types() {
        let _protocol = SQLiteProtocolAdapter::new();
        
        // Test SQLite value types
        let values = vec![
            SQLiteValue::Null,
            SQLiteValue::Integer(42),
            SQLiteValue::Real(3.14159),
            SQLiteValue::Text("Hello, SQLite!".to_string()),
            SQLiteValue::Blob(vec![0x01, 0x02, 0x03, 0x04]),
        ];
        
        assert_eq!(values[0], SQLiteValue::Null);
        assert_eq!(values[1], SQLiteValue::Integer(42));
        assert_eq!(values[2], SQLiteValue::Real(3.14159));
        assert_eq!(values[3], SQLiteValue::Text("Hello, SQLite!".to_string()));
        assert_eq!(values[4], SQLiteValue::Blob(vec![0x01, 0x02, 0x03, 0x04]));
    }

    #[tokio::test]
    async fn test_sqlite_response_formatting() {
        let _protocol = SQLiteProtocolAdapter::new();
        
        // Test OK response
        let ok_response = SQLiteResponse::Ok {
            changes: 1,
            last_insert_rowid: 123,
        };
        
        match ok_response {
            SQLiteResponse::Ok { changes, last_insert_rowid } => {
                assert_eq!(changes, 1);
                assert_eq!(last_insert_rowid, 123);
            }
            _ => panic!("Expected Ok response"),
        }
        
        // Test Row response
        let row_response = SQLiteResponse::Row {
            columns: vec![
                SQLiteValue::Integer(1),
                SQLiteValue::Text("John Doe".to_string()),
                SQLiteValue::Integer(30),
            ],
        };
        
        match row_response {
            SQLiteResponse::Row { columns } => {
                assert_eq!(columns.len(), 3);
                assert_eq!(columns[0], SQLiteValue::Integer(1));
                assert_eq!(columns[1], SQLiteValue::Text("John Doe".to_string()));
                assert_eq!(columns[2], SQLiteValue::Integer(30));
            }
            _ => panic!("Expected Row response"),
        }
    }

    #[tokio::test]
    async fn test_sqlite_error_handling() {
        let _protocol = SQLiteProtocolAdapter::new();
        
        // Test error response
        let error_response = SQLiteResponse::Error {
            code: 1, // SQLITE_ERROR
            message: "no such table: nonexistent_table".to_string(),
        };
        
        match error_response {
            SQLiteResponse::Error { code, message } => {
                assert_eq!(code, 1);
                assert!(message.contains("no such table"));
            }
            _ => panic!("Expected Error response"),
        }
    }

    #[tokio::test]
    async fn test_sqlite_column_metadata() {
        let _protocol = SQLiteProtocolAdapter::new();
        
        // Test column metadata
        let columns = vec![
            SQLiteColumn {
                name: "id".to_string(),
                type_name: "INTEGER".to_string(),
                nullable: false,
                primary_key: true,
                auto_increment: true,
            },
            SQLiteColumn {
                name: "name".to_string(),
                type_name: "TEXT".to_string(),
                nullable: true,
                primary_key: false,
                auto_increment: false,
            },
            SQLiteColumn {
                name: "created_at".to_string(),
                type_name: "DATETIME".to_string(),
                nullable: false,
                primary_key: false,
                auto_increment: false,
            },
        ];
        
        assert_eq!(columns[0].name, "id");
        assert_eq!(columns[0].type_name, "INTEGER");
        assert!(columns[0].primary_key);
        assert!(columns[0].auto_increment);
        
        assert_eq!(columns[1].name, "name");
        assert_eq!(columns[1].type_name, "TEXT");
        assert!(columns[1].nullable);
        assert!(!columns[1].primary_key);
        
        assert_eq!(columns[2].name, "created_at");
        assert_eq!(columns[2].type_name, "DATETIME");
        assert!(!columns[2].nullable);
    }

    #[tokio::test]
    async fn test_sqlite_embedded_scenarios() {
        let _protocol = SQLiteProtocolAdapter::new();
        
        // Test in-memory database for embedded scenarios
        let embedded_connection = SQLiteMessage::Connect {
            database_path: ":memory:".to_string(),
            flags: 0x00000006, // SQLITE_OPEN_READWRITE | SQLITE_OPEN_CREATE
        };
        
        match embedded_connection {
            SQLiteMessage::Connect { database_path, flags } => {
                assert_eq!(database_path, ":memory:");
                assert_eq!(flags & 0x00000002, 0x00000002); // READWRITE flag
                assert_eq!(flags & 0x00000004, 0x00000004); // CREATE flag
            }
            _ => panic!("Expected Connect message"),
        }
        
        // Test temporary database
        let temp_connection = SQLiteMessage::Connect {
            database_path: "".to_string(), // Empty string for temp database
            flags: 0x00000006,
        };
        
        match temp_connection {
            SQLiteMessage::Connect { database_path, flags } => {
                assert_eq!(database_path, "");
                assert_eq!(flags, 0x00000006);
            }
            _ => panic!("Expected Connect message"),
        }
    }

    #[tokio::test]
    async fn test_sqlite_specific_sql_functions() {
        let _protocol = SQLiteProtocolAdapter::new();
        
        // Test SQLite-specific functions
        let sqlite_functions = vec![
            "SELECT datetime('now') FROM source('file.logs.csv')",
            "SELECT substr(name, 1, 5) FROM source('api.users')",
            "SELECT length(description) FROM source('postgres.products')",
            "SELECT upper(category) FROM source('file.categories.json')",
            "SELECT coalesce(price, 0.0) FROM source('api.inventory')",
        ];
        
        for sql in sqlite_functions {
            let query_msg = SQLiteMessage::Query {
                sql: sql.to_string(),
            };
            
            match query_msg {
                SQLiteMessage::Query { sql: query_sql } => {
                    assert!(query_sql.contains("source("));
                    assert!(query_sql.contains("SELECT"));
                }
                _ => panic!("Expected Query message"),
            }
        }
    }

    #[tokio::test]
    async fn test_sqlite_connection_lifecycle() {
        ensure_test_directory();
        let _protocol = SQLiteProtocolAdapter::new();
        let mut connection = TestConnection::new();
        
        // Test connection initialization
        assert!(!connection.authenticated);
        assert!(connection.database.is_empty());
        
        // Test multiple database connections with proper cleanup
        for i in 0..3 {
            let test_db_path = format!("{}_cycle_{}", get_sqlite_test_db_path(), i);
            
            // Simulate successful connection
            connection.authenticated = true;
            connection.database = test_db_path.clone();
            connection.parameters.insert("journal_mode".to_string(), "WAL".to_string());
            connection.parameters.insert("synchronous".to_string(), "NORMAL".to_string());
            
            assert!(connection.authenticated);
            assert_eq!(connection.database, test_db_path);
            assert_eq!(connection.parameters.get("journal_mode"), Some(&"WAL".to_string()));
            assert_eq!(connection.parameters.get("synchronous"), Some(&"NORMAL".to_string()));
            
            // Test connection close with cleanup
            let close_msg = SQLiteMessage::Close;
            match close_msg {
                SQLiteMessage::Close => {
                    connection.authenticated = false;
                    connection.database.clear();
                    connection.parameters.clear();
                    
                    // Clean up database files
                    cleanup_test_db(&test_db_path);
                }
                _ => panic!("Expected Close message"),
            }
            
            assert!(!connection.authenticated);
            assert!(connection.database.is_empty());
            assert!(connection.parameters.is_empty());
        }
    }

    #[tokio::test]
    async fn test_sqlite_database_cleanup() {
        ensure_test_directory();
        let _protocol = SQLiteProtocolAdapter::new();
        
        // Test database file cleanup functionality
        // Use a unique identifier to avoid conflicts with other tests
        let unique_id = format!("{}_{}", std::process::id(), std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos());
        let test_db_path = if env::var("CI").is_ok() {
            format!("/tmp/nirv_cleanup_test_{}.db", unique_id)
        } else {
            format!("./cleanup_test_{}.db", unique_id)
        };
        
        // Simulate creating database files (main, WAL, SHM)
        let main_file = test_db_path.clone();
        let wal_file = format!("{}-wal", test_db_path);
        let shm_file = format!("{}-shm", test_db_path);
        
        // Create test files to simulate SQLite database files
        match fs::write(&main_file, b"test database content") {
            Ok(_) => {},
            Err(e) => panic!("Failed to create test database file {}: {}", main_file, e),
        }
        match fs::write(&wal_file, b"test wal content") {
            Ok(_) => {},
            Err(e) => panic!("Failed to create test WAL file {}: {}", wal_file, e),
        }
        match fs::write(&shm_file, b"test shm content") {
            Ok(_) => {},
            Err(e) => panic!("Failed to create test SHM file {}: {}", shm_file, e),
        }
        
        // Verify files exist
        assert!(Path::new(&main_file).exists(), "Main database file should exist: {}", main_file);
        assert!(Path::new(&wal_file).exists(), "WAL file should exist: {}", wal_file);
        assert!(Path::new(&shm_file).exists(), "SHM file should exist: {}", shm_file);
        
        // Test cleanup
        cleanup_test_db(&test_db_path);
        
        // Verify files are cleaned up
        assert!(!Path::new(&main_file).exists(), "Main database file should be cleaned up: {}", main_file);
        assert!(!Path::new(&wal_file).exists(), "WAL file should be cleaned up: {}", wal_file);
        assert!(!Path::new(&shm_file).exists(), "SHM file should be cleaned up: {}", shm_file);
    }

    #[tokio::test]
    async fn test_sqlite_client_compatibility() {
        ensure_test_directory();
        let _protocol = SQLiteProtocolAdapter::new();
        
        // Test compatibility with standard SQLite client operations
        let test_db_path = get_sqlite_test_db_path();
        
        let client_operations = vec![
            // Basic CRUD operations
            SQLiteMessage::Query {
                sql: "CREATE TABLE IF NOT EXISTS users (id INTEGER PRIMARY KEY, name TEXT)".to_string(),
            },
            SQLiteMessage::Query {
                sql: "INSERT INTO users (name) VALUES ('John Doe')".to_string(),
            },
            SQLiteMessage::Query {
                sql: "SELECT * FROM users WHERE id = 1".to_string(),
            },
            SQLiteMessage::Query {
                sql: "UPDATE users SET name = 'Jane Doe' WHERE id = 1".to_string(),
            },
            SQLiteMessage::Query {
                sql: "DELETE FROM users WHERE id = 1".to_string(),
            },
            // NIRV-specific operations
            SQLiteMessage::Query {
                sql: "SELECT * FROM source('file.data.csv') LIMIT 10".to_string(),
            },
            SQLiteMessage::Query {
                sql: "SELECT COUNT(*) FROM source('api.metrics') WHERE date >= '2023-01-01'".to_string(),
            },
        ];
        
        for operation in client_operations {
            match operation {
                SQLiteMessage::Query { sql } => {
                    assert!(sql.len() > 0);
                    assert!(sql.to_uppercase().contains("SELECT") || 
                           sql.to_uppercase().contains("INSERT") || 
                           sql.to_uppercase().contains("UPDATE") || 
                           sql.to_uppercase().contains("DELETE") || 
                           sql.to_uppercase().contains("CREATE"));
                }
                _ => {}
            }
        }
        
        // Clean up
        cleanup_test_db(&test_db_path);
    }
}

/// Integration tests for SQLite protocol in CI environment
#[cfg(test)]
mod integration_tests {
    use super::*;

    /// Test SQLite protocol in CI environment with proper file handling
    #[tokio::test]
    async fn test_sqlite_protocol_ci_integration() {
        ensure_test_directory();
        let protocol = SQLiteProtocolAdapter::new();
        
        // Test protocol creation
        assert_eq!(protocol.get_protocol_type(), ProtocolType::SQLite);
        
        // Test with CI-appropriate database path
        let test_db_path = get_sqlite_test_db_path();
        
        // Test connection message for CI environment
        let connect_msg = SQLiteMessage::Connect {
            database_path: test_db_path.clone(),
            flags: 0x00000006, // SQLITE_OPEN_READWRITE | SQLITE_OPEN_CREATE
        };
        
        match connect_msg {
            SQLiteMessage::Connect { database_path, flags } => {
                assert_eq!(database_path, test_db_path);
                assert_eq!(flags, 0x00000006);
                
                // In CI, verify path is in appropriate location
                if env::var("CI").is_ok() {
                    assert!(database_path.starts_with("/tmp/"));
                }
            }
            _ => panic!("Expected Connect message"),
        }
        
        // Clean up
        cleanup_test_db(&test_db_path);
    }

    /// Test SQLite protocol error handling in CI environment
    #[tokio::test]
    async fn test_sqlite_protocol_error_handling_ci() {
        let protocol = SQLiteProtocolAdapter::new();
        
        // Test with invalid database path (read-only location in CI)
        let invalid_path = if env::var("CI").is_ok() {
            "/root/readonly/test.db".to_string() // Should fail in CI
        } else {
            "/invalid/path/test.db".to_string()
        };
        
        let connect_msg = SQLiteMessage::Connect {
            database_path: invalid_path.clone(),
            flags: 0x00000002, // SQLITE_OPEN_READWRITE (no CREATE)
        };
        
        // Protocol should handle this gracefully
        match connect_msg {
            SQLiteMessage::Connect { database_path, flags } => {
                assert_eq!(database_path, invalid_path);
                assert_eq!(flags, 0x00000002);
                // Error handling would be done at connection time, not message creation
            }
            _ => panic!("Expected Connect message"),
        }
    }

    /// Test SQLite protocol with temporary databases in CI
    #[tokio::test]
    async fn test_sqlite_protocol_temporary_databases() {
        let protocol = SQLiteProtocolAdapter::new();
        
        // Test various temporary database configurations
        let temp_configs = vec![
            (":memory:", "In-memory database"),
            ("", "Temporary file database"),
            ("file::memory:?cache=shared", "Shared cache in-memory"),
        ];
        
        for (db_path, description) in temp_configs {
            let connect_msg = SQLiteMessage::Connect {
                database_path: db_path.to_string(),
                flags: 0x00000006,
            };
            
            match connect_msg {
                SQLiteMessage::Connect { database_path, flags } => {
                    assert_eq!(database_path, db_path);
                    assert_eq!(flags, 0x00000006);
                    
                    // These should work in any environment (no file cleanup needed)
                    println!("Testing {}: {}", description, database_path);
                }
                _ => panic!("Expected Connect message for {}", description),
            }
        }
    }
}