//! Integration tests for SCIM 2.0 $ref field compliance
//!
//! Tests that verify proper $ref fields are included in:
//! - Group.members array (references to Users/Groups)
//! - User.groups array (references to Groups)
//!
//! These tests ensure both HTTP and MCP interfaces return SCIM 2.0 compliant data.

use scim_server::ResourceProvider;
use scim_server::providers::StandardResourceProvider;
use scim_server::resource::RequestContext;
use scim_server::resource::ScimOperation;
use scim_server::resource_handlers::{create_group_resource_handler, create_user_resource_handler};
use scim_server::storage::InMemoryStorage;
use scim_server::{ScimServerBuilder, TenantStrategy};
use serde_json::{Value, json};

/// Test that Group.members array includes proper $ref fields
#[tokio::test]
async fn test_group_members_include_ref_fields() {
    let storage = InMemoryStorage::new();
    let provider = StandardResourceProvider::new(storage);
    let server = ScimServerBuilder::new(provider)
        .with_base_url("https://example.com".to_string())
        .with_tenant_strategy(TenantStrategy::SingleTenant)
        .build()
        .expect("Failed to build server");
    let context = RequestContext::with_generated_id();

    // Step 1: Create a user first
    let user_data = json!({
        "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
        "userName": "john.doe@example.com",
        "name": {
            "givenName": "John",
            "familyName": "Doe"
        },
        "emails": [{
            "value": "john.doe@example.com",
            "type": "work",
            "primary": true
        }]
    });

    let created_user = server
        .provider()
        .create_resource("User", user_data, &context)
        .await
        .expect("Failed to create user");

    let user_id = created_user.resource().get_id().unwrap();

    // Step 2: Create a group with the user as a member
    let group_data = json!({
        "schemas": ["urn:ietf:params:scim:schemas:core:2.0:Group"],
        "displayName": "Engineering Team",
        "members": [{
            "value": user_id,
            "$ref": format!("https://example.com/v2/Users/{}", user_id),
            "type": "User",
            "display": "John Doe"
        }]
    });

    let created_group = server
        .provider()
        .create_resource("Group", group_data, &context)
        .await
        .expect("Failed to create group");

    // Step 3: Verify the created group has proper $ref field in members
    let group_json = server
        .serialize_resource_with_refs(created_group.resource(), context.tenant_id())
        .expect("Failed to serialize group with refs");
    let members = group_json["members"]
        .as_array()
        .expect("Group should have members array");

    assert_eq!(members.len(), 1, "Group should have exactly one member");

    let member = &members[0];
    assert_eq!(member["value"], user_id, "Member value should be user ID");
    assert_eq!(member["type"], "User", "Member type should be User");
    assert_eq!(
        member["display"], "John Doe",
        "Member display should be set"
    );

    // CRITICAL: This should pass but currently fails - $ref field missing
    assert!(member["$ref"].is_string(), "Member must have $ref field");
    assert_eq!(
        member["$ref"].as_str().unwrap(),
        format!("https://example.com/v2/Users/{}", user_id),
        "$ref should contain proper URI to user resource"
    );

    // Step 4: Get the group by ID and verify $ref persists
    let retrieved_group = server
        .provider()
        .get_resource(
            "Group",
            created_group.resource().get_id().unwrap(),
            &context,
        )
        .await
        .expect("Failed to retrieve group");

    let retrieved_group_json = server
        .serialize_resource_with_refs(retrieved_group.unwrap().resource(), context.tenant_id())
        .expect("Failed to serialize retrieved group with refs");
    let retrieved_members = retrieved_group_json["members"]
        .as_array()
        .expect("Retrieved group should have members array");

    let retrieved_member = &retrieved_members[0];
    assert!(
        retrieved_member["$ref"].is_string(),
        "Retrieved member must have $ref field"
    );
    assert_eq!(
        retrieved_member["$ref"].as_str().unwrap(),
        format!("https://example.com/v2/Users/{}", user_id),
        "Retrieved $ref should contain proper URI to user resource"
    );
}

