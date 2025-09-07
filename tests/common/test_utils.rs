//! Shared test utilities for unified SCIM provider testing.
//!
//! This module provides common utilities, fixtures, and helper functions
//! that are used across all SCIM integration tests. It supports both single-tenant
//! and multi-tenant scenarios using the unified ResourceProvider interface.

use scim_server::ResourceProvider;
use scim_server::resource::{RequestContext, Resource, TenantContext};
use serde_json::{Value, json};
use std::collections::HashMap;
use std::sync::Arc;

// Re-export key types for convenience
pub use scim_server::resource::{IsolationLevel, ListQuery, TenantPermissions};

/// Creates a test context for single-tenant scenarios
pub fn create_single_tenant_context() -> RequestContext {
    RequestContext::with_generated_id()
}

/// Creates a test context for a given tenant ID
pub fn create_multi_tenant_context(tenant_id: &str) -> RequestContext {
    let tenant_context = TenantContext::new(tenant_id.to_string(), format!("{}_client", tenant_id));
    RequestContext::with_tenant_generated_id(tenant_context)
}

/// Creates a test user JSON object with the given username
pub fn create_test_user(username: &str) -> Value {
    json!({
        "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
        "userName": username,
        "active": true,
        "emails": [{
            "value": format!("{}@example.com", username),
            "type": "work",
            "primary": true
        }],
        "name": {
            "formatted": format!("Test User {}", username),
            "familyName": "User",
            "givenName": "Test"
        },
        "displayName": format!("Test User {}", username)
    })
}

/// Creates a test group JSON object with the given display name
pub fn create_test_group(display_name: &str) -> Value {
    json!({
        "schemas": ["urn:ietf:params:scim:schemas:core:2.0:Group"],
        "displayName": display_name,
        "members": []
    })
}

/// Creates multiple test contexts for different tenants
pub fn create_multi_tenant_contexts(tenant_ids: &[&str]) -> HashMap<String, RequestContext> {
    tenant_ids
        .iter()
        .map(|&tenant_id| {
            (
                tenant_id.to_string(),
                create_multi_tenant_context(tenant_id),
            )
        })
        .collect()
}

/// Test harness for unified provider testing (supports both single and multi-tenant)
pub struct UnifiedTestHarness<P> {
    pub provider: Arc<P>,
    pub contexts: HashMap<String, RequestContext>,
}

