# Migration Guide: scim-server v0.2.x to v0.3.0

This guide helps you migrate from scim-server v0.2.x to the new v0.3.0 architecture that separates storage concerns from SCIM logic.

## 🚨 Breaking Changes Overview

Version 0.3.0 introduces a **fundamental architectural change** that separates storage operations from SCIM business logic. This provides significant benefits but requires migration of custom provider implementations.

### What's Changing

- **New `StorageProvider` trait** - Simple storage interface (~50 lines to implement)
- **`StandardResourceProvider<S>`** - Generic SCIM logic layer over storage providers
- **Renamed `InMemoryProvider`** - Now `StandardResourceProvider<InMemoryStorageProvider>`
- **Simplified custom providers** - Focus only on storage, not SCIM compliance

### What's NOT Changing

- **Public API remains the same** - All your application code continues to work
- **Same SCIM compliance** - Full RFC 7644 support maintained
- **ETag concurrency control** - All existing features preserved
- **Multi-tenant support** - Tenant isolation continues to work

## 📅 Timeline

- **v0.2.3** (Current) - Stable release with full PATCH support
- **v0.3.0-alpha** (Q1 2025) - Preview release for testing migration
- **v0.3.0** (Q2 2025) - Final release with breaking changes
- **Migration period** - v0.2.x maintained for 6 months after v0.3.0

> **Important**: This library is under active development until v0.9.0. Breaking changes are signaled by minor version increments. Always pin to exact versions (`=0.x.y`) for stability.

## 🔄 Migration Scenarios

### Scenario 1: Using InMemoryProvider (Most Users)

**Before (v0.2.x):**
```toml
[dependencies]
scim-server = "=0.2.3"  # Exact version pinning recommended
```
```rust
use scim_server::providers::InMemoryProvider;

let provider = InMemoryProvider::new();
let server = ScimServer::new(provider)?;
```

**After (v0.3.0):**
```toml
[dependencies]
scim-server = "=0.3.0"  # Continue exact version pinning
```
```rust
use scim_server::providers::InMemoryProvider; // Type alias maintained

let provider = InMemoryProvider::new(); // Same interface
let server = ScimServer::new(provider)?;
```

**Action Required:** ✅ **None** - Backward compatible via type alias (just update version pin)

### Scenario 2: Custom Provider Implementation

**Before (v0.2.x):**
```rust
use scim_server::resource::ResourceProvider;

pub struct DatabaseProvider {
    pool: sqlx::PgPool,
}

impl ResourceProvider for DatabaseProvider {
    type Error = DatabaseError;
    
    // Must implement ALL 15+ SCIM operations
    async fn create_resource(&self, ...) -> Result<Resource, Self::Error> {
        // 1. Storage logic
        // 2. SCIM validation 
        // 3. Schema enforcement
        // 4. Error handling
        // ~100 lines per method
    }
    
    async fn update_resource(&self, ...) -> Result<Resource, Self::Error> {
        // Another 100+ lines...
    }
    
    // ... 13 more complex methods
}
```

**After (v0.3.0):**
```rust
use scim_server::storage::StorageProvider;
use scim_server::providers::StandardResourceProvider;

pub struct DatabaseStorageProvider {
    pool: sqlx::PgPool,
}

impl StorageProvider for DatabaseStorageProvider {
    type Error = DatabaseError;
    
    // Only implement 5 simple storage operations
    async fn create(&self, tenant_id: &str, resource_type: &str, id: &str, data: Value) -> Result<(), Self::Error> {
        // Just store the data - no SCIM logic needed
        sqlx::query!("INSERT INTO resources (tenant_id, resource_type, id, data) VALUES ($1, $2, $3, $4)")
            .bind(tenant_id).bind(resource_type).bind(id).bind(&data)
            .execute(&self.pool).await?;
        Ok(())
    }
    
    async fn read(&self, tenant_id: &str, resource_type: &str, id: &str) -> Result<Option<Value>, Self::Error> {
        // Just retrieve the data
        let row = sqlx::query!("SELECT data FROM resources WHERE tenant_id = $1 AND resource_type = $2 AND id = $3")
            .bind(tenant_id).bind(resource_type).bind(id)
            .fetch_optional(&self.pool).await?;
        Ok(row.map(|r| r.data))
    }
    
    // ... 3 more simple methods (~10 lines each)
}

// SCIM logic handled automatically
pub type DatabaseProvider = StandardResourceProvider<DatabaseStorageProvider>;
```

**Action Required:** 🔄 **Refactor** - Extract storage logic, remove SCIM business logic

## 🛠️ Step-by-Step Migration

### Step 1: Assess Your Current Implementation

Identify which category your usage falls into:

```bash
# Check if you use only InMemoryProvider
grep -r "InMemoryProvider" src/
# ✅ No action needed if only using InMemoryProvider

# Check for custom ResourceProvider implementations
grep -r "impl.*ResourceProvider" src/
# 🔄 Migration needed if you have custom implementations
```

