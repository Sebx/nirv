use async_trait::async_trait;
use std::fmt;
use std::time::{Duration, Instant};
use tiberius::{Client, Config, AuthMethod, EncryptionLevel};
use tokio::net::TcpStream;
use tokio::sync::Mutex;
use tokio_util::compat::{TokioAsyncWriteCompatExt, Compat};
use futures_util::TryStreamExt;

use crate::connectors::{Connector, ConnectorInitConfig, ConnectorCapabilities};
use crate::utils::{
    types::{
        ConnectorType, ConnectorQuery, QueryResult, Schema, ColumnMetadata, DataType,
        Row, Value, QueryOperation, PredicateOperator
    },
    error::{ConnectorError, NirvResult},
};

/// SQL Server connector using tiberius
pub struct SqlServerConnector {
    client: Mutex<Option<Client<Compat<TcpStream>>>>,
    connected: bool,
    connection_config: Option<Config>,
}

impl fmt::Debug for SqlServerConnector {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SqlServerConnector")
            .field("connected", &self.connected)
            .finish()
    }
}

impl SqlServerConnector {
    /// Create a new SQL Server connector
    pub fn new() -> Self {
        Self {
            client: Mutex::new(None),
            connected: false,
            connection_config: None,
        }
    }
    
    /// Build connection string from configuration parameters
    pub fn build_connection_string(&self, config: &ConnectorInitConfig) -> NirvResult<String> {
        let server = config.connection_params.get("server")
            .ok_or_else(|| ConnectorError::ConnectionFailed(
                "server parameter is required".to_string()
            ))?;
        
        let default_port = "1433".to_string();
        let port = config.connection_params.get("port")
            .unwrap_or(&default_port);
        
        let database = config.connection_params.get("database")
            .ok_or_else(|| ConnectorError::ConnectionFailed(
                "database parameter is required".to_string()
            ))?;
        
        let username = config.connection_params.get("username")
            .ok_or_else(|| ConnectorError::ConnectionFailed(
                "username parameter is required".to_string()
            ))?;
        
        let password = config.connection_params.get("password")
            .ok_or_else(|| ConnectorError::ConnectionFailed(
                "password parameter is required".to_string()
            ))?;
        
        let trust_cert = config.connection_params.get("trust_cert")
            .map(|s| s.parse::<bool>().unwrap_or(false))
            .unwrap_or(false);
        
        let mut connection_string = format!(
            "server={},{};database={};user={};password={}",
            server, port, database, username, password
        );
        
        if trust_cert {
            connection_string.push_str(";TrustServerCertificate=true");
        }
        
        Ok(connection_string)
    }
    
    /// Build SQL query from internal query representation
    pub fn build_sql_query(&self, query: &crate::utils::types::InternalQuery) -> NirvResult<String> {
        match query.operation {
            QueryOperation::Select => {
                let mut sql = String::from("SELECT ");
                
                // Handle LIMIT with TOP clause (SQL Server style)
                if let Some(limit) = query.limit {
                    sql.push_str(&format!("TOP {} ", limit));
                }
                
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
                
                Ok(sql)
            }
            _ => Err(ConnectorError::UnsupportedOperation(
                format!("Operation {:?} not supported by SQL Server connector", query.operation)
            ).into()),
        }
    }
    
