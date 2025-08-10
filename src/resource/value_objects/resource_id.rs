//! ResourceId value object for SCIM resource identifiers.
//!
//! This module provides a type-safe wrapper around resource IDs with built-in validation.
//! Resource IDs are fundamental identifiers in SCIM and must follow specific format rules.

use crate::error::{ValidationError, ValidationResult};
use crate::resource::value_objects::value_object_trait::{SchemaConstructible, ValueObject};
use crate::schema::types::{AttributeDefinition, AttributeType};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_json::Value;
use std::any::Any;
use std::fmt;

/// A validated SCIM resource identifier.
///
/// ResourceId represents a unique identifier for a SCIM resource. It enforces
/// validation rules at construction time, ensuring that only valid resource IDs
/// can exist in the system.
///
/// ## Validation Rules
///
/// - Must not be empty
/// - Must be a valid string
/// - Additional format rules may be added in the future
///
/// ## Examples
///
/// ```rust
/// use scim_server::resource::value_objects::ResourceId;
///
/// fn main() -> Result<(), Box<dyn std::error::Error>> {
///     // Valid resource ID
///     let id = ResourceId::new("2819c223-7f76-453a-919d-413861904646".to_string())?;
///     println!("Resource ID: {}", id.as_str());
///
///     // Invalid resource ID - returns ValidationError
///     let invalid = ResourceId::new("".to_string());
///     assert!(invalid.is_err());
///
///     Ok(())
/// }
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ResourceId(String);

impl ResourceId {
    /// Create a new ResourceId with validation.
    ///
    /// This is the primary constructor that enforces all validation rules.
    /// Use this method when creating ResourceId instances from untrusted input.
    ///
    /// # Arguments
    ///
    /// * `value` - The string value to validate and wrap
    ///
    /// # Returns
    ///
    /// * `Ok(ResourceId)` - If the value is valid
    /// * `Err(ValidationError)` - If the value violates validation rules
    ///
    /// # Examples
    ///
    /// ```rust
    /// use scim_server::resource::value_objects::ResourceId;
    ///
    /// fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let id = ResourceId::new("user-123".to_string())?;
    ///     let empty_id = ResourceId::new("".to_string()); // Error
    ///     assert!(empty_id.is_err());
    ///     Ok(())
    /// }
    /// ```
    pub fn new(value: String) -> ValidationResult<Self> {
        Self::validate_format(&value)?;
        Ok(Self(value))
    }

    /// Create a ResourceId without validation.
    ///
    /// This constructor bypasses validation and should only be used in contexts
    /// where the value is guaranteed to be valid (e.g., from trusted data sources
    /// like databases where validation has already occurred).
    ///
    /// # Safety
    ///
    /// The caller must ensure that the value meets all ResourceId validation requirements.
    /// Using this with invalid data may lead to inconsistent system state.
    ///
    /// # Arguments
    ///
    /// * `value` - The pre-validated string value
    ///
    /// # Examples
    ///
    /// This method is for internal crate usage only when loading pre-validated
    /// resource IDs from trusted sources like databases.
    ///
    /// ```text
    /// // Internal usage pattern:
    /// let id = ResourceId::new_unchecked("db-validated-id".to_string());
    /// ```
    #[allow(dead_code)]
    pub(crate) fn new_unchecked(value: String) -> Self {
        Self(value)
    }

    /// Get the string representation of the ResourceId.
    ///
    /// Returns a reference to the underlying string value. This is safe
    /// because the value is guaranteed to be valid by construction.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use scim_server::resource::value_objects::ResourceId;
    ///
    /// fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let id = ResourceId::new("test-id".to_string())?;
    ///     assert_eq!(id.as_str(), "test-id");
    ///     Ok(())
    /// }
    /// ```
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Get the owned string value of the ResourceId.
    ///
    /// Consumes the ResourceId and returns the underlying string.
    /// Use this when you need to transfer ownership of the string value.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use scim_server::resource::value_objects::ResourceId;
    ///
    /// fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let id = ResourceId::new("test-id".to_string())?;
    ///     let owned_string = id.into_string();
    ///     assert_eq!(owned_string, "test-id");
    ///     Ok(())
    /// }
    /// ```
    pub fn into_string(self) -> String {
        self.0
    }

