//! Test data factory for SCIM PATCH operations
//!
//! This module provides utilities for creating test data, PATCH requests,
//! and resource configurations used throughout the PATCH integration tests.

use super::*;
use serde_json::{Value, json};
use std::collections::HashMap;

/// Factory for creating test data and PATCH requests
pub struct TestDataFactory;

impl TestDataFactory {
    /// Create a complete User resource with all standard attributes
    pub fn user_with_all_attributes() -> Value {
        json!({
            "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
            "id": "user-123",
            "userName": "john.doe@example.com",
            "name": {
                "formatted": "Mr. John Doe",
                "familyName": "Doe",
                "givenName": "John",
                "middleName": "William",
                "honorificPrefix": "Mr.",
                "honorificSuffix": "Jr."
            },
            "displayName": "John Doe",
            "nickName": "Johnny",
            "profileUrl": "https://example.com/users/john",
            "emails": [
                {
                    "value": "john.doe@work.example.com",
                    "type": "work",
                    "primary": true
                },
                {
                    "value": "john.doe@home.example.com",
                    "type": "home",
                    "primary": false
                }
            ],
            "phoneNumbers": [
                {
                    "value": "+1-555-555-1234",
                    "type": "work"
                },
                {
                    "value": "+1-555-555-5678",
                    "type": "mobile"
                }
            ],
            "addresses": [
                {
                    "type": "work",
                    "streetAddress": "100 Universal City Plaza",
                    "locality": "Hollywood",
                    "region": "CA",
                    "postalCode": "91608",
                    "country": "USA",
                    "formatted": "100 Universal City Plaza\nHollywood, CA 91608 USA",
                    "primary": true
                }
            ],
            "active": true,
            "title": "Software Engineer",
            "userType": "Employee",
            "preferredLanguage": "en-US",
            "locale": "en-US",
            "timezone": "America/Los_Angeles",
            "meta": {
                "resourceType": "User",
                "created": "2025-08-12T10:00:00.000Z",
                "lastModified": "2025-08-12T10:00:00.000Z",
                "location": "https://example.com/scim/v2/Users/user-123",
                "version": "W/\"abc123\""
            }
        })
    }

    /// Create a minimal User resource with only required attributes
    pub fn user_with_minimal_attributes() -> Value {
        json!({
            "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
            "id": "user-minimal",
            "userName": "minimal.user@example.com",
            "active": true,
            "meta": {
                "resourceType": "User",
                "created": "2025-08-12T10:00:00.000Z",
                "lastModified": "2025-08-12T10:00:00.000Z",
                "location": "https://example.com/scim/v2/Users/user-minimal",
                "version": "W/\"def456\""
            }
        })
    }

    /// Create a Group resource with members
    pub fn group_with_members() -> Value {
        json!({
            "schemas": ["urn:ietf:params:scim:schemas:core:2.0:Group"],
            "id": "group-123",
            "displayName": "Engineering Team",
            "members": [
                {
                    "value": "user-123",
                    "type": "User",
                    "display": "John Doe"
                },
                {
                    "value": "user-456",
                    "type": "User",
                    "display": "Jane Smith"
                }
            ],
            "meta": {
                "resourceType": "Group",
                "created": "2025-08-12T10:00:00.000Z",
                "lastModified": "2025-08-12T10:00:00.000Z",
                "location": "https://example.com/scim/v2/Groups/group-123",
                "version": "W/\"ghi789\""
            }
        })
    }

    /// Create a User with Enterprise extension
    pub fn user_with_enterprise_extension() -> Value {
        let mut user = Self::user_with_all_attributes();
        user["schemas"] = json!([
            "urn:ietf:params:scim:schemas:core:2.0:User",
            "urn:ietf:params:scim:schemas:extension:enterprise:2.0:User"
        ]);
        user["urn:ietf:params:scim:schemas:extension:enterprise:2.0:User"] = json!({
            "employeeNumber": "12345",
            "costCenter": "Engineering",
            "organization": "Acme Corp",
            "division": "Technology",
            "department": "Software Development",
            "manager": {
                "value": "user-manager",
                "displayName": "Manager Name"
            }
        });
        user
    }

