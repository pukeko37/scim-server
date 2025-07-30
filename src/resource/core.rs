//! Core types for SCIM resource operations.
//!
//! This module contains the fundamental data structures used throughout
//! the SCIM server for representing resources and operation contexts.

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Tenant isolation level configuration
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum IsolationLevel {
    /// Strict isolation - complete separation of tenant data
    Strict,
    /// Standard isolation - shared infrastructure with data separation
    Standard,
    /// Shared resources - some resources may be shared between tenants
    Shared,
}

impl Default for IsolationLevel {
    fn default() -> Self {
        IsolationLevel::Standard
    }
}

/// Tenant permissions for resource operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TenantPermissions {
    pub can_create: bool,
    pub can_read: bool,
    pub can_update: bool,
    pub can_delete: bool,
    pub can_list: bool,
    pub max_users: Option<usize>,
    pub max_groups: Option<usize>,
}

impl Default for TenantPermissions {
    fn default() -> Self {
        Self {
            can_create: true,
            can_read: true,
            can_update: true,
            can_delete: true,
            can_list: true,
            max_users: None,
            max_groups: None,
        }
    }
}

/// Tenant context for multi-tenant operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TenantContext {
    pub tenant_id: String,
    pub client_id: String,
    pub isolation_level: IsolationLevel,
    pub permissions: TenantPermissions,
}

impl TenantContext {
    /// Create a new tenant context with default permissions
    pub fn new(tenant_id: String, client_id: String) -> Self {
        Self {
            tenant_id,
            client_id,
            isolation_level: IsolationLevel::default(),
            permissions: TenantPermissions::default(),
        }
    }

    /// Create a tenant context with custom isolation level
    pub fn with_isolation_level(mut self, level: IsolationLevel) -> Self {
        self.isolation_level = level;
        self
    }

    /// Create a tenant context with custom permissions
    pub fn with_permissions(mut self, permissions: TenantPermissions) -> Self {
        self.permissions = permissions;
        self
    }

    /// Check if the tenant has permission for a specific operation
    pub fn can_perform_operation(&self, operation: &str) -> bool {
        match operation {
            "create" => self.permissions.can_create,
            "read" => self.permissions.can_read,
            "update" => self.permissions.can_update,
            "delete" => self.permissions.can_delete,
            "list" => self.permissions.can_list,
            _ => false,
        }
    }

    /// Check if tenant has reached user limit
    pub fn check_user_limit(&self, current_count: usize) -> bool {
        match self.permissions.max_users {
            Some(limit) => current_count < limit,
            None => true,
        }
    }

    /// Check if tenant has reached group limit
    pub fn check_group_limit(&self, current_count: usize) -> bool {
        match self.permissions.max_groups {
            Some(limit) => current_count < limit,
            None => true,
        }
    }
}

/// Generic SCIM resource representation.
///
/// A resource is a structured data object with a type identifier and JSON data.
/// This design provides flexibility while maintaining schema validation through
/// the server layer.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Resource {
    /// The type of this resource (e.g., "User", "Group")
    pub resource_type: String,
    /// The resource data as validated JSON
    pub data: Value,
}

impl Resource {
    /// Create a new resource with the given type and data.
    ///
    /// # Arguments
    /// * `resource_type` - The SCIM resource type identifier
    /// * `data` - The resource data as a JSON value
    ///
    /// # Example
    /// ```rust
    /// use scim_server::Resource;
    /// use serde_json::json;
    ///
    /// let user_data = json!({
    ///     "userName": "jdoe",
    ///     "displayName": "John Doe"
    /// });
    /// let resource = Resource::new("User".to_string(), user_data);
    /// ```
    pub fn new(resource_type: String, data: Value) -> Self {
        Self {
            resource_type,
            data,
        }
    }

    /// Get the unique identifier of this resource.
    ///
    /// Returns the "id" field from the resource data if present.
    pub fn get_id(&self) -> Option<&str> {
        self.data.get("id")?.as_str()
    }

    /// Get the userName field for User resources.
    ///
    /// This is a convenience method for accessing the required userName field.
    pub fn get_username(&self) -> Option<&str> {
        self.data.get("userName")?.as_str()
    }

    /// Get a specific attribute value from the resource data.
    ///
    /// # Arguments
    /// * `attribute_name` - The name of the attribute to retrieve
    pub fn get_attribute(&self, attribute_name: &str) -> Option<&Value> {
        self.data.get(attribute_name)
    }

    /// Set a specific attribute value in the resource data.
    ///
    /// # Arguments
    /// * `attribute_name` - The name of the attribute to set
    /// * `value` - The value to set
    pub fn set_attribute(&mut self, attribute_name: String, value: Value) {
        if let Some(obj) = self.data.as_object_mut() {
            obj.insert(attribute_name, value);
        }
    }

