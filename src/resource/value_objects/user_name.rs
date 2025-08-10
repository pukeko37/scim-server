//! UserName value object for SCIM user identifiers.
//!
//! This module provides a type-safe wrapper around user names with built-in validation.
//! User names are fundamental identifiers in SCIM that must be unique and follow specific rules.

use crate::error::{ValidationError, ValidationResult};
use crate::resource::value_objects::value_object_trait::{SchemaConstructible, ValueObject};
use crate::schema::types::{AttributeDefinition, AttributeType};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_json::Value;
use std::any::Any;
use std::fmt;

/// A validated SCIM user name.
///
/// UserName represents a unique identifier for a SCIM user. It enforces
/// validation rules at construction time, ensuring that only valid user names
/// can exist in the system.
///
/// ## Validation Rules
///
/// - Must not be empty
/// - Must be a valid string
/// - Future: May include character set restrictions, length limits, etc.
///
/// ## Examples
///
/// ```rust
/// use scim_server::resource::value_objects::UserName;
///
/// fn main() -> Result<(), Box<dyn std::error::Error>> {
///     // Valid user name
///     let username = UserName::new("bjensen@example.com".to_string())?;
///     println!("User name: {}", username.as_str());
///
///     // Invalid user name - returns ValidationError
///     let invalid = UserName::new("".to_string()); // Error
///     assert!(invalid.is_err());
///
///     Ok(())
/// }
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct UserName(String);

impl UserName {
    /// Create a new UserName with validation.
    ///
    /// This is the primary constructor that enforces all validation rules.
    /// Use this method when creating UserName instances from untrusted input.
    ///
    /// # Arguments
    ///
    /// * `value` - The string value to validate and wrap
    ///
    /// # Returns
    ///
    /// * `Ok(UserName)` - If the value is valid
    /// * `Err(ValidationError)` - If the value violates validation rules
    pub fn new(value: String) -> ValidationResult<Self> {
        Self::validate_format(&value)?;
        Ok(Self(value))
    }

    /// Create a UserName without validation.
    ///
    /// This constructor bypasses validation and should only be used in contexts
    /// where the value is guaranteed to be valid (e.g., from trusted data sources).
    ///
    /// # Safety
    ///
    /// The caller must ensure that the value meets all UserName validation requirements.
    #[allow(dead_code)]
    pub(crate) fn new_unchecked(value: String) -> Self {
        Self(value)
    }

    /// Get the string representation of the UserName.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Get the owned string value of the UserName.
    pub fn into_string(self) -> String {
        self.0
    }

    /// Validate the format of a user name string.
    ///
    /// This function contains validation logic for user names.
    /// Currently implements basic validation that can be enhanced in the future.
    fn validate_format(value: &str) -> ValidationResult<()> {
        // User name should not be empty
        if value.is_empty() {
            return Err(ValidationError::MissingRequiredAttribute {
                attribute: "userName".to_string(),
            });
        }

        // TODO: Add more sophisticated user name validation if needed
        // For now, we accept any non-empty string as a valid user name
        // Future enhancements might include:
        // - Email format validation if email-based usernames are required
        // - Character set restrictions (alphanumeric + specific symbols)
        // - Length limits (min/max)
        // - Reserved name checking
        // - Case sensitivity rules

        Ok(())
    }
}

impl fmt::Display for UserName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Serialize for UserName {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.0.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for UserName {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        Self::new(value).map_err(serde::de::Error::custom)
    }
}

impl TryFrom<String> for UserName {
    type Error = ValidationError;

    fn try_from(value: String) -> ValidationResult<Self> {
        Self::new(value)
    }
}

impl TryFrom<&str> for UserName {
    type Error = ValidationError;

    fn try_from(value: &str) -> ValidationResult<Self> {
        Self::new(value.to_string())
    }
}

impl ValueObject for UserName {
    fn attribute_type(&self) -> AttributeType {
        AttributeType::String
    }

    fn attribute_name(&self) -> &str {
        "userName"
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

        if definition.name != "userName" {
            return Err(ValidationError::InvalidAttributeName {
                actual: definition.name.clone(),
                expected: "userName".to_string(),
            });
        }

        Ok(())
    }

    fn as_json_value(&self) -> Value {
        Value::String(self.0.clone())
    }

    fn supports_definition(&self, definition: &AttributeDefinition) -> bool {
        definition.data_type == AttributeType::String && definition.name == "userName"
    }

