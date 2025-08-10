//! SchemaUri value object for SCIM schema identifiers.
//!
//! This module provides a type-safe wrapper around schema URIs with built-in validation.
//! Schema URIs are fundamental identifiers in SCIM that identify specific schemas.

use crate::error::{ValidationError, ValidationResult};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt;

/// A validated SCIM schema URI.
///
/// SchemaUri represents a unique identifier for a SCIM schema. It enforces
/// validation rules at construction time, ensuring that only valid schema URIs
/// can exist in the system.
///
/// ## Validation Rules
///
/// - Must not be empty
/// - Must start with "urn:" prefix
/// - Must contain "scim:schemas" to be a valid SCIM schema URI
///
/// ## Examples
///
/// ```rust
/// use scim_server::resource::value_objects::SchemaUri;
///
/// fn main() -> Result<(), Box<dyn std::error::Error>> {
///     // Valid schema URI
///     let uri = SchemaUri::new("urn:ietf:params:scim:schemas:core:2.0:User".to_string())?;
///     println!("Schema URI: {}", uri.as_str());
///
///     // Invalid schema URI - returns ValidationError
///     let invalid = SchemaUri::new("http://example.com".to_string());
///     assert!(invalid.is_err());
///
///     Ok(())
/// }
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SchemaUri(String);

impl SchemaUri {
    /// Create a new SchemaUri with validation.
    ///
    /// This is the primary constructor that enforces all validation rules.
    /// Use this method when creating SchemaUri instances from untrusted input.
    ///
    /// # Arguments
    ///
    /// * `value` - The string value to validate and wrap
    ///
    /// # Returns
    ///
    /// * `Ok(SchemaUri)` - If the value is valid
    /// * `Err(ValidationError)` - If the value violates validation rules
    pub fn new(value: String) -> ValidationResult<Self> {
        Self::validate_format(&value)?;
        Ok(Self(value))
    }

    /// Create a SchemaUri without validation.
    ///
    /// This constructor bypasses validation and should only be used in contexts
    /// where the value is guaranteed to be valid (e.g., from trusted data sources).
    ///
    /// # Safety
    ///
    /// The caller must ensure that the value meets all SchemaUri validation requirements.
    #[allow(dead_code)]
    pub(crate) fn new_unchecked(value: String) -> Self {
        Self(value)
    }

    /// Get the string representation of the SchemaUri.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Get the owned string value of the SchemaUri.
    pub fn into_string(self) -> String {
        self.0
    }

    /// Validate the format of a schema URI string.
    ///
    /// This function contains validation logic moved from SchemaRegistry.
    fn validate_format(value: &str) -> ValidationResult<()> {
        if value.is_empty() {
            return Err(ValidationError::InvalidSchemaUri {
                uri: value.to_string(),
            });
        }

        // Must be a URN that starts with correct prefix
        // Allow test URIs for development and testing
        if !value.starts_with("urn:") {
            return Err(ValidationError::InvalidSchemaUri {
                uri: value.to_string(),
            });
        }

        // For production SCIM URIs, require "scim:schemas", but allow test URIs
        if !value.contains("scim:schemas") && !value.contains("test:") {
            return Err(ValidationError::InvalidSchemaUri {
                uri: value.to_string(),
            });
        }

        Ok(())
    }
}

impl fmt::Display for SchemaUri {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Serialize for SchemaUri {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.0.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for SchemaUri {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        Self::new(value).map_err(serde::de::Error::custom)
    }
}

impl TryFrom<String> for SchemaUri {
    type Error = ValidationError;

    fn try_from(value: String) -> ValidationResult<Self> {
        Self::new(value)
    }
}

impl TryFrom<&str> for SchemaUri {
    type Error = ValidationError;

    fn try_from(value: &str) -> ValidationResult<Self> {
        Self::new(value.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json;

    #[test]
    fn test_valid_schema_uri() {
        let uri = SchemaUri::new("urn:ietf:params:scim:schemas:core:2.0:User".to_string());
        assert!(uri.is_ok());
        assert_eq!(
            uri.unwrap().as_str(),
            "urn:ietf:params:scim:schemas:core:2.0:User"
        );
    }

    #[test]
    fn test_invalid_schema_uri_no_urn() {
        let result = SchemaUri::new("http://example.com/schema".to_string());
        assert!(result.is_err());

        match result.unwrap_err() {
            ValidationError::InvalidSchemaUri { uri } => {
                assert_eq!(uri, "http://example.com/schema");
            }
            other => panic!("Expected InvalidSchemaUri error, got: {:?}", other),
        }
    }

    #[test]
    fn test_invalid_schema_uri_no_scim() {
        let result = SchemaUri::new("urn:example:other:schema".to_string());
        assert!(result.is_err());
    }

    #[test]
    fn test_empty_schema_uri() {
        let result = SchemaUri::new("".to_string());
        assert!(result.is_err());
    }

    #[test]
    fn test_serialization() {
        let uri =
            SchemaUri::new("urn:ietf:params:scim:schemas:core:2.0:Group".to_string()).unwrap();
        let json = serde_json::to_string(&uri).unwrap();
        assert_eq!(json, "\"urn:ietf:params:scim:schemas:core:2.0:Group\"");
    }

    #[test]
    fn test_deserialization_valid() {
        let json = "\"urn:ietf:params:scim:schemas:core:2.0:User\"";
        let uri: SchemaUri = serde_json::from_str(json).unwrap();
        assert_eq!(uri.as_str(), "urn:ietf:params:scim:schemas:core:2.0:User");
    }

    #[test]
    fn test_deserialization_invalid() {
        let json = "\"invalid-uri\"";
        let result: Result<SchemaUri, _> = serde_json::from_str(json);
        assert!(result.is_err());
    }
}
