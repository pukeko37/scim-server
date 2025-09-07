//! Resource type registration and operation support management.
//!
//! This module handles the registration of resource types with their handlers
//! and supported operations, as well as validation of operation support.

use super::core::ScimServer;
use crate::error::{ScimError, ScimResult};
use crate::providers::ResourceProvider;
use crate::resource::{ResourceHandler, ScimOperation};
use crate::schema::Schema;
use std::sync::Arc;

impl<P: ResourceProvider> ScimServer<P> {
    /// Register a resource type with its handler and supported operations
    pub fn register_resource_type(
        &mut self,
        resource_type: &str,
        handler: ResourceHandler,
        operations: Vec<ScimOperation>,
    ) -> Result<(), ScimError> {
        // Register the schema
        self.schema_registry
            .add_schema(handler.schema.clone())
            .map_err(|e| ScimError::internal(format!("Failed to add schema: {}", e)))?;

        // Register the handler
        self.resource_handlers
            .insert(resource_type.to_string(), Arc::new(handler));

        // Register supported operations
        self.supported_operations
            .insert(resource_type.to_string(), operations);

        Ok(())
    }

    /// Get all registered resource types
    pub fn get_supported_resource_types(&self) -> Vec<&str> {
        self.resource_handlers.keys().map(|s| s.as_str()).collect()
    }

    /// Get supported operations for a resource type
    pub fn get_supported_operations(&self, resource_type: &str) -> Option<&Vec<ScimOperation>> {
        self.supported_operations.get(resource_type)
    }

    /// Helper method to ensure operation is supported for a resource type
    pub(super) fn ensure_operation_supported(
        &self,
        resource_type: &str,
        operation: &ScimOperation,
    ) -> ScimResult<()> {
        let operations = self
            .supported_operations
            .get(resource_type)
            .ok_or_else(|| ScimError::UnsupportedResourceType(resource_type.to_string()))?;

        if !operations.contains(operation) {
            return Err(ScimError::UnsupportedOperation {
                resource_type: resource_type.to_string(),
                operation: format!("{:?}", operation),
            });
        }

        Ok(())
    }

    /// Helper method to get handler for a resource type
    pub(super) fn get_handler(&self, resource_type: &str) -> ScimResult<Arc<ResourceHandler>> {
        self.resource_handlers
            .get(resource_type)
            .cloned()
            .ok_or_else(|| ScimError::UnsupportedResourceType(resource_type.to_string()))
    }

    /// Helper method to get schema for a resource type
    pub(super) fn get_schema_for_resource_type(&self, resource_type: &str) -> ScimResult<Schema> {
        let handler = self.get_handler(resource_type)?;
        Ok(handler.schema.clone())
    }
}
