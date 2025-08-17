# Implementation Guide

This guide shows how to implement ETag-based concurrency control using the StandardResourceProvider and actual working examples from the SCIM Server library.

## Setting Up a Provider with Concurrency Support

All providers in the SCIM Server library automatically support conditional operations through the `ResourceProvider` trait:

```rust
use scim_server::{
    providers::StandardResourceProvider,
    storage::InMemoryStorage,
    resource::{RequestContext, ResourceProvider},
};

async fn setup_provider() -> StandardResourceProvider<InMemoryStorage> {
    let storage = InMemoryStorage::new();
    StandardResourceProvider::new(storage)
}
```

## Basic Conditional Operations

### Getting Versioned Resources

```rust
use scim_server::resource::{RequestContext, ResourceProvider};
use serde_json::json;

async fn get_with_version_example() -> Result<(), Box<dyn std::error::Error>> {
    let provider = setup_provider().await;
    let context = RequestContext::with_generated_id();

    // Create a user first
    let user_data = json!({
        "userName": "alice.smith",
        "displayName": "Alice Smith",
        "active": true
    });

    let user = provider.create_resource("User", user_data, &context).await?;
    let user_id = user.get_id().unwrap();

    // Get the resource with version information
    let versioned = provider
        .get_versioned_resource("User", user_id, &context)
        .await?;

    match versioned {
        Some(versioned_resource) => {
            let version = versioned_resource.version();
            let resource = versioned_resource.resource();
            
            println!("User ID: {}", resource.get_id().unwrap());
            println!("Version: {}", version.to_http_header());
            println!("Resource data: {}", resource.to_json()?);
        }
        None => {
            println!("User not found");
        }
    }

    Ok(())
}
```

### Conditional Updates

```rust
use scim_server::resource::version::ConditionalResult;

async fn conditional_update_example() -> Result<(), Box<dyn std::error::Error>> {
    let provider = setup_provider().await;
    let context = RequestContext::with_generated_id();

    // Create initial user
    let user_data = json!({
        "userName": "bob.jones",
        "displayName": "Bob Jones",
        "active": true
    });

    let user = provider.create_resource("User", user_data, &context).await?;
    let user_id = user.get_id().unwrap();

    // Get current version
    let versioned = provider
        .get_versioned_resource("User", user_id, &context)
        .await?
        .expect("User should exist");

    let current_version = versioned.version();

    // Prepare update
    let update_data = json!({
        "userName": "bob.jones",
        "displayName": "Bob Updated",
        "active": false
    });

    // Perform conditional update
    let result = provider.conditional_update(
        "User",
        user_id,
        update_data,
        current_version,
        &context
    ).await?;

    match result {
        ConditionalResult::Success(updated_resource) => {
            println!("✅ Update successful!");
            println!("New version: {}", updated_resource.version().to_http_header());
            
            // Verify the update
            let resource = updated_resource.resource();
            let display_name = resource.get_attribute("displayName")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown");
            println!("Updated display name: {}", display_name);
        }
        ConditionalResult::VersionMismatch(conflict) => {
            println!("❌ Version conflict detected!");
            println!("Expected: {}", conflict.expected.to_http_header());
            println!("Current: {}", conflict.current.to_http_header());
            println!("Message: {}", conflict.message);
        }
        ConditionalResult::NotFound => {
            println!("❌ User not found (may have been deleted)");
        }
    }

    Ok(())
}
```

### Conditional Deletes

```rust
async fn conditional_delete_example() -> Result<(), Box<dyn std::error::Error>> {
    let provider = setup_provider().await;
    let context = RequestContext::with_generated_id();

    // Create a user to delete
    let user_data = json!({
        "userName": "charlie.brown",
        "displayName": "Charlie Brown",
        "active": true
    });

    let user = provider.create_resource("User", user_data, &context).await?;
    let user_id = user.get_id().unwrap();

    // Get current version
    let versioned = provider
        .get_versioned_resource("User", user_id, &context)
        .await?
        .expect("User should exist");

    let current_version = versioned.version();

    // Perform conditional delete
    let result = provider.conditional_delete(
        "User",
        user_id,
        current_version,
        &context
    ).await?;

    match result {
        ConditionalResult::Success(()) => {
            println!("✅ Delete successful!");
            
            // Verify deletion
            let exists = provider.resource_exists("User", user_id, &context).await?;
            println!("Resource still exists: {}", exists);
        }
        ConditionalResult::VersionMismatch(conflict) => {
            println!("❌ Version conflict - resource was modified!");
            println!("Expected: {}", conflict.expected.to_http_header());
            println!("Current: {}", conflict.current.to_http_header());
        }
        ConditionalResult::NotFound => {
            println!("ℹ️ Resource was already deleted");
        }
    }

    Ok(())
}
```

