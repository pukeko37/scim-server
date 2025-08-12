//! Unit tests for the operation handler module.

use scim_server::ScimServer;
use scim_server::multi_tenant::ScimOperation;
use scim_server::operation_handler::{ScimOperationHandler, ScimOperationRequest};
use scim_server::providers::InMemoryProvider;
use scim_server::resource_handlers::create_user_resource_handler;
use serde_json::json;

#[tokio::test]
async fn test_operation_handler_create() {
    let provider = InMemoryProvider::new();
    let mut server = ScimServer::new(provider).unwrap();

    // Register User resource type
    let user_schema = server
        .get_schema_by_id("urn:ietf:params:scim:schemas:core:2.0:User")
        .unwrap()
        .clone();
    let user_handler = create_user_resource_handler(user_schema);
    server
        .register_resource_type("User", user_handler, vec![ScimOperation::Create])
        .unwrap();

    let handler = ScimOperationHandler::new(server);

    let request = ScimOperationRequest::create(
        "User",
        json!({
            "userName": "testuser",
            "name": {
                "givenName": "Test",
                "familyName": "User"
            }
        }),
    );

    let response = handler.handle_operation(request).await;
    assert!(response.success);
    assert!(response.data.is_some());
    assert!(response.error.is_none());
}

#[tokio::test]
async fn test_operation_handler_get_schemas() {
    let provider = InMemoryProvider::new();
    let server = ScimServer::new(provider).unwrap();
    let handler = ScimOperationHandler::new(server);

    let request = ScimOperationRequest::get_schemas();
    let response = handler.handle_operation(request).await;

    assert!(response.success);
    assert!(response.data.is_some());
    if let Some(data) = response.data {
        assert!(data.get("schemas").is_some());
    }
}

#[tokio::test]
async fn test_operation_handler_error_handling() {
    let provider = InMemoryProvider::new();
    let server = ScimServer::new(provider).unwrap();
    let handler = ScimOperationHandler::new(server);

    // Try to get a non-existent resource
    let request = ScimOperationRequest::get("User", "non-existent-id");
    let response = handler.handle_operation(request).await;

    assert!(!response.success);
    assert!(response.error.is_some());
    assert!(response.error_code.is_some());
}

#[tokio::test]
async fn test_conditional_update_with_correct_version() {
    use scim_server::resource::version::ScimVersion;

    let provider = InMemoryProvider::new();
    let mut server = ScimServer::new(provider).unwrap();

    // Register User resource type
    let user_schema = server
        .get_schema_by_id("urn:ietf:params:scim:schemas:core:2.0:User")
        .unwrap()
        .clone();
    let user_handler = create_user_resource_handler(user_schema);
    server
        .register_resource_type(
            "User",
            user_handler,
            vec![
                ScimOperation::Create,
                ScimOperation::Update,
                ScimOperation::Read,
            ],
        )
        .unwrap();

    let handler = ScimOperationHandler::new(server);

    // Create a user first
    let create_request = ScimOperationRequest::create(
        "User",
        json!({
            "userName": "testuser",
            "name": {
                "givenName": "Test",
                "familyName": "User"
            }
        }),
    );

    let create_response = handler.handle_operation(create_request).await;
    assert!(create_response.success);

    let user_data = create_response.data.unwrap();
    let user_id = user_data["id"].as_str().unwrap();

    // Get the user to obtain current version
    let get_request = ScimOperationRequest::get("User", user_id);
    let get_response = handler.handle_operation(get_request).await;
    assert!(get_response.success);

    // Extract current version from response metadata
    let current_version = get_response
        .metadata
        .additional
        .get("version")
        .and_then(|v| v.as_str())
        .map(|v| ScimVersion::from_hash(v))
        .expect("Response should include version information");

    // Update with correct version should succeed
    let update_request = ScimOperationRequest::update(
        "User",
        user_id,
        json!({
            "userName": "updateduser",
            "name": {
                "givenName": "Updated",
                "familyName": "User"
            }
        }),
    )
    .with_expected_version(current_version);

    let update_response = handler.handle_operation(update_request).await;
    assert!(update_response.success);
    assert!(update_response.metadata.additional.contains_key("version"));
    assert!(update_response.metadata.additional.contains_key("etag"));
}

