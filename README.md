# SCIM Server

[![Crates.io](https://img.shields.io/crates/v/scim-server.svg)](https://crates.io/crates/scim-server)
[![Documentation](https://docs.rs/scim-server/badge.svg)](https://docs.rs/scim-server)
[![Downloads](https://img.shields.io/crates/d/scim-server.svg)](https://crates.io/crates/scim-server)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Rust](https://img.shields.io/badge/rust-1.75+-blue.svg)](https://www.rust-lang.org)

A comprehensive **SCIM 2.0 server library** for Rust that makes identity provisioning simple, type-safe, and enterprise-ready. SCIM (System for Cross-domain Identity Management) is the industry standard for automating user provisioning between identity providers and applications.

## 📢 Notice Board

| | |
|---|---|
| **Current Version** | `0.5.3` |
| **Latest Changes** | Major documentation expansion with Architecture Deep Dives, concept guides, and streamlined README |
| **Stability** | Pre-1.0 development. Minor version increments indicate breaking changes. Pin to exact versions (`scim-server = "=0.5.3"`) for production use |
| **Breaking Changes** | Minor version increments signal breaking changes until v1.0. See [CHANGELOG](CHANGELOG.md) for migration guides |

## Quick Start

Get up and running in minutes with our [Getting Started Guide](https://pukeko37.github.io/scim-server/getting-started/installation.html).

## Key Features

- **Type-Safe by Design** - Leverage Rust's type system to prevent runtime errors
- **Multi-Tenant Ready** - Built-in support for multiple organizations/tenants  
- **Full SCIM 2.0 Compliance** - Complete implementation of RFC 7643 and RFC 7644
- **High Performance** - Async-first architecture with minimal overhead
- **Framework Agnostic** - Works with Axum, Warp, Actix, or any HTTP framework
- **AI-Ready** - Built-in Model Context Protocol (MCP) for AI tool integration
- **ETag Concurrency Control** - Prevents lost updates in multi-client scenarios
- **Enterprise Grade** - Production-ready with comprehensive error handling and logging

## Architecture

**Client Applications** → **SCIM Server** → **Your Storage Backend**

The SCIM Server acts as intelligent middleware that handles provisioning complexity, validation, schema management, multi-tenancy, and concurrency control while you focus on your storage implementation.

## Documentation

| Resource | Description |
|----------|-------------|
| **[📚 User Guide](https://pukeko37.github.io/scim-server/)** | Comprehensive tutorials, concepts, and integration patterns |
| **[🔧 API Reference](https://docs.rs/scim-server/latest/scim_server/)** | Detailed API documentation with examples |
| **[💡 Examples](examples/)** | Ready-to-run code examples for common use cases |
| **[📝 Changelog](CHANGELOG.md)** | Version history and migration guides |

### Learning Path

1. **[Installation](https://pukeko37.github.io/scim-server/getting-started/installation.html)** - Get started in minutes
2. **[Your First SCIM Server](https://pukeko37.github.io/scim-server/getting-started/first-server.html)** - Build a basic server
3. **[Core Concepts](https://pukeko37.github.io/scim-server/concepts/operation-handlers.html)** - Understand the architecture
4. **[Examples](examples/)** - Explore real-world implementations

## Common Use Cases

Browse our [examples directory](examples/) for complete implementations:

- **[Basic Usage](examples/basic_usage.rs)** - Simple CRUD operations
- **[Multi-Tenant](examples/multi_tenant_example.rs)** - Enterprise tenant isolation
- **[Group Management](examples/group_example.rs)** - Managing groups and membership
- **[ETag Concurrency](examples/etag_concurrency_example.rs)** - Preventing lost updates
- **[AI Integration](examples/mcp_server_example.rs)** - MCP server for AI agents
- **[Authentication](examples/compile_time_auth_example.rs)** - Type-safe auth patterns



## Contributing

We welcome contributions! Please:

1. Check existing [issues](https://github.com/pukeko37/scim-server/issues) or create a new one
2. Read our [User Guide](https://pukeko37.github.io/scim-server/) for development information
3. Follow our [examples](examples/) for code style and patterns

## License

Licensed under the [MIT License](LICENSE).

---

**Need help?** Check the [User Guide](https://pukeko37.github.io/scim-server/) or [open an issue](https://github.com/pukeko37/scim-server/issues).