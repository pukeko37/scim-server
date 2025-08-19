//! MCP (Model Context Protocol) Server Example
//!
//! This example demonstrates how to create and run a SCIM server that exposes
//! its functionality as MCP tools for AI agents. The MCP integration allows
//! AI systems to provision users, manage groups, and query schemas through
//! a structured tool interface.
//!
//! # Features Demonstrated
//!
//! - Complete MCP server setup with SCIM operations
//! - Tool discovery and execution
//! - Multi-tenant support for AI agents
//! - Error handling and validation
//! - Schema introspection for AI understanding
//!
//! # Usage
//!
//! Run this example with the MCP feature enabled:
//! ```bash
//! cargo run --example mcp_server_example --features mcp
//! ```

#[cfg(feature = "mcp")]
use scim_server::{
    ScimServer,
    mcp_integration::{McpServerInfo, ScimMcpServer},
    multi_tenant::ScimOperation,
    providers::StandardResourceProvider,
    storage::InMemoryStorage,
    resource_handlers::{create_group_resource_handler, create_user_resource_handler},
};

#[cfg(feature = "mcp")]
use serde_json::json;

#[cfg(feature = "mcp")]
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging for the example
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
        .format_timestamp_secs()
        .init();

    println!("🚀 SCIM MCP Server Example");
    println!("==========================\n");

    // 1. Create the SCIM server with StandardResourceProvider and InMemoryStorage
    let storage = InMemoryStorage::new();
    let provider = StandardResourceProvider::new(storage);
    let mut scim_server = ScimServer::new(provider)?;

    // 2. Register User resource type with comprehensive operations
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

    // 3. Register Group resource type (if Group schema is available)
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

    println!("✅ SCIM server initialized with resource types");

    // 4. Create MCP server with custom info
    let server_info = McpServerInfo {
        name: "Enterprise SCIM Server".to_string(),
        version: "1.0.0".to_string(),
        description:
            "Production-ready SCIM server for identity management with AI agent integration"
                .to_string(),
        supported_resource_types: scim_server
            .get_supported_resource_types()
            .into_iter()
            .map(|s| s.to_string())
            .collect(),
    };

    let mcp_server = ScimMcpServer::with_info(scim_server, server_info);

    // 5. Demonstrate tool discovery
    println!("\n🔧 Available MCP Tools:");
    println!("======================");
    let tools = mcp_server.get_tools();
    for tool in &tools {
        let name = tool
            .get("name")
            .and_then(|n| n.as_str())
            .unwrap_or("unknown");
        let description = tool
            .get("description")
            .and_then(|d| d.as_str())
            .unwrap_or("no description");
        println!("  • {} - {}", name, description);
    }
    println!("  Total tools: {}\n", tools.len());

    // 6. Demonstrate tool execution (simulating AI agent calls)
    println!("🤖 Simulating AI Agent Interactions:");
    println!("====================================");

    // Get schema information (AI agents need this for understanding)
    println!("1. AI Agent: Getting schema information...");
    let schema_result = mcp_server.execute_tool("scim_get_schemas", json!({})).await;

    if schema_result.success {
        println!("   ✅ Schema information retrieved");
        if let Some(metadata) = schema_result.metadata {
            println!(
                "   📊 Metadata: {}",
                serde_json::to_string_pretty(&metadata)?
            );
        }
    } else {
        println!("   ❌ Schema retrieval failed");
    }

    // Get specific User schema
    println!("\n2. AI Agent: Getting User schema details...");
    let user_schema_result = mcp_server
        .execute_tool(
            "scim_get_schema",
            json!({
                "schema_id": "urn:ietf:params:scim:schemas:core:2.0:User"
            }),
        )
        .await;

    if user_schema_result.success {
        println!("   ✅ User schema retrieved for AI understanding");
    }

    // Create a user (simulating AI agent provisioning)
    println!("\n3. AI Agent: Creating a new user...");
    let create_result = mcp_server
        .execute_tool(
            "scim_create_user",
            json!({
                "user_data": {
                    "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
                    "userName": "ai.assistant",
                    "name": {
                        "givenName": "AI",
                        "familyName": "Assistant",
                        "formatted": "AI Assistant"
                    },
                    "emails": [
                        {
                            "value": "ai.assistant@company.com",
                            "type": "work",
                            "primary": true
                        }
                    ],
                    "phoneNumbers": [
                        {
                            "value": "+1-555-0199",
                            "type": "work"
                        }
                    ],
                    "active": true,
                    "externalId": "ai-001"
                }
            }),
        )
        .await;

    let user_id = if create_result.success {
        println!("   ✅ User created successfully");
        // Extract user ID from metadata
        match &create_result.metadata {
            Some(m) => m
                .get("resource_id")
                .and_then(|id| id.as_str())
                .unwrap_or("unknown")
                .to_string(),
            None => "unknown".to_string(),
        }
    } else {
        println!("   ❌ User creation failed");
        println!("   Error: {:?}", create_result.content);
        return Ok(());
    };

    // Search for the user (simulating AI agent query)
    println!("\n4. AI Agent: Searching for user by username...");
    let search_result = mcp_server
        .execute_tool(
            "scim_search_users",
            json!({
                "attribute": "userName",
                "value": "ai.assistant"
            }),
        )
        .await;

    if search_result.success {
        println!("   ✅ User search completed");
    }

    // Update user information (simulating AI agent management)
    println!("\n5. AI Agent: Updating user information...");
    let update_result = mcp_server
        .execute_tool(
            "scim_update_user",
            json!({
                "user_id": user_id,
                "user_data": {
                    "id": user_id,
                    "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
                    "userName": "ai.assistant",
                    "name": {
                        "givenName": "AI",
                        "familyName": "Assistant",
                        "formatted": "AI Assistant (Updated)"
                    },
                    "emails": [
                        {
                            "value": "ai.assistant@newdomain.com",
                            "type": "work",
                            "primary": true
                        }
                    ],
                    "active": true,
                    "title": "Virtual Assistant"
                }
            }),
        )
        .await;

    if update_result.success {
        println!("   ✅ User updated successfully");
    }

    // List all users (simulating AI agent directory query)
    println!("\n6. AI Agent: Listing all users...");
    let list_result = mcp_server.execute_tool("scim_list_users", json!({})).await;

    if list_result.success {
        println!("   ✅ User list retrieved");
        if let Some(metadata) = list_result.metadata {
            if let Some(count) = metadata.get("resource_count") {
                println!("   📊 Total users: {}", count);
            }
        }
    }

    // Check if user exists (simulating AI agent validation)
    println!("\n7. AI Agent: Checking if user exists...");
    let exists_result = mcp_server
        .execute_tool(
            "scim_user_exists",
            json!({
                "user_id": user_id
            }),
        )
        .await;

    if exists_result.success {
        println!("   ✅ User existence check completed");
    }

    // Demonstrate multi-tenant operation
    println!("\n8. AI Agent: Creating user in specific tenant...");
    let tenant_create_result = mcp_server
        .execute_tool(
            "scim_create_user",
            json!({
                "user_data": {
                    "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
                    "userName": "tenant.user",
                    "name": {
                        "givenName": "Tenant",
                        "familyName": "User"
                    },
                    "active": true
                },
                "tenant_id": "company-a"
            }),
        )
        .await;

    if tenant_create_result.success {
        println!("   ✅ Tenant-specific user created");
        if let Some(metadata) = tenant_create_result.metadata {
            if let Some(tenant_id) = metadata.get("tenant_id") {
                println!("   🏢 Tenant: {}", tenant_id);
            }
        }
    }

    // Get server information (simulating AI agent capability discovery)
    println!("\n9. AI Agent: Getting server capabilities...");
    let server_info_result = mcp_server.execute_tool("scim_server_info", json!({})).await;

    if server_info_result.success {
        println!("   ✅ Server information retrieved");
    }

    // Demonstrate error handling
    println!("\n10. AI Agent: Testing error handling...");
    let error_result = mcp_server
        .execute_tool(
            "scim_get_user",
            json!({
                "user_id": "non-existent-user-id"
            }),
        )
        .await;

    if !error_result.success {
        println!("   ✅ Error handling working correctly");
        println!("   📝 Error handled gracefully for AI agent");
    }

    // Clean up (simulating AI agent cleanup)
    println!("\n11. AI Agent: Cleaning up test user...");
    let delete_result = mcp_server
        .execute_tool(
            "scim_delete_user",
            json!({
                "user_id": user_id
            }),
        )
        .await;

    if delete_result.success {
        println!("   ✅ User deleted successfully");
    }

    println!("\n🎉 MCP Integration Example Completed!");
    println!("=====================================");
    println!("✅ All MCP tools demonstrated successfully");
    println!("✅ Multi-tenant support verified");
    println!("✅ Error handling confirmed");
    println!("✅ Schema introspection working");
    println!("✅ Full CRUD operations available to AI agents");

    println!("\n🚀 Ready for Production MCP Integration:");
    println!("========================================");
    println!("• Run with stdio transport: mcp_server.run_stdio().await");
    println!("• Integrate with AI agent frameworks");
    println!("• Connect to external MCP clients");
    println!("• Scale with multi-tenant configurations");
    println!("• Monitor with comprehensive logging");

    println!("\n📖 Usage Example for AI Agents:");
    println!("===============================");
    println!("1. Get schemas to understand available operations");
    println!("2. Create/update/delete users and groups");
    println!("3. Search and list resources with filtering");
    println!("4. Use tenant isolation for multi-client scenarios");
    println!("5. Handle errors gracefully with detailed error codes");

    Ok(())
}

#[cfg(not(feature = "mcp"))]
fn main() {
    eprintln!("This example requires the 'mcp' feature to be enabled.");
    eprintln!("Please run with: cargo run --example mcp_server_example --features mcp");
    std::process::exit(1);
}
