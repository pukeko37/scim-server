//! Multi-Tenant Test Utilities
//!
//! This module provides common utilities, fixtures, and helper functions
//! specifically for testing multi-tenant functionality in the SCIM server.
//! These utilities are shared across all multi-tenant integration tests.

use serde_json::{Value, json};
use std::collections::HashMap;

// Re-export from integration tests for convenience
pub use crate::integration::multi_tenant::core::{
    AuthInfo, AuthInfoBuilder, EnhancedRequestContext, IsolationLevel, TenantContext,
    TenantContextBuilder, TenantPermissions,
};

/// Multi-tenant test context builder for creating realistic test scenarios
#[derive(Debug, Clone)]
pub struct MultiTenantTestContext {
    pub tenants: Vec<TenantTestSetup>,
    pub shared_resources: Vec<SharedResource>,
}

impl MultiTenantTestContext {
    pub fn new() -> Self {
        Self {
            tenants: Vec::new(),
            shared_resources: Vec::new(),
        }
    }

    pub fn add_tenant(mut self, setup: TenantTestSetup) -> Self {
        self.tenants.push(setup);
        self
    }

    pub fn add_shared_resource(mut self, resource: SharedResource) -> Self {
        self.shared_resources.push(resource);
        self
    }

    pub fn get_tenant(&self, tenant_id: &str) -> Option<&TenantTestSetup> {
        self.tenants.iter().find(|t| t.tenant_id == tenant_id)
    }
}

/// Setup configuration for a test tenant
#[derive(Debug, Clone)]
pub struct TenantTestSetup {
    pub tenant_id: String,
    pub display_name: String,
    pub isolation_level: IsolationLevel,
    pub max_users: Option<usize>,
    pub max_groups: Option<usize>,
    pub features: HashMap<String, bool>,
    pub test_users: Vec<TestUser>,
    pub test_groups: Vec<TestGroup>,
}

impl TenantTestSetup {
    pub fn new(tenant_id: &str, display_name: &str) -> Self {
        Self {
            tenant_id: tenant_id.to_string(),
            display_name: display_name.to_string(),
            isolation_level: IsolationLevel::Standard,
            max_users: None,
            max_groups: None,
            features: HashMap::new(),
            test_users: Vec::new(),
            test_groups: Vec::new(),
        }
    }

    pub fn with_isolation_level(mut self, level: IsolationLevel) -> Self {
        self.isolation_level = level;
        self
    }

    pub fn with_limits(mut self, max_users: Option<usize>, max_groups: Option<usize>) -> Self {
        self.max_users = max_users;
        self.max_groups = max_groups;
        self
    }

    pub fn with_feature(mut self, feature: &str, enabled: bool) -> Self {
        self.features.insert(feature.to_string(), enabled);
        self
    }

    pub fn add_test_user(mut self, user: TestUser) -> Self {
        self.test_users.push(user);
        self
    }

    pub fn add_test_group(mut self, group: TestGroup) -> Self {
        self.test_groups.push(group);
        self
    }

    pub fn to_tenant_context(&self) -> TenantContext {
        TenantContextBuilder::new(&self.tenant_id)
            .with_isolation_level(self.isolation_level.clone())
            .build()
    }

    pub fn to_request_context(&self) -> EnhancedRequestContext {
        EnhancedRequestContext {
            request_id: format!("test_req_{}", self.tenant_id),
            tenant_context: self.to_tenant_context(),
        }
    }
}

/// Test user data structure
#[derive(Debug, Clone)]
pub struct TestUser {
    pub username: String,
    pub email: String,
    pub given_name: String,
    pub family_name: String,
    pub display_name: String,
    pub active: bool,
    pub roles: Vec<String>,
    pub custom_attributes: HashMap<String, Value>,
}

impl TestUser {
    pub fn new(username: &str) -> Self {
        Self {
            username: username.to_string(),
            email: format!("{}@example.com", username),
            given_name: username.to_string(),
            family_name: "User".to_string(),
            display_name: format!("{} User", username),
            active: true,
            roles: Vec::new(),
            custom_attributes: HashMap::new(),
        }
    }

