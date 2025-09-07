//! Bulk operations tests for advanced multi-tenant features.
//!
//! This module contains tests for bulk create/update/delete operations
//! with tenant isolation, error handling, and performance validation.

use super::{
    bulk_operations::{
        BulkOperation, BulkOperationRequest, BulkOperationType, MigrationStrategy,
        TenantMigrationRequest,
    },
    integration::{AdvancedMultiTenantProvider, TestAdvancedProvider},
};
use scim_server::ResourceProvider;
use scim_server::resource::{RequestContext, TenantContext};
use serde_json::json;

#[cfg(test)]
mod bulk_operations_tests {
    use super::*;

    fn create_test_context(tenant_id: &str) -> RequestContext {
        let tenant_context = TenantContext::new(tenant_id.to_string(), "test-client".to_string());
        RequestContext::with_tenant(format!("req_{}", tenant_id), tenant_context)
    }

    fn create_test_user(username: &str) -> serde_json::Value {
        json!({
            "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
            "userName": username,
            "displayName": format!("{} User", username),
            "active": true
        })
    }

    #[tokio::test]
    async fn test_bulk_create_operations() {
        let provider = TestAdvancedProvider::new();
        let tenant_id = "bulk_test_tenant";
        let context = create_test_context(tenant_id);

        let bulk_request = BulkOperationRequest {
            tenant_id: tenant_id.to_string(),
            operations: vec![
                BulkOperation {
                    operation_type: BulkOperationType::Create,
                    resource_type: "User".to_string(),
                    resource_id: None,
                    data: Some(create_test_user("bulk_user_1")),
                },
                BulkOperation {
                    operation_type: BulkOperationType::Create,
                    resource_type: "User".to_string(),
                    resource_id: None,
                    data: Some(create_test_user("bulk_user_2")),
                },
                BulkOperation {
                    operation_type: BulkOperationType::Create,
                    resource_type: "User".to_string(),
                    resource_id: None,
                    data: Some(create_test_user("bulk_user_3")),
                },
            ],
            fail_on_errors: false,
            continue_on_error: true,
        };

        let result = provider
            .execute_bulk_operation(bulk_request, &context)
            .await;
        assert!(result.is_ok());

        let bulk_result = result.unwrap();
        assert_eq!(bulk_result.total_operations, 3);
        assert_eq!(bulk_result.successful_operations, 3);
        assert_eq!(bulk_result.failed_operations, 0);

        // Verify all users were created
        let users = provider
            .list_resources("User", None, &context)
            .await
            .unwrap();
        assert_eq!(users.len(), 3);
    }

    #[tokio::test]
    async fn test_bulk_operations_with_failures() {
        let provider = TestAdvancedProvider::new();
        let tenant_id = "bulk_error_tenant";
        let context = create_test_context(tenant_id);

        // Create a user first to test duplicate prevention
        let _existing_user = provider
            .create_resource("User", create_test_user("existing_user"), &context)
            .await
            .unwrap();

        let bulk_request = BulkOperationRequest {
            tenant_id: tenant_id.to_string(),
            operations: vec![
                BulkOperation {
                    operation_type: BulkOperationType::Create,
                    resource_type: "User".to_string(),
                    resource_id: None,
                    data: Some(create_test_user("new_user")),
                },
                BulkOperation {
                    operation_type: BulkOperationType::Create,
                    resource_type: "User".to_string(),
                    resource_id: None,
                    data: Some(create_test_user("existing_user")), // This should fail
                },
                BulkOperation {
                    operation_type: BulkOperationType::Create,
                    resource_type: "User".to_string(),
                    resource_id: None,
                    data: None, // This should fail - missing data
                },
            ],
            fail_on_errors: false,
            continue_on_error: true,
        };

        let result = provider
            .execute_bulk_operation(bulk_request, &context)
            .await;
        assert!(result.is_ok());

        let bulk_result = result.unwrap();
        assert_eq!(bulk_result.total_operations, 3);
        assert_eq!(bulk_result.successful_operations, 1);
        assert_eq!(bulk_result.failed_operations, 2);

        // Verify error details
        assert!(!bulk_result.results[1].success);
        assert!(!bulk_result.results[2].success);

        assert!(
            bulk_result.results[2]
                .error
                .as_ref()
                .unwrap()
                .contains("No data provided for create operation")
        );
    }

