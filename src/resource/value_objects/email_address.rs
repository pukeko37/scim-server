//! EmailAddress value object for SCIM email addresses.
//!
//! This module provides a type-safe wrapper around email addresses with built-in validation.
//! Email addresses are complex multi-valued attributes in SCIM with specific structure requirements.

use crate::error::{ValidationError, ValidationResult};
use crate::resource::value_objects::value_object_trait::{SchemaConstructible, ValueObject};
use crate::schema::types::{AttributeDefinition, AttributeType};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::any::Any;
use std::fmt;

/// A validated SCIM email address.
///
/// EmailAddress represents an email address with optional metadata like type and primary flag.
/// It enforces validation rules at construction time, ensuring that only valid email addresses
/// can exist in the system.
///
/// ## Validation Rules
///
/// - Email value must not be empty
/// - Email value must be a valid string
/// - Email type, if provided, must not be empty
/// - Primary flag is optional
/// - Display name is optional
///
/// ## Examples
///
/// ```rust
/// use scim_server::resource::value_objects::EmailAddress;
///
/// fn main() -> Result<(), Box<dyn std::error::Error>> {
///     // Valid email address
///     let email = EmailAddress::new(
///         "bjensen@example.com".to_string(),
///         Some("work".to_string()),
///         Some(true),
///         Some("Barbara Jensen".to_string())
///     )?;
///     println!("Email: {}", email.value());
///
///     // Simple email without metadata
///     let simple_email = EmailAddress::new_simple("user@example.com".to_string())?;
///
///     Ok(())
/// }
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct EmailAddress {
    pub value: String,
    #[serde(rename = "type")]
    pub email_type: Option<String>,
    pub primary: Option<bool>,
    pub display: Option<String>,
}

impl EmailAddress {
    /// Create a new EmailAddress with full metadata.
    ///
    /// This is the primary constructor that enforces all validation rules.
    /// Use this method when creating EmailAddress instances from untrusted input.
    ///
    /// # Arguments
    ///
    /// * `value` - The email address string to validate
    /// * `email_type` - Optional type designation (e.g., "work", "home")
    /// * `primary` - Optional flag indicating if this is the primary email
    /// * `display` - Optional display name for the email
    ///
    /// # Returns
    ///
    /// * `Ok(EmailAddress)` - If all values are valid
    /// * `Err(ValidationError)` - If any value violates validation rules
    pub fn new(
        value: String,
        email_type: Option<String>,
        primary: Option<bool>,
        display: Option<String>,
    ) -> ValidationResult<Self> {
        Self::validate_value(&value)?;
        if let Some(ref type_val) = email_type {
            Self::validate_type(type_val)?;
        }
        if let Some(ref display_val) = display {
            Self::validate_display(display_val)?;
        }

        Ok(Self {
            value,
            email_type,
            primary,
            display,
        })
    }

    /// Create a simple EmailAddress with just the email value.
    ///
    /// Convenience constructor for creating email addresses without metadata.
    ///
    /// # Arguments
    ///
    /// * `value` - The email address string to validate
    ///
    /// # Returns
    ///
    /// * `Ok(EmailAddress)` - If the value is valid
    /// * `Err(ValidationError)` - If the value violates validation rules
    pub fn new_simple(value: String) -> ValidationResult<Self> {
        Self::new(value, None, None, None)
    }

    /// Create an EmailAddress without validation.
    ///
    /// This constructor bypasses validation and should only be used in contexts
    /// where the values are guaranteed to be valid (e.g., from trusted data sources).
    ///
    /// # Safety
    ///
    /// The caller must ensure that all values meet EmailAddress validation requirements.
    #[allow(dead_code)]
    pub(crate) fn new_unchecked(
        value: String,
        email_type: Option<String>,
        primary: Option<bool>,
        display: Option<String>,
    ) -> Self {
        Self {
            value,
            email_type,
            primary,
            display,
        }
    }

    /// Get the email address value.
    pub fn value(&self) -> &str {
        &self.value
    }

    /// Get the email type.
    pub fn email_type(&self) -> Option<&str> {
        self.email_type.as_deref()
    }

    /// Get the primary flag.
    pub fn primary(&self) -> Option<bool> {
        self.primary
    }

    /// Get the display name.
    pub fn display(&self) -> Option<&str> {
        self.display.as_deref()
    }

    /// Check if this is marked as the primary email.
    pub fn is_primary(&self) -> bool {
        self.primary.unwrap_or(false)
    }

    /// Validate the email address value.
    fn validate_value(value: &str) -> ValidationResult<()> {
        if value.is_empty() {
            return Err(ValidationError::MissingRequiredSubAttribute {
                attribute: "emails".to_string(),
                sub_attribute: "value".to_string(),
            });
        }

        // TODO: Add more sophisticated email validation if needed
        // For now, we accept any non-empty string as a valid email
        // Future enhancements might include:
        // - RFC 5322 email format validation
        // - Domain validation
        // - Length limits
        // - Character set restrictions

        Ok(())
    }

