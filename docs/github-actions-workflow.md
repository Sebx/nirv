# GitHub Actions CI/CD Workflow Documentation

## Overview

The Nirv database engine project uses an enhanced GitHub Actions CI/CD pipeline that provides comprehensive testing, code quality enforcement, and automated release management. This document describes the workflow features, configuration, and troubleshooting procedures.

## Workflow Structure

### Main Workflow (`rust.yml`)

The main CI/CD workflow consists of four primary stages:

1. **Preparation Stage** - Environment setup and dependency caching
2. **Quality Assurance Stage** - Code formatting, linting, security, and documentation checks
3. **Testing Stage** - Unit tests, integration tests, and database-specific testing
4. **Performance Monitoring** - Pipeline performance analysis and reporting

### Release Workflow (`release.yml`)

The release workflow is triggered by version tags and includes:

1. **Pre-Release Validation** - Comprehensive testing before release
2. **Multi-Platform Builds** - Artifact generation for Linux, Windows, and macOS
3. **Draft Release Creation** - Automated release preparation with checksums
4. **Failure Handling** - Error recovery and notification system

## Key Features

### Database Testing with Service Containers

The workflow provisions containerized database services for comprehensive testing:

- **SQL Server** (`mcr.microsoft.com/mssql/server:2022-latest`)
- **PostgreSQL** (`postgres:15`)
- **MySQL** (`mysql:8.0`)
- **SQLite** (file-based, no container required)

#### Configuration

Database services are conditionally provisioned based on test suite requirements:

```yaml
services:
  sqlserver:
    image: ${{ (matrix.test-suite == 'sqlserver' || matrix.test-suite == 'integration') && 'mcr.microsoft.com/mssql/server:2022-latest' || '' }}
    env:
      ACCEPT_EULA: Y
      SA_PASSWORD: YourStrong@Passw0rd
      MSSQL_PID: Developer
    ports:
      - 1433:1433
    options: >-
      --health-cmd "/opt/mssql-tools/bin/sqlcmd -S localhost -U sa -P YourStrong@Passw0rd -Q 'SELECT 1' || exit 1"
      --health-interval 10s
      --health-timeout 5s
      --health-retries 5
      --health-start-period 10s
```

### Advanced Caching Strategy

The workflow implements a multi-level caching system:

#### Cache Types

1. **Cargo Registry Cache**
   - Path: `~/.cargo/registry/`
   - Key: `cargo-registry-{os}-{Cargo.lock hash}`
   - Stores downloaded crate metadata and source code

2. **Cargo Binaries Cache**
   - Path: `~/.cargo/bin/`
   - Key: `cargo-bin-{os}-{Cargo.toml hash}`
   - Stores compiled cargo tools (clippy, rustfmt, etc.)

3. **Build Artifacts Cache**
   - Path: `target/`
   - Key: `cargo-build-{os}-{Cargo.lock hash}-{source hash}`
   - Stores compiled dependencies and build outputs

#### Cache Invalidation

The workflow includes intelligent cache invalidation:

- **Dependency Changes**: Monitors `Cargo.lock` changes
- **Source Changes**: Tracks source file modifications
- **Automatic Cleanup**: Removes stale artifacts when dependencies change

### Code Quality Enforcement

#### Formatting Checks
- Uses `cargo fmt --check` with strict enforcement
- Fails the build on any formatting violations
- No `continue-on-error` - formatting must be correct

#### Clippy Analysis
- Treats all warnings as errors with `-D warnings`
- Includes pedantic lints with `-D clippy::pedantic`
- Comprehensive analysis across all targets and features

#### Security Auditing
- Uses `cargo audit` to scan for vulnerabilities
- Denies warnings, unmaintained crates, unsound crates, and yanked crates
- Updates advisory database before scanning

#### Documentation Validation
- Checks for missing documentation on public APIs
- Validates intra-doc links
- Generates documentation coverage reports
- Enforces minimum documentation standards

#### Code Coverage
- Uses `cargo-tarpaulin` for coverage analysis
- Minimum coverage threshold: 70%
- Target coverage threshold: 85%
- Generates HTML and XML reports

### Parallel Test Execution

The workflow optimizes test execution through strategic parallelization:

#### Test Suite Matrix

```yaml
strategy:
  fail-fast: false
  max-parallel: 2
  matrix:
    test-suite: [unit, integration, sqlserver, protocol]
```

#### Resource Allocation

- **Unit Tests**: High priority, full CPU utilization
- **Integration Tests**: Medium priority, controlled parallelization
- **Database Tests**: Low priority, limited parallelization to prevent contention

### Performance Monitoring

The workflow includes comprehensive performance monitoring:

#### Thresholds

- Preparation: 5 minutes maximum
- Quality checks: 10 minutes maximum
- Test suites: 15 minutes maximum
- Total pipeline: 25 minutes maximum

#### Metrics

- Cache hit rates and effectiveness
- Job duration tracking
- Resource utilization monitoring
- Performance trend analysis

## Environment Variables

