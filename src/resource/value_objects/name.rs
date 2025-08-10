//! Name value object for SCIM user name components.
//!
//! This module provides a type-safe wrapper around SCIM name attributes with built-in validation.
//! Name attributes represent the components of a user's real name as defined in RFC 7643 Section 4.1.1.

use crate::error::{ValidationError, ValidationResult};
use crate::resource::value_objects::value_object_trait::{SchemaConstructible, ValueObject};
use crate::schema::types::{AttributeDefinition, AttributeType};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::any::Any;
use std::fmt;

/// A validated SCIM name attribute.
///
/// Name represents the components of a user's real name as defined in RFC 7643.
/// It enforces validation rules at construction time, ensuring that only valid name
/// attributes can exist in the system.
///
/// ## Validation Rules
///
/// - At least one name component must be provided (not all fields can be empty/None)
/// - Individual name components cannot be empty strings
/// - All fields are optional but if provided must contain meaningful content
///
/// ## Examples
///
/// ```rust
/// use scim_server::resource::value_objects::Name;
///
/// fn main() -> Result<(), Box<dyn std::error::Error>> {
///     // Create with full name components
///     let name = Name::new(
///         Some("Ms. Barbara J Jensen, III".to_string()),
///         Some("Jensen".to_string()),
///         Some("Barbara".to_string()),
///         Some("Jane".to_string()),
///         Some("Ms.".to_string()),
///         Some("III".to_string())
///     )?;
///
///     // Create with minimal components
///     let simple_name = Name::new_simple(
///         "John".to_string(),
///         "Doe".to_string()
///     )?;
///
///     Ok(())
/// }
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Name {
    pub formatted: Option<String>,
    #[serde(rename = "familyName")]
    pub family_name: Option<String>,
    #[serde(rename = "givenName")]
    pub given_name: Option<String>,
    #[serde(rename = "middleName")]
    pub middle_name: Option<String>,
    #[serde(rename = "honorificPrefix")]
    pub honorific_prefix: Option<String>,
    #[serde(rename = "honorificSuffix")]
    pub honorific_suffix: Option<String>,
}

impl Name {
    /// Create a new Name with all components.
    ///
    /// This is the primary constructor that enforces all validation rules.
    /// Use this method when creating Name instances from untrusted input.
    ///
    /// # Arguments
    ///
    /// * `formatted` - The full name, formatted for display
    /// * `family_name` - The family name or last name
    /// * `given_name` - The given name or first name
    /// * `middle_name` - The middle name(s)
    /// * `honorific_prefix` - The honorific prefix or title (e.g., "Ms.", "Dr.")
    /// * `honorific_suffix` - The honorific suffix (e.g., "III", "Jr.")
    ///
    /// # Returns
    ///
    /// * `Ok(Name)` - If at least one field is provided and all provided fields are valid
    /// * `Err(ValidationError)` - If all fields are None/empty or any field violates validation rules
    pub fn new(
        formatted: Option<String>,
        family_name: Option<String>,
        given_name: Option<String>,
        middle_name: Option<String>,
        honorific_prefix: Option<String>,
        honorific_suffix: Option<String>,
    ) -> ValidationResult<Self> {
        // Validate individual components
        if let Some(ref f) = formatted {
            Self::validate_name_component(f, "formatted")?;
        }
        if let Some(ref fn_val) = family_name {
            Self::validate_name_component(fn_val, "familyName")?;
        }
        if let Some(ref gn) = given_name {
            Self::validate_name_component(gn, "givenName")?;
        }
        if let Some(ref mn) = middle_name {
            Self::validate_name_component(mn, "middleName")?;
        }
        if let Some(ref hp) = honorific_prefix {
            Self::validate_name_component(hp, "honorificPrefix")?;
        }
        if let Some(ref hs) = honorific_suffix {
            Self::validate_name_component(hs, "honorificSuffix")?;
        }

        // Ensure at least one component is provided
        if formatted.is_none()
            && family_name.is_none()
            && given_name.is_none()
            && middle_name.is_none()
            && honorific_prefix.is_none()
            && honorific_suffix.is_none()
        {
            return Err(ValidationError::custom(
                "At least one name component must be provided",
            ));
        }

        Ok(Self {
            formatted,
            family_name,
            given_name,
            middle_name,
            honorific_prefix,
            honorific_suffix,
        })
    }

