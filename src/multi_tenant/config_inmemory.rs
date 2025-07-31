//! In-memory tenant configuration provider implementation.
//!
//! This module provides an in-memory implementation of the `TenantConfigurationProvider`
//! trait, suitable for development, testing, and small deployments. It stores all
//! configurations in memory with optional caching capabilities.
//!
//! # Features
//!
//! * **Fast Access**: All configurations stored in memory for immediate access
//! * **Thread Safety**: Uses async-aware locks for concurrent access
//! * **Optional Persistence**: Can save/load configurations to/from JSON files
//! * **Full Provider API**: Implements all configuration provider methods
//! * **Statistics**: Tracks usage statistics and performance metrics
//!
//! # Example Usage
//!
//! ```rust,no_run
//! use scim_server::multi_tenant::{InMemoryConfigurationProvider, TenantConfiguration};
//! use std::sync::Arc;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // Create in-memory provider
//!     let provider = Arc::new(InMemoryConfigurationProvider::new());
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
    ConfigurationQuery, ConfigurationQueryResult, ConfigurationStats, ConfigurationValidator,
    SortOrder, TenantConfigurationProvider, ValidationContext,
};
use crate::multi_tenant::configuration::{ConfigurationError, TenantConfiguration};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use tokio::fs;
use tokio::sync::RwLock;

/// In-memory configuration provider with optional file persistence.
#[derive(Debug)]
pub struct InMemoryConfigurationProvider {
    /// Storage for configurations indexed by tenant ID
    configurations: Arc<RwLock<HashMap<String, TenantConfiguration>>>,
    /// Configuration provider settings
    settings: ProviderSettings,
    /// Usage statistics
    stats: Arc<RwLock<ProviderStats>>,
    /// Cache statistics
    cache_stats: Arc<RwLock<CacheStats>>,
}

/// Settings for the in-memory configuration provider.
#[derive(Debug, Clone)]
pub struct ProviderSettings {
    /// Whether to enable file persistence
    pub enable_persistence: bool,
    /// File path for persistence (if enabled)
    pub persistence_file: Option<String>,
    /// Maximum number of configurations to store
    pub max_configurations: Option<usize>,
    /// Whether to enable detailed statistics tracking
    pub enable_detailed_stats: bool,
    /// Auto-save interval in seconds (0 to disable)
    pub auto_save_interval_secs: u64,
}

impl Default for ProviderSettings {
    fn default() -> Self {
        Self {
            enable_persistence: false,
            persistence_file: None,
            max_configurations: Some(10000),
            enable_detailed_stats: true,
            auto_save_interval_secs: 0,
        }
    }
}

/// Internal statistics for the provider.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct ProviderStats {
    /// Total number of create operations
    pub creates: u64,
    /// Total number of read operations
    pub reads: u64,
    /// Total number of update operations
    pub updates: u64,
    /// Total number of delete operations
    pub deletes: u64,
    /// Total number of list operations
    pub lists: u64,
    /// Total number of validation operations
    pub validations: u64,
    /// Number of validation errors
    pub validation_errors: u64,
    /// Number of version conflicts
    pub version_conflicts: u64,
    /// Average operation time in microseconds
    pub average_operation_time_us: u64,
    /// When the provider was created
    pub created_at: DateTime<Utc>,
    /// Last operation timestamp
    pub last_operation_at: Option<DateTime<Utc>>,
}

impl InMemoryConfigurationProvider {
    /// Create a new in-memory configuration provider with default settings.
    pub fn new() -> Self {
        Self::with_settings(ProviderSettings::default())
    }

    /// Create a new in-memory configuration provider with custom settings.
    pub fn with_settings(settings: ProviderSettings) -> Self {
        let mut provider_stats = ProviderStats::default();
        provider_stats.created_at = Utc::now();

        let cache_stats = CacheStats {
            cache_hits: 0,
            cache_misses: 0,
            hit_ratio: 0.0,
            cached_items: 0,
            memory_usage: 0,
            evictions: 0,
            average_lookup_time_us: 0,
        };

        Self {
            configurations: Arc::new(RwLock::new(HashMap::new())),
            settings,
            stats: Arc::new(RwLock::new(provider_stats)),
            cache_stats: Arc::new(RwLock::new(cache_stats)),
        }
    }

    /// Create a provider with file persistence enabled.
    pub fn with_persistence<P: AsRef<Path>>(file_path: P) -> Self {
        let mut settings = ProviderSettings::default();
        settings.enable_persistence = true;
        settings.persistence_file = Some(file_path.as_ref().to_string_lossy().to_string());
        Self::with_settings(settings)
    }

