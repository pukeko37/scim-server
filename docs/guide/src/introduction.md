# SCIM Server Guide

Welcome to the comprehensive guide for the SCIM Server library! This guide will take you from initial setup to advanced usage patterns, helping you build robust identity provisioning systems with Rust.

## What is SCIM Server?

SCIM Server is a comprehensive Rust library providing modular components for building SCIM 2.0-compliant systems. Rather than a monolithic solution, it offers composable building blocks that you combine to construct whatever identity provisioning system your application needs.

The library uses the SCIM 2.0 protocol as a framework to standardize the validation and processing of provisioning data. Instead of rolling your own SCIM implementation, you get enterprise-ready building blocks: resource providers, storage abstractions, schema validation, multi-tenant context management, and extensible value objects.

### Why SCIM?

SCIM (System for Cross-domain Identity Management) was designed by the IETF to solve the fundamental challenge of identity management in multi-domain scenarios—particularly enterprise-to-cloud and inter-cloud environments. As defined in RFC 7643 and RFC 7644, SCIM addresses the cost and complexity of user management operations that plague modern organizations.

Before SCIM, every identity integration required custom development, proprietary APIs, and ongoing maintenance. Organizations faced manual provisioning overhead, security gaps from delayed deprovisioning, and compliance risks from inconsistent access controls. SCIM transforms this by providing a standardized HTTP-based protocol that reduces integration complexity while applying proven authentication, authorization, and privacy models.

The protocol's emphasis on simplicity of development and integration makes it practical for real-world deployment, while its extensible schema model allows organizations to define custom attributes and entirely new resource types beyond the standard User and Group schemas—all while maintaining interoperability across identity providers and applications.

### Why These Components?

This library provides the essential building blocks you need without the complexity of building them yourself:

- **ResourceProvider Trait**: Abstract interface for SCIM resource operations, implement once for any storage backend
- **Schema System**: RFC 7643/7644 compliant validation with extensible custom schemas and value objects  
- **Multi-Tenant Components**: Context management and tenant isolation built into the core abstractions
- **Storage Abstraction**: Pluggable backends through the StorageProvider trait - use in-memory, database, or custom
- **Type Safety**: Compile-time guarantees through Rust's type system and schema-driven value objects
- **Concurrency Control**: ETag-based optimistic locking components for production deployment
- **Protocol Compliance**: All the complex SCIM protocol handling so you focus on your application logic
- **AI Integration**: Model Context Protocol components for AI agent discovery and interaction

## Architecture Overview

The SCIM Server follows a clean trait-based architecture with clear separation of concerns:

```text
┌─────────────────┐    ┌──────────────────┐    ┌─────────────────┐
│  Client Layer   │    │   SCIM Server    │    │ Resource Layer  │
│                 │    │                  │    │                 │
│  • MCP AI       │───▶│  • Operations    │───▶│ ResourceProvider│
│  • Web Framework│    │  • Multi-tenant  │    │      trait      │
│  • Custom       │    │  • Type Safety   │    │                 │
│                 │    │                  │    │                 │
└─────────────────┘    └──────────────────┘    └─────────────────┘
                              │                          │
                              ▼                          ▼
                       ┌──────────────────┐    ┌─────────────────┐
                       │ Schema System    │    │ Storage Layer   │
                       │                  │    │                 │
                       │ • SchemaRegistry │    │ StorageProvider │
                       │ • Validation     │    │      trait      │
                       │ • Value Objects  │    │  • In-Memory    │
                       │ • Extensions     │    │  • Database     │
                       └──────────────────┘    │  • Custom       │
                                               └─────────────────┘
```

**Client Layer**: Your integration points - compose these components into web endpoints, AI tools, or custom applications.

**SCIM Server**: Orchestration component that coordinates resource operations using your provider implementations.

**Resource Layer**: `ResourceProvider` trait - implement this interface for your data model, or use the provided `StandardResourceProvider` for common scenarios.

**Schema System**: Schema registry and validation components - extend with custom schemas and value objects.

**Storage Layer**: `StorageProvider` trait - plug in any persistence backend (database, cloud storage, etc.).

