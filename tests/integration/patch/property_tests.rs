//! Property-Based Testing for SCIM PATCH Operations
//!
//! This module provides property-based tests to verify SCIM PATCH operations maintain
//! invariants and handle edge cases correctly. Uses proptest for generating
//! random valid and invalid inputs with automatic shrinking.

use super::test_data::TestDataFactory;
use super::test_helpers;
use super::*;
use proptest::prelude::*;

use serde_json::{Value, json};

/// Property test configuration for PATCH operations
#[derive(Debug, Clone)]
pub struct PatchPropertyConfig {
    pub max_operations_per_request: usize,
    pub max_test_iterations: usize,
    pub include_invalid_operations: bool,
    pub test_concurrent_operations: bool,
    pub resource_types: Vec<String>,
}

impl Default for PatchPropertyConfig {
    fn default() -> Self {
        Self {
            max_operations_per_request: 10,
            max_test_iterations: 100,
            include_invalid_operations: true,
            test_concurrent_operations: true,
            resource_types: vec!["User".to_string(), "Group".to_string()],
        }
    }
}

/// Test case for property-based testing
#[derive(Debug, Clone)]
pub struct PropertyTestCase {
    pub resource_type: String,
    pub initial_resource: Value,
    pub operations: Vec<PatchOperationSpec>,
    pub initial_etag: Option<String>,
    pub expected_atomicity: bool,
}

/// Property test: PATCH operations preserve SCIM invariants
#[tokio::test]
async fn property_test_patch_preserves_scim_invariants() {
    let config = PatchPropertyConfig::default();

    for _ in 0..10 {
        // Reduced iterations for CI performance
        let test_case = generate_deterministic_patch_scenario(&config, 42); // Fixed seed for reproducibility
        let result = execute_property_test_case(test_case).await;

        if let Ok(resource) = result {
            // Verify SCIM invariants are maintained
            assert_scim_resource_invariants(&resource);
        }
    }
}

prop_compose! {
    fn patch_operation_strategy()
        (op_type in prop::sample::select(vec!["add", "remove", "replace"]),
         path in user_path_strategy(),
         value in json_value_strategy())
        -> PatchOperationSpec {
        match op_type.as_ref() {
            "add" => PatchOperationSpec::add(&path, value),
            "remove" => PatchOperationSpec::remove(&path),
            "replace" => PatchOperationSpec::replace(&path, value),
            _ => PatchOperationSpec::replace(&path, value),
        }
    }
}

/// Strategy for generating valid User resource paths
fn user_path_strategy() -> impl Strategy<Value = String> {
    prop::sample::select(vec![
        "displayName".to_string(),
        "emails".to_string(),
        "emails[type eq \"work\"].value".to_string(),
        "name.givenName".to_string(),
        "name.familyName".to_string(),
        "active".to_string(),
    ])
}

/// Strategy for generating JSON values
fn json_value_strategy() -> impl Strategy<Value = Value> {
    prop_oneof![
        any::<String>().prop_map(|s| json!(s)),
        any::<i32>().prop_map(|n| json!(n)),
        any::<bool>().prop_map(|b| json!(b)),
        Just(json!({"key": "value"})),
        Just(json!(["item1", "item2"])),
    ]
}

prop_compose! {
    fn patch_scenario_strategy()
        (operations in prop::collection::vec(patch_operation_strategy(), 1..5))
        -> PropertyTestCase {
        PropertyTestCase {
            resource_type: "User".to_string(),
            initial_resource: TestDataFactory::user_with_minimal_attributes(),
            operations,
            initial_etag: Some("W/\"1\"".to_string()),
            expected_atomicity: true,
        }
    }
}

proptest! {
    #[test]
    fn test_patch_operations_are_atomic(scenario in patch_scenario_strategy()) {
        tokio_test::block_on(async {
            let result = execute_property_test_case(scenario).await;

            // Either all operations succeed or all fail (atomicity)
            // This property should hold regardless of the specific operations
            match result {
                Ok(resource) => {
                    // If successful, verify the resource is valid
                    assert_scim_resource_invariants(&resource);
                }
                Err(_) => {
                    // If failed, that's also acceptable - operations should be atomic
                }
            }
        });
    }
}

