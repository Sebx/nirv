# NIRV Engine

Universal data virtualization and compute orchestration engine written in Rust that allows users to query any backend (SQL databases, APIs, LLMs, file systems, or compute nodes) through a single unified SQL interface.

## 🚀 Key Features

- **Drop-in Database Replacement**: Acts as a PostgreSQL, MySQL, or SQLite server - existing applications connect without code changes
- **Universal SQL Interface**: Query files, APIs, databases, and other data sources using familiar SQL syntax
- **Protocol Compatibility**: Full wire protocol support for PostgreSQL, MySQL, and SQLite clients
- **Async Architecture**: Built on Tokio for high-performance concurrent query processing
- **Extensible Connectors**: Plugin architecture for adding new data source types
- **Zero-Code Integration**: JDBC and native database drivers work out of the box

## 🏗️ Architecture Overview

NIRV Engine uses a dispatcher-based architecture with protocol adapters that maintain compatibility with existing database clients:

```
┌─────────────────┐    ┌──────────────────┐
│   DB Clients    │    │   CLI Interface  │
│ (JDBC/Native)   │    │                  │
└─────────┬───────┘    └─────────┬────────┘
          │                      │
          ▼                      ▼
┌─────────────────────────────────────────────┐
│            Protocol Layer                   │
│  ┌─────────────┐  ┌─────────────┐         │
│  │ PostgreSQL  │  │   MySQL     │         │
│  │  Adapter    │  │  Adapter    │         │
│  └─────────────┘  └─────────────┘         │
└─────────────────────┬───────────────────────┘
                      │
                      ▼
┌─────────────────────────────────────────────┐
│               Core Engine                   │
│  ┌─────────────┐  ┌─────────────┐         │
│  │    Query    │  │    Query    │         │
│  │   Parser    │  │  Executor   │         │
│  └─────────────┘  └─────────────┘         │
│                  ┌─────────────┐           │
│                  │ Dispatcher  │           │
│                  └─────────────┘           │
└─────────────────────┬───────────────────────┘
                      │
                      ▼
┌─────────────────────────────────────────────┐
│            Connector Layer                  │
│  ┌─────────────┐  ┌─────────────┐         │
│  │ PostgreSQL  │  │    File     │         │
│  │ Connector   │  │ Connector   │         │
│  └─────────────┘  └─────────────┘         │
│  ┌─────────────┐  ┌─────────────┐         │
│  │    REST     │  │   Custom    │         │
│  │ Connector   │  │ Connector   │         │
│  └─────────────┘  └─────────────┘         │
└─────────────────────────────────────────────┘
```

## 📦 Installation

### Prerequisites

