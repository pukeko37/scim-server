# Multi-Tenancy

Multi-tenancy is a core architectural pattern that allows a single SCIM Server instance to serve multiple organizations while keeping their data completely isolated. This chapter explains how SCIM Server implements multi-tenancy and how to use it effectively.

## What is Multi-Tenancy?

Multi-tenancy is a software architecture where a single instance of an application serves multiple customers (tenants). Each tenant's data is isolated and invisible to other tenants, creating the appearance of having their own dedicated instance.

### Benefits of Multi-Tenancy

- **Cost Efficiency**: Shared infrastructure reduces operational costs
- **Simplified Management**: Single deployment to maintain and update
- **Resource Optimization**: Better utilization of hardware and services
- **Faster Scaling**: Add new tenants without new infrastructure
- **Centralized Security**: Consistent security policies across all tenants

### SCIM Server's Multi-Tenant Approach

SCIM Server implements multi-tenancy at the application layer, providing:

- **Complete Data Isolation**: No tenant can access another's data
- **Tenant-Specific Configuration**: Schemas and settings per tenant
- **Performance Isolation**: Resource quotas and monitoring per tenant
- **Independent Scaling**: Different tenants can have different performance profiles

## Core Concepts

### Tenant Context

Each tenant is identified through a `TenantContext` that's included in request operations:

```rust
use scim_server::{
    ScimServer,
    storage::InMemoryStorage,
    resource::{RequestContext, TenantContext},
    providers::StandardResourceProvider,
};
use serde_json::json;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create server with multi-tenant support
    let storage = InMemoryStorage::new();
    let provider = StandardResourceProvider::new(storage);
    let server = ScimServer::new(provider)?;
    
    // Create tenant-specific context
    let tenant_context = TenantContext::new(
        "acme-corp".to_string(),
        "client-123".to_string(),
    );
    
    let request_context = RequestContext::with_tenant_generated_id(tenant_context);
    
    // Operations are scoped to specific tenants through the request context
    let user_data = json!({
        "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
        "userName": "alice@acme.com",
        "name": {
            "formatted": "Alice Smith",
            "familyName": "Smith",
            "givenName": "Alice"
        },
        "displayName": "Alice Smith"
    });
    
    let user = server.create_resource("User", user_data, &request_context).await?;
    
    Ok(())
}
```

### Tenant Isolation

Data isolation is enforced at multiple levels:

1. **Request Level**: All operations require a request context with tenant information
2. **Validation Level**: Schemas are applied consistently within tenant boundaries
3. **Storage Level**: Data is partitioned by tenant context
4. **Provider Level**: Resource providers handle tenant-specific data routing

### Tenant Configuration

Each tenant can have its own configuration through `TenantContext`:

```rust
use scim_server::resource::{TenantContext, IsolationLevel, TenantPermissions};

let tenant_permissions = TenantPermissions {
    can_create: true,
    can_read: true,
    can_update: true,
    can_delete: false, // Restrict delete operations
    can_list: true,
    max_users: Some(1000),
    max_groups: Some(50),
};

let tenant_context = TenantContext::new("acme-corp".to_string(), "client-123".to_string())
    .with_isolation_level(IsolationLevel::Strict)
    .with_permissions(tenant_permissions);

let request_context = RequestContext::with_tenant_generated_id(tenant_context);
```

## Implementation Patterns

### Basic Multi-Tenant Setup

The simplest multi-tenant setup uses tenant contexts in all operations:

```rust
use scim_server::{
    ScimServer,
    storage::InMemoryStorage,
    resource::{RequestContext, TenantContext, ResourceProvider},
    providers::StandardResourceProvider,
    resource_handlers::{create_user_resource_handler, create_group_resource_handler},
    schema::SchemaRegistry,
    resource::ScimOperation,
};
use serde_json::json;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let storage = InMemoryStorage::new();
    let provider = StandardResourceProvider::new(storage);
    let mut server = ScimServer::new(provider)?;
    
    // Register resource types
    let schema_registry = SchemaRegistry::new()?;
    let user_schema = schema_registry.get_user_schema();
    let user_handler = create_user_resource_handler(user_schema);
    server.register_resource_type(
        "User",
        user_handler,
        vec![ScimOperation::Create, ScimOperation::Read, ScimOperation::List],
    )?;
    
    // Create contexts for different tenants
    let acme_context = RequestContext::with_tenant_generated_id(
        TenantContext::new("acme-corp".to_string(), "client-123".to_string())
    );
    
    let beta_context = RequestContext::with_tenant_generated_id(
        TenantContext::new("beta-inc".to_string(), "client-456".to_string())
    );
    
    // Acme Corp user
    let acme_user_data = json!({
        "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
        "userName": "alice@acme.com",
        "name": {
            "formatted": "Alice Smith",
            "familyName": "Smith",
            "givenName": "Alice"
        },
        "displayName": "Alice Smith"
    });
    
    let acme_user = server.create_resource("User", acme_user_data, &acme_context).await?;
    
    // Beta Inc user
    let beta_user_data = json!({
        "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
        "userName": "bob@beta.com",
        "name": {
            "formatted": "Bob Johnson",
            "familyName": "Johnson",
            "givenName": "Bob"
        },
        "displayName": "Bob Johnson"
    });
    
    let beta_user = server.create_resource("User", beta_user_data, &beta_context).await?;
    
    // Users are completely isolated
    let acme_users = server.list_resources("User", &acme_context).await?;
    let beta_users = server.list_resources("User", &beta_context).await?;
    
    println!("Acme Corp has {} users", acme_users.len()); // 1
    println!("Beta Inc has {} users", beta_users.len());   // 1
    
    // acme_users contains only Alice, beta_users contains only Bob
    
    Ok(())
}
```

### HTTP Integration with Tenant Context

When integrating with HTTP frameworks, extract tenant ID from the request:

```rust
use axum::{extract::Path, Extension, Json};
use scim_server::{ScimServer, resource::{RequestContext, TenantContext}};

async fn create_user(
    Path(tenant_id): Path<String>,
    Extension(server): Extension<ScimServer<impl ResourceProvider>>,
    Json(user_data): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, AppError> {
    let tenant_context = TenantContext::new(tenant_id, "web-client".to_string());
    let request_context = RequestContext::with_tenant_generated_id(tenant_context);
    
    let user = server.create_resource("User", user_data, &request_context).await?;
    Ok(Json(user.to_json()?))
}

// Route: POST /tenants/{tenant_id}/scim/v2/Users
```

### Permission-Based Access Control

Use tenant permissions to control operations:

```rust
use scim_server::resource::{TenantContext, TenantPermissions};

// Create read-only tenant
let readonly_permissions = TenantPermissions {
    can_create: false,
    can_read: true,
    can_update: false,
    can_delete: false,
    can_list: true,
    max_users: Some(100),
    max_groups: None,
};

let readonly_context = RequestContext::with_tenant_generated_id(
    TenantContext::new("readonly-tenant".to_string(), "client-789".to_string())
        .with_permissions(readonly_permissions)
);

// Validate operations before executing
if !readonly_context.can_perform_operation("create") {
    return Err("Create operation not permitted for this tenant".into());
}

// This will be rejected by the context validation
let result = server.create_resource("User", user_data, &readonly_context).await;
```

## Storage Considerations

### Database Multi-Tenancy

When using database storage, tenant isolation can be implemented in several ways:

#### Shared Database, Separate Schemas
```sql
-- Each tenant gets its own schema
CREATE SCHEMA tenant_acme_corp;
CREATE SCHEMA tenant_beta_inc;

-- Tables are created in tenant-specific schemas
CREATE TABLE tenant_acme_corp.users (...);
CREATE TABLE tenant_beta_inc.users (...);
```

