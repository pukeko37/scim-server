//! Core operation handler infrastructure
//!
//! This module contains the foundational types and main dispatcher logic for SCIM operations.
//! It provides the central handler struct and operation dispatch functionality that other
//! operation handler modules depend on.

use crate::{
    ResourceProvider, ScimServer,
    resource::version::ScimVersion,
    resource::{RequestContext, TenantContext},
};
use log::{debug, info, warn};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

/// Framework-agnostic operation handler for SCIM operations
///
/// This handler provides a structured interface for performing SCIM operations
/// without being tied to any specific transport layer (HTTP, MCP, etc.).
pub struct ScimOperationHandler<P: ResourceProvider> {
    pub(super) server: ScimServer<P>,
}

/// Structured request for SCIM operations
///
/// This type encapsulates all the information needed to perform a SCIM operation
/// in a transport-agnostic way.
#[derive(Debug, Clone, PartialEq)]
pub struct ScimOperationRequest {
    /// The type of operation to perform
    pub operation: ScimOperationType,
    /// The resource type (e.g., "User", "Group")
    pub resource_type: String,
    /// Resource ID for operations that target a specific resource
    pub resource_id: Option<String>,
    /// Data payload for create/update operations
    pub data: Option<Value>,
    /// Query parameters for list/search operations
    pub query: Option<ScimQuery>,
    /// Tenant context for multi-tenant operations
    pub tenant_context: Option<TenantContext>,
    /// Request ID for tracing and correlation
    pub request_id: Option<String>,
    /// Expected version for conditional operations
    pub expected_version: Option<ScimVersion>,
}

/// Types of SCIM operations supported by the handler
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ScimOperationType {
    /// Create a new resource
    Create,
    /// Get a specific resource by ID
    Get,
    /// Update an existing resource
    Update,
    /// Delete a resource
    Delete,
    /// List resources with optional pagination and filtering
    List,
    /// Search resources by attribute
    Search,
    /// Get all available schemas
    GetSchemas,
    /// Get a specific schema by ID
    GetSchema,
    /// Check if a resource exists
    Exists,
}

/// Query parameters for list and search operations
#[derive(Debug, Clone, PartialEq)]
pub struct ScimQuery {
    /// Maximum number of results to return
    pub count: Option<usize>,
    /// Starting index for pagination
    pub start_index: Option<usize>,
    /// Filter expression for search
    pub filter: Option<String>,
    /// Attributes to include in results
    pub attributes: Option<Vec<String>>,
    /// Attributes to exclude from results
    pub excluded_attributes: Option<Vec<String>>,
    /// Specific attribute to search on
    pub search_attribute: Option<String>,
    /// Value to search for
    pub search_value: Option<Value>,
}

/// Structured response from SCIM operations
///
/// This type provides a consistent response format across all operation types
/// and transport layers.
#[derive(Debug, Clone, PartialEq)]
pub struct ScimOperationResponse {
    /// Whether the operation succeeded
    pub success: bool,
    /// The primary data returned by the operation
    pub data: Option<Value>,
    /// Error message if the operation failed
    pub error: Option<String>,
    /// Error code for programmatic handling
    pub error_code: Option<String>,
    /// Additional metadata about the operation including version information
    pub metadata: OperationMetadata,
}

/// Metadata about a SCIM operation
///
/// Contains contextual information about the operation including version data
/// for ETag-based concurrency control.
#[derive(Debug, Clone, PartialEq)]
pub struct OperationMetadata {
    /// Resource type involved in the operation
    pub resource_type: Option<String>,
    /// Resource ID if applicable
    pub resource_id: Option<String>,
    /// Number of resources returned (for list operations)
    pub resource_count: Option<usize>,
    /// Total number of resources available (for pagination)
    pub total_results: Option<usize>,
    /// Request ID for tracing
    pub request_id: String,
    /// Tenant ID if applicable
    pub tenant_id: Option<String>,
    /// Resource schemas involved
    pub schemas: Option<Vec<String>>,
    /// Additional metadata including version information
    pub additional: HashMap<String, Value>,
}

impl<P: ResourceProvider + Sync> ScimOperationHandler<P> {
    /// Create a new operation handler with the given SCIM server.
    pub fn new(server: ScimServer<P>) -> Self {
        Self { server }
    }

    /// Handle a structured SCIM operation request.
    ///
    /// This is the main entry point that dispatches to specific operation handlers
    /// based on the operation type.
    pub async fn handle_operation(&self, request: ScimOperationRequest) -> ScimOperationResponse {
        let request_id = request
            .request_id
            .clone()
            .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());

        info!(
            "SCIM operation handler processing {:?} for {} (request: '{}')",
            request.operation, request.resource_type, request_id
        );

        let context = self.create_request_context(&request, &request_id);

        let result = match request.operation {
            ScimOperationType::Create => {
                super::handlers::crud::handle_create(self, request, &context).await
            }
            ScimOperationType::Get => {
                super::handlers::crud::handle_get(self, request, &context).await
            }
            ScimOperationType::Update => {
                super::handlers::crud::handle_update(self, request, &context).await
            }
            ScimOperationType::Delete => {
                super::handlers::crud::handle_delete(self, request, &context).await
            }
            ScimOperationType::List => {
                super::handlers::query::handle_list(self, request, &context).await
            }
            ScimOperationType::Search => {
                super::handlers::query::handle_search(self, request, &context).await
            }
            ScimOperationType::GetSchemas => {
                super::handlers::schema::handle_get_schemas(self, request, &context).await
            }
            ScimOperationType::GetSchema => {
                super::handlers::schema::handle_get_schema(self, request, &context).await
            }
            ScimOperationType::Exists => {
                super::handlers::utility::handle_exists(self, request, &context).await
            }
        };

        match &result {
            Ok(_) => {
                debug!(
                    "SCIM operation handler completed successfully (request: '{}')",
                    request_id
                );
            }
            Err(e) => {
                warn!(
                    "SCIM operation handler failed: {} (request: '{}')",
                    e, request_id
                );
            }
        }

        result.unwrap_or_else(|e| super::errors::create_error_response(e, request_id))
    }

    /// Create a RequestContext from the operation request.
    pub(super) fn create_request_context(
        &self,
        request: &ScimOperationRequest,
        request_id: &str,
    ) -> RequestContext {
        match &request.tenant_context {
            Some(tenant_ctx) => {
                RequestContext::with_tenant(request_id.to_string(), tenant_ctx.clone())
            }
            None => RequestContext::new(request_id.to_string()),
        }
    }

    /// Get access to the underlying SCIM server.
    pub(super) fn server(&self) -> &ScimServer<P> {
        &self.server
    }
}
