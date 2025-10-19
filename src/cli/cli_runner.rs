use clap::Parser;
use colored::*;
use crate::cli::{CliArgs, Commands, OutputFormatter};
use crate::engine::{DefaultQueryParser, DefaultQueryExecutor, DefaultDispatcher, Dispatcher};
use crate::connectors::{MockConnector, Connector};
use crate::utils::error::NirvResult;

/// Main CLI runner that handles command execution
pub struct CliRunner {
    query_parser: DefaultQueryParser,
    query_executor: DefaultQueryExecutor,
    dispatcher: DefaultDispatcher,
}

impl CliRunner {
    /// Create a new CLI runner with default components
    pub async fn new() -> NirvResult<Self> {
        let query_parser = DefaultQueryParser::new()?;
        let mut dispatcher = DefaultDispatcher::new();
        
        // Register and connect mock connector for testing
        let mut mock_connector = Box::new(MockConnector::new());
        let config = crate::connectors::ConnectorInitConfig::new();
        mock_connector.connect(config).await?;
        dispatcher.register_connector("mock", mock_connector).await?;
        
        let query_executor = DefaultQueryExecutor::new();
        
        Ok(Self {
            query_parser,
            query_executor,
            dispatcher,
        })
    }
    
    /// Execute a SQL query and return formatted results
    pub async fn execute_query(&self, sql: &str, format: &crate::cli::OutputFormat, verbose: bool) -> NirvResult<String> {
        if verbose {
            eprintln!("{}", OutputFormatter::format_info(&format!("Parsing query: {}", sql)));
        }
        
        // Parse the SQL query
        let internal_query = self.query_parser.parse(sql)?;
        
        if verbose {
            eprintln!("{}", OutputFormatter::format_info(&format!("Query parsed successfully. Sources: {:?}", 
                internal_query.sources.iter().map(|s| format!("{}.{}", s.object_type, s.identifier)).collect::<Vec<_>>())));
        }
        
        // Route the query through the dispatcher
        let connector_queries = self.dispatcher.route_query(&internal_query).await?;
        
        if verbose {
            eprintln!("{}", OutputFormatter::format_info(&format!("Query routed to {} connector(s)", connector_queries.len())));
        }
        
        // Execute the distributed query
        let result = self.dispatcher.execute_distributed_query(connector_queries).await?;
        
        if verbose {
            eprintln!("{}", OutputFormatter::format_info(&format!("Query executed successfully. {} rows returned", result.row_count())));
        }
        
        // Format the results
        Ok(OutputFormatter::format_result(&result, format))
    }
    
    /// List available data sources
    pub fn list_sources(&self, detailed: bool) -> String {
        let available_types = self.dispatcher.list_available_types();
        
        if available_types.is_empty() {
            return OutputFormatter::format_info("No data sources are currently registered.");
        }
        
        let mut output = String::new();
        output.push_str(&format!("{}\n", "Available Data Sources:".bold()));
        
        for data_type in &available_types {
            if detailed {
                if let Some(connector) = self.dispatcher.get_connector(data_type) {
                    let capabilities = connector.get_capabilities();
                    output.push_str(&format!("  {} {}\n", "•".green(), data_type.cyan().bold()));
                    output.push_str(&format!("    Type: {:?}\n", connector.get_connector_type()));
                    output.push_str(&format!("    Connected: {}\n", 
                        if connector.is_connected() { "Yes".green() } else { "No".red() }));
                    output.push_str(&format!("    Supports Joins: {}\n", 
                        if capabilities.supports_joins { "Yes".green() } else { "No".red() }));
                    output.push_str(&format!("    Supports Transactions: {}\n", 
                        if capabilities.supports_transactions { "Yes".green() } else { "No".red() }));
                    output.push_str(&format!("    Max Concurrent Queries: {}\n", 
                        capabilities.max_concurrent_queries.map(|n| n.to_string()).unwrap_or_else(|| "Unlimited".to_string())));
                } else {
                    output.push_str(&format!("  {} {} (connector not found)\n", "•".red(), data_type));
                }
            } else {
                output.push_str(&format!("  {} {}\n", "•".green(), data_type.cyan()));
            }
        }
        
        output
    }
    