    pub fn with_email(mut self, email: &str) -> Self {
        self.email = email.to_string();
        self
    }

    pub fn with_name(mut self, given_name: &str, family_name: &str) -> Self {
        self.given_name = given_name.to_string();
        self.family_name = family_name.to_string();
        self.display_name = format!("{} {}", given_name, family_name);
        self
    }

    pub fn inactive(mut self) -> Self {
        self.active = false;
        self
    }

    pub fn with_role(mut self, role: &str) -> Self {
        self.roles.push(role.to_string());
        self
    }

    pub fn with_custom_attribute(mut self, key: &str, value: Value) -> Self {
        self.custom_attributes.insert(key.to_string(), value);
        self
    }

    pub fn to_scim_json(&self) -> Value {
        let mut user = json!({
            "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
            "userName": self.username,
            "emails": [{
                "value": self.email,
                "type": "work",
                "primary": true
            }],
            "name": {
                "givenName": self.given_name,
                "familyName": self.family_name,
                "formatted": format!("{} {}", self.given_name, self.family_name)
            },
            "displayName": self.display_name,
            "active": self.active
        });

        // Add roles if present
        if !self.roles.is_empty() {
            user["roles"] = json!(
                self.roles
                    .iter()
                    .map(|role| {
                        json!({
                            "value": role,
                            "type": "role"
                        })
                    })
                    .collect::<Vec<_>>()
            );
        }

        // Add custom attributes
        if let Some(obj) = user.as_object_mut() {
            for (key, value) in &self.custom_attributes {
                obj.insert(key.clone(), value.clone());
            }
        }

        user
    }
}

/// Test group data structure
#[derive(Debug, Clone)]
pub struct TestGroup {
    pub display_name: String,
    pub description: String,
    pub group_type: String,
    pub members: Vec<String>,
    pub custom_attributes: HashMap<String, Value>,
}

impl TestGroup {
    pub fn new(display_name: &str) -> Self {
        Self {
            display_name: display_name.to_string(),
            description: format!("{} group for testing", display_name),
            group_type: "security".to_string(),
            members: Vec::new(),
            custom_attributes: HashMap::new(),
        }
    }

    pub fn with_description(mut self, description: &str) -> Self {
        self.description = description.to_string();
        self
    }

    pub fn with_type(mut self, group_type: &str) -> Self {
        self.group_type = group_type.to_string();
        self
    }

    pub fn with_member(mut self, member_id: &str) -> Self {
        self.members.push(member_id.to_string());
        self
    }

    pub fn with_custom_attribute(mut self, key: &str, value: Value) -> Self {
        self.custom_attributes.insert(key.to_string(), value);
        self
    }

    pub fn to_scim_json(&self) -> Value {
        let members: Vec<Value> = self
            .members
            .iter()
            .map(|id| {
                json!({
                    "value": id,
                    "type": "User"
                })
            })
            .collect();

        let mut group = json!({
            "schemas": ["urn:ietf:params:scim:schemas:core:2.0:Group"],
            "displayName": self.display_name,
            "description": self.description,
            "members": members
        });

        // Add custom attributes
        if let Some(obj) = group.as_object_mut() {
            for (key, value) in &self.custom_attributes {
                obj.insert(key.clone(), value.clone());
            }
        }

        group
    }
}

/// Shared resource that might be accessed across tenants (with proper isolation)
#[derive(Debug, Clone)]
pub struct SharedResource {
    pub resource_type: String,
    pub resource_id: String,
    pub data: Value,
    pub access_tenants: Vec<String>,
}

impl SharedResource {
    pub fn new(resource_type: &str, resource_id: &str, data: Value) -> Self {
        Self {
            resource_type: resource_type.to_string(),
            resource_id: resource_id.to_string(),
            data,
            access_tenants: Vec::new(),
        }
    }

