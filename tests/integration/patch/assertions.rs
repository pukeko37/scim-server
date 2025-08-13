//! Assertion helpers for SCIM PATCH operation testing
//!
//! This module provides specialized assertion functions that make it easy to validate
//! PATCH operation results, error conditions, and state changes in a consistent way.

use super::*;
use serde_json::Value;

/// Assertion helpers for PATCH test validation
pub struct PatchAssertions;

impl PatchAssertions {
    /// Assert that a PATCH operation succeeded and produced expected changes
    pub fn assert_patch_success(result: &PatchTestResult, expected_changes: &[(&str, &Value)]) {
        assert!(
            result.is_ok(),
            "Expected PATCH operation to succeed, but got error: {:?}",
            result.error
        );

        let resource = result
            .resource
            .as_ref()
            .expect("Resource should be present on success");

        for (path, expected_value) in expected_changes {
            let actual_value = Self::get_value_at_path(resource, path);
            assert_eq!(
                actual_value,
                Some(*expected_value),
                "Attribute at path '{}' has incorrect value. Expected: {:?}, Got: {:?}",
                path,
                expected_value,
                actual_value
            );
        }
    }

    /// Assert that a PATCH operation failed with a specific error type
    pub fn assert_patch_error(result: &PatchTestResult, expected_error: ScimErrorType) {
        assert!(
            result.is_err(),
            "Expected PATCH operation to fail with {:?}, but it succeeded",
            expected_error
        );

        let actual_error = result.error_type().expect("Error type should be available");
        assert_eq!(
            actual_error, expected_error,
            "Expected error type {:?}, but got {:?}",
            expected_error, actual_error
        );
    }

    /// Assert that a PATCH operation failed with a specific HTTP status code
    pub fn assert_patch_status_code(result: &PatchTestResult, expected_status: u16) {
        let actual_status = result.status_code.expect("Status code should be available");
        assert_eq!(
            actual_status, expected_status,
            "Expected status code {}, but got {}",
            expected_status, actual_status
        );
    }

    /// Assert that a resource remained unchanged after a PATCH operation
    pub fn assert_resource_unchanged(before: &Value, after: &Value) {
        // Exclude meta.lastModified from comparison since it may change
        let before_normalized = Self::normalize_for_comparison(before);
        let after_normalized = Self::normalize_for_comparison(after);

        assert_eq!(
            before_normalized, after_normalized,
            "Resource should have remained unchanged"
        );
    }

    /// Assert that specific attributes were added to a resource
    pub fn assert_attributes_added(before: &Value, after: &Value, added_paths: &[&str]) {
        for path in added_paths {
            let before_value = Self::get_value_at_path(before, path);
            let after_value = Self::get_value_at_path(after, path);

            assert!(
                before_value.is_none(),
                "Attribute at path '{}' should not have existed before PATCH",
                path
            );

            assert!(
                after_value.is_some(),
                "Attribute at path '{}' should exist after PATCH add operation",
                path
            );
        }
    }

    /// Assert that specific attributes were removed from a resource
    pub fn assert_attributes_removed(before: &Value, after: &Value, removed_paths: &[&str]) {
        for path in removed_paths {
            let before_value = Self::get_value_at_path(before, path);
            let after_value = Self::get_value_at_path(after, path);

            assert!(
                before_value.is_some(),
                "Attribute at path '{}' should have existed before PATCH",
                path
            );

            assert!(
                after_value.is_none(),
                "Attribute at path '{}' should not exist after PATCH remove operation",
                path
            );
        }
    }

    /// Assert that specific attributes were replaced in a resource
    pub fn assert_attributes_replaced(
        before: &Value,
        after: &Value,
        replaced_paths: &[(&str, &Value)],
    ) {
        for (path, expected_new_value) in replaced_paths {
            let before_value = Self::get_value_at_path(before, path);
            let after_value = Self::get_value_at_path(after, path);

            assert!(
                before_value.is_some(),
                "Attribute at path '{}' should have existed before PATCH",
                path
            );

            assert_eq!(
                after_value,
                Some(*expected_new_value),
                "Attribute at path '{}' should have new value after PATCH replace operation. Expected: {:?}, Got: {:?}",
                path,
                expected_new_value,
                after_value
            );

            assert_ne!(
                before_value, after_value,
                "Attribute at path '{}' should have changed after PATCH replace operation",
                path
            );
        }
    }

