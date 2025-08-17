# Advanced Provider Features

This guide covers advanced features and patterns for working with storage and resource providers in the SCIM Server library, including conditional operations, versioning, multi-operation patterns, and performance optimization.

## Conditional Operations and ETags

The SCIM Server provides built-in support for ETag-based optimistic concurrency control through conditional operations.

### ETag Generation

ETags are automatically generated for all resources:

```rust
use scim_server::{
    providers::StandardResourceProvider,
    storage::InMemoryStorage,
    resource::{RequestContext, ResourceProvider},
};

let storage = InMemoryStorage::new();
let provider = StandardResourceProvider::new(storage);
let context = RequestContext::with_generated_id();

// Create a resource - ETag is automatically generated
let user = provider.create_resource(
    "User",
    json!({
        "userName": "alice@example.com",
        "displayName": "Alice Smith"
    }),
    &context,
).await?;

// The resource now has an ETag in its metadata
println!("ETag: {}", user.get_version().unwrap());
```

### Conditional Updates

Use ETags to prevent lost updates in concurrent scenarios:

```rust
use scim_server::resource::version::{ConditionalResult, ScimVersion};

// Get the current resource with its ETag
let current_user = provider.get_resource("User", "user-123", &context)
    .await?
    .ok_or("User not found")?;

let current_etag = current_user.get_version().unwrap();

// Attempt conditional update
let updated_data = json!({
    "id": "user-123",
    "userName": "alice@example.com",
    "displayName": "Alice Updated"
});

match provider.conditional_update(
    "User",
    "user-123", 
    updated_data,
    &ScimVersion::from_etag(current_etag),
    &context,
).await? {
    ConditionalResult::Success(updated_user) => {
        println!("Update successful: {}", updated_user.get_version().unwrap());
    }
    ConditionalResult::Conflict(conflict) => {
        println!("Version conflict: expected {}, got {}", 
                 conflict.expected_version, 
                 conflict.current_version);
        // Handle conflict - retry, merge, or report error
    }
}
```

### Conditional Deletes

Safely delete resources with version checking:

```rust
// Delete only if the version matches
match provider.conditional_delete(
    "User",
    "user-123",
    &ScimVersion::from_etag("W/\"abc123\""),
    &context,
).await? {
    ConditionalResult::Success(was_deleted) => {
        if was_deleted {
            println!("User deleted successfully");
        } else {
            println!("User was already deleted");
        }
    }
    ConditionalResult::Conflict(conflict) => {
        println!("Cannot delete: version mismatch");
    }
}
```

### If-Match and If-None-Match Headers

Handle HTTP conditional headers:

```rust
use scim_server::resource::version::VersionCondition;

// If-Match: update only if ETag matches
let condition = VersionCondition::IfMatch(ScimVersion::from_etag("W/\"abc123\""));

// If-None-Match: update only if ETag doesn't match  
let condition = VersionCondition::IfNoneMatch(ScimVersion::from_etag("W/\"xyz789\""));

// Apply condition to update
let result = provider.conditional_update_with_condition(
    "User",
    "user-123",
    updated_data,
    condition,
    &context,
).await?;
```

## Resource Versioning

### Version Management

Resources automatically track version information:

```rust
use scim_server::resource::version::VersionedResource;

// Create versioned resource
let versioned_user = provider.create_versioned_resource(
    "User",
    user_data,
    &context,
).await?;

println!("Resource version: {}", versioned_user.version);
println!("Created at: {}", versioned_user.resource.get_created().unwrap());
println!("Last modified: {}", versioned_user.resource.get_last_modified().unwrap());

// Get resource with version info
let versioned = provider.get_versioned_resource("User", "user-123", &context).await?;
if let Some(v) = versioned {
    println!("Current version: {}", v.version);
    println!("Resource data: {}", serde_json::to_string_pretty(&v.resource)?);
}
```

### Version History

Track changes over time:

