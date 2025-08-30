//! Storage abstraction layer for SCIM resources.
//!
//! This module provides a clean separation between storage concerns and SCIM protocol logic.
//! The `StorageProvider` trait defines pure data storage operations that are protocol-agnostic,
//! allowing for pluggable storage backends while keeping SCIM-specific logic in the provider layer.
//!
//! # Architecture
//!
//! The storage layer is responsible for:
//! - Pure PUT/GET/DELETE operations on JSON data
//! - Tenant isolation and data organization
//! - Basic querying and filtering
//! - Data persistence and retrieval
//!
//! The storage layer is NOT responsible for:
//! - SCIM metadata generation (timestamps, versions, etc.)
//! - SCIM validation rules
//! - Business logic (limits, permissions, etc.)
//! - Protocol-specific transformations
//!
//! # Design Philosophy
//!
//! This interface follows the principle that at the storage level, CREATE and UPDATE are
//! the same operation - you're just putting data at a location. The distinction between
//! "create" vs "update" is business logic that belongs in the SCIM provider layer.
//!
//! # Example Usage
//!
//! ```rust
//! use scim_server::storage::{StorageProvider, StorageKey, InMemoryStorage};
//! use serde_json::json;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let storage = InMemoryStorage::new();
//!
//! // Put a resource (works for both create and update)
//! let key = StorageKey::new("tenant1", "User", "123");
//! let user_data = json!({
//!     "id": "123",
//!     "userName": "john.doe",
//!     "displayName": "John Doe"
//! });
//! let stored_data = storage.put(key.clone(), user_data).await?;
//!
//! // Get the resource
//! let retrieved = storage.get(key.clone()).await?;
//! assert!(retrieved.is_some());
//!
//! // Delete the resource
//! let was_deleted = storage.delete(key).await?;
//! assert!(was_deleted);
//! # Ok(())
//! # }
//! ```

pub mod errors;
pub mod in_memory;

pub use errors::StorageError;
pub use in_memory::{InMemoryStorage, InMemoryStorageStats};

use serde_json::Value;
use std::fmt;
use std::future::Future;

/// A hierarchical key for identifying resources in storage.
///
/// Resources are organized as: `tenant_id` → `resource_type` → `resource_id`
/// This provides natural tenant isolation and efficient querying.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct StorageKey {
    tenant_id: String,
    resource_type: String,
    resource_id: String,
}

impl StorageKey {
    /// Create a new storage key.
    pub fn new(
        tenant_id: impl Into<String>,
        resource_type: impl Into<String>,
        resource_id: impl Into<String>,
    ) -> Self {
        Self {
            tenant_id: tenant_id.into(),
            resource_type: resource_type.into(),
            resource_id: resource_id.into(),
        }
    }

    /// Get the tenant ID.
    pub fn tenant_id(&self) -> &str {
        &self.tenant_id
    }

    /// Get the resource type.
    pub fn resource_type(&self) -> &str {
        &self.resource_type
    }

    /// Get the resource ID.
    pub fn resource_id(&self) -> &str {
        &self.resource_id
    }

    /// Create a prefix key for listing resources of a type within a tenant.
    pub fn prefix(tenant_id: impl Into<String>, resource_type: impl Into<String>) -> StoragePrefix {
        StoragePrefix {
            tenant_id: tenant_id.into(),
            resource_type: resource_type.into(),
        }
    }
}

impl fmt::Display for StorageKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}/{}/{}",
            self.tenant_id, self.resource_type, self.resource_id
        )
    }
}

/// A prefix for querying resources by tenant and type.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoragePrefix {
    tenant_id: String,
    resource_type: String,
}

impl StoragePrefix {
    /// Get the tenant ID.
    pub fn tenant_id(&self) -> &str {
        &self.tenant_id
    }

    /// Get the resource type.
    pub fn resource_type(&self) -> &str {
        &self.resource_type
    }
}

impl fmt::Display for StoragePrefix {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}/{}", self.tenant_id, self.resource_type)
    }
}

