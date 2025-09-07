//! Resource CRUD operations for the SCIM server.
//!
//! This module contains all the async operations for creating, reading,
//! updating, deleting, listing, and searching resources through the
//! registered resource providers.

use super::core::ScimServer;
use crate::error::ScimResult;
use crate::providers::ResourceProvider;
use crate::resource::{RequestContext, Resource, ScimOperation};
use log::{debug, info, warn};
use serde_json::Value;

impl<P: ResourceProvider + Sync> ScimServer<P> {
    /// Generic create operation for any resource type
    pub async fn create_resource(
        &self,
        resource_type: &str,
        data: Value,
        context: &RequestContext,
    ) -> ScimResult<Resource> {
        info!(
            "SCIM create {} operation initiated (request: '{}')",
            resource_type, context.request_id
        );

        // Check if resource type is supported
        self.ensure_operation_supported(resource_type, &ScimOperation::Create)?;

        // Get the schema for validation
        let schema = self.get_schema_for_resource_type(resource_type)?;

        // Validate against schema
        self.schema_registry.validate_resource(&schema, &data)?;

        // Delegate to provider
        let result = self
            .provider
            .create_resource(resource_type, data, context)
            .await
            .map(|versioned_resource| versioned_resource.into_resource())
            .map_err(|e| crate::error::ScimError::ProviderError(e.to_string()));

        match &result {
            Ok(resource) => {
                info!(
                    "SCIM create {} operation completed successfully: ID '{}' (request: '{}')",
                    resource_type,
                    resource.get_id().unwrap_or("unknown"),
                    context.request_id
                );
            }
            Err(e) => {
                warn!(
                    "SCIM create {} operation failed: {} (request: '{}')",
                    resource_type, e, context.request_id
                );
            }
        }

        result
    }

    /// Create a resource and return its JSON representation with $ref fields.
    ///
    /// This is a convenience method that combines resource creation with
    /// proper $ref field injection for SCIM compliance.
    pub async fn create_resource_with_refs(
        &self,
        resource_type: &str,
        data: Value,
        context: &RequestContext,
    ) -> ScimResult<serde_json::Value> {
        let resource = self.create_resource(resource_type, data, context).await?;
        self.serialize_resource_with_refs(&resource, context.tenant_id())
            .map_err(|e| e.into())
    }

    /// Generic read operation
    pub async fn get_resource(
        &self,
        resource_type: &str,
        id: &str,
        context: &RequestContext,
    ) -> ScimResult<Option<Resource>> {
        debug!(
            "SCIM get {} operation initiated for ID '{}' (request: '{}')",
            resource_type, id, context.request_id
        );

        // Check if resource type is supported
        self.ensure_operation_supported(resource_type, &ScimOperation::Read)?;

        let result = self
            .provider
            .get_resource(resource_type, id, context)
            .await
            .map_err(|e| crate::error::ScimError::ProviderError(e.to_string()));

        match &result {
            Ok(Some(_)) => {
                debug!(
                    "SCIM get {} operation completed: found resource with ID '{}' (request: '{}')",
                    resource_type, id, context.request_id
                );
            }
            Ok(None) => {
                debug!(
                    "SCIM get {} operation completed: resource with ID '{}' not found (request: '{}')",
                    resource_type, id, context.request_id
                );
            }
            Err(e) => {
                warn!(
                    "SCIM get {} operation failed for ID '{}': {} (request: '{}')",
                    resource_type, id, e, context.request_id
                );
            }
        }

        result.map(|opt| opt.map(|vr| vr.into_resource()))
    }

    /// Generic update operation
    pub async fn update_resource(
        &self,
        resource_type: &str,
        id: &str,
        data: Value,
        context: &RequestContext,
    ) -> ScimResult<Resource> {
        info!(
            "SCIM update {} operation initiated for ID '{}' (request: '{}')",
            resource_type, id, context.request_id
        );

        // Check if resource type is supported
        self.ensure_operation_supported(resource_type, &ScimOperation::Update)?;

        // Get the schema for validation
        let schema = self.get_schema_for_resource_type(resource_type)?;

        // Validate against schema
        self.schema_registry.validate_resource(&schema, &data)?;

        let result = self
            .provider
            .update_resource(resource_type, id, data, None, context)
            .await
            .map(|versioned_resource| versioned_resource.into_resource())
            .map_err(|e| crate::error::ScimError::ProviderError(e.to_string()));

        match &result {
            Ok(_) => {
                info!(
                    "SCIM update {} operation completed successfully for ID '{}' (request: '{}')",
                    resource_type, id, context.request_id
                );
            }
            Err(e) => {
                warn!(
                    "SCIM update {} operation failed for ID '{}': {} (request: '{}')",
                    resource_type, id, e, context.request_id
                );
            }
        }

        result
    }

    /// Generic delete operation
    pub async fn delete_resource(
        &self,
        resource_type: &str,
        id: &str,
        context: &RequestContext,
    ) -> ScimResult<()> {
        info!(
            "SCIM delete {} operation initiated for ID '{}' (request: '{}')",
            resource_type, id, context.request_id
        );

        // Check if resource type is supported
        self.ensure_operation_supported(resource_type, &ScimOperation::Delete)?;

        let result = self
            .provider
            .delete_resource(resource_type, id, None, context)
            .await
            .map_err(|e| crate::error::ScimError::ProviderError(e.to_string()));

        match &result {
            Ok(_) => {
                info!(
                    "SCIM delete {} operation completed successfully for ID '{}' (request: '{}')",
                    resource_type, id, context.request_id
                );
            }
            Err(e) => {
                warn!(
                    "SCIM delete {} operation failed for ID '{}': {} (request: '{}')",
                    resource_type, id, e, context.request_id
                );
            }
        }

        result
    }

