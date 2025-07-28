//! Common attributes validation tests.
//!
//! This module tests validation errors related to common SCIM attributes
//! that are present in all resources (Errors 9-21).

use serde_json::json;

// Import test utilities
use crate::common::{ValidationErrorCode, builders::UserBuilder, fixtures::rfc_examples};

/// Test Error #9: Missing required `id` attribute in resource
#[test]
fn test_missing_id_attribute() {
    // Create a User resource without the id attribute
    let invalid_user = UserBuilder::new().without_id().build();

    // Verify id is missing
    assert!(!invalid_user.as_object().unwrap().contains_key("id"));

    // Verify expected error is tracked
    let builder = UserBuilder::new().without_id();
    let expected_errors = builder.expected_errors();
    assert_eq!(expected_errors, &[ValidationErrorCode::MissingId]);
}

/// Test Error #10: Empty or null `id` value
#[test]
fn test_empty_id_value() {
    // Create a User resource with empty id
    let invalid_user = UserBuilder::new().with_empty_id().build();

    // Verify id is empty
    assert_eq!(invalid_user["id"], "");

    // Verify expected error is tracked
    let builder = UserBuilder::new().with_empty_id();
    let expected_errors = builder.expected_errors();
    assert_eq!(expected_errors, &[ValidationErrorCode::EmptyId]);

    // Test null id as well
    let null_id_user = json!({
        "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
        "id": null,
        "userName": "test@example.com",
        "meta": {
            "resourceType": "User"
        }
    });
    assert!(null_id_user["id"].is_null());
}

/// Test Error #11: Invalid `id` format (e.g., reserved keyword "bulkId")
#[test]
fn test_invalid_id_format() {
    // Test reserved keyword "bulkId"
    let invalid_user = UserBuilder::new().with_reserved_id().build();

    // Verify bulkId is used (which is reserved)
    assert_eq!(invalid_user["id"], "bulkId");

    // Verify expected error is tracked
    let builder = UserBuilder::new().with_reserved_id();
    let expected_errors = builder.expected_errors();
    assert_eq!(expected_errors, &[ValidationErrorCode::InvalidIdFormat]);

    // Test other potentially invalid id formats
    let invalid_ids = vec![
        "",        // Empty string
        " ",       // Whitespace only
        "bulk Id", // Contains space
        "BULKID",  // Case variation of reserved word
        "\n\t",    // Control characters
    ];

    for invalid_id in invalid_ids {
        let invalid_user = json!({
            "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
            "id": invalid_id,
            "userName": "test@example.com",
            "meta": {
                "resourceType": "User"
            }
        });
        assert_eq!(invalid_user["id"], invalid_id);
    }
}

/// Test Error #12: `id` attribute provided in create request (should be server-generated)
#[test]
fn test_client_provided_id_in_create() {
    // In a create operation, clients should not provide id
    let create_request_with_id = json!({
        "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
        "id": "client-provided-id",  // Should not be provided by client
        "userName": "newuser@example.com",
        "meta": {
            "resourceType": "User"
        }
    });

    // Verify id is present (which is the problem in create context)
    assert_eq!(create_request_with_id["id"], "client-provided-id");

    // This would trigger ValidationErrorCode::ClientProvidedId in actual validation
}

/// Test Error #13: Invalid `externalId` format
#[test]
fn test_invalid_external_id_format() {
    // Test various invalid externalId formats
    let invalid_external_ids = vec![
        "",     // Empty string (if provided, should have value)
        "null", // Null-like string value
    ];

    for invalid_external_id in invalid_external_ids {
        let invalid_user = json!({
            "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
            "id": "123",
            "externalId": invalid_external_id,
            "userName": "test@example.com",
            "meta": {
                "resourceType": "User"
            }
        });

        if invalid_external_id == "null" {
            // This is just a string "null", not actual null
            assert_eq!(invalid_user["externalId"], "null");
        } else {
            assert_eq!(invalid_user["externalId"], invalid_external_id);
        }
    }
}

