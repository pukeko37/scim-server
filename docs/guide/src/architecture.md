# Architecture

The SCIM Server follows a clean trait-based architecture with clear separation of concerns designed for maximum composability and extensibility.

## Component Architecture

The library is built around composable traits that you implement for your specific needs:

```text
┌─────────────────┐    ┌──────────────────┐    ┌─────────────────┐
│  Client Layer   │    │   SCIM Server    │    │ Resource Layer  │
│                 │    │                  │    │                 │
│  • MCP AI       │───▶│  • Operations    │───▶│ ResourceProvider│
│  • Web Framework│    │  • Multi-tenant  │    │      trait      │
│  • Custom       │    │  • Type Safety   │    │                 │
└─────────────────┘    └──────────────────┘    └─────────────────┘
                              │                          │
                              ▼                          ▼
                       ┌──────────────────┐    ┌─────────────────┐
                       │ Schema System    │    │ Storage Layer   │
                       │                  │    │                 │
                       │ • SchemaRegistry │    │ StorageProvider │
                       │ • Validation     │    │      trait      │
                       │ • Value Objects  │    │  • In-Memory    │
                       │ • Extensions     │    │  • Database     │
                       └──────────────────┘    │  • Custom       │
                                               └─────────────────┘
```

## Layer Responsibilities

**Client Layer**: Your integration points - compose these components into web endpoints, AI tools, or custom applications.

