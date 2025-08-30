//! Error types and statistics for in-memory resource provider implementations.
//!
//! This module provides error types and statistics structures that are shared
//! between different in-memory resource provider implementations.
//!
//! # Key Types
//!
//! - [`InMemoryError`] - Provider-specific error types for in-memory operations
//! - [`InMemoryStats`] - Resource statistics and performance metrics
//!
//! # Usage
//!
//! These types are used with `StandardResourceProvider<InMemoryStorage>`:
//!
//! ```rust
//! use scim_server::providers::StandardResourceProvider;
//! use scim_server::storage::InMemoryStorage;
//!
//! let storage = InMemoryStorage::new();
//! let provider = StandardResourceProvider::new(storage);
//! ```

use thiserror::Error;

/// Errors that can occur during in-memory provider operations.
///
/// This error type is used by in-memory resource providers to represent
/// various failure conditions during SCIM operations.
#[derive(Debug, Clone, Error)]
pub enum InMemoryError {
    #[error("Resource not found: {resource_type} with id '{id}' in tenant '{tenant_id}'")]
    ResourceNotFound {
        /// The type of resource that was not found
        resource_type: String,
        /// The ID of the resource that was not found
        id: String,
        /// The tenant ID where the resource was not found
        tenant_id: String,
    },

    #[error("Duplicate attribute '{attribute}' with value '{value}' for {resource_type} in tenant '{tenant_id}'")]
    DuplicateAttribute {
        /// The type of resource with duplicate attribute
        resource_type: String,
        /// The name of the duplicate attribute
        attribute: String,
        /// The duplicate value
        value: String,
        /// The tenant ID where the duplicate was found
        tenant_id: String,
    },

    #[error("Invalid resource data: {message}")]
    InvalidData {
        /// Description of the invalid data
        message: String,
    },

    #[error("Query error: {message}")]
    QueryError {
        /// Description of the query error
        message: String,
    },

    #[error("Internal error: {message}")]
    Internal {
        /// Description of the internal error
        message: String,
    },

    #[error("Invalid input: {message}")]
    InvalidInput {
        /// Description of what input was invalid
        message: String,
    },

    #[error("Resource not found: {resource_type} with id '{id}'")]
    NotFound {
        /// The type of resource that was not found
        resource_type: String,
        /// The ID of the resource that was not found
        id: String,
    },

    #[error("Precondition failed: {message}")]
    PreconditionFailed {
        /// Description of the precondition failure
        message: String,
    },

    #[error("Duplicate resource: {resource_type} with userName '{username}' already exists in tenant '{tenant_id}'")]
    DuplicateUserName {
        /// The resource type that had the duplicate
        resource_type: String,
        /// The duplicate userName
        username: String,
        /// The tenant where the duplicate was found
        tenant_id: String,
    },

    #[error("Patch operation failed: {message}")]
    PatchOperationFailed {
        /// Description of why the patch operation failed
        message: String,
    },

    #[error("Version conflict: {conflict}")]
    VersionConflict {
        /// Details of the version conflict
        #[from]
        conflict: crate::resource::version::VersionConflict,
    },
}



/// Statistics about the in-memory provider state.
///
/// Provides metrics about resource counts, tenants, and resource types
/// for monitoring and debugging purposes.
#[derive(Debug, Clone)]
pub struct InMemoryStats {
    /// Number of active tenants in the provider
    pub tenant_count: usize,
    /// Total number of resources across all tenants
    pub total_resources: usize,
    /// Number of distinct resource types
    pub resource_type_count: usize,
    /// List of resource type names
    pub resource_types: Vec<String>,
}

impl InMemoryStats {
    /// Create new empty statistics.
    pub fn new() -> Self {
        Self {
            tenant_count: 0,
            total_resources: 0,
            resource_type_count: 0,
            resource_types: Vec::new(),
        }
    }

    /// Check if the provider is empty (no resources).
    pub fn is_empty(&self) -> bool {
        self.total_resources == 0
    }
}

impl Default for InMemoryStats {
    fn default() -> Self {
        Self::new()
    }
}
