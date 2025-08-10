//! Serialization and deserialization implementations for SCIM resources.
//!
//! This module provides Serde implementations for the Resource struct,
//! enabling seamless JSON serialization/deserialization while maintaining
//! type safety for core attributes.

use crate::resource::resource::Resource;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_json::Value;

impl Serialize for Resource {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.to_json()
            .map_err(serde::ser::Error::custom)?
            .serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for Resource {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = Value::deserialize(deserializer)?;

        // We need a resource type to properly parse, but JSON doesn't contain it
        // For now, we'll extract it from the schema URIs or use a default
        let resource_type = if let Some(obj) = value.as_object() {
            if let Some(schemas) = obj.get("schemas").and_then(|s| s.as_array()) {
                if let Some(first_schema) = schemas.first().and_then(|s| s.as_str()) {
                    if first_schema.contains("User") {
                        "User".to_string()
                    } else if first_schema.contains("Group") {
                        "Group".to_string()
                    } else {
                        "Resource".to_string()
                    }
                } else {
                    "Resource".to_string()
                }
            } else {
                "Resource".to_string()
            }
        } else {
            return Err(serde::de::Error::custom("Resource must be a JSON object"));
        };

        Self::from_json(resource_type, value)
            .map_err(|e| serde::de::Error::custom(format!("Validation error: {}", e)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::resource::value_objects::{ResourceId, SchemaUri, UserName};
    use serde_json::json;

    #[test]
    fn test_resource_serialization() {
        let resource = Resource::new(
            "User".to_string(),
            Some(ResourceId::new("123".to_string()).unwrap()),
            vec![SchemaUri::new("urn:ietf:params:scim:schemas:core:2.0:User".to_string()).unwrap()],
            None,
            Some(UserName::new("jdoe".to_string()).unwrap()),
            serde_json::Map::new(),
        );

        let serialized = serde_json::to_string(&resource).unwrap();
        assert!(serialized.contains("\"id\":\"123\""));
        assert!(serialized.contains("\"userName\":\"jdoe\""));
    }

    #[test]
    fn test_resource_deserialization() {
        let json_data = json!({
            "id": "123",
            "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
            "userName": "jdoe",
            "displayName": "John Doe"
        });

        let resource: Resource = serde_json::from_value(json_data).unwrap();
        assert_eq!(resource.get_id(), Some("123"));
        assert_eq!(resource.get_username(), Some("jdoe"));
        assert_eq!(resource.resource_type, "User");
    }

    #[test]
    fn test_round_trip_serialization() {
        let original_json = json!({
            "id": "456",
            "schemas": ["urn:ietf:params:scim:schemas:core:2.0:Group"],
            "displayName": "Test Group",
            "members": []
        });

        let resource: Resource = serde_json::from_value(original_json.clone()).unwrap();
        let serialized = serde_json::to_value(&resource).unwrap();

        assert_eq!(serialized["id"], "456");
        assert_eq!(serialized["displayName"], "Test Group");
        assert!(serialized["schemas"].is_array());
    }
}
