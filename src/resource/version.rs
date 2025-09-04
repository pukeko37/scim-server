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
//! # Type-Safe Format Management
//!
//! This module uses phantom types to distinguish between HTTP ETag format and raw
//! internal format at compile time, preventing format confusion:
//!
//! * [`HttpVersion`] - HTTP ETag format ("W/\"abc123\"")
//! * [`RawVersion`] - Internal raw format ("abc123")
//! * [`ConditionalResult`] - Result type for conditional operations
//! * [`VersionConflict`] - Error details for version mismatches
//!
//! # Basic Usage
//!
//! ```rust
//! use scim_server::resource::version::{RawVersion, HttpVersion};
//!
//! // Create version from hash string (for provider-specific versioning)
//! let raw_version = RawVersion::from_hash("db-sequence-123");
//!
//! // Create version from content hash (automatic versioning)
//! let resource_json = br#"{"id":"123","userName":"john.doe","active":true}"#;
//! let content_version = RawVersion::from_content(resource_json);
//!
//! // Parse from HTTP weak ETag header (client-provided versions)
//! let etag_version: HttpVersion = "W/\"abc123def\"".parse().unwrap();
//!
//! // Convert to HTTP weak ETag header (for responses)
//! let etag_header = HttpVersion::from(raw_version).to_string(); // Returns: "W/\"abc123def\""
//!
//! // Check version equality (works across formats)
//! let matches = raw_version == etag_version;
//! ```
//!
//! # Format Conversions
//!
//! ```rust
//! use scim_server::resource::version::{RawVersion, HttpVersion};
//!
//! // Raw to HTTP format
//! let raw_version = RawVersion::from_hash("abc123");
//! let http_version = HttpVersion::from(raw_version);
//!
//! // HTTP to Raw format
//! let http_version: HttpVersion = "W/\"xyz789\"".parse().unwrap();
//! let raw_version = RawVersion::from(http_version);
//!
//! // Direct string parsing
//! let raw_from_str: RawVersion = "abc123".parse().unwrap();
//! let http_from_str: HttpVersion = "\"xyz789\"".parse().unwrap();
//! ```
//!
//! # Conditional Operations
//!
//! ```rust,no_run
//! use scim_server::resource::version::{ConditionalResult, RawVersion, HttpVersion};
//! use scim_server::resource::{ResourceProvider, RequestContext};
//! use serde_json::json;
//!
//! # async fn example<P: ResourceProvider + Sync>(provider: &P) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
//! let context = RequestContext::with_generated_id();
//! let expected_version = RawVersion::from_hash("current-version");
//! let update_data = json!({"userName": "updated.name", "active": false});
//!
//! // Conditional update with type-safe version handling
//! match provider.conditional_update("User", "123", update_data, &expected_version, &context).await {
//!     Ok(ConditionalResult::Success(updated_resource)) => {
//!         println!("Update succeeded: {}", updated_resource.resource().get_id().unwrap_or("unknown"));
//!     }
//!     Ok(ConditionalResult::VersionMismatch(conflict)) => {
//!         println!("Version conflict: expected {}, found {}", conflict.expected, conflict.current);
//!     }
//!     Ok(ConditionalResult::NotFound) => {
//!         println!("Resource not found");
//!     }
//!     Err(e) => {
//!         println!("Operation failed: {}", e);
//!     }
//! }
//! # Ok(())
//! # }
//! ```

use base64::{Engine, engine::general_purpose::STANDARD as BASE64};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use sha2::{Digest, Sha256};
use std::{fmt, marker::PhantomData, str::FromStr};
use thiserror::Error;

// Phantom type markers for format distinction
#[derive(Debug, Clone, Copy)]
pub struct Http;

#[derive(Debug, Clone, Copy)]
pub struct Raw;