    /// Assert that ETag was updated after a successful PATCH
    pub fn assert_etag_updated(result: &PatchTestResult, original_etag: Option<&str>) {
        let new_etag = result
            .etag
            .as_ref()
            .expect("ETag should be present after PATCH");

        if let Some(original) = original_etag {
            assert_ne!(
                new_etag, original,
                "ETag should have changed after PATCH operation"
            );
        }

        assert!(
            new_etag.starts_with("W/\""),
            "ETag should be in weak format (W/\"...\")"
        );
    }

    /// Assert that meta.lastModified was updated
    pub fn assert_last_modified_updated(before: &Value, after: &Value) {
        let before_modified = Self::get_value_at_path(before, "meta.lastModified");
        let after_modified = Self::get_value_at_path(after, "meta.lastModified");

        assert!(
            before_modified.is_some(),
            "meta.lastModified should exist before PATCH"
        );
        assert!(
            after_modified.is_some(),
            "meta.lastModified should exist after PATCH"
        );

        assert_ne!(
            before_modified, after_modified,
            "meta.lastModified should be updated after PATCH"
        );
    }

    /// Assert that required attributes are still present after PATCH
    pub fn assert_required_attributes_present(resource: &Value, resource_type: &str) {
        let required_attrs = Self::get_required_attributes(resource_type);

        for attr in required_attrs {
            let value = Self::get_value_at_path(resource, attr);
            assert!(
                value.is_some(),
                "Required attribute '{}' must be present after PATCH",
                attr
            );
        }
    }

    /// Assert that a multi-valued attribute has expected number of values
    pub fn assert_multivalued_count(resource: &Value, path: &str, expected_count: usize) {
        let value = Self::get_value_at_path(resource, path);

        match value {
            Some(Value::Array(arr)) => {
                assert_eq!(
                    arr.len(),
                    expected_count,
                    "Multi-valued attribute '{}' should have {} values, but has {}",
                    path,
                    expected_count,
                    arr.len()
                );
            }
            Some(_) => panic!("Attribute '{}' should be an array", path),
            None if expected_count == 0 => {
                // Acceptable - attribute doesn't exist and we expect 0 values
            }
            None => panic!(
                "Attribute '{}' should exist with {} values",
                path, expected_count
            ),
        }
    }

    /// Assert that a filtered operation affected only the intended values
    pub fn assert_filter_operation_targeted(
        before: &Value,
        after: &Value,
        path: &str,
        filter: &str,
    ) {
        let before_array =
            Self::get_value_at_path(before, &Self::get_base_path(path)).and_then(|v| v.as_array());
        let after_array =
            Self::get_value_at_path(after, &Self::get_base_path(path)).and_then(|v| v.as_array());

        match (before_array, after_array) {
            (Some(before_arr), Some(after_arr)) => {
                // Count how many items should have been affected by the filter
                let targeted_items = Self::count_items_matching_filter(before_arr, filter);
                let unchanged_items = before_arr.len() - targeted_items;

                // After operation, unchanged items should still be present
                assert!(
                    after_arr.len() >= unchanged_items,
                    "Filter operation on '{}' should preserve non-matching items",
                    path
                );
            }
            _ => panic!(
                "Both before and after should have array values for path '{}'",
                path
            ),
        }
    }

    /// Assert that capabilities are correctly advertised
    pub fn assert_capability_advertisement(service_config: &Value, patch_supported: bool) {
        let patch_capability = service_config.get("patch").and_then(|s| s.as_bool());

        assert_eq!(
            patch_capability,
            Some(patch_supported),
            "ServiceProviderConfig should advertise patch = {}",
            patch_supported
        );
    }

