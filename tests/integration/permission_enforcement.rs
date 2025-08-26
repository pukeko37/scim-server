//! Integration tests for permission enforcement and tenant isolation.
//!
//! This module tests that the InMemoryProvider correctly enforces tenant permissions
//! and maintains proper isolation between tenants.

use scim_server::{
    RequestContext, TenantContext, TenantPermissions, ResourceProvider,
    providers::StandardResourceProvider,
    storage::InMemoryStorage,
};
use serde_json::json;

/// Test that tenant permissions are properly enforced for CRUD operations
#[tokio::test]
async fn test_permission_enforcement_crud_operations() {
    let storage = InMemoryStorage::new();
    let provider = StandardResourceProvider::new(storage);

    // Create a tenant with restrictive permissions
    let mut restrictive_perms = TenantPermissions::default();
    restrictive_perms.can_create = true;
    restrictive_perms.can_read = true;
    restrictive_perms.can_update = false; // No update allowed
    restrictive_perms.can_delete = false; // No delete allowed
    restrictive_perms.can_list = true;

    let tenant_context =
        TenantContext::new("restricted-tenant".to_string(), "client-123".to_string())
            .with_permissions(restrictive_perms);
    let context = RequestContext::with_tenant_generated_id(tenant_context);

    // Create should work
    let user_data = json!({
        "userName": "test.user",
        "displayName": "Test User",
        "emails": [{"value": "test@example.com", "primary": true}]
    });

    let created_user = provider
        .create_resource("User", user_data, &context)
        .await
        .expect("Create should be allowed");

    let user_id = created_user.get_id().unwrap();

    // Read should work
    let read_result = provider
        .get_resource("User", user_id, &context)
        .await
        .expect("Read should be allowed");
    assert!(read_result.is_some());

    // List should work
    let list_result = provider
        .list_resources("User", None, &context)
        .await
        .expect("List should be allowed");
    assert_eq!(list_result.len(), 1);

    // Update should fail
    let update_data = json!({
        "userName": "test.user",
        "displayName": "Updated User"
    });

    let update_result = provider
        .update_resource("User", user_id, update_data, &context)
        .await;
    assert!(
        update_result.is_err(),
        "Update should be blocked by permissions"
    );

    // Delete should fail
    let delete_result = provider.delete_resource("User", user_id, &context).await;
    assert!(
        delete_result.is_err(),
        "Delete should be blocked by permissions"
    );
}

/// Test that resource limits are properly enforced
#[tokio::test]
async fn test_resource_limits_enforcement() {
    let storage = InMemoryStorage::new();
    let provider = StandardResourceProvider::new(storage);

    // Create a tenant with user limit of 2
    let mut limited_perms = TenantPermissions::default();
    limited_perms.max_users = Some(2);
    limited_perms.max_groups = Some(1);

    let tenant_context = TenantContext::new("limited-tenant".to_string(), "client-456".to_string())
        .with_permissions(limited_perms);
    let context = RequestContext::with_tenant_generated_id(tenant_context);

    // Create first user - should succeed
    let user1_data = json!({
        "userName": "user1",
        "displayName": "User One"
    });

    let user1 = provider
        .create_resource("User", user1_data, &context)
        .await
        .expect("First user should be created");
    assert!(user1.get_id().is_some());

    // Create second user - should succeed
    let user2_data = json!({
        "userName": "user2",
        "displayName": "User Two"
    });

    let user2 = provider
        .create_resource("User", user2_data, &context)
        .await
        .expect("Second user should be created");
    assert!(user2.get_id().is_some());

    // Create third user - should fail due to limit
    let user3_data = json!({
        "userName": "user3",
        "displayName": "User Three"
    });

    let user3_result = provider.create_resource("User", user3_data, &context).await;
    assert!(user3_result.is_err(), "Third user should exceed limit");

    // Create first group - should succeed
    let group1_data = json!({
        "displayName": "Group One",
        "description": "First group"
    });

    let group1 = provider
        .create_resource("Group", group1_data, &context)
        .await
        .expect("First group should be created");
    assert!(group1.get_id().is_some());

    // Create second group - should fail due to limit
    let group2_data = json!({
        "displayName": "Group Two",
        "description": "Second group"
    });

    let group2_result = provider
        .create_resource("Group", group2_data, &context)
        .await;
    assert!(group2_result.is_err(), "Second group should exceed limit");
}

