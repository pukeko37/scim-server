# Configuration Options

This reference documents all configuration options available in the SCIM Server library, including server settings, provider configurations, and runtime parameters.

## Configuration Overview

The SCIM Server library supports multiple configuration approaches:

1. **Builder Pattern** - Programmatic configuration with type safety
2. **Environment Variables** - Runtime configuration for deployment
3. **Configuration Files** - TOML/JSON files for complex setups
4. **Hybrid Approach** - Combining multiple configuration sources

## Server Configuration

### Basic Server Setup

```rust
use scim_server::{ScimServer, ServerConfig};

let config = ServerConfig::builder()
    .bind_address("0.0.0.0:8080")
    .max_connections(1000)
    .request_timeout_ms(30000)
    .enable_cors(true)
    .build()?;

let server = ScimServer::with_config(storage, config).await?;
```

### ServerConfig Options

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `bind_address` | `String` | `"127.0.0.1:3000"` | Address and port to bind the server |
| `max_connections` | `u32` | `512` | Maximum concurrent connections |
| `request_timeout_ms` | `u64` | `30000` | Request timeout in milliseconds |
| `keep_alive_timeout_ms` | `u64` | `60000` | Keep-alive connection timeout |
| `max_request_size` | `usize` | `1048576` | Maximum request body size (1MB) |
| `enable_cors` | `bool` | `false` | Enable CORS headers |
| `cors_origins` | `Vec<String>` | `["*"]` | Allowed CORS origins |
| `enable_compression` | `bool` | `true` | Enable gzip compression |
| `thread_pool_size` | `Option<usize>` | `None` | Custom thread pool size |

### Environment Variables

All server options can be configured via environment variables:

```bash
export SCIM_BIND_ADDRESS="0.0.0.0:8080"
export SCIM_MAX_CONNECTIONS=2000
export SCIM_REQUEST_TIMEOUT_MS=45000
export SCIM_ENABLE_CORS=true
export SCIM_CORS_ORIGINS="https://app.example.com,https://admin.example.com"
export SCIM_MAX_REQUEST_SIZE=2097152
```

### Configuration File

Create a `scim-config.toml` file:

```toml
[server]
bind_address = "0.0.0.0:8080"
max_connections = 1000
request_timeout_ms = 30000
keep_alive_timeout_ms = 60000
max_request_size = 1048576
enable_cors = true
cors_origins = ["https://app.example.com"]
enable_compression = true
thread_pool_size = 8

[logging]
level = "info"
format = "json"
file_path = "/var/log/scim-server.log"
max_file_size = "100MB"
max_files = 10

[security]
require_https = true
allowed_auth_methods = ["bearer", "basic"]
token_validation_endpoint = "https://auth.example.com/validate"
```

Load the configuration:

```rust
use scim_server::config::load_config;

let config = load_config("scim-config.toml")?;
let server = ScimServer::with_config(storage, config).await?;
```

## Storage Provider Configuration

### In-Memory Storage

```rust
use scim_server::storage::{InMemoryStorage, InMemoryConfig};

let config = InMemoryConfig::builder()
    .initial_capacity(10000)
    .enable_persistence(true)
    .persistence_file("/data/scim-backup.json")
    .backup_interval_ms(300000) // 5 minutes
    .build()?;

let storage = InMemoryStorage::with_config(config)?;
```

#### InMemoryConfig Options

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `initial_capacity` | `usize` | `1000` | Initial HashMap capacity |
| `enable_persistence` | `bool` | `false` | Enable periodic backups |
| `persistence_file` | `Option<PathBuf>` | `None` | Backup file path |
| `backup_interval_ms` | `u64` | `300000` | Backup interval (5 min) |
| `compression` | `bool` | `true` | Compress backup files |
| `max_memory_mb` | `Option<usize>` | `None` | Memory usage limit |

### Database Storage

```rust
use scim_server::storage::{DatabaseStorage, DatabaseConfig};

let config = DatabaseConfig::builder()
    .connection_url("postgresql://user:pass@localhost/scim")
    .max_connections(20)
    .connection_timeout_ms(5000)
    .idle_timeout_ms(600000)
    .enable_ssl(true)
    .migration_auto_run(true)
    .build()?;

let storage = DatabaseStorage::with_config(config).await?;
```

