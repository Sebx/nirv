use async_trait::async_trait;
use std::collections::HashMap;
use std::time::{Duration, Instant};
use crate::connectors::connector_trait::{Connector, ConnectorInitConfig, ConnectorCapabilities};
use crate::utils::{
    types::{
        ConnectorType, ConnectorQuery, QueryResult, Schema, ColumnMetadata, 
        DataType, Row, Value, Index, QueryOperation, PredicateOperator
    },
    error::{ConnectorError, NirvResult},
};

/// Mock connector for testing with deterministic in-memory data
#[derive(Debug)]
pub struct MockConnector {
    connected: bool,
    test_data: HashMap<String, TestTable>,
    connection_delay_ms: u64,
}

/// Test table structure for mock data
#[derive(Debug, Clone)]
struct TestTable {
    schema: Schema,
    rows: Vec<Row>,
}

impl MockConnector {
    /// Create a new mock connector with default test data
    pub fn new() -> Self {
        let mut connector = Self {
            connected: false,
            test_data: HashMap::new(),
            connection_delay_ms: 10, // Simulate small connection delay
        };
        
        connector.initialize_test_data();
        connector
    }
    
    /// Create a mock connector with custom connection delay
    pub fn with_delay(delay_ms: u64) -> Self {
        let mut connector = Self::new();
        connector.connection_delay_ms = delay_ms;
        connector
    }
    
    /// Add custom test data for a table
    pub fn add_test_data(&mut self, table_name: &str, rows: Vec<Vec<Value>>) {
        self.add_test_data_with_schema(table_name, rows, None);
    }
    
    /// Add custom test data for a table with column names
    pub fn add_test_data_with_columns(&mut self, table_name: &str, column_names: Vec<&str>, rows: Vec<Vec<Value>>) {
        // Create schema with provided column names
        let columns = if let Some(first_row) = rows.first() {
            first_row.iter().enumerate().map(|(i, value)| {
                let data_type = match value {
                    Value::Integer(_) => DataType::Integer,
                    Value::Float(_) => DataType::Float,
                    Value::Text(_) => DataType::Text,
                    Value::Boolean(_) => DataType::Boolean,
                    Value::Date(_) => DataType::Date,
                    Value::DateTime(_) => DataType::DateTime,
                    Value::Json(_) => DataType::Json,
                    Value::Binary(_) => DataType::Binary,
                    Value::Null => DataType::Text, // Default for null
                };
                
                let column_name = column_names.get(i).unwrap_or(&format!("column_{}", i).as_str()).to_string();
                
                ColumnMetadata {
                    name: column_name,
                    data_type,
                    nullable: true,
                }
            }).collect()
        } else {
            vec![]
        };
        
        let schema = Schema {
            name: table_name.to_string(),
            columns,
            primary_key: None,
            indexes: vec![],
        };
        
        let table_rows: Vec<Row> = rows.into_iter().map(Row::new).collect();
        
        self.test_data.insert(table_name.to_string(), TestTable {
            schema,
            rows: table_rows,
        });
    }
    
    /// Add custom test data for a table with optional schema
    fn add_test_data_with_schema(&mut self, table_name: &str, rows: Vec<Vec<Value>>, _schema: Option<Schema>) {
        // Create a simple schema based on the first row
        let columns = if let Some(first_row) = rows.first() {
            first_row.iter().enumerate().map(|(i, value)| {
                let data_type = match value {
                    Value::Integer(_) => DataType::Integer,
                    Value::Float(_) => DataType::Float,
                    Value::Text(_) => DataType::Text,
                    Value::Boolean(_) => DataType::Boolean,
                    Value::Date(_) => DataType::Date,
                    Value::DateTime(_) => DataType::DateTime,
                    Value::Json(_) => DataType::Json,
                    Value::Binary(_) => DataType::Binary,
                    Value::Null => DataType::Text, // Default for null
                };
                
                ColumnMetadata {
                    name: format!("column_{}", i),
                    data_type,
                    nullable: true,
                }
            }).collect()
        } else {
            vec![]
        };
        
        let schema = Schema {
            name: table_name.to_string(),
            columns,
            primary_key: None,
            indexes: vec![],
        };
        
        let table_rows: Vec<Row> = rows.into_iter().map(Row::new).collect();
        
        self.test_data.insert(table_name.to_string(), TestTable {
            schema,
            rows: table_rows,
        });
    }
    
