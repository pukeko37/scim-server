# SCIM Server Logging Guide

This document provides comprehensive information about logging in the SCIM server library.

## Overview

The SCIM server uses the standard Rust `log` crate as a logging facade, allowing you to choose your preferred logging backend (env_logger, tracing, slog, etc.). This approach provides maximum flexibility while maintaining consistent logging throughout the library.

## Logging Philosophy

- **Non-intrusive**: The library doesn't force a specific logging implementation
- **Structured**: Logs include contextual information like request IDs and tenant information
- **Configurable**: Different log levels for different components
- **Production-ready**: Appropriate log levels and performance considerations

## Quick Start

Add a logging backend to your application:

```toml
[dependencies]
# For simple applications
env_logger = "0.10"

# For structured logging in production
tracing-subscriber = "0.3"
```

Initialize logging in your application:

```rust
// Simple logging (good for development)
env_logger::init();

// Or with custom configuration
env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
    .format_timestamp_secs()
    .init();
```

## Log Levels

The SCIM server uses standard log levels:

- **ERROR**: Critical errors that prevent operations from completing
- **WARN**: Non-critical issues that should be investigated
- **INFO**: Important operational events (resource creation, deletion, etc.)
- **DEBUG**: Detailed operational information (resource retrieval, listing, etc.)
- **TRACE**: Very detailed information including request data

## What Gets Logged

### SCIM Operations
All SCIM operations are logged with structured information:

```
[INFO] SCIM create User operation initiated (request: 'abc-123')
[INFO] SCIM create User operation completed successfully: ID '42' (request: 'abc-123')
```

### Provider Operations
Resource provider operations include tenant context:

```
[INFO] Creating User resource for tenant 'default' (request: 'abc-123')
[DEBUG] Getting User resource with ID '42' for tenant 'tenant-1' (request: 'def-456')
```

### Error Conditions
Errors are logged with full context:

```
[WARN] SCIM delete User operation failed for ID 'missing': Resource not found
[WARN] Attempted to delete non-existent User resource with ID 'missing' for tenant 'default'
```

### Multi-tenant Context
All logs include tenant information when available:

```
[INFO] Creating User resource for tenant 'enterprise-corp' (request: 'xyz-789')
[DEBUG] Found 15 User resources for tenant 'startup-inc' (after filtering)
```

## Configuration Examples

### Development Configuration

```rust
use env_logger::Builder;
use log::LevelFilter;

Builder::new()
    .filter_level(LevelFilter::Debug)
    .filter_module("scim_server", LevelFilter::Trace)
    .init();
```

### Production Configuration

```rust
use env_logger::Builder;
use log::LevelFilter;

Builder::new()
    .filter_level(LevelFilter::Info)
    .filter_module("scim_server::providers", LevelFilter::Debug)
    .format_timestamp_secs()
    .init();
```

### Environment Variable Configuration

```bash
# Basic debug logging
export RUST_LOG=debug

# Detailed SCIM server logging
export RUST_LOG=scim_server=trace

# Provider-specific logging
export RUST_LOG=scim_server::providers=debug

# Mixed configuration
export RUST_LOG=info,scim_server::providers=debug,scim_server::resource=trace
```

## Structured Logging with Tracing

For production applications, consider using the `tracing` ecosystem:

```toml
[dependencies]
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["json", "env-filter"] }
```

```rust
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

tracing_subscriber::registry()
    .with(tracing_subscriber::EnvFilter::new(
        std::env::var("RUST_LOG").unwrap_or_else(|_| "info".into()),
    ))
    .with(tracing_subscriber::fmt::layer().json())
    .init();
```

This produces structured JSON logs suitable for log aggregation systems.

## Log Content Structure

### Request Context
Every log entry includes:
- **Request ID**: Unique identifier for request tracing
- **Tenant ID**: For multi-tenant operations
- **Resource Type**: The type of resource being operated on
- **Operation**: The SCIM operation being performed

### Example Log Entry Analysis

```
[INFO scim_server::providers::in_memory] Creating User resource for tenant 'tenant-1' (request: 'a1b2c3d4')
```

- **Level**: INFO
- **Module**: scim_server::providers::in_memory
- **Operation**: Creating User resource
- **Tenant**: tenant-1
- **Request ID**: a1b2c3d4

## Performance Considerations

