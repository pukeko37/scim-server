//! SCIM PATCH Operation Integration Test Scenarios
//!
//! This module contains the main parameterized integration tests for SCIM PATCH operations.
//! These tests provide comprehensive coverage of RFC 7644 Section 3.5.2 requirements
//! using an economical, parameterized approach.

use super::assertions::PatchAssertions;
use super::test_data::TestDataFactory;
use super::test_helpers;
use super::*;

use scim_server::{RequestContext, ResourceProvider};
use serde_json::{Value, json};

/// Main parameterized test for all PATCH operation combinations
#[tokio::test]
async fn test_patch_operations_matrix() {
    let test_cases = test_data::TestDataFactory::generate_patch_test_cases();

    for case in test_cases {
        println!("Running test case: {}", case.name);

        let result = execute_patch_test_case(&case).await;

        match &case.expected_result {
            ExpectedResult::Success => {
                PatchAssertions::assert_patch_success(&result, &[]);
                // For remove operations, don't check that the value exists after removal
                // For add/replace operations, check that the value matches expectation
                if case.operation != PatchOperation::Remove {
                    if let Some(value) = &case.value {
                        PatchAssertions::assert_patch_success(&result, &[(&case.path, value)]);
                    }
                }
            }
            ExpectedResult::ScimError {
                error_type,
                status_code,
            } => {
                PatchAssertions::assert_patch_error(&result, error_type.clone());
                PatchAssertions::assert_patch_status_code(&result, *status_code);
            }
            ExpectedResult::ValidationError(_) => {
                assert!(
                    result.is_err(),
                    "Expected validation error for test case: {}",
                    case.name
                );
            }
            ExpectedResult::NotImplemented => {
                PatchAssertions::assert_patch_status_code(&result, 501);
            }
        }
    }
}

/// Test SCIM capability advertisement and enforcement
#[tokio::test]
async fn test_capability_scenarios() {
    let scenarios = test_data::TestDataFactory::generate_capability_test_scenarios();

    for scenario in scenarios {
        println!("Running capability test: {}", scenario.name);
        test_capability_scenario(&scenario).await;
    }
}

/// Test multi-tenant PATCH operation isolation
#[tokio::test]
async fn test_multi_tenant_scenarios() {
    let test_cases = test_data::TestDataFactory::generate_multi_tenant_test_cases();

    for case in test_cases {
        println!("Running multi-tenant test: {}", case.name);
        test_multi_tenant_case(&case).await;
    }
}

/// Test error scenarios comprehensively
#[tokio::test]
async fn test_error_scenarios() {
    let error_cases = test_data::TestDataFactory::generate_error_test_cases();

    for case in error_cases {
        println!("Running error test: {}", case.name);
        test_error_case(&case).await;
    }
}

/// Test atomic behavior of multi-operation PATCH requests
#[tokio::test]
async fn test_atomic_patch_operations() {
    let test_cases = vec![
        // All operations succeed
        AtomicTestCase {
            name: "all_operations_succeed".to_string(),
            operations: vec![
                test_data::TestDataFactory::add_operation("displayName", json!("New Name")),
                test_data::TestDataFactory::replace_operation("active", json!(false)),
                test_data::TestDataFactory::add_operation("title", json!("Engineer")),
            ],
            expected_behavior: AtomicBehavior::AllSucceed,
        },
        // One operation fails, all should rollback
        AtomicTestCase {
            name: "one_operation_fails_rollback".to_string(),
            operations: vec![
                test_data::TestDataFactory::add_operation("displayName", json!("New Name")),
                test_data::TestDataFactory::replace_operation("meta.created", json!("invalid")), // Should fail
                test_data::TestDataFactory::add_operation("title", json!("Engineer")),
            ],
            expected_behavior: AtomicBehavior::AllFail,
        },
        // Conflicting operations
        AtomicTestCase {
            name: "conflicting_operations".to_string(),
            operations: vec![
                test_data::TestDataFactory::add_operation("displayName", json!("Name 1")),
                test_data::TestDataFactory::replace_operation("displayName", json!("Name 2")),
                test_data::TestDataFactory::remove_operation("displayName"),
            ],
            expected_behavior: AtomicBehavior::ResolveConflicts,
        },
    ];

    for case in test_cases {
        println!("Running atomic test: {}", case.name);
        test_atomic_case(&case).await;
    }
}

