# Migration Guide: InMemoryProvider to StandardResourceProvider

This guide helps you migrate from the deprecated `InMemoryProvider` to the new `StandardResourceProvider` introduced in v0.3.0.

## Overview

In v0.3.0, we introduced the `StandardResourceProvider<S>` which provides the same functionality as `InMemoryProvider` but with pluggable storage backends. The `InMemoryProvider` is now deprecated and will be removed in v0.4.0.

## Quick Migration

### Before (v0.2.x)
```rust
use scim_server::providers::InMemoryProvider;

let provider = InMemoryProvider::new();
```

### After (v0.3.0+)
```rust
use scim_server::{
    providers::StandardResourceProvider,
    storage::InMemoryStorage,
};

let storage = InMemoryStorage::new();
let provider = StandardResourceProvider::new(storage);
```

## Detailed Migration Steps

### 1. Update Dependencies

Ensure you're using scim-server v0.3.0 or later:

```toml
[dependencies]
scim-server = "0.3.0"
```

### 2. Update Imports

**Old imports:**
```rust
use scim_server::providers::InMemoryProvider;
```

**New imports:**
```rust
use scim_server::{
    providers::StandardResourceProvider,
    storage::InMemoryStorage,
};
```

### 3. Update Provider Creation

**Old code:**
```rust
let provider = InMemoryProvider::new();
```

**New code:**
```rust
let storage = InMemoryStorage::new();
let provider = StandardResourceProvider::new(storage);
```

### 4. Update Type Annotations

If you have explicit type annotations, update them:

**Old:**
```rust
let provider: InMemoryProvider = InMemoryProvider::new();
```

**New:**
```rust
let provider: StandardResourceProvider<InMemoryStorage> = 
    StandardResourceProvider::new(InMemoryStorage::new());

// Or use type inference:
let provider = StandardResourceProvider::new(InMemoryStorage::new());
```

### 5. Update Function Parameters

**Old:**
```rust
fn setup_server(provider: InMemoryProvider) -> ScimServer<InMemoryProvider> {
    ScimServer::new(provider)
}
```

**New:**
```rust
fn setup_server(
    provider: StandardResourceProvider<InMemoryStorage>
) -> ScimServer<StandardResourceProvider<InMemoryStorage>> {
    ScimServer::new(provider)
}

// Or use generic parameters:
fn setup_server<S: StorageProvider>(
    provider: StandardResourceProvider<S>
) -> ScimServer<StandardResourceProvider<S>> {
    ScimServer::new(provider)
}
```

### 6. Update Error Handling

Error types remain the same (`InMemoryError`), so error handling code doesn't need to change:

```rust
// This works with both providers
match provider.create_resource("User", user_data, &context).await {
    Ok(user) => println!("Created user: {}", user.get_id().unwrap()),
    Err(InMemoryError::DuplicateAttribute { attribute, value, .. }) => {
        println!("Duplicate {}: {}", attribute, value);
    }
    Err(e) => println!("Error: {}", e),
}
```

## API Compatibility

The `StandardResourceProvider` implements the exact same `ResourceProvider` trait as `InMemoryProvider`, so all existing code using the provider interface will work without changes:

```rust
// All these methods work identically
provider.create_resource("User", data, &context).await?;
provider.get_resource("User", "123", &context).await?;
provider.update_resource("User", "123", data, &context).await?;
provider.delete_resource("User", "123", &context).await?;
provider.list_resources("User", None, &context).await?;
provider.find_resource_by_attribute("User", "userName", &value, &context).await?;
provider.resource_exists("User", "123", &context).await?;
```

## Additional Features

The `StandardResourceProvider` includes additional features not available in `InMemoryProvider`:

### Statistics
```rust
let stats = provider.get_stats().await;
println!("Total resources: {}", stats.total_resources);
println!("Tenant count: {}", stats.tenant_count);
```

### Clear Functionality (for testing)
```rust
provider.clear().await;
```

### Conditional Operations
```rust
use scim_server::resource::version::ScimVersion;

let version = ScimVersion::from_hash("abc123");
let result = provider.conditional_update(
    "User",
    "123",
    updated_data,
    &version,
    &context
).await?;
```

## Future Storage Backends

