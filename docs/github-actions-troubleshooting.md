# GitHub Actions Troubleshooting Guide

## Overview

This guide provides solutions to common issues encountered with the Nirv database engine GitHub Actions CI/CD pipeline. Issues are organized by category with step-by-step resolution procedures.

## Database Service Issues

### SQL Server Connection Failures

#### Symptoms
- Tests fail with connection timeout errors
- "Cannot connect to SQL Server" messages
- Health check failures for SQL Server service

#### Common Causes and Solutions

**1. Service Startup Timeout**
```
Error: CRITICAL: SQL Server service failed to start within 30 seconds
```

**Solution:**
- SQL Server container needs more time to initialize
- Check if the container image is being pulled for the first time
- Verify sufficient resources are available

**Debugging Steps:**
```bash
# Check service logs in workflow
docker logs <sqlserver-container-id>

# Verify service health
docker exec <container-id> /opt/mssql-tools/bin/sqlcmd -S localhost -U sa -P YourStrong@Passw0rd -Q "SELECT 1"
```

**2. Password Policy Violations**
```
Error: Password validation failed
```

**Solution:**
- Ensure password meets SQL Server complexity requirements
- Current password: `YourStrong@Passw0rd` meets requirements
- If changing, ensure: 8+ characters, uppercase, lowercase, number, special character

**3. Port Conflicts**
```
Error: Port 1433 already in use
```

**Solution:**
- Check for conflicting services in the runner
- Verify port mapping in workflow configuration
- Consider using dynamic port allocation if needed

### PostgreSQL Connection Issues

#### Symptoms
- PostgreSQL connection refused
- Authentication failures
- Database does not exist errors

#### Solutions

**1. Service Not Ready**
```
Error: connection refused to localhost:5432
```

**Solution:**
```yaml
# Ensure proper health check configuration
options: >-
  --health-cmd pg_isready
  --health-interval 10s
  --health-timeout 5s
  --health-retries 5
```

**2. Database Creation Issues**
```
Error: database "testdb" does not exist
```

**Solution:**
- Verify `POSTGRES_DB` environment variable is set
- Check database initialization in service configuration
- Ensure connection string matches database name

**3. Authentication Problems**
```
Error: password authentication failed for user "postgres"
```

**Solution:**
- Verify `POSTGRES_PASSWORD` environment variable
- Check connection string credentials
- Ensure user permissions are correctly configured

### MySQL Connection Problems

#### Symptoms
- MySQL connection denied
- Access denied for user errors
- Unknown database errors

#### Solutions

**1. Service Initialization Delay**
```
Error: Can't connect to MySQL server on 'localhost'
```

**Solution:**
- MySQL takes time to initialize, especially on first run
- Increase health check retry count
- Add explicit wait in test steps

**2. User Permission Issues**
```
Error: Access denied for user 'testuser'@'localhost'
```

**Solution:**
- Verify `MYSQL_USER` and `MYSQL_PASSWORD` environment variables
- Check that user creation completed successfully
- Ensure database grants are properly applied

**3. Database Not Found**
```
Error: Unknown database 'testdb'
```

**Solution:**
- Verify `MYSQL_DATABASE` environment variable
- Check database creation in service logs
- Ensure connection string uses correct database name

## Cache-Related Issues

### Cache Miss Problems

#### Symptoms
- Consistently low cache hit rates (<50%)
- Builds taking longer than expected
- "Cache not found" messages in logs

#### Solutions

**1. Cache Key Issues**
```
Warning: Cache hit rate below 50%
```

**Debugging:**
```bash
# Check cache key generation
echo "cargo-${{ runner.os }}-${{ hashFiles('**/Cargo.lock') }}"

# Verify file hashes
sha256sum Cargo.lock
find src -name "*.rs" -type f -exec sha256sum {} \; | sort | sha256sum
```

**Solution:**
- Ensure Cargo.lock is committed to repository
- Verify cache key patterns match across jobs
- Check for frequent dependency changes

**2. Cache Corruption**
```
Error: Failed to restore cache
```

**Solution:**
- Clear cache manually through GitHub Actions interface
- Update cache keys to force regeneration
- Check for disk space issues in runner

**3. Cache Size Limits**
```
Warning: Cache size exceeds limits
```

**Solution:**
- Review cached directories for unnecessary files
- Exclude large temporary files from cache paths
- Consider splitting cache into smaller segments

### Cache Invalidation Issues

#### Symptoms
- Stale build artifacts
- Tests passing locally but failing in CI
- Inconsistent build results

#### Solutions

**1. Dependency Cache Not Invalidated**
```bash
# Manual cache invalidation
rm -rf ~/.cargo/registry/cache
rm -rf target/debug/deps target/debug/build
```

