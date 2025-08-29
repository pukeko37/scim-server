//! Meta value object for SCIM resource metadata.
//!
//! This module provides a type-safe wrapper around SCIM meta attributes with built-in validation.
//! Meta attributes contain common metadata for all SCIM resources including timestamps, location,
//! and version information.

use crate::error::{ValidationError, ValidationResult};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt;

/// A validated SCIM meta attribute.
///
/// Meta represents the common metadata for SCIM resources as defined in RFC 7643.
/// It enforces validation rules at construction time, ensuring that only valid meta
/// attributes can exist in the system.
///
/// ## Validation Rules
///
/// - Resource type must not be empty
/// - Created timestamp must be valid ISO 8601 format
/// - Last modified timestamp must be valid ISO 8601 format
/// - Last modified must not be before created timestamp
/// - Location URI, if provided, must be valid format
/// - Version, if provided, must follow ETag format
///
/// ## Examples
///
/// ```rust
/// use scim_server::resource::value_objects::Meta;
/// use chrono::Utc;
///
/// fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let now = Utc::now();
///     let meta = Meta::new(
///         "User".to_string(),
///         now,
///         now,
///         Some("https://example.com/Users/123".to_string()),
///         Some("W/\"123-456\"".to_string())
///     )?;
///     println!("Resource type: {}", meta.resource_type());
///
///     Ok(())
/// }
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Meta {
    #[serde(rename = "resourceType")]
    pub resource_type: String,
    pub created: DateTime<Utc>,
    #[serde(rename = "lastModified")]
    pub last_modified: DateTime<Utc>,
    pub location: Option<String>,
    pub version: Option<String>,
}

impl Meta {
    /// Create a new Meta with full attributes.
    ///
    /// This is the primary constructor that enforces all validation rules.
    /// Use this method when creating Meta instances from untrusted input.
    ///
    /// # Arguments
    ///
    /// * `resource_type` - The SCIM resource type (e.g., "User", "Group")
    /// * `created` - The resource creation timestamp
    /// * `last_modified` - The resource last modification timestamp
    /// * `location` - Optional location URI for the resource
    /// * `version` - Optional version identifier (ETag format)
    ///
    /// # Returns
    ///
    /// * `Ok(Meta)` - If all values are valid
    /// * `Err(ValidationError)` - If any value violates validation rules
    pub fn new(
        resource_type: String,
        created: DateTime<Utc>,
        last_modified: DateTime<Utc>,
        location: Option<String>,
        version: Option<String>,
    ) -> ValidationResult<Self> {
        Self::validate_resource_type(&resource_type)?;
        Self::validate_timestamps(created, last_modified)?;
        if let Some(ref location_val) = location {
            Self::validate_location(location_val)?;
        }
        if let Some(ref version_val) = version {
            Self::validate_version(version_val)?;
        }

        Ok(Self {
            resource_type,
            created,
            last_modified,
            location,
            version,
        })
    }

    /// Create a simple Meta with just resource type and timestamps.
    ///
    /// Convenience constructor for creating meta attributes without optional fields.
    ///
    /// # Arguments
    ///
    /// * `resource_type` - The SCIM resource type
    /// * `created` - The resource creation timestamp
    /// * `last_modified` - The resource last modification timestamp
    ///
    /// # Returns
    ///
    /// * `Ok(Meta)` - If the values are valid
    /// * `Err(ValidationError)` - If any value violates validation rules
    pub fn new_simple(
        resource_type: String,
        created: DateTime<Utc>,
        last_modified: DateTime<Utc>,
    ) -> ValidationResult<Self> {
        Self::new(resource_type, created, last_modified, None, None)
    }

    /// Create a Meta for a new resource with current timestamp.
    ///
    /// Convenience constructor for creating meta attributes for new resources.
    /// Sets both created and last_modified to the current time.
    ///
    /// # Arguments
    ///
    /// * `resource_type` - The SCIM resource type
    ///
    /// # Returns
    ///
    /// * `Ok(Meta)` - If the resource type is valid
    /// * `Err(ValidationError)` - If the resource type violates validation rules
    pub fn new_for_creation(resource_type: String) -> ValidationResult<Self> {
        let now = Utc::now();
        Self::new_simple(resource_type, now, now)
    }

    /// Get the resource type.
    pub fn resource_type(&self) -> &str {
        &self.resource_type
    }

    /// Get the created timestamp.
    pub fn created(&self) -> DateTime<Utc> {
        self.created
    }

