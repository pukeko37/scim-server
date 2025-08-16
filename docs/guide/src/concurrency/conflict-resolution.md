# Conflict Resolution

When multiple clients attempt to modify the same SCIM resource simultaneously, version conflicts can occur. This guide covers practical strategies for detecting, handling, and resolving these conflicts using the SCIM Server library's ETag-based concurrency control.

## Understanding Conflicts

A version conflict occurs when:

1. Client A reads a resource (gets version V1)
2. Client B reads the same resource (also gets version V1)
3. Client A updates the resource (creates version V2)
4. Client B attempts to update with version V1 (conflict!)

The SCIM Server library detects this scenario and returns a `ConditionalResult::VersionMismatch` instead of silently overwriting Client A's changes.

## Basic Conflict Detection

```rust
use scim_server::{
    providers::StandardResourceProvider,
    storage::InMemoryStorage,
    resource::{RequestContext, ResourceProvider, version::ConditionalResult},
};
use serde_json::json;

async fn demonstrate_conflict() -> Result<(), Box<dyn std::error::Error>> {
    let storage = InMemoryStorage::new();
    let provider = StandardResourceProvider::new(storage);
    let context = RequestContext::with_generated_id();

    // Create a user
    let user_data = json!({
        "userName": "alice.smith",
        "displayName": "Alice Smith",
        "active": true
    });

    let user = provider.create_resource("User", user_data, &context).await?;
    let user_id = user.get_id().unwrap();

    // Both clients get the same initial version
    let client_a_version = provider
        .get_versioned_resource("User", user_id, &context)
        .await?
        .unwrap()
        .version()
        .clone();

    let client_b_version = client_a_version.clone(); // Same version

    // Client A updates successfully
    let client_a_update = json!({
        "userName": "alice.smith",
        "displayName": "Alice Updated by A",
        "active": true
    });

    let result_a = provider.conditional_update(
        "User",
        user_id,
        client_a_update,
        &client_a_version,
        &context
    ).await?;

    assert!(matches!(result_a, ConditionalResult::Success(_)));
    println!("✅ Client A update succeeded");

    // Client B tries to update with stale version
    let client_b_update = json!({
        "userName": "alice.smith",
        "displayName": "Alice Updated by B",
        "active": false
    });

    let result_b = provider.conditional_update(
        "User",
        user_id,
        client_b_update,
        &client_b_version, // This is now stale!
        &context
    ).await?;

    match result_b {
        ConditionalResult::VersionMismatch(conflict) => {
            println!("❌ Client B update failed - conflict detected!");
            println!("Expected version: {}", conflict.expected.to_http_header());
            println!("Current version:  {}", conflict.current.to_http_header());
            println!("Message: {}", conflict.message);
        }
        _ => panic!("Expected version conflict"),
    }

    Ok(())
}
```

## Conflict Resolution Strategies

### 1. Retry with Latest Version (Last-Writer-Wins)

The simplest strategy is to fetch the latest version and retry the operation:

```rust
async fn retry_with_latest_version(
    provider: &StandardResourceProvider<InMemoryStorage>,
    resource_type: &str,
    resource_id: &str,
    update_data: serde_json::Value,
    context: &RequestContext,
    max_retries: usize,
) -> Result<(), Box<dyn std::error::Error>> {
    for attempt in 0..max_retries {
        // Get the current (latest) version
        let versioned = provider
            .get_versioned_resource(resource_type, resource_id, context)
            .await?
            .ok_or("Resource not found")?;

        // Attempt update with current version
        match provider.conditional_update(
            resource_type,
            resource_id,
            update_data.clone(),
            versioned.version(),
            context
        ).await? {
            ConditionalResult::Success(updated) => {
                println!("✅ Update succeeded on attempt {}", attempt + 1);
                return Ok(());
            }
            ConditionalResult::VersionMismatch(_) => {
                if attempt + 1 >= max_retries {
                    return Err(format!("Failed after {} attempts", max_retries).into());
                }
                println!("🔄 Conflict on attempt {}, retrying...", attempt + 1);
                
                // Brief delay before retry
                let delay = std::time::Duration::from_millis(100 * (attempt + 1) as u64);
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

### 2. Merge Changes

For more sophisticated conflict resolution, merge the changes instead of overwriting:

```rust
use serde_json::{Value, Map};

