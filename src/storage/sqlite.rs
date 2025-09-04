//! SQLite-based storage implementation for SCIM resources.
//!
//! This module provides persistent storage using SQLite database with the same interface
//! as in-memory storage. Resources are stored as key-value pairs where the key represents
//! the hierarchical tenant/resource_type/resource_id structure and the value contains
//! the JSON resource data.
//!
//! # Database Schema
//!
//! The storage uses a simple table structure:
//! - `tenant_id`: Text field for tenant isolation
//! - `resource_type`: Text field for resource type (User, Group, etc.)
//! - `resource_id`: Text field for the resource identifier
//! - `data`: Text field containing JSON resource data
//! - Primary key: (tenant_id, resource_type, resource_id)
//!
//! # Usage
//!
//! ```rust
//! use scim_server::storage::{SqliteStorage, StorageProvider, StorageKey};
//! use serde_json::json;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // Default database at scim_data/scim_server.db (creates directory if needed)
//! let storage = SqliteStorage::new().await?;
//!
//! // Or custom path
//! let storage = SqliteStorage::new_with_path("custom/path/data.db").await?;
//!
//! // Or in-memory for testing
//! let storage = SqliteStorage::new_in_memory().await?;
//!
//! let key = StorageKey::new("tenant1", "User", "123");
//! let user_data = json!({
//!     "id": "123",
//!     "userName": "john.doe"
//! });
//!
//! let stored = storage.put(key.clone(), user_data).await?;
//! let retrieved = storage.get(key).await?;
//!
//! // Check statistics
//! let stats = storage.stats().await?;
//! println!("Total resources: {}", stats.total_resources);
//! # Ok(())
//! # }
//! ```
//!
//! # Database Creation Behavior
//!
//! SQLiteStorage provides explicit control over database file creation:
//!
//! - **`new()`**: Creates database at `scim_data/scim_server.db`
//! - **`new_with_path(path)`**: Creates database at custom path
//! - **`new_in_memory()`**: Creates temporary in-memory database
//!
//! If the database file doesn't exist, it will be created along with any
//! necessary parent directories. If it exists, it will be opened for read-write access.
//!
//! # Examples
//!
//! ```rust
//! use scim_server::storage::{SqliteStorage, StorageProvider, StorageKey};
//! use serde_json::json;
//!
//! # async fn examples() -> Result<(), Box<dyn std::error::Error>> {
//! // Production usage - creates scim_data/scim_server.db
//! let storage = SqliteStorage::new().await?;
//!
//! // Store a user
//! let key = StorageKey::new("company1", "User", "user123");
//! let user = json!({
//!     "id": "user123",
//!     "userName": "john.doe",
//!     "displayName": "John Doe",
//!     "emails": [{"value": "john@company1.com", "primary": true}]
//! });
//!
//! storage.put(key.clone(), user.clone()).await?;
//!
//! // Retrieve the user
//! let retrieved = storage.get(key).await?;
//! assert_eq!(retrieved, Some(user));
//!
//! // Get statistics
//! let stats = storage.stats().await?;
//! println!("Storage contains {} resources across {} tenants",
//!          stats.total_resources, stats.tenant_count);
//!
//! // Search for users
//! let found = storage.find_by_attribute(
//!     StorageKey::prefix("company1", "User"),
//!     "userName",
//!     "john.doe"
//! ).await?;
//! println!("Found {} users matching criteria", found.len());
//! # Ok(())
//! # }
//! ```

use crate::storage::{StorageError, StorageKey, StoragePrefix, StorageProvider, StorageStats};
use serde_json::Value;
use sqlx::{Row, SqlitePool};

/// SQLite-based storage provider for SCIM resources.
///
/// Provides persistent storage with the same interface as InMemoryStorage.
/// Uses a simple key-value table structure for efficient storage and retrieval.
pub struct SqliteStorage {
    pool: SqlitePool,
}

impl SqliteStorage {
    /// Create a new SQLite storage instance with the default database file path.
    ///
    /// Creates or opens the database at `scim_data/scim_server.db`. The database file
    /// will be created if it doesn't exist, along with the `scim_data` directory.
    ///
    /// # Returns
    /// A new SqliteStorage instance with initialized database schema.
    pub async fn new() -> Result<Self, StorageError> {
        Self::new_with_path("scim_data/scim_server.db").await
    }

