use async_trait::async_trait;
use deadpool_postgres::{Config, Pool, Runtime};

use std::time::{Duration, Instant};
use tokio_postgres::{NoTls, Row as PgRow};

use crate::connectors::connector_trait::{Connector, ConnectorInitConfig, ConnectorCapabilities};
use crate::utils::{
    types::{
        ConnectorType, ConnectorQuery, QueryResult, Schema, ColumnMetadata, 
        DataType, Row, Value, Index, QueryOperation, PredicateOperator
    },
    error::{ConnectorError, NirvResult},
};

/// PostgreSQL connector using tokio-postgres with connection pooling
#[derive(Debug)]
pub struct PostgresConnector {
    pool: Option<Pool>,
    connected: bool,
}

impl PostgresConnector {
    /// Create a new PostgreSQL connector
    pub fn new() -> Self {
        Self {
            pool: None,
            connected: false,
        }
    }
    
    /// Convert PostgreSQL row to internal Row representation
    fn convert_pg_row(&self, pg_row: &PgRow) -> NirvResult<Row> {
        let mut values = Vec::new();
        
        for i in 0..pg_row.len() {
            let value = self.convert_pg_value(pg_row, i)?;
            values.push(value);
        }
        
        Ok(Row::new(values))
    }
    
    /// Convert PostgreSQL value to internal Value representation
    fn convert_pg_value(&self, row: &PgRow, index: usize) -> NirvResult<Value> {
        let column = &row.columns()[index];
        let type_oid = column.type_().oid();
        
        // Handle NULL values first
        if row.try_get::<_, Option<String>>(index).unwrap_or(None).is_none() {
            return Ok(Value::Null);
        }
        
        // Convert based on PostgreSQL type OID
        match type_oid {
            // Text types
            25 | 1043 | 1042 => { // TEXT, VARCHAR, CHAR
                let val: String = row.try_get(index)
                    .map_err(|e| ConnectorError::QueryExecutionFailed(format!("Failed to get text value: {}", e)))?;
                Ok(Value::Text(val))
            }
            // Integer types
            23 => { // INT4
                let val: i32 = row.try_get(index)
                    .map_err(|e| ConnectorError::QueryExecutionFailed(format!("Failed to get int4 value: {}", e)))?;
                Ok(Value::Integer(val as i64))
            }
            20 => { // INT8
                let val: i64 = row.try_get(index)
                    .map_err(|e| ConnectorError::QueryExecutionFailed(format!("Failed to get int8 value: {}", e)))?;
                Ok(Value::Integer(val))
            }
            21 => { // INT2
                let val: i16 = row.try_get(index)
                    .map_err(|e| ConnectorError::QueryExecutionFailed(format!("Failed to get int2 value: {}", e)))?;
                Ok(Value::Integer(val as i64))
            }
            // Float types
            700 => { // FLOAT4
                let val: f32 = row.try_get(index)
                    .map_err(|e| ConnectorError::QueryExecutionFailed(format!("Failed to get float4 value: {}", e)))?;
                Ok(Value::Float(val as f64))
            }
            701 => { // FLOAT8
                let val: f64 = row.try_get(index)
                    .map_err(|e| ConnectorError::QueryExecutionFailed(format!("Failed to get float8 value: {}", e)))?;
                Ok(Value::Float(val))
            }
            // Boolean type
            16 => { // BOOL
                let val: bool = row.try_get(index)
                    .map_err(|e| ConnectorError::QueryExecutionFailed(format!("Failed to get bool value: {}", e)))?;
                Ok(Value::Boolean(val))
            }
            // JSON types
            114 | 3802 => { // JSON, JSONB
                let val: String = row.try_get(index)
                    .map_err(|e| ConnectorError::QueryExecutionFailed(format!("Failed to get json value: {}", e)))?;
                Ok(Value::Json(val))
            }
            // Date/Time types
            1082 => { // DATE
                let val: String = row.try_get(index)
                    .map_err(|e| ConnectorError::QueryExecutionFailed(format!("Failed to get date value: {}", e)))?;
                Ok(Value::Date(val))
            }
            1114 | 1184 => { // TIMESTAMP, TIMESTAMPTZ
                let val: String = row.try_get(index)
                    .map_err(|e| ConnectorError::QueryExecutionFailed(format!("Failed to get timestamp value: {}", e)))?;
                Ok(Value::DateTime(val))
            }
            // Binary types
            17 => { // BYTEA
                let val: Vec<u8> = row.try_get(index)
                    .map_err(|e| ConnectorError::QueryExecutionFailed(format!("Failed to get bytea value: {}", e)))?;
                Ok(Value::Binary(val))
            }
            // Default: convert to string
            _ => {
                let val: String = row.try_get(index)
                    .map_err(|e| ConnectorError::QueryExecutionFailed(format!("Failed to get value as string: {}", e)))?;
                Ok(Value::Text(val))
            }
        }
    }
    