impl<P> UnifiedTestHarness<P>
where
    P: ResourceProvider + Send + Sync + 'static,
    P::Error: std::error::Error + Send + Sync + 'static,
{
    /// Creates a new test harness for single-tenant testing
    pub fn new_single_tenant(provider: P) -> Self {
        let mut contexts = HashMap::new();
        contexts.insert("default".to_string(), create_single_tenant_context());

        Self {
            provider: Arc::new(provider),
            contexts,
        }
    }

    /// Creates a new test harness for multi-tenant testing with specified tenant IDs
    pub fn new_multi_tenant(provider: P, tenant_ids: &[&str]) -> Self {
        let contexts = create_multi_tenant_contexts(tenant_ids);

        Self {
            provider: Arc::new(provider),
            contexts,
        }
    }

    /// Creates a new test harness for single-tenant testing from Arc<P>
    pub fn from_arc_single_tenant(provider: Arc<P>) -> Self {
        let mut contexts = HashMap::new();
        contexts.insert("default".to_string(), create_single_tenant_context());

        Self { provider, contexts }
    }

    /// Creates a new test harness for multi-tenant testing from Arc<P>
    pub fn from_arc_multi_tenant(provider: Arc<P>, tenant_ids: &[&str]) -> Self {
        let contexts = create_multi_tenant_contexts(tenant_ids);

        Self { provider, contexts }
    }

    /// Gets the context for a specific tenant (or "default" for single-tenant)
    pub fn context(&self, tenant_id: &str) -> &RequestContext {
        self.contexts
            .get(tenant_id)
            .unwrap_or_else(|| panic!("No context found for tenant: {}", tenant_id))
    }

    /// Gets the default context (useful for single-tenant scenarios)
    pub fn default_context(&self) -> &RequestContext {
        self.context("default")
    }

    /// Creates a user in the specified tenant (or default context)
    pub async fn create_user(
        &self,
        tenant_id: Option<&str>,
        username: &str,
    ) -> Result<Resource, Box<dyn std::error::Error + Send + Sync>> {
        let user_data = create_test_user(username);
        let context = match tenant_id {
            Some(id) => self.context(id),
            None => self.default_context(),
        };

        let resource = self
            .provider
            .create_resource("User", user_data, context)
            .await
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?;

        Ok(resource.into_resource())
    }

    /// Creates a group in the specified tenant (or default context)
    pub async fn create_group(
        &self,
        tenant_id: Option<&str>,
        display_name: &str,
    ) -> Result<Resource, Box<dyn std::error::Error + Send + Sync>> {
        let group_data = create_test_group(display_name);
        let context = match tenant_id {
            Some(id) => self.context(id),
            None => self.default_context(),
        };

        let resource = self
            .provider
            .create_resource("Group", group_data, context)
            .await
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?;

        Ok(resource.into_resource())
    }

    /// Verifies that a resource exists in the specified tenant but not in others (multi-tenant only)
    pub async fn verify_tenant_isolation(
        &self,
        owner_tenant_id: &str,
        resource_type: &str,
        resource: &Resource,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let resource_id = resource
            .id
            .as_ref()
            .ok_or("Resource must have an ID for isolation testing")?
            .as_str();

        let owner_context = self.context(owner_tenant_id);

        // Resource should exist in the correct tenant
        let exists = self
            .provider
            .resource_exists(resource_type, resource_id, owner_context)
            .await
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?;

        if !exists {
            return Err(format!("Resource should exist in tenant {}", owner_tenant_id).into());
        }

        // Resource should not be accessible from other tenant contexts
        for (other_tenant_id, other_context) in &self.contexts {
            if other_tenant_id != owner_tenant_id && other_tenant_id != "default" {
                let result = self
                    .provider
                    .get_resource(resource_type, resource_id, other_context)
                    .await
                    .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?;

                if result.is_some() {
                    return Err(format!(
                        "Resource should not be accessible from tenant {}",
                        other_tenant_id
                    )
                    .into());
                }
            }
        }

        Ok(())
    }

    /// Lists all resources of a given type in a specific tenant
    pub async fn list_resources(
        &self,
        tenant_id: Option<&str>,
        resource_type: &str,
    ) -> Result<Vec<Resource>, Box<dyn std::error::Error + Send + Sync>> {
        let context = match tenant_id {
            Some(id) => self.context(id),
            None => self.default_context(),
        };

        let resources = self
            .provider
            .list_resources(resource_type, None, context)
            .await
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?;

        Ok(resources.into_iter().map(|r| r.into_resource()).collect())
    }
}

/// Common test scenarios for unified provider testing
pub struct TestScenarios;

impl TestScenarios {
    /// Creates a basic single-tenant test scenario
    pub fn single_tenant<P>(provider: P) -> UnifiedTestHarness<P>
    where
        P: ResourceProvider + Send + Sync + 'static,
        P::Error: std::error::Error + Send + Sync + 'static,
    {
        UnifiedTestHarness::new_single_tenant(provider)
    }

    /// Creates a basic two-tenant test scenario
    pub fn basic_two_tenant<P>(provider: P) -> UnifiedTestHarness<P>
    where
        P: ResourceProvider + Send + Sync + 'static,
        P::Error: std::error::Error + Send + Sync + 'static,
    {
        UnifiedTestHarness::new_multi_tenant(provider, &["tenant_a", "tenant_b"])
    }

