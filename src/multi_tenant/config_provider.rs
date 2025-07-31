//! Configuration provider trait for multi-tenant SCIM operations.
//!
//! This module defines the `TenantConfigurationProvider` trait that enables
//! storage, retrieval, and management of tenant-specific configurations.
//! Implementations can use various backends such as databases, files, or
//! external configuration services.
//!
//! # Design Principles
//!
//! * **Async Operations**: All operations are async for scalability
//! * **Type Safety**: Strong typing for configuration operations
//! * **Versioning**: Optimistic locking with configuration versions
//! * **Validation**: Built-in validation before storing configurations
//!
//! # Example Usage
//!
//! ```rust,no_run
//! use scim_server::multi_tenant::{TenantConfigurationProvider, TenantConfiguration};
//! use std::sync::Arc;
//!
//! async fn example(provider: Arc<dyn TenantConfigurationProvider>) -> Result<(), Box<dyn std::error::Error>> {
//!     // Get configuration for a tenant
//!     let config = provider.get_configuration("tenant-a").await?;
//!
//!     // Update configuration
//!     let mut updated_config = config.clone();
//!     updated_config.touch();
//!     provider.update_configuration(updated_config).await?;
//!
//!     // List all tenant configurations
//!     let all_configs = provider.list_configurations().await?;
//!     println!("Found {} tenant configurations", all_configs.len());
//!
//!     Ok(())
//! }
//! ```

use crate::multi_tenant::configuration::{ConfigurationError, TenantConfiguration};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Configuration query parameters for filtering and pagination.
#[derive(Debug, Clone, Default)]
pub struct ConfigurationQuery {
    /// Filter by tenant IDs
    pub tenant_ids: Option<Vec<String>>,
    /// Filter by display name pattern
    pub display_name_filter: Option<String>,
    /// Filter configurations modified after this date
    pub modified_after: Option<DateTime<Utc>>,
    /// Filter configurations modified before this date
    pub modified_before: Option<DateTime<Utc>>,
    /// Pagination offset
    pub offset: Option<usize>,
    /// Maximum number of results to return
    pub limit: Option<usize>,
    /// Sort order for results
    pub sort_order: SortOrder,
}

/// Sort order for configuration queries.
#[derive(Debug, Clone, Default)]
pub enum SortOrder {
    /// Sort by tenant ID ascending
    #[default]
    TenantIdAsc,
    /// Sort by tenant ID descending
    TenantIdDesc,
    /// Sort by display name ascending
    DisplayNameAsc,
    /// Sort by display name descending
    DisplayNameDesc,
    /// Sort by last modified ascending (oldest first)
    LastModifiedAsc,
    /// Sort by last modified descending (newest first)
    LastModifiedDesc,
    /// Sort by creation date ascending
    CreatedAsc,
    /// Sort by creation date descending
    CreatedDesc,
}

/// Result of a configuration query operation.
#[derive(Debug, Clone)]
pub struct ConfigurationQueryResult {
    /// The configurations matching the query
    pub configurations: Vec<TenantConfiguration>,
    /// Total number of configurations (before pagination)
    pub total_count: usize,
    /// Whether there are more results available
    pub has_more: bool,
    /// Next offset for pagination
    pub next_offset: Option<usize>,
}

/// Statistics about configuration storage and usage.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigurationStats {
    /// Total number of tenant configurations
    pub total_configurations: usize,
    /// Number of configurations created in the last 24 hours
    pub recent_configurations: usize,
    /// Number of configurations modified in the last 24 hours
    pub recently_modified: usize,
    /// Average configuration size in bytes
    pub average_size: usize,
    /// Total storage used by configurations in bytes
    pub total_storage_used: usize,
    /// Most recently created configuration
    pub newest_configuration: Option<DateTime<Utc>>,
    /// Oldest configuration
    pub oldest_configuration: Option<DateTime<Utc>>,
    /// Configuration version distribution
    pub version_distribution: HashMap<u64, usize>,
}

/// Configuration validation context for provider-specific validation.
#[derive(Debug, Clone)]
pub struct ValidationContext {
    /// Whether this is a new configuration (create) or update
    pub is_create: bool,
    /// Previous configuration for updates
    pub previous_configuration: Option<TenantConfiguration>,
    /// Additional validation parameters
    pub validation_params: HashMap<String, String>,
}

