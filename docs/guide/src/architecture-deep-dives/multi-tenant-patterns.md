# Multi-Tenant Architecture Patterns

This deep dive explores end-to-end multi-tenant patterns in SCIM Server, from authentication and tenant resolution through storage isolation and URL generation strategies. It provides practical guidance for implementing robust multi-tenant SCIM systems that scale.

## Overview

Multi-tenant architecture in SCIM Server involves several interconnected patterns that work together to provide complete tenant isolation. This document shows how these patterns combine to create production-ready multi-tenant systems.

**Core Multi-Tenant Flow:**
```text
Client Request → Authentication → Tenant Resolution → Context Propagation → 
Storage Isolation → Response Generation → Tenant-Specific URLs
```

## Complete Multi-Tenant Request Flow

```text
┌─────────────────────────────────────────────────────────────────────────────┐
│ 1. Authentication & Credential Extraction                                  │
│    • Extract API key, JWT token, or subdomain                             │
│    • Validate credential format and signature                              │
│    • Handle authentication failures                                        │
└─────────────────────────────────────────────────────────────────────────────┘
    ↓ Validated Credential
┌─────────────────────────────────────────────────────────────────────────────┐
│ 2. Tenant Resolution                                                        │
│    • Map credential → TenantContext                                        │
│    • Load tenant permissions and limits                                    │
│    • Apply isolation level configuration                                   │
└─────────────────────────────────────────────────────────────────────────────┘
    ↓ TenantContext
┌─────────────────────────────────────────────────────────────────────────────┐
│ 3. Request Context Enhancement                                              │
│    • Create RequestContext with tenant information                         │
│    • Validate operation permissions                                        │
│    • Set up request tracing and audit context                             │
└─────────────────────────────────────────────────────────────────────────────┘
    ↓ Tenant-Aware RequestContext
┌─────────────────────────────────────────────────────────────────────────────┐
│ 4. Storage Key Scoping                                                      │
│    • Apply tenant prefix to all storage keys                              │
│    • Enforce isolation level constraints                                   │
│    • Handle cross-tenant reference validation                              │
└─────────────────────────────────────────────────────────────────────────────┘
    ↓ Tenant-Scoped Storage Operations
┌─────────────────────────────────────────────────────────────────────────────┐
│ 5. Resource Operations with Tenant Enforcement                             │
│    • Apply tenant-specific resource limits                                 │
│    • Validate cross-tenant resource references                             │
│    • Enforce tenant-specific schema extensions                             │
└─────────────────────────────────────────────────────────────────────────────┘
    ↓ Tenant-Compliant Resources
┌─────────────────────────────────────────────────────────────────────────────┐
│ 6. Response Assembly with Tenant URLs                                      │
│    • Generate tenant-specific resource URLs                                │
│    • Apply tenant branding and response customization                      │
│    • Include tenant-aware pagination and filtering                         │
└─────────────────────────────────────────────────────────────────────────────┘
    ↓ Tenant-Specific SCIM Response
```

## Tenant Resolution Patterns

### Pattern 1: API Key-Based Resolution

Most common for API-driven integrations:

```rust
use scim_server::multi_tenant::{TenantResolver, TenantContext};
use std::collections::HashMap;
use tokio::sync::RwLock;

pub struct ApiKeyTenantResolver {
    // In production: use database or distributed cache
    tenant_mappings: RwLock<HashMap<String, TenantContext>>,
    default_permissions: TenantPermissions,
}

impl ApiKeyTenantResolver {
    pub fn new() -> Self {
        Self {
            tenant_mappings: RwLock::new(HashMap::new()),
            default_permissions: TenantPermissions::default(),
        }
    }
    
    pub async fn register_tenant(
        &self,
        api_key: String,
        tenant_id: String,
        client_id: String,
        custom_permissions: Option<TenantPermissions>,
    ) -> Result<(), TenantError> {
        let permissions = custom_permissions.unwrap_or_else(|| self.default_permissions.clone());
        
        let tenant_context = TenantContext::new(tenant_id, client_id)
            .with_permissions(permissions)
            .with_isolation_level(IsolationLevel::Standard);
            
        self.tenant_mappings.write().await
            .insert(api_key, tenant_context);
            
        Ok(())
    }
}

impl TenantResolver for ApiKeyTenantResolver {
    type Error = TenantResolutionError;
    
    async fn resolve_tenant(&self, api_key: &str) -> Result<TenantContext, Self::Error> {
        self.tenant_mappings.read().await
            .get(api_key)
            .cloned()
            .ok_or(TenantResolutionError::TenantNotFound(api_key.to_string()))
    }
    
    async fn validate_tenant(&self, tenant_context: &TenantContext) -> Result<bool, Self::Error> {
        // Additional validation logic
        Ok(tenant_context.is_active() && !tenant_context.is_suspended())
    }
}
```

