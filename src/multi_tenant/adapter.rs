//! Provider adapter utilities for the unified ResourceProvider trait.
//!
//! This module provides utilities for working with the unified ResourceProvider trait
//! that supports both single and multi-tenant operations through the RequestContext.
//!
//! Since the ResourceProvider is now unified, these are primarily validation and
//! convenience utilities rather than true adapters.

use crate::providers::ResourceProvider;
use crate::resource::version::RawVersion;
use crate::resource::{ListQuery, RequestContext, TenantContext, versioned::VersionedResource};
use serde_json::Value;
use std::future::Future;

/// Error types for adapter operations.
#[derive(Debug, thiserror::Error)]
pub enum AdapterError<E> {
    /// Error from the underlying provider
    #[error("Provider error: {0}")]
    Provider(#[source] E),

    /// Tenant validation error
    #[error("Tenant validation error: {message}")]
    TenantValidation { message: String },

    /// Context conversion error
    #[error("Context conversion error: {message}")]
    ContextConversion { message: String },
}

/// Validation wrapper that ensures tenant context is properly handled.
///
/// This wrapper validates tenant contexts and provides clear error messages
/// when operations are performed with incorrect tenant contexts.
pub struct TenantValidatingProvider<P> {
    inner: P,
}

impl<P> TenantValidatingProvider<P> {
    /// Create a new validating provider wrapper.
    pub fn new(provider: P) -> Self {
        Self { inner: provider }
    }

    /// Get reference to the inner provider.
    pub fn inner(&self) -> &P {
        &self.inner
    }

    /// Consume wrapper and return inner provider.
    pub fn into_inner(self) -> P {
        self.inner
    }
}

impl<P> ResourceProvider for TenantValidatingProvider<P>
where
    P: ResourceProvider + Send + Sync,
    P::Error: Send + Sync + 'static,
{
    type Error = AdapterError<P::Error>;

    fn create_resource(
        &self,
        resource_type: &str,
        data: Value,
        context: &RequestContext,
    ) -> impl Future<Output = Result<VersionedResource, Self::Error>> + Send {
        async move {
            // Validate context consistency
            self.validate_context_consistency(context)?;

            self.inner
                .create_resource(resource_type, data, context)
                .await
                .map_err(AdapterError::Provider)
        }
    }

    fn get_resource(
        &self,
        resource_type: &str,
        id: &str,
        context: &RequestContext,
    ) -> impl Future<Output = Result<Option<VersionedResource>, Self::Error>> + Send {
        async move {
            self.validate_context_consistency(context)?;

            self.inner
                .get_resource(resource_type, id, context)
                .await
                .map_err(AdapterError::Provider)
        }
    }

    fn update_resource(
        &self,
        resource_type: &str,
        id: &str,
        data: Value,
        expected_version: Option<&RawVersion>,
        context: &RequestContext,
    ) -> impl Future<Output = Result<VersionedResource, Self::Error>> + Send {
        async move {
            self.validate_context_consistency(context)?;

            self.inner
                .update_resource(resource_type, id, data, expected_version, context)
                .await
                .map_err(AdapterError::Provider)
        }
    }

    fn delete_resource(
        &self,
        resource_type: &str,
        id: &str,
        expected_version: Option<&RawVersion>,
        context: &RequestContext,
    ) -> impl Future<Output = Result<(), Self::Error>> + Send {
        async move {
            self.validate_context_consistency(context)?;

            self.inner
                .delete_resource(resource_type, id, expected_version, context)
                .await
                .map_err(AdapterError::Provider)
        }
    }

    fn list_resources(
        &self,
        resource_type: &str,
        query: Option<&ListQuery>,
        context: &RequestContext,
    ) -> impl Future<Output = Result<Vec<VersionedResource>, Self::Error>> + Send {
        async move {
            self.validate_context_consistency(context)?;

            self.inner
                .list_resources(resource_type, query, context)
                .await
                .map_err(AdapterError::Provider)
        }
    }

    fn find_resources_by_attribute(
        &self,
        resource_type: &str,
        attribute_name: &str,
        attribute_value: &str,
        context: &RequestContext,
    ) -> impl Future<Output = Result<Vec<VersionedResource>, Self::Error>> + Send {
        async move {
            self.validate_context_consistency(context)?;

            self.inner
                .find_resources_by_attribute(
                    resource_type,
                    attribute_name,
                    attribute_value,
                    context,
                )
                .await
                .map_err(AdapterError::Provider)
        }
    }

    fn patch_resource(
        &self,
        resource_type: &str,
        id: &str,
        patch_request: &Value,
        expected_version: Option<&RawVersion>,
        context: &RequestContext,
    ) -> impl Future<Output = Result<VersionedResource, Self::Error>> + Send {
        async move {
            self.validate_context_consistency(context)?;

            self.inner
                .patch_resource(resource_type, id, patch_request, expected_version, context)
                .await
                .map_err(AdapterError::Provider)
        }
    }

    fn resource_exists(
        &self,
        resource_type: &str,
        id: &str,
        context: &RequestContext,
    ) -> impl Future<Output = Result<bool, Self::Error>> + Send {
        async move {
            self.validate_context_consistency(context)?;

            self.inner
                .resource_exists(resource_type, id, context)
                .await
                .map_err(AdapterError::Provider)
        }
    }
}

// TenantValidator is implemented via blanket impl

impl<P> TenantValidatingProvider<P>
where
    P: ResourceProvider,
{
    /// Validate that the request context is internally consistent.
    fn validate_context_consistency(
        &self,
        context: &RequestContext,
    ) -> Result<(), AdapterError<P::Error>> {
        // Ensure request ID is not empty
        if context.request_id.trim().is_empty() {
            return Err(AdapterError::ContextConversion {
                message: "Request ID cannot be empty".to_string(),
            });
        }

        // Validate tenant context if present
        if let Some(tenant_context) = &context.tenant_context {
            if tenant_context.tenant_id.trim().is_empty() {
                return Err(AdapterError::TenantValidation {
                    message: "Tenant ID cannot be empty".to_string(),
                });
            }
        }

        Ok(())
    }
}

/// Trait for converting providers to single-tenant mode (legacy compatibility).
///
/// Since ResourceProvider is now unified, this is mainly for API compatibility.
pub trait ToSingleTenant<P> {
    /// Convert to a provider that validates single-tenant contexts.
    fn to_single_tenant(self) -> TenantValidatingProvider<P>;
}

impl<P> ToSingleTenant<P> for P
where
    P: ResourceProvider,
{
    fn to_single_tenant(self) -> TenantValidatingProvider<P> {
        TenantValidatingProvider::new(self)
    }
}

/// Legacy type alias for backward compatibility.
///
/// Note: With the unified ResourceProvider, this is now just a validation wrapper.
pub type SingleTenantAdapter<P> = TenantValidatingProvider<P>;

/// Context conversion utilities.
pub struct ContextConverter;

impl ContextConverter {
    /// Create a single-tenant RequestContext.
    pub fn single_tenant_context(request_id: Option<String>) -> RequestContext {
        match request_id {
            Some(id) => RequestContext::new(id),
            None => RequestContext::with_generated_id(),
        }
    }

