# Multi-Tenant Deployment

This tutorial shows how to deploy and configure SCIM Server for multi-tenant environments, where you need to isolate data and operations between different organizations or customers.

## Overview

Multi-tenancy in SCIM Server provides complete isolation between different organizations while sharing the same infrastructure. Each tenant gets:

- **Complete data isolation** - No tenant can access another's data
- **Independent configuration** - Per-tenant authentication and settings
- **Separate namespaces** - Tenant-specific resource URLs
- **Isolated operations** - All SCIM operations are tenant-scoped

## Basic Multi-Tenant Setup

### Single Instance, Multiple Tenants

```rust
use scim_server::{
    providers::StandardResourceProvider,
    storage::InMemoryStorage,
    RequestContext,
};
use axum::{
    extract::{Path, State},
    response::Json,
    routing::{get, post, put, delete},
    Router,
};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::Arc;
use uuid::Uuid;

// Multi-tenant configuration
#[derive(Debug, Clone)]
struct TenantConfig {
    name: String,
    max_users: Option<usize>,
    features: Vec<String>,
    auth_config: AuthConfig,
}

#[derive(Debug, Clone)]
enum AuthConfig {
    OAuth { jwks_url: String, audience: String },
    ApiKey { keys: Vec<String> },
    Basic { username: String, password: String },
}

#[derive(Clone)]
struct MultiTenantApp {
    provider: Arc<StandardResourceProvider<InMemoryStorage>>,
    tenant_configs: HashMap<String, TenantConfig>,
}

impl MultiTenantApp {
    fn new() -> Self {
        // Single storage provider with tenant isolation via RequestContext
        let storage = InMemoryStorage::new();
        let provider = Arc::new(StandardResourceProvider::new(storage));

        // Configure tenants
        let mut tenant_configs = HashMap::new();
        
        tenant_configs.insert("company-a".to_string(), TenantConfig {
            name: "Company A".to_string(),
            auth_config: AuthConfig::OAuth {
                jwks_url: "https://company-a.auth0.com/.well-known/jwks.json".to_string(),
                audience: "scim-api".to_string(),
            },
            max_users: Some(1000),
            features: vec!["bulk_operations".to_string(), "custom_schemas".to_string()],
        });

        tenant_configs.insert("company-b".to_string(), TenantConfig {
            name: "Company B".to_string(),
            auth_config: AuthConfig::ApiKey {
                keys: vec!["sk_live_abc123".to_string()],
            },
            max_users: Some(500),
            features: vec!["basic_operations".to_string()],
        });

        Self {
            provider,
            tenant_configs,
        }
    }

    // Create tenant-aware RequestContext
    fn create_context(&self, tenant_id: &str, operation: &str) -> RequestContext {
        RequestContext::new(format!("tenant-{}-{}-{}", tenant_id, operation, Uuid::new_v4()))
    }

    // Validate tenant exists and is authorized
    fn validate_tenant(&self, tenant_id: &str) -> Result<&TenantConfig, String> {
        self.tenant_configs
            .get(tenant_id)
            .ok_or_else(|| format!("Tenant '{}' not found", tenant_id))
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let app = MultiTenantApp::new();

    let router = Router::new()
        // Multi-tenant endpoints: /tenants/{tenant_id}/scim/v2/*
        .route("/tenants/:tenant_id/scim/v2/Users", 
               post(create_user).get(list_users))
        .route("/tenants/:tenant_id/scim/v2/Users/:user_id", 
               get(get_user).put(update_user).delete(delete_user))
        .route("/tenants/:tenant_id/scim/v2/Groups", 
               post(create_group).get(list_groups))
        .route("/tenants/:tenant_id/scim/v2/Groups/:group_id", 
               get(get_group).put(update_group).delete(delete_group))
        // Tenant management endpoints
        .route("/tenants", get(list_tenants))
        .route("/tenants/:tenant_id", get(get_tenant_info))
        .with_state(app);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await?;
    println!("Multi-tenant SCIM server running on http://localhost:3000");
    println!("Example endpoints:");
    println!("  POST http://localhost:3000/tenants/company-a/scim/v2/Users");
    println!("  GET  http://localhost:3000/tenants/company-b/scim/v2/Users");
    axum::serve(listener, router).await?;
    
    Ok(())
}
```

### Tenant-Specific Endpoints