/// Core trait for storage providers that handle pure data persistence operations.
///
/// This trait defines a protocol-agnostic interface for storing and retrieving JSON data
/// with tenant isolation. Implementations should focus solely on data persistence and
/// retrieval without any SCIM-specific logic.
///
/// # Design Principles
///
/// - **PUT/GET/DELETE Model**: Simple, fundamental operations
/// - **PUT Returns Data**: Supports SCIM requirement to return resource state after operations
/// - **DELETE Returns Boolean**: Indicates whether resource existed (for proper HTTP status codes)
/// - **Tenant Isolation**: All operations are scoped to a specific tenant via StorageKey
/// - **Protocol Agnostic**: No awareness of SCIM structures or semantics
/// - **Async First**: All operations return futures for scalability
/// - **Error Transparency**: Storage errors are clearly separated from protocol errors
///
/// # Key Design Decisions
///
/// - **No separate CREATE/UPDATE**: Both are just PUT operations. Business logic determines
///   whether this should be treated as create vs update.
/// - **PUT returns stored data**: This enables SCIM providers to return the complete resource
///   state after modifications without a separate GET call.
/// - **DELETE returns boolean**: Allows proper HTTP status code handling (204 vs 404).
pub trait StorageProvider: Send + Sync {
    /// The error type returned by storage operations.
    type Error: std::error::Error + Send + Sync + 'static;

    /// Store data at the specified key and return the stored data.
    ///
    /// # Arguments
    /// * `key` - The storage key identifying the resource location
    /// * `data` - The JSON data to store
    ///
    /// # Returns
    /// The data that was actually stored (may include storage-level metadata).
    ///
    /// # Behavior
    /// - If a resource with the same key already exists, it is completely replaced
    /// - The storage implementation should ensure atomic operations where possible
    /// - No validation is performed on the data structure
    /// - The returned data should be exactly what would be retrieved by `get()`
    fn put(
        &self,
        key: StorageKey,
        data: Value,
    ) -> impl Future<Output = Result<Value, Self::Error>> + Send;

    /// Retrieve data by key.
    ///
    /// # Arguments
    /// * `key` - The storage key identifying the resource
    ///
    /// # Returns
    /// `Some(data)` if the resource exists, `None` if it doesn't exist.
    fn get(
        &self,
        key: StorageKey,
    ) -> impl Future<Output = Result<Option<Value>, Self::Error>> + Send;

    /// Delete data by key.
    ///
    /// # Arguments
    /// * `key` - The storage key identifying the resource
    ///
    /// # Returns
    /// `true` if the resource was deleted, `false` if it didn't exist.
    ///
    /// # Note
    /// This follows SCIM/HTTP semantics where DELETE operations don't return resource data.
    /// The boolean return value allows proper HTTP status code selection (204 vs 404).
    fn delete(&self, key: StorageKey) -> impl Future<Output = Result<bool, Self::Error>> + Send;

    /// List resources matching a prefix with pagination.
    ///
    /// # Arguments
    /// * `prefix` - The storage prefix (tenant + resource type)
    /// * `offset` - The number of resources to skip (0-based)
    /// * `limit` - The maximum number of resources to return
    ///
    /// # Returns
    /// A vector of (key, data) pairs.
    ///
    /// # Behavior
    /// - Results should be consistently ordered (e.g., by resource ID)
    /// - If `offset` exceeds the total count, an empty vector should be returned
    /// - If `limit` is 0, an empty vector should be returned
    fn list(
        &self,
        prefix: StoragePrefix,
        offset: usize,
        limit: usize,
    ) -> impl Future<Output = Result<Vec<(StorageKey, Value)>, Self::Error>> + Send;

    /// Find resources by a specific attribute value.
    ///
    /// # Arguments
    /// * `prefix` - The storage prefix (tenant + resource type)
    /// * `attribute` - The JSON path of the attribute to search (e.g., "userName", "emails.0.value")
    /// * `value` - The exact value to match
    ///
    /// # Returns
    /// A vector of (key, data) pairs for matching resources.
    ///
    /// # Behavior
    /// - Performs exact string matching on the specified attribute
    /// - Supports nested attributes using dot notation
    /// - Returns all matching resources (no pagination)
    /// - Empty vector if no matches found
    fn find_by_attribute(
        &self,
        prefix: StoragePrefix,
        attribute: &str,
        value: &str,
    ) -> impl Future<Output = Result<Vec<(StorageKey, Value)>, Self::Error>> + Send;

    /// Check if a resource exists.
    ///
    /// # Arguments
    /// * `key` - The storage key identifying the resource
    ///
    /// # Returns
    /// `true` if the resource exists, `false` if it doesn't.
    ///
    /// # Performance Note
    /// This should be more efficient than `get()` as it doesn't need to return data.
    fn exists(&self, key: StorageKey) -> impl Future<Output = Result<bool, Self::Error>> + Send;