/// Test that User.groups array includes proper $ref fields when client maintains referential integrity
#[tokio::test]
async fn test_user_groups_include_ref_fields() {
    let storage = InMemoryStorage::new();
    let provider = StandardResourceProvider::new(storage);
    let server = ScimServerBuilder::new(provider)
        .with_base_url("https://example.com".to_string())
        .with_tenant_strategy(TenantStrategy::SingleTenant)
        .build()
        .expect("Failed to build server");
    let context = RequestContext::with_generated_id();

    // Step 1: Create a group first
    let group_data = json!({
        "schemas": ["urn:ietf:params:scim:schemas:core:2.0:Group"],
        "displayName": "Marketing Team"
    });

    let created_group = server
        .provider()
        .create_resource("Group", group_data, &context)
        .await
        .expect("Failed to create group");

    let group_id = created_group.resource().get_id().unwrap();

    // Step 2: Create a user with groups array (client maintains referential integrity)
    let user_data = json!({
        "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
        "userName": "jane.smith@example.com",
        "name": {
            "givenName": "Jane",
            "familyName": "Smith"
        },
        "groups": [{
            "value": group_id,
            "type": "direct",
            "display": "Marketing Team"
        }]
    });

    let created_user = server
        .provider()
        .create_resource("User", user_data, &context)
        .await
        .expect("Failed to create user");

    // Step 3: Verify the user has groups array with proper $ref fields
    let user_json = server
        .serialize_resource_with_refs(created_user.resource(), context.tenant_id())
        .expect("Failed to serialize user with refs");

    assert!(
        user_json["groups"].is_array(),
        "User must have groups array"
    );

    let groups = user_json["groups"]
        .as_array()
        .expect("User should have groups array");

    assert_eq!(
        groups.len(),
        1,
        "User should be member of exactly one group"
    );

    let group_ref = &groups[0];
    assert_eq!(
        group_ref["value"], group_id,
        "Group value should be group ID"
    );
    assert!(
        group_ref["$ref"].is_string(),
        "Group reference must have $ref field"
    );
    assert_eq!(
        group_ref["$ref"].as_str().unwrap(),
        format!("https://example.com/v2/Groups/{}", group_id),
        "$ref should contain proper URI to group resource"
    );
}

/// Test bidirectional references are maintained consistently
/// Test bidirectional Group <-> User membership references when client maintains both sides
#[tokio::test]
async fn test_bidirectional_membership_references() {
    let storage = InMemoryStorage::new();
    let provider = StandardResourceProvider::new(storage);
    let server = ScimServerBuilder::new(provider)
        .with_base_url("https://example.com".to_string())
        .with_tenant_strategy(TenantStrategy::SingleTenant)
        .build()
        .expect("Failed to build server");
    let context = RequestContext::with_generated_id();

    // Step 1: Create user first
    let user_data = json!({
        "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
        "userName": "bidirectional@example.com",
        "name": {"givenName": "Test", "familyName": "User"}
    });

    let user = server
        .provider()
        .create_resource("User", user_data, &context)
        .await
        .expect("Failed to create user");
    let user_id = user.resource().get_id().unwrap().to_string();

    // Step 2: Create group with user as member
    let group_data = json!({
        "schemas": ["urn:ietf:params:scim:schemas:core:2.0:Group"],
        "displayName": "Test Group",
        "members": [{
            "value": user_id,
            "$ref": format!("https://example.com/v2/Users/{}", user_id),
            "type": "User",
            "display": "Test User"
        }]
    });

    let group = server
        .provider()
        .create_resource("Group", group_data, &context)
        .await
        .expect("Failed to create group");
    let group_id = group.resource().get_id().unwrap().to_string();

    // Step 3: Client updates user to include group reference (maintaining referential integrity)
    let user_update_data = json!({
        "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
        "userName": "bidirectional@example.com",
        "name": {"givenName": "Test", "familyName": "User"},
        "groups": [{
            "value": group_id,
            "type": "direct",
            "display": "Test Group"
        }]
    });

    let updated_user = server
        .provider()
        .update_resource("User", &user_id, user_update_data, None, &context)
        .await
        .expect("Failed to update user with group reference");

    // Step 4: Verify Group -> User reference has $ref
    let group_json = server
        .serialize_resource_with_refs(group.resource(), context.tenant_id())
        .expect("Failed to serialize group with refs");
    let group_members = group_json["members"].as_array().unwrap();
    assert!(
        group_members[0]["$ref"].is_string(),
        "Group member must have $ref"
    );

    // Step 5: Verify User -> Group reference has $ref (client-maintained bidirectional)
    let updated_user_json = server
        .serialize_resource_with_refs(updated_user.resource(), context.tenant_id())
        .expect("Failed to serialize updated user with refs");
    assert!(
        updated_user_json["groups"].is_array(),
        "User must have groups array"
    );
    let user_groups = updated_user_json["groups"].as_array().unwrap();
    assert_eq!(user_groups.len(), 1, "User should be in exactly one group");
    assert!(
        user_groups[0]["$ref"].is_string(),
        "User group reference must have $ref"
    );
    assert_eq!(
        user_groups[0]["$ref"].as_str().unwrap(),
        format!("https://example.com/v2/Groups/{}", group_id),
        "User group $ref should point to correct group URI"
    );
}

