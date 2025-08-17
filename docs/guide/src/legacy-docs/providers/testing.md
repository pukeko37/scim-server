# Provider Testing

This guide covers comprehensive testing strategies for storage and resource providers in the SCIM Server library. Testing ensures your providers work correctly, handle edge cases gracefully, and perform well under load.

## Testing Architecture

The SCIM Server's two-layer architecture requires testing at both levels:

- **Storage Provider Tests**: Test data persistence, retrieval, and tenant isolation
- **Resource Provider Tests**: Test SCIM protocol logic, validation, and metadata handling
- **Integration Tests**: Test the complete stack working together

## Storage Provider Testing

### Basic CRUD Operations

Test fundamental storage operations:

```rust
#[cfg(test)]
mod storage_tests {
    use super::*;
    use scim_server::storage::{StorageProvider, StorageKey, StoragePrefix, StorageError};
    use serde_json::json;
    use tokio_test;

    async fn test_storage_crud<S: StorageProvider>(storage: S) 
    where 
        S::Error: std::fmt::Debug,
    {
        let key = StorageKey::new("test-tenant", "User", "user-123");
        let data = json!({
            "userName": "alice@example.com",
            "displayName": "Alice Smith"
        });

        // Test put operation
        let stored = storage.put(key.clone(), data.clone()).await.unwrap();
        assert_eq!(stored, data);

        // Test get operation
        let retrieved = storage.get(key.clone()).await.unwrap();
        assert_eq!(retrieved, Some(data.clone()));

        // Test exists operation
        let exists = storage.exists(key.clone()).await.unwrap();
        assert!(exists);

        // Test delete operation
        let deleted = storage.delete(key.clone()).await.unwrap();
        assert!(deleted);

        // Verify deletion
        let after_delete = storage.get(key.clone()).await.unwrap();
        assert_eq!(after_delete, None);

        // Test delete non-existent
        let not_deleted = storage.delete(key).await.unwrap();
        assert!(!not_deleted);
    }

    #[tokio::test]
    async fn test_inmemory_storage_crud() {
        let storage = InMemoryStorage::new();
        test_storage_crud(storage).await;
    }

    #[tokio::test]
    async fn test_custom_storage_crud() {
        let storage = MyCustomStorage::new();
        test_storage_crud(storage).await;
    }
}
```

### Tenant Isolation Testing

Verify that tenant data is properly isolated:

```rust
#[tokio::test]
async fn test_tenant_isolation() {
    let storage = InMemoryStorage::new();
    
    // Create resources in different tenants
    let tenant1_key = StorageKey::new("tenant-1", "User", "user-123");
    let tenant2_key = StorageKey::new("tenant-2", "User", "user-123");
    
    let tenant1_data = json!({"userName": "alice@tenant1.com"});
    let tenant2_data = json!({"userName": "alice@tenant2.com"});
    
    storage.put(tenant1_key.clone(), tenant1_data.clone()).await.unwrap();
    storage.put(tenant2_key.clone(), tenant2_data.clone()).await.unwrap();
    
    // Verify isolation - same resource ID but different tenants
    let retrieved1 = storage.get(tenant1_key).await.unwrap();
    let retrieved2 = storage.get(tenant2_key).await.unwrap();
    
    assert_eq!(retrieved1, Some(tenant1_data));
    assert_eq!(retrieved2, Some(tenant2_data));
    
    // Verify list operations are also isolated
    let prefix1 = StorageKey::prefix("tenant-1", "User");
    let prefix2 = StorageKey::prefix("tenant-2", "User");
    
    let list1 = storage.list(prefix1, 0, 100).await.unwrap();
    let list2 = storage.list(prefix2, 0, 100).await.unwrap();
    
    assert_eq!(list1.len(), 1);
    assert_eq!(list2.len(), 1);
    assert_ne!(list1[0].1, list2[0].1); // Different data
}
```

### Query Operations Testing

Test list, search, and pagination:

