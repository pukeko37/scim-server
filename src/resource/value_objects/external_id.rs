//! ExternalId value object for SCIM external identifiers.
//!
//! This module provides a type-safe wrapper around external IDs with built-in validation.
//! External IDs are optional identifiers that link SCIM resources to external systems.

use crate::error::{ValidationError, ValidationResult};
use crate::resource::value_objects::value_object_trait::{SchemaConstructible, ValueObject};
use crate::schema::types::{AttributeDefinition, AttributeType};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_json::Value;
use std::any::Any;
use std::fmt;

/// A validated SCIM external identifier.
///
/// ExternalId represents an optional identifier that links a SCIM resource to an
/// external system. It enforces validation rules at construction time, ensuring
/// that only valid external IDs can exist in the system.
///
/// ## Validation Rules
///
/// - Must not be empty if provided
/// - Must be a valid string
/// - Null values are handled separately (`Option<ExternalId>`)
///
/// ## Examples
///
/// ```rust
/// use scim_server::resource::value_objects::ExternalId;
///
/// fn main() -> Result<(), Box<dyn std::error::Error>> {
///     // Valid external ID
///     let ext_id = ExternalId::new("701984".to_string())?;
///     println!("External ID: {}", ext_id.as_str());
///
///     // Invalid external ID - returns ValidationError
///     let invalid = ExternalId::new("".to_string()); // Error
///     assert!(invalid.is_err());
///
///     Ok(())
/// }
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ExternalId(String);

impl ExternalId {
    /// Create a new ExternalId with validation.
    ///
    /// This is the primary constructor that enforces all validation rules.
    /// Use this method when creating ExternalId instances from untrusted input.
    ///
    /// # Arguments
    ///
    /// * `value` - The string value to validate and wrap
    ///
    /// # Returns
    ///
    /// * `Ok(ExternalId)` - If the value is valid
    /// * `Err(ValidationError)` - If the value violates validation rules
    pub fn new(value: String) -> ValidationResult<Self> {
        Self::validate_format(&value)?;
        Ok(Self(value))
    }

    /// Create an ExternalId without validation.
    ///
    /// This constructor bypasses validation and should only be used in contexts
    /// where the value is guaranteed to be valid (e.g., from trusted data sources).
    ///
    /// # Safety
    ///
    /// The caller must ensure that the value meets all ExternalId validation requirements.
    #[allow(dead_code)]
    pub(crate) fn new_unchecked(value: String) -> Self {
        Self(value)
    }

    /// Get the string representation of the ExternalId.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Get the owned string value of the ExternalId.
    pub fn into_string(self) -> String {
        self.0
    }

    /// Validate the format of an external ID string.
    ///
    /// This function contains validation logic moved from SchemaRegistry.
    fn validate_format(value: &str) -> ValidationResult<()> {
        // External ID should not be empty if provided
        if value.is_empty() {
            return Err(ValidationError::InvalidExternalId);
        }

        // TODO: Add more sophisticated external ID format validation if needed
        // For now, we accept any non-empty string as a valid external ID
        // Future enhancements might include:
        // - Character set restrictions
        // - Length limits
        // - Format-specific validation for different external systems

        Ok(())
    }
}

impl fmt::Display for ExternalId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Serialize for ExternalId {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.0.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for ExternalId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        Self::new(value).map_err(serde::de::Error::custom)
    }
}

impl TryFrom<String> for ExternalId {
    type Error = ValidationError;

    fn try_from(value: String) -> ValidationResult<Self> {
        Self::new(value)
    }
}

impl TryFrom<&str> for ExternalId {
    type Error = ValidationError;

    fn try_from(value: &str) -> ValidationResult<Self> {
        Self::new(value.to_string())
    }
}

impl ValueObject for ExternalId {
    fn attribute_type(&self) -> AttributeType {
        AttributeType::String
    }

    fn attribute_name(&self) -> &str {
        "externalId"
    }

    fn to_json(&self) -> ValidationResult<Value> {
        Ok(Value::String(self.0.clone()))
    }

    fn validate_against_schema(&self, definition: &AttributeDefinition) -> ValidationResult<()> {
        if definition.data_type != AttributeType::String {
            return Err(ValidationError::InvalidAttributeType {
                attribute: definition.name.clone(),
                expected: "string".to_string(),
                actual: format!("{:?}", definition.data_type),
            });
        }

        if definition.name != "externalId" {
            return Err(ValidationError::InvalidAttributeName {
                actual: definition.name.clone(),
                expected: "externalId".to_string(),
            });
        }

        Ok(())
    }

    fn as_json_value(&self) -> Value {
        Value::String(self.0.clone())
    }

    fn supports_definition(&self, definition: &AttributeDefinition) -> bool {
        definition.data_type == AttributeType::String && definition.name == "externalId"
    }

