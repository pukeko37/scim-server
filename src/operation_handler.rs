//! Operation Handler Foundation
//!
//! This module provides a framework-agnostic operation handler that serves as the foundation
//! for both HTTP and MCP integrations. It abstracts SCIM operations into structured
//! request/response types while maintaining type safety and comprehensive error handling.

use crate::error::{ScimError, ScimResult};
use crate::resource::{RequestContext, ResourceProvider, TenantContext};
use crate::scim_server::ScimServer;
use log::{debug, info, warn};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

/// Framework-agnostic operation handler for SCIM operations.
///
/// This handler provides structured request/response handling that can be used
/// by both HTTP handlers and MCP tools, ensuring consistent behavior across
/// different integration methods.
pub struct ScimOperationHandler<P: ResourceProvider> {
    server: ScimServer<P>,
}

/// Structured request for SCIM operations.
///
/// This abstraction allows the same operation logic to be used by different
/// frontends (HTTP, MCP, etc.) while maintaining type safety.
#[derive(Debug, Clone, Serialize, Deserialize)]
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
    /// Tenant context for multi-tenant scenarios
    pub tenant_context: Option<TenantContext>,
    /// Request ID for tracing (will be generated if not provided)
    pub request_id: Option<String>,
}

/// Operation types supported by the handler.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
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
    /// Search for resources by attribute
    Search,
    /// Get all available schemas
    GetSchemas,
    /// Get a specific schema by ID
    GetSchema,
    /// Check if a resource exists
    Exists,
}

/// Query parameters for list and search operations.
#[derive(Debug, Clone, Serialize, Deserialize)]
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
    /// Attribute name for search operations
    pub search_attribute: Option<String>,
    /// Value to search for
    pub search_value: Option<Value>,
}

/// Structured response from SCIM operations.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScimOperationResponse {
    /// Whether the operation succeeded
    pub success: bool,
    /// The primary data returned by the operation
    pub data: Option<Value>,
    /// Error message if the operation failed
    pub error: Option<String>,
    /// Error code for programmatic handling
    pub error_code: Option<String>,
    /// Additional metadata about the operation
    pub metadata: OperationMetadata,
}

/// Metadata about the operation result.
#[derive(Debug, Clone, Serialize, Deserialize)]
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
    /// Schemas involved in the operation
    pub schemas: Option<Vec<String>>,
    /// Additional context-specific metadata
    pub additional: HashMap<String, Value>,
}

impl<P: ResourceProvider> ScimOperationHandler<P> {
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
            ScimOperationType::Create => self.handle_create(request, &context).await,
            ScimOperationType::Get => self.handle_get(request, &context).await,
            ScimOperationType::Update => self.handle_update(request, &context).await,
            ScimOperationType::Delete => self.handle_delete(request, &context).await,
            ScimOperationType::List => self.handle_list(request, &context).await,
            ScimOperationType::Search => self.handle_search(request, &context).await,
            ScimOperationType::GetSchemas => self.handle_get_schemas(request, &context).await,
            ScimOperationType::GetSchema => self.handle_get_schema(request, &context).await,
            ScimOperationType::Exists => self.handle_exists(request, &context).await,
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

