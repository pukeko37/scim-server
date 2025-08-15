# Concurrency Overview

This guide provides an overview of concurrency management in the SCIM Server library. Understanding concurrency control is essential for building reliable, multi-user SCIM deployments that maintain data consistency under load.

## What is Concurrency Control?

Concurrency control ensures that simultaneous operations on shared resources don't interfere with each other or corrupt data. In SCIM servers, this is particularly important when multiple clients are:

- Modifying the same user or group
- Creating resources with unique constraints
- Performing bulk operations
- Running in distributed deployments

## SCIM 2.0 Concurrency Model

### ETags and Optimistic Concurrency

SCIM 2.0 uses ETags (Entity Tags) to implement optimistic concurrency control:

1. **ETag Generation**: Each resource gets a unique version identifier
2. **Client Requests**: Clients include ETags in conditional requests
3. **Version Checking**: Server validates ETags before modifications
4. **Conflict Detection**: Mismatched ETags indicate concurrent modifications

```rust
use scim_server::concurrency::{ETagManager, VersionControl};
use scim_server::models::{User, Meta};
use chrono::Utc;

pub struct ETagManager {
    hash_algorithm: HashAlgorithm,
    weak_etags: bool,
}

impl ETagManager {
    pub fn new() -> Self {
        Self {
            hash_algorithm: HashAlgorithm::Sha256,
            weak_etags: true, // SCIM typically uses weak ETags
        }
    }

    pub fn generate_etag(&self, resource: &dyn Resource) -> String {
        let content = self.serialize_for_etag(resource);
        let hash = self.calculate_hash(&content);
        let timestamp = Utc::now().timestamp_millis();
        
        if self.weak_etags {
            format!("W/\"{}-{}\"", hash, timestamp)
        } else {
            format!("\"{}-{}\"", hash, timestamp)
        }
    }

    pub fn parse_etag(&self, etag: &str) -> Result<ETagInfo, ETagError> {
        let is_weak = etag.starts_with("W/");
        let tag_value = if is_weak {
            &etag[3..etag.len()-1] // Remove W/" and "
        } else {
            &etag[1..etag.len()-1] // Remove " and "
        };

        let parts: Vec<&str> = tag_value.split('-').collect();
        if parts.len() != 2 {
            return Err(ETagError::InvalidFormat);
        }

        Ok(ETagInfo {
            hash: parts[0].to_string(),
            timestamp: parts[1].parse()?,
            is_weak,
        })
    }
}

pub struct ETagInfo {
    pub hash: String,
    pub timestamp: i64,
    pub is_weak: bool,
}
```

### HTTP Conditional Headers

SCIM uses standard HTTP conditional headers for concurrency control:

- **If-Match**: Proceed only if ETag matches (for updates/deletes)
- **If-None-Match**: Proceed only if ETag doesn't match (for creates)
- **If-Modified-Since**: Check modification time
- **If-Unmodified-Since**: Check that resource hasn't been modified

```rust
#[derive(Debug, Clone)]
pub struct ConditionalRequest {
    pub if_match: Option<String>,
    pub if_none_match: Option<String>,
    pub if_modified_since: Option<DateTime<Utc>>,
    pub if_unmodified_since: Option<DateTime<Utc>>,
}

impl ConditionalRequest {
    pub fn from_headers(headers: &HeaderMap) -> Self {
        Self {
            if_match: headers.get("If-Match")
                .and_then(|v| v.to_str().ok())
                .map(|s| s.to_string()),
            if_none_match: headers.get("If-None-Match")
                .and_then(|v| v.to_str().ok())
                .map(|s| s.to_string()),
            if_modified_since: headers.get("If-Modified-Since")
                .and_then(|v| v.to_str().ok())
                .and_then(|s| DateTime::parse_from_rfc2822(s).ok())
                .map(|dt| dt.with_timezone(&Utc)),
            if_unmodified_since: headers.get("If-Unmodified-Since")
                .and_then(|v| v.to_str().ok())
                .and_then(|s| DateTime::parse_from_rfc2822(s).ok())
                .map(|dt| dt.with_timezone(&Utc)),
        }
    }
}
```

## Concurrency Strategies

### 1. Optimistic Concurrency Control

Assumes conflicts are rare and checks for them before committing:

