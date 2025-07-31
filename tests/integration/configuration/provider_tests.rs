//! Provider-specific tests for tenant configuration management.
//!
//! This module tests the behavior of different configuration provider
//! implementations, focusing on provider-specific features, error handling,
//! and edge cases.

// Removed unused import
use scim_server::multi_tenant::config_inmemory::ProviderSettings;
use scim_server::multi_tenant::{
    BulkConfigurationOperation, BulkOperationResult, CachedConfigurationProvider,
    ConfigurationError, ConfigurationQuery, InMemoryConfigurationProvider, TenantConfiguration,
    TenantConfigurationProvider, ValidationContext,
};
use std::collections::HashMap;
use std::sync::Arc;
use tempfile::tempdir;

/// Test in-memory provider basic operations.
#[tokio::test]
async fn test_in_memory_provider_basic_operations() {
    let provider = Arc::new(InMemoryConfigurationProvider::new());

    // Test empty state
    assert_eq!(provider.count_configurations().await.unwrap(), 0);
    assert!(
        provider
            .get_configuration("nonexistent")
            .await
            .unwrap()
            .is_none()
    );

    // Create configuration
    let config = TenantConfiguration::builder("test-tenant".to_string())
        .with_display_name("Test Tenant".to_string())
        .build()
        .unwrap();

    let created = provider
        .create_configuration(config.clone())
        .await
        .expect("Should create configuration");

    assert_eq!(created.tenant_id, "test-tenant");
    assert_eq!(created.display_name, "Test Tenant");
    assert_eq!(created.version, 1);

    // Retrieve configuration
    let retrieved = provider
        .get_configuration("test-tenant")
        .await
        .unwrap()
        .unwrap();

    assert_eq!(retrieved, created);

    // Update configuration
    let mut updated_config = retrieved.clone();
    updated_config.display_name = "Updated Test Tenant".to_string();

    let updated = provider
        .update_configuration(updated_config)
        .await
        .expect("Should update configuration");

    assert_eq!(updated.display_name, "Updated Test Tenant");
    assert_eq!(updated.version, 2);

    // Delete configuration
    provider
        .delete_configuration("test-tenant", Some(updated.version))
        .await
        .expect("Should delete configuration");

    assert!(
        provider
            .get_configuration("test-tenant")
            .await
            .unwrap()
            .is_none()
    );
}

/// Test provider error handling and edge cases.
#[tokio::test]
async fn test_provider_error_handling() {
    let provider = Arc::new(InMemoryConfigurationProvider::new());

    // Test invalid tenant IDs
    let long_id = "x".repeat(300);
    let invalid_ids = vec!["", "tenant@invalid", "tenant with spaces", &long_id];

    for invalid_id in invalid_ids {
        let result = provider.get_configuration(invalid_id).await;
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            ConfigurationError::ValidationError { .. }
        ));
    }

    // Test duplicate creation
    let config = TenantConfiguration::builder("duplicate-test".to_string())
        .build()
        .unwrap();

    provider
        .create_configuration(config.clone())
        .await
        .expect("Should create first configuration");

    let duplicate_result = provider.create_configuration(config).await;
    assert!(duplicate_result.is_err());
    assert!(matches!(
        duplicate_result.unwrap_err(),
        ConfigurationError::Conflict { .. }
    ));

    // Test update non-existent configuration
    let nonexistent_config = TenantConfiguration::builder("nonexistent".to_string())
        .build()
        .unwrap();

    let update_result = provider.update_configuration(nonexistent_config).await;
    assert!(update_result.is_err());
    assert!(matches!(
        update_result.unwrap_err(),
        ConfigurationError::NotFound { .. }
    ));

    // Test version mismatch
    let existing = provider
        .get_configuration("duplicate-test")
        .await
        .unwrap()
        .unwrap();

    let mut stale_config = existing.clone();
    stale_config.version = 999; // Wrong version

    let version_result = provider.update_configuration(stale_config).await;
    assert!(version_result.is_err());
    assert!(matches!(
        version_result.unwrap_err(),
        ConfigurationError::VersionMismatch { .. }
    ));

    // Test delete non-existent configuration
    let delete_result = provider.delete_configuration("nonexistent", None).await;
    assert!(delete_result.is_err());
    assert!(matches!(
        delete_result.unwrap_err(),
        ConfigurationError::NotFound { .. }
    ));
}