#### DatabaseConfig Options

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `connection_url` | `String` | Required | Database connection URL |
| `max_connections` | `u32` | `10` | Connection pool size |
| `min_connections` | `u32` | `1` | Minimum pool connections |
| `connection_timeout_ms` | `u64` | `30000` | Connection timeout |
| `idle_timeout_ms` | `u64` | `600000` | Idle connection timeout |
| `max_lifetime_ms` | `Option<u64>` | `None` | Max connection lifetime |
| `enable_ssl` | `bool` | `false` | Require SSL connections |
| `ssl_ca_file` | `Option<PathBuf>` | `None` | SSL CA certificate file |
| `migration_auto_run` | `bool` | `true` | Auto-run migrations |
| `query_timeout_ms` | `u64` | `30000` | Query execution timeout |
| `enable_logging` | `bool` | `false` | Log SQL queries |

### Custom Storage Provider

```rust
use scim_server::storage::{StorageProvider, ProviderConfig};

#[derive(Debug, Clone)]
pub struct CustomConfig {
    pub api_endpoint: String,
    pub api_key: String,
    pub timeout_ms: u64,
    pub retry_attempts: u32,
}

impl ProviderConfig for CustomConfig {
    fn validate(&self) -> Result<(), ConfigError> {
        if self.api_endpoint.is_empty() {
            return Err(ConfigError::MissingRequired("api_endpoint"));
        }
        if self.timeout_ms == 0 {
            return Err(ConfigError::InvalidValue("timeout_ms must be > 0"));
        }
        Ok(())
    }
}
```

## Multi-Tenancy Configuration

### Tenant Settings

```rust
use scim_server::{MultiTenantConfig, TenantConfig};

let tenant_config = TenantConfig::builder()
    .tenant_id("acme-corp")
    .display_name("ACME Corporation")
    .max_users(5000)
    .max_groups(500)
    .storage_isolation_level("strict")
    .custom_schemas(vec!["urn:acme:schemas:employee"])
    .rate_limit_per_minute(1000)
    .enable_bulk_operations(true)
    .data_retention_days(2555) // 7 years
    .build()?;

let multi_tenant_config = MultiTenantConfig::builder()
    .default_tenant_config(tenant_config)
    .tenant_resolver("header") // "header", "subdomain", "path"
    .tenant_header_name("X-Tenant-ID")
    .enable_tenant_creation(false)
    .build()?;
```

#### TenantConfig Options

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `tenant_id` | `String` | Required | Unique tenant identifier |
| `display_name` | `String` | `tenant_id` | Human-readable name |
| `max_users` | `Option<u32>` | `None` | Maximum user limit |
| `max_groups` | `Option<u32>` | `None` | Maximum group limit |
| `storage_isolation_level` | `String` | `"strict"` | `"strict"`, `"logical"`, `"none"` |
| `custom_schemas` | `Vec<String>` | `[]` | Additional schema URIs |
| `rate_limit_per_minute` | `Option<u32>` | `None` | Tenant-specific rate limit |
| `enable_pagination` | `bool` | `true` | Enable paginated list responses |
| `max_page_size` | `u32` | `1000` | Maximum items per page in list requests |
| `data_retention_days` | `Option<u32>` | `None` | Data retention policy |
| `encryption_key_id` | `Option<String>` | `None` | Tenant-specific encryption |

### Tenant Resolution

```rust
// Header-based tenant resolution
let config = MultiTenantConfig::builder()
    .tenant_resolver("header")
    .tenant_header_name("X-Tenant-ID")
    .build()?;

// Subdomain-based tenant resolution  
let config = MultiTenantConfig::builder()
    .tenant_resolver("subdomain")
    .subdomain_pattern("{tenant}.api.example.com")
    .build()?;

// Path-based tenant resolution
let config = MultiTenantConfig::builder()
    .tenant_resolver("path")
    .path_pattern("/tenants/{tenant}/scim")
    .build()?;
```

## Security Configuration

### Authentication

