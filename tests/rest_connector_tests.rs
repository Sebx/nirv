use std::collections::HashMap;
use std::time::Duration;
use tokio_test;
use serde_json::json;
use reqwest::Method;

use nirv_engine::connectors::{
    RestConnector, EndpointMapping, AuthConfig, RateLimitConfig,
    Connector, ConnectorInitConfig
};
use nirv_engine::utils::types::{
    ConnectorQuery, ConnectorType, InternalQuery, QueryOperation,
    DataSource, Predicate, PredicateOperator, PredicateValue
};

/// Mock HTTP server for testing
struct MockServer {
    port: u16,
}

impl MockServer {
    async fn start() -> Self {
        // For now, we'll use a simple approach without an actual server
        // In a real implementation, you'd use something like wiremock
        Self { port: 8080 }
    }
    
    fn base_url(&self) -> String {
        format!("http://localhost:{}", self.port)
    }
}

#[tokio::test]
async fn test_rest_connector_creation() {
    let connector = RestConnector::new();
    
    assert_eq!(connector.get_connector_type(), ConnectorType::Rest);
    assert!(!connector.is_connected());
    assert!(!connector.supports_transactions());
}

#[tokio::test]
async fn test_rest_connector_with_auth_config() {
    let auth = AuthConfig::ApiKey {
        header: "X-API-Key".to_string(),
        key: "test-key-123".to_string(),
    };
    
    let connector = RestConnector::new().with_auth(auth);
    assert_eq!(connector.get_connector_type(), ConnectorType::Rest);
}

#[tokio::test]
async fn test_rest_connector_with_rate_limit() {
    let rate_config = RateLimitConfig {
        requests_per_second: 5.0,
        burst_size: 10,
    };
    
    let connector = RestConnector::new().with_rate_limit(rate_config);
    assert_eq!(connector.get_connector_type(), ConnectorType::Rest);
}

#[tokio::test]
async fn test_rest_connector_with_cache_ttl() {
    let connector = RestConnector::new()
        .with_cache_ttl(Duration::from_secs(600));
    
    assert_eq!(connector.get_connector_type(), ConnectorType::Rest);
}

#[tokio::test]
async fn test_connector_init_config_for_rest() {
    let mut config = ConnectorInitConfig::new()
        .with_param("base_url", "https://api.example.com")
        .with_param("auth_type", "api_key")
        .with_param("api_key", "test-key")
        .with_param("cache_ttl_seconds", "300")
        .with_timeout(60);
    
    assert_eq!(config.connection_params.get("base_url"), Some(&"https://api.example.com".to_string()));
    assert_eq!(config.connection_params.get("auth_type"), Some(&"api_key".to_string()));
    assert_eq!(config.timeout_seconds, Some(60));
}#[
tokio::test]
async fn test_endpoint_mapping_creation() {
    let mapping = EndpointMapping {
        path: "/api/users".to_string(),
        method: Method::GET,
        query_params: {
            let mut params = HashMap::new();
            params.insert("limit".to_string(), "100".to_string());
            params
        },
        response_path: Some("data".to_string()),
        id_field: Some("id".to_string()),
    };
    
    assert_eq!(mapping.path, "/api/users");
    assert_eq!(mapping.method, Method::GET);
    assert_eq!(mapping.query_params.get("limit"), Some(&"100".to_string()));
    assert_eq!(mapping.response_path, Some("data".to_string()));
    assert_eq!(mapping.id_field, Some("id".to_string()));
}