        result.unwrap_or_else(|e| self.create_error_response(e, request_id))
    }

    /// Create a RequestContext from the operation request.
    fn create_request_context(
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

    /// Handle create operations.
    async fn handle_create(
        &self,
        request: ScimOperationRequest,
        context: &RequestContext,
    ) -> ScimResult<ScimOperationResponse> {
        let data = request.data.ok_or_else(|| {
            ScimError::invalid_request("Missing data for create operation".to_string())
        })?;

        let resource = self
            .server
            .create_resource(&request.resource_type, data, context)
            .await?;

        Ok(ScimOperationResponse {
            success: true,
            data: Some(resource.to_json()?),
            error: None,
            error_code: None,
            metadata: OperationMetadata {
                resource_type: Some(request.resource_type),
                resource_id: resource.get_id().map(|s| s.to_string()),
                resource_count: Some(1),
                total_results: None,
                request_id: context.request_id.clone(),
                tenant_id: context.tenant_context.as_ref().map(|t| t.tenant_id.clone()),
                schemas: Some(
                    resource
                        .schemas
                        .iter()
                        .map(|s| s.as_str().to_string())
                        .collect(),
                ),
                additional: HashMap::new(),
            },
        })
    }

    /// Handle get operations.
    async fn handle_get(
        &self,
        request: ScimOperationRequest,
        context: &RequestContext,
    ) -> ScimResult<ScimOperationResponse> {
        let resource_id = request.resource_id.ok_or_else(|| {
            ScimError::invalid_request("Missing resource_id for get operation".to_string())
        })?;

        let resource = self
            .server
            .get_resource(&request.resource_type, &resource_id, context)
            .await?;

        match resource {
            Some(resource) => Ok(ScimOperationResponse {
                success: true,
                data: Some(resource.to_json()?),
                error: None,
                error_code: None,
                metadata: OperationMetadata {
                    resource_type: Some(request.resource_type),
                    resource_id: Some(resource_id),
                    resource_count: Some(1),
                    total_results: None,
                    request_id: context.request_id.clone(),
                    tenant_id: context.tenant_context.as_ref().map(|t| t.tenant_id.clone()),
                    schemas: Some(
                        resource
                            .schemas
                            .iter()
                            .map(|s| s.as_str().to_string())
                            .collect(),
                    ),
                    additional: HashMap::new(),
                },
            }),
            None => Err(ScimError::resource_not_found(
                request.resource_type,
                resource_id,
            )),
        }
    }

    /// Handle update operations.
    async fn handle_update(
        &self,
        request: ScimOperationRequest,
        context: &RequestContext,
    ) -> ScimResult<ScimOperationResponse> {
        let resource_id = request.resource_id.ok_or_else(|| {
            ScimError::invalid_request("Missing resource_id for update operation".to_string())
        })?;

        let data = request.data.ok_or_else(|| {
            ScimError::invalid_request("Missing data for update operation".to_string())
        })?;

        let resource = self
            .server
            .update_resource(&request.resource_type, &resource_id, data, context)
            .await?;

        Ok(ScimOperationResponse {
            success: true,
            data: Some(resource.to_json()?),
            error: None,
            error_code: None,
            metadata: OperationMetadata {
                resource_type: Some(request.resource_type),
                resource_id: Some(resource_id),
                resource_count: Some(1),
                total_results: None,
                request_id: context.request_id.clone(),
                tenant_id: context.tenant_context.as_ref().map(|t| t.tenant_id.clone()),
                schemas: Some(
                    resource
                        .schemas
                        .iter()
                        .map(|s| s.as_str().to_string())
                        .collect(),
                ),
                additional: HashMap::new(),
            },
        })
    }

    /// Handle delete operations.
    async fn handle_delete(
        &self,
        request: ScimOperationRequest,
        context: &RequestContext,
    ) -> ScimResult<ScimOperationResponse> {
        let resource_id = request.resource_id.ok_or_else(|| {
            ScimError::invalid_request("Missing resource_id for delete operation".to_string())
        })?;

        self.server
            .delete_resource(&request.resource_type, &resource_id, context)
            .await?;

        Ok(ScimOperationResponse {
            success: true,
            data: None,
            error: None,
            error_code: None,
            metadata: OperationMetadata {
                resource_type: Some(request.resource_type),
                resource_id: Some(resource_id),
                resource_count: Some(0),
                total_results: None,
                request_id: context.request_id.clone(),
                tenant_id: context.tenant_context.as_ref().map(|t| t.tenant_id.clone()),
                schemas: None,
                additional: HashMap::new(),
            },
        })
    }

    /// Handle list operations.
    async fn handle_list(
        &self,
        request: ScimOperationRequest,
        context: &RequestContext,
    ) -> ScimResult<ScimOperationResponse> {
        let resources = self
            .server
            .list_resources(&request.resource_type, context)
            .await?;

        let resource_data: Vec<Value> = resources
            .iter()
            .map(|r| r.to_json())
            .collect::<Result<Vec<_>, _>>()?;
        let count = resource_data.len();

        Ok(ScimOperationResponse {
            success: true,
            data: Some(serde_json::json!({
                "Resources": resource_data,
                "totalResults": count,
                "startIndex": request.query.as_ref().and_then(|q| q.start_index).unwrap_or(1),
                "itemsPerPage": count
            })),
            error: None,
            error_code: None,
            metadata: OperationMetadata {
                resource_type: Some(request.resource_type),
                resource_id: None,
                resource_count: Some(count),
                total_results: Some(count),
                request_id: context.request_id.clone(),
                tenant_id: context.tenant_context.as_ref().map(|t| t.tenant_id.clone()),
                schemas: None,
                additional: HashMap::new(),
            },
        })
    }

    /// Handle search operations.
    async fn handle_search(
        &self,
        request: ScimOperationRequest,
        context: &RequestContext,
    ) -> ScimResult<ScimOperationResponse> {
        let query = request.query.ok_or_else(|| {
            ScimError::invalid_request("Missing query for search operation".to_string())
        })?;

        let search_attribute = query.search_attribute.ok_or_else(|| {
            ScimError::invalid_request("Missing search_attribute for search operation".to_string())
        })?;

        let search_value = query.search_value.ok_or_else(|| {
            ScimError::invalid_request("Missing search_value for search operation".to_string())
        })?;

        let resource = self
            .server
            .find_resource_by_attribute(
                &request.resource_type,
                &search_attribute,
                &search_value,
                context,
            )
            .await?;

        match resource {
            Some(resource) => Ok(ScimOperationResponse {
                success: true,
                data: Some(resource.to_json()?),
                error: None,
                error_code: None,
                metadata: OperationMetadata {
                    resource_type: Some(request.resource_type),
                    resource_id: resource.get_id().map(|s| s.to_string()),
                    resource_count: Some(1),
                    total_results: Some(1),
                    request_id: context.request_id.clone(),
                    tenant_id: context.tenant_context.as_ref().map(|t| t.tenant_id.clone()),
                    schemas: Some(
                        resource
                            .schemas
                            .iter()
                            .map(|s| s.as_str().to_string())
                            .collect(),
                    ),
                    additional: HashMap::new(),
                },
            }),
            None => Ok(ScimOperationResponse {
                success: true,
                data: None,
                error: None,
                error_code: None,
                metadata: OperationMetadata {
                    resource_type: Some(request.resource_type),
                    resource_id: None,
                    resource_count: Some(0),
                    total_results: Some(0),
                    request_id: context.request_id.clone(),
                    tenant_id: context.tenant_context.as_ref().map(|t| t.tenant_id.clone()),
                    schemas: None,
                    additional: HashMap::new(),
                },
            }),
        }
    }

    /// Handle schema retrieval operations.
    async fn handle_get_schemas(
        &self,
        _request: ScimOperationRequest,
        context: &RequestContext,
    ) -> ScimResult<ScimOperationResponse> {
        let schemas = self.server.get_all_schemas();
        let schema_data: Vec<Value> = schemas
            .iter()
            .map(|s| {
                serde_json::json!({
                    "id": s.id,
                    "name": s.name,
                    "description": s.description,
                    "attributes": s.attributes.iter().map(|attr| {
                        serde_json::json!({
                            "name": attr.name,
                            "type": attr.data_type,
                            "required": attr.required,
                            "multiValued": attr.multi_valued,
                            "mutability": attr.mutability,
                            "returned": attr.returned,
                            "uniqueness": attr.uniqueness,
                            "canonicalValues": attr.canonical_values
                        })
                    }).collect::<Vec<_>>()
                })
            })
            .collect();

        Ok(ScimOperationResponse {
            success: true,
            data: Some(serde_json::json!({
                "schemas": schema_data,
                "totalResults": schema_data.len()
            })),
            error: None,
            error_code: None,
            metadata: OperationMetadata {
                resource_type: None,
                resource_id: None,
                resource_count: Some(schema_data.len()),
                total_results: Some(schema_data.len()),
                request_id: context.request_id.clone(),
                tenant_id: context.tenant_context.as_ref().map(|t| t.tenant_id.clone()),
                schemas: Some(schemas.iter().map(|s| s.id.clone()).collect()),
                additional: HashMap::new(),
            },
        })
    }

    /// Handle single schema retrieval.
    async fn handle_get_schema(
        &self,
        request: ScimOperationRequest,
        context: &RequestContext,
    ) -> ScimResult<ScimOperationResponse> {
        let schema_id = request.resource_id.ok_or_else(|| {
            ScimError::invalid_request("Missing schema_id for get schema operation".to_string())
        })?;

        let schema = self
            .server
            .get_schema_by_id(&schema_id)
            .ok_or_else(|| ScimError::schema_not_found(schema_id.clone()))?;

        let schema_data = serde_json::json!({
            "id": schema.id,
            "name": schema.name,
            "description": schema.description,
            "attributes": schema.attributes.iter().map(|attr| {
                serde_json::json!({
                    "name": attr.name,
                    "type": attr.data_type,
                    "required": attr.required,
                    "multiValued": attr.multi_valued,
                    "mutability": attr.mutability,
                    "returned": attr.returned,
                    "uniqueness": attr.uniqueness,
                    "canonicalValues": attr.canonical_values
                })
            }).collect::<Vec<_>>()
        });

        Ok(ScimOperationResponse {
            success: true,
            data: Some(schema_data),
            error: None,
            error_code: None,
            metadata: OperationMetadata {
                resource_type: None,
                resource_id: Some(schema_id),
                resource_count: Some(1),
                total_results: Some(1),
                request_id: context.request_id.clone(),
                tenant_id: context.tenant_context.as_ref().map(|t| t.tenant_id.clone()),
                schemas: Some(vec![schema.id.clone()]),
                additional: HashMap::new(),
            },
        })
    }

    /// Handle resource existence check.
    async fn handle_exists(
        &self,
        request: ScimOperationRequest,
        context: &RequestContext,
    ) -> ScimResult<ScimOperationResponse> {
        let resource_id = request.resource_id.ok_or_else(|| {
            ScimError::invalid_request("Missing resource_id for exists operation".to_string())
        })?;

        let exists = self
            .server
            .resource_exists(&request.resource_type, &resource_id, context)
            .await?;

        Ok(ScimOperationResponse {
            success: true,
            data: Some(serde_json::json!({ "exists": exists })),
            error: None,
            error_code: None,
            metadata: OperationMetadata {
                resource_type: Some(request.resource_type),
                resource_id: Some(resource_id),
                resource_count: if exists { Some(1) } else { Some(0) },
                total_results: if exists { Some(1) } else { Some(0) },
                request_id: context.request_id.clone(),
                tenant_id: context.tenant_context.as_ref().map(|t| t.tenant_id.clone()),
                schemas: None,
                additional: HashMap::new(),
            },
        })
    }

    /// Create an error response from a ScimError.
    fn create_error_response(&self, error: ScimError, request_id: String) -> ScimOperationResponse {
        let (error_message, error_code) = match &error {
            ScimError::Validation(ve) => (
                format!("Validation error: {}", ve),
                Some("VALIDATION_ERROR"),
            ),
            ScimError::ResourceNotFound { resource_type, id } => (
                format!("Resource not found: {} with ID {}", resource_type, id),
                Some("RESOURCE_NOT_FOUND"),
            ),
            ScimError::SchemaNotFound { schema_id } => (
                format!("Schema not found: {}", schema_id),
                Some("SCHEMA_NOT_FOUND"),
            ),
            ScimError::UnsupportedResourceType(resource_type) => (
                format!("Unsupported resource type: {}", resource_type),
                Some("UNSUPPORTED_RESOURCE_TYPE"),
            ),
            ScimError::UnsupportedOperation {
                resource_type,
                operation,
            } => (
                format!(
                    "Unsupported operation {} for resource type {}",
                    operation, resource_type
                ),
                Some("UNSUPPORTED_OPERATION"),
            ),
            ScimError::InvalidRequest { message } => (
                format!("Invalid request: {}", message),
                Some("INVALID_REQUEST"),
            ),
            ScimError::Provider(provider_error) => (
                format!("Provider error: {}", provider_error),
                Some("PROVIDER_ERROR"),
            ),
            ScimError::Internal { message } => (
                format!("Internal error: {}", message),
                Some("INTERNAL_ERROR"),
            ),
            _ => (error.to_string(), Some("UNKNOWN_ERROR")),
        };

        ScimOperationResponse {
            success: false,
            data: None,
            error: Some(error_message),
            error_code: error_code.map(|s| s.to_string()),
            metadata: OperationMetadata {
                resource_type: None,
                resource_id: None,
                resource_count: None,
                total_results: None,
                request_id,
                tenant_id: None,
                schemas: None,
                additional: HashMap::new(),
            },
        }
    }
}

