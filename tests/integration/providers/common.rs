//! Common Provider Test Utilities
//!
//! This module provides shared utilities, fixtures, and helper functions
//! for testing all provider implementations in the multi-tenant SCIM system.
//! These utilities ensure consistent testing patterns across different provider types.

use crate::integration::multi_tenant::core::{
    EnhancedRequestContext, TenantContext, TenantContextBuilder,
};
use crate::integration::multi_tenant::provider_trait::{ListQuery, MultiTenantResourceProvider};
use scim_server::Resource;
use serde_json::{Value, json};
use std::collections::HashMap;
use std::sync::Arc;

// ============================================================================
// Test Data Builders and Fixtures
// ============================================================================

/// Builder for creating comprehensive test scenarios with multiple tenants
#[derive(Debug, Clone)]
pub struct MultiTenantScenarioBuilder {
    tenants: Vec<TenantTestData>,
    users_per_tenant: usize,
    groups_per_tenant: usize,
}

impl MultiTenantScenarioBuilder {
    pub fn new() -> Self {
        Self {
            tenants: Vec::new(),
            users_per_tenant: 3,
            groups_per_tenant: 2,
        }
    }

    pub fn add_tenant(mut self, tenant_id: &str) -> Self {
        self.tenants.push(TenantTestData::new(tenant_id));
        self
    }

    pub fn with_users_per_tenant(mut self, count: usize) -> Self {
        self.users_per_tenant = count;
        self
    }

    pub fn with_groups_per_tenant(mut self, count: usize) -> Self {
        self.groups_per_tenant = count;
        self
    }

    pub fn build(self) -> MultiTenantTestScenario {
        MultiTenantTestScenario {
            tenants: self.tenants,
            users_per_tenant: self.users_per_tenant,
            groups_per_tenant: self.groups_per_tenant,
        }
    }
}

/// Test data for a single tenant
#[derive(Debug, Clone)]
pub struct TenantTestData {
    pub tenant_id: String,
    pub context: TenantContext,
    pub users: Vec<TestUserData>,
    pub groups: Vec<TestGroupData>,
}

impl TenantTestData {
    pub fn new(tenant_id: &str) -> Self {
        Self {
            tenant_id: tenant_id.to_string(),
            context: TenantContextBuilder::new(tenant_id).build(),
            users: Vec::new(),
            groups: Vec::new(),
        }
    }

    pub fn add_user(mut self, username: &str) -> Self {
        self.users.push(TestUserData::new(username));
        self
    }

    pub fn add_group(mut self, group_name: &str) -> Self {
        self.groups.push(TestGroupData::new(group_name));
        self
    }
}

/// Test data for a user resource
#[derive(Debug, Clone)]
pub struct TestUserData {
    pub username: String,
    pub display_name: String,
    pub email: String,
    pub active: bool,
    pub attributes: HashMap<String, Value>,
}

impl TestUserData {
    pub fn new(username: &str) -> Self {
        Self {
            username: username.to_string(),
            display_name: format!("{} User", username),
            email: format!("{}@example.com", username),
            active: true,
            attributes: HashMap::new(),
        }
    }

    pub fn with_display_name(mut self, display_name: &str) -> Self {
        self.display_name = display_name.to_string();
        self
    }

    pub fn with_email(mut self, email: &str) -> Self {
        self.email = email.to_string();
        self
    }

    pub fn inactive(mut self) -> Self {
        self.active = false;
        self
    }

    pub fn with_attribute(mut self, key: &str, value: Value) -> Self {
        self.attributes.insert(key.to_string(), value);
        self
    }

    pub fn to_scim_json(&self) -> Value {
        let mut json = json!({
            "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
            "userName": self.username,
            "displayName": self.display_name,
            "active": self.active,
            "emails": [{
                "value": self.email,
                "type": "work",
                "primary": true
            }]
        });

        // Add custom attributes
        if let Some(obj) = json.as_object_mut() {
            for (key, value) in &self.attributes {
                obj.insert(key.clone(), value.clone());
            }
        }

        json
    }
}

