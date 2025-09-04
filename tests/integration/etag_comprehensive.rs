//! Comprehensive ETag concurrency control integration tests.
//!
//! This module contains real-world scenarios and edge cases for ETag-based
//! concurrency control in SCIM operations. These tests validate the robustness
//! and practical usability of the versioning system.

use scim_server::providers::StandardResourceProvider;
use scim_server::resource::{
    core::RequestContext,
    version::{ConditionalResult, HttpVersion, RawVersion},
};
use scim_server::storage::InMemoryStorage;
use serde_json::json;
use std::sync::Arc;
use tokio::task::JoinSet;

/// Test HTTP header integration with realistic ETag scenarios
#[tokio::test]
async fn test_http_etag_roundtrip_scenarios() {
    let storage = InMemoryStorage::new();
    let provider = StandardResourceProvider::new(storage);
    let context = RequestContext::with_generated_id();

    // Create user with complex data
    let user_data = json!({
        "userName": "etag.test@company.com",
        "name": {
            "familyName": "Test",
            "givenName": "ETag",
            "formatted": "ETag Test"
        },
        "emails": [
            {
                "value": "etag.test@company.com",
                "type": "work",
                "primary": true
            }
        ],
        "active": true,
        "groups": [],
        "roles": ["developer", "admin"]
    });

    let created_user = provider
        .create_versioned_resource("User", user_data, &context)
        .await
        .expect("Failed to create user");

    let user_id = created_user.resource().get_id().unwrap();
    let version = created_user.version();

    // Test ETag header conversion
    let etag_header = HttpVersion::from(version.clone()).to_string();
    assert!(etag_header.starts_with("W/\""));
    assert!(etag_header.ends_with('"'));
    assert!(etag_header.contains("W/")); // Should be weak ETag

    // Parse ETag back from header
    let parsed_version: HttpVersion = etag_header.parse().expect("Failed to parse ETag header");
    assert!(*version == parsed_version);

    // Test conditional update with ETag
    let update_data = json!({
        "userName": "etag.test@company.com",
        "name": {
            "familyName": "Test",
            "givenName": "ETag",
            "formatted": "ETag Test",
            "middleName": "HTTP"
        },
        "emails": [
            {
                "value": "etag.test@company.com",
                "type": "work",
                "primary": true
            }
        ],
        "active": true,
        "groups": [],
        "roles": ["developer", "admin", "architect"]
    });

    let update_result = provider
        .conditional_update(
            "User",
            user_id,
            update_data,
            &RawVersion::from(parsed_version),
            &context,
        )
        .await
        .expect("Update operation failed");

    match update_result {
        ConditionalResult::Success(updated_user) => {
            let resource_json = updated_user
                .resource()
                .to_json()
                .expect("Should convert to JSON");
            assert_eq!(
                resource_json
                    .get("name")
                    .unwrap()
                    .get("middleName")
                    .unwrap(),
                &json!("HTTP")
            );
            assert_eq!(
                resource_json
                    .get("roles")
                    .unwrap()
                    .as_array()
                    .unwrap()
                    .len(),
                3
            );
        }
        _ => panic!("Update should have succeeded"),
    }
}