### Pattern 2: JWT-Based Resolution with Claims

Ideal for OAuth2/OpenID Connect integrations:

```rust
use jsonwebtoken::{decode, DecodingKey, Validation, Algorithm};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct TenantClaims {
    pub sub: String,           // Subject (user ID)
    pub tenant_id: String,     // Tenant identifier
    pub client_id: String,     // Client application ID
    pub scopes: Vec<String>,   // Granted scopes
    pub resource_limits: ResourceLimits,
    pub exp: usize,           // Expiration time
    pub iat: usize,           // Issued at
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ResourceLimits {
    pub max_users: Option<usize>,
    pub max_groups: Option<usize>,
    pub allowed_operations: Vec<String>,
}

pub struct JwtTenantResolver {
    decoding_key: DecodingKey,
    validation: Validation,
}

impl JwtTenantResolver {
    pub fn new(jwt_secret: &str) -> Self {
        let mut validation = Validation::new(Algorithm::HS256);
        validation.validate_exp = true;
        validation.validate_nbf = true;
        
        Self {
            decoding_key: DecodingKey::from_secret(jwt_secret.as_ref()),
            validation,
        }
    }
}

impl TenantResolver for JwtTenantResolver {
    type Error = JwtTenantError;
    
    async fn resolve_tenant(&self, token: &str) -> Result<TenantContext, Self::Error> {
        // Decode and validate JWT
        let token_data = decode::<TenantClaims>(
            token,
            &self.decoding_key,
            &self.validation
        )?;
        
        let claims = token_data.claims;
        
        // Convert JWT claims to tenant permissions
        let permissions = TenantPermissions {
            can_create: claims.scopes.contains(&"scim:create".to_string()),
            can_read: claims.scopes.contains(&"scim:read".to_string()),
            can_update: claims.scopes.contains(&"scim:update".to_string()),
            can_delete: claims.scopes.contains(&"scim:delete".to_string()),
            can_list: claims.scopes.contains(&"scim:list".to_string()),
            max_users: claims.resource_limits.max_users,
            max_groups: claims.resource_limits.max_groups,
        };
        
        // Build tenant context
        Ok(TenantContext::new(claims.tenant_id, claims.client_id)
            .with_permissions(permissions)
            .with_isolation_level(IsolationLevel::Standard)
            .with_metadata("subject", claims.sub))
    }
}
```

### Pattern 3: Database-Backed Resolution with Caching

Production-ready pattern for large-scale deployments:

```rust
use sqlx::{PgPool, Row};
use std::time::{Duration, Instant};

pub struct DatabaseTenantResolver {
    db_pool: PgPool,
    cache: Arc<RwLock<TenantCache>>,
    cache_ttl: Duration,
}

#[derive(Clone)]
struct CachedTenant {
    tenant_context: TenantContext,
    cached_at: Instant,
}

type TenantCache = HashMap<String, CachedTenant>;

impl DatabaseTenantResolver {
    pub fn new(db_pool: PgPool, cache_ttl: Duration) -> Self {
        Self {
            db_pool,
            cache: Arc::new(RwLock::new(HashMap::new())),
            cache_ttl,
        }
    }
    
    async fn fetch_tenant_from_db(&self, api_key: &str) -> Result<TenantContext, DatabaseError> {
        let row = sqlx::query!(
            r#"
            SELECT 
                t.tenant_id, 
                t.client_id, 
                t.isolation_level,
                t.is_active,
                tp.can_create,
                tp.can_read, 
                tp.can_update, 
                tp.can_delete,
                tp.can_list,
                tp.max_users,
                tp.max_groups
            FROM tenants t
            JOIN tenant_permissions tp ON t.tenant_id = tp.tenant_id
            WHERE t.api_key = $1 AND t.is_active = true
            "#,
            api_key
        )
        .fetch_optional(&self.db_pool)
        .await?
        .ok_or(DatabaseError::TenantNotFound)?;
        
        let permissions = TenantPermissions {
            can_create: row.can_create,
            can_read: row.can_read,
            can_update: row.can_update,
            can_delete: row.can_delete,
            can_list: row.can_list,
            max_users: row.max_users.map(|n| n as usize),
            max_groups: row.max_groups.map(|n| n as usize),
        };
        
        let isolation_level = match row.isolation_level.as_str() {
            "strict" => IsolationLevel::Strict,
            "shared" => IsolationLevel::Shared,
            _ => IsolationLevel::Standard,
        };
        
        Ok(TenantContext::new(row.tenant_id, row.client_id)
            .with_permissions(permissions)
            .with_isolation_level(isolation_level))
    }
    
    fn is_cache_valid(&self, cached_tenant: &CachedTenant) -> bool {
        cached_tenant.cached_at.elapsed() < self.cache_ttl
    }
}

impl TenantResolver for DatabaseTenantResolver {
    type Error = DatabaseTenantError;
    
    async fn resolve_tenant(&self, api_key: &str) -> Result<TenantContext, Self::Error> {
        // 1. Check cache first
        {
            let cache = self.cache.read().await;
            if let Some(cached) = cache.get(api_key) {
                if self.is_cache_valid(cached) {
                    return Ok(cached.tenant_context.clone());
                }
            }
        }
        
        // 2. Fetch from database
        let tenant_context = self.fetch_tenant_from_db(api_key).await?;
        
        // 3. Update cache
        {
            let mut cache = self.cache.write().await;
            cache.insert(api_key.to_string(), CachedTenant {
                tenant_context: tenant_context.clone(),
                cached_at: Instant::now(),
            });
        }
        
        Ok(tenant_context)
    }
}
```

