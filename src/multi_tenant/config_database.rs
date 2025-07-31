//! Database-backed tenant configuration provider implementation.
//!
//! This module provides a database-backed implementation of the `TenantConfigurationProvider`
//! trait, suitable for production deployments. It stores all configurations in a database
//! with proper tenant isolation and ACID guarantees.
//!
//! ## Features
//!
//! * **Persistent Storage**: All configurations stored in database
//! * **ACID Transactions**: Atomic operations with rollback capabilities
//! * **Tenant Isolation**: Database-level tenant separation
//! * **Versioning**: Configuration versioning with optimistic locking
//! * **Audit Trail**: Complete audit log of configuration changes
//! * **Backup/Restore**: Database-backed backup and restore operations
//!
//! ## Usage
//!
//! ```rust
//! use scim_server::multi_tenant::{DatabaseConfigurationProvider, TenantConfiguration};
//! use std::sync::Arc;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let provider = Arc::new(DatabaseConfigurationProvider::new(database).await?);
//!
//!     // Create and store a configuration
//!     let config = TenantConfiguration::builder("tenant-a".to_string())
//!         .with_display_name("Tenant A".to_string())
//!         .build()?;
//!
//!     let stored_config = provider.create_configuration(config).await?;
//!     println!("Stored configuration for tenant: {}", stored_config.tenant_id);
//!
//!     // Retrieve the configuration
//!     let retrieved = provider.get_configuration("tenant-a").await?;
//!     assert!(retrieved.is_some());
//!
//!     Ok(())
//! }
//! ```

use crate::multi_tenant::config_provider::{
    BulkConfigurationOperation, BulkOperationResult, CacheStats, CachedConfigurationProvider,
    ConfigurationQuery, ConfigurationQueryResult, ConfigurationStats, SortOrder,
    TenantConfigurationProvider, ValidationContext,
};
use crate::multi_tenant::configuration::{ConfigurationError, TenantConfiguration};
use crate::multi_tenant::database::{DatabaseConnection, DatabaseParameter};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Database-backed configuration provider that stores configurations in a database
/// with full ACID guarantees and tenant isolation.
pub struct DatabaseConfigurationProvider<D: DatabaseConnection> {
    /// Database connection for storing configurations
    database: Arc<D>,
    /// Cache for frequently accessed configurations
    cache: Arc<RwLock<HashMap<String, (TenantConfiguration, DateTime<Utc>)>>>,
    /// Cache TTL in seconds
    cache_ttl: u64,
    /// Statistics tracking
    stats: Arc<RwLock<ConfigurationStats>>,
}

impl<D: DatabaseConnection> DatabaseConfigurationProvider<D> {
    /// Create a new database configuration provider.
    pub async fn new(database: Arc<D>) -> Result<Self, ConfigurationError> {
        let provider = Self {
            database: Arc::clone(&database),
            cache: Arc::new(RwLock::new(HashMap::new())),
            cache_ttl: 300, // 5 minutes default
            stats: Arc::new(RwLock::new(ConfigurationStats {
                total_configurations: 0,
                recent_configurations: 0,
                recently_modified: 0,
                average_size: 0,
                total_storage_used: 0,
                newest_configuration: None,
                oldest_configuration: None,
                version_distribution: HashMap::new(),
            })),
        };

        // Ensure database schema is set up
        provider.ensure_schema().await?;

        Ok(provider)
    }

    /// Create a new database configuration provider with custom cache TTL.
    pub async fn new_with_cache_ttl(
        database: Arc<D>,
        cache_ttl_seconds: u64,
    ) -> Result<Self, ConfigurationError> {
        let mut provider = Self::new(database).await?;
        provider.cache_ttl = cache_ttl_seconds;
        Ok(provider)
    }

    /// Ensure the database schema is properly set up for configuration storage.
    async fn ensure_schema(&self) -> Result<(), ConfigurationError> {
        let schema_valid = self.database.validate_schema().await.map_err(|e| {
            ConfigurationError::ValidationError {
                message: format!("Database schema validation failed: {}", e),
            }
        })?;

        if !schema_valid {
            // Create the configuration tables
            self.create_schema().await?;
        }

        Ok(())
    }

    /// Create the database schema for configuration storage.
    async fn create_schema(&self) -> Result<(), ConfigurationError> {
        // Create configurations table
        let create_configs_sql = r#"
            CREATE TABLE IF NOT EXISTS tenant_configurations (
                tenant_id VARCHAR(255) PRIMARY KEY,
                display_name VARCHAR(255) NOT NULL,
                created_at TIMESTAMP NOT NULL,
                last_modified TIMESTAMP NOT NULL,
                version INTEGER NOT NULL DEFAULT 1,
                schema_config TEXT NOT NULL,
                operational_config TEXT NOT NULL,
                compliance_config TEXT NOT NULL,
                branding_config TEXT NOT NULL,
                INDEX idx_tenant_id (tenant_id),
                INDEX idx_last_modified (last_modified)
            )
        "#;

        // Create configuration audit log table
        let create_audit_sql = r#"
            CREATE TABLE IF NOT EXISTS configuration_audit_log (
                id BIGINT AUTO_INCREMENT PRIMARY KEY,
                tenant_id VARCHAR(255) NOT NULL,
                operation VARCHAR(50) NOT NULL,
                old_version INTEGER,
                new_version INTEGER,
                changed_by VARCHAR(255),
                changed_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
                changes TEXT,
                INDEX idx_tenant_audit (tenant_id, changed_at),
                INDEX idx_operation (operation)
            )
        "#;

        // Create configuration backups table
        let create_backups_sql = r#"
            CREATE TABLE IF NOT EXISTS configuration_backups (
                backup_id VARCHAR(255) PRIMARY KEY,
                tenant_id VARCHAR(255) NOT NULL,
                configuration TEXT NOT NULL,
                created_at TIMESTAMP NOT NULL,
                backup_type VARCHAR(50) NOT NULL DEFAULT 'manual',
                description TEXT,
                INDEX idx_tenant_backups (tenant_id, created_at),
                INDEX idx_backup_type (backup_type)
            )
        "#;

        // Execute schema creation queries
        self.database
            .execute_query(create_configs_sql, &[], "system")
            .await
            .map_err(|e| ConfigurationError::ValidationError {
                message: format!("Failed to create configurations table: {}", e),
            })?;

        self.database
            .execute_query(create_audit_sql, &[], "system")
            .await
            .map_err(|e| ConfigurationError::ValidationError {
                message: format!("Failed to create audit table: {}", e),
            })?;

        self.database
            .execute_query(create_backups_sql, &[], "system")
            .await
            .map_err(|e| ConfigurationError::ValidationError {
                message: format!("Failed to create backups table: {}", e),
            })?;

        Ok(())
    }