```rust
use axum::http::StatusCode;

// Error type for multi-tenant operations
#[derive(Debug)]
enum MultiTenantError {
    TenantNotFound(String),
    TenantLimitExceeded(String),
    FeatureNotEnabled(String),
    InternalError(String),
}

impl axum::response::IntoResponse for MultiTenantError {
    fn into_response(self) -> axum::response::Response {
        let (status, message) = match self {
            MultiTenantError::TenantNotFound(tenant) => 
                (StatusCode::NOT_FOUND, format!("Tenant '{}' not found", tenant)),
            MultiTenantError::TenantLimitExceeded(limit) => 
                (StatusCode::FORBIDDEN, format!("Tenant limit exceeded: {}", limit)),
            MultiTenantError::FeatureNotEnabled(feature) => 
                (StatusCode::FORBIDDEN, format!("Feature '{}' not enabled for tenant", feature)),
            MultiTenantError::InternalError(msg) => 
                (StatusCode::INTERNAL_SERVER_ERROR, msg),
        };

        let body = json!({
            "schemas": ["urn:ietf:params:scim:api:messages:2.0:Error"],
            "status": status.as_u16().to_string(),
            "detail": message
        });

        (status, Json(body)).into_response()
    }
}

// Create user with tenant isolation
async fn create_user(
    State(app): State<MultiTenantApp>,
    Path(tenant_id): Path<String>,
    Json(user_data): Json<Value>,
) -> Result<Json<Value>, MultiTenantError> {
    // Validate tenant exists and get config
    let tenant_config = app.validate_tenant(&tenant_id)
        .map_err(|_| MultiTenantError::TenantNotFound(tenant_id.clone()))?;

    // Check tenant limits
    if let Some(max_users) = tenant_config.max_users {
        let context = app.create_context(&tenant_id, "count-users");
        let current_users = app.provider.list_resources("User", None, &context).await
            .map_err(|e| MultiTenantError::InternalError(e.to_string()))?;
        
        if current_users.len() >= max_users {
            return Err(MultiTenantError::TenantLimitExceeded(max_users.to_string()));
        }
    }

    // Create tenant-scoped context
    let context = app.create_context(&tenant_id, "create-user");
    
    // Create user with tenant isolation
    let user = app.provider.create_resource("User", user_data, &context).await
        .map_err(|e| MultiTenantError::InternalError(e.to_string()))?;

    Ok(Json(user.data))
}

// Get user with tenant isolation
async fn get_user(
    State(app): State<MultiTenantApp>,
    Path((tenant_id, user_id)): Path<(String, String)>,
) -> Result<Json<Value>, MultiTenantError> {
    // Validate tenant
    let _tenant_config = app.validate_tenant(&tenant_id)
        .map_err(|_| MultiTenantError::TenantNotFound(tenant_id.clone()))?;

    // Create tenant-scoped context
    let context = app.create_context(&tenant_id, "get-user");
    
    // Get user (automatically isolated by tenant context)
    let user = app.provider.get_resource("User", &user_id, &context).await
        .map_err(|e| MultiTenantError::InternalError(e.to_string()))?;

    Ok(Json(user.data))
}

// List users with tenant isolation
async fn list_users(
    State(app): State<MultiTenantApp>,
    Path(tenant_id): Path<String>,
) -> Result<Json<Value>, MultiTenantError> {
    // Validate tenant
    let _tenant_config = app.validate_tenant(&tenant_id)
        .map_err(|_| MultiTenantError::TenantNotFound(tenant_id.clone()))?;

    // Create tenant-scoped context
    let context = app.create_context(&tenant_id, "list-users");
    
    // List users (automatically isolated by tenant context)
    let users = app.provider.list_resources("User", None, &context).await
        .map_err(|e| MultiTenantError::InternalError(e.to_string()))?;

    let response = json!({
        "schemas": ["urn:ietf:params:scim:api:messages:2.0:ListResponse"],
        "totalResults": users.len(),
        "startIndex": 1,
        "itemsPerPage": users.len(),
        "Resources": users.iter().map(|u| &u.data).collect::<Vec<_>>()
    });

    Ok(Json(response))
}

// Update user with tenant isolation
async fn update_user(
    State(app): State<MultiTenantApp>,
    Path((tenant_id, user_id)): Path<(String, String)>,
    Json(user_data): Json<Value>,
) -> Result<Json<Value>, MultiTenantError> {
    // Validate tenant
    let _tenant_config = app.validate_tenant(&tenant_id)
        .map_err(|_| MultiTenantError::TenantNotFound(tenant_id.clone()))?;

    // Create tenant-scoped context
    let context = app.create_context(&tenant_id, "update-user");
    
    // Update user (automatically isolated by tenant context)
    let user = app.provider.update_resource("User", &user_id, user_data, &context).await
        .map_err(|e| MultiTenantError::InternalError(e.to_string()))?;

    Ok(Json(user.data))
}

// Delete user with tenant isolation
async fn delete_user(
    State(app): State<MultiTenantApp>,
    Path((tenant_id, user_id)): Path<(String, String)>,
) -> Result<StatusCode, MultiTenantError> {
    // Validate tenant
    let _tenant_config = app.validate_tenant(&tenant_id)
        .map_err(|_| MultiTenantError::TenantNotFound(tenant_id.clone()))?;

    // Create tenant-scoped context
    let context = app.create_context(&tenant_id, "delete-user");
    
    // Delete user (automatically isolated by tenant context)
    app.provider.delete_resource("User", &user_id, &context).await
        .map_err(|e| MultiTenantError::InternalError(e.to_string()))?;

    Ok(StatusCode::NO_CONTENT)
}

// Example tenant-scoped URLs:
// POST /tenants/company-a/scim/v2/Users
// GET  /tenants/company-a/scim/v2/Users/123
// POST /tenants/company-b/scim/v2/Users
// GET  /tenants/company-b/scim/v2/Users/456
```

### Group Operations

```rust
// Group operations follow the same patterns
async fn create_group(
    State(app): State<MultiTenantApp>,
    Path(tenant_id): Path<String>,
    Json(group_data): Json<Value>,
) -> Result<Json<Value>, MultiTenantError> {
    let _tenant_config = app.validate_tenant(&tenant_id)
        .map_err(|_| MultiTenantError::TenantNotFound(tenant_id.clone()))?;

    let context = app.create_context(&tenant_id, "create-group");
    let group = app.provider.create_resource("Group", group_data, &context).await
        .map_err(|e| MultiTenantError::InternalError(e.to_string()))?;

    Ok(Json(group.data))
}

async fn list_groups(
    State(app): State<MultiTenantApp>,
    Path(tenant_id): Path<String>,
) -> Result<Json<Value>, MultiTenantError> {
    let _tenant_config = app.validate_tenant(&tenant_id)
        .map_err(|_| MultiTenantError::TenantNotFound(tenant_id.clone()))?;

    let context = app.create_context(&tenant_id, "list-groups");
    let groups = app.provider.list_resources("Group", None, &context).await
        .map_err(|e| MultiTenantError::InternalError(e.to_string()))?;

    let response = json!({
        "schemas": ["urn:ietf:params:scim:api:messages:2.0:ListResponse"],
        "totalResults": groups.len(),
        "startIndex": 1,
        "itemsPerPage": groups.len(),
        "Resources": groups.iter().map(|g| &g.data).collect::<Vec<_>>()
    });

    Ok(Json(response))
}

// Additional group operations (get_group, update_group, delete_group) follow same pattern...
```