/// Opaque version identifier for SCIM resources with compile-time format safety.
///
/// This type uses phantom types to distinguish between HTTP ETag format and raw
/// internal format at compile time, preventing format confusion and runtime errors.
/// The internal representation remains opaque to prevent direct manipulation.
///
/// Versions can be created from:
/// - Provider-specific identifiers (database sequence numbers, timestamps, etc.)
/// - Content hashes (for stateless version generation)
/// - String parsing with automatic format detection
///
/// # Type Safety
///
/// The phantom type parameter prevents mixing formats accidentally:
/// ```compile_fail
/// use scim_server::resource::version::{RawVersion, HttpVersion};
///
/// let raw_version = RawVersion::from_hash("123");
/// let http_version: HttpVersion = "W/\"456\"".parse().unwrap();
///
/// // This won't compile - cannot pass HttpVersion where RawVersion expected
/// some_function_expecting_raw(http_version);
/// ```
///
/// # Examples
///
/// ```rust
/// use scim_server::resource::version::{RawVersion, HttpVersion};
///
/// // From hash string (always produces Raw format)
/// let raw_version = RawVersion::from_hash("12345");
///
/// // From content hash (always produces Raw format)
/// let content = br#"{"id":"123","name":"John Doe"}"#;
/// let hash_version = RawVersion::from_content(content);
///
/// // Parse from strings with format detection
/// let raw_parsed: RawVersion = "abc123def".parse().unwrap();
/// let http_parsed: HttpVersion = "\"abc123def\"".parse().unwrap();
/// ```
#[derive(Debug, Clone, Eq, Hash)]
pub struct ScimVersion<Format> {
    /// Opaque version identifier
    opaque: String,
    /// Phantom type marker for compile-time format distinction
    #[allow(dead_code)]
    _format: PhantomData<Format>,
}

/// Type alias for HTTP ETag format versions ("W/\"abc123\"")
pub type HttpVersion = ScimVersion<Http>;

/// Type alias for raw internal format versions ("abc123")
pub type RawVersion = ScimVersion<Raw>;

// Core constructors (always produce Raw format as the canonical form)
impl<Format> ScimVersion<Format> {
    /// Create a version from resource content.
    ///
    /// This generates a deterministic hash-based version from the resource content,
    /// ensuring universal compatibility across all provider implementations.
    /// The version is based on the full resource content including all fields.
    ///
    /// Always produces a [`RawVersion`] as content hashing creates canonical versions.
    ///
    /// # Arguments
    /// * `content` - The complete resource content as bytes
    ///
    /// # Examples
    /// ```rust
    /// use scim_server::resource::version::RawVersion;
    ///
    /// let resource_json = br#"{"id":"123","userName":"john.doe"}"#;
    /// let version = RawVersion::from_content(resource_json);
    /// ```
    pub fn from_content(content: &[u8]) -> RawVersion {
        let mut hasher = Sha256::new();
        hasher.update(content);
        let hash = hasher.finalize();
        let encoded = BASE64.encode(&hash[..8]); // Use first 8 bytes for shorter ETags

        ScimVersion {
            opaque: encoded,
            _format: PhantomData,
        }
    }

    /// Create a version from a pre-computed hash string.
    ///
    /// This is useful for provider-specific versioning schemes such as database
    /// sequence numbers, timestamps, or UUIDs. The provider can use any string
    /// as a version identifier.
    ///
    /// Always produces a [`RawVersion`] as the canonical internal format.
    ///
    /// # Arguments
    /// * `hash_string` - Provider-specific version identifier
    ///
    /// # Examples
    /// ```rust
    /// use scim_server::resource::version::RawVersion;
    ///
    /// // Database sequence number
    /// let db_version = RawVersion::from_hash("seq_12345");
    ///
    /// // Timestamp-based version
    /// let time_version = RawVersion::from_hash("1703123456789");
    ///
    /// // UUID-based version
    /// let uuid_version = RawVersion::from_hash("550e8400-e29b-41d4-a716-446655440000");
    /// ```
    pub fn from_hash(hash_string: impl AsRef<str>) -> RawVersion {
        ScimVersion {
            opaque: hash_string.as_ref().to_string(),
            _format: PhantomData,
        }
    }

    /// Get the opaque version string.
    ///
    /// This is primarily for internal use and debugging. The opaque string
    /// should not be relied upon for any business logic outside of equality comparisons.
    pub fn as_str(&self) -> &str {
        &self.opaque
    }
}

// Display implementation for Raw format (simple string output)
impl fmt::Display for ScimVersion<Raw> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.opaque)
    }
}

// Display implementation for HTTP format (weak ETag format)
impl fmt::Display for ScimVersion<Http> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "W/\"{}\"", self.opaque)
    }
}

// FromStr implementation for Raw format (direct string parsing)
impl FromStr for ScimVersion<Raw> {
    type Err = VersionError;

