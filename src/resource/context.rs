//! Request context and query structures for SCIM operations.
//!
//! This module provides request tracking, tenant context, and query parameters
//! for SCIM operations with support for multi-tenant environments.

use crate::resource::tenant::{IsolationLevel, TenantContext};
use uuid::Uuid;

/// Request context for SCIM operations.
///
/// Provides request tracking for logging and auditing purposes.
/// Optionally includes tenant context for multi-tenant operations.
#[derive(Debug, Clone)]
pub struct RequestContext {
    /// Unique identifier for this request
    pub request_id: String,
    /// Optional tenant context for multi-tenant operations
    pub tenant_context: Option<TenantContext>,
}

impl RequestContext {
    /// Create a new request context with a specific request ID.
    pub fn new(request_id: String) -> Self {
        Self {
            request_id,
            tenant_context: None,
        }
    }

    /// Create a new request context with a generated request ID.
    pub fn with_generated_id() -> Self {
        Self {
            request_id: Uuid::new_v4().to_string(),
            tenant_context: None,
        }
    }

    /// Create a new request context with tenant information.
    pub fn with_tenant(request_id: String, tenant_context: TenantContext) -> Self {
        Self {
            request_id,
            tenant_context: Some(tenant_context),
        }
    }

    /// Create a new request context with generated ID and tenant information.
    pub fn with_tenant_generated_id(tenant_context: TenantContext) -> Self {
        Self {
            request_id: Uuid::new_v4().to_string(),
            tenant_context: Some(tenant_context),
        }
    }

    /// Get the tenant ID if this is a multi-tenant request.
    pub fn tenant_id(&self) -> Option<&str> {
        self.tenant_context.as_ref().map(|t| t.tenant_id.as_str())
    }

    /// Get the client ID if this is a multi-tenant request.
    pub fn client_id(&self) -> Option<&str> {
        self.tenant_context.as_ref().map(|t| t.client_id.as_str())
    }

    /// Check if this is a multi-tenant request.
    pub fn is_multi_tenant(&self) -> bool {
        self.tenant_context.is_some()
    }

    /// Get the isolation level for this request.
    pub fn isolation_level(&self) -> Option<&IsolationLevel> {
        self.tenant_context.as_ref().map(|t| &t.isolation_level)
    }

    /// Check if the tenant has permission for a specific operation.
    pub fn can_perform_operation(&self, operation: &str) -> bool {
        match &self.tenant_context {
            Some(tenant) => tenant.can_perform_operation(operation),
            None => true, // Single-tenant operations are always allowed
        }
    }

    /// Validate that this context can perform the requested operation.
    pub fn validate_operation(&self, operation: &str) -> Result<(), String> {
        if self.can_perform_operation(operation) {
            Ok(())
        } else {
            Err(format!(
                "Operation '{}' not permitted for tenant",
                operation
            ))
        }
    }
}

impl Default for RequestContext {
    fn default() -> Self {
        Self::with_generated_id()
    }
}

/// Query parameters for listing resources.
///
/// This structure supports pagination, filtering, and attribute selection
/// for SCIM list operations.
#[derive(Debug, Clone, Default)]
pub struct ListQuery {
    /// Maximum number of results to return
    pub count: Option<usize>,
    /// Starting index for pagination
    pub start_index: Option<usize>,
    /// Filter expression
    pub filter: Option<String>,
    /// Attributes to include in results
    pub attributes: Vec<String>,
    /// Attributes to exclude from results
    pub excluded_attributes: Vec<String>,
}

impl ListQuery {
    /// Create a new empty query.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the maximum count.
    pub fn with_count(mut self, count: usize) -> Self {
        self.count = Some(count);
        self
    }

    /// Set the starting index.
    pub fn with_start_index(mut self, start_index: usize) -> Self {
        self.start_index = Some(start_index);
        self
    }

    /// Set a filter expression.
    pub fn with_filter(mut self, filter: String) -> Self {
        self.filter = Some(filter);
        self
    }

    /// Add an attribute to include in results.
    pub fn with_attribute(mut self, attribute: String) -> Self {
        self.attributes.push(attribute);
        self
    }

    /// Add multiple attributes to include in results.
    pub fn with_attributes(mut self, attributes: Vec<String>) -> Self {
        self.attributes.extend(attributes);
        self
    }

    /// Add an attribute to exclude from results.
    pub fn with_excluded_attribute(mut self, attribute: String) -> Self {
        self.excluded_attributes.push(attribute);
        self
    }

    /// Add multiple attributes to exclude from results.
    pub fn with_excluded_attributes(mut self, attributes: Vec<String>) -> Self {
        self.excluded_attributes.extend(attributes);
        self
    }
}
