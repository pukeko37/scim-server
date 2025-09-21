# Resource Provider Architecture

This deep dive explores the Resource Provider architecture in SCIM Server, covering when to use StandardResourceProvider versus custom implementations, design patterns for business logic integration, and strategies for connecting to existing data models and external systems.

## Overview

The `ResourceProvider` trait is the primary integration point for implementing SCIM business logic. It abstracts away SCIM protocol details while giving you full control over how resources are created, validated, stored, and retrieved. Understanding this architecture is crucial for building production SCIM systems that integrate with your existing infrastructure.

**Key Architectural Decision:** Choose between `StandardResourceProvider` (composition-based) or custom `ResourceProvider` implementation (direct control).

## ResourceProvider Architecture Overview

```text
SCIM Server
    ↓ SCIM Operations
┌─────────────────────────────────────────────────────────────────────────────┐
│ ResourceProvider Trait                                                      │
│                                                                             │
│ ┌─────────────────────┐              ┌─────────────────────────────────────┐ │
│ │ StandardResource    │              │ Custom ResourceProvider            │ │
│ │ Provider<S>         │              │ Implementation                      │ │
│ │                     │              │                                     │ │
│ │ • Schema validation │              │ • Direct business logic control   │ │
│ │ • Generic CRUD ops  │              │ • Custom validation rules          │ │
│ │ • Storage agnostic  │              │ • Integration with existing APIs   │ │
│ │ • ETag support      │              │ • Custom error handling            │ │
│ │ • Multi-tenant      │              │ • Performance optimizations       │ │
│ └─────────────────────┘              └─────────────────────────────────────┘ │
│           ↓                                            ↓                     │
│ ┌─────────────────────┐              ┌─────────────────────────────────────┐ │
│ │ StorageProvider     │              │ Your Business Layer                 │ │
│ │ • InMemoryStorage   │              │ • Database DAOs                     │ │
│ │ • Database adapters │              │ • External APIs                     │ │
│ │ • Custom backends   │              │ • Message queues                    │ │
│ └─────────────────────┘              │ • Legacy systems                    │ │
│                                      └─────────────────────────────────────┘ │
└─────────────────────────────────────────────────────────────────────────────┘
```

## Decision Matrix: StandardResourceProvider vs Custom

### Use StandardResourceProvider When:

- **✅ Standard SCIM compliance** is your primary goal
- **✅ Simple data models** that map well to SCIM schemas
- **✅ Storage abstraction** meets your needs
- **✅ Built-in features** (validation, ETag, multi-tenant) are sufficient
- **✅ Rapid prototyping** or getting started quickly

### Use Custom ResourceProvider When:

- **✅ Complex business logic** requires custom validation or processing
- **✅ Existing data models** don't map cleanly to SCIM
- **✅ Performance requirements** need specialized optimizations
- **✅ External system integration** requires custom API calls
- **✅ Advanced error handling** or audit requirements
- **✅ Custom resource types** beyond standard User/Group

## StandardResourceProvider Deep Dive

### Basic Setup Pattern

```rust
use scim_server::providers::StandardResourceProvider;
use scim_server::storage::InMemoryStorage;
use scim_server::ScimServer;

// Simple in-memory setup
let storage = InMemoryStorage::new();
let provider = StandardResourceProvider::new(storage);
let server = ScimServer::new(provider)?;
```

### Database Integration Pattern

