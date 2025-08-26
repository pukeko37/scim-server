//! MCP protocol layer for tool discovery and dispatch
//!
//! This module handles the core MCP protocol functionality including tool discovery,
//! execution dispatch, and protocol communication. It serves as the interface between
//! AI agents and the SCIM server operations.

use super::core::{ScimMcpServer, ScimToolResult};
use super::handlers::{system_info, user_crud, user_queries};
use super::tools::{system_schemas, user_schemas};
use crate::ResourceProvider;
use log::{debug, error, info, warn};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

/// MCP JSON-RPC request structure
#[derive(Debug, Deserialize)]
struct McpRequest {
    jsonrpc: String,
    id: Option<Value>,
    method: String,
    params: Option<Value>,
}

/// MCP JSON-RPC response structure
#[derive(Debug, Serialize)]
pub struct McpResponse {
    pub jsonrpc: String,
    pub id: Option<Value>,
    pub result: Option<Value>,
    pub error: Option<Value>,
}

/// MCP JSON-RPC error structure
#[derive(Debug, Serialize)]
struct McpError {
    code: i32,
    message: String,
    data: Option<Value>,
}

impl McpResponse {
    /// Create a successful response
    fn success(id: Option<Value>, result: Value) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id,
            result: Some(result),
            error: None,
        }
    }

    /// Create an error response
    fn error(id: Option<Value>, code: i32, message: String) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id,
            result: None,
            error: Some(json!(McpError {
                code,
                message,
                data: None
            })),
        }
    }
}