    /// Get the last modified timestamp.
    pub fn last_modified(&self) -> DateTime<Utc> {
        self.last_modified
    }

    /// Get the location URI.
    pub fn location(&self) -> Option<&str> {
        self.location.as_deref()
    }

    /// Get the version identifier.
    pub fn version(&self) -> Option<&str> {
        self.version.as_deref()
    }

    /// Create a new Meta with updated last modified timestamp.
    ///
    /// This method creates a new Meta instance with the last_modified timestamp
    /// updated to the current time, preserving all other attributes.
    pub fn with_updated_timestamp(&self) -> Self {
        Self {
            resource_type: self.resource_type.clone(),
            created: self.created,
            last_modified: Utc::now(),
            location: self.location.clone(),
            version: self.version.clone(),
        }
    }

    /// Create a new Meta with a specific location.
    ///
    /// This method creates a new Meta instance with the location set to the
    /// provided value, preserving all other attributes.
    pub fn with_location(mut self, location: String) -> ValidationResult<Self> {
        Self::validate_location(&location)?;
        self.location = Some(location);
        Ok(self)
    }

    /// Create a new Meta with a specific version.
    ///
    /// This method creates a new Meta instance with the version set to the
    /// provided value, preserving all other attributes.
    pub fn with_version(mut self, version: String) -> ValidationResult<Self> {
        Self::validate_version(&version)?;
        self.version = Some(version);
        Ok(self)
    }

    /// Generate a location URI for the resource.
    ///
    /// Creates a standard SCIM location URI based on the base URL, resource type,
    /// and resource ID.
    pub fn generate_location(base_url: &str, resource_type: &str, resource_id: &str) -> String {
        format!(
            "{}/{}s/{}",
            base_url.trim_end_matches('/'),
            resource_type,
            resource_id
        )
    }



    /// Validate the resource type value.
    fn validate_resource_type(resource_type: &str) -> ValidationResult<()> {
        if resource_type.is_empty() {
            return Err(ValidationError::MissingResourceType);
        }

        // Resource type should be a valid identifier
        if !resource_type
            .chars()
            .all(|c| c.is_alphanumeric() || c == '_')
        {
            return Err(ValidationError::InvalidResourceType {
                resource_type: resource_type.to_string(),
            });
        }

        Ok(())
    }

    /// Validate the timestamp values.
    fn validate_timestamps(
        created: DateTime<Utc>,
        last_modified: DateTime<Utc>,
    ) -> ValidationResult<()> {
        if last_modified < created {
            return Err(ValidationError::Custom {
                message: "Last modified timestamp cannot be before created timestamp".to_string(),
            });
        }

        Ok(())
    }

    /// Validate the location URI value.
    fn validate_location(location: &str) -> ValidationResult<()> {
        if location.is_empty() {
            return Err(ValidationError::InvalidLocationUri);
        }

        // Basic URI validation - should start with http:// or https://
        if !location.starts_with("http://") && !location.starts_with("https://") {
            return Err(ValidationError::InvalidLocationUri);
        }

        Ok(())
    }

    /// Validate the version identifier value.
    fn validate_version(version: &str) -> ValidationResult<()> {
        if version.is_empty() {
            return Err(ValidationError::InvalidVersionFormat);
        }

        // Version should follow ETag format: W/"..." or "..."
        if !version.starts_with("W/\"") && !version.starts_with('"') {
            return Err(ValidationError::InvalidVersionFormat);
        }

        if !version.ends_with('"') {
            return Err(ValidationError::InvalidVersionFormat);
        }

        Ok(())
    }
}

impl fmt::Display for Meta {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Meta(resourceType={}, created={}, lastModified={})",
            self.resource_type,
            self.created.to_rfc3339(),
            self.last_modified.to_rfc3339()
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{TimeZone, Utc};
    use serde_json;

    #[test]
    fn test_valid_meta_full() {
        let created = Utc.with_ymd_and_hms(2023, 1, 1, 12, 0, 0).unwrap();
        let modified = Utc.with_ymd_and_hms(2023, 1, 2, 12, 0, 0).unwrap();

        let meta = Meta::new(
            "User".to_string(),
            created,
            modified,
            Some("https://example.com/Users/123".to_string()),
            Some("W/\"123-456\"".to_string()),
        );
        assert!(meta.is_ok());

        let meta = meta.unwrap();
        assert_eq!(meta.resource_type(), "User");
        assert_eq!(meta.created(), created);
        assert_eq!(meta.last_modified(), modified);
        assert_eq!(meta.location(), Some("https://example.com/Users/123"));
        assert_eq!(meta.version(), Some("W/\"123-456\""));
    }