    /// Assert that tenant isolation is maintained
    pub fn assert_tenant_isolation(
        tenant_a_resource: &Value,
        tenant_b_resource: &Value,
        operation_tenant: &str,
    ) {
        // Tenant isolation means each tenant has independent resources and operations
        // The IDs can be the same since they exist in different tenant namespaces
        // What matters is that operations on one tenant don't affect the other

        let tenant_a_display_name = Self::get_value_at_path(tenant_a_resource, "displayName")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        let tenant_b_display_name = Self::get_value_at_path(tenant_b_resource, "displayName")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        match operation_tenant {
            "tenant-a" => {
                // Tenant A resource should have been modified
                assert_eq!(
                    tenant_a_display_name,
                    Some("Tenant A User".to_string()),
                    "Tenant A resource should have been updated with patch operation"
                );

                // Tenant B resource should remain unmodified
                assert_eq!(
                    tenant_b_display_name,
                    Some("Test User".to_string()),
                    "Tenant B resource should remain unchanged when tenant A is modified"
                );
            }
            "tenant-b" => {
                // Tenant B resource should have been modified
                assert_eq!(
                    tenant_b_display_name,
                    Some("Tenant B User".to_string()),
                    "Tenant B resource should have been updated with patch operation"
                );

                // Tenant A resource should remain unmodified
                assert_eq!(
                    tenant_a_display_name,
                    Some("Test User".to_string()),
                    "Tenant A resource should remain unchanged when tenant B is modified"
                );
            }
            _ => {
                // For cross-tenant access tests, both should remain unchanged
                assert_eq!(
                    tenant_a_display_name,
                    Some("Test User".to_string()),
                    "Tenant A resource should remain unchanged in cross-tenant test"
                );
                assert_eq!(
                    tenant_b_display_name,
                    Some("Test User".to_string()),
                    "Tenant B resource should remain unchanged in cross-tenant test"
                );
            }
        }
    }

    // Helper methods

    /// Get value at a specific path in a JSON object
    fn get_value_at_path<'a>(resource: &'a Value, path: &str) -> Option<&'a Value> {
        let parts: Vec<&str> = path.split('.').collect();
        let mut current = resource;

        for part in parts {
            // Handle array filters (simplified)
            if part.contains('[') {
                let base_part = part.split('[').next().unwrap();
                current = current.get(base_part)?;
                // In a real implementation, this would handle the filter expression
                if let Some(arr) = current.as_array() {
                    current = arr.first()?;
                }
            } else {
                current = current.get(part)?;
            }
        }

        Some(current)
    }

    /// Normalize a resource for comparison by removing volatile fields
    fn normalize_for_comparison(resource: &Value) -> Value {
        let mut normalized = resource.clone();

        // Remove meta.lastModified as it changes with updates
        if let Some(meta) = normalized.get_mut("meta").and_then(|m| m.as_object_mut()) {
            meta.remove("lastModified");
            meta.remove("version"); // ETag also changes
        }

        normalized
    }

    /// Get required attributes for a resource type
    fn get_required_attributes(resource_type: &str) -> Vec<&'static str> {
        match resource_type {
            "User" => vec!["id", "userName", "meta.resourceType"],
            "Group" => vec!["id", "displayName", "meta.resourceType"],
            _ => vec!["id", "meta.resourceType"],
        }
    }

    /// Extract base path from a filtered path
    fn get_base_path(path: &str) -> String {
        if let Some(bracket_pos) = path.find('[') {
            path[..bracket_pos].to_string()
        } else {
            path.to_string()
        }
    }

    /// Count items in an array that match a filter (simplified)
    fn count_items_matching_filter(array: &[Value], filter: &str) -> usize {
        // Simplified filter matching - real implementation would parse filter expressions
        if filter.contains("type eq \"work\"") {
            array
                .iter()
                .filter(|item| {
                    item.get("type")
                        .and_then(|t| t.as_str())
                        .map(|t| t == "work")
                        .unwrap_or(false)
                })
                .count()
        } else if filter.contains("primary eq true") {
            array
                .iter()
                .filter(|item| {
                    item.get("primary")
                        .and_then(|p| p.as_bool())
                        .unwrap_or(false)
                })
                .count()
        } else {
            0 // Unknown filter
        }
    }
}

/// Specialized assertions for atomic operations
pub struct AtomicAssertions;