/// Test that update operations preserve $ref fields
#[tokio::test]
async fn test_update_operations_preserve_ref_fields() {
    let storage = InMemoryStorage::new();
    let provider = StandardResourceProvider::new(storage);
    let server = ScimServerBuilder::new(provider)
        .with_base_url("https://example.com".to_string())
        .with_tenant_strategy(TenantStrategy::SingleTenant)
        .build()
        .expect("Failed to build server");
    let context = RequestContext::with_generated_id();

    // Create user
    let user_data = json!({
        "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
        "userName": "update.test@example.com",
        "name": {"givenName": "Update", "familyName": "Test"}
    });

    let user = server
        .provider()
        .create_resource("User", user_data, &context)
        .await
        .expect("Failed to create user");
    let user_id = user.resource().get_id().unwrap();

    // Create group with user
    let group_data = json!({
        "schemas": ["urn:ietf:params:scim:schemas:core:2.0:Group"],
        "displayName": "Original Group",
        "members": [{
            "value": user_id,
            "$ref": format!("https://example.com/v2/Users/{}", user_id),
            "type": "User",
            "display": "Update Test"
        }]
    });

    let group = server
        .provider()
        .create_resource("Group", group_data, &context)
        .await
        .expect("Failed to create group");
    let group_id = group.resource().get_id().unwrap();
    let _group_json = server
        .serialize_resource_with_refs(group.resource(), context.tenant_id())
        .expect("Failed to serialize group with refs");
    let group_version = group.version().clone();

    // Update group displayName
    let update_data = json!({
        "schemas": ["urn:ietf:params:scim:schemas:core:2.0:Group"],
        "displayName": "Updated Group Name",
        "members": [{
            "value": user_id,
            "$ref": format!("https://example.com/v2/Users/{}", user_id),
            "type": "User",
            "display": "Update Test"
        }]
    });

    let updated_group = server
        .provider()
        .update_resource(
            "Group",
            group_id,
            update_data,
            Some(&group_version),
            &context,
        )
        .await
        .expect("Failed to update group");

    // updated_group is now a VersionedResource directly

    // Step 4: Verify $ref fields are preserved after update
    let updated_json = server
        .serialize_resource_with_refs(updated_group.resource(), context.tenant_id())
        .expect("Failed to serialize updated group with refs");
    let updated_members = updated_json["members"]
        .as_array()
        .expect("Updated group should have members array");

    assert_eq!(
        updated_members.len(),
        1,
        "Group should still have one member"
    );
    let member = &updated_members[0];

    assert!(
        member["$ref"].is_string(),
        "Updated member must preserve $ref field"
    );
    assert_eq!(
        member["$ref"].as_str().unwrap(),
        format!("https://example.com/v2/Users/{}", user_id),
        "Updated member $ref should still point to correct user URI"
    );
    assert_eq!(
        updated_json["displayName"], "Updated Group Name",
        "Display name should be updated"
    );
}

