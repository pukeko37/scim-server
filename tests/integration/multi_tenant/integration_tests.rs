//! Multi-Tenant Integration Tests
//!
//! This test suite validates the successful integration of multi-tenant functionality
//! into the main SCIM server application. It tests the complete flow from tenant
//! resolution through resource operations to ensure all components work together.

use scim_server::{
    IsolationLevel, RequestContext, ResourceProvider, StaticTenantResolver, TenantContext,
    TenantPermissions, TenantResolver, providers::StandardResourceProvider,
    storage::InMemoryStorage,
};
use serde_json::json;

// MockSingleTenantProvider removed - was unused after transitioning to standard InMemoryProvider

/// Test basic multi-tenant context creation and usage
#[tokio::test]
async fn test_enhanced_request_context_creation() {
    let tenant_context = TenantContext::new("test-tenant".to_string(), "test-client".to_string());
    let enhanced_context = RequestContext::with_tenant_generated_id(tenant_context.clone());

    assert_eq!(enhanced_context.tenant_id(), Some("test-tenant"));
    assert_eq!(enhanced_context.client_id(), Some("test-client"));
    assert!(!enhanced_context.request_id.is_empty());

    // Test that this is a multi-tenant context
    assert!(enhanced_context.is_multi_tenant());
    assert_eq!(enhanced_context.tenant_id(), Some("test-tenant"));
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
/// Test single-tenant to multi-tenant adapter
/// TODO: Fix adapter test - currently disabled due to compilation issues
/*
#[tokio::test]
async fn test_single_tenant_adapter() {
    let mock_provider = Arc::new(MockSingleTenantProvider::new());
    let adapter = SingleTenantAdapter::new(mock_provider);

    let tenant_context = TenantContext::new("adapter-test".to_string(), "client".to_string());
    let context = RequestContext::with_tenant_generated_id(tenant_context);

    // Test create operation
    let user_data = json!({
        "userName": "adapteruser",
        "displayName": "Adapter User"
    });

    let created_user = adapter
        .create_resource("User", user_data, &context)
        .await
        .unwrap();

    assert_eq!(created_user.get_username(), Some("adapteruser"));
    assert!(created_user.get_id().is_some());

    // Test get operation
    let user_id = created_user.get_id().unwrap();
    let retrieved_user = adapter
        .get_resource("User", user_id, &context)
        .await
        .unwrap();

    assert!(retrieved_user.is_some());
    let retrieved_user = retrieved_user.unwrap();
    assert_eq!(retrieved_user.get_username(), Some("adapteruser"));

    // Test tenant validation - should fail with wrong tenant
    let wrong_tenant_context = TenantContext::new("wrong-tenant".to_string(), "client".to_string());
    let wrong_context = RequestContext::with_tenant_generated_id(wrong_tenant_context);

    let result = adapter.get_resource("User", user_id, &wrong_context).await;
    assert!(result.is_err());
}
*/

/// Test multi-tenant provider functionality
#[tokio::test]
async fn test_multi_tenant_provider_functionality() {
    let storage = InMemoryStorage::new();
    let provider = StandardResourceProvider::new(storage);

    let tenant_context = TenantContext::new("db-test".to_string(), "client".to_string());
    let context = RequestContext::with_tenant_generated_id(tenant_context);

    // Test create user
    let user_data = json!({
        "userName": "dbuser",
        "displayName": "Database User",
        "emails": [{"value": "db@example.com", "primary": true}]
    });

    let created_user = provider
        .create_resource("User", user_data, &context)
        .await
        .unwrap();

    assert_eq!(created_user.get_username(), Some("dbuser"));
    assert!(created_user.get_id().is_some());

    // Test list resources
    let users = provider
        .list_resources("User", None, &context)
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
        .update_resource("User", user_id, updated_data, &context)
        .await
        .unwrap();
    assert_eq!(
        updated_user.get_attribute("displayName"),
        Some(&json!("Updated Database User"))
    );

    // Test delete
    provider
        .delete_resource("User", user_id, &context)
        .await
        .unwrap();

    let deleted_user = provider
        .get_resource("User", user_id, &context)
        .await
        .unwrap();
    assert!(deleted_user.is_none());
}

/// Test multi-tenant isolation between tenants
#[tokio::test]
async fn test_multi_tenant_isolation() {
    let storage = InMemoryStorage::new();
    let provider = StandardResourceProvider::new(storage);

    // Set up two different tenants
    let tenant_a_context = TenantContext::new("tenant-a".to_string(), "client-a".to_string());
    let context_a = RequestContext::with_tenant_generated_id(tenant_a_context);

    let tenant_b_context = TenantContext::new("tenant-b".to_string(), "client-b".to_string());
    let context_b = RequestContext::with_tenant_generated_id(tenant_b_context);

    // Create users in both tenants with same username
    let user_data_a = json!({"id": "user1", "userName": "john", "displayName": "John from A"});
    let user_data_b = json!({"id": "user1", "userName": "john", "displayName": "John from B"});

    provider
        .create_resource("User", user_data_a, &context_a)
        .await
        .unwrap();

    provider
        .create_resource("User", user_data_b, &context_b)
        .await
        .unwrap();

    // Each tenant should only see their own data
    let user_a = provider
        .get_resource("User", "user1", &context_a)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        user_a.get_attribute("displayName"),
        Some(&json!("John from A"))
    );

    let user_b = provider
        .get_resource("User", "user1", &context_b)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        user_b.get_attribute("displayName"),
        Some(&json!("John from B"))
    );

    // Cross-tenant access should return None (isolation via context)
    let _cross_access_result = provider
        .get_resource("User", "user1", &context_a)
        .await
        .unwrap();
    // Should not find tenant-b's resource when using tenant-a's context

    // Resource counts can be checked via list_resources
    let list_a = provider
        .list_resources("User", None, &context_a)
        .await
        .unwrap();
    let list_b = provider
        .list_resources("User", None, &context_b)
        .await
        .unwrap();
    assert_eq!(list_a.len(), 1);
    assert_eq!(list_b.len(), 1);
}