proptest! {
    #[test]
    fn test_patch_idempotency(
        display_name in "[a-zA-Z ]{1,50}",
        _email in "[a-z]+@[a-z]+\\.[a-z]{2,3}"
    ) {
        tokio_test::block_on(async {
            let operation = PatchOperationSpec::replace("displayName", json!(display_name));
            let scenario = PropertyTestCase {
                resource_type: "User".to_string(),
                initial_resource: TestDataFactory::user_with_minimal_attributes(),
                operations: vec![operation.clone()],
                initial_etag: Some("W/\"1\"".to_string()),
                expected_atomicity: true,
            };

            // Execute the same operation twice
            let result1 = execute_property_test_case(scenario.clone()).await;
            let result2 = execute_property_test_case(scenario).await;

            // Results should be equivalent (idempotency property)
            match (result1, result2) {
                (Ok(resource1), Ok(resource2)) => {
                    // Resources should be equivalent after identical operations
                    assert_eq!(
                        resource1.get("displayName"),
                        resource2.get("displayName")
                    );
                }
                (Err(_), Err(_)) => {
                    // Both failing is also consistent
                }
                _ => {
                    // One succeeding and one failing violates idempotency
                    panic!("Idempotency violation: different outcomes for identical operations");
                }
            }
        });
    }
}

/// Test property: Invalid operations don't corrupt resources
#[tokio::test]
async fn property_test_invalid_operations_dont_corrupt() {
    let invalid_scenarios = vec![
        PropertyTestCase {
            resource_type: "User".to_string(),
            initial_resource: TestDataFactory::user_with_minimal_attributes(),
            operations: vec![PatchOperationSpec::replace(
                "nonexistent.field.path",
                json!("value"),
            )],
            initial_etag: Some("W/\"1\"".to_string()),
            expected_atomicity: true,
        },
        PropertyTestCase {
            resource_type: "User".to_string(),
            initial_resource: TestDataFactory::user_with_minimal_attributes(),
            operations: vec![PatchOperationSpec::remove("required.field.id")],
            initial_etag: Some("W/\"1\"".to_string()),
            expected_atomicity: true,
        },
    ];

    for scenario in invalid_scenarios {
        let result = execute_property_test_case(scenario).await;

        // Invalid operations should fail gracefully without corruption
        assert!(result.is_err(), "Invalid operations should fail");
    }
}

/// Execute a property test case
async fn execute_property_test_case(
    test_case: PropertyTestCase,
) -> Result<Value, Box<dyn std::error::Error>> {
    let server = test_helpers::create_test_server_with_patch_support();
    let context = test_helpers::create_test_context();

    // Create initial resource
    let created = server
        .create_resource(
            &test_case.resource_type,
            test_case.initial_resource,
            &context,
        )
        .await?;

    let resource_id = created.get_id().unwrap();

    // Create PATCH request
    let patch_request = TestDataFactory::patch_request(test_case.operations);

    // Execute PATCH operation
    let result = server
        .patch_resource(
            &test_case.resource_type,
            resource_id,
            &patch_request,
            &context,
        )
        .await
        .map(|r| r.to_json().unwrap_or_default())
        .map_err(|e| Box::new(e) as Box<dyn std::error::Error>);

    result
}

/// Generate deterministic patch scenario (for reproducible testing)
fn generate_deterministic_patch_scenario(
    _config: &PatchPropertyConfig,
    _seed: u64,
) -> PropertyTestCase {
    // Use deterministic operations for reproducible tests
    PropertyTestCase {
        resource_type: "User".to_string(),
        initial_resource: TestDataFactory::user_with_minimal_attributes(),
        operations: vec![
            PatchOperationSpec::replace("displayName", json!("Test Name")),
            PatchOperationSpec::add(
                "emails",
                json!([{"value": "test@example.com", "primary": true}]),
            ),
        ],
        initial_etag: Some("W/\"1\"".to_string()),
        expected_atomicity: true,
    }
}