    /// Load configurations from the persistence file.
    pub async fn load_from_file(&self) -> Result<usize, ConfigurationError> {
        if !self.settings.enable_persistence {
            return Err(ConfigurationError::ValidationError {
                message: "Persistence is not enabled".to_string(),
            });
        }

        let file_path = self.settings.persistence_file.as_ref().ok_or_else(|| {
            ConfigurationError::ValidationError {
                message: "No persistence file configured".to_string(),
            }
        })?;

        if !tokio::fs::try_exists(file_path).await.unwrap_or(false) {
            return Ok(0); // File doesn't exist, nothing to load
        }

        let content = fs::read_to_string(file_path).await.map_err(|e| {
            ConfigurationError::ValidationError {
                message: format!("Failed to read persistence file: {}", e),
            }
        })?;

        let stored_configs: Vec<TenantConfiguration> = serde_json::from_str(&content)
            .map_err(|e| ConfigurationError::SerializationError { source: e })?;

        let mut configurations = self.configurations.write().await;
        let count = stored_configs.len();

        for config in stored_configs {
            configurations.insert(config.tenant_id.clone(), config);
        }

        Ok(count)
    }

    /// Save configurations to the persistence file.
    pub async fn save_to_file(&self) -> Result<usize, ConfigurationError> {
        if !self.settings.enable_persistence {
            return Err(ConfigurationError::ValidationError {
                message: "Persistence is not enabled".to_string(),
            });
        }

        let file_path = self.settings.persistence_file.as_ref().ok_or_else(|| {
            ConfigurationError::ValidationError {
                message: "No persistence file configured".to_string(),
            }
        })?;

        let configurations = self.configurations.read().await;
        let configs: Vec<TenantConfiguration> = configurations.values().cloned().collect();
        let count = configs.len();

        let content = serde_json::to_string_pretty(&configs)
            .map_err(|e| ConfigurationError::SerializationError { source: e })?;

        fs::write(file_path, content)
            .await
            .map_err(|e| ConfigurationError::ValidationError {
                message: format!("Failed to write persistence file: {}", e),
            })?;

        Ok(count)
    }

    /// Get the current number of stored configurations.
    pub async fn size(&self) -> usize {
        self.configurations.read().await.len()
    }

    /// Clear all configurations from memory.
    pub async fn clear(&self) -> usize {
        let mut configurations = self.configurations.write().await;
        let count = configurations.len();
        configurations.clear();
        count
    }

    /// Record operation statistics.
    async fn record_operation(&self, operation: &str, duration_us: u64) {
        if !self.settings.enable_detailed_stats {
            return;
        }

        let mut stats = self.stats.write().await;
        stats.last_operation_at = Some(Utc::now());

        match operation {
            "create" => stats.creates += 1,
            "read" => stats.reads += 1,
            "update" => stats.updates += 1,
            "delete" => stats.deletes += 1,
            "list" => stats.lists += 1,
            "validate" => stats.validations += 1,
            _ => {}
        }

        // Update average operation time using exponential moving average
        if stats.average_operation_time_us == 0 {
            stats.average_operation_time_us = duration_us;
        } else {
            stats.average_operation_time_us =
                (stats.average_operation_time_us * 9 + duration_us) / 10;
        }
    }

    /// Record cache hit.
    async fn record_cache_hit(&self) {
        let mut cache_stats = self.cache_stats.write().await;
        cache_stats.cache_hits += 1;
        cache_stats.calculate_hit_ratio();
    }

    /// Record cache miss.
    async fn record_cache_miss(&self) {
        let mut cache_stats = self.cache_stats.write().await;
        cache_stats.cache_misses += 1;
        cache_stats.calculate_hit_ratio();
    }

    /// Update cache item count.
    async fn update_cache_items(&self, count: usize) {
        let mut cache_stats = self.cache_stats.write().await;
        cache_stats.cached_items = count;
    }