```rust
use scim_server::auth::{AuthConfig, AuthMethod};

let auth_config = AuthConfig::builder()
    .enabled_methods(vec![AuthMethod::Bearer, AuthMethod::Basic])
    .bearer_token_validation("jwt") // "jwt", "opaque", "custom"
    .jwt_issuer("https://auth.example.com")
    .jwt_audience("scim-api")
    .jwt_public_key_url("https://auth.example.com/.well-known/jwks.json")
    .basic_auth_realm("SCIM API")
    .token_cache_ttl_ms(300000) // 5 minutes
    .enable_token_introspection(true)
    .introspection_endpoint("https://auth.example.com/introspect")
    .build()?;
```

#### AuthConfig Options

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `enabled_methods` | `Vec<AuthMethod>` | `[Bearer]` | Allowed auth methods |
| `bearer_token_validation` | `String` | `"opaque"` | Token validation method |
| `jwt_issuer` | `Option<String>` | `None` | JWT issuer for validation |
| `jwt_audience` | `Option<String>` | `None` | Expected JWT audience |
| `jwt_public_key_url` | `Option<String>` | `None` | JWK Set URL for JWT validation |
| `jwt_algorithm` | `String` | `"RS256"` | JWT signing algorithm |
| `basic_auth_realm` | `String` | `"SCIM"` | Basic auth realm |
| `token_cache_ttl_ms` | `u64` | `300000` | Token validation cache TTL |
| `enable_token_introspection` | `bool` | `false` | Use OAuth2 token introspection |
| `introspection_endpoint` | `Option<String>` | `None` | Token introspection endpoint |
| `client_id` | `Option<String>` | `None` | OAuth2 client ID |
| `client_secret` | `Option<String>` | `None` | OAuth2 client secret |

### TLS Configuration

```rust
use scim_server::tls::{TlsConfig, TlsVersion};

let tls_config = TlsConfig::builder()
    .certificate_file("/etc/ssl/certs/server.crt")
    .private_key_file("/etc/ssl/private/server.key")
    .ca_certificate_file("/etc/ssl/certs/ca.crt")
    .min_tls_version(TlsVersion::V1_2)
    .require_client_cert(false)
    .cipher_suites(vec![
        "TLS_AES_256_GCM_SHA384",
        "TLS_CHACHA20_POLY1305_SHA256"
    ])
    .build()?;
```

## Logging Configuration

### Basic Logging Setup

```rust
use scim_server::logging::{LoggingConfig, LogLevel, LogFormat};

let logging_config = LoggingConfig::builder()
    .level(LogLevel::Info)
    .format(LogFormat::Json)
    .enable_console(true)
    .enable_file(true)
    .file_path("/var/log/scim-server.log")
    .max_file_size("100MB")
    .max_files(10)
    .enable_compression(true)
    .fields(vec!["timestamp", "level", "message", "tenant_id", "user_id"])
    .build()?;
```

#### LoggingConfig Options

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `level` | `LogLevel` | `Info` | Minimum log level |
| `format` | `LogFormat` | `Text` | Log output format |
| `enable_console` | `bool` | `true` | Log to console/stdout |
| `enable_file` | `bool` | `false` | Log to files |
| `file_path` | `Option<PathBuf>` | `None` | Log file path |
| `max_file_size` | `String` | `"100MB"` | Maximum log file size |
| `max_files` | `u32` | `10` | Number of rotated files |
| `enable_compression` | `bool` | `true` | Compress rotated logs |
| `fields` | `Vec<String>` | Default set | Fields to include in logs |
| `exclude_paths` | `Vec<String>` | `[]` | Paths to exclude from logging |
| `enable_request_logging` | `bool` | `true` | Log HTTP requests |
| `log_request_body` | `bool` | `false` | Include request body in logs |
| `log_response_body` | `bool` | `false` | Include response body in logs |

### Structured Logging

```rust
use tracing::{info, warn, error};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

// Custom logging setup
tracing_subscriber::registry()
    .with(tracing_subscriber::EnvFilter::new("scim_server=info"))
    .with(tracing_subscriber::fmt::layer()
        .json()
        .with_current_span(false)
        .with_span_list(true))
    .init();

// Usage in code
info!(
    tenant_id = %tenant_id,
    user_id = %user_id,
    operation = "create_user",
    "User created successfully"
);
```

## Performance Configuration

### Caching

