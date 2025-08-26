//! Unit tests for the StandardResourceProvider.
//!
//! These tests are copied from the InMemoryProvider tests and adapted to use
//! StandardResourceProvider<InMemoryStorage> to ensure behavioral equivalence.

use scim_server::providers::{InMemoryError, StandardResourceProvider};
use scim_server::resource::version::ConditionalResult;
use scim_server::resource::{ListQuery, RequestContext, ResourceProvider, TenantContext};
use scim_server::storage::InMemoryStorage;
use serde_json::json;
use std::sync::Arc;

fn create_test_user_data(username: &str) -> serde_json::Value {
    json!({
        "userName": username,
        "displayName": format!("User {}", username),
        "active": true
    })
}

#[tokio::test]
async fn test_single_tenant_operations() {
    let storage = InMemoryStorage::new();
    let provider = StandardResourceProvider::new(storage);
    let context = RequestContext::with_generated_id();

    // Create user
    let user_data = create_test_user_data("john.doe");
    let user = provider
        .create_resource("User", user_data, &context)
        .await
        .unwrap();
    let user_id = user.get_id().unwrap();

    // Get user
    let retrieved = provider
        .get_resource("User", user_id, &context)
        .await
        .unwrap();
    assert!(retrieved.is_some());
    assert_eq!(retrieved.unwrap().get_username(), Some("john.doe"));

    // Update user
    let update_data = json!({
        "userName": "john.doe",
        "displayName": "John Updated",
        "active": false
    });
    let _updated = provider
        .update_resource("User", user_id, update_data, &context)
        .await
        .unwrap();
    // Check that the resource was updated (we'll verify via getting it back)
    let verified = provider
        .get_resource("User", user_id, &context)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        verified.get_attribute("displayName"),
        Some(&json!("John Updated"))
    );

    // List users
    let query = ListQuery::default();
    let users = provider
        .list_resources("User", Some(&query), &context)
        .await
        .unwrap();
    assert_eq!(users.len(), 1);

    // Delete user
    provider
        .delete_resource("User", user_id, &context)
        .await
        .unwrap();
    let deleted = provider
        .get_resource("User", user_id, &context)
        .await
        .unwrap();
    assert!(deleted.is_none());
}

#[tokio::test]
async fn test_multi_tenant_isolation() {
    let storage = InMemoryStorage::new();
    let provider = StandardResourceProvider::new(storage);

    // Create users in different tenants
    let tenant_a_context = TenantContext::new("tenant-a".to_string(), "client-a".to_string());
    let context_a = RequestContext::with_tenant_generated_id(tenant_a_context);

    let tenant_b_context = TenantContext::new("tenant-b".to_string(), "client-b".to_string());
    let context_b = RequestContext::with_tenant_generated_id(tenant_b_context);

    // Create user in tenant A
    let user_a_data = create_test_user_data("alice.tenant.a");
    let user_a = provider
        .create_resource("User", user_a_data, &context_a)
        .await
        .unwrap();
    let _user_a_id = user_a.get_id().unwrap();

    // Create user in tenant B (different username)
    let user_b_data = create_test_user_data("alice.tenant.b");
    let user_b = provider
        .create_resource("User", user_b_data, &context_b)
        .await
        .unwrap();
    let _user_b_id = user_b.get_id().unwrap();

    // Verify isolation using username search - tenant B should not find tenant A's user
    let alice_a_from_b = provider
        .find_resource_by_attribute("User", "userName", &json!("alice.tenant.a"), &context_b)
        .await
        .unwrap();
    assert!(
        alice_a_from_b.is_none(),
        "Tenant B should not find tenant A's user by username"
    );

    // Verify tenant A should not find tenant B's user
    let alice_b_from_a = provider
        .find_resource_by_attribute("User", "userName", &json!("alice.tenant.b"), &context_a)
        .await
        .unwrap();
    assert!(
        alice_b_from_a.is_none(),
        "Tenant A should not find tenant B's user by username"
    );

    // Each tenant should find their own user
    let alice_a_from_a = provider
        .find_resource_by_attribute("User", "userName", &json!("alice.tenant.a"), &context_a)
        .await
        .unwrap();
    assert!(
        alice_a_from_a.is_some(),
        "Tenant A should find its own user"
    );

    let alice_b_from_b = provider
        .find_resource_by_attribute("User", "userName", &json!("alice.tenant.b"), &context_b)
        .await
        .unwrap();
    assert!(
        alice_b_from_b.is_some(),
        "Tenant B should find its own user"
    );

    // Each tenant should see only their own user
    let query = ListQuery::default();
    let users_a = provider
        .list_resources("User", Some(&query), &context_a)
        .await
        .unwrap();
    let users_b = provider
        .list_resources("User", Some(&query), &context_b)
        .await
        .unwrap();

    assert_eq!(users_a.len(), 1);
    assert_eq!(users_b.len(), 1);
    assert_eq!(users_a[0].get_username(), Some("alice.tenant.a"));
    assert_eq!(users_b[0].get_username(), Some("alice.tenant.b"));
}