/// Test data for a group resource
#[derive(Debug, Clone)]
pub struct TestGroupData {
    pub display_name: String,
    pub description: String,
    pub members: Vec<String>,
    pub attributes: HashMap<String, Value>,
}

impl TestGroupData {
    pub fn new(display_name: &str) -> Self {
        Self {
            display_name: display_name.to_string(),
            description: format!("{} group for testing", display_name),
            members: Vec::new(),
            attributes: HashMap::new(),
        }
    }

    pub fn with_description(mut self, description: &str) -> Self {
        self.description = description.to_string();
        self
    }

    pub fn with_member(mut self, member_id: &str) -> Self {
        self.members.push(member_id.to_string());
        self
    }

    pub fn with_attribute(mut self, key: &str, value: Value) -> Self {
        self.attributes.insert(key.to_string(), value);
        self
    }

    pub fn to_scim_json(&self) -> Value {
        let members: Vec<Value> = self
            .members
            .iter()
            .map(|id| json!({"value": id, "type": "User"}))
            .collect();

        let mut json = json!({
            "schemas": ["urn:ietf:params:scim:schemas:core:2.0:Group"],
            "displayName": self.display_name,
            "description": self.description,
            "members": members
        });

        // Add custom attributes
        if let Some(obj) = json.as_object_mut() {
            for (key, value) in &self.attributes {
                obj.insert(key.clone(), value.clone());
            }
        }

        json
    }
}

/// Complete multi-tenant test scenario
#[derive(Debug)]
pub struct MultiTenantTestScenario {
    pub tenants: Vec<TenantTestData>,
    pub users_per_tenant: usize,
    pub groups_per_tenant: usize,
}

impl MultiTenantTestScenario {
    /// Populate a provider with the test scenario data
    pub async fn populate_provider<P: MultiTenantResourceProvider>(
        &self,
        provider: &P,
    ) -> Result<PopulatedTestData, Box<dyn std::error::Error>>
    where
        P::Error: std::fmt::Debug,
    {
        let mut populated = PopulatedTestData::new();

        for tenant_data in &self.tenants {
            let context = EnhancedRequestContext {
                request_id: format!("req_{}", tenant_data.tenant_id),
                tenant_context: tenant_data.context.clone(),
            };

            let mut tenant_result = TenantPopulationResult {
                tenant_id: tenant_data.tenant_id.clone(),
                users: Vec::new(),
                groups: Vec::new(),
            };

            // Create users for this tenant
            for i in 0..self.users_per_tenant {
                let username = format!("user_{}_{}", tenant_data.tenant_id, i + 1);
                let user_data = TestUserData::new(&username).to_scim_json();

                let created_user = provider
                    .create_resource(&tenant_data.tenant_id, "User", user_data, &context)
                    .await
                    .map_err(|e| format!("Failed to create user {}: {:?}", username, e))?;

                tenant_result.users.push(created_user);
            }

            // Create groups for this tenant
            for i in 0..self.groups_per_tenant {
                let group_name = format!("group_{}_{}", tenant_data.tenant_id, i + 1);
                let group_data = TestGroupData::new(&group_name).to_scim_json();

                let created_group = provider
                    .create_resource(&tenant_data.tenant_id, "Group", group_data, &context)
                    .await
                    .map_err(|e| format!("Failed to create group {}: {:?}", group_name, e))?;

                tenant_result.groups.push(created_group);
            }

            populated.tenants.push(tenant_result);
        }

        Ok(populated)
    }
}

/// Results of populating a provider with test data
#[derive(Debug)]
pub struct PopulatedTestData {
    pub tenants: Vec<TenantPopulationResult>,
}

impl PopulatedTestData {
    pub fn new() -> Self {
        Self {
            tenants: Vec::new(),
        }
    }

    pub fn get_tenant(&self, tenant_id: &str) -> Option<&TenantPopulationResult> {
        self.tenants.iter().find(|t| t.tenant_id == tenant_id)
    }

    pub fn total_users(&self) -> usize {
        self.tenants.iter().map(|t| t.users.len()).sum()
    }