    /// Build SQL for a single predicate
    pub fn build_predicate_sql(&self, predicate: &crate::utils::types::Predicate) -> NirvResult<String> {
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
    pub fn format_predicate_value(&self, value: &crate::utils::types::PredicateValue) -> NirvResult<String> {
        match value {
            crate::utils::types::PredicateValue::String(s) => {
                // Escape single quotes by doubling them
                Ok(format!("'{}'", s.replace('\'', "''")))
            },
            crate::utils::types::PredicateValue::Number(n) => Ok(n.to_string()),
            crate::utils::types::PredicateValue::Integer(i) => Ok(i.to_string()),
            crate::utils::types::PredicateValue::Boolean(b) => {
                // SQL Server uses 1/0 for boolean values
                Ok(if *b { "1".to_string() } else { "0".to_string() })
            },
            crate::utils::types::PredicateValue::Null => Ok("NULL".to_string()),
            crate::utils::types::PredicateValue::List(_) => {
                Err(ConnectorError::QueryExecutionFailed(
                    "List values should be handled by IN operator".to_string()
                ).into())
            }
        }
    }
    
    /// Convert SQL Server type to internal DataType
    pub fn sqlserver_type_to_data_type(&self, sql_type: &str) -> DataType {
        match sql_type.to_lowercase().as_str() {
            // Text types
            "varchar" | "nvarchar" | "char" | "nchar" | "text" | "ntext" => DataType::Text,
            
            // Integer types
            "int" | "bigint" | "smallint" | "tinyint" => DataType::Integer,
            
            // Float types
            "float" | "real" | "decimal" | "numeric" | "money" | "smallmoney" => DataType::Float,
            
            // Boolean type
            "bit" => DataType::Boolean,
            
            // Date types
            "date" => DataType::Date,
            "datetime" | "datetime2" | "datetimeoffset" | "smalldatetime" | "time" => DataType::DateTime,
            
            // Binary types
            "varbinary" | "binary" | "image" => DataType::Binary,
            
            // JSON (SQL Server 2016+)
            "json" => DataType::Json,
            
            // Default to text for unknown types
            _ => DataType::Text,
        }
    }
    
    /// Sanitize a table name for safe embedding in SQL strings.
    /// Only allows alphanumeric characters, underscores, hyphens, and dots.
    pub fn sanitize_table_name(name: &str) -> NirvResult<String> {
        let valid = name.chars().all(|c| c.is_alphanumeric() || c == '_' || c == '-' || c == '.');
        if valid && !name.is_empty() {
            Ok(name.to_string())
        } else {
            Err(crate::utils::error::ConnectorError::SchemaRetrievalFailed(
                format!("Invalid table name: '{}'", name)
            ).into())
        }
    }

    /// Convert tiberius row value to internal Value representation
    fn convert_row_value(&self, row: &tiberius::Row, index: usize) -> NirvResult<Value> {
        // Try different types in order of likelihood
        if let Ok(Some(val)) = row.try_get::<&str, usize>(index) {
            return Ok(Value::Text(val.to_string()));
        }
        if let Ok(Some(val)) = row.try_get::<i32, usize>(index) {
            return Ok(Value::Integer(val as i64));
        }
        if let Ok(Some(val)) = row.try_get::<i64, usize>(index) {
            return Ok(Value::Integer(val));
        }
        if let Ok(Some(val)) = row.try_get::<f64, usize>(index) {
            return Ok(Value::Float(val));
        }
        if let Ok(Some(val)) = row.try_get::<f32, usize>(index) {
            return Ok(Value::Float(val as f64));
        }
        if let Ok(Some(val)) = row.try_get::<bool, usize>(index) {
            return Ok(Value::Boolean(val));
        }
        if let Ok(Some(val)) = row.try_get::<&[u8], usize>(index) {
            return Ok(Value::Binary(val.to_vec()));
        }
        
        // If all else fails, return null
        Ok(Value::Null)
    }
}

impl Default for SqlServerConnector {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Connector for SqlServerConnector {
    async fn connect(&mut self, config: ConnectorInitConfig) -> NirvResult<()> {
        let server = config.connection_params.get("server")
            .ok_or_else(|| ConnectorError::ConnectionFailed(
                "server parameter is required".to_string()
            ))?;
        
        let port = config.connection_params.get("port")
            .unwrap_or(&"1433".to_string())
            .parse::<u16>()
            .map_err(|e| ConnectorError::ConnectionFailed(format!("Invalid port: {}", e)))?;
        
        let database = config.connection_params.get("database")
            .ok_or_else(|| ConnectorError::ConnectionFailed(
                "database parameter is required".to_string()
            ))?;
        
        let username = config.connection_params.get("username")
            .ok_or_else(|| ConnectorError::ConnectionFailed(
                "username parameter is required".to_string()
            ))?;
        
        let password = config.connection_params.get("password")
            .ok_or_else(|| ConnectorError::ConnectionFailed(
                "password parameter is required".to_string()
            ))?;
        
        let trust_cert = config.connection_params.get("trust_cert")
            .map(|s| s.parse::<bool>().unwrap_or(false))
            .unwrap_or(false);
        
        // Create tiberius configuration
        let mut tiberius_config = Config::new();
        tiberius_config.host(server);
        tiberius_config.port(port);
        tiberius_config.database(database);
        tiberius_config.authentication(AuthMethod::sql_server(username, password));
        
        if trust_cert {
            tiberius_config.encryption(EncryptionLevel::NotSupported);
        }
        
        let timeout = Duration::from_secs(config.timeout_seconds.unwrap_or(30));
        
        // Connect to SQL Server
        let tcp = tokio::time::timeout(timeout, TcpStream::connect(tiberius_config.get_addr())).await
            .map_err(|_| ConnectorError::Timeout(format!(
                "Connection timeout after {}s when connecting to SQL Server at {}:{}",
                timeout.as_secs(), server, port
            )))?
            .map_err(|e| ConnectorError::ConnectionFailed(format!(
                "Failed to establish TCP connection to SQL Server at {}:{} - Error: {} (Check if server is running and port is accessible)",
                server, port, e
            )))?;
        
        let client = Client::connect(tiberius_config.clone(), tcp.compat_write()).await
            .map_err(|e| ConnectorError::ConnectionFailed(format!(
                "Failed to authenticate with SQL Server at {}:{} using username '{}' - Error: {} (Check credentials and server configuration)",
                server, port, username, e
            )))?;
        
        *self.client.lock().await = Some(client);
        self.connection_config = Some(tiberius_config);
        self.connected = true;
        
        Ok(())
    }
    