impl<P: ResourceProvider + Send + Sync + 'static> ScimMcpServer<P> {
    /// Get the list of available MCP tools as JSON
    ///
    /// Returns all tool definitions that AI agents can discover and execute.
    /// Each tool includes its schema, parameters, and documentation.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # #[cfg(feature = "mcp")]
    /// use scim_server::mcp_integration::ScimMcpServer;
    /// use scim_server::providers::StandardResourceProvider;
    /// use scim_server::storage::InMemoryStorage;
    /// # async fn example(mcp_server: ScimMcpServer<StandardResourceProvider<InMemoryStorage>>) {
    /// let tools = mcp_server.get_tools();
    /// println!("Available tools: {}", tools.len());
    /// # }
    /// ```
    pub fn get_tools(&self) -> Vec<Value> {
        vec![
            user_schemas::create_user_tool(),
            user_schemas::get_user_tool(),
            user_schemas::update_user_tool(),
            user_schemas::delete_user_tool(),
            user_schemas::list_users_tool(),
            user_schemas::search_users_tool(),
            user_schemas::user_exists_tool(),
            system_schemas::get_schemas_tool(),
            system_schemas::get_server_info_tool(),
        ]
    }

    /// Execute a tool by name with arguments
    ///
    /// This is the main dispatch function that routes tool execution requests
    /// to the appropriate handler based on the tool name.
    ///
    /// # Arguments
    /// * `tool_name` - The name of the tool to execute
    /// * `arguments` - JSON arguments for the tool execution
    ///
    /// # Returns
    /// A `ScimToolResult` containing the execution outcome
    pub async fn execute_tool(&self, tool_name: &str, arguments: Value) -> ScimToolResult {
        debug!("Executing MCP tool: {} with args: {}", tool_name, arguments);

        match tool_name {
            // User CRUD operations
            "scim_create_user" => user_crud::handle_create_user(self, arguments).await,
            "scim_get_user" => user_crud::handle_get_user(self, arguments).await,
            "scim_update_user" => user_crud::handle_update_user(self, arguments).await,
            "scim_delete_user" => user_crud::handle_delete_user(self, arguments).await,

            // User query operations
            "scim_list_users" => user_queries::handle_list_users(self, arguments).await,
            "scim_search_users" => user_queries::handle_search_users(self, arguments).await,
            "scim_user_exists" => user_queries::handle_user_exists(self, arguments).await,

            // System information operations
            "scim_get_schemas" => system_info::handle_get_schemas(self, arguments).await,
            "scim_server_info" => system_info::handle_server_info(self, arguments).await,

            // Unknown tool
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

    /// Run the MCP server using stdio communication
    ///
    /// Starts the MCP server and begins listening for tool execution requests
    /// over standard input/output. This is the standard MCP communication method.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// # #[cfg(feature = "mcp")]
    /// use scim_server::mcp_integration::ScimMcpServer;
    /// use scim_server::providers::StandardResourceProvider;
    /// use scim_server::storage::InMemoryStorage;
    /// # async fn example(mcp_server: ScimMcpServer<StandardResourceProvider<InMemoryStorage>>) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    /// // Run MCP server
    /// mcp_server.run_stdio().await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn run_stdio(self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        info!("SCIM MCP server starting stdio communication");
        info!(
            "Available tools: {:?}",
            self.get_tools()
                .iter()
                .map(|t| t.get("name"))
                .collect::<Vec<_>>()
        );

        let stdin = tokio::io::stdin();
        let mut stdout = tokio::io::stdout();
        let mut reader = BufReader::new(stdin);
        let mut line = String::new();

        info!("SCIM MCP server ready - listening on stdio");

        loop {
            line.clear();
            match reader.read_line(&mut line).await {
                Ok(0) => {
                    debug!("EOF received, shutting down");
                    break; // EOF
                }
                Ok(_) => {
                    let line_content = line.trim();
                    if line_content.is_empty() {
                        continue;
                    }

                    debug!("Received request: {}", line_content);

                    if let Some(response) = self.handle_mcp_request(line_content).await {
                        let response_json = match serde_json::to_string(&response) {
                            Ok(json) => json,
                            Err(e) => {
                                error!("Failed to serialize response: {}", e);
                                continue;
                            }
                        };

                        debug!("Sending response: {}", response_json);

                        if let Err(e) = stdout.write_all(response_json.as_bytes()).await {
                            error!("Failed to write response: {}", e);
                            break;
                        }
                        if let Err(e) = stdout.write_all(b"\n").await {
                            error!("Failed to write newline: {}", e);
                            break;
                        }
                        if let Err(e) = stdout.flush().await {
                            error!("Failed to flush stdout: {}", e);
                            break;
                        }
                    }
                }
                Err(e) => {
                    error!("Error reading from stdin: {}", e);
                    break;
                }
            }
        }

        info!("SCIM MCP server shutting down");
        Ok(())
    }

    /// Handle a single MCP request and return the appropriate response
    pub async fn handle_mcp_request(&self, line: &str) -> Option<McpResponse> {
        let request: McpRequest = match serde_json::from_str(line) {
            Ok(req) => req,
            Err(e) => {
                warn!("Failed to parse JSON request: {} - Input: {}", e, line);
                return Some(McpResponse::error(None, -32700, "Parse error".to_string()));
            }
        };

        debug!(
            "Processing method: {} with id: {:?}",
            request.method, request.id
        );

        match request.method.as_str() {
            "initialize" => Some(self.handle_initialize(request.id)),
            "notifications/initialized" => {
                debug!("Received initialized notification - handshake complete");
                None // Notifications don't require responses
            }
            "tools/list" => Some(self.handle_tools_list(request.id)),
            "tools/call" => Some(self.handle_tools_call(request.id, request.params).await),
            "ping" => Some(self.handle_ping(request.id)),
            _ => {
                warn!("Unknown method: {}", request.method);
                Some(McpResponse::error(
                    request.id,
                    -32601,
                    "Method not found".to_string(),
                ))
            }
        }
    }

    /// Handle initialize request
    fn handle_initialize(&self, id: Option<Value>) -> McpResponse {
        debug!("Handling initialize request");

        let result = json!({
            "protocolVersion": "2024-11-05",
            "capabilities": {
                "tools": {}
            },
            "serverInfo": {
                "name": self.server_info.name,
                "version": self.server_info.version,
                "description": self.server_info.description
            }
        });

        McpResponse::success(id, result)
    }

    /// Handle tools/list request
    fn handle_tools_list(&self, id: Option<Value>) -> McpResponse {
        debug!("Handling tools/list request");

        let tools = self.get_tools();
        let result = json!({
            "tools": tools
        });

        McpResponse::success(id, result)
    }

    /// Handle tools/call request
    async fn handle_tools_call(&self, id: Option<Value>, params: Option<Value>) -> McpResponse {
        debug!("Handling tools/call request");

        let params = match params {
            Some(p) => p,
            None => {
                return McpResponse::error(
                    id,
                    -32602,
                    "Invalid params: missing parameters".to_string(),
                );
            }
        };

        let tool_name = match params.get("name").and_then(|n| n.as_str()) {
            Some(name) => name,
            None => {
                return McpResponse::error(
                    id,
                    -32602,
                    "Invalid params: missing tool name".to_string(),
                );
            }
        };

        let arguments = params.get("arguments").cloned().unwrap_or(json!({}));

        debug!(
            "Executing tool: {} with arguments: {}",
            tool_name, arguments
        );

        let tool_result = self.execute_tool(tool_name, arguments).await;

        if tool_result.success {
            let result = json!({
                "content": [
                    {
                        "type": "text",
                        "text": serde_json::to_string_pretty(&tool_result.content)
                            .unwrap_or_else(|_| "Error serializing result".to_string())
                    }
                ],
                "_meta": tool_result.metadata
            });

            McpResponse::success(id, result)
        } else {
            McpResponse::error(
                id,
                -32000,
                format!(
                    "Tool execution failed: {}",
                    tool_result
                        .content
                        .get("error")
                        .and_then(|e| e.as_str())
                        .unwrap_or("Unknown error")
                ),
            )
        }
    }

    /// Handle ping request
    fn handle_ping(&self, id: Option<Value>) -> McpResponse {
        debug!("Handling ping request");
        McpResponse::success(id, json!({}))
    }
}
