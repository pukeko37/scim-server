# MCP Integration (AI Agent Support)

MCP (Model Context Protocol) Integration enables AI agents to perform identity management operations through a standardized, discoverable tool interface. This integration transforms SCIM operations into structured tools that AI systems can understand, discover, and execute, making identity management accessible to artificial intelligence workflows and automation systems.

> **Note**: MCP Integration is available behind the `mcp` feature flag. Add `features = ["mcp"]` to your `Cargo.toml` dependency to enable this functionality.

See the [MCP Integration API documentation](https://docs.rs/scim-server/latest/scim_server/mcp_integration/index.html) for complete details.

## Value Proposition

MCP Integration delivers comprehensive AI-first identity management capabilities:

- **AI-Native Interface**: Structured tool discovery and execution designed for AI agent workflows
- **Schema-Driven Operations**: AI agents understand SCIM data structures through JSON Schema definitions
- **Automatic Tool Discovery**: Dynamic exposure of available operations based on server configuration
- **Conversational Identity Management**: Natural language to structured SCIM operations translation
- **Multi-Tenant AI Support**: Tenant-aware operations for enterprise AI deployment scenarios
- **Version-Aware AI Operations**: Built-in optimistic locking prevents AI-induced data conflicts
- **Error-Resilient AI Workflows**: Structured error responses enable AI decision making and recovery

## Architecture Overview

MCP Integration operates as an AI-agent bridge on top of the [Operation Handler](./operation-handlers.md) layer:

```text
AI Agent (Claude, GPT, Custom)
    ↓
MCP Protocol (JSON-RPC 2.0)
    ↓
ScimMcpServer (AI Agent Bridge)
├── Tool Discovery & Schema Generation
├── Parameter Validation & Conversion
├── AI-Friendly Error Translation
├── Version Metadata Management
└── Tenant Context Extraction
    ↓
Operation Handler (Framework Abstraction)
    ↓
SCIM Server (Business Logic)
```

### Core Components

1. **[`ScimMcpServer`](https://docs.rs/scim-server/latest/scim_server/mcp_integration/struct.ScimMcpServer.html)**: Main MCP server wrapper exposing SCIM operations as AI tools
2. **[Tool Schemas](https://docs.rs/scim-server/latest/scim_server/mcp_integration/index.html#tool-schemas)**: JSON Schema definitions for AI agent tool discovery
3. **Tool Handlers**: Execution logic for each exposed SCIM operation
4. **Protocol Layer**: MCP JSON-RPC 2.0 protocol implementation
5. **[`ScimToolResult`](https://docs.rs/scim-server/latest/scim_server/mcp_integration/enum.ScimToolResult.html)**: Structured results optimized for AI decision making

## Use Cases

### 1. Conversational HR Assistant

**AI-powered employee onboarding and management**

```rust
// Setup MCP server for HR AI assistant
let hr_server_info = McpServerInfo {
    name: "HR Identity Management".to_string(),
    version: "1.0.0".to_string(),
    description: "AI-powered employee lifecycle management".to_string(),
    supported_resource_types: vec!["User".to_string(), "Group".to_string()],
};

let mcp_server = ScimMcpServer::with_info(scim_server, hr_server_info);

// AI agent discovers available tools
let tools = mcp_server.get_tools();
// Returns: create_user, get_user, update_user, delete_user, list_users, etc.

// AI agent executes conversational commands:
// "Create a new employee John Doe with email john.doe@company.com"
let result = mcp_server.execute_tool("scim_create_user", json!({
    "user_data": {
        "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
        "userName": "john.doe@company.com",
        "name": {
            "givenName": "John",
            "familyName": "Doe"
        },
        "emails": [{
            "value": "john.doe@company.com",
            "primary": true
        }],
        "active": true
    }
})).await;

// AI receives structured response with raw version for follow-up operations
if result.success {
    let user_id = result.metadata.as_ref()
        .and_then(|m| m.get("resource_id"))
        .and_then(|id| id.as_str());
    let version = result.metadata.as_ref()
        .and_then(|m| m.get("version"))
        .and_then(|v| v.as_str());
    // AI can now reference this user and version in subsequent operations
}
```

**Benefits**: Natural language HR operations, automatic compliance, conversation history tracking.

### 2. DevOps Automation Agent

**AI-driven development environment provisioning**

```rust
// Multi-tenant development environment management
let devops_context = json!({
    "tenant_id": "dev-environment-123"
});

// AI agent: "Set up development accounts for the new team"
let team_members = vec!["alice.dev", "bob.dev", "charlie.dev"];

for username in team_members {
    let create_result = mcp_server.execute_tool("scim_create_user", json!({
        "user_data": {
            "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
            "userName": username,
            "active": true,
            "emails": [{
                "value": format!("{}@dev.company.com", username),
                "primary": true
            }]
        },
        "tenant_id": "dev-environment-123"
    })).await;

    if !create_result.success {
        // AI can understand and act on structured errors
        println!("Failed to create {}: {}", username, 
                 create_result.content.get("error").unwrap());
    }
}

// AI agent: "Create development team group and add all developers"
let group_result = mcp_server.execute_tool("scim_create_group", json!({
    "group_data": {
        "schemas": ["urn:ietf:params:scim:schemas:core:2.0:Group"],
        "displayName": "Development Team",
        "members": team_user_ids  // Collected from previous operations
    },
    "tenant_id": "dev-environment-123"
})).await;
```

**Benefits**: Automated environment setup, consistent development team provisioning, AI-driven scaling.

### 3. Compliance and Audit AI

**AI agent for identity governance and compliance monitoring**

```rust
// AI agent performing compliance audit
let compliance_server = ScimMcpServer::with_info(scim_server, McpServerInfo {
    name: "Compliance Monitor".to_string(),
    description: "AI-powered identity compliance and audit system".to_string(),
    ..Default::default()
});

// AI agent: "Check all inactive users and prepare deprovisioning report"
let inactive_users_result = mcp_server.execute_tool("scim_search_users", json!({
    "attribute": "active",
    "value": "false"
})).await;

if let Some(users) = inactive_users_result.content.as_array() {
    for user in users {
        let user_id = user.get("id").and_then(|id| id.as_str()).unwrap();
        
        // AI agent: "Analyze user access patterns and recommend action"
        let user_details = mcp_server.execute_tool("scim_get_user", json!({
            "user_id": user_id
        })).await;
        
        // AI processes user data and determines compliance actions
        // Based on last login, role, department, etc.
    }
}

// AI agent generates compliance report and recommended actions
```

**Benefits**: Automated compliance monitoring, intelligent audit trails, AI-driven governance decisions.

### 4. Customer Support AI

**AI-powered customer identity troubleshooting**

```rust
// Support AI with customer context
let support_context = json!({
    "tenant_id": "customer-acme-corp"
});

// AI agent: "Help customer john.doe@acme.com who can't log in"
let user_search_result = mcp_server.execute_tool("scim_search_users", json!({
    "attribute": "userName", 
    "value": "john.doe@acme.com",
    "tenant_id": "customer-acme-corp"
})).await;

if let Some(user_data) = user_search_result.content.as_array() {
    if let Some(user) = user_data.first() {
        let is_active = user.get("active").and_then(|a| a.as_bool()).unwrap_or(false);
        
        if !is_active {
            // AI agent: "User account is inactive, reactivating..."
            let reactivate_result = mcp_server.execute_tool("scim_update_user", json!({
                "user_id": user.get("id").unwrap(),
                "user_data": {
                    "active": true
                },
                "tenant_id": "customer-acme-corp"
            })).await;
            
            if reactivate_result.success {
                // AI provides customer with resolution confirmation
            }
        }
    }
}
```

**Benefits**: Instant customer issue resolution, multi-tenant support context, automated troubleshooting.

### 5. Security Response AI

**AI agent for automated security incident response**

```rust
// Security AI with emergency response capabilities
let security_context = json!({
    "tenant_id": "production-environment"
});

// AI agent: "Detect compromised user accounts and take protective action"
let suspicious_users = vec!["compromised.user1", "compromised.user2"];

for username in suspicious_users {
    // AI agent: "Immediately disable compromised account"
    let disable_result = mcp_server.execute_tool("scim_update_user", json!({
        "user_id": username,
        "user_data": {
            "active": false
        },
        "tenant_id": "production-environment",
        "expected_version": current_version  // Raw version format for MCP
    })).await;

    if disable_result.success {
        // AI logs security action and updates incident response system
        let audit_metadata = disable_result.metadata.unwrap();
        // Security AI can track exactly what actions were taken when
    } else {
        // AI escalates if automated response fails
        alert_security_team(&disable_result.content);
    }
}
```

**Benefits**: Rapid incident response, automated security actions, comprehensive audit trails.

## Design Patterns

### Tool Discovery Pattern

AI agents discover available operations through structured schemas:

```rust
pub fn get_tools(&self) -> Vec<Value> {
    vec![
        user_schemas::create_user_tool(),
        user_schemas::get_user_tool(),
        user_schemas::update_user_tool(),
        // ... other tools
    ]
}
```

This enables dynamic capability discovery based on server configuration.

### Parameter Validation Pattern

JSON Schema validation ensures AI agents provide correct parameters:

```json
{
    "name": "scim_create_user",
    "inputSchema": {
        "type": "object",
        "properties": {
            "user_data": {
                "type": "object",
                "properties": {
                    "schemas": {"type": "array"},
                    "userName": {"type": "string"}
                },
                "required": ["schemas", "userName"]
            }
        },
        "required": ["user_data"]
    }
}
```

This provides AI agents with clear parameter requirements and validation rules.

### Structured Response Pattern

Consistent response format enables AI decision making:

```rust
pub struct ScimToolResult {
    pub success: bool,
    pub content: Value,           // Main data or error information
    pub metadata: Option<Value>,  // Operation context and version info
}
```

This allows AI agents to understand operation outcomes and plan subsequent actions.

### Version Propagation Pattern

AI agents receive raw version information for safe concurrent operations:

```rust
// Response includes raw version metadata (no HTTP ETag formatting)
let metadata = json!({
    "operation": "create_user",
    "resource_id": "123",
    "version": "abc123def"  // Raw version format for MCP
});
```

This enables AI agents to perform version-aware operations without conflicts. The MCP integration automatically converts HTTP ETags to raw version strings for consistent programmatic access.

## Integration with Other Components

### Operation Handler Integration

MCP Integration leverages Operation Handlers for structured processing:

- **Request Translation**: Converts MCP tool calls to ScimOperationRequest format
- **Response Formatting**: Transforms operation responses to AI-friendly format
- **Error Handling**: Provides structured error information for AI decision making
- **Version Management**: Automatically includes version metadata in responses

### SCIM Server Integration

Direct integration with SCIM Server core functionality:

- **Dynamic Tool Generation**: Available tools reflect registered resource types
- **Schema Discovery**: AI agents can introspect available schemas and capabilities
- **Multi-Tenant Support**: Automatic tenant context extraction and validation
- **Permission Enforcement**: Tenant permissions automatically applied to AI operations

### Multi-Tenant Integration

Seamless tenant awareness for enterprise AI deployment:

- **Tenant Context Extraction**: Automatically extracts tenant information from tool parameters
- **Scoped Operations**: All AI operations automatically tenant-scoped
- **Tenant Validation**: Ensures AI agents can only access authorized tenants
- **Cross-Tenant Prevention**: Prevents AI agents from accidentally crossing tenant boundaries

## Best Practices

### 1. Design AI-Friendly Tool Schemas

```rust
// Good: Clear, descriptive schemas with validation
pub fn create_user_tool() -> Value {
    json!({
        "name": "scim_create_user",
        "description": "Create a new user in the SCIM server",
        "inputSchema": {
            "type": "object",
            "properties": {
                "user_data": {
                    "description": "User data conforming to SCIM User schema",
                    "properties": {
                        "userName": {
                            "type": "string",
                            "description": "Unique identifier for the user"
                        }
                    }
                }
            }
        }
    })
}

// Avoid: Vague schemas without clear validation
// AI agents need precise parameter requirements
```

### 2. Provide Rich Error Context for AI Agents

```rust
// Good: Structured error responses AI agents can understand
ScimToolResult {
    success: false,
    content: json!({
        "error": "User already exists",
        "error_code": "duplicate_username",
        "suggested_action": "try_different_username"
    }),
    metadata: Some(json!({
        "operation": "create_user",
        "conflict_field": "userName"
    }))
}

// Avoid: Generic error messages
ScimToolResult {
    success: false,
    content: json!({"error": "Failed"}),  // Not actionable for AI
    metadata: None
}
```

### 3. Include Version Information for AI Concurrency

```rust
// Good: Always include raw version metadata
let mut metadata = json!({
    "operation": "update_user",
    "resource_id": user_id
});

// Convert ETag to raw version for MCP clients
if let Some(etag) = response.metadata.additional.get("etag") {
    if let Some(raw_version) = etag_to_raw_version(etag) {
        metadata["version"] = json!(raw_version);
    }
}

// This enables AI agents to perform safe concurrent operations
```

### 4. Use Tenant Context Consistently

```rust
// Good: Extract and validate tenant context from AI requests
let tenant_context = arguments
    .get("tenant_id")
    .and_then(|t| t.as_str())
    .map(|id| TenantContext::new(id.to_string(), "ai-agent".to_string()));

let request = ScimOperationRequest::create("User", user_data)
    .with_tenant_context(tenant_context);

// Avoid: Ignoring tenant context in AI operations
// This can lead to cross-tenant data access
```

### 5. Design for AI Conversation Flows

```rust
// Good: Operations return metadata for follow-up actions
ScimToolResult {
    success: true,
    content: user_json,
    metadata: Some(json!({
        "operation": "create_user",
        "resource_id": "user-123",
        "next_actions": ["add_to_group", "set_permissions", "send_welcome"]
    }))
}

// This helps AI agents plan multi-step workflows
```

## When to Use MCP Integration

### Primary Scenarios

1. **AI-Powered HR Systems**: Conversational employee lifecycle management
2. **DevOps Automation**: AI-driven environment and user provisioning
3. **Compliance Monitoring**: Automated identity governance and audit
4. **Customer Support**: AI-powered identity troubleshooting and resolution
5. **Security Response**: Automated incident response and threat mitigation

### Implementation Strategies

| Scenario | AI Agent Type | Complexity | Benefits |
|----------|---------------|------------|----------|
| HR Assistant | Conversational AI (Claude, GPT) | Low | Natural language HR operations |
| DevOps Automation | Workflow AI (custom agents) | Medium | Automated provisioning at scale |
| Compliance Monitor | Analytics AI (specialized) | Medium | Continuous governance monitoring |
| Security Response | Response AI (real-time) | High | Instant threat mitigation |
| Customer Support | Support AI (chat-based) | Low | 24/7 identity issue resolution |

## Comparison with Traditional Integration Approaches

| Approach | AI Accessibility | Discovery | Validation | Automation |
|----------|------------------|-----------|------------|------------|
| **MCP Integration** | ✅ Native | ✅ Automatic | ✅ JSON Schema | ✅ Conversational |
| REST API | ⚠️ Manual | ❌ Static | ⚠️ Manual | ⚠️ Scripted |
| GraphQL | ⚠️ Schema-based | ⚠️ Introspection | ⚠️ Type System | ⚠️ Query-based |
| Custom Protocol | ❌ Requires Training | ❌ Manual | ❌ Custom | ❌ Programmatic |

MCP Integration provides the optimal path for AI agents to perform identity management operations with native understanding, automatic discovery, and conversational interaction patterns.

## Version Handling in MCP vs HTTP

The MCP integration uses raw version strings instead of HTTP ETags for better programmatic access by AI agents:

### HTTP Integration
- Uses HTTP ETag format: `W/"abc123def"`
- Suitable for web browsers and HTTP clients
- Standard HTTP conditional request headers

### MCP Integration  
- Uses raw version format: `"abc123def"`
- Better for JSON-RPC and programmatic access
- AI agents work directly with version strings
- Automatic conversion from ETags to raw format

The MCP handlers automatically convert between formats using the `etag_to_raw_version()` utility function, ensuring AI agents always receive consistent raw version strings regardless of the internal representation.

## AI Agent Capabilities Enabled

### Schema Understanding
- AI agents can introspect available SCIM schemas
- Automatic validation ensures compliance with SCIM 2.0
- Dynamic tool discovery based on server configuration

### Conversational Operations
- Natural language requests translated to structured SCIM operations
- AI agents understand operation outcomes and can plan follow-up actions
- Multi-step workflows handled through conversation context

### Error Recovery
- Structured error responses enable AI decision making
- Suggested actions help AI agents resolve issues automatically
- Retry logic with raw version awareness prevents infinite loops

### Multi-Tenant Awareness
- AI agents understand tenant boundaries and permissions
- Automatic tenant context validation prevents cross-tenant access
- Enterprise deployment ready with proper isolation

MCP Integration transforms SCIM Server into an AI-native identity management platform, enabling artificial intelligence to perform sophisticated identity operations through natural, discoverable, and safe interaction patterns. This creates new possibilities for automated identity governance, conversational HR systems, and intelligent security response capabilities.

## Feature Flag Usage

To enable MCP Integration, add the feature flag to your `Cargo.toml`:

```toml
[dependencies]
scim-server = { version = "0.5.2", features = ["mcp"] }
```

The MCP integration includes:
- Tool discovery and execution
- Raw version handling optimized for AI agents
- Multi-tenant support for enterprise AI deployment
- Structured error responses for AI decision making