    fn clone_boxed(&self) -> Box<dyn ValueObject> {
        Box::new(self.clone())
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl SchemaConstructible for UserName {
    fn from_schema_and_value(
        definition: &AttributeDefinition,
        value: &Value,
    ) -> ValidationResult<Self> {
        if definition.name != "userName" || definition.data_type != AttributeType::String {
            return Err(ValidationError::UnsupportedAttributeType {
                attribute: definition.name.clone(),
                type_name: format!("{:?}", definition.data_type),
            });
        }

        if let Some(username_str) = value.as_str() {
            Self::new(username_str.to_string())
        } else {
            Err(ValidationError::InvalidAttributeType {
                attribute: definition.name.clone(),
                expected: "string".to_string(),
                actual: "non-string".to_string(),
            })
        }
    }

    fn can_construct_from(definition: &AttributeDefinition) -> bool {
        definition.name == "userName" && definition.data_type == AttributeType::String
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
    fn test_valid_user_name_email() {
        let username = UserName::new("bjensen@example.com".to_string());
        assert!(username.is_ok());
        assert_eq!(username.unwrap().as_str(), "bjensen@example.com");
    }

    #[test]
    fn test_valid_user_name_simple() {
        let username = UserName::new("bjensen".to_string());
        assert!(username.is_ok());
        assert_eq!(username.unwrap().as_str(), "bjensen");
    }

    #[test]
    fn test_valid_user_name_with_numbers() {
        let username = UserName::new("user123".to_string());
        assert!(username.is_ok());
        assert_eq!(username.unwrap().as_str(), "user123");
    }

    #[test]
    fn test_empty_user_name() {
        let result = UserName::new("".to_string());
        assert!(result.is_err());

        match result.unwrap_err() {
            ValidationError::MissingRequiredAttribute { attribute } => {
                assert_eq!(attribute, "userName");
            }
            other => panic!("Expected MissingRequiredAttribute error, got: {:?}", other),
        }
    }

    #[test]
    fn test_new_unchecked() {
        let username = UserName::new_unchecked("unchecked-username".to_string());
        assert_eq!(username.as_str(), "unchecked-username");
    }

    #[test]
    fn test_into_string() {
        let username = UserName::new("test-username".to_string()).unwrap();
        let string_value = username.into_string();
        assert_eq!(string_value, "test-username");
    }

    #[test]
    fn test_display() {
        let username = UserName::new("display-test".to_string()).unwrap();
        assert_eq!(format!("{}", username), "display-test");
    }

    #[test]
    fn test_serialization() {
        let username = UserName::new("serialize-test@example.com".to_string()).unwrap();
        let json = serde_json::to_string(&username).unwrap();
        assert_eq!(json, "\"serialize-test@example.com\"");
    }

    #[test]
    fn test_deserialization_valid() {
        let json = "\"deserialize-test@example.com\"";
        let username: UserName = serde_json::from_str(json).unwrap();
        assert_eq!(username.as_str(), "deserialize-test@example.com");
    }

    #[test]
    fn test_deserialization_invalid() {
        let json = "\"\""; // Empty string
        let result: Result<UserName, _> = serde_json::from_str(json);
        assert!(result.is_err());
    }

    #[test]
    fn test_try_from_string() {
        let result = UserName::try_from("try-from-test".to_string());
        assert!(result.is_ok());
        assert_eq!(result.unwrap().as_str(), "try-from-test");

        let empty_result = UserName::try_from("".to_string());
        assert!(empty_result.is_err());
    }

    #[test]
    fn test_try_from_str() {
        let result = UserName::try_from("try-from-str-test");
        assert!(result.is_ok());
        assert_eq!(result.unwrap().as_str(), "try-from-str-test");

        let empty_result = UserName::try_from("");
        assert!(empty_result.is_err());
    }

    #[test]
    fn test_equality() {
        let username1 = UserName::new("same-username".to_string()).unwrap();
        let username2 = UserName::new("same-username".to_string()).unwrap();
        let username3 = UserName::new("different-username".to_string()).unwrap();

        assert_eq!(username1, username2);
        assert_ne!(username1, username3);
    }

    #[test]
    fn test_hash() {
        use std::collections::HashMap;

        let username1 = UserName::new("hash-test-1".to_string()).unwrap();
        let username2 = UserName::new("hash-test-2".to_string()).unwrap();

        let mut map = HashMap::new();
        map.insert(username1.clone(), "value1");
        map.insert(username2.clone(), "value2");

        assert_eq!(map.get(&username1), Some(&"value1"));
        assert_eq!(map.get(&username2), Some(&"value2"));
    }

    #[test]
    fn test_clone() {
        let username = UserName::new("clone-test".to_string()).unwrap();
        let cloned = username.clone();

        assert_eq!(username, cloned);
        assert_eq!(username.as_str(), cloned.as_str());
    }
}