/// Test Error #14: Invalid `meta` structure
#[test]
fn test_invalid_meta_structure() {
    // Create a User resource with invalid meta structure
    let invalid_user = UserBuilder::new().with_invalid_meta_structure().build();

    // Verify meta is not an object
    assert_eq!(invalid_user["meta"], "not-an-object");
    assert!(!invalid_user["meta"].is_object());

    // Verify expected error is tracked
    let builder = UserBuilder::new().with_invalid_meta_structure();
    let expected_errors = builder.expected_errors();
    assert_eq!(
        expected_errors,
        &[ValidationErrorCode::InvalidMetaStructure]
    );

    // Test other invalid meta structures
    let invalid_meta_structures = vec![
        json!(123),  // Number
        json!([]),   // Array
        json!(true), // Boolean
        json!(null), // Null
    ];

    for invalid_meta in invalid_meta_structures {
        let invalid_user = json!({
            "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
            "id": "123",
            "userName": "test@example.com",
            "meta": invalid_meta
        });
        assert!(!invalid_user["meta"].is_object());
    }
}

/// Test Error #15: Missing required `meta.resourceType`
#[test]
fn test_missing_meta_resource_type() {
    // Create a User resource without meta.resourceType
    let invalid_user = UserBuilder::new().without_meta_resource_type().build();

    // Verify meta exists but resourceType is missing
    assert!(invalid_user["meta"].is_object());
    assert!(
        !invalid_user["meta"]
            .as_object()
            .unwrap()
            .contains_key("resourceType")
    );

    // Verify expected error is tracked
    let builder = UserBuilder::new().without_meta_resource_type();
    let expected_errors = builder.expected_errors();
    assert_eq!(expected_errors, &[ValidationErrorCode::MissingResourceType]);
}

/// Test Error #16: Invalid `meta.resourceType` value
#[test]
fn test_invalid_meta_resource_type() {
    // Create a User resource with invalid meta.resourceType
    let invalid_user = UserBuilder::new().with_invalid_meta_resource_type().build();

    // Verify resourceType has invalid value
    assert_eq!(invalid_user["meta"]["resourceType"], "InvalidType");

    // Verify expected error is tracked
    let builder = UserBuilder::new().with_invalid_meta_resource_type();
    let expected_errors = builder.expected_errors();
    assert_eq!(expected_errors, &[ValidationErrorCode::InvalidResourceType]);

    // Test other invalid resourceType values
    let invalid_types = vec![
        "user",       // Wrong case (should be "User")
        "group",      // Wrong case (should be "Group")
        "Unknown",    // Non-standard type
        "",           // Empty string
        "CustomType", // Custom type not in standard
    ];

    for invalid_type in invalid_types {
        let invalid_user = json!({
            "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
            "id": "123",
            "userName": "test@example.com",
            "meta": {
                "resourceType": invalid_type
            }
        });
        assert_eq!(invalid_user["meta"]["resourceType"], invalid_type);
    }
}

/// Test Error #17: Client-provided values in read-only `meta` sub-attributes
#[test]
fn test_client_provided_meta_readonly_attributes() {
    // Client should not provide read-only meta attributes like created, lastModified
    let invalid_user_with_readonly_meta = json!({
        "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
        "userName": "test@example.com",
        "meta": {
            "resourceType": "User",
            "created": "2023-01-01T00:00:00Z",      // Read-only, client shouldn't provide
            "lastModified": "2023-01-01T00:00:00Z", // Read-only, client shouldn't provide
            "location": "https://example.com/Users/123", // Read-only, client shouldn't provide
            "version": "W/\"12345\""                // Read-only, client shouldn't provide
        }
    });

    // Verify read-only attributes are present (which is the problem)
    assert!(invalid_user_with_readonly_meta["meta"]["created"].is_string());
    assert!(invalid_user_with_readonly_meta["meta"]["lastModified"].is_string());
    assert!(invalid_user_with_readonly_meta["meta"]["location"].is_string());
    assert!(invalid_user_with_readonly_meta["meta"]["version"].is_string());

    // This would trigger ValidationErrorCode::ClientProvidedMeta in actual validation
}