```rust
use scim_server::resource::version::VersionHistory;

// Providers can optionally implement version history
pub struct VersionedStorageProvider<S> {
    inner: S,
    version_store: VersionHistory,
}

impl<S: StorageProvider> VersionedStorageProvider<S> {
    async fn get_resource_history(
        &self,
        resource_type: &str,
        id: &str,
        context: &RequestContext,
    ) -> Result<Vec<VersionedResource>, Self::Error> {
        // Return all versions of the resource
        self.version_store.get_history(resource_type, id, context).await
    }
    
    async fn get_resource_at_version(
        &self,
        resource_type: &str,
        id: &str,
        version: &ScimVersion,
        context: &RequestContext,
    ) -> Result<Option<Resource>, Self::Error> {
        // Get resource at specific version
        self.version_store.get_at_version(resource_type, id, version, context).await
    }
}
```

## Multi-Operation Patterns

> **⚠️ Note**: Bulk operations are not yet implemented. Use these patterns for efficient multi-resource operations.

### Batch Processing

Process multiple operations efficiently:

```rust
use scim_server::{ResourceProvider, RequestContext};
use serde_json::json;

// Process multiple user creations
async fn create_multiple_users(
    provider: &impl ResourceProvider,
    _tenant_id: &str,
    users_data: Vec<serde_json::Value>
) -> Result<Vec<serde_json::Value>, Box<dyn std::error::Error>> {
    let context = RequestContext::new("batch-create".to_string());
    let mut results = Vec::new();
    
    // Sequential processing (safest approach)
    for user_data in users_data {
        let user = provider.create_resource("User", user_data, &context).await?;
        results.push(user);
    }
    
    Ok(results)
}

// Parallel processing (for independent operations)
async fn create_users_parallel(
    provider: &impl ResourceProvider,
    _tenant_id: &str,
    users_data: Vec<serde_json::Value>
) -> Result<Vec<serde_json::Value>, Box<dyn std::error::Error>> {
    let context = RequestContext::new("parallel-create".to_string());
    
    let futures: Vec<_> = users_data.into_iter()
        .map(|data| provider.create_resource("User", data, &context))
        .collect();
    
    let results = try_join_all(futures).await?;
    Ok(results)
}

// Mixed operations with error handling
async fn process_mixed_operations(
    provider: &impl ResourceProvider,
    tenant_id: &str,
) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let context = RequestContext::new("mixed-ops", None);
    let mut results = Vec::new();
    
    // Create a new user
    let user_data = json!({
        "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
        "userName": "alice@example.com",
        "displayName": "Alice Smith"
    });
    
    match provider.create_resource("User", user_data, &context).await {
        Ok(user) => {
            let user_id = user.id().unwrap().to_string();
            results.push(format!("Created user: {}", user_id));
            
            // Update the user
            let update_data = json!({
                "displayName": "Alice Johnson"
            });
            
            match provider.update_resource("User", &user_id, update_data, &context).await {
                Ok(_) => results.push(format!("Updated user: {}", user_id)),
                Err(e) => results.push(format!("Failed to update user {}: {}", user_id, e)),
            }
        }
        Err(e) => results.push(format!("Failed to create user: {}", e)),
    }
    
    Ok(results)
}
```

### Transaction-like Operations

While true ACID transactions aren't part of SCIM, you can implement compensating patterns:

```rust
use scim_server::{ResourceProvider, RequestContext};

struct Operation {
    operation_type: String,
    resource_type: String,
    resource_id: Option<String>,
    data: serde_json::Value,
}

struct CompensatingAction {
    action: String,
    resource_type: String,
    resource_id: String,
}

async fn execute_with_compensation(
    provider: &impl ResourceProvider,
    tenant_id: &str,
    operations: Vec<Operation>
) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let context = RequestContext::new("compensating-ops", None);
    let mut completed = Vec::new();
    let mut compensations = Vec::new();
    
    for op in operations {
        let result = match op.operation_type.as_str() {
            "CREATE" => {
                match provider.create_resource(&op.resource_type, op.data, &context).await {
                    Ok(resource) => {
                        let id = resource.id().unwrap().to_string();
                        compensations.push(CompensatingAction {
                            action: "DELETE".to_string(),
                            resource_type: op.resource_type.clone(),
                            resource_id: id.clone(),
                        });
                        Ok(format!("Created {}: {}", op.resource_type, id))
                    }
                    Err(e) => Err(e.into())
                }
            }
            "UPDATE" => {
                let id = op.resource_id.unwrap();
                match provider.update_resource(&op.resource_type, &id, op.data, &context).await {
                    Ok(_) => Ok(format!("Updated {}: {}", op.resource_type, id)),
                    Err(e) => Err(e.into())
                }
            }
            _ => Err("Unsupported operation".into())
        };
        
        match result {
            Ok(msg) => completed.push(msg),
            Err(e) => {
                // Rollback completed operations
                for compensation in compensations.iter().rev() {
                    if compensation.action == "DELETE" {
                        let _ = provider.delete_resource(
                            &compensation.resource_type, 
                            &compensation.resource_id, 
                            &context
                        ).await;
                    }
                }
                return Err(e);
            }
        }
    }
    
    Ok(completed)
}
```

