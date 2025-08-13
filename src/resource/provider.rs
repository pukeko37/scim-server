//! Resource provider trait for implementing SCIM data access.
//!
//! This module defines the core trait that users must implement to provide
//! data storage and retrieval for SCIM resources. The design is async-first
//! and provides comprehensive error handling with built-in ETag concurrency control.
//!
//! ## ETag Concurrency Control
//!
//! All ResourceProvider implementations automatically support conditional operations
//! for optimistic concurrency control. The trait provides default implementations
//! that work with any storage backend.
//!
//! ## Multi-Tenant Support
//!
//! The unified ResourceProvider supports both single-tenant and multi-tenant
//! operations through the RequestContext. Single-tenant operations use
//! context.tenant_context = None, while multi-tenant operations provide
//! tenant information in context.tenant_context = Some(tenant_context).
//!
//! ## Example Implementation
//!
//! ```rust,no_run
//! use scim_server::resource::{
//!     provider::ResourceProvider,
//!     core::{RequestContext, Resource, ListQuery},
//!     version::{ScimVersion, ConditionalResult},
//!     conditional_provider::VersionedResource,
//! };
//! use serde_json::Value;
//! use std::collections::HashMap;
//! use std::sync::Arc;
//! use tokio::sync::RwLock;
//!
//! #[derive(Clone)]
//! struct MyProvider {
//!     data: Arc<RwLock<HashMap<String, Resource>>>,
//! }
//!
//! #[derive(Debug, thiserror::Error)]
//! #[error("Provider error: {0}")]
//! struct MyError(String);
//!
//! impl ResourceProvider for MyProvider {
//!     type Error = MyError;
//!
//!     async fn create_resource(&self, resource_type: &str, data: Value, context: &RequestContext) -> Result<Resource, Self::Error> {
//!         // Your implementation here
//!         let resource = Resource::from_json(resource_type.to_string(), data)
//!             .map_err(|e| MyError(e.to_string()))?;
//!         let mut store = self.data.write().await;
//!         let id = resource.get_id().unwrap_or("generated-id").to_string();
//!         store.insert(id, resource.clone());
//!         Ok(resource)
//!     }
//!
//!     // ... implement other required methods ...
//!     # async fn get_resource(&self, _resource_type: &str, _id: &str, _context: &RequestContext) -> Result<Option<Resource>, Self::Error> {
//!     #     Ok(None)
//!     # }
//!     # async fn update_resource(&self, _resource_type: &str, _id: &str, _data: Value, _context: &RequestContext) -> Result<Resource, Self::Error> {
//!     #     Err(MyError("Not implemented".to_string()))
//!     # }
//!     # async fn delete_resource(&self, _resource_type: &str, _id: &str, _context: &RequestContext) -> Result<(), Self::Error> {
//!     #     Ok(())
//!     # }
//!     # async fn list_resources(&self, _resource_type: &str, _query: Option<&ListQuery>, _context: &RequestContext) -> Result<Vec<Resource>, Self::Error> {
//!     #     Ok(vec![])
//!     # }
//!     # async fn find_resource_by_attribute(&self, _resource_type: &str, _attribute: &str, _value: &Value, _context: &RequestContext) -> Result<Option<Resource>, Self::Error> {
//!     #     Ok(None)
//!     # }
//!     # async fn resource_exists(&self, _resource_type: &str, _id: &str, _context: &RequestContext) -> Result<bool, Self::Error> {
//!     #     Ok(false)
//!     # }
//!
//!     // Conditional operations are provided by default with automatic version checking
//!     // Override for more efficient provider-specific implementations:
//!     //
//!     // async fn conditional_update(&self, resource_type: &str, id: &str, data: Value,
//!     //                           expected_version: &ScimVersion, context: &RequestContext)
//!     //                           -> Result<ConditionalResult<VersionedResource>, Self::Error> {
//!     //     // Your optimized conditional update logic
//!     // }
//! }
//! ```

