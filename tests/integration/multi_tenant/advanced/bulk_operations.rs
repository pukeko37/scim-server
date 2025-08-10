//! Bulk operations structures and types for multi-tenant SCIM operations.
//!
//! This module contains the data structures for handling bulk operations
//! within tenant-scoped contexts, including batch create/update/delete
//! operations and tenant data migration functionality.

use scim_server::Resource;
use serde_json::Value;

/// Bulk operation request for tenant-scoped operations
#[derive(Debug)]
pub struct BulkOperationRequest {
    pub tenant_id: String,
    pub operations: Vec<BulkOperation>,
    pub fail_on_errors: bool,
    pub continue_on_error: bool,
}

#[derive(Debug)]
pub struct BulkOperation {
    pub operation_type: BulkOperationType,
    pub resource_type: String,
    pub resource_id: Option<String>,
    pub data: Option<Value>,
}

#[derive(Debug)]
pub enum BulkOperationType {
    Create,
    Update,
    Delete,
    Patch,
}

/// Results from bulk operations
#[derive(Debug)]
pub struct BulkOperationResult {
    pub total_operations: usize,
    pub successful_operations: usize,
    pub failed_operations: usize,
    pub results: Vec<BulkOperationItemResult>,
    pub duration: std::time::Duration,
}

#[derive(Debug)]
pub struct BulkOperationItemResult {
    pub operation_index: usize,
    pub success: bool,
    pub resource: Option<Resource>,
    pub error: Option<String>,
}

/// Tenant data migration request
#[derive(Debug)]
pub struct TenantMigrationRequest {
    pub source_tenant_id: String,
    pub target_tenant_id: String,
    pub resource_types: Vec<String>,
    pub migration_strategy: MigrationStrategy,
    pub preserve_ids: bool,
}

#[derive(Debug)]
pub enum MigrationStrategy {
    Copy,  // Copy resources to target tenant
    Move,  // Move resources from source to target
    Merge, // Merge with existing resources in target
}