    /// Filter and sort configurations based on query parameters.
    fn filter_and_sort_configurations(
        configurations: &HashMap<String, TenantConfiguration>,
        query: &ConfigurationQuery,
    ) -> Vec<TenantConfiguration> {
        let mut filtered: Vec<TenantConfiguration> = configurations
            .values()
            .filter(|config| {
                // Filter by tenant IDs
                if let Some(ref tenant_ids) = query.tenant_ids {
                    if !tenant_ids.contains(&config.tenant_id) {
                        return false;
                    }
                }

                // Filter by display name
                if let Some(ref filter) = query.display_name_filter {
                    if !config
                        .display_name
                        .to_lowercase()
                        .contains(&filter.to_lowercase())
                    {
                        return false;
                    }
                }

                // Filter by modification date
                if let Some(after) = query.modified_after {
                    if config.last_modified <= after {
                        return false;
                    }
                }

                if let Some(before) = query.modified_before {
                    if config.last_modified >= before {
                        return false;
                    }
                }

                true
            })
            .cloned()
            .collect();

        // Sort results
        match query.sort_order {
            SortOrder::TenantIdAsc => {
                filtered.sort_by(|a, b| a.tenant_id.cmp(&b.tenant_id));
            }
            SortOrder::TenantIdDesc => {
                filtered.sort_by(|a, b| b.tenant_id.cmp(&a.tenant_id));
            }
            SortOrder::DisplayNameAsc => {
                filtered.sort_by(|a, b| a.display_name.cmp(&b.display_name));
            }
            SortOrder::DisplayNameDesc => {
                filtered.sort_by(|a, b| b.display_name.cmp(&a.display_name));
            }
            SortOrder::LastModifiedAsc => {
                filtered.sort_by(|a, b| a.last_modified.cmp(&b.last_modified));
            }
            SortOrder::LastModifiedDesc => {
                filtered.sort_by(|a, b| b.last_modified.cmp(&a.last_modified));
            }
            SortOrder::CreatedAsc => {
                filtered.sort_by(|a, b| a.created_at.cmp(&b.created_at));
            }
            SortOrder::CreatedDesc => {
                filtered.sort_by(|a, b| b.created_at.cmp(&a.created_at));
            }
        }

        filtered
    }
}

impl Default for InMemoryConfigurationProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl ConfigurationValidator for InMemoryConfigurationProvider {}

#[async_trait]
impl TenantConfigurationProvider for InMemoryConfigurationProvider {
    type Error = ConfigurationError;

    async fn create_configuration(
        &self,
        mut configuration: TenantConfiguration,
    ) -> Result<TenantConfiguration, Self::Error> {
        let start_time = std::time::Instant::now();

        // Validate tenant ID
        self.validate_tenant_id(&configuration.tenant_id)?;

        // Validate configuration
        configuration.validate()?;
        self.validate_configuration_size(&configuration)?;

        // Check if we're at capacity
        if let Some(max_configs) = self.settings.max_configurations {
            let current_count = self.configurations.read().await.len();
            if current_count >= max_configs {
                return Err(ConfigurationError::ValidationError {
                    message: format!("Maximum configuration limit ({}) reached", max_configs),
                });
            }
        }

        let mut configurations = self.configurations.write().await;

        // Check for conflicts
        if configurations.contains_key(&configuration.tenant_id) {
            return Err(ConfigurationError::Conflict {
                message: format!(
                    "Configuration already exists for tenant: {}",
                    configuration.tenant_id
                ),
            });
        }

        // Set creation metadata
        let now = Utc::now();
        configuration.created_at = now;
        configuration.last_modified = now;
        configuration.version = 1;

        // Store the configuration
        let tenant_id = configuration.tenant_id.clone();
        configurations.insert(tenant_id, configuration.clone());

        // Update statistics
        let duration = start_time.elapsed().as_micros() as u64;
        self.record_operation("create", duration).await;
        self.update_cache_items(configurations.len()).await;

        // Auto-save if enabled
        if self.settings.enable_persistence && self.settings.auto_save_interval_secs > 0 {
            drop(configurations); // Release the lock before async operation
            let _ = self.save_to_file().await; // Ignore errors for auto-save
        }

        Ok(configuration)
    }

    async fn get_configuration(
        &self,
        tenant_id: &str,
    ) -> Result<Option<TenantConfiguration>, Self::Error> {
        let start_time = std::time::Instant::now();

        self.validate_tenant_id(tenant_id)?;

        let configurations = self.configurations.read().await;
        let result = configurations.get(tenant_id).cloned();

        // Update statistics
        let duration = start_time.elapsed().as_micros() as u64;
        self.record_operation("read", duration).await;

        if result.is_some() {
            self.record_cache_hit().await;
        } else {
            self.record_cache_miss().await;
        }

        Ok(result)
    }

    async fn update_configuration(
        &self,
        mut configuration: TenantConfiguration,
    ) -> Result<TenantConfiguration, Self::Error> {
        let start_time = std::time::Instant::now();

        // Validate tenant ID and configuration
        self.validate_tenant_id(&configuration.tenant_id)?;
        configuration.validate()?;
        self.validate_configuration_size(&configuration)?;

        let mut configurations = self.configurations.write().await;

        // Check if configuration exists
        let existing = configurations
            .get(&configuration.tenant_id)
            .ok_or_else(|| ConfigurationError::NotFound {
                tenant_id: configuration.tenant_id.clone(),
            })?;

        // Check version for optimistic locking
        if configuration.version != existing.version {
            let mut stats = self.stats.write().await;
            stats.version_conflicts += 1;
            return Err(ConfigurationError::VersionMismatch {
                expected: existing.version,
                actual: configuration.version,
            });
        }

        // Update metadata
        configuration.last_modified = Utc::now();
        configuration.version += 1;

        // Store the updated configuration
        let tenant_id = configuration.tenant_id.clone();
        configurations.insert(tenant_id, configuration.clone());

        // Update statistics
        let duration = start_time.elapsed().as_micros() as u64;
        self.record_operation("update", duration).await;

        // Auto-save if enabled
        if self.settings.enable_persistence && self.settings.auto_save_interval_secs > 0 {
            drop(configurations); // Release the lock before async operation
            let _ = self.save_to_file().await; // Ignore errors for auto-save
        }

        Ok(configuration)
    }

