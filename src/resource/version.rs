//! Version control types for SCIM resources.
//!
//! This module provides types and functionality for handling resource versioning
//! and conditional operations, enabling ETag-based concurrency control as specified
//! in RFC 7644 (SCIM 2.0) and RFC 7232 (HTTP ETags).
//!
//! # ETag Concurrency Control
//!
//! The version system provides automatic optimistic concurrency control for SCIM
//! resources, preventing lost updates when multiple clients modify the same resource
//! simultaneously. All versions are computed deterministically from resource content
//! using SHA-256 hashing.
//!
//! # Core Types
//!
//! * [`ScimVersion`] - Opaque version identifier for resources
//! * [`ConditionalResult`] - Result type for conditional operations
//! * [`VersionConflict`] - Error details for version mismatches
//!
//! # Basic Usage
//!
//! ```rust
//! use scim_server::resource::version::{ScimVersion, ConditionalResult};
//!
//! // Create version from hash string (for provider-specific versioning)
//! let version = ScimVersion::from_hash("db-sequence-123");
//!
//! // Create version from content hash (automatic versioning)
//! let resource_json = br#"{"id":"123","userName":"john.doe","active":true}"#;
//! let content_version = ScimVersion::from_content(resource_json);
//!
//! // Parse from HTTP weak ETag header (client-provided versions)
//! let etag_version = ScimVersion::parse_http_header("W/\"abc123def\"").unwrap();
//!
//! // Convert to HTTP weak ETag header (for responses)
//! let etag_header = version.to_http_header(); // Returns: "W/abc123def"
//!
//! // Check version equality (for conditional operations)
//! let matches = version.matches(&etag_version);
//! ```
//!
//! # Conditional Operations
//!
//! ```rust,no_run
//! use scim_server::resource::version::{ConditionalResult, ScimVersion};
//! use scim_server::resource::{ResourceProvider, RequestContext};
//! use serde_json::json;
//!
//! # async fn example<P: ResourceProvider + Sync>(provider: &P) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
//! let context = RequestContext::with_generated_id();
//! let expected_version = ScimVersion::from_hash("current-version");
//! let update_data = json!({"userName": "updated.name", "active": false});
//!
//! // Conditional update with version checking
//! match provider.conditional_update("User", "123", update_data, &expected_version, &context).await? {
//!     ConditionalResult::Success(versioned_resource) => {
//!         println!("Update successful!");
//!         println!("New weak ETag: {}", versioned_resource.version().to_http_header());
//!     },
//!     ConditionalResult::VersionMismatch(conflict) => {
//!         println!("Version conflict detected!");
//!         println!("Expected: {}", conflict.expected);
//!         println!("Current: {}", conflict.current);
//!         println!("Message: {}", conflict.message);
//!         // Client should refresh and retry with current version
//!     },
//!     ConditionalResult::NotFound => {
//!         println!("Resource not found");
//!         // Handle missing resource scenario
//!     }
//! }
//! # Ok(())
//! # }
//! ```
//!
//! # HTTP Integration
//!
//! The version system integrates seamlessly with HTTP weak ETags:
//!
//! ```rust
//! use scim_server::resource::version::ScimVersion;
//!
//! // Server generates weak ETag for response
//! let resource_data = br#"{"id":"123","userName":"alice","active":true}"#;
//! let version = ScimVersion::from_content(resource_data);
//! let etag_header = version.to_http_header(); // "W/xyz789abc"
//!
//! // Client provides weak ETag in subsequent request (If-Match header)
//! let client_etag = "W/\"xyz789abc\"";
//! let client_version = ScimVersion::parse_http_header(client_etag).unwrap();
//!
//! // Server validates version before operation
//! if version.matches(&client_version) {
//!     println!("Versions match - proceed with operation");
//! } else {
//!     println!("Version mismatch - return 412 Precondition Failed");
//! }
//! ```
//!
//! # Version Properties
//!
//! - **Deterministic**: Same content always produces the same version
//! - **Content-Based**: Any change to resource data changes the version
//! - **Collision-Resistant**: SHA-256 based hashing prevents accidental conflicts
//! - **Compact**: Base64 encoded for efficient transmission
//! - **Opaque**: Internal representation prevents manipulation
//! - **HTTP Compatible**: Direct integration with weak ETag headers

