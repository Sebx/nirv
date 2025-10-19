use async_trait::async_trait;
use std::collections::HashMap;
use std::time::{Duration, Instant};
use std::sync::Arc;
use reqwest::{Client, Method, Response};
use serde_json::{Value as JsonValue, Map};
use url::Url;
use dashmap::DashMap;
use tokio::time::sleep;

use crate::connectors::{Connector, ConnectorInitConfig, ConnectorCapabilities};
use crate::utils::{
    types::{
        ConnectorType, ConnectorQuery, QueryResult, Schema, ColumnMetadata, DataType,
        Row, Value, PredicateOperator, PredicateValue
    },
    error::{ConnectorError, NirvResult},
};

/// Authentication configuration for REST APIs
#[derive(Debug, Clone)]
pub enum AuthConfig {
    None,
    ApiKey { header: String, key: String },
    Bearer { token: String },
    Basic { username: String, password: String },
}

/// Cache entry for REST responses
#[derive(Debug, Clone)]
struct CacheEntry {
    data: JsonValue,
    timestamp: Instant,
    ttl: Duration,
}

impl CacheEntry {
    fn new(data: JsonValue, ttl: Duration) -> Self {
        Self {
            data,
            timestamp: Instant::now(),
            ttl,
        }
    }
    
    fn is_expired(&self) -> bool {
        self.timestamp.elapsed() > self.ttl
    }
}

/// Rate limiting configuration
#[derive(Debug, Clone)]
pub struct RateLimitConfig {
    pub requests_per_second: f64,
    pub burst_size: u32,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            requests_per_second: 10.0,
            burst_size: 10,
        }
    }
}

/// Rate limiter state
#[derive(Debug)]
struct RateLimiter {
    config: RateLimitConfig,
    last_request: Option<Instant>,
    tokens: f64,
}

impl RateLimiter {
    fn new(config: RateLimitConfig) -> Self {
        Self {
            tokens: config.burst_size as f64,
            config,
            last_request: None,
        }
    }
    
    async fn acquire(&mut self) -> NirvResult<()> {
        let now = Instant::now();
        
        // Refill tokens based on time elapsed
        if let Some(last) = self.last_request {
            let elapsed = now.duration_since(last).as_secs_f64();
            self.tokens = (self.tokens + elapsed * self.config.requests_per_second)
                .min(self.config.burst_size as f64);
        }
        
        if self.tokens >= 1.0 {
            self.tokens -= 1.0;
            self.last_request = Some(now);
            Ok(())
        } else {
            // Calculate wait time
            let wait_time = Duration::from_secs_f64(1.0 / self.config.requests_per_second);
            sleep(wait_time).await;
            self.tokens = (self.config.burst_size as f64 - 1.0).max(0.0);
            self.last_request = Some(Instant::now());
            Ok(())
        }
    }
}

/// REST API connector with authentication, caching, and rate limiting
pub struct RestConnector {
    client: Option<Client>,
    base_url: Option<Url>,
    auth_config: AuthConfig,
    cache: Arc<DashMap<String, CacheEntry>>,
    cache_ttl: Duration,
    rate_limiter: Option<RateLimiter>,
    connected: bool,
    endpoint_mappings: HashMap<String, EndpointMapping>,
}

/// Mapping configuration for REST endpoints
#[derive(Debug, Clone)]
pub struct EndpointMapping {
    pub path: String,
    pub method: Method,
    pub query_params: HashMap<String, String>,
    pub response_path: Option<String>, // JSONPath to extract data array
    pub id_field: Option<String>,      // Field to use as primary key
}

impl RestConnector {
    /// Create a new REST connector
    pub fn new() -> Self {
        Self {
            client: None,
            base_url: None,
            auth_config: AuthConfig::None,
            cache: Arc::new(DashMap::new()),
            cache_ttl: Duration::from_secs(300), // 5 minutes default
            rate_limiter: None,
            connected: false,
            endpoint_mappings: HashMap::new(),
        }
    }
    