/// Test concurrent modification simulation with multiple users
#[tokio::test]
async fn test_multi_user_concurrent_modification() {
    let storage = InMemoryStorage::new();
    let provider = Arc::new(StandardResourceProvider::new(storage));
    let context = RequestContext::with_generated_id();

    // Create a shared group resource
    let group_data = json!({
        "displayName": "Engineering Team",
        "members": [
            {
                "value": "user1",
                "display": "Alice Developer"
            },
            {
                "value": "user2",
                "display": "Bob Engineer"
            }
        ],
        "meta": {
            "resourceType": "Group"
        }
    });

    let created_group = provider
        .create_versioned_resource("Group", group_data, &context)
        .await
        .expect("Failed to create group");

    let group_id = created_group
        .resource()
        .get_id()
        .expect("Created group should have an ID")
        .to_string();
    let initial_version = created_group.version().clone();

    // Simulate 3 administrators trying to modify the group concurrently
    let mut join_set = JoinSet::new();

    // Admin 1: Add new member
    let provider_1 = Arc::clone(&provider);
    let group_id_1 = group_id.clone();
    let version_1 = initial_version.clone();
    let context_1 = context.clone();
    join_set.spawn(async move {
        let new_member_data = json!({
            "displayName": "Engineering Team",
            "members": [
                {
                    "value": "user1",
                    "display": "Alice Developer"
                },
                {
                    "value": "user2",
                    "display": "Bob Engineer"
                },
                {
                    "value": "user3",
                    "display": "Charlie Architect"
                }
            ],
            "meta": {
                "resourceType": "Group"
            }
        });

        provider_1
            .conditional_update(
                "Group",
                &group_id_1,
                new_member_data,
                &version_1,
                &context_1,
            )
            .await
    });

    // Admin 2: Change group name
    let provider_2 = Arc::clone(&provider);
    let group_id_2 = group_id.clone();
    let version_2 = initial_version.clone();
    let context_2 = context.clone();
    join_set.spawn(async move {
        let rename_data = json!({
            "displayName": "Senior Engineering Team",
            "members": [
                {
                    "value": "user1",
                    "display": "Alice Developer"
                },
                {
                    "value": "user2",
                    "display": "Bob Engineer"
                }
            ],
            "meta": {
                "resourceType": "Group"
            }
        });

        provider_2
            .conditional_update("Group", &group_id_2, rename_data, &version_2, &context_2)
            .await
    });

    // Admin 3: Remove a member
    let provider_3 = Arc::clone(&provider);
    let group_id_3 = group_id.clone();
    let version_3 = initial_version.clone();
    let context_3 = context.clone();
    join_set.spawn(async move {
        let remove_member_data = json!({
            "displayName": "Engineering Team",
            "members": [
                {
                    "value": "user1",
                    "display": "Alice Developer"
                }
            ],
            "meta": {
                "resourceType": "Group"
            }
        });

        provider_3
            .conditional_update(
                "Group",
                &group_id_3,
                remove_member_data,
                &version_3,
                &context_3,
            )
            .await
    });

    // Collect results
    let mut results = Vec::new();
    while let Some(result) = join_set.join_next().await {
        results.push(result.expect("Task panicked").expect("Operation failed"));
    }

    // Exactly one operation should succeed, two should fail with version conflicts
    let successes = results.iter().filter(|r| r.is_success()).count();
    let conflicts = results.iter().filter(|r| r.is_version_mismatch()).count();

    assert_eq!(successes, 1, "Exactly one operation should succeed");
    assert_eq!(
        conflicts, 2,
        "Two operations should fail with version conflicts"
    );

    // Verify the final state is consistent
    let final_group = provider
        .get_versioned_resource("Group", &group_id, &context)
        .await
        .expect("Failed to get final group")
        .expect("Group should exist");

    // The final group should have one of the three intended modifications
    let final_group_json = final_group
        .resource()
        .to_json()
        .expect("Should convert to JSON");
    let display_name = final_group_json
        .get("displayName")
        .unwrap()
        .as_str()
        .unwrap();
    let members = final_group_json.get("members").unwrap().as_array().unwrap();

    // Verify it's one of the three possible outcomes
    let is_valid_outcome = (display_name == "Engineering Team" && members.len() == 3) ||  // Admin 1 won
        (display_name == "Senior Engineering Team" && members.len() == 2) ||  // Admin 2 won
        (display_name == "Engineering Team" && members.len() == 1); // Admin 3 won

    assert!(
        is_valid_outcome,
        "Final state should match one of the three operations"
    );
}

