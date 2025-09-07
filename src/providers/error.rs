//! Error types for resource provider implementations.
//!
//! This module provides error types that are shared across different resource
//! provider implementations, independent of the underlying storage backend.
//!
//! # Key Types
//!
//! - [`ProviderError`] - Generic provider error type for resource operations
//!
//! # Usage
//!
//! This error type is used with `StandardResourceProvider<T>` regardless of storage backend:
//!
//! ```rust
//! use scim_server::providers::StandardResourceProvider;
//! use scim_server::storage::InMemoryStorage;
//!
//! let storage = InMemoryStorage::new();
//! let provider = StandardResourceProvider::new(storage);
//! ```

use thiserror::Error;

/// Errors that can occur during resource provider operations.
///
/// This error type is used by resource providers to represent various failure
/// conditions during SCIM operations, independent of the underlying storage backend.
#[derive(Debug, Clone, Error)]
pub enum ProviderError {
    #[error("Resource not found: {resource_type} with id '{id}' in tenant '{tenant_id}'")]
    ResourceNotFound {
        /// The type of resource that was not found
        resource_type: String,
        /// The ID of the resource that was not found
        id: String,
        /// The tenant ID where the resource was not found
        tenant_id: String,
    },

    #[error(
        "Duplicate attribute '{attribute}' with value '{value}' for {resource_type} in tenant '{tenant_id}'"
    )]
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

    #[error("Storage error: {message}")]
    Storage {
        /// Description of the storage error
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

    #[error(
        "Duplicate resource: {resource_type} with userName '{username}' already exists in tenant '{tenant_id}'"
    )]
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

impl From<String> for ProviderError {
    fn from(message: String) -> Self {
        ProviderError::Internal { message }
    }
}
