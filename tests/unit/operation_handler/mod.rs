//! Unit tests for the operation handler module.

use scim_server::ScimServer;
use scim_server::multi_tenant::ScimOperation;
use scim_server::operation_handler::{ScimOperationHandler, ScimOperationRequest};
use scim_server::providers::StandardResourceProvider;
use scim_server::resource::version::RawVersion;
use scim_server::resource_handlers::{create_group_resource_handler, create_user_resource_handler};
use scim_server::storage::InMemoryStorage;
use scim_server::{ScimServerBuilder, TenantContext, TenantStrategy};
use serde_json::json;

#[tokio::test]
async fn test_operation_handler_create() {
    let storage = InMemoryStorage::new();
    let provider = StandardResourceProvider::new(storage);
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
    let storage = InMemoryStorage::new();
    let provider = StandardResourceProvider::new(storage);
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
    let storage = InMemoryStorage::new();
    let provider = StandardResourceProvider::new(storage);
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
    let storage = InMemoryStorage::new();
    let provider = StandardResourceProvider::new(storage);
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
        .map(|v| RawVersion::from_hash(v))
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
    let storage = InMemoryStorage::new();
    let provider = StandardResourceProvider::new(storage);
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
    let old_version = RawVersion::from_hash("incorrect-version");
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
    let storage = InMemoryStorage::new();
    let provider = StandardResourceProvider::new(storage);
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
        .map(|v| RawVersion::from_hash(v))
        .expect("Response should include version information");

    // Delete with correct version should succeed
    let delete_request =
        ScimOperationRequest::delete("User", user_id).with_expected_version(current_version);

    let delete_response = handler.handle_operation(delete_request).await;
    assert!(delete_response.success);
}

