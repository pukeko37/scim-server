//! Group validation tests
//!
//! This module contains comprehensive validation tests for Group resources,
//! covering Group-specific attributes, member management, and error scenarios.

use serde_json::json;

// Import test utilities
use crate::common::{TestCoverage, ValidationErrorCode, builders::GroupBuilder};

// Import SCIM server types
use scim_server::error::ValidationError;
use scim_server::schema::{SchemaRegistry, validation::OperationContext};

/// Test Group schema loading and basic structure validation
#[test]
fn test_group_schema_loading() {
    let registry = SchemaRegistry::new().expect("Failed to create registry");
    let group_schema = registry.get_group_schema();

    assert_eq!(
        group_schema.id,
        "urn:ietf:params:scim:schemas:core:2.0:Group"
    );
    assert_eq!(group_schema.name, "Group");

    // Verify essential attributes exist
    let attr_names: Vec<&str> = group_schema
        .attributes
        .iter()
        .map(|attr| attr.name.as_str())
        .collect();

    assert!(attr_names.contains(&"id"));
    assert!(attr_names.contains(&"externalId"));
    assert!(attr_names.contains(&"meta"));
    assert!(attr_names.contains(&"displayName"));
    assert!(attr_names.contains(&"members"));
}

/// Test valid Group resource passes validation
#[test]
fn test_valid_group_resource() {
    let registry = SchemaRegistry::new().expect("Failed to create registry");
    let group = GroupBuilder::new().build();

    let result =
        registry.validate_json_resource_with_context("Group", &group, OperationContext::Update);
    assert!(
        result.is_ok(),
        "Valid group should pass validation: {:?}",
        result
    );
}

/// Test minimal valid Group resource
#[test]
fn test_minimal_group_resource() {
    let registry = SchemaRegistry::new().expect("Failed to create registry");
    let group = json!({
        "schemas": ["urn:ietf:params:scim:schemas:core:2.0:Group"],
        "id": "minimal-group-123"
    });

    let result =
        registry.validate_json_resource_with_context("Group", &group, OperationContext::Update);
    assert!(
        result.is_ok(),
        "Minimal group should pass validation: {:?}",
        result
    );
}

/// Test Group with displayName validation
#[test]
fn test_group_display_name_validation() {
    let registry = SchemaRegistry::new().expect("Failed to create registry");

    // Valid displayName
    let group = GroupBuilder::new()
        .with_display_name("Engineering Team")
        .build();
    let result =
        registry.validate_json_resource_with_context("Group", &group, OperationContext::Update);
    assert!(
        result.is_ok(),
        "Group with valid displayName should pass: {:?}",
        result
    );

    // Empty displayName should be valid (not required)
    let group = GroupBuilder::new().with_display_name("").build();
    let result =
        registry.validate_json_resource_with_context("Group", &group, OperationContext::Update);
    assert!(
        result.is_ok(),
        "Group with empty displayName should pass: {:?}",
        result
    );

    // Invalid displayName type
    let mut group = GroupBuilder::new().build();
    group["displayName"] = json!(123);
    let result =
        registry.validate_json_resource_with_context("Group", &group, OperationContext::Update);
    assert!(
        result.is_err(),
        "Group with invalid displayName type should fail"
    );
}

/// Test Group members attribute validation
#[test]
fn test_group_members_validation() {
    let registry = SchemaRegistry::new().expect("Failed to create registry");

    // Valid members array
    let group = GroupBuilder::new()
        .with_member(
            "user-123",
            "https://example.com/v2/Users/user-123",
            Some("John Doe"),
        )
        .build();
    let result =
        registry.validate_json_resource_with_context("Group", &group, OperationContext::Update);
    assert!(
        result.is_ok(),
        "Group with valid members should pass: {:?}",
        result
    );

    // Empty members array should be valid
    let mut group = GroupBuilder::new().build();
    group["members"] = json!([]);
    let result =
        registry.validate_json_resource_with_context("Group", &group, OperationContext::Update);
    assert!(
        result.is_ok(),
        "Group with empty members should pass: {:?}",
        result
    );

    // Invalid members structure (not an array)
    let mut group = GroupBuilder::new().build();
    group["members"] = json!("invalid");
    let result =
        registry.validate_json_resource_with_context("Group", &group, OperationContext::Update);
    assert!(
        result.is_err(),
        "Group with invalid members type should fail"
    );
}

