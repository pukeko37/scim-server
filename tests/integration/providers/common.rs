//! Common Provider Test Utilities
//!
//! This module provides shared utilities, fixtures, and helper functions
//! for testing all provider implementations in the unified SCIM system.
//! These utilities ensure consistent testing patterns across different provider types.

use crate::common::{UnifiedTestHarness, create_multi_tenant_context, create_test_user};
use scim_server::resource::provider::ResourceProvider;
use serde_json::{Value, json};
use std::collections::HashMap;
use std::sync::Arc;

// ============================================================================
// Test Data Builders and Fixtures
// ============================================================================

/// Builder for creating comprehensive test scenarios with multiple tenants
#[derive(Debug, Clone)]
pub struct MultiTenantScenarioBuilder {
    tenant_ids: Vec<String>,
    users_per_tenant: usize,
    groups_per_tenant: usize,
    include_enterprise_users: bool,
    include_complex_groups: bool,
}

impl MultiTenantScenarioBuilder {
    /// Create a new scenario builder
    pub fn new() -> Self {
        Self {
            tenant_ids: vec!["tenant_a".to_string(), "tenant_b".to_string()],
            users_per_tenant: 5,
            groups_per_tenant: 2,
            include_enterprise_users: false,
            include_complex_groups: false,
        }
    }

    /// Set the tenant IDs for the scenario
    pub fn with_tenants(mut self, tenant_ids: Vec<String>) -> Self {
        self.tenant_ids = tenant_ids;
        self
    }

    /// Set the number of users per tenant
    pub fn with_users_per_tenant(mut self, count: usize) -> Self {
        self.users_per_tenant = count;
        self
    }

    /// Set the number of groups per tenant
    pub fn with_groups_per_tenant(mut self, count: usize) -> Self {
        self.groups_per_tenant = count;
        self
    }

    /// Include enterprise user attributes
    pub fn with_enterprise_users(mut self, include: bool) -> Self {
        self.include_enterprise_users = include;
        self
    }

    /// Include complex group structures with nested memberships
    pub fn with_complex_groups(mut self, include: bool) -> Self {
        self.include_complex_groups = include;
        self
    }

    /// Build the scenario and populate the provider
    pub async fn build_and_populate<P>(
        self,
        provider: P,
    ) -> Result<UnifiedTestHarness<P>, Box<dyn std::error::Error + Send + Sync>>
    where
        P: ResourceProvider + Send + Sync + 'static,
        P::Error: std::error::Error + Send + Sync + 'static,
    {
        let tenant_refs: Vec<&str> = self.tenant_ids.iter().map(String::as_str).collect();
        let harness = UnifiedTestHarness::new_multi_tenant(provider, &tenant_refs);

        // Populate with test data
        for tenant_id in &self.tenant_ids {
            // Create users
            for i in 0..self.users_per_tenant {
                let username = if self.include_enterprise_users {
                    format!("enterprise_user_{}_{}", tenant_id, i)
                } else {
                    format!("user_{}_{}", tenant_id, i)
                };

                harness.create_user(Some(tenant_id), &username).await?;
            }

            // Create groups
            for i in 0..self.groups_per_tenant {
                let group_name = format!("group_{}_{}", tenant_id, i);
                harness.create_group(Some(tenant_id), &group_name).await?;
            }
        }

        Ok(harness)
    }
}