    /// Show schema information for a data source
    pub async fn show_schema(&self, source: &str) -> NirvResult<String> {
        // Parse source identifier (e.g., "postgres.users" -> type="postgres", identifier="users")
        let parts: Vec<&str> = source.split('.').collect();
        if parts.len() != 2 {
            return Err(crate::utils::error::NirvError::Internal(
                "Source must be in format 'type.identifier' (e.g., 'postgres.users')".to_string()
            ));
        }
        
        let object_type = parts[0];
        let identifier = parts[1];
        
        // Check if the data object type is registered
        if !self.dispatcher.is_type_registered(object_type) {
            return Err(crate::utils::error::NirvError::Dispatcher(
                crate::utils::error::DispatcherError::UnregisteredObjectType(
                    format!("Data object type '{}' is not registered. Available types: {:?}", 
                           object_type, 
                           self.dispatcher.list_available_types())
                )
            ));
        }
        
        // Get the connector and retrieve schema
        if let Some(connector) = self.dispatcher.get_connector(object_type) {
            let schema = connector.get_schema(identifier).await?;
            
            let mut output = String::new();
            output.push_str(&format!("{} {}\n", "Schema for".bold(), source.cyan().bold()));
            output.push_str(&format!("Name: {}\n", schema.name));
            
            if let Some(pk) = &schema.primary_key {
                output.push_str(&format!("Primary Key: {}\n", pk.join(", ").yellow()));
            }
            
            output.push_str(&format!("\n{}\n", "Columns:".bold()));
            for col in &schema.columns {
                let nullable_str = if col.nullable { "NULL" } else { "NOT NULL" };
                let nullable_colored = if col.nullable { 
                    nullable_str.yellow() 
                } else { 
                    nullable_str.green() 
                };
                
                output.push_str(&format!("  {} {} {} {}\n", 
                    "•".green(),
                    col.name.cyan().bold(),
                    format!("{:?}", col.data_type).blue(),
                    nullable_colored
                ));
            }
            
            if !schema.indexes.is_empty() {
                output.push_str(&format!("\n{}\n", "Indexes:".bold()));
                for index in &schema.indexes {
                    let unique_str = if index.unique { " (UNIQUE)" } else { "" };
                    output.push_str(&format!("  {} {} on ({}){}\n", 
                        "•".green(),
                        index.name.cyan(),
                        index.columns.join(", ").yellow(),
                        unique_str.magenta()
                    ));
                }
            }
            
            Ok(output)
        } else {
            Err(crate::utils::error::NirvError::Internal(
                format!("Connector for type '{}' not found", object_type)
            ))
        }
    }
}

/// Main entry point for CLI execution
pub async fn run_cli() -> anyhow::Result<()> {
    let args = CliArgs::parse();
    
    // Initialize CLI runner
    let runner = match CliRunner::new().await {
        Ok(runner) => runner,
        Err(e) => {
            eprintln!("{}", OutputFormatter::format_error(&e));
            std::process::exit(1);
        }
    };
    
    // Execute the command
    let result = match args.command {
        Commands::Query { sql, format, config: _, verbose } => {
            match runner.execute_query(&sql, &format, verbose).await {
                Ok(output) => {
                    println!("{}", output);
                    Ok(())
                }
                Err(e) => {
                    eprintln!("{}", OutputFormatter::format_error(&e));
                    std::process::exit(1);
                }
            }
        }
        
        Commands::Sources { detailed } => {
            let output = runner.list_sources(detailed);
            println!("{}", output);
            Ok(())
        }
        
        Commands::Schema { source } => {
            match runner.show_schema(&source).await {
                Ok(output) => {
                    println!("{}", output);
                    Ok(())
                }
                Err(e) => {
                    eprintln!("{}", OutputFormatter::format_error(&e));
                    std::process::exit(1);
                }
            }
        }
    };
    
    result
}