    /// Validate the format of a resource ID string.
    ///
    /// This function contains the core validation logic moved from SchemaRegistry.
    /// It enforces all the rules that define a valid resource ID.
    ///
    /// # Arguments
    ///
    /// * `value` - The string to validate
    ///
    /// # Returns
    ///
    /// * `Ok(())` - If the value is valid
    /// * `Err(ValidationError)` - If the value violates any rules
    fn validate_format(value: &str) -> ValidationResult<()> {
        // Check if id is empty
        if value.is_empty() {
            return Err(ValidationError::EmptyId);
        }

        // TODO: Add more sophisticated ID format validation if needed
        // For now, we accept any non-empty string as a valid ID
        // Future enhancements might include:
        // - UUID format validation
        // - Character set restrictions
        // - Length limits

        Ok(())
    }
}

impl fmt::Display for ResourceId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Serialize for ResourceId {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.0.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for ResourceId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        Self::new(value).map_err(serde::de::Error::custom)
    }
}

/// Convert from String to ResourceId with validation.
impl TryFrom<String> for ResourceId {
    type Error = ValidationError;

    fn try_from(value: String) -> ValidationResult<Self> {
        Self::new(value)
    }
}

/// Convert from &str to ResourceId with validation.
impl TryFrom<&str> for ResourceId {
    type Error = ValidationError;

    fn try_from(value: &str) -> ValidationResult<Self> {
        Self::new(value.to_string())
    }
}

impl ValueObject for ResourceId {
    fn attribute_type(&self) -> AttributeType {
        AttributeType::String
    }

    fn attribute_name(&self) -> &str {
        "id"
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

        if definition.name != "id" {
            return Err(ValidationError::InvalidAttributeName {
                actual: definition.name.clone(),
                expected: "id".to_string(),
            });
        }

        Ok(())
    }

    fn as_json_value(&self) -> Value {
        Value::String(self.0.clone())
    }

    fn supports_definition(&self, definition: &AttributeDefinition) -> bool {
        definition.data_type == AttributeType::String && definition.name == "id"
    }