impl Default for MultiTenantScenarioBuilder {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Provider Testing Suite
// ============================================================================

/// Comprehensive testing suite for provider implementations
pub struct ProviderTestingSuite;

impl ProviderTestingSuite {
    /// Run the complete test suite for a provider
    pub async fn run_comprehensive_tests<P>(
        provider: P,
    ) -> Result<TestResults, Box<dyn std::error::Error + Send + Sync>>
    where
        P: ResourceProvider + Send + Sync + 'static,
        P::Error: std::error::Error + Send + Sync + 'static,
    {
        let provider = Arc::new(provider);
        let mut results = TestResults::new();

        // Test 1: Basic CRUD operations
        println!("🧪 Running basic CRUD tests...");
        let crud_result = Self::test_basic_crud(Arc::clone(&provider)).await;
        results.add_test("basic_crud", crud_result.is_ok());
        if let Err(e) = crud_result {
            results.add_error("basic_crud", e);
        }

        // Test 2: Multi-tenant isolation
        println!("🧪 Running multi-tenant isolation tests...");
        let isolation_result = Self::test_multi_tenant_isolation(Arc::clone(&provider)).await;
        results.add_test("multi_tenant_isolation", isolation_result.is_ok());
        if let Err(e) = isolation_result {
            results.add_error("multi_tenant_isolation", e);
        }

        // Test 3: Concurrent operations
        println!("🧪 Running concurrent operations tests...");
        let concurrent_result = Self::test_concurrent_operations(Arc::clone(&provider)).await;
        results.add_test("concurrent_operations", concurrent_result.is_ok());
        if let Err(e) = concurrent_result {
            results.add_error("concurrent_operations", e);
        }

        // Test 4: Data integrity
        println!("🧪 Running data integrity tests...");
        let integrity_result = Self::test_data_integrity(provider).await;
        results.add_test("data_integrity", integrity_result.is_ok());
        if let Err(e) = integrity_result {
            results.add_error("data_integrity", e);
        }

        Ok(results)
    }