    pub fn total_groups(&self) -> usize {
        self.tenants.iter().map(|t| t.groups.len()).sum()
    }
}

/// Population results for a single tenant
#[derive(Debug)]
pub struct TenantPopulationResult {
    pub tenant_id: String,
    pub users: Vec<Resource>,
    pub groups: Vec<Resource>,
}

// ============================================================================
// Provider Testing Utilities
// ============================================================================

/// Comprehensive provider testing harness
pub struct ProviderTestingSuite<P: MultiTenantResourceProvider> {
    provider: Arc<P>,
    scenario: MultiTenantTestScenario,
}

impl<P: MultiTenantResourceProvider + 'static> ProviderTestingSuite<P>
where
    P::Error: std::fmt::Debug,
{
    pub fn new(provider: Arc<P>) -> Self {
        let scenario = MultiTenantScenarioBuilder::new()
            .add_tenant("tenant_alpha")
            .add_tenant("tenant_beta")
            .add_tenant("tenant_gamma")
            .with_users_per_tenant(5)
            .with_groups_per_tenant(3)
            .build();

        Self { provider, scenario }
    }

    pub fn with_scenario(mut self, scenario: MultiTenantTestScenario) -> Self {
        self.scenario = scenario;
        self
    }

    /// Run a comprehensive test suite on the provider
    pub async fn run_comprehensive_tests(&self) -> Result<TestResults, Box<dyn std::error::Error>> {
        let mut results = TestResults::new();

        // Test 1: Basic functionality
        let basic_result = self.test_basic_functionality().await?;
        results.add_test_result("basic_functionality", basic_result);

        // Test 2: Tenant isolation
        let isolation_result = self.test_tenant_isolation().await?;
        results.add_test_result("tenant_isolation", isolation_result);

        // Test 3: Concurrent operations
        let concurrent_result = self.test_concurrent_operations().await?;
        results.add_test_result("concurrent_operations", concurrent_result);

        // Test 4: Error handling
        let error_result = self.test_error_handling().await?;
        results.add_test_result("error_handling", error_result);

        // Test 5: Performance characteristics
        let performance_result = self.test_performance_characteristics().await?;
        results.add_test_result("performance", performance_result);

        Ok(results)
    }

    async fn test_basic_functionality(&self) -> Result<TestResult, Box<dyn std::error::Error>> {
        let start = std::time::Instant::now();

        // Populate provider with test data
        let populated = self.scenario.populate_provider(&*self.provider).await?;

        // Verify all resources were created
        assert_eq!(populated.tenants.len(), self.scenario.tenants.len());

        for tenant_result in &populated.tenants {
            assert_eq!(tenant_result.users.len(), self.scenario.users_per_tenant);
            assert_eq!(tenant_result.groups.len(), self.scenario.groups_per_tenant);
        }

        Ok(TestResult {
            duration: start.elapsed(),
            success: true,
            details: format!(
                "Created {} users and {} groups across {} tenants",
                populated.total_users(),
                populated.total_groups(),
                populated.tenants.len()
            ),
        })
    }

    async fn test_tenant_isolation(&self) -> Result<TestResult, Box<dyn std::error::Error>> {
        let start = std::time::Instant::now();

        // Populate provider with test data
        let populated = self.scenario.populate_provider(&*self.provider).await?;

        // Test that each tenant can only see its own resources
        for tenant_result in &populated.tenants {
            let context = self.create_context(&tenant_result.tenant_id);

            // List users for this tenant
            let users = self
                .provider
                .list_resources(&tenant_result.tenant_id, "User", None, &context)
                .await
                .map_err(|e| {
                    format!(
                        "Failed to list users for {}: {:?}",
                        tenant_result.tenant_id, e
                    )
                })?;

            // Should only see this tenant's users
            assert_eq!(users.len(), self.scenario.users_per_tenant);

            // Verify no cross-tenant access by checking user IDs
            for user in &users {
                let user_id = user.get_id().ok_or("User missing ID")?;

                // Try to access this user from other tenants
                for other_tenant in &populated.tenants {
                    if other_tenant.tenant_id != tenant_result.tenant_id {
                        let other_context = self.create_context(&other_tenant.tenant_id);
                        let cross_access = self
                            .provider
                            .get_resource(&other_tenant.tenant_id, "User", &user_id, &other_context)
                            .await
                            .map_err(|e| format!("Cross-tenant access test failed: {:?}", e))?;

                        assert!(
                            cross_access.is_none(),
                            "Tenant {} should not access user {} from tenant {}",
                            other_tenant.tenant_id,
                            user_id,
                            tenant_result.tenant_id
                        );
                    }
                }
            }
        }

        Ok(TestResult {
            duration: start.elapsed(),
            success: true,
            details: "All tenants properly isolated".to_string(),
        })
    }

    async fn test_concurrent_operations(&self) -> Result<TestResult, Box<dyn std::error::Error>> {
        let start = std::time::Instant::now();
        let mut handles = Vec::new();

        // Run concurrent operations across all tenants
        for tenant_data in &self.scenario.tenants {
            let provider_clone = self.provider.clone();
            let tenant_id = tenant_data.tenant_id.clone();
            let context = self.create_context(&tenant_id);

            let handle = tokio::spawn(async move {
                let mut operations = 0;

                // Perform multiple operations concurrently
                for i in 0..10 {
                    let username = format!("concurrent_user_{}_{}", tenant_id, i);
                    let user_data = TestUserData::new(&username).to_scim_json();

                    let result = provider_clone
                        .create_resource(&tenant_id, "User", user_data, &context)
                        .await;

                    match result {
                        Ok(_) => operations += 1,
                        Err(e) => return Err(format!("Concurrent operation failed: {:?}", e)),
                    }
                }

                Ok(operations)
            });

            handles.push(handle);
        }

        // Wait for all concurrent operations
        let mut total_operations = 0;
        for handle in handles {
            let operations = handle
                .await
                .map_err(|e| format!("Concurrent task failed: {}", e))??;
            total_operations += operations;
        }

        Ok(TestResult {
            duration: start.elapsed(),
            success: true,
            details: format!("Completed {} concurrent operations", total_operations),
        })
    }

    async fn test_error_handling(&self) -> Result<TestResult, Box<dyn std::error::Error>> {
        let start = std::time::Instant::now();
        let tenant_id = "error_test_tenant";
        let context = self.create_context(tenant_id);

        // Test getting non-existent resource
        let result = self
            .provider
            .get_resource(tenant_id, "User", "nonexistent_id", &context)
            .await;

        match result {
            Ok(None) => {} // Expected
            Ok(Some(_)) => return Err("Should not have found non-existent resource".into()),
            Err(e) => return Err(format!("Unexpected error: {:?}", e).into()),
        }

        // Test updating non-existent resource
        let update_result = self
            .provider
            .update_resource(tenant_id, "User", "nonexistent_id", json!({}), &context)
            .await;

        assert!(
            update_result.is_err(),
            "Update of non-existent resource should fail"
        );

        // Test deleting non-existent resource
        let delete_result = self
            .provider
            .delete_resource(tenant_id, "User", "nonexistent_id", &context)
            .await;

        assert!(
            delete_result.is_err(),
            "Delete of non-existent resource should fail"
        );

        Ok(TestResult {
            duration: start.elapsed(),
            success: true,
            details: "Error scenarios handled correctly".to_string(),
        })
    }

    async fn test_performance_characteristics(
        &self,
    ) -> Result<TestResult, Box<dyn std::error::Error>> {
        let start = std::time::Instant::now();
        let tenant_id = "performance_tenant";
        let context = self.create_context(tenant_id);

        // Test creation performance
        let create_start = std::time::Instant::now();
        let mut created_ids = Vec::new();

        for i in 0..100 {
            let username = format!("perf_user_{}", i);
            let user_data = TestUserData::new(&username).to_scim_json();

            let result = self
                .provider
                .create_resource(tenant_id, "User", user_data, &context)
                .await
                .map_err(|e| format!("Performance test create failed: {:?}", e))?;

            if let Some(id) = result.get_id() {
                created_ids.push(id.to_string());
            }
        }

        let create_duration = create_start.elapsed();

        // Test read performance
        let read_start = std::time::Instant::now();
        for id in &created_ids {
            let _result = self
                .provider
                .get_resource(tenant_id, "User", id, &context)
                .await
                .map_err(|e| format!("Performance test read failed: {:?}", e))?;
        }
        let read_duration = read_start.elapsed();

        // Test list performance
        let list_start = std::time::Instant::now();
        let _list_result = self
            .provider
            .list_resources(tenant_id, "User", None, &context)
            .await
            .map_err(|e| format!("Performance test list failed: {:?}", e))?;
        let list_duration = list_start.elapsed();

        let details = format!(
            "Created 100 users in {:?}, read in {:?}, listed in {:?}",
            create_duration, read_duration, list_duration
        );

        Ok(TestResult {
            duration: start.elapsed(),
            success: true,
            details,
        })
    }

    fn create_context(&self, tenant_id: &str) -> EnhancedRequestContext {
        let tenant_context = TenantContextBuilder::new(tenant_id).build();
        EnhancedRequestContext {
            request_id: format!("test_req_{}", tenant_id),
            tenant_context,
        }
    }
}