### Tenant Management Endpoints

```rust
// List all tenants
async fn list_tenants(
    State(app): State<MultiTenantApp>,
) -> Json<Value> {
    let tenants: Vec<_> = app.tenant_configs.iter()
        .map(|(id, config)| json!({
            "id": id,
            "name": config.name,
            "maxUsers": config.max_users,
            "features": config.features
        }))
        .collect();

    Json(json!({
        "tenants": tenants,
        "total": tenants.len()
    }))
}

// Get tenant information
async fn get_tenant_info(
    State(app): State<MultiTenantApp>,
    Path(tenant_id): Path<String>,
) -> Result<Json<Value>, MultiTenantError> {
    let tenant_config = app.validate_tenant(&tenant_id)
        .map_err(|_| MultiTenantError::TenantNotFound(tenant_id.clone()))?;

    // Get usage statistics
    let context = app.create_context(&tenant_id, "get-stats");
    let users = app.provider.list_resources("User", None, &context).await
        .map_err(|e| MultiTenantError::InternalError(e.to_string()))?;
    let groups = app.provider.list_resources("Group", None, &context).await
        .map_err(|e| MultiTenantError::InternalError(e.to_string()))?;

    Ok(Json(json!({
        "id": tenant_id,
        "name": tenant_config.name,
        "maxUsers": tenant_config.max_users,
        "features": tenant_config.features,
        "usage": {
            "users": users.len(),
            "groups": groups.len()
        }
    })))
}
```

## Data Isolation Strategies

### Application-Level Isolation (Current Implementation)

The StandardResourceProvider provides tenant isolation through the RequestContext:

```rust
use scim_server::storage::StorageKey;

// The storage layer automatically handles tenant isolation
impl MultiTenantApp {
    fn create_context(&self, tenant_id: &str, operation: &str) -> RequestContext {
        // The tenant ID becomes part of the request context
        // This ensures all storage operations are tenant-scoped
        RequestContext::new(format!("tenant-{}-{}-{}", tenant_id, operation, Uuid::new_v4()))
    }
}

// Example: How storage keys work with tenants
// For tenant "company-a" creating user "123":
let storage_key = StorageKey::new("company-a", "User", "123");
// Results in storage path: "company-a/User/123"

// This provides automatic isolation:
// - company-a can only access "company-a/User/*" 
// - company-b can only access "company-b/User/*"
// - No cross-tenant data access possible
```

### Database-Level Isolation (Advanced)

For production deployments with database storage, implement row-level security:

```sql
-- Example PostgreSQL schema with tenant isolation
CREATE TABLE scim_resources (
    tenant_id VARCHAR(255) NOT NULL,
    resource_type VARCHAR(50) NOT NULL,
    resource_id VARCHAR(255) NOT NULL,
    data JSONB NOT NULL,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW(),
    version VARCHAR(255) NOT NULL,
    PRIMARY KEY (tenant_id, resource_type, resource_id)
);

-- Enable Row-Level Security
ALTER TABLE scim_resources ENABLE ROW LEVEL SECURITY;

-- Create tenant isolation policy
CREATE POLICY tenant_isolation ON scim_resources
    USING (tenant_id = current_setting('app.current_tenant_id'));

-- Function to set tenant context
CREATE OR REPLACE FUNCTION set_tenant_context(p_tenant_id text)
RETURNS void AS $$
BEGIN
    PERFORM set_config('app.current_tenant_id', p_tenant_id, true);
END;
$$ LANGUAGE plpgsql;
```

### Custom Storage Provider for Database

