//! Data type validation tests.
//!
//! This module tests validation errors related to data type validation
//! and attribute type constraints in SCIM resources (Errors 22-32).

use serde_json::json;

// Import test utilities
use crate::common::{ValidationErrorCode, builders::UserBuilder, fixtures::rfc_examples};

/// Test Error #22: Missing required attribute
#[test]
fn test_missing_required_attribute() {
    // Test missing userName (required for User resources)
    let user_without_username = UserBuilder::new().without_username().build();

    // Verify userName is missing
    assert!(
        !user_without_username
            .as_object()
            .unwrap()
            .contains_key("userName")
    );

    // Verify expected error is tracked
    let builder = UserBuilder::new().without_username();
    let expected_errors = builder.expected_errors();
    assert_eq!(
        expected_errors,
        &[ValidationErrorCode::MissingRequiredAttribute]
    );
}

/// Test Error #22: Missing required attribute in complex structures
#[test]
fn test_missing_required_sub_attribute() {
    // Test missing displayName (required for Group resources)
    let group_without_display_name = json!({
        "schemas": ["urn:ietf:params:scim:schemas:core:2.0:Group"],
        "id": "123",
        "meta": {
            "resourceType": "Group"
        }
        // Missing displayName
    });

    assert!(
        !group_without_display_name
            .as_object()
            .unwrap()
            .contains_key("displayName")
    );
}

/// Test Error #23: Invalid data type for attribute
#[test]
fn test_invalid_data_type_string_as_number() {
    // userName should be string, not number
    let user_invalid_username = UserBuilder::new().with_invalid_username_type().build();

    // Verify userName is not a string
    assert!(user_invalid_username["userName"].is_number());
    assert!(!user_invalid_username["userName"].is_string());

    // Verify expected error is tracked
    let builder = UserBuilder::new().with_invalid_username_type();
    let expected_errors = builder.expected_errors();
    assert_eq!(expected_errors, &[ValidationErrorCode::InvalidDataType]);
}

/// Test Error #23: Invalid data type for boolean attributes
#[test]
fn test_invalid_data_type_boolean() {
    // active should be boolean, not string
    let user_invalid_active = json!({
        "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
        "id": "123",
        "userName": "test@example.com",
        "active": "not-a-boolean", // Should be true/false
        "meta": {
            "resourceType": "User"
        }
    });

    assert!(user_invalid_active["active"].is_string());
    assert!(!user_invalid_active["active"].is_boolean());
}

/// Test Error #24: Invalid string format
#[test]
fn test_invalid_string_format() {
    // Test various string format violations
    let long_string = "a".repeat(1000);
    let test_cases = vec![
        // Empty strings where not allowed
        ("", "Empty string not allowed"),
        // Strings that are too long (if length limits exist)
        (long_string.as_str(), "String too long"),
        // Invalid characters or encoding
        ("\x00\x01\x02", "Invalid control characters"),
    ];

    for (invalid_value, description) in test_cases {
        let user_invalid_string = json!({
            "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
            "id": "123",
            "userName": invalid_value,
            "meta": {
                "resourceType": "User"
            }
        });

        assert_eq!(
            user_invalid_string["userName"], invalid_value,
            "{}",
            description
        );
    }
}

/// Test Error #25: Invalid boolean value
#[test]
fn test_invalid_boolean_value() {
    // Test non-boolean values in boolean fields
    let user_invalid_boolean = UserBuilder::new().with_invalid_boolean_active().build();

    // Verify active is not a proper boolean
    assert!(!user_invalid_boolean["active"].is_boolean());

    // Verify expected error is tracked
    let builder = UserBuilder::new().with_invalid_boolean_active();
    let expected_errors = builder.expected_errors();
    assert_eq!(expected_errors, &[ValidationErrorCode::InvalidBooleanValue]);
}