    /// Count the total number of resources matching a prefix.
    ///
    /// # Arguments
    /// * `prefix` - The storage prefix (tenant + resource type)
    ///
    /// # Returns
    /// The total count of matching resources.
    fn count(
        &self,
        prefix: StoragePrefix,
    ) -> impl Future<Output = Result<usize, Self::Error>> + Send;

    /// List all tenant IDs that currently have data in storage.
    ///
    /// Returns tenant IDs for all tenants that contain at least one resource of any type.
    /// This method enables dynamic tenant discovery without requiring hardcoded tenant patterns.
    ///
    /// # Returns
    ///
    /// A vector of tenant ID strings. Empty vector if no tenants have data.
    ///
    /// # Errors
    ///
    /// Returns storage-specific errors if the discovery operation fails.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use scim_server::storage::{StorageProvider, InMemoryStorage};
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let storage = InMemoryStorage::new();
    /// let tenants = storage.list_tenants().await?;
    /// println!("Found {} tenants", tenants.len());
    /// # Ok(())
    /// # }
    /// ```
    fn list_tenants(&self) -> impl Future<Output = Result<Vec<String>, Self::Error>> + Send;

    /// List all resource types for a specific tenant.
    ///
    /// Returns resource type names (e.g., "User", "Group") that exist within the specified
    /// tenant. Only resource types with at least one stored resource are included.
    ///
    /// # Arguments
    ///
    /// * `tenant_id` - The tenant ID to query for resource types
    ///
    /// # Returns
    ///
    /// A vector of resource type strings. Empty vector if tenant doesn't exist or has no resources.
    ///
    /// # Errors
    ///
    /// Returns storage-specific errors if the query operation fails.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use scim_server::storage::{StorageProvider, InMemoryStorage};
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let storage = InMemoryStorage::new();
    /// let types = storage.list_resource_types("tenant1").await?;
    /// for resource_type in types {
    ///     println!("Tenant has resource type: {}", resource_type);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    fn list_resource_types(
        &self,
        tenant_id: &str,
    ) -> impl Future<Output = Result<Vec<String>, Self::Error>> + Send;

    /// List all resource types across all tenants.
    ///
    /// Returns a deduplicated collection of all resource type names found across all tenants
    /// in storage. This provides a global view of resource types without tenant boundaries.
    ///
    /// # Returns
    ///
    /// A vector of unique resource type strings. Empty vector if no resources exist.
    ///
    /// # Errors
    ///
    /// Returns storage-specific errors if the discovery operation fails.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use scim_server::storage::{StorageProvider, InMemoryStorage};
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let storage = InMemoryStorage::new();
    /// let all_types = storage.list_all_resource_types().await?;
    /// println!("System supports {} resource types", all_types.len());
    /// # Ok(())
    /// # }
    /// ```
    fn list_all_resource_types(&self) -> impl Future<Output = Result<Vec<String>, Self::Error>> + Send;

    /// Clear all data from storage.
    ///
    /// Removes all resources from all tenants, effectively resetting the storage to an empty state.
    /// This operation is primarily intended for testing scenarios and should be used with caution
    /// in production environments.
    ///
    /// # Returns
    ///
    /// `Ok(())` on successful clearing, or a storage-specific error on failure.
    ///
    /// # Errors
    ///
    /// Returns storage-specific errors if the clear operation fails partially or completely.
    ///
    /// # Behavior
    ///
    /// - Removes all resources from all tenants atomically where possible
    /// - After successful clearing, [`list_tenants`] should return an empty vector
    /// - Primarily intended for testing scenarios
    ///
    /// # Examples
    ///
    /// ```rust
    /// use scim_server::storage::{StorageProvider, InMemoryStorage};
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let storage = InMemoryStorage::new();
    /// // ... populate storage with data ...
    /// storage.clear().await?;
    /// let tenants = storage.list_tenants().await?;
    /// assert_eq!(tenants.len(), 0);
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// [`list_tenants`]: Self::list_tenants
    fn clear(&self) -> impl Future<Output = Result<(), Self::Error>> + Send;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_storage_key() {
        let key = StorageKey::new("tenant1", "User", "123");
        assert_eq!(key.tenant_id(), "tenant1");
        assert_eq!(key.resource_type(), "User");
        assert_eq!(key.resource_id(), "123");
        assert_eq!(key.to_string(), "tenant1/User/123");
    }

    #[tokio::test]
    async fn test_storage_prefix() {
        let prefix = StorageKey::prefix("tenant1", "User");
        assert_eq!(prefix.tenant_id(), "tenant1");
        assert_eq!(prefix.resource_type(), "User");
        assert_eq!(prefix.to_string(), "tenant1/User");
    }
}
