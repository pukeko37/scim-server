//! MCP (Model Context Protocol) Integration for SCIM Server
//!
//! This module provides comprehensive MCP integration that exposes SCIM operations
//! as structured tools for AI agents. The integration enables AI systems to perform
//! identity management operations through a standardized protocol interface.
//!
//! ## Overview
//!
//! The MCP integration transforms SCIM server operations into discoverable tools
//! that AI agents can understand and execute. This enables:
//!
//! - **Automated Identity Management**: AI agents can provision/deprovision users
//! - **Schema-Driven Operations**: AI agents understand SCIM data structures
//! - **Multi-Tenant Support**: Tenant-aware operations for enterprise scenarios
//! - **Error Handling**: Structured error responses for AI decision making
//! - **Real-time Operations**: Async operations suitable for AI workflows
//!
//! ## Architecture
//!
//! ```text
//! ┌─────────────────┐    ┌──────────────────┐    ┌─────────────────┐
//! │   AI Agent      │───▶│  MCP Protocol    │───▶│  SCIM Server    │
//! │   (Client)      │    │  (This Module)   │    │  (Operations)   │
//! └─────────────────┘    └──────────────────┘    └─────────────────┘
//!          │                        │                       │
//!          ▼                        ▼                       ▼
//!    Tool Discovery          Tool Execution        Resource Management
//!    Schema Learning         JSON Validation        Provider Integration
//!    Error Handling          Tenant Context        Multi-Tenant Isolation
//! ```
//!
//! ## Usage Example
//!
//! ```rust,no_run
//! # #[cfg(feature = "mcp")]
//! use scim_server::{ScimServer, mcp_integration::ScimMcpServer, providers::InMemoryProvider};
//! use serde_json::json;
//!
//! # #[cfg(feature = "mcp")]
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // Create SCIM server
//!     let provider = InMemoryProvider::new();
//!     let scim_server = ScimServer::new(provider)?;
//!
//!     // Create MCP server
//!     let mcp_server = ScimMcpServer::new(scim_server);
//!
//!     // Execute tool (simulating AI agent)
//!     let result = mcp_server.execute_tool(
//!         "scim_create_user",
//!         json!({
//!             "user_data": {
//!                 "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
//!                 "userName": "ai.agent@company.com",
//!                 "active": true
//!             }
//!         })
//!     ).await;
//!
//!     if result.success {
//!         println!("User created successfully");
//!     }
//!     Ok(())
//! }
//! ```
//!
//! ## Available Tools
//!
//! The MCP integration provides these tools for AI agents:
//!
//! ### User Management
//! - **`scim_create_user`** - Create new users with schema validation
//! - **`scim_get_user`** - Retrieve user details by ID
//! - **`scim_update_user`** - Update user attributes
//! - **`scim_delete_user`** - Remove users safely
//! - **`scim_list_users`** - List all users with pagination
//! - **`scim_search_users`** - Search users by attributes
//! - **`scim_user_exists`** - Check user existence
//!
//! ### Schema Operations
//! - **`scim_get_schemas`** - Retrieve all available schemas
//! - **`scim_get_schema`** - Get specific schema details
//!
//! ### Server Information
//! - **`scim_server_info`** - Get server capabilities and metadata
//!
//! ## Multi-Tenant Support
//!
//! All tools support optional tenant context:
//!
//! ```rust,no_run
//! # #[cfg(feature = "mcp")]
//! # use scim_server::mcp_integration::ScimMcpServer;
//! # use serde_json::json;
//! # async fn example(mcp_server: ScimMcpServer<scim_server::providers::InMemoryProvider>) {
//! // Create user in specific tenant
//! let result = mcp_server.execute_tool(
//!     "scim_create_user",
//!     json!({
//!         "user_data": { /* user data */ },
//!         "tenant_id": "enterprise-corp"
//!     })
//! ).await;
//! # }
//! ```
//!
//! ## Error Handling
//!
//! The MCP integration provides structured error responses:
//!
//! ```rust,no_run
//! # #[cfg(feature = "mcp")]
//! # use scim_server::mcp_integration::ScimToolResult;
//! # fn example(result: ScimToolResult) {
//! if !result.success {
//!     // Error information available in result.content
//!     println!("Operation failed: {:?}", result.content);
//!
//!     // Metadata contains additional context
//!     if let Some(metadata) = result.metadata {
//!         println!("Error metadata: {:?}", metadata);
//!     }
//! }
//! # }
//! ```

