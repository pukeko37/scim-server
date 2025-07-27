//! User resource handler implementation using the dynamic schema approach.
//!
//! This module provides a factory function for creating User resource handlers
//! that replace the hard-coded User methods with dynamic, schema-driven operations.

use crate::resource::SchemaResourceBuilder;
use crate::schema::Schema;
use serde_json::{Value, json};
use std::collections::HashMap;

/// Create a User resource handler with all the functionality of the original hard-coded implementation
pub fn create_user_resource_handler(user_schema: Schema) -> crate::resource::ResourceHandler {
    SchemaResourceBuilder::new(user_schema)
        // ID attribute handling
        .with_getter("id", |data| {
            data.get("id")?.as_str().map(|s| Value::String(s.to_string()))
        })
        .with_setter("id", |data, value| {
            if let Some(obj) = data.as_object_mut() {
                obj.insert("id".to_string(), value);
            }
            Ok(())
        })

        // userName attribute handling
        .with_getter("userName", |data| {
            data.get("userName")?.as_str().map(|s| Value::String(s.to_string()))
        })
        .with_setter("userName", |data, value| {
            if let Some(obj) = data.as_object_mut() {
                obj.insert("userName".to_string(), value);
            }
            Ok(())
        })

        // displayName attribute handling
        .with_getter("displayName", |data| {
            data.get("displayName")?.as_str().map(|s| Value::String(s.to_string()))
        })
        .with_setter("displayName", |data, value| {
            if let Some(obj) = data.as_object_mut() {
                obj.insert("displayName".to_string(), value);
            }
            Ok(())
        })

        // active attribute with default behavior
        .with_getter("active", |data| {
            Some(Value::Bool(
                data.get("active")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(true)
            ))
        })
        .with_setter("active", |data, value| {
            if let Some(obj) = data.as_object_mut() {
                obj.insert("active".to_string(), value);
            }
            Ok(())
        })

        // emails complex attribute handling
        .with_getter("emails", |data| {
            data.get("emails").cloned()
        })
        .with_setter("emails", |data, value| {
            if let Some(obj) = data.as_object_mut() {
                obj.insert("emails".to_string(), value);
            }
            Ok(())
        })

        // emails transformer for structured access
        .with_transformer("emails", |data, operation| {
            match operation {
                "get_structured" => {
                    data.get("emails")
                        .and_then(|v| v.as_array())
                        .map(|arr| {
                            let emails: Vec<Value> = arr.iter()
                                .filter_map(|email| {
                                    let value = email.get("value")?.as_str()?;
                                    Some(json!({
                                        "value": value,
                                        "type": email.get("type").and_then(|t| t.as_str()).unwrap_or(""),
                                        "primary": email.get("primary").and_then(|p| p.as_bool()).unwrap_or(false),
                                        "display": email.get("display").and_then(|d| d.as_str()).unwrap_or("")
                                    }))
                                })
                                .collect();
                            Value::Array(emails)
                        })
                }
                "get_primary" => {
                    data.get("emails")
                        .and_then(|v| v.as_array())
                        .and_then(|arr| {
                            arr.iter().find(|email| {
                                email.get("primary").and_then(|p| p.as_bool()).unwrap_or(false)
                            })
                        })
                        .and_then(|email| email.get("value"))
                        .cloned()
                }
                _ => None
            }
        })

        // name complex attribute handling
        .with_getter("name", |data| {
            data.get("name").cloned()
        })
        .with_setter("name", |data, value| {
            if let Some(obj) = data.as_object_mut() {
                obj.insert("name".to_string(), value);
            }
            Ok(())
        })

        // name transformer for structured access
        .with_transformer("name", |data, operation| {
            match operation {
                "get_formatted" => {
                    data.get("name")
                        .and_then(|name| name.get("formatted"))
                        .cloned()
                }
                "get_family_name" => {
                    data.get("name")
                        .and_then(|name| name.get("familyName"))
                        .cloned()
                }
                "get_given_name" => {
                    data.get("name")
                        .and_then(|name| name.get("givenName"))
                        .cloned()
                }
                _ => None
            }
        })

        // schemas attribute handling
        .with_getter("schemas", |data| {
            data.get("schemas").cloned().or_else(|| {
                // Default to User schema if not present
                Some(Value::Array(vec![
                    Value::String("urn:ietf:params:scim:schemas:core:2.0:User".to_string())
                ]))
            })
        })
        .with_setter("schemas", |data, value| {
            if let Some(obj) = data.as_object_mut() {
                obj.insert("schemas".to_string(), value);
            }
            Ok(())
        })

        // Custom methods that replace the old hard-coded methods
        .with_custom_method("get_username", |resource| {
            Ok(resource.get_attribute_dynamic("userName")
                .unwrap_or(Value::Null))
        })

        .with_custom_method("get_id", |resource| {
            Ok(resource.get_attribute_dynamic("id")
                .unwrap_or(Value::Null))
        })

        .with_custom_method("is_active", |resource| {
            Ok(resource.get_attribute_dynamic("active")
                .unwrap_or(Value::Bool(true)))
        })

        .with_custom_method("get_emails", |resource| {
            Ok(resource.get_attribute_dynamic("emails")
                .unwrap_or(Value::Array(vec![])))
        })

        .with_custom_method("get_primary_email", |resource| {
            let emails = resource.get_attribute_dynamic("emails")
                .and_then(|v| v.as_array().cloned())
                .unwrap_or_default();

            for email in emails {
                if email.get("primary").and_then(|p| p.as_bool()).unwrap_or(false) {
                    if let Some(value) = email.get("value") {
                        return Ok(value.clone());
                    }
                }
            }
            Ok(Value::Null)
        })

        .with_custom_method("get_display_name", |resource| {
            Ok(resource.get_attribute_dynamic("displayName")
                .unwrap_or(Value::Null))
        })

        .with_custom_method("get_formatted_name", |resource| {
            if let Some(name) = resource.get_attribute_dynamic("name") {
                if let Some(formatted) = name.get("formatted") {
                    return Ok(formatted.clone());
                }
            }
            Ok(Value::Null)
        })

        .with_custom_method("get_schemas", |resource| {
            Ok(resource.get_attribute_dynamic("schemas")
                .unwrap_or_else(|| Value::Array(vec![
                    Value::String("urn:ietf:params:scim:schemas:core:2.0:User".to_string())
                ])))
        })

        .with_custom_method("add_metadata", |resource| {
            // This would typically receive parameters from context
            let base_url = "https://example.com/scim"; // Would come from server config
            let now = chrono::Utc::now().to_rfc3339();

            let meta = json!({
                "resourceType": resource.resource_type,
                "created": now,
                "lastModified": now,
                "location": format!("{}/{}s/{}",
                    base_url,
                    resource.resource_type,
                    resource.get_attribute_dynamic("id")
                        .and_then(|v| v.as_str().map(|s| s.to_string()))
                        .unwrap_or_else(|| "".to_string())
                ),
                "version": format!("W/\"{}-{}\"",
                    resource.get_attribute_dynamic("id")
                        .and_then(|v| v.as_str().map(|s| s.to_string()))
                        .unwrap_or_else(|| "".to_string()),
                    now
                )
            });

            Ok(meta)
        })

        .with_custom_method("validate_email_format", |resource| {
            let emails = resource.get_attribute_dynamic("emails")
                .and_then(|v| v.as_array().cloned())
                .unwrap_or_default();

            for email in &emails {
                if let Some(email_value) = email.get("value").and_then(|v| v.as_str()) {
                    if !email_value.contains('@') {
                        return Ok(Value::Bool(false));
                    }
                }
            }
            Ok(Value::Bool(true))
        })

        .with_custom_method("get_work_email", |resource| {
            let emails = resource.get_attribute_dynamic("emails")
                .and_then(|v| v.as_array().cloned())
                .unwrap_or_default();

            for email in emails {
                if email.get("type").and_then(|t| t.as_str()) == Some("work") {
                    if let Some(value) = email.get("value") {
                        return Ok(value.clone());
                    }
                }
            }
            Ok(Value::Null)
        })

        .with_custom_method("set_primary_email", |_resource| {
            // This is a demonstration of how you might implement a setter method
            // In practice, this would need additional parameters
            Ok(Value::Bool(true)) // Placeholder return
        })

        // Database mapping for converting between SCIM and database schemas
        .with_database_mapping("users", {
            let mut mappings = HashMap::new();
            mappings.insert("userName".to_string(), "username".to_string());
            mappings.insert("displayName".to_string(), "full_name".to_string());
            mappings.insert("active".to_string(), "is_active".to_string());
            mappings.insert("emails".to_string(), "email_addresses".to_string());
            mappings.insert("id".to_string(), "user_id".to_string());
            mappings.insert("name".to_string(), "name_data".to_string());
            mappings.insert("schemas".to_string(), "scim_schemas".to_string());
            mappings
        })

        .build()
}