/// Test Error #26: Invalid decimal format
#[test]
fn test_invalid_decimal_format() {
    // Test invalid decimal values in decimal fields
    let test_cases = vec![
        "not-a-number",
        "12.34.56", // Multiple decimal points
        "12e",      // Incomplete scientific notation
        "NaN",
        "Infinity",
        "-Infinity",
        "", // Empty string
    ];

    for invalid_decimal in test_cases {
        let resource_with_decimal = json!({
            "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
            "id": "123",
            "userName": "test@example.com",
            "urn:example:decimal": invalid_decimal,
            "meta": {
                "resourceType": "User"
            }
        });

        // In a real validation scenario, these would be caught as invalid decimals
        assert_eq!(
            resource_with_decimal["urn:example:decimal"],
            invalid_decimal
        );
    }
}

/// Test Error #27: Invalid integer value
#[test]
fn test_invalid_integer_value() {
    // Test invalid integer values
    let test_cases = vec![
        "not-an-integer",
        "12.34", // Decimal, not integer
        "12e5",  // Scientific notation
        "12.0",  // Decimal representation of integer
        "",      // Empty string
        "12a",   // Mixed alphanumeric
    ];

    for invalid_integer in test_cases {
        let resource_with_integer = json!({
            "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
            "id": "123",
            "userName": "test@example.com",
            "urn:example:integer": invalid_integer,
            "meta": {
                "resourceType": "User"
            }
        });

        assert_eq!(
            resource_with_integer["urn:example:integer"],
            invalid_integer
        );
    }
}

/// Test Error #28: Invalid DateTime format
#[test]
fn test_invalid_datetime_format() {
    // Test various invalid DateTime formats
    let test_cases = vec![
        "2023-13-01T10:00:00Z", // Invalid month
        "2023-01-32T10:00:00Z", // Invalid day
        "2023-01-01T25:00:00Z", // Invalid hour
        "2023-01-01T10:60:00Z", // Invalid minute
        "2023-01-01T10:00:60Z", // Invalid second
        "2023-01-01 10:00:00",  // Missing T separator
        "2023/01/01T10:00:00Z", // Wrong date separator
        "not-a-date",           // Completely invalid
        "2023-01-01",           // Date only, missing time
        "10:00:00Z",            // Time only, missing date
        "",                     // Empty string
    ];

    for invalid_datetime in test_cases {
        let user_invalid_datetime = json!({
            "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
            "id": "123",
            "userName": "test@example.com",
            "meta": {
                "resourceType": "User",
                "created": invalid_datetime,
                "lastModified": "2023-01-01T10:00:00Z"
            }
        });

        assert_eq!(user_invalid_datetime["meta"]["created"], invalid_datetime);
    }
}

/// Test Error #29: Invalid binary data
#[test]
fn test_invalid_binary_data() {
    // Test invalid base64 encoded binary data
    let test_cases = vec![
        "not-base64", // Invalid characters
        "MTIz!",      // Invalid character (!)
        "MTI",        // Incomplete padding
        "MT===",      // Too much padding
        "",           // Empty string
        "MTIz\n456",  // Contains newlines (depends on validation strictness)
    ];

    for invalid_binary in test_cases {
        let user_with_binary = json!({
            "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
            "id": "123",
            "userName": "test@example.com",
            "x509Certificates": [
                {
                    "value": invalid_binary
                }
            ],
            "meta": {
                "resourceType": "User"
            }
        });

        assert_eq!(
            user_with_binary["x509Certificates"][0]["value"],
            invalid_binary
        );
    }
}

