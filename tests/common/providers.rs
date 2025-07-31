//! Provider Test Utilities
//!
//! This module provides common utilities, fixtures, and helper functions
//! specifically for testing provider implementations in the multi-tenant SCIM system.
//! These utilities are shared across all provider-specific integration tests.

use serde_json::{Value, json};
use std::collections::HashMap;

// Re-export from integration tests for convenience
pub use crate::integration::providers::common::{
    MultiTenantScenarioBuilder, PopulatedTestData, ProviderTestConfig, ProviderTestingSuite,
    TenantPopulationResult, TestResult, TestResults,
};

/// Provider test categories for systematic testing
#[derive(Debug, Clone, PartialEq)]
pub enum ProviderTestCategory {
    BasicFunctionality,
    TenantIsolation,
    ConcurrentOperations,
    ErrorHandling,
    Performance,
    Configuration,
    Security,
    Compliance,
}

/// Provider test result with detailed metrics
#[derive(Debug)]
pub struct ProviderTestResult {
    pub category: ProviderTestCategory,
    pub test_name: String,
    pub success: bool,
    pub duration: std::time::Duration,
    pub details: String,
    pub metrics: HashMap<String, f64>,
}

impl ProviderTestResult {
    pub fn new(category: ProviderTestCategory, test_name: &str) -> Self {
        Self {
            category,
            test_name: test_name.to_string(),
            success: false,
            duration: std::time::Duration::from_secs(0),
            details: String::new(),
            metrics: HashMap::new(),
        }
    }

    pub fn success(mut self, duration: std::time::Duration, details: &str) -> Self {
        self.success = true;
        self.duration = duration;
        self.details = details.to_string();
        self
    }

    pub fn failure(mut self, duration: std::time::Duration, error: &str) -> Self {
        self.success = false;
        self.duration = duration;
        self.details = format!("FAILED: {}", error);
        self
    }

    pub fn with_metric(mut self, key: &str, value: f64) -> Self {
        self.metrics.insert(key.to_string(), value);
        self
    }
}

/// Provider performance metrics
#[derive(Debug, Clone)]
pub struct ProviderPerformanceMetrics {
    pub operations_per_second: f64,
    pub average_latency_ms: f64,
    pub p95_latency_ms: f64,
    pub p99_latency_ms: f64,
    pub memory_usage_mb: f64,
    pub cpu_usage_percent: f64,
    pub error_rate_percent: f64,
}

impl Default for ProviderPerformanceMetrics {
    fn default() -> Self {
        Self {
            operations_per_second: 0.0,
            average_latency_ms: 0.0,
            p95_latency_ms: 0.0,
            p99_latency_ms: 0.0,
            memory_usage_mb: 0.0,
            cpu_usage_percent: 0.0,
            error_rate_percent: 0.0,
        }
    }
}

/// Provider test data generator
pub struct ProviderTestDataGenerator;

impl ProviderTestDataGenerator {
    /// Generate realistic user data for testing
    pub fn generate_users(count: usize, tenant_prefix: &str) -> Vec<Value> {
        (0..count)
            .map(|i| {
                json!({
                    "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
                    "userName": format!("{}user{:04}", tenant_prefix, i),
                    "emails": [{
                        "value": format!("{}user{}@example.com", tenant_prefix, i),
                        "type": "work",
                        "primary": true
                    }],
                    "name": {
                        "givenName": format!("Test{}", i),
                        "familyName": "User",
                        "formatted": format!("Test{} User", i)
                    },
                    "displayName": format!("Test{} User", i),
                    "active": i % 10 != 0, // 90% active, 10% inactive
                    "userType": if i % 5 == 0 { "Employee" } else { "Contractor" },
                    "title": match i % 4 {
                        0 => "Developer",
                        1 => "Manager",
                        2 => "Analyst",
                        _ => "Specialist"
                    },
                    "department": match i % 3 {
                        0 => "Engineering",
                        1 => "Sales",
                        _ => "Support"
                    }
                })
            })
            .collect()
    }

    /// Generate realistic group data for testing
    pub fn generate_groups(count: usize, tenant_prefix: &str) -> Vec<Value> {
        (0..count)
            .map(|i| {
                json!({
                    "schemas": ["urn:ietf:params:scim:schemas:core:2.0:Group"],
                    "displayName": format!("{}Group{:04}", tenant_prefix, i),
                    "description": format!("Test group {} for {}", i, tenant_prefix),
                    "groupType": match i % 3 {
                        0 => "security",
                        1 => "distribution",
                        _ => "application"
                    },
                    "members": [] // Empty initially, can be populated later
                })
            })
            .collect()
    }