/// Create a Group resource handler (example of how to add new resource types)
pub fn create_group_resource_handler(group_schema: Schema) -> crate::resource::ResourceHandler {
    SchemaResourceBuilder::new(group_schema)
        .with_getter("displayName", |data| {
            data.get("displayName")?
                .as_str()
                .map(|s| Value::String(s.to_string()))
        })
        .with_setter("displayName", |data, value| {
            if let Some(obj) = data.as_object_mut() {
                obj.insert("displayName".to_string(), value);
            }
            Ok(())
        })
        .with_getter("members", |data| data.get("members").cloned())
        .with_setter("members", |data, value| {
            if let Some(obj) = data.as_object_mut() {
                obj.insert("members".to_string(), value);
            }
            Ok(())
        })
        .with_custom_method("get_display_name", |resource| {
            Ok(resource
                .get_attribute_dynamic("displayName")
                .unwrap_or(Value::Null))
        })
        .with_custom_method("get_members", |resource| {
            Ok(resource
                .get_attribute_dynamic("members")
                .unwrap_or(Value::Array(vec![])))
        })
        .with_custom_method("add_member", |_resource| {
            // Placeholder for adding a member - would need additional parameters
            Ok(Value::Bool(true))
        })
        .with_database_mapping("groups", {
            let mut mappings = HashMap::new();
            mappings.insert("displayName".to_string(), "group_name".to_string());
            mappings.insert("members".to_string(), "member_data".to_string());
            mappings.insert("id".to_string(), "group_id".to_string());
            mappings
        })
        .build()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::{AttributeDefinition, AttributeType, Mutability, Schema, Uniqueness};
    use serde_json::json;

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

    #[test]
    fn test_user_handler_creation() {
        let schema = create_test_user_schema();
        let handler = create_user_resource_handler(schema.clone());

        assert_eq!(handler.schema.id, schema.id);
        assert_eq!(handler.schema.name, "User");
        assert!(!handler.handlers.is_empty());
        assert!(!handler.custom_methods.is_empty());
        assert!(!handler.mappers.is_empty());
    }

    #[test]
    fn test_user_resource_dynamic_operations() {
        use crate::resource::DynamicResource;
        use std::sync::Arc;

        let schema = create_test_user_schema();
        let handler = Arc::new(create_user_resource_handler(schema));

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
        use crate::resource::DynamicResource;
        use std::sync::Arc;

        let schema = create_test_user_schema();
        let handler = Arc::new(create_user_resource_handler(schema));

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
