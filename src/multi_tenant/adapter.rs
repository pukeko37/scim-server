//! Provider adapter for bridging single-tenant and multi-tenant providers.
//!
//! This module provides adapters that allow existing single-tenant `ResourceProvider`
//! implementations to work seamlessly with the multi-tenant system. This enables
//! gradual migration and backward compatibility.

use crate::multi_tenant::provider::{MultiTenantResourceProvider, TenantValidator};
use crate::resource::{
    EnhancedRequestContext, ListQuery, RequestContext, Resource, ResourceProvider,
};
use serde_json::Value;
use std::marker::PhantomData;
use std::sync::Arc;

/// Adapter that wraps a single-tenant ResourceProvider to work with multi-tenant operations.
///
/// This adapter allows existing single-tenant providers to be used in a multi-tenant
/// context by:
/// * Converting multi-tenant requests to single-tenant format
/// * Adding tenant validation and isolation
/// * Preserving the original provider's behavior
///
/// # Example Usage
///
/// ```rust,no_run
/// use scim_server::multi_tenant::SingleTenantAdapter;
/// use scim_server::{ResourceProvider, TenantContext, EnhancedRequestContext};
/// use serde_json::Value;
/// use std::sync::Arc;
///
/// // Your existing single-tenant provider
/// struct MyProvider;
///
/// #[derive(Debug, thiserror::Error)]
/// #[error("Provider error")]
/// struct MyError;
///
/// impl ResourceProvider for MyProvider {
///     type Error = MyError;
///     // ... implement methods
/// #   async fn create_resource(&self, resource_type: &str, data: Value, context: &scim_server::RequestContext) -> Result<scim_server::Resource, Self::Error> { unimplemented!() }
/// #   async fn get_resource(&self, resource_type: &str, id: &str, context: &scim_server::RequestContext) -> Result<Option<scim_server::Resource>, Self::Error> { unimplemented!() }
/// #   async fn update_resource(&self, resource_type: &str, id: &str, data: Value, context: &scim_server::RequestContext) -> Result<scim_server::Resource, Self::Error> { unimplemented!() }
/// #   async fn delete_resource(&self, resource_type: &str, id: &str, context: &scim_server::RequestContext) -> Result<(), Self::Error> { unimplemented!() }
/// #   async fn list_resources(&self, resource_type: &str, query: Option<&scim_server::ListQuery>, context: &scim_server::RequestContext) -> Result<Vec<scim_server::Resource>, Self::Error> { unimplemented!() }
/// #   async fn find_resource_by_attribute(&self, resource_type: &str, attribute: &str, value: &Value, context: &scim_server::RequestContext) -> Result<Option<scim_server::Resource>, Self::Error> { unimplemented!() }
/// #   async fn resource_exists(&self, resource_type: &str, id: &str, context: &scim_server::RequestContext) -> Result<bool, Self::Error> { unimplemented!() }
/// }
///
/// // Wrap it for multi-tenant use
/// let provider = Arc::new(MyProvider);
/// let multi_tenant_provider = SingleTenantAdapter::new(provider);
///
/// // Now it can be used with multi-tenant operations
/// # async fn example(context: EnhancedRequestContext) -> Result<(), Box<dyn std::error::Error>> {
/// let user_data = serde_json::json!({"userName": "test"});
/// let result = multi_tenant_provider.create_resource(
///     "tenant-a",
///     "User",
///     user_data,
///     &context
/// ).await?;
/// # Ok(())
/// # }
/// ```
pub struct SingleTenantAdapter<P> {
    provider: Arc<P>,
}

impl<P> SingleTenantAdapter<P>
where
    P: ResourceProvider + Send + Sync,
{
    /// Create a new adapter wrapping the given single-tenant provider.
    pub fn new(provider: Arc<P>) -> Self {
        Self { provider }
    }

    /// Get a reference to the underlying provider.
    pub fn inner(&self) -> &P {
        &self.provider
    }

    /// Convert an enhanced request context to a regular request context.
    fn to_single_tenant_context(&self, context: &EnhancedRequestContext) -> RequestContext {
        RequestContext::with_tenant(context.request_id.clone(), context.tenant_context.clone())
    }
}