/// Test nested group membership $ref handling
#[tokio::test]
async fn test_nested_group_membership_refs() {
    let storage = InMemoryStorage::new();
    let provider = StandardResourceProvider::new(storage);
    let server = ScimServerBuilder::new(provider)
        .with_base_url("https://example.com".to_string())
        .with_tenant_strategy(TenantStrategy::SingleTenant)
        .build()
        .expect("Failed to build server");
    let context = RequestContext::with_generated_id();

    // Create parent group
    let parent_group_data = json!({
        "schemas": ["urn:ietf:params:scim:schemas:core:2.0:Group"],
        "displayName": "Parent Group"
    });

    let parent_group = server
        .provider()
        .create_resource("Group", parent_group_data, &context)
        .await
        .expect("Failed to create parent group");
    let parent_id = parent_group.resource().get_id().unwrap();

    // Create child group as member of parent group
    let child_group_data = json!({
        "schemas": ["urn:ietf:params:scim:schemas:core:2.0:Group"],
        "displayName": "Child Group",
        "members": [{
            "value": parent_id,
            "$ref": format!("https://example.com/v2/Groups/{}", parent_id),
            "type": "Group",
            "display": "Parent Group"
        }]
    });

    let child_group = server
        .provider()
        .create_resource("Group", child_group_data, &context)
        .await
        .expect("Failed to create child group");

    // Verify child group has proper $ref to parent group
    let child_group_json = server
        .serialize_resource_with_refs(child_group.resource(), context.tenant_id())
        .expect("Failed to serialize child group with refs");
    let child_members = child_group_json["members"]
        .as_array()
        .expect("Child group should have members array");

    assert_eq!(child_members.len(), 1, "Child group should have one member");
    let group_member = &child_members[0];

    assert_eq!(
        group_member["value"], parent_id,
        "Member value should be parent group ID"
    );
    assert_eq!(group_member["type"], "Group", "Member type should be Group");
    assert!(
        group_member["$ref"].is_string(),
        "Group member must have $ref field"
    );
    assert_eq!(
        group_member["$ref"].as_str().unwrap(),
        format!("https://example.com/v2/Groups/{}", parent_id),
        "$ref should contain proper URI to parent group"
    );
}

/// Test multiple members in group all have $ref fields
/// Test multiple group members all have proper $ref fields
#[tokio::test]
async fn test_multiple_group_members_all_have_refs() {
    let storage = InMemoryStorage::new();
    let provider = StandardResourceProvider::new(storage);
    let server = ScimServerBuilder::new(provider)
        .with_base_url("https://example.com".to_string())
        .with_tenant_strategy(TenantStrategy::SingleTenant)
        .build()
        .expect("Failed to build server");
    let context = RequestContext::with_generated_id();

    // Create multiple users
    let mut user_ids = Vec::new();
    for i in 1..=3 {
        let user_data = json!({
            "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
            "userName": format!("user{}@example.com", i),
            "name": {
                "givenName": format!("User{}", i),
                "familyName": "Test"
            }
        });

        let user = server
            .provider()
            .create_resource("User", user_data, &context)
            .await
            .expect("Failed to create user");
        user_ids.push(user.resource().get_id().unwrap().to_string());
    }

    // Create group with all users as members
    let members_data: Vec<Value> = user_ids
        .iter()
        .enumerate()
        .map(|(i, user_id)| {
            json!({
                "value": user_id,
                "$ref": format!("https://example.com/v2/Users/{}", user_id),
                "type": "User",
                "display": format!("User{} Test", i + 1)
            })
        })
        .collect();

    let group_data = json!({
        "schemas": ["urn:ietf:params:scim:schemas:core:2.0:Group"],
        "displayName": "Multi-Member Group",
        "members": members_data
    });

    let group = server
        .provider()
        .create_resource("Group", group_data, &context)
        .await
        .expect("Failed to create group with multiple members");

    // Step 3: Verify all members have $ref fields
    let group_json = server
        .serialize_resource_with_refs(group.resource(), context.tenant_id())
        .expect("Failed to serialize group with refs");
    let group_members = group_json["members"]
        .as_array()
        .expect("Group should have members array");

    assert_eq!(group_members.len(), 3, "Group should have 3 members");

    for (i, member) in group_members.iter().enumerate() {
        assert!(
            member["$ref"].is_string(),
            "Member {} must have $ref field",
            i
        );
        assert_eq!(
            member["type"], "User",
            "Member {} should be of type User",
            i
        );
        assert!(
            member["value"].is_string(),
            "Member {} must have value field",
            i
        );

        let expected_ref = format!(
            "https://example.com/v2/Users/{}",
            member["value"].as_str().unwrap()
        );
        assert_eq!(
            member["$ref"], expected_ref,
            "Member {} $ref should be correct",
            i
        );
    }

    // Note: We do NOT check if users have groups arrays because the SCIM server
    // does not maintain referential integrity - that's the client's responsibility
}

