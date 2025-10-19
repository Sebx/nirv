use nirv_engine::connectors::{Connector, ConnectorInitConfig, FileConnector};
use nirv_engine::utils::{
    types::{ConnectorType, ConnectorQuery, QueryOperation, DataSource, InternalQuery, Value, DataType, Predicate, PredicateOperator, PredicateValue},
    error::{ConnectorError, NirvError},
};
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use tempfile::TempDir;

/// Helper function to create a temporary directory with test files
fn create_test_files() -> TempDir {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    
    // Create test CSV file
    let csv_content = "id,name,age,active\n1,John,25,true\n2,Jane,30,false\n3,Bob,35,true\n";
    fs::write(temp_dir.path().join("users.csv"), csv_content).expect("Failed to write CSV file");
    
    // Create test JSON file
    let json_content = r#"[
        {"id": 1, "name": "John", "age": 25, "active": true},
        {"id": 2, "name": "Jane", "age": 30, "active": false},
        {"id": 3, "name": "Bob", "age": 35, "active": true}
    ]"#;
    fs::write(temp_dir.path().join("users.json"), json_content).expect("Failed to write JSON file");
    
    // Create subdirectory with more files
    let subdir = temp_dir.path().join("data");
    fs::create_dir(&subdir).expect("Failed to create subdirectory");
    
    let csv_content2 = "product_id,product_name,price\n101,Widget,19.99\n102,Gadget,29.99\n";
    fs::write(subdir.join("products.csv"), csv_content2).expect("Failed to write products CSV");
    
    temp_dir
}

/// Helper function to create file connector config
fn create_file_config(base_path: &Path) -> ConnectorInitConfig {
    ConnectorInitConfig::new()
        .with_param("base_path", base_path.to_str().unwrap())
        .with_param("file_extensions", "csv,json")
        .with_timeout(30)
}

/// Helper function to create a test query for a specific file
fn create_file_query(file_name: &str) -> ConnectorQuery {
    let mut query = InternalQuery::new(QueryOperation::Select);
    query.sources.push(DataSource {
        object_type: "file".to_string(),
        identifier: file_name.to_string(),
        alias: None,
    });
    
    ConnectorQuery {
        connector_type: ConnectorType::File,
        query,
        connection_params: HashMap::new(),
    }
}