/// Test version conflict resolution workflow
#[tokio::test]
async fn test_comprehensive_conflict_resolution() {
    let storage = InMemoryStorage::new();
    let provider = StandardResourceProvider::new(storage);
    let context = RequestContext::with_generated_id();

    // Create user with initial state
    let initial_data = json!({
        "userName": "conflict.resolution@test.com",
        "name": {
            "givenName": "John",
            "familyName": "Doe"
        },
        "active": true,
        "department": "Engineering",
        "title": "Developer",
        "phoneNumbers": [
            {
                "value": "555-1234",
                "type": "work"
            }
        ]
    });

    let created_user = provider
        .create_versioned_resource("User", initial_data, &context)
        .await
        .expect("Failed to create user");

    let user_id = created_user.resource().get_id().unwrap();
    let version_1 = created_user.version().clone();

    // First update: HR changes title and department
    let hr_update = json!({
        "userName": "conflict.resolution@test.com",
        "name": {
            "givenName": "John",
            "familyName": "Doe"
        },
        "active": true,
        "department": "Senior Engineering",
        "title": "Senior Developer",
        "phoneNumbers": [
            {
                "value": "555-1234",
                "type": "work"
            }
        ]
    });

    let hr_result = provider
        .conditional_update("User", user_id, hr_update, &version_1, &context)
        .await
        .expect("HR update failed");

    let promoted_user = match hr_result {
        ConditionalResult::Success(user) => user,
        _ => panic!("HR update should succeed"),
    };

    let version_2 = promoted_user.version().clone();

    // Second update attempt: IT tries to update phone with stale version
    let it_update = json!({
        "userName": "conflict.resolution@test.com",
        "name": {
            "givenName": "John",
            "familyName": "Doe"
        },
        "active": true,
        "department": "Engineering",  // Stale value
        "title": "Developer",         // Stale value
        "phoneNumbers": [
            {
                "value": "555-5678",
                "type": "work"
            },
            {
                "value": "555-9012",
                "type": "mobile"
            }
        ]
    });

    let it_result = provider
        .conditional_update("User", user_id, it_update, &version_1, &context) // Using old version
        .await
        .expect("IT update operation failed");

    // Should get version conflict
    let conflict = match it_result {
        ConditionalResult::VersionMismatch(conflict) => conflict,
        _ => panic!("IT update should fail with version conflict"),
    };

    assert_eq!(conflict.expected, version_1);
    assert_eq!(conflict.current, version_2);
    assert!(conflict.message.contains("modified"));

    // IT resolves conflict by getting current state and merging changes
    let current_user = provider
        .get_versioned_resource("User", user_id, &context)
        .await
        .expect("Failed to get current user")
        .expect("User should exist");

    // IT creates merged update preserving HR changes
    let merged_update = json!({
        "userName": "conflict.resolution@test.com",
        "name": {
            "givenName": "John",
            "familyName": "Doe"
        },
        "active": true,
        "department": "Senior Engineering",  // Preserved from HR
        "title": "Senior Developer",         // Preserved from HR
        "phoneNumbers": [
            {
                "value": "555-5678",
                "type": "work"
            },
            {
                "value": "555-9012",
                "type": "mobile"
            }
        ]
    });

    let merged_result = provider
        .conditional_update(
            "User",
            user_id,
            merged_update,
            current_user.version(),
            &context,
        )
        .await
        .expect("Merged update failed");

    let final_user = match merged_result {
        ConditionalResult::Success(user) => user,
        _ => panic!("Merged update should succeed"),
    };

    // Verify both changes are preserved
    let final_json = final_user
        .resource()
        .to_json()
        .expect("Should convert to JSON");
    assert_eq!(final_json.get("title").unwrap(), &json!("Senior Developer"));
    assert_eq!(
        final_json.get("department").unwrap(),
        &json!("Senior Engineering")
    );
    assert_eq!(
        final_json
            .get("phoneNumbers")
            .unwrap()
            .as_array()
            .unwrap()
            .len(),
        2
    );
}

