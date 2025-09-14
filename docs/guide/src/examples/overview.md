# Examples Overview

This section provides practical examples demonstrating the key features and capabilities of the SCIM Server library. Each example is designed to showcase specific functionality with clear, executable code that you can run and modify.

## How to Use These Examples

All examples are located in the [`examples/` directory](https://github.com/pukeko37/scim-server/tree/main/examples) of the repository. Each example can be run directly:

```bash
# Basic examples
cargo run --example basic_usage
cargo run --example multi_tenant_example

# MCP examples (require mcp feature)
cargo run --example mcp_server_example --features mcp
```

## Example Categories

### 🚀 Core Examples
Learn the fundamental building blocks of SCIM server implementation:

- **[Basic Usage](./basic-usage.md)** - Essential CRUD operations with users and groups
- **[Multi-Tenant Server](./multi-tenant.md)** - Complete tenant isolation and management 
- **[Group Management](./group-management.md)** - Working with groups and member relationships

### 🔧 Advanced Features
Explore sophisticated server capabilities:

- **[ETag Concurrency Control](./etag-concurrency.md)** - Prevent data conflicts with version control
- **[Operation Handlers](./operation-handlers.md)** - Framework-agnostic request/response handling
- **[Builder Pattern Configuration](./builder-pattern.md)** - Flexible server setup and configuration

### 🤖 MCP Integration (AI Agents)
See how AI agents can interact with your SCIM server:

- **[MCP Server](./mcp-server.md)** - Expose SCIM operations as AI tools
- **[MCP with ETag Support](./mcp-etag.md)** - Version-aware AI operations
- **[Simple MCP Demo](./simple-mcp-demo.md)** - Quick AI integration setup
- **[MCP STDIO Server](./mcp-stdio-server.md)** - Standard I/O protocol server

### 🔐 Security & Authentication
Implement robust security patterns:

- **[Compile-Time Auth](./compile-time-auth.md)** - Type-safe authentication patterns
- **[Role-Based Access Control](./rbac.md)** - Advanced permission management

### 🛠️ Infrastructure & Operations
Production-ready operational patterns:

- **[Logging Backends](./logging-backends.md)** - Structured logging with multiple backends
- **[Logging Configuration](./logging-setup.md)** - Comprehensive logging setup
- **[Provider Modes](./provider-modes.md)** - Different provider implementation patterns
- **[Automated Capabilities](./automated-capabilities.md)** - Dynamic capability discovery

## Learning Path

**New to SCIM?** Start with [Basic Usage](./basic-usage.md) to understand core concepts.

**Building multi-tenant systems?** Progress to [Multi-Tenant Server](./multi-tenant.md) for isolation patterns.

**Adding AI capabilities?** Explore the MCP Integration examples starting with [Simple MCP Demo](./simple-mcp-demo.md).

**Production deployment?** Review [ETag Concurrency Control](./etag-concurrency.md) and logging examples.

## Running Examples

### Prerequisites
- Rust 1.75 or later
- Clone the [scim-server repository](https://github.com/pukeko37/scim-server)

### Basic Examples
```bash
cd scim-server
cargo run --example basic_usage
```

### MCP Examples
```bash
# Enable MCP feature for AI integration examples
cargo run --example mcp_server_example --features mcp
```

### Development Setup
```bash
# Run with logging to see detailed output
RUST_LOG=debug cargo run --example multi_tenant_example
```

## Key Concepts Demonstrated

Each example showcases different aspects of the SCIM Server library:

- **[`ScimServer`](https://docs.rs/scim-server/latest/scim_server/struct.ScimServer.html)** - Central orchestration component
- **[`ResourceProvider`](https://docs.rs/scim-server/latest/scim_server/trait.ResourceProvider.html)** - Business logic abstraction
- **[`StorageProvider`](https://docs.rs/scim-server/latest/scim_server/storage/trait.StorageProvider.html)** - Data persistence layer
- **[Multi-Tenant Context](https://docs.rs/scim-server/latest/scim_server/struct.TenantContext.html)** - Tenant isolation
- **[Schema System](https://docs.rs/scim-server/latest/scim_server/schema/index.html)** - Validation and extensions
- **[MCP Integration](https://docs.rs/scim-server/latest/scim_server/mcp_integration/index.html)** - AI agent support

## Related Documentation

- **[Getting Started Guide](../getting-started/first-server.md)** - Step-by-step tutorials
- **[Architecture Overview](../architecture.md)** - System design principles  
- **[API Reference](https://docs.rs/scim-server/latest/scim_server/)** - Complete API documentation

## Contributing Examples

Have an interesting use case or pattern? Examples are welcome! See the [contribution guidelines](https://github.com/pukeko37/scim-server/blob/main/CONTRIBUTING.md) for details on adding new examples.