    /// Initialize deterministic test data
    fn initialize_test_data(&mut self) {
        // Users table
        let users_schema = Schema {
            name: "users".to_string(),
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
                    name: "age".to_string(),
                    data_type: DataType::Integer,
                    nullable: true,
                },
                ColumnMetadata {
                    name: "active".to_string(),
                    data_type: DataType::Boolean,
                    nullable: false,
                },
            ],
            primary_key: Some(vec!["id".to_string()]),
            indexes: vec![
                Index {
                    name: "idx_users_email".to_string(),
                    columns: vec!["email".to_string()],
                    unique: true,
                },
            ],
        };
        
        let users_rows = vec![
            Row::new(vec![
                Value::Integer(1),
                Value::Text("Alice Johnson".to_string()),
                Value::Text("alice@example.com".to_string()),
                Value::Integer(30),
                Value::Boolean(true),
            ]),
            Row::new(vec![
                Value::Integer(2),
                Value::Text("Bob Smith".to_string()),
                Value::Text("bob@example.com".to_string()),
                Value::Integer(25),
                Value::Boolean(true),
            ]),
            Row::new(vec![
                Value::Integer(3),
                Value::Text("Charlie Brown".to_string()),
                Value::Null,
                Value::Integer(35),
                Value::Boolean(false),
            ]),
        ];
        
        self.test_data.insert("users".to_string(), TestTable {
            schema: users_schema,
            rows: users_rows,
        });
        
        // Products table
        let products_schema = Schema {
            name: "products".to_string(),
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
                    name: "category".to_string(),
                    data_type: DataType::Text,
                    nullable: true,
                },
            ],
            primary_key: Some(vec!["id".to_string()]),
            indexes: vec![],
        };
        
        let products_rows = vec![
            Row::new(vec![
                Value::Integer(1),
                Value::Text("Laptop".to_string()),
                Value::Float(999.99),
                Value::Text("Electronics".to_string()),
            ]),
            Row::new(vec![
                Value::Integer(2),
                Value::Text("Coffee Mug".to_string()),
                Value::Float(12.50),
                Value::Text("Kitchen".to_string()),
            ]),
        ];
        
        self.test_data.insert("products".to_string(), TestTable {
            schema: products_schema,
            rows: products_rows,
        });
    }
    
    /// Apply WHERE clause filtering to rows
    fn apply_filters(&self, rows: &[Row], query: &ConnectorQuery) -> Vec<Row> {
        if query.query.predicates.is_empty() {
            return rows.to_vec();
        }
        
        let table_name = if let Some(source) = query.query.sources.first() {
            &source.identifier
        } else {
            return rows.to_vec();
        };
        
        let schema = if let Some(table) = self.test_data.get(table_name) {
            &table.schema
        } else {
            return rows.to_vec();
        };
        
        rows.iter()
            .filter(|row| {
                query.query.predicates.iter().all(|predicate| {
                    // Find column index
                    let col_index = schema.columns.iter()
                        .position(|col| col.name == predicate.column);
                    
                    if let Some(index) = col_index {
                        if let Some(value) = row.get(index) {
                            self.evaluate_predicate(value, &predicate.operator, &predicate.value)
                        } else {
                            false
                        }
                    } else {
                        false
                    }
                })
            })
            .cloned()
            .collect()
    }
    
    /// Evaluate a single predicate against a value
    fn evaluate_predicate(&self, value: &Value, operator: &PredicateOperator, predicate_value: &crate::utils::types::PredicateValue) -> bool {
        use crate::utils::types::PredicateValue;
        
        match operator {
            PredicateOperator::Equal => {
                match (value, predicate_value) {
                    (Value::Integer(v), PredicateValue::Integer(p)) => v == p,
                    (Value::Text(v), PredicateValue::String(p)) => v == p,
                    (Value::Float(v), PredicateValue::Number(p)) => (v - p).abs() < f64::EPSILON,
                    (Value::Boolean(v), PredicateValue::Boolean(p)) => v == p,
                    (Value::Null, PredicateValue::Null) => true,
                    _ => false,
                }
            },
            PredicateOperator::NotEqual => !self.evaluate_predicate(value, &PredicateOperator::Equal, predicate_value),
            PredicateOperator::GreaterThan => {
                match (value, predicate_value) {
                    (Value::Integer(v), PredicateValue::Integer(p)) => v > p,
                    (Value::Float(v), PredicateValue::Number(p)) => v > p,
                    _ => false,
                }
            },
            PredicateOperator::GreaterThanOrEqual => {
                self.evaluate_predicate(value, &PredicateOperator::GreaterThan, predicate_value) ||
                self.evaluate_predicate(value, &PredicateOperator::Equal, predicate_value)
            },
            PredicateOperator::LessThan => {
                match (value, predicate_value) {
                    (Value::Integer(v), PredicateValue::Integer(p)) => v < p,
                    (Value::Float(v), PredicateValue::Number(p)) => v < p,
                    _ => false,
                }
            },
            PredicateOperator::LessThanOrEqual => {
                self.evaluate_predicate(value, &PredicateOperator::LessThan, predicate_value) ||
                self.evaluate_predicate(value, &PredicateOperator::Equal, predicate_value)
            },
            PredicateOperator::Like => {
                match (value, predicate_value) {
                    (Value::Text(v), PredicateValue::String(p)) => {
                        // Simple LIKE implementation (% as wildcard)
                        let pattern = p.replace('%', ".*");
                        regex::Regex::new(&pattern).map(|re| re.is_match(v)).unwrap_or(false)
                    },
                    _ => false,
                }
            },
            PredicateOperator::IsNull => matches!(value, Value::Null),
            PredicateOperator::IsNotNull => !matches!(value, Value::Null),
            PredicateOperator::In => {
                if let PredicateValue::List(values) = predicate_value {
                    values.iter().any(|pv| self.evaluate_predicate(value, &PredicateOperator::Equal, pv))
                } else {
                    false
                }
            },
        }
    }
    
    /// Apply LIMIT clause to rows
    fn apply_limit(&self, rows: Vec<Row>, limit: Option<u64>) -> Vec<Row> {
        if let Some(limit_count) = limit {
            rows.into_iter().take(limit_count as usize).collect()
        } else {
            rows
        }
    }
}