    #[tokio::test]
    async fn test_bulk_operations_tenant_isolation() {
        let provider = TestAdvancedProvider::new();
        let tenant_a = "bulk_tenant_a";
        let tenant_b = "bulk_tenant_b";
        let context_a = create_test_context(tenant_a);
        let context_b = create_test_context(tenant_b);

        // Create users in tenant A via bulk operation
        let bulk_request_a = BulkOperationRequest {
            tenant_id: tenant_a.to_string(),
            operations: vec![
                BulkOperation {
                    operation_type: BulkOperationType::Create,
                    resource_type: "User".to_string(),
                    resource_id: None,
                    data: Some(create_test_user("user_a1")),
                },
                BulkOperation {
                    operation_type: BulkOperationType::Create,
                    resource_type: "User".to_string(),
                    resource_id: None,
                    data: Some(create_test_user("user_a2")),
                },
            ],
            fail_on_errors: false,
            continue_on_error: true,
        };

        let _result_a = provider
            .execute_bulk_operation(bulk_request_a, &context_a)
            .await
            .unwrap();

        // Create users in tenant B via bulk operation
        let bulk_request_b = BulkOperationRequest {
            tenant_id: tenant_b.to_string(),
            operations: vec![BulkOperation {
                operation_type: BulkOperationType::Create,
                resource_type: "User".to_string(),
                resource_id: None,
                data: Some(create_test_user("user_b1")),
            }],
            fail_on_errors: false,
            continue_on_error: true,
        };

        let _result_b = provider
            .execute_bulk_operation(bulk_request_b, &context_b)
            .await
            .unwrap();

        // Verify tenant isolation
        let users_a = provider
            .list_resources("User", None, &context_a)
            .await
            .unwrap();
        let users_b = provider
            .list_resources("User", None, &context_b)
            .await
            .unwrap();

        assert_eq!(users_a.len(), 2);
        assert_eq!(users_b.len(), 1);

        // Verify usernames are isolated
        let usernames_a: Vec<String> = users_a
            .iter()
            .map(|u| u.get_username().unwrap().to_string())
            .collect();
        let usernames_b: Vec<String> = users_b
            .iter()
            .map(|u| u.get_username().unwrap().to_string())
            .collect();

        assert!(usernames_a.contains(&"user_a1".to_string()));
        assert!(usernames_a.contains(&"user_a2".to_string()));
        assert!(usernames_b.contains(&"user_b1".to_string()));
        assert!(!usernames_a.contains(&"user_b1".to_string()));
        assert!(!usernames_b.contains(&"user_a1".to_string()));
    }

    #[tokio::test]
    async fn test_bulk_operation_fail_fast() {
        let provider = TestAdvancedProvider::new();
        let tenant_id = "fail_fast_tenant";
        let context = create_test_context(tenant_id);

        let bulk_request = BulkOperationRequest {
            tenant_id: tenant_id.to_string(),
            operations: vec![
                BulkOperation {
                    operation_type: BulkOperationType::Create,
                    resource_type: "User".to_string(),
                    resource_id: None,
                    data: Some(create_test_user("user1")),
                },
                BulkOperation {
                    operation_type: BulkOperationType::Create,
                    resource_type: "User".to_string(),
                    resource_id: None,
                    data: None, // This will fail
                },
                BulkOperation {
                    operation_type: BulkOperationType::Create,
                    resource_type: "User".to_string(),
                    resource_id: None,
                    data: Some(create_test_user("user3")),
                },
            ],
            fail_on_errors: true, // Enable fail-fast
            continue_on_error: false,
        };

        let result = provider
            .execute_bulk_operation(bulk_request, &context)
            .await
            .unwrap();

        // Should stop after the first failure
        assert_eq!(result.successful_operations, 1);
        assert_eq!(result.failed_operations, 1);
        assert_eq!(result.results.len(), 2); // Only processed first 2 operations
    }

