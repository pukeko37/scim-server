# Provider Architecture

This document explains the two-layer architecture of the SCIM Server library and how storage and resource providers work together.

## Overview

The SCIM Server uses a clean separation between storage concerns and SCIM protocol logic through two main abstractions:

```
┌─────────────────────────────────────────────┐
│                SCIM Server                  │
├─────────────────────────────────────────────┤
│           ResourceProvider Layer            │
│  (SCIM protocol logic, validation, etc.)   │
├─────────────────────────────────────────────┤
│           StorageProvider Layer             │
│    (Pure data persistence operations)      │
└─────────────────────────────────────────────┘
```

## Two-Layer Architecture

### StorageProvider Layer (Low-Level)

The `StorageProvider` trait defines pure data persistence operations that are protocol-agnostic:

```rust
pub trait StorageProvider: Send + Sync {
    type Error: std::error::Error + Send + Sync + 'static;

    // Core operations
    async fn put(&self, key: StorageKey, data: Value) -> Result<Value, Self::Error>;
    async fn get(&self, key: StorageKey) -> Result<Option<Value>, Self::Error>;
    async fn delete(&self, key: StorageKey) -> Result<bool, Self::Error>;
    
    // Query operations
    async fn list(&self, prefix: StoragePrefix, offset: usize, limit: usize) 
        -> Result<Vec<(StorageKey, Value)>, Self::Error>;
    async fn find_by_attribute(&self, prefix: StoragePrefix, attribute: &str, value: &str) 
        -> Result<Vec<(StorageKey, Value)>, Self::Error>;
    async fn exists(&self, key: StorageKey) -> Result<bool, Self::Error>;
    async fn count(&self, prefix: StoragePrefix) -> Result<usize, Self::Error>;
}
```

**Responsibilities:**
- Pure PUT/GET/DELETE operations on JSON data
- Tenant isolation through hierarchical keys
- Basic querying and filtering
- Data persistence and retrieval

**Not Responsible For:**
- SCIM metadata generation (timestamps, versions, etc.)
- SCIM validation rules
- Business logic (limits, permissions, etc.)
- Protocol-specific transformations

### ResourceProvider Layer (High-Level)

The `ResourceProvider` trait defines SCIM-aware operations:

```rust
pub trait ResourceProvider: Send + Sync {
    type Error: std::error::Error + Send + Sync + 'static;

    // SCIM operations
    async fn create_resource(&self, resource_type: &str, data: Value, context: &RequestContext) 
        -> Result<Resource, Self::Error>;
    async fn get_resource(&self, resource_type: &str, id: &str, context: &RequestContext) 
        -> Result<Option<Resource>, Self::Error>;
    async fn update_resource(&self, resource_type: &str, id: &str, data: Value, context: &RequestContext) 
        -> Result<Resource, Self::Error>;
    async fn delete_resource(&self, resource_type: &str, id: &str, context: &RequestContext) 
        -> Result<bool, Self::Error>;
    async fn list_resources(&self, resource_type: &str, query: Option<ListQuery>, context: &RequestContext) 
        -> Result<Vec<Resource>, Self::Error>;
    async fn find_resource_by_attribute(&self, resource_type: &str, attribute: &str, value: &Value, context: &RequestContext) 
        -> Result<Option<Resource>, Self::Error>;
    async fn patch_resource(&self, resource_type: &str, id: &str, patch: Value, context: &RequestContext) 
        -> Result<Resource, Self::Error>;
}
```

**Responsibilities:**
- SCIM metadata generation (timestamps, ETags, versions)
- SCIM validation and business rules
- Request context handling (tenant isolation)
- Resource type management
- Patch operation processing
- Error translation from storage to SCIM errors

## Key Design Principles

### 1. Separation of Concerns

**Storage Layer** handles "where" and "how" data is stored:
- Database connections
- File systems
- Memory structures
- Indexing and optimization

**Resource Layer** handles "what" the data means:
- SCIM protocol compliance
- Resource validation
- Metadata management
- Business logic

### 2. PUT/GET/DELETE Model

The storage layer uses a simple model where CREATE and UPDATE are both PUT operations:

```rust
// Both create and update use the same operation
let stored = storage.put(key, data).await?;
```

The distinction between "create" vs "update" is business logic that belongs in the ResourceProvider layer.

### 3. Tenant Isolation

All storage operations are scoped by tenant through the `StorageKey` structure:

```rust
pub struct StorageKey {
    tenant_id: String,      // "tenant-1" or "default"
    resource_type: String,  // "User", "Group", etc.
    resource_id: String,    // "user-123"
}
```

This provides natural tenant isolation without requiring complex tenant management systems.

### 4. Context-Driven Operations

The ResourceProvider uses `RequestContext` to determine operational mode:

```rust
// Single-tenant operation
let context = RequestContext::with_generated_id();

// Multi-tenant operation  
let tenant_context = TenantContext::new("tenant-1".to_string(), "client-1".to_string());
let context = RequestContext::with_tenant_generated_id(tenant_context);
```

## Implementation Patterns

### Standard Provider Pattern

The most common pattern is using `StandardResourceProvider` with a pluggable storage backend:

```rust
use scim_server::{
    providers::StandardResourceProvider,
    storage::InMemoryStorage,
};

// Create storage backend
let storage = InMemoryStorage::new();

// Create resource provider with storage
let provider = StandardResourceProvider::new(storage);
```

### Direct Implementation Pattern

For simple use cases, you can implement `ResourceProvider` directly:

```rust
use scim_server::{
    providers::InMemoryProvider,
    resource::ResourceProvider,
};

// Direct implementation (deprecated in favor of StandardResourceProvider)
let provider = InMemoryProvider::new();
```

## Available Implementations

### Storage Providers

1. **InMemoryStorage**
   - Thread-safe in-memory storage using `HashMap`
   - Suitable for testing and development
   - No persistence across restarts

2. **Custom Storage** (implement `StorageProvider`)
   - Database backends (PostgreSQL, MySQL, etc.)
   - File-based storage
   - Cloud storage systems
   - Distributed storage systems

### Resource Providers

1. **StandardResourceProvider<S>**
   - Production-ready implementation
   - Works with any `StorageProvider`
   - Full SCIM protocol support
   - Automatic tenant isolation

2. **InMemoryProvider** (Legacy)
   - Direct in-memory implementation
   - Deprecated in favor of `StandardResourceProvider + InMemoryStorage`
   - Maintained for backward compatibility

## Data Flow

Here's how a typical request flows through the architecture:

```
1. HTTP Request → SCIM Server
2. SCIM Server → ResourceProvider.create_resource()
3. ResourceProvider:
   - Validates SCIM data
   - Generates metadata (timestamps, ETag)
   - Determines tenant from RequestContext
   - Creates StorageKey
4. ResourceProvider → StorageProvider.put()
5. StorageProvider:
   - Stores JSON data at key
   - Returns stored data
6. ResourceProvider:
   - Creates Resource from stored data
   - Returns Resource to SCIM Server
7. SCIM Server → HTTP Response
```

## Error Handling

The architecture uses layered error handling:

### Storage Errors
```rust
pub enum StorageError {
    NotFound(String),
    Conflict(String),
    Internal(String),
    // ...
}
```

### Resource Provider Errors
```rust
// Each provider defines its own error type
impl From<StorageError> for MyProviderError {
    fn from(storage_error: StorageError) -> Self {
        // Transform storage errors to provider errors
    }
}
```

## Multi-Tenancy Architecture

### Automatic Tenant Isolation

The architecture provides automatic tenant isolation through the key hierarchy:

```
Storage Layout:
├── tenant-1/
│   ├── User/
│   │   ├── user-123 → {user data}
│   │   └── user-456 → {user data}
│   └── Group/
│       └── group-789 → {group data}
├── tenant-2/
│   ├── User/
│   │   └── user-123 → {different user data}
│   └── Group/
└── default/  (single-tenant mode)
    ├── User/
    └── Group/
```

### Context-Based Routing

The `RequestContext` determines which tenant namespace to use:

```rust
fn effective_tenant_id(context: &RequestContext) -> &str {
    context.tenant_context
        .as_ref()
        .map(|tc| tc.tenant_id.as_str())
        .unwrap_or("default")
}
```

## Performance Considerations

### Storage Layer Optimizations

- **Connection Pooling**: Implement at the storage layer
- **Caching**: Can be added as a storage layer decorator
- **Indexing**: Handle in the storage implementation
- **Batching**: Implement batch operations in storage

### Resource Layer Optimizations

- **Resource Caching**: Cache parsed Resource objects
- **Metadata Caching**: Cache computed metadata
- **Validation Caching**: Cache validation results

## Testing Strategy

### Unit Testing Storage

Test storage providers independently:

```rust
#[tokio::test]
async fn test_storage_operations() {
    let storage = MyStorage::new();
    let key = StorageKey::new("tenant1", "User", "123");
    let data = json!({"userName": "test"});
    
    // Test put/get/delete cycle
    storage.put(key.clone(), data.clone()).await.unwrap();
    let retrieved = storage.get(key.clone()).await.unwrap();
    assert_eq!(retrieved, Some(data));
}
```

### Integration Testing

Test the full stack with both layers:

```rust
#[tokio::test]
async fn test_full_stack() {
    let storage = InMemoryStorage::new();
    let provider = StandardResourceProvider::new(storage);
    let context = RequestContext::with_generated_id();
    
    let user = provider.create_resource(
        "User",
        json!({"userName": "test"}),
        &context,
    ).await.unwrap();
    
    assert!(user.get_id().is_some());
}
```

## Extension Points

### Custom Storage Backends

Implement `StorageProvider` for custom backends:

```rust
struct MyDatabaseStorage {
    pool: ConnectionPool,
}

impl StorageProvider for MyDatabaseStorage {
    type Error = MyStorageError;
    
    async fn put(&self, key: StorageKey, data: Value) -> Result<Value, Self::Error> {
        // Database-specific implementation
    }
    
    // ... implement other methods
}
```

### Custom Resource Logic

Extend `StandardResourceProvider` or implement `ResourceProvider` directly:

```rust
struct CustomResourceProvider<S> {
    storage: S,
    validator: CustomValidator,
}

impl<S: StorageProvider> ResourceProvider for CustomResourceProvider<S> {
    type Error = CustomProviderError;
    
    async fn create_resource(&self, resource_type: &str, data: Value, context: &RequestContext) -> Result<Resource, Self::Error> {
        // Custom validation and processing
        self.validator.validate(&data)?;
        
        // Delegate to storage
        let key = self.build_key(resource_type, context);
        let stored = self.storage.put(key, data).await?;
        
        // Custom post-processing
        Ok(Resource::from_json(resource_type.to_string(), stored)?)
    }
}
```

## Next Steps

- [Basic Implementation](./basic.md) - Learn how to implement storage providers
- [Advanced Features](./advanced.md) - Explore advanced provider capabilities
- [Testing](./testing.md) - Comprehensive testing strategies