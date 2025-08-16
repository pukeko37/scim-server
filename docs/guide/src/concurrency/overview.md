# Concurrency Overview

This guide covers the ETag-based concurrency control features available in the SCIM Server library. These features provide optimistic concurrency control to prevent lost updates when multiple clients modify the same resources simultaneously.

## What is Concurrency Control?

Concurrency control ensures that simultaneous operations on shared resources don't interfere with each other or corrupt data. In SCIM servers, this is particularly important when multiple clients are:

- Modifying the same user or group
- Creating resources with unique constraints
- Performing bulk operations
- Running in distributed deployments

## SCIM 2.0 ETag Support

The SCIM Server library implements ETag-based optimistic concurrency control as specified in RFC 7644 (SCIM 2.0) and RFC 7232 (HTTP ETags).

### How ETags Work

1. **Version Generation**: Each resource gets a unique version identifier computed from its content
2. **Client Requests**: Clients include ETags in conditional requests via HTTP headers
3. **Version Checking**: Server validates ETags before modifications
4. **Conflict Detection**: Mismatched ETags indicate concurrent modifications

## Core Types

The concurrency system is built around three main types:

### ScimVersion

Represents an opaque version identifier for resources:

```rust
use scim_server::resource::version::ScimVersion;

// Version is automatically computed from resource content
let resource_data = br#"{"id":"123","userName":"john.doe","active":true}"#;
let version = ScimVersion::from_content(resource_data);

// Convert to HTTP weak ETag header for responses
let etag_header = version.to_http_header(); // Returns: "W/abc123def"

// Parse from client-provided ETag header
let client_version = ScimVersion::parse_http_header("W/\"abc123def\"").unwrap();

// Check if versions match
if version.matches(&client_version) {
    println!("Versions match - safe to proceed");
}
```

### VersionedResource

Wraps a resource with its version information:

```rust
use scim_server::resource::{
    conditional_provider::VersionedResource,
    core::Resource,
};
use serde_json::json;

let resource = Resource::from_json("User".to_string(), json!({
    "id": "123",
    "userName": "john.doe",
    "active": true
})).unwrap();

let versioned = VersionedResource::new(resource);
println!("Resource version: {}", versioned.version().to_http_header());

// Access the underlying resource
let resource_ref = versioned.resource();
let version_ref = versioned.version();
```

### ConditionalResult

Represents the outcome of conditional operations:

```rust
use scim_server::resource::version::ConditionalResult;

match conditional_result {
    ConditionalResult::Success(versioned_resource) => {
        println!("Operation succeeded!");
        println!("New version: {}", versioned_resource.version().to_http_header());
    },
    ConditionalResult::VersionMismatch(conflict) => {
        println!("Version conflict detected!");
        println!("Expected: {}", conflict.expected.to_http_header());
        println!("Current: {}", conflict.current.to_http_header());
        println!("Message: {}", conflict.message);
    },
    ConditionalResult::NotFound => {
        println!("Resource not found");
    }
}
```

## Provider Integration

All resource providers in the SCIM Server library support conditional operations through the `ResourceProvider` trait:

### Conditional Updates

```rust
use scim_server::{
    providers::StandardResourceProvider,
    storage::InMemoryStorage,
    resource::{RequestContext, ResourceProvider},
};
use serde_json::json;

async fn conditional_update_example() -> Result<(), Box<dyn std::error::Error>> {
    let storage = InMemoryStorage::new();
    let provider = StandardResourceProvider::new(storage);
    let context = RequestContext::with_generated_id();

    // Get current resource with version
    let versioned = provider
        .get_versioned_resource("User", "123", &context)
        .await?
        .ok_or("User not found")?;
    
    let current_version = versioned.version();

    // Prepare update data
    let update_data = json!({
        "userName": "john.updated",
        "displayName": "John Updated",
        "active": false
    });

    // Attempt conditional update
    match provider.conditional_update(
        "User",
        "123",
        update_data,
        current_version,
        &context
    ).await? {
        ConditionalResult::Success(updated_resource) => {
            println!("Update successful!");
            println!("New version: {}", updated_resource.version().to_http_header());
        },
        ConditionalResult::VersionMismatch(conflict) => {
            println!("Conflict detected - another client modified the resource");
            println!("Expected version: {}", conflict.expected.to_http_header());
            println!("Current version: {}", conflict.current.to_http_header());
            // Client should refresh and retry
        },
        ConditionalResult::NotFound => {
            println!("Resource was deleted by another client");
        }
    }

    Ok(())
}
```

### Conditional Deletes

