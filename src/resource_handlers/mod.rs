//! Resource handler implementations using the dynamic schema approach.
//!
//! This module provides factory functions for creating resource handlers that replace
//! hard-coded resource methods with dynamic, schema-driven operations. The handlers
//! support comprehensive attribute management, custom methods, and database mapping.
//!
//! # Module Organization
//!
//! * [`user`] - User resource handler with comprehensive attribute and method support
//! * [`group`] - Group resource handler for group management operations
//! * [`tests`] - Test infrastructure and comprehensive test cases
//!
//! # Usage
//!
//! ```rust
//! use scim_server::resource_handlers::{create_user_resource_handler, create_group_resource_handler};
//! use scim_server::schema::SchemaRegistry;
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! // Create schema registry
//! let registry = SchemaRegistry::new()?;
//!
//! // Create User resource handler
//! let user_schema = registry.get_user_schema().clone();
//! let user_handler = create_user_resource_handler(user_schema);
//!
//! // Create Group resource handler
//! let group_schema = registry.get_group_schema().clone();
//! let group_handler = create_group_resource_handler(group_schema);
//! # Ok(())
//! # }
//! ```

pub mod group;
pub mod user;

#[cfg(test)]
pub mod tests;

// Re-export the main factory functions to maintain API compatibility
pub use group::create_group_resource_handler;
pub use user::create_user_resource_handler;

#[cfg(test)]
mod integration_tests {
    use super::*;
    use crate::schema::SchemaRegistry;

    #[test]
    fn test_module_integration() {
        let registry = SchemaRegistry::new().expect("Failed to create registry");

        // Test that both handlers can be created
        let user_schema = registry.get_user_schema().clone();
        let user_handler = create_user_resource_handler(user_schema);
        assert_eq!(user_handler.schema.name, "User");

        let group_schema = registry.get_group_schema().clone();
        let group_handler = create_group_resource_handler(group_schema);
        assert_eq!(group_handler.schema.name, "Group");
    }

    #[test]
    fn test_handler_compatibility() {
        // Test that handlers maintain expected interface
        let registry = SchemaRegistry::new().expect("Failed to create registry");

        let user_schema = registry.get_user_schema().clone();
        let user_handler = create_user_resource_handler(user_schema);

        // Verify handler has expected components
        assert!(
            !user_handler.handlers.is_empty(),
            "User handler should have attribute handlers"
        );
        assert!(
            !user_handler.custom_methods.is_empty(),
            "User handler should have custom methods"
        );
        assert!(
            !user_handler.mappers.is_empty(),
            "User handler should have database mappers"
        );

        let group_schema = registry.get_group_schema().clone();
        let group_handler = create_group_resource_handler(group_schema);

        assert!(
            !group_handler.handlers.is_empty(),
            "Group handler should have attribute handlers"
        );
        assert!(
            !group_handler.custom_methods.is_empty(),
            "Group handler should have custom methods"
        );
        assert!(
            !group_handler.mappers.is_empty(),
            "Group handler should have database mappers"
        );
    }
}
