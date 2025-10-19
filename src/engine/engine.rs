use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::signal;
use tokio::task::JoinHandle;
use tokio::net::TcpListener;

use crate::{
    engine::{
        Dispatcher, DefaultDispatcher,
        QueryParser, DefaultQueryParser,
        QueryPlanner, DefaultQueryPlanner,
        QueryExecutor, DefaultQueryExecutor,
    },
    protocol::{ProtocolAdapter, ProtocolType},
    connectors::{ConnectorRegistry, Connector},
    utils::{
        config::{EngineConfig, ProtocolConfig, ProtocolType as ConfigProtocolType},
        error::{NirvResult, NirvError},
        types::QueryResult,
    },
};

/// Main NIRV Engine that coordinates all components
pub struct Engine {
    /// Engine configuration
    config: EngineConfig,
    /// Query parser for SQL parsing
    query_parser: Arc<dyn QueryParser>,
    /// Query planner for execution planning
    query_planner: Arc<dyn QueryPlanner>,
    /// Query executor for plan execution
    query_executor: Arc<RwLock<dyn QueryExecutor>>,
    /// Dispatcher for routing queries to connectors
    dispatcher: Arc<RwLock<dyn Dispatcher>>,
    /// Protocol adapters for client connections
    protocol_adapters: HashMap<ProtocolType, Arc<dyn ProtocolAdapter>>,
    /// Running protocol server tasks
    server_tasks: Vec<JoinHandle<()>>,
    /// Shutdown signal
    shutdown_signal: Option<tokio::sync::broadcast::Sender<()>>,
}

impl Engine {
    /// Create a new engine with the given configuration
    pub fn new(config: EngineConfig) -> Self {
        let query_parser = Arc::new(DefaultQueryParser::new().expect("Failed to create query parser"));
        let query_planner = Arc::new(DefaultQueryPlanner::new());
        let query_executor = Arc::new(RwLock::new(DefaultQueryExecutor::new()));
        let dispatcher = Arc::new(RwLock::new(DefaultDispatcher::new()));
        
        Self {
            config,
            query_parser,
            query_planner,
            query_executor,
            dispatcher,
            protocol_adapters: HashMap::new(),
            server_tasks: Vec::new(),
            shutdown_signal: None,
        }
    }
    
    /// Create an engine with custom components
    pub fn with_components(
        config: EngineConfig,
        query_parser: Arc<dyn QueryParser>,
        query_planner: Arc<dyn QueryPlanner>,
        query_executor: Arc<RwLock<dyn QueryExecutor>>,
        dispatcher: Arc<RwLock<dyn Dispatcher>>,
    ) -> Self {
        Self {
            config,
            query_parser,
            query_planner,
            query_executor,
            dispatcher,
            protocol_adapters: HashMap::new(),
            server_tasks: Vec::new(),
            shutdown_signal: None,
        }
    }
    
    /// Initialize the engine and start all services
    pub async fn initialize(&mut self) -> NirvResult<()> {
        // Initialize connector registry
        let connector_registry = self.initialize_connectors().await?;
        
        // Set connector registry in query executor
        {
            let mut executor = self.query_executor.write().await;
            executor.set_connector_registry(connector_registry);
        }
        
        // Initialize protocol adapters
        self.initialize_protocol_adapters().await?;
        
        // Start protocol servers (only if we have protocol adapters configured)
        if !self.config.protocol_adapters.is_empty() {
            self.start_protocol_servers().await?;
        }
        
        Ok(())
    }
    
    /// Initialize connectors from configuration
    async fn initialize_connectors(&mut self) -> NirvResult<ConnectorRegistry> {
        let mut registry = ConnectorRegistry::new();
        
        for (name, connector_config) in &self.config.connectors {
            // For MVP, we'll create mock connectors
            // In future tasks, we'll create actual connector implementations
            let connector = self.create_connector(connector_config)?;
            registry.register(name.clone(), connector)?;
        }
        
        // Also register any connectors that were added manually
        // This ensures the registry is properly initialized even with empty config
        
        Ok(registry)
    }
    
    /// Create a connector based on configuration
    fn create_connector(&self, _config: &crate::utils::config::ConnectorConfig) -> NirvResult<Box<dyn Connector>> {
        // For MVP, return a mock connector
        // This will be expanded in future tasks to create actual connectors
        use crate::connectors::MockConnector;
        Ok(Box::new(MockConnector::new()))
    }
    
