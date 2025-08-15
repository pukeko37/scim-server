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

### Tenant Identification

Each tenant is identified by a unique tenant ID that's used throughout the system:

```rust
use scim_server::{ScimServer, TenantId, storage::InMemoryStorage};

// Create a server with multi-tenant support
let storage = InMemoryStorage::new();
let server = ScimServer::new(storage).await?;

// Operations are scoped to specific tenants
let tenant_id = TenantId::new("acme-corp");
let user = server.create_user(&tenant_id, user_data).await?;
```

### Tenant Isolation

Data isolation is enforced at multiple levels:

1. **API Level**: All operations require a tenant context
2. **Validation Level**: Schemas are tenant-specific
3. **Storage Level**: Data is partitioned by tenant
4. **Authentication Level**: Users belong to specific tenants

### Tenant Configuration

Each tenant can have its own configuration:

```rust
use scim_server::{TenantConfig, SchemaConfig};

let tenant_config = TenantConfig::builder()
    .tenant_id("acme-corp")
    .display_name("Acme Corporation")
    .max_users(1000)
    .custom_schema(custom_employee_schema)
    .features(vec!["bulk_operations", "filtering"])
    .build();

server.configure_tenant(tenant_config).await?;
```

## Implementation Patterns

### Basic Multi-Tenant Setup

The simplest multi-tenant setup uses tenant IDs in all operations:

```rust
use scim_server::{ScimServer, TenantId, storage::InMemoryStorage};
use serde_json::json;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let storage = InMemoryStorage::new();
    let server = ScimServer::new(storage).await?;
    
    // Create users for different tenants
    let acme_tenant = TenantId::new("acme-corp");
    let beta_tenant = TenantId::new("beta-inc");
    
    // Acme Corp user
    let acme_user = server.create_user(&acme_tenant, json!({
        "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
        "userName": "alice@acme.com",
        "displayName": "Alice Smith"
    })).await?;
    
    // Beta Inc user
    let beta_user = server.create_user(&beta_tenant, json!({
        "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
        "userName": "bob@beta.com",
        "displayName": "Bob Johnson"
    })).await?;
    
    // Users are completely isolated
    let acme_users = server.list_users(&acme_tenant).await?;
    let beta_users = server.list_users(&beta_tenant).await?;
    
    // acme_users contains only Alice, beta_users contains only Bob
    
    Ok(())
}
```

### HTTP Integration with Tenant Context

When integrating with HTTP frameworks, extract tenant ID from the request:

```rust
use axum::{extract::Path, Extension, Json};
use scim_server::{ScimServer, TenantId};

async fn create_user(
    Path(tenant_id): Path<String>,
    Extension(server): Extension<ScimServer>,
    Json(user_data): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, AppError> {
    let tenant = TenantId::new(tenant_id);
    let user = server.create_user(&tenant, user_data).await?;
    Ok(Json(user))
}

// Route: POST /tenants/{tenant_id}/scim/v2/Users
```

### Tenant-Specific Schemas

Different tenants can have different schema requirements:

```rust
use scim_server::{CustomSchema, AttributeType, TenantId};

// Healthcare tenant needs additional fields
let healthcare_schema = CustomSchema::builder()
    .id("urn:healthcare:schemas:Employee")
    .add_attribute("licenseNumber", AttributeType::String, false)
    .add_attribute("department", AttributeType::String, false)
    .add_attribute("certifications", AttributeType::MultiValue, false)
    .build();

let healthcare_tenant = TenantId::new("healthcare-corp");
server.register_schema(&healthcare_tenant, healthcare_schema).await?;

// Financial tenant needs different fields
let financial_schema = CustomSchema::builder()
    .id("urn:financial:schemas:Employee")
    .add_attribute("employeeId", AttributeType::String, true)
    .add_attribute("clearanceLevel", AttributeType::String, false)
    .add_attribute("tradingPermissions", AttributeType::MultiValue, false)
    .build();

let financial_tenant = TenantId::new("financial-corp");
server.register_schema(&financial_tenant, financial_schema).await?;
```

## Tenant Management

### Creating and Configuring Tenants

```rust
use scim_server::{TenantConfig, ResourceQuota, TenantFeatures};

async fn setup_new_tenant(
    server: &ScimServer,
    tenant_id: &str,
    org_name: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let tenant = TenantId::new(tenant_id);
    
    // Configure tenant with specific limits and features
    let config = TenantConfig::builder()
        .tenant_id(tenant_id)
        .display_name(org_name)
        .quota(ResourceQuota {
            max_users: 500,
            max_groups: 50,
            max_custom_resources: 100,
        })
        .features(TenantFeatures {
            bulk_operations: true,
            patch_operations: true,
            filtering: true,
            sorting: true,
            ai_integration: false, // Disabled for basic plan
        })
        .build();
    
    server.create_tenant(config).await?;
    
    // Set up default schemas
    server.register_default_schemas(&tenant).await?;
    
    Ok(())
}
```

### Tenant Discovery and Listing

