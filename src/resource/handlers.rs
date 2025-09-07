//! Handler infrastructure for dynamic resource operations.
//!
//! This module provides the infrastructure for creating and managing
//! dynamic resource handlers that can be configured at runtime with
//! custom attribute handlers, mappers, and methods.

use crate::schema::Schema;

/// Handler for a specific resource type containing its schema
#[derive(Clone)]
pub struct ResourceHandler {
    pub schema: Schema,
}

impl std::fmt::Debug for ResourceHandler {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ResourceHandler")
            .field("schema", &self.schema)
            .finish()
    }
}

/// Builder for creating resource handlers
pub struct SchemaResourceBuilder {
    schema: Schema,
}

impl SchemaResourceBuilder {
    pub fn new(schema: Schema) -> Self {
        Self { schema }
    }

    pub fn build(self) -> ResourceHandler {
        ResourceHandler {
            schema: self.schema,
        }
    }
}