```rust
use scim_server::cache::{CacheConfig, CacheBackend};

let cache_config = CacheConfig::builder()
    .backend(CacheBackend::InMemory)
    .max_entries(10000)
    .ttl_ms(300000) // 5 minutes
    .enable_user_cache(true)
    .enable_group_cache(true)
    .enable_schema_cache(true)
    .cache_key_prefix("scim:")
    .enable_compression(true)
    .build()?;
```

#### CacheConfig Options

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `backend` | `CacheBackend` | `InMemory` | Cache storage backend |
| `max_entries` | `usize` | `1000` | Maximum cached entries |
| `ttl_ms` | `u64` | `300000` | Time-to-live in milliseconds |
| `enable_user_cache` | `bool` | `true` | Cache user resources |
| `enable_group_cache` | `bool` | `true` | Cache group resources |
| `enable_schema_cache` | `bool` | `true` | Cache schema definitions |
| `cache_key_prefix` | `String` | `"scim:"` | Prefix for cache keys |
| `enable_compression` | `bool` | `false` | Compress cached values |
| `redis_url` | `Option<String>` | `None` | Redis connection URL |
| `redis_pool_size` | `u32` | `10` | Redis connection pool size |

### Rate Limiting

```rust
use scim_server::rate_limit::{RateLimitConfig, RateLimitAlgorithm};

let rate_limit_config = RateLimitConfig::builder()
    .algorithm(RateLimitAlgorithm::TokenBucket)
    .requests_per_minute(1000)
    .burst_size(100)
    .enable_per_tenant_limits(true)
    .enable_per_user_limits(false)
    .storage_backend("memory") // "memory", "redis"
    .redis_url("redis://localhost:6379")
    .window_size_ms(60000)
    .build()?;
```

## Validation Configuration

### Schema Validation

```rust
use scim_server::validation::{ValidationConfig, ValidationMode};

let validation_config = ValidationConfig::builder()
    .mode(ValidationMode::Strict)
    .enable_custom_validators(true)
    .allow_unknown_attributes(false)
    .require_schemas_field(true)
    .validate_references(true)
    .max_string_length(1000)
    .max_array_size(100)
    .custom_validators_path("./validators")
    .build()?;
```

#### ValidationConfig Options

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `mode` | `ValidationMode` | `Strict` | Validation strictness level |
| `enable_custom_validators` | `bool` | `false` | Enable custom validation logic |
| `allow_unknown_attributes` | `bool` | `false` | Allow undefined attributes |
| `require_schemas_field` | `bool` | `true` | Require schemas field in resources |
| `validate_references` | `bool` | `true` | Validate resource references |
| `max_string_length` | `usize` | `1000` | Maximum string attribute length |
| `max_array_size` | `usize` | `100` | Maximum array size |
| `email_validation_strict` | `bool` | `true` | Strict email format validation |
| `phone_validation_strict` | `bool` | `false` | Strict phone format validation |
| `custom_validators_path` | `Option<PathBuf>` | `None` | Path to custom validator modules |

## Monitoring Configuration

### Metrics

```rust
use scim_server::metrics::{MetricsConfig, MetricsBackend};

let metrics_config = MetricsConfig::builder()
    .backend(MetricsBackend::Prometheus)
    .enable_http_metrics(true)
    .enable_business_metrics(true)
    .endpoint_path("/metrics")
    .collection_interval_ms(15000)
    .histogram_buckets(vec![0.001, 0.01, 0.1, 1.0, 10.0])
    .enable_detailed_labels(true)
    .build()?;
```

### Health Checks

```rust
use scim_server::health::{HealthConfig, HealthCheck};

let health_config = HealthConfig::builder()
    .endpoint_path("/health")
    .enable_readiness_check(true)
    .enable_liveness_check(true)
    .storage_check_timeout_ms(5000)
    .auth_check_timeout_ms(3000)
    .custom_checks(vec![
        HealthCheck::new("database", check_database_health),
        HealthCheck::new("auth_service", check_auth_service),
    ])
    .build()?;
```

## Environment-Specific Configurations

### Development

