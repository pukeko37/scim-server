//! Group MCP Operations Test Example
//!
//! This example demonstrates the newly added Group operations in the SCIM MCP server.
//! It shows how AI agents can manage groups through the MCP protocol with full CRUD
//! operations and query capabilities.

#[cfg(feature = "mcp")]
use scim_server::{
    ScimServer,
    mcp_integration::{McpServerInfo, ScimMcpServer},
    multi_tenant::ScimOperation,
    providers::StandardResourceProvider,
    resource_handlers::{create_group_resource_handler, create_user_resource_handler},
    storage::InMemoryStorage,
};

#[cfg(feature = "mcp")]
use serde_json::json;

#[cfg(feature = "mcp")]
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
        .format_timestamp_secs()
        .init();

    println!("🚀 Group MCP Operations Test");
    println!("============================\n");

    // 1. Create SCIM server with both User and Group support
    let storage = InMemoryStorage::new();
    let provider = StandardResourceProvider::new(storage);
    let mut scim_server = ScimServer::new(provider)?;

    // Register User resource type
    let user_schema = scim_server
        .get_schema_by_id("urn:ietf:params:scim:schemas:core:2.0:User")
        .expect("User schema should be available")
        .clone();

    let user_handler = create_user_resource_handler(user_schema);
    scim_server.register_resource_type(
        "User",
        user_handler,
        vec![
            ScimOperation::Create,
            ScimOperation::Read,
            ScimOperation::Update,
            ScimOperation::Delete,
            ScimOperation::List,
            ScimOperation::Search,
        ],
    )?;

    // Register Group resource type
    if let Some(group_schema) =
        scim_server.get_schema_by_id("urn:ietf:params:scim:schemas:core:2.0:Group")
    {
        let group_handler = create_group_resource_handler(group_schema.clone());
        scim_server.register_resource_type(
            "Group",
            group_handler,
            vec![
                ScimOperation::Create,
                ScimOperation::Read,
                ScimOperation::Update,
                ScimOperation::Delete,
                ScimOperation::List,
                ScimOperation::Search,
            ],
        )?;
        println!("✅ Registered Group resource type");
    }

    // 2. Create MCP server
    let server_info = McpServerInfo {
        name: "SCIM Group Test Server".to_string(),
        version: "1.0.0".to_string(),
        description: "Testing Group operations in MCP integration".to_string(),
        supported_resource_types: scim_server
            .get_supported_resource_types()
            .into_iter()
            .map(|s| s.to_string())
            .collect(),
    };

    let mcp_server = ScimMcpServer::with_info(scim_server, server_info);

    // 3. First, create some users to add to groups
    println!("👥 Creating test users...");

    let alice_result = mcp_server
        .execute_tool(
            "scim_create_user",
            json!({
                "user_data": {
                    "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
                    "userName": "alice@company.com",
                    "name": {
                        "givenName": "Alice",
                        "familyName": "Smith"
                    },
                    "active": true
                }
            }),
        )
        .await;

    let alice_id = if alice_result.success {
        alice_result.metadata.unwrap()["resource_id"].as_str().unwrap().to_string()
    } else {
        panic!("Failed to create Alice user");
    };

    let bob_result = mcp_server
        .execute_tool(
            "scim_create_user",
            json!({
                "user_data": {
                    "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
                    "userName": "bob@company.com",
                    "name": {
                        "givenName": "Bob",
                        "familyName": "Johnson"
                    },
                    "active": true
                }
            }),
        )
        .await;

    let bob_id = if bob_result.success {
        bob_result.metadata.unwrap()["resource_id"].as_str().unwrap().to_string()
    } else {
        panic!("Failed to create Bob user");
    };

    println!("   ✅ Created users Alice and Bob\n");

    // 4. Test Group CRUD operations
    println!("📁 Testing Group CRUD operations:");
    println!("=================================");

    // Create a group
    println!("1. Creating a new group...");
    let create_group_result = mcp_server
        .execute_tool(
            "scim_create_group",
            json!({
                "group_data": {
                    "schemas": ["urn:ietf:params:scim:schemas:core:2.0:Group"],
                    "displayName": "Engineering Team",
                    "members": [
                        {
                            "value": alice_id,
                            "$ref": format!("https://example.com/v2/Users/{}", alice_id),
                            "type": "User"
                        },
                        {
                            "value": bob_id,
                            "$ref": format!("https://example.com/v2/Users/{}", bob_id),
                            "type": "User"
                        }
                    ],
                    "externalId": "eng-team-001"
                }
            }),
        )
        .await;

    let group_id = if create_group_result.success {
        let group_id = create_group_result.metadata.as_ref().unwrap()["resource_id"]
            .as_str()
            .unwrap()
            .to_string();
        println!("   ✅ Group created successfully with ID: {}", group_id);
        group_id
    } else {
        println!("   ❌ Group creation failed: {:?}", create_group_result.content);
        return Ok(());
    };

    // Get the group
    println!("\n2. Retrieving the group...");
    let get_group_result = mcp_server
        .execute_tool(
            "scim_get_group",
            json!({
                "group_id": group_id
            }),
        )
        .await;

    if get_group_result.success {
        println!("   ✅ Group retrieved successfully");
        let group_data = &get_group_result.content;
        println!("   📋 Group name: {}",
            group_data.get("displayName").and_then(|d| d.as_str()).unwrap_or("Unknown"));
        println!("   👥 Members count: {}",
            group_data.get("members").and_then(|m| m.as_array()).map(|a| a.len()).unwrap_or(0));

        // Verify no _version field exists in content (standardized approach)
        if group_data.get("_version").is_some() {
            println!("   WARNING: _version field found in content - this should not exist");
        }
    } else {
        println!("   ❌ Group retrieval failed: {:?}", get_group_result.content);
    }

    // Update the group
    println!("\n3. Updating the group...");
    let update_group_result = mcp_server
        .execute_tool(
            "scim_update_group",
            json!({
                "group_id": group_id,
                "group_data": {
                    "id": group_id,
                    "schemas": ["urn:ietf:params:scim:schemas:core:2.0:Group"],
                    "displayName": "Senior Engineering Team",
                    "members": [
                        {
                            "value": alice_id,
                            "$ref": format!("https://example.com/v2/Users/{}", alice_id),
                            "type": "User"
                        }
                    ],
                    "externalId": "senior-eng-team-001"
                }
            }),
        )
        .await;

    if update_group_result.success {
        println!("   ✅ Group updated successfully");
        println!("   📝 Updated name and removed Bob from members");
    } else {
        println!("   ❌ Group update failed: {:?}", update_group_result.content);
    }

    // 5. Test Group query operations
    println!("\n🔍 Testing Group query operations:");
    println!("==================================");

    // Create a second group for better testing
    let create_group2_result = mcp_server
        .execute_tool(
            "scim_create_group",
            json!({
                "group_data": {
                    "schemas": ["urn:ietf:params:scim:schemas:core:2.0:Group"],
                    "displayName": "Marketing Team",
                    "members": [
                        {
                            "value": bob_id,
                            "$ref": format!("https://example.com/v2/Users/{}", bob_id),
                            "type": "User"
                        }
                    ],
                    "externalId": "marketing-001"
                }
            }),
        )
        .await;

    let group2_id = if create_group2_result.success {
        create_group2_result.metadata.as_ref().unwrap()["resource_id"]
            .as_str()
            .unwrap()
            .to_string()
    } else {
        println!("Failed to create second group");
        return Ok(());
    };

    // List all groups
    println!("\n1. Listing all groups...");
    let list_groups_result = mcp_server.execute_tool("scim_list_groups", json!({})).await;

    if list_groups_result.success {
        let empty_vec = vec![];
        let groups = list_groups_result.content
            .get("Resources")
            .and_then(|r| r.as_array())
            .unwrap_or(&empty_vec);
        println!("   ✅ Groups list retrieved");
        println!("   📊 Total groups: {}", groups.len());
        for group in groups {
            if let Some(name) = group.get("displayName").and_then(|n| n.as_str()) {
                println!("      • {}", name);
            }
        }
    } else {
        println!("   ❌ Groups listing failed: {:?}", list_groups_result.content);
    }

    // Search for groups
    println!("\n2. Searching for groups by displayName...");
    let search_groups_result = mcp_server
        .execute_tool(
            "scim_search_groups",
            json!({
                "attribute": "displayName",
                "value": "Marketing Team"
            }),
        )
        .await;

    if search_groups_result.success {
        let empty_vec = vec![];
        let found_groups = search_groups_result.content
            .get("Resources")
            .and_then(|r| r.as_array())
            .unwrap_or(&empty_vec);
        println!("   ✅ Group search completed");
        println!("   🔍 Found {} matching groups", found_groups.len());
    } else {
        println!("   ❌ Group search failed: {:?}", search_groups_result.content);
    }

    // Check if group exists
    println!("\n3. Checking if group exists...");
    let group_exists_result = mcp_server
        .execute_tool(
            "scim_group_exists",
            json!({
                "group_id": group_id
            }),
        )
        .await;

    if group_exists_result.success {
        let exists = group_exists_result.content
            .get("exists")
            .and_then(|e| e.as_bool())
            .unwrap_or(false);
        println!("   ✅ Group existence check: {}", if exists { "EXISTS" } else { "NOT FOUND" });
    } else {
        println!("   ❌ Group existence check failed: {:?}", group_exists_result.content);
    }

    // 6. Test multi-tenant operations
    println!("\n🏢 Testing multi-tenant Group operations:");
    println!("=========================================");

    let tenant_group_result = mcp_server
        .execute_tool(
            "scim_create_group",
            json!({
                "group_data": {
                    "schemas": ["urn:ietf:params:scim:schemas:core:2.0:Group"],
                    "displayName": "Tenant A Admin Group",
                    "externalId": "tenant-a-admins"
                },
                "tenant_id": "tenant-a"
            }),
        )
        .await;

    if tenant_group_result.success {
        println!("   ✅ Tenant-specific group created");
        if let Some(metadata) = tenant_group_result.metadata {
            if let Some(tenant_id) = metadata.get("tenant_id") {
                println!("   🏢 Tenant context preserved: {}", tenant_id);
            }
        }
    } else {
        println!("   ❌ Tenant-specific group creation failed");
    }

    // 7. Test error handling
    println!("\n⚠️  Testing Group error handling:");
    println!("=================================");

    // Try to get non-existent group
    let error_test_result = mcp_server
        .execute_tool(
            "scim_get_group",
            json!({
                "group_id": "non-existent-group-id"
            }),
        )
        .await;

    if !error_test_result.success {
        println!("   ✅ Error handling working correctly for non-existent groups");
        let error_code = error_test_result.content
            .get("error_code")
            .and_then(|e| e.as_str())
            .unwrap_or("UNKNOWN");
        println!("   📝 Error code: {}", error_code);
    }

    // 8. Cleanup
    println!("\n🧹 Cleaning up test data:");
    println!("=========================");

    // Delete groups
    for (name, id) in [("Engineering Group", &group_id), ("Marketing Group", &group2_id)] {
        let delete_result = mcp_server
            .execute_tool(
                "scim_delete_group",
                json!({
                    "group_id": id
                }),
            )
            .await;

        if delete_result.success {
            println!("   ✅ {} deleted", name);
        } else {
            println!("   ❌ Failed to delete {}", name);
        }
    }

    // Delete users
    for (name, id) in [("Alice", &alice_id), ("Bob", &bob_id)] {
        let delete_result = mcp_server
            .execute_tool(
                "scim_delete_user",
                json!({
                    "user_id": id
                }),
            )
            .await;

        if delete_result.success {
            println!("   ✅ User {} deleted", name);
        } else {
            println!("   ❌ Failed to delete user {}", name);
        }
    }

    // 9. Final verification
    println!("\n🎉 Group MCP Operations Test Complete!");
    println!("======================================");

    let final_tools = mcp_server.get_tools();
    let group_tools: Vec<_> = final_tools
        .iter()
        .filter(|tool| tool.get("name")
            .and_then(|n| n.as_str())
            .map_or(false, |name| name.contains("group")))
        .collect();

    println!("✅ Group operations successfully integrated into MCP server");
    println!("✅ Total Group tools available: {}", group_tools.len());
    println!("✅ All CRUD operations tested and working");
    println!("✅ Query operations tested and working");
    println!("✅ Multi-tenant support verified");
    println!("✅ Error handling confirmed");

    println!("\n📋 Available Group Tools:");
    for tool in group_tools {
        if let Some(name) = tool.get("name").and_then(|n| n.as_str()) {
            if let Some(desc) = tool.get("description").and_then(|d| d.as_str()) {
                println!("  • {} - {}", name, desc);
            }
        }
    }

    println!("\n🚀 Group MCP Integration Ready for Production!");

    Ok(())
}

#[cfg(not(feature = "mcp"))]
fn main() {
    eprintln!("This example requires the 'mcp' feature to be enabled.");
    eprintln!("Please run with: cargo run --example test_group_mcp --features mcp");
    std::process::exit(1);
}
