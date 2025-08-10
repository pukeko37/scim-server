//! PhoneNumber value object for SCIM user phone number components.
//!
//! This module provides a type-safe wrapper around SCIM phoneNumbers attributes with built-in validation.
//! PhoneNumber attributes represent phone numbers as defined in RFC 7643 Section 4.1.2.

use crate::error::{ValidationError, ValidationResult};
use serde::{Deserialize, Serialize};
use std::fmt;

/// A validated SCIM phone number attribute.
///
/// PhoneNumber represents a phone number as defined in RFC 7643.
/// It enforces validation rules at construction time, ensuring that only valid phone number
/// attributes can exist in the system.
///
/// ## Validation Rules
///
/// - Phone number value cannot be empty
/// - Phone number should follow RFC 3966 format when possible (tel:+1-201-555-0123)
/// - Type must be one of canonical values: "work", "home", "mobile", "fax", "pager", "other" when provided
/// - Display name is optional and used for human-readable representation
/// - Primary can only be true for one phone number in a collection
///
/// ## Examples
///
/// ```rust
/// use scim_server::resource::value_objects::PhoneNumber;
///
/// fn main() -> Result<(), Box<dyn std::error::Error>> {
///     // Create with full phone number components
///     let phone = PhoneNumber::new(
///         "+1-201-555-0123".to_string(),
///         Some("Work Phone".to_string()),
///         Some("work".to_string()),
///         Some(true)
///     )?;
///
///     // Create with minimal components
///     let simple_phone = PhoneNumber::new_simple(
///         "+1-555-123-4567".to_string(),
///         "mobile".to_string()
///     )?;
///
///     Ok(())
/// }
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PhoneNumber {
    pub value: String,
    pub display: Option<String>,
    #[serde(rename = "type")]
    pub phone_type: Option<String>,
    pub primary: Option<bool>,
}

impl PhoneNumber {
    /// Create a new PhoneNumber with all components.
    ///
    /// This is the primary constructor that enforces all validation rules.
    /// Use this method when creating PhoneNumber instances from untrusted input.
    ///
    /// # Arguments
    ///
    /// * `value` - The phone number value (preferably in RFC 3966 format)
    /// * `display` - Optional human-readable display name
    /// * `phone_type` - The type of phone number ("work", "home", "mobile", etc.)
    /// * `primary` - Whether this is the primary phone number
    ///
    /// # Returns
    ///
    /// * `Ok(PhoneNumber)` - If the phone number is valid
    /// * `Err(ValidationError)` - If any field violates validation rules
    pub fn new(
        value: String,
        display: Option<String>,
        phone_type: Option<String>,
        primary: Option<bool>,
    ) -> ValidationResult<Self> {
        // Validate the phone number value
        Self::validate_phone_value(&value)?;

        // Validate display if provided
        if let Some(ref d) = display {
            Self::validate_display(d)?;
        }

        // Validate phone type if provided
        if let Some(ref pt) = phone_type {
            Self::validate_phone_type(pt)?;
        }

        Ok(Self {
            value,
            display,
            phone_type,
            primary,
        })
    }

    /// Create a simple PhoneNumber with just value and type.
    ///
    /// Convenience constructor for creating basic phone number structures.
    ///
    /// # Arguments
    ///
    /// * `value` - The phone number value
    /// * `phone_type` - The type of phone number
    ///
    /// # Returns
    ///
    /// * `Ok(PhoneNumber)` - If the phone number is valid
    /// * `Err(ValidationError)` - If any component violates validation rules
    pub fn new_simple(value: String, phone_type: String) -> ValidationResult<Self> {
        Self::new(value, None, Some(phone_type), None)
    }

    /// Create a work PhoneNumber.
    ///
    /// Convenience constructor for work phone numbers.
    ///
    /// # Arguments
    ///
    /// * `value` - The phone number value
    ///
    /// # Returns
    ///
    /// * `Ok(PhoneNumber)` - If the phone number is valid
    /// * `Err(ValidationError)` - If the phone number violates validation rules
    pub fn new_work(value: String) -> ValidationResult<Self> {
        Self::new(value, None, Some("work".to_string()), None)
    }

    /// Create a mobile PhoneNumber.
    ///
    /// Convenience constructor for mobile phone numbers.
    ///
    /// # Arguments
    ///
    /// * `value` - The phone number value
    ///
    /// # Returns
    ///
    /// * `Ok(PhoneNumber)` - If the phone number is valid
    /// * `Err(ValidationError)` - If the phone number violates validation rules
    pub fn new_mobile(value: String) -> ValidationResult<Self> {
        Self::new(value, None, Some("mobile".to_string()), None)
    }