    /// Create a simple Name with just given and family names.
    ///
    /// Convenience constructor for creating basic name structures.
    ///
    /// # Arguments
    ///
    /// * `given_name` - The given name or first name
    /// * `family_name` - The family name or last name
    ///
    /// # Returns
    ///
    /// * `Ok(Name)` - If the names are valid
    /// * `Err(ValidationError)` - If any name violates validation rules
    pub fn new_simple(given_name: String, family_name: String) -> ValidationResult<Self> {
        Self::new(None, Some(family_name), Some(given_name), None, None, None)
    }

    /// Create a Name with a formatted display name only.
    ///
    /// Convenience constructor for cases where only a formatted name is available.
    ///
    /// # Arguments
    ///
    /// * `formatted` - The full formatted name
    ///
    /// # Returns
    ///
    /// * `Ok(Name)` - If the formatted name is valid
    /// * `Err(ValidationError)` - If the name violates validation rules
    pub fn new_formatted(formatted: String) -> ValidationResult<Self> {
        Self::new(Some(formatted), None, None, None, None, None)
    }

    /// Create a Name instance without validation for internal use.
    ///
    /// This method bypasses validation and should only be used when the data
    /// is known to be valid, such as when deserializing from trusted sources.
    ///
    /// # Safety
    ///
    /// The caller must ensure that the provided values are valid according to
    /// SCIM name validation rules.
    #[allow(dead_code)]
    pub(crate) fn new_unchecked(
        formatted: Option<String>,
        family_name: Option<String>,
        given_name: Option<String>,
        middle_name: Option<String>,
        honorific_prefix: Option<String>,
        honorific_suffix: Option<String>,
    ) -> Self {
        Self {
            formatted,
            family_name,
            given_name,
            middle_name,
            honorific_prefix,
            honorific_suffix,
        }
    }

    /// Get the formatted name.
    pub fn formatted(&self) -> Option<&str> {
        self.formatted.as_deref()
    }

    /// Get the family name.
    pub fn family_name(&self) -> Option<&str> {
        self.family_name.as_deref()
    }

    /// Get the given name.
    pub fn given_name(&self) -> Option<&str> {
        self.given_name.as_deref()
    }

    /// Get the middle name.
    pub fn middle_name(&self) -> Option<&str> {
        self.middle_name.as_deref()
    }

    /// Get the honorific prefix.
    pub fn honorific_prefix(&self) -> Option<&str> {
        self.honorific_prefix.as_deref()
    }

    /// Get the honorific suffix.
    pub fn honorific_suffix(&self) -> Option<&str> {
        self.honorific_suffix.as_deref()
    }

    /// Generate a formatted display name from components.
    ///
    /// Creates a formatted name string from the available name components
    /// if no explicit formatted name is provided.
    ///
    /// # Returns
    ///
    /// The formatted name if available, otherwise a constructed name from components,
    /// or None if no components are available.
    pub fn display_name(&self) -> Option<String> {
        if let Some(ref formatted) = self.formatted {
            return Some(formatted.clone());
        }

        let mut parts = Vec::new();

        if let Some(ref prefix) = self.honorific_prefix {
            parts.push(prefix.as_str());
        }
        if let Some(ref given) = self.given_name {
            parts.push(given.as_str());
        }
        if let Some(ref middle) = self.middle_name {
            parts.push(middle.as_str());
        }
        if let Some(ref family) = self.family_name {
            parts.push(family.as_str());
        }
        if let Some(ref suffix) = self.honorific_suffix {
            parts.push(suffix.as_str());
        }

        if parts.is_empty() {
            None
        } else {
            Some(parts.join(" "))
        }
    }

    /// Check if the name has any meaningful content.
    pub fn is_empty(&self) -> bool {
        self.formatted.is_none()
            && self.family_name.is_none()
            && self.given_name.is_none()
            && self.middle_name.is_none()
            && self.honorific_prefix.is_none()
            && self.honorific_suffix.is_none()
    }

    /// Validate a name component.
    fn validate_name_component(value: &str, field_name: &str) -> ValidationResult<()> {
        if value.trim().is_empty() {
            return Err(ValidationError::custom(format!(
                "{}: Name component cannot be empty or contain only whitespace",
                field_name
            )));
        }

        // Check for reasonable length (SCIM doesn't specify but let's be practical)
        if value.len() > 256 {
            return Err(ValidationError::custom(format!(
                "{}: Name component exceeds maximum length of 256 characters",
                field_name
            )));
        }

        // Check for control characters that shouldn't be in names
        if value
            .chars()
            .any(|c| c.is_control() && c != '\n' && c != '\r' && c != '\t')
        {
            return Err(ValidationError::custom(format!(
                "{}: Name component contains invalid control characters",
                field_name
            )));
        }

        Ok(())
    }

