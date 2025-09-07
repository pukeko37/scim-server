//! Resource Core Functionality Tests
//!
//! This test demonstrates the Resource implementation that uses
//! value objects as core members instead of raw JSON strings.

use scim_server::error::ValidationError;
use scim_server::resource::Resource;
use scim_server::resource::value_objects::{ExternalId, ResourceId, SchemaUri, UserName};
use scim_server::schema::registry::SchemaRegistry;
use scim_server::schema::validation::OperationContext;
use serde_json::{Map, json};

#[test]
fn test_resource_creation_from_json() {
    let user_data = json!({
        "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
        "id": "test-user-123",
        "userName": "testuser@example.com",
        "externalId": "ext-123",
        "displayName": "Test User",
        "active": true
    });

    let resource = Resource::from_json("User".to_string(), user_data).unwrap();

    // Verify that core fields are extracted as value objects
    assert_eq!(resource.resource_type, "User");
    assert_eq!(resource.get_id(), Some("test-user-123"));
    assert_eq!(resource.get_username(), Some("testuser@example.com"));
    assert_eq!(resource.get_external_id(), Some("ext-123"));
    assert_eq!(resource.schemas.len(), 1);
    assert_eq!(
        resource.schemas[0].as_str(),
        "urn:ietf:params:scim:schemas:core:2.0:User"
    );

    // Verify that extended attributes are preserved
    assert_eq!(
        resource.get_attribute("displayName"),
        Some(&json!("Test User"))
    );
    assert_eq!(resource.get_attribute("active"), Some(&json!(true)));
}

#[test]
fn test_resource_validation_errors() {
    // Empty ID should fail during resource construction
    let invalid_id = json!({
        "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
        "id": "",
        "userName": "testuser"
    });

    let result = Resource::from_json("User".to_string(), invalid_id);
    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), ValidationError::EmptyId));

    // Invalid schema URI should fail
    let invalid_schema = json!({
        "schemas": ["http://invalid-schema"],
        "id": "test-id",
        "userName": "testuser"
    });

    let result = Resource::from_json("User".to_string(), invalid_schema);
    assert!(result.is_err());

    // Empty external ID should fail
    let empty_external = json!({
        "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
        "id": "test-id",
        "userName": "testuser",
        "externalId": ""
    });

    let result = Resource::from_json("User".to_string(), empty_external);
    assert!(result.is_err());
}

#[test]
fn test_hybrid_validation() {
    let registry = SchemaRegistry::new().expect("Failed to create registry");

    let user_data = json!({
        "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
        "id": "validation-test-123",
        "userName": "validuser@example.com",
        "externalId": "ext-valid-456",
        "displayName": "Valid User",
        "active": true
    });

    let resource = Resource::from_json("User".to_string(), user_data).unwrap();
    let result = registry.validate_resource_hybrid(&resource);

    assert!(
        result.is_ok(),
        "Hybrid validation should pass: {:?}",
        result
    );
}

#[test]
fn test_json_serialization_roundtrip() {
    let original_data = json!({
        "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
        "id": "serialize-test",
        "userName": "serializeuser",
        "externalId": "ext-serialize",
        "displayName": "Serialize User",
        "active": true
    });

    let resource = Resource::from_json("User".to_string(), original_data.clone()).unwrap();
    let serialized = resource.to_json().unwrap();
    assert_eq!(serialized["id"], "serialize-test");
    assert_eq!(serialized["userName"], "serializeuser");
    assert_eq!(serialized["externalId"], "ext-serialize");
    assert_eq!(
        serialized["schemas"][0],
        "urn:ietf:params:scim:schemas:core:2.0:User"
    );
    assert_eq!(serialized["displayName"], "Serialize User");
    assert_eq!(serialized["active"], true);

    // Test round-trip serialization
    let deserialized = Resource::from_json("User".to_string(), serialized).unwrap();
    assert_eq!(resource.get_id(), deserialized.get_id());
    assert_eq!(resource.get_username(), deserialized.get_username());
    assert_eq!(resource.get_external_id(), deserialized.get_external_id());
}

