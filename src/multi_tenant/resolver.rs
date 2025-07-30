//! Tenant resolution for multi-tenant SCIM operations.
//!
//! This module provides traits and implementations for resolving tenant contexts
//! from authentication credentials. This is a critical component for multi-tenant
//! security as it maps incoming requests to the appropriate tenant context.

use crate::resource::TenantContext;
use std::collections::HashMap;
use std::future::Future;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Trait for resolving tenant contexts from authentication credentials.
///
/// Implementations of this trait are responsible for mapping authentication
/// information (such as API keys, JWT tokens, or other credentials) to
/// tenant contexts that define the scope and permissions for operations.
///
/// # Security Considerations
///
/// * Always validate credentials before returning tenant context
/// * Implement rate limiting to prevent brute force attacks
/// * Log authentication attempts for audit purposes
/// * Use secure credential storage and comparison
///
/// # Example Implementation
///
/// ```rust,no_run
/// use scim_server::multi_tenant::TenantResolver;
/// use scim_server::TenantContext;
/// use std::collections::HashMap;
///
/// struct DatabaseTenantResolver {
///     // In a real implementation, this would be a database connection
///     credentials: HashMap<String, TenantContext>,
/// }
///
/// impl TenantResolver for DatabaseTenantResolver {
///     type Error = String;
///
///     async fn resolve_tenant(&self, credential: &str) -> Result<TenantContext, Self::Error> {
///         self.credentials
///             .get(credential)
///             .cloned()
///             .ok_or_else(|| "Invalid credentials".to_string())
///     }
///
///     async fn validate_tenant(&self, tenant_id: &str) -> Result<bool, Self::Error> {
///         Ok(self.credentials.values().any(|ctx| ctx.tenant_id == tenant_id))
///     }
/// }
/// ```
pub trait TenantResolver: Send + Sync {
    /// Error type for resolver operations
    type Error: std::error::Error + Send + Sync + 'static;

    /// Resolve a tenant context from authentication credentials.
    ///
    /// # Arguments
    /// * `credential` - Authentication credential (API key, token, etc.)
    ///
    /// # Returns
    /// The tenant context if credentials are valid
    ///
    /// # Errors
    /// Returns an error if:
    /// * Credentials are invalid
    /// * Tenant is not found
    /// * Tenant is disabled/suspended
    /// * Database/storage access fails
    fn resolve_tenant(
        &self,
        credential: &str,
    ) -> impl Future<Output = Result<TenantContext, Self::Error>> + Send;

    /// Validate that a tenant exists and is active.
    ///
    /// # Arguments
    /// * `tenant_id` - The tenant identifier to validate
    ///
    /// # Returns
    /// True if the tenant exists and is active
    ///
    /// # Errors
    /// Returns an error if validation fails due to system issues
    fn validate_tenant(
        &self,
        tenant_id: &str,
    ) -> impl Future<Output = Result<bool, Self::Error>> + Send;

    /// Get all active tenant IDs (useful for admin operations).
    ///
    /// # Returns
    /// A vector of all active tenant identifiers
    ///
    /// # Errors
    /// Returns an error if tenant enumeration fails
    fn list_tenants(&self) -> impl Future<Output = Result<Vec<String>, Self::Error>> + Send {
        async move {
            // Default implementation returns empty list
            Ok(vec![])
        }
    }

    /// Check if a credential is valid without returning the full context.
    ///
    /// This is useful for lightweight authentication checks.
    ///
    /// # Arguments
    /// * `credential` - Authentication credential to validate
    ///
    /// # Returns
    /// True if the credential is valid
    fn is_valid_credential(
        &self,
        credential: &str,
    ) -> impl Future<Output = Result<bool, Self::Error>> + Send {
        async move {
            match self.resolve_tenant(credential).await {
                Ok(_) => Ok(true),
                Err(_) => Ok(false),
            }
        }
    }
}

