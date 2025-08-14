//! In-memory storage implementation for SCIM resources.
//!
//! This module provides a thread-safe in-memory implementation of the `StorageProvider`
//! trait using HashMap and RwLock for concurrent access. It's designed for testing,
//! development, and scenarios where persistence is not required.
//!
//! # Features
//!
//! * Thread-safe concurrent access with async RwLock
//! * Automatic tenant isolation through hierarchical key structure
//! * Efficient querying with attribute-based searches
//! * Consistent ordering for list operations
//! * No external dependencies beyond standard library
//!
//! # Performance Characteristics
//!
//! * PUT/GET/DELETE: O(1) average case
//! * LIST with pagination: O(n) where n is total resources in prefix
//! * FIND_BY_ATTRIBUTE: O(n) with JSON parsing overhead
//! * EXISTS/COUNT: O(1) and O(n) respectively
//!
//! # Example Usage
//!
//! ```rust
//! use scim_server::storage::{InMemoryStorage, StorageProvider, StorageKey};
//! use serde_json::json;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let storage = InMemoryStorage::new();
//!
//! // Store a user
//! let key = StorageKey::new("tenant1", "User", "user123");
//! let user_data = json!({
//!     "id": "user123",
//!     "userName": "john.doe",
//!     "displayName": "John Doe",
//!     "emails": [{"value": "john@example.com", "primary": true}]
//! });
//!
//! let stored = storage.put(key.clone(), user_data).await?;
//! println!("Stored: {}", stored);
//!
//! // Retrieve the user
//! let retrieved = storage.get(key.clone()).await?;
//! assert!(retrieved.is_some());
//!
//! // Search by email
//! let prefix = StorageKey::prefix("tenant1", "User");
//! let found = storage.find_by_attribute(prefix, "emails.0.value", "john@example.com").await?;
//! assert_eq!(found.len(), 1);
//!
//! // Delete the user
//! let was_deleted = storage.delete(key).await?;
//! assert!(was_deleted);
//! # Ok(())
//! # }
//! ```

use crate::storage::{StorageError, StorageKey, StoragePrefix, StorageProvider};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Thread-safe in-memory storage implementation.
///
/// Uses a nested HashMap structure for efficient storage and retrieval:
/// `tenant_id` → `resource_type` → `resource_id` → `data`
///
/// All operations are async and thread-safe using tokio's RwLock.
#[derive(Clone)]
pub struct InMemoryStorage {
    // Structure: tenant_id -> resource_type -> resource_id -> data
    data: Arc<RwLock<HashMap<String, HashMap<String, HashMap<String, Value>>>>>,
}