    /// Get the schemas associated with this resource.
    pub fn get_schemas(&self) -> Vec<String> {
        self.data
            .get("schemas")
            .and_then(|s| s.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str())
                    .map(|s| s.to_string())
                    .collect()
            })
            .unwrap_or_else(|| {
                // Default schema based on resource type
                match self.resource_type.as_str() {
                    "User" => vec!["urn:ietf:params:scim:schemas:core:2.0:User".to_string()],
                    "Group" => vec!["urn:ietf:params:scim:schemas:core:2.0:Group".to_string()],
                    _ => vec![],
                }
            })
    }

    /// Add metadata to the resource.
    ///
    /// This method sets common SCIM metadata fields like resourceType,
    /// created, lastModified, and location.
    pub fn add_metadata(&mut self, base_url: &str, created: &str, last_modified: &str) {
        let meta = serde_json::json!({
            "resourceType": self.resource_type,
            "created": created,
            "lastModified": last_modified,
            "location": format!("{}/{}s/{}", base_url, self.resource_type, self.get_id().unwrap_or("")),
            "version": format!("W/\"{}-{}\"", self.get_id().unwrap_or(""), last_modified)
        });

        self.set_attribute("meta".to_string(), meta);
    }

    /// Check if this resource is active.
    ///
    /// Returns the value of the "active" field, defaulting to true if not present.
    pub fn is_active(&self) -> bool {
        self.data
            .get("active")
            .and_then(|v| v.as_bool())
            .unwrap_or(true)
    }

    /// Get all email addresses from the resource.
    pub fn get_emails(&self) -> Vec<super::types::EmailAddress> {
        self.data
            .get("emails")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|email| {
                        let value = email.get("value")?.as_str()?;
                        Some(super::types::EmailAddress {
                            value: value.to_string(),
                            email_type: email
                                .get("type")
                                .and_then(|t| t.as_str())
                                .map(|s| s.to_string()),
                            primary: email.get("primary").and_then(|p| p.as_bool()),
                            display: email
                                .get("display")
                                .and_then(|d| d.as_str())
                                .map(|s| s.to_string()),
                        })
                    })
                    .collect()
            })
            .unwrap_or_default()
    }
}

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
            request_id: uuid::Uuid::new_v4().to_string(),
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
            request_id: uuid::Uuid::new_v4().to_string(),
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

/// Enhanced request context for multi-tenant operations.
///
/// This is a convenience type that guarantees the presence of tenant context.
#[derive(Debug, Clone)]
pub struct EnhancedRequestContext {
    pub request_id: String,
    pub tenant_context: TenantContext,
}

impl EnhancedRequestContext {
    /// Create a new enhanced request context.
    pub fn new(request_id: String, tenant_context: TenantContext) -> Self {
        Self {
            request_id,
            tenant_context,
        }
    }

    /// Create a new enhanced request context with generated request ID.
    pub fn with_generated_id(tenant_context: TenantContext) -> Self {
        Self {
            request_id: uuid::Uuid::new_v4().to_string(),
            tenant_context,
        }
    }

    /// Convert to a regular RequestContext.
    pub fn to_request_context(self) -> RequestContext {
        RequestContext {
            request_id: self.request_id,
            tenant_context: Some(self.tenant_context),
        }
    }

    /// Get the tenant ID.
    pub fn tenant_id(&self) -> &str {
        &self.tenant_context.tenant_id
    }

    /// Get the client ID.
    pub fn client_id(&self) -> &str {
        &self.tenant_context.client_id
    }

    /// Get the isolation level.
    pub fn isolation_level(&self) -> &IsolationLevel {
        &self.tenant_context.isolation_level
    }

    /// Check if the tenant has permission for a specific operation.
    pub fn can_perform_operation(&self, operation: &str) -> bool {
        self.tenant_context.can_perform_operation(operation)
    }

    /// Validate that this context can perform the requested operation.
    pub fn validate_operation(&self, operation: &str) -> Result<(), String> {
        if self.can_perform_operation(operation) {
            Ok(())
        } else {
            Err(format!(
                "Operation '{}' not permitted for tenant {}",
                operation,
                self.tenant_id()
            ))
        }
    }
}

impl TryFrom<RequestContext> for EnhancedRequestContext {
    type Error = String;

    fn try_from(context: RequestContext) -> Result<Self, Self::Error> {
        match context.tenant_context {
            Some(tenant_context) => Ok(EnhancedRequestContext {
                request_id: context.request_id,
                tenant_context,
            }),
            None => Err("RequestContext does not contain tenant information".to_string()),
        }
    }
}

/// Supported SCIM operations for resource types
#[derive(Debug, Clone, PartialEq)]
pub enum ScimOperation {
    Create,
    Read,
    Update,
    Delete,
    List,
    Search,
}

/// Query parameters for listing resources (future extension).
///
/// This structure is prepared for future pagination and filtering support
/// but is not used in the MVP implementation.
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
}