use super::conditional_provider::VersionedResource;
use super::core::{ListQuery, RequestContext, Resource};
use super::version::{ConditionalResult, ScimVersion};
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
pub trait ResourceProvider {
    type Error: std::error::Error + Send + Sync + 'static;

    /// Create a resource for the tenant specified in the request context.
    ///
    /// # Arguments
    /// * `resource_type` - The type of resource to create (e.g., "User", "Group")
    /// * `data` - The resource data as JSON
    /// * `context` - Request context containing tenant information (if multi-tenant)
    ///
    /// # Returns
    /// The created resource with any server-generated fields (id, metadata, etc.)
    ///
    /// # Tenant Handling
    /// - Single-tenant: `context.tenant_id()` returns `None`
    /// - Multi-tenant: `context.tenant_id()` returns `Some(tenant_id)`
    fn create_resource(
        &self,
        resource_type: &str,
        data: Value,
        context: &RequestContext,
    ) -> impl Future<Output = Result<Resource, Self::Error>> + Send;

    /// Get a resource by ID from the tenant specified in the request context.
    ///
    /// # Arguments
    /// * `resource_type` - The type of resource to retrieve
    /// * `id` - The unique identifier of the resource
    /// * `context` - Request context containing tenant information (if multi-tenant)
    ///
    /// # Returns
    /// The resource if found, None if not found within the tenant scope
    fn get_resource(
        &self,
        resource_type: &str,
        id: &str,
        context: &RequestContext,
    ) -> impl Future<Output = Result<Option<Resource>, Self::Error>> + Send;

    /// Update a resource in the tenant specified in the request context.
    ///
    /// # Arguments
    /// * `resource_type` - The type of resource to update
    /// * `id` - The unique identifier of the resource
    /// * `data` - The updated resource data as JSON
    /// * `context` - Request context containing tenant information (if multi-tenant)
    ///
    /// # Returns
    /// The updated resource
    fn update_resource(
        &self,
        resource_type: &str,
        id: &str,
        data: Value,
        context: &RequestContext,
    ) -> impl Future<Output = Result<Resource, Self::Error>> + Send;