#[tokio::test]
async fn test_username_duplicate_detection() {
    let storage = InMemoryStorage::new();
    let provider = StandardResourceProvider::new(storage);
    let context = RequestContext::with_generated_id();

    // Create first user
    let user1_data = create_test_user_data("duplicate");
    let _user1 = provider
        .create_resource("User", user1_data, &context)
        .await
        .unwrap();

    // Attempt to create second user with same username
    let user2_data = create_test_user_data("duplicate");
    let result = provider.create_resource("User", user2_data, &context).await;

    assert!(result.is_err());
    match result.unwrap_err() {
        InMemoryError::DuplicateAttribute {
            attribute, value, ..
        } => {
            assert_eq!(attribute, "userName");
            assert_eq!(value, "duplicate");
        }
        _ => panic!("Expected DuplicateAttribute error"),
    }
}

#[tokio::test]
async fn test_cross_tenant_username_allowed() {
    let storage = InMemoryStorage::new();
    let provider = StandardResourceProvider::new(storage);

    let tenant_a_context = TenantContext::new("tenant-a".to_string(), "client-a".to_string());
    let context_a = RequestContext::with_tenant_generated_id(tenant_a_context);

    let tenant_b_context = TenantContext::new("tenant-b".to_string(), "client-b".to_string());
    let context_b = RequestContext::with_tenant_generated_id(tenant_b_context);

    // Create user with same username in both tenants - should succeed
    let user_data = create_test_user_data("shared.name");

    let user_a = provider
        .create_resource("User", user_data.clone(), &context_a)
        .await
        .unwrap();
    let user_b = provider
        .create_resource("User", user_data, &context_b)
        .await
        .unwrap();

    assert_eq!(user_a.get_username(), Some("shared.name"));
    assert_eq!(user_b.get_username(), Some("shared.name"));
}

#[tokio::test]
async fn test_find_resource_by_attribute() {
    let storage = InMemoryStorage::new();
    let provider = StandardResourceProvider::new(storage);
    let context = RequestContext::with_generated_id();

    // Create a user
    let user_data = create_test_user_data("john.doe");
    let _user = provider
        .create_resource("User", user_data, &context)
        .await
        .unwrap();

    // Find by userName
    let found = provider
        .find_resource_by_attribute("User", "userName", &json!("john.doe"), &context)
        .await
        .unwrap();

    assert!(found.is_some());
    assert_eq!(found.unwrap().get_username(), Some("john.doe"));

    // Find by non-existent attribute value
    let not_found = provider
        .find_resource_by_attribute("User", "userName", &json!("nonexistent"), &context)
        .await
        .unwrap();

    assert!(not_found.is_none());
}