```rust
#[tokio::test]
async fn test_list_operations() {
    let storage = InMemoryStorage::new();
    let prefix = StorageKey::prefix("tenant-1", "User");
    
    // Create multiple resources
    for i in 1..=10 {
        let key = StorageKey::new("tenant-1", "User", &format!("user-{:03}", i));
        let data = json!({
            "userName": format!("user{}@example.com", i),
            "displayName": format!("User {}", i)
        });
        storage.put(key, data).await.unwrap();
    }
    
    // Test list all
    let all_users = storage.list(prefix.clone(), 0, 100).await.unwrap();
    assert_eq!(all_users.len(), 10);
    
    // Test pagination
    let page1 = storage.list(prefix.clone(), 0, 3).await.unwrap();
    let page2 = storage.list(prefix.clone(), 3, 3).await.unwrap();
    let page3 = storage.list(prefix.clone(), 6, 3).await.unwrap();
    let page4 = storage.list(prefix.clone(), 9, 3).await.unwrap();
    
    assert_eq!(page1.len(), 3);
    assert_eq!(page2.len(), 3);
    assert_eq!(page3.len(), 3);
    assert_eq!(page4.len(), 1);
    
    // Verify no overlap between pages
    let all_ids: HashSet<_> = page1.iter().chain(&page2).chain(&page3).chain(&page4)
        .map(|(key, _)| key.resource_id())
        .collect();
    assert_eq!(all_ids.len(), 10);
}

#[tokio::test]
async fn test_find_by_attribute() {
    let storage = InMemoryStorage::new();
    let prefix = StorageKey::prefix("tenant-1", "User");
    
    // Create test users
    let users = vec![
        ("user-1", "alice@example.com", "Alice Smith"),
        ("user-2", "bob@example.com", "Bob Jones"),
        ("user-3", "alice@company.com", "Alice Johnson"),
    ];
    
    for (id, username, display_name) in users {
        let key = StorageKey::new("tenant-1", "User", id);
        let data = json!({
            "userName": username,
            "displayName": display_name
        });
        storage.put(key, data).await.unwrap();
    }
    
    // Test exact match
    let alice_users = storage.find_by_attribute(
        prefix.clone(),
        "userName",
        "alice@example.com"
    ).await.unwrap();
    assert_eq!(alice_users.len(), 1);
    assert_eq!(alice_users[0].0.resource_id(), "user-1");
    
    // Test no matches
    let no_matches = storage.find_by_attribute(
        prefix.clone(),
        "userName",
        "nonexistent@example.com"
    ).await.unwrap();
    assert_eq!(no_matches.len(), 0);
}
```

### Concurrent Access Testing

Test thread safety and concurrent operations:

```rust
use std::sync::Arc;
use tokio::task::JoinSet;

#[tokio::test]
async fn test_concurrent_access() {
    let storage = Arc::new(InMemoryStorage::new());
    let mut join_set = JoinSet::new();
    
    // Spawn multiple concurrent operations
    for i in 0..10 {
        let storage_clone = Arc::clone(&storage);
        join_set.spawn(async move {
            let key = StorageKey::new("tenant-1", "User", &format!("user-{}", i));
            let data = json!({
                "userName": format!("user{}@example.com", i),
                "displayName": format!("User {}", i)
            });
            
            // Perform multiple operations
            storage_clone.put(key.clone(), data.clone()).await.unwrap();
            let retrieved = storage_clone.get(key.clone()).await.unwrap();
            assert_eq!(retrieved, Some(data));
            
            storage_clone.delete(key).await.unwrap()
        });
    }
    
    // Wait for all operations to complete
    while let Some(result) = join_set.join_next().await {
        result.unwrap(); // Panic if any task failed
    }
    
    // Verify final state
    let prefix = StorageKey::prefix("tenant-1", "User");
    let remaining = storage.list(prefix, 0, 100).await.unwrap();
    assert_eq!(remaining.len(), 0);
}
```

### Error Handling Testing

Test error conditions and edge cases:

```rust
#[tokio::test]
async fn test_error_handling() {
    let storage = InMemoryStorage::new();
    
    // Test get non-existent resource
    let key = StorageKey::new("tenant-1", "User", "non-existent");
    let result = storage.get(key.clone()).await.unwrap();
    assert_eq!(result, None);
    
    // Test delete non-existent resource
    let deleted = storage.delete(key).await.unwrap();
    assert!(!deleted);
    
    // Test invalid operations (implementation specific)
    // For example, if your storage has size limits:
    // let large_data = json!({"data": "x".repeat(1_000_000)});
    // let result = storage.put(key, large_data).await;
    // assert!(result.is_err());
}
```