### Component Architecture

The library is built around composable traits that you implement for your specific needs:

- **`ResourceProvider`**: Your main integration point - implement SCIM operations for your data model, or use `StandardResourceProvider` for typical use cases
- **`StorageProvider`**: Persistence abstraction - use the provided `InMemoryStorage` for development, or connect to any database or custom backend
- **`SchemaConstructible`**: Extension point for custom SCIM attributes and value objects
- **`ValueObject`**: Type-safe components for SCIM attribute validation and serialization
- **`TenantResolver`**: Multi-tenant components for resolving tenant context from authentication

Mix and match these components to build exactly what you need - use the out-of-the-box implementations for rapid development, or implement custom traits for specialized requirements. Scale from simple single-tenant systems to complex multi-tenant platforms with custom schemas and AI integration.

### Key Features

- **Schema Extension Components**: Build custom SCIM attributes and resource types while maintaining protocol compliance
- **Concurrency Control Components**: ETag-based optimistic locking for production-grade conflict resolution
- **Observability Components**: Structured logging with request tracing and tenant context
- **Discovery Components**: Automatic capability detection and ServiceProviderConfig generation
- **Framework Agnostic**: Components work with any web framework - Axum, Warp, Actix, or custom HTTP handling
- **Enterprise Ready**: Protocol compliance ensures compatibility with Okta, Azure Entra, Google Workspace, and other SCIM clients

## Component Benefits

Instead of implementing SCIM protocol complexity from scratch, compose these proven components:

| **Building From Scratch** | **Using SCIM Server Components** |
|-------------------------|----------------------|
| ❌ Implement RFC 7643/7644 compliance | ✅ RFC-compliant validation components |
| ❌ Build concurrency control systems | ✅ ETag-based optimistic locking components |
| ❌ Create extensible schema systems | ✅ Dynamic schema registry and value objects |
| ❌ Design multi-tenant architectures | ✅ Multi-tenant context and isolation components |
| ❌ Handle SCIM protocol edge cases | ✅ Battle-tested protocol implementation |

**Result**: Combine pre-built components to create exactly the SCIM system your application needs.

## Who Should Use This Library?

This component library is designed for:

- **Rust Developers** who need SCIM components in their applications
- **System Architects** designing identity systems with pluggable components
- **Library Authors** building higher-level identity abstractions
- **AI Tool Builders** who need SCIM protocol components for agent integration
- **Enterprise Developers** requiring RFC-compliant identity provisioning components

## How to Use This Guide

The guide is organized into progressive sections:

1. **Getting Started**: Quick setup and basic usage
2. **Core Concepts**: Understanding the fundamental ideas
3. **Tutorials**: Step-by-step guides for common scenarios
4. **How-To Guides**: Solutions for specific problems
5. **Advanced Topics**: Deep dives into complex scenarios
6. **Reference**: Technical specifications and details

### Learning Path

**New to SCIM?** Start with [SCIM Protocol Overview](./concepts/scim-protocol.md) to understand the standard.

**Ready to code?** Jump to [Your First SCIM Server](./getting-started/first-server.md) for hands-on experience.

**Building production systems?** Read through [Core Concepts](./concepts/architecture.md) and [Advanced Topics](./advanced/production-deployment.md).

**Solving specific problems?** Use the [How-To Guides](./how-to/troubleshooting.md) section.

## What You'll Learn

By the end of this guide, you'll understand how to:

- Compose SCIM Server components for your specific requirements
- Implement the ResourceProvider trait for your application's data model
- Create custom schema extensions and value objects
- Build multi-tenant systems using the provided context components
- Integrate SCIM components with web frameworks and AI tools
- Deploy production systems using the concurrency and observability components

## Getting Help

- **Examples**: Check the [examples directory](https://github.com/pukeko37/scim-server/tree/main/examples) for working code
- **API Documentation**: See [docs.rs](https://docs.rs/scim-server) for detailed API reference
- **Issues**: Report bugs or ask questions on [GitHub Issues](https://github.com/pukeko37/scim-server/issues)

Let's get started! 🚀