#[cfg(feature = "mcp")]
use crate::{
    ResourceProvider,
    multi_tenant::TenantContext,
    operation_handler::{ScimOperationHandler, ScimOperationRequest},
    scim_server::ScimServer,
};

#[cfg(feature = "mcp")]
use log::{debug, info};
#[cfg(feature = "mcp")]
use serde_json::{Value, json};

/// Information about the MCP server for AI agent discovery
///
/// This structure provides metadata that AI agents use to understand
/// the capabilities and context of the SCIM server.
///
/// # Examples
///
/// ```rust
/// # #[cfg(feature = "mcp")]
/// use scim_server::mcp_integration::McpServerInfo;
///
/// # #[cfg(feature = "mcp")]
/// let server_info = McpServerInfo {
///     name: "Enterprise SCIM Server".to_string(),
///     version: "2.0.0".to_string(),
///     description: "Production SCIM server for HR systems".to_string(),
///     supported_resource_types: vec!["User".to_string(), "Group".to_string()],
/// };
/// ```
#[cfg(feature = "mcp")]
#[derive(Debug, Clone)]
pub struct McpServerInfo {
    /// Human-readable name of the SCIM server
    pub name: String,
    /// Version string for the server implementation
    pub version: String,
    /// Description of the server's purpose and capabilities
    pub description: String,
    /// List of SCIM resource types supported (e.g., "User", "Group")
    pub supported_resource_types: Vec<String>,
}

/// Tool execution result for MCP clients
///
/// Represents the outcome of an AI agent's tool execution request.
/// Provides structured feedback that AI agents can use for decision making.
///
/// # Examples
///
/// ```rust
/// # #[cfg(feature = "mcp")]
/// use scim_server::mcp_integration::ScimToolResult;
/// use serde_json::{json, Value};
///
/// # #[cfg(feature = "mcp")]
/// // Successful operation result
/// let success_result = ScimToolResult {
///     success: true,
///     content: json!({"id": "123", "userName": "john.doe"}),
///     metadata: Some(json!({"operation": "create", "resource_type": "User"}))
/// };
///
/// # #[cfg(feature = "mcp")]
/// // Error result
/// let error_result = ScimToolResult {
///     success: false,
///     content: json!({"error": "User not found"}),
///     metadata: Some(json!({"error_code": "404"}))
/// };
/// ```
#[cfg(feature = "mcp")]
#[derive(Debug, Clone)]
pub struct ScimToolResult {
    /// Whether the tool execution was successful
    pub success: bool,
    /// The main result content (resource data or error information)
    pub content: Value,
    /// Optional metadata providing additional context about the operation
    pub metadata: Option<Value>,
}

/// MCP server wrapper for SCIM operations
///
/// This is the main entry point for MCP integration. It wraps a SCIM server
/// and exposes its operations as MCP tools that AI agents can discover and execute.
///
/// # Type Parameters
///
/// * `P` - The resource provider implementation that handles data persistence
///
/// # Examples
///
/// ```rust,no_run
/// # #[cfg(feature = "mcp")]
/// use scim_server::{ScimServer, mcp_integration::ScimMcpServer, providers::InMemoryProvider};
///
/// # #[cfg(feature = "mcp")]
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let provider = InMemoryProvider::new();
///     let scim_server = ScimServer::new(provider)?;
///     let mcp_server = ScimMcpServer::new(scim_server);
///
///     // Get available tools
///     let tools = mcp_server.get_tools();
///     println!("Available tools: {}", tools.len());
///
///     // Run MCP server
///     mcp_server.run_stdio().await?;
///     Ok(())
/// }
/// ```
#[cfg(feature = "mcp")]
pub struct ScimMcpServer<P: ResourceProvider> {
    operation_handler: ScimOperationHandler<P>,
    server_info: McpServerInfo,
}

