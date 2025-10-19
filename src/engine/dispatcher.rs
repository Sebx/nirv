use async_trait::async_trait;
use std::collections::HashMap;
use crate::utils::{
    types::{InternalQuery, ConnectorQuery, QueryResult, DataSource},
    error::{NirvResult, DispatcherError, NirvError},
};
use crate::connectors::{Connector, ConnectorRegistry};

/// Central routing component that manages data object type resolution and connector selection
#[async_trait]
pub trait Dispatcher: Send + Sync {
    /// Register a connector for a specific data object type
    async fn register_connector(&mut self, object_type: &str, connector: Box<dyn Connector>) -> NirvResult<()>;
    
    /// Route a query to appropriate connectors based on data object types
    async fn route_query(&self, query: &InternalQuery) -> NirvResult<Vec<ConnectorQuery>>;
    
    /// Execute a distributed query across multiple connectors
    async fn execute_distributed_query(&self, queries: Vec<ConnectorQuery>) -> NirvResult<QueryResult>;
    
    /// List all available data object types
    fn list_available_types(&self) -> Vec<String>;
    
    /// Check if a data object type is registered
    fn is_type_registered(&self, object_type: &str) -> bool;
    
    /// Get connector for a specific data object type
    fn get_connector(&self, object_type: &str) -> Option<&dyn Connector>;
}

/// Data object type registry that maps types to their corresponding connectors
#[derive(Debug)]
pub struct DataObjectTypeRegistry {
    /// Maps data object type names to connector names
    type_to_connector: HashMap<String, String>,
    /// Maps connector names to their capabilities
    connector_capabilities: HashMap<String, ConnectorCapabilities>,
}

/// Capabilities of a connector for routing decisions
#[derive(Debug, Clone)]
pub struct ConnectorCapabilities {
    pub supports_joins: bool,
    pub supports_aggregations: bool,
    pub supports_subqueries: bool,
    pub max_concurrent_queries: Option<u32>,
}

impl DataObjectTypeRegistry {
    /// Create a new empty registry
    pub fn new() -> Self {
        Self {
            type_to_connector: HashMap::new(),
            connector_capabilities: HashMap::new(),
        }
    }
    
    /// Register a data object type with its connector
    pub fn register_type(&mut self, object_type: &str, connector_name: &str, capabilities: ConnectorCapabilities) -> NirvResult<()> {
        if self.type_to_connector.contains_key(object_type) {
            return Err(NirvError::Dispatcher(DispatcherError::RegistrationFailed(
                format!("Data object type '{}' is already registered", object_type)
            )));
        }
        
        self.type_to_connector.insert(object_type.to_string(), connector_name.to_string());
        self.connector_capabilities.insert(connector_name.to_string(), capabilities);
        Ok(())
    }
    
    /// Get the connector name for a data object type
    pub fn get_connector_for_type(&self, object_type: &str) -> Option<&String> {
        self.type_to_connector.get(object_type)
    }
    
    /// Get capabilities for a connector
    pub fn get_connector_capabilities(&self, connector_name: &str) -> Option<&ConnectorCapabilities> {
        self.connector_capabilities.get(connector_name)
    }
    
    /// List all registered data object types
    pub fn list_types(&self) -> Vec<String> {
        self.type_to_connector.keys().cloned().collect()
    }
    
    /// Check if a type is registered
    pub fn is_type_registered(&self, object_type: &str) -> bool {
        self.type_to_connector.contains_key(object_type)
    }
    
    /// Unregister a data object type
    pub fn unregister_type(&mut self, object_type: &str) -> Option<String> {
        self.type_to_connector.remove(object_type)
    }
}

impl Default for DataObjectTypeRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Default implementation of the Dispatcher trait
pub struct DefaultDispatcher {
    /// Registry for managing connectors
    connector_registry: ConnectorRegistry,
    /// Registry for mapping data object types to connectors
    type_registry: DataObjectTypeRegistry,
}

impl DefaultDispatcher {
    /// Create a new dispatcher with empty registries
    pub fn new() -> Self {
        Self {
            connector_registry: ConnectorRegistry::new(),
            type_registry: DataObjectTypeRegistry::new(),
        }
    }
    
    /// Create a dispatcher with existing registries
    pub fn with_registries(connector_registry: ConnectorRegistry, type_registry: DataObjectTypeRegistry) -> Self {
        Self {
            connector_registry,
            type_registry,
        }
    }
    