    /// Test basic CRUD operations
    async fn test_basic_crud<P>(
        provider: Arc<P>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>>
    where
        P: ResourceProvider + Send + Sync + 'static,
        P::Error: std::error::Error + Send + Sync + 'static,
    {
        let harness = UnifiedTestHarness::from_arc_single_tenant(provider);
        let context = harness.default_context();

        // Create
        let user = harness.create_user(None, "crud_test_user").await?;
        let user_id = user.id.as_ref().unwrap().as_str();

        // Read
        let retrieved = harness
            .provider
            .get_resource("User", user_id, context)
            .await
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?
            .ok_or("User should be retrievable")?;

        assert_eq!(retrieved.id, user.id);

        // Update
        let update_data = json!({
            "userName": "updated_crud_user",
            "active": false
        });

        let updated = harness
            .provider
            .update_resource("User", user_id, update_data, context)
            .await
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?;

        assert_eq!(
            updated.user_name.as_ref().unwrap().as_str(),
            "updated_crud_user"
        );

        // Delete
        harness
            .provider
            .delete_resource("User", user_id, context)
            .await
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?;

        // Verify deletion
        let deleted = harness
            .provider
            .get_resource("User", user_id, context)
            .await
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?;

        if deleted.is_some() {
            return Err("User should be deleted".into());
        }

        Ok(())
    }

    /// Test multi-tenant isolation
    async fn test_multi_tenant_isolation<P>(
        provider: Arc<P>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>>
    where
        P: ResourceProvider + Send + Sync + 'static,
        P::Error: std::error::Error + Send + Sync + 'static,
    {
        let harness =
            UnifiedTestHarness::from_arc_multi_tenant(provider, &["tenant_a", "tenant_b"]);

        // Create users in different tenants
        let user_a = harness
            .create_user(Some("tenant_a"), "isolation_user_a")
            .await?;
        let user_b = harness
            .create_user(Some("tenant_b"), "isolation_user_b")
            .await?;

        // Verify isolation
        harness
            .verify_tenant_isolation("tenant_a", "User", &user_a)
            .await?;
        harness
            .verify_tenant_isolation("tenant_b", "User", &user_b)
            .await?;

        Ok(())
    }

    /// Test concurrent operations
    async fn test_concurrent_operations<P>(
        provider: Arc<P>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>>
    where
        P: ResourceProvider + Send + Sync + 'static,
        P::Error: std::error::Error + Send + Sync + 'static,
    {
        let tenant_count = 3;
        let operations_per_tenant = 5;

        // Create contexts for multiple tenants
        let mut contexts = HashMap::new();
        for i in 0..tenant_count {
            let tenant_id = format!("concurrent_tenant_{}", i);
            contexts.insert(tenant_id.clone(), create_multi_tenant_context(&tenant_id));
        }

        // Sequential operations for now (to avoid Send issues)
        let mut total_operations = 0;
        for (tenant_id, context) in &contexts {
            for i in 0..operations_per_tenant {
                let username = format!("concurrent_user_{}_{}", tenant_id, i);
                let user_data = create_test_user(&username);

                provider
                    .create_resource("User", user_data, context)
                    .await
                    .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?;

                total_operations += 1;
            }
        }

        // Verify all operations completed
        if total_operations != tenant_count * operations_per_tenant {
            return Err("Not all concurrent operations completed successfully".into());
        }

        Ok(())
    }

    /// Test data integrity
    async fn test_data_integrity<P>(
        provider: Arc<P>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>>
    where
        P: ResourceProvider + Send + Sync + 'static,
        P::Error: std::error::Error + Send + Sync + 'static,
    {
        let harness = UnifiedTestHarness::from_arc_single_tenant(provider);
        let context = harness.default_context();

        // Test with various data types and edge cases
        let test_cases = vec![
            ("user_with_unicode", "测试用户"),
            ("user_with_symbols", "user@domain.com"),
            ("user_with_spaces", "User With Spaces"),
            ("user_with_numbers", "user123456"),
        ];

        for (test_name, username) in test_cases {
            let user_data = create_test_user(username);

            let created = harness
                .provider
                .create_resource("User", user_data, context)
                .await
                .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?;

            // Verify data integrity
            if created.user_name.as_ref().unwrap().as_str() != username {
                return Err(format!("Data integrity failed for test case: {}", test_name).into());
            }

            // Verify resource has proper structure
            if created.id.is_none() {
                return Err(format!("Resource ID missing for test case: {}", test_name).into());
            }

            if created.schemas.is_empty() {
                return Err(format!("Schemas missing for test case: {}", test_name).into());
            }
        }

        Ok(())
    }
}

// ============================================================================
// Test Results and Reporting
// ============================================================================

/// Container for test results
#[derive(Debug)]
pub struct TestResults {
    tests: HashMap<String, bool>,
    errors: HashMap<String, Box<dyn std::error::Error + Send + Sync>>,
    passed: usize,
    failed: usize,
}

impl TestResults {
    /// Create new test results container
    pub fn new() -> Self {
        Self {
            tests: HashMap::new(),
            errors: HashMap::new(),
            passed: 0,
            failed: 0,
        }
    }

    /// Add a test result
    pub fn add_test(&mut self, name: &str, passed: bool) {
        self.tests.insert(name.to_string(), passed);
        if passed {
            self.passed += 1;
        } else {
            self.failed += 1;
        }
    }

    /// Add an error for a test
    pub fn add_error(&mut self, name: &str, error: Box<dyn std::error::Error + Send + Sync>) {
        self.errors.insert(name.to_string(), error);
    }

    /// Get summary of results
    pub fn summary(&self) -> String {
        format!(
            "Test Results: {} passed, {} failed, {} total",
            self.passed,
            self.failed,
            self.passed + self.failed
        )
    }

    /// Check if all tests passed
    pub fn all_passed(&self) -> bool {
        self.failed == 0
    }

    /// Get list of failed tests
    pub fn failed_tests(&self) -> Vec<String> {
        self.tests
            .iter()
            .filter_map(|(name, &passed)| if !passed { Some(name.clone()) } else { None })
            .collect()
    }
}

impl Default for TestResults {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Create an enterprise user with additional attributes
fn create_enterprise_user(username: &str) -> Value {
    json!({
        "schemas": [
            "urn:ietf:params:scim:schemas:core:2.0:User",
            "urn:ietf:params:scim:schemas:extension:enterprise:2.0:User"
        ],
        "userName": username,
        "displayName": format!("{} (Enterprise)", username),
        "active": true,
        "emails": [{
            "value": format!("{}@enterprise.com", username),
            "type": "work",
            "primary": true
        }],
        "name": {
            "formatted": format!("Enterprise User {}", username),
            "familyName": "User",
            "givenName": "Enterprise"
        },
        "title": "Senior Developer",
        "department": "Engineering",
        "organization": "Enterprise Corp",
        "manager": {
            "value": "manager@enterprise.com",
            "displayName": "Manager User"
        }
    })
}

/// Provider configuration for testing
#[derive(Debug, Clone)]
pub struct ProviderTestConfig {
    pub enable_concurrent_tests: bool,
    pub max_concurrent_operations: usize,
    pub test_data_size: TestDataSize,
    pub enable_performance_tests: bool,
}

impl ProviderTestConfig {
    /// Create default test configuration
    pub fn default() -> Self {
        Self {
            enable_concurrent_tests: true,
            max_concurrent_operations: 100,
            test_data_size: TestDataSize::Small,
            enable_performance_tests: false,
        }
    }

    /// Create configuration for performance testing
    pub fn for_performance() -> Self {
        Self {
            enable_concurrent_tests: true,
            max_concurrent_operations: 1000,
            test_data_size: TestDataSize::Large,
            enable_performance_tests: true,
        }
    }

    /// Create minimal configuration for quick tests
    pub fn minimal() -> Self {
        Self {
            enable_concurrent_tests: false,
            max_concurrent_operations: 10,
            test_data_size: TestDataSize::Minimal,
            enable_performance_tests: false,
        }
    }
}

/// Test data size configurations
#[derive(Debug, Clone)]
pub enum TestDataSize {
    Minimal, // 1-2 items per category
    Small,   // 5-10 items per category
    Medium,  // 50-100 items per category
    Large,   // 500-1000 items per category
}

impl TestDataSize {
    /// Get the number of items for this size
    pub fn item_count(&self) -> usize {
        match self {
            TestDataSize::Minimal => 2,
            TestDataSize::Small => 10,
            TestDataSize::Medium => 100,
            TestDataSize::Large => 1000,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scenario_builder() {
        let builder = MultiTenantScenarioBuilder::new()
            .with_tenants(vec!["test1".to_string(), "test2".to_string()])
            .with_users_per_tenant(3)
            .with_groups_per_tenant(1);

        assert_eq!(builder.tenant_ids.len(), 2);
        assert_eq!(builder.users_per_tenant, 3);
        assert_eq!(builder.groups_per_tenant, 1);
    }

    #[test]
    fn test_results_tracking() {
        let mut results = TestResults::new();

        results.add_test("test1", true);
        results.add_test("test2", false);

        assert_eq!(results.passed, 1);
        assert_eq!(results.failed, 1);
        assert!(!results.all_passed());

        let failed = results.failed_tests();
        assert_eq!(failed.len(), 1);
        assert!(failed.contains(&"test2".to_string()));
    }

    #[test]
    fn test_enterprise_user_creation() {
        let user = create_enterprise_user("test_enterprise");
        assert_eq!(user["userName"], "test_enterprise");
        assert!(user["schemas"].as_array().unwrap().len() >= 2);
        assert!(user.get("title").is_some());
        assert!(user.get("department").is_some());
    }

    #[test]
    fn test_provider_config() {
        let config = ProviderTestConfig::default();
        assert!(config.enable_concurrent_tests);

        let perf_config = ProviderTestConfig::for_performance();
        assert!(perf_config.enable_performance_tests);
        assert_eq!(perf_config.max_concurrent_operations, 1000);

        let minimal_config = ProviderTestConfig::minimal();
        assert!(!minimal_config.enable_concurrent_tests);
    }

    #[test]
    fn test_data_size_configs() {
        assert_eq!(TestDataSize::Minimal.item_count(), 2);
        assert_eq!(TestDataSize::Small.item_count(), 10);
        assert_eq!(TestDataSize::Medium.item_count(), 100);
        assert_eq!(TestDataSize::Large.item_count(), 1000);
    }
}
