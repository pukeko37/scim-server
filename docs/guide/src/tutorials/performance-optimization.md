# Performance Optimization

This tutorial covers techniques for optimizing SCIM Server performance, including database optimization, caching strategies, connection pooling, and monitoring performance bottlenecks.

## Overview

Performance optimization in SCIM Server involves several layers:

- **Database Performance**: Query optimization, indexing, and connection pooling
- **Application Performance**: Efficient data structures and algorithms
- **Caching**: Strategic caching of frequently accessed data
- **Network Performance**: Connection reuse and payload optimization
- **Monitoring**: Identifying and resolving bottlenecks

## Database Optimization

### Query Performance

**Efficient data loading patterns:**
```rust
use scim_server::{ListOptions};

// Inefficient: Load all users then filter in memory
async fn get_active_users_slow(provider: &impl Provider, tenant_id: &str) -> Result<Vec<ScimUser>, Error> {
    let all_users = provider.list_users(tenant_id, &ListOptions::default()).await?;
    let active_users: Vec<_> = all_users.resources.into_iter()
        .filter(|user| user.active())
        .collect();
    Ok(active_users)
}

// Better: Use pagination to limit memory usage
async fn get_users_paginated(provider: &impl Provider, tenant_id: &str) -> Result<Vec<ScimUser>, Error> {
    let options = ListOptions::builder()
        .count(Some(100))        // Limit to 100 users per request
        .start_index(Some(1))    // Start from first user
        .build();
    
    let response = provider.list_users(tenant_id, &options).await?;
    
    // Filter in memory for now (database filtering not yet implemented)
    let active_users: Vec<_> = response.resources.into_iter()
        .filter(|user| user.active())
        .collect();
    
    Ok(active_users)
}
```

**Optimize complex queries:**
```sql
-- Add indexes for common filter patterns
CREATE INDEX CONCURRENTLY idx_users_active_dept ON users(tenant_id, active, department) 
WHERE active = true;

CREATE INDEX CONCURRENTLY idx_users_email_lookup ON users(tenant_id, (data->>'primaryEmail'));

CREATE INDEX CONCURRENTLY idx_users_last_modified ON users(tenant_id, updated_at) 
WHERE updated_at > NOW() - INTERVAL '30 days';

-- Use partial indexes for common conditions
CREATE INDEX CONCURRENTLY idx_groups_with_members ON groups(tenant_id, display_name) 
WHERE jsonb_array_length(data->'members') > 0;
```

### Connection Pooling

**Optimize database connections:**
```rust
use sqlx::postgres::PgPoolOptions;
use std::time::Duration;

pub async fn create_optimized_pool(database_url: &str) -> Result<sqlx::PgPool, sqlx::Error> {
    PgPoolOptions::new()
        .max_connections(20)                    // Adjust based on your load
        .min_connections(5)                     // Keep minimum connections warm
        .acquire_timeout(Duration::from_secs(30))
        .idle_timeout(Some(Duration::from_secs(600)))
        .max_lifetime(Some(Duration::from_secs(1800)))
        .test_before_acquire(true)              // Test connections before use
        .after_connect(|conn, _meta| {
            Box::pin(async move {
                // Optimize connection settings
                sqlx::query("SET statement_timeout = '30s'")
                    .execute(conn)
                    .await?;
                sqlx::query("SET lock_timeout = '10s'")
                    .execute(conn)
                    .await?;
                Ok(())
            })
        })
        .connect(database_url)
        .await
}
```

### Batch Operations

**Use transactions for related operations:**
```rust
use sqlx::{Transaction, Postgres};

async fn create_user_with_groups_optimized(
    provider: &DatabaseProvider,
    tenant_id: &str,
    user: ScimUser,
    group_ids: Vec<String>,
) -> Result<ScimUser, ProviderError> {
    let mut tx = provider.begin_transaction().await?;
    
    // Create user
    let created_user = tx.create_user(tenant_id, user).await?;
    
    // Add to groups in batch
    if !group_ids.is_empty() {
        let query = format!(
            "INSERT INTO group_memberships (group_id, user_id) VALUES {}",
            group_ids.iter()
                .map(|_| "($1, $2)")
                .collect::<Vec<_>>()
                .join(", ")
        );
        
        let mut query_builder = sqlx::query(&query);
        for group_id in &group_ids {
            query_builder = query_builder.bind(group_id).bind(created_user.id());
        }
        
        query_builder.execute(&mut *tx).await?;
    }
    
    tx.commit().await?;
    Ok(created_user)
}
```