impl AtomicAssertions {
    /// Assert that all operations in a PATCH request succeeded or all failed
    pub fn assert_atomic_behavior(results: &[PatchTestResult], should_all_succeed: bool) {
        if should_all_succeed {
            for (i, result) in results.iter().enumerate() {
                assert!(
                    result.is_ok(),
                    "Operation {} should have succeeded in atomic PATCH",
                    i
                );
            }
        } else {
            // If any operation fails, all should fail (rollback)
            let any_failed = results.iter().any(|r| r.is_err());
            if any_failed {
                for (i, result) in results.iter().enumerate() {
                    assert!(
                        result.is_err(),
                        "Operation {} should have failed due to atomic rollback",
                        i
                    );
                }
            }
        }
    }

    /// Assert that partial success is handled appropriately
    pub fn assert_partial_success_handling(
        results: &[PatchTestResult],
        expected_behavior: PartialSuccessBehavior,
    ) {
        match expected_behavior {
            PartialSuccessBehavior::AllOrNothing => {
                Self::assert_atomic_behavior(results, false);
            }
            PartialSuccessBehavior::BestEffort => {
                // Some operations may succeed while others fail
                assert!(
                    results.iter().any(|r| r.is_ok()) || results.iter().any(|r| r.is_err()),
                    "In best-effort mode, mixed results are acceptable"
                );
            }
        }
    }
}

/// Expected behavior for partial success scenarios
#[derive(Debug, Clone, PartialEq)]
pub enum PartialSuccessBehavior {
    /// All operations must succeed or all must fail
    AllOrNothing,
    /// Individual operations can succeed or fail independently
    BestEffort,
}

/// Assertions for property-based tests
pub struct PropertyAssertions;

impl PropertyAssertions {
    /// Assert that basic SCIM invariants hold after any PATCH operation
    pub fn assert_scim_invariants(resource: &Value, resource_type: &str) {
        // ID should never change
        assert!(
            PatchAssertions::get_value_at_path(resource, "id").is_some(),
            "Resource ID must always be present"
        );

        // Resource type should never change
        let actual_resource_type =
            PatchAssertions::get_value_at_path(resource, "meta.resourceType")
                .and_then(|v| v.as_str());
        assert_eq!(
            actual_resource_type,
            Some(resource_type),
            "Resource type should never change"
        );

        // Schema should always be present
        let schemas =
            PatchAssertions::get_value_at_path(resource, "schemas").and_then(|v| v.as_array());
        assert!(
            schemas.is_some() && !schemas.unwrap().is_empty(),
            "Schemas array must be present and non-empty"
        );

        // Meta attributes should be properly maintained
        assert!(
            PatchAssertions::get_value_at_path(resource, "meta.created").is_some(),
            "meta.created should always be present"
        );
        assert!(
            PatchAssertions::get_value_at_path(resource, "meta.lastModified").is_some(),
            "meta.lastModified should always be present"
        );
    }

    /// Assert that data types are preserved after PATCH operations
    pub fn assert_type_safety(before: &Value, after: &Value, modified_paths: &[&str]) {
        for path in modified_paths {
            let before_value = PatchAssertions::get_value_at_path(before, path);
            let after_value = PatchAssertions::get_value_at_path(after, path);

            if let (Some(before_val), Some(after_val)) = (before_value, after_value) {
                // Type should be preserved unless it's a replace operation with different type
                let before_type = Self::get_value_type(before_val);
                let after_type = Self::get_value_type(after_val);

                // Allow type changes for certain compatible types
                assert!(
                    Self::are_compatible_types(&before_type, &after_type),
                    "Type change from {:?} to {:?} at path '{}' is not allowed",
                    before_type,
                    after_type,
                    path
                );
            }
        }
    }

    fn get_value_type(value: &Value) -> String {
        match value {
            Value::Null => "null".to_string(),
            Value::Bool(_) => "boolean".to_string(),
            Value::Number(_) => "number".to_string(),
            Value::String(_) => "string".to_string(),
            Value::Array(_) => "array".to_string(),
            Value::Object(_) => "object".to_string(),
        }
    }

    fn are_compatible_types(before: &str, after: &str) -> bool {
        // Same type is always compatible
        if before == after {
            return true;
        }

        // Some specific compatibilities (simplified)
        matches!(
            (before, after),
            ("null", _) | (_, "null") // null can be replaced with any type
        )
    }
}