    async fn delete_configuration(
        &self,
        tenant_id: &str,
        expected_version: Option<u64>,
    ) -> Result<(), Self::Error> {
        let start_time = std::time::Instant::now();

        self.validate_tenant_id(tenant_id)?;

        let mut configurations = self.configurations.write().await;

        // Check if configuration exists
        let existing =
            configurations
                .get(tenant_id)
                .ok_or_else(|| ConfigurationError::NotFound {
                    tenant_id: tenant_id.to_string(),
                })?;

        // Check version for optimistic locking if provided
        if let Some(expected) = expected_version {
            if existing.version != expected {
                let mut stats = self.stats.write().await;
                stats.version_conflicts += 1;
                return Err(ConfigurationError::VersionMismatch {
                    expected,
                    actual: existing.version,
                });
            }
        }

        // Remove the configuration
        configurations.remove(tenant_id);

        // Update statistics
        let duration = start_time.elapsed().as_micros() as u64;
        self.record_operation("delete", duration).await;
        self.update_cache_items(configurations.len()).await;

        // Auto-save if enabled
        if self.settings.enable_persistence && self.settings.auto_save_interval_secs > 0 {
            drop(configurations); // Release the lock before async operation
            let _ = self.save_to_file().await; // Ignore errors for auto-save
        }

        Ok(())
    }

    async fn configuration_exists(&self, tenant_id: &str) -> Result<bool, Self::Error> {
        self.validate_tenant_id(tenant_id)?;
        let configurations = self.configurations.read().await;
        Ok(configurations.contains_key(tenant_id))
    }

    async fn list_configurations(
        &self,
        query: &ConfigurationQuery,
    ) -> Result<ConfigurationQueryResult, Self::Error> {
        let start_time = std::time::Instant::now();

        let configurations = self.configurations.read().await;
        let filtered = Self::filter_and_sort_configurations(&configurations, query);

        let total_count = filtered.len();
        let offset = query.offset.unwrap_or(0);
        let limit = query.limit.unwrap_or(total_count);

        let paginated: Vec<TenantConfiguration> =
            filtered.into_iter().skip(offset).take(limit).collect();

        let has_more = offset + paginated.len() < total_count;
        let next_offset = if has_more {
            Some(offset + paginated.len())
        } else {
            None
        };

        // Update statistics
        let duration = start_time.elapsed().as_micros() as u64;
        self.record_operation("list", duration).await;

        Ok(ConfigurationQueryResult {
            configurations: paginated,
            total_count,
            has_more,
            next_offset,
        })
    }

    async fn count_configurations(&self) -> Result<usize, Self::Error> {
        Ok(self.configurations.read().await.len())
    }

    async fn get_configuration_stats(&self) -> Result<ConfigurationStats, Self::Error> {
        let configurations = self.configurations.read().await;
        let _stats = self.stats.read().await;

        let total_configurations = configurations.len();
        let now = Utc::now();
        let twenty_four_hours_ago = now - chrono::Duration::hours(24);

        let recent_configurations = configurations
            .values()
            .filter(|c| c.created_at > twenty_four_hours_ago)
            .count();

        let recently_modified = configurations
            .values()
            .filter(|c| c.last_modified > twenty_four_hours_ago)
            .count();

        let total_size: usize = configurations
            .values()
            .map(|c| serde_json::to_string(c).unwrap_or_default().len())
            .sum();

        let average_size = if total_configurations > 0 {
            total_size / total_configurations
        } else {
            0
        };

        let newest_configuration = configurations.values().map(|c| c.created_at).max();

        let oldest_configuration = configurations.values().map(|c| c.created_at).min();

        let mut version_distribution = HashMap::new();
        for config in configurations.values() {
            *version_distribution.entry(config.version).or_insert(0) += 1;
        }

        Ok(ConfigurationStats {
            total_configurations,
            recent_configurations,
            recently_modified,
            average_size,
            total_storage_used: total_size,
            newest_configuration,
            oldest_configuration,
            version_distribution,
        })
    }

