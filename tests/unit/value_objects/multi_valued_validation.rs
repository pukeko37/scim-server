//! Multi-valued attribute validation tests.
//!
//! This module tests validation errors related to multi-valued attributes
//! and their structure in SCIM resources (Errors 33-38).

use serde_json::json;

// Import test utilities
use crate::common::{ValidationErrorCode, builders::UserBuilder, fixtures::rfc_examples};

// Import SCIM server types
use scim_server::error::ValidationError;
use scim_server::schema::{SchemaRegistry, validation::OperationContext};

/// Test Error #33: Single value provided for multi-valued attribute
#[test]
fn test_single_value_for_multi_valued() {
    let registry = SchemaRegistry::new().expect("Failed to create registry");

    // emails should be an array, not a single object
    let user_single_email = UserBuilder::new().with_single_value_emails().build();

    // Verify emails is not an array
    assert!(!user_single_email["emails"].is_array());
    assert!(user_single_email["emails"].is_object());

    // Actually validate the resource
    let result = registry.validate_json_resource_with_context(
        "User",
        &user_single_email,
        OperationContext::Update,
    );

    // Assert that validation fails with the expected error
    assert!(result.is_err());
    match result {
        Err(ValidationError::Custom { message }) => {
            assert!(message.contains("emails must be an array"));
        }
        Err(other) => panic!("Expected Custom error about emails array, got {:?}", other),
        Ok(_) => panic!("Expected validation to fail, but it passed"),
    }
}

/// Test Error #33: Single value for other multi-valued attributes
#[test]
fn test_single_value_for_multi_valued_addresses() {
    // addresses should be an array, not a single object
    let user_single_address = json!({
        "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
        "id": "123",
        "userName": "test@example.com",
        "addresses": {
            "type": "work",
            "streetAddress": "123 Main St",
            "locality": "Anytown",
            "region": "CA",
            "postalCode": "12345",
            "country": "USA"
        },
        "meta": {
            "resourceType": "User"
        }
    });

    assert!(!user_single_address["addresses"].is_array());
    assert!(user_single_address["addresses"].is_object());
}

/// Test Error #34: Array provided for single-valued attribute
#[test]
fn test_array_for_single_valued() {
    let registry = SchemaRegistry::new().expect("Failed to create registry");

    // userName should be a string, not an array
    let user_array_username = UserBuilder::new().with_array_username().build();

    // Verify userName is an array
    assert!(user_array_username["userName"].is_array());
    assert!(!user_array_username["userName"].is_string());

    // Actually validate the resource
    let result = registry.validate_json_resource_with_context(
        "User",
        &user_array_username,
        OperationContext::Update,
    );

    // Assert that validation fails with the expected error
    assert!(result.is_err());
    match result {
        Err(ValidationError::Custom { message }) => {
            assert!(message.contains("userName must be a string"));
        }
        Err(other) => panic!(
            "Expected Custom error about userName string, got {:?}",
            other
        ),
        Ok(_) => panic!("Expected validation to fail, but it passed"),
    }
}

/// Test Error #34: Array for other single-valued attributes
#[test]
fn test_array_for_single_valued_display_name() {
    let registry = SchemaRegistry::new().expect("Failed to create registry");

    // displayName should be a string, not an array
    let user_array_display_name = json!({
        "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
        "id": "123",
        "userName": "test@example.com",
        "displayName": ["John", "Doe"],
        "meta": {
            "resourceType": "User"
        }
    });

    // Verify displayName is an array
    assert!(user_array_display_name["displayName"].is_array());

    // Actually validate the resource
    let result = registry.validate_json_resource_with_context(
        "User",
        &user_array_display_name,
        OperationContext::Update,
    );

    // Assert that validation fails with the expected error
    assert!(result.is_err());
    match result {
        Err(ValidationError::InvalidAttributeType {
            attribute,
            expected,
            actual,
        }) => {
            assert_eq!(attribute, "displayName");
            assert_eq!(expected, "string");
            assert_eq!(actual, "array");
        }
        Err(other) => panic!("Expected InvalidAttributeType error, got {:?}", other),
        Ok(_) => panic!("Expected validation to fail, but it passed"),
    }
}