```rust
use scim_server::storage::StorageProvider;
use sqlx::{PgPool, Row};

pub struct PostgresStorageProvider {
    pool: PgPool,
}

impl StorageProvider for PostgresStorageProvider {
    type Error = sqlx::Error;
    
    async fn put(
        &self,
        key: StorageKey,
        data: Value,
        context: &RequestContext,
    ) -> Result<Value, Self::Error> {
        // Add metadata for versioning and tenant isolation
        let mut enriched_data = data;
        enriched_data["meta"] = json!({
            "version": generate_version(&enriched_data),
            "created": chrono::Utc::now(),
            "lastModified": chrono::Utc::now(),
            "resourceType": extract_resource_type(&key),
            "location": generate_location(&key, context),
        });
        
        // Apply tenant scoping if needed
        let table_name = if context.is_multi_tenant() {
            format!("scim_resources_{}", context.tenant_id().unwrap())
        } else {
            "scim_resources".to_string()
        };
        
        sqlx::query(&format!(
            "INSERT INTO {} (id, resource_type, data, version, created_at) 
             VALUES ($1, $2, $3, $4, NOW())
             ON CONFLICT (id) DO UPDATE SET 
             data = $3, version = $4, updated_at = NOW()",
            table_name
        ))
        .bind(key.as_str())
        .bind(extract_resource_type(&key))
        .bind(&enriched_data)
        .bind(enriched_data["meta"]["version"].as_str())
        .execute(&self.pool)
        .await?;
        
        Ok(enriched_data)
    }
    
    async fn get(
        &self,
        key: StorageKey,
        context: &RequestContext,
    ) -> Result<Option<Value>, Self::Error> {
        let table_name = if context.is_multi_tenant() {
            format!("scim_resources_{}", context.tenant_id().unwrap())
        } else {
            "scim_resources".to_string()
        };
        
        let row = sqlx::query(&format!("SELECT data FROM {} WHERE id = $1", table_name))
            .bind(key.as_str())
            .fetch_optional(&self.pool)
            .await?;
            
        Ok(row.map(|r| r.get("data")))
    }
    
    async fn list(
        &self,
        prefix: StoragePrefix,
        context: &RequestContext,
    ) -> Result<Vec<Value>, Self::Error> {
        let table_name = if context.is_multi_tenant() {
            format!("scim_resources_{}", context.tenant_id().unwrap())
        } else {
            "scim_resources".to_string()
        };
        
        let resource_type = prefix.as_str().trim_end_matches(':');
        
        let rows = sqlx::query(&format!(
            "SELECT data FROM {} WHERE resource_type = $1 ORDER BY created_at",
            table_name
        ))
        .bind(resource_type)
        .fetch_all(&self.pool)
        .await?;
        
        Ok(rows.into_iter().map(|r| r.get("data")).collect())
    }
    
    async fn delete(
        &self,
        key: StorageKey,
        context: &RequestContext,
    ) -> Result<bool, Self::Error> {
        let table_name = if context.is_multi_tenant() {
            format!("scim_resources_{}", context.tenant_id().unwrap())
        } else {
            "scim_resources".to_string()
        };
        
        let result = sqlx::query(&format!("DELETE FROM {} WHERE id = $1", table_name))
            .bind(key.as_str())
            .execute(&self.pool)
            .await?;
            
        Ok(result.rows_affected() > 0)
    }
}

// Usage with StandardResourceProvider
let postgres_storage = PostgresStorageProvider::new(db_pool);
let provider = StandardResourceProvider::new(postgres_storage);
let server = ScimServer::new(provider)?;
```

### Extending StandardResourceProvider with Custom Validation

```rust
use scim_server::providers::StandardResourceProvider;
use scim_server::schema::{ValidationResult, ValidationError};

pub struct ValidatingResourceProvider<S: StorageProvider> {
    inner: StandardResourceProvider<S>,
    business_validator: BusinessRuleValidator,
}

impl<S: StorageProvider> ValidatingResourceProvider<S> {
    pub fn new(storage: S, validator: BusinessRuleValidator) -> Self {
        Self {
            inner: StandardResourceProvider::new(storage),
            business_validator: validator,
        }
    }
    
    async fn validate_business_rules(
        &self,
        resource_type: &str,
        data: &Value,
        context: &RequestContext,
    ) -> ValidationResult<()> {
        match resource_type {
            "User" => self.business_validator.validate_user(data, context).await,
            "Group" => self.business_validator.validate_group(data, context).await,
            _ => Ok(()), // No custom validation for other types
        }
    }
}

impl<S: StorageProvider> ResourceProvider for ValidatingResourceProvider<S> {
    type Error = StandardProviderError<S::Error>;
    
    async fn create_resource(
        &self,
        resource_type: &str,
        data: Value,
        context: &RequestContext,
    ) -> Result<Resource, Self::Error> {
        // Apply custom business rule validation before delegating
        self.validate_business_rules(resource_type, &data, context)
            .await
            .map_err(|e| StandardProviderError::ValidationError(e))?;
            
        // Delegate to StandardResourceProvider
        self.inner.create_resource(resource_type, data, context).await
    }
    
    async fn update_resource(
        &self,
        resource_type: &str,
        id: &str,
        data: Value,
        context: &RequestContext,
    ) -> Result<Resource, Self::Error> {
        // Custom validation before update
        self.validate_business_rules(resource_type, &data, context)
            .await
            .map_err(|e| StandardProviderError::ValidationError(e))?;
            
        self.inner.update_resource(resource_type, id, data, context).await
    }
    
    // Delegate other methods to inner provider
    async fn get_resource(
        &self,
        resource_type: &str,
        id: &str,
        context: &RequestContext,
    ) -> Result<Option<Resource>, Self::Error> {
        self.inner.get_resource(resource_type, id, context).await
    }
    
    // ... other methods
}

pub struct BusinessRuleValidator {
    // External dependencies for validation
    user_service: Arc<dyn UserService>,
    policy_engine: Arc<dyn PolicyEngine>,
}

impl BusinessRuleValidator {
    async fn validate_user(&self, data: &Value, context: &RequestContext) -> ValidationResult<()> {
        // Custom user validation logic
        if let Some(email) = data.get("emails").and_then(|e| e.as_array()) {
            for email_obj in email {
                if let Some(email_value) = email_obj.get("value").and_then(|v| v.as_str()) {
                    // Check if email is already in use
                    if self.user_service.email_exists(email_value).await? {
                        return Err(ValidationError::custom(
                            "email",
                            "Email address already in use",
                        ));
                    }
                    
                    // Validate against company policy
                    if !self.policy_engine.validate_email_domain(email_value, context).await? {
                        return Err(ValidationError::custom(
                            "email",
                            "Email domain not allowed for this tenant",
                        ));
                    }
                }
            }
        }
        
        Ok(())
    }
}
```

