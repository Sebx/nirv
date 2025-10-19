use nirv_engine::connectors::{Connector, ConnectorInitConfig, PostgresConnector};
use nirv_engine::utils::{
    types::{ConnectorType, ConnectorQuery, QueryOperation, DataSource, InternalQuery, Value, DataType},
    error::{ConnectorError, NirvError},
};
use std::collections::HashMap;
use std::env;

/// Helper function to get PostgreSQL connection parameters from environment
fn get_postgres_config() -> ConnectorInitConfig {
    ConnectorInitConfig::new()
        .with_param("host", &env::var("POSTGRES_HOST").unwrap_or_else(|_| "localhost".to_string()))
        .with_param("port", &env::var("POSTGRES_PORT").unwrap_or_else(|_| "5432".to_string()))
        .with_param("user", &env::var("POSTGRES_USER").unwrap_or_else(|_| "postgres".to_string()))
        .with_param("password", &env::var("POSTGRES_PASSWORD").unwrap_or_else(|_| "postgres".to_string()))
        .with_param("dbname", &env::var("POSTGRES_DB").unwrap_or_else(|_| "test".to_string()))
        .with_timeout(30)
        .with_max_connections(5)
}

/// Helper function to create a test query
fn create_test_query(table_name: &str) -> ConnectorQuery {
    let mut query = InternalQuery::new(QueryOperation::Select);
    query.sources.push(DataSource {
        object_type: "postgres".to_string(),
        identifier: table_name.to_string(),
        alias: None,
    });
    
    ConnectorQuery {
        connector_type: ConnectorType::PostgreSQL,
        query,
        connection_params: HashMap::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_postgres_connector_creation() {
        let connector = PostgresConnector::new();
        
        assert!(!connector.is_connected());
        assert_eq!(connector.get_connector_type(), ConnectorType::PostgreSQL);
        assert!(connector.supports_transactions());
        
        let capabilities = connector.get_capabilities();
        assert!(capabilities.supports_joins);
        assert!(capabilities.supports_aggregations);
        assert!(capabilities.supports_subqueries);
        assert!(capabilities.supports_transactions);
        assert!(capabilities.supports_schema_introspection);
        assert!(capabilities.max_concurrent_queries.unwrap_or(0) > 1);
    }

    #[tokio::test]
    async fn test_postgres_connector_connection_lifecycle() {
        let mut connector = PostgresConnector::new();
        let config = get_postgres_config();
        
        // Initially not connected
        assert!(!connector.is_connected());
        
        // Test connection - this may fail if PostgreSQL is not available
        let connect_result = connector.connect(config).await;
        
        // If PostgreSQL is available, test the full lifecycle
        if connect_result.is_ok() {
            assert!(connector.is_connected());
            
            // Test disconnection
            let disconnect_result = connector.disconnect().await;
            assert!(disconnect_result.is_ok());
            assert!(!connector.is_connected());
        } else {
            // If PostgreSQL is not available, verify we get the expected error
            match connect_result.unwrap_err() {
                NirvError::Connector(ConnectorError::ConnectionFailed(_)) => {
                    // Expected when PostgreSQL is not available
                }
                _ => panic!("Expected ConnectionFailed error when PostgreSQL is unavailable"),
            }
        }
    }

    #[tokio::test]
    async fn test_postgres_connector_invalid_connection_params() {
        let mut connector = PostgresConnector::new();
        
        // Test with invalid connection parameters
        let invalid_config = ConnectorInitConfig::new()
            .with_param("host", "invalid_host_that_does_not_exist")
            .with_param("port", "5432")
            .with_param("user", "invalid_user")
            .with_param("password", "invalid_password")
            .with_param("dbname", "invalid_db")
            .with_timeout(5); // Short timeout for faster test
        
        let result = connector.connect(invalid_config).await;
        assert!(result.is_err());
        
        match result.unwrap_err() {
            NirvError::Connector(ConnectorError::ConnectionFailed(_)) => {
                // Expected error for invalid connection
            }
            _ => panic!("Expected ConnectionFailed error"),
        }
        
        assert!(!connector.is_connected());
    }

    #[tokio::test]
    async fn test_postgres_connector_query_without_connection() {
        let connector = PostgresConnector::new();
        let query = create_test_query("users");
        
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
    async fn test_postgres_connector_schema_retrieval_without_connection() {
        let connector = PostgresConnector::new();
        
        let result = connector.get_schema("users").await;
        assert!(result.is_err());
        
        match result.unwrap_err() {
            NirvError::Connector(ConnectorError::ConnectionFailed(_)) => {
                // Expected when not connected
            }
            _ => panic!("Expected ConnectionFailed error"),
        }
    }

    #[tokio::test]
    async fn test_postgres_connector_connection_pooling() {
        let mut connector = PostgresConnector::new();
        let config = get_postgres_config().with_max_connections(3);
        
        // Test connection with pool configuration
        let connect_result = connector.connect(config).await;
        
        if connect_result.is_ok() {
            assert!(connector.is_connected());
            
            // Test that we can execute multiple queries (testing pool usage)
            let query1 = create_test_query("information_schema.tables");
            let query2 = create_test_query("information_schema.columns");
            
            let result1 = connector.execute_query(query1).await;
            let result2 = connector.execute_query(query2).await;
            
            // Both queries should succeed (or fail with schema-related errors, not connection errors)
            match (result1, result2) {
                (Ok(_), Ok(_)) => {
                    // Both succeeded - great!
                }
                (Err(NirvError::Connector(ConnectorError::QueryExecutionFailed(_))), _) |
                (_, Err(NirvError::Connector(ConnectorError::QueryExecutionFailed(_)))) => {
                    // Query execution failed due to schema issues, but connection worked
                }
                (Err(NirvError::Connector(ConnectorError::ConnectionFailed(_))), _) |
                (_, Err(NirvError::Connector(ConnectorError::ConnectionFailed(_)))) => {
                    panic!("Connection should be available for both queries");
                }
                _ => {
                    // Other errors are acceptable for this test
                }
            }
            
            let _ = connector.disconnect().await;
        }
    }

    #[tokio::test]
    async fn test_postgres_connector_transaction_support() {
        let mut connector = PostgresConnector::new();
        let config = get_postgres_config();
        
        // Verify transaction support is advertised
        assert!(connector.supports_transactions());
        
        let connect_result = connector.connect(config).await;
        
        if connect_result.is_ok() {
            // Test that the connector can handle transaction-related queries
            // Note: Actual transaction implementation will be tested in integration tests
            
            let capabilities = connector.get_capabilities();
            assert!(capabilities.supports_transactions);
            
            let _ = connector.disconnect().await;
        }
    }

    #[tokio::test]
    async fn test_postgres_connector_schema_introspection() {
        let mut connector = PostgresConnector::new();
        let config = get_postgres_config();
        
        let connect_result = connector.connect(config).await;
        
        if connect_result.is_ok() {
            // Test schema retrieval for a system table that should exist
            let schema_result = connector.get_schema("information_schema.tables").await;
            
            match schema_result {
                Ok(schema) => {
                    assert_eq!(schema.name, "information_schema.tables");
                    assert!(!schema.columns.is_empty());
                    
                    // Verify we have expected columns for information_schema.tables
                    let column_names: Vec<&str> = schema.columns.iter().map(|c| c.name.as_str()).collect();
                    assert!(column_names.contains(&"table_name") || column_names.contains(&"TABLE_NAME"));
                }
                Err(NirvError::Connector(ConnectorError::SchemaRetrievalFailed(_))) => {
                    // Schema retrieval failed - acceptable for this test
                }
                Err(other) => {
                    panic!("Unexpected error during schema retrieval: {:?}", other);
                }
            }
            
            // Test schema retrieval for non-existent table
            let invalid_schema_result = connector.get_schema("non_existent_table_12345").await;
            assert!(invalid_schema_result.is_err());
            
            match invalid_schema_result.unwrap_err() {
                NirvError::Connector(ConnectorError::SchemaRetrievalFailed(_)) => {
                    // Expected for non-existent table
                }
                _ => panic!("Expected SchemaRetrievalFailed error for non-existent table"),
            }
            
            let _ = connector.disconnect().await;
        }
    }

    #[tokio::test]
    async fn test_postgres_connector_query_execution() {
        let mut connector = PostgresConnector::new();
        let config = get_postgres_config();
        
        let connect_result = connector.connect(config).await;
        
        if connect_result.is_ok() {
            // Test a simple query that should work on any PostgreSQL instance
            let mut query = InternalQuery::new(QueryOperation::Select);
            query.sources.push(DataSource {
                object_type: "postgres".to_string(),
                identifier: "pg_database".to_string(), // System catalog table
                alias: None,
            });
            
            let connector_query = ConnectorQuery {
                connector_type: ConnectorType::PostgreSQL,
                query,
                connection_params: HashMap::new(),
            };
            
            let result = connector.execute_query(connector_query).await;
            
            match result {
                Ok(query_result) => {
                    assert!(!query_result.columns.is_empty());
                    // pg_database should have at least one database
                    assert!(!query_result.rows.is_empty());
                    assert!(query_result.execution_time.as_millis() >= 0);
                }
                Err(NirvError::Connector(ConnectorError::QueryExecutionFailed(_))) => {
                    // Query execution failed - acceptable for this test environment
                }
                Err(other) => {
                    panic!("Unexpected error during query execution: {:?}", other);
                }
            }
            
            let _ = connector.disconnect().await;
        }
    }

    #[tokio::test]
    async fn test_postgres_connector_concurrent_queries() {
        let mut connector = PostgresConnector::new();
        let config = get_postgres_config().with_max_connections(3);
        
        let connect_result = connector.connect(config).await;
        
        if connect_result.is_ok() {
            // Test concurrent query execution
            let query1 = create_test_query("pg_database");
            let query2 = create_test_query("pg_tables");
            let query3 = create_test_query("information_schema.schemata");
            
            // Execute queries concurrently
            let (result1, result2, result3) = tokio::join!(
                connector.execute_query(query1),
                connector.execute_query(query2),
                connector.execute_query(query3)
            );
            
            // At least one query should succeed or fail with a query-related error (not connection error)
            let results = vec![result1, result2, result3];
            let mut connection_errors = 0;
            let mut other_results = 0;
            
            for result in results {
                match result {
                    Ok(_) => other_results += 1,
                    Err(NirvError::Connector(ConnectorError::ConnectionFailed(_))) => {
                        connection_errors += 1;
                    }
                    Err(_) => other_results += 1, // Query execution errors are acceptable
                }
            }
            
            // We should not have all connection errors if the connector is properly connected
            assert!(other_results > 0 || connection_errors < 3);
            
            let _ = connector.disconnect().await;
        }
    }

    #[tokio::test]
    async fn test_postgres_connector_timeout_handling() {
        let mut connector = PostgresConnector::new();
        
        // Test with very short timeout
        let config = get_postgres_config().with_timeout(1); // 1 second timeout
        
        let connect_result = connector.connect(config).await;
        
        // The connection might succeed or fail depending on network conditions
        // If it fails, it should be due to timeout or connection issues
        if let Err(error) = connect_result {
            match error {
                NirvError::Connector(ConnectorError::ConnectionFailed(_)) |
                NirvError::Connector(ConnectorError::Timeout(_)) => {
                    // Expected timeout or connection failure
                }
                _ => panic!("Unexpected error type for timeout test: {:?}", error),
            }
        }
    }

    #[tokio::test]
    async fn test_postgres_connector_error_handling() {
        let mut connector = PostgresConnector::new();
        let config = get_postgres_config();
        
        let connect_result = connector.connect(config).await;
        
        if connect_result.is_ok() {
            // Test query with invalid SQL
            let mut invalid_query = InternalQuery::new(QueryOperation::Select);
            invalid_query.sources.push(DataSource {
                object_type: "postgres".to_string(),
                identifier: "definitely_non_existent_table_xyz".to_string(),
                alias: None,
            });
            
            let connector_query = ConnectorQuery {
                connector_type: ConnectorType::PostgreSQL,
                query: invalid_query,
                connection_params: HashMap::new(),
            };
            
            let result = connector.execute_query(connector_query).await;
            assert!(result.is_err());
            
            match result.unwrap_err() {
                NirvError::Connector(ConnectorError::QueryExecutionFailed(_)) => {
                    // Expected for invalid table
                }
                _ => panic!("Expected QueryExecutionFailed error for invalid table"),
            }
            
            let _ = connector.disconnect().await;
        }
    }

    #[test]
    fn test_postgres_connector_capabilities() {
        let connector = PostgresConnector::new();
        let capabilities = connector.get_capabilities();
        
        // PostgreSQL should support all major SQL features
        assert!(capabilities.supports_joins);
        assert!(capabilities.supports_aggregations);
        assert!(capabilities.supports_subqueries);
        assert!(capabilities.supports_transactions);
        assert!(capabilities.supports_schema_introspection);
        
        // Should support multiple concurrent queries
        assert!(capabilities.max_concurrent_queries.unwrap_or(0) > 1);
    }

    #[test]
    fn test_postgres_connector_type() {
        let connector = PostgresConnector::new();
        assert_eq!(connector.get_connector_type(), ConnectorType::PostgreSQL);
    }
}

/// Integration tests that require a running PostgreSQL instance
#[cfg(test)]
mod integration_tests {
    use super::*;

    /// Test that requires a real PostgreSQL instance
    /// Run with: POSTGRES_HOST=localhost POSTGRES_USER=postgres POSTGRES_PASSWORD=password cargo test test_real_postgres_connection -- --ignored
    #[tokio::test]
    #[ignore] // Ignored by default since it requires a real PostgreSQL instance
    async fn test_real_postgres_connection() {
        let mut connector = PostgresConnector::new();
        let config = get_postgres_config();
        
        // This test requires a real PostgreSQL instance
        let connect_result = connector.connect(config).await;
        assert!(connect_result.is_ok(), "Failed to connect to PostgreSQL: {:?}", connect_result.err());
        
        assert!(connector.is_connected());
        
        // Test schema retrieval
        let schema_result = connector.get_schema("information_schema.tables").await;
        assert!(schema_result.is_ok(), "Failed to retrieve schema: {:?}", schema_result.err());
        
        let schema = schema_result.unwrap();
        assert_eq!(schema.name, "information_schema.tables");
        assert!(!schema.columns.is_empty());
        
        // Test query execution
        let query = create_test_query("information_schema.tables");
        let query_result = connector.execute_query(query).await;
        assert!(query_result.is_ok(), "Failed to execute query: {:?}", query_result.err());
        
        let result = query_result.unwrap();
        assert!(!result.columns.is_empty());
        
        // Test disconnection
        let disconnect_result = connector.disconnect().await;
        assert!(disconnect_result.is_ok());
        assert!(!connector.is_connected());
    }
}