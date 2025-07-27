# SCIM Server Library for Rust

[![Crates.io](https://img.shields.io/crates/v/scim-server.svg)](https://crates.io/crates/scim-server)
[![Documentation](https://docs.rs/scim-server/badge.svg)](https://docs.rs/scim-server)
[![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

A comprehensive **System for Cross-domain Identity Management (SCIM) server library** in Rust that enables developers to implement SCIM-compliant identity providers with minimal effort. SCIM is an IETF standard (RFC 7643/7644) for automating user provisioning between cloud applications and identity systems.

## 🚀 Features

- **Type-safe state machine** preventing invalid operations at compile time
- **Trait-based architecture** for flexible data access patterns  
- **Full RFC 7643/7644 compliance** for core User schema
- **Async-first design** with functional programming patterns
- **Comprehensive validation** with detailed error reporting
- **Zero-cost abstractions** leveraging Rust's type system
- **Extensible schema system** (future versions)

## 📋 Current Status - MVP

This is the **Minimum Viable Product (MVP)** release focusing on core functionality:

### ✅ Included
- Core SCIM User schema with validation
- Basic CRUD operations (Create, Read, Update, Delete, List)
- Schema discovery endpoints (`/Schemas`, `/ServiceProviderConfig`)
- Type-safe server state management
- Comprehensive error handling
- In-memory reference implementation

### 🔮 Future Versions
- Group resources and custom resource types
- Advanced filtering, sorting, and pagination
- Bulk operations and PATCH support
- Authentication and authorization frameworks
- Database integration examples
- HTTP server bindings (Axum, Warp, etc.)

## 🏃 Quick Start

Add this to your `Cargo.toml`:

```toml
[dependencies]
scim-server = "0.1"
async-trait = "0.1"
tokio = { version = "1.0", features = ["full"] }
serde_json = "1.0"
```

### Basic Usage

```rust
use scim_server::{ScimServer, ResourceProvider, Resource, RequestContext};
use async_trait::async_trait;
use std::collections::HashMap;
use tokio::sync::RwLock;
use std::sync::Arc;

// Implement your data storage layer
struct MyProvider {
    users: Arc<RwLock<HashMap<String, Resource>>>,
}

#[derive(Debug, thiserror::Error)]
#[error("Storage error: {message}")]
struct StorageError {
    message: String,
}

#[async_trait]
impl ResourceProvider for MyProvider {
    type Error = StorageError;

    async fn create_user(
        &self,
        mut user: Resource,
        _context: &RequestContext,
    ) -> Result<Resource, Self::Error> {
        let id = uuid::Uuid::new_v4().to_string();
        user.set_attribute("id".to_string(), serde_json::Value::String(id.clone()));
        
        let mut users = self.users.write().await;
        users.insert(id, user.clone());
        Ok(user)
    }

    async fn get_user(
        &self,
        id: &str,
        _context: &RequestContext,
    ) -> Result<Option<Resource>, Self::Error> {
        let users = self.users.read().await;
        Ok(users.get(id).cloned())
    }

    async fn update_user(
        &self,
        id: &str,
        mut user: Resource,
        _context: &RequestContext,
    ) -> Result<Resource, Self::Error> {
        user.set_attribute("id".to_string(), serde_json::Value::String(id.to_string()));
        let mut users = self.users.write().await;
        users.insert(id.to_string(), user.clone());
        Ok(user)
    }

    async fn delete_user(
        &self,
        id: &str,
        _context: &RequestContext,
    ) -> Result<(), Self::Error> {
        let mut users = self.users.write().await;
        users.remove(id);
        Ok(())
    }

    async fn list_users(
        &self,
        _context: &RequestContext,
    ) -> Result<Vec<Resource>, Self::Error> {
        let users = self.users.read().await;
        Ok(users.values().cloned().collect())
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create your data provider
    let provider = MyProvider {
        users: Arc::new(RwLock::new(HashMap::new())),
    };

    // Build the SCIM server
    let server = ScimServer::builder()
        .with_resource_provider(provider)
        .build()?;

    // Create a user
    let user_data = serde_json::json!({
        "userName": "john.doe",
        "displayName": "John Doe",
        "emails": [{
            "value": "john@example.com",
            "type": "work",
            "primary": true
        }],
        "active": true
    });

    let context = RequestContext::new();
    let created_user = server.create_user(user_data, context.clone()).await?;
    println!("Created user: {}", created_user.get_id().unwrap());

    // List all users
    let users = server.list_users(context).await?;
    println!("Total users: {}", users.len());

    Ok(())
}
```

## 🏗️ Architecture

### Type-Safe State Machine

The server uses compile-time state checking to prevent invalid operations:

```rust
// Server starts in Uninitialized state
let builder = ScimServer::builder();

// Configure the server (still Uninitialized)
let configured_builder = builder.with_resource_provider(my_provider);

// Build transitions to Ready state
let server: ScimServer<Ready, MyProvider> = configured_builder.build()?;

// Only Ready servers can perform SCIM operations
let schemas = server.get_schemas().await?; // ✅ Compiles
```

### Resource Provider Trait

Implement the `ResourceProvider` trait to define your data access layer:

```rust
#[async_trait]
pub trait ResourceProvider: Send + Sync {
    type Error: std::error::Error + Send + Sync + 'static;

    async fn create_user(&self, user: Resource, context: &RequestContext) -> Result<Resource, Self::Error>;
    async fn get_user(&self, id: &str, context: &RequestContext) -> Result<Option<Resource>, Self::Error>;
    async fn update_user(&self, id: &str, user: Resource, context: &RequestContext) -> Result<Resource, Self::Error>;
    async fn delete_user(&self, id: &str, context: &RequestContext) -> Result<(), Self::Error>;
    async fn list_users(&self, context: &RequestContext) -> Result<Vec<Resource>, Self::Error>;
}
```

### Schema Validation

All resources are automatically validated against the SCIM User schema:

```rust
// ✅ Valid user - passes validation
let valid_user = serde_json::json!({
    "userName": "jdoe",  // Required field
    "emails": [{
        "value": "john@example.com",
        "type": "work"  // Must be from canonical values
    }]
});

// ❌ Invalid user - fails validation
let invalid_user = serde_json::json!({
    "displayName": "John Doe"
    // Missing required "userName" field
});
```

## 📚 Core Concepts

### Resources

Resources represent SCIM entities (Users, Groups, etc.). For the MVP, only User resources are supported:

```rust
let user = Resource::new("User".to_string(), user_data);

// Access common attributes
let id = user.get_id();
let username = user.get_username();
let emails = user.get_emails();
let is_active = user.is_active();

// Access any attribute
let display_name = user.get_attribute("displayName");
```

### Request Context

Provides contextual information for operations:

```rust
let context = RequestContext::new()
    .with_metadata("client_id".to_string(), "my-app".to_string());

// Use context for auditing, authorization, etc.
let user = provider.create_user(resource, &context).await?;
```

### Error Handling

Comprehensive error types with detailed information:

```rust
match server.create_user(invalid_data, context).await {
    Ok(user) => println!("Created: {}", user.get_id().unwrap()),
    Err(ScimError::Validation(e)) => eprintln!("Validation failed: {}", e),
    Err(ScimError::Provider(e)) => eprintln!("Storage error: {}", e),
    Err(e) => eprintln!("Other error: {}", e),
}
```

## 🔧 Configuration

### Service Provider Configuration

Configure server capabilities:

```rust
use scim_server::ServiceProviderConfig;

let config = ServiceProviderConfig {
    patch_supported: false,
    bulk_supported: false,
    filter_supported: false,
    change_password_supported: false,
    sort_supported: false,
    etag_supported: false,
    authentication_schemes: vec![],
    bulk_max_operations: None,
    bulk_max_payload_size: None,
    filter_max_results: Some(100),
};

let server = ScimServer::builder()
    .with_resource_provider(provider)
    .with_service_config(config)
    .build()?;
```

## 🧪 Testing

Run the test suite:

```bash
cargo test
```

Run the included example:

```bash
cargo run --example basic_usage
```

## 📖 Examples

The `examples/` directory contains:

- **basic_usage.rs**: Complete example showing all CRUD operations
- More examples coming in future releases

## 🛣️ Roadmap

### v0.2.0 - Groups and Extensions
- Group resource support
- Custom resource types
- Schema extensions

### v0.3.0 - Advanced Features  
- Filtering and search (SCIM filter expressions)
- Sorting and pagination
- PATCH operations

### v0.4.0 - Enterprise Features
- Bulk operations
- ETags and versioning
- Advanced authentication schemes

### v1.0.0 - Production Ready
- HTTP server integrations (Axum, Warp)
- Database connectors
- Performance optimizations
- Comprehensive documentation

## 🤝 Contributing

Contributions are welcome! Please see our [Contributing Guide](CONTRIBUTING.md) for details.

### Development Setup

```bash
git clone https://github.com/your-org/scim-server-rust
cd scim-server-rust
cargo test
cargo run --example basic_usage
```

## 📜 License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## 🙏 Acknowledgments

- [RFC 7643 - SCIM Core Schema](https://tools.ietf.org/html/rfc7643)
- [RFC 7644 - SCIM Protocol](https://tools.ietf.org/html/rfc7644)
- The Rust community for excellent async and web ecosystem

## 📞 Support

- 📚 [Documentation](https://docs.rs/scim-server)
- 🐛 [Issue Tracker](https://github.com/your-org/scim-server-rust/issues)
- 💬 [Discussions](https://github.com/your-org/scim-server-rust/discussions)

---

**Built with ❤️ in Rust**