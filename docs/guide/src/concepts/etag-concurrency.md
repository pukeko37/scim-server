# ETag Concurrency Control

ETag concurrency control is a critical feature for preventing lost updates in multi-client environments. This chapter explains how SCIM Server implements enterprise-grade optimistic locking using ETags to ensure data consistency.

## What are ETags?

ETags (Entity Tags) are HTTP headers that represent the version of a resource. They enable optimistic concurrency control, where multiple clients can work on the same resource without locking, but updates are validated to prevent conflicts.

### Benefits of ETag Concurrency Control

- **Prevent Lost Updates**: Avoid scenarios where one client overwrites another's changes
- **Optimistic Locking**: No blocking - clients work independently until conflict detection
- **Performance**: Better than pessimistic locking for distributed systems
- **Consistency**: Ensure data integrity in concurrent environments
- **Auditability**: Track version changes for compliance and debugging

## How ETags Work in SCIM Server

SCIM Server automatically manages ETags for all resources, providing seamless concurrency control:

```rust
use scim_server::{ScimServer, storage::InMemoryStorage};
use serde_json::json;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let storage = InMemoryStorage::new();
    let server = ScimServer::new(storage).await?;
    
    // Create a user - ETag is automatically generated
    let user_data = json!({
        "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
        "userName": "alice@example.com",
        "active": true
    });

    let user = server.create_user("tenant-1", user_data).await?;
    println!("Created user with ETag: {}", user.meta.version);
    // Output: W/"1-abc123def456"
    
    Ok(())
}
```

## ETag Format and Structure

SCIM Server uses weak ETags following HTTP standards:

```
W/"<version>-<hash>"
```