    /// Create a PhoneNumber instance without validation for internal use.
    ///
    /// This method bypasses validation and should only be used when the data
    /// is known to be valid, such as when deserializing from trusted sources.
    ///
    /// # Safety
    ///
    /// The caller must ensure that the provided values are valid according to
    /// SCIM phone number validation rules.
    #[allow(dead_code)]
    pub(crate) fn new_unchecked(
        value: String,
        display: Option<String>,
        phone_type: Option<String>,
        primary: Option<bool>,
    ) -> Self {
        Self {
            value,
            display,
            phone_type,
            primary,
        }
    }

    /// Get the phone number value.
    pub fn value(&self) -> &str {
        &self.value
    }

    /// Get the display name.
    pub fn display(&self) -> Option<&str> {
        self.display.as_deref()
    }

    /// Get the phone type.
    pub fn phone_type(&self) -> Option<&str> {
        self.phone_type.as_deref()
    }

    /// Get whether this is the primary phone number.
    pub fn is_primary(&self) -> bool {
        self.primary.unwrap_or(false)
    }

    /// Get a display-friendly representation of the phone number.
    ///
    /// Returns the display name if available, otherwise the phone number value.
    pub fn display_value(&self) -> &str {
        self.display.as_deref().unwrap_or(&self.value)
    }

    /// Check if this phone number uses RFC 3966 format.
    pub fn is_rfc3966_format(&self) -> bool {
        self.value.starts_with("tel:")
    }

    /// Convert to RFC 3966 format if possible.
    ///
    /// Attempts to convert the phone number to RFC 3966 format.
    /// This is a simple conversion that handles common cases.
    pub fn to_rfc3966(&self) -> String {
        if self.is_rfc3966_format() {
            self.value.clone()
        } else {
            // Simple conversion - in practice you might want more sophisticated parsing
            let cleaned = self
                .value
                .chars()
                .filter(|c| c.is_ascii_digit() || *c == '+' || *c == '-')
                .collect::<String>();

            if cleaned.starts_with('+') {
                format!("tel:{}", cleaned)
            } else {
                format!("tel:+{}", cleaned)
            }
        }
    }

    /// Validate the phone number value.
    fn validate_phone_value(value: &str) -> ValidationResult<()> {
        if value.trim().is_empty() {
            return Err(ValidationError::custom(
                "value: Phone number value cannot be empty",
            ));
        }

        // Check for reasonable length
        if value.len() > 50 {
            return Err(ValidationError::custom(
                "value: Phone number exceeds maximum length of 50 characters",
            ));
        }

        // If it claims to be RFC 3966 format, do basic validation first
        if value.starts_with("tel:") {
            let phone_part = &value[4..];
            if phone_part.is_empty() {
                return Err(ValidationError::custom(
                    "value: RFC 3966 format phone number cannot be empty after 'tel:' prefix",
                ));
            }
        }

        // Basic format validation - should contain digits and possibly formatting characters
        if !value.chars().any(|c| c.is_ascii_digit()) {
            return Err(ValidationError::custom(
                "value: Phone number must contain at least one digit",
            ));
        }

        // Check for obviously invalid characters
        let has_invalid_chars = value.chars().any(|c| {
            !c.is_ascii_digit()
                && c != '+'
                && c != '-'
                && c != '('
                && c != ')'
                && c != ' '
                && c != '.'
                && c != ':'
                && !c.is_ascii_alphabetic() // for tel: prefix
        });

        if has_invalid_chars {
            return Err(ValidationError::custom(
                "value: Phone number contains invalid characters",
            ));
        }

        Ok(())
    }

    /// Validate the display name.
    fn validate_display(display: &str) -> ValidationResult<()> {
        if display.trim().is_empty() {
            return Err(ValidationError::custom(
                "display: Display name cannot be empty or contain only whitespace",
            ));
        }

        // Check for reasonable length
        if display.len() > 256 {
            return Err(ValidationError::custom(
                "display: Display name exceeds maximum length of 256 characters",
            ));
        }

        Ok(())
    }

    /// Validate phone type against canonical values.
    fn validate_phone_type(phone_type: &str) -> ValidationResult<()> {
        if phone_type.trim().is_empty() {
            return Err(ValidationError::custom("type: Phone type cannot be empty"));
        }

        // SCIM canonical values for phone type
        let valid_types = ["work", "home", "mobile", "fax", "pager", "other"];
        if !valid_types.contains(&phone_type) {
            return Err(ValidationError::custom(format!(
                "type: '{}' is not a valid phone type. Valid types are: {:?}",
                phone_type, valid_types
            )));
        }

        Ok(())
    }
}

