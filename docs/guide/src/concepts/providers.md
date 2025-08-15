# Storage Providers

Storage providers are the backbone of the SCIM Server library, handling all data persistence and retrieval operations. The library uses a two-layer architecture that cleanly separates storage concerns from SCIM protocol logic.

## Overview

The SCIM Server implements data access through two complementary abstractions:

- **StorageProvider**: Low-level trait for pure data persistence operations
- **ResourceProvider**: High-level trait for SCIM-aware resource management

This separation allows you to plug in different storage backends (database, file system, cloud storage) without changing SCIM protocol logic, and conversely modify SCIM behavior without touching storage implementation.

```rust
use scim_server::{
    providers::StandardResourceProvider,
    storage::InMemoryStorage,
    resource::{RequestContext, ResourceProvider},
};

// Create storage backend
let storage = InMemoryStorage::new();

// Create SCIM provider with storage
let provider = StandardResourceProvider::new(storage);

// Use for SCIM operations
let context = RequestContext::with_generated_id();
let user = provider.create_resource("User", user_data, &context).await?;
```

## StorageProvider Layer

The `StorageProvider` trait defines protocol-agnostic storage operations:

### Core Operations

```rust
pub trait StorageProvider: Send + Sync {
    type Error: std::error::Error + Send + Sync + 'static;

    // Basic CRUD operations
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

### Tenant Isolation

All storage operations are scoped by tenant through hierarchical keys:

```rust
pub struct StorageKey {
    tenant_id: String,      // "tenant-1" or "default"
    resource_type: String,  // "User", "Group", etc.
    resource_id: String,    // "user-123"
}

// Examples:
// StorageKey::new("tenant-1", "User", "alice-123")
// StorageKey::new("default", "Group", "admins-456")
```

This provides automatic tenant isolation without complex tenant management systems.

## Built-in Storage Providers

### InMemoryStorage

Thread-safe in-memory storage using `HashMap`:

```rust
use scim_server::storage::InMemoryStorage;

let storage = InMemoryStorage::new();

// Get statistics
let stats = storage.stats().await;
println!("Total resources: {}", stats.total_resources);
println!("Tenants: {}", stats.tenant_count);
```

**Use Cases:**
- Development and testing
- Proof of concepts
- Small deployments without persistence requirements

**Characteristics:**
- Thread-safe with `RwLock`
- No persistence across restarts
- Excellent performance for development
- Built-in statistics and metrics

### Custom Storage Implementation

Implement `StorageProvider` for custom backends:

```rust
use scim_server::storage::{StorageProvider, StorageKey, StorageError};
use serde_json::Value;

#[derive(Clone)]
pub struct DatabaseStorage {
    pool: sqlx::PgPool,
}

impl StorageProvider for DatabaseStorage {
    type Error = StorageError;
    
    async fn put(&self, key: StorageKey, data: Value) -> Result<Value, Self::Error> {
        sqlx::query!(
            "INSERT INTO scim_resources (tenant_id, resource_type, resource_id, data) 
             VALUES ($1, $2, $3, $4) 
             ON CONFLICT (tenant_id, resource_type, resource_id) 
             DO UPDATE SET data = $4, updated_at = NOW()",
            key.tenant_id(),
            key.resource_type(), 
            key.resource_id(),
            data
        )
        .execute(&self.pool)
        .await
        .map_err(|e| StorageError::Internal(e.to_string()))?;
        
        Ok(data)
    }
    
    async fn get(&self, key: StorageKey) -> Result<Option<Value>, Self::Error> {
        let row = sqlx::query!(
            "SELECT data FROM scim_resources 
             WHERE tenant_id = $1 AND resource_type = $2 AND resource_id = $3",
            key.tenant_id(),
            key.resource_type(),
            key.resource_id()
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| StorageError::Internal(e.to_string()))?;
        
        Ok(row.map(|r| r.data))
    }
    
    // ... implement other methods
}
```

## ResourceProvider Layer

The `ResourceProvider` trait handles SCIM-specific logic:

### Standard Implementation

Most applications use `StandardResourceProvider` with a pluggable storage backend:

```rust
use scim_server::providers::StandardResourceProvider;

let storage = DatabaseStorage::new(pool);
let provider = StandardResourceProvider::new(storage);

// The provider handles:
// - SCIM metadata generation (timestamps, ETags)
// - Resource validation
// - Tenant context processing
// - Error translation
```

### Direct Implementation

For custom SCIM behavior, implement `ResourceProvider` directly:

```rust
use scim_server::resource::{ResourceProvider, Resource, RequestContext};

