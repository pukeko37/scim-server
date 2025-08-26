//! System information handlers for MCP integration
//!
//! This module contains handlers for system-level operations that provide
//! metadata about the SCIM server capabilities, schemas, and configuration.
//! These operations help AI agents understand the server's capabilities.

use crate::{
    ResourceProvider,
    mcp_integration::core::{ScimMcpServer, ScimToolResult},
    operation_handler::ScimOperationRequest,
};
use serde_json::{Value, json};

/// Handle schema retrieval through MCP
///
/// Returns all available SCIM schemas that the server supports.
/// This helps AI agents understand the data structures they can work with.
///
/// # Errors
///
/// Returns error result if:
/// - Schema registry is not properly initialized
/// - Internal server error during schema retrieval
/// - Storage provider failure
pub async fn handle_get_schemas<P: ResourceProvider + Send + Sync + 'static>(
    server: &ScimMcpServer<P>,
    _arguments: Value,
) -> ScimToolResult {
    let request = ScimOperationRequest::get_schemas();

    let response = server.operation_handler.handle_operation(request).await;

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

/// Handle server information retrieval through MCP
///
/// Returns comprehensive information about the SCIM server including capabilities,
/// version information, and supported operations. This helps AI agents understand
/// what the server can do and how to interact with it effectively.
///
/// # Errors
///
/// This function rarely fails but may return error if:
/// - Internal server configuration is corrupted
/// - Memory allocation failure during info collection
pub async fn handle_server_info<P: ResourceProvider + Send + Sync + 'static>(
    server: &ScimMcpServer<P>,
    _arguments: Value,
) -> ScimToolResult {
    let info = json!({
        "name": server.server_info.name,
        "version": server.server_info.version,
        "description": server.server_info.description,
        "supported_resource_types": server.server_info.supported_resource_types,
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
            "available_tools": server.get_tools().len()
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