use base64::{Engine, engine::general_purpose::STANDARD as BASE64};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fmt;
use thiserror::Error;

/// Opaque version identifier for SCIM resources.
///
/// Represents a version of a resource that can be used for optimistic concurrency
/// control. The internal representation is opaque to prevent direct manipulation
/// and ensure version consistency across different provider implementations.
///
/// Versions can be created from:
/// - Provider-specific identifiers (database sequence numbers, timestamps, etc.)
/// - Content hashes (for stateless version generation)
/// - HTTP ETag headers (for parsing client-provided versions)
///
/// # Examples
///
/// ```rust
/// use scim_server::resource::version::ScimVersion;
///
/// // From hash string
/// let version = ScimVersion::from_hash("12345");
///
/// // From content hash
/// let content = br#"{"id":"123","name":"John Doe"}"#;
/// let hash_version = ScimVersion::from_content(content);
///
/// // From HTTP ETag
/// let etag_version = ScimVersion::parse_http_header("\"abc123def\"").unwrap();
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ScimVersion {
    /// Opaque version identifier
    opaque: String,
}

impl ScimVersion {
    /// Create a version from resource content.
    ///
    /// This generates a deterministic hash-based version from the resource content,
    /// ensuring universal compatibility across all provider implementations.
    /// The version is based on the full resource content including all fields.
    ///
    /// # Arguments
    /// * `content` - The complete resource content as bytes
    ///
    /// # Examples
    /// ```rust
    /// use scim_server::resource::version::ScimVersion;
    ///
    /// let resource_json = br#"{"id":"123","userName":"john.doe"}"#;
    /// let version = ScimVersion::from_content(resource_json);
    /// ```
    pub fn from_content(content: &[u8]) -> Self {
        let mut hasher = Sha256::new();
        hasher.update(content);
        let hash = hasher.finalize();
        let encoded = BASE64.encode(&hash[..8]); // Use first 8 bytes for shorter ETags
        Self { opaque: encoded }
    }

    /// Create a version from a pre-computed hash string.
    ///
    /// This is useful for provider-specific versioning schemes such as database
    /// sequence numbers, timestamps, or UUIDs. The provider can use any string
    /// as a version identifier.
    ///
    /// # Arguments
    /// * `hash_string` - Provider-specific version identifier
    ///
    /// # Examples
    /// ```rust
    /// use scim_server::resource::version::ScimVersion;
    ///
    /// // Database sequence number
    /// let db_version = ScimVersion::from_hash("seq_12345");
    ///
    /// // Timestamp-based version
    /// let time_version = ScimVersion::from_hash("1703123456789");
    ///
    /// // UUID-based version
    /// let uuid_version = ScimVersion::from_hash("550e8400-e29b-41d4-a716-446655440000");
    /// ```
    ///
    /// # Examples
    /// ```rust
    /// use scim_server::resource::version::ScimVersion;
    ///
    /// let version = ScimVersion::from_hash("abc123def");
    /// ```
    pub fn from_hash(hash_string: impl AsRef<str>) -> Self {
        Self {
            opaque: hash_string.as_ref().to_string(),
        }
    }