    /// Validate the email type value.
    fn validate_type(email_type: &str) -> ValidationResult<()> {
        if email_type.is_empty() {
            return Err(ValidationError::InvalidStringFormat {
                attribute: "emails.type".to_string(),
                details: "Email type cannot be empty".to_string(),
            });
        }

        // TODO: Add canonical value validation if needed
        // Common types include: "work", "home", "other"

        Ok(())
    }

    /// Validate the display name value.
    fn validate_display(_display: &str) -> ValidationResult<()> {
        // Display name can be empty, so no validation needed for now
        // Future enhancements might include:
        // - Length limits
        // - Character set restrictions

        Ok(())
    }
}

impl fmt::Display for EmailAddress {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(ref display) = self.display {
            write!(f, "{} <{}>", display, self.value)
        } else {
            write!(f, "{}", self.value)
        }
    }
}

impl TryFrom<String> for EmailAddress {
    type Error = ValidationError;

    fn try_from(value: String) -> ValidationResult<Self> {
        Self::new_simple(value)
    }
}

impl TryFrom<&str> for EmailAddress {
    type Error = ValidationError;

    fn try_from(value: &str) -> ValidationResult<Self> {
        Self::new_simple(value.to_string())
    }
}

impl ValueObject for EmailAddress {
    fn attribute_type(&self) -> AttributeType {
        AttributeType::Complex
    }

    fn attribute_name(&self) -> &str {
        "emails"
    }

    fn to_json(&self) -> ValidationResult<Value> {
        Ok(serde_json::to_value(self)?)
    }

    fn validate_against_schema(&self, definition: &AttributeDefinition) -> ValidationResult<()> {
        if definition.data_type != AttributeType::Complex {
            return Err(ValidationError::InvalidAttributeType {
                attribute: definition.name.clone(),
                expected: "complex".to_string(),
                actual: format!("{:?}", definition.data_type),
            });
        }

        // EmailAddress can work with both "emails" and "value" attribute names
        if definition.name != "emails" && definition.name != "value" {
            return Err(ValidationError::InvalidAttributeName {
                actual: definition.name.clone(),
                expected: "emails or value".to_string(),
            });
        }

        Ok(())
    }

    fn as_json_value(&self) -> Value {
        serde_json::to_value(self).unwrap_or(Value::Null)
    }

    fn supports_definition(&self, definition: &AttributeDefinition) -> bool {
        definition.data_type == AttributeType::Complex
            && (definition.name == "emails" || definition.name == "value")
    }

    fn clone_boxed(&self) -> Box<dyn ValueObject> {
        Box::new(self.clone())
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl SchemaConstructible for EmailAddress {
    fn from_schema_and_value(
        definition: &AttributeDefinition,
        value: &Value,
    ) -> ValidationResult<Self> {
        if definition.data_type != AttributeType::Complex {
            return Err(ValidationError::UnsupportedAttributeType {
                attribute: definition.name.clone(),
                type_name: format!("{:?}", definition.data_type),
            });
        }

        if let Value::Object(obj) = value {
            let email_value = obj.get("value").and_then(|v| v.as_str()).ok_or_else(|| {
                ValidationError::InvalidAttributeType {
                    attribute: definition.name.clone(),
                    expected: "object with 'value' field".to_string(),
                    actual: "object without 'value' field".to_string(),
                }
            })?;

            let email_type = obj
                .get("type")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());

            let primary = obj.get("primary").and_then(|v| v.as_bool());

            let display = obj
                .get("display")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());

            Self::new(email_value.to_string(), email_type, primary, display)
        } else if let Some(email_str) = value.as_str() {
            // Handle simple string case
            Self::new(email_str.to_string(), None, None, None)
        } else {
            Err(ValidationError::InvalidAttributeType {
                attribute: definition.name.clone(),
                expected: "object or string".to_string(),
                actual: "neither object nor string".to_string(),
            })
        }
    }

    fn can_construct_from(definition: &AttributeDefinition) -> bool {
        definition.data_type == AttributeType::Complex
            && (definition.name == "emails"
                || definition.name == "value"
                || definition.name.contains("email"))
    }