    /// Create a Name from a JSON value.
    pub fn from_json(value: &Value) -> ValidationResult<Self> {
        if let Value::Object(obj) = value {
            let formatted = obj
                .get("formatted")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());

            let family_name = obj
                .get("familyName")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());

            let given_name = obj
                .get("givenName")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());

            let middle_name = obj
                .get("middleName")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());

            let honorific_prefix = obj
                .get("honorificPrefix")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());

            let honorific_suffix = obj
                .get("honorificSuffix")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());

            Self::new(
                formatted,
                family_name,
                given_name,
                middle_name,
                honorific_prefix,
                honorific_suffix,
            )
        } else {
            Err(ValidationError::InvalidAttributeType {
                attribute: "name".to_string(),
                expected: "object".to_string(),
                actual: "non-object".to_string(),
            })
        }
    }
}

impl ValueObject for Name {
    fn attribute_type(&self) -> AttributeType {
        AttributeType::Complex
    }

    fn attribute_name(&self) -> &str {
        "name"
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

        if definition.name != "name" {
            return Err(ValidationError::InvalidAttributeName {
                actual: definition.name.clone(),
                expected: "name".to_string(),
            });
        }

        Ok(())
    }

    fn as_json_value(&self) -> Value {
        serde_json::to_value(self).unwrap_or(Value::Null)
    }

    fn supports_definition(&self, definition: &AttributeDefinition) -> bool {
        definition.data_type == AttributeType::Complex && definition.name == "name"
    }