### Efficient Storage Patterns

Optimize storage operations for multiple resources:

```rust
use std::collections::HashMap;

// Batch retrieval pattern
async fn get_multiple_users_by_ids(
    provider: &impl ResourceProvider,
    tenant_id: &str,
    user_ids: Vec<String>
) -> Result<HashMap<String, ScimUser>, Box<dyn std::error::Error>> {
    let context = RequestContext::new("batch-get", None);
    let mut users = HashMap::new();
    
    // For now, individual requests (until batch APIs are implemented)
    for user_id in user_ids {
        match provider.get_resource("User", &user_id, &context).await {
            Ok(Some(user)) => {
                users.insert(user_id, user);
            }
            Ok(None) => {
                // User not found, skip
            }
            Err(e) => {
                eprintln!("Failed to get user {}: {}", user_id, e);
                // Continue with other users
            }
        }
    }
    
    Ok(users)
}
```

## Performance Optimization

### Connection Pooling

Optimize database connections:

```rust
use sqlx::{Pool, Postgres};
use std::time::Duration;

pub struct OptimizedPostgresStorage {
    pool: Pool<Postgres>,
}

impl OptimizedPostgresStorage {
    pub async fn new(database_url: &str) -> Result<Self, sqlx::Error> {
        let pool = sqlx::postgres::PgPoolOptions::new()
            .max_connections(100)
            .min_connections(10)
            .max_lifetime(Duration::from_secs(1800))  // 30 minutes
            .idle_timeout(Duration::from_secs(600))   // 10 minutes
            .acquire_timeout(Duration::from_secs(30))
            .test_before_acquire(true)
            .connect(database_url)
            .await?;
            
        Ok(Self { pool })
    }
}
```

### Caching Layer

Add caching to storage providers:

```rust
use tokio::sync::RwLock;
use std::collections::HashMap;
use std::time::{Duration, Instant};

pub struct CachedStorage<S> {
    inner: S,
    cache: Arc<RwLock<HashMap<String, CachedEntry>>>,
    ttl: Duration,
}

struct CachedEntry {
    value: Value,
    inserted_at: Instant,
}

impl<S: StorageProvider> CachedStorage<S> {
    pub fn new(inner: S, ttl: Duration) -> Self {
        Self {
            inner,
            cache: Arc::new(RwLock::new(HashMap::new())),
            ttl,
        }
    }
    
    fn cache_key(&self, key: &StorageKey) -> String {
        format!("{}/{}/{}", key.tenant_id(), key.resource_type(), key.resource_id())
    }
    
    async fn cleanup_expired(&self) {
        let mut cache = self.cache.write().await;
        let now = Instant::now();
        cache.retain(|_, entry| now.duration_since(entry.inserted_at) < self.ttl);
    }
}

impl<S: StorageProvider> StorageProvider for CachedStorage<S> {
    type Error = S::Error;
    
    async fn get(&self, key: StorageKey) -> Result<Option<Value>, Self::Error> {
        let cache_key = self.cache_key(&key);
        
        // Check cache first
        {
            let cache = self.cache.read().await;
            if let Some(entry) = cache.get(&cache_key) {
                let age = Instant::now().duration_since(entry.inserted_at);
                if age < self.ttl {
                    return Ok(Some(entry.value.clone()));
                }
            }
        }
        
        // Cache miss - get from storage
        let result = self.inner.get(key).await?;
        
        // Update cache
        if let Some(ref value) = result {
            let mut cache = self.cache.write().await;
            cache.insert(cache_key, CachedEntry {
                value: value.clone(),
                inserted_at: Instant::now(),
            });
        }
        
        Ok(result)
    }
    
    async fn put(&self, key: StorageKey, data: Value) -> Result<Value, Self::Error> {
        let result = self.inner.put(key.clone(), data).await?;
        
        // Update cache
        let cache_key = self.cache_key(&key);
        let mut cache = self.cache.write().await;
        cache.insert(cache_key, CachedEntry {
            value: result.clone(),
            inserted_at: Instant::now(),
        });
        
        Ok(result)
    }
    
    async fn delete(&self, key: StorageKey) -> Result<bool, Self::Error> {
        let result = self.inner.delete(key.clone()).await?;
        
        // Remove from cache
        let cache_key = self.cache_key(&key);
        let mut cache = self.cache.write().await;
        cache.remove(&cache_key);
        
        Ok(result)
    }
    
    // Implement other methods...
}
```