impl fmt::Display for PhoneNumber {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(phone_type) = &self.phone_type {
            write!(f, "{} ({})", self.display_value(), phone_type)
        } else {
            write!(f, "{}", self.display_value())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_phone_number_full() {
        let phone = PhoneNumber::new(
            "+1-201-555-0123".to_string(),
            Some("Work Phone".to_string()),
            Some("work".to_string()),
            Some(true),
        );

        assert!(phone.is_ok());
        let phone = phone.unwrap();
        assert_eq!(phone.value(), "+1-201-555-0123");
        assert_eq!(phone.display(), Some("Work Phone"));
        assert_eq!(phone.phone_type(), Some("work"));
        assert!(phone.is_primary());
    }

    #[test]
    fn test_valid_phone_number_simple() {
        let phone = PhoneNumber::new_simple("555-123-4567".to_string(), "mobile".to_string());

        assert!(phone.is_ok());
        let phone = phone.unwrap();
        assert_eq!(phone.value(), "555-123-4567");
        assert_eq!(phone.phone_type(), Some("mobile"));
        assert!(!phone.is_primary());
    }

    #[test]
    fn test_valid_phone_number_work() {
        let phone = PhoneNumber::new_work("+1-555-123-4567".to_string());

        assert!(phone.is_ok());
        let phone = phone.unwrap();
        assert_eq!(phone.phone_type(), Some("work"));
        assert_eq!(phone.value(), "+1-555-123-4567");
    }

    #[test]
    fn test_valid_phone_number_mobile() {
        let phone = PhoneNumber::new_mobile("(555) 123-4567".to_string());

        assert!(phone.is_ok());
        let phone = phone.unwrap();
        assert_eq!(phone.phone_type(), Some("mobile"));
        assert_eq!(phone.value(), "(555) 123-4567");
    }

    #[test]
    fn test_empty_phone_value() {
        let result = PhoneNumber::new("".to_string(), None, Some("work".to_string()), None);
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Phone number value cannot be empty")
        );
    }