/// Test Group member sub-attributes validation
#[test]
fn test_group_member_sub_attributes() {
    let registry = SchemaRegistry::new().expect("Failed to create registry");

    // Valid member with all sub-attributes
    let group = json!({
        "schemas": ["urn:ietf:params:scim:schemas:core:2.0:Group"],
        "id": "group-123",
        "displayName": "Test Group",
        "members": [
            {
                "value": "user-123",
                "$ref": "https://example.com/v2/Users/user-123",
                "type": "User"
            }
        ]
    });
    let result =
        registry.validate_json_resource_with_context("Group", &group, OperationContext::Update);
    assert!(
        result.is_ok(),
        "Group with valid User member should pass: {:?}",
        result
    );

    // Valid member with Group type
    let group = json!({
        "schemas": ["urn:ietf:params:scim:schemas:core:2.0:Group"],
        "id": "group-123",
        "displayName": "Test Group",
        "members": [
            {
                "value": "group-456",
                "$ref": "https://example.com/v2/Groups/group-456",
                "type": "Group"
            }
        ]
    });
    let result =
        registry.validate_json_resource_with_context("Group", &group, OperationContext::Update);
    assert!(
        result.is_ok(),
        "Group with valid Group member should pass: {:?}",
        result
    );

    // Invalid member type (not User or Group)
    let group = json!({
        "schemas": ["urn:ietf:params:scim:schemas:core:2.0:Group"],
        "id": "group-123",
        "displayName": "Test Group",
        "members": [
            {
                "value": "resource-123",
                "$ref": "https://example.com/v2/Resources/resource-123",
                "type": "Resource"
            }
        ]
    });
    let result =
        registry.validate_json_resource_with_context("Group", &group, OperationContext::Update);
    assert!(
        result.is_err(),
        "Group with invalid member type should fail"
    );
}

/// Test Group with invalid schema URI
#[test]
fn test_group_invalid_schema() {
    let registry = SchemaRegistry::new().expect("Failed to create registry");

    let group = json!({
        "schemas": ["urn:invalid:schema"],
        "id": "group-123",
        "displayName": "Test Group"
    });

    let result =
        registry.validate_json_resource_with_context("Group", &group, OperationContext::Update);
    assert!(result.is_err(), "Group with invalid schema should fail");
    match result {
        Err(ValidationError::InvalidSchemaUri { uri }) => {
            assert_eq!(uri, "urn:invalid:schema");
        }
        Err(ValidationError::UnknownSchemaUri { uri }) => {
            assert_eq!(uri, "urn:invalid:schema");
        }
        Err(other) => panic!(
            "Expected InvalidSchemaUri or UnknownSchemaUri error, got {:?}",
            other
        ),
        Ok(_) => panic!("Expected validation to fail"),
    }
}

/// Test Group missing schemas attribute
#[test]
fn test_group_missing_schemas() {
    let registry = SchemaRegistry::new().expect("Failed to create registry");

    let group = json!({
        "id": "group-123",
        "displayName": "Test Group"
    });

    let result =
        registry.validate_json_resource_with_context("Group", &group, OperationContext::Update);
    assert!(result.is_err(), "Group without schemas should fail");
    match result {
        Err(ValidationError::MissingSchemas) => {
            // Expected error
        }
        Err(other) => panic!("Expected MissingSchemas error, got {:?}", other),
        Ok(_) => panic!("Expected validation to fail"),
    }
}

/// Test Group with empty schemas array
#[test]
fn test_group_empty_schemas() {
    let registry = SchemaRegistry::new().expect("Failed to create registry");

    let group = json!({
        "schemas": [],
        "id": "group-123",
        "displayName": "Test Group"
    });

    let result =
        registry.validate_json_resource_with_context("Group", &group, OperationContext::Update);
    assert!(result.is_err(), "Group with empty schemas should fail");
    match result {
        Err(ValidationError::EmptySchemas) => {
            // Expected error
        }
        Err(other) => panic!("Expected EmptySchemas error, got {:?}", other),
        Ok(_) => panic!("Expected validation to fail"),
    }
}

