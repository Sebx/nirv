use async_trait::async_trait;
use std::path::{Path, PathBuf};
use std::fs;
use glob::glob;
use csv::ReaderBuilder;
use serde_json;

use crate::connectors::{Connector, ConnectorInitConfig, ConnectorCapabilities};
use crate::utils::{
    types::{
        ConnectorType, ConnectorQuery, QueryResult, Schema, ColumnMetadata, DataType, 
        Row, Value, PredicateOperator, PredicateValue
    },
    error::{ConnectorError, NirvResult},
};

/// File system connector for CSV, JSON, and other file formats
pub struct FileConnector {
    base_path: Option<PathBuf>,
    supported_extensions: Vec<String>,
    connected: bool,
}

impl FileConnector {
    /// Create a new file connector instance
    pub fn new() -> Self {
        Self {
            base_path: None,
            supported_extensions: vec!["csv".to_string(), "json".to_string()],
            connected: false,
        }
    }

    /// Check if a file extension is supported
    fn is_supported_extension(&self, extension: &str) -> bool {
        self.supported_extensions.iter().any(|ext| ext.eq_ignore_ascii_case(extension))
    }

    /// Resolve file path, handling patterns and relative paths
    fn resolve_file_path(&self, identifier: &str) -> NirvResult<Vec<PathBuf>> {
        let base_path = self.base_path.as_ref()
            .ok_or_else(|| ConnectorError::ConnectionFailed("Not connected".to_string()))?;

        let full_path = base_path.join(identifier);
        
        // Check if it's a glob pattern
        if identifier.contains('*') || identifier.contains('?') {
            let pattern = full_path.to_string_lossy();
            let mut paths = Vec::new();
            
            match glob(&pattern) {
                Ok(entries) => {
                    for entry in entries {
                        match entry {
                            Ok(path) => {
                                if path.is_file() {
                                    if let Some(ext) = path.extension() {
                                        if self.is_supported_extension(&ext.to_string_lossy()) {
                                            paths.push(path);
                                        }
                                    }
                                }
                            }
                            Err(_) => continue,
                        }
                    }
                }
                Err(e) => {
                    return Err(ConnectorError::QueryExecutionFailed(
                        format!("Pattern matching failed: {}", e)
                    ).into());
                }
            }
            
            if paths.is_empty() {
                return Err(ConnectorError::QueryExecutionFailed(
                    format!("No files found matching pattern: {}", identifier)
                ).into());
            }
            
            Ok(paths)
        } else {
            // Single file
            if !full_path.exists() {
                return Err(ConnectorError::QueryExecutionFailed(
                    format!("File not found: {}", identifier)
                ).into());
            }
            
            if !full_path.is_file() {
                return Err(ConnectorError::QueryExecutionFailed(
                    format!("Path is not a file: {}", identifier)
                ).into());
            }
            
            // Check file extension
            if let Some(ext) = full_path.extension() {
                if !self.is_supported_extension(&ext.to_string_lossy()) {
                    return Err(ConnectorError::UnsupportedOperation(
                        format!("Unsupported file extension: {}", ext.to_string_lossy())
                    ).into());
                }
            } else {
                return Err(ConnectorError::UnsupportedOperation(
                    "File has no extension".to_string()
                ).into());
            }
            
            Ok(vec![full_path])
        }
    }

    /// Parse CSV file and return structured data
    fn parse_csv_file(&self, file_path: &Path) -> NirvResult<(Vec<ColumnMetadata>, Vec<Row>)> {
        let content = fs::read_to_string(file_path)
            .map_err(|e| ConnectorError::QueryExecutionFailed(
                format!("Failed to read CSV file: {}", e)
            ))?;

        let mut reader = ReaderBuilder::new()
            .has_headers(true)
            .from_reader(content.as_bytes());

        // Get headers
        let headers = reader.headers()
            .map_err(|e| ConnectorError::QueryExecutionFailed(
                format!("Failed to read CSV headers: {}", e)
            ))?;

        let columns: Vec<ColumnMetadata> = headers.iter()
            .map(|header| ColumnMetadata {
                name: header.to_string(),
                data_type: DataType::Text, // Default to text, could be improved with type inference
                nullable: true,
            })
            .collect();

        let mut rows = Vec::new();
        for result in reader.records() {
            let record = result
                .map_err(|e| ConnectorError::QueryExecutionFailed(
                    format!("Failed to read CSV record: {}", e)
                ))?;

            let values: Vec<Value> = record.iter()
                .map(|field| {
                    // Try to infer type from string value
                    if field.is_empty() {
                        Value::Null
                    } else if let Ok(int_val) = field.parse::<i64>() {
                        Value::Integer(int_val)
                    } else if let Ok(float_val) = field.parse::<f64>() {
                        Value::Float(float_val)
                    } else if let Ok(bool_val) = field.parse::<bool>() {
                        Value::Boolean(bool_val)
                    } else {
                        Value::Text(field.to_string())
                    }
                })
                .collect();

            rows.push(Row::new(values));
        }

        Ok((columns, rows))
    }

