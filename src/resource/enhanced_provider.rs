//! Enhanced resource provider trait with built-in version support.
//!
//! This module replaces the existing ResourceProvider trait with a version-aware
//! interface that supports conditional operations by default. All operations
//! now include version handling for optimistic concurrency control.
//!
//! # Design Philosophy
//!
//! * All operations are version-aware by design
//! * No fallback mechanisms or compatibility layers
//! * Clean, simple API with built-in concurrency safety
//! * Version checking is mandatory, not optional
//!
//! # Usage
//!
//! ```rust
//! use scim_server::resource::{
//!     enhanced_provider::{EnhancedResourceProvider, VersionedResource},
//!     version::{ScimVersion, ConditionalResult},
//!     core::RequestContext,
//! };
//! use serde_json::json;
//!
//! # async fn example(provider: impl EnhancedResourceProvider, context: RequestContext) -> Result<(), Box<dyn std::error::Error>> {
//! // Create always returns a versioned resource
//! let user = provider.create_resource(
//!     "User",
//!     json!({"userName": "john.doe", "active": true}),
//!     &context
//! ).await?;
//!
//! // Get always returns version information
//! let retrieved = provider.get_resource("User", "123", &context).await?;
//! if let Some(versioned_user) = retrieved {
//!     println!("Current version: {}", versioned_user.version());
//! }
//!
//! // Update requires expected version
//! match provider.update_resource(
//!     "User",
//!     "123",
//!     json!({"userName": "john.doe", "active": false}),
//!     &user.version(), // Must provide expected version
//!     &context,
//! ).await? {
//!     ConditionalResult::Success(updated) => {
//!         println!("Update successful, new version: {}", updated.version());
//!     }
//!     ConditionalResult::VersionMismatch(conflict) => {
//!         println!("Conflict detected: {}", conflict);
//!         // Handle conflict appropriately
//!     }
//!     ConditionalResult::NotFound => {
//!         println!("Resource not found");
//!     }
//! }
//!
//! // Delete requires expected version
//! let delete_result = provider.delete_resource(
//!     "User",
//!     "123",
//!     &updated_version,
//!     &context
//! ).await?;
//! # Ok(())
//! # }
//! ```

use super::{
    core::{ListQuery, RequestContext, Resource},
    version::{ConditionalResult, ScimVersion, VersionConflict},
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::future::Future;

/// A resource with its associated version information.
///
/// This is the primary resource type returned by all provider operations.
/// It combines a SCIM resource with its current version for concurrency control.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionedResource {
    /// The SCIM resource data
    resource: Resource,
    /// The version computed from the resource content
    version: ScimVersion,
}

impl VersionedResource {
    /// Create a new versioned resource with auto-computed version.
    pub fn new(resource: Resource) -> Self {
        let version = Self::compute_version(&resource);
        Self { resource, version }
    }

    /// Create a versioned resource with a specific version.
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
    pub fn update_resource(&mut self, new_resource: Resource) {
        self.version = Self::compute_version(&new_resource);
        self.resource = new_resource;
    }

    /// Check if this resource's version matches the expected version.
    pub fn version_matches(&self, expected: &ScimVersion) -> bool {
        self.version.matches(expected)
    }

    /// Refresh the version based on current resource content.
    pub fn refresh_version(&mut self) {
        self.version = Self::compute_version(&self.resource);
    }

    /// Compute version from resource content.
    fn compute_version(resource: &Resource) -> ScimVersion {
        let json_bytes = resource.to_json().unwrap().to_string().into_bytes();
        ScimVersion::from_content(&json_bytes)
    }
}

/// Enhanced resource provider with built-in version support.
///
/// This trait replaces the original ResourceProvider with version-aware operations.
/// All operations now include version handling for optimistic concurrency control.
///
/// # Key Changes from Original ResourceProvider
///
/// * `create_resource` returns `VersionedResource` instead of `Resource`
/// * `get_resource` returns `Option<VersionedResource>` instead of `Option<Resource>`
/// * `update_resource` requires `expected_version` parameter and returns `ConditionalResult`
/// * `delete_resource` requires `expected_version` parameter and returns `ConditionalResult`
/// * `list_resources` returns `Vec<VersionedResource>` instead of `Vec<Resource>`
///
/// # Thread Safety
///
/// Implementations must ensure that version checking and resource modification
/// are atomic to prevent race conditions between concurrent operations.
pub trait EnhancedResourceProvider {
    type Error: std::error::Error + Send + Sync + 'static;

