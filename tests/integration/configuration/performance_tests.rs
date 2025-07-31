//! Performance-focused tests for tenant configuration management.
//!
//! This module tests the performance characteristics of configuration
//! providers under various load conditions, including concurrent access,
//! large configuration sets, and bulk operations.

// Removed unused import
use scim_server::multi_tenant::{
    BulkConfigurationOperation, CachedConfigurationProvider, ConfigurationQuery,
    InMemoryConfigurationProvider, TenantConfiguration, TenantConfigurationProvider,
};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Semaphore;

/// Test concurrent configuration access performance.
#[tokio::test]
async fn test_concurrent_configuration_access() {
    let provider = Arc::new(InMemoryConfigurationProvider::new());
    let num_configs = 100;
    let concurrent_readers = 50;

    // Create initial configurations
    for i in 0..num_configs {
        let config = TenantConfiguration::builder(format!("perf-tenant-{}", i))
            .with_display_name(format!("Performance Test Tenant {}", i))
            .build()
            .expect("Should build performance test configuration");

        provider
            .create_configuration(config)
            .await
            .expect("Should create performance test configuration");
    }

    // Measure concurrent read performance
    let start_time = Instant::now();
    let semaphore = Arc::new(Semaphore::new(concurrent_readers));
    let mut handles = Vec::new();

    for i in 0..concurrent_readers {
        let provider_clone = provider.clone();
        let semaphore_clone = semaphore.clone();
        let tenant_id = format!("perf-tenant-{}", i % num_configs);

        let handle = tokio::spawn(async move {
            let _permit = semaphore_clone.acquire().await.unwrap();
            let start = Instant::now();

            // Perform multiple reads
            for _ in 0..10 {
                let _config = provider_clone
                    .get_configuration(&tenant_id)
                    .await
                    .expect("Should retrieve configuration");
            }

            start.elapsed()
        });

        handles.push(handle);
    }

    // Wait for all operations to complete
    let mut total_duration = Duration::ZERO;
    for handle in handles {
        let duration = handle.await.expect("Task should complete");
        total_duration += duration;
    }

    let total_elapsed = start_time.elapsed();
    let operations_per_second = (concurrent_readers * 10) as f64 / total_elapsed.as_secs_f64();

    println!(
        "Concurrent read performance: {:.2} ops/sec, total time: {:?}",
        operations_per_second, total_elapsed
    );

    // Assert reasonable performance (should handle at least 1000 ops/sec)
    assert!(operations_per_second > 1000.0);
}