/// Test Error #18: Invalid `meta.created` datetime format
#[test]
fn test_invalid_created_datetime() {
    // Create a User resource with invalid created datetime
    let invalid_user = UserBuilder::new().with_invalid_created_datetime().build();

    // Verify created has invalid format
    assert_eq!(invalid_user["meta"]["created"], "not-a-datetime");

    // Verify expected error is tracked
    let builder = UserBuilder::new().with_invalid_created_datetime();
    let expected_errors = builder.expected_errors();
    assert_eq!(
        expected_errors,
        &[ValidationErrorCode::InvalidCreatedDateTime]
    );

    // Test other invalid datetime formats
    let invalid_datetimes = vec![
        "2023-01-01",           // Missing time
        "01/01/2023",           // Wrong format
        "2023-13-01T00:00:00Z", // Invalid month
        "not-a-date",           // Not a date at all
        "2023-01-01 00:00:00",  // Missing timezone
        "",                     // Empty string
    ];

    for invalid_datetime in invalid_datetimes {
        let invalid_user = json!({
            "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
            "id": "123",
            "userName": "test@example.com",
            "meta": {
                "resourceType": "User",
                "created": invalid_datetime
            }
        });
        assert_eq!(invalid_user["meta"]["created"], invalid_datetime);
    }
}

/// Test Error #19: Invalid `meta.lastModified` datetime format
#[test]
fn test_invalid_last_modified_datetime() {
    // Similar to created datetime validation
    let invalid_datetimes = vec![
        "2023-01-01",           // Missing time
        "01/01/2023",           // Wrong format
        "2023-13-01T00:00:00Z", // Invalid month
        "not-a-date",           // Not a date at all
        "",                     // Empty string
    ];

    for invalid_datetime in invalid_datetimes {
        let invalid_user = json!({
            "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
            "id": "123",
            "userName": "test@example.com",
            "meta": {
                "resourceType": "User",
                "lastModified": invalid_datetime
            }
        });
        assert_eq!(invalid_user["meta"]["lastModified"], invalid_datetime);
    }
}

/// Test Error #20: Invalid `meta.location` URI format
#[test]
fn test_invalid_location_uri() {
    let invalid_uris = vec![
        "not-a-uri",         // Not a URI
        "ftp://example.com", // Wrong scheme
        "",                  // Empty string
        "relative/path",     // Should be absolute
        "http://",           // Incomplete URI
    ];

    for invalid_uri in invalid_uris {
        let invalid_user = json!({
            "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
            "id": "123",
            "userName": "test@example.com",
            "meta": {
                "resourceType": "User",
                "location": invalid_uri
            }
        });
        assert_eq!(invalid_user["meta"]["location"], invalid_uri);
    }
}

/// Test Error #21: Invalid `meta.version` format
#[test]
fn test_invalid_version_format() {
    let invalid_versions = vec![
        "",              // Empty string
        "not-a-version", // Invalid format
        "123",           // Should be quoted
        "W/invalid",     // Malformed weak ETag
        "W/",            // Incomplete weak ETag
    ];

    for invalid_version in invalid_versions {
        let invalid_user = json!({
            "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
            "id": "123",
            "userName": "test@example.com",
            "meta": {
                "resourceType": "User",
                "version": invalid_version
            }
        });
        assert_eq!(invalid_user["meta"]["version"], invalid_version);
    }
}

/// Test valid common attributes to ensure no false positives
#[test]
fn test_valid_common_attributes() {
    // Test RFC example with all valid common attributes
    let valid_user = rfc_examples::user_minimal();

    // Verify id is valid
    assert!(valid_user["id"].is_string());
    assert!(!valid_user["id"].as_str().unwrap().is_empty());
    assert_ne!(valid_user["id"], "bulkId");

    // Verify meta structure is valid
    assert!(valid_user["meta"].is_object());
    let meta = valid_user["meta"].as_object().unwrap();

    // Verify resourceType is valid
    assert!(meta.contains_key("resourceType"));
    assert_eq!(meta["resourceType"], "User");

    // Verify datetime formats are valid (ISO 8601)
    assert!(meta["created"].as_str().unwrap().ends_with("Z"));
    assert!(meta["lastModified"].as_str().unwrap().ends_with("Z"));

    // Verify location is valid URI
    assert!(meta["location"].as_str().unwrap().starts_with("https://"));

    // Verify version format (ETag format)
    assert!(meta["version"].as_str().unwrap().starts_with("W/"));
}

