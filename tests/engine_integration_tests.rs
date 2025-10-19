use nirv_engine::{
    Engine, EngineBuilder,
    DefaultQueryParser, DefaultQueryPlanner, DefaultQueryExecutor, DefaultDispatcher,
    MockConnector, ConnectorInitConfig, Connector,
    EngineConfig, ProtocolConfig, DispatcherConfig, SecurityConfig,
    NirvResult, NirvError,
};
use nirv_engine::config::ProtocolType as ConfigProtocolType;
use std::sync::Arc;
use tokio::sync::RwLock;
use std::collections::HashMap;

/// Test engine initialization with default configuration
#[tokio::test]
async fn test_engine_initialization_default_config() -> NirvResult<()> {
    let config = EngineConfig::default();
    let mut engine = Engine::new(config);
    
    // Initialize the engine for testing (without starting servers)
    let result = engine.initialize_for_testing().await;
    assert!(result.is_ok(), "Engine initialization should succeed with default config");
    
    // Shutdown the engine
    engine.shutdown().await?;
    
    Ok(())
}

/// Test engine initialization with custom configuration
#[tokio::test]
async fn test_engine_initialization_custom_config() -> NirvResult<()> {
    let mut config = EngineConfig::default();
    
    // Use a different port to avoid conflicts
    config.protocol_adapters.clear();
    config.protocol_adapters.push(ProtocolConfig {
        protocol_type: ConfigProtocolType::MySQL,
        bind_address: "127.0.0.1".to_string(),
        port: 13306, // Use a different port
        tls_config: None,
        max_connections: Some(50),
        connection_timeout: Some(30),
    });
    
    // Add a mock connector configuration
    config.connectors.insert("test_mock".to_string(), nirv_engine::ConnectorConfig {
        connector_type: nirv_engine::ConnectorType::Mock,
        connection_string: None,
        parameters: HashMap::new(),
        pool_config: None,
        timeout_config: None,
    });
    
    let mut engine = Engine::new(config);
    
    // Initialize the engine for testing (without starting servers)
    let result = engine.initialize_for_testing().await;
    assert!(result.is_ok(), "Engine initialization should succeed with custom config");
    
    // Shutdown the engine
    engine.shutdown().await?;
    
    Ok(())
}

/// Test engine builder pattern
#[tokio::test]
async fn test_engine_builder() -> NirvResult<()> {
    let mut config = EngineConfig::default();
    config.protocol_adapters.clear(); // Remove default protocol adapters to avoid port conflicts
    
    let query_parser = Arc::new(DefaultQueryParser::new()?);
    let query_planner = Arc::new(DefaultQueryPlanner::new());
    let query_executor = Arc::new(RwLock::new(DefaultQueryExecutor::new()));
    let dispatcher = Arc::new(RwLock::new(DefaultDispatcher::new()));
    
    let mut engine = EngineBuilder::new()
        .with_config(config)
        .with_query_parser(query_parser)
        .with_query_planner(query_planner)
        .with_query_executor(query_executor)
        .with_dispatcher(dispatcher)
        .build()?;
    
    // Initialize the engine for testing
    let result = engine.initialize_for_testing().await;
    assert!(result.is_ok(), "Engine built with builder pattern should initialize successfully");
    
    // Shutdown the engine
    engine.shutdown().await?;
    
    Ok(())
}

/// Test engine query execution end-to-end
#[tokio::test]
async fn test_engine_query_execution() -> NirvResult<()> {
    let mut config = EngineConfig::default();
    config.protocol_adapters.clear(); // Remove default protocol adapters
    
    let mut engine = Engine::new(config);
    
    // Initialize the engine first
    engine.initialize_for_testing().await?;
    
    // Register a mock connector
    let mut mock_connector = Box::new(MockConnector::new());
    let connector_config = ConnectorInitConfig::new();
    mock_connector.connect(connector_config).await?;
    engine.register_connector("mock", mock_connector).await?;
    
    // Execute a simple query
    let sql = "SELECT * FROM source('mock.users') LIMIT 5";
    let result = engine.execute_query(sql).await;
    
    if let Err(ref e) = result {
        eprintln!("Query execution failed: {:?}", e);
    }
    assert!(result.is_ok(), "Query execution should succeed");
    
    let query_result = result?;
    assert!(!query_result.is_empty(), "Query should return results");
    assert!(query_result.execution_time.as_millis() > 0, "Execution time should be recorded");
    
    Ok(())
}