## Resource Provider Testing

### SCIM Protocol Testing

Test SCIM-specific functionality:

```rust
#[cfg(test)]
mod resource_provider_tests {
    use super::*;
    use scim_server::{
        providers::StandardResourceProvider,
        storage::InMemoryStorage,
        resource::{RequestContext, TenantContext, ResourceProvider},
    };
    use serde_json::json;

    #[tokio::test]
    async fn test_resource_metadata_generation() {
        let storage = InMemoryStorage::new();
        let provider = StandardResourceProvider::new(storage);
        let context = RequestContext::with_generated_id();
        
        let user_data = json!({
            "userName": "alice@example.com",
            "displayName": "Alice Smith"
        });
        
        let user = provider.create_resource("User", user_data, &context).await.unwrap();
        
        // Verify SCIM metadata is generated
        assert!(user.get_id().is_some());
        assert!(user.get_created().is_some());
        assert!(user.get_last_modified().is_some());
        assert!(user.get_version().is_some());
        assert_eq!(user.get_username().unwrap(), "alice@example.com");
    }

    #[tokio::test]
    async fn test_tenant_context_handling() {
        let storage = InMemoryStorage::new();
        let provider = StandardResourceProvider::new(storage);
        
        // Single-tenant context
        let single_context = RequestContext::with_generated_id();
        
        // Multi-tenant context
        let tenant_context = TenantContext::new("tenant-1".to_string(), "client-1".to_string());
        let multi_context = RequestContext::with_tenant_generated_id(tenant_context);
        
        let user_data = json!({
            "userName": "alice@example.com",
            "displayName": "Alice Smith"
        });
        
        // Create users in different contexts
        let single_user = provider.create_resource("User", user_data.clone(), &single_context).await.unwrap();
        let multi_user = provider.create_resource("User", user_data, &multi_context).await.unwrap();
        
        // Verify they are isolated
        let single_list = provider.list_resources("User", None, &single_context).await.unwrap();
        let multi_list = provider.list_resources("User", None, &multi_context).await.unwrap();
        
        assert_eq!(single_list.len(), 1);
        assert_eq!(multi_list.len(), 1);
        assert_ne!(single_user.get_id(), multi_user.get_id());
    }

    #[tokio::test]
    async fn test_duplicate_username_handling() {
        let storage = InMemoryStorage::new();
        let provider = StandardResourceProvider::new(storage);
        let context = RequestContext::with_generated_id();
        
        let user_data = json!({
            "userName": "alice@example.com",
            "displayName": "Alice Smith"
        });
        
        // Create first user
        let user1 = provider.create_resource("User", user_data.clone(), &context).await.unwrap();
        
        // Try to create duplicate username
        let result = provider.create_resource("User", user_data, &context).await;
        
        // Should fail with conflict
        assert!(result.is_err());
    }
}
```

### Conditional Operations Testing

Test ETag-based concurrency control:

```rust
#[tokio::test]
async fn test_conditional_operations() {
    let storage = InMemoryStorage::new();
    let provider = StandardResourceProvider::new(storage);
    let context = RequestContext::with_generated_id();
    
    // Create a user
    let user_data = json!({
        "userName": "alice@example.com",
        "displayName": "Alice Smith"
    });
    
    let user = provider.create_resource("User", user_data, &context).await.unwrap();
    let user_id = user.get_id().unwrap();
    let version = user.get_version().unwrap();
    
    // Test successful conditional update
    let updated_data = json!({
        "id": user_id,
        "userName": "alice@example.com",
        "displayName": "Alice Updated"
    });
    
    let result = provider.conditional_update(
        "User",
        user_id,
        updated_data,
        &ScimVersion::from_etag(version),
        &context,
    ).await.unwrap();
    
    match result {
        ConditionalResult::Success(updated_user) => {
            assert_eq!(updated_user.get_display_name().unwrap(), "Alice Updated");
            assert_ne!(updated_user.get_version().unwrap(), version); // Version should change
        }
        ConditionalResult::Conflict(_) => panic!("Should not have conflict"),
    }
    
    // Test conditional update with wrong version (should conflict)
    let wrong_version_data = json!({
        "id": user_id,
        "userName": "alice@example.com",
        "displayName": "Alice Wrong Version"
    });
    
    let conflict_result = provider.conditional_update(
        "User",
        user_id,
        wrong_version_data,
        &ScimVersion::from_etag(version), // Old version
        &context,
    ).await.unwrap();
    
    match conflict_result {
        ConditionalResult::Success(_) => panic!("Should have conflict"),
        ConditionalResult::Conflict(conflict) => {
            assert_eq!(conflict.expected_version, version);
            assert_ne!(conflict.current_version, version);
        }
    }
}
```