// ============================================================================
// Test Results and Reporting
// ============================================================================

/// Results from a single test
#[derive(Debug)]
pub struct TestResult {
    pub duration: std::time::Duration,
    pub success: bool,
    pub details: String,
}

/// Collection of test results
#[derive(Debug)]
pub struct TestResults {
    results: HashMap<String, TestResult>,
    total_duration: std::time::Duration,
}

impl TestResults {
    pub fn new() -> Self {
        Self {
            results: HashMap::new(),
            total_duration: std::time::Duration::from_secs(0),
        }
    }

    pub fn add_test_result(&mut self, test_name: &str, result: TestResult) {
        self.total_duration += result.duration;
        self.results.insert(test_name.to_string(), result);
    }

    pub fn all_passed(&self) -> bool {
        self.results.values().all(|r| r.success)
    }

    pub fn print_summary(&self) {
        println!("\n📊 Provider Test Results Summary");
        println!("================================");

        for (test_name, result) in &self.results {
            let status = if result.success {
                "✅ PASS"
            } else {
                "❌ FAIL"
            };
            println!("{} {} ({:?})", status, test_name, result.duration);
            println!("   {}", result.details);
        }

        println!("\nTotal Duration: {:?}", self.total_duration);
        println!(
            "Overall Result: {}",
            if self.all_passed() {
                "✅ ALL TESTS PASSED"
            } else {
                "❌ SOME TESTS FAILED"
            }
        );
    }
}