pub struct CustomResourceProvider {
    storage: Box<dyn StorageProvider<Error = StorageError>>,
    validator: CustomValidator,
}

impl ResourceProvider for CustomResourceProvider {
    type Error = CustomError;
    
    async fn create_resource(
        &self,
        resource_type: &str,
        data: Value,
        context: &RequestContext,
    ) -> Result<Resource, Self::Error> {
        // Custom validation
        self.validator.validate_resource(resource_type, &data)?;
        
        // Custom metadata
        let enriched_data = self.add_custom_metadata(data, context)?;
        
        // Delegate to storage
        let key = self.build_storage_key(resource_type, context);
        let stored = self.storage.put(key, enriched_data).await?;
        
        Ok(Resource::from_json(resource_type.to_string(), stored)?)
    }
    
    // ... implement other methods
}
```

## Multi-Tenancy Support

### Context-Driven Isolation

The library provides automatic tenant isolation through `RequestContext`:

```rust
use scim_server::resource::{RequestContext, TenantContext};

// Single-tenant operation (uses "default" tenant)
let single_context = RequestContext::with_generated_id();

// Multi-tenant operation
let tenant_context = TenantContext::new(
    "customer-123".to_string(),
    "app-456".to_string(),
);
let multi_context = RequestContext::with_tenant_generated_id(tenant_context);

// Same provider, different tenant isolation
let user1 = provider.create_resource("User", data1, &single_context).await?;
let user2 = provider.create_resource("User", data2, &multi_context).await?;
```

### Storage Layout

Resources are automatically organized by tenant:

```
Storage Hierarchy:
├── default/           # Single-tenant operations
│   ├── User/
│   │   ├── user-1 → {user data}
│   │   └── user-2 → {user data}
│   └── Group/
│       └── group-1 → {group data}
├── customer-123/      # Tenant-specific data
│   ├── User/
│   │   └── user-1 → {different user data}
│   └── Group/
└── customer-456/      # Another tenant
    └── User/
        └── user-1 → {yet different user data}
```

## Error Handling

### Storage Error Types

```rust
use scim_server::storage::StorageError;

pub enum StorageError {
    NotFound(String),
    Conflict(String),
    Internal(String),
}

// Usage in custom storage
impl StorageProvider for MyStorage {
    type Error = StorageError;
    
    async fn get(&self, key: StorageKey) -> Result<Option<Value>, Self::Error> {
        self.database.get(&key)
            .await
            .map_err(|e| StorageError::Internal(e.to_string()))
    }
}
```

### Error Propagation

The architecture provides clean error propagation from storage to SCIM:

```rust
StorageError → ResourceProviderError → SCIM HTTP Status
NotFound     → ResourceNotFound       → 404 Not Found
Conflict     → ResourceConflict       → 409 Conflict
Internal     → InternalError          → 500 Internal Server Error
```

## Performance Considerations

### Storage Layer Optimizations

```rust
// Connection pooling in storage
pub struct PooledStorage {
    pool: Arc<Pool<PostgresConnectionManager>>,
}

// Caching decorator
pub struct CachedStorage<S> {
    inner: S,
    cache: Arc<Cache<String, Value>>,
}

impl<S: StorageProvider> StorageProvider for CachedStorage<S> {
    type Error = S::Error;
    
    async fn get(&self, key: StorageKey) -> Result<Option<Value>, Self::Error> {
        let cache_key = format!("{}", key);
        
        // Check cache first
        if let Some(cached) = self.cache.get(&cache_key).await {
            return Ok(Some(cached));
        }
        
        // Fallback to storage
        let result = self.inner.get(key).await?;
        
        // Cache the result
        if let Some(ref value) = result {
            self.cache.insert(cache_key, value.clone()).await;
        }
        
        Ok(result)
    }
}
```

### Resource Layer Optimizations

- **Metadata Caching**: Cache computed SCIM metadata
- **Validation Caching**: Cache validation results for schemas
- **Bulk Operations**: Implement batch processing for list operations

## Testing Strategies

### Unit Testing Storage

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use scim_server::storage::{StorageKey, StoragePrefix};
    use serde_json::json;
    
    #[tokio::test]
    async fn test_storage_crud() {
        let storage = MyStorage::new();
        let key = StorageKey::new("tenant1", "User", "123");
        let data = json!({"userName": "test"});
        
        // Test put
        let stored = storage.put(key.clone(), data.clone()).await.unwrap();
        assert_eq!(stored, data);
        
        // Test get
        let retrieved = storage.get(key.clone()).await.unwrap();
        assert_eq!(retrieved, Some(data));
        
        // Test delete
        let deleted = storage.delete(key.clone()).await.unwrap();
        assert!(deleted);
        
        // Verify deletion
        let after_delete = storage.get(key).await.unwrap();
        assert_eq!(after_delete, None);
    }
    
    #[tokio::test]
    async fn test_tenant_isolation() {
        let storage = MyStorage::new();
        
        let key1 = StorageKey::new("tenant1", "User", "123");
        let key2 = StorageKey::new("tenant2", "User", "123");
        
        let data1 = json!({"userName": "user1"});
        let data2 = json!({"userName": "user2"});
        
        storage.put(key1.clone(), data1.clone()).await.unwrap();
        storage.put(key2.clone(), data2.clone()).await.unwrap();
        
        // Verify isolation
        assert_eq!(storage.get(key1).await.unwrap(), Some(data1));
        assert_eq!(storage.get(key2).await.unwrap(), Some(data2));
    }
}
```

