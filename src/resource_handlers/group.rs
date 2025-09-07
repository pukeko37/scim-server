//! Group resource handler implementation.
//!
//! This module provides a factory function for creating Group resource handlers
//! that contain schema information for Group resources.

use crate::resource::SchemaResourceBuilder;
use crate::schema::Schema;

/// Create a Group resource handler with the provided schema.
///
/// This handler contains the schema definition for Group resources,
/// which can be used for validation and other schema-driven operations.
pub fn create_group_resource_handler(group_schema: Schema) -> crate::resource::ResourceHandler {
    SchemaResourceBuilder::new(group_schema).build()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::registry::SchemaRegistry;

    #[test]
    fn test_group_handler_creation() {
        let registry = SchemaRegistry::new().expect("Failed to create schema registry");
        let group_schema = registry.get_group_schema();

        let handler = create_group_resource_handler(group_schema.clone());

        assert_eq!(handler.schema.id, group_schema.id);
        assert_eq!(handler.schema.name, group_schema.name);
    }
}