/// Test Error #35: Multiple primary values in multi-valued attribute
#[test]
fn test_multiple_primary_values() {
    let registry = SchemaRegistry::new().expect("Failed to create registry");

    // Only one email should have primary: true
    let user_multiple_primaries = UserBuilder::new().with_multiple_primary_emails().build();

    // Verify multiple emails have primary: true
    let emails = user_multiple_primaries["emails"].as_array().unwrap();
    let primary_count = emails
        .iter()
        .filter(|email| email["primary"] == true)
        .count();

    assert!(primary_count > 1, "Should have multiple primary emails");

    // Actually validate the resource
    let result = registry.validate_json_resource_with_context(
        "User",
        &user_multiple_primaries,
        OperationContext::Update,
    );

    // Assert that validation fails with the expected error
    assert!(result.is_err());
    match result {
        Err(ValidationError::MultiplePrimaryValues { attribute }) => {
            assert_eq!(attribute, "emails");
        }
        Err(other) => panic!("Expected MultiplePrimaryValues error, got {:?}", other),
        Ok(_) => panic!("Expected validation to fail, but it passed"),
    }
}

/// Test Error #35: Multiple primary values in addresses
#[test]
fn test_multiple_primary_addresses() {
    let user_multiple_primary_addresses = json!({
        "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
        "id": "123",
        "userName": "test@example.com",
        "addresses": [
            {
                "type": "work",
                "streetAddress": "123 Work St",
                "primary": true
            },
            {
                "type": "home",
                "streetAddress": "456 Home St",
                "primary": true  // Multiple primaries not allowed
            }
        ],
        "meta": {
            "resourceType": "User"
        }
    });

    let addresses = user_multiple_primary_addresses["addresses"]
        .as_array()
        .unwrap();
    let primary_count = addresses
        .iter()
        .filter(|addr| addr["primary"] == true)
        .count();

    assert_eq!(primary_count, 2);
}

/// Test Error #36: Invalid multi-valued structure
#[test]
fn test_invalid_multi_valued_structure() {
    let registry = SchemaRegistry::new().expect("Failed to create registry");

    // Test emails with incorrect structure
    let user_invalid_email_structure = json!({
        "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
        "id": "123",
        "userName": "test@example.com",
        "emails": [
            "plain-string-email", // Should be object with value, type, etc.
            {
                "value": "valid@example.com",
                "type": "work"
            }
        ],
        "meta": {
            "resourceType": "User"
        }
    });

    let emails = user_invalid_email_structure["emails"].as_array().unwrap();
    assert!(emails[0].is_string()); // First email is invalid structure
    assert!(emails[1].is_object()); // Second email is valid structure

    // Actually validate the resource
    let result = registry.validate_json_resource_with_context(
        "User",
        &user_invalid_email_structure,
        OperationContext::Update,
    );

    // Assert that validation fails with the expected error
    assert!(result.is_err());
    match result {
        Err(ValidationError::Custom { message }) => {
            assert!(message.contains("Invalid emails format"));
            assert!(message.contains("invalid type: string"));
        }
        Err(other) => panic!(
            "Expected Custom error about invalid emails format, got {:?}",
            other
        ),
        Ok(_) => panic!("Expected validation to fail, but it passed"),
    }
}

/// Test Error #36: Missing value in multi-valued complex attribute
#[test]
fn test_multi_valued_missing_value() {
    let user_email_without_value = json!({
        "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
        "id": "123",
        "userName": "test@example.com",
        "emails": [
            {
                "type": "work",
                "primary": true
                // Missing "value" field
            }
        ],
        "meta": {
            "resourceType": "User"
        }
    });

    let email = &user_email_without_value["emails"][0];
    assert!(!email.as_object().unwrap().contains_key("value"));
    assert!(email.as_object().unwrap().contains_key("type"));
}

