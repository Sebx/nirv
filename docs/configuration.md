# NIRV Engine Configuration Reference (v0.1.0)

This document provides a comprehensive reference for configuring NIRV Engine.

## Current Status
- ✅ **Production Ready** - All configuration options functional
- ✅ **Multi-Protocol Support** - PostgreSQL, MySQL, SQLite protocol adapters
- ✅ **Multi-Connector Support** - Database, API, file system connectors
- ✅ **Security Features** - Authentication, authorization, audit logging
- ✅ **Performance Tuning** - Connection pooling, caching, optimization

## Table of Contents

- [Configuration Overview](#configuration-overview)
- [Configuration File Format](#configuration-file-format)
- [Protocol Adapters](#protocol-adapters)
- [Connectors](#connectors)
- [Dispatcher Configuration](#dispatcher-configuration)
- [Security Configuration](#security-configuration)
- [Environment Variables](#environment-variables)
- [Configuration Examples](#configuration-examples)
- [Validation and Troubleshooting](#validation-and-troubleshooting)

## Configuration Overview

NIRV Engine uses TOML configuration files to define:
- Protocol adapters (PostgreSQL, MySQL, SQLite)
- Data source connectors (databases, APIs, files)
- Security settings (authentication, authorization, audit)
- Performance tuning (connection pools, timeouts, caching)

### Configuration Loading Order

1. Built-in defaults
2. Configuration file (specified with `--config` or `NIRV_CONFIG`)
3. Environment variable overrides

### Configuration File Locations

NIRV searches for configuration files in this order:
1. File specified with `--config` option
2. `NIRV_CONFIG` environment variable
3. `./nirv.toml` (current directory)
4. `~/.nirv/config.toml` (user home)
5. `/etc/nirv/config.toml` (system-wide)

## Configuration File Format

NIRV Engine uses TOML (Tom's Obvious, Minimal Language) for configuration files.

### Basic Structure

```toml
# Protocol adapters define which database protocols to support
[[protocol_adapters]]
protocol_type = "PostgreSQL"
bind_address = "127.0.0.1"
port = 5432

# Connectors define connections to data sources
[connectors.my_postgres]
connector_type = "PostgreSQL"
connection_string = "postgresql://user:pass@host/db"

# Dispatcher controls query routing and execution
[dispatcher]
max_concurrent_queries = 100

# Security settings
[security.authentication]
enabled = false
```

## Protocol Adapters

Protocol adapters implement standard database wire protocols for client compatibility.

### PostgreSQL Protocol Adapter

```toml
[[protocol_adapters]]
protocol_type = "PostgreSQL"
bind_address = "127.0.0.1"      # IP address to bind to
port = 5432                     # Port number (1-65535)
max_connections = 100           # Maximum concurrent connections
connection_timeout = 30         # Connection timeout in seconds

# Optional TLS configuration
[protocol_adapters.tls_config]
cert_file = "/path/to/cert.pem"
key_file = "/path/to/key.pem"
ca_file = "/path/to/ca.pem"     # Optional CA certificate
require_client_cert = false     # Require client certificates
```

### MySQL Protocol Adapter

```toml
[[protocol_adapters]]
protocol_type = "MySQL"
bind_address = "0.0.0.0"
port = 3306
max_connections = 50
connection_timeout = 30
```

### SQLite Protocol Adapter

```toml
[[protocol_adapters]]
protocol_type = "SQLite"
bind_address = "127.0.0.1"
port = 5433
max_connections = 10
connection_timeout = 30
```

### Protocol Adapter Options

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `protocol_type` | string | - | Protocol type: "PostgreSQL", "MySQL", "SQLite" |
| `bind_address` | string | "127.0.0.1" | IP address to bind to |
| `port` | integer | varies | Port number (1-65535) |
| `max_connections` | integer | 100 | Maximum concurrent connections |
| `connection_timeout` | integer | 30 | Connection timeout in seconds |

## Connectors

Connectors provide access to various data sources through a unified interface.

### PostgreSQL Connector

```toml
[connectors.postgres_main]
connector_type = "PostgreSQL"
connection_string = "postgresql://username:password@hostname:port/database"

# Connection pool configuration
[connectors.postgres_main.pool_config]
min_connections = 2             # Minimum pool size
max_connections = 20            # Maximum pool size
connection_timeout = 30         # Connection timeout (seconds)
idle_timeout = 600             # Idle connection timeout (seconds)
max_lifetime = 3600            # Maximum connection lifetime (seconds)

# Query timeout configuration
[connectors.postgres_main.timeout_config]
connect_timeout = 30           # Connection establishment timeout
query_timeout = 300            # Query execution timeout
transaction_timeout = 600      # Transaction timeout
```

### MySQL Connector

```toml
[connectors.mysql_analytics]
connector_type = "MySQL"
connection_string = "mysql://user:password@hostname:port/database"

[connectors.mysql_analytics.pool_config]
min_connections = 1
max_connections = 10
connection_timeout = 30
idle_timeout = 600
max_lifetime = 3600
```

### File Connector

```toml
[connectors.local_files]
connector_type = "File"
parameters = { 
    base_path = "/data/files",          # Base directory for files
    formats = ["csv", "json", "parquet"], # Supported file formats
    encoding = "utf-8",                 # Text file encoding
    delimiter = ",",                    # CSV delimiter (optional)
    has_header = true                   # CSV has header row (optional)
}
```

### REST API Connector

```toml
[connectors.api_service]
connector_type = "REST"
parameters = { 
    base_url = "https://api.example.com/v1",  # API base URL
    api_key = "your-api-key-here",            # API key for authentication
    timeout = "30",                           # Request timeout (seconds)
    retry_attempts = "3",                     # Number of retry attempts
    rate_limit = "100",                       # Requests per minute
    auth_header = "Authorization",            # Authentication header name
    auth_prefix = "Bearer "                   # Authentication prefix
}
```

### S3 Connector

```toml
[connectors.s3_data]
connector_type = "S3"
parameters = {
    bucket = "my-data-bucket",
    region = "us-west-2",
    access_key_id = "AKIAIOSFODNN7EXAMPLE",
    secret_access_key = "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY",
    endpoint = "https://s3.amazonaws.com",    # Optional custom endpoint
    path_style = false                        # Use path-style URLs
}
```

### Mock Connector (Testing)

```toml
[connectors.mock]
connector_type = "Mock"
# Mock connector uses built-in test data
```

### Connector Options

#### Common Options

| Option | Type | Description |
|--------|------|-------------|
| `connector_type` | string | Connector type identifier |
| `connection_string` | string | Database connection string (for DB connectors) |
| `parameters` | table | Connector-specific parameters |

#### Pool Configuration

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `min_connections` | integer | 1 | Minimum pool size |
| `max_connections` | integer | 10 | Maximum pool size |
| `connection_timeout` | integer | 30 | Connection timeout (seconds) |
| `idle_timeout` | integer | 600 | Idle connection timeout (seconds) |
| `max_lifetime` | integer | 3600 | Maximum connection lifetime (seconds) |

#### Timeout Configuration

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `connect_timeout` | integer | 30 | Connection establishment timeout |
| `query_timeout` | integer | 300 | Query execution timeout |
| `transaction_timeout` | integer | 600 | Transaction timeout |

## Dispatcher Configuration

The dispatcher controls query routing, execution, and performance optimization.

```toml
[dispatcher]
max_concurrent_queries = 100        # Maximum parallel queries
query_cache_size = 256             # Query result cache size (MB)
enable_cross_connector_joins = true # Allow joins across data sources
default_timeout = 300              # Default query timeout (seconds)
connection_pool_size = 50          # Global connection pool size
enable_query_optimization = true    # Enable query optimization
```

### Dispatcher Options

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `max_concurrent_queries` | integer | 100 | Maximum parallel queries |
| `query_cache_size` | integer | 256 | Cache size in MB (optional) |
| `enable_cross_connector_joins` | boolean | false | Allow cross-source joins |
| `default_timeout` | integer | 300 | Default timeout in seconds |
| `connection_pool_size` | integer | 50 | Global connection pool size |
| `enable_query_optimization` | boolean | true | Enable query optimization |

## Security Configuration

Security settings control authentication, authorization, and audit logging.

### Authentication Configuration

```toml
[security.authentication]
enabled = true                      # Enable authentication
auth_method = "Password"            # Authentication method
user_database = "/etc/nirv/users.db" # User database file

# LDAP configuration (when auth_method = "LDAP")
[security.authentication.ldap_config]
server_url = "ldap://ldap.example.com:389"
bind_dn = "cn=admin,dc=example,dc=com"
bind_password = "admin_password"
user_search_base = "ou=users,dc=example,dc=com"
user_search_filter = "(uid={username})"

# OAuth2 configuration (when auth_method = "OAuth2")
[security.authentication.oauth2_config]
client_id = "your-client-id"
client_secret = "your-client-secret"
auth_url = "https://auth.example.com/oauth2/authorize"
token_url = "https://auth.example.com/oauth2/token"
```

### Authentication Methods

| Method | Description | Configuration |
|--------|-------------|---------------|
| `None` | No authentication | Default, no additional config |
| `Password` | Username/password | Requires user database |
| `Certificate` | Client certificates | Requires TLS configuration |
| `LDAP` | LDAP authentication | Requires LDAP configuration |
| `OAuth2` | OAuth2 authentication | Requires OAuth2 configuration |

### Authorization Configuration

```toml
[security.authorization]
enabled = true                      # Enable authorization
default_permissions = ["Read"]      # Default permissions for users

# Role-based permissions
[security.authorization.role_mappings]
admin = ["Read", "Write", "Admin", "Connect"]
user = ["Read", "Connect"]
readonly = ["Read"]
guest = []
```

### Permission Types

| Permission | Description |
|------------|-------------|
| `Read` | Execute SELECT queries |
| `Write` | Execute INSERT, UPDATE, DELETE queries |
| `Admin` | Administrative operations |
| `Connect` | Establish connections |

### Audit Logging Configuration

```toml
[security.audit_logging]
enabled = true                      # Enable audit logging
log_file = "/var/log/nirv/audit.log" # Log file path (optional)
log_queries = true                  # Log executed queries
log_connections = true              # Log connection attempts
log_errors = true                   # Log error events
log_level = "info"                  # Log level: debug, info, warn, error
max_log_size = 100                  # Maximum log file size (MB)
log_rotation = true                 # Enable log rotation
```

## Environment Variables

Configuration values can be overridden using environment variables with the `NIRV_` prefix.

### General Variables

| Variable | Description | Example |
|----------|-------------|---------|
| `NIRV_CONFIG` | Configuration file path | `/etc/nirv/config.toml` |
| `NIRV_LOG_LEVEL` | Global log level | `debug`, `info`, `warn`, `error` |

### Protocol Adapter Variables

```bash
# Override PostgreSQL port
export NIRV_PROTOCOL_ADAPTERS_0_PORT=5433

# Override MySQL bind address
export NIRV_PROTOCOL_ADAPTERS_1_BIND_ADDRESS="0.0.0.0"
```

### Connector Variables

```bash
# Override PostgreSQL connection string
export NIRV_CONNECTORS_POSTGRES_MAIN_CONNECTION_STRING="postgresql://newuser:newpass@newhost/newdb"

# Override API base URL
export NIRV_CONNECTORS_API_SERVICE_PARAMETERS_BASE_URL="https://api-staging.example.com"

# Override file connector base path
export NIRV_CONNECTORS_LOCAL_FILES_PARAMETERS_BASE_PATH="/new/data/path"
```

### Dispatcher Variables

```bash
# Override concurrent query limit
export NIRV_DISPATCHER_MAX_CONCURRENT_QUERIES=200

# Enable cross-connector joins
export NIRV_DISPATCHER_ENABLE_CROSS_CONNECTOR_JOINS=true
```

### Security Variables

```bash
# Enable authentication
export NIRV_SECURITY_AUTHENTICATION_ENABLED=true

# Set authentication method
export NIRV_SECURITY_AUTHENTICATION_AUTH_METHOD="LDAP"

# Override LDAP server
export NIRV_SECURITY_AUTHENTICATION_LDAP_CONFIG_SERVER_URL="ldap://new-ldap.example.com"
```

## Configuration Examples

### Development Configuration

```toml
# dev-config.toml - Development environment
[[protocol_adapters]]
protocol_type = "PostgreSQL"
bind_address = "127.0.0.1"
port = 5432
max_connections = 10

[connectors.mock]
connector_type = "Mock"

[connectors.dev_postgres]
connector_type = "PostgreSQL"
connection_string = "postgresql://dev:dev@localhost:5432/dev_db"

[connectors.dev_postgres.pool_config]
min_connections = 1
max_connections = 5

[dispatcher]
max_concurrent_queries = 10
enable_cross_connector_joins = false

[security.authentication]
enabled = false

[security.audit_logging]
enabled = true
log_queries = true
log_connections = false
```

### Production Configuration

```toml
# prod-config.toml - Production environment
[[protocol_adapters]]
protocol_type = "PostgreSQL"
bind_address = "0.0.0.0"
port = 5432
max_connections = 1000
connection_timeout = 30

[protocol_adapters.tls_config]
cert_file = "/etc/ssl/certs/nirv.crt"
key_file = "/etc/ssl/private/nirv.key"
require_client_cert = false

[[protocol_adapters]]
protocol_type = "MySQL"
bind_address = "0.0.0.0"
port = 3306
max_connections = 500

[connectors.prod_postgres]
connector_type = "PostgreSQL"
connection_string = "postgresql://nirv_user:secure_password@db.example.com:5432/production"

[connectors.prod_postgres.pool_config]
min_connections = 10
max_connections = 100
connection_timeout = 30
idle_timeout = 600
max_lifetime = 3600

[connectors.prod_postgres.timeout_config]
connect_timeout = 30
query_timeout = 300
transaction_timeout = 600

[connectors.analytics_mysql]
connector_type = "MySQL"
connection_string = "mysql://analytics:password@analytics.example.com:3306/analytics"

[connectors.analytics_mysql.pool_config]
min_connections = 5
max_connections = 50

[connectors.api_service]
connector_type = "REST"
parameters = { 
    base_url = "https://api.example.com/v1",
    api_key = "prod-api-key-here",
    timeout = "30",
    retry_attempts = "3"
}

[dispatcher]
max_concurrent_queries = 500
query_cache_size = 1024
enable_cross_connector_joins = true
default_timeout = 300

[security.authentication]
enabled = true
auth_method = "LDAP"

[security.authentication.ldap_config]
server_url = "ldaps://ldap.example.com:636"
bind_dn = "cn=nirv,ou=services,dc=example,dc=com"
bind_password = "service_password"
user_search_base = "ou=users,dc=example,dc=com"
user_search_filter = "(uid={username})"

[security.authorization]
enabled = true
default_permissions = ["Read"]

[security.authorization.role_mappings]
admin = ["Read", "Write", "Admin", "Connect"]
analyst = ["Read", "Connect"]
readonly = ["Read"]

[security.audit_logging]
enabled = true
log_file = "/var/log/nirv/audit.log"
log_queries = true
log_connections = true
log_errors = true
max_log_size = 100
log_rotation = true
```

### Multi-Environment Configuration

```toml
# multi-env-config.toml - Multiple environments
[[protocol_adapters]]
protocol_type = "PostgreSQL"
bind_address = "0.0.0.0"
port = 5432

# Development database
[connectors.dev_db]
connector_type = "PostgreSQL"
connection_string = "postgresql://dev:dev@dev-db.example.com/dev"

# Staging database
[connectors.staging_db]
connector_type = "PostgreSQL"
connection_string = "postgresql://staging:staging@staging-db.example.com/staging"

# Production database (read-only)
[connectors.prod_readonly]
connector_type = "PostgreSQL"
connection_string = "postgresql://readonly:readonly@prod-db.example.com/production"

# File-based data sources
[connectors.local_files]
connector_type = "File"
parameters = { 
    base_path = "/data/shared",
    formats = ["csv", "json", "parquet"]
}

# External API
[connectors.external_api]
connector_type = "REST"
parameters = { 
    base_url = "https://external-api.example.com",
    api_key = "external-api-key"
}

[dispatcher]
max_concurrent_queries = 100
enable_cross_connector_joins = true

[security.authentication]
enabled = true
auth_method = "Password"
user_database = "/etc/nirv/users.db"

[security.audit_logging]
enabled = true
log_file = "/var/log/nirv/audit.log"
log_queries = true
log_connections = true
```

## Validation and Troubleshooting

### Configuration Validation

NIRV Engine validates configuration on startup:

```bash
# Test configuration file
nirv query "SELECT 1" --config config.toml --verbose

# Validate specific sections
nirv validate-config config.toml
```

### Common Configuration Errors

#### Invalid TOML Syntax

```
Error: Configuration error: Invalid TOML syntax at line 15: Expected '=' after key
```

**Solution:** Validate TOML syntax using online validators or TOML-aware editors.

#### Invalid Port Numbers

```
Error: Configuration error: Invalid port number in protocol_adapters[0]: port must be between 1 and 65535
```

**Solution:** Use valid port numbers (1-65535) and ensure ports are not already in use.

#### Missing Required Fields

```
Error: Configuration error: Missing required field 'connector_type' in connectors.postgres_main
```

**Solution:** Ensure all required fields are present in connector configurations.

#### Invalid Connection Strings

```
Error: Configuration error: Invalid connection string format for PostgreSQL connector
```

**Solution:** Use correct connection string format for each connector type.

### Configuration Testing

```bash
# Test database connectivity
nirv sources --detailed --config config.toml

# Test specific connector
nirv query "SELECT 1" --config config.toml --verbose

# Test authentication
nirv query "SELECT 1" --config config-with-auth.toml
```

### Performance Tuning

#### Connection Pool Sizing

- **Min connections:** Start with 1-2 per connector
- **Max connections:** Based on database limits and expected load
- **Idle timeout:** 10-15 minutes for most use cases
- **Max lifetime:** 1-2 hours to prevent connection leaks

#### Query Performance

- **Cache size:** 256-1024 MB for typical workloads
- **Concurrent queries:** Based on system resources and connector limits
- **Timeouts:** Balance between user experience and resource usage

#### Memory Usage

- **Query cache:** Monitor memory usage and adjust cache size
- **Connection pools:** Limit total connections across all connectors
- **Result streaming:** Enable for large result sets

### Monitoring Configuration

```toml
# Add monitoring and metrics
[monitoring]
enabled = true
metrics_port = 9090
health_check_interval = 30

[monitoring.prometheus]
enabled = true
endpoint = "/metrics"

[monitoring.logging]
level = "info"
format = "json"
output = "/var/log/nirv/nirv.log"
```

This configuration reference provides comprehensive coverage of all NIRV Engine configuration options. For specific use cases or advanced configurations, consult the main documentation or community resources.