**2. Source Cache Staleness**
- Verify source hash calculation includes all relevant files
- Check that cache keys properly incorporate source changes
- Ensure cache restore keys are appropriately ordered

## Code Quality Check Failures

### Formatting Issues

#### Symptoms
```
Error: Code formatting does not comply with rustfmt standards
```

#### Solutions

**1. Fix Formatting Locally**
```bash
# Format all code
cargo fmt

# Check formatting without making changes
cargo fmt --check

# Format specific files
cargo fmt src/main.rs
```

**2. Configuration Issues**
- Check for `.rustfmt.toml` configuration file
- Verify rustfmt version consistency
- Ensure all team members use same rustfmt settings

### Clippy Violations

#### Symptoms
```
Error: Code contains clippy warnings that must be resolved
```

#### Solutions

**1. Fix Clippy Warnings**
```bash
# Run clippy with same settings as CI
cargo clippy --all-targets --all-features -- -D warnings -D clippy::all -D clippy::pedantic

# Fix specific warnings
cargo clippy --fix --all-targets --all-features
```

**2. Suppress Specific Warnings (Use Sparingly)**
```rust
#[allow(clippy::pedantic)]
fn legacy_function() {
    // Code that can't be easily updated
}
```

**3. Configuration Management**
```toml
# clippy.toml
avoid-breaking-exported-api = false
cognitive-complexity-threshold = 30
```

### Security Audit Failures

#### Symptoms
```
Error: Dependencies contain known security vulnerabilities
```

#### Solutions

**1. Update Vulnerable Dependencies**
```bash
# Check for updates
cargo update

# Update specific dependency
cargo update -p vulnerable_crate

# Check audit status
cargo audit
```

**2. Handle False Positives**
```toml
# .cargo/audit.toml
[advisories]
ignore = [
    "RUSTSEC-2020-0001",  # Specific advisory to ignore
]

[bans]
deny = [
    "vulnerable_crate",   # Completely ban a crate
]
```

**3. Temporary Workarounds**
- Document why vulnerability is not applicable
- Set timeline for proper fix
- Monitor for updates to vulnerable dependencies

### Documentation Issues

#### Symptoms
```
Error: Missing documentation detected for public API items
```

#### Solutions

**1. Add Missing Documentation**
```rust
/// This function performs database connection validation
/// 
/// # Arguments
/// * `connection_string` - The database connection string to validate
/// 
/// # Returns
/// * `Result<(), Error>` - Ok if connection is valid, Error otherwise
pub fn validate_connection(connection_string: &str) -> Result<(), Error> {
    // Implementation
}
```

**2. Configure Documentation Requirements**
```rust
#![warn(missing_docs)]
#![warn(rustdoc::broken_intra_doc_links)]
```

**3. Generate Documentation Locally**
```bash
# Generate and open documentation
cargo doc --open --no-deps --all-features

# Check for warnings
cargo doc --no-deps --all-features 2>&1 | grep warning
```

### Coverage Issues

#### Symptoms
```
Error: Code coverage below minimum threshold
```

#### Solutions

**1. Add Missing Tests**
```bash
# Run coverage locally
cargo tarpaulin --all-features --out html

# Open coverage report
open tarpaulin-report.html
```

**2. Identify Uncovered Code**
- Review HTML coverage report
- Focus on critical functionality first
- Add unit tests for uncovered functions
- Add integration tests for uncovered workflows

**3. Adjust Coverage Thresholds**
```yaml
# In workflow, adjust if necessary
MIN_COVERAGE=65  # Temporary reduction
TARGET_COVERAGE=80  # Adjusted target
```

## Performance Issues

### Pipeline Timeout Problems

#### Symptoms
- Jobs exceeding time thresholds
- Pipeline taking longer than 25 minutes total
- Resource contention warnings

#### Solutions

**1. Optimize Parallel Execution**
```yaml
# Adjust parallelization settings
strategy:
  max-parallel: 4  # Increase if resources allow
  
# Optimize resource allocation
env:
  CARGO_BUILD_JOBS: 2  # Adjust based on available CPU
```

**2. Cache Optimization**
- Monitor cache hit rates
- Optimize cache key strategies
- Consider cache warming for frequently used dependencies

**3. Test Suite Optimization**
- Split large test suites
- Optimize database test parallelization
- Remove or optimize slow tests

### Resource Contention

#### Symptoms
- Database connection failures under load
- Memory exhaustion errors
- Disk space issues

#### Solutions

**1. Database Connection Limits**
```yaml
# Reduce parallel database tests
strategy:
  max-parallel: 1  # For database-heavy test suites
```

**2. Memory Management**
```bash
# Monitor memory usage
free -h

# Optimize Rust compilation memory usage
export CARGO_BUILD_JOBS=1  # Reduce parallel compilation
```