    /// Parse a version from an HTTP ETag header value.
    ///
    /// Accepts both weak and strong ETags as defined in RFC 7232.
    /// Weak ETags (prefixed with "W/") are treated the same as strong ETags
    /// for SCIM resource versioning purposes.
    ///
    /// # Arguments
    /// * `etag_header` - The ETag header value (e.g., "\"abc123\"" or "W/\"abc123\"")
    ///
    /// # Returns
    /// The parsed version or an error if the ETag format is invalid
    ///
    /// # Examples
    /// ```rust
    /// use scim_server::resource::version::ScimVersion;
    ///
    /// let version = ScimVersion::parse_http_header("\"abc123\"").unwrap();
    /// let weak_version = ScimVersion::parse_http_header("W/\"abc123\"").unwrap();
    /// ```
    pub fn parse_http_header(etag_header: &str) -> Result<Self, VersionError> {
        let trimmed = etag_header.trim();

        // Handle weak ETags by removing W/ prefix
        let etag_value = if trimmed.starts_with("W/") {
            &trimmed[2..]
        } else {
            trimmed
        };

        // Remove surrounding quotes
        if etag_value.len() < 2 || !etag_value.starts_with('"') || !etag_value.ends_with('"') {
            return Err(VersionError::InvalidEtagFormat(etag_header.to_string()));
        }

        let opaque = etag_value[1..etag_value.len() - 1].to_string();

        if opaque.is_empty() {
            return Err(VersionError::InvalidEtagFormat(etag_header.to_string()));
        }

        Ok(Self { opaque })
    }

    /// Convert version to HTTP ETag header value.
    ///
    /// This generates a weak HTTP ETag header value that can be used in conditional
    /// HTTP requests. SCIM resources use weak ETags since they represent semantic
    /// equivalence rather than byte-for-byte identity. The returned value includes
    /// the W/ prefix and surrounding quotes required by RFC 7232.
    ///
    /// # Examples
    /// ```rust
    /// use scim_server::resource::version::ScimVersion;
    ///
    /// let version = ScimVersion::from_hash("12345");
    /// let etag = version.to_http_header();
    /// assert_eq!(etag, "W/\"12345\"");
    /// ```
    pub fn to_http_header(&self) -> String {
        format!("W/\"{}\"", self.opaque)
    }

    /// Check if this version matches another version.
    ///
    /// This is used for conditional operations to determine if the expected
    /// version matches the current version of a resource.
    ///
    /// # Arguments
    /// * `other` - The version to compare against
    ///
    /// # Examples
    /// ```rust
    /// use scim_server::resource::version::ScimVersion;
    ///
    /// let v1 = ScimVersion::from_hash("123");
    /// let v2 = ScimVersion::from_hash("123");
    /// let v3 = ScimVersion::from_hash("456");
    ///
    /// assert!(v1.matches(&v2));
    /// assert!(!v1.matches(&v3));
    /// ```
    pub fn matches(&self, other: &ScimVersion) -> bool {
        self.opaque == other.opaque
    }

    /// Get the opaque version string.
    ///
    /// This is primarily for internal use and debugging. The opaque string
    /// should not be relied upon for any business logic.
    pub fn as_str(&self) -> &str {
        &self.opaque
    }
}

impl fmt::Display for ScimVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.opaque)
    }
}

/// Result type for conditional SCIM operations.
///
/// Represents the outcome of a conditional operation that depends on
/// resource versioning. This allows providers to indicate whether
/// an operation succeeded, failed due to a version mismatch, or
/// failed because the resource was not found.
///
/// # Examples
///
/// ```rust
/// use scim_server::resource::version::{ConditionalResult, ScimVersion, VersionConflict};
/// use serde_json::json;
///
/// // Successful operation
/// let success = ConditionalResult::Success(json!({"id": "123"}));
///
/// // Version mismatch
/// let expected = ScimVersion::from_hash("1");
/// let current = ScimVersion::from_hash("2");
/// let conflict: ConditionalResult<serde_json::Value> = ConditionalResult::VersionMismatch(VersionConflict {
///     expected,
///     current,
///     message: "Resource was modified by another client".to_string(),
/// });
///
/// // Resource not found
/// let not_found: ConditionalResult<serde_json::Value> = ConditionalResult::NotFound;
/// ```
#[derive(Debug, Clone, PartialEq)]
pub enum ConditionalResult<T> {
    /// Operation completed successfully
    Success(T),

    /// Operation failed due to version mismatch
    VersionMismatch(VersionConflict),

    /// Operation failed because the resource was not found
    NotFound,
}

