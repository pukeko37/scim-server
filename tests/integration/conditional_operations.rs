//! Integration test proving conditional operations prevent data loss.
//!
//! This test demonstrates the core value proposition: conditional operations
//! prevent data corruption when multiple clients modify the same resource.

use scim_server::providers::StandardResourceProvider;
use scim_server::resource::{core::RequestContext, version::ConditionalResult};
use scim_server::storage::InMemoryStorage;
use serde_json::json;
use std::sync::Arc;
use tokio;

/// Test that proves conditional operations prevent data loss in concurrent scenarios.
/// This is the PRIMARY test that validates the entire concurrency control system.
#[tokio::test]
async fn test_conditional_operations_prevent_data_loss() {
    let storage = InMemoryStorage::new();
    let provider = StandardResourceProvider::new(storage);
    let context = RequestContext::with_generated_id();

    // === Setup: Create a user ===
    let user_data = json!({
        "userName": "admin.user",
        "active": true,
        "department": "Engineering"
    });

    let created_user = provider
        .create_versioned_resource("User", user_data, &context)
        .await
        .expect("Failed to create user");

    let user_id = created_user.resource().get_id().unwrap();
    let initial_version = created_user.version().clone();

    // === Scenario: Two admins try to update the same user ===

    // Admin A: Wants to disable the user (security incident)
    let admin_a_update = json!({
        "userName": "admin.user",
        "active": false,  // ← CRITICAL: Disable user
        "department": "Engineering",
        "notes": "Disabled due to security incident"
    });

    // Admin B: Wants to change department (unaware of security incident)
    let admin_b_update = json!({
        "userName": "admin.user",
        "active": true,   // ← Admin B thinks user is still active
        "department": "Security",
        "notes": "Transferred to security team"
    });

    // === Test: Admin A updates first (disables user) ===
    let result_a = provider
        .conditional_update("User", user_id, admin_a_update, &initial_version, &context)
        .await
        .expect("Admin A update failed");

    let updated_user = match result_a {
        ConditionalResult::Success(user) => user,
        _ => panic!("Admin A update should have succeeded"),
    };

    // Verify Admin A's update worked
    assert_eq!(
        updated_user.resource().get("active").unwrap(),
        &json!(false)
    );
    assert_eq!(
        updated_user.resource().get("notes").unwrap(),
        &json!("Disabled due to security incident")
    );

    let new_version = updated_user.version().clone();
    assert!(!new_version.matches(&initial_version)); // Version should have changed

    // === Test: Admin B tries to update with stale version ===
    let result_b = provider
        .conditional_update(
            "User",
            user_id,
            admin_b_update,
            &initial_version, // ← Using OLD version (stale)
            &context,
        )
        .await
        .expect("Admin B update failed");

    // Admin B should get a version conflict
    match result_b {
        ConditionalResult::VersionMismatch(conflict) => {
            assert_eq!(conflict.expected, initial_version);
            assert_eq!(conflict.current, new_version);
            assert!(conflict.message.contains("modified by another client"));
        }
        _ => panic!("Admin B should have gotten a version conflict"),
    }

    // === Verification: User remains disabled (no data loss) ===
    let final_user = provider
        .get_versioned_resource("User", user_id, &context)
        .await
        .expect("Failed to get final user")
        .expect("User should exist");

    // The user should still be DISABLED (Admin A's critical security change preserved)
    assert_eq!(final_user.resource().get("active").unwrap(), &json!(false));
    assert_eq!(
        final_user.resource().get("notes").unwrap(),
        &json!("Disabled due to security incident")
    );
    assert_eq!(
        final_user.resource().get("department").unwrap(),
        &json!("Engineering")
    );

    // Admin B's changes should NOT have been applied
    assert_ne!(
        final_user.resource().get("department").unwrap(),
        &json!("Security")
    );
}

