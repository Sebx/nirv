//! SQL Server integration example.
//!
//! Demonstrates connecting SqlServerConnector to a real SQL Server instance,
//! executing a SELECT query, and retrieving schema information.
//!
//! Required environment variables:
//!   SQLSERVER_HOST, SQLSERVER_PORT, SQLSERVER_DATABASE, SQLSERVER_USER, SQLSERVER_PASSWORD
//!
//! Usage:
//!   SQLSERVER_HOST=localhost SQLSERVER_PORT=1433 SQLSERVER_DATABASE=tempdb \
//!   SQLSERVER_USER=sa SQLSERVER_PASSWORD=YourStrong@Passw0rd \
//!   cargo run --example sqlserver_integration

use std::env;
use std::collections::HashMap;
use std::time::Instant;
use nirv_engine::connectors::{SqlServerConnector, Connector, ConnectorInitConfig};
use nirv_engine::utils::types::{ConnectorType, ConnectorQuery, InternalQuery, QueryOperation, DataSource};

fn require_env(key: &str) -> String {
    env::var(key).unwrap_or_else(|_| {
        eprintln!("Error: required environment variable '{}' is not set.", key);
        eprintln!("Set the following variables before running:");
        eprintln!("  SQLSERVER_HOST  - hostname or IP of the SQL Server instance");
        eprintln!("  SQLSERVER_PORT  - TCP port (default 1433)");
        eprintln!("  SQLSERVER_DATABASE - database to connect to (e.g. tempdb)");
        eprintln!("  SQLSERVER_USER  - SQL Server login name");
        eprintln!("  SQLSERVER_PASSWORD - SQL Server login password");
        std::process::exit(1);
    })
}

#[tokio::main]
async fn main() {
    let host = require_env("SQLSERVER_HOST");
    let port = require_env("SQLSERVER_PORT");
    let database = require_env("SQLSERVER_DATABASE");
    let user = require_env("SQLSERVER_USER");
    let password = require_env("SQLSERVER_PASSWORD");

    println!("Connecting to SQL Server at {}:{}...", host, port);

    let config = ConnectorInitConfig::new()
        .with_param("server", &host)
        .with_param("port", &port)
        .with_param("database", &database)
        .with_param("username", &user)
        .with_param("password", &password)
        .with_param("trust_cert", "true")
        .with_timeout(30);

    let mut connector = SqlServerConnector::new();

    if let Err(e) = connector.connect(config).await {
        eprintln!("Error: Failed to connect to SQL Server: {}", e);
        std::process::exit(1);
    }

    println!("Connected successfully.\n");

    // --- Execute query ---
    let mut internal_query = InternalQuery::new(QueryOperation::Select);
    internal_query.sources.push(DataSource {
        object_type: "sqlserver".to_string(),
        identifier: "INFORMATION_SCHEMA.TABLES".to_string(),
        alias: None,
    });

    let connector_query = ConnectorQuery {
        connector_type: ConnectorType::SqlServer,
        query: internal_query,
        connection_params: HashMap::new(),
    };

    println!("Executing: SELECT * FROM INFORMATION_SCHEMA.TABLES");
    let query_start = Instant::now();
    match connector.execute_query(connector_query).await {
        Ok(result) => {
            let col_names: Vec<&str> = result.columns.iter().map(|c| c.name.as_str()).collect();
            println!("Columns: {}", col_names.join(", "));
            println!("Showing first 5 rows:");
            for (i, row) in result.rows.iter().take(5).enumerate() {
                println!("  Row {}: {:?}", i + 1, row.values);
            }
            println!("Total rows returned: {}", result.rows.len());
            println!("Execution time: {}ms\n", query_start.elapsed().as_millis());
        }
        Err(e) => {
            eprintln!("Error executing query: {}", e);
            let _ = connector.disconnect().await;
            std::process::exit(1);
        }
    }

    // --- Get schema ---
    println!("Retrieving schema for INFORMATION_SCHEMA.TABLES...");
    let schema_start = Instant::now();
    match connector.get_schema("INFORMATION_SCHEMA.TABLES").await {
        Ok(schema) => {
            println!("Schema: {}", schema.name);
            println!("Columns ({}):", schema.columns.len());
            for col in &schema.columns {
                println!(
                    "  - {} ({:?}, nullable: {})",
                    col.name, col.data_type, col.nullable
                );
            }
            match &schema.primary_key {
                Some(pk) => println!("Primary key: {}", pk.join(", ")),
                None => println!("Primary key: none"),
            }
            println!("Schema retrieval time: {}ms", schema_start.elapsed().as_millis());
        }
        Err(e) => {
            eprintln!("Error retrieving schema: {}", e);
            let _ = connector.disconnect().await;
            std::process::exit(1);
        }
    }

    let _ = connector.disconnect().await;
    println!("\nDone.");
}