impl ScimOperationRequest {
    /// Create a new create operation request.
    pub fn create(resource_type: impl Into<String>, data: Value) -> Self {
        Self {
            operation: ScimOperationType::Create,
            resource_type: resource_type.into(),
            resource_id: None,
            data: Some(data),
            query: None,
            tenant_context: None,
            request_id: None,
        }
    }

    /// Create a new get operation request.
    pub fn get(resource_type: impl Into<String>, resource_id: impl Into<String>) -> Self {
        Self {
            operation: ScimOperationType::Get,
            resource_type: resource_type.into(),
            resource_id: Some(resource_id.into()),
            data: None,
            query: None,
            tenant_context: None,
            request_id: None,
        }
    }

    /// Create a new update operation request.
    pub fn update(
        resource_type: impl Into<String>,
        resource_id: impl Into<String>,
        data: Value,
    ) -> Self {
        Self {
            operation: ScimOperationType::Update,
            resource_type: resource_type.into(),
            resource_id: Some(resource_id.into()),
            data: Some(data),
            query: None,
            tenant_context: None,
            request_id: None,
        }
    }

    /// Create a new delete operation request.
    pub fn delete(resource_type: impl Into<String>, resource_id: impl Into<String>) -> Self {
        Self {
            operation: ScimOperationType::Delete,
            resource_type: resource_type.into(),
            resource_id: Some(resource_id.into()),
            data: None,
            query: None,
            tenant_context: None,
            request_id: None,
        }
    }