/// Test engine query execution with invalid SQL
#[tokio::test]
async fn test_engine_query_execution_invalid_sql() -> NirvResult<()> {
    let config = EngineConfig::default();
    let engine = Engine::new(config);
    
    // Execute invalid SQL
    let sql = "INVALID SQL SYNTAX";
    let result = engine.execute_query(sql).await;
    
    assert!(result.is_err(), "Invalid SQL should return an error");
    
    match result.unwrap_err() {
        NirvError::QueryParsing(_) => {
            // Expected error type
        }
        other => panic!("Expected QueryParsing error, got: {:?}", other),
    }
    
    Ok(())
}

/// Test engine query execution with unregistered data source
#[tokio::test]
async fn test_engine_query_execution_unregistered_source() -> NirvResult<()> {
    let mut config = EngineConfig::default();
    config.protocol_adapters.clear(); // Remove default protocol adapters
    
    let mut engine = Engine::new(config);
    
    // Initialize the engine first
    engine.initialize_for_testing().await?;
    
    // Execute query with unregistered source
    let sql = "SELECT * FROM source('unregistered.table')";
    let result = engine.execute_query(sql).await;
    
    assert!(result.is_err(), "Query with unregistered source should return an error");
    
    match result.unwrap_err() {
        NirvError::Dispatcher(_) | NirvError::Internal(_) => {
            // Expected error types (Internal errors can occur during routing)
        }
        other => panic!("Expected Dispatcher or Internal error, got: {:?}", other),
    }
    
    Ok(())
}

/// Test engine connector registration and listing
#[tokio::test]
async fn test_engine_connector_management() -> NirvResult<()> {
    let mut config = EngineConfig::default();
    config.protocol_adapters.clear(); // Remove default protocol adapters
    
    let mut engine = Engine::new(config);
    
    // Initialize the engine first
    engine.initialize_for_testing().await?;
    
    // Initially no types should be available
    let initial_types = engine.list_available_types().await;
    assert!(initial_types.is_empty(), "No types should be available initially");
    
    // Register a mock connector
    let mut mock_connector = Box::new(MockConnector::new());
    let connector_config = ConnectorInitConfig::new();
    mock_connector.connect(connector_config).await?;
    engine.register_connector("test_mock", mock_connector).await?;
    
    // Now one type should be available
    let types_after_registration = engine.list_available_types().await;
    assert_eq!(types_after_registration.len(), 1, "One type should be available after registration");
    assert!(types_after_registration.contains(&"test_mock".to_string()), "Registered type should be in the list");
    
    Ok(())
}

/// Test engine graceful shutdown
#[tokio::test]
async fn test_engine_graceful_shutdown() -> NirvResult<()> {
    let mut config = EngineConfig::default();
    config.protocol_adapters.clear(); // Remove default protocol adapters to avoid port conflicts
    
    let mut engine = Engine::new(config);
    
    // Initialize the engine for testing
    engine.initialize_for_testing().await?;
    
    // Shutdown should complete without errors
    let shutdown_result = engine.shutdown().await;
    assert!(shutdown_result.is_ok(), "Graceful shutdown should succeed");
    
    Ok(())
}

/// Test engine configuration loading and validation
#[tokio::test]
async fn test_engine_configuration_validation() -> NirvResult<()> {
    // Test with minimal valid configuration
    let minimal_config = EngineConfig {
        protocol_adapters: vec![],
        connectors: HashMap::new(),
        dispatcher: DispatcherConfig::default(),
        security: SecurityConfig::default(),
    };
    
    let mut engine = Engine::new(minimal_config);
    let result = engine.initialize().await;
    assert!(result.is_ok(), "Engine should initialize with minimal config");
    
    engine.shutdown().await?;
    
    Ok(())
}