### Indexing for Search

Optimize attribute searches:

```rust
use std::collections::BTreeMap;

pub struct IndexedStorage<S> {
    inner: S,
    // Index: (tenant, resource_type, attribute) -> (value -> resource_ids)
    attribute_index: Arc<RwLock<BTreeMap<String, BTreeMap<String, HashSet<String>>>>>,
}

impl<S: StorageProvider> IndexedStorage<S> {
    pub fn new(inner: S) -> Self {
        Self {
            inner,
            attribute_index: Arc::new(RwLock::new(BTreeMap::new())),
        }
    }
    
    fn index_key(&self, prefix: &StoragePrefix, attribute: &str) -> String {
        format!("{}/{}#{}", prefix.tenant_id(), prefix.resource_type(), attribute)
    }
    
    async fn update_index(&self, key: &StorageKey, data: &Value) {
        let mut index = self.attribute_index.write().await;
        
        // Index common searchable attributes
        let searchable_attributes = ["userName", "displayName", "email.value"];
        
        for attr in &searchable_attributes {
            if let Some(attr_value) = self.extract_attribute_value(data, attr) {
                let index_key = self.index_key(
                    &StorageKey::prefix(key.tenant_id(), key.resource_type()),
                    attr
                );
                
                let value_index = index.entry(index_key).or_insert_with(BTreeMap::new);
                let resource_set = value_index.entry(attr_value).or_insert_with(HashSet::new);
                resource_set.insert(key.resource_id().to_string());
            }
        }
    }
    
    fn extract_attribute_value(&self, data: &Value, attribute: &str) -> Option<String> {
        // Simple attribute extraction - can be enhanced for nested attributes
        data.get(attribute)?.as_str().map(|s| s.to_string())
    }
}

impl<S: StorageProvider> StorageProvider for IndexedStorage<S> {
    type Error = S::Error;
    
    async fn find_by_attribute(
        &self,
        prefix: StoragePrefix,
        attribute: &str,
        value: &str,
    ) -> Result<Vec<(StorageKey, Value)>, Self::Error> {
        let index_key = self.index_key(&prefix, attribute);
        
        // Try index first
        {
            let index = self.attribute_index.read().await;
            if let Some(value_index) = index.get(&index_key) {
                if let Some(resource_ids) = value_index.get(value) {
                    // Found in index - get the resources
                    let mut results = Vec::new();
                    for resource_id in resource_ids {
                        let key = StorageKey::new(
                            prefix.tenant_id(),
                            prefix.resource_type(),
                            resource_id
                        );
                        if let Some(data) = self.inner.get(key.clone()).await? {
                            results.push((key, data));
                        }
                    }
                    return Ok(results);
                }
            }
        }
        
        // Fallback to full scan
        self.inner.find_by_attribute(prefix, attribute, value).await
    }
    
    async fn put(&self, key: StorageKey, data: Value) -> Result<Value, Self::Error> {
        let result = self.inner.put(key.clone(), data.clone()).await?;
        
        // Update indexes
        self.update_index(&key, &result).await;
        
        Ok(result)
    }
    
    // Implement other methods with index maintenance...
}
```

