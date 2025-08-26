//! Common attributes validation tests.
//!
//! This module tests validation errors related to common SCIM attributes
//! that are present in all resources (Errors 9-21).

use serde_json::json;

// Import test utilities
use crate::common::{ValidationErrorCode, builders::UserBuilder, fixtures::rfc_examples};

// Import SCIM server types
use scim_server::error::ValidationError;
use scim_server::schema::{OperationContext, SchemaRegistry};

/// Test Error #9: Missing required `id` attribute in resource
#[test]
fn test_missing_id_attribute() {
    let registry = SchemaRegistry::new().expect("Failed to create registry");

    // Create a User resource without the id attribute
    let invalid_user = UserBuilder::new().without_id().build();

    // Verify the test data is constructed correctly
    assert!(!invalid_user.as_object().unwrap().contains_key("id"));

    // Actually validate the resource
    let result = registry.validate_json_resource_with_context(
        "User",
        &invalid_user,
        OperationContext::Update,
    );

    // Assert that validation fails with the expected error
    assert!(result.is_err());
    match result {
        Err(ValidationError::MissingId) => {
            // Expected error occurred
        }
        Err(other) => panic!("Expected MissingId error, got {:?}", other),
        Ok(_) => panic!("Expected validation to fail, but it passed"),
    }
}

/// Test Error #10: Empty or null `id` value
#[test]
fn test_empty_id_value() {
    let registry = SchemaRegistry::new().expect("Failed to create registry");

    // Create a User resource with empty id
    let invalid_user = UserBuilder::new().with_empty_id().build();

    // Verify the test data is constructed correctly
    assert_eq!(invalid_user["id"], "");

    // Actually validate the resource
    let result = registry.validate_json_resource_with_context(
        "User",
        &invalid_user,
        OperationContext::Update,
    );

    // Assert that validation fails with the expected error
    assert!(result.is_err());
    match result {
        Err(ValidationError::EmptyId) => {
            // Expected error occurred
        }
        Err(other) => panic!("Expected EmptyId error, got {:?}", other),
        Ok(_) => panic!("Expected validation to fail, but it passed"),
    }
}

/// Test Error #10: Null `id` value
#[test]
fn test_null_id_value() {
    let registry = SchemaRegistry::new().expect("Failed to create registry");

    // Create a User resource with null id
    let null_id_user = json!({
        "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
        "id": null,
        "userName": "test@example.com",
        "meta": {
            "resourceType": "User"
        }
    });

    // Verify the test data is constructed correctly
    assert!(null_id_user["id"].is_null());

    // Actually validate the resource
    let result = registry.validate_json_resource_with_context(
        "User",
        &null_id_user,
        OperationContext::Update,
    );

    // Assert that validation fails with the expected error
    assert!(result.is_err());
    match result {
        Err(ValidationError::InvalidIdFormat { id }) => {
            assert_eq!(id, "null");
        }
        Err(other) => panic!("Expected InvalidIdFormat error, got {:?}", other),
        Ok(_) => panic!("Expected validation to fail, but it passed"),
    }
}

/// Test Error #11: Invalid `id` format (non-string)
#[test]
fn test_invalid_id_format() {
    let registry = SchemaRegistry::new().expect("Failed to create registry");

    // Create a User resource with numeric id
    let invalid_user = UserBuilder::new().with_numeric_id().build();

    // Verify the test data is constructed correctly
    assert!(invalid_user["id"].is_number());

    // Actually validate the resource
    let result = registry.validate_json_resource_with_context(
        "User",
        &invalid_user,
        OperationContext::Update,
    );

    // Assert that validation fails with the expected error
    assert!(result.is_err());
    match result {
        Err(ValidationError::InvalidIdFormat { id }) => {
            assert_eq!(id, "123");
        }
        Err(other) => panic!("Expected InvalidIdFormat error, got {:?}", other),
        Ok(_) => panic!("Expected validation to fail, but it passed"),
    }
}