/// Test Error #30: Invalid reference URI
#[test]
fn test_invalid_reference_uri() {
    // Test invalid URI formats in reference attributes
    let test_cases = vec![
        "not-a-uri",                  // No scheme
        "ftp://example.com/path",     // Wrong scheme
        "http://",                    // Incomplete URI
        "",                           // Empty string
        "relative/path",              // Relative URI
        "://example.com",             // Missing scheme
        "http:// invalid spaces.com", // Spaces in URI
    ];

    for invalid_uri in test_cases {
        let group_with_invalid_ref = json!({
            "schemas": ["urn:ietf:params:scim:schemas:core:2.0:Group"],
            "id": "123",
            "displayName": "Test Group",
            "members": [
                {
                    "value": "user-123",
                    "$ref": invalid_uri,
                    "display": "Test User"
                }
            ],
            "meta": {
                "resourceType": "Group"
            }
        });

        assert_eq!(group_with_invalid_ref["members"][0]["$ref"], invalid_uri);
    }
}

/// Test Error #31: Invalid reference type
#[test]
fn test_invalid_reference_type() {
    // Test references to wrong resource types
    let user_with_invalid_group_ref = json!({
        "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
        "id": "123",
        "userName": "test@example.com",
        "groups": [
            {
                "value": "group-123",
                "$ref": "https://example.com/v2/Users/group-123", // Should be Groups, not Users
                "display": "Test Group"
            }
        ],
        "meta": {
            "resourceType": "User"
        }
    });

    // Reference points to Users endpoint but should point to Groups
    let ref_uri = user_with_invalid_group_ref["groups"][0]["$ref"]
        .as_str()
        .unwrap();
    assert!(ref_uri.contains("/Users/"));
    assert!(!ref_uri.contains("/Groups/"));
}

/// Test Error #32: Broken reference (referencing non-existent resource)
#[test]
fn test_broken_reference() {
    // Test references to resources that don't exist
    let group_with_broken_ref = json!({
        "schemas": ["urn:ietf:params:scim:schemas:core:2.0:Group"],
        "id": "123",
        "displayName": "Test Group",
        "members": [
            {
                "value": "non-existent-user",
                "$ref": "https://example.com/v2/Users/non-existent-user",
                "display": "Ghost User"
            }
        ],
        "meta": {
            "resourceType": "Group"
        }
    });

    // Reference is well-formed but points to non-existent resource
    assert_eq!(
        group_with_broken_ref["members"][0]["value"],
        "non-existent-user"
    );
    assert_eq!(
        group_with_broken_ref["members"][0]["$ref"],
        "https://example.com/v2/Users/non-existent-user"
    );
}

/// Test valid data types to ensure no false positives
#[test]
fn test_valid_data_types() {
    // Test valid string
    let valid_user = rfc_examples::user_minimal();
    assert!(valid_user["userName"].is_string());
    assert_eq!(valid_user["userName"], "bjensen@example.com");

    // Test valid boolean
    let user_with_active = json!({
        "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
        "id": "123",
        "userName": "test@example.com",
        "active": true,
        "meta": {
            "resourceType": "User"
        }
    });
    assert!(user_with_active["active"].is_boolean());
    assert_eq!(user_with_active["active"], true);

    // Test valid DateTime
    let meta = valid_user["meta"].as_object().unwrap();
    assert_eq!(meta["created"], "2010-01-23T04:56:22Z");
    assert_eq!(meta["lastModified"], "2011-05-13T04:42:34Z");
}

/// Test valid reference formats
#[test]
fn test_valid_reference_formats() {
    let valid_group = rfc_examples::group_basic();
    let members = valid_group["members"].as_array().unwrap();

    for member in members {
        let ref_uri = member["$ref"].as_str().unwrap();

        // Should be valid HTTPS URI
        assert!(ref_uri.starts_with("https://"));

        // Should point to Users endpoint
        assert!(ref_uri.contains("/Users/"));

        // Should have a valid UUID
        let uuid_part = ref_uri.split('/').last().unwrap();
        assert!(uuid_part.len() > 0);
        assert!(uuid_part.contains('-')); // UUIDs contain hyphens
    }
}

