# Operation Handlers

Operation Handlers are the framework-agnostic bridge between transport layers and the SCIM Server core, providing structured request/response handling with built-in concurrency control and comprehensive error management. They abstract SCIM operations into a transport-neutral interface that can be used with HTTP frameworks, MCP protocols, CLI tools, or any other integration pattern.

See the [Operation Handler API documentation](https://docs.rs/scim-server/latest/scim_server/operation_handler/index.html) for complete details.

## Value Proposition

Operation Handlers deliver critical integration capabilities:

- **Framework Agnostic**: Work with any transport layer (HTTP, MCP, CLI, custom protocols)
- **Structured Interface**: Consistent request/response patterns across all operation types
- **Built-in ETag Support**: Automatic version control and concurrency management
- **Comprehensive Error Handling**: Structured error responses with proper SCIM compliance
- **Request Tracing**: Built-in request ID correlation and operational logging
- **Multi-Tenant Aware**: Seamless tenant context propagation through operations
- **Type-Safe Operations**: Strongly typed operation dispatch with compile-time safety

## Architecture Overview

Operation Handlers sit between transport layers and the SCIM Server core:

```text
Transport Layer (HTTP/MCP/CLI)
    ↓
ScimOperationHandler (Framework Abstraction)
├── Request Structuring & Validation
├── Operation Dispatch & Routing
├── ETag Version Management
├── Error Handling & Translation
└── Response Formatting
    ↓
SCIM Server (Business Logic)
    ↓
Resource Providers & Storage
```

### Core Components

1. **[`ScimOperationHandler`](https://docs.rs/scim-server/latest/scim_server/operation_handler/struct.ScimOperationHandler.html)**: Main dispatcher for all SCIM operations
2. **[`ScimOperationRequest`](https://docs.rs/scim-server/latest/scim_server/operation_handler/struct.ScimOperationRequest.html)**: Structured request wrapper with validation
3. **[`ScimOperationResponse`](https://docs.rs/scim-server/latest/scim_server/operation_handler/struct.ScimOperationResponse.html)**: Consistent response format with metadata
4. **[`OperationMetadata`](https://docs.rs/scim-server/latest/scim_server/operation_handler/struct.OperationMetadata.html)**: Version control, tracing, and operational data
5. **Builder Utilities**: Convenient construction helpers for requests

## Use Cases

### 1. HTTP Framework Integration

**Building REST APIs with any HTTP framework**

```rust
use scim_server::operation_handler::{ScimOperationHandler, ScimOperationRequest};
use axum::{Json, Path, extract::Query, response::Json as ResponseJson};

// Setup once
let handler = ScimOperationHandler::new(scim_server);

// HTTP handler functions
async fn create_user(
    Path(resource_type): Path<String>,
    Json(data): Json<Value>
) -> ResponseJson<Value> {
    let request = ScimOperationRequest::create(resource_type, data)
        .with_request_id(uuid::Uuid::new_v4().to_string());
    
    let response = handler.handle_operation(request).await;
    
    if response.success {
        ResponseJson(response.data.unwrap())
    } else {
        // Handle error appropriately
        ResponseJson(json!({"error": response.error}))
    }
}

async fn get_user(
    Path((resource_type, id)): Path<(String, String)>
) -> ResponseJson<Value> {
    let request = ScimOperationRequest::get(resource_type, id);
    let response = handler.handle_operation(request).await;
    // Handle response...
}
```

**Benefits**: Framework independence, consistent error handling, automatic ETag support.

### 2. MCP Protocol Integration

**AI agent tool integration through Model Context Protocol**

```rust
// MCP tool handler
async fn handle_scim_tool(tool_name: &str, args: Value) -> McpResult {
    let request = match tool_name {
        "create_user" => {
            ScimOperationRequest::create("User", args["user_data"].clone())
                .with_tenant_context(extract_tenant_from_args(&args)?)
        },
        "get_user" => {
            ScimOperationRequest::get("User", args["user_id"].as_str().unwrap())
        },
        "update_user" => {
            ScimOperationRequest::update(
                "User", 
                args["user_id"].as_str().unwrap(),
                args["user_data"].clone()
            ).with_expected_version(args["expected_version"].as_str())
        },
        _ => return Err(McpError::UnknownTool),
    };
    
    let response = operation_handler.handle_operation(request).await;
    
    McpResult {
        success: response.success,
        content: response.data,
        metadata: response.metadata,
    }
}
```

**Benefits**: Structured AI interactions, automatic version management, consistent tool interface.

### 3. CLI Tool Development

**Command-line identity management tools**

```rust
// CLI command handlers
async fn cli_create_user(args: &CreateUserArgs) -> CliResult {
    let user_data = json!({
        "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
        "userName": args.username,
        "displayName": args.display_name,
        "active": true
    });
    
    let request = ScimOperationRequest::create("User", user_data)
        .with_tenant_context(args.tenant_context.clone())
        .with_request_id(format!("cli-{}", uuid::Uuid::new_v4()));
    
    let response = handler.handle_operation(request).await;
    
    if response.success {
        println!("✅ User created successfully");
        if let Some(etag) = response.metadata.additional.get("etag") {
            println!("   ETag: {}", etag.as_str().unwrap());
        }
        Ok(())
    } else {
        eprintln!("❌ Failed to create user: {}", response.error.unwrap());
        Err(CliError::OperationFailed)
    }
}
```

**Benefits**: Consistent command behavior, built-in error reporting, automatic version tracking.

### 4. Batch Processing Systems

**Bulk identity operations with concurrency control**

```rust
async fn batch_update_users(updates: Vec<UserUpdate>) -> BatchResult {
    let mut results = Vec::new();
    
    for update in updates {
        let request = ScimOperationRequest::update("User", &update.id, update.data)
            .with_expected_version(update.expected_version)
            .with_request_id(format!("batch-{}", uuid::Uuid::new_v4()));
        
        let response = handler.handle_operation(request).await;
        
        results.push(BatchUpdateResult {
            user_id: update.id.clone(),
            success: response.success,
            error: response.error,
            new_version: response.metadata.additional
                .get("version")
                .and_then(|v| v.as_str())
                .map(String::from),
        });
    }
    
    BatchResult { 
        total: updates.len(),
        succeeded: results.iter().filter(|r| r.success).count(),
        results 
    }
}
```

**Benefits**: Built-in concurrency control, consistent error handling, operation tracing.

### 5. Custom Protocol Integration

**Embedding SCIM in domain-specific protocols**

```rust
// Custom protocol message handler
async fn handle_identity_message(msg: IdentityMessage) -> ProtocolResponse {
    let request = match msg.operation {
        IdentityOperation::Provision { user_spec } => {
            ScimOperationRequest::create("User", user_spec.to_scim_json())
                .with_tenant_context(msg.tenant_context)
        },
        IdentityOperation::Deprovision { user_id } => {
            ScimOperationRequest::delete("User", user_id)
                .with_tenant_context(msg.tenant_context)
        },
        IdentityOperation::Query { criteria } => {
            ScimOperationRequest::search("User")
                .with_query(criteria.to_scim_query())
                .with_tenant_context(msg.tenant_context)
        },
    };
    
    let response = handler.handle_operation(request).await;
    
    ProtocolResponse {
        correlation_id: msg.correlation_id,
        success: response.success,
        payload: response.data,
        error_details: response.error,
    }
}
```

**Benefits**: Protocol-agnostic operations, consistent behavior patterns, built-in error handling.

## Design Patterns

### Request Builder Pattern

Fluent API for constructing operation requests:

```rust
let request = ScimOperationRequest::update("User", "123", user_data)
    .with_tenant_context(tenant_ctx)
    .with_request_id("req-456")
    .with_expected_version("v1.2.3");
```

This provides type-safe request construction with optional parameters.

### Structured Response Pattern

Consistent response format across all operations:

```rust
pub struct ScimOperationResponse {
    pub success: bool,
    pub data: Option<Value>,
    pub error: Option<String>,
    pub error_code: Option<String>,
    pub metadata: OperationMetadata,
}
```

This ensures uniform error handling and metadata access.

### Operation Dispatch Pattern

Type-safe operation routing:

```rust
match request.operation {
    ScimOperationType::Create => handle_create(handler, request, context).await,
    ScimOperationType::Update => handle_update(handler, request, context).await,
    ScimOperationType::Get => handle_get(handler, request, context).await,
    // ... other operations
}
```

This provides compile-time guarantees about operation handling.

### Metadata Propagation Pattern

Automatic metadata management:

```rust
pub struct OperationMetadata {
    pub request_id: String,
    pub tenant_id: Option<String>,
    pub resource_count: Option<usize>,
    pub additional: HashMap<String, Value>, // Includes ETag info
}
```

This ensures consistent metadata across all operations.

## Integration with Other Components

### SCIM Server Integration

Operation Handlers orchestrate SCIM Server operations:

- **Operation Dispatch**: Routes structured requests to appropriate server methods
- **Context Management**: Ensures proper tenant context flows through operations
- **Response Formatting**: Converts server responses to structured format
- **Error Translation**: Maps SCIM errors to transport-appropriate formats

### Version Control Integration

Built-in ETag concurrency control:

- **Automatic Versioning**: All resources include version information in responses
- **Conditional Operations**: Support for If-Match/If-None-Match semantics
- **Conflict Detection**: Structured responses for version conflicts
- **Version Propagation**: Version metadata included in all operation responses

### Multi-Tenant Integration

Seamless tenant context handling:

- **Context Extraction**: Automatically extracts tenant information from requests
- **Tenant Validation**: Ensures tenant context is properly validated
- **Scoped Operations**: All operations automatically tenant-scoped
- **Tenant Metadata**: Tenant information included in operation metadata

### Error Handling Integration

Comprehensive error management:

- **Structured Errors**: SCIM-compliant error responses with proper status codes
- **Error Propagation**: Consistent error handling across all operation types
- **Debug Information**: Rich error context for troubleshooting
- **Transport Agnostic**: Error format suitable for any transport layer

## Best Practices

### 1. Use Request Builders for Complex Operations

```rust
// Good: Use builder pattern for readable construction
let request = ScimOperationRequest::update("User", user_id, user_data)
    .with_tenant_context(tenant_context)
    .with_expected_version(current_version)
    .with_request_id(correlation_id);

// Avoid: Manual struct construction
let request = ScimOperationRequest {
    operation: ScimOperationType::Update,
    resource_type: "User".to_string(),
    // ... many fields
};
```

### 2. Always Handle Version Information

```rust
// Good: Check and use version information
let response = handler.handle_operation(request).await;
if response.success {
    let new_version = response.metadata.additional
        .get("version")
        .and_then(|v| v.as_str());
    // Store version for next operation
}

// Avoid: Ignoring version information
// This leads to lost updates and concurrency issues
```

### 3. Implement Proper Error Handling

```rust
// Good: Handle different error types appropriately
match response.error_code.as_deref() {
    Some("version_conflict") => {
        // Handle version conflict specifically
        retry_with_fresh_version().await
    },
    Some("resource_not_found") => {
        // Handle missing resource
        return NotFoundError;
    },
    Some(_) | None if !response.success => {
        // Handle other errors
        log_error(&response.error);
        return ServerError;
    },
    _ => {
        // Success case
        process_response_data(response.data)
    }
}

// Avoid: Generic error handling that loses context
if !response.success {
    return Err("Operation failed");
}
```

### 4. Use Request IDs for Tracing

```rust
// Good: Consistent request ID usage
let request_id = generate_correlation_id();
let request = ScimOperationRequest::create("User", data)
    .with_request_id(request_id.clone());

let response = handler.handle_operation(request).await;
log::info!("Operation {} completed: {}", request_id, response.success);

// Avoid: No request correlation
// This makes debugging and tracing difficult
```

### 5. Leverage Tenant Context Appropriately

```rust
// Good: Explicit tenant context handling
let request = if let Some(tenant_ctx) = extract_tenant_from_auth(auth_header) {
    ScimOperationRequest::create("User", data)
        .with_tenant_context(tenant_ctx)
} else {
    ScimOperationRequest::create("User", data) // Single tenant
};

// Avoid: Ignoring tenant context in multi-tenant scenarios
// This can lead to cross-tenant data access
```

## When to Use Operation Handlers

### Primary Scenarios

1. **HTTP Framework Integration**: Building REST APIs that expose SCIM endpoints
2. **Protocol Integration**: Adapting SCIM to custom protocols (MCP, GraphQL, gRPC)
3. **CLI Tools**: Building command-line identity management utilities
4. **Batch Processing**: Implementing bulk identity operations
5. **Testing Frameworks**: Creating test harnesses that need structured SCIM operations

### Implementation Strategies

| Scenario | Approach | Complexity | Benefits |
|----------|----------|------------|----------|
| REST API | Direct handler integration | Low | Framework independence, built-in ETag |
| MCP Protocol | Tool handler delegation | Medium | Structured AI interactions |
| CLI Tools | Command handler wrapper | Low | Consistent CLI behavior |
| Batch Processing | Async handler coordination | Medium | Concurrency control, error handling |
| Custom Protocols | Protocol adapter layer | High | Protocol flexibility, SCIM compliance |

## Comparison with Direct SCIM Server Usage

| Approach | Abstraction | Error Handling | Version Control | Complexity |
|----------|-------------|----------------|-----------------|------------|
| **Operation Handlers** | ✅ High | ✅ Structured | ✅ Built-in | Low |
| Direct SCIM Server | ⚠️ Medium | ⚠️ Manual | ⚠️ Manual | Medium |
| Custom Integration | ❌ Low | ❌ Ad-hoc | ❌ Custom | High |

Operation Handlers provide the optimal balance of abstraction and functionality for most integration scenarios, offering structured operations with built-in best practices.

## Framework Examples

### Axum Integration

```rust
async fn axum_create_resource(
    Path(resource_type): Path<String>,
    headers: HeaderMap,
    Json(data): Json<Value>
) -> Result<Json<Value>, StatusCode> {
    let request = ScimOperationRequest::create(resource_type, data)
        .with_request_id(extract_request_id(&headers));
    
    let response = handler.handle_operation(request).await;
    
    if response.success {
        Ok(Json(response.data.unwrap()))
    } else {
        Err(map_error_to_status(&response))
    }
}
```

### Warp Integration

```rust
fn warp_scim_routes() -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
    warp::path("Users")
        .and(warp::post())
        .and(warp::body::json())
        .and_then(move |data: Value| {
            let handler = handler.clone();
            async move {
                let request = ScimOperationRequest::create("User", data);
                let response = handler.handle_operation(request).await;
                Ok::<_, warp::Rejection>(warp::reply::json(&response.data))
            }
        })
}
```

Operation Handlers serve as the foundational integration layer that enables SCIM Server to work seamlessly with any transport mechanism while maintaining consistent behavior, proper error handling, and automatic concurrency control across all integration patterns.