impl<T> ConditionalResult<T> {
    /// Check if the result represents a successful operation.
    pub fn is_success(&self) -> bool {
        matches!(self, ConditionalResult::Success(_))
    }

    /// Check if the result represents a version mismatch.
    pub fn is_version_mismatch(&self) -> bool {
        matches!(self, ConditionalResult::VersionMismatch(_))
    }

    /// Check if the result represents a not found error.
    pub fn is_not_found(&self) -> bool {
        matches!(self, ConditionalResult::NotFound)
    }

    /// Extract the success value, if present.
    pub fn into_success(self) -> Option<T> {
        match self {
            ConditionalResult::Success(value) => Some(value),
            _ => None,
        }
    }

    /// Extract the version conflict, if present.
    pub fn into_version_conflict(self) -> Option<VersionConflict> {
        match self {
            ConditionalResult::VersionMismatch(conflict) => Some(conflict),
            _ => None,
        }
    }

    /// Map the success value to a different type.
    pub fn map<U, F>(self, f: F) -> ConditionalResult<U>
    where
        F: FnOnce(T) -> U,
    {
        match self {
            ConditionalResult::Success(value) => ConditionalResult::Success(f(value)),
            ConditionalResult::VersionMismatch(conflict) => {
                ConditionalResult::VersionMismatch(conflict)
            }
            ConditionalResult::NotFound => ConditionalResult::NotFound,
        }
    }
}

/// Details about a version conflict during a conditional operation.
///
/// Provides information about the expected version (from the client)
/// and the current version (from the server), along with a human-readable
/// error message.
///
/// # Examples
///
/// ```rust
/// use scim_server::resource::version::{VersionConflict, ScimVersion};
///
/// let conflict = VersionConflict {
///     expected: ScimVersion::from_hash("1"),
///     current: ScimVersion::from_hash("2"),
///     message: "Resource was modified by another client".to_string(),
/// };
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VersionConflict {
    /// The version that was expected by the client
    pub expected: ScimVersion,

    /// The current version of the resource on the server
    pub current: ScimVersion,

    /// Human-readable error message describing the conflict
    pub message: String,
}

impl VersionConflict {
    /// Create a new version conflict.
    ///
    /// # Arguments
    /// * `expected` - The version expected by the client
    /// * `current` - The current version on the server
    /// * `message` - Human-readable error message
    pub fn new(expected: ScimVersion, current: ScimVersion, message: impl Into<String>) -> Self {
        Self {
            expected,
            current,
            message: message.into(),
        }
    }

    /// Create a standard version conflict message.
    ///
    /// # Arguments
    /// * `expected` - The version expected by the client
    /// * `current` - The current version on the server
    pub fn standard_message(expected: ScimVersion, current: ScimVersion) -> Self {
        Self::new(
            expected,
            current,
            "Resource was modified by another client. Please refresh and try again.",
        )
    }
}

impl fmt::Display for VersionConflict {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Version conflict: expected '{}', found '{}'. {}",
            self.expected, self.current, self.message
        )
    }
}

impl std::error::Error for VersionConflict {}

/// Errors that can occur during version operations.
#[derive(Debug, Error, Clone, PartialEq)]
pub enum VersionError {
    /// Invalid ETag format provided
    #[error("Invalid ETag format: {0}")]
    InvalidEtagFormat(String),