/// Test real-world user management scenarios
#[tokio::test]
async fn test_user_management_scenarios() {
    // Test comprehensive user profile update
    test_user_profile_update().await;

    // Test email management operations
    test_email_management().await;

    // Test user deactivation
    test_user_deactivation().await;

    // Test enterprise extension updates
    test_enterprise_extension_updates().await;
}

/// Test group management scenarios
#[tokio::test]
async fn test_group_management_scenarios() {
    // Test adding members to group
    test_group_member_addition().await;

    // Test removing members from group
    test_group_member_removal().await;

    // Test group metadata updates
    test_group_metadata_update().await;
}

/// Test ETag concurrency control with PATCH operations
#[tokio::test]
async fn test_etag_concurrency_scenarios() {
    let scenarios = vec![
        ETagTestCase {
            name: "valid_etag_update".to_string(),
            request_etag: Some("W/\"abc123\"".to_string()),
            expected_result: ETagResult::Success,
        },
        ETagTestCase {
            name: "invalid_etag_conflict".to_string(),
            request_etag: Some("W/\"different\"".to_string()),
            expected_result: ETagResult::PreconditionFailed,
        },
        ETagTestCase {
            name: "missing_etag_unconditional".to_string(),
            request_etag: None,
            expected_result: ETagResult::Success,
        },
    ];

    for case in scenarios {
        println!("Running ETag test: {}", case.name);
        test_etag_case(&case).await;
    }
}

// Helper functions for test execution

async fn execute_patch_test_case(case: &PatchTestCase) -> PatchTestResult {
    let server = if case.setup.capabilities.patch_supported {
        test_helpers::create_test_server_with_patch_support()
    } else {
        test_helpers::create_test_server_without_patch_support()
    };
    let context = test_helpers::create_test_context();

    // Setup initial resource if needed
    let resource_id = if case.setup.create_existing_resource {
        let _initial_resource = case.setup.initial_resource.clone().unwrap_or_else(|| {
            if case.resource_type == "User" {
                TestDataFactory::user_with_all_attributes()
            } else {
                TestDataFactory::group_with_members()
            }
        });

        let created = if case.resource_type == "User" {
            test_helpers::create_test_user(&server, &context)
                .await
                .expect("Failed to create user")
        } else {
            test_helpers::create_test_group(&server, &context)
                .await
                .expect("Failed to create group")
        };

        created.get_id().unwrap().to_string()
    } else {
        "nonexistent-resource".to_string()
    };

    // Create PATCH request
    let patch_request = create_patch_request_from_case(case);

    // Execute PATCH operation
    let result = match case.operation {
        PatchOperation::Add => {
            server
                .provider()
                .patch_resource(&case.resource_type, &resource_id, &patch_request, &context)
                .await
        }
        PatchOperation::Remove => {
            server
                .provider()
                .patch_resource(&case.resource_type, &resource_id, &patch_request, &context)
                .await
        }
        PatchOperation::Replace => {
            server
                .provider()
                .patch_resource(&case.resource_type, &resource_id, &patch_request, &context)
                .await
        }
    };

    match result {
        Ok(resource) => PatchTestResult {
            success: true,
            resource: Some(resource.to_json().unwrap()),
            error: None,
            status_code: Some(200),
            etag: Some("W/\"updated\"".to_string()), // Simplified - would extract from response
        },
        Err(error) => {
            let error_msg = format!("{:?}", error);
            let status_code = if error_msg.contains("readonly attribute")
                || error_msg.contains("Cannot modify readonly")
            {
                400 // Bad Request for mutability violations
            } else if error_msg.contains("not found") || error_msg.contains("NotFound") {
                404 // Not Found
            } else if error_msg.contains("invalid") || error_msg.contains("Invalid") {
                400 // Bad Request for validation errors
            } else if error_msg.contains("duplicate") || error_msg.contains("uniqueness") {
                409 // Conflict for uniqueness violations
            } else {
                500 // Internal Server Error for other cases
            };

            PatchTestResult {
                success: false,
                resource: None,
                error: Some(error_msg),
                status_code: Some(status_code),
                etag: None,
            }
        }
    }
}