## Caching Strategies

### Redis Caching

**Implement multi-layer caching:**
```rust
use redis::{AsyncCommands, Client};
use serde::{Serialize, Deserialize};
use std::time::Duration;

#[derive(Clone)]
pub struct CachedProvider {
    inner: DatabaseProvider,
    redis: Client,
    cache_ttl: Duration,
}

impl CachedProvider {
    pub fn new(inner: DatabaseProvider, redis_url: &str, cache_ttl: Duration) -> Result<Self, redis::RedisError> {
        let redis = Client::open(redis_url)?;
        Ok(Self { inner, redis, cache_ttl })
    }
    
    async fn get_user_cached(&self, tenant_id: &str, user_id: &str) -> Result<Option<ScimUser>, ProviderError> {
        let cache_key = format!("user:{}:{}", tenant_id, user_id);
        
        // Try L1 cache (Redis)
        if let Ok(mut conn) = self.redis.get_async_connection().await {
            if let Ok(cached_data) = conn.get::<_, String>(&cache_key).await {
                if let Ok(user) = serde_json::from_str::<ScimUser>(&cached_data) {
                    return Ok(Some(user));
                }
            }
        }
        
        // L2 cache miss - fetch from database
        let user = self.inner.get_user(tenant_id, user_id).await?;
        
        // Cache the result
        if let (Some(ref user), Ok(mut conn)) = (&user, self.redis.get_async_connection().await) {
            if let Ok(serialized) = serde_json::to_string(user) {
                let _: Result<(), _> = conn.setex(&cache_key, self.cache_ttl.as_secs(), serialized).await;
            }
        }
        
        Ok(user)
    }
    
    async fn invalidate_user_cache(&self, tenant_id: &str, user_id: &str) -> Result<(), redis::RedisError> {
        let cache_key = format!("user:{}:{}", tenant_id, user_id);
        let mut conn = self.redis.get_async_connection().await?;
        conn.del(&cache_key).await?;
        
        // Also invalidate related caches
        let pattern = format!("users:{}:*", tenant_id);
        self.invalidate_pattern(&pattern).await?;
        
        Ok(())
    }
    
    async fn invalidate_pattern(&self, pattern: &str) -> Result<(), redis::RedisError> {
        let mut conn = self.redis.get_async_connection().await?;
        let keys: Vec<String> = conn.keys(pattern).await?;
        
        if !keys.is_empty() {
            conn.del(&keys).await?;
        }
        
        Ok(())
    }
}

// Implement cache-aware operations
#[async_trait]
impl Provider for CachedProvider {
    async fn get_user(&self, tenant_id: &str, user_id: &str) -> Result<Option<ScimUser>, ProviderError> {
        self.get_user_cached(tenant_id, user_id).await
    }
    
    async fn update_user(&self, tenant_id: &str, user: ScimUser) -> Result<ScimUser, ProviderError> {
        let updated_user = self.inner.update_user(tenant_id, user).await?;
        
        // Invalidate cache
        if let Err(e) = self.invalidate_user_cache(tenant_id, updated_user.id()).await {
            tracing::warn!("Failed to invalidate user cache: {}", e);
        }
        
        Ok(updated_user)
    }
}
```

### In-Memory Caching