impl Default for MockConnector {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Connector for MockConnector {
    async fn connect(&mut self, _config: ConnectorInitConfig) -> NirvResult<()> {
        // Simulate connection delay
        if self.connection_delay_ms > 0 {
            tokio::time::sleep(Duration::from_millis(self.connection_delay_ms)).await;
        }
        
        self.connected = true;
        Ok(())
    }
    
    async fn execute_query(&self, query: ConnectorQuery) -> NirvResult<QueryResult> {
        if !self.connected {
            return Err(ConnectorError::ConnectionFailed("Not connected".to_string()).into());
        }
        
        let start_time = Instant::now();
        
        // Add a small delay to ensure execution time is recorded
        tokio::time::sleep(tokio::time::Duration::from_millis(1)).await;
        
        match query.query.operation {
            QueryOperation::Select => {
                if let Some(source) = query.query.sources.first() {
                    if let Some(table) = self.test_data.get(&source.identifier) {
                        let filtered_rows = self.apply_filters(&table.rows, &query);
                        // Note: Limit is handled by the query executor, not the connector
                        
                        let result = QueryResult {
                            columns: table.schema.columns.clone(),
                            rows: filtered_rows,
                            affected_rows: None,
                            execution_time: start_time.elapsed(),
                        };
                        
                        Ok(result)
                    } else {
                        Err(ConnectorError::QueryExecutionFailed(
                            format!("Table '{}' not found", source.identifier)
                        ).into())
                    }
                } else {
                    Err(ConnectorError::QueryExecutionFailed(
                        "No data source specified in query".to_string()
                    ).into())
                }
            },
            _ => Err(ConnectorError::UnsupportedOperation(
                format!("Operation {:?} not supported by MockConnector", query.query.operation)
            ).into()),
        }
    }
    
