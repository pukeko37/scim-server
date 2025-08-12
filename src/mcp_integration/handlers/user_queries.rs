//! User query operation handlers for MCP integration
//!
//! This module contains the implementation of all user query and search operations
//! exposed through the MCP protocol. These handlers provide read-only access to
//! user data with proper tenant isolation and structured responses for AI agents.

use crate::{
    ResourceProvider,
    mcp_integration::core::{ScimMcpServer, ScimToolResult},
    multi_tenant::TenantContext,
    operation_handler::ScimOperationRequest,
};
use serde_json::{Value, json};

/// Handle user listing through MCP
///
/// Lists users with optional pagination and tenant isolation.
/// Returns a structured list of users for AI agent processing.
pub async fn handle_list_users<P: ResourceProvider + Send + Sync + 'static>(
    server: &ScimMcpServer<P>,
    arguments: Value,
) -> ScimToolResult {
    let tenant_context = arguments
        .get("tenant_id")
        .and_then(|t| t.as_str())
        .map(|id| TenantContext::new(id.to_string(), "mcp-client".to_string()));

    let mut request = ScimOperationRequest::list("User".to_string());
    if let Some(tenant) = tenant_context {
        request = request.with_tenant(tenant);
    }

    let response = server.operation_handler.handle_operation(request).await;

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

/// Handle user search through MCP
///
/// Searches for users by attribute value with tenant isolation.
/// Provides filtered results based on the search criteria.
pub async fn handle_search_users<P: ResourceProvider + Send + Sync + 'static>(
    server: &ScimMcpServer<P>,
    arguments: Value,
) -> ScimToolResult {
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

    let response = server.operation_handler.handle_operation(request).await;

    if response.success {
        let users = response
            .data
            .and_then(|data| data.get("Resources").cloned())
            .unwrap_or_else(|| json!([]));

        // Filter users by the specified attribute and value
        let filtered_users: Vec<Value> = users
            .as_array()
            .unwrap_or(&vec![])
            .iter()
            .filter(|user| {
                user.get(attribute)
                    .and_then(|attr_val| attr_val.as_str())
                    .map_or(false, |attr_str| attr_str == value)
            })
            .cloned()
            .collect();

        ScimToolResult {
            success: true,
            content: json!({
                "Resources": filtered_users,
                "totalResults": filtered_users.len(),
                "searchCriteria": {
                    "attribute": attribute,
                    "value": value
                }
            }),
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

/// Handle user existence check through MCP
///
/// Checks if a user exists by ID with tenant isolation.
/// Returns a simple boolean result for AI agent decision making.
pub async fn handle_user_exists<P: ResourceProvider + Send + Sync + 'static>(
    server: &ScimMcpServer<P>,
    arguments: Value,
) -> ScimToolResult {
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

    let response = server.operation_handler.handle_operation(request).await;

    ScimToolResult {
        success: true,
        content: json!({
            "exists": response.success,
            "user_id": user_id
        }),
        metadata: Some(json!({
            "operation": "user_exists",
            "resource_type": "User",
            "resource_id": user_id
        })),
    }
}