#[tokio::test]
async fn test_conditional_update_version_mismatch() {
    use scim_server::resource::version::ScimVersion;

    let provider = InMemoryProvider::new();
    let mut server = ScimServer::new(provider).unwrap();

    // Register User resource type
    let user_schema = server
        .get_schema_by_id("urn:ietf:params:scim:schemas:core:2.0:User")
        .unwrap()
        .clone();
    let user_handler = create_user_resource_handler(user_schema);
    server
        .register_resource_type(
            "User",
            user_handler,
            vec![ScimOperation::Create, ScimOperation::Update],
        )
        .unwrap();

    let handler = ScimOperationHandler::new(server);

    // Create a user first
    let create_request = ScimOperationRequest::create(
        "User",
        json!({
            "userName": "testuser",
            "name": {
                "givenName": "Test",
                "familyName": "User"
            }
        }),
    );

    let create_response = handler.handle_operation(create_request).await;
    assert!(create_response.success);

    let user_data = create_response.data.unwrap();
    let user_id = user_data["id"].as_str().unwrap();

    // Try to update with incorrect version should fail with version mismatch
    let old_version = ScimVersion::from_hash("incorrect-version");
    let update_request = ScimOperationRequest::update(
        "User",
        user_id,
        json!({
            "userName": "updateduser"
        }),
    )
    .with_expected_version(old_version);

    let update_response = handler.handle_operation(update_request).await;
    assert!(!update_response.success);
    assert_eq!(
        update_response.error_code.as_deref(),
        Some("version_mismatch")
    );
    assert!(update_response.error.is_some());
    assert!(
        update_response
            .metadata
            .additional
            .contains_key("expected_version")
    );
    assert!(
        update_response
            .metadata
            .additional
            .contains_key("current_version")
    );
}

#[tokio::test]
async fn test_conditional_delete_with_correct_version() {
    use scim_server::resource::version::ScimVersion;

    let provider = InMemoryProvider::new();
    let mut server = ScimServer::new(provider).unwrap();

    // Register User resource type
    let user_schema = server
        .get_schema_by_id("urn:ietf:params:scim:schemas:core:2.0:User")
        .unwrap()
        .clone();
    let user_handler = create_user_resource_handler(user_schema);
    server
        .register_resource_type(
            "User",
            user_handler,
            vec![
                ScimOperation::Create,
                ScimOperation::Delete,
                ScimOperation::Read,
            ],
        )
        .unwrap();

    let handler = ScimOperationHandler::new(server);

    // Create a user first
    let create_request = ScimOperationRequest::create(
        "User",
        json!({
            "userName": "testuser",
            "name": {
                "givenName": "Test",
                "familyName": "User"
            }
        }),
    );

    let create_response = handler.handle_operation(create_request).await;
    assert!(create_response.success);

    let user_data = create_response.data.unwrap();
    let user_id = user_data["id"].as_str().unwrap();

    // Get the user to obtain current version
    let get_request = ScimOperationRequest::get("User", user_id);
    let get_response = handler.handle_operation(get_request).await;
    assert!(get_response.success);

    // Extract current version from response metadata
    let current_version = get_response
        .metadata
        .additional
        .get("version")
        .and_then(|v| v.as_str())
        .map(|v| ScimVersion::from_hash(v))
        .expect("Response should include version information");

    // Delete with correct version should succeed
    let delete_request =
        ScimOperationRequest::delete("User", user_id).with_expected_version(current_version);

    let delete_response = handler.handle_operation(delete_request).await;
    assert!(delete_response.success);
}