    /// Create a PATCH request with multiple operations
    pub fn patch_request(operations: Vec<PatchOperationSpec>) -> Value {
        json!({
            "schemas": ["urn:ietf:params:scim:api:messages:2.0:PatchOp"],
            "Operations": operations
        })
    }

    /// Create an add operation
    pub fn add_operation(path: &str, value: Value) -> PatchOperationSpec {
        PatchOperationSpec::add(path, value)
    }

    /// Create a remove operation
    pub fn remove_operation(path: &str) -> PatchOperationSpec {
        PatchOperationSpec::remove(path)
    }

    /// Create a replace operation
    pub fn replace_operation(path: &str, value: Value) -> PatchOperationSpec {
        PatchOperationSpec::replace(path, value)
    }

    /// Create test values appropriate for different path types
    pub fn generate_test_value_for_path(path: &str) -> Option<Value> {
        match path {
            "displayName" => Some(json!("Updated Display Name")),
            "active" => Some(json!(false)),
            "name.givenName" => Some(json!("UpdatedFirst")),
            "name.familyName" => Some(json!("UpdatedLast")),
            "emails" => Some(json!([{
                "value": "new.email@example.com",
                "type": "work",
                "primary": false,
                "display": null
            }])),
            "phoneNumbers" => Some(json!([{
                "value": "+1-555-999-0000",
                "type": "mobile",
                "display": null,
                "primary": null
            }])),
            "title" => Some(json!("Senior Software Engineer")),
            "userType" => Some(json!("Contractor")),
            "locale" => Some(json!("en-GB")),
            "timezone" => Some(json!("Europe/London")),
            // Multi-valued with filters don't need values for remove operations
            path if path.contains("[") => None,
            // Read-only attributes
            "id" | "meta.created" | "meta.resourceType" => Some(json!("should-be-rejected")),
            // Unknown attributes
            _ => Some(json!("test-value")),
        }
    }

    /// Generate comprehensive test cases for all operation/path combinations
    pub fn generate_patch_test_cases() -> Vec<PatchTestCase> {
        let operations = [
            ("add", PatchOperation::Add),
            ("remove", PatchOperation::Remove),
            ("replace", PatchOperation::Replace),
        ];

        let paths = [
            // Simple paths that should succeed
            ("simple_display_name", "displayName", PathType::Simple, true),
            ("simple_active", "active", PathType::Simple, true),
            ("simple_title", "title", PathType::Simple, true),
            // Complex paths that should succeed
            (
                "complex_given_name",
                "name.givenName",
                PathType::Complex,
                true,
            ),
            (
                "complex_family_name",
                "name.familyName",
                PathType::Complex,
                true,
            ),
            // Multi-valued paths that should succeed
            ("multivalued_emails", "emails", PathType::MultiValued, true),
            (
                "multivalued_phones",
                "phoneNumbers",
                PathType::MultiValued,
                true,
            ),
            // Filtered paths that should succeed
            (
                "filtered_work_email",
                "emails[type eq \"work\"]",
                PathType::Filtered,
                true,
            ),
            (
                "filtered_primary_email",
                "emails[primary eq true]",
                PathType::Filtered,
                true,
            ),
            // Paths that should fail - read-only
            ("readonly_id", "id", PathType::ReadOnly, false),
            (
                "readonly_meta_created",
                "meta.created",
                PathType::ReadOnly,
                false,
            ),
            (
                "readonly_meta_resource_type",
                "meta.resourceType",
                PathType::ReadOnly,
                false,
            ),
            // Paths that should fail - invalid
            (
                "invalid_nonexistent",
                "nonexistent.invalid",
                PathType::Invalid,
                false,
            ),
            (
                "invalid_malformed",
                "emails[invalid syntax",
                PathType::Invalid,
                false,
            ),
        ];

        let resource_types = ["User", "Group"];

        let mut test_cases = Vec::new();

        for (op_name, op) in &operations {
            for (path_name, path, path_type, should_succeed) in &paths {
                for resource_type in &resource_types {
                    // Skip some combinations that don't make sense
                    if *resource_type == "Group"
                        && (path.starts_with("name.") || path.starts_with("emails"))
                    {
                        continue;
                    }

                    let expected_result = if *should_succeed {
                        match path_type {
                            PathType::ReadOnly | PathType::Immutable => ExpectedResult::ScimError {
                                error_type: ScimErrorType::Mutability,
                                status_code: 400,
                            },
                            PathType::Invalid => ExpectedResult::ScimError {
                                error_type: ScimErrorType::InvalidPath,
                                status_code: 400,
                            },
                            _ => ExpectedResult::Success,
                        }
                    } else {
                        match path_type {
                            PathType::ReadOnly | PathType::Immutable => ExpectedResult::ScimError {
                                error_type: ScimErrorType::Mutability,
                                status_code: 400,
                            },
                            PathType::Invalid => ExpectedResult::ScimError {
                                error_type: ScimErrorType::InvalidPath,
                                status_code: 400,
                            },
                            _ => ExpectedResult::ScimError {
                                error_type: ScimErrorType::InvalidValue,
                                status_code: 400,
                            },
                        }
                    };

                    let test_case = PatchTestCase::new(
                        format!("{}_{}_on_{}", op_name, path_name, resource_type),
                        *op,
                        *path,
                        Self::generate_test_value_for_path(path),
                        expected_result,
                    )
                    .with_resource_type(*resource_type);

                    test_cases.push(test_case);
                }
            }
        }

        test_cases
    }

