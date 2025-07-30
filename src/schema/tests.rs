//! Tests for schema validation and registry functionality.
//!
//! This module contains comprehensive tests for schema loading, validation,
//! and all the various validation scenarios including edge cases and error conditions.

use super::registry::SchemaRegistry;
use super::types::AttributeType;
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
    let registry = SchemaRegistry::new().expect("Failed to create registry");

    // Test valid resource with ID
    let valid_user = json!({
        "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
        "id": "12345",
        "userName": "testuser@example.com",
        "meta": {
            "resourceType": "User"
        }
    });

    assert!(registry.validate_scim_resource(&valid_user).is_ok());

    // Test missing ID
    let missing_id_user = json!({
        "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
        "userName": "testuser@example.com",
        "meta": {
            "resourceType": "User"
        }
    });

    match registry.validate_scim_resource(&missing_id_user) {
        Err(ValidationError::MissingId) => {
            // Expected error
        }
        other => panic!("Expected MissingId error, got {:?}", other),
    }

    // Test empty ID
    let empty_id_user = json!({
        "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
        "id": "",
        "userName": "testuser@example.com",
        "meta": {
            "resourceType": "User"
        }
    });

    match registry.validate_scim_resource(&empty_id_user) {
        Err(ValidationError::EmptyId) => {
            // Expected error
        }
        other => panic!("Expected EmptyId error, got {:?}", other),
    }

    // Test invalid ID type
    let invalid_id_user = json!({
        "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
        "id": 12345,
        "userName": "testuser@example.com",
        "meta": {
            "resourceType": "User"
        }
    });

    match registry.validate_scim_resource(&invalid_id_user) {
        Err(ValidationError::InvalidIdFormat { .. }) => {
            // Expected error
        }
        other => panic!("Expected InvalidIdFormat error, got {:?}", other),
    }
}

#[test]
fn test_external_id_validation() {
    let registry = SchemaRegistry::new().expect("Failed to create registry");

    // Test valid external ID
    let valid_user = json!({
        "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
        "id": "12345",
        "userName": "testuser@example.com",
        "externalId": "ext123",
        "meta": {
            "resourceType": "User"
        }
    });

    assert!(registry.validate_scim_resource(&valid_user).is_ok());

    // Test invalid external ID type
    let invalid_external_id_user = json!({
        "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
        "id": "12345",
        "userName": "testuser@example.com",
        "externalId": 999,
        "meta": {
            "resourceType": "User"
        }
    });

    match registry.validate_scim_resource(&invalid_external_id_user) {
        Err(ValidationError::InvalidExternalId) => {
            // Expected error
        }
        other => panic!("Expected InvalidExternalId error, got {:?}", other),
    }

    // Test empty external ID
    let empty_external_id_user = json!({
        "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
        "id": "12345",
        "userName": "testuser@example.com",
        "externalId": "",
        "meta": {
            "resourceType": "User"
        }
    });

    match registry.validate_scim_resource(&empty_external_id_user) {
        Err(ValidationError::InvalidExternalId) => {
            // Expected error
        }
        other => panic!("Expected InvalidExternalId error, got {:?}", other),
    }
}

#[test]
fn test_phase_2_integration() {
    let registry = SchemaRegistry::new().expect("Failed to create registry");

    // Test that Phase 2 validation is actually being called in the main validation flow

    // Test 1: Comprehensive valid resource passes all Phase 2 validations
    let valid_user = json!({
        "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
        "id": "valid-id-123",
        "userName": "testuser@example.com",
        "externalId": "ext-valid-123",
        "meta": {
            "resourceType": "User",
            "created": "2023-01-01T00:00:00Z",
            "lastModified": "2023-01-01T00:00:00Z",
            "location": "https://example.com/Users/valid-id-123",
            "version": "v1.0"
        }
    });

    assert!(
        registry.validate_scim_resource(&valid_user).is_ok(),
        "Valid user should pass all Phase 2 validations"
    );

    // Test 2: Multiple Phase 2 errors are caught correctly
    let invalid_user_missing_id = json!({
        "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
        // Missing id
        "userName": "testuser@example.com",
        "meta": {
            "resourceType": "User"
        }
    });

    match registry.validate_scim_resource(&invalid_user_missing_id) {
        Err(ValidationError::MissingId) => {
            // Expected - ID validation caught the missing ID
        }
        other => panic!(
            "Expected MissingId error from Phase 2 validation, got {:?}",
            other
        ),
    }

    // Test 3: External ID validation is integrated
    let invalid_external_id = json!({
        "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
        "id": "valid-id",
        "userName": "testuser@example.com",
        "externalId": false, // Invalid type
        "meta": {
            "resourceType": "User"
        }
    });

    match registry.validate_scim_resource(&invalid_external_id) {
        Err(ValidationError::InvalidExternalId) => {
            // Expected - External ID validation caught the invalid type
        }
        other => panic!(
            "Expected InvalidExternalId error from Phase 2 validation, got {:?}",
            other
        ),
    }

    // Test 4: Meta validation enhancements are working
    let invalid_resource_type = json!({
        "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
        "id": "valid-id",
        "userName": "testuser@example.com",
        "meta": {
            "resourceType": "InvalidType" // Should fail our enhanced validation
        }
    });

    match registry.validate_scim_resource(&invalid_resource_type) {
        Err(ValidationError::InvalidResourceType { resource_type }) => {
            assert_eq!(resource_type, "InvalidType");
        }
        other => panic!(
            "Expected InvalidResourceType error from Phase 2 validation, got {:?}",
            other
        ),
    }

    // Test 5: Validation order - ID validation happens before schema validation
    let missing_id_and_schemas = json!({
        // Missing schemas array AND missing id
        "userName": "testuser@example.com",
        "meta": {
            "resourceType": "User"
        }
    });

    match registry.validate_scim_resource(&missing_id_and_schemas) {
        Err(ValidationError::MissingSchemas) => {
            // Schema validation happens first, so this is expected
        }
        other => panic!(
            "Expected MissingSchemas error (schema validation first), got {:?}",
            other
        ),
    }
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
            "version": "W/\"3694e05e9dff592\"",
            "location": "https://example.com/v2/Groups/e9e30dba-f08f-4109-8486-d5c6a331660a"
        }
    });

    let result = registry.validate_scim_resource(&group);
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

    let result = registry.validate_scim_resource(&group);
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

    let result = registry.validate_scim_resource(&group);
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
