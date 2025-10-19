use nirv_engine::{
    engine::{QueryExecutor, ExecutionPlan, PlanNode, DefaultQueryExecutor},
    connectors::{MockConnector, ConnectorRegistry, Connector, ConnectorInitConfig},
    utils::{
        types::{InternalQuery, QueryOperation, DataSource, Column, QueryResult, ConnectorType, Value, Row, ColumnMetadata, DataType},
        error::{NirvResult, NirvError},
    },
};
use std::time::Duration;

#[tokio::test]
async fn test_query_executor_single_table_scan() {
    let mut executor = DefaultQueryExecutor::new();
    
    // Create a mock connector with test data
    let mut mock_connector = MockConnector::new();
    mock_connector.add_test_data("users", vec![
        vec![Value::Integer(1), Value::Text("Alice".to_string())],
        vec![Value::Integer(2), Value::Text("Bob".to_string())],
    ]);
    
    // Connect the mock connector
    mock_connector.connect(ConnectorInitConfig::new()).await.unwrap();
    
    let mut connector_registry = ConnectorRegistry::new();
    connector_registry.register("mock_0".to_string(), Box::new(mock_connector)).unwrap();
    
    executor.set_connector_registry(connector_registry);
    
    // Create execution plan
    let plan = ExecutionPlan {
        nodes: vec![
            PlanNode::TableScan {
                source: DataSource {
                    object_type: "mock".to_string(),
                    identifier: "users".to_string(),
                    alias: None,
                },
                projections: vec![
                    Column { name: "id".to_string(), alias: None, source: None },
                    Column { name: "name".to_string(), alias: None, source: None },
                ],
                predicates: vec![],
            }
        ],
        estimated_cost: 1.0,
    };
    
    let result = executor.execute_plan(&plan).await;
    if let Err(e) = &result {
        println!("Error: {:?}", e);
    }
    assert!(result.is_ok());
    
    let query_result = result.unwrap();
    assert_eq!(query_result.row_count(), 2);
    assert!(query_result.execution_time > Duration::from_millis(0));
}

#[tokio::test]
async fn test_query_executor_with_limit() {
    let mut executor = DefaultQueryExecutor::new();
    
    // Create a mock connector with test data
    let mut mock_connector = MockConnector::new();
    mock_connector.add_test_data("users", vec![
        vec![Value::Integer(1), Value::Text("Alice".to_string())],
        vec![Value::Integer(2), Value::Text("Bob".to_string())],
        vec![Value::Integer(3), Value::Text("Charlie".to_string())],
    ]);
    
    // Connect the mock connector
    mock_connector.connect(ConnectorInitConfig::new()).await.unwrap();
    
    let mut connector_registry = ConnectorRegistry::new();
    connector_registry.register("mock_0".to_string(), Box::new(mock_connector)).unwrap();
    
    executor.set_connector_registry(connector_registry);
    
    // Create execution plan with limit
    let plan = ExecutionPlan {
        nodes: vec![
            PlanNode::TableScan {
                source: DataSource {
                    object_type: "mock".to_string(),
                    identifier: "users".to_string(),
                    alias: None,
                },
                projections: vec![
                    Column { name: "*".to_string(), alias: None, source: None },
                ],
                predicates: vec![],
            },
            PlanNode::Limit {
                count: 2,
                input: Box::new(PlanNode::TableScan {
                    source: DataSource {
                        object_type: "mock".to_string(),
                        identifier: "users".to_string(),
                        alias: None,
                    },
                    projections: vec![],
                    predicates: vec![],
                }),
            }
        ],
        estimated_cost: 1.5,
    };
    
    let result = executor.execute_plan(&plan).await;
    assert!(result.is_ok());
    
    let query_result = result.unwrap();
    assert_eq!(query_result.row_count(), 2); // Limited to 2 rows
}