The pluggable storage design allows for future storage implementations:

```rust
// Future PostgreSQL storage (example)
let storage = PostgresStorage::new("postgresql://localhost/scim").await?;
let provider = StandardResourceProvider::new(storage);

// Future Redis storage (example)
let storage = RedisStorage::new("redis://localhost:6379").await?;
let provider = StandardResourceProvider::new(storage);
```

## Testing Migration

Here's a simple test to verify your migration:

```rust
#[tokio::test]
async fn test_migration() {
    use scim_server::{
        providers::StandardResourceProvider,
        storage::InMemoryStorage,
        RequestContext,
    };
    use serde_json::json;

    let storage = InMemoryStorage::new();
    let provider = StandardResourceProvider::new(storage);
    let context = RequestContext::new("test".to_string());

    // Test basic operations
    let user = provider.create_resource(
        "User",
        json!({
            "userName": "test@example.com",
            "active": true
        }),
        &context
    ).await.unwrap();

    assert!(user.get_id().is_some());
    assert_eq!(user.get_username().unwrap(), "test@example.com");

    // Test retrieval
    let retrieved = provider.get_resource(
        "User",
        user.get_id().unwrap(),
        &context
    ).await.unwrap();

    assert!(retrieved.is_some());
}
```

## Common Migration Issues

### Issue: Clone Errors
**Problem:** `StandardResourceProvider` doesn't implement `Clone` by default.

**Solution:** Create a new instance or use `Arc<StandardResourceProvider<S>>`:

```rust
use std::sync::Arc;

let storage = InMemoryStorage::new();
let provider = Arc::new(StandardResourceProvider::new(storage));

// Now you can clone the Arc
let provider_clone = provider.clone();
```

### Issue: Type Inference
**Problem:** Rust can't infer the storage type in generic contexts.

**Solution:** Use explicit type annotations:

```rust
fn create_provider() -> StandardResourceProvider<InMemoryStorage> {
    StandardResourceProvider::new(InMemoryStorage::new())
}
```

### Issue: Missing Storage Import
**Problem:** Forgot to import `InMemoryStorage`.

**Solution:** Add the storage import:

```rust
use scim_server::storage::InMemoryStorage;
```

## Complete Example Migration

### Before (v0.2.x)
```rust
use scim_server::{
    providers::InMemoryProvider,
    RequestContext, ScimServer,
};
use serde_json::json;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let provider = InMemoryProvider::new();
    let server = ScimServer::new(provider)?;
    
    let context = RequestContext::new("example".to_string());
    
    let user = provider.create_resource(
        "User",
        json!({"userName": "test@example.com"}),
        &context
    ).await?;
    
    println!("Created user: {}", user.get_id().unwrap());
    Ok(())
}
```

### After (v0.3.0+)
```rust
use scim_server::{
    providers::StandardResourceProvider,
    storage::InMemoryStorage,
    RequestContext, ScimServer,
};
use serde_json::json;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let storage = InMemoryStorage::new();
    let provider = StandardResourceProvider::new(storage);
    let server = ScimServer::new(provider)?;
    
    let context = RequestContext::new("example".to_string());
    
    let user = provider.create_resource(
        "User",
        json!({"userName": "test@example.com"}),
        &context
    ).await?;
    
    println!("Created user: {}", user.get_id().unwrap());
    Ok(())
}
```

## Timeline

- **v0.3.0**: `StandardResourceProvider` introduced, `InMemoryProvider` deprecated
- **v0.4.0**: `InMemoryProvider` will be removed (planned)

Start your migration now to ensure compatibility with future versions.

## Getting Help

If you encounter issues during migration:

1. Check the [examples directory](../examples/) for updated examples
2. Review the [API documentation](https://docs.rs/scim-server/)
3. Open an issue on [GitHub](https://github.com/your-org/scim-server/issues)

## Migration Checklist

- [ ] Update to scim-server v0.3.0+
- [ ] Replace `InMemoryProvider` imports with `StandardResourceProvider` and `InMemoryStorage`
- [ ] Update provider creation code
- [ ] Update type annotations if needed
- [ ] Test all existing functionality
- [ ] Consider using new features (stats, conditional operations)
- [ ] Update documentation and comments
- [ ] Run full test suite