/// Test provider validation methods.
#[tokio::test]
async fn test_provider_validation() {
    let provider = Arc::new(InMemoryConfigurationProvider::new());

    // Test valid configuration
    let valid_config = TenantConfiguration::builder("valid-tenant".to_string())
        .build()
        .unwrap();

    let context = ValidationContext {
        is_create: true,
        previous_configuration: None,
        validation_params: HashMap::new(),
    };

    provider
        .validate_configuration(&valid_config, &context)
        .await
        .expect("Should validate valid configuration");

    // Test invalid configuration (zero resource limits)
    let mut invalid_config = valid_config.clone();
    invalid_config.operational.resource_limits.max_users = Some(0);

    let validation_result = provider
        .validate_configuration(&invalid_config, &context)
        .await;
    assert!(validation_result.is_err());

    // Test oversized configuration
    let mut oversized_config = valid_config.clone();
    oversized_config.schema.custom_attributes.insert(
        "huge_field".to_string(),
        serde_json::Value::String("x".repeat(2 * 1024 * 1024)), // 2MB string
    );

    let size_result = provider
        .validate_configuration(&oversized_config, &context)
        .await;
    assert!(size_result.is_err());
}

/// Test provider bulk operations.
#[tokio::test]
async fn test_provider_bulk_operations() {
    let provider = Arc::new(InMemoryConfigurationProvider::new());

    let config1 = TenantConfiguration::builder("bulk-1".to_string())
        .build()
        .unwrap();
    let config2 = TenantConfiguration::builder("bulk-2".to_string())
        .build()
        .unwrap();
    let config3 = TenantConfiguration::builder("bulk-3".to_string())
        .build()
        .unwrap();

    // Test mixed bulk operations
    let operations = vec![
        BulkConfigurationOperation::Create(config1.clone()),
        BulkConfigurationOperation::Create(config2.clone()),
        BulkConfigurationOperation::Validate(config3.clone()),
        BulkConfigurationOperation::Create(config1.clone()), // Duplicate - should fail
        BulkConfigurationOperation::Delete {
            tenant_id: "nonexistent".to_string(),
            expected_version: None,
        },
    ];

    let results = provider
        .bulk_operations(&operations)
        .await
        .expect("Should complete bulk operations");

    assert_eq!(results.len(), 5);

    // Check individual results
    assert!(matches!(results[0], BulkOperationResult::Success { .. }));
    assert!(matches!(results[1], BulkOperationResult::Success { .. }));
    assert!(matches!(results[2], BulkOperationResult::Success { .. }));
    assert!(matches!(results[3], BulkOperationResult::Error { .. })); // Duplicate
    assert!(matches!(results[4], BulkOperationResult::Error { .. })); // Nonexistent

    // Verify successful operations actually worked
    assert_eq!(provider.count_configurations().await.unwrap(), 2);
    assert!(
        provider
            .get_configuration("bulk-1")
            .await
            .unwrap()
            .is_some()
    );
    assert!(
        provider
            .get_configuration("bulk-2")
            .await
            .unwrap()
            .is_some()
    );

    // Test bulk updates
    let created1 = provider.get_configuration("bulk-1").await.unwrap().unwrap();
    let created2 = provider.get_configuration("bulk-2").await.unwrap().unwrap();

    let mut updated1 = created1.clone();
    updated1.display_name = "Updated Bulk 1".to_string();
    let mut updated2 = created2.clone();
    updated2.display_name = "Updated Bulk 2".to_string();

    let update_operations = vec![
        BulkConfigurationOperation::Update(updated1),
        BulkConfigurationOperation::Update(updated2),
    ];

    let update_results = provider
        .bulk_operations(&update_operations)
        .await
        .expect("Should complete bulk updates");

    assert_eq!(update_results.len(), 2);
    assert!(matches!(
        update_results[0],
        BulkOperationResult::Success { .. }
    ));
    assert!(matches!(
        update_results[1],
        BulkOperationResult::Success { .. }
    ));

    // Verify updates
    let final1 = provider.get_configuration("bulk-1").await.unwrap().unwrap();
    let final2 = provider.get_configuration("bulk-2").await.unwrap().unwrap();

    assert_eq!(final1.display_name, "Updated Bulk 1");
    assert_eq!(final1.version, 2);
    assert_eq!(final2.display_name, "Updated Bulk 2");
    assert_eq!(final2.version, 2);
}

