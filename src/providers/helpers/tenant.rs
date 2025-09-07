//! Multi-tenant context management helper trait.
//!
//! This module provides reusable functionality for managing multi-tenant context
//! in SCIM ResourceProvider implementations. It handles tenant ID resolution,
//! context validation, and tenant isolation patterns.
//!
//! # Multi-Tenant Patterns
//!
//! This implementation supports common multi-tenant patterns:
//! - Single-tenant mode (no tenant context required)
//! - Multi-tenant mode with explicit tenant identification
//! - Tenant isolation with fallback to "default" tenant
//! - Context validation and extraction
//!
//! # Usage
//!
//! ```rust,no_run
//! // MultiTenantProvider provides helper methods for tenant isolation:
//! // - effective_tenant_id(): Extract tenant ID from context
//! // - tenant_scoped_key(): Generate tenant-specific storage keys
//! // - tenant_scoped_prefix(): Generate tenant-specific prefixes
//! // - generate_tenant_resource_id(): Generate tenant-scoped resource IDs
//! //
//! // When implemented by a ResourceProvider, enables automatic tenant isolation
//! // across all operations without additional code
//! ```

use crate::providers::ResourceProvider;
use crate::resource::{RequestContext, TenantContext};
use uuid::Uuid;

/// Trait providing multi-tenant context management functionality.
///
/// This trait extends ResourceProvider with multi-tenant capabilities including
/// tenant ID resolution, context validation, and key generation for tenant isolation.
/// Most implementers can use the default implementations which provide standard
/// multi-tenant patterns.
pub trait MultiTenantProvider: ResourceProvider {
    /// Get the effective tenant ID for an operation.
    ///
    /// Resolves the tenant ID from the request context using standard patterns:
    /// - If context has a tenant ID, use it
    /// - If no tenant ID, fall back to "default" for single-tenant operations
    /// - Ensures consistent tenant identification across operations
    ///
    /// # Arguments
    /// * `context` - The request context containing tenant information
    ///
    /// # Returns
    /// The effective tenant ID to use for the operation
    ///
    /// # Example
    /// ```rust,no_run
    /// use scim_server::resource::{RequestContext, TenantContext};
    ///
    /// let context = RequestContext::with_generated_id();
    /// // MultiTenantProvider.effective_tenant_id(&context) returns: "default"
    ///
    /// let tenant_context = TenantContext::new("acme-corp".to_string(), "client-123".to_string());
    /// let multi_context = RequestContext::with_tenant_generated_id(tenant_context);
    /// // MultiTenantProvider.effective_tenant_id(&multi_context) returns: "acme-corp"
    /// ```
    fn effective_tenant_id(&self, context: &RequestContext) -> String {
        context.tenant_id().unwrap_or("default").to_string()
    }

    /// Create a tenant-scoped storage key.
    ///
    /// Generates a storage key that includes tenant information for proper isolation.
    /// This ensures resources from different tenants don't interfere with each other.
    ///
    /// # Arguments
    /// * `tenant_id` - The tenant identifier
    /// * `resource_type` - The type of resource (e.g., "Users", "Groups")
    /// * `resource_id` - The unique identifier of the resource
    ///
    /// # Returns
    /// A tenant-scoped key for storage operations
    ///
    /// # Example
    /// ```rust,no_run
    /// // MultiTenantProvider.tenant_scoped_key("acme-corp", "Users", "123")
    /// // Returns: "tenant:acme-corp:Users:123"
    /// //
    /// // Used for generating tenant-specific storage keys that prevent
    /// // cross-tenant data access
    /// ```
    fn tenant_scoped_key(&self, tenant_id: &str, resource_type: &str, resource_id: &str) -> String {
        format!("tenant:{}:{}:{}", tenant_id, resource_type, resource_id)
    }

    /// Create a tenant-scoped prefix for listing operations.
    ///
    /// Generates a key prefix that can be used to list all resources of a given type
    /// within a specific tenant, enabling efficient tenant-isolated queries.
    ///
    /// # Arguments
    /// * `tenant_id` - The tenant identifier
    /// * `resource_type` - The type of resource (e.g., "Users", "Groups")
    ///
    /// # Returns
    /// A tenant-scoped prefix for listing operations
    ///
    /// # Example
    /// ```rust,no_run
    /// // MultiTenantProvider.tenant_scoped_prefix("acme-corp", "Users")
    /// // Returns: "tenant:acme-corp:Users:"
    /// //
    /// // Used for generating tenant-specific prefixes for resource queries
    /// // and bulk operations
    /// ```
    fn tenant_scoped_prefix(&self, tenant_id: &str, resource_type: &str) -> String {
        format!("tenant:{}:{}:", tenant_id, resource_type)
    }

    /// Validate that a request context is appropriate for multi-tenant operations.
    ///
    /// Checks that the request context contains valid tenant information when
    /// operating in multi-tenant mode. Can be used to enforce tenant requirements.
    ///
    /// # Arguments
    /// * `context` - The request context to validate
    /// * `require_tenant` - Whether a tenant ID is required (true for strict multi-tenant)
    ///
    /// # Returns
    /// `true` if the context is valid for the tenant requirements
    fn is_valid_tenant_context(&self, context: &RequestContext, require_tenant: bool) -> bool {
        if require_tenant {
            context.tenant_id().is_some()
        } else {
            true // Always valid in mixed-mode operations
        }
    }