### Integration Testing

```rust
#[tokio::test]
async fn test_full_provider_stack() {
    let storage = InMemoryStorage::new();
    let provider = StandardResourceProvider::new(storage);
    
    let context = RequestContext::with_generated_id();
    
    // Test full SCIM workflow
    let user = provider.create_resource(
        "User",
        json!({
            "userName": "alice@example.com",
            "displayName": "Alice Smith"
        }),
        &context,
    ).await.unwrap();
    
    assert!(user.get_id().is_some());
    assert_eq!(user.get_username().unwrap(), "alice@example.com");
    
    // Test retrieval
    let retrieved = provider.get_resource(
        "User",
        user.get_id().unwrap(),
        &context,
    ).await.unwrap();
    
    assert!(retrieved.is_some());
    assert_eq!(retrieved.unwrap().get_username().unwrap(), "alice@example.com");
}
```

## Best Practices

### Provider Selection

Choose the right provider pattern for your use case:

- **Standard + InMemory**: Development, testing, proof of concepts
- **Standard + Database**: Production deployments with persistence
- **Standard + Custom**: Specialized storage requirements (cloud, distributed)
- **Custom ResourceProvider**: Non-standard SCIM behavior or extensive customization

### Configuration Management

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub struct StorageConfig {
    pub storage_type: String,
    pub connection_url: Option<String>,
    pub max_connections: Option<u32>,
    pub enable_ssl: bool,
    pub cache_ttl_seconds: Option<u64>,
}

pub async fn create_storage_provider(config: &StorageConfig) -> Result<Box<dyn StorageProvider<Error = StorageError>>, ConfigError> {
    match config.storage_type.as_str() {
        "memory" => Ok(Box::new(InMemoryStorage::new())),
        "postgres" => {
            let pool = create_postgres_pool(&config.connection_url.as_ref().unwrap()).await?;
            Ok(Box::new(PostgresStorage::new(pool)))
        }
        "redis" => {
            let client = create_redis_client(&config.connection_url.as_ref().unwrap()).await?;
            Ok(Box::new(RedisStorage::new(client)))
        }
        _ => Err(ConfigError::UnsupportedStorageType(config.storage_type.clone())),
    }
}
```

### Monitoring and Observability

```rust
use tracing::{info, error, instrument};

impl<S: StorageProvider> StandardResourceProvider<S> {
    #[instrument(skip(self, data, context))]
    async fn create_resource(
        &self,
        resource_type: &str,
        data: Value,
        context: &RequestContext,
    ) -> Result<Resource, Self::Error> {
        info!(
            resource_type = resource_type,
            tenant_id = context.tenant_context.as_ref().map(|t| t.tenant_id.as_str()),
            "Creating resource"
        );
        
        let result = self.inner_create_resource(resource_type, data, context).await;
        
        match &result {
            Ok(resource) => {
                info!(
                    resource_type = resource_type,
                    resource_id = resource.get_id().unwrap_or("unknown"),
                    "Resource created successfully"
                );
            }
            Err(e) => {
                error!(
                    resource_type = resource_type,
                    error = %e,
                    "Failed to create resource"
                );
            }
        }
        
        result
    }
}
```

## Next Steps

- [Provider Architecture](../providers/architecture.md) - Deep dive into the two-layer architecture
- [Basic Implementation](../providers/basic.md) - Learn to implement storage providers
- [Advanced Features](../providers/advanced.md) - Explore advanced provider capabilities
- [Multi-Tenancy](./multi-tenancy.md) - Comprehensive guide to multi-tenant deployments