    async fn get_schema(&self, object_name: &str) -> NirvResult<Schema> {
        if !self.connected {
            return Err(ConnectorError::ConnectionFailed("Not connected".to_string()).into());
        }
        
        if let Some(table) = self.test_data.get(object_name) {
            Ok(table.schema.clone())
        } else {
            Err(ConnectorError::SchemaRetrievalFailed(
                format!("Object '{}' not found", object_name)
            ).into())
        }
    }
    
    async fn disconnect(&mut self) -> NirvResult<()> {
        self.connected = false;
        Ok(())
    }
    
    fn get_connector_type(&self) -> ConnectorType {
        ConnectorType::Mock
    }
    
    fn supports_transactions(&self) -> bool {
        false
    }
    
    fn is_connected(&self) -> bool {
        self.connected
    }
    
    fn get_capabilities(&self) -> ConnectorCapabilities {
        ConnectorCapabilities {
            supports_joins: false,
            supports_aggregations: false,
            supports_subqueries: false,
            supports_transactions: false,
            supports_schema_introspection: true,
            max_concurrent_queries: Some(10),
        }
    }
}#[
cfg(test)]
mod tests {
    use super::*;
    use crate::utils::types::{
        InternalQuery, QueryOperation, DataSource, Predicate, PredicateOperator, PredicateValue
    };

    #[tokio::test]
    async fn test_mock_connector_creation() {
        let connector = MockConnector::new();
        
        assert!(!connector.is_connected());
        assert_eq!(connector.get_connector_type(), ConnectorType::Mock);
        assert!(!connector.supports_transactions());
        assert_eq!(connector.test_data.len(), 2); // users and products tables
    }

    #[tokio::test]
    async fn test_mock_connector_with_delay() {
        let connector = MockConnector::with_delay(50);
        
        assert!(!connector.is_connected());
        assert_eq!(connector.connection_delay_ms, 50);
    }

    #[tokio::test]
    async fn test_mock_connector_connection_lifecycle() {
        let mut connector = MockConnector::new();
        let config = ConnectorInitConfig::new();
        
        // Initially not connected
        assert!(!connector.is_connected());
        
        // Connect
        let result = connector.connect(config).await;
        assert!(result.is_ok());
        assert!(connector.is_connected());
        
        // Disconnect
        let result = connector.disconnect().await;
        assert!(result.is_ok());
        assert!(!connector.is_connected());
    }

    #[tokio::test]
    async fn test_mock_connector_connection_delay() {
        let mut connector = MockConnector::with_delay(10);
        let config = ConnectorInitConfig::new();
        
        let start = std::time::Instant::now();
        let result = connector.connect(config).await;
        let elapsed = start.elapsed();
        
        assert!(result.is_ok());
        assert!(connector.is_connected());
        assert!(elapsed >= std::time::Duration::from_millis(10));
    }

    #[tokio::test]
    async fn test_mock_connector_query_without_connection() {
        let connector = MockConnector::new();
        let query = ConnectorQuery {
            connector_type: ConnectorType::Mock,
            query: InternalQuery::new(QueryOperation::Select),
            connection_params: std::collections::HashMap::new(),
        };
        
        let result = connector.execute_query(query).await;
        assert!(result.is_err());
        
        match result.unwrap_err() {
            crate::utils::error::NirvError::Connector(ConnectorError::ConnectionFailed(msg)) => {
                assert_eq!(msg, "Not connected");
            }
            _ => panic!("Expected ConnectionFailed error"),
        }
    }

