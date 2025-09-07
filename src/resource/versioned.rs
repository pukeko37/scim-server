//! Versioned resource types for SCIM resource versioning.
//!
//! This module provides the `VersionedResource` type for handling SCIM resources
//! with version control. It enables conditional operations with ETag-based
//! concurrency control for preventing lost updates.
//!
//! # Core Type
//!
//! * [`VersionedResource`] - Resource wrapper that includes automatic version computation
//!
//! # Usage
//!
//! ```rust
//! use scim_server::resource::{
//!     versioned::VersionedResource,
//!     Resource,
//! };
//! use scim_server::resource::version::HttpVersion;
//! use serde_json::json;
//!
//! let resource = Resource::from_json("User".to_string(), json!({
//!     "id": "123",
//!     "userName": "john.doe",
//!     "active": true
//! })).unwrap();
//!
//! let versioned = VersionedResource::new(resource);
//! println!(
//!     "Resource version: {}",
//!     HttpVersion::from(versioned.version().clone())
//! );
//! ```

use super::{
    resource::Resource,
    version::{RawVersion, ScimVersion},
};
use serde::{Deserialize, Serialize};

/// A resource with its associated version information.
///
/// This wrapper combines a SCIM resource with its version, enabling
/// conditional operations that can detect concurrent modifications.
/// The version is automatically computed from the resource content.
///
/// # Examples
///
/// ```rust
/// use scim_server::resource::{
///     versioned::VersionedResource,
///     Resource,
/// };
/// use scim_server::resource::version::HttpVersion;
/// use serde_json::json;
///
/// let resource = Resource::from_json("User".to_string(), json!({
///     "id": "123",
///     "userName": "john.doe",
///     "active": true
/// })).unwrap();
///
/// let versioned = VersionedResource::new(resource);
/// println!(
///     "Resource version: {}",
///     HttpVersion::from(versioned.version().clone())
/// );
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionedResource {
    /// The SCIM resource data
    resource: Resource,

    /// The version computed from the resource content
    version: RawVersion,
}

impl VersionedResource {
    /// Create a new versioned resource.
    ///
    /// The version is automatically computed from the resource's JSON representation,
    /// ensuring consistency across all provider implementations.
    ///
    /// # Arguments
    /// * `resource` - The SCIM resource
    ///
    /// # Examples
    /// ```rust
    /// use scim_server::resource::{
    ///     versioned::VersionedResource,
    ///     Resource,
    /// };
    /// use serde_json::json;
    ///
    /// let resource = Resource::from_json("User".to_string(), json!({
    ///     "id": "123",
    ///     "userName": "john.doe"
    /// })).unwrap();
    ///
    /// let versioned = VersionedResource::new(resource);
    /// ```
    pub fn new(resource: Resource) -> Self {
        let version = Self::get_or_compute_version(&resource);
        Self { resource, version }
    }

    /// Create a versioned resource with a specific version.
    ///
    /// This is useful when migrating from existing systems or when the version
    /// needs to be preserved from external sources.
    ///
    /// # Arguments
    /// * `resource` - The SCIM resource
    /// * `version` - The specific version to use
    ///
    /// # Examples
    /// ```rust
    /// use scim_server::resource::{
    ///     versioned::VersionedResource,
    ///     Resource,
    ///     version::RawVersion,
    /// };
    /// use serde_json::json;
    ///
    /// let resource = Resource::from_json("User".to_string(), json!({"id": "123"})).unwrap();
    /// let version = RawVersion::from_hash("custom-version");
    /// let versioned = VersionedResource::with_version(resource, version);
    /// ```
    pub fn with_version(resource: Resource, version: RawVersion) -> Self {
        Self { resource, version }
    }

    /// Get the resource data.
    pub fn resource(&self) -> &Resource {
        &self.resource
    }

    /// Get the resource version.
    pub fn version(&self) -> &RawVersion {
        &self.version
    }

