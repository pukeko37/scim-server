//! Schema management operations for the SCIM server.
//!
//! This module contains all schema-related functionality including
//! schema retrieval, validation helpers, and schema registry access.

use super::core::ScimServer;
use crate::error::ScimResult;
use crate::providers::ResourceProvider;
use crate::schema::Schema;

impl<P: ResourceProvider> ScimServer<P> {
    /// Get schema for any registered resource type
    pub fn get_resource_schema(&self, resource_type: &str) -> ScimResult<Schema> {
        let handler = self.get_handler(resource_type)?;
        Ok(handler.schema.clone())
    }

    /// Get all registered schemas
    pub fn get_all_schemas(&self) -> Vec<&Schema> {
        self.resource_handlers
            .values()
            .map(|handler| &handler.schema)
            .collect()
    }

    /// Get schema from schema registry by ID
    pub fn get_schema_by_id(&self, schema_id: &str) -> Option<&Schema> {
        self.schema_registry.get_schema(schema_id)
    }
}
