# SCIM Operation Handler Foundation

## Overview

The `ScimOperationHandler` provides a framework-agnostic foundation for SCIM operations that can serve as the basis for multiple integration patterns including HTTP handlers, MCP (Model Context Protocol) tools, CLI applications, and custom integrations.

## Architecture

The operation handler implements a structured request/response pattern that abstracts SCIM operations from specific transport mechanisms:

```
┌─────────────────┐    ┌──────────────────────┐    ┌─────────────────┐
│   HTTP Handler  │    │  MCP Integration     │    │  CLI Tool       │
└─────────┬───────┘    └──────────┬───────────┘    └─────────┬───────┘
          │                       │                          │
          ▼                       ▼                          ▼
    ┌─────────────────────────────────────────────────────────────────┐
    │              ScimOperationHandler                               │
    │  ┌─────────────────────────────────────────────────────────┐    │
    │  │            ScimOperationRequest                         │    │
    │  │  • operation: Create/Get/Update/Delete/List/Search      │    │
    │  │  • resource_type: "User", "Group", etc.                │    │
    │  │  • data: JSON payload                                  │    │
    │  │  • query: Search/filter parameters                     │    │
    │  │  • tenant_context: Multi-tenant support                │    │
    │  └─────────────────────────────────────────────────────────┘    │
    │                             │                                   │
    │                             ▼                                   │
    │  ┌─────────────────────────────────────────────────────────┐    │
    │  │           ScimOperationResponse                         │    │
    │  │  • success: bool                                        │    │
    │  │  • data: JSON result                                   │    │
    │  │  • error: Error message                                │    │
    │  │  • error_code: Machine-readable error type             │    │
    │  │  • metadata: Operation context and statistics          │    │
    │  └─────────────────────────────────────────────────────────┘    │
    └─────────────────────────────────────────────────────────────────┘
                                  │
                                  ▼
                      ┌─────────────────────┐
                      │    ScimServer       │
                      │  ┌───────────────┐  │
                      │  │ResourceProvider│ │
                      │  └───────────────┘  │
                      └─────────────────────┘
```

## Core Components

### ScimOperationRequest

Structured request type that encapsulates all operation parameters:

```rust
pub struct ScimOperationRequest {
    pub operation: ScimOperationType,
    pub resource_type: String,
    pub resource_id: Option<String>,
    pub data: Option<Value>,
    pub query: Option<ScimQuery>,
    pub tenant_context: Option<TenantContext>,
    pub request_id: Option<String>,
}
```

**Builder Methods:**
- `ScimOperationRequest::create(resource_type, data)`
- `ScimOperationRequest::get(resource_type, resource_id)`
- `ScimOperationRequest::update(resource_type, resource_id, data)`
- `ScimOperationRequest::delete(resource_type, resource_id)`
- `ScimOperationRequest::list(resource_type)`
- `ScimOperationRequest::search(resource_type, attribute, value)`
- `ScimOperationRequest::get_schemas()`
- `ScimOperationRequest::get_schema(schema_id)`

### ScimOperationResponse

Structured response with comprehensive metadata:

```rust
pub struct ScimOperationResponse {
    pub success: bool,
    pub data: Option<Value>,
    pub error: Option<String>,
    pub error_code: Option<String>,
    pub metadata: OperationMetadata,
}
```

**Metadata includes:**
- Resource information (type, ID, count)
- Request tracing (request_id, tenant_id)
- Schema information
- Additional context-specific data

### Supported Operations

| Operation | Description | Required Fields | Response Data |
|-----------|-------------|-----------------|---------------|
| `Create` | Create new resource | `data` | Created resource JSON |
| `Get` | Retrieve resource by ID | `resource_id` | Resource JSON or null |
| `Update` | Update existing resource | `resource_id`, `data` | Updated resource JSON |
| `Delete` | Delete resource | `resource_id` | null |
| `List` | List all resources | - | Array of resources |
| `Search` | Find resource by attribute | `query` with search params | Resource JSON or null |
| `GetSchemas` | Get all schemas | - | Array of schema definitions |
| `GetSchema` | Get specific schema | `resource_id` (schema ID) | Schema definition |
| `Exists` | Check resource existence | `resource_id` | `{"exists": boolean}` |

## Usage Patterns

### Basic Usage

```rust
use scim_server::{
    ScimServer, 
    operation_handler::{ScimOperationHandler, ScimOperationRequest},
    providers::InMemoryProvider
};

// Setup
let provider = InMemoryProvider::new();
let server = ScimServer::new(provider)?;
let handler = ScimOperationHandler::new(server);

// Create user
let request = ScimOperationRequest::create("User", json!({
    "userName": "john.doe",
    "name": {"givenName": "John", "familyName": "Doe"}
}));

let response = handler.handle_operation(request).await;
if response.success {
    println!("Created user: {}", response.data.unwrap());
}
```

### Multi-Tenant Usage

```rust
use scim_server::resource::TenantContext;

let tenant_context = TenantContext::new("tenant-a".to_string(), "client-a".to_string());

let request = ScimOperationRequest::create("User", user_data)
    .with_tenant(tenant_context)
    .with_request_id("req-123");

let response = handler.handle_operation(request).await;
```

### Error Handling

```rust
let response = handler.handle_operation(request).await;

if !response.success {
    match response.error_code.as_deref() {
        Some("RESOURCE_NOT_FOUND") => {
            // Handle not found
        },
        Some("VALIDATION_ERROR") => {
            // Handle validation failure
        },
        Some("UNSUPPORTED_RESOURCE_TYPE") => {
            // Handle unsupported type
        },
        _ => {
            // Handle other errors
        }
    }
}
```

## Integration Patterns

### HTTP Framework Integration

