# Operation Handlers

This example demonstrates the framework-agnostic operation handler layer that bridges transport protocols and the SCIM server core. It shows how to build structured request/response handling with built-in concurrency control and comprehensive error management.

## What This Example Demonstrates

- **Framework-Agnostic Integration** - Working with any transport layer (HTTP, MCP, CLI, custom protocols)
- **Structured Request/Response Handling** - Consistent patterns across all operation types
- **Built-in ETag Support** - Automatic version control and concurrency management
- **Comprehensive Error Translation** - Converting internal errors to structured responses
- **Request Tracing** - Built-in request ID correlation and operational logging
- **Multi-Tenant Request Handling** - Seamless tenant context propagation

## Key Features Showcased

### Operation Handler Abstraction
See how [`ScimOperationHandler`](https://docs.rs/scim-server/latest/scim_server/operation_handler/struct.ScimOperationHandler.html) provides a clean abstraction layer between HTTP frameworks and SCIM business logic, enabling consistent behavior across different integration patterns.

### Structured Request Processing
Watch [`ScimOperationRequest`](https://docs.rs/scim-server/latest/scim_server/operation_handler/struct.ScimOperationRequest.html) standardize request handling with built-in validation, parameter extraction, and context management - regardless of the underlying transport.

### Consistent Response Formatting
Explore how [`ScimOperationResponse`](https://docs.rs/scim-server/latest/scim_server/operation_handler/struct.ScimOperationResponse.html) ensures uniform response structure with proper HTTP status codes, headers, and JSON formatting across all operations.

### Operational Metadata Management
The example shows [`OperationMetadata`](https://docs.rs/scim-server/latest/scim_server/operation_handler/struct.OperationMetadata.html) handling version control, request tracing, and performance metrics automatically.

## Concepts Explored

This example demonstrates the bridge between transport and business logic:

- **[Operation Handlers](../concepts/operation-handlers.md)** - Complete framework abstraction patterns
- **[SCIM Server](../concepts/scim-server.md)** - Core business logic integration
- **[Concurrency Control](../concepts/concurrency.md)** - Version-aware operation handling
- **[Multi-Tenant Architecture](../concepts/multi-tenant-architecture.md)** - Tenant-aware request processing

## Perfect For Building

This example is essential if you're:

- **Building REST APIs** - HTTP framework integration with any web server
- **Creating Custom Protocols** - Non-HTTP transport layer implementation
- **Implementing Middleware** - Request/response processing pipelines
- **Testing SCIM Operations** - Framework-independent testing harnesses

## Integration Patterns

The example covers multiple integration scenarios:

### HTTP Framework Integration
See how operation handlers work with popular Rust web frameworks:
- **Axum** - Clean async handler integration
- **Actix-web** - Actor-based request processing
- **Warp** - Filter-based routing compatibility
- **Rocket** - Type-safe request handling

### Custom Protocol Support
Explore how the same operation handlers can work with:
- **gRPC Services** - Protocol buffer integration
- **WebSocket Connections** - Real-time operation handling  
- **Command-Line Tools** - CLI-based SCIM operations
- **Message Queues** - Asynchronous operation processing

### Testing and Development
The framework-agnostic design enables:
- **Unit Testing** - Testing business logic without HTTP setup
- **Integration Testing** - Protocol-independent test suites
- **Development Tools** - CLI utilities and debugging tools

## Request Processing Pipeline

Watch the complete request lifecycle:

1. **Request Structuring** - Converting transport-specific requests to standard format
2. **Validation** - Parameter checking and constraint enforcement
3. **Context Extraction** - Tenant and authentication information processing
4. **Operation Dispatch** - Routing to appropriate business logic handlers
5. **Response Formatting** - Converting results to transport-appropriate format

## Error Handling Excellence

The example demonstrates sophisticated error management:

- **Error Translation** - Converting internal errors to appropriate HTTP status codes
- **Structured Responses** - Consistent error format across all operations
- **Context Preservation** - Maintaining request context through error scenarios
- **Logging Integration** - Comprehensive error tracking and debugging support

## Running the Example

```bash
cargo run --example operation_handler_example
```

The output shows complete request/response cycles with detailed logging, error scenarios, and performance metrics - demonstrating production-ready operation handling.

## Production Benefits

This example illustrates critical production capabilities:

- **Transport Flexibility** - Easy migration between different protocols and frameworks
- **Consistent Behavior** - Same business logic regardless of integration method
- **Operational Visibility** - Built-in logging, metrics, and tracing
- **Error Resilience** - Graceful handling of edge cases and failures

## Advanced Features

Explore sophisticated operation handler capabilities:

- **Conditional Operations** - Built-in ETag support for concurrency control
- **Bulk Operation Support** - Efficient handling of multiple resources
- **Schema Validation** - Automatic request/response validation
- **Performance Optimization** - Minimal overhead and maximum throughput

## Next Steps

After exploring operation handlers:

- **[ETag Concurrency Control](./etag-concurrency.md)** - Add version-aware operations
- **[Multi-Tenant Server](./multi-tenant.md)** - Tenant-aware request processing
- **[MCP Server](./mcp-server.md)** - AI agent protocol integration

## Source Code

View the complete implementation: [`examples/operation_handler_example.rs`](https://github.com/pukeko37/scim-server/blob/main/examples/operation_handler_example.rs)

## Related Documentation

- **[Operation Handlers Concepts](../concepts/operation-handlers.md)** - Architectural overview and design patterns
- **[Configuration Guide](../getting-started/configuration.md)** - Integrating handlers with server setup
- **[Operation Handler API Reference](https://docs.rs/scim-server/latest/scim_server/operation_handler/index.html)** - Complete API documentation