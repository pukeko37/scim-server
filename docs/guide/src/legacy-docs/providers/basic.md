# Basic Provider Implementation

This guide walks you through implementing storage providers for the SCIM Server library. The SCIM Server uses a two-layer architecture that separates storage concerns from SCIM protocol logic.

## Architecture Overview

The SCIM Server uses two main abstractions:

- **StorageProvider**: Low-level trait for pure data persistence (PUT/GET/DELETE operations on JSON)
- **ResourceProvider**: High-level trait for SCIM-aware operations (handles SCIM metadata, validation, etc.)

The library provides `StandardResourceProvider` which implements `ResourceProvider` using any `StorageProvider` backend.

## Using the Standard Provider

The simplest approach is to use `StandardResourceProvider` with the built-in `InMemoryStorage`:

```rust
use scim_server::{
    providers::StandardResourceProvider,
    storage::InMemoryStorage,
    resource::{RequestContext, ResourceProvider},
};
use serde_json::json;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create storage backend
    let storage = InMemoryStorage::new();
    
    // Create provider with storage
    let provider = StandardResourceProvider::new(storage);
    
    // Create a user
    let context = RequestContext::with_generated_id();
    let user = provider.create_resource(
        "User",
        json!({
            "userName": "john.doe",
            "displayName": "John Doe",
            "emails": [{
                "value": "john@example.com",
                "primary": true
            }]
        }),
        &context,
    ).await?;
    
    println!("Created user: {}", user.get_id().unwrap());
    Ok(())
}
```

## Multi-Tenant Operations

The same provider works for multi-tenant scenarios using `TenantContext`:

```rust
use scim_server::{
    providers::StandardResourceProvider,
    storage::InMemoryStorage,
    resource::{RequestContext, TenantContext, ResourceProvider},
};
use serde_json::json;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let storage = InMemoryStorage::new();
    let provider = StandardResourceProvider::new(storage);
    
    // Create tenant context
    let tenant_context = TenantContext::new(
        "tenant-1".to_string(),
        "client-1".to_string(),
    );
    let context = RequestContext::with_tenant_generated_id(tenant_context);
    
    // Create user in specific tenant
    let user = provider.create_resource(
        "User",
        json!({
            "userName": "alice@tenant1.com",
            "displayName": "Alice"
        }),
        &context,
    ).await?;
    
    // Users are automatically isolated by tenant
    println!("Created user in tenant-1: {}", user.get_id().unwrap());
    Ok(())
}
```

## Implementing Custom Storage

To implement custom storage, create a type that implements `StorageProvider`:

```rust
use scim_server::storage::{StorageProvider, StorageKey, StoragePrefix, StorageError};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Clone)]
pub struct CustomStorage {
    data: Arc<RwLock<HashMap<String, Value>>>,
}

impl CustomStorage {
    pub fn new() -> Self {
        Self {
            data: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    
    fn key_string(key: &StorageKey) -> String {
        format!("{}/{}/{}", key.tenant_id(), key.resource_type(), key.resource_id())
    }
}

impl StorageProvider for CustomStorage {
    type Error = StorageError;
    
    async fn put(&self, key: StorageKey, data: Value) -> Result<Value, Self::Error> {
        let key_str = Self::key_string(&key);
        let mut store = self.data.write().await;
        store.insert(key_str, data.clone());
        Ok(data)
    }
    
    async fn get(&self, key: StorageKey) -> Result<Option<Value>, Self::Error> {
        let key_str = Self::key_string(&key);
        let store = self.data.read().await;
        Ok(store.get(&key_str).cloned())
    }
    
    async fn delete(&self, key: StorageKey) -> Result<bool, Self::Error> {
        let key_str = Self::key_string(&key);
        let mut store = self.data.write().await;
        Ok(store.remove(&key_str).is_some())
    }
    
    async fn list(
        &self,
        prefix: StoragePrefix,
        offset: usize,
        limit: usize,
    ) -> Result<Vec<(StorageKey, Value)>, Self::Error> {
        let prefix_str = format!("{}/{}/", prefix.tenant_id(), prefix.resource_type());
        let store = self.data.read().await;
        
        let mut results: Vec<_> = store
            .iter()
            .filter(|(k, _)| k.starts_with(&prefix_str))
            .skip(offset)
            .take(limit)
            .map(|(k, v)| {
                let parts: Vec<&str> = k.split('/').collect();
                let key = StorageKey::new(&parts[0], &parts[1], &parts[2]);
                (key, v.clone())
            })
            .collect();
            
        // Sort for consistent ordering
        results.sort_by(|a, b| a.0.resource_id().cmp(b.0.resource_id()));
        Ok(results)
    }
    
    async fn find_by_attribute(
        &self,
        prefix: StoragePrefix,
        attribute: &str,
        value: &str,
    ) -> Result<Vec<(StorageKey, Value)>, Self::Error> {
        let prefix_str = format!("{}/{}/", prefix.tenant_id(), prefix.resource_type());
        let store = self.data.read().await;
        
        let results: Vec<_> = store
            .iter()
            .filter(|(k, v)| {
                k.starts_with(&prefix_str) && 
                self.matches_attribute(v, attribute, value)
            })
            .map(|(k, v)| {
                let parts: Vec<&str> = k.split('/').collect();
                let key = StorageKey::new(&parts[0], &parts[1], &parts[2]);
                (key, v.clone())
            })
            .collect();
            
        Ok(results)
    }
    
    async fn exists(&self, key: StorageKey) -> Result<bool, Self::Error> {
        let key_str = Self::key_string(&key);
        let store = self.data.read().await;
        Ok(store.contains_key(&key_str))
    }
    
    async fn count(&self, prefix: StoragePrefix) -> Result<usize, Self::Error> {
        let prefix_str = format!("{}/{}/", prefix.tenant_id(), prefix.resource_type());
        let store = self.data.read().await;
        let count = store.keys().filter(|k| k.starts_with(&prefix_str)).count();
        Ok(count)
    }
}

impl CustomStorage {
    fn matches_attribute(&self, data: &Value, attribute: &str, value: &str) -> bool {
        // Simple attribute matching - you can extend this for nested attributes
        if let Some(attr_value) = data.get(attribute) {
            if let Some(string_value) = attr_value.as_str() {
                return string_value == value;
            }
        }
        false
    }
}
```

## Using Your Custom Storage

Once you have a `StorageProvider` implementation, use it with `StandardResourceProvider`:

```rust
use scim_server::providers::StandardResourceProvider;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Use your custom storage
    let storage = CustomStorage::new();
    let provider = StandardResourceProvider::new(storage);
    
    // Now you can use the provider normally
    let context = RequestContext::with_generated_id();
    let user = provider.create_resource(
        "User",
        json!({"userName": "test@example.com"}),
        &context,
    ).await?;
    
    println!("User created with custom storage!");
    Ok(())
}
```

## Storage Provider Design Principles

When implementing `StorageProvider`, follow these principles:

### 1. Protocol Agnostic
Storage providers handle pure data operations and don't know about SCIM:
- Store/retrieve JSON values
- No SCIM validation or metadata generation
- No business logic

### 2. Tenant Isolation
All operations are scoped by tenant through `StorageKey`:
- Tenant information is built into every key
- No cross-tenant data access
- Natural tenant isolation

### 3. Simple Operations
Core operations are PUT/GET/DELETE:
- `put()` works for both create and update
- `get()` returns `Option<Value>`
- `delete()` returns boolean (existed or not)

### 4. Consistent Ordering
List operations should return consistent results:
- Sort by resource ID for predictable pagination
- Implement proper offset/limit handling

### 5. Attribute Search
`find_by_attribute()` enables SCIM filtering:
- Support exact string matching
- Handle nested attributes with dot notation
- Return all matching resources

## Database Storage Example

Here's an example using a database (with SQLx):

