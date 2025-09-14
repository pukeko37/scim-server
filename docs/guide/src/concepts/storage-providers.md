# Storage Providers

Storage Providers form the data persistence layer of the SCIM Server architecture, providing a clean abstraction between SCIM protocol logic and data storage implementation. They handle pure data operations on JSON resources while remaining completely agnostic to SCIM semantics.

See the [StorageProvider API documentation](https://docs.rs/scim-server/latest/scim_server/storage/trait.StorageProvider.html) for complete details.

## Value Proposition

Storage Providers deliver focused data persistence capabilities:

- **Clean Separation**: Pure data operations isolated from SCIM business logic
- **Storage Agnostic**: Unified interface works with any storage backend
- **Simple Operations**: Focused on PUT/GET/DELETE with minimal complexity
- **Tenant Isolation**: Built-in support for multi-tenant data organization
- **Performance Optimized**: Direct storage operations without protocol overhead
- **Pluggable Backends**: Easy to swap storage implementations

## Architecture Overview

Storage Providers operate at the lowest level of the SCIM Server stack:

```text
Resource Provider (Business Logic)
    ↓
Storage Provider (Data Persistence)
├── PUT/GET/DELETE Operations
├── JSON Document Storage
├── Tenant Key Organization
├── Basic Querying & Filtering
└── Backend Implementation
    ↓ (examples)
├── InMemoryStorage
├── SqliteStorage
└── CustomStorage
```

### Design Philosophy

The storage layer follows a fundamental principle: **at the storage level, CREATE and UPDATE are the same operation**. You're simply putting data at a location. The distinction between "create" vs "update" is business logic that belongs in the Resource Provider layer.

This design provides several benefits:
- **Simplicity**: Fewer operations to implement and understand
- **Consistency**: Same operation semantics regardless of whether data exists
- **Performance**: No need to check existence before operations
- **Flexibility**: Storage backends can optimize PUT operations as needed

## Core Interface

The [`StorageProvider` trait](https://docs.rs/scim-server/latest/scim_server/storage/trait.StorageProvider.html) defines the contract for data persistence:

```rust
pub trait StorageProvider: Send + Sync {
    type Error: std::error::Error + Send + Sync + 'static;

    // Core data operations
    async fn put(&self, key: StorageKey, data: Value) -> Result<Value, Self::Error>;
    async fn get(&self, key: StorageKey) -> Result<Option<Value>, Self::Error>;
    async fn delete(&self, key: StorageKey) -> Result<bool, Self::Error>;
    
    // Query operations
    async fn list(&self, prefix: StoragePrefix, start_index: usize, count: usize) 
        -> Result<Vec<(StorageKey, Value)>, Self::Error>;
    async fn find_by_attribute(&self, prefix: StoragePrefix, 
        attribute_name: &str, attribute_value: &str) 
        -> Result<Vec<(StorageKey, Value)>, Self::Error>;
    
    // Utility operations
    async fn exists(&self, key: StorageKey) -> Result<bool, Self::Error>;
    async fn count(&self, prefix: StoragePrefix) -> Result<usize, Self::Error>;
    async fn clear(&self) -> Result<(), Self::Error>;
    
    // Discovery operations
    async fn list_tenants(&self) -> Result<Vec<String>, Self::Error>;
    async fn list_resource_types(&self, tenant_id: &str) -> Result<Vec<String>, Self::Error>;
    async fn list_all_resource_types(&self) -> Result<Vec<String>, Self::Error>;
    
    // Statistics
    async fn stats(&self) -> Result<StorageStats, Self::Error>;
}
```

### Key Organization

Storage uses hierarchical keys for tenant and resource type isolation:

```rust
pub struct StorageKey {
    tenant_id: String,      // "tenant-123"
    resource_type: String,  // "User" or "Group"
    resource_id: String,    // "user-456"
}

// Examples:
// tenant-123/User/user-456
// tenant-123/Group/group-789
// default/User/admin-user
```

## Available Implementations

### 1. InMemoryStorage

**Perfect for development, testing, and proof-of-concepts**

```rust
use scim_server::storage::InMemoryStorage;

let storage = InMemoryStorage::new();

// Benefits:
// - Instant startup
// - No external dependencies
// - Perfect for unit tests
// - High performance

// Limitations:
// - Data lost on restart
// - Memory usage grows with data
// - Single-process only
```

**Use Cases:**
- Unit and integration testing
- Development environments
- Demos and prototypes
- Temporary data scenarios

### 2. SqliteStorage

**Production-ready persistence with zero configuration**

```rust
use scim_server::storage::SqliteStorage;

let storage = SqliteStorage::new("scim_data.db").await?;

// Benefits:
// - File-based persistence
// - No server setup required
// - ACID transactions
// - Excellent performance
// - Cross-platform

// Limitations:
// - Single-writer concurrency
// - File-system dependent
// - Size limitations for very large datasets
```

**Use Cases:**
- Single-server deployments
- Small to medium scale applications
- Desktop applications
- Edge computing scenarios

### 3. Custom Storage Implementations

**Extend to any backend you need**

```rust
use scim_server::storage::StorageProvider;

pub struct RedisStorage {
    client: redis::Client,
}

impl StorageProvider for RedisStorage {
    type Error = redis::RedisError;

    async fn put(&self, key: StorageKey, data: Value) -> Result<Value, Self::Error> {
        let redis_key = format!("{}/{}/{}", key.tenant_id(), key.resource_type(), key.resource_id());
        let json_string = data.to_string();
        
        self.client.set(&redis_key, &json_string).await?;
        Ok(data) // Return what was stored
    }

    // ... implement other methods
}
```

## Use Cases

### 1. Development and Testing

**Rapid iteration with in-memory storage**

```rust
use scim_server::storage::InMemoryStorage;
use scim_server::providers::StandardResourceProvider;

#[tokio::test]
async fn test_user_operations() {
    // Setup - instant, no external dependencies
    let storage = InMemoryStorage::new();
    let provider = StandardResourceProvider::new(storage);
    
    // Test operations
    let context = RequestContext::with_generated_id();
    let user = provider.create_resource("User", user_data, &context).await?;
    
    // Clean slate for each test
    provider.clear().await;
    
    assert_eq!(provider.get_stats().await.total_resources, 0);
}
```

**Benefits**: Fast test execution, isolated test environments, no cleanup required.

### 2. Single-Server Production

**Persistent storage without infrastructure complexity**

```rust
use scim_server::storage::SqliteStorage;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Production-ready storage with single file
    let storage = SqliteStorage::new("/var/lib/scim/users.db").await?;
    let provider = StandardResourceProvider::new(storage);
    
    // Data persists across application restarts
    let server = ScimServer::new(provider);
    server.run("0.0.0.0:8080").await?;
    
    Ok(())
}
```

**Benefits**: Data persistence, ACID guarantees, simple deployment.

### 3. Distributed Systems

**Custom storage for scalability**

```rust
pub struct CassandraStorage {
    session: Arc<Session>,
    keyspace: String,
}

impl StorageProvider for CassandraStorage {
    async fn put(&self, key: StorageKey, data: Value) -> Result<Value, Self::Error> {
        let cql = "INSERT INTO resources (tenant_id, resource_type, resource_id, data) VALUES (?, ?, ?, ?)";
        
        self.session.query(cql, (
            key.tenant_id(),
            key.resource_type(), 
            key.resource_id(),
            data.to_string()
        )).await?;
        
        Ok(data)
    }
    
    async fn get(&self, key: StorageKey) -> Result<Option<Value>, Self::Error> {
        let cql = "SELECT data FROM resources WHERE tenant_id = ? AND resource_type = ? AND resource_id = ?";
        
        match self.session.query(cql, (key.tenant_id(), key.resource_type(), key.resource_id())).await? {
            Some(row) => {
                let json_str: String = row.get("data")?;
                Ok(Some(serde_json::from_str(&json_str)?))
            }
            None => Ok(None)
        }
    }
}
```

**Benefits**: Horizontal scalability, high availability, geographic distribution.

### 4. Hybrid Storage Strategies

**Different backends for different use cases**

```rust
pub struct HybridStorage {
    hot_storage: InMemoryStorage,    // Recently accessed data
    cold_storage: SqliteStorage,     // Persistent storage
}

impl StorageProvider for HybridStorage {
    async fn get(&self, key: StorageKey) -> Result<Option<Value>, Self::Error> {
        // Try hot storage first
        if let Some(data) = self.hot_storage.get(key.clone()).await? {
            return Ok(Some(data));
        }
        
        // Fall back to cold storage and warm cache
        if let Some(data) = self.cold_storage.get(key.clone()).await? {
            self.hot_storage.put(key, data.clone()).await?;
            return Ok(Some(data));
        }
        
        Ok(None)
    }
}
```

**Benefits**: Performance optimization, cost efficiency, flexible data lifecycle.

## Data Organization Patterns

### Tenant Isolation

Storage automatically isolates data by tenant:

```text
Storage Layout:
├── tenant-a/
│   ├── User/
│   │   ├── user-1 → {"id": "user-1", "userName": "alice", ...}
│   │   └── user-2 → {"id": "user-2", "userName": "bob", ...}
│   └── Group/
│       └── group-1 → {"id": "group-1", "displayName": "Admins", ...}
├── tenant-b/
│   └── User/
│       └── user-3 → {"id": "user-3", "userName": "charlie", ...}
└── default/
    └── User/
        └── admin → {"id": "admin", "userName": "admin", ...}
```

### Efficient Querying

Storage providers optimize common query patterns:

```rust
// List all users in a tenant
let prefix = StorageKey::prefix("tenant-123", "User");
let users = storage.list(prefix, 0, 100).await?;

// Find user by username
let matches = storage.find_by_attribute(prefix, "userName", "alice").await?;

// Count resources for capacity planning
let user_count = storage.count(prefix).await?;
```

## Performance Considerations

### 1. Storage Selection by Scale

| Scale | Recommended Storage | Reasoning |
|-------|-------------------|-----------|
| < 1K resources | InMemoryStorage | Maximum performance, simple setup |
| 1K - 100K resources | SqliteStorage | Balanced performance, persistence |
| 100K+ resources | Custom (Postgres/Cassandra) | Scalability, advanced features |

### 2. Key Design Impact

Efficient key structure enables fast operations:

```rust
// Good: Hierarchical keys enable prefix operations
let key = StorageKey::new("tenant-123", "User", "user-456");

// Good: Batch operations on prefixes
let prefix = StorageKey::prefix("tenant-123", "User");
let all_users = storage.list(prefix, 0, usize::MAX).await?;
```

### 3. JSON Storage Optimization

Storage providers work with JSON documents:

```rust
// Storage receives fully-formed JSON
let user_json = json!({
    "id": "user-123",
    "userName": "alice",
    "meta": {
        "resourceType": "User",
        "created": "2023-01-01T00:00:00Z",
        "version": "v1"
    }
});

// Storage doesn't parse or validate - just stores
storage.put(key, user_json).await?;
```

## Best Practices

### 1. Choose Storage Based on Requirements

```rust
// Development: Fast iteration, no persistence needed
let storage = InMemoryStorage::new();

// Production: Small scale, simple deployment
let storage = SqliteStorage::new("app.db").await?;

// Enterprise: High scale, distributed
let storage = CustomDistributedStorage::new().await?;
```

### 2. Handle Errors Appropriately

```rust
// Good: Specific error handling
match storage.get(key).await {
    Ok(Some(data)) => process_data(data),
    Ok(None) => handle_not_found(),
    Err(e) => log_storage_error(e),
}

// Avoid: Ignoring storage errors
let data = storage.get(key).await.unwrap(); // Can panic!
```

### 3. Use Efficient Queries

```rust
// Good: Use prefix queries for lists
let prefix = StorageKey::prefix(tenant_id, resource_type);
let resources = storage.list(prefix, start, count).await?;

// Avoid: Individual gets for list operations
for id in resource_ids {
    let key = StorageKey::new(tenant_id, resource_type, id);
    let resource = storage.get(key).await?; // N+1 queries!
}
```

### 4. Monitor Storage Performance

```rust
// Get insights into storage usage
let stats = storage.stats().await?;
println!("Tenants: {}, Resources: {}", 
         stats.tenant_count, stats.total_resources);

// Use for capacity planning and optimization
if stats.total_resources > 10000 {
    consider_scaling_storage();
}
```

## Integration Patterns

### Factory Pattern

Create storage based on configuration:

```rust
pub fn create_storage(config: &StorageConfig) -> Box<dyn StorageProvider> {
    match config.storage_type {
        StorageType::Memory => Box::new(InMemoryStorage::new()),
        StorageType::Sqlite { path } => Box::new(SqliteStorage::new(path).await.unwrap()),
        StorageType::Custom { .. } => Box::new(CustomStorage::new(config)),
    }
}
```

### Migration Support

Move between storage backends:

```rust
pub async fn migrate_storage<F, T>(from: F, to: T) -> Result<(), Box<dyn std::error::Error>>
where
    F: StorageProvider,
    T: StorageProvider,
{
    let tenants = from.list_tenants().await?;
    
    for tenant_id in tenants {
        let resource_types = from.list_resource_types(&tenant_id).await?;
        
        for resource_type in resource_types {
            let prefix = StorageKey::prefix(&tenant_id, &resource_type);
            let resources = from.list(prefix, 0, usize::MAX).await?;
            
            for (key, data) in resources {
                to.put(key, data).await?;
            }
        }
    }
    
    Ok(())
}
```

The Storage Provider layer enables SCIM Server to work with any data backend while maintaining clean separation between storage concerns and SCIM protocol logic. Whether you need the simplicity of in-memory storage for testing or the scalability of distributed databases for production, the unified interface makes it seamless to switch between implementations.