/// Test provider listing and querying.
#[tokio::test]
async fn test_provider_listing_and_querying() {
    let provider = Arc::new(InMemoryConfigurationProvider::new());

    // Create test configurations with different characteristics
    let configs = vec![
        ("alpha-tenant", "Alpha Company", chrono::Utc::now()),
        ("beta-tenant", "Beta Corporation", chrono::Utc::now()),
        ("gamma-tenant", "Gamma Industries", chrono::Utc::now()),
        ("delta-tenant", "Delta Solutions", chrono::Utc::now()),
    ];

    for (tenant_id, display_name, _created) in &configs {
        let config = TenantConfiguration::builder(tenant_id.to_string())
            .with_display_name(display_name.to_string())
            .build()
            .unwrap();

        provider
            .create_configuration(config)
            .await
            .expect("Should create test configuration");

        // Add small delay to ensure different timestamps
        tokio::time::sleep(tokio::time::Duration::from_millis(1)).await;
    }

    // Test basic listing
    let all_configs = provider.get_all_configurations().await.unwrap();
    assert_eq!(all_configs.len(), 4);

    // Test listing with default query
    let default_query = ConfigurationQuery::default();
    let default_result = provider.list_configurations(&default_query).await.unwrap();
    assert_eq!(default_result.configurations.len(), 4);
    assert_eq!(default_result.total_count, 4);
    assert!(!default_result.has_more);

    // Test pagination
    let paginated_query = ConfigurationQuery {
        offset: Some(1),
        limit: Some(2),
        ..Default::default()
    };
    let paginated_result = provider
        .list_configurations(&paginated_query)
        .await
        .unwrap();
    assert_eq!(paginated_result.configurations.len(), 2);
    assert_eq!(paginated_result.total_count, 4);
    assert!(paginated_result.has_more);
    assert_eq!(paginated_result.next_offset, Some(3));

    // Test filtering by tenant IDs
    let filtered_query = ConfigurationQuery {
        tenant_ids: Some(vec!["alpha-tenant".to_string(), "gamma-tenant".to_string()]),
        ..Default::default()
    };
    let filtered_result = provider.list_configurations(&filtered_query).await.unwrap();
    assert_eq!(filtered_result.configurations.len(), 2);
    assert_eq!(filtered_result.total_count, 2);

    // Test filtering by display name
    let name_filtered_query = ConfigurationQuery {
        display_name_filter: Some("Company".to_string()),
        ..Default::default()
    };
    let name_filtered_result = provider
        .list_configurations(&name_filtered_query)
        .await
        .unwrap();
    assert_eq!(name_filtered_result.configurations.len(), 1);
    assert_eq!(
        name_filtered_result.configurations[0].tenant_id,
        "alpha-tenant"
    );

    // Test sorting
    use scim_server::multi_tenant::config_provider::SortOrder;

    let sorted_query = ConfigurationQuery {
        sort_order: SortOrder::DisplayNameDesc,
        ..Default::default()
    };
    let sorted_result = provider.list_configurations(&sorted_query).await.unwrap();
    assert_eq!(sorted_result.configurations.len(), 4);
    assert_eq!(
        sorted_result.configurations[0].display_name,
        "Gamma Industries"
    );
    assert_eq!(
        sorted_result.configurations[3].display_name,
        "Alpha Company"
    );
}

/// Test provider statistics.
#[tokio::test]
async fn test_provider_statistics() {
    let provider = Arc::new(InMemoryConfigurationProvider::new());

    // Test empty statistics
    let empty_stats = provider.get_configuration_stats().await.unwrap();
    assert_eq!(empty_stats.total_configurations, 0);
    assert_eq!(empty_stats.recent_configurations, 0);
    assert_eq!(empty_stats.recently_modified, 0);
    assert_eq!(empty_stats.average_size, 0);
    assert_eq!(empty_stats.total_storage_used, 0);
    assert!(empty_stats.newest_configuration.is_none());
    assert!(empty_stats.oldest_configuration.is_none());
    assert!(empty_stats.version_distribution.is_empty());

    // Create configurations
    for i in 1..=5 {
        let config = TenantConfiguration::builder(format!("stats-tenant-{}", i))
            .with_display_name(format!("Stats Tenant {}", i))
            .build()
            .unwrap();

        provider
            .create_configuration(config)
            .await
            .expect("Should create stats test configuration");

        tokio::time::sleep(tokio::time::Duration::from_millis(1)).await;
    }

    // Update some configurations to create version diversity
    for i in 1..=3 {
        let existing = provider
            .get_configuration(&format!("stats-tenant-{}", i))
            .await
            .unwrap()
            .unwrap();

        let mut updated = existing.clone();
        updated.display_name = format!("Updated Stats Tenant {}", i);

        provider
            .update_configuration(updated)
            .await
            .expect("Should update configuration");
    }

    // Test populated statistics
    let stats = provider.get_configuration_stats().await.unwrap();
    assert_eq!(stats.total_configurations, 5);
    assert_eq!(stats.recent_configurations, 5); // All created recently
    assert_eq!(stats.recently_modified, 5); // All were modified recently (including updates)
    assert!(stats.average_size > 0);
    assert!(stats.total_storage_used > 0);
    assert!(stats.newest_configuration.is_some());
    assert!(stats.oldest_configuration.is_some());

    // Check version distribution
    assert!(stats.version_distribution.contains_key(&1)); // 2 configs at version 1
    assert!(stats.version_distribution.contains_key(&2)); // 3 configs at version 2
    assert_eq!(stats.version_distribution.get(&1), Some(&2));
    assert_eq!(stats.version_distribution.get(&2), Some(&3));
}