    /// Create a resource and return it with version information.
    ///
    /// # Arguments
    /// * `resource_type` - The type of resource to create (e.g., "User", "Group")
    /// * `data` - The resource data as JSON
    /// * `context` - Request context containing tenant information
    ///
    /// # Returns
    /// The created resource with its initial version
    fn create_resource(
        &self,
        resource_type: &str,
        data: Value,
        context: &RequestContext,
    ) -> impl Future<Output = Result<VersionedResource, Self::Error>> + Send;

    /// Get a resource by ID with its version information.
    ///
    /// # Arguments
    /// * `resource_type` - The type of resource to retrieve
    /// * `id` - The unique identifier of the resource
    /// * `context` - Request context containing tenant information
    ///
    /// # Returns
    /// The versioned resource if found, None if not found
    fn get_resource(
        &self,
        resource_type: &str,
        id: &str,
        context: &RequestContext,
    ) -> impl Future<Output = Result<Option<VersionedResource>, Self::Error>> + Send;

    /// Update a resource only if the version matches.
    ///
    /// This operation will only succeed if the current resource version matches
    /// the expected version, preventing concurrent modification conflicts.
    ///
    /// # Arguments
    /// * `resource_type` - The type of resource to update
    /// * `id` - The unique identifier of the resource
    /// * `data` - The updated resource data as JSON
    /// * `expected_version` - The version the client expects the resource to have
    /// * `context` - Request context containing tenant information
    ///
    /// # Returns
    /// * `Success(VersionedResource)` - Update succeeded with new version
    /// * `VersionMismatch(VersionConflict)` - Resource was modified by another client
    /// * `NotFound` - Resource does not exist
    fn update_resource(
        &self,
        resource_type: &str,
        id: &str,
        data: Value,
        expected_version: &ScimVersion,
        context: &RequestContext,
    ) -> impl Future<Output = Result<ConditionalResult<VersionedResource>, Self::Error>> + Send;

    /// Delete a resource only if the version matches.
    ///
    /// This operation will only succeed if the current resource version matches
    /// the expected version, preventing accidental deletion of modified resources.
    ///
    /// # Arguments
    /// * `resource_type` - The type of resource to delete
    /// * `id` - The unique identifier of the resource
    /// * `expected_version` - The version the client expects the resource to have
    /// * `context` - Request context containing tenant information
    ///
    /// # Returns
    /// * `Success(())` - Delete succeeded
    /// * `VersionMismatch(VersionConflict)` - Resource was modified by another client
    /// * `NotFound` - Resource does not exist
    fn delete_resource(
        &self,
        resource_type: &str,
        id: &str,
        expected_version: &ScimVersion,
        context: &RequestContext,
    ) -> impl Future<Output = Result<ConditionalResult<()>, Self::Error>> + Send;

    /// List resources with their version information.
    ///
    /// # Arguments
    /// * `resource_type` - The type of resources to list
    /// * `query` - Optional query parameters for filtering, sorting, pagination
    /// * `context` - Request context containing tenant information
    ///
    /// # Returns
    /// A vector of versioned resources
    fn list_resources(
        &self,
        resource_type: &str,
        query: Option<&ListQuery>,
        context: &RequestContext,
    ) -> impl Future<Output = Result<Vec<VersionedResource>, Self::Error>> + Send;

    /// Find a resource by attribute value with version information.
    ///
    /// # Arguments
    /// * `resource_type` - The type of resource to search
    /// * `attribute` - The attribute name to search by
    /// * `value` - The attribute value to search for
    /// * `context` - Request context containing tenant information
    ///
    /// # Returns
    /// The first matching versioned resource, if found
    fn find_resource_by_attribute(
        &self,
        resource_type: &str,
        attribute: &str,
        value: &Value,
        context: &RequestContext,
    ) -> impl Future<Output = Result<Option<VersionedResource>, Self::Error>> + Send;