/// Helper function to create a test query with WHERE clause
fn create_file_query_with_where(file_name: &str, column: &str, operator: PredicateOperator, value: PredicateValue) -> ConnectorQuery {
    let mut query = InternalQuery::new(QueryOperation::Select);
    query.sources.push(DataSource {
        object_type: "file".to_string(),
        identifier: file_name.to_string(),
        alias: None,
    });
    query.predicates.push(Predicate {
        column: column.to_string(),
        operator,
        value,
    });
    
    ConnectorQuery {
        connector_type: ConnectorType::File,
        query,
        connection_params: HashMap::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_file_connector_creation() {
        let connector = FileConnector::new();
        
        assert!(!connector.is_connected());
        assert_eq!(connector.get_connector_type(), ConnectorType::File);
        assert!(!connector.supports_transactions());
        
        let capabilities = connector.get_capabilities();
        assert!(!capabilities.supports_joins); // File connector doesn't support joins across files
        assert!(capabilities.supports_aggregations);
        assert!(!capabilities.supports_subqueries);
        assert!(!capabilities.supports_transactions);
        assert!(capabilities.supports_schema_introspection);
        assert_eq!(capabilities.max_concurrent_queries, Some(1));
    }

    #[tokio::test]
    async fn test_file_connector_connection_lifecycle() {
        let temp_dir = create_test_files();
        let mut connector = FileConnector::new();
        let config = create_file_config(temp_dir.path());
        
        // Initially not connected
        assert!(!connector.is_connected());
        
        // Test connection
        let connect_result = connector.connect(config).await;
        assert!(connect_result.is_ok(), "Failed to connect: {:?}", connect_result.err());
        assert!(connector.is_connected());
        
        // Test disconnection
        let disconnect_result = connector.disconnect().await;
        assert!(disconnect_result.is_ok());
        assert!(!connector.is_connected());
    }

    #[tokio::test]
    async fn test_file_connector_invalid_base_path() {
        let mut connector = FileConnector::new();
        
        // Test with non-existent base path
        let invalid_config = ConnectorInitConfig::new()
            .with_param("base_path", "/non/existent/path/that/should/not/exist")
            .with_param("file_extensions", "csv,json");
        
        let result = connector.connect(invalid_config).await;
        assert!(result.is_err());
        
        match result.unwrap_err() {
            NirvError::Connector(ConnectorError::ConnectionFailed(_)) => {
                // Expected error for invalid path
            }
            _ => panic!("Expected ConnectionFailed error"),
        }
        
        assert!(!connector.is_connected());
    }

    #[tokio::test]
    async fn test_csv_file_parsing_and_querying() {
        let temp_dir = create_test_files();
        let mut connector = FileConnector::new();
        let config = create_file_config(temp_dir.path());
        
        let connect_result = connector.connect(config).await;
        assert!(connect_result.is_ok());
        
        // Test CSV file query
        let query = create_file_query("users.csv");
        let result = connector.execute_query(query).await;
        
        assert!(result.is_ok(), "Failed to query CSV file: {:?}", result.err());
        
        let query_result = result.unwrap();
        assert_eq!(query_result.columns.len(), 4); // id, name, age, active
        assert_eq!(query_result.rows.len(), 3); // 3 data rows
        
        // Verify column names and types
        let column_names: Vec<&str> = query_result.columns.iter().map(|c| c.name.as_str()).collect();
        assert!(column_names.contains(&"id"));
        assert!(column_names.contains(&"name"));
        assert!(column_names.contains(&"age"));
        assert!(column_names.contains(&"active"));
        
        // Verify first row data
        let first_row = &query_result.rows[0];
        assert_eq!(first_row.values.len(), 4);
        
        let _ = connector.disconnect().await;
    }

    #[tokio::test]
    async fn test_json_file_parsing_and_querying() {
        let temp_dir = create_test_files();
        let mut connector = FileConnector::new();
        let config = create_file_config(temp_dir.path());
        
        let connect_result = connector.connect(config).await;
        assert!(connect_result.is_ok());
        
        // Test JSON file query
        let query = create_file_query("users.json");
        let result = connector.execute_query(query).await;
        
        assert!(result.is_ok(), "Failed to query JSON file: {:?}", result.err());
        
        let query_result = result.unwrap();
        assert_eq!(query_result.columns.len(), 4); // id, name, age, active
        assert_eq!(query_result.rows.len(), 3); // 3 data rows
        
        // Verify column names
        let column_names: Vec<&str> = query_result.columns.iter().map(|c| c.name.as_str()).collect();
        assert!(column_names.contains(&"id"));
        assert!(column_names.contains(&"name"));
        assert!(column_names.contains(&"age"));
        assert!(column_names.contains(&"active"));
        
        let _ = connector.disconnect().await;
    }

    #[tokio::test]
    async fn test_where_clause_pushdown_csv() {
        let temp_dir = create_test_files();
        let mut connector = FileConnector::new();
        let config = create_file_config(temp_dir.path());
        
        let connect_result = connector.connect(config).await;
        assert!(connect_result.is_ok());
        
        // Test WHERE clause with equality
        let query = create_file_query_with_where(
            "users.csv",
            "name",
            PredicateOperator::Equal,
            PredicateValue::String("John".to_string())
        );
        
        let result = connector.execute_query(query).await;
        assert!(result.is_ok(), "Failed to execute WHERE query: {:?}", result.err());
        
        let query_result = result.unwrap();
        assert_eq!(query_result.rows.len(), 1); // Should only return John's row
        
        // Verify the returned row is correct
        let row = &query_result.rows[0];
        let name_index = query_result.columns.iter().position(|c| c.name == "name").unwrap();
        match &row.values[name_index] {
            Value::Text(name) => assert_eq!(name, "John"),
            _ => panic!("Expected text value for name"),
        }
        
        let _ = connector.disconnect().await;
    }

    #[tokio::test]
    async fn test_where_clause_pushdown_json() {
        let temp_dir = create_test_files();
        let mut connector = FileConnector::new();
        let config = create_file_config(temp_dir.path());
        
        let connect_result = connector.connect(config).await;
        assert!(connect_result.is_ok());
        
        // Test WHERE clause with boolean value
        let query = create_file_query_with_where(
            "users.json",
            "active",
            PredicateOperator::Equal,
            PredicateValue::Boolean(true)
        );
        
        let result = connector.execute_query(query).await;
        assert!(result.is_ok(), "Failed to execute WHERE query on JSON: {:?}", result.err());
        
        let query_result = result.unwrap();
        assert_eq!(query_result.rows.len(), 2); // Should return John and Bob (active=true)
        
        let _ = connector.disconnect().await;
    }

    #[tokio::test]
    async fn test_where_clause_numeric_comparison() {
        let temp_dir = create_test_files();
        let mut connector = FileConnector::new();
        let config = create_file_config(temp_dir.path());
        
        let connect_result = connector.connect(config).await;
        assert!(connect_result.is_ok());
        
        // Test WHERE clause with greater than
        let query = create_file_query_with_where(
            "users.csv",
            "age",
            PredicateOperator::GreaterThan,
            PredicateValue::Integer(30)
        );
        
        let result = connector.execute_query(query).await;
        assert!(result.is_ok(), "Failed to execute numeric WHERE query: {:?}", result.err());
        
        let query_result = result.unwrap();
        assert_eq!(query_result.rows.len(), 1); // Should only return Bob (age=35)
        
        let _ = connector.disconnect().await;
    }

    #[tokio::test]
    async fn test_directory_scanning_and_file_discovery() {
        let temp_dir = create_test_files();
        let mut connector = FileConnector::new();
        let config = create_file_config(temp_dir.path());
        
        let connect_result = connector.connect(config).await;
        assert!(connect_result.is_ok());
        
        // Test querying file in subdirectory
        let query = create_file_query("data/products.csv");
        let result = connector.execute_query(query).await;
        
        assert!(result.is_ok(), "Failed to query file in subdirectory: {:?}", result.err());
        
        let query_result = result.unwrap();
        assert_eq!(query_result.columns.len(), 3); // product_id, product_name, price
        assert_eq!(query_result.rows.len(), 2); // 2 products
        
        // Verify column names
        let column_names: Vec<&str> = query_result.columns.iter().map(|c| c.name.as_str()).collect();
        assert!(column_names.contains(&"product_id"));
        assert!(column_names.contains(&"product_name"));
        assert!(column_names.contains(&"price"));
        
        let _ = connector.disconnect().await;
    }

    #[tokio::test]
    async fn test_file_pattern_matching() {
        let temp_dir = create_test_files();
        
        // Create additional files with patterns
        let pattern_content = "test_id,test_value\n1,alpha\n2,beta\n";
        fs::write(temp_dir.path().join("test_data_001.csv"), pattern_content).expect("Failed to write pattern file");
        fs::write(temp_dir.path().join("test_data_002.csv"), pattern_content).expect("Failed to write pattern file");
        
        let mut connector = FileConnector::new();
        let config = create_file_config(temp_dir.path());
        
        let connect_result = connector.connect(config).await;
        assert!(connect_result.is_ok());
        
        // Test querying with wildcard pattern
        let query = create_file_query("test_data_*.csv");
        let result = connector.execute_query(query).await;
        
        // This should either succeed (if pattern matching is implemented) or fail gracefully
        match result {
            Ok(query_result) => {
                // If pattern matching works, we should get combined results
                assert!(query_result.rows.len() >= 2); // At least 2 rows from one file
            }
            Err(NirvError::Connector(ConnectorError::QueryExecutionFailed(_))) => {
                // Pattern matching not implemented yet - acceptable
            }
            Err(other) => {
                panic!("Unexpected error for pattern query: {:?}", other);
            }
        }
        
        let _ = connector.disconnect().await;
    }

    #[tokio::test]
    async fn test_schema_introspection_csv() {
        let temp_dir = create_test_files();
        let mut connector = FileConnector::new();
        let config = create_file_config(temp_dir.path());
        
        let connect_result = connector.connect(config).await;
        assert!(connect_result.is_ok());
        
        // Test schema retrieval for CSV file
        let schema_result = connector.get_schema("users.csv").await;
        assert!(schema_result.is_ok(), "Failed to get CSV schema: {:?}", schema_result.err());
        
        let schema = schema_result.unwrap();
        assert_eq!(schema.name, "users.csv");
        assert_eq!(schema.columns.len(), 4);
        
        // Verify column names
        let column_names: Vec<&str> = schema.columns.iter().map(|c| c.name.as_str()).collect();
        assert!(column_names.contains(&"id"));
        assert!(column_names.contains(&"name"));
        assert!(column_names.contains(&"age"));
        assert!(column_names.contains(&"active"));
        
        let _ = connector.disconnect().await;
    }

    #[tokio::test]
    async fn test_schema_introspection_json() {
        let temp_dir = create_test_files();
        let mut connector = FileConnector::new();
        let config = create_file_config(temp_dir.path());
        
        let connect_result = connector.connect(config).await;
        assert!(connect_result.is_ok());
        
        // Test schema retrieval for JSON file
        let schema_result = connector.get_schema("users.json").await;
        assert!(schema_result.is_ok(), "Failed to get JSON schema: {:?}", schema_result.err());
        
        let schema = schema_result.unwrap();
        assert_eq!(schema.name, "users.json");
        assert_eq!(schema.columns.len(), 4);
        
        // Verify column names
        let column_names: Vec<&str> = schema.columns.iter().map(|c| c.name.as_str()).collect();
        assert!(column_names.contains(&"id"));
        assert!(column_names.contains(&"name"));
        assert!(column_names.contains(&"age"));
        assert!(column_names.contains(&"active"));
        
        let _ = connector.disconnect().await;
    }

    #[tokio::test]
    async fn test_query_without_connection() {
        let connector = FileConnector::new();
        let query = create_file_query("users.csv");
        
        let result = connector.execute_query(query).await;
        assert!(result.is_err());
        
        match result.unwrap_err() {
            NirvError::Connector(ConnectorError::ConnectionFailed(_)) => {
                // Expected when not connected
            }
            _ => panic!("Expected ConnectionFailed error"),
        }
    }

    #[tokio::test]
    async fn test_schema_retrieval_without_connection() {
        let connector = FileConnector::new();
        
        let result = connector.get_schema("users.csv").await;
        assert!(result.is_err());
        
        match result.unwrap_err() {
            NirvError::Connector(ConnectorError::ConnectionFailed(_)) => {
                // Expected when not connected
            }
            _ => panic!("Expected ConnectionFailed error"),
        }
    }

    #[tokio::test]
    async fn test_non_existent_file_query() {
        let temp_dir = create_test_files();
        let mut connector = FileConnector::new();
        let config = create_file_config(temp_dir.path());
        
        let connect_result = connector.connect(config).await;
        assert!(connect_result.is_ok());
        
        // Test querying non-existent file
        let query = create_file_query("non_existent_file.csv");
        let result = connector.execute_query(query).await;
        
        assert!(result.is_err());
        match result.unwrap_err() {
            NirvError::Connector(ConnectorError::QueryExecutionFailed(_)) => {
                // Expected for non-existent file
            }
            _ => panic!("Expected QueryExecutionFailed error for non-existent file"),
        }
        
        let _ = connector.disconnect().await;
    }

    #[tokio::test]
    async fn test_unsupported_file_format() {
        let temp_dir = create_test_files();
        
        // Create unsupported file format
        fs::write(temp_dir.path().join("data.txt"), "some text content").expect("Failed to write text file");
        
        let mut connector = FileConnector::new();
        let config = create_file_config(temp_dir.path());
        
        let connect_result = connector.connect(config).await;
        assert!(connect_result.is_ok());
        
        // Test querying unsupported file format
        let query = create_file_query("data.txt");
        let result = connector.execute_query(query).await;
        
        assert!(result.is_err());
        match result.unwrap_err() {
            NirvError::Connector(ConnectorError::UnsupportedOperation(_)) => {
                // Expected for unsupported file format
            }
            NirvError::Connector(ConnectorError::QueryExecutionFailed(_)) => {
                // Also acceptable
            }
            _ => panic!("Expected UnsupportedOperation or QueryExecutionFailed error"),
        }
        
        let _ = connector.disconnect().await;
    }

    #[test]
    fn test_file_connector_capabilities() {
        let connector = FileConnector::new();
        let capabilities = connector.get_capabilities();
        
        // File connector should have limited capabilities
        assert!(!capabilities.supports_joins); // No cross-file joins
        assert!(capabilities.supports_aggregations); // Basic aggregations
        assert!(!capabilities.supports_subqueries); // No subqueries
        assert!(!capabilities.supports_transactions); // No transactions
        assert!(capabilities.supports_schema_introspection); // Schema from file headers
        
        // Should support only single concurrent query
        assert_eq!(capabilities.max_concurrent_queries, Some(1));
    }

    #[test]
    fn test_file_connector_type() {
        let connector = FileConnector::new();
        assert_eq!(connector.get_connector_type(), ConnectorType::File);
    }
}

/// Performance tests for file connector optimization
#[cfg(test)]
mod performance_tests {
    use super::*;

    #[tokio::test]
    async fn test_where_clause_pushdown_performance() {
        let temp_dir = create_test_files();
        
        // Create larger test file for performance testing
        let mut large_csv_content = "id,name,category,value\n".to_string();
        for i in 1..=1000 {
            large_csv_content.push_str(&format!("{},Item{},Category{},{}\n", i, i, i % 10, i * 10));
        }
        fs::write(temp_dir.path().join("large_data.csv"), large_csv_content).expect("Failed to write large CSV");
        
        let mut connector = FileConnector::new();
        let config = create_file_config(temp_dir.path());
        
        let connect_result = connector.connect(config).await;
        assert!(connect_result.is_ok());
        
        // Test WHERE clause pushdown - should be faster than loading all data
        let query = create_file_query_with_where(
            "large_data.csv",
            "category",
            PredicateOperator::Equal,
            PredicateValue::String("Category5".to_string())
        );
        
        let start_time = std::time::Instant::now();
        let result = connector.execute_query(query).await;
        let execution_time = start_time.elapsed();
        
        assert!(result.is_ok(), "Failed to execute performance test query: {:?}", result.err());
        
        let query_result = result.unwrap();
        // Should return approximately 100 rows (1000 / 10 categories)
        assert!(query_result.rows.len() >= 90 && query_result.rows.len() <= 110);
        
        // Execution should be reasonably fast (less than 1 second for this small test)
        assert!(execution_time.as_secs() < 1, "Query took too long: {:?}", execution_time);
        
        let _ = connector.disconnect().await;
    }
}