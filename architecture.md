# NIRV Engine Architecture

This document provides a comprehensive overview of NIRV Engine's architecture, design decisions, and component relationships.

## Table of Contents

- [Design Philosophy](#design-philosophy)
- [System Architecture](#system-architecture)
- [Core Components](#core-components)
- [Data Flow](#data-flow)
- [Protocol Layer](#protocol-layer)
- [Connector Architecture](#connector-architecture)
- [Query Processing Pipeline](#query-processing-pipeline)
- [Configuration System](#configuration-system)
- [Error Handling Strategy](#error-handling-strategy)
- [Testing Architecture](#testing-architecture)
- [Performance Considerations](#performance-considerations)
- [Security Model](#security-model)
- [Extensibility Points](#extensibility-points)

## Design Philosophy

NIRV Engine is built on several core design principles that guide architectural decisions:

### 1. Protocol Compatibility First
The engine acts as a drop-in replacement for existing database connections by implementing standard wire protocols (PostgreSQL, MySQL, SQLite). This ensures zero-code-change adoption for existing applications.

### 2. Dispatcher-Centric Architecture
A central dispatcher component manages query routing and data source resolution, enabling clean separation between query processing and data access logic.

### 3. Async-First Design
All I/O operations use Rust's async/await patterns with the Tokio runtime, enabling high-performance concurrent query processing without blocking operations.

### 4. Trait-Based Extensibility
Clean abstractions through Rust traits allow adding new connectors, protocols, and query processors without modifying core engine logic.

### 5. Test-Driven Implementation
Comprehensive test coverage ensures reliability and enables confident refactoring as the system evolves.

## System Architecture

### High-Level Architecture Diagram

```
┌─────────────────┐    ┌──────────────────┐    ┌─────────────────┐
│   DB Clients    │    │   CLI Interface  │    │  Management UI  │
│ (JDBC/Native)   │    │                  │    │   (Future)      │
└─────────┬───────┘    └─────────┬────────┘    └─────────────────┘
          │                      │
          ▼                      ▼
┌─────────────────────────────────────────────────────────────────┐
│                    Protocol Layer                               │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐            │
│  │ PostgreSQL  │  │   MySQL     │  │   SQLite    │            │
│  │  Adapter    │  │  Adapter    │  │  Adapter    │            │
│  └─────────────┘  └─────────────┘  └─────────────┘            │
└─────────────────────┬───────────────────────────────────────────┘
                      │
                      ▼
┌─────────────────────────────────────────────────────────────────┐
│                     Core Engine                                 │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐            │
│  │    Query    │  │    Query    │  │    Query    │            │
│  │   Parser    │  │   Planner   │  │  Executor   │            │
│  └─────────────┘  └─────────────┘  └─────────────┘            │
│                           │                                     │
│                           ▼                                     │
│                  ┌─────────────┐                               │
│                  │ Dispatcher  │                               │
│                  └─────────────┘                               │
└─────────────────────┬───────────────────────────────────────────┘
                      │
                      ▼
┌─────────────────────────────────────────────────────────────────┐
│                  Connector Layer                                │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐            │
│  │    Mock     │  │ PostgreSQL  │  │    File     │            │
│  │ Connector   │  │ Connector   │  │ Connector   │            │
│  └─────────────┘  └─────────────┘  └─────────────┘            │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐            │
│  │    REST     │  │    LLM      │  │   Custom    │            │
│  │ Connector   │  │ Connector   │  │ Connector   │            │
│  └─────────────┘  └─────────────┘  └─────────────┘            │
└─────────────────────────────────────────────────────────────────┘
```

### Component Interaction Flow

1. **Client Connection**: Database clients connect using standard protocols
2. **Protocol Translation**: Protocol adapters translate wire protocol messages to internal representations
3. **Query Processing**: Core engine parses, plans, and executes queries
4. **Dispatch Routing**: Dispatcher routes queries to appropriate connectors based on data source types
5. **Data Access**: Connectors execute queries against their respective backends
6. **Result Aggregation**: Results are collected, formatted, and returned through the protocol layer

## Core Components

### 1. Protocol Layer

The protocol layer maintains compatibility with existing database clients by implementing standard wire protocols.

#### PostgreSQL Protocol Adapter
- Implements PostgreSQL wire protocol v3.0
- Handles connection establishment, authentication, and query/response cycles
- Supports prepared statements and extended query protocol
- Manages connection state and transaction boundaries

#### MySQL Protocol Adapter
- Implements MySQL/MariaDB client-server protocol
- Handles MySQL-specific authentication mechanisms
- Supports both text and binary result protocols
- Manages MySQL-specific connection attributes

#### SQLite Protocol Adapter
- Provides SQLite-compatible interface for embedded scenarios
- Simulates file-based database connections
- Supports SQLite-specific SQL dialect and functions

### 2. Core Engine Components

#### Query Parser (`src/engine/query_parser.rs`)
**Purpose**: Converts SQL queries from various dialects into internal representation.

**Key Features**:
- Multi-dialect SQL parsing using `sqlparser-rs`
- Source function extraction (`SELECT * FROM source('type.identifier')`)
- Query validation and comprehensive error reporting
- AST generation optimized for query planning

**Design Rationale**: 
- Separates parsing concerns from execution logic
- Enables support for multiple SQL dialects
- Provides foundation for query optimization

#### Query Planner (`src/engine/query_planner.rs`)
**Purpose**: Analyzes parsed queries and creates optimal execution plans.

**Key Features**:
- Data source identification and validation
- Join optimization across heterogeneous sources
- Predicate pushdown to appropriate connectors
- Execution cost estimation and plan selection

#### Query Executor (`src/engine/query_executor.rs`)
**Purpose**: Executes planned queries using registered connectors.

**Key Features**:
- Concurrent execution of independent operations
- Result aggregation and formatting
- Error handling and rollback coordination
- Connection pooling and resource management

#### Dispatcher (`src/engine/dispatcher.rs`)
**Purpose**: Central routing component managing data object type resolution and connector selection.

**Key Features**:
- Data object type registry and lookup
- Connector lifecycle management
- Load balancing across connector instances
- Cross-connector query coordination

**Interface Definition**:
```rust
#[async_trait]
pub trait Dispatcher {
    async fn register_connector(&mut self, object_type: &str, connector: Box<dyn Connector>) -> Result<()>;
    async fn route_query(&self, query: &InternalQuery) -> Result<Vec<ConnectorQuery>>;
    async fn execute_distributed_query(&self, queries: Vec<ConnectorQuery>) -> Result<QueryResult>;
    fn list_available_types(&self) -> Vec<String>;
    fn is_type_registered(&self, object_type: &str) -> bool;
    fn get_connector(&self, object_type: &str) -> Option<&dyn Connector>;
}
```

### 3. Connector Layer

The connector layer provides a unified interface for accessing heterogeneous data sources.

#### Base Connector Trait
```rust
#[async_trait]
pub trait Connector: Send + Sync {
    async fn connect(&mut self, config: ConnectorInitConfig) -> Result<()>;
    async fn execute_query(&self, query: ConnectorQuery) -> Result<QueryResult>;
    async fn get_schema(&self, object_name: &str) -> Result<Schema>;
    async fn disconnect(&mut self) -> Result<()>;
    fn get_connector_type(&self) -> ConnectorType;
    fn get_capabilities(&self) -> ConnectorCapabilities;
    fn is_connected(&self) -> bool;
}
```

#### Connector Implementations

1. **Mock Connector**: In-memory test data with deterministic responses for testing
2. **PostgreSQL Connector**: Native PostgreSQL database access with connection pooling
3. **File Connector**: CSV, JSON, Parquet file access with SQL interface
4. **REST Connector**: HTTP API access with query parameter mapping and authentication
5. **LLM Connector**: Large Language Model integration for AI-powered queries

## Data Flow

### Query Execution Flow

```mermaid
sequenceDiagram
    participant Client
    participant Protocol
    participant Parser
    participant Planner
    participant Dispatcher
    participant Connector
    participant Backend

    Client->>Protocol: SQL Query
    Protocol->>Parser: Parse Query
    Parser->>Planner: Internal Query
    Planner->>Dispatcher: Execution Plan
    Dispatcher->>Connector: Connector Query
    Connector->>Backend: Native Query
    Backend-->>Connector: Raw Results
    Connector-->>Dispatcher: Query Result
    Dispatcher-->>Planner: Aggregated Result
    Planner-->>Protocol: Formatted Result
    Protocol-->>Client: Protocol Response
```

### Data Source Registration Flow

1. **Configuration Loading**: Engine loads connector configurations from TOML files
2. **Connector Instantiation**: Connectors are created with their specific configurations
3. **Connection Establishment**: Connectors establish connections to their backends
4. **Registration**: Dispatcher registers connectors with their data object types
5. **Capability Discovery**: Dispatcher queries connector capabilities for optimization

## Protocol Layer

### Protocol Adapter Architecture

Each protocol adapter implements the `ProtocolAdapter` trait:

```rust
#[async_trait]
pub trait ProtocolAdapter {
    async fn accept_connection(&self, stream: TcpStream) -> Result<Connection>;
    async fn authenticate(&self, conn: &mut Connection, credentials: Credentials) -> Result<()>;
    async fn handle_query(&self, conn: &Connection, query: ProtocolQuery) -> Result<ProtocolResponse>;
    fn get_protocol_type(&self) -> ProtocolType;
}
```

### Protocol-Specific Considerations

#### PostgreSQL Protocol
- Supports both simple and extended query protocols
- Handles prepared statements and parameter binding
- Manages transaction state and savepoints
- Implements PostgreSQL-specific error codes and messages

#### MySQL Protocol
- Handles MySQL's multi-statement capabilities
- Supports MySQL-specific data types and functions
- Manages MySQL's connection attributes and capabilities
- Implements MySQL's authentication plugin system

#### SQLite Protocol
- Simulates file-based database operations
- Supports SQLite's pragma statements
- Handles SQLite-specific SQL extensions
- Manages virtual table interfaces

## Connector Architecture

### Connector Lifecycle

1. **Initialization**: Connector is created with configuration parameters
2. **Connection**: Establishes connection to backend system
3. **Registration**: Registers with dispatcher for specific data object types
4. **Query Processing**: Handles queries routed by dispatcher
5. **Cleanup**: Gracefully disconnects and releases resources

### Connector Capabilities

Each connector exposes its capabilities through the `ConnectorCapabilities` struct:

```rust
pub struct ConnectorCapabilities {
    pub supports_joins: bool,
    pub supports_transactions: bool,
    pub supports_prepared_statements: bool,
    pub max_concurrent_queries: Option<u32>,
    pub supported_operations: Vec<QueryOperation>,
}
```

### Error Handling in Connectors

Connectors implement comprehensive error handling:
- Connection failures with automatic retry logic
- Query timeout handling with configurable limits
- Resource exhaustion detection and recovery
- Backend-specific error translation to NIRV error types

## Query Processing Pipeline

### 1. Parsing Phase
- SQL dialect detection and normalization
- Syntax validation and error reporting
- AST generation with source function extraction
- Query type classification (SELECT, INSERT, UPDATE, DELETE)

### 2. Planning Phase
- Data source identification and validation
- Join strategy selection for cross-connector queries
- Predicate pushdown optimization
- Execution cost estimation

### 3. Execution Phase
- Connector query generation
- Parallel execution coordination
- Result streaming and aggregation
- Error handling and rollback management

### 4. Result Processing
- Data type normalization across connectors
- Result formatting for protocol compatibility
- Metadata generation (column names, types, constraints)
- Performance metrics collection

## Configuration System

### Configuration File Structure

NIRV Engine uses TOML configuration files with the following structure:

```toml
# Protocol adapters configuration
[[protocol_adapters]]
protocol_type = "PostgreSQL"
bind_address = "127.0.0.1"
port = 5432
max_connections = 100
connection_timeout = 30

# Connector configurations
[connectors.postgres_main]
connector_type = "PostgreSQL"
connection_string = "postgresql://user:pass@localhost/mydb"

[connectors.postgres_main.pool_config]
min_connections = 2
max_connections = 20
connection_timeout = 30

# Dispatcher configuration
[dispatcher]
max_concurrent_queries = 100
enable_cross_connector_joins = true
default_timeout = 300

# Security configuration
[security.authentication]
enabled = false
auth_method = "None"
```

### Configuration Loading

1. **Default Configuration**: Engine starts with sensible defaults
2. **File Loading**: Loads configuration from specified TOML file
3. **Environment Override**: Environment variables can override file settings
4. **Validation**: Configuration is validated before engine startup
5. **Hot Reload**: Configuration changes can be applied without restart (future feature)

## Error Handling Strategy

### Error Hierarchy

```rust
#[derive(Debug, thiserror::Error)]
pub enum NirvError {
    #[error("Protocol error: {0}")]
    Protocol(#[from] ProtocolError),
    
    #[error("Query parsing error: {0}")]
    QueryParsing(#[from] QueryParsingError),
    
    #[error("Connector error: {0}")]
    Connector(#[from] ConnectorError),
    
    #[error("Dispatcher error: {0}")]
    Dispatcher(#[from] DispatcherError),
    
    #[error("Configuration error: {0}")]
    Configuration(String),
    
    #[error("Internal error: {0}")]
    Internal(String),
}
```

### Error Handling Principles

1. **Graceful Degradation**: Failed connectors don't affect other operations
2. **Detailed Context**: Errors include source information and suggested fixes
3. **Protocol Compliance**: Database protocol errors follow standard error formats
4. **Structured Logging**: All errors are logged with appropriate context
5. **Recovery Mechanisms**: Automatic retry for transient failures

## Testing Architecture

### Test Strategy

1. **Unit Tests**: Individual component testing with mocked dependencies
2. **Integration Tests**: End-to-end query execution with real connectors
3. **Protocol Tests**: Database client compatibility verification
4. **Performance Tests**: Concurrent query handling and throughput measurement
5. **Chaos Tests**: Failure scenario testing and recovery validation

### Test Data Management

The `MockConnector` provides deterministic test data:

```rust
pub struct MockConnector {
    test_data: HashMap<String, Vec<Row>>,
    schemas: HashMap<String, Schema>,
    connection_state: ConnectionState,
}
```

### Key Test Scenarios

1. **Protocol Compatibility**: 
   - PostgreSQL client connection and query execution
   - MySQL Workbench compatibility
   - JDBC driver integration

2. **Query Processing**:
   - Multi-source joins across different connector types
   - Complex WHERE clause pushdown optimization
   - Aggregation across heterogeneous data sources

3. **Connector Reliability**:
   - Connection failure recovery
   - Partial result handling
   - Timeout and retry mechanisms

## Performance Considerations

### Optimization Strategies

1. **Connection Pooling**: Efficient resource management for backend connections
2. **Query Caching**: Result caching for frequently accessed data
3. **Parallel Execution**: Concurrent processing of independent operations
4. **Memory Management**: Streaming results for large datasets
5. **Predicate Pushdown**: Moving filters closer to data sources

### Performance Monitoring

- Query execution time tracking
- Connection pool utilization metrics
- Memory usage monitoring
- Throughput and latency measurements
- Error rate tracking

## Security Model

### Authentication

- Support for database-native authentication methods
- Integration with external authentication systems (LDAP, OAuth2)
- Certificate-based authentication for enhanced security
- User database management for local authentication

### Authorization

- Role-based access control for data sources
- Fine-grained permissions (Read, Write, Admin, Connect)
- Data source-level access restrictions
- Query-level authorization checks

### Audit Logging

- Comprehensive query and access logging
- Connection attempt tracking
- Error and security event logging
- Configurable log levels and destinations

### Data Security

- TLS encryption for all protocol connections
- Secure credential storage and management
- SQL injection prevention through parameterized queries
- Input validation and sanitization

## Extensibility Points

### Adding New Connectors

1. Implement the `Connector` trait
2. Define connector-specific configuration
3. Register with the dispatcher
4. Add connector type to the registry

### Adding New Protocols

1. Implement the `ProtocolAdapter` trait
2. Handle protocol-specific message formats
3. Integrate with the core engine
4. Add protocol configuration options

### Custom Query Processors

1. Implement query processing interfaces
2. Register with the query planner
3. Handle custom SQL extensions
4. Integrate with result formatting

### Plugin Architecture (Future)

- Dynamic loading of connector plugins
- Runtime registration of new data source types
- Hot-swappable query processors
- External authentication providers