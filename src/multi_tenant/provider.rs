//! Multi-tenant resource provider trait.
//!
//! This module defines the `MultiTenantResourceProvider` trait that extends
//! the basic `ResourceProvider` with tenant-aware operations. This enables
//! proper tenant isolation and multi-tenant resource management.

use crate::resource::{EnhancedRequestContext, ListQuery, Resource};
use serde_json::Value;
use std::future::Future;

/// Multi-tenant resource provider trait for tenant-aware SCIM operations.
///
/// This trait extends the basic resource operations with tenant context,
/// ensuring that all operations are scoped to the appropriate tenant and
/// that tenant isolation is maintained.
///
/// # Design Principles
///
/// * **Tenant Isolation**: All operations include explicit tenant scoping
/// * **Type Safety**: Tenant context is required for all operations
/// * **Performance**: Async-first design for scalability
/// * **Flexibility**: Generic error handling for different implementations
///
/// # Example Implementation
///
/// ```rust,no_run
/// use scim_server::multi_tenant::MultiTenantResourceProvider;
/// use scim_server::{EnhancedRequestContext, Resource, ListQuery};
/// use serde_json::Value;
/// use std::collections::HashMap;
/// use std::sync::Arc;
/// use tokio::sync::RwLock;
///
/// struct MyMultiTenantProvider {
///     // tenant_id -> resource_type -> resource_id -> resource
///     data: Arc<RwLock<HashMap<String, HashMap<String, HashMap<String, Resource>>>>>,
/// }
///
/// #[derive(Debug, thiserror::Error)]
/// #[error("Provider error: {message}")]
/// struct ProviderError {
///     message: String,
/// }
///
/// impl MultiTenantResourceProvider for MyMultiTenantProvider {
///     type Error = ProviderError;
///
///     async fn create_resource(
///         &self,
///         tenant_id: &str,
///         resource_type: &str,
///         data: Value,
///         context: &EnhancedRequestContext,
///     ) -> Result<Resource, Self::Error> {
///         // Validate tenant access
///         if context.tenant_id() != tenant_id {
///             return Err(ProviderError {
///                 message: "Tenant mismatch".to_string()
///             });
///         }
///
///         let resource = Resource::new(resource_type.to_string(), data);
///         let resource_id = resource.get_id().unwrap_or("").to_string();
///
///         let mut data_guard = self.data.write().await;
///         data_guard
///             .entry(tenant_id.to_string())
///             .or_insert_with(HashMap::new)
///             .entry(resource_type.to_string())
///             .or_insert_with(HashMap::new)
///             .insert(resource_id, resource.clone());
///
///         Ok(resource)
///     }
///
///     async fn get_resource(
///         &self,
///         tenant_id: &str,
///         resource_type: &str,
///         id: &str,
///         context: &EnhancedRequestContext,
///     ) -> Result<Option<Resource>, Self::Error> {
///         if context.tenant_id() != tenant_id {
///             return Err(ProviderError {
///                 message: "Tenant mismatch".to_string()
///             });
///         }
///
///         let data_guard = self.data.read().await;
///         Ok(data_guard
///             .get(tenant_id)
///             .and_then(|tenant_data| tenant_data.get(resource_type))
///             .and_then(|type_data| type_data.get(id))
///             .cloned())
///     }
///
///     // ... implement other methods
/// #   async fn update_resource(&self, tenant_id: &str, resource_type: &str, id: &str, data: Value, context: &EnhancedRequestContext) -> Result<Resource, Self::Error> { unimplemented!() }
/// #   async fn delete_resource(&self, tenant_id: &str, resource_type: &str, id: &str, context: &EnhancedRequestContext) -> Result<(), Self::Error> { unimplemented!() }
/// #   async fn list_resources(&self, tenant_id: &str, resource_type: &str, query: Option<&ListQuery>, context: &EnhancedRequestContext) -> Result<Vec<Resource>, Self::Error> { unimplemented!() }
/// #   async fn find_resource_by_attribute(&self, tenant_id: &str, resource_type: &str, attribute: &str, value: &Value, context: &EnhancedRequestContext) -> Result<Option<Resource>, Self::Error> { unimplemented!() }
/// #   async fn resource_exists(&self, tenant_id: &str, resource_type: &str, id: &str, context: &EnhancedRequestContext) -> Result<bool, Self::Error> { unimplemented!() }
/// #   async fn get_resource_count(&self, tenant_id: &str, resource_type: &str, context: &EnhancedRequestContext) -> Result<usize, Self::Error> { unimplemented!() }
/// }
/// ```
pub trait MultiTenantResourceProvider {
    /// Error type for provider operations
    type Error: std::error::Error + Send + Sync + 'static;