    /// Check if a resource exists.
    ///
    /// # Arguments
    /// * `resource_type` - The type of resource to check
    /// * `id` - The unique identifier of the resource
    /// * `context` - Request context containing tenant information
    ///
    /// # Returns
    /// True if the resource exists, false otherwise
    fn resource_exists(
        &self,
        resource_type: &str,
        id: &str,
        context: &RequestContext,
    ) -> impl Future<Output = Result<bool, Self::Error>> + Send;
}

/// Extension trait providing convenience methods for enhanced providers.
pub trait EnhancedProviderExt: EnhancedResourceProvider {
    /// Convenience method for single-tenant resource creation.
    fn create_single_tenant(
        &self,
        resource_type: &str,
        data: Value,
        request_id: Option<String>,
    ) -> impl Future<Output = Result<VersionedResource, Self::Error>> + Send
    where
        Self: Sync,
    {
        async move {
            let context = match request_id {
                Some(id) => RequestContext::new(id),
                None => RequestContext::with_generated_id(),
            };
            self.create_resource(resource_type, data, &context).await
        }
    }

    /// Convenience method for multi-tenant resource creation.
    fn create_multi_tenant(
        &self,
        tenant_id: &str,
        resource_type: &str,
        data: Value,
        request_id: Option<String>,
    ) -> impl Future<Output = Result<VersionedResource, Self::Error>> + Send
    where
        Self: Sync,
    {
        async move {
            use super::core::TenantContext;

            let tenant_context = TenantContext {
                tenant_id: tenant_id.to_string(),
                client_id: "default-client".to_string(),
                permissions: Default::default(),
                isolation_level: Default::default(),
            };

            let context = match request_id {
                Some(id) => RequestContext::with_tenant(id, tenant_context),
                None => RequestContext::with_tenant_generated_id(tenant_context),
            };

            self.create_resource(resource_type, data, &context).await
        }
    }

    /// Convenience method for single-tenant resource retrieval.
    fn get_single_tenant(
        &self,
        resource_type: &str,
        id: &str,
        request_id: Option<String>,
    ) -> impl Future<Output = Result<Option<VersionedResource>, Self::Error>> + Send
    where
        Self: Sync,
    {
        async move {
            let context = match request_id {
                Some(req_id) => RequestContext::new(req_id),
                None => RequestContext::with_generated_id(),
            };
            self.get_resource(resource_type, id, &context).await
        }
    }

    /// Force update without version checking (use with caution).
    ///
    /// This bypasses version checking by first retrieving the current version
    /// and then performing the update. Should only be used in scenarios where
    /// version conflicts are acceptable or when migrating data.
    fn force_update(
        &self,
        resource_type: &str,
        id: &str,
        data: Value,
        context: &RequestContext,
    ) -> impl Future<Output = Result<Option<VersionedResource>, Self::Error>> + Send
    where
        Self: Sync,
    {
        async move {
            // Get current version first
            let current = self.get_resource(resource_type, id, context).await?;

            if let Some(versioned) = current {
                // Use current version for update
                match self
                    .update_resource(resource_type, id, data, versioned.version(), context)
                    .await?
                {
                    ConditionalResult::Success(updated) => Ok(Some(updated)),
                    ConditionalResult::NotFound => Ok(None),
                    ConditionalResult::VersionMismatch(_) => {
                        // This should not happen since we just got the version,
                        // but if it does, it means concurrent modification occurred
                        // between our get and update. Return None to indicate failure.
                        Ok(None)
                    }
                }
            } else {
                Ok(None)
            }
        }
    }
}

/// Blanket implementation of EnhancedProviderExt for all types implementing EnhancedResourceProvider.
impl<T: EnhancedResourceProvider> EnhancedProviderExt for T {}

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
                "userName": "test.user",
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
                "userName": "test.user",
                "active": true
            }),
        )
        .unwrap();

        let resource2 = Resource::from_json(
            "User".to_string(),
            json!({
                "id": "123",
                "userName": "test.user",
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
                "userName": "test.user",
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
                "userName": "test.user",
                "active": false
            }),
        )
        .unwrap();

        versioned.update_resource(updated_resource);

        // Version should change after update
        assert!(!versioned.version().matches(&old_version));
        assert_eq!(versioned.resource().get("active").unwrap(), &json!(false));
    }

    #[test]
    fn test_versioned_resource_version_matching() {
        let resource = Resource::from_json(
            "User".to_string(),
            json!({
                "id": "123",
                "userName": "test.user"
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
