//! Test infrastructure and test cases for resource handlers.
//!
//! This module contains test cases for the simplified resource handlers
//! that focus on schema containment rather than complex dynamic behavior.

#[cfg(test)]
use super::*;
#[cfg(test)]
use crate::schema::registry::SchemaRegistry;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_user_handler_schema() {
        let registry = SchemaRegistry::new().expect("Failed to create registry");
        let user_schema = registry.get_user_schema();

        let handler = create_user_resource_handler(user_schema.clone());

        // Verify the handler contains the correct schema
        assert_eq!(handler.schema.id, user_schema.id);
        assert_eq!(handler.schema.name, "User");
        assert!(!handler.schema.attributes.is_empty());
    }

    #[test]
    fn test_group_handler_schema() {
        let registry = SchemaRegistry::new().expect("Failed to create registry");
        let group_schema = registry.get_group_schema();

        let handler = create_group_resource_handler(group_schema.clone());

        // Verify the handler contains the correct schema
        assert_eq!(handler.schema.id, group_schema.id);
        assert_eq!(handler.schema.name, "Group");
        assert!(!handler.schema.attributes.is_empty());
    }

    #[test]
    fn test_handler_debug_format() {
        let registry = SchemaRegistry::new().expect("Failed to create registry");
        let user_schema = registry.get_user_schema();
        let handler = create_user_resource_handler(user_schema.clone());

        // Verify debug formatting works
        let debug_str = format!("{:?}", handler);
        assert!(debug_str.contains("ResourceHandler"));
        assert!(debug_str.contains("schema"));
    }
}