#### Shared Database, Shared Tables with Tenant Column
```sql
-- Single table with tenant_id column
CREATE TABLE users (
    tenant_id VARCHAR(255) NOT NULL,
    user_id VARCHAR(255) NOT NULL,
    username VARCHAR(255) NOT NULL,
    -- ... other columns
    PRIMARY KEY (tenant_id, user_id)
);

-- All queries include tenant_id in WHERE clause
SELECT * FROM users WHERE tenant_id = 'acme-corp';
```

#### Separate Databases per Tenant
```rust
use scim_server::storage::DatabaseConfig;

// Configure separate databases per tenant
let acme_config = DatabaseConfig::new("postgresql://host/acme_corp_db");
let beta_config = DatabaseConfig::new("postgresql://host/beta_inc_db");

// Storage provider handles routing based on tenant context
let storage = MultiTenantDatabaseStorage::new()
    .add_tenant("acme-corp", acme_config)
    .add_tenant("beta-inc", beta_config);
```

## Security and Compliance

### Authentication Integration

Combine multi-tenancy with authentication for complete security:

```rust
async fn authenticated_operation(
    tenant_id: String,
    auth_token: String,
    server: &ScimServer<impl ResourceProvider>,
) -> Result<Vec<Resource>, Box<dyn std::error::Error>> {
    // Verify authentication and extract user info
    let authenticated_user = verify_auth_token(&auth_token)?;
    
    // Verify user has access to this tenant
    if !authenticated_user.has_tenant_access(&tenant_id) {
        return Err("Access denied for tenant".into());
    }
    
    // Create tenant context with verified access
    let tenant_context = TenantContext::new(tenant_id, authenticated_user.client_id);
    let request_context = RequestContext::with_tenant_generated_id(tenant_context);
    
    // Operation is now both authenticated and tenant-scoped
    let users = server.list_resources("User", &request_context).await?;
    
    Ok(users)
}
```

### Audit Logging

Multi-tenant systems require comprehensive audit trails:

```rust
use log::info;

// All operations are logged with tenant context
async fn audited_create_user(
    server: &ScimServer<impl ResourceProvider>,
    user_data: serde_json::Value,
    context: &RequestContext,
) -> Result<Resource, Box<dyn std::error::Error>> {
    info!("User creation attempt - Tenant: {:?}, Request: {}", 
          context.tenant_id(), context.request_id);
    
    let result = server.create_resource("User", user_data, context).await;
    
    match &result {
        Ok(user) => {
            info!("User created successfully - Tenant: {:?}, User ID: {:?}, Request: {}", 
                  context.tenant_id(), user.get_id(), context.request_id);
        }
        Err(e) => {
            info!("User creation failed - Tenant: {:?}, Error: {}, Request: {}", 
                  context.tenant_id(), e, context.request_id);
        }
    }
    
    result
}
```

## Performance and Scaling

### Resource Isolation

Prevent one tenant from affecting others through request context validation:

```rust
use scim_server::resource::{TenantContext, TenantPermissions};

// Set up tenant limits
let limited_permissions = TenantPermissions {
    can_create: true,
    can_read: true,
    can_update: true,
    can_delete: true,
    can_list: true,
    max_users: Some(500),
    max_groups: Some(25),
};

let tenant_context = TenantContext::new("limited-tenant".to_string(), "client-999".to_string())
    .with_permissions(limited_permissions);

// Check limits before operations
async fn create_user_with_limits(
    server: &ScimServer<impl ResourceProvider>,
    user_data: serde_json::Value,
    context: &RequestContext,
) -> Result<Resource, Box<dyn std::error::Error>> {
    // Check current user count
    let current_users = server.list_resources("User", context).await?;
    
    if let Some(tenant_context) = &context.tenant_context {
        if !tenant_context.check_user_limit(current_users.len()) {
            return Err("User limit exceeded for tenant".into());
        }
    }
    
    server.create_resource("User", user_data, context).await
        .map_err(|e| e.into())
}
```

