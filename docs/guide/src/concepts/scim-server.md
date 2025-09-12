# SCIM Server

The SCIM Server is the central orchestration layer of the SCIM Server library, providing a complete, dynamic SCIM 2.0 protocol implementation that can handle any resource type registered at runtime. It serves as the primary interface between your application and the SCIM protocol, eliminating hard-coded resource types and enabling truly schema-driven identity management.

## Value Proposition

The SCIM Server module delivers comprehensive identity management capabilities:

- **Dynamic Resource Management**: Register any resource type at runtime without code changes
- **Complete SCIM 2.0 Compliance**: Full implementation of SCIM protocol semantics and behaviors
- **Multi-Tenant Architecture**: Built-in tenant isolation with flexible URL generation strategies
- **Schema-Driven Operations**: Automatic validation and processing based on SCIM schemas
- **Pluggable Storage**: Storage-agnostic design works with any backend implementation
- **Production Ready**: Comprehensive error handling, logging, concurrency control, and observability
- **Zero Configuration**: Works out-of-the-box with sensible defaults while remaining highly configurable

## Architecture Overview

The SCIM Server operates as the orchestration hub in the library's layered architecture:

```text
SCIM Server (Orchestration Layer)
├── Resource Registration & Validation
├── Schema Management & Discovery
├── Operation Routing & Authorization
├── Multi-Tenant URL Generation
├── Concurrency & Version Control
└── Provider Abstraction
    ↓
Resource Provider (Business Logic)
    ↓
Storage Provider (Data Persistence)
```

### Core Components

1. **ScimServer Struct**: The main server instance with pluggable providers
2. **ScimServerBuilder**: Fluent configuration API for server setup
3. **Resource Registration**: Runtime registration of resource types and operations
4. **Schema Management**: Automatic schema validation and discovery
5. **Operation Router**: Dynamic dispatch to appropriate handlers
6. **URL Generation**: Multi-tenant aware endpoint URL creation

## Use Cases

### 1. Single-Tenant Identity Server

**Simple identity management for single organizations**

```rust
use scim_server::{ScimServer, ScimServerBuilder};
use scim_server::providers::StandardResourceProvider;
use scim_server::storage::InMemoryStorage;
use scim_server::resource::{RequestContext, ScimOperation};

// Setup server with provider
let storage = InMemoryStorage::new();
let provider = StandardResourceProvider::new(storage);
let mut server = ScimServer::new(provider)?;

// Register User resource type
server.register_resource_type(
    "User",
    user_handler,
    vec![ScimOperation::Create, ScimOperation::Read, ScimOperation::Update, ScimOperation::Delete]
)?;

// Create user through SCIM server
let context = RequestContext::new("request-123".to_string());
let user_data = json!({
    "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
    "userName": "alice@company.com",
    "displayName": "Alice Smith",
    "emails": [{"value": "alice@company.com", "primary": true}]
});

let created_user = server.create_resource("User", user_data, &context).await?;
```

**Benefits**: Automatic schema validation, metadata management, standardized error handling.

### 2. Multi-Tenant SaaS Platform

**Identity management for multiple customer organizations**

```rust
use scim_server::{ScimServerBuilder, TenantStrategy};
use scim_server::resource::{RequestContext, TenantContext, TenantPermissions};

// Configure multi-tenant server
let mut server = ScimServerBuilder::new(provider)
    .with_base_url("https://api.company.com")
    .with_tenant_strategy(TenantStrategy::PathBased)
    .build()?;

// Register resource types
server.register_resource_type("User", user_handler, user_operations)?;
server.register_resource_type("Group", group_handler, group_operations)?;

// Tenant-specific operations
let tenant_permissions = TenantPermissions {
    max_users: Some(1000),
    max_groups: Some(50),
    allowed_operations: vec!["create".into(), "read".into(), "update".into()],
};

let tenant_context = TenantContext {
    tenant_id: "customer-123".to_string(),
    client_id: "scim-client-1".to_string(),
    permissions: tenant_permissions,
};

let context = RequestContext::with_tenant_generated_id(tenant_context);

// Operations automatically scoped to tenant
let user = server.create_resource("User", user_data, &context).await?;
```

