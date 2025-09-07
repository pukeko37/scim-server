//! Resource provider trait for implementing SCIM data access.
//!
//! This module defines the core trait that users must implement to provide
//! data storage and retrieval for SCIM resources. Supports both single-tenant
//! and multi-tenant operations with optional version-aware concurrency control.
//!
//! # Key Types
//!
//! - [`ResourceProvider`] - Main trait for implementing storage backends
//!
//! # Examples
//!
//! ```rust
//! use scim_server::providers::ResourceProvider;
//!
//! struct MyProvider;
//! // Implement ResourceProvider for your storage backend
//! ```

use crate::resource::{
    ListQuery, RequestContext, version::RawVersion, versioned::VersionedResource,
};
use serde_json::Value;
use std::future::Future;

/// Unified resource provider trait supporting both single and multi-tenant operations.
///
/// This trait provides a unified interface for SCIM resource operations that works
/// for both single-tenant and multi-tenant scenarios:
///
/// - **Single-tenant**: Operations use RequestContext with tenant_context = None
/// - **Multi-tenant**: Operations use RequestContext with tenant_context = Some(...)
///
/// The provider implementation can check `context.tenant_id()` to determine
/// the effective tenant for the operation.
///
/// # Version Control
///
/// All operations support optional version control through the `expected_version` parameter.
/// When provided, operations perform optimistic concurrency control to prevent lost updates.
/// When `None`, operations proceed without version checking.
pub trait ResourceProvider {
    /// Error type returned by all provider operations
    type Error: std::error::Error + Send + Sync + 'static;

    /// Create a resource for the tenant specified in the request context.
    ///
    /// # Arguments
    /// * `resource_type` - The type of resource to create (e.g., "User", "Group")
    /// * `data` - The resource data as JSON
    /// * `context` - Request context containing tenant information (if multi-tenant)
    ///
    /// # Returns
    /// The created resource with version information and any server-generated fields
    ///
    /// # Tenant Handling
    /// - Single-tenant: `context.tenant_id()` returns `None`
    /// - Multi-tenant: `context.tenant_id()` returns `Some(tenant_id)`
    fn create_resource(
        &self,
        resource_type: &str,
        data: Value,
        context: &RequestContext,
    ) -> impl Future<Output = Result<VersionedResource, Self::Error>> + Send;

    /// Get a resource by ID from the tenant specified in the request context.
    ///
    /// # Arguments
    /// * `resource_type` - The type of resource to retrieve
    /// * `id` - The unique identifier of the resource
    /// * `context` - Request context containing tenant information (if multi-tenant)
    ///
    /// # Returns
    /// The resource with version information if found, None if not found within the tenant scope
    fn get_resource(
        &self,
        resource_type: &str,
        id: &str,
        context: &RequestContext,
    ) -> impl Future<Output = Result<Option<VersionedResource>, Self::Error>> + Send;

    /// Update a resource in the tenant specified in the request context.
    ///
    /// Supports both regular updates and conditional updates with version checking.
    /// When `expected_version` is provided, performs optimistic concurrency control.
    ///
    /// # Arguments
    /// * `resource_type` - The type of resource to update
    /// * `id` - The unique identifier of the resource
    /// * `data` - The updated resource data as JSON
    /// * `expected_version` - Optional expected version for conditional updates
    /// * `context` - Request context containing tenant information (if multi-tenant)
    ///
    /// # Returns
    /// - When `expected_version` is `None`: `Ok(VersionedResource)` with the updated resource
    /// - When `expected_version` is `Some`: `Ok(VersionedResource)` on success, or error on version mismatch
    fn update_resource(
        &self,
        resource_type: &str,
        id: &str,
        data: Value,
        expected_version: Option<&RawVersion>,
        context: &RequestContext,
    ) -> impl Future<Output = Result<VersionedResource, Self::Error>> + Send;

    /// Delete a resource from the tenant specified in the request context.
    ///
    /// Supports both regular deletes and conditional deletes with version checking.
    /// When `expected_version` is provided, performs optimistic concurrency control.
    ///
    /// # Arguments
    /// * `resource_type` - The type of resource to delete
    /// * `id` - The unique identifier of the resource
    /// * `expected_version` - Optional expected version for conditional deletes
    /// * `context` - Request context containing tenant information (if multi-tenant)
    ///
    /// # Returns
    /// - When `expected_version` is `None`: `Ok(())` on successful deletion
    /// - When `expected_version` is `Some`: `Ok(())` on success, or error on version mismatch
    fn delete_resource(
        &self,
        resource_type: &str,
        id: &str,
        expected_version: Option<&RawVersion>,
        context: &RequestContext,
    ) -> impl Future<Output = Result<(), Self::Error>> + Send;