/// Test that $ref fields use correct base URL configuration
#[tokio::test]
async fn test_ref_fields_use_correct_base_url() {
    // Setup: Create configured SCIM server with custom base URL
    let storage = InMemoryStorage::new();
    let provider = StandardResourceProvider::new(storage);

    let mut server = ScimServerBuilder::new(provider)
        .with_base_url("https://scim.company.com")
        .with_tenant_strategy(TenantStrategy::SingleTenant)
        .build()
        .expect("Failed to build SCIM server");

    // Register resource types
    let user_schema = server
        .get_schema_by_id("urn:ietf:params:scim:schemas:core:2.0:User")
        .expect("User schema should exist")
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
            ],
        )
        .expect("Failed to register User resource type");

    let group_schema = server
        .get_schema_by_id("urn:ietf:params:scim:schemas:core:2.0:Group")
        .expect("Group schema should exist")
        .clone();
    let group_handler = create_group_resource_handler(group_schema);
    server
        .register_resource_type(
            "Group",
            group_handler,
            vec![
                ScimOperation::Create,
                ScimOperation::Read,
                ScimOperation::Update,
            ],
        )
        .expect("Failed to register Group resource type");

    let context = RequestContext::with_generated_id();

    // Step 1: Create user (without specifying $ref - server should generate it)
    let user_data = json!({
        "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
        "userName": "baseurl.test@example.com",
        "name": {"givenName": "BaseURL", "familyName": "Test"}
    });

    let user = server
        .create_resource("User", user_data, &context)
        .await
        .expect("Failed to create user");

    let user_id = user.get_id().unwrap();

    // Step 2: Create group with user member (without $ref - server should generate it)
    let group_data = json!({
        "schemas": ["urn:ietf:params:scim:schemas:core:2.0:Group"],
        "displayName": "Base URL Test Group",
        "members": [{
            "value": user_id,
            "type": "User",
            "display": "BaseURL Test"
        }]
    });

    let group_json = server
        .create_resource_with_refs("Group", group_data, &context)
        .await
        .expect("Failed to create group");

    // Step 3: Verify $ref URLs are automatically generated with correct base URL
    let members = group_json["members"].as_array().unwrap();
    let member = &members[0];

    // The server should automatically add the $ref field
    assert!(
        member["$ref"].is_string(),
        "Server should automatically generate $ref field"
    );

    let ref_url = member["$ref"].as_str().unwrap();
    let expected_url = format!("https://scim.company.com/v2/Users/{}", user_id);

    assert_eq!(ref_url, expected_url, "$ref should use configured base URL");
    assert!(
        ref_url.starts_with("https://scim.company.com"),
        "$ref should use configured domain"
    );
    assert!(
        ref_url.contains("/v2/Users/"),
        "$ref should contain correct SCIM path"
    );
    assert!(ref_url.ends_with(&user_id), "$ref should end with user ID");
}