    fn clone_boxed(&self) -> Box<dyn ValueObject> {
        Box::new(self.clone())
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl SchemaConstructible for ResourceId {
    fn from_schema_and_value(
        definition: &AttributeDefinition,
        value: &Value,
    ) -> ValidationResult<Self> {
        if definition.name != "id" || definition.data_type != AttributeType::String {
            return Err(ValidationError::UnsupportedAttributeType {
                attribute: definition.name.clone(),
                type_name: format!("{:?}", definition.data_type),
            });
        }

        if let Some(id_str) = value.as_str() {
            Self::new(id_str.to_string())
        } else {
            Err(ValidationError::InvalidAttributeType {
                attribute: definition.name.clone(),
                expected: "string".to_string(),
                actual: "non-string".to_string(),
            })
        }
    }

    fn can_construct_from(definition: &AttributeDefinition) -> bool {
        definition.name == "id" && definition.data_type == AttributeType::String
    }

    fn constructor_priority() -> u8 {
        100 // High priority for exact name match
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::types::{Mutability, Uniqueness};
    use serde_json;

    #[test]
    fn test_valid_resource_id() {
        let id = ResourceId::new("2819c223-7f76-453a-919d-413861904646".to_string());
        assert!(id.is_ok());

        let id = id.unwrap();
        assert_eq!(id.as_str(), "2819c223-7f76-453a-919d-413861904646");
    }

    #[test]
    fn test_empty_resource_id() {
        let result = ResourceId::new("".to_string());
        assert!(result.is_err());

        match result.unwrap_err() {
            ValidationError::EmptyId => {} // Expected
            other => panic!("Expected EmptyId error, got: {:?}", other),
        }
    }

    #[test]
    fn test_simple_string_id() {
        let id = ResourceId::new("user-123".to_string());
        assert!(id.is_ok());

        let id = id.unwrap();
        assert_eq!(id.as_str(), "user-123");
    }

    #[test]
    fn test_new_unchecked() {
        let id = ResourceId::new_unchecked("unchecked-id".to_string());
        assert_eq!(id.as_str(), "unchecked-id");
    }

    #[test]
    fn test_into_string() {
        let id = ResourceId::new("test-id".to_string()).unwrap();
        let string_value = id.into_string();
        assert_eq!(string_value, "test-id");
    }

    #[test]
    fn test_display() {
        let id = ResourceId::new("display-test".to_string()).unwrap();
        assert_eq!(format!("{}", id), "display-test");
    }

    #[test]
    fn test_serialization() {
        let id = ResourceId::new("serialize-test".to_string()).unwrap();
        let json = serde_json::to_string(&id).unwrap();
        assert_eq!(json, "\"serialize-test\"");
    }

    #[test]
    fn test_deserialization_valid() {
        let json = "\"deserialize-test\"";
        let id: ResourceId = serde_json::from_str(json).unwrap();
        assert_eq!(id.as_str(), "deserialize-test");
    }

    #[test]
    fn test_deserialization_invalid() {
        let json = "\"\""; // Empty string
        let result: Result<ResourceId, _> = serde_json::from_str(json);
        assert!(result.is_err());
    }

    #[test]
    fn test_try_from_string() {
        let result = ResourceId::try_from("try-from-test".to_string());
        assert!(result.is_ok());
        assert_eq!(result.unwrap().as_str(), "try-from-test");

        let empty_result = ResourceId::try_from("".to_string());
        assert!(empty_result.is_err());
    }

    #[test]
    fn test_try_from_str() {
        let result = ResourceId::try_from("try-from-str-test");
        assert!(result.is_ok());
        assert_eq!(result.unwrap().as_str(), "try-from-str-test");

        let empty_result = ResourceId::try_from("");
        assert!(empty_result.is_err());
    }

    #[test]
    fn test_equality() {
        let id1 = ResourceId::new("same-id".to_string()).unwrap();
        let id2 = ResourceId::new("same-id".to_string()).unwrap();
        let id3 = ResourceId::new("different-id".to_string()).unwrap();

        assert_eq!(id1, id2);
        assert_ne!(id1, id3);
    }

    #[test]
    fn test_hash() {
        use std::collections::HashMap;

        let id1 = ResourceId::new("hash-test-1".to_string()).unwrap();
        let id2 = ResourceId::new("hash-test-2".to_string()).unwrap();

        let mut map = HashMap::new();
        map.insert(id1.clone(), "value1");
        map.insert(id2.clone(), "value2");

        assert_eq!(map.get(&id1), Some(&"value1"));
        assert_eq!(map.get(&id2), Some(&"value2"));
    }

    #[test]
    fn test_clone() {
        let id = ResourceId::new("clone-test".to_string()).unwrap();
        let cloned = id.clone();

        assert_eq!(id, cloned);
        assert_eq!(id.as_str(), cloned.as_str());
    }

    #[test]
    fn test_value_object_trait() {
        let id = ResourceId::new("test-id".to_string()).unwrap();

        assert_eq!(id.attribute_type(), AttributeType::String);
        assert_eq!(id.attribute_name(), "id");
        assert_eq!(id.as_json_value(), Value::String("test-id".to_string()));

        let json_result = id.to_json().unwrap();
        assert_eq!(json_result, Value::String("test-id".to_string()));
    }

    #[test]
    fn test_schema_constructible_trait() {
        let definition = AttributeDefinition {
            name: "id".to_string(),
            data_type: AttributeType::String,
            multi_valued: false,
            required: true,
            case_exact: true,
            mutability: Mutability::ReadOnly,
            uniqueness: Uniqueness::Server,
            canonical_values: vec![],
            sub_attributes: vec![],
            returned: None,
        };

        let value = Value::String("test-id".to_string());
        let result = ResourceId::from_schema_and_value(&definition, &value);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().as_str(), "test-id");

        // Test can_construct_from
        assert!(ResourceId::can_construct_from(&definition));

        // Test with wrong attribute name
        let mut wrong_def = definition.clone();
        wrong_def.name = "userName".to_string();
        assert!(!ResourceId::can_construct_from(&wrong_def));
    }

    #[test]
    fn test_validate_against_schema() {
        let id = ResourceId::new("test-id".to_string()).unwrap();

        let valid_definition = AttributeDefinition {
            name: "id".to_string(),
            data_type: AttributeType::String,
            multi_valued: false,
            required: true,
            case_exact: true,
            mutability: Mutability::ReadOnly,
            uniqueness: Uniqueness::Server,
            canonical_values: vec![],
            sub_attributes: vec![],
            returned: None,
        };

        assert!(id.validate_against_schema(&valid_definition).is_ok());

        // Test with wrong type
        let mut invalid_def = valid_definition.clone();
        invalid_def.data_type = AttributeType::Integer;
        assert!(id.validate_against_schema(&invalid_def).is_err());

        // Test with wrong name
        invalid_def.name = "userName".to_string();
        invalid_def.data_type = AttributeType::String;
        assert!(id.validate_against_schema(&invalid_def).is_err());
    }
}
