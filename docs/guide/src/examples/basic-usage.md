# Basic Usage

This example demonstrates the fundamental operations of a SCIM server using the [`StandardResourceProvider`](https://docs.rs/scim-server/latest/scim_server/providers/struct.StandardResourceProvider.html) with in-memory storage. It's the perfect starting point for understanding core SCIM functionality.

## What This Example Demonstrates

- **Essential CRUD Operations** - Create, read, update, delete, and list users
- **SCIM 2.0 Compliance** - Proper schema validation and metadata management  
- **Resource Provider Pattern** - Using the standard provider for typical use cases
- **Request Context** - Tracking operations with unique request identifiers
- **Error Handling** - Graceful handling of validation and operational errors

## Key Features Showcased

### Resource Creation and Validation
The example shows how the [`StandardResourceProvider`](https://docs.rs/scim-server/latest/scim_server/providers/struct.StandardResourceProvider.html) automatically validates user data against SCIM 2.0 schemas, ensuring compliance and preventing invalid data from entering your system.

### Metadata Management  
Watch as the server automatically generates proper SCIM metadata including timestamps, resource versions, and location URLs - all handled transparently by the [`ResourceProvider`](https://docs.rs/scim-server/latest/scim_server/trait.ResourceProvider.html) implementation.

### Storage Abstraction
See how the [`InMemoryStorage`](https://docs.rs/scim-server/latest/scim_server/storage/struct.InMemoryStorage.html) backend provides a clean separation between business logic and data persistence, making it easy to swap storage implementations.

## Concepts Explored

This example serves as an introduction to several key concepts covered in depth elsewhere:

- **[Resource Providers](../concepts/resource-providers.md)** - The business logic layer that implements SCIM semantics
- **[Storage Providers](../concepts/storage-providers.md)** - The data persistence abstraction
- **[Request Context](../concepts/scim-server.md#request-context)** - Operation tracking and tenant scoping
- **[Schema Validation](../concepts/schemas.md)** - Ensuring SCIM 2.0 compliance

## Perfect For Learning

This example is ideal if you're:

- **New to SCIM** - Understand the basic protocol operations
- **Evaluating the library** - See core functionality in action  
- **Building simple systems** - Single-tenant identity management
- **Understanding the architecture** - See how components work together

## Running the Example

```bash
cargo run --example basic_usage
```

The example creates several users, demonstrates various query patterns, shows update operations, and illustrates proper error handling - all with clear console output explaining each step.

## Next Steps

After exploring basic usage, consider:

- **[Multi-Tenant Server](./multi-tenant.md)** - Add tenant isolation capabilities
- **[Group Management](./group-management.md)** - Work with groups and member relationships
- **[Configuration Guide](../getting-started/configuration.md)** - Learn advanced server setup

## Source Code

View the complete implementation: [`examples/basic_usage.rs`](https://github.com/pukeko37/scim-server/blob/main/examples/basic_usage.rs)