    /// Extract data sources from a query
    fn extract_data_sources<'a>(&self, query: &'a InternalQuery) -> Vec<&'a DataSource> {
        query.sources.iter().collect()
    }
    
    /// Validate that all data sources in a query are registered
    fn validate_data_sources(&self, sources: &[&DataSource]) -> NirvResult<()> {
        for source in sources {
            if !self.type_registry.is_type_registered(&source.object_type) {
                return Err(NirvError::Dispatcher(DispatcherError::UnregisteredObjectType(
                    format!("Data object type '{}' is not registered. Available types: {:?}", 
                           source.object_type, 
                           self.type_registry.list_types())
                )));
            }
        }
        Ok(())
    }
    
    /// Create connector queries for single-source routing
    fn create_connector_queries(&self, query: &InternalQuery, sources: &[&DataSource]) -> NirvResult<Vec<ConnectorQuery>> {
        let mut connector_queries = Vec::new();
        
        for source in sources {
            let connector_name = self.type_registry
                .get_connector_for_type(&source.object_type)
                .ok_or_else(|| NirvError::Dispatcher(DispatcherError::UnregisteredObjectType(
                    source.object_type.clone()
                )))?;
            
            let connector = self.connector_registry
                .get(connector_name)
                .ok_or_else(|| NirvError::Dispatcher(DispatcherError::NoSuitableConnector))?;
            
            let connector_query = ConnectorQuery {
                connector_type: connector.get_connector_type(),
                query: query.clone(),
                connection_params: HashMap::new(),
            };
            
            connector_queries.push(connector_query);
        }
        
        Ok(connector_queries)
    }
}

impl Default for DefaultDispatcher {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Dispatcher for DefaultDispatcher {
    async fn register_connector(&mut self, object_type: &str, connector: Box<dyn Connector>) -> NirvResult<()> {
        let connector_name = format!("{}_{}", object_type, self.connector_registry.len());
        let capabilities = ConnectorCapabilities {
            supports_joins: connector.get_capabilities().supports_joins,
            supports_aggregations: connector.get_capabilities().supports_aggregations,
            supports_subqueries: connector.get_capabilities().supports_subqueries,
            max_concurrent_queries: connector.get_capabilities().max_concurrent_queries,
        };
        
        // Register the connector in the connector registry
        self.connector_registry.register(connector_name.clone(), connector)?;
        
        // Register the data object type mapping
        self.type_registry.register_type(object_type, &connector_name, capabilities)?;
        
        Ok(())
    }
    
    async fn route_query(&self, query: &InternalQuery) -> NirvResult<Vec<ConnectorQuery>> {
        // Extract data sources from the query
        let sources = self.extract_data_sources(query);
        
        if sources.is_empty() {
            return Err(NirvError::Dispatcher(DispatcherError::RoutingFailed(
                "No data sources found in query".to_string()
            )));
        }
        
        // Validate that all data sources are registered
        self.validate_data_sources(&sources)?;
        
        // For MVP, we only support single-source queries
        if sources.len() > 1 {
            return Err(NirvError::Dispatcher(DispatcherError::CrossConnectorJoinUnsupported));
        }
        
        // Create connector queries for routing
        self.create_connector_queries(query, &sources)
    }
    
    async fn execute_distributed_query(&self, queries: Vec<ConnectorQuery>) -> NirvResult<QueryResult> {
        if queries.is_empty() {
            return Ok(QueryResult::new());
        }
        
        // For MVP, we only handle single connector queries
        if queries.len() > 1 {
            return Err(NirvError::Dispatcher(DispatcherError::CrossConnectorJoinUnsupported));
        }
        
        let connector_query = &queries[0];
        let connector_name = self.type_registry
            .get_connector_for_type(&connector_query.query.sources[0].object_type)
            .ok_or_else(|| NirvError::Dispatcher(DispatcherError::UnregisteredObjectType(
                connector_query.query.sources[0].object_type.clone()
            )))?;
        
        let connector = self.connector_registry
            .get(connector_name)
            .ok_or_else(|| NirvError::Dispatcher(DispatcherError::NoSuitableConnector))?;
        
        connector.execute_query(connector_query.clone()).await
    }
    
    fn list_available_types(&self) -> Vec<String> {
        self.type_registry.list_types()
    }
    
    fn is_type_registered(&self, object_type: &str) -> bool {
        self.type_registry.is_type_registered(object_type)
    }
    
