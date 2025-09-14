# MCP Server

This example demonstrates how to create and run a SCIM server that exposes its functionality as MCP (Model Context Protocol) tools for AI agents. The MCP integration transforms SCIM operations into a structured tool interface that AI systems can discover, understand, and execute.

## What This Example Demonstrates

- **AI-Native SCIM Interface** - Complete SCIM operations exposed as discoverable AI tools
- **Tool Schema Generation** - Automatic JSON Schema creation for AI agent understanding
- **Multi-Tenant AI Support** - Tenant-aware operations for enterprise AI deployment
- **Error-Resilient AI Workflows** - Structured error responses enabling AI decision making
- **Schema Introspection** - Dynamic discovery of SCIM capabilities and resource types
- **Version-Aware AI Operations** - Built-in concurrency control for AI-driven updates

## Key Features Showcased

### AI Tool Discovery
Watch how [`ScimMcpServer`](https://docs.rs/scim-server/latest/scim_server/mcp_integration/struct.ScimMcpServer.html) automatically exposes SCIM operations as structured tools that AI agents can discover and understand without manual configuration.

### Structured Tool Execution
See how complex SCIM operations are transformed into simple, parameterized tools that AI agents can execute with natural language input, complete with validation and error handling.

### Schema-Driven AI Understanding
The example demonstrates how AI agents can introspect SCIM schemas to understand resource structures, attribute types, and validation rules - enabling intelligent data manipulation.

### Enterprise AI Integration
Explore how [`McpServerInfo`](https://docs.rs/scim-server/latest/scim_server/mcp_integration/struct.McpServerInfo.html) provides comprehensive server capabilities to AI agents, enabling sophisticated identity management workflows.

## Concepts Explored

This example bridges AI and identity management through several key concepts:

- **[MCP Integration](../concepts/mcp-integration.md)** - Complete AI agent support architecture
- **[Operation Handlers](../concepts/operation-handlers.md)** - Framework-agnostic operation abstraction
- **[SCIM Server](../concepts/scim-server.md)** - Core protocol implementation
- **[Schema Discovery](../concepts/schemas.md)** - Dynamic capability advertisement

## Perfect For Building

This example is essential if you're:

- **Building AI-Powered Identity Systems** - Automated user provisioning and management
- **Creating Conversational HR Tools** - Natural language identity operations
- **Implementing Smart Workflows** - AI-driven identity lifecycle management
- **Enterprise AI Integration** - Connecting AI agents to identity infrastructure

## AI Agent Capabilities

The MCP server exposes comprehensive identity management tools:

### User Management Tools
- **Create User** - Provision new user accounts with validation
- **Get User** - Retrieve user information by ID or username
- **Update User** - Modify user attributes with conflict detection
- **Delete User** - Deactivate or remove user accounts
- **List Users** - Query and filter user populations
- **Search Users** - Find users by specific attributes

### Group Management Tools
- **Create Group** - Establish new groups with member management
- **Manage Members** - Add and remove group members
- **Group Queries** - Search and filter group collections

### Schema Discovery Tools
- **List Schemas** - Discover available resource types and attributes
- **Get Server Info** - Understand server capabilities and configuration
- **Introspect Resources** - Examine resource structure and validation rules

## AI Workflow Examples

The example demonstrates several AI agent interaction patterns:

### Conversational User Creation
AI agents can create users from natural language descriptions, automatically mapping human-readable requests to proper SCIM resource structures.

### Intelligent Error Recovery
When operations fail, the structured error responses help AI agents understand what went wrong and how to correct the issue.

### Multi-Step Workflows
Complex identity operations can be broken down into multiple tool calls, with the AI agent orchestrating the sequence based on business logic.

### Schema-Aware Operations
AI agents can inspect schemas before operations, ensuring they provide appropriate data types and required fields.

## Running the Example

```bash
cargo run --example mcp_server_example --features mcp
```

The server starts listening on standard input/output for MCP protocol messages, ready to receive tool discovery and execution requests from AI agents.

## Integration with AI Systems

This example works with various AI agent frameworks:

- **Claude Desktop** - Direct MCP protocol integration
- **Custom AI Agents** - JSON-RPC 2.0 protocol support
- **Workflow Automation** - Programmatic AI agent integration
- **Enterprise AI Platforms** - Structured tool interface compatibility

## Production Considerations

The example illustrates enterprise-ready AI integration patterns:

- **Security Boundaries** - Tenant isolation for AI operations
- **Audit Trails** - Comprehensive logging of AI-driven changes
- **Rate Limiting** - Controlled AI agent access patterns
- **Error Handling** - Graceful failure modes for AI workflows

## Multi-Tenant AI Operations

See how AI agents can work with tenant-scoped operations, enabling:

- **Customer-Specific AI** - Agents that operate within tenant boundaries
- **Isolated AI Workflows** - Preventing cross-tenant data access
- **Tenant-Aware Automation** - Context-sensitive AI operations

## Next Steps

After exploring MCP integration:

- **[MCP with ETag Support](./mcp-etag.md)** - Add version control to AI operations
- **[Simple MCP Demo](./simple-mcp-demo.md)** - Quick integration patterns
- **[MCP STDIO Server](./mcp-stdio-server.md)** - Standard I/O protocol implementation

## Source Code

View the complete implementation: [`examples/mcp_server_example.rs`](https://github.com/pukeko37/scim-server/blob/main/examples/mcp_server_example.rs)

## Related Documentation

- **[Setting Up Your MCP Server](../getting-started/mcp-server.md)** - Step-by-step MCP setup guide
- **[MCP Integration Concepts](../concepts/mcp-integration.md)** - Architectural overview and patterns
- **[MCP API Reference](https://docs.rs/scim-server/latest/scim_server/mcp_integration/index.html)** - Complete MCP integration documentation