#[tokio::test]
async fn test_conditional_delete_version_mismatch() {
    use scim_server::resource::version::ScimVersion;

    let provider = InMemoryProvider::new();
    let mut server = ScimServer::new(provider).unwrap();

    // Register User resource type
    let user_schema = server
        .get_schema_by_id("urn:ietf:params:scim:schemas:core:2.0:User")
        .unwrap()
        .clone();
    let user_handler = create_user_resource_handler(user_schema);
    server
        .register_resource_type(
            "User",
            user_handler,
            vec![ScimOperation::Create, ScimOperation::Delete],
        )
        .unwrap();

    let handler = ScimOperationHandler::new(server);

    // Create a user first
    let create_request = ScimOperationRequest::create(
        "User",
        json!({
            "userName": "testuser",
            "name": {
                "givenName": "Test",
                "familyName": "User"
            }
        }),
    );

    let create_response = handler.handle_operation(create_request).await;
    assert!(create_response.success);

    let user_data = create_response.data.unwrap();
    let user_id = user_data["id"].as_str().unwrap();

    // Try to delete with incorrect version should fail with version mismatch
    let old_version = ScimVersion::from_hash("incorrect-version");
    let delete_request =
        ScimOperationRequest::delete("User", user_id).with_expected_version(old_version);

    let delete_response = handler.handle_operation(delete_request).await;
    assert!(!delete_response.success);
    assert_eq!(
        delete_response.error_code.as_deref(),
        Some("version_mismatch")
    );
    assert!(delete_response.error.is_some());
    assert!(
        delete_response
            .metadata
            .additional
            .contains_key("expected_version")
    );
    assert!(
        delete_response
            .metadata
            .additional
            .contains_key("current_version")
    );
}

#[tokio::test]
async fn test_regular_operations_include_version_info() {
    let provider = InMemoryProvider::new();
    let mut server = ScimServer::new(provider).unwrap();

    // Register User resource type
    let user_schema = server
        .get_schema_by_id("urn:ietf:params:scim:schemas:core:2.0:User")
        .unwrap()
        .clone();
    let user_handler = create_user_resource_handler(user_schema);
    server
        .register_resource_type(
            "User",
            user_handler,
            vec![
                ScimOperation::Create,
                ScimOperation::Update,
                ScimOperation::Read,
            ],
        )
        .unwrap();

    let handler = ScimOperationHandler::new(server);

    // Create a user (should include version info)
    let create_request = ScimOperationRequest::create(
        "User",
        json!({
            "userName": "testuser",
            "name": {
                "givenName": "Test",
                "familyName": "User"
            }
        }),
    );

    let create_response = handler.handle_operation(create_request).await;
    assert!(create_response.success);
    // CREATE operations should include version info too
    assert!(create_response.metadata.additional.contains_key("version"));
    assert!(create_response.metadata.additional.contains_key("etag"));

    let user_data = create_response.data.unwrap();
    let user_id = user_data["id"].as_str().unwrap();

    // Get user (should include version info)
    let get_request = ScimOperationRequest::get("User", user_id);
    let get_response = handler.handle_operation(get_request).await;
    assert!(get_response.success);
    assert!(get_response.metadata.additional.contains_key("version"));
    assert!(get_response.metadata.additional.contains_key("etag"));

    // Update without expected_version (should still include version info)
    let update_request = ScimOperationRequest::update(
        "User",
        user_id,
        json!({
            "userName": "updateduser",
            "name": {
                "givenName": "Updated",
                "familyName": "User"
            }
        }),
    );

    let update_response = handler.handle_operation(update_request).await;
    assert!(update_response.success);
    assert!(update_response.metadata.additional.contains_key("version"));
    assert!(update_response.metadata.additional.contains_key("etag"));
}