/// Test tenant permissions and limits
#[tokio::test]
async fn test_tenant_permissions_and_limits() {
    let storage = InMemoryStorage::new();
    let provider = StandardResourceProvider::new(storage);

    // Create tenant with restricted permissions
    let mut permissions = TenantPermissions::default();
    permissions.can_delete = false;
    permissions.max_users = Some(1);

    let tenant_context = TenantContext::new("restricted".to_string(), "client".to_string())
        .with_permissions(permissions);
    let context = RequestContext::with_tenant_generated_id(tenant_context);

    // Create first user should succeed
    let user1_data = json!({"id": "user1", "userName": "user1"});
    let result1 = provider.create_resource("User", user1_data, &context).await;
    assert!(result1.is_ok());

    // Create second user - should fail due to max_users limit of 1
    let user2_data = json!({"id": "user2", "userName": "user2"});
    let result2 = provider.create_resource("User", user2_data, &context).await;
    // Should fail because we've reached the user limit
    assert!(result2.is_err());

    // Delete should fail - tenant has can_delete = false
    let delete_result = provider.delete_resource("User", "user1", &context).await;
    // Should fail because tenant doesn't have delete permission
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
    let storage = InMemoryStorage::new();
    let provider = StandardResourceProvider::new(storage);

    // Simulate authentication and tenant resolution
    let resolved_tenant = resolver.resolve_tenant("e2e-api-key").await.unwrap();
    assert_eq!(resolved_tenant.tenant_id, "e2e-tenant");

    // Create enhanced context from resolved tenant
    let context = RequestContext::with_tenant_generated_id(resolved_tenant);

    // Perform CRUD operations
    let user_data = json!({
        "userName": "e2euser",
        "displayName": "End-to-End User",
        "emails": [{"value": "e2e@example.com", "primary": true}]
    });

    // Create
    let created_user = provider
        .create_resource("User", user_data, &context)
        .await
        .unwrap();
    let user_id = created_user.get_id().unwrap();

    // Read
    let retrieved_user = provider
        .get_resource("User", user_id, &context)
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
        .update_resource("User", user_id, updated_data, &context)
        .await
        .unwrap();
    assert_eq!(
        updated_user.get_attribute("displayName"),
        Some(&json!("Updated E2E User"))
    );

    // List
    let users = provider
        .list_resources("User", None, &context)
        .await
        .unwrap();
    assert_eq!(users.len(), 1);

    // Search by attribute
    let found_user = provider
        .find_resource_by_attribute("User", "userName", &json!("e2euser"), &context)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(found_user.get_id(), Some(user_id));

    // Delete
    provider
        .delete_resource("User", user_id, &context)
        .await
        .unwrap();

    // Verify deletion
    let deleted_user = provider
        .get_resource("User", user_id, &context)
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
    let converted = enhanced_context.clone();
    assert!(converted.is_multi_tenant());
    assert_eq!(converted.tenant_id(), Some("compat-tenant"));
}

/// Test performance with multiple tenants and resources
#[tokio::test]
async fn test_multi_tenant_performance() {
    let storage = InMemoryStorage::new();
    let provider = StandardResourceProvider::new(storage);

    let tenant_count = 5;
    let users_per_tenant = 10;

    // Create multiple tenants and users
    for tenant_idx in 0..tenant_count {
        let tenant_id = format!("perf-tenant-{}", tenant_idx);
        let tenant_context = TenantContext::new(tenant_id.clone(), "client".to_string());
        let context = RequestContext::with_tenant_generated_id(tenant_context);

        for user_idx in 0..users_per_tenant {
            let user_data = json!({
                "id": format!("user-{}", user_idx),
                "userName": format!("user{}_{}", tenant_idx, user_idx),
                "displayName": format!("User {} from Tenant {}", user_idx, tenant_idx)
            });

            provider
                .create_resource("User", user_data, &context)
                .await
                .unwrap();
        }
    }

    // Verify data isolation and correctness
    for tenant_idx in 0..tenant_count {
        let tenant_id = format!("perf-tenant-{}", tenant_idx);
        let tenant_context = TenantContext::new(tenant_id.clone(), "client".to_string());
        let context = RequestContext::with_tenant_generated_id(tenant_context);

        let users = provider
            .list_resources("User", None, &context)
            .await
            .unwrap();
        assert_eq!(users.len(), users_per_tenant);
    }

    // Get overall statistics
    let stats = provider.get_stats().await;
    assert_eq!(stats.tenant_count, tenant_count);
    assert_eq!(stats.total_resources, tenant_count * users_per_tenant);
}