## Working with HTTP Headers

### Parsing Client Headers

```rust
use scim_server::resource::version::ScimVersion;
use axum::http::HeaderMap;

fn extract_if_match_header(headers: &HeaderMap) -> Option<ScimVersion> {
    headers
        .get("If-Match")
        .and_then(|h| h.to_str().ok())
        .and_then(|s| ScimVersion::parse_http_header(s).ok())
}

fn extract_if_none_match_header(headers: &HeaderMap) -> Option<ScimVersion> {
    headers
        .get("If-None-Match")
        .and_then(|h| h.to_str().ok())
        .and_then(|s| ScimVersion::parse_http_header(s).ok())
}

// Example usage in a web handler
async fn handle_update_request(
    headers: HeaderMap,
    user_id: String,
    update_data: serde_json::Value,
) -> Result<(), Box<dyn std::error::Error>> {
    let provider = setup_provider().await;
    let context = RequestContext::with_generated_id();

    if let Some(expected_version) = extract_if_match_header(&headers) {
        // Client provided If-Match header - use conditional update
        let result = provider.conditional_update(
            "User",
            &user_id,
            update_data,
            &expected_version,
            &context
        ).await?;

        match result {
            ConditionalResult::Success(_) => {
                println!("Conditional update succeeded");
            }
            ConditionalResult::VersionMismatch(_) => {
                println!("Return 412 Precondition Failed");
            }
            ConditionalResult::NotFound => {
                println!("Return 404 Not Found");
            }
        }
    } else {
        // No If-Match header - use regular update (less safe)
        let _updated = provider.update_resource("User", &user_id, update_data, &context).await?;
        println!("Regular update performed");
    }

    Ok(())
}
```

### Setting Response Headers

```rust
use axum::http::{HeaderMap, HeaderValue};

fn add_etag_header(headers: &mut HeaderMap, version: &ScimVersion) -> Result<(), Box<dyn std::error::Error>> {
    let etag_value = version.to_http_header();
    headers.insert("ETag", HeaderValue::from_str(&etag_value)?);
    Ok(())
}

// Example: Adding ETag to response
async fn get_user_with_etag(user_id: &str) -> Result<(serde_json::Value, HeaderMap), Box<dyn std::error::Error>> {
    let provider = setup_provider().await;
    let context = RequestContext::with_generated_id();

    let versioned = provider
        .get_versioned_resource("User", user_id, &context)
        .await?
        .ok_or("User not found")?;

    let resource_json = versioned.resource().to_json()?;
    let mut headers = HeaderMap::new();
    add_etag_header(&mut headers, versioned.version())?;

    Ok((resource_json, headers))
}
```

## Implementing Retry Logic

### Simple Retry with Exponential Backoff

```rust
use std::time::Duration;

async fn retry_conditional_update(
    provider: &StandardResourceProvider<InMemoryStorage>,
    resource_type: &str,
    resource_id: &str,
    update_data: serde_json::Value,
    context: &RequestContext,
    max_retries: usize,
) -> Result<(), Box<dyn std::error::Error>> {
    for attempt in 0..max_retries {
        // Get current version
        let versioned = provider
            .get_versioned_resource(resource_type, resource_id, context)
            .await?
            .ok_or("Resource not found")?;

        // Attempt conditional update
        match provider.conditional_update(
            resource_type,
            resource_id,
            update_data.clone(),
            versioned.version(),
            context
        ).await? {
            ConditionalResult::Success(_) => {
                println!("Update succeeded on attempt {}", attempt + 1);
                return Ok(());
            }
            ConditionalResult::VersionMismatch(_) => {
                if attempt + 1 >= max_retries {
                    return Err("Maximum retries exceeded".into());
                }
                
                // Exponential backoff
                let delay = Duration::from_millis(100 * 2_u64.pow(attempt as u32));
                println!("Version conflict on attempt {}, retrying in {:?}", attempt + 1, delay);
                tokio::time::sleep(delay).await;
            }
            ConditionalResult::NotFound => {
                return Err("Resource was deleted".into());
            }
        }
    }

    Err("Should not reach here".into())
}
```

### Retry with Jitter

