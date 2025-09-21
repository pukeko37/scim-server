# Request Lifecycle & Context Management

This deep dive explores how requests flow through the SCIM server architecture from HTTP entry point to storage operations, with particular focus on context propagation, tenant resolution, and the integration points between components.

## Overview

Understanding the request lifecycle is fundamental to working with SCIM Server, as it shows how all the individual components work together to process SCIM operations. This end-to-end view helps you make informed decisions about where to implement custom logic, how to handle errors, and where performance optimizations matter most.

## Complete Request Flow

```text
HTTP Request
    ↓
┌─────────────────────────────────────────────────────────────────────────────┐
│ 1. HTTP Integration Layer (Your Web Framework)                             │
│    • Extract SCIM request details (method, path, headers, body)            │
│    • Handle authentication (API keys, OAuth, etc.)                         │
│    • Create initial request context                                        │
└─────────────────────────────────────────────────────────────────────────────┘
    ↓ ScimOperationRequest
┌─────────────────────────────────────────────────────────────────────────────┐
│ 2. Operation Handler Layer                                                  │
│    • ScimOperationHandler processes structured SCIM request               │
│    • Validates SCIM protocol compliance                                    │
│    • Extracts operation metadata                                           │
└─────────────────────────────────────────────────────────────────────────────┘
    ↓ Validated Operation + RequestContext
┌─────────────────────────────────────────────────────────────────────────────┐
│ 3. Tenant Resolution & Context Enhancement                                 │
│    • TenantResolver maps credentials → TenantContext                       │
│    • RequestContext enhanced with tenant information                       │
│    • Permissions and isolation levels applied                              │
└─────────────────────────────────────────────────────────────────────────────┘
    ↓ Tenant-Aware RequestContext
┌─────────────────────────────────────────────────────────────────────────────┐
│ 4. SCIM Server Orchestration                                              │
│    • Route operation to appropriate method                                  │
│    • Apply schema validation                                               │
│    • Handle concurrency control (ETag processing)                          │
└─────────────────────────────────────────────────────────────────────────────┘
    ↓ Validated Operation + Enhanced Context
┌─────────────────────────────────────────────────────────────────────────────┐
│ 5. Resource Provider Business Logic                                        │
│    • Apply business rules and transformations                              │
│    • Handle resource-specific validation                                   │
│    • Coordinate with storage operations                                    │
└─────────────────────────────────────────────────────────────────────────────┘
    ↓ Storage Operations + Context
┌─────────────────────────────────────────────────────────────────────────────┐
│ 6. Storage Provider Data Persistence                                       │
│    • Apply tenant-scoped storage keys                                      │
│    • Perform actual data operations (CRUD)                                 │
│    • Handle storage-level errors and retries                               │
└─────────────────────────────────────────────────────────────────────────────┘
    ↓ Storage Results
┌─────────────────────────────────────────────────────────────────────────────┐
│ 7. Response Assembly & Error Handling                                      │
│    • Convert storage results to SCIM resources                             │
│    • Apply response transformations                                        │
│    • Handle errors with appropriate SCIM error responses                   │
└─────────────────────────────────────────────────────────────────────────────┘
    ↓ ScimOperationResponse
HTTP Response (SCIM-compliant JSON)
```

## Context Propagation Architecture

The `RequestContext` is the backbone of the request lifecycle, carrying essential information through every layer of the system.

### RequestContext Structure

```rust
pub struct RequestContext {
    pub request_id: String,           // Unique identifier for tracing
    tenant_context: Option<TenantContext>, // Multi-tenant information
}

impl RequestContext {
    // Single-tenant constructor
    pub fn new(request_id: String) -> Self
    
    // Multi-tenant constructor  
    pub fn with_tenant_generated_id(tenant_context: TenantContext) -> Self
    
    // Context queries
    pub fn tenant_id(&self) -> Option<&str>
    pub fn is_multi_tenant(&self) -> bool
    pub fn can_perform_operation(&self, operation: &str) -> bool
}
```