**[SCIM Server](https://docs.rs/scim-server/latest/scim_server/struct.ScimServer.html)**: Orchestration component that coordinates resource operations using your provider implementations.

**Resource Layer**: [`ResourceProvider` trait](https://docs.rs/scim-server/latest/scim_server/trait.ResourceProvider.html) - implement this interface for your data model, or use the provided [`StandardResourceProvider`](https://docs.rs/scim-server/latest/scim_server/providers/struct.StandardResourceProvider.html) for common scenarios.

**Schema System**: [Schema registry](https://docs.rs/scim-server/latest/scim_server/schema/struct.SchemaRegistry.html) and validation components - extend with custom schemas and value objects.

**Storage Layer**: [`StorageProvider` trait](https://docs.rs/scim-server/latest/scim_server/storage/trait.StorageProvider.html) - use the provided [`InMemoryStorage`](https://docs.rs/scim-server/latest/scim_server/storage/struct.InMemoryStorage.html) for development, or connect to any database or custom backend.

## Core Traits

### ResourceProvider
Your main integration point for SCIM resource operations - see the [full API documentation](https://docs.rs/scim-server/latest/scim_server/trait.ResourceProvider.html):

```rust
pub trait ResourceProvider {
    type Error: std::error::Error + Send + Sync + 'static;

    async fn create_resource(
        &self,
        resource_type: &str,
        data: Value,
        context: &RequestContext,
    ) -> Result<Resource, Self::Error>;

    async fn get_resource(
        &self,
        resource_type: &str,
        id: &str,
        context: &RequestContext,
    ) -> Result<Option<Resource>, Self::Error>;

    // ... other CRUD operations
}
```

**Implementation Options:**
- Use [`StandardResourceProvider<S>`](https://docs.rs/scim-server/latest/scim_server/providers/struct.StandardResourceProvider.html) with any [`StorageProvider`](https://docs.rs/scim-server/latest/scim_server/storage/trait.StorageProvider.html) for typical use cases
- Implement directly for custom business logic and data models
- Wrap existing services or databases

### StorageProvider
Pure data persistence abstraction - see the [full API documentation](https://docs.rs/scim-server/latest/scim_server/storage/trait.StorageProvider.html):

```rust
pub trait StorageProvider: Send + Sync {
    type Error: std::error::Error + Send + Sync + 'static;

    async fn put(&self, key: StorageKey, data: Value) -> Result<Value, Self::Error>;
    async fn get(&self, key: StorageKey) -> Result<Option<Value>, Self::Error>;
    async fn delete(&self, key: StorageKey) -> Result<bool, Self::Error>;
    async fn list(&self, prefix: StoragePrefix) -> Result<Vec<Value>, Self::Error>;
}
```

**Implementation Options:**
- Use [`InMemoryStorage`](https://docs.rs/scim-server/latest/scim_server/storage/struct.InMemoryStorage.html) for development and testing
- Implement for your database (PostgreSQL, MongoDB, etc.)
- Connect to cloud storage or external APIs

### Value Objects
Type-safe SCIM attribute handling - see the [schema documentation](https://docs.rs/scim-server/latest/scim_server/schema/index.html):

```rust
pub trait ValueObject: Debug + Send + Sync {
    fn attribute_type(&self) -> AttributeType;
    fn to_json(&self) -> ValidationResult<Value>;
    fn validate_against_schema(&self, definition: &AttributeDefinition) -> ValidationResult<()>;
    // ...
}

pub trait SchemaConstructible: ValueObject + Sized {
    fn from_schema_and_value(
        definition: &AttributeDefinition,
        value: &Value,
    ) -> ValidationResult<Self>;
    // ...
}
```

**Extension Points:**
- Create custom value objects for domain-specific attributes
- Implement validation logic for business rules
- Support for complex multi-valued attributes

## Multi-Tenant Architecture

The library provides several components for multi-tenant systems:

### TenantResolver
Maps authentication credentials to tenant context - see the [multi-tenant API documentation](https://docs.rs/scim-server/latest/scim_server/multi_tenant/trait.TenantResolver.html):

```rust
pub trait TenantResolver: Send + Sync {
    type Error: std::error::Error + Send + Sync + 'static;

    async fn resolve_tenant(&self, credential: &str) -> Result<TenantContext, Self::Error>;
}
```

### RequestContext
Carries tenant and request information through all operations - see the [RequestContext API](https://docs.rs/scim-server/latest/scim_server/struct.RequestContext.html):

```rust
pub struct RequestContext {
    pub request_id: String,
    tenant_context: Option<TenantContext>,
}
```

### Tenant Isolation
- All [`ResourceProvider`](https://docs.rs/scim-server/latest/scim_server/trait.ResourceProvider.html) operations include [`RequestContext`](https://docs.rs/scim-server/latest/scim_server/struct.RequestContext.html)
- Storage keys automatically include tenant ID
- Schema validation respects tenant-specific extensions

## Schema System Architecture

### SchemaRegistry
Central registry for SCIM schemas - see the [SchemaRegistry API](https://docs.rs/scim-server/latest/scim_server/schema/struct.SchemaRegistry.html):

- Loads and validates RFC 7643 core schemas
- Supports custom schema extensions
- Provides validation services for all operations

### Dynamic Value Objects
- Runtime creation from schema definitions
- Type-safe attribute handling
- Extensible factory pattern for custom types

### Extension Model
- Custom resource types beyond User/Group
- Organization-specific attributes
- Maintains SCIM compliance and interoperability

## Concurrency Control

### ETag-Based Versioning
Built into the core architecture:

- Automatic version generation from resource content
- Conditional operations (If-Match, If-None-Match)
- Conflict detection and resolution
- Production-ready optimistic locking

### Version-Aware Operations
All resource operations support conditional execution - see the [Resource API](https://docs.rs/scim-server/latest/scim_server/struct.Resource.html):

```rust
// Conditional update with version check
let result = provider.conditional_update(
    "User", 
    &user_id, 
    updated_data, 
    &expected_version, 
    &context
).await?;

match result {
    ConditionalResult::Success(resource) => // Update succeeded
    ConditionalResult::PreconditionFailed => // Version conflict
}
```

## Integration Patterns

### Web Framework Integration
Components work with any HTTP framework:

1. Extract SCIM request details
2. Create [`RequestContext`](https://docs.rs/scim-server/latest/scim_server/struct.RequestContext.html) with tenant info
3. Call appropriate [`ScimServer`](https://docs.rs/scim-server/latest/scim_server/struct.ScimServer.html) operations
4. Format responses per SCIM specification

### AI Agent Integration
[Model Context Protocol (MCP) components](https://docs.rs/scim-server/latest/scim_server/mcp_integration/index.html):

1. Expose SCIM operations as discoverable tools
2. Structured schemas for AI understanding
3. Error handling designed for AI decision making
4. Multi-tenant aware tool descriptions

### Custom Client Integration
Direct component usage:

1. Implement [`ResourceProvider`](https://docs.rs/scim-server/latest/scim_server/trait.ResourceProvider.html) for your data model
2. Choose appropriate [`StorageProvider`](https://docs.rs/scim-server/latest/scim_server/storage/trait.StorageProvider.html)
3. Configure [schema extensions](./concepts/schema-mechanisms.md) as needed
4. Build custom API layer or integration logic

## Performance Considerations

### Async-First Design
- All I/O operations are async
- Non-blocking concurrent operations
- Efficient resource utilization

### Minimal Allocations
- Zero-copy JSON processing where possible
- Efficient value object system
- Smart caching in schema registry

### Scalability Features
- Pluggable storage for horizontal scaling
- Multi-tenant isolation for SaaS platforms
- Connection pooling support through storage traits