    /// Initialize protocol adapters
    async fn initialize_protocol_adapters(&mut self) -> NirvResult<()> {
        for protocol_config in &self.config.protocol_adapters {
            let adapter = self.create_protocol_adapter(protocol_config)?;
            let protocol_type = match protocol_config.protocol_type {
                ConfigProtocolType::PostgreSQL => ProtocolType::PostgreSQL,
                ConfigProtocolType::MySQL => ProtocolType::MySQL,
                ConfigProtocolType::SQLite => ProtocolType::SQLite,
            };
            self.protocol_adapters.insert(protocol_type, adapter);
        }
        Ok(())
    }
    
    /// Create a protocol adapter based on configuration
    fn create_protocol_adapter(&self, config: &ProtocolConfig) -> NirvResult<Arc<dyn ProtocolAdapter>> {
        match config.protocol_type {
            ConfigProtocolType::PostgreSQL => {
                use crate::protocol::PostgreSQLProtocolAdapter;
                Ok(Arc::new(PostgreSQLProtocolAdapter::new()))
            }
            ConfigProtocolType::MySQL => {
                use crate::protocol::MySQLProtocolAdapter;
                Ok(Arc::new(MySQLProtocolAdapter::new()))
            }
            ConfigProtocolType::SQLite => {
                use crate::protocol::SQLiteProtocolAdapter;
                Ok(Arc::new(SQLiteProtocolAdapter::new()))
            }
        }
    }
    
    /// Start protocol servers for client connections
    async fn start_protocol_servers(&mut self) -> NirvResult<()> {
        let (shutdown_tx, _) = tokio::sync::broadcast::channel(1);
        self.shutdown_signal = Some(shutdown_tx.clone());
        
        for protocol_config in &self.config.protocol_adapters {
            let protocol_type = match protocol_config.protocol_type {
                ConfigProtocolType::PostgreSQL => ProtocolType::PostgreSQL,
                ConfigProtocolType::MySQL => ProtocolType::MySQL,
                ConfigProtocolType::SQLite => ProtocolType::SQLite,
            };
            
            let adapter = self.protocol_adapters
                .get(&protocol_type)
                .ok_or_else(|| NirvError::Internal(
                    format!("Protocol adapter not found: {:?}", protocol_config.protocol_type)
                ))?
                .clone();
            
            let bind_address = format!("{}:{}", protocol_config.bind_address, protocol_config.port);
            let listener = TcpListener::bind(&bind_address).await
                .map_err(|e| NirvError::Internal(
                    format!("Failed to bind to {}: {}", bind_address, e)
                ))?;
            
            let engine_ref = EngineRef {
                query_parser: self.query_parser.clone(),
                query_planner: self.query_planner.clone(),
                query_executor: self.query_executor.clone(),
                dispatcher: self.dispatcher.clone(),
            };
            
            let mut shutdown_rx = shutdown_tx.subscribe();
            let task = tokio::spawn(async move {
                loop {
                    tokio::select! {
                        result = listener.accept() => {
                            match result {
                                Ok((stream, _addr)) => {
                                    let adapter_clone = adapter.clone();
                                    let engine_clone = engine_ref.clone();
                                    tokio::spawn(async move {
                                        if let Err(e) = Self::handle_client_connection(
                                            adapter_clone,
                                            engine_clone,
                                            stream
                                        ).await {
                                            eprintln!("Client connection error: {}", e);
                                        }
                                    });
                                }
                                Err(e) => {
                                    eprintln!("Failed to accept connection: {}", e);
                                }
                            }
                        }
                        _ = shutdown_rx.recv() => {
                            break;
                        }
                    }
                }
            });
            
            self.server_tasks.push(task);
        }
        
        Ok(())
    }
    
    /// Handle a client connection through a protocol adapter
    async fn handle_client_connection(
        adapter: Arc<dyn ProtocolAdapter>,
        _engine: EngineRef,
        stream: tokio::net::TcpStream,
    ) -> NirvResult<()> {
        // Accept the connection
        let mut connection = adapter.accept_connection(stream).await?;
        
        // For MVP, we'll skip authentication
        // In future tasks, we'll implement proper authentication
        
        // Handle queries in a loop
        loop {
            // For MVP, we'll implement a simple query handling loop
            // In future tasks, we'll implement proper protocol message handling
            break;
        }
        
        // Terminate the connection
        adapter.terminate_connection(&mut connection).await?;
        
        Ok(())
    }
    
    /// Execute a query through the engine
    pub async fn execute_query(&self, query_string: &str) -> NirvResult<QueryResult> {
        // Parse the query
        let internal_query = self.query_parser.parse_sql(query_string).await?;
        
        // Route the query through the dispatcher
        let dispatcher = self.dispatcher.read().await;
        let connector_queries = dispatcher.route_query(&internal_query).await?;
        
        // Execute the distributed query
        dispatcher.execute_distributed_query(connector_queries).await
    }
    
