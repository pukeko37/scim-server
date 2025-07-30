//! Phase 1 Integration Tests
//!
//! This test suite validates the successful integration of multi-tenant functionality
//! into the main SCIM server application. It tests the complete flow from tenant
//! resolution through resource operations to ensure all components work together.

use scim_server::{
    DatabaseResourceProvider, EnhancedRequestContext, IsolationLevel, MultiTenantResourceProvider,
    RequestContext, Resource, ResourceProvider, SingleTenantAdapter, StaticTenantResolver,
    TenantContext, TenantPermissions, TenantResolver,
};
use serde_json::{Value, json};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Mock single-tenant provider for testing adapter functionality
struct MockSingleTenantProvider {
    resources: Arc<RwLock<HashMap<String, HashMap<String, Resource>>>>,
}

impl MockSingleTenantProvider {
    fn new() -> Self {
        Self {
            resources: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

#[derive(Debug, thiserror::Error)]
#[error("Mock provider error")]
struct MockError;

impl ResourceProvider for MockSingleTenantProvider {
    type Error = MockError;

    async fn create_resource(
        &self,
        resource_type: &str,
        mut data: Value,
        context: &RequestContext,
    ) -> Result<Resource, Self::Error> {
        // Generate ID if not provided
        let id = if data.get("id").is_none() {
            let generated_id = uuid::Uuid::new_v4().to_string();
            data.as_object_mut()
                .unwrap()
                .insert("id".to_string(), Value::String(generated_id.clone()));
            generated_id
        } else {
            data.get("id").unwrap().as_str().unwrap().to_string()
        };

        // Add tenant information from context if available
        if let Some(tenant_id) = context.tenant_id() {
            data.as_object_mut().unwrap().insert(
                "tenant_id".to_string(),
                Value::String(tenant_id.to_string()),
            );
        }

        let resource = Resource::new(resource_type.to_string(), data);

        let mut resources = self.resources.write().await;
        resources
            .entry(resource_type.to_string())
            .or_insert_with(HashMap::new)
            .insert(id, resource.clone());

        Ok(resource)
    }

    async fn get_resource(
        &self,
        resource_type: &str,
        id: &str,
        _context: &RequestContext,
    ) -> Result<Option<Resource>, Self::Error> {
        let resources = self.resources.read().await;
        Ok(resources
            .get(resource_type)
            .and_then(|type_resources| type_resources.get(id))
            .cloned())
    }

    async fn update_resource(
        &self,
        resource_type: &str,
        id: &str,
        data: Value,
        _context: &RequestContext,
    ) -> Result<Resource, Self::Error> {
        let resource = Resource::new(resource_type.to_string(), data);
        let mut resources = self.resources.write().await;
        resources
            .entry(resource_type.to_string())
            .or_insert_with(HashMap::new)
            .insert(id.to_string(), resource.clone());
        Ok(resource)
    }

    async fn delete_resource(
        &self,
        resource_type: &str,
        id: &str,
        _context: &RequestContext,
    ) -> Result<(), Self::Error> {
        let mut resources = self.resources.write().await;
        if let Some(type_resources) = resources.get_mut(resource_type) {
            type_resources.remove(id);
        }
        Ok(())
    }

    async fn list_resources(
        &self,
        resource_type: &str,
        _query: Option<&scim_server::ListQuery>,
        _context: &RequestContext,
    ) -> Result<Vec<Resource>, Self::Error> {
        let resources = self.resources.read().await;
        Ok(resources
            .get(resource_type)
            .map(|type_resources| type_resources.values().cloned().collect())
            .unwrap_or_default())
    }

    async fn find_resource_by_attribute(
        &self,
        resource_type: &str,
        attribute: &str,
        value: &Value,
        _context: &RequestContext,
    ) -> Result<Option<Resource>, Self::Error> {
        let resources = self.resources.read().await;
        Ok(resources
            .get(resource_type)
            .and_then(|type_resources| {
                type_resources
                    .values()
                    .find(|resource| resource.get_attribute(attribute) == Some(value))
            })
            .cloned())
    }

    async fn resource_exists(
        &self,
        resource_type: &str,
        id: &str,
        _context: &RequestContext,
    ) -> Result<bool, Self::Error> {
        let resources = self.resources.read().await;
        Ok(resources
            .get(resource_type)
            .map(|type_resources| type_resources.contains_key(id))
            .unwrap_or(false))
    }
}

/// Test basic multi-tenant context creation and usage
#[tokio::test]
async fn test_enhanced_request_context_creation() {
    let tenant_context = TenantContext::new("test-tenant".to_string(), "test-client".to_string());
    let enhanced_context = EnhancedRequestContext::with_generated_id(tenant_context.clone());

    assert_eq!(enhanced_context.tenant_id(), "test-tenant");
    assert_eq!(enhanced_context.client_id(), "test-client");
    assert!(!enhanced_context.request_id.is_empty());

    // Test conversion to regular context
    let regular_context = enhanced_context.to_request_context();
    assert!(regular_context.is_multi_tenant());
    assert_eq!(regular_context.tenant_id(), Some("test-tenant"));
}

/// Test tenant resolver functionality
#[tokio::test]
async fn test_tenant_resolver() {
    let resolver = StaticTenantResolver::new();

    let tenant_a = TenantContext::new("tenant-a".to_string(), "client-a".to_string())
        .with_isolation_level(IsolationLevel::Strict);
    let tenant_b = TenantContext::new("tenant-b".to_string(), "client-b".to_string());

    resolver.add_tenant("api-key-a", tenant_a.clone()).await;
    resolver.add_tenant("api-key-b", tenant_b.clone()).await;

    // Test successful resolution
    let resolved_a = resolver.resolve_tenant("api-key-a").await.unwrap();
    assert_eq!(resolved_a.tenant_id, "tenant-a");
    assert_eq!(resolved_a.isolation_level, IsolationLevel::Strict);

    let resolved_b = resolver.resolve_tenant("api-key-b").await.unwrap();
    assert_eq!(resolved_b.tenant_id, "tenant-b");
    assert_eq!(resolved_b.isolation_level, IsolationLevel::Standard);

    // Test invalid credential
    let invalid_result = resolver.resolve_tenant("invalid-key").await;
    assert!(invalid_result.is_err());

    // Test tenant validation
    assert!(resolver.validate_tenant("tenant-a").await.unwrap());
    assert!(!resolver.validate_tenant("nonexistent").await.unwrap());
}

/// Test single-tenant adapter with multi-tenant context
#[tokio::test]
async fn test_single_tenant_adapter() {
    let mock_provider = Arc::new(MockSingleTenantProvider::new());
    let adapter = SingleTenantAdapter::new(mock_provider);

    let tenant_context = TenantContext::new("adapter-test".to_string(), "client".to_string());
    let context = EnhancedRequestContext::with_generated_id(tenant_context);

    // Test create operation
    let user_data = json!({
        "userName": "testuser",
        "displayName": "Test User"
    });

    let created_user = adapter
        .create_resource("adapter-test", "User", user_data, &context)
        .await
        .unwrap();

    assert_eq!(created_user.get_username(), Some("testuser"));
    assert!(created_user.get_id().is_some());

    // Test get operation
    let user_id = created_user.get_id().unwrap();
    let retrieved_user = adapter
        .get_resource("adapter-test", "User", user_id, &context)
        .await
        .unwrap();

    assert!(retrieved_user.is_some());
    let retrieved_user = retrieved_user.unwrap();
    assert_eq!(retrieved_user.get_username(), Some("testuser"));

    // Test tenant validation - should fail with wrong tenant
    let wrong_tenant_context = TenantContext::new("wrong-tenant".to_string(), "client".to_string());
    let wrong_context = EnhancedRequestContext::with_generated_id(wrong_tenant_context);

    let result = adapter
        .get_resource("adapter-test", "User", user_id, &wrong_context)
        .await;
    assert!(result.is_err());
}

/// Test database-backed multi-tenant provider
#[tokio::test]
async fn test_database_multi_tenant_provider() {
    let provider = DatabaseResourceProvider::new_in_memory()
        .await
        .expect("Failed to create database provider");

    let tenant_context = TenantContext::new("db-test".to_string(), "client".to_string());
    let context = EnhancedRequestContext::with_generated_id(tenant_context);

    // Test create user
    let user_data = json!({
        "userName": "dbuser",
        "displayName": "Database User",
        "emails": [{"value": "db@example.com", "primary": true}]
    });

    let created_user = provider
        .create_resource("db-test", "User", user_data, &context)
        .await
        .unwrap();

    assert_eq!(created_user.get_username(), Some("dbuser"));
    assert!(created_user.get_id().is_some());

    // Test resource count
    let count = provider
        .get_resource_count("db-test", "User", &context)
        .await
        .unwrap();
    assert_eq!(count, 1);

    // Test list resources
    let users = provider
        .list_resources("db-test", "User", None, &context)
        .await
        .unwrap();
    assert_eq!(users.len(), 1);
    assert_eq!(users[0].get_username(), Some("dbuser"));

    // Test update
    let user_id = created_user.get_id().unwrap();
    let updated_data = json!({
        "id": user_id,
        "userName": "dbuser",
        "displayName": "Updated Database User"
    });

    let updated_user = provider
        .update_resource("db-test", "User", user_id, updated_data, &context)
        .await
        .unwrap();
    assert_eq!(
        updated_user.get_attribute("displayName"),
        Some(&json!("Updated Database User"))
    );

    // Test delete
    provider
        .delete_resource("db-test", "User", user_id, &context)
        .await
        .unwrap();

    let deleted_user = provider
        .get_resource("db-test", "User", user_id, &context)
        .await
        .unwrap();
    assert!(deleted_user.is_none());
}

/// Test multi-tenant isolation between tenants
#[tokio::test]
async fn test_multi_tenant_isolation() {
    let provider = DatabaseResourceProvider::new_in_memory()
        .await
        .expect("Failed to create database provider");

    // Set up two different tenants
    let tenant_a_context = TenantContext::new("tenant-a".to_string(), "client-a".to_string());
    let context_a = EnhancedRequestContext::with_generated_id(tenant_a_context);

    let tenant_b_context = TenantContext::new("tenant-b".to_string(), "client-b".to_string());
    let context_b = EnhancedRequestContext::with_generated_id(tenant_b_context);

    // Create users in both tenants with same username
    let user_data_a = json!({"id": "user1", "userName": "john", "displayName": "John from A"});
    let user_data_b = json!({"id": "user1", "userName": "john", "displayName": "John from B"});

    provider
        .create_resource("tenant-a", "User", user_data_a, &context_a)
        .await
        .unwrap();

    provider
        .create_resource("tenant-b", "User", user_data_b, &context_b)
        .await
        .unwrap();

    // Each tenant should only see their own data
    let user_a = provider
        .get_resource("tenant-a", "User", "user1", &context_a)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        user_a.get_attribute("displayName"),
        Some(&json!("John from A"))
    );

    let user_b = provider
        .get_resource("tenant-b", "User", "user1", &context_b)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        user_b.get_attribute("displayName"),
        Some(&json!("John from B"))
    );

    // Cross-tenant access should fail
    let cross_access_result = provider
        .get_resource("tenant-b", "User", "user1", &context_a)
        .await;
    assert!(cross_access_result.is_err());

    // Resource counts should be isolated
    let count_a = provider
        .get_resource_count("tenant-a", "User", &context_a)
        .await
        .unwrap();
    let count_b = provider
        .get_resource_count("tenant-b", "User", &context_b)
        .await
        .unwrap();
    assert_eq!(count_a, 1);
    assert_eq!(count_b, 1);
}

/// Test tenant permissions and limits
#[tokio::test]
async fn test_tenant_permissions_and_limits() {
    let provider = DatabaseResourceProvider::new_in_memory()
        .await
        .expect("Failed to create database provider");

    // Create tenant with restricted permissions
    let mut permissions = TenantPermissions::default();
    permissions.can_delete = false;
    permissions.max_users = Some(1);

    let tenant_context = TenantContext::new("restricted".to_string(), "client".to_string())
        .with_permissions(permissions);
    let context = EnhancedRequestContext::with_generated_id(tenant_context);

    // Create first user should succeed
    let user1_data = json!({"id": "user1", "userName": "user1"});
    let result1 = provider
        .create_resource("restricted", "User", user1_data, &context)
        .await;
    assert!(result1.is_ok());

    // Create second user should fail due to limit
    let user2_data = json!({"id": "user2", "userName": "user2"});
    let result2 = provider
        .create_resource("restricted", "User", user2_data, &context)
        .await;
    assert!(result2.is_err());

    // Delete should fail due to permission restriction
    let delete_result = provider
        .delete_resource("restricted", "User", "user1", &context)
        .await;
    assert!(delete_result.is_err());
}

/// Test end-to-end workflow: resolver -> provider -> operations
#[tokio::test]
async fn test_end_to_end_workflow() {
    // Set up tenant resolver
    let resolver = StaticTenantResolver::new();
    let tenant_context = TenantContext::new("e2e-tenant".to_string(), "e2e-client".to_string());
    resolver
        .add_tenant("e2e-api-key", tenant_context.clone())
        .await;

    // Set up multi-tenant provider
    let provider = DatabaseResourceProvider::new_in_memory()
        .await
        .expect("Failed to create database provider");

    // Simulate authentication and tenant resolution
    let resolved_tenant = resolver.resolve_tenant("e2e-api-key").await.unwrap();
    assert_eq!(resolved_tenant.tenant_id, "e2e-tenant");

    // Create enhanced context from resolved tenant
    let context = EnhancedRequestContext::with_generated_id(resolved_tenant);

    // Perform CRUD operations
    let user_data = json!({
        "userName": "e2euser",
        "displayName": "End-to-End User",
        "emails": [{"value": "e2e@example.com", "primary": true}]
    });

    // Create
    let created_user = provider
        .create_resource("e2e-tenant", "User", user_data, &context)
        .await
        .unwrap();
    let user_id = created_user.get_id().unwrap();

    // Read
    let retrieved_user = provider
        .get_resource("e2e-tenant", "User", user_id, &context)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(retrieved_user.get_username(), Some("e2euser"));

    // Update
    let updated_data = json!({
        "id": user_id,
        "userName": "e2euser",
        "displayName": "Updated E2E User"
    });
    let updated_user = provider
        .update_resource("e2e-tenant", "User", user_id, updated_data, &context)
        .await
        .unwrap();
    assert_eq!(
        updated_user.get_attribute("displayName"),
        Some(&json!("Updated E2E User"))
    );

    // List
    let users = provider
        .list_resources("e2e-tenant", "User", None, &context)
        .await
        .unwrap();
    assert_eq!(users.len(), 1);

    // Search by attribute
    let found_user = provider
        .find_resource_by_attribute(
            "e2e-tenant",
            "User",
            "userName",
            &json!("e2euser"),
            &context,
        )
        .await
        .unwrap()
        .unwrap();
    assert_eq!(found_user.get_id(), Some(user_id));

    // Delete
    provider
        .delete_resource("e2e-tenant", "User", user_id, &context)
        .await
        .unwrap();

    // Verify deletion
    let deleted_user = provider
        .get_resource("e2e-tenant", "User", user_id, &context)
        .await
        .unwrap();
    assert!(deleted_user.is_none());
}

/// Test backward compatibility with existing single-tenant code
#[tokio::test]
async fn test_backward_compatibility() {
    // Test that existing RequestContext still works
    let context = RequestContext::new("test-request".to_string());
    assert_eq!(context.request_id, "test-request");
    assert!(!context.is_multi_tenant());
    assert!(context.tenant_id().is_none());

    // Test that we can add tenant information to existing context
    let tenant_context = TenantContext::new("compat-tenant".to_string(), "client".to_string());
    let enhanced_context = RequestContext::with_tenant("test-request".to_string(), tenant_context);

    assert!(enhanced_context.is_multi_tenant());
    assert_eq!(enhanced_context.tenant_id(), Some("compat-tenant"));

    // Test conversion from enhanced to regular context
    let converted: Result<EnhancedRequestContext, _> = enhanced_context.try_into();
    assert!(converted.is_ok());

    let converted = converted.unwrap();
    assert_eq!(converted.tenant_id(), "compat-tenant");
}

/// Test performance with multiple tenants and resources
#[tokio::test]
async fn test_multi_tenant_performance() {
    let provider = DatabaseResourceProvider::new_in_memory()
        .await
        .expect("Failed to create database provider");

    let tenant_count = 5;
    let users_per_tenant = 10;

    // Create multiple tenants and users
    for tenant_idx in 0..tenant_count {
        let tenant_id = format!("perf-tenant-{}", tenant_idx);
        let tenant_context = TenantContext::new(tenant_id.clone(), "client".to_string());
        let context = EnhancedRequestContext::with_generated_id(tenant_context);

        for user_idx in 0..users_per_tenant {
            let user_data = json!({
                "id": format!("user-{}", user_idx),
                "userName": format!("user{}_{}", tenant_idx, user_idx),
                "displayName": format!("User {} from Tenant {}", user_idx, tenant_idx)
            });

            provider
                .create_resource(&tenant_id, "User", user_data, &context)
                .await
                .unwrap();
        }
    }

    // Verify data isolation and correctness
    for tenant_idx in 0..tenant_count {
        let tenant_id = format!("perf-tenant-{}", tenant_idx);
        let tenant_context = TenantContext::new(tenant_id.clone(), "client".to_string());
        let context = EnhancedRequestContext::with_generated_id(tenant_context);

        let count = provider
            .get_resource_count(&tenant_id, "User", &context)
            .await
            .unwrap();
        assert_eq!(count, users_per_tenant);

        let users = provider
            .list_resources(&tenant_id, "User", None, &context)
            .await
            .unwrap();
        assert_eq!(users.len(), users_per_tenant);
    }

    // Get overall statistics
    let stats = provider.get_stats().await;
    assert_eq!(stats.tenant_count, tenant_count);
    assert_eq!(stats.total_resources, tenant_count * users_per_tenant);
}