The operation handler can be easily integrated with any HTTP framework:

```rust
// Axum example
async fn scim_endpoint(
    Path((resource_type, operation)): Path<(String, String)>,
    Json(body): Json<Value>,
    handler: Extension<ScimOperationHandler<MyProvider>>,
) -> impl IntoResponse {
    let request = match operation.as_str() {
        "create" => ScimOperationRequest::create(resource_type, body),
        "get" => ScimOperationRequest::get(resource_type, body["id"].as_str().unwrap()),
        _ => return StatusCode::BAD_REQUEST.into_response(),
    };
    
    let response = handler.handle_operation(request).await;
    
    if response.success {
        Json(response.data).into_response()
    } else {
        (StatusCode::BAD_REQUEST, Json(json!({"error": response.error}))).into_response()
    }
}
```

### MCP Tool Integration

For AI agent integration via Model Context Protocol:

```rust
pub struct ScimMcpTools<P: ResourceProvider> {
    handler: ScimOperationHandler<P>,
}

impl<P: ResourceProvider> ScimMcpTools<P> {
    pub async fn create_user(&self, user_data: Value, tenant_id: Option<String>) -> ToolResult {
        let mut request = ScimOperationRequest::create("User", user_data);
        
        if let Some(tid) = tenant_id {
            let tenant_ctx = TenantContext::new(tid, "mcp-client".to_string());
            request = request.with_tenant(tenant_ctx);
        }
        
        let response = self.handler.handle_operation(request).await;
        
        if response.success {
            ToolResult::success(response.data.unwrap())
        } else {
            ToolResult::error(response.error.unwrap())
        }
    }
}
```

### CLI Tool Integration

For command-line applications:

```rust
#[derive(Parser)]
enum ScimCommand {
    Create { resource_type: String, data: String },
    Get { resource_type: String, id: String },
    List { resource_type: String },
}

async fn handle_cli_command(cmd: ScimCommand, handler: &ScimOperationHandler<impl ResourceProvider>) {
    let request = match cmd {
        ScimCommand::Create { resource_type, data } => {
            let json_data: Value = serde_json::from_str(&data)?;
            ScimOperationRequest::create(resource_type, json_data)
        },
        ScimCommand::Get { resource_type, id } => {
            ScimOperationRequest::get(resource_type, id)
        },
        ScimCommand::List { resource_type } => {
            ScimOperationRequest::list(resource_type)
        },
    };
    
    let response = handler.handle_operation(request).await;
    println!("{}", serde_json::to_string_pretty(&response)?);
}
```

## Error Codes

The operation handler provides standardized error codes for programmatic handling:

| Error Code | Description | HTTP Status Equivalent |
|------------|-------------|------------------------|
| `VALIDATION_ERROR` | Schema validation failed | 400 Bad Request |
| `RESOURCE_NOT_FOUND` | Resource doesn't exist | 404 Not Found |
| `SCHEMA_NOT_FOUND` | Schema doesn't exist | 404 Not Found |
| `UNSUPPORTED_RESOURCE_TYPE` | Resource type not registered | 400 Bad Request |
| `UNSUPPORTED_OPERATION` | Operation not allowed | 405 Method Not Allowed |
| `INVALID_REQUEST` | Malformed request | 400 Bad Request |
| `PROVIDER_ERROR` | Backend provider error | 500 Internal Server Error |
| `INTERNAL_ERROR` | Unexpected server error | 500 Internal Server Error |

## Performance Considerations

### Async-First Design
- All operations are async for high concurrency
- Non-blocking I/O throughout the stack
- Efficient resource utilization

### Memory Efficiency
- Structured requests avoid string parsing overhead
- Shared schema registry across operations
- Minimal allocations in hot paths

### Request Tracing
- Built-in request ID generation and propagation
- Tenant context isolation
- Comprehensive operation metadata

## Testing Support

The operation handler is designed for easy testing:

```rust
#[tokio::test]
async fn test_user_operations() {
    let provider = InMemoryProvider::new();
    let mut server = ScimServer::new(provider).unwrap();
    
    // Register test resource types
    let user_schema = server.get_schema_by_id("urn:ietf:params:scim:schemas:core:2.0:User").unwrap().clone();
    let user_handler = create_user_resource_handler(user_schema);
    server.register_resource_type("User", user_handler, vec![ScimOperation::Create]).unwrap();
    
    let handler = ScimOperationHandler::new(server);
    
    // Test operations
    let request = ScimOperationRequest::create("User", json!({"userName": "test"}));
    let response = handler.handle_operation(request).await;
    
    assert!(response.success);
    assert!(response.data.is_some());
}
```

## Best Practices

### Request Construction
- Use builder methods for type safety
- Always include request IDs for tracing
- Set tenant context for multi-tenant scenarios

### Error Handling
- Check `success` field before accessing `data`
- Use `error_code` for programmatic error handling
- Log `metadata.request_id` for debugging

### Performance
- Reuse `ScimOperationHandler` instances
- Batch operations when possible
- Use appropriate query parameters for list operations

### Security
- Validate input data before creating requests
- Use tenant context for proper isolation
- Implement authentication/authorization at the transport layer

## Future Extensions

The operation handler foundation is designed for extensibility:

- **Bulk Operations**: Support for batch requests
- **Streaming**: Large result set streaming
- **Caching**: Transparent operation caching
- **Metrics**: Built-in operation metrics collection
- **Hooks**: Pre/post operation hooks for custom logic

## Conclusion

The `ScimOperationHandler` provides a clean, type-safe foundation for SCIM operations that can be adapted to any integration pattern. By abstracting the core SCIM logic from transport concerns, it enables consistent behavior across HTTP, MCP, CLI, and custom integrations while maintaining performance and reliability.