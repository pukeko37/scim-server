//! Group resource handler implementation using the dynamic schema approach.
//!
//! This module provides a factory function for creating Group resource handlers
//! that support group management operations with dynamic, schema-driven behavior.

use crate::resource::SchemaResourceBuilder;
use crate::schema::Schema;
use serde_json::Value;
use std::collections::HashMap;

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
