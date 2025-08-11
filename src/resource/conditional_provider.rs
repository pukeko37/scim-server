//! Versioned resource types for SCIM resource versioning.
//!
//! This module provides types for handling versioned SCIM resources that support
//! conditional operations with version control. As of Phase 3, conditional operations
//! are mandatory and built into the core ResourceProvider trait, ensuring all providers
//! support ETag-based concurrency control.
//!
//! # Mandatory Conditional Operations Architecture
//!
//! The SCIM server library now requires all ResourceProvider implementations to support
//! conditional operations. This design decision provides:
//!
//! - **Universal Concurrency Control**: All resources automatically support ETag versioning
//! - **Simplified Architecture**: Single code path with consistent behavior
//! - **Type Safety**: Compile-time guarantees for version-aware operations
//! - **Production Readiness**: Built-in protection against lost updates
//!
//! # Core Types
//!
//! * [`VersionedResource`] - Resource wrapper that includes automatic version computation
//!
//! # Usage with Mandatory Conditional Operations
//!
//! ```rust,no_run
//! use scim_server::resource::{
//!     provider::ResourceProvider,
//!     conditional_provider::VersionedResource,
//!     version::{ScimVersion, ConditionalResult},
//!     core::{Resource, RequestContext},
//! };
//! use serde_json::Value;
//! use std::collections::HashMap;
//! use std::sync::Arc;
//! use tokio::sync::RwLock;
//!
//! #[derive(Debug)]
//! struct MyError(String);
//! impl std::fmt::Display for MyError {
//!     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
//!         write!(f, "{}", self.0)
//!     }
//! }
//! impl std::error::Error for MyError {}
//!
//! #[derive(Clone)]
//! struct MyProvider {
//!     data: Arc<RwLock<HashMap<String, VersionedResource>>>,
//! }
//!
//! impl ResourceProvider for MyProvider {
//!     type Error = MyError;
//!
//!     // All providers must implement these core CRUD methods
//!     async fn create_resource(&self, resource_type: &str, data: Value, context: &RequestContext) -> Result<Resource, Self::Error> {
//!         let resource = Resource::from_json(resource_type.to_string(), data)
//!             .map_err(|e| MyError(e.to_string()))?;
//!         let mut store = self.data.write().await;
//!         let id = resource.get_id().unwrap_or("generated-id").to_string();
//!         let versioned = VersionedResource::new(resource.clone());
//!         store.insert(id, versioned);
//!         Ok(resource)
//!     }
//!
//!     async fn get_resource(&self, _resource_type: &str, id: &str, _context: &RequestContext) -> Result<Option<Resource>, Self::Error> {
//!         let store = self.data.read().await;
//!         Ok(store.get(id).map(|v| v.resource().clone()))
//!     }
//!
//!     // ... implement other required methods ...
//!     # async fn update_resource(&self, _resource_type: &str, _id: &str, _data: Value, _context: &RequestContext) -> Result<Resource, Self::Error> {
//!     #     todo!("Implement your update logic here")
//!     # }
//!     # async fn delete_resource(&self, _resource_type: &str, _id: &str, _context: &RequestContext) -> Result<(), Self::Error> {
//!     #     todo!("Implement your delete logic here")
//!     # }
//!     # async fn list_resources(&self, _resource_type: &str, _query: Option<&scim_server::resource::core::ListQuery>, _context: &RequestContext) -> Result<Vec<Resource>, Self::Error> {
//!     #     todo!("Implement your list logic here")
//!     # }
//!     # async fn find_resource_by_attribute(&self, _resource_type: &str, _attribute: &str, _value: &Value, _context: &RequestContext) -> Result<Option<Resource>, Self::Error> {
//!     #     todo!("Implement your find logic here")
//!     # }
//!     # async fn resource_exists(&self, _resource_type: &str, _id: &str, _context: &RequestContext) -> Result<bool, Self::Error> {
//!     #     todo!("Implement your exists logic here")
//!     # }
//!
//!     // Conditional operations are MANDATORY - provided by default with automatic implementation
//!     // Override these methods for optimized conditional operations at the storage layer:
//!
//!     // async fn conditional_update(&self, resource_type: &str, id: &str, data: Value,
//!     //                           expected_version: &ScimVersion, context: &RequestContext)
//!     //                           -> Result<ConditionalResult<VersionedResource>, Self::Error> {
//!     //     // Your database-level conditional update with version checking
//!     // }
//!     //
//!     // async fn conditional_delete(&self, resource_type: &str, id: &str,
//!     //                           expected_version: &ScimVersion, context: &RequestContext)
//!     //                           -> Result<ConditionalResult<()>, Self::Error> {
//!     //     // Your database-level conditional delete with version checking
//!     // }
//! }
//! ```
//!
//! # Architectural Benefits
//!
//! Making conditional operations mandatory provides several advantages:
//!
//! ## Simplified Codebase
//! - Single code path for all operations
//! - No optional/conditional provider detection
//! - Consistent behavior across all implementations
//!
//! ## Enhanced Type Safety
//! - Compile-time guarantees for version support
//! - No runtime checks for capability detection
//! - Clear API contracts for all providers
//!
//! ## Production Readiness
//! - Built-in concurrency control for all resources
//! - Automatic protection against lost updates
//! - Enterprise-grade data integrity guarantees
//!
//! ## Developer Experience
//! - Consistent APIs across all providers
//! - Clear documentation and examples
//! - Better IDE support and tooling