    #[tokio::test]
    async fn test_bulk_update_operations() {
        let provider = TestAdvancedProvider::new();
        let tenant_id = "bulk_update_tenant";
        let context = create_test_context(tenant_id);

        // Create initial users
        let user1 = provider
            .create_resource("User", create_test_user("update_user1"), &context)
            .await
            .unwrap();
        let user2 = provider
            .create_resource("User", create_test_user("update_user2"), &context)
            .await
            .unwrap();

        // Prepare bulk update request
        let bulk_request = BulkOperationRequest {
            tenant_id: tenant_id.to_string(),
            operations: vec![
                BulkOperation {
                    operation_type: BulkOperationType::Update,
                    resource_type: "User".to_string(),
                    resource_id: Some(user1.get_id().unwrap().to_string()),
                    data: Some(json!({
                        "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
                        "userName": "update_user1",
                        "displayName": "Updated User 1",
                        "active": false
                    })),
                },
                BulkOperation {
                    operation_type: BulkOperationType::Update,
                    resource_type: "User".to_string(),
                    resource_id: Some(user2.get_id().unwrap().to_string()),
                    data: Some(json!({
                        "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
                        "userName": "update_user2",
                        "displayName": "Updated User 2",
                        "active": false
                    })),
                },
            ],
            fail_on_errors: false,
            continue_on_error: true,
        };

        let result = provider
            .execute_bulk_operation(bulk_request, &context)
            .await
            .unwrap();

        assert_eq!(result.successful_operations, 2);
        assert_eq!(result.failed_operations, 0);

        // Verify updates were applied
        let updated_user1 = provider
            .get_resource("User", user1.get_id().unwrap(), &context)
            .await
            .unwrap()
            .unwrap();

        assert_eq!(
            updated_user1
                .resource()
                .get_attribute("displayName")
                .unwrap(),
            &json!("Updated User 1")
        );
        assert_eq!(
            updated_user1.get_attribute("active").unwrap(),
            &json!(false)
        );
    }

    #[tokio::test]
    async fn test_bulk_delete_operations() {
        let provider = TestAdvancedProvider::new();
        let tenant_id = "bulk_delete_tenant";
        let context = create_test_context(tenant_id);

        // Create initial users
        let user1 = provider
            .create_resource("User", create_test_user("delete_user1"), &context)
            .await
            .unwrap();
        let user2 = provider
            .create_resource("User", create_test_user("delete_user2"), &context)
            .await
            .unwrap();
        let user3 = provider
            .create_resource("User", create_test_user("delete_user3"), &context)
            .await
            .unwrap();

        // Prepare bulk delete request
        let bulk_request = BulkOperationRequest {
            tenant_id: tenant_id.to_string(),
            operations: vec![
                BulkOperation {
                    operation_type: BulkOperationType::Delete,
                    resource_type: "User".to_string(),
                    resource_id: Some(user1.get_id().unwrap().to_string()),
                    data: None,
                },
                BulkOperation {
                    operation_type: BulkOperationType::Delete,
                    resource_type: "User".to_string(),
                    resource_id: Some(user2.resource().get_id().unwrap().to_string()),
                    data: None,
                },
            ],
            fail_on_errors: false,
            continue_on_error: true,
        };

        let result = provider
            .execute_bulk_operation(bulk_request, &context)
            .await
            .unwrap();

        assert_eq!(result.successful_operations, 2);
        assert_eq!(result.failed_operations, 0);

        // Verify deletions
        let remaining_users = provider
            .list_resources("User", None, &context)
            .await
            .unwrap();

        assert_eq!(remaining_users.len(), 1);
        assert_eq!(
            remaining_users[0].resource().get_id().unwrap(),
            user3.resource().get_id().unwrap()
        );
    }

