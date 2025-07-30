//! Test infrastructure and test cases for resource handlers.
//!
//! This module contains all test-related code including test schema creation
//! and comprehensive test cases for the User and Group resource handlers.

#[cfg(test)]
use super::*;
#[cfg(test)]
use crate::resource::DynamicResource;
#[cfg(test)]
use crate::schema::{AttributeDefinition, AttributeType, Mutability, Schema, Uniqueness};
#[cfg(test)]
use serde_json::{Value, json};
#[cfg(test)]
use std::sync::Arc;

#[cfg(test)]
fn create_test_user_schema() -> Schema {
    Schema {
        id: "urn:ietf:params:scim:schemas:core:2.0:User".to_string(),
        name: "User".to_string(),
        description: "User Account".to_string(),
        attributes: vec![
            AttributeDefinition {
                name: "id".to_string(),
                data_type: AttributeType::String,
                required: false,
                mutability: Mutability::ReadOnly,
                uniqueness: Uniqueness::Server,
                ..Default::default()
            },
            AttributeDefinition {
                name: "userName".to_string(),
                data_type: AttributeType::String,
                required: true,
                mutability: Mutability::ReadWrite,
                uniqueness: Uniqueness::Server,
                ..Default::default()
            },
            AttributeDefinition {
                name: "displayName".to_string(),
                data_type: AttributeType::String,
                required: false,
                mutability: Mutability::ReadWrite,
                ..Default::default()
            },
            AttributeDefinition {
                name: "emails".to_string(),
                data_type: AttributeType::Complex,
                multi_valued: true,
                required: false,
                mutability: Mutability::ReadWrite,
                ..Default::default()
            },
            AttributeDefinition {
                name: "active".to_string(),
                data_type: AttributeType::Boolean,
                required: false,
                mutability: Mutability::ReadWrite,
                ..Default::default()
            },
        ],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_user_handler_creation() {
        let schema = create_test_user_schema();
        let handler = user::create_user_resource_handler(schema.clone());

        assert_eq!(handler.schema.id, schema.id);
        assert_eq!(handler.schema.name, "User");
        assert!(!handler.handlers.is_empty());
        assert!(!handler.custom_methods.is_empty());
        assert!(!handler.mappers.is_empty());
    }

    #[test]
    fn test_user_resource_dynamic_operations() {
        let schema = create_test_user_schema();
        let handler = Arc::new(user::create_user_resource_handler(schema));

        let user_data = json!({
            "userName": "testuser",
            "displayName": "Test User",
            "emails": [
                {
                    "value": "test@example.com",
                    "type": "work",
                    "primary": true
                }
            ],
            "active": true
        });

        let resource = DynamicResource::new("User".to_string(), user_data, handler);

        // Test dynamic attribute access
        assert_eq!(
            resource.get_attribute_dynamic("userName"),
            Some(Value::String("testuser".to_string()))
        );

        // Test custom methods
        let username = resource.call_custom_method("get_username").unwrap();
        assert_eq!(username, Value::String("testuser".to_string()));

        let primary_email = resource.call_custom_method("get_primary_email").unwrap();
        assert_eq!(primary_email, Value::String("test@example.com".to_string()));

        let is_active = resource.call_custom_method("is_active").unwrap();
        assert_eq!(is_active, Value::Bool(true));
    }

    #[test]
    fn test_database_mapping() {
        let schema = create_test_user_schema();
        let handler = Arc::new(user::create_user_resource_handler(schema));

        let user_data = json!({
            "userName": "testuser",
            "displayName": "Test User",
            "id": "123"
        });

        let resource = DynamicResource::new("User".to_string(), user_data, handler);

        // Test mapping to database schema
        let db_data = resource.to_implementation_schema(0).unwrap();

        assert_eq!(
            db_data.get("username"),
            Some(&Value::String("testuser".to_string()))
        );
        assert_eq!(
            db_data.get("full_name"),
            Some(&Value::String("Test User".to_string()))
        );
        assert_eq!(
            db_data.get("user_id"),
            Some(&Value::String("123".to_string()))
        );
    }
}
