# Examples Documentation

This directory contains comprehensive examples demonstrating how to use the SCIM Server crate in various scenarios. All examples are tested and ready to run.

## Table of Contents

- [Basic Examples](#basic-examples)
- [Advanced Examples](#advanced-examples)
- [Integration Examples](#integration-examples)
- [Production Examples](#production-examples)
- [Running Examples](#running-examples)

## Basic Examples

### [Basic Server Setup](basic-server.md)
Learn how to create a minimal SCIM server with in-memory storage.

```rust
use scim_server::{ScimServer, providers::InMemoryProvider, create_user_resource_handler};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let provider = InMemoryProvider::new();
    let mut server = ScimServer::new(provider);
    server.register_resource_handler("User", create_user_resource_handler());
    
    println!("SCIM Server ready!");
    Ok(())
}
```

**What you'll learn**:
- Creating a SCIM server instance
- Registering resource handlers
- Basic server lifecycle

### [Resource Operations](resource-operations.md)
Comprehensive guide to CRUD operations on SCIM resources.

```rust
// Create a user
let user_data = json!({
    "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
    "userName": "john.doe@example.com",
    "name": {
        "givenName": "John",
        "familyName": "Doe"
    }
});

let context = RequestContext::with_generated_id();
let user = server.create_resource("User", user_data, &context).await?;
```

**What you'll learn**:
- Creating, reading, updating, and deleting resources
- Working with JSON payloads
- Error handling for resource operations

### [Value Objects Usage](value-objects.md)
Type-safe handling of SCIM attributes using value objects.

```rust
use scim_server::resource::value_objects::*;

// Type-safe resource construction
let username = UserName::new("jane.smith@example.com".to_string())?;
let name = Name::new_simple("Jane".to_string(), "Smith".to_string())?;
let email = EmailAddress::new_simple("jane@example.com".to_string())?;

let resource = ResourceBuilder::new("User")
    .user_name(username)?
    .name(name)?
    .build()?;
```

**What you'll learn**:
- Creating type-safe value objects
- Using the ResourceBuilder pattern
- Validation and error handling

## Advanced Examples

### [Multi-tenant Server](multi-tenant-server.md)
Building a multi-tenant SCIM server with complete tenant isolation.

```rust
use scim_server::multi_tenant::{StaticTenantResolver, TenantContext};

let resolver = StaticTenantResolver::builder()
    .add_tenant("company-a", "client-123")
    .add_tenant("company-b", "client-456")
    .build();

// Tenant-isolated operations
let tenant_context = TenantContext::new("company-a".to_string(), "client-123".to_string());
let context = RequestContext::with_tenant_generated_id(tenant_context);
```

**What you'll learn**:
- Setting up tenant resolution
- Creating tenant-isolated contexts
- Implementing tenant-aware providers
- Security considerations for multi-tenancy

### [Custom Resource Providers](custom-providers.md)
Implementing custom storage backends for production use.

```rust
impl ResourceProvider for DatabaseProvider {
    type Error = DatabaseError;

    fn create_resource(&self, resource_type: &str, data: Value, context: &RequestContext) -> impl Future<Output = Result<Resource, Self::Error>> + Send {
        async move {
            let resource = Resource::from_json(resource_type.to_string(), data)?;
            // Store in database with tenant isolation
            Ok(resource)
        }
    }
    // ... other methods
}
```

**What you'll learn**:
- Implementing the ResourceProvider trait
- Database integration patterns
- Error handling for storage operations
- Performance optimization techniques

### [Schema Validation](schema-validation.md)
Advanced schema validation and custom validation rules.

```rust
use scim_server::schema::{SchemaRegistry, OperationContext};

let registry = SchemaRegistry::new()?;
registry.validate_json_resource_with_context("User", &user_data, OperationContext::Create)?;

// Custom business validation
fn validate_business_rules(resource: &Resource) -> ValidationResult<()> {
    // Implement your specific validation logic
    Ok(())
}
```

**What you'll learn**:
- Using the schema registry
- Implementing custom validation rules
- Understanding operation contexts
- Error reporting for validation failures

## Integration Examples

### [Axum Integration](axum-integration.md)
Complete HTTP server using Axum web framework.

```rust
use axum::{routing::*, Router, Json};

async fn create_user(
    State(server): State<SharedServer>,
    Json(payload): Json<Value>,
) -> Result<Json<Value>, StatusCode> {
    let context = RequestContext::with_generated_id();
    
    match server.create_resource("User", payload, &context).await {
        Ok(resource) => Ok(Json(resource.to_json().unwrap())),
        Err(_) => Err(StatusCode::BAD_REQUEST),
    }
}

let app = Router::new()
    .route("/Users", post(create_user))
    .route("/Users/:id", get(get_user))
    .with_state(server);
```

**What you'll learn**:
- HTTP endpoint implementation
- Request/response handling
- Error mapping to HTTP status codes
- Middleware integration

### [Warp Integration](warp-integration.md)
Alternative HTTP server implementation using Warp.

```rust
use warp::Filter;

let users = warp::path("Users")
    .and(warp::post())
    .and(warp::body::json())
    .and(with_server(server))
    .and_then(create_user_handler);
```

**What you'll learn**:
- Warp filter composition
- Async handler implementation
- State management with Warp

### [Database Integration](database-integration.md)
Production-ready database integration examples.

```rust
// PostgreSQL provider
pub struct PostgresProvider {
    pool: sqlx::PgPool,
}

// MongoDB provider  
pub struct MongoProvider {
    database: mongodb::Database,
}

// Redis provider (for caching)
pub struct RedisProvider {
    client: redis::Client,
}
```

**What you'll learn**:
- Multiple database backend implementations
- Connection pooling and management
- Transaction handling
- Performance optimization for different databases

## Production Examples

### [Complete Production Server](production-server.md)
A full-featured production-ready SCIM server implementation.

**Features included**:
- Multi-tenant architecture
- Database persistence
- HTTP API with authentication
- Comprehensive logging
- Health checks and metrics
- Configuration management
- Error handling and recovery

### [Docker Deployment](docker-deployment.md)
Containerized deployment with Docker and Docker Compose.

```dockerfile
FROM rust:1.70 as builder
WORKDIR /app
COPY . .
RUN cargo build --release

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates
COPY --from=builder /app/target/release/scim-server /usr/local/bin/
CMD ["scim-server"]
```

**What you'll learn**:
- Creating optimized Docker images
- Multi-stage builds for Rust applications
- Docker Compose for development and production
- Environment variable configuration

### [Kubernetes Deployment](kubernetes-deployment.md)
Scalable deployment on Kubernetes with proper resource management.

```yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: scim-server
spec:
  replicas: 3
  selector:
    matchLabels:
      app: scim-server
  template:
    spec:
      containers:
      - name: scim-server
        image: scim-server:latest
        resources:
          requests:
            memory: "256Mi"
            cpu: "250m"
          limits:
            memory: "512Mi"
            cpu: "500m"
```

**What you'll learn**:
- Kubernetes deployment manifests
- Service configuration
- Resource limits and requests
- Horizontal pod autoscaling
- Health check configuration

### [Monitoring and Observability](monitoring.md)
Production monitoring with metrics, logging, and tracing.

```rust
use tracing::{info, error, instrument};
use metrics::{counter, histogram, gauge};

#[instrument(skip(provider, context))]
async fn monitored_create_resource<P: ResourceProvider>(
    provider: &P,
    resource_type: &str,
    data: Value,
    context: &RequestContext,
) -> Result<Resource, P::Error> {
    let start = std::time::Instant::now();
    
    counter!("scim.resources.create.attempts", 1, "resource_type" => resource_type);
    
    match provider.create_resource(resource_type, data, context).await {
        Ok(resource) => {
            let duration = start.elapsed();
            histogram!("scim.resources.create.duration", duration);
            counter!("scim.resources.create.success", 1, "resource_type" => resource_type);
            
            info!("Resource created successfully");
            Ok(resource)
        }
        Err(e) => {
            counter!("scim.resources.create.errors", 1, "resource_type" => resource_type);
            error!("Resource creation failed: {:?}", e);
            Err(e)
        }
    }
}
```

**What you'll learn**:
- Structured logging with tracing
- Metrics collection and export
- Performance monitoring
- Error tracking and alerting

## Running Examples

### Prerequisites

Make sure you have:
- Rust 1.70 or later
- Docker (for database examples)
- PostgreSQL, MongoDB, or Redis (depending on the example)

### Running Basic Examples

```bash
# Clone the repository
git clone <repository-url>
cd scim-server

# Run the basic server example
cargo run --example basic-server

# Run with specific example
cargo run --example multi-tenant-server

# Run with logging enabled
RUST_LOG=debug cargo run --example advanced-server
```

### Running Database Examples

```bash
# Start required services with Docker Compose
docker-compose up -d postgres redis

# Run PostgreSQL example
DATABASE_URL=postgresql://scim:password@localhost/scim cargo run --example postgres-provider

# Run Redis example  
REDIS_URL=redis://localhost:6379 cargo run --example redis-provider
```

### Running Production Examples

```bash
# Build optimized binary
cargo build --release

# Run production server with configuration
SCIM_DATABASE_URL=postgresql://user:pass@host/db \
SCIM_BIND_ADDRESS=0.0.0.0 \
SCIM_PORT=3000 \
SCIM_LOG_LEVEL=info \
./target/release/scim-server
```

## Example Categories

### 🚀 **Getting Started**
- [Basic Server](basic-server.md) - Minimal working server
- [First Resource](first-resource.md) - Creating your first user
- [Simple CRUD](simple-crud.md) - Basic create, read, update, delete

### 🏗️ **Core Features**
- [Resource Management](resource-management.md) - Advanced resource operations
- [Value Objects](value-objects.md) - Type-safe attribute handling
- [Error Handling](error-handling.md) - Comprehensive error management
- [Schema Validation](schema-validation.md) - Validation patterns

### 🏢 **Multi-tenancy**
- [Basic Multi-tenancy](basic-multi-tenancy.md) - Simple tenant isolation
- [Advanced Tenancy](advanced-multi-tenancy.md) - Complex tenant scenarios
- [Tenant Resolution](tenant-resolution.md) - Custom resolution strategies
- [Tenant Security](tenant-security.md) - Security best practices

### 💾 **Storage & Providers**
- [In-Memory Provider](in-memory-provider.md) - Development and testing
- [PostgreSQL Provider](postgresql-provider.md) - Production database
- [MongoDB Provider](mongodb-provider.md) - Document database
- [Redis Provider](redis-provider.md) - Caching and session storage
- [Custom Provider](custom-provider.md) - Implementing your own

### 🌐 **Web Integration**
- [Axum HTTP Server](axum-integration.md) - Modern async web framework
- [Warp HTTP Server](warp-integration.md) - Composable web server
- [Actix-Web Integration](actix-integration.md) - High-performance web framework
- [Authentication](authentication.md) - JWT, OAuth, API keys

### 📊 **Production Features**
- [Logging & Monitoring](monitoring.md) - Observability setup
- [Health Checks](health-checks.md) - Application health monitoring
- [Configuration](configuration.md) - Environment-based configuration
- [Performance Testing](performance-testing.md) - Load testing and benchmarks

### 🚀 **Deployment**
- [Docker Setup](docker-deployment.md) - Containerized deployment
- [Kubernetes](kubernetes-deployment.md) - Scalable orchestration
- [AWS Deployment](aws-deployment.md) - Cloud deployment guide
- [Monitoring Stack](monitoring-stack.md) - Production monitoring setup

## Code Quality

All examples follow these standards:

- ✅ **Compile and run successfully**
- ✅ **Include comprehensive error handling**
- ✅ **Follow Rust best practices**
- ✅ **Demonstrate realistic usage scenarios**
- ✅ **Include inline documentation**
- ✅ **Show both success and error cases**

## Contributing Examples

We welcome community examples! When contributing:

1. **Follow the template structure**
2. **Include comprehensive documentation**
3. **Test your example thoroughly**
4. **Add it to this index**
5. **Update any related documentation**

### Example Template

```rust
//! # Example: [Brief Description]
//!
//! This example demonstrates [specific feature/use case].
//!
//! ## Prerequisites
//! - [List any dependencies or setup required]
//!
//! ## Usage
//! ```bash
//! cargo run --example example-name
//! ```

use scim_server::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Your example code here
    Ok(())
}
```

## Support

- **Questions**: Open a discussion on GitHub
- **Bugs**: Report issues with examples
- **Improvements**: Submit pull requests with enhancements

---

*All examples are maintained alongside the main codebase to ensure they remain current and functional.*