**Benefits**: Automatic tenant isolation, resource limits, tenant-specific URL generation.

### 3. Custom Resource Types

**Managing application-specific identity resources**

```rust
// Register custom resource type at runtime
let application_schema = Schema {
    id: "urn:example:schemas:Application".to_string(),
    name: "Application".to_string(),
    description: "Custom application resource".to_string(),
    attributes: vec![
        // Define custom attributes
        create_attribute("displayName", AttributeType::String, false, true, false),
        create_attribute("version", AttributeType::String, false, false, false),
        create_attribute("permissions", AttributeType::Complex, true, false, false),
    ],
};

let app_handler = ResourceHandler {
    resource_type: "Application".to_string(),
    schema: application_schema,
    endpoint: "/Applications".to_string(),
};

server.register_resource_type(
    "Application",
    app_handler,
    vec![ScimOperation::Create, ScimOperation::Read, ScimOperation::List]
)?;

// Use custom resource type
let app_data = json!({
    "schemas": ["urn:example:schemas:Application"],
    "displayName": "My Application",
    "version": "1.2.3",
    "permissions": [
        {"name": "read", "scope": "user"},
        {"name": "write", "scope": "admin"}
    ]
});

let application = server.create_resource("Application", app_data, &context).await?;
```

**Benefits**: No code changes for new resource types, automatic schema validation, consistent API.

### 4. Schema Discovery and Introspection

**Dynamic discovery of server capabilities**

```rust
// Automatic capability discovery
let capabilities = server.discover_capabilities()?;
println!("Supported resource types: {:?}", capabilities.resource_types);
println!("Supported operations: {:?}", capabilities.supported_operations);

// SCIM ServiceProviderConfig generation
let service_config = server.get_service_provider_config()?;
println!("Authentication schemes: {:?}", service_config.authentication_schemes);
println!("Bulk operations: {:?}", service_config.bulk);

// Schema introspection
let all_schemas = server.get_all_schemas();
for schema in all_schemas {
    println!("Schema: {} - {}", schema.id, schema.description);
}

// Resource type specific schema
let user_schema = server.get_resource_schema("User")?;
println!("User schema attributes: {}", user_schema.attributes.len());
```

**Benefits**: Automatic capability advertisement, standards-compliant discovery, runtime introspection.

### 5. Advanced URL Generation

**Flexible endpoint URL generation for different deployment patterns**

```rust
// Subdomain-based tenant isolation
let server = ScimServerBuilder::new(provider)
    .with_base_url("https://scim.company.com")
    .with_tenant_strategy(TenantStrategy::Subdomain)
    .build()?;

// Generates: https://tenant123.scim.company.com/v2/Users/user456
let ref_url = server.generate_ref_url(Some("tenant123"), "Users", "user456")?;

// Path-based tenant isolation
let server = ScimServerBuilder::new(provider)
    .with_base_url("https://api.company.com")
    .with_tenant_strategy(TenantStrategy::PathBased)
    .build()?;

// Generates: https://api.company.com/tenant123/v2/Users/user456
let ref_url = server.generate_ref_url(Some("tenant123"), "Users", "user456")?;

// Single tenant mode
let server = ScimServerBuilder::new(provider)
    .with_base_url("https://identity.company.com")
    .build()?;

// Generates: https://identity.company.com/v2/Users/user456
let ref_url = server.generate_ref_url(None, "Users", "user456")?;
```

**Benefits**: Flexible deployment patterns, proper SCIM $ref field generation, tenant-aware URLs.

## Design Patterns

### Builder Pattern for Configuration

The SCIM Server uses the builder pattern for flexible configuration:

```rust
pub struct ScimServerBuilder<P> {
    provider: P,
    config: ScimServerConfig,
}

impl<P: ResourceProvider> ScimServerBuilder<P> {
    pub fn new(provider: P) -> Self;
    pub fn with_base_url(self, base_url: impl Into<String>) -> Self;
    pub fn with_tenant_strategy(self, strategy: TenantStrategy) -> Self;
    pub fn with_scim_version(self, version: impl Into<String>) -> Self;
    pub fn build(self) -> Result<ScimServer<P>, ScimError>;
}
```

This allows for fluent, type-safe configuration while maintaining defaults.

### Dynamic Resource Registration

Resources are registered at runtime without compile-time dependencies:

```rust
pub fn register_resource_type(
    &mut self,
    resource_type: &str,
    handler: ResourceHandler,
    operations: Vec<ScimOperation>,
) -> Result<(), ScimError>
```

This enables:
- Plugin architectures
- Configuration-driven resource types
- Runtime schema evolution
- Multi-version support

### Provider Abstraction

The server is generic over any `ResourceProvider` implementation:

```rust
pub struct ScimServer<P> {
    provider: P,
    // ...
}

impl<P: ResourceProvider + Sync> ScimServer<P> {
    // Operations delegate to provider
}
```

This enables:
- Pluggable storage backends
- Custom business logic
- Testing with mock providers
- Incremental migration strategies

## Integration with Other Components

### Resource Integration

The SCIM Server works seamlessly with the Resource system:

- **Type Safety**: Core attributes use validated value objects
- **Flexibility**: Extended attributes remain as JSON
- **Serialization**: Automatic $ref field injection for SCIM compliance
- **Metadata**: Automatic timestamp and version management

### Resource Provider Integration

The server orchestrates provider operations:

- **Operation Dispatch**: Routes operations to appropriate provider methods
- **Context Passing**: Ensures request context flows through all operations
- **Error Translation**: Converts provider errors to SCIM-compliant responses
- **Concurrency**: Manages version-aware operations for conflict prevention

### Storage Provider Integration

Through the Resource Provider layer:

- **Storage Agnostic**: Works with any storage implementation
- **Transaction Support**: Leverages provider transaction capabilities
- **Bulk Operations**: Coordinates multi-resource operations
- **Query Translation**: Converts SCIM queries to storage-specific formats

### Schema Integration

Deep integration with the schema system:

- **Automatic Validation**: All operations validated against registered schemas
- **Schema Discovery**: Runtime introspection of available schemas
- **Extension Support**: Handles custom schema extensions transparently
- **Compliance Checking**: Ensures SCIM 2.0 specification adherence

## Error Handling

The SCIM Server provides comprehensive error handling:

### Structured Error Types

```rust
pub enum ScimError {
    UnsupportedResourceType(String),
    UnsupportedOperation { resource_type: String, operation: String },
    SchemaValidation { schema_id: String, message: String },
    InvalidRequest { message: String },
    ResourceNotFound { resource_type: String, id: String },
    ConflictError { message: String },
    // ...
}
```

### Operation-Specific Error Handling

Each operation handles errors appropriately:

- **Create**: Schema validation, uniqueness conflicts, tenant limits
- **Read**: Resource not found, authorization failures
- **Update**: Version conflicts, schema validation, immutable field protection
- **Delete**: Resource not found, referential integrity
- **List/Search**: Query validation, pagination errors

### Provider Error Translation

Provider errors are automatically translated to SCIM-compliant responses:

```rust
let result = self
    .provider
    .create_resource(resource_type, data, context)
    .await
    .map_err(|e| ScimError::ProviderError(e.to_string()));
```

## Best Practices

### 1. Use Builder Pattern for Configuration

Always use the builder for server setup:

```rust
// Good: Explicit configuration
let server = ScimServerBuilder::new(provider)
    .with_base_url("https://api.company.com")
    .with_tenant_strategy(TenantStrategy::PathBased)
    .build()?;

// Avoid: Default constructor for production
let server = ScimServer::new(provider)?; // Uses localhost defaults
```