async fn merge_update_strategy(
    provider: &StandardResourceProvider<InMemoryStorage>,
    resource_type: &str,
    resource_id: &str,
    my_changes: serde_json::Value,
    original_version: &scim_server::resource::version::ScimVersion,
    context: &RequestContext,
) -> Result<(), Box<dyn std::error::Error>> {
    // Get the current state
    let current_versioned = provider
        .get_versioned_resource(resource_type, resource_id, context)
        .await?
        .ok_or("Resource not found")?;

    // Check if there's actually a conflict
    if current_versioned.version().matches(original_version) {
        // No conflict - can apply changes directly
        return apply_direct_update(provider, resource_type, resource_id, my_changes, context).await;
    }

    println!("🔀 Conflict detected, attempting merge...");

    // Get the current resource data
    let current_data = current_versioned.resource().to_json()?;
    
    // Perform a simple merge: current data + my changes
    let merged_data = merge_json_objects(current_data, my_changes)?;

    // Attempt update with current version
    match provider.conditional_update(
        resource_type,
        resource_id,
        merged_data,
        current_versioned.version(),
        context
    ).await? {
        ConditionalResult::Success(_) => {
            println!("✅ Merge update succeeded");
            Ok(())
        }
        ConditionalResult::VersionMismatch(_) => {
            println!("❌ Another conflict occurred during merge, retry needed");
            Err("Merge failed due to additional conflict".into())
        }
        ConditionalResult::NotFound => {
            Err("Resource was deleted during merge".into())
        }
    }
}

fn merge_json_objects(mut base: Value, changes: Value) -> Result<Value, Box<dyn std::error::Error>> {
    if let (Some(base_obj), Some(changes_obj)) = (base.as_object_mut(), changes.as_object()) {
        for (key, value) in changes_obj {
            match key.as_str() {
                // Don't overwrite read-only fields
                "id" | "meta" => continue,
                // Merge other fields
                _ => {
                    base_obj.insert(key.clone(), value.clone());
                }
            }
        }
    }
    Ok(base)
}

async fn apply_direct_update(
    provider: &StandardResourceProvider<InMemoryStorage>,
    resource_type: &str,
    resource_id: &str,
    update_data: serde_json::Value,
    context: &RequestContext,
) -> Result<(), Box<dyn std::error::Error>> {
    let versioned = provider
        .get_versioned_resource(resource_type, resource_id, context)
        .await?
        .ok_or("Resource not found")?;

    provider.conditional_update(
        resource_type,
        resource_id,
        update_data,
        versioned.version(),
        context
    ).await?;

    Ok(())
}
```

### 3. Field-Level Conflict Resolution

For fine-grained control, resolve conflicts at the field level:

```rust
use std::collections::HashMap;

async fn field_level_merge(
    provider: &StandardResourceProvider<InMemoryStorage>,
    resource_type: &str,
    resource_id: &str,
    my_changes: serde_json::Value,
    context: &RequestContext,
) -> Result<(), Box<dyn std::error::Error>> {
    // Get current resource
    let current_versioned = provider
        .get_versioned_resource(resource_type, resource_id, context)
        .await?
        .ok_or("Resource not found")?;

    let current_data = current_versioned.resource().to_json()?;
    
    // Perform intelligent field-level merge
    let merged_data = intelligent_merge(current_data, my_changes)?;

    // Apply the merged update
    match provider.conditional_update(
        resource_type,
        resource_id,
        merged_data,
        current_versioned.version(),
        context
    ).await? {
        ConditionalResult::Success(_) => {
            println!("✅ Field-level merge succeeded");
            Ok(())
        }
        ConditionalResult::VersionMismatch(_) => {
            Err("Additional conflict during field merge".into())
        }
        ConditionalResult::NotFound => {
            Err("Resource deleted during merge".into())
        }
    }
}