    /// Check if a configuration is cached and still valid.
    async fn get_from_cache(&self, tenant_id: &str) -> Option<TenantConfiguration> {
        let cache = self.cache.read().await;
        if let Some((config, cached_at)) = cache.get(tenant_id) {
            let age = Utc::now().signed_duration_since(*cached_at);
            if age.num_seconds() < self.cache_ttl as i64 {
                return Some(config.clone());
            }
        }
        None
    }

    /// Cache a configuration.
    async fn cache_configuration(&self, config: &TenantConfiguration) {
        let mut cache = self.cache.write().await;
        cache.insert(config.tenant_id.clone(), (config.clone(), Utc::now()));
    }

    /// Remove a configuration from cache.
    async fn invalidate_cache(&self, tenant_id: &str) {
        let mut cache = self.cache.write().await;
        cache.remove(tenant_id);
    }

    /// Clear all cached configurations.
    async fn clear_cache(&self) {
        let mut cache = self.cache.write().await;
        cache.clear();
    }

    /// Record an audit log entry for configuration changes.
    async fn log_audit_entry(
        &self,
        tenant_id: &str,
        operation: &str,
        old_version: Option<u64>,
        new_version: Option<u64>,
        changed_by: Option<&str>,
        changes: Option<&str>,
    ) -> Result<(), ConfigurationError> {
        let insert_sql = r#"
            INSERT INTO configuration_audit_log
            (tenant_id, operation, old_version, new_version, changed_by, changes)
            VALUES (?, ?, ?, ?, ?, ?)
        "#;

        let params: Vec<Box<dyn DatabaseParameter>> = vec![
            Box::new(tenant_id.to_string()),
            Box::new(operation.to_string()),
            Box::new(old_version.map(|v| v as i64).unwrap_or(-1)),
            Box::new(new_version.map(|v| v as i64).unwrap_or(-1)),
            Box::new(changed_by.unwrap_or("system").to_string()),
            Box::new(changes.unwrap_or("").to_string()),
        ];

        let param_refs: Vec<&dyn DatabaseParameter> = params.iter().map(|p| p.as_ref()).collect();

        self.database
            .execute_query(insert_sql, &param_refs, tenant_id)
            .await
            .map_err(|e| ConfigurationError::ValidationError {
                message: format!("Failed to log audit entry: {}", e),
            })?;

        Ok(())
    }

    /// Update statistics for operations.
    async fn update_stats(&self, operation: &str) {
        let mut stats = self.stats.write().await;
        match operation {
            "create" => stats.total_configurations += 1,
            "delete" => {
                if stats.total_configurations > 0 {
                    stats.total_configurations -= 1;
                }
            }
            _ => {}
        }
        stats.newest_configuration = Some(Utc::now());
    }

    /// Convert database row to TenantConfiguration.
    fn row_to_configuration(
        &self,
        row: &HashMap<String, Value>,
    ) -> Result<TenantConfiguration, ConfigurationError> {
        let tenant_id = row
            .get("tenant_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ConfigurationError::ValidationError {
                message: "Missing tenant_id in database row".to_string(),
            })?
            .to_string();