```rust
// List all configured tenants
let tenants = server.list_tenants().await?;

for tenant in tenants {
    println!("Tenant: {} ({})", tenant.id, tenant.display_name);
    println!("  Users: {}/{}", tenant.user_count, tenant.max_users);
    println!("  Features: {:?}", tenant.enabled_features);
}

// Get specific tenant information
let tenant_info = server.get_tenant_info(&TenantId::new("acme-corp")).await?;
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

// Storage provider handles routing based on tenant
let storage = MultiTenantDatabaseStorage::new()
    .add_tenant("acme-corp", acme_config)
    .add_tenant("beta-inc", beta_config);
```

## Security and Compliance

### Authentication Integration

Combine multi-tenancy with authentication for complete security:

```rust
use scim_server::auth::{AuthenticationWitness, TenantAuthority};

async fn authenticated_operation(
    auth: AuthenticationWitness<Authenticated>,
    tenant_id: TenantId,
    server: &ScimServer,
) -> Result<(), AuthError> {
    // Verify user has access to this tenant
    let tenant_authority = auth.verify_tenant_access(&tenant_id)?;
    
    // Operation is now both authenticated and tenant-scoped
    let users = server.list_users_with_auth(
        &tenant_id,
        &auth,
        &tenant_authority
    ).await?;
    
    Ok(())
}
```

### Audit Logging

Multi-tenant systems require comprehensive audit trails:

```rust
use scim_server::audit::{AuditEvent, AuditLogger};

// All operations are logged with tenant context
let audit_logger = AuditLogger::new();

// Log tenant-scoped operations
audit_logger.log(AuditEvent::UserCreated {
    tenant_id: "acme-corp",
    user_id: "user123",
    performed_by: "admin@acme.com",
    timestamp: Utc::now(),
}).await?;
```

### Data Residency and Compliance

Different tenants may have different compliance requirements:

```rust
use scim_server::{DataResidency, ComplianceMode};

let eu_tenant_config = TenantConfig::builder()
    .tenant_id("eu-customer")
    .data_residency(DataResidency::EuropeanUnion)
    .compliance_mode(ComplianceMode::GDPR)
    .encryption_required(true)
    .audit_retention_days(2555) // 7 years
    .build();

let us_tenant_config = TenantConfig::builder()
    .tenant_id("us-customer")
    .data_residency(DataResidency::UnitedStates)
    .compliance_mode(ComplianceMode::SOX)
    .encryption_required(true)
    .audit_retention_days(2555)
    .build();
```

## Performance and Scaling

### Resource Isolation

Prevent one tenant from affecting others:

```rust
use scim_server::{ResourceLimits, RateLimiting};

let tenant_limits = ResourceLimits::builder()
    .max_requests_per_minute(1000)
    .max_concurrent_operations(50)
    .max_query_complexity(100)
    .memory_limit_mb(512)
    .build();

server.set_tenant_limits(&tenant_id, tenant_limits).await?;
```

### Monitoring per Tenant

Track performance and usage metrics per tenant:

```rust
use scim_server::metrics::{TenantMetrics, MetricsCollector};

let metrics = server.get_tenant_metrics(&tenant_id).await?;

println!("Tenant {} metrics:", tenant_id);
println!("  Requests/minute: {}", metrics.requests_per_minute);
println!("  Average response time: {}ms", metrics.avg_response_time);
println!("  Storage usage: {}MB", metrics.storage_usage_mb);
println!("  Error rate: {}%", metrics.error_rate);
```

## Best Practices

### Tenant ID Strategy

Choose tenant IDs carefully:

```rust
// Good: Clear, unique, URL-safe
let tenant_id = TenantId::new("acme-corp-prod");

// Avoid: Ambiguous or containing sensitive data
let bad_tenant_id = TenantId::new("customer-123-secret-key");
```

### Configuration Management

Use environment-specific configurations:

```rust
use scim_server::config::TenantConfigLoader;

// Load tenant configurations from external sources
let config_loader = TenantConfigLoader::new()
    .from_file("tenants.yaml")
    .from_env_prefix("SCIM_TENANT_")
    .from_database(&config_db);

let tenant_configs = config_loader.load_all().await?;
```

### Error Handling

Provide tenant-aware error messages:

```rust
use scim_server::error::{ScimError, TenantError};

match server.get_user(&tenant_id, &user_id).await {
    Ok(user) => Ok(user),
    Err(ScimError::TenantNotFound(tenant)) => {
        Err(format!("Organization '{}' not found", tenant))
    },
    Err(ScimError::UserNotFound { tenant, user_id }) => {
        Err(format!("User '{}' not found in organization '{}'", user_id, tenant))
    },
    Err(e) => Err(e.into()),
}
```

### Testing Multi-Tenant Code

Test with multiple tenants to ensure isolation:

```rust
#[tokio::test]
async fn test_tenant_isolation() {
    let server = test_server().await;
    
    let tenant_a = TenantId::new("tenant-a");
    let tenant_b = TenantId::new("tenant-b");
    
    // Create user in tenant A
    let user_a = server.create_user(&tenant_a, user_data.clone()).await?;
    
    // Verify user is not visible in tenant B
    let tenant_b_users = server.list_users(&tenant_b).await?;
    assert!(tenant_b_users.is_empty());
    
    // Verify user is visible in tenant A
    let tenant_a_users = server.list_users(&tenant_a).await?;
    assert_eq!(tenant_a_users.len(), 1);
}
```

Multi-tenancy in SCIM Server provides a robust foundation for building SaaS applications that serve multiple organizations while maintaining security, performance, and compliance requirements.