    /// Convert into the underlying resource, discarding version information.
    pub fn into_resource(self) -> Resource {
        self.resource
    }

    /// Get the unique identifier of this resource.
    ///
    /// Delegates to the inner resource's `get_id()` method.
    pub fn get_id(&self) -> Option<&str> {
        self.resource.get_id()
    }

    /// Get the userName field for User resources.
    ///
    /// Delegates to the inner resource's `get_username()` method.
    pub fn get_username(&self) -> Option<&str> {
        self.resource.get_username()
    }

    /// Get the external id if present.
    ///
    /// Delegates to the inner resource's `get_external_id()` method.
    pub fn get_external_id(&self) -> Option<&str> {
        self.resource.get_external_id()
    }

    /// Get the meta attributes if present.
    ///
    /// Delegates to the inner resource's `get_meta()` method.
    pub fn get_meta(&self) -> Option<&crate::resource::value_objects::Meta> {
        self.resource.get_meta()
    }

    /// Get an attribute value from the resource.
    ///
    /// Delegates to the inner resource's `get()` method.
    pub fn get(&self, key: &str) -> Option<&serde_json::Value> {
        self.resource.get(key)
    }

    /// Get an attribute value from the resource.
    ///
    /// Delegates to the inner resource's `get_attribute()` method.
    /// This is an alias for `get()` for consistency with Resource API.
    pub fn get_attribute(&self, attribute_name: &str) -> Option<&serde_json::Value> {
        self.resource.get_attribute(attribute_name)
    }

    /// Update the resource content and recompute the version.
    ///
    /// This ensures the version always reflects the current resource state.
    ///
    /// # Arguments
    /// * `new_resource` - The updated resource data
    ///
    /// # Examples
    /// ```rust
    /// use scim_server::resource::{
    ///     versioned::VersionedResource,
    ///     Resource,
    /// };
    /// use serde_json::json;
    ///
    /// let resource = Resource::from_json("User".to_string(), json!({"id": "123", "active": true})).unwrap();
    /// let mut versioned = VersionedResource::new(resource);
    ///
    /// let updated = Resource::from_json("User".to_string(), json!({"id": "123", "active": false})).unwrap();
    /// let old_version = versioned.version().clone();
    /// versioned.update_resource(updated);
    ///
    /// assert!(versioned.version() != &old_version);
    /// ```
    pub fn update_resource(&mut self, new_resource: Resource) {
        self.version = Self::compute_version(&new_resource);
        self.resource = new_resource;
    }

    /// Check if this resource's version matches the expected version.
    ///
    /// # Arguments
    /// * `expected` - The expected version to check against
    ///
    /// # Returns
    /// `true` if versions match, `false` otherwise
    pub fn version_matches<F>(&self, expected: &ScimVersion<F>) -> bool {
        self.version == *expected
    }

    /// Refresh the version based on current resource content.
    ///
    /// This is useful if the resource was modified externally and the version
    /// needs to be synchronized.
    pub fn refresh_version(&mut self) {
        self.version = Self::compute_version(&self.resource);
    }

    /// Get version from resource meta or compute from content if not available.
    ///
    /// This first tries to extract the version from the resource's meta field.
    /// Meta now stores versions in raw format internally.
    /// If no version exists in meta, it computes one from the resource content.
    fn get_or_compute_version(resource: &Resource) -> RawVersion {
        // Try to get version from meta first (now stored in raw format)
        if let Some(meta) = resource.get_meta() {
            if let Some(meta_version) = meta.version() {
                // Meta now stores raw versions, so parse directly
                if let Ok(version) = meta_version.parse::<RawVersion>() {
                    return version;
                }
            }
        }

        // Fallback: compute version from content
        Self::compute_version(resource)
    }

