//! Request builder utilities for ScimOperationRequest
//!
//! This module provides convenient builder methods for constructing
//! ScimOperationRequest instances for different operation types.

use crate::{
    operation_handler::core::{ScimOperationRequest, ScimOperationType, ScimQuery},
    resource::{TenantContext, version::RawVersion},
};
use serde_json::Value;

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
            expected_version: None,
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
            expected_version: None,
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
            expected_version: None,
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
            expected_version: None,
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
            expected_version: None,
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
            expected_version: None,
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
            expected_version: None,
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
            expected_version: None,
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
            expected_version: None,
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

    /// Add expected version for conditional operations.
    ///
    /// This enables ETag-based concurrency control for update and delete operations.
    /// The operation will only succeed if the current resource version matches
    /// the expected version.
    ///
    /// # Examples
    /// ```rust
    /// use scim_server::operation_handler::ScimOperationRequest;
    /// use scim_server::resource::version::{RawVersion, HttpVersion};
    /// use serde_json::json;
    ///
    /// let version: HttpVersion = "\"abc123\"".parse().unwrap();
    /// let request = ScimOperationRequest::update(
    ///     "User",
    ///     "123",
    ///     json!({"userName": "updated.name"})
    /// ).with_expected_version(version);
    /// ```
    pub fn with_expected_version(mut self, version: impl Into<RawVersion>) -> Self {
        self.expected_version = Some(version.into());

        self
    }
}