- **W/**: Indicates a weak ETag (semantic equivalence)
- **version**: Monotonically increasing version number
- **hash**: Content hash for additional validation

Examples:
- `W/"1-a1b2c3d4"` - Version 1, first creation
- `W/"2-e5f6g7h8"` - Version 2, after first update
- `W/"3-i9j0k1l2"` - Version 3, after second update

## Basic Concurrency Control

### Reading Resources with ETags

All read operations return the current ETag:

```rust
use scim_server::{ScimServer, TenantId};

async fn read_user_with_etag(
    server: &ScimServer,
    tenant_id: &TenantId,
    user_id: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let user = server.get_user(tenant_id, user_id).await?;
    
    println!("User: {}", user.user_name);
    println!("Current ETag: {}", user.meta.version);
    println!("Last Modified: {}", user.meta.last_modified);
    
    Ok(())
}
```

### Conditional Updates

Use ETags to ensure updates only succeed if the resource hasn't changed:

```rust
use scim_server::{ScimServer, ETag, ConditionalResult};

async fn safe_update_user(
    server: &ScimServer,
    tenant_id: &TenantId,
    user_id: &str,
    expected_etag: &ETag,
    updates: serde_json::Value,
) -> Result<(), Box<dyn std::error::Error>> {
    match server.conditional_update_user(
        tenant_id,
        user_id,
        updates,
        Some(expected_etag),
    ).await? {
        ConditionalResult::Success(updated_user) => {
            println!("Update successful!");
            println!("New ETag: {}", updated_user.meta.version);
        },
        ConditionalResult::VersionMismatch { expected, current } => {
            println!("Version conflict detected!");
            println!("Expected: {}, Current: {}", expected, current);
            // Handle conflict - see conflict resolution section
        },
        ConditionalResult::NotFound => {
            println!("User no longer exists");
        }
    }
    
    Ok(())
}
```

## Advanced Concurrency Patterns

### Optimistic Update with Retry

Handle conflicts gracefully with automatic retry:

```rust
use scim_server::{ScimServer, ConditionalResult, BackoffStrategy};
use tokio::time::{sleep, Duration};

async fn optimistic_update_with_retry(
    server: &ScimServer,
    tenant_id: &TenantId,
    user_id: &str,
    update_fn: impl Fn(&serde_json::Value) -> serde_json::Value,
    max_retries: u32,
) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    for attempt in 0..max_retries {
        // Get current version
        let current_user = server.get_user(tenant_id, user_id).await?;
        let current_etag = &current_user.meta.version;
        
        // Apply updates
        let updated_data = update_fn(&current_user);
        
        // Attempt conditional update
        match server.conditional_update_user(
            tenant_id,
            user_id,
            updated_data,
            Some(current_etag),
        ).await? {
            ConditionalResult::Success(user) => return Ok(user),
            ConditionalResult::VersionMismatch { .. } => {
                if attempt < max_retries - 1 {
                    // Exponential backoff before retry
                    let delay = Duration::from_millis(100 * 2_u64.pow(attempt));
                    sleep(delay).await;
                    continue;
                } else {
                    return Err("Max retries exceeded".into());
                }
            },
            ConditionalResult::NotFound => {
                return Err("User was deleted during update".into());
            }
        }
    }
    
    unreachable!()
}
```

### Batch Operations with Version Checking
## Advanced Patterns

### Multiple Operations with ETags

```rust
use scim_server::{ConditionalResult, VersionedResource};

async fn batch_update_with_etags(
    provider: &impl ResourceProvider,
    tenant_id: &TenantId,
    operations: Vec<(String, serde_json::Value, ETag)>, // (user_id, data, expected_etag)
) -> Result<Vec<ConditionalResult<VersionedResource>>, Box<dyn std::error::Error>> {
    let mut results = Vec::new();
    let context = RequestContext::new("batch-update", None);
    
    // Process each operation individually (bulk operations not yet implemented)
    for (user_id, data, expected_etag) in operations {
        let result = provider.conditional_update(
            "User",
            &user_id,
            data,
            &expected_etag,
            &context
        ).await?;
        
        match &result {
            ConditionalResult::Success(versioned) => {
                println!("Updated {}: new version {}", user_id, versioned.version());
            },
            ConditionalResult::VersionMismatch(conflict) => {
                println!("Conflict on {}: expected {}, got {}", 
                         user_id, expected_etag, conflict.current_version);
            },
            ConditionalResult::NotFound => {
                println!("User {} not found", user_id);
            }
        }
        
        results.push(result);
    }
    
    Ok(results)
}
```

## Conflict Resolution Strategies

When version conflicts occur, several strategies can be employed:

### Strategy 1: Last Writer Wins (Forced Update)

```rust
async fn force_update(
    server: &ScimServer,
    tenant_id: &TenantId,
    user_id: &str,
    updates: serde_json::Value,
) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    // Update without ETag check - potentially dangerous!
    let result = server.update_user(tenant_id, user_id, updates).await?;
    println!("Forced update completed");
    Ok(result)
}
```

⚠️ **Warning**: Use this strategy only when you're certain it's safe to overwrite changes.

### Strategy 2: Merge Changes

```rust
use serde_json::{Value, Map};

async fn merge_and_update(
    server: &ScimServer,
    tenant_id: &TenantId,
    user_id: &str,
    my_changes: serde_json::Value,
    max_attempts: u32,
) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    for attempt in 0..max_attempts {
        // Get current state
        let current_user = server.get_user(tenant_id, user_id).await?;
        let current_etag = &current_user.meta.version;
        
        // Merge changes (simple field-level merge)
        let merged_data = merge_user_data(&current_user, &my_changes)?;
        
        // Attempt update with current ETag
        match server.conditional_update_user(
            tenant_id,
            user_id,
            merged_data,
            Some(current_etag),
        ).await? {
            ConditionalResult::Success(user) => return Ok(user),
            ConditionalResult::VersionMismatch { .. } => {
                // Retry with fresh data
                continue;
            },
            ConditionalResult::NotFound => {
                return Err("User was deleted".into());
            }
        }
    }
    
    Err("Failed to merge after maximum attempts".into())
}

fn merge_user_data(
    current: &serde_json::Value,
    changes: &serde_json::Value,
) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    let mut merged = current.clone();
    
    if let (Some(current_obj), Some(changes_obj)) = (
        merged.as_object_mut(),
        changes.as_object()
    ) {
        for (key, value) in changes_obj {
            // Simple field replacement - you might want more sophisticated merging
            current_obj.insert(key.clone(), value.clone());
        }
    }
    
    Ok(merged)
}
```

### Strategy 3: User-Mediated Resolution

```rust
use scim_server::ConflictResolution;

async fn resolve_conflict_interactively(
    server: &ScimServer,
    tenant_id: &TenantId,
    user_id: &str,
    my_changes: serde_json::Value,
    conflict: ConflictResolution,
) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    println!("Conflict detected for user {}", user_id);
    println!("Your changes: {}", serde_json::to_string_pretty(&my_changes)?);
    println!("Current state: {}", serde_json::to_string_pretty(&conflict.current_state)?);
    
    // In a real application, present UI for user to choose resolution
    let resolution = prompt_user_for_resolution(&my_changes, &conflict.current_state)?;
    
    match resolution {
        UserChoice::KeepMine => {
            // Force update with my changes
            server.update_user(tenant_id, user_id, my_changes).await
        },
        UserChoice::KeepTheirs => {
            // Return current state, no update needed
            Ok(conflict.current_state)
        },
        UserChoice::Merge(merged_data) => {
            // Use user-provided merge
            server.update_user(tenant_id, user_id, merged_data).await
        }
    }
}

enum UserChoice {
    KeepMine,
    KeepTheirs,
    Merge(serde_json::Value),
}
```

## HTTP Integration

### ETag Headers in HTTP Responses

SCIM Server automatically includes ETag headers in HTTP responses:

```rust
use axum::{response::Response, http::HeaderMap};

async fn http_get_user(
    tenant_id: String,
    user_id: String,
    server: ScimServer,
) -> Result<Response, AppError> {
    let user = server.get_user(&TenantId::new(tenant_id), &user_id).await?;
    
    let mut headers = HeaderMap::new();
    headers.insert("ETag", user.meta.version.to_string().parse()?);
    headers.insert("Last-Modified", user.meta.last_modified.to_rfc2822().parse()?);
    
    let response = Response::builder()
        .status(200)
        .header("Content-Type", "application/scim+json")
        .header("ETag", user.meta.version.to_string())
        .body(serde_json::to_string(&user)?)
        .unwrap();
    
    Ok(response)
}
```

### Conditional Requests with If-Match

Handle conditional updates via HTTP If-Match headers:

```rust
use axum::{extract::HeaderMap, http::StatusCode};

async fn http_update_user(
    tenant_id: String,
    user_id: String,
    headers: HeaderMap,
    Json(updates): Json<serde_json::Value>,
    server: ScimServer,
) -> Result<Response, AppError> {
    let if_match = headers.get("If-Match")
        .and_then(|v| v.to_str().ok())
        .map(ETag::parse)
        .transpose()?;
    
    match server.conditional_update_user(
        &TenantId::new(tenant_id),
        &user_id,
        updates,
        if_match.as_ref(),
    ).await? {
        ConditionalResult::Success(user) => {
            Ok(Response::builder()
                .status(200)
                .header("ETag", user.meta.version.to_string())
                .body(serde_json::to_string(&user)?)
                .unwrap())
        },
        ConditionalResult::VersionMismatch { expected, current } => {
            Ok(Response::builder()
                .status(StatusCode::PRECONDITION_FAILED)
                .header("ETag", current.to_string())
                .body(format!("Version mismatch: expected {}, current {}", expected, current))
                .unwrap())
        },
        ConditionalResult::NotFound => {
            Ok(Response::builder()
                .status(StatusCode::NOT_FOUND)
                .body("User not found")
                .unwrap())
        }
    }
}
```

## Performance Considerations

### ETag Storage Optimization

ETags are stored efficiently to minimize overhead:

```rust
use scim_server::storage::{ETagStorage, CompressionLevel};

// Configure ETag storage for optimal performance
let etag_config = ETagStorage::builder()
    .compression(CompressionLevel::Fast)
    .cache_size_mb(256)
    .cleanup_interval_hours(24)
    .build();

let storage = InMemoryStorage::new()
    .with_etag_config(etag_config);
```

### Batch ETag Operations

Efficiently handle ETags in bulk operations:

```rust
// Pre-fetch ETags for bulk validation
let user_ids = vec!["user1", "user2", "user3"];
let etags = server.get_etags(&tenant_id, &user_ids).await?;

// Validate all ETags before proceeding with bulk operation
for (user_id, expected_etag) in expected_etags {
    let current_etag = etags.get(user_id).ok_or("User not found")?;
    if current_etag != &expected_etag {
        return Err(format!("Version mismatch for user {}", user_id).into());
    }
}
```

## AI Integration and ETags

ETags work seamlessly with AI tools via MCP:

```rust
use scim_server::mcp::{McpTool, ConflictResolutionStrategy};

// AI can handle conflicts intelligently
let ai_conflict_resolver = McpTool::new("claude-3-5-sonnet")
    .with_conflict_strategy(ConflictResolutionStrategy::SmartMerge)
    .with_retry_limit(3);

// AI assistant automatically handles ETag conflicts
let result = ai_conflict_resolver.update_user_safe(
    &tenant_id,
    &user_id,
    json!({
        "active": false,
        "lastLogin": "2024-01-15T10:30:00Z"
    })
).await?;
```

## Best Practices

### Always Use ETags for Updates

```rust
// Good: Always check ETags for updates
let user = server.get_user(&tenant_id, &user_id).await?;
let current_etag = &user.meta.version;
let result = server.conditional_update_user(&tenant_id, &user_id, updates, Some(current_etag)).await?;

// Avoid: Blind updates without version checking
let result = server.update_user(&tenant_id, &user_id, updates).await?; // Risky!
```

### Handle All Conflict Cases

```rust
match server.conditional_update_user(&tenant_id, &user_id, updates, Some(&etag)).await? {
    ConditionalResult::Success(user) => {
        // Success case
    },
    ConditionalResult::VersionMismatch { expected, current } => {
        // Always handle conflicts
    },
    ConditionalResult::NotFound => {
        // Handle deletion case
    }
}
```

### Monitor Conflict Rates

```rust
use scim_server::metrics::ConflictMetrics;

// Track conflict rates to identify problematic access patterns
let metrics = server.get_conflict_metrics(&tenant_id).await?;
if metrics.conflict_rate_percent > 5.0 {
    log::warn!("High conflict rate detected: {}%", metrics.conflict_rate_percent);
    // Consider implementing additional coordination mechanisms
}
```

ETag concurrency control in SCIM Server provides robust protection against data loss while maintaining high performance in concurrent environments. By understanding and properly implementing these patterns, you can build reliable multi-client systems that handle conflicts gracefully.