    /// Generate mixed resource data for bulk testing
    pub fn generate_mixed_resources(
        user_count: usize,
        group_count: usize,
        tenant_prefix: &str,
    ) -> Vec<(String, Value)> {
        let mut resources = Vec::new();

        // Add users
        for user in Self::generate_users(user_count, tenant_prefix) {
            resources.push(("User".to_string(), user));
        }

        // Add groups
        for group in Self::generate_groups(group_count, tenant_prefix) {
            resources.push(("Group".to_string(), group));
        }

        resources
    }

    /// Generate performance test data with varying sizes
    pub fn generate_performance_data(
        small_count: usize,
        medium_count: usize,
        large_count: usize,
        tenant_prefix: &str,
    ) -> HashMap<String, Vec<Value>> {
        let mut data = HashMap::new();

        data.insert(
            "small_users".to_string(),
            Self::generate_users(small_count, &format!("{}_small_", tenant_prefix)),
        );

        data.insert(
            "medium_users".to_string(),
            Self::generate_users(medium_count, &format!("{}_med_", tenant_prefix)),
        );

        data.insert(
            "large_users".to_string(),
            Self::generate_users(large_count, &format!("{}_large_", tenant_prefix)),
        );

        data
    }
}

/// Provider test validation utilities
pub struct ProviderTestValidation;

impl ProviderTestValidation {
    /// Validate that a resource has required SCIM fields
    pub fn validate_scim_resource(resource: &Value, resource_type: &str) -> Result<(), String> {
        // Check schemas
        let schemas = resource
            .get("schemas")
            .ok_or("Missing schemas array")?
            .as_array()
            .ok_or("Schemas must be an array")?;

        if schemas.is_empty() {
            return Err("Schemas array cannot be empty".to_string());
        }

        let expected_schema = format!("urn:ietf:params:scim:schemas:core:2.0:{}", resource_type);
        if !schemas.iter().any(|s| s.as_str() == Some(&expected_schema)) {
            return Err(format!("Missing required schema: {}", expected_schema));
        }

        // Check ID (if present)
        if let Some(id) = resource.get("id") {
            if !id.is_string() || id.as_str().unwrap().is_empty() {
                return Err("ID must be a non-empty string".to_string());
            }
        }

        // Resource-specific validation
        match resource_type {
            "User" => Self::validate_user_resource(resource)?,
            "Group" => Self::validate_group_resource(resource)?,
            _ => {} // Skip validation for unknown types
        }

        Ok(())
    }

    fn validate_user_resource(resource: &Value) -> Result<(), String> {
        // userName is required
        resource
            .get("userName")
            .ok_or("Missing required userName")?
            .as_str()
            .ok_or("userName must be a string")?;

        // Validate emails structure if present
        if let Some(emails) = resource.get("emails") {
            let emails_array = emails.as_array().ok_or("emails must be an array")?;

            for email in emails_array {
                email
                    .get("value")
                    .ok_or("Email must have value field")?
                    .as_str()
                    .ok_or("Email value must be a string")?;
            }
        }

        // Validate name structure if present
        if let Some(name) = resource.get("name") {
            if !name.is_object() {
                return Err("name must be an object".to_string());
            }
        }

        Ok(())
    }

    fn validate_group_resource(resource: &Value) -> Result<(), String> {
        // displayName is required
        resource
            .get("displayName")
            .ok_or("Missing required displayName")?
            .as_str()
            .ok_or("displayName must be a string")?;

        // Validate members structure if present
        if let Some(members) = resource.get("members") {
            let members_array = members.as_array().ok_or("members must be an array")?;

            for member in members_array {
                member
                    .get("value")
                    .ok_or("Member must have value field")?
                    .as_str()
                    .ok_or("Member value must be a string")?;
            }
        }

        Ok(())
    }

    /// Validate tenant isolation in a list of resources
    pub fn validate_tenant_isolation(
        resources: &[Value],
        _expected_tenant_markers: &HashMap<String, String>,
    ) -> Result<(), String> {
        for resource in resources {
            // Verify resource has proper structure
            Self::validate_scim_resource(resource, "unknown")?;

            // Additional tenant-specific validation would go here
            // This depends on how tenant information is encoded in resources
        }

        Ok(())
    }