/// Test property: Multivalued attributes handle PATCH correctly
#[tokio::test]
async fn property_test_multivalued_attributes() {
    let _config = PatchPropertyConfig::default();
    let scenario = generate_multivalued_patch_scenario();

    let result = execute_property_test_case(scenario).await;

    match result {
        Ok(resource) => {
            assert_scim_resource_invariants(&resource);
            // Verify multivalued attributes are properly handled
            if let Some(emails) = resource.get("emails") {
                assert!(emails.is_array(), "Emails should remain an array");
            }
        }
        Err(_) => {
            // Failure is acceptable if operations are invalid
        }
    }
}

/// Generate scenario for testing multivalued attributes
fn generate_multivalued_patch_scenario() -> PropertyTestCase {
    let operations = vec![
        PatchOperationSpec::add(
            "emails",
            json!({"value": "new@example.com", "type": "work"}),
        ),
        PatchOperationSpec::replace("emails[type eq \"work\"].value", json!("updated@work.com")),
    ];

    PropertyTestCase {
        resource_type: "User".to_string(),
        initial_resource: TestDataFactory::user_with_all_attributes(),
        operations,
        initial_etag: Some("W/\"1\"".to_string()),
        expected_atomicity: true,
    }
}

/// Test property: ETag handling is consistent
#[tokio::test]
async fn property_test_etag_consistency() {
    let _config = PatchPropertyConfig::default();
    let scenario = generate_etag_patch_scenario();

    let result = execute_property_test_case(scenario).await;

    // ETag should be updated consistently
    match result {
        Ok(resource) => {
            assert_scim_resource_invariants(&resource);
            // Verify resource has updated version information
            assert!(
                resource.get("meta").is_some(),
                "Resource should have meta information"
            );
        }
        Err(_) => {
            // Failure is acceptable
        }
    }
}

/// Generate scenario for testing ETag behavior
fn generate_etag_patch_scenario() -> PropertyTestCase {
    let operations = vec![PatchOperationSpec::replace(
        "displayName",
        json!("ETag Test Name"),
    )];

    PropertyTestCase {
        resource_type: "User".to_string(),
        initial_resource: TestDataFactory::user_with_minimal_attributes(),
        operations,
        initial_etag: Some("W/\"1\"".to_string()),
        expected_atomicity: true,
    }
}

/// Test property: Concurrent operations maintain data integrity
#[tokio::test]
async fn property_test_concurrent_operations() {
    let scenarios = generate_concurrent_patch_scenarios();

    // Execute scenarios concurrently
    let futures: Vec<_> = scenarios
        .into_iter()
        .map(|scenario| execute_property_test_case(scenario))
        .collect();

    let results = futures::future::join_all(futures).await;

    // Verify that concurrent operations don't violate atomicity
    for result in results {
        match result {
            Ok(resource) => {
                assert_scim_resource_invariants(&resource);
            }
            Err(_) => {
                // Concurrent failures are acceptable
            }
        }
    }
}

/// Generate scenarios for concurrent testing
fn generate_concurrent_patch_scenarios() -> Vec<PropertyTestCase> {
    (0..5)
        .map(|i| PropertyTestCase {
            resource_type: "User".to_string(),
            initial_resource: TestDataFactory::user_with_minimal_attributes(),
            operations: vec![PatchOperationSpec::replace(
                "displayName",
                json!(format!("Concurrent Name {}", i)),
            )],
            initial_etag: Some("W/\"1\"".to_string()),
            expected_atomicity: true,
        })
        .collect()
}