/// Test that tenants are properly isolated from each other
#[tokio::test]
async fn test_tenant_isolation() {
    let storage = InMemoryStorage::new();
    let provider = StandardResourceProvider::new(storage);

    // Create two different tenant contexts
    let tenant_a_context = TenantContext::new("tenant-a".to_string(), "client-a".to_string());
    let context_a = RequestContext::with_tenant_generated_id(tenant_a_context);

    let tenant_b_context = TenantContext::new("tenant-b".to_string(), "client-b".to_string());
    let context_b = RequestContext::with_tenant_generated_id(tenant_b_context);

    // Create users in both tenants with different usernames for cross-tenant testing
    // Create users in both tenants with different usernames
    let user_data = json!({
        "userName": "alice.tenant.a",
        "displayName": "Alice in Tenant A"
    });

    let user_a = provider
        .create_resource("User", user_data, &context_a)
        .await
        .expect("User creation in tenant A should succeed");

    let user_data_b = json!({
        "userName": "alice.tenant.b",
        "displayName": "Alice in Tenant B"
    });

    let user_b = provider
        .create_resource("User", user_data_b, &context_b)
        .await
        .expect("User creation in tenant B should succeed");

    let actual_user_a_id = user_a.get_id().unwrap();
    let actual_user_b_id = user_b.get_id().unwrap();

    // Tenant A accessing user ID "1" should find its own user, not tenant B's user
    let cross_access_result = provider
        .get_resource("User", &actual_user_b_id, &context_a)
        .await
        .expect("Query should succeed");

    if let Some(ref found_user) = cross_access_result {
        // Should find its own user, not tenant B's user
        assert_eq!(
            found_user.get_username().unwrap(),
            "alice.tenant.a",
            "Tenant A should find its own user with same ID, not tenant B's user"
        );
    }

    // Tenant B accessing user ID should find its own user or none
    let cross_access_result = provider
        .get_resource("User", &actual_user_a_id, &context_b)
        .await
        .expect("Query should succeed");

    if let Some(ref found_user) = cross_access_result {
        // Should find its own user, not tenant A's user
        assert_eq!(
            found_user.get_username().unwrap(),
            "alice.tenant.b",
            "Tenant B should find its own user with same ID, not tenant A's user"
        );
    }

    // Tenant A should not find tenant B's user by username
    let username_search_result = provider
        .find_resource_by_attribute("User", "userName", &json!("alice.tenant.b"), &context_a)
        .await
        .expect("Search should succeed");
    assert!(
        username_search_result.is_none(),
        "Tenant A should not find tenant B's user by username"
    );

    // Tenant B should not find tenant A's user by username
    let username_search_result = provider
        .find_resource_by_attribute("User", "userName", &json!("alice.tenant.a"), &context_b)
        .await
        .expect("Search should succeed");
    assert!(
        username_search_result.is_none(),
        "Tenant B should not find tenant A's user by username"
    );

    // Verify list operations are isolated
    let tenant_a_users = provider
        .list_resources("User", None, &context_a)
        .await
        .expect("List should succeed");
    assert_eq!(tenant_a_users.len(), 1);

    let tenant_b_users = provider
        .list_resources("User", None, &context_b)
        .await
        .expect("List should succeed");
    assert_eq!(tenant_b_users.len(), 1);
}

/// Test that single-tenant operations work correctly without permissions
#[tokio::test]
async fn test_single_tenant_permissions() {
    let storage = InMemoryStorage::new();
    let provider = StandardResourceProvider::new(storage);

    // Create a single-tenant context (no tenant information)
    let single_context = RequestContext::with_generated_id();

    // All operations should be allowed for single-tenant
    let user_data = json!({
        "userName": "single.user",
        "displayName": "Single Tenant User"
    });

    let created_user = provider
        .create_resource("User", user_data, &single_context)
        .await
        .expect("Single-tenant create should always work");

    let user_id = created_user.get_id().unwrap();

    // Read should work
    let read_result = provider
        .get_resource("User", user_id, &single_context)
        .await
        .expect("Single-tenant read should work");
    assert!(read_result.is_some());

    // Update should work
    let update_data = json!({
        "userName": "single.user",
        "displayName": "Updated Single User"
    });

    let updated_user = provider
        .update_resource("User", user_id, update_data, &single_context)
        .await
        .expect("Single-tenant update should work");
    assert_eq!(
        updated_user.get_attribute("displayName").unwrap(),
        &json!("Updated Single User")
    );

    // Delete should work
    provider
        .delete_resource("User", user_id, &single_context)
        .await
        .expect("Single-tenant delete should work");

    // Verify deletion
    let deleted_result = provider
        .get_resource("User", user_id, &single_context)
        .await
        .expect("Query should succeed");
    assert!(deleted_result.is_none());
}

