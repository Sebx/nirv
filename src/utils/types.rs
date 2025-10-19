use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;

/// Internal representation of a parsed SQL query
#[derive(Debug, Clone, PartialEq)]
pub struct InternalQuery {
    pub operation: QueryOperation,
    pub sources: Vec<DataSource>,
    pub projections: Vec<Column>,
    pub predicates: Vec<Predicate>,
    pub joins: Vec<Join>,
    pub ordering: Option<OrderBy>,
    pub limit: Option<u64>,
}

/// Types of SQL operations supported
#[derive(Debug, Clone, PartialEq)]
pub enum QueryOperation {
    Select,
    Insert,
    Update,
    Delete,
}

/// Data source specification in a query
#[derive(Debug, Clone, PartialEq)]
pub struct DataSource {
    pub object_type: String,      // e.g., "postgres", "file", "api"
    pub identifier: String,       // e.g., "users", "data.csv", "endpoint"
    pub alias: Option<String>,
}

/// Column specification in projections
#[derive(Debug, Clone, PartialEq)]
pub struct Column {
    pub name: String,
    pub alias: Option<String>,
    pub source: Option<String>,   // Source table/object alias
}

/// WHERE clause predicates
#[derive(Debug, Clone, PartialEq)]
pub struct Predicate {
    pub column: String,
    pub operator: PredicateOperator,
    pub value: PredicateValue,
}

/// Predicate operators
#[derive(Debug, Clone, PartialEq)]
pub enum PredicateOperator {
    Equal,
    NotEqual,
    GreaterThan,
    GreaterThanOrEqual,
    LessThan,
    LessThanOrEqual,
    Like,
    In,
    IsNull,
    IsNotNull,
}

/// Values in predicates
#[derive(Debug, Clone, PartialEq)]
pub enum PredicateValue {
    String(String),
    Number(f64),
    Integer(i64),
    Boolean(bool),
    Null,
    List(Vec<PredicateValue>),
}

/// JOIN specifications
#[derive(Debug, Clone, PartialEq)]
pub struct Join {
    pub join_type: JoinType,
    pub left_source: String,
    pub right_source: String,
    pub on_condition: Vec<JoinCondition>,
}

/// Types of JOINs
#[derive(Debug, Clone, PartialEq)]
pub enum JoinType {
    Inner,
    Left,
    Right,
    Full,
}

/// JOIN conditions
#[derive(Debug, Clone, PartialEq)]
pub struct JoinCondition {
    pub left_column: String,
    pub right_column: String,
}

/// ORDER BY specification
#[derive(Debug, Clone, PartialEq)]
pub struct OrderBy {
    pub columns: Vec<OrderColumn>,
}

/// Column ordering specification
#[derive(Debug, Clone, PartialEq)]
pub struct OrderColumn {
    pub column: String,
    pub direction: OrderDirection,
}

/// Sort direction
#[derive(Debug, Clone, PartialEq)]
pub enum OrderDirection {
    Ascending,
    Descending,
}

/// Query execution result
#[derive(Debug, Clone)]
pub struct QueryResult {
    pub columns: Vec<ColumnMetadata>,
    pub rows: Vec<Row>,
    pub affected_rows: Option<u64>,
    pub execution_time: Duration,
}

/// Metadata for result columns
#[derive(Debug, Clone, PartialEq)]
pub struct ColumnMetadata {
    pub name: String,
    pub data_type: DataType,
    pub nullable: bool,
}

/// Supported data types
#[derive(Debug, Clone, PartialEq)]
pub enum DataType {
    Text,
    Integer,
    Float,
    Boolean,
    Date,
    DateTime,
    Json,
    Binary,
}

/// A row of data in query results
#[derive(Debug, Clone)]
pub struct Row {
    pub values: Vec<Value>,
}

/// Individual cell values
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Text(String),
    Integer(i64),
    Float(f64),
    Boolean(bool),
    Date(String),      // ISO 8601 format
    DateTime(String),  // ISO 8601 format
    Json(String),
    Binary(Vec<u8>),
    Null,
}

/// Schema information for data objects
#[derive(Debug, Clone)]
pub struct Schema {
    pub name: String,
    pub columns: Vec<ColumnMetadata>,
    pub primary_key: Option<Vec<String>>,
    pub indexes: Vec<Index>,
}

/// Index information
#[derive(Debug, Clone)]
pub struct Index {
    pub name: String,
    pub columns: Vec<String>,
    pub unique: bool,
}