## URL Generation Strategies

Different deployment patterns require different URL generation approaches:

### Strategy 1: Subdomain-Based Tenancy

```rust
use scim_server::TenantStrategy;

pub struct SubdomainTenantStrategy {
    base_domain: String,
    use_https: bool,
}

impl SubdomainTenantStrategy {
    pub fn new(base_domain: String, use_https: bool) -> Self {
        Self { base_domain, use_https }
    }
    
    pub fn generate_tenant_base_url(&self, tenant_id: &str) -> String {
        let protocol = if self.use_https { "https" } else { "http" };
        format!("{}://{}.{}", protocol, tenant_id, self.base_domain)
    }
    
    pub fn generate_resource_url(
        &self,
        tenant_id: &str,
        resource_type: &str,
        resource_id: &str,
    ) -> String {
        format!(
            "{}/scim/v2/{}/{}",
            self.generate_tenant_base_url(tenant_id),
            resource_type,
            resource_id
        )
    }
    
    pub fn extract_tenant_from_host(&self, host: &str) -> Option<String> {
        if let Some(subdomain) = host.strip_suffix(&format!(".{}", self.base_domain)) {
            if !subdomain.contains('.') {
                return Some(subdomain.to_string());
            }
        }
        None
    }
}

// Usage in HTTP handler
async fn extract_tenant_from_request(
    req: &HttpRequest,
    strategy: &SubdomainTenantStrategy,
) -> Result<String, TenantExtractionError> {
    let host = req.headers()
        .get("host")
        .and_then(|h| h.to_str().ok())
        .ok_or(TenantExtractionError::MissingHost)?;
        
    strategy.extract_tenant_from_host(host)
        .ok_or(TenantExtractionError::InvalidSubdomain)
}
```

### Strategy 2: Path-Based Tenancy

```rust
pub struct PathBasedTenantStrategy {
    base_url: String,
}

impl PathBasedTenantStrategy {
    pub fn new(base_url: String) -> Self {
        Self { base_url }
    }
    
    pub fn generate_resource_url(
        &self,
        tenant_id: &str,
        resource_type: &str,
        resource_id: &str,
    ) -> String {
        format!(
            "{}/tenants/{}/scim/v2/{}/{}",
            self.base_url,
            tenant_id,
            resource_type,
            resource_id
        )
    }
    
    pub fn extract_tenant_from_path(&self, path: &str) -> Option<String> {
        // Path format: /tenants/{tenant_id}/scim/v2/...
        let parts: Vec<&str> = path.split('/').collect();
        if parts.len() >= 3 && parts[1] == "tenants" {
            Some(parts[2].to_string())
        } else {
            None
        }
    }
}

// Usage in routing
pub fn setup_tenant_routes(app: &mut App) {
    app.route(
        "/tenants/{tenant_id}/scim/v2/{resource_type}",
        web::post().to(create_resource_handler)
    )
    .route(
        "/tenants/{tenant_id}/scim/v2/{resource_type}/{resource_id}",
        web::get().to(get_resource_handler)
    );
}
```