    /// Configure authentication
    pub fn with_auth(mut self, auth: AuthConfig) -> Self {
        self.auth_config = auth;
        self
    }
    
    /// Configure cache TTL
    pub fn with_cache_ttl(mut self, ttl: Duration) -> Self {
        self.cache_ttl = ttl;
        self
    }
    
    /// Configure rate limiting
    pub fn with_rate_limit(mut self, config: RateLimitConfig) -> Self {
        self.rate_limiter = Some(RateLimiter::new(config));
        self
    }
    
    /// Add endpoint mapping
    pub fn add_endpoint_mapping(&mut self, name: String, mapping: EndpointMapping) {
        self.endpoint_mappings.insert(name, mapping);
    }
    
    /// Build HTTP request with authentication
    async fn build_request(&self, method: Method, url: &Url) -> NirvResult<reqwest::RequestBuilder> {
        let client = self.client.as_ref()
            .ok_or_else(|| ConnectorError::ConnectionFailed("Not connected".to_string()))?;
        
        let mut request = client.request(method, url.clone());
        
        // Apply authentication
        match &self.auth_config {
            AuthConfig::None => {},
            AuthConfig::ApiKey { header, key } => {
                request = request.header(header, key);
            },
            AuthConfig::Bearer { token } => {
                request = request.bearer_auth(token);
            },
            AuthConfig::Basic { username, password } => {
                request = request.basic_auth(username, Some(password));
            },
        }
        
        Ok(request)
    }
    
    /// Execute HTTP request with rate limiting
    async fn execute_request(&mut self, method: Method, url: &Url) -> NirvResult<Response> {
        // Apply rate limiting
        if let Some(ref mut limiter) = self.rate_limiter {
            limiter.acquire().await?;
        }
        
        let request = self.build_request(method, url).await?;
        
        let response = request.send().await
            .map_err(|e| ConnectorError::QueryExecutionFailed(
                format!("HTTP request failed: {}", e)
            ))?;
        
        if !response.status().is_success() {
            return Err(ConnectorError::QueryExecutionFailed(
                format!("HTTP request failed with status: {}", response.status())
            ).into());
        }
        
        Ok(response)
    }
    
    /// Get data from cache or fetch from API
    async fn get_cached_or_fetch(&mut self, cache_key: &str, url: &Url, method: Method) -> NirvResult<JsonValue> {
        // Check cache first
        if let Some(entry) = self.cache.get(cache_key) {
            if !entry.is_expired() {
                return Ok(entry.data.clone());
            }
        }
        
        // Fetch from API
        let response = self.execute_request(method, url).await?;
        let json_data: JsonValue = response.json().await
            .map_err(|e| ConnectorError::QueryExecutionFailed(
                format!("Failed to parse JSON response: {}", e)
            ))?;
        
        // Cache the result
        let entry = CacheEntry::new(json_data.clone(), self.cache_ttl);
        self.cache.insert(cache_key.to_string(), entry);
        
        Ok(json_data)
    }
    
    /// Extract data array from JSON response using JSONPath
    fn extract_data_array(&self, json: &JsonValue, path: Option<&str>) -> NirvResult<Vec<JsonValue>> {
        match path {
            Some(json_path) => {
                // Simple JSONPath implementation for basic cases
                let parts: Vec<&str> = json_path.split('.').collect();
                let mut current = json;
                
                for part in parts {
                    if part.is_empty() {
                        continue;
                    }
                    
                    current = current.get(part)
                        .ok_or_else(|| ConnectorError::QueryExecutionFailed(
                            format!("JSONPath '{}' not found in response", json_path)
                        ))?;
                }
                
                match current {
                    JsonValue::Array(arr) => Ok(arr.clone()),
                    _ => Err(ConnectorError::QueryExecutionFailed(
                        format!("JSONPath '{}' does not point to an array", json_path)
                    ).into()),
                }
            },
            None => {
                match json {
                    JsonValue::Array(arr) => Ok(arr.clone()),
                    JsonValue::Object(_) => Ok(vec![json.clone()]),
                    _ => Err(ConnectorError::QueryExecutionFailed(
                        "Response is not an array or object".to_string()
                    ).into()),
                }
            }
        }
    }
    