    /// List resources from the tenant specified in the request context.
    ///
    /// # Arguments
    /// * `resource_type` - The type of resources to list
    /// * `query` - Optional query parameters for filtering, sorting, pagination
    /// * `context` - Request context containing tenant information (if multi-tenant)
    ///
    /// # Returns
    /// A vector of resources with version information from the specified tenant
    fn list_resources(
        &self,
        resource_type: &str,
        _query: Option<&ListQuery>,
        context: &RequestContext,
    ) -> impl Future<Output = Result<Vec<VersionedResource>, Self::Error>> + Send;

    /// Find resources by attribute value within the tenant specified in the request context.
    ///
    /// # Arguments
    /// * `resource_type` - The type of resources to search
    /// * `attribute_name` - The attribute name to search by (e.g., "userName")
    /// * `attribute_value` - The attribute value to match
    /// * `context` - Request context containing tenant information (if multi-tenant)
    ///
    /// # Returns
    /// A vector of matching resources with version information from the specified tenant
    fn find_resources_by_attribute(
        &self,
        resource_type: &str,
        attribute_name: &str,
        attribute_value: &str,
        context: &RequestContext,
    ) -> impl Future<Output = Result<Vec<VersionedResource>, Self::Error>> + Send;

    /// Apply a PATCH operation to a resource.
    ///
    /// Supports both regular patches and conditional patches with version checking.
    /// When `expected_version` is provided, performs optimistic concurrency control.
    ///
    /// # Arguments
    /// * `resource_type` - The type of resource to patch
    /// * `id` - The unique identifier of the resource
    /// * `patch_request` - The PATCH operations as JSON
    /// * `expected_version` - Optional expected version for conditional patches
    /// * `context` - Request context containing tenant information
    ///
    /// # Returns
    /// - When `expected_version` is `None`: `Ok(VersionedResource)` with the patched resource
    /// - When `expected_version` is `Some`: `Ok(VersionedResource)` on success, or error on version mismatch
    fn patch_resource(
        &self,
        resource_type: &str,
        id: &str,
        patch_request: &Value,
        expected_version: Option<&RawVersion>,
        context: &RequestContext,
    ) -> impl Future<Output = Result<VersionedResource, Self::Error>> + Send;

    /// Check if a resource exists within the tenant specified in the request context.
    ///
    /// # Arguments
    /// * `resource_type` - The type of resource to check
    /// * `id` - The unique identifier of the resource
    /// * `context` - Request context containing tenant information (if multi-tenant)
    ///
    /// # Returns
    /// True if the resource exists in the tenant scope, false otherwise
    fn resource_exists(
        &self,
        resource_type: &str,
        id: &str,
        context: &RequestContext,
    ) -> impl Future<Output = Result<bool, Self::Error>> + Send;
}

/// Extension trait providing convenience methods for ResourceProvider implementations.
///
/// This trait provides single-tenant convenience methods that automatically create
/// the appropriate RequestContext for common scenarios.
pub trait ResourceProviderExt: ResourceProvider {
    /// Convenience method for single-tenant resource creation.
    ///
    /// Creates a RequestContext with no tenant information and calls create_resource.
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
                Some(id) => RequestContext::new(id),
                None => RequestContext::with_generated_id(),
            };

            self.get_resource(resource_type, id, &context).await
        }
    }

    /// Convenience method for single-tenant resource update.
    fn update_single_tenant(
        &self,
        resource_type: &str,
        id: &str,
        data: Value,
        expected_version: Option<&RawVersion>,
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

            self.update_resource(resource_type, id, data, expected_version, &context)
                .await
        }
    }

    /// Convenience method for single-tenant resource deletion.
    fn delete_single_tenant(
        &self,
        resource_type: &str,
        id: &str,
        expected_version: Option<&RawVersion>,
        request_id: Option<String>,
    ) -> impl Future<Output = Result<(), Self::Error>> + Send
    where
        Self: Sync,
    {
        async move {
            let context = match request_id {
                Some(id) => RequestContext::new(id),
                None => RequestContext::with_generated_id(),
            };

            self.delete_resource(resource_type, id, expected_version, &context)
                .await
        }
    }
}

// Automatically implement ResourceProviderExt for all ResourceProvider implementations
impl<T: ResourceProvider> ResourceProviderExt for T {}