### Strategy 3: Header-Based Tenancy

Useful for API gateways and proxy scenarios:

```rust
pub struct HeaderBasedTenantStrategy;

impl HeaderBasedTenantStrategy {
    pub fn extract_tenant_from_headers(
        headers: &HeaderMap,
    ) -> Result<String, TenantExtractionError> {
        // Try different header names in order of preference
        let header_names = ["x-tenant-id", "x-client-id", "tenant"];
        
        for header_name in &header_names {
            if let Some(header_value) = headers.get(*header_name) {
                if let Ok(tenant_id) = header_value.to_str() {
                    if !tenant_id.is_empty() {
                        return Ok(tenant_id.to_string());
                    }
                }
            }
        }
        
        Err(TenantExtractionError::MissingTenantHeader)
    }
    
    pub fn generate_resource_url(
        &self,
        base_url: &str,
        resource_type: &str,
        resource_id: &str,
    ) -> String {
        // Headers don't affect URLs in this strategy
        format!("{}/scim/v2/{}/{}", base_url, resource_type, resource_id)
    }
}
```

## Storage Isolation Patterns

### Strict Isolation Pattern

Complete separation with no shared data:

```rust
use scim_server::storage::{StorageProvider, StorageKey, StoragePrefix};

pub struct StrictIsolationProvider<S: StorageProvider> {
    inner_storage: S,
}

impl<S: StorageProvider> StrictIsolationProvider<S> {
    pub fn new(inner_storage: S) -> Self {
        Self { inner_storage }
    }
    
    fn tenant_scoped_key(&self, tenant_id: &str, original_key: &StorageKey) -> StorageKey {
        StorageKey::new(format!("tenant:{}:{}", tenant_id, original_key.as_str()))
    }
    
    fn tenant_scoped_prefix(&self, tenant_id: &str, original_prefix: &StoragePrefix) -> StoragePrefix {
        StoragePrefix::new(format!("tenant:{}:{}", tenant_id, original_prefix.as_str()))
    }
}

impl<S: StorageProvider> StorageProvider for StrictIsolationProvider<S> {
    type Error = S::Error;
    
    async fn put(
        &self,
        key: StorageKey,
        data: Value,
        context: &RequestContext,
    ) -> Result<Value, Self::Error> {
        let tenant_id = context.tenant_id()
            .ok_or_else(|| StorageError::TenantRequired)?;
            
        let scoped_key = self.tenant_scoped_key(tenant_id, &key);
        self.inner_storage.put(scoped_key, data, context).await
    }
    
    async fn get(
        &self,
        key: StorageKey,
        context: &RequestContext,
    ) -> Result<Option<Value>, Self::Error> {
        let tenant_id = context.tenant_id()
            .ok_or_else(|| StorageError::TenantRequired)?;
            
        let scoped_key = self.tenant_scoped_key(tenant_id, &key);
        self.inner_storage.get(scoped_key, context).await
    }
    
    async fn list(
        &self,
        prefix: StoragePrefix,
        context: &RequestContext,
    ) -> Result<Vec<Value>, Self::Error> {
        let tenant_id = context.tenant_id()
            .ok_or_else(|| StorageError::TenantRequired)?;
            
        let scoped_prefix = self.tenant_scoped_prefix(tenant_id, &prefix);
        let results = self.inner_storage.list(scoped_prefix, context).await?;
        
        // Additional validation to ensure no cross-tenant data leakage
        let expected_prefix = format!("tenant:{}:", tenant_id);
        let filtered_results: Vec<Value> = results
            .into_iter()
            .filter(|item| {
                if let Some(id) = item.get("id").and_then(|v| v.as_str()) {
                    id.starts_with(&expected_prefix)
                } else {
                    false
                }
            })
            .collect();
            
        Ok(filtered_results)
    }
    
    async fn delete(
        &self,
        key: StorageKey,
        context: &RequestContext,
    ) -> Result<bool, Self::Error> {
        let tenant_id = context.tenant_id()
            .ok_or_else(|| StorageError::TenantRequired)?;
            
        let scoped_key = self.tenant_scoped_key(tenant_id, &key);
        self.inner_storage.delete(scoped_key, context).await
    }
}
```

### Standard Isolation with Shared Resources

Allows some shared data while maintaining tenant boundaries:

```rust
pub struct StandardIsolationProvider<S: StorageProvider> {
    inner_storage: S,
    shared_prefixes: HashSet<String>,
}

impl<S: StorageProvider> StandardIsolationProvider<S> {
    pub fn new(inner_storage: S) -> Self {
        let mut shared_prefixes = HashSet::new();
        shared_prefixes.insert("schema:".to_string());
        shared_prefixes.insert("config:".to_string());
        
        Self {
            inner_storage,
            shared_prefixes,
        }
    }
    
    fn should_scope_key(&self, key: &StorageKey) -> bool {
        !self.shared_prefixes.iter()
            .any(|prefix| key.as_str().starts_with(prefix))
    }
    
    fn apply_tenant_scoping(
        &self,
        key: StorageKey,
        context: &RequestContext,
    ) -> StorageKey {
        if self.should_scope_key(&key) {
            if let Some(tenant_id) = context.tenant_id() {
                StorageKey::new(format!("tenant:{}:{}", tenant_id, key.as_str()))
            } else {
                key
            }
        } else {
            key
        }
    }
}

impl<S: StorageProvider> StorageProvider for StandardIsolationProvider<S> {
    type Error = S::Error;
    
    async fn put(
        &self,
        key: StorageKey,
        data: Value,
        context: &RequestContext,
    ) -> Result<Value, Self::Error> {
        let scoped_key = self.apply_tenant_scoping(key, context);
        self.inner_storage.put(scoped_key, data, context).await
    }
    
    // Similar implementations for get, list, delete...
}
```

## Permission Enforcement Patterns

### Resource Limit Enforcement

```rust
use scim_server::multi_tenant::TenantValidator;

pub struct ResourceLimitValidator;

impl TenantValidator for ResourceLimitValidator {
    async fn validate_create_operation(
        &self,
        resource_type: &str,
        context: &RequestContext,
        storage: &impl StorageProvider,
    ) -> Result<(), ValidationError> {
        let tenant_context = context.tenant_context()
            .ok_or(ValidationError::TenantRequired)?;
            
        match resource_type {
            "User" => {
                if let Some(max_users) = tenant_context.permissions.max_users {
                    let current_count = self.count_resources("User", context, storage).await?;
                    if current_count >= max_users {
                        return Err(ValidationError::ResourceLimitExceeded {
                            resource_type: "User".to_string(),
                            current: current_count,
                            limit: max_users,
                        });
                    }
                }
            },
            "Group" => {
                if let Some(max_groups) = tenant_context.permissions.max_groups {
                    let current_count = self.count_resources("Group", context, storage).await?;
                    if current_count >= max_groups {
                        return Err(ValidationError::ResourceLimitExceeded {
                            resource_type: "Group".to_string(),
                            current: current_count,
                            limit: max_groups,
                        });
                    }
                }
            },
            _ => {
                // Custom resource type validation
            }
        }
        
        Ok(())
    }
    
    async fn count_resources(
        &self,
        resource_type: &str,
        context: &RequestContext,
        storage: &impl StorageProvider,
    ) -> Result<usize, ValidationError> {
        let prefix = StoragePrefix::new(format!("{}:", resource_type.to_lowercase()));
        let resources = storage.list(prefix, context).await?;
        Ok(resources.len())
    }
}
```

### Operation Permission Enforcement

```rust
pub fn validate_operation_permission(
    operation: ScimOperation,
    context: &RequestContext,
) -> Result<(), PermissionError> {
    let tenant_context = context.tenant_context()
        .ok_or(PermissionError::TenantRequired)?;
        
    let has_permission = match operation {
        ScimOperation::Create => tenant_context.permissions.can_create,
        ScimOperation::GetById | ScimOperation::List => tenant_context.permissions.can_read,
        ScimOperation::Update => tenant_context.permissions.can_update,
        ScimOperation::Delete => tenant_context.permissions.can_delete,
    };
    
    if !has_permission {
        return Err(PermissionError::OperationNotAllowed {
            operation,
            tenant_id: tenant_context.tenant_id.clone(),
        });
    }
    
    Ok(())
}
```

## Complete Integration Example

Here's how all patterns work together in a production setup:

```rust
use axum::{extract::Path, http::HeaderMap, Extension, Json};
use scim_server::{ScimServer, RequestContext, ScimOperationHandler};

pub async fn multi_tenant_scim_handler(
    Path(path_params): Path<HashMap<String, String>>,
    headers: HeaderMap,
    Json(body): Json<Value>,
    Extension(server): Extension<Arc<ScimServer<MultiTenantProvider>>>,
    Extension(tenant_resolver): Extension<Arc<DatabaseTenantResolver>>,
    Extension(url_strategy): Extension<Arc<SubdomainTenantStrategy>>,
) -> Result<Json<Value>, ScimError> {
    // 1. Extract tenant identifier using configured strategy
    let tenant_id = match url_strategy.extract_tenant_from_host(
        headers.get("host").and_then(|h| h.to_str().ok()).unwrap_or("")
    ) {
        Some(id) => id,
        None => return Err(ScimError::TenantNotFound("Unable to determine tenant".into())),
    };
    
    // 2. Extract API key for authentication
    let api_key = headers.get("authorization")
        .and_then(|h| h.to_str().ok())
        .and_then(|s| s.strip_prefix("Bearer "))
        .ok_or(ScimError::Unauthorized("Missing or invalid API key".into()))?;
    
    // 3. Resolve tenant context
    let tenant_context = tenant_resolver.resolve_tenant(api_key).await
        .map_err(|e| ScimError::TenantResolutionFailed(e.to_string()))?;
    
    // 4. Validate tenant ID matches resolved context
    if tenant_context.tenant_id != tenant_id {
        return Err(ScimError::TenantMismatch {
            requested: tenant_id,
            resolved: tenant_context.tenant_id,
        });
    }
    
    // 5. Create request context
    let request_context = RequestContext::with_tenant_generated_id(tenant_context);
    
    // 6. Build SCIM operation request
    let operation_request = ScimOperationRequest::from_http_parts(
        &path_params,
        &headers,
        body,
    )?;
    
    // 7. Validate operation permissions
    validate_operation_permission(
        operation_request.operation_type(),
        &request_context,
    )?;
    
    // 8. Process operation
    let handler = ScimOperationHandler::new(&server);
    let response = handler
        .handle_operation(operation_request, &request_context)
        .await?;
    
    // 9. Apply tenant-specific URL generation to response
    let mut response_json = response.into_json();
    if let Some(location) = response_json.get_mut("meta").and_then(|m| m.get_mut("location")) {
        if let Some(resource_id) = response_json.get("id").and_then(|v| v.as_str()) {
            let tenant_url = url_strategy.generate_resource_url(
                &tenant_id,
                &operation_request.resource_type,
                resource_id,
            );
            *location = Value::String(tenant_url);
        }
    }
    
    Ok(Json(response_json))
}
```

## Testing Multi-Tenant Patterns

### Integration Test Setup

```rust
#[tokio::test]
async fn test_complete_multi_tenant_flow() {
    // Setup multi-tenant infrastructure
    let storage = InMemoryStorage::new();
    let isolated_storage = StrictIsolationProvider::new(storage);
    let provider = StandardResourceProvider::new(isolated_storage);
    let server = ScimServer::new(provider).unwrap();
    
    // Setup tenant resolver
    let tenant_resolver = StaticTenantResolver::new();
    
    // Add test tenants
    let tenant_a = TenantContext::new("tenant-a".into(), "client-a".into())
        .with_permissions(TenantPermissions {
            max_users: Some(10),
            ..Default::default()
        });
    let tenant_b = TenantContext::new("tenant-b".into(), "client-b".into());
    
    tenant_resolver.add_tenant("api-key-a", tenant_a).await;
    tenant_resolver.add_tenant("api-key-b", tenant_b).await;
    
    // Test tenant isolation
    let context_a = RequestContext::with_tenant_generated_id(
        tenant_resolver.resolve_tenant("api-key-a").await.unwrap()
    );
    let context_b = RequestContext::with_tenant_generated_id(
        tenant_resolver.resolve_tenant("api-key-b").await.unwrap()
    );
    
    // Create resources in different tenants
    let handler = ScimOperationHandler::new(&server);
    
    let user_a = handler.handle_create(
        "User",
        json!({"userName": "user.a", "displayName": "User A"}),
        &context_a,
    ).await.unwrap();
    
    let user_b = handler.handle_create(
        "User", 
        json!({"userName": "user.b", "displayName": "User B"}),
        &context_b,
    ).await.unwrap();
    
    // Verify isolation - tenant A cannot see tenant B's user
    let tenant_a_users = handler.handle_list("User", &ListQuery::default(), &context_a)
        .await.unwrap();
    assert_eq!(tenant_a_users.resources.len(), 1);
    assert_eq!(tenant_a_users.resources[0].get("userName").unwrap(), "user.a");
    
    let tenant_b_users = handler.handle_list("User", &ListQuery::default(), &context_b)
        .await.unwrap();
    assert_eq!(tenant_b_users.resources.len(), 1);
    assert_eq!(tenant_b_users.resources[0].get("userName").unwrap(), "user.b");
    
    // Test resource limit enforcement for tenant A
    let mut create_futures = Vec::new();
    for i in 1..15 {  // Try to create more than the limit of 10
        let user_data = json!({
            "userName": format!("user.a.{}", i),
            "displayName": format!("User A {}", i)
        });
        create_futures.push(handler.handle_create("User", user_data, &context_a));
    }
    
    let results = futures::future::join_all(create_futures).await;
    let successful_creates = results.into_iter().filter(|r| r.is_ok()).count();
    
    // Should only allow 9 more users (10 total - 1 already created)
    assert_eq!(successful_creates, 9);
}
```

