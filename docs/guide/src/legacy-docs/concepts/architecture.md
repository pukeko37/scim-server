# Architecture

This chapter explains the SCIM Server's three-layer architecture and how it enables flexible, scalable identity provisioning systems.

## Overview

The SCIM Server acts as **intelligent middleware** that handles all provisioning complexity so your applications don't have to. It follows a clean three-layer architecture that separates concerns and enables flexibility:

```
┌─────────────────┐    ┌──────────────────┐    ┌─────────────────┐
│   Client Layer  │    │   SCIM Server    │    │  Storage Layer  │
│                 │    │                  │    │                 │
│  • Web Apps     │───▶│  • Validation    │───▶│  • In-Memory    │
│  • AI Tools     │    │  • Operations    │    │  • Database     │
│  • CLI Scripts  │    │  • Multi-tenant  │    │  • Custom       │
│  • Custom APIs  │    │  • Type Safety   │    │                 │
└─────────────────┘    └──────────────────┘    └─────────────────┘
```

This design provides flexibility at each layer while maintaining consistency and type safety throughout.

## Client Layer: Multiple Ways to Connect

The SCIM Server supports diverse client types through standardized interfaces:

### Web Applications
- **Admin Portals**: Full-featured management interfaces
- **User Dashboards**: Self-service identity management
- **Sync Tools**: Automated synchronization between systems
- **Integration APIs**: RESTful endpoints for custom applications

### AI Assistants
- **Natural Language Processing**: Convert human requests to SCIM operations
- **Model Context Protocol (MCP)**: Direct integration with Claude, ChatGPT, and custom bots
- **Conversational Interfaces**: Chat-based identity management
- **Intelligent Automation**: AI-driven provisioning decisions

### Automation Tools
- **CLI Scripts**: Command-line tools for bulk operations
- **Migration Scripts**: Data import/export utilities
- **DevOps Pipelines**: CI/CD integration for automated provisioning
- **Batch Processing**: Scheduled bulk operations

### Custom Integrations
- **GraphQL**: Type-safe query interfaces
- **gRPC**: High-performance binary protocols
- **Message Queues**: Asynchronous processing workflows
- **Webhooks**: Event-driven integrations
- **Custom Protocols**: Adapt to any existing system

## Intelligence Layer: The SCIM Server Core

The SCIM Server core provides enterprise-grade capabilities that would take months to build yourself:

### Dynamic Schema Management
- **Custom Resource Types**: Define schemas beyond users and groups
- **Automatic Validation**: Schema-driven input validation
- **Schema Evolution**: Version and migrate schemas over time
- **Type Safety**: Compile-time schema validation

### Comprehensive Validation
- **Input Validation**: Automatic validation against SCIM schemas
- **Business Rules**: Custom validation logic
- **Error Reporting**: Detailed, actionable error messages
- **Data Integrity**: Ensure consistency across operations

### Standardized Operations
- **CRUD Operations**: Create, Read, Update, Delete with SCIM semantics
- **Filtering**: Rich query capabilities with SCIM filter syntax
- **Bulk Operations**: Efficient batch processing
- **PATCH Operations**: Granular updates with RFC 7644 compliance

### Multi-Tenant Architecture
- **Organization Isolation**: Complete data separation between tenants
- **Configuration Management**: Tenant-specific settings and schemas
- **Resource Scoping**: Automatic tenant boundary enforcement
- **Performance Isolation**: Independent scaling per tenant

### Automatic Capabilities
- **Service Provider Configuration**: Self-documenting API features
- **Schema Discovery**: Runtime schema introspection
- **Capability Advertisement**: Automatic feature detection
- **API Documentation**: Generated OpenAPI specifications

## Storage Layer: Flexible Backend Options

Choose your data storage strategy without changing your application code:

### Development Options
### Currently Available
- **In-Memory Storage**: Fast prototyping and testing (✅ [`InMemoryStorage`](https://docs.rs/scim-server/latest/scim_server/storage/struct.InMemoryStorage.html))

### Custom Storage Providers
- **Trait-Based**: Implement the [`StorageProvider`](https://docs.rs/scim-server/latest/scim_server/storage/trait.StorageProvider.html) trait
- **Async Support**: Full async/await compatibility 
- **Error Handling**: Rich error types and recovery
- **Testing**: Built-in test utilities

> **Roadmap Note**: Additional storage providers (PostgreSQL, MySQL, DynamoDB, etc.) are planned for future releases. Currently, only in-memory storage is implemented. You can implement custom storage providers by implementing the `StorageProvider` trait.

## Value Proposition: Complexity Reduction

Instead of building provisioning logic into every application, SCIM Server centralizes complexity:

| **Without SCIM Server** | **With SCIM Server** |
|-------------------------|----------------------|
| ❌ Custom validation in each app | ✅ **Centralized validation engine** |
| ❌ Manual concurrency control | ✅ **Automatic ETag versioning** |
| ❌ Manual schema management | ✅ **Dynamic schema registry** |
| ❌ Ad-hoc API endpoints | ✅ **Standardized SCIM protocol** |
| ❌ Reinvent capability discovery | ✅ **Automatic capability construction** |
| ❌ Build multi-tenancy from scratch | ✅ **Built-in tenant isolation** |
| ❌ Custom error handling per resource | ✅ **Consistent error semantics** |
| ❌ Lost updates in concurrent scenarios | ✅ **Version conflict detection** |

**Result**: Your applications focus on business logic while SCIM Server handles all provisioning complexity with enterprise-grade capabilities.

## Design Principles

The architecture follows key design principles:

### Separation of Concerns
- **HTTP handling** is separate from SCIM logic
- **Business logic** is separate from data storage
- **Validation** is separate from persistence
- **Multi-tenancy** is handled at the core, not storage level

### Type Safety
- **Compile-time validation** prevents runtime errors
- **Strong typing** throughout the API
- **Schema validation** ensures data integrity
- **Error types** provide rich error handling

### Performance
- **Async-first design** for high concurrency
- **Minimal allocations** in hot paths
- **Efficient serialization** with zero-copy where possible
- **Connection pooling** for database providers

### Flexibility
- **Pluggable storage** adapts to any backend
- **Framework agnostic** works with any HTTP library
- **Extensible schemas** support custom resource types
- **Configuration driven** behavior without code changes

## Scalability Considerations

The architecture scales in multiple dimensions:

### Horizontal Scaling
- **Stateless design** enables multiple server instances
- **Database scaling** through read replicas and sharding
- **Load balancing** across SCIM server instances
- **Cache layers** for frequently accessed data

### Vertical Scaling
- **Async processing** maximizes CPU utilization
- **Memory efficiency** through careful allocation patterns
- **Connection reuse** reduces resource overhead
- **Batching** optimizes database interactions

### Multi-Tenant Scaling
- **Tenant isolation** prevents noisy neighbor problems
- **Resource quotas** enable fair resource sharing
- **Performance monitoring** per tenant
- **Independent scaling** based on tenant needs

## Security Architecture

Security is built into every layer:

### Authentication & Authorization
- **Compile-time auth** prevents unauthorized access
- **Tenant isolation** enforces data boundaries
- **Role-based access** with fine-grained permissions
- **API key management** with rotation support

### Data Protection
- **Encryption at rest** through storage providers
- **Encryption in transit** via HTTPS
- **PII handling** with appropriate safeguards
- **Audit logging** for compliance requirements

### Concurrency Safety
- **ETag versioning** prevents lost updates
- **Atomic operations** ensure data consistency
- **Transaction support** where available
- **Conflict resolution** with clear error messages

This architecture provides a solid foundation for building scalable, secure, and maintainable identity provisioning systems while keeping complexity manageable for developers.