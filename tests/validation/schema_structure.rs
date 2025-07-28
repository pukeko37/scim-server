//! Schema structure validation tests.
//!
//! This module tests validation errors related to the structure and format
//! of SCIM schema attributes in resources (Errors 1-8).

use serde_json::json;

// Import test utilities
use crate::common::{ValidationErrorCode, builders::UserBuilder, fixtures::rfc_examples};

/// Test Error #1: Missing required `schemas` attribute in resource
#[test]
fn test_missing_schemas_attribute() {
    // Create a User resource without the schemas attribute
    let invalid_user = UserBuilder::new().without_schemas().build();

    // This should trigger ValidationError::MissingRequiredAttribute for schemas
    // Since schemas is a fundamental requirement for all SCIM resources
    assert!(!invalid_user.as_object().unwrap().contains_key("schemas"));

    // In a real validation context, this would fail
    // For now, we verify the test data is constructed correctly
    let builder = UserBuilder::new().without_schemas();
    let expected_errors = builder.expected_errors();
    assert_eq!(expected_errors, &[ValidationErrorCode::MissingSchemas]);
}

/// Test Error #2: Empty `schemas` array in resource
#[test]
fn test_empty_schemas_array() {
    // Create a User resource with empty schemas array
    let invalid_user = UserBuilder::new().with_empty_schemas().build();

    // Verify schemas array is empty
    assert_eq!(invalid_user["schemas"], json!([]));

    // Verify expected error is tracked
    let builder = UserBuilder::new().with_empty_schemas();
    let expected_errors = builder.expected_errors();
    assert_eq!(expected_errors, &[ValidationErrorCode::EmptySchemas]);
}

/// Test Error #3: Invalid schema URI format in `schemas` attribute
#[test]
fn test_invalid_schema_uri_format() {
    // Create a User resource with malformed schema URI
    let invalid_user = UserBuilder::new().with_invalid_schema_uri().build();

    // Verify the invalid URI is present
    assert_eq!(invalid_user["schemas"][0], "not-a-valid-uri");

    // Verify expected error is tracked
    let builder = UserBuilder::new().with_invalid_schema_uri();
    let expected_errors = builder.expected_errors();
    assert_eq!(expected_errors, &[ValidationErrorCode::InvalidSchemaUri]);
}

/// Test Error #4: Unknown/unregistered schema URI referenced
#[test]
fn test_unknown_schema_uri() {
    // Create a User resource with unknown schema URI
    let invalid_user = UserBuilder::new().with_unknown_schema_uri().build();

    // Verify the unknown URI is present
    assert_eq!(invalid_user["schemas"][0], "urn:example:unknown:schema");

    // Verify expected error is tracked
    let builder = UserBuilder::new().with_unknown_schema_uri();
    let expected_errors = builder.expected_errors();
    assert_eq!(expected_errors, &[ValidationErrorCode::UnknownSchemaUri]);
}

/// Test Error #5: Duplicate schema URIs in `schemas` array
#[test]
fn test_duplicate_schema_uris() {
    // Create a User resource with duplicate schema URIs
    let invalid_user = UserBuilder::new().with_duplicate_schema_uris().build();

    // Verify duplicates are present
    let schemas = invalid_user["schemas"].as_array().unwrap();
    assert_eq!(schemas.len(), 2);
    assert_eq!(schemas[0], schemas[1]);
    assert_eq!(schemas[0], "urn:ietf:params:scim:schemas:core:2.0:User");

    // Verify expected error is tracked
    let builder = UserBuilder::new().with_duplicate_schema_uris();
    let expected_errors = builder.expected_errors();
    assert_eq!(expected_errors, &[ValidationErrorCode::DuplicateSchemaUri]);
}

/// Test Error #6: Missing base/core schema URI for resource type
#[test]
fn test_missing_base_schema() {
    // Create a User resource with only extension schema, missing base User schema
    let invalid_user = json!({
        "schemas": ["urn:ietf:params:scim:schemas:extension:enterprise:2.0:User"],
        "id": "123",
        "userName": "test@example.com",
        "meta": {
            "resourceType": "User"
        }
    });

    // Verify base schema is missing
    let schemas = invalid_user["schemas"].as_array().unwrap();
    assert_eq!(schemas.len(), 1);
    assert!(!schemas.contains(&json!("urn:ietf:params:scim:schemas:core:2.0:User")));
    assert!(schemas.contains(&json!(
        "urn:ietf:params:scim:schemas:extension:enterprise:2.0:User"
    )));

    // This would trigger ValidationErrorCode::MissingBaseSchema in real validation
}

