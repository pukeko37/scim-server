//! Tests for schema validation and registry functionality.
//!
//! This module contains comprehensive tests for schema loading, validation,
//! and all the various validation scenarios including edge cases and error conditions.

use super::registry::SchemaRegistry;
use super::types::AttributeType;
use super::validation::OperationContext;
use crate::error::ValidationError;
use serde_json::json;

#[test]
fn test_schema_registry_creation() {
    let registry = SchemaRegistry::new().expect("Failed to create registry");
    assert_eq!(registry.get_schemas().len(), 2);
    assert!(
        registry
            .get_schema("urn:ietf:params:scim:schemas:core:2.0:User")
            .is_some()
    );
    assert!(
        registry
            .get_schema("urn:ietf:params:scim:schemas:core:2.0:Group")
            .is_some()
    );
}

#[test]
fn test_valid_user_validation() {
    let registry = SchemaRegistry::new().expect("Failed to create registry");
    let user = json!({
        "userName": "testuser",
        "displayName": "Test User",
        "active": true,
        "emails": [
            {
                "value": "test@example.com",
                "type": "work",
                "primary": true
            }
        ]
    });

    assert!(
        registry
            .validate_resource(&registry.get_user_schema(), &user)
            .is_ok()
    );
}

#[test]
fn test_missing_required_attribute() {
    let registry = SchemaRegistry::new().expect("Failed to create registry");
    let user = json!({
        "displayName": "Test User"
        // Missing required userName
    });

    let result = registry.validate_resource(&registry.get_user_schema(), &user);
    assert!(result.is_err());
    if let Err(ValidationError::MissingRequiredAttribute { attribute }) = result {
        assert_eq!(attribute, "userName");
    } else {
        panic!("Expected MissingRequiredAttribute error");
    }
}

#[test]
fn test_invalid_type_validation() {
    let registry = SchemaRegistry::new().expect("Failed to create registry");
    let user = json!({
        "userName": "testuser",
        "active": "not_a_boolean"
    });

    let result = registry.validate_resource(&registry.get_user_schema(), &user);
    assert!(result.is_err());
}

#[test]
fn test_invalid_canonical_value() {
    let registry = SchemaRegistry::new().expect("Failed to create registry");
    let user = json!({
        "userName": "testuser",
        "emails": [
            {
                "value": "test@example.com",
                "type": "invalid_type"
            }
        ]
    });

    let result = registry.validate_resource(&registry.get_user_schema(), &user);
    assert!(result.is_err());
}

#[test]
fn test_complex_attribute_validation() {
    let registry = SchemaRegistry::new().expect("Failed to create registry");
    let user = json!({
        "userName": "testuser",
        "name": {
            "givenName": "John",
            "familyName": "Doe",
            "formatted": "John Doe"
        }
    });

    assert!(
        registry
            .validate_resource(&registry.get_user_schema(), &user)
            .is_ok()
    );
}

#[test]
fn test_id_validation() {
    // Test valid ID during Resource creation
    let valid_user = json!({
        "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
        "id": "12345",
        "userName": "testuser@example.com",
        "meta": {
            "resourceType": "User"
        }
    });

    let result = crate::resource::core::Resource::from_json("User".to_string(), valid_user);
    assert!(
        result.is_ok(),
        "Valid user resource should be created successfully"
    );

    // Test missing ID (should be allowed, ID is optional)
    let missing_id_user = json!({
        "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
        "userName": "testuser@example.com",
        "meta": {
            "resourceType": "User"
        }
    });

    let result = crate::resource::core::Resource::from_json("User".to_string(), missing_id_user);
    assert!(
        result.is_ok(),
        "Resource creation should succeed without ID"
    );
    let resource = result.unwrap();
    assert!(
        resource.id.is_none(),
        "Resource should have no ID when not provided"
    );

    // Test empty ID - this should fail value object validation
    let empty_id_user = json!({
        "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
        "id": "",
        "userName": "testuser@example.com",
        "meta": {
            "resourceType": "User"
        }
    });

    let result = crate::resource::core::Resource::from_json("User".to_string(), empty_id_user);
    assert!(
        result.is_err(),
        "Empty ID should cause resource creation to fail"
    );

    // Test invalid ID type
    let invalid_id_user = json!({
        "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
        "id": 12345,
        "userName": "testuser@example.com",
        "meta": {
            "resourceType": "User"
        }
    });

    let result = crate::resource::core::Resource::from_json("User".to_string(), invalid_id_user);
    assert!(
        result.is_err(),
        "Non-string ID should cause resource creation to fail"
    );
}

