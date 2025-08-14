//! Storage-specific error types for pure data operations.
//!
//! This module defines errors that can occur during storage operations, separate from
//! SCIM protocol errors or business logic errors. These errors focus on data persistence
//! and retrieval failures.

use std::fmt;

/// Errors that can occur during storage operations.
///
/// These errors represent failures in the storage layer and are protocol-agnostic.
/// They focus on data persistence, retrieval, and basic storage operations without
/// any knowledge of SCIM semantics or business rules.
#[derive(Debug)]
pub enum StorageError {
    /// The requested resource was not found.
    ResourceNotFound {
        tenant_id: String,
        resource_type: String,
        id: String,
    },

    /// The resource already exists when it shouldn't (for operations requiring uniqueness).
    ResourceAlreadyExists {
        tenant_id: String,
        resource_type: String,
        id: String,
    },

    /// Invalid data format or structure that cannot be stored.
    InvalidData {
        message: String,
        cause: Option<String>,
    },

    /// The tenant was not found or is invalid.
    TenantNotFound { tenant_id: String },

    /// Invalid query parameters or search criteria.
    InvalidQuery {
        message: String,
        attribute: Option<String>,
        value: Option<String>,
    },

    /// Storage capacity exceeded (disk full, memory limit, etc.).
    CapacityExceeded {
        message: String,
        current_count: Option<usize>,
        limit: Option<usize>,
    },

    /// Concurrent modification detected (optimistic locking failure).
    ConcurrentModification {
        tenant_id: String,
        resource_type: String,
        id: String,
        expected_version: Option<String>,
        actual_version: Option<String>,
    },

    /// Storage backend is temporarily unavailable.
    Unavailable {
        message: String,
        retry_after: Option<std::time::Duration>,
    },

    /// Permission denied for the storage operation.
    PermissionDenied { operation: String, resource: String },

    /// Timeout occurred during storage operation.
    Timeout {
        operation: String,
        duration: std::time::Duration,
    },

    /// Corruption detected in stored data.
    DataCorruption {
        tenant_id: String,
        resource_type: String,
        id: Option<String>,
        details: String,
    },

    /// Configuration error in the storage backend.
    Configuration {
        message: String,
        parameter: Option<String>,
    },

    /// Network-related error for distributed storage systems.
    Network {
        message: String,
        endpoint: Option<String>,
    },

    /// Serialization or deserialization error.
    Serialization {
        message: String,
        data_type: Option<String>,
    },

    /// Generic internal storage error.
    Internal {
        message: String,
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },
}

impl fmt::Display for StorageError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            StorageError::ResourceNotFound {
                tenant_id,
                resource_type,
                id,
            } => {
                write!(
                    f,
                    "Resource not found: {}/{}/{}",
                    tenant_id, resource_type, id
                )
            }
            StorageError::ResourceAlreadyExists {
                tenant_id,
                resource_type,
                id,
            } => {
                write!(
                    f,
                    "Resource already exists: {}/{}/{}",
                    tenant_id, resource_type, id
                )
            }
            StorageError::InvalidData { message, cause } => {
                if let Some(cause) = cause {
                    write!(f, "Invalid data: {} (cause: {})", message, cause)
                } else {
                    write!(f, "Invalid data: {}", message)
                }
            }
            StorageError::TenantNotFound { tenant_id } => {
                write!(f, "Tenant not found: {}", tenant_id)
            }
            StorageError::InvalidQuery {
                message,
                attribute,
                value,
            } => match (attribute, value) {
                (Some(attr), Some(val)) => {
                    write!(
                        f,
                        "Invalid query: {} (attribute: {}, value: {})",
                        message, attr, val
                    )
                }
                (Some(attr), None) => {
                    write!(f, "Invalid query: {} (attribute: {})", message, attr)
                }
                _ => write!(f, "Invalid query: {}", message),
            },
            StorageError::CapacityExceeded {
                message,
                current_count,
                limit,
            } => match (current_count, limit) {
                (Some(current), Some(max)) => {
                    write!(f, "Capacity exceeded: {} ({}/{})", message, current, max)
                }
                _ => write!(f, "Capacity exceeded: {}", message),
            },
            StorageError::ConcurrentModification {
                tenant_id,
                resource_type,
                id,
                expected_version,
                actual_version,
            } => match (expected_version, actual_version) {
                (Some(expected), Some(actual)) => {
                    write!(
                        f,
                        "Concurrent modification detected for {}/{}/{}: expected version {}, found {}",
                        tenant_id, resource_type, id, expected, actual
                    )
                }
                _ => {
                    write!(
                        f,
                        "Concurrent modification detected for {}/{}/{}",
                        tenant_id, resource_type, id
                    )
                }
            },
            StorageError::Unavailable {
                message,
                retry_after,
            } => {
                if let Some(duration) = retry_after {
                    write!(
                        f,
                        "Storage unavailable: {} (retry after {:?})",
                        message, duration
                    )
                } else {
                    write!(f, "Storage unavailable: {}", message)
                }
            }
            StorageError::PermissionDenied {
                operation,
                resource,
            } => {
                write!(f, "Permission denied: {} on {}", operation, resource)
            }
            StorageError::Timeout {
                operation,
                duration,
            } => {
                write!(f, "Timeout during {} after {:?}", operation, duration)
            }
            StorageError::DataCorruption {
                tenant_id,
                resource_type,
                id,
                details,
            } => {
                if let Some(resource_id) = id {
                    write!(
                        f,
                        "Data corruption in {}/{}/{}: {}",
                        tenant_id, resource_type, resource_id, details
                    )
                } else {
                    write!(
                        f,
                        "Data corruption in {}/{}: {}",
                        tenant_id, resource_type, details
                    )
                }
            }
            StorageError::Configuration { message, parameter } => {
                if let Some(param) = parameter {
                    write!(f, "Configuration error: {} (parameter: {})", message, param)
                } else {
                    write!(f, "Configuration error: {}", message)
                }
            }
            StorageError::Network { message, endpoint } => {
                if let Some(ep) = endpoint {
                    write!(f, "Network error: {} (endpoint: {})", message, ep)
                } else {
                    write!(f, "Network error: {}", message)
                }
            }
            StorageError::Serialization { message, data_type } => {
                if let Some(dtype) = data_type {
                    write!(f, "Serialization error: {} (type: {})", message, dtype)
                } else {
                    write!(f, "Serialization error: {}", message)
                }
            }
            StorageError::Internal { message, .. } => {
                write!(f, "Internal storage error: {}", message)
            }
        }
    }
}

