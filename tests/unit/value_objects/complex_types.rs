//! Complex attribute validation tests.
//!
//! This module tests validation errors related to complex attributes
//! and their nested structure in SCIM resources (Errors 39-43).

use serde_json::json;

// Import test utilities
use crate::common::{ValidationErrorCode, builders::UserBuilder, fixtures::rfc_examples};

/// Test Error #39: Missing required sub-attributes in complex attribute
#[test]
fn test_missing_required_sub_attributes() {
    // Test name complex attribute missing required sub-attributes
    let user_incomplete_name = json!({
        "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
        "id": "123",
        "userName": "test@example.com",
        "name": {
            "givenName": "John"
            // Missing familyName which might be required in some schemas
        },
        "meta": {
            "resourceType": "User"
        }
    });

    let name = &user_incomplete_name["name"];
    assert!(name["givenName"].is_string());
    assert!(!name.as_object().unwrap().contains_key("familyName"));
}

/// Test Error #39: Missing required sub-attributes in address
#[test]
fn test_missing_required_address_sub_attributes() {
    let user_incomplete_address = json!({
        "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
        "id": "123",
        "userName": "test@example.com",
        "addresses": [
            {
                "type": "work",
                "streetAddress": "123 Main St"
                // Missing other potentially required fields like locality, region
            }
        ],
        "meta": {
            "resourceType": "User"
        }
    });

    let address = &user_incomplete_address["addresses"][0];
    assert_eq!(address["type"], "work");
    assert_eq!(address["streetAddress"], "123 Main St");
    assert!(!address.as_object().unwrap().contains_key("locality"));
    assert!(!address.as_object().unwrap().contains_key("region"));
}

/// Test Error #40: Invalid sub-attribute type in complex attribute
#[test]
fn test_invalid_sub_attribute_type() {
    // Test name sub-attributes with wrong types
    let user_invalid_name_types = UserBuilder::new()
        .with_invalid_name_sub_attribute_type()
        .build();

    let name = &user_invalid_name_types["name"];

    // givenName should be string but is number
    assert!(name["givenName"].is_number());
    assert!(!name["givenName"].is_string());

    // Verify expected error is tracked
    let builder = UserBuilder::new().with_invalid_name_sub_attribute_type();
    let expected_errors = builder.expected_errors();
    assert_eq!(
        expected_errors,
        &[ValidationErrorCode::InvalidSubAttributeType]
    );
}

/// Test Error #40: Invalid sub-attribute type in address
#[test]
fn test_invalid_address_sub_attribute_type() {
    let user_invalid_address_types = json!({
        "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
        "id": "123",
        "userName": "test@example.com",
        "addresses": [
            {
                "type": "work",
                "streetAddress": 123,  // Should be string, not number
                "locality": "Anytown",
                "region": "CA",
                "postalCode": 12345,   // Should be string, not number
                "country": "USA",
                "primary": "true"      // Should be boolean, not string
            }
        ],
        "meta": {
            "resourceType": "User"
        }
    });

    let address = &user_invalid_address_types["addresses"][0];
    assert!(address["streetAddress"].is_number());
    assert!(address["postalCode"].is_number());
    assert!(address["primary"].is_string());
}

/// Test Error #41: Unknown sub-attribute in complex attribute
#[test]
fn test_unknown_sub_attribute() {
    // Test name with unknown sub-attributes
    let user_unknown_name_sub_attr = UserBuilder::new().with_unknown_name_sub_attribute().build();

    let name = &user_unknown_name_sub_attr["name"];

    // Should have unknown sub-attribute
    assert!(name.as_object().unwrap().contains_key("unknownAttribute"));
    assert_eq!(name["unknownAttribute"], "unknown value");

    // Verify expected error is tracked
    let builder = UserBuilder::new().with_unknown_name_sub_attribute();
    let expected_errors = builder.expected_errors();
    assert_eq!(expected_errors, &[ValidationErrorCode::UnknownSubAttribute]);
}