**Application-level caching for frequently accessed data:**
```rust
use std::sync::Arc;
use tokio::sync::RwLock;
use std::collections::HashMap;
use std::time::{Duration, Instant};

#[derive(Clone)]
struct CacheEntry<T> {
    value: T,
    expires_at: Instant,
}

#[derive(Clone)]
pub struct MemoryCache<T> {
    data: Arc<RwLock<HashMap<String, CacheEntry<T>>>>,
    ttl: Duration,
}

impl<T: Clone> MemoryCache<T> {
    pub fn new(ttl: Duration) -> Self {
        Self {
            data: Arc::new(RwLock::new(HashMap::new())),
            ttl,
        }
    }
    
    pub async fn get(&self, key: &str) -> Option<T> {
        let data = self.data.read().await;
        
        if let Some(entry) = data.get(key) {
            if entry.expires_at > Instant::now() {
                return Some(entry.value.clone());
            }
        }
        
        None
    }
    
    pub async fn set(&self, key: String, value: T) {
        let mut data = self.data.write().await;
        data.insert(key, CacheEntry {
            value,
            expires_at: Instant::now() + self.ttl,
        });
    }
    
    pub async fn invalidate(&self, key: &str) {
        let mut data = self.data.write().await;
        data.remove(key);
    }
    
    // Background cleanup task
    pub async fn cleanup_expired(&self) {
        let mut data = self.data.write().await;
        let now = Instant::now();
        data.retain(|_, entry| entry.expires_at > now);
    }
}

// Usage in provider
#[derive(Clone)]
pub struct MemoryCachedProvider {
    inner: DatabaseProvider,
    user_cache: MemoryCache<ScimUser>,
    schema_cache: MemoryCache<Schema>,
}

impl MemoryCachedProvider {
    pub fn new(inner: DatabaseProvider) -> Self {
        let provider = Self {
            inner,
            user_cache: MemoryCache::new(Duration::from_secs(300)), // 5 minutes
            schema_cache: MemoryCache::new(Duration::from_secs(3600)), // 1 hour
        };
        
        // Start cleanup task
        let cache = provider.user_cache.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(60));
            loop {
                interval.tick().await;
                cache.cleanup_expired().await;
            }
        });
        
        provider
    }
}
```

## Connection and Resource Management

### HTTP Client Optimization

**Reuse HTTP connections:**
```rust
use reqwest::Client;
use std::time::Duration;

lazy_static! {
    static ref HTTP_CLIENT: Client = Client::builder()
        .timeout(Duration::from_secs(30))
        .connect_timeout(Duration::from_secs(10))
        .pool_max_idle_per_host(10)
        .pool_idle_timeout(Duration::from_secs(90))
        .build()
        .expect("Failed to create HTTP client");
}

// Use the shared client for external API calls
async fn validate_oauth_token(token: &str) -> Result<Claims, Error> {
    let response = HTTP_CLIENT
        .post("https://oauth.provider.com/introspect")
        .form(&[("token", token)])
        .send()
        .await?;
    
    let claims: Claims = response.json().await?;
    Ok(claims)
}
```

### Resource Pooling

**Implement object pooling for expensive operations:**
```rust
use deadpool::managed::{Manager, Object, Pool, PoolError};
use async_trait::async_trait;

#[derive(Clone)]
pub struct ExpensiveResource {
    // Some expensive-to-create resource
    id: uuid::Uuid,
    data: Vec<u8>,
}

pub struct ResourceManager;

#[async_trait]
impl Manager for ResourceManager {
    type Type = ExpensiveResource;
    type Error = Box<dyn std::error::Error + Send + Sync>;
    
    async fn create(&self) -> Result<Self::Type, Self::Error> {
        // Expensive resource creation
        tokio::time::sleep(Duration::from_millis(100)).await;
        
        Ok(ExpensiveResource {
            id: uuid::Uuid::new_v4(),
            data: vec![0u8; 1024 * 1024], // 1MB
        })
    }
    
    async fn recycle(&self, _obj: &mut Self::Type) -> Result<(), Self::Error> {
        // Reset/cleanup resource for reuse
        Ok(())
    }
}

// Usage
pub async fn create_resource_pool() -> Pool<ResourceManager> {
    Pool::builder(ResourceManager)
        .max_size(10)
        .build()
        .expect("Failed to create resource pool")
}
```

## Algorithm and Data Structure Optimization

### Efficient Data Structures