    fn get_connector(&self, object_type: &str) -> Option<&dyn Connector> {
        let connector_name = self.type_registry.get_connector_for_type(object_type)?;
        self.connector_registry.get(connector_name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::utils::types::{QueryOperation, ConnectorType, Schema, ColumnMetadata, DataType};
    use crate::connectors::{ConnectorInitConfig, ConnectorCapabilities as ConnectorTraitCapabilities};
    use std::time::Duration;

    // Mock connector for testing
    struct TestConnector {
        connector_type: ConnectorType,
        connected: bool,
        capabilities: ConnectorTraitCapabilities,
    }

    impl TestConnector {
        fn new(connector_type: ConnectorType) -> Self {
            Self {
                connector_type,
                connected: false,
                capabilities: ConnectorTraitCapabilities::default(),
            }
        }
        
        fn with_capabilities(mut self, capabilities: ConnectorTraitCapabilities) -> Self {
            self.capabilities = capabilities;
            self
        }
    }

    #[async_trait]
    impl Connector for TestConnector {
        async fn connect(&mut self, _config: ConnectorInitConfig) -> NirvResult<()> {
            self.connected = true;
            Ok(())
        }

        async fn execute_query(&self, _query: ConnectorQuery) -> NirvResult<QueryResult> {
            let mut result = QueryResult::new();
            result.execution_time = Duration::from_millis(10);
            Ok(result)
        }

        async fn get_schema(&self, object_name: &str) -> NirvResult<Schema> {
            Ok(Schema {
                name: object_name.to_string(),
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
                primary_key: Some(vec!["id".to_string()]),
                indexes: vec![],
            })
        }

        async fn disconnect(&mut self) -> NirvResult<()> {
            self.connected = false;
            Ok(())
        }

        fn get_connector_type(&self) -> ConnectorType {
            self.connector_type.clone()
        }

        fn supports_transactions(&self) -> bool {
            self.capabilities.supports_transactions
        }

        fn is_connected(&self) -> bool {
            self.connected
        }

        fn get_capabilities(&self) -> ConnectorTraitCapabilities {
            self.capabilities.clone()
        }
    }

    #[test]
    fn test_data_object_type_registry_creation() {
        let registry = DataObjectTypeRegistry::new();
        
        assert!(registry.list_types().is_empty());
        assert!(!registry.is_type_registered("test_type"));
    }

    #[test]
    fn test_data_object_type_registry_register_type() {
        let mut registry = DataObjectTypeRegistry::new();
        let capabilities = ConnectorCapabilities {
            supports_joins: true,
            supports_aggregations: false,
            supports_subqueries: true,
            max_concurrent_queries: Some(5),
        };
        
        let result = registry.register_type("postgres", "postgres_connector", capabilities.clone());
        assert!(result.is_ok());
        
        assert!(registry.is_type_registered("postgres"));
        assert_eq!(registry.get_connector_for_type("postgres"), Some(&"postgres_connector".to_string()));
        
        let retrieved_capabilities = registry.get_connector_capabilities("postgres_connector");
        assert!(retrieved_capabilities.is_some());
        assert!(retrieved_capabilities.unwrap().supports_joins);
        assert!(!retrieved_capabilities.unwrap().supports_aggregations);
    }

    #[test]
    fn test_data_object_type_registry_duplicate_registration() {
        let mut registry = DataObjectTypeRegistry::new();
        let capabilities = ConnectorCapabilities {
            supports_joins: false,
            supports_aggregations: false,
            supports_subqueries: false,
            max_concurrent_queries: Some(1),
        };
        
        // First registration should succeed
        let result1 = registry.register_type("postgres", "connector1", capabilities.clone());
        assert!(result1.is_ok());
        
        // Second registration with same type should fail
        let result2 = registry.register_type("postgres", "connector2", capabilities);
        assert!(result2.is_err());
        
        match result2.unwrap_err() {
            NirvError::Dispatcher(DispatcherError::RegistrationFailed(msg)) => {
                assert!(msg.contains("already registered"));
            }
            _ => panic!("Expected RegistrationFailed error"),
        }
    }

    #[test]
    fn test_data_object_type_registry_list_types() {
        let mut registry = DataObjectTypeRegistry::new();
        let capabilities = ConnectorCapabilities {
            supports_joins: false,
            supports_aggregations: false,
            supports_subqueries: false,
            max_concurrent_queries: Some(1),
        };
        
        registry.register_type("postgres", "pg_connector", capabilities.clone()).unwrap();
        registry.register_type("mysql", "mysql_connector", capabilities.clone()).unwrap();
        registry.register_type("file", "file_connector", capabilities).unwrap();
        
        let types = registry.list_types();
        assert_eq!(types.len(), 3);
        assert!(types.contains(&"postgres".to_string()));
        assert!(types.contains(&"mysql".to_string()));
        assert!(types.contains(&"file".to_string()));
    }

    #[test]
    fn test_data_object_type_registry_unregister_type() {
        let mut registry = DataObjectTypeRegistry::new();
        let capabilities = ConnectorCapabilities {
            supports_joins: false,
            supports_aggregations: false,
            supports_subqueries: false,
            max_concurrent_queries: Some(1),
        };
        
        registry.register_type("postgres", "pg_connector", capabilities).unwrap();
        assert!(registry.is_type_registered("postgres"));
        
        let removed = registry.unregister_type("postgres");
        assert_eq!(removed, Some("pg_connector".to_string()));
        assert!(!registry.is_type_registered("postgres"));
        
        // Try to unregister non-existent type
        let non_existent = registry.unregister_type("non_existent");
        assert_eq!(non_existent, None);
    }

    #[test]
    fn test_default_dispatcher_creation() {
        let dispatcher = DefaultDispatcher::new();
        
        assert!(dispatcher.list_available_types().is_empty());
        assert!(!dispatcher.is_type_registered("test_type"));
    }

    #[tokio::test]
    async fn test_dispatcher_register_connector() {
        let mut dispatcher = DefaultDispatcher::new();
        let connector = Box::new(TestConnector::new(ConnectorType::Mock));
        
        let result = dispatcher.register_connector("mock", connector).await;
        assert!(result.is_ok());
        
        assert!(dispatcher.is_type_registered("mock"));
        assert_eq!(dispatcher.list_available_types(), vec!["mock".to_string()]);
    }

    #[tokio::test]
    async fn test_dispatcher_register_multiple_connectors() {
        let mut dispatcher = DefaultDispatcher::new();
        
        let mock_connector = Box::new(TestConnector::new(ConnectorType::Mock));
        let postgres_connector = Box::new(TestConnector::new(ConnectorType::PostgreSQL));
        
        dispatcher.register_connector("mock", mock_connector).await.unwrap();
        dispatcher.register_connector("postgres", postgres_connector).await.unwrap();
        
        let types = dispatcher.list_available_types();
        assert_eq!(types.len(), 2);
        assert!(types.contains(&"mock".to_string()));
        assert!(types.contains(&"postgres".to_string()));
    }

    #[tokio::test]
    async fn test_dispatcher_get_connector() {
        let mut dispatcher = DefaultDispatcher::new();
        let connector = Box::new(TestConnector::new(ConnectorType::Mock));
        
        dispatcher.register_connector("mock", connector).await.unwrap();
        
        let retrieved = dispatcher.get_connector("mock");
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().get_connector_type(), ConnectorType::Mock);
        
        let non_existent = dispatcher.get_connector("non_existent");
        assert!(non_existent.is_none());
    }

    #[tokio::test]
    async fn test_dispatcher_route_query_single_source() {
        let mut dispatcher = DefaultDispatcher::new();
        let connector = Box::new(TestConnector::new(ConnectorType::Mock));
        
        dispatcher.register_connector("mock", connector).await.unwrap();
        
        let mut query = InternalQuery::new(QueryOperation::Select);
        query.sources.push(DataSource {
            object_type: "mock".to_string(),
            identifier: "test_table".to_string(),
            alias: None,
        });
        
        let result = dispatcher.route_query(&query).await;
        assert!(result.is_ok());
        
        let connector_queries = result.unwrap();
        assert_eq!(connector_queries.len(), 1);
        assert_eq!(connector_queries[0].connector_type, ConnectorType::Mock);
    }

    #[tokio::test]
    async fn test_dispatcher_route_query_unregistered_type() {
        let dispatcher = DefaultDispatcher::new();
        
        let mut query = InternalQuery::new(QueryOperation::Select);
        query.sources.push(DataSource {
            object_type: "unregistered".to_string(),
            identifier: "test_table".to_string(),
            alias: None,
        });
        
        let result = dispatcher.route_query(&query).await;
        assert!(result.is_err());
        
        match result.unwrap_err() {
            NirvError::Dispatcher(DispatcherError::UnregisteredObjectType(msg)) => {
                assert!(msg.contains("unregistered"));
                assert!(msg.contains("not registered"));
            }
            _ => panic!("Expected UnregisteredObjectType error"),
        }
    }

    #[tokio::test]
    async fn test_dispatcher_route_query_no_sources() {
        let dispatcher = DefaultDispatcher::new();
        let query = InternalQuery::new(QueryOperation::Select);
        
        let result = dispatcher.route_query(&query).await;
        assert!(result.is_err());
        
        match result.unwrap_err() {
            NirvError::Dispatcher(DispatcherError::RoutingFailed(msg)) => {
                assert!(msg.contains("No data sources found"));
            }
            _ => panic!("Expected RoutingFailed error"),
        }
    }

    #[tokio::test]
    async fn test_dispatcher_route_query_multiple_sources_unsupported() {
        let mut dispatcher = DefaultDispatcher::new();
        let connector = Box::new(TestConnector::new(ConnectorType::Mock));
        
        dispatcher.register_connector("mock", connector).await.unwrap();
        
        let mut query = InternalQuery::new(QueryOperation::Select);
        query.sources.push(DataSource {
            object_type: "mock".to_string(),
            identifier: "table1".to_string(),
            alias: None,
        });
        query.sources.push(DataSource {
            object_type: "mock".to_string(),
            identifier: "table2".to_string(),
            alias: None,
        });
        
        let result = dispatcher.route_query(&query).await;
        assert!(result.is_err());
        
        match result.unwrap_err() {
            NirvError::Dispatcher(DispatcherError::CrossConnectorJoinUnsupported) => {},
            _ => panic!("Expected CrossConnectorJoinUnsupported error"),
        }
    }

    #[tokio::test]
    async fn test_dispatcher_execute_distributed_query() {
        let mut dispatcher = DefaultDispatcher::new();
        let connector = Box::new(TestConnector::new(ConnectorType::Mock));
        
        dispatcher.register_connector("mock", connector).await.unwrap();
        
        let mut query = InternalQuery::new(QueryOperation::Select);
        query.sources.push(DataSource {
            object_type: "mock".to_string(),
            identifier: "test_table".to_string(),
            alias: None,
        });
        
        let connector_query = ConnectorQuery {
            connector_type: ConnectorType::Mock,
            query,
            connection_params: HashMap::new(),
        };
        
        let result = dispatcher.execute_distributed_query(vec![connector_query]).await;
        assert!(result.is_ok());
        
        let query_result = result.unwrap();
        assert!(query_result.execution_time > Duration::from_millis(0));
    }

    #[tokio::test]
    async fn test_dispatcher_execute_distributed_query_empty() {
        let dispatcher = DefaultDispatcher::new();
        
        let result = dispatcher.execute_distributed_query(vec![]).await;
        assert!(result.is_ok());
        
        let query_result = result.unwrap();
        assert_eq!(query_result.row_count(), 0);
    }

    #[tokio::test]
    async fn test_dispatcher_execute_distributed_query_multiple_unsupported() {
        let dispatcher = DefaultDispatcher::new();
        
        let query1 = ConnectorQuery {
            connector_type: ConnectorType::Mock,
            query: InternalQuery::new(QueryOperation::Select),
            connection_params: HashMap::new(),
        };
        
        let query2 = ConnectorQuery {
            connector_type: ConnectorType::PostgreSQL,
            query: InternalQuery::new(QueryOperation::Select),
            connection_params: HashMap::new(),
        };
        
        let result = dispatcher.execute_distributed_query(vec![query1, query2]).await;
        assert!(result.is_err());
        
        match result.unwrap_err() {
            NirvError::Dispatcher(DispatcherError::CrossConnectorJoinUnsupported) => {},
            _ => panic!("Expected CrossConnectorJoinUnsupported error"),
        }
    }

    #[test]
    fn test_connector_capabilities_creation() {
        let capabilities = ConnectorCapabilities {
            supports_joins: true,
            supports_aggregations: false,
            supports_subqueries: true,
            max_concurrent_queries: Some(10),
        };
        
        assert!(capabilities.supports_joins);
        assert!(!capabilities.supports_aggregations);
        assert!(capabilities.supports_subqueries);
        assert_eq!(capabilities.max_concurrent_queries, Some(10));
    }
}