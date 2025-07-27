# SCIM Server Documentation Guide

This guide provides a comprehensive overview of the SCIM Server library documentation, API reference, and usage patterns.

## 📚 Documentation Structure

### Generated API Documentation
The complete API documentation is generated using `cargo doc` and includes:

- **[Main API Reference](target/doc/scim_server/index.html)** - Complete library overview
- **[All Items](target/doc/scim_server/all.html)** - Alphabetical list of all public items

### Core Modules

#### 🏗️ [Dynamic Server](target/doc/scim_server/dynamic_server/index.html)
The heart of the library - provides runtime resource type registration and dynamic operations.

**Key Components:**
- `DynamicScimServer` - Main server implementation
- Resource type registration system
- Schema-driven validation
- Generic CRUD operations

#### 🔧 [Resource Management](target/doc/scim_server/resource/index.html)
Resource models and provider interfaces for handling SCIM entities.

**Key Components:**
- `DynamicResourceProvider` trait - Implement to handle resource storage
- `Resource` - Core resource representation
- `RequestContext` - Request metadata and auditing
- `ScimOperation` - Supported operations enum

#### 📋 [Schema System](target/doc/scim_server/schema/index.html)
Schema definitions, validation, and SCIM compliance.

**Key Components:**
- `Schema` - SCIM schema representation
- `SchemaRegistry` - Schema loading and management
- `AttributeDefinition` - Individual attribute specifications
- Validation system

#### ⚠️ [Error Handling](target/doc/scim_server/error/index.html)
Comprehensive error types with detailed context.

**Key Components:**
- `ScimError` - Main error type
- `ValidationError` - Schema validation failures
- `BuildError` - Server configuration errors

#### 🛠️ [Server Utilities](target/doc/scim_server/server/index.html)
Basic server implementation and service provider configuration.

**Key Components:**
- `ScimServer` - Basic server for schema access
- `ServiceProviderConfig` - SCIM service capabilities

#### 👤 [User Handlers](target/doc/scim_server/user_handler/index.html)
Pre-built resource handlers for common SCIM resource types.

**Key Components:**
- `create_user_resource_handler()` - User resource handler factory
- `create_group_resource_handler()` - Group resource handler factory

## 🚀 Quick Navigation

### For New Users
1. **[Library Overview](target/doc/scim_server/index.html)** - Start here for an introduction
2. **[Basic Example](examples/basic_usage.rs)** - Complete working example
3. **[DynamicScimServer](target/doc/scim_server/dynamic_server/struct.DynamicScimServer.html)** - Main server API

### For Implementation
1. **[DynamicResourceProvider](target/doc/scim_server/resource/trait.DynamicResourceProvider.html)** - Implement this trait
2. **[Schema Guide](SCHEMAS.md)** - Understanding SCIM schemas
3. **[Error Types](target/doc/scim_server/error/index.html)** - Handle errors properly

### For Advanced Usage
1. **[Resource Handlers](target/doc/scim_server/user_handler/index.html)** - Custom resource types
2. **[Schema Registry](target/doc/scim_server/schema/struct.SchemaRegistry.html)** - Schema management
3. **[Database Mapping](target/doc/scim_server/resource/struct.DatabaseMapper.html)** - Database integration

## 📖 Usage Patterns

### Basic Server Setup
```rust
use scim_server::{DynamicScimServer, DynamicResourceProvider};

// 1. Create your provider
let provider = MyProvider::new();

// 2. Create dynamic server
let mut server = DynamicScimServer::new(provider)?;

// 3. Register resource types
let user_schema = server.get_schema_by_id("urn:ietf:params:scim:schemas:core:2.0:User")?;
let user_handler = create_user_resource_handler(user_schema.clone());
server.register_resource_type("User", user_handler, operations)?;
```

### Provider Implementation
```rust
#[async_trait]
impl DynamicResourceProvider for MyProvider {
    type Error = MyError;

    async fn create_resource(
        &self,
        resource_type: &str,
        data: Value,
        context: &RequestContext,
    ) -> Result<Resource, Self::Error> {
        // Your implementation
    }

    // ... implement other required methods
}
```

### Error Handling
```rust
match server.create_resource("User", user_data, &context).await {
    Ok(user) => println!("Created: {}", user.get_id().unwrap()),
    Err(ScimError::Validation(e)) => eprintln!("Validation failed: {}", e),
    Err(ScimError::UnsupportedResourceType(t)) => eprintln!("Unsupported type: {}", t),
    Err(e) => eprintln!("Other error: {}", e),
}
```

