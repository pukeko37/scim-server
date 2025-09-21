# Storage & Persistence Patterns

This deep dive explores storage and persistence patterns in SCIM Server, covering different storage backends, caching strategies, performance optimization techniques, and patterns for integrating with existing databases and external systems.

## Overview

The storage layer in SCIM Server provides the foundation for data persistence while maintaining abstraction from specific storage technologies. This document shows how to implement robust storage patterns that scale from simple in-memory setups to complex distributed systems.

**Core Storage Flow:**
```text
Resource Operations → Storage Provider → Backend Implementation → 
Data Persistence → Caching → Performance Optimization
```

## Storage Architecture Overview

```text
┌─────────────────────────────────────────────────────────────────────────────┐
│ Storage Provider Trait (Abstract Interface)                                │
│                                                                             │
│ ┌─────────────────┐ ┌─────────────────┐ ┌─────────────────────────────────┐ │
│ │ In-Memory       │ │ Database        │ │ External System                 │ │
│ │ Storage         │ │ Storage         │ │ Storage                         │ │
│ │                 │ │                 │ │                                 │ │
│ │ • Development   │ │ • PostgreSQL    │ │ • REST APIs                     │ │
│ │ • Testing       │ │ • MongoDB       │ │ • GraphQL                       │ │
│ │ • Prototyping   │ │ • Redis         │ │ • Message Queues                │ │
│ │ • Simple apps   │ │ • DynamoDB      │ │ • Legacy systems                │ │
│ └─────────────────┘ └─────────────────┘ └─────────────────────────────────┘ │
└─────────────────────────────────────────────────────────────────────────────┘
                                    ↓
┌─────────────────────────────────────────────────────────────────────────────┐
│ Storage Enhancement Layers                                                  │
│                                                                             │
│ ┌─────────────────┐ ┌─────────────────┐ ┌─────────────────────────────────┐ │
│ │ Caching Layer   │ │ Connection      │ │ Monitoring &                    │ │
│ │                 │ │ Pooling         │ │ Observability                   │ │
│ │ • Redis         │ │                 │ │                                 │ │
│ │ • In-memory     │ │ • Database      │ │ • Metrics collection            │ │
│ │ • Multi-level   │ │   pools         │ │ • Performance tracking          │ │
│ │ • Write-through │ │ • Connection    │ │ • Error monitoring              │ │
│ │ • Write-behind  │ │   management    │ │ • Distributed tracing           │ │
│ └─────────────────┘ └─────────────────┘ └─────────────────────────────────┘ │
└─────────────────────────────────────────────────────────────────────────────┘
                                    ↓
┌─────────────────────────────────────────────────────────────────────────────┐
│ Advanced Storage Patterns                                                   │
│ • Sharding • Replication • Event sourcing • CQRS • Backup/Recovery         │
└─────────────────────────────────────────────────────────────────────────────┘
```

## Core Storage Patterns

### Pattern 1: Database Storage with Connection Pooling

