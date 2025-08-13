# SCIM Server

[![Crates.io](https://img.shields.io/crates/v/scim-server.svg)](https://crates.io/crates/scim-server)
[![Documentation](https://docs.rs/scim-server/badge.svg)](https://docs.rs/scim-server)
[![Downloads](https://img.shields.io/crates/d/scim-server.svg)](https://crates.io/crates/scim-server)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Rust](https://img.shields.io/badge/rust-1.75+-blue.svg)](https://www.rust-lang.org)

A comprehensive **SCIM 2.0 server library** for Rust that makes identity provisioning simple, type-safe, and stable.

> **⚠️ DEVELOPMENT WARNING:** This library is under active development and subject to breaking changes until version 0.9.0. Breaking changes will be signaled by semver minor version increments (0.3.0, 0.4.0, etc.), with patch releases (0.3.1, 0.3.2) being non-breaking. Please pin to exact minor versions in production deployments.

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
- **🏢 Enterprise** - Database providers with full ACID compliance
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

**Result**: Your SaaS applications focus on business logic while the SCIM server handles all provisioning complexity with enterprise-grade capabilities.

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
- 📖 **Stable & Complete** - Extensive testing (863 tests), logging, and comprehensive error handling

## 🚀 Quick Start

Add to your `Cargo.toml`:

```toml
[dependencies]
# Pin to exact version for stability during active development
scim-server = "=0.2.3"
tokio = { version = "1.0", features = ["full"] }
serde_json = "1.0"
```

> **Version Pinning Recommended**: Use `=0.2.3` (exact version) instead of `0.2.3` (compatible) to avoid breaking changes during active development.
</newtext>

<old_text>
**Timeline:** Q2 2025 - This will be a breaking change requiring migration

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
- ✅ **PATCH Operations** - Complete RFC 7644 PATCH implementation with add/remove/replace operations

### Advanced Capabilities
- 🏗️ **Multi-Tenant Architecture** - Isolate data between organizations
- 🔍 **Automatic Discovery** - Service provider configuration and schema endpoints
- 🎛️ **Provider Capabilities** - Automatic feature detection and advertisement
- 📝 **Comprehensive Logging** - Structured logging with multiple backends
- 🔧 **Value Objects** - Type-safe domain modeling with compile-time validation

### 🔄 ETag Concurrency Control

**Enterprise-Grade Optimistic Locking** - Prevent lost updates in multi-client environments:

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

### 🔐 Compile-Time Authentication (NEW in 0.2.1)

**Zero-Cost Security Enforcement** - Catch authentication bugs at compile time, not runtime:

```rust
use scim_server::auth::{
    AuthenticationState, Unauthenticated, Authenticated,
    LinearCredentials, AuthenticationWitness, TenantAuthority
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Start with unauthenticated state - compile-time enforced
    let mut credentials: LinearCredentials<Unauthenticated> =
        LinearCredentials::new("user123", "tenant456");

    // Authentication consumes credentials (can only happen once)
    let auth_witness: AuthenticationWitness<Authenticated> =
        credentials.authenticate("valid_token").await?;

    // Tenant authority proves compile-time tenant access rights
    let tenant_authority: TenantAuthority =
        auth_witness.verify_tenant_access("tenant456")?;

    // Operations require authentication witness - impossible to bypass
    let protected_data = server
        .get_protected_resource(&auth_witness, &tenant_authority)
        .await?;

    // ❌ This would be a compile error:
    // let data = server.get_protected_resource(); // Missing auth witness

    Ok(())
}
```

**Authentication Features:**
- 🛡️ **Compile-Time Security** - Authentication bugs caught during compilation
- 🔄 **Linear Credentials** - Can only be used once, preventing replay attacks
- 🏢 **Tenant Isolation** - Type-safe multi-tenant access control
- ⚡ **Zero Runtime Cost** - All checks happen at compile time
- 🎯 **RBAC Support** - Role-based access control with type safety

### Framework Integration
- 🌐 **HTTP Framework Agnostic** - Bring your own web framework
- 🔌 **Operation Handler Foundation** - Clean abstraction for SCIM operations
- 🤖 **MCP Integration** - Model Context Protocol support for AI tools

### 🔧 PATCH Operations (NEW in 0.2.3)

**Complete RFC 7644 PATCH Implementation** - Granular resource updates with full SCIM compliance:

```rust
use scim_server::{ScimServer, providers::InMemoryProvider, resource::RequestContext};
use serde_json::json;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let provider = InMemoryProvider::new();
    let server = ScimServer::new(provider)?;
    let context = RequestContext::with_generated_id();

    // Create a user first
    let user_data = json!({
        "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
        "userName": "alice@example.com",
        "active": true,
        "emails": [{"value": "alice@example.com", "primary": true}]
    });

    let user = server.provider()
        .create_resource("User", user_data, &context)
        .await?;

    // PATCH operations: add, remove, replace
    let patch_request = json!({
        "Operations": [
            {
                "op": "replace",
                "path": "active",
                "value": false
            },
            {
                "op": "add",
                "path": "emails",
                "value": {"value": "alice.work@example.com", "type": "work"}
            },
            {
                "op": "remove",
                "path": "emails[type eq \"work\"]"
            }
        ]
    });

    let patched_user = server.provider()
        .patch_resource("User", &user.id, patch_request, &context)
        .await?;

    println!("User updated via PATCH: {}", patched_user.id);
    Ok(())
}
```

**PATCH Features:**
- ✅ **Three Operations** - `add`, `remove`, and `replace` operations per RFC 7644
- ✅ **Path Expressions** - Complex path syntax for nested attributes and arrays
- ✅ **Multi-valued Attributes** - Safe operations on emails, phone numbers, addresses
- ✅ **Schema Validation** - Automatic validation against SCIM schemas
- ✅ **ETag Integration** - Works seamlessly with concurrency control
- ✅ **Atomic Operations** - All-or-nothing PATCH application with rollback

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
| [`compile_time_auth`](examples/compile_time_auth_example.rs) | Type-safe authentication at compile time |
| [`compile_time_rbac`](examples/compile_time_rbac_example.rs) | Role-based access control with type safety |
| [`etag_concurrency`](examples/etag_concurrency_example.rs) | ETag-based optimistic locking |

## 🏗️ Architecture

```
┌─────────────────┐    ┌──────────────────┐    ┌─────────────────┐
│   HTTP Layer    │    │   SCIM Server    │    │   Provider      │
│                 │    │                  │    │                 │
│  • Axum         │───▶│  • Validation    │───▶│  • In-Memory    │
│  • Warp         │    │  • Operations    │    │  • Database     │
│  • Actix        │    │  • Multi-tenant  │    │  • Custom       │
│  • Custom       │    │  • Type Safety   │    │                 │
└─────────────────┘    └──────────────────┘    └─────────────────┘
```

The library provides a clean separation between:
- **HTTP handling** (your choice of framework)
- **SCIM logic** (validation, operations, multi-tenancy)
- **Data storage** (pluggable providers)

## 📖 Documentation

| Resource | Description |
|----------|-------------|
| [📚 API Documentation](https://docs.rs/scim-server) | Complete API reference with examples |
| [🚀 Quick Start](docs/guides/quick-start.md) | Get running in 5 minutes |
| [🔧 PATCH Operations](docs/guides/patch-operations.md) | Complete guide to PATCH operations |
| [📖 User Guide](docs/guides/user-guide.md) | Step-by-step tutorials |
| [🏗️ Architecture Guide](docs/guides/architecture.md) | Design decisions and patterns |
| [✅ SCIM Compliance](docs/reference/scim-compliance.md) | RFC 7644 implementation details |
| [📌 Versioning Strategy](VERSIONING.md) | Version pinning and stability guide |

## 🗺️ What's Coming Next

### Version 0.3.0: Storage Provider Architecture (Breaking Changes)

The next major release will introduce architecture improvement that separates storage concerns from SCIM logic:

```rust
// Current: Complex provider implementation (1000+ lines)
impl ResourceProvider for CustomProvider {
    // Implement all SCIM operations + storage + validation
}

// Future: Simple storage provider (50 lines)
impl StorageProvider for CustomStorageProvider {
    // Just basic CRUD operations
}

// SCIM logic handled by StandardResourceProvider
type CustomProvider = StandardResourceProvider<CustomStorageProvider>;
```

**Benefits:**
- 🎯 **Reduced Complexity** - Custom providers need only ~50 lines vs 1000+
- 🏗️ **Better Architecture** - Clear separation between storage and SCIM logic
- 🔄 **Consistent Behavior** - All providers get same SCIM compliance automatically
- ⚡ **Easier Optimization** - Storage providers can focus purely on performance
- 🧪 **Improved Testing** - Storage and SCIM logic tested independently

**Timeline:** August 2025 - This will be a breaking change requiring migration

| Resource | Description |
|----------|-------------|
| [🏢 Multi-Tenancy](docs/api/multi-tenancy.md) | Multi-tenant setup and usage |
| [🔐 Compile-Time Authentication](docs/COMPILE_TIME_AUTHENTICATION.md) | Type-safe authentication system |

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

See the repository's contributing guidelines for detailed information on how to contribute.

### Development Principles
- **Type safety first** - Leverage Rust's type system
- **YAGNI compliance** - Build only what's needed now
- **Functional patterns** - Immutable data and pure functions
- **Comprehensive testing** - Every feature thoroughly tested

### 🏆 Stable & Enterprise-Ready

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

**94% SCIM 2.0 Compliance** - See [compliance report](ReferenceNotes/SCIM_2_0_COMPLIANCE_SUMMARY.md) for details.

## 📝 License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## 🙏 Acknowledgments

- [SCIM 2.0 Specification](https://tools.ietf.org/html/rfc7644) - The foundation this library implements
- [Rust Community](https://www.rust-lang.org/community) - For the amazing ecosystem and support

---

**Ready to get started?** Check out the [Quick Start Guide](docs/guides/quick-start.md) or browse the [examples](examples/).