/// Test large-scale configuration listing performance.
#[tokio::test]
async fn test_large_scale_listing_performance() {
    let provider = Arc::new(InMemoryConfigurationProvider::new());
    let num_configs = 10000;

    // Create large number of configurations
    let creation_start = Instant::now();
    for i in 0..num_configs {
        let config = TenantConfiguration::builder(format!("large-scale-{:06}", i))
            .with_display_name(format!("Large Scale Test {}", i))
            .build()
            .expect("Should build large scale configuration");

        provider
            .create_configuration(config)
            .await
            .expect("Should create large scale configuration");

        // Progress indicator
        if i % 1000 == 0 {
            println!("Created {} configurations...", i);
        }
    }

    let creation_duration = creation_start.elapsed();
    println!(
        "Created {} configurations in {:?} ({:.2} configs/sec)",
        num_configs,
        creation_duration,
        num_configs as f64 / creation_duration.as_secs_f64()
    );

    // Test full listing performance
    let list_start = Instant::now();
    let all_configs = provider
        .get_all_configurations()
        .await
        .expect("Should list all configurations");
    let list_duration = list_start.elapsed();

    assert_eq!(all_configs.len(), num_configs);
    println!(
        "Listed {} configurations in {:?} ({:.2} configs/sec)",
        all_configs.len(),
        list_duration,
        all_configs.len() as f64 / list_duration.as_secs_f64()
    );

    // Test paginated listing performance
    let page_size = 1000;
    let pagination_start = Instant::now();
    let mut total_retrieved = 0;
    let mut offset = 0;

    loop {
        let query = ConfigurationQuery {
            offset: Some(offset),
            limit: Some(page_size),
            ..Default::default()
        };

        let result = provider
            .list_configurations(&query)
            .await
            .expect("Should list configurations with pagination");

        total_retrieved += result.configurations.len();
        offset += page_size;

        if !result.has_more {
            break;
        }
    }

    let pagination_duration = pagination_start.elapsed();
    assert_eq!(total_retrieved, num_configs);
    println!(
        "Paginated listing of {} configurations in {:?} ({:.2} configs/sec)",
        total_retrieved,
        pagination_duration,
        total_retrieved as f64 / pagination_duration.as_secs_f64()
    );

    // Test filtered listing performance
    let filter_start = Instant::now();
    let query = ConfigurationQuery {
        display_name_filter: Some("Large Scale Test".to_string()),
        ..Default::default()
    };

    let filtered_result = provider
        .list_configurations(&query)
        .await
        .expect("Should filter configurations");

    let filter_duration = filter_start.elapsed();
    assert_eq!(filtered_result.configurations.len(), num_configs);
    println!(
        "Filtered {} configurations in {:?} ({:.2} configs/sec)",
        filtered_result.configurations.len(),
        filter_duration,
        filtered_result.configurations.len() as f64 / filter_duration.as_secs_f64()
    );

    // Assert reasonable performance thresholds
    assert!(creation_duration.as_secs() < 30); // Should create 10k configs in under 30 seconds
    assert!(list_duration.as_millis() < 1000); // Should list 10k configs in under 1 second
    assert!(pagination_duration.as_secs() < 5); // Should paginate through 10k configs in under 5 seconds
    assert!(filter_duration.as_millis() < 2000); // Should filter 10k configs in under 2 seconds
}

/// Test bulk operations performance.
#[tokio::test]
async fn test_bulk_operations_performance() {
    let provider = Arc::new(InMemoryConfigurationProvider::new());
    let batch_size = 1000;

    // Prepare bulk create operations
    let create_operations: Vec<BulkConfigurationOperation> = (0..batch_size)
        .map(|i| {
            let config = TenantConfiguration::builder(format!("bulk-create-{:04}", i))
                .with_display_name(format!("Bulk Create Test {}", i))
                .build()
                .expect("Should build bulk create configuration");
            BulkConfigurationOperation::Create(config)
        })
        .collect();

    // Measure bulk create performance
    let create_start = Instant::now();
    let create_results = provider
        .bulk_operations(&create_operations)
        .await
        .expect("Should perform bulk create operations");
    let create_duration = create_start.elapsed();

    // Verify all creates succeeded
    let successful_creates = create_results
        .iter()
        .filter(|r| {
            matches!(
                r,
                scim_server::multi_tenant::BulkOperationResult::Success { .. }
            )
        })
        .count();
    assert_eq!(successful_creates, batch_size);

    println!(
        "Bulk created {} configurations in {:?} ({:.2} configs/sec)",
        successful_creates,
        create_duration,
        successful_creates as f64 / create_duration.as_secs_f64()
    );

    // Prepare bulk update operations
    let configs = provider
        .get_all_configurations()
        .await
        .expect("Should get all configurations for update");

    let update_operations: Vec<BulkConfigurationOperation> = configs
        .into_iter()
        .map(|mut config| {
            config.display_name = format!("Updated {}", config.display_name);
            BulkConfigurationOperation::Update(config)
        })
        .collect();

    // Measure bulk update performance
    let update_start = Instant::now();
    let update_results = provider
        .bulk_operations(&update_operations)
        .await
        .expect("Should perform bulk update operations");
    let update_duration = update_start.elapsed();

    // Verify all updates succeeded
    let successful_updates = update_results
        .iter()
        .filter(|r| {
            matches!(
                r,
                scim_server::multi_tenant::BulkOperationResult::Success { .. }
            )
        })
        .count();
    assert_eq!(successful_updates, batch_size);

    println!(
        "Bulk updated {} configurations in {:?} ({:.2} configs/sec)",
        successful_updates,
        update_duration,
        successful_updates as f64 / update_duration.as_secs_f64()
    );

    // Prepare bulk delete operations
    let delete_operations: Vec<BulkConfigurationOperation> = (0..batch_size)
        .map(|i| BulkConfigurationOperation::Delete {
            tenant_id: format!("bulk-create-{:04}", i),
            expected_version: None,
        })
        .collect();

    // Measure bulk delete performance
    let delete_start = Instant::now();
    let delete_results = provider
        .bulk_operations(&delete_operations)
        .await
        .expect("Should perform bulk delete operations");
    let delete_duration = delete_start.elapsed();

    // Verify all deletes succeeded
    let successful_deletes = delete_results
        .iter()
        .filter(|r| {
            matches!(
                r,
                scim_server::multi_tenant::BulkOperationResult::Success { .. }
            )
        })
        .count();
    assert_eq!(successful_deletes, batch_size);

    println!(
        "Bulk deleted {} configurations in {:?} ({:.2} configs/sec)",
        successful_deletes,
        delete_duration,
        successful_deletes as f64 / delete_duration.as_secs_f64()
    );

    // Verify provider is empty
    let final_count = provider
        .count_configurations()
        .await
        .expect("Should count configurations");
    assert_eq!(final_count, 0);

    // Assert reasonable performance thresholds
    assert!(create_duration.as_secs() < 5); // Should bulk create 1k configs in under 5 seconds
    assert!(update_duration.as_secs() < 5); // Should bulk update 1k configs in under 5 seconds
    assert!(delete_duration.as_secs() < 5); // Should bulk delete 1k configs in under 5 seconds
}