**Use appropriate data structures for different access patterns:**
```rust
use std::collections::{HashMap, BTreeMap, HashSet};
use indexmap::IndexMap;

#[derive(Clone)]
pub struct OptimizedUserStore {
    // Fast lookup by ID
    users_by_id: HashMap<String, ScimUser>,
    
    // Fast lookup by username (unique)
    users_by_username: HashMap<String, String>, // username -> id
    
    // Fast lookup by email
    users_by_email: HashMap<String, String>, // email -> id
    
    // Ordered access for pagination
    users_ordered: IndexMap<String, ScimUser>, // maintains insertion order
    
    // Fast membership testing
    active_user_ids: HashSet<String>,
}

impl OptimizedUserStore {
    pub fn new() -> Self {
        Self {
            users_by_id: HashMap::new(),
            users_by_username: HashMap::new(),
            users_by_email: HashMap::new(),
            users_ordered: IndexMap::new(),
            active_user_ids: HashSet::new(),
        }
    }
    
    pub fn add_user(&mut self, user: ScimUser) {
        let id = user.id().to_string();
        let username = user.username().to_string();
        
        // Update all indexes
        self.users_by_username.insert(username, id.clone());
        
        if let Some(email) = user.primary_email() {
            self.users_by_email.insert(email.to_string(), id.clone());
        }
        
        if user.active() {
            self.active_user_ids.insert(id.clone());
        }
        
        self.users_by_id.insert(id.clone(), user.clone());
        self.users_ordered.insert(id, user);
    }
    
    pub fn get_by_username(&self, username: &str) -> Option<&ScimUser> {
        self.users_by_username
            .get(username)
            .and_then(|id| self.users_by_id.get(id))
    }
    
    pub fn get_active_users(&self) -> impl Iterator<Item = &ScimUser> {
        self.active_user_ids
            .iter()
            .filter_map(|id| self.users_by_id.get(id))
    }
    
    pub fn paginate(&self, start: usize, count: usize) -> impl Iterator<Item = &ScimUser> {
        self.users_ordered
            .values()
            .skip(start)
            .take(count)
    }
}
```

### Bulk Processing

**Optimize bulk operations:**
```rust
use futures::stream::{self, StreamExt};
use std::sync::atomic::{AtomicUsize, Ordering};

pub struct BulkProcessor {
    concurrency_limit: usize,
    batch_size: usize,
}

impl BulkProcessor {
    pub fn new(concurrency_limit: usize, batch_size: usize) -> Self {
        Self {
            concurrency_limit,
            batch_size,
        }
    }
    
    pub async fn process_users_bulk<F, Fut>(
        &self,
        users: Vec<ScimUser>,
        processor: F,
    ) -> Result<Vec<ProcessResult>, Error>
    where
        F: Fn(ScimUser) -> Fut + Clone + Send + 'static,
        Fut: Future<Output = Result<ScimUser, Error>> + Send,
    {
        let processed_count = AtomicUsize::new(0);
        let total_count = users.len();
        
        let results = stream::iter(users)
            .map(move |user| {
                let processor = processor.clone();
                let processed_count = &processed_count;
                
                async move {
                    let result = processor(user).await;
                    let count = processed_count.fetch_add(1, Ordering::Relaxed) + 1;
                    
                    if count % 100 == 0 {
                        tracing::info!("Processed {}/{} users", count, total_count);
                    }
                    
                    result
                }
            })
            .buffer_unordered(self.concurrency_limit)
            .collect::<Vec<_>>()
            .await;
        
        Ok(results.into_iter().collect())
    }
    
    pub async fn process_in_batches<T, F, Fut>(
        &self,
        items: Vec<T>,
        processor: F,
    ) -> Result<Vec<T>, Error>
    where
        T: Send + 'static,
        F: Fn(Vec<T>) -> Fut + Send + 'static,
        Fut: Future<Output = Result<Vec<T>, Error>> + Send,
    {
        let mut results = Vec::new();
        
        for batch in items.chunks(self.batch_size) {
            let batch_result = processor(batch.to_vec()).await?;
            results.extend(batch_result);
        }
        
        Ok(results)
    }
}
```

## Performance Monitoring

### Metrics Collection