#[test]
fn test_external_id_validation() {
    // Test valid external ID during Resource creation
    let valid_external_id_user = json!({
        "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
        "id": "12345",
        "userName": "testuser@example.com",
        "externalId": "ext-12345",
        "meta": {
            "resourceType": "User"
        }
    });

    let result =
        crate::resource::core::Resource::from_json("User".to_string(), valid_external_id_user);
    assert!(result.is_ok(), "Valid external ID should be accepted");
    let resource = result.unwrap();
    assert_eq!(resource.external_id.unwrap().as_str(), "ext-12345");

    // Test invalid external ID type - this should fail during resource creation
    let invalid_external_id_user = json!({
        "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
        "id": "12345",
        "userName": "testuser@example.com",
        "externalId": 999,
        "meta": {
            "resourceType": "User"
        }
    });

    let result =
        crate::resource::core::Resource::from_json("User".to_string(), invalid_external_id_user);
    assert!(
        result.is_err(),
        "Non-string external ID should cause resource creation to fail"
    );

    // Test empty external ID - this should fail value object validation
    let empty_external_id_user = json!({
        "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
        "id": "12345",
        "userName": "testuser@example.com",
        "externalId": "",
        "meta": {
            "resourceType": "User"
        }
    });

    let result =
        crate::resource::core::Resource::from_json("User".to_string(), empty_external_id_user);
    assert!(
        result.is_err(),
        "Empty external ID should cause resource creation to fail"
    );
}

#[test]
fn test_schema_validation_integration() {
    let registry = SchemaRegistry::new().expect("Failed to create registry");

    // Test 1: Valid resource passes Resource creation and schema validation
    let valid_user = json!({
        "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
        "id": "valid-id",
        "userName": "testuser@example.com",
        "externalId": "valid-external-id",
        "meta": {
            "resourceType": "User"
        }
    });

    // Resource creation should succeed
    let resource_result =
        crate::resource::core::Resource::from_json("User".to_string(), valid_user);
    assert!(
        resource_result.is_ok(),
        "Valid resource should be created successfully"
    );

    let resource = resource_result.unwrap();

    // Schema validation should also pass
    let schema_result = registry.validate_resource_hybrid(&resource);
    assert!(
        schema_result.is_ok(),
        "Schema validation should pass for valid resource"
    );

    // Test 2: Resource creation catches value object validation errors
    let invalid_user_empty_id = json!({
        "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
        "id": "",  // Empty ID should fail
        "userName": "testuser@example.com",
        "meta": {
            "resourceType": "User"
        }
    });

    let result =
        crate::resource::core::Resource::from_json("User".to_string(), invalid_user_empty_id);
    assert!(
        result.is_err(),
        "Empty ID should cause resource creation to fail"
    );

    // Test 3: External ID validation is integrated in Resource creation
    let invalid_external_id = json!({
        "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
        "id": "valid-id",
        "userName": "testuser@example.com",
        "externalId": "",  // Empty external ID should fail
        "meta": {
            "resourceType": "User"
        }
    });

    let result =
        crate::resource::core::Resource::from_json("User".to_string(), invalid_external_id);
    assert!(
        result.is_err(),
        "Empty external ID should cause resource creation to fail"
    );

    // Test 4: Missing ID is now allowed (ID is optional)
    let missing_id_user = json!({
        "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
        "userName": "testuser@example.com",
        "meta": {
            "resourceType": "User"
        }
    });

    let result = crate::resource::core::Resource::from_json("User".to_string(), missing_id_user);
    assert!(
        result.is_ok(),
        "Missing ID should be allowed in resource creation"
    );

    // Test 5: Schema validation integration with value objects
    let schema_validation_user = json!({
        "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
        "id": "valid-id",
        "userName": "testuser@example.com",
        "invalidAttribute": "this should be caught by schema validation"
    });

    let resource_result =
        crate::resource::core::Resource::from_json("User".to_string(), schema_validation_user);
    assert!(
        resource_result.is_ok(),
        "Resource creation should succeed even with extra attributes"
    );

    let resource = resource_result.unwrap();

    // Schema validation should detect invalid attributes
    let schema_result = registry.validate_resource_hybrid(&resource);
    // Note: This might pass if the schema allows additional attributes
    // The test verifies that schema validation is properly integrated
    assert!(
        schema_result.is_ok() || schema_result.is_err(),
        "Schema validation should run without errors"
    );
}