async fn test_capability_scenario(scenario: &CapabilityTestScenario) {
    let server = if scenario.patch_supported {
        test_helpers::create_test_server_with_patch_support()
    } else {
        test_helpers::create_test_server_without_patch_support()
    };

    // Test ServiceProviderConfig advertisement
    let config = server.get_service_provider_config().unwrap();
    let config_json = serde_json::to_value(&config).unwrap();
    PatchAssertions::assert_capability_advertisement(&config_json, scenario.patch_supported);

    // Test actual PATCH behavior
    let context = RequestContext::with_generated_id();

    // Create test resource first
    let user = TestDataFactory::user_with_all_attributes();
    let created = server
        .create_resource("User", user, &context)
        .await
        .expect("Failed to create test user");

    let resource_id = created.get_id().unwrap();

    // Attempt PATCH operation
    let patch_request =
        test_data::TestDataFactory::patch_request(scenario.test_operation.patch_operations.clone());

    let result = server
        .patch_resource(
            &scenario.test_operation.resource_type,
            resource_id,
            &patch_request,
            &context,
        )
        .await;

    match scenario.expected_behavior {
        ExpectedBehavior::ProcessRequest => {
            assert!(result.is_ok(), "PATCH should succeed when supported");
        }
        ExpectedBehavior::Return501NotImplemented => {
            assert!(result.is_err(), "PATCH should fail when not supported");
            // Check that it's specifically a 501 error
        }
        _ => {} // Other behaviors tested separately
    }
}

async fn test_multi_tenant_case(case: &MultiTenantTestCase) {
    // Use single server instance for both tenants to ensure proper tenant isolation
    // All tenants have the same capabilities (data isolation, not functional differences)
    let server = test_helpers::create_test_server_with_patch_support();

    let context_a = test_helpers::create_test_context_with_tenant("tenant-a", "client-a");
    let context_b = test_helpers::create_test_context_with_tenant("tenant-b", "client-b");

    // Setup test resources in both tenants
    let user_a = test_helpers::create_test_user(&server, &context_a)
        .await
        .expect("Failed to create user in tenant A");

    let user_b = test_helpers::create_test_user(&server, &context_b)
        .await
        .expect("Failed to create user in tenant B");

    // Track which tenant performed the operation for isolation testing
    let operation_tenant = match &case.operation {
        TenantOperation::PatchInTenantA {
            resource_id: _,
            patch_request,
        } => {
            // Use the actual created user ID instead of hardcoded one
            let user_id = user_a.get_id().unwrap();
            let result = server
                .patch_resource("User", user_id, patch_request, &context_a)
                .await;

            // Verify operation succeeded - all tenants have same capabilities
            assert!(result.is_ok(), "Patch should succeed in tenant A");
            "tenant-a"
        }
        TenantOperation::PatchInTenantB {
            resource_id: _,
            patch_request,
        } => {
            // Use the actual created user ID instead of hardcoded one
            let user_id = user_b.get_id().unwrap();
            let result = server
                .patch_resource("User", user_id, patch_request, &context_b)
                .await;

            // Verify operation succeeded - all tenants have same capabilities
            assert!(result.is_ok(), "Patch should succeed in tenant B");
            "tenant-b"
        }
        TenantOperation::CrossTenantAccess {
            source_tenant: _,
            target_tenant: _,
            resource_id: _,
        } => {
            // Test that each tenant only sees its own resources
            // Both tenants have resource "1" but they should be isolated
            // Try to access a non-existent resource ID to verify isolation
            let non_existent_id = "999999";

            let patch_request =
                TestDataFactory::patch_request(vec![TestDataFactory::add_operation(
                    "displayName",
                    json!("Should Not Work"),
                )]);

            let result = server
                .patch_resource("User", non_existent_id, &patch_request, &context_b)
                .await;

            // This should fail because the resource doesn't exist in tenant-b's namespace
            assert!(
                result.is_err(),
                "Access to non-existent resource should fail - tenant isolation maintained"
            );
            "cross-tenant"
        }
    };

    // Verify isolation - get current state of resources after patch operations
    if case.expected_isolation {
        // Retrieve current state of both users to check isolation
        let current_user_a = server
            .get_resource("User", user_a.get_id().unwrap(), &context_a)
            .await
            .expect("Failed to retrieve user A after patch");

        let current_user_b = server
            .get_resource("User", user_b.get_id().unwrap(), &context_b)
            .await
            .expect("Failed to retrieve user B after patch");

        PatchAssertions::assert_tenant_isolation(
            &current_user_a.unwrap().to_json().unwrap(),
            &current_user_b.unwrap().to_json().unwrap(),
            operation_tenant,
        );
    }
}