    /// Create a multi-tenant RequestContext.
    pub fn multi_tenant_context(
        tenant_id: String,
        client_id: Option<String>,
        request_id: Option<String>,
    ) -> RequestContext {
        let tenant_context = TenantContext {
            tenant_id,
            client_id: client_id.unwrap_or_else(|| "default-client".to_string()),
            permissions: Default::default(),
            isolation_level: Default::default(),
        };

        match request_id {
            Some(id) => RequestContext::with_tenant(id, tenant_context),
            None => RequestContext::with_tenant_generated_id(tenant_context),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, thiserror::Error)]
    #[error("Mock error")]
    struct MockError;

    struct MockProvider;

    impl ResourceProvider for MockProvider {
        type Error = MockError;

        async fn create_resource(
            &self,
            _resource_type: &str,
            _data: Value,
            _context: &RequestContext,
        ) -> Result<VersionedResource, Self::Error> {
            Err(MockError)
        }

        async fn get_resource(
            &self,
            _resource_type: &str,
            _id: &str,
            _context: &RequestContext,
        ) -> Result<Option<VersionedResource>, Self::Error> {
            Ok(None)
        }

        async fn update_resource(
            &self,
            _resource_type: &str,
            _id: &str,
            _data: Value,
            _expected_version: Option<&RawVersion>,
            _context: &RequestContext,
        ) -> Result<VersionedResource, Self::Error> {
            Err(MockError)
        }

        async fn delete_resource(
            &self,
            _resource_type: &str,
            _id: &str,
            _expected_version: Option<&RawVersion>,
            _context: &RequestContext,
        ) -> Result<(), Self::Error> {
            Ok(())
        }

        async fn list_resources(
            &self,
            _resource_type: &str,
            _query: Option<&ListQuery>,
            _context: &RequestContext,
        ) -> Result<Vec<VersionedResource>, Self::Error> {
            Ok(vec![])
        }

        async fn find_resources_by_attribute(
            &self,
            _resource_type: &str,
            _attribute_name: &str,
            _attribute_value: &str,
            _context: &RequestContext,
        ) -> Result<Vec<VersionedResource>, Self::Error> {
            Ok(vec![])
        }

        async fn patch_resource(
            &self,
            _resource_type: &str,
            _id: &str,
            _patch_request: &Value,
            _expected_version: Option<&RawVersion>,
            _context: &RequestContext,
        ) -> Result<VersionedResource, Self::Error> {
            Err(MockError)
        }

        async fn resource_exists(
            &self,
            _resource_type: &str,
            _id: &str,
            _context: &RequestContext,
        ) -> Result<bool, Self::Error> {
            Ok(false)
        }
    }

    #[tokio::test]
    async fn test_validating_provider() {
        let provider = MockProvider;
        let validating_provider = TenantValidatingProvider::new(provider);

        let context = RequestContext::with_generated_id();
        let result = validating_provider
            .get_resource("User", "123", &context)
            .await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_context_validation() {
        let provider = MockProvider;
        let validating_provider = TenantValidatingProvider::new(provider);

        // Empty request ID should fail
        let context = RequestContext::new("".to_string());
        let result = validating_provider
            .get_resource("User", "123", &context)
            .await;

        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            AdapterError::ContextConversion { .. }
        ));
    }

    #[test]
    fn test_context_converter() {
        // Single-tenant context
        let context = ContextConverter::single_tenant_context(Some("req-123".to_string()));
        assert_eq!(context.request_id, "req-123");
        assert!(context.tenant_context.is_none());

        // Multi-tenant context
        let context = ContextConverter::multi_tenant_context(
            "tenant-1".to_string(),
            Some("client-1".to_string()),
            Some("req-456".to_string()),
        );
        assert_eq!(context.request_id, "req-456");
        assert!(context.tenant_context.is_some());
        assert_eq!(context.tenant_id(), Some("tenant-1"));
    }

    #[test]
    fn test_to_single_tenant_trait() {
        let provider = MockProvider;
        let _validating_provider = provider.to_single_tenant();
    }
}