/// Test engine error handling and recovery
#[tokio::test]
async fn test_engine_error_handling() -> NirvResult<()> {
    let mut config = EngineConfig::default();
    config.protocol_adapters.clear(); // Remove default protocol adapters
    
    let mut engine = Engine::new(config);
    
    // Initialize the engine first
    engine.initialize_for_testing().await?;
    
    // Register a mock connector
    let mut mock_connector = Box::new(MockConnector::new());
    let connector_config = ConnectorInitConfig::new();
    mock_connector.connect(connector_config).await?;
    engine.register_connector("mock", mock_connector).await?;
    
    // Test various error scenarios
    let test_cases = vec![
        ("", "Empty query should be handled gracefully"),
        ("SELECT", "Incomplete query should be handled gracefully"),
        ("SELECT * FROM", "Query without source should be handled gracefully"),
        ("SELECT * FROM source('invalid')", "Invalid source format should be handled gracefully"),
    ];
    
    for (sql, description) in test_cases {
        let result = engine.execute_query(sql).await;
        assert!(result.is_err(), "{}", description);
        
        // Ensure the error is of the expected type
        match result.unwrap_err() {
            NirvError::QueryParsing(_) | NirvError::Dispatcher(_) | NirvError::Internal(_) => {
                // Expected error types (Internal errors can occur during parsing/execution)
            }
            other => panic!("Unexpected error type for '{}': {:?}", sql, other),
        }
    }
    
    Ok(())
}

/// Test engine component integration
#[tokio::test]
async fn test_engine_component_integration() -> NirvResult<()> {
    let mut config = EngineConfig::default();
    config.protocol_adapters.clear(); // Remove default protocol adapters
    
    let mut engine = Engine::new(config);
    
    // Initialize the engine first
    engine.initialize_for_testing().await?;
    
    // Register a mock connector
    let mut mock_connector = Box::new(MockConnector::new());
    let connector_config = ConnectorInitConfig::new();
    mock_connector.connect(connector_config).await?;
    
    engine.register_connector("mock", mock_connector).await?;
    
    // Test that all components work together
    let sql = "SELECT id, name FROM source('mock.users') WHERE id > 0 ORDER BY name LIMIT 3";
    let result = engine.execute_query(sql).await?;
    
    // Verify the result structure
    assert!(!result.is_empty(), "Query should return results");
    assert!(result.columns.len() > 0, "Result should have column metadata");
    assert!(result.execution_time.as_millis() > 0, "Execution time should be recorded");
    
    // Verify that the query was processed through all components
    // (parser -> planner -> executor -> dispatcher -> connector)
    
    Ok(())
}

/// Test engine concurrent query execution
#[tokio::test]
async fn test_engine_concurrent_queries() -> NirvResult<()> {
    let mut config = EngineConfig::default();
    config.protocol_adapters.clear(); // Remove default protocol adapters
    
    let mut engine = Engine::new(config);
    
    // Initialize the engine first
    engine.initialize_for_testing().await?;
    
    let engine = Arc::new(engine);
    
    // Register a mock connector
    let mut mock_connector = Box::new(MockConnector::new());
    let connector_config = ConnectorInitConfig::new();
    mock_connector.connect(connector_config).await?;
    engine.register_connector("mock", mock_connector).await?;
    
    // Execute multiple queries concurrently
    let mut handles = Vec::new();
    
    for i in 0..5 {
        let engine_clone = engine.clone();
        let handle = tokio::spawn(async move {
            let sql = format!("SELECT * FROM source('mock.users') WHERE id = {} LIMIT 1", i);
            engine_clone.execute_query(&sql).await
        });
        handles.push(handle);
    }
    
    // Wait for all queries to complete
    let mut results = Vec::new();
    for handle in handles {
        let result = handle.await.expect("Task should complete");
        results.push(result);
    }
    
    // All queries should succeed
    for (i, result) in results.into_iter().enumerate() {
        assert!(result.is_ok(), "Concurrent query {} should succeed", i);
    }
    
    Ok(())
}

/// Test engine with different protocol configurations
#[tokio::test]
async fn test_engine_protocol_configurations() -> NirvResult<()> {
    let mut config = EngineConfig::default();
    
    // Add multiple protocol adapters with different ports to avoid conflicts
    config.protocol_adapters = vec![
        ProtocolConfig {
            protocol_type: ConfigProtocolType::PostgreSQL,
            bind_address: "127.0.0.1".to_string(),
            port: 15432, // Use different port
            tls_config: None,
            max_connections: Some(100),
            connection_timeout: Some(30),
        },
        ProtocolConfig {
            protocol_type: ConfigProtocolType::MySQL,
            bind_address: "127.0.0.1".to_string(),
            port: 13306, // Use different port
            tls_config: None,
            max_connections: Some(50),
            connection_timeout: Some(30),
        },
    ];
    
    let mut engine = Engine::new(config);
    
    // Initialize for testing (without starting servers to avoid port conflicts)
    let result = engine.initialize_for_testing().await;
    assert!(result.is_ok(), "Engine should initialize with multiple protocol adapters");
    
    engine.shutdown().await?;
    
    Ok(())
}