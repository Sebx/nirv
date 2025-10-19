use nirv_engine::connectors::{
    SqlServerConnector, Connector, ConnectorInitConfig
};
use nirv_engine::utils::types::{
    ConnectorType, InternalQuery, QueryOperation,
    DataSource, Predicate, PredicateOperator, PredicateValue, DataType
};

#[tokio::test]
async fn test_sqlserver_connector_creation() {
    let connector = SqlServerConnector::new();
    
    assert_eq!(connector.get_connector_type(), ConnectorType::SqlServer);
    assert!(!connector.is_connected());
    assert!(connector.supports_transactions());
}

#[tokio::test]
async fn test_sqlserver_connector_capabilities() {
    let connector = SqlServerConnector::new();
    let capabilities = connector.get_capabilities();
    
    assert!(capabilities.supports_joins);
    assert!(capabilities.supports_aggregations);
    assert!(capabilities.supports_subqueries);
    assert!(capabilities.supports_transactions);
    assert!(capabilities.supports_schema_introspection);
    assert_eq!(capabilities.max_concurrent_queries, Some(20));
}

#[tokio::test]
async fn test_sqlserver_connector_init_config() {
    let config = ConnectorInitConfig::new()
        .with_param("server", "localhost")
        .with_param("port", "1433")
        .with_param("database", "testdb")
        .with_param("username", "sa")
        .with_param("password", "password123")
        .with_param("trust_cert", "true")
        .with_timeout(30)
        .with_max_connections(10);
    
    assert_eq!(config.connection_params.get("server"), Some(&"localhost".to_string()));
    assert_eq!(config.connection_params.get("port"), Some(&"1433".to_string()));
    assert_eq!(config.connection_params.get("database"), Some(&"testdb".to_string()));
    assert_eq!(config.connection_params.get("username"), Some(&"sa".to_string()));
    assert_eq!(config.connection_params.get("password"), Some(&"password123".to_string()));
    assert_eq!(config.connection_params.get("trust_cert"), Some(&"true".to_string()));
    assert_eq!(config.timeout_seconds, Some(30));
    assert_eq!(config.max_connections, Some(10));
}

#[tokio::test]
async fn test_sqlserver_connector_connection_string_building() {
    let connector = SqlServerConnector::new();
    
    let config = ConnectorInitConfig::new()
        .with_param("server", "localhost")
        .with_param("port", "1433")
        .with_param("database", "testdb")
        .with_param("username", "sa")
        .with_param("password", "password123")
        .with_param("trust_cert", "true");
    
    // This should not panic and should build a valid connection string internally
    let connection_string = connector.build_connection_string(&config).unwrap();
    
    assert!(connection_string.contains("server=localhost"));
    assert!(connection_string.contains("database=testdb"));
    assert!(connection_string.contains("user=sa"));
    assert!(connection_string.contains("password=password123"));
    assert!(connection_string.contains("TrustServerCertificate=true"));
}

#[tokio::test]
async fn test_sqlserver_connector_sql_query_building() {
    let connector = SqlServerConnector::new();
    
    // Test SELECT query building
    let mut internal_query = InternalQuery::new(QueryOperation::Select);
    internal_query.sources.push(DataSource {
        object_type: "sqlserver".to_string(),
        identifier: "users".to_string(),
        alias: Some("u".to_string()),
    });
    
    internal_query.projections.push(nirv_engine::utils::types::Column {
        name: "id".to_string(),
        alias: None,
        source: Some("u".to_string()),
    });
    
    internal_query.projections.push(nirv_engine::utils::types::Column {
        name: "name".to_string(),
        alias: Some("user_name".to_string()),
        source: Some("u".to_string()),
    });
    
    internal_query.predicates.push(Predicate {
        column: "age".to_string(),
        operator: PredicateOperator::GreaterThan,
        value: PredicateValue::Integer(18),
    });
    
    internal_query.limit = Some(100);
    
    let sql = connector.build_sql_query(&internal_query).unwrap();
    
    assert!(sql.contains("SELECT"));
    assert!(sql.contains("id"));
    assert!(sql.contains("name AS user_name"));
    assert!(sql.contains("FROM users AS u"));
    assert!(sql.contains("WHERE"));
    assert!(sql.contains("age > 18"));
    assert!(sql.contains("TOP 100") || sql.contains("OFFSET 0 ROWS FETCH NEXT 100 ROWS ONLY"));
}