    fn clone_boxed(&self) -> Box<dyn ValueObject> {
        Box::new(self.clone())
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl SchemaConstructible for Name {
    fn from_schema_and_value(
        definition: &AttributeDefinition,
        value: &Value,
    ) -> ValidationResult<Self> {
        if definition.name != "name" || definition.data_type != AttributeType::Complex {
            return Err(ValidationError::UnsupportedAttributeType {
                attribute: definition.name.clone(),
                type_name: format!("{:?}", definition.data_type),
            });
        }

        Self::from_json(value)
    }

    fn can_construct_from(definition: &AttributeDefinition) -> bool {
        definition.name == "name" && definition.data_type == AttributeType::Complex
    }

    fn constructor_priority() -> u8 {
        100 // High priority for exact name match
    }
}

impl fmt::Display for Name {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.display_name() {
            Some(name) => write!(f, "{}", name),
            None => write!(f, "[Empty Name]"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_name_full() {
        let name = Name::new(
            Some("Ms. Barbara J Jensen, III".to_string()),
            Some("Jensen".to_string()),
            Some("Barbara".to_string()),
            Some("Jane".to_string()),
            Some("Ms.".to_string()),
            Some("III".to_string()),
        );

        assert!(name.is_ok());
        let name = name.unwrap();
        assert_eq!(name.formatted(), Some("Ms. Barbara J Jensen, III"));
        assert_eq!(name.family_name(), Some("Jensen"));
        assert_eq!(name.given_name(), Some("Barbara"));
        assert_eq!(name.middle_name(), Some("Jane"));
        assert_eq!(name.honorific_prefix(), Some("Ms."));
        assert_eq!(name.honorific_suffix(), Some("III"));
    }

    #[test]
    fn test_valid_name_simple() {
        let name = Name::new_simple("John".to_string(), "Doe".to_string());

        assert!(name.is_ok());
        let name = name.unwrap();
        assert_eq!(name.given_name(), Some("John"));
        assert_eq!(name.family_name(), Some("Doe"));
        assert_eq!(name.formatted(), None);
    }

    #[test]
    fn test_valid_name_formatted_only() {
        let name = Name::new_formatted("John Doe".to_string());

        assert!(name.is_ok());
        let name = name.unwrap();
        assert_eq!(name.formatted(), Some("John Doe"));
        assert_eq!(name.given_name(), None);
        assert_eq!(name.family_name(), None);
    }

    #[test]
    fn test_empty_name_components() {
        let result = Name::new(Some("".to_string()), None, None, None, None, None);
        assert!(result.is_err());
    }

    #[test]
    fn test_whitespace_only_components() {
        let result = Name::new(None, Some("   ".to_string()), None, None, None, None);
        assert!(result.is_err());
    }

    #[test]
    fn test_all_none_components() {
        let result = Name::new(None, None, None, None, None, None);
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("At least one name component")
        );
    }

    #[test]
    fn test_too_long_component() {
        let long_name = "a".repeat(300);
        let result = Name::new_formatted(long_name);
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("exceeds maximum length")
        );
    }

    #[test]
    fn test_control_characters() {
        let result = Name::new_formatted("John\x00Doe".to_string());
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("invalid control characters")
        );
    }

    #[test]
    fn test_display_name_with_formatted() {
        let name = Name::new_formatted("Dr. John Smith Jr.".to_string()).unwrap();
        assert_eq!(name.display_name(), Some("Dr. John Smith Jr.".to_string()));
    }

    #[test]
    fn test_display_name_from_components() {
        let name = Name::new(
            None,
            Some("Smith".to_string()),
            Some("John".to_string()),
            Some("Michael".to_string()),
            Some("Dr.".to_string()),
            Some("Jr.".to_string()),
        )
        .unwrap();

        assert_eq!(
            name.display_name(),
            Some("Dr. John Michael Smith Jr.".to_string())
        );
    }

    #[test]
    fn test_display_name_partial_components() {
        let name = Name::new(
            None,
            Some("Doe".to_string()),
            Some("Jane".to_string()),
            None,
            None,
            None,
        )
        .unwrap();

        assert_eq!(name.display_name(), Some("Jane Doe".to_string()));
    }

    #[test]
    fn test_is_empty() {
        let empty_name = Name::new_unchecked(None, None, None, None, None, None);
        assert!(empty_name.is_empty());

        let non_empty_name = Name::new_simple("John".to_string(), "Doe".to_string()).unwrap();
        assert!(!non_empty_name.is_empty());
    }

    #[test]
    fn test_new_unchecked() {
        let name = Name::new_unchecked(
            Some("John Doe".to_string()),
            Some("Doe".to_string()),
            Some("John".to_string()),
            None,
            None,
            None,
        );

        assert_eq!(name.formatted(), Some("John Doe"));
        assert_eq!(name.family_name(), Some("Doe"));
        assert_eq!(name.given_name(), Some("John"));
    }

    #[test]
    fn test_display() {
        let name = Name::new_simple("John".to_string(), "Doe".to_string()).unwrap();
        assert_eq!(format!("{}", name), "John Doe");

        let empty_name = Name::new_unchecked(None, None, None, None, None, None);
        assert_eq!(format!("{}", empty_name), "[Empty Name]");
    }

    #[test]
    fn test_serialization() {
        let name = Name::new(
            Some("Ms. Barbara J Jensen, III".to_string()),
            Some("Jensen".to_string()),
            Some("Barbara".to_string()),
            Some("Jane".to_string()),
            Some("Ms.".to_string()),
            Some("III".to_string()),
        )
        .unwrap();

        let json = serde_json::to_string(&name).unwrap();
        assert!(json.contains("\"formatted\":\"Ms. Barbara J Jensen, III\""));
        assert!(json.contains("\"familyName\":\"Jensen\""));
        assert!(json.contains("\"givenName\":\"Barbara\""));
    }

    #[test]
    fn test_deserialization() {
        let json = r#"{
            "formatted": "Ms. Barbara J Jensen, III",
            "familyName": "Jensen",
            "givenName": "Barbara",
            "middleName": "Jane",
            "honorificPrefix": "Ms.",
            "honorificSuffix": "III"
        }"#;

        let name: Name = serde_json::from_str(json).unwrap();
        assert_eq!(name.formatted(), Some("Ms. Barbara J Jensen, III"));
        assert_eq!(name.family_name(), Some("Jensen"));
        assert_eq!(name.given_name(), Some("Barbara"));
    }

    #[test]
    fn test_equality() {
        let name1 = Name::new_simple("John".to_string(), "Doe".to_string()).unwrap();
        let name2 = Name::new_simple("John".to_string(), "Doe".to_string()).unwrap();
        let name3 = Name::new_simple("Jane".to_string(), "Doe".to_string()).unwrap();

        assert_eq!(name1, name2);
        assert_ne!(name1, name3);
    }

    #[test]
    fn test_clone() {
        let original = Name::new(
            Some("Dr. John Smith".to_string()),
            Some("Smith".to_string()),
            Some("John".to_string()),
            None,
            Some("Dr.".to_string()),
            None,
        )
        .unwrap();

        let cloned = original.clone();
        assert_eq!(original, cloned);
        assert_eq!(cloned.formatted(), Some("Dr. John Smith"));
        assert_eq!(cloned.family_name(), Some("Smith"));
    }

    #[test]
    fn test_allows_newlines_in_formatted() {
        let name = Name::new_formatted("John\nDoe".to_string());
        assert!(name.is_ok());
    }

    #[test]
    fn test_allows_tabs_in_formatted() {
        let name = Name::new_formatted("John\tDoe".to_string());
        assert!(name.is_ok());
    }
}
