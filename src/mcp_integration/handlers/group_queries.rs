//! Group query operation handlers for MCP integration
//!
//! This module contains the implementation of all group query and search operations
//! exposed through the MCP protocol. These handlers provide read-only access to
//! group data with proper tenant isolation and structured responses for AI agents.

use crate::{
    ResourceProvider,
    mcp_integration::core::{ScimMcpServer, ScimToolResult},
    mcp_integration::handlers::convert_resources_versions,
    multi_tenant::TenantContext,
    operation_handler::ScimOperationRequest,
};
use serde_json::{Value, json};

/// Handle group listing through MCP
///
/// Lists groups with optional pagination and tenant isolation.
/// Returns a structured list of groups for AI agent processing.
///
/// # Errors
///
/// Returns error result if:
/// - Tenant permissions are insufficient
/// - Internal server error during list operation
/// - Storage provider failure
pub async fn handle_list_groups<P: ResourceProvider + Send + Sync + 'static>(
    server: &ScimMcpServer<P>,
    arguments: Value,
) -> ScimToolResult {
    let tenant_context = arguments
        .get("tenant_id")
        .and_then(|t| t.as_str())
        .map(|id| TenantContext::new(id.to_string(), "mcp-client".to_string()));

    let mut request = ScimOperationRequest::list("Group".to_string());
    if let Some(tenant) = tenant_context {
        request = request.with_tenant(tenant);
    }

    let response = server.operation_handler.handle_operation(request).await;

    if response.success {
        let mut content = response.data.unwrap_or_else(|| json!([]));

        // Convert ETag versions to raw format for MCP consistency
        // Handle both direct array format and Resources wrapper format
        if content.is_array() {
            // Direct array format - convert the array directly
            convert_resources_versions(&mut content);
        } else if let Some(resources) = content.get_mut("Resources") {
            // Wrapped format - convert the Resources array
            convert_resources_versions(resources);
        }

        ScimToolResult {
            success: true,
            content,
            metadata: Some(json!({
                "operation": "list_groups",
                "resource_type": "Group"
            })),
        }
    } else {
        ScimToolResult {
            success: false,
            content: json!({
                "error": response.error.unwrap_or_else(|| "List failed".to_string()),
                "error_code": "LIST_GROUPS_FAILED"
            }),
            metadata: None,
        }
    }
}

/// Handle group search through MCP
///
/// Searches for groups by attribute value with tenant isolation.
/// Provides filtered results based on the search criteria.
///
/// # Errors
///
/// Returns error result if:
/// - Required attribute or value parameters are missing
/// - Search attribute is not supported or invalid
/// - Tenant permissions are insufficient
/// - Internal server error during search operation
pub async fn handle_search_groups<P: ResourceProvider + Send + Sync + 'static>(
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

    let mut request = ScimOperationRequest::list("Group".to_string());

    if let Some(tenant) = tenant_context {
        request = request.with_tenant(tenant);
    }

    let response = server.operation_handler.handle_operation(request).await;

    if response.success {
        let groups = response
            .data
            .and_then(|data| data.get("Resources").cloned())
            .unwrap_or_else(|| json!([]));

        // Filter groups by the specified attribute and value
        let filtered_groups: Vec<Value> = groups
            .as_array()
            .unwrap_or(&vec![])
            .iter()
            .filter(|group| {
                group
                    .get(attribute)
                    .and_then(|attr_val| attr_val.as_str())
                    .map_or(false, |attr_str| attr_str == value)
            })
            .cloned()
            .collect();

        // Convert ETag versions to raw format for MCP consistency
        let mut filtered_resources_json = json!(filtered_groups);
        convert_resources_versions(&mut filtered_resources_json);
        let filtered_groups = filtered_resources_json.as_array().unwrap().clone();

        ScimToolResult {
            success: true,
            content: json!({
                "Resources": filtered_groups,
                "totalResults": filtered_groups.len(),
                "searchCriteria": {
                    "attribute": attribute,
                    "value": value
                }
            }),
            metadata: Some(json!({
                "operation": "search_groups",
                "resource_type": "Group",
                "search_attribute": attribute,
                "search_value": value
            })),
        }
    } else {
        ScimToolResult {
            success: false,
            content: json!({
                "error": response.error.unwrap_or_else(|| "Search failed".to_string()),
                "error_code": "SEARCH_GROUPS_FAILED"
            }),
            metadata: None,
        }
    }
}

/// Handle group existence check through MCP
///
/// Checks if a group exists by ID with tenant isolation.
/// Returns a simple boolean result for AI agent decision making.
///
/// # Errors
///
/// Returns error result if:
/// - Required group_id parameter is missing
/// - Tenant permissions are insufficient
/// - Internal server error during existence check
pub async fn handle_group_exists<P: ResourceProvider + Send + Sync + 'static>(
    server: &ScimMcpServer<P>,
    arguments: Value,
) -> ScimToolResult {
    let group_id = match arguments.get("group_id").and_then(|id| id.as_str()) {
        Some(id) => id,
        None => {
            return ScimToolResult {
                success: false,
                content: json!({"error": "Missing group_id parameter"}),
                metadata: None,
            };
        }
    };

    let tenant_context = arguments
        .get("tenant_id")
        .and_then(|t| t.as_str())
        .map(|id| TenantContext::new(id.to_string(), "mcp-client".to_string()));

    let mut request = ScimOperationRequest::get("Group".to_string(), group_id.to_string());
    if let Some(tenant) = tenant_context {
        request = request.with_tenant(tenant);
    }

    let response = server.operation_handler.handle_operation(request).await;

    ScimToolResult {
        success: true,
        content: json!({
            "exists": response.success,
            "group_id": group_id
        }),
        metadata: Some(json!({
            "operation": "group_exists",
            "resource_type": "Group",
            "resource_id": group_id
        })),
    }
}