## Custom ResourceProvider Patterns

### Direct Database Integration

```rust
use scim_server::{ResourceProvider, Resource, RequestContext};
use uuid::Uuid;

pub struct DirectDatabaseProvider {
    db_pool: PgPool,
    user_mapper: UserMapper,
    group_mapper: GroupMapper,
}

impl ResourceProvider for DirectDatabaseProvider {
    type Error = DatabaseProviderError;
    
    async fn create_resource(
        &self,
        resource_type: &str,
        data: Value,
        context: &RequestContext,
    ) -> Result<Resource, Self::Error> {
        match resource_type {
            "User" => self.create_user(data, context).await,
            "Group" => self.create_group(data, context).await,
            _ => Err(DatabaseProviderError::UnsupportedResourceType(resource_type.to_string())),
        }
    }
    
    // ... other trait methods
}

impl DirectDatabaseProvider {
    async fn create_user(
        &self,
        scim_data: Value,
        context: &RequestContext,
    ) -> Result<Resource, DatabaseProviderError> {
        // 1. Convert SCIM data to internal user model
        let user_model = self.user_mapper.scim_to_internal(&scim_data)?;
        
        // 2. Apply tenant scoping
        let tenant_scoped_user = if let Some(tenant_id) = context.tenant_id() {
            user_model.with_tenant_id(tenant_id.to_string())
        } else {
            user_model
        };
        
        // 3. Begin database transaction
        let mut tx = self.db_pool.begin().await?;
        
        // 4. Insert user record
        let user_id = Uuid::new_v4();
        sqlx::query!(
            r#"
            INSERT INTO users (
                id, tenant_id, username, email, first_name, last_name,
                active, created_at, updated_at
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, NOW(), NOW())
            "#,
            user_id,
            tenant_scoped_user.tenant_id,
            tenant_scoped_user.username,
            tenant_scoped_user.email,
            tenant_scoped_user.first_name,
            tenant_scoped_user.last_name,
            tenant_scoped_user.active,
        )
        .execute(&mut *tx)
        .await?;
        
        // 5. Handle multi-valued attributes (emails, phone numbers)
        if let Some(emails) = scim_data.get("emails").and_then(|v| v.as_array()) {
            for (idx, email_obj) in emails.iter().enumerate() {
                if let Some(email) = email_obj.get("value").and_then(|v| v.as_str()) {
                    let is_primary = email_obj.get("primary")
                        .and_then(|v| v.as_bool())
                        .unwrap_or(idx == 0);
                    let email_type = email_obj.get("type")
                        .and_then(|v| v.as_str())
                        .unwrap_or("work");
                        
                    sqlx::query!(
                        "INSERT INTO user_emails (user_id, email, email_type, is_primary) VALUES ($1, $2, $3, $4)",
                        user_id,
                        email,
                        email_type,
                        is_primary,
                    )
                    .execute(&mut *tx)
                    .await?;
                }
            }
        }
        
        // 6. Commit transaction
        tx.commit().await?;
        
        // 7. Fetch complete user data
        let created_user = self.get_user_by_id(&user_id.to_string(), context).await?
            .ok_or(DatabaseProviderError::UserNotFound)?;
            
        // 8. Convert back to SCIM resource
        let scim_resource = self.user_mapper.internal_to_scim(created_user)?;
        
        Ok(Resource::new(scim_resource))
    }
    
    async fn get_user_by_id(
        &self,
        id: &str,
        context: &RequestContext,
    ) -> Result<Option<InternalUser>, DatabaseProviderError> {
        let user_uuid = Uuid::parse_str(id)?;
        
        // Build tenant-aware query
        let tenant_filter = if let Some(tenant_id) = context.tenant_id() {
            format!("AND tenant_id = '{}'", tenant_id)
        } else {
            "AND tenant_id IS NULL".to_string()
        };
        
        let user_row = sqlx::query(&format!(
            "SELECT * FROM users WHERE id = $1 {}",
            tenant_filter
        ))
        .bind(user_uuid)
        .fetch_optional(&self.db_pool)
        .await?;
        
        match user_row {
            Some(row) => {
                // Fetch associated emails
                let emails = sqlx::query!(
                    "SELECT email, email_type, is_primary FROM user_emails WHERE user_id = $1",
                    user_uuid
                )
                .fetch_all(&self.db_pool)
                .await?;
                
                Ok(Some(InternalUser {
                    id: row.get("id"),
                    tenant_id: row.get("tenant_id"),
                    username: row.get("username"),
                    first_name: row.get("first_name"),
                    last_name: row.get("last_name"),
                    active: row.get("active"),
                    emails: emails.into_iter().map(|e| UserEmail {
                        value: e.email,
                        email_type: e.email_type,
                        primary: e.is_primary,
                    }).collect(),
                    created_at: row.get("created_at"),
                    updated_at: row.get("updated_at"),
                }))
            },
            None => Ok(None),
        }
    }
}
```