### Context Creation Patterns

#### Single-Tenant Context
```rust
// Simple single-tenant setup
let context = RequestContext::new("req-12345".to_string());
```

#### Multi-Tenant Context with Tenant Resolution
```rust
// Resolve tenant from authentication
let tenant_context = tenant_resolver
    .resolve_tenant(api_key)
    .await?;

let context = RequestContext::with_tenant_generated_id(tenant_context);
```

#### Context with Custom Request ID
```rust
// Use correlation ID from HTTP headers
let request_id = extract_correlation_id(&headers)
    .unwrap_or_else(|| generate_request_id());
    
let mut context = RequestContext::new(request_id);
if let Some(tenant) = resolved_tenant {
    context = RequestContext::with_tenant(request_id, tenant);
}
```

## Integration Layer Patterns

### Web Framework Integration

The HTTP integration layer is where you connect SCIM Server to your web framework. Here's how different frameworks typically integrate:

#### Axum Integration Pattern
```rust
use axum::{extract::Path, http::HeaderMap, Json};
use scim_server::{ScimServer, RequestContext, ScimOperationHandler};

async fn scim_operation_handler(
    Path((tenant_id, resource_type, resource_id)): Path<(String, String, Option<String>)>,
    headers: HeaderMap,
    Json(body): Json<Value>,
    Extension(server): Extension<Arc<ScimServer<MyProvider>>>,
    Extension(tenant_resolver): Extension<Arc<dyn TenantResolver>>,
) -> Result<Json<Value>, ScimError> {
    // 1. Create operation request from HTTP details
    let operation_request = ScimOperationRequest::from_http(
        method, &path, headers, body
    )?;
    
    // 2. Resolve tenant context
    let api_key = extract_api_key(&headers)?;
    let tenant_context = tenant_resolver.resolve_tenant(&api_key).await?;
    let request_context = RequestContext::with_tenant_generated_id(tenant_context);
    
    // 3. Process through operation handler
    let operation_handler = ScimOperationHandler::new(&server);
    let response = operation_handler
        .handle_operation(operation_request, &request_context)
        .await?;
    
    // 4. Convert to HTTP response
    Ok(Json(response.into_json()))
}
```

#### Actix-Web Integration Pattern
```rust
use actix_web::{web, HttpRequest, HttpResponse, Result};

async fn scim_handler(
    req: HttpRequest,
    path: web::Path<(String, String)>,
    body: web::Json<Value>,
    server: web::Data<ScimServer<MyProvider>>,
) -> Result<HttpResponse> {
    // Similar pattern but with Actix-specific extractors
    let (tenant_id, resource_type) = path.into_inner();
    
    // Extract and create context
    let context = create_request_context(&req, &tenant_id).await?;
    
    // Process operation
    let response = process_scim_operation(
        &server, 
        &req.method(), 
        &req.uri().path(),
        body.into_inner(),
        &context
    ).await?;
    
    Ok(HttpResponse::Ok().json(response))
}
```

### Operation Handler Integration

The `ScimOperationHandler` provides a framework-agnostic way to process SCIM operations:

```rust
use scim_server::{ScimOperationHandler, ScimOperationRequest, ScimOperationResponse};

// Create operation handler
let handler = ScimOperationHandler::new(&scim_server);

// Process different operation types
match operation_request.operation_type() {
    ScimOperation::Create => {
        let response = handler.handle_create(
            &operation_request.resource_type,
            operation_request.resource_data,
            &request_context
        ).await?;
    },
    ScimOperation::GetById => {
        let response = handler.handle_get(
            &operation_request.resource_type,
            &operation_request.resource_id.unwrap(),
            &request_context
        ).await?;
    },
    // ... other operations
}
```

## Tenant Resolution Integration Points

Multi-tenant applications need to resolve tenant context early in the request lifecycle:

### Database-Backed Tenant Resolution
```rust
use scim_server::multi_tenant::TenantResolver;

pub struct DatabaseTenantResolver {
    db_pool: PgPool,
    cache: Arc<RwLock<HashMap<String, TenantContext>>>,
}

impl TenantResolver for DatabaseTenantResolver {
    type Error = DatabaseError;
    
    async fn resolve_tenant(&self, credential: &str) -> Result<TenantContext, Self::Error> {
        // 1. Check cache first
        if let Some(tenant) = self.cache.read().unwrap().get(credential) {
            return Ok(tenant.clone());
        }
        
        // 2. Query database
        let tenant_record = sqlx::query_as!(
            TenantRecord,
            "SELECT tenant_id, client_id, permissions FROM tenants WHERE api_key = $1",
            credential
        )
        .fetch_optional(&self.db_pool)
        .await?
        .ok_or(DatabaseError::TenantNotFound)?;
        
        // 3. Build tenant context
        let tenant_context = TenantContext::new(
            tenant_record.tenant_id,
            tenant_record.client_id
        ).with_permissions(tenant_record.permissions);
        
        // 4. Cache for future requests
        self.cache.write().unwrap()
            .insert(credential.to_string(), tenant_context.clone());
            
        Ok(tenant_context)
    }
}
```

### JWT-Based Tenant Resolution
```rust
pub struct JwtTenantResolver {
    jwt_secret: String,
}

impl TenantResolver for JwtTenantResolver {
    type Error = JwtError;
    
    async fn resolve_tenant(&self, token: &str) -> Result<TenantContext, Self::Error> {
        // 1. Validate and decode JWT
        let claims: TenantClaims = jsonwebtoken::decode(
            token,
            &DecodingKey::from_secret(self.jwt_secret.as_ref()),
            &Validation::default()
        )?.claims;
        
        // 2. Extract tenant information from claims
        let permissions = TenantPermissions {
            can_create: claims.permissions.contains("create"),
            can_update: claims.permissions.contains("update"),
            max_users: claims.limits.max_users,
            max_groups: claims.limits.max_groups,
            ..Default::default()
        };
        
        // 3. Build tenant context
        Ok(TenantContext::new(claims.tenant_id, claims.client_id)
            .with_permissions(permissions)
            .with_isolation_level(claims.isolation_level))
    }
}
```

## Error Handling Patterns

Errors can occur at any stage of the request lifecycle. The SCIM Server provides structured error handling:

### Error Propagation Flow
```text
Storage Error → Resource Provider Error → SCIM Error → HTTP Error Response

Examples:
• Database connection failure → StorageError → ScimError::InternalError → 500
• Tenant not found → ResolverError → ScimError::Unauthorized → 401  
• Resource not found → ResourceError → ScimError::NotFound → 404
• Validation failure → ValidationError → ScimError::BadRequest → 400
• Version conflict → ConcurrencyError → ScimError::PreconditionFailed → 412
```

### Custom Error Handling
```rust
// Custom error mapper for your web framework
impl From<ScimError> for HttpResponse {
    fn from(error: ScimError) -> Self {
        let (status_code, scim_error_response) = match error {
            ScimError::NotFound(msg) => (
                StatusCode::NOT_FOUND,
                ScimErrorResponse::not_found(&msg)
            ),
            ScimError::TenantNotFound(tenant_id) => (
                StatusCode::UNAUTHORIZED,
                ScimErrorResponse::unauthorized(&format!("Invalid tenant: {}", tenant_id))
            ),
            ScimError::ValidationError(details) => (
                StatusCode::BAD_REQUEST,
                ScimErrorResponse::bad_request_with_details(details)
            ),
            _ => (
                StatusCode::INTERNAL_SERVER_ERROR,
                ScimErrorResponse::internal_error()
            ),
        };
        
        HttpResponse::build(status_code).json(scim_error_response)
    }
}
```

## Performance Considerations