/// Test memory usage and efficiency.
#[tokio::test]
async fn test_memory_efficiency() {
    let provider = Arc::new(InMemoryConfigurationProvider::new());
    let num_configs = 5000;

    // Get initial memory usage baseline
    let _initial_stats = CachedConfigurationProvider::get_cache_stats(&*provider)
        .await
        .expect("Should get initial cache stats");

    // Create configurations and measure memory growth
    for i in 0..num_configs {
        let config = TenantConfiguration::builder(format!("memory-test-{:05}", i))
            .with_display_name(format!("Memory Test Configuration {}", i))
            .build()
            .expect("Should build memory test configuration");

        provider
            .create_configuration(config)
            .await
            .expect("Should create memory test configuration");

        // Check memory usage every 1000 configurations
        if i % 1000 == 999 {
            let stats = CachedConfigurationProvider::get_cache_stats(&*provider)
                .await
                .expect("Should get cache stats");

            let memory_per_config = stats.memory_usage / (i + 1);
            println!(
                "After {} configs: {} bytes total, {} bytes/config",
                i + 1,
                stats.memory_usage,
                memory_per_config
            );

            // Assert reasonable memory usage (should be less than 10KB per config)
            assert!(memory_per_config < 10 * 1024);
        }
    }

    // Get final memory statistics
    let final_stats = CachedConfigurationProvider::get_cache_stats(&*provider)
        .await
        .expect("Should get final cache stats");

    let total_memory_mb = final_stats.memory_usage as f64 / (1024.0 * 1024.0);
    let memory_per_config = final_stats.memory_usage / num_configs;

    println!(
        "Final memory usage: {:.2} MB total, {} bytes per config",
        total_memory_mb, memory_per_config
    );

    // Assert memory efficiency
    assert!(memory_per_config < 5 * 1024); // Should use less than 5KB per config
    assert!(total_memory_mb < 25.0); // Should use less than 25MB for 5k configs
    assert_eq!(final_stats.cached_items, num_configs);

    // Test memory cleanup
    let clear_start = Instant::now();
    CachedConfigurationProvider::clear_all_cache(&*provider)
        .await
        .expect("Should clear all cache");
    let clear_duration = clear_start.elapsed();

    let cleared_stats = CachedConfigurationProvider::get_cache_stats(&*provider)
        .await
        .expect("Should get cleared cache stats");

    assert_eq!(cleared_stats.cached_items, 0);
    assert_eq!(cleared_stats.memory_usage, 0);
    println!(
        "Cleared {} configurations in {:?}",
        num_configs, clear_duration
    );

    // Assert reasonable cleanup performance
    assert!(clear_duration.as_millis() < 100); // Should clear in under 100ms
}