    async fn execute_query(&self, query: ConnectorQuery) -> NirvResult<QueryResult> {
        if !self.connected {
            return Err(ConnectorError::ConnectionFailed("Not connected to SQL Server".to_string()).into());
        }

        let start_time = Instant::now();
        let sql = self.build_sql_query(&query.query)?;

        let mut client_guard = self.client.lock().await;
        let client = client_guard.as_mut()
            .ok_or_else(|| ConnectorError::ConnectionFailed("No active SQL Server client".to_string()))?;

        let mut stream = client.simple_query(sql.as_str()).await
            .map_err(|e| ConnectorError::QueryExecutionFailed(
                format!("SQL Server query execution failed: {}", e)
            ))?;

        let mut columns: Vec<ColumnMetadata> = Vec::new();
        let mut rows: Vec<Row> = Vec::new();
        let mut columns_built = false;

        while let Some(item) = stream.try_next().await
            .map_err(|e| ConnectorError::QueryExecutionFailed(format!("Error reading query stream: {}", e)))?
        {
            match item {
                tiberius::QueryItem::Metadata(meta) => {
                    if !columns_built {
                        for col in meta.columns() {
                            columns.push(ColumnMetadata {
                                name: col.name().to_string(),
                                data_type: self.sqlserver_type_to_data_type(
                                    &format!("{:?}", col.column_type())
                                ),
                                nullable: true,
                            });
                        }
                        columns_built = true;
                    }
                }
                tiberius::QueryItem::Row(row) => {
                    let values: Vec<Value> = (0..row.len())
                        .map(|i| self.convert_row_value(&row, i).unwrap_or(Value::Null))
                        .collect();
                    rows.push(Row::new(values));
                }
            }
        }

        Ok(QueryResult {
            columns,
            rows,
            affected_rows: None,
            execution_time: start_time.elapsed(),
        })
    }
    