    #[test]
    fn test_valid_meta_simple() {
        let created = Utc.with_ymd_and_hms(2023, 1, 1, 12, 0, 0).unwrap();
        let modified = Utc.with_ymd_and_hms(2023, 1, 1, 12, 30, 0).unwrap();

        let meta = Meta::new_simple("Group".to_string(), created, modified);
        assert!(meta.is_ok());

        let meta = meta.unwrap();
        assert_eq!(meta.resource_type(), "Group");
        assert_eq!(meta.created(), created);
        assert_eq!(meta.last_modified(), modified);
        assert_eq!(meta.location(), None);
        assert_eq!(meta.version(), None);
    }

    #[test]
    fn test_new_for_creation() {
        let meta = Meta::new_for_creation("User".to_string());
        assert!(meta.is_ok());

        let meta = meta.unwrap();
        assert_eq!(meta.resource_type(), "User");
        assert_eq!(meta.created(), meta.last_modified());
    }

    #[test]
    fn test_empty_resource_type() {
        let now = Utc::now();
        let result = Meta::new_simple("".to_string(), now, now);
        assert!(result.is_err());

        match result.unwrap_err() {
            ValidationError::MissingResourceType => {}
            other => panic!("Expected MissingResourceType error, got: {:?}", other),
        }
    }

    #[test]
    fn test_invalid_resource_type() {
        let now = Utc::now();
        let result = Meta::new_simple("Invalid-Type!".to_string(), now, now);
        assert!(result.is_err());

        match result.unwrap_err() {
            ValidationError::InvalidResourceType { resource_type } => {
                assert_eq!(resource_type, "Invalid-Type!");
            }
            other => panic!("Expected InvalidResourceType error, got: {:?}", other),
        }
    }

    #[test]
    fn test_invalid_timestamps() {
        let created = Utc.with_ymd_and_hms(2023, 1, 2, 12, 0, 0).unwrap();
        let modified = Utc.with_ymd_and_hms(2023, 1, 1, 12, 0, 0).unwrap(); // Before created

        let result = Meta::new_simple("User".to_string(), created, modified);
        assert!(result.is_err());

        match result.unwrap_err() {
            ValidationError::Custom { message } => {
                assert!(message.contains("Last modified timestamp cannot be before created"));
            }
            other => panic!("Expected Custom error, got: {:?}", other),
        }
    }