    fn clone_boxed(&self) -> Box<dyn ValueObject> {
        Box::new(self.clone())
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl SchemaConstructible for ExternalId {
    fn from_schema_and_value(
        definition: &AttributeDefinition,
        value: &Value,
    ) -> ValidationResult<Self> {
        if definition.name != "externalId" || definition.data_type != AttributeType::String {
            return Err(ValidationError::UnsupportedAttributeType {
                attribute: definition.name.clone(),
                type_name: format!("{:?}", definition.data_type),
            });
        }

        if let Some(ext_id_str) = value.as_str() {
            Self::new(ext_id_str.to_string())
        } else {
            Err(ValidationError::InvalidAttributeType {
                attribute: definition.name.clone(),
                expected: "string".to_string(),
                actual: "non-string".to_string(),
            })
        }
    }

    fn can_construct_from(definition: &AttributeDefinition) -> bool {
        definition.name == "externalId" && definition.data_type == AttributeType::String
    }

    fn constructor_priority() -> u8 {
        100 // High priority for exact name match
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json;

    #[test]
    fn test_valid_external_id() {
        let ext_id = ExternalId::new("701984".to_string());
        assert!(ext_id.is_ok());
        assert_eq!(ext_id.unwrap().as_str(), "701984");
    }

    #[test]
    fn test_valid_external_id_alphanumeric() {
        let ext_id = ExternalId::new("EXT-123-ABC".to_string());
        assert!(ext_id.is_ok());
        assert_eq!(ext_id.unwrap().as_str(), "EXT-123-ABC");
    }

    #[test]
    fn test_empty_external_id() {
        let result = ExternalId::new("".to_string());
        assert!(result.is_err());

        match result.unwrap_err() {
            ValidationError::InvalidExternalId => {} // Expected
            other => panic!("Expected InvalidExternalId error, got: {:?}", other),
        }
    }

    #[test]
    fn test_new_unchecked() {
        let ext_id = ExternalId::new_unchecked("unchecked-ext-id".to_string());
        assert_eq!(ext_id.as_str(), "unchecked-ext-id");
    }

    #[test]
    fn test_into_string() {
        let ext_id = ExternalId::new("test-ext-id".to_string()).unwrap();
        let string_value = ext_id.into_string();
        assert_eq!(string_value, "test-ext-id");
    }

    #[test]
    fn test_display() {
        let ext_id = ExternalId::new("display-test".to_string()).unwrap();
        assert_eq!(format!("{}", ext_id), "display-test");
    }

    #[test]
    fn test_serialization() {
        let ext_id = ExternalId::new("serialize-test".to_string()).unwrap();
        let json = serde_json::to_string(&ext_id).unwrap();
        assert_eq!(json, "\"serialize-test\"");
    }

    #[test]
    fn test_deserialization_valid() {
        let json = "\"deserialize-test\"";
        let ext_id: ExternalId = serde_json::from_str(json).unwrap();
        assert_eq!(ext_id.as_str(), "deserialize-test");
    }

    #[test]
    fn test_deserialization_invalid() {
        let json = "\"\""; // Empty string
        let result: Result<ExternalId, _> = serde_json::from_str(json);
        assert!(result.is_err());
    }

    #[test]
    fn test_try_from_string() {
        let result = ExternalId::try_from("try-from-test".to_string());
        assert!(result.is_ok());
        assert_eq!(result.unwrap().as_str(), "try-from-test");

        let empty_result = ExternalId::try_from("".to_string());
        assert!(empty_result.is_err());
    }

    #[test]
    fn test_try_from_str() {
        let result = ExternalId::try_from("try-from-str-test");
        assert!(result.is_ok());
        assert_eq!(result.unwrap().as_str(), "try-from-str-test");

        let empty_result = ExternalId::try_from("");
        assert!(empty_result.is_err());
    }

    #[test]
    fn test_equality() {
        let ext_id1 = ExternalId::new("same-ext-id".to_string()).unwrap();
        let ext_id2 = ExternalId::new("same-ext-id".to_string()).unwrap();
        let ext_id3 = ExternalId::new("different-ext-id".to_string()).unwrap();

        assert_eq!(ext_id1, ext_id2);
        assert_ne!(ext_id1, ext_id3);
    }

    #[test]
    fn test_hash() {
        use std::collections::HashMap;

        let ext_id1 = ExternalId::new("hash-test-1".to_string()).unwrap();
        let ext_id2 = ExternalId::new("hash-test-2".to_string()).unwrap();

        let mut map = HashMap::new();
        map.insert(ext_id1.clone(), "value1");
        map.insert(ext_id2.clone(), "value2");

        assert_eq!(map.get(&ext_id1), Some(&"value1"));
        assert_eq!(map.get(&ext_id2), Some(&"value2"));
    }

    #[test]
    fn test_clone() {
        let ext_id = ExternalId::new("clone-test".to_string()).unwrap();
        let cloned = ext_id.clone();

        assert_eq!(ext_id, cloned);
        assert_eq!(ext_id.as_str(), cloned.as_str());
    }
}
