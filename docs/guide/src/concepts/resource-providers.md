# Resource Providers

Resource Providers form the business logic layer of the SCIM Server architecture, implementing SCIM protocol semantics while remaining agnostic to storage implementation details. They bridge the gap between HTTP requests and data persistence, handling validation, metadata management, concurrency control, and multi-tenancy.

See the [ResourceProvider API documentation](https://docs.rs/scim-server/latest/scim_server/trait.ResourceProvider.html) for complete details.

## Value Proposition

Resource Providers deliver several critical capabilities:

- **SCIM Protocol Compliance**: Full implementation of SCIM 2.0 semantics and behaviors
- **Business Logic Separation**: Clean separation between protocol logic and storage concerns
- **Multi-Tenancy Support**: Built-in tenant isolation and resource limits
- **Concurrency Control**: Optimistic locking with version-aware operations
- **Pluggable Architecture**: Storage-agnostic design enables diverse backends
- **Production Ready**: Comprehensive error handling, logging, and observability

## Architecture Overview

Resource Providers operate as the orchestration layer in the SCIM Server stack:

```text
HTTP Layer
    ↓
Resource Provider (Business Logic)
├── SCIM Protocol Logic
├── Validation & Metadata
├── Concurrency Control
├── Multi-Tenancy
└── Error Handling
    ↓
Storage Provider (Data Persistence)
```

### Key Components

1. **[`ResourceProvider` Trait](https://docs.rs/scim-server/latest/scim_server/trait.ResourceProvider.html)**: Unified interface for all SCIM operations
2. **[`StandardResourceProvider`](https://docs.rs/scim-server/latest/scim_server/providers/struct.StandardResourceProvider.html)**: Production-ready implementation
3. **[Helper Traits](https://docs.rs/scim-server/latest/scim_server/providers/index.html)**: Composable functionality for custom providers
4. **[Context Management](https://docs.rs/scim-server/latest/scim_server/struct.RequestContext.html)**: Request scoping and tenant isolation

## Core Interface

The [`ResourceProvider` trait](https://docs.rs/scim-server/latest/scim_server/trait.ResourceProvider.html) defines the contract for SCIM operations:

```rust
pub trait ResourceProvider {
    type Error: std::error::Error + Send + Sync + 'static;

    // Core CRUD operations
    async fn create_resource(&self, resource_type: &str, data: Value, context: &RequestContext) 
        -> Result<VersionedResource, Self::Error>;
    
    async fn get_resource(&self, resource_type: &str, id: &str, context: &RequestContext) 
        -> Result<Option<VersionedResource>, Self::Error>;
    
    async fn update_resource(&self, resource_type: &str, id: &str, data: Value, 
        expected_version: Option<&RawVersion>, context: &RequestContext) 
        -> Result<VersionedResource, Self::Error>;
    
    async fn delete_resource(&self, resource_type: &str, id: &str, 
        expected_version: Option<&RawVersion>, context: &RequestContext) 
        -> Result<(), Self::Error>;
    
    // Query operations
    async fn list_resources(&self, resource_type: &str, query: Option<&ListQuery>, 
        context: &RequestContext) -> Result<Vec<VersionedResource>, Self::Error>;
    
    async fn find_resources_by_attribute(&self, resource_type: &str, 
        attribute_name: &str, attribute_value: &str, context: &RequestContext) 
        -> Result<Vec<VersionedResource>, Self::Error>;
    
    // Advanced operations
    async fn patch_resource(&self, resource_type: &str, id: &str, 
        patch_request: &Value, expected_version: Option<&RawVersion>, 
        context: &RequestContext) -> Result<VersionedResource, Self::Error>;
    
    async fn resource_exists(&self, resource_type: &str, id: &str, 
        context: &RequestContext) -> Result<bool, Self::Error>;
}
```

## Use Cases

### 1. Single-Tenant SCIM Server

**Simple identity management for single organizations**

```rust
use scim_server::providers::StandardResourceProvider;
use scim_server::storage::InMemoryStorage;
use scim_server::resource::RequestContext;

// Setup
let storage = InMemoryStorage::new();
let provider = StandardResourceProvider::new(storage);

// Single-tenant context (no tenant isolation)
let context = RequestContext::with_generated_id();

// Create user
let user_data = json!({
    "userName": "alice@company.com",
    "displayName": "Alice Smith",
    "emails": [{"value": "alice@company.com", "primary": true}]
});

let user = provider.create_resource("User", user_data, &context).await?;
println!("Created user: {}", user.resource().get_id().unwrap());
```

**Benefits**: Simplified setup, automatic metadata management, built-in validation.

### 2. Multi-Tenant SaaS Platform

**Identity management for multiple customer organizations**

```rust
use scim_server::resource::{RequestContext, TenantContext, TenantPermissions};

// Configure tenant with resource limits
let permissions = TenantPermissions {
    max_users: Some(1000),
    max_groups: Some(50),
    allowed_operations: vec!["create".into(), "read".into(), "update".into()],
};

let tenant_context = TenantContext {
    tenant_id: "customer-123".to_string(),
    client_id: "scim-client-1".to_string(),
    permissions,
};

let context = RequestContext::with_tenant_generated_id(tenant_context);

// Operations are automatically scoped to this tenant
let user = provider.create_resource("User", user_data, &context).await?;

// This user only exists within "customer-123" tenant
let retrieved = provider.get_resource("User", &user.resource().get_id().unwrap(), &context).await?;
```

**Benefits**: Automatic tenant isolation, resource limits, per-tenant permissions.

### 3. Version-Aware Operations

**Preventing lost updates in concurrent environments**

```rust
// Get current resource with version
let user = provider.get_resource("User", "123", &context).await?.unwrap();
let current_version = user.version();

// Modify user data
let mut updated_data = user.resource().to_json()?;
updated_data["displayName"] = json!("Updated Name");

// Conditional update - only succeeds if version matches
match provider.update_resource("User", "123", updated_data, 
    Some(current_version), &context).await {
    Ok(updated_user) => println!("Update successful"),
    Err(ProviderError::PreconditionFailed { .. }) => {
        println!("Resource was modified by another process");
        // Handle conflict resolution
    }
}
```

**Benefits**: Prevents lost updates, enables conflict detection, maintains data consistency.

### 4. Custom Business Logic

**Implementing domain-specific validation and processing**

```rust
use scim_server::providers::ResourceProvider;

pub struct CustomResourceProvider<S: StorageProvider> {
    standard: StandardResourceProvider<S>,
    audit_logger: AuditLogger,
}

impl<S: StorageProvider> ResourceProvider for CustomResourceProvider<S> {
    type Error = ProviderError;

    async fn create_resource(&self, resource_type: &str, mut data: Value, 
        context: &RequestContext) -> Result<VersionedResource, Self::Error> {
        
        // Custom validation
        if resource_type == "User" {
            self.validate_company_email(&data)?;
            self.assign_department(&mut data, context)?;
        }

        // Delegate to standard implementation
        let resource = self.standard.create_resource(resource_type, data, context).await?;

        // Custom post-processing
        self.audit_logger.log_creation(&resource, context).await;
        self.send_welcome_email(&resource).await;

        Ok(resource)
    }
    
    // ... other methods delegate to standard provider ...
}
```

**Benefits**: Extend standard behavior, add custom validation, integrate with external systems.

## Implementation Patterns

### 1. Delegating Provider Pattern

Build on top of `StandardResourceProvider` for custom logic:

```rust
pub struct EnterpriseProvider<S> {
    standard: StandardResourceProvider<S>,
    ldap_sync: LdapSync,
    compliance_checker: ComplianceChecker,
}

impl<S: StorageProvider> EnterpriseProvider<S> {
    // Override specific operations while delegating others
    async fn create_user_with_compliance(&self, data: Value, context: &RequestContext) 
        -> Result<VersionedResource, ProviderError> {
        
        // Pre-creation compliance check
        self.compliance_checker.validate_user_data(&data)?;
        
        // Standard creation
        let user = self.standard.create_resource("User", data, context).await?;
        
        // Post-creation sync
        self.ldap_sync.sync_user(&user).await?;
        
        Ok(user)
    }
}
```

### 2. Middleware Provider Pattern

Chain multiple providers for cross-cutting concerns:

```rust
pub struct LoggingProvider<P> {
    inner: P,
    logger: Logger,
}

impl<P: ResourceProvider> ResourceProvider for LoggingProvider<P> {
    type Error = P::Error;

    async fn create_resource(&self, resource_type: &str, data: Value, 
        context: &RequestContext) -> Result<VersionedResource, Self::Error> {
        
        let start = Instant::now();
        self.logger.info("Creating {} resource", resource_type);
        
        let result = self.inner.create_resource(resource_type, data, context).await;
        
        self.logger.info("Create operation completed in {:?}", start.elapsed());
        result
    }
}
```

### 3. Storage-Agnostic Provider

Work with any storage backend:

```rust
// Works with in-memory storage for testing
let memory_provider = StandardResourceProvider::new(InMemoryStorage::new());

// Works with SQLite for persistence
let sqlite_provider = StandardResourceProvider::new(SqliteStorage::new("users.db")?);

// Works with custom storage implementations
let custom_provider = StandardResourceProvider::new(MyCustomStorage::new());
```

## Helper Traits

Resource Providers compose functionality through helper traits:

### ScimMetadataManager

Handles SCIM metadata (timestamps, versions, locations):

```rust
use scim_server::providers::helpers::ScimMetadataManager;

// Automatically implemented for providers
impl<S> ScimMetadataManager for StandardResourceProvider<S> {
    fn add_creation_metadata(&self, resource: &mut Resource, base_url: &str) -> Result<(), String>;
    fn update_modification_metadata(&self, resource: &mut Resource) -> Result<(), String>;
}
```

### MultiTenantProvider

Manages tenant isolation and resource limits:

```rust
use scim_server::providers::helpers::MultiTenantProvider;

// Provides tenant-aware ID generation and validation
impl<S> MultiTenantProvider for StandardResourceProvider<S> {
    fn effective_tenant_id(&self, context: &RequestContext) -> String;
    fn generate_tenant_resource_id(&self, tenant_id: &str, resource_type: &str) -> String;
}
```

### ScimPatchOperations

Implements SCIM PATCH semantics:

```rust
use scim_server::providers::helpers::ScimPatchOperations;

// Handles complex PATCH operations
impl<S> ScimPatchOperations for StandardResourceProvider<S> {
    fn apply_patch_operation(&self, data: &mut Value, operation: &Value) -> Result<(), ProviderError>;
}
```

## Best Practices

### 1. Use Standard Provider as Base

Start with `StandardResourceProvider` and extend as needed:

```rust
// Good: Build on proven foundation
let provider = StandardResourceProvider::new(storage);

// Avoid: Implementing from scratch unless necessary
struct FullCustomProvider; // Requires implementing all SCIM logic
```

### 2. Delegate Storage Concerns

Keep providers focused on business logic:

```rust
// Good: Provider handles SCIM logic, storage handles persistence
let result = self.storage.put(key, processed_data).await?;

// Avoid: Provider handling storage implementation details
let result = self.write_to_database_with_connection_pooling(data).await?;
```

### 3. Handle Errors Appropriately

Use structured error types for better error handling:

```rust
// Good: Specific error types enable proper HTTP status codes
return Err(ProviderError::DuplicateAttribute { 
    resource_type: "User".to_string(),
    attribute: "userName".to_string(),
    // ...
});

// Avoid: Generic errors lose important context
return Err("duplicate username".into());
```

### 4. Leverage Context Information

Use `RequestContext` for operation scoping:

```rust
// Good: Context-aware operations
let tenant_id = context.tenant_id().unwrap_or("default");
context.validate_operation("create")?;

// Avoid: Hardcoded assumptions
let tenant_id = "default"; // Breaks multi-tenancy
```

## When to Implement Custom Providers

### Scenarios for Custom Implementation

1. **Complex Business Rules**: Domain-specific validation beyond SCIM
2. **External System Integration**: Real-time sync with HR systems, directories
3. **Compliance Requirements**: Audit logging, data residency, encryption
4. **Performance Optimization**: Caching, batching, specialized queries
5. **Legacy System Integration**: Adapting existing identity stores

### Implementation Strategies

| Requirement | Approach | Complexity |
|-------------|----------|------------|
| Simple Extensions | Delegate to Standard | Low |
| Custom Validation | Override Specific Methods | Medium |
| External Integration | Middleware Pattern | Medium |
| Full Custom Logic | Implement from Trait | High |

The Resource Provider layer is where SCIM Server's flexibility shines, allowing you to implement exactly the business logic your application requires while leveraging battle-tested infrastructure for storage, HTTP handling, and protocol compliance.