### External API Integration Pattern

```rust
use reqwest::Client;
use serde::{Deserialize, Serialize};

pub struct ExternalApiProvider {
    http_client: Client,
    api_base_url: String,
    api_token: String,
    scim_mapper: ScimApiMapper,
}

impl ResourceProvider for ExternalApiProvider {
    type Error = ApiProviderError;
    
    async fn create_resource(
        &self,
        resource_type: &str,
        data: Value,
        context: &RequestContext,
    ) -> Result<Resource, Self::Error> {
        match resource_type {
            "User" => self.create_user_via_api(data, context).await,
            "Group" => self.create_group_via_api(data, context).await,
            _ => Err(ApiProviderError::UnsupportedResourceType(resource_type.to_string())),
        }
    }
    
    async fn get_resource(
        &self,
        resource_type: &str,
        id: &str,
        context: &RequestContext,
    ) -> Result<Option<Resource>, Self::Error> {
        let api_endpoint = match resource_type {
            "User" => format!("{}/api/v1/users/{}", self.api_base_url, id),
            "Group" => format!("{}/api/v1/groups/{}", self.api_base_url, id),
            _ => return Err(ApiProviderError::UnsupportedResourceType(resource_type.to_string())),
        };
        
        let response = self.http_client
            .get(&api_endpoint)
            .header("Authorization", format!("Bearer {}", self.api_token))
            .header("X-Tenant-ID", context.tenant_id().unwrap_or("default"))
            .send()
            .await?;
            
        match response.status() {
            reqwest::StatusCode::OK => {
                let api_data: Value = response.json().await?;
                let scim_resource = self.scim_mapper.api_to_scim(resource_type, api_data)?;
                Ok(Some(Resource::new(scim_resource)))
            },
            reqwest::StatusCode::NOT_FOUND => Ok(None),
            status => Err(ApiProviderError::ApiError(status, response.text().await?)),
        }
    }
    
    // ... other methods
}

impl ExternalApiProvider {
    async fn create_user_via_api(
        &self,
        scim_data: Value,
        context: &RequestContext,
    ) -> Result<Resource, ApiProviderError> {
        // 1. Convert SCIM to external API format
        let api_payload = self.scim_mapper.scim_to_api("User", scim_data)?;
        
        // 2. Add tenant context to API call
        let mut enriched_payload = api_payload;
        if let Some(tenant_id) = context.tenant_id() {
            enriched_payload["tenant_id"] = json!(tenant_id);
        }
        
        // 3. Make API call
        let response = self.http_client
            .post(&format!("{}/api/v1/users", self.api_base_url))
            .header("Authorization", format!("Bearer {}", self.api_token))
            .header("Content-Type", "application/json")
            .json(&enriched_payload)
            .send()
            .await?;
            
        match response.status() {
            reqwest::StatusCode::CREATED => {
                let created_user: Value = response.json().await?;
                let scim_resource = self.scim_mapper.api_to_scim("User", created_user)?;
                Ok(Resource::new(scim_resource))
            },
            status => {
                let error_body = response.text().await?;
                Err(ApiProviderError::ApiError(status, error_body))
            }
        }
    }
}

#[derive(Debug)]
pub struct ScimApiMapper {
    // Configuration for field mappings
    user_field_mappings: HashMap<String, String>,
    group_field_mappings: HashMap<String, String>,
}

impl ScimApiMapper {
    pub fn new() -> Self {
        let mut user_mappings = HashMap::new();
        user_mappings.insert("userName".to_string(), "username".to_string());
        user_mappings.insert("displayName".to_string(), "display_name".to_string());
        user_mappings.insert("name.givenName".to_string(), "first_name".to_string());
        user_mappings.insert("name.familyName".to_string(), "last_name".to_string());
        
        Self {
            user_field_mappings: user_mappings,
            group_field_mappings: HashMap::new(),
        }
    }
    
    pub fn scim_to_api(&self, resource_type: &str, scim_data: Value) -> Result<Value, MappingError> {
        match resource_type {
            "User" => self.map_user_scim_to_api(scim_data),
            "Group" => self.map_group_scim_to_api(scim_data),
            _ => Err(MappingError::UnsupportedResourceType(resource_type.to_string())),
        }
    }
    
    fn map_user_scim_to_api(&self, scim_data: Value) -> Result<Value, MappingError> {
        let mut api_data = json!({});
        
        // Map simple fields
        for (scim_field, api_field) in &self.user_field_mappings {
            if let Some(value) = scim_data.get(scim_field) {
                api_data[api_field] = value.clone();
            }
        }
        
        // Handle complex fields like emails
        if let Some(emails) = scim_data.get("emails").and_then(|v| v.as_array()) {
            if let Some(primary_email) = emails.iter().find(|e| {
                e.get("primary").and_then(|p| p.as_bool()).unwrap_or(false)
            }) {
                if let Some(email_value) = primary_email.get("value") {
                    api_data["email"] = email_value.clone();
                }
            }
        }
        
        Ok(api_data)
    }
    
    pub fn api_to_scim(&self, resource_type: &str, api_data: Value) -> Result<Value, MappingError> {
        match resource_type {
            "User" => self.map_user_api_to_scim(api_data),
            "Group" => self.map_group_api_to_scim(api_data),
            _ => Err(MappingError::UnsupportedResourceType(resource_type.to_string())),
        }
    }
    
    fn map_user_api_to_scim(&self, api_data: Value) -> Result<Value, MappingError> {
        let mut scim_data = json!({
            "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
        });
        
        // Reverse mapping from API to SCIM
        for (scim_field, api_field) in &self.user_field_mappings {
            if let Some(value) = api_data.get(api_field) {
                scim_data[scim_field] = value.clone();
            }
        }
        
        // Handle complex fields
        if let Some(email) = api_data.get("email").and_then(|v| v.as_str()) {
            scim_data["emails"] = json!([{
                "value": email,
                "type": "work",
                "primary": true
            }]);
        }
        
        // Add metadata
        scim_data["meta"] = json!({
            "resourceType": "User",
            "created": api_data.get("created_at").unwrap_or(&json!("")),
            "lastModified": api_data.get("updated_at").unwrap_or(&json!("")),
        });
        
        Ok(scim_data)
    }
}
```