    /// Convert PostgreSQL type OID to internal DataType
    fn pg_type_to_data_type(&self, type_oid: u32) -> DataType {
        match type_oid {
            25 | 1043 | 1042 => DataType::Text,     // TEXT, VARCHAR, CHAR
            23 | 20 | 21 => DataType::Integer,      // INT4, INT8, INT2
            700 | 701 => DataType::Float,           // FLOAT4, FLOAT8
            16 => DataType::Boolean,                // BOOL
            114 | 3802 => DataType::Json,           // JSON, JSONB
            1082 => DataType::Date,                 // DATE
            1114 | 1184 => DataType::DateTime,      // TIMESTAMP, TIMESTAMPTZ
            17 => DataType::Binary,                 // BYTEA
            _ => DataType::Text,                    // Default to text
        }
    }
    
    /// Build SQL query from internal query representation
    fn build_sql_query(&self, query: &crate::utils::types::InternalQuery) -> NirvResult<String> {
        match query.operation {
            QueryOperation::Select => {
                let mut sql = String::from("SELECT ");
                
                // Handle projections
                if query.projections.is_empty() {
                    sql.push('*');
                } else {
                    let projections: Vec<String> = query.projections.iter()
                        .map(|col| {
                            if let Some(alias) = &col.alias {
                                format!("{} AS {}", col.name, alias)
                            } else {
                                col.name.clone()
                            }
                        })
                        .collect();
                    sql.push_str(&projections.join(", "));
                }
                
                // Handle FROM clause
                if let Some(source) = query.sources.first() {
                    sql.push_str(" FROM ");
                    sql.push_str(&source.identifier);
                    if let Some(alias) = &source.alias {
                        sql.push_str(" AS ");
                        sql.push_str(alias);
                    }
                } else {
                    return Err(ConnectorError::QueryExecutionFailed(
                        "No data source specified in query".to_string()
                    ).into());
                }
                
                // Handle WHERE clause
                if !query.predicates.is_empty() {
                    sql.push_str(" WHERE ");
                    let predicates: Vec<String> = query.predicates.iter()
                        .map(|pred| self.build_predicate_sql(pred))
                        .collect::<Result<Vec<_>, _>>()?;
                    sql.push_str(&predicates.join(" AND "));
                }
                
                // Handle ORDER BY
                if let Some(order_by) = &query.ordering {
                    sql.push_str(" ORDER BY ");
                    let order_columns: Vec<String> = order_by.columns.iter()
                        .map(|col| {
                            let direction = match col.direction {
                                crate::utils::types::OrderDirection::Ascending => "ASC",
                                crate::utils::types::OrderDirection::Descending => "DESC",
                            };
                            format!("{} {}", col.column, direction)
                        })
                        .collect();
                    sql.push_str(&order_columns.join(", "));
                }
                
                // Handle LIMIT
                if let Some(limit) = query.limit {
                    sql.push_str(&format!(" LIMIT {}", limit));
                }
                
                Ok(sql)
            }
            _ => Err(ConnectorError::UnsupportedOperation(
                format!("Operation {:?} not supported by PostgreSQL connector", query.operation)
            ).into()),
        }
    }
    