/// Test provider persistence functionality.
#[tokio::test]
async fn test_provider_persistence() {
    let temp_dir = tempdir().expect("Should create temp directory");
    let file_path = temp_dir.path().join("test_persistence.json");

    let provider = InMemoryConfigurationProvider::with_persistence(&file_path);

    // Create configurations
    for i in 1..=3 {
        let config = TenantConfiguration::builder(format!("persist-tenant-{}", i))
            .with_display_name(format!("Persistent Tenant {}", i))
            .build()
            .unwrap();

        provider
            .create_configuration(config)
            .await
            .expect("Should create persistent configuration");
    }

    assert_eq!(provider.size().await, 3);

    // Save to file
    let saved_count = provider.save_to_file().await.expect("Should save to file");
    assert_eq!(saved_count, 3);
    assert!(file_path.exists());

    // Clear memory and verify empty
    let cleared_count = provider.clear().await;
    assert_eq!(cleared_count, 3);
    assert_eq!(provider.size().await, 0);

    // Load from file
    let loaded_count = provider
        .load_from_file()
        .await
        .expect("Should load from file");
    assert_eq!(loaded_count, 3);
    assert_eq!(provider.size().await, 3);

    // Verify configurations are correctly loaded
    for i in 1..=3 {
        let config = provider
            .get_configuration(&format!("persist-tenant-{}", i))
            .await
            .unwrap()
            .unwrap();
        assert_eq!(config.display_name, format!("Persistent Tenant {}", i));
    }

    // Test loading non-existent file
    let bad_path = temp_dir.path().join("nonexistent.json");
    let bad_provider = InMemoryConfigurationProvider::with_persistence(&bad_path);
    let load_result = bad_provider.load_from_file().await;
    assert!(load_result.is_ok());
    assert_eq!(load_result.unwrap(), 0); // No configurations loaded
}

/// Test provider backup and restore functionality.
#[tokio::test]
async fn test_provider_backup_restore() {
    let provider = Arc::new(InMemoryConfigurationProvider::new());
    let temp_dir = tempdir().expect("Should create temp directory");
    let backup_location = temp_dir.path().to_string_lossy().to_string();

    // Create test configurations
    for i in 1..=5 {
        let config = TenantConfiguration::builder(format!("backup-tenant-{}", i))
            .with_display_name(format!("Backup Tenant {}", i))
            .build()
            .unwrap();

        provider
            .create_configuration(config)
            .await
            .expect("Should create backup test configuration");
    }

    // Test full backup
    let backup_id = provider
        .backup_configurations(None, &backup_location)
        .await
        .expect("Should backup all configurations");
    assert!(backup_id.starts_with("backup-"));
    println!("Full backup ID: {}", backup_id);

    // Add delay to ensure different timestamps
    tokio::time::sleep(tokio::time::Duration::from_millis(1001)).await;

    // Test selective backup
    let selective_tenants = vec!["backup-tenant-1".to_string(), "backup-tenant-3".to_string()];
    let selective_backup_id = provider
        .backup_configurations(Some(&selective_tenants), &backup_location)
        .await
        .expect("Should backup selected configurations");
    println!("Selective backup ID: {}", selective_backup_id);

    // Clear provider and test full restore
    provider.clear().await;
    assert_eq!(provider.size().await, 0);

    // Add small delay to ensure file operations complete
    tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

    println!("Attempting to restore from backup_id: {}", backup_id);
    println!("Backup location: {}", backup_location);
    let restored_count = provider
        .restore_configurations(&backup_id, &backup_location, true)
        .await
        .expect("Should restore all configurations");
    println!("Restored {} configurations", restored_count);
    println!("Provider size after restore: {}", provider.size().await);
    assert_eq!(restored_count, 5);
    assert_eq!(provider.size().await, 5);

    // Test selective restore over existing (no overwrite)
    let partial_restore_count = provider
        .restore_configurations(&selective_backup_id, &backup_location, false)
        .await
        .expect("Should attempt selective restore");
    assert_eq!(partial_restore_count, 0); // No new configurations added
    assert_eq!(provider.size().await, 5); // Still 5 configurations

    // Test selective restore with overwrite
    provider.clear().await;
    let overwrite_restore_count = provider
        .restore_configurations(&selective_backup_id, &backup_location, true)
        .await
        .expect("Should restore with overwrite");
    assert_eq!(overwrite_restore_count, 2); // Only the 2 selected tenants
    assert_eq!(provider.size().await, 2);

    // Verify correct configurations were restored
    assert!(
        provider
            .get_configuration("backup-tenant-1")
            .await
            .unwrap()
            .is_some()
    );
    assert!(
        provider
            .get_configuration("backup-tenant-3")
            .await
            .unwrap()
            .is_some()
    );
    assert!(
        provider
            .get_configuration("backup-tenant-2")
            .await
            .unwrap()
            .is_none()
    );
}

