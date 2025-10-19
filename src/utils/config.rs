use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use crate::utils::types::ConnectorType;

/// Main engine configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct EngineConfig {
    pub protocol_adapters: Vec<ProtocolConfig>,
    pub connectors: HashMap<String, ConnectorConfig>,
    pub dispatcher: DispatcherConfig,
    pub security: SecurityConfig,
}

/// Protocol adapter configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ProtocolConfig {
    pub protocol_type: ProtocolType,
    pub bind_address: String,
    pub port: u16,
    pub tls_config: Option<TlsConfig>,
    pub max_connections: Option<u32>,
    pub connection_timeout: Option<u64>, // seconds
}

/// Supported protocol types
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub enum ProtocolType {
    PostgreSQL,
    MySQL,
    SQLite,
}

/// TLS configuration for protocols
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TlsConfig {
    pub cert_file: String,
    pub key_file: String,
    pub ca_file: Option<String>,
    pub require_client_cert: bool,
}

/// Connector configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ConnectorConfig {
    pub connector_type: ConnectorType,
    pub connection_string: Option<String>,
    pub parameters: HashMap<String, String>,
    pub pool_config: Option<PoolConfig>,
    pub timeout_config: Option<TimeoutConfig>,
}

/// Connection pool configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PoolConfig {
    pub min_connections: u32,
    pub max_connections: u32,
    pub connection_timeout: u64,    // seconds
    pub idle_timeout: u64,          // seconds
    pub max_lifetime: Option<u64>,  // seconds
}

/// Timeout configuration for connectors
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TimeoutConfig {
    pub connect_timeout: u64,       // seconds
    pub query_timeout: u64,         // seconds
    pub transaction_timeout: u64,   // seconds
}

/// Dispatcher configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DispatcherConfig {
    pub max_concurrent_queries: u32,
    pub query_cache_size: Option<u64>,  // MB
    pub enable_cross_connector_joins: bool,
    pub default_timeout: u64,           // seconds
}

/// Security configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SecurityConfig {
    pub authentication: AuthenticationConfig,
    pub authorization: AuthorizationConfig,
    pub audit_logging: AuditConfig,
}

/// Authentication configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AuthenticationConfig {
    pub enabled: bool,
    pub auth_method: AuthMethod,
    pub user_database: Option<String>,
    pub ldap_config: Option<LdapConfig>,
}

/// Authentication methods
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub enum AuthMethod {
    None,
    Password,
    Certificate,
    LDAP,
    OAuth2,
}

/// LDAP configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LdapConfig {
    pub server_url: String,
    pub bind_dn: String,
    pub bind_password: String,
    pub user_search_base: String,
    pub user_search_filter: String,
}

/// Authorization configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AuthorizationConfig {
    pub enabled: bool,
    pub default_permissions: Vec<Permission>,
    pub role_mappings: HashMap<String, Vec<Permission>>,
}

/// Permission types
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub enum Permission {
    Read,
    Write,
    Admin,
    Connect,
}

/// Audit logging configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AuditConfig {
    pub enabled: bool,
    pub log_file: Option<String>,
    pub log_queries: bool,
    pub log_connections: bool,
    pub log_errors: bool,
}

impl Default for EngineConfig {
    fn default() -> Self {
        Self {
            protocol_adapters: vec![
                ProtocolConfig {
                    protocol_type: ProtocolType::PostgreSQL,
                    bind_address: "127.0.0.1".to_string(),
                    port: 5432,
                    tls_config: None,
                    max_connections: Some(100),
                    connection_timeout: Some(30),
                }
            ],
            connectors: HashMap::new(),
            dispatcher: DispatcherConfig::default(),
            security: SecurityConfig::default(),
        }
    }
}

impl Default for DispatcherConfig {
    fn default() -> Self {
        Self {
            max_concurrent_queries: 100,
            query_cache_size: Some(256), // 256 MB
            enable_cross_connector_joins: false,
            default_timeout: 300, // 5 minutes
        }
    }
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            authentication: AuthenticationConfig {
                enabled: false,
                auth_method: AuthMethod::None,
                user_database: None,
                ldap_config: None,
            },
            authorization: AuthorizationConfig {
                enabled: false,
                default_permissions: vec![Permission::Read],
                role_mappings: HashMap::new(),
            },
            audit_logging: AuditConfig {
                enabled: true,
                log_file: None,
                log_queries: true,
                log_connections: true,
                log_errors: true,
            },
        }
    }
}