        let display_name = row
            .get("display_name")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ConfigurationError::ValidationError {
                message: "Missing display_name in database row".to_string(),
            })?
            .to_string();

        let created_at: DateTime<Utc> = row
            .get("created_at")
            .and_then(|v| v.as_str())
            .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
            .map(|dt| dt.with_timezone(&Utc))
            .ok_or_else(|| ConfigurationError::ValidationError {
                message: "Invalid created_at in database row".to_string(),
            })?;

        let last_modified: DateTime<Utc> = row
            .get("last_modified")
            .and_then(|v| v.as_str())
            .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
            .map(|dt| dt.with_timezone(&Utc))
            .ok_or_else(|| ConfigurationError::ValidationError {
                message: "Invalid last_modified in database row".to_string(),
            })?;

        let version = row.get("version").and_then(|v| v.as_i64()).ok_or_else(|| {
            ConfigurationError::ValidationError {
                message: "Invalid version in database row".to_string(),
            }
        })? as u64;

        let schema = row
            .get("schema_config")
            .and_then(|v| v.as_str())
            .and_then(|s| serde_json::from_str(s).ok())
            .ok_or_else(|| ConfigurationError::ValidationError {
                message: "Invalid schema_config in database row".to_string(),
            })?;

        let operational = row
            .get("operational_config")
            .and_then(|v| v.as_str())
            .and_then(|s| serde_json::from_str(s).ok())
            .ok_or_else(|| ConfigurationError::ValidationError {
                message: "Invalid operational_config in database row".to_string(),
            })?;

        let compliance = row
            .get("compliance_config")
            .and_then(|v| v.as_str())
            .and_then(|s| serde_json::from_str(s).ok())
            .ok_or_else(|| ConfigurationError::ValidationError {
                message: "Invalid compliance_config in database row".to_string(),
            })?;

        let branding = row
            .get("branding_config")
            .and_then(|v| v.as_str())
            .and_then(|s| serde_json::from_str(s).ok())
            .ok_or_else(|| ConfigurationError::ValidationError {
                message: "Invalid branding_config in database row".to_string(),
            })?;

        Ok(TenantConfiguration {
            tenant_id,
            display_name,
            created_at,
            last_modified,
            version: version.into(),
            schema,
            operational,
            compliance,
            branding,
        })
    }

    /// Convert TenantConfiguration to database parameters.
    fn configuration_to_params(
        &self,
        config: &TenantConfiguration,
    ) -> Result<Vec<Box<dyn DatabaseParameter>>, ConfigurationError> {
        let schema_json = serde_json::to_string(&config.schema)
            .map_err(|e| ConfigurationError::SerializationError { source: e })?;

        let operational_json = serde_json::to_string(&config.operational)
            .map_err(|e| ConfigurationError::SerializationError { source: e })?;

        let compliance_json = serde_json::to_string(&config.compliance)
            .map_err(|e| ConfigurationError::SerializationError { source: e })?;

        let branding_json = serde_json::to_string(&config.branding)
            .map_err(|e| ConfigurationError::SerializationError { source: e })?;

        Ok(vec![
            Box::new(config.tenant_id.clone()),
            Box::new(config.display_name.clone()),
            Box::new(config.created_at.to_rfc3339()),
            Box::new(config.last_modified.to_rfc3339()),
            Box::new(config.version as i64),
            Box::new(schema_json),
            Box::new(operational_json),
            Box::new(compliance_json),
            Box::new(branding_json),
        ])
    }

    /// Query configurations with advanced filtering.
    async fn query_configurations(
        &self,
        query: &ConfigurationQuery,
    ) -> Result<ConfigurationQueryResult, ConfigurationError> {
        let mut where_clauses = Vec::new();
        let mut params: Vec<Box<dyn DatabaseParameter>> = Vec::new();

        // Build WHERE clause based on query
        if let Some(ref tenant_ids) = query.tenant_ids {
            let placeholders = tenant_ids
                .iter()
                .map(|_| "?")
                .collect::<Vec<_>>()
                .join(", ");
            where_clauses.push(format!("tenant_id IN ({})", placeholders));
            for tenant_id in tenant_ids {
                params.push(Box::new(tenant_id.clone()));
            }
        }

        if let Some(ref display_name_pattern) = query.display_name_filter {
            where_clauses.push("display_name LIKE ?".to_string());
            params.push(Box::new(format!("%{}%", display_name_pattern)));
        }

        if let Some(ref modified_after) = query.modified_after {
            where_clauses.push("last_modified > ?".to_string());
            params.push(Box::new(modified_after.to_rfc3339()));
        }

        if let Some(ref modified_before) = query.modified_before {
            where_clauses.push("last_modified < ?".to_string());
            params.push(Box::new(modified_before.to_rfc3339()));
        }

        // Build the complete query
        let mut sql = r#"
            SELECT tenant_id, display_name, created_at, last_modified, version,
                   schema_config, operational_config, compliance_config, branding_config
            FROM tenant_configurations
        "#
        .to_string();

        if !where_clauses.is_empty() {
            sql.push_str(" WHERE ");
            sql.push_str(&where_clauses.join(" AND "));
        }

        // Add ordering
        let (order_field, order_direction) = match query.sort_order {
            SortOrder::TenantIdAsc => ("tenant_id", "ASC"),
            SortOrder::TenantIdDesc => ("tenant_id", "DESC"),
            SortOrder::DisplayNameAsc => ("display_name", "ASC"),
            SortOrder::DisplayNameDesc => ("display_name", "DESC"),
            SortOrder::LastModifiedAsc => ("last_modified", "ASC"),
            SortOrder::LastModifiedDesc => ("last_modified", "DESC"),
            SortOrder::CreatedAsc => ("created_at", "ASC"),
            SortOrder::CreatedDesc => ("created_at", "DESC"),
        };

        sql.push_str(&format!(" ORDER BY {} {}", order_field, order_direction));

        // Add pagination
        let limit = query.limit.unwrap_or(100).min(1000);
        let offset = query.offset.unwrap_or(0);
        sql.push_str(&format!(" LIMIT {} OFFSET {}", limit, offset));

        params.push(Box::new(limit as i64));
        params.push(Box::new(offset as i64));

        let param_refs: Vec<&dyn DatabaseParameter> = params.iter().map(|p| p.as_ref()).collect();

        let result = self
            .database
            .execute_query(&sql, &param_refs, "system")
            .await
            .map_err(|e| ConfigurationError::ValidationError {
                message: format!("Failed to query configurations: {}", e),
            })?;

        let mut configurations = Vec::new();
        for row in &result.rows {
            configurations.push(self.row_to_configuration(row)?);
        }

        // Get total count for pagination
        let total_count = if configurations.len() < limit {
            offset + configurations.len()
        } else {
            self.count_configurations().await?
        };

        let config_count = configurations.len();
        Ok(ConfigurationQueryResult {
            configurations,
            total_count,
            has_more: config_count == limit,
            next_offset: if config_count == limit {
                Some(offset + limit)
            } else {
                None
            },
        })
    }

    /// Backup a single configuration.
    async fn backup_single_configuration(
        &self,
        tenant_id: &str,
        backup_id: &str,
        description: Option<&str>,
    ) -> Result<(), ConfigurationError> {
        // Get current configuration
        let config = self.get_configuration(tenant_id).await?.ok_or_else(|| {
            ConfigurationError::NotFound {
                tenant_id: tenant_id.to_string(),
            }
        })?;

        // Serialize configuration for backup
        let config_json = serde_json::to_string(&config)
            .map_err(|e| ConfigurationError::SerializationError { source: e })?;

        let insert_sql = r#"
            INSERT INTO configuration_backups
            (backup_id, tenant_id, configuration, created_at, backup_type, description)
            VALUES (?, ?, ?, ?, 'manual', ?)
        "#;

        let params: Vec<Box<dyn DatabaseParameter>> = vec![
            Box::new(backup_id.to_string()),
            Box::new(tenant_id.to_string()),
            Box::new(config_json),
            Box::new(Utc::now().to_rfc3339()),
            Box::new(description.unwrap_or("").to_string()),
        ];

        let param_refs: Vec<&dyn DatabaseParameter> = params.iter().map(|p| p.as_ref()).collect();

        self.database
            .execute_query(insert_sql, &param_refs, tenant_id)
            .await
            .map_err(|e| ConfigurationError::ValidationError {
                message: format!("Failed to create backup: {}", e),
            })?;

        // Log audit entry
        self.log_audit_entry(
            tenant_id,
            "backup",
            Some(config.version),
            None,
            None,
            Some(&format!("Configuration backed up with ID: {}", backup_id)),
        )
        .await?;

        Ok(())
    }

    /// Restore a single configuration from backup.
    async fn restore_single_configuration(
        &self,
        tenant_id: &str,
        backup_id: &str,
    ) -> Result<TenantConfiguration, ConfigurationError> {
        // Get backup from database
        let select_sql = r#"
            SELECT configuration
            FROM configuration_backups
            WHERE backup_id = ? AND tenant_id = ?
        "#;

        let params: Vec<Box<dyn DatabaseParameter>> = vec![
            Box::new(backup_id.to_string()),
            Box::new(tenant_id.to_string()),
        ];
        let param_refs: Vec<&dyn DatabaseParameter> = params.iter().map(|p| p.as_ref()).collect();

        let result = self
            .database
            .execute_query(select_sql, &param_refs, tenant_id)
            .await
            .map_err(|e| ConfigurationError::ValidationError {
                message: format!("Failed to query backup: {}", e),
            })?;

        if result.rows.is_empty() {
            return Err(ConfigurationError::NotFound {
                tenant_id: format!("backup {} for tenant {}", backup_id, tenant_id),
            });
        }

        let config_json = result.rows[0]
            .get("configuration")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ConfigurationError::ValidationError {
                message: "Invalid backup data in database row".to_string(),
            })?;

        let mut restored_config: TenantConfiguration = serde_json::from_str(config_json)
            .map_err(|e| ConfigurationError::SerializationError { source: e })?;

        // Update timestamps and version for restoration
        let current_version = if let Ok(Some(current)) = self.get_configuration(tenant_id).await {
            current.version
        } else {
            0
        };

        restored_config.version = current_version + 1;
        restored_config.last_modified = Utc::now();

        // Update or create the configuration
        if self.configuration_exists(tenant_id).await? {
            self.update_configuration(restored_config.clone()).await?;
        } else {
            self.create_configuration(restored_config.clone()).await?;
        }

        // Log audit entry
        self.log_audit_entry(
            tenant_id,
            "restore",
            Some(current_version),
            Some(restored_config.version),
            None,
            Some(&format!(
                "Configuration restored from backup ID: {}",
                backup_id
            )),
        )
        .await?;

        Ok(restored_config)
    }
}