```rust
use rand::Rng;

async fn retry_with_jitter(
    provider: &StandardResourceProvider<InMemoryStorage>,
    resource_type: &str,
    resource_id: &str,
    update_data: serde_json::Value,
    context: &RequestContext,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut rng = rand::thread_rng();
    
    for attempt in 0..5 {
        let versioned = provider
            .get_versioned_resource(resource_type, resource_id, context)
            .await?
            .ok_or("Resource not found")?;

        match provider.conditional_update(
            resource_type,
            resource_id,
            update_data.clone(),
            versioned.version(),
            context
        ).await? {
            ConditionalResult::Success(_) => return Ok(()),
            ConditionalResult::VersionMismatch(_) if attempt < 4 => {
                // Add jitter to avoid thundering herd
                let base_delay = 100 * 2_u64.pow(attempt as u32);
                let jitter = rng.gen_range(0..=base_delay / 2);
                let delay = Duration::from_millis(base_delay + jitter);
                
                tokio::time::sleep(delay).await;
            }
            ConditionalResult::VersionMismatch(_) => {
                return Err("Too many conflicts".into());
            }
            ConditionalResult::NotFound => {
                return Err("Resource deleted".into());
            }
        }
    }

    Ok(())
}
```

## Advanced Patterns

### Optimistic Update with Fallback

```rust
async fn optimistic_update_pattern(
    provider: &StandardResourceProvider<InMemoryStorage>,
    user_id: &str,
    context: &RequestContext,
) -> Result<(), Box<dyn std::error::Error>> {
    // Try optimistic update first (assume no conflicts)
    let versioned = provider
        .get_versioned_resource("User", user_id, context)
        .await?
        .ok_or("User not found")?;

    let update_data = json!({
        "userName": versioned.resource().get_attribute("userName"),
        "displayName": "Quick Update",
        "active": true
    });

    match provider.conditional_update(
        "User",
        user_id,
        update_data.clone(),
        versioned.version(),
        context
    ).await? {
        ConditionalResult::Success(updated) => {
            println!("Optimistic update succeeded");
            return Ok(());
        }
        ConditionalResult::VersionMismatch(_) => {
            println!("Conflict detected, falling back to retry logic");
            // Fall through to retry logic
        }
        ConditionalResult::NotFound => {
            return Err("Resource deleted".into());
        }
    }

    // Fallback: use retry logic for conflicted update
    retry_conditional_update(provider, "User", user_id, update_data, context, 3).await
}
```

### Batch Operations with Conflict Handling

```rust
async fn batch_update_with_conflicts(
    provider: &StandardResourceProvider<InMemoryStorage>,
    updates: Vec<(String, serde_json::Value)>, // (user_id, update_data)
    context: &RequestContext,
) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let mut successful_updates = Vec::new();
    let mut failed_updates = Vec::new();

    for (user_id, update_data) in updates {
        match retry_conditional_update(provider, "User", &user_id, update_data, context, 3).await {
            Ok(()) => {
                successful_updates.push(user_id.clone());
                println!("✅ Updated user {}", user_id);
            }
            Err(e) => {
                failed_updates.push((user_id.clone(), e.to_string()));
                println!("❌ Failed to update user {}: {}", user_id, e);
            }
        }
    }

    println!("Batch update complete: {} successful, {} failed", 
             successful_updates.len(), failed_updates.len());

    if !failed_updates.is_empty() {
        println!("Failed updates: {:?}", failed_updates);
    }

    Ok(successful_updates)
}
```

## Testing Concurrency

### Unit Testing Conditional Operations

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use tokio_test;

    #[tokio::test]
    async fn test_conditional_update_success() {
        let provider = setup_provider().await;
        let context = RequestContext::with_generated_id();

        // Create user
        let user_data = json!({
            "userName": "test.user",
            "displayName": "Test User",
            "active": true
        });

        let user = provider.create_resource("User", user_data, &context).await.unwrap();
        let user_id = user.get_id().unwrap();

        // Get version
        let versioned = provider
            .get_versioned_resource("User", user_id, &context)
            .await
            .unwrap()
            .unwrap();

        // Update
        let update_data = json!({
            "userName": "test.user",
            "displayName": "Updated User",
            "active": false
        });

        let result = provider.conditional_update(
            "User",
            user_id,
            update_data,
            versioned.version(),
            &context
        ).await.unwrap();

        assert!(matches!(result, ConditionalResult::Success(_)));
    }

    #[tokio::test]
    async fn test_version_conflict() {
        let provider = setup_provider().await;
        let context = RequestContext::with_generated_id();

        // Create user
        let user_data = json!({
            "userName": "conflict.user",
            "displayName": "Conflict User",
            "active": true
        });

        let user = provider.create_resource("User", user_data, &context).await.unwrap();
        let user_id = user.get_id().unwrap();

        // Get initial version
        let initial_versioned = provider
            .get_versioned_resource("User", user_id, &context)
            .await
            .unwrap()
            .unwrap();

        // Simulate another client's update
        let intermediate_update = json!({
            "userName": "conflict.user",
            "displayName": "Modified by Client 1",
            "active": true
        });

        provider.conditional_update(
            "User",
            user_id,
            intermediate_update,
            initial_versioned.version(),
            &context
        ).await.unwrap();

        // Now try to update with stale version
        let conflicting_update = json!({
            "userName": "conflict.user",
            "displayName": "Modified by Client 2",
            "active": false
        });

        let result = provider.conditional_update(
            "User",
            user_id,
            conflicting_update,
            initial_versioned.version(), // This is now stale
            &context
        ).await.unwrap();

        assert!(matches!(result, ConditionalResult::VersionMismatch(_)));
    }
}
```

## Performance Optimization

### Caching Versions

```rust
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