**3. Disk Space Management**
```bash
# Clean up build artifacts
cargo clean

# Remove unused Docker images
docker system prune -f
```

## Release Process Issues

### Release Validation Failures

#### Symptoms
- Pre-release tests failing
- Version mismatch errors
- Changelog validation failures

#### Solutions

**1. Version Consistency**
```bash
# Check version consistency
grep version Cargo.toml
git tag --list | tail -5

# Fix version mismatch
# Update Cargo.toml version to match tag
```

**2. Changelog Issues**
```markdown
# Ensure changelog entry exists
## [1.0.0] - 2024-01-15
### Added
- New feature description
### Fixed
- Bug fix description
```

**3. Test Environment Issues**
- Ensure all database services start properly
- Verify test data is properly initialized
- Check for environment-specific test failures

### Artifact Build Failures

#### Symptoms
- Cross-compilation errors
- Missing dependencies on target platforms
- Checksum generation failures

#### Solutions

**1. Cross-Compilation Issues**
```bash
# Install target toolchain
rustup target add x86_64-pc-windows-msvc

# Test cross-compilation locally
cargo build --target x86_64-pc-windows-msvc
```

**2. Platform-Specific Dependencies**
```toml
# Cargo.toml - conditional dependencies
[target.'cfg(windows)'.dependencies]
winapi = "0.3"

[target.'cfg(unix)'.dependencies]
libc = "0.2"
```

**3. Checksum Verification**
```bash
# Verify checksums locally
sha256sum artifact.tar.gz
sha256sum -c artifact.tar.gz.sha256
```

## General Debugging Strategies

### Workflow Debugging

**1. Enable Debug Logging**
```yaml
env:
  ACTIONS_STEP_DEBUG: true
  ACTIONS_RUNNER_DEBUG: true
```

**2. Add Debug Steps**
```yaml
- name: Debug Environment
  run: |
    echo "Runner OS: ${{ runner.os }}"
    echo "GitHub Event: ${{ github.event_name }}"
    echo "Ref: ${{ github.ref }}"
    env | sort
```

**3. Reproduce Locally**
```bash
# Use act to run GitHub Actions locally
act -j test

# Run specific job
act -j quality
```

### Service Debugging

**1. Check Service Logs**
```yaml
- name: Debug Services
  if: failure()
  run: |
    docker ps -a
    docker logs $(docker ps -q --filter ancestor=postgres:15)
```

**2. Test Service Connectivity**
```yaml
- name: Test Database Connectivity
  run: |
    nc -zv localhost 5432
    nc -zv localhost 1433
    nc -zv localhost 3306
```

### Cache Debugging

**1. Cache Analysis**
```yaml
- name: Debug Cache
  run: |
    echo "Cache key: cargo-${{ runner.os }}-${{ hashFiles('**/Cargo.lock') }}"
    ls -la ~/.cargo/registry/cache/ || echo "No registry cache"
    ls -la target/ || echo "No build cache"
```

**2. Manual Cache Management**
```bash
# Clear specific cache
gh api -X DELETE /repos/OWNER/REPO/actions/caches/CACHE_ID

# List all caches
gh api /repos/OWNER/REPO/actions/caches
```

## Getting Help

### Internal Resources

1. **Workflow Logs**: Check detailed logs in GitHub Actions interface
2. **Performance Reports**: Review pipeline performance metrics
3. **Cache Reports**: Analyze cache effectiveness data

### External Resources

1. **GitHub Actions Documentation**: https://docs.github.com/en/actions
2. **Rust CI/CD Best Practices**: https://doc.rust-lang.org/cargo/guide/continuous-integration.html
3. **Docker Service Containers**: https://docs.github.com/en/actions/using-containerized-services

### Escalation Process

1. **Check Known Issues**: Review this troubleshooting guide
2. **Search Workflow Logs**: Look for specific error patterns
3. **Reproduce Locally**: Try to replicate the issue in local environment
4. **Create Issue**: Document the problem with logs and reproduction steps
5. **Contact Maintainers**: Reach out to project maintainers with detailed information

## Prevention Strategies

### Proactive Monitoring

1. **Set up Alerts**: Monitor pipeline performance trends
2. **Regular Reviews**: Weekly review of cache effectiveness and performance
3. **Dependency Updates**: Monthly dependency update cycles
4. **Security Scans**: Regular security audit reviews

### Best Practices

1. **Test Locally**: Always test changes locally before pushing
2. **Incremental Changes**: Make small, focused changes to workflows
3. **Documentation**: Keep workflow documentation up to date
4. **Monitoring**: Regularly review pipeline metrics and performance

This troubleshooting guide should help resolve most common issues encountered with the GitHub Actions CI/CD pipeline. For issues not covered here, follow the escalation process to get additional support.