    #[tokio::test]
    async fn test_mock_connector_select_users_table() {
        let mut connector = MockConnector::new();
        let config = ConnectorInitConfig::new();
        connector.connect(config).await.unwrap();
        
        let mut query = InternalQuery::new(QueryOperation::Select);
        query.sources.push(DataSource {
            object_type: "mock".to_string(),
            identifier: "users".to_string(),
            alias: None,
        });
        
        let connector_query = ConnectorQuery {
            connector_type: ConnectorType::Mock,
            query,
            connection_params: std::collections::HashMap::new(),
        };
        
        let result = connector.execute_query(connector_query).await;
        assert!(result.is_ok());
        
        let query_result = result.unwrap();
        assert_eq!(query_result.columns.len(), 5); // id, name, email, age, active
        assert_eq!(query_result.rows.len(), 3); // Alice, Bob, Charlie
        assert!(query_result.execution_time > std::time::Duration::from_nanos(0));
        
        // Check first row (Alice)
        let first_row = &query_result.rows[0];
        assert_eq!(first_row.get(0), Some(&Value::Integer(1)));
        assert_eq!(first_row.get(1), Some(&Value::Text("Alice Johnson".to_string())));
        assert_eq!(first_row.get(2), Some(&Value::Text("alice@example.com".to_string())));
        assert_eq!(first_row.get(3), Some(&Value::Integer(30)));
        assert_eq!(first_row.get(4), Some(&Value::Boolean(true)));
    }

    #[tokio::test]
    async fn test_mock_connector_select_products_table() {
        let mut connector = MockConnector::new();
        let config = ConnectorInitConfig::new();
        connector.connect(config).await.unwrap();
        
        let mut query = InternalQuery::new(QueryOperation::Select);
        query.sources.push(DataSource {
            object_type: "mock".to_string(),
            identifier: "products".to_string(),
            alias: None,
        });
        
        let connector_query = ConnectorQuery {
            connector_type: ConnectorType::Mock,
            query,
            connection_params: std::collections::HashMap::new(),
        };
        
        let result = connector.execute_query(connector_query).await;
        assert!(result.is_ok());
        
        let query_result = result.unwrap();
        assert_eq!(query_result.columns.len(), 4); // id, name, price, category
        assert_eq!(query_result.rows.len(), 2); // Laptop, Coffee Mug
        
        // Check first row (Laptop)
        let first_row = &query_result.rows[0];
        assert_eq!(first_row.get(0), Some(&Value::Integer(1)));
        assert_eq!(first_row.get(1), Some(&Value::Text("Laptop".to_string())));
        assert_eq!(first_row.get(2), Some(&Value::Float(999.99)));
        assert_eq!(first_row.get(3), Some(&Value::Text("Electronics".to_string())));
    }

    #[tokio::test]
    async fn test_mock_connector_query_non_existent_table() {
        let mut connector = MockConnector::new();
        let config = ConnectorInitConfig::new();
        connector.connect(config).await.unwrap();
        
        let mut query = InternalQuery::new(QueryOperation::Select);
        query.sources.push(DataSource {
            object_type: "mock".to_string(),
            identifier: "non_existent".to_string(),
            alias: None,
        });
        
        let connector_query = ConnectorQuery {
            connector_type: ConnectorType::Mock,
            query,
            connection_params: std::collections::HashMap::new(),
        };
        
        let result = connector.execute_query(connector_query).await;
        assert!(result.is_err());
        
        match result.unwrap_err() {
            crate::utils::error::NirvError::Connector(ConnectorError::QueryExecutionFailed(msg)) => {
                assert!(msg.contains("Table 'non_existent' not found"));
            }
            _ => panic!("Expected QueryExecutionFailed error"),
        }
    }