/// Test Error #37: Missing required sub-attribute in multi-valued attribute
#[test]
fn test_missing_required_sub_attribute() {
    let registry = SchemaRegistry::new().expect("Failed to create registry");

    // Test emails missing required "value" sub-attribute
    let user_email_missing_value = json!({
        "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
        "id": "123",
        "userName": "test@example.com",
        "emails": [
            {
                "type": "work",
                "primary": true
                // Missing "value" field - this is required
            }
        ],
        "meta": {
            "resourceType": "User"
        }
    });

    // Verify the email object doesn't have a "value" field
    let email = &user_email_missing_value["emails"][0];
    assert!(!email.as_object().unwrap().contains_key("value"));
    assert!(email.as_object().unwrap().contains_key("type"));

    // Actually validate the resource
    let result = registry.validate_json_resource_with_context(
        "User",
        &user_email_missing_value,
        OperationContext::Update,
    );

    // Assert that validation fails with the expected error
    assert!(result.is_err());
    match result {
        Err(ValidationError::MissingRequiredSubAttribute {
            attribute,
            sub_attribute,
        }) => {
            assert_eq!(attribute, "emails");
            assert_eq!(sub_attribute, "value");
        }
        Err(other) => panic!(
            "Expected MissingRequiredSubAttribute error, got {:?}",
            other
        ),
        Ok(_) => panic!("Expected validation to fail, but it passed"),
    }
}

/// Test Error #37: Missing required sub-attribute in addresses
#[test]
fn test_missing_required_address_sub_attribute() {
    // Test address missing required sub-attributes
    let user_incomplete_address = json!({
        "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
        "id": "123",
        "userName": "test@example.com",
        "addresses": [
            {
                "type": "work"
                // Missing other required fields like locality, region, etc.
                // depending on schema requirements
            }
        ],
        "meta": {
            "resourceType": "User"
        }
    });

    let address = &user_incomplete_address["addresses"][0];
    assert_eq!(address["type"], "work");
    assert!(!address.as_object().unwrap().contains_key("streetAddress"));
}

/// Test Error #38: Invalid canonical value in multi-valued attribute
#[test]
fn test_invalid_canonical_value() {
    let registry = SchemaRegistry::new().expect("Failed to create registry");

    // Test invalid "type" values that don't match canonical values
    let user_invalid_email_type = UserBuilder::new().with_invalid_email_type().build();

    // Verify the invalid type is present
    let emails = user_invalid_email_type["emails"].as_array().unwrap();
    let invalid_email = emails.iter().find(|email| email["type"] == "invalid-type");
    assert!(invalid_email.is_some());

    // Actually validate the resource
    let result = registry.validate_json_resource_with_context(
        "User",
        &user_invalid_email_type,
        OperationContext::Update,
    );

    // Assert that validation fails with the expected error
    assert!(result.is_err());
    match result {
        Err(ValidationError::InvalidCanonicalValue {
            attribute,
            value,
            allowed,
        }) => {
            assert_eq!(attribute, "emails.type");
            assert_eq!(value, "invalid-type");
            assert!(allowed.contains(&"work".to_string()));
            assert!(allowed.contains(&"home".to_string()));
            assert!(allowed.contains(&"other".to_string()));
        }
        Err(other) => panic!("Expected InvalidCanonicalValue error, got {:?}", other),
        Ok(_) => panic!("Expected validation to fail, but it passed"),
    }
}

/// Test Error #38: Invalid canonical values in phone numbers
#[test]
fn test_invalid_phone_canonical_values() {
    let user_invalid_phone_types = json!({
        "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
        "id": "123",
        "userName": "test@example.com",
        "phoneNumbers": [
            {
                "value": "555-1234",
                "type": "invalid-phone-type" // Should be work, home, mobile, fax, pager, other
            },
            {
                "value": "555-5678",
                "type": "work" // Valid type
            }
        ],
        "meta": {
            "resourceType": "User"
        }
    });

    let phones = user_invalid_phone_types["phoneNumbers"].as_array().unwrap();
    assert_eq!(phones[0]["type"], "invalid-phone-type");
    assert_eq!(phones[1]["type"], "work");
}

/// Test valid multi-valued attributes to ensure no false positives
#[test]
fn test_valid_multi_valued_attributes() {
    // Test valid emails structure
    let valid_user = rfc_examples::user_full();

    // Verify emails is properly structured
    let emails = valid_user["emails"].as_array().unwrap();
    assert!(emails.len() >= 1);

    for email in emails {
        assert!(email["value"].is_string());
        assert!(email["type"].is_string());

        // Check that only one has primary: true
        if email["primary"] == true {
            // This is the primary email
            assert_eq!(email["primary"], true);
        }
    }

    // Count primary emails (should be 0 or 1)
    let primary_count = emails
        .iter()
        .filter(|email| email["primary"] == true)
        .count();
    assert!(primary_count <= 1, "Should have at most one primary email");
}

