//! Complete MCP Stdio Server Example
//!
//! This example demonstrates how to create and run a complete MCP (Model Context Protocol)
//! stdio server using the SCIM server integration. The server exposes SCIM operations
//! as discoverable tools that AI agents can use for identity management.
//!
//! ## Features Demonstrated
//!
//! - Complete MCP stdio protocol implementation
//! - SCIM User resource operations (Create, Read, Update, Delete)
//! - Tool discovery for AI agents
//! - JSON-RPC message handling
//! - Error handling and validation
//! - Multi-tenant support
//! - ETag-based concurrency control
//!
//! ## Usage
//!
//! Run this example with the MCP feature enabled:
//! ```bash
//! cargo run --example mcp_stdio_server --features mcp
//! ```
//!
//! The server will start and listen on stdin/stdout for MCP protocol messages.
//! You can interact with it using any MCP-compatible client or by sending
//! JSON-RPC messages directly.
//!
//! ## Example MCP Messages
//!
//! ### Initialize the server:
//! ```json
//! {"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"test-client","version":"1.0.0"}}}
//! ```
//!
//! ### List available tools:
//! ```json
//! {"jsonrpc":"2.0","id":2,"method":"tools/list","params":{}}
//! ```
//!
//! ### Create a user:
//! ```json
//! {"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"scim_create_user","arguments":{"user_data":{"schemas":["urn:ietf:params:scim:schemas:core:2.0:User"],"userName":"john.doe@example.com","active":true,"name":{"givenName":"John","familyName":"Doe"}}}}}
//! ```

#[cfg(feature = "mcp")]
use scim_server::{
    mcp_integration::ScimMcpServer,
    multi_tenant::ScimOperation,
    providers::StandardResourceProvider,
    resource_handlers::{create_user_resource_handler, create_group_resource_handler},
    scim_server::ScimServer,
    storage::InMemoryStorage,
};

#[cfg(feature = "mcp")]
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Initialize logging
    env_logger::init();

    eprintln!("🚀 Starting SCIM MCP Stdio Server");
    eprintln!("==================================\n");

    // 1. Create storage and provider
    let storage = InMemoryStorage::new();
    let provider = StandardResourceProvider::new(storage);
    let mut scim_server = ScimServer::new(provider)?;

    // 2. Register User resource type with all operations
    let user_schema = scim_server
        .get_schema_by_id("urn:ietf:params:scim:schemas:core:2.0:User")
        .expect("Failed to get user schema")
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

    eprintln!("✅ Registered User resource type with operations:");
    eprintln!("   • Create (scim_create_user)");
    eprintln!("   • Read (scim_get_user)");
    eprintln!("   • Update (scim_update_user)");
    eprintln!("   • Delete (scim_delete_user)");
    eprintln!("   • Search (scim_search_users, scim_list_users)");
    eprintln!("   • Existence Check (scim_user_exists)");
    eprintln!();

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

        eprintln!("✅ Registered Group resource type with operations:");
        eprintln!("   • Create, Read, Update, Delete");
        eprintln!("   • List, Search operations");
        eprintln!("   (Note: Group-specific MCP tools not yet implemented)");
        eprintln!();
    }

    // 4. Create MCP server
    let mcp_server = ScimMcpServer::new(scim_server);

    // 5. Display available tools
    let tools = mcp_server.get_tools();
    eprintln!("🔧 Available MCP Tools ({} total):", tools.len());
    eprintln!("=====================================");

    for (i, tool) in tools.iter().enumerate() {
        let name = tool.get("name").and_then(|n| n.as_str()).unwrap_or("Unknown");
        let description = tool
            .get("description")
            .and_then(|d| d.as_str())
            .unwrap_or("No description");
        eprintln!("{}. {} - {}", i + 1, name, description);
    }

    eprintln!();
    eprintln!("🎯 Server Capabilities:");
    eprintln!("=======================");
    eprintln!("• JSON-RPC 2.0 protocol support");
    eprintln!("• SCIM 2.0 compliant operations");
    eprintln!("• Multi-tenant isolation");
    eprintln!("• ETag-based optimistic locking");
    eprintln!("• Comprehensive error handling");
    eprintln!("• Async/non-blocking operations");
    eprintln!();

    eprintln!("📡 Starting MCP stdio communication...");
    eprintln!("Listening for JSON-RPC messages on stdin");
    eprintln!("Send MCP protocol messages to interact with the server");
    eprintln!("Use Ctrl+C or send EOF to stop the server");
    eprintln!("==========================================\n");

    // 6. Start the MCP stdio server
    // This will run until EOF is received or the process is terminated
    mcp_server.run_stdio().await?;

    eprintln!("\n✅ SCIM MCP Server shutdown complete");
    Ok(())
}

#[cfg(not(feature = "mcp"))]
fn main() {
    eprintln!("This example requires the 'mcp' feature to be enabled.");
    eprintln!("Run with: cargo run --example mcp_stdio_server --features mcp");
    std::process::exit(1);
}