    /// Convert JSON object to Row
    fn json_to_row(&self, json_obj: &JsonValue, columns: &[ColumnMetadata]) -> Row {
        let mut values = Vec::new();
        
        for column in columns {
            let value = if let JsonValue::Object(obj) = json_obj {
                obj.get(&column.name)
                    .map(|v| self.json_value_to_value(v))
                    .unwrap_or(Value::Null)
            } else {
                Value::Null
            };
            values.push(value);
        }
        
        Row::new(values)
    }
    
    /// Convert JsonValue to our Value type
    fn json_value_to_value(&self, json_val: &JsonValue) -> Value {
        match json_val {
            JsonValue::Null => Value::Null,
            JsonValue::Bool(b) => Value::Boolean(*b),
            JsonValue::Number(n) => {
                if let Some(i) = n.as_i64() {
                    Value::Integer(i)
                } else if let Some(f) = n.as_f64() {
                    Value::Float(f)
                } else {
                    Value::Text(n.to_string())
                }
            },
            JsonValue::String(s) => Value::Text(s.clone()),
            JsonValue::Array(_) | JsonValue::Object(_) => {
                Value::Json(json_val.to_string())
            },
        }
    }
    
    /// Infer schema from JSON data
    fn infer_schema_from_json(&self, data: &[JsonValue], object_name: &str) -> Schema {
        let mut columns = Vec::new();
        
        if let Some(first_obj) = data.first() {
            if let JsonValue::Object(obj) = first_obj {
                for (key, value) in obj {
                    let data_type = match value {
                        JsonValue::Null => DataType::Text,
                        JsonValue::Bool(_) => DataType::Boolean,
                        JsonValue::Number(n) => {
                            if n.is_i64() {
                                DataType::Integer
                            } else {
                                DataType::Float
                            }
                        },
                        JsonValue::String(_) => DataType::Text,
                        JsonValue::Array(_) | JsonValue::Object(_) => DataType::Json,
                    };
                    
                    columns.push(ColumnMetadata {
                        name: key.clone(),
                        data_type,
                        nullable: true,
                    });
                }
            }
        }
        
        Schema {
            name: object_name.to_string(),
            columns,
            primary_key: None,
            indexes: Vec::new(),
        }
    }
    
    /// Apply WHERE clause predicates to filter data
    fn apply_predicates(&self, data: Vec<JsonValue>, predicates: &[crate::utils::types::Predicate]) -> Vec<JsonValue> {
        if predicates.is_empty() {
            return data;
        }
        
        data.into_iter()
            .filter(|item| {
                if let JsonValue::Object(obj) = item {
                    predicates.iter().all(|predicate| {
                        if let Some(field_value) = obj.get(&predicate.column) {
                            let value = self.json_value_to_value(field_value);
                            self.evaluate_predicate(&value, &predicate.operator, &predicate.value)
                        } else {
                            false
                        }
                    })
                } else {
                    false
                }
            })
            .collect()
    }
    
    /// Evaluate a single predicate
    fn evaluate_predicate(&self, value: &Value, operator: &PredicateOperator, predicate_value: &PredicateValue) -> bool {
        match operator {
            PredicateOperator::Equal => self.values_equal(value, predicate_value),
            PredicateOperator::NotEqual => !self.values_equal(value, predicate_value),
            PredicateOperator::GreaterThan => self.value_greater_than(value, predicate_value),
            PredicateOperator::GreaterThanOrEqual => {
                self.value_greater_than(value, predicate_value) || self.values_equal(value, predicate_value)
            },
            PredicateOperator::LessThan => self.value_less_than(value, predicate_value),
            PredicateOperator::LessThanOrEqual => {
                self.value_less_than(value, predicate_value) || self.values_equal(value, predicate_value)
            },
            PredicateOperator::Like => self.value_like(value, predicate_value),
            PredicateOperator::In => self.value_in(value, predicate_value),
            PredicateOperator::IsNull => matches!(value, Value::Null),
            PredicateOperator::IsNotNull => !matches!(value, Value::Null),
        }
    }
    