### Database Connection Strings

```bash
# SQL Server
SQLSERVER_CONNECTION_STRING="Server=localhost,1433;Database=tempdb;User Id=sa;Password=YourStrong@Passw0rd;TrustServerCertificate=true;"

# PostgreSQL
POSTGRES_CONNECTION_STRING="postgresql://postgres:postgres@localhost:5432/testdb"

# MySQL
MYSQL_CONNECTION_STRING="mysql://testuser:testpassword@localhost:3306/testdb"
```

### Performance Configuration

```bash
CARGO_TERM_COLOR=always
RUST_BACKTRACE=1
MAX_PREPARE_TIME=5
MAX_QUALITY_TIME=10
MAX_TEST_TIME=15
MAX_TOTAL_PIPELINE_TIME=25
```

## Release Process

### Triggering a Release

1. Ensure all tests pass on the main branch
2. Update `CHANGELOG.md` with release notes
3. Verify package version matches intended release version
4. Create and push a version tag:

```bash
git tag v1.0.0
git push origin v1.0.0
```

### Release Validation

The release workflow performs comprehensive validation:

1. **Pre-Release Testing**: All test suites with database services
2. **Code Quality**: Format, clippy, and security checks
3. **Build Validation**: Release binary compilation and smoke tests
4. **Metadata Validation**: Version consistency and changelog verification

### Artifact Generation

Release artifacts are built for multiple platforms:

- **Linux**: `nirv-linux-x86_64.tar.gz`
- **Windows**: `nirv-windows-x86_64.zip`
- **macOS**: `nirv-macos-x86_64.tar.gz`

Each artifact includes:
- Compiled binary
- README, LICENSE, and CHANGELOG
- SHA256, SHA512, and MD5 checksums

### Draft Release Creation

The workflow automatically creates draft releases with:

- Extracted changelog content
- Release asset uploads
- Checksum verification information
- Installation and verification instructions

## Configuration Files

### Workflow Files

- `.github/workflows/rust.yml` - Main CI/CD pipeline
- `.github/workflows/release.yml` - Release automation

### Cache Configuration

Cache keys are automatically generated based on:
- Operating system
- Cargo.lock hash (for dependency caches)
- Source file hashes (for build caches)
- Cargo.toml hash (for binary caches)

### Service Health Checks

Database services include comprehensive health checks:

```yaml
options: >-
  --health-cmd "health_check_command"
  --health-interval 10s
  --health-timeout 5s
  --health-retries 5
  --health-start-period 10s
```

## Best Practices

### Development Workflow

1. **Local Testing**: Run tests locally before pushing
2. **Formatting**: Use `cargo fmt` to format code
3. **Linting**: Address clippy warnings with `cargo clippy`
4. **Documentation**: Document public APIs thoroughly
5. **Testing**: Maintain high test coverage

### Branch Protection

Recommended branch protection rules:

- Require status checks to pass
- Require branches to be up to date
- Require review from code owners
- Restrict pushes to main branch

### Security Considerations

- Database passwords are hardcoded for CI (acceptable for testing)
- Use secrets for production database connections
- Regularly update dependency versions
- Monitor security advisories

### Performance Optimization

- Monitor cache hit rates in workflow logs
- Optimize test parallelization based on resource usage
- Consider splitting large test suites if they exceed time thresholds
- Use appropriate resource priorities for different job types

## Monitoring and Maintenance

### Regular Maintenance Tasks

1. **Update Dependencies**: Regularly update Rust toolchain and dependencies
2. **Review Performance**: Monitor pipeline duration trends
3. **Cache Analysis**: Review cache effectiveness metrics
4. **Security Updates**: Keep database images and tools updated

### Performance Metrics

Monitor these key metrics:

- **Cache Hit Rate**: Target >80% for optimal performance
- **Pipeline Duration**: Should stay under 25 minutes total
- **Test Coverage**: Maintain >70% minimum, target >85%
- **Build Success Rate**: Monitor for reliability trends

### Alerts and Notifications

The workflow provides alerts for:

- Performance threshold violations
- Cache effectiveness degradation
- Security vulnerability detection
- Release process failures

## Integration with Development Tools

### IDE Integration

The workflow is designed to work with:

- **VS Code**: Rust analyzer integration
- **IntelliJ IDEA**: Rust plugin support
- **Vim/Neovim**: Rust language server

### Local Development

Developers can run equivalent checks locally:

```bash
# Format check
cargo fmt --check

# Clippy analysis
cargo clippy --all-targets --all-features -- -D warnings

# Security audit
cargo audit

# Documentation check
cargo doc --no-deps --all-features

# Code coverage
cargo tarpaulin --all-features
```

### Git Hooks

Consider setting up pre-commit hooks to run basic checks:

```bash
#!/bin/sh
# .git/hooks/pre-commit
cargo fmt --check && cargo clippy -- -D warnings
```

This documentation provides comprehensive guidance for understanding, using, and maintaining the enhanced GitHub Actions CI/CD pipeline for the Nirv database engine project.