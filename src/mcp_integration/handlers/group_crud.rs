//! Group CRUD operation handlers for MCP integration
//!
//! This module contains the implementation of all group Create, Read, Update, Delete
//! operations exposed through the MCP protocol. These handlers provide the business
//! logic for group lifecycle management with proper error handling, tenant isolation,
//! and version-based concurrency control.

use crate::{
    ResourceProvider,
    mcp_integration::core::{ScimMcpServer, ScimToolResult},
    mcp_integration::handlers::etag_to_raw_version,
    multi_tenant::TenantContext,
    operation_handler::ScimOperationRequest,
    resource::version::ScimVersion,
};
use serde_json::{Value, json};

/// Handle group creation through MCP
///
/// Creates a new group resource with tenant isolation and versioning support.
/// Returns the created group with version metadata for subsequent operations.
///
/// # Errors
///
/// Returns error result if:
/// - Required group_data parameter is missing
/// - Group data fails SCIM schema validation
/// - displayName already exists (duplicate group)
/// - Tenant permissions are insufficient
/// - Internal server error during creation
pub async fn handle_create_group<P: ResourceProvider + Send + Sync + 'static>(
    server: &ScimMcpServer<P>,
    arguments: Value,
) -> ScimToolResult {
    let group_data = match arguments.get("group_data") {
        Some(data) => data.clone(),
        None => {
            return ScimToolResult {
                success: false,
                content: json!({"error": "Missing group_data parameter"}),
                metadata: None,
            };
        }
    };

    let tenant_context = arguments
        .get("tenant_id")
        .and_then(|t| t.as_str())
        .map(|id| TenantContext::new(id.to_string(), "mcp-client".to_string()));

    let mut request = ScimOperationRequest::create("Group".to_string(), group_data);
    if let Some(tenant) = tenant_context {
        request = request.with_tenant(tenant);
    }

    let response = server.operation_handler.handle_operation(request).await;

    if response.success {
        let content = response
            .data
            .unwrap_or_else(|| json!({"status": "created"}));

        // Include version information in response for AI agent to use in subsequent operations
        let mut metadata = json!({
            "operation": "create_group",
            "resource_type": "Group",
            "resource_id": response.metadata.resource_id
        });

        if let Some(etag) = response.metadata.additional.get("etag") {
            if let Some(raw_version) = etag_to_raw_version(etag) {
                metadata["version"] = json!(raw_version);
            }
        }

        ScimToolResult {
            success: true,
            content,
            metadata: Some(metadata),
        }
    } else {
        ScimToolResult {
            success: false,
            content: json!({
                "error": response.error.unwrap_or_else(|| "Create failed".to_string()),
                "error_code": "CREATE_GROUP_FAILED"
            }),
            metadata: None,
        }
    }
}

/// Handle group retrieval through MCP
///
/// Retrieves a group by ID with tenant isolation and includes version information
/// for subsequent conditional operations.
///
/// # Errors
///
/// Returns error result if:
/// - Required group_id parameter is missing
/// - Group with specified ID does not exist
/// - Tenant permissions are insufficient
/// - Internal server error during retrieval
pub async fn handle_get_group<P: ResourceProvider + Send + Sync + 'static>(
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

    if response.success {
        let content = response
            .data
            .unwrap_or_else(|| json!({"status": "retrieved"}));

        let mut metadata = json!({
            "operation": "get_group",
            "resource_type": "Group",
            "resource_id": group_id
        });

        // Include version information for AI to use in conditional operations
        if let Some(etag) = response.metadata.additional.get("etag") {
            if let Some(raw_version) = etag_to_raw_version(etag) {
                metadata["version"] = json!(raw_version);
            }
        }

        ScimToolResult {
            success: true,
            content,
            metadata: Some(metadata),
        }
    } else {
        let error_msg = response
            .error
            .unwrap_or_else(|| "Group not found".to_string());
        ScimToolResult {
            success: false,
            content: json!({
                "error": error_msg,
                "error_code": if error_msg.contains("not found") { "GROUP_NOT_FOUND" } else { "GET_GROUP_FAILED" },
                "group_id": group_id
            }),
            metadata: Some(json!({
                "operation": "get_group",
                "resource_id": group_id
            })),
        }
    }
}