    /// Generic list operation for any resource type
    pub async fn list_resources(
        &self,
        resource_type: &str,
        context: &RequestContext,
    ) -> ScimResult<Vec<Resource>> {
        debug!(
            "SCIM list {} operation initiated (request: '{}')",
            resource_type, context.request_id
        );

        // Check if resource type is supported
        self.ensure_operation_supported(resource_type, &ScimOperation::List)?;

        let result = self
            .provider
            .list_resources(resource_type, None, context)
            .await
            .map(|versioned_resources| {
                versioned_resources
                    .into_iter()
                    .map(|vr| vr.into_resource())
                    .collect::<Vec<_>>()
            })
            .map_err(|e| crate::error::ScimError::internal(format!("Provider error: {}", e)));

        match &result {
            Ok(resources) => {
                debug!(
                    "SCIM list {} operation completed: found {} resources (request: '{}')",
                    resource_type,
                    resources.len(),
                    context.request_id
                );
            }
            Err(e) => {
                warn!(
                    "SCIM list {} operation failed: {} (request: '{}')",
                    resource_type, e, context.request_id
                );
            }
        }

        result
    }

    /// Generic search by attribute (replaces find_user_by_username)
    pub async fn find_resource_by_attribute(
        &self,
        resource_type: &str,
        attribute: &str,
        value: &Value,
        context: &RequestContext,
    ) -> ScimResult<Option<Resource>> {
        debug!(
            "SCIM find {} operation initiated for {}='{}' (request: '{}')",
            resource_type, attribute, value, context.request_id
        );

        // Check if resource type is supported
        self.ensure_operation_supported(resource_type, &ScimOperation::Search)?;

        let value_str = match value {
            serde_json::Value::String(s) => s.as_str(),
            _ => {
                return Err(crate::error::ScimError::InvalidRequest {
                    message: "Attribute value must be a string".to_string(),
                });
            }
        };

        let result = self
            .provider
            .find_resources_by_attribute(resource_type, attribute, value_str, context)
            .await
            .map(|versioned_resources| {
                versioned_resources
                    .into_iter()
                    .map(|vr| vr.into_resource())
                    .next() // Take first match for this API
            })
            .map_err(|e| crate::error::ScimError::ProviderError(e.to_string()));

        match &result {
            Ok(Some(resource)) => {
                debug!(
                    "SCIM find {} operation completed: found resource with ID '{}' (request: '{}')",
                    resource_type,
                    resource.get_id().unwrap_or("unknown"),
                    context.request_id
                );
            }
            Ok(None) => {
                debug!(
                    "SCIM find {} operation completed: no resource found for {}='{}' (request: '{}')",
                    resource_type, attribute, value, context.request_id
                );
            }
            Err(e) => {
                warn!(
                    "SCIM find {} operation failed for {}='{}': {} (request: '{}')",
                    resource_type, attribute, value, e, context.request_id
                );
            }
        }

        result
    }

    /// Check if a resource exists
    pub async fn resource_exists(
        &self,
        resource_type: &str,
        id: &str,
        context: &RequestContext,
    ) -> ScimResult<bool> {
        debug!(
            "SCIM resource exists check for {} with ID '{}' (request: '{}')",
            resource_type, id, context.request_id
        );
        self.provider
            .resource_exists(resource_type, id, context)
            .await
            .map_err(|e| crate::error::ScimError::ProviderError(e.to_string()))
    }

    /// Generic patch operation for any resource type
    pub async fn patch_resource(
        &self,
        resource_type: &str,
        id: &str,
        patch_request: &Value,
        context: &RequestContext,
    ) -> ScimResult<Resource> {
        info!(
            "SCIM patch {} operation initiated for ID '{}' (request: '{}')",
            resource_type, id, context.request_id
        );

        // Check if resource type is supported for patch operations
        self.ensure_operation_supported(resource_type, &ScimOperation::Patch)?;

        // Validate patch request structure
        let operations = patch_request
            .get("Operations")
            .and_then(|ops| ops.as_array())
            .ok_or_else(|| {
                crate::error::ScimError::invalid_request(
                    "PATCH request must contain Operations array".to_string(),
                )
            })?;

        // Validate that operations array is not empty
        if operations.is_empty() {
            return Err(crate::error::ScimError::invalid_request(
                "Invalid Operations array: cannot be empty".to_string(),
            ));
        }

        // Delegate to provider
        let result = self
            .provider
            .patch_resource(resource_type, id, &patch_request, None, context)
            .await
            .map(|versioned_resource| versioned_resource.into_resource())
            .map_err(|e| crate::error::ScimError::ProviderError(e.to_string()));

        match &result {
            Ok(resource) => {
                info!(
                    "SCIM patch {} operation completed successfully for ID '{}' (request: '{}')",
                    resource_type,
                    resource.get_id().unwrap_or("unknown"),
                    context.request_id
                );
            }
            Err(e) => {
                warn!(
                    "SCIM patch {} operation failed for ID '{}': {} (request: '{}')",
                    resource_type, id, e, context.request_id
                );
            }
        }

        result
    }
}