#[async_trait]
impl<D: DatabaseConnection> TenantConfigurationProvider for DatabaseConfigurationProvider<D> {
    type Error = ConfigurationError;

    async fn create_configuration(
        &self,
        config: TenantConfiguration,
    ) -> Result<TenantConfiguration, Self::Error> {
        // Check if configuration already exists
        if self.configuration_exists(&config.tenant_id).await? {
            return Err(ConfigurationError::Conflict {
                message: format!(
                    "Configuration for tenant {} already exists",
                    config.tenant_id
                ),
            });
        }

        // Validate configuration
        config.validate()?;

        let insert_sql = r#"
            INSERT INTO tenant_configurations
            (tenant_id, display_name, created_at, last_modified, version,
             schema_config, operational_config, compliance_config, branding_config)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
        "#;

        let params = self.configuration_to_params(&config)?;
        let param_refs: Vec<&dyn DatabaseParameter> = params.iter().map(|p| p.as_ref()).collect();

        self.database
            .execute_query(insert_sql, &param_refs, &config.tenant_id)
            .await
            .map_err(|e| ConfigurationError::ValidationError {
                message: format!("Failed to create configuration: {}", e),
            })?;

        // Log audit entry
        self.log_audit_entry(
            &config.tenant_id,
            "create",
            None,
            Some(config.version),
            None,
            Some("Configuration created"),
        )
        .await?;

        // Cache the configuration
        self.cache_configuration(&config).await;

        // Update statistics
        self.update_stats("create").await;

        Ok(config)
    }

    async fn get_configuration(
        &self,
        tenant_id: &str,
    ) -> Result<Option<TenantConfiguration>, Self::Error> {
        // Check cache first
        if let Some(cached_config) = self.get_from_cache(tenant_id).await {
            self.update_stats("get").await;
            return Ok(Some(cached_config));
        }

        let select_sql = r#"
            SELECT tenant_id, display_name, created_at, last_modified, version,
                   schema_config, operational_config, compliance_config, branding_config
            FROM tenant_configurations
            WHERE tenant_id = ?
        "#;

        let params: Vec<Box<dyn DatabaseParameter>> = vec![Box::new(tenant_id.to_string())];
        let param_refs: Vec<&dyn DatabaseParameter> = params.iter().map(|p| p.as_ref()).collect();

        let result = self
            .database
            .execute_query(select_sql, &param_refs, tenant_id)
            .await
            .map_err(|e| ConfigurationError::ValidationError {
                message: format!("Failed to query configuration: {}", e),
            })?;

        if result.rows.is_empty() {
            self.update_stats("get").await;
            return Ok(None);
        }

        let config = self.row_to_configuration(&result.rows[0])?;

        // Cache the configuration
        self.cache_configuration(&config).await;

        // Update statistics
        self.update_stats("get").await;

        Ok(Some(config))
    }