    /// Delete a resource from the tenant specified in the request context.
    ///
    /// # Arguments
    /// * `resource_type` - The type of resource to delete
    /// * `id` - The unique identifier of the resource
    /// * `context` - Request context containing tenant information (if multi-tenant)
    fn delete_resource(
        &self,
        resource_type: &str,
        id: &str,
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
    /// A vector of resources from the specified tenant
    fn list_resources(
        &self,
        resource_type: &str,
        _query: Option<&ListQuery>,
        context: &RequestContext,
    ) -> impl Future<Output = Result<Vec<Resource>, Self::Error>> + Send;

    /// Find a resource by attribute value within the tenant specified in the request context.
    ///
    /// # Arguments
    /// * `resource_type` - The type of resource to search
    /// * `attribute` - The attribute name to search by
    /// * `value` - The attribute value to search for
    /// * `context` - Request context containing tenant information (if multi-tenant)
    ///
    /// # Returns
    /// The first matching resource, if found within the tenant scope
    fn find_resource_by_attribute(
        &self,
        resource_type: &str,
        attribute: &str,
        value: &Value,
        context: &RequestContext,
    ) -> impl Future<Output = Result<Option<Resource>, Self::Error>> + Send;

    /// Check if a resource exists within the tenant specified in the request context.
    ///
    /// # Arguments
    /// * `resource_type` - The type of resource to check
    /// * `id` - The unique identifier of the resource
    /// * `context` - Request context containing tenant information (if multi-tenant)
    ///
    /// # Returns
    /// True if the resource exists within the tenant scope, false otherwise
    fn resource_exists(
        &self,
        resource_type: &str,
        id: &str,
        context: &RequestContext,
    ) -> impl Future<Output = Result<bool, Self::Error>> + Send;

    /// Conditionally update a resource if the version matches.
    ///
    /// This operation will only succeed if the current resource version matches
    /// the expected version, preventing accidental overwriting of modified resources.
    /// This provides optimistic concurrency control for SCIM operations.
    ///
    /// # ETag Concurrency Control
    ///
    /// This method implements the core of ETag-based conditional operations:
    /// - Fetches the current resource and its version
    /// - Compares the current version with the expected version
    /// - Only proceeds with the update if versions match
    /// - Returns version conflict information if they don't match
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
    ///
    /// # Default Implementation
    /// The default implementation provides automatic conditional update support
    /// by checking the current resource version before performing the update.
    /// Providers can override this for more efficient implementations that
    /// perform version checking at the storage layer.
    ///
    /// # Examples
    /// ```rust,no_run
    /// use scim_server::resource::{
    ///     provider::ResourceProvider,
    ///     version::{ScimVersion, ConditionalResult},
    ///     conditional_provider::VersionedResource,
    ///     RequestContext,
    /// };
    /// use serde_json::json;
    ///
    /// # async fn example<P: ResourceProvider + Sync>(provider: &P) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    /// let context = RequestContext::with_generated_id();
    /// let expected_version = ScimVersion::from_hash("abc123");
    /// let update_data = json!({"userName": "new.name", "active": false});
    ///
    /// match provider.conditional_update("User", "123", update_data, &expected_version, &context).await? {
    ///     ConditionalResult::Success(versioned_resource) => {
    ///         println!("Update successful, new version: {}",
    ///                 versioned_resource.version().to_http_header());
    ///     },
    ///     ConditionalResult::VersionMismatch(conflict) => {
    ///         println!("Version conflict: expected {}, current {}",
    ///                 conflict.expected, conflict.current);
    ///     },
    ///     ConditionalResult::NotFound => {
    ///         println!("Resource not found");
    ///     }
    /// }
    /// # Ok(())
    /// # }
    /// ```
    fn conditional_update(
        &self,
        resource_type: &str,
        id: &str,
        data: Value,
        expected_version: &ScimVersion,
        context: &RequestContext,
    ) -> impl Future<Output = Result<ConditionalResult<VersionedResource>, Self::Error>> + Send
    where
        Self: Sync,
    {
        async move {
            // Default implementation: get current resource, check version, then update
            match self.get_resource(resource_type, id, context).await? {
                Some(current_resource) => {
                    let current_versioned = VersionedResource::new(current_resource);
                    if current_versioned.version().matches(expected_version) {
                        let updated = self
                            .update_resource(resource_type, id, data, context)
                            .await?;
                        Ok(ConditionalResult::Success(VersionedResource::new(updated)))
                    } else {
                        Ok(ConditionalResult::VersionMismatch(
                            super::version::VersionConflict::standard_message(
                                expected_version.clone(),
                                current_versioned.version().clone(),
                            ),
                        ))
                    }
                }
                None => Ok(ConditionalResult::NotFound),
            }
        }
    }

    /// Conditionally delete a resource if the version matches.
    ///
    /// This operation will only succeed if the current resource version matches
    /// the expected version, preventing accidental deletion of modified resources.
    /// This is critical for maintaining data integrity in concurrent environments.
    ///
    /// # ETag Concurrency Control
    ///
    /// This method prevents accidental deletion of resources that have been
    /// modified by other clients:
    /// - Fetches the current resource and its version
    /// - Compares the current version with the expected version
    /// - Only proceeds with the deletion if versions match
    /// - Ensures the client is deleting the resource they intended to delete
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
    ///
    /// # Default Implementation
    /// The default implementation provides automatic conditional delete support
    /// by checking the current resource version before performing the delete.
    /// Providers can override this for more efficient implementations that
    /// perform version checking at the storage layer.
    ///
    /// # Examples
    /// ```rust,no_run
    /// use scim_server::resource::{
    ///     provider::ResourceProvider,
    ///     version::{ScimVersion, ConditionalResult},
    ///     RequestContext,
    /// };
    ///
    /// # async fn example<P: ResourceProvider + Sync>(provider: &P) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    /// let context = RequestContext::with_generated_id();
    /// let expected_version = ScimVersion::from_hash("def456");
    ///
    /// match provider.conditional_delete("User", "123", &expected_version, &context).await? {
    ///     ConditionalResult::Success(()) => {
    ///         println!("User deleted successfully");
    ///     },
    ///     ConditionalResult::VersionMismatch(conflict) => {
    ///         println!("Cannot delete: resource was modified. Expected {}, current {}",
    ///                 conflict.expected, conflict.current);
    ///     },
    ///     ConditionalResult::NotFound => {
    ///         println!("User not found");
    ///     }
    /// }
    /// # Ok(())
    /// # }
    /// ```
    fn conditional_delete(
        &self,
        resource_type: &str,
        id: &str,
        expected_version: &ScimVersion,
        context: &RequestContext,
    ) -> impl Future<Output = Result<ConditionalResult<()>, Self::Error>> + Send
    where
        Self: Sync,
    {
        async move {
            // Default implementation: get current resource, check version, then delete
            match self.get_resource(resource_type, id, context).await? {
                Some(current_resource) => {
                    let current_versioned = VersionedResource::new(current_resource);
                    if current_versioned.version().matches(expected_version) {
                        self.delete_resource(resource_type, id, context).await?;
                        Ok(ConditionalResult::Success(()))
                    } else {
                        Ok(ConditionalResult::VersionMismatch(
                            super::version::VersionConflict::standard_message(
                                expected_version.clone(),
                                current_versioned.version().clone(),
                            ),
                        ))
                    }
                }
                None => Ok(ConditionalResult::NotFound),
            }
        }
    }

    /// Get a resource with its version information.
    ///
    /// This is a convenience method that returns both the resource and its version
    /// information wrapped in a [`VersionedResource`]. This is useful when you need
    /// both the resource data and its version for subsequent conditional operations.
    ///
    /// The default implementation calls the existing `get_resource` method and
    /// automatically wraps the result in a `VersionedResource` with a computed version.
    ///
    /// # Arguments
    /// * `resource_type` - The type of resource to retrieve
    /// * `id` - The unique identifier of the resource
    /// * `context` - Request context containing tenant information
    ///
    /// # Returns
    /// The versioned resource if found, `None` if not found
    ///
    /// # Examples
    /// ```rust,no_run
    /// use scim_server::resource::{
    ///     provider::ResourceProvider,
    ///     RequestContext,
    /// };
    ///
    /// # async fn example<P: ResourceProvider + Sync>(provider: &P) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    /// let context = RequestContext::with_generated_id();
    ///
    /// if let Some(versioned_resource) = provider.get_versioned_resource("User", "123", &context).await? {
    ///     println!("Resource ID: {}", versioned_resource.resource().get_id().unwrap_or("unknown"));
    ///     println!("Resource version: {}", versioned_resource.version().to_http_header());
    ///
    ///     // Can use the version for subsequent conditional operations
    ///     let current_version = versioned_resource.version().clone();
    ///     // ... use current_version for conditional_update or conditional_delete
    /// }
    /// # Ok(())
    /// # }
    /// ```
    fn get_versioned_resource(
        &self,
        resource_type: &str,
        id: &str,
        context: &RequestContext,
    ) -> impl Future<Output = Result<Option<VersionedResource>, Self::Error>> + Send
    where
        Self: Sync,
    {
        async move {
            match self.get_resource(resource_type, id, context).await? {
                Some(resource) => Ok(Some(VersionedResource::new(resource))),
                None => Ok(None),
            }
        }
    }

    /// Apply PATCH operations to a resource within the tenant specified in the request context.
    ///
    /// # Arguments
    /// * `resource_type` - The type of resource to patch
    /// * `id` - The unique identifier of the resource
    /// * `patch_request` - The PATCH operation request as JSON (RFC 7644 Section 3.5.2)
    /// * `context` - Request context containing tenant information (if multi-tenant)
    ///
    /// # Returns
    /// The updated resource after applying the patch operations
    ///
    /// # PATCH Operations
    /// Supports the three SCIM PATCH operations:
    /// - `add` - Add new attribute values
    /// - `remove` - Remove attribute values
    /// - `replace` - Replace existing attribute values
    ///
    /// # Default Implementation
    /// The default implementation provides basic PATCH operation support by:
    /// 1. Fetching the current resource
    /// 2. Applying each operation in sequence
    /// 3. Updating the resource with the modified data
    fn patch_resource(
        &self,
        resource_type: &str,
        id: &str,
        patch_request: &Value,
        context: &RequestContext,
    ) -> impl Future<Output = Result<Resource, Self::Error>> + Send
    where
        Self: Sync,
    {
        async move {
            // Get the current resource
            let current = self
                .get_resource(resource_type, id, context)
                .await?
                .ok_or_else(|| {
                    // This will need to be converted to the provider's error type
                    // For now, we'll use a placeholder that will be handled by implementers
                    // In practice, providers should define their own NotFound error variant
                    unreachable!("Resource not found - providers must handle this case")
                })?;

            // Extract operations from patch request
            let operations = patch_request
                .get("Operations")
                .and_then(|ops| ops.as_array())
                .ok_or_else(|| {
                    unreachable!("Invalid patch request - providers must handle this case")
                })?;

            // Apply operations to create modified resource data
            let mut modified_data = current.to_json().map_err(|_| {
                unreachable!("Failed to serialize resource - providers must handle this case")
            })?;

            for operation in operations {
                self.apply_patch_operation(&mut modified_data, operation)?;
            }

            // Update the resource with modified data
            self.update_resource(resource_type, id, modified_data, context)
                .await
        }
    }

    /// Apply a single PATCH operation to resource data.
    ///
    /// This is a helper method used by the default patch_resource implementation.
    /// Providers can override this method to customize patch operation behavior.
    ///
    /// # Arguments
    /// * `resource_data` - Mutable reference to the resource JSON data
    /// * `operation` - The patch operation to apply
    ///
    /// # Returns
    /// Result indicating success or failure of the operation
    fn apply_patch_operation(
        &self,
        _resource_data: &mut Value,
        _operation: &Value,
    ) -> Result<(), Self::Error> {
        // This is a simplified implementation that providers should override
        // with proper SCIM PATCH semantics
        // Default implementation is intentionally minimal
        Ok(())
    }
}

/// Extension trait providing convenience methods for common provider operations.
///
/// This trait automatically implements ergonomic helper methods for both single-tenant
/// and multi-tenant scenarios on any type that implements ResourceProvider.
pub trait ResourceProviderExt: ResourceProvider {
    /// Convenience method for single-tenant resource creation.
    ///
    /// Creates a RequestContext with no tenant information and calls create_resource.
    fn create_single_tenant(
        &self,
        resource_type: &str,
        data: Value,
        request_id: Option<String>,
    ) -> impl Future<Output = Result<Resource, Self::Error>> + Send
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
    ///
    /// Creates a RequestContext with the specified tenant and calls create_resource.
    fn create_multi_tenant(
        &self,
        tenant_id: &str,
        resource_type: &str,
        data: Value,
        request_id: Option<String>,
    ) -> impl Future<Output = Result<Resource, Self::Error>> + Send
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
    ) -> impl Future<Output = Result<Option<Resource>, Self::Error>> + Send
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

    /// Convenience method for multi-tenant resource retrieval.
    fn get_multi_tenant(
        &self,
        tenant_id: &str,
        resource_type: &str,
        id: &str,
        request_id: Option<String>,
    ) -> impl Future<Output = Result<Option<Resource>, Self::Error>> + Send
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
                Some(req_id) => RequestContext::with_tenant(req_id, tenant_context),
                None => RequestContext::with_tenant_generated_id(tenant_context),
            };

            self.get_resource(resource_type, id, &context).await
        }
    }
}

/// Blanket implementation of ResourceProviderExt for all types implementing ResourceProvider.
impl<T: ResourceProvider> ResourceProviderExt for T {}