    /// Create a new SQLite storage instance with a custom database file path.
    ///
    /// # Arguments
    /// * `database_path` - Path to the SQLite database file
    ///
    /// # Returns
    /// A new SqliteStorage instance with initialized database schema.
    ///
    /// # Behavior
    /// - Creates the database file if it doesn't exist
    /// - Creates parent directories if they don't exist
    /// - Opens existing database for read-write access
    pub async fn new_with_path(database_path: &str) -> Result<Self, StorageError> {
        use sqlx::sqlite::SqliteConnectOptions;
        use std::path::Path;

        // Create parent directory if it doesn't exist
        if let Some(parent) = Path::new(database_path).parent() {
            if !parent.exists() {
                std::fs::create_dir_all(parent).map_err(|e| {
                    StorageError::configuration(format!(
                        "Failed to create directory {}: {}",
                        parent.display(),
                        e
                    ))
                })?;
            }
        }

        // Configure SQLite connection with explicit creation behavior
        let options = SqliteConnectOptions::new()
            .filename(database_path)
            .create_if_missing(true)
            .foreign_keys(true);

        let pool = SqlitePool::connect_with(options).await.map_err(|e| {
            StorageError::configuration(format!(
                "Failed to connect to SQLite database at {}: {}",
                database_path, e
            ))
        })?;

        let storage = Self { pool };
        storage.initialize_schema().await?;
        Ok(storage)
    }

    /// Create a new in-memory SQLite storage instance for testing.
    ///
    /// # Returns
    /// A new SqliteStorage instance with initialized database schema.
    pub async fn new_in_memory() -> Result<Self, StorageError> {
        let pool = SqlitePool::connect(":memory:").await.map_err(|e| {
            StorageError::configuration(format!("Failed to create in-memory SQLite: {}", e))
        })?;

        let storage = Self { pool };
        storage.initialize_schema().await?;
        Ok(storage)
    }

    /// Initialize the database schema if it doesn't exist.
    async fn initialize_schema(&self) -> Result<(), StorageError> {
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS scim_resources (
                tenant_id TEXT NOT NULL,
                resource_type TEXT NOT NULL,
                resource_id TEXT NOT NULL,
                data TEXT NOT NULL,
                PRIMARY KEY (tenant_id, resource_type, resource_id)
            )
            "#,
        )
        .execute(&self.pool)
        .await
        .map_err(|e| StorageError::internal(format!("Failed to create schema: {}", e)))?;

        Ok(())
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

impl StorageProvider for SqliteStorage {
    type Error = StorageError;

    async fn put(&self, key: StorageKey, data: Value) -> Result<Value, Self::Error> {
        let data_str = serde_json::to_string(&data)
            .map_err(|e| StorageError::serialization(format!("Failed to serialize data: {}", e)))?;

        sqlx::query(
            "INSERT OR REPLACE INTO scim_resources (tenant_id, resource_type, resource_id, data) VALUES (?, ?, ?, ?)"
        )
        .bind(key.tenant_id())
        .bind(key.resource_type())
        .bind(key.resource_id())
        .bind(&data_str)
        .execute(&self.pool)
        .await
        .map_err(|e| StorageError::internal(format!("Failed to store resource: {}", e)))?;

        Ok(data)
    }

    async fn get(&self, key: StorageKey) -> Result<Option<Value>, Self::Error> {
        let row = sqlx::query(
            "SELECT data FROM scim_resources WHERE tenant_id = ? AND resource_type = ? AND resource_id = ?"
        )
        .bind(key.tenant_id())
        .bind(key.resource_type())
        .bind(key.resource_id())
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| StorageError::internal(format!("Failed to fetch resource: {}", e)))?;

        match row {
            Some(row) => {
                let data_str: String = row.get("data");
                let value = serde_json::from_str(&data_str).map_err(|e| {
                    StorageError::serialization(format!("Failed to deserialize data: {}", e))
                })?;
                Ok(Some(value))
            }
            None => Ok(None),
        }
    }

    async fn delete(&self, key: StorageKey) -> Result<bool, Self::Error> {
        let result = sqlx::query(
            "DELETE FROM scim_resources WHERE tenant_id = ? AND resource_type = ? AND resource_id = ?"
        )
        .bind(key.tenant_id())
        .bind(key.resource_type())
        .bind(key.resource_id())
        .execute(&self.pool)
        .await
        .map_err(|e| StorageError::internal(format!("Failed to delete resource: {}", e)))?;

        Ok(result.rows_affected() > 0)
    }

    async fn list(
        &self,
        prefix: StoragePrefix,
        offset: usize,
        limit: usize,
    ) -> Result<Vec<(StorageKey, Value)>, Self::Error> {
        let rows = sqlx::query(
            "SELECT resource_id, data FROM scim_resources
             WHERE tenant_id = ? AND resource_type = ?
             ORDER BY resource_id
             LIMIT ? OFFSET ?",
        )
        .bind(prefix.tenant_id())
        .bind(prefix.resource_type())
        .bind(limit as i64)
        .bind(offset as i64)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| StorageError::internal(format!("Failed to list resources: {}", e)))?;

        let mut results = Vec::new();
        for row in rows {
            let resource_id: String = row.get("resource_id");
            let data_str: String = row.get("data");
            let data: Value = serde_json::from_str(&data_str).map_err(|e| {
                StorageError::serialization(format!("Failed to deserialize data: {}", e))
            })?;

            let key = StorageKey::new(prefix.tenant_id(), prefix.resource_type(), resource_id);
            results.push((key, data));
        }