#[tokio::test]
async fn test_phase_3_complete_integration() {
    // Comprehensive test demonstrating complete Phase 3 ETag functionality
    use scim_server::resource::version::ScimVersion;

    let provider = InMemoryProvider::new();
    let mut server = ScimServer::new(provider).unwrap();

    // Register User resource type with all operations
    let user_schema = server
        .get_schema_by_id("urn:ietf:params:scim:schemas:core:2.0:User")
        .unwrap()
        .clone();
    let user_handler = create_user_resource_handler(user_schema);
    server
        .register_resource_type(
            "User",
            user_handler,
            vec![
                ScimOperation::Create,
                ScimOperation::Read,
                ScimOperation::Update,
                ScimOperation::Delete,
            ],
        )
        .unwrap();

    let handler = ScimOperationHandler::new(server);

    // 1. Create user - should return version information
    let create_request = ScimOperationRequest::create(
        "User",
        json!({
            "userName": "integration.test",
            "name": {
                "givenName": "Integration",
                "familyName": "Test"
            },
            "active": true
        }),
    );

    let create_response = handler.handle_operation(create_request).await;
    assert!(create_response.success);
    assert!(create_response.metadata.additional.contains_key("version"));
    assert!(create_response.metadata.additional.contains_key("etag"));

    let user_data = create_response.data.unwrap();
    let user_id = user_data["id"].as_str().unwrap();
    let v1_etag = create_response.metadata.additional["etag"]
        .as_str()
        .unwrap();

    // 2. Get user - should return same version
    let get_request = ScimOperationRequest::get("User", user_id);
    let get_response = handler.handle_operation(get_request).await;
    assert!(get_response.success);
    assert_eq!(
        get_response.metadata.additional["etag"].as_str().unwrap(),
        v1_etag
    );

    // 3. Regular update (no expected_version) - should succeed and return new version
    let v1_version = ScimVersion::from_hash(
        create_response.metadata.additional["version"]
            .as_str()
            .unwrap(),
    );

    let update1_request = ScimOperationRequest::update(
        "User",
        user_id,
        json!({
            "userName": "integration.updated",
            "name": {
                "givenName": "Integration",
                "familyName": "Updated"
            },
            "active": true
        }),
    );

    let update1_response = handler.handle_operation(update1_request).await;
    assert!(update1_response.success);
    assert!(update1_response.metadata.additional.contains_key("version"));
    let v2_etag = update1_response.metadata.additional["etag"]
        .as_str()
        .unwrap();
    assert_ne!(v1_etag, v2_etag); // Version should have changed

    // 4. Conditional update with correct version - should succeed
    let v2_version = ScimVersion::from_hash(
        update1_response.metadata.additional["version"]
            .as_str()
            .unwrap(),
    );

    let conditional_update_request = ScimOperationRequest::update(
        "User",
        user_id,
        json!({
            "userName": "integration.conditional",
            "active": false
        }),
    )
    .with_expected_version(v2_version);

    let conditional_update_response = handler.handle_operation(conditional_update_request).await;
    assert!(conditional_update_response.success);
    let v3_etag = conditional_update_response.metadata.additional["etag"]
        .as_str()
        .unwrap();
    assert_ne!(v2_etag, v3_etag); // Version should have changed again

    // 5. Conditional update with old version - should fail
    let stale_update_request = ScimOperationRequest::update(
        "User",
        user_id,
        json!({
            "userName": "should.fail"
        }),
    )
    .with_expected_version(v1_version); // Using old version

    let stale_update_response = handler.handle_operation(stale_update_request).await;
    assert!(!stale_update_response.success);
    assert_eq!(
        stale_update_response.error_code.as_deref(),
        Some("version_mismatch")
    );
    assert!(
        stale_update_response
            .metadata
            .additional
            .contains_key("expected_version")
    );
    assert!(
        stale_update_response
            .metadata
            .additional
            .contains_key("current_version")
    );

    // 6. Conditional delete with correct version - should succeed
    let v3_version = ScimVersion::from_hash(
        conditional_update_response.metadata.additional["version"]
            .as_str()
            .unwrap(),
    );

    let conditional_delete_request =
        ScimOperationRequest::delete("User", user_id).with_expected_version(v3_version);

    let conditional_delete_response = handler.handle_operation(conditional_delete_request).await;
    assert!(conditional_delete_response.success);

    // 7. Verify user is actually deleted
    let verify_request = ScimOperationRequest::get("User", user_id);
    let verify_response = handler.handle_operation(verify_request).await;
    assert!(!verify_response.success);
    assert!(
        verify_response
            .error_code
            .as_deref()
            .unwrap()
            .contains("NOT_FOUND")
    );
}