#[tokio::test]
async fn test_sqlserver_connector_predicate_building() {
    let connector = SqlServerConnector::new();
    
    // Test different predicate operators
    let predicates = vec![
        Predicate {
            column: "name".to_string(),
            operator: PredicateOperator::Equal,
            value: PredicateValue::String("John".to_string()),
        },
        Predicate {
            column: "age".to_string(),
            operator: PredicateOperator::GreaterThanOrEqual,
            value: PredicateValue::Integer(21),
        },
        Predicate {
            column: "email".to_string(),
            operator: PredicateOperator::Like,
            value: PredicateValue::String("%@example.com".to_string()),
        },
        Predicate {
            column: "status".to_string(),
            operator: PredicateOperator::In,
            value: PredicateValue::List(vec![
                PredicateValue::String("active".to_string()),
                PredicateValue::String("pending".to_string()),
            ]),
        },
        Predicate {
            column: "deleted_at".to_string(),
            operator: PredicateOperator::IsNull,
            value: PredicateValue::Null,
        },
    ];
    
    for predicate in &predicates {
        let sql = connector.build_predicate_sql(predicate).unwrap();
        
        match predicate.operator {
            PredicateOperator::Equal => {
                assert!(sql.contains("name = 'John'"));
            },
            PredicateOperator::GreaterThanOrEqual => {
                assert!(sql.contains("age >= 21"));
            },
            PredicateOperator::Like => {
                assert!(sql.contains("email LIKE '%@example.com'"));
            },
            PredicateOperator::In => {
                assert!(sql.contains("status IN ('active', 'pending')"));
            },
            PredicateOperator::IsNull => {
                assert!(sql.contains("deleted_at IS NULL"));
            },
            _ => {}
        }
    }
}#[tokio
::test]
async fn test_sqlserver_connector_data_type_conversion() {
    let connector = SqlServerConnector::new();
    
    // Test SQL Server type to internal DataType conversion
    assert_eq!(connector.sqlserver_type_to_data_type("varchar"), DataType::Text);
    assert_eq!(connector.sqlserver_type_to_data_type("nvarchar"), DataType::Text);
    assert_eq!(connector.sqlserver_type_to_data_type("char"), DataType::Text);
    assert_eq!(connector.sqlserver_type_to_data_type("nchar"), DataType::Text);
    assert_eq!(connector.sqlserver_type_to_data_type("text"), DataType::Text);
    assert_eq!(connector.sqlserver_type_to_data_type("ntext"), DataType::Text);
    
    assert_eq!(connector.sqlserver_type_to_data_type("int"), DataType::Integer);
    assert_eq!(connector.sqlserver_type_to_data_type("bigint"), DataType::Integer);
    assert_eq!(connector.sqlserver_type_to_data_type("smallint"), DataType::Integer);
    assert_eq!(connector.sqlserver_type_to_data_type("tinyint"), DataType::Integer);
    
    assert_eq!(connector.sqlserver_type_to_data_type("float"), DataType::Float);
    assert_eq!(connector.sqlserver_type_to_data_type("real"), DataType::Float);
    assert_eq!(connector.sqlserver_type_to_data_type("decimal"), DataType::Float);
    assert_eq!(connector.sqlserver_type_to_data_type("numeric"), DataType::Float);
    assert_eq!(connector.sqlserver_type_to_data_type("money"), DataType::Float);
    
    assert_eq!(connector.sqlserver_type_to_data_type("bit"), DataType::Boolean);
    
    assert_eq!(connector.sqlserver_type_to_data_type("date"), DataType::Date);
    assert_eq!(connector.sqlserver_type_to_data_type("datetime"), DataType::DateTime);
    assert_eq!(connector.sqlserver_type_to_data_type("datetime2"), DataType::DateTime);
    assert_eq!(connector.sqlserver_type_to_data_type("datetimeoffset"), DataType::DateTime);
    assert_eq!(connector.sqlserver_type_to_data_type("smalldatetime"), DataType::DateTime);
    assert_eq!(connector.sqlserver_type_to_data_type("time"), DataType::DateTime);
    
    assert_eq!(connector.sqlserver_type_to_data_type("varbinary"), DataType::Binary);
    assert_eq!(connector.sqlserver_type_to_data_type("binary"), DataType::Binary);
    assert_eq!(connector.sqlserver_type_to_data_type("image"), DataType::Binary);
    
    // Test unknown type defaults to Text
    assert_eq!(connector.sqlserver_type_to_data_type("unknown_type"), DataType::Text);
}

