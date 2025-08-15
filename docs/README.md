# SCIM Server Documentation

Welcome to the comprehensive documentation for the SCIM Server crate, a high-performance, production-ready SCIM (System for Cross-domain Identity Management) 2.0 implementation in Rust.

## 📚 Documentation Index

### Getting Started
- **[User Guide](guide/book/)** - Complete tutorials and getting started guide
- **[Installation](guide/book/getting-started/installation.html)** - Installation and setup instructions
- **[Your First Server](guide/book/getting-started/first-server.html)** - Build your first SCIM server
- **[Basic Operations](guide/book/getting-started/basic-operations.html)** - Core concepts and simple examples
- **[Multi-Tenancy](guide/book/concepts/multi-tenancy.html)** - Multi-tenant architecture guide

### API Documentation
- **[API Reference](api/README.md)** - Complete API documentation
- **[Core Types](api/core-types.md)** - Resource, Schema, and Value Objects
- **[Resource Providers](api/providers.md)** - Storage backend interfaces
- **[Multi-tenancy](api/multi-tenancy.md)** - Multi-tenant architecture
- **[Error Handling](api/error-handling.md)** - Error types and handling patterns
- **[Version Control](api/version-control.md)** - ETag implementation and conditional operations

### Core Concepts
- **[SCIM Protocol](guide/book/concepts/scim-protocol.html)** - Understanding SCIM 2.0
- **[Architecture](guide/book/concepts/architecture.html)** - System architecture and design
- **[Resource Model](guide/book/concepts/resource-model.html)** - SCIM resource structure
- **[Storage Providers](guide/book/concepts/providers.html)** - Provider system overview

### Tutorials
- **[Custom Resources](guide/book/tutorials/custom-resources.html)** - Implementing custom resource types
- **[Framework Integration](guide/book/tutorials/framework-integration.html)** - Web framework integration
- **[Multi-Tenant Deployment](guide/book/tutorials/multi-tenant-deployment.html)** - Production multi-tenancy
- **[Production Deployment](guide/book/advanced/production-deployment.html)** - Production setup guide

### Working Code Examples
- **[Code Examples](../examples/)** - Complete working Rust examples
- **[Basic Usage](../examples/basic_usage.rs)** - Simple SCIM server implementation  
- **[Multi-tenant Example](../examples/multi_tenant_example.rs)** - Multi-tenant setup
- **[Group Management](../examples/group_example.rs)** - Group operations example
- **[ETag Concurrency](../examples/etag_concurrency_example.rs)** - Concurrency control
- **[MCP Integration](../examples/mcp_server_example.rs)** - AI agent integration

### Reference
- **[SCIM 2.0 Compliance](reference/scim-compliance.md)** - RFC compliance status
- **[Schema Reference](reference/schemas.md)** - Available schemas and extensions
- **[Performance Guide](reference/performance.md)** - Performance optimization
- **[Security Guide](reference/security.md)** - Security considerations

## 🚀 Quick Navigation

### I want to...
- **Build a basic SCIM server** → [Your First Server](guide/book/getting-started/first-server.html)
- **See working code examples** → [Code Examples](../examples/)
- **Understand the architecture** → [Architecture Guide](guide/book/concepts/architecture.html)
- **Implement a custom provider** → [Storage Providers](guide/book/concepts/providers.html)
- **Deploy to production** → [Production Deployment](guide/book/advanced/production-deployment.html)
- **Check SCIM compliance** → [SCIM 2.0 Compliance](reference/scim-compliance-actual.md)
- **Browse API documentation** → [API Reference](api/README.md)
- **Prevent lost updates** → [ETag Concurrency](guide/book/concepts/etag-concurrency.html)

## 📋 Documentation Status

| Section | Status | Last Updated |
|---------|--------|--------------|
| API Documentation | ✅ Complete | 2024-12-19 |
| Getting Started | ✅ Complete | 2024-12-19 |
| User Guide | ✅ Complete | 2024-12-19 |
| Developer Guide | ✅ Complete | 2024-12-19 |
| Examples | ✅ Complete | 2024-12-19 |
| Architecture | ✅ Complete | 2024-12-19 |
| SCIM Compliance | ✅ Complete | 2024-12-19 |
| Performance Guide | ✅ Complete | 2024-12-19 |
| ETag Concurrency Control | ✅ Complete | 2024-12-19 |

## 🏗️ About This Project

The SCIM Server crate provides:

- **🔒 Type-safe SCIM 2.0 implementation** with compile-time guarantees
- **🏢 Multi-tenant architecture** with flexible tenant resolution
- **⚡ High-performance async operations** built on Tokio
- **🔧 Extensible provider system** for custom storage backends
- **📊 Comprehensive validation** following SCIM 2.0 specifications
- **🔍 Rich logging and observability** for production monitoring
- **🏷️ Built-in ETag concurrency control** preventing lost updates automatically
- **🧪 Extensive test coverage** with working code examples
- **💻 Ready-to-run examples** for common use cases

## 📊 Project Statistics

- **Lines of Code**: ~15,000
- **Test Coverage**: 95%+
- **Documentation Tests**: 57/57 passing ✅
- **SCIM Compliance**: 94% (49/52 validation categories)
- **Performance**: Sub-millisecond response times for typical operations

## 🔧 Generated Documentation

### Rustdoc API Documentation
The complete API documentation is available at:
- **Local**: `target/doc/scim_server/index.html` (after running `cargo doc`)
- **Online**: [docs.rs/scim-server](https://docs.rs/scim-server) (when published)

### Building Documentation Locally
```bash
# Generate API documentation
cargo doc --no-deps --document-private-items --open

# Run documentation tests
cargo test --doc

# Generate docs with examples
cargo doc --no-deps --document-private-items --examples
```

## 🤝 Contributing to Documentation

We welcome documentation improvements! See our [Developer Guide](guides/developer-guide.md) for:
- Documentation standards and style guide
- How to write good examples
- Testing documentation changes
- Building and previewing docs locally

## 📞 Support

- **Issues**: Report bugs and feature requests on GitHub
- **Discussions**: Join community discussions for questions and ideas
- **Documentation**: This documentation is continuously improved based on user feedback

---

*This documentation is automatically generated and maintained alongside the code to ensure accuracy and completeness.*