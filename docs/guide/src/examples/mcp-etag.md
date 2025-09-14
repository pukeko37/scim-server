# MCP with ETag Support

This example demonstrates how to combine MCP (Model Context Protocol) integration with ETag-based concurrency control, enabling AI agents to perform version-aware identity management operations. It shows how AI systems can handle concurrent access scenarios and prevent data conflicts through proper version management.

## What This Example Demonstrates

- **Version-Aware AI Operations** - AI agents that understand and work with resource versions
- **Conflict-Resilient AI Workflows** - Automatic handling of version conflicts in AI-driven updates
- **Optimistic Concurrency for AI** - Non-blocking AI operations with conflict detection
- **Intelligent Retry Logic** - AI agents that can recover from version conflicts gracefully
- **Production-Safe AI Integration** - Preventing AI-induced data corruption in concurrent environments
- **Multi-Client AI Scenarios** - Multiple AI agents working safely with shared resources

## Key Features Showcased

### AI-Aware Version Management
See how [`ScimMcpServer`](https://docs.rs/scim-server/latest/scim_server/mcp_integration/struct.ScimMcpServer.html) exposes version information to AI agents, enabling them to make informed decisions about when and how to update resources.

### Conditional AI Operations
Watch AI agents use version parameters in their tool calls, leveraging the same [`ConditionalOperations`](https://docs.rs/scim-server/latest/scim_server/providers/helpers/conditional/trait.ConditionalOperations.html) that power HTTP-based concurrency control.

### Structured Conflict Responses
Explore how version conflicts are communicated to AI agents through [`ScimToolResult`](https://docs.rs/scim-server/latest/scim_server/mcp_integration/enum.ScimToolResult.html), providing enough context for intelligent conflict resolution.

### AI Retry Patterns
The example demonstrates how AI agents can implement exponential backoff and intelligent retry logic when encountering version conflicts, preventing infinite retry loops.

## Concepts Explored

This example combines advanced AI and concurrency concepts:

- **[MCP Integration](../concepts/mcp-integration.md)** - AI agent support with advanced features
- **[Concurrency Control](../concepts/concurrency.md)** - Version-based conflict prevention
- **[Operation Handlers](../concepts/operation-handlers.md)** - Framework-agnostic version handling
- **[SCIM Server](../concepts/scim-server.md)** - Server-level concurrency orchestration

## Perfect For Building

This example is essential if you're:

- **Building Production AI Systems** - AI agents that work safely in concurrent environments
- **Implementing Automated Workflows** - AI-driven processes that must handle conflicts gracefully
- **Creating Resilient AI Agents** - Systems that can recover from operational conflicts
- **Enterprise AI Integration** - AI agents working with shared enterprise resources

## AI Agent Scenarios

The example covers sophisticated AI interaction patterns:

### Collaborative AI Agents
Multiple AI agents working on the same resources simultaneously, with automatic conflict detection and resolution when their operations overlap.

### Long-Running AI Workflows
AI agents that perform multi-step operations over time, using version checking to ensure their assumptions about resource state remain valid.

### AI-Human Collaboration
Scenarios where AI agents and human administrators work on the same resources, with version control preventing conflicts between automated and manual changes.

### Batch AI Operations
AI agents performing bulk updates with version validation, ensuring consistency across multiple related resource changes.

## Version-Aware Tool Operations

The MCP server exposes enhanced tools with version support:

### User Management with Versions
- **Get User with Version** - Retrieve users with current version information
- **Update User with Version Check** - Conditional updates that respect current versions
- **Create User with Conflict Detection** - Prevent duplicate creation with version validation

### Group Operations with Concurrency Control
- **Modify Group Membership** - Version-aware member addition and removal
- **Update Group Properties** - Conditional group updates with conflict detection
- **Bulk Group Operations** - Multiple group changes with consistent versioning

### Schema Operations
- **Version-Aware Schema Discovery** - Understanding current schema versions
- **Schema Extension Validation** - Ensuring extensions don't conflict with current state

## Running the Example

```bash
cargo run --example mcp_etag_example --features mcp
```

The server demonstrates version-aware AI operations with simulated concurrent access scenarios, showing how AI agents handle conflicts and maintain data consistency.

## AI Conflict Resolution Strategies

The example illustrates different approaches AI agents can take when encountering version conflicts:

### Immediate Retry
Simple retry logic for transient conflicts, with exponential backoff to prevent system overload.

### Conflict Analysis
AI agents that examine conflict details and make intelligent decisions about how to proceed based on the nature of the changes.

### User Consultation
AI workflows that escalate version conflicts to human decision-makers when automatic resolution isn't appropriate.

### Alternative Strategy Selection
AI agents that choose different approaches when their preferred operation conflicts with concurrent changes.

## Production Benefits

This example demonstrates critical production AI capabilities:

- **Data Integrity** - Preventing AI-induced data corruption through version validation
- **System Stability** - Avoiding cascade failures from AI retry storms
- **Operational Reliability** - Predictable AI behavior in concurrent scenarios
- **Audit Compliance** - Version tracking for all AI-initiated changes

## Multi-Tenant Version Management

See how version control works in multi-tenant AI scenarios:

- **Tenant-Scoped Versions** - Version management within tenant boundaries
- **Cross-Tenant Conflict Prevention** - Ensuring AI agents respect tenant isolation
- **Per-Tenant AI Policies** - Different conflict resolution strategies for different customers

## Integration with AI Frameworks

The example works with various AI agent systems:

- **Autonomous Agents** - Self-directing AI systems with conflict awareness
- **Workflow Orchestrators** - Multi-step AI processes with version checkpoints
- **Decision Support Systems** - AI assistants that help humans navigate conflicts
- **Automated Operations** - AI-driven identity lifecycle management

## Advanced Features

Explore sophisticated version-aware AI capabilities:

- **Predictive Conflict Avoidance** - AI agents that anticipate and avoid conflicts
- **Collaborative Decision Making** - Multiple AI agents negotiating resource changes
- **Version History Analysis** - AI systems that learn from past conflict patterns
- **Adaptive Retry Strategies** - AI agents that adjust behavior based on conflict frequency

## Next Steps

After exploring MCP with ETag support:

- **[ETag Concurrency Control](./etag-concurrency.md)** - Understanding the underlying concurrency mechanisms
- **[MCP Server](./mcp-server.md)** - Full-featured MCP integration without version focus
- **[Multi-Tenant Server](./multi-tenant.md)** - Tenant-aware version management

## Source Code

View the complete implementation: [`examples/mcp_etag_example.rs`](https://github.com/pukeko37/scim-server/blob/main/examples/mcp_etag_example.rs)

## Related Documentation

- **[Concurrency Control Concepts](../concepts/concurrency.md)** - Complete concurrency control overview
- **[MCP Integration Guide](../concepts/mcp-integration.md)** - AI agent architecture and patterns
- **[MCP API Reference](https://docs.rs/scim-server/latest/scim_server/mcp_integration/index.html)** - Complete MCP integration documentation