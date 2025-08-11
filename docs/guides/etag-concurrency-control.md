# ETag Concurrency Control Guide

This guide explains how to use the built-in ETag concurrency control features of the SCIM Server library to prevent lost updates and ensure data integrity in concurrent environments.

## Table of Contents

- [Overview](#overview)
- [Core Concepts](#core-concepts)
- [Architecture](#architecture)
- [Basic Usage](#basic-usage)
- [HTTP Integration](#http-integration)
- [Provider Implementation](#provider-implementation)
- [Advanced Scenarios](#advanced-scenarios)
- [Best Practices](#best-practices)
- [Troubleshooting](#troubleshooting)

## Overview

The SCIM Server library provides automatic ETag-based concurrency control that prevents lost updates when multiple clients modify the same resource simultaneously. This feature is built into the core ResourceProvider trait and works with any storage backend.

### Key Benefits

- **Automatic**: No configuration required - works out of the box
- **Universal**: All resources automatically include version information
- **HTTP Compliant**: Full RFC 7232 ETag support for web applications
- **Provider Agnostic**: Works with any storage backend implementation
- **Type Safe**: Compile-time guarantees for version-aware operations

### What Problems Does This Solve?

```text
Without ETag Control:           With ETag Control:
==================            =================

Client A reads User            Client A reads User (v1)
Client B reads User            Client B reads User (v1)
Client A updates User          Client A updates User with v1 ✓ → User (v2)
Client B updates User          Client B updates User with v1 ✗ → Conflict!
→ Client A's changes lost      → Client B must refresh and retry
```

## Core Concepts

### ScimVersion

Every resource has an associated version computed from its content:

```rust
use scim_server::resource::version::ScimVersion;

// Automatic version from content
let resource_json = br#"{"id":"123","userName":"john.doe","active":true}"#;
let version = ScimVersion::from_content(resource_json);

// Provider-specific version
let db_version = ScimVersion::from_hash("db-sequence-456");

// HTTP ETag parsing
let client_version = ScimVersion::parse_http_header("\"W/abc123def\"").unwrap();

// HTTP ETag generation
let etag_header = version.to_http_header(); // "W/xyz789abc"
```

### VersionedResource

Resources are wrapped with their versions for conditional operations:

```rust
use scim_server::resource::conditional_provider::VersionedResource;

let versioned = VersionedResource::new(resource);
println!("Version: {}", versioned.version().to_http_header());
println!("Resource ID: {}", versioned.resource().get_id().unwrap());
```

### ConditionalResult

Conditional operations return specialized results:

```rust
use scim_server::resource::version::ConditionalResult;

match result {
    ConditionalResult::Success(versioned_resource) => {
        println!("Operation succeeded: {}", versioned_resource.version().to_http_header());
    },
    ConditionalResult::VersionMismatch(conflict) => {
        println!("Version conflict: expected {}, current {}", 
                 conflict.expected, conflict.current);
    },
    ConditionalResult::NotFound => {
        println!("Resource not found");
    }
}
```

## Architecture

### Mandatory Conditional Operations

As of Phase 3, all ResourceProvider implementations include conditional operations by default. This architectural decision provides:

- **Single Code Path**: Consistent behavior across all providers
- **Type Safety**: Compile-time guarantees for version support
- **Production Ready**: Built-in concurrency control for all resources

### Version Computation

Versions are computed using SHA-256 hashing of resource content:

```rust
// Same content always produces same version
let user1 = json!({"id": "123", "userName": "alice", "active": true});
let user2 = json!({"id": "123", "userName": "alice", "active": true});

let v1 = ScimVersion::from_content(user1.to_string().as_bytes());
let v2 = ScimVersion::from_content(user2.to_string().as_bytes());

assert!(v1.matches(&v2)); // Always true for identical content
```

### Provider Integration

```text
┌─────────────────────────────────────────────────┐
│                Application Layer                │
├─────────────────────────────────────────────────┤
│            Operation Handler                    │  ← Handles ETags
├─────────────────────────────────────────────────┤
│              SCIM Server                        │  ← Version management
├─────────────────────────────────────────────────┤
│            ResourceProvider                     │  ← Conditional operations
├─────────────────────────────────────────────────┤
│              Storage Backend                    │  ← Your database
└─────────────────────────────────────────────────┘
```

## Basic Usage

### Creating Resources with Versions

All create operations automatically include version information:

```rust
use scim_server::{ScimServer, providers::InMemoryProvider};
use scim_server::operation_handler::{ScimOperationHandler, ScimOperationRequest};
use serde_json::json;

let provider = InMemoryProvider::new();
let server = ScimServer::new(provider)?;
let handler = ScimOperationHandler::new(server);

let create_request = ScimOperationRequest::create(
    "User",
    json!({
        "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
        "userName": "alice.doe",
        "active": true
    })
);

let response = handler.handle_operation(create_request).await;
if response.success {
    let version = response.metadata.additional.get("version").unwrap();
    let etag = response.metadata.additional.get("etag").unwrap();
    
    println!("Created user with version: {}", version.as_str().unwrap());
    println!("ETag header: {}", etag.as_str().unwrap());
}
```

### Conditional Updates

Update resources only if the version matches:

```rust
use scim_server::resource::version::ScimVersion;

// Get current resource version
let get_request = ScimOperationRequest::get("User", "user-123");
let get_response = handler.handle_operation(get_request).await;

let current_etag = get_response.metadata.additional.get("etag").unwrap();
let current_version = ScimVersion::parse_http_header(current_etag.as_str().unwrap())?;

// Conditional update
let update_request = ScimOperationRequest::update(
    "User",
    "user-123",
    json!({
        "id": "user-123",
        "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
        "userName": "alice.doe",
        "active": false  // Changed field
    })
).with_expected_version(current_version);

let update_response = handler.handle_operation(update_request).await;
if update_response.success {
    println!("Update succeeded!");
} else {
    println!("Version conflict: {}", update_response.error.unwrap());
}
```

### Conditional Deletes

Delete resources only if the version matches:

```rust
let delete_request = ScimOperationRequest::delete("User", "user-123")
    .with_expected_version(current_version);

let delete_response = handler.handle_operation(delete_request).await;
if delete_response.success {
    println!("Delete succeeded!");
} else {
    println!("Version conflict - resource was modified");
}
```

## HTTP Integration

### Server-Side ETag Generation

```rust
// In your HTTP handler
use axum::{response::IntoResponse, http::HeaderMap};

async fn get_user(user_id: String) -> impl IntoResponse {
    let get_request = ScimOperationRequest::get("User", &user_id);
    let response = handler.handle_operation(get_request).await;
    
    if response.success {
        let etag = response.metadata.additional.get("etag").unwrap();
        
        let mut headers = HeaderMap::new();
        headers.insert("ETag", etag.as_str().unwrap().parse().unwrap());
        
        (headers, response.data.unwrap().to_string())
    } else {
        // Handle error
    }
}
```

### Client-Side Conditional Requests

```rust
// In your HTTP handler for PUT/PATCH requests
use axum::{extract::Path, http::HeaderMap};

async fn update_user(
    Path(user_id): Path<String>,
    headers: HeaderMap,
    data: String
) -> impl IntoResponse {
    
    // Get If-Match header from client
    let if_match = headers.get("If-Match")
        .and_then(|v| v.to_str().ok())
        .map(|s| ScimVersion::parse_http_header(s))
        .transpose()
        .unwrap();
    
    let mut update_request = ScimOperationRequest::update(
        "User", 
        &user_id, 
        serde_json::from_str(&data)?
    );
    
    if let Some(version) = if_match {
        update_request = update_request.with_expected_version(version);
    }
    
    let response = handler.handle_operation(update_request).await;
    
    if response.success {
        let new_etag = response.metadata.additional.get("etag").unwrap();
        let mut headers = HeaderMap::new();
        headers.insert("ETag", new_etag.as_str().unwrap().parse().unwrap());
        (StatusCode::OK, headers, response.data.unwrap().to_string())
    } else {
        match response.error_code.as_deref() {
            Some("version_conflict") => {
                (StatusCode::PRECONDITION_FAILED, HeaderMap::new(), response.error.unwrap())
            },
            _ => {
                (StatusCode::BAD_REQUEST, HeaderMap::new(), response.error.unwrap())
            }
        }
    }
}
```

## Provider Implementation

### Using Default Conditional Operations

The ResourceProvider trait provides default implementations that work with any storage backend:

```rust
use scim_server::resource::provider::ResourceProvider;

#[derive(Clone)]
struct MyProvider {
    // Your storage fields
}

impl ResourceProvider for MyProvider {
    type Error = MyError;
    
    // Implement required CRUD methods
    async fn create_resource(&self, resource_type: &str, data: Value, context: &RequestContext) -> Result<Resource, Self::Error> {
        // Your create implementation
    }
    
    async fn get_resource(&self, resource_type: &str, id: &str, context: &RequestContext) -> Result<Option<Resource>, Self::Error> {
        // Your get implementation
    }
    
    // ... other required methods
    
    // Conditional operations work automatically with default implementations!
    // They call your get_resource(), check versions, then call update_resource() or delete_resource()
}
```

### Optimized Conditional Operations

For better performance, implement conditional operations at the storage layer:

```rust
impl ResourceProvider for DatabaseProvider {
    // ... other methods
    
    async fn conditional_update(
        &self,
        resource_type: &str,
        id: &str,
        data: Value,
        expected_version: &ScimVersion,
        context: &RequestContext,
    ) -> Result<ConditionalResult<VersionedResource>, Self::Error> {
        
        // SQL with version checking in a single transaction
        let query = "
            UPDATE resources 
            SET data = $1, version = $2, updated_at = NOW()
            WHERE id = $3 AND resource_type = $4 AND version = $5
            RETURNING data, version
        ";
        
        let new_version = ScimVersion::from_content(data.to_string().as_bytes());
        
        match sqlx::query_as::<_, (Value, String)>(query)
            .bind(&data)
            .bind(new_version.as_str())
            .bind(id)
            .bind(resource_type)
            .bind(expected_version.as_str())
            .fetch_optional(&self.pool)
            .await?
        {
            Some((resource_data, _)) => {
                let resource = Resource::from_json(resource_type.to_string(), resource_data)?;
                Ok(ConditionalResult::Success(VersionedResource::new(resource)))
            },
            None => {
                // Check if resource exists
                if self.resource_exists(resource_type, id, context).await? {
                    // Get current version for conflict information
                    let current = self.get_versioned_resource(resource_type, id, context).await?
                        .unwrap(); // We know it exists
                    
                    Ok(ConditionalResult::VersionMismatch(
                        VersionConflict::standard_message(
                            expected_version.clone(),
                            current.version().clone()
                        )
                    ))
                } else {
                    Ok(ConditionalResult::NotFound)
                }
            }
        }
    }
}
```

## Advanced Scenarios

### Handling Concurrent Modifications

```rust
async fn handle_concurrent_update(
    handler: &ScimOperationHandler<impl ResourceProvider>,
    user_id: &str,
    mut update_data: Value,
    max_retries: usize
) -> Result<(), Box<dyn std::error::Error>> {
    
    for attempt in 0..max_retries {
        // Get current version
        let get_request = ScimOperationRequest::get("User", user_id);
        let get_response = handler.handle_operation(get_request).await;
        
        if !get_response.success {
            return Err("User not found".into());
        }
        
        let current_etag = get_response.metadata.additional.get("etag").unwrap();
        let current_version = ScimVersion::parse_http_header(current_etag.as_str().unwrap())?;
        
        // Merge any changes that happened since we started
        if attempt > 0 {
            update_data = merge_changes(update_data, get_response.data.unwrap())?;
        }
        
        // Attempt conditional update
        let update_request = ScimOperationRequest::update("User", user_id, update_data.clone())
            .with_expected_version(current_version);
        
        let update_response = handler.handle_operation(update_request).await;
        
        if update_response.success {
            println!("Update succeeded on attempt {}", attempt + 1);
            return Ok(());
        }
        
        if update_response.error_code.as_deref() != Some("version_conflict") {
            return Err(update_response.error.unwrap().into());
        }
        
        println!("Version conflict on attempt {}, retrying...", attempt + 1);
        tokio::time::sleep(Duration::from_millis(100 * (attempt + 1) as u64)).await;
    }
    
    Err("Max retries exceeded".into())
}

fn merge_changes(our_changes: Value, current_state: Value) -> Result<Value, Box<dyn std::error::Error>> {
    // Implement your conflict resolution strategy
    // This could be last-writer-wins, field-level merging, or custom business logic
    Ok(our_changes) // Simplified for example
}
```

### Bulk Operations with Version Control

```rust
async fn bulk_update_with_versions(
    handler: &ScimOperationHandler<impl ResourceProvider>,
    updates: Vec<(String, Value)> // (user_id, update_data)
) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    
    let mut successful_updates = Vec::new();
    
    for (user_id, update_data) in updates {
        // Get current version
        let get_request = ScimOperationRequest::get("User", &user_id);
        let get_response = handler.handle_operation(get_request).await;
        
        if !get_response.success {
            println!("Skipping {}: not found", user_id);
            continue;
        }
        
        let current_etag = get_response.metadata.additional.get("etag").unwrap();
        let current_version = ScimVersion::parse_http_header(current_etag.as_str().unwrap())?;
        
        // Conditional update
        let update_request = ScimOperationRequest::update("User", &user_id, update_data)
            .with_expected_version(current_version);
        
        let update_response = handler.handle_operation(update_request).await;
        
        if update_response.success {
            successful_updates.push(user_id);
        } else {
            println!("Failed to update {}: {}", user_id, update_response.error.unwrap());
        }
    }
    
    Ok(successful_updates)
}
```

## Best Practices

### 1. Always Use Conditional Operations in Production

```rust
// ❌ Dangerous - can cause lost updates
let update_request = ScimOperationRequest::update("User", id, data);

// ✅ Safe - uses version control
let update_request = ScimOperationRequest::update("User", id, data)
    .with_expected_version(current_version);
```

### 2. Handle Version Conflicts Gracefully

```rust
match response.error_code.as_deref() {
    Some("version_conflict") => {
        // Refresh data and prompt user to resolve conflicts
        handle_version_conflict(&user_id).await?;
    },
    Some("not_found") => {
        // Resource was deleted
        handle_resource_deleted(&user_id).await?;
    },
    _ => {
        // Other errors
        return Err(response.error.unwrap().into());
    }
}
```

### 3. Implement Retry Logic with Backoff

```rust
async fn retry_with_backoff<F, Fut, T, E>(
    mut operation: F,
    max_retries: usize
) -> Result<T, E>
where
    F: FnMut() -> Fut,
    Fut: Future<Output = Result<T, E>>,
{
    for attempt in 0..max_retries {
        match operation().await {
            Ok(result) => return Ok(result),
            Err(err) if attempt < max_retries - 1 => {
                let delay = Duration::from_millis(100 * 2_u64.pow(attempt as u32));
                tokio::time::sleep(delay).await;
            },
            Err(err) => return Err(err),
        }
    }
    unreachable!()
}
```

### 4. Use Version Information in APIs

```rust
// Include version info in API responses
#[derive(Serialize)]
struct UserResponse {
    #[serde(flatten)]
    user: Value,
    #[serde(rename = "_version")]
    version: String,
    #[serde(rename = "_etag")]
    etag: String,
}

// Accept version in API requests
#[derive(Deserialize)]
struct UpdateUserRequest {
    #[serde(flatten)]
    user_data: Value,
    #[serde(rename = "_expected_version")]
    expected_version: Option<String>,
}
```

### 5. Monitor Version Conflicts

```rust
use log::{warn, info};

if let Some("version_conflict") = response.error_code.as_deref() {
    warn!(
        "Version conflict for user {}: {}",
        user_id,
        response.error.unwrap()
    );
    
    // Track metrics
    metrics::counter!("scim.version_conflicts.total").increment(1);
    metrics::histogram!("scim.version_conflicts.by_resource", "resource_type" => "User").record(1.0);
}
```

## Troubleshooting

### Common Issues

#### 1. Version Conflicts in High-Concurrency Scenarios

**Problem**: Frequent version conflicts when many clients update the same resources.

**Solution**: Implement intelligent retry logic and consider field-level versioning for specific use cases.

```rust
// Use exponential backoff with jitter
let jitter = rand::random::<u64>() % 50;
let delay = Duration::from_millis(100 * 2_u64.pow(attempt as u32) + jitter);
```

#### 2. Performance Impact of Version Checking

**Problem**: Additional database queries for version checking.

**Solution**: Implement conditional operations at the storage layer to minimize round trips.

#### 3. Client Not Providing ETags

**Problem**: Clients performing unconditional updates.

**Solution**: Make ETag handling explicit in your API design and return 428 Precondition Required for critical operations.

```rust
if critical_operation && expected_version.is_none() {
    return Err(ScimError::precondition_required("ETag required for this operation"));
}
```

### Debugging Version Issues

Enable detailed logging to debug version-related issues:

```rust
use log::debug;

debug!(
    "Conditional update: resource={}, id={}, expected={}, current={}",
    resource_type,
    id,
    expected_version.as_str(),
    current_version.as_str()
);
```

### Testing Version Control

```rust
#[tokio::test]
async fn test_concurrent_update_protection() {
    let provider = InMemoryProvider::new();
    let server = ScimServer::new(provider)?;
    let handler = ScimOperationHandler::new(server);
    
    // Create initial resource
    let create_request = ScimOperationRequest::create("User", user_data());
    let create_response = handler.handle_operation(create_request).await;
    let user_id = create_response.metadata.resource_id.unwrap();
    
    // Get initial version
    let get_request = ScimOperationRequest::get("User", &user_id);
    let get_response = handler.handle_operation(get_request).await;
    let version = ScimVersion::parse_http_header(
        get_response.metadata.additional.get("etag").unwrap().as_str().unwrap()
    )?;
    
    // First update succeeds
    let update1 = ScimOperationRequest::update("User", &user_id, update_data_1())
        .with_expected_version(version.clone());
    let response1 = handler.handle_operation(update1).await;
    assert!(response1.success);
    
    // Second update with same version fails
    let update2 = ScimOperationRequest::update("User", &user_id, update_data_2())
        .with_expected_version(version);
    let response2 = handler.handle_operation(update2).await;
    assert!(!response2.success);
    assert_eq!(response2.error_code.as_deref(), Some("version_conflict"));
}
```

## Conclusion

ETag concurrency control in the SCIM Server library provides robust protection against lost updates with minimal configuration. By understanding the core concepts and following the best practices outlined in this guide, you can build reliable, concurrent SCIM applications that maintain data integrity even under high load.

Key takeaways:

1. **Automatic Protection**: All resources include version information by default
2. **Simple API**: Conditional operations are easy to use with clear error handling
3. **HTTP Compliant**: Full RFC 7232 ETag support for web applications
4. **Flexible Implementation**: Works with any storage backend with optimization opportunities
5. **Production Ready**: Built-in concurrency control suitable for enterprise applications

For more examples and advanced usage patterns, see the `examples/etag_concurrency_example.rs` file in the repository.