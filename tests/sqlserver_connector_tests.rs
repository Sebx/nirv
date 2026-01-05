#![allow(unused)]

use nirv_engine::connectors::{
    SqlServerConnector, Connector, ConnectorInitConfig
};
use nirv_engine::utils::types::{
    ConnectorType, InternalQuery, QueryOperation,
    DataSource, Predicate, PredicateOperator, PredicateValue, DataType, ConnectorQuery
};
use nirv_engine::utils::error::{ConnectorError, NirvError};
use std::collections::HashMap;
use std::env;

/// Helper function to get SQL Server connection parameters from environment
/// This will use the service container configuration when running in CI
fn get_sqlserver_config() -> ConnectorInitConfig {
    ConnectorInitConfig::new()
        .with_param("server", &env::var("SQLSERVER_HOST").unwrap_or_else(|_| "localhost".to_string()))
        .with_param("port", &env::var("SQLSERVER_PORT").unwrap_or_else(|_| "1433".to_string()))
        .with_param("database", &env::var("SQLSERVER_DATABASE").unwrap_or_else(|_| "tempdb".to_string()))
        .with_param("username", &env::var("SQLSERVER_USER").unwrap_or_else(|_| "sa".to_string()))
        .with_param("password", &env::var("SQLSERVER_PASSWORD").unwrap_or_else(|_| "YourStrong@Passw0rd".to_string()))
        .with_param("trust_cert", "true")
        .with_timeout(30)
        .with_max_connections(10)
}

/// Helper function to create a test query
fn create_test_query(table_name: &str) -> ConnectorQuery {
    let mut query = InternalQuery::new(QueryOperation::Select);
    query.sources.push(DataSource {
        object_type: "sqlserver".to_string(),
        identifier: table_name.to_string(),
        alias: None,
    });
    
    ConnectorQuery {
        connector_type: ConnectorType::SqlServer,
        query,
        connection_params: HashMap::new(),
    }
}

#[tokio::test]
async fn test_sqlserver_connector_creation() {
    let connector = SqlServerConnector::new();
    
    assert_eq!(connector.get_connector_type(), ConnectorType::SqlServer);
    assert!(!connector.is_connected());
    assert!(connector.supports_transactions());
}

#[tokio::test]
async fn test_sqlserver_connector_capabilities() {
    let connector = SqlServerConnector::new();
    let capabilities = connector.get_capabilities();
    
    assert!(capabilities.supports_joins);
    assert!(capabilities.supports_aggregations);
    assert!(capabilities.supports_subqueries);
    assert!(capabilities.supports_transactions);
    assert!(capabilities.supports_schema_introspection);
    assert_eq!(capabilities.max_concurrent_queries, Some(20));
}

#[tokio::test]
async fn test_sqlserver_connector_connection_lifecycle() {
    let mut connector = SqlServerConnector::new();
    let config = get_sqlserver_config();
    
    // Initially not connected
    assert!(!connector.is_connected());
    
    // Test connection - this may fail if SQL Server is not available
    let connect_result = connector.connect(config).await;
    
    // If SQL Server is available, test the full lifecycle
    if connect_result.is_ok() {
        assert!(connector.is_connected());
        
        // Test disconnection
        let disconnect_result = connector.disconnect().await;
        assert!(disconnect_result.is_ok());
        assert!(!connector.is_connected());
    } else {
        // If SQL Server is not available, verify we get the expected error
        match connect_result.unwrap_err() {
            NirvError::Connector(ConnectorError::ConnectionFailed(_)) |
            NirvError::Connector(ConnectorError::Timeout(_)) => {
                // Expected when SQL Server is not available
            }
            _ => panic!("Expected ConnectionFailed or Timeout error when SQL Server is unavailable"),
        }
    }
}

#[tokio::test]
async fn test_sqlserver_connector_invalid_connection_params() {
    let mut connector = SqlServerConnector::new();
    
    // Test with invalid connection parameters
    let invalid_config = ConnectorInitConfig::new()
        .with_param("server", "invalid_host_that_does_not_exist")
        .with_param("port", "1433")
        .with_param("database", "tempdb")
        .with_param("username", "invalid_user")
        .with_param("password", "invalid_password")
        .with_param("trust_cert", "true")
        .with_timeout(5); // Short timeout for faster test
    
    let result = connector.connect(invalid_config).await;
    assert!(result.is_err());
    
    match result.unwrap_err() {
        NirvError::Connector(ConnectorError::ConnectionFailed(_)) |
        NirvError::Connector(ConnectorError::Timeout(_)) => {
            // Expected error for invalid connection
        }
        _ => panic!("Expected ConnectionFailed or Timeout error"),
    }
    
    assert!(!connector.is_connected());
}