    /// Build SQL for a single predicate
    fn build_predicate_sql(&self, predicate: &crate::utils::types::Predicate) -> NirvResult<String> {
        let operator_sql = match predicate.operator {
            PredicateOperator::Equal => "=",
            PredicateOperator::NotEqual => "!=",
            PredicateOperator::GreaterThan => ">",
            PredicateOperator::GreaterThanOrEqual => ">=",
            PredicateOperator::LessThan => "<",
            PredicateOperator::LessThanOrEqual => "<=",
            PredicateOperator::Like => "LIKE",
            PredicateOperator::IsNull => "IS NULL",
            PredicateOperator::IsNotNull => "IS NOT NULL",
            PredicateOperator::In => "IN",
        };
        
        match predicate.operator {
            PredicateOperator::IsNull | PredicateOperator::IsNotNull => {
                Ok(format!("{} {}", predicate.column, operator_sql))
            }
            PredicateOperator::In => {
                if let crate::utils::types::PredicateValue::List(values) = &predicate.value {
                    let value_strings: Vec<String> = values.iter()
                        .map(|v| self.format_predicate_value(v))
                        .collect::<Result<Vec<_>, _>>()?;
                    Ok(format!("{} IN ({})", predicate.column, value_strings.join(", ")))
                } else {
                    Err(ConnectorError::QueryExecutionFailed(
                        "IN operator requires a list of values".to_string()
                    ).into())
                }
            }
            _ => {
                let value_str = self.format_predicate_value(&predicate.value)?;
                Ok(format!("{} {} {}", predicate.column, operator_sql, value_str))
            }
        }
    }
    
    /// Format predicate value for SQL
    fn format_predicate_value(&self, value: &crate::utils::types::PredicateValue) -> NirvResult<String> {
        match value {
            crate::utils::types::PredicateValue::String(s) => Ok(format!("'{}'", s.replace('\'', "''"))),
            crate::utils::types::PredicateValue::Number(n) => Ok(n.to_string()),
            crate::utils::types::PredicateValue::Integer(i) => Ok(i.to_string()),
            crate::utils::types::PredicateValue::Boolean(b) => Ok(b.to_string()),
            crate::utils::types::PredicateValue::Null => Ok("NULL".to_string()),
            crate::utils::types::PredicateValue::List(_) => {
                Err(ConnectorError::QueryExecutionFailed(
                    "List values should be handled by IN operator".to_string()
                ).into())
            }
        }
    }
}

impl Default for PostgresConnector {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Connector for PostgresConnector {
    async fn connect(&mut self, config: ConnectorInitConfig) -> NirvResult<()> {
        let host = config.connection_params.get("host")
            .unwrap_or(&"localhost".to_string()).clone();
        let port = config.connection_params.get("port")
            .unwrap_or(&"5432".to_string())
            .parse::<u16>()
            .map_err(|e| ConnectorError::ConnectionFailed(format!("Invalid port: {}", e)))?;
        let user = config.connection_params.get("user")
            .unwrap_or(&"postgres".to_string()).clone();
        let password = config.connection_params.get("password")
            .unwrap_or(&"".to_string()).clone();
        let dbname = config.connection_params.get("dbname")
            .unwrap_or(&"postgres".to_string()).clone();
        
        let max_size = config.max_connections.unwrap_or(10) as usize;
        let timeout = Duration::from_secs(config.timeout_seconds.unwrap_or(30));
        
        // Create deadpool configuration
        let mut pg_config = Config::new();
        pg_config.host = Some(host);
        pg_config.port = Some(port);
        pg_config.user = Some(user);
        pg_config.password = Some(password);
        pg_config.dbname = Some(dbname);
        pg_config.pool = Some(deadpool_postgres::PoolConfig::new(max_size));
        
        // Create connection pool
        let pool = pg_config.create_pool(Some(Runtime::Tokio1), NoTls)
            .map_err(|e| ConnectorError::ConnectionFailed(format!("Failed to create pool: {}", e)))?;
        
        // Test the connection
        let _client = tokio::time::timeout(timeout, pool.get()).await
            .map_err(|_| ConnectorError::Timeout("Connection timeout".to_string()))?
            .map_err(|e| ConnectorError::ConnectionFailed(format!("Failed to get connection: {}", e)))?;
        
        self.pool = Some(pool);
        self.connected = true;
        
        Ok(())
    }
    