impl InMemoryStorage {
    /// Create a new empty in-memory storage instance.
    pub fn new() -> Self {
        Self {
            data: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Get storage statistics for debugging and monitoring.
    pub async fn stats(&self) -> InMemoryStorageStats {
        let data_guard = self.data.read().await;
        let mut tenant_count = 0;
        let mut resource_type_count = 0;
        let mut total_resources = 0;

        for (_, tenant_data) in data_guard.iter() {
            tenant_count += 1;
            for (_, type_data) in tenant_data.iter() {
                resource_type_count += 1;
                total_resources += type_data.len();
            }
        }

        InMemoryStorageStats {
            tenant_count,
            resource_type_count,
            total_resources,
        }
    }

    /// Clear all data (useful for testing).
    pub async fn clear(&self) {
        let mut data_guard = self.data.write().await;
        data_guard.clear();
    }

    /// Get all tenant IDs currently in storage.
    pub async fn list_tenants(&self) -> Vec<String> {
        let data_guard = self.data.read().await;
        data_guard.keys().cloned().collect()
    }

    /// Get all resource types for a specific tenant.
    pub async fn list_resource_types(&self, tenant_id: &str) -> Vec<String> {
        let data_guard = self.data.read().await;
        data_guard
            .get(tenant_id)
            .map(|tenant_data| tenant_data.keys().cloned().collect())
            .unwrap_or_default()
    }

    /// Extract a nested attribute value from JSON data using dot notation.
    fn extract_attribute_value(data: &Value, attribute_path: &str) -> Option<String> {
        let parts: Vec<&str> = attribute_path.split('.').collect();
        let mut current = data;

        for part in parts {
            if let Ok(index) = part.parse::<usize>() {
                // Array index
                current = current.get(index)?;
            } else {
                // Object key
                current = current.get(part)?;
            }
        }

        // Convert the final value to string for comparison
        match current {
            Value::String(s) => Some(s.clone()),
            Value::Number(n) => Some(n.to_string()),
            Value::Bool(b) => Some(b.to_string()),
            _ => current.as_str().map(|s| s.to_string()),
        }
    }
}

impl Default for InMemoryStorage {
    fn default() -> Self {
        Self::new()
    }
}

impl StorageProvider for InMemoryStorage {
    type Error = StorageError;

    async fn put(&self, key: StorageKey, data: Value) -> Result<Value, Self::Error> {
        let mut data_guard = self.data.write().await;

        // Ensure the nested structure exists
        let tenant_data = data_guard
            .entry(key.tenant_id().to_string())
            .or_insert_with(HashMap::new);

        let type_data = tenant_data
            .entry(key.resource_type().to_string())
            .or_insert_with(HashMap::new);

        // Store the data
        type_data.insert(key.resource_id().to_string(), data.clone());

        // Return the stored data (in this implementation, it's unchanged)
        Ok(data)
    }

    async fn get(&self, key: StorageKey) -> Result<Option<Value>, Self::Error> {
        let data_guard = self.data.read().await;

        let result = data_guard
            .get(key.tenant_id())
            .and_then(|tenant_data| tenant_data.get(key.resource_type()))
            .and_then(|type_data| type_data.get(key.resource_id()))
            .cloned();

        Ok(result)
    }

    async fn delete(&self, key: StorageKey) -> Result<bool, Self::Error> {
        let mut data_guard = self.data.write().await;

        let existed = if let Some(tenant_data) = data_guard.get_mut(key.tenant_id()) {
            if let Some(type_data) = tenant_data.get_mut(key.resource_type()) {
                type_data.remove(key.resource_id()).is_some()
            } else {
                false
            }
        } else {
            false
        };

        Ok(existed)
    }

    async fn list(
        &self,
        prefix: StoragePrefix,
        offset: usize,
        limit: usize,
    ) -> Result<Vec<(StorageKey, Value)>, Self::Error> {
        if limit == 0 {
            return Ok(Vec::new());
        }

        let data_guard = self.data.read().await;

        let type_data = match data_guard
            .get(prefix.tenant_id())
            .and_then(|tenant_data| tenant_data.get(prefix.resource_type()))
        {
            Some(data) => data,
            None => return Ok(Vec::new()),
        };

        // Collect and sort keys for consistent ordering
        let mut keys: Vec<_> = type_data.keys().collect();
        keys.sort();

        // Apply pagination
        let results: Vec<(StorageKey, Value)> = keys
            .into_iter()
            .skip(offset)
            .take(limit)
            .filter_map(|resource_id| {
                type_data.get(resource_id).map(|data| {
                    (
                        StorageKey::new(prefix.tenant_id(), prefix.resource_type(), resource_id),
                        data.clone(),
                    )
                })
            })
            .collect();

        Ok(results)
    }

    async fn find_by_attribute(
        &self,
        prefix: StoragePrefix,
        attribute: &str,
        value: &str,
    ) -> Result<Vec<(StorageKey, Value)>, Self::Error> {
        let data_guard = self.data.read().await;

        let type_data = match data_guard
            .get(prefix.tenant_id())
            .and_then(|tenant_data| tenant_data.get(prefix.resource_type()))
        {
            Some(data) => data,
            None => return Ok(Vec::new()),
        };

        let mut results = Vec::new();

        for (resource_id, resource_data) in type_data {
            if let Some(attr_value) = Self::extract_attribute_value(resource_data, attribute) {
                if attr_value == value {
                    results.push((
                        StorageKey::new(prefix.tenant_id(), prefix.resource_type(), resource_id),
                        resource_data.clone(),
                    ));
                }
            }
        }

        // Sort results by resource ID for consistency
        results.sort_by(|a, b| a.0.resource_id().cmp(b.0.resource_id()));

        Ok(results)
    }

    async fn exists(&self, key: StorageKey) -> Result<bool, Self::Error> {
        let data_guard = self.data.read().await;

        let exists = data_guard
            .get(key.tenant_id())
            .and_then(|tenant_data| tenant_data.get(key.resource_type()))
            .and_then(|type_data| type_data.get(key.resource_id()))
            .is_some();

        Ok(exists)
    }

    async fn count(&self, prefix: StoragePrefix) -> Result<usize, Self::Error> {
        let data_guard = self.data.read().await;

        let count = data_guard
            .get(prefix.tenant_id())
            .and_then(|tenant_data| tenant_data.get(prefix.resource_type()))
            .map(|type_data| type_data.len())
            .unwrap_or(0);

        Ok(count)
    }
}

/// Statistics about the current state of in-memory storage.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InMemoryStorageStats {
    /// Number of tenants with data
    pub tenant_count: usize,
    /// Number of resource types across all tenants
    pub resource_type_count: usize,
    /// Total number of individual resources
    pub total_resources: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[tokio::test]
    async fn test_put_and_get() {
        let storage = InMemoryStorage::new();
        let key = StorageKey::new("tenant1", "User", "123");
        let data = json!({"id": "123", "name": "test"});

        // Put data
        let stored = storage.put(key.clone(), data.clone()).await.unwrap();
        assert_eq!(stored, data);

        // Get data
        let retrieved = storage.get(key).await.unwrap();
        assert_eq!(retrieved, Some(data));
    }

    #[tokio::test]
    async fn test_get_nonexistent() {
        let storage = InMemoryStorage::new();
        let key = StorageKey::new("tenant1", "User", "999");

        let result = storage.get(key).await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_delete() {
        let storage = InMemoryStorage::new();
        let key = StorageKey::new("tenant1", "User", "123");
        let data = json!({"id": "123", "name": "test"});

        // Put data first
        storage.put(key.clone(), data).await.unwrap();

        // Delete should return true
        let deleted = storage.delete(key.clone()).await.unwrap();
        assert!(deleted);

        // Get should return None
        let retrieved = storage.get(key.clone()).await.unwrap();
        assert!(retrieved.is_none());

        // Delete again should return false
        let deleted_again = storage.delete(key).await.unwrap();
        assert!(!deleted_again);
    }

    #[tokio::test]
    async fn test_exists() {
        let storage = InMemoryStorage::new();
        let key = StorageKey::new("tenant1", "User", "123");
        let data = json!({"id": "123", "name": "test"});

        // Should not exist initially
        assert!(!storage.exists(key.clone()).await.unwrap());

        // Put data
        storage.put(key.clone(), data).await.unwrap();

        // Should exist now
        assert!(storage.exists(key.clone()).await.unwrap());

        // Delete data
        storage.delete(key.clone()).await.unwrap();

        // Should not exist anymore
        assert!(!storage.exists(key).await.unwrap());
    }

    #[tokio::test]
    async fn test_list_with_pagination() {
        let storage = InMemoryStorage::new();
        let prefix = StorageKey::prefix("tenant1", "User");

        // Store multiple resources
        for i in 1..=5 {
            let key = StorageKey::new("tenant1", "User", &format!("{}", i));
            let data = json!({"id": i, "name": format!("user{}", i)});
            storage.put(key, data).await.unwrap();
        }

        // Test pagination
        let page1 = storage.list(prefix.clone(), 0, 2).await.unwrap();
        assert_eq!(page1.len(), 2);
        assert_eq!(page1[0].0.resource_id(), "1");
        assert_eq!(page1[1].0.resource_id(), "2");

        let page2 = storage.list(prefix.clone(), 2, 2).await.unwrap();
        assert_eq!(page2.len(), 2);
        assert_eq!(page2[0].0.resource_id(), "3");
        assert_eq!(page2[1].0.resource_id(), "4");

        let page3 = storage.list(prefix, 4, 2).await.unwrap();
        assert_eq!(page3.len(), 1);
        assert_eq!(page3[0].0.resource_id(), "5");
    }

    #[tokio::test]
    async fn test_find_by_attribute() {
        let storage = InMemoryStorage::new();
        let prefix = StorageKey::prefix("tenant1", "User");

        // Store users with different userNames
        let user1 = json!({
            "id": "1",
            "userName": "john.doe",
            "emails": [{"value": "john@example.com", "primary": true}]
        });
        let user2 = json!({
            "id": "2",
            "userName": "jane.doe",
            "emails": [{"value": "jane@example.com", "primary": true}]
        });

        storage
            .put(StorageKey::new("tenant1", "User", "1"), user1)
            .await
            .unwrap();
        storage
            .put(StorageKey::new("tenant1", "User", "2"), user2)
            .await
            .unwrap();

        // Find by userName
        let found = storage
            .find_by_attribute(prefix.clone(), "userName", "john.doe")
            .await
            .unwrap();
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].0.resource_id(), "1");

        // Find by nested email
        let found = storage
            .find_by_attribute(prefix.clone(), "emails.0.value", "jane@example.com")
            .await
            .unwrap();
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].0.resource_id(), "2");

        // Find non-existent
        let found = storage
            .find_by_attribute(prefix, "userName", "nonexistent")
            .await
            .unwrap();
        assert_eq!(found.len(), 0);
    }

    #[tokio::test]
    async fn test_count() {
        let storage = InMemoryStorage::new();
        let prefix = StorageKey::prefix("tenant1", "User");

        // Initially empty
        assert_eq!(storage.count(prefix.clone()).await.unwrap(), 0);

        // Add some resources
        for i in 1..=3 {
            let key = StorageKey::new("tenant1", "User", &format!("{}", i));
            let data = json!({"id": i});
            storage.put(key, data).await.unwrap();
        }

        assert_eq!(storage.count(prefix).await.unwrap(), 3);
    }

    #[tokio::test]
    async fn test_tenant_isolation() {
        let storage = InMemoryStorage::new();

        // Store same resource ID in different tenants
        let key1 = StorageKey::new("tenant1", "User", "123");
        let key2 = StorageKey::new("tenant2", "User", "123");
        let data1 = json!({"tenant": "1"});
        let data2 = json!({"tenant": "2"});

        storage.put(key1.clone(), data1.clone()).await.unwrap();
        storage.put(key2.clone(), data2.clone()).await.unwrap();

        // Verify isolation
        assert_eq!(storage.get(key1).await.unwrap(), Some(data1));
        assert_eq!(storage.get(key2).await.unwrap(), Some(data2));

        // Verify counts are isolated
        assert_eq!(
            storage
                .count(StorageKey::prefix("tenant1", "User"))
                .await
                .unwrap(),
            1
        );
        assert_eq!(
            storage
                .count(StorageKey::prefix("tenant2", "User"))
                .await
                .unwrap(),
            1
        );
    }

    #[tokio::test]
    async fn test_stats() {
        let storage = InMemoryStorage::new();

        // Initially empty
        let stats = storage.stats().await;
        assert_eq!(stats.tenant_count, 0);
        assert_eq!(stats.resource_type_count, 0);
        assert_eq!(stats.total_resources, 0);

        // Add some data
        storage
            .put(StorageKey::new("tenant1", "User", "1"), json!({"id": "1"}))
            .await
            .unwrap();
        storage
            .put(StorageKey::new("tenant1", "User", "2"), json!({"id": "2"}))
            .await
            .unwrap();
        storage
            .put(StorageKey::new("tenant1", "Group", "1"), json!({"id": "1"}))
            .await
            .unwrap();
        storage
            .put(StorageKey::new("tenant2", "User", "1"), json!({"id": "1"}))
            .await
            .unwrap();

        let stats = storage.stats().await;
        assert_eq!(stats.tenant_count, 2);
        assert_eq!(stats.resource_type_count, 3); // tenant1:User, tenant1:Group, tenant2:User
        assert_eq!(stats.total_resources, 4);
    }

    #[tokio::test]
    async fn test_clear() {
        let storage = InMemoryStorage::new();

        // Add some data
        storage
            .put(StorageKey::new("tenant1", "User", "1"), json!({"id": "1"}))
            .await
            .unwrap();

        // Verify data exists
        assert_eq!(
            storage
                .count(StorageKey::prefix("tenant1", "User"))
                .await
                .unwrap(),
            1
        );

        // Clear all data
        storage.clear().await;

        // Verify data is gone
        assert_eq!(
            storage
                .count(StorageKey::prefix("tenant1", "User"))
                .await
                .unwrap(),
            0
        );
        let stats = storage.stats().await;
        assert_eq!(stats.total_resources, 0);
    }

    #[tokio::test]
    async fn test_list_tenants_and_resource_types() {
        let storage = InMemoryStorage::new();

        // Add data for multiple tenants and types
        storage
            .put(StorageKey::new("tenant1", "User", "1"), json!({"id": "1"}))
            .await
            .unwrap();
        storage
            .put(StorageKey::new("tenant1", "Group", "1"), json!({"id": "1"}))
            .await
            .unwrap();
        storage
            .put(StorageKey::new("tenant2", "User", "1"), json!({"id": "1"}))
            .await
            .unwrap();

        // Test list_tenants
        let mut tenants = storage.list_tenants().await;
        tenants.sort();
        assert_eq!(tenants, vec!["tenant1", "tenant2"]);

        // Test list_resource_types
        let mut types1 = storage.list_resource_types("tenant1").await;
        types1.sort();
        assert_eq!(types1, vec!["Group", "User"]);

        let types2 = storage.list_resource_types("tenant2").await;
        assert_eq!(types2, vec!["User"]);

        // Non-existent tenant
        let types_none = storage.list_resource_types("nonexistent").await;
        assert!(types_none.is_empty());
    }

    #[tokio::test]
    async fn test_extract_attribute_value() {
        let data = json!({
            "userName": "john.doe",
            "emails": [
                {"value": "john@example.com", "primary": true},
                {"value": "john.doe@work.com", "primary": false}
            ],
            "address": {
                "street": "123 Main St",
                "city": "Anytown"
            }
        });

        // Simple attribute
        assert_eq!(
            InMemoryStorage::extract_attribute_value(&data, "userName"),
            Some("john.doe".to_string())
        );

        // Nested object
        assert_eq!(
            InMemoryStorage::extract_attribute_value(&data, "address.city"),
            Some("Anytown".to_string())
        );

        // Array index
        assert_eq!(
            InMemoryStorage::extract_attribute_value(&data, "emails.0.value"),
            Some("john@example.com".to_string())
        );

        // Boolean value
        assert_eq!(
            InMemoryStorage::extract_attribute_value(&data, "emails.0.primary"),
            Some("true".to_string())
        );

        // Non-existent path
        assert_eq!(
            InMemoryStorage::extract_attribute_value(&data, "nonexistent"),
            None
        );

        // Invalid array index
        assert_eq!(
            InMemoryStorage::extract_attribute_value(&data, "emails.99.value"),
            None
        );
    }
}