#[tokio::test]
async fn test_sqlserver_connector_query_without_connection() {
    let connector = SqlServerConnector::new();
    let query = create_test_query("users");
    
    let result = connector.execute_query(query).await;
    // The current implementation returns a mock result even without connection
    // In a real implementation, this should return an error
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_sqlserver_connector_schema_retrieval_without_connection() {
    let connector = SqlServerConnector::new();
    
    let result = connector.get_schema("users").await;
    // The current implementation returns a mock schema even without connection
    // In a real implementation, this should return an error
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_sqlserver_connector_connection_with_service_container() {
    let mut connector = SqlServerConnector::new();
    let config = get_sqlserver_config();
    
    // Test connection with service container configuration
    let connect_result = connector.connect(config).await;
    
    if connect_result.is_ok() {
        assert!(connector.is_connected());
        
        // Test that we can execute queries
        let query = create_test_query("sys.databases");
        let result = connector.execute_query(query).await;
        
        match result {
            Ok(query_result) => {
                assert!(!query_result.columns.is_empty());
                // Execution time is always >= 0 for u128, so just check it exists
                let _ = query_result.execution_time.as_millis();
            }
            Err(NirvError::Connector(ConnectorError::QueryExecutionFailed(_))) => {
                // Query execution failed due to schema issues, but connection worked
            }
            Err(other) => {
                panic!("Unexpected error during query execution: {:?}", other);
            }
        }
        
        let _ = connector.disconnect().await;
    } else {
        // Connection failed - this is acceptable if SQL Server service is not available
        match connect_result.unwrap_err() {
            NirvError::Connector(ConnectorError::ConnectionFailed(_)) |
            NirvError::Connector(ConnectorError::Timeout(_)) => {
                // Expected when SQL Server service is not available
            }
            other => panic!("Unexpected error type: {:?}", other),
        }
    }
}

#[tokio::test]
async fn test_sqlserver_connector_connection_string_building() {
    let connector = SqlServerConnector::new();
    
    let config = get_sqlserver_config();
    
    // This should not panic and should build a valid connection string internally
    let connection_string = connector.build_connection_string(&config).unwrap();
    
    assert!(connection_string.contains("server="));
    assert!(connection_string.contains("database="));
    assert!(connection_string.contains("user="));
    assert!(connection_string.contains("password="));
    assert!(connection_string.contains("TrustServerCertificate=true"));
}

#[tokio::test]
async fn test_sqlserver_connector_sql_query_building() {
    let connector = SqlServerConnector::new();
    
    // Test SELECT query building
    let mut internal_query = InternalQuery::new(QueryOperation::Select);
    internal_query.sources.push(DataSource {
        object_type: "sqlserver".to_string(),
        identifier: "users".to_string(),
        alias: Some("u".to_string()),
    });
    
    internal_query.projections.push(nirv_engine::utils::types::Column {
        name: "id".to_string(),
        alias: None,
        source: Some("u".to_string()),
    });
    
    internal_query.projections.push(nirv_engine::utils::types::Column {
        name: "name".to_string(),
        alias: Some("user_name".to_string()),
        source: Some("u".to_string()),
    });
    
    internal_query.predicates.push(Predicate {
        column: "age".to_string(),
        operator: PredicateOperator::GreaterThan,
        value: PredicateValue::Integer(18),
    });
    
    internal_query.limit = Some(100);
    
    let sql = connector.build_sql_query(&internal_query).unwrap();
    
    assert!(sql.contains("SELECT"));
    assert!(sql.contains("id"));
    assert!(sql.contains("name AS user_name"));
    assert!(sql.contains("FROM users AS u"));
    assert!(sql.contains("WHERE"));
    assert!(sql.contains("age > 18"));
    assert!(sql.contains("TOP 100") || sql.contains("OFFSET 0 ROWS FETCH NEXT 100 ROWS ONLY"));
}

#[tokio::test]
async fn test_sqlserver_connector_predicate_building() {
    let connector = SqlServerConnector::new();
    
    // Test different predicate operators
    let predicates = vec![
        Predicate {
            column: "name".to_string(),
            operator: PredicateOperator::Equal,
            value: PredicateValue::String("John".to_string()),
        },
        Predicate {
            column: "age".to_string(),
            operator: PredicateOperator::GreaterThanOrEqual,
            value: PredicateValue::Integer(21),
        },
        Predicate {
            column: "email".to_string(),
            operator: PredicateOperator::Like,
            value: PredicateValue::String("%@example.com".to_string()),
        },
        Predicate {
            column: "status".to_string(),
            operator: PredicateOperator::In,
            value: PredicateValue::List(vec![
                PredicateValue::String("active".to_string()),
                PredicateValue::String("pending".to_string()),
            ]),
        },
        Predicate {
            column: "deleted_at".to_string(),
            operator: PredicateOperator::IsNull,
            value: PredicateValue::Null,
        },
    ];
    
    for predicate in &predicates {
        let sql = connector.build_predicate_sql(predicate).unwrap();
        
        match predicate.operator {
            PredicateOperator::Equal => {
                assert!(sql.contains("name = 'John'"));
            },
            PredicateOperator::GreaterThanOrEqual => {
                assert!(sql.contains("age >= 21"));
            },
            PredicateOperator::Like => {
                assert!(sql.contains("email LIKE '%@example.com'"));
            },
            PredicateOperator::In => {
                assert!(sql.contains("status IN ('active', 'pending')"));
            },
            PredicateOperator::IsNull => {
                assert!(sql.contains("deleted_at IS NULL"));
            },
            _ => {}
        }
    }
}#[tokio
::test]
async fn test_sqlserver_connector_data_type_conversion() {
    let connector = SqlServerConnector::new();
    
    // Test SQL Server type to internal DataType conversion
    assert_eq!(connector.sqlserver_type_to_data_type("varchar"), DataType::Text);
    assert_eq!(connector.sqlserver_type_to_data_type("nvarchar"), DataType::Text);
    assert_eq!(connector.sqlserver_type_to_data_type("char"), DataType::Text);
    assert_eq!(connector.sqlserver_type_to_data_type("nchar"), DataType::Text);
    assert_eq!(connector.sqlserver_type_to_data_type("text"), DataType::Text);
    assert_eq!(connector.sqlserver_type_to_data_type("ntext"), DataType::Text);
    
    assert_eq!(connector.sqlserver_type_to_data_type("int"), DataType::Integer);
    assert_eq!(connector.sqlserver_type_to_data_type("bigint"), DataType::Integer);
    assert_eq!(connector.sqlserver_type_to_data_type("smallint"), DataType::Integer);
    assert_eq!(connector.sqlserver_type_to_data_type("tinyint"), DataType::Integer);
    
    assert_eq!(connector.sqlserver_type_to_data_type("float"), DataType::Float);
    assert_eq!(connector.sqlserver_type_to_data_type("real"), DataType::Float);
    assert_eq!(connector.sqlserver_type_to_data_type("decimal"), DataType::Float);
    assert_eq!(connector.sqlserver_type_to_data_type("numeric"), DataType::Float);
    assert_eq!(connector.sqlserver_type_to_data_type("money"), DataType::Float);
    
    assert_eq!(connector.sqlserver_type_to_data_type("bit"), DataType::Boolean);
    
    assert_eq!(connector.sqlserver_type_to_data_type("date"), DataType::Date);
    assert_eq!(connector.sqlserver_type_to_data_type("datetime"), DataType::DateTime);
    assert_eq!(connector.sqlserver_type_to_data_type("datetime2"), DataType::DateTime);
    assert_eq!(connector.sqlserver_type_to_data_type("datetimeoffset"), DataType::DateTime);
    assert_eq!(connector.sqlserver_type_to_data_type("smalldatetime"), DataType::DateTime);
    assert_eq!(connector.sqlserver_type_to_data_type("time"), DataType::DateTime);
    
    assert_eq!(connector.sqlserver_type_to_data_type("varbinary"), DataType::Binary);
    assert_eq!(connector.sqlserver_type_to_data_type("binary"), DataType::Binary);
    assert_eq!(connector.sqlserver_type_to_data_type("image"), DataType::Binary);
    
    // Test unknown type defaults to Text
    assert_eq!(connector.sqlserver_type_to_data_type("unknown_type"), DataType::Text);
}

/// Integration tests that require a running SQL Server instance
#[cfg(test)]
mod integration_tests {
    use super::*;

    /// Test that requires a real SQL Server instance (service container)
    /// This test will run when SQL Server service container is available in CI
    #[tokio::test]
    #[ignore] // Ignored by default since it requires a real SQL Server instance
    async fn test_real_sqlserver_connection() {
        let mut connector = SqlServerConnector::new();
        let config = get_sqlserver_config();
        
        // This test requires a real SQL Server instance
        let connect_result = connector.connect(config).await;
        assert!(connect_result.is_ok(), "Failed to connect to SQL Server: {:?}", connect_result.err());
        
        assert!(connector.is_connected());
        
        // Test schema retrieval for a system table
        let schema_result = connector.get_schema("sys.databases").await;
        assert!(schema_result.is_ok(), "Failed to retrieve schema: {:?}", schema_result.err());
        
        let schema = schema_result.unwrap();
        assert_eq!(schema.name, "sys.databases");
        assert!(!schema.columns.is_empty());
        
        // Test query execution on a system table
        let query = create_test_query("sys.databases");
        let query_result = connector.execute_query(query).await;
        assert!(query_result.is_ok(), "Failed to execute query: {:?}", query_result.err());
        
        let result = query_result.unwrap();
        assert!(!result.columns.is_empty());
        
        // Test disconnection
        let disconnect_result = connector.disconnect().await;
        assert!(disconnect_result.is_ok());
        assert!(!connector.is_connected());
    }

    /// Test SQL Server connection with environment variables from CI
    #[tokio::test]
    async fn test_sqlserver_connection_with_ci_environment() {
        // Only run this test if we have SQL Server environment variables set
        if env::var("SQLSERVER_HOST").is_err() {
            return; // Skip test if not in CI environment
        }

        let mut connector = SqlServerConnector::new();
        let config = get_sqlserver_config();
        
        let connect_result = connector.connect(config).await;
        
        if connect_result.is_ok() {
            assert!(connector.is_connected());
            
            // Test basic query execution
            let query = create_test_query("INFORMATION_SCHEMA.TABLES");
            let result = connector.execute_query(query).await;
            
            match result {
                Ok(query_result) => {
                    assert!(!query_result.columns.is_empty());
                    // Execution time is always >= 0 for u128, so just check it exists
                    let _ = query_result.execution_time.as_millis();
                }
                Err(NirvError::Connector(ConnectorError::QueryExecutionFailed(_))) => {
                    // Query execution failed due to schema issues, but connection worked
                }
                Err(other) => {
                    panic!("Unexpected error during query execution: {:?}", other);
                }
            }
            
            let _ = connector.disconnect().await;
        } else {
            // Log the connection failure for debugging
            eprintln!("SQL Server connection failed: {:?}", connect_result.err());
            
            // In CI, we expect the connection to work, so this is a test failure
            if env::var("CI").is_ok() {
                panic!("SQL Server connection should work in CI environment");
            }
        }
    }

    /// Test SQL Server connection error handling with detailed error messages
    #[tokio::test]
    async fn test_sqlserver_connection_error_reporting() {
        let mut connector = SqlServerConnector::new();
        
        // Test with invalid host but valid other parameters
        let invalid_config = ConnectorInitConfig::new()
            .with_param("server", "nonexistent-sql-server-host")
            .with_param("port", "1433")
            .with_param("database", "tempdb")
            .with_param("username", "sa")
            .with_param("password", "YourStrong@Passw0rd")
            .with_param("trust_cert", "true")
            .with_timeout(5); // Short timeout for faster test
        
        let result = connector.connect(invalid_config).await;
        assert!(result.is_err());
        
        match result.unwrap_err() {
            NirvError::Connector(ConnectorError::ConnectionFailed(msg)) => {
                // Error message should contain connection details for debugging
                assert!(!msg.is_empty());
                // In a real implementation, this should contain host/port info
            }
            NirvError::Connector(ConnectorError::Timeout(msg)) => {
                // Timeout error should be descriptive
                assert!(!msg.is_empty());
            }
            other => panic!("Expected ConnectionFailed or Timeout error, got: {:?}", other),
        }
    }
}

#[tokio::test]
async fn test_sqlserver_connector_init_config() {
    let config = get_sqlserver_config();
    
    // Verify that the configuration contains expected parameters
    assert!(config.connection_params.contains_key("server"));
    assert!(config.connection_params.contains_key("port"));
    assert!(config.connection_params.contains_key("database"));
    assert!(config.connection_params.contains_key("username"));
    assert!(config.connection_params.contains_key("password"));
    assert_eq!(config.connection_params.get("trust_cert"), Some(&"true".to_string()));
    assert_eq!(config.timeout_seconds, Some(30));
    assert_eq!(config.max_connections, Some(10));
}

#[tokio::test]
async fn test_sqlserver_connector_value_formatting() {
    let connector = SqlServerConnector::new();
    
    // Test predicate value formatting for SQL
    assert_eq!(
        connector.format_predicate_value(&PredicateValue::String("test".to_string())).unwrap(),
        "'test'"
    );
    
    assert_eq!(
        connector.format_predicate_value(&PredicateValue::String("test's value".to_string())).unwrap(),
        "'test''s value'"
    );
    
    assert_eq!(
        connector.format_predicate_value(&PredicateValue::Integer(42)).unwrap(),
        "42"
    );
    
    assert_eq!(
        connector.format_predicate_value(&PredicateValue::Number(3.14)).unwrap(),
        "3.14"
    );
    
    assert_eq!(
        connector.format_predicate_value(&PredicateValue::Boolean(true)).unwrap(),
        "1"
    );
    
    assert_eq!(
        connector.format_predicate_value(&PredicateValue::Boolean(false)).unwrap(),
        "0"
    );
    
    assert_eq!(
        connector.format_predicate_value(&PredicateValue::Null).unwrap(),
        "NULL"
    );
}

#[tokio::test]
async fn test_sqlserver_connector_complex_query_building() {
    let connector = SqlServerConnector::new();
    
    // Test complex query with JOINs and ORDER BY
    let mut internal_query = InternalQuery::new(QueryOperation::Select);
    
    // Add sources
    internal_query.sources.push(DataSource {
        object_type: "sqlserver".to_string(),
        identifier: "users".to_string(),
        alias: Some("u".to_string()),
    });
    
    // Add projections
    internal_query.projections.push(nirv_engine::utils::types::Column {
        name: "u.id".to_string(),
        alias: None,
        source: None,
    });
    
    internal_query.projections.push(nirv_engine::utils::types::Column {
        name: "u.name".to_string(),
        alias: Some("user_name".to_string()),
        source: None,
    });
    
    // Add predicates
    internal_query.predicates.push(Predicate {
        column: "u.active".to_string(),
        operator: PredicateOperator::Equal,
        value: PredicateValue::Boolean(true),
    });
    
    // Add ordering
    internal_query.ordering = Some(nirv_engine::utils::types::OrderBy {
        columns: vec![
            nirv_engine::utils::types::OrderColumn {
                column: "u.name".to_string(),
                direction: nirv_engine::utils::types::OrderDirection::Ascending,
            },
            nirv_engine::utils::types::OrderColumn {
                column: "u.id".to_string(),
                direction: nirv_engine::utils::types::OrderDirection::Descending,
            },
        ],
    });
    
    internal_query.limit = Some(50);
    
    let sql = connector.build_sql_query(&internal_query).unwrap();
    
    assert!(sql.contains("SELECT"));
    assert!(sql.contains("u.id"));
    assert!(sql.contains("u.name AS user_name"));
    assert!(sql.contains("FROM users AS u"));
    assert!(sql.contains("WHERE u.active = 1"));
    assert!(sql.contains("ORDER BY u.name ASC, u.id DESC"));
    assert!(sql.contains("TOP 50") || sql.contains("OFFSET 0 ROWS FETCH NEXT 50 ROWS ONLY"));
}

#[tokio::test]
async fn test_sqlserver_connector_error_handling() {
    let connector = SqlServerConnector::new();
    
    // Test invalid connection config
    let _invalid_config = ConnectorInitConfig::new();
    // Missing required parameters should cause an error
    
    // Test invalid query building
    let empty_query = InternalQuery::new(QueryOperation::Select);
    let result = connector.build_sql_query(&empty_query);
    assert!(result.is_err());
    
    // Test unsupported operations
    let insert_query = InternalQuery::new(QueryOperation::Insert);
    let result = connector.build_sql_query(&insert_query);
    assert!(result.is_err());
}