    async fn validate_configuration(
        &self,
        configuration: &TenantConfiguration,
        _context: &ValidationContext,
    ) -> Result<(), Self::Error> {
        let start_time = std::time::Instant::now();

        self.validate_tenant_id(&configuration.tenant_id)?;
        self.validate_version(configuration.version)?;
        configuration.validate()?;
        self.validate_configuration_size(configuration)?;

        let duration = start_time.elapsed().as_micros() as u64;
        self.record_operation("validate", duration).await;

        Ok(())
    }

    async fn backup_configurations(
        &self,
        tenant_ids: Option<&[String]>,
        backup_location: &str,
    ) -> Result<String, Self::Error> {
        let configurations = self.configurations.read().await;

        let configs_to_backup: Vec<TenantConfiguration> = match tenant_ids {
            Some(ids) => configurations
                .values()
                .filter(|c| ids.contains(&c.tenant_id))
                .cloned()
                .collect(),
            None => configurations.values().cloned().collect(),
        };

        let backup_id = format!("backup-{}", Utc::now().timestamp());
        let backup_file = format!("{}/{}.json", backup_location, backup_id);

        let content = serde_json::to_string_pretty(&configs_to_backup)
            .map_err(|e| ConfigurationError::SerializationError { source: e })?;

        fs::write(&backup_file, content).await.map_err(|e| {
            ConfigurationError::ValidationError {
                message: format!("Failed to write backup file: {}", e),
            }
        })?;

        Ok(backup_id)
    }

    async fn restore_configurations(
        &self,
        backup_id: &str,
        backup_location: &str,
        overwrite_existing: bool,
    ) -> Result<usize, Self::Error> {
        let backup_file = format!("{}/{}.json", backup_location, backup_id);

        let content = fs::read_to_string(&backup_file).await.map_err(|e| {
            ConfigurationError::ValidationError {
                message: format!("Failed to read backup file: {}", e),
            }
        })?;

        let backup_configs: Vec<TenantConfiguration> = serde_json::from_str(&content)
            .map_err(|e| ConfigurationError::SerializationError { source: e })?;

        let mut configurations = self.configurations.write().await;
        let mut restored_count = 0;

        for config in backup_configs {
            if !overwrite_existing && configurations.contains_key(&config.tenant_id) {
                continue; // Skip if exists and not overwriting
            }

            configurations.insert(config.tenant_id.clone(), config);
            restored_count += 1;
        }

        self.update_cache_items(configurations.len()).await;

        Ok(restored_count)
    }

