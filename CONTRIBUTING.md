# Contributing to NIRV Engine (v0.1.0)

Thank you for your interest in contributing to NIRV Engine! This document provides guidelines and information for contributors.

## Current Project Status
- ✅ **Production Ready** - Core features stable and tested
- ✅ **Active Development** - Continuous improvements and new features
- ✅ **Community Driven** - Open to contributions and feedback
- 📊 **Test Coverage** - 119 tests with 96% pass rate
- 🚀 **Growing Ecosystem** - Multiple connectors and protocols supported

## Table of Contents

- [Code of Conduct](#code-of-conduct)
- [Getting Started](#getting-started)
- [Development Setup](#development-setup)
- [Contributing Guidelines](#contributing-guidelines)
- [Testing](#testing)
- [Documentation](#documentation)
- [Pull Request Process](#pull-request-process)
- [Issue Reporting](#issue-reporting)

## Code of Conduct

This project adheres to a code of conduct that we expect all contributors to follow. Please be respectful and constructive in all interactions.

## Getting Started

### Prerequisites

- Rust 1.70.0 or later
- Git
- Basic understanding of async Rust programming
- Familiarity with database protocols (helpful but not required)

### Development Setup

1. **Fork and Clone**
   ```bash
   git clone https://github.com/your-username/nirv-engine.git
   cd nirv-engine
   ```

2. **Install Dependencies**
   ```bash
   cargo build
   ```

3. **Run Tests**
   ```bash
   cargo test
   ```

4. **Run Examples**
   ```bash
   cargo run --example sqlserver_simulation
   ```

## Contributing Guidelines

### Current Priority Areas

We're actively seeking contributions in these areas:

#### 🔧 **Code Quality & Testing**
- Fix floating-point precision issues in query planner tests
- Clean up unused code and warnings
- Improve error handling and edge cases
- Add more integration test scenarios

#### 🚀 **Feature Development**
- Complete cross-connector join implementation
- MySQL connector implementation
- SQLite connector
- MongoDB connector
- Performance optimization

#### 📚 **Documentation & Examples**
- More usage examples and tutorials
- Performance tuning guides
- Architecture deep-dives
- Video tutorials and demos

### Areas for Contribution

We welcome contributions in the following areas:

#### 🔌 **New Connectors**
- MySQL connector implementation
- SQLite connector
- MongoDB connector
- Redis connector
- Custom database connectors

#### 🛠 **Protocol Adapters**
- MySQL wire protocol
- Redis protocol (RESP)
- GraphQL protocol adapter
- Custom protocol implementations

#### 🚀 **Engine Enhancements**
- Query optimization improvements
- Performance enhancements
- Memory usage optimizations
- Error handling improvements

#### 📚 **Documentation**
- API documentation improvements
- Tutorial and example creation
- Architecture documentation
- Performance guides

#### 🧪 **Testing**
- Integration test scenarios
- Performance benchmarks
- Protocol compliance tests
- Edge case coverage

### Code Style

We follow standard Rust conventions:

- Use `rustfmt` for code formatting
- Use `clippy` for linting
- Follow Rust naming conventions
- Write comprehensive documentation
- Include unit tests for new functionality

```bash
# Format code
cargo fmt

# Run clippy
cargo clippy

# Check documentation
cargo doc --no-deps --open
```

### Architecture Guidelines

#### Connector Implementation

When implementing a new connector:

1. **Implement the `Connector` trait**
   ```rust
   #[async_trait]
   impl Connector for YourConnector {
       async fn connect(&mut self, config: ConnectorInitConfig) -> NirvResult<()> {
           // Implementation
       }
       
       async fn execute_query(&self, query: ConnectorQuery) -> NirvResult<QueryResult> {
           // Implementation
       }
       
       // ... other required methods
   }
   ```

2. **Follow TDD approach**
   - Write tests first
   - Implement minimal functionality to pass tests
   - Refactor and optimize

3. **Handle errors appropriately**
   - Use `ConnectorError` for connector-specific errors
   - Provide meaningful error messages
   - Handle connection failures gracefully

#### Protocol Adapter Implementation

When implementing a protocol adapter:

1. **Implement the `ProtocolAdapter` trait**
   ```rust
   #[async_trait]
   impl ProtocolAdapter for YourProtocol {
       async fn accept_connection(&self, stream: TcpStream) -> NirvResult<Connection> {
           // Implementation
       }
       
       async fn parse_message(&self, conn: &Connection, data: &[u8]) -> NirvResult<ProtocolQuery> {
           // Implementation
       }
       
       // ... other required methods
   }
   ```

2. **Follow protocol specifications**
   - Implement wire protocol correctly
   - Handle authentication properly
   - Support all required message types

3. **Provide comprehensive tests**
   - Test packet parsing
   - Test response formatting
   - Test error conditions

### Testing Requirements

All contributions must include appropriate tests:

#### Unit Tests
```rust
#[tokio::test]
async fn test_connector_functionality() {
    let connector = YourConnector::new();
    // Test implementation
}
```

#### Integration Tests
```rust
#[tokio::test]
async fn test_end_to_end_scenario() {
    // Test complete workflow
}
```

#### Test Coverage
- Aim for high test coverage (>80%)
- Test both success and error paths
- Include edge cases and boundary conditions
- Test async functionality properly

## Testing

### Running Tests

```bash
# Run all tests
cargo test

# Run specific test module
cargo test connector_tests

# Run with output
cargo test -- --nocapture

# Run integration tests
cargo test --test integration_tests
```

### Test Organization

- **Unit tests**: In `src/` files using `#[cfg(test)]`
- **Integration tests**: In `tests/` directory
- **Examples**: In `examples/` directory

### Test Data

- Use mock data for unit tests
- Provide test fixtures for integration tests
- Document test setup requirements

## Documentation

### Code Documentation

- Document all public APIs with rustdoc
- Include examples in documentation
- Explain complex algorithms and protocols
- Document error conditions

```rust
/// Connects to a SQL Server database using the specified configuration.
/// 
/// # Arguments
/// 
/// * `config` - Connection configuration including server, credentials, etc.
/// 
/// # Returns
/// 
/// Returns `Ok(())` on successful connection, or a `ConnectorError` on failure.
/// 
/// # Examples
/// 
/// ```rust
/// let mut connector = SqlServerConnector::new();
/// let config = ConnectorInitConfig::new()
///     .with_param("server", "localhost");
/// connector.connect(config).await?;
/// ```
pub async fn connect(&mut self, config: ConnectorInitConfig) -> NirvResult<()> {
    // Implementation
}
```

### README Updates

When adding new features:
- Update the feature list in README.md
- Add usage examples
- Update the roadmap if applicable

## Pull Request Process

### Before Submitting

1. **Ensure tests pass**
   ```bash
   cargo test
   ```

2. **Format code**
   ```bash
   cargo fmt
   ```

3. **Run clippy**
   ```bash
   cargo clippy
   ```

4. **Update documentation**
   - Update README.md if needed
   - Add/update rustdoc comments
   - Update CHANGELOG.md

### PR Guidelines

1. **Create a focused PR**
   - One feature or fix per PR
   - Keep changes as small as possible
   - Avoid mixing refactoring with new features

2. **Write a clear description**
   - Explain what the PR does
   - Reference related issues
   - Include testing information

3. **Follow the template**
   ```markdown
   ## Description
   Brief description of changes
   
   ## Type of Change
   - [ ] Bug fix
   - [ ] New feature
   - [ ] Breaking change
   - [ ] Documentation update
   
   ## Testing
   - [ ] Unit tests added/updated
   - [ ] Integration tests added/updated
   - [ ] Manual testing performed
   
   ## Checklist
   - [ ] Code follows style guidelines
   - [ ] Self-review completed
   - [ ] Documentation updated
   - [ ] Tests pass locally
   ```

### Review Process

1. **Automated checks** must pass
2. **Code review** by maintainers
3. **Testing** verification
4. **Documentation** review
5. **Merge** after approval

## Issue Reporting

### Bug Reports

When reporting bugs, please include:

- **Environment information**
  - Rust version
  - Operating system
  - NIRV Engine version

- **Steps to reproduce**
  - Minimal code example
  - Expected vs actual behavior
  - Error messages or logs

- **Additional context**
  - Screenshots if applicable
  - Related issues or PRs

### Feature Requests

When requesting features:

- **Use case description**
- **Proposed solution**
- **Alternative solutions considered**
- **Additional context**

### Issue Templates

Use the provided issue templates for:
- Bug reports
- Feature requests
- Documentation improvements
- Performance issues

## Development Tips

### Debugging

- Use `RUST_LOG=debug` for detailed logging
- Use `cargo test -- --nocapture` to see test output
- Use `cargo run --example <name>` to test examples

### Performance

- Profile with `cargo bench` for benchmarks
- Use `cargo flamegraph` for performance analysis
- Monitor memory usage with appropriate tools

### IDE Setup

Recommended VS Code extensions:
- rust-analyzer
- CodeLLDB (for debugging)
- Better TOML
- GitLens

## Getting Help

- **Documentation**: Check the rustdoc documentation
- **Examples**: Look at the examples directory
- **Issues**: Search existing issues for similar problems
- **Discussions**: Use GitHub Discussions for questions

## Recognition

Contributors will be recognized in:
- CHANGELOG.md for significant contributions
- README.md contributors section
- Release notes for major features

Thank you for contributing to NIRV Engine! 🚀