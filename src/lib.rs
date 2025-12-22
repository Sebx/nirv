//! # NIRV Engine
//!
//! A universal data virtualization and compute orchestration engine that provides
//! a unified interface to query and manipulate data across multiple sources including
//! databases, APIs, and file systems.
//!
//! ## Features
//!
//! - **Multi-Source Connectors**: SQL Server, PostgreSQL, REST APIs, File Systems
//! - **Protocol Adapters**: TDS, PostgreSQL Wire Protocol, HTTP/REST
//! - **Query Engine**: Parsing, planning, and distributed execution
//! - **Schema Introspection**: Automatic schema discovery and metadata management
//! - **Connection Pooling**: Efficient resource management
//! - **Async/Await**: Full async support with Tokio runtime
//!
//! ## Quick Start
//!
//! ```rust
//! use nirv_engine::connectors::{SqlServerConnector, Connector, ConnectorInitConfig};
//! use nirv_engine::protocol::{SqlServerProtocol, ProtocolAdapter};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // Create and configure a SQL Server connector
//!     let mut connector = SqlServerConnector::new();
//!     
//!     let config = ConnectorInitConfig::new()
//!         .with_param("server", "localhost")
//!         .with_param("database", "mydb")
//!         .with_param("username", "user")
//!         .with_param("password", "password");
//!     
//!     // Connect to the database
//!     connector.connect(config).await?;
//!     
//!     println!("Connected! Connector type: {:?}", connector.get_connector_type());
//!     Ok(())
//! }
//! ```
//!
//! ## Architecture
//!
//! NIRV Engine follows a modular architecture:
//!
//! - [`engine`] - Query parsing, planning, and execution
//! - [`connectors`] - Data source abstractions and implementations
//! - [`protocol`] - Wire-level communication protocol adapters
//! - [`utils`] - Common types, error handling, and utilities
//! - [`cli`] - Command-line interface components
//!
//! ## SQL Server Support
//!
//! NIRV Engine provides comprehensive SQL Server support through:
//!
//! ### TDS Protocol Implementation
//! - TDS 7.4 protocol support
//! - Authentication with SQL Server credentials
//! - Query execution with proper result formatting
//! - Error handling with SQL Server error codes
//!
//! ### Connector Features
//! - Connection management with connection pooling
//! - Query building with SQL Server-specific syntax
//! - Schema introspection via INFORMATION_SCHEMA
//! - Transaction support for ACID compliance
//!
//! ```rust
//! use nirv_engine::protocol::{SqlServerProtocol, Connection, Credentials, ProtocolType};
//! use tokio::net::TcpStream;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // Create protocol adapter
//! let protocol = SqlServerProtocol::new();
//!
//! // Simulate authentication
//! let credentials = Credentials::new("sa".to_string(), "master".to_string())
//!     .with_password("password".to_string());
//!
//! # let stream = TcpStream::connect("127.0.0.1:1433").await?;
//! let mut connection = Connection::new(stream, ProtocolType::SqlServer);
//! protocol.authenticate(&mut connection, credentials).await?;
//! # Ok(())
//! # }
//! ```
//!
//! ## REST API Integration
//!
//! ```rust
//! use nirv_engine::connectors::{RestConnector, AuthConfig, RateLimitConfig};
//! use std::time::Duration;
//!
//! let connector = RestConnector::new()
//!     .with_auth(AuthConfig::ApiKey {
//!         header: "X-API-Key".to_string(),
//!         key: "your-api-key".to_string(),
//!     })
//!     .with_cache_ttl(Duration::from_secs(300))
//!     .with_rate_limit(RateLimitConfig {
//!         requests_per_second: 10.0,
//!         burst_size: 20,
//!     });
//! ```

pub mod engine;
pub mod connectors;
pub mod protocol;
pub mod cli;
pub mod utils;

pub use engine::*;
pub use connectors::*;
pub use protocol::*;
pub use cli::*;
pub use utils::*;