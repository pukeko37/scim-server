//! Common test utilities for SCIM validation testing.
//!
//! This module provides macros, builders, and utilities to support comprehensive
//! validation testing across all SCIM validation error types.

use serde_json::{Value, json};

pub mod builders;
pub mod fixtures;

/// Validation error codes for tracking test coverage
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ValidationErrorCode {
    // Schema Structure Validation Errors (1-8)
    MissingSchemas,           // Error #1
    EmptySchemas,             // Error #2
    InvalidSchemaUri,         // Error #3
    UnknownSchemaUri,         // Error #4
    DuplicateSchemaUri,       // Error #5
    MissingBaseSchema,        // Error #6
    ExtensionWithoutBase,     // Error #7
    MissingRequiredExtension, // Error #8

    // Common Attribute Validation Errors (9-21)
    MissingId,               // Error #9
    EmptyId,                 // Error #10
    InvalidIdFormat,         // Error #11
    ClientProvidedId,        // Error #12
    InvalidExternalId,       // Error #13
    InvalidMetaStructure,    // Error #14
    MissingResourceType,     // Error #15
    InvalidResourceType,     // Error #16
    ClientProvidedMeta,      // Error #17
    InvalidCreatedDateTime,  // Error #18
    InvalidModifiedDateTime, // Error #19
    InvalidLocationUri,      // Error #20
    InvalidVersionFormat,    // Error #21

    // Attribute Type Validation Errors (22-32)
    MissingRequiredAttribute, // Error #22
    InvalidDataType,          // Error #23
    InvalidStringFormat,      // Error #24
    InvalidBooleanValue,      // Error #25
    InvalidDecimalFormat,     // Error #26
    InvalidIntegerValue,      // Error #27
    InvalidDateTimeFormat,    // Error #28
    InvalidBinaryData,        // Error #29
    InvalidReferenceUri,      // Error #30
    InvalidReferenceType,     // Error #31
    BrokenReference,          // Error #32

    // Multi-valued Attribute Validation Errors (33-38)
    SingleValueForMultiValued,   // Error #33
    ArrayForSingleValued,        // Error #34
    MultiplePrimaryValues,       // Error #35
    InvalidMultiValuedStructure, // Error #36
    MissingRequiredSubAttribute, // Error #37
    InvalidCanonicalValue,       // Error #38

    // Complex Attribute Validation Errors (39-43)
    MissingRequiredSubAttributes, // Error #39
    InvalidSubAttributeType,      // Error #40
    UnknownSubAttribute,          // Error #41
    NestedComplexAttributes,      // Error #42
    MalformedComplexStructure,    // Error #43

    // Attribute Characteristics Validation Errors (44-52)
    CaseSensitivityViolation,        // Error #44
    ReadOnlyMutabilityViolation,     // Error #45
    ImmutableMutabilityViolation,    // Error #46
    WriteOnlyAttributeReturned,      // Error #47
    ServerUniquenessViolation,       // Error #48
    GlobalUniquenessViolation,       // Error #49
    InvalidCanonicalValueChoice,     // Error #50
    UnknownAttributeForSchema,       // Error #51
    RequiredCharacteristicViolation, // Error #52
}

/// Custom assertion macro for validation errors
#[macro_export]
macro_rules! assert_validation_error {
    ($result:expr, $expected_error:expr) => {
        match $result {
            Err(ScimError::Validation(_)) => {
                // Error occurred as expected
            }
            Ok(_) => panic!(
                "Expected validation error {:?}, but validation passed",
                $expected_error
            ),
            Err(other) => panic!(
                "Expected validation error {:?}, got {:?}",
                $expected_error, other
            ),
        }
    };
}

/// Custom assertion macro for specific error messages
#[macro_export]
macro_rules! assert_error_message_contains {
    ($result:expr, $substring:expr) => {
        match $result {
            Err(err) => assert!(
                err.to_string().contains($substring),
                "Error message '{}' does not contain '{}'",
                err.to_string(),
                $substring
            ),
            Ok(_) => panic!(
                "Expected error containing '{}', but validation passed",
                $substring
            ),
        }
    };
}

/// Custom assertion macro for successful validation
#[macro_export]
macro_rules! assert_validation_success {
    ($result:expr) => {
        match $result {
            Ok(_) => {
                // Success as expected
            }
            Err(err) => panic!("Expected validation to succeed, but got error: {}", err),
        }
    };
}

/// Custom assertion macro for specific validation error types
#[macro_export]
macro_rules! assert_specific_validation_error {
    ($result:expr, $error_variant:pat) => {
        match $result {
            Err(ScimError::Validation($error_variant)) => {
                // Specific error type matched
            }
            Ok(_) => panic!("Expected validation error, but validation passed"),
            Err(other) => panic!("Expected specific validation error, got {:?}", other),
        }
    };
}

/// Test coverage tracker to ensure all validation errors are tested
#[derive(Debug, Default)]
pub struct TestCoverage {
    covered_errors: std::collections::HashSet<ValidationErrorCode>,
}

impl TestCoverage {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn mark_tested(&mut self, error: ValidationErrorCode) {
        self.covered_errors.insert(error);
    }

    pub fn is_tested(&self, error: &ValidationErrorCode) -> bool {
        self.covered_errors.contains(error)
    }

    pub fn coverage_percentage(&self) -> f64 {
        let total_errors = Self::total_validation_errors();
        if total_errors == 0 {
            100.0
        } else {
            (self.covered_errors.len() as f64 / total_errors as f64) * 100.0
        }
    }

    pub fn covered_errors(&self) -> &std::collections::HashSet<ValidationErrorCode> {
        &self.covered_errors
    }

    pub fn untested_errors(&self) -> Vec<ValidationErrorCode> {
        Self::all_validation_errors()
            .into_iter()
            .filter(|error| !self.covered_errors.contains(error))
            .collect()
    }