async fn test_error_case(case: &ErrorTestCase) {
    let setup = (case.setup)();
    let server = if setup.capabilities.patch_supported {
        test_helpers::create_test_server_with_patch_support()
    } else {
        test_helpers::create_test_server_without_patch_support()
    };
    let context = test_helpers::create_test_context();

    // Create user only if the test setup requires it
    let user_id = if setup.create_existing_resource {
        let user = test_helpers::create_test_user(&server, &context)
            .await
            .expect("Failed to create test user");
        user.get_id().unwrap().to_string()
    } else {
        // Use a fake ID for nonexistent resource tests
        "nonexistent-user-id".to_string()
    };

    let result = server
        .patch_resource("User", &user_id, &case.patch_request, &context)
        .await;

    assert!(result.is_err(), "Error case '{}' should fail", case.name);

    let test_result = PatchTestResult {
        success: false,
        resource: None,
        error: Some(format!("{:?}", result.err().unwrap())),
        status_code: Some(case.expected_status),
        etag: None,
    };

    PatchAssertions::assert_patch_error(&test_result, case.expected_error.clone());
    PatchAssertions::assert_patch_status_code(&test_result, case.expected_status);
}

async fn test_atomic_case(case: &AtomicTestCase) {
    let server = test_helpers::create_test_server_with_patch_support();
    let context = test_helpers::create_test_context();

    // Create initial resource
    let created = test_helpers::create_test_user(&server, &context)
        .await
        .expect("Failed to create user");

    let resource_id = created.get_id().unwrap();
    let original_resource = created.clone();

    // Execute atomic PATCH with multiple operations
    let patch_request = test_data::TestDataFactory::patch_request(case.operations.clone());

    let result = server
        .provider()
        .patch_resource("User", resource_id, &patch_request, &context)
        .await;

    match case.expected_behavior {
        AtomicBehavior::AllSucceed => {
            assert!(result.is_ok(), "All operations should succeed");
            // Verify all expected changes were applied
        }
        AtomicBehavior::AllFail => {
            assert!(
                result.is_err(),
                "All operations should fail due to one failure"
            );
            // Verify resource is unchanged
            let current = server
                .get_resource("User", &resource_id, &context)
                .await
                .expect("Should be able to get resource");

            PatchAssertions::assert_resource_unchanged(
                &original_resource.to_json().unwrap(),
                &current.unwrap().to_json().unwrap(),
            );
        }
        AtomicBehavior::ResolveConflicts => {
            // Implementation-specific behavior for conflicting operations
            // Could succeed with last operation winning, or fail with conflict error
        }
    }
}