fn intelligent_merge(
    current: serde_json::Value,
    changes: serde_json::Value,
) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    let mut result = current.clone();
    
    if let (Some(result_obj), Some(changes_obj)) = (result.as_object_mut(), changes.as_object()) {
        for (key, new_value) in changes_obj {
            match key.as_str() {
                // Read-only fields - skip
                "id" | "meta" => continue,
                
                // Additive fields - merge arrays
                "emails" | "phoneNumbers" | "addresses" => {
                    merge_multi_valued_attribute(result_obj, key, new_value)?;
                }
                
                // Simple overwrite for other fields
                _ => {
                    result_obj.insert(key.clone(), new_value.clone());
                }
            }
        }
    }
    
    Ok(result)
}

fn merge_multi_valued_attribute(
    result_obj: &mut Map<String, Value>,
    field_name: &str,
    new_values: &Value,
) -> Result<(), Box<dyn std::error::Error>> {
    if let Some(existing_array) = result_obj.get_mut(field_name) {
        if let (Some(existing_vec), Some(new_vec)) = (existing_array.as_array_mut(), new_values.as_array()) {
            for new_item in new_vec {
                // Add if not already present (simple deduplication)
                if !existing_vec.contains(new_item) {
                    existing_vec.push(new_item.clone());
                }
            }
        }
    } else {
        // Field doesn't exist, add it
        result_obj.insert(field_name.to_string(), new_values.clone());
    }
    
    Ok(())
}
```

### 4. User-Guided Conflict Resolution

For critical conflicts that require human intervention:

```rust
#[derive(Debug)]
pub struct ConflictDetails {
    pub resource_id: String,
    pub field_conflicts: Vec<FieldConflict>,
    pub current_version: String,
    pub expected_version: String,
}

#[derive(Debug)]
pub struct FieldConflict {
    pub field_name: String,
    pub my_value: serde_json::Value,
    pub their_value: serde_json::Value,
    pub base_value: Option<serde_json::Value>,
}

async fn user_guided_resolution(
    provider: &StandardResourceProvider<InMemoryStorage>,
    resource_id: &str,
    my_changes: serde_json::Value,
    original_resource: &serde_json::Value,
    context: &RequestContext,
) -> Result<ConflictDetails, Box<dyn std::error::Error>> {
    // Get current state
    let current_versioned = provider
        .get_versioned_resource("User", resource_id, context)
        .await?
        .ok_or("Resource not found")?;

    let current_data = current_versioned.resource().to_json()?;
    
    // Identify conflicting fields
    let conflicts = identify_field_conflicts(original_resource, &current_data, &my_changes)?;
    
    Ok(ConflictDetails {
        resource_id: resource_id.to_string(),
        field_conflicts: conflicts,
        current_version: current_versioned.version().to_http_header(),
        expected_version: "version-from-client".to_string(), // Would come from client
    })
}

fn identify_field_conflicts(
    original: &serde_json::Value,
    current: &serde_json::Value,
    my_changes: &serde_json::Value,
) -> Result<Vec<FieldConflict>, Box<dyn std::error::Error>> {
    let mut conflicts = Vec::new();
    
    if let (Some(original_obj), Some(current_obj), Some(changes_obj)) = 
        (original.as_object(), current.as_object(), my_changes.as_object()) {
        
        for (field, my_value) in changes_obj {
            let original_value = original_obj.get(field);
            let current_value = current_obj.get(field);
            
            // Check if the field was modified by both parties
            if let (Some(original_val), Some(current_val)) = (original_value, current_value) {
                if original_val != current_val && current_val != my_value {
                    conflicts.push(FieldConflict {
                        field_name: field.clone(),
                        my_value: my_value.clone(),
                        their_value: current_val.clone(),
                        base_value: Some(original_val.clone()),
                    });
                }
            }
        }
    }
    
    Ok(conflicts)
}