**Track key performance indicators:**
```rust
use prometheus::{Counter, Histogram, Gauge, register_counter, register_histogram, register_gauge};
use std::time::Instant;

lazy_static! {
    static ref OPERATION_DURATION: Histogram = register_histogram!(
        "scim_operation_duration_seconds",
        "Duration of SCIM operations",
        vec![0.001, 0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0]
    ).unwrap();
    
    static ref CACHE_HITS: Counter = register_counter!(
        "scim_cache_hits_total",
        "Total cache hits"
    ).unwrap();
    
    static ref CACHE_MISSES: Counter = register_counter!(
        "scim_cache_misses_total",
        "Total cache misses"
    ).unwrap();
    
    static ref ACTIVE_CONNECTIONS: Gauge = register_gauge!(
        "scim_active_db_connections",
        "Number of active database connections"
    ).unwrap();
}

pub struct PerformanceTracker;

impl PerformanceTracker {
    pub fn time_operation<F, T>(operation_name: &str, f: F) -> T
    where
        F: FnOnce() -> T,
    {
        let _timer = OPERATION_DURATION
            .with_label_values(&[operation_name])
            .start_timer();
        
        f()
    }
    
    pub async fn time_async_operation<F, Fut, T>(operation_name: &str, f: F) -> T
    where
        F: FnOnce() -> Fut,
        Fut: Future<Output = T>,
    {
        let _timer = OPERATION_DURATION
            .with_label_values(&[operation_name])
            .start_timer();
        
        f().await
    }
    
    pub fn record_cache_hit() {
        CACHE_HITS.inc();
    }
    
    pub fn record_cache_miss() {
        CACHE_MISSES.inc();
    }
    
    pub fn set_active_connections(count: i64) {
        ACTIVE_CONNECTIONS.set(count as f64);
    }
}

// Usage in provider
impl CachedProvider {
    async fn get_user(&self, tenant_id: &str, user_id: &str) -> Result<Option<ScimUser>, ProviderError> {
        PerformanceTracker::time_async_operation("get_user", async {
            if let Some(user) = self.get_from_cache(tenant_id, user_id).await {
                PerformanceTracker::record_cache_hit();
                return Ok(Some(user));
            }
            
            PerformanceTracker::record_cache_miss();
            let user = self.inner.get_user(tenant_id, user_id).await?;
            
            if let Some(ref user) = user {
                self.cache_user(tenant_id, user).await;
            }
            
            Ok(user)
        }).await
    }
}
```

### Performance Profiling

**Add profiling capabilities:**
```rust
use std::time::{Duration, Instant};
use std::collections::HashMap;
use tokio::sync::RwLock;

#[derive(Clone)]
pub struct ProfileData {
    pub calls: u64,
    pub total_duration: Duration,
    pub min_duration: Duration,
    pub max_duration: Duration,
    pub avg_duration: Duration,
}

#[derive(Clone)]
pub struct Profiler {
    data: Arc<RwLock<HashMap<String, ProfileData>>>,
}

impl Profiler {
    pub fn new() -> Self {
        Self {
            data: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    
    pub async fn profile<F, T>(&self, name: &str, f: F) -> T
    where
        F: FnOnce() -> T,
    {
        let start = Instant::now();
        let result = f();
        let duration = start.elapsed();
        
        self.record(name, duration).await;
        result
    }
    
    pub async fn profile_async<F, Fut, T>(&self, name: &str, f: F) -> T
    where
        F: FnOnce() -> Fut,
        Fut: Future<Output = T>,
    {
        let start = Instant::now();
        let result = f().await;
        let duration = start.elapsed();
        
        self.record(name, duration).await;
        result
    }
    
    async fn record(&self, name: &str, duration: Duration) {
        let mut data = self.data.write().await;
        
        let entry = data.entry(name.to_string()).or_insert(ProfileData {
            calls: 0,
            total_duration: Duration::ZERO,
            min_duration: Duration::MAX,
            max_duration: Duration::ZERO,
            avg_duration: Duration::ZERO,
        });
        
        entry.calls += 1;
        entry.total_duration += duration;
        entry.min_duration = entry.min_duration.min(duration);
        entry.max_duration = entry.max_duration.max(duration);
        entry.avg_duration = entry.total_duration / entry.calls as u32;
    }
    
    pub async fn get_report(&self) -> HashMap<String, ProfileData> {
        self.data.read().await.clone()
    }
    
    pub async fn reset(&self) {
        self.data.write().await.clear();
    }
}

// Usage
lazy_static! {
    static ref GLOBAL_PROFILER: Profiler = Profiler::new();
}

// Endpoint to get profiling data
async fn profiling_report() -> Json<serde_json::Value> {
    let report = GLOBAL_PROFILER.get_report().await;
    
    let formatted_report: HashMap<String, serde_json::Value> = report
        .into_iter()
        .map(|(name, data)| {
            (name, json!({
                "calls": data.calls,
                "total_duration_ms": data.total_duration.as_millis(),
                "avg_duration_ms": data.avg_duration.as_millis(),
                "min_duration_ms": data.min_duration.as_millis(),
                "max_duration_ms": data.max_duration.as_millis(),
            }))
        })
        .collect();
    
    Json(json!(formatted_report))
}
```

