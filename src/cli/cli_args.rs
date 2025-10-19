use clap::{Parser, Subcommand, ValueEnum};

/// NIRV Engine CLI - Universal data virtualization and compute orchestration
#[derive(Parser, Debug)]
#[command(name = "nirv")]
#[command(about = "Universal data virtualization and compute orchestration engine")]
#[command(version = "0.1.0")]
pub struct CliArgs {
    #[command(subcommand)]
    pub command: Commands,
}

/// Available CLI commands
#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Execute a SQL query
    Query {
        /// SQL query to execute
        #[arg(value_name = "SQL")]
        sql: String,
        
        /// Output format
        #[arg(short, long, default_value = "table")]
        format: OutputFormat,
        
        /// Connector configuration file
        #[arg(short, long)]
        config: Option<String>,
        
        /// Enable verbose output
        #[arg(short, long)]
        verbose: bool,
    },
    
    /// List available data sources
    Sources {
        /// Show detailed information
        #[arg(short, long)]
        detailed: bool,
    },
    
    /// Show schema information for a data source
    Schema {
        /// Data source identifier (e.g., "postgres.users")
        source: String,
    },
}

/// Output format options
#[derive(ValueEnum, Debug, Clone)]
pub enum OutputFormat {
    /// Formatted table output
    Table,
    /// JSON output
    Json,
    /// CSV output
    Csv,
}

impl std::fmt::Display for OutputFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OutputFormat::Table => write!(f, "table"),
            OutputFormat::Json => write!(f, "json"),
            OutputFormat::Csv => write!(f, "csv"),
        }
    }
}