//! MCP ETag Concurrency Control Example
//!
//! This example demonstrates how AI agents can use ETag versioning through the
//! Model Context Protocol (MCP) integration to perform safe concurrent operations
//! on SCIM resources. It shows how ETags prevent lost updates and enable proper
//! conflict resolution in AI-driven identity management scenarios.

#[cfg(feature = "mcp")]
use scim_server::{
    ScimServer, mcp_integration::ScimMcpServer, multi_tenant::ScimOperation,
    providers::StandardResourceProvider, resource_handlers::create_user_resource_handler,
    storage::InMemoryStorage,
};
#[cfg(feature = "mcp")]
use serde_json::json;

#[cfg(feature = "mcp")]
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    env_logger::init();

    println!("🤖 MCP ETag Concurrency Control Example");
    println!("=======================================\n");

    // 1. Setup SCIM server with MCP integration
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
        ],
    )?;

    let mcp_server = ScimMcpServer::new(scim_server);

    println!("✅ MCP server initialized with ETag support\n");

    // === AVAILABLE MCP TOOLS ===
    println!("🔧 AVAILABLE MCP TOOLS WITH ETAG SUPPORT");
    println!("=========================================");

    let tools = mcp_server.get_tools();
    for tool in &tools {
        let name = tool["name"].as_str().unwrap();
        let description = tool["description"].as_str().unwrap();
        println!("• {}: {}", name, description);

        // Show ETag parameters for update/delete tools
        if name.contains("update") || name.contains("delete") {
            if let Some(expected_version_prop) =
                tool["input_schema"]["properties"]["expected_version"].as_object()
            {
                println!(
                    "  └─ ETag parameter: {}",
                    expected_version_prop["description"].as_str().unwrap()
                );
            }
        }
    }

    println!();

    // === AI AGENT USER CREATION ===
    println!("👤 AI AGENT: CREATING NEW USER");
    println!("==============================");

    let create_result = mcp_server
        .execute_tool(
            "scim_create_user",
            json!({
                "user_data": {
                    "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
                    "userName": "ai.assistant@company.com",
                    "name": {
                        "familyName": "Assistant",
                        "givenName": "AI",
                        "formatted": "AI Assistant"
                    },
                    "emails": [
                        {
                            "value": "ai.assistant@company.com",
                            "type": "work",
                            "primary": true
                        }
                    ],
                    "active": true
                }
            }),
        )
        .await;

    let (user_id, initial_etag) = if create_result.success {
        let user_id = create_result.metadata.as_ref().unwrap()["resource_id"]
            .as_str()
            .unwrap()
            .to_string();
        let etag = create_result.metadata.as_ref().unwrap()["etag"]
            .as_str()
            .unwrap()
            .to_string();

        println!("✅ AI Agent successfully created user:");
        println!("   User ID: {}", user_id);
        println!("   Initial ETag: {}", etag);
        println!("   Response includes ETag for subsequent operations");

        (user_id, etag)
    } else {
        panic!("Failed to create user: {:?}", create_result.content);
    };

    println!();

    // === AI AGENT RETRIEVAL WITH VERSION ===
    println!("🔍 AI AGENT: RETRIEVING USER WITH VERSION");
    println!("=========================================");

    let get_result = mcp_server
        .execute_tool(
            "scim_get_user",
            json!({
                "user_id": user_id
            }),
        )
        .await;

    if get_result.success {
        let current_etag = get_result.metadata.as_ref().unwrap()["etag"]
            .as_str()
            .unwrap();
        println!("✅ AI Agent retrieved user successfully:");
        println!("   Current ETag: {}", current_etag);
        println!("   User data includes _etag field for easy access");

        // Show that ETag is also embedded in content for easy AI access
        if let Some(embedded_etag) = get_result.content["_etag"].as_str() {
            println!("   Embedded ETag in content: {}", embedded_etag);
        }
    }

    println!();

    // === AI AGENT CONDITIONAL UPDATE (SUCCESS) ===
    println!("✅ AI AGENT: CONDITIONAL UPDATE (SUCCESS)");
    println!("==========================================");

    let conditional_update_result = mcp_server
        .execute_tool(
            "scim_update_user",
            json!({
                "user_id": user_id,
                "user_data": {
                    "id": user_id,
                    "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
                    "userName": "ai.assistant@company.com",
                    "name": {
                        "familyName": "Assistant",
                        "givenName": "AI",
                        "formatted": "AI Assistant (Updated)"
                    },
                    "emails": [
                        {
                            "value": "ai.assistant@newcompany.com",
                            "type": "work",
                            "primary": true
                        }
                    ],
                    "active": true
                },
                "expected_version": initial_etag  // Using ETag from creation
            }),
        )
        .await;

    let _new_etag = if conditional_update_result.success {
        let new_etag = conditional_update_result.metadata.as_ref().unwrap()["etag"]
            .as_str()
            .unwrap()
            .to_string();
        println!("✅ AI Agent conditional update succeeded:");
        println!("   Old ETag: {}", initial_etag);
        println!("   New ETag: {}", new_etag);
        println!("   Version-safe update completed");
        new_etag
    } else {
        panic!(
            "Conditional update should have succeeded: {:?}",
            conditional_update_result.content
        );
    };

    println!();

    // === AI AGENT CONCURRENT MODIFICATION SIMULATION ===
    println!("⚠️  AI AGENT: SIMULATING VERSION CONFLICT");
    println!("=========================================");

    // Simulate another AI agent trying to update with stale ETag
    let conflict_update_result = mcp_server
        .execute_tool(
            "scim_update_user",
            json!({
                "user_id": user_id,
                "user_data": {
                    "id": user_id,
                    "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
                    "userName": "ai.assistant@company.com",
                    "active": false  // Different change
                },
                "expected_version": initial_etag  // Using stale ETag
            }),
        )
        .await;

    if !conflict_update_result.success {
        println!("✅ AI Agent properly detected version conflict:");
        println!(
            "   Error: {}",
            conflict_update_result.content["error"].as_str().unwrap()
        );
        println!(
            "   Error Code: {}",
            conflict_update_result.content["error_code"]
                .as_str()
                .unwrap()
        );

        let is_version_conflict = conflict_update_result.content["is_version_conflict"]
            .as_bool()
            .unwrap_or(false);
        println!("   Is Version Conflict: {}", is_version_conflict);

        if is_version_conflict {
            println!("   → AI Agent should refresh user data and retry");
        }
    } else {
        panic!("Update should have failed due to version conflict");
    }

    println!();

    // === AI AGENT CONFLICT RESOLUTION ===
    println!("🔄 AI AGENT: CONFLICT RESOLUTION STRATEGY");
    println!("=========================================");

    // Step 1: AI Agent detects conflict and refreshes data
    println!("Step 1: AI Agent refreshes user data after conflict");
    let refresh_result = mcp_server
        .execute_tool(
            "scim_get_user",
            json!({
                "user_id": user_id
            }),
        )
        .await;

    let current_etag = if refresh_result.success {
        let current_etag = refresh_result.metadata.as_ref().unwrap()["etag"]
            .as_str()
            .unwrap()
            .to_string();
        println!("✅ Refreshed user data successfully");
        println!("   Current ETag: {}", current_etag);
        current_etag
    } else {
        panic!("Failed to refresh user data");
    };

    // Step 2: AI Agent retries with current ETag
    println!("\nStep 2: AI Agent retries update with current ETag");
    let retry_update_result = mcp_server
        .execute_tool(
            "scim_update_user",
            json!({
                "user_id": user_id,
                "user_data": {
                    "id": user_id,
                    "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
                    "userName": "ai.assistant@company.com",
                    "name": {
                        "familyName": "Assistant",
                        "givenName": "AI",
                        "formatted": "AI Assistant (Conflict Resolved)"
                    },
                    "emails": [
                        {
                            "value": "ai.assistant@newcompany.com",
                            "type": "work",
                            "primary": true
                        }
                    ],
                    "active": false  // Applying the change that failed before
                },
                "expected_version": current_etag  // Using current ETag
            }),
        )
        .await;

    let final_etag = if retry_update_result.success {
        let final_etag = retry_update_result.metadata.as_ref().unwrap()["etag"]
            .as_str()
            .unwrap()
            .to_string();
        println!("✅ AI Agent retry succeeded:");
        println!("   Previous ETag: {}", current_etag);
        println!("   Final ETag: {}", final_etag);
        println!("   Conflict successfully resolved");
        final_etag
    } else {
        panic!(
            "Retry update should have succeeded: {:?}",
            retry_update_result.content
        );
    };

    println!();

    // === AI AGENT CONDITIONAL DELETE ===
    println!("🗑️  AI AGENT: CONDITIONAL DELETE");
    println!("=================================");

    // First try delete with wrong ETag
    println!("Attempting delete with stale ETag (should fail):");
    let wrong_delete_result = mcp_server
        .execute_tool(
            "scim_delete_user",
            json!({
                "user_id": user_id,
                "expected_version": initial_etag  // Stale ETag
            }),
        )
        .await;

    if !wrong_delete_result.success {
        println!("✅ AI Agent properly rejected unsafe delete:");
        println!(
            "   Error: {}",
            wrong_delete_result.content["error"].as_str().unwrap()
        );
        let is_version_conflict = wrong_delete_result.content["is_version_conflict"]
            .as_bool()
            .unwrap_or(false);
        println!("   Is Version Conflict: {}", is_version_conflict);
    }

    // Now try delete with correct ETag
    println!("\nAttempting delete with current ETag (should succeed):");
    let correct_delete_result = mcp_server
        .execute_tool(
            "scim_delete_user",
            json!({
                "user_id": user_id,
                "expected_version": final_etag  // Current ETag
            }),
        )
        .await;

    if correct_delete_result.success {
        println!("✅ AI Agent successfully deleted user:");
        println!("   Safe deletion completed with version check");
    } else {
        panic!("Delete with correct ETag should have succeeded");
    }

    println!();

    // === AI AGENT BEST PRACTICES ===
    println!("🎯 AI AGENT BEST PRACTICES FOR ETAG USAGE");
    println!("==========================================");

    println!("1. Always capture ETag from operation responses:");
    println!("   • Create operations return ETag in metadata['etag']");
    println!("   • Get operations return ETag in metadata['etag']");
    println!("   • Content also includes _etag field for convenience");

    println!("\n2. Use ETags for all update and delete operations:");
    println!("   • Include 'expected_version' parameter with ETag value");
    println!("   • ETag format: W/\"<version-string>\" (weak ETag)");

    println!("\n3. Handle version conflicts gracefully:");
    println!("   • Check 'is_version_conflict' field in error responses");
    println!("   • Refresh resource data when conflicts occur");
    println!("   • Retry operation with current ETag");

    println!("\n4. ETag format and parsing:");
    println!("   • Always use complete ETag including W/ prefix");
    println!("   • Example: W/\"abc123def456\"");
    println!("   • Do not modify or parse ETag content manually");

    println!();

    // === MULTI-TENANT AI OPERATIONS ===
    println!("🏢 AI AGENT: MULTI-TENANT OPERATIONS");
    println!("====================================");

    // Create user in specific tenant
    let tenant_create_result = mcp_server
        .execute_tool(
            "scim_create_user",
            json!({
                "user_data": {
                    "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
                    "userName": "tenant.ai@enterprise.com",
                    "active": true
                },
                "tenant_id": "enterprise-corp"
            }),
        )
        .await;

    if tenant_create_result.success {
        let tenant_user_id = tenant_create_result.metadata.as_ref().unwrap()["resource_id"]
            .as_str()
            .unwrap();
        let tenant_etag = tenant_create_result.metadata.as_ref().unwrap()["etag"]
            .as_str()
            .unwrap();

        println!("✅ AI Agent created user in tenant 'enterprise-corp':");
        println!("   User ID: {}", tenant_user_id);
        println!("   ETag: {}", tenant_etag);

        // Update in same tenant with ETag
        let tenant_update_result = mcp_server
            .execute_tool(
                "scim_update_user",
                json!({
                    "user_id": tenant_user_id,
                    "user_data": {
                        "id": tenant_user_id,
                        "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
                        "userName": "tenant.ai@enterprise.com",
                        "active": false
                    },
                    "expected_version": tenant_etag,
                    "tenant_id": "enterprise-corp"
                }),
            )
            .await;

        if tenant_update_result.success {
            println!("✅ AI Agent updated tenant user with ETag successfully");
        }
    }

    println!();

    // === CONCLUSION ===
    println!("🎉 MCP ETAG CONCURRENCY CONTROL EXAMPLE COMPLETED!");
    println!("==================================================");
    println!("✅ Demonstrated AI agent ETag usage patterns");
    println!("✅ Showed version conflict detection and resolution");
    println!("✅ Illustrated safe conditional operations");
    println!("✅ Covered multi-tenant ETag scenarios");
    println!("✅ Provided AI agent best practices");
    println!();
    println!("🤖 AI INTEGRATION BENEFITS:");
    println!("   • Automatic ETag handling in MCP tools");
    println!("   • Clear version conflict indicators");
    println!("   • Embedded ETags in response content");
    println!("   • Structured error responses for AI decision making");
    println!("   • Multi-tenant aware versioning");
    println!("   • Zero-configuration optimistic locking");

    Ok(())
}

#[cfg(not(feature = "mcp"))]
fn main() {
    println!("This example requires the 'mcp' feature to be enabled.");
    println!("Run with: cargo run --example mcp_etag_example --features mcp");
}