/// Test provider statistics calculation performance.
#[tokio::test]
async fn test_statistics_performance() {
    let provider = Arc::new(InMemoryConfigurationProvider::new());
    let num_configs = 2000;

    // Create configurations with varied characteristics
    for i in 0..num_configs {
        let config = TenantConfiguration::builder(format!("stats-perf-{:04}", i))
            .with_display_name(format!("Statistics Performance Test {}", i))
            .build()
            .expect("Should build stats performance configuration");

        provider
            .create_configuration(config)
            .await
            .expect("Should create stats performance configuration");

        // Update some configurations to create version diversity
        if i % 3 == 0 {
            let existing = provider
                .get_configuration(&format!("stats-perf-{:04}", i))
                .await
                .expect("Should get configuration")
                .expect("Configuration should exist");

            let mut updated = existing.clone();
            updated.display_name = format!("Updated {}", updated.display_name);

            provider
                .update_configuration(updated)
                .await
                .expect("Should update configuration");
        }
    }

    // Measure statistics calculation performance
    let stats_start = Instant::now();
    let stats = provider
        .get_configuration_stats()
        .await
        .expect("Should get configuration statistics");
    let stats_duration = stats_start.elapsed();

    // Verify statistics accuracy
    assert_eq!(stats.total_configurations, num_configs);
    assert!(stats.average_size > 0);
    assert!(stats.total_storage_used > 0);
    assert!(stats.newest_configuration.is_some());
    assert!(stats.oldest_configuration.is_some());
    assert!(!stats.version_distribution.is_empty());

    println!(
        "Calculated statistics for {} configurations in {:?}",
        num_configs, stats_duration
    );
    println!("  Average config size: {} bytes", stats.average_size);
    println!("  Total storage used: {} bytes", stats.total_storage_used);
    println!("  Version distribution: {:?}", stats.version_distribution);

    // Assert reasonable statistics performance
    assert!(stats_duration.as_millis() < 1000); // Should calculate stats in under 1 second

    // Test repeated statistics calls (should be fast due to caching characteristics)
    let repeat_start = Instant::now();
    for _ in 0..10 {
        let _repeat_stats = provider
            .get_configuration_stats()
            .await
            .expect("Should get repeated statistics");
    }
    let repeat_duration = repeat_start.elapsed();

    println!(
        "10 repeated statistics calls took {:?} (avg: {:?})",
        repeat_duration,
        repeat_duration / 10
    );

    // Repeated calls should be reasonably fast (increased threshold for CI environments)
    assert!(repeat_duration.as_millis() < 2000);
}