## Integration Testing

### Full Stack Testing

Test the complete SCIM workflow:

```rust
#[tokio::test]
async fn test_full_scim_workflow() {
    let storage = InMemoryStorage::new();
    let provider = StandardResourceProvider::new(storage);
    let context = RequestContext::with_generated_id();
    
    // Create user
    let create_data = json!({
        "userName": "alice@example.com",
        "name": {
            "givenName": "Alice",
            "familyName": "Smith"
        },
        "emails": [{
            "value": "alice@example.com",
            "primary": true
        }]
    });
    
    let user = provider.create_resource("User", create_data, &context).await.unwrap();
    let user_id = user.get_id().unwrap();
    
    // Read user
    let retrieved = provider.get_resource("User", user_id, &context).await.unwrap();
    assert!(retrieved.is_some());
    let retrieved_user = retrieved.unwrap();
    assert_eq!(retrieved_user.get_username().unwrap(), "alice@example.com");
    
    // Update user
    let update_data = json!({
        "id": user_id,
        "userName": "alice@example.com",
        "name": {
            "givenName": "Alice",
            "familyName": "Johnson"
        },
        "emails": [{
            "value": "alice@example.com",
            "primary": true
        }]
    });
    
    let updated_user = provider.update_resource("User", user_id, update_data, &context).await.unwrap();
    assert_eq!(updated_user.get_family_name().unwrap(), "Johnson");
    
    // List users
    let users = provider.list_resources("User", None, &context).await.unwrap();
    assert_eq!(users.len(), 1);
    
    // Search user
    let found = provider.find_resource_by_attribute(
        "User",
        "userName",
        &json!("alice@example.com"),
        &context,
    ).await.unwrap();
    assert!(found.is_some());
    
    // Delete user
    let deleted = provider.delete_resource("User", user_id, &context).await.unwrap();
    assert!(deleted);
    
    // Verify deletion
    let after_delete = provider.get_resource("User", user_id, &context).await.unwrap();
    assert!(after_delete.is_none());
}
```

### Multi-Tenant Integration Testing

```rust
#[tokio::test]
async fn test_multi_tenant_integration() {
    let storage = InMemoryStorage::new();
    let provider = StandardResourceProvider::new(storage);
    
    // Create contexts for different tenants
    let tenant1_context = RequestContext::with_tenant_generated_id(
        TenantContext::new("tenant-1".to_string(), "client-1".to_string())
    );
    let tenant2_context = RequestContext::with_tenant_generated_id(
        TenantContext::new("tenant-2".to_string(), "client-2".to_string())
    );
    
    // Create users in different tenants
    let user_data = json!({
        "userName": "alice@example.com",
        "displayName": "Alice Smith"
    });
    
    let tenant1_user = provider.create_resource("User", user_data.clone(), &tenant1_context).await.unwrap();
    let tenant2_user = provider.create_resource("User", user_data, &tenant2_context).await.unwrap();
    
    // Verify isolation
    let tenant1_users = provider.list_resources("User", None, &tenant1_context).await.unwrap();
    let tenant2_users = provider.list_resources("User", None, &tenant2_context).await.unwrap();
    
    assert_eq!(tenant1_users.len(), 1);
    assert_eq!(tenant2_users.len(), 1);
    assert_ne!(tenant1_user.get_id(), tenant2_user.get_id());
    
    // Verify cross-tenant access fails
    let cross_access = provider.get_resource(
        "User",
        tenant1_user.get_id().unwrap(),
        &tenant2_context,
    ).await.unwrap();
    assert!(cross_access.is_none()); // Should not find user from different tenant
}
```