```rust
use scim_server::storage::{StorageProvider, StorageKey, StoragePrefix};
use sqlx::{PgPool, Row, Postgres, Transaction};
use serde_json::{Value, json};
use std::collections::HashMap;
use uuid::Uuid;

pub struct PostgresStorageProvider {
    pool: PgPool,
    table_name: String,
    connection_config: ConnectionConfig,
    performance_monitor: Arc<PerformanceMonitor>,
}

#[derive(Clone)]
pub struct ConnectionConfig {
    pub max_connections: u32,
    pub min_connections: u32,
    pub acquire_timeout: Duration,
    pub idle_timeout: Duration,
    pub max_lifetime: Duration,
}

impl PostgresStorageProvider {
    pub async fn new(
        database_url: &str,
        table_name: String,
        config: ConnectionConfig,
    ) -> Result<Self, StorageError> {
        let pool = PgPool::connect_with(
            sqlx::postgres::PgPoolOptions::new()
                .max_connections(config.max_connections)
                .min_connections(config.min_connections)
                .acquire_timeout(config.acquire_timeout)
                .idle_timeout(config.idle_timeout)
                .max_lifetime(config.max_lifetime)
                .parse(database_url)?
        ).await?;
        
        let provider = Self {
            pool,
            table_name,
            connection_config: config,
            performance_monitor: Arc::new(PerformanceMonitor::new()),
        };
        
        // Initialize database schema
        provider.initialize_schema().await?;
        
        Ok(provider)
    }
    
    async fn initialize_schema(&self) -> Result<(), StorageError> {
        let create_table_sql = format!(
            r#"
            CREATE TABLE IF NOT EXISTS {} (
                id TEXT PRIMARY KEY,
                resource_type TEXT NOT NULL,
                tenant_id TEXT,
                data JSONB NOT NULL,
                version TEXT NOT NULL,
                created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
                updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
                deleted_at TIMESTAMP WITH TIME ZONE,
                INDEX idx_resource_type ON {} (resource_type),
                INDEX idx_tenant_id ON {} (tenant_id),
                INDEX idx_tenant_resource ON {} (tenant_id, resource_type),
                INDEX idx_data_gin ON {} USING GIN (data)
            )
            "#,
            self.table_name, self.table_name, self.table_name, self.table_name, self.table_name
        );
        
        sqlx::query(&create_table_sql)
            .execute(&self.pool)
            .await?;
        
        Ok(())
    }
    
    async fn with_transaction<F, R>(&self, operation: F) -> Result<R, StorageError>
    where
        F: FnOnce(&mut Transaction<Postgres>) -> futures::future::BoxFuture<Result<R, StorageError>>,
    {
        let mut tx = self.pool.begin().await?;
        let result = operation(&mut tx).await;
        
        match result {
            Ok(value) => {
                tx.commit().await?;
                Ok(value)
            }
            Err(err) => {
                tx.rollback().await?;
                Err(err)
            }
        }
    }
    
    fn extract_tenant_and_resource_info(&self, key: &StorageKey) -> (Option<String>, String, String) {
        let key_str = key.as_str();
        
        // Handle tenant-scoped keys (format: "tenant:tenant_id:resource_type:resource_id")
        if key_str.starts_with("tenant:") {
            let parts: Vec<&str> = key_str.splitn(4, ':').collect();
            if parts.len() == 4 {
                return (
                    Some(parts[1].to_string()),
                    parts[2].to_string(),
                    parts[3].to_string(),
                );
            }
        }
        
        // Handle non-tenant keys (format: "resource_type:resource_id")
        let parts: Vec<&str> = key_str.splitn(2, ':').collect();
        if parts.len() == 2 {
            (None, parts[0].to_string(), parts[1].to_string())
        } else {
            (None, "unknown".to_string(), key_str.to_string())
        }
    }
    
    async fn generate_version(&self, data: &Value) -> String {
        use std::hash::{Hash, Hasher};
        use std::collections::hash_map::DefaultHasher;
        
        let mut hasher = DefaultHasher::new();
        data.to_string().hash(&mut hasher);
        chrono::Utc::now().timestamp_nanos().hash(&mut hasher);
        
        format!("W/\"{}\"", hasher.finish())
    }
}

impl StorageProvider for PostgresStorageProvider {
    type Error = PostgresStorageError;
    
    async fn put(
        &self,
        key: StorageKey,
        mut data: Value,
        context: &RequestContext,
    ) -> Result<Value, Self::Error> {
        let start_time = std::time::Instant::now();
        
        let (tenant_id, resource_type, resource_id) = self.extract_tenant_and_resource_info(&key);
        let version = self.generate_version(&data).await;
        
        // Add metadata to the data
        data["meta"] = json!({
            "version": version,
            "created": chrono::Utc::now().to_rfc3339(),
            "lastModified": chrono::Utc::now().to_rfc3339(),
            "resourceType": resource_type,
            "location": format!("/scim/v2/{}/{}", resource_type, resource_id)
        });
        
        data["id"] = json!(resource_id);
        
        let result = self.with_transaction(|tx| {
            Box::pin(async move {
                // Use UPSERT to handle both create and update
                let query = format!(
                    r#"
                    INSERT INTO {} (id, resource_type, tenant_id, data, version, created_at, updated_at)
                    VALUES ($1, $2, $3, $4, $5, NOW(), NOW())
                    ON CONFLICT (id) DO UPDATE SET
                        data = EXCLUDED.data,
                        version = EXCLUDED.version,
                        updated_at = NOW()
                    RETURNING data
                    "#,
                    self.table_name
                );
                
                let row = sqlx::query(&query)
                    .bind(&resource_id)
                    .bind(&resource_type)
                    .bind(&tenant_id)
                    .bind(&data)
                    .bind(&version)
                    .fetch_one(&mut **tx)
                    .await?;
                
                let stored_data: Value = row.get("data");
                Ok(stored_data)
            })
        }).await?;
        
        let duration = start_time.elapsed();
        self.performance_monitor.record_operation("put", duration, true);
        
        Ok(result)
    }
    
    async fn get(
        &self,
        key: StorageKey,
        _context: &RequestContext,
    ) -> Result<Option<Value>, Self::Error> {
        let start_time = std::time::Instant::now();
        
        let (tenant_id, _resource_type, resource_id) = self.extract_tenant_and_resource_info(&key);
        
        let query = format!(
            "SELECT data FROM {} WHERE id = $1 AND ($2::TEXT IS NULL OR tenant_id = $2) AND deleted_at IS NULL",
            self.table_name
        );
        
        let result = sqlx::query(&query)
            .bind(&resource_id)
            .bind(&tenant_id)
            .fetch_optional(&self.pool)
            .await?;
        
        let duration = start_time.elapsed();
        self.performance_monitor.record_operation("get", duration, true);
        
        Ok(result.map(|row| row.get("data")))
    }
    
    async fn list(
        &self,
        prefix: StoragePrefix,
        _context: &RequestContext,
    ) -> Result<Vec<Value>, Self::Error> {
        let start_time = std::time::Instant::now();
        
        let prefix_str = prefix.as_str();
        let (tenant_id, resource_type) = if prefix_str.starts_with("tenant:") {
            let parts: Vec<&str> = prefix_str.splitn(3, ':').collect();
            if parts.len() == 3 {
                (Some(parts[1].to_string()), parts[2].trim_end_matches(':').to_string())
            } else {
                (None, prefix_str.trim_end_matches(':').to_string())
            }
        } else {
            (None, prefix_str.trim_end_matches(':').to_string())
        };
        
        let query = format!(
            r#"
            SELECT data FROM {}
            WHERE resource_type = $1
            AND ($2::TEXT IS NULL OR tenant_id = $2)
            AND deleted_at IS NULL
            ORDER BY created_at
            "#,
            self.table_name
        );
        
        let rows = sqlx::query(&query)
            .bind(&resource_type)
            .bind(&tenant_id)
            .fetch_all(&self.pool)
            .await?;
        
        let result: Vec<Value> = rows.into_iter()
            .map(|row| row.get("data"))
            .collect();
        
        let duration = start_time.elapsed();
        self.performance_monitor.record_operation("list", duration, true);
        
        Ok(result)
    }
    
    async fn delete(
        &self,
        key: StorageKey,
        _context: &RequestContext,
    ) -> Result<bool, Self::Error> {
        let start_time = std::time::Instant::now();
        
        let (tenant_id, _resource_type, resource_id) = self.extract_tenant_and_resource_info(&key);
        
        let result = self.with_transaction(|tx| {
            Box::pin(async move {
                // Soft delete by setting deleted_at timestamp
                let query = format!(
                    "UPDATE {} SET deleted_at = NOW() WHERE id = $1 AND ($2::TEXT IS NULL OR tenant_id = $2) AND deleted_at IS NULL",
                    self.table_name
                );
                
                let result = sqlx::query(&query)
                    .bind(&resource_id)
                    .bind(&tenant_id)
                    .execute(&mut **tx)
                    .await?;
                
                Ok(result.rows_affected() > 0)
            })
        }).await?;
        
        let duration = start_time.elapsed();
        self.performance_monitor.record_operation("delete", duration, true);
        
        Ok(result)
    }
    
    async fn exists(
        &self,
        key: StorageKey,
        _context: &RequestContext,
    ) -> Result<bool, Self::Error> {
        let start_time = std::time::Instant::now();
        
        let (tenant_id, _resource_type, resource_id) = self.extract_tenant_and_resource_info(&key);
        
        let query = format!(
            "SELECT 1 FROM {} WHERE id = $1 AND ($2::TEXT IS NULL OR tenant_id = $2) AND deleted_at IS NULL LIMIT 1",
            self.table_name
        );
        
        let result = sqlx::query(&query)
            .bind(&resource_id)
            .bind(&tenant_id)
            .fetch_optional(&self.pool)
            .await?;
        
        let duration = start_time.elapsed();
        self.performance_monitor.record_operation("exists", duration, true);
        
        Ok(result.is_some())
    }
}
```