    /// Parse JSON file and return structured data
    fn parse_json_file(&self, file_path: &Path) -> NirvResult<(Vec<ColumnMetadata>, Vec<Row>)> {
        let content = fs::read_to_string(file_path)
            .map_err(|e| ConnectorError::QueryExecutionFailed(
                format!("Failed to read JSON file: {}", e)
            ))?;

        let json_data: serde_json::Value = serde_json::from_str(&content)
            .map_err(|e| ConnectorError::QueryExecutionFailed(
                format!("Failed to parse JSON: {}", e)
            ))?;

        match json_data {
            serde_json::Value::Array(array) => {
                if array.is_empty() {
                    return Ok((Vec::new(), Vec::new()));
                }

                // Infer schema from first object
                let mut columns = Vec::new();
                if let Some(first_obj) = array.first() {
                    if let serde_json::Value::Object(obj) = first_obj {
                        for key in obj.keys() {
                            columns.push(ColumnMetadata {
                                name: key.clone(),
                                data_type: DataType::Text, // Default to text
                                nullable: true,
                            });
                        }
                    }
                }

                // Convert array to rows
                let mut rows = Vec::new();
                for item in array {
                    if let serde_json::Value::Object(obj) = item {
                        let mut values = Vec::new();
                        for column in &columns {
                            let value = obj.get(&column.name)
                                .map(|v| self.json_value_to_value(v))
                                .unwrap_or(Value::Null);
                            values.push(value);
                        }
                        rows.push(Row::new(values));
                    }
                }

                Ok((columns, rows))
            }
            _ => Err(ConnectorError::QueryExecutionFailed(
                "JSON file must contain an array of objects".to_string()
            ).into())
        }
    }

    /// Convert serde_json::Value to our Value type
    fn json_value_to_value(&self, json_val: &serde_json::Value) -> Value {
        match json_val {
            serde_json::Value::Null => Value::Null,
            serde_json::Value::Bool(b) => Value::Boolean(*b),
            serde_json::Value::Number(n) => {
                if let Some(i) = n.as_i64() {
                    Value::Integer(i)
                } else if let Some(f) = n.as_f64() {
                    Value::Float(f)
                } else {
                    Value::Text(n.to_string())
                }
            }
            serde_json::Value::String(s) => Value::Text(s.clone()),
            serde_json::Value::Array(_) | serde_json::Value::Object(_) => {
                Value::Json(json_val.to_string())
            }
        }
    }

    /// Apply WHERE clause predicates to filter rows
    fn apply_predicates(&self, columns: &[ColumnMetadata], rows: Vec<Row>, predicates: &[crate::utils::types::Predicate]) -> Vec<Row> {
        if predicates.is_empty() {
            return rows;
        }

        rows.into_iter()
            .filter(|row| {
                predicates.iter().all(|predicate| {
                    // Find column index
                    let column_index = columns.iter()
                        .position(|col| col.name == predicate.column);
                    
                    if let Some(index) = column_index {
                        if let Some(value) = row.values.get(index) {
                            self.evaluate_predicate(value, &predicate.operator, &predicate.value)
                        } else {
                            false
                        }
                    } else {
                        false // Column not found
                    }
                })
            })
            .collect()
    }

    /// Evaluate a single predicate against a value
    fn evaluate_predicate(&self, value: &Value, operator: &PredicateOperator, predicate_value: &PredicateValue) -> bool {
        match operator {
            PredicateOperator::Equal => self.values_equal(value, predicate_value),
            PredicateOperator::NotEqual => !self.values_equal(value, predicate_value),
            PredicateOperator::GreaterThan => self.value_greater_than(value, predicate_value),
            PredicateOperator::GreaterThanOrEqual => {
                self.value_greater_than(value, predicate_value) || self.values_equal(value, predicate_value)
            }
            PredicateOperator::LessThan => self.value_less_than(value, predicate_value),
            PredicateOperator::LessThanOrEqual => {
                self.value_less_than(value, predicate_value) || self.values_equal(value, predicate_value)
            }
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
                // Simple LIKE implementation - convert SQL LIKE to regex
                let regex_pattern = pattern
                    .replace('%', ".*")
                    .replace('_', ".");
                
                if let Ok(regex) = regex::Regex::new(&format!("^{}$", regex_pattern)) {
                    regex.is_match(v)
                } else {
                    false
                }
            }
            _ => false,
        }
    }

    /// Check if value is in list
    fn value_in(&self, value: &Value, predicate_value: &PredicateValue) -> bool {
        match predicate_value {
            PredicateValue::List(list) => {
                list.iter().any(|item| self.values_equal(value, item))
            }
            _ => false,
        }
    }
}