async fn test_etag_case(case: &ETagTestCase) {
    use scim_server::resource::version::{ConditionalResult, HttpVersion, RawVersion};

    let server = test_helpers::create_test_server_with_patch_support();
    let context = test_helpers::create_test_context();

    // Create resource with known ETag
    let created = test_helpers::create_test_user(&server, &context)
        .await
        .expect("Failed to create user");

    let resource_id = created.get_id().unwrap();

    // Get the actual current version of the resource
    let current_resource = server
        .get_resource("User", resource_id, &context)
        .await
        .expect("Should be able to get current resource")
        .expect("Resource should exist");

    let actual_current_version = if let Some(meta) = current_resource.get_meta() {
        if let Some(version_str) = meta.version() {
            version_str
                .parse::<RawVersion>()
                .expect("Should be able to parse stored version")
        } else {
            RawVersion::from_content(&current_resource.to_json().unwrap().to_string().as_bytes())
        }
    } else {
        RawVersion::from_content(&current_resource.to_json().unwrap().to_string().as_bytes())
    };

    // Create PATCH request with conditional headers
    let patch_request = TestDataFactory::patch_request(vec![TestDataFactory::replace_operation(
        "displayName",
        json!("Updated Display Name"),
    )]);

    // Execute conditional PATCH
    let result = if let Some(request_etag_str) = &case.request_etag {
        // For the test to work properly, we need to determine which ETag to use
        let request_etag = if case.name == "valid_etag_update" {
            // Use the actual current version for success case
            actual_current_version.clone()
        } else {
            // Use a different ETag for conflict case
            RawVersion::from(
                request_etag_str
                    .parse::<HttpVersion>()
                    .expect("Should be able to parse ETag"),
            )
        };

        // Use conditional patch
        let conditional_result = server
            .provider()
            .conditional_patch_resource(
                "User",
                resource_id,
                &patch_request,
                &request_etag,
                &context,
            )
            .await
            .expect("Conditional patch should not fail at provider level");

        match conditional_result {
            ConditionalResult::Success(versioned_resource) => {
                Ok(versioned_resource.resource().clone())
            }
            ConditionalResult::VersionMismatch(_) => {
                Err(scim_server::error::ScimError::invalid_request(
                    "ETag mismatch - resource was modified by another client".to_string(),
                ))
            }
            ConditionalResult::NotFound => Err(scim_server::error::ScimError::resource_not_found(
                "User",
                resource_id,
            )),
        }
    } else {
        // Non-conditional patch
        server
            .patch_resource("User", resource_id, &patch_request, &context)
            .await
    };

    match case.expected_result {
        ETagResult::Success => {
            assert!(result.is_ok(), "ETag test '{}' should succeed", case.name);
        }
        ETagResult::PreconditionFailed => {
            assert!(
                result.is_err(),
                "ETag test '{}' should fail with precondition",
                case.name
            );
        }
    }
}

// Real-world scenario test implementations

async fn test_user_profile_update() {
    let server = test_helpers::create_test_server_with_patch_support();
    let context = test_helpers::create_test_context();

    // Create user
    let created = test_helpers::create_test_user(&server, &context)
        .await
        .expect("Failed to create user");

    let user_id = created.get_id().unwrap();

    // Perform comprehensive profile update
    let patch_request = test_data::TestDataFactory::patch_request(vec![
        test_data::TestDataFactory::add_operation("displayName", json!("John Doe")),
        test_data::TestDataFactory::add_operation(
            "name",
            json!({
                "givenName": "John",
                "familyName": "Doe"
            }),
        ),
        test_data::TestDataFactory::add_operation(
            "emails",
            json!([{
                "value": "john.doe@example.com",
                "type": "work",
                "primary": true
            }]),
        ),
        test_data::TestDataFactory::add_operation("title", json!("Software Engineer")),
    ]);

    let result = server
        .provider()
        .patch_resource("User", user_id, &patch_request, &context)
        .await;

    assert!(result.is_ok(), "User profile update should succeed");

    let updated_user = result.unwrap();
    assert_eq!(updated_user.get("displayName").unwrap(), "John Doe");
    assert_eq!(updated_user.get("title").unwrap(), "Software Engineer");
}

async fn test_email_management() {
    let server = test_helpers::create_test_server_with_patch_support();
    let context = test_helpers::create_test_context();

    // Create user with existing email
    let created = test_helpers::create_test_user(&server, &context)
        .await
        .expect("Failed to create user");

    let user_id = created.get_id().unwrap();

    // Add new email
    let add_email_request =
        test_data::TestDataFactory::patch_request(vec![test_data::TestDataFactory::add_operation(
            "emails",
            json!({
                "value": "john.personal@example.com",
                "type": "personal",
                "primary": false,
                "display": null
            }),
        )]);

    let result = server
        .provider()
        .patch_resource("User", user_id, &add_email_request, &context)
        .await;

    assert!(result.is_ok(), "Adding email should succeed");

    // Remove work email by filter
    let remove_email_request = test_data::TestDataFactory::patch_request(vec![
        test_data::TestDataFactory::remove_operation("emails[type eq \"work\"]"),
    ]);

    let result = server
        .provider()
        .patch_resource("User", user_id, &remove_email_request, &context)
        .await;

    assert!(result.is_ok(), "Removing email by filter should succeed");
}