### Pattern 2: Redis-Based Caching Storage

```rust
use redis::{Client, Commands, Connection, RedisResult};
use serde_json::{Value, json};

pub struct RedisStorageProvider {
    client: Client,
    connection_pool: deadpool_redis::Pool,
    key_prefix: String,
    default_ttl: Option<usize>,
    performance_monitor: Arc<PerformanceMonitor>,
}

impl RedisStorageProvider {
    pub async fn new(
        redis_url: &str,
        key_prefix: String,
        default_ttl: Option<usize>,
        pool_size: usize,
    ) -> Result<Self, RedisStorageError> {
        let client = Client::open(redis_url)?;
        
        let config = deadpool_redis::Config::from_url(redis_url);
        let pool = config.create_pool(Some(deadpool_redis::Runtime::Tokio1))?;
        
        Ok(Self {
            client,
            connection_pool: pool,
            key_prefix,
            default_ttl,
            performance_monitor: Arc::new(PerformanceMonitor::new()),
        })
    }
    
    fn make_redis_key(&self, key: &StorageKey) -> String {
        format!("{}:{}", self.key_prefix, key.as_str())
    }
    
    fn make_index_key(&self, prefix: &StoragePrefix) -> String {
        format!("{}:index:{}", self.key_prefix, prefix.as_str().trim_end_matches(':'))
    }
    
    async fn get_connection(&self) -> Result<deadpool_redis::Connection, RedisStorageError> {
        self.connection_pool.get().await
            .map_err(RedisStorageError::from)
    }
}

impl StorageProvider for RedisStorageProvider {
    type Error = RedisStorageError;
    
    async fn put(
        &self,
        key: StorageKey,
        mut data: Value,
        context: &RequestContext,
    ) -> Result<Value, Self::Error> {
        let start_time = std::time::Instant::now();
        
        let redis_key = self.make_redis_key(&key);
        let mut conn = self.get_connection().await?;
        
        // Add metadata
        data["meta"] = json!({
            "version": format!("W/\"{}\"", uuid::Uuid::new_v4()),
            "created": chrono::Utc::now().to_rfc3339(),
            "lastModified": chrono::Utc::now().to_rfc3339(),
        });
        
        let serialized = serde_json::to_string(&data)?;
        
        // Store the data
        if let Some(ttl) = self.default_ttl {
            redis::cmd("SETEX")
                .arg(&redis_key)
                .arg(ttl)
                .arg(&serialized)
                .query_async(&mut conn)
                .await?;
        } else {
            redis::cmd("SET")
                .arg(&redis_key)
                .arg(&serialized)
                .query_async(&mut conn)
                .await?;
        }
        
        // Update indexes for list operations
        let (tenant_id, resource_type, resource_id) = self.extract_key_parts(&key);
        if let Some(resource_type) = resource_type {
            let index_key = if let Some(tenant_id) = tenant_id {
                format!("{}:index:tenant:{}:{}", self.key_prefix, tenant_id, resource_type)
            } else {
                format!("{}:index:{}", self.key_prefix, resource_type)
            };
            
            redis::cmd("SADD")
                .arg(&index_key)
                .arg(&redis_key)
                .query_async(&mut conn)
                .await?;
                
            // Set TTL on index as well if configured
            if let Some(ttl) = self.default_ttl {
                redis::cmd("EXPIRE")
                    .arg(&index_key)
                    .arg(ttl * 2) // Indexes live longer than data
                    .query_async(&mut conn)
                    .await?;
            }
        }
        
        let duration = start_time.elapsed();
        self.performance_monitor.record_operation("put", duration, true);
        
        Ok(data)
    }
    
    async fn get(
        &self,
        key: StorageKey,
        _context: &RequestContext,
    ) -> Result<Option<Value>, Self::Error> {
        let start_time = std::time::Instant::now();
        
        let redis_key = self.make_redis_key(&key);
        let mut conn = self.get_connection().await?;
        
        let result: Option<String> = redis::cmd("GET")
            .arg(&redis_key)
            .query_async(&mut conn)
            .await?;
        
        let duration = start_time.elapsed();
        self.performance_monitor.record_operation("get", duration, true);
        
        match result {
            Some(data_str) => {
                let data: Value = serde_json::from_str(&data_str)?;
                Ok(Some(data))
            },
            None => Ok(None),
        }
    }
    
    async fn list(
        &self,
        prefix: StoragePrefix,
        _context: &RequestContext,
    ) -> Result<Vec<Value>, Self::Error> {
        let start_time = std::time::Instant::now();
        
        let index_key = self.make_index_key(&prefix);
        let mut conn = self.get_connection().await?;
        
        // Get all keys from the index
        let redis_keys: Vec<String> = redis::cmd("SMEMBERS")
            .arg(&index_key)
            .query_async(&mut conn)
            .await?;
        
        let mut results = Vec::new();
        
        // Batch get all values
        if !redis_keys.is_empty() {
            let values: Vec<Option<String>> = redis::cmd("MGET")
                .arg(&redis_keys)
                .query_async(&mut conn)
                .await?;
            
            for value in values.into_iter().flatten() {
                if let Ok(data) = serde_json::from_str::<Value>(&value) {
                    results.push(data);
                }
            }
        }
        
        let duration = start_time.elapsed();
        self.performance_monitor.record_operation("list", duration, true);
        
        Ok(results)
    }
    
    async fn delete(
        &self,
        key: StorageKey,
        _context: &RequestContext,
    ) -> Result<bool, Self::Error> {
        let start_time = std::time::Instant::now();
        
        let redis_key = self.make_redis_key(&key);
        let mut conn = self.get_connection().await?;
        
        // Remove from main storage
        let deleted: i32 = redis::cmd("DEL")
            .arg(&redis_key)
            .query_async(&mut conn)
            .await?;
        
        // Remove from indexes
        let (tenant_id, resource_type, _) = self.extract_key_parts(&key);
        if let Some(resource_type) = resource_type {
            let index_key = if let Some(tenant_id) = tenant_id {
                format!("{}:index:tenant:{}:{}", self.key_prefix, tenant_id, resource_type)
            } else {
                format!("{}:index:{}", self.key_prefix, resource_type)
            };
            
            redis::cmd("SREM")
                .arg(&index_key)
                .arg(&redis_key)
                .query_async(&mut conn)
                .await?;
        }
        
        let duration = start_time.elapsed();
        self.performance_monitor.record_operation("delete", duration, true);
        
        Ok(deleted > 0)
    }
    
    async fn exists(
        &self,
        key: StorageKey,
        _context: &RequestContext,
    ) -> Result<bool, Self::Error> {
        let start_time = std::time::Instant::now();
        
        let redis_key = self.make_redis_key(&key);
        let mut conn = self.get_connection().await?;
        
        let exists: bool = redis::cmd("EXISTS")
            .arg(&redis_key)
            .query_async(&mut conn)
            .await?;
        
        let duration = start_time.elapsed();
        self.performance_monitor.record_operation("exists", duration, true);
        
        Ok(exists)
    }
}
```

