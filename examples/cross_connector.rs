//! Cross-connector example.
//!
//! Demonstrates querying data from both SQL Server and PostgreSQL in a single workflow,
//! then printing a combined summary table.
//!
//! Required environment variables:
//!   SQLSERVER_HOST, SQLSERVER_PORT, SQLSERVER_DATABASE, SQLSERVER_USER, SQLSERVER_PASSWORD
//!   POSTGRES_HOST, POSTGRES_PORT, POSTGRES_DB, POSTGRES_USER, POSTGRES_PASSWORD
//!
//! Usage:
//!   SQLSERVER_HOST=localhost SQLSERVER_PORT=1433 SQLSERVER_DATABASE=tempdb \
//!   SQLSERVER_USER=sa SQLSERVER_PASSWORD=YourStrong@Passw0rd \
//!   POSTGRES_HOST=localhost POSTGRES_PORT=5432 POSTGRES_DB=testdb \
//!   POSTGRES_USER=postgres POSTGRES_PASSWORD=postgres \
//!   cargo run --example cross_connector

use std::collections::HashMap;
use std::env;
use std::time::Instant;

use nirv_engine::connectors::{Connector, ConnectorInitConfig, PostgresConnector, SqlServerConnector};
use nirv_engine::utils::types::{
    ConnectorQuery, ConnectorType, DataSource, InternalQuery, QueryOperation,
};

fn require_env(key: &str) -> String {
    env::var(key).unwrap_or_else(|_| {
        eprintln!("Error: required environment variable '{}' is not set.", key);
        eprintln!("SQL Server vars: SQLSERVER_HOST, SQLSERVER_PORT, SQLSERVER_DATABASE, SQLSERVER_USER, SQLSERVER_PASSWORD");
        eprintln!("PostgreSQL vars: POSTGRES_HOST, POSTGRES_PORT, POSTGRES_DB, POSTGRES_USER, POSTGRES_PASSWORD");
        std::process::exit(1);
    })
}

struct ConnectorSummary {
    name: String,
    rows_returned: usize,
    columns: usize,
    execution_time_ms: u128,
}

#[tokio::main]
async fn main() {
    // --- Read SQL Server environment variables ---
    let ss_host = require_env("SQLSERVER_HOST");
    let ss_port = require_env("SQLSERVER_PORT");
    let ss_database = require_env("SQLSERVER_DATABASE");
    let ss_user = require_env("SQLSERVER_USER");
    let ss_password = require_env("SQLSERVER_PASSWORD");

    // --- Read PostgreSQL environment variables ---
    let pg_host = require_env("POSTGRES_HOST");
    let pg_port = require_env("POSTGRES_PORT");
    let pg_db = require_env("POSTGRES_DB");
    let pg_user = require_env("POSTGRES_USER");
    let pg_password = require_env("POSTGRES_PASSWORD");

    let mut summaries: Vec<ConnectorSummary> = Vec::new();

    // =========================================================
    // SQL Server
    // =========================================================
    println!("Connecting to SQL Server at {}:{}...", ss_host, ss_port);

    let ss_config = ConnectorInitConfig::new()
        .with_param("server", &ss_host)
        .with_param("port", &ss_port)
        .with_param("database", &ss_database)
        .with_param("username", &ss_user)
        .with_param("password", &ss_password)
        .with_param("trust_cert", "true")
        .with_timeout(30);

    let mut ss_connector = SqlServerConnector::new();
    if let Err(e) = ss_connector.connect(ss_config).await {
        eprintln!("Error connecting to SQL Server: {}", e);
        std::process::exit(1);
    }
    println!("SQL Server connected.");

    let mut ss_internal_query = InternalQuery::new(QueryOperation::Select);
    ss_internal_query.sources.push(DataSource {
        object_type: "sqlserver".to_string(),
        identifier: "INFORMATION_SCHEMA.TABLES".to_string(),
        alias: None,
    });

    let ss_connector_query = ConnectorQuery {
        connector_type: ConnectorType::SqlServer,
        query: ss_internal_query,
        connection_params: HashMap::new(),
    };

    println!("Executing: SELECT * FROM INFORMATION_SCHEMA.TABLES");
    let ss_start = Instant::now();
    match ss_connector.execute_query(ss_connector_query).await {
        Ok(result) => {
            summaries.push(ConnectorSummary {
                name: "SQL Server".to_string(),
                rows_returned: result.rows.len(),
                columns: result.columns.len(),
                execution_time_ms: ss_start.elapsed().as_millis(),
            });
        }
        Err(e) => {
            eprintln!("SQL Server query failed: {}", e);
            let _ = ss_connector.disconnect().await;
            std::process::exit(1);
        }
    }

    let _ = ss_connector.disconnect().await;
    println!("SQL Server disconnected.");

    // =========================================================
    // PostgreSQL
    // =========================================================
    println!("\nConnecting to PostgreSQL at {}:{}...", pg_host, pg_port);

    let pg_config = ConnectorInitConfig::new()
        .with_param("host", &pg_host)
        .with_param("port", &pg_port)
        .with_param("dbname", &pg_db)
        .with_param("user", &pg_user)
        .with_param("password", &pg_password)
        .with_timeout(30);

    let mut pg_connector = PostgresConnector::new();
    if let Err(e) = pg_connector.connect(pg_config).await {
        eprintln!("Error connecting to PostgreSQL: {}", e);
        std::process::exit(1);
    }
    println!("PostgreSQL connected.");

    let mut pg_internal_query = InternalQuery::new(QueryOperation::Select);
    pg_internal_query.sources.push(DataSource {
        object_type: "postgres".to_string(),
        identifier: "information_schema.tables".to_string(),
        alias: None,
    });
    pg_internal_query.limit = Some(10);

    let pg_connector_query = ConnectorQuery {
        connector_type: ConnectorType::PostgreSQL,
        query: pg_internal_query,
        connection_params: HashMap::new(),
    };

    println!("Executing: SELECT * FROM information_schema.tables LIMIT 10");
    let pg_start = Instant::now();
    match pg_connector.execute_query(pg_connector_query).await {
        Ok(result) => {
            summaries.push(ConnectorSummary {
                name: "PostgreSQL".to_string(),
                rows_returned: result.rows.len(),
                columns: result.columns.len(),
                execution_time_ms: pg_start.elapsed().as_millis(),
            });
        }
        Err(e) => {
            eprintln!("PostgreSQL query failed: {}", e);
            let _ = pg_connector.disconnect().await;
            std::process::exit(1);
        }
    }

    let _ = pg_connector.disconnect().await;
    println!("PostgreSQL disconnected.");

    // =========================================================
    // Print combined summary table
    // =========================================================
    println!();
    println!(
        "{:<15} {:<15} {:<10} {:<20}",
        "Connector", "Rows Returned", "Columns", "Execution Time (ms)"
    );
    println!("{}", "-".repeat(62));
    for s in &summaries {
        println!(
            "{:<15} {:<15} {:<10} {:<20}",
            s.name, s.rows_returned, s.columns, s.execution_time_ms
        );
    }
    println!();
    println!("Done.");
}