```rust
pub struct OptimisticConcurrencyManager {
    etag_manager: ETagManager,
    conflict_resolver: Box<dyn ConflictResolver>,
}

impl OptimisticConcurrencyManager {
    pub async fn update_resource<T: Resource>(
        &self,
        storage: &dyn StorageProvider,
        tenant_id: &str,
        resource_id: &str,
        updated_resource: T,
        conditions: &ConditionalRequest,
    ) -> Result<T, ConcurrencyError> {
        // Get current resource
        let current = storage.get_resource(tenant_id, resource_id).await?;
        
        // Validate conditions
        self.validate_conditions(&current, conditions)?;
        
        // Attempt update
        match storage.update_resource(tenant_id, resource_id, updated_resource).await {
            Ok(result) => Ok(result),
            Err(StorageError::VersionMismatch) => {
                // Handle conflict
                self.handle_version_conflict(storage, tenant_id, resource_id, updated_resource).await
            }
            Err(e) => Err(ConcurrencyError::StorageError(e)),
        }
    }

    fn validate_conditions(
        &self,
        resource: &dyn Resource,
        conditions: &ConditionalRequest,
    ) -> Result<(), ConcurrencyError> {
        // Validate If-Match
        if let Some(if_match) = &conditions.if_match {
            let current_etag = resource.get_etag();
            if !self.etags_match(if_match, &current_etag) {
                return Err(ConcurrencyError::PreconditionFailed("If-Match failed".to_string()));
            }
        }

        // Validate If-None-Match
        if let Some(if_none_match) = &conditions.if_none_match {
            let current_etag = resource.get_etag();
            if self.etags_match(if_none_match, &current_etag) {
                return Err(ConcurrencyError::NotModified);
            }
        }

        // Validate If-Unmodified-Since
        if let Some(if_unmodified_since) = conditions.if_unmodified_since {
            if let Some(last_modified) = resource.get_last_modified() {
                if last_modified > if_unmodified_since {
                    return Err(ConcurrencyError::PreconditionFailed("Resource was modified".to_string()));
                }
            }
        }

        Ok(())
    }
}
```

### 2. Pessimistic Concurrency Control

Uses locks to prevent concurrent access:

```rust
pub trait LockManager: Send + Sync {
    async fn acquire_lock(&self, resource_id: &str, lock_type: LockType, timeout: Duration) -> Result<Lock, LockError>;
    async fn release_lock(&self, lock: Lock) -> Result<(), LockError>;
    async fn extend_lock(&self, lock: &mut Lock, duration: Duration) -> Result<(), LockError>;
}

#[derive(Debug, Clone)]
pub enum LockType {
    Shared,    // Multiple readers
    Exclusive, // Single writer
}

pub struct Lock {
    pub id: String,
    pub resource_id: String,
    pub lock_type: LockType,
    pub acquired_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub owner: String,
}

pub struct PessimisticConcurrencyManager {
    lock_manager: Box<dyn LockManager>,
    default_timeout: Duration,
}

impl PessimisticConcurrencyManager {
    pub async fn with_exclusive_lock<T, F, Fut>(
        &self,
        resource_id: &str,
        operation: F,
    ) -> Result<T, ConcurrencyError>
    where
        F: FnOnce() -> Fut,
        Fut: Future<Output = Result<T, ConcurrencyError>>,
    {
        let lock = self.lock_manager
            .acquire_lock(resource_id, LockType::Exclusive, self.default_timeout)
            .await?;

        let result = operation().await;

        self.lock_manager.release_lock(lock).await?;
        result
    }
}
```

### 3. Hybrid Approach

Combines optimistic and pessimistic strategies:

```rust
pub struct HybridConcurrencyManager {
    optimistic: OptimisticConcurrencyManager,
    pessimistic: PessimisticConcurrencyManager,
    conflict_threshold: usize,
    conflict_tracker: ConflictTracker,
}

impl HybridConcurrencyManager {
    pub async fn update_resource<T: Resource>(
        &self,
        resource_id: &str,
        update_fn: impl FnOnce(&T) -> Result<T, ConcurrencyError>,
    ) -> Result<T, ConcurrencyError> {
        let conflict_count = self.conflict_tracker.get_recent_conflicts(resource_id).await;
        
        if conflict_count > self.conflict_threshold {
            // High contention - use pessimistic locking
            self.pessimistic.with_exclusive_lock(resource_id, || async {
                // Perform update within lock
                self.optimistic.update_resource(resource_id, update_fn).await
            }).await
        } else {
            // Low contention - use optimistic approach
            match self.optimistic.update_resource(resource_id, update_fn).await {
                Ok(result) => Ok(result),
                Err(ConcurrencyError::VersionMismatch) => {
                    // Record conflict and retry with lock
                    self.conflict_tracker.record_conflict(resource_id).await;
                    self.pessimistic.with_exclusive_lock(resource_id, || async {
                        self.optimistic.update_resource(resource_id, update_fn).await
                    }).await
                }
                Err(e) => Err(e),
            }
        }
    }
}
```

## Distributed Concurrency

### Redis-Based Distributed Locking