    /// Validate bulk operation results
    pub fn validate_bulk_results(
        results: &[Value],
        expected_success_count: usize,
    ) -> Result<(), String> {
        if results.len() < expected_success_count {
            return Err(format!(
                "Expected at least {} results, got {}",
                expected_success_count,
                results.len()
            ));
        }

        for result in results {
            Self::validate_scim_resource(result, "unknown")?;
        }

        Ok(())
    }
}

/// Provider performance testing utilities
pub struct ProviderPerformanceTester;

impl ProviderPerformanceTester {
    /// Run a performance test with specified parameters
    pub async fn run_performance_test<F, Fut>(
        test_name: &str,
        operation_count: usize,
        concurrent_operations: usize,
        operation: F,
    ) -> ProviderPerformanceMetrics
    where
        F: Fn(usize) -> Fut + Send + Sync + Clone + 'static,
        Fut: std::future::Future<Output = Result<(), Box<dyn std::error::Error + Send + Sync>>>
            + Send
            + 'static,
    {
        let start_time = std::time::Instant::now();
        let _handles: Vec<
            tokio::task::JoinHandle<Result<usize, Box<dyn std::error::Error + Send + Sync>>>,
        > = Vec::new();
        let mut latencies = Vec::new();
        let mut errors = 0;

        // Run operations concurrently
        for batch_start in (0..operation_count).step_by(concurrent_operations) {
            let batch_end = std::cmp::min(batch_start + concurrent_operations, operation_count);
            let mut batch_handles = Vec::new();

            for i in batch_start..batch_end {
                let op = operation.clone();
                let handle = tokio::spawn(async move {
                    let op_start = std::time::Instant::now();
                    let result = op(i).await;
                    let duration = op_start.elapsed();
                    (result, duration)
                });
                batch_handles.push(handle);
            }

            // Wait for batch to complete
            for handle in batch_handles {
                match handle.await {
                    Ok((result, duration)) => {
                        latencies.push(duration.as_millis() as f64);
                        if result.is_err() {
                            errors += 1;
                        }
                    }
                    Err(_) => {
                        errors += 1;
                    }
                }
            }
        }

        let total_duration = start_time.elapsed();

        // Calculate metrics
        let operations_per_second = operation_count as f64 / total_duration.as_secs_f64();
        let average_latency = latencies.iter().sum::<f64>() / latencies.len() as f64;

        latencies.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let p95_index = (latencies.len() as f64 * 0.95) as usize;
        let p99_index = (latencies.len() as f64 * 0.99) as usize;
        let p95_latency = latencies.get(p95_index).copied().unwrap_or(0.0);
        let p99_latency = latencies.get(p99_index).copied().unwrap_or(0.0);

        let error_rate = (errors as f64 / operation_count as f64) * 100.0;

        println!(
            "Performance test '{}' completed: {:.2} ops/sec, {:.2}ms avg latency, {:.2}% error rate",
            test_name, operations_per_second, average_latency, error_rate
        );

        ProviderPerformanceMetrics {
            operations_per_second,
            average_latency_ms: average_latency,
            p95_latency_ms: p95_latency,
            p99_latency_ms: p99_latency,
            memory_usage_mb: 0.0,   // Would need actual memory monitoring
            cpu_usage_percent: 0.0, // Would need actual CPU monitoring
            error_rate_percent: error_rate,
        }
    }

    /// Compare performance metrics between different providers
    pub fn compare_providers(
        provider_a_metrics: &ProviderPerformanceMetrics,
        provider_b_metrics: &ProviderPerformanceMetrics,
    ) -> String {
        let ops_ratio =
            provider_a_metrics.operations_per_second / provider_b_metrics.operations_per_second;
        let latency_ratio =
            provider_a_metrics.average_latency_ms / provider_b_metrics.average_latency_ms;
        let error_ratio =
            provider_a_metrics.error_rate_percent / provider_b_metrics.error_rate_percent.max(0.01);

        format!(
            "Provider comparison:\n\
             - Operations/sec: {:.2}x\n\
             - Average latency: {:.2}x\n\
             - Error rate: {:.2}x",
            ops_ratio, latency_ratio, error_ratio
        )
    }
}

/// Provider configuration testing utilities
pub struct ProviderConfigTester;

