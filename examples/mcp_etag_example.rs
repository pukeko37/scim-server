//! MCP Version-Based Concurrency Control Example
//!
//! This example demonstrates how AI agents can use raw version strings through the
//! Model Context Protocol (MCP) integration to perform safe concurrent operations
//! on SCIM resources. It shows how versions prevent lost updates and enable proper
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

    println!("🤖 MCP Version-Based Concurrency Control Example");
    println!("===============================================\n");

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

    println!("✅ MCP server initialized with version support\n");

    // === AVAILABLE MCP TOOLS ===
    println!("🔧 AVAILABLE MCP TOOLS WITH VERSION SUPPORT");
    println!("==========================================");

    let tools = mcp_server.get_tools();
    for tool in &tools {
        let name = tool["name"].as_str().unwrap();
        let description = tool["description"].as_str().unwrap();
        println!("• {}: {}", name, description);

        // Show version parameters for update/delete tools
        if name.contains("update") || name.contains("delete") {
            if let Some(expected_version_prop) =
                tool["inputSchema"]["properties"]["expected_version"].as_object()
            {
                println!(
                    "  └─ Version parameter: {}",
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

    let (user_id, initial_version) = if create_result.success {
        let user_id = create_result.metadata.as_ref().unwrap()["resource_id"]
            .as_str()
            .unwrap()
            .to_string();
        let version = create_result.metadata.as_ref().unwrap()["version"]
            .as_str()
            .unwrap()
            .to_string();

        println!("✅ AI Agent successfully created user:");
        println!("   User ID: {}", user_id);
        println!("   Initial Version: {}", version);
        println!("   Response includes version for subsequent operations");

        (user_id, version)
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
        let current_version = get_result.metadata.as_ref().unwrap()["version"]
            .as_str()
            .unwrap();
        println!("✅ AI Agent retrieved user successfully:");
        println!("   Current Version: {}", current_version);
        println!("   Version is available in standard meta.version field");

        // Verify no _version field exists in content (standardized approach)
        if get_result.content.get("_version").is_some() {
            println!("   WARNING: _version field found in content - this should not exist");
        }
    }

    println!();

    // === AI AGENT CONDITIONAL UPDATE WITH VERSION ===
    println!("\n🤖 AI AGENT: CONDITIONAL UPDATE WITH VERSION");
    println!("============================================");

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
                "expected_version": initial_version  // Using ETag format version
            }),
        )
        .await;

    let _new_version = if conditional_update_result.success {
        let new_version = conditional_update_result.metadata.as_ref().unwrap()["version"]
            .as_str()
            .unwrap()
            .to_string();
        println!("✅ AI Agent conditional update succeeded:");
        println!("   Old Version: {}", initial_version);
        println!("   New Version: {}", new_version);
        println!("   Version-safe update completed");
        new_version
    } else {
        panic!(
            "Conditional update should have succeeded: {:?}",
            conditional_update_result.content
        );
    };

    println!();

    // === AI AGENT CONCURRENT MODIFICATION SIMULATION ===
    // === AI AGENT CONFLICT DETECTION ===
    println!("\n🚨 AI AGENT: SIMULATING VERSION CONFLICT");
    println!("=========================================");

    // Simulate another AI agent trying to update with stale version
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
                "expected_version": initial_version  // Using stale ETag version
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

    let current_version = if refresh_result.success {
        let current_version = refresh_result.metadata.as_ref().unwrap()["version"]
            .as_str()
            .unwrap()
            .to_string();
        println!("✅ Refreshed user data successfully");
        println!("   Current Version: {}", current_version);
        current_version
    } else {
        panic!("Failed to refresh user data");
    };

    // Step 2: AI Agent retries with current version
    println!("\nStep 2: AI Agent retries update with current version");
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
                "expected_version": current_version  // Using current ETag version
            }),
        )
        .await;

    let final_version = if retry_update_result.success {
        let final_version = retry_update_result.metadata.as_ref().unwrap()["version"]
            .as_str()
            .unwrap()
            .to_string();
        println!("✅ AI Agent retry succeeded:");
        println!("   Previous Version: {}", current_version);
        println!("   Final Version: {}", final_version);
        println!("   Conflict successfully resolved");
        final_version
    } else {
        panic!(
            "Retry update should have succeeded: {:?}",
            retry_update_result.content
        );
    };

    println!();

    // === AI AGENT CONDITIONAL DELETE ===
    println!("\n🗑️  AI AGENT: CONDITIONAL DELETE WITH VERSION");
    println!("====================================");

    // First try delete with wrong version
    println!("Attempting delete with stale version (should fail):");
    let wrong_delete_result = mcp_server
        .execute_tool(
            "scim_delete_user",
            json!({
                "user_id": user_id,
                "expected_version": initial_version  // Stale version
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

    // Now try delete with correct version
    println!("\nAttempting delete with current version (should succeed):");
    let correct_delete_result = mcp_server
        .execute_tool(
            "scim_delete_user",
            json!({
                "user_id": user_id,
                "expected_version": final_version
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
    println!("🎯 AI AGENT BEST PRACTICES FOR VERSION USAGE");
    println!("===========================================");

    println!("1. Always capture version from operation responses:");
    println!("   • Create operations return version in metadata['version']");
    println!("   • Get operations return version in metadata['version']");
    println!("   • Content also includes _version field for convenience");

    println!("\n2. Use raw versions for all update and delete operations:");
    println!("   • Include 'expected_version' parameter with raw version value");
    println!("   • Version format: Simple string like 'abc123def456'");

    println!("\n3. Handle version conflicts gracefully:");
    println!("   • Check 'error_code' for 'VERSION_MISMATCH' in error responses");
    println!("   • Refresh resource data when conflicts occur");
    println!("   • Retry operation with current version");

    println!("\n4. Version format and usage:");
    println!("   • Use raw version strings directly");
    println!("   • Example: 'abc123def456' (no HTTP formatting needed)");
    println!("   • No parsing or modification required - use as-is");

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
        let tenant_version = tenant_create_result.metadata.as_ref().unwrap()["version"]
            .as_str()
            .unwrap();

        println!("✅ AI Agent created user in tenant 'enterprise-corp':");
        println!("   User ID: {}", tenant_user_id);
        println!("   Version: {}", tenant_version);

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
                    "expected_version": tenant_version,
                    "tenant_id": "enterprise-corp"
                }),
            )
            .await;

        if tenant_update_result.success {
            println!("✅ AI Agent updated tenant user with version successfully");
        }
    }

    println!();

    // === CONCLUSION ===
    println!("🎉 MCP VERSION-BASED CONCURRENCY CONTROL EXAMPLE COMPLETED!");
    println!("==========================================================");
    println!("✅ Demonstrated AI agent version usage patterns");
    println!("✅ Showed version conflict detection and resolution");
    println!("✅ Illustrated safe conditional operations");
    println!("✅ Covered multi-tenant version scenarios");
    println!("✅ Provided AI agent best practices");
    println!();
    println!("🤖 AI INTEGRATION BENEFITS:");
    println!("   • Simple raw version handling in MCP tools");
    println!("   • Clear version conflict indicators");
    println!("   • Embedded versions in response content");
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