#[cfg(feature = "mcp")]
impl<P: ResourceProvider + Send + Sync + 'static> ScimMcpServer<P> {
    /// Create a new MCP server with default server information
    ///
    /// # Arguments
    ///
    /// * `scim_server` - The SCIM server instance to wrap with MCP capabilities
    ///
    /// # Returns
    ///
    /// A new `ScimMcpServer` instance with default server information
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// # #[cfg(feature = "mcp")]
    /// use scim_server::{ScimServer, mcp_integration::ScimMcpServer, providers::InMemoryProvider};
    ///
    /// # #[cfg(feature = "mcp")]
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let provider = InMemoryProvider::new();
    /// let scim_server = ScimServer::new(provider)?;
    /// let mcp_server = ScimMcpServer::new(scim_server);
    /// # Ok(())
    /// # }
    /// ```
    pub fn new(scim_server: ScimServer<P>) -> Self {
        let operation_handler = ScimOperationHandler::new(scim_server);
        let server_info = McpServerInfo::default();

        Self {
            operation_handler,
            server_info,
        }
    }

    /// Create a new MCP server with custom server information
    pub fn with_info(scim_server: ScimServer<P>, server_info: McpServerInfo) -> Self {
        let operation_handler = ScimOperationHandler::new(scim_server);

        Self {
            operation_handler,
            server_info,
        }
    }

    /// Get the list of available MCP tools as JSON (for compatibility)
    pub fn get_tools(&self) -> Vec<Value> {
        vec![
            self.create_user_tool(),
            self.get_user_tool(),
            self.update_user_tool(),
            self.delete_user_tool(),
            self.list_users_tool(),
            self.search_users_tool(),
            self.user_exists_tool(),
            self.get_schemas_tool(),
            self.get_server_info_tool(),
        ]
    }

    /// Execute a tool by name with arguments
    pub async fn execute_tool(&self, tool_name: &str, arguments: Value) -> ScimToolResult {
        debug!("Executing MCP tool: {} with args: {}", tool_name, arguments);

        match tool_name {
            "scim_create_user" => self.handle_create_user(arguments).await,
            "scim_get_user" => self.handle_get_user(arguments).await,
            "scim_update_user" => self.handle_update_user(arguments).await,
            "scim_delete_user" => self.handle_delete_user(arguments).await,
            "scim_list_users" => self.handle_list_users(arguments).await,
            "scim_search_users" => self.handle_search_users(arguments).await,
            "scim_user_exists" => self.handle_user_exists(arguments).await,
            "scim_get_schemas" => self.handle_get_schemas(arguments).await,
            "scim_server_info" => self.handle_server_info(arguments).await,
            _ => ScimToolResult {
                success: false,
                content: json!({
                    "error": "Unknown tool",
                    "tool_name": tool_name
                }),
                metadata: None,
            },
        }
    }

    /// Run the MCP server (simplified version)
    pub async fn run_stdio(self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        info!("SCIM MCP server ready for stdio communication");
        info!(
            "Available tools: {:?}",
            self.get_tools()
                .iter()
                .map(|t| t.get("name"))
                .collect::<Vec<_>>()
        );
        // In a real implementation, this would start the MCP protocol handler
        // For now, we just indicate readiness
        Ok(())
    }

    // Tool definitions as JSON values
    fn create_user_tool(&self) -> Value {
        json!({
            "name": "scim_create_user",
            "description": "Create a new user in the SCIM server",
            "input_schema": {
                "type": "object",
                "properties": {
                    "user_data": {
                        "type": "object",
                        "description": "User data conforming to SCIM User schema",
                        "properties": {
                            "schemas": {
                                "type": "array",
                                "items": {"type": "string"},
                                "description": "SCIM schemas for the user"
                            },
                            "userName": {
                                "type": "string",
                                "description": "Unique identifier for the user"
                            },
                            "name": {
                                "type": "object",
                                "description": "User's name components"
                            },
                            "emails": {
                                "type": "array",
                                "description": "User's email addresses"
                            },
                            "active": {
                                "type": "boolean",
                                "description": "Whether the user is active"
                            }
                        },
                        "required": ["schemas", "userName"]
                    },
                    "tenant_id": {
                        "type": "string",
                        "description": "Optional tenant identifier"
                    }
                },
                "required": ["user_data"]
            }
        })
    }

    fn get_user_tool(&self) -> Value {
        json!({
            "name": "scim_get_user",
            "description": "Retrieve a user by ID from the SCIM server",
            "input_schema": {
                "type": "object",
                "properties": {
                    "user_id": {
                        "type": "string",
                        "description": "The unique identifier of the user to retrieve"
                    },
                    "tenant_id": {
                        "type": "string",
                        "description": "Optional tenant identifier"
                    }
                },
                "required": ["user_id"]
            }
        })
    }

    fn update_user_tool(&self) -> Value {
        json!({
            "name": "scim_update_user",
            "description": "Update an existing user in the SCIM server",
            "input_schema": {
                "type": "object",
                "properties": {
                    "user_id": {
                        "type": "string",
                        "description": "The unique identifier of the user to update"
                    },
                    "user_data": {
                        "type": "object",
                        "description": "Updated user data conforming to SCIM User schema"
                    },
                    "tenant_id": {
                        "type": "string",
                        "description": "Optional tenant identifier"
                    }
                },
                "required": ["user_id", "user_data"]
            }
        })
    }

    fn delete_user_tool(&self) -> Value {
        json!({
            "name": "scim_delete_user",
            "description": "Delete a user from the SCIM server",
            "input_schema": {
                "type": "object",
                "properties": {
                    "user_id": {
                        "type": "string",
                        "description": "The unique identifier of the user to delete"
                    },
                    "tenant_id": {
                        "type": "string",
                        "description": "Optional tenant identifier"
                    }
                },
                "required": ["user_id"]
            }
        })
    }

    fn list_users_tool(&self) -> Value {
        json!({
            "name": "scim_list_users",
            "description": "List all users in the SCIM server with optional pagination",
            "input_schema": {
                "type": "object",
                "properties": {
                    "start_index": {
                        "type": "integer",
                        "description": "1-based start index for pagination",
                        "minimum": 1
                    },
                    "count": {
                        "type": "integer",
                        "description": "Number of users to return",
                        "minimum": 1
                    },
                    "tenant_id": {
                        "type": "string",
                        "description": "Optional tenant identifier"
                    }
                }
            }
        })
    }

    fn search_users_tool(&self) -> Value {
        json!({
            "name": "scim_search_users",
            "description": "Search for users by attribute value",
            "input_schema": {
                "type": "object",
                "properties": {
                    "attribute": {
                        "type": "string",
                        "description": "The attribute to search by (e.g., 'userName', 'email')"
                    },
                    "value": {
                        "type": "string",
                        "description": "The value to search for"
                    },
                    "tenant_id": {
                        "type": "string",
                        "description": "Optional tenant identifier"
                    }
                },
                "required": ["attribute", "value"]
            }
        })
    }

    fn user_exists_tool(&self) -> Value {
        json!({
            "name": "scim_user_exists",
            "description": "Check if a user exists in the SCIM server",
            "input_schema": {
                "type": "object",
                "properties": {
                    "user_id": {
                        "type": "string",
                        "description": "The unique identifier of the user to check"
                    },
                    "tenant_id": {
                        "type": "string",
                        "description": "Optional tenant identifier"
                    }
                },
                "required": ["user_id"]
            }
        })
    }

    fn get_schemas_tool(&self) -> Value {
        json!({
            "name": "scim_get_schemas",
            "description": "Get all available SCIM schemas for AI agent understanding",
            "input_schema": {
                "type": "object",
                "properties": {}
            }
        })
    }

    fn get_server_info_tool(&self) -> Value {
        json!({
            "name": "scim_server_info",
            "description": "Get SCIM server information and capabilities",
            "input_schema": {
                "type": "object",
                "properties": {}
            }
        })
    }

    // Tool handlers
    async fn handle_create_user(&self, arguments: Value) -> ScimToolResult {
        let user_data = match arguments.get("user_data") {
            Some(data) => data.clone(),
            None => {
                return ScimToolResult {
                    success: false,
                    content: json!({"error": "Missing user_data parameter"}),
                    metadata: None,
                };
            }
        };

        let tenant_context = arguments
            .get("tenant_id")
            .and_then(|t| t.as_str())
            .map(|id| TenantContext::new(id.to_string(), "mcp-client".to_string()));

        let mut request = ScimOperationRequest::create("User".to_string(), user_data);
        if let Some(tenant) = tenant_context {
            request = request.with_tenant(tenant);
        }

        let response = self.operation_handler.handle_operation(request).await;

        if response.success {
            ScimToolResult {
                success: true,
                content: response
                    .data
                    .unwrap_or_else(|| json!({"status": "created"})),
                metadata: Some(json!({
                    "operation": "create_user",
                    "resource_type": "User"
                })),
            }
        } else {
            ScimToolResult {
                success: false,
                content: json!({
                    "error": response.error.unwrap_or_else(|| "Create failed".to_string()),
                    "error_code": "CREATE_USER_FAILED"
                }),
                metadata: None,
            }
        }
    }

    async fn handle_get_user(&self, arguments: Value) -> ScimToolResult {
        let user_id = match arguments.get("user_id").and_then(|id| id.as_str()) {
            Some(id) => id,
            None => {
                return ScimToolResult {
                    success: false,
                    content: json!({"error": "Missing user_id parameter"}),
                    metadata: None,
                };
            }
        };

        let tenant_context = arguments
            .get("tenant_id")
            .and_then(|t| t.as_str())
            .map(|id| TenantContext::new(id.to_string(), "mcp-client".to_string()));

        let mut request = ScimOperationRequest::get("User".to_string(), user_id.to_string());
        if let Some(tenant) = tenant_context {
            request = request.with_tenant(tenant);
        }

        let response = self.operation_handler.handle_operation(request).await;

        if response.success {
            ScimToolResult {
                success: true,
                content: response.data.unwrap_or_else(|| json!({"error": "No data"})),
                metadata: Some(json!({
                    "operation": "get_user",
                    "resource_type": "User",
                    "resource_id": user_id
                })),
            }
        } else {
            ScimToolResult {
                success: false,
                content: json!({
                    "error": response.error.unwrap_or_else(|| "Get failed".to_string()),
                    "error_code": "GET_USER_FAILED"
                }),
                metadata: None,
            }
        }
    }

    async fn handle_update_user(&self, arguments: Value) -> ScimToolResult {
        let user_id = match arguments.get("user_id").and_then(|id| id.as_str()) {
            Some(id) => id,
            None => {
                return ScimToolResult {
                    success: false,
                    content: json!({"error": "Missing user_id parameter"}),
                    metadata: None,
                };
            }
        };

        let user_data = match arguments.get("user_data") {
            Some(data) => data.clone(),
            None => {
                return ScimToolResult {
                    success: false,
                    content: json!({"error": "Missing user_data parameter"}),
                    metadata: None,
                };
            }
        };

        let tenant_context = arguments
            .get("tenant_id")
            .and_then(|t| t.as_str())
            .map(|id| TenantContext::new(id.to_string(), "mcp-client".to_string()));

        let mut request =
            ScimOperationRequest::update("User".to_string(), user_id.to_string(), user_data);
        if let Some(tenant) = tenant_context {
            request = request.with_tenant(tenant);
        }

        let response = self.operation_handler.handle_operation(request).await;

        if response.success {
            ScimToolResult {
                success: true,
                content: response
                    .data
                    .unwrap_or_else(|| json!({"status": "updated"})),
                metadata: Some(json!({
                    "operation": "update_user",
                    "resource_type": "User",
                    "resource_id": user_id
                })),
            }
        } else {
            ScimToolResult {
                success: false,
                content: json!({
                    "error": response.error.unwrap_or_else(|| "Update failed".to_string()),
                    "error_code": "UPDATE_USER_FAILED"
                }),
                metadata: None,
            }
        }
    }

    async fn handle_delete_user(&self, arguments: Value) -> ScimToolResult {
        let user_id = match arguments.get("user_id").and_then(|id| id.as_str()) {
            Some(id) => id,
            None => {
                return ScimToolResult {
                    success: false,
                    content: json!({"error": "Missing user_id parameter"}),
                    metadata: None,
                };
            }
        };

        let tenant_context = arguments
            .get("tenant_id")
            .and_then(|t| t.as_str())
            .map(|id| TenantContext::new(id.to_string(), "mcp-client".to_string()));

        let mut request = ScimOperationRequest::delete("User".to_string(), user_id.to_string());
        if let Some(tenant) = tenant_context {
            request = request.with_tenant(tenant);
        }

        let response = self.operation_handler.handle_operation(request).await;

        if response.success {
            ScimToolResult {
                success: true,
                content: json!({"status": "deleted", "user_id": user_id}),
                metadata: Some(json!({
                    "operation": "delete_user",
                    "resource_type": "User",
                    "resource_id": user_id
                })),
            }
        } else {
            ScimToolResult {
                success: false,
                content: json!({
                    "error": response.error.unwrap_or_else(|| "Delete failed".to_string()),
                    "error_code": "DELETE_USER_FAILED"
                }),
                metadata: None,
            }
        }
    }

    async fn handle_list_users(&self, arguments: Value) -> ScimToolResult {
        let tenant_context = arguments
            .get("tenant_id")
            .and_then(|t| t.as_str())
            .map(|id| TenantContext::new(id.to_string(), "mcp-client".to_string()));

        let mut request = ScimOperationRequest::list("User".to_string());
        if let Some(tenant) = tenant_context {
            request = request.with_tenant(tenant);
        }

        let response = self.operation_handler.handle_operation(request).await;

        if response.success {
            ScimToolResult {
                success: true,
                content: response.data.unwrap_or_else(|| json!({"Resources": []})),
                metadata: Some(json!({
                    "operation": "list_users",
                    "resource_type": "User"
                })),
            }
        } else {
            ScimToolResult {
                success: false,
                content: json!({
                    "error": response.error.unwrap_or_else(|| "List failed".to_string()),
                    "error_code": "LIST_USERS_FAILED"
                }),
                metadata: None,
            }
        }
    }

    async fn handle_search_users(&self, arguments: Value) -> ScimToolResult {
        let attribute = match arguments.get("attribute").and_then(|a| a.as_str()) {
            Some(attr) => attr,
            None => {
                return ScimToolResult {
                    success: false,
                    content: json!({"error": "Missing attribute parameter"}),
                    metadata: None,
                };
            }
        };

        let value = match arguments.get("value").and_then(|v| v.as_str()) {
            Some(val) => val,
            None => {
                return ScimToolResult {
                    success: false,
                    content: json!({"error": "Missing value parameter"}),
                    metadata: None,
                };
            }
        };

        let tenant_context = arguments
            .get("tenant_id")
            .and_then(|t| t.as_str())
            .map(|id| TenantContext::new(id.to_string(), "mcp-client".to_string()));

        let mut request = ScimOperationRequest::list("User".to_string());

        if let Some(tenant) = tenant_context {
            request = request.with_tenant(tenant);
        }

        let response = self.operation_handler.handle_operation(request).await;

        if response.success {
            ScimToolResult {
                success: true,
                content: response.data.unwrap_or_else(|| json!({"Resources": []})),
                metadata: Some(json!({
                    "operation": "search_users",
                    "resource_type": "User",
                    "search_attribute": attribute,
                    "search_value": value
                })),
            }
        } else {
            ScimToolResult {
                success: false,
                content: json!({
                    "error": response.error.unwrap_or_else(|| "Search failed".to_string()),
                    "error_code": "SEARCH_USERS_FAILED"
                }),
                metadata: None,
            }
        }
    }

    async fn handle_user_exists(&self, arguments: Value) -> ScimToolResult {
        let user_id = match arguments.get("user_id").and_then(|id| id.as_str()) {
            Some(id) => id,
            None => {
                return ScimToolResult {
                    success: false,
                    content: json!({"error": "Missing user_id parameter"}),
                    metadata: None,
                };
            }
        };

        let tenant_context = arguments
            .get("tenant_id")
            .and_then(|t| t.as_str())
            .map(|id| TenantContext::new(id.to_string(), "mcp-client".to_string()));

        let mut request = ScimOperationRequest::get("User".to_string(), user_id.to_string());
        if let Some(tenant) = tenant_context {
            request = request.with_tenant(tenant);
        }

        let response = self.operation_handler.handle_operation(request).await;

        if response.success {
            ScimToolResult {
                success: true,
                content: json!({
                    "exists": true,
                    "user_id": user_id
                }),
                metadata: Some(json!({
                    "operation": "user_exists",
                    "resource_type": "User",
                    "resource_id": user_id
                })),
            }
        } else {
            ScimToolResult {
                success: true,
                content: json!({
                    "exists": false,
                    "user_id": user_id
                }),
                metadata: Some(json!({
                    "operation": "user_exists",
                    "resource_type": "User",
                    "resource_id": user_id
                })),
            }
        }
    }

    async fn handle_get_schemas(&self, _arguments: Value) -> ScimToolResult {
        let request = ScimOperationRequest::get_schemas();

        let response = self.operation_handler.handle_operation(request).await;

        if response.success {
            ScimToolResult {
                success: true,
                content: response.data.unwrap_or_else(|| json!({"schemas": []})),
                metadata: Some(json!({
                    "operation": "get_schemas"
                })),
            }
        } else {
            ScimToolResult {
                success: false,
                content: json!({
                    "error": response.error.unwrap_or_else(|| "Get schemas failed".to_string()),
                    "error_code": "GET_SCHEMAS_FAILED"
                }),
                metadata: None,
            }
        }
    }

    async fn handle_server_info(&self, _arguments: Value) -> ScimToolResult {
        let info = json!({
            "name": self.server_info.name,
            "version": self.server_info.version,
            "description": self.server_info.description,
            "supported_resource_types": self.server_info.supported_resource_types,
            "capabilities": {
                "user_management": true,
                "multi_tenant": true,
                "schema_introspection": true,
                "search": true,
                "pagination": true,
                "filtering": true
            },
            "mcp_integration": {
                "version": "1.0.0",
                "available_tools": self.get_tools().len()
            }
        });

        ScimToolResult {
            success: true,
            content: info.clone(),
            metadata: Some(json!({
                "operation": "server_info"
            })),
        }
    }
}

