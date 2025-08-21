# SCIM Server

[![Crates.io](https://img.shields.io/crates/v/scim-server.svg)](https://crates.io/crates/scim-server)
[![Documentation](https://docs.rs/scim-server/badge.svg)](https://docs.rs/scim-server)
[![Downloads](https://img.shields.io/crates/d/scim-server.svg)](https://crates.io/crates/scim-server)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Rust](https://img.shields.io/badge/rust-1.75+-blue.svg)](https://www.rust-lang.org)

A comprehensive **SCIM 2.0 server library** for Rust that makes identity provisioning simple, type-safe, and enterprise-ready.

**SCIM (System for Cross-domain Identity Management)** is the industry standard for automating user provisioning between identity providers and applications.

> **Development Status**: This library is under active development. Pin to exact versions for stability: `scim-server = "=0.3.10"`. Breaking changes are signaled by minor version increments until v1.0.

## Quick Start

Add to your `Cargo.toml`:

```toml
[dependencies]
scim-server = "=0.3.10"
tokio = { version = "1.0", features = ["full"] }
serde_json = "1.0"
```

Create a basic SCIM server:

```rust
use scim_server::{
    RequestContext,
    providers::StandardResourceProvider,
    storage::InMemoryStorage,
    resource::provider::ResourceProvider,
};
use serde_json::json;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create StandardResourceProvider with InMemoryStorage
    let storage = InMemoryStorage::new();
    let provider = StandardResourceProvider::new(storage);
    
    // Create request context
    let context = RequestContext::new("example-request-1".to_string());
    
    // Create a user
    let user_data = json!({
        "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
        "userName": "john.doe",
        "emails": [{"value": "john@example.com", "primary": true}]
    });
    
    let user = provider.create_resource("User", user_data, &context).await?;
    println!("Created user: {}", user.id);
    
    Ok(())
}
```

## Key Features

- **Type-Safe by Design** - Leverage Rust's type system to prevent runtime errors
- **Multi-Tenant Ready** - Built-in support for multiple organizations/tenants  
- **Full SCIM 2.0 Compliance** - Complete implementation of RFC 7643 and RFC 7644
- **High Performance** - Async-first with minimal overhead
- **Framework Agnostic** - Works with Axum, Warp, Actix, or any HTTP framework
- **AI-Ready** - Built-in Model Context Protocol for AI tool integration
- **ETag Concurrency Control** - Prevents lost updates in multi-client scenarios

## How It Works

The SCIM Server acts as intelligent middleware that handles provisioning complexity:

**Client Applications** → **SCIM Server** → **Your Storage Backend**

- **Clients**: Web apps, AI assistants, CLI tools, custom integrations
- **SCIM Server**: Validation, schema management, multi-tenancy, concurrency control
- **Storage**: In-memory, database, cloud, or custom providers

## Documentation

| Resource | Description |
|----------|-------------|
| [User Guide](https://pukeko37.github.io/scim-server/) | Comprehensive tutorials and concepts |
| [API Documentation](https://docs.rs/scim-server/latest/scim_server/) | Detailed API reference with examples |
| [Examples](examples/) | Copy-paste starting points for common use cases |
| [CHANGELOG](CHANGELOG.md) | Version history and migration guides |

### Learning Path

1. **Start Here**: Follow the Quick Start above
2. **Learn Concepts**: Read the [User Guide](https://pukeko37.github.io/scim-server/) 
3. **See Examples**: Browse [examples/](examples/) for your use case
4. **API Reference**: Check [docs.rs](https://docs.rs/scim-server/latest/scim_server/) for detailed API docs

## Common Use Cases

```rust
use scim_server::{
    ScimServer,
    providers::StandardResourceProvider,
    storage::InMemoryStorage,
};

// Multi-tenant server with StandardResourceProvider
let storage = InMemoryStorage::new();
let provider = StandardResourceProvider::new(storage);
let mut server = ScimServer::new(provider)?;

// Register tenant
server.register_tenant("org1").await?;

// Custom resource types
server.register_schema(custom_schema).await?;

// Web framework integration (Axum example)
let app = Router::new()
    .route("/scim/v2/Users", post(create_user))
    .layer(Extension(server));
```

See [examples/](examples/) for complete working examples including:
- Basic CRUD operations with `StandardResourceProvider`
- Multi-tenant setups
- Web framework integrations
- Authentication patterns
- ETag concurrency control
- AI assistant integration

## Migration from InMemoryProvider

If you're upgrading from the deprecated `InMemoryProvider`, update your code:

```rust
// Old (deprecated)
use scim_server::providers::InMemoryProvider;
let provider = InMemoryProvider::new();

// New (current)
use scim_server::{
    providers::StandardResourceProvider,
    storage::InMemoryStorage,
};
let storage = InMemoryStorage::new();
let provider = StandardResourceProvider::new(storage);
```

## Contributing

We welcome contributions! Please see our [User Guide](https://pukeko37.github.io/scim-server/) for development information, or [open an issue](https://github.com/pukeko37/scim-server/issues) to discuss your ideas.

## License

Licensed under the [MIT License](LICENSE).

---

**Need help?** Check the [User Guide](https://pukeko37.github.io/scim-server/) or [open an issue](https://github.com/pukeko37/scim-server/issues).