    /// Create a new list operation request.
    pub fn list(resource_type: impl Into<String>) -> Self {
        Self {
            operation: ScimOperationType::List,
            resource_type: resource_type.into(),
            resource_id: None,
            data: None,
            query: None,
            tenant_context: None,
            request_id: None,
        }
    }

    /// Create a new search operation request.
    pub fn search(
        resource_type: impl Into<String>,
        attribute: impl Into<String>,
        value: Value,
    ) -> Self {
        Self {
            operation: ScimOperationType::Search,
            resource_type: resource_type.into(),
            resource_id: None,
            data: None,
            query: Some(ScimQuery {
                count: None,
                start_index: None,
                filter: None,
                attributes: None,
                excluded_attributes: None,
                search_attribute: Some(attribute.into()),
                search_value: Some(value),
            }),
            tenant_context: None,
            request_id: None,
        }
    }

    /// Create a new get schemas operation request.
    pub fn get_schemas() -> Self {
        Self {
            operation: ScimOperationType::GetSchemas,
            resource_type: "Schema".to_string(),
            resource_id: None,
            data: None,
            query: None,
            tenant_context: None,
            request_id: None,
        }
    }

    /// Create a new get schema operation request.
    pub fn get_schema(schema_id: impl Into<String>) -> Self {
        Self {
            operation: ScimOperationType::GetSchema,
            resource_type: "Schema".to_string(),
            resource_id: Some(schema_id.into()),
            data: None,
            query: None,
            tenant_context: None,
            request_id: None,
        }
    }