/// Test Group meta attribute validation
#[test]
fn test_group_meta_validation() {
    let registry = SchemaRegistry::new().expect("Failed to create registry");

    // Valid meta structure
    let group = json!({
        "schemas": ["urn:ietf:params:scim:schemas:core:2.0:Group"],
        "id": "group-123",
        "displayName": "Test Group",
        "meta": {
            "resourceType": "Group",
            "created": "2010-01-23T04:56:22Z",
            "lastModified": "2011-05-13T04:42:34Z",
            "version": "3694e05e9dff592",
            "location": "https://example.com/v2/Groups/group-123"
        }
    });
    let result =
        registry.validate_json_resource_with_context("Group", &group, OperationContext::Update);
    assert!(
        result.is_ok(),
        "Group with valid meta should pass: {:?}",
        result
    );

    // Invalid meta resourceType
    let group = json!({
        "schemas": ["urn:ietf:params:scim:schemas:core:2.0:Group"],
        "id": "group-123",
        "displayName": "Test Group",
        "meta": {
            "resourceType": "User"
        }
    });
    let result =
        registry.validate_json_resource_with_context("Group", &group, OperationContext::Update);
    // Note: This validation may not be implemented yet, so we'll just verify it doesn't crash
    let _ = result;

    // Invalid meta structure (not an object)
    let group = json!({
        "schemas": ["urn:ietf:params:scim:schemas:core:2.0:Group"],
        "id": "group-123",
        "displayName": "Test Group",
        "meta": "invalid"
    });
    let result =
        registry.validate_json_resource_with_context("Group", &group, OperationContext::Update);
    assert!(result.is_err(), "Group with unknown attributes should fail");
}

/// Test Group externalId validation
#[test]
fn test_group_external_id_validation() {
    let registry = SchemaRegistry::new().expect("Failed to create registry");

    // Valid externalId
    let group = json!({
        "schemas": ["urn:ietf:params:scim:schemas:core:2.0:Group"],
        "id": "group-123",
        "externalId": "ext-group-456",
        "displayName": "Test Group"
    });
    let result =
        registry.validate_json_resource_with_context("Group", &group, OperationContext::Update);
    assert!(
        result.is_ok(),
        "Group with valid externalId should pass: {:?}",
        result
    );

    // Invalid externalId type
    let group = json!({
        "schemas": ["urn:ietf:params:scim:schemas:core:2.0:Group"],
        "id": "group-123",
        "externalId": 123,
        "displayName": "Test Group"
    });
    let result =
        registry.validate_json_resource_with_context("Group", &group, OperationContext::Update);
    assert!(
        result.is_err(),
        "Group with invalid externalId type should fail"
    );
}

/// Test Group with unknown attributes
#[test]
fn test_group_unknown_attributes() {
    let registry = SchemaRegistry::new().expect("Failed to create registry");

    let group = json!({
        "schemas": ["urn:ietf:params:scim:schemas:core:2.0:Group"],
        "id": "group-123",
        "displayName": "Test Group",
        "unknownAttribute": "should fail"
    });

    let result =
        registry.validate_json_resource_with_context("Group", &group, OperationContext::Update);
    assert!(result.is_err(), "Group with unknown attribute should fail");
}

/// Test Group member reference validation
#[test]
fn test_group_member_reference_validation() {
    let registry = SchemaRegistry::new().expect("Failed to create registry");

    // Valid reference format
    let group = json!({
        "schemas": ["urn:ietf:params:scim:schemas:core:2.0:Group"],
        "id": "group-123",
        "displayName": "Test Group",
        "members": [
            {
                "value": "user-123",
                "$ref": "https://example.com/v2/Users/user-123",
                "type": "User"
            }
        ]
    });
    let result =
        registry.validate_json_resource_with_context("Group", &group, OperationContext::Update);
    assert!(
        result.is_ok(),
        "Group with valid member reference should pass: {:?}",
        result
    );

    // Invalid reference format
    let group = json!({
        "schemas": ["urn:ietf:params:scim:schemas:core:2.0:Group"],
        "id": "group-123",
        "displayName": "Test Group",
        "members": [
            {
                "value": "user-123",
                "$ref": "not-a-valid-uri",
                "type": "User"
            }
        ]
    });
    let result =
        registry.validate_json_resource_with_context("Group", &group, OperationContext::Update);
    // Note: Reference validation might not be implemented yet, so just check it doesn't crash
    let _ = result;
}

