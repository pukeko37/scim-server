//! Parameterized tests for storage providers.
//!
//! This module contains tests that work with any StorageProvider implementation,
//! allowing us to test both InMemoryStorage and SqliteStorage with the same test suite.

use super::{StorageError, StorageKey, StorageProvider};
use serde_json::json;

/// Test suite for any StorageProvider implementation.
///
/// This function contains all the tests that should pass for any correct
/// implementation of the StorageProvider trait.
pub async fn test_storage_provider<S>(storage: S)
where
    S: StorageProvider<Error = StorageError> + Send + Sync,
{
    test_put_and_get(&storage).await;
    test_get_nonexistent(&storage).await;
    test_delete(&storage).await;
    test_exists(&storage).await;
    test_list_with_pagination(&storage).await;
    test_find_by_attribute(&storage).await;
    test_count(&storage).await;
    test_tenant_isolation(&storage).await;
    test_stats(&storage).await;
    test_clear(&storage).await;
    test_list_tenants_and_resource_types(&storage).await;
}

async fn test_put_and_get<S>(storage: &S)
where
    S: StorageProvider<Error = StorageError> + Send + Sync,
{
    storage.clear().await.unwrap();
    let key = StorageKey::new("tenant1", "User", "123");
    let data = json!({"id": "123", "name": "test"});

    // Put data
    let stored = storage.put(key.clone(), data.clone()).await.unwrap();
    assert_eq!(stored, data);

    // Get data
    let retrieved = storage.get(key).await.unwrap();
    assert_eq!(retrieved, Some(data));
}

async fn test_get_nonexistent<S>(storage: &S)
where
    S: StorageProvider<Error = StorageError> + Send + Sync,
{
    storage.clear().await.unwrap();
    let key = StorageKey::new("tenant1", "User", "999");

    let result = storage.get(key).await.unwrap();
    assert!(result.is_none());
}