    /// Create a new resource exists operation request.
    pub fn exists(resource_type: impl Into<String>, resource_id: impl Into<String>) -> Self {
        Self {
            operation: ScimOperationType::Exists,
            resource_type: resource_type.into(),
            resource_id: Some(resource_id.into()),
            data: None,
            query: None,
            tenant_context: None,
            request_id: None,
        }
    }

    /// Add tenant context to the request.
    pub fn with_tenant(mut self, tenant_context: TenantContext) -> Self {
        self.tenant_context = Some(tenant_context);
        self
    }

    /// Add request ID to the request.
    pub fn with_request_id(mut self, request_id: impl Into<String>) -> Self {
        self.request_id = Some(request_id.into());
        self
    }

    /// Add query parameters to the request.
    pub fn with_query(mut self, query: ScimQuery) -> Self {
        self.query = Some(query);
        self
    }
}

impl ScimQuery {
    /// Create a new empty query.
    pub fn new() -> Self {
        Self {
            count: None,
            start_index: None,
            filter: None,
            attributes: None,
            excluded_attributes: None,
            search_attribute: None,
            search_value: None,
        }
    }

    /// Set pagination parameters.
    pub fn with_pagination(mut self, start_index: usize, count: usize) -> Self {
        self.start_index = Some(start_index);
        self.count = Some(count);
        self
    }