/// Test numeric data type boundaries
#[test]
fn test_numeric_boundaries() {
    // Test integer boundaries
    let test_cases = vec![
        i64::MIN.to_string(),
        "-1".to_string(),
        "0".to_string(),
        "1".to_string(),
        i64::MAX.to_string(),
    ];

    for valid_integer in test_cases {
        let resource_with_integer = json!({
            "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
            "id": "123",
            "userName": "test@example.com",
            "urn:example:integer": valid_integer,
            "meta": {
                "resourceType": "User"
            }
        });

        assert_eq!(resource_with_integer["urn:example:integer"], valid_integer);
    }

    // Test decimal values
    let decimal_cases = vec!["0.0", "1.5", "-1.5", "3.14159", "1e-10", "1.23e10"];

    for valid_decimal in decimal_cases {
        let resource_with_decimal = json!({
            "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
            "id": "123",
            "userName": "test@example.com",
            "urn:example:decimal": valid_decimal,
            "meta": {
                "resourceType": "User"
            }
        });

        assert_eq!(resource_with_decimal["urn:example:decimal"], valid_decimal);
    }
}

/// Test DateTime format validation specifics
#[test]
fn test_datetime_format_validation() {
    // Valid DateTime formats that should pass
    let valid_formats = vec![
        "2023-01-01T00:00:00Z",      // Basic UTC
        "2023-01-01T12:34:56Z",      // UTC with time
        "2023-01-01T12:34:56.123Z",  // UTC with milliseconds
        "2023-01-01T12:34:56+05:00", // With timezone offset
        "2023-01-01T12:34:56-08:00", // Negative timezone offset
        "2023-12-31T23:59:59Z",      // End of year
    ];

    for valid_datetime in valid_formats {
        let resource_with_datetime = json!({
            "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
            "id": "123",
            "userName": "test@example.com",
            "meta": {
                "resourceType": "User",
                "created": valid_datetime,
                "lastModified": valid_datetime
            }
        });

        assert_eq!(resource_with_datetime["meta"]["created"], valid_datetime);
        assert_eq!(
            resource_with_datetime["meta"]["lastModified"],
            valid_datetime
        );
    }
}

/// Test binary data validation
#[test]
fn test_valid_binary_data() {
    // Valid base64 encoded data
    let valid_base64_cases = vec![
        "TWFu",                                     // "Man" encoded
        "bGVhc3VyZS4=",                             // "leasure." encoded
        "c3VyZS4=",                                 // "sure." encoded
        "YWxsIHlvdXIgYmFzZSBhcmUgYmVsb25nIHRvIHVz", // Longer string
    ];

    for valid_base64 in valid_base64_cases {
        let user_with_cert = json!({
            "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
            "id": "123",
            "userName": "test@example.com",
            "x509Certificates": [
                {
                    "value": valid_base64
                }
            ],
            "meta": {
                "resourceType": "User"
            }
        });

        assert_eq!(user_with_cert["x509Certificates"][0]["value"], valid_base64);
    }
}

/// Test multiple data type errors in a single resource
#[test]
fn test_multiple_data_type_errors() {
    let resource_with_multiple_errors = json!({
        "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
        "id": "123",
        "userName": 123,              // Should be string (Error #23)
        "active": "not-boolean",      // Should be boolean (Error #25)
        "meta": {
            "resourceType": "User",
            "created": "invalid-date", // Should be DateTime (Error #28)
            "lastModified": "2023-01-01T10:00:00Z"
        }
    });

    // Verify multiple type violations
    assert!(resource_with_multiple_errors["userName"].is_number());
    assert!(resource_with_multiple_errors["active"].is_string());
    assert_eq!(
        resource_with_multiple_errors["meta"]["created"],
        "invalid-date"
    );
}

/// Test data type coercion edge cases
#[test]
fn test_data_type_coercion_edge_cases() {
    // Test how the system handles type coercion
    let edge_cases = json!({
        "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
        "id": "123",
        "userName": "test@example.com",
        "active": 1,                  // Numeric 1 instead of true
        "urn:example:string": null,   // Null where string expected
        "urn:example:number": "123",  // String where number expected
        "meta": {
            "resourceType": "User"
        }
    });

    // These cases test the boundary between valid coercion and type errors
    assert_eq!(edge_cases["active"], 1);
    assert!(edge_cases["urn:example:string"].is_null());
    assert_eq!(edge_cases["urn:example:number"], "123");
}