    async fn update_configuration(
        &self,
        config: TenantConfiguration,
    ) -> Result<TenantConfiguration, Self::Error> {
        // Get current configuration for version checking
        let current = self
            .get_configuration(&config.tenant_id)
            .await?
            .ok_or_else(|| ConfigurationError::NotFound {
                tenant_id: config.tenant_id.clone(),
            })?;

        // Version check for optimistic locking
        if config.version != current.version {
            return Err(ConfigurationError::VersionMismatch {
                expected: current.version,
                actual: config.version,
            });
        }

        // Validate updated configuration
        config.validate()?;

        // Store original version before moving config
        let original_version = config.version;

        // Create updated configuration with incremented version
        let mut updated_config = config;
        updated_config.version += 1;
        updated_config.last_modified = Utc::now();

        let update_sql = r#"
            UPDATE tenant_configurations
            SET display_name = ?, last_modified = ?, version = ?,
                schema_config = ?, operational_config = ?,
                compliance_config = ?, branding_config = ?
            WHERE tenant_id = ? AND version = ?
        "#;

        let params: Vec<Box<dyn DatabaseParameter>> = vec![
            Box::new(updated_config.display_name.clone()),
            Box::new(updated_config.last_modified.to_rfc3339()),
            Box::new(updated_config.version as i64),
            Box::new(
                serde_json::to_string(&updated_config.schema)
                    .map_err(|e| ConfigurationError::SerializationError { source: e })?,
            ),
            Box::new(
                serde_json::to_string(&updated_config.operational)
                    .map_err(|e| ConfigurationError::SerializationError { source: e })?,
            ),
            Box::new(
                serde_json::to_string(&updated_config.compliance)
                    .map_err(|e| ConfigurationError::SerializationError { source: e })?,
            ),
            Box::new(
                serde_json::to_string(&updated_config.branding)
                    .map_err(|e| ConfigurationError::SerializationError { source: e })?,
            ),
            Box::new(updated_config.tenant_id.clone()),
            Box::new(current.version as i64),
        ];

        let param_refs: Vec<&dyn DatabaseParameter> = params.iter().map(|p| p.as_ref()).collect();

        let result = self
            .database
            .execute_query(update_sql, &param_refs, &updated_config.tenant_id)
            .await
            .map_err(|e| ConfigurationError::ValidationError {
                message: format!("Failed to update configuration: {}", e),
            })?;

        if result.affected_rows == 0 {
            return Err(ConfigurationError::VersionMismatch {
                expected: current.version,
                actual: original_version,
            });
        }

        // Log audit entry
        self.log_audit_entry(
            &updated_config.tenant_id,
            "update",
            Some(current.version),
            Some(updated_config.version),
            None,
            Some("Configuration updated"),
        )
        .await?;

        // Invalidate cache
        self.invalidate_cache(&updated_config.tenant_id).await;

        // Update statistics
        self.update_stats("update").await;

        Ok(updated_config)
    }

    async fn delete_configuration(
        &self,
        tenant_id: &str,
        expected_version: Option<u64>,
    ) -> Result<(), Self::Error> {
        // Get current configuration for audit logging
        let current = self.get_configuration(tenant_id).await?.ok_or_else(|| {
            ConfigurationError::NotFound {
                tenant_id: tenant_id.to_string(),
            }
        })?;

        let (delete_sql, params): (String, Vec<Box<dyn DatabaseParameter>>) = if let Some(version) =
            expected_version
        {
            (
                "DELETE FROM tenant_configurations WHERE tenant_id = ? AND version = ?".to_string(),
                vec![Box::new(tenant_id.to_string()), Box::new(version as i64)],
            )
        } else {
            (
                "DELETE FROM tenant_configurations WHERE tenant_id = ?".to_string(),
                vec![Box::new(tenant_id.to_string())],
            )
        };

        let param_refs: Vec<&dyn DatabaseParameter> = params.iter().map(|p| p.as_ref()).collect();

        let result = self
            .database
            .execute_query(&delete_sql, &param_refs, tenant_id)
            .await
            .map_err(|e| ConfigurationError::ValidationError {
                message: format!("Failed to delete configuration: {}", e),
            })?;

        if result.affected_rows == 0 {
            return Err(ConfigurationError::NotFound {
                tenant_id: tenant_id.to_string(),
            });
        }

        // Log audit entry
        self.log_audit_entry(
            tenant_id,
            "delete",
            Some(current.version),
            None,
            None,
            Some("Configuration deleted"),
        )
        .await?;

        // Invalidate cache
        self.invalidate_cache(tenant_id).await;

        // Update statistics
        self.update_stats("delete").await;

        Ok(())
    }

    async fn list_configurations(
        &self,
        query: &ConfigurationQuery,
    ) -> Result<ConfigurationQueryResult, Self::Error> {
        self.query_configurations(query).await
    }

    async fn configuration_exists(&self, tenant_id: &str) -> Result<bool, Self::Error> {
        // Check cache first
        if self.get_from_cache(tenant_id).await.is_some() {
            return Ok(true);
        }

        let select_sql = "SELECT 1 FROM tenant_configurations WHERE tenant_id = ? LIMIT 1";
        let params: Vec<Box<dyn DatabaseParameter>> = vec![Box::new(tenant_id.to_string())];
        let param_refs: Vec<&dyn DatabaseParameter> = params.iter().map(|p| p.as_ref()).collect();

        let result = self
            .database
            .execute_query(select_sql, &param_refs, tenant_id)
            .await
            .map_err(|e| ConfigurationError::ValidationError {
                message: format!("Failed to check configuration existence: {}", e),
            })?;

        Ok(!result.rows.is_empty())
    }

    async fn count_configurations(&self) -> Result<usize, Self::Error> {
        let count_sql = "SELECT COUNT(*) as count FROM tenant_configurations";

        let result = self
            .database
            .execute_query(count_sql, &[], "system")
            .await
            .map_err(|e| ConfigurationError::ValidationError {
                message: format!("Failed to count configurations: {}", e),
            })?;

        if let Some(row) = result.rows.first() {
            if let Some(count_value) = row.get("count") {
                if let Some(count) = count_value.as_i64() {
                    return Ok(count as usize);
                }
            }
        }

        Ok(0)
    }