/// Test $ref fields with subdomain-based multi-tenant configuration
#[tokio::test]
async fn test_ref_fields_subdomain_multitenant() {
    let storage = InMemoryStorage::new();
    let provider = StandardResourceProvider::new(storage);

    let mut server = ScimServerBuilder::new(provider)
        .with_base_url("https://scim.company.com")
        .with_tenant_strategy(TenantStrategy::Subdomain)
        .build()
        .expect("Failed to build SCIM server");

    // Register resource types
    let user_schema = server
        .get_schema_by_id("urn:ietf:params:scim:schemas:core:2.0:User")
        .expect("User schema should exist")
        .clone();
    let user_handler = create_user_resource_handler(user_schema);
    server
        .register_resource_type(
            "User",
            user_handler,
            vec![ScimOperation::Create, ScimOperation::Read],
        )
        .expect("Failed to register User resource type");

    let group_schema = server
        .get_schema_by_id("urn:ietf:params:scim:schemas:core:2.0:Group")
        .expect("Group schema should exist")
        .clone();
    let group_handler = create_group_resource_handler(group_schema);
    server
        .register_resource_type(
            "Group",
            group_handler,
            vec![ScimOperation::Create, ScimOperation::Read],
        )
        .expect("Failed to register Group resource type");

    // Create tenant context
    use scim_server::TenantContext;
    let tenant_context = TenantContext::new("acme".to_string(), "client-123".to_string());
    let context = RequestContext::with_tenant_generated_id(tenant_context);

    // Create user
    let user_data = json!({
        "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
        "userName": "tenant.user@acme.com",
        "name": {"givenName": "Tenant", "familyName": "User"}
    });

    let user = server
        .create_resource("User", user_data, &context)
        .await
        .expect("Failed to create user");

    let user_id = user.get_id().unwrap();

    // Create group with user member
    let group_data = json!({
        "schemas": ["urn:ietf:params:scim:schemas:core:2.0:Group"],
        "displayName": "Tenant Test Group",
        "members": [{
            "value": user_id,
            "type": "User",
            "display": "Tenant User"
        }]
    });

    let group_json = server
        .create_resource_with_refs("Group", group_data, &context)
        .await
        .expect("Failed to create group");

    // Verify $ref URL uses subdomain strategy
    let members = group_json["members"].as_array().unwrap();
    let member = &members[0];

    assert!(
        member["$ref"].is_string(),
        "Server should generate $ref field for tenant"
    );

    let ref_url = member["$ref"].as_str().unwrap();
    let expected_url = "https://acme.scim.company.com/v2/Users/".to_string() + &user_id;

    assert_eq!(
        ref_url, expected_url,
        "$ref should use subdomain tenant strategy"
    );
    assert!(
        ref_url.starts_with("https://acme.scim.company.com"),
        "$ref should use tenant subdomain"
    );
}

