# Simple MCP Demo

This example provides the quickest way to get started with MCP (Model Context Protocol) integration, demonstrating how to expose basic SCIM operations to AI agents with minimal setup. It's perfect for understanding MCP concepts and testing AI agent interactions.

## What This Example Demonstrates

- **Minimal MCP Setup** - Get AI agents working with SCIM in under 50 lines of code
- **Basic Tool Exposure** - Essential user management operations as AI tools
- **Simple Protocol Integration** - Standard I/O based MCP communication
- **Quick Testing Patterns** - Immediate feedback and validation for AI interactions
- **Foundation Building** - Starting point for more sophisticated AI integrations

## Key Features Showcased

### Streamlined Integration
See how [`ScimMcpServer`](https://docs.rs/scim-server/latest/scim_server/mcp_integration/struct.ScimMcpServer.html) can be set up with minimal configuration, focusing on core functionality rather than advanced features.

### Essential Tool Set
The example exposes a focused set of AI tools covering the most common identity management operations:
- User creation and retrieval
- Basic user queries
- Schema discovery for AI understanding

### Zero-Configuration AI Support
Watch how AI agents can immediately start working with your SCIM server without complex setup, authentication, or protocol negotiation.

### Interactive Testing
The demo is designed for immediate interaction, allowing you to test AI agent communication patterns and understand the request/response flow.

## Concepts Explored

This example introduces fundamental MCP concepts:

- **[MCP Integration](../concepts/mcp-integration.md)** - AI agent support architecture basics
- **[SCIM Server](../concepts/scim-server.md)** - Core server functionality
- **[Basic Usage Patterns](./basic-usage.md)** - Underlying SCIM operations

## Perfect For Getting Started

This example is ideal if you're:

- **New to MCP** - Understanding AI agent integration concepts
- **Rapid Prototyping** - Quick setup for testing AI workflows
- **Proof of Concept** - Demonstrating AI-driven identity management
- **Learning Integration** - Understanding how SCIM and AI agents work together

## Tool Capabilities

The simple demo exposes core identity management tools:

### User Management
- **Create User** - Provision new accounts with basic validation
- **Get User** - Retrieve user information by username or ID
- **List Users** - Browse available user accounts

### System Discovery
- **Server Info** - Basic server capabilities and configuration
- **Schema Info** - Available resource types for AI understanding

## AI Interaction Flow

The example demonstrates a typical AI agent workflow:

1. **Tool Discovery** - AI agent requests available tools
2. **Schema Understanding** - Agent learns about user attributes and validation
3. **Operation Execution** - Agent performs identity management tasks
4. **Result Processing** - Agent receives structured responses for decision making

## Running the Example

```bash
cargo run --example simple_mcp_demo --features mcp
```

The server starts in interactive mode, ready to receive MCP protocol messages and demonstrate AI agent communication patterns.

## Testing with AI Agents

Once running, you can test with various AI systems:

### Manual Protocol Testing
Send JSON-RPC messages directly to understand the protocol:
```bash
echo '{"jsonrpc":"2.0","method":"tools/list","id":1}' | cargo run --example simple_mcp_demo --features mcp
```

### AI Agent Integration
Connect with MCP-compatible AI agents to see natural language identity management in action.

## Key Differences from Full MCP Server

This simplified demo differs from the [full MCP server example](./mcp-server.md):

- **Reduced Tool Set** - Focus on essential operations only
- **Minimal Configuration** - Default settings for quick startup
- **No Multi-Tenancy** - Single-tenant operation for simplicity
- **Basic Error Handling** - Simple error responses without complex recovery

## Extending the Demo

Natural extensions to explore:

- **Additional Tools** - Add group management or advanced user operations
- **Authentication** - Integrate with your authentication system
- **Multi-Tenancy** - Add tenant context for enterprise scenarios
- **Custom Schemas** - Extend with organization-specific attributes

## Running the Example

The demo starts immediately and provides clear output showing:
- Available tools and their schemas
- Example AI agent interactions
- Request/response patterns
- Error handling demonstrations

## Next Steps

After exploring the simple demo:

- **[MCP Server](./mcp-server.md)** - Full-featured MCP integration
- **[MCP with ETag Support](./mcp-etag.md)** - Add version control for AI operations
- **[Basic Usage](./basic-usage.md)** - Understanding underlying SCIM operations

## Source Code

View the complete implementation: [`examples/simple_mcp_demo.rs`](https://github.com/pukeko37/scim-server/blob/main/examples/simple_mcp_demo.rs)

## Related Documentation

- **[Setting Up Your MCP Server](../getting-started/mcp-server.md)** - Step-by-step MCP setup guide
- **[MCP Integration Concepts](../concepts/mcp-integration.md)** - Architectural overview
- **[MCP API Reference](https://docs.rs/scim-server/latest/scim_server/mcp_integration/index.html)** - Complete MCP documentation