    /// Check if two values are equal
    fn values_equal(&self, value: &Value, predicate_value: &PredicateValue) -> bool {
        match (value, predicate_value) {
            (Value::Text(v), PredicateValue::String(p)) => v == p,
            (Value::Integer(v), PredicateValue::Integer(p)) => v == p,
            (Value::Float(v), PredicateValue::Number(p)) => (v - p).abs() < f64::EPSILON,
            (Value::Boolean(v), PredicateValue::Boolean(p)) => v == p,
            (Value::Null, PredicateValue::Null) => true,
            // Type coercion
            (Value::Integer(v), PredicateValue::Number(p)) => (*v as f64 - p).abs() < f64::EPSILON,
            (Value::Float(v), PredicateValue::Integer(p)) => (v - *p as f64).abs() < f64::EPSILON,
            _ => false,
        }
    }
    
    /// Check if value is greater than predicate value
    fn value_greater_than(&self, value: &Value, predicate_value: &PredicateValue) -> bool {
        match (value, predicate_value) {
            (Value::Integer(v), PredicateValue::Integer(p)) => v > p,
            (Value::Float(v), PredicateValue::Number(p)) => v > p,
            (Value::Integer(v), PredicateValue::Number(p)) => (*v as f64) > *p,
            (Value::Float(v), PredicateValue::Integer(p)) => *v > (*p as f64),
            (Value::Text(v), PredicateValue::String(p)) => v > p,
            _ => false,
        }
    }
    
    /// Check if value is less than predicate value
    fn value_less_than(&self, value: &Value, predicate_value: &PredicateValue) -> bool {
        match (value, predicate_value) {
            (Value::Integer(v), PredicateValue::Integer(p)) => v < p,
            (Value::Float(v), PredicateValue::Number(p)) => v < p,
            (Value::Integer(v), PredicateValue::Number(p)) => (*v as f64) < *p,
            (Value::Float(v), PredicateValue::Integer(p)) => *v < (*p as f64),
            (Value::Text(v), PredicateValue::String(p)) => v < p,
            _ => false,
        }
    }
    
    /// Check if value matches LIKE pattern
    fn value_like(&self, value: &Value, predicate_value: &PredicateValue) -> bool {
        match (value, predicate_value) {
            (Value::Text(v), PredicateValue::String(pattern)) => {
                let regex_pattern = pattern
                    .replace('%', ".*")
                    .replace('_', ".");
                
                if let Ok(regex) = regex::Regex::new(&format!("^{}$", regex_pattern)) {
                    regex.is_match(v)
                } else {
                    false
                }
            },
            _ => false,
        }
    }
    
    /// Check if value is in list
    fn value_in(&self, value: &Value, predicate_value: &PredicateValue) -> bool {
        match predicate_value {
            PredicateValue::List(list) => {
                list.iter().any(|item| self.values_equal(value, item))
            },
            _ => false,
        }
    }
}