    /// Set filter expression.
    pub fn with_filter(mut self, filter: impl Into<String>) -> Self {
        self.filter = Some(filter.into());
        self
    }

    /// Set search parameters.
    pub fn with_search(mut self, attribute: impl Into<String>, value: Value) -> Self {
        self.search_attribute = Some(attribute.into());
        self.search_value = Some(value);
        self
    }

    /// Set attributes to include.
    pub fn with_attributes(mut self, attributes: Vec<String>) -> Self {
        self.attributes = Some(attributes);
        self
    }

    /// Set attributes to exclude.
    pub fn with_excluded_attributes(mut self, excluded_attributes: Vec<String>) -> Self {
        self.excluded_attributes = Some(excluded_attributes);
        self
    }
}

impl Default for ScimQuery {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::multi_tenant::ScimOperation;
    use crate::providers::InMemoryProvider;
    use crate::resource_handlers::create_user_resource_handler;
    use serde_json::json;

    #[tokio::test]
    async fn test_operation_handler_create() {
        let provider = InMemoryProvider::new();
        let mut server = ScimServer::new(provider).unwrap();

        // Register User resource type
        let user_schema = server
            .get_schema_by_id("urn:ietf:params:scim:schemas:core:2.0:User")
            .unwrap()
            .clone();
        let user_handler = create_user_resource_handler(user_schema);
        server
            .register_resource_type("User", user_handler, vec![ScimOperation::Create])
            .unwrap();

        let handler = ScimOperationHandler::new(server);

        let request = ScimOperationRequest::create(
            "User",
            json!({
                "userName": "testuser",
                "name": {
                    "givenName": "Test",
                    "familyName": "User"
                }
            }),
        );

        let response = handler.handle_operation(request).await;
        assert!(response.success);
        assert!(response.data.is_some());
        assert!(response.error.is_none());
    }

    #[tokio::test]
    async fn test_operation_handler_get_schemas() {
        let provider = InMemoryProvider::new();
        let server = ScimServer::new(provider).unwrap();
        let handler = ScimOperationHandler::new(server);

        let request = ScimOperationRequest::get_schemas();
        let response = handler.handle_operation(request).await;

        assert!(response.success);
        assert!(response.data.is_some());
        if let Some(data) = response.data {
            assert!(data.get("schemas").is_some());
        }
    }

    #[tokio::test]
    async fn test_operation_handler_error_handling() {
        let provider = InMemoryProvider::new();
        let server = ScimServer::new(provider).unwrap();
        let handler = ScimOperationHandler::new(server);

        // Try to get a non-existent resource
        let request = ScimOperationRequest::get("User", "non-existent-id");
        let response = handler.handle_operation(request).await;

        assert!(!response.success);
        assert!(response.error.is_some());
        assert!(response.error_code.is_some());
    }
}