- **Log Level Impact**: TRACE and DEBUG levels include data serialization
- **Production Recommendations**: Use INFO level or higher for production
- **Async Logging**: Consider async logging backends for high-throughput applications

## Security Considerations

- **Sensitive Data**: The library avoids logging sensitive data like passwords
- **Data Exposure**: TRACE level may include request data - use carefully in production
- **Log Storage**: Ensure logs are stored securely and access is controlled

## Integration Examples

### With Tokio Tracing

```rust
use tracing::{info, instrument};

#[instrument]
async fn create_user_with_tracing() {
    let provider = InMemoryProvider::new();
    let mut server = ScimServer::new(provider)?;
    
    let context = RequestContext::with_generated_id();
    let user = server.create_resource("User", user_data, &context).await?;
    
    info!("User created successfully");
}
```

### With Slog

```rust
use slog::{Drain, Logger, o, info};
use slog_term;
use slog_async;

let decorator = slog_term::TermDecorator::new().build();
let drain = slog_term::FullFormat::new(decorator).build().fuse();
let drain = slog_async::Async::new(drain).build().fuse();
let logger = Logger::root(drain, o!());

// Configure log crate to use slog
slog_stdlog::init_with_level(log::Level::Info).unwrap();
```

## Monitoring and Alerting

### Key Metrics to Monitor
- Error rate by operation type
- Response times for SCIM operations
- Resource creation/deletion rates
- Authentication failures
- Tenant isolation violations

### Example Alert Queries (for structured logs)

```json
{
  "alert": "High Error Rate",
  "query": "level:ERROR AND module:scim_server",
  "threshold": "> 10 errors/minute"
}
```

```json
{
  "alert": "Failed Authentication",
  "query": "level:WARN AND message:*authentication*",
  "threshold": "> 5 failures/minute"
}
```

## Debugging Guide

### Common Debug Scenarios

1. **Request Tracing**: Use request IDs to follow operations across components
2. **Multi-tenant Issues**: Filter logs by tenant ID
3. **Resource Lifecycle**: Track resource creation through deletion
4. **Performance Issues**: Use DEBUG level to identify slow operations

### Debug Configuration

```rust
env_logger::Builder::new()
    .filter_level(log::LevelFilter::Warn)  // Default to WARN
    .filter_module("scim_server", log::LevelFilter::Debug)  // Debug SCIM server
    .filter_module("my_app", log::LevelFilter::Info)  // Your app level
    .init();
```

## Best Practices

### For Library Users
1. **Initialize Early**: Set up logging before creating SCIM server
2. **Use Request IDs**: Include request IDs in your own logs for correlation
3. **Monitor Errors**: Set up alerts for ERROR and WARN level logs
4. **Test Logging**: Verify log output in development and staging

### For Production
1. **Structured Logging**: Use JSON format for log aggregation
2. **Log Rotation**: Configure log rotation to manage disk space
3. **Centralized Logging**: Use ELK stack, Splunk, or similar systems
4. **Performance**: Use async logging for high-throughput applications

### For Development
1. **Verbose Logging**: Use DEBUG or TRACE for development
2. **Module Filtering**: Focus on specific modules during debugging
3. **Color Output**: Use colored output for better readability
4. **Real-time Monitoring**: Use tools like `tail -f` for live log monitoring

## Troubleshooting

### Common Issues

**No logs appearing:**
```rust
// Ensure logging is initialized
env_logger::init();
```

**Too verbose:**
```bash
# Reduce log level
export RUST_LOG=warn
```

**Missing context:**
```bash
# Enable debug logging for specific modules
export RUST_LOG=scim_server::providers=debug
```

**Performance impact:**
```rust
// Use appropriate log levels for production
Builder::new()
    .filter_level(LevelFilter::Info)  // Not Debug or Trace
    .init();
```

## Example Applications

See the complete logging example in `examples/logging_example.rs` for a comprehensive demonstration of all logging features.

## Contributing

When contributing to the SCIM server library:

1. Use appropriate log levels
2. Include contextual information (request IDs, tenant IDs)
3. Follow the existing logging patterns
4. Test logging output
5. Update this documentation for significant changes

## References

- [Rust log crate documentation](https://docs.rs/log/)
- [env_logger documentation](https://docs.rs/env_logger/)
- [tracing documentation](https://docs.rs/tracing/)
- [Logging best practices for Rust](https://rust-lang-nursery.github.io/rust-cookbook/development_tools/debugging/log.html)