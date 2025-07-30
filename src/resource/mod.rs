//! Resource model and provider trait for SCIM resources.
//!
//! This module defines the core resource abstractions that users implement
//! to provide data access for SCIM operations. The design emphasizes
//! type safety and async patterns while keeping the interface simple.
//!
//! # Module Organization
//!
//! * [`core`] - Core types like `Resource`, `RequestContext`, `ScimOperation`, and `ListQuery`
//! * [`types`] - Domain-specific data structures like `EmailAddress`
//! * [`mapper`] - Schema mapping functionality for converting between formats
//! * [`handlers`] - Dynamic handler infrastructure for runtime resource operations
//! * [`provider`] - The main `ResourceProvider` trait for data access

pub mod core;
pub mod handlers;
pub mod mapper;
pub mod provider;
pub mod types;

// Re-export all public types to maintain API compatibility
pub use core::{ListQuery, RequestContext, Resource, ScimOperation};
pub use handlers::{AttributeHandler, DynamicResource, ResourceHandler, SchemaResourceBuilder};
pub use mapper::{DatabaseMapper, SchemaMapper};
pub use provider::ResourceProvider;
pub use types::EmailAddress;

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_resource_creation() {
        let data = json!({
            "userName": "testuser",
            "displayName": "Test User"
        });
        let resource = Resource::new("User".to_string(), data);

        assert_eq!(resource.resource_type, "User");
        assert_eq!(resource.get_username(), Some("testuser"));
    }

    #[test]
    fn test_resource_id_extraction() {
        let data = json!({
            "id": "12345",
            "userName": "testuser"
        });
        let resource = Resource::new("User".to_string(), data);

        assert_eq!(resource.get_id(), Some("12345"));
    }

    #[test]
    fn test_resource_schemas() {
        let data = json!({
            "userName": "testuser"
        });
        let resource = Resource::new("User".to_string(), data);

        let schemas = resource.get_schemas();
        assert_eq!(schemas.len(), 1);
        assert_eq!(schemas[0], "urn:ietf:params:scim:schemas:core:2.0:User");
    }

    #[test]
    fn test_email_extraction() {
        let data = json!({
            "userName": "testuser",
            "emails": [
                {
                    "value": "test@example.com",
                    "type": "work",
                    "primary": true
                }
            ]
        });
        let resource = Resource::new("User".to_string(), data);

        let emails = resource.get_emails();
        assert_eq!(emails.len(), 1);
        assert_eq!(emails[0].value, "test@example.com");
        assert_eq!(emails[0].email_type, Some("work".to_string()));
        assert_eq!(emails[0].primary, Some(true));
    }

    #[test]
    fn test_request_context_creation() {
        let context = RequestContext::new("test-request".to_string());
        assert!(!context.request_id.is_empty());

        let context_with_id = RequestContext::new("test-123".to_string());
        assert_eq!(context_with_id.request_id, "test-123");
    }

    #[test]
    fn test_resource_active_status() {
        let active_data = json!({
            "userName": "testuser",
            "active": true
        });
        let active_resource = Resource::new("User".to_string(), active_data);
        assert!(active_resource.is_active());

        let inactive_data = json!({
            "userName": "testuser",
            "active": false
        });
        let inactive_resource = Resource::new("User".to_string(), inactive_data);
        assert!(!inactive_resource.is_active());

        let no_active_data = json!({
            "userName": "testuser"
        });
        let default_resource = Resource::new("User".to_string(), no_active_data);
        assert!(default_resource.is_active()); // Default to true
    }
}
