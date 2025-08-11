# SCIM Server

[![Crates.io](https://img.shields.io/crates/v/scim-server.svg)](https://crates.io/crates/scim-server)
[![Documentation](https://docs.rs/scim-server/badge.svg)](https://docs.rs/scim-server)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Rust](https://img.shields.io/badge/rust-1.75+-blue.svg)](https://www.rust-lang.org)

A comprehensive **SCIM 2.0 server library** for Rust that makes identity provisioning simple, type-safe, and production-ready.

> **SCIM (System for Cross-domain Identity Management)** is the industry standard for automating user provisioning between identity providers and applications. Think automatic user onboarding/offboarding across your entire tech stack.

## 🏗️ How It Works: Client → Server → Provider Architecture

The SCIM Server acts as an **intelligent middleware** that handles all provisioning complexity so your applications don't have to:

### **Multiple Ways to Connect**

Connect any type of client through standardized interfaces:

- **🌐 Web Applications** - REST APIs for admin portals, user dashboards, and sync tools
- **🤖 AI Assistants** - Natural language provisioning via Model Context Protocol (Claude, ChatGPT, custom bots)
- **⚡ Automation Tools** - CLI scripts for bulk imports, migrations, and DevOps pipelines  
- **🔧 Custom Integrations** - GraphQL, gRPC, message queues, webhooks, or any protocol you need

### **The Intelligence Layer**

The SCIM Server core provides enterprise-grade capabilities that would take months to build yourself:

- **📋 Dynamic Schema Management** - Define custom resource types with automatic validation
- **🛡️ Type-Safe Validation** - Comprehensive error checking with detailed reporting
- **⚙️ Standardized Operations** - Consistent CRUD, filtering, and bulk operations across all resources
- **🏢 Multi-Tenant Architecture** - Built-in organization isolation and configuration management
- **🔍 Automatic Capabilities** - Self-documenting API features and service provider configuration

### **Flexible Storage Backend**

Choose your data storage strategy without changing your application code:

- **🚀 Development** - In-memory providers for testing and prototyping
- **🏢 Production** - Database providers with full ACID compliance
- **☁️ Cloud-Native** - Custom providers for S3, DynamoDB, or any storage system
- **🔄 Multi-Tenant** - Automatic tenant isolation with shared or dedicated infrastructure
- **🏷️ ETag Concurrency Control** - Built-in optimistic locking prevents lost updates

### 💡 **Value Proposition: Offload Complexity from Your SaaS**

Instead of building provisioning logic into every Rust application:

| **Without SCIM Server** | **With SCIM Server** |
|-------------------------|----------------------|
| ❌ Custom validation in each app | ✅ **Centralized validation engine** |
| ❌ Manual concurrency control | ✅ **Automatic ETag versioning with optimistic locking** |
| ❌ Manual schema management | ✅ **Dynamic schema registry** |
| ❌ Ad-hoc API endpoints | ✅ **Standardized SCIM protocol** |
| ❌ Reinvent capability discovery | ✅ **Automatic capability construction** |
| ❌ Build multi-tenancy from scratch | ✅ **Built-in tenant isolation** |
| ❌ Custom error handling per resource | ✅ **Consistent error semantics with conflict resolution** |
| ❌ Lost updates in concurrent scenarios | ✅ **Version conflict detection and prevention** |

**Result**: Your SaaS applications focus on business logic while the SCIM server handles all provisioning complexity with enterprise-grade reliability.

## ✨ Why Choose This Library?

- 🛡️ **Type-Safe by Design** - Leverage Rust's type system to prevent runtime errors
- 🏢 **Multi-Tenant Ready** - Built-in support for multiple organizations/tenants
- 📋 **Full SCIM 2.0 Compliance** - Comprehensive implementation of RFC 7643 and RFC 7644
- ⚡ **High Performance** - Async-first with minimal overhead
- 🔌 **Framework Agnostic** - Works with any HTTP framework (Axum, Warp, Actix, etc.)
- 🧩 **Provider Flexibility** - In-memory, database, or custom backends
- 🤖 **AI-Ready with MCP** - Built-in Model Context Protocol for AI tool integration
- 🎯 **Beyond Users & Groups** - Extensible schema system for any resource type
- 🔄 **ETag Concurrency Control** - Optimistic locking prevents lost updates in multi-client scenarios
- 🧵 **Thread-Safe Operations** - Concurrent access safety with atomic version checking
- 📖 **Production Ready** - Extensive testing (827 tests), logging, and error handling

## 🚀 Quick Start

Add to your `Cargo.toml`:

```toml
[dependencies]
scim-server = "0.2.0"
tokio = { version = "1.0", features = ["full"] }
serde_json = "1.0"
```

### Minimal Example

```rust
use scim_server::{ScimServer, providers::InMemoryProvider, resource::RequestContext};
use serde_json::json;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create a SCIM server with in-memory storage
    let provider = InMemoryProvider::new();
    let server = ScimServer::new(provider)?;
    
    // Create a user with automatic ETag versioning
    let context = RequestContext::with_generated_id();
    let user_data = json!({
        "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
        "userName": "john.doe@example.com",
        "active": true
    });
    
    let versioned_user = server.provider()
        .create_versioned_resource("User", user_data, &context)
        .await?;
    
    println!("Created user with ETag: {}", versioned_user.version().to_http_header());
    Ok(())
}
```

### Complete HTTP Server Example

```rust
use scim_server::{ScimServer, InMemoryProvider, ScimUser};
use serde_json::json;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let provider = InMemoryProvider::new();
    let server = ScimServer::new(provider);

    // Create a user
    let user_data = json!({
        "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
        "userName": "alice@example.com",
        "name": {
            "givenName": "Alice",
            "familyName": "Smith"
        },
        "emails": [{
            "value": "alice@example.com",
            "primary": true
        }]
    });

    let user = server.create_user("tenant-1", user_data).await?;
    println!("Created user: {}", user.id);

    // Server integrates with your HTTP framework of choice
    // See examples/ for Axum, Warp, and Actix integrations
    Ok(())
}
```

## 🎯 Key Features

### Core SCIM 2.0 Support
- ✅ **Users & Groups** - Full lifecycle management (CRUD operations)
- ✅ **Schema Validation** - Automatic validation against SCIM schemas
- ✅ **Filtering & Pagination** - Efficient queries with SCIM filter syntax
- ✅ **Bulk Operations** - Handle multiple operations in a single request
- ✅ **Patch Operations** - Granular updates with RFC 6902 JSON Patch

### Advanced Capabilities
- 🏗️ **Multi-Tenant Architecture** - Isolate data between organizations
- 🔍 **Automatic Discovery** - Service provider configuration and schema endpoints
- 🎛️ **Provider Capabilities** - Automatic feature detection and advertisement
- 📝 **Comprehensive Logging** - Structured logging with multiple backends
- 🔧 **Value Objects** - Type-safe domain modeling with compile-time validation

### 🔄 ETag Concurrency Control (NEW in 0.2.0)

**Production-Grade Optimistic Locking** - Prevent lost updates in multi-client environments:

```rust
use scim_server::{ScimServer, providers::InMemoryProvider, resource::RequestContext};
use serde_json::json;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let provider = InMemoryProvider::new();
    let server = ScimServer::new(provider)?;
    let context = RequestContext::with_generated_id();

    // Create user with automatic versioning
    let user_data = json!({
        "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
        "userName": "alice@example.com",
        "active": true
    });
    
    let versioned_user = server.provider()
        .create_versioned_resource("User", user_data, &context)
        .await?;
    
    println!("User ETag: {}", versioned_user.version().to_http_header());
    // Output: User ETag: W/"abc123def456"

    // Conditional update - only succeeds if version matches
    let update_data = json!({"active": false});
    let current_version = versioned_user.version();
    
    match server.provider()
        .conditional_update("User", "123", update_data, current_version, &context)
        .await? 
    {
        ConditionalResult::Success(updated) => {
            println!("Update successful! New ETag: {}", updated.version().to_http_header());
        },
        ConditionalResult::VersionMismatch(conflict) => {
            println!("Version conflict detected!");
            println!("Expected: {}, Current: {}", conflict.expected, conflict.current);
            // Handle conflict: refresh, merge, or retry
        },
        ConditionalResult::NotFound => {
            println!("Resource no longer exists");
        }
    }
    
    Ok(())
}
```

**ETag Features:**
- 🔒 **Weak ETags** - Semantic equivalence versioning (`W/"version"`)
- ⚡ **Atomic Operations** - Thread-safe version checking and updates
- 🤖 **AI Agent Safe** - MCP integration with conflict resolution workflows
- 🏢 **Multi-Tenant** - Version isolation across tenant boundaries
- 📊 **Conflict Resolution** - Structured error responses with resolution guidance

### Framework Integration
- 🌐 **HTTP Framework Agnostic** - Bring your own web framework
- 🔌 **Operation Handler Foundation** - Clean abstraction for SCIM operations
- 🤖 **MCP Integration** - Model Context Protocol support for AI tools

### 🤖 AI-Powered Identity Management

**Built-in MCP (Model Context Protocol) Support** - Connect AI assistants directly to your identity data:

```rust
use scim_server::{McpServer, ScimServer, InMemoryProvider};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let provider = InMemoryProvider::new();
    let scim_server = ScimServer::new(provider);
    
    // Enable MCP for AI tool integration
    let mcp_server = McpServer::new(scim_server);
    mcp_server.start("stdio").await?;
    
    // Now AI assistants can:
    // - Query users: "Find all users in the engineering department"
    // - Manage groups: "Add Alice to the admin group" 
    // - Audit access: "Who has access to the finance system?"
    // - Automate onboarding: "Create accounts for new hire John Doe"
    
    Ok(())
}
```

**AI Use Cases Enabled:**
- 🔍 **Intelligent Queries** - Natural language user/group searches
- ⚡ **Automated Provisioning** - AI-driven user onboarding/offboarding
- 🛡️ **Security Auditing** - AI-powered access reviews and compliance checks
- 📊 **Identity Analytics** - Smart insights into user patterns and group dynamics
- 🤝 **Conversational Admin** - Chat-based identity management operations

**MCP Tools Provided:**
- `list_users` - Query users with natural language filters
- `create_user` - Provision new users with AI validation
- `manage_groups` - Intelligent group membership management
- `audit_access` - Security and compliance reporting
- `bulk_operations` - AI-optimized batch processing

Perfect for building AI-enhanced admin dashboards, chatbots, and automated identity workflows!

### 🎯 Beyond Identity: Custom Resource Management

**SCIM isn't just for users and groups** - it's a powerful foundation for managing ANY structured resource:

```rust
use scim_server::{ScimServer, CustomSchema, ResourceType};
use serde_json::json;

// Define custom schemas for your domain
let device_schema = CustomSchema::builder()
    .id("urn:example:schemas:Device")
    .add_attribute("serialNumber", AttributeType::String, true)
    .add_attribute("manufacturer", AttributeType::String, false)
    .add_attribute("location", AttributeType::Complex, false)
    .add_sub_attribute("location", "building", AttributeType::String)
    .add_sub_attribute("location", "room", AttributeType::String)
    .build();

let mut server = ScimServer::new(provider);
server.register_schema(device_schema)?;

// Now manage devices with full SCIM capabilities
let device = server.create_resource("devices", json!({
    "schemas": ["urn:example:schemas:Device"],
    "serialNumber": "DEV-001",
    "manufacturer": "Acme Corp",
    "location": {
        "building": "HQ",
        "room": "Server Room A"
    }
})).await?;
```

**Real-World Custom Schema Use Cases:**

| Domain | Schema Example | Business Value |
|--------|----------------|----------------|
| 🖥️ **IT Asset Management** | Devices, Software Licenses, Certificates | Automated asset lifecycle, compliance tracking |
| 🏢 **Facility Management** | Rooms, Equipment, Access Cards | Smart building automation, space optimization |
| 📚 **Learning Management** | Courses, Certifications, Learning Paths | Skill tracking, compliance training automation |
| 🔐 **Access Control** | Permissions, Roles, Entitlements | Fine-grained authorization, audit trails |
| 💼 **Business Resources** | Projects, Budgets, Approvals | Workflow automation, resource allocation |
| 🌐 **Cloud Resources** | VMs, Databases, Storage Buckets | Infrastructure as Code, cost management |

**Why SCIM for Custom Resources?**
- ✅ **Standardized API** - Consistent CRUD, filtering, and bulk operations
- ✅ **Schema Validation** - Type safety and data integrity out of the box  
- ✅ **Multi-Tenant Ready** - Isolate resources by organization/tenant
- ✅ **Audit & Compliance** - Built-in change tracking and versioning
- ✅ **AI Integration** - Custom resources become AI-queryable via MCP
- ✅ **Enterprise Integration** - Standard protocol for system interoperability

**Transform any data model into a fully-featured API with enterprise-grade capabilities!**

## 📚 Examples

| Example | Description |
|---------|-------------|
| [`basic_server`](examples/basic_usage.rs) | Simple SCIM server setup |
| [`multi_tenant`](examples/multi_tenant_example.rs) | Multi-organization support |
| [`custom_provider`](examples/provider_modes.rs) | Custom storage backends |
| [`mcp_integration`](examples/mcp_server_example.rs) | AI assistant integration via MCP |
| [`custom_schemas`](examples/) | Define and manage custom resource types |
| [`web_framework`](examples/) | Integration with Axum/Warp/Actix |
| [`bulk_operations`](examples/) | Handling bulk SCIM requests |

## 🏗️ Architecture

```
┌─────────────────┐    ┌──────────────────┐    ┌─────────────────┐
│   HTTP Layer    │    │   SCIM Server    │    │   Provider      │
│                 │    │                  │    │                 │
│  • Axum        │───▶│  • Validation    │───▶│  • In-Memory    │
│  • Warp        │    │  • Operations    │    │  • Database     │
│  • Actix       │    │  • Multi-tenant  │    │  • Custom       │
│  • Custom      │    │  • Type Safety   │    │                 │
└─────────────────┘    └──────────────────┘    └─────────────────┘
```

The library provides a clean separation between:
- **HTTP handling** (your choice of framework)
- **SCIM logic** (validation, operations, multi-tenancy)
- **Data storage** (pluggable providers)

## 📖 Documentation

| Resource | Description |
|----------|-------------|
| [API Documentation](https://docs.rs/scim-server) | Complete API reference |
| [Roadmap](ROADMAP.md) | Feature roadmap and future releases |
| [User Guide](docs/guides/user-guide.md) | Step-by-step tutorials |
| [Architecture Guide](docs/guides/architecture.md) | Design decisions and patterns |
| [SCIM Compliance](docs/reference/scim-compliance.md) | RFC 7644 implementation details |
| [Multi-Tenancy](docs/api/multi-tenancy.md) | Multi-tenant setup and usage |

## 🛠️ Development

```bash
# Clone the repository
git clone https://github.com/pukeko37/scim-server.git
cd scim-server

# Run tests
cargo test

# Run with logging
RUST_LOG=debug cargo run --example basic_usage

# Generate documentation
cargo doc --open
```

### Testing
- **100%** documentation test coverage
- **Comprehensive** integration test suite
- **Multi-tenant** validation scenarios
- **Performance** benchmarks included

## 🤝 Contributing

We welcome contributions! Here's how you can help:

1. 🐛 **Report bugs** via GitHub Issues
2. 💡 **Suggest features** or improvements
3. 📖 **Improve documentation**
4. 🔧 **Submit pull requests**

See [CONTRIBUTING.md](CONTRIBUTING.md) for detailed guidelines.

### Development Principles
- **Type safety first** - Leverage Rust's type system
- **YAGNI compliance** - Build only what's needed now
- **Functional patterns** - Immutable data and pure functions
- **Comprehensive testing** - Every feature thoroughly tested

## 🏆 Production Ready

This library is designed for production use with:

- ✅ **Extensive error handling** with detailed error types
- ✅ **Performance optimizations** and benchmarking
- ✅ **Memory safety** guaranteed by Rust
- ✅ **Concurrent access** patterns handled safely
- ✅ **Logging integration** for observability
- ✅ **Documentation** for all public APIs

## 📋 SCIM 2.0 Compliance

| Feature | Status | RFC Section |
|---------|--------|-------------|
| User Resources | ✅ Complete | RFC 7643 §4.1 |
| Group Resources | ✅ Complete | RFC 7643 §4.2 |
| Schema Discovery | ✅ Complete | RFC 7644 §4 |
| Resource CRUD | ✅ Complete | RFC 7644 §3.2-3.5 |
| Filtering | ✅ Complete | RFC 7644 §3.4.2.2 |
| Bulk Operations | ✅ Complete | RFC 7644 §3.7 |
| Patch Operations | ✅ Complete | RFC 7644 §3.5.2 |

**94% SCIM 2.0 Compliance** - See [compliance report](SCIM_2_0_COMPLIANCE_SUMMARY.md) for details.

## 📝 License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## 🙏 Acknowledgments

- [SCIM 2.0 Specification](https://tools.ietf.org/html/rfc7644) - The foundation this library implements
- [Rust Community](https://www.rust-lang.org/community) - For the amazing ecosystem and support

---

**Ready to get started?** Check out the [Quick Start Guide](docs/guides/quick-start.md) or browse the [examples](examples/).
