//! PostgreSQL integration example.
//!
//! Demonstrates connecting PostgresConnector to a real PostgreSQL instance,
//! executing a SELECT query, and retrieving schema information.
//!
//! Required environment variables:
//!   POSTGRES_HOST, POSTGRES_PORT, POSTGRES_DB, POSTGRES_USER, POSTGRES_PASSWORD
//!
//! Usage:
//!   POSTGRES_HOST=localhost POSTGRES_PORT=5432 POSTGRES_DB=testdb \
//!   POSTGRES_USER=postgres POSTGRES_PASSWORD=postgres \
//!   cargo run --example postgres_integration

use std::env;
use std::collections::HashMap;

use nirv_engine::connectors::{PostgresConnector, Connector, ConnectorInitConfig};
use nirv_engine::utils::types::{ConnectorType, ConnectorQuery, InternalQuery, QueryOperation, DataSource};

fn require_env(key: &str) -> String {
    env::var(key).unwrap_or_else(|_| {
        eprintln!("Error: required environment variable '{}' is not set.", key);
        eprintln!("Set the following variables before running:");
        eprintln!("  POSTGRES_HOST, POSTGRES_PORT, POSTGRES_DB, POSTGRES_USER, POSTGRES_PASSWORD");
        std::process::exit(1);
    })
}

#[tokio::main]
async fn main() {
    let host = require_env("POSTGRES_HOST");
    let port = require_env("POSTGRES_PORT");
    let dbname = require_env("POSTGRES_DB");
    let user = require_env("POSTGRES_USER");
    let password = require_env("POSTGRES_PASSWORD");

    println!("Connecting to PostgreSQL at {}:{}...", host, port);

    let config = ConnectorInitConfig::new()
        .with_param("host", &host)
        .with_param("port", &port)
        .with_param("dbname", &dbname)
        .with_param("user", &user)
        .with_param("password", &password)
        .with_timeout(30);

    let mut connector = PostgresConnector::new();

    if let Err(e) = connector.connect(config).await {
        eprintln!("Error: Failed to connect to PostgreSQL: {}", e);
        std::process::exit(1);
    }

    println!("Connected successfully.\n");

    // --- Execute query ---
    let mut internal_query = InternalQuery::new(QueryOperation::Select);
    internal_query.sources.push(DataSource {
        object_type: "postgres".to_string(),
        identifier: "information_schema.tables".to_string(),
        alias: None,
    });
    internal_query.limit = Some(10);

    let connector_query = ConnectorQuery {
        connector_type: ConnectorType::PostgreSQL,
        query: internal_query,
        connection_params: HashMap::new(),
    };

    println!("Executing: SELECT * FROM information_schema.tables LIMIT 10");
    match connector.execute_query(connector_query).await {
        Ok(result) => {
            let col_names: Vec<&str> = result.columns.iter().map(|c| c.name.as_str()).collect();
            println!("Columns: {}", col_names.join(", "));
            println!("Rows ({}):", result.rows.len());
            for (i, row) in result.rows.iter().enumerate() {
                println!("  Row {}: {:?}", i + 1, row.values);
            }
            println!("Execution time: {}ms\n", result.execution_time.as_millis());
        }
        Err(e) => {
            eprintln!("Error executing query: {}", e);
            let _ = connector.disconnect().await;
            std::process::exit(1);
        }
    }

    // --- Get schema ---
    println!("Retrieving schema for information_schema.tables...");
    match connector.get_schema("information_schema.tables").await {
        Ok(schema) => {
            println!("Schema: {}", schema.name);
            println!("Columns ({}):", schema.columns.len());
            for col in &schema.columns {
                println!("  - {} ({:?}, nullable: {})", col.name, col.data_type, col.nullable);
            }
            if let Some(pk) = &schema.primary_key {
                println!("Primary key: {}", pk.join(", "));
            } else {
                println!("Primary key: none");
            }
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