## Custom Validation

### Schema Validation

Add custom validation logic:

```rust
use serde_json::Value;

pub trait ResourceValidator: Send + Sync {
    type Error: std::error::Error + Send + Sync + 'static;
    
    async fn validate_resource(
        &self,
        resource_type: &str,
        data: &Value,
        context: &RequestContext,
    ) -> Result<(), Self::Error>;
    
    async fn validate_update(
        &self,
        resource_type: &str,
        id: &str,
        current: &Value,
        updated: &Value,
        context: &RequestContext,
    ) -> Result<(), Self::Error>;
}

pub struct SchemaValidator {
    schemas: HashMap<String, ResourceSchema>,
}

impl ResourceValidator for SchemaValidator {
    type Error = ValidationError;
    
    async fn validate_resource(
        &self,
        resource_type: &str,
        data: &Value,
        context: &RequestContext,
    ) -> Result<(), Self::Error> {
        let schema = self.schemas.get(resource_type)
            .ok_or_else(|| ValidationError::UnknownResourceType(resource_type.to_string()))?;
        
        // Validate required fields
        for required_field in &schema.required_fields {
            if !data.get(required_field).is_some() {
                return Err(ValidationError::MissingRequiredField(required_field.clone()));
            }
        }
        
        // Validate field types and constraints
        for (field_name, field_schema) in &schema.fields {
            if let Some(field_value) = data.get(field_name) {
                self.validate_field(field_value, field_schema)?;
            }
        }
        
        // Custom business rules
        self.validate_business_rules(resource_type, data, context).await?;
        
        Ok(())
    }
    
    async fn validate_update(
        &self,
        resource_type: &str,
        id: &str,
        current: &Value,
        updated: &Value,
        context: &RequestContext,
    ) -> Result<(), Self::Error> {
        // First validate the updated resource
        self.validate_resource(resource_type, updated, context).await?;
        
        // Check immutable fields
        let schema = self.schemas.get(resource_type).unwrap();
        for immutable_field in &schema.immutable_fields {
            let current_value = current.get(immutable_field);
            let updated_value = updated.get(immutable_field);
            
            if current_value != updated_value {
                return Err(ValidationError::ImmutableFieldModified(immutable_field.clone()));
            }
        }
        
        Ok(())
    }
}

pub struct ValidatingResourceProvider<P, V> {
    inner: P,
    validator: V,
}

impl<P: ResourceProvider, V: ResourceValidator> ResourceProvider for ValidatingResourceProvider<P, V> {
    type Error = CombinedError<P::Error, V::Error>;
    
    async fn create_resource(
        &self,
        resource_type: &str,
        data: Value,
        context: &RequestContext,
    ) -> Result<Resource, Self::Error> {
        // Validate before creation
        self.validator.validate_resource(resource_type, &data, context)
            .await
            .map_err(CombinedError::ValidationError)?;
        
        // Delegate to inner provider
        self.inner.create_resource(resource_type, data, context)
            .await
            .map_err(CombinedError::ProviderError)
    }
    
    async fn update_resource(
        &self,
        resource_type: &str,
        id: &str,
        data: Value,
        context: &RequestContext,
    ) -> Result<Resource, Self::Error> {
        // Get current resource for validation
        let current = self.inner.get_resource(resource_type, id, context)
            .await
            .map_err(CombinedError::ProviderError)?
            .ok_or_else(|| CombinedError::ProviderError(/* NotFound error */))?;
        
        // Validate the update
        self.validator.validate_update(resource_type, id, current.data(), &data, context)
            .await
            .map_err(CombinedError::ValidationError)?;
        
        // Perform update
        self.inner.update_resource(resource_type, id, data, context)
            .await
            .map_err(CombinedError::ProviderError)
    }
    
    // Implement other methods...
}
```

## Monitoring and Metrics

### Instrumentation

Add comprehensive monitoring:

