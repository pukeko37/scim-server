//! User CRUD operation handlers for MCP integration
//!
//! This module contains the implementation of all user Create, Read, Update, Delete
//! operations exposed through the MCP protocol. These handlers provide the business
//! logic for user lifecycle management with proper error handling, tenant isolation,
//! and ETag concurrency control.

use crate::{
    ResourceProvider,
    mcp_integration::core::{ScimMcpServer, ScimToolResult},
    multi_tenant::TenantContext,
    operation_handler::ScimOperationRequest,
    resource::version::ScimVersion,
};
use serde_json::{Value, json};

/// Handle user creation through MCP
///
/// Creates a new user resource with tenant isolation and ETag versioning support.
/// Returns the created user with version metadata for subsequent operations.
///
/// # Errors
///
/// Returns error result if:
/// - Required user_data parameter is missing
/// - User data fails SCIM schema validation
/// - userName already exists (duplicate user)
/// - Tenant permissions are insufficient
/// - Internal server error during creation
pub async fn handle_create_user<P: ResourceProvider + Send + Sync + 'static>(
    server: &ScimMcpServer<P>,
    arguments: Value,
) -> ScimToolResult {
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

    let response = server.operation_handler.handle_operation(request).await;

    if response.success {
        let mut content = response
            .data
            .unwrap_or_else(|| json!({"status": "created"}));

        // Include version information in response for AI agent to use in subsequent operations
        let mut metadata = json!({
            "operation": "create_user",
            "resource_type": "User",
            "resource_id": response.metadata.resource_id
        });

        if let Some(version) = response.metadata.additional.get("version") {
            metadata["version"] = version.clone();
        }
        if let Some(etag) = response.metadata.additional.get("etag") {
            metadata["etag"] = etag.clone();
            // Also include in content for easy access by AI
            if let Some(content_obj) = content.as_object_mut() {
                content_obj.insert("_etag".to_string(), etag.clone());
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
                "error_code": "CREATE_USER_FAILED"
            }),
            metadata: None,
        }
    }
}