#[tokio::test]
async fn test_resource_exists() {
    let storage = InMemoryStorage::new();
    let provider = StandardResourceProvider::new(storage);
    let context = RequestContext::with_generated_id();

    // Create a user
    let user_data = create_test_user_data("test.user");
    let user = provider
        .create_resource("User", user_data, &context)
        .await
        .unwrap();
    let user_id = user.get_id().unwrap();

    // Check existence
    let exists = provider
        .resource_exists("User", user_id, &context)
        .await
        .unwrap();
    assert!(exists);

    // Check non-existent resource
    let not_exists = provider
        .resource_exists("User", "nonexistent-id", &context)
        .await
        .unwrap();
    assert!(!not_exists);

    // Delete and check again
    provider
        .delete_resource("User", user_id, &context)
        .await
        .unwrap();

    let exists_after_delete = provider
        .resource_exists("User", user_id, &context)
        .await
        .unwrap();
    assert!(!exists_after_delete);
}

#[tokio::test]
async fn test_provider_stats() {
    let storage = InMemoryStorage::new();
    let provider = StandardResourceProvider::new(storage);

    let tenant_a_context = TenantContext::new("tenant-a".to_string(), "client-a".to_string());
    let context_a = RequestContext::with_tenant_generated_id(tenant_a_context);

    let single_context = RequestContext::with_generated_id();

    // Create resources in different tenants and types
    let _user1 = provider
        .create_resource("User", create_test_user_data("user1"), &context_a)
        .await
        .unwrap();
    let _user2 = provider
        .create_resource("User", create_test_user_data("user2"), &single_context)
        .await
        .unwrap();
    let _group = provider
        .create_resource("Group", json!({"displayName": "Test Group"}), &context_a)
        .await
        .unwrap();

    let final_stats = provider.get_stats().await;
    assert_eq!(final_stats.tenant_count, 2); // tenant-a and default
    assert_eq!(final_stats.total_resources, 3);
    assert_eq!(final_stats.resource_type_count, 2); // User and Group
    assert!(final_stats.resource_types.contains(&"User".to_string()));
    assert!(final_stats.resource_types.contains(&"Group".to_string()));
}

#[tokio::test]
async fn test_clear_functionality() {
    let storage = InMemoryStorage::new();
    let provider = StandardResourceProvider::new(storage);
    let context = RequestContext::with_generated_id();

    // Create some data
    let _user = provider
        .create_resource("User", create_test_user_data("test"), &context)
        .await
        .unwrap();

    let stats_before = provider.get_stats().await;
    assert!(stats_before.total_resources > 0);

    // Clear all data
    provider.clear().await;

    let stats_after = provider.get_stats().await;
    assert_eq!(stats_after.total_resources, 0);
    assert_eq!(stats_after.tenant_count, 0);
}

#[tokio::test]
async fn test_conditional_operations_via_resource_provider() {
    // This test ensures StandardResourceProvider implements conditional operations via ResourceProvider trait
    // Using static dispatch with generic function to enforce trait bounds
    async fn test_provider<P>(provider: &P, context: &RequestContext)
    where
        P: ResourceProvider<Error = InMemoryError> + Sync,
    {
        // Create a user first using regular provider
        let user_data = create_test_user_data("jane.doe");
        let user = provider
            .create_resource("User", user_data, context)
            .await
            .unwrap();
        let user_id = user.get_id().unwrap();

        // Get versioned resource using trait method
        let versioned = provider
            .get_versioned_resource("User", user_id, context)
            .await
            .unwrap()
            .unwrap();

        // Test conditional update using trait method
        let update_data = json!({
            "userName": "jane.doe",
            "displayName": "Jane Updated",
            "active": false
        });

        let result = provider
            .conditional_update("User", user_id, update_data, versioned.version(), context)
            .await
            .unwrap();

        // Should succeed since version matches
        assert!(matches!(result, ConditionalResult::Success(_)));
    }

    let storage = InMemoryStorage::new();
    let provider = StandardResourceProvider::new(storage);
    let context = RequestContext::with_generated_id();

    // This tests that conditional operations work via ResourceProvider trait
    test_provider(&provider, &context).await;
}