/// Test delete operation concurrency scenarios
#[tokio::test]
async fn test_conditional_delete_scenarios() {
    let storage = InMemoryStorage::new();
    let provider = StandardResourceProvider::new(storage);
    let context = RequestContext::with_generated_id();

    // Create temporary user
    let temp_user_data = json!({
        "userName": "temp.user@test.com",
        "active": true,
        "temporary": true
    });

    let created_user = provider
        .create_versioned_resource("User", temp_user_data, &context)
        .await
        .expect("Failed to create temp user");

    let user_id = created_user.resource().get_id().unwrap();
    let version_1 = created_user.version().clone();

    // Admin A: Adds important audit data
    let audit_update = json!({
        "userName": "temp.user@test.com",
        "active": true,
        "temporary": true,
        "auditTrail": [
            {
                "action": "login",
                "timestamp": "2024-01-15T10:30:00Z",
                "ip": "192.168.1.100"
            },
            {
                "action": "dataAccess",
                "timestamp": "2024-01-15T10:35:00Z",
                "resource": "sensitive-document-123"
            }
        ]
    });

    let audit_result = provider
        .conditional_update("User", user_id, audit_update, &version_1, &context)
        .await
        .expect("Audit update failed");

    let updated_user = match audit_result {
        ConditionalResult::Success(user) => user,
        _ => panic!("Audit update should succeed"),
    };

    // Admin B: Tries to delete user with old version (before audit data added)
    let delete_result = provider
        .conditional_delete("User", user_id, &version_1, &context)
        .await
        .expect("Delete operation failed");

    // Delete should fail due to version mismatch
    match delete_result {
        ConditionalResult::VersionMismatch(conflict) => {
            println!("✅ Version mismatch detected as expected");
            assert_eq!(conflict.expected, version_1);
            assert_eq!(conflict.current, *updated_user.version());
        }
        _ => panic!("Delete should fail with version conflict"),
    }

    // Verify user still exists with audit data
    let preserved_user = provider
        .get_versioned_resource("User", user_id, &context)
        .await
        .expect("Failed to get preserved user")
        .expect("User should still exist");

    let preserved_json = preserved_user
        .resource()
        .to_json()
        .expect("Should convert to JSON");
    assert!(preserved_json.get("auditTrail").is_some());

    // Admin B gets current version and deletes with proper version
    let proper_delete_result = provider
        .conditional_delete("User", user_id, preserved_user.version(), &context)
        .await
        .expect("Proper delete failed");

    match proper_delete_result {
        ConditionalResult::Success(_) => {
            // Delete succeeded
        }
        _ => panic!("Delete with correct version should succeed"),
    }

    // Verify user is deleted
    let deleted_user = provider
        .get_versioned_resource("User", user_id, &context)
        .await
        .expect("Get operation failed");

    assert!(deleted_user.is_none(), "User should be deleted");
}

/// Test edge cases with malformed or unusual data
#[tokio::test]
async fn test_etag_edge_cases() {
    let storage = InMemoryStorage::new();
    let provider = StandardResourceProvider::new(storage);
    let context = RequestContext::with_generated_id();

    // Test with empty arrays and edge case values
    let edge_case_data = json!({
        "userName": "edge.case@test.com",
        "active": true,
        "emails": [],
        "phoneNumbers": [],
        "name": {
            "givenName": "",
            "familyName": "Test"
        },
        "groups": []
    });

    let created_user = provider
        .create_versioned_resource("User", edge_case_data, &context)
        .await
        .expect("Failed to create edge case user");

    let user_id = created_user.resource().get_id().unwrap();
    let version = created_user.version();

    // Test update with Unicode characters
    let unicode_update = json!({
        "userName": "edge.case@test.com",
        "active": true,
        "emails": [],
        "phoneNumbers": [],
        "name": {
            "givenName": "测试",
            "familyName": "Test",
            "honorificPrefix": "Dr.",
            "honorificSuffix": "PhD"
        },
        "groups": [],
        "preferredLanguage": "zh-CN",
        "locale": "zh_CN",
        "title": "软件工程师 🚀"
    });

    let unicode_result = provider
        .conditional_update("User", user_id, unicode_update, version, &context)
        .await
        .expect("Unicode update failed");

    match unicode_result {
        ConditionalResult::Success(updated_user) => {
            let updated_json = updated_user
                .resource()
                .to_json()
                .expect("Should convert to JSON");
            assert_eq!(
                updated_json.get("name").unwrap().get("givenName").unwrap(),
                &json!("测试")
            );
            assert_eq!(updated_json.get("title").unwrap(), &json!("软件工程师 🚀"));

            // Verify ETag generation works with Unicode content
            let unicode_etag = HttpVersion::from(updated_user.version().clone()).to_string();
            let parsed_unicode_version: HttpVersion =
                unicode_etag.parse().expect("Failed to parse Unicode ETag");
            assert!(updated_user.version() == &parsed_unicode_version);
        }
        _ => panic!("Unicode update should succeed"),
    }
}