    /// Generate error test cases
    pub fn generate_error_test_cases() -> Vec<ErrorTestCase> {
        vec![
            ErrorTestCase {
                name: "patch_nonexistent_resource".to_string(),
                setup: Self::setup_empty_provider,
                patch_request: Self::patch_request(vec![Self::add_operation(
                    "displayName",
                    json!("New Name"),
                )]),
                expected_error: ScimErrorType::NotFound,
                expected_status: 404,
            },
            ErrorTestCase {
                name: "patch_with_invalid_etag".to_string(),
                setup: Self::setup_existing_user,
                patch_request: Self::patch_request_with_etag("invalid-etag"),
                expected_error: ScimErrorType::PreconditionFailed,
                expected_status: 412,
            },
            ErrorTestCase {
                name: "patch_readonly_attribute".to_string(),
                setup: Self::setup_existing_user,
                patch_request: Self::patch_request(vec![Self::replace_operation(
                    "meta.created",
                    json!("2025-01-01T00:00:00.000Z"),
                )]),
                expected_error: ScimErrorType::Mutability,
                expected_status: 400,
            },
            ErrorTestCase {
                name: "patch_invalid_json".to_string(),
                setup: Self::setup_existing_user,
                patch_request: json!({"invalid": "structure"}),
                expected_error: ScimErrorType::InvalidSyntax,
                expected_status: 400,
            },
            ErrorTestCase {
                name: "patch_empty_operations".to_string(),
                setup: Self::setup_existing_user,
                patch_request: Self::patch_request(vec![]),
                expected_error: ScimErrorType::InvalidValue,
                expected_status: 400,
            },
        ]
    }

    /// Generate capability test scenarios
    pub fn generate_capability_test_scenarios() -> Vec<CapabilityTestScenario> {
        vec![
            CapabilityTestScenario {
                name: "patch_supported_true".to_string(),
                patch_supported: true,
                expected_behavior: ExpectedBehavior::ProcessRequest,
                test_operation: TestOperation {
                    resource_type: "User".to_string(),
                    resource_id: "user-123".to_string(),
                    patch_operations: vec![Self::add_operation("displayName", json!("Test Name"))],
                },
            },
            CapabilityTestScenario {
                name: "patch_supported_false".to_string(),
                patch_supported: false,
                expected_behavior: ExpectedBehavior::Return501NotImplemented,
                test_operation: TestOperation {
                    resource_type: "User".to_string(),
                    resource_id: "user-123".to_string(),
                    patch_operations: vec![Self::add_operation("displayName", json!("Test Name"))],
                },
            },
        ]
    }