impl std::error::Error for StorageError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            StorageError::Internal { source, .. } => source
                .as_ref()
                .map(|e| e.as_ref() as &(dyn std::error::Error + 'static)),
            _ => None,
        }
    }
}

impl StorageError {
    /// Create a new ResourceNotFound error.
    pub fn resource_not_found(
        tenant_id: impl Into<String>,
        resource_type: impl Into<String>,
        id: impl Into<String>,
    ) -> Self {
        Self::ResourceNotFound {
            tenant_id: tenant_id.into(),
            resource_type: resource_type.into(),
            id: id.into(),
        }
    }

    /// Create a new ResourceAlreadyExists error.
    pub fn resource_already_exists(
        tenant_id: impl Into<String>,
        resource_type: impl Into<String>,
        id: impl Into<String>,
    ) -> Self {
        Self::ResourceAlreadyExists {
            tenant_id: tenant_id.into(),
            resource_type: resource_type.into(),
            id: id.into(),
        }
    }

    /// Create a new InvalidData error.
    pub fn invalid_data(message: impl Into<String>) -> Self {
        Self::InvalidData {
            message: message.into(),
            cause: None,
        }
    }

    /// Create a new InvalidData error with a cause.
    pub fn invalid_data_with_cause(message: impl Into<String>, cause: impl Into<String>) -> Self {
        Self::InvalidData {
            message: message.into(),
            cause: Some(cause.into()),
        }
    }

    /// Create a new TenantNotFound error.
    pub fn tenant_not_found(tenant_id: impl Into<String>) -> Self {
        Self::TenantNotFound {
            tenant_id: tenant_id.into(),
        }
    }

    /// Create a new InvalidQuery error.
    pub fn invalid_query(message: impl Into<String>) -> Self {
        Self::InvalidQuery {
            message: message.into(),
            attribute: None,
            value: None,
        }
    }

    /// Create a new CapacityExceeded error.
    pub fn capacity_exceeded(message: impl Into<String>) -> Self {
        Self::CapacityExceeded {
            message: message.into(),
            current_count: None,
            limit: None,
        }
    }

    /// Create a new ConcurrentModification error.
    pub fn concurrent_modification(
        tenant_id: impl Into<String>,
        resource_type: impl Into<String>,
        id: impl Into<String>,
    ) -> Self {
        Self::ConcurrentModification {
            tenant_id: tenant_id.into(),
            resource_type: resource_type.into(),
            id: id.into(),
            expected_version: None,
            actual_version: None,
        }
    }

    /// Create a new Unavailable error.
    pub fn unavailable(message: impl Into<String>) -> Self {
        Self::Unavailable {
            message: message.into(),
            retry_after: None,
        }
    }