## Migration Patterns

### Single-Tenant to Multi-Tenant Migration

```rust
pub struct TenantMigrationService<S: StorageProvider> {
    storage: S,
    default_tenant_id: String,
}

impl<S: StorageProvider> TenantMigrationService<S> {
    pub async fn migrate_to_multi_tenant(
        &self,
        default_client_id: String,
    ) -> Result<MigrationReport, MigrationError> {
        let mut report = MigrationReport::new();
        
        // 1. Create default tenant context
        let default_tenant = TenantContext::new(
            self.default_tenant_id.clone(),
            default_client_id,
        );
        
        // 2. Migrate existing users
        let users = self.storage
            .list(StoragePrefix::new("user:"), &RequestContext::migration())
            .await?;
            
        for user in users {
            let old_key = StorageKey::from_resource(&user)?;
            let new_key = StorageKey::new(format!(
                "tenant:{}:{}",
                self.default_tenant_id,
                old_key.as_str()
            ));
            
            // Copy to new location
            self.storage.put(new_key, user.clone(), &RequestContext::migration()).await?;
            report.users_migrated += 1;
        }
        
        // 3. Migrate existing groups
        let groups = self.storage
            .list(StoragePrefix::new("group:"), &RequestContext::migration())
            .await?;
            
        for group in groups {
            let old_key = StorageKey::from_resource(&group)?;
            let new_key = StorageKey::new(format!(
                "tenant:{}:{}",
                self.default_tenant_id,
                old_key.as_str()
            ));
            
            self.storage.put(new_key, group.clone(), &RequestContext::migration()).await?;
            report.groups_migrated += 1;
        }
        
        // 4. Clean up old data (optional, use with caution)
        if report.cleanup_old_data {
            self.cleanup_pre_migration_data().await?;
        }
        
        Ok(report)
    }
}

#[derive(Debug)]
pub struct MigrationReport {
    pub users_migrated: usize,
    pub groups_migrated: usize,
    pub errors: Vec<MigrationError>,
    pub cleanup_old_data: bool,
}
```

## Production Considerations

### Monitoring and Observability

```rust
use tracing::{info, warn, error, span, Level};
use metrics::{counter, histogram, gauge};

pub struct MultiTenantMetrics;

impl MultiTenantMetrics {
    pub fn record_tenant_operation(
        &self,
        tenant_id: &str,
        operation: &str,
        resource_type: &str,
        duration: Duration,
        success: bool,
    ) {
        // Record operation metrics
        counter!("scim_operations_total")
            .with_tag("tenant_id", tenant_id)
            .with_tag("operation", operation)
            .with_tag("resource_type", resource_type)
            .with_tag("success", success.to_string())
            .increment(1);
            
        histogram!("scim_operation_duration_seconds")
            .with_tag("tenant_id", tenant_id)
            .with_tag("operation", operation)
            .record(duration.as_secs_f64());
    }
    
    pub fn record_tenant_resource_count(
        &self,
        tenant_id: &str,
        resource_type: &str,
        count: usize,
    ) {
        gauge!("scim_tenant_resources")
            .with_tag("tenant_id", tenant_id)
            .with_tag("resource_type", resource_type)
            .set(count as f64);
    }
    
    pub fn record_tenant_limit_approaching(
        &self,
        tenant_id: &str,
        resource_type: &str,
        current: usize,
        limit: usize,
    ) {
        let utilization = (current as f64 / limit as f64) * 100.0;
        
        gauge!("scim_tenant_limit_utilization")
            .with_tag("tenant_id", tenant_id)
            .with_tag("resource_type", resource_type)
            .set(utilization);
            
        if utilization > 80.0 {
            warn!(
                tenant_id = %tenant_id,
                resource_type = %resource_type,
                current = current,
                limit = limit,
                utilization = %format!("{:.1}%", utilization),
                "Tenant approaching resource limit"
            );
        }
    }
}
```