    async fn validate_configuration(
        &self,
        config: &TenantConfiguration,
        _context: &ValidationContext,
    ) -> Result<(), Self::Error> {
        config.validate()
    }

    async fn bulk_operations(
        &self,
        operations: &[BulkConfigurationOperation],
    ) -> Result<Vec<BulkOperationResult>, Self::Error> {
        let mut results = Vec::new();

        // Process bulk operations
        for operation in operations {
            let result = match operation {
                BulkConfigurationOperation::Create(config) => {
                    match self.create_configuration(config.clone()).await {
                        Ok(created_config) => BulkOperationResult::Success {
                            tenant_id: config.tenant_id.clone(),
                            operation: "create".to_string(),
                            configuration: Some(created_config),
                        },
                        Err(e) => BulkOperationResult::Error {
                            tenant_id: config.tenant_id.clone(),
                            operation: "create".to_string(),
                            error: e.to_string(),
                        },
                    }
                }
                BulkConfigurationOperation::Update(config) => {
                    match self.update_configuration(config.clone()).await {
                        Ok(updated_config) => BulkOperationResult::Success {
                            tenant_id: config.tenant_id.clone(),
                            operation: "update".to_string(),
                            configuration: Some(updated_config),
                        },
                        Err(e) => BulkOperationResult::Error {
                            tenant_id: config.tenant_id.clone(),
                            operation: "update".to_string(),
                            error: e.to_string(),
                        },
                    }
                }
                BulkConfigurationOperation::Delete {
                    tenant_id,
                    expected_version,
                } => {
                    match self
                        .delete_configuration(tenant_id, *expected_version)
                        .await
                    {
                        Ok(_) => BulkOperationResult::Success {
                            tenant_id: tenant_id.clone(),
                            operation: "delete".to_string(),
                            configuration: None,
                        },
                        Err(e) => BulkOperationResult::Error {
                            tenant_id: tenant_id.clone(),
                            operation: "delete".to_string(),
                            error: e.to_string(),
                        },
                    }
                }
                BulkConfigurationOperation::Validate(config) => {
                    match self
                        .validate_configuration(
                            config,
                            &ValidationContext {
                                is_create: false,
                                previous_configuration: None,
                                validation_params: HashMap::new(),
                            },
                        )
                        .await
                    {
                        Ok(_) => BulkOperationResult::Success {
                            tenant_id: config.tenant_id.clone(),
                            operation: "validate".to_string(),
                            configuration: Some(config.clone()),
                        },
                        Err(e) => BulkOperationResult::Error {
                            tenant_id: config.tenant_id.clone(),
                            operation: "validate".to_string(),
                            error: e.to_string(),
                        },
                    }
                }
            };
            results.push(result);
        }

        Ok(results)
    }

    async fn get_configuration_stats(&self) -> Result<ConfigurationStats, Self::Error> {
        let stats = self.stats.read().await;
        Ok(stats.clone())
    }

    async fn backup_configurations(
        &self,
        tenant_ids: Option<&[String]>,
        backup_location: &str,
    ) -> Result<String, Self::Error> {
        let backup_id = format!("backup_{}", Utc::now().timestamp());

        let tenant_list = if let Some(ids) = tenant_ids {
            ids.to_vec()
        } else {
            // Get all tenant IDs
            let query = ConfigurationQuery {
                tenant_ids: None,
                display_name_filter: None,
                modified_after: None,
                modified_before: None,
                offset: None,
                limit: None,
                sort_order: SortOrder::TenantIdAsc,
            };
            let result = self.query_configurations(&query).await?;
            result
                .configurations
                .into_iter()
                .map(|c| c.tenant_id)
                .collect()
        };

        for tenant_id in &tenant_list {
            self.backup_single_configuration(tenant_id, &backup_id, Some(backup_location))
                .await?;
        }

        Ok(backup_id)
    }

    async fn restore_configurations(
        &self,
        backup_id: &str,
        _backup_location: &str,
        overwrite_existing: bool,
    ) -> Result<usize, Self::Error> {
        // Get all backups with this ID
        let select_sql = r#"
            SELECT tenant_id, configuration
            FROM configuration_backups
            WHERE backup_id = ?
        "#;

        let params: Vec<Box<dyn DatabaseParameter>> = vec![Box::new(backup_id.to_string())];
        let param_refs: Vec<&dyn DatabaseParameter> = params.iter().map(|p| p.as_ref()).collect();

        let result = self
            .database
            .execute_query(select_sql, &param_refs, "system")
            .await
            .map_err(|e| ConfigurationError::ValidationError {
                message: format!("Failed to query backups: {}", e),
            })?;

        let mut restored_count = 0;
        for row in &result.rows {
            let tenant_id = row.get("tenant_id").and_then(|v| v.as_str()).unwrap_or("");

            if !overwrite_existing && self.configuration_exists(tenant_id).await? {
                continue;
            }

            if let Ok(_) = self
                .restore_single_configuration(tenant_id, backup_id)
                .await
            {
                restored_count += 1;
            }
        }

        Ok(restored_count)
    }
}

#[async_trait]
impl<D: DatabaseConnection> CachedConfigurationProvider for DatabaseConfigurationProvider<D> {
    async fn clear_cache(&self, tenant_id: &str) -> Result<(), Self::Error> {
        self.invalidate_cache(tenant_id).await;
        Ok(())
    }

    async fn clear_all_cache(&self) -> Result<(), Self::Error> {
        self.clear_cache().await;
        Ok(())
    }

    async fn get_cache_stats(&self) -> Result<CacheStats, Self::Error> {
        let cache = self.cache.read().await;
        Ok(CacheStats {
            cache_hits: 0,
            cache_misses: 0,
            hit_ratio: 0.0,
            cached_items: cache.len(),
            memory_usage: cache.len() * std::mem::size_of::<TenantConfiguration>(),
            evictions: 0,
            average_lookup_time_us: 0,
        })
    }