```toml
[server]
bind_address = "127.0.0.1:3000"
max_connections = 100
enable_cors = true
cors_origins = ["*"]

[logging]
level = "debug"
format = "text"
enable_console = true
enable_file = false
enable_request_logging = true
log_request_body = true
log_response_body = true

[storage.in_memory]
initial_capacity = 100
enable_persistence = false

[validation]
mode = "permissive"
allow_unknown_attributes = true

[auth]
enabled_methods = ["bearer"]
bearer_token_validation = "none" # Skip validation for dev
```

### Production

```toml
[server]
bind_address = "0.0.0.0:8080"
max_connections = 2000
request_timeout_ms = 30000
enable_compression = true
thread_pool_size = 16

[logging]
level = "warn"
format = "json"
enable_console = false
enable_file = true
file_path = "/var/log/scim-server.log"
max_file_size = "100MB"
max_files = 30
enable_compression = true
enable_request_logging = false

[storage.database]
connection_url = "postgresql://scim:${SCIM_DB_PASSWORD}@postgres:5432/scim"
max_connections = 50
connection_timeout_ms = 5000
enable_ssl = true
migration_auto_run = true

[auth]
enabled_methods = ["bearer"]
bearer_token_validation = "jwt"
jwt_issuer = "https://auth.company.com"
jwt_audience = "scim-api"
jwt_public_key_url = "https://auth.company.com/.well-known/jwks.json"
token_cache_ttl_ms = 300000

[tls]
certificate_file = "/etc/ssl/certs/server.crt"
private_key_file = "/etc/ssl/private/server.key"
min_tls_version = "1.2"

[cache]
backend = "redis"
redis_url = "redis://redis:6379"
max_entries = 100000
ttl_ms = 300000

[rate_limit]
requests_per_minute = 10000
burst_size = 1000
enable_per_tenant_limits = true
storage_backend = "redis"

[validation]
mode = "strict"
enable_custom_validators = true
allow_unknown_attributes = false
require_schemas_field = true

[metrics]
backend = "prometheus"
enable_http_metrics = true
enable_business_metrics = true
endpoint_path = "/metrics"

[health]
endpoint_path = "/health"
enable_readiness_check = true
enable_liveness_check = true
```

## Configuration Validation

### Automatic Validation

```rust
use scim_server::config::{Config, ConfigError};

// Configuration is automatically validated
let config = Config::from_file("config.toml")?;

// Manual validation
config.validate()?;

// Check specific sections
config.server.validate()?;
config.auth.validate()?;
config.storage.validate()?;
```

### Custom Validation Rules

```rust
use scim_server::config::{ConfigValidator, ValidationResult};

struct CustomValidator;

impl ConfigValidator for CustomValidator {
    fn validate(&self, config: &Config) -> ValidationResult {
        let mut errors = Vec::new();
        
        // Custom business rules
        if config.server.max_connections > 10000 {
            errors.push("max_connections too high for this deployment".into());
        }
        
        if config.rate_limit.requests_per_minute * config.multi_tenant.max_tenants > 1000000 {
            errors.push("Combined rate limit too high".into());
        }
        
        if errors.is_empty() {
            Ok(())
        } else {
            Err(ConfigError::ValidationFailed(errors))
        }
    }
}

// Use custom validator
let config = Config::from_file("config.toml")?
    .with_validator(CustomValidator)
    .validate()?;
```

## Configuration Best Practices

### Security

1. **Never hardcode secrets** - Use environment variables or secret management
2. **Enable TLS in production** - Always use HTTPS for SCIM endpoints
3. **Validate JWT tokens** - Use proper JWT validation with key rotation
4. **Set resource limits** - Prevent DoS attacks with proper limits
5. **Enable audit logging** - Track all configuration changes

### Performance

1. **Tune connection pools** - Match database connections to expected load
2. **Configure caching** - Enable appropriate caching for your workload
3. **Set timeouts** - Prevent hanging requests with proper timeouts
4. **Monitor metrics** - Enable comprehensive metrics collection
5. **Use compression** - Enable response compression for large responses

### Reliability

1. **Enable health checks** - Configure proper health check endpoints
2. **Set up logging** - Comprehensive logging for troubleshooting
3. **Configure retries** - Implement retry logic for transient failures
4. **Plan for scaling** - Design configuration for horizontal scaling
5. **Test configurations** - Validate configurations in staging environments

This comprehensive configuration reference provides all the options needed to deploy and operate the SCIM Server library in any environment.