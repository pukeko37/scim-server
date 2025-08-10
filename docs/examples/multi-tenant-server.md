# Multi-Tenant Server Example

This document provides a complete example of building a multi-tenant SCIM server that can serve multiple organizations or contexts from a single server instance, with proper tenant isolation and configuration.

## Table of Contents

- [Overview](#overview)
- [Multi-Tenant Concepts](#multi-tenant-concepts)
- [Basic Multi-Tenant Setup](#basic-multi-tenant-setup)
- [Advanced Tenant Resolution](#advanced-tenant-resolution)
- [Per-Tenant Configuration](#per-tenant-configuration)
- [Tenant Isolation Testing](#tenant-isolation-testing)
- [Production Considerations](#production-considerations)
- [Migration from Single-Tenant](#migration-from-single-tenant)

## Overview

A multi-tenant SCIM server allows multiple organizations to share the same server infrastructure while maintaining complete data isolation. Each tenant has:

- Separate data storage (logical or physical)
- Independent configuration settings
- Isolated resource namespaces
- Custom schemas and validation rules
- Separate authentication and authorization

### Benefits of Multi-Tenancy

- **Cost Efficiency** - Shared infrastructure reduces operational costs
- **Maintenance** - Single codebase to maintain and update
- **Scalability** - Easier to scale resources for all tenants
- **Feature Consistency** - All tenants get the same feature set
- **Compliance** - Centralized security and compliance management

## Multi-Tenant Concepts

### Tenant Identification

Tenants are identified through various mechanisms:

1. **Subdomain-based**: `acme.scim.example.com`, `globex.scim.example.com`
2. **Path-based**: `/scim/acme/v2/Users`, `/scim/globex/v2/Users`
3. **Header-based**: `X-Tenant-ID: acme-corp`
4. **Token-based**: Embedded in JWT claims

### Tenant Context

Each request operates within a tenant context that provides:

```rust
use scim_server::multi_tenant::{TenantContext, TenantId, ScimConfig};

pub struct TenantContext {
    id: TenantId,
    config: ScimConfig,
    provider: Box<dyn ResourceProvider>,
}

impl TenantContext {
    pub fn id(&self) -> &TenantId { &self.id }
    pub fn config(&self) -> &ScimConfig { &self.config }
    pub fn resource_provider(&self) -> &dyn ResourceProvider { &*self.provider }
}
```

## Basic Multi-Tenant Setup

### Step 1: Static Tenant Configuration

Create `src/main.rs` with static tenant setup:

```rust
use scim_server::{ScimServer, ServerConfig};
use scim_server::multi_tenant::{
    StaticTenantResolver, TenantId, TenantContext, ScimConfig
};
use scim_server::providers::InMemoryProvider;
use scim_server::error::Result;
use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .with_target(false)
        .finish()
        .init();

    info!("Starting Multi-Tenant SCIM Server...");

    // Create tenant resolver
    let resolver = setup_tenants().await?;

    // Configure server with multi-tenant resolver
    let config = ServerConfig::builder()
        .host("localhost")
        .port(8080)
        .base_url("http://localhost:8080/scim/v2")
        .tenant_resolver(resolver)
        .enable_cors(true)
        .cors_origins(vec!["*"]) // Permissive for demo
        .enable_request_logging(true)
        .build()?;

    // Start server
    let server = ScimServer::new(config);
    
    print_tenant_info();
    
    server.run().await
}

async fn setup_tenants() -> Result<StaticTenantResolver> {
    let mut resolver = StaticTenantResolver::new();

    // Configure Acme Corporation
    setup_acme_tenant(&mut resolver).await?;
    
    // Configure Globex Corporation
    setup_globex_tenant(&mut resolver).await?;
    
    // Configure Development Tenant
    setup_dev_tenant(&mut resolver).await?;

    Ok(resolver)
}

async fn setup_acme_tenant(resolver: &mut StaticTenantResolver) -> Result<()> {
    let tenant_id = TenantId::new("acme-corp")?;
    let provider = InMemoryProvider::new();
    
    // Populate with sample Acme users
    populate_acme_data(&provider).await?;
    
    let config = ScimConfig::builder()
        .resource_types(vec!["User", "Group"])
        .schemas(vec![
            "urn:ietf:params:scim:schemas:core:2.0:User",
            "urn:ietf:params:scim:schemas:core:2.0:Group",
            "urn:ietf:params:scim:schemas:extension:enterprise:2.0:User"
        ])
        .provider(provider)
        .strict_validation(true)
        .max_results_per_page(50)
        .build()?;

    resolver.add_tenant(tenant_id, TenantContext::new(config))?;
    info!("Configured tenant: acme-corp");
    Ok(())
}

async fn setup_globex_tenant(resolver: &mut StaticTenantResolver) -> Result<()> {
    let tenant_id = TenantId::new("globex-corp")?;
    let provider = InMemoryProvider::new();
    
    // Populate with sample Globex users
    populate_globex_data(&provider).await?;
    
    let config = ScimConfig::builder()
        .resource_types(vec!["User", "Group", "Device"]) // Custom resource type
        .schemas(vec![
            "urn:ietf:params:scim:schemas:core:2.0:User",
            "urn:ietf:params:scim:schemas:core:2.0:Group",
            "urn:globex:scim:schemas:Device" // Custom schema
        ])
        .provider(provider)
        .strict_validation(false) // More relaxed validation
        .allow_unknown_attributes(true)
        .max_results_per_page(100)
        .build()?;

    resolver.add_tenant(tenant_id, TenantContext::new(config))?;
    info!("Configured tenant: globex-corp");
    Ok(())
}

async fn setup_dev_tenant(resolver: &mut StaticTenantResolver) -> Result<()> {
    let tenant_id = TenantId::new("development")?;
    let provider = InMemoryProvider::new();
    
    let config = ScimConfig::builder()
        .resource_types(vec!["User", "Group"])
        .schemas(vec!["urn:ietf:params:scim:schemas:core:2.0:User"])
        .provider(provider)
        .strict_validation(false)
        .allow_unknown_attributes(true)
        .enable_debug_mode(true)
        .build()?;

    resolver.add_tenant(tenant_id, TenantContext::new(config))?;
    info!("Configured tenant: development");
    Ok(())
}
```

### Step 2: Sample Data Population

```rust
use scim_server::resource::{ResourceBuilder};
use scim_server::resource::value_objects::{
    ResourceId, UserName, EmailAddress, Name, GroupMember
};

async fn populate_acme_data(provider: &InMemoryProvider) -> Result<()> {
    // Create Acme users
    let users = vec![
        create_user("acme-001", "alice.smith", "Alice", "Smith", "alice.smith@acme.com")?,
        create_user("acme-002", "bob.jones", "Bob", "Jones", "bob.jones@acme.com")?,
        create_user("acme-003", "carol.white", "Carol", "White", "carol.white@acme.com")?,
    ];

    for user in users {
        provider.create_resource(user).await?;
    }

    // Create Acme groups
    let admin_group = ResourceBuilder::new()
        .id(ResourceId::new("acme-admins")?)
        .display_name("Acme Administrators")
        .add_group_member(GroupMember::new_user(ResourceId::new("acme-001")?)
            .with_display_name("Alice Smith"))
        .build()?;

    let dev_group = ResourceBuilder::new()
        .id(ResourceId::new("acme-devs")?)
        .display_name("Acme Developers")
        .add_group_member(GroupMember::new_user(ResourceId::new("acme-002")?)
            .with_display_name("Bob Jones"))
        .add_group_member(GroupMember::new_user(ResourceId::new("acme-003")?)
            .with_display_name("Carol White"))
        .build()?;

    provider.create_resource(admin_group).await?;
    provider.create_resource(dev_group).await?;

    Ok(())
}

async fn populate_globex_data(provider: &InMemoryProvider) -> Result<()> {
    // Create Globex users with different attributes
    let users = vec![
        create_globex_user("globex-001", "john.doe", "John", "Doe", "john.doe@globex.com", "SALES")?,
        create_globex_user("globex-002", "jane.brown", "Jane", "Brown", "jane.brown@globex.com", "ENGINEERING")?,
        create_globex_user("globex-003", "mike.davis", "Mike", "Davis", "mike.davis@globex.com", "MARKETING")?,
    ];

    for user in users {
        provider.create_resource(user).await?;
    }

    Ok(())
}

fn create_user(
    id: &str,
    username: &str,
    given_name: &str,
    family_name: &str,
    email: &str,
) -> Result<Resource> {
    let name = Name::builder()
        .given_name(given_name)
        .family_name(family_name)
        .formatted(&format!("{} {}", given_name, family_name))
        .build();

    ResourceBuilder::new()
        .id(ResourceId::new(id)?)
        .user_name(UserName::new(username)?)
        .name(name)
        .display_name(&format!("{} {}", given_name, family_name))
        .add_email(EmailAddress::new(email)?.with_type("work").with_primary(true))
        .active(true)
        .build()
}

fn create_globex_user(
    id: &str,
    username: &str,
    given_name: &str,
    family_name: &str,
    email: &str,
    department: &str,
) -> Result<Resource> {
    let mut user = create_user(id, username, given_name, family_name, email)?;
    
    // Add custom department attribute for Globex
    user.set_attribute("department", serde_json::Value::String(department.to_string()))?;
    
    Ok(user)
}

fn print_tenant_info() {
    info!("=== Multi-Tenant SCIM Server Started ===");
    info!("Base URL: http://localhost:8080/scim/v2");
    info!("");
    info!("Configured Tenants:");
    info!("  1. acme-corp (strict validation, enterprise features)");
    info!("     - 3 users, 2 groups");
    info!("     - Access: Add 'X-Tenant-ID: acme-corp' header");
    info!("  2. globex-corp (relaxed validation, custom attributes)");
    info!("     - 3 users with department info");
    info!("     - Access: Add 'X-Tenant-ID: globex-corp' header");
    info!("  3. development (minimal validation, debug mode)");
    info!("     - Empty tenant for testing");
    info!("     - Access: Add 'X-Tenant-ID: development' header");
    info!("");
    info!("Example Usage:");
    info!("  curl -H 'X-Tenant-ID: acme-corp' http://localhost:8080/scim/v2/Users");
    info!("  curl -H 'X-Tenant-ID: globex-corp' http://localhost:8080/scim/v2/Users");
}
```

### Step 3: Tenant Resolution Middleware

```rust
// src/middleware/tenant_resolution.rs
use axum::{
    extract::{Request, State},
    middleware::Next,
    response::Response,
    http::HeaderMap,
};
use scim_server::multi_tenant::{TenantResolver, TenantContext};
use scim_server::error::{Result, ScimError};

pub async fn tenant_resolution_middleware<R: TenantResolver>(
    State(resolver): State<R>,
    headers: HeaderMap,
    mut request: Request,
    next: Next,
) -> Result<Response> {
    // Extract tenant hint from various sources
    let tenant_hint = extract_tenant_hint(&headers, &request)?;
    
    // Resolve tenant context
    let tenant_context = resolver.resolve_tenant(&tenant_hint).await
        .map_err(|_| ScimError::TenantNotFound { 
            tenant_id: tenant_hint.clone() 
        })?;
    
    // Add tenant context to request extensions
    request.extensions_mut().insert(tenant_context);
    
    // Continue processing
    next.run(request).await
}

fn extract_tenant_hint(headers: &HeaderMap, request: &Request) -> Result<String> {
    // Try X-Tenant-ID header first
    if let Some(tenant_id) = headers.get("X-Tenant-ID") {
        return Ok(tenant_id.to_str()
            .map_err(|_| ScimError::bad_request("Invalid tenant ID in header"))?
            .to_string());
    }
    
    // Try subdomain extraction
    if let Some(host) = headers.get("Host") {
        let host_str = host.to_str()
            .map_err(|_| ScimError::bad_request("Invalid host header"))?;
        
        if let Some(subdomain) = extract_subdomain(host_str) {
            return Ok(subdomain);
        }
    }
    
    // Try path-based extraction
    let path = request.uri().path();
    if let Some(tenant_from_path) = extract_tenant_from_path(path) {
        return Ok(tenant_from_path);
    }
    
    // Default tenant or error
    Err(ScimError::TenantResolutionFailed {
        hint: "No tenant identifier found".to_string(),
        reason: "Expected X-Tenant-ID header, subdomain, or path prefix".to_string(),
    })
}

fn extract_subdomain(host: &str) -> Option<String> {
    let parts: Vec<&str> = host.split('.').collect();
    if parts.len() >= 3 {
        Some(parts[0].to_string())
    } else {
        None
    }
}

fn extract_tenant_from_path(path: &str) -> Option<String> {
    // Extract from path like /scim/tenant-id/v2/Users
    let parts: Vec<&str> = path.split('/').collect();
    if parts.len() >= 3 && parts[1] == "scim" {
        Some(parts[2].to_string())
    } else {
        None
    }
}
```

## Advanced Tenant Resolution

### Database-Backed Tenant Resolution

```rust
// src/tenant_resolver.rs
use scim_server::multi_tenant::{TenantResolver, TenantContext, TenantId, ScimConfig};
use scim_server::providers::DatabaseProvider;
use scim_server::error::{Result, ScimError};
use async_trait::async_trait;
use sqlx::{PgPool, Row};
use std::collections::HashMap;
use tokio::sync::RwLock;

pub struct DatabaseTenantResolver {
    db_pool: PgPool,
    cache: RwLock<HashMap<String, TenantContext>>,
    cache_ttl: std::time::Duration,
}

impl DatabaseTenantResolver {
    pub async fn new(database_url: &str) -> Result<Self> {
        let db_pool = PgPool::connect(database_url).await
            .map_err(|e| ScimError::internal_error(format!("Failed to connect to database: {}", e)))?;
        
        // Ensure tenant configuration table exists
        sqlx::query(r#"
            CREATE TABLE IF NOT EXISTS tenant_configs (
                tenant_id VARCHAR(255) PRIMARY KEY,
                display_name VARCHAR(255) NOT NULL,
                database_url VARCHAR(1000),
                config_json JSONB NOT NULL,
                active BOOLEAN DEFAULT true,
                created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
                updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
            )
        "#)
        .execute(&db_pool)
        .await
        .map_err(|e| ScimError::internal_error(format!("Failed to create tenant table: {}", e)))?;

        Ok(Self {
            db_pool,
            cache: RwLock::new(HashMap::new()),
            cache_ttl: std::time::Duration::from_secs(300), // 5 minutes
        })
    }
    
    async fn load_tenant_from_db(&self, tenant_id: &str) -> Result<TenantContext> {
        let row = sqlx::query(
            "SELECT tenant_id, display_name, database_url, config_json FROM tenant_configs WHERE tenant_id = $1 AND active = true"
        )
        .bind(tenant_id)
        .fetch_optional(&self.db_pool)
        .await
        .map_err(|e| ScimError::internal_error(format!("Database query failed: {}", e)))?;
        
        let row = row.ok_or_else(|| ScimError::TenantNotFound {
            tenant_id: tenant_id.to_string(),
        })?;
        
        let config_json: serde_json::Value = row.get("config_json");
        let tenant_config: TenantConfiguration = serde_json::from_value(config_json)
            .map_err(|e| ScimError::internal_error(format!("Invalid tenant config: {}", e)))?;
        
        // Create provider for this tenant
        let provider = if let Some(db_url) = row.get::<Option<String>, _>("database_url") {
            Box::new(DatabaseProvider::new(&db_url).await?) as Box<dyn ResourceProvider>
        } else {
            Box::new(InMemoryProvider::new()) as Box<dyn ResourceProvider>
        };
        
        let scim_config = ScimConfig::builder()
            .resource_types(tenant_config.resource_types)
            .schemas(tenant_config.schemas)
            .provider(provider)
            .strict_validation(tenant_config.strict_validation)
            .max_results_per_page(tenant_config.max_results_per_page)
            .build()?;
        
        Ok(TenantContext::new(scim_config))
    }
}

#[async_trait]
impl TenantResolver for DatabaseTenantResolver {
    async fn resolve_tenant(&self, hint: &str) -> Result<TenantContext> {
        // Check cache first
        {
            let cache = self.cache.read().await;
            if let Some(context) = cache.get(hint) {
                return Ok(context.clone());
            }
        }
        
        // Load from database
        let context = self.load_tenant_from_db(hint).await?;
        
        // Cache the result
        {
            let mut cache = self.cache.write().await;
            cache.insert(hint.to_string(), context.clone());
        }
        
        Ok(context)
    }
    
    async fn list_tenants(&self) -> Result<Vec<TenantId>> {
        let rows = sqlx::query("SELECT tenant_id FROM tenant_configs WHERE active = true")
            .fetch_all(&self.db_pool)
            .await
            .map_err(|e| ScimError::internal_error(format!("Failed to list tenants: {}", e)))?;
        
        let tenant_ids = rows.into_iter()
            .map(|row| TenantId::new(row.get::<String, _>("tenant_id")))
            .collect::<Result<Vec<_>, _>>()?;
        
        Ok(tenant_ids)
    }
}

#[derive(serde::Deserialize, serde::Serialize)]
struct TenantConfiguration {
    resource_types: Vec<String>,
    schemas: Vec<String>,
    strict_validation: bool,
    max_results_per_page: usize,
}
```

### Dynamic Tenant Registration

```rust
// src/tenant_management.rs
use scim_server::multi_tenant::{TenantId, TenantContext, ScimConfig};
use scim_server::providers::InMemoryProvider;
use scim_server::error::Result;

pub struct TenantManager {
    resolver: DatabaseTenantResolver,
}

impl TenantManager {
    pub fn new(resolver: DatabaseTenantResolver) -> Self {
        Self { resolver }
    }
    
    pub async fn register_tenant(
        &self,
        tenant_id: &str,
        display_name: &str,
        config: TenantRegistrationRequest,
    ) -> Result<TenantContext> {
        // Validate tenant ID is available
        if self.resolver.resolve_tenant(tenant_id).await.is_ok() {
            return Err(ScimError::Conflict {
                message: format!("Tenant '{}' already exists", tenant_id),
                existing_resource: Some(tenant_id.to_string()),
            });
        }
        
        // Create provider for new tenant
        let provider = match config.provider_type.as_str() {
            "memory" => Box::new(InMemoryProvider::new()) as Box<dyn ResourceProvider>,
            "database" => {
                let db_url = config.database_url.ok_or_else(|| {
                    ScimError::bad_request("Database URL required for database provider")
                })?;
                Box::new(DatabaseProvider::new(&db_url).await?) as Box<dyn ResourceProvider>
            }
            _ => return Err(ScimError::bad_request(
                format!("Unsupported provider type: {}", config.provider_type)
            )),
        };
        
        // Create SCIM configuration
        let scim_config = ScimConfig::builder()
            .resource_types(config.resource_types)
            .schemas(config.schemas)
            .provider(provider)
            .strict_validation(config.strict_validation.unwrap_or(true))
            .max_results_per_page(config.max_results_per_page.unwrap_or(20))
            .build()?;
        
        let tenant_context = TenantContext::new(scim_config);
        
        // Store in database
        let config_json = serde_json::to_value(&TenantConfiguration {
            resource_types: config.resource_types,
            schemas: config.schemas,
            strict_validation: config.strict_validation.unwrap_or(true),
            max_results_per_page: config.max_results_per_page.unwrap_or(20),
        })?;
        
        sqlx::query(r#"
            INSERT INTO tenant_configs (tenant_id, display_name, database_url, config_json)
            VALUES ($1, $2, $3, $4)
        "#)
        .bind(tenant_id)
        .bind(display_name)
        .bind(config.database_url)
        .bind(config_json)
        .execute(&self.resolver.db_pool)
        .await
        .map_err(|e| ScimError::internal_error(format!("Failed to register tenant: {}", e)))?;
        
        info!("Registered new tenant: {} ({})", tenant_id, display_name);
        Ok(tenant_context)
    }
    
    pub async fn unregister_tenant(&self, tenant_id: &str) -> Result<()> {
        sqlx::query("UPDATE tenant_configs SET active = false WHERE tenant_id = $1")
            .bind(tenant_id)
            .execute(&self.resolver.db_pool)
            .await
            .map_err(|e| ScimError::internal_error(format!("Failed to unregister tenant: {}", e)))?;
        
        // Clear from cache
        {
            let mut cache = self.resolver.cache.write().await;
            cache.remove(tenant_id);
        }
        
        info!("Unregistered tenant: {}", tenant_id);
        Ok(())
    }
}

#[derive(serde::Deserialize)]
pub struct TenantRegistrationRequest {
    pub provider_type: String,
    pub database_url: Option<String>,
    pub resource_types: Vec<String>,
    pub schemas: Vec<String>,
    pub strict_validation: Option<bool>,
    pub max_results_per_page: Option<usize>,
}
```

## Per-Tenant Configuration

### Different Providers per Tenant

```rust
async fn setup_heterogeneous_tenants() -> Result<StaticTenantResolver> {
    let mut resolver = StaticTenantResolver::new();
    
    // Tenant with in-memory storage (fast, temporary)
    resolver.add_tenant(
        TenantId::new("memory-tenant")?,
        TenantContext::new(ScimConfig::builder()
            .provider(InMemoryProvider::new())
            .build()?)
    )?;
    
    // Tenant with PostgreSQL storage (persistent)
    resolver.add_tenant(
        TenantId::new("postgres-tenant")?,
        TenantContext::new(ScimConfig::builder()
            .provider(DatabaseProvider::new("postgresql://user:pass@localhost/tenant_db").await?)
            .build()?)
    )?;
    
    // Tenant with Redis storage (cached)
    resolver.add_tenant(
        TenantId::new("redis-tenant")?,
        TenantContext::new(ScimConfig::builder()
            .provider(RedisProvider::new("redis://localhost:6379/1").await?)
            .build()?)
    )?;
    
    // Tenant with external API provider (proxy)
    resolver.add_tenant(
        TenantId::new("api-tenant")?,
        TenantContext::new(ScimConfig::builder()
            .provider(ApiProvider::new("https://external-api.example.com", "api-key")?)
            .build()?)
    )?;
    
    Ok(resolver)
}
```

### Tenant-Specific Schema Configuration

```rust
use scim_server::schema::{Schema, SchemaBuilder, AttributeDefinition};

async fn setup_tenant_with_custom_schema() -> Result<TenantContext> {
    // Define custom employee schema
    let employee_schema = SchemaBuilder::new()
        .id("urn:company:scim:schemas:Employee")
        .name("Employee")
        .description("Extended employee attributes")
        .add_attribute(AttributeDefinition::builder()
            .name("employeeNumber")
            .type_("string")
            .required(true)
            .unique(true)
            .description("Unique employee identifier")
            .build()?)
        .add_attribute(AttributeDefinition::builder()
            .name("department")
            .type_("string")
            .required(false)
            .canonical_values(vec!["HR", "Engineering", "Sales", "Marketing"])
            .build()?)
        .add_attribute(AttributeDefinition::builder()
            .name("startDate")
            .type_("dateTime")
            .required(false)
            .build()?)
        .add_attribute(AttributeDefinition::builder()
            .name("manager")
            .type_("reference")
            .reference_types(vec!["User"])
            .required(false)
            .build()?)
        .build()?;
    
    let config = ScimConfig::builder()
        .resource_types(vec!["User", "Group"])
        .schemas(vec![
            "urn:ietf:params:scim:schemas:core:2.0:User",
            "urn:ietf:params:scim:schemas:core:2.0:Group",
            "urn:company:scim:schemas:Employee"
        ])
        .custom_schema(employee_schema)
        .provider(InMemoryProvider::new())
        .strict_validation(true)
        .build()?;
    
    Ok(TenantContext::new(config))
}
```

### Tenant-Specific Validation Rules

```rust
pub struct TenantValidationRules {
    tenant_id: TenantId,
}

impl TenantValidationRules {
    pub async fn validate_user_creation(&self, user: &Resource) -> Result<()> {
        match self.tenant_id.as_str() {
            "enterprise-corp" => self.validate_enterprise_user(user).await,
            "startup-inc" => self.validate_startup_user(user).await,
            _ => Ok(()), // Default validation only
        }
    }
    
    async fn validate_enterprise_user(&self, user: &Resource) -> Result<()> {
        // Enterprise users must have employee number
        if user.get_attribute("employeeNumber").is_none() {
            return Err(ScimError::validation_error(
                "employeeNumber",
                "Employee number is required for enterprise users"
            ));
        }
        
        // Must have a manager (except for CEO)
        if user.get_attribute("manager").is_none() {
            if let Some(title) = user.get_attribute("title") {
                if title.as_str() != Some("CEO") {
                    return Err(ScimError::validation_error(
                        "manager",
                        "All enterprise users must have a manager (except CEO)"
                    ));
                }
            }
        }
        
        Ok(())
    }
    
    async fn validate_startup_user(&self, user: &Resource) -> Result<()> {
        // Startup users must have GitHub username
        if user.get_attribute("githubUsername").is_none() {
            return Err(ScimError::validation_error(
                "githubUsername",
                "GitHub username is required for all startup employees"
            ));
        }
        
        Ok(())
    }
}
```

## Tenant Isolation Testing

### Testing Tenant Data Isolation

```rust
// tests/multi_tenant_tests.rs
use scim_server::multi_tenant::{StaticTenantResolver, TenantId, TenantContext};
use scim_server::providers::InMemoryProvider;
use scim_server::resource::{ResourceBuilder, ResourceType};

#[tokio::test]
async fn test_tenant_data_isolation() {
    let mut resolver = StaticTenantResolver::new();
    
    // Setup two tenants with separate providers
    let tenant_a_provider = InMemoryProvider::new();
    let tenant_b_provider = InMemoryProvider::new();
    
    resolver.add_tenant(
        TenantId::new("tenant-a")?,
        TenantContext::new(ScimConfig::builder()
            .provider(tenant_a_provider)
            .build()?)
    )?;
    
    resolver.add_tenant(
        TenantId::new("tenant-b")?,
        TenantContext::new(ScimConfig::builder()
            .provider(tenant_b_provider)
            .build()?)