    pub fn total_validation_errors() -> usize {
        // Total count of validation errors we're tracking
        52 // Errors 1-52 in our current implementation
    }

    fn all_validation_errors() -> Vec<ValidationErrorCode> {
        use ValidationErrorCode::*;
        vec![
            // Schema Structure (1-8)
            MissingSchemas,
            EmptySchemas,
            InvalidSchemaUri,
            UnknownSchemaUri,
            DuplicateSchemaUri,
            MissingBaseSchema,
            ExtensionWithoutBase,
            MissingRequiredExtension,
            // Common Attributes (9-21)
            MissingId,
            EmptyId,
            InvalidIdFormat,
            ClientProvidedId,
            InvalidExternalId,
            InvalidMetaStructure,
            MissingResourceType,
            InvalidResourceType,
            ClientProvidedMeta,
            InvalidCreatedDateTime,
            InvalidModifiedDateTime,
            InvalidLocationUri,
            InvalidVersionFormat,
            // Attribute Types (22-32)
            MissingRequiredAttribute,
            InvalidDataType,
            InvalidStringFormat,
            InvalidBooleanValue,
            InvalidDecimalFormat,
            InvalidIntegerValue,
            InvalidDateTimeFormat,
            InvalidBinaryData,
            InvalidReferenceUri,
            InvalidReferenceType,
            BrokenReference,
            // Multi-valued Attributes (33-38)
            SingleValueForMultiValued,
            ArrayForSingleValued,
            MultiplePrimaryValues,
            InvalidMultiValuedStructure,
            MissingRequiredSubAttribute,
            InvalidCanonicalValue,
            // Complex Attributes (39-43)
            MissingRequiredSubAttributes,
            InvalidSubAttributeType,
            UnknownSubAttribute,
            NestedComplexAttributes,
            MalformedComplexStructure,
            // Attribute Characteristics (44-52)
            CaseSensitivityViolation,
            ReadOnlyMutabilityViolation,
            ImmutableMutabilityViolation,
            WriteOnlyAttributeReturned,
            ServerUniquenessViolation,
            GlobalUniquenessViolation,
            InvalidCanonicalValueChoice,
            UnknownAttributeForSchema,
            RequiredCharacteristicViolation,
        ]
    }
}

/// Helper function to load test fixtures
pub fn load_fixture(path: &str) -> Value {
    let fixture_path = format!("tests/fixtures/{}", path);
    let content = std::fs::read_to_string(&fixture_path)
        .unwrap_or_else(|_| panic!("Failed to load fixture: {}", fixture_path));
    serde_json::from_str(&content)
        .unwrap_or_else(|_| panic!("Failed to parse JSON fixture: {}", fixture_path))
}

/// Helper function to create a basic valid user resource for testing
pub fn valid_user_minimal() -> Value {
    json!({
        "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
        "id": "2819c223-7f76-453a-919d-413861904646",
        "userName": "bjensen@example.com",
        "meta": {
            "resourceType": "User",
            "created": "2010-01-23T04:56:22Z",
            "lastModified": "2011-05-13T04:42:34Z",
            "version": "W/\"3694e05e9dff590\"",
            "location": "https://example.com/v2/Users/2819c223-7f76-453a-919d-413861904646"
        }
    })
}

/// Helper function to create a basic valid group resource for testing
pub fn valid_group_minimal() -> Value {
    json!({
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
    })
}

/// Helper to create test data with specified modifications
pub fn modify_json(mut base: Value, path: &str, new_value: Option<Value>) -> Value {
    let path_parts: Vec<&str> = path.split('.').collect();

    if path_parts.len() == 1 {
        match new_value {
            Some(value) => {
                base[path_parts[0]] = value;
            }
            None => {
                if let Some(obj) = base.as_object_mut() {
                    obj.remove(path_parts[0]);
                }
            }
        }
    } else {
        // For nested paths, we'd need more complex logic
        // For now, just handle simple cases
        if path_parts.len() == 2 {
            match new_value {
                Some(value) => {
                    base[path_parts[0]][path_parts[1]] = value;
                }
                None => {
                    if let Some(obj) = base[path_parts[0]].as_object_mut() {
                        obj.remove(path_parts[1]);
                    }
                }
            }
        }
    }

    base
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_coverage_tracker() {
        let mut coverage = TestCoverage::new();
        assert_eq!(coverage.coverage_percentage(), 0.0);

        coverage.mark_tested(ValidationErrorCode::MissingSchemas);
        assert!(coverage.is_tested(&ValidationErrorCode::MissingSchemas));
        assert!(!coverage.is_tested(&ValidationErrorCode::EmptySchemas));

        let untested = coverage.untested_errors();
        assert!(!untested.contains(&ValidationErrorCode::MissingSchemas));
        assert!(untested.contains(&ValidationErrorCode::EmptySchemas));
    }

    #[test]
    fn test_valid_user_minimal() {
        let user = valid_user_minimal();
        assert_eq!(
            user["schemas"][0],
            "urn:ietf:params:scim:schemas:core:2.0:User"
        );
        assert_eq!(user["userName"], "bjensen@example.com");
    }

    #[test]
    fn test_modify_json() {
        let base = json!({"name": "test", "nested": {"value": 42}});

        // Remove a field
        let modified = modify_json(base.clone(), "name", None);
        assert!(!modified.as_object().unwrap().contains_key("name"));

        // Modify a field
        let modified = modify_json(base.clone(), "name", Some(json!("new_value")));
        assert_eq!(modified["name"], "new_value");

        // Modify nested field
        let modified = modify_json(base.clone(), "nested.value", Some(json!(100)));
        assert_eq!(modified["nested"]["value"], 100);
    }
}