    #[test]
    fn test_invalid_location() {
        let now = Utc::now();

        // Empty location
        let result = Meta::new("User".to_string(), now, now, Some("".to_string()), None);
        assert!(result.is_err());

        // Invalid URI format
        let result = Meta::new(
            "User".to_string(),
            now,
            now,
            Some("not-a-uri".to_string()),
            None,
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_version() {
        let now = Utc::now();

        // Empty version
        let result = Meta::new("User".to_string(), now, now, None, Some("".to_string()));
        assert!(result.is_err());

        // Invalid ETag format
        let result = Meta::new(
            "User".to_string(),
            now,
            now,
            None,
            Some("invalid-etag".to_string()),
        );
        assert!(result.is_err());

        match result.unwrap_err() {
            ValidationError::InvalidVersionFormat => {}
            other => panic!("Expected InvalidVersionFormat error, got: {:?}", other),
        }
    }

    #[test]
    fn test_with_updated_timestamp() {
        let created = Utc.with_ymd_and_hms(2023, 1, 1, 12, 0, 0).unwrap();
        let meta = Meta::new_simple("User".to_string(), created, created).unwrap();

        std::thread::sleep(std::time::Duration::from_millis(10));
        let updated_meta = meta.with_updated_timestamp();

        assert_eq!(updated_meta.created(), created);
        assert!(updated_meta.last_modified() > created);
        assert_eq!(updated_meta.resource_type(), "User");
    }

    #[test]
    fn test_with_location() {
        let now = Utc::now();
        let meta = Meta::new_simple("User".to_string(), now, now).unwrap();

        let meta_with_location = meta
            .clone()
            .with_location("https://example.com/Users/123".to_string());
        assert!(meta_with_location.is_ok());

        let meta_with_location = meta_with_location.unwrap();
        assert_eq!(
            meta_with_location.location(),
            Some("https://example.com/Users/123")
        );

        // Test invalid location
        let invalid_result = meta.with_location("invalid-uri".to_string());
        assert!(invalid_result.is_err());
    }

    #[test]
    fn test_with_version() {
        let now = Utc::now();
        let meta = Meta::new_simple("User".to_string(), now, now).unwrap();

        let meta_with_version = meta.clone().with_version("W/\"123-456\"".to_string());
        assert!(meta_with_version.is_ok());

        let meta_with_version = meta_with_version.unwrap();
        assert_eq!(meta_with_version.version(), Some("W/\"123-456\""));

        // Test invalid version
        let invalid_result = meta.with_version("invalid-version".to_string());
        assert!(invalid_result.is_err());
    }

    #[test]
    fn test_generate_location() {
        let location = Meta::generate_location("https://example.com", "User", "123");
        assert_eq!(location, "https://example.com/Users/123");

        // Test with trailing slash
        let location = Meta::generate_location("https://example.com/", "Group", "456");
        assert_eq!(location, "https://example.com/Groups/456");
    }



    #[test]
    fn test_display() {
        let created = Utc.with_ymd_and_hms(2023, 1, 1, 12, 0, 0).unwrap();
        let modified = Utc.with_ymd_and_hms(2023, 1, 2, 12, 0, 0).unwrap();

        let meta = Meta::new_simple("User".to_string(), created, modified).unwrap();
        let display_str = format!("{}", meta);

        assert!(display_str.contains("User"));
        assert!(display_str.contains("2023-01-01T12:00:00"));
        assert!(display_str.contains("2023-01-02T12:00:00"));
    }

    #[test]
    fn test_serialization() {
        let created = Utc.with_ymd_and_hms(2023, 1, 1, 12, 0, 0).unwrap();
        let modified = Utc.with_ymd_and_hms(2023, 1, 2, 12, 0, 0).unwrap();

        let meta = Meta::new(
            "User".to_string(),
            created,
            modified,
            Some("https://example.com/Users/123".to_string()),
            Some("W/\"123-456\"".to_string()),
        )
        .unwrap();

        let json = serde_json::to_string(&meta).unwrap();
        assert!(json.contains("\"resourceType\":\"User\""));
        assert!(json.contains("\"lastModified\""));
        assert!(json.contains("\"location\":\"https://example.com/Users/123\""));
        assert!(json.contains("\"version\":\"W/\\\"123-456\\\"\""));
    }

    #[test]
    fn test_deserialization() {
        let json = r#"{
            "resourceType": "Group",
            "created": "2023-01-01T12:00:00Z",
            "lastModified": "2023-01-02T12:00:00Z",
            "location": "https://example.com/Groups/456",
            "version": "W/\"456-789\""
        }"#;

        let meta: Meta = serde_json::from_str(json).unwrap();
        assert_eq!(meta.resource_type(), "Group");
        assert_eq!(meta.location(), Some("https://example.com/Groups/456"));
        assert_eq!(meta.version(), Some("W/\"456-789\""));
    }

    #[test]
    fn test_equality() {
        let created = Utc.with_ymd_and_hms(2023, 1, 1, 12, 0, 0).unwrap();
        let modified = Utc.with_ymd_and_hms(2023, 1, 2, 12, 0, 0).unwrap();

        let meta1 = Meta::new_simple("User".to_string(), created, modified).unwrap();
        let meta2 = Meta::new_simple("User".to_string(), created, modified).unwrap();
        let meta3 = Meta::new_simple("Group".to_string(), created, modified).unwrap();

        assert_eq!(meta1, meta2);
        assert_ne!(meta1, meta3);
    }

    #[test]
    fn test_clone() {
        let created = Utc.with_ymd_and_hms(2023, 1, 1, 12, 0, 0).unwrap();
        let modified = Utc.with_ymd_and_hms(2023, 1, 2, 12, 0, 0).unwrap();

        let meta = Meta::new(
            "User".to_string(),
            created,
            modified,
            Some("https://example.com/Users/123".to_string()),
            Some("W/\"123-456\"".to_string()),
        )
        .unwrap();

        let cloned = meta.clone();
        assert_eq!(meta, cloned);
        assert_eq!(meta.resource_type(), cloned.resource_type());
        assert_eq!(meta.created(), cloned.created());
        assert_eq!(meta.last_modified(), cloned.last_modified());
        assert_eq!(meta.location(), cloned.location());
        assert_eq!(meta.version(), cloned.version());
    }
}