    /// Create a new PermissionDenied error.
    pub fn permission_denied(operation: impl Into<String>, resource: impl Into<String>) -> Self {
        Self::PermissionDenied {
            operation: operation.into(),
            resource: resource.into(),
        }
    }

    /// Create a new Timeout error.
    pub fn timeout(operation: impl Into<String>, duration: std::time::Duration) -> Self {
        Self::Timeout {
            operation: operation.into(),
            duration,
        }
    }

    /// Create a new DataCorruption error.
    pub fn data_corruption(
        tenant_id: impl Into<String>,
        resource_type: impl Into<String>,
        details: impl Into<String>,
    ) -> Self {
        Self::DataCorruption {
            tenant_id: tenant_id.into(),
            resource_type: resource_type.into(),
            id: None,
            details: details.into(),
        }
    }

    /// Create a new Configuration error.
    pub fn configuration(message: impl Into<String>) -> Self {
        Self::Configuration {
            message: message.into(),
            parameter: None,
        }
    }

    /// Create a new Network error.
    pub fn network(message: impl Into<String>) -> Self {
        Self::Network {
            message: message.into(),
            endpoint: None,
        }
    }

    /// Create a new Serialization error.
    pub fn serialization(message: impl Into<String>) -> Self {
        Self::Serialization {
            message: message.into(),
            data_type: None,
        }
    }

    /// Create a new Internal error.
    pub fn internal(message: impl Into<String>) -> Self {
        Self::Internal {
            message: message.into(),
            source: None,
        }
    }

    /// Create a new Internal error with a source error.
    pub fn internal_with_source(
        message: impl Into<String>,
        source: Box<dyn std::error::Error + Send + Sync>,
    ) -> Self {
        Self::Internal {
            message: message.into(),
            source: Some(source),
        }
    }

    /// Check if this error indicates a resource was not found.
    pub fn is_not_found(&self) -> bool {
        matches!(self, StorageError::ResourceNotFound { .. })
    }

    /// Check if this error indicates a conflict (resource already exists or concurrent modification).
    pub fn is_conflict(&self) -> bool {
        matches!(
            self,
            StorageError::ResourceAlreadyExists { .. }
                | StorageError::ConcurrentModification { .. }
        )
    }

    /// Check if this error indicates a temporary failure that might succeed on retry.
    pub fn is_temporary(&self) -> bool {
        matches!(
            self,
            StorageError::Unavailable { .. }
                | StorageError::Timeout { .. }
                | StorageError::Network { .. }
        )
    }

    /// Check if this error indicates invalid input data.
    pub fn is_invalid_input(&self) -> bool {
        matches!(
            self,
            StorageError::InvalidData { .. } | StorageError::InvalidQuery { .. }
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_storage_error_display() {
        let error = StorageError::resource_not_found("tenant1", "User", "123");
        assert_eq!(error.to_string(), "Resource not found: tenant1/User/123");

        let error = StorageError::invalid_data("malformed JSON");
        assert_eq!(error.to_string(), "Invalid data: malformed JSON");

        let error = StorageError::capacity_exceeded("disk full");
        assert_eq!(error.to_string(), "Capacity exceeded: disk full");
    }

    #[test]
    fn test_storage_error_type_checks() {
        let not_found = StorageError::resource_not_found("tenant1", "User", "123");
        assert!(not_found.is_not_found());
        assert!(!not_found.is_conflict());
        assert!(!not_found.is_temporary());

        let conflict = StorageError::resource_already_exists("tenant1", "User", "123");
        assert!(!conflict.is_not_found());
        assert!(conflict.is_conflict());
        assert!(!conflict.is_temporary());

        let timeout = StorageError::timeout("query", std::time::Duration::from_secs(30));
        assert!(!timeout.is_not_found());
        assert!(!timeout.is_conflict());
        assert!(timeout.is_temporary());

        let invalid = StorageError::invalid_data("bad format");
        assert!(invalid.is_invalid_input());
    }

    #[test]
    fn test_storage_error_constructors() {
        let error = StorageError::invalid_data_with_cause("parse error", "unexpected token");
        if let StorageError::InvalidData { message, cause } = error {
            assert_eq!(message, "parse error");
            assert_eq!(cause, Some("unexpected token".to_string()));
        } else {
            panic!("Expected InvalidData error");
        }

        let error = StorageError::concurrent_modification("tenant1", "User", "123");
        if let StorageError::ConcurrentModification {
            tenant_id,
            resource_type,
            id,
            ..
        } = error
        {
            assert_eq!(tenant_id, "tenant1");
            assert_eq!(resource_type, "User");
            assert_eq!(id, "123");
        } else {
            panic!("Expected ConcurrentModification error");
        }
    }
}