    #[test]
    fn test_invalid_phone_type() {
        let result = PhoneNumber::new(
            "555-123-4567".to_string(),
            None,
            Some("business".to_string()), // Should be work, home, mobile, fax, pager, or other
            None,
        );
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("not a valid phone type")
        );
    }

    #[test]
    fn test_too_long_phone_value() {
        let long_phone = "1".repeat(60);
        let result = PhoneNumber::new_work(long_phone);
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("exceeds maximum length")
        );
    }

    #[test]
    fn test_phone_without_digits() {
        let result = PhoneNumber::new_work("abc-def-ghij".to_string());
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("must contain at least one digit")
        );
    }

    #[test]
    fn test_phone_with_invalid_characters() {
        let result = PhoneNumber::new_work("555-123-4567#".to_string());
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("contains invalid characters")
        );
    }

    #[test]
    fn test_empty_display() {
        let result = PhoneNumber::new(
            "555-123-4567".to_string(),
            Some("".to_string()),
            Some("work".to_string()),
            None,
        );
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Display name cannot be empty")
        );
    }

    #[test]
    fn test_rfc3966_format() {
        let phone = PhoneNumber::new_work("tel:+1-201-555-0123".to_string()).unwrap();
        assert!(phone.is_rfc3966_format());

        let phone2 = PhoneNumber::new_work("555-123-4567".to_string()).unwrap();
        assert!(!phone2.is_rfc3966_format());
    }

    #[test]
    fn test_to_rfc3966() {
        let phone = PhoneNumber::new_work("555-123-4567".to_string()).unwrap();
        assert_eq!(phone.to_rfc3966(), "tel:+555-123-4567");

        let phone2 = PhoneNumber::new_work("+1-555-123-4567".to_string()).unwrap();
        assert_eq!(phone2.to_rfc3966(), "tel:+1-555-123-4567");

        let phone3 = PhoneNumber::new_work("tel:+1-555-123-4567".to_string()).unwrap();
        assert_eq!(phone3.to_rfc3966(), "tel:+1-555-123-4567");
    }

    #[test]
    fn test_display_value() {
        let phone = PhoneNumber::new(
            "555-123-4567".to_string(),
            Some("My Work Phone".to_string()),
            Some("work".to_string()),
            None,
        )
        .unwrap();
        assert_eq!(phone.display_value(), "My Work Phone");

        let phone2 = PhoneNumber::new_work("555-123-4567".to_string()).unwrap();
        assert_eq!(phone2.display_value(), "555-123-4567");
    }

    #[test]
    fn test_new_unchecked() {
        let phone = PhoneNumber::new_unchecked(
            "555-123-4567".to_string(),
            Some("Work Phone".to_string()),
            Some("work".to_string()),
            Some(true),
        );

        assert_eq!(phone.value(), "555-123-4567");
        assert_eq!(phone.display(), Some("Work Phone"));
        assert_eq!(phone.phone_type(), Some("work"));
        assert!(phone.is_primary());
    }

    #[test]
    fn test_display() {
        let phone = PhoneNumber::new(
            "555-123-4567".to_string(),
            Some("My Work Phone".to_string()),
            Some("work".to_string()),
            None,
        )
        .unwrap();
        assert_eq!(format!("{}", phone), "My Work Phone (work)");

        let phone2 = PhoneNumber::new(
            "555-123-4567".to_string(),
            None,
            Some("mobile".to_string()),
            None,
        )
        .unwrap();
        assert_eq!(format!("{}", phone2), "555-123-4567 (mobile)");

        let phone3 = PhoneNumber::new("555-123-4567".to_string(), None, None, None).unwrap();
        assert_eq!(format!("{}", phone3), "555-123-4567");
    }

    #[test]
    fn test_serialization() {
        let phone = PhoneNumber::new(
            "+1-201-555-0123".to_string(),
            Some("Work Phone".to_string()),
            Some("work".to_string()),
            Some(true),
        )
        .unwrap();

        let json = serde_json::to_string(&phone).unwrap();
        assert!(json.contains("\"value\":\"+1-201-555-0123\""));
        assert!(json.contains("\"display\":\"Work Phone\""));
        assert!(json.contains("\"type\":\"work\""));
        assert!(json.contains("\"primary\":true"));
    }

    #[test]
    fn test_deserialization() {
        let json = r#"{
            "value": "+1-201-555-0123",
            "display": "Work Phone",
            "type": "work",
            "primary": true
        }"#;

        let phone: PhoneNumber = serde_json::from_str(json).unwrap();
        assert_eq!(phone.value(), "+1-201-555-0123");
        assert_eq!(phone.display(), Some("Work Phone"));
        assert_eq!(phone.phone_type(), Some("work"));
        assert!(phone.is_primary());
    }

    #[test]
    fn test_equality() {
        let phone1 = PhoneNumber::new_work("555-123-4567".to_string()).unwrap();
        let phone2 = PhoneNumber::new_work("555-123-4567".to_string()).unwrap();
        let phone3 = PhoneNumber::new_mobile("555-123-4567".to_string()).unwrap();

        assert_eq!(phone1, phone2);
        assert_ne!(phone1, phone3);
    }

    #[test]
    fn test_clone() {
        let original = PhoneNumber::new(
            "+1-555-123-4567".to_string(),
            Some("Work Phone".to_string()),
            Some("work".to_string()),
            Some(true),
        )
        .unwrap();

        let cloned = original.clone();
        assert_eq!(original, cloned);
        assert_eq!(cloned.value(), "+1-555-123-4567");
        assert_eq!(cloned.phone_type(), Some("work"));
    }

    #[test]
    fn test_valid_phone_types() {
        for phone_type in ["work", "home", "mobile", "fax", "pager", "other"] {
            let phone = PhoneNumber::new(
                "555-123-4567".to_string(),
                None,
                Some(phone_type.to_string()),
                None,
            );
            assert!(phone.is_ok(), "Phone type '{}' should be valid", phone_type);
        }
    }

    #[test]
    fn test_various_phone_formats() {
        let formats = [
            "555-123-4567",
            "(555) 123-4567",
            "+1-555-123-4567",
            "tel:+1-555-123-4567",
            "555.123.4567",
            "1 555 123 4567",
        ];

        for format in &formats {
            let phone = PhoneNumber::new_work(format.to_string());
            assert!(phone.is_ok(), "Phone format '{}' should be valid", format);
        }
    }

    #[test]
    fn test_invalid_rfc3966_format() {
        let result = PhoneNumber::new_work("tel:".to_string());
        assert!(result.is_err());
        let error_msg = result.unwrap_err().to_string();
        assert!(error_msg.contains("cannot be empty after 'tel:' prefix"));

        // Test valid RFC 3966 format
        let result2 = PhoneNumber::new(
            "tel:+1-555-123-4567".to_string(),
            None,
            Some("work".to_string()),
            None,
        );
        assert!(result2.is_ok());
    }
}