// Example of presenting conflicts to user
fn present_conflicts_to_user(conflicts: &ConflictDetails) {
    println!("⚠️ Conflict Resolution Required");
    println!("Resource ID: {}", conflicts.resource_id);
    println!("Expected Version: {}", conflicts.expected_version);
    println!("Current Version: {}", conflicts.current_version);
    println!();
    
    for (i, conflict) in conflicts.field_conflicts.iter().enumerate() {
        println!("Conflict {}:", i + 1);
        println!("  Field: {}", conflict.field_name);
        
        if let Some(base) = &conflict.base_value {
            println!("  Original: {}", base);
        }
        
        println!("  Your change: {}", conflict.my_value);
        println!("  Their change: {}", conflict.their_value);
        println!("  Choose: (1) Keep yours, (2) Keep theirs, (3) Merge");
        println!();
    }
}
```

## Advanced Conflict Resolution

### Retry with Exponential Backoff

```rust
use rand::Rng;

async fn retry_with_backoff_and_jitter(
    provider: &StandardResourceProvider<InMemoryStorage>,
    resource_type: &str,
    resource_id: &str,
    update_data: serde_json::Value,
    context: &RequestContext,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut rng = rand::thread_rng();
    let max_retries = 5;
    let base_delay_ms = 100;
    
    for attempt in 0..max_retries {
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
            ConditionalResult::Success(_) => {
                if attempt > 0 {
                    println!("✅ Update succeeded after {} retries", attempt);
                }
                return Ok(());
            }
            ConditionalResult::VersionMismatch(_) => {
                if attempt + 1 >= max_retries {
                    return Err("Maximum retries exceeded".into());
                }
                
                // Exponential backoff with jitter
                let delay_ms = base_delay_ms * 2_u64.pow(attempt as u32);
                let jitter_ms = rng.gen_range(0..=delay_ms / 2);
                let total_delay = std::time::Duration::from_millis(delay_ms + jitter_ms);
                
                println!("🔄 Retry {} after {:?}", attempt + 1, total_delay);
                tokio::time::sleep(total_delay).await;
            }
            ConditionalResult::NotFound => {
                return Err("Resource was deleted".into());
            }
        }
    }
    
    Err("Should not reach here".into())
}
```

### Conflict Metrics and Monitoring

```rust
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

#[derive(Debug, Default)]
pub struct ConflictMetrics {
    total_attempts: AtomicUsize,
    conflicts: AtomicUsize,
    successful_retries: AtomicUsize,
    failed_retries: AtomicUsize,
}

impl ConflictMetrics {
    pub fn record_attempt(&self) {
        self.total_attempts.fetch_add(1, Ordering::Relaxed);
    }
    
    pub fn record_conflict(&self) {
        self.conflicts.fetch_add(1, Ordering::Relaxed);
    }
    
    pub fn record_successful_retry(&self) {
        self.successful_retries.fetch_add(1, Ordering::Relaxed);
    }
    
    pub fn record_failed_retry(&self) {
        self.failed_retries.fetch_add(1, Ordering::Relaxed);
    }
    
    pub fn conflict_rate(&self) -> f64 {
        let total = self.total_attempts.load(Ordering::Relaxed);
        let conflicts = self.conflicts.load(Ordering::Relaxed);
        
        if total == 0 {
            0.0
        } else {
            conflicts as f64 / total as f64
        }
    }
    
    pub fn print_stats(&self) {
        let total = self.total_attempts.load(Ordering::Relaxed);
        let conflicts = self.conflicts.load(Ordering::Relaxed);
        let successful = self.successful_retries.load(Ordering::Relaxed);
        let failed = self.failed_retries.load(Ordering::Relaxed);
        
        println!("📊 Conflict Statistics:");
        println!("  Total attempts: {}", total);
        println!("  Conflicts: {} ({:.1}%)", conflicts, self.conflict_rate() * 100.0);
        println!("  Successful retries: {}", successful);
        println!("  Failed retries: {}", failed);
    }
}