    async fn execute_query(&self, query: ConnectorQuery) -> NirvResult<QueryResult> {
        if !self.connected {
            return Err(ConnectorError::ConnectionFailed("Not connected".to_string()).into());
        }
        
        let pool = self.pool.as_ref()
            .ok_or_else(|| ConnectorError::ConnectionFailed("No connection pool available".to_string()))?;
        
        let start_time = Instant::now();
        
        // Build SQL query
        let sql = self.build_sql_query(&query.query)?;
        
        // Get connection from pool
        let client = pool.get().await
            .map_err(|e| ConnectorError::ConnectionFailed(format!("Failed to get connection from pool: {}", e)))?;
        
        // Execute query
        let pg_rows = client.query(&sql, &[]).await
            .map_err(|e| ConnectorError::QueryExecutionFailed(format!("Query execution failed: {}", e)))?;
        
        // Convert results
        let mut columns = Vec::new();
        let mut rows = Vec::new();
        
        if let Some(first_row) = pg_rows.first() {
            // Extract column metadata
            for column in first_row.columns() {
                columns.push(ColumnMetadata {
                    name: column.name().to_string(),
                    data_type: self.pg_type_to_data_type(column.type_().oid()),
                    nullable: true, // PostgreSQL doesn't provide nullable info in query results
                });
            }
        }
        
        // Convert all rows
        for pg_row in &pg_rows {
            let row = self.convert_pg_row(pg_row)?;
            rows.push(row);
        }
        
        let execution_time = start_time.elapsed();
        
        Ok(QueryResult {
            columns,
            rows,
            affected_rows: Some(pg_rows.len() as u64),
            execution_time,
        })
    }
    