impl<P> Clone for SingleTenantAdapter<P> {
    fn clone(&self) -> Self {
        Self {
            provider: Arc::clone(&self.provider),
        }
    }
}

/// Error wrapper for single-tenant adapter operations
#[derive(Debug, thiserror::Error)]
pub enum AdapterError<E> {
    #[error("Tenant validation failed: {message}")]
    TenantValidation { message: String },
    #[error("Permission denied: {message}")]
    PermissionDenied { message: String },
    #[error("Provider error: {0}")]
    Provider(#[from] E),
}

impl<P> MultiTenantResourceProvider for SingleTenantAdapter<P>
where
    P: ResourceProvider + Send + Sync,
    P::Error: Send + Sync + 'static,
{
    type Error = AdapterError<P::Error>;

    async fn create_resource(
        &self,
        tenant_id: &str,
        resource_type: &str,
        data: Value,
        context: &EnhancedRequestContext,
    ) -> Result<Resource, Self::Error> {
        // Validate tenant context
        self.validate_tenant_context(tenant_id, context)
            .map_err(|msg| AdapterError::TenantValidation { message: msg })?;

        // Validate operation permission
        self.validate_operation_permission("create", context)
            .map_err(|msg| AdapterError::PermissionDenied { message: msg })?;

        // Convert context and delegate to single-tenant provider
        let single_context = self.to_single_tenant_context(context);
        let resource = self
            .provider
            .create_resource(resource_type, data, &single_context)
            .await?;

        Ok(resource)
    }

    async fn get_resource(
        &self,
        tenant_id: &str,
        resource_type: &str,
        id: &str,
        context: &EnhancedRequestContext,
    ) -> Result<Option<Resource>, Self::Error> {
        // Validate tenant context
        self.validate_tenant_context(tenant_id, context)
            .map_err(|msg| AdapterError::TenantValidation { message: msg })?;

        // Validate operation permission
        self.validate_operation_permission("read", context)
            .map_err(|msg| AdapterError::PermissionDenied { message: msg })?;

        // Convert context and delegate to single-tenant provider
        let single_context = self.to_single_tenant_context(context);
        let resource = self
            .provider
            .get_resource(resource_type, id, &single_context)
            .await?;

        Ok(resource)
    }

    async fn update_resource(
        &self,
        tenant_id: &str,
        resource_type: &str,
        id: &str,
        data: Value,
        context: &EnhancedRequestContext,
    ) -> Result<Resource, Self::Error> {
        // Validate tenant context
        self.validate_tenant_context(tenant_id, context)
            .map_err(|msg| AdapterError::TenantValidation { message: msg })?;

        // Validate operation permission
        self.validate_operation_permission("update", context)
            .map_err(|msg| AdapterError::PermissionDenied { message: msg })?;

        // Convert context and delegate to single-tenant provider
        let single_context = self.to_single_tenant_context(context);
        let resource = self
            .provider
            .update_resource(resource_type, id, data, &single_context)
            .await?;

        Ok(resource)
    }

    async fn delete_resource(
        &self,
        tenant_id: &str,
        resource_type: &str,
        id: &str,
        context: &EnhancedRequestContext,
    ) -> Result<(), Self::Error> {
        // Validate tenant context
        self.validate_tenant_context(tenant_id, context)
            .map_err(|msg| AdapterError::TenantValidation { message: msg })?;

        // Validate operation permission
        self.validate_operation_permission("delete", context)
            .map_err(|msg| AdapterError::PermissionDenied { message: msg })?;

        // Convert context and delegate to single-tenant provider
        let single_context = self.to_single_tenant_context(context);
        self.provider
            .delete_resource(resource_type, id, &single_context)
            .await?;

        Ok(())
    }

    async fn list_resources(
        &self,
        tenant_id: &str,
        resource_type: &str,
        query: Option<&ListQuery>,
        context: &EnhancedRequestContext,
    ) -> Result<Vec<Resource>, Self::Error> {
        // Validate tenant context
        self.validate_tenant_context(tenant_id, context)
            .map_err(|msg| AdapterError::TenantValidation { message: msg })?;

        // Validate operation permission
        self.validate_operation_permission("list", context)
            .map_err(|msg| AdapterError::PermissionDenied { message: msg })?;

        // Convert context and delegate to single-tenant provider
        let single_context = self.to_single_tenant_context(context);
        let resources = self
            .provider
            .list_resources(resource_type, query, &single_context)
            .await?;

        Ok(resources)
    }

    async fn find_resource_by_attribute(
        &self,
        tenant_id: &str,
        resource_type: &str,
        attribute: &str,
        value: &Value,
        context: &EnhancedRequestContext,
    ) -> Result<Option<Resource>, Self::Error> {
        // Validate tenant context
        self.validate_tenant_context(tenant_id, context)
            .map_err(|msg| AdapterError::TenantValidation { message: msg })?;

        // Validate operation permission
        self.validate_operation_permission("read", context)
            .map_err(|msg| AdapterError::PermissionDenied { message: msg })?;

        // Convert context and delegate to single-tenant provider
        let single_context = self.to_single_tenant_context(context);
        let resource = self
            .provider
            .find_resource_by_attribute(resource_type, attribute, value, &single_context)
            .await?;

        Ok(resource)
    }

    async fn resource_exists(
        &self,
        tenant_id: &str,
        resource_type: &str,
        id: &str,
        context: &EnhancedRequestContext,
    ) -> Result<bool, Self::Error> {
        // Validate tenant context
        self.validate_tenant_context(tenant_id, context)
            .map_err(|msg| AdapterError::TenantValidation { message: msg })?;

        // Validate operation permission
        self.validate_operation_permission("read", context)
            .map_err(|msg| AdapterError::PermissionDenied { message: msg })?;

        // Convert context and delegate to single-tenant provider
        let single_context = self.to_single_tenant_context(context);
        let exists = self
            .provider
            .resource_exists(resource_type, id, &single_context)
            .await?;

        Ok(exists)
    }

    async fn get_resource_count(
        &self,
        tenant_id: &str,
        resource_type: &str,
        context: &EnhancedRequestContext,
    ) -> Result<usize, Self::Error> {
        // Validate tenant context
        self.validate_tenant_context(tenant_id, context)
            .map_err(|msg| AdapterError::TenantValidation { message: msg })?;

        // Validate operation permission
        self.validate_operation_permission("read", context)
            .map_err(|msg| AdapterError::PermissionDenied { message: msg })?;

        // For single-tenant providers, we need to list all resources and count them
        // This is not the most efficient, but it's a reasonable fallback
        let single_context = self.to_single_tenant_context(context);
        let resources = self
            .provider
            .list_resources(resource_type, None, &single_context)
            .await?;

        Ok(resources.len())
    }
}

/// Utility trait for converting multi-tenant providers to work with single-tenant interfaces.
///
/// This is the reverse adapter - it allows multi-tenant providers to be used in contexts
/// that expect single-tenant providers, but requires a default tenant to be specified.
pub trait ToSingleTenant<P> {
    /// Convert to a single-tenant interface using the specified default tenant.
    fn with_default_tenant(self, default_tenant: &str) -> MultiTenantToSingleAdapter<P>;
}

impl<P> ToSingleTenant<P> for P
where
    P: MultiTenantResourceProvider,
{
    fn with_default_tenant(self, default_tenant: &str) -> MultiTenantToSingleAdapter<P> {
        MultiTenantToSingleAdapter::new(self, default_tenant.to_string())
    }
}

/// Adapter that wraps a multi-tenant provider to work with single-tenant interfaces.
///
/// This adapter allows multi-tenant providers to be used in single-tenant contexts
/// by automatically supplying a default tenant ID for all operations.
pub struct MultiTenantToSingleAdapter<P> {
    provider: P,
    default_tenant: String,
    _phantom: PhantomData<P>,
}

impl<P> MultiTenantToSingleAdapter<P>
where
    P: MultiTenantResourceProvider,
{
    /// Create a new adapter with the specified default tenant.
    pub fn new(provider: P, default_tenant: String) -> Self {
        Self {
            provider,
            default_tenant,
            _phantom: PhantomData,
        }
    }

    /// Convert a single-tenant context to an enhanced context for the default tenant.
    fn to_multi_tenant_context(
        &self,
        context: &RequestContext,
    ) -> Result<EnhancedRequestContext, String> {
        match context.tenant_context.as_ref() {
            Some(tenant_context) => Ok(EnhancedRequestContext::new(
                context.request_id.clone(),
                tenant_context.clone(),
            )),
            None => Err(
                "RequestContext must contain tenant information for multi-tenant operations"
                    .to_string(),
            ),
        }
    }
}

impl<P> ResourceProvider for MultiTenantToSingleAdapter<P>
where
    P: MultiTenantResourceProvider + Send + Sync,
    P::Error: Send + Sync + 'static,
{
    type Error = AdapterError<P::Error>;

    async fn create_resource(
        &self,
        resource_type: &str,
        data: Value,
        context: &RequestContext,
    ) -> Result<Resource, Self::Error> {
        let enhanced_context = self
            .to_multi_tenant_context(context)
            .map_err(|msg| AdapterError::TenantValidation { message: msg })?;

        self.provider
            .create_resource(&self.default_tenant, resource_type, data, &enhanced_context)
            .await
            .map_err(AdapterError::Provider)
    }

    async fn get_resource(
        &self,
        resource_type: &str,
        id: &str,
        context: &RequestContext,
    ) -> Result<Option<Resource>, Self::Error> {
        let enhanced_context = self
            .to_multi_tenant_context(context)
            .map_err(|msg| AdapterError::TenantValidation { message: msg })?;

        self.provider
            .get_resource(&self.default_tenant, resource_type, id, &enhanced_context)
            .await
            .map_err(AdapterError::Provider)
    }

    async fn update_resource(
        &self,
        resource_type: &str,
        id: &str,
        data: Value,
        context: &RequestContext,
    ) -> Result<Resource, Self::Error> {
        let enhanced_context = self
            .to_multi_tenant_context(context)
            .map_err(|msg| AdapterError::TenantValidation { message: msg })?;

        self.provider
            .update_resource(
                &self.default_tenant,
                resource_type,
                id,
                data,
                &enhanced_context,
            )
            .await
            .map_err(AdapterError::Provider)
    }

    async fn delete_resource(
        &self,
        resource_type: &str,
        id: &str,
        context: &RequestContext,
    ) -> Result<(), Self::Error> {
        let enhanced_context = self
            .to_multi_tenant_context(context)
            .map_err(|msg| AdapterError::TenantValidation { message: msg })?;

        self.provider
            .delete_resource(&self.default_tenant, resource_type, id, &enhanced_context)
            .await
            .map_err(AdapterError::Provider)
    }

    async fn list_resources(
        &self,
        resource_type: &str,
        query: Option<&ListQuery>,
        context: &RequestContext,
    ) -> Result<Vec<Resource>, Self::Error> {
        let enhanced_context = self
            .to_multi_tenant_context(context)
            .map_err(|msg| AdapterError::TenantValidation { message: msg })?;

        self.provider
            .list_resources(
                &self.default_tenant,
                resource_type,
                query,
                &enhanced_context,
            )
            .await
            .map_err(AdapterError::Provider)
    }

    async fn find_resource_by_attribute(
        &self,
        resource_type: &str,
        attribute: &str,
        value: &Value,
        context: &RequestContext,
    ) -> Result<Option<Resource>, Self::Error> {
        let enhanced_context = self
            .to_multi_tenant_context(context)
            .map_err(|msg| AdapterError::TenantValidation { message: msg })?;

        self.provider
            .find_resource_by_attribute(
                &self.default_tenant,
                resource_type,
                attribute,
                value,
                &enhanced_context,
            )
            .await
            .map_err(AdapterError::Provider)
    }

    async fn resource_exists(
        &self,
        resource_type: &str,
        id: &str,
        context: &RequestContext,
    ) -> Result<bool, Self::Error> {
        let enhanced_context = self
            .to_multi_tenant_context(context)
            .map_err(|msg| AdapterError::TenantValidation { message: msg })?;

        self.provider
            .resource_exists(&self.default_tenant, resource_type, id, &enhanced_context)
            .await
            .map_err(AdapterError::Provider)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::resource::{IsolationLevel, TenantContext, TenantPermissions};
    use serde_json::json;
    use std::collections::HashMap;
    use tokio::sync::RwLock;

    // Mock single-tenant provider for testing
    struct MockProvider {
        resources: Arc<RwLock<HashMap<String, HashMap<String, Resource>>>>,
    }

    impl MockProvider {
        fn new() -> Self {
            Self {
                resources: Arc::new(RwLock::new(HashMap::new())),
            }
        }
    }

    #[derive(Debug, thiserror::Error)]
    #[error("Mock error")]
    struct MockError;

    impl ResourceProvider for MockProvider {
        type Error = MockError;

        async fn create_resource(
            &self,
            resource_type: &str,
            data: Value,
            _context: &RequestContext,
        ) -> Result<Resource, Self::Error> {
            let resource = Resource::new(resource_type.to_string(), data);
            let id = resource.get_id().unwrap_or("generated-id").to_string();

            let mut resources = self.resources.write().await;
            resources
                .entry(resource_type.to_string())
                .or_insert_with(HashMap::new)
                .insert(id, resource.clone());

            Ok(resource)
        }

        async fn get_resource(
            &self,
            resource_type: &str,
            id: &str,
            _context: &RequestContext,
        ) -> Result<Option<Resource>, Self::Error> {
            let resources = self.resources.read().await;
            Ok(resources
                .get(resource_type)
                .and_then(|type_resources| type_resources.get(id))
                .cloned())
        }

        async fn update_resource(
            &self,
            resource_type: &str,
            id: &str,
            data: Value,
            _context: &RequestContext,
        ) -> Result<Resource, Self::Error> {
            let resource = Resource::new(resource_type.to_string(), data);
            let mut resources = self.resources.write().await;
            resources
                .entry(resource_type.to_string())
                .or_insert_with(HashMap::new)
                .insert(id.to_string(), resource.clone());

            Ok(resource)
        }

        async fn delete_resource(
            &self,
            resource_type: &str,
            id: &str,
            _context: &RequestContext,
        ) -> Result<(), Self::Error> {
            let mut resources = self.resources.write().await;
            if let Some(type_resources) = resources.get_mut(resource_type) {
                type_resources.remove(id);
            }
            Ok(())
        }

        async fn list_resources(
            &self,
            resource_type: &str,
            _query: Option<&ListQuery>,
            _context: &RequestContext,
        ) -> Result<Vec<Resource>, Self::Error> {
            let resources = self.resources.read().await;
            Ok(resources
                .get(resource_type)
                .map(|type_resources| type_resources.values().cloned().collect())
                .unwrap_or_default())
        }

        async fn find_resource_by_attribute(
            &self,
            resource_type: &str,
            attribute: &str,
            value: &Value,
            _context: &RequestContext,
        ) -> Result<Option<Resource>, Self::Error> {
            let resources = self.resources.read().await;
            Ok(resources
                .get(resource_type)
                .and_then(|type_resources| {
                    type_resources
                        .values()
                        .find(|resource| resource.get_attribute(attribute) == Some(value))
                })
                .cloned())
        }

        async fn resource_exists(
            &self,
            resource_type: &str,
            id: &str,
            _context: &RequestContext,
        ) -> Result<bool, Self::Error> {
            let resources = self.resources.read().await;
            Ok(resources
                .get(resource_type)
                .map(|type_resources| type_resources.contains_key(id))
                .unwrap_or(false))
        }
    }

    #[tokio::test]
    async fn test_single_tenant_adapter_basic_operations() {
        let provider = Arc::new(MockProvider::new());
        let adapter = SingleTenantAdapter::new(provider);

        let tenant_context = TenantContext::new("test-tenant".to_string(), "client".to_string());
        let context = EnhancedRequestContext::with_generated_id(tenant_context);

        // Test create
        let user_data = json!({"id": "user1", "userName": "testuser"});
        let user = adapter
            .create_resource("test-tenant", "User", user_data, &context)
            .await
            .unwrap();
        assert_eq!(user.get_username(), Some("testuser"));

        // Test get
        let retrieved = adapter
            .get_resource("test-tenant", "User", "user1", &context)
            .await
            .unwrap();
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().get_username(), Some("testuser"));

        // Test exists
        let exists = adapter
            .resource_exists("test-tenant", "User", "user1", &context)
            .await
            .unwrap();
        assert!(exists);
    }

    #[tokio::test]
    async fn test_single_tenant_adapter_tenant_validation() {
        let provider = Arc::new(MockProvider::new());
        let adapter = SingleTenantAdapter::new(provider);

        let tenant_context = TenantContext::new("tenant-a".to_string(), "client".to_string());
        let context = EnhancedRequestContext::with_generated_id(tenant_context);

        // Should fail with mismatched tenant
        let user_data = json!({"userName": "testuser"});
        let result = adapter
            .create_resource("tenant-b", "User", user_data, &context)
            .await;

        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            AdapterError::TenantValidation { .. }
        ));
    }

    #[tokio::test]
    async fn test_single_tenant_adapter_permission_validation() {
        let provider = Arc::new(MockProvider::new());
        let adapter = SingleTenantAdapter::new(provider);

        let mut permissions = TenantPermissions::default();
        permissions.can_create = false;

        let tenant_context = TenantContext::new("test-tenant".to_string(), "client".to_string())
            .with_permissions(permissions);
        let context = EnhancedRequestContext::with_generated_id(tenant_context);

        // Should fail due to permission restriction
        let user_data = json!({"userName": "testuser"});
        let result = adapter
            .create_resource("test-tenant", "User", user_data, &context)
            .await;

        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            AdapterError::PermissionDenied { .. }
        ));
    }

    #[tokio::test]
    async fn test_single_tenant_adapter_resource_count() {
        let provider = Arc::new(MockProvider::new());
        let adapter = SingleTenantAdapter::new(provider);

        let tenant_context = TenantContext::new("test-tenant".to_string(), "client".to_string());
        let context = EnhancedRequestContext::with_generated_id(tenant_context);

        // Initially should be 0
        let count = adapter
            .get_resource_count("test-tenant", "User", &context)
            .await
            .unwrap();
        assert_eq!(count, 0);

        // Create a user
        let user_data = json!({"id": "user1", "userName": "testuser"});
        adapter
            .create_resource("test-tenant", "User", user_data, &context)
            .await
            .unwrap();

        // Count should be 1
        let count = adapter
            .get_resource_count("test-tenant", "User", &context)
            .await
            .unwrap();
        assert_eq!(count, 1);
    }
}