    #[tokio::test]
    async fn test_mock_connector_query_with_where_clause() {
        let mut connector = MockConnector::new();
        let config = ConnectorInitConfig::new();
        connector.connect(config).await.unwrap();
        
        let mut query = InternalQuery::new(QueryOperation::Select);
        query.sources.push(DataSource {
            object_type: "mock".to_string(),
            identifier: "users".to_string(),
            alias: None,
        });
        
        // Add WHERE age > 25
        query.predicates.push(Predicate {
            column: "age".to_string(),
            operator: PredicateOperator::GreaterThan,
            value: PredicateValue::Integer(25),
        });
        
        let connector_query = ConnectorQuery {
            connector_type: ConnectorType::Mock,
            query,
            connection_params: std::collections::HashMap::new(),
        };
        
        let result = connector.execute_query(connector_query).await;
        assert!(result.is_ok());
        
        let query_result = result.unwrap();
        assert_eq!(query_result.rows.len(), 2); // Alice (30) and Charlie (35)
        
        // Check that returned users have age > 25
        for row in &query_result.rows {
            if let Some(Value::Integer(age)) = row.get(3) {
                assert!(*age > 25);
            }
        }
    }

    #[tokio::test]
    async fn test_mock_connector_query_with_limit() {
        let mut connector = MockConnector::new();
        let config = ConnectorInitConfig::new();
        connector.connect(config).await.unwrap();
        
        let mut query = InternalQuery::new(QueryOperation::Select);
        query.sources.push(DataSource {
            object_type: "mock".to_string(),
            identifier: "users".to_string(),
            alias: None,
        });
        query.limit = Some(2);
        
        let connector_query = ConnectorQuery {
            connector_type: ConnectorType::Mock,
            query,
            connection_params: std::collections::HashMap::new(),
        };
        
        let result = connector.execute_query(connector_query).await;
        assert!(result.is_ok());
        
        let query_result = result.unwrap();
        assert_eq!(query_result.rows.len(), 2); // Limited to 2 rows
    }

    #[tokio::test]
    async fn test_mock_connector_query_with_equal_predicate() {
        let mut connector = MockConnector::new();
        let config = ConnectorInitConfig::new();
        connector.connect(config).await.unwrap();
        
        let mut query = InternalQuery::new(QueryOperation::Select);
        query.sources.push(DataSource {
            object_type: "mock".to_string(),
            identifier: "users".to_string(),
            alias: None,
        });
        
        // Add WHERE name = 'Alice Johnson'
        query.predicates.push(Predicate {
            column: "name".to_string(),
            operator: PredicateOperator::Equal,
            value: PredicateValue::String("Alice Johnson".to_string()),
        });
        
        let connector_query = ConnectorQuery {
            connector_type: ConnectorType::Mock,
            query,
            connection_params: std::collections::HashMap::new(),
        };
        
        let result = connector.execute_query(connector_query).await;
        assert!(result.is_ok());
        
        let query_result = result.unwrap();
        assert_eq!(query_result.rows.len(), 1); // Only Alice
        
        let alice_row = &query_result.rows[0];
        assert_eq!(alice_row.get(1), Some(&Value::Text("Alice Johnson".to_string())));
    }

    #[tokio::test]
    async fn test_mock_connector_query_with_null_predicate() {
        let mut connector = MockConnector::new();
        let config = ConnectorInitConfig::new();
        connector.connect(config).await.unwrap();
        
        let mut query = InternalQuery::new(QueryOperation::Select);
        query.sources.push(DataSource {
            object_type: "mock".to_string(),
            identifier: "users".to_string(),
            alias: None,
        });
        
        // Add WHERE email IS NULL
        query.predicates.push(Predicate {
            column: "email".to_string(),
            operator: PredicateOperator::IsNull,
            value: PredicateValue::Null,
        });
        
        let connector_query = ConnectorQuery {
            connector_type: ConnectorType::Mock,
            query,
            connection_params: std::collections::HashMap::new(),
        };
        
        let result = connector.execute_query(connector_query).await;
        assert!(result.is_ok());
        
        let query_result = result.unwrap();
        assert_eq!(query_result.rows.len(), 1); // Only Charlie has null email
        
        let charlie_row = &query_result.rows[0];
        assert_eq!(charlie_row.get(1), Some(&Value::Text("Charlie Brown".to_string())));
        assert_eq!(charlie_row.get(2), Some(&Value::Null));
    }

