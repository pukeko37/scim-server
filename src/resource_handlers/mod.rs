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
//! - `tests` - Test infrastructure and comprehensive test cases
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
    fn test_user_handler_functionality() {
        // Test that handlers contain the expected schema
        let registry = SchemaRegistry::new().expect("Failed to create registry");

        let user_schema = registry.get_user_schema().clone();
        let user_handler = create_user_resource_handler(user_schema.clone());

        // Verify handler has the correct schema
        assert_eq!(user_handler.schema.id, user_schema.id);
        assert_eq!(user_handler.schema.name, user_schema.name);

        let group_schema = registry.get_group_schema().clone();
        let group_handler = create_group_resource_handler(group_schema.clone());

        assert_eq!(group_handler.schema.id, group_schema.id);
        assert_eq!(group_handler.schema.name, group_schema.name);
    }
}
