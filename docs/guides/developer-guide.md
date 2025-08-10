# Developer Guide

A comprehensive guide for developers working on the SCIM Server crate, covering development setup, contribution guidelines, architecture principles, and testing strategies.

## Table of Contents

- [Development Environment](#development-environment)
- [Project Structure](#project-structure)
- [Architecture Principles](#architecture-principles)
- [Development Workflow](#development-workflow)
- [Testing Strategy](#testing-strategy)
- [Code Style Guide](#code-style-guide)
- [Performance Guidelines](#performance-guidelines)
- [Security Considerations](#security-considerations)
- [Contributing](#contributing)
- [Release Process](#release-process)

## Development Environment

### Prerequisites

- **Rust**: 1.70 or later (latest stable recommended)
- **Git**: For version control
- **Docker**: For integration tests and local services
- **IDE**: VS Code with rust-analyzer or IntelliJ with Rust plugin

### Setup Instructions

```bash
# Clone the repository
git clone <repository-url>
cd scim-server

# Install Rust toolchain components
rustup component add clippy rustfmt

# Install development dependencies
cargo install cargo-watch cargo-expand cargo-audit

# Verify setup
cargo check
cargo test
cargo doc
```

### Development Tools

#### Essential Tools
```bash
# File watcher for continuous development
cargo install cargo-watch
cargo watch -x check -x test

# Code formatting
cargo fmt

# Linting
cargo clippy -- -D warnings

# Security audit
cargo audit

# Expand macros for debugging
cargo expand
```

#### Optional Tools
```bash
# Benchmark testing
cargo install cargo-criterion

# Test coverage
cargo install cargo-tarpaulin
cargo tarpaulin --out Html

# Dependency analysis
cargo install cargo-deps
cargo deps --all-deps | dot -Tpng > dependencies.png
```

### IDE Configuration

#### VS Code Extensions
- `rust-analyzer` - Language server for Rust
- `CodeLLDB` - Debugger support
- `Better TOML` - TOML file support
- `Error Lens` - Inline error display

#### VS Code Settings (`.vscode/settings.json`)
```json
{
    "rust-analyzer.cargo.features": "all",
    "rust-analyzer.check.command": "clippy",
    "rust-analyzer.imports.granularity.group": "module",
    "rust-analyzer.imports.prefix": "crate",
    "[rust]": {
        "editor.formatOnSave": true,
        "editor.defaultFormatter": "rust-lang.rust-analyzer"
    }
}
```

## Project Structure

### High-Level Organization

```
scim-server/
├── src/                          # Source code
│   ├── lib.rs                   # Library root and public API
│   ├── error.rs                 # Error types and handling
│   ├── resource/                # SCIM resource management
│   │   ├── mod.rs              # Resource module root
│   │   ├── resource.rs         # Core Resource type
│   │   ├── builder.rs          # ResourceBuilder implementation
│   │   └── value_objects/      # Type-safe SCIM attributes
│   ├── schema/                  # Schema management and validation
│   │   ├── mod.rs              # Schema module root
│   │   ├── registry.rs         # Schema loading and management
│   │   ├── validation.rs       # Validation logic
│   │   └── types.rs            # Schema data structures
│   ├── multi_tenant/            # Multi-tenancy support
│   │   ├── mod.rs              # Multi-tenant module root
│   │   ├── resolver.rs         # Tenant resolution strategies
│   │   └── scim_config.rs      # Tenant-specific configuration
│   ├── providers/               # Storage provider implementations
│   │   ├── mod.rs              # Provider module root
│   │   └── in_memory.rs        # In-memory provider for testing
│   ├── resource_handlers/       # Resource-specific operation handlers
│   │   ├── mod.rs              # Handler module root
│   │   ├── user.rs             # User resource handler
│   │   └── group.rs            # Group resource handler
│   ├── scim_server/            # Main server implementation
│   │   ├── mod.rs              # Server module root
│   │   ├── core.rs             # Core server functionality
│   │   └── operations.rs       # SCIM operation implementations
│   └── schema_discovery.rs      # Schema discovery utilities
├── tests/                       # Integration tests
├── examples/                    # Usage examples
├── benches/                     # Performance benchmarks
├── schemas/                     # SCIM schema definitions
└── docs/                        # Documentation
```

### Module Responsibilities

#### `resource` Module
- **Purpose**: Core SCIM resource types and construction
- **Key Types**: `Resource`, `ResourceBuilder`
- **Dependencies**: `value_objects`, `error`
- **Stability**: High - Core API, changes require careful consideration

#### `value_objects` Module
- **Purpose**: Type-safe SCIM attribute implementations
- **Key Types**: `ResourceId`, `UserName`, `MultiValuedAttribute<T>`
- **Dependencies**: `error`
- **Stability**: High - Value objects should be immutable and stable

#### `schema` Module
- **Purpose**: SCIM schema loading, validation, and management
- **Key Types**: `SchemaRegistry`, `Schema`, `AttributeDefinition`
- **Dependencies**: `resource`, `error`
- **Stability**: Medium - May evolve as SCIM extensions are added

#### `multi_tenant` Module
- **Purpose**: Multi-tenancy support and tenant resolution
- **Key Types**: `TenantContext`, `TenantResolver`, `StaticTenantResolver`
- **Dependencies**: `error`
- **Stability**: Medium - May add new resolution strategies

#### `providers` Module
- **Purpose**: Storage backend abstractions and implementations
- **Key Types**: `ResourceProvider`, `InMemoryProvider`
- **Dependencies**: `resource`, `multi_tenant`, `error`
- **Stability**: Medium - New providers may be added

#### `scim_server` Module
- **Purpose**: Main server orchestration and operation handling
- **Key Types**: `ScimServer`, `ScimOperation`
- **Dependencies**: All other modules
- **Stability**: Medium - Server interface may evolve

## Architecture Principles

### Type Safety First

The crate prioritizes compile-time safety over runtime flexibility:

```rust
// Good: Compile-time validation
let id = ResourceId::new(uuid_string)?;  // Validates at construction
let username = UserName::new(email)?;    // Cannot be invalid

// Avoid: Runtime validation everywhere
struct UnsafeResource {
    id: String,  // Could be invalid
    username: String,  // No validation
}
```

### Functional Programming Patterns

Prefer immutable data structures and functional composition:

```rust
// Good: Functional style with method chaining
let result = resource
    .emails()
    .unwrap_or_default()
    .iter()
    .filter(|e| e.email_type().map_or(false, |t| t == "work"))
    .map(|e| e.value())
    .collect::<Vec<_>>();

// Avoid: Imperative style with mutation
let mut work_emails = Vec::new();
if let Some(emails) = resource.emails() {
    for email in emails.iter() {
        if let Some(email_type) = email.email_type() {
            if email_type == "work" {
                work_emails.push(email.value());
            }
        }
    }
}
```

### Error Handling Philosophy

Use specific error types that provide actionable information:

```rust
// Good: Specific, actionable errors
#[derive(Debug, thiserror::Error)]
pub enum ValidationError {
    #[error("Invalid resource ID format: {value}")]
    InvalidResourceId { value: String },
    
    #[error("Missing required attribute: {attribute}")]
    MissingRequiredAttribute { attribute: String },
    
    #[error("Invalid value '{value}' for attribute '{attribute}': {reason}")]
    InvalidAttributeValue { attribute: String, value: String, reason: String },
}

// Avoid: Generic errors
#[derive(Debug, thiserror::Error)]
pub enum GenericError {
    #[error("Something went wrong: {0}")]
    Generic(String),
}
```

### Async-First Design

All I/O operations are async by default:

```rust
// Good: Async trait methods
impl ResourceProvider for MyProvider {
    fn create_resource(&self, ...) -> impl Future<Output = Result<Resource, Self::Error>> + Send {
        async move { ... }
    }
}

// Avoid: Blocking operations in async contexts
impl ResourceProvider for BlockingProvider {
    fn create_resource(&self, ...) -> impl Future<Output = Result<Resource, Self::Error>> + Send {
        async move { 
            std::thread::sleep(Duration::from_millis(100)); // DON'T DO THIS
            ...
        }
    }
}
```

## Development Workflow

### Feature Development Process

1. **Create Feature Branch**
   ```bash
   git checkout -b feature/new-feature-name
   ```

2. **Write Tests First** (TDD Approach)
   ```rust
   #[cfg(test)]
   mod tests {
       use super::*;
       
       #[test]
       fn test_new_feature() {
           // Write the test for your feature first
           assert_eq!(new_feature_function(), expected_result);
       }
   }
   ```

3. **Implement Feature**
   ```rust
   // Implement the minimum code to make tests pass
   pub fn new_feature_function() -> ExpectedType {
       // Implementation
   }
   ```

4. **Run Full Test Suite**
   ```bash
   cargo test
   cargo test --doc
   cargo clippy -- -D warnings
   cargo fmt --check
   ```

5. **Update Documentation**
   ```rust
   /// Brief description of the function.
   ///
   /// # Arguments
   /// * `param1` - Description of parameter
   ///
   /// # Returns
   /// Description of return value
   ///
   /// # Examples
   /// ```
   /// let result = new_feature_function();
   /// assert_eq!(result, expected);
   /// ```
   pub fn new_feature_function() -> ExpectedType {
       // Implementation
   }
   ```

6. **Performance Testing**
   ```bash
   cargo bench
   ```

### Code Review Guidelines

#### For Authors
- **Self-review first**: Review your own code before submitting
- **Test coverage**: Ensure new code is well-tested
- **Documentation**: Update docs for any public API changes
- **Performance**: Consider performance implications
- **Breaking changes**: Clearly document any breaking changes

#### For Reviewers
- **Functionality**: Does the code work as intended?
- **Design**: Does it fit well with the existing architecture?
- **Performance**: Are there obvious performance issues?
- **Security**: Are there security implications?
- **Testing**: Is the code adequately tested?
- **Documentation**: Is the code well-documented?

### Continuous Integration

Our CI pipeline runs:
1. **Format check**: `cargo fmt -- --check`
2. **Lint check**: `cargo clippy -- -D warnings`
3. **Test suite**: `cargo test`
4. **Doc tests**: `cargo test --doc`
5. **Security audit**: `cargo audit`
6. **Benchmark regression**: `cargo bench` (on performance-sensitive changes)

## Testing Strategy

### Test Categories

#### 1. Unit Tests
Test individual functions and methods in isolation.

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_resource_id_validation() {
        // Valid UUID
        let valid_id = ResourceId::new("2819c223-7f76-453a-919d-413861904646".to_string());
        assert!(valid_id.is_ok());
        
        // Invalid UUID
        let invalid_id = ResourceId::new("not-a-uuid".to_string());
        assert!(invalid_id.is_err());
    }
    
    #[test]
    fn test_multi_valued_attribute_primary() {
        let emails = vec![
            EmailAddress::new_simple("work@example.com".to_string()).unwrap(),
            EmailAddress::new_simple("personal@example.com".to_string()).unwrap(),
        ];
        
        let multi_emails = MultiValuedAttribute::new(emails).unwrap();
        assert!(multi_emails.primary().is_none());
        
        let with_primary = multi_emails.with_primary(0).unwrap();
        assert!(with_primary.primary().is_some());
    }
}
```

#### 2. Integration Tests
Test complete workflows and component interactions.

```rust
// tests/integration_tests.rs
use scim_server::*;

#[tokio::test]
async fn test_complete_user_lifecycle() {
    let provider = providers::InMemoryProvider::new();
    let mut server = ScimServer::new(provider);
    server.register_resource_handler("User", create_user_resource_handler());
    
    let context = RequestContext::with_generated_id();
    
    // Create
    let user_data = json!({
        "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
        "userName": "test.user@example.com"
    });
    let created = server.create_resource("User", user_data, &context).await.unwrap();
    let user_id = created.id().unwrap().as_str();
    
    // Read
    let retrieved = server.get_resource("User", user_id, &context).await.unwrap();
    assert!(retrieved.is_some());
    
    // Update
    let update_data = json!({
        "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
        "userName": "updated.user@example.com"
    });
    let updated = server.update_resource("User", user_id, update_data, &context).await.unwrap();
    assert_eq!(updated.user_name().unwrap().as_str(), "updated.user@example.com");
    
    // Delete
    server.delete_resource("User", user_id, &context).await.unwrap();
    let deleted = server.get_resource("User", user_id, &context).await.unwrap();
    assert!(deleted.is_none());
}
```

#### 3. Documentation Tests
Ensure all examples in documentation compile and run.

```rust
/// Creates a new resource ID from a UUID string.
///
/// # Examples
///
/// ```
/// use scim_server::resource::value_objects::ResourceId;
///
/// fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let id = ResourceId::new("2819c223-7f76-453a-919d-413861904646".to_string())?;
///     assert_eq!(id.as_str(), "2819c223-7f76-453a-919d-413861904646");
///     Ok(())
/// }
/// ```
pub fn new(value: String) -> ValidationResult<Self> {
    // Implementation
}
```

#### 4. Property-Based Tests
Use property-based testing for complex validation logic.

```rust
use proptest::prelude::*;

proptest! {
    #[test]
    fn test_resource_id_roundtrip(uuid in "[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}") {
        let id = ResourceId::new(uuid.clone()).unwrap();
        prop_assert_eq!(id.as_str(), uuid);
        prop_assert_eq!(id.into_string(), uuid);
    }
    
    #[test]
    fn test_multi_valued_attribute_invariants(
        values in prop::collection::vec(any::<String>(), 1..10),
        primary_index in 0usize..10
    ) {
        let emails: Result<Vec<_>, _> = values.into_iter()
            .map(|v| EmailAddress::new_simple(format!("{}@example.com", v)))
            .collect();
            
        if let Ok(emails) = emails {
            let multi_attr = MultiValuedAttribute::new(emails).unwrap();
            
            if primary_index < multi_attr.len() {
                let with_primary = multi_attr.with_primary(primary_index).unwrap();
                prop_assert!(with_primary.primary().is_some());
            }
        }
    }
}
```

### Testing Commands

```bash
# Run all tests
cargo test

# Run only unit tests
cargo test --lib

# Run only integration tests
cargo test --test integration_tests

# Run only documentation tests
cargo test --doc

# Run specific test
cargo test test_resource_creation

# Run tests with logging
RUST_LOG=debug cargo test test_name -- --nocapture

# Run tests with coverage
cargo tarpaulin
```

## Code Style Guide

### Rust Style Guidelines

Follow the official Rust style guide with these project-specific additions:

#### Naming Conventions
```rust
// Types: PascalCase
pub struct ResourceProvider;
pub enum ValidationError;

// Functions and variables: snake_case
pub fn create_resource() -> ValidationResult<Resource>;
let user_name = "john.doe@example.com";

// Constants: SCREAMING_SNAKE_CASE
const DEFAULT_TIMEOUT: Duration = Duration::from_secs(30);

// Modules: snake_case
mod value_objects;
mod multi_tenant;
```

#### Error Handling Patterns
```rust
// Good: Use specific error types
fn parse_email(input: &str) -> Result<EmailAddress, ValidationError> {
    if input.is_empty() {
        return Err(ValidationError::MissingRequiredAttribute {
            attribute: "emails.value".to_string(),
        });
    }
    // ...
}

// Good: Use ? operator for error propagation
fn create_user_resource(data: Value) -> ValidationResult<Resource> {
    let username = extract_username(&data)?;
    let name = extract_name(&data)?;
    
    ResourceBuilder::new("User")
        .user_name(username)?
        .name(name)?
        .build()
}

// Avoid: Unwrap in library code
fn bad_example(input: &str) -> String {
    input.parse().unwrap()  // DON'T DO THIS
}
```

#### Documentation Standards
```rust
/// Brief one-line description.
///
/// Longer description explaining the purpose, behavior, and any important
/// details about the function. Use multiple paragraphs if needed.
///
/// # Arguments
///
/// * `param1` - Description of the first parameter
/// * `param2` - Description of the second parameter
///
/// # Returns
///
/// Description of what the function returns and under what conditions.
///
/// # Errors
///
/// This function returns an error if:
/// - Condition 1 that causes an error
/// - Condition 2 that causes an error
///
/// # Examples
///
/// ```
/// use scim_server::example::function_name;
///
/// fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let result = function_name("parameter")?;
///     assert_eq!(result, expected_value);
///     Ok(())
/// }
/// ```
///
/// # Panics
///
/// This function panics if [describe panic conditions, avoid if possible].
///
/// # Safety
///
/// This function is unsafe because [describe safety requirements for unsafe functions].
pub fn function_name(param1: Type1, param2: Type2) -> Result<ReturnType, ErrorType> {
    // Implementation
}
```

### Code Organization Patterns

#### Struct Definition Pattern
```rust
/// Brief description of the struct.
///
/// Longer explanation of the struct's purpose and usage.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MyStruct {
    /// Description of field1
    field1: Type1,
    /// Description of field2  
    field2: Type2,
    /// Private field (no pub)
    internal_field: Type3,
}

impl MyStruct {
    /// Constructor documentation
    pub fn new(param1: Type1, param2: Type2) -> ValidationResult<Self> {
        // Validation logic
        Ok(Self {
            field1: param1,
            field2: param2,
            internal_field: default_value(),
        })
    }
    
    /// Accessor method documentation
    pub fn field1(&self) -> &Type1 {
        &self.field1
    }
    
    /// Business logic method documentation
    pub fn do_something(&self) -> ValidationResult<Type4> {
        // Implementation
    }
}
```

#### Module Structure Pattern
```rust
//! Module-level documentation explaining the module's purpose.
//!
//! This module provides [high-level description of functionality].
//!
//! # Usage
//!
//! ```
//! use crate::module_name::ImportantType;
//!
//! let instance = ImportantType::new()?;
//! ```

// Public re-exports
pub use submodule::PublicType;

// Private modules
mod private_submodule;

// Public modules
pub mod public_submodule;

// Private types and functions
struct InternalType;

fn internal_helper() -> Result<(), Error> {
    // Implementation
}

// Public API
pub struct PublicType;

impl PublicType {
    /// Public constructor
    pub fn new() -> ValidationResult<Self> {
        // Implementation
    }
}
```

## Performance Guidelines

### Memory Management

#### Prefer Borrowing Over Cloning
```rust
// Good: Use references when possible
pub fn process_emails(emails: &MultiValuedAttribute<EmailAddress>) -> Vec<&str> {
    emails.iter().map(|e| e.value()).collect()
}

// Avoid: Unnecessary cloning
pub fn process_emails_bad(emails: &MultiValuedAttribute<EmailAddress>) -> Vec<String> {
    emails.iter().map(|e| e.value().to_string()).collect()  // Unnecessary allocation
}
```

#### Use Cow for Flexible Ownership
```rust
use std::borrow::Cow;

pub fn format_display_name(name: &Name) -> Cow<str> {
    match name.formatted() {
        Some(formatted) => Cow::Borrowed(formatted),
        None => Cow::Owned(format!("{} {}", 
            name.given_name().unwrap_or(""), 
            name.family_name().unwrap_or("")
        )),
    }
}
```

### Async Performance

#### Use Stream Processing for Large Collections
```rust
use futures::stream::{self, StreamExt};

async fn process_resources_efficiently<P: ResourceProvider>(
    provider: &P,
    resource_ids: Vec<String>,
    context: &RequestContext,
) -> Result<Vec<Resource>, P::Error> {
    // Process in parallel with concurrency limit
    let results = stream::iter(resource_ids)
        .map(|id| provider.get_resource("User", &id, context))
        .buffer_unordered(10)  // Limit concurrent operations
        .collect::<Vec<_>>()
        .await;
    
    // Collect successful results
    results.into_iter()
        .filter_map(|r| r.ok().flatten())
        .collect()
}
```

#### Avoid Blocking Operations
```rust
// Good: Async all the way
async fn async_validation(resource: &Resource) -> ValidationResult<()> {
    let schema_registry = SchemaRegistry::new()?;
    schema_registry.validate_resource_hybrid(resource)?;
    Ok(())
}

// Avoid: Blocking in async context
async fn blocking_validation(resource: &Resource) -> ValidationResult<()> {
    std::thread::sleep(Duration::from_millis(100));  // DON'T DO THIS
    Ok(())
}
```

### Database Performance

#### Use Prepared Statements
```rust
impl DatabaseProvider {
    async fn create_resource_optimized(&self, resource: &Resource) -> Result<(), DatabaseError> {
        // Good: Prepared statement
        let query = sqlx::query!(
            "INSERT INTO scim_resources (id, resource_type, data, tenant_id) VALUES ($1, $2, $3, $4)",
            resource.id().unwrap().as_str(),
            resource.resource_type(),
            serde_json::to_string(&resource.to_json()?)?,
            self.get_tenant_id()
        );
        
        query.execute(&self.pool).await?;
        Ok(())
    }
}
```

#### Batch Operations
```rust
async fn create_resources_batch<P: ResourceProvider>(
    provider: &P,
    resources: Vec<Value>,
    context: &RequestContext,
) -> Result<Vec<Resource>, P::Error> {
    // Process in batches for better performance
    const BATCH_SIZE: usize = 100;
    
    let mut results = Vec::new();
    
    for chunk in resources.chunks(BATCH_SIZE) {
        let batch_futures: Vec<_> = chunk
            .iter()
            .map(|data| provider.create_resource("User", data.clone(), context))
            .collect();
        
        let batch_results = futures::future::try_join_all(batch_futures).await?;
        results.extend(batch_results);
    }
    
    Ok(results)
}
```

## Security Considerations

### Input Validation

All external input must be validated before processing:

```rust
pub fn validate_user_input(input: &Value) -> ValidationResult<()> {
    // Size limits
    let json_str = serde_json::to_string(input)?;
    if json_str.len() > MAX_RESOURCE_SIZE {
        return Err(ValidationError::ResourceTooLarge);
    }
    
    // Schema validation
    let schema_registry = SchemaRegistry::new()?;
    schema_registry.validate_json_resource_with_context("User", input, OperationContext::Create)?;
    
    // Business rule validation
    validate_business_rules(input)?;
    
    Ok(())
}
```

### Tenant Isolation

Ensure complete tenant isolation in all operations:

```rust
impl DatabaseProvider {
    async fn get_resource_secure(
        &self,
        resource_type: &str,
        id: &str,
        context: &RequestContext,
    ) -> Result<Option<Resource>, DatabaseError> {
        // ALWAYS include tenant_id in queries
        let tenant_id = context.tenant_context()
            .map(|t| t.tenant_id())
            .unwrap_or("default");
        
        let query = sqlx::query_as!(
            ResourceRow,
            "SELECT * FROM scim_resources WHERE id = $1 AND resource_type = $2 AND tenant_id = $3",
            id,
            resource_type,
            tenant_id
        );
        
        // ... rest of implementation
    }
}
```

### Secret Management

Never hardcode secrets in source code:

```rust
// Good: Load from environment
pub fn load_config() -> Result<Config, ConfigError> {
    Config {
        database_url: std::env::var("DATABASE_URL")?,
        jwt_secret: std::env::var("JWT_SECRET")?,
        // ...
    }
}

// Avoid: Hardcoded secrets
pub fn bad_config() -> Config {
    Config {
        database_url: "postgresql://user:password@localhost/db".to_string(),  // DON'T DO THIS
        jwt_secret: "super-secret-key".to_string(),  // DON'T DO THIS
    }
}
```

## Contributing

### Before You Start

1. **Read the documentation** to understand the project goals and architecture
2. **Check existing issues** to see if your idea is already being worked on
3. **Open an issue** to discuss major changes before implementing
4. **Start small** with bug fixes or documentation improvements

### Contribution Types

#### Bug Fixes
1. **Reproduce the bug** with a failing test
2. **Fix the issue** with minimal code changes
3. **Verify the fix** doesn't break existing functionality
4. **Update documentation** if the fix changes behavior

#### New Features
1. **Design discussion** in GitHub issues
2. **API design** that fits with existing patterns
3. **Implementation** with comprehensive tests
4. **Documentation** including examples and migration guide

#### Performance Improvements
1. **Benchmark current performance** to establish baseline
2. **Implement optimization** with before/after measurements
3. **Verify correctness** with existing test suite
4. **Document performance characteristics**

### Pull Request Process

1. **Create descriptive PR title**: "Add support for custom schema validation"
2. **Fill out PR template** with all required information
3. **Link related issues**: "Fixes #123, Addresses #456"
4. **Request appropriate reviewers** based on changed modules
5. **Address review feedback** promptly and thoroughly
6. **Squash commits** before merging (if requested)

### Code Review Checklist

#### For All Changes
- [ ] Code compiles without warnings
- [ ] All tests pass (including doc tests)
- [ ] Code follows project style guidelines
- [ ] Documentation is updated
- [ ] CHANGELOG is updated for user-facing changes

#### For New Features
- [ ] Feature has comprehensive tests
- [ ] Feature is documented with examples
- [ ] Feature follows existing API patterns
- [ ] Performance impact is considered
- [ ] Breaking changes are clearly marked

#### For Bug Fixes
- [ ] Bug is reproduced with a test
- [ ] Fix is minimal and targeted
- [ ] Regression tests are added
- [ ] Root cause is documented

## Release Process

### Version Management

We follow semantic versioning (SemVer):
- **MAJOR**: Breaking API changes
- **MINOR**: New features, backward compatible
- **PATCH**: Bug fixes, backward compatible

### Release Checklist

#### Pre-release
- [ ] All CI checks pass
- [ ] Documentation is up to date
- [ ] CHANGELOG is updated
- [ ] Version number is bumped in Cargo.toml
- [ ] Performance benchmarks are run
- [ ] Security audit passes

#### Release
- [ ] Create release tag: `git tag v0.1.0`
- [ ] Push tag: `git push origin v0.1.0`
- [ ] Publish to crates.io: `cargo publish`
- [ ] Create GitHub release with release notes
- [ ] Update documentation sites

#### Post-release
- [ ] Monitor for issues in the first 24 hours
- [ ] Update dependent projects
- [ ] Announce release in community channels

## Development Tips

### Debugging Techniques

#### Use Comprehensive Logging
```rust
use log::{debug, info, warn, error};

pub async fn debug_resource_creation(data: &Value) -> ValidationResult<Resource> {
    debug!("Creating resource from data: {}", serde_json::to_string_pretty(data)?);
    
    let resource = Resource::from_json("User".to_string(), data.clone())?;
    info!("Resource created successfully with ID: {:?}", resource.id());
    
    Ok(resource)
}
```

#### Add Trace Points for Complex Operations
```rust
use tracing::{instrument, debug, info};

#[instrument(skip(provider, context))]
async fn traced_operation<P: ResourceProvider>(
    provider: &P,
    resource_type: &str,
    context: &RequestContext,
) -> Result<Vec<Resource>, P::Error> {
    debug!("Starting resource list operation");
    
    let resources = provider.list_resources(resource_type, None, context).await?;
    
    info!("Listed {} resources", resources.len());
    Ok(resources)
}
```

#### Use Property-Based Testing for Edge Cases
```rust
use proptest::prelude::*;

proptest! {
    #[test]
    fn fuzz_email_validation(
        local in "[a-zA-Z0-9._%+-]{1,64}",
        domain in "[a-zA-Z0-9.-]{1,255}\\.[a-zA-Z]{2,}"
    ) {
        let email = format!("{}@{}", local, domain);
        
        // This should either succeed or fail gracefully
        match EmailAddress::new_simple(email.clone()) {
            Ok(addr) => prop_assert_eq!(addr.value(), email),
            Err(e) => {
                // Error should be informative
                prop_assert!(e.to_string().contains("email") || e.to_string().contains("validation"));
            }
        }
    }
}
```

### Performance Profiling

####