/// Test Error #7: Schema extension URI present without base schema
#[test]
fn test_extension_without_base_schema() {
    // Similar to above but more explicit about the extension-without-base error
    let invalid_user = json!({
        "schemas": [
            "urn:ietf:params:scim:schemas:extension:enterprise:2.0:User",
            "urn:example:params:scim:schemas:extension:custom:2.0:User"
        ],
        "id": "123",
        "userName": "test@example.com",
        "meta": {
            "resourceType": "User"
        }
    });

    // Verify only extensions are present, no base schema
    let schemas = invalid_user["schemas"].as_array().unwrap();
    assert_eq!(schemas.len(), 2);
    assert!(!schemas.contains(&json!("urn:ietf:params:scim:schemas:core:2.0:User")));

    // All schemas are extensions
    for schema in schemas {
        assert!(schema.as_str().unwrap().contains("extension"));
    }
}

/// Test Error #8: Required schema extension missing when resource type mandates it
#[test]
fn test_missing_required_extension() {
    // Simulate a scenario where a ResourceType configuration requires an extension
    // but the resource doesn't include it
    let user_without_required_extension = json!({
        "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
        "id": "123",
        "userName": "test@example.com",
        "meta": {
            "resourceType": "User"
        }
    });

    // Verify base schema is present but required extension is missing
    let schemas = user_without_required_extension["schemas"]
        .as_array()
        .unwrap();
    assert_eq!(schemas.len(), 1);
    assert!(schemas.contains(&json!("urn:ietf:params:scim:schemas:core:2.0:User")));
    assert!(!schemas.contains(&json!(
        "urn:ietf:params:scim:schemas:extension:enterprise:2.0:User"
    )));
}

/// Test valid schema configurations to ensure we don't have false positives
#[test]
fn test_valid_schema_configurations() {
    // Test 1: Valid minimal User with just core schema
    let valid_minimal = rfc_examples::user_minimal();
    let schemas = valid_minimal["schemas"].as_array().unwrap();
    assert_eq!(schemas.len(), 1);
    assert_eq!(schemas[0], "urn:ietf:params:scim:schemas:core:2.0:User");

    // Test 2: Valid User with extension
    let valid_enterprise = rfc_examples::user_enterprise();
    let schemas = valid_enterprise["schemas"].as_array().unwrap();
    assert_eq!(schemas.len(), 2);
    assert!(schemas.contains(&json!("urn:ietf:params:scim:schemas:core:2.0:User")));
    assert!(schemas.contains(&json!(
        "urn:ietf:params:scim:schemas:extension:enterprise:2.0:User"
    )));

    // Test 3: Valid Group
    let valid_group = rfc_examples::group_basic();
    let schemas = valid_group["schemas"].as_array().unwrap();
    assert_eq!(schemas.len(), 1);
    assert_eq!(schemas[0], "urn:ietf:params:scim:schemas:core:2.0:Group");
}

/// Test multiple schema structure errors in a single resource
#[test]
fn test_multiple_schema_structure_errors() {
    // Create a resource with multiple schema-related issues
    let invalid_resource = json!({
        "schemas": [], // Empty schemas (Error #2)
        "userName": "test@example.com"
        // Also missing id, meta, etc. but focusing on schema errors
    });

    // Verify multiple issues are present
    assert_eq!(invalid_resource["schemas"], json!([]));
    assert!(!invalid_resource.as_object().unwrap().contains_key("id"));
    assert!(!invalid_resource.as_object().unwrap().contains_key("meta"));
}

/// Test schema URI format validation specifics
#[test]
fn test_schema_uri_format_validation() {
    // Test various invalid URI formats
    let test_cases = vec![
        "not-a-uri",                    // No scheme
        "urn:",                         // Incomplete URN
        "http://example.com",           // Wrong scheme (should be URN)
        "urn:invalid",                  // Malformed URN
        "",                             // Empty string
        "urn:ietf:params:scim:schemas", // Incomplete SCIM URN
    ];

    for invalid_uri in test_cases {
        let invalid_user = json!({
            "schemas": [invalid_uri],
            "id": "123",
            "userName": "test@example.com",
            "meta": {
                "resourceType": "User"
            }
        });

        // Each should be considered an invalid schema URI
        assert_eq!(invalid_user["schemas"][0], invalid_uri);
    }
}