```rust
use sqlx::{PgPool, Row};
use scim_server::storage::{StorageProvider, StorageKey, StoragePrefix, StorageError};
use serde_json::Value;

#[derive(Clone)]
pub struct PostgresStorage {
    pool: PgPool,
}

impl PostgresStorage {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

impl StorageProvider for PostgresStorage {
    type Error = StorageError;
    
    async fn put(&self, key: StorageKey, data: Value) -> Result<Value, Self::Error> {
        sqlx::query!(
            r#"
            INSERT INTO scim_resources (tenant_id, resource_type, resource_id, data)
            VALUES ($1, $2, $3, $4)
            ON CONFLICT (tenant_id, resource_type, resource_id)
            DO UPDATE SET data = $4, updated_at = NOW()
            "#,
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
            "SELECT data FROM scim_resources WHERE tenant_id = $1 AND resource_type = $2 AND resource_id = $3",
            key.tenant_id(),
            key.resource_type(),
            key.resource_id()
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| StorageError::Internal(e.to_string()))?;
        
        Ok(row.map(|r| r.data))
    }
    
    async fn delete(&self, key: StorageKey) -> Result<bool, Self::Error> {
        let result = sqlx::query!(
            "DELETE FROM scim_resources WHERE tenant_id = $1 AND resource_type = $2 AND resource_id = $3",
            key.tenant_id(),
            key.resource_type(),
            key.resource_id()
        )
        .execute(&self.pool)
        .await
        .map_err(|e| StorageError::Internal(e.to_string()))?;
        
        Ok(result.rows_affected() > 0)
    }
    
    // ... implement other methods
}
```

## Error Handling

Storage providers should use `StorageError` for consistent error handling:

```rust
use scim_server::storage::StorageError;

// For not found errors
return Err(StorageError::NotFound("Resource not found".to_string()));

// For constraint violations
return Err(StorageError::Conflict("Duplicate key".to_string()));

// For internal errors
return Err(StorageError::Internal(database_error.to_string()));
```

## Testing Your Storage Provider

Test your storage provider with the built-in test utilities:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use scim_server::storage::{StorageKey, StoragePrefix};
    use serde_json::json;
    
    #[tokio::test]
    async fn test_basic_operations() {
        let storage = CustomStorage::new();
        let key = StorageKey::new("tenant1", "User", "123");
        let data = json!({"userName": "test"});
        
        // Test put
        let stored = storage.put(key.clone(), data.clone()).await.unwrap();
        assert_eq!(stored, data);
        
        // Test get
        let retrieved = storage.get(key.clone()).await.unwrap();
        assert_eq!(retrieved, Some(data));
        
        // Test exists
        let exists = storage.exists(key.clone()).await.unwrap();
        assert!(exists);
        
        // Test delete
        let deleted = storage.delete(key.clone()).await.unwrap();
        assert!(deleted);
        
        // Verify deletion
        let after_delete = storage.get(key).await.unwrap();
        assert_eq!(after_delete, None);
    }
    
    #[tokio::test]
    async fn test_tenant_isolation() {
        let storage = CustomStorage::new();
        
        let tenant1_key = StorageKey::new("tenant1", "User", "123");
        let tenant2_key = StorageKey::new("tenant2", "User", "123");
        
        let data1 = json!({"userName": "user1"});
        let data2 = json!({"userName": "user2"});
        
        storage.put(tenant1_key.clone(), data1.clone()).await.unwrap();
        storage.put(tenant2_key.clone(), data2.clone()).await.unwrap();
        
        // Verify isolation
        let retrieved1 = storage.get(tenant1_key).await.unwrap();
        let retrieved2 = storage.get(tenant2_key).await.unwrap();
        
        assert_eq!(retrieved1, Some(data1));
        assert_eq!(retrieved2, Some(data2));
    }
}
```

## Next Steps

- [Advanced Provider Features](./advanced.md) - Learn about conditional operations and versioning
- [Provider Testing](./testing.md) - Comprehensive testing strategies
- [Architecture Overview](./architecture.md) - Deep dive into the provider architecture