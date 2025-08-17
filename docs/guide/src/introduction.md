# SCIM Server Guide

Welcome to the comprehensive guide for the SCIM Server library! This guide will take you from initial setup to advanced usage patterns, helping you build robust identity provisioning systems with Rust.

## What is SCIM Server?

SCIM Server is a comprehensive Rust library for building SCIM 2.0-compliant identity provisioning systems. It serves as both an integration framework for connecting data sources to applications and a complete solution for enterprise identity management.

The library implements the SCIM 2.0 protocol—the industry standard for automating user provisioning between identity providers and applications. SCIM 2.0 provides a unified REST API that eliminates custom integrations and enables seamless synchronization across systems.

### Why SCIM?

SCIM (System for Cross-domain Identity Management) was designed by the IETF to solve the fundamental challenge of identity management in multi-domain scenarios—particularly enterprise-to-cloud and inter-cloud environments. As defined in RFC 7643 and RFC 7644, SCIM addresses the cost and complexity of user management operations that plague modern organizations.

Before SCIM, every identity integration required custom development, proprietary APIs, and ongoing maintenance. Organizations faced manual provisioning overhead, security gaps from delayed deprovisioning, and compliance risks from inconsistent access controls. SCIM transforms this by providing a standardized HTTP-based protocol that reduces integration complexity while applying proven authentication, authorization, and privacy models.

The protocol's emphasis on simplicity of development and integration makes it practical for real-world deployment, while its extensible schema model allows organizations to define custom attributes and entirely new resource types beyond the standard User and Group schemas—all while maintaining interoperability across identity providers and applications.

### Why This Library?

This library transforms complex enterprise provisioning into straightforward implementation:

- **Standardized Operations**: Consistent CRUD operations across all systems with RFC 7643/7644 compliance
- **Rich Filtering**: Powerful query capabilities using standardized filter syntax (`userName eq "alice@example.com"`)
- **Custom Resources**: Define new resource types beyond Users and Groups while maintaining SCIM compliance
- **Type Safety**: Compile-time guarantees prevent invalid operations
- **Multi-Tenancy**: Built-in tenant isolation and context management
- **Performance**: Async-first architecture with minimal overhead
- **Flexibility**: Framework-agnostic with pluggable storage
- **AI Integration**: Model Context Protocol support for AI agents

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

**Client Layer**: Integration points for different client types (AI agents, web frameworks, custom clients).

**SCIM Server**: Core orchestration with operation handling and multi-tenant context.

**Resource Layer**: `ResourceProvider` trait abstracts SCIM resource operations from storage.

**Schema System**: Dynamic schema registry with validation and extensible value objects.

**Storage Layer**: `StorageProvider` trait for pluggable data persistence backends.

### Trait-Based Design

The architecture is built around key traits that provide clear contracts and pluggability:

- **`ResourceProvider`**: Core abstraction for SCIM resource operations (create, read, update, delete, list)
- **`StorageProvider`**: Pure data persistence layer separated from SCIM protocol concerns
- **`SchemaConstructible`**: Enables dynamic value object creation from schema definitions
- **`ValueObject`**: Type-safe SCIM attribute handling with schema validation
- **`TenantResolver`**: Multi-tenant context resolution from authentication credentials

This separation allows you to implement storage backends, custom authentication, and schema extensions independently while maintaining type safety and SCIM compliance.

### Key Features

- **Schema Extensions**: Define custom attributes and resource types while maintaining SCIM compliance
- **ETag Concurrency**: Automatic optimistic locking with conditional operations and conflict detection
- **Observability**: Structured logging with request IDs, tenant context, and performance metrics
- **Auto-Discovery**: Runtime capability detection and ServiceProviderConfig generation
- **Framework Support**: Works with Axum, Warp, Actix, and other web frameworks
- **Enterprise Integration**: Seamless compatibility with Okta, Azure Entra, Google Workspace, and other identity providers

## Value Proposition

Instead of building provisioning logic into every application, the SCIM Server centralizes complexity:

| **Without SCIM Server** | **With SCIM Server** |
|-------------------------|----------------------|
| ❌ Custom validation in each app | ✅ Centralized validation engine |
| ❌ Manual concurrency control | ✅ Automatic ETag versioning |
| ❌ Manual schema management | ✅ Dynamic schema registry |
| ❌ Ad-hoc API endpoints | ✅ Standardized SCIM protocol |
| ❌ Build multi-tenancy from scratch | ✅ Built-in tenant isolation |

**Result**: Your applications focus on business logic while SCIM Server handles all provisioning complexity.

## Who Should Use This Guide?

This guide is designed for:

- **Rust Developers** building identity-aware applications
- **System Architects** designing multi-tenant SaaS platforms
- **DevOps Engineers** automating user provisioning workflows
- **AI Engineers** integrating identity management with AI tools
- **Security Engineers** implementing enterprise identity solutions

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

By the end of this guide, you'll be able to:

- Set up and configure SCIM Server for your use case
- Implement multi-tenant identity provisioning systems
- Handle complex scenarios like custom resources and authentication
- Integrate with web frameworks and AI tools
- Deploy production-ready SCIM services
- Troubleshoot common issues and optimize performance

## Getting Help

- **Examples**: Check the [examples directory](https://github.com/pukeko37/scim-server/tree/main/examples) for working code
- **API Documentation**: See [docs.rs](https://docs.rs/scim-server) for detailed API reference
- **Issues**: Report bugs or ask questions on [GitHub Issues](https://github.com/pukeko37/scim-server/issues)

Let's get started! 🚀