    /// Create a new resource in the specified tenant.
    ///
    /// # Arguments
    /// * `tenant_id` - The tenant identifier for isolation
    /// * `resource_type` - The type of resource to create (e.g., "User", "Group")
    /// * `data` - The resource data as JSON
    /// * `context` - Enhanced request context with tenant information
    ///
    /// # Returns
    /// The created resource with any server-generated fields (id, metadata, etc.)
    ///
    /// # Errors
    /// Returns an error if:
    /// * Tenant validation fails
    /// * Resource creation fails
    /// * Tenant limits are exceeded
    /// * Permission validation fails
    fn create_resource(
        &self,
        tenant_id: &str,
        resource_type: &str,
        data: Value,
        context: &EnhancedRequestContext,
    ) -> impl Future<Output = Result<Resource, Self::Error>> + Send;

    /// Get a resource by ID from the specified tenant.
    ///
    /// # Arguments
    /// * `tenant_id` - The tenant identifier for isolation
    /// * `resource_type` - The type of resource to retrieve
    /// * `id` - The unique identifier of the resource
    /// * `context` - Enhanced request context with tenant information
    ///
    /// # Returns
    /// The resource if found, None if not found
    ///
    /// # Errors
    /// Returns an error if:
    /// * Tenant validation fails
    /// * Permission validation fails
    /// * Data access fails
    fn get_resource(
        &self,
        tenant_id: &str,
        resource_type: &str,
        id: &str,
        context: &EnhancedRequestContext,
    ) -> impl Future<Output = Result<Option<Resource>, Self::Error>> + Send;

    /// Update an existing resource in the specified tenant.
    ///
    /// # Arguments
    /// * `tenant_id` - The tenant identifier for isolation
    /// * `resource_type` - The type of resource to update
    /// * `id` - The unique identifier of the resource
    /// * `data` - The updated resource data as JSON
    /// * `context` - Enhanced request context with tenant information
    ///
    /// # Returns
    /// The updated resource
    ///
    /// # Errors
    /// Returns an error if:
    /// * Resource not found
    /// * Tenant validation fails
    /// * Permission validation fails
    /// * Update operation fails
    fn update_resource(
        &self,
        tenant_id: &str,
        resource_type: &str,
        id: &str,
        data: Value,
        context: &EnhancedRequestContext,
    ) -> impl Future<Output = Result<Resource, Self::Error>> + Send;

    /// Delete a resource from the specified tenant.
    ///
    /// # Arguments
    /// * `tenant_id` - The tenant identifier for isolation
    /// * `resource_type` - The type of resource to delete
    /// * `id` - The unique identifier of the resource
    /// * `context` - Enhanced request context with tenant information
    ///
    /// # Errors
    /// Returns an error if:
    /// * Resource not found
    /// * Tenant validation fails
    /// * Permission validation fails
    /// * Delete operation fails
    fn delete_resource(
        &self,
        tenant_id: &str,
        resource_type: &str,
        id: &str,
        context: &EnhancedRequestContext,
    ) -> impl Future<Output = Result<(), Self::Error>> + Send;

    /// List resources from the specified tenant.
    ///
    /// # Arguments
    /// * `tenant_id` - The tenant identifier for isolation
    /// * `resource_type` - The type of resources to list
    /// * `query` - Optional query parameters for filtering and pagination
    /// * `context` - Enhanced request context with tenant information
    ///
    /// # Returns
    /// A vector of resources matching the criteria
    ///
    /// # Errors
    /// Returns an error if:
    /// * Tenant validation fails
    /// * Permission validation fails
    /// * Query execution fails
    fn list_resources(
        &self,
        tenant_id: &str,
        resource_type: &str,
        query: Option<&ListQuery>,
        context: &EnhancedRequestContext,
    ) -> impl Future<Output = Result<Vec<Resource>, Self::Error>> + Send;

    /// Find a resource by attribute value in the specified tenant.
    ///
    /// # Arguments
    /// * `tenant_id` - The tenant identifier for isolation
    /// * `resource_type` - The type of resource to search
    /// * `attribute` - The attribute name to search by
    /// * `value` - The attribute value to match
    /// * `context` - Enhanced request context with tenant information
    ///
    /// # Returns
    /// The first resource matching the criteria, if any
    ///
    /// # Errors
    /// Returns an error if:
    /// * Tenant validation fails
    /// * Permission validation fails
    /// * Search operation fails
    fn find_resource_by_attribute(
        &self,
        tenant_id: &str,
        resource_type: &str,
        attribute: &str,
        value: &Value,
        context: &EnhancedRequestContext,
    ) -> impl Future<Output = Result<Option<Resource>, Self::Error>> + Send;

    /// Check if a resource exists in the specified tenant.
    ///
    /// # Arguments
    /// * `tenant_id` - The tenant identifier for isolation
    /// * `resource_type` - The type of resource to check
    /// * `id` - The unique identifier of the resource
    /// * `context` - Enhanced request context with tenant information
    ///
    /// # Returns
    /// True if the resource exists, false otherwise
    ///
    /// # Errors
    /// Returns an error if:
    /// * Tenant validation fails
    /// * Permission validation fails
    /// * Existence check fails
    fn resource_exists(
        &self,
        tenant_id: &str,
        resource_type: &str,
        id: &str,
        context: &EnhancedRequestContext,
    ) -> impl Future<Output = Result<bool, Self::Error>> + Send;