/// Trait for providing tenant configuration storage and management.
///
/// Implementations of this trait handle the persistence and retrieval of
/// tenant configurations, providing a clean abstraction over different
/// storage backends.
#[async_trait]
pub trait TenantConfigurationProvider: Send + Sync {
    /// Error type for configuration operations
    type Error: std::error::Error + Send + Sync + 'static;

    /// Create a new tenant configuration.
    ///
    /// This method stores a new configuration for a tenant. The configuration
    /// is validated before storage, and the operation fails if the tenant
    /// already has a configuration.
    ///
    /// # Arguments
    ///
    /// * `configuration` - The configuration to create
    ///
    /// # Returns
    ///
    /// The created configuration with any provider-specific modifications
    /// (such as generated timestamps or IDs).
    ///
    /// # Errors
    ///
    /// * `ConfigurationError::ValidationError` - If the configuration is invalid
    /// * `ConfigurationError::Conflict` - If a configuration already exists for this tenant
    async fn create_configuration(
        &self,
        configuration: TenantConfiguration,
    ) -> Result<TenantConfiguration, Self::Error>;

    /// Get a tenant configuration by tenant ID.
    ///
    /// # Arguments
    ///
    /// * `tenant_id` - The unique identifier of the tenant
    ///
    /// # Returns
    ///
    /// The tenant configuration if found, or None if no configuration exists.
    async fn get_configuration(
        &self,
        tenant_id: &str,
    ) -> Result<Option<TenantConfiguration>, Self::Error>;

    /// Update an existing tenant configuration.
    ///
    /// This method updates an existing configuration using optimistic locking
    /// based on the configuration version. The operation fails if the version
    /// doesn't match the stored version.
    ///
    /// # Arguments
    ///
    /// * `configuration` - The updated configuration
    ///
    /// # Returns
    ///
    /// The updated configuration with incremented version.
    ///
    /// # Errors
    ///
    /// * `ConfigurationError::NotFound` - If the configuration doesn't exist
    /// * `ConfigurationError::VersionMismatch` - If the version is outdated
    /// * `ConfigurationError::ValidationError` - If the updated configuration is invalid
    async fn update_configuration(
        &self,
        configuration: TenantConfiguration,
    ) -> Result<TenantConfiguration, Self::Error>;

    /// Delete a tenant configuration.
    ///
    /// # Arguments
    ///
    /// * `tenant_id` - The unique identifier of the tenant
    /// * `expected_version` - Expected version for optimistic locking
    ///
    /// # Errors
    ///
    /// * `ConfigurationError::NotFound` - If the configuration doesn't exist
    /// * `ConfigurationError::VersionMismatch` - If the version is outdated
    async fn delete_configuration(
        &self,
        tenant_id: &str,
        expected_version: Option<u64>,
    ) -> Result<(), Self::Error>;

    /// Check if a configuration exists for a tenant.
    ///
    /// # Arguments
    ///
    /// * `tenant_id` - The unique identifier of the tenant
    ///
    /// # Returns
    ///
    /// True if a configuration exists, false otherwise.
    async fn configuration_exists(&self, tenant_id: &str) -> Result<bool, Self::Error>;

    /// List configurations based on query parameters.
    ///
    /// # Arguments
    ///
    /// * `query` - Query parameters for filtering and pagination
    ///
    /// # Returns
    ///
    /// A result containing the matching configurations and metadata.
    async fn list_configurations(
        &self,
        query: &ConfigurationQuery,
    ) -> Result<ConfigurationQueryResult, Self::Error>;

    /// Get all tenant configurations.
    ///
    /// This is a convenience method equivalent to calling `list_configurations`
    /// with an empty query.
    ///
    /// # Returns
    ///
    /// All tenant configurations in the system.
    async fn get_all_configurations(&self) -> Result<Vec<TenantConfiguration>, Self::Error> {
        let query = ConfigurationQuery::default();
        let result = self.list_configurations(&query).await?;
        Ok(result.configurations)
    }

    /// Count the total number of configurations.
    ///
    /// # Returns
    ///
    /// The total number of tenant configurations.
    async fn count_configurations(&self) -> Result<usize, Self::Error>;

    /// Get configuration statistics and metrics.
    ///
    /// # Returns
    ///
    /// Statistics about configuration storage and usage.
    async fn get_configuration_stats(&self) -> Result<ConfigurationStats, Self::Error>;