/// Static in-memory tenant resolver for testing and simple deployments.
///
/// This implementation stores tenant mappings in memory and is suitable for:
/// * Development and testing environments
/// * Simple deployments with a small number of tenants
/// * Proof-of-concept implementations
///
/// For production use with many tenants, consider implementing a database-backed resolver.
///
/// # Example Usage
///
/// ```rust
/// use scim_server::multi_tenant::StaticTenantResolver;
/// use scim_server::{TenantContext, IsolationLevel};
///
/// let mut resolver = StaticTenantResolver::new();
///
/// // Add tenant mappings
/// resolver.add_tenant(
///     "api-key-tenant-a",
///     TenantContext::new("tenant-a".to_string(), "client-a".to_string())
///         .with_isolation_level(IsolationLevel::Strict)
/// );
///
/// resolver.add_tenant(
///     "api-key-tenant-b",
///     TenantContext::new("tenant-b".to_string(), "client-b".to_string())
/// );
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// // Resolve tenant from credentials
/// let tenant_context = resolver.resolve_tenant("api-key-tenant-a").await?;
/// assert_eq!(tenant_context.tenant_id, "tenant-a");
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Clone)]
pub struct StaticTenantResolver {
    tenants: Arc<RwLock<HashMap<String, TenantContext>>>,
}