    #[tokio::test]
    async fn test_bulk_mixed_operations() {
        let provider = TestAdvancedProvider::new();
        let tenant_id = "bulk_mixed_tenant";
        let context = create_test_context(tenant_id);

        // Create an initial user for update/delete
        let existing_user = provider
            .create_resource("User", create_test_user("existing_user"), &context)
            .await
            .unwrap();

        let bulk_request = BulkOperationRequest {
            tenant_id: tenant_id.to_string(),
            operations: vec![
                // Create operation
                BulkOperation {
                    operation_type: BulkOperationType::Create,
                    resource_type: "User".to_string(),
                    resource_id: None,
                    data: Some(create_test_user("new_user")),
                },
                // Update operation
                BulkOperation {
                    operation_type: BulkOperationType::Update,
                    resource_type: "User".to_string(),
                    resource_id: Some(existing_user.resource().get_id().unwrap().to_string()),
                    data: Some(json!({
                        "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
                        "userName": "existing_user",
                        "displayName": "Updated Existing User",
                        "active": true
                    })),
                },
                // Delete operation (create another user first to delete)
                BulkOperation {
                    operation_type: BulkOperationType::Create,
                    resource_type: "User".to_string(),
                    resource_id: None,
                    data: Some(create_test_user("temp_user")),
                },
            ],
            fail_on_errors: false,
            continue_on_error: true,
        };

        let result = provider
            .execute_bulk_operation(bulk_request, &context)
            .await
            .unwrap();

        assert_eq!(result.successful_operations, 3);
        assert_eq!(result.failed_operations, 0);

        // Verify final state
        let users = provider
            .list_resources("User", None, &context)
            .await
            .unwrap();

        assert_eq!(users.len(), 3); // existing_user (updated), new_user, temp_user

        let updated_user = users
            .iter()
            .find(|u| u.resource().get_username().unwrap() == "existing_user")
            .unwrap();
        assert_eq!(
            updated_user
                .resource()
                .get_attribute("displayName")
                .unwrap(),
            &json!("Updated Existing User")
        );
    }

    #[tokio::test]
    async fn test_tenant_migration_basic() {
        let provider = TestAdvancedProvider::new();
        let source_context = create_test_context("source_tenant");
        let _target_context = create_test_context("target_tenant");

        // Create some data in source tenant
        let _user1 = provider
            .create_resource("User", create_test_user("migrate_user1"), &source_context)
            .await
            .unwrap();
        let _user2 = provider
            .create_resource("User", create_test_user("migrate_user2"), &source_context)
            .await
            .unwrap();

        // Test migration request (simplified - actual implementation would vary)
        let migration_request = TenantMigrationRequest {
            source_tenant_id: "source_tenant".to_string(),
            target_tenant_id: "target_tenant".to_string(),
            resource_types: vec!["User".to_string()],
            migration_strategy: MigrationStrategy::Copy,
            preserve_ids: false,
        };

        let result = provider
            .migrate_tenant_data(migration_request, &source_context)
            .await;

        // For now, just verify the operation completes without error
        // In a real implementation, this would copy data between tenants
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_bulk_operation_performance_tracking() {
        let provider = TestAdvancedProvider::new();
        let tenant_id = "performance_tenant";
        let context = create_test_context(tenant_id);

        let bulk_request = BulkOperationRequest {
            tenant_id: tenant_id.to_string(),
            operations: vec![
                BulkOperation {
                    operation_type: BulkOperationType::Create,
                    resource_type: "User".to_string(),
                    resource_id: None,
                    data: Some(create_test_user("perf_user1")),
                },
                BulkOperation {
                    operation_type: BulkOperationType::Create,
                    resource_type: "User".to_string(),
                    resource_id: None,
                    data: Some(create_test_user("perf_user2")),
                },
            ],
            fail_on_errors: false,
            continue_on_error: true,
        };

        let start_time = std::time::Instant::now();
        let result = provider
            .execute_bulk_operation(bulk_request, &context)
            .await
            .unwrap();
        let elapsed = start_time.elapsed();

        // Verify performance tracking
        assert!(result.duration > std::time::Duration::from_nanos(1));
        assert!(result.duration <= elapsed);
        assert_eq!(result.successful_operations, 2);
    }
}