    async fn get_schema(&self, object_name: &str) -> NirvResult<Schema> {
        if !self.connected {
            return Err(ConnectorError::ConnectionFailed("Not connected".to_string()).into());
        }
        
        let pool = self.pool.as_ref()
            .ok_or_else(|| ConnectorError::ConnectionFailed("No connection pool available".to_string()))?;
        
        let client = pool.get().await
            .map_err(|e| ConnectorError::ConnectionFailed(format!("Failed to get connection from pool: {}", e)))?;
        
        // Parse table name (handle schema.table format)
        let (schema_name, table_name) = if object_name.contains('.') {
            let parts: Vec<&str> = object_name.splitn(2, '.').collect();
            (parts[0].to_string(), parts[1].to_string())
        } else {
            ("public".to_string(), object_name.to_string())
        };
        
        // Query column information
        let column_query = "
            SELECT 
                column_name,
                data_type,
                is_nullable,
                udt_name,
                ordinal_position
            FROM information_schema.columns 
            WHERE table_schema = $1 AND table_name = $2
            ORDER BY ordinal_position
        ";
        
        let column_rows = client.query(column_query, &[&schema_name, &table_name]).await
            .map_err(|e| ConnectorError::SchemaRetrievalFailed(format!("Failed to retrieve column info: {}", e)))?;
        
        if column_rows.is_empty() {
            return Err(ConnectorError::SchemaRetrievalFailed(
                format!("Table '{}' not found", object_name)
            ).into());
        }
        
        let mut columns = Vec::new();
        for row in &column_rows {
            let column_name: String = row.get("column_name");
            let data_type_str: String = row.get("data_type");
            let is_nullable: String = row.get("is_nullable");
            
            let data_type = match data_type_str.as_str() {
                "character varying" | "text" | "character" => DataType::Text,
                "integer" | "bigint" | "smallint" => DataType::Integer,
                "real" | "double precision" | "numeric" => DataType::Float,
                "boolean" => DataType::Boolean,
                "date" => DataType::Date,
                "timestamp without time zone" | "timestamp with time zone" => DataType::DateTime,
                "json" | "jsonb" => DataType::Json,
                "bytea" => DataType::Binary,
                _ => DataType::Text,
            };
            
            columns.push(ColumnMetadata {
                name: column_name,
                data_type,
                nullable: is_nullable == "YES",
            });
        }
        
        // Query primary key information
        let pk_query = "
            SELECT column_name
            FROM information_schema.key_column_usage
            WHERE table_schema = $1 AND table_name = $2
            AND constraint_name IN (
                SELECT constraint_name
                FROM information_schema.table_constraints
                WHERE table_schema = $1 AND table_name = $2
                AND constraint_type = 'PRIMARY KEY'
            )
            ORDER BY ordinal_position
        ";
        
        let pk_rows = client.query(pk_query, &[&schema_name, &table_name]).await
            .map_err(|e| ConnectorError::SchemaRetrievalFailed(format!("Failed to retrieve primary key info: {}", e)))?;
        
        let primary_key = if pk_rows.is_empty() {
            None
        } else {
            Some(pk_rows.iter().map(|row| row.get::<_, String>("column_name")).collect())
        };
        
        // Query index information
        let index_query = "
            SELECT 
                i.indexname,
                array_agg(a.attname ORDER BY a.attnum) as columns,
                i.indexdef LIKE '%UNIQUE%' as is_unique
            FROM pg_indexes i
            JOIN pg_class c ON c.relname = i.tablename
            JOIN pg_namespace n ON n.oid = c.relnamespace
            JOIN pg_index idx ON idx.indexrelid = (
                SELECT oid FROM pg_class WHERE relname = i.indexname
            )
            JOIN pg_attribute a ON a.attrelid = c.oid AND a.attnum = ANY(idx.indkey)
            WHERE n.nspname = $1 AND i.tablename = $2
            AND i.indexname NOT LIKE '%_pkey'
            GROUP BY i.indexname, i.indexdef
        ";
        
        let index_rows = client.query(index_query, &[&schema_name, &table_name]).await
            .unwrap_or_else(|_| Vec::new()); // Ignore errors for index retrieval
        
        let mut indexes = Vec::new();
        for row in &index_rows {
            let index_name: String = row.get("indexname");
            let columns_array: Vec<String> = row.get("columns");
            let is_unique: bool = row.get("is_unique");
            
            indexes.push(Index {
                name: index_name,
                columns: columns_array,
                unique: is_unique,
            });
        }
        
        Ok(Schema {
            name: object_name.to_string(),
            columns,
            primary_key,
            indexes,
        })
    }
    
    async fn disconnect(&mut self) -> NirvResult<()> {
        self.pool = None;
        self.connected = false;
        Ok(())
    }
    
    fn get_connector_type(&self) -> ConnectorType {
        ConnectorType::PostgreSQL
    }
    
    fn supports_transactions(&self) -> bool {
        true
    }
    
    fn is_connected(&self) -> bool {
        self.connected
    }
    
    fn get_capabilities(&self) -> ConnectorCapabilities {
        ConnectorCapabilities {
            supports_joins: true,
            supports_aggregations: true,
            supports_subqueries: true,
            supports_transactions: true,
            supports_schema_introspection: true,
            max_concurrent_queries: Some(10),
        }
    }
}