/// Connector types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ConnectorType {
    Mock,
    PostgreSQL,
    MySQL,
    SQLite,
    File,
    Rest,
    LLM,
    Custom(String),
}

/// Query to be executed by a specific connector
#[derive(Debug, Clone)]
pub struct ConnectorQuery {
    pub connector_type: ConnectorType,
    pub query: InternalQuery,
    pub connection_params: HashMap<String, String>,
}

impl InternalQuery {
    /// Create a new empty query
    pub fn new(operation: QueryOperation) -> Self {
        Self {
            operation,
            sources: Vec::new(),
            projections: Vec::new(),
            predicates: Vec::new(),
            joins: Vec::new(),
            ordering: None,
            limit: None,
        }
    }
}

impl QueryResult {
    /// Create a new empty result
    pub fn new() -> Self {
        Self {
            columns: Vec::new(),
            rows: Vec::new(),
            affected_rows: None,
            execution_time: Duration::from_millis(0),
        }
    }
    
    /// Get the number of rows in the result
    pub fn row_count(&self) -> usize {
        self.rows.len()
    }
    
    /// Check if the result is empty
    pub fn is_empty(&self) -> bool {
        self.rows.is_empty()
    }
}

impl Row {
    /// Create a new row with the given values
    pub fn new(values: Vec<Value>) -> Self {
        Self { values }
    }
    
    /// Get a value by column index
    pub fn get(&self, index: usize) -> Option<&Value> {
        self.values.get(index)
    }
}

impl Default for QueryResult {
    fn default() -> Self {
        Self::new()
    }
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_internal_query_creation() {
        let query = InternalQuery::new(QueryOperation::Select);
        assert_eq!(query.operation, QueryOperation::Select);
        assert!(query.sources.is_empty());
        assert!(query.projections.is_empty());
        assert!(query.predicates.is_empty());
        assert!(query.joins.is_empty());
        assert!(query.ordering.is_none());
        assert!(query.limit.is_none());
    }

    #[test]
    fn test_data_source_creation() {
        let source = DataSource {
            object_type: "postgres".to_string(),
            identifier: "users".to_string(),
            alias: Some("u".to_string()),
        };
        assert_eq!(source.object_type, "postgres");
        assert_eq!(source.identifier, "users");
        assert_eq!(source.alias, Some("u".to_string()));
    }

    #[test]
    fn test_query_result_creation() {
        let result = QueryResult::new();
        assert!(result.is_empty());
        assert_eq!(result.row_count(), 0);
        assert!(result.columns.is_empty());
        assert!(result.rows.is_empty());
        assert!(result.affected_rows.is_none());
    }

    #[test]
    fn test_row_creation_and_access() {
        let values = vec![
            Value::Text("John".to_string()),
            Value::Integer(25),
            Value::Boolean(true),
        ];
        let row = Row::new(values);
        
        assert_eq!(row.get(0), Some(&Value::Text("John".to_string())));
        assert_eq!(row.get(1), Some(&Value::Integer(25)));
        assert_eq!(row.get(2), Some(&Value::Boolean(true)));
        assert_eq!(row.get(3), None);
    }

    #[test]
    fn test_predicate_value_types() {
        let string_val = PredicateValue::String("test".to_string());
        let number_val = PredicateValue::Number(3.14);
        let int_val = PredicateValue::Integer(42);
        let bool_val = PredicateValue::Boolean(true);
        let null_val = PredicateValue::Null;
        
        match string_val {
            PredicateValue::String(s) => assert_eq!(s, "test"),
            _ => panic!("Expected string value"),
        }
        
        match number_val {
            PredicateValue::Number(n) => assert_eq!(n, 3.14),
            _ => panic!("Expected number value"),
        }
        
        match int_val {
            PredicateValue::Integer(i) => assert_eq!(i, 42),
            _ => panic!("Expected integer value"),
        }
        
        match bool_val {
            PredicateValue::Boolean(b) => assert!(b),
            _ => panic!("Expected boolean value"),
        }
        
        match null_val {
            PredicateValue::Null => {},
            _ => panic!("Expected null value"),
        }
    }

    #[test]
    fn test_connector_type_serialization() {
        let mock_type = ConnectorType::Mock;
        let postgres_type = ConnectorType::PostgreSQL;
        let custom_type = ConnectorType::Custom("MyConnector".to_string());
        
        assert_eq!(mock_type, ConnectorType::Mock);
        assert_eq!(postgres_type, ConnectorType::PostgreSQL);
        assert_eq!(custom_type, ConnectorType::Custom("MyConnector".to_string()));
    }
}