### Pattern 3: Multi-Level Caching Storage

```rust
pub struct MultiLevelCacheStorage<P: StorageProvider> {
    primary_storage: P,
    l1_cache: Arc<RwLock<lru::LruCache<String, CacheEntry>>>,
    l2_cache: Option<Arc<dyn L2Cache>>,
    cache_config: CacheConfig,
    performance_monitor: Arc<PerformanceMonitor>,
}

#[derive(Clone)]
struct CacheEntry {
    data: Value,
    cached_at: std::time::Instant,
    ttl: Duration,
    access_count: AtomicU64,
}

#[derive(Clone)]
pub struct CacheConfig {
    pub l1_size: usize,
    pub l1_ttl: Duration,
    pub l2_ttl: Duration,
    pub write_through: bool,
    pub write_behind: bool,
    pub write_behind_delay: Duration,
}

pub trait L2Cache: Send + Sync {
    async fn get(&self, key: &str) -> Result<Option<Value>, CacheError>;
    async fn put(&self, key: &str, value: &Value, ttl: Duration) -> Result<(), CacheError>;
    async fn delete(&self, key: &str) -> Result<bool, CacheError>;
}

impl<P: StorageProvider> MultiLevelCacheStorage<P> {
    pub fn new(primary_storage: P, config: CacheConfig) -> Self {
        Self {
            primary_storage,
            l1_cache: Arc::new(RwLock::new(lru::LruCache::new(config.l1_size))),
            l2_cache: None,
            cache_config: config,
            performance_monitor: Arc::new(PerformanceMonitor::new()),
        }
    }
    
    pub fn with_l2_cache(mut self, l2_cache: Arc<dyn L2Cache>) -> Self {
        self.l2_cache = Some(l2_cache);
        self
    }
    
    async fn get_from_l1(&self, key: &str) -> Option<Value> {
        let mut cache = self.l1_cache.write().unwrap();
        
        if let Some(entry) = cache.get_mut(key) {
            // Check TTL
            if entry.cached_at.elapsed() < entry.ttl {
                entry.access_count.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                return Some(entry.data.clone());
            } else {
                // Entry expired, remove it
                cache.pop(key);
            }
        }
        
        None
    }
    
    fn put_to_l1(&self, key: String, value: Value) {
        let entry = CacheEntry {
            data: value,
            cached_at: std::time::Instant::now(),
            ttl: self.cache_config.l1_ttl,
            access_count: AtomicU64::new(1),
        };
        
        let mut cache = self.l1_cache.write().unwrap();
        cache.put(key, entry);
    }
    
    async fn get_from_l2(&self, key: &str) -> Result<Option<Value>, CacheError> {
        if let Some(ref l2) = self.l2_cache {
            l2.get(key).await
        } else {
            Ok(None)
        }
    }
    
    async fn put_to_l2(&self, key: &str, value: &Value) -> Result<(), CacheError> {
        if let Some(ref l2) = self.l2_cache {
            l2.put(key, value, self.cache_config.l2_ttl).await
        } else {
            Ok(())
        }
    }
    
    async fn invalidate_cache(&self, key: &str) {
        // Remove from L1
        {
            let mut cache = self.l1_cache.write().unwrap();
            cache.pop(key);
        }
        
        // Remove from L2
        if let Some(ref l2) = self.l2_cache {
            let _ = l2.delete(key).await;
        }
    }
    
    fn make_cache_key(&self, storage_key: &StorageKey) -> String {
        format!("cache:{}", storage_key.as_str())
    }
}

impl<P: StorageProvider> StorageProvider for MultiLevelCacheStorage<P> {
    type Error = MultiLevelCacheError<P::Error>;
    
    async fn get(
        &self,
        key: StorageKey,
        context: &RequestContext,
    ) -> Result<Option<Value>, Self::Error> {
        let start_time = std::time::Instant::now();
        let cache_key = self.make_cache_key(&key);
        
        // Try L1 cache first
        if let Some(value) = self.get_from_l1(&cache_key).await {
            let duration = start_time.elapsed();
            self.performance_monitor.record_cache_hit("l1", duration);
            return Ok(Some(value));
        }
        
        // Try L2 cache
        if let Ok(Some(value)) = self.get_from_l2(&cache_key).await {
            // Promote to L1
            self.put_to_l1(cache_key.clone(), value.clone());
            
            let duration = start_time.elapsed();
            self.performance_monitor.record_cache_hit("l2", duration);
            return Ok(Some(value));
        }
        
        // Cache miss - fetch from primary storage
        let result = self.primary_storage.get(key, context).await
            .map_err(MultiLevelCacheError::StorageError)?;
        
        if let Some(ref value) = result {
            // Cache the result
            self.put_to_l1(cache_key.clone(), value.clone());
            let _ = self.put_to_l2(&cache_key, value).await;
        }
        
        let duration = start_time.elapsed();
        self.performance_monitor.record_cache_miss(duration);
        
        Ok(result)
    }
    
    async fn put(
        &self,
        key: StorageKey,
        data: Value,
        context: &RequestContext,
    ) -> Result<Value, Self::Error> {
        let start_time = std::time::Instant::now();
        let cache_key = self.make_cache_key(&key);
        
        if self.cache_config.write_through {
            // Write-through: write to storage first, then cache
            let result = self.primary_storage.put(key, data, context).await
                .map_err(MultiLevelCacheError::StorageError)?;
            
            // Update caches with the result
            self.put_to_l1(cache_key.clone(), result.clone());
            let _ = self.put_to_l2(&cache_key, &result).await;
            
            let duration = start_time.elapsed();
            self.performance_monitor.record_operation("put_write_through", duration, true);
            
            Ok(result)
        } else if self.cache_config.write_behind {
            // Write-behind: write to cache immediately, schedule storage write
            self.put_to_l1(cache_key.clone(), data.clone());
            let _ = self.put_to_l2(&cache_key, &data).await;
            
            // Schedule async write to primary storage
            let primary = self.primary_storage.clone();
            let write_key = key.clone();
            let write_data = data.clone();
            let write_context = context.clone();
            let delay = self.cache_config.write_behind_delay;
            
            tokio::spawn(async move {
                tokio::time::sleep(delay).await;
                let _ = primary.put(write_key, write_data, &write_context).await;
            });
            
            let duration = start_time.elapsed();
            self.performance_monitor.record_operation("put_write_behind", duration, true);
            
            Ok(data)
        } else {
            // Direct write-through to primary storage
            let result = self.primary_storage.put(key, data, context).await
                .map_err(MultiLevelCacheError::StorageError)?;
            
            // Invalidate cache to ensure consistency
            self.invalidate_cache(&cache_key).await;
            
            let duration = start_time.elapsed();
            self.performance_monitor.record_operation("put_direct", duration, true);
            
            Ok(result)
        }
    }
    
    async fn delete(
        &self,
        key: StorageKey,
        context: &RequestContext,
    ) -> Result<bool, Self::Error> {
        let start_time = std::time::Instant::now();
        let cache_key = self.make_cache_key(&key);
        
        // Delete from primary storage
        let result = self.primary_storage.delete(key, context).await
            .map_err(MultiLevelCacheError::StorageError)?;
        
        // Invalidate cache
        self.invalidate_cache(&cache_key).await;
        
        let duration = start_time.elapsed();
        self.performance_monitor.record_operation("delete", duration, true);
        
        Ok(result)
    }
    
    async fn list(
        &self,
        prefix: StoragePrefix,
        context: &RequestContext,
    ) -> Result<Vec<Value>, Self::Error> {
        // List operations typically bypass cache due to complexity
        // In production, you might cache list results with invalidation strategies
        self.primary_storage.list(prefix, context).await
            .map_err(MultiLevelCacheError::StorageError)
    }
    
    async fn exists(
        &self,
        key: StorageKey,
        context: &RequestContext,
    ) -> Result<bool, Self::Error> {
        let cache_key = self.make_cache_key(&key);
        
        // Check L1 cache first
        if self.get_from_l1(&cache_key).await.is_some() {
            return Ok(true);
        }
        
        // Check L2 cache
        if let Ok(Some(_)) = self.get_from_l2(&cache_key).await {
            return Ok(true);
        }
        
        // Check primary storage
        self.primary_storage.exists(key, context).await
            .map_err(MultiLevelCacheError::StorageError)
    }
}

// Redis-based L2 cache implementation
pub struct RedisL2Cache {
    pool: deadpool_redis::Pool,
    key_prefix: String,
}

impl RedisL2Cache {
    pub fn new(redis_url: &str, key_prefix: String) -> Result<Self, RedisError> {
        let config = deadpool_redis::Config::from_url(redis_url);
        let pool = config.create_pool(Some(deadpool_redis::Runtime::Tokio1))?;
        
        Ok(Self {
            pool,
            key_prefix,
        })
    }
}

impl L2Cache for RedisL2Cache {
    async fn get(&self, key: &str) -> Result<Option<Value>, CacheError> {
        let redis_key = format!("{}:l2:{}", self.key_prefix, key);
        let mut conn = self.pool.get().await?;
        
        let result: Option<String> = redis::cmd("GET")
            .arg(&redis_key)
            .query_async(&mut conn)
            .await?;
        
        match result {
            Some(data_str) => Ok(Some(serde_json::from_str(&data_str)?)),
            None => Ok(None),
        }
    }
    
    async fn put(&self, key: &str, value: &Value, ttl: Duration) -> Result<(), CacheError> {
        let redis_key = format!("{}:l2:{}", self.key_prefix, key);
        let mut conn = self.pool.get().await?;
        
        let serialized = serde_json::to_string(value)?;
        
        redis::cmd("SETEX")
            .arg(&redis_key)
            .arg(ttl.as_secs())
            .arg(&serialized)
            .query_async(&mut conn)
            .await?;
        
        Ok(())
    }
    
    async fn delete(&self, key: &str) -> Result<bool, CacheError> {
        let redis_key = format!("{}:l2:{}", self.key_prefix, key);
        let mut conn = self.pool.get().await?;
        
        let deleted: i32 = redis::cmd("DEL")
            .arg(&redis_key)
            .query_async(&mut conn)
            .await?;
        
        Ok(deleted > 0)
    }
}
```

