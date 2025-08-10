//! Tenant-related types for multi-tenant SCIM operations.
//!
//! This module contains the fundamental data structures for managing tenant
//! contexts, permissions, and isolation levels in multi-tenant environments.

use serde::{Deserialize, Serialize};

/// Tenant isolation level configuration
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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

/// Tenant-specific permissions for resource operations
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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