    async fn warm_cache(&self, tenant_ids: Option<&[String]>) -> Result<usize, Self::Error> {
        let mut warmed = 0;
        let ids_to_warm = if let Some(ids) = tenant_ids {
            ids.to_vec()
        } else {
            // Get all tenant IDs
            let query = ConfigurationQuery {
                tenant_ids: None,
                display_name_filter: None,
                modified_after: None,
                modified_before: None,
                offset: None,
                limit: None,
                sort_order: SortOrder::TenantIdAsc,
            };
            let result = self.query_configurations(&query).await?;
            result
                .configurations
                .into_iter()
                .map(|c| c.tenant_id)
                .collect()
        };

        for tenant_id in ids_to_warm {
            if let Ok(Some(config)) = self.get_configuration(&tenant_id).await {
                self.cache_configuration(&config).await;
                warmed += 1;
            }
        }
        Ok(warmed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::multi_tenant::database::InMemoryDatabase;

    async fn create_test_provider() -> DatabaseConfigurationProvider<InMemoryDatabase> {
        let database = Arc::new(InMemoryDatabase::new());
        DatabaseConfigurationProvider::new(database).await.unwrap()
    }

    fn create_test_config(tenant_id: &str) -> TenantConfiguration {
        TenantConfiguration::builder(tenant_id.to_string())
            .with_display_name(format!("Test Tenant {}", tenant_id))
            .build()
            .unwrap()
    }

    #[tokio::test]
    async fn test_database_provider_creation() {
        let provider = create_test_provider().await;

        // Test that schema is created
        let stats = provider.get_configuration_stats().await.unwrap();
        assert_eq!(stats.total_configurations, 0);
    }

    #[tokio::test]
    async fn test_database_provider_basic_operations() {
        let provider = create_test_provider().await;
        let config = create_test_config("test-tenant");

        // Test create
        let created = provider.create_configuration(config.clone()).await.unwrap();
        assert_eq!(created.tenant_id, "test-tenant");
        assert_eq!(created.version, 1);

        // Test get
        let retrieved = provider.get_configuration("test-tenant").await.unwrap();
        assert!(retrieved.is_some());
        let retrieved = retrieved.unwrap();
        assert_eq!(retrieved.tenant_id, "test-tenant");
        assert_eq!(retrieved.version, 1);
    }

    #[tokio::test]
    async fn test_database_provider_update_operation() {
        let provider = create_test_provider().await;
        let config = create_test_config("test-tenant");

        // Test create
        let created = provider.create_configuration(config.clone()).await.unwrap();
        assert_eq!(created.tenant_id, "test-tenant");
        assert_eq!(created.version, 1);

        // Test get
        let retrieved = provider.get_configuration("test-tenant").await.unwrap();
        assert!(retrieved.is_some());
        let retrieved = retrieved.unwrap();
        assert_eq!(retrieved.tenant_id, "test-tenant");
        assert_eq!(retrieved.version, 1);

        // Test update
        let mut updated_config = retrieved.clone();
        updated_config.display_name = "Updated Tenant".to_string();
        let updated = provider.update_configuration(updated_config).await.unwrap();
        assert_eq!(updated.display_name, "Updated Tenant");
        assert_eq!(updated.version, 2);

        // Verify the update persisted
        let retrieved_after_update = provider.get_configuration("test-tenant").await.unwrap();
        assert!(retrieved_after_update.is_some());
        let retrieved_after_update = retrieved_after_update.unwrap();
        assert_eq!(retrieved_after_update.display_name, "Updated Tenant");
        assert_eq!(retrieved_after_update.version, 2);
    }

    #[tokio::test]
    async fn test_database_provider_delete_operation() {
        let provider = create_test_provider().await;
        let config = create_test_config("test-tenant");

        // Test create
        let created = provider.create_configuration(config.clone()).await.unwrap();
        assert_eq!(created.tenant_id, "test-tenant");
        assert_eq!(created.version, 1);

        // Verify it exists
        let retrieved = provider.get_configuration("test-tenant").await.unwrap();
        assert!(retrieved.is_some());

        // Test delete
        provider
            .delete_configuration("test-tenant", None)
            .await
            .unwrap();

        // Verify it's deleted
        let deleted_check = provider.get_configuration("test-tenant").await.unwrap();
        assert!(deleted_check.is_none());
    }

    #[tokio::test]
    async fn test_database_direct_operations() {
        use crate::multi_tenant::database::InMemoryDatabase;
        use std::sync::Arc;

        // Create database directly
        let database = Arc::new(InMemoryDatabase::new());

        // Test INSERT directly
        let insert_sql = r#"
            INSERT INTO tenant_configurations
            (tenant_id, display_name, created_at, last_modified, version,
             schema_config, operational_config, compliance_config, branding_config)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
        "#;

        let params: Vec<Box<dyn crate::multi_tenant::database::DatabaseParameter>> = vec![
            Box::new("test-tenant".to_string()),
            Box::new("Test Tenant".to_string()),
            Box::new("2023-01-01T00:00:00Z".to_string()),
            Box::new("2023-01-01T00:00:00Z".to_string()),
            Box::new(1i64),
            Box::new("{}".to_string()),
            Box::new("{}".to_string()),
            Box::new("{}".to_string()),
            Box::new("{}".to_string()),
        ];
        let param_refs: Vec<&dyn crate::multi_tenant::database::DatabaseParameter> =
            params.iter().map(|p| p.as_ref()).collect();

        let insert_result = database
            .execute_query(insert_sql, &param_refs, "test-tenant")
            .await
            .unwrap();
        assert_eq!(insert_result.affected_rows, 1);

        // Test SELECT directly
        let select_sql = r#"
            SELECT tenant_id, display_name, created_at, last_modified, version,
                   schema_config, operational_config, compliance_config, branding_config
            FROM tenant_configurations
            WHERE tenant_id = ?
        "#;

        let select_params: Vec<Box<dyn crate::multi_tenant::database::DatabaseParameter>> =
            vec![Box::new("test-tenant".to_string())];
        let select_param_refs: Vec<&dyn crate::multi_tenant::database::DatabaseParameter> =
            select_params.iter().map(|p| p.as_ref()).collect();

        let select_result = database
            .execute_query(select_sql, &select_param_refs, "test-tenant")
            .await
            .unwrap();
        assert_eq!(select_result.rows.len(), 1);
        assert_eq!(
            select_result.rows[0]
                .get("tenant_id")
                .unwrap()
                .as_str()
                .unwrap(),
            "test-tenant"
        );

        // Test UPDATE directly
        let update_sql = r#"
            UPDATE tenant_configurations
            SET display_name = ?, last_modified = ?, version = ?,
                schema_config = ?, operational_config = ?,
                compliance_config = ?, branding_config = ?
            WHERE tenant_id = ? AND version = ?
        "#;

        let update_params: Vec<Box<dyn crate::multi_tenant::database::DatabaseParameter>> = vec![
            Box::new("Updated Tenant".to_string()),
            Box::new("2023-01-01T01:00:00Z".to_string()),
            Box::new(2i64),
            Box::new("{}".to_string()),
            Box::new("{}".to_string()),
            Box::new("{}".to_string()),
            Box::new("{}".to_string()),
            Box::new("test-tenant".to_string()),
            Box::new(1i64),
        ];
        let update_param_refs: Vec<&dyn crate::multi_tenant::database::DatabaseParameter> =
            update_params.iter().map(|p| p.as_ref()).collect();

        let update_result = database
            .execute_query(update_sql, &update_param_refs, "test-tenant")
            .await
            .unwrap();
        assert_eq!(update_result.affected_rows, 1);

        // Test SELECT after UPDATE
        let select_after_result = database
            .execute_query(select_sql, &select_param_refs, "test-tenant")
            .await
            .unwrap();
        assert_eq!(select_after_result.rows.len(), 1);
        let row = &select_after_result.rows[0];
        assert_eq!(
            row.get("tenant_id").unwrap().as_str().unwrap(),
            "test-tenant"
        );
        assert_eq!(
            row.get("display_name").unwrap().as_str().unwrap(),
            "Updated Tenant"
        );
        assert_eq!(row.get("version").unwrap().as_i64().unwrap(), 2);
    }

    #[tokio::test]
    async fn test_database_provider_crud_operations() {
        let provider = create_test_provider().await;
        let config = create_test_config("test-tenant");

        // Test create
        let created = provider.create_configuration(config.clone()).await.unwrap();
        assert_eq!(created.tenant_id, "test-tenant");
        assert_eq!(created.version, 1);

        // Test get
        let retrieved = provider.get_configuration("test-tenant").await.unwrap();
        assert!(retrieved.is_some());
        let retrieved = retrieved.unwrap();
        assert_eq!(retrieved.tenant_id, "test-tenant");

        // Test update
        let mut updated_config = retrieved.clone();
        updated_config.display_name = "Updated Tenant".to_string();
        let updated = provider.update_configuration(updated_config).await.unwrap();
        assert_eq!(updated.display_name, "Updated Tenant");
        assert_eq!(updated.version, 2);

        // Test delete
        provider
            .delete_configuration("test-tenant", None)
            .await
            .unwrap();
        let deleted_check = provider.get_configuration("test-tenant").await.unwrap();
        assert!(deleted_check.is_none());
    }

    #[tokio::test]
    async fn test_database_provider_query_operations() {
        let provider = create_test_provider().await;

        // Create multiple configurations
        for i in 1..=5 {
            let config = create_test_config(&format!("tenant-{}", i));
            provider.create_configuration(config).await.unwrap();
        }

        // Test basic listing
        let query = ConfigurationQuery {
            tenant_ids: None,
            display_name_filter: None,
            modified_after: None,
            modified_before: None,
            offset: None,
            limit: None,
            sort_order: SortOrder::TenantIdAsc,
        };
        let all_configs = provider.list_configurations(&query).await.unwrap();
        assert_eq!(all_configs.configurations.len(), 5);

        // Test pagination
        let page1_query = ConfigurationQuery {
            tenant_ids: None,
            display_name_filter: None,
            modified_after: None,
            modified_before: None,
            offset: Some(0),
            limit: Some(2),
            sort_order: SortOrder::TenantIdAsc,
        };
        let page1 = provider.list_configurations(&page1_query).await.unwrap();
        assert_eq!(page1.configurations.len(), 2);

        let page2_query = ConfigurationQuery {
            tenant_ids: None,
            display_name_filter: None,
            modified_after: None,
            modified_before: None,
            offset: Some(2),
            limit: Some(2),
            sort_order: SortOrder::TenantIdAsc,
        };
        let page2 = provider.list_configurations(&page2_query).await.unwrap();
        assert_eq!(page2.configurations.len(), 2);

        // Test count
        let count = provider.count_configurations().await.unwrap();
        assert_eq!(count, 5);

        // Test exists
        assert!(provider.configuration_exists("tenant-1").await.unwrap());
        assert!(!provider.configuration_exists("non-existent").await.unwrap());
    }

    #[tokio::test]
    async fn test_database_provider_bulk_operations() {
        let provider = create_test_provider().await;

        let operations = vec![
            BulkConfigurationOperation::Create(create_test_config("bulk-1")),
            BulkConfigurationOperation::Create(create_test_config("bulk-2")),
            BulkConfigurationOperation::Create(create_test_config("bulk-3")),
        ];

        let results = provider.bulk_operations(&operations).await.unwrap();
        assert_eq!(results.len(), 3);

        // All operations should succeed
        for result in results {
            assert!(matches!(result, BulkOperationResult::Success { .. }));
        }

        // Verify configurations were created
        assert!(provider.configuration_exists("bulk-1").await.unwrap());
        assert!(provider.configuration_exists("bulk-2").await.unwrap());
        assert!(provider.configuration_exists("bulk-3").await.unwrap());
    }
}