    fn from_str(version_str: &str) -> Result<Self, Self::Err> {
        let trimmed = version_str.trim();

        if trimmed.is_empty() {
            return Err(VersionError::ParseError(
                "Version string cannot be empty".to_string(),
            ));
        }

        Ok(ScimVersion {
            opaque: trimmed.to_string(),
            _format: PhantomData,
        })
    }
}

// FromStr implementation for HTTP format (ETag parsing)
impl FromStr for ScimVersion<Http> {
    type Err = VersionError;

    fn from_str(etag_header: &str) -> Result<Self, Self::Err> {
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

        Ok(ScimVersion {
            opaque,
            _format: PhantomData,
        })
    }
}

// Bidirectional conversions through owned values
impl From<ScimVersion<Raw>> for ScimVersion<Http> {
    fn from(raw: ScimVersion<Raw>) -> Self {
        ScimVersion {
            opaque: raw.opaque,
            _format: PhantomData,
        }
    }
}

impl From<ScimVersion<Http>> for ScimVersion<Raw> {
    fn from(http: ScimVersion<Http>) -> Self {
        ScimVersion {
            opaque: http.opaque,
            _format: PhantomData,
        }
    }
}

// Cross-format comparison (versions are equal if opaque strings match)
impl<F1, F2> PartialEq<ScimVersion<F2>> for ScimVersion<F1> {
    fn eq(&self, other: &ScimVersion<F2>) -> bool {
        self.opaque == other.opaque
    }
}

// Serde implementations that preserve the opaque string regardless of format
impl<Format> Serialize for ScimVersion<Format> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.opaque.serialize(serializer)
    }
}

impl<'de, Format> Deserialize<'de> for ScimVersion<Format> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let opaque = String::deserialize(deserializer)?;
        Ok(ScimVersion {
            opaque,
            _format: PhantomData,
        })
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
/// use scim_server::resource::version::{ConditionalResult, RawVersion, VersionConflict};
/// use serde_json::json;
///
/// // Successful operation
/// let success = ConditionalResult::Success(json!({"id": "123"}));
///
/// // Version mismatch
/// let expected = RawVersion::from_hash("1");
/// let current = RawVersion::from_hash("2");
/// let conflict: ConditionalResult<serde_json::Value> = ConditionalResult::VersionMismatch(VersionConflict {
///     expected: expected.into(),
///     current: current.into(),
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
/// error message. Uses [`RawVersion`] internally for consistent storage
/// and comparison.
///
/// # Examples
///
/// ```rust
/// use scim_server::resource::version::{VersionConflict, RawVersion};
///
/// let expected = RawVersion::from_hash("1");
/// let current = RawVersion::from_hash("2");
/// let conflict = VersionConflict {
///     expected,
///     current,
///     message: "Resource was modified by another client".to_string(),
/// };
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VersionConflict {
    /// The version that was expected by the client (raw format)
    pub expected: RawVersion,

    /// The current version of the resource on the server (raw format)
    pub current: RawVersion,

    /// Human-readable error message describing the conflict
    pub message: String,
}

impl VersionConflict {
    /// Create a new version conflict.
    ///
    /// Accepts versions in any format and converts to raw format for internal storage.
    ///
    /// # Arguments
    /// * `expected` - The version expected by the client
    /// * `current` - The current version on the server
    /// * `message` - Human-readable error message
    pub fn new<E, C>(expected: E, current: C, message: impl Into<String>) -> Self
    where
        E: Into<RawVersion>,
        C: Into<RawVersion>,
    {
        Self {
            expected: expected.into(),
            current: current.into(),
            message: message.into(),
        }
    }

    /// Create a standard version conflict message.
    ///
    /// Accepts versions in any format and converts to raw format for internal storage.
    ///
    /// # Arguments
    /// * `expected` - The version expected by the client
    /// * `current` - The current version on the server
    pub fn standard_message<E, C>(expected: E, current: C) -> Self
    where
        E: Into<RawVersion>,
        C: Into<RawVersion>,
    {
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
        let content1 = b"test content";
        let content2 = b"test content";
        let content3 = b"different content";

        let version1 = RawVersion::from_content(content1);
        let version2 = RawVersion::from_content(content2);
        let version3 = RawVersion::from_content(content3);

        // Same content should produce same version
        assert_eq!(version1, version2);
        // Different content should produce different version
        assert_ne!(version1, version3);
    }