impl Default for FileConnector {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Connector for FileConnector {
    async fn connect(&mut self, config: ConnectorInitConfig) -> NirvResult<()> {
        let base_path_str = config.connection_params.get("base_path")
            .ok_or_else(|| ConnectorError::ConnectionFailed(
                "base_path parameter is required".to_string()
            ))?;

        let base_path = PathBuf::from(base_path_str);
        
        if !base_path.exists() {
            return Err(ConnectorError::ConnectionFailed(
                format!("Base path does not exist: {}", base_path_str)
            ).into());
        }

        if !base_path.is_dir() {
            return Err(ConnectorError::ConnectionFailed(
                format!("Base path is not a directory: {}", base_path_str)
            ).into());
        }

        // Update supported extensions if provided
        if let Some(extensions_str) = config.connection_params.get("file_extensions") {
            self.supported_extensions = extensions_str
                .split(',')
                .map(|ext| ext.trim().to_lowercase())
                .collect();
        }

        self.base_path = Some(base_path);
        self.connected = true;

        Ok(())
    }

    async fn execute_query(&self, query: ConnectorQuery) -> NirvResult<QueryResult> {
        if !self.connected {
            return Err(ConnectorError::ConnectionFailed(
                "File connector is not connected".to_string()
            ).into());
        }

        if query.query.sources.is_empty() {
            return Err(ConnectorError::QueryExecutionFailed(
                "No data source specified in query".to_string()
            ).into());
        }

        let source = &query.query.sources[0]; // For now, handle single source
        let file_paths = self.resolve_file_path(&source.identifier)?;

        let mut all_columns: Option<Vec<ColumnMetadata>> = None;
        let mut all_rows = Vec::new();

        // Process each file (for pattern matching)
        for file_path in file_paths {
            let (columns, mut rows) = if let Some(ext) = file_path.extension() {
                match ext.to_string_lossy().to_lowercase().as_str() {
                    "csv" => self.parse_csv_file(&file_path)?,
                    "json" => self.parse_json_file(&file_path)?,
                    _ => return Err(ConnectorError::UnsupportedOperation(
                        format!("Unsupported file extension: {}", ext.to_string_lossy())
                    ).into()),
                }
            } else {
                return Err(ConnectorError::UnsupportedOperation(
                    "File has no extension".to_string()
                ).into());
            };

            // Apply WHERE clause predicates (pushdown optimization)
            rows = self.apply_predicates(&columns, rows, &query.query.predicates);

            // For multiple files, ensure schema compatibility
            if let Some(ref existing_columns) = all_columns {
                if existing_columns.len() != columns.len() || 
                   existing_columns.iter().zip(columns.iter()).any(|(a, b)| a.name != b.name) {
                    return Err(ConnectorError::QueryExecutionFailed(
                        "Schema mismatch between files in pattern".to_string()
                    ).into());
                }
            } else {
                all_columns = Some(columns);
            }

            all_rows.extend(rows);
        }

        let columns = all_columns.unwrap_or_default();

        // Apply LIMIT if specified
        if let Some(limit) = query.query.limit {
            all_rows.truncate(limit as usize);
        }

        Ok(QueryResult {
            columns,
            rows: all_rows,
            affected_rows: None,
            execution_time: std::time::Duration::from_millis(0), // TODO: measure actual time
        })
    }

    async fn get_schema(&self, object_name: &str) -> NirvResult<Schema> {
        if !self.connected {
            return Err(ConnectorError::ConnectionFailed(
                "File connector is not connected".to_string()
            ).into());
        }

        let file_paths = self.resolve_file_path(object_name)?;
        
        if file_paths.is_empty() {
            return Err(ConnectorError::SchemaRetrievalFailed(
                format!("No files found for: {}", object_name)
            ).into());
        }

        // Use first file for schema (assuming all files in pattern have same schema)
        let file_path = &file_paths[0];
        
        let (columns, _) = if let Some(ext) = file_path.extension() {
            match ext.to_string_lossy().to_lowercase().as_str() {
                "csv" => self.parse_csv_file(file_path)?,
                "json" => self.parse_json_file(file_path)?,
                _ => return Err(ConnectorError::UnsupportedOperation(
                    format!("Unsupported file extension: {}", ext.to_string_lossy())
                ).into()),
            }
        } else {
            return Err(ConnectorError::UnsupportedOperation(
                "File has no extension".to_string()
            ).into());
        };

        Ok(Schema {
            name: object_name.to_string(),
            columns,
            primary_key: None,
            indexes: Vec::new(),
        })
    }

    async fn disconnect(&mut self) -> NirvResult<()> {
        self.base_path = None;
        self.connected = false;
        Ok(())
    }

    fn get_connector_type(&self) -> ConnectorType {
        ConnectorType::File
    }

    fn supports_transactions(&self) -> bool {
        false // File system doesn't support transactions
    }

    fn is_connected(&self) -> bool {
        self.connected
    }

    fn get_capabilities(&self) -> ConnectorCapabilities {
        ConnectorCapabilities {
            supports_joins: false, // No cross-file joins for now
            supports_aggregations: true, // Basic aggregations can be implemented
            supports_subqueries: false,
            supports_transactions: false,
            supports_schema_introspection: true,
            max_concurrent_queries: Some(1), // Single-threaded file access for now
        }
    }
}