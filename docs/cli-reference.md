# NIRV Engine CLI Reference (v0.1.0)

This document provides a comprehensive reference for the NIRV Engine command-line interface.

## Current Status
- ✅ **Fully Functional** - CLI interface working with all core features
- ✅ **Multiple Output Formats** - Table, JSON, CSV support
- ✅ **Multi-Source Queries** - PostgreSQL, File, REST API, Mock connectors
- ✅ **SQL Server Ready** - Complete TDS protocol support
- 📊 **Test Coverage** - CLI integration tests passing

## Table of Contents

- [Installation](#installation)
- [Global Options](#global-options)
- [Commands](#commands)
  - [query](#query-command)
  - [sources](#sources-command)
  - [schema](#schema-command)
- [Output Formats](#output-formats)
- [Configuration](#configuration)
- [Environment Variables](#environment-variables)
- [Examples](#examples)
- [Troubleshooting](#troubleshooting)

## Installation

### From Source
```bash
git clone https://github.com/nirv/nirv-engine.git
cd nirv-engine
cargo install --path .
```

### Using Cargo
```bash
cargo install nirv-engine
```

### Verify Installation
```bash
nirv --version
```

## Global Options

These options are available for all commands:

| Option | Short | Description |
|--------|-------|-------------|
| `--help` | `-h` | Show help information |
| `--version` | `-V` | Show version information |

## Commands

### query Command

Execute SQL queries against registered data sources.

#### Syntax
```bash
nirv query [OPTIONS] <SQL>
```

#### Arguments
- `<SQL>` - SQL query to execute (required)

#### Options

| Option | Short | Type | Default | Description |
|--------|-------|------|---------|-------------|
| `--format` | `-f` | string | `table` | Output format: `table`, `json`, `csv` |
| `--config` | `-c` | path | - | Path to configuration file |
| `--verbose` | `-v` | flag | false | Enable verbose output for debugging |

#### Examples

**Basic Query:**
```bash
nirv query "SELECT * FROM source('mock.users')"
```

**JSON Output:**
```bash
nirv query "SELECT name, age FROM source('mock.users')" --format json
```

**CSV Output:**
```bash
nirv query "SELECT * FROM source('mock.users') WHERE age > 25" --format csv
```

**With Custom Configuration:**
```bash
nirv query "SELECT * FROM source('postgres.customers')" --config /etc/nirv/prod-config.toml
```

**Verbose Mode:**
```bash
nirv query "SELECT * FROM source('mock.users')" --verbose
```

#### Query Syntax

NIRV Engine uses a special `source()` function to specify data sources:

```sql
-- Basic syntax
SELECT * FROM source('type.identifier')

-- Examples
SELECT * FROM source('mock.users')          -- Mock connector
SELECT * FROM source('postgres.customers')  -- PostgreSQL connector
SELECT * FROM source('file.products.csv')   -- File connector
SELECT * FROM source('api.users')           -- REST API connector

-- Standard SQL operations
SELECT name, email FROM source('mock.users') WHERE age > 25
SELECT COUNT(*) FROM source('postgres.orders') GROUP BY status
SELECT * FROM source('file.data.csv') ORDER BY created_at DESC LIMIT 10

-- Cross-source joins (when enabled)
SELECT u.name, o.total 
FROM source('postgres.users') u
JOIN source('api.orders') o ON u.id = o.user_id
```

#### Output Examples

**Table Format (Default):**
```
+----+----------+------------------+-----+---------------------+
| id | name     | email            | age | created_at          |
+----+----------+------------------+-----+---------------------+
| 1  | John Doe | john@example.com | 30  | 2024-01-15 10:30:00 |
| 2  | Jane Doe | jane@example.com | 25  | 2024-01-16 14:20:00 |
+----+----------+------------------+-----+---------------------+

2 rows in 15.23ms
```

**JSON Format:**
```json
{
  "data": [
    {
      "id": 1,
      "name": "John Doe",
      "email": "john@example.com",
      "age": 30,
      "created_at": "2024-01-15 10:30:00"
    },
    {
      "id": 2,
      "name": "Jane Doe",
      "email": "jane@example.com",
      "age": 25,
      "created_at": "2024-01-16 14:20:00"
    }
  ],
  "metadata": {
    "columns": [
      {"name": "id", "type": "Integer", "nullable": false},
      {"name": "name", "type": "Text", "nullable": false},
      {"name": "email", "type": "Text", "nullable": true},
      {"name": "age", "type": "Integer", "nullable": true},
      {"name": "created_at", "type": "DateTime", "nullable": false}
    ],
    "row_count": 2,
    "execution_time_ms": 15
  }
}
```

**CSV Format:**
```csv
id,name,email,age,created_at
1,John Doe,john@example.com,30,2024-01-15 10:30:00
2,Jane Doe,jane@example.com,25,2024-01-16 14:20:00
```

### sources Command

List available data sources and their connection status.

#### Syntax
```bash
nirv sources [OPTIONS]
```

#### Options

| Option | Short | Description |
|--------|-------|-------------|
| `--detailed` | `-d` | Show detailed connector information |

#### Examples

**Basic List:**
```bash
nirv sources
```

**Detailed Information:**
```bash
nirv sources --detailed
```

#### Output Examples

**Basic Output:**
```
Available Data Sources:
  • mock
  • postgres_main
  • api_service
```

**Detailed Output:**
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
  • api_service
    Type: REST
    Connected: Yes
    Supports Joins: No
    Supports Transactions: No
    Max Concurrent Queries: 10
```

### schema Command

Display schema information for a specific data source.

#### Syntax
```bash
nirv schema <SOURCE>
```

#### Arguments
- `<SOURCE>` - Data source identifier in format `type.identifier` (required)

#### Examples

```bash
nirv schema "mock.users"
nirv schema "postgres.customers"
nirv schema "file.products"
```

#### Output Example

```
Schema for mock.users
Name: users
Primary Key: id

Columns:
  • id Integer NOT NULL
  • name Text NOT NULL
  • email Text NULL
  • age Integer NULL
  • created_at DateTime NOT NULL

Indexes:
  • idx_email on (email) (UNIQUE)
  • idx_age on (age)
```

## Output Formats

### Table Format

The default format provides a human-readable table with:
- Colored headers and data types
- Proper column alignment
- Execution time and row count footer
- Visual separators for clarity

**Features:**
- Automatic column width calculation
- Color coding for different data types
- NULL values displayed as dimmed text
- Boolean values colored (green for true, red for false)

### JSON Format

Structured JSON output suitable for programmatic processing:
- `data` array containing result rows
- `metadata` object with column information and execution stats
- Proper JSON data type mapping
- Binary data encoded as Base64

**Use Cases:**
- API integration
- Data processing pipelines
- Automated testing
- Log analysis

### CSV Format

Standard comma-separated values format:
- Header row with column names
- Proper CSV escaping for special characters
- Compatible with spreadsheet applications
- Suitable for data export/import

**Use Cases:**
- Data export to Excel/Google Sheets
- Bulk data processing
- ETL pipelines
- Data analysis tools

## Configuration

### Configuration File

Specify a custom configuration file:

```bash
nirv query "SELECT * FROM source('postgres.users')" --config /path/to/config.toml
```

### Default Configuration Locations

NIRV searches for configuration files in the following order:
1. File specified with `--config` option
2. `NIRV_CONFIG` environment variable
3. `./nirv.toml` (current directory)
4. `~/.nirv/config.toml` (user home directory)
5. `/etc/nirv/config.toml` (system-wide)

### Configuration Example

```toml
# Basic configuration for CLI usage
[[protocol_adapters]]
protocol_type = "PostgreSQL"
bind_address = "127.0.0.1"
port = 5432

[connectors.postgres_main]
connector_type = "PostgreSQL"
connection_string = "postgresql://user:pass@localhost/database"

[connectors.mock]
connector_type = "Mock"
```

## Environment Variables

### Configuration Override

| Variable | Description | Example |
|----------|-------------|---------|
| `NIRV_CONFIG` | Default configuration file path | `/etc/nirv/config.toml` |
| `NIRV_LOG_LEVEL` | Log level for debugging | `debug`, `info`, `warn`, `error` |
| `NIRV_OUTPUT_FORMAT` | Default output format | `table`, `json`, `csv` |

### Usage Examples

```bash
# Set default configuration
export NIRV_CONFIG="/etc/nirv/prod-config.toml"

# Enable debug logging
export NIRV_LOG_LEVEL=debug
nirv query "SELECT * FROM source('mock.users')"

# Set default output format
export NIRV_OUTPUT_FORMAT=json
nirv query "SELECT * FROM source('mock.users')"
```

### Connector-Specific Variables

```bash
# Override PostgreSQL connection
export NIRV_CONNECTORS_POSTGRES_MAIN_CONNECTION_STRING="postgresql://newuser:newpass@newhost/newdb"

# Override API endpoint
export NIRV_CONNECTORS_API_SERVICE_BASE_URL="https://api-staging.example.com"
```

## Examples

### Basic Data Exploration

```bash
# List available data sources
nirv sources

# Explore a data source schema
nirv schema "mock.users"

# Query all data
nirv query "SELECT * FROM source('mock.users')"

# Count records
nirv query "SELECT COUNT(*) as total FROM source('mock.users')"
```

### Filtering and Sorting

```bash
# Filter by condition
nirv query "SELECT * FROM source('mock.users') WHERE age > 25"

# Sort results
nirv query "SELECT name, age FROM source('mock.users') ORDER BY age DESC"

# Limit results
nirv query "SELECT * FROM source('mock.users') LIMIT 5"

# Complex filtering
nirv query "SELECT name, email FROM source('mock.users') WHERE age BETWEEN 25 AND 35 AND email IS NOT NULL"
```

### Aggregation Queries

```bash
# Count by group
nirv query "SELECT age, COUNT(*) as count FROM source('mock.users') GROUP BY age"

# Average calculation
nirv query "SELECT AVG(age) as average_age FROM source('mock.users')"

# Multiple aggregations
nirv query "SELECT COUNT(*) as total, MIN(age) as min_age, MAX(age) as max_age FROM source('mock.users')"
```

### Cross-Source Queries

```bash
# Join different data sources (when cross-connector joins are enabled)
nirv query "
  SELECT u.name, o.total 
  FROM source('postgres.users') u
  JOIN source('api.orders') o ON u.id = o.user_id
  WHERE o.status = 'completed'
"

# Union data from multiple sources
nirv query "
  SELECT name, 'postgres' as source FROM source('postgres.users')
  UNION ALL
  SELECT name, 'api' as source FROM source('api.users')
"
```

### Output Format Examples

```bash
# Export to CSV file
nirv query "SELECT * FROM source('mock.users')" --format csv > users.csv

# Process JSON with jq
nirv query "SELECT name, age FROM source('mock.users')" --format json | jq '.data[] | select(.age > 30)'

# Pretty table for presentation
nirv query "SELECT name, age, email FROM source('mock.users') ORDER BY name" --format table
```

### Debugging and Troubleshooting

```bash
# Enable verbose mode for debugging
nirv query "SELECT * FROM source('mock.users')" --verbose

# Check data source connectivity
nirv sources --detailed

# Validate query syntax
nirv query "SELECT * FROM source('mock.users') WHERE invalid_column = 1" --verbose
```

## Troubleshooting

### Common Error Messages

#### Query Parsing Errors

**Error:** `Query parsing error: Expected 'source(' function call`
```bash
# Incorrect
nirv query "SELECT * FROM users"

# Correct
nirv query "SELECT * FROM source('mock.users')"
```

**Error:** `Query parsing error: Unexpected token at line 1, column 15`
```bash
# Check SQL syntax
nirv query "SELECT * FROM source('mock.users') WHERE age > 25" --verbose
```

#### Data Source Errors

**Error:** `Dispatcher error: Data object type 'unknown' is not registered`
```bash
# Check available sources
nirv sources

# Use correct source identifier
nirv query "SELECT * FROM source('mock.users')"  # not 'unknown.users'
```

**Error:** `Connector error: Failed to connect to PostgreSQL database`
```bash
# Check configuration
nirv sources --detailed

# Verify connection string in config file
cat config.toml | grep -A 3 "postgres"
```

#### Configuration Errors

**Error:** `Configuration error: Invalid port number in protocol_adapters[0]`
```bash
# Validate configuration file
nirv query "SELECT 1" --config config.toml --verbose
```

**Error:** `Permission denied when reading configuration file`
```bash
# Check file permissions
ls -la config.toml
chmod 644 config.toml
```

### Debug Mode

Enable verbose output to see detailed execution information:

```bash
nirv query "SELECT * FROM source('mock.users')" --verbose
```

**Verbose Output Example:**
```
Info: Parsing query: SELECT * FROM source('mock.users')
Info: Query parsed successfully. Sources: ["mock.users"]
Info: Query routed to 1 connector(s)
Info: Query executed successfully. 3 rows returned

id | name     | email              | age | created_at
---|----------|--------------------|----|------------------
1  | John Doe | john@example.com   | 30 | 2024-01-15 10:30:00
2  | Jane Doe | jane@example.com   | 25 | 2024-01-16 14:20:00
3  | Bob Smith| bob@example.com    | 35 | 2024-01-17 09:15:00

3 rows in 12.45ms
```

### Performance Tips

1. **Use LIMIT clauses** for large datasets to avoid memory issues
2. **Enable verbose mode** to identify slow operations
3. **Use appropriate output formats** (CSV for large exports, JSON for processing)
4. **Filter data early** with WHERE clauses to reduce network transfer
5. **Check connector capabilities** with `nirv sources --detailed`

### Getting Help

- Use `--help` with any command for detailed usage information
- Check the main documentation at [docs.nirv.dev](https://docs.nirv.dev)
- Report issues at [GitHub Issues](https://github.com/nirv/nirv-engine/issues)
- Join the community at [Discord](https://discord.gg/nirv)