// ============================================================================
// Provider Configuration Testing
// ============================================================================

/// Utilities for testing provider configurations
pub struct ProviderConfigTester;

impl ProviderConfigTester {
    /// Test provider with different configuration scenarios
    pub async fn test_configuration_scenarios<P, F>(
        provider_factory: F,
        configs: Vec<ProviderTestConfig>,
    ) -> Result<(), Box<dyn std::error::Error>>
    where
        P: MultiTenantResourceProvider + 'static,
        P::Error: std::fmt::Debug,
        F: Fn(ProviderTestConfig) -> Result<P, Box<dyn std::error::Error>>,
    {
        for config in configs {
            println!("Testing configuration: {}", config.name);

            let provider = provider_factory(config.clone())?;
            let test_suite = ProviderTestingSuite::new(Arc::new(provider));

            let results = test_suite.run_comprehensive_tests().await?;

            if !results.all_passed() {
                return Err(format!("Configuration '{}' failed tests", config.name).into());
            }

            println!("Configuration '{}' passed all tests", config.name);
        }

        Ok(())
    }
}

/// Test configuration for a provider
#[derive(Debug, Clone)]
pub struct ProviderTestConfig {
    pub name: String,
    pub settings: HashMap<String, Value>,
}

impl ProviderTestConfig {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            settings: HashMap::new(),
        }
    }

    pub fn with_setting(mut self, key: &str, value: Value) -> Self {
        self.settings.insert(key.to_string(), value);
        self
    }
}
