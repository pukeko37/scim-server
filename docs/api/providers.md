# Resource Providers API Reference

This document provides comprehensive API documentation for the SCIM Server resource provider system. Resource providers abstract storage backends and enable pluggable storage implementations.

## Table of Contents

- [Overview](#overview)
- [ResourceProvider Trait](#resourceprovider-trait)
- [Built-in Providers](#built-in-providers)
- [Provider Configuration](#provider-configuration)
- [Custom Provider Implementation](#custom-provider-implementation)
- [Error Handling](#error-handling)
- [Testing Providers](#testing-providers)
- [Performance Considerations](#performance-considerations)

## Overview

The provider system enables the SCIM server to work with different storage backends through a common interface. This abstraction allows you to:

- Switch between storage backends without changing business logic
- Implement custom storage solutions for specific requirements
- Test with in-memory providers during development
- Scale with high-performance database providers in production

### Provider Architecture

```
┌─────────────────────────────────────────────────────────┐
│                 SCIM Resource Handlers                  │
├─────────────────────────────────────────────────────────┤
│              ResourceProvider Trait                     │
│                 (Common Interface)                      │
├─────────────────────────────────────────────────────────┤
│StandardResourceProvider│  DatabaseProvider  │ CustomProvider │
│                        │                    │                │
│  ┌───────────────────┐ │ ┌────────────────┐ │ ┌────────────┐ │
│  │  InMemoryStorage  │ │ │  SQL Database  │ │ │ External   │ │
│  │     Backend       │ │ │  Connection    │ │ │ API Client │ │
│  └───────────────┘ │ └────────────────┘ │ └────────────┘ │
└─────────────────────────────────────────────────────────┘
```

## ResourceProvider Trait

The core provider interface that all storage backends must implement:

```rust
use async_trait::async_trait;
use scim_server::resource::{Resource, ResourceType};
use scim_server::resource::value_objects::ResourceId;
use scim_server::error::Result;

#[async_trait]
pub trait ResourceProvider: Send + Sync + Clone {
    /// Create a new resource in the storage backend
    async fn create_resource(&self, resource: Resource) -> Result<Resource>;
    
    /// Retrieve a resource by its ID
    async fn get_resource(&self, id: &ResourceId) -> Result<Option<Resource>>;
    
    /// Update an existing resource (full replacement)
    async fn update_resource(&self, resource: Resource) -> Result<Resource>;
    
    /// Partially update a resource using PATCH operations
    async fn patch_resource(&self, id: &ResourceId, patch: PatchRequest) -> Result<Resource>;
    
    /// Delete a resource by its ID
    async fn delete_resource(&self, id: &ResourceId) -> Result<()>;
    
    /// List all resources of a specific type
    async fn list_resources(&self, resource_type: ResourceType) -> Result<Vec<Resource>>;
    
    /// Search resources using SCIM filter expressions
    async fn search_resources(&self, query: &SearchQuery) -> Result<SearchResult>;
    
    /// Perform bulk operations
    async fn bulk_operations(&self, operations: BulkRequest) -> Result<BulkResponse>;
    
    /// Check the health status of the provider
    async fn health_check(&self) -> Result<HealthStatus>;
    
    /// Get provider-specific statistics
    async fn get_statistics(&self) -> Result<ProviderStatistics>;
    
    /// Initialize the provider (called at startup)
    async fn initialize(&self) -> Result<()> {
        Ok(())
    }
    
    /// Shutdown the provider gracefully
    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }
}
```

### Core Operations

#### Creating Resources

```rust
async fn create_resource(&self, resource: Resource) -> Result<Resource>;
```

**Parameters:**
- `resource`: The resource to create. Must have a valid ID and pass schema validation.

**Returns:**
- `Ok(Resource)`: The created resource, potentially with server-generated metadata
- `Err(ScimError)`: If creation fails due to conflicts, validation errors, or storage issues

**Example:**
```rust
use scim_server::resource::{Resource, ResourceBuilder};
use scim_server::resource::value_objects::{ResourceId, UserName};

let resource = ResourceBuilder::new()
    .id(ResourceId::new("user-123")?)
    .user_name(UserName::new("john.doe")?)
    .display_name("John Doe")
    .build()?;

let created = provider.create_resource(resource).await?;
println!("Created resource with ID: {}", created.id());
```

#### Retrieving Resources

```rust
async fn get_resource(&self, id: &ResourceId) -> Result<Option<Resource>>;
```

**Parameters:**
- `id`: The unique identifier of the resource to retrieve

**Returns:**
- `Ok(Some(Resource))`: The resource if found
- `Ok(None)`: If no resource exists with the given ID
- `Err(ScimError)`: If retrieval fails due to storage issues

**Example:**
```rust
let id = ResourceId::new("user-123")?;
match provider.get_resource(&id).await? {
    Some(resource) => println!("Found user: {}", resource.display_name().unwrap_or("N/A")),
    None => println!("User not found"),
}
```

#### Updating Resources

```rust
async fn update_resource(&self, resource: Resource) -> Result<Resource>;
```

**Parameters:**
- `resource`: The updated resource. Must include the resource ID.

**Returns:**
- `Ok(Resource)`: The updated resource with current metadata
- `Err(ScimError)`: If update fails due to not found, validation, or storage issues

**Example:**
```rust
let mut resource = provider.get_resource(&id).await?.unwrap();
resource.set_display_name(Some("Updated Name"));
let updated = provider.update_resource(resource).await?;
```

#### Searching Resources

```rust
async fn search_resources(&self, query: &SearchQuery) -> Result<SearchResult>;
```

**Parameters:**
- `query`: Search parameters including filters, sorting, and pagination

**Returns:**
- `Ok(SearchResult)`: Paginated search results with metadata
- `Err(ScimError)`: If search fails due to invalid filters or storage issues

**Example:**
```rust
use scim_server::search::{SearchQuery, FilterExpression};

let query = SearchQuery::builder()
    .filter(FilterExpression::parse(r#"userName eq "john.doe""#)?)
    .start_index(1)
    .count(10)
    .build();

let results = provider.search_resources(&query).await?;
println!("Found {} resources", results.total_results);
```

## Built-in Providers

### StandardResourceProvider with InMemoryStorage

The current recommended approach using `StandardResourceProvider` with `InMemoryStorage` backend:

```rust
use scim_server::{
    providers::StandardResourceProvider,
    storage::InMemoryStorage,
    RequestContext,
    resource::provider::ResourceProvider,
};

// Basic usage
let storage = InMemoryStorage::new();
let provider = StandardResourceProvider::new(storage);

// Create request context
let context = RequestContext::new("example-request".to_string());

// Use the provider
let user_data = serde_json::json!({
    "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
    "userName": "john.doe@example.com"
});

let user = provider.create_resource("User", user_data, &context).await?;
```

**Features:**
- ✅ Fast read/write operations
- ✅ Full SCIM query support
- ✅ Thread-safe concurrent access
- ✅ Type-safe resource operations
- ✅ Standardized provider interface
- ❌ Data not persisted across restarts
- ❌ Memory usage grows with data size

**Use Cases:**
- Development and testing
- Small datasets that fit in memory
- Temporary or cache-like storage
- Unit test isolation
- Quick prototyping

**Migration from InMemoryProvider:**

```rust
// Old (deprecated)
use scim_server::providers::InMemoryProvider;
let provider = InMemoryProvider::new();

// New (current)
use scim_server::{
    providers::StandardResourceProvider,
    storage::InMemoryStorage,
};
let storage = InMemoryStorage::new();
let provider = StandardResourceProvider::new(storage);
```

### DatabaseProvider

A SQL database-backed provider for production use:

```rust
use scim_server::providers::DatabaseProvider;

// PostgreSQL
let provider = DatabaseProvider::new(
    "postgresql://user:password@localhost/scim"
).await?;

// SQLite
let provider = DatabaseProvider::new(
    "sqlite:scim.db"
).await?;

// With configuration
let provider = DatabaseProvider::builder()
    .connection_string("postgresql://user:password@localhost/scim")
    .max_connections(20)
    .connection_timeout(Duration::from_secs(10))
    .enable_prepared_statements(true)
    .build()
    .await?;
```

**Features:**
- ✅ Persistent storage
- ✅ ACID transactions
- ✅ Efficient querying with indexes
- ✅ Connection pooling
- ✅ Migration support
- ⚠️ Requires database setup
- ⚠️ Network latency for remote databases

**Database Schema:**

The provider automatically creates the necessary tables:

```sql
-- Resources table
CREATE TABLE scim_resources (
    id VARCHAR(255) PRIMARY KEY,
    resource_type VARCHAR(50) NOT NULL,
    tenant_id VARCHAR(255),
    version INTEGER NOT NULL DEFAULT 1,
    data JSONB NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    
    INDEX idx_resource_type (resource_type),
    INDEX idx_tenant_id (tenant_id),
    INDEX idx_created_at (created_at)
);

-- Schema definitions table
CREATE TABLE scim_schemas (
    uri VARCHAR(255) PRIMARY KEY,
    definition JSONB NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);
```

## Provider Configuration

### Configuration Builder Pattern

All providers support a builder pattern for configuration:

```rust
use scim_server::providers::{ProviderConfig, DatabaseProvider};

let config = ProviderConfig::builder()
    .connection_string("postgresql://localhost/scim")
    .max_connections(20)
    .connection_timeout(Duration::from_secs(30))
    .query_timeout(Duration::from_secs(10))
    .enable_connection_pooling(true)
    .enable_prepared_statements(true)
    .enable_transactions(true)
    .retry_attempts(3)
    .retry_delay(Duration::from_millis(100))
    .build()?;

let provider = DatabaseProvider::with_config(config).await?;
```

### Environment-Based Configuration

```rust
use std::env;

fn create_provider_from_env() -> Result<Box<dyn ResourceProvider>> {
    let provider_type = env::var("SCIM_PROVIDER_TYPE")?;
    
    match provider_type.as_str() {
        "memory" => {
            let storage = InMemoryStorage::new();
            Ok(Box::new(StandardResourceProvider::new(storage)))
        }
        "database" => {
            let db_url = env::var("DATABASE_URL")?;
            let provider = DatabaseProvider::new(&db_url).await?;
            Ok(Box::new(provider))
        }
        "redis" => {
            let redis_url = env::var("REDIS_URL")?;
            let provider = RedisProvider::new(&redis_url).await?;
            Ok(Box::new(provider))
        }
        _ => Err(format!("Unknown provider type: {}", provider_type).into())
    }
}
```

## Custom Provider Implementation

### Implementing ResourceProvider

Here's a complete example of implementing a custom provider:

```rust
use async_trait::async_trait;
use scim_server::providers::ResourceProvider;
use scim_server::resource::{Resource, ResourceType};
use scim_server::resource::value_objects::ResourceId;
use scim_server::error::{Result, ScimError};
use std::collections::HashMap;
use tokio::sync::RwLock;

pub struct CustomApiProvider {
    client: reqwest::Client,
    base_url: String,
    api_key: String,
    cache: RwLock<HashMap<ResourceId, Resource>>,
}

impl CustomApiProvider {
    pub fn new(base_url: String, api_key: String) -> Self {
        Self {
            client: reqwest::Client::new(),
            base_url,
            api_key,
            cache: RwLock::new(HashMap::new()),
        }
    }
    
    async fn make_request<T>(&self, method: &str, path: &str, body: Option<T>) -> Result<serde_json::Value>
    where
        T: serde::Serialize,
    {
        let url = format!("{}/{}", self.base_url, path);
        let mut request = self.client
            .request(method.parse().unwrap(), &url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json");
        
        if let Some(body) = body {
            request = request.json(&body);
        }
        
        let response = request.send().await
            .map_err(|e| ScimError::ProviderError { 
                source: Box::new(e) 
            })?;
        
        if !response.status().is_success() {
            return Err(ScimError::ProviderError {
                source: Box::new(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    format!("API request failed with status: {}", response.status())
                ))
            });
        }
        
        response.json().await
            .map_err(|e| ScimError::ProviderError { 
                source: Box::new(e) 
            })
    }
}

#[async_trait]
impl ResourceProvider for CustomApiProvider {
    async fn create_resource(&self, resource: Resource) -> Result<Resource> {
        // Convert resource to API format
        let api_data = serde_json::to_value(&resource)?;
        
        // Make API call
        let response = self.make_request(
            "POST", 
            &format!("resources/{}", resource.resource_type()), 
            Some(api_data)
        ).await?;
        
        // Convert response back to Resource
        let created_resource: Resource = serde_json::from_value(response)?;
        
        // Cache the created resource
        let mut cache = self.cache.write().await;
        cache.insert(created_resource.id().clone(), created_resource.clone());
        
        Ok(created_resource)
    }
    
    async fn get_resource(&self, id: &ResourceId) -> Result<Option<Resource>> {
        // Check cache first
        {
            let cache = self.cache.read().await;
            if let Some(resource) = cache.get(id) {
                return Ok(Some(resource.clone()));
            }
        }
        
        // Make API call
        let response = self.make_request::<()>(
            "GET", 
            &format!("resources/{}", id.as_str()), 
            None
        ).await;
        
        match response {
            Ok(data) => {
                let resource: Resource = serde_json::from_value(data)?;
                
                // Update cache
                let mut cache = self.cache.write().await;
                cache.insert(id.clone(), resource.clone());
                
                Ok(Some(resource))
            }
            Err(ScimError::NotFound { .. }) => Ok(None),
            Err(e) => Err(e),
        }
    }
    
    async fn update_resource(&self, resource: Resource) -> Result<Resource> {
        let api_data = serde_json::to_value(&resource)?;
        
        let response = self.make_request(
            "PUT", 
            &format!("resources/{}", resource.id().as_str()), 
            Some(api_data)
        ).await?;
        
        let updated_resource: Resource = serde_json::from_value(response)?;
        
        // Update cache
        let mut cache = self.cache.write().await;
        cache.insert(updated_resource.id().clone(), updated_resource.clone());
        
        Ok(updated_resource)
    }
    
    async fn patch_resource(&self, id: &ResourceId, patch: PatchRequest) -> Result<Resource> {
        let patch_data = serde_json::to_value(&patch)?;
        
        let response = self.make_request(
            "PATCH", 
            &format!("resources/{}", id.as_str()), 
            Some(patch_data)
        ).await?;
        
        let patched_resource: Resource = serde_json::from_value(response)?;
        
        // Update cache
        let mut cache = self.cache.write().await;
        cache.insert(id.clone(), patched_resource.clone());
        
        Ok(patched_resource)
    }
    
    async fn delete_resource(&self, id: &ResourceId) -> Result<()> {
        self.make_request::<()>("DELETE", &format!("resources/{}", id.as_str()), None).await?;
        
        // Remove from cache
        let mut cache = self.cache.write().await;
        cache.remove(id);
        
        Ok(())
    }
    
    async fn list_resources(&self, resource_type: ResourceType) -> Result<Vec<Resource>> {
        let response = self.make_request::<()>(
            "GET", 
            &format!("resources?resourceType={}", resource_type), 
            None
        ).await?;
        
        let resources: Vec<Resource> = serde_json::from_value(response["resources"].clone())?;
        
        // Update cache with all resources
        let mut cache = self.cache.write().await;
        for resource in &resources {
            cache.insert(resource.id().clone(), resource.clone());
        }
        
        Ok(resources)
    }
    
    async fn search_resources(&self, query: &SearchQuery) -> Result<SearchResult> {
        let query_params = serde_json::to_value(query)?;
        
        let response = self.make_request(
            "POST", 
            "search", 
            Some(query_params)
        ).await?;
        
        let search_result: SearchResult = serde_json::from_value(response)?;
        
        // Cache found resources
        let mut cache = self.cache.write().await;
        for resource in &search_result.resources {
            cache.insert(resource.id().clone(), resource.clone());
        }
        
        Ok(search_result)
    }
    
    async fn bulk_operations(&self, operations: BulkRequest) -> Result<BulkResponse> {
        let bulk_data = serde_json::to_value(&operations)?;
        
        let response = self.make_request(
            "POST", 
            "bulk", 
            Some(bulk_data)
        ).await?;
        
        let bulk_response: BulkResponse = serde_json::from_value(response)?;
        
        // Clear cache after bulk operations (could be complex to update precisely)
        let mut cache = self.cache.write().await;
        cache.clear();
        
        Ok(bulk_response)
    }
    
    async fn health_check(&self) -> Result<HealthStatus> {
        match self.make_request::<()>("GET", "health", None).await {
            Ok(_) => Ok(HealthStatus::healthy()),
            Err(_) => Ok(HealthStatus::unhealthy("API not reachable")),
        }
    }
    
    async fn get_statistics(&self) -> Result<ProviderStatistics> {
        let response = self.make_request::<()>("GET", "statistics", None).await?;
        let stats: ProviderStatistics = serde_json::from_value(response)?;
        Ok(stats)
    }
}
```

### Provider Lifecycle Management

```rust
impl CustomApiProvider {
    async fn initialize(&self) -> Result<()> {
        // Verify API connectivity
        self.health_check().await?;
        
        // Pre-warm any necessary connections
        self.client.get(&format!("{}/health", self.base_url))
            .send()
            .await
            .map_err(|e| ScimError::ProviderError { source: Box::new(e) })?;
        
        tracing::info!("Custom API provider initialized successfully");
        Ok(())
    }
    
    async fn shutdown(&self) -> Result<()> {
        // Flush any pending operations
        // Close connections gracefully
        tracing::info!("Custom API provider shut down gracefully");
        Ok(())
    }
}
```

## Error Handling

### Provider-Specific Errors

Providers should map their internal errors to SCIM errors:

```rust
impl From<sqlx::Error> for ScimError {
    fn from(err: sqlx::Error) -> Self {
        match err {
            sqlx::Error::RowNotFound => ScimError::NotFound {
                resource_type: "Resource".to_string(),
                id: "unknown".to_string(),
            },
            sqlx::Error::Database(db_err) if db_err.is_unique_violation() => {
                ScimError::Conflict {
                    message: "Resource already exists".to_string(),
                }
            }
            _ => ScimError::ProviderError {
                source: Box::new(err),
            }
        }
    }
}
```

### Error Context

Add context to errors for better debugging:

```rust
async fn create_resource(&self, resource: Resource) -> Result<Resource> {
    let resource_id = resource.id().clone();
    
    self.database.create_resource(resource).await
        .map_err(|e| e.with_context(format!("Failed to create resource {}", resource_id)))
}
```

## Testing Providers

### Provider Test Suite

Create a reusable test suite for any provider implementation:

```rust
// tests/common/provider_test_suite.rs
use scim_server::providers::ResourceProvider;
use scim_server::resource::{Resource, ResourceType};

pub async fn test_provider_compliance<P: ResourceProvider + 'static>(provider: P) {
    test_create_resource(&provider).await;
    test_get_resource(&provider).await;
    test_update_resource(&provider).await;
    test_delete_resource(&provider).await;
    test_list_resources(&provider).await;
    test_search_resources(&provider).await;
    test_concurrent_access(&provider).await;
}

async fn test_create_resource<P: ResourceProvider>(provider: &P) {
    let resource = create_test_user();
    let created = provider.create_resource(resource).await.unwrap();
    
    assert!(!created.id().as_str().is_empty());
    assert!(created.meta().created().is_some());
    assert!(created.meta().last_modified().is_some());
}

async fn test_get_resource<P: ResourceProvider>(provider: &P) {
    // First create a resource
    let resource = create_test_user();
    let created = provider.create_resource(resource).await.unwrap();
    
    // Then retrieve it
    let retrieved = provider.get_resource(created.id()).await.unwrap();
    assert!(retrieved.is_some());
    assert_eq!(retrieved.unwrap().id(), created.id());
    
    // Test non-existent resource
    let non_existent = ResourceId::new("does-not-exist").unwrap();
    let not_found = provider.get_resource(&non_existent).await.unwrap();
    assert!(not_found.is_none());
}

async fn test_concurrent_access<P: ResourceProvider>(provider: &P) {
    use tokio::task::JoinSet;
    use std::sync::Arc;
    
    let provider = Arc::new(provider);
    let mut join_set = JoinSet::new();
    
    // Spawn multiple concurrent operations
    for i in 0..10 {
        let provider = Arc::clone(&provider);
        join_set.spawn(async move {
            let resource = create_test_user_with_id(&format!("concurrent-user-{}", i));
            provider.create_resource(resource).await
        });
    }
    
    // Wait for all operations to complete
    let mut results = Vec::new();
    while let Some(result) = join_set.join_next().await {
        results.push(result.unwrap().unwrap());
    }
    
    assert_eq!(results.len(), 10);
    
    // Verify all resources exist
    let all_resources = provider.list_resources(ResourceType::User).await.unwrap();
    assert!(all_resources.len() >= 10);
}
```

### Using the Test Suite

```rust
// tests/integration/providers.rs
mod provider_test_suite;

#[tokio::test]
async fn test_in_memory_provider_compliance() {
    let storage = InMemoryStorage::new();
    let provider = StandardResourceProvider::new(storage);
    provider_test_suite::test_provider_compliance(provider).await;
}

#[tokio::test]
async fn test_database_provider_compliance() {
    let provider = setup_test_database_provider().await;
    provider_test_suite::test_provider_compliance(provider).await;
    cleanup_test_database().await;
}
```

## Performance Considerations

### Benchmarking Providers

```rust
// benches/provider_performance.rs
use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use scim_server::{
    providers::{StandardResourceProvider, DatabaseProvider},
    storage::InMemoryStorage,
};

fn bench_provider_operations(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    
    let storage = InMemoryStorage::new();
    let providers: Vec<(&str, Box<dyn ResourceProvider>)> = vec![
        ("memory", Box::new(StandardResourceProvider::new(storage))),
        ("database", Box::new(rt.block_on(setup_test_db_provider()))),
    ];
    
    for (name, provider) in providers {
        c.bench_with_input(
            BenchmarkId::new("create_resource", name),
            &provider,
            |b, provider| {
                b.to_async(&rt).iter(|| async {
                    let resource = create_test_resource();
                    black_box(provider.create_resource(resource).await.unwrap())
                })
            }
        );
        
        c.bench_with_input(
            BenchmarkId::new("get_resource", name),
            &provider,
            |b, provider| {
                b.to_async(&rt).iter_batched(
                    || {
                        // Setup: create a resource to retrieve
                        rt.block_on(async {
                            let resource = create_test_resource();
                            provider.create_resource(resource).await.unwrap()
                        })
                    },
                    |created_resource| async move {
                        black_box(provider.get_resource(created_resource.id()).await.unwrap())
                    },
                    criterion::BatchSize::SmallInput
                )
            }
        );
    }
}

criterion_group!(benches, bench_provider_operations);
criterion_main!(benches);
```

### Optimization Patterns

#### Connection Pooling

```rust
use sqlx::{Pool, Postgres};
use std::time::Duration;

pub struct OptimizedDatabaseProvider {
    pool: Pool<Postgres>,
    statement_cache: StatementCache,
}

impl OptimizedDatabaseProvider {
    pub async fn new(database_url: &str) -> Result<Self> {
        let pool = sqlx::postgres::PgPoolOptions::new()
            .max_connections(20)
            .min_connections(5)
            .acquire_timeout(Duration::from_secs(10))
            .idle_timeout(Duration::from_secs(300))
            .max_lifetime(Duration::from_secs(1800))
            .connect(database_url)
            .await?;
        
        Ok(Self {
            pool,
            statement_cache: StatementCache::new(),
        })
    }
}
```

#### Caching Layer

```rust
use std::sync::Arc;
use tokio::sync::RwLock;
use lru::LruCache;

pub struct CachedProvider<P: ResourceProvider> {
    inner: P,
    cache: Arc<RwLock<LruCache<ResourceId, Resource>>>,
    cache_ttl: Duration,
}

impl<P: ResourceProvider> CachedProvider<P> {
    pub fn new(inner: P, cache_size: usize, ttl: Duration) -> Self {
        Self {
            inner,
            cache: Arc::new(RwLock::new(LruCache::new(cache_size))),
            cache_ttl: ttl,
        }
    }
}

#[async_trait]
impl<P: ResourceProvider + 'static> ResourceProvider for CachedProvider<P> {
    async fn get_resource(&self, id: &ResourceId) -> Result<Option<Resource>> {
        // Check cache first
        {
            let cache = self.cache.read().await;
            if let Some(resource) = cache.peek(id).cloned() {
                return Ok(Some(resource));
            }
        }
        
        // Fallback to inner provider
        let resource = self.inner.get_resource(id).await?;
        
        // Cache the result
        if let Some(ref resource) = resource {
            let mut cache = self.cache.write().await;
            cache.put(id.clone(), resource.clone());
        }
        
        Ok(resource)
    }
    
    async fn create_resource(&self, resource: Resource) -> Result<Resource> {
        let created = self.inner.create_resource(resource).await?;
        
        // Add to cache
        let mut cache = self.cache.write().await;
        cache.put(created.id().clone(), created.clone());
        
        Ok(created)
    }
    
    async fn update_resource(&self, resource: Resource) -> Result<Resource> {
        let updated = self.inner.update_resource(resource).await?;
        
        // Update cache
        let mut cache = self.cache.write().await;
        cache.put(updated.id().clone(), updated.clone());
        
        Ok(updated)
    }
    
    async fn delete_resource(&self, id: &ResourceId) -> Result<()> {
        self.inner.delete_resource(id).await?;
        
        // Remove from cache
        let mut cache = self.cache.write().await;
        cache.pop(id);
        
        Ok(())
    }
    
    // Delegate other methods to inner provider
    async fn list_resources(&self, resource_type: ResourceType) -> Result<Vec<Resource>> {
        self.inner.list_resources(resource_type).await
    }
    
    async fn search_resources(&self, query: &SearchQuery) -> Result<SearchResult> {
        self.inner.search_resources(query).await
    }
    
    async fn bulk_operations(&self, operations: BulkRequest) -> Result<BulkResponse> {
        let result = self.inner.bulk_operations(operations).await?;
        
        // Clear cache after bulk operations (simpler than selective updates)
        let mut cache = self.cache.write().await;
        cache.clear();
        
        Ok(result)
    }
    
    async fn health_check(&self) -> Result<HealthStatus> {
        self.inner.health_check().await
    }
    
    async fn get_statistics(&self) -> Result<ProviderStatistics> {
        let mut stats = self.inner.get_statistics().await?;
        
        // Add cache statistics
        let cache = self.cache.read().await;
        stats.cache_stats = Some(CacheStatistics {
            size: cache.len(),
            capacity: cache.cap(),
            hit_rate: 0.0, // Would need to track hits/misses
        });
        
        Ok(stats)
    }
}
```

## Provider Statistics and Monitoring

### Statistics Interface

```rust
#[derive(Debug, Clone, serde::Serialize)]