# SCIM Server

[![Crates.io](https://img.shields.io/crates/v/scim-server.svg)](https://crates.io/crates/scim-server)
[![Documentation](https://docs.rs/scim-server/badge.svg)](https://docs.rs/scim-server)
[![Downloads](https://img.shields.io/crates/d/scim-server.svg)](https://crates.io/crates/scim-server)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Rust](https://img.shields.io/badge/rust-1.75+-blue.svg)](https://www.rust-lang.org)

A comprehensive **SCIM 2.0 server library** for Rust that makes identity provisioning simple, type-safe, and enterprise-ready.

**SCIM (System for Cross-domain Identity Management)** is the industry standard for automating user provisioning between identity providers and applications.

> **Development Status**: This library is under active development. Pin to exact versions for stability: `scim-server = "=0.5.2"`. Breaking changes are signaled by minor version increments until v1.0.

## 🚨 v0.5.0 Breaking Changes

**Provider Interface Refactored**: Major simplification through helper traits with method renames:

```rust
// Before v0.5.0
provider.conditional_update(resource_type, id, data, version, context).await?;
provider.conditional_delete(resource_type, id, version, context).await?;

// v0.5.0+ (current) 
provider.conditional_update_resource(resource_type, id, data, version, context).await?;
provider.conditional_delete_resource(resource_type, id, version, context).await?;
```

**Architecture Improvements**: `StandardResourceProvider` simplified by ~500 lines through helper trait composition while maintaining full functionality.

## Quick Start

Add to your `Cargo.toml`:

```toml
[dependencies]
scim-server = "=0.5.2"
tokio = { version = "1.0", features = ["full"] }
serde_json = "1.0"
```

Create a basic SCIM server:

```rust
use scim_server::{
    ScimServer,
    providers::StandardResourceProvider,
    storage::InMemoryStorage,
    resource_handlers::{create_user_resource_handler, create_group_resource_handler},
    multi_tenant::ScimOperation,
    RequestContext,
};
use serde_json::json;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create storage and provider
    let storage = InMemoryStorage::new();
    let provider = StandardResourceProvider::new(storage);
    let mut server = ScimServer::new(provider)?;

    // Register User resource type with schema validation
    let user_schema = server
        .get_schema_by_id("urn:ietf:params:scim:schemas:core:2.0:User")
        .expect("User schema should exist").clone();
    let user_handler = create_user_resource_handler(user_schema);
    server.register_resource_type("User", user_handler,
        vec![ScimOperation::Create, ScimOperation::Read])?;

    // Create request context
    let context = RequestContext::new("example-request-1".to_string());

    // Create a user with full SCIM compliance
    let user_data = json!({
        "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
        "userName": "john.doe",
        "emails": [{"value": "john@example.com", "primary": true}]
    });

    let user_json = server.create_resource_with_refs("User", user_data, &context).await?;
    println!("Created user: {}", user_json["userName"]);

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
- **Comprehensive Documentation** - Detailed concept guides and integration examples

## Documentation

**📚 New in v0.5.1**: Comprehensive concept documentation with 2,100+ lines covering:

- **[Operation Handlers](https://docs.rs/scim-server/latest/scim_server/)** - Framework-agnostic integration for HTTP, MCP, CLI, and custom protocols
- **[MCP Integration](https://docs.rs/scim-server/latest/scim_server/)** - AI-native interface with tool discovery for conversational identity management  
- **[SCIM Server Architecture](https://docs.rs/scim-server/latest/scim_server/)** - Dynamic resource management with multi-tenant support
- **[Multi-Tenant Patterns](https://docs.rs/scim-server/latest/scim_server/)** - Enterprise deployment strategies for SaaS and compliance scenarios

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
    ScimServer, ScimServerBuilder, TenantStrategy,
    providers::StandardResourceProvider,
    storage::InMemoryStorage,
    resource_handlers::{create_user_resource_handler, create_group_resource_handler},
    multi_tenant::ScimOperation,
};

// Multi-tenant server with proper configuration
let storage = InMemoryStorage::new();
let provider = StandardResourceProvider::new(storage);
let mut server = ScimServerBuilder::new(provider)
    .with_base_url("https://api.company.com")
    .with_tenant_strategy(TenantStrategy::PathBased)
    .build()?;

// Register User and Group resource types
let user_schema = server.get_schema_by_id("urn:ietf:params:scim:schemas:core:2.0:User")?;
let user_handler = create_user_resource_handler(user_schema.clone());
server.register_resource_type("User", user_handler,
    vec![ScimOperation::Create, ScimOperation::Read, ScimOperation::Update])?;

let group_schema = server.get_schema_by_id("urn:ietf:params:scim:schemas:core:2.0:Group")?;
let group_handler = create_group_resource_handler(group_schema.clone());
server.register_resource_type("Group", group_handler,
    vec![ScimOperation::Create, ScimOperation::Read])?;

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

## Storage Backends

The recommended approach is to use `ScimServer` with `StandardResourceProvider` and pluggable storage:

```rust
use scim_server::{
    ScimServer,
    providers::StandardResourceProvider,
    storage::InMemoryStorage,
    resource_handlers::{create_user_resource_handler},
    multi_tenant::ScimOperation,
};

let storage = InMemoryStorage::new();
let provider = StandardResourceProvider::new(storage);
let mut server = ScimServer::new(provider)?;

// Register resource types for full SCIM compliance
let user_schema = server.get_schema_by_id("urn:ietf:params:scim:schemas:core:2.0:User")?;
let user_handler = create_user_resource_handler(user_schema.clone());
server.register_resource_type("User", user_handler,
    vec![ScimOperation::Create, ScimOperation::Read, ScimOperation::Update])?;
```

## Contributing

We welcome contributions! Please see our [User Guide](https://pukeko37.github.io/scim-server/) for development information, or [open an issue](https://github.com/pukeko37/scim-server/issues) to discuss your ideas.

## License

Licensed under the [MIT License](LICENSE).

---

**Need help?** Check the [User Guide](https://pukeko37.github.io/scim-server/) or [open an issue](https://github.com/pukeko37/scim-server/issues).
