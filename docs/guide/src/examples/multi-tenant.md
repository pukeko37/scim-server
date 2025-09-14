# Multi-Tenant Server

This example demonstrates a complete multi-tenant SCIM server implementation, showcasing how to isolate resources and operations across different customer organizations within a single deployment. It's essential for SaaS providers who need to serve multiple enterprise customers.

## What This Example Demonstrates

- **Complete Tenant Isolation** - Strict separation of data and operations between tenants
- **Tenant Resolution** - Mapping authentication credentials to tenant contexts
- **Resource Permissions** - Granular control over tenant capabilities and quotas
- **Isolation Levels** - Different strategies for tenant data separation
- **Realistic Multi-Tenant Patterns** - Production-ready tenant management scenarios

## Key Features Showcased

### Tenant Context Management
See how [`TenantContext`](https://docs.rs/scim-server/latest/scim_server/struct.TenantContext.html) provides complete tenant identity and permissions, ensuring that every operation is properly scoped to the correct customer organization.

### Credential-Based Resolution
The example demonstrates [`StaticTenantResolver`](https://docs.rs/scim-server/latest/scim_server/multi_tenant/struct.StaticTenantResolver.html) mapping authentication tokens to specific tenants, showing how to integrate with your existing authentication infrastructure.

### Resource Isolation Strategies
Explore different [`IsolationLevel`](https://docs.rs/scim-server/latest/scim_server/resource/enum.IsolationLevel.html) options - from strict separation to shared resources with tenant scoping - and understand when to use each approach.

### Tenant Permissions
Watch [`TenantPermissions`](https://docs.rs/scim-server/latest/scim_server/resource/struct.TenantPermissions.html) in action, controlling resource quotas, operation limits, and feature access on a per-tenant basis.

## Concepts Explored

This example brings together multiple advanced concepts:

- **[Multi-Tenant Architecture](../concepts/multi-tenant-architecture.md)** - Complete tenant isolation patterns
- **[Resource Providers](../concepts/resource-providers.md)** - Tenant-aware business logic
- **[Request Context](../concepts/scim-server.md#request-context)** - Tenant information flow
- **[Storage Providers](../concepts/storage-providers.md)** - Tenant-scoped data persistence

## Perfect For Building

This example is essential if you're:

- **Building SaaS Platforms** - Multi-customer identity management
- **Enterprise Integration** - Serving multiple organizations
- **Scalable Architecture** - Tenant isolation at scale
- **Production Systems** - Real-world multi-tenancy patterns

## Scenario Walkthrough

The example creates multiple tenant scenarios:

1. **Enterprise Customer A** - Full-featured tenant with high limits
2. **Startup Customer B** - Basic tenant with restricted permissions
3. **Trial Customer C** - Limited-time tenant with minimal access

Each tenant operates completely independently, demonstrating true isolation while sharing the same server infrastructure.

## Multi-Tenant Operations

Watch how the same operations behave differently across tenants:

- **User Creation** - Respects per-tenant quotas and permissions
- **Resource Queries** - Returns only tenant-specific data
- **Schema Extensions** - Tenant-specific attribute customizations
- **Audit Trails** - Proper tenant attribution for all operations

## Running the Example

```bash
cargo run --example multi_tenant_example
```

The output shows clear tenant separation, permission enforcement, and isolation verification - demonstrating that tenants truly cannot access each other's data.

## Production Considerations

This example illustrates production-ready patterns:

- **Security Boundaries** - Preventing cross-tenant data leakage
- **Resource Management** - Quota enforcement and capacity planning
- **Operational Visibility** - Tenant-aware logging and monitoring
- **Configuration Management** - Per-tenant customization capabilities

## Next Steps

After exploring multi-tenant architecture:

- **[ETag Concurrency Control](./etag-concurrency.md)** - Add conflict prevention to multi-tenant operations
- **[MCP Server](./mcp-server.md)** - Enable AI agents to work with tenant-scoped operations
- **[Operation Handlers](./operation-handlers.md)** - Framework-agnostic tenant-aware request handling

## Source Code

View the complete implementation: [`examples/multi_tenant_example.rs`](https://github.com/pukeko37/scim-server/blob/main/examples/multi_tenant_example.rs)