#[tokio::test]
async fn test_conditional_delete_version_mismatch() {
    let storage = InMemoryStorage::new();
    let provider = StandardResourceProvider::new(storage);
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
    let old_version = RawVersion::from_hash("incorrect-version");
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
    let storage = InMemoryStorage::new();
    let provider = StandardResourceProvider::new(storage);
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

    let storage = InMemoryStorage::new();
    let provider = StandardResourceProvider::new(storage);
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
    let v1_version = RawVersion::from_hash(
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
    let v2_version = RawVersion::from_hash(
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
    let v3_version = RawVersion::from_hash(
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

/// Test that operation handler create operation returns Groups with $ref fields in members array
#[tokio::test]
async fn test_operation_handler_create_group_includes_ref_fields() {
    let storage = InMemoryStorage::new();
    let provider = StandardResourceProvider::new(storage);

    // Create server with specific configuration for $ref generation
    let mut server = ScimServerBuilder::new(provider)
        .with_base_url("https://scim.company.com")
        .with_tenant_strategy(TenantStrategy::SingleTenant)
        .build()
        .unwrap();

    // Register User and Group resource types
    let user_schema = server
        .get_schema_by_id("urn:ietf:params:scim:schemas:core:2.0:User")
        .unwrap()
        .clone();
    let user_handler = create_user_resource_handler(user_schema);
    server
        .register_resource_type(
            "User",
            user_handler,
            vec![ScimOperation::Create, ScimOperation::Read],
        )
        .unwrap();

    let group_schema = server
        .get_schema_by_id("urn:ietf:params:scim:schemas:core:2.0:Group")
        .unwrap()
        .clone();
    let group_handler = create_group_resource_handler(group_schema);
    server
        .register_resource_type("Group", group_handler, vec![ScimOperation::Create])
        .unwrap();

    let handler = ScimOperationHandler::new(server);

    // First create a user to reference
    let user_request = ScimOperationRequest::create(
        "User",
        json!({
            "userName": "testuser@company.com",
            "name": {
                "givenName": "Test",
                "familyName": "User"
            }
        }),
    );

    let user_response = handler.handle_operation(user_request).await;
    assert!(user_response.success, "User creation should succeed");
    let user_id = user_response.metadata.resource_id.unwrap();

    // Create group with the user as a member (without $ref - should be added automatically)
    let group_request = ScimOperationRequest::create(
        "Group",
        json!({
            "displayName": "Test Engineering Team",
            "members": [{
                "value": user_id,
                "type": "User",
                "display": "Test User"
            }]
        }),
    );

    let group_response = handler.handle_operation(group_request).await;

    // Verify response succeeded
    assert!(group_response.success, "Group creation should succeed");
    assert!(
        group_response.data.is_some(),
        "Group response should contain data"
    );

    let group_data = group_response.data.unwrap();
    let members = group_data["members"].as_array().unwrap();
    let member = &members[0];

    // This is the key test - operation handler should include $ref fields
    assert!(
        member["$ref"].is_string(),
        "Operation handler should automatically generate $ref field"
    );

    let ref_url = member["$ref"].as_str().unwrap();
    let expected_url = format!("https://scim.company.com/v2/Users/{}", user_id);
    assert_eq!(
        ref_url, expected_url,
        "Operation handler $ref should use server configuration"
    );

    // Verify other member fields are preserved
    assert_eq!(member["value"], user_id);
    assert_eq!(member["type"], "User");
    assert_eq!(member["display"], "Test User");
}

/// Test that operation handler get operation returns Groups with $ref fields
#[tokio::test]
async fn test_operation_handler_get_group_includes_ref_fields() {
    let storage = InMemoryStorage::new();
    let provider = StandardResourceProvider::new(storage);

    let mut server = ScimServerBuilder::new(provider)
        .with_base_url("https://api.example.com")
        .with_tenant_strategy(TenantStrategy::SingleTenant)
        .build()
        .unwrap();

    // Register resource types
    let user_schema = server
        .get_schema_by_id("urn:ietf:params:scim:schemas:core:2.0:User")
        .unwrap()
        .clone();
    let user_handler = create_user_resource_handler(user_schema);
    server
        .register_resource_type(
            "User",
            user_handler,
            vec![ScimOperation::Create, ScimOperation::Read],
        )
        .unwrap();

    let group_schema = server
        .get_schema_by_id("urn:ietf:params:scim:schemas:core:2.0:Group")
        .unwrap()
        .clone();
    let group_handler = create_group_resource_handler(group_schema);
    server
        .register_resource_type(
            "Group",
            group_handler,
            vec![ScimOperation::Create, ScimOperation::Read],
        )
        .unwrap();

    let handler = ScimOperationHandler::new(server);

    // Create user and group first
    let user_request = ScimOperationRequest::create(
        "User",
        json!({
            "userName": "gettest@example.com",
            "name": { "givenName": "Get", "familyName": "Test" }
        }),
    );
    let user_response = handler.handle_operation(user_request).await;
    let user_id = user_response.metadata.resource_id.unwrap();

    let group_request = ScimOperationRequest::create(
        "Group",
        json!({
            "displayName": "Get Test Group",
            "members": [{
                "value": user_id,
                "type": "User",
                "display": "Get Test"
            }]
        }),
    );
    let group_response = handler.handle_operation(group_request).await;
    let group_id = group_response.metadata.resource_id.unwrap();

    // Now get the group - this should include $ref fields
    let get_request = ScimOperationRequest::get("Group", &group_id);
    let get_response = handler.handle_operation(get_request).await;

    assert!(get_response.success, "Group get should succeed");
    assert!(get_response.data.is_some(), "Group get should return data");

    let group_data = get_response.data.unwrap();
    let members = group_data["members"].as_array().unwrap();
    let member = &members[0];

    // The get operation should also include $ref fields
    assert!(
        member["$ref"].is_string(),
        "Operation handler get should include $ref fields"
    );

    let ref_url = member["$ref"].as_str().unwrap();
    let expected_url = format!("https://api.example.com/v2/Users/{}", user_id);
    assert_eq!(
        ref_url, expected_url,
        "Get operation $ref should use correct base URL"
    );
}

/// Test that operation handler works with multi-tenant $ref generation
#[tokio::test]
async fn test_operation_handler_multitenant_ref_fields() {
    let storage = InMemoryStorage::new();
    let provider = StandardResourceProvider::new(storage);

    // Configure for subdomain-based multi-tenancy
    let mut server = ScimServerBuilder::new(provider)
        .with_base_url("https://scim.example.com")
        .with_tenant_strategy(TenantStrategy::Subdomain)
        .build()
        .unwrap();

    // Register resource types
    let user_schema = server
        .get_schema_by_id("urn:ietf:params:scim:schemas:core:2.0:User")
        .unwrap()
        .clone();
    let user_handler = create_user_resource_handler(user_schema);
    server
        .register_resource_type("User", user_handler, vec![ScimOperation::Create])
        .unwrap();

    let group_schema = server
        .get_schema_by_id("urn:ietf:params:scim:schemas:core:2.0:Group")
        .unwrap()
        .clone();
    let group_handler = create_group_resource_handler(group_schema);
    server
        .register_resource_type("Group", group_handler, vec![ScimOperation::Create])
        .unwrap();

    let handler = ScimOperationHandler::new(server);

    // Create tenant context
    let tenant_context = TenantContext::new("acme-corp".to_string(), "client-123".to_string());

    // Create user in tenant
    let mut user_request = ScimOperationRequest::create(
        "User",
        json!({
            "userName": "tenant.user@acme.com",
            "name": { "givenName": "Tenant", "familyName": "User" }
        }),
    );
    user_request = user_request.with_tenant(tenant_context.clone());

    let user_response = handler.handle_operation(user_request).await;
    assert!(user_response.success);
    let user_id = user_response.metadata.resource_id.unwrap();

    // Create group in same tenant
    let mut group_request = ScimOperationRequest::create(
        "Group",
        json!({
            "displayName": "Tenant Test Group",
            "members": [{
                "value": user_id,
                "type": "User",
                "display": "Tenant User"
            }]
        }),
    );
    group_request = group_request.with_tenant(tenant_context);

    let group_response = handler.handle_operation(group_request).await;

    assert!(
        group_response.success,
        "Tenant group creation should succeed"
    );

    let group_data = group_response.data.unwrap();
    let members = group_data["members"].as_array().unwrap();
    let member = &members[0];

    // Should generate subdomain-based $ref URL
    assert!(
        member["$ref"].is_string(),
        "Multi-tenant operation should include $ref"
    );

    let ref_url = member["$ref"].as_str().unwrap();
    let expected_url = format!("https://acme-corp.scim.example.com/v2/Users/{}", user_id);
    assert_eq!(
        ref_url, expected_url,
        "Multi-tenant $ref should use subdomain strategy"
    );
}

/// Test that operation handler list operations include $ref fields
#[tokio::test]
async fn test_operation_handler_list_includes_ref_fields() {
    let storage = InMemoryStorage::new();
    let provider = StandardResourceProvider::new(storage);

    let mut server = ScimServerBuilder::new(provider)
        .with_base_url("https://list.test.com")
        .with_tenant_strategy(TenantStrategy::SingleTenant)
        .build()
        .unwrap();

    // Register resource types
    let user_schema = server
        .get_schema_by_id("urn:ietf:params:scim:schemas:core:2.0:User")
        .unwrap()
        .clone();
    let user_handler = create_user_resource_handler(user_schema);
    server
        .register_resource_type(
            "User",
            user_handler,
            vec![ScimOperation::Create, ScimOperation::List],
        )
        .unwrap();

    let group_schema = server
        .get_schema_by_id("urn:ietf:params:scim:schemas:core:2.0:Group")
        .unwrap()
        .clone();
    let group_handler = create_group_resource_handler(group_schema);
    server
        .register_resource_type(
            "Group",
            group_handler,
            vec![ScimOperation::Create, ScimOperation::List],
        )
        .unwrap();

    let handler = ScimOperationHandler::new(server);

    // Create test data
    let user_request = ScimOperationRequest::create(
        "User",
        json!({
            "userName": "listuser@test.com",
            "name": { "givenName": "List", "familyName": "User" }
        }),
    );
    let user_response = handler.handle_operation(user_request).await;
    let user_id = user_response.metadata.resource_id.unwrap();

    let group_request = ScimOperationRequest::create(
        "Group",
        json!({
            "displayName": "List Test Group",
            "members": [{
                "value": user_id,
                "type": "User",
                "display": "List User"
            }]
        }),
    );
    let _group_response = handler.handle_operation(group_request).await;

    // List groups - should include $ref fields
    let list_request = ScimOperationRequest::list("Group");
    let list_response = handler.handle_operation(list_request).await;

    assert!(list_response.success, "List operation should succeed");
    assert!(list_response.data.is_some(), "List should return data");

    let list_data = list_response.data.unwrap();
    let groups = list_data.as_array().unwrap();
    assert!(!groups.is_empty(), "Should have at least one group");

    let group = &groups[0];
    let members = group["members"].as_array().unwrap();
    let member = &members[0];

    // List operations should also include $ref fields
    assert!(
        member["$ref"].is_string(),
        "List operation should include $ref fields"
    );

    let ref_url = member["$ref"].as_str().unwrap();
    let expected_url = format!("https://list.test.com/v2/Users/{}", user_id);
    assert_eq!(
        ref_url, expected_url,
        "List operation $ref should be correct"
    );
}