    /// Register a connector with the dispatcher
    pub async fn register_connector(&self, object_type: &str, connector: Box<dyn Connector>) -> NirvResult<()> {
        let mut dispatcher = self.dispatcher.write().await;
        dispatcher.register_connector(object_type, connector).await
    }
    
    /// Initialize the engine for testing (without starting protocol servers)
    pub async fn initialize_for_testing(&mut self) -> NirvResult<()> {
        // Initialize connector registry
        let connector_registry = self.initialize_connectors().await?;
        
        // Set connector registry in query executor
        {
            let mut executor = self.query_executor.write().await;
            executor.set_connector_registry(connector_registry);
        }
        
        // Initialize protocol adapters but don't start servers
        self.initialize_protocol_adapters().await?;
        
        Ok(())
    }
    
    /// Get available data object types
    pub async fn list_available_types(&self) -> Vec<String> {
        let dispatcher = self.dispatcher.read().await;
        dispatcher.list_available_types()
    }
    
    /// Shutdown the engine gracefully
    pub async fn shutdown(&mut self) -> NirvResult<()> {
        // Send shutdown signal to all servers
        if let Some(shutdown_tx) = &self.shutdown_signal {
            let _ = shutdown_tx.send(());
        }
        
        // Wait for all server tasks to complete
        for task in self.server_tasks.drain(..) {
            let _ = task.await;
        }
        
        // Disconnect all connectors
        // This would be implemented when we have actual connector implementations
        
        Ok(())
    }
    
    /// Wait for shutdown signal (Ctrl+C)
    pub async fn wait_for_shutdown(&self) -> NirvResult<()> {
        signal::ctrl_c().await
            .map_err(|e| NirvError::Internal(format!("Failed to listen for shutdown signal: {}", e)))?;
        Ok(())
    }
}

/// Reference to engine components for use in async tasks
#[derive(Clone)]
struct EngineRef {
    query_parser: Arc<dyn QueryParser>,
    query_planner: Arc<dyn QueryPlanner>,
    query_executor: Arc<RwLock<dyn QueryExecutor>>,
    dispatcher: Arc<RwLock<dyn Dispatcher>>,
}

/// Builder for creating Engine instances
pub struct EngineBuilder {
    config: Option<EngineConfig>,
    query_parser: Option<Arc<dyn QueryParser>>,
    query_planner: Option<Arc<dyn QueryPlanner>>,
    query_executor: Option<Arc<RwLock<dyn QueryExecutor>>>,
    dispatcher: Option<Arc<RwLock<dyn Dispatcher>>>,
}

impl EngineBuilder {
    /// Create a new engine builder
    pub fn new() -> Self {
        Self {
            config: None,
            query_parser: None,
            query_planner: None,
            query_executor: None,
            dispatcher: None,
        }
    }
    
    /// Set the engine configuration
    pub fn with_config(mut self, config: EngineConfig) -> Self {
        self.config = Some(config);
        self
    }
    
    /// Set a custom query parser
    pub fn with_query_parser(mut self, parser: Arc<dyn QueryParser>) -> Self {
        self.query_parser = Some(parser);
        self
    }
    
    /// Set a custom query planner
    pub fn with_query_planner(mut self, planner: Arc<dyn QueryPlanner>) -> Self {
        self.query_planner = Some(planner);
        self
    }
    
    /// Set a custom query executor
    pub fn with_query_executor(mut self, executor: Arc<RwLock<dyn QueryExecutor>>) -> Self {
        self.query_executor = Some(executor);
        self
    }
    
    /// Set a custom dispatcher
    pub fn with_dispatcher(mut self, dispatcher: Arc<RwLock<dyn Dispatcher>>) -> Self {
        self.dispatcher = Some(dispatcher);
        self
    }
    
    /// Build the engine
    pub fn build(self) -> NirvResult<Engine> {
        let config = self.config.unwrap_or_else(EngineConfig::default);
        
        if let (Some(parser), Some(planner), Some(executor), Some(dispatcher)) = (
            self.query_parser,
            self.query_planner,
            self.query_executor,
            self.dispatcher,
        ) {
            Ok(Engine::with_components(config, parser, planner, executor, dispatcher))
        } else {
            Ok(Engine::new(config))
        }
    }
}

impl Default for EngineBuilder {
    fn default() -> Self {
        Self::new()
    }
}