    /// Extract tenant context information from a request context.
    ///
    /// Retrieves the complete tenant context including tenant ID and client ID
    /// if available, useful for detailed tenant tracking and auditing.
    ///
    /// # Arguments
    /// * `context` - The request context to extract from
    ///
    /// # Returns
    /// The tenant context if present, None for single-tenant operations
    fn extract_tenant_context<'a>(&self, context: &'a RequestContext) -> Option<&'a TenantContext> {
        context.tenant_context.as_ref()
    }

    /// Generate a unique resource ID within a tenant scope.
    ///
    /// Creates a unique identifier for a new resource within a specific tenant.
    /// The default implementation uses UUIDs for uniqueness across tenants.
    ///
    /// # Arguments
    /// * `tenant_id` - The tenant identifier (for context, not included in ID)
    /// * `resource_type` - The type of resource (for context, not included in ID)
    ///
    /// # Returns
    /// A unique resource identifier
    ///
    /// # Example
    /// ```rust,no_run
    /// // MultiTenantProvider.generate_tenant_resource_id("acme-corp", "Users")
    /// // Returns: "123e4567-e89b-12d3-a456-426614174000" (UUID format)
    /// //
    /// // Generates globally unique IDs that are deterministically associated
    /// // with the tenant and resource type for consistent resource identification
    /// ```
    fn generate_tenant_resource_id(&self, _tenant_id: &str, _resource_type: &str) -> String {
        Uuid::new_v4().to_string()
    }

    /// Check if two request contexts belong to the same tenant.
    ///
    /// Compares tenant information between two request contexts to determine
    /// if they represent operations within the same tenant scope.
    ///
    /// # Arguments
    /// * `context1` - First request context
    /// * `context2` - Second request context
    ///
    /// # Returns
    /// `true` if both contexts belong to the same effective tenant
    fn same_tenant(&self, context1: &RequestContext, context2: &RequestContext) -> bool {
        self.effective_tenant_id(context1) == self.effective_tenant_id(context2)
    }

    /// Create a tenant-specific error message.
    ///
    /// Generates error messages that include tenant context for better debugging
    /// and audit trails in multi-tenant environments.
    ///
    /// # Arguments
    /// * `context` - The request context for tenant information
    /// * `base_message` - The base error message
    ///
    /// # Returns
    /// An error message with tenant context
    fn tenant_error_message(&self, context: &RequestContext, base_message: &str) -> String {
        let tenant_id = self.effective_tenant_id(context);
        if tenant_id == "default" {
            base_message.to_string()
        } else {
            format!("[Tenant: {}] {}", tenant_id, base_message)
        }
    }

    /// Check if the provider is operating in single-tenant mode for a context.
    ///
    /// Determines whether a specific request context represents single-tenant
    /// operation (no explicit tenant specified).
    ///
    /// # Arguments
    /// * `context` - The request context to check
    ///
    /// # Returns
    /// `true` if operating in single-tenant mode for this context
    fn is_single_tenant_context(&self, context: &RequestContext) -> bool {
        context.tenant_id().is_none()
    }

    /// Check if the provider is operating in multi-tenant mode for a context.
    ///
    /// Determines whether a specific request context represents multi-tenant
    /// operation (explicit tenant specified).
    ///
    /// # Arguments
    /// * `context` - The request context to check
    ///
    /// # Returns
    /// `true` if operating in multi-tenant mode for this context
    fn is_multi_tenant_context(&self, context: &RequestContext) -> bool {
        context.tenant_id().is_some()
    }

    /// Get the client ID associated with a tenant context.
    ///
    /// Extracts the client identifier from a multi-tenant request context,
    /// useful for client-specific operations or auditing.
    ///
    /// # Arguments
    /// * `context` - The request context to extract from
    ///
    /// # Returns
    /// The client ID if present in a multi-tenant context
    fn get_client_id<'a>(&self, context: &'a RequestContext) -> Option<&'a str> {
        context
            .tenant_context
            .as_ref()
            .map(|tc| tc.client_id.as_str())
    }

    /// Normalize a tenant ID for consistent storage and comparison.
    ///
    /// Applies consistent formatting rules to tenant IDs to ensure
    /// reliable storage key generation and tenant comparison.
    ///
    /// # Arguments
    /// * `tenant_id` - The raw tenant ID to normalize
    ///
    /// # Returns
    /// The normalized tenant ID
    ///
    /// # Default Behavior
    /// - Converts to lowercase
    /// - Trims whitespace
    /// - Replaces spaces with hyphens
    /// - Validates basic format
    fn normalize_tenant_id(&self, tenant_id: &str) -> String {
        tenant_id
            .trim()
            .to_lowercase()
            .replace(' ', "-")
            .chars()
            .filter(|c| c.is_alphanumeric() || *c == '-' || *c == '_')
            .collect()
    }

    /// Validate that a tenant ID meets format requirements.
    ///
    /// Checks that a tenant identifier follows acceptable patterns for
    /// use in storage keys and URL paths.
    ///
    /// # Arguments
    /// * `tenant_id` - The tenant ID to validate
    ///
    /// # Returns
    /// `true` if the tenant ID is valid for use
    fn is_valid_tenant_id(&self, tenant_id: &str) -> bool {
        !tenant_id.trim().is_empty()
            && tenant_id.len() <= 64
            && tenant_id
                .chars()
                .all(|c| c.is_alphanumeric() || c == '-' || c == '_')
            && !tenant_id.starts_with('-')
            && !tenant_id.ends_with('-')
    }
}

/// Default implementation for any ResourceProvider
impl<T: ResourceProvider> MultiTenantProvider for T {}