## Performance Testing

### Load Testing

Test provider performance under load:

```rust
use std::time::Instant;
use tokio::task::JoinSet;

#[tokio::test]
async fn test_provider_performance() {
    let storage = InMemoryStorage::new();
    let provider = Arc::new(StandardResourceProvider::new(storage));
    
    let num_operations = 1000;
    let num_concurrent = 10;
    
    let start = Instant::now();
    let mut join_set = JoinSet::new();
    
    for batch in 0..num_concurrent {
        let provider_clone = Arc::clone(&provider);
        join_set.spawn(async move {
            let context = RequestContext::with_generated_id();
            
            for i in 0..(num_operations / num_concurrent) {
                let user_id = format!("user-{}-{}", batch, i);
                let user_data = json!({
                    "userName": format!("{}@example.com", user_id),
                    "displayName": format!("User {}", user_id)
                });
                
                // Create, read, update, delete cycle
                let user = provider_clone.create_resource("User", user_data, &context).await.unwrap();
                let id = user.get_id().unwrap();
                
                let retrieved = provider_clone.get_resource("User", id, &context).await.unwrap();
                assert!(retrieved.is_some());
                
                let updated_data = json!({
                    "id": id,
                    "userName": format!("{}@example.com", user_id),
                    "displayName": format!("Updated User {}", user_id)
                });
                
                let updated = provider_clone.update_resource("User", id, updated_data, &context).await.unwrap();
                assert_eq!(updated.get_display_name().unwrap(), format!("Updated User {}", user_id));
                
                let deleted = provider_clone.delete_resource("User", id, &context).await.unwrap();
                assert!(deleted);
            }
        });
    }
    
    // Wait for all operations to complete
    while let Some(result) = join_set.join_next().await {
        result.unwrap();
    }
    
    let duration = start.elapsed();
    let ops_per_second = (num_operations * 4) as f64 / duration.as_secs_f64(); // 4 ops per iteration
    
    println!("Completed {} operations in {:?}", num_operations * 4, duration);
    println!("Performance: {:.2} operations/second", ops_per_second);
    
    // Assert minimum performance threshold (adjust based on requirements)
    assert!(ops_per_second > 100.0, "Performance below threshold: {:.2} ops/sec", ops_per_second);
}
```

### Memory Usage Testing

Test memory efficiency and leak detection:

```rust
#[tokio::test]
async fn test_memory_usage() {
    let storage = InMemoryStorage::new();
    let provider = StandardResourceProvider::new(storage.clone());
    let context = RequestContext::with_generated_id();
    
    // Create many resources
    let num_resources = 10000;
    for i in 0..num_resources {
        let user_data = json!({
            "userName": format!("user{}@example.com", i),
            "displayName": format!("User {}", i)
        });
        
        provider.create_resource("User", user_data, &context).await.unwrap();
    }
    
    // Check storage stats
    let stats = storage.stats().await;
    assert_eq!(stats.total_resources, num_resources);
    assert_eq!(stats.tenant_count, 1); // All in default tenant
    
    // Delete all resources
    let users = provider.list_resources("User", None, &context).await.unwrap();
    for user in users {
        provider.delete_resource("User", user.get_id().unwrap(), &context).await.unwrap();
    }
    
    // Verify cleanup
    let final_stats = storage.stats().await;
    assert_eq!(final_stats.total_resources, 0);
}
```

## Test Utilities and Helpers

### Reusable Test Fixtures

Create common test utilities:

```rust
pub mod test_utils {
    use super::*;
    use serde_json::{json, Value};
    use uuid::Uuid;

    pub fn random_tenant_id() -> String {
        format!("test-tenant-{}", Uuid::new_v4())
    }

    pub fn sample_user_data(username: &str) -> Value {
        json!({
            "userName": username,
            "name": {
                "givenName": "Test",
                "familyName": "User"
            },
            "displayName": format!("Test User {}", username),
            "emails": [{
                "value": username,
                "primary": true
            }]
        })
    }

    pub fn sample_group_data(name: &str) -> Value {
        json!({
            "displayName": name,
            "members": []
        })
    }

    pub async fn create_test_provider() -> StandardResourceProvider<InMemoryStorage> {
        let storage = InMemoryStorage::new();
        StandardResourceProvider::new(storage)
    }

    pub async fn setup_test_data(
        provider: &StandardResourceProvider<InMemoryStorage>,
        context: &RequestContext,
    ) -> Vec<String> {
        let mut user_ids = Vec::new();
        
        for i in 1..=5 {
            let username = format!("testuser{}@example.com", i);
            let user_data = sample_user_data(&username);
            
            let user = provider.create_resource("User", user_data, context).await.unwrap();
            user_ids.push(user.get_id().unwrap().to_string());
        }
        
        user_ids
    }
}

// Usage in tests
#[tokio::test]
async fn test_with_utilities() {
    let provider = test_utils::create_test_provider().await;
    let context = RequestContext::with_generated_id();
    
    let user_ids = test_utils::setup_test_data(&provider, &context).await;
    assert_eq!(user_ids.len(), 5);
    
    // Your test logic here...
}
```

### Property-Based Testing

Use property-based testing for edge cases:

```rust
use proptest::prelude::*;

proptest! {
    #[test]
    fn test_resource_id_roundtrip(
        tenant_id in "[a-zA-Z0-9-]{1,50}",
        resource_type in "[a-zA-Z]{1,20}",
        resource_id in "[a-zA-Z0-9-]{1,50}"
    ) {
        tokio_test::block_on(async {
            let storage = InMemoryStorage::new();
            let key = StorageKey::new(&tenant_id, &resource_type, &resource_id);
            let data = json!({"test": "data"});
            
            let stored = storage.put(key.clone(), data.clone()).await.unwrap();
            let retrieved = storage.get(key).await.unwrap();
            
            prop_assert_eq!(retrieved, Some(stored));
        });
    }
}
```

## Continuous Integration Testing

### GitHub Actions Example

```yaml
name: Provider Tests

on:
  push:
    branches: [ main ]
  pull_request:
    branches: [ main ]

jobs:
  test:
    runs-on: ubuntu-latest
    strategy:
      matrix:
        rust: [stable, beta, nightly]
        
    steps:
    - uses: actions/checkout@v3
    
    - name: Install Rust
      uses: actions-rs/toolchain@v1
      with:
        toolchain: ${{ matrix.rust }}
        override: true
        components: rustfmt, clippy
        
    - name: Cache dependencies
      uses: actions/cache@v3
      with:
        path: |
          ~/.cargo/registry
          ~/.cargo/git
          target
        key: ${{ runner.os }}-cargo-${{ hashFiles('**/Cargo.lock') }}
        
    - name: Run tests
      run: cargo test --all-features
      
    - name: Run storage provider tests
      run: cargo test storage::tests --all-features
      
    - name: Run resource provider tests  
      run: cargo test providers::tests --all-features
      
    - name: Run integration tests
      run: cargo test integration --all-features
      
    - name: Check formatting
      run: cargo fmt -- --check
      
    - name: Run clippy
      run: cargo clippy -- -D warnings
```

## Best Practices

### Test Organization

1. **Separate Concerns**: Test storage and resource providers separately
2. **Use Descriptive Names**: Test names should clearly indicate what is being tested
3. **Test Edge Cases**: Include tests for error conditions and boundary cases
4. **Performance Regression**: Include performance tests in CI
5. **Documentation**: Document complex test scenarios

### Test Data Management

1. **Isolated Tests**: Each test should create its own data
2. **Cleanup**: Tests should clean up after themselves
3. **Deterministic**: Tests should produce consistent results
4. **Realistic Data**: Use realistic test data that matches production patterns

### Error Testing

1. **Expected Errors**: Test that errors are properly handled and returned
2. **Recovery**: Test that providers can recover from transient errors
3. **Resource Cleanup**: Ensure resources are properly cleaned up on errors

This comprehensive testing approach ensures your providers are reliable, performant, and ready for production use.