    /// Get the count of resources of a specific type in the tenant.
    ///
    /// This is useful for enforcing tenant limits and providing usage statistics.
    ///
    /// # Arguments
    /// * `tenant_id` - The tenant identifier for isolation
    /// * `resource_type` - The type of resources to count
    /// * `context` - Enhanced request context with tenant information
    ///
    /// # Returns
    /// The number of resources of the specified type
    ///
    /// # Errors
    /// Returns an error if:
    /// * Tenant validation fails
    /// * Permission validation fails
    /// * Count operation fails
    fn get_resource_count(
        &self,
        tenant_id: &str,
        resource_type: &str,
        context: &EnhancedRequestContext,
    ) -> impl Future<Output = Result<usize, Self::Error>> + Send;
}

/// Helper trait for validating tenant context in provider operations.
///
/// This trait provides common validation logic that can be reused across
/// different multi-tenant provider implementations.
pub trait TenantValidator {
    /// Validate that the context matches the requested tenant.
    fn validate_tenant_context(
        &self,
        tenant_id: &str,
        context: &EnhancedRequestContext,
    ) -> Result<(), String> {
        if context.tenant_id() != tenant_id {
            return Err(format!(
                "Tenant mismatch: context has '{}', operation requested '{}'",
                context.tenant_id(),
                tenant_id
            ));
        }
        Ok(())
    }

    /// Validate that the tenant has permission for the operation.
    fn validate_operation_permission(
        &self,
        operation: &str,
        context: &EnhancedRequestContext,
    ) -> Result<(), String> {
        context.validate_operation(operation)
    }

    /// Validate tenant limits for resource creation.
    fn validate_tenant_limits(
        &self,
        resource_type: &str,
        current_count: usize,
        context: &EnhancedRequestContext,
    ) -> Result<(), String> {
        let can_create = match resource_type {
            "User" => context.tenant_context.check_user_limit(current_count),
            "Group" => context.tenant_context.check_group_limit(current_count),
            _ => true, // No limits for other resource types
        };

        if !can_create {
            Err(format!(
                "Tenant '{}' has reached the limit for {} resources",
                context.tenant_id(),
                resource_type
            ))
        } else {
            Ok(())
        }
    }
}

/// Default implementation of TenantValidator for any type.
impl<T> TenantValidator for T {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::resource::{TenantContext, TenantPermissions};

    struct MockValidator;

    #[test]
    fn test_tenant_validator_success() {
        let validator = MockValidator;
        let tenant_context = TenantContext::new("test-tenant".to_string(), "client".to_string());
        let context = EnhancedRequestContext::with_generated_id(tenant_context);

        // Should succeed with matching tenant
        assert!(
            validator
                .validate_tenant_context("test-tenant", &context)
                .is_ok()
        );

        // Should succeed with valid operation
        assert!(
            validator
                .validate_operation_permission("read", &context)
                .is_ok()
        );
    }

    #[test]
    fn test_tenant_validator_failure() {
        let validator = MockValidator;
        let tenant_context = TenantContext::new("test-tenant".to_string(), "client".to_string());
        let context = EnhancedRequestContext::with_generated_id(tenant_context);

        // Should fail with mismatched tenant
        let result = validator.validate_tenant_context("different-tenant", &context);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Tenant mismatch"));
    }

    #[test]
    fn test_tenant_limits_validation() {
        let validator = MockValidator;

        // Test with no limits
        let tenant_context = TenantContext::new("test".to_string(), "client".to_string());
        let context = EnhancedRequestContext::with_generated_id(tenant_context);

        assert!(
            validator
                .validate_tenant_limits("User", 100, &context)
                .is_ok()
        );

        // Test with limits
        let mut permissions = TenantPermissions::default();
        permissions.max_users = Some(5);
        let tenant_context = TenantContext::new("test".to_string(), "client".to_string())
            .with_permissions(permissions);
        let context = EnhancedRequestContext::with_generated_id(tenant_context);

        assert!(
            validator
                .validate_tenant_limits("User", 3, &context)
                .is_ok()
        );
        assert!(
            validator
                .validate_tenant_limits("User", 5, &context)
                .is_err()
        );
    }

    #[test]
    fn test_operation_permission_validation() {
        let validator = MockValidator;

        // Test with restricted permissions
        let mut permissions = TenantPermissions::default();
        permissions.can_delete = false;
        let tenant_context = TenantContext::new("test".to_string(), "client".to_string())
            .with_permissions(permissions);
        let context = EnhancedRequestContext::with_generated_id(tenant_context);

        assert!(
            validator
                .validate_operation_permission("read", &context)
                .is_ok()
        );
        assert!(
            validator
                .validate_operation_permission("delete", &context)
                .is_err()
        );
    }
}
