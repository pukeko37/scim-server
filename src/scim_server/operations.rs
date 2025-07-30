//! Resource CRUD operations for the SCIM server.
//!
//! This module contains all the async operations for creating, reading,
//! updating, deleting, listing, and searching resources through the
//! registered resource providers.

use super::core::ScimServer;
use crate::error::ScimResult;
use crate::resource::{RequestContext, Resource, ResourceProvider, ScimOperation};
use serde_json::Value;

impl<P: ResourceProvider> ScimServer<P> {
    /// Generic create operation for any resource type
    pub async fn create_resource(
        &self,
        resource_type: &str,
        data: Value,
        context: &RequestContext,
    ) -> ScimResult<Resource> {
        // Check if resource type is supported
        self.ensure_operation_supported(resource_type, &ScimOperation::Create)?;

        // Get the schema for validation
        let schema = self.get_schema_for_resource_type(resource_type)?;

        // Validate against schema
        self.schema_registry.validate_resource(&schema, &data)?;

        // Delegate to provider
        self.provider
            .create_resource(resource_type, data, context)
            .await
            .map_err(|e| crate::error::ScimError::ProviderError(e.to_string()))
    }

    /// Generic read operation
    pub async fn get_resource(
        &self,
        resource_type: &str,
        id: &str,
        context: &RequestContext,
    ) -> ScimResult<Option<Resource>> {
        // Check if resource type is supported
        self.ensure_operation_supported(resource_type, &ScimOperation::Read)?;

        self.provider
            .get_resource(resource_type, id, context)
            .await
            .map_err(|e| crate::error::ScimError::ProviderError(e.to_string()))
    }

    /// Generic update operation
    pub async fn update_resource(
        &self,
        resource_type: &str,
        id: &str,
        data: Value,
        context: &RequestContext,
    ) -> ScimResult<Resource> {
        self.ensure_operation_supported(resource_type, &ScimOperation::Update)?;

        // Get the schema for validation
        let schema = self.get_schema_for_resource_type(resource_type)?;

        // Validate against schema
        self.schema_registry.validate_resource(&schema, &data)?;

        self.provider
            .update_resource(resource_type, id, data, context)
            .await
            .map_err(|e| crate::error::ScimError::ProviderError(e.to_string()))
    }

    /// Generic delete operation
    pub async fn delete_resource(
        &self,
        resource_type: &str,
        id: &str,
        context: &RequestContext,
    ) -> ScimResult<()> {
        self.ensure_operation_supported(resource_type, &ScimOperation::Delete)?;

        self.provider
            .delete_resource(resource_type, id, context)
            .await
            .map_err(|e| crate::error::ScimError::ProviderError(e.to_string()))
    }

    /// Generic list operation for any resource type
    pub async fn list_resources(
        &self,
        resource_type: &str,
        context: &RequestContext,
    ) -> ScimResult<Vec<Resource>> {
        // Check if resource type is supported
        self.ensure_operation_supported(resource_type, &ScimOperation::List)?;

        self.provider
            .list_resources(resource_type, None, context)
            .await
            .map_err(|e| crate::error::ScimError::internal(format!("Provider error: {}", e)))
    }

    /// Generic search by attribute (replaces find_user_by_username)
    pub async fn find_resource_by_attribute(
        &self,
        resource_type: &str,
        attribute: &str,
        value: &Value,
        context: &RequestContext,
    ) -> ScimResult<Option<Resource>> {
        // Check if resource type is supported
        self.ensure_operation_supported(resource_type, &ScimOperation::Search)?;

        self.provider
            .find_resource_by_attribute(resource_type, attribute, value, context)
            .await
            .map_err(|e| crate::error::ScimError::ProviderError(e.to_string()))
    }

    /// Check if a resource exists
    pub async fn resource_exists(
        &self,
        resource_type: &str,
        id: &str,
        context: &RequestContext,
    ) -> ScimResult<bool> {
        self.provider
            .resource_exists(resource_type, id, context)
            .await
            .map_err(|e| crate::error::ScimError::ProviderError(e.to_string()))
    }
}