/// Test that shows how Admin B can properly handle the conflict and make an informed decision.
#[tokio::test]
async fn test_conflict_resolution_workflow() {
    let storage = InMemoryStorage::new();
    let provider = StandardResourceProvider::new(storage);
    let context = RequestContext::with_generated_id();

    // Setup: Create user
    let user_data = json!({
        "userName": "conflict.user",
        "active": true,
        "role": "Developer"
    });

    let created_user = provider
        .create_versioned_resource("User", user_data, &context)
        .await
        .unwrap();

    let user_id = created_user.resource().get_id().unwrap();
    let version_1 = created_user.version().clone();

    // Admin A: Promotes user to manager
    let promotion_update = json!({
        "userName": "conflict.user",
        "active": true,
        "role": "Manager",
        "team": "Backend"
    });

    let result_a = provider
        .conditional_update("User", user_id, promotion_update, &version_1, &context)
        .await
        .unwrap();

    let promoted_user = match result_a {
        ConditionalResult::Success(user) => user,
        _ => panic!("Promotion should succeed"),
    };

    let _version_2 = promoted_user.version().clone();

    // Admin B: Tries to change team (unaware of promotion)
    let team_update = json!({
        "userName": "conflict.user",
        "active": true,
        "role": "Developer", // ← Stale role
        "team": "Frontend"
    });

    // Admin B gets conflict
    let result_b = provider
        .conditional_update("User", user_id, team_update, &version_1, &context)
        .await
        .unwrap();

    assert!(matches!(result_b, ConditionalResult::VersionMismatch(_)));

    // === Proper Conflict Resolution: Admin B refreshes and tries again ===

    // 1. Admin B gets current state
    let current_user = provider
        .get_versioned_resource("User", user_id, &context)
        .await
        .unwrap()
        .unwrap();

    // 2. Admin B sees the user was promoted to Manager
    assert_eq!(
        current_user.resource().get("role").unwrap(),
        &json!("Manager")
    );

    // 3. Admin B makes informed decision: keep promotion, just change team
    let informed_update = json!({
        "userName": "conflict.user",
        "active": true,
        "role": "Manager", // ← Preserves the promotion
        "team": "Frontend" // ← Admin B's intended change
    });

    // 4. Admin B updates with current version
    let result_b_retry = provider
        .conditional_update(
            "User",
            user_id,
            informed_update,
            current_user.version(), // ← Using CURRENT version
            &context,
        )
        .await
        .unwrap();

    // 5. Success! Both changes are preserved
    let final_user = match result_b_retry {
        ConditionalResult::Success(user) => user,
        _ => panic!("Informed update should succeed"),
    };

    assert_eq!(
        final_user.resource().get("role").unwrap(),
        &json!("Manager")
    );
    assert_eq!(
        final_user.resource().get("team").unwrap(),
        &json!("Frontend")
    );
}

/// Test that concurrent delete operations are also protected by versioning.
#[tokio::test]
async fn test_conditional_delete_prevents_accidental_deletion() {
    let storage = InMemoryStorage::new();
    let provider = StandardResourceProvider::new(storage);
    let context = RequestContext::with_generated_id();

    // Setup: Create user
    let user_data = json!({
        "userName": "delete.test",
        "active": true
    });

    let created_user = provider
        .create_versioned_resource("User", user_data, &context)
        .await
        .unwrap();

    let user_id = created_user.resource().get_id().unwrap();
    let version_1 = created_user.version().clone();

    // Admin A: Updates user (adds important data)
    let update_data = json!({
        "userName": "delete.test",
        "active": true,
        "importantData": "DO NOT DELETE - contains critical audit trail"
    });

    let update_result = provider
        .conditional_update("User", user_id, update_data, &version_1, &context)
        .await
        .unwrap();

    let updated_user = match update_result {
        ConditionalResult::Success(user) => user,
        _ => panic!("Update should succeed"),
    };

    // Admin B: Tries to delete user with old version (before important data was added)
    let delete_result = provider
        .conditional_delete(
            "User", user_id, &version_1, // ← Old version, before important data was added
            &context,
        )
        .await
        .unwrap();

    // Delete should fail due to version mismatch
    match delete_result {
        ConditionalResult::VersionMismatch(conflict) => {
            assert_eq!(conflict.expected, version_1);
            assert!(conflict.current.matches(updated_user.version()));
        }
        _ => panic!("Delete should have failed with version mismatch"),
    }

    // Verify user still exists with important data
    let final_user = provider
        .get_versioned_resource("User", user_id, &context)
        .await
        .unwrap()
        .unwrap();

    assert_eq!(
        final_user.resource().get("importantData").unwrap(),
        &json!("DO NOT DELETE - contains critical audit trail")
    );
}

/// Performance test: Ensure conditional operations don't add significant overhead.
#[tokio::test]
async fn test_conditional_operations_performance() {
    let storage = InMemoryStorage::new();
    let provider = Arc::new(StandardResourceProvider::new(storage));
    let context = RequestContext::with_generated_id();

    // Create initial resource
    let user_data = json!({
        "userName": "perf.test",
        "active": true,
        "counter": 0
    });

    let created_user = provider
        .create_versioned_resource("User", user_data, &context)
        .await
        .unwrap();

    let user_id = created_user.resource().get_id().unwrap().to_string();

    // Test: Multiple sequential updates (simulating normal operation)
    let start = std::time::Instant::now();
    let mut current_version = created_user.version().clone();

    for i in 1..=100 {
        let update_data = json!({
            "userName": "perf.test",
            "active": true,
            "counter": i
        });

        let result = provider
            .conditional_update("User", &user_id, update_data, &current_version, &context)
            .await
            .unwrap();

        match result {
            ConditionalResult::Success(updated) => {
                current_version = updated.version().clone();
            }
            _ => panic!("Sequential update {} should succeed", i),
        }
    }

    let duration = start.elapsed();

    // Should complete 100 updates in reasonable time (adjust threshold as needed)
    assert!(
        duration.as_millis() < 1000,
        "100 updates took {:?}",
        duration
    );

    // Verify final state
    let final_user = provider
        .get_versioned_resource("User", &user_id, &context)
        .await
        .unwrap()
        .unwrap();

    assert_eq!(final_user.resource().get("counter").unwrap(), &json!(100));
}