```rust
use tracing::{info, warn, error, instrument, Span};
use std::time::Instant;

pub struct InstrumentedProvider<P> {
    inner: P,
    metrics: Arc<ProviderMetrics>,
}

pub struct ProviderMetrics {
    pub operation_counter: metrics::Counter,
    pub operation_duration: metrics::Histogram,
    pub error_counter: metrics::Counter,
    pub active_operations: metrics::Gauge,
}

impl<P: ResourceProvider> InstrumentedProvider<P> {
    pub fn new(inner: P, metrics: Arc<ProviderMetrics>) -> Self {
        Self { inner, metrics }
    }
}

impl<P: ResourceProvider> ResourceProvider for InstrumentedProvider<P> {
    type Error = P::Error;
    
    #[instrument(skip(self, data, context), fields(
        resource_type = resource_type,
        tenant_id = context.tenant_context.as_ref().map(|t| t.tenant_id.as_str()),
        operation = "create"
    ))]
    async fn create_resource(
        &self,
        resource_type: &str,
        data: Value,
        context: &RequestContext,
    ) -> Result<Resource, Self::Error> {
        let start = Instant::now();
        let _guard = self.metrics.active_operations.increment();
        
        info!("Creating resource");
        
        let result = self.inner.create_resource(resource_type, data, context).await;
        
        let duration = start.elapsed();
        
        match &result {
            Ok(resource) => {
                info!(
                    resource_id = resource.get_id().unwrap_or("unknown"),
                    duration_ms = duration.as_millis(),
                    "Resource created successfully"
                );
                
                self.metrics.operation_counter
                    .with_labels(&[("operation", "create"), ("status", "success")])
                    .increment();
            }
            Err(e) => {
                error!(
                    error = %e,
                    duration_ms = duration.as_millis(),
                    "Failed to create resource"
                );
                
                self.metrics.operation_counter
                    .with_labels(&[("operation", "create"), ("status", "error")])
                    .increment();
                    
                self.metrics.error_counter
                    .with_labels(&[("operation", "create")])
                    .increment();
            }
        }
        
        self.metrics.operation_duration
            .with_labels(&[("operation", "create")])
            .observe(duration.as_secs_f64());
        
        result
    }
    
    // Similar instrumentation for other methods...
}
```

### Health Checks

Implement comprehensive health monitoring:

```rust
use serde::{Serialize, Deserialize};
use std::time::{Duration, SystemTime};

#[derive(Debug, Serialize, Deserialize)]
pub struct HealthStatus {
    pub status: HealthState,
    pub version: String,
    pub uptime: Duration,
    pub checks: Vec<HealthCheck>,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum HealthState {
    Healthy,
    Degraded,
    Unhealthy,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct HealthCheck {
    pub name: String,
    pub status: HealthState,
    pub duration: Duration,
    pub message: Option<String>,
}

pub trait HealthProvider {
    async fn health_check(&self) -> Result<HealthStatus, Box<dyn std::error::Error>>;
}

impl<S: StorageProvider> HealthProvider for StandardResourceProvider<S> {
    async fn health_check(&self) -> Result<HealthStatus, Box<dyn std::error::Error>> {
        let start_time = SystemTime::now();
        let mut checks = Vec::new();
        
        // Check storage connectivity
        let storage_check_start = Instant::now();
        let storage_status = match self.test_storage_connectivity().await {
            Ok(_) => HealthState::Healthy,
            Err(e) => {
                checks.push(HealthCheck {
                    name: "storage".to_string(),
                    status: HealthState::Unhealthy,
                    duration: storage_check_start.elapsed(),
                    message: Some(e.to_string()),
                });
                HealthState::Unhealthy
            }
        };
        
        if matches!(storage_status, HealthState::Healthy) {
            checks.push(HealthCheck {
                name: "storage".to_string(),
                status: HealthState::Healthy,
                duration: storage_check_start.elapsed(),
                message: None,
            });
        }
        
        // Check resource operations
        let ops_check_start = Instant::now();
        let ops_status = match self.test_basic_operations().await {
            Ok(_) => HealthState::Healthy,
            Err(e) => {
                checks.push(HealthCheck {
                    name: "operations".to_string(),
                    status: HealthState::Unhealthy,
                    duration: ops_check_start.elapsed(),
                    message: Some(e.to_string()),
                });
                HealthState::Unhealthy
            }
        };
        
        if matches!(ops_status, HealthState::Healthy) {
            checks.push(HealthCheck {
                name: "operations".to_string(),
                status: HealthState::Healthy,
                duration: ops_check_start.elapsed(),
                message: None,
            });
        }
        
        // Determine overall status
        let overall_status = if checks.iter().any(|c| matches!(c.status, HealthState::Unhealthy)) {
            HealthState::Unhealthy
        } else if checks.iter().any(|c| matches!(c.status, HealthState::Degraded)) {
            HealthState::Degraded
        } else {
            HealthState::Healthy
        };
        
        Ok(HealthStatus {
            status: overall_status,
            version: env!("CARGO_PKG_VERSION").to_string(),
            uptime: start_time.elapsed().unwrap_or(Duration::ZERO),
            checks,
        })
    }
}
```

