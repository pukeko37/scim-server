//! Verification tests for the provider restructuring.
//!
//! These tests verify that the new ProviderError and ProviderStats types
//! work correctly and that the obsolete InMemoryError and InMemoryStats
//! have been successfully removed.

use scim_server::ResourceProvider;
use scim_server::providers::{ProviderError, ProviderStats, StandardResourceProvider};
use scim_server::resource::RequestContext;
use scim_server::storage::InMemoryStorage;
use serde_json::json;

#[tokio::test]
async fn test_provider_error_usage() {
    let storage = InMemoryStorage::new();
    let provider = StandardResourceProvider::new(storage);
    let context = RequestContext::with_generated_id();

    // Try to get a non-existent resource - should return Ok(None)
    let result = provider.get_resource("User", "nonexistent", &context).await;
    assert!(result.is_ok()); // get_resource returns Ok(None) for non-existent resources
    assert!(result.unwrap().is_none());

    // Try to update a non-existent resource - should return ProviderError::ResourceNotFound
    let update_data = json!({
        "userName": "test.user",
        "displayName": "Test User"
    });

    let result = provider
        .update_resource("User", "nonexistent", update_data, None, &context)
        .await;
    assert!(result.is_err());

    match result.unwrap_err() {
        ProviderError::ResourceNotFound {
            resource_type,
            id,
            tenant_id,
        } => {
            assert_eq!(resource_type, "User");
            assert_eq!(id, "nonexistent");
            assert_eq!(tenant_id, "default");
        }
        _ => panic!("Expected ResourceNotFound error"),
    }
}

#[tokio::test]
async fn test_provider_stats_usage() {
    let storage = InMemoryStorage::new();
    let provider = StandardResourceProvider::new(storage);
    let context = RequestContext::with_generated_id();

    // Initially empty
    let stats = provider.get_stats().await;
    assert!(stats.is_empty());
    assert_eq!(stats.tenant_count, 0);
    assert_eq!(stats.total_resources, 0);

    // Create some resources
    let user_data = json!({
        "userName": "john.doe",
        "displayName": "John Doe",
        "active": true
    });

    provider
        .create_resource("User", user_data.clone(), &context)
        .await
        .unwrap();

    let group_data = json!({
        "displayName": "Developers"
    });

    provider
        .create_resource("Group", group_data, &context)
        .await
        .unwrap();

    // Check stats
    let stats = provider.get_stats().await;
    assert!(!stats.is_empty());
    assert_eq!(stats.tenant_count, 1);
    assert_eq!(stats.total_resources, 2);
    assert_eq!(stats.resource_type_count, 2);
    assert!(stats.resource_types.contains(&"User".to_string()));
    assert!(stats.resource_types.contains(&"Group".to_string()));
}

#[tokio::test]
async fn test_provider_duplicate_error() {
    let storage = InMemoryStorage::new();
    let provider = StandardResourceProvider::new(storage);
    let context = RequestContext::with_generated_id();

    // Create a user
    let user_data = json!({
        "userName": "duplicate.user",
        "displayName": "First User"
    });

    provider
        .create_resource("User", user_data.clone(), &context)
        .await
        .unwrap();

    // Try to create another user with same username - should fail
    let duplicate_data = json!({
        "userName": "duplicate.user",
        "displayName": "Second User"
    });

    let result = provider
        .create_resource("User", duplicate_data, &context)
        .await;
    assert!(result.is_err());

    match result.unwrap_err() {
        ProviderError::DuplicateAttribute {
            attribute, value, ..
        } => {
            assert_eq!(attribute, "userName");
            assert_eq!(value, "duplicate.user");
        }
        _ => panic!("Expected DuplicateAttribute error"),
    }
}

#[tokio::test]
async fn test_provider_error_variants() {
    // Test that we can construct different error variants
    let error1 = ProviderError::InvalidData {
        message: "Test invalid data".to_string(),
    };

    let error2 = ProviderError::Internal {
        message: "Test internal error".to_string(),
    };

    let error3 = ProviderError::NotFound {
        resource_type: "User".to_string(),
        id: "123".to_string(),
    };

    // Verify Display implementations work
    assert!(error1.to_string().contains("Invalid resource data"));
    assert!(error2.to_string().contains("Internal error"));
    assert!(error3.to_string().contains("Resource not found"));
}

#[test]
fn test_provider_stats_default() {
    let stats = ProviderStats::default();
    assert!(stats.is_empty());
    assert_eq!(stats.tenant_count, 0);
    assert_eq!(stats.total_resources, 0);
    assert_eq!(stats.resource_type_count, 0);
    assert!(stats.resource_types.is_empty());
}