### Hybrid Provider Pattern

Sometimes you need different strategies for different resource types:

```rust
pub struct HybridResourceProvider {
    user_provider: Box<dyn ResourceProvider<Error = Box<dyn std::error::Error + Send + Sync>>>,
    group_provider: Box<dyn ResourceProvider<Error = Box<dyn std::error::Error + Send + Sync>>>,
    default_provider: StandardResourceProvider<InMemoryStorage>,
}

impl ResourceProvider for HybridResourceProvider {
    type Error = HybridProviderError;
    
    async fn create_resource(
        &self,
        resource_type: &str,
        data: Value,
        context: &RequestContext,
    ) -> Result<Resource, Self::Error> {
        match resource_type {
            "User" => {
                self.user_provider
                    .create_resource(resource_type, data, context)
                    .await
                    .map_err(|e| HybridProviderError::UserProviderError(e))
            },
            "Group" => {
                self.group_provider
                    .create_resource(resource_type, data, context)
                    .await
                    .map_err(|e| HybridProviderError::GroupProviderError(e))
            },
            _ => {
                // Fall back to default provider for other resource types
                self.default_provider
                    .create_resource(resource_type, data, context)
                    .await
                    .map_err(|e| HybridProviderError::DefaultProviderError(e))
            }
        }
    }
    
    // Similar delegation for other methods...
}
```

## Performance Optimization Patterns

### Caching Layer Pattern

