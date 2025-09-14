# ETag Concurrency Control

This example demonstrates the built-in ETag concurrency control features of the SCIM server library, showing how to use conditional operations to prevent lost updates and handle version conflicts in multi-client scenarios.

## What This Example Demonstrates

- **Version-Based Conflict Prevention** - Using ETags to detect and prevent concurrent modification conflicts
- **Conditional Operations** - HTTP-style conditional requests with If-Match and If-None-Match semantics
- **Optimistic Locking** - Non-blocking concurrency control for high-performance scenarios
- **Conflict Resolution Patterns** - Handling version mismatches and update conflicts gracefully
- **Production-Ready Patterns** - Real-world concurrency scenarios and best practices

## Key Features Showcased

### Automatic ETag Generation
See how the [`ResourceProvider`](https://docs.rs/scim-server/latest/scim_server/trait.ResourceProvider.html) automatically generates version identifiers for every resource, enabling precise conflict detection without manual version management.

### Conditional Update Operations
Watch [`ConditionalOperations`](https://docs.rs/scim-server/latest/scim_server/providers/helpers/conditional/trait.ConditionalOperations.html) in action, demonstrating how to perform updates only when the expected version matches the current resource state.

### Version Conflict Handling
Explore different [`ConditionalResult`](https://docs.rs/scim-server/latest/scim_server/resource/version/enum.ConditionalResult.html) outcomes and learn how to implement proper retry logic and conflict resolution strategies.

### HTTP ETag Integration
The example shows how [`HttpVersion`](https://docs.rs/scim-server/latest/scim_server/resource/version/struct.HttpVersion.html) and [`RawVersion`](https://docs.rs/scim-server/latest/scim_server/resource/version/struct.RawVersion.html) types integrate with standard HTTP caching and conditional request mechanisms.

## Concepts Explored

This example demonstrates advanced concurrency concepts:

- **[Concurrency Control](../concepts/concurrency.md)** - Complete overview of version-based conflict prevention
- **[Resource Providers](../concepts/resource-providers.md)** - Provider-level concurrency implementation
- **[Operation Handlers](../concepts/operation-handlers.md)** - Framework integration with ETag support
- **[SCIM Server](../concepts/scim-server.md)** - Server-level concurrency orchestration

## Perfect For Understanding

This example is essential if you're:

- **Building Multi-Client Systems** - Multiple applications updating the same resources
- **Implementing Enterprise Integration** - HR systems, identity providers, and applications synchronizing data
- **Ensuring Data Consistency** - Preventing lost updates in concurrent environments
- **Production Deployment** - Real-world conflict handling and resolution

## Concurrency Scenarios

The example simulates realistic concurrent access patterns:

### Simultaneous Updates
Two clients attempt to update the same user simultaneously, demonstrating how ETag validation prevents the "last writer wins" problem and preserves both sets of changes.

### Retry Logic Implementation
Watch proper retry patterns when version conflicts occur, including exponential backoff and conflict resolution strategies.

### Bulk Operation Safety
See how concurrency control applies to bulk operations, ensuring consistency across multiple resource updates.

### Mixed Operation Types
The example shows how different operation types (create, read, update, delete) interact with version control mechanisms.

## Version Management

Understand the complete version lifecycle:

- **Version Generation** - How ETags are computed from resource content
- **Version Validation** - Comparing expected vs. actual versions
- **Version Evolution** - How versions change with resource updates
- **Version Exposure** - Making versions available to HTTP clients

## Running the Example

```bash
cargo run --example etag_concurrency_example
```

The output shows detailed version tracking, conflict detection, successful conditional operations, and failed operations with proper error handling - all demonstrating production-ready concurrency patterns.

## Production Benefits

This example illustrates critical production capabilities:

- **Data Integrity** - Preventing corruption from concurrent modifications
- **Performance** - Non-blocking optimistic concurrency vs. expensive locking
- **Scalability** - Handling multiple clients without serialization bottlenecks
- **Reliability** - Predictable conflict detection and resolution

## Integration Patterns

See how ETag concurrency integrates with:

- **HTTP Frameworks** - Standard If-Match/If-None-Match header handling
- **Multi-Tenant Systems** - Per-tenant version management
- **AI Agent Operations** - Version-aware automated operations
- **Bulk APIs** - Consistent versioning across multiple resources

## Next Steps

After exploring concurrency control:

- **[Operation Handlers](./operation-handlers.md)** - Framework integration with conditional operations
- **[MCP with ETag Support](./mcp-etag.md)** - Version-aware AI agent operations
- **[Multi-Tenant Server](./multi-tenant.md)** - Tenant-scoped concurrency control

## Source Code

View the complete implementation: [`examples/etag_concurrency_example.rs`](https://github.com/pukeko37/scim-server/blob/main/examples/etag_concurrency_example.rs)