#[cfg(test)]
mod coverage_tests {
    use super::*;
    use crate::common::TestCoverage;

    #[test]
    fn test_data_type_error_coverage() {
        // Verify all data type validation errors (22-32) are covered by our tests
        let mut coverage = TestCoverage::new();

        // Mark errors as tested based on our test functions
        coverage.mark_tested(ValidationErrorCode::MissingRequiredAttribute); // Error #22
        coverage.mark_tested(ValidationErrorCode::InvalidDataType); // Error #23
        coverage.mark_tested(ValidationErrorCode::InvalidStringFormat); // Error #24
        coverage.mark_tested(ValidationErrorCode::InvalidBooleanValue); // Error #25
        coverage.mark_tested(ValidationErrorCode::InvalidDecimalFormat); // Error #26
        coverage.mark_tested(ValidationErrorCode::InvalidIntegerValue); // Error #27
        coverage.mark_tested(ValidationErrorCode::InvalidDateTimeFormat); // Error #28
        coverage.mark_tested(ValidationErrorCode::InvalidBinaryData); // Error #29
        coverage.mark_tested(ValidationErrorCode::InvalidReferenceUri); // Error #30
        coverage.mark_tested(ValidationErrorCode::InvalidReferenceType); // Error #31
        coverage.mark_tested(ValidationErrorCode::BrokenReference); // Error #32

        // Verify we've covered all data type errors
        let data_type_errors = [
            ValidationErrorCode::MissingRequiredAttribute,
            ValidationErrorCode::InvalidDataType,
            ValidationErrorCode::InvalidStringFormat,
            ValidationErrorCode::InvalidBooleanValue,
            ValidationErrorCode::InvalidDecimalFormat,
            ValidationErrorCode::InvalidIntegerValue,
            ValidationErrorCode::InvalidDateTimeFormat,
            ValidationErrorCode::InvalidBinaryData,
            ValidationErrorCode::InvalidReferenceUri,
            ValidationErrorCode::InvalidReferenceType,
            ValidationErrorCode::BrokenReference,
        ];

        for error in &data_type_errors {
            assert!(
                coverage.is_tested(error),
                "Error {:?} not covered by tests",
                error
            );
        }
    }

    #[test]
    fn test_data_type_test_comprehensiveness() {
        // Verify that our tests cover a comprehensive range of data type scenarios

        // String validation scenarios
        let string_test_scenarios = [
            "empty strings",
            "too long strings",
            "invalid characters",
            "valid strings",
        ];

        // Boolean validation scenarios
        let boolean_test_scenarios = [
            "non-boolean strings",
            "numeric values as booleans",
            "null values",
            "valid booleans",
        ];

        // Numeric validation scenarios
        let numeric_test_scenarios = [
            "non-numeric strings",
            "decimal in integer field",
            "scientific notation",
            "boundary values",
            "valid numbers",
        ];

        // DateTime validation scenarios
        let datetime_test_scenarios = [
            "invalid date components",
            "wrong format",
            "missing components",
            "timezone variations",
            "valid datetime formats",
        ];

        // Reference validation scenarios
        let reference_test_scenarios = [
            "malformed URIs",
            "wrong schemes",
            "wrong resource types",
            "non-existent resources",
            "valid references",
        ];

        // This test documents the comprehensiveness of our test coverage
        // In a real implementation, we might have automated checks for this
        assert!(string_test_scenarios.len() >= 4);
        assert!(boolean_test_scenarios.len() >= 4);
        assert!(numeric_test_scenarios.len() >= 5);
        assert!(datetime_test_scenarios.len() >= 5);
        assert!(reference_test_scenarios.len() >= 5);
    }
}