- Rust 1.70+ (install from [rustup.rs](https://rustup.rs/))
- Git

### From Source

```bash
# Clone the repository
git clone https://github.com/nirv/nirv-engine.git
cd nirv-engine

# Build the project
cargo build --release

# Install the binary
cargo install --path .
```

### Using Cargo

```bash
cargo install nirv-engine
```

## 🚀 Quick Start

### 1. Start NIRV Engine as PostgreSQL Server

```bash
# Start with default PostgreSQL protocol on port 5432
nirv server --protocol postgres --port 5432

# Or with custom configuration
nirv server --config config.toml
```

### 2. Connect with Any PostgreSQL Client

```bash
# Using psql
psql -h localhost -p 5432 -U nirv

# Using any JDBC application
jdbc:postgresql://localhost:5432/nirv
```

### 3. Query Different Data Sources

```sql
-- Query a CSV file
SELECT * FROM source('file.users.csv') WHERE age > 25;

-- Query a REST API
SELECT name, email FROM source('api.users') LIMIT 10;

-- Query a PostgreSQL database
SELECT * FROM source('postgres.customers') 
WHERE created_date > '2024-01-01';

-- Join across different sources
SELECT u.name, f.balance 
FROM source('api.users') u
JOIN source('file.finances.csv') f ON u.id = f.user_id;
```

## 💻 CLI Usage

NIRV Engine provides a comprehensive command-line interface for executing queries, managing data sources, and inspecting schemas.

### Available Commands

#### Execute Queries (`nirv query`)

Execute SQL queries against registered data sources with various output formats.

```bash
# Basic query execution with table output (default)
nirv query "SELECT * FROM source('mock.users')"

# JSON output format
nirv query "SELECT name, age FROM source('mock.users')" --format json

# CSV output format
nirv query "SELECT * FROM source('mock.users') WHERE age > 25" --format csv

# Enable verbose output for debugging
nirv query "SELECT * FROM source('mock.users')" --verbose

# Use custom configuration file
nirv query "SELECT * FROM source('postgres.customers')" --config /path/to/config.toml
```

**Query Command Options:**
- `--format, -f`: Output format (`table`, `json`, `csv`) - Default: `table`
- `--config, -c`: Path to configuration file
- `--verbose, -v`: Enable verbose output for debugging
- `--help, -h`: Show help information

#### List Data Sources (`nirv sources`)

Display available data sources and their connection status.

```bash
# List all registered data sources
nirv sources

# Show detailed information including capabilities and connection status
nirv sources --detailed
```

**Example Output:**
```
Available Data Sources:
  • mock
  • postgres_main
  • api_service
```

**Detailed Output Example:**
```
Available Data Sources:
  • mock
    Type: Mock
    Connected: Yes
    Supports Joins: Yes
    Supports Transactions: No
    Max Concurrent Queries: Unlimited
  • postgres_main
    Type: PostgreSQL
    Connected: Yes
    Supports Joins: Yes
    Supports Transactions: Yes
    Max Concurrent Queries: 20
```

**Sources Command Options:**
- `--detailed, -d`: Show detailed connector information
- `--help, -h`: Show help information

#### Show Schema Information (`nirv schema`)

Display schema information for a specific data source.

```bash
# Show schema for a data source (format: type.identifier)
nirv schema "mock.users"
nirv schema "postgres.customers"
nirv schema "file.products"
```

**Example Output:**
```
Schema for mock.users
Name: users
Primary Key: id

Columns:
  • id i32 NOT NULL
  • name String NOT NULL
  • email String NULL
  • age i32 NULL
  • created_at DateTime NOT NULL

Indexes:
  • idx_email on (email) (UNIQUE)
  • idx_age on (age)
```

**Schema Command Options:**
- `--help, -h`: Show help information

### Global CLI Options

These options are available for all commands:

- `--help, -h`: Show help information
- `--version, -V`: Show version information

### Query Syntax

NIRV Engine uses a special `source()` function to specify data sources in SQL queries:

```sql
-- Basic syntax
SELECT * FROM source('type.identifier')

-- Examples
SELECT * FROM source('mock.users')
SELECT * FROM source('postgres.customers')
SELECT * FROM source('file.products.csv')
SELECT * FROM source('api.users')

-- With WHERE clauses
SELECT name, email FROM source('mock.users') WHERE age > 25

-- Cross-source joins (when enabled)
SELECT u.name, o.total 
FROM source('postgres.users') u
JOIN source('api.orders') o ON u.id = o.user_id

-- Aggregations
SELECT COUNT(*), AVG(age) FROM source('mock.users')

-- Ordering and limits
SELECT * FROM source('mock.users') ORDER BY created_at DESC LIMIT 10
```

### Error Handling

The CLI provides clear error messages for common issues:

**Query Parsing Errors:**
```bash
$ nirv query "SELECT * FROM invalid_syntax"
Error: Query parsing error: Expected 'source(' function call
```

**Unregistered Data Source:**
```bash
$ nirv query "SELECT * FROM source('unknown.table')"
Error: Dispatcher error: Data object type 'unknown' is not registered. Available types: ["mock"]
```

**Connection Issues:**
```bash
$ nirv schema "postgres.users"
Error: Connector error: Failed to connect to PostgreSQL database
```

### Configuration File Usage

Specify custom configuration files to use different connector setups:

```bash
# Use development configuration
nirv query "SELECT * FROM source('postgres.users')" --config dev-config.toml

# Use production configuration
nirv query "SELECT * FROM source('postgres.users')" --config prod-config.toml
```

### Verbose Mode

Enable verbose mode to see detailed execution information:

```bash
$ nirv query "SELECT * FROM source('mock.users')" --verbose
Parsing query: SELECT * FROM source('mock.users')
Query parsed successfully. Sources: ["mock.users"]
Query routed to 1 connector(s)
Query executed successfully. 3 rows returned

id | name     | email              | age | created_at
---|----------|--------------------|----|------------------
1  | John Doe | john@example.com   | 30 | 2024-01-15 10:30:00
2  | Jane Doe | jane@example.com   | 25 | 2024-01-16 14:20:00
3  | Bob Smith| bob@example.com    | 35 | 2024-01-17 09:15:00
```

### Exit Codes

The CLI uses standard exit codes:
- `0`: Success
- `1`: General error (query parsing, execution, or connection issues)

### Environment Variables

The CLI respects the following environment variables:

- `NIRV_CONFIG`: Default configuration file path
- `NIRV_LOG_LEVEL`: Log level (`debug`, `info`, `warn`, `error`)
- `NIRV_OUTPUT_FORMAT`: Default output format (`table`, `json`, `csv`)

## ⚙️ Configuration

NIRV Engine uses TOML configuration files for comprehensive system configuration. The configuration system supports protocol adapters, connectors, security settings, and performance tuning.

### Configuration File Structure

Create a `config.toml` file with the following structure:

```toml
# Protocol Adapters Configuration
# Define which database protocols NIRV should support and on which ports

[[protocol_adapters]]
protocol_type = "PostgreSQL"
bind_address = "127.0.0.1"
port = 5432
max_connections = 100
connection_timeout = 30

[[protocol_adapters]]
protocol_type = "MySQL"
bind_address = "127.0.0.1"
port = 3306
max_connections = 50
connection_timeout = 30

[[protocol_adapters]]
protocol_type = "SQLite"
bind_address = "127.0.0.1"
port = 5433

# TLS Configuration (optional)
[protocol_adapters.tls_config]
cert_file = "/path/to/cert.pem"
key_file = "/path/to/key.pem"
ca_file = "/path/to/ca.pem"
require_client_cert = false

# Connector Configurations
# Define connections to various data sources

[connectors.postgres_main]
connector_type = "PostgreSQL"
connection_string = "postgresql://username:password@localhost:5432/database"

[connectors.postgres_main.pool_config]
min_connections = 2
max_connections = 20
connection_timeout = 30
idle_timeout = 600
max_lifetime = 3600

[connectors.postgres_main.timeout_config]
connect_timeout = 30
query_timeout = 300
transaction_timeout = 600

[connectors.mysql_analytics]
connector_type = "MySQL"
connection_string = "mysql://user:pass@localhost:3306/analytics"

[connectors.mysql_analytics.pool_config]
min_connections = 1
max_connections = 10
connection_timeout = 30

[connectors.api_service]
connector_type = "REST"
parameters = { 
    base_url = "https://api.example.com/v1", 
    api_key = "your-api-key-here",
    timeout = "30",
    retry_attempts = "3"
}

[connectors.local_files]
connector_type = "File"
parameters = { 
    base_path = "/data/files", 
    formats = ["csv", "json", "parquet"],
    encoding = "utf-8"
}

[connectors.s3_data]
connector_type = "S3"
parameters = {
    bucket = "my-data-bucket",
    region = "us-west-2",
    access_key_id = "AKIAIOSFODNN7EXAMPLE",
    secret_access_key = "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY"
}

# Dispatcher Configuration
# Controls query routing and execution behavior

[dispatcher]
max_concurrent_queries = 100
query_cache_size = 256  # MB
enable_cross_connector_joins = true
default_timeout = 300  # seconds

# Security Configuration
# Authentication, authorization, and audit settings

[security.authentication]
enabled = false
auth_method = "None"  # Options: "None", "Password", "Certificate", "LDAP", "OAuth2"
user_database = "/path/to/users.db"

[security.authentication.ldap_config]
server_url = "ldap://ldap.example.com:389"
bind_dn = "cn=admin,dc=example,dc=com"
bind_password = "admin_password"
user_search_base = "ou=users,dc=example,dc=com"
user_search_filter = "(uid={username})"

[security.authorization]
enabled = false
default_permissions = ["Read"]

[security.authorization.role_mappings]
admin = ["Read", "Write", "Admin", "Connect"]
user = ["Read", "Connect"]
readonly = ["Read"]

[security.audit_logging]
enabled = true
log_file = "/var/log/nirv/audit.log"
log_queries = true
log_connections = true
log_errors = true
```

### Configuration Sections

#### Protocol Adapters

Configure which database protocols NIRV should support:

```toml
[[protocol_adapters]]
protocol_type = "PostgreSQL"  # "PostgreSQL", "MySQL", "SQLite"
bind_address = "0.0.0.0"      # IP address to bind to
port = 5432                   # Port number
max_connections = 100         # Maximum concurrent connections
connection_timeout = 30       # Connection timeout in seconds
```

#### Connectors

Define connections to various data sources:

**PostgreSQL Connector:**
```toml
[connectors.my_postgres]
connector_type = "PostgreSQL"
connection_string = "postgresql://user:pass@host:port/database"

[connectors.my_postgres.pool_config]
min_connections = 2
max_connections = 20
connection_timeout = 30
idle_timeout = 600
max_lifetime = 3600
```

**MySQL Connector:**
```toml
[connectors.my_mysql]
connector_type = "MySQL"
connection_string = "mysql://user:pass@host:port/database"
```

**File Connector:**
```toml
[connectors.local_files]
connector_type = "File"
parameters = { 
    base_path = "/data", 
    formats = ["csv", "json", "parquet"],
    encoding = "utf-8"
}
```

**REST API Connector:**
```toml
[connectors.api]
connector_type = "REST"
parameters = { 
    base_url = "https://api.example.com",
    api_key = "your-key",
    timeout = "30",
    retry_attempts = "3"
}
```

#### Dispatcher Settings

Control query routing and execution:

```toml
[dispatcher]
max_concurrent_queries = 100        # Maximum parallel queries
query_cache_size = 256             # Cache size in MB
enable_cross_connector_joins = true # Allow joins across data sources
default_timeout = 300              # Default query timeout in seconds
```

#### Security Configuration

**Authentication:**
```toml
[security.authentication]
enabled = true
auth_method = "Password"  # "None", "Password", "Certificate", "LDAP", "OAuth2"
user_database = "/etc/nirv/users.db"
```

**Authorization:**
```toml
[security.authorization]
enabled = true
default_permissions = ["Read"]

[security.authorization.role_mappings]
admin = ["Read", "Write", "Admin", "Connect"]
user = ["Read", "Connect"]
```

**Audit Logging:**
```toml
[security.audit_logging]
enabled = true
log_file = "/var/log/nirv/audit.log"
log_queries = true
log_connections = true
log_errors = true
```

### Configuration Loading Priority

NIRV Engine loads configuration in the following order (later sources override earlier ones):

1. **Built-in defaults**
2. **Configuration file** (specified with `--config` or `NIRV_CONFIG` environment variable)
3. **Environment variables** (prefixed with `NIRV_`)

### Environment Variable Overrides

Configuration values can be overridden using environment variables:

```bash
# Override protocol port
export NIRV_PROTOCOL_ADAPTERS_0_PORT=5433

# Override connector connection string
export NIRV_CONNECTORS_POSTGRES_MAIN_CONNECTION_STRING="postgresql://newuser:newpass@newhost/newdb"

# Override dispatcher settings
export NIRV_DISPATCHER_MAX_CONCURRENT_QUERIES=200
```

### Configuration Validation

NIRV Engine validates configuration on startup and provides detailed error messages:

```bash
$ nirv server --config invalid-config.toml
Error: Configuration error: Invalid port number in protocol_adapters[0]: port must be between 1 and 65535
```

### Example Configurations

#### Development Configuration (`dev-config.toml`)
```toml
[[protocol_adapters]]
protocol_type = "PostgreSQL"
bind_address = "127.0.0.1"
port = 5432

[connectors.mock]
connector_type = "Mock"

[dispatcher]
max_concurrent_queries = 10
enable_cross_connector_joins = false

[security.authentication]
enabled = false
```

#### Production Configuration (`prod-config.toml`)
```toml
[[protocol_adapters]]
protocol_type = "PostgreSQL"
bind_address = "0.0.0.0"
port = 5432
max_connections = 1000

[connectors.prod_postgres]
connector_type = "PostgreSQL"
connection_string = "postgresql://nirv:secure_password@db.example.com:5432/production"

[connectors.prod_postgres.pool_config]
min_connections = 10
max_connections = 100

[dispatcher]
max_concurrent_queries = 500
query_cache_size = 1024
enable_cross_connector_joins = true

[security.authentication]
enabled = true
auth_method = "LDAP"

[security.audit_logging]
enabled = true
log_file = "/var/log/nirv/audit.log"
```

## 🔌 Supported Data Sources

### Databases
- **PostgreSQL**: Native connection with full SQL support
- **MySQL/MariaDB**: Native connection and protocol compatibility
- **SQLite**: File-based and in-memory databases

### Files
- **CSV**: Comma-separated values with automatic schema detection
- **JSON**: Structured JSON data with nested object support
- **Parquet**: Columnar format with efficient querying

### APIs
- **REST**: HTTP APIs with authentication and parameter mapping
- **GraphQL**: GraphQL endpoints with query translation
- **gRPC**: Protocol buffer services

### Cloud Services
- **AWS S3**: Object storage with file format detection
- **Google Cloud Storage**: Cloud object storage
- **Azure Blob Storage**: Microsoft cloud storage

## 🔧 Development

### Project Structure

```
nirv-engine/
├── Cargo.toml          # Project configuration and dependencies
├── README.md           # Project documentation
├── architecture.md     # Detailed architecture documentation
├── src/                # Source code
│   ├── lib.rs          # Main library entry point
│   ├── main.rs         # Binary entry point
│   ├── engine/         # Core engine components
│   │   ├── mod.rs      # Engine module exports
│   │   ├── query_parser.rs    # SQL query parsing
│   │   ├── query_planner.rs   # Query execution planning
│   │   ├── query_executor.rs  # Query execution
│   │   ├── dispatcher.rs      # Query routing and dispatch
│   │   └── engine.rs          # Main engine coordination
│   ├── connectors/     # Data source connectors
│   │   ├── mod.rs      # Connector module exports
│   │   ├── connector_trait.rs # Base connector interface
│   │   ├── mock_connector.rs  # Test connector
│   │   ├── postgres_connector.rs # PostgreSQL connector
│   │   ├── file_connector.rs     # File system connector
│   │   └── rest_connector.rs     # REST API connector
│   ├── protocol/       # Database protocol adapters
│   │   ├── mod.rs      # Protocol module exports
│   │   ├── protocol_trait.rs     # Base protocol interface
│   │   ├── postgres_protocol.rs  # PostgreSQL wire protocol
│   │   ├── mysql_protocol.rs     # MySQL wire protocol
│   │   └── sqlite_protocol.rs    # SQLite protocol
│   ├── cli/            # Command-line interface
│   │   ├── mod.rs      # CLI module exports
│   │   ├── cli_args.rs # Argument parsing with clap
│   │   ├── cli_runner.rs       # CLI execution logic
│   │   └── output_formatter.rs # Result formatting
│   └── utils/          # Utility modules
│       ├── mod.rs      # Utils module exports
│       ├── error.rs    # Error types and handling
│       ├── config.rs   # Configuration types
│       └── types.rs    # Common data types
├── tests/              # Integration tests
│   ├── cli_integration_tests.rs      # CLI testing
│   ├── engine_integration_tests.rs   # Engine testing
│   ├── postgres_integration_tests.rs # PostgreSQL tests
│   ├── postgres_protocol_tests.rs    # Protocol tests
│   ├── query_executor_tests.rs       # Executor tests
│   └── query_planner_tests.rs        # Planner tests
└── target/             # Build artifacts (generated)
```

### Development Environment Setup

#### Prerequisites

```bash
# Install Rust (latest stable)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.cargo/env

# Install additional tools
cargo install cargo-watch    # For file watching during development
cargo install cargo-audit    # For security auditing
cargo install cargo-outdated # For dependency management
```

#### Clone and Setup

```bash
# Clone the repository
git clone https://github.com/nirv/nirv-engine.git
cd nirv-engine

# Install dependencies and build
cargo build

# Run tests to verify setup
cargo test
```

### Building

#### Development Builds

```bash
# Debug build (faster compilation, slower execution)
cargo build

# Build specific binary
cargo build --bin nirv

# Build library only
cargo build --lib
```

#### Release Builds

```bash
# Release build (slower compilation, faster execution)
cargo build --release

# Build with all features enabled
cargo build --release --all-features

# Build for specific target
cargo build --release --target x86_64-unknown-linux-gnu
```

#### Feature Flags

```bash
# Build with specific features
cargo build --features "postgres mysql"

# Build without default features
cargo build --no-default-features

# List available features
cargo metadata --format-version 1 | jq '.packages[0].features'
```

### Testing

#### Running Tests

```bash
# Run all tests
cargo test

# Run tests with output
cargo test -- --nocapture

# Run specific test module
cargo test engine::tests
cargo test connectors::mock_connector::tests

# Run integration tests only
cargo test --test '*'

# Run unit tests only
cargo test --lib
```

#### Test Categories

```bash
# Run tests by pattern
cargo test query_parser
cargo test postgres
cargo test cli

# Run tests with specific verbosity
cargo test -- --test-threads=1 --nocapture

# Run ignored tests
cargo test -- --ignored
```

#### Test Coverage

```bash
# Install coverage tool
cargo install cargo-tarpaulin

# Generate coverage report
cargo tarpaulin --out Html --output-dir coverage

# View coverage report
open coverage/tarpaulin-report.html
```

### Code Quality

#### Formatting

```bash
# Format all code
cargo fmt

# Check formatting without applying
cargo fmt -- --check

# Format specific file
cargo fmt src/engine/query_parser.rs
```

#### Linting

```bash
# Run clippy lints
cargo clippy

# Run clippy with all features
cargo clippy --all-features

# Run clippy with specific lint levels
cargo clippy -- -D warnings

# Fix automatically fixable lints
cargo clippy --fix
```

#### Static Analysis

```bash
# Check code without building
cargo check

# Check all targets
cargo check --all-targets

# Audit dependencies for security issues
cargo audit

# Check for outdated dependencies
cargo outdated
```

### Development Workflow

#### File Watching

```bash
# Watch for changes and run tests
cargo watch -x test

# Watch and run specific command
cargo watch -x "test engine::tests"

# Watch and run clippy
cargo watch -x clippy

# Watch and build
cargo watch -x build
```

#### Debugging

```bash
# Build with debug symbols
cargo build --profile dev

# Run with debug logging
RUST_LOG=debug cargo run -- query "SELECT * FROM source('mock.users')"

# Use rust-gdb for debugging
rust-gdb target/debug/nirv

# Use lldb on macOS
rust-lldb target/debug/nirv
```

### Contributing Guidelines

#### Code Style

- Follow Rust naming conventions (snake_case for functions/variables, PascalCase for types)
- Use `cargo fmt` for consistent formatting
- Ensure `cargo clippy` passes without warnings
- Add comprehensive documentation for public APIs
- Write tests for new functionality

#### Commit Guidelines

```bash
# Use conventional commit format
git commit -m "feat: add PostgreSQL connector support"
git commit -m "fix: resolve query parsing issue with nested functions"
git commit -m "docs: update configuration examples"
git commit -m "test: add integration tests for MySQL protocol"
```

#### Pull Request Process

1. **Fork the repository** and create a feature branch
2. **Write tests** for new functionality
3. **Ensure all tests pass**: `cargo test`
4. **Run linting**: `cargo clippy`
5. **Format code**: `cargo fmt`
6. **Update documentation** if needed
7. **Create pull request** with clear description

#### Testing New Features

```bash
# Test your changes thoroughly
cargo test

# Test with different configurations
cargo test --features "postgres mysql sqlite"

# Run integration tests
cargo test --test '*'

# Test CLI functionality
cargo run -- query "SELECT * FROM source('mock.users')"
cargo run -- sources --detailed
```

### Performance Profiling

#### CPU Profiling

```bash
# Install profiling tools
cargo install cargo-profdata
cargo install flamegraph

# Generate flame graph
cargo flamegraph --bin nirv -- query "SELECT * FROM source('mock.users')"

# Profile with perf (Linux)
perf record --call-graph=dwarf cargo run --release -- query "SELECT * FROM source('mock.users')"
perf report
```

#### Memory Profiling

```bash
# Install valgrind (Linux)
sudo apt-get install valgrind

# Run with valgrind
valgrind --tool=massif cargo run --release -- query "SELECT * FROM source('mock.users')"

# Use heaptrack (Linux)
heaptrack cargo run --release -- query "SELECT * FROM source('mock.users')"
```

#### Benchmarking

```bash
# Run benchmarks
cargo bench

# Run specific benchmark
cargo bench query_parser

# Generate benchmark report
cargo bench -- --output-format html
```

### Documentation

#### Generating Documentation

```bash
# Generate API documentation
cargo doc

# Generate and open documentation
cargo doc --open

# Generate documentation with private items
cargo doc --document-private-items

# Generate documentation for all features
cargo doc --all-features
```

#### Documentation Guidelines

- Use `///` for public API documentation
- Include examples in documentation comments
- Document error conditions and panics
- Keep documentation up to date with code changes

### Release Process

#### Version Management

```bash
# Update version in Cargo.toml
# Follow semantic versioning (MAJOR.MINOR.PATCH)

# Create git tag
git tag -a v0.2.0 -m "Release version 0.2.0"
git push origin v0.2.0
```

#### Publishing

```bash
# Dry run publish
cargo publish --dry-run

# Publish to crates.io
cargo publish
```

## 🤝 Contributing

1. Fork the repository
2. Create a feature branch (`git checkout -b feature/amazing-feature`)
3. Make your changes
4. Add tests for new functionality
5. Run the test suite (`cargo test`)
6. Commit your changes (`git commit -m 'Add amazing feature'`)
7. Push to the branch (`git push origin feature/amazing-feature`)
8. Open a Pull Request

### Development Guidelines

- Follow Rust naming conventions and idioms
- Add comprehensive tests for new features
- Update documentation for API changes
- Use `cargo fmt` and `cargo clippy` before committing
- Write clear commit messages

## 📚 Documentation

### Core Documentation
- [Architecture Guide](architecture.md) - Detailed system design and component relationships
- [Configuration Reference](docs/configuration.md) - Complete configuration options and examples
- [CLI Reference](docs/cli-reference.md) - Comprehensive command-line interface guide

### API and Development
- [API Documentation](https://docs.rs/nirv-engine) - Generated Rust API documentation
- [Connector Development Guide](docs/connectors.md) - Guide for creating custom connectors
- [Protocol Development Guide](docs/protocols.md) - Guide for implementing new database protocols

### Deployment and Operations
- [Installation Guide](docs/installation.md) - Detailed installation instructions for various platforms
- [Deployment Guide](docs/deployment.md) - Production deployment best practices
- [Monitoring and Observability](docs/monitoring.md) - Metrics, logging, and health checks
- [Security Guide](docs/security.md) - Security configuration and best practices

### Tutorials and Examples
- [Quick Start Tutorial](docs/quickstart.md) - Step-by-step getting started guide
- [Configuration Examples](docs/examples/) - Real-world configuration examples
- [Query Examples](docs/query-examples.md) - SQL query patterns and best practices
- [Integration Examples](docs/integrations/) - Examples for common tools and frameworks

## 🐛 Troubleshooting

### Common Issues and Solutions

#### Connection Issues

**Problem: Connection Refused**
```bash
Error: Connection refused (os error 111)
```

**Solutions:**
```bash
# Check if NIRV is running
ps aux | grep nirv

# Check port availability
netstat -tlnp | grep 5432

# Verify configuration
nirv query "SELECT 1" --config your-config.toml --verbose

# Check firewall settings
sudo ufw status
```

**Problem: Authentication Failed**
```bash
Error: Protocol error: Authentication failed for user 'username'
```

**Solutions:**
- Verify username and password in connection string
- Check authentication method in security configuration
- Ensure user exists in configured user database
- For LDAP: verify LDAP server connectivity and credentials

#### Query Issues

**Problem: Query Parsing Errors**
```bash
Error: Query parsing error: Expected 'source(' function call
```

**Solutions:**
```bash
# Use correct source() syntax
nirv query "SELECT * FROM source('type.identifier')"

# Enable verbose mode for detailed parsing information
nirv query "SELECT * FROM source('mock.users')" --verbose

# Check available data sources
nirv sources --detailed
```

**Problem: Unregistered Data Source**
```bash
Error: Dispatcher error: Data object type 'unknown' is not registered
```

**Solutions:**
```bash
# List available data sources
nirv sources

# Check connector configuration
cat config.toml | grep -A 5 "\[connectors"

# Verify connector is properly configured and connected
nirv sources --detailed
```

#### Connector Issues

**Problem: PostgreSQL Connection Failed**
```bash
Error: Connector error: Failed to connect to PostgreSQL database
```

**Solutions:**
- Verify PostgreSQL server is running and accessible
- Check connection string format: `postgresql://user:pass@host:port/database`
- Test connection manually: `psql -h host -p port -U user -d database`
- Check network connectivity and firewall rules
- Verify SSL/TLS configuration if required

**Problem: File Connector Issues**
```bash
Error: Connector error: File not found or permission denied
```

**Solutions:**
- Verify file path exists and is readable
- Check file permissions: `ls -la /path/to/file`
- Ensure base_path in configuration is correct
- Verify file format is supported (CSV, JSON, Parquet)

**Problem: REST API Connector Issues**
```bash
Error: Connector error: HTTP request failed with status 401
```

**Solutions:**
- Verify API endpoint URL and authentication credentials
- Check API key validity and permissions
- Test API manually: `curl -H "Authorization: Bearer token" https://api.example.com/endpoint`
- Verify network connectivity and proxy settings

#### Performance Issues

**Problem: Slow Query Execution**
```bash
Query executed successfully. 1000 rows returned (execution time: 30.5s)
```

**Solutions:**
- Enable query caching in dispatcher configuration
- Increase connection pool size for frequently used connectors
- Use WHERE clauses to reduce data transfer
- Consider adding indexes to underlying data sources
- Monitor resource usage: `top`, `htop`, `iostat`

**Problem: High Memory Usage**
```bash
Error: Out of memory error during query execution
```

**Solutions:**
- Reduce query result set size with LIMIT clauses
- Increase system memory or swap space
- Configure streaming for large result sets
- Optimize connector pool settings
- Use pagination for large datasets

#### Configuration Issues

**Problem: Invalid Configuration**
```bash
Error: Configuration error: Invalid port number in protocol_adapters[0]
```

**Solutions:**
- Validate TOML syntax: Use online TOML validators
- Check configuration against examples in documentation
- Verify all required fields are present
- Use configuration validation: `nirv validate-config config.toml`

**Problem: Permission Denied**
```bash
Error: Permission denied when reading configuration file
```

**Solutions:**
```bash
# Check file permissions
ls -la config.toml

# Fix permissions
chmod 644 config.toml

# Verify user has read access
sudo -u nirv cat config.toml
```

### Debugging Tools

#### Enable Verbose Logging

```bash
# CLI verbose mode
nirv query "SELECT * FROM source('mock.users')" --verbose

# Environment variable
export NIRV_LOG_LEVEL=debug
nirv query "SELECT * FROM source('mock.users')"
```

#### Configuration Validation

```bash
# Validate configuration file
nirv validate-config config.toml

# Test specific connector
nirv test-connector postgres_main --config config.toml
```

#### Health Checks

```bash
# Check system status
nirv status

# List all connectors and their status
nirv sources --detailed

# Test connectivity to all configured data sources
nirv health-check --all
```

### Log Analysis

#### Log Locations

- **System logs**: `/var/log/nirv/nirv.log`
- **Audit logs**: `/var/log/nirv/audit.log`
- **Error logs**: `/var/log/nirv/error.log`

#### Common Log Patterns

**Connection Issues:**
```
ERROR [nirv::connectors::postgres] Failed to establish connection: Connection refused
```

**Query Parsing Issues:**
```
ERROR [nirv::engine::query_parser] Parse error at line 1, column 15: Expected identifier
```

**Authentication Issues:**
```
WARN [nirv::protocol::postgres] Authentication failed for user 'testuser' from 127.0.0.1
```

### Performance Monitoring

#### Key Metrics to Monitor

- **Query execution time**: Average and 95th percentile
- **Connection pool utilization**: Active vs. available connections
- **Memory usage**: Heap and system memory consumption
- **Error rates**: Failed queries and connection attempts
- **Throughput**: Queries per second

#### Monitoring Commands

```bash
# Real-time query monitoring
nirv monitor --queries

# Connection pool status
nirv monitor --connections

# System resource usage
nirv monitor --resources
```

### Getting Help

If you encounter issues not covered in this troubleshooting guide:

1. **Check the logs** for detailed error messages
2. **Enable verbose mode** to get more diagnostic information
3. **Search existing issues** on GitHub
4. **Create a new issue** with:
   - NIRV Engine version
   - Configuration file (sanitized)
   - Complete error message
   - Steps to reproduce
   - System information (OS, Rust version)

#### Support Channels

- 📧 **Email**: support@nirv.dev
- 💬 **Discord**: [NIRV Community](https://discord.gg/nirv)
- 🐛 **GitHub Issues**: [Report bugs and request features](https://github.com/nirv/nirv-engine/issues)
- 📖 **Documentation**: [Comprehensive guides and API docs](https://docs.nirv.dev)

## 📄 License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## 🙏 Acknowledgments

- Built with [Rust](https://www.rust-lang.org/) and [Tokio](https://tokio.rs/)
- SQL parsing powered by [sqlparser-rs](https://github.com/sqlparser-rs/sqlparser-rs)
- Protocol implementations inspired by PostgreSQL and MySQL specifications

## 📞 Support

- 📧 Email: support@nirv.dev
- 💬 Discord: [NIRV Community](https://discord.gg/nirv)
- 🐛 Issues: [GitHub Issues](https://github.com/nirv/nirv-engine/issues)
- 📖 Docs: [Documentation Site](https://docs.nirv.dev)