    fn constructor_priority() -> u8 {
        80 // Lower priority than exact name matches
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json;

    #[test]
    fn test_valid_email_full() {
        let email = EmailAddress::new(
            "bjensen@example.com".to_string(),
            Some("work".to_string()),
            Some(true),
            Some("Barbara Jensen".to_string()),
        );
        assert!(email.is_ok());

        let email = email.unwrap();
        assert_eq!(email.value(), "bjensen@example.com");
        assert_eq!(email.email_type(), Some("work"));
        assert_eq!(email.primary(), Some(true));
        assert_eq!(email.display(), Some("Barbara Jensen"));
        assert!(email.is_primary());
    }

    #[test]
    fn test_valid_email_simple() {
        let email = EmailAddress::new_simple("user@example.com".to_string());
        assert!(email.is_ok());

        let email = email.unwrap();
        assert_eq!(email.value(), "user@example.com");
        assert_eq!(email.email_type(), None);
        assert_eq!(email.primary(), None);
        assert_eq!(email.display(), None);
        assert!(!email.is_primary());
    }

    #[test]
    fn test_empty_email_value() {
        let result = EmailAddress::new_simple("".to_string());
        assert!(result.is_err());

        match result.unwrap_err() {
            ValidationError::MissingRequiredSubAttribute {
                attribute,
                sub_attribute,
            } => {
                assert_eq!(attribute, "emails");
                assert_eq!(sub_attribute, "value");
            }
            other => panic!(
                "Expected MissingRequiredSubAttribute error, got: {:?}",
                other
            ),
        }
    }

    #[test]
    fn test_empty_email_type() {
        let result = EmailAddress::new(
            "test@example.com".to_string(),
            Some("".to_string()),
            None,
            None,
        );
        assert!(result.is_err());

        match result.unwrap_err() {
            ValidationError::InvalidStringFormat { attribute, details } => {
                assert_eq!(attribute, "emails.type");
                assert!(details.contains("cannot be empty"));
            }
            other => panic!("Expected InvalidStringFormat error, got: {:?}", other),
        }
    }

    #[test]
    fn test_new_unchecked() {
        let email = EmailAddress::new_unchecked(
            "unchecked@example.com".to_string(),
            Some("work".to_string()),
            Some(false),
            None,
        );
        assert_eq!(email.value(), "unchecked@example.com");
        assert_eq!(email.email_type(), Some("work"));
        assert_eq!(email.primary(), Some(false));
    }

    #[test]
    fn test_display() {
        let email_with_display = EmailAddress::new(
            "test@example.com".to_string(),
            None,
            None,
            Some("Test User".to_string()),
        )
        .unwrap();
        assert_eq!(
            format!("{}", email_with_display),
            "Test User <test@example.com>"
        );

        let email_without_display =
            EmailAddress::new_simple("test@example.com".to_string()).unwrap();
        assert_eq!(format!("{}", email_without_display), "test@example.com");
    }

    #[test]
    fn test_serialization() {
        let email = EmailAddress::new(
            "serialize@example.com".to_string(),
            Some("work".to_string()),
            Some(true),
            Some("Serialize Test".to_string()),
        )
        .unwrap();

        let json = serde_json::to_string(&email).unwrap();
        let expected = r#"{"value":"serialize@example.com","type":"work","primary":true,"display":"Serialize Test"}"#;
        assert_eq!(json, expected);
    }

    #[test]
    fn test_deserialization() {
        let json = r#"{"value":"deserialize@example.com","type":"home","primary":false}"#;
        let email: EmailAddress = serde_json::from_str(json).unwrap();

        assert_eq!(email.value(), "deserialize@example.com");
        assert_eq!(email.email_type(), Some("home"));
        assert_eq!(email.primary(), Some(false));
        assert_eq!(email.display(), None);
    }

    #[test]
    fn test_try_from_string() {
        let result = EmailAddress::try_from("try-from@example.com".to_string());
        assert!(result.is_ok());
        assert_eq!(result.unwrap().value(), "try-from@example.com");

        let empty_result = EmailAddress::try_from("".to_string());
        assert!(empty_result.is_err());
    }

    #[test]
    fn test_try_from_str() {
        let result = EmailAddress::try_from("try-from-str@example.com");
        assert!(result.is_ok());
        assert_eq!(result.unwrap().value(), "try-from-str@example.com");

        let empty_result = EmailAddress::try_from("");
        assert!(empty_result.is_err());
    }

    #[test]
    fn test_equality() {
        let email1 = EmailAddress::new(
            "same@example.com".to_string(),
            Some("work".to_string()),
            Some(true),
            None,
        )
        .unwrap();
        let email2 = EmailAddress::new(
            "same@example.com".to_string(),
            Some("work".to_string()),
            Some(true),
            None,
        )
        .unwrap();
        let email3 = EmailAddress::new_simple("different@example.com".to_string()).unwrap();

        assert_eq!(email1, email2);
        assert_ne!(email1, email3);
    }

    #[test]
    fn test_hash() {
        use std::collections::HashMap;

        let email1 = EmailAddress::new_simple("hash-test-1@example.com".to_string()).unwrap();
        let email2 = EmailAddress::new_simple("hash-test-2@example.com".to_string()).unwrap();

        let mut map = HashMap::new();
        map.insert(email1.clone(), "value1");
        map.insert(email2.clone(), "value2");

        assert_eq!(map.get(&email1), Some(&"value1"));
        assert_eq!(map.get(&email2), Some(&"value2"));
    }

    #[test]
    fn test_clone() {
        let email = EmailAddress::new(
            "clone@example.com".to_string(),
            Some("work".to_string()),
            Some(true),
            Some("Clone Test".to_string()),
        )
        .unwrap();
        let cloned = email.clone();

        assert_eq!(email, cloned);
        assert_eq!(email.value(), cloned.value());
        assert_eq!(email.email_type(), cloned.email_type());
        assert_eq!(email.primary(), cloned.primary());
        assert_eq!(email.display(), cloned.display());
    }
}