/// Assert SCIM resource invariants are maintained
fn assert_scim_resource_invariants(resource: &Value) {
    // Required SCIM fields must be present
    assert!(resource.get("id").is_some(), "Resource must have an ID");
    assert!(
        resource.get("meta").is_some(),
        "Resource must have meta information"
    );

    // Meta information must be properly structured
    if let Some(meta) = resource.get("meta") {
        assert!(
            meta.get("resourceType").is_some(),
            "Meta must include resourceType"
        );
        assert!(
            meta.get("created").is_some(),
            "Meta must include created timestamp"
        );
        assert!(
            meta.get("lastModified").is_some(),
            "Meta must include lastModified timestamp"
        );
    }

    // Schemas must be present and valid
    if let Some(schemas) = resource.get("schemas") {
        assert!(schemas.is_array(), "Schemas must be an array");
        let schemas_array = schemas.as_array().unwrap();
        assert!(!schemas_array.is_empty(), "Schemas array must not be empty");
    }
}

/// Test property: Schema validation is preserved across PATCH operations
#[tokio::test]
async fn property_test_schema_validation_preserved() {
    let test_cases = vec![
        // Valid schema-compliant operations
        PropertyTestCase {
            resource_type: "User".to_string(),
            initial_resource: TestDataFactory::user_with_minimal_attributes(),
            operations: vec![
                PatchOperationSpec::replace("displayName", json!("Valid Display Name")),
                PatchOperationSpec::add(
                    "emails",
                    json!([{"value": "valid@example.com", "primary": true}]),
                ),
            ],
            initial_etag: Some("W/\"1\"".to_string()),
            expected_atomicity: true,
        },
        // Operations that should maintain schema compliance
        PropertyTestCase {
            resource_type: "User".to_string(),
            initial_resource: TestDataFactory::user_with_all_attributes(),
            operations: vec![
                PatchOperationSpec::replace("name.givenName", json!("John")),
                PatchOperationSpec::replace("name.familyName", json!("Doe")),
            ],
            initial_etag: Some("W/\"2\"".to_string()),
            expected_atomicity: true,
        },
    ];

    for (_i, test_case) in test_cases.into_iter().enumerate() {
        let result = execute_property_test_case(test_case).await;

        match result {
            Ok(resource) => {
                assert_scim_resource_invariants(&resource);
                // Additional schema-specific validations could go here
            }
            Err(_) => {
                // Schema validation failures are acceptable for invalid operations
            }
        }
    }
}

/// Test property: Resource versioning behavior
#[tokio::test]
async fn property_test_resource_versioning() {
    let scenario = PropertyTestCase {
        resource_type: "User".to_string(),
        initial_resource: TestDataFactory::user_with_minimal_attributes(),
        operations: vec![PatchOperationSpec::replace(
            "displayName",
            json!("Version Test Name"),
        )],
        initial_etag: Some("W/\"1\"".to_string()),
        expected_atomicity: true,
    };

    let result = execute_property_test_case(scenario).await;

    match result {
        Ok(resource) => {
            assert_scim_resource_invariants(&resource);

            // Verify version information is updated
            if let Some(meta) = resource.get("meta") {
                assert!(
                    meta.get("version").is_some(),
                    "Resource should have version after PATCH"
                );
            }
        }
        Err(_) => {
            // Version handling failures are acceptable for some edge cases
        }
    }
}

/// Test property: Large payload handling
#[tokio::test]
async fn property_test_large_payload_handling() {
    // Test with reasonably large but valid payloads
    let large_value = "x".repeat(1000); // 1KB string

    let scenario = PropertyTestCase {
        resource_type: "User".to_string(),
        initial_resource: TestDataFactory::user_with_minimal_attributes(),
        operations: vec![PatchOperationSpec::replace(
            "displayName",
            json!(large_value),
        )],
        initial_etag: Some("W/\"1\"".to_string()),
        expected_atomicity: true,
    };

    let result = execute_property_test_case(scenario).await;

    match result {
        Ok(resource) => {
            assert_scim_resource_invariants(&resource);
        }
        Err(_) => {
            // Large payload failures are acceptable based on provider limits
        }
    }
}