### Monitoring per Tenant

Track performance and usage metrics per tenant:

```rust
use std::collections::HashMap;

#[derive(Debug)]
struct TenantMetrics {
    requests_count: u64,
    users_count: usize,
    groups_count: usize,
    storage_usage_bytes: u64,
}

async fn collect_tenant_metrics(
    server: &ScimServer<impl ResourceProvider>,
    tenant_id: &str,
) -> Result<TenantMetrics, Box<dyn std::error::Error>> {
    let tenant_context = TenantContext::new(tenant_id.to_string(), "metrics-client".to_string());
    let request_context = RequestContext::with_tenant_generated_id(tenant_context);
    
    let users = server.list_resources("User", &request_context).await?;
    let groups = server.list_resources("Group", &request_context).await?;
    
    Ok(TenantMetrics {
        requests_count: 0, // Would be tracked separately
        users_count: users.len(),
        groups_count: groups.len(),
        storage_usage_bytes: 0, // Would be calculated from resource sizes
    })
}
```

## Best Practices

### Tenant ID Strategy

Choose tenant IDs carefully:

```rust
// Good: Clear, unique, URL-safe
let tenant_context = TenantContext::new("acme-corp-prod".to_string(), "web-app".to_string());

// Avoid: Ambiguous or containing sensitive data
// let bad_tenant_context = TenantContext::new("customer-123-secret-key".to_string(), "app".to_string());
```

### Error Handling

Provide tenant-aware error messages:

```rust
use scim_server::error::ScimError;

async fn safe_tenant_operation(
    server: &ScimServer<impl ResourceProvider>,
    resource_type: &str,
    resource_id: &str,
    context: &RequestContext,
) -> Result<Option<Resource>, String> {
    match server.get_resource(resource_type, resource_id, context).await {
        Ok(resource) => Ok(resource),
        Err(ScimError::ResourceNotFound { resource_type, id }) => {
            if let Some(tenant_id) = context.tenant_id() {
                Err(format!("{} '{}' not found in tenant '{}'", resource_type, id, tenant_id))
            } else {
                Err(format!("{} '{}' not found", resource_type, id))
            }
        },
        Err(e) => Err(format!("Operation failed: {}", e)),
    }
}
```

### Testing Multi-Tenant Code

Test with multiple tenants to ensure isolation:

```rust
#[tokio::test]
async fn test_tenant_isolation() -> Result<(), Box<dyn std::error::Error>> {
    let storage = InMemoryStorage::new();
    let provider = StandardResourceProvider::new(storage);
    let server = ScimServer::new(provider)?;
    
    let tenant_a_context = RequestContext::with_tenant_generated_id(
        TenantContext::new("tenant-a".to_string(), "client-1".to_string())
    );
    let tenant_b_context = RequestContext::with_tenant_generated_id(
        TenantContext::new("tenant-b".to_string(), "client-2".to_string())
    );
    
    let user_data = serde_json::json!({
        "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
        "userName": "test@example.com",
        "displayName": "Test User"
    });
    
    // Create user in tenant A
    let _user_a = server.create_resource("User", user_data.clone(), &tenant_a_context).await?;
    
    // Verify user is not visible in tenant B
    let tenant_b_users = server.list_resources("User", &tenant_b_context).await?;
    assert!(tenant_b_users.is_empty());
    
    // Verify user is visible in tenant A
    let tenant_a_users = server.list_resources("User", &tenant_a_context).await?;
    assert_eq!(tenant_a_users.len(), 1);
    
    Ok(())
}
```

Multi-tenancy in SCIM Server provides a robust foundation for building SaaS applications that serve multiple organizations while maintaining security, performance, and compliance requirements through proper use of tenant contexts and request isolation.