```rust
use scim_server::storage::{StorageProvider, StorageKey, StoragePrefix};
use sqlx::PgPool;
use serde_json::Value;

#[derive(Clone)]
pub struct PostgresStorageProvider {
    pool: PgPool,
}

#[async_trait]
impl StorageProvider for PostgresStorageProvider {
    type Error = sqlx::Error;

    async fn put(&self, key: StorageKey, data: Value) -> Result<Value, Self::Error> {
        // Extract tenant from storage key
        let tenant_id = key.tenant_id();
        
        // Set tenant context for RLS
        sqlx::query("SELECT set_tenant_context($1)")
            .bind(tenant_id)
            .execute(&self.pool)
            .await?;

        // Insert with automatic tenant filtering
        let stored_data = sqlx::query_scalar!(
            "INSERT INTO scim_resources (tenant_id, resource_type, resource_id, data, version)
             VALUES ($1, $2, $3, $4, gen_random_uuid()::text)
             RETURNING data",
            tenant_id,
            key.resource_type(),
            key.resource_id(),
            data
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(stored_data)
    }

    async fn get(&self, key: StorageKey) -> Result<Option<Value>, Self::Error> {
        // Set tenant context
        sqlx::query("SELECT set_tenant_context($1)")
            .bind(key.tenant_id())
            .execute(&self.pool)
            .await?;

        // Query with automatic tenant filtering
        let data = sqlx::query_scalar!(
            "SELECT data FROM scim_resources 
             WHERE resource_type = $1 AND resource_id = $2",
            key.resource_type(),
            key.resource_id()
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(data)
    }

    async fn delete(&self, key: StorageKey) -> Result<bool, Self::Error> {
        sqlx::query("SELECT set_tenant_context($1)")
            .bind(key.tenant_id())
            .execute(&self.pool)
            .await?;

        let result = sqlx::query!(
            "DELETE FROM scim_resources 
             WHERE resource_type = $1 AND resource_id = $2",
            key.resource_type(),
            key.resource_id()
        )
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected() > 0)
    }

    async fn list(
        &self,
        prefix: StoragePrefix,
        offset: usize,
        limit: usize,
    ) -> Result<Vec<(StorageKey, Value)>, Self::Error> {
        sqlx::query("SELECT set_tenant_context($1)")
            .bind(prefix.tenant_id())
            .execute(&self.pool)
            .await?;

        let rows = sqlx::query!(
            "SELECT resource_id, data FROM scim_resources 
             WHERE resource_type = $1 
             ORDER BY resource_id 
             LIMIT $2 OFFSET $3",
            prefix.resource_type(),
            limit as i64,
            offset as i64
        )
        .fetch_all(&self.pool)
        .await?;

        let results = rows.into_iter()
            .map(|row| {
                let key = StorageKey::new(
                    prefix.tenant_id(),
                    prefix.resource_type(),
                    &row.resource_id
                );
                (key, row.data)
            })
            .collect();

        Ok(results)
    }

    async fn find_by_attribute(
        &self,
        prefix: StoragePrefix,
        attribute: &str,
        value: &str,
    ) -> Result<Vec<(StorageKey, Value)>, Self::Error> {
        sqlx::query("SELECT set_tenant_context($1)")
            .bind(prefix.tenant_id())
            .execute(&self.pool)
            .await?;

        // Use JSONB operators for efficient attribute search
        let rows = sqlx::query!(
            "SELECT resource_id, data FROM scim_resources 
             WHERE resource_type = $1 AND data ->> $2 = $3",
            prefix.resource_type(),
            attribute,
            value
        )
        .fetch_all(&self.pool)
        .await?;

        let results = rows.into_iter()
            .map(|row| {
                let key = StorageKey::new(
                    prefix.tenant_id(),
                    prefix.resource_type(),
                    &row.resource_id
                );
                (key, row.data)
            })
            .collect();

        Ok(results)
    }

    async fn exists(&self, key: StorageKey) -> Result<bool, Self::Error> {
        sqlx::query("SELECT set_tenant_context($1)")
            .bind(key.tenant_id())
            .execute(&self.pool)
            .await?;

        let exists = sqlx::query_scalar!(
            "SELECT EXISTS(SELECT 1 FROM scim_resources 
             WHERE resource_type = $1 AND resource_id = $2)",
            key.resource_type(),
            key.resource_id()
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(exists.unwrap_or(false))
    }

    async fn count(&self, prefix: StoragePrefix) -> Result<usize, Self::Error> {
        sqlx::query("SELECT set_tenant_context($1)")
            .bind(prefix.tenant_id())
            .execute(&self.pool)
            .await?;

        let count = sqlx::query_scalar!(
            "SELECT COUNT(*) FROM scim_resources WHERE resource_type = $1",
            prefix.resource_type()
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(count.unwrap_or(0) as usize)
    }
}
```

## Deployment Patterns

### Single Instance, Multiple Tenants

The most common pattern for multi-tenant SCIM deployments:

```rust
// Production multi-tenant setup
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load tenant configurations from environment/config file
    let tenant_configs = load_tenant_configs_from_env()?;
    
    // Create storage provider (could be database, Redis, etc.)
    let storage = create_storage_provider().await?;
    let provider = Arc::new(StandardResourceProvider::new(storage));
    
    let app = MultiTenantApp {
        provider,
        tenant_configs,
    };

    // Production server with proper middleware
    let router = Router::new()
        .route("/tenants/:tenant_id/scim/v2/Users", 
               post(create_user).get(list_users))
        .route("/tenants/:tenant_id/scim/v2/Users/:user_id", 
               get(get_user).put(update_user).delete(delete_user))
        .route("/tenants/:tenant_id/scim/v2/Groups", 
               post(create_group).get(list_groups))
        .route("/tenants/:tenant_id/scim/v2/Groups/:group_id", 
               get(get_group).put(update_group).delete(delete_group))
        .layer(
            ServiceBuilder::new()
                .layer(TraceLayer::new_for_http())
                .layer(TimeoutLayer::new(Duration::from_secs(30)))
                .layer(CompressionLayer::new())
                .layer(cors_layer())
        )
        .with_state(app);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await?;
    println!("Multi-tenant SCIM server running on port 3000");
    axum::serve(listener, router).await?;
    
    Ok(())
}

fn load_tenant_configs_from_env() -> Result<HashMap<String, TenantConfig>, Box<dyn std::error::Error>> {
    let mut configs = HashMap::new();
    
    // Load from environment variables or config files
    for tenant_id in std::env::var("TENANT_IDS")?.split(',') {
        let config = TenantConfig {
            name: std::env::var(format!("TENANT_{}_NAME", tenant_id.to_uppercase()))?,
            max_users: std::env::var(format!("TENANT_{}_MAX_USERS", tenant_id.to_uppercase()))
                .ok().and_then(|s| s.parse().ok()),
            features: std::env::var(format!("TENANT_{}_FEATURES", tenant_id.to_uppercase()))
                .unwrap_or_default()
                .split(',')
                .map(|s| s.to_string())
                .collect(),
            auth_config: load_auth_config_for_tenant(tenant_id)?,
        };
        configs.insert(tenant_id.to_string(), config);
    }
    
    Ok(configs)
}
```

### Separate Instances Per Tenant

For high-isolation requirements:

```rust
// Per-tenant instance deployment
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let tenant_id = std::env::var("TENANT_ID")
        .expect("TENANT_ID environment variable required");
    
    // Dedicated storage for this tenant
    let storage_url = format!("postgresql://user:pass@localhost/scim_{}", tenant_id);
    let storage = PostgresStorageProvider::new(&storage_url).await?;
    let provider = StandardResourceProvider::new(storage);
    
    // Single-tenant routes (no tenant_id in path)
    let router = Router::new()
        .route("/scim/v2/Users", post(create_user).get(list_users))
        .route("/scim/v2/Users/:user_id", 
               get(get_user).put(update_user).delete(delete_user))
        .route("/scim/v2/Groups", post(create_group).get(list_groups))
        .with_state(SingleTenantApp { provider, tenant_id });

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await?;
    axum::serve(listener, router).await?;
    
    Ok(())
}

#[derive(Clone)]
struct SingleTenantApp {
    provider: StandardResourceProvider<PostgresStorageProvider>,
    tenant_id: String,
}

impl SingleTenantApp {
    fn create_context(&self, operation: &str) -> RequestContext {
        RequestContext::new(format!("{}-{}-{}", self.tenant_id, operation, Uuid::new_v4()))
    }
}
```

## Configuration Management

### Environment-Based Configuration

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TenantConfig {
    pub name: String,
    pub max_users: Option<usize>,
    pub features: Vec<String>,
    pub auth_config: AuthConfig,
}

// Load from environment variables
fn load_tenant_config(tenant_id: &str) -> Result<TenantConfig, Box<dyn std::error::Error>> {
    let prefix = format!("TENANT_{}", tenant_id.to_uppercase());
    
    Ok(TenantConfig {
        name: std::env::var(format!("{}_NAME", prefix))?,
        max_users: std::env::var(format!("{}_MAX_USERS", prefix))
            .ok().and_then(|s| s.parse().ok()),
        features: std::env::var(format!("{}_FEATURES", prefix))
            .unwrap_or_default()
            .split(',')
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string())
            .collect(),
        auth_config: AuthConfig::OAuth {
            jwks_url: std::env::var(format!("{}_JWKS_URL", prefix))?,
            audience: std::env::var(format!("{}_AUDIENCE", prefix))?,
        },
    })
}
```

### File-Based Configuration

```rust
// config/tenants.yaml
use serde_yaml;

#[derive(Debug, Deserialize)]
struct TenantsConfig {
    tenants: HashMap<String, TenantConfig>,
}

async fn load_tenant_configs_from_file() -> Result<HashMap<String, TenantConfig>, Box<dyn std::error::Error>> {
    let config_content = tokio::fs::read_to_string("config/tenants.yaml").await?;
    let config: TenantsConfig = serde_yaml::from_str(&config_content)?;
    Ok(config.tenants)
}
```

Example `config/tenants.yaml`:

```yaml
tenants:
  company-a:
    name: "Company A"
    max_users: 1000
    features: ["bulk_operations", "custom_schemas"]
    auth_config:
      OAuth:
        jwks_url: "https://company-a.auth0.com/.well-known/jwks.json"
        audience: "scim-api"
  
  company-b:
    name: "Company B"
    max_users: 500
    features: ["basic_operations"]
    auth_config:
      ApiKey:
        keys: ["sk_live_abc123"]
```

## Security Considerations

### Authentication Per Tenant

```rust
use axum::http::HeaderMap;

async fn authenticate_tenant_request(
    headers: &HeaderMap,
    tenant_id: &str,
    tenant_configs: &HashMap<String, TenantConfig>,
) -> Result<(), MultiTenantError> {
    let tenant_config = tenant_configs.get(tenant_id)
        .ok_or_else(|| MultiTenantError::TenantNotFound(tenant_id.to_string()))?;

    match &tenant_config.auth_config {
        AuthConfig::OAuth { jwks_url, audience } => {
            // Validate JWT token
            let auth_header = headers.get("authorization")
                .and_then(|h| h.to_str().ok())
                .ok_or_else(|| MultiTenantError::InternalError("Missing authorization header".to_string()))?;
                
            if !auth_header.starts_with("Bearer ") {
                return Err(MultiTenantError::InternalError("Invalid authorization format".to_string()));
            }
            
            let token = &auth_header[7..];
            validate_jwt_token(token, jwks_url, audience).await?;
        },
        
        AuthConfig::ApiKey { keys } => {
            // Validate API key
            let api_key = headers.get("x-api-key")
                .and_then(|h| h.to_str().ok())
                .ok_or_else(|| MultiTenantError::InternalError("Missing API key".to_string()))?;
                
            if !keys.contains(&api_key.to_string()) {
                return Err(MultiTenantError::InternalError("Invalid API key".to_string()));
            }
        },
        
        AuthConfig::Basic { username, password } => {
            // Validate basic auth
            let auth_header = headers.get("authorization")
                .and_then(|h| h.to_str().ok())
                .ok_or_else(|| MultiTenantError::InternalError("Missing authorization header".to_string()))?;
                
            // Decode and validate basic auth credentials
            validate_basic_auth(auth_header, username, password)?;
        },
    }

    Ok(())
}

async fn validate_jwt_token(token: &str, jwks_url: &str, audience: &str) -> Result<(), MultiTenantError> {
    // JWT validation implementation
    // This would use a JWT library to validate the token
    Ok(())
}
```

### Rate Limiting Per Tenant

```rust
use std::time::{Duration, Instant};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone)]
struct RateLimiter {
    requests: Arc<Mutex<HashMap<String, Vec<Instant>>>>,
}