### Health Checks and Diagnostics

```rust
pub struct MultiTenantHealthCheck<S: StorageProvider> {
    storage: S,
    tenant_resolver: Arc<dyn TenantResolver>,
}

impl<S: StorageProvider> MultiTenantHealthCheck<S> {
    pub async fn check_tenant_health(
        &self,
        tenant_id: &str,
    ) -> Result<TenantHealthReport, HealthCheckError> {
        let mut report = TenantHealthReport::new(tenant_id.to_string());
        
        // Check tenant resolution
        let test_credential = format!("health-check-{}", tenant_id);
        match self.tenant_resolver.resolve_tenant(&test_credential).await {
            Ok(_) => report.tenant_resolution = HealthStatus::Healthy,
            Err(e) => {
                report.tenant_resolution = HealthStatus::Unhealthy(e.to_string());
                report.overall_health = HealthStatus::Degraded;
            }
        }
        
        // Check storage connectivity for tenant
        let test_context = RequestContext::with_tenant_id(tenant_id.to_string());
        let test_key = StorageKey::new("health-check");
        let test_data = json!({"health": "check", "timestamp": chrono::Utc::now()});
        
        match self.storage.put(test_key.clone(), test_data, &test_context).await {
            Ok(_) => {
                report.storage_write = HealthStatus::Healthy;
                
                // Test read
                match self.storage.get(test_key.clone(), &test_context).await {
                    Ok(Some(_)) => report.storage_read = HealthStatus::Healthy,
                    Ok(None) => report.storage_read = HealthStatus::Unhealthy("Data not found".into()),
                    Err(e) => report.storage_read = HealthStatus::Unhealthy(e.to_string()),
                }
                
                // Cleanup
                let _ = self.storage.delete(test_key, &test_context).await;
            },
            Err(e) => {
                report.storage_write = HealthStatus::Unhealthy(e.to_string());
                report.overall_health = HealthStatus::Unhealthy("Storage write failed".into());
            }
        }
        
        Ok(report)
    }
}

#[derive(Debug)]
pub struct TenantHealthReport {
    pub tenant_id: String,
    pub overall_health: HealthStatus,
    pub tenant_resolution: HealthStatus,
    pub storage_read: HealthStatus,
    pub storage_write: HealthStatus,
    pub checked_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug)]
pub enum HealthStatus {
    Healthy,
    Degraded,
    Unhealthy(String),
}
```

## Best Practices Summary

### Configuration Management
- **Use environment-specific tenant configurations** for different deployment stages
- **Implement tenant configuration hot-reloading** for operational flexibility
- **Validate tenant configurations at startup** to catch errors early

### Security Considerations
- **Always validate tenant boundaries** in storage operations
- **Use strong isolation levels for sensitive data**
- **Implement audit logging for all multi-tenant operations**
- **Regularly rotate tenant credentials and API keys**

### Performance Optimization  
- **Cache tenant resolution results** with appropriate TTL
- **Use connection pooling** for database-backed tenant resolvers
- **Implement pagination for large tenant lists**
- **Monitor tenant-specific resource usage patterns**

### Operational Excellence
- **Implement comprehensive health checks** for all tenant components
- **Use structured logging with tenant context** for debugging
- **Set up alerts for tenant limit violations**
- **Plan for tenant migration and evolution scenarios**

## Related Topics

- **[Request Lifecycle & Context Management](./request-lifecycle.md)** - How tenant context flows through requests
- **[Resource Provider Architecture](./resource-provider-architecture.md)** - Implementing tenant-aware business logic
- **[Multi-Tenant Architecture](../concepts/multi-tenant-architecture.md)** - Core concepts and components
- **[Multi-Tenant Server Example](../examples/multi-tenant.md)** - Complete implementation example

## Next Steps

Now that you understand multi-tenant architecture patterns:

1. **Choose your tenant strategy** (subdomain, path-based, or header-based)
2. **Implement tenant resolution** for your authentication system
3. **Configure storage isolation** based on your security requirements  
4. **Set up monitoring and health checks** for production deployment
5. **Plan tenant migration strategies** for future scalability needs