async fn monitored_conditional_update(
    provider: &StandardResourceProvider<InMemoryStorage>,
    resource_type: &str,
    resource_id: &str,
    update_data: serde_json::Value,
    context: &RequestContext,
    metrics: Arc<ConflictMetrics>,
) -> Result<(), Box<dyn std::error::Error>> {
    metrics.record_attempt();
    
    match retry_with_backoff_and_jitter(provider, resource_type, resource_id, update_data, context).await {
        Ok(()) => {
            // Check if we had to retry
            // This is a simplified example - in practice you'd track this in the retry function
            Ok(())
        }
        Err(e) => {
            if e.to_string().contains("conflict") || e.to_string().contains("Maximum retries") {
                metrics.record_conflict();
                metrics.record_failed_retry();
            }
            Err(e)
        }
    }
}
```

## Error Handling Best Practices

### Structured Error Types

```rust
#[derive(Debug, thiserror::Error)]
pub enum ConflictResolutionError {
    #[error("Resource not found: {resource_id}")]
    ResourceNotFound { resource_id: String },
    
    #[error("Too many conflicts after {attempts} attempts")]
    TooManyConflicts { attempts: usize },
    
    #[error("Merge failed: {reason}")]
    MergeFailed { reason: String },
    
    #[error("User intervention required for {field_count} fields")]
    UserInterventionRequired { field_count: usize },
    
    #[error("Provider error: {source}")]
    ProviderError { source: String },
}

async fn robust_conflict_resolution(
    provider: &StandardResourceProvider<InMemoryStorage>,
    resource_type: &str,
    resource_id: &str,
    update_data: serde_json::Value,
    context: &RequestContext,
) -> Result<(), ConflictResolutionError> {
    const MAX_ATTEMPTS: usize = 3;
    
    for attempt in 0..MAX_ATTEMPTS {
        let versioned = provider
            .get_versioned_resource(resource_type, resource_id, context)
            .await
            .map_err(|e| ConflictResolutionError::ProviderError { 
                source: e.to_string() 
            })?
            .ok_or_else(|| ConflictResolutionError::ResourceNotFound {
                resource_id: resource_id.to_string(),
            })?;

        match provider.conditional_update(
            resource_type,
            resource_id,
            update_data.clone(),
            versioned.version(),
            context
        ).await {
            Ok(ConditionalResult::Success(_)) => return Ok(()),
            Ok(ConditionalResult::VersionMismatch(_)) => {
                if attempt + 1 >= MAX_ATTEMPTS {
                    return Err(ConflictResolutionError::TooManyConflicts { 
                        attempts: MAX_ATTEMPTS 
                    });
                }
                // Continue to next attempt
            }
            Ok(ConditionalResult::NotFound) => {
                return Err(ConflictResolutionError::ResourceNotFound {
                    resource_id: resource_id.to_string(),
                });
            }
            Err(e) => {
                return Err(ConflictResolutionError::ProviderError { 
                    source: e.to_string() 
                });
            }
        }
    }
    
    Err(ConflictResolutionError::TooManyConflicts { 
        attempts: MAX_ATTEMPTS 
    })
}
```

## Best Practices Summary

### 1. Always Handle All ConditionalResult Cases

```rust
// Good: Handle all cases
match result {
    ConditionalResult::Success(resource) => {
        // Handle success
    },
    ConditionalResult::VersionMismatch(conflict) => {
        // Handle conflict appropriately
    },
    ConditionalResult::NotFound => {
        // Handle missing resource
    }
}

// Avoid: Ignoring conflict cases
if let ConditionalResult::Success(resource) = result {
    // This ignores conflicts!
}
```

### 2. Implement Appropriate Retry Logic

- Use exponential backoff to avoid overwhelming the server
- Add jitter to prevent thundering herd problems
- Set reasonable maximum retry limits
- Log conflicts for monitoring

### 3. Choose the Right Resolution Strategy

- **Simple retry**: For non-critical updates where last-writer-wins is acceptable
- **Merge**: For updates that can be safely combined
- **User intervention**: For critical data where conflicts need human review

### 4. Monitor Conflict Rates

- Track conflict frequency to identify hotspots
- High conflict rates may indicate design issues
- Consider redesigning highly contended resources

### 5. Provide Clear Error Messages

```rust
// Good: Informative error message
ConflictResolutionError::TooManyConflicts { attempts: 3 }

// Good: Include context
format!("Failed to update user {} after {} attempts due to version conflicts", 
        user_id, max_attempts)
```

This conflict resolution guide provides practical strategies for handling version conflicts in real-world applications using the SCIM Server library's concurrency features.