impl StaticTenantResolver {
    /// Create a new empty static tenant resolver.
    pub fn new() -> Self {
        Self {
            tenants: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Add a tenant mapping to the resolver.
    ///
    /// # Arguments
    /// * `credential` - Authentication credential for the tenant
    /// * `tenant_context` - The tenant context to associate with the credential
    ///
    /// # Example
    /// ```rust
    /// use scim_server::multi_tenant::StaticTenantResolver;
    /// use scim_server::TenantContext;
    ///
    /// let mut resolver = StaticTenantResolver::new();
    /// resolver.add_tenant(
    ///     "api-key-123",
    ///     TenantContext::new("tenant-a".to_string(), "client-a".to_string())
    /// );
    /// ```
    pub async fn add_tenant(&self, credential: &str, tenant_context: TenantContext) {
        let mut tenants = self.tenants.write().await;
        tenants.insert(credential.to_string(), tenant_context);
    }

    /// Remove a tenant mapping from the resolver.
    ///
    /// # Arguments
    /// * `credential` - Authentication credential to remove
    ///
    /// # Returns
    /// The removed tenant context, if it existed
    pub async fn remove_tenant(&self, credential: &str) -> Option<TenantContext> {
        let mut tenants = self.tenants.write().await;
        tenants.remove(credential)
    }

    /// Get the number of configured tenants.
    pub async fn tenant_count(&self) -> usize {
        let tenants = self.tenants.read().await;
        tenants.len()
    }

    /// Clear all tenant mappings.
    pub async fn clear(&self) {
        let mut tenants = self.tenants.write().await;
        tenants.clear();
    }

    /// Get all credentials (useful for testing).
    pub async fn get_all_credentials(&self) -> Vec<String> {
        let tenants = self.tenants.read().await;
        tenants.keys().cloned().collect()
    }
}

impl Default for StaticTenantResolver {
    fn default() -> Self {
        Self::new()
    }
}

/// Error type for static tenant resolver operations
#[derive(Debug, thiserror::Error)]
pub enum StaticResolverError {
    #[error("Invalid credentials: {credential}")]
    InvalidCredentials { credential: String },
    #[error("Tenant not found: {tenant_id}")]
    TenantNotFound { tenant_id: String },
}

impl TenantResolver for StaticTenantResolver {
    type Error = StaticResolverError;

    async fn resolve_tenant(&self, credential: &str) -> Result<TenantContext, Self::Error> {
        let tenants = self.tenants.read().await;
        tenants
            .get(credential)
            .cloned()
            .ok_or_else(|| StaticResolverError::InvalidCredentials {
                credential: credential.to_string(),
            })
    }

    async fn validate_tenant(&self, tenant_id: &str) -> Result<bool, Self::Error> {
        let tenants = self.tenants.read().await;
        Ok(tenants.values().any(|ctx| ctx.tenant_id == tenant_id))
    }

    async fn list_tenants(&self) -> Result<Vec<String>, Self::Error> {
        let tenants = self.tenants.read().await;
        Ok(tenants.values().map(|ctx| ctx.tenant_id.clone()).collect())
    }
}

/// Builder for creating a StaticTenantResolver with predefined tenants.
///
/// This builder provides a fluent interface for setting up multiple tenants
/// at once, which is useful for testing and initial configuration.
///
/// # Example
/// ```rust
/// use scim_server::multi_tenant::StaticTenantResolverBuilder;
/// use scim_server::{TenantContext, IsolationLevel};
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let resolver = StaticTenantResolverBuilder::new()
///     .with_tenant(
///         "key1",
///         TenantContext::new("tenant1".to_string(), "client1".to_string())
///     )
///     .with_tenant(
///         "key2",
///         TenantContext::new("tenant2".to_string(), "client2".to_string())
///             .with_isolation_level(IsolationLevel::Strict)
///     )
///     .build()
///     .await;
///
/// assert_eq!(resolver.tenant_count().await, 2);
/// # Ok(())
/// # }
/// ```
pub struct StaticTenantResolverBuilder {
    tenants: Vec<(String, TenantContext)>,
}

impl StaticTenantResolverBuilder {
    /// Create a new builder.
    pub fn new() -> Self {
        Self {
            tenants: Vec::new(),
        }
    }

    /// Add a tenant to the builder.
    pub fn with_tenant(mut self, credential: &str, tenant_context: TenantContext) -> Self {
        self.tenants.push((credential.to_string(), tenant_context));
        self
    }

    /// Build the resolver with all configured tenants.
    pub async fn build(self) -> StaticTenantResolver {
        let resolver = StaticTenantResolver::new();
        for (credential, tenant_context) in self.tenants {
            resolver.add_tenant(&credential, tenant_context).await;
        }
        resolver
    }
}

impl Default for StaticTenantResolverBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::resource::{IsolationLevel, TenantPermissions};

    #[tokio::test]
    async fn test_static_resolver_basic_operations() {
        let resolver = StaticTenantResolver::new();
        assert_eq!(resolver.tenant_count().await, 0);

        let tenant_context =
            TenantContext::new("test-tenant".to_string(), "test-client".to_string());
        resolver
            .add_tenant("test-key", tenant_context.clone())
            .await;

        assert_eq!(resolver.tenant_count().await, 1);

        let resolved = resolver.resolve_tenant("test-key").await.unwrap();
        assert_eq!(resolved.tenant_id, "test-tenant");
        assert_eq!(resolved.client_id, "test-client");
    }

    #[tokio::test]
    async fn test_static_resolver_invalid_credentials() {
        let resolver = StaticTenantResolver::new();
        let result = resolver.resolve_tenant("invalid-key").await;
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            StaticResolverError::InvalidCredentials { .. }
        ));
    }

    #[tokio::test]
    async fn test_static_resolver_tenant_validation() {
        let resolver = StaticTenantResolver::new();
        let tenant_context = TenantContext::new("valid-tenant".to_string(), "client".to_string());
        resolver.add_tenant("key", tenant_context).await;

        assert!(resolver.validate_tenant("valid-tenant").await.unwrap());
        assert!(!resolver.validate_tenant("invalid-tenant").await.unwrap());
    }

    #[tokio::test]
    async fn test_static_resolver_list_tenants() {
        let resolver = StaticTenantResolver::new();

        resolver
            .add_tenant(
                "key1",
                TenantContext::new("tenant1".to_string(), "client1".to_string()),
            )
            .await;
        resolver
            .add_tenant(
                "key2",
                TenantContext::new("tenant2".to_string(), "client2".to_string()),
            )
            .await;

        let tenants = resolver.list_tenants().await.unwrap();
        assert_eq!(tenants.len(), 2);
        assert!(tenants.contains(&"tenant1".to_string()));
        assert!(tenants.contains(&"tenant2".to_string()));
    }

    #[tokio::test]
    async fn test_static_resolver_remove_tenant() {
        let resolver = StaticTenantResolver::new();
        let tenant_context = TenantContext::new("test".to_string(), "client".to_string());
        resolver.add_tenant("key", tenant_context.clone()).await;

        assert_eq!(resolver.tenant_count().await, 1);

        let removed = resolver.remove_tenant("key").await;
        assert!(removed.is_some());
        assert_eq!(removed.unwrap().tenant_id, "test");
        assert_eq!(resolver.tenant_count().await, 0);

        let not_found = resolver.remove_tenant("nonexistent").await;
        assert!(not_found.is_none());
    }

    #[tokio::test]
    async fn test_static_resolver_clear() {
        let resolver = StaticTenantResolver::new();
        resolver
            .add_tenant(
                "key1",
                TenantContext::new("tenant1".to_string(), "client1".to_string()),
            )
            .await;
        resolver
            .add_tenant(
                "key2",
                TenantContext::new("tenant2".to_string(), "client2".to_string()),
            )
            .await;

        assert_eq!(resolver.tenant_count().await, 2);
        resolver.clear().await;
        assert_eq!(resolver.tenant_count().await, 0);
    }

    #[tokio::test]
    async fn test_static_resolver_is_valid_credential() {
        let resolver = StaticTenantResolver::new();
        resolver
            .add_tenant(
                "valid-key",
                TenantContext::new("tenant".to_string(), "client".to_string()),
            )
            .await;

        assert!(resolver.is_valid_credential("valid-key").await.unwrap());
        assert!(!resolver.is_valid_credential("invalid-key").await.unwrap());
    }

    #[tokio::test]
    async fn test_static_resolver_builder() {
        let resolver = StaticTenantResolverBuilder::new()
            .with_tenant(
                "key1",
                TenantContext::new("tenant1".to_string(), "client1".to_string()),
            )
            .with_tenant(
                "key2",
                TenantContext::new("tenant2".to_string(), "client2".to_string())
                    .with_isolation_level(IsolationLevel::Strict),
            )
            .build()
            .await;

        assert_eq!(resolver.tenant_count().await, 2);

        let tenant1 = resolver.resolve_tenant("key1").await.unwrap();
        assert_eq!(tenant1.tenant_id, "tenant1");
        assert_eq!(tenant1.isolation_level, IsolationLevel::Standard);

        let tenant2 = resolver.resolve_tenant("key2").await.unwrap();
        assert_eq!(tenant2.tenant_id, "tenant2");
        assert_eq!(tenant2.isolation_level, IsolationLevel::Strict);
    }

    #[tokio::test]
    async fn test_static_resolver_get_all_credentials() {
        let resolver = StaticTenantResolver::new();
        resolver
            .add_tenant(
                "key1",
                TenantContext::new("tenant1".to_string(), "client1".to_string()),
            )
            .await;
        resolver
            .add_tenant(
                "key2",
                TenantContext::new("tenant2".to_string(), "client2".to_string()),
            )
            .await;

        let credentials = resolver.get_all_credentials().await;
        assert_eq!(credentials.len(), 2);
        assert!(credentials.contains(&"key1".to_string()));
        assert!(credentials.contains(&"key2".to_string()));
    }

    #[tokio::test]
    async fn test_complex_tenant_context() {
        let mut permissions = TenantPermissions::default();
        permissions.max_users = Some(100);
        permissions.can_delete = false;

        let tenant_context =
            TenantContext::new("complex-tenant".to_string(), "complex-client".to_string())
                .with_isolation_level(IsolationLevel::Strict)
                .with_permissions(permissions);

        let resolver = StaticTenantResolver::new();
        resolver.add_tenant("complex-key", tenant_context).await;

        let resolved = resolver.resolve_tenant("complex-key").await.unwrap();
        assert_eq!(resolved.isolation_level, IsolationLevel::Strict);
        assert_eq!(resolved.permissions.max_users, Some(100));
        assert!(!resolved.permissions.can_delete);
        assert!(resolved.check_user_limit(50));
        assert!(!resolved.check_user_limit(100));
    }
}