```rust
async fn conditional_delete_example() -> Result<(), Box<dyn std::error::Error>> {
    let storage = InMemoryStorage::new();
    let provider = StandardResourceProvider::new(storage);
    let context = RequestContext::with_generated_id();

    // Get current version
    let versioned = provider
        .get_versioned_resource("User", "123", &context)
        .await?
        .ok_or("User not found")?;

    let current_version = versioned.version();

    // Attempt conditional delete
    match provider.conditional_delete(
        "User",
        "123",
        current_version,
        &context
    ).await? {
        ConditionalResult::Success(()) => {
            println!("Delete successful!");
        },
        ConditionalResult::VersionMismatch(conflict) => {
            println!("Conflict detected - resource was modified");
            // Handle conflict appropriately
        },
        ConditionalResult::NotFound => {
            println!("Resource already deleted");
        }
    }

    Ok(())
}
```

## HTTP Integration

The version system integrates seamlessly with HTTP headers:

### Request Headers

- **If-Match**: Client provides expected ETag for updates/deletes
- **If-None-Match**: Client provides ETag for conditional creates

### Response Headers

- **ETag**: Server provides current resource version
- **412 Precondition Failed**: Returned on version conflicts

```rust
// Example: Processing HTTP conditional request
use axum::http::HeaderMap;

fn extract_if_match(headers: &HeaderMap) -> Option<ScimVersion> {
    headers.get("If-Match")
        .and_then(|h| h.to_str().ok())
        .and_then(|s| ScimVersion::parse_http_header(s).ok())
}

fn add_etag_header(headers: &mut HeaderMap, version: &ScimVersion) {
    headers.insert("ETag", version.to_http_header().parse().unwrap());
}
```

## Conflict Resolution Strategies

When version conflicts occur, applications can handle them in several ways:

### 1. Retry with Latest Version

```rust
async fn retry_update_example() -> Result<(), Box<dyn std::error::Error>> {
    let storage = InMemoryStorage::new();
    let provider = StandardResourceProvider::new(storage);
    let context = RequestContext::with_generated_id();

    let update_data = json!({"displayName": "New Name"});
    let mut retry_count = 0;
    let max_retries = 3;

    loop {
        // Get latest version
        let versioned = provider
            .get_versioned_resource("User", "123", &context)
            .await?
            .ok_or("User not found")?;

        // Attempt update
        match provider.conditional_update(
            "User",
            "123",
            update_data.clone(),
            versioned.version(),
            &context
        ).await? {
            ConditionalResult::Success(updated) => {
                println!("Update succeeded after {} retries", retry_count);
                return Ok(());
            },
            ConditionalResult::VersionMismatch(_) => {
                retry_count += 1;
                if retry_count >= max_retries {
                    return Err("Max retries exceeded".into());
                }
                // Brief delay before retry
                tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                continue;
            },
            ConditionalResult::NotFound => {
                return Err("Resource was deleted".into());
            }
        }
    }
}
```

### 2. Merge Changes

```rust
async fn merge_update_example() -> Result<(), Box<dyn std::error::Error>> {
    let storage = InMemoryStorage::new();
    let provider = StandardResourceProvider::new(storage);
    let context = RequestContext::with_generated_id();

    // Get original resource (before our changes)
    let original = provider.get_resource("User", "123", &context).await?
        .ok_or("User not found")?;

    // Our intended changes
    let my_changes = json!({"displayName": "My Update"});

    // Get current version
    let current_versioned = provider
        .get_versioned_resource("User", "123", &context)
        .await?
        .ok_or("User not found")?;

    // Check if resource was modified by others
    if VersionedResource::new(original.clone()).version() != *current_versioned.version() {
        // Resource was modified - attempt merge
        let current_resource = current_versioned.resource();
        
        // Simple merge: preserve other changes, apply our changes
        let mut merged_data = current_resource.to_json()?;
        if let (Some(merged_obj), Some(changes_obj)) = 
            (merged_data.as_object_mut(), my_changes.as_object()) {
            for (key, value) in changes_obj {
                merged_obj.insert(key.clone(), value.clone());
            }
        }

        // Attempt update with current version
        match provider.conditional_update(
            "User",
            "123",
            merged_data,
            current_versioned.version(),
            &context
        ).await? {
            ConditionalResult::Success(_) => {
                println!("Merge successful");
            },
            ConditionalResult::VersionMismatch(_) => {
                println!("Another conflict occurred during merge - retry needed");
            },
            ConditionalResult::NotFound => {
                return Err("Resource was deleted".into());
            }
        }
    }

    Ok(())
}
```

## Error Handling

Concurrency-related errors are handled through the `ConditionalResult` type and standard provider error types:

```rust
use scim_server::resource::version::{ConditionalResult, VersionConflict};

// Version conflicts are returned as ConditionalResult::VersionMismatch
// rather than errors, allowing graceful handling
fn handle_conflict(conflict: VersionConflict) {
    eprintln!("Version conflict detected:");
    eprintln!("  Expected: {}", conflict.expected.to_http_header());
    eprintln!("  Current:  {}", conflict.current.to_http_header());
    eprintln!("  Message:  {}", conflict.message);
    
    // Recommend client action
    eprintln!("Client should refresh resource and retry operation");
}
```

## Best Practices

### 1. Always Use Conditional Operations

```rust
// Good: Check version before update
async fn good_update() -> Result<(), Box<dyn std::error::Error>> {
    let storage = InMemoryStorage::new();
    let provider = StandardResourceProvider::new(storage);
    let context = RequestContext::with_generated_id();
    
    let versioned = provider.get_versioned_resource("User", "123", &context).await?
        .ok_or("User not found")?;
    
    let result = provider.conditional_update(
        "User", "123", 
        json!({"active": false}),
        versioned.version(),
        &context
    ).await?;
    
    match result {
        ConditionalResult::Success(_) => println!("Update succeeded"),
        ConditionalResult::VersionMismatch(_) => println!("Conflict - retry needed"),
        ConditionalResult::NotFound => println!("Resource deleted"),
    }
    
    Ok(())
}

// Avoid: Blind updates without version checking
async fn risky_update() -> Result<(), Box<dyn std::error::Error>> {
    let storage = InMemoryStorage::new();
    let provider = StandardResourceProvider::new(storage);
    let context = RequestContext::with_generated_id();
    
    // This could overwrite concurrent changes!
    let _updated = provider.update_resource(
        "User", "123",
        json!({"active": false}),
        &context
    ).await?;
    
    Ok(())
}
```

### 2. Handle All ConditionalResult Cases

```rust
// Always handle all possible outcomes
match conditional_result {
    ConditionalResult::Success(resource) => {
        // Handle successful operation
    },
    ConditionalResult::VersionMismatch(conflict) => {
        // Always handle conflicts appropriately
        // Don't ignore or treat as fatal errors
    },
    ConditionalResult::NotFound => {
        // Handle missing resource case
    }
}
```

### 3. Implement Retry Logic

```rust
async fn robust_update() -> Result<(), Box<dyn std::error::Error>> {
    let storage = InMemoryStorage::new();
    let provider = StandardResourceProvider::new(storage);
    let context = RequestContext::with_generated_id();
    let update_data = json!({"active": false});
    
    for attempt in 1..=3 {
        let versioned = provider.get_versioned_resource("User", "123", &context).await?
            .ok_or("User not found")?;
            
        match provider.conditional_update(
            "User", "123",
            update_data.clone(),
            versioned.version(),
            &context
        ).await? {
            ConditionalResult::Success(_) => return Ok(()),
            ConditionalResult::VersionMismatch(_) if attempt < 3 => {
                // Retry with exponential backoff
                let delay = std::time::Duration::from_millis(100 * 2_u64.pow(attempt - 1));
                tokio::time::sleep(delay).await;
                continue;
            },
            ConditionalResult::VersionMismatch(_) => {
                return Err("Too many version conflicts".into());
            },
            ConditionalResult::NotFound => {
                return Err("Resource was deleted".into());
            }
        }
    }
    
    Ok(())
}
```

### 4. Monitor Conflict Rates

Track version conflicts in your application to identify resources with high contention:

```rust
struct ConflictMetrics {
    total_attempts: usize,
    version_conflicts: usize,
}

impl ConflictMetrics {
    fn record_attempt(&mut self) {
        self.total_attempts += 1;
    }
    
    fn record_conflict(&mut self) {
        self.version_conflicts += 1;
    }
    
    fn conflict_rate(&self) -> f64 {
        if self.total_attempts == 0 {
            0.0
        } else {
            self.version_conflicts as f64 / self.total_attempts as f64
        }
    }
}
```

## Performance Considerations

### Version Computation

- Versions are computed using SHA-256 hashing of resource content
- Computation is deterministic and relatively fast
- Consider caching versions for frequently accessed resources

### Storage Optimization

- Implement conditional operations at the storage layer for best performance
- Use database features like row versioning where available
- Consider using database-specific optimistic locking mechanisms

## Next Steps

- [Implementation Guide](./implementation.md) - Learn to implement conditional operations
- [Conflict Resolution](./conflict-resolution.md) - Handle version conflicts gracefully