    /// Creates a complex multi-tenant test scenario
    pub fn complex_multi_tenant<P>(provider: P) -> UnifiedTestHarness<P>
    where
        P: ResourceProvider + Send + Sync + 'static,
        P::Error: std::error::Error + Send + Sync + 'static,
    {
        UnifiedTestHarness::new_multi_tenant(
            provider,
            &["corp_a", "corp_b", "corp_c", "test_tenant"],
        )
    }

    /// Creates a high-security isolation test scenario
    pub fn high_security_isolation<P>(provider: P) -> UnifiedTestHarness<P>
    where
        P: ResourceProvider + Send + Sync + 'static,
        P::Error: std::error::Error + Send + Sync + 'static,
    {
        UnifiedTestHarness::new_multi_tenant(provider, &["secure_tenant", "standard_tenant"])
    }
}

/// Assertion helpers for unified provider tests
pub mod assertions {
    use super::*;

    /// Asserts that cross-tenant access is properly denied
    pub async fn assert_cross_tenant_isolation<P>(
        provider: &P,
        resource_type: &str,
        resource: &Resource,
        owner_context: &RequestContext,
        other_context: &RequestContext,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>>
    where
        P: ResourceProvider,
        P::Error: std::error::Error + Send + Sync + 'static,
    {
        let resource_id = resource
            .id
            .as_ref()
            .ok_or("Resource must have an ID for isolation testing")?
            .as_str();

        // Resource should be accessible from owner context
        let owner_result = provider
            .get_resource(resource_type, resource_id, owner_context)
            .await
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?;

        if owner_result.is_none() {
            return Err("Resource should be accessible from owner context".into());
        }

        // Resource should NOT be accessible from other context
        let other_result = provider
            .get_resource(resource_type, resource_id, other_context)
            .await
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?;

        if other_result.is_some() {
            return Err("Cross-tenant access should be denied but resource was found".into());
        }

        Ok(())
    }

    /// Asserts that tenant-specific operations work correctly
    pub async fn assert_tenant_scoped_operations<P>(
        provider: &P,
        context: &RequestContext,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>>
    where
        P: ResourceProvider,
        P::Error: std::error::Error + Send + Sync + 'static,
    {
        let user_data = create_test_user("test_user");

        // Create should work
        let resource = provider
            .create_resource("User", user_data, context)
            .await
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?;

        let resource_id = resource
            .resource()
            .get_id()
            .ok_or("Resource should have an ID")?;

        // Get should work
        let retrieved = provider
            .get_resource("User", resource_id, context)
            .await
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?
            .ok_or("Resource should be found")?;

        if retrieved.get_id() != resource.get_id() {
            return Err("Retrieved resource ID should match created resource ID".into());
        }

        // List should include the resource
        let resources = provider
            .list_resources("User", None, context)
            .await
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?;

        let found = resources.iter().any(|r| r.get_id() == resource.get_id());
        if !found {
            return Err("List should include the created resource".into());
        }

        // Delete should work
        provider
            .delete_resource("User", resource_id, None, context)
            .await
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?;

        // Resource should no longer exist
        let exists = provider
            .resource_exists("User", resource_id, context)
            .await
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?;

        if exists {
            return Err("Resource should no longer exist after deletion".into());
        }

        Ok(())
    }

    /// Asserts that resources have proper validated fields
    pub fn assert_resource_validation(
        resource: &Resource,
        resource_type: &str,
    ) -> Result<(), String> {
        // Check resource type
        if resource.resource_type != resource_type {
            return Err(format!(
                "Expected resource type '{}', got '{}'",
                resource_type, resource.resource_type
            ));
        }

        // Check that ID is present and valid
        if resource.id.is_none() {
            return Err("Resource should have an ID".to_string());
        }

        // Check schemas
        if resource.schemas.is_empty() {
            return Err("Resource should have at least one schema".to_string());
        }

        // Type-specific validations
        match resource_type {
            "User" => {
                if resource.user_name.is_none() {
                    return Err("User resource should have a username".to_string());
                }
            }
            "Group" => {
                if !resource.attributes.contains_key("displayName") {
                    return Err("Group resource should have a displayName".to_string());
                }
            }
            _ => {}
        }

        Ok(())
    }
}

/// Performance testing utilities
pub mod performance {
    use super::*;
    use std::time::{Duration, Instant};

    /// Measures the performance of concurrent operations across multiple tenants
    pub async fn measure_concurrent_operations<P>(
        provider: Arc<P>,
        contexts: &HashMap<String, RequestContext>,
        operations_per_tenant: usize,
    ) -> Duration
    where
        P: ResourceProvider + Send + Sync + 'static,
        P::Error: std::error::Error + Send + Sync + 'static,
    {
        let start = Instant::now();
        let mut handles = vec![];

        for (tenant_id, context) in contexts {
            let provider_clone = Arc::clone(&provider);
            let context_clone = context.clone();
            let tenant_id_clone = tenant_id.clone();

            let handle = tokio::spawn(async move {
                for i in 0..operations_per_tenant {
                    let username = format!("perf_user_{}_{}", tenant_id_clone, i);
                    let user_data = create_test_user(&username);

                    let _ = provider_clone
                        .create_resource("User", user_data, &context_clone)
                        .await;
                }
            });

            handles.push(handle);
        }

        // Wait for all operations to complete
        for handle in handles {
            let _ = handle.await;
        }

        start.elapsed()
    }

    /// Verifies that performance doesn't degrade significantly with multiple tenants
    pub async fn verify_scaling_performance<P>(
        provider_factory: impl Fn() -> P,
        max_tenants: usize,
        operations_per_tenant: usize,
    ) -> Vec<(usize, Duration)>
    where
        P: ResourceProvider + Send + Sync + 'static,
        P::Error: std::error::Error + Send + Sync + 'static,
    {
        let mut results = vec![];

        for num_tenants in (1..=max_tenants).step_by(std::cmp::max(1, max_tenants / 5)) {
            let provider = Arc::new(provider_factory());
            let tenant_ids: Vec<String> = (0..num_tenants)
                .map(|i| format!("perf_tenant_{}", i))
                .collect();

            let tenant_refs: Vec<&str> = tenant_ids.iter().map(String::as_str).collect();
            let contexts = create_multi_tenant_contexts(&tenant_refs);

            let duration =
                measure_concurrent_operations(provider, &contexts, operations_per_tenant).await;
            results.push((num_tenants, duration));
        }

        results
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_single_tenant_context() {
        let context = create_single_tenant_context();
        assert!(context.tenant_id().is_none());
    }

    #[test]
    fn test_create_multi_tenant_context() {
        let context = create_multi_tenant_context("test_tenant");
        assert_eq!(context.tenant_id(), Some("test_tenant"));
    }

    #[test]
    fn test_create_test_user() {
        let user = create_test_user("testuser");
        assert_eq!(user["userName"], "testuser");
        assert_eq!(user["emails"][0]["value"], "testuser@example.com");
    }

    #[test]
    fn test_create_test_group() {
        let group = create_test_group("Test Group");
        assert_eq!(group["displayName"], "Test Group");
        assert!(group["members"].is_array());
    }

    #[test]
    fn test_multi_tenant_contexts() {
        let contexts = create_multi_tenant_contexts(&["tenant1", "tenant2"]);
        assert_eq!(contexts.len(), 2);
        assert!(contexts.contains_key("tenant1"));
        assert!(contexts.contains_key("tenant2"));
    }

    #[test]
    fn test_resource_validation() {
        use scim_server::resource::builder::ResourceBuilder;
        use scim_server::resource::value_objects::{ResourceId, UserName};

        let resource = ResourceBuilder::new("User".to_string())
            .with_id(ResourceId::new("test-123".to_string()).unwrap())
            .with_username(UserName::new("testuser".to_string()).unwrap())
            .build()
            .unwrap();

        assert!(assertions::assert_resource_validation(&resource, "User").is_ok());
    }
}