/// Test configuration search and filtering performance.
#[tokio::test]
async fn test_search_filtering_performance() {
    let provider = Arc::new(InMemoryConfigurationProvider::new());
    let num_configs = 5000;

    // Create configurations with varied naming patterns
    for i in 0..num_configs {
        let category = match i % 5 {
            0 => "production",
            1 => "staging",
            2 => "development",
            3 => "testing",
            _ => "sandbox",
        };

        let config = TenantConfiguration::builder(format!("search-{}-{:04}", category, i))
            .with_display_name(format!("{} Environment {}", category.to_uppercase(), i))
            .build()
            .expect("Should build search test configuration");

        provider
            .create_configuration(config)
            .await
            .expect("Should create search test configuration");
    }

    // Test different search patterns
    let search_patterns = vec![
        ("production", num_configs / 5),
        ("PRODUCTION", num_configs / 5), // Case insensitive
        ("Environment", num_configs),    // All match
        ("xyz", 0),                      // No matches
    ];

    for (pattern, expected_count) in search_patterns {
        let search_start = Instant::now();
        let query = ConfigurationQuery {
            display_name_filter: Some(pattern.to_string()),
            ..Default::default()
        };

        let results = provider
            .list_configurations(&query)
            .await
            .expect("Should search configurations");

        let search_duration = search_start.elapsed();

        assert_eq!(results.configurations.len(), expected_count);
        println!(
            "Search for '{}' found {} configurations in {:?} ({:.2} configs/sec)",
            pattern,
            results.configurations.len(),
            search_duration,
            num_configs as f64 / search_duration.as_secs_f64()
        );

        // Assert reasonable search performance
        assert!(search_duration.as_millis() < 1000); // Should search in under 1 second
    }

    // Test tenant ID filtering
    let tenant_filter_start = Instant::now();
    let production_tenant_ids: Vec<String> = (0..num_configs)
        .step_by(5)
        .map(|i| format!("search-production-{:04}", i))
        .collect();

    let tenant_query = ConfigurationQuery {
        tenant_ids: Some(production_tenant_ids.clone()),
        ..Default::default()
    };

    let tenant_results = provider
        .list_configurations(&tenant_query)
        .await
        .expect("Should filter by tenant IDs");

    let tenant_filter_duration = tenant_filter_start.elapsed();

    assert_eq!(
        tenant_results.configurations.len(),
        production_tenant_ids.len()
    );
    println!(
        "Filtered {} tenant IDs in {:?} ({:.2} configs/sec)",
        tenant_results.configurations.len(),
        tenant_filter_duration,
        num_configs as f64 / tenant_filter_duration.as_secs_f64()
    );

    // Assert reasonable filtering performance
    assert!(tenant_filter_duration.as_millis() < 500);
}

/// Test concurrent modification performance and consistency.
#[tokio::test]
async fn test_concurrent_modification_performance() {
    let provider = Arc::new(InMemoryConfigurationProvider::new());
    let num_workers = 20;
    let operations_per_worker = 50;

    // Create initial configurations for updates
    for i in 0..num_workers {
        let config = TenantConfiguration::builder(format!("concurrent-{:02}", i))
            .with_display_name(format!("Concurrent Test {}", i))
            .build()
            .expect("Should build concurrent test configuration");

        provider
            .create_configuration(config)
            .await
            .expect("Should create concurrent test configuration");
    }

    // Launch concurrent workers
    let start_time = Instant::now();
    let mut handles = Vec::new();

    for worker_id in 0..num_workers {
        let provider_clone = provider.clone();
        let tenant_id = format!("concurrent-{:02}", worker_id);

        let handle = tokio::spawn(async move {
            let mut operations_completed = 0;

            for op_num in 0..operations_per_worker {
                // Perform read-modify-write cycle
                let existing = provider_clone
                    .get_configuration(&tenant_id)
                    .await
                    .expect("Should get configuration")
                    .expect("Configuration should exist");

                let mut updated = existing.clone();
                updated.display_name = format!("Worker {} Operation {}", worker_id, op_num);

                // Some operations might fail due to version conflicts - that's expected
                if provider_clone.update_configuration(updated).await.is_ok() {
                    operations_completed += 1;
                }
            }

            operations_completed
        });

        handles.push(handle);
    }

    // Wait for all workers to complete
    let mut total_completed = 0;
    for handle in handles {
        let completed = handle.await.expect("Worker should complete");
        total_completed += completed;
    }

    let total_duration = start_time.elapsed();
    let operations_per_second = total_completed as f64 / total_duration.as_secs_f64();

    println!(
        "Concurrent modifications: {} operations completed in {:?} ({:.2} ops/sec)",
        total_completed, total_duration, operations_per_second
    );

    // Verify final state consistency
    let final_configs = provider
        .get_all_configurations()
        .await
        .expect("Should get final configurations");

    assert_eq!(final_configs.len(), num_workers);

    // All configurations should have been updated at least once
    for config in &final_configs {
        assert!(config.version > 1);
        assert!(config.display_name.contains("Worker"));
    }

    // Assert reasonable concurrent performance
    assert!(operations_per_second > 100.0); // Should handle at least 100 concurrent ops/sec
    assert!(total_completed >= num_workers); // Each worker should succeed at least once
}