    pub fn with_access_for_tenant(mut self, tenant_id: &str) -> Self {
        self.access_tenants.push(tenant_id.to_string());
        self
    }

    pub fn can_access(&self, tenant_id: &str) -> bool {
        self.access_tenants.contains(&tenant_id.to_string())
    }
}

/// Predefined tenant scenarios for common testing patterns
pub struct TenantScenarios;

impl TenantScenarios {
    /// Basic multi-tenant scenario with two simple tenants
    pub fn basic_multi_tenant() -> MultiTenantTestContext {
        MultiTenantTestContext::new()
            .add_tenant(
                TenantTestSetup::new("tenant_a", "Tenant A Corporation")
                    .add_test_user(TestUser::new("alice").with_role("admin"))
                    .add_test_user(TestUser::new("bob").with_role("user"))
                    .add_test_group(TestGroup::new("Administrators")),
            )
            .add_tenant(
                TenantTestSetup::new("tenant_b", "Tenant B Inc")
                    .add_test_user(TestUser::new("charlie").with_role("manager"))
                    .add_test_user(TestUser::new("diana").with_role("user"))
                    .add_test_group(TestGroup::new("Managers")),
            )
    }

    /// Enterprise scenario with strict isolation and compliance requirements
    pub fn enterprise_compliance() -> MultiTenantTestContext {
        MultiTenantTestContext::new()
            .add_tenant(
                TenantTestSetup::new("enterprise_corp", "Enterprise Corporation")
                    .with_isolation_level(IsolationLevel::Strict)
                    .with_limits(Some(1000), Some(100))
                    .with_feature("audit_logging", true)
                    .with_feature("advanced_validation", true)
                    .add_test_user(
                        TestUser::new("enterprise.admin")
                            .with_email("admin@enterprise.com")
                            .with_name("Enterprise", "Admin")
                            .with_role("global_admin")
                            .with_custom_attribute("department", json!("IT"))
                            .with_custom_attribute("clearance_level", json!("high")),
                    )
                    .add_test_user(
                        TestUser::new("enterprise.user")
                            .with_email("user@enterprise.com")
                            .with_name("Enterprise", "User")
                            .with_role("employee")
                            .with_custom_attribute("department", json!("Sales")),
                    )
                    .add_test_group(
                        TestGroup::new("IT Department")
                            .with_description("Information Technology Department")
                            .with_custom_attribute("cost_center", json!("IT001")),
                    ),
            )
            .add_tenant(
                TenantTestSetup::new("startup_inc", "Startup Inc")
                    .with_isolation_level(IsolationLevel::Standard)
                    .with_limits(Some(50), Some(10))
                    .with_feature("audit_logging", false)
                    .add_test_user(
                        TestUser::new("founder")
                            .with_email("founder@startup.com")
                            .with_name("Startup", "Founder")
                            .with_role("owner"),
                    )
                    .add_test_user(
                        TestUser::new("employee1")
                            .with_email("emp1@startup.com")
                            .with_name("First", "Employee")
                            .with_role("developer"),
                    )
                    .add_test_group(TestGroup::new("All Hands")),
            )
    }

    /// Performance testing scenario with many tenants and resources
    pub fn performance_testing() -> MultiTenantTestContext {
        let mut context = MultiTenantTestContext::new();

        for i in 1..=10 {
            let tenant_id = format!("perf_tenant_{:02}", i);
            let mut tenant_setup =
                TenantTestSetup::new(&tenant_id, &format!("Performance Tenant {}", i))
                    .with_isolation_level(IsolationLevel::Standard)
                    .with_feature("bulk_operations", true);

            // Add multiple users per tenant
            for j in 1..=20 {
                let username = format!("user_{}_{:02}", i, j);
                tenant_setup = tenant_setup.add_test_user(
                    TestUser::new(&username)
                        .with_email(&format!("{}@perf{}.com", username, i))
                        .with_role("user"),
                );
            }

            // Add multiple groups per tenant
            for k in 1..=5 {
                let group_name = format!("Group {} {}", i, k);
                tenant_setup =
                    tenant_setup.add_test_group(TestGroup::new(&group_name).with_description(
                        &format!("Performance test group {} for tenant {}", k, i),
                    ));
            }

            context = context.add_tenant(tenant_setup);
        }

        context
    }