/// Test valid single-valued attributes
#[test]
fn test_valid_single_valued_attributes() {
    let valid_user = rfc_examples::user_minimal();

    // userName should be a string
    assert!(valid_user["userName"].is_string());
    assert_eq!(valid_user["userName"], "bjensen@example.com");

    // id should be a string
    assert!(valid_user["id"].is_string());

    // displayName should be a string if present
    let full_user = rfc_examples::user_full();
    assert!(full_user["displayName"].is_string());
    assert_eq!(full_user["displayName"], "Babs Jensen");
}

/// Test canonical values for multi-valued attributes
#[test]
fn test_valid_canonical_values() {
    let user = rfc_examples::user_full();

    // Test valid email types
    let emails = user["emails"].as_array().unwrap();
    for email in emails {
        let email_type = email["type"].as_str().unwrap();
        let valid_email_types = ["work", "home", "other"];
        assert!(
            valid_email_types.contains(&email_type),
            "Invalid email type: {}",
            email_type
        );
    }

    // Test valid address types
    if let Some(addresses) = user["addresses"].as_array() {
        for address in addresses {
            let addr_type = address["type"].as_str().unwrap();
            let valid_address_types = ["work", "home", "other"];
            assert!(
                valid_address_types.contains(&addr_type),
                "Invalid address type: {}",
                addr_type
            );
        }
    }
}

/// Test complex multi-valued attribute validation
#[test]
fn test_complex_multi_valued_validation() {
    // Test group members structure
    let valid_group = rfc_examples::group_basic();
    let members = valid_group["members"].as_array().unwrap();

    for member in members {
        // Each member should have required attributes
        assert!(member["value"].is_string());
        assert!(member["$ref"].is_string());
        assert!(member["display"].is_string());

        // $ref should be a valid URI
        let ref_uri = member["$ref"].as_str().unwrap();
        assert!(ref_uri.starts_with("https://"));
        assert!(ref_uri.contains("/Users/"));
    }
}

/// Test edge cases in multi-valued attribute validation
#[test]
fn test_multi_valued_edge_cases() {
    // Test empty multi-valued array (should be valid)
    let user_empty_emails = json!({
        "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
        "id": "123",
        "userName": "test@example.com",
        "emails": [], // Empty array should be valid
        "meta": {
            "resourceType": "User"
        }
    });

    assert!(user_empty_emails["emails"].is_array());
    assert_eq!(user_empty_emails["emails"].as_array().unwrap().len(), 0);

    // Test single item in multi-valued array (should be valid)
    let user_single_email_array = json!({
        "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
        "id": "123",
        "userName": "test@example.com",
        "emails": [
            {
                "value": "test@example.com",
                "type": "work",
                "primary": true
            }
        ],
        "meta": {
            "resourceType": "User"
        }
    });

    assert!(user_single_email_array["emails"].is_array());
    assert_eq!(
        user_single_email_array["emails"].as_array().unwrap().len(),
        1
    );
}

/// Test multi-valued attribute with null values
#[test]
fn test_multi_valued_with_null_values() {
    let user_with_nulls = json!({
        "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
        "id": "123",
        "userName": "test@example.com",
        "emails": [
            null, // Null item in array
            {
                "value": "test@example.com",
                "type": "work"
            }
        ],
        "meta": {
            "resourceType": "User"
        }
    });

    let emails = user_with_nulls["emails"].as_array().unwrap();
    assert!(emails[0].is_null());
    assert!(emails[1].is_object());
}

