# Basic Server Example

This document provides a complete, step-by-step example of building a basic SCIM server using the SCIM Server crate. This example demonstrates the core concepts and provides a foundation you can build upon.

## Table of Contents

- [Overview](#overview)
- [Prerequisites](#prerequisites)
- [Basic Server Implementation](#basic-server-implementation)
- [Running the Server](#running-the-server)
- [Testing the Server](#testing-the-server)
- [Configuration Options](#configuration-options)
- [Adding Custom Logic](#adding-custom-logic)
- [Next Steps](#next-steps)

## Overview

This example creates a minimal but functional SCIM server that:

- Supports User and Group resources
- Uses in-memory storage for simplicity
- Provides all standard SCIM 2.0 endpoints
- Includes basic logging and error handling
- Can be extended with custom providers

## Prerequisites

Add the SCIM Server crate to your `Cargo.toml`:

```toml
[dependencies]
scim-server = "0.1.0"
tokio = { version = "1.0", features = ["full"] }
tracing = "0.1"
tracing-subscriber = "0.3"
serde_json = "1.0"
```

## Basic Server Implementation

### Step 1: Create the Main Server

Create `src/main.rs`:

```rust
use scim_server::{ScimServer, ServerConfig};
use scim_server::providers::InMemoryProvider;
use scim_server::error::Result;
use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .with_target(false)
        .finish();
    tracing::subscriber::set_global_default(subscriber)
        .expect("Setting default subscriber failed");

    info!("Starting SCIM Server...");

    // Create in-memory provider
    let provider = InMemoryProvider::new();

    // Configure server
    let config = ServerConfig::builder()
        .host("localhost")
        .port(8080)
        .base_url("http://localhost:8080/scim/v2")
        .provider(provider)
        .enable_cors(true)
        .cors_origins(vec!["http://localhost:3000"])
        .build()?;

    info!("Server configured on {}:{}", config.host(), config.port());

    // Create and start server
    let server = ScimServer::new(config);
    
    info!("SCIM Server starting on http://localhost:8080");
    info!("API Base URL: http://localhost:8080/scim/v2");
    info!("Available endpoints:");
    info!("  GET    /scim/v2/Users");
    info!("  POST   /scim/v2/Users");
    info!("  GET    /scim/v2/Users/{{id}}");
    info!("  PUT    /scim/v2/Users/{{id}}");
    info!("  PATCH  /scim/v2/Users/{{id}}");
    info!("  DELETE /scim/v2/Users/{{id}}");
    info!("  GET    /scim/v2/Groups");
    info!("  POST   /scim/v2/Groups");
    info!("  GET    /scim/v2/Groups/{{id}}");
    info!("  PUT    /scim/v2/Groups/{{id}}");
    info!("  PATCH  /scim/v2/Groups/{{id}}");
    info!("  DELETE /scim/v2/Groups/{{id}}");

    server.run().await?;

    Ok(())
}
```

### Step 2: Add Sample Data (Optional)

Create `src/sample_data.rs` to populate the server with test data:

```rust
use scim_server::resource::{Resource, ResourceBuilder};
use scim_server::resource::value_objects::{
    ResourceId, UserName, EmailAddress, Name, Address, PhoneNumber
};
use scim_server::providers::ResourceProvider;
use scim_server::error::Result;

pub async fn populate_sample_data<P: ResourceProvider>(provider: &P) -> Result<()> {
    // Create sample users
    let users = vec![
        create_sample_user("user-001", "john.doe", "John", "Doe", "john.doe@example.com")?,
        create_sample_user("user-002", "jane.smith", "Jane", "Smith", "jane.smith@example.com")?,
        create_sample_user("user-003", "bob.wilson", "Bob", "Wilson", "bob.wilson@example.com")?,
    ];

    for user in users {
        provider.create_resource(user).await?;
    }

    // Create sample groups
    let groups = vec![
        create_sample_group("group-001", "Administrators", vec!["user-001"])?,
        create_sample_group("group-002", "Developers", vec!["user-001", "user-002"])?,
        create_sample_group("group-003", "Support Team", vec!["user-003"])?,
    ];

    for group in groups {
        provider.create_resource(group).await?;
    }

    println!("Sample data populated successfully!");
    Ok(())
}

fn create_sample_user(
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
        .add_phone(PhoneNumber::new("+1-555-0123")?.with_type("work"))
        .add_address(Address::builder()
            .street_address("123 Main St")
            .locality("Anytown")
            .region("CA")
            .postal_code("90210")
            .country("US")
            .type_("work")
            .build())
        .active(true)
        .build()
}

fn create_sample_group(
    id: &str,
    display_name: &str,
    member_ids: Vec<&str>,
) -> Result<Resource> {
    let mut builder = ResourceBuilder::new()
        .id(ResourceId::new(id)?)
        .display_name(display_name);

    for member_id in member_ids {
        builder = builder.add_group_member(
            GroupMember::new_user(ResourceId::new(member_id)?)
                .with_display_name(format!("User {}", member_id))
        );
    }

    builder.build()
}
```

### Step 3: Enhanced Main with Sample Data

Update `src/main.rs` to include sample data:

```rust
mod sample_data;

use scim_server::{ScimServer, ServerConfig};
use scim_server::providers::InMemoryProvider;
use scim_server::error::Result;
use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .with_target(false)
        .with_thread_names(true)
        .finish();
    tracing::subscriber::set_global_default(subscriber)
        .expect("Setting default subscriber failed");

    info!("Starting Basic SCIM Server Example...");

    // Create provider and populate with sample data
    let provider = InMemoryProvider::new();
    
    // Populate sample data
    sample_data::populate_sample_data(&provider).await?;
    info!("Sample data loaded");

    // Configure server
    let config = ServerConfig::builder()
        .host("localhost")
        .port(8080)
        .base_url("http://localhost:8080/scim/v2")
        .provider(provider)
        .enable_cors(true)
        .cors_origins(vec!["http://localhost:3000", "http://localhost:8000"])
        .cors_methods(vec!["GET", "POST", "PUT", "PATCH", "DELETE"])
        .enable_request_logging(true)
        .build()?;

    // Create server
    let server = ScimServer::new(config);
    
    print_server_info();
    
    // Start server (this will block)
    server.run().await?;

    Ok(())
}

fn print_server_info() {
    info!("=== SCIM Server Started ===");
    info!("Base URL: http://localhost:8080/scim/v2");
    info!("");
    info!("Available Endpoints:");
    info!("  Users:");
    info!("    GET    /scim/v2/Users              - List all users");
    info!("    POST   /scim/v2/Users              - Create new user");
    info!("    GET    /scim/v2/Users/{{id}}         - Get specific user");
    info!("    PUT    /scim/v2/Users/{{id}}         - Update user (full)");
    info!("    PATCH  /scim/v2/Users/{{id}}         - Update user (partial)");
    info!("    DELETE /scim/v2/Users/{{id}}         - Delete user");
    info!("");
    info!("  Groups:");
    info!("    GET    /scim/v2/Groups             - List all groups");
    info!("    POST   /scim/v2/Groups             - Create new group");
    info!("    GET    /scim/v2/Groups/{{id}}        - Get specific group");
    info!("    PUT    /scim/v2/Groups/{{id}}        - Update group (full)");
    info!("    PATCH  /scim/v2/Groups/{{id}}        - Update group (partial)");
    info!("    DELETE /scim/v2/Groups/{{id}}        - Delete group");
    info!("");
    info!("  Discovery:");
    info!("    GET    /scim/v2/ServiceProviderConfig - Server capabilities");
    info!("    GET    /scim/v2/Schemas             - Available schemas");
    info!("    GET    /scim/v2/ResourceTypes       - Supported resource types");
    info!("");
    info!("Server ready to accept requests!");
}
```

## Running the Server

### Development Mode

```bash
# Run with cargo
cargo run

# Run with debug logging
RUST_LOG=debug cargo run

# Run with automatic reloading (install cargo-watch first)
cargo install cargo-watch
cargo watch -x run
```

### Production Mode

```bash
# Build optimized binary
cargo build --release

# Run the binary
./target/release/basic-scim-server

# Or run with specific configuration
SCIM_HOST=0.0.0.0 SCIM_PORT=8443 ./target/release/basic-scim-server
```

## Testing the Server

### Using curl

Once the server is running, you can test it with curl:

```bash
# List all users
curl -X GET http://localhost:8080/scim/v2/Users \
  -H "Content-Type: application/json"

# Get a specific user
curl -X GET http://localhost:8080/scim/v2/Users/user-001 \
  -H "Content-Type: application/json"

# Create a new user
curl -X POST http://localhost:8080/scim/v2/Users \
  -H "Content-Type: application/json" \
  -d '{
    "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
    "userName": "new.user",
    "name": {
      "givenName": "New",
      "familyName": "User"
    },
    "emails": [{
      "value": "new.user@example.com",
      "type": "work",
      "primary": true
    }],
    "active": true
  }'

# Update a user
curl -X PUT http://localhost:8080/scim/v2/Users/user-001 \
  -H "Content-Type: application/json" \
  -d '{
    "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
    "id": "user-001",
    "userName": "john.doe",
    "displayName": "John Doe (Updated)",
    "active": true
  }'

# Patch a user (partial update)
curl -X PATCH http://localhost:8080/scim/v2/Users/user-001 \
  -H "Content-Type: application/json" \
  -d '{
    "schemas": ["urn:ietf:params:scim:api:messages:2.0:PatchOp"],
    "Operations": [{
      "op": "replace",
      "path": "displayName",
      "value": "John Doe (Patched)"
    }]
  }'

# Delete a user
curl -X DELETE http://localhost:8080/scim/v2/Users/user-001

# List all groups
curl -X GET http://localhost:8080/scim/v2/Groups \
  -H "Content-Type: application/json"

# Search for users
curl -X GET "http://localhost:8080/scim/v2/Users?filter=userName%20eq%20%22john.doe%22" \
  -H "Content-Type: application/json"
```

### Using a REST Client

Example requests for tools like Postman or Insomnia:

**GET /scim/v2/Users**
```
GET http://localhost:8080/scim/v2/Users
Content-Type: application/json
```

**POST /scim/v2/Users**
```
POST http://localhost:8080/scim/v2/Users
Content-Type: application/json

{
  "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
  "userName": "alice.johnson",
  "name": {
    "givenName": "Alice",
    "familyName": "Johnson",
    "formatted": "Alice Johnson"
  },
  "displayName": "Alice Johnson",
  "emails": [{
    "value": "alice.johnson@example.com",
    "type": "work",
    "primary": true
  }],
  "phoneNumbers": [{
    "value": "+1-555-0199",
    "type": "work"
  }],
  "addresses": [{
    "streetAddress": "456 Oak Avenue",
    "locality": "Springfield",
    "region": "IL",
    "postalCode": "62701",
    "country": "US",
    "type": "work"
  }],
  "active": true
}
```

## Configuration Options

### Environment Variables

The basic server can be configured using environment variables:

```bash
# Server binding
export SCIM_HOST=0.0.0.0
export SCIM_PORT=8080

# Base URL for resource links
export SCIM_BASE_URL=https://api.example.com/scim/v2

# Logging level
export RUST_LOG=info

# CORS settings
export SCIM_CORS_ORIGINS=https://admin.example.com,https://app.example.com
```

### Configuration File Support

Create `config.toml`:

```toml
[server]
host = "localhost"
port = 8080
base_url = "http://localhost:8080/scim/v2"

[cors]
enabled = true
origins = ["http://localhost:3000", "http://localhost:8000"]
methods = ["GET", "POST", "PUT", "PATCH", "DELETE"]
headers = ["Content-Type", "Authorization"]

[logging]
level = "info"
format = "pretty"
enable_request_logging = true

[provider]
type = "memory"
initial_capacity = 1000
```

Update your main function to use configuration files:

```rust
use std::env;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging first
    init_logging();

    // Load configuration
    let config_path = env::var("SCIM_CONFIG")
        .unwrap_or_else(|_| "config.toml".to_string());
    
    let config = if std::path::Path::new(&config_path).exists() {
        ServerConfig::from_file(&config_path).await?
    } else {
        // Fallback to environment/defaults
        create_default_config().await?
    };

    // Create and start server
    let server = ScimServer::new(config);
    server.run().await
}

async fn create_default_config() -> Result<ServerConfig> {
    let provider = InMemoryProvider::new();
    
    ServerConfig::builder()
        .host(env::var("SCIM_HOST").unwrap_or_else(|_| "localhost".to_string()))
        .port(env::var("SCIM_PORT")
            .unwrap_or_else(|_| "8080".to_string())
            .parse()
            .unwrap_or(8080))
        .base_url(env::var("SCIM_BASE_URL")
            .unwrap_or_else(|_| "http://localhost:8080/scim/v2".to_string()))
        .provider(provider)
        .enable_cors(true)
        .build()
}

fn init_logging() {
    let log_level = env::var("RUST_LOG").unwrap_or_else(|_| "info".to_string());
    
    let subscriber = FmtSubscriber::builder()
        .with_max_level(log_level.parse().unwrap_or(Level::INFO))
        .with_target(false)
        .with_thread_names(true)
        .finish();
    
    tracing::subscriber::set_global_default(subscriber)
        .expect("Setting default subscriber failed");
}
```

## Adding Custom Logic

### Custom Validation

Add business-specific validation rules:

```rust
use scim_server::resource::Resource;
use scim_server::error::{Result, ScimError};

async fn validate_business_rules(resource: &Resource) -> Result<()> {
    // Example: Validate department codes
    if let Some(department) = resource.get_attribute("department") {
        if let Some(dept_str) = department.as_str() {
            if !is_valid_department_code(dept_str) {
                return Err(ScimError::validation_error(
                    "department",
                    format!("Invalid department code: {}", dept_str)
                ));
            }
        }
    }

    // Example: Validate employee number format
    if let Some(emp_number) = resource.get_attribute("employeeNumber") {
        if let Some(emp_str) = emp_number.as_str() {
            if !emp_str.starts_with("EMP") || emp_str.len() != 10 {
                return Err(ScimError::validation_error(
                    "employeeNumber",
                    "Employee number must start with 'EMP' and be 10 characters long"
                ));
            }
        }
    }

    Ok(())
}

fn is_valid_department_code(code: &str) -> bool {
    const VALID_DEPARTMENTS: &[&str] = &[
        "ENG", "HR", "SALES", "MARKETING", "FINANCE", "LEGAL", "IT"
    ];
    VALID_DEPARTMENTS.contains(&code)
}
```

### Pre/Post Processing Hooks

Add hooks for custom processing:

```rust
use scim_server::middleware::RequestHook;
use async_trait::async_trait;

pub struct AuditHook;

#[async_trait]
impl RequestHook for AuditHook {
    async fn before_create(&self, resource: &Resource) -> Result<()> {
        info!("Creating resource: {} ({})", 
              resource.id(), 
              resource.resource_type());
        
        // Log to audit system
        audit_log("RESOURCE_CREATE", resource).await?;
        Ok(())
    }
    
    async fn after_create(&self, resource: &Resource) -> Result<()> {
        info!("Resource created successfully: {}", resource.id());
        
        // Send notification
        send_creation_notification(resource).await?;
        Ok(())
    }
    
    async fn before_delete(&self, id: &ResourceId) -> Result<()> {
        warn!("Deleting resource: {}", id);
        
        // Check if resource can be safely deleted
        if is_critical_resource(id).await? {
            return Err(ScimError::forbidden(
                "Cannot delete critical system resource"
            ));
        }
        
        Ok(())
    }
}

// Register the hook
let config = ServerConfig::builder()
    .provider(provider)
    .add_hook(Box::new(AuditHook))
    .build()?;
```

### Custom Error Responses

Customize error response format:

```rust
use axum::{response::IntoResponse, Json};
use serde_json::json;

impl IntoResponse for ScimError {
    fn into_response(self) -> axum::response::Response {
        let status_code = axum::http::StatusCode::from_u16(self.status_code())
            .unwrap_or(axum::http::StatusCode::INTERNAL_SERVER_ERROR);

        let error_response = json!({
            "schemas": ["urn:ietf:params:scim:api:messages:2.0:Error"],
            "status": self.status_code().to_string(),
            "scimType": self.scim_type(),
            "detail": self.to_string(),
            "location": self.location(),
            "timestamp": chrono::Utc::now().to_rfc3339(),
            "correlationId": generate_correlation_id()
        });

        (status_code, Json(error_response)).into_response()
    }
}
```

## Health Check Endpoint

Add a health check endpoint for monitoring:

```rust
use axum::{routing::get, Router, Json};
use serde_json::json;

async fn health_check() -> Json<serde_json::Value> {
    Json(json!({
        "status": "healthy",
        "timestamp": chrono::Utc::now().to_rfc3339(),
        "version": env!("CARGO_PKG_VERSION"),
        "uptime": get_uptime_seconds()
    }))
}

// Add to your router
let app = Router::new()
    .route("/health", get(health_check))
    .nest("/scim/v2", scim_routes);
```

## Graceful Shutdown

Handle shutdown signals gracefully:

```rust
use tokio::signal;
use tracing::{info, warn};

#[tokio::main]
async fn main() -> Result<()> {
    // ... initialization code ...

    let server = ScimServer::new(config);
    
    // Set up graceful shutdown
    let shutdown_signal = async {
        signal::ctrl_c()
            .await
            .expect("Failed to install CTRL+C signal handler");
        
        info!("Shutdown signal received, starting graceful shutdown...");
    };

    // Run server with graceful shutdown
    tokio::select! {
        result = server.run() => {
            if let Err(e) = result {
                warn!("Server error: {}", e);
            }
        }
        _ = shutdown_signal => {
            info!("Shutdown signal received");
        }
    }

    info!("Server shutdown complete");
    Ok(())
}
```

## Extended Example with Middleware

Add custom middleware for additional functionality:

```rust
use axum::{
    middleware::{self, Next},
    request::Request,
    response::Response,
};
use tower::ServiceBuilder;
use tower_http::{
    trace::TraceLayer,
    timeout::TimeoutLayer,
    limit::RequestBodyLimitLayer,
};
use std::time::Duration;

async fn request_id_middleware(
    mut request: Request,
    next: Next,
) -> Result<Response, ScimError> {
    // Generate unique request ID
    let request_id = uuid::Uuid::new_v4().to_string();
    
    // Add to request headers
    request.headers_mut().insert(
        "x-request-id",
        request_id.parse().unwrap()
    );
    
    // Add to tracing context
    let span = tracing::info_span!("request", request_id = %request_id);
    let _enter = span.enter();
    
    // Process request
    let response = next.run(request).await?;
    
    // Add request ID to response
    let mut response = response.into_response();
    response.headers_mut().insert(
        "x-request-id",
        request_id.parse().unwrap()
    );
    
    Ok(response)
}

// Apply middleware to your server
let app = Router::new()
    .nest("/scim/v2", scim_routes)
    .layer(
        ServiceBuilder::new()
            .layer(TraceLayer::new_for_http())
            .layer(middleware::from_fn(request_id_middleware))
            .layer(TimeoutLayer::new(Duration::from_secs(30)))
            .layer(RequestBodyLimitLayer::new(1024 * 1024)) // 1MB limit
    );
```

## Complete Working Example

Here's a complete, minimal working example in a single file:

```rust
// src/main.rs
use scim_server::{ScimServer, ServerConfig};
use scim_server::providers::InMemoryProvider;
use scim_server::resource::{ResourceBuilder};
use scim_server::resource::value_objects::{ResourceId, UserName, EmailAddress, Name};
use scim_server::error::Result;
use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;

#[tokio::main]
async fn main() -> Result<()> {
    // Setup logging
    FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .init();

    // Create provider with sample data
    let provider = InMemoryProvider::new();
    
    // Add a sample user
    let sample_user = ResourceBuilder::new()
        .id(ResourceId::new("demo-user")?)
        .user_name(UserName::new("demo.user")?)
        .display_name("Demo User")
        .add_email(EmailAddress::new("demo@example.com")?)
        .active(true)
        .build()?;
    
    provider.create_resource(sample_user).await?;

    // Configure and start server
    let config = ServerConfig::builder()
        .host("localhost")
        .port(8080)
        .base_url("http://localhost:8080/scim/v2")
        .provider(provider)
        .enable_cors(true)
        .build()?;

    info!("Starting SCIM server on http://localhost:8080");
    info!("Try: curl http://localhost:8080/scim/v2/Users");

    ScimServer::new(config).run().await
}
```

## Common Issues and Solutions

### Port Already in Use

```rust
use std::net::TcpListener;

fn find_available_port(start_port: u16) -> Result<u16> {
    for port in start_port..start_port + 100 {
        if TcpListener::bind(("localhost", port)).is_ok() {
            return Ok(port);
        }
    }
    Err(ScimError::internal_error("No available ports found"))
}

// Use in configuration
let port = find_available_port(8080)?;
let config = ServerConfig::builder()
    .port(port)
    .build()?;
```

### CORS Issues

```rust
// Permissive CORS for development
let config = ServerConfig::builder()
    .enable_cors(true)
    .cors_origins(vec!["*"])  // Allow all origins (development only!)
    .cors_methods(vec!["GET", "POST", "PUT", "PATCH", "DELETE", "OPTIONS"])
    .cors_headers(vec!["*"])
    .build()?;

// Restrictive CORS for production
let config = ServerConfig::builder()
    .enable_cors(true)
    .cors_origins(vec![
        "https://admin.mycompany.com",
        "https://app.mycompany.com"
    ])
    .cors_methods(vec!["GET", "POST", "PUT", "PATCH", "DELETE"])
    .cors_headers(vec!["Content-Type", "Authorization"])
    .cors_max_age(Duration::from_secs(3600))
    .build()?;
```

### Memory Usage Monitoring

```rust
use sysinfo::{System, SystemExt};

async fn memory_monitor() {
    let mut system = System::new_all();
    
    loop {
        system.refresh_all();
        let used_memory = system.used_memory();
        let total_memory = system.total_memory();
        let usage_percent = (used_memory as f64 / total_memory as f64) * 100.0;
        
        if usage_percent > 80.0 {
            warn!("High memory usage: {:.1}%", usage_percent);
        }
        
        tokio::time::sleep(Duration::from_secs(60)).await;
    }
}

// Start monitoring in background
tokio::spawn(memory_monitor());
```

## Next Steps

Now that you have a basic SCIM server running, you can:

1. **[Implement Multi-Tenancy](multi-tenant-server.md)** - Support multiple organizations
2. **[Create Custom Providers](custom-providers.md)** - Connect to your database
3. **[Add Advanced Features](advanced-features.md)** - Schema validation, bulk operations
4. **[Deploy to Production](../guides/tutorial-production.md)** - Production deployment guide
5. **[Explore the API](../api/README.md)** - Learn about advanced API features

## Troubleshooting

### Server Won't Start

1. Check if port is available: `netstat -an | grep 8080`
2. Verify configuration: Enable debug logging with `RUST_LOG=debug`
3. Check file permissions if using configuration files

### API Requests Failing

1. Verify Content-Type header: `application/json`
2. Check CORS configuration for browser requests
3. Validate request body format against SCIM schema
4. Check server logs for detailed error information

### Performance Issues

1. Monitor memory usage with system tools
2. Enable request logging to identify slow operations
3. Consider switching to a database provider for large datasets
4. Implement caching for frequently accessed resources

This basic server example provides a solid foundation for building more complex SCIM implementations. The modular architecture makes it easy to add features incrementally as your requirements grow.