#[tokio::test]
async fn test_auth_config_variants() {
    // Test API Key auth
    let api_key_auth = AuthConfig::ApiKey {
        header: "Authorization".to_string(),
        key: "Bearer token123".to_string(),
    };
    
    match api_key_auth {
        AuthConfig::ApiKey { header, key } => {
            assert_eq!(header, "Authorization");
            assert_eq!(key, "Bearer token123");
        },
        _ => panic!("Expected API key auth"),
    }
    
    // Test Bearer auth
    let bearer_auth = AuthConfig::Bearer {
        token: "jwt-token-here".to_string(),
    };
    
    match bearer_auth {
        AuthConfig::Bearer { token } => {
            assert_eq!(token, "jwt-token-here");
        },
        _ => panic!("Expected Bearer auth"),
    }
    
    // Test Basic auth
    let basic_auth = AuthConfig::Basic {
        username: "user".to_string(),
        password: "pass".to_string(),
    };
    
    match basic_auth {
        AuthConfig::Basic { username, password } => {
            assert_eq!(username, "user");
            assert_eq!(password, "pass");
        },
        _ => panic!("Expected Basic auth"),
    }
    
    // Test None auth
    let no_auth = AuthConfig::None;
    match no_auth {
        AuthConfig::None => {},
        _ => panic!("Expected no auth"),
    }
}

#[tokio::test]
async fn test_rate_limit_config() {
    let config = RateLimitConfig {
        requests_per_second: 2.5,
        burst_size: 5,
    };
    
    assert_eq!(config.requests_per_second, 2.5);
    assert_eq!(config.burst_size, 5);
    
    // Test default
    let default_config = RateLimitConfig::default();
    assert_eq!(default_config.requests_per_second, 10.0);
    assert_eq!(default_config.burst_size, 10);
}#[tokio
::test]
async fn test_connector_capabilities() {
    let connector = RestConnector::new();
    let capabilities = connector.get_capabilities();
    
    assert!(!capabilities.supports_joins);
    assert!(capabilities.supports_aggregations);
    assert!(!capabilities.supports_subqueries);
    assert!(!capabilities.supports_transactions);
    assert!(capabilities.supports_schema_introspection);
    assert_eq!(capabilities.max_concurrent_queries, Some(5));
}

#[tokio::test]
async fn test_connector_query_creation() {
    let internal_query = InternalQuery::new(QueryOperation::Select);
    
    let connector_query = ConnectorQuery {
        connector_type: ConnectorType::Rest,
        query: internal_query,
        connection_params: HashMap::new(),
    };
    
    assert_eq!(connector_query.connector_type, ConnectorType::Rest);
    assert_eq!(connector_query.query.operation, QueryOperation::Select);
}

#[tokio::test]
async fn test_predicate_creation_for_rest_filtering() {
    // Test string predicate
    let string_predicate = Predicate {
        column: "name".to_string(),
        operator: PredicateOperator::Equal,
        value: PredicateValue::String("John".to_string()),
    };
    
    assert_eq!(string_predicate.column, "name");
    assert_eq!(string_predicate.operator, PredicateOperator::Equal);
    match &string_predicate.value {
        PredicateValue::String(s) => assert_eq!(s, "John"),
        _ => panic!("Expected string value"),
    }
    
    // Test integer predicate
    let int_predicate = Predicate {
        column: "age".to_string(),
        operator: PredicateOperator::GreaterThan,
        value: PredicateValue::Integer(25),
    };
    
    assert_eq!(int_predicate.column, "age");
    assert_eq!(int_predicate.operator, PredicateOperator::GreaterThan);
    
    // Test IN predicate
    let in_predicate = Predicate {
        column: "status".to_string(),
        operator: PredicateOperator::In,
        value: PredicateValue::List(vec![
            PredicateValue::String("active".to_string()),
            PredicateValue::String("pending".to_string()),
        ]),
    };
    
    assert_eq!(in_predicate.column, "status");
    assert_eq!(in_predicate.operator, PredicateOperator::In);
}

#[tokio::test]
async fn test_data_source_for_rest_endpoint() {
    let data_source = DataSource {
        object_type: "rest".to_string(),
        identifier: "users".to_string(),
        alias: Some("u".to_string()),
    };
    
    assert_eq!(data_source.object_type, "rest");
    assert_eq!(data_source.identifier, "users");
    assert_eq!(data_source.alias, Some("u".to_string()));
}