    /// Validate a configuration before storage.
    ///
    /// This method allows providers to implement custom validation logic
    /// beyond the basic configuration validation.
    ///
    /// # Arguments
    ///
    /// * `configuration` - The configuration to validate
    /// * `context` - Validation context with additional information
    ///
    /// # Returns
    ///
    /// Ok(()) if validation passes, or an error describing the validation failure.
    async fn validate_configuration(
        &self,
        configuration: &TenantConfiguration,
        context: &ValidationContext,
    ) -> Result<(), Self::Error>;

    /// Backup configurations to an external location.
    ///
    /// This method creates a backup of all or selected configurations.
    /// The backup format and destination are implementation-specific.
    ///
    /// # Arguments
    ///
    /// * `tenant_ids` - Optional list of tenant IDs to backup. If None, all configurations are backed up.
    /// * `backup_location` - Implementation-specific backup location identifier
    ///
    /// # Returns
    ///
    /// A backup identifier that can be used for restoration.
    async fn backup_configurations(
        &self,
        tenant_ids: Option<&[String]>,
        backup_location: &str,
    ) -> Result<String, Self::Error>;

    /// Restore configurations from a backup.
    ///
    /// This method restores configurations from a previously created backup.
    ///
    /// # Arguments
    ///
    /// * `backup_id` - The backup identifier returned from `backup_configurations`
    /// * `backup_location` - Implementation-specific backup location identifier
    /// * `overwrite_existing` - Whether to overwrite existing configurations
    ///
    /// # Returns
    ///
    /// The number of configurations restored.
    async fn restore_configurations(
        &self,
        backup_id: &str,
        backup_location: &str,
        overwrite_existing: bool,
    ) -> Result<usize, Self::Error>;

    /// Perform bulk operations on configurations.
    ///
    /// This method allows for efficient bulk operations on multiple configurations.
    ///
    /// # Arguments
    ///
    /// * `operations` - List of bulk operations to perform
    ///
    /// # Returns
    ///
    /// Results of each operation in the same order as the input.
    async fn bulk_operations(
        &self,
        operations: &[BulkConfigurationOperation],
    ) -> Result<Vec<BulkOperationResult>, Self::Error>;
}

/// Bulk operation types for configuration management.
#[derive(Debug, Clone)]
pub enum BulkConfigurationOperation {
    /// Create a new configuration
    Create(TenantConfiguration),
    /// Update an existing configuration
    Update(TenantConfiguration),
    /// Delete a configuration
    Delete {
        tenant_id: String,
        expected_version: Option<u64>,
    },
    /// Validate a configuration without storing it
    Validate(TenantConfiguration),
}

/// Result of a bulk operation.
#[derive(Debug, Clone)]
pub enum BulkOperationResult {
    /// Operation completed successfully
    Success {
        tenant_id: String,
        operation: String,
        configuration: Option<TenantConfiguration>,
    },
    /// Operation failed
    Error {
        tenant_id: String,
        operation: String,
        error: String,
    },
}

/// Helper trait for configuration provider validation.
pub trait ConfigurationValidator {
    /// Validate that a tenant ID is valid for this provider.
    fn validate_tenant_id(&self, tenant_id: &str) -> Result<(), ConfigurationError> {
        if tenant_id.is_empty() {
            return Err(ConfigurationError::ValidationError {
                message: "Tenant ID cannot be empty".to_string(),
            });
        }

        if tenant_id.len() > 255 {
            return Err(ConfigurationError::ValidationError {
                message: "Tenant ID cannot exceed 255 characters".to_string(),
            });
        }

        // Basic character validation - only alphanumeric, hyphens, and underscores
        if !tenant_id
            .chars()
            .all(|c| c.is_alphanumeric() || c == '-' || c == '_')
        {
            return Err(ConfigurationError::ValidationError {
                message:
                    "Tenant ID can only contain alphanumeric characters, hyphens, and underscores"
                        .to_string(),
            });
        }

        Ok(())
    }

    /// Validate that a configuration version is reasonable.
    fn validate_version(&self, version: u64) -> Result<(), ConfigurationError> {
        if version == 0 {
            return Err(ConfigurationError::ValidationError {
                message: "Configuration version must be greater than 0".to_string(),
            });
        }
        Ok(())
    }