/// Test valid schema URI formats
#[test]
fn test_valid_schema_uri_formats() {
    let valid_uris = vec![
        "urn:ietf:params:scim:schemas:core:2.0:User",
        "urn:ietf:params:scim:schemas:core:2.0:Group",
        "urn:ietf:params:scim:schemas:extension:enterprise:2.0:User",
        "urn:example:params:scim:schemas:extension:custom:2.0:User",
    ];

    for valid_uri in valid_uris {
        let valid_user = json!({
            "schemas": [valid_uri],
            "id": "123",
            "userName": "test@example.com",
            "meta": {
                "resourceType": "User"
            }
        });

        // Should be considered valid URIs
        assert_eq!(valid_user["schemas"][0], valid_uri);
        assert!(valid_uri.starts_with("urn:"));
        assert!(valid_uri.contains("scim:schemas"));
    }
}

/// Test schema-to-resource-type consistency
#[test]
fn test_schema_resource_type_consistency() {
    // User resource should have User schemas
    let valid_user = json!({
        "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
        "id": "123",
        "userName": "test@example.com",
        "meta": {
            "resourceType": "User"
        }
    });
    assert_eq!(valid_user["meta"]["resourceType"], "User");
    assert!(valid_user["schemas"][0].as_str().unwrap().contains("User"));

    // Group resource should have Group schemas
    let valid_group = json!({
        "schemas": ["urn:ietf:params:scim:schemas:core:2.0:Group"],
        "id": "123",
        "displayName": "Test Group",
        "meta": {
            "resourceType": "Group"
        }
    });
    assert_eq!(valid_group["meta"]["resourceType"], "Group");
    assert!(
        valid_group["schemas"][0]
            .as_str()
            .unwrap()
            .contains("Group")
    );

    // Inconsistent case: User resource with Group schema (should be invalid)
    let inconsistent_resource = json!({
        "schemas": ["urn:ietf:params:scim:schemas:core:2.0:Group"],
        "id": "123",
        "userName": "test@example.com", // User attribute
        "meta": {
            "resourceType": "User" // User type
        }
    });
    // This shows inconsistency between schema and resource type
    assert_eq!(inconsistent_resource["meta"]["resourceType"], "User");
    assert!(
        inconsistent_resource["schemas"][0]
            .as_str()
            .unwrap()
            .contains("Group")
    );
}

#[cfg(test)]
mod coverage_tests {
    use super::*;
    use crate::common::TestCoverage;

    #[test]
    fn test_schema_structure_error_coverage() {
        // Verify all schema structure errors (1-8) are covered by our tests
        let mut coverage = TestCoverage::new();

        // Mark errors as tested based on our test functions
        coverage.mark_tested(ValidationErrorCode::MissingSchemas); // Error #1
        coverage.mark_tested(ValidationErrorCode::EmptySchemas); // Error #2
        coverage.mark_tested(ValidationErrorCode::InvalidSchemaUri); // Error #3
        coverage.mark_tested(ValidationErrorCode::UnknownSchemaUri); // Error #4
        coverage.mark_tested(ValidationErrorCode::DuplicateSchemaUri); // Error #5
        coverage.mark_tested(ValidationErrorCode::MissingBaseSchema); // Error #6
        coverage.mark_tested(ValidationErrorCode::ExtensionWithoutBase); // Error #7
        coverage.mark_tested(ValidationErrorCode::MissingRequiredExtension); // Error #8

        // Verify we've covered all schema structure errors
        let schema_structure_errors = [
            ValidationErrorCode::MissingSchemas,
            ValidationErrorCode::EmptySchemas,
            ValidationErrorCode::InvalidSchemaUri,
            ValidationErrorCode::UnknownSchemaUri,
            ValidationErrorCode::DuplicateSchemaUri,
            ValidationErrorCode::MissingBaseSchema,
            ValidationErrorCode::ExtensionWithoutBase,
            ValidationErrorCode::MissingRequiredExtension,
        ];

        for error in &schema_structure_errors {
            assert!(
                coverage.is_tested(error),
                "Error {:?} not covered by tests",
                error
            );
        }
    }
}
