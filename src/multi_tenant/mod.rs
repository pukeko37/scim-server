//! Multi-tenant SCIM server capabilities.
//!
//! This module provides the core infrastructure for multi-tenant SCIM operations,
//! including tenant resolution, multi-tenant resource providers, and tenant-aware
//! error handling.
//!
//! # Architecture
//!
//! The multi-tenant system is built around several key concepts:
//!
//! * **Tenant Resolution**: Mapping authentication credentials to tenant contexts
//! * **Multi-Tenant Providers**: Resource providers that understand tenant isolation
//! * **Enhanced Context**: Request contexts that carry tenant information
//! * **Isolation Levels**: Different levels of tenant data separation
//!
//! # Example Usage
//!
//! ```rust,no_run
//! use scim_server::multi_tenant::{
//!     MultiTenantResourceProvider, TenantResolver, StaticTenantResolver
//! };
//! use scim_server::{TenantContext, EnhancedRequestContext, Resource};
//! use serde_json::json;
//!
//! // Set up tenant resolver
//! let mut resolver = StaticTenantResolver::new();
//! resolver.add_tenant("api-key-123", TenantContext::new(
//!     "tenant-a".to_string(),
//!     "client-a".to_string()
//! ));
//!
//! // Use in multi-tenant operations
//! # async fn example(provider: impl MultiTenantResourceProvider) -> Result<(), Box<dyn std::error::Error>> {
//! let tenant_context = resolver.resolve_tenant("api-key-123").await?;
//! let context = EnhancedRequestContext::with_generated_id(tenant_context);
//!
//! let user_data = json!({
//!     "userName": "john.doe",
//!     "displayName": "John Doe"
//! });
//!
//! let user = provider.create_resource("tenant-a", "User", user_data, &context).await?;
//! # Ok(())
//! # }
//! ```

pub mod adapter;
pub mod database;
pub mod provider;
pub mod resolver;

// Re-export key types for convenience
pub use adapter::{SingleTenantAdapter, ToSingleTenant};
pub use database::{DatabaseResourceProvider, InMemoryDatabase};
pub use provider::MultiTenantResourceProvider;
pub use resolver::{StaticTenantResolver, TenantResolver};

// Re-export core types from resource module
pub use crate::resource::{
    EnhancedRequestContext, IsolationLevel, TenantContext, TenantPermissions,
};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::resource::RequestContext;

    #[test]
    fn test_module_exports() {
        // Test that all expected types are accessible
        let _resolver = StaticTenantResolver::new();

        // Test core type re-exports
        let tenant_context = TenantContext::new("test".to_string(), "client".to_string());
        let _enhanced_context = EnhancedRequestContext::with_generated_id(tenant_context);
    }

    #[test]
    fn test_isolation_levels() {
        assert_eq!(IsolationLevel::default(), IsolationLevel::Standard);

        let strict = IsolationLevel::Strict;
        let standard = IsolationLevel::Standard;
        let shared = IsolationLevel::Shared;

        assert_ne!(strict, standard);
        assert_ne!(standard, shared);
        assert_ne!(strict, shared);
    }

    #[test]
    fn test_tenant_permissions() {
        let default_perms = TenantPermissions::default();
        assert!(default_perms.can_create);
        assert!(default_perms.can_read);
        assert!(default_perms.can_update);
        assert!(default_perms.can_delete);
        assert!(default_perms.can_list);
        assert!(default_perms.max_users.is_none());
        assert!(default_perms.max_groups.is_none());
    }

    #[test]
    fn test_tenant_context_operations() {
        let context = TenantContext::new("test-tenant".to_string(), "test-client".to_string())
            .with_isolation_level(IsolationLevel::Strict);

        assert_eq!(context.tenant_id, "test-tenant");
        assert_eq!(context.client_id, "test-client");
        assert_eq!(context.isolation_level, IsolationLevel::Strict);

        assert!(context.can_perform_operation("create"));
        assert!(context.can_perform_operation("read"));
        assert!(!context.can_perform_operation("invalid"));

        assert!(context.check_user_limit(100));
        assert!(context.check_group_limit(50));
    }

    #[test]
    fn test_tenant_context_with_limits() {
        let mut permissions = TenantPermissions::default();
        permissions.max_users = Some(10);
        permissions.max_groups = Some(5);

        let context = TenantContext::new("test".to_string(), "client".to_string())
            .with_permissions(permissions);

        assert!(context.check_user_limit(5));
        assert!(!context.check_user_limit(10));
        assert!(context.check_group_limit(3));
        assert!(!context.check_group_limit(5));
    }

    #[test]
    fn test_enhanced_request_context() {
        let tenant_context = TenantContext::new("test".to_string(), "client".to_string());
        let enhanced = EnhancedRequestContext::with_generated_id(tenant_context.clone());

        assert_eq!(enhanced.tenant_id(), "test");
        assert_eq!(enhanced.client_id(), "client");
        assert_eq!(enhanced.isolation_level(), &IsolationLevel::Standard);
        assert!(enhanced.can_perform_operation("read"));
        assert!(enhanced.validate_operation("create").is_ok());

        let regular_context = enhanced.to_request_context();
        assert!(regular_context.is_multi_tenant());
        assert_eq!(regular_context.tenant_id(), Some("test"));
    }

    #[test]
    fn test_request_context_conversion() {
        let tenant_context = TenantContext::new("test".to_string(), "client".to_string());
        let regular = RequestContext::with_tenant("req-123".to_string(), tenant_context);

        let enhanced: Result<EnhancedRequestContext, _> = regular.try_into();
        assert!(enhanced.is_ok());

        let enhanced = enhanced.unwrap();
        assert_eq!(enhanced.tenant_id(), "test");
        assert_eq!(enhanced.request_id, "req-123");
    }

    #[test]
    fn test_request_context_conversion_failure() {
        let regular = RequestContext::new("req-123".to_string());
        let enhanced: Result<EnhancedRequestContext, _> = regular.try_into();
        assert!(enhanced.is_err());
        assert!(
            enhanced
                .unwrap_err()
                .contains("does not contain tenant information")
        );
    }
}