use super::{core::Resource, version::ScimVersion};
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
///     conditional_provider::VersionedResource,
///     core::Resource,
/// };
/// use serde_json::json;
///
/// let resource = Resource::from_json("User".to_string(), json!({
///     "id": "123",
///     "userName": "john.doe",
///     "active": true
/// })).unwrap();
///
/// let versioned = VersionedResource::new(resource);
/// println!("Resource version: {}", versioned.version().to_http_header());
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionedResource {
    /// The SCIM resource data
    resource: Resource,

    /// The version computed from the resource content
    version: ScimVersion,
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
    ///     conditional_provider::VersionedResource,
    ///     core::Resource,
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
        let version = Self::compute_version(&resource);
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
    ///     conditional_provider::VersionedResource,
    ///     core::Resource,
    ///     version::ScimVersion,
    /// };
    /// use serde_json::json;
    ///
    /// let resource = Resource::from_json("User".to_string(), json!({"id": "123"})).unwrap();
    /// let version = ScimVersion::from_hash("custom-version");
    /// let versioned = VersionedResource::with_version(resource, version);
    /// ```
    pub fn with_version(resource: Resource, version: ScimVersion) -> Self {
        Self { resource, version }
    }

    /// Get the resource data.
    pub fn resource(&self) -> &Resource {
        &self.resource
    }

    /// Get the resource version.
    pub fn version(&self) -> &ScimVersion {
        &self.version
    }

    /// Convert into the underlying resource, discarding version information.
    pub fn into_resource(self) -> Resource {
        self.resource
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
    ///     conditional_provider::VersionedResource,
    ///     core::Resource,
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
    /// assert!(!versioned.version().matches(&old_version));
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
    pub fn version_matches(&self, expected: &ScimVersion) -> bool {
        self.version.matches(expected)
    }

    /// Refresh the version based on current resource content.
    ///
    /// This is useful if the resource was modified externally and the version
    /// needs to be synchronized.
    pub fn refresh_version(&mut self) {
        self.version = Self::compute_version(&self.resource);
    }

    /// Compute version from resource content.
    ///
    /// This uses the resource's JSON representation to generate a consistent
    /// hash-based version that reflects all resource data.
    fn compute_version(resource: &Resource) -> ScimVersion {
        let json_bytes = resource.to_json().unwrap().to_string().into_bytes();
        ScimVersion::from_content(&json_bytes)
    }
}

/// Historical note: Extension trait for conditional operations (Phase 1-2).
///
/// This trait was used during the development phases when conditional operations
/// were optional. As of Phase 3, all conditional operations are mandatory and
/// built into the core ResourceProvider trait.
///
/// # Migration to Mandatory Architecture
///
/// The library has evolved from optional conditional operations to mandatory ones:
///
/// - **Phase 1-2**: Conditional operations were optional via this extension trait
/// - **Phase 3**: Conditional operations moved to core ResourceProvider trait
/// - **Current**: All providers automatically support conditional operations
///
/// This change ensures:
/// - Universal concurrency control for all SCIM resources
/// - Simplified integration with automatic ETag support
/// - Consistent behavior across different provider implementations
/// - Production-ready concurrency control out of the box
///
/// All new code should use the conditional methods directly on ResourceProvider
/// rather than this historical extension trait.

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
        assert_eq!(versioned.resource().get_id(), resource.get_id());
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
        assert!(!versioned1.version().matches(versioned2.version()));
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
        assert!(!versioned.version().matches(&old_version));
        assert_eq!(versioned.resource().get_id(), Some("123"));
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
        let different_version = ScimVersion::from_hash("different");

        assert!(versioned.version_matches(&version_copy));
        assert!(!versioned.version_matches(&different_version));
    }

    #[test]
    fn test_versioned_resource_with_version() {
        let resource = Resource::from_json("User".to_string(), json!({"id": "123"})).unwrap();
        let custom_version = ScimVersion::from_hash("custom-version-123");

        let versioned = VersionedResource::with_version(resource.clone(), custom_version.clone());

        assert_eq!(versioned.resource().get_id(), resource.get_id());
        assert_eq!(versioned.version(), &custom_version);
    }

    #[test]
    fn test_versioned_resource_refresh_version() {
        let resource =
            Resource::from_json("User".to_string(), json!({"id": "123", "data": "test"})).unwrap();
        let custom_version = ScimVersion::from_hash("custom");

        let mut versioned = VersionedResource::with_version(resource, custom_version.clone());
        assert_eq!(versioned.version(), &custom_version);

        versioned.refresh_version();
        // After refresh, version should be computed from content, not the custom version
        assert!(!versioned.version().matches(&custom_version));
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

        assert_eq!(
            versioned.resource().get_id(),
            deserialized.resource().get_id()
        );
        assert!(versioned.version().matches(deserialized.version()));
    }
}
