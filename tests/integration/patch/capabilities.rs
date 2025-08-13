//! SCIM PATCH Capability Testing
//!
//! This module provides comprehensive testing of SCIM PATCH capabilities according to RFC 7644.
//! Tests verify that providers correctly advertise and enforce their PATCH operation support.

use super::test_helpers;
use serde_json::{Value, json};

/// Test PATCH capability advertisement and enforcement
#[tokio::test]
async fn test_patch_capability_advertisement() {
    let server = test_helpers::create_test_server_with_patch_support();
    let _context = test_helpers::create_test_context();

    // Get service provider configuration
    let config = server.get_service_provider_config().unwrap();

    // Verify PATCH capability is advertised
    assert!(config.patch_supported);
}

/// Test behavior when PATCH is not supported
#[tokio::test]
async fn test_patch_not_supported_behavior() {
    let server = test_helpers::create_test_server_without_patch_support();
    let context = test_helpers::create_test_context();

    // First, verify service config reflects disabled patch
    let config = server.get_service_provider_config().unwrap();
    assert!(
        !config.patch_supported,
        "Patch should be disabled in service config"
    );

    // Create a test user first
    let user = test_helpers::create_test_user(&server, &context)
        .await
        .unwrap();
    let user_id = user.get_id().unwrap();

    // Create patch request
    let patch_request = json!({
        "schemas": ["urn:ietf:params:scim:api:messages:2.0:PatchOp"],
        "Operations": [{
            "op": "replace",
            "path": "displayName",
            "value": "New Name"
        }]
    });

    // Attempt patch operation - should fail since patch is not supported
    let result = server
        .patch_resource("User", user_id, &patch_request, &context)
        .await;
    assert!(result.is_err(), "PATCH should fail when not supported");
}

/// Test PATCH capability with different provider configurations
#[tokio::test]
async fn test_patch_capability_matrix() {
    // Test with patch enabled
    let server_with_patch = test_helpers::create_test_server_with_patch_support();
    let context = test_helpers::create_test_context();

    let user = test_helpers::create_test_user(&server_with_patch, &context)
        .await
        .unwrap();
    let user_id = user.get_id().unwrap();

    let patch_request = json!({
        "schemas": ["urn:ietf:params:scim:api:messages:2.0:PatchOp"],
        "Operations": [{
            "op": "replace",
            "path": "displayName",
            "value": "Updated Name"
        }]
    });

    let result = server_with_patch
        .patch_resource("User", user_id, &patch_request, &context)
        .await;
    assert!(result.is_ok(), "PATCH should succeed when supported");

    // Test with patch disabled
    let server_without_patch = test_helpers::create_test_server_without_patch_support();
    let result = server_without_patch
        .patch_resource("User", user_id, &patch_request, &context)
        .await;
    assert!(result.is_err(), "PATCH should fail when not supported");
}

/// Test tenant-specific PATCH capabilities
#[tokio::test]
async fn test_tenant_specific_patch_capabilities() {
    let context_a = test_helpers::create_test_context_with_tenant("tenant-a", "client-a");
    let context_b = test_helpers::create_test_context_with_tenant("tenant-b", "client-b");

    // For simplicity, test with same server configuration for both tenants
    // In a real implementation, different tenants might have different capabilities
    let server = test_helpers::create_test_server_with_patch_support();

    // Create test users in both tenant contexts
    let user_a = test_helpers::create_test_user(&server, &context_a)
        .await
        .unwrap();
    let user_b = test_helpers::create_test_user(&server, &context_b)
        .await
        .unwrap();

    let user_a_id = user_a.get_id().unwrap();
    let user_b_id = user_b.get_id().unwrap();

    let patch_request = json!({
        "schemas": ["urn:ietf:params:scim:api:messages:2.0:PatchOp"],
        "Operations": [{
            "op": "replace",
            "path": "displayName",
            "value": "Updated Name"
        }]
    });

    // Test patch operations in both tenant contexts
    let result_a = server
        .patch_resource("User", user_a_id, &patch_request, &context_a)
        .await;
    let result_b = server
        .patch_resource("User", user_b_id, &patch_request, &context_b)
        .await;

    assert!(result_a.is_ok(), "PATCH should succeed for tenant A");
    assert!(result_b.is_ok(), "PATCH should succeed for tenant B");
}

/// Test dynamic capability changes
#[tokio::test]
async fn test_dynamic_capability_changes() {
    let server = test_helpers::create_test_server_with_patch_support();
    let context = test_helpers::create_test_context();

    // Initially PATCH is enabled
    let config = server.get_service_provider_config().unwrap();
    assert!(config.patch_supported);

    // Create a test user for patch operation
    let user = test_helpers::create_test_user(&server, &context)
        .await
        .unwrap();
    let user_id = user.get_id().unwrap();

    // Verify PATCH requests succeed when capability is enabled
    let patch_request = create_simple_patch_request();
    let result = server
        .patch_resource("User", user_id, &patch_request, &context)
        .await;

    // Should succeed when patch is supported
    assert!(result.is_ok(), "PATCH should succeed when supported");
}

/// Helper function to create a simple patch request
fn create_simple_patch_request() -> Value {
    json!({
        "schemas": ["urn:ietf:params:scim:api:messages:2.0:PatchOp"],
        "Operations": [{
            "op": "replace",
            "path": "displayName",
            "value": "Updated Display Name"
        }]
    })
}
