# Setting Up Your MCP Server

This guide shows you how to set up a working Model Context Protocol (MCP) server that exposes your SCIM operations as discoverable tools for AI agents.

## What is MCP Integration?

The MCP integration allows AI agents to interact with your SCIM server through a standardized protocol. AI agents can discover available tools (like "create user" or "search users") and execute them with proper validation and error handling.

**Key Benefits:**
- **AI-Friendly Interface** - Structured tool discovery and execution
- **Multi-Tenant Support** - Isolated operations for different clients
- **Schema Introspection** - AI agents can understand your data model
- **Error Handling** - Graceful error responses with detailed information

## Quick Start

### 1. Enable the MCP Feature

Add the MCP feature to your `Cargo.toml`:

```toml
[dependencies]
scim-server = { version = "0.5.0", features = ["mcp"] }
tokio = { version = "1.0", features = ["full"] }
serde_json = "1.0"
env_logger = "0.10"  # For logging (recommended)
```

### 2. Basic MCP Server (30 lines)

Create a minimal MCP server that exposes SCIM operations:

```rust
use scim_server::{
    mcp_integration::ScimMcpServer,
    multi_tenant::ScimOperation,
    providers::StandardResourceProvider,
    resource_handlers::create_user_resource_handler,
    scim_server::ScimServer,
    storage::InMemoryStorage,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Initialize logging for debugging
    env_logger::init();

    // Create SCIM server with in-memory storage
    let storage = InMemoryStorage::new();
    let provider = StandardResourceProvider::new(storage);
    let mut scim_server = ScimServer::new(provider)?;

    // Register User resource type with full operations
    let user_schema = scim_server
        .get_schema_by_id("urn:ietf:params:scim:schemas:core:2.0:User")?
        .clone();
    let user_handler = create_user_resource_handler(user_schema);
    
    scim_server.register_resource_type(
        "User",
        user_handler,
        vec![
            ScimOperation::Create, ScimOperation::Read,
            ScimOperation::Update, ScimOperation::Delete,
            ScimOperation::List, ScimOperation::Search,
        ],
    )?;

    // Create and start MCP server
    let mcp_server = ScimMcpServer::new(scim_server);
    println!("🚀 MCP Server starting - listening on stdio");
    
    // This runs until EOF (Ctrl+D) or process termination
    mcp_server.run_stdio().await?;
    
    Ok(())
}
```

### 3. Run Your MCP Server

```bash
cargo run --features mcp
```

The server starts and listens on standard input/output for MCP protocol messages.

## Testing Your Server

You can test the server by sending JSON-RPC messages directly:

### 1. Initialize Connection
```bash
echo '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{}}}' | your_server
```

### 2. Discover Available Tools
```bash
echo '{"jsonrpc":"2.0","id":2,"method":"tools/list","params":{}}' | your_server
```

### 3. Create a User
```bash
echo '{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"scim_create_user","arguments":{"user_data":{"schemas":["urn:ietf:params:scim:schemas:core:2.0:User"],"userName":"alice@example.com","active":true}}}}' | your_server
```

## Production-Ready Setup

For production use, you'll want a more comprehensive setup:

```rust
use scim_server::{
    mcp_integration::{McpServerInfo, ScimMcpServer},
    multi_tenant::ScimOperation,
    providers::StandardResourceProvider,
    resource_handlers::{create_user_resource_handler, create_group_resource_handler},
    scim_server::ScimServer,
    storage::InMemoryStorage,  // Replace with your database storage
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Production logging configuration
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
        .init();

    // Create SCIM server (use your production storage here)
    let storage = InMemoryStorage::new(); // Replace with PostgresStorage, etc.
    let provider = StandardResourceProvider::new(storage);
    let mut scim_server = ScimServer::new(provider)?;

    // Register User resource type
    let user_schema = scim_server
        .get_schema_by_id("urn:ietf:params:scim:schemas:core:2.0:User")?
        .clone();
    let user_handler = create_user_resource_handler(user_schema);
    
    scim_server.register_resource_type(
        "User",
        user_handler,
        vec![
            ScimOperation::Create, ScimOperation::Read, ScimOperation::Update,
            ScimOperation::Delete, ScimOperation::List, ScimOperation::Search,
        ],
    )?;

    // Register Group resource type (if needed)
    if let Some(group_schema) = scim_server
        .get_schema_by_id("urn:ietf:params:scim:schemas:core:2.0:Group")
    {
        let group_handler = create_group_resource_handler(group_schema.clone());
        scim_server.register_resource_type(
            "Group",
            group_handler,
            vec![
                ScimOperation::Create, ScimOperation::Read, ScimOperation::Update,
                ScimOperation::Delete, ScimOperation::List, ScimOperation::Search,
            ],
        )?;
    }

    // Create MCP server with custom information
    let server_info = McpServerInfo {
        name: "Production SCIM Server".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        description: "Enterprise SCIM server with MCP integration".to_string(),
        supported_resource_types: vec!["User".to_string(), "Group".to_string()],
    };
    
    let mcp_server = ScimMcpServer::with_info(scim_server, server_info);
    
    // Log available tools
    let tools = mcp_server.get_tools();
    log::info!("🔧 Available MCP tools: {}", tools.len());
    for tool in &tools {
        if let Some(name) = tool.get("name").and_then(|n| n.as_str()) {
            log::info!("   • {}", name);
        }
    }
    
    log::info!("🚀 MCP Server ready - listening on stdio");
    
    // Start the server
    mcp_server.run_stdio().await?;
    
    log::info!("✅ MCP Server shutdown complete");
    Ok(())
}
```

## Available MCP Tools

Your MCP server exposes these tools to AI agents:

### User Management
- **`scim_create_user`** - Create a new user
- **`scim_get_user`** - Retrieve user by ID
- **`scim_update_user`** - Update existing user
- **`scim_delete_user`** - Delete user by ID
- **`scim_list_users`** - List all users with pagination
- **`scim_search_users`** - Search users by attribute
- **`scim_user_exists`** - Check if user exists

### System Information
- **`scim_get_schemas`** - Get all SCIM schemas
- **`scim_server_info`** - Get server capabilities and info

## Multi-Tenant Support

The MCP server supports multi-tenant operations out of the box:

```json
{
  "jsonrpc": "2.0",
  "id": 4,
  "method": "tools/call",
  "params": {
    "name": "scim_create_user",
    "arguments": {
      "user_data": {
        "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
        "userName": "bob@tenant-a.com",
        "active": true
      },
      "tenant_id": "tenant-a"
    }
  }
}
```

Each tenant's data is completely isolated from others.

## Integration with AI Agents

Your MCP server can be used with any MCP-compatible AI agent or framework. The AI agent will:

1. **Initialize** - Establish connection and capabilities
2. **Discover Tools** - Get list of available SCIM operations
3. **Get Schemas** - Understand your data model structure  
4. **Execute Operations** - Create, read, update, delete resources
5. **Handle Errors** - Process validation and operation errors gracefully

## Error Handling

The MCP server provides structured error responses for AI agents:

```json
{
  "jsonrpc": "2.0",
  "id": 5,
  "error": {
    "code": -32000,
    "message": "Tool execution failed: Validation error: userName is required"
  }
}
```

## Logging and Monitoring

Enable comprehensive logging for production deployments:

```rust
// In your main function
env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
    .format_timestamp_secs()
    .init();

log::info!("MCP Server starting");
log::debug!("Available tools: {:?}", mcp_server.get_tools().len());
```

## Running Examples

The crate includes complete working examples:

```bash
# Basic MCP server
cargo run --example mcp_stdio_server --features mcp

# Comprehensive example with demos
cargo run --example mcp_server_example --features mcp
```

## Next Steps

- **[Storage Backends](../storage/overview.md)** - Replace InMemoryStorage with PostgreSQL or other databases
- **[Multi-Tenant Configuration](../multi-tenant/setup.md)** - Advanced tenant management
- **[Custom Resource Types](../advanced/custom-resources.md)** - Beyond User and Group
- **[Production Deployment](../deployment/mcp-production.md)** - Scaling and monitoring MCP servers

## Complete Working Example

See [`examples/mcp_stdio_server.rs`](../../../../examples/mcp_stdio_server.rs) for a complete, production-ready MCP server implementation with:

- Comprehensive error handling
- Multi-tenant support  
- Full logging configuration
- Tool discovery and execution
- Schema introspection
- Graceful shutdown handling

The MCP integration makes your SCIM server AI-ready with minimal configuration!