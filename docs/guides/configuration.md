# Configuration Guide

This guide covers how to configure the SCIM Server for different deployment scenarios, including single-tenant and multi-tenant setups, provider configuration, and server customization options.

## Table of Contents

- [Basic Configuration](#basic-configuration)
- [Server Setup](#server-setup)
- [Multi-Tenant Configuration](#multi-tenant-configuration)
- [Provider Configuration](#provider-configuration)
- [Schema Configuration](#schema-configuration)
- [Logging Configuration](#logging-configuration)
- [Security Configuration](#security-configuration)
- [Performance Tuning](#performance-tuning)

## Basic Configuration

### Server Configuration Structure

The SCIM server uses a structured configuration approach with sensible defaults:

```rust
use scim_server::config::{ServerConfig, ServerBuilder};
use scim_server::providers::InMemoryProvider;

// Basic server configuration
let config = ServerConfig::builder()
    .host("0.0.0.0")
    .port(8080)
    .base_url("https://api.example.com/scim/v2")
    .provider(InMemoryProvider::new())
    .build()?;
```

### Environment-Based Configuration

Configure the server using environment variables:

```rust
use std::env;

let config = ServerConfig::builder()
    .host(env::var("SCIM_HOST").unwrap_or_else(|_| "localhost".to_string()))
    .port(env::var("SCIM_PORT")
        .unwrap_or_else(|_| "8080".to_string())
        .parse()
        .unwrap_or(8080))
    .base_url(env::var("SCIM_BASE_URL")
        .unwrap_or_else(|_| "http://localhost:8080/scim/v2".to_string()))
    .build()?;
```

### Configuration File Support

Load configuration from TOML files:

```toml
# scim-config.toml
[server]
host = "0.0.0.0"
port = 8080
base_url = "https://api.example.com/scim/v2"
max_connections = 1000
request_timeout_seconds = 30

[logging]
level = "info"
format = "json"
enable_request_logging = true

[security]
require_tls = true
cors_enabled = true
cors_origins = ["https://admin.example.com"]
```

```rust
use scim_server::config::ServerConfig;

// Load from configuration file
let config = ServerConfig::from_file("scim-config.toml").await?;
```

## Server Setup

### Basic Server Setup

```rust
use scim_server::{ScimServer, ServerConfig};
use scim_server::providers::InMemoryProvider;

#[tokio::main]
async fn main() -> Result<()> {
    // Create provider
    let provider = InMemoryProvider::new();
    
    // Configure server
    let config = ServerConfig::builder()
        .host("localhost")
        .port(8080)
        .provider(provider)
        .build()?;
    
    // Start server
    let server = ScimServer::new(config);
    server.run().await?;
    
    Ok(())
}
```

### Advanced Server Configuration

```rust
use scim_server::{ScimServer, ServerConfig};
use scim_server::middleware::{CorsMiddleware, AuthMiddleware};
use scim_server::providers::DatabaseProvider;
use tokio::time::Duration;

#[tokio::main]
async fn main() -> Result<()> {
    let config = ServerConfig::builder()
        .host("0.0.0.0")
        .port(8443)
        .base_url("https://scim.mycompany.com/v2")
        .provider(DatabaseProvider::new(&database_url).await?)
        .request_timeout(Duration::from_secs(30))
        .max_payload_size(1024 * 1024) // 1MB
        .enable_cors(true)
        .cors_origins(vec!["https://admin.mycompany.com"])
        .build()?;
    
    let server = ScimServer::new(config);
    server.run().await?;
    
    Ok(())
}
```

## Multi-Tenant Configuration

### Static Tenant Resolution

Configure tenants statically at startup:

```rust
use scim_server::multi_tenant::{
    StaticTenantResolver, TenantId, TenantContext, ScimConfig
};
use scim_server::providers::InMemoryProvider;

async fn setup_multi_tenant_server() -> Result<()> {
    let mut resolver = StaticTenantResolver::new();
    
    // Configure tenant A
    let tenant_a_config = ScimConfig::builder()
        .resource_types(vec!["User", "Group"])
        .schemas(vec!["urn:ietf:params:scim:schemas:core:2.0:User"])
        .provider(InMemoryProvider::new())
        .build()?;
    
    resolver.add_tenant(
        TenantId::new("tenant-a")?,
        TenantContext::new(tenant_a_config)
    )?;
    
    // Configure tenant B with different settings
    let tenant_b_config = ScimConfig::builder()
        .resource_types(vec!["User", "Group", "CustomResource"])
        .schemas(vec![
            "urn:ietf:params:scim:schemas:core:2.0:User",
            "urn:mycompany:scim:schemas:CustomResource"
        ])
        .provider(DatabaseProvider::new(&tenant_b_db_url).await?)
        .build()?;
    
    resolver.add_tenant(
        TenantId::new("tenant-b")?,
        TenantContext::new(tenant_b_config)
    )?;
    
    Ok(())
}
```

### Dynamic Tenant Resolution

Implement custom tenant resolution logic:

```rust
use scim_server::multi_tenant::{TenantResolver, TenantContext};
use async_trait::async_trait;

struct DatabaseTenantResolver {
    database: DatabasePool,
}

#[async_trait]
impl TenantResolver for DatabaseTenantResolver {
    async fn resolve_tenant(&self, hint: &str) -> Result<TenantContext> {
        // Custom logic to resolve tenant from database
        let tenant_config = self.database
            .get_tenant_config(hint)
            .await?;
        
        TenantContext::from_config(tenant_config)
    }
    
    async fn list_tenants(&self) -> Result<Vec<TenantId>> {
        self.database.list_tenant_ids().await
    }
}
```

### Per-Tenant Configuration

Configure different settings per tenant:

```rust
// Tenant with custom schema validation
let strict_tenant = ScimConfig::builder()
    .strict_validation(true)
    .custom_schemas(load_custom_schemas()?)
    .provider(DatabaseProvider::with_encryption(&encrypted_db_url).await?)
    .build()?;

// Tenant with relaxed validation for development
let dev_tenant = ScimConfig::builder()
    .strict_validation(false)
    .allow_unknown_attributes(true)
    .provider(InMemoryProvider::new())
    .build()?;
```

## Provider Configuration

### In-Memory Provider

Suitable for development and testing:

```rust
use scim_server::providers::InMemoryProvider;

let provider = InMemoryProvider::builder()
    .initial_capacity(1000)
    .enable_persistence(false)
    .build();
```

### Database Provider Configuration

Configure database connections and settings:

```rust
use scim_server::providers::DatabaseProvider;

let provider = DatabaseProvider::builder()
    .connection_string("postgresql://user:pass@localhost/scim")
    .max_connections(20)
    .connection_timeout(Duration::from_secs(10))
    .enable_connection_pooling(true)
    .enable_prepared_statements(true)
    .build()
    .await?;
```

### Custom Provider Configuration

Configure your custom provider:

```rust
struct CustomProviderConfig {
    api_endpoint: String,
    api_key: String,
    timeout: Duration,
    retry_attempts: u32,
}

impl CustomProviderConfig {
    pub fn from_env() -> Result<Self> {
        Ok(Self {
            api_endpoint: env::var("CUSTOM_API_ENDPOINT")?,
            api_key: env::var("CUSTOM_API_KEY")?,
            timeout: Duration::from_secs(
                env::var("CUSTOM_TIMEOUT")?.parse()?
            ),
            retry_attempts: env::var("CUSTOM_RETRIES")?.parse()?,
        })
    }
}
```

## Schema Configuration

### Built-in Schemas

Configure which built-in schemas to support:

```rust
use scim_server::schema::{SchemaRegistry, CoreSchemas};

let registry = SchemaRegistry::builder()
    .add_core_user_schema()
    .add_core_group_schema()
    .add_enterprise_user_extension()
    .build()?;
```

### Custom Schema Registration

Register custom schemas for your domain:

```rust
use scim_server::schema::{Schema, SchemaBuilder, AttributeDefinition};

// Define custom schema
let custom_schema = SchemaBuilder::new()
    .id("urn:mycompany:scim:schemas:Employee")
    .name("Employee")
    .description("Custom employee attributes")
    .add_attribute(AttributeDefinition::builder()
        .name("employeeNumber")
        .type_("string")
        .required(true)
        .unique(true)
        .build()?)
    .add_attribute(AttributeDefinition::builder()
        .name("department")
        .type_("string")
        .required(false)
        .build()?)
    .build()?;

// Register with server
let config = ServerConfig::builder()
    .custom_schema(custom_schema)
    .build()?;
```

## Logging Configuration

### Basic Logging Setup

```rust
use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;

// Initialize logging
let subscriber = FmtSubscriber::builder()
    .with_max_level(Level::INFO)
    .with_target(true)
    .with_thread_ids(true)
    .finish();

tracing::subscriber::set_global_default(subscriber)?;

info!("SCIM server starting up");
```

### Structured Logging

Configure JSON logging for production:

```rust
use tracing_subscriber::{EnvFilter, fmt, prelude::*};

tracing_subscriber::registry()
    .with(EnvFilter::from_default_env())
    .with(fmt::layer()
        .json()
        .with_current_span(false)
        .with_span_list(true))
    .init();
```

### Request Logging

Enable detailed request/response logging:

```rust
let config = ServerConfig::builder()
    .enable_request_logging(true)
    .log_request_bodies(false)  // Don't log sensitive data
    .log_response_bodies(false)
    .log_performance_metrics(true)
    .build()?;
```

## Security Configuration

### TLS Configuration

```rust
use scim_server::security::TlsConfig;

let tls_config = TlsConfig::builder()
    .cert_file("path/to/cert.pem")
    .key_file("path/to/key.pem")
    .require_client_cert(false)
    .build()?;

let config = ServerConfig::builder()
    .tls(tls_config)
    .build()?;
```

### Authentication Configuration

```rust
use scim_server::auth::{AuthConfig, BearerTokenAuth};

let auth_config = AuthConfig::builder()
    .bearer_token_validation(BearerTokenAuth::new(
        "https://auth.example.com/.well-known/jwks.json"
    ))
    .require_authentication(true)
    .build()?;

let config = ServerConfig::builder()
    .authentication(auth_config)
    .build()?;
```

### CORS Configuration

```rust
let config = ServerConfig::builder()
    .enable_cors(true)
    .cors_origins(vec![
        "https://admin.example.com",
        "https://app.example.com"
    ])
    .cors_methods(vec!["GET", "POST", "PUT", "PATCH", "DELETE"])
    .cors_headers(vec!["Authorization", "Content-Type"])
    .cors_max_age(Duration::from_secs(3600))
    .build()?;
```

## Performance Tuning

### Connection Limits

```rust
let config = ServerConfig::builder()
    .max_concurrent_connections(1000)
    .connection_timeout(Duration::from_secs(30))
    .keep_alive_timeout(Duration::from_secs(60))
    .build()?;
```

### Caching Configuration

```rust
use scim_server::cache::{CacheConfig, CacheBackend};

let cache_config = CacheConfig::builder()
    .backend(CacheBackend::Redis("redis://localhost:6379"))
    .default_ttl(Duration::from_secs(300))
    .max_entries(10000)
    .enable_resource_caching(true)
    .enable_schema_caching(true)
    .build()?;

let config = ServerConfig::builder()
    .cache(cache_config)
    .build()?;
```

### Resource Limits

```rust
let config = ServerConfig::builder()
    .max_request_size(1024 * 1024)     // 1MB
    .max_response_size(5 * 1024 * 1024) // 5MB
    .max_results_per_page(100)
    .default_results_per_page(20)
    .build()?;
```

## Configuration Examples

### Development Configuration

```rust
// Development setup with relaxed settings
let dev_config = ServerConfig::builder()
    .host("localhost")
    .port(8080)
    .provider(InMemoryProvider::new())
    .enable_cors(true)
    .cors_origins(vec!["http://localhost:3000"])
    .log_level("debug")
    .strict_validation(false)
    .build()?;
```

### Production Configuration

```rust
// Production setup with security and performance optimization
let prod_config = ServerConfig::builder()
    .host("0.0.0.0")
    .port(443)
    .base_url("https://scim.mycompany.com/v2")
    .provider(DatabaseProvider::new(&database_url).await?)
    .tls(load_tls_config()?)
    .authentication(load_auth_config()?)
    .enable_cors(true)
    .cors_origins(load_allowed_origins())
    .max_concurrent_connections(2000)
    .request_timeout(Duration::from_secs(30))
    .log_level("info")
    .strict_validation(true)
    .enable_metrics(true)
    .build()?;
```

### Multi-Tenant Production Configuration

```rust
use scim_server::multi_tenant::StaticTenantResolver;

async fn setup_production_multi_tenant() -> Result<ServerConfig> {
    let mut resolver = StaticTenantResolver::new();
    
    // Configure each tenant
    for tenant_config in load_tenant_configs().await? {
        let tenant_id = TenantId::new(&tenant_config.id)?;
        let provider = DatabaseProvider::new(&tenant_config.database_url).await?;
        
        let scim_config = ScimConfig::builder()
            .resource_types(tenant_config.resource_types)
            .schemas(tenant_config.schemas)
            .provider(provider)
            .strict_validation(tenant_config.strict_mode)
            .build()?;
        
        resolver.add_tenant(tenant_id, TenantContext::new(scim_config))?;
    }
    
    ServerConfig::builder()
        .host("0.0.0.0")
        .port(443)
        .tenant_resolver(resolver)
        .tls(load_tls_config()?)
        .authentication(load_auth_config()?)
        .build()
}
```

## Configuration Validation

### Validate Configuration at Startup

```rust
use scim_server::config::validation::ConfigValidator;

async fn validate_and_start_server(config: ServerConfig) -> Result<()> {
    // Validate configuration
    ConfigValidator::new()
        .validate_connectivity(&config).await?
        .validate_schemas(&config).await?
        .validate_providers(&config).await?;
    
    // Start server with validated config
    let server = ScimServer::new(config);
    server.run().await
}
```

### Configuration Health Checks

```rust
impl ServerConfig {
    pub async fn health_check(&self) -> Result<HealthStatus> {
        let mut status = HealthStatus::new();
        
        // Check provider connectivity
        status.add_check("provider", self.provider.health_check().await?);
        
        // Check tenant resolution
        if let Some(resolver) = &self.tenant_resolver {
            status.add_check("tenants", resolver.health_check().await?);
        }
        
        // Check external dependencies
        if let Some(auth) = &self.authentication {
            status.add_check("auth", auth.health_check().await?);
        }
        
        Ok(status)
    }
}
```

## Best Practices

### Configuration Security

1. **Never hardcode secrets**:
```rust
// Good: Load from environment or secure storage
let api_key = env::var("API_KEY")?;

// Bad: Hardcoded in source
// let api_key = "secret-key-123";
```

2. **Use secure defaults**:
```rust
let config = ServerConfig::builder()
    .require_tls(true)              // Always require TLS in production
    .strict_validation(true)        // Enable strict SCIM validation
    .log_sensitive_data(false)      // Don't log passwords, tokens
    .build()?;
```

3. **Validate configuration early**:
```rust
// Validate at startup, not at runtime
config.validate()?;
```

### Configuration Organization

1. **Separate environments**:
```
configs/
├── development.toml
├── staging.toml
├── production.toml
└── test.toml
```

2. **Use configuration profiles**:
```rust
let config_file = match env::var("SCIM_ENV")?.as_str() {
    "development" => "configs/development.toml",
    "staging" => "configs/staging.toml",
    "production" => "configs/production.toml",
    _ => return Err("Invalid SCIM_ENV value".into()),
};
```

### Monitoring Configuration

```rust
let config = ServerConfig::builder()
    .enable_metrics(true)
    .metrics_endpoint("/metrics")
    .health_check_endpoint("/health")
    .prometheus_registry(registry)
    .build()?;
```

## Troubleshooting Configuration

### Common Configuration Issues

1. **Port binding failures**:
```rust
// Check if port is available
if !is_port_available(config.port()) {
    return Err(format!("Port {} is already in use", config.port()).into());
}
```

2. **Provider connection issues**:
```rust
// Test provider connectivity during configuration
config.provider().health_check().await?;
```

3. **Schema loading failures**:
```rust
// Validate all schemas are loadable
for schema_uri in config.schemas() {
    SchemaRegistry::load_schema(schema_uri)?;
}
```

### Configuration Debugging

Enable detailed configuration logging:

```rust
use tracing::debug;

debug!("Server configuration: {:#?}", config);
debug!("Loaded {} tenant configurations", resolver.tenant_count());
debug!("Provider type: {}", config.provider().type_name());
```

## Next Steps

- **[Quick Start Guide](quick-start.md)** - Set up your first server
- **[Architecture Guide](architecture.md)** - Understand the system design
- **[API Reference](../api/README.md)** - Explore the complete API
- **[Examples](../examples/README.md)** - See complete configuration examples
- **[Security Guide](../reference/security.md)** - Learn about security best practices