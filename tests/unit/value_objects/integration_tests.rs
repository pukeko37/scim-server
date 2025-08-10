//! Integration tests for value object integration with Resource creation and validation.
//!
//! This module tests the Phase 2 clean break implementation where value objects are
//! core members of the Resource struct and validation happens during Resource construction.

use scim_server::resource::core::Resource;
use scim_server::schema::registry::SchemaRegistry;
use serde_json::json;

/// Test successful Resource creation with valid value objects
#[test]
fn test_resource_creation_with_valid_value_objects() {
    // Create a valid user resource JSON
    let user_data = json!({
        "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
        "id": "2819c223-7f76-453a-919d-413861904646",
        "userName": "bjensen@example.com",
        "externalId": "701984",
        "displayName": "Barbara Jensen",
        "emails": [{
            "value": "bjensen@example.com",
            "type": "work",
            "primary": true
        }]
    });

    // Test Resource creation with value object validation
    let result = Resource::from_json("User".to_string(), user_data);
    assert!(
        result.is_ok(),
        "Resource creation should succeed with valid data"
    );

    let resource = result.unwrap();

    // Verify core value objects are properly extracted
    assert!(resource.id.is_some(), "Resource ID should be extracted");
    assert_eq!(
        resource.id.as_ref().unwrap().as_str(),
        "2819c223-7f76-453a-919d-413861904646"
    );

    assert!(resource.user_name.is_some(), "Username should be extracted");
    assert_eq!(
        resource.user_name.as_ref().unwrap().as_str(),
        "bjensen@example.com"
    );

    assert!(
        resource.external_id.is_some(),
        "External ID should be extracted"
    );
    assert_eq!(resource.external_id.as_ref().unwrap().as_str(), "701984");

    assert_eq!(resource.schemas.len(), 1);
    assert_eq!(
        resource.schemas[0].as_str(),
        "urn:ietf:params:scim:schemas:core:2.0:User"
    );
}

/// Test Resource creation fails with invalid value objects
#[test]
fn test_resource_creation_with_invalid_value_objects() {
    // Test invalid resource ID (empty string)
    let invalid_id_data = json!({
        "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
        "id": "",
        "userName": "bjensen@example.com"
    });

    let result = Resource::from_json("User".to_string(), invalid_id_data);
    assert!(
        result.is_err(),
        "Resource creation should fail with empty ID"
    );

    // Test invalid username (empty string)
    let invalid_username_data = json!({
        "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
        "id": "valid-id",
        "userName": ""
    });

    let result = Resource::from_json("User".to_string(), invalid_username_data);
    assert!(
        result.is_err(),
        "Resource creation should fail with empty username"
    );

    // Test invalid schema URI (empty string)
    let invalid_schema_data = json!({
        "schemas": [""],
        "id": "valid-id",
        "userName": "valid@example.com"
    });

    let result = Resource::from_json("User".to_string(), invalid_schema_data);
    assert!(
        result.is_err(),
        "Resource creation should fail with empty schema URI"
    );
}

/// Test Resource creation with missing required core fields
#[test]
fn test_resource_creation_with_missing_core_fields() {
    // Missing schemas (should get default)
    let no_schemas_data = json!({
        "id": "valid-id",
        "userName": "bjensen@example.com"
    });

    let result = Resource::from_json("User".to_string(), no_schemas_data);
    assert!(
        result.is_ok(),
        "Resource creation should succeed without explicit schemas"
    );

    let resource = result.unwrap();
    assert_eq!(resource.schemas.len(), 1);
    assert_eq!(
        resource.schemas[0].as_str(),
        "urn:ietf:params:scim:schemas:core:2.0:User"
    );

    // Missing userName (should be None for User resources)
    let no_username_data = json!({
        "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
        "id": "valid-id",
        "displayName": "Test User"
    });

    let result = Resource::from_json("User".to_string(), no_username_data);
    assert!(
        result.is_ok(),
        "Resource creation should succeed without username"
    );

    let resource = result.unwrap();
    assert!(
        resource.user_name.is_none(),
        "Username should be None when not provided"
    );
}

/// Test Resource creation with various external ID formats
#[test]
fn test_resource_creation_with_external_ids() {
    // Numeric external ID
    let numeric_external_id = json!({
        "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
        "userName": "user1@example.com",
        "externalId": "12345"
    });

    let result = Resource::from_json("User".to_string(), numeric_external_id);
    assert!(result.is_ok(), "Should handle numeric external ID");
    assert_eq!(result.unwrap().external_id.unwrap().as_str(), "12345");

    // Alphanumeric external ID
    let alpha_external_id = json!({
        "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
        "userName": "user2@example.com",
        "externalId": "EXT-ABC-123"
    });

    let result = Resource::from_json("User".to_string(), alpha_external_id);
    assert!(result.is_ok(), "Should handle alphanumeric external ID");
    assert_eq!(result.unwrap().external_id.unwrap().as_str(), "EXT-ABC-123");

    // UUID external ID
    let uuid_external_id = json!({
        "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
        "userName": "user3@example.com",
        "externalId": "550e8400-e29b-41d4-a716-446655440000"
    });

    let result = Resource::from_json("User".to_string(), uuid_external_id);
    assert!(result.is_ok(), "Should handle UUID external ID");
    assert_eq!(
        result.unwrap().external_id.unwrap().as_str(),
        "550e8400-e29b-41d4-a716-446655440000"
    );
}