### 2. Register All Required Operations

Be explicit about supported operations:

```rust
// Good: Explicit operation support
server.register_resource_type(
    "User",
    user_handler,
    vec![ScimOperation::Create, ScimOperation::Read, ScimOperation::Update]
)?;

// Avoid: Supporting all operations by default
// Some providers may not support all operations efficiently
```

### 3. Handle Multi-Tenancy Consistently

Choose a tenant strategy and use it consistently:

```rust
// Good: Consistent tenant strategy
let server = ScimServerBuilder::new(provider)
    .with_tenant_strategy(TenantStrategy::PathBased)
    .build()?;

// All operations automatically handle tenant isolation
let user = server.create_resource("User", data, &tenant_context).await?;

// Avoid: Manual tenant handling
// Let the server handle tenant isolation automatically
```

### 4. Leverage Schema Validation

Trust the automatic schema validation:

```rust
// Good: Let server validate automatically
let result = server.create_resource("User", user_data, &context).await;
match result {
    Ok(user) => process_user(user),
    Err(ScimError::SchemaValidation { message, .. }) => handle_validation_error(message),
    Err(e) => handle_other_error(e),
}

// Avoid: Manual validation before server operations
// The server already provides comprehensive validation
```

### 5. Use Proper Error Handling

Handle different error types appropriately:

```rust
// Good: Structured error handling
match server.get_resource("User", id, &context).await {
    Ok(Some(user)) => Ok(user),
    Ok(None) => Err(HttpError::NotFound),
    Err(ScimError::UnsupportedResourceType(_)) => Err(HttpError::BadRequest),
    Err(ScimError::ProviderError(_)) => Err(HttpError::InternalServerError),
    Err(e) => Err(HttpError::from(e)),
}

// Avoid: Generic error handling
// Loses important context for proper HTTP responses
```

## When to Use SCIM Server Directly

### Primary Use Cases

1. **HTTP Server Implementation**: Building REST APIs that expose SCIM endpoints
2. **Application Integration**: Embedding SCIM capabilities into existing applications
3. **Identity Bridges**: Creating adapters between different identity systems
4. **Testing Frameworks**: Building test harnesses for SCIM compliance
5. **Custom Protocols**: Implementing SCIM over non-HTTP transports

### Implementation Strategies

| Scenario | Approach | Complexity |
|----------|----------|------------|
| Simple REST API | Use with HTTP framework | Low |
| Multi-tenant SaaS | Builder with tenant strategy | Medium |
| Custom Resources | Runtime registration | Medium |
| Protocol Bridge | Custom resource provider | High |
| Embedded Identity | Direct server integration | Medium |

## Comparison with Alternative Approaches

| Approach | Flexibility | Compliance | Performance | Complexity |
|----------|-------------|------------|-------------|------------|
| **SCIM Server** | ✅ Very High | ✅ Complete | ✅ High | Medium |
| Hard-coded Resources | ❌ Low | ⚠️ Partial | ✅ Very High | Low |
| Generic REST Framework | ✅ High | ❌ Manual | ✅ High | High |
| Identity Provider SDK | ⚠️ Medium | ✅ High | ⚠️ Medium | Low |

The SCIM Server provides the optimal balance of flexibility, compliance, and performance for identity management scenarios, offering complete SCIM 2.0 implementation while remaining adaptable to diverse deployment requirements.

## Relationship to HTTP Layer

While the SCIM Server handles protocol semantics, it's designed to work with any HTTP framework:

- **Framework Agnostic**: No dependencies on specific HTTP libraries
- **Clean Separation**: HTTP concerns handled separately from SCIM logic
- **Easy Integration**: Simple async interface maps directly to HTTP handlers
- **Standard Responses**: Returns structured data suitable for JSON serialization

This design enables the SCIM Server to serve as the core for various deployment scenarios, from embedded applications to high-performance web services, while maintaining full SCIM 2.0 compliance and providing the flexibility needed for real-world identity management systems.