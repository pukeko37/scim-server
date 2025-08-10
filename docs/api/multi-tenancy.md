# Multi-tenancy API Reference

This document provides comprehensive documentation for the multi-tenancy features in the SCIM Server crate. Multi-tenancy allows a single SCIM server instance to serve multiple isolated tenants, making it ideal for SaaS applications.

## Table of Contents

- [Overview](#overview)
- [Core Types](#core-types)
- [Tenant Resolution](#tenant-resolution)
- [Request Context](#request-context)
- [Integration Patterns](#integration-patterns)
- [Security Considerations](#security-considerations)
- [Best Practices](#best-practices)

## Overview

The multi-tenancy system is built around three core concepts:

1. **TenantContext** - Identifies which tenant a request belongs to
2. **TenantResolver** - Determines tenant from incoming requests
3. **RequestContext** - Carries tenant information through operations

### Key Benefits

- **Complete Isolation**: Tenants cannot access each other's data
- **Flexible Resolution**: Multiple strategies for determining tenant context
- **Type Safety**: Compile-time guarantees for tenant handling
- **Performance**: Minimal overhead for tenant context propagation
- **Auditability**: Full tenant tracking for compliance and debugging

## Core Types

### TenantContext

Represents the identity and isolation context for a specific tenant.

#### Definition

```rust
pub struct TenantContext {
    tenant_id: String,
    client_id: String,
}
```

#### Construction

```rust
use scim_server::multi_tenant::TenantContext;

fn main() {
    // Create tenant context with explicit IDs
    let context = TenantContext::new(
        "company-abc".to_string(), 
        "client-xyz".to_string()
    );
    
    println!("Tenant: {}", context.tenant_id());
    println!("Client: {}", context.client_id());
}
```

#### Methods

```rust
impl TenantContext {
    /// Create a new tenant context
    pub fn new(tenant_id: String, client_id: String) -> Self
    
    /// Get the tenant identifier
    pub fn tenant_id(&self) -> &str
    
    /// Get the client identifier  
    pub fn client_id(&self) -> &str
    
    /// Create a tenant-specific key for storage isolation
    pub fn scoped_key(&self, key: &str) -> String
}
```

#### Usage Examples

```rust
use scim_server::multi_tenant::TenantContext;

fn tenant_context_examples() {
    let context = TenantContext::new("acme-corp".to_string(), "app-123".to_string());
    
    // Access tenant information
    assert_eq!(context.tenant_id(), "acme-corp");
    assert_eq!(context.client_id(), "app-123");
    
    // Create scoped storage keys
    let user_key = context.scoped_key("users:john-doe");
    // Result: "acme-corp:app-123:users:john-doe"
    
    let schema_key = context.scoped_key("schemas:User");
    // Result: "acme-corp:app-123:schemas:User"
}
```

### TenantResolver

Trait for implementing tenant resolution strategies.

#### Definition

```rust
pub trait TenantResolver {
    type Error: std::error::Error + Send + Sync + 'static;
    
    async fn resolve_tenant(&self, request: &TenantResolutionRequest) -> Result<TenantContext, Self::Error>;
}
```

#### TenantResolutionRequest

Contains information from the incoming request used for tenant resolution:

```rust
pub struct TenantResolutionRequest {
    pub host: Option<String>,
    pub path: Option<String>,
    pub headers: HashMap<String, String>,
    pub client_certificate: Option<ClientCertificate>,
}
```

### StaticTenantResolver

A simple implementation that maps clients to tenants using a static configuration.

#### Definition

```rust
pub struct StaticTenantResolver {
    // Internal mapping from client_id to tenant information
}
```

#### Construction

```rust
use scim_server::multi_tenant::StaticTenantResolver;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Using builder pattern
    let resolver = StaticTenantResolver::builder()
        .add_tenant("tenant-1", "client-abc")
        .add_tenant("tenant-2", "client-def")
        .add_tenant("tenant-3", "client-ghi")
        .build();
    
    println!("Resolver configured with 3 tenants");
    Ok(())
}
```

#### Methods

```rust
impl StaticTenantResolver {
    /// Create a new builder for configuring tenant mappings
    pub fn builder() -> StaticTenantResolverBuilder
    
    /// Add a tenant mapping (for direct construction)
    pub fn add_tenant(&mut self, tenant_id: &str, client_id: &str) -> Result<(), ValidationError>
    
    /// Get all configured tenants
    pub fn tenants(&self) -> Vec<&TenantContext>
    
    /// Check if a tenant exists
    pub fn has_tenant(&self, tenant_id: &str) -> bool
}
```

#### Builder Pattern

```rust
use scim_server::multi_tenant::StaticTenantResolverBuilder;

fn builder_example() -> Result<(), Box<dyn std::error::Error>> {
    let resolver = StaticTenantResolverBuilder::new()
        .add_tenant("production-tenant", "prod-client-123")
        .add_tenant("staging-tenant", "stage-client-456")
        .add_tenant("development-tenant", "dev-client-789")
        .build();
    
    // Resolver is ready to use
    Ok(())
}
```

## Tenant Resolution

### Resolution Strategies

#### By Client Certificate (Recommended for Production)

```rust
use scim_server::multi_tenant::{TenantResolver, TenantResolutionRequest};

pub struct CertificateTenantResolver {
    // Certificate to tenant mapping
}

impl TenantResolver for CertificateTenantResolver {
    type Error = TenantResolutionError;
    
    async fn resolve_tenant(&self, request: &TenantResolutionRequest) -> Result<TenantContext, Self::Error> {
        if let Some(cert) = &request.client_certificate {
            let tenant_id = extract_tenant_from_cert(cert)?;
            let client_id = extract_client_from_cert(cert)?;
            
            Ok(TenantContext::new(tenant_id, client_id))
        } else {
            Err(TenantResolutionError::MissingCertificate)
        }
    }
}
```

#### By HTTP Header

```rust
pub struct HeaderTenantResolver;

impl TenantResolver for HeaderTenantResolver {
    type Error = TenantResolutionError;
    
    async fn resolve_tenant(&self, request: &TenantResolutionRequest) -> Result<TenantContext, Self::Error> {
        let tenant_id = request.headers
            .get("X-Tenant-ID")
            .ok_or(TenantResolutionError::MissingTenantHeader)?;
            
        let client_id = request.headers
            .get("X-Client-ID")
            .ok_or(TenantResolutionError::MissingClientHeader)?;
        
        Ok(TenantContext::new(tenant_id.clone(), client_id.clone()))
    }
}
```

#### By Subdomain

```rust
pub struct SubdomainTenantResolver {
    domain_mapping: HashMap<String, TenantContext>,
}

impl TenantResolver for SubdomainTenantResolver {
    type Error = TenantResolutionError;
    
    async fn resolve_tenant(&self, request: &TenantResolutionRequest) -> Result<TenantContext, Self::Error> {
        let host = request.host
            .as_ref()
            .ok_or(TenantResolutionError::MissingHost)?;
        
        // Extract subdomain: "tenant1.api.example.com" -> "tenant1"
        let subdomain = host.split('.').next()
            .ok_or(TenantResolutionError::InvalidHost)?;
        
        self.domain_mapping
            .get(subdomain)
            .cloned()
            .ok_or(TenantResolutionError::UnknownTenant)
    }
}
```

### Custom Tenant Resolution

```rust
use scim_server::multi_tenant::{TenantResolver, TenantResolutionRequest, TenantContext};
use async_trait::async_trait;

pub struct DatabaseTenantResolver {
    db_pool: sqlx::PgPool,
}

#[async_trait]
impl TenantResolver for DatabaseTenantResolver {
    type Error = DatabaseTenantError;
    
    async fn resolve_tenant(&self, request: &TenantResolutionRequest) -> Result<TenantContext, Self::Error> {
        // Extract API key from Authorization header
        let api_key = request.headers
            .get("Authorization")
            .and_then(|auth| auth.strip_prefix("Bearer "))
            .ok_or(DatabaseTenantError::MissingApiKey)?;
        
        // Query database for tenant information
        let query = "SELECT tenant_id, client_id FROM api_keys WHERE key_hash = $1 AND active = true";
        let row: Option<(String, String)> = sqlx::query_as(query)
            .bind(hash_api_key(api_key))
            .fetch_optional(&self.db_pool)
            .await?;
        
        match row {
            Some((tenant_id, client_id)) => Ok(TenantContext::new(tenant_id, client_id)),
            None => Err(DatabaseTenantError::InvalidApiKey),
        }
    }
}
```

## Request Context

### Creating Tenant-Aware Contexts

```rust
use scim_server::resource::RequestContext;
use scim_server::multi_tenant::TenantContext;

fn create_contexts() {
    // Single-tenant operation (no tenant isolation)
    let single_context = RequestContext::with_generated_id();
    
    // Multi-tenant operation (with tenant isolation)
    let tenant_context = TenantContext::new("tenant-1".to_string(), "client-a".to_string());
    let multi_context = RequestContext::with_tenant_generated_id(tenant_context);
    
    // Custom operation ID with tenant
    let custom_context = RequestContext::with_tenant("custom-op-123".to_string(), tenant_context);
}
```

### Context Propagation

```rust
async fn context_propagation_example<P: ResourceProvider>(
    provider: &P,
    tenant_context: TenantContext
) -> Result<(), P::Error> {
    // Create tenant-aware context
    let context = RequestContext::with_tenant_generated_id(tenant_context);
    
    // All operations automatically include tenant context
    let user_data = json!({
        "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
        "userName": "tenant.user@example.com"
    });
    
    // Provider receives full context including tenant information
    let user = provider.create_resource("User", user_data, &context).await?;
    
    // Subsequent operations maintain tenant isolation
    let retrieved = provider.get_resource("User", user.id().unwrap().as_str(), &context).await?;
    
    Ok(())
}
```

## Integration Patterns

### Web Framework Integration

#### With Axum

```rust
use axum::{
    extract::{State, Request},
    middleware::{self, Next},
    response::Response,
    http::StatusCode,
};

// Middleware to extract tenant context
async fn tenant_middleware(
    mut request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    // Extract tenant information from request
    let tenant_id = request.headers()
        .get("X-Tenant-ID")
        .and_then(|h| h.to_str().ok())
        .ok_or(StatusCode::BAD_REQUEST)?;
    
    let client_id = request.headers()
        .get("X-Client-ID")
        .and_then(|h| h.to_str().ok())
        .ok_or(StatusCode::BAD_REQUEST)?;
    
    // Create tenant context
    let tenant_context = TenantContext::new(tenant_id.to_string(), client_id.to_string());
    let request_context = RequestContext::with_tenant_generated_id(tenant_context);
    
    // Add to request extensions
    request.extensions_mut().insert(request_context);
    
    Ok(next.run(request).await)
}

// Route handler that uses tenant context
async fn create_user_with_tenant(
    State(server): State<SharedServer>,
    request: Request,
    Json(payload): Json<Value>,
) -> Result<Json<Value>, StatusCode> {
    // Extract tenant context from middleware
    let context = request.extensions()
        .get::<RequestContext>()
        .ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;
    
    match server.create_resource("User", payload, context).await {
        Ok(resource) => Ok(Json(resource.to_json().unwrap())),
        Err(_) => Err(StatusCode::BAD_REQUEST),
    }
}

// Set up router with tenant middleware
fn create_app(server: SharedServer) -> Router {
    Router::new()
        .route("/Users", post(create_user_with_tenant))
        .route("/Users/:id", get(get_user_with_tenant))
        .layer(middleware::from_fn(tenant_middleware))
        .with_state(server)
}
```

#### With Warp

```rust
use warp::{Filter, Rejection, Reply};

// Extract tenant context from headers
fn with_tenant_context() -> impl Filter<Extract = (RequestContext,), Error = Rejection> + Clone {
    warp::header::<String>("x-tenant-id")
        .and(warp::header::<String>("x-client-id"))
        .map(|tenant_id: String, client_id: String| {
            let tenant_context = TenantContext::new(tenant_id, client_id);
            RequestContext::with_tenant_generated_id(tenant_context)
        })
}

// Route with tenant isolation
let users_route = warp::path("Users")
    .and(warp::post())
    .and(warp::body::json())
    .and(with_tenant_context())
    .and(with_server(server))
    .and_then(create_user_handler);
```

### Provider Integration

#### Database Provider with Tenant Isolation

```rust
use scim_server::{ResourceProvider, RequestContext};

impl ResourceProvider for DatabaseProvider {
    type Error = DatabaseError;

    fn create_resource(
        &self, 
        resource_type: &str, 
        data: Value, 
        context: &RequestContext
    ) -> impl Future<Output = Result<Resource, Self::Error>> + Send {
        async move {
            let resource = Resource::from_json(resource_type.to_string(), data)?;
            
            // Extract tenant information for isolation
            let tenant_id = context.tenant_context()
                .map(|t| t.tenant_id())
                .unwrap_or("default");
            
            // Store with tenant isolation
            let query = "INSERT INTO scim_resources (id, resource_type, data, tenant_id) VALUES ($1, $2, $3, $4)";
            sqlx::query(query)
                .bind(resource.id().unwrap().as_str())
                .bind(resource_type)
                .bind(serde_json::to_string(&resource.to_json()?)?)
                .bind(tenant_id)
                .execute(&self.pool)
                .await?;
            
            Ok(resource)
        }
    }

    fn get_resource(
        &self, 
        resource_type: &str, 
        id: &str, 
        context: &RequestContext
    ) -> impl Future<Output = Result<Option<Resource>, Self::Error>> + Send {
        async move {
            let tenant_id = context.tenant_context()
                .map(|t| t.tenant_id())
                .unwrap_or("default");
            
            // Query with tenant isolation
            let query = "SELECT data FROM scim_resources WHERE id = $1 AND resource_type = $2 AND tenant_id = $3";
            let row: Option<(String,)> = sqlx::query_as(query)
                .bind(id)
                .bind(resource_type)
                .bind(tenant_id)
                .fetch_optional(&self.pool)
                .await?;
            
            match row {
                Some((data,)) => {
                    let json: Value = serde_json::from_str(&data)?;
                    let resource = Resource::from_json(resource_type.to_string(), json)?;
                    Ok(Some(resource))
                }
                None => Ok(None),
            }
        }
    }

    fn list_resources(
        &self, 
        resource_type: &str, 
        query: Option<&ListQuery>, 
        context: &RequestContext
    ) -> impl Future<Output = Result<Vec<Resource>, Self::Error>> + Send {
        async move {
            let tenant_id = context.tenant_context()
                .map(|t| t.tenant_id())
                .unwrap_or("default");
            
            let (limit, offset) = query
                .map(|q| (q.count().unwrap_or(50), q.start_index().unwrap_or(1) - 1))
                .unwrap_or((50, 0));
            
            // List with tenant isolation and pagination
            let query_str = "SELECT data FROM scim_resources WHERE resource_type = $1 AND tenant_id = $2 LIMIT $3 OFFSET $4";
            let rows: Vec<(String,)> = sqlx::query_as(query_str)
                .bind(resource_type)
                .bind(tenant_id)
                .bind(limit as i64)
                .bind(offset as i64)
                .fetch_all(&self.pool)
                .await?;
            
            let mut resources = Vec::new();
            for (data,) in rows {
                let json: Value = serde_json::from_str(&data)?;
                let resource = Resource::from_json(resource_type.to_string(), json)?;
                resources.push(resource);
            }
            
            Ok(resources)
        }
    }
}
```

## Integration Patterns

### Pattern 1: Header-Based Tenant Resolution

```rust
use axum::{extract::Request, middleware::Next, response::Response};

async fn extract_tenant_from_headers(
    mut request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    let headers = request.headers();
    
    // Extract tenant from custom headers
    let tenant_id = headers.get("X-Tenant-ID")
        .and_then(|h| h.to_str().ok())
        .ok_or(StatusCode::BAD_REQUEST)?;
    
    let client_id = headers.get("X-Client-ID")
        .and_then(|h| h.to_str().ok())
        .ok_or(StatusCode::BAD_REQUEST)?;
    
    // Validate tenant access (implement your logic)
    validate_tenant_access(tenant_id, client_id)?;
    
    // Create and store context
    let tenant_context = TenantContext::new(tenant_id.to_string(), client_id.to_string());
    let request_context = RequestContext::with_tenant_generated_id(tenant_context);
    
    request.extensions_mut().insert(request_context);
    
    Ok(next.run(request).await)
}
```

### Pattern 2: JWT-Based Tenant Resolution

```rust
use jsonwebtoken::{decode, DecodingKey, Validation, Algorithm};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
struct TenantClaims {
    tenant_id: String,
    client_id: String,
    exp: usize,
}

pub struct JwtTenantResolver {
    decoding_key: DecodingKey,
    validation: Validation,
}

impl JwtTenantResolver {
    pub fn new(secret: &[u8]) -> Self {
        let mut validation = Validation::new(Algorithm::HS256);
        validation.set_issuer(&["your-issuer"]);
        
        Self {
            decoding_key: DecodingKey::from_secret(secret),
            validation,
        }
    }
}

impl TenantResolver for JwtTenantResolver {
    type Error = TenantResolutionError;
    
    async fn resolve_tenant(&self, request: &TenantResolutionRequest) -> Result<TenantContext, Self::Error> {
        let auth_header = request.headers
            .get("Authorization")
            .ok_or(TenantResolutionError::MissingAuthHeader)?;
        
        let token = auth_header.strip_prefix("Bearer ")
            .ok_or(TenantResolutionError::InvalidAuthFormat)?;
        
        let token_data = decode::<TenantClaims>(
            token,
            &self.decoding_key,
            &self.validation,
        )?;
        
        Ok(TenantContext::new(
            token_data.claims.tenant_id,
            token_data.claims.client_id,
        ))
    }
}
```

### Pattern 3: API Key-Based Resolution

```rust
pub struct ApiKeyTenantResolver {
    key_to_tenant: HashMap<String, TenantContext>,
}

impl ApiKeyTenantResolver {
    pub fn new() -> Self {
        Self {
            key_to_tenant: HashMap::new(),
        }
    }
    
    pub fn add_api_key(&mut self, api_key: String, tenant_context: TenantContext) {
        self.key_to_tenant.insert(api_key, tenant_context);
    }
}

impl TenantResolver for ApiKeyTenantResolver {
    type Error = TenantResolutionError;
    
    async fn resolve_tenant(&self, request: &TenantResolutionRequest) -> Result<TenantContext, Self::Error> {
        let api_key = request.headers
            .get("X-API-Key")
            .ok_or(TenantResolutionError::MissingApiKey)?;
        
        self.key_to_tenant
            .get(api_key)
            .cloned()
            .ok_or(TenantResolutionError::InvalidApiKey)
    }
}
```

## Security Considerations

### Tenant Isolation Principles

1. **Complete Data Isolation**: Tenants must never see each other's data
2. **Request Validation**: Always validate tenant access before processing
3. **Context Propagation**: Ensure tenant context flows through entire request pipeline
4. **Audit Trail**: Log all tenant-specific operations for compliance

### Implementation Checklist

#### Provider Implementation
- [ ] All database queries include tenant_id in WHERE clauses
- [ ] Indexes include tenant_id for performance
- [ ] Migrations properly set up tenant-specific tables/schemas
- [ ] Connection pooling respects tenant boundaries when needed

#### Request Processing
- [ ] Tenant resolution happens before any resource operations
- [ ] Invalid tenant requests are rejected early
- [ ] Tenant context is immutable once set
- [ ] All operations use the resolved tenant context

#### Error Handling
- [ ] Tenant resolution errors are logged but don't leak information
- [ ] Resource not found vs. tenant not authorized are distinguished
- [ ] Error messages don't reveal information about other tenants

### Example Security Implementation

```rust
use scim_server::multi_tenant::TenantContext;

pub struct SecureTenantValidator;

impl SecureTenantValidator {
    /// Validate that the tenant is authorized for this operation
    pub fn validate_tenant_access(
        &self,
        tenant_context: &TenantContext,
        requested_resource_id: &str,
        operation: &str,
    ) -> Result<(), SecurityError> {
        // Implement your authorization logic
        
        // Example: Check tenant permissions
        if !self.has_permission(tenant_context.tenant_id(), operation) {
            return Err(SecurityError::InsufficientPermissions);
        }
        
        // Example: Validate resource ownership
        if !self.owns_resource(tenant_context.tenant_id(), requested_resource_id) {
            return Err(SecurityError::ResourceNotFound); // Don't reveal it exists
        }
        
        Ok(())
    }
    
    /// Rate limiting per tenant
    pub async fn check_rate_limit(&self, tenant_context: &TenantContext) -> Result<(), SecurityError> {
        let key = format!("rate_limit:{}", tenant_context.tenant_id());
        
        if self.rate_limiter.is_rate_limited(&key).await {
            return Err(SecurityError::RateLimitExceeded);
        }
        
        Ok(())
    }
}
```

### Audit Logging

```rust
use log::{info, warn, error};

fn audit_tenant_operation(
    tenant_context: &TenantContext,
    operation: &str,
    resource_type: &str,
    resource_id: Option<&str>,
    result: &str,
) {
    info!(
        "tenant_operation";
        "tenant_id" => tenant_context.tenant_id(),
        "client_id" => tenant_context.client_id(),
        "operation" => operation,
        "resource_type" => resource_type,
        "resource_id" => resource_id.unwrap_or("N/A"),
        "result" => result,
    );
}

// Usage in provider
async fn audited_create_resource<P: ResourceProvider>(
    provider: &P,
    resource_type: &str,
    data: Value,
    context: &RequestContext,
) -> Result<Resource, P::Error> {
    let start_time = std::time::Instant::now();
    
    match provider.create_resource(resource_type, data, context).await {
        Ok(resource) => {
            if let Some(tenant) = context.tenant_context() {
                audit_tenant_operation(
                    tenant,
                    "CREATE",
                    resource_type,
                    resource.id().map(|id| id.as_str()),
                    "SUCCESS",
                );
            }
            Ok(resource)
        }
        Err(e) => {
            if let Some(tenant) = context.tenant_context() {
                audit_tenant_operation(
                    tenant,
                    "CREATE",
                    resource_type,
                    None,
                    "FAILED",
                );
            }
            Err(e)
        }
    }
}
```

## Best Practices

### Tenant Resolution
1. **Resolve early**: Determine tenant context as early as possible in request processing
2. **Validate thoroughly**: Ensure tenant has permission for the requested operation
3. **Cache resolution**: Cache tenant resolution results to avoid repeated lookups
4. **Handle errors gracefully**: Provide clear errors without leaking tenant information

### Context Management
1. **Immutable contexts**: Never modify tenant context after creation
2. **Propagate completely**: Ensure tenant context flows through entire operation
3. **Log operations**: Include tenant information in all operation logs
4. **Monitor isolation**: Regularly verify tenant isolation is working correctly

### Performance Optimization
1. **Index by tenant**: Include tenant_id in all database indexes
2. **Connection pooling**: Consider per-tenant connection pools for high-isolation needs
3. **Caching**: Implement tenant-aware caching strategies
4. **Batch operations**: Group operations by tenant when possible

### Error Handling
1. **Specific errors**: Use specific error types for tenant-related failures
2. **Security-conscious**: Don't leak information about other tenants in error messages
3. **Graceful degradation**: Handle tenant resolution failures gracefully
4. **Monitoring**: Alert on tenant resolution failures

## Advanced Usage

### Dynamic Tenant Configuration

```rust
pub struct DynamicTenantResolver {
    tenant_service: Arc<dyn TenantService + Send + Sync>,
}

#[async_trait]
impl TenantResolver for DynamicTenantResolver {
    type Error = TenantResolutionError;
    
    async fn resolve_tenant(&self, request: &TenantResolutionRequest) -> Result<TenantContext, Self::Error> {
        let api_key = extract_api_key(request)?;
        
        // Dynamic lookup from external service
        let tenant_info = self.tenant_service
            .get_tenant_by_api_key(&api_key)
            .await?;
        
        Ok(TenantContext::new(tenant_info.tenant_id, tenant_info.client_id))
    }
}
```

### Tenant-Specific Configuration

```rust
pub struct TenantConfigResolver {
    configs: HashMap<String, TenantConfig>,
}

#[derive(Clone)]
pub struct TenantConfig {
    pub max_users: usize,
    pub allowed_schemas: Vec<String>,
    pub custom_attributes: HashMap<String, Value>,
}

impl TenantConfigResolver {
    pub fn get_config(&self, tenant_context: &TenantContext) -> Option<&TenantConfig> {
        self.configs.get(tenant_context.tenant_id())
    }
    
    pub fn validate_operation(
        &self,
        tenant_context: &TenantContext,
        operation: &str,
        resource_data: &Value,
    ) -> ValidationResult<()> {
        let config = self.get_config(tenant_context)
            .ok_or_else(|| ValidationError::custom("Tenant configuration not found"))?;
        
        // Tenant-specific validation
        match operation {
            "CREATE" => {
                // Check user limits
                if resource_data.get("schemas").is_some() {
                    self.validate_schema_allowed(config, resource_data)?;
                }
            }
            _ => {}
        }
        
        Ok(())
    }
}
```

### Metrics and Monitoring

```rust
use std::sync::atomic::{AtomicU64, Ordering};
use std::collections::HashMap;

pub struct TenantMetrics {
    operations_by_tenant: HashMap<String, AtomicU64>,
    errors_by_tenant: HashMap<String, AtomicU64>,
}

impl TenantMetrics {
    pub fn record_operation(&self, tenant_id: &str, operation: &str) {
        let key = format!("{}:{}", tenant_id, operation);
        self.operations_by_tenant
            .entry(key)
            .or_insert_with(|| AtomicU64::new(0))
            .fetch_add(1, Ordering::Relaxed);
    }
    
    pub fn record_error(&self, tenant_id: &str, error_type: &str) {
        let key = format!("{}:{}", tenant_id, error_type);
        self.errors_by_tenant
            .entry(key)
            .or_insert_with(|| AtomicU64::new(0))
            .fetch_add(1, Ordering::Relaxed);
    }
    
    pub fn get_tenant_stats(&self, tenant_id: &str) -> TenantStats {
        // Aggregate metrics for the tenant
        TenantStats {
            total_operations: self.get_total_operations(tenant_id),
            error_rate: self.calculate_error_rate(tenant_id),
            last_activity: self.get_last_activity(tenant_id),
        }
    }
}
```

## Troubleshooting

### Common Issues

#### "Tenant not found"
**Symptom**: Requests return 404 or unauthorized errors
**Causes**:
- Tenant resolver not configured correctly
- Tenant mapping missing from resolver
- Headers/certificates not properly formatted

**Solutions**:
```rust
// Debug tenant resolution
async fn debug_tenant_resolution(resolver: &impl TenantResolver, request: &TenantResolutionRequest) {
    match resolver.resolve_tenant(request).await {
        Ok(context) => {
            println!("