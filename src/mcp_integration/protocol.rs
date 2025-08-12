//! MCP protocol layer for tool discovery and dispatch
//!
//! This module handles the core MCP protocol functionality including tool discovery,
//! execution dispatch, and protocol communication. It serves as the interface between
//! AI agents and the SCIM server operations.

use super::core::{ScimMcpServer, ScimToolResult};
use super::handlers::{system_info, user_crud, user_queries};
use super::tools::{system_schemas, user_schemas};
use crate::ResourceProvider;
use log::{debug, info};
use serde_json::{Value, json};

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
    /// # async fn example(mcp_server: ScimMcpServer<scim_server::providers::InMemoryProvider>) {
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
    /// # async fn example(mcp_server: ScimMcpServer<scim_server::providers::InMemoryProvider>) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    /// // Run MCP server
    /// mcp_server.run_stdio().await?;
    /// # Ok(())
    /// # }
    /// ```
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
}