impl Default for PoolConfig {
    fn default() -> Self {
        Self {
            min_connections: 1,
            max_connections: 10,
            connection_timeout: 30,
            idle_timeout: 600,
            max_lifetime: Some(3600),
        }
    }
}

impl Default for TimeoutConfig {
    fn default() -> Self {
        Self {
            connect_timeout: 30,
            query_timeout: 300,
            transaction_timeout: 600,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_engine_config_default() {
        let config = EngineConfig::default();
        
        assert_eq!(config.protocol_adapters.len(), 1);
        assert_eq!(config.protocol_adapters[0].protocol_type, ProtocolType::PostgreSQL);
        assert_eq!(config.protocol_adapters[0].bind_address, "127.0.0.1");
        assert_eq!(config.protocol_adapters[0].port, 5432);
        assert!(config.connectors.is_empty());
    }

    #[test]
    fn test_protocol_config_creation() {
        let config = ProtocolConfig {
            protocol_type: ProtocolType::MySQL,
            bind_address: "0.0.0.0".to_string(),
            port: 3306,
            tls_config: None,
            max_connections: Some(50),
            connection_timeout: Some(60),
        };
        
        assert_eq!(config.protocol_type, ProtocolType::MySQL);
        assert_eq!(config.bind_address, "0.0.0.0");
        assert_eq!(config.port, 3306);
        assert_eq!(config.max_connections, Some(50));
    }

    #[test]
    fn test_connector_config_creation() {
        let mut parameters = HashMap::new();
        parameters.insert("host".to_string(), "localhost".to_string());
        parameters.insert("database".to_string(), "test_db".to_string());
        
        let config = ConnectorConfig {
            connector_type: ConnectorType::PostgreSQL,
            connection_string: Some("postgresql://localhost/test".to_string()),
            parameters,
            pool_config: Some(PoolConfig::default()),
            timeout_config: Some(TimeoutConfig::default()),
        };
        
        assert_eq!(config.connector_type, ConnectorType::PostgreSQL);
        assert!(config.connection_string.is_some());
        assert_eq!(config.parameters.get("host"), Some(&"localhost".to_string()));
        assert!(config.pool_config.is_some());
        assert!(config.timeout_config.is_some());
    }

    #[test]
    fn test_dispatcher_config_default() {
        let config = DispatcherConfig::default();
        
        assert_eq!(config.max_concurrent_queries, 100);
        assert_eq!(config.query_cache_size, Some(256));
        assert!(!config.enable_cross_connector_joins);
        assert_eq!(config.default_timeout, 300);
    }

    #[test]
    fn test_security_config_default() {
        let config = SecurityConfig::default();
        
        assert!(!config.authentication.enabled);
        assert_eq!(config.authentication.auth_method, AuthMethod::None);
        assert!(!config.authorization.enabled);
        assert_eq!(config.authorization.default_permissions, vec![Permission::Read]);
        assert!(config.audit_logging.enabled);
        assert!(config.audit_logging.log_queries);
    }

    #[test]
    fn test_pool_config_default() {
        let config = PoolConfig::default();
        
        assert_eq!(config.min_connections, 1);
        assert_eq!(config.max_connections, 10);
        assert_eq!(config.connection_timeout, 30);
        assert_eq!(config.idle_timeout, 600);
        assert_eq!(config.max_lifetime, Some(3600));
    }

    #[test]
    fn test_timeout_config_default() {
        let config = TimeoutConfig::default();
        
        assert_eq!(config.connect_timeout, 30);
        assert_eq!(config.query_timeout, 300);
        assert_eq!(config.transaction_timeout, 600);
    }

    #[test]
    fn test_auth_method_variants() {
        let methods = vec![
            AuthMethod::None,
            AuthMethod::Password,
            AuthMethod::Certificate,
            AuthMethod::LDAP,
            AuthMethod::OAuth2,
        ];
        
        assert_eq!(methods.len(), 5);
        assert_eq!(methods[0], AuthMethod::None);
        assert_eq!(methods[1], AuthMethod::Password);
    }

    #[test]
    fn test_permission_variants() {
        let permissions = vec![
            Permission::Read,
            Permission::Write,
            Permission::Admin,
            Permission::Connect,
        ];
        
        assert_eq!(permissions.len(), 4);
        assert_eq!(permissions[0], Permission::Read);
        assert_eq!(permissions[1], Permission::Write);
    }
}