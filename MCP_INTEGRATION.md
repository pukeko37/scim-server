# MCP (Model Context Protocol) Integration for SCIM Server

This document describes the MCP integration feature that allows AI agents to interact with the SCIM server through a standardized protocol.

## Overview

The MCP integration exposes SCIM server functionality as a set of tools that AI agents can discover and use. This enables AI systems to perform identity management operations like creating users, managing groups, and querying schemas through a structured interface.

## Features

- **Complete SCIM Operation Coverage**: All major SCIM operations (Create, Read, Update, Delete, List, Search) are exposed as MCP tools
- **Multi-Tenant Support**: AI agents can work with specific tenants for isolated operations
- **Schema Introspection**: AI agents can discover and understand available SCIM schemas
- **Error Handling**: Comprehensive error responses help AI agents understand and recover from failures
- **Type Safety**: All operations are validated according to SCIM schemas

## Enabling MCP Integration

The MCP integration is available as an optional feature. Enable it in your `Cargo.toml`:

```toml
[dependencies]
scim-server = { version = "0.1.0", features = ["mcp"] }
```

## Quick Start

```rust
use scim_server::{ScimServer, providers::InMemoryProvider};
use scim_server::mcp_integration::{ScimMcpServer, McpServerInfo};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create a SCIM server
    let provider = InMemoryProvider::new();
    let scim_server = ScimServer::new(provider)?;
    
    // Create MCP server
    let mcp_server = ScimMcpServer::new(scim_server);
    
    // Run with stdio transport (for AI agent communication)
    mcp_server.run_stdio().await?;
    
    Ok(())
}
```

## Available MCP Tools

The integration provides the following tools for AI agents:

### User Management
- `scim_create_user` - Create a new user
- `scim_get_user` - Retrieve a user by ID
- `scim_update_user` - Update user information
- `scim_delete_user` - Delete a user
- `scim_list_users` - List all users with pagination
- `scim_search_users` - Search users by attribute
- `scim_user_exists` - Check if a user exists

### Schema Operations
- `scim_get_schemas` - Get all available SCIM schemas
- `scim_server_info` - Get server capabilities and information

## Tool Usage Examples

### Creating a User

```json
{
  "tool": "scim_create_user",
  "arguments": {
    "user_data": {
      "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
      "userName": "ai.assistant",
      "name": {
        "givenName": "AI",
        "familyName": "Assistant"
      },
      "emails": [
        {
          "value": "ai.assistant@company.com",
          "type": "work",
          "primary": true
        }
      ],
      "active": true
    },
    "tenant_id": "company-a"  // Optional for multi-tenant scenarios
  }
}
```

### Searching for Users

```json
{
  "tool": "scim_search_users",
  "arguments": {
    "attribute": "userName",
    "value": "ai.assistant",
    "tenant_id": "company-a"  // Optional
  }
}
```

### Getting Schema Information

```json
{
  "tool": "scim_get_schemas",
  "arguments": {}
}
```

## Multi-Tenant Support

AI agents can work with specific tenants by including a `tenant_id` parameter in their tool calls. This enables:

- **Tenant Isolation**: Operations are scoped to specific tenant contexts
- **Multi-Client Scenarios**: Different AI agents can work with different tenants
- **Access Control**: Tenant-specific permissions and data isolation

## Error Handling

The MCP integration provides structured error responses that help AI agents understand what went wrong:

```json
{
  "success": false,
  "content": {
    "error": "Missing user_id parameter",
    "error_code": "GET_USER_FAILED"
  },
  "metadata": null
}
```

Common error scenarios:
- Missing required parameters
- Validation failures
- Resource not found
- Permission denied
- Schema violations

## Server Configuration

You can customize the MCP server information:

```rust
use scim_server::mcp_integration::McpServerInfo;

let server_info = McpServerInfo {
    name: "Enterprise SCIM Server".to_string(),
    version: "1.0.0".to_string(),
    description: "Production SCIM server with AI integration".to_string(),
    supported_resource_types: vec!["User".to_string(), "Group".to_string()],
};

let mcp_server = ScimMcpServer::with_info(scim_server, server_info);
```

## Testing the Integration

Run the comprehensive example to see the MCP integration in action:

```bash
cargo run --example mcp_server_example --features mcp
```

This demonstrates:
- Tool discovery
- User lifecycle operations
- Multi-tenant scenarios
- Error handling
- Schema introspection

## Integration with AI Frameworks

The MCP server can be integrated with various AI frameworks that support the Model Context Protocol:

1. **Claude Desktop**: Configure the SCIM server as an MCP server
2. **Custom AI Applications**: Use MCP client libraries to connect
3. **Development Tools**: Integrate with IDEs and development environments

## Architecture

```
┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐
│   AI Agent      │───▶│  MCP Server     │───▶│  SCIM Server    │
│                 │    │                 │    │                 │
│ - Discovers     │    │ - Tool Registry │    │ - User Store    │
│   tools         │    │ - Request       │    │ - Schema        │
│ - Calls         │    │   Translation   │    │   Validation    │
│   operations    │    │ - Error         │    │ - Multi-tenant  │
│ - Handles       │    │   Handling      │    │   Support       │
│   responses     │    │                 │    │                 │
└─────────────────┘    └─────────────────┘    └─────────────────┘
```

## Security Considerations

- **Input Validation**: All tool inputs are validated against SCIM schemas
- **Tenant Isolation**: Multi-tenant operations maintain data separation
- **Error Sanitization**: Error messages don't leak sensitive information
- **Rate Limiting**: Consider implementing rate limiting for AI agent operations

## Future Enhancements

Planned improvements include:
- Group management tools
- Bulk operations support
- Advanced filtering and sorting
- Webhook notifications
- Real-time updates via MCP subscriptions

## Dependencies

The MCP integration requires the following dependencies:

```toml
rust-mcp-sdk = "0.5"
async-trait = "0.1"
serde_json = "1.0"
```

These are automatically included when the `mcp` feature is enabled.

## Troubleshooting

### Common Issues

1. **Tool Not Found**: Ensure the MCP feature is enabled and the server is properly initialized
2. **Validation Errors**: Check that tool arguments match the expected schema
3. **Permission Denied**: Verify tenant permissions and access controls
4. **Connection Issues**: Ensure the stdio transport is properly configured

### Debug Logging

Enable debug logging to troubleshoot issues:

```rust
env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("debug")).init();
```

## Contributing

To contribute to the MCP integration:

1. Ensure tests pass: `cargo test --features mcp`
2. Add new tools following the existing patterns
3. Update documentation for new features
4. Maintain backward compatibility

## License

The MCP integration is part of the SCIM server project and follows the same licensing terms.