/// Test Resource schema validation integration
#[test]
fn test_resource_with_schema_validation() {
    let registry = SchemaRegistry::new().expect("Failed to create registry");

    // Create resource with valid data
    let valid_data = json!({
        "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
        "userName": "testuser@example.com",
        "name": {
            "givenName": "Test",
            "familyName": "User"
        },
        "emails": [{
            "value": "test@example.com",
            "type": "work"
        }]
    });

    let resource = Resource::from_json("User".to_string(), valid_data)
        .expect("Resource creation should succeed");

    // Test schema validation using the new hybrid approach
    let validation_result = registry.validate_resource_hybrid(&resource);
    assert!(
        validation_result.is_ok(),
        "Schema validation should pass for valid resource"
    );
}

/// Test Resource email extraction functionality
#[test]
fn test_resource_email_extraction() {
    let email_data = json!({
        "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
        "userName": "multi@example.com",
        "emails": [
            {
                "value": "work@example.com",
                "type": "work",
                "primary": true
            },
            {
                "value": "home@example.com",
                "type": "home",
                "primary": false
            }
        ]
    });

    let resource = Resource::from_json("User".to_string(), email_data)
        .expect("Resource creation should succeed");

    let emails = resource.get_emails();
    let emails = emails.expect("Should have emails");
    assert_eq!(emails.len(), 2, "Should extract both email addresses");

    let email0 = emails.get(0).expect("Should have first email");
    assert_eq!(email0.value(), "work@example.com");

    let email1 = emails.get(1).expect("Should have second email");
    assert_eq!(email1.value(), "home@example.com");
}

/// Test Resource active status handling
#[test]
fn test_resource_active_status() {
    // Explicitly active user
    let active_data = json!({
        "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
        "userName": "active@example.com",
        "active": true
    });

    let resource = Resource::from_json("User".to_string(), active_data)
        .expect("Resource creation should succeed");
    assert!(
        resource.is_active(),
        "User should be active when explicitly set"
    );

    // Explicitly inactive user
    let inactive_data = json!({
        "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
        "userName": "inactive@example.com",
        "active": false
    });

    let resource = Resource::from_json("User".to_string(), inactive_data)
        .expect("Resource creation should succeed");
    assert!(
        !resource.is_active(),
        "User should be inactive when explicitly set"
    );

    // User without active field (should default to true)
    let default_data = json!({
        "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
        "userName": "default@example.com"
    });

    let resource = Resource::from_json("User".to_string(), default_data)
        .expect("Resource creation should succeed");
    assert!(resource.is_active(), "User should default to active");
}

/// Test Resource attribute access
#[test]
fn test_resource_attribute_access() {
    let user_data = json!({
        "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
        "userName": "testuser@example.com",
        "displayName": "Test User",
        "title": "Software Engineer",
        "department": "Engineering"
    });

    let mut resource = Resource::from_json("User".to_string(), user_data)
        .expect("Resource creation should succeed");

    // Test getting attributes
    assert_eq!(
        resource
            .get_attribute("displayName")
            .and_then(|v| v.as_str()),
        Some("Test User")
    );
    assert_eq!(
        resource.get_attribute("title").and_then(|v| v.as_str()),
        Some("Software Engineer")
    );

    // Test setting attributes
    resource.set_attribute("title".to_string(), json!("Senior Software Engineer"));
    assert_eq!(
        resource.get_attribute("title").and_then(|v| v.as_str()),
        Some("Senior Software Engineer")
    );

    // Test core fields are not in attributes (they're separate fields)
    assert!(
        resource.get_attribute("userName").is_none(),
        "userName should not be in attributes"
    );
    assert!(
        resource.get_attribute("id").is_none(),
        "id should not be in attributes"
    );
    assert!(
        resource.get_attribute("schemas").is_none(),
        "schemas should not be in attributes"
    );
}

/// Test Group resource creation (to test resource type flexibility)
#[test]
fn test_group_resource_creation() {
    let group_data = json!({
        "schemas": ["urn:ietf:params:scim:schemas:core:2.0:Group"],
        "displayName": "Engineering Team",
        "members": [
            {
                "value": "user1",
                "type": "User"
            }
        ]
    });

    let resource = Resource::from_json("Group".to_string(), group_data)
        .expect("Group resource creation should succeed");

    assert_eq!(resource.resource_type, "Group");
    assert_eq!(resource.schemas.len(), 1);
    assert_eq!(
        resource.schemas[0].as_str(),
        "urn:ietf:params:scim:schemas:core:2.0:Group"
    );

    // Group resources should not have username
    assert!(
        resource.user_name.is_none(),
        "Groups should not have usernames"
    );

    // But should have displayName in attributes
    assert_eq!(
        resource
            .get_attribute("displayName")
            .and_then(|v| v.as_str()),
        Some("Engineering Team")
    );
}

/// Test Resource serialization round-trip
#[test]
fn test_resource_serialization_round_trip() {
    let original_data = json!({
        "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
        "id": "test-id-123",
        "userName": "roundtrip@example.com",
        "externalId": "EXT-123",
        "displayName": "Round Trip User",
        "emails": [{
            "value": "rt@example.com",
            "type": "work",
            "primary": true
        }]
    });

    // Create resource from JSON
    let resource = Resource::from_json("User".to_string(), original_data.clone())
        .expect("Resource creation should succeed");

    // Serialize back to JSON
    let serialized = resource.to_json().unwrap();

    // Verify core fields are preserved
    assert_eq!(
        serialized["schemas"][0],
        "urn:ietf:params:scim:schemas:core:2.0:User"
    );
    assert_eq!(serialized["id"], "test-id-123");
    assert_eq!(serialized["userName"], "roundtrip@example.com");
    assert_eq!(serialized["externalId"], "EXT-123");
    assert_eq!(serialized["displayName"], "Round Trip User");
    assert_eq!(serialized["emails"][0]["value"], "rt@example.com");
}
