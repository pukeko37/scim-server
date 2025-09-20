# Library Introduction

SCIM Server is a comprehensive Rust library that implements the SCIM 2.0 (System for Cross-domain Identity Management) protocol—the industry standard for user provisioning. Instead of building custom user management APIs from scratch (which typically takes 3-6 developer months), SCIM Server provides a type-safe, high-performance foundation that gets you from zero to enterprise-ready user provisioning in weeks, not months.

## What is SCIM Server?

SCIM Server is a Rust library that provides all the essential components for building SCIM 2.0-compliant systems. Instead of implementing SCIM from scratch, you get proven building blocks that handle the complex parts while letting you focus on your application logic.

The library uses the SCIM 2.0 protocol as a framework to standardize identity data validation and processing. You compose the components you need—from simple single-tenant systems to complex multi-tenant platforms with custom schemas and AI integration.

## What You Get

### Ready-to-Use Components
- **[`StandardResourceProvider`](https://docs.rs/scim-server/latest/scim_server/providers/struct.StandardResourceProvider.html)**: Complete SCIM resource operations for typical use cases
- **[`InMemoryStorage`](https://docs.rs/scim-server/latest/scim_server/storage/struct.InMemoryStorage.html) and [`SqliteStorage`](https://docs.rs/scim-server/latest/scim_server/storage/sqlite/struct.SqliteStorage.html)**: Development and testing storage backends
- **[Schema Registry](https://docs.rs/scim-server/latest/scim_server/schema/struct.SchemaRegistry.html)**: Pre-loaded with RFC 7643 User and Group schemas
- **ETag Versioning**: Automatic concurrency control for production deployments

### Extension Points
- **[`ResourceProvider` trait](https://docs.rs/scim-server/latest/scim_server/trait.ResourceProvider.html)**: Implement for custom business logic and data models
- **[`StorageProvider` trait](https://docs.rs/scim-server/latest/scim_server/storage/trait.StorageProvider.html)**: Connect to any database or storage system
- **Custom Value Objects**: Type-safe handling of domain-specific attributes
- **[Multi-Tenant Context](https://docs.rs/scim-server/latest/scim_server/struct.TenantContext.html)**: Built-in tenant isolation and context management

### Enterprise Features
- **Protocol Compliance**: All the RFC 7643/7644 complexity handled correctly
- **[Schema Extensions](https://docs.rs/scim-server/latest/scim_server/schema/index.html)**: Add custom attributes while maintaining SCIM compatibility
- **[AI Integration](https://docs.rs/scim-server/latest/scim_server/mcp_integration/index.html)**: Model Context Protocol support for AI agent interactions
- **Production Ready**: Structured logging, error handling, and performance optimizations



---

*Need to understand why SCIM Server is essential for enterprise adoption? See [Why You Need SCIM Server](./why-scim-server.md) for the business case and problem context.*