    /// Generate multi-tenant test cases
    pub fn generate_multi_tenant_test_cases() -> Vec<MultiTenantTestCase> {
        vec![
            MultiTenantTestCase {
                name: "isolated_tenant_operations".to_string(),
                tenant_a_capabilities: TestCapabilities {
                    patch_supported: true,
                    etag_supported: true,
                    custom_capabilities: HashMap::new(),
                },
                tenant_b_capabilities: TestCapabilities {
                    patch_supported: true,
                    etag_supported: true,
                    custom_capabilities: HashMap::new(),
                },
                operation: TenantOperation::PatchInTenantA {
                    resource_id: "user-123".to_string(),
                    patch_request: Self::patch_request(vec![Self::add_operation(
                        "displayName",
                        json!("Tenant A User"),
                    )]),
                },
                expected_isolation: true,
            },
            MultiTenantTestCase {
                name: "cross_tenant_access_prevention".to_string(),
                tenant_a_capabilities: TestCapabilities {
                    patch_supported: true,
                    etag_supported: true,
                    custom_capabilities: HashMap::new(),
                },
                tenant_b_capabilities: TestCapabilities {
                    patch_supported: true,
                    etag_supported: true,
                    custom_capabilities: HashMap::new(),
                },
                operation: TenantOperation::CrossTenantAccess {
                    source_tenant: "tenant-a".to_string(),
                    target_tenant: "tenant-b".to_string(),
                    resource_id: "1".to_string(),
                },
                expected_isolation: true,
            },
            MultiTenantTestCase {
                name: "bidirectional_tenant_operations".to_string(),
                tenant_a_capabilities: TestCapabilities {
                    patch_supported: true,
                    etag_supported: true,
                    custom_capabilities: HashMap::new(),
                },
                tenant_b_capabilities: TestCapabilities {
                    patch_supported: true,
                    etag_supported: true,
                    custom_capabilities: HashMap::new(),
                },
                operation: TenantOperation::PatchInTenantB {
                    resource_id: "user-456".to_string(),
                    patch_request: Self::patch_request(vec![Self::add_operation(
                        "displayName",
                        json!("Tenant B User"),
                    )]),
                },
                expected_isolation: true,
            },
        ]
    }

    // Helper functions for test setup
    fn setup_empty_provider() -> TestSetup {
        TestSetup {
            create_existing_resource: false,
            ..Default::default()
        }
    }

    fn setup_existing_user() -> TestSetup {
        TestSetup {
            create_existing_resource: true,
            initial_resource: Some(Self::user_with_all_attributes()),
            ..Default::default()
        }
    }

    fn patch_request_with_etag(etag: &str) -> Value {
        let mut request = Self::patch_request(vec![Self::add_operation(
            "displayName",
            json!("Updated Name"),
        )]);
        // This would be set in the HTTP headers in a real implementation
        request["etag"] = json!(etag);
        request
    }
}

/// Path utilities for test data generation
pub struct PathUtils;

impl PathUtils {
    /// Check if a path represents a read-only attribute
    pub fn is_readonly_path(path: &str) -> bool {
        matches!(
            path,
            "id" | "meta.created" | "meta.resourceType" | "meta.location"
        )
    }

    /// Check if a path represents an immutable attribute
    pub fn is_immutable_path(path: &str) -> bool {
        matches!(path, "id" | "userName") // userName can be immutable in some implementations
    }

    /// Check if a path is valid syntax
    pub fn is_valid_path_syntax(path: &str) -> bool {
        // Simple validation - real implementation would be more comprehensive
        !path.is_empty()
            && !path.contains("invalid syntax")
            && !path.starts_with('.')
            && !path.ends_with('.')
    }

    /// Extract the root attribute from a complex path
    pub fn get_root_attribute(path: &str) -> &str {
        path.split('.')
            .next()
            .unwrap_or(path)
            .split('[')
            .next()
            .unwrap_or(path)
    }

    /// Check if path targets a multi-valued attribute
    pub fn is_multivalued_path(path: &str) -> bool {
        let root = Self::get_root_attribute(path);
        matches!(
            root,
            "emails" | "phoneNumbers" | "addresses" | "members" | "groups"
        )
    }

    /// Check if path contains a filter expression
    pub fn has_filter(path: &str) -> bool {
        path.contains('[') && path.contains(']')
    }
}