## Load Testing and Benchmarking

### Load Testing Setup

**Create load tests to identify bottlenecks:**
```rust
#[cfg(test)]
mod load_tests {
    use super::*;
    use tokio::task::JoinSet;
    use std::sync::Arc;
    use std::time::{Duration, Instant};
    
    #[tokio::test]
    #[ignore] // Run with --ignored flag
    async fn load_test_user_operations() {
        let provider = create_test_provider().await;
        let tenant_id = "load-test-tenant";
        
        let concurrent_operations = 100;
        let operations_per_task = 10;
        
        let start_time = Instant::now();
        let mut tasks = JoinSet::new();
        
        for task_id in 0..concurrent_operations {
            let provider = provider.clone();
            let tenant_id = tenant_id.to_string();
            
            tasks.spawn(async move {
                for i in 0..operations_per_task {
                    let user = ScimUser::builder()
                        .username(&format!("user-{}-{}", task_id, i))
                        .given_name("Load")
                        .family_name("Test")
                        .email(&format!("user-{}-{}@test.com", task_id, i))
                        .build()
                        .unwrap();
                    
                    // Create user
                    let created = provider.create_user(&tenant_id, user).await.unwrap();
                    
                    // Read user
                    let _read = provider.get_user(&tenant_id, created.id()).await.unwrap();
                    
                    // Update user
                    let mut updated = created;
                    updated.set_given_name("Updated");
                    let _updated = provider.update_user(&tenant_id, updated).await.unwrap();
                }
            });
        }
        
        // Wait for all tasks to complete
        while let Some(result) = tasks.join_next().await {
            result.unwrap();
        }
        
        let total_duration = start_time.elapsed();
        let total_operations = concurrent_operations * operations_per_task * 3; // create, read, update
        let ops_per_second = total_operations as f64 / total_duration.as_secs_f64();
        
        println!("Load test completed:");
        println!("  Total operations: {}", total_operations);
        println!("  Total duration: {:?}", total_duration);
        println!("  Operations per second: {:.2}", ops_per_second);
        
        // Assert minimum performance requirements
        assert!(ops_per_second > 100.0, "Performance below threshold: {} ops/sec", ops_per_second);
    }
    
    #[tokio::test]
    #[ignore]
    async fn benchmark_filtering_performance() {
        let provider = create_test_provider().await;
        let tenant_id = "benchmark-tenant";
        
        // Create test data
        for i in 0..1000 {
            let user = ScimUser::builder()
                .username(&format!("user-{}", i))
                .given_name("Benchmark")
                .family_name("User")
                .department(if i % 3 == 0 { "Engineering" } else { "Sales" })
                .active(i % 2 == 0)
                .build()
                .unwrap();
            
            provider.create_user(tenant_id, user).await.unwrap();
        }
        
        // Benchmark different page sizes for pagination performance
        let page_sizes = [10, 50, 100, 500, 1000];
        
        for page_size in page_sizes {
            let start = Instant::now();
            let iterations = 50;
            
            for _ in 0..iterations {
                let options = ListOptions::builder()
                    .count(Some(page_size))
                    .start_index(Some(1))
                    .build();
                
                let results = provider.list_users(tenant_id, &options).await.unwrap();
                
                // Simulate in-memory filtering work
                let _active_users: Vec<_> = results.resources.into_iter()
                    .filter(|user| user.active())
                    .collect();
            }
            
            let duration = start.elapsed();
            let avg_duration = duration / iterations;
            
            println!("Page size {}: avg {}ms", page_size, avg_duration.as_millis());
        }
    }
}
```

This comprehensive performance optimization guide covers all major aspects of making SCIM Server performant at scale, from database optimization to application-level caching and monitoring.