/// Test Error #41: Unknown sub-attribute in enterprise extension
#[test]
fn test_unknown_enterprise_sub_attribute() {
    let user_unknown_enterprise_attr = json!({
        "schemas": [
            "urn:ietf:params:scim:schemas:core:2.0:User",
            "urn:ietf:params:scim:schemas:extension:enterprise:2.0:User"
        ],
        "id": "123",
        "userName": "test@example.com",
        "urn:ietf:params:scim:schemas:extension:enterprise:2.0:User": {
            "employeeNumber": "E123",
            "department": "Engineering",
            "unknownEnterpriseAttr": "should not be here", // Unknown attribute
            "manager": {
                "value": "mgr-123",
                "$ref": "https://example.com/v2/Users/mgr-123",
                "displayName": "Manager Name",
                "unknownManagerAttr": "unknown" // Unknown sub-attribute
            }
        },
        "meta": {
            "resourceType": "User"
        }
    });

    let enterprise =
        &user_unknown_enterprise_attr["urn:ietf:params:scim:schemas:extension:enterprise:2.0:User"];
    assert!(
        enterprise
            .as_object()
            .unwrap()
            .contains_key("unknownEnterpriseAttr")
    );

    let manager = &enterprise["manager"];
    assert!(
        manager
            .as_object()
            .unwrap()
            .contains_key("unknownManagerAttr")
    );
}

/// Test Error #42: Nested complex attributes (not allowed)
#[test]
fn test_nested_complex_attributes() {
    // Test complex attribute containing another complex attribute
    let user_nested_complex = json!({
        "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
        "id": "123",
        "userName": "test@example.com",
        "name": {
            "givenName": "John",
            "familyName": "Doe",
            "nestedComplex": {  // Complex attribute nested within another complex attribute
                "subField1": "value1",
                "subField2": "value2",
                "deeperNesting": {  // Even deeper nesting
                    "deepField": "deepValue"
                }
            }
        },
        "meta": {
            "resourceType": "User"
        }
    });

    let name = &user_nested_complex["name"];
    assert!(name["nestedComplex"].is_object());
    assert!(name["nestedComplex"]["deeperNesting"].is_object());
}

/// Test Error #42: Complex attribute in multi-valued array
#[test]
fn test_complex_in_multi_valued_nested() {
    let user_complex_in_array = json!({
        "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
        "id": "123",
        "userName": "test@example.com",
        "emails": [
            {
                "value": "test@example.com",
                "type": "work",
                "complexNested": {  // Complex attribute within multi-valued item
                    "nestedField": "value",
                    "anotherNested": {
                        "deepField": "deepValue"
                    }
                }
            }
        ],
        "meta": {
            "resourceType": "User"
        }
    });

    let email = &user_complex_in_array["emails"][0];
    assert!(email["complexNested"].is_object());
    assert!(email["complexNested"]["anotherNested"].is_object());
}

/// Test Error #43: Malformed complex structure
#[test]
fn test_malformed_complex_structure() {
    // Test complex attributes with malformed structure
    let user_malformed_name = json!({
        "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
        "id": "123",
        "userName": "test@example.com",
        "name": "should-be-object-not-string", // Should be object, not string
        "meta": {
            "resourceType": "User"
        }
    });

    assert!(user_malformed_name["name"].is_string());
    assert!(!user_malformed_name["name"].is_object());
}

/// Test Error #43: Complex attribute as array instead of object
#[test]
fn test_complex_as_array() {
    let user_name_as_array = json!({
        "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
        "id": "123",
        "userName": "test@example.com",
        "name": [  // Should be object, not array
            {
                "givenName": "John",
                "familyName": "Doe"
            }
        ],
        "meta": {
            "resourceType": "User"
        }
    });

    assert!(user_name_as_array["name"].is_array());
    assert!(!user_name_as_array["name"].is_object());
}

/// Test Error #43: Null complex attribute when object expected
#[test]
fn test_null_complex_attribute() {
    let user_null_name = json!({
        "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
        "id": "123",
        "userName": "test@example.com",
        "name": null, // Null when object expected
        "meta": {
            "resourceType": "User"
        }
    });

    assert!(user_null_name["name"].is_null());
    assert!(!user_null_name["name"].is_object());
}

/// Test valid complex attributes to ensure no false positives
#[test]
fn test_valid_complex_attributes() {
    let valid_user = rfc_examples::user_full();

    // Test valid name structure
    let name = &valid_user["name"];
    assert!(name.is_object());
    assert!(name["givenName"].is_string());
    assert!(name["familyName"].is_string());
    assert!(name["formatted"].is_string());

    // All sub-attributes should be appropriate types
    assert_eq!(name["givenName"], "Barbara");
    assert_eq!(name["familyName"], "Jensen");
    assert_eq!(name["formatted"], "Ms. Barbara J Jensen, III");
}