/// Test version stability across serialization boundaries
#[tokio::test]
async fn test_version_serialization_stability() {
    let storage = InMemoryStorage::new();
    let provider = StandardResourceProvider::new(storage);
    let context = RequestContext::with_generated_id();

    // Create user with complex nested structure
    let complex_data = json!({
        "userName": "serialization.test@example.com",
        "name": {
            "formatted": "Jane Smith-Johnson",
            "givenName": "Jane",
            "familyName": "Smith-Johnson",
            "middleName": "Marie"
        },
        "emails": [
            {
                "value": "jane@work.com",
                "type": "work",
                "primary": true
            },
            {
                "value": "jane@personal.com",
                "type": "home",
                "primary": false
            }
        ],
        "addresses": [
            {
                "streetAddress": "123 Main St",
                "locality": "Anytown",
                "region": "CA",
                "postalCode": "12345",
                "country": "US",
                "type": "work",
                "primary": true
            }
        ],
        "phoneNumbers": [
            {
                "value": "+1-555-123-4567",
                "type": "work"
            }
        ],
        "active": true
    });

    let created_user = provider
        .create_versioned_resource("User", complex_data.clone(), &context)
        .await
        .expect("Failed to create complex user");

    let original_version = created_user.version().clone();

    // Serialize and deserialize the version
    let serialized = serde_json::to_string(&original_version).expect("Failed to serialize version");
    let deserialized: RawVersion =
        serde_json::from_str(&serialized).expect("Failed to deserialize version");

    assert!(original_version == deserialized);

    // Test that content-based versions are deterministic across re-creation
    let content_bytes = b"deterministic content test";
    let content_version_1 = RawVersion::from_content(content_bytes);
    let content_version_2 = RawVersion::from_content(content_bytes);

    assert!(content_version_1 == content_version_2);

    // Test ETag round-trip stability
    let etag_header = HttpVersion::from(original_version.clone()).to_string();
    let etag_version: HttpVersion = etag_header.parse().expect("Failed to parse ETag");
    let second_etag = etag_version.to_string();

    assert_eq!(etag_header, second_etag);
    assert!(original_version == etag_version);
}

/// Test performance characteristics under load
#[tokio::test]
async fn test_etag_performance_under_load() {
    let storage = InMemoryStorage::new();
    let provider = Arc::new(StandardResourceProvider::new(storage));
    let context = RequestContext::with_generated_id();

    // Create initial resources
    let mut initial_resources = Vec::new();
    for i in 0..50 {
        let user_data = json!({
            "userName": format!("perf.user.{}@test.com", i),
            "active": true,
            "employeeNumber": format!("EMP{:04}", i),
            "department": "Performance Testing"
        });

        let created = provider
            .create_versioned_resource("User", user_data, &context)
            .await
            .expect("Failed to create performance test user");

        initial_resources.push((
            created
                .resource()
                .get_id()
                .expect("Created user should have an ID")
                .to_string(),
            created.version().clone(),
        ));
    }

    // Perform concurrent updates
    let start_time = std::time::Instant::now();
    let mut join_set = JoinSet::new();

    for (user_id, version) in initial_resources {
        let provider_clone = Arc::clone(&provider);
        let context_clone = context.clone();

        join_set.spawn(async move {
            let update_data = json!({
                "userName": format!("updated.{}", user_id),
                "active": true,
                "employeeNumber": format!("UPD-{}", user_id),
                "department": "Performance Testing Updated",
                "lastModified": "2024-01-15T12:00:00Z"
            });

            provider_clone
                .conditional_update("User", &user_id, update_data, &version, &context_clone)
                .await
        });
    }

    // Wait for all updates to complete
    let mut successful_updates = 0;
    while let Some(result) = join_set.join_next().await {
        let update_result = result
            .expect("Task panicked")
            .expect("Update operation failed");
        if update_result.is_success() {
            successful_updates += 1;
        }
    }

    let duration = start_time.elapsed();

    // Performance assertions
    assert_eq!(successful_updates, 50, "All updates should succeed");
    assert!(
        duration.as_millis() < 5000,
        "50 concurrent updates should complete in under 5 seconds, took {:?}",
        duration
    );

    println!("Completed 50 concurrent updates in {:?}", duration);
}