impl Default for RestConnector {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Connector for RestConnector {
    async fn connect(&mut self, config: ConnectorInitConfig) -> NirvResult<()> {
        let base_url_str = config.connection_params.get("base_url")
            .ok_or_else(|| ConnectorError::ConnectionFailed(
                "base_url parameter is required".to_string()
            ))?;
        
        let base_url = Url::parse(base_url_str)
            .map_err(|e| ConnectorError::ConnectionFailed(
                format!("Invalid base URL: {}", e)
            ))?;
        
        // Configure authentication
        if let Some(auth_type) = config.connection_params.get("auth_type") {
            self.auth_config = match auth_type.as_str() {
                "api_key" => {
                    let header = config.connection_params.get("auth_header")
                        .unwrap_or(&"X-API-Key".to_string()).clone();
                    let key = config.connection_params.get("api_key")
                        .ok_or_else(|| ConnectorError::ConnectionFailed(
                            "api_key parameter is required for API key auth".to_string()
                        ))?.clone();
                    AuthConfig::ApiKey { header, key }
                },
                "bearer" => {
                    let token = config.connection_params.get("bearer_token")
                        .ok_or_else(|| ConnectorError::ConnectionFailed(
                            "bearer_token parameter is required for bearer auth".to_string()
                        ))?.clone();
                    AuthConfig::Bearer { token }
                },
                "basic" => {
                    let username = config.connection_params.get("username")
                        .ok_or_else(|| ConnectorError::ConnectionFailed(
                            "username parameter is required for basic auth".to_string()
                        ))?.clone();
                    let password = config.connection_params.get("password")
                        .ok_or_else(|| ConnectorError::ConnectionFailed(
                            "password parameter is required for basic auth".to_string()
                        ))?.clone();
                    AuthConfig::Basic { username, password }
                },
                "none" | _ => AuthConfig::None,
            };
        }
        
        // Configure cache TTL
        if let Some(cache_ttl_str) = config.connection_params.get("cache_ttl_seconds") {
            if let Ok(ttl_seconds) = cache_ttl_str.parse::<u64>() {
                self.cache_ttl = Duration::from_secs(ttl_seconds);
            }
        }
        
        // Configure rate limiting
        if let Some(rps_str) = config.connection_params.get("rate_limit_rps") {
            if let Ok(rps) = rps_str.parse::<f64>() {
                let burst_size = config.connection_params.get("rate_limit_burst")
                    .and_then(|s| s.parse::<u32>().ok())
                    .unwrap_or(10);
                
                let rate_config = RateLimitConfig {
                    requests_per_second: rps,
                    burst_size,
                };
                self.rate_limiter = Some(RateLimiter::new(rate_config));
            }
        }
        
        // Create HTTP client
        let timeout = Duration::from_secs(config.timeout_seconds.unwrap_or(30));
        let client = Client::builder()
            .timeout(timeout)
            .build()
            .map_err(|e| ConnectorError::ConnectionFailed(
                format!("Failed to create HTTP client: {}", e)
            ))?;
        
        self.client = Some(client);
        self.base_url = Some(base_url);
        self.connected = true;
        
        Ok(())
    }
    
    async fn execute_query(&self, query: ConnectorQuery) -> NirvResult<QueryResult> {
        if !self.connected {
            return Err(ConnectorError::ConnectionFailed("Not connected".to_string()).into());
        }
        
        if query.query.sources.is_empty() {
            return Err(ConnectorError::QueryExecutionFailed(
                "No data source specified in query".to_string()
            ).into());
        }
        
        let source = &query.query.sources[0];
        let endpoint_name = &source.identifier;
        
        // Get endpoint mapping
        let mapping = self.endpoint_mappings.get(endpoint_name)
            .ok_or_else(|| ConnectorError::QueryExecutionFailed(
                format!("No endpoint mapping found for '{}'", endpoint_name)
            ))?;
        
        let base_url = self.base_url.as_ref()
            .ok_or_else(|| ConnectorError::ConnectionFailed("Not connected".to_string()))?;
        
        let mut url = base_url.join(&mapping.path)
            .map_err(|e| ConnectorError::QueryExecutionFailed(
                format!("Failed to build URL: {}", e)
            ))?;
        
        // Add query parameters
        {
            let mut query_pairs = url.query_pairs_mut();
            for (key, value) in &mapping.query_params {
                query_pairs.append_pair(key, value);
            }
        }
        
        let start_time = Instant::now();
        let cache_key = format!("{}:{}", endpoint_name, url.as_str());
        
        // This is a bit of a hack to get around the borrow checker
        // We need to make the method call mutable but we can't change the trait
        let mut temp_connector = RestConnector {
            client: self.client.clone(),
            base_url: self.base_url.clone(),
            auth_config: self.auth_config.clone(),
            cache: self.cache.clone(),
            cache_ttl: self.cache_ttl,
            rate_limiter: None, // We'll handle rate limiting differently
            connected: self.connected,
            endpoint_mappings: self.endpoint_mappings.clone(),
        };
        
        let json_data = temp_connector.get_cached_or_fetch(&cache_key, &url, mapping.method.clone()).await?;
        let data_array = temp_connector.extract_data_array(&json_data, mapping.response_path.as_deref())?;
        
        // Apply WHERE clause predicates
        let filtered_data = temp_connector.apply_predicates(data_array, &query.query.predicates);
        
        // Infer schema from data
        let schema = temp_connector.infer_schema_from_json(&filtered_data, endpoint_name);
        
        // Convert to rows
        let mut rows = Vec::new();
        for item in &filtered_data {
            let row = temp_connector.json_to_row(item, &schema.columns);
            rows.push(row);
        }
        
        // Apply LIMIT if specified
        if let Some(limit) = query.query.limit {
            rows.truncate(limit as usize);
        }
        
        let execution_time = start_time.elapsed();
        
        Ok(QueryResult {
            columns: schema.columns,
            rows,
            affected_rows: Some(filtered_data.len() as u64),
            execution_time,
        })
    }
    