        Ok(results)
    }

    async fn find_by_attribute(
        &self,
        prefix: StoragePrefix,
        attribute: &str,
        value: &str,
    ) -> Result<Vec<(StorageKey, Value)>, Self::Error> {
        let rows = sqlx::query(
            "SELECT resource_id, data FROM scim_resources WHERE tenant_id = ? AND resource_type = ?"
        )
        .bind(prefix.tenant_id())
        .bind(prefix.resource_type())
        .fetch_all(&self.pool)
        .await
        .map_err(|e| StorageError::internal(format!("Failed to search resources: {}", e)))?;

        let mut results = Vec::new();
        for row in rows {
            let resource_id: String = row.get("resource_id");
            let data_str: String = row.get("data");
            let data: Value = serde_json::from_str(&data_str).map_err(|e| {
                StorageError::serialization(format!("Failed to deserialize data: {}", e))
            })?;

            if let Some(attr_value) = Self::extract_attribute_value(&data, attribute) {
                if attr_value == value {
                    let key =
                        StorageKey::new(prefix.tenant_id(), prefix.resource_type(), resource_id);
                    results.push((key, data));
                }
            }
        }

        Ok(results)
    }

    async fn exists(&self, key: StorageKey) -> Result<bool, Self::Error> {
        let row = sqlx::query(
            "SELECT 1 FROM scim_resources WHERE tenant_id = ? AND resource_type = ? AND resource_id = ?"
        )
        .bind(key.tenant_id())
        .bind(key.resource_type())
        .bind(key.resource_id())
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| StorageError::internal(format!("Failed to check resource existence: {}", e)))?;

        Ok(row.is_some())
    }

    async fn count(&self, prefix: StoragePrefix) -> Result<usize, Self::Error> {
        let row = sqlx::query(
            "SELECT COUNT(*) as count FROM scim_resources WHERE tenant_id = ? AND resource_type = ?"
        )
        .bind(prefix.tenant_id())
        .bind(prefix.resource_type())
        .fetch_one(&self.pool)
        .await
        .map_err(|e| StorageError::internal(format!("Failed to count resources: {}", e)))?;

        let count: i64 = row.get("count");
        Ok(count as usize)
    }

    async fn list_tenants(&self) -> Result<Vec<String>, Self::Error> {
        let rows = sqlx::query("SELECT DISTINCT tenant_id FROM scim_resources ORDER BY tenant_id")
            .fetch_all(&self.pool)
            .await
            .map_err(|e| StorageError::internal(format!("Failed to list tenants: {}", e)))?;

        let tenants = rows.into_iter().map(|row| row.get("tenant_id")).collect();
        Ok(tenants)
    }

    async fn list_resource_types(&self, tenant_id: &str) -> Result<Vec<String>, Self::Error> {
        let rows = sqlx::query(
            "SELECT DISTINCT resource_type FROM scim_resources WHERE tenant_id = ? ORDER BY resource_type"
        )
        .bind(tenant_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| StorageError::internal(format!("Failed to list resource types: {}", e)))?;

        let types = rows
            .into_iter()
            .map(|row| row.get("resource_type"))
            .collect();
        Ok(types)
    }

    async fn list_all_resource_types(&self) -> Result<Vec<String>, Self::Error> {
        let rows =
            sqlx::query("SELECT DISTINCT resource_type FROM scim_resources ORDER BY resource_type")
                .fetch_all(&self.pool)
                .await
                .map_err(|e| {
                    StorageError::internal(format!("Failed to list all resource types: {}", e))
                })?;

        let types = rows
            .into_iter()
            .map(|row| row.get("resource_type"))
            .collect();
        Ok(types)
    }

    async fn clear(&self) -> Result<(), Self::Error> {
        sqlx::query("DELETE FROM scim_resources")
            .execute(&self.pool)
            .await
            .map_err(|e| StorageError::internal(format!("Failed to clear storage: {}", e)))?;

        Ok(())
    }

    async fn stats(&self) -> Result<StorageStats, Self::Error> {
        let tenant_count_row =
            sqlx::query("SELECT COUNT(DISTINCT tenant_id) as count FROM scim_resources")
                .fetch_one(&self.pool)
                .await
                .map_err(|e| StorageError::internal(format!("Failed to count tenants: {}", e)))?;
        let tenant_count: i64 = tenant_count_row.get("count");

        let resource_type_count_row = sqlx::query(
            "SELECT COUNT(DISTINCT tenant_id || '/' || resource_type) as count FROM scim_resources",
        )
        .fetch_one(&self.pool)
        .await
        .map_err(|e| StorageError::internal(format!("Failed to count resource types: {}", e)))?;
        let resource_type_count: i64 = resource_type_count_row.get("count");

        let total_resources_row = sqlx::query("SELECT COUNT(*) as count FROM scim_resources")
            .fetch_one(&self.pool)
            .await
            .map_err(|e| {
                StorageError::internal(format!("Failed to count total resources: {}", e))
            })?;
        let total_resources: i64 = total_resources_row.get("count");

        Ok(StorageStats {
            tenant_count: tenant_count as usize,
            resource_type_count: resource_type_count as usize,
            total_resources: total_resources as usize,
        })
    }
}