/// Test provider cache functionality.
#[tokio::test]
async fn test_provider_cache_functionality() {
    let provider = Arc::new(InMemoryConfigurationProvider::new());

    // Create test configurations
    for i in 1..=3 {
        let config = TenantConfiguration::builder(format!("cache-tenant-{}", i))
            .build()
            .unwrap();

        provider
            .create_configuration(config)
            .await
            .expect("Should create cache test configuration");
    }

    // Test cache stats
    let cache_stats = provider.get_cache_stats().await.unwrap();
    assert_eq!(cache_stats.cached_items, 3);
    assert!(cache_stats.memory_usage > 0);

    // Test cache warming (all configurations)
    let warmed_count = provider.warm_cache(None).await.unwrap();
    assert_eq!(warmed_count, 3);

    // Test selective cache warming
    let selective_tenants = vec!["cache-tenant-1".to_string(), "cache-tenant-2".to_string()];
    let selective_warmed = provider.warm_cache(Some(&selective_tenants)).await.unwrap();
    assert_eq!(selective_warmed, 2);

    // Test individual cache clearing
    provider
        .clear_cache("cache-tenant-1")
        .await
        .expect("Should clear individual cache entry");

    let after_clear_stats = provider.get_cache_stats().await.unwrap();
    assert_eq!(after_clear_stats.cached_items, 2);

    // Test full cache clearing
    provider
        .clear_all_cache()
        .await
        .expect("Should clear all cache");

    let empty_cache_stats = provider.get_cache_stats().await.unwrap();
    assert_eq!(empty_cache_stats.cached_items, 0);
    assert!(empty_cache_stats.evictions > 0);
}

/// Test provider capacity limits.
#[tokio::test]
async fn test_provider_capacity_limits() {
    let settings = ProviderSettings {
        max_configurations: Some(3),
        ..Default::default()
    };
    let provider = InMemoryConfigurationProvider::with_settings(settings);

    // Should allow up to the limit
    for i in 1..=3 {
        let config = TenantConfiguration::builder(format!("capacity-tenant-{}", i))
            .build()
            .unwrap();

        provider
            .create_configuration(config)
            .await
            .expect("Should create configuration within limit");
    }

    assert_eq!(provider.size().await, 3);

    // Should reject beyond the limit
    let over_limit_config = TenantConfiguration::builder("over-limit".to_string())
        .build()
        .unwrap();

    let result = provider.create_configuration(over_limit_config).await;
    assert!(result.is_err());
    let error = result.unwrap_err();
    assert!(matches!(error, ConfigurationError::ValidationError { .. }));
    assert!(error.to_string().contains("Maximum configuration limit"));

    // Should still be at the limit
    assert_eq!(provider.size().await, 3);

    // Should allow creation after deletion
    provider
        .delete_configuration("capacity-tenant-1", None)
        .await
        .expect("Should delete configuration");

    assert_eq!(provider.size().await, 2);

    let new_config = TenantConfiguration::builder("new-after-delete".to_string())
        .build()
        .unwrap();

    provider
        .create_configuration(new_config)
        .await
        .expect("Should create configuration after deletion");

    assert_eq!(provider.size().await, 3);
}