impl ProviderConfigTester {
    /// Create standard test configurations for providers
    pub fn standard_configs() -> Vec<ProviderTestConfig> {
        vec![
            ProviderTestConfig::new("minimal")
                .with_setting("max_connections", json!(1))
                .with_setting("timeout_ms", json!(1000)),
            ProviderTestConfig::new("standard")
                .with_setting("max_connections", json!(10))
                .with_setting("timeout_ms", json!(5000))
                .with_setting("cache_enabled", json!(true)),
            ProviderTestConfig::new("high_performance")
                .with_setting("max_connections", json!(100))
                .with_setting("timeout_ms", json!(10000))
                .with_setting("cache_enabled", json!(true))
                .with_setting("bulk_operations", json!(true)),
            ProviderTestConfig::new("strict_security")
                .with_setting("max_connections", json!(5))
                .with_setting("timeout_ms", json!(2000))
                .with_setting("encryption", json!(true))
                .with_setting("audit_logging", json!(true)),
        ]
    }

    /// Validate provider configuration
    pub fn validate_config(config: &ProviderTestConfig) -> Result<(), String> {
        // Basic validation
        if config.name.is_empty() {
            return Err("Configuration name cannot be empty".to_string());
        }

        // Validate specific settings
        if let Some(max_conn) = config.settings.get("max_connections") {
            if let Some(val) = max_conn.as_i64() {
                if val <= 0 {
                    return Err("max_connections must be positive".to_string());
                }
            }
        }

        if let Some(timeout) = config.settings.get("timeout_ms") {
            if let Some(val) = timeout.as_i64() {
                if val <= 0 {
                    return Err("timeout_ms must be positive".to_string());
                }
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_provider_test_result() {
        let result =
            ProviderTestResult::new(ProviderTestCategory::BasicFunctionality, "test_create")
                .success(
                    std::time::Duration::from_millis(100),
                    "Successfully created resource",
                )
                .with_metric("operations_per_second", 1000.0);

        assert!(result.success);
        assert_eq!(result.test_name, "test_create");
        assert_eq!(result.category, ProviderTestCategory::BasicFunctionality);
        assert_eq!(result.metrics.get("operations_per_second"), Some(&1000.0));
    }

    #[test]
    fn test_generate_users() {
        let users = ProviderTestDataGenerator::generate_users(3, "test_");
        assert_eq!(users.len(), 3);

        for (i, user) in users.iter().enumerate() {
            assert_eq!(user["userName"], format!("test_user{:04}", i));
            assert_eq!(
                user["emails"][0]["value"],
                format!("test_user{}@example.com", i)
            );
            assert!(
                user["schemas"]
                    .as_array()
                    .unwrap()
                    .contains(&json!("urn:ietf:params:scim:schemas:core:2.0:User"))
            );
        }
    }

    #[test]
    fn test_generate_groups() {
        let groups = ProviderTestDataGenerator::generate_groups(2, "org_");
        assert_eq!(groups.len(), 2);

        for (i, group) in groups.iter().enumerate() {
            assert_eq!(group["displayName"], format!("org_Group{:04}", i));
            assert!(
                group["schemas"]
                    .as_array()
                    .unwrap()
                    .contains(&json!("urn:ietf:params:scim:schemas:core:2.0:Group"))
            );
        }
    }

    #[test]
    fn test_validate_scim_resource() {
        let valid_user = json!({
            "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
            "id": "123",
            "userName": "testuser"
        });

        let result = ProviderTestValidation::validate_scim_resource(&valid_user, "User");
        assert!(result.is_ok());

        let invalid_user = json!({
            "schemas": [],
            "userName": "testuser"
        });

        let result = ProviderTestValidation::validate_scim_resource(&invalid_user, "User");
        assert!(result.is_err());
    }

    #[test]
    fn test_provider_config_validation() {
        let valid_config = ProviderTestConfig::new("test")
            .with_setting("max_connections", json!(10))
            .with_setting("timeout_ms", json!(5000));

        assert!(ProviderConfigTester::validate_config(&valid_config).is_ok());

        let invalid_config = ProviderTestConfig::new("").with_setting("max_connections", json!(-1));

        assert!(ProviderConfigTester::validate_config(&invalid_config).is_err());
    }

    #[test]
    fn test_standard_configs() {
        let configs = ProviderConfigTester::standard_configs();
        assert!(!configs.is_empty());

        for config in &configs {
            assert!(ProviderConfigTester::validate_config(config).is_ok());
        }
    }

    #[test]
    fn test_performance_metrics_default() {
        let metrics = ProviderPerformanceMetrics::default();
        assert_eq!(metrics.operations_per_second, 0.0);
        assert_eq!(metrics.average_latency_ms, 0.0);
        assert_eq!(metrics.error_rate_percent, 0.0);
    }
}
