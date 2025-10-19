use async_trait::async_trait;
use std::collections::HashMap;
use crate::utils::{
    types::{ConnectorType, ConnectorQuery, QueryResult, Schema},
    error::{ConnectorError, NirvResult},
};

/// Configuration for connector initialization
#[derive(Debug, Clone)]
pub struct ConnectorInitConfig {
    pub connection_params: HashMap<String, String>,
    pub timeout_seconds: Option<u64>,
    pub max_connections: Option<u32>,
}

impl ConnectorInitConfig {
    /// Create a new connector configuration
    pub fn new() -> Self {
        Self {
            connection_params: HashMap::new(),
            timeout_seconds: Some(30),
            max_connections: Some(10),
        }
    }
    
    /// Add a connection parameter
    pub fn with_param(mut self, key: &str, value: &str) -> Self {
        self.connection_params.insert(key.to_string(), value.to_string());
        self
    }
    
    /// Set timeout in seconds
    pub fn with_timeout(mut self, seconds: u64) -> Self {
        self.timeout_seconds = Some(seconds);
        self
    }
    
    /// Set maximum connections
    pub fn with_max_connections(mut self, max: u32) -> Self {
        self.max_connections = Some(max);
        self
    }
}

impl Default for ConnectorInitConfig {
    fn default() -> Self {
        Self::new()
    }
}

/// Base trait for all data source connectors
#[async_trait]
pub trait Connector: Send + Sync {
    /// Establish connection to the backend data source
    async fn connect(&mut self, config: ConnectorInitConfig) -> NirvResult<()>;
    
    /// Execute a query against the connected data source
    async fn execute_query(&self, query: ConnectorQuery) -> NirvResult<QueryResult>;
    
    /// Retrieve schema information for a specific data object
    async fn get_schema(&self, object_name: &str) -> NirvResult<Schema>;
    
    /// Close connection and cleanup resources
    async fn disconnect(&mut self) -> NirvResult<()>;
    
    /// Get the type of this connector
    fn get_connector_type(&self) -> ConnectorType;
    
    /// Check if this connector supports transactions
    fn supports_transactions(&self) -> bool;
    
    /// Check if the connector is currently connected
    fn is_connected(&self) -> bool;
    
    /// Get connector-specific capabilities
    fn get_capabilities(&self) -> ConnectorCapabilities;
}

/// Capabilities supported by a connector
#[derive(Debug, Clone)]
pub struct ConnectorCapabilities {
    pub supports_joins: bool,
    pub supports_aggregations: bool,
    pub supports_subqueries: bool,
    pub supports_transactions: bool,
    pub supports_schema_introspection: bool,
    pub max_concurrent_queries: Option<u32>,
}

impl Default for ConnectorCapabilities {
    fn default() -> Self {
        Self {
            supports_joins: false,
            supports_aggregations: false,
            supports_subqueries: false,
            supports_transactions: false,
            supports_schema_introspection: true,
            max_concurrent_queries: Some(1),
        }
    }
}

/// Registry for managing connector instances
pub struct ConnectorRegistry {
    connectors: HashMap<String, Box<dyn Connector>>,
}

impl ConnectorRegistry {
    /// Create a new empty registry
    pub fn new() -> Self {
        Self {
            connectors: HashMap::new(),
        }
    }
    
    /// Register a connector with a given name
    pub fn register(&mut self, name: String, connector: Box<dyn Connector>) -> NirvResult<()> {
        if self.connectors.contains_key(&name) {
            return Err(crate::utils::error::NirvError::Dispatcher(
                crate::utils::error::DispatcherError::RegistrationFailed(
                    format!("Connector '{}' is already registered", name)
                )
            ));
        }
        
        self.connectors.insert(name, connector);
        Ok(())
    }
    
    /// Get a connector by name
    pub fn get(&self, name: &str) -> Option<&dyn Connector> {
        self.connectors.get(name).map(|c| c.as_ref())
    }
    
    /// Get a mutable reference to a connector by name
    pub fn get_mut(&mut self, name: &str) -> Option<&mut Box<dyn Connector>> {
        self.connectors.get_mut(name)
    }
    
    /// List all registered connector names
    pub fn list_connectors(&self) -> Vec<String> {
        self.connectors.keys().cloned().collect()
    }
    
    /// Remove a connector from the registry
    pub fn unregister(&mut self, name: &str) -> Option<Box<dyn Connector>> {
        self.connectors.remove(name)
    }
    
    /// Check if a connector is registered
    pub fn contains(&self, name: &str) -> bool {
        self.connectors.contains_key(name)
    }
    
    /// Get the number of registered connectors
    pub fn len(&self) -> usize {
        self.connectors.len()
    }
    
    /// Check if the registry is empty
    pub fn is_empty(&self) -> bool {
        self.connectors.is_empty()
    }
}