    /// Security testing scenario focusing on isolation verification
    pub fn security_isolation() -> MultiTenantTestContext {
        MultiTenantTestContext::new()
            .add_tenant(
                TenantTestSetup::new("secure_tenant", "Secure Tenant")
                    .with_isolation_level(IsolationLevel::Strict)
                    .with_feature("encryption_at_rest", true)
                    .with_feature("audit_logging", true)
                    .add_test_user(
                        TestUser::new("secure.user")
                            .with_email("secure@secure.com")
                            .with_custom_attribute("security_clearance", json!("secret")),
                    )
                    .add_test_group(
                        TestGroup::new("Security Group")
                            .with_custom_attribute("classification", json!("restricted")),
                    ),
            )
            .add_tenant(
                TenantTestSetup::new("public_tenant", "Public Tenant")
                    .with_isolation_level(IsolationLevel::Standard)
                    .add_test_user(
                        TestUser::new("public.user")
                            .with_email("public@public.com")
                            .with_custom_attribute("security_clearance", json!("public")),
                    )
                    .add_test_group(
                        TestGroup::new("Public Group")
                            .with_custom_attribute("classification", json!("public")),
                    ),
            )
    }
}

/// Utility functions for multi-tenant testing
pub struct MultiTenantTestUtils;

impl MultiTenantTestUtils {
    /// Create an authentication info for a specific tenant
    pub fn create_auth_for_tenant(tenant_id: &str) -> AuthInfo {
        AuthInfoBuilder::new()
            .with_api_key(&format!("api_key_{}", tenant_id))
            .build()
    }

    /// Create multiple request contexts for different tenants
    pub fn create_contexts_for_tenants(
        tenant_ids: &[&str],
    ) -> HashMap<String, EnhancedRequestContext> {
        tenant_ids
            .iter()
            .map(|&tenant_id| {
                let context = TenantContextBuilder::new(tenant_id).build();
                let request_context = EnhancedRequestContext {
                    request_id: format!("test_req_{}", tenant_id),
                    tenant_context: context,
                };
                (tenant_id.to_string(), request_context)
            })
            .collect()
    }

    /// Verify that two resources belong to different tenants
    pub fn assert_cross_tenant_isolation(
        resource1: &Value,
        tenant1: &str,
        resource2: &Value,
        tenant2: &str,
    ) {
        // This would be implemented based on how tenant information is stored in resources
        // For now, we can verify they have different IDs
        let id1 = resource1.get("id").and_then(|v| v.as_str());
        let id2 = resource2.get("id").and_then(|v| v.as_str());

        assert_ne!(
            id1, id2,
            "Resources from different tenants should have different IDs"
        );
        assert_ne!(tenant1, tenant2, "Tenants should be different");
    }

    /// Generate test data for bulk operations
    pub fn generate_bulk_test_data(
        tenant_id: &str,
        resource_type: &str,
        count: usize,
    ) -> Vec<Value> {
        (0..count)
            .map(|i| match resource_type {
                "User" => TestUser::new(&format!("bulk_user_{}_{}", tenant_id, i))
                    .with_email(&format!("bulk{}@{}.com", i, tenant_id))
                    .to_scim_json(),
                "Group" => TestGroup::new(&format!("Bulk Group {} {}", tenant_id, i))
                    .with_description(&format!("Bulk test group {} for {}", i, tenant_id))
                    .to_scim_json(),
                _ => json!({
                    "schemas": [format!("urn:ietf:params:scim:schemas:core:2.0:{}", resource_type)],
                    "displayName": format!("Bulk {} {} {}", resource_type, tenant_id, i)
                }),
            })
            .collect()
    }

