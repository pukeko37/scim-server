# Conflict Resolution

> **TODO**: This section is under development. Basic conflict resolution patterns are outlined below.

## Overview

When multiple clients attempt to modify the same SCIM resource simultaneously, conflicts can occur. This guide covers strategies for detecting and resolving these conflicts using ETags and version control.

## Conflict Detection

### ETag-Based Detection

```rust
use scim_server::{ConditionalResult, ETag};

async fn update_user_with_conflict_detection(
    provider: &impl ResourceProvider,
    user_id: &str,
    data: serde_json::Value,
    expected_etag: &ETag,
) -> Result<ScimUser, ConflictError> {
    let context = RequestContext::new("update", None);
    
    match provider.conditional_update("User", user_id, data, expected_etag, &context).await? {
        ConditionalResult::Success(user) => Ok(user),
        ConditionalResult::VersionMismatch(conflict) => {
            Err(ConflictError::VersionMismatch {
                expected: expected_etag.clone(),
                current: conflict.current_version,
                resource_id: user_id.to_string(),
            })
        },
        ConditionalResult::NotFound => Err(ConflictError::ResourceNotFound),
    }
}
```

## Resolution Strategies

### 1. Last-Writer-Wins

```rust
// Simple approach: retry with latest version
async fn retry_with_latest(
    provider: &impl ResourceProvider,
    user_id: &str,
    data: serde_json::Value,
) -> Result<ScimUser, Box<dyn std::error::Error>> {
    let context = RequestContext::new("retry", None);
    
    // Get current version
    let current = provider.get_resource("User", user_id, &context).await?
        .ok_or("User not found")?;
    
    // Apply update with current version
    provider.update_resource("User", user_id, data, &context).await
}
```

### 2. Three-Way Merge

> **TODO**: Implement sophisticated merge strategies for complex conflicts.

### 3. Client-Side Resolution

> **TODO**: Add examples for client-side conflict resolution patterns.

## Error Handling

```rust
#[derive(Debug)]
pub enum ConflictError {
    VersionMismatch {
        expected: ETag,
        current: ETag,
        resource_id: String,
    },
    ResourceNotFound,
    MergeConflict {
        field: String,
        local_value: serde_json::Value,
        remote_value: serde_json::Value,
    },
}

impl std::fmt::Display for ConflictError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConflictError::VersionMismatch { expected, current, resource_id } => {
                write!(f, "Version conflict on {}: expected {}, got {}", 
                       resource_id, expected, current)
            },
            ConflictError::ResourceNotFound => write!(f, "Resource not found"),
            ConflictError::MergeConflict { field, .. } => {
                write!(f, "Merge conflict in field: {}", field)
            },
        }
    }
}
```

## Best Practices

1. **Always use ETags** for update operations
2. **Implement retry logic** with exponential backoff
3. **Provide clear error messages** to clients
4. **Log conflicts** for monitoring and debugging

> **TODO**: Add more sophisticated conflict resolution algorithms and patterns.
