# Builder Pattern Configuration

This example demonstrates the flexible configuration capabilities of the SCIM server using the builder pattern. It shows how to construct servers with different deployment patterns, tenant strategies, and feature configurations through a fluent, type-safe API.

## What This Example Demonstrates

- **Fluent Configuration API** - Chain configuration methods for readable server setup
- **Deployment Pattern Flexibility** - Single-tenant, multi-tenant, and hybrid configurations
- **URL Generation Strategies** - Different approaches to endpoint and reference URL creation
- **Feature Toggle Management** - Enabling and configuring optional capabilities
- **Environment-Specific Setup** - Development, staging, and production configurations
- **Configuration Validation** - Compile-time and runtime validation of server settings

## Key Features Showcased

### Flexible Server Construction
See how [`ScimServerBuilder`](https://docs.rs/scim-server/latest/scim_server/struct.ScimServerBuilder.html) enables readable, maintainable server configuration through method chaining and type-safe parameter handling.

### Tenant Strategy Configuration
Explore different [`TenantStrategy`](https://docs.rs/scim-server/latest/scim_server/enum.TenantStrategy.html) options and understand when to use subdomain-based, path-based, or single-tenant URL patterns.

### Base URL Management
Watch how proper base URL configuration affects `$ref` field generation, resource location URLs, and integration with different deployment environments.

### Configuration Composition
The example demonstrates how to compose complex configurations from simpler building blocks, enabling reusable configuration patterns across different environments.

## Concepts Explored

This example showcases configuration and deployment patterns:

- **[SCIM Server](../concepts/scim-server.md#builder-pattern-for-configuration)** - Builder pattern implementation and usage
- **[Multi-Tenant Architecture](../concepts/multi-tenant-architecture.md)** - Tenant strategy selection and configuration
- **[Architecture Overview](../architecture.md)** - How configuration affects system behavior

## Perfect For Understanding

This example is essential if you're:

- **Configuring Production Deployments** - Environment-specific server setup
- **Building Flexible Systems** - Runtime configuration and feature toggles
- **Managing Multiple Environments** - Development, testing, and production configurations
- **Creating Deployment Templates** - Reusable configuration patterns

## Configuration Scenarios

The example covers multiple deployment patterns:

### Development Configuration
Simple, localhost-based setup with minimal security and maximum debugging visibility:
- In-memory storage for quick iteration
- Detailed logging and error reporting
- Single-tenant mode for simplicity
- Development-friendly base URLs

### Production Multi-Tenant Setup
Enterprise-ready configuration with proper tenant isolation:
- Database-backed storage with connection pooling
- Path-based tenant strategy for clean URLs
- Production base URLs with HTTPS
- Enhanced security and audit logging

### Hybrid Cloud Deployment
Sophisticated configuration for cloud-native deployments:
- Subdomain-based tenant isolation
- Environment variable integration
- Feature flags and capability toggles
- Observability and monitoring integration

## Builder Method Categories

Explore the different types of configuration available:

### Core Configuration
- **Base URL Setup** - Foundation for all URL generation
- **SCIM Version** - Protocol version specification
- **Server Metadata** - Identity and capability information

### Multi-Tenancy Configuration
- **Tenant Strategy** - URL pattern and isolation approach
- **Default Permissions** - Baseline tenant capabilities
- **Isolation Levels** - Data separation strategies

### Feature Configuration
- **Optional Capabilities** - MCP integration, bulk operations, filtering
- **Performance Tuning** - Connection pools, caching, timeouts
- **Security Settings** - Authentication requirements, CORS policies

## Running the Example

```bash
cargo run --example builder_example
```

The output shows different server configurations being built and validated, demonstrating how the builder pattern creates properly configured servers for various deployment scenarios.

## Configuration Best Practices

The example illustrates production-ready configuration patterns:

- **Environment Separation** - Different configurations for different environments
- **Validation Strategy** - Early validation of configuration parameters
- **Default Management** - Sensible defaults with explicit overrides
- **Documentation** - Self-documenting configuration through method names

## Type Safety Benefits

See how the builder pattern provides compile-time guarantees:

- **Required Parameters** - Compiler ensures essential configuration is provided
- **Valid Combinations** - Type system prevents invalid configuration states
- **Method Chaining** - Fluent API with proper return types
- **Error Prevention** - Many configuration errors caught at compile time

## Configuration Reusability

Learn patterns for creating reusable configurations:

- **Configuration Templates** - Base configurations for common scenarios
- **Environment Abstraction** - Parameterized configurations for different deployments
- **Feature Composition** - Mixing and matching capabilities as needed
- **Validation Helpers** - Shared validation logic across configurations

## Next Steps

After exploring builder pattern configuration:

- **[Multi-Tenant Server](./multi-tenant.md)** - See multi-tenant configurations in action
- **[Operation Handlers](./operation-handlers.md)** - Framework integration with configured servers
- **[Basic Usage](./basic-usage.md)** - Simple configurations for getting started

## Source Code

View the complete implementation: [`examples/builder_example.rs`](https://github.com/pukeko37/scim-server/blob/main/examples/builder_example.rs)

## Related Documentation

- **[Configuration Guide](../getting-started/configuration.md)** - Comprehensive server setup documentation
- **[ScimServerBuilder API Reference](https://docs.rs/scim-server/latest/scim_server/struct.ScimServerBuilder.html)** - Complete builder API
- **[Multi-Tenant Architecture](../concepts/multi-tenant-architecture.md)** - Tenant strategy details and patterns