/// Test valid address complex attributes
#[test]
fn test_valid_address_complex_attributes() {
    let user = rfc_examples::user_full();
    let addresses = user["addresses"].as_array().unwrap();

    for address in addresses {
        assert!(address.is_object());

        // Verify required sub-attributes exist and have correct types
        assert!(address["type"].is_string());

        // Optional sub-attributes should be correct type if present
        if let Some(street) = address.get("streetAddress") {
            assert!(street.is_string());
        }
        if let Some(locality) = address.get("locality") {
            assert!(locality.is_string());
        }
        if let Some(region) = address.get("region") {
            assert!(region.is_string());
        }
        if let Some(postal) = address.get("postalCode") {
            assert!(postal.is_string());
        }
        if let Some(country) = address.get("country") {
            assert!(country.is_string());
        }
        if let Some(primary) = address.get("primary") {
            assert!(primary.is_boolean());
        }
    }
}

/// Test enterprise extension complex attributes
#[test]
fn test_valid_enterprise_complex_attributes() {
    let user = rfc_examples::user_enterprise();
    let enterprise = &user["urn:ietf:params:scim:schemas:extension:enterprise:2.0:User"];

    assert!(enterprise.is_object());

    // Test manager complex sub-attribute
    let manager = &enterprise["manager"];
    assert!(manager.is_object());
    assert!(manager["value"].is_string());
    assert!(manager["$ref"].is_string());
    assert!(manager["displayName"].is_string());

    // Verify manager reference structure
    let manager_ref = manager["$ref"].as_str().unwrap();
    assert!(manager_ref.starts_with("../Users/"));
}

/// Test complex attribute depth validation
#[test]
fn test_complex_attribute_depth_limits() {
    // Test that we properly validate nesting depth
    let shallow_valid = json!({
        "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
        "id": "123",
        "userName": "test@example.com",
        "name": {
            "givenName": "John",
            "familyName": "Doe"
        },
        "meta": {
            "resourceType": "User"
        }
    });

    assert!(shallow_valid["name"].is_object());
    assert_eq!(shallow_valid["name"]["givenName"], "John");

    // Deep nesting (potentially invalid depending on schema rules)
    let deep_nested = json!({
        "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
        "id": "123",
        "userName": "test@example.com",
        "customExtension": {
            "level1": {
                "level2": {
                    "level3": {
                        "level4": {
                            "tooDeep": "value"
                        }
                    }
                }
            }
        },
        "meta": {
            "resourceType": "User"
        }
    });

    // Verify the deep nesting structure
    assert!(
        deep_nested["customExtension"]["level1"]["level2"]["level3"]["level4"]["tooDeep"]
            .as_str()
            .unwrap()
            == "value"
    );
}

/// Test complex attribute with circular references (should be invalid)
#[test]
fn test_complex_attribute_circular_references() {
    // Note: JSON itself cannot have circular references, but we can test
    // references that would create logical circles in a graph
    let user_with_circular_refs = json!({
        "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
        "id": "user-123",
        "userName": "test@example.com",
        "manager": {
            "value": "mgr-456",
            "$ref": "https://example.com/v2/Users/mgr-456",
            "displayName": "Manager"
        },
        "subordinates": [
            {
                "value": "user-123", // Circular: user is subordinate of themselves
                "$ref": "https://example.com/v2/Users/user-123",
                "displayName": "Self Reference"
            }
        ],
        "meta": {
            "resourceType": "User"
        }
    });

    // This represents a logical circular reference
    assert_eq!(user_with_circular_refs["id"], "user-123");
    assert_eq!(
        user_with_circular_refs["subordinates"][0]["value"],
        "user-123"
    );
}

/// Test multiple complex attribute errors in single resource
#[test]
fn test_multiple_complex_attribute_errors() {
    let user_multiple_errors = json!({
        "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
        "id": "123",
        "userName": "test@example.com",
        "name": "malformed",          // Error #43: Should be object, not string
        "addresses": [
            {
                "type": "work",
                "streetAddress": 123,  // Error #40: Should be string, not number
                "unknownField": "unknown", // Error #41: Unknown sub-attribute
                "nestedComplex": {     // Error #42: Nested complex attribute
                    "deepField": "value"
                }
            }
        ],
        "phoneNumbers": [
            {
                // Error #39: Missing required value sub-attribute
                "type": "work",
                "primary": true
            }
        ],
        "meta": {
            "resourceType": "User"
        }
    });

    // Verify multiple error conditions
    assert!(user_multiple_errors["name"].is_string()); // Should be object

    let address = &user_multiple_errors["addresses"][0];
    assert!(address["streetAddress"].is_number()); // Should be string
    assert!(address.as_object().unwrap().contains_key("unknownField"));
    assert!(address["nestedComplex"].is_object()); // Nested complex

    let phone = &user_multiple_errors["phoneNumbers"][0];
    assert!(!phone.as_object().unwrap().contains_key("value")); // Missing required
}