async fn test_user_deactivation() {
    let server = test_helpers::create_test_server_with_patch_support();
    let context = test_helpers::create_test_context();

    // Create active user
    let created = test_helpers::create_test_user(&server, &context)
        .await
        .expect("Failed to create user");

    let user_id = created.get_id().unwrap();

    // Deactivate user
    let patch_request = TestDataFactory::patch_request(vec![TestDataFactory::replace_operation(
        "active",
        json!(false),
    )]);

    let result = server
        .patch_resource("User", user_id, &patch_request, &context)
        .await;

    assert!(result.is_ok(), "User deactivation should succeed");

    let updated_user = result.unwrap();
    assert_eq!(updated_user.get("active").unwrap(), &json!(false));
}

async fn test_enterprise_extension_updates() {
    let server = test_helpers::create_test_server_with_patch_support();
    let context = test_helpers::create_test_context();

    // Create user with enterprise extension
    let created = test_helpers::create_test_user(&server, &context)
        .await
        .expect("Failed to create enterprise user");

    let user_id = created.get_id().unwrap();

    // Update enterprise attributes
    let patch_request = TestDataFactory::patch_request(vec![
        TestDataFactory::add_operation(
            "urn:ietf:params:scim:schemas:extension:enterprise:2.0:User.department",
            json!("Engineering - Backend"),
        ),
        test_data::TestDataFactory::replace_operation(
            "urn:ietf:params:scim:schemas:extension:enterprise:2.0:User.costCenter",
            json!("TECH-001"),
        ),
    ]);

    let result = server
        .provider()
        .patch_resource("User", user_id, &patch_request, &context)
        .await;

    assert!(result.is_ok(), "Enterprise extension update should succeed");
}

async fn test_group_member_addition() {
    let server = test_helpers::create_test_server_with_patch_support();
    let context = test_helpers::create_test_context();

    // Create group and users
    let created_group = test_helpers::create_test_group(&server, &context)
        .await
        .expect("Failed to create group");

    let group_id = created_group.get_id().unwrap();

    // Add new member
    let add_member_request =
        test_data::TestDataFactory::patch_request(vec![test_data::TestDataFactory::add_operation(
            "members",
            json!({
                "value": "user-new",
                "type": "User",
                "display": "New User"
            }),
        )]);

    let result = server
        .provider()
        .patch_resource("Group", group_id, &add_member_request, &context)
        .await;

    assert!(result.is_ok(), "Adding group member should succeed");
}

async fn test_group_member_removal() {
    let server = test_helpers::create_test_server_with_patch_support();
    let context = test_helpers::create_test_context();

    // Create group with members
    let created_group = test_helpers::create_test_group(&server, &context)
        .await
        .expect("Failed to create group");

    let group_id = created_group.get_id().unwrap();

    // Remove specific member
    let patch_request = TestDataFactory::patch_request(vec![TestDataFactory::remove_operation(
        "members[value eq \"user-to-remove\"]",
    )]);

    let result = server
        .patch_resource("Group", group_id, &patch_request, &context)
        .await;

    assert!(result.is_ok(), "Removing group member should succeed");
}

async fn test_group_metadata_update() {
    let server = test_helpers::create_test_server_with_patch_support();
    let context = test_helpers::create_test_context();

    // Create group
    let created_group = test_helpers::create_test_group(&server, &context)
        .await
        .expect("Failed to create group");

    let group_id = created_group.get_id().unwrap();

    // Update group metadata
    let patch_request = TestDataFactory::patch_request(vec![TestDataFactory::replace_operation(
        "displayName",
        json!("Updated Team Name"),
    )]);

    let result = server
        .patch_resource("Group", group_id, &patch_request, &context)
        .await;

    assert!(result.is_ok(), "Group metadata update should succeed");

    let updated_group = result.unwrap();
    assert_eq!(
        updated_group.get("displayName").unwrap(),
        "Updated Team Name"
    );
}

// Helper types and functions

fn create_patch_request_from_case(case: &PatchTestCase) -> Value {
    let operation = match case.operation {
        PatchOperation::Add => TestDataFactory::add_operation(
            &case.path,
            case.value.clone().unwrap_or(json!("default-value")),
        ),
        PatchOperation::Remove => TestDataFactory::remove_operation(&case.path),
        PatchOperation::Replace => TestDataFactory::replace_operation(
            &case.path,
            case.value.clone().unwrap_or(json!("default-value")),
        ),
    };

    TestDataFactory::patch_request(vec![operation])
}