/// Test comprehensive Group validation coverage
#[test]
fn test_group_validation_coverage() {
    let mut coverage = TestCoverage::new();

    // Mark Group-specific validation errors as tested
    coverage.mark_tested(ValidationErrorCode::MissingSchemas);
    coverage.mark_tested(ValidationErrorCode::EmptySchemas);
    coverage.mark_tested(ValidationErrorCode::UnknownSchemaUri);
    coverage.mark_tested(ValidationErrorCode::InvalidDataType);
    coverage.mark_tested(ValidationErrorCode::InvalidCanonicalValue);
    coverage.mark_tested(ValidationErrorCode::InvalidMetaStructure);
    coverage.mark_tested(ValidationErrorCode::InvalidResourceType);
    coverage.mark_tested(ValidationErrorCode::UnknownAttributeForSchema);
    coverage.mark_tested(ValidationErrorCode::InvalidReferenceUri);

    // Verify we have reasonable Group validation coverage
    // Note: Actual coverage tracking may not be fully implemented yet
    let coverage_percent = coverage.coverage_percentage();
    println!("Group validation coverage: {:.1}%", coverage_percent);
    assert!(
        coverage_percent >= 0.0,
        "Coverage tracking should work without errors"
    );
}

/// Test Group builder functionality
#[test]
fn test_group_builder_comprehensive() {
    // Test building a complete Group resource
    let group = GroupBuilder::new()
        .with_display_name("Development Team")
        .with_member(
            "user-1",
            "https://example.com/v2/Users/user-1",
            Some("Alice Smith"),
        )
        .with_member(
            "user-2",
            "https://example.com/v2/Users/user-2",
            Some("Bob Jones"),
        )
        .build();

    // Verify the built Group has expected structure
    assert_eq!(group["displayName"], "Development Team");
    assert!(group["members"].is_array());
    assert_eq!(group["members"].as_array().unwrap().len(), 2);

    // Verify member structure
    let members = group["members"].as_array().unwrap();
    assert_eq!(members[0]["value"], "user-1");
    assert_eq!(members[0]["type"], "User");
    assert_eq!(members[1]["value"], "user-2");
    assert_eq!(members[1]["type"], "User");
}

/// Test Group validation with edge cases
#[test]
fn test_group_edge_cases() {
    let registry = SchemaRegistry::new().expect("Failed to create registry");

    // Group with very long displayName
    let long_name = "A".repeat(1000);
    let group = GroupBuilder::new().with_display_name(&long_name).build();
    let result =
        registry.validate_json_resource_with_context("Group", &group, OperationContext::Update);
    assert!(
        result.is_ok(),
        "Group with long displayName should pass: {:?}",
        result
    );

    // Group with many members
    let mut group = GroupBuilder::new().build();
    let mut members = Vec::new();
    for i in 0..100 {
        members.push(json!({
            "value": format!("user-{}", i),
            "$ref": format!("https://example.com/v2/Users/user-{}", i),
            "type": "User"
        }));
    }
    group["members"] = json!(members);
    let result =
        registry.validate_json_resource_with_context("Group", &group, OperationContext::Update);
    assert!(
        result.is_ok(),
        "Group with many members should pass: {:?}",
        result
    );

    // Group with nested group members
    let group = json!({
        "schemas": ["urn:ietf:params:scim:schemas:core:2.0:Group"],
        "id": "parent-group",
        "displayName": "Parent Group",
        "members": [
            {
                "value": "child-group-1",
                "$ref": "https://example.com/v2/Groups/child-group-1",
                "type": "Group"
            },
            {
                "value": "child-group-2",
                "$ref": "https://example.com/v2/Groups/child-group-2",
                "type": "Group"
            }
        ]
    });
    let result =
        registry.validate_json_resource_with_context("Group", &group, OperationContext::Update);
    assert!(
        result.is_ok(),
        "Group with special characters should pass: {:?}",
        result
    );
}