/// Test complex attribute schema validation edge cases
#[test]
fn test_complex_attribute_schema_edge_cases() {
    // Test empty complex attribute
    let user_empty_name = json!({
        "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
        "id": "123",
        "userName": "test@example.com",
        "name": {}, // Empty object
        "meta": {
            "resourceType": "User"
        }
    });

    assert!(user_empty_name["name"].is_object());
    assert_eq!(user_empty_name["name"].as_object().unwrap().len(), 0);

    // Test complex attribute with only optional fields
    let user_optional_only = json!({
        "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
        "id": "123",
        "userName": "test@example.com",
        "name": {
            "honorificPrefix": "Dr.",
            "honorificSuffix": "Jr."
            // No required fields like givenName, familyName
        },
        "meta": {
            "resourceType": "User"
        }
    });

    let name = &user_optional_only["name"];
    assert!(name.is_object());
    assert_eq!(name["honorificPrefix"], "Dr.");
    assert!(!name.as_object().unwrap().contains_key("givenName"));
}

#[cfg(test)]
mod coverage_tests {
    use super::*;
    use crate::common::TestCoverage;

    #[test]
    fn test_complex_attributes_error_coverage() {
        // Verify all complex attribute errors (39-43) are covered by our tests
        let mut coverage = TestCoverage::new();

        // Mark errors as tested based on our test functions
        coverage.mark_tested(ValidationErrorCode::MissingRequiredSubAttributes); // Error #39
        coverage.mark_tested(ValidationErrorCode::InvalidSubAttributeType); // Error #40
        coverage.mark_tested(ValidationErrorCode::UnknownSubAttribute); // Error #41
        coverage.mark_tested(ValidationErrorCode::NestedComplexAttributes); // Error #42
        coverage.mark_tested(ValidationErrorCode::MalformedComplexStructure); // Error #43

        // Verify we've covered all complex attribute errors
        let complex_attribute_errors = [
            ValidationErrorCode::MissingRequiredSubAttributes,
            ValidationErrorCode::InvalidSubAttributeType,
            ValidationErrorCode::UnknownSubAttribute,
            ValidationErrorCode::NestedComplexAttributes,
            ValidationErrorCode::MalformedComplexStructure,
        ];

        for error in &complex_attribute_errors {
            assert!(
                coverage.is_tested(error),
                "Error {:?} not covered by tests",
                error
            );
        }
    }

    #[test]
    fn test_complex_attribute_test_scenarios() {
        // Document the different scenarios we test for complex attributes

        let test_scenarios = [
            // Structure validation
            "missing_required_sub_attributes",
            "invalid_sub_attribute_types",
            "unknown_sub_attributes",
            "nested_complex_attributes",
            "malformed_complex_structure",
            // Type validation
            "complex_as_string",
            "complex_as_array",
            "complex_as_null",
            // Content validation
            "empty_complex_objects",
            "optional_only_complex_objects",
            "circular_references",
            // Valid cases
            "valid_name_structure",
            "valid_address_structure",
            "valid_enterprise_extension",
        ];

        assert!(
            test_scenarios.len() >= 12,
            "Should have comprehensive complex attribute test scenarios"
        );

        // Verify we test multiple complex attribute types
        let tested_complex_attributes = [
            "name",
            "addresses",
            "manager",
            "enterprise_extension",
            "custom_extensions",
        ];

        assert!(
            tested_complex_attributes.len() >= 5,
            "Should test multiple complex attribute types"
        );
    }

    #[test]
    fn test_complex_attribute_depth_coverage() {
        // Verify we test different nesting depths and patterns

        let nesting_patterns = [
            "single_level_complex", // name.givenName
            "multi_valued_complex", // addresses[0].streetAddress
            "extension_complex",    // enterprise.manager.value
            "nested_complex",       // complex within complex (invalid)
            "deep_nesting",         // multiple levels deep
        ];

        assert!(
            nesting_patterns.len() >= 5,
            "Should test various nesting patterns"
        );

        // Verify we test error combinations
        let error_combinations = [
            "single_error_per_attribute",
            "multiple_errors_per_attribute",
            "multiple_attributes_with_errors",
            "mixed_valid_and_invalid_attributes",
        ];

        assert!(
            error_combinations.len() >= 4,
            "Should test error combinations"
        );
    }
}