/// Handle group update through MCP
///
/// Updates an existing group with optional version-based conditional update.
/// Supports optimistic concurrency control to prevent lost updates.
///
/// # Errors
///
/// Returns error result if:
/// - Required group_id or group_data parameters are missing
/// - Group with specified ID does not exist
/// - Version conflict (if expected_version provided)
/// - Group data fails SCIM schema validation
/// - Tenant permissions are insufficient
pub async fn handle_update_group<P: ResourceProvider + Send + Sync + 'static>(
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

    let group_data = match arguments.get("group_data") {
        Some(data) => data.clone(),
        None => {
            return ScimToolResult {
                success: false,
                content: json!({"error": "Missing group_data parameter"}),
                metadata: None,
            };
        }
    };

    let tenant_context = arguments
        .get("tenant_id")
        .and_then(|t| t.as_str())
        .map(|id| TenantContext::new(id.to_string(), "mcp-client".to_string()));

    let mut request =
        ScimOperationRequest::update("Group".to_string(), group_id.to_string(), group_data);
    if let Some(tenant) = tenant_context {
        request = request.with_tenant(tenant);
    }

    // Handle optional version-based conditional update
    if let Some(expected_version_str) = arguments.get("expected_version").and_then(|v| v.as_str()) {
        // Try parsing as HTTP ETag format first
        let version_result = ScimVersion::parse_http_header(expected_version_str)
            .or_else(|_| ScimVersion::parse_raw(expected_version_str));

        match version_result {
            Ok(version) => {
                request = request.with_expected_version(version);
            }
            Err(_) => {
                return ScimToolResult {
                    success: false,
                    content: json!({
                        "error": format!("Invalid expected_version format: '{}'. Use raw format (e.g., 'abc123def') or ETag format (e.g., 'W/\"abc123def\"')", expected_version_str),
                        "error_code": "INVALID_VERSION_FORMAT"
                    }),
                    metadata: None,
                };
            }
        }
    }

    let response = server.operation_handler.handle_operation(request).await;

    if response.success {
        let content = response
            .data
            .unwrap_or_else(|| json!({"status": "updated"}));

        let mut metadata = json!({
            "operation": "update_group",
            "resource_type": "Group",
            "resource_id": group_id
        });

        // Include updated version information
        if let Some(etag) = response.metadata.additional.get("etag") {
            if let Some(raw_version) = etag_to_raw_version(etag) {
                metadata["version"] = json!(raw_version);
            }
        }

        ScimToolResult {
            success: true,
            content,
            metadata: Some(metadata),
        }
    } else {
        let error_msg = response
            .error
            .unwrap_or_else(|| "Update failed".to_string());
        let error_code = if error_msg.contains("version mismatch")
            || error_msg.contains("modified by another client")
        {
            "VERSION_MISMATCH"
        } else if error_msg.contains("not found") {
            "GROUP_NOT_FOUND"
        } else {
            "UPDATE_GROUP_FAILED"
        };

        ScimToolResult {
            success: false,
            content: json!({
                "error": error_msg,
                "error_code": error_code,
                "group_id": group_id
            }),
            metadata: Some(json!({
                "operation": "update_group",
                "resource_id": group_id,
                "conditional_update": arguments.get("expected_version").is_some()
            })),
        }
    }
}

/// Handle group deletion through MCP
///
/// Deletes a group with optional version-based conditional delete.
/// Supports optimistic concurrency control to prevent accidental deletion of modified resources.
///
/// # Errors
///
/// Returns error result if:
/// - Required group_id parameter is missing
/// - Group with specified ID does not exist
/// - Version conflict (if expected_version provided)
/// - Tenant permissions are insufficient
/// - Internal server error during deletion
pub async fn handle_delete_group<P: ResourceProvider + Send + Sync + 'static>(
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

    let mut request = ScimOperationRequest::delete("Group".to_string(), group_id.to_string());
    if let Some(tenant) = tenant_context {
        request = request.with_tenant(tenant);
    }

    // Handle optional version-based conditional delete
    if let Some(expected_version_str) = arguments.get("expected_version").and_then(|v| v.as_str()) {
        // Try parsing as HTTP ETag format first
        let version_result = ScimVersion::parse_http_header(expected_version_str)
            .or_else(|_| ScimVersion::parse_raw(expected_version_str));

        match version_result {
            Ok(version) => {
                request = request.with_expected_version(version);
            }
            Err(_) => {
                return ScimToolResult {
                    success: false,
                    content: json!({
                        "error": format!("Invalid expected_version format: '{}'. Use raw format (e.g., 'abc123def') or ETag format (e.g., 'W/\"abc123def\"')", expected_version_str),
                        "error_code": "INVALID_VERSION_FORMAT"
                    }),
                    metadata: None,
                };
            }
        }
    }

    let response = server.operation_handler.handle_operation(request).await;

    if response.success {
        ScimToolResult {
            success: true,
            content: json!({"status": "deleted", "group_id": group_id}),
            metadata: Some(json!({
                "operation": "delete_group",
                "resource_type": "Group",
                "resource_id": group_id,
                "conditional_delete": arguments.get("expected_version").is_some()
            })),
        }
    } else {
        let error_msg = response
            .error
            .unwrap_or_else(|| "Delete failed".to_string());
        let error_code = if error_msg.contains("version mismatch")
            || error_msg.contains("modified by another client")
        {
            "VERSION_MISMATCH"
        } else if error_msg.contains("not found") {
            "GROUP_NOT_FOUND"
        } else {
            "DELETE_GROUP_FAILED"
        };

        ScimToolResult {
            success: false,
            content: json!({
                "error": error_msg,
                "error_code": error_code,
                "group_id": group_id
            }),
            metadata: Some(json!({
                "operation": "delete_group",
                "resource_id": group_id,
                "conditional_delete": arguments.get("expected_version").is_some()
            })),
        }
    }
}