```rust
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

pub struct CachingResourceProvider<P: ResourceProvider> {
    inner_provider: P,
    cache: Arc<RwLock<ResourceCache>>,
    cache_ttl: Duration,
}

struct ResourceCache {
    resources: HashMap<String, CachedResource>,
    list_cache: HashMap<String, CachedList>,
}

struct CachedResource {
    resource: Resource,
    cached_at: Instant,
    version: String,
}

struct CachedList {
    resources: Vec<Resource>,
    cached_at: Instant,
}

impl<P: ResourceProvider> CachingResourceProvider<P> {
    pub fn new(inner_provider: P, cache_ttl: Duration) -> Self {
        Self {
            inner_provider,
            cache: Arc::new(RwLock::new(ResourceCache {
                resources: HashMap::new(),
                list_cache: HashMap::new(),
            })),
            cache_ttl,
        }
    }
    
    fn cache_key(&self, resource_type: &str, id: &str, context: &RequestContext) -> String {
        if let Some(tenant_id) = context.tenant_id() {
            format!("{}:{}:{}", tenant_id, resource_type, id)
        } else {
            format!("{}:{}", resource_type, id)
        }
    }
    
    fn list_cache_key(&self, resource_type: &str, context: &RequestContext) -> String {
        if let Some(tenant_id) = context.tenant_id() {
            format!("{}:{}:list", tenant_id, resource_type)
        } else {
            format!("{}:list", resource_type)
        }
    }
    
    async fn is_cache_valid(&self, cached_at: Instant) -> bool {
        cached_at.elapsed() < self.cache_ttl
    }
    
    async fn invalidate_related_cache(&self, resource_type: &str, context: &RequestContext) {
        let mut cache = self.cache.write().await;
        
        // Invalidate list cache for this resource type
        let list_key = self.list_cache_key(resource_type, context);
        cache.list_cache.remove(&list_key);
        
        // For group operations, also invalidate user caches (due to membership changes)
        if resource_type == "Group" {
            cache.list_cache.remove(&self.list_cache_key("User", context));
        }
    }
}

impl<P: ResourceProvider> ResourceProvider for CachingResourceProvider<P> {
    type Error = P::Error;
    
    async fn get_resource(
        &self,
        resource_type: &str,
        id: &str,
        context: &RequestContext,
    ) -> Result<Option<Resource>, Self::Error> {
        let cache_key = self.cache_key(resource_type, id, context);
        
        // Check cache first
        {
            let cache = self.cache.read().await;
            if let Some(cached) = cache.resources.get(&cache_key) {
                if self.is_cache_valid(cached.cached_at).await {
                    return Ok(Some(cached.resource.clone()));
                }
            }
        }
        
        // Cache miss or expired - fetch from inner provider
        let resource = self.inner_provider.get_resource(resource_type, id, context).await?;
        
        // Update cache
        if let Some(ref res) = resource {
            let mut cache = self.cache.write().await;
            cache.resources.insert(cache_key, CachedResource {
                resource: res.clone(),
                cached_at: Instant::now(),
                version: res.version().unwrap_or_default(),
            });
        }
        
        Ok(resource)
    }
    
    async fn create_resource(
        &self,
        resource_type: &str,
        data: Value,
        context: &RequestContext,
    ) -> Result<Resource, Self::Error> {
        let resource = self.inner_provider.create_resource(resource_type, data, context).await?;
        
        // Invalidate related caches
        self.invalidate_related_cache(resource_type, context).await;
        
        // Cache the new resource
        let cache_key = self.cache_key(resource_type, resource.id(), context);
        let mut cache = self.cache.write().await;
        cache.resources.insert(cache_key, CachedResource {
            resource: resource.clone(),
            cached_at: Instant::now(),
            version: resource.version().unwrap_or_default(),
        });
        
        Ok(resource)
    }
    
    async fn update_resource(
        &self,
        resource_type: &str,
        id: &str,
        data: Value,
        context: &RequestContext,
    ) -> Result<Resource, Self::Error> {
        let resource = self.inner_provider.update_resource(resource_type, id, data, context).await?;
        
        // Update cache
        let cache_key = self.cache_key(resource_type, id, context);
        let mut cache = self.cache.write().await;
        cache.resources.insert(cache_key, CachedResource {
            resource: resource.clone(),
            cached_at: Instant::now(),
            version: resource.version().unwrap_or_default(),
        });
        
        // Invalidate list caches
        self.invalidate_related_cache(resource_type, context).await;
        
        Ok(resource)
    }
    
    async fn delete_resource(
        &self,
        resource_type: &str,
        id: &str,
        context: &RequestContext,
    ) -> Result<bool, Self::Error> {
        let deleted = self.inner_provider.delete_resource(resource_type, id, context).await?;
        
        if deleted {
            // Remove from cache
            let cache_key = self.cache_key(resource_type, id, context);
            let mut cache = self.cache.write().await;
            cache.resources.remove(&cache_key);
            
            // Invalidate list caches
            self.invalidate_related_cache(resource_type, context).await;
        }
        
        Ok(deleted)
    }
}
```

### Connection Pooling Pattern

```rust
use deadpool_postgres::{Config, Pool, Runtime};
use tokio_postgres::NoTls;

pub struct PooledDatabaseProvider {
    pool: Pool,
    max_connections: usize,
}

impl PooledDatabaseProvider {
    pub fn new(database_url: &str, max_connections: usize) -> Result<Self, PoolError> {
        let mut cfg = Config::new();
        cfg.url = Some(database_url.to_string());
        cfg.pool = Some(deadpool_postgres::PoolConfig::new(max_connections));
        
        let pool = cfg.create_pool(Some(Runtime::Tokio1), NoTls)?;
        
        Ok(Self {
            pool,
            max_connections,
        })
    }
    
    pub async fn health_check(&self) -> Result<(), HealthCheckError> {
        let client = self.pool.get().await?;
        client.execute("SELECT 1", &[]).await?;
        Ok(())
    }
    
    pub async fn get_pool_status(&self) -> PoolStatus {
        let status = self.pool.status();
        PoolStatus {
            size: status.size,
            available: status.available,
            waiting: status.waiting,
            max_size: self.max_connections,
        }
    }
}
```

## Error Handling Patterns

### Comprehensive Error Mapping