#[test]
fn test_resource_with_minimal_fields() {
    let minimal_data = json!({
        "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
        "userName": "minimaluser"
        // No ID, no externalId
    });

    let resource = Resource::from_json("User".to_string(), minimal_data).unwrap();

    assert_eq!(resource.resource_type, "User");
    assert_eq!(resource.get_id(), None); // No ID provided
    assert_eq!(resource.get_username(), Some("minimaluser"));
    assert_eq!(resource.get_external_id(), None); // No external ID provided
    assert_eq!(resource.schemas.len(), 1);
}

#[test]
fn test_resource_default_schema() {
    let data_without_schemas = json!({
        "id": "default-schema-test",
        "userName": "defaultuser"
        // No schemas array - should get default
    });

    let resource = Resource::from_json("User".to_string(), data_without_schemas).unwrap();

    assert_eq!(resource.schemas.len(), 1);
    assert_eq!(
        resource.schemas[0].as_str(),
        "urn:ietf:params:scim:schemas:core:2.0:User"
    );
}

#[test]
fn test_group_resource() {
    let registry = SchemaRegistry::new().expect("Failed to create registry");

    let group_data = json!({
        "schemas": ["urn:ietf:params:scim:schemas:core:2.0:Group"],
        "id": "group-test",
        "displayName": "Test Group",
        "externalId": "ext-group-123"
    });

    let resource = Resource::from_json("Group".to_string(), group_data).unwrap();
    assert_eq!(resource.resource_type, "Group");
    assert_eq!(resource.get_id(), Some("group-test"));
    assert_eq!(resource.get_external_id(), Some("ext-group-123"));

    let result = registry.validate_resource_hybrid(&resource);
    assert!(result.is_ok(), "Group validation should pass: {:?}", result);
}

#[test]
fn test_value_object_access() {
    let data = json!({
        "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
        "id": "value-object-test",
        "userName": "vouser",
        "externalId": "ext-vo-123"
    });

    let resource = Resource::from_json("User".to_string(), data).unwrap();

    // Direct access to value objects
    assert!(resource.id.is_some());
    assert_eq!(resource.id.as_ref().unwrap().as_str(), "value-object-test");

    assert!(resource.user_name.is_some());
    assert_eq!(resource.user_name.as_ref().unwrap().as_str(), "vouser");

    assert!(resource.external_id.is_some());
    assert_eq!(
        resource.external_id.as_ref().unwrap().as_str(),
        "ext-vo-123"
    );

    assert_eq!(resource.schemas.len(), 1);
    assert_eq!(
        resource.schemas[0].as_str(),
        "urn:ietf:params:scim:schemas:core:2.0:User"
    );

    // Convenience methods still work
    assert_eq!(resource.get_id(), Some("value-object-test"));
    assert_eq!(resource.get_username(), Some("vouser"));
    assert_eq!(resource.get_external_id(), Some("ext-vo-123"));
}

#[test]
fn test_resource_builder_pattern() {
    let resource_id = ResourceId::new("builder-test-123".to_string()).unwrap();
    let user_name = UserName::new("builderuser@example.com".to_string()).unwrap();
    let external_id = ExternalId::new("ext-builder-456".to_string()).unwrap();
    let schema_uri =
        SchemaUri::new("urn:ietf:params:scim:schemas:core:2.0:User".to_string()).unwrap();

    let mut attributes = Map::new();
    attributes.insert("displayName".to_string(), json!("Builder User"));
    attributes.insert("active".to_string(), json!(true));

    let resource = Resource::new(
        "User".to_string(),
        Some(resource_id),
        vec![schema_uri],
        Some(external_id),
        Some(user_name),
        attributes,
    );

    assert_eq!(resource.get_id(), Some("builder-test-123"));
    assert_eq!(resource.get_username(), Some("builderuser@example.com"));
    assert_eq!(resource.get_external_id(), Some("ext-builder-456"));
    assert_eq!(
        resource.get_attribute("displayName"),
        Some(&json!("Builder User"))
    );
    assert_eq!(resource.get_attribute("active"), Some(&json!(true)));
}