    async fn bulk_operations(
        &self,
        operations: &[BulkConfigurationOperation],
    ) -> Result<Vec<BulkOperationResult>, Self::Error> {
        let mut results = Vec::with_capacity(operations.len());

        for operation in operations {
            let result = match operation {
                BulkConfigurationOperation::Create(config) => {
                    match self.create_configuration(config.clone()).await {
                        Ok(created_config) => BulkOperationResult::Success {
                            tenant_id: created_config.tenant_id.clone(),
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
                            tenant_id: updated_config.tenant_id.clone(),
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
                } => match self
                    .delete_configuration(tenant_id, *expected_version)
                    .await
                {
                    Ok(()) => BulkOperationResult::Success {
                        tenant_id: tenant_id.clone(),
                        operation: "delete".to_string(),
                        configuration: None,
                    },
                    Err(e) => BulkOperationResult::Error {
                        tenant_id: tenant_id.clone(),
                        operation: "delete".to_string(),
                        error: e.to_string(),
                    },
                },
                BulkConfigurationOperation::Validate(config) => {
                    let context = ValidationContext {
                        is_create: true,
                        previous_configuration: None,
                        validation_params: HashMap::new(),
                    };
                    match self.validate_configuration(config, &context).await {
                        Ok(()) => BulkOperationResult::Success {
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
}

#[async_trait]
impl CachedConfigurationProvider for InMemoryConfigurationProvider {
    async fn clear_cache(&self, tenant_id: &str) -> Result<(), Self::Error> {
        self.validate_tenant_id(tenant_id)?;

        let mut configurations = self.configurations.write().await;
        configurations.remove(tenant_id);
        self.update_cache_items(configurations.len()).await;

        Ok(())
    }

    async fn clear_all_cache(&self) -> Result<(), Self::Error> {
        let cleared_count = self.clear().await;
        self.update_cache_items(0).await;

        // Reset cache statistics
        let mut cache_stats = self.cache_stats.write().await;
        *cache_stats = CacheStats {
            cache_hits: 0,
            cache_misses: 0,
            hit_ratio: 0.0,
            cached_items: 0,
            memory_usage: 0,
            evictions: cache_stats.evictions + cleared_count as u64,
            average_lookup_time_us: cache_stats.average_lookup_time_us,
        };

        Ok(())
    }

    async fn get_cache_stats(&self) -> Result<CacheStats, Self::Error> {
        let mut cache_stats = self.cache_stats.read().await.clone();
        cache_stats.cached_items = self.configurations.read().await.len();

        // Estimate memory usage
        let configurations = self.configurations.read().await;
        cache_stats.memory_usage = configurations
            .values()
            .map(|c| serde_json::to_string(c).unwrap_or_default().len())
            .sum();

        Ok(cache_stats)
    }

    async fn warm_cache(&self, tenant_ids: Option<&[String]>) -> Result<usize, Self::Error> {
        // For in-memory provider, all configurations are already "cached"
        let configurations = self.configurations.read().await;

        let count = match tenant_ids {
            Some(ids) => configurations
                .keys()
                .filter(|tenant_id| ids.contains(tenant_id))
                .count(),
            None => configurations.len(),
        };

        Ok(count)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::multi_tenant::config_provider::ConfigurationQuery;
    use tempfile::tempdir;

    async fn create_test_configuration(tenant_id: &str) -> TenantConfiguration {
        TenantConfiguration::builder(tenant_id.to_string())
            .with_display_name(format!("Test Tenant {}", tenant_id))
            .build()
            .expect("Should build test configuration")
    }

    #[tokio::test]
    async fn test_provider_creation() {
        let provider = InMemoryConfigurationProvider::new();
        assert_eq!(provider.size().await, 0);

        let provider_with_settings =
            InMemoryConfigurationProvider::with_settings(ProviderSettings {
                enable_persistence: false,
                max_configurations: Some(100),
                ..Default::default()
            });
        assert_eq!(provider_with_settings.size().await, 0);
    }

    #[tokio::test]
    async fn test_create_configuration() {
        let provider = InMemoryConfigurationProvider::new();
        let config = create_test_configuration("tenant-a").await;

        let created = provider
            .create_configuration(config.clone())
            .await
            .expect("Should create configuration");

        assert_eq!(created.tenant_id, "tenant-a");
        assert_eq!(created.version, 1);
        assert!(created.created_at <= Utc::now());
        assert!(created.last_modified <= Utc::now());
    }

    #[tokio::test]
    async fn test_create_duplicate_configuration() {
        let provider = InMemoryConfigurationProvider::new();
        let config = create_test_configuration("tenant-a").await;

        provider
            .create_configuration(config.clone())
            .await
            .expect("Should create first configuration");

        let result = provider.create_configuration(config).await;
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            ConfigurationError::Conflict { .. }
        ));
    }

    #[tokio::test]
    async fn test_get_configuration() {
        let provider = InMemoryConfigurationProvider::new();
        let config = create_test_configuration("tenant-a").await;

        // Non-existent configuration
        let result = provider
            .get_configuration("tenant-a")
            .await
            .expect("Should not error");
        assert!(result.is_none());

        // Create and retrieve
        provider
            .create_configuration(config)
            .await
            .expect("Should create configuration");

        let retrieved = provider
            .get_configuration("tenant-a")
            .await
            .expect("Should retrieve configuration")
            .expect("Should find configuration");

        assert_eq!(retrieved.tenant_id, "tenant-a");
    }

    #[tokio::test]
    async fn test_update_configuration() {
        let provider = InMemoryConfigurationProvider::new();
        let config = create_test_configuration("tenant-a").await;

        let created = provider
            .create_configuration(config)
            .await
            .expect("Should create configuration");

        let mut updated_config = created.clone();
        updated_config.display_name = "Updated Name".to_string();

        let updated = provider
            .update_configuration(updated_config)
            .await
            .expect("Should update configuration");

        assert_eq!(updated.display_name, "Updated Name");
        assert_eq!(updated.version, 2);
        assert!(updated.last_modified > created.last_modified);
    }

    #[tokio::test]
    async fn test_update_nonexistent_configuration() {
        let provider = InMemoryConfigurationProvider::new();
        let config = create_test_configuration("tenant-a").await;

        let result = provider.update_configuration(config).await;
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            ConfigurationError::NotFound { .. }
        ));
    }

    #[tokio::test]
    async fn test_version_mismatch() {
        let provider = InMemoryConfigurationProvider::new();
        let config = create_test_configuration("tenant-a").await;

        let created = provider
            .create_configuration(config)
            .await
            .expect("Should create configuration");

        let mut stale_config = created.clone();
        stale_config.version = 999; // Wrong version

        let result = provider.update_configuration(stale_config).await;
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            ConfigurationError::VersionMismatch { .. }
        ));
    }

    #[tokio::test]
    async fn test_delete_configuration() {
        let provider = InMemoryConfigurationProvider::new();
        let config = create_test_configuration("tenant-a").await;

        let created = provider
            .create_configuration(config)
            .await
            .expect("Should create configuration");

        provider
            .delete_configuration("tenant-a", Some(created.version))
            .await
            .expect("Should delete configuration");

        let result = provider
            .get_configuration("tenant-a")
            .await
            .expect("Should not error");
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_delete_with_wrong_version() {
        let provider = InMemoryConfigurationProvider::new();
        let config = create_test_configuration("tenant-a").await;

        provider
            .create_configuration(config)
            .await
            .expect("Should create configuration");

        let result = provider.delete_configuration("tenant-a", Some(999)).await;
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            ConfigurationError::VersionMismatch { .. }
        ));
    }

    #[tokio::test]
    async fn test_configuration_exists() {
        let provider = InMemoryConfigurationProvider::new();
        let config = create_test_configuration("tenant-a").await;

        assert!(
            !provider
                .configuration_exists("tenant-a")
                .await
                .expect("Should not error")
        );

        provider
            .create_configuration(config)
            .await
            .expect("Should create configuration");

        assert!(
            provider
                .configuration_exists("tenant-a")
                .await
                .expect("Should not error")
        );
    }

    #[tokio::test]
    async fn test_list_configurations() {
        let provider = InMemoryConfigurationProvider::new();

        // Create multiple configurations
        for i in 1..=5 {
            let config = create_test_configuration(&format!("tenant-{}", i)).await;
            provider
                .create_configuration(config)
                .await
                .expect("Should create configuration");
        }

        let query = ConfigurationQuery::default();
        let result = provider
            .list_configurations(&query)
            .await
            .expect("Should list configurations");

        assert_eq!(result.configurations.len(), 5);
        assert_eq!(result.total_count, 5);
        assert!(!result.has_more);
    }

    #[tokio::test]
    async fn test_list_configurations_with_pagination() {
        let provider = InMemoryConfigurationProvider::new();

        // Create multiple configurations
        for i in 1..=10 {
            let config = create_test_configuration(&format!("tenant-{:02}", i)).await;
            provider
                .create_configuration(config)
                .await
                .expect("Should create configuration");
        }

        let query = ConfigurationQuery {
            offset: Some(3),
            limit: Some(4),
            ..Default::default()
        };
        let result = provider
            .list_configurations(&query)
            .await
            .expect("Should list configurations");

        assert_eq!(result.configurations.len(), 4);
        assert_eq!(result.total_count, 10);
        assert!(result.has_more);
        assert_eq!(result.next_offset, Some(7));
    }

    #[tokio::test]
    async fn test_list_configurations_with_filtering() {
        let provider = InMemoryConfigurationProvider::new();

        let config1 = TenantConfiguration::builder("alpha-tenant".to_string())
            .with_display_name("Alpha Company".to_string())
            .build()
            .expect("Should build configuration");

        let config2 = TenantConfiguration::builder("beta-tenant".to_string())
            .with_display_name("Beta Corporation".to_string())
            .build()
            .expect("Should build configuration");

        provider
            .create_configuration(config1)
            .await
            .expect("Should create configuration");
        provider
            .create_configuration(config2)
            .await
            .expect("Should create configuration");

        let query = ConfigurationQuery {
            display_name_filter: Some("Alpha".to_string()),
            ..Default::default()
        };
        let result = provider
            .list_configurations(&query)
            .await
            .expect("Should list configurations");

        assert_eq!(result.configurations.len(), 1);
        assert_eq!(result.configurations[0].tenant_id, "alpha-tenant");
    }

    #[tokio::test]
    async fn test_count_configurations() {
        let provider = InMemoryConfigurationProvider::new();

        assert_eq!(
            provider
                .count_configurations()
                .await
                .expect("Should not error"),
            0
        );

        let config = create_test_configuration("tenant-a").await;
        provider
            .create_configuration(config)
            .await
            .expect("Should create configuration");

        assert_eq!(
            provider
                .count_configurations()
                .await
                .expect("Should not error"),
            1
        );
    }

    #[tokio::test]
    async fn test_get_configuration_stats() {
        let provider = InMemoryConfigurationProvider::new();

        let stats = provider
            .get_configuration_stats()
            .await
            .expect("Should get stats");
        assert_eq!(stats.total_configurations, 0);

        let config = create_test_configuration("tenant-a").await;
        provider
            .create_configuration(config)
            .await
            .expect("Should create configuration");

        let stats = provider
            .get_configuration_stats()
            .await
            .expect("Should get stats");
        assert_eq!(stats.total_configurations, 1);
        assert!(stats.average_size > 0);
        assert!(stats.newest_configuration.is_some());
    }

    #[tokio::test]
    async fn test_validate_configuration() {
        let provider = InMemoryConfigurationProvider::new();
        let config = create_test_configuration("tenant-a").await;

        let context = ValidationContext {
            is_create: true,
            previous_configuration: None,
            validation_params: HashMap::new(),
        };

        provider
            .validate_configuration(&config, &context)
            .await
            .expect("Should validate configuration");
    }

    #[tokio::test]
    async fn test_bulk_operations() {
        let provider = InMemoryConfigurationProvider::new();

        let config1 = create_test_configuration("tenant-1").await;
        let config2 = create_test_configuration("tenant-2").await;

        let operations = vec![
            BulkConfigurationOperation::Create(config1),
            BulkConfigurationOperation::Create(config2),
            BulkConfigurationOperation::Delete {
                tenant_id: "nonexistent".to_string(),
                expected_version: None,
            },
        ];

        let results = provider
            .bulk_operations(&operations)
            .await
            .expect("Should perform bulk operations");

        assert_eq!(results.len(), 3);
        assert!(matches!(results[0], BulkOperationResult::Success { .. }));
        assert!(matches!(results[1], BulkOperationResult::Success { .. }));
        assert!(matches!(results[2], BulkOperationResult::Error { .. }));
    }

    #[tokio::test]
    async fn test_cache_operations() {
        let provider = InMemoryConfigurationProvider::new();
        let config = create_test_configuration("tenant-a").await;

        provider
            .create_configuration(config)
            .await
            .expect("Should create configuration");

        // Test cache stats
        let stats = provider
            .get_cache_stats()
            .await
            .expect("Should get cache stats");
        assert_eq!(stats.cached_items, 1);

        // Test cache clearing
        provider
            .clear_cache("tenant-a")
            .await
            .expect("Should clear cache");

        let stats = provider
            .get_cache_stats()
            .await
            .expect("Should get cache stats");
        assert_eq!(stats.cached_items, 0);
    }

    #[tokio::test]
    async fn test_persistence() {
        let temp_dir = tempdir().expect("Should create temp dir");
        let file_path = temp_dir.path().join("test_configs.json");

        let provider = InMemoryConfigurationProvider::with_persistence(&file_path);

        // Create configurations
        for i in 1..=3 {
            let config = create_test_configuration(&format!("tenant-{}", i)).await;
            provider
                .create_configuration(config)
                .await
                .expect("Should create configuration");
        }

        // Save to file
        let saved_count = provider.save_to_file().await.expect("Should save to file");
        assert_eq!(saved_count, 3);

        // Clear memory and reload
        provider.clear().await;
        assert_eq!(provider.size().await, 0);

        let loaded_count = provider
            .load_from_file()
            .await
            .expect("Should load from file");
        assert_eq!(loaded_count, 3);
        assert_eq!(provider.size().await, 3);
    }

    #[tokio::test]
    async fn test_backup_and_restore() {
        let provider = InMemoryConfigurationProvider::new();
        let temp_dir = tempdir().expect("Should create temp dir");
        let backup_location = temp_dir.path().to_string_lossy().to_string();

        // Create configurations
        for i in 1..=3 {
            let config = create_test_configuration(&format!("tenant-{}", i)).await;
            provider
                .create_configuration(config)
                .await
                .expect("Should create configuration");
        }

        // Backup all configurations
        let backup_id = provider
            .backup_configurations(None, &backup_location)
            .await
            .expect("Should backup configurations");
        assert!(backup_id.starts_with("backup-"));

        // Clear and restore
        provider.clear().await;
        assert_eq!(provider.size().await, 0);

        let restored_count = provider
            .restore_configurations(&backup_id, &backup_location, true)
            .await
            .expect("Should restore configurations");
        assert_eq!(restored_count, 3);
        assert_eq!(provider.size().await, 3);
    }

    #[tokio::test]
    async fn test_capacity_limits() {
        let settings = ProviderSettings {
            max_configurations: Some(2),
            ..Default::default()
        };
        let provider = InMemoryConfigurationProvider::with_settings(settings);

        // Should allow up to the limit
        for i in 1..=2 {
            let config = create_test_configuration(&format!("tenant-{}", i)).await;
            provider
                .create_configuration(config)
                .await
                .expect("Should create configuration");
        }

        // Should reject beyond the limit
        let config = create_test_configuration("tenant-3").await;
        let result = provider.create_configuration(config).await;
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Maximum configuration limit")
        );
    }

    #[tokio::test]
    async fn test_invalid_tenant_id() {
        let provider = InMemoryConfigurationProvider::new();

        // Test empty tenant ID
        let result = provider.get_configuration("").await;
        assert!(result.is_err());

        // Test invalid characters
        let result = provider.get_configuration("tenant@invalid").await;
        assert!(result.is_err());

        // Test too long tenant ID
        let long_id = "a".repeat(300);
        let result = provider.get_configuration(&long_id).await;
        assert!(result.is_err());
    }
}