/// Test mixed tenant and single-tenant isolation
#[tokio::test]
async fn test_mixed_tenant_isolation() {
    let storage = InMemoryStorage::new();
    let provider = StandardResourceProvider::new(storage);

    // Create a single-tenant context
    let single_context = RequestContext::with_generated_id();

    // Create a multi-tenant context
    let tenant_context = TenantContext::new("multi-tenant".to_string(), "client-789".to_string());
    let multi_context = RequestContext::with_tenant_generated_id(tenant_context);

    // Create users in both contexts
    let single_user_data = json!({
        "userName": "single.user",
        "displayName": "Single Tenant User"
    });

    let single_user = provider
        .create_resource("User", single_user_data, &single_context)
        .await
        .expect("Single-tenant user creation should succeed");

    let multi_user_data = json!({
        "userName": "multi.user",
        "displayName": "Multi Tenant User"
    });

    let multi_user = provider
        .create_resource("User", multi_user_data, &multi_context)
        .await
        .expect("Multi-tenant user creation should succeed");

    // Single-tenant should not see multi-tenant users
    let single_users = provider
        .list_resources("User", None, &single_context)
        .await
        .expect("Single-tenant list should work");
    assert_eq!(single_users.len(), 1);
    assert_eq!(single_users[0].get_username().unwrap(), "single.user");

    // Multi-tenant should not see single-tenant users
    let multi_users = provider
        .list_resources("User", None, &multi_context)
        .await
        .expect("Multi-tenant list should work");
    assert_eq!(multi_users.len(), 1);
    assert_eq!(multi_users[0].get_username().unwrap(), "multi.user");

    // Cross-access should not work
    let single_user_id = single_user.get_id().unwrap();
    let multi_user_id = multi_user.get_id().unwrap();

    let cross_access_1 = provider
        .get_resource("User", multi_user_id, &single_context)
        .await
        .expect("Query should succeed");

    if let Some(ref found_user) = cross_access_1 {
        // Should find its own user, not the multi-tenant user
        assert_eq!(
            found_user.get_username().unwrap(),
            "single.user",
            "Single-tenant should find its own user with same ID, not multi-tenant user"
        );
    }

    let cross_access_2 = provider
        .get_resource("User", single_user_id, &multi_context)
        .await
        .expect("Query should succeed");

    if let Some(ref found_user) = cross_access_2 {
        // Should find its own user, not the single-tenant user
        assert_eq!(
            found_user.get_username().unwrap(),
            "multi.user",
            "Multi-tenant should find its own user with same ID, not single-tenant user"
        );
    }
}

/// Test that permission validation works with different operations
#[tokio::test]
async fn test_operation_specific_permissions() {
    let storage = InMemoryStorage::new();
    let provider = StandardResourceProvider::new(storage);

    // Create a read-only tenant
    let mut readonly_perms = TenantPermissions::default();
    readonly_perms.can_create = false;
    readonly_perms.can_read = true;
    readonly_perms.can_update = false;
    readonly_perms.can_delete = false;
    readonly_perms.can_list = true;

    let readonly_tenant =
        TenantContext::new("readonly-tenant".to_string(), "readonly-client".to_string())
            .with_permissions(readonly_perms);
    let readonly_context = RequestContext::with_tenant_generated_id(readonly_tenant);

    // Create a write-only tenant
    let mut writeonly_perms = TenantPermissions::default();
    writeonly_perms.can_create = true;
    writeonly_perms.can_read = false;
    writeonly_perms.can_update = true;
    writeonly_perms.can_delete = true;
    writeonly_perms.can_list = false;

    let writeonly_tenant = TenantContext::new(
        "writeonly-tenant".to_string(),
        "writeonly-client".to_string(),
    )
    .with_permissions(writeonly_perms);
    let writeonly_context = RequestContext::with_tenant_generated_id(writeonly_tenant);

    // First, create a user in the readonly tenant using a full-permission context for setup
    let full_readonly_tenant =
        TenantContext::new("readonly-tenant".to_string(), "readonly-client".to_string());
    let full_readonly_context = RequestContext::with_tenant_generated_id(full_readonly_tenant);

    let readonly_user_data = json!({
        "userName": "readonly.user",
        "displayName": "Readonly User"
    });

    let readonly_user = provider
        .create_resource("User", readonly_user_data.clone(), &full_readonly_context)
        .await
        .expect("User creation should work with full permissions for setup");

    let readonly_user_id = readonly_user.get_id().unwrap();

    // Test read-only tenant permissions
    let read_result = provider
        .get_resource("User", readonly_user_id, &readonly_context)
        .await
        .expect("Read should be allowed for readonly tenant");
    assert!(read_result.is_some());

    let list_result = provider
        .list_resources("User", None, &readonly_context)
        .await
        .expect("List should be allowed for readonly tenant");
    assert_eq!(list_result.len(), 1);

    let create_result = provider
        .create_resource("User", readonly_user_data.clone(), &readonly_context)
        .await;
    assert!(
        create_result.is_err(),
        "Create should be blocked for readonly tenant"
    );

    // Test write-only tenant by creating a user first
    let writeonly_user_data = json!({
        "userName": "writeonly.user",
        "displayName": "Writeonly User"
    });

    let writeonly_user = provider
        .create_resource("User", writeonly_user_data, &writeonly_context)
        .await
        .expect("Create should be allowed for writeonly tenant");

    let writeonly_user_id = writeonly_user.get_id().unwrap();

    // Test write-only restrictions
    let read_result = provider
        .get_resource("User", writeonly_user_id, &writeonly_context)
        .await;
    assert!(
        read_result.is_err(),
        "Read should be blocked for writeonly tenant"
    );

    let list_result = provider
        .list_resources("User", None, &writeonly_context)
        .await;
    assert!(
        list_result.is_err(),
        "List should be blocked for writeonly tenant"
    );

    let update_data = json!({
        "userName": "writeonly.user",
        "displayName": "Updated Writeonly User"
    });

    let update_result = provider
        .update_resource("User", writeonly_user_id, update_data, &writeonly_context)
        .await
        .expect("Update should be allowed for writeonly tenant");
    assert_eq!(
        update_result.get_attribute("displayName").unwrap(),
        &json!("Updated Writeonly User")
    );
}
