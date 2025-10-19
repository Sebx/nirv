// Protocol adapter implementations
pub mod protocol_trait;
pub mod postgres_protocol;
pub mod mysql_protocol;
pub mod sqlite_protocol;
pub mod sqlserver_protocol;

pub use protocol_trait::*;
pub use postgres_protocol::*;
pub use mysql_protocol::*;
pub use sqlite_protocol::*;
pub use sqlserver_protocol::*;

// Type aliases for convenience
pub type PostgreSQLProtocolAdapter = postgres_protocol::PostgresProtocol;
pub type SqlServerProtocolAdapter = sqlserver_protocol::SqlServerProtocol;