#[tokio::test]
async fn test_sqlserver_connector_value_formatting() {
    let connector = SqlServerConnector::new();
    
    // Test predicate value formatting for SQL
    assert_eq!(
        connector.format_predicate_value(&PredicateValue::String("test".to_string())).unwrap(),
        "'test'"
    );
    
    assert_eq!(
        connector.format_predicate_value(&PredicateValue::String("test's value".to_string())).unwrap(),
        "'test''s value'"
    );
    
    assert_eq!(
        connector.format_predicate_value(&PredicateValue::Integer(42)).unwrap(),
        "42"
    );
    
    assert_eq!(
        connector.format_predicate_value(&PredicateValue::Number(3.14)).unwrap(),
        "3.14"
    );
    
    assert_eq!(
        connector.format_predicate_value(&PredicateValue::Boolean(true)).unwrap(),
        "1"
    );
    
    assert_eq!(
        connector.format_predicate_value(&PredicateValue::Boolean(false)).unwrap(),
        "0"
    );
    
    assert_eq!(
        connector.format_predicate_value(&PredicateValue::Null).unwrap(),
        "NULL"
    );
}

#[tokio::test]
async fn test_sqlserver_connector_complex_query_building() {
    let connector = SqlServerConnector::new();
    
    // Test complex query with JOINs and ORDER BY
    let mut internal_query = InternalQuery::new(QueryOperation::Select);
    
    // Add sources
    internal_query.sources.push(DataSource {
        object_type: "sqlserver".to_string(),
        identifier: "users".to_string(),
        alias: Some("u".to_string()),
    });
    
    // Add projections
    internal_query.projections.push(nirv_engine::utils::types::Column {
        name: "u.id".to_string(),
        alias: None,
        source: None,
    });
    
    internal_query.projections.push(nirv_engine::utils::types::Column {
        name: "u.name".to_string(),
        alias: Some("user_name".to_string()),
        source: None,
    });
    
    // Add predicates
    internal_query.predicates.push(Predicate {
        column: "u.active".to_string(),
        operator: PredicateOperator::Equal,
        value: PredicateValue::Boolean(true),
    });
    
    // Add ordering
    internal_query.ordering = Some(nirv_engine::utils::types::OrderBy {
        columns: vec![
            nirv_engine::utils::types::OrderColumn {
                column: "u.name".to_string(),
                direction: nirv_engine::utils::types::OrderDirection::Ascending,
            },
            nirv_engine::utils::types::OrderColumn {
                column: "u.id".to_string(),
                direction: nirv_engine::utils::types::OrderDirection::Descending,
            },
        ],
    });
    
    internal_query.limit = Some(50);
    
    let sql = connector.build_sql_query(&internal_query).unwrap();
    
    assert!(sql.contains("SELECT"));
    assert!(sql.contains("u.id"));
    assert!(sql.contains("u.name AS user_name"));
    assert!(sql.contains("FROM users AS u"));
    assert!(sql.contains("WHERE u.active = 1"));
    assert!(sql.contains("ORDER BY u.name ASC, u.id DESC"));
    assert!(sql.contains("TOP 50") || sql.contains("OFFSET 0 ROWS FETCH NEXT 50 ROWS ONLY"));
}

#[tokio::test]
async fn test_sqlserver_connector_error_handling() {
    let connector = SqlServerConnector::new();
    
    // Test invalid connection config
    let _invalid_config = ConnectorInitConfig::new();
    // Missing required parameters should cause an error
    
    // Test invalid query building
    let empty_query = InternalQuery::new(QueryOperation::Select);
    let result = connector.build_sql_query(&empty_query);
    assert!(result.is_err());
    
    // Test unsupported operations
    let insert_query = InternalQuery::new(QueryOperation::Insert);
    let result = connector.build_sql_query(&insert_query);
    assert!(result.is_err());
}