```rust
use scim_server::{ScimError, ScimResult};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum CustomProviderError {
    #[error("Database error: {0}")]
    DatabaseError(#[from] sqlx::Error),
    
    #[error("External API error: {status} - {message}")]
    ApiError { status: reqwest::StatusCode, message: String },
    
    #[error("Business rule violation: {rule} - {details}")]
    BusinessRuleViolation { rule: String, details: String },
    
    #[error("Resource not found: {resource_type} with id {id}")]
    ResourceNotFound { resource_type: String, id: String },
    
    #[error("Tenant limit exceeded: {resource_type} ({current}/{limit})")]
    TenantLimitExceeded { resource_type: String, current: usize, limit: usize },
    
    #[error("Invalid data format: {field} - {reason}")]
    InvalidDataFormat { field: String, reason: String },
    
    #[error("Concurrent modification detected for {resource_type} {id}")]
    ConcurrentModification { resource_type: String, id: String },
}

impl From<CustomProviderError> for ScimError {
    fn from(error: CustomProviderError) -> Self {
        match error {
            CustomProviderError::ResourceNotFound { resource_type, id } => {
                ScimError::NotFound(format!("{} {} not found", resource_type, id))
            },
            CustomProviderError::TenantLimitExceeded { resource_type, current, limit } => {
                ScimError::BadRequest(format!(
                    "Tenant limit exceeded for {}: {}/{}", 
                    resource_type, current, limit
                ))
            },
            CustomProviderError::BusinessRuleViolation { rule, details } => {
                ScimError::BadRequest(format!("Business rule '{}' violated: {}", rule, details))
            },
            CustomProviderError::ConcurrentModification { resource_type, id } => {
                ScimError::PreconditionFailed(format!(
                    "Resource {} {} was modified by another request",
                    resource_type, id
                ))
            },
            CustomProviderError::InvalidDataFormat { field, reason } => {
                ScimError::BadRequest(format!("Invalid {}: {}", field, reason))
            },
            CustomProviderError::DatabaseError(db_error) => {
                // Log the detailed database error but return generic error to client
                tracing::error!(error = %db_error, "Database operation failed");
                ScimError::InternalError("Database operation failed".to_string())
            },
            CustomProviderError::ApiError { status, message } => {
                tracing::error!(status = %status, message = %message, "External API error");
                match status {
                    reqwest::StatusCode::NOT_FOUND => ScimError::NotFound("Resource not found in external system".to_string()),
                    reqwest::StatusCode::CONFLICT => ScimError::Conflict("Resource conflict in external system".to_string()),
                    _ => ScimError::InternalError("External system error".to_string()),
                }
            },
        }
    }
}
```

## Testing Patterns

### Mock Provider for Testing

```rust
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

pub struct MockResourceProvider {
    resources: Arc<Mutex<HashMap<String, Value>>>,
    fail_on_create: bool,
    simulate_latency: Option<Duration>,
}

impl MockResourceProvider {
    pub fn new() -> Self {
        Self {
            resources: Arc::new(Mutex::new(HashMap::new())),
            fail_on_create: false,
            simulate_latency: None,
        }
    }
    
    pub fn with_failure_simulation(mut self, fail_on_create: bool) -> Self {
        self.fail_on_create = fail_on_create;
        self
    }
    
    pub fn with_latency_simulation(mut self, latency: Duration) -> Self {
        self.simulate_latency = Some(latency);
        self
    }
    
    pub fn preset_resource(&self, resource_type: &str, id: &str, data: Value) {
        let key = format!("{}:{}", resource_type, id);
        self.resources.lock().unwrap().insert(key, data);
    }
    
    pub fn resource_count(&self, resource_type: &str) -> usize {
        let resources = self.resources.lock().unwrap();
        resources.keys().filter(|k| k.starts_with(&format!("{}:", resource_type))).count()
    }
}

#[async_trait]
impl ResourceProvider for MockResourceProvider {
    type Error = MockProviderError;
    
    async fn create_resource(
        &self,
        resource_type: &str,
        mut data: Value,
        _context: &RequestContext,
    ) -> Result<Resource, Self::Error> {
        if let Some(latency) = self.simulate_latency {
            tokio::time::sleep(latency).await;
        }
        
        if self.fail_on_create {
            return Err(MockProviderError::SimulatedFailure);
        }
        
        let id = uuid::Uuid::new_v4().to_string();
        data["id"] = json!(id);
        data["meta"] = json!({
            "resourceType": resource_type,
            "created": chrono::Utc::now().to_rfc3339(),
            "lastModified": chrono::Utc::now().to_rfc3339(),
            "version": format!("W/\"{}\"", uuid::Uuid::new_v4()),
        });
        
        let key = format!("{}:{}", resource_type, id);
        self.resources.lock().unwrap().insert(key, data.clone());
        
        Ok(Resource::new(data))
    }
    
    async fn get_resource(
        &self,
        resource_type: &str,
        id: &str,
        _context: &RequestContext,
    ) -> Result<Option<Resource>, Self::Error> {
        if let Some(latency) = self.simulate_latency {
            tokio::time::sleep(latency).await;
        }
        
        let key = format!("{}:{}", resource_type, id);
        let resources = self.resources.lock().unwrap();
        
        Ok(resources.get(&key).map(|data| Resource::new(data.clone())))
    }
    
    // ... other methods
}

#[derive(Error, Debug)]
pub enum MockProviderError {
    #[error("Simulated failure for testing")]
    SimulatedFailure,
}
```