    /// Version parsing failed
    #[error("Failed to parse version: {0}")]
    ParseError(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_from_content() {
        let content = br#"{"id":"123","userName":"john.doe"}"#;
        let version = ScimVersion::from_content(content);

        // Version should be deterministic
        let version2 = ScimVersion::from_content(content);
        assert_eq!(version, version2);

        // Different content should produce different versions
        let different_content = br#"{"id":"123","userName":"jane.doe"}"#;
        let different_version = ScimVersion::from_content(different_content);
        assert_ne!(version, different_version);
    }

    #[test]
    fn test_version_from_hash() {
        let hash_string = "abc123def456";
        let version = ScimVersion::from_hash(hash_string);
        assert_eq!(version.as_str(), hash_string);
        assert_eq!(version.to_http_header(), "W/\"abc123def456\"");

        // Test with different hash strings
        let version2 = ScimVersion::from_hash("different123");
        assert_ne!(version, version2);
    }

    #[test]
    fn test_version_parse_http_header() {
        // Strong ETag
        let version = ScimVersion::parse_http_header("\"abc123\"").unwrap();
        assert_eq!(version.as_str(), "abc123");

        // Weak ETag
        let weak_version = ScimVersion::parse_http_header("W/\"abc123\"").unwrap();
        assert_eq!(weak_version.as_str(), "abc123");

        // Invalid formats
        assert!(ScimVersion::parse_http_header("abc123").is_err());
        assert!(ScimVersion::parse_http_header("\"\"").is_err());
        assert!(ScimVersion::parse_http_header("").is_err());
    }

    #[test]
    fn test_version_matches() {
        let content = br#"{"id":"123","data":"test"}"#;
        let v1 = ScimVersion::from_content(content);
        let v2 = ScimVersion::from_content(content);
        let v3 = ScimVersion::from_content(br#"{"id":"456","data":"test"}"#);

        assert!(v1.matches(&v2));
        assert!(!v1.matches(&v3));
    }

    #[test]
    fn test_version_round_trip() {
        let content = br#"{"id":"test","version":"round-trip"}"#;
        let original = ScimVersion::from_content(content);
        let etag = original.to_http_header();
        let parsed = ScimVersion::parse_http_header(&etag).unwrap();

        assert_eq!(original, parsed);
    }

    #[test]
    fn test_conditional_result() {
        let success: ConditionalResult<i32> = ConditionalResult::Success(42);
        assert!(success.is_success());
        assert_eq!(success.into_success(), Some(42));

        let conflict = ConditionalResult::<i32>::VersionMismatch(VersionConflict::new(
            ScimVersion::from_hash("version1"),
            ScimVersion::from_hash("version2"),
            "test conflict",
        ));
        assert!(conflict.is_version_mismatch());

        let not_found: ConditionalResult<i32> = ConditionalResult::NotFound;
        assert!(not_found.is_not_found());
    }

    #[test]
    fn test_conditional_result_map() {
        let success: ConditionalResult<i32> = ConditionalResult::Success(42);
        let mapped = success.map(|x| x.to_string());
        assert_eq!(mapped.into_success(), Some("42".to_string()));
    }

    #[test]
    fn test_version_conflict() {
        let conflict = VersionConflict::standard_message(
            ScimVersion::from_hash("version1"),
            ScimVersion::from_hash("version2"),
        );

        assert_eq!(conflict.expected.as_str(), "version1");
        assert_eq!(conflict.current.as_str(), "version2");
        assert!(!conflict.message.is_empty());
    }

    #[test]
    fn test_version_conflict_display() {
        let conflict = VersionConflict::new(
            ScimVersion::from_hash("old-hash"),
            ScimVersion::from_hash("new-hash"),
            "Custom message",
        );

        let display = format!("{}", conflict);
        assert!(display.contains("old-hash"));
        assert!(display.contains("new-hash"));
        assert!(display.contains("Custom message"));
    }

    #[test]
    fn test_version_serialization() {
        let content = br#"{"test":"serialization"}"#;
        let version = ScimVersion::from_content(content);

        // Test JSON serialization
        let json = serde_json::to_string(&version).unwrap();
        let deserialized: ScimVersion = serde_json::from_str(&json).unwrap();

        assert_eq!(version, deserialized);
    }

    #[test]
    fn test_version_conflict_serialization() {
        let conflict = VersionConflict::new(
            ScimVersion::from_hash("hash-v1"),
            ScimVersion::from_hash("hash-v2"),
            "Serialization test conflict",
        );

        // Test JSON serialization
        let json = serde_json::to_string(&conflict).unwrap();
        let deserialized: VersionConflict = serde_json::from_str(&json).unwrap();

        assert_eq!(conflict, deserialized);
    }
}