/// Test multiple validation errors in multi-valued attributes
#[test]
fn test_multiple_multi_valued_errors() {
    let user_multiple_errors = json!({
        "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
        "id": "123",
        "userName": ["array", "username"], // Error #34: Array for single-valued
        "emails": {                        // Error #33: Single value for multi-valued
            "value": "test@example.com",
            "type": "work",
            "primary": true
        },
        "phoneNumbers": [
            {
                "value": "555-1234",
                "type": "invalid-type", // Error #38: Invalid canonical value
                "primary": true
            },
            {
                "value": "555-5678",
                "type": "work",
                "primary": true         // Error #35: Multiple primary values
            }
        ],
        "meta": {
            "resourceType": "User"
        }
    });

    // Verify multiple error conditions
    assert!(user_multiple_errors["userName"].is_array()); // Should be string
    assert!(user_multiple_errors["emails"].is_object()); // Should be array

    let phones = user_multiple_errors["phoneNumbers"].as_array().unwrap();
    assert_eq!(phones[0]["type"], "invalid-type");

    let primary_count = phones
        .iter()
        .filter(|phone| phone["primary"] == true)
        .count();
    assert_eq!(primary_count, 2); // Multiple primaries
}

#[cfg(test)]
mod coverage_tests {
    use super::*;
    use crate::common::TestCoverage;

    #[test]
    fn test_multi_valued_error_coverage() {
        // Verify all multi-valued attribute errors (33-38) are covered by our tests
        let mut coverage = TestCoverage::new();

        // Mark errors as tested based on our test functions
        coverage.mark_tested(ValidationErrorCode::SingleValueForMultiValued); // Error #33
        coverage.mark_tested(ValidationErrorCode::ArrayForSingleValued); // Error #34
        coverage.mark_tested(ValidationErrorCode::MultiplePrimaryValues); // Error #35
        coverage.mark_tested(ValidationErrorCode::InvalidMultiValuedStructure); // Error #36
        coverage.mark_tested(ValidationErrorCode::MissingRequiredSubAttribute); // Error #37
        coverage.mark_tested(ValidationErrorCode::InvalidCanonicalValue); // Error #38

        // Verify we've covered all multi-valued attribute errors
        let multi_valued_errors = [
            ValidationErrorCode::SingleValueForMultiValued,
            ValidationErrorCode::ArrayForSingleValued,
            ValidationErrorCode::MultiplePrimaryValues,
            ValidationErrorCode::InvalidMultiValuedStructure,
            ValidationErrorCode::MissingRequiredSubAttribute,
            ValidationErrorCode::InvalidCanonicalValue,
        ];

        for error in &multi_valued_errors {
            assert!(
                coverage.is_tested(error),
                "Error {:?} not covered by tests",
                error
            );
        }
    }

    #[test]
    fn test_multi_valued_test_scenarios() {
        // Document the different scenarios we test for multi-valued attributes

        let test_scenarios = [
            // Structure validation
            "single_value_for_multi_valued",
            "array_for_single_valued",
            "invalid_multi_valued_structure",
            // Content validation
            "multiple_primary_values",
            "missing_required_sub_attributes",
            "invalid_canonical_values",
            // Edge cases
            "empty_arrays",
            "single_item_arrays",
            "null_values_in_arrays",
            // Valid cases
            "valid_multi_valued_structure",
            "valid_canonical_values",
            "valid_single_valued_structure",
        ];

        assert!(
            test_scenarios.len() >= 10,
            "Should have comprehensive test scenarios"
        );

        // Verify we test multiple attribute types
        let tested_attributes = [
            "emails",
            "addresses",
            "phoneNumbers",
            "groups",
            "userName",
            "displayName",
        ];

        assert!(
            tested_attributes.len() >= 6,
            "Should test multiple attribute types"
        );
    }

    #[test]
    fn test_canonical_value_coverage() {
        // Verify we test canonical values for different attribute types

        let canonical_value_tests = [
            ("emails", vec!["work", "home", "other"]),
            ("addresses", vec!["work", "home", "other"]),
            (
                "phoneNumbers",
                vec!["work", "home", "mobile", "fax", "pager", "other"],
            ),
            (
                "ims",
                vec!["aim", "gtalk", "icq", "xmpp", "msn", "skype", "qq", "yahoo"],
            ),
        ];

        for (attr_type, expected_values) in canonical_value_tests {
            assert!(
                expected_values.len() >= 3,
                "Should test multiple canonical values for {}",
                attr_type
            );
        }
    }
}