    async fn get_schema(&self, object_name: &str) -> NirvResult<Schema> {
        if !self.connected {
            return Err(ConnectorError::ConnectionFailed("Not connected to SQL Server".to_string()).into());
        }

        let _sanitized = Self::sanitize_table_name(object_name)?;

        // Parse schema and table name from dot notation
        let (schema_name, table_name) = if object_name.contains('.') {
            let parts: Vec<&str> = object_name.splitn(2, '.').collect();
            (parts[0].to_string(), parts[1].to_string())
        } else {
            ("%".to_string(), object_name.to_string())
        };

        let mut client_guard = self.client.lock().await;
        let client = client_guard.as_mut()
            .ok_or_else(|| ConnectorError::ConnectionFailed("No active SQL Server client".to_string()))?;

        // Query column information from INFORMATION_SCHEMA
        let col_sql = format!(
            "SELECT COLUMN_NAME, DATA_TYPE, IS_NULLABLE, ORDINAL_POSITION \
             FROM INFORMATION_SCHEMA.COLUMNS \
             WHERE TABLE_NAME = '{}' AND TABLE_SCHEMA LIKE '{}' \
             ORDER BY ORDINAL_POSITION",
            table_name, schema_name
        );

        let mut columns: Vec<ColumnMetadata> = Vec::new();

        {
            let mut col_stream = client.simple_query(col_sql.as_str()).await
                .map_err(|e| ConnectorError::SchemaRetrievalFailed(
                    format!("Failed to query INFORMATION_SCHEMA.COLUMNS: {}", e)
                ))?;

            while let Some(item) = col_stream.try_next().await
                .map_err(|e| ConnectorError::SchemaRetrievalFailed(format!("Error reading column stream: {}", e)))?
            {
                if let tiberius::QueryItem::Row(row) = item {
                    let col_name: &str = row.try_get(0)
                        .map_err(|e| ConnectorError::SchemaRetrievalFailed(format!("Failed to get COLUMN_NAME: {}", e)))?
                        .unwrap_or("unknown");
                    let data_type_str: &str = row.try_get(1)
                        .map_err(|e| ConnectorError::SchemaRetrievalFailed(format!("Failed to get DATA_TYPE: {}", e)))?
                        .unwrap_or("varchar");
                    let is_nullable: &str = row.try_get(2)
                        .map_err(|e| ConnectorError::SchemaRetrievalFailed(format!("Failed to get IS_NULLABLE: {}", e)))?
                        .unwrap_or("YES");

                    columns.push(ColumnMetadata {
                        name: col_name.to_string(),
                        data_type: self.sqlserver_type_to_data_type(data_type_str),
                        nullable: is_nullable == "YES",
                    });
                }
            }
        } // col_stream is dropped here, releasing the mutable borrow on client

        if columns.is_empty() {
            return Err(ConnectorError::SchemaRetrievalFailed(
                format!("Table '{}' not found in INFORMATION_SCHEMA", object_name)
            ).into());
        }

        // Query primary key information
        let pk_sql = format!(
            "SELECT kcu.COLUMN_NAME \
             FROM INFORMATION_SCHEMA.KEY_COLUMN_USAGE kcu \
             JOIN INFORMATION_SCHEMA.TABLE_CONSTRAINTS tc \
               ON kcu.CONSTRAINT_NAME = tc.CONSTRAINT_NAME \
               AND kcu.TABLE_NAME = tc.TABLE_NAME \
             WHERE kcu.TABLE_NAME = '{}' \
               AND tc.CONSTRAINT_TYPE = 'PRIMARY KEY' \
             ORDER BY kcu.ORDINAL_POSITION",
            table_name
        );

        let mut pk_columns: Vec<String> = Vec::new();

        {
            let mut pk_stream = client.simple_query(pk_sql.as_str()).await
                .map_err(|e| ConnectorError::SchemaRetrievalFailed(
                    format!("Failed to query primary keys: {}", e)
                ))?;

            while let Some(item) = pk_stream.try_next().await
                .map_err(|e| ConnectorError::SchemaRetrievalFailed(format!("Error reading PK stream: {}", e)))?
            {
                if let tiberius::QueryItem::Row(row) = item {
                    if let Ok(Some(col_name)) = row.try_get::<&str, usize>(0) {
                        pk_columns.push(col_name.to_string());
                    }
                }
            }
        } // pk_stream is dropped here

        let primary_key = if pk_columns.is_empty() { None } else { Some(pk_columns) };

        Ok(Schema {
            name: object_name.to_string(),
            columns,
            primary_key,
            indexes: Vec::new(),
        })
    }
    
    async fn disconnect(&mut self) -> NirvResult<()> {
        *self.client.lock().await = None;
        self.connected = false;
        self.connection_config = None;
        Ok(())
    }
    
    fn get_connector_type(&self) -> ConnectorType {
        ConnectorType::SqlServer
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
            max_concurrent_queries: Some(20),
        }
    }
}