## 🎯 Common Use Cases

### Enterprise Identity Provider
```rust
// Register multiple resource types
server.register_resource_type("User", user_handler, user_operations)?;
server.register_resource_type("Group", group_handler, group_operations)?;

// Implement SCIM endpoints
let user = server.create_resource("User", user_data, &context).await?;
let groups = server.list_resources("Group", &context).await?;
```

### Custom Resource Management
```rust
// Define custom schema
let device_schema = Schema { /* custom schema */ };
let device_handler = ResourceHandler::new(device_schema);

// Register custom resource type
server.register_resource_type("Device", device_handler, operations)?;

// Use like any other resource
let device = server.create_resource("Device", device_data, &context).await?;
```

### Multi-Tenant SaaS
```rust
// Use request context for tenant isolation
let context = RequestContext::new("request-123")
    .with_metadata("tenant_id".to_string(), "acme-corp".to_string());

// All operations include tenant context
let user = server.create_resource("User", user_data, &context).await?;
```

## 🔍 Search and Discovery

### Finding API Items
- **By Category**: Browse modules in the [main documentation](target/doc/scim_server/index.html)
- **Alphabetically**: Use the [all items](target/doc/scim_server/all.html) page
- **By Function**: Search using your browser's find function (Ctrl+F)

### Key Traits to Implement
1. **`DynamicResourceProvider`** - Core data access interface
2. **`std::error::Error`** - For custom error types
3. **`Send + Sync`** - Required for async operations

### Important Types
1. **`Resource`** - Represents any SCIM resource
2. **`RequestContext`** - Carries request metadata
3. **`Schema`** - Defines resource structure
4. **`ScimOperation`** - Defines allowed operations

## 🛠️ Development Workflow

### Generating Documentation
```bash
# Generate docs for this crate only
cargo doc --no-deps

# Generate and open in browser
cargo doc --no-deps --open

# Use the provided script
./generate-docs.sh
```

### Testing Documentation Examples
```bash
# Test all documentation examples
cargo test --doc

# Test specific module examples
cargo test --doc schema
```

### Checking Documentation Coverage
```bash
# Enable missing docs warnings
RUSTDOCFLAGS="-D missing_docs" cargo doc --no-deps
```

## 📋 Best Practices

### Documentation Style
- All public APIs include comprehensive documentation
- Examples demonstrate real-world usage
- Error conditions are clearly documented
- Type parameters and lifetimes are explained

### API Design
- Generic interfaces work with any resource type
- Schema-driven validation ensures correctness
- Async throughout for high performance
- Type safety prevents runtime errors

### Error Handling
- Specific error types for different failure modes
- Detailed error messages with context
- Graceful degradation where possible
- Clear guidance on error recovery

## 🔗 External Resources

### SCIM Standards
- [RFC 7643 - SCIM Core Schema](https://tools.ietf.org/html/rfc7643)
- [RFC 7644 - SCIM Protocol](https://tools.ietf.org/html/rfc7644)

### Rust Documentation
- [The Rust Book](https://doc.rust-lang.org/book/)
- [Async Programming in Rust](https://rust-lang.github.io/async-book/)
- [API Guidelines](https://rust-lang.github.io/api-guidelines/)

### Related Crates
- [async-trait](https://docs.rs/async-trait/) - Async trait support
- [serde](https://docs.rs/serde/) - Serialization framework
- [tokio](https://docs.rs/tokio/) - Async runtime

## 🚀 Getting Started Checklist

- [ ] Read the [library overview](target/doc/scim_server/index.html)
- [ ] Run the [basic example](examples/basic_usage.rs)
- [ ] Understand [DynamicResourceProvider](target/doc/scim_server/resource/trait.DynamicResourceProvider.html)
- [ ] Review [error handling](target/doc/scim_server/error/index.html) patterns
- [ ] Explore [schema system](target/doc/scim_server/schema/index.html)
- [ ] Check [resource handlers](target/doc/scim_server/user_handler/index.html) for common types
- [ ] Implement your first provider
- [ ] Test with multiple resource types

---

**Need help?** The generated documentation includes detailed examples, type information, and usage patterns for every public API. Start with the [main documentation page](target/doc/scim_server/index.html) and explore from there.