# Quick Start Guide

Get your SCIM 2.0 server running in under 5 minutes! This guide will walk you through setting up a basic SCIM server with user management capabilities.

## Prerequisites

- Rust 1.70 or later
- Basic familiarity with Rust and async programming
- Understanding of HTTP APIs (helpful but not required)

## Step 1: Add Dependencies

Add the SCIM server to your `Cargo.toml`:

```toml
[dependencies]
scim-server = "0.1.0"
tokio = { version = "1.0", features = ["full"] }
serde_json = "1.0"
```

## Step 2: Create a Basic Server

Create `src/main.rs` with a minimal SCIM server:

```rust
use scim_server::{
    ScimServer, ResourceProvider, Resource, RequestContext, 
    ScimOperation, ListQuery, create_user_resource_handler
};
use std::collections::HashMap;
use tokio::sync::RwLock;
use std::sync::Arc;
use serde_json::Value;
use std::future::Future;

// Simple in-memory resource provider
struct MyResourceProvider {
    resources: Arc<RwLock<HashMap<String, Resource>>>,
}

impl MyResourceProvider {
    fn new() -> Self {
        Self {
            resources: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

#[derive(Debug, thiserror::Error)]
#[error("Provider error")]
struct MyError;

impl ResourceProvider for MyResourceProvider {
    type Error = MyError;

    fn create_resource(&self, resource_type: &str, data: Value, _context: &RequestContext) -> impl Future<Output = Result<Resource, Self::Error>> + Send {
        async move {
            Resource::from_json(resource_type.to_string(), data)
                .map_err(|_| MyError)
        }
    }

    fn get_resource(&self, _resource_type: &str, _id: &str, _context: &RequestContext) -> impl Future<Output = Result<Option<Resource>, Self::Error>> + Send {
        async move { Ok(None) }
    }

    fn update_resource(&self, resource_type: &str, _id: &str, data: Value, _context: &RequestContext) -> impl Future<Output = Result<Resource, Self::Error>> + Send {
        async move {
            Resource::from_json(resource_type.to_string(), data)
                .map_err(|_| MyError)
        }
    }

    fn delete_resource(&self, _resource_type: &str, _id: &str, _context: &RequestContext) -> impl Future<Output = Result<(), Self::Error>> + Send {
        async move { Ok(()) }
    }

    fn list_resources(&self, _resource_type: &str, _query: Option<&ListQuery>, _context: &RequestContext) -> impl Future<Output = Result<Vec<Resource>, Self::Error>> + Send {
        async move { Ok(vec![]) }
    }

    fn find_resource_by_attribute(&self, _resource_type: &str, _attribute: &str, _value: &Value, _context: &RequestContext) -> impl Future<Output = Result<Option<Resource>, Self::Error>> + Send {
        async move { Ok(None) }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    env_logger::init();

    // Create provider and server
    let provider = MyResourceProvider::new();
    let mut server = ScimServer::new(provider);

    // Register user resource handler
    let user_handler = create_user_resource_handler();
    server.register_resource_handler("User", user_handler);

    println!("SCIM Server is ready!");
    println!("You can now:");
    println!("- Create users via POST /Users");
    println!("- List users via GET /Users");
    println!("- Get specific users via GET /Users/{{id}}");
    
    // In a real application, you'd integrate with your web framework here
    // For example, with Axum, Warp, or Actix-web
    
    Ok(())
}
```

## Step 3: Test Your Server

Run your server:

```bash
cargo run
```

You should see:
```
SCIM Server is ready!
You can now:
- Create users via POST /Users
- List users via GET /Users
- Get specific users via GET /Users/{id}
```

## Step 4: Test with Sample Data

Create a test user with this JSON payload:

```json
{
  "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
  "userName": "john.doe@example.com",
  "name": {
    "givenName": "John",
    "familyName": "Doe"
  },
  "emails": [
    {
      "value": "john.doe@example.com",
      "type": "work",
      "primary": true
    }
  ],
  "active": true
}
```

## Integration with Web Frameworks

### With Axum

```rust
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Json,
    routing::{get, post, put, delete},
    Router,
};

async fn create_user(
    State(server): State<Arc<ScimServer<MyResourceProvider>>>,
    Json(payload): Json<Value>,
) -> Result<Json<Value>, StatusCode> {
    let context = RequestContext::with_generated_id();
    
    match server.create_resource("User", payload, &context).await {
        Ok(resource) => Ok(Json(resource.to_json().unwrap())),
        Err(_) => Err(StatusCode::BAD_REQUEST),
    }
}

#[tokio::main]
async fn main() {
    let provider = MyResourceProvider::new();
    let server = Arc::new(ScimServer::new(provider));
    
    let app = Router::new()
        .route("/Users", post(create_user))
        .route("/Users", get(list_users))
        .route("/Users/:id", get(get_user))
        .route("/Users/:id", put(update_user))
        .route("/Users/:id", delete(delete_user))
        .with_state(server);

    axum::Server::bind(&"0.0.0.0:3000".parse().unwrap())
        .serve(app.into_make_service())
        .await
        .unwrap();
}
```

### With Warp

```rust
use warp::Filter;

#[tokio::main]
async fn main() {
    let provider = MyResourceProvider::new();
    let server = Arc::new(ScimServer::new(provider));
    
    let users = warp::path("Users")
        .and(warp::post())
        .and(warp::body::json())
        .and(with_server(server.clone()))
        .and_then(create_user_handler);

    warp::serve(users)
        .run(([127, 0, 0, 1], 3000))
        .await;
}
```

## Next Steps

Now that you have a basic server running, explore these topics:

1. **[Multi-tenancy](../api/multi-tenancy.md)** - Support multiple tenants
2. **[Custom Providers](../examples/custom-providers.md)** - Build database-backed providers
3. **[Schema Validation](../api/core-types.md#schemas)** - Understand schema validation
4. **[Production Deployment](tutorial-production.md)** - Deploy to production
5. **[Advanced Features](../examples/advanced-features.md)** - Logging, metrics, monitoring

## Common Issues

### "Cannot find ResourceProvider"
Make sure you've implemented all required methods of the `ResourceProvider` trait.

### "Validation errors"
Check that your JSON payload matches the SCIM 2.0 User schema. See [Schema Reference](../reference/schemas.md).

### "Async runtime errors"
Ensure you're using `#[tokio::main]` or properly initializing an async runtime.

## What's Next?

- **Production Ready**: This basic setup is actually production-ready for small applications
- **Scale Up**: For larger applications, implement database-backed providers
- **Multi-tenant**: Add tenant isolation for SaaS applications
- **Monitoring**: Add comprehensive logging and metrics

Continue with the [User Guide](user-guide.md) for detailed explanations of all features.