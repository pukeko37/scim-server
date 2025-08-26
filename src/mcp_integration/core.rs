//! Core MCP integration infrastructure
//!
//! This module contains the foundational types and constructors for MCP integration.
//! It provides the basic building blocks that other MCP modules depend on.

use crate::{ResourceProvider, operation_handler::ScimOperationHandler, scim_server::ScimServer};
use serde_json::Value;

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

impl Default for McpServerInfo {
    fn default() -> Self {
        Self {
            name: "SCIM Server".to_string(),
            version: "2.0".to_string(),
            description: "A comprehensive SCIM 2.0 server implementation".to_string(),
            supported_resource_types: vec!["User".to_string(), "Group".to_string()],
        }
    }
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
/// use scim_server::{ScimServer, mcp_integration::ScimMcpServer, providers::StandardResourceProvider};
/// use scim_server::storage::InMemoryStorage;
///
/// # #[cfg(feature = "mcp")]
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let storage = InMemoryStorage::new();
///     let provider = StandardResourceProvider::new(storage);
///     let scim_server = ScimServer::new(provider)?;
///     let mcp_server = ScimMcpServer::new(scim_server);
///
///     // Get available tools
///     let tools = mcp_server.get_tools();
///     println!("Available tools: {}", tools.len());
///
///     // Run MCP server
///     mcp_server.run_stdio().await.unwrap();
///     Ok(())
/// }
/// ```
pub struct ScimMcpServer<P: ResourceProvider> {
    pub(crate) operation_handler: ScimOperationHandler<P>,
    pub(crate) server_info: McpServerInfo,
}

impl<P: ResourceProvider + Send + Sync + 'static> ScimMcpServer<P> {
    /// Create a new MCP server with default configuration
    ///
    /// # Arguments
    /// * `scim_server` - The SCIM server instance to wrap
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// # #[cfg(feature = "mcp")]
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// use scim_server::{ScimServer, mcp_integration::ScimMcpServer, providers::StandardResourceProvider};
    /// use scim_server::storage::InMemoryStorage;
    ///
    /// let storage = InMemoryStorage::new();
    /// let provider = StandardResourceProvider::new(storage);
    /// let scim_server = ScimServer::new(provider)?;
    /// let mcp_server = ScimMcpServer::new(scim_server);
    /// # Ok(())
    /// # }
    /// ```
    pub fn new(scim_server: ScimServer<P>) -> Self {
        let operation_handler = ScimOperationHandler::new(scim_server);
        Self {
            operation_handler,
            server_info: McpServerInfo::default(),
        }
    }

    /// Create a new MCP server with custom server information
    ///
    /// # Arguments
    /// * `scim_server` - The SCIM server instance to wrap
    /// * `server_info` - Custom server metadata for AI agent discovery
    pub fn with_info(scim_server: ScimServer<P>, server_info: McpServerInfo) -> Self {
        let operation_handler = ScimOperationHandler::new(scim_server);
        Self {
            operation_handler,
            server_info,
        }
    }

    /// Get server information for introspection
    ///
    /// Returns a reference to the server metadata that AI agents use for discovery.
    /// This is primarily used for testing and debugging purposes.
    pub fn server_info(&self) -> &McpServerInfo {
        &self.server_info
    }
}
