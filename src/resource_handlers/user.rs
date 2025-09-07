//! User resource handler implementation.
//!
//! This module provides a factory function for creating User resource handlers
//! that contain schema information for User resources.

use crate::resource::SchemaResourceBuilder;
use crate::schema::Schema;

/// Create a User resource handler with the provided schema.
///
/// This handler contains the schema definition for User resources,
/// which can be used for validation and other schema-driven operations.
pub fn create_user_resource_handler(user_schema: Schema) -> crate::resource::ResourceHandler {
    SchemaResourceBuilder::new(user_schema).build()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::registry::SchemaRegistry;

    #[test]
    fn test_user_handler_creation() {
        let registry = SchemaRegistry::new().expect("Failed to create schema registry");
        let user_schema = registry.get_user_schema();

        let handler = create_user_resource_handler(user_schema.clone());

        assert_eq!(handler.schema.id, user_schema.id);
        assert_eq!(handler.schema.name, user_schema.name);
    }
}