impl Default for ConnectorRegistry {
    fn default() -> Self {
        Self::new()
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::utils::types::ConnectorType;

    #[test]
    fn test_connector_init_config_creation() {
        let config = ConnectorInitConfig::new();
        
        assert!(config.connection_params.is_empty());
        assert_eq!(config.timeout_seconds, Some(30));
        assert_eq!(config.max_connections, Some(10));
    }

    #[test]
    fn test_connector_init_config_builder_pattern() {
        let config = ConnectorInitConfig::new()
            .with_param("host", "localhost")
            .with_param("port", "5432")
            .with_timeout(60)
            .with_max_connections(20);
        
        assert_eq!(config.connection_params.get("host"), Some(&"localhost".to_string()));
        assert_eq!(config.connection_params.get("port"), Some(&"5432".to_string()));
        assert_eq!(config.timeout_seconds, Some(60));
        assert_eq!(config.max_connections, Some(20));
    }

    #[test]
    fn test_connector_init_config_default() {
        let config = ConnectorInitConfig::default();
        
        assert!(config.connection_params.is_empty());
        assert_eq!(config.timeout_seconds, Some(30));
        assert_eq!(config.max_connections, Some(10));
    }

    #[test]
    fn test_connector_capabilities_default() {
        let capabilities = ConnectorCapabilities::default();
        
        assert!(!capabilities.supports_joins);
        assert!(!capabilities.supports_aggregations);
        assert!(!capabilities.supports_subqueries);
        assert!(!capabilities.supports_transactions);
        assert!(capabilities.supports_schema_introspection);
        assert_eq!(capabilities.max_concurrent_queries, Some(1));
    }

    #[test]
    fn test_connector_registry_creation() {
        let registry = ConnectorRegistry::new();
        
        assert!(registry.is_empty());
        assert_eq!(registry.len(), 0);
        assert!(registry.list_connectors().is_empty());
    }

    #[test]
    fn test_connector_registry_default() {
        let registry = ConnectorRegistry::default();
        
        assert!(registry.is_empty());
        assert_eq!(registry.len(), 0);
    }

    // Mock connector for testing registry functionality
    struct TestConnector {
        connector_type: ConnectorType,
        connected: bool,
    }

    impl TestConnector {
        fn new(connector_type: ConnectorType) -> Self {
            Self {
                connector_type,
                connected: false,
            }
        }
    }

    #[async_trait]
    impl Connector for TestConnector {
        async fn connect(&mut self, _config: ConnectorInitConfig) -> NirvResult<()> {
            self.connected = true;
            Ok(())
        }

        async fn execute_query(&self, _query: ConnectorQuery) -> NirvResult<QueryResult> {
            Ok(QueryResult::new())
        }

        async fn get_schema(&self, _object_name: &str) -> NirvResult<Schema> {
            Ok(Schema {
                name: "test".to_string(),
                columns: vec![],
                primary_key: None,
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
            false
        }

        fn is_connected(&self) -> bool {
            self.connected
        }

        fn get_capabilities(&self) -> ConnectorCapabilities {
            ConnectorCapabilities::default()
        }
    }

    #[test]
    fn test_connector_registry_register_and_get() {
        let mut registry = ConnectorRegistry::new();
        let connector = Box::new(TestConnector::new(ConnectorType::Mock));
        
        // Register connector
        let result = registry.register("test_connector".to_string(), connector);
        assert!(result.is_ok());
        
        // Check registry state
        assert!(!registry.is_empty());
        assert_eq!(registry.len(), 1);
        assert!(registry.contains("test_connector"));
        
        // Get connector
        let retrieved = registry.get("test_connector");
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().get_connector_type(), ConnectorType::Mock);
        
        // List connectors
        let connectors = registry.list_connectors();
        assert_eq!(connectors.len(), 1);
        assert!(connectors.contains(&"test_connector".to_string()));
    }

    #[test]
    fn test_connector_registry_duplicate_registration() {
        let mut registry = ConnectorRegistry::new();
        let connector1 = Box::new(TestConnector::new(ConnectorType::Mock));
        let connector2 = Box::new(TestConnector::new(ConnectorType::PostgreSQL));
        
        // Register first connector
        let result1 = registry.register("test_connector".to_string(), connector1);
        assert!(result1.is_ok());
        
        // Try to register with same name
        let result2 = registry.register("test_connector".to_string(), connector2);
        assert!(result2.is_err());
        
        // Should still have only one connector
        assert_eq!(registry.len(), 1);
    }

    #[test]
    fn test_connector_registry_get_mut() {
        let mut registry = ConnectorRegistry::new();
        let connector = Box::new(TestConnector::new(ConnectorType::Mock));
        
        registry.register("test_connector".to_string(), connector).unwrap();
        
        // Get mutable reference
        let connector_mut = registry.get_mut("test_connector");
        assert!(connector_mut.is_some());
        
        // Get non-existent connector
        let non_existent = registry.get_mut("non_existent");
        assert!(non_existent.is_none());
    }

    #[test]
    fn test_connector_registry_unregister() {
        let mut registry = ConnectorRegistry::new();
        let connector = Box::new(TestConnector::new(ConnectorType::Mock));
        
        registry.register("test_connector".to_string(), connector).unwrap();
        assert_eq!(registry.len(), 1);
        
        // Unregister connector
        let removed = registry.unregister("test_connector");
        assert!(removed.is_some());
        assert_eq!(registry.len(), 0);
        assert!(registry.is_empty());
        
        // Try to unregister non-existent connector
        let non_existent = registry.unregister("non_existent");
        assert!(non_existent.is_none());
    }

    #[test]
    fn test_connector_registry_get_non_existent() {
        let registry = ConnectorRegistry::new();
        
        let connector = registry.get("non_existent");
        assert!(connector.is_none());
        
        assert!(!registry.contains("non_existent"));
    }
}