#[tokio::test]
async fn test_conditional_provider_concurrent_updates() {
    use tokio::task::JoinSet;

    let storage = InMemoryStorage::new();
    let provider = Arc::new(StandardResourceProvider::new(storage));
    let context = RequestContext::with_generated_id();

    // Create a user first
    let user_data = create_test_user_data("concurrent.user");
    let user = provider
        .create_resource("User", user_data, &context)
        .await
        .unwrap();
    let user_id = user.get_id().unwrap().to_string();

    // Get initial version
    let initial_versioned = provider
        .get_versioned_resource("User", &user_id, &context)
        .await
        .unwrap()
        .unwrap();

    // Launch concurrent updates with the same version
    let mut tasks = JoinSet::new();
    let num_concurrent = 10;

    for i in 0..num_concurrent {
        let provider_clone: Arc<StandardResourceProvider<InMemoryStorage>> = Arc::clone(&provider);
        let context_clone = context.clone();
        let user_id_clone = user_id.clone();
        let version_clone = initial_versioned.version().clone();

        tasks.spawn(async move {
            let update_data = json!({
                "userName": "concurrent.user",
                "displayName": format!("Update {}", i),
                "active": true
            });

            provider_clone
                .conditional_update(
                    &"User",
                    &user_id_clone,
                    update_data,
                    &version_clone,
                    &context_clone,
                )
                .await
        });
    }

    // Collect results
    let mut success_count = 0;
    let mut conflict_count = 0;

    while let Some(result) = tasks.join_next().await {
        match result.unwrap().unwrap() {
            ConditionalResult::Success(_) => success_count += 1,
            ConditionalResult::VersionMismatch(_) => conflict_count += 1,
            ConditionalResult::NotFound => panic!("Resource should exist"),
        }
    }

    // Only one update should succeed, others should get version conflicts
    assert_eq!(success_count, 1, "Exactly one update should succeed");
    assert_eq!(
        conflict_count,
        num_concurrent - 1,
        "Other updates should conflict"
    );
}

#[tokio::test]
async fn test_conditional_provider_delete_version_conflict() {
    let storage = InMemoryStorage::new();
    let provider = StandardResourceProvider::new(storage);
    let context = RequestContext::with_generated_id();

    // Create a user
    let user_data = create_test_user_data("delete.user");
    let user = provider
        .create_resource("User", user_data, &context)
        .await
        .unwrap();
    let user_id = user.get_id().unwrap();

    // Get initial version
    let initial_versioned = provider
        .get_versioned_resource("User", user_id, &context)
        .await
        .unwrap()
        .unwrap();

    // Update the resource to change its version
    let update_data = json!({
        "userName": "delete.user",
        "displayName": "Updated Before Delete",
        "active": false
    });
    provider
        .update_resource("User", user_id, update_data, &context)
        .await
        .unwrap();

    // Try to delete with old version - should fail
    let delete_result = provider
        .conditional_delete("User", user_id, initial_versioned.version(), &context)
        .await
        .unwrap();

    // Should get version conflict
    assert!(matches!(
        delete_result,
        ConditionalResult::VersionMismatch(_)
    ));

    // Resource should still exist
    let still_exists = provider
        .resource_exists("User", user_id, &context)
        .await
        .unwrap();
    assert!(still_exists);
}

#[tokio::test]
async fn test_conditional_provider_successful_delete() {
    let storage = InMemoryStorage::new();
    let provider = StandardResourceProvider::new(storage);
    let context = RequestContext::with_generated_id();

    // Create a user
    let user_data = create_test_user_data("delete.success");
    let user = provider
        .create_resource("User", user_data, &context)
        .await
        .unwrap();
    let user_id = user.get_id().unwrap();

    // Get current version
    let current_versioned = provider
        .get_versioned_resource("User", user_id, &context)
        .await
        .unwrap()
        .unwrap();

    // Delete with correct version - should succeed
    let delete_result = provider
        .conditional_delete("User", user_id, current_versioned.version(), &context)
        .await
        .unwrap();

    // Should succeed
    assert!(matches!(delete_result, ConditionalResult::Success(())));

    // Resource should no longer exist
    let exists = provider
        .resource_exists("User", user_id, &context)
        .await
        .unwrap();
    assert!(!exists);
}