struct VersionCache {
    cache: Arc<RwLock<HashMap<String, ScimVersion>>>,
}

impl VersionCache {
    fn new() -> Self {
        Self {
            cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    async fn get(&self, key: &str) -> Option<ScimVersion> {
        let cache = self.cache.read().await;
        cache.get(key).cloned()
    }

    async fn set(&self, key: String, version: ScimVersion) {
        let mut cache = self.cache.write().await;
        cache.insert(key, version);
    }

    async fn invalidate(&self, key: &str) {
        let mut cache = self.cache.write().await;
        cache.remove(key);
    }
}

// Example usage with caching
async fn cached_get_versioned_resource(
    provider: &StandardResourceProvider<InMemoryStorage>,
    resource_type: &str,
    resource_id: &str,
    context: &RequestContext,
    cache: &VersionCache,
) -> Result<Option<VersionedResource>, Box<dyn std::error::Error>> {
    let cache_key = format!("{}:{}", resource_type, resource_id);
    
    // Check cache first
    if let Some(cached_version) = cache.get(&cache_key).await {
        // Get resource and verify version still matches
        if let Some(resource) = provider.get_resource(resource_type, resource_id, context).await? {
            let current_version = VersionedResource::new(resource.clone()).version().clone();
            if current_version.matches(&cached_version) {
                return Ok(Some(VersionedResource::new(resource)));
            } else {
                // Cache is stale, invalidate
                cache.invalidate(&cache_key).await;
            }
        }
    }
    
    // Cache miss or stale - fetch fresh
    let versioned = provider.get_versioned_resource(resource_type, resource_id, context).await?;
    
    if let Some(ref v) = versioned {
        cache.set(cache_key, v.version().clone()).await;
    }
    
    Ok(versioned)
}
```

## Error Handling Best Practices

### Comprehensive Error Handling

```rust
#[derive(Debug)]
enum UpdateError {
    NotFound,
    Conflict(String),
    TooManyRetries,
    InvalidData(String),
    Internal(String),
}

impl std::fmt::Display for UpdateError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            UpdateError::NotFound => write!(f, "Resource not found"),
            UpdateError::Conflict(msg) => write!(f, "Version conflict: {}", msg),
            UpdateError::TooManyRetries => write!(f, "Too many retry attempts"),
            UpdateError::InvalidData(msg) => write!(f, "Invalid data: {}", msg),
            UpdateError::Internal(msg) => write!(f, "Internal error: {}", msg),
        }
    }
}

impl std::error::Error for UpdateError {}

async fn robust_conditional_update(
    provider: &StandardResourceProvider<InMemoryStorage>,
    resource_type: &str,
    resource_id: &str,
    update_data: serde_json::Value,
    context: &RequestContext,
) -> Result<VersionedResource, UpdateError> {
    const MAX_RETRIES: usize = 3;
    
    for attempt in 0..MAX_RETRIES {
        // Get current resource
        let versioned = match provider.get_versioned_resource(resource_type, resource_id, context).await {
            Ok(Some(v)) => v,
            Ok(None) => return Err(UpdateError::NotFound),
            Err(e) => return Err(UpdateError::Internal(e.to_string())),
        };

        // Attempt update
        match provider.conditional_update(
            resource_type,
            resource_id,
            update_data.clone(),
            versioned.version(),
            context
        ).await {
            Ok(ConditionalResult::Success(updated)) => {
                return Ok(updated);
            }
            Ok(ConditionalResult::VersionMismatch(conflict)) => {
                if attempt + 1 >= MAX_RETRIES {
                    return Err(UpdateError::TooManyRetries);
                }
                
                // Log conflict and retry
                println!("Attempt {}: Version conflict - {}", attempt + 1, conflict.message);
                
                let delay = std::time::Duration::from_millis(100 * 2_u64.pow(attempt as u32));
                tokio::time::sleep(delay).await;
            }
            Ok(ConditionalResult::NotFound) => {
                return Err(UpdateError::NotFound);
            }
            Err(e) => {
                return Err(UpdateError::Internal(e.to_string()));
            }
        }
    }
    
    Err(UpdateError::TooManyRetries)
}
```

This implementation guide provides practical, working examples for using the actual concurrency features available in the SCIM Server library. All examples are based on the real `StandardResourceProvider` implementation and can be adapted for your specific use cases.