async fn test_delete<S>(storage: &S)
where
    S: StorageProvider<Error = StorageError> + Send + Sync,
{
    storage.clear().await.unwrap();
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

async fn test_exists<S>(storage: &S)
where
    S: StorageProvider<Error = StorageError> + Send + Sync,
{
    storage.clear().await.unwrap();
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

async fn test_list_with_pagination<S>(storage: &S)
where
    S: StorageProvider<Error = StorageError> + Send + Sync,
{
    storage.clear().await.unwrap();
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

async fn test_find_by_attribute<S>(storage: &S)
where
    S: StorageProvider<Error = StorageError> + Send + Sync,
{
    storage.clear().await.unwrap();
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

async fn test_count<S>(storage: &S)
where
    S: StorageProvider<Error = StorageError> + Send + Sync,
{
    storage.clear().await.unwrap();
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

async fn test_tenant_isolation<S>(storage: &S)
where
    S: StorageProvider<Error = StorageError> + Send + Sync,
{
    storage.clear().await.unwrap();
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

async fn test_stats<S>(storage: &S)
where
    S: StorageProvider<Error = StorageError> + Send + Sync,
{
    storage.clear().await.unwrap();
    // Initially empty
    let stats = storage.stats().await.unwrap();
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

    let stats = storage.stats().await.unwrap();
    assert_eq!(stats.tenant_count, 2);
    assert_eq!(stats.resource_type_count, 3); // tenant1:User, tenant1:Group, tenant2:User
    assert_eq!(stats.total_resources, 4);
}

async fn test_clear<S>(storage: &S)
where
    S: StorageProvider<Error = StorageError> + Send + Sync,
{
    storage.clear().await.unwrap();
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
    storage.clear().await.unwrap();

    // Verify data is gone
    assert_eq!(
        storage
            .count(StorageKey::prefix("tenant1", "User"))
            .await
            .unwrap(),
        0
    );
    let stats = storage.stats().await.unwrap();
    assert_eq!(stats.total_resources, 0);
}

async fn test_list_tenants_and_resource_types<S>(storage: &S)
where
    S: StorageProvider<Error = StorageError> + Send + Sync,
{
    storage.clear().await.unwrap();
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
    let mut tenants = storage.list_tenants().await.unwrap();
    tenants.sort();
    assert_eq!(tenants, vec!["tenant1", "tenant2"]);

    // Test list_resource_types
    let mut types1 = storage.list_resource_types("tenant1").await.unwrap();
    types1.sort();
    assert_eq!(types1, vec!["Group", "User"]);

    let types2 = storage.list_resource_types("tenant2").await.unwrap();
    assert_eq!(types2, vec!["User"]);

    // Non-existent tenant
    let types_none = storage.list_resource_types("nonexistent").await.unwrap();
    assert!(types_none.is_empty());
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::{InMemoryStorage, SqliteStorage};

    #[tokio::test]
    async fn test_in_memory_storage() {
        let storage = InMemoryStorage::new();
        test_storage_provider(storage).await;
    }

    #[tokio::test]
    async fn test_sqlite_storage() {
        let storage = SqliteStorage::new_in_memory().await.unwrap();
        test_storage_provider(storage).await;
    }

    #[tokio::test]
    async fn test_sqlite_persistence() {
        // This test demonstrates that SQLite storage persists data
        // (though using in-memory for testing, in real usage you'd use a file path)
        let storage = SqliteStorage::new_in_memory().await.unwrap();

        let key = StorageKey::new("tenant1", "User", "persistent-user");
        let user_data = json!({
            "id": "persistent-user",
            "userName": "persistent.user",
            "displayName": "Persistent User",
            "emails": [{"value": "persistent@example.com", "primary": true}]
        });

        // Store data
        let stored = storage.put(key.clone(), user_data.clone()).await.unwrap();
        assert_eq!(stored, user_data);

        // Verify it can be retrieved
        let retrieved = storage.get(key.clone()).await.unwrap();
        assert_eq!(retrieved, Some(user_data.clone()));

        // Verify stats show the data
        let stats = storage.stats().await.unwrap();
        assert_eq!(stats.tenant_count, 1);
        assert_eq!(stats.resource_type_count, 1);
        assert_eq!(stats.total_resources, 1);

        // Verify search works
        let found = storage
            .find_by_attribute(
                StorageKey::prefix("tenant1", "User"),
                "userName",
                "persistent.user",
            )
            .await
            .unwrap();
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].0, key);
        assert_eq!(found[0].1, user_data);
    }

    #[tokio::test]
    async fn test_sqlite_file_creation() {
        // Test that SQLite properly creates database file and directory

        let temp_dir = std::env::temp_dir();
        let test_db_path = temp_dir.join("test_scim_storage").join("test.db");
        let test_db_str = test_db_path.to_str().unwrap();

        // Ensure the path doesn't exist initially
        if test_db_path.exists() {
            std::fs::remove_file(&test_db_path).unwrap();
        }
        if let Some(parent) = test_db_path.parent() {
            if parent.exists() {
                std::fs::remove_dir_all(parent).unwrap();
            }
        }

        // Create storage - should create directory and file
        let storage = SqliteStorage::new_with_path(test_db_str).await.unwrap();

        // Verify file was created
        assert!(test_db_path.exists());

        // Test basic functionality
        let key = StorageKey::new("tenant1", "User", "test-user");
        let user_data = json!({"id": "test-user", "userName": "test.user"});

        let stored = storage.put(key.clone(), user_data.clone()).await.unwrap();
        assert_eq!(stored, user_data);

        let retrieved = storage.get(key).await.unwrap();
        assert_eq!(retrieved, Some(user_data));

        // Clean up
        std::fs::remove_file(&test_db_path).unwrap();
        if let Some(parent) = test_db_path.parent() {
            std::fs::remove_dir_all(parent).unwrap();
        }
    }

    #[tokio::test]
    async fn test_sqlite_default_path() {
        // Test that the default new() method creates database at scim_data/scim_server.db
        use std::path::Path;

        // Clean up any existing database first
        let default_path = Path::new("scim_data/scim_server.db");
        if default_path.exists() {
            std::fs::remove_file(default_path).unwrap();
        }
        if let Some(parent) = default_path.parent() {
            if parent.exists() && parent.read_dir().unwrap().next().is_none() {
                std::fs::remove_dir(parent).unwrap();
            }
        }

        // Create storage with default path
        let storage = SqliteStorage::new().await.unwrap();

        // Verify file was created at expected location
        assert!(default_path.exists());

        // Test basic functionality
        let key = StorageKey::new("tenant1", "User", "default-user");
        let user_data = json!({"id": "default-user", "userName": "default.user"});

        let stored = storage.put(key.clone(), user_data.clone()).await.unwrap();
        assert_eq!(stored, user_data);

        let retrieved = storage.get(key).await.unwrap();
        assert_eq!(retrieved, Some(user_data));

        // Verify stats work
        let stats = storage.stats().await.unwrap();
        assert_eq!(stats.total_resources, 1);

        // Note: We don't clean up the default database in this test since it's the
        // intended location for the application. In a real scenario, this would be
        // the persistent database that should remain between runs.
    }
}