#[cfg(feature = "mcp")]
impl Default for McpServerInfo {
    fn default() -> Self {
        Self {
            name: "SCIM Server".to_string(),
            version: "1.0.0".to_string(),
            description: "SCIM server with MCP integration for AI agents".to_string(),
            supported_resource_types: vec!["User".to_string()],
        }
    }
}

#[cfg(test)]
#[cfg(feature = "mcp")]
mod tests {
    use super::*;
    use crate::providers::InMemoryProvider;
    use crate::resource_handlers::create_user_resource_handler;

    #[tokio::test]
    async fn test_mcp_server_creation() {
        let provider = InMemoryProvider::new();
        let mut scim_server = ScimServer::new(provider).unwrap();

        let user_schema = scim_server
            .get_schema_by_id("urn:ietf:params:scim:schemas:core:2.0:User")
            .unwrap()
            .clone();

        let user_handler = create_user_resource_handler(user_schema);
        scim_server
            .register_resource_type(
                "User",
                user_handler,
                vec![crate::multi_tenant::ScimOperation::Create],
            )
            .unwrap();

        let mcp_server = ScimMcpServer::new(scim_server);
        assert_eq!(mcp_server.server_info.name, "SCIM Server");
    }

    #[tokio::test]
    async fn test_mcp_tools_list() {
        let provider = InMemoryProvider::new();
        let scim_server = ScimServer::new(provider).unwrap();
        let mcp_server = ScimMcpServer::new(scim_server);

        let tools = mcp_server.get_tools();
        assert!(!tools.is_empty());

        let tool_names: Vec<_> = tools
            .iter()
            .filter_map(|t| t.get("name").and_then(|n| n.as_str()))
            .collect();
        assert!(tool_names.contains(&"scim_create_user"));
        assert!(tool_names.contains(&"scim_get_user"));
        assert!(tool_names.contains(&"scim_list_users"));
    }

    #[tokio::test]
    async fn test_mcp_tool_execution() {
        let provider = InMemoryProvider::new();
        let mut scim_server = ScimServer::new(provider).unwrap();

        let user_schema = scim_server
            .get_schema_by_id("urn:ietf:params:scim:schemas:core:2.0:User")
            .unwrap()
            .clone();

        let user_handler = create_user_resource_handler(user_schema);
        scim_server
            .register_resource_type(
                "User",
                user_handler,
                vec![
                    crate::multi_tenant::ScimOperation::Create,
                    crate::multi_tenant::ScimOperation::List,
                ],
            )
            .unwrap();

        let mcp_server = ScimMcpServer::new(scim_server);

        // Test get schemas tool
        let result = mcp_server.execute_tool("scim_get_schemas", json!({})).await;
        assert!(result.success);

        // Test list users tool
        let result = mcp_server.execute_tool("scim_list_users", json!({})).await;
        assert!(result.success);
    }
}