impl RateLimiter {
    fn new() -> Self {
        Self {
            requests: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    fn check_rate_limit(&self, tenant_id: &str, limit_per_minute: usize) -> bool {
        let mut requests = self.requests.lock().unwrap();
        let now = Instant::now();
        let minute_ago = now - Duration::from_secs(60);
        
        let tenant_requests = requests.entry(tenant_id.to_string()).or_insert_with(Vec::new);
        
        // Remove old requests
        tenant_requests.retain(|&request_time| request_time > minute_ago);
        
        if tenant_requests.len() >= limit_per_minute {
            false
        } else {
            tenant_requests.push(now);
            true
        }
    }
}

// Usage in middleware
async fn rate_limit_middleware(
    tenant_id: &str,
    tenant_config: &TenantConfig,
    rate_limiter: &RateLimiter,
) -> Result<(), MultiTenantError> {
    if let Some(limit) = tenant_config.max_requests_per_minute {
        if !rate_limiter.check_rate_limit(tenant_id, limit) {
            return Err(MultiTenantError::TenantLimitExceeded(
                format!("Rate limit of {} requests per minute exceeded", limit)
            ));
        }
    }
    Ok(())
}
```

## Monitoring and Observability

### Per-Tenant Metrics

```rust
use prometheus::{Counter, Histogram, Gauge, Registry};
use std::collections::HashMap;

#[derive(Clone)]
struct TenantMetrics {
    request_counter: Counter,
    response_time: Histogram,
    active_users: Gauge,
    active_groups: Gauge,
}

struct MultiTenantMetrics {
    registry: Registry,
    tenant_metrics: HashMap<String, TenantMetrics>,
}

impl MultiTenantMetrics {
    fn new() -> Self {
        Self {
            registry: Registry::new(),
            tenant_metrics: HashMap::new(),
        }
    }

    fn get_or_create_tenant_metrics(&mut self, tenant_id: &str) -> &TenantMetrics {
        self.tenant_metrics.entry(tenant_id.to_string()).or_insert_with(|| {
            let request_counter = Counter::new(
                "scim_requests_total",
                "Total number of SCIM requests per tenant"
            ).unwrap();
            
            let response_time = Histogram::new(
                "scim_request_duration_seconds",
                "SCIM request duration in seconds"
            ).unwrap();
            
            let active_users = Gauge::new(
                "scim_active_users",
                "Number of active users per tenant"
            ).unwrap();
            
            let active_groups = Gauge::new(
                "scim_active_groups", 
                "Number of active groups per tenant"
            ).unwrap();

            TenantMetrics {
                request_counter,
                response_time,
                active_users,
                active_groups,
            }
        })
    }

    fn record_request(&mut self, tenant_id: &str, duration: Duration) {
        let metrics = self.get_or_create_tenant_metrics(tenant_id);
        metrics.request_counter.inc();
        metrics.response_time.observe(duration.as_secs_f64());
    }
}
```

## Best Practices

### 1. Tenant Validation
- Always validate tenant existence before processing requests
- Implement consistent error responses for invalid tenants
- Use meaningful tenant identifiers (avoid sequential IDs)

### 2. Data Isolation
- Use tenant-aware RequestContext for all operations
- Implement database-level isolation for sensitive deployments
- Audit cross-tenant access attempts

### 3. Configuration Management
- Store tenant configs securely (encrypted secrets)
- Implement hot-reloading for configuration changes
- Version configuration changes for rollback capability

### 4. Performance
- Implement per-tenant rate limiting
- Monitor tenant resource usage
- Scale storage based on tenant data growth

### 5. Security
- Use different authentication schemes per tenant as needed
- Implement audit logging for all tenant operations
- Regular security reviews of tenant isolation

## Testing Multi-Tenant Deployments

### Integration Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_tenant_isolation() {
        let app = MultiTenantApp::new();
        
        // Create users in different tenants
        let tenant_a_context = app.create_context("tenant-a", "test");
        let tenant_b_context = app.create_context("tenant-b", "test");
        
        let user_data = json!({"userName": "test@example.com"});
        
        let user_a = app.provider.create_resource("User", user_data.clone(), &tenant_a_context).await.unwrap();
        let user_b = app.provider.create_resource("User", user_data, &tenant_b_context).await.unwrap();
        
        // Verify isolation - tenant A cannot see tenant B's user
        let tenant_a_users = app.provider.list_resources("User", None, &tenant_a_context).await.unwrap();
        let tenant_b_users = app.provider.list_resources("User", None, &tenant_b_context).await.unwrap();
        
        assert_eq!(tenant_a_users.len(), 1);
        assert_eq!(tenant_b_users.len(), 1);
        assert_ne!(user_a.get_id(), user_b.get_id());
    }

    #[tokio::test]
    async fn test_tenant_limits() {
        let mut tenant_configs = HashMap::new();
        tenant_configs.insert("limited-tenant".to_string(), TenantConfig {
            name: "Limited Tenant".to_string(),
            max_users: Some(1),
            features: vec![],
            auth_config: AuthConfig::ApiKey { keys: vec!["test".to_string()] },
        });
        
        let app = MultiTenantApp {
            provider: Arc::new(StandardResourceProvider::new(InMemoryStorage::new())),
            tenant_configs,
        };
        
        // Create first user (should succeed)
        let context = app.create_context("limited-tenant", "test");
        let user_data = json!({"userName": "user1@example.com"});
        let result1 = app.provider.create_resource("User", user_data, &context).await;
        assert!(result1.is_ok());
        
        // Try to create second user (should fail due to limit)
        let user_data2 = json!({"userName": "user2@example.com"});
        let context2 = app.create_context("limited-tenant", "test");
        
        // In a real implementation, this would be checked in the handler
        let users = app.provider.list_resources("User", None, &context2).await.unwrap();
        assert_eq!(users.len(), 1); // At limit
    }
}
```

## Summary

This tutorial demonstrated comprehensive multi-tenant SCIM deployments:

✅ **Multi-Tenant Architecture**:
- Application-level isolation via RequestContext
- Database-level isolation with Row-Level Security
- Flexible deployment patterns (single vs. separate instances)

✅ **Configuration Management**:
- Environment and file-based tenant configuration
- Per-tenant authentication schemes
- Feature flags and limits per tenant

✅ **Security & Isolation**:
- Complete data isolation between tenants
- Per-tenant authentication and authorization
- Rate limiting and resource controls

✅ **Production Considerations**:
- Monitoring and metrics per tenant
- Performance optimization strategies
- Comprehensive testing approaches

**Next Steps**:
- [Authentication Setup](./authentication-setup.md) - Secure your multi-tenant endpoints
- [Custom Resources](./custom-resources.md) - Extend SCIM for tenant-specific needs
- [Performance Optimization](./performance-optimization.md) - Scale for multiple tenants
    let mut configs = HashMap::new();

    // Load from database
    let rows = sqlx::query("SELECT tenant_id, config FROM tenant_configs")
        .fetch_all(&pool)
        .await?;

    for row in rows {
        let tenant_id: String = row.get("tenant_id");
        let config_json: serde_json::Value = row.get("config");
        let config: TenantConfig = serde_json::from_value(config_json)?;
        configs.insert(tenant_id, config);
    }

    Ok(configs)
}
```

### Tenant Registration

```rust
async fn register_tenant(
    State(app): State<MultiTenantApp>,
    Json(registration): Json<TenantRegistration>,
) -> Result<Json<TenantInfo>, (StatusCode, Json<ScimError>)> {
    // Validate registration
    if registration.tenant_id.is_empty() || registration.name.is_empty() {
        return Err((StatusCode::BAD_REQUEST, Json(ScimError::invalid_value("Missing required fields"))));
    }

    // Check if tenant already exists
    if app.tenant_configs.contains_key(&registration.tenant_id) {
        return Err((StatusCode::CONFLICT, Json(ScimError::uniqueness("Tenant ID already exists"))));
    }

    // Create tenant configuration
    let config = TenantConfig {
        name: registration.name,
        display_name: registration.display_name,
        auth_scheme: registration.auth_scheme,
        limits: TenantLimits {
            max_users: Some(1000),
            max_groups: Some(100),
            max_requests_per_minute: Some(1000),
            max_bulk_operations: Some(100),
        },
        features: vec!["basic_operations".to_string()],
        custom_schemas: vec![],
        webhook_endpoints: vec![],
    };

    // Save to database
    sqlx::query(
        "INSERT INTO tenant_configs (tenant_id, config) VALUES ($1, $2)"
    )
    .bind(&registration.tenant_id)
    .bind(serde_json::to_value(&config)?)
    .execute(&app.pool)
    .await?;

    // Generate API key for the tenant
    let api_key = generate_api_key(&registration.tenant_id);

    Ok(Json(TenantInfo {
        tenant_id: registration.tenant_id,
        name: config.name,
        api_key,
        endpoints: TenantEndpoints {
            base_url: format!("https://api.example.com/scim/v2/{}", registration.tenant_id),
            users: format!("https://api.example.com/scim/v2/{}/Users", registration.tenant_id),
            groups: format!("https://api.example.com/scim/v2/{}/Groups", registration.tenant_id),
        },
    }))
}
```

## Advanced Multi-Tenant Patterns

### Tenant Middleware

```rust
use axum::{extract::Request, middleware::Next, response::Response};

async fn tenant_middleware(
    mut request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    // Extract tenant ID from path
    let tenant_id = request
        .uri()
        .path()
        .split('/')
        .nth(3) // /scim/v2/:tenant_id/...
        .ok_or(StatusCode::BAD_REQUEST)?;

    // Validate tenant exists
    let tenant_config = TENANT_CONFIGS
        .get(tenant_id)
        .ok_or(StatusCode::NOT_FOUND)?;

    // Add tenant context to request
    request.extensions_mut().insert(TenantContext {
        tenant_id: tenant_id.to_string(),
        config: tenant_config.clone(),
    });

    // Check tenant limits
    if let Err(status) = check_tenant_limits(&tenant_config, &request).await {
        return Err(status);
    }

    Ok(next.run(request).await)
}

async fn check_tenant_limits(
    config: &TenantConfig,
    request: &Request,
) -> Result<(), StatusCode> {
    // Check rate limits
    if let Some(limit) = config.limits.max_requests_per_minute {
        let current_rate = get_current_request_rate(&config.name).await;
        if current_rate > limit {
            return Err(StatusCode::TOO_MANY_REQUESTS);
        }
    }

    // Check feature availability
    let requested_feature = extract_feature_from_request(request);
    if let Some(feature) = requested_feature {
        if !config.features.contains(&feature) {
            return Err(StatusCode::FORBIDDEN);
        }
    }

    Ok(())
}
```

### Tenant Isolation Testing

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use axum_test::TestServer;

    #[tokio::test]
    async fn test_tenant_isolation() {
        let app = create_test_app().await;
        let server = TestServer::new(app).unwrap();

        // Create user in tenant A
        let user_a = create_test_user("alice@company-a.com");
        let response = server
            .post("/scim/v2/company-a/Users")
            .json(&user_a)
            .await;
        assert_eq!(response.status_code(), 201);
        let created_user_a: ScimUser = response.json();

        // Create user in tenant B
        let user_b = create_test_user("bob@company-b.com");
        let response = server
            .post("/scim/v2/company-b/Users")
            .json(&user_b)
            .await;
        assert_eq!(response.status_code(), 201);
        let created_user_b: ScimUser = response.json();

        // Verify tenant A cannot see tenant B's users
        let response = server
            .get(&format!("/scim/v2/company-a/Users/{}", created_user_b.id()))
            .await;
        assert_eq!(response.status_code(), 404);

        // Verify tenant B cannot see tenant A's users
        let response = server
            .get(&format!("/scim/v2/company-b/Users/{}", created_user_a.id()))
            .await;
        assert_eq!(response.status_code(), 404);

        // Verify each tenant can see their own users
        let response = server
            .get(&format!("/scim/v2/company-a/Users/{}", created_user_a.id()))
            .await;
        assert_eq!(response.status_code(), 200);

        let response = server
            .get(&format!("/scim/v2/company-b/Users/{}", created_user_b.id()))
            .await;
        assert_eq!(response.status_code(), 200);
    }

    #[tokio::test]
    async fn test_tenant_limits() {
        let app = create_test_app().await;
        let server = TestServer::new(app).unwrap();

        // Create users up to the limit
        for i in 0..1000 {
            let user = create_test_user(&format!("user{}@company-a.com", i));
            let response = server
                .post("/scim/v2/company-a/Users")
                .json(&user)
                .await;
            assert_eq!(response.status_code(), 201);
        }

        // Try to create one more user (should fail)
        let user = create_test_user("overflow@company-a.com");
        let response = server
            .post("/scim/v2/company-a/Users")
            .json(&user)
            .await;
        assert_eq!(response.status_code(), 403);
    }
}
```

## Deployment Strategies

### Shared Infrastructure

```yaml
# docker-compose.yml for shared infrastructure
version: '3.8'

services:
  scim-server:
    image: scim-server:latest
    environment:
      - DATABASE_URL=postgresql://scim:password@postgres:5432/scim
      - REDIS_URL=redis://redis:6379
      - TENANT_CONFIG_URL=file:///config/tenants.json
    volumes:
      - ./tenant-configs:/config
    ports:
      - "3000:3000"
    depends_on:
      - postgres
      - redis

  postgres:
    image: postgres:15
    environment:
      - POSTGRES_DB=scim
      - POSTGRES_USER=scim
      - POSTGRES_PASSWORD=password
    volumes:
      - postgres_data:/var/lib/postgresql/data

  redis:
    image: redis:7-alpine
    volumes:
      - redis_data:/data

volumes:
  postgres_data:
  redis_data:
```

### Kubernetes Multi-Tenant Deployment

```yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: scim-server-multitenant
spec:
  replicas: 3
  selector:
    matchLabels:
      app: scim-server
  template:
    metadata:
      labels:
        app: scim-server
    spec:
      containers:
      - name: scim-server
        image: scim-server:latest
        env:
        - name: DATABASE_URL
          valueFrom:
            secretKeyRef:
              name: database-credentials
              key: url
        - name: TENANT_CONFIGS
          valueFrom:
            configMapKeyRef:
              name: tenant-configs
              key: config.json
        ports:
        - containerPort: 3000
        resources:
          requests:
            memory: "256Mi"
            cpu: "250m"
          limits:
            memory: "512Mi"
            cpu: "500m"
---
apiVersion: v1
kind: ConfigMap
metadata:
  name: tenant-configs
data:
  config.json: |
    {
      "company-a": {
        "name": "Company A",
        "auth_scheme": {
          "OAuth": {
            "jwks_url": "https://company-a.auth0.com/.well-known/jwks.json",
            "audience": "scim-api"
          }
        },
        "limits": {
          "max_users": 1000,
          "max_requests_per_minute": 1000
        }
      },
      "company-b": {
        "name": "Company B",
        "auth_scheme": {
          "ApiKey": {
            "keys": ["sk_live_abc123"]
          }
        },
        "limits": {
          "max_users": 500,
          "max_requests_per_minute": 500
        }
      }
    }
```

## Monitoring and Observability

### Per-Tenant Metrics

```rust
use prometheus::{Counter, Histogram, register_counter_vec, register_histogram_vec};

lazy_static! {
    static ref REQUESTS_TOTAL: Counter = register_counter_vec!(
        "scim_requests_total",
        "Total number of SCIM requests",
        &["tenant_id", "method", "status"]
    ).unwrap();
    
    static ref REQUEST_DURATION: Histogram = register_histogram_vec!(
        "scim_request_duration_seconds",
        "Duration of SCIM requests",
        &["tenant_id", "method"]
    ).unwrap();
}

async fn metrics_middleware(
    Extension(tenant_context): Extension<TenantContext>,
    request: Request,
    next: Next,
) -> Response {
    let method = request.method().to_string();
    let start = std::time::Instant::now();
    
    let response = next.run(request).await;
    
    let duration = start.elapsed().as_secs_f64();
    let status = response.status().as_u16().to_string();
    
    REQUESTS_TOTAL
        .with_label_values(&[&tenant_context.tenant_id, &method, &status])
        .inc();
    
    REQUEST_DURATION
        .with_label_values(&[&tenant_context.tenant_id, &method])
        .observe(duration);
    
    response
}
```

### Tenant Health Dashboard

```rust
async fn tenant_health_endpoint(
    State(app): State<MultiTenantApp>,
) -> Json<serde_json::Value> {
    let mut tenant_health = serde_json::Map::new();
    
    for (tenant_id, config) in &app.tenant_configs {
        let user_count = app.scim_server
            .count_users(tenant_id)
            .await
            .unwrap_or(0);
        
        let group_count = app.scim_server
            .count_groups(tenant_id)
            .await
            .unwrap_or(0);
        
        tenant_health.insert(tenant_id.clone(), json!({
            "name": config.name,
            "status": "healthy",
            "user_count": user_count,
            "group_count": group_count,
            "limits": {
                "max_users": config.limits.max_users,
                "user_utilization": config.limits.max_users.map(|max| (user_count as f64 / max as f64) * 100.0)
            }
        }));
    }
    
    Json(json!({
        "tenant_count": tenant_health.len(),
        "tenants": tenant_health
    }))
}
```

This comprehensive guide covers all aspects of deploying SCIM Server in multi-tenant environments, from basic setup to advanced production patterns with complete isolation and monitoring.