//! Dynamic SCIM server implementation with runtime resource type registration.
//!
//! This module provides a completely dynamic SCIM server that can handle any
//! resource type registered at runtime, eliminating hard-coded resource types
//! and enabling true schema-driven operations.
//!
//! # Module Organization
//!
//! * [`core`] - Core ScimServer struct and initialization
//! * [`registration`] - Resource type registration and operation support management
//! * [`operations`] - CRUD operations for resources (create, read, update, delete, list, search)
//! * [`schema_management`] - Schema-related operations and validation helpers
//! * [`tests`] - Test infrastructure and comprehensive test cases

pub mod core;
pub mod operations;
pub mod registration;
pub mod schema_management;

#[cfg(test)]
pub mod tests;

// Re-export the main ScimServer type to maintain API compatibility
pub use core::ScimServer;

#[cfg(test)]
mod integration_tests {
    use super::*;
    use crate::resource::{RequestContext, ScimOperation};
    use serde_json::json;

    #[tokio::test]
    async fn test_module_integration() {
        let provider = tests::TestProvider::new();
        let mut server = ScimServer::new(provider).expect("Failed to create server");

        // Test that all modules work together
        let user_schema = tests::create_test_user_schema();
        let user_handler = tests::create_user_resource_handler(user_schema);

        // Registration module
        server
            .register_resource_type(
                "User",
                user_handler,
                vec![ScimOperation::Create, ScimOperation::Read],
            )
            .expect("Registration should work");

        let context = RequestContext::new("integration-test".to_string());

        // Operations module
        let user_data = json!({"userName": "integration_user"});
        let created = server
            .create_resource("User", user_data, &context)
            .await
            .expect("Create operation should work");

        assert_eq!(created.resource_type, "User");
        assert_eq!(created.get_username(), Some("integration_user"));

        // Schema management module
        let schemas = server.get_all_schemas();
        assert!(!schemas.is_empty(), "Should have registered schemas");

        let user_schema = server.get_resource_schema("User");
        assert!(user_schema.is_ok(), "Should be able to get User schema");
    }
}