    async fn get_schema(&self, object_name: &str) -> NirvResult<Schema> {
        if !self.connected {
            return Err(ConnectorError::ConnectionFailed("Not connected".to_string()).into());
        }
        
        // Get endpoint mapping
        let mapping = self.endpoint_mappings.get(object_name)
            .ok_or_else(|| ConnectorError::SchemaRetrievalFailed(
                format!("No endpoint mapping found for '{}'", object_name)
            ))?;
        
        let base_url = self.base_url.as_ref()
            .ok_or_else(|| ConnectorError::ConnectionFailed("Not connected".to_string()))?;
        
        let mut url = base_url.join(&mapping.path)
            .map_err(|e| ConnectorError::SchemaRetrievalFailed(
                format!("Failed to build URL: {}", e)
            ))?;
        
        // Add query parameters
        {
            let mut query_pairs = url.query_pairs_mut();
            for (key, value) in &mapping.query_params {
                query_pairs.append_pair(key, value);
            }
        }
        
        let cache_key = format!("schema:{}:{}", object_name, url.as_str());
        
        // Similar hack for mutability
        let mut temp_connector = RestConnector {
            client: self.client.clone(),
            base_url: self.base_url.clone(),
            auth_config: self.auth_config.clone(),
            cache: self.cache.clone(),
            cache_ttl: self.cache_ttl,
            rate_limiter: None,
            connected: self.connected,
            endpoint_mappings: self.endpoint_mappings.clone(),
        };
        
        let json_data = temp_connector.get_cached_or_fetch(&cache_key, &url, mapping.method.clone()).await?;
        let data_array = temp_connector.extract_data_array(&json_data, mapping.response_path.as_deref())?;
        
        Ok(temp_connector.infer_schema_from_json(&data_array, object_name))
    }
    
    async fn disconnect(&mut self) -> NirvResult<()> {
        self.client = None;
        self.base_url = None;
        self.connected = false;
        self.cache.clear();
        Ok(())
    }
    
    fn get_connector_type(&self) -> ConnectorType {
        ConnectorType::Rest
    }
    
    fn supports_transactions(&self) -> bool {
        false // REST APIs typically don't support transactions
    }
    
    fn is_connected(&self) -> bool {
        self.connected
    }
    
    fn get_capabilities(&self) -> ConnectorCapabilities {
        ConnectorCapabilities {
            supports_joins: false, // No cross-endpoint joins for now
            supports_aggregations: true, // Basic aggregations can be implemented
            supports_subqueries: false,
            supports_transactions: false,
            supports_schema_introspection: true,
            max_concurrent_queries: Some(5), // Limited by rate limiting
        }
    }
}