#[test]
fn test_json_validation_compatibility() {
    let registry = SchemaRegistry::new().expect("Failed to create registry");

    let user_json = json!({
        "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
        "id": "json-compat-test",
        "userName": "jsonuser",
        "displayName": "JSON User"
    });

    // Test that the registry can validate JSON directly
    let result =
        registry.validate_json_resource_with_context("User", &user_json, OperationContext::Update);
    assert!(result.is_ok(), "JSON validation should work: {:?}", result);

    // Should produce same result as hybrid validation
    let resource = Resource::from_json("User".to_string(), user_json).unwrap();
    let hybrid_result = registry.validate_resource_hybrid(&resource);
    assert!(hybrid_result.is_ok(), "Hybrid validation should also work");
}

#[test]
fn test_extended_attributes_preservation() {
    let data_with_extensions = json!({
        "schemas": [
            "urn:ietf:params:scim:schemas:core:2.0:User",
            "urn:ietf:params:scim:schemas:extension:enterprise:2.0:User"
        ],
        "id": "ext-test",
        "userName": "extuser",
        "displayName": "Extended User",
        "customAttribute": "custom value",
        "urn:ietf:params:scim:schemas:extension:enterprise:2.0:User": {
            "employeeNumber": "12345",
            "department": "Engineering"
        }
    });

    let resource = Resource::from_json("User".to_string(), data_with_extensions).unwrap();

    // Core attributes should be extracted as value objects
    assert_eq!(resource.get_id(), Some("ext-test"));
    assert_eq!(resource.get_username(), Some("extuser"));

    // Extended attributes should be preserved in attributes map
    assert_eq!(
        resource.get_attribute("displayName"),
        Some(&json!("Extended User"))
    );
    assert_eq!(
        resource.get_attribute("customAttribute"),
        Some(&json!("custom value"))
    );

    // Extension schema should be preserved
    let extension_data =
        resource.get_attribute("urn:ietf:params:scim:schemas:extension:enterprise:2.0:User");
    assert!(extension_data.is_some());
    let ext_obj = extension_data.unwrap().as_object().unwrap();
    assert_eq!(ext_obj["employeeNumber"], "12345");
    assert_eq!(ext_obj["department"], "Engineering");

    // Schemas should include both core and extension
    assert_eq!(resource.schemas.len(), 2);
    let schema_strings = resource.get_schemas();
    assert!(schema_strings.contains(&"urn:ietf:params:scim:schemas:core:2.0:User".to_string()));
    assert!(
        schema_strings
            .contains(&"urn:ietf:params:scim:schemas:extension:enterprise:2.0:User".to_string())
    );
}

#[test]
fn test_resource_serde() {
    let original_data = json!({
        "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
        "id": "serde-test",
        "userName": "serdeuser",
        "displayName": "Serde User"
    });

    let resource = Resource::from_json("User".to_string(), original_data).unwrap();

    // Test serialization
    let serialized_json = serde_json::to_value(&resource).unwrap();
    assert_eq!(serialized_json["id"], "serde-test");
    assert_eq!(serialized_json["userName"], "serdeuser");
    assert_eq!(serialized_json["displayName"], "Serde User");

    // Test deserialization
    let deserialized: Resource = serde_json::from_value(serialized_json).unwrap();
    assert_eq!(deserialized.get_id(), Some("serde-test"));
    assert_eq!(deserialized.get_username(), Some("serdeuser"));
    assert_eq!(
        deserialized.get_attribute("displayName"),
        Some(&json!("Serde User"))
    );
}