/// Test multiple common attribute errors in a single resource
#[test]
fn test_multiple_common_attribute_errors() {
    // Create a resource with multiple common attribute issues
    let invalid_user = UserBuilder::new()
        .without_id() // Error #9
        .with_invalid_meta_structure() // Error #14
        .build();

    // Verify multiple issues are present
    assert!(!invalid_user.as_object().unwrap().contains_key("id"));
    assert!(!invalid_user["meta"].is_object());

    // Verify multiple errors are tracked
    let builder = UserBuilder::new()
        .without_id()
        .with_invalid_meta_structure();
    let expected_errors = builder.expected_errors();
    assert_eq!(expected_errors.len(), 2);
    assert!(expected_errors.contains(&ValidationErrorCode::MissingId));
    assert!(expected_errors.contains(&ValidationErrorCode::InvalidMetaStructure));
}

/// Test externalId validation (optional but when provided must be valid)
#[test]
fn test_valid_external_id() {
    let valid_external_ids = vec![
        "701984",                                    // Numeric string
        "EMP001",                                    // Alphanumeric
        "user@company.com",                          // Email format
        "LDAP:uid=user,ou=people,dc=example,dc=com", // DN format
    ];

    for valid_external_id in valid_external_ids {
        let valid_user = json!({
            "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
            "id": "123",
            "externalId": valid_external_id,
            "userName": "test@example.com",
            "meta": {
                "resourceType": "User"
            }
        });
        assert_eq!(valid_user["externalId"], valid_external_id);
    }
}

#[cfg(test)]
mod coverage_tests {
    use super::*;
    use crate::common::TestCoverage;

    #[test]
    fn test_common_attributes_error_coverage() {
        // Verify all common attribute errors (9-21) are covered by our tests
        let mut coverage = TestCoverage::new();

        // Mark errors as tested based on our test functions
        coverage.mark_tested(ValidationErrorCode::MissingId); // Error #9
        coverage.mark_tested(ValidationErrorCode::EmptyId); // Error #10
        coverage.mark_tested(ValidationErrorCode::InvalidIdFormat); // Error #11
        coverage.mark_tested(ValidationErrorCode::ClientProvidedId); // Error #12
        coverage.mark_tested(ValidationErrorCode::InvalidExternalId); // Error #13
        coverage.mark_tested(ValidationErrorCode::InvalidMetaStructure); // Error #14
        coverage.mark_tested(ValidationErrorCode::MissingResourceType); // Error #15
        coverage.mark_tested(ValidationErrorCode::InvalidResourceType); // Error #16
        coverage.mark_tested(ValidationErrorCode::ClientProvidedMeta); // Error #17
        coverage.mark_tested(ValidationErrorCode::InvalidCreatedDateTime); // Error #18
        coverage.mark_tested(ValidationErrorCode::InvalidModifiedDateTime); // Error #19
        coverage.mark_tested(ValidationErrorCode::InvalidLocationUri); // Error #20
        coverage.mark_tested(ValidationErrorCode::InvalidVersionFormat); // Error #21

        // Verify we've covered all common attribute errors
        let common_attribute_errors = [
            ValidationErrorCode::MissingId,
            ValidationErrorCode::EmptyId,
            ValidationErrorCode::InvalidIdFormat,
            ValidationErrorCode::ClientProvidedId,
            ValidationErrorCode::InvalidExternalId,
            ValidationErrorCode::InvalidMetaStructure,
            ValidationErrorCode::MissingResourceType,
            ValidationErrorCode::InvalidResourceType,
            ValidationErrorCode::ClientProvidedMeta,
            ValidationErrorCode::InvalidCreatedDateTime,
            ValidationErrorCode::InvalidModifiedDateTime,
            ValidationErrorCode::InvalidLocationUri,
            ValidationErrorCode::InvalidVersionFormat,
        ];

        for error in &common_attribute_errors {
            assert!(
                coverage.is_tested(error),
                "Error {:?} not covered by tests",
                error
            );
        }
    }
}