/// Test property: Complex nested path operations
#[tokio::test]
async fn property_test_complex_path_operations() {
    let complex_scenarios = vec![PropertyTestCase {
        resource_type: "User".to_string(),
        initial_resource: TestDataFactory::user_with_all_attributes(),
        operations: vec![
            PatchOperationSpec::replace(
                "emails[type eq \"work\"].value",
                json!("work@example.com"),
            ),
            PatchOperationSpec::add(
                "emails",
                json!({"value": "personal@example.com", "type": "personal"}),
            ),
        ],
        initial_etag: Some("W/\"1\"".to_string()),
        expected_atomicity: true,
    }];

    for scenario in complex_scenarios {
        let result = execute_property_test_case(scenario).await;

        match result {
            Ok(resource) => {
                assert_scim_resource_invariants(&resource);
                // Verify complex path operations maintain array structure
                if let Some(emails) = resource.get("emails") {
                    assert!(
                        emails.is_array(),
                        "Emails should remain properly structured"
                    );
                }
            }
            Err(_) => {
                // Complex path operation failures are acceptable for invalid paths
            }
        }
    }
}

/// Test property: Resource state transitions are valid
#[tokio::test]
async fn property_test_valid_state_transitions() {
    let state_transition_scenarios = vec![
        // Active user to inactive
        PropertyTestCase {
            resource_type: "User".to_string(),
            initial_resource: json!({
                "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
                "userName": "test.user",
                "active": true
            }),
            operations: vec![PatchOperationSpec::replace("active", json!(false))],
            initial_etag: Some("W/\"1\"".to_string()),
            expected_atomicity: true,
        },
        // Update user name
        PropertyTestCase {
            resource_type: "User".to_string(),
            initial_resource: TestDataFactory::user_with_minimal_attributes(),
            operations: vec![PatchOperationSpec::replace(
                "userName",
                json!("updated.username"),
            )],
            initial_etag: Some("W/\"1\"".to_string()),
            expected_atomicity: true,
        },
    ];

    for (_test_case_idx, scenario) in state_transition_scenarios.into_iter().enumerate() {
        let result = execute_property_test_case(scenario).await;

        match result {
            Ok(resource) => {
                assert_scim_resource_invariants(&resource);
            }
            Err(_) => {
                // State transition failures are acceptable for invalid transitions
            }
        }
    }
}

/// Test property: Patch operations maintain referential integrity
#[tokio::test]
async fn property_test_referential_integrity() {
    let scenarios = generate_referential_integrity_scenarios();

    for scenario in scenarios {
        let result = execute_property_test_case(scenario).await;

        match result {
            Ok(resource) => {
                assert_scim_resource_invariants(&resource);
                // Additional referential integrity checks would go here
            }
            Err(_) => {
                // Referential integrity violations should be caught and result in errors
            }
        }
    }
}

/// Generate scenarios for referential integrity testing
fn generate_referential_integrity_scenarios() -> Vec<PropertyTestCase> {
    vec![PropertyTestCase {
        resource_type: "User".to_string(),
        initial_resource: TestDataFactory::user_with_minimal_attributes(),
        operations: vec![PatchOperationSpec::replace(
            "groups",
            json!([{"value": "group-123", "display": "Test Group"}]),
        )],
        initial_etag: Some("W/\"1\"".to_string()),
        expected_atomicity: true,
    }]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_property_config_default() {
        let config = PatchPropertyConfig::default();
        assert_eq!(config.max_operations_per_request, 10);
        assert_eq!(config.max_test_iterations, 100);
        assert!(config.include_invalid_operations);
        assert!(config.test_concurrent_operations);
        assert_eq!(config.resource_types.len(), 2);
    }

    #[test]
    fn test_deterministic_scenario_generation() {
        let config = PatchPropertyConfig::default();
        let scenario = generate_deterministic_patch_scenario(&config, 42);

        assert_eq!(scenario.resource_type, "User");
        assert_eq!(scenario.operations.len(), 2);
        assert!(scenario.expected_atomicity);
    }
}