/// Handle user retrieval through MCP
///
/// Retrieves a user by ID with tenant isolation and includes ETag information
/// for subsequent conditional operations.
///
/// # Errors
///
/// Returns error result if:
/// - Required user_id parameter is missing
/// - User with specified ID does not exist
/// - Tenant permissions are insufficient
/// - Internal server error during retrieval
pub async fn handle_get_user<P: ResourceProvider + Send + Sync + 'static>(
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

    if response.success {
        let mut content = response
            .data
            .unwrap_or_else(|| json!({"status": "retrieved"}));

        let mut metadata = json!({
            "operation": "get_user",
            "resource_type": "User",
            "resource_id": user_id
        });

        // Include version/ETag information for AI to use in conditional operations
        if let Some(version) = response.metadata.additional.get("version") {
            metadata["version"] = version.clone();
        }
        if let Some(etag) = response.metadata.additional.get("etag") {
            metadata["etag"] = etag.clone();
            // Include ETag in content for AI convenience
            if let Some(content_obj) = content.as_object_mut() {
                content_obj.insert("_etag".to_string(), etag.clone());
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
            .unwrap_or_else(|| "User not found".to_string());
        ScimToolResult {
            success: false,
            content: json!({
                "error": error_msg,
                "error_code": if error_msg.contains("not found") { "USER_NOT_FOUND" } else { "GET_USER_FAILED" },
                "user_id": user_id
            }),
            metadata: Some(json!({
                "operation": "get_user",
                "resource_id": user_id
            })),
        }
    }
}

/// Handle user update through MCP
///
/// Updates an existing user with optional ETag-based conditional update.
/// Supports optimistic concurrency control to prevent lost updates.
///
/// # Errors
///
/// Returns error result if:
/// - Required user_id or user_data parameters are missing
/// - User with specified ID does not exist
/// - ETag version conflict (if expected_version provided)
/// - User data fails SCIM schema validation
/// - Tenant permissions are insufficient
pub async fn handle_update_user<P: ResourceProvider + Send + Sync + 'static>(
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

    // Handle optional ETag-based conditional update
    if let Some(expected_version_str) = arguments.get("expected_version").and_then(|v| v.as_str()) {
        match ScimVersion::parse_http_header(expected_version_str) {
            Ok(version) => {
                request = request.with_expected_version(version);
            }
            Err(_) => {
                return ScimToolResult {
                    success: false,
                    content: json!({
                        "error": format!("Invalid expected_version format: '{}'. Expected ETag format like 'W/\"abc123\"'", expected_version_str),
                        "error_code": "INVALID_VERSION_FORMAT"
                    }),
                    metadata: None,
                };
            }
        }
    }

    let response = server.operation_handler.handle_operation(request).await;

    if response.success {
        let mut content = response
            .data
            .unwrap_or_else(|| json!({"status": "updated"}));

        let mut metadata = json!({
            "operation": "update_user",
            "resource_type": "User",
            "resource_id": user_id
        });

        // Include updated version information
        if let Some(version) = response.metadata.additional.get("version") {
            metadata["version"] = version.clone();
        }
        if let Some(etag) = response.metadata.additional.get("etag") {
            metadata["etag"] = etag.clone();
            // Include new ETag in content for AI convenience
            if let Some(content_obj) = content.as_object_mut() {
                content_obj.insert("_etag".to_string(), etag.clone());
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
        let error_code = if error_msg.contains("version mismatch") || error_msg.contains("ETag") {
            "VERSION_MISMATCH"
        } else if error_msg.contains("not found") {
            "USER_NOT_FOUND"
        } else {
            "UPDATE_USER_FAILED"
        };

        ScimToolResult {
            success: false,
            content: json!({
                "error": error_msg,
                "error_code": error_code,
                "user_id": user_id
            }),
            metadata: Some(json!({
                "operation": "update_user",
                "resource_id": user_id,
                "conditional_update": arguments.get("expected_version").is_some()
            })),
        }
    }
}

/// Handle user deletion through MCP
///
/// Deletes a user with optional ETag-based conditional delete.
/// Supports optimistic concurrency control to prevent accidental deletion of modified resources.
///
/// # Errors
///
/// Returns error result if:
/// - Required user_id parameter is missing
/// - User with specified ID does not exist
/// - ETag version conflict (if expected_version provided)
/// - Tenant permissions are insufficient
/// - Internal server error during deletion
pub async fn handle_delete_user<P: ResourceProvider + Send + Sync + 'static>(
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

    let mut request = ScimOperationRequest::delete("User".to_string(), user_id.to_string());
    if let Some(tenant) = tenant_context {
        request = request.with_tenant(tenant);
    }

    // Handle optional ETag-based conditional delete
    if let Some(expected_version_str) = arguments.get("expected_version").and_then(|v| v.as_str()) {
        match ScimVersion::parse_http_header(expected_version_str) {
            Ok(version) => {
                request = request.with_expected_version(version);
            }
            Err(_) => {
                return ScimToolResult {
                    success: false,
                    content: json!({
                        "error": format!("Invalid expected_version format: '{}'. Expected ETag format like 'W/\"abc123\"'", expected_version_str),
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
            content: json!({"status": "deleted", "user_id": user_id}),
            metadata: Some(json!({
                "operation": "delete_user",
                "resource_type": "User",
                "resource_id": user_id,
                "conditional_delete": arguments.get("expected_version").is_some()
            })),
        }
    } else {
        let error_msg = response
            .error
            .unwrap_or_else(|| "Delete failed".to_string());
        let error_code = if error_msg.contains("version mismatch") || error_msg.contains("ETag") {
            "VERSION_MISMATCH"
        } else if error_msg.contains("not found") {
            "USER_NOT_FOUND"
        } else {
            "DELETE_USER_FAILED"
        };

        ScimToolResult {
            success: false,
            content: json!({
                "error": error_msg,
                "error_code": error_code,
                "user_id": user_id
            }),
            metadata: Some(json!({
                "operation": "delete_user",
                "resource_id": user_id,
                "conditional_delete": arguments.get("expected_version").is_some()
            })),
        }
    }
}