## Advanced Storage Patterns

### Pattern 4: Event Sourcing with SCIM

```rust
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScimEvent {
    pub event_id: String,
    pub event_type: ScimEventType,
    pub aggregate_id: String,
    pub aggregate_type: String,
    pub tenant_id: Option<String>,
    pub event_data: Value,
    pub metadata: EventMetadata,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ScimEventType {
    ResourceCreated,
    ResourceUpdated,
    ResourceDeleted,
    ResourceRestored,
    SchemaRegistered,
    SchemaUpdated,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventMetadata {
    pub user_id: Option<String>,
    pub request_id: String,
    pub client_id: String,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
}

pub struct EventSourcedStorageProvider<E: EventStore> {
    event_store: E,
    snapshot_store: Box<dyn SnapshotStore>,
    projector: Arc<ScimProjector>,
    snapshot_frequency: usize,
    performance_monitor: Arc<PerformanceMonitor>,
}

pub trait EventStore: Send + Sync {
    type Error: std::error::Error + Send + Sync + 'static;
    
    async fn append_events(
        &self,
        stream_id: &str,
        events: Vec<ScimEvent>,
        expected_version: Option<u64>,
    ) -> Result<u64, Self::Error>;
    
    async fn read_events(
        &self,
        stream_id: &str,
        from_version: u64,
        max_count: Option<usize>,
    ) -> Result<Vec<ScimEvent>, Self::Error>;
    
    async fn read_all_events(
        &self,
        from_position: u64,
        max_count: Option<usize>,
    ) -> Result<Vec<ScimEvent>, Self::Error>;
}

pub trait SnapshotStore: Send + Sync {
    async fn save_snapshot(
        &self,
        aggregate_id: &str,
        version: u64,
        data: &Value,
    ) -> Result<(), SnapshotError>;
    
    async fn load_snapshot(
        &self,
        aggregate_id: &str,
    ) -> Result<Option<(u64, Value)>, SnapshotError>;
}

impl<E: EventStore> EventSourcedStorageProvider<E> {
    pub fn new(
        event_store: E,
        snapshot_store: Box<dyn SnapshotStore>,
        snapshot_frequency: usize,
    ) -> Self {
        Self {
            event_store,
            snapshot_store,
            projector: Arc::new(ScimProjector::new()),
            snapshot_frequency,
            performance_monitor: Arc::new(PerformanceMonitor::new()),
        }
    }
    
    async fn load_aggregate(
        &self,
        aggregate_id: &str,
    ) -> Result<Option<ScimAggregate>, EventStoreError> {
        let start_time = std::time::Instant::now();
        
        // Try to load from snapshot first
        let (start_version, mut aggregate) = match self.snapshot_store.load_snapshot(aggregate_id).await? {
            Some((version, data)) => (version + 1, Some(ScimAggregate::from_snapshot(data)?)),
            None => (0, None),
        };
        
        // Load events since snapshot
        let events = self.event_store.read_events(
            &format!("resource-{}", aggregate_id),
            start_version,
            None,
        ).await?;
        
        // Apply events to rebuild current state
        if !events.is_empty() {
            let mut current_aggregate = aggregate.unwrap_or_else(|| ScimAggregate::new(aggregate_id.to_string()));
            
            for event in events {
                current_aggregate = self.projector.apply_event(current_aggregate, &event)?;
            }
            
            aggregate = Some(current_aggregate);
        }
        
        let duration = start_time.elapsed();
        self.performance_monitor.record_operation("load_aggregate", duration, aggregate.is_some());
        
        Ok(aggregate)
    }
    
    async fn save_events_and_maybe_snapshot(
        &self,
        aggregate: &ScimAggregate,
        events: Vec<ScimEvent>,
    ) -> Result<(), EventStoreError> {
        let stream_id = format!("resource-{}", aggregate.id);
        
        // Append events
        let new_version = self.event_store.append_events(
            &stream_id,
            events,
            Some(aggregate.version),
        ).await?;
        
        // Save snapshot if needed
        if new_version % self.snapshot_frequency as u64 == 0 {
            self.snapshot_store.save_snapshot(
                &aggregate.id,
                new_version,
                &aggregate.to_snapshot()?,
            ).await?;
        }
        
        Ok(())
    }
}

impl<E: EventStore> StorageProvider for EventSourcedStorageProvider<E> {
    type Error = EventStoreError;
    
    async fn put(
        &self,
        key: StorageKey,
        data: Value,
        context: &RequestContext,
    ) -> Result<Value, Self::Error> {
        let (tenant_id, resource_type, resource_id) = self.extract_key_parts(&key);
        
        // Load existing aggregate
        let mut aggregate = self.load_aggregate(&resource_id).await?
            .unwrap_or_else(|| ScimAggregate::new(resource_id.clone()));
        
        // Determine event type
        let event_type = if aggregate.version == 0 {
            ScimEventType::ResourceCreated
        } else {
            ScimEventType::ResourceUpdated
        };
        
        // Create event
        let event = ScimEvent {
            event_id: uuid::Uuid::new_v4().to_string(),
            event_type,
            aggregate_id: resource_id.clone(),
            aggregate_type: resource_type.unwrap_or_else(|| "Unknown".to_string()),
            tenant_id: tenant_id.clone(),
            event_data: data.clone(),
            metadata: EventMetadata {
                user_id: context.user_id(),
                request_id: context.request_id.clone(),
                client_id: context.client_id().unwrap_or_else(|| "unknown".to_string()),
                ip_address: None, // Would be extracted from HTTP context in real implementation
                user_agent: None,
            },
            timestamp: Utc::now(),
        };
        
        // Apply event to aggregate
        aggregate = self.projector.apply_event(aggregate, &event)?;
        
        // Save event and maybe snapshot
        self.save_events_and_maybe_snapshot(&aggregate, vec![event]).await?;
        
        // Return the current state
        Ok(aggregate.current_data)
    }
    
    async fn get(
        &self,
        key: StorageKey,
        _context: &RequestContext,
    ) -> Result<Option<Value>, Self::Error> {
        let (_, _, resource_id) = self.extract_key_parts(&key);
        
        let aggregate = self.load_aggregate(&resource_id).await?;
        Ok(aggregate.map(|a| a.current_data))
    }
    
    async fn delete(
        &self,
        key: StorageKey,
        context: &RequestContext,
    ) -> Result<bool, Self::Error> {
        let (tenant_id, resource_type, resource_id) = self.extract_key_parts(&key);
        
        // Load aggregate to check if it exists
        let aggregate = match self.load_aggregate(&resource_id).await? {
            Some(agg) if !agg.is_deleted => agg,
            _ => return Ok(false), // Already deleted or doesn't exist
        };
        
        // Create deletion event
        let event = ScimEvent {
            event_id: uuid::Uuid::new_v4().to_string(),
            event_type: ScimEventType::ResourceDeleted,
            aggregate_id: resource_id.clone(),
            aggregate_type: resource_type.unwrap_or_else(|| "Unknown".to_string()),
            tenant_id: tenant_id.clone(),
            event_data: json!({"deleted": true}),
            metadata: EventMetadata {
                user_id: context.user_id(),
                request_id: context.request_id.clone(),
                client_id: context.client_id().unwrap_or_else(|| "unknown".to_string()),
                ip_address: None,
                user_agent: None,
            },
            timestamp: Utc::now(),
        };
        
        // Apply event and save
        let updated_aggregate = self.projector.apply_event(aggregate, &event)?;
        self.save_events_and_maybe_snapshot(&updated_aggregate, vec![event]).await?;
        
        Ok(true)
    }
    
    // List and exists operations would need to be implemented with projections
    // This is simplified for brevity
    async fn list(
        &self,
        _prefix: StoragePrefix,
        _context: &RequestContext,
    ) -> Result<Vec<Value>, Self::Error> {
        // In a real implementation, this would use read-model projections
        todo!("List operations require read-model projections")
    }
    
    async fn exists(
        &self,
        key: StorageKey,
        context: &RequestContext,
    ) -> Result<bool, Self::Error> {
        Ok(self.get(key, context).await?.is_some())
    }
}

#[derive(Debug, Clone)]
pub struct ScimAggregate {
    pub id: String,
    pub version: u64,
    pub current_data: Value,
    pub is_deleted: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl ScimAggregate {
    pub fn new(id: String) -> Self {
        Self {
            id,
            version: 0,
            current_data: json!({}),
            is_deleted: false,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }
    
    pub fn from_snapshot(data: Value) -> Result<Self, serde_json::Error> {
        serde_json::from_value(data)
    }
    
    pub fn to_snapshot(&self) -> Result<Value, serde_json::Error> {
        serde_json::to_value(self)
    }
}

pub struct ScimProjector;

impl ScimProjector {
    pub fn new() -> Self {
        Self
    }
    
    pub fn apply_event(
        &self,
        mut aggregate: ScimAggregate,
        event: &ScimEvent,
    ) -> Result<ScimAggregate, ProjectionError> {
        match event.event_type {
            ScimEventType::ResourceCreated => {
                aggregate.current_data = event.event_data.clone();
                aggregate.created_at = event.timestamp;
                aggregate.updated_at = event.timestamp;
            },
            ScimEventType::ResourceUpdated => {
                // Merge the update data
                if let (Value::Object(ref mut current), Value::Object(update)) = 
                    (&mut aggregate.current_data, &event.event_data) {
                    for (key, value) in update {
                        current.insert(key.clone(), value.clone());
                    }
                }
                aggregate.updated_at = event.timestamp;
            },
            ScimEventType::ResourceDeleted => {
                aggregate.is_deleted = true;
                aggregate.updated_at = event.timestamp;
            },
            ScimEventType::ResourceRestored => {
                aggregate.is_deleted = false;
                aggregate.updated_at = event.timestamp;
            },
            _ => {} // Other event types handled elsewhere
        }
        
        aggregate.version += 1;
        Ok(aggregate)
    }
}
```