    /// Compute version from resource content.
    ///
    /// This uses the resource's JSON representation to generate a consistent
    /// hash-based version that reflects all resource data.
    fn compute_version(resource: &Resource) -> RawVersion {
        let json_bytes = resource.to_json().unwrap().to_string().into_bytes();
        RawVersion::from_content(&json_bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_versioned_resource_creation() {
        let resource = Resource::from_json(
            "User".to_string(),
            json!({
                "id": "123",
                "userName": "john.doe",
                "active": true
            }),
        )
        .unwrap();

        let versioned = VersionedResource::new(resource.clone());
        assert_eq!(versioned.get_id(), resource.get_id());
        assert!(!versioned.version().as_str().is_empty());
    }

    #[test]
    fn test_versioned_resource_version_changes() {
        let resource1 = Resource::from_json(
            "User".to_string(),
            json!({
                "id": "123",
                "userName": "john.doe",
                "active": true
            }),
        )
        .unwrap();

        let resource2 = Resource::from_json(
            "User".to_string(),
            json!({
                "id": "123",
                "userName": "john.doe",
                "active": false // Changed field
            }),
        )
        .unwrap();

        let versioned1 = VersionedResource::new(resource1);
        let versioned2 = VersionedResource::new(resource2);

        // Different content should produce different versions
        assert!(versioned1.version() != versioned2.version());
    }

    #[test]
    fn test_versioned_resource_update() {
        let initial_resource = Resource::from_json(
            "User".to_string(),
            json!({
                "id": "123",
                "userName": "john.doe",
                "active": true
            }),
        )
        .unwrap();

        let mut versioned = VersionedResource::new(initial_resource);
        let old_version = versioned.version().clone();

        let updated_resource = Resource::from_json(
            "User".to_string(),
            json!({
                "id": "123",
                "userName": "john.doe",
                "active": false
            }),
        )
        .unwrap();

        versioned.update_resource(updated_resource);

        // Version should change after update
        assert!(versioned.version() != &old_version);
        assert_eq!(versioned.get_id(), Some("123"));
    }

    #[test]
    fn test_versioned_resource_version_matching() {
        let resource = Resource::from_json(
            "User".to_string(),
            json!({
                "id": "123",
                "userName": "test"
            }),
        )
        .unwrap();

        let versioned = VersionedResource::new(resource);
        let version_copy = versioned.version().clone();
        let different_version = RawVersion::from_hash("different");

        assert!(versioned.version_matches(&version_copy));
        assert!(!versioned.version_matches(&different_version));
    }

    #[test]
    fn test_versioned_resource_with_version() {
        let resource = Resource::from_json("User".to_string(), json!({"id": "123"})).unwrap();
        let custom_version = RawVersion::from_hash("custom-version-123");

        let versioned = VersionedResource::with_version(resource.clone(), custom_version.clone());

        assert_eq!(versioned.get_id(), resource.get_id());
        assert_eq!(versioned.version(), &custom_version);
    }

    #[test]
    fn test_versioned_resource_refresh_version() {
        let resource =
            Resource::from_json("User".to_string(), json!({"id": "123", "data": "test"})).unwrap();
        let custom_version = RawVersion::from_hash("custom");

        let mut versioned = VersionedResource::with_version(resource, custom_version.clone());
        assert_eq!(versioned.version(), &custom_version);

        versioned.refresh_version();
        // After refresh, version should be computed from content, not the custom version
        assert!(versioned.version() != &custom_version);
    }

    #[test]
    fn test_versioned_resource_serialization() {
        let resource = Resource::from_json(
            "User".to_string(),
            json!({
                "id": "123",
                "userName": "test.user"
            }),
        )
        .unwrap();

        let versioned = VersionedResource::new(resource);

        // Test JSON serialization round-trip
        let json = serde_json::to_string(&versioned).unwrap();
        let deserialized: VersionedResource = serde_json::from_str(&json).unwrap();

        assert_eq!(versioned.get_id(), deserialized.get_id());
        assert!(versioned.version() == deserialized.version());
    }
}