```rust
use redis::{Client, Commands};

pub struct RedisLockManager {
    client: Client,
    lock_prefix: String,
    default_ttl: Duration,
}

impl RedisLockManager {
    pub fn new(redis_url: &str) -> Result<Self, LockError> {
        Ok(Self {
            client: Client::open(redis_url)?,
            lock_prefix: "scim:lock:".to_string(),
            default_ttl: Duration::from_secs(30),
        })
    }

    fn lock_key(&self, resource_id: &str) -> String {
        format!("{}{}", self.lock_prefix, resource_id)
    }
}

#[async_trait]
impl LockManager for RedisLockManager {
    async fn acquire_lock(
        &self,
        resource_id: &str,
        lock_type: LockType,
        timeout: Duration,
    ) -> Result<Lock, LockError> {
        let mut conn = self.client.get_async_connection().await?;
        let lock_key = self.lock_key(resource_id);
        let lock_value = uuid::Uuid::new_v4().to_string();
        let ttl_seconds = self.default_ttl.as_secs() as usize;

        let start = Instant::now();
        
        loop {
            // Try to acquire lock using SET NX EX
            let result: Option<String> = conn.set_nx_ex(&lock_key, &lock_value, ttl_seconds).await?;
            
            if result.is_some() {
                return Ok(Lock {
                    id: lock_value,
                    resource_id: resource_id.to_string(),
                    lock_type,
                    acquired_at: Utc::now(),
                    expires_at: Utc::now() + chrono::Duration::from_std(self.default_ttl).unwrap(),
                    owner: "current_process".to_string(),
                });
            }

            if start.elapsed() >= timeout {
                return Err(LockError::Timeout);
            }

            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    }

    async fn release_lock(&self, lock: Lock) -> Result<(), LockError> {
        let mut conn = self.client.get_async_connection().await?;
        let lock_key = self.lock_key(&lock.resource_id);

        // Use Lua script for atomic check-and-delete
        let script = r#"
            if redis.call("GET", KEYS[1]) == ARGV[1] then
                return redis.call("DEL", KEYS[1])
            else
                return 0
            end
        "#;

        let result: i32 = redis::Script::new(script)
            .key(&lock_key)
            .arg(&lock.id)
            .invoke_async(&mut conn)
            .await?;

        if result == 1 {
            Ok(())
        } else {
            Err(LockError::LockNotFound)
        }
    }
}
```

## Error Handling

### Concurrency-Related Errors

```rust
#[derive(Debug, thiserror::Error)]
pub enum ConcurrencyError {
    #[error("Version mismatch detected")]
    VersionMismatch,
    
    #[error("Precondition failed: {0}")]
    PreconditionFailed(String),
    
    #[error("Resource not modified")]
    NotModified,
    
    #[error("Lock acquisition failed: {0}")]
    LockFailed(String),
    
    #[error("Lock timeout exceeded")]
    LockTimeout,
    
    #[error("Deadlock detected")]
    Deadlock,
    
    #[error("Conflict resolution failed: {0}")]
    ConflictResolutionFailed(String),
    
    #[error("Storage error: {0}")]
    StorageError(#[from] StorageError),
}

impl From<ConcurrencyError> for ScimError {
    fn from(error: ConcurrencyError) -> Self {
        match error {
            ConcurrencyError::VersionMismatch => ScimError::PreconditionFailed,
            ConcurrencyError::PreconditionFailed(_) => ScimError::PreconditionFailed,
            ConcurrencyError::NotModified => ScimError::NotModified,
            ConcurrencyError::LockTimeout => ScimError::TooManyRequests,
            _ => ScimError::InternalServerError,
        }
    }
}
```

## Performance Considerations

### ETag Optimization

```rust
pub struct OptimizedETagManager {
    cache: Arc<RwLock<LruCache<String, String>>>,
    hash_cache_size: usize,
}

impl OptimizedETagManager {
    pub fn generate_etag_cached(&self, resource: &dyn Resource) -> String {
        let resource_key = format!("{}:{}", resource.get_type(), resource.get_id());
        
        // Check cache first
        {
            let cache = self.cache.read().unwrap();
            if let Some(cached_etag) = cache.peek(&resource_key) {
                // Verify resource hasn't changed
                if self.resource_unchanged(resource, cached_etag) {
                    return cached_etag.clone();
                }
            }
        }
        
        // Generate new ETag
        let new_etag = self.generate_fresh_etag(resource);
        
        // Update cache
        {
            let mut cache = self.cache.write().unwrap();
            cache.put(resource_key, new_etag.clone());
        }
        
        new_etag
    }
}
```

### Lock Granularity

Choose appropriate lock granularity for your use case:

- **Resource-level**: Lock individual users/groups (fine-grained)
- **Tenant-level**: Lock entire tenant (coarse-grained)
- **Attribute-level**: Lock specific attributes (very fine-grained)
- **Operation-level**: Lock based on operation type

## Best Practices

### 1. ETag Management
- Always include ETags in resource metadata
- Use weak ETags for flexibility
- Cache ETag calculations for performance
- Implement ETag validation consistently

### 2. Lock Management
- Keep lock duration minimal
- Implement lock timeouts
- Use deadlock detection
- Consider lock hierarchies

### 3. Conflict Resolution
- Provide meaningful error messages
- Implement retry strategies
- Log concurrency conflicts
- Monitor conflict rates

### 4. Performance
- Use appropriate concurrency strategy for load patterns
- Monitor lock contention
- Optimize for common cases
- Consider eventual consistency where appropriate

## Next Steps

- [Implementation](./implementation.md) - Learn to implement concurrency control
- [Conflict Resolution](./conflict-resolution.md) - Handle conflicts gracefully