# Multi-Tenant Architecture

The Multi-Tenant Architecture in SCIM Server provides complete isolation and management of identity resources across multiple customer organizations within a single deployment. It enables Software-as-a-Service (SaaS) providers to serve multiple tenants while ensuring strict data isolation, flexible URL generation, and tenant-specific configuration management.

See the [Multi-Tenant API documentation](https://docs.rs/scim-server/latest/scim_server/multi_tenant/index.html) for complete details.

## Value Proposition

The Multi-Tenant Architecture delivers comprehensive multi-tenancy capabilities:

- **Complete Tenant Isolation**: Strict separation of data, operations, and configurations between tenants
- **Flexible URL Strategies**: Multiple deployment patterns for tenant-specific endpoints
- **Scalable Authentication**: Credential-based tenant resolution with pluggable authentication
- **Resource Limits & Permissions**: Granular control over tenant capabilities and quotas
- **Zero Configuration Overhead**: Automatic tenant handling with sensible defaults
- **SCIM Compliance**: Full SCIM 2.0 compliance maintained across all tenant scenarios
- **Production Ready**: Comprehensive security, audit trails, and operational monitoring

## Architecture Overview

The Multi-Tenant Architecture operates as a cross-cutting concern throughout the SCIM Server stack:

```text
Multi-Tenant Architecture (Cross-Cutting Layer)
├── Tenant Resolution & Authentication
├── Request Context & Isolation
├── URL Generation & Routing
├── Resource Scoping & Limits
└── Configuration Management
    ↓
Applied to All Layers:
├── SCIM Server (Tenant-Aware Operations)
├── Resource Providers (Tenant Isolation)
├── Storage Providers (Tenant Scoping)
└── Schema Management (Tenant Extensions)
```

### Core Components

1. **[`TenantContext`](https://docs.rs/scim-server/latest/scim_server/struct.TenantContext.html)**: Complete tenant identity and permissions
2. **[`RequestContext`](https://docs.rs/scim-server/latest/scim_server/struct.RequestContext.html)**: Tenant-aware request handling with automatic scoping
3. **[`TenantResolver`](https://docs.rs/scim-server/latest/scim_server/multi_tenant/trait.TenantResolver.html)**: Authentication credential to tenant mapping
4. **[`TenantStrategy`](https://docs.rs/scim-server/latest/scim_server/enum.TenantStrategy.html)**: Flexible URL generation patterns
5. **[Multi-Tenant Provider](https://docs.rs/scim-server/latest/scim_server/multi_tenant/index.html)**: Storage-level tenant isolation helpers
6. **[`ScimTenantConfiguration`](https://docs.rs/scim-server/latest/scim_server/multi_tenant/struct.ScimTenantConfiguration.html)**: SCIM-specific tenant settings

## Use Cases

### 1. SaaS Identity Provider

**Multi-customer identity management platform**

```rust
use scim_server::{ScimServerBuilder, TenantStrategy};
use scim_server::multi_tenant::{StaticTenantResolver, ScimTenantConfiguration};
use scim_server::resource::{TenantContext, TenantPermissions, RequestContext};

// Configure multi-tenant server with subdomain strategy
let server = ScimServerBuilder::new(provider)
    .with_base_url("https://identity.company.com")
    .with_tenant_strategy(TenantStrategy::Subdomain)
    .build()?;

// Set up tenant resolver for authentication
let mut resolver = StaticTenantResolver::new();

// Add customer tenants with specific limits
let acme_permissions = TenantPermissions {
    can_create: true,
    can_read: true,
    can_update: true,
    can_delete: false, // Restrict deletion for this customer
    max_users: Some(1000),
    max_groups: Some(50),
    ..Default::default()
};

let acme_context = TenantContext::new("acme-corp".to_string(), "scim-client-1".to_string())
    .with_permissions(acme_permissions);

resolver.add_tenant("api-key-acme-123", acme_context).await;

// Enterprise customer with higher limits
let enterprise_permissions = TenantPermissions {
    max_users: Some(10000),
    max_groups: Some(500),
    ..Default::default()
};

let enterprise_context = TenantContext::new("enterprise".to_string(), "scim-client-ent".to_string())
    .with_permissions(enterprise_permissions);

resolver.add_tenant("api-key-enterprise-456", enterprise_context).await;

// Operations automatically scoped to tenant
let tenant_context = resolver.resolve_tenant("api-key-acme-123").await?;
let context = RequestContext::with_tenant_generated_id(tenant_context);

// This user only exists within "acme-corp" tenant
let user = server.create_resource("User", user_data, &context).await?;
```

**Benefits**: Complete customer isolation, flexible billing models, tenant-specific limits.

### 2. Enterprise Multi-Division Identity

**Single enterprise with multiple business divisions**

```rust
// Path-based tenant strategy for internal divisions
let server = ScimServerBuilder::new(provider)
    .with_base_url("https://hr.enterprise.com")
    .with_tenant_strategy(TenantStrategy::PathBased)
    .build()?;

// Configure division-specific contexts
let hr_context = TenantContext::new("hr-division".to_string(), "hr-client".to_string())
    .with_isolation_level(IsolationLevel::Standard);

let engineering_context = TenantContext::new("engineering".to_string(), "eng-client".to_string())
    .with_isolation_level(IsolationLevel::Strict);

let sales_context = TenantContext::new("sales".to_string(), "sales-client".to_string())
    .with_isolation_level(IsolationLevel::Shared);

// Division-specific operations with different isolation levels
let hr_request = RequestContext::with_tenant_generated_id(hr_context);
let engineering_request = RequestContext::with_tenant_generated_id(engineering_context);
let sales_request = RequestContext::with_tenant_generated_id(sales_context);

// URLs generated: https://hr.enterprise.com/hr-division/v2/Users/123
//                 https://hr.enterprise.com/engineering/v2/Users/456
//                 https://hr.enterprise.com/sales/v2/Users/789
```

**Benefits**: Division autonomy, shared corporate policies, centralized management.

### 3. Development Environment Isolation

**Separate environments for development, staging, and production**

```rust
// Single server handling multiple environments as tenants
let server = ScimServerBuilder::new(provider)
    .with_base_url("https://scim-dev.company.com")
    .with_tenant_strategy(TenantStrategy::PathBased)
    .build()?;

// Development environment with relaxed permissions
let dev_permissions = TenantPermissions {
    can_create: true,
    can_update: true,
    can_delete: true, // Allow deletion in dev
    max_users: Some(100), // Smaller limits for dev
    max_groups: Some(10),
    ..Default::default()
};

let dev_context = TenantContext::new("development".to_string(), "dev-client".to_string())
    .with_permissions(dev_permissions)
    .with_isolation_level(IsolationLevel::Shared); // Allow cross-team access

// Staging environment with production-like restrictions
let staging_permissions = TenantPermissions {
    can_delete: false, // Restrict deletion in staging
    max_users: Some(500),
    max_groups: Some(25),
    ..Default::default()
};

let staging_context = TenantContext::new("staging".to_string(), "staging-client".to_string())
    .with_permissions(staging_permissions)
    .with_isolation_level(IsolationLevel::Standard);

// Production environment with strict limits
let prod_permissions = TenantPermissions {
    can_delete: false, // No deletion in production
    max_users: Some(5000),
    max_groups: Some(100),
    ..Default::default()
};

let prod_context = TenantContext::new("production".to_string(), "prod-client".to_string())
    .with_permissions(prod_permissions)
    .with_isolation_level(IsolationLevel::Strict);
```

**Benefits**: Environment isolation, development flexibility, production safety.

### 4. Geographic Data Residency

**Regional tenant isolation for compliance**

```rust
// Region-specific tenant configurations
let eu_server = ScimServerBuilder::new(eu_provider)
    .with_base_url("https://eu.identity.company.com")
    .with_tenant_strategy(TenantStrategy::Subdomain)
    .build()?;

let us_server = ScimServerBuilder::new(us_provider)
    .with_base_url("https://us.identity.company.com")
    .with_tenant_strategy(TenantStrategy::Subdomain)
    .build()?;

// EU tenant with GDPR compliance settings
let eu_context = TenantContext::new("customer-eu".to_string(), "eu-client".to_string())
    .with_isolation_level(IsolationLevel::Strict);

// US tenant with different compliance requirements
let us_context = TenantContext::new("customer-us".to_string(), "us-client".to_string())
    .with_isolation_level(IsolationLevel::Standard);

// Data automatically scoped to appropriate region and compliance rules
```

**Benefits**: Regulatory compliance, data residency, regional performance.

### 5. Customer White-Label Solutions

**Tenant-specific branding and configuration**

```rust
// Configure tenant-specific SCIM settings
let white_label_config = ScimTenantConfiguration::builder("customer-brand".to_string())
    .with_endpoint_path("/api/scim/v2")
    .with_scim_rate_limit(200, Duration::from_secs(60))
    .with_scim_client("brand-client-1", "custom-api-key")
    .enable_scim_audit_log()
    .with_custom_schema_extensions(vec![
        ScimSchemaExtension::new("urn:customer:schema:Brand", brand_attributes)
    ])
    .build()?;

// Server with customer-specific subdomain
let server = ScimServerBuilder::new(provider)
    .with_base_url("https://identity.customer.com")
    .with_tenant_strategy(TenantStrategy::SingleTenant) // Customer gets dedicated subdomain
    .build()?;

// Customer-specific extensions and branding automatically applied
```

**Benefits**: Brand consistency, customer-specific features, dedicated endpoints.

## Design Patterns

### Tenant Resolution Pattern

Authentication credentials map to tenant contexts:

```rust
pub trait TenantResolver: Send + Sync {
    type Error: std::error::Error + Send + Sync + 'static;

    async fn resolve_tenant(&self, credential: &str) -> Result<TenantContext, Self::Error>;
    async fn validate_tenant(&self, tenant_id: &str) -> Result<bool, Self::Error>;
    async fn get_all_tenants(&self) -> Result<Vec<String>, Self::Error>;
}
```

This pattern enables:
- Pluggable authentication strategies
- Credential-to-tenant mapping
- Dynamic tenant discovery
- Authentication audit trails

### Context Propagation Pattern

Request contexts carry tenant information through all operations:

```rust
pub struct RequestContext {
    pub request_id: String,
    tenant_context: Option<TenantContext>,
}

impl RequestContext {
    pub fn with_tenant_generated_id(tenant_context: TenantContext) -> Self;
    pub fn tenant_id(&self) -> Option<&str>;
    pub fn is_multi_tenant(&self) -> bool;
    pub fn can_perform_operation(&self, operation: &str) -> bool;
}
```

This ensures:
- Automatic tenant scoping
- Permission validation
- Audit trail continuity
- Consistent isolation

### URL Generation Strategy Pattern

Flexible tenant URL patterns:

```rust
pub enum TenantStrategy {
    SingleTenant,        // https://api.com/v2/Users/123
    Subdomain,          // https://tenant.api.com/v2/Users/123
    PathBased,          // https://api.com/tenant/v2/Users/123
}

pub fn generate_ref_url(&self, tenant_id: Option<&str>, resource_type: &str, resource_id: &str) -> Result<String, ScimError>;
```

This provides:
- Deployment flexibility
- Customer preferences accommodation
- Migration path support
- SCIM $ref compliance

### Storage Isolation Pattern

Multi-tenant providers ensure data separation:

```rust
pub trait MultiTenantProvider: ResourceProvider {
    fn effective_tenant_id(&self, context: &RequestContext) -> String;
    fn tenant_scoped_key(&self, tenant_id: &str, resource_type: &str, resource_id: &str) -> String;
    fn tenant_scoped_prefix(&self, tenant_id: &str, resource_type: &str) -> String;
}
```

This guarantees:
- Data isolation at storage level
- Consistent key generation
- Tenant-scoped queries
- Cross-tenant access prevention

## Integration with Other Components

### SCIM Server Integration

The SCIM Server provides multi-tenant orchestration:

- **Automatic Tenant Handling**: All operations automatically scoped to tenant context
- **URL Generation**: Server configuration drives tenant-aware URL generation
- **Permission Enforcement**: Tenant permissions validated before operations
- **Resource Limits**: Tenant quotas enforced during resource creation

### Resource Provider Integration

Resource Providers implement tenant isolation:

- **Context Propagation**: Tenant context flows through all provider operations
- **Scoped Operations**: All CRUD operations automatically tenant-scoped
- **Limit Enforcement**: Resource limits checked during creation operations
- **Audit Integration**: Tenant information included in all audit logs

### Storage Provider Integration

Storage layer ensures tenant data separation:

- **Key Prefixing**: All storage keys include tenant identifiers
- **Query Scoping**: List and search operations automatically scoped to tenant
- **Batch Operations**: Multi-resource operations maintain tenant boundaries
- **Migration Support**: Tenant data can be moved between storage backends

### Resource Integration

Resources maintain tenant awareness:

- **Metadata Injection**: Tenant information included in resource metadata
- **Reference Generation**: $ref fields use tenant-specific URLs
- **Version Control**: Tenant-scoped version management
- **Schema Extensions**: Tenant-specific schema customizations supported

## Security Considerations

### Isolation Levels

Three levels of tenant isolation:

```rust
pub enum IsolationLevel {
    Strict,    // Complete separation, no shared resources
    Standard,  // Shared infrastructure, separate data
    Shared,    // Some resources may be shared between tenants
}
```

Each level provides different security and resource sharing characteristics.

### Permission Management

Granular tenant permissions:

```rust
pub struct TenantPermissions {
    pub can_create: bool,
    pub can_read: bool, 
    pub can_update: bool,
    pub can_delete: bool,
    pub can_list: bool,
    pub max_users: Option<usize>,
    pub max_groups: Option<usize>,
}
```

Enables fine-grained control over tenant capabilities.

### Credential Security

- **Secure Resolution**: TenantResolver implementations should use secure credential storage
- **Rate Limiting**: Built-in rate limiting prevents authentication attacks
- **Audit Logging**: All authentication attempts logged for security monitoring
- **Token Validation**: Support for various authentication schemes (API keys, JWT, OAuth)

## Best Practices

### 1. Choose Appropriate Tenant Strategy

Select the tenant strategy based on deployment requirements:

```rust
// Good: Match strategy to deployment model
let saas_server = ScimServerBuilder::new(provider)
    .with_tenant_strategy(TenantStrategy::Subdomain) // Clear tenant separation
    .build()?;

let enterprise_server = ScimServerBuilder::new(provider)
    .with_tenant_strategy(TenantStrategy::PathBased) // Internal division structure
    .build()?;

// Avoid: Using single tenant for multi-customer SaaS
let wrong_server = ScimServerBuilder::new(provider)
    .with_tenant_strategy(TenantStrategy::SingleTenant) // No tenant isolation
    .build()?;
```

### 2. Implement Robust Tenant Resolution

Use secure and efficient tenant resolution:

```rust
// Good: Database-backed with caching
struct DatabaseTenantResolver {
    db: DatabasePool,
    cache: Arc<RwLock<HashMap<String, TenantContext>>>,
}

impl TenantResolver for DatabaseTenantResolver {
    async fn resolve_tenant(&self, credential: &str) -> Result<TenantContext, Self::Error> {
        // Check cache first
        if let Some(context) = self.cache.read().await.get(credential) {
            return Ok(context.clone());
        }
        
        // Query database with secure credential comparison
        let context = self.db.get_tenant_by_credential(credential).await?;
        
        // Cache result
        self.cache.write().await.insert(credential.to_string(), context.clone());
        Ok(context)
    }
}

// Avoid: Hardcoded credentials in production
let static_resolver = StaticTenantResolver::new(); // Only for testing/examples
```

### 3. Set Appropriate Resource Limits

Define realistic tenant resource limits:

```rust
// Good: Tiered limits based on customer plan
let basic_permissions = TenantPermissions {
    max_users: Some(100),
    max_groups: Some(10),
    can_delete: false, // Prevent accidental data loss
    ..Default::default()
};

let enterprise_permissions = TenantPermissions {
    max_users: Some(10000),
    max_groups: Some(500),
    can_delete: true, // Full capabilities for enterprise
    ..Default::default()
};

// Avoid: Unlimited resources without business justification
let dangerous_permissions = TenantPermissions {
    max_users: None, // Could lead to resource exhaustion
    max_groups: None,
    ..Default::default()
};
```

### 4. Use Proper Isolation Levels

Choose isolation level based on security requirements:

```rust
// Good: Strict isolation for sensitive data
let healthcare_context = TenantContext::new("hospital".to_string(), "health-client".to_string())
    .with_isolation_level(IsolationLevel::Strict); // HIPAA compliance

let internal_context = TenantContext::new("internal".to_string(), "internal-client".to_string())
    .with_isolation_level(IsolationLevel::Standard); // Standard business use

let dev_context = TenantContext::new("development".to_string(), "dev-client".to_string())
    .with_isolation_level(IsolationLevel::Shared); // Development flexibility

// Avoid: One-size-fits-all isolation
// Different tenants have different security requirements
```

### 5. Handle Multi-Tenant Errors Gracefully

Provide clear error messages for tenant issues:

```rust
// Good: Specific error handling
match server.create_resource("User", data, &context).await {
    Ok(user) => Ok(user),
    Err(ScimError::TenantLimitExceeded { limit, current }) => {
        HttpResponse::PaymentRequired()
            .json(json!({
                "error": "tenant_limit_exceeded",
                "message": format!("User limit of {} exceeded (current: {})", limit, current)
            }))
    },
    Err(ScimError::TenantNotFound { tenant_id }) => {
        HttpResponse::Unauthorized()
            .json(json!({
                "error": "invalid_tenant",
                "message": "Tenant not found or inactive"
            }))
    },
    Err(e) => handle_other_errors(e),
}

// Avoid: Generic error handling that loses tenant context
```

## When to Use Multi-Tenant Architecture

### Primary Scenarios

1. **Software-as-a-Service (SaaS)**: Multiple customers sharing infrastructure
2. **Enterprise Divisions**: Large organizations with multiple business units
3. **Development Environments**: Separate dev/staging/production environments
4. **Geographic Regions**: Compliance-driven data residency requirements
5. **White-Label Solutions**: Customer-specific branding and configuration

### Implementation Strategies

| Scenario | Strategy | Complexity | Isolation |
|----------|----------|------------|-----------|
| SaaS Multi-Customer | Subdomain | Medium | High |
| Enterprise Divisions | Path-Based | Low | Medium |
| Environment Separation | Path-Based | Low | High |
| Geographic Regions | Separate Deployments | High | Very High |
| White-Label | Single Tenant per Domain | Medium | Very High |

## Comparison with Alternative Approaches

| Approach | Isolation | Scalability | Complexity | Compliance |
|----------|-----------|-------------|------------|------------|
| **Multi-Tenant Architecture** | ✅ Complete | ✅ High | Medium | ✅ Excellent |
| Separate Deployments | ✅ Perfect | ⚠️ Limited | High | ✅ Excellent |
| Database Schemas | ⚠️ Good | ✅ High | Low | ⚠️ Good |
| Application Logic Only | ❌ Poor | ✅ High | Low | ❌ Poor |

The Multi-Tenant Architecture provides the optimal balance of isolation, scalability, and operational simplicity for identity management scenarios requiring tenant separation.

## Migration and Evolution

### Single to Multi-Tenant Migration

The architecture supports gradual migration:

1. **Start Single-Tenant**: Begin with `TenantStrategy::SingleTenant`
2. **Add Default Tenant**: Migrate existing data to "default" tenant context
3. **Enable Multi-Tenancy**: Switch to `TenantStrategy::PathBased` or `TenantStrategy::Subdomain`
4. **Add New Tenants**: Register additional tenants without affecting existing data

### Tenant Strategy Evolution

Tenant strategies can evolve as requirements change:

- **Development → Production**: Move from `PathBased` to `Subdomain` for customer isolation
- **Single → Multi**: Enable multi-tenancy without breaking existing integrations
- **Subdomain → Custom**: Transition to customer-specific domains as business grows

The Multi-Tenant Architecture in SCIM Server provides enterprise-grade multi-tenancy that scales from development environments to global SaaS platforms, ensuring complete tenant isolation while maintaining operational simplicity and SCIM compliance.