## Performance Monitoring and Observability

```rust
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::time::{Duration, Instant};
use tracing::{info, warn, error};

#[derive(Debug)]
pub struct PerformanceMonitor {
    metrics: PerformanceMetrics,
    slow_query_threshold: Duration,
    error_rate_threshold: f64,
}

#[derive(Debug, Default)]
pub struct PerformanceMetrics {
    pub total_operations: AtomicU64,
    pub successful_operations: AtomicU64,
    pub failed_operations: AtomicU64,
    pub total_duration: AtomicU64, // in nanoseconds
    pub cache_hits: AtomicU64,
    pub cache_misses: AtomicU64,
    pub connection_pool_size: AtomicUsize,
    pub active_connections: AtomicUsize,
}

impl PerformanceMonitor {
    pub fn new() -> Self {
        Self {
            metrics: PerformanceMetrics::default(),
            slow_query_threshold: Duration::from_millis(100),
            error_rate_threshold: 0.05, // 5%
        }
    }
    
    pub fn record_operation(&self, operation: &str, duration: Duration, success: bool) {
        self.metrics.total_operations.fetch_add(1, Ordering::Relaxed);
        self.metrics.total_duration.fetch_add(duration.as_nanos() as u64, Ordering::Relaxed);
        
        if success {
            self.metrics.successful_operations.fetch_add(1, Ordering::Relaxed);
        } else {
            self.metrics.failed_operations.fetch_add(1, Ordering::Relaxed);
        }
        
        // Log slow operations
        if duration > self.slow_query_threshold {
            warn!(
                operation = operation,
                duration_ms = duration.as_millis(),
                "Slow storage operation detected"
            );
        }
        
        // Check error rate
        let total = self.metrics.total_operations.load(Ordering::Relaxed);
        let failed = self.metrics.failed_operations.load(Ordering::Relaxed);
        
        if total > 0 {
            let error_rate = failed as f64 / total as f64;
            if error_rate > self.error_rate_threshold {
                error!(
                    operation = operation,
                    error_rate = error_rate,
                    threshold = self.error_rate_threshold,
                    "High error rate detected"
                );
            }
        }
    }
    
    pub fn record_cache_hit(&self, level: &str, duration: Duration) {
        self.metrics.cache_hits.fetch_add(1, Ordering::Relaxed);
        
        info!(
            cache_level = level,
            duration_μs = duration.as_micros(),
            "Cache hit"
        );
    }
    
    pub fn record_cache_miss(&self, duration: Duration) {
        self.metrics.cache_misses.fetch_add(1, Ordering::Relaxed);
        
        info!(
            duration_ms = duration.as_millis(),
            "Cache miss"
        );
    }
    
    pub fn get_metrics_snapshot(&self) -> MetricsSnapshot {
        let total_ops = self.metrics.total_operations.load(Ordering::Relaxed);
        let successful_ops = self.metrics.successful_operations.load(Ordering::Relaxed);
        let failed_ops = self.metrics.failed_operations.load(Ordering::Relaxed);
        let total_duration_ns = self.metrics.total_duration.load(Ordering::Relaxed);
        let cache_hits = self.metrics.cache_hits.load(Ordering::Relaxed);
        let cache_misses = self.metrics.cache_misses.load(Ordering::Relaxed);
        
        MetricsSnapshot {
            total_operations: total_ops,
            successful_operations: successful_ops,
            failed_operations: failed_ops,
            error_rate: if total_ops > 0 { failed_ops as f64 / total_ops as f64 } else { 0.0 },
            average_duration: if total_ops > 0 { 
                Duration::from_nanos(total_duration_ns / total_ops) 
            } else { 
                Duration::ZERO 
            },
            cache_hit_rate: if (cache_hits + cache_misses) > 0 {
                cache_hits as f64 / (cache_hits + cache_misses) as f64
            } else { 
                0.0 
            },
            active_connections: self.metrics.active_connections.load(Ordering::Relaxed),
            pool_size: self.metrics.connection_pool_size.load(Ordering::Relaxed),
        }
    }
}

#[derive(Debug, Clone)]
pub struct MetricsSnapshot {
    pub total_operations: u64,
    pub successful_operations: u64,
    pub failed_operations: u64,
    pub error_rate: f64,
    pub average_duration: Duration,
    pub cache_hit_rate: f64,
    pub active_connections: usize,
    pub pool_size: usize,
}

impl MetricsSnapshot {
    pub fn log_summary(&self) {
        info!(
            total_operations = self.total_operations,
            error_rate = %format!("{:.2}%", self.error_rate * 100.0),
            avg_duration_ms = self.average_duration.as_millis(),
            cache_hit_rate = %format!("{:.2}%", self.cache_hit_rate * 100.0),
            active_connections = self.active_connections,
            pool_size = self.pool_size,
            "Storage performance metrics"
        );
    }
}
```