### Context Creation Optimization
```rust
// Avoid unnecessary tenant resolution for public endpoints
pub async fn optimized_context_creation(
    api_key: Option<&str>,
    tenant_resolver: &Arc<dyn TenantResolver>,
    request_id: String,
) -> Result<RequestContext, ScimError> {
    match api_key {
        Some(key) => {
            // Only resolve tenant when needed
            let tenant_context = tenant_resolver.resolve_tenant(key).await?;
            Ok(RequestContext::with_tenant(request_id, tenant_context))
        },
        None => {
            // Single-tenant or public operation
            Ok(RequestContext::new(request_id))
        }
    }
}
```

### Async Best Practices
```rust
// Concurrent operations where possible
pub async fn batch_operation_handler(
    operations: Vec<ScimOperationRequest>,
    context: &RequestContext,
    server: &ScimServer<impl ResourceProvider>,
) -> Vec<Result<ScimOperationResponse, ScimError>> {
    // Process operations concurrently
    let futures = operations.into_iter().map(|op| {
        let handler = ScimOperationHandler::new(server);
        handler.handle_operation(op, context)
    });
    
    futures::future::join_all(futures).await
}
```

## Debugging and Observability

### Request Tracing
```rust
use tracing::{info, error, span, Level};

pub async fn traced_operation_handler(
    operation: ScimOperationRequest,
    context: &RequestContext,
    server: &ScimServer<impl ResourceProvider>,
) -> Result<ScimOperationResponse, ScimError> {
    let span = span!(
        Level::INFO, 
        "scim_operation",
        request_id = %context.request_id,
        tenant_id = %context.tenant_id().unwrap_or("single-tenant"),
        operation_type = %operation.operation_type(),
        resource_type = %operation.resource_type
    );
    
    async move {
        info!("Processing SCIM operation");
        
        let result = ScimOperationHandler::new(server)
            .handle_operation(operation, context)
            .await;
            
        match &result {
            Ok(response) => info!(
                status = "success", 
                resource_id = ?response.resource_id()
            ),
            Err(error) => error!(
                status = "error",
                error = %error,
                error_type = ?std::mem::discriminant(error)
            ),
        }
        
        result
    }.instrument(span).await
}
```

## Integration Testing Patterns

### End-to-End Request Testing
```rust
#[tokio::test]
async fn test_complete_request_lifecycle() {
    // Setup
    let storage = InMemoryStorage::new();
    let provider = StandardResourceProvider::new(storage);
    let server = ScimServer::new(provider).unwrap();
    let tenant_resolver = StaticTenantResolver::new();
    
    // Add test tenant
    let tenant_context = TenantContext::new("test-tenant".into(), "test-client".into());
    tenant_resolver.add_tenant("test-api-key", tenant_context).await;
    
    // Simulate HTTP request
    let operation_request = ScimOperationRequest::create(
        "User",
        json!({
            "userName": "test.user",
            "displayName": "Test User"
        })
    );
    
    // Resolve tenant (simulating middleware)
    let resolved_tenant = tenant_resolver.resolve_tenant("test-api-key").await.unwrap();
    let context = RequestContext::with_tenant_generated_id(resolved_tenant);
    
    // Process operation
    let handler = ScimOperationHandler::new(&server);
    let response = handler.handle_operation(operation_request, &context).await.unwrap();
    
    // Verify response
    assert!(response.is_success());
    assert!(response.resource_id().is_some());
}
```

## Related Topics

- **[Multi-Tenant Architecture Patterns](./multi-tenant-patterns.md)** - Deep dive into tenant isolation strategies
- **[Resource Provider Architecture](./resource-provider-architecture.md)** - Business logic layer implementation patterns
- **[Operation Handlers](../concepts/operation-handlers.md)** - Framework-agnostic request processing
- **[Multi-Tenant Architecture](../concepts/multi-tenant-architecture.md)** - Core multi-tenancy concepts

## Next Steps

Now that you understand how requests flow through the system:

1. **Implement your HTTP integration layer** using the patterns shown above
2. **Set up tenant resolution** if building a multi-tenant system
3. **Add proper error handling** and observability for production use
4. **Consider Resource Provider architecture** for your business logic needs