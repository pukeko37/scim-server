//! Simple MCP Demo
//!
//! This example demonstrates the basic MCP integration functionality
//! in a simple, easy-to-understand format.

#[cfg(feature = "mcp")]
use scim_server::{
    ScimServer,
    mcp_integration::{McpServerInfo, ScimMcpServer},
    multi_tenant::ScimOperation,
    providers::StandardResourceProvider,
    storage::InMemoryStorage,
    resource_handlers::create_user_resource_handler,
};

#[cfg(feature = "mcp")]
use serde_json::json;

#[cfg(feature = "mcp")]
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize simple logging
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    println!("🤖 Simple SCIM MCP Demo");
    println!("========================\n");

    // 1. Create a basic SCIM server
    let storage = InMemoryStorage::new();
    let provider = StandardResourceProvider::new(storage);
    let mut scim_server = ScimServer::new(provider)?;

    // 2. Register User resource type
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
        ],
    )?;

    println!("✅ SCIM server initialized");

    // 3. Create MCP server
    let server_info = McpServerInfo {
        name: "Demo SCIM Server".to_string(),
        version: "1.0.0".to_string(),
        description: "Simple SCIM server with MCP integration".to_string(),
        supported_resource_types: vec!["User".to_string()],
    };

    let mcp_server = ScimMcpServer::with_info(scim_server, server_info);
    println!("✅ MCP server created");

    // 4. Show available tools
    let tools = mcp_server.get_tools();
    println!("\n🔧 Available Tools ({}):", tools.len());
    for tool in &tools {
        if let Some(name) = tool.get("name").and_then(|n| n.as_str()) {
            if let Some(desc) = tool.get("description").and_then(|d| d.as_str()) {
                println!("  • {}: {}", name, desc);
            }
        }
    }

    // 5. Demonstrate some basic operations
    println!("\n🚀 Testing MCP Operations:");

    // Create a user
    println!("\n1. Creating a user...");
    let create_result = mcp_server
        .execute_tool(
            "scim_create_user",
            json!({
                "user_data": {
                    "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
                    "userName": "demo_user",
                    "name": {
                        "givenName": "Demo",
                        "familyName": "User"
                    },
                    "emails": [
                        {
                            "value": "demo@example.com",
                            "primary": true
                        }
                    ],
                    "active": true
                }
            }),
        )
        .await;

    if create_result.success {
        println!("   ✅ User created successfully");
        println!(
            "   📝 Result: {}",
            serde_json::to_string_pretty(&create_result.content)?
        );
    } else {
        println!("   ❌ User creation failed");
        println!(
            "   📝 Error: {}",
            serde_json::to_string_pretty(&create_result.content)?
        );
    }

    // List users
    println!("\n2. Listing users...");
    let list_result = mcp_server.execute_tool("scim_list_users", json!({})).await;

    if list_result.success {
        println!("   ✅ Users listed successfully");
        if let Some(resources) = list_result.content.get("Resources") {
            if let Some(array) = resources.as_array() {
                println!("   📊 Found {} user(s)", array.len());
            }
        }
    } else {
        println!("   ❌ Listing failed");
    }

    // Get schemas
    println!("\n3. Getting SCIM schemas...");
    let schema_result = mcp_server.execute_tool("scim_get_schemas", json!({})).await;

    if schema_result.success {
        println!("   ✅ Schemas retrieved");
        if let Some(schemas) = schema_result.content.get("Resources") {
            if let Some(array) = schemas.as_array() {
                println!("   📋 Found {} schema(s)", array.len());
                for schema in array {
                    if let Some(id) = schema.get("id").and_then(|i| i.as_str()) {
                        println!("     - {}", id);
                    }
                }
            }
        }
    } else {
        println!("   ❌ Schema retrieval failed");
    }

    // Get server info
    println!("\n4. Getting server information...");
    let info_result = mcp_server.execute_tool("scim_server_info", json!({})).await;

    if info_result.success {
        println!("   ✅ Server info retrieved");
        if let Some(name) = info_result.content.get("name") {
            println!("   🏷️  Server: {}", name);
        }
        if let Some(capabilities) = info_result.content.get("capabilities") {
            println!(
                "   ⚙️  Capabilities: {}",
                serde_json::to_string_pretty(capabilities)?
            );
        }
    }

    // Test error handling
    println!("\n5. Testing error handling...");
    let error_result = mcp_server
        .execute_tool(
            "scim_get_user",
            json!({
                "user_id": "non-existent"
            }),
        )
        .await;

    if !error_result.success {
        println!("   ✅ Error handling working correctly");
        println!(
            "   📝 Error response: {}",
            serde_json::to_string_pretty(&error_result.content)?
        );
    }

    println!("\n🎉 Demo completed successfully!");
    println!("\n💡 This MCP server can now be used by AI agents to:");
    println!("   • Discover available SCIM operations");
    println!("   • Create and manage users");
    println!("   • Query schemas and server capabilities");
    println!("   • Handle errors gracefully");
    println!("\n🔗 To use with AI agents, run: mcp_server.run_stdio().await");

    Ok(())
}

#[cfg(not(feature = "mcp"))]
fn main() {
    eprintln!("This demo requires the 'mcp' feature to be enabled.");
    eprintln!("Please run with: cargo run --example simple_mcp_demo --features mcp");
    std::process::exit(1);
}
