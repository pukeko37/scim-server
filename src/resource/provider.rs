//! Resource provider trait for implementing SCIM data access.
//!
//! This module defines the core trait that users must implement to provide
//! data storage and retrieval for SCIM resources. The design is async-first
//! and provides comprehensive error handling.
//!
//! The unified ResourceProvider supports both single-tenant and multi-tenant
//! operations through the RequestContext. Single-tenant operations use
//! context.tenant_context = None, while multi-tenant operations provide
//! tenant information in context.tenant_context = Some(tenant_context).

use super::core::{ListQuery, RequestContext, Resource};
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
