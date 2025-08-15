# SCIM Server

[![Crates.io](https://img.shields.io/crates/v/scim-server.svg)](https://crates.io/crates/scim-server)
[![Documentation](https://docs.rs/scim-server/badge.svg)](https://docs.rs/scim-server)
[![Downloads](https://img.shields.io/crates/d/scim-server.svg)](https://crates.io/crates/scim-server)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Rust](https://img.shields.io/badge/rust-1.75+-blue.svg)](https://www.rust-lang.org)

A comprehensive **SCIM 2.0 server library** for Rust that makes identity provisioning simple, type-safe, and enterprise-ready.

**SCIM (System for Cross-domain Identity Management)** is the industry standard for automating user provisioning between identity providers and applications.

> **⚠️ Development Status**: This library is under active development. Pin to exact versions for stability: `scim-server = "=0.3.2"`. Breaking changes are signaled by minor version increments until v1.0.

## ⚡ Quick Start

Add to your `Cargo.toml`:

```toml
[dependencies]
scim-server = "=0.3.2"
tokio = { version = "1.0", features = ["full"] }
serde_json = "1.0"
```

Create a basic SCIM server:

```rust
use scim_server::{ScimServer, storage::InMemoryStorage};
use serde_json::json;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create server with in-memory storage
    let storage = InMemoryStorage::new();
    let server = ScimServer::new(storage).await?;
    
    // Create a user
    let user_data = json!({
        "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
        "userName": "john.doe",
        "emails": [{"value": "john@example.com", "primary": true}]
    });
    
    let user = server.create_user("tenant-1", user_data).await?;
    println!("Created user: {}", user.id);
    
    Ok(())
}
```

## ✨ Key Features

- 🛡️ **Type-Safe by Design** - Leverage Rust's type system to prevent runtime errors
- 🏢 **Multi-Tenant Ready** - Built-in support for multiple organizations/tenants  
- 📋 **Full SCIM 2.0 Compliance** - Complete implementation of RFC 7643 and RFC 7644
- ⚡ **High Performance** - Async-first with minimal overhead
- 🔌 **Framework Agnostic** - Works with Axum, Warp, Actix, or any HTTP framework
- 🤖 **AI-Ready** - Built-in Model Context Protocol for AI tool integration
- 🔄 **ETag Concurrency Control** - Prevents lost updates in multi-client scenarios

## 🏗️ How It Works

The SCIM Server acts as intelligent middleware that handles provisioning complexity:

**Client Applications** → **SCIM Server** → **Your Storage Backend**

- **Clients**: Web apps, AI assistants, CLI tools, custom integrations
- **SCIM Server**: Validation, schema management, multi-tenancy, concurrency control
- **Storage**: In-memory, database, cloud, or custom providers

## 📚 Documentation

| Resource | Description |
|----------|-------------|
| 📖 **[User Guide](docs/guide/book/)** | Comprehensive tutorials and concepts |
| 🔧 **[API Documentation](https://docs.rs/scim-server)** | Detailed API reference with examples |
| 💡 **[Examples](examples/)** | Copy-paste starting points for common use cases |
| 📋 **[CHANGELOG](CHANGELOG.md)** | Version history and migration guides |

### Learning Path

1. **Start Here**: Follow the Quick Start above
2. **Learn Concepts**: Read the [User Guide](docs/guide/book/) 
3. **See Examples**: Browse [examples/](examples/) for your use case
4. **API Reference**: Check [docs.rs](https://docs.rs/scim-server) for detailed API docs

## 🚀 Common Use Cases

```rust
// Multi-tenant server
let server = ScimServer::new(storage)
    .with_tenant("org1")
    .await?;

// Custom resource types
server.register_schema(custom_schema).await?;

// Web framework integration (Axum example)
let app = Router::new()
    .route("/scim/v2/Users", post(create_user))
    .layer(Extension(server));
```

See [examples/](examples/) for complete working examples including:
- Basic CRUD operations
- Multi-tenant setups
- Web framework integrations
- Authentication patterns
- ETag concurrency control
- AI assistant integration

## 🤝 Contributing

We welcome contributions! Please see:

- [Contributing Guide](docs/reference/development/contributing.md)
- [Development Setup](docs/reference/development/setup.md)
- [Code Standards](docs/reference/development/standards.md)

## 📄 License

Licensed under the [MIT License](LICENSE).

---

**Need help?** Check the [User Guide](docs/guide/book/) or [open an issue](https://github.com/pukeko37/scim-server/issues).