## Best Practices Summary

### Storage Selection Guidelines

1. **Development and Testing**
   - Use `InMemoryStorage` for unit tests and local development
   - Quick to set up, no external dependencies
   - Perfect for prototyping and CI/CD pipelines

2. **Production Database Storage**
   - Use PostgreSQL for ACID compliance and complex queries
   - Use MongoDB for document-oriented flexibility
   - Use Redis for high-performance caching and session storage

3. **Hybrid Approaches**
   - Multi-level caching for read-heavy workloads
   - Event sourcing for audit trails and complex business logic
   - CQRS for separating read/write optimization

### Performance Optimization

1. **Connection Management**
   - Use connection pooling for database storage
   - Configure appropriate pool sizes and timeouts
   - Monitor connection usage and adjust as needed

2. **Caching Strategies**
   - Implement L1 (in-memory) cache for hot data
   - Use L2 (Redis) cache for shared/distributed caching
   - Choose appropriate TTLs based on data volatility

3. **Query Optimization**
   - Index frequently queried fields (tenant_id, resource_type)
   - Use batch operations where possible
   - Implement pagination for large result sets

4. **Monitoring and Alerting**
   - Track key metrics (latency, error rate, cache hit ratio)
   - Set up alerts for performance degradation
   - Use distributed tracing for complex request flows

## Related Topics

- **[Resource Provider Architecture](./resource-provider-architecture.md)** - How storage integrates with business logic
- **[Multi-Tenant Architecture Patterns](./multi-tenant-patterns.md)** - Tenant-specific storage isolation
- **[Storage Providers](../concepts/storage-providers.md)** - Core storage concepts and interfaces

## Next Steps

Now that you understand storage and persistence patterns:

1. **Choose your storage backend** based on scalability and consistency requirements
2. **Implement appropriate caching** for your read/write patterns  
3. **Set up monitoring and alerting** for production operations
4. **Consider advanced patterns** like event sourcing for audit requirements
5. **Plan for data migration** and backup/recovery strategies