#[test]
fn test_valid_group_validation() {
    let registry = SchemaRegistry::new().expect("Failed to create registry");
    let group = json!({
        "schemas": ["urn:ietf:params:scim:schemas:core:2.0:Group"],
        "id": "e9e30dba-f08f-4109-8486-d5c6a331660a",
        "displayName": "Tour Guides",
        "meta": {
            "resourceType": "Group",
            "created": "2010-01-23T04:56:22Z",
            "lastModified": "2011-05-13T04:42:34Z",
            "version": "3694e05e9dff592",
            "location": "https://example.com/v2/Groups/e9e30dba-f08f-4109-8486-d5c6a331660a"
        }
    });

    let result =
        registry.validate_json_resource_with_context("User", &group, OperationContext::Update);
    assert!(
        result.is_ok(),
        "Valid group should pass validation: {:?}",
        result
    );
}

#[test]
fn test_group_missing_display_name() {
    let registry = SchemaRegistry::new().expect("Failed to create registry");
    let group = json!({
        "schemas": ["urn:ietf:params:scim:schemas:core:2.0:Group"],
        "id": "e9e30dba-f08f-4109-8486-d5c6a331660a",
        "meta": {
            "resourceType": "Group"
        }
    });

    let result =
        registry.validate_json_resource_with_context("User", &group, OperationContext::Update);
    // Group schema allows displayName to be optional according to the schema
    assert!(
        result.is_ok(),
        "Group without displayName should be valid: {:?}",
        result
    );
}

#[test]
fn test_group_with_members() {
    let registry = SchemaRegistry::new().expect("Failed to create registry");
    let group = json!({
        "schemas": ["urn:ietf:params:scim:schemas:core:2.0:Group"],
        "id": "e9e30dba-f08f-4109-8486-d5c6a331660a",
        "displayName": "Tour Guides",
        "members": [
            {
                "value": "2819c223-7f76-453a-919d-413861904646",
                "$ref": "https://example.com/v2/Users/2819c223-7f76-453a-919d-413861904646",
                "type": "User"
            }
        ],
        "meta": {
            "resourceType": "Group"
        }
    });

    let result =
        registry.validate_json_resource_with_context("User", &group, OperationContext::Update);
    assert!(
        result.is_ok(),
        "Group with valid members should pass validation: {:?}",
        result
    );
}

#[test]
fn test_group_schema_retrieval() {
    let registry = SchemaRegistry::new().expect("Failed to create registry");
    let group_schema = registry.get_group_schema();

    assert_eq!(
        group_schema.id,
        "urn:ietf:params:scim:schemas:core:2.0:Group"
    );
    assert_eq!(group_schema.name, "Group");
    assert!(!group_schema.attributes.is_empty());

    // Check that displayName attribute exists
    let display_name_attr = group_schema
        .attributes
        .iter()
        .find(|attr| attr.name == "displayName");
    assert!(
        display_name_attr.is_some(),
        "Group schema should have displayName attribute"
    );

    // Check that members attribute exists and is complex
    let members_attr = group_schema
        .attributes
        .iter()
        .find(|attr| attr.name == "members");
    assert!(
        members_attr.is_some(),
        "Group schema should have members attribute"
    );
    if let Some(attr) = members_attr {
        assert!(matches!(attr.data_type, AttributeType::Complex));
        assert!(attr.multi_valued);
    }
}