/// Test $ref fields with path-based multi-tenant configuration
#[tokio::test]
async fn test_ref_fields_path_based_multitenant() {
    let storage = InMemoryStorage::new();
    let provider = StandardResourceProvider::new(storage);

    let mut server = ScimServerBuilder::new(provider)
        .with_base_url("https://api.company.com")
        .with_tenant_strategy(TenantStrategy::PathBased)
        .build()
        .expect("Failed to build SCIM server");

    // Register resource types
    let user_schema = server
        .get_schema_by_id("urn:ietf:params:scim:schemas:core:2.0:User")
        .expect("User schema should exist")
        .clone();
    let user_handler = create_user_resource_handler(user_schema);
    server
        .register_resource_type(
            "User",
            user_handler,
            vec![ScimOperation::Create, ScimOperation::Read],
        )
        .expect("Failed to register User resource type");

    let group_schema = server
        .get_schema_by_id("urn:ietf:params:scim:schemas:core:2.0:Group")
        .expect("Group schema should exist")
        .clone();
    let group_handler = create_group_resource_handler(group_schema);
    server
        .register_resource_type(
            "Group",
            group_handler,
            vec![ScimOperation::Create, ScimOperation::Read],
        )
        .expect("Failed to register Group resource type");

    // Create tenant context
    use scim_server::TenantContext;
    let tenant_context = TenantContext::new("enterprise".to_string(), "ent-client-456".to_string());
    let context = RequestContext::with_tenant_generated_id(tenant_context);

    // Create user
    let user_data = json!({
        "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
        "userName": "enterprise.user@company.com",
        "name": {"givenName": "Enterprise", "familyName": "User"}
    });

    let user = server
        .create_resource("User", user_data, &context)
        .await
        .expect("Failed to create user");

    let user_id = user.get_id().unwrap();

    // Create group with user member
    let group_data = json!({
        "schemas": ["urn:ietf:params:scim:schemas:core:2.0:Group"],
        "displayName": "Enterprise Group",
        "members": [{
            "value": user_id,
            "type": "User",
            "display": "Enterprise User"
        }]
    });

    let group_json = server
        .create_resource_with_refs("Group", group_data, &context)
        .await
        .expect("Failed to create group");

    // Verify $ref URL uses path-based strategy
    let members = group_json["members"].as_array().unwrap();
    let member = &members[0];

    assert!(
        member["$ref"].is_string(),
        "Server should generate $ref field for path-based tenant"
    );

    let ref_url = member["$ref"].as_str().unwrap();
    let expected_url = format!("https://api.company.com/enterprise/v2/Users/{}", user_id);

    assert_eq!(
        ref_url, expected_url,
        "$ref should use path-based tenant strategy"
    );
    assert!(
        ref_url.contains("/enterprise/v2/"),
        "$ref should include tenant in path"
    );
}

/// Test error handling when tenant is required but missing
#[tokio::test]
async fn test_missing_tenant_error() {
    let storage = InMemoryStorage::new();
    let provider = StandardResourceProvider::new(storage);

    let mut server = ScimServerBuilder::new(provider)
        .with_base_url("https://scim.company.com")
        .with_tenant_strategy(TenantStrategy::PathBased) // Requires tenant
        .build()
        .expect("Failed to build SCIM server");

    // Register User and Group resource types
    let user_schema = server
        .get_schema_by_id("urn:ietf:params:scim:schemas:core:2.0:User")
        .expect("User schema should exist")
        .clone();
    let user_handler = create_user_resource_handler(user_schema);
    server
        .register_resource_type("User", user_handler, vec![ScimOperation::Create])
        .expect("Failed to register User resource type");

    let group_schema = server
        .get_schema_by_id("urn:ietf:params:scim:schemas:core:2.0:Group")
        .expect("Group schema should exist")
        .clone();
    let group_handler = create_group_resource_handler(group_schema);
    server
        .register_resource_type("Group", group_handler, vec![ScimOperation::Create])
        .expect("Failed to register Group resource type");

    // Create context WITHOUT tenant (should cause error during $ref generation)
    let context = RequestContext::with_generated_id();

    // Create a group with members to trigger $ref generation
    let group_data = json!({
        "schemas": ["urn:ietf:params:scim:schemas:core:2.0:Group"],
        "displayName": "No Tenant Group",
        "members": [{
            "value": "user-123",
            "type": "User",
            "display": "Test User"
        }]
    });

    // This should fail during serialization because tenant is required for path-based strategy
    let result = server
        .create_resource_with_refs("Group", group_data, &context)
        .await;

    assert!(
        result.is_err(),
        "Should fail when tenant required but missing"
    );
    let error_msg = result.unwrap_err().to_string();
    assert!(
        error_msg.contains("Tenant ID required"),
        "Error should mention missing tenant ID"
    );
}