    /// Validate that configuration size is within limits.
    fn validate_configuration_size(
        &self,
        configuration: &TenantConfiguration,
    ) -> Result<(), ConfigurationError> {
        let serialized_size = serde_json::to_string(configuration)
            .map_err(|e| ConfigurationError::ValidationError {
                message: format!(
                    "Failed to serialize configuration for size validation: {}",
                    e
                ),
            })?
            .len();

        const MAX_CONFIG_SIZE: usize = 1024 * 1024; // 1MB
        if serialized_size > MAX_CONFIG_SIZE {
            return Err(ConfigurationError::ValidationError {
                message: format!(
                    "Configuration size ({} bytes) exceeds maximum allowed size ({} bytes)",
                    serialized_size, MAX_CONFIG_SIZE
                ),
            });
        }

        Ok(())
    }
}

/// Extension trait for configuration providers with caching capabilities.
#[async_trait]
pub trait CachedConfigurationProvider: TenantConfigurationProvider {
    /// Clear the configuration cache for a specific tenant.
    ///
    /// # Arguments
    ///
    /// * `tenant_id` - The tenant ID to clear from cache
    async fn clear_cache(&self, tenant_id: &str) -> Result<(), Self::Error>;

    /// Clear the entire configuration cache.
    async fn clear_all_cache(&self) -> Result<(), Self::Error>;

    /// Get cache statistics.
    ///
    /// # Returns
    ///
    /// Statistics about cache usage and performance.
    async fn get_cache_stats(&self) -> Result<CacheStats, Self::Error>;

    /// Warm up the cache with frequently accessed configurations.
    ///
    /// # Arguments
    ///
    /// * `tenant_ids` - Optional list of tenant IDs to preload. If None, all configurations are loaded.
    async fn warm_cache(&self, tenant_ids: Option<&[String]>) -> Result<usize, Self::Error>;
}

/// Cache statistics for configuration providers.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheStats {
    /// Total number of cache hits
    pub cache_hits: u64,
    /// Total number of cache misses
    pub cache_misses: u64,
    /// Cache hit ratio (0.0 to 1.0)
    pub hit_ratio: f64,
    /// Number of items currently in cache
    pub cached_items: usize,
    /// Total memory used by cache in bytes
    pub memory_usage: usize,
    /// Number of cache evictions
    pub evictions: u64,
    /// Average cache lookup time in microseconds
    pub average_lookup_time_us: u64,
}

impl CacheStats {
    /// Calculate the cache hit ratio.
    pub fn calculate_hit_ratio(&mut self) {
        let total_requests = self.cache_hits + self.cache_misses;
        self.hit_ratio = if total_requests > 0 {
            self.cache_hits as f64 / total_requests as f64
        } else {
            0.0
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_configuration_query_default() {
        let query = ConfigurationQuery::default();
        assert!(query.tenant_ids.is_none());
        assert!(query.display_name_filter.is_none());
        assert!(query.offset.is_none());
        assert!(query.limit.is_none());
        assert!(matches!(query.sort_order, SortOrder::TenantIdAsc));
    }

    #[test]
    fn test_sort_order_default() {
        let sort_order = SortOrder::default();
        assert!(matches!(sort_order, SortOrder::TenantIdAsc));
    }

    #[test]
    fn test_validation_context() {
        let context = ValidationContext {
            is_create: true,
            previous_configuration: None,
            validation_params: HashMap::new(),
        };

        assert!(context.is_create);
        assert!(context.previous_configuration.is_none());
        assert!(context.validation_params.is_empty());
    }

    #[test]
    fn test_cache_stats_hit_ratio() {
        let mut stats = CacheStats {
            cache_hits: 80,
            cache_misses: 20,
            hit_ratio: 0.0,
            cached_items: 100,
            memory_usage: 1024,
            evictions: 5,
            average_lookup_time_us: 150,
        };

        stats.calculate_hit_ratio();
        assert_eq!(stats.hit_ratio, 0.8);

        // Test edge case with no requests
        stats.cache_hits = 0;
        stats.cache_misses = 0;
        stats.calculate_hit_ratio();
        assert_eq!(stats.hit_ratio, 0.0);
    }

    #[test]
    fn test_bulk_operation_types() {
        use crate::multi_tenant::configuration::TenantConfiguration;

        let config = TenantConfiguration::builder("test-tenant".to_string())
            .build()
            .expect("Should build configuration");

        let operations = vec![
            BulkConfigurationOperation::Create(config.clone()),
            BulkConfigurationOperation::Update(config.clone()),
            BulkConfigurationOperation::Delete {
                tenant_id: "test-tenant".to_string(),
                expected_version: Some(1),
            },
            BulkConfigurationOperation::Validate(config),
        ];

        assert_eq!(operations.len(), 4);
    }
}