    #[tokio::test]
    async fn test_mock_connector_unsupported_operation() {
        let mut connector = MockConnector::new();
        let config = ConnectorInitConfig::new();
        connector.connect(config).await.unwrap();
        
        let query = InternalQuery::new(QueryOperation::Insert);
        let connector_query = ConnectorQuery {
            connector_type: ConnectorType::Mock,
            query,
            connection_params: std::collections::HashMap::new(),
        };
        
        let result = connector.execute_query(connector_query).await;
        assert!(result.is_err());
        
        match result.unwrap_err() {
            crate::utils::error::NirvError::Connector(ConnectorError::UnsupportedOperation(msg)) => {
                assert!(msg.contains("Operation Insert not supported"));
            }
            _ => panic!("Expected UnsupportedOperation error"),
        }
    }

    #[tokio::test]
    async fn test_mock_connector_get_schema() {
        let mut connector = MockConnector::new();
        let config = ConnectorInitConfig::new();
        connector.connect(config).await.unwrap();
        
        // Get users table schema
        let result = connector.get_schema("users").await;
        assert!(result.is_ok());
        
        let schema = result.unwrap();
        assert_eq!(schema.name, "users");
        assert_eq!(schema.columns.len(), 5);
        assert_eq!(schema.primary_key, Some(vec!["id".to_string()]));
        assert_eq!(schema.indexes.len(), 1);
        
        // Check column metadata
        assert_eq!(schema.columns[0].name, "id");
        assert_eq!(schema.columns[0].data_type, DataType::Integer);
        assert!(!schema.columns[0].nullable);
        
        assert_eq!(schema.columns[1].name, "name");
        assert_eq!(schema.columns[1].data_type, DataType::Text);
        assert!(!schema.columns[1].nullable);
        
        assert_eq!(schema.columns[2].name, "email");
        assert_eq!(schema.columns[2].data_type, DataType::Text);
        assert!(schema.columns[2].nullable);
    }

    #[tokio::test]
    async fn test_mock_connector_get_schema_non_existent() {
        let mut connector = MockConnector::new();
        let config = ConnectorInitConfig::new();
        connector.connect(config).await.unwrap();
        
        let result = connector.get_schema("non_existent").await;
        assert!(result.is_err());
        
        match result.unwrap_err() {
            crate::utils::error::NirvError::Connector(ConnectorError::SchemaRetrievalFailed(msg)) => {
                assert!(msg.contains("Object 'non_existent' not found"));
            }
            _ => panic!("Expected SchemaRetrievalFailed error"),
        }
    }

    #[tokio::test]
    async fn test_mock_connector_get_schema_without_connection() {
        let connector = MockConnector::new();
        
        let result = connector.get_schema("users").await;
        assert!(result.is_err());
        
        match result.unwrap_err() {
            crate::utils::error::NirvError::Connector(ConnectorError::ConnectionFailed(msg)) => {
                assert_eq!(msg, "Not connected");
            }
            _ => panic!("Expected ConnectionFailed error"),
        }
    }

    #[tokio::test]
    async fn test_mock_connector_capabilities() {
        let connector = MockConnector::new();
        let capabilities = connector.get_capabilities();
        
        assert!(!capabilities.supports_joins);
        assert!(!capabilities.supports_aggregations);
        assert!(!capabilities.supports_subqueries);
        assert!(!capabilities.supports_transactions);
        assert!(capabilities.supports_schema_introspection);
        assert_eq!(capabilities.max_concurrent_queries, Some(10));
    }

    #[test]
    fn test_mock_connector_default() {
        let connector = MockConnector::default();
        
        assert!(!connector.is_connected());
        assert_eq!(connector.get_connector_type(), ConnectorType::Mock);
        assert_eq!(connector.test_data.len(), 2);
    }
}