    #[test]
    fn test_version_from_hash() {
        let version1 = RawVersion::from_hash("abc123def");
        let version2 = RawVersion::from_hash("abc123def");
        let version3 = RawVersion::from_hash("xyz789");

        assert_eq!(version1, version2);
        assert_ne!(version1, version3);
        assert_eq!(version1.as_str(), "abc123def");
    }

    #[test]
    fn test_http_version_parse() {
        // Test weak ETag parsing
        let version1: HttpVersion = "W/\"abc123\"".parse().unwrap();
        assert_eq!(version1.as_str(), "abc123");

        // Test strong ETag parsing
        let version2: HttpVersion = "\"xyz789\"".parse().unwrap();
        assert_eq!(version2.as_str(), "xyz789");

        // Test invalid formats
        assert!("invalid".parse::<HttpVersion>().is_err());
        assert!("\"\"".parse::<HttpVersion>().is_err());
        assert!("W/invalid".parse::<HttpVersion>().is_err());
    }

    #[test]
    fn test_raw_version_parse() {
        let version: RawVersion = "abc123def".parse().unwrap();
        assert_eq!(version.as_str(), "abc123def");

        // Test empty string fails
        assert!("".parse::<RawVersion>().is_err());
        assert!("   ".parse::<RawVersion>().is_err());
    }

    #[test]
    fn test_format_display() {
        let raw_version = RawVersion::from_hash("abc123");
        let http_version = HttpVersion::from(raw_version.clone());

        assert_eq!(raw_version.to_string(), "abc123");
        assert_eq!(http_version.to_string(), "W/\"abc123\"");

        // Cross-format equality is guaranteed by type system
        assert_eq!(raw_version, http_version);
    }

    #[test]
    fn test_conditional_result() {
        let success: ConditionalResult<i32> = ConditionalResult::Success(42);
        let not_found: ConditionalResult<i32> = ConditionalResult::NotFound;
        let conflict: ConditionalResult<i32> =
            ConditionalResult::VersionMismatch(VersionConflict::new(
                RawVersion::from_hash("1"),
                RawVersion::from_hash("2"),
                "test conflict",
            ));

        assert!(success.is_success());
        assert!(!success.is_version_mismatch());
        assert!(!success.is_not_found());

        assert!(!not_found.is_success());
        assert!(!not_found.is_version_mismatch());
        assert!(not_found.is_not_found());

        assert!(!conflict.is_success());
        assert!(conflict.is_version_mismatch());
        assert!(!conflict.is_not_found());
    }

    #[test]
    fn test_conditional_result_map() {
        let success: ConditionalResult<i32> = ConditionalResult::Success(21);
        let doubled = success.map(|x| x * 2);
        assert_eq!(doubled.into_success(), Some(42));
    }

    #[test]
    fn test_version_conflict() {
        let expected = RawVersion::from_hash("1");
        let current = RawVersion::from_hash("2");
        let conflict = VersionConflict::new(expected.clone(), current.clone(), "test message");

        assert_eq!(conflict.expected, expected);
        assert_eq!(conflict.current, current);
        assert_eq!(conflict.message, "test message");
    }

    #[test]
    fn test_version_conflict_display() {
        let conflict = VersionConflict::standard_message(
            RawVersion::from_hash("old"),
            RawVersion::from_hash("new"),
        );
        let display_str = format!("{}", conflict);
        assert!(display_str.contains("expected 'old'"));
        assert!(display_str.contains("found 'new'"));
        assert!(display_str.contains("Resource was modified"));
    }

    #[test]
    fn test_version_serialization() {
        let version = RawVersion::from_hash("test123");
        let json = serde_json::to_string(&version).unwrap();
        assert_eq!(json, "\"test123\"");

        let deserialized: RawVersion = serde_json::from_str(&json).unwrap();
        assert_eq!(version, deserialized);
    }

    #[test]
    fn test_version_conflict_serialization() {
        let conflict = VersionConflict::new(
            RawVersion::from_hash("1"),
            RawVersion::from_hash("2"),
            "test",
        );

        let json = serde_json::to_string(&conflict).unwrap();
        let deserialized: VersionConflict = serde_json::from_str(&json).unwrap();
        assert_eq!(conflict, deserialized);
    }
}