### Integration Test Patterns

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use scim_server::{ScimServer, RequestContext};
    use serde_json::json;
    
    #[tokio::test]
    async fn test_custom_provider_integration() {
        let provider = MockResourceProvider::new();
        let server = ScimServer::new(provider).unwrap();
        let context = RequestContext::new("test-request".to_string());
        
        // Test resource creation
        let user_data = json!({
            "userName": "test.user",
            "displayName": "Test User",
            "emails": [{
                "value": "test@example.com",
                "primary": true
            }]
        });
        
        let created_user = server
            .create_resource("User", user_data.clone(), &context)
            .await
            .expect("Failed to create user");
            
        assert!(created_user.id().is_some());
        assert_eq!(created_user.get("userName").unwrap(), "test.user");
        
        // Test resource retrieval
        let retrieved_user = server
            .get_resource("User", created_user.id().unwrap(), &context)
            .await
            .expect("Failed to get user")
            .expect("User not found");
            
        assert_eq!(retrieved_user.get("userName").unwrap(), "test.user");
    }
    
    #[tokio::test]
    async fn test_multi_tenant_provider_isolation() {
        let provider = MockResourceProvider::new();
        let server = ScimServer::new(provider).unwrap();
        
        // Create contexts for different tenants
        let tenant_a_context = RequestContext::with_tenant_generated_id(
            TenantContext::new("tenant-a".to_string(), "client-a".to_string())
        );
        let tenant_b_context = RequestContext::with_tenant_generated_id(
            TenantContext::new("tenant-b".to_string(), "client-b".to_string())
        );
        
        // Create user in tenant A
        let user_data = json!({
            "userName": "user.a",
            "displayName": "User A"
        });
        
        let user_a = server
            .create_resource("User", user_data, &tenant_a_context)
            .await
            .expect("Failed to create user for tenant A");
            
        // Try to access from tenant B - should not be found
        let not_found = server
            .get_resource("User", user_a.id().unwrap(), &tenant_b_context)
            .await
            .expect("Operation should succeed but return None");
            
        assert!(not_found.is_none(), "Tenant isolation violated");
        
        // But should be accessible from tenant A
        let found = server
            .get_resource("User", user_a.id().unwrap(), &tenant_a_context)
            .await
            .expect("Should be able to access own tenant's resources")
            .expect("User should exist in tenant A");
            
        assert_eq!(found.get("userName").unwrap(), "user.a");
    }
    
    #[tokio::test]
    async fn test_error_handling() {
        let provider = MockResourceProvider::new().with_failure_simulation(true);
        let server = ScimServer::new(provider).unwrap();
        let context = RequestContext::new("test-request".to_string());
        
        let user_data = json!({
            "userName": "test.user",
            "displayName": "Test User"
        });
        
        let result = server
            .create_resource("User", user_data, &context)
            .await;
            
        assert!(result.is_err(), "Expected simulated failure");
        
        match result.unwrap_err() {
            ScimError::InternalError(_) => {
                // Expected error type
            },
            other => panic!("Unexpected error type: {:?}", other),
        }
    }
}
```

## Best Practices Summary

### Architecture Decisions

1. **Start with StandardResourceProvider** for rapid development and standard compliance
2. **Move to custom ResourceProvider** when you need specialized business logic
3. **Use hybrid approaches** for different resource types with different requirements
4. **Consider caching layers** for performance-critical applications
5. **Implement comprehensive error handling** with proper error mapping

### Performance Considerations

1. **Use connection pooling** for database providers
2. **Implement caching** for frequently accessed resources
3. **Optimize database queries** with proper indexing and query patterns
4. **Consider async operations** for external API integrations
5. **Monitor resource provider performance** with metrics and tracing

### Security and Multi-Tenancy

1. **Always validate tenant boundaries** in custom providers
2. **Apply tenant scoping** at the storage level
3. **Implement proper permission checking** before resource operations
4. **Use secure credential handling** for external API integrations
5. **Audit all resource operations** for compliance requirements

## Related Topics

- **[Request Lifecycle & Context Management](./request-lifecycle.md)** - How requests flow through resource providers
- **[Multi-Tenant Architecture Patterns](./multi-tenant-patterns.md)** - Tenant isolation in resource providers
- **[Storage Providers](../concepts/storage-providers.md)** - Understanding the storage abstraction layer
- **[Resource Providers](../concepts/resource-providers.md)** - Core concepts and interfaces

## Next Steps

Now that you understand Resource Provider architecture:

1. **Evaluate your requirements** using the decision matrix
2. **Choose your provider strategy** (Standard, Custom, or Hybrid)
3. **Implement your data integration layer** following the patterns
4. **Add proper error handling and logging** for production readiness
5. **Set up performance monitoring** and optimization strategies