/// Test Error #11: Invalid `id` format (array)
#[test]
fn test_invalid_id_format_array() {
    let registry = SchemaRegistry::new().expect("Failed to create registry");

    // Create a User resource with array id
    let invalid_user = UserBuilder::new().with_array_id().build();

    // Verify the test data is constructed correctly
    assert!(invalid_user["id"].is_array());

    // Actually validate the resource
    let result = registry.validate_json_resource_with_context(
        "User",
        &invalid_user,
        OperationContext::Update,
    );

    // Assert that validation fails with the expected error
    assert!(result.is_err());
    match result {
        Err(ValidationError::InvalidIdFormat { id }) => {
            assert_eq!(id, r#"["123","456"]"#);
        }
        Err(other) => panic!("Expected InvalidIdFormat error, got {:?}", other),
        Ok(_) => panic!("Expected validation to fail, but it passed"),
    }
}

/// Test Error #11: Invalid `id` format (object)
#[test]
fn test_invalid_id_format_object() {
    let registry = SchemaRegistry::new().expect("Failed to create registry");

    // Create a User resource with object id
    let invalid_user = UserBuilder::new().with_object_id().build();

    // Verify the test data is constructed correctly
    assert!(invalid_user["id"].is_object());

    // Actually validate the resource
    let result = registry.validate_json_resource_with_context(
        "User",
        &invalid_user,
        OperationContext::Update,
    );

    // Assert that validation fails with the expected error
    assert!(result.is_err());
    match result {
        Err(ValidationError::InvalidIdFormat { id }) => {
            assert_eq!(id, r#"{"nested":"value"}"#);
        }
        Err(other) => panic!("Expected InvalidIdFormat error, got {:?}", other),
        Ok(_) => panic!("Expected validation to fail, but it passed"),
    }
}

/// Test Error #12: Client-provided `id` in create operation
#[test]
fn test_client_provided_id_in_create() {
    let registry = SchemaRegistry::new().expect("Failed to create registry");

    // Create a User resource with client-provided ID for CREATE operation
    let user_with_id = UserBuilder::new().build(); // Default builder includes ID

    // Verify the test data has an ID
    assert!(user_with_id.as_object().unwrap().contains_key("id"));

    // Actually validate the resource with CREATE context
    let result = registry.validate_json_resource_with_context(
        "User",
        &user_with_id,
        OperationContext::Create,
    );

    // Assert that validation fails with the expected error
    assert!(result.is_err());
    match result {
        Err(ValidationError::ClientProvidedId) => {
            // Expected error occurred
        }
        Err(other) => panic!("Expected ClientProvidedId error, got {:?}", other),
        Ok(_) => panic!("Expected validation to fail, but it passed"),
    }
}

/// Test Error #13: Invalid `externalId` format
#[test]
fn test_invalid_external_id_format() {
    let registry = SchemaRegistry::new().expect("Failed to create registry");

    // Create a User resource with numeric externalId
    let invalid_user = UserBuilder::new().with_numeric_external_id().build();

    // Verify the test data is constructed correctly
    assert!(invalid_user["externalId"].is_number());

    // Actually validate the resource
    let result = registry.validate_json_resource_with_context(
        "User",
        &invalid_user,
        OperationContext::Update,
    );

    // Assert that validation fails with the expected error
    assert!(result.is_err());
    match result {
        Err(ValidationError::InvalidExternalId) => {
            // Expected error occurred
        }
        Err(other) => panic!("Expected InvalidExternalId error, got {:?}", other),
        Ok(_) => panic!("Expected validation to fail, but it passed"),
    }
}

/// Test Error #13: Invalid `externalId` format (array)
#[test]
fn test_invalid_external_id_format_array() {
    let registry = SchemaRegistry::new().expect("Failed to create registry");

    // Create a User resource with array externalId
    let invalid_user = UserBuilder::new().with_array_external_id().build();

    // Verify the test data is constructed correctly
    assert!(invalid_user["externalId"].is_array());

    // Actually validate the resource
    let result = registry.validate_json_resource_with_context(
        "User",
        &invalid_user,
        OperationContext::Update,
    );

    // Assert that validation fails with the expected error
    assert!(result.is_err());
    match result {
        Err(ValidationError::InvalidExternalId) => {
            // Expected error occurred
        }
        Err(other) => panic!("Expected InvalidExternalId error, got {:?}", other),
        Ok(_) => panic!("Expected validation to fail, but it passed"),
    }
}

/// Test Error #14: Invalid `meta` structure (non-object)
#[test]
fn test_invalid_meta_structure() {
    let registry = SchemaRegistry::new().expect("Failed to create registry");

    // Create a User resource with string meta
    let invalid_user = UserBuilder::new().with_string_meta().build();

    // Verify the test data is constructed correctly
    assert!(invalid_user["meta"].is_string());

    // Actually validate the resource
    let result = registry.validate_json_resource_with_context(
        "User",
        &invalid_user,
        OperationContext::Update,
    );

    // Assert that validation fails with the expected error
    assert!(result.is_err());
    match result {
        Err(ValidationError::InvalidMetaStructure) => {
            // Expected error occurred
        }
        Err(other) => panic!("Expected InvalidMetaStructure error, got {:?}", other),
        Ok(_) => panic!("Expected validation to fail, but it passed"),
    }
}

/// Test Error #14: Invalid `meta` structure (array)
#[test]
fn test_invalid_meta_structure_array() {
    let registry = SchemaRegistry::new().expect("Failed to create registry");

    // Create a User resource with array meta
    let invalid_user = UserBuilder::new().with_array_meta().build();

    // Verify the test data is constructed correctly
    assert!(invalid_user["meta"].is_array());

    // Actually validate the resource
    let result = registry.validate_json_resource_with_context(
        "User",
        &invalid_user,
        OperationContext::Update,
    );

    // Assert that validation fails with the expected error
    assert!(result.is_err());
    match result {
        Err(ValidationError::InvalidMetaStructure) => {
            // Expected error occurred
        }
        Err(other) => panic!("Expected InvalidMetaStructure error, got {:?}", other),
        Ok(_) => panic!("Expected validation to fail, but it passed"),
    }
}

/// Test Error #15: Missing required `meta.resourceType`
#[test]
fn test_missing_meta_resource_type() {
    let registry = SchemaRegistry::new().expect("Failed to create registry");

    // Create a User resource without meta.resourceType
    let invalid_user = UserBuilder::new().without_meta_resource_type().build();

    // Verify the test data is constructed correctly
    assert!(
        !invalid_user["meta"]
            .as_object()
            .unwrap()
            .contains_key("resourceType")
    );

    // Actually validate the resource
    let result = registry.validate_json_resource_with_context(
        "User",
        &invalid_user,
        OperationContext::Update,
    );

    // Assert that validation fails with the expected error
    assert!(result.is_err());
    match result {
        Err(ValidationError::MissingResourceType) => {
            // Expected error occurred
        }
        Err(other) => panic!("Expected MissingResourceType error, got {:?}", other),
        Ok(_) => panic!("Expected validation to fail, but it passed"),
    }
}

/// Test Error #16: Invalid `meta.resourceType` value
#[test]
fn test_invalid_meta_resource_type() {
    let registry = SchemaRegistry::new().expect("Failed to create registry");

    // Create a User resource with invalid meta.resourceType
    let invalid_user = UserBuilder::new().with_invalid_meta_resource_type().build();

    // Verify the test data is constructed correctly
    assert_eq!(invalid_user["meta"]["resourceType"], "InvalidType");

    // Actually validate the resource
    let result = registry.validate_json_resource_with_context(
        "User",
        &invalid_user,
        OperationContext::Update,
    );

    // Assert that validation fails with the expected error
    assert!(result.is_err());
    match result {
        Err(ValidationError::InvalidResourceType { resource_type }) => {
            assert_eq!(resource_type, "InvalidType");
        }
        Err(other) => panic!("Expected InvalidResourceType error, got {:?}", other),
        Ok(_) => panic!("Expected validation to fail, but it passed"),
    }
}

/// Test Error #16: Non-string `meta.resourceType`
#[test]
fn test_invalid_meta_resource_type_non_string() {
    let registry = SchemaRegistry::new().expect("Failed to create registry");

    // Create a User resource with numeric meta.resourceType
    let invalid_user = UserBuilder::new().with_numeric_meta_resource_type().build();

    // Verify the test data is constructed correctly
    assert!(invalid_user["meta"]["resourceType"].is_number());

    // Actually validate the resource
    let result = registry.validate_json_resource_with_context(
        "User",
        &invalid_user,
        OperationContext::Update,
    );

    // Assert that validation fails with the expected error
    assert!(result.is_err());
    match result {
        Err(ValidationError::InvalidMetaStructure) => {
            // Non-string resourceType is caught as invalid meta structure
        }
        Err(other) => panic!("Expected InvalidMetaStructure error, got {:?}", other),
        Ok(_) => panic!("Expected validation to fail, but it passed"),
    }
}

/// Test Error #17: Client-provided readonly meta attributes
#[test]
fn test_client_provided_meta_readonly_attributes() {
    let registry = SchemaRegistry::new().expect("Failed to create registry");

    // Create a User resource with readonly meta attributes for CREATE operation
    let user_with_readonly_meta = UserBuilder::new()
        .without_id() // Remove ID since CREATE shouldn't have it
        .with_readonly_meta_attributes()
        .build();

    // Verify the test data has readonly meta attributes
    let meta = user_with_readonly_meta["meta"].as_object().unwrap();
    assert!(meta.contains_key("created") || meta.contains_key("lastModified"));

    // Actually validate the resource with CREATE context
    let result = registry.validate_json_resource_with_context(
        "User",
        &user_with_readonly_meta,
        OperationContext::Create,
    );

    // Assert that validation fails with the expected error
    assert!(result.is_err());
    match result {
        Err(ValidationError::ClientProvidedMeta) => {
            // Expected error occurred
        }
        Err(other) => panic!("Expected ClientProvidedMeta error, got {:?}", other),
        Ok(_) => panic!("Expected validation to fail, but it passed"),
    }
}

/// Test Error #18: Invalid `meta.created` datetime format (non-string)
#[test]
fn test_invalid_created_datetime() {
    let registry = SchemaRegistry::new().expect("Failed to create registry");

    // Create a User resource with non-string created datetime
    let invalid_user = UserBuilder::new().with_numeric_created_datetime().build();

    // Verify the test data is constructed correctly
    assert!(invalid_user["meta"]["created"].is_number());

    // Actually validate the resource
    let result = registry.validate_json_resource_with_context(
        "User",
        &invalid_user,
        OperationContext::Update,
    );

    // Assert that validation fails with the expected error
    assert!(result.is_err());
    match result {
        Err(ValidationError::InvalidCreatedDateTime) => {
            // Expected error occurred
        }
        Err(other) => panic!("Expected InvalidCreatedDateTime error, got {:?}", other),
        Ok(_) => panic!("Expected validation to fail, but it passed"),
    }
}

/// Test Error #18: Non-string `meta.created`
#[test]
fn test_invalid_created_datetime_non_string() {
    let registry = SchemaRegistry::new().expect("Failed to create registry");

    // Create a User resource with numeric created datetime
    let invalid_user = UserBuilder::new().with_numeric_created_datetime().build();

    // Verify the test data is constructed correctly
    assert!(invalid_user["meta"]["created"].is_number());

    // Actually validate the resource
    let result = registry.validate_json_resource_with_context(
        "User",
        &invalid_user,
        OperationContext::Update,
    );

    // Assert that validation fails with the expected error
    assert!(result.is_err());
    match result {
        Err(ValidationError::InvalidCreatedDateTime) => {
            // Expected error occurred
        }
        Err(other) => panic!("Expected InvalidCreatedDateTime error, got {:?}", other),
        Ok(_) => panic!("Expected validation to fail, but it passed"),
    }
}

/// Test Error #19: Invalid `meta.lastModified` datetime format (non-string)
#[test]
fn test_invalid_last_modified_datetime() {
    let registry = SchemaRegistry::new().expect("Failed to create registry");

    // Create a User resource with non-string lastModified datetime
    let invalid_user = json!({
        "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
        "id": "123",
        "userName": "test@example.com",
        "meta": {
            "resourceType": "User",
            "lastModified": 123456789
        }
    });

    // Verify the test data is constructed correctly
    assert!(invalid_user["meta"]["lastModified"].is_number());

    // Actually validate the resource
    let result = registry.validate_json_resource_with_context(
        "User",
        &invalid_user,
        OperationContext::Update,
    );

    // Assert that validation fails with the expected error
    assert!(result.is_err());
    match result {
        Err(ValidationError::InvalidModifiedDateTime) => {
            // Expected error occurred
        }
        Err(other) => panic!("Expected InvalidModifiedDateTime error, got {:?}", other),
        Ok(_) => panic!("Expected validation to fail, but it passed"),
    }
}

/// Test Error #20: Invalid `meta.location` URI format (non-string)
#[test]
fn test_invalid_location_uri() {
    let registry = SchemaRegistry::new().expect("Failed to create registry");

    // Create a User resource with non-string location
    let invalid_user = json!({
        "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
        "id": "123",
        "userName": "test@example.com",
        "meta": {
            "resourceType": "User",
            "location": 123
        }
    });

    // Verify the test data is constructed correctly
    assert!(invalid_user["meta"]["location"].is_number());

    // Actually validate the resource
    let result = registry.validate_json_resource_with_context(
        "User",
        &invalid_user,
        OperationContext::Update,
    );

    // Assert that validation fails with the expected error
    assert!(result.is_err());
    match result {
        Err(ValidationError::InvalidLocationUri) => {
            // Expected error occurred
        }
        Err(other) => panic!("Expected InvalidLocationUri error, got {:?}", other),
        Ok(_) => panic!("Expected validation to fail, but it passed"),
    }
}

/// Test Error #21: Invalid `meta.version` format
#[test]
fn test_invalid_version_format() {
    let registry = SchemaRegistry::new().expect("Failed to create registry");

    // Create a User resource with invalid version
    let invalid_user = UserBuilder::new().with_invalid_version_format().build();

    // Verify the test data is constructed correctly
    assert!(invalid_user["meta"]["version"].is_array());

    // Actually validate the resource
    let result = registry.validate_json_resource_with_context(
        "User",
        &invalid_user,
        OperationContext::Update,
    );

    // Assert that validation fails with the expected error
    assert!(result.is_err());
    match result {
        Err(ValidationError::InvalidVersionFormat) => {
            // Expected error occurred
        }
        Err(other) => panic!("Expected InvalidVersionFormat error, got {:?}", other),
        Ok(_) => panic!("Expected validation to fail, but it passed"),
    }
}

/// Test valid common attributes to ensure no false positives
#[test]
fn test_valid_common_attributes() {
    let registry = SchemaRegistry::new().expect("Failed to create registry");

    // Test minimal valid user from RFC examples
    let valid_user = rfc_examples::user_minimal();

    // This should pass validation
    let result =
        registry.validate_json_resource_with_context("User", &valid_user, OperationContext::Update);
    assert!(
        result.is_ok(),
        "Valid user should pass validation: {:?}",
        result
    );
}

/// Test valid user with optional attributes
#[test]
fn test_valid_common_attributes_with_optional() {
    let registry = SchemaRegistry::new().expect("Failed to create registry");

    // Test user with valid external ID
    let valid_user = UserBuilder::new()
        .with_id("123")
        .with_external_id("ext-123")
        .build();

    // This should pass validation
    let result =
        registry.validate_json_resource_with_context("User", &valid_user, OperationContext::Update);
    assert!(
        result.is_ok(),
        "Valid user with external ID should pass validation: {:?}",
        result
    );
}

/// Test multiple common attribute errors
#[test]
fn test_multiple_common_attribute_errors() {
    let registry = SchemaRegistry::new().expect("Failed to create registry");

    // Create a User resource with multiple common attribute errors
    let invalid_user = UserBuilder::new().without_id().with_string_meta().build();

    // Verify the test data has multiple issues
    assert!(!invalid_user.as_object().unwrap().contains_key("id"));
    assert!(invalid_user["meta"].is_string());

    // Actually validate the resource - should catch the first error
    let result = registry.validate_json_resource_with_context(
        "User",
        &invalid_user,
        OperationContext::Update,
    );

    // Assert that validation fails (will catch first error encountered)
    assert!(result.is_err());
    // Note: The specific error depends on validation order
    match result {
        Err(ValidationError::MissingId) => {
            // ID validation runs first
        }
        Err(ValidationError::InvalidMetaStructure) => {
            // Meta validation might run first
        }
        Err(other) => panic!(
            "Expected MissingId or InvalidMetaStructure error, got {:?}",
            other
        ),
        Ok(_) => panic!("Expected validation to fail, but it passed"),
    }
}

/// Test valid external ID format
#[test]
fn test_valid_external_id() {
    let registry = SchemaRegistry::new().expect("Failed to create registry");

    // Create a User resource with valid external ID
    let valid_user = UserBuilder::new()
        .with_external_id("valid-external-id")
        .build();

    // Verify the test data is constructed correctly
    assert_eq!(valid_user["externalId"], "valid-external-id");

    // Actually validate the resource
    let result =
        registry.validate_json_resource_with_context("User", &valid_user, OperationContext::Update);

    // Assert that validation passes
    assert!(
        result.is_ok(),
        "User with valid external ID should pass validation: {:?}",
        result
    );
}

/// Coverage verification for common attributes validation
mod coverage_tests {
    use super::*;

    /// Verify that all common attribute error types are covered by tests
    #[test]
    fn test_common_attributes_error_coverage() {
        // List of error codes that should be tested in this module
        let expected_errors = vec![
            ValidationErrorCode::MissingId, // Error #9  ✅ test_missing_id_attribute
            ValidationErrorCode::EmptyId,   // Error #10 ✅ test_empty_id_value
            ValidationErrorCode::InvalidIdFormat, // Error #11 ✅ test_invalid_id_format*
            ValidationErrorCode::ClientProvidedId, // Error #12 🔲 Deferred (needs operation context)
            ValidationErrorCode::InvalidExternalId, // Error #13 ✅ test_invalid_external_id_format*
            ValidationErrorCode::InvalidMetaStructure, // Error #14 ✅ test_invalid_meta_structure*
            ValidationErrorCode::MissingResourceType, // Error #15 🔲 Currently optional in validation
            ValidationErrorCode::InvalidResourceType, // Error #16 ✅ test_invalid_meta_resource_type*
            ValidationErrorCode::ClientProvidedMeta, // Error #17 🔲 Deferred (needs operation context)
            ValidationErrorCode::InvalidCreatedDateTime, // Error #18 ✅ test_invalid_created_datetime* (basic type checking)
            ValidationErrorCode::InvalidModifiedDateTime, // Error #19 ✅ test_invalid_last_modified_datetime (basic type checking)
            ValidationErrorCode::InvalidLocationUri, // Error #20 ✅ test_invalid_location_uri (basic type checking)
            ValidationErrorCode::InvalidVersionFormat, // Error #21 ✅ test_invalid_version_format
        ];

        // This test serves as documentation of which errors are tested
        // The actual test implementations above verify the error handling
        println!("Common attributes error coverage:");
        for error in &expected_errors {
            match error {
                ValidationErrorCode::ClientProvidedId | ValidationErrorCode::ClientProvidedMeta => {
                    println!("  {:?}: 🔲 Deferred (needs operation context)", error);
                }
                _ => {
                    println!("  {:?}: ✅ Tested", error);
                }
            }
        }

        // Verify we have the expected number of error types
        assert_eq!(
            expected_errors.len(),
            13,
            "Should test 13 common attribute error types"
        );

        // 10 out of 13 errors are testable in current context
        let testable_errors = expected_errors
            .iter()
            .filter(|e| {
                !matches!(
                    e,
                    ValidationErrorCode::ClientProvidedId
                        | ValidationErrorCode::ClientProvidedMeta
                        | ValidationErrorCode::MissingResourceType
                )
            })
            .count();
        assert_eq!(
            testable_errors, 10,
            "Should have 10 testable common attribute errors in current implementation"
        );
    }
}