#[tokio::test]
async fn test_query_executor_result_formatting() {
    let mut executor = DefaultQueryExecutor::new();
    
    // Create a mock connector with typed test data
    let mut mock_connector = MockConnector::new();
    mock_connector.add_test_data_with_columns("products", vec!["id", "name", "price"], vec![
        vec![Value::Integer(1), Value::Text("Laptop".to_string()), Value::Float(999.99)],
        vec![Value::Integer(2), Value::Text("Mouse".to_string()), Value::Float(29.99)],
    ]);
    
    // Connect the mock connector
    mock_connector.connect(ConnectorInitConfig::new()).await.unwrap();
    
    let mut connector_registry = ConnectorRegistry::new();
    connector_registry.register("mock_0".to_string(), Box::new(mock_connector)).unwrap();
    
    executor.set_connector_registry(connector_registry);
    
    // Create execution plan
    let plan = ExecutionPlan {
        nodes: vec![
            PlanNode::TableScan {
                source: DataSource {
                    object_type: "mock".to_string(),
                    identifier: "products".to_string(),
                    alias: None,
                },
                projections: vec![
                    Column { name: "id".to_string(), alias: None, source: None },
                    Column { name: "name".to_string(), alias: None, source: None },
                    Column { name: "price".to_string(), alias: None, source: None },
                ],
                predicates: vec![],
            }
        ],
        estimated_cost: 1.0,
    };
    
    let result = executor.execute_plan(&plan).await;
    assert!(result.is_ok());
    
    let query_result = result.unwrap();
    assert_eq!(query_result.row_count(), 2);
    assert_eq!(query_result.columns.len(), 3);
    
    // Check column metadata
    assert_eq!(query_result.columns[0].name, "id");
    assert_eq!(query_result.columns[1].name, "name");
    assert_eq!(query_result.columns[2].name, "price");
    
    // Check first row values
    let first_row = &query_result.rows[0];
    assert_eq!(first_row.get(0), Some(&Value::Integer(1)));
    assert_eq!(first_row.get(1), Some(&Value::Text("Laptop".to_string())));
    assert_eq!(first_row.get(2), Some(&Value::Float(999.99)));
}

#[tokio::test]
async fn test_query_executor_concurrent_execution() {
    let mut executor = DefaultQueryExecutor::new();
    
    // Create a single mock connector with both tables
    let mut mock_connector = MockConnector::new();
    mock_connector.add_test_data("table1", vec![
        vec![Value::Integer(1), Value::Text("Data1".to_string())],
    ]);
    mock_connector.add_test_data("table2", vec![
        vec![Value::Integer(2), Value::Text("Data2".to_string())],
    ]);
    mock_connector.connect(ConnectorInitConfig::new()).await.unwrap();
    
    let mut connector_registry = ConnectorRegistry::new();
    connector_registry.register("mock_0".to_string(), Box::new(mock_connector)).unwrap();
    
    executor.set_connector_registry(connector_registry);
    
    // Create two execution plans
    let plan1 = ExecutionPlan {
        nodes: vec![
            PlanNode::TableScan {
                source: DataSource {
                    object_type: "mock".to_string(),
                    identifier: "table1".to_string(),
                    alias: None,
                },
                projections: vec![Column { name: "*".to_string(), alias: None, source: None }],
                predicates: vec![],
            }
        ],
        estimated_cost: 1.0,
    };
    
    let plan2 = ExecutionPlan {
        nodes: vec![
            PlanNode::TableScan {
                source: DataSource {
                    object_type: "mock".to_string(),
                    identifier: "table2".to_string(),
                    alias: None,
                },
                projections: vec![Column { name: "*".to_string(), alias: None, source: None }],
                predicates: vec![],
            }
        ],
        estimated_cost: 1.0,
    };
    
    // Execute both plans concurrently
    let (result1, result2) = tokio::join!(
        executor.execute_plan(&plan1),
        executor.execute_plan(&plan2)
    );
    
    assert!(result1.is_ok());
    assert!(result2.is_ok());
    
    let query_result1 = result1.unwrap();
    let query_result2 = result2.unwrap();
    
    assert_eq!(query_result1.row_count(), 1);
    assert_eq!(query_result2.row_count(), 1);
}

#[tokio::test]
async fn test_query_executor_error_propagation() {
    let executor = DefaultQueryExecutor::new();
    
    // Create execution plan with non-existent connector
    let plan = ExecutionPlan {
        nodes: vec![
            PlanNode::TableScan {
                source: DataSource {
                    object_type: "nonexistent".to_string(),
                    identifier: "table".to_string(),
                    alias: None,
                },
                projections: vec![],
                predicates: vec![],
            }
        ],
        estimated_cost: 1.0,
    };
    
    let result = executor.execute_plan(&plan).await;
    assert!(result.is_err());
    
    match result.unwrap_err() {
        NirvError::Internal(msg) => {
            assert!(msg.contains("No connector registry"));
        }
        _ => panic!("Expected Internal error for missing connector"),
    }
}

#[tokio::test]
async fn test_query_executor_empty_plan() {
    let executor = DefaultQueryExecutor::new();
    
    let plan = ExecutionPlan {
        nodes: vec![],
        estimated_cost: 0.0,
    };
    
    let result = executor.execute_plan(&plan).await;
    assert!(result.is_ok());
    
    let query_result = result.unwrap();
    assert_eq!(query_result.row_count(), 0);
    assert!(query_result.is_empty());
}