    /// Verify tenant isolation across a list of resources
    pub fn verify_tenant_resource_isolation(
        resources: &[Value],
        _expected_tenant_markers: &HashMap<String, String>,
    ) {
        for resource in resources {
            // This would need to be implemented based on how tenant information
            // is stored or tracked in resources. For now, we can verify basic structure.
            assert!(resource.get("id").is_some(), "Resource should have an ID");
            assert!(
                resource.get("schemas").is_some(),
                "Resource should have schemas"
            );
        }
    }

    /// Create a complete test environment with multiple tenants
    pub fn setup_test_environment() -> MultiTenantTestContext {
        TenantScenarios::basic_multi_tenant()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tenant_setup_creation() {
        let setup = TenantTestSetup::new("test_tenant", "Test Tenant")
            .with_isolation_level(IsolationLevel::Strict)
            .with_limits(Some(100), Some(10))
            .with_feature("test_feature", true);

        assert_eq!(setup.tenant_id, "test_tenant");
        assert_eq!(setup.display_name, "Test Tenant");
        assert_eq!(setup.isolation_level, IsolationLevel::Strict);
        assert_eq!(setup.max_users, Some(100));
        assert_eq!(setup.max_groups, Some(10));
        assert_eq!(setup.features.get("test_feature"), Some(&true));
    }

    #[test]
    fn test_test_user_creation() {
        let user = TestUser::new("testuser")
            .with_email("test@example.com")
            .with_name("Test", "User")
            .with_role("admin")
            .with_custom_attribute("department", json!("IT"));

        assert_eq!(user.username, "testuser");
        assert_eq!(user.email, "test@example.com");
        assert_eq!(user.given_name, "Test");
        assert_eq!(user.family_name, "User");
        assert!(user.roles.contains(&"admin".to_string()));

        let scim_json = user.to_scim_json();
        assert_eq!(scim_json["userName"], "testuser");
        assert_eq!(scim_json["emails"][0]["value"], "test@example.com");
        assert_eq!(scim_json["department"], json!("IT"));
    }

    #[test]
    fn test_test_group_creation() {
        let group = TestGroup::new("Test Group")
            .with_description("A test group")
            .with_member("user123")
            .with_custom_attribute("cost_center", json!("CC001"));

        assert_eq!(group.display_name, "Test Group");
        assert_eq!(group.description, "A test group");
        assert!(group.members.contains(&"user123".to_string()));

        let scim_json = group.to_scim_json();
        assert_eq!(scim_json["displayName"], "Test Group");
        assert_eq!(scim_json["members"][0]["value"], "user123");
        assert_eq!(scim_json["cost_center"], json!("CC001"));
    }

    #[test]
    fn test_tenant_scenarios() {
        let basic = TenantScenarios::basic_multi_tenant();
        assert_eq!(basic.tenants.len(), 2);
        assert!(basic.get_tenant("tenant_a").is_some());
        assert!(basic.get_tenant("tenant_b").is_some());

        let enterprise = TenantScenarios::enterprise_compliance();
        assert_eq!(enterprise.tenants.len(), 2);

        let enterprise_tenant = enterprise.get_tenant("enterprise_corp").unwrap();
        assert_eq!(enterprise_tenant.isolation_level, IsolationLevel::Strict);
        assert_eq!(enterprise_tenant.features.get("audit_logging"), Some(&true));
    }

    #[test]
    fn test_multi_tenant_test_utils() {
        let auth = MultiTenantTestUtils::create_auth_for_tenant("test_tenant");
        assert_eq!(auth.api_key, Some("api_key_test_tenant".to_string()));

        let contexts = MultiTenantTestUtils::create_contexts_for_tenants(&["tenant1", "tenant2"]);
        assert_eq!(contexts.len(), 2);
        assert!(contexts.contains_key("tenant1"));
        assert!(contexts.contains_key("tenant2"));

        let bulk_data = MultiTenantTestUtils::generate_bulk_test_data("test", "User", 3);
        assert_eq!(bulk_data.len(), 3);
        assert_eq!(bulk_data[0]["userName"], "bulk_user_test_0");
    }
}