## Best Practices

### Error Handling

Implement comprehensive error handling:

```rust
#[derive(Debug, thiserror::Error)]
pub enum AdvancedProviderError {
    #[error("Storage error: {0}")]
    Storage(#[from] StorageError),
    
    #[error("Validation error: {0}")]
    Validation(#[from] ValidationError),
    
    #[error("Version conflict: expected {expected}, got {current}")]
    VersionConflict { expected: String, current: String },
    
    #[error("Rate limit exceeded: {limit} requests per {window:?}")]
    RateLimitExceeded { limit: u32, window: Duration },
    
    #[error("Quota exceeded: {current}/{limit} resources")]
    QuotaExceeded { current: usize, limit: usize },
    
    #[error("Circuit breaker open: {service}")]
    CircuitBreakerOpen { service: String },
}

// Implement recovery strategies
impl AdvancedProviderError {
    pub fn is_retryable(&self) -> bool {
        matches!(self, 
            Self::Storage(StorageError::Internal(_)) |
            Self::CircuitBreakerOpen { .. }
        )
    }
    
    pub fn retry_delay(&self) -> Option<Duration> {
        match self {
            Self::RateLimitExceeded { window, .. } => Some(*window),
            Self::CircuitBreakerOpen { .. } => Some(Duration::from_secs(5)),
            _ => None,
        }
    }
}
```

### Configuration

Use structured configuration:

```rust
#[derive(Debug, Deserialize)]
pub struct AdvancedProviderConfig {
    pub storage: StorageConfig,
    pub caching: Option<CacheConfig>,
    pub validation: ValidationConfig,
    pub performance: PerformanceConfig,
    pub monitoring: MonitoringConfig,
}

#[derive(Debug, Deserialize)]
pub struct CacheConfig {
    pub enabled: bool,
    pub ttl_seconds: u64,
    pub max_entries: usize,
}

#[derive(Debug, Deserialize)]
pub struct PerformanceConfig {
    pub bulk_batch_size: usize,
    pub connection_pool_size: u32,
    pub query_timeout_seconds: u64,
}

#[derive(Debug, Deserialize)]
pub struct MonitoringConfig {
    pub enable_metrics: bool,
    pub enable_tracing: bool,
    pub health_check_interval_seconds: u64,
}

// Factory function
pub async fn create_advanced_provider(
    config: AdvancedProviderConfig,
) -> Result<Box<dyn ResourceProvider<Error = AdvancedProviderError>>, ConfigError> {
    // Create storage layer
    let storage = create_storage_provider(&config.storage).await?;
    
    // Add caching if configured
    let storage: Box<dyn StorageProvider<Error = StorageError>> = if let Some(cache_config) = config.caching {
        if cache_config.enabled {
            Box::new(CachedStorage::new(
                storage,
                Duration::from_secs(cache_config.ttl_seconds),
            ))
        } else {
            storage
        }
    } else {
        storage
    };
    
    // Create resource provider
    let provider = StandardResourceProvider::new(storage);
    
    // Add validation layer
    let validator = create_validator(&config.validation)?;
    let provider = ValidatingResourceProvider::new(provider, validator);