### Step 2: Install Migration Preview

```toml
[dependencies]
# During transition period, use preview version with exact pinning
scim-server = "=0.3.0-alpha"
```

> **⚠️ Version Pinning Required**: Always use exact version pinning (`=0.3.0-alpha`) during active development to avoid unexpected breaking changes.

### Step 3: Extract Storage Logic

If you have a custom provider, separate storage operations from SCIM logic:

**Identify Storage Operations:**
```rust
// Keep these (pure storage):
async fn create_resource() -> Result<Resource, Error> {
    let data = /* SCIM logic */;
    self.db.insert(data).await?; // ← Keep this
    /* More SCIM logic */
}

// Extract to StorageProvider:
async fn create() -> Result<(), Error> {
    self.db.insert(data).await // ← Just this
}
```

**Remove SCIM Logic:**
```rust
// Remove these (handled by StandardResourceProvider):
- Schema validation
- Attribute filtering
- Meta attribute management
- Error code mapping
- ETag generation
- PATCH operation logic
```

### Step 4: Implement StorageProvider

Create your new storage provider:

```rust
use scim_server::storage::{StorageProvider, StorageError};
use serde_json::Value;

pub struct YourStorageProvider {
    // Your storage backend (database, file system, etc.)
}

impl StorageProvider for YourStorageProvider {
    type Error = YourStorageError;
    
    async fn create(&self, tenant_id: &str, resource_type: &str, id: &str, data: Value) -> Result<(), Self::Error> {
        // Just store the data
    }
    
    async fn read(&self, tenant_id: &str, resource_type: &str, id: &str) -> Result<Option<Value>, Self::Error> {
        // Just retrieve the data
    }
    
    async fn update(&self, tenant_id: &str, resource_type: &str, id: &str, data: Value) -> Result<(), Self::Error> {
        // Just update the data
    }
    
    async fn delete(&self, tenant_id: &str, resource_type: &str, id: &str) -> Result<(), Self::Error> {
        // Just delete the data
    }
    
    async fn list(&self, tenant_id: &str, resource_type: &str) -> Result<Vec<(String, Value)>, Self::Error> {
        // Just list the data
    }
}
```

### Step 5: Update Type Definitions

Replace your provider type:

```rust
// Before
type MyProvider = CustomResourceProvider;

// After  
type MyProvider = StandardResourceProvider<YourStorageProvider>;
```

### Step 6: Update Instantiation

Update how you create provider instances:

```rust
// Before
let provider = CustomResourceProvider::new(config);

// After
let storage = YourStorageProvider::new(config);
let provider = StandardResourceProvider::new(storage);
```

## 🧪 Testing Your Migration

### Test Storage Provider Independently

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    
    #[tokio::test]
    async fn test_storage_crud() {
        let storage = YourStorageProvider::new_test();
        
        // Test basic CRUD operations
        let data = json!({"userName": "test@example.com"});
        
        storage.create("tenant1", "User", "123", data.clone()).await.unwrap();
        let retrieved = storage.read("tenant1", "User", "123").await.unwrap();
        assert_eq!(retrieved, Some(data));
        
        storage.delete("tenant1", "User", "123").await.unwrap();
        let deleted = storage.read("tenant1", "User", "123").await.unwrap();
        assert_eq!(deleted, None);
    }
}
```

### Test Full SCIM Functionality

```rust
#[tokio::test]
async fn test_scim_operations() {
    let storage = YourStorageProvider::new_test();
    let provider = StandardResourceProvider::new(storage);
    let server = ScimServer::new(provider).unwrap();
    
    // All existing SCIM tests should pass
    let context = RequestContext::with_generated_id();
    let user_data = json!({
        "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
        "userName": "test@example.com"
    });
    
    let user = server.provider().create_resource("User", user_data, &context).await.unwrap();
    assert!(!user.id.is_empty());
    assert!(user.meta.is_some());
}
```

## 📈 Benefits After Migration

### For Simple Use Cases
- **No changes required** - InMemoryProvider continues to work
- **Same performance** - No overhead from new architecture
- **Same API** - All existing code unchanged

### For Custom Providers
- **90% less code** - Focus only on storage, not SCIM logic
- **Consistent behavior** - Automatic SCIM compliance
- **Easier testing** - Test storage separately from SCIM logic
- **Better performance** - Optimized SCIM layer shared across providers
- **Future-proof** - New SCIM features automatically available

### Example Code Reduction

```
Before (Custom ResourceProvider):
- create_resource(): 120 lines
- update_resource(): 115 lines  
- patch_resource(): 95 lines
- delete_resource(): 45 lines
- list_resources(): 80 lines
- ... 10 more methods
Total: ~1,000+ lines

After (StorageProvider):
- create(): 8 lines
- read(): 6 lines
- update(): 8 lines
- delete(): 4 lines
- list(): 12 lines
Total: ~50 lines (95% reduction!)
```

## 🚨 Common Migration Issues

### Issue 1: SCIM Logic in Storage Layer

**Problem:**
```rust
// DON'T DO THIS in StorageProvider
async fn create(&self, data: Value) -> Result<(), Error> {
    // ❌ Don't validate schemas in storage layer
    if !data.get("schemas").is_some() {
        return Err(Error::InvalidSchema);
    }
    
    // ❌ Don't generate meta attributes in storage layer
    let meta = Meta::new_for_creation("User", &data["id"]);
    
    self.db.insert(data).await
}
```

**Solution:**
```rust
// DO THIS - just store the data
async fn create(&self, tenant_id: &str, resource_type: &str, id: &str, data: Value) -> Result<(), Error> {
    // ✅ Just store what you're given
    self.db.insert(tenant_id, resource_type, id, data).await
}
```

### Issue 2: Complex Error Mapping

**Problem:**
```rust
// DON'T DO THIS - complex SCIM error mapping
impl From<DatabaseError> for ScimError {
    fn from(err: DatabaseError) -> Self {
        match err {
            DatabaseError::NotFound => ScimError::ResourceNotFound,
            DatabaseError::ConstraintViolation => ScimError::Uniqueness,
            // ... complex mapping
        }
    }
}
```

**Solution:**
```rust
// DO THIS - simple storage errors
#[derive(Debug, thiserror::Error)]
pub enum StorageError {
    #[error("Not found")]
    NotFound,
    #[error("Already exists")]
    AlreadyExists,
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),
}
```

### Issue 3: Tenant Handling

**Problem:**
```rust
// DON'T DO THIS - complex tenant logic in storage
async fn create(&self, context: &RequestContext, data: Value) -> Result<(), Error> {
    let tenant_id = if let Some(tenant) = &context.tenant_context {
        &tenant.tenant_id
    } else {
        "default"
    };
    // Complex tenant validation logic...
}
```

**Solution:**
```rust
// DO THIS - tenant_id provided directly
async fn create(&self, tenant_id: &str, resource_type: &str, id: &str, data: Value) -> Result<(), Error> {
    // StandardResourceProvider handles tenant logic
    self.db.insert(tenant_id, resource_type, id, data).await
}
```

## 📞 Getting Help

### Migration Support

- **GitHub Discussions** - Ask migration questions
- **Migration Examples** - Check `examples/migration/` directory
- **Discord Channel** - Real-time help during migration period
- **Documentation** - Updated guides and API docs

### Migration Tools

```bash
# Check migration compatibility
cargo check --features "migration-check"

# Run migration tests
cargo test --features "migration-test"

# Generate migration report
cargo run --bin migration-report
```

## 🎯 Migration Timeline

### Phase 1: Preparation (Now - Q1 2025)
- ✅ Read this migration guide
- ✅ Audit your current provider usage
- ✅ Plan your storage layer extraction

### Phase 2: Alpha Testing (Q1 2025)
- 🔄 Install v0.3.0-alpha
- 🔄 Test migration in development environment
- 🔄 Provide feedback on migration experience

### Phase 3: Release Candidate (Q2 2025)
- 🔄 Migrate to v0.3.0-rc
- 🔄 Final testing and validation
- 🔄 Performance benchmarking

### Phase 4: Production Migration (Q2 2025)
- 🎯 Upgrade to v0.3.0 stable
- 🎯 Deploy to production
- 🎯 Monitor and validate

### Phase 5: Legacy Support (6 months)
- 📞 v0.2.x maintained for critical issues only
- 📞 Migration support and documentation
- 📞 Community assistance

## ✅ Migration Checklist

### Pre-Migration
- [ ] Audit current provider usage
- [ ] Identify custom ResourceProvider implementations
- [ ] Review storage vs SCIM logic separation
- [ ] Plan testing strategy

### During Migration
- [ ] Install v0.3.0-alpha for testing
- [ ] Extract storage operations from SCIM logic
- [ ] Implement StorageProvider trait
- [ ] Update type definitions and instantiation
- [ ] Run comprehensive tests

### Post-Migration
- [ ] Validate all SCIM operations work correctly
- [ ] Performance test against baseline
- [ ] Update documentation and examples
- [ ] Deploy to staging/production
- [ ] Monitor for issues

### Validation
- [ ] All existing tests pass
- [ ] No performance degradation
- [ ] SCIM compliance maintained
- [ ] Multi-tenant isolation working
- [ ] ETag concurrency control functional

---

**Need Help?** Join our [Discord](https://discord.gg/scim-server) or create a [GitHub Discussion](https://github.com/pukeko37/scim-server/discussions) for migration assistance.

The migration to v0.3.0 represents a significant architectural improvement that will make custom providers much simpler to implement and maintain. While it requires some effort, the long-term benefits of reduced complexity and improved maintainability make this migration worthwhile.