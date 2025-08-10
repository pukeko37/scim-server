//! # Logging Example
//!
//! This example demonstrates how to configure and use logging with the SCIM server.
//! The SCIM server uses the standard `log` crate, allowing you to choose your
//! preferred logging backend (env_logger, tracing, slog, etc.).

use scim_server::{
    InMemoryProvider, RequestContext, ScimOperation, ScimServer, TenantContext,
    create_user_resource_handler,
};
use serde_json::json;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging - you can choose different backends:

    // Option 1: env_logger (simple, good for development)
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("debug"))
        .format_timestamp_secs()
        .init();

    // Option 2: For production, you might use structured logging with tracing
    // tracing_subscriber::fmt::init();

    // Option 3: Or configure with custom filters
    // env_logger::Builder::new()
    //     .filter_level(log::LevelFilter::Info)
    //     .filter_module("scim_server::providers", log::LevelFilter::Debug)
    //     .filter_module("scim_server::resource", log::LevelFilter::Trace)
    //     .init();

    log::info!("🚀 Starting SCIM Server Logging Example");
    log::info!("========================================");

    // Create provider and server
    let provider = InMemoryProvider::new();
    let mut server = ScimServer::new(provider)?;

    // Register User resource type
    let user_schema = server
        .get_schema_by_id("urn:ietf:params:scim:schemas:core:2.0:User")
        .unwrap()
        .clone();
    let user_handler = create_user_resource_handler(user_schema);
    server.register_resource_type(
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

    log::info!("✅ SCIM Server initialized with logging");

    // Demonstrate single-tenant operations with logging
    log::info!("📝 Demonstrating single-tenant operations...");

    let single_context = RequestContext::with_generated_id();

    // Create a user - this will generate INFO logs
    let user_data = json!({
        "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
        "userName": "john.doe",
        "displayName": "John Doe",
        "emails": [{
            "value": "john.doe@example.com",
            "type": "work",
            "primary": true
        }],
        "active": true
    });

    log::info!("Creating user with structured logging...");
    let created_user = server
        .create_resource("User", user_data.clone(), &single_context)
        .await?;
    let user_id = created_user.get_id().unwrap();

    log::info!("User created with ID: {}", user_id);

    // Get the user - this will generate DEBUG logs
    log::info!("Retrieving user...");
    let retrieved_user = server
        .get_resource("User", &user_id, &single_context)
        .await?;

    if retrieved_user.is_some() {
        log::info!("✅ User retrieved successfully");
    }

    // Update the user - this will generate INFO and TRACE logs
    log::info!("Updating user...");
    let update_data = json!({
        "id": user_id,
        "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
        "userName": "john.doe",
        "displayName": "John Doe (Updated)",
        "active": false
    });

    let updated_user = server
        .update_resource("User", &user_id, update_data, &single_context)
        .await?;
    log::info!("User updated: active = {}", updated_user.is_active());

    // List users - this will generate DEBUG logs
    log::info!("Listing all users...");
    let users = server.list_resources("User", &single_context).await?;
    log::info!("Found {} users", users.len());

    // Demonstrate multi-tenant operations with logging
    log::info!("\n📝 Demonstrating multi-tenant operations...");

    let tenant_context = TenantContext::new("tenant-1".to_string(), "client-1".to_string());
    let multi_context = RequestContext::with_tenant_generated_id(tenant_context);

    // Create user in tenant - logs will include tenant information
    log::info!("Creating user in tenant context...");
    let tenant_user = server
        .create_resource("User", user_data, &multi_context)
        .await?;
    let tenant_user_id = tenant_user.get_id().unwrap();

    log::info!("Tenant user created with ID: {}", tenant_user_id);

    // List users in tenant
    let tenant_users = server.list_resources("User", &multi_context).await?;
    log::info!("Found {} users in tenant", tenant_users.len());

    // Demonstrate error logging
    log::info!("\n📝 Demonstrating error scenarios...");

    // Try to get non-existent user - this will generate DEBUG logs
    log::info!("Attempting to retrieve non-existent user...");
    let missing_user = server
        .get_resource("User", "non-existent-id", &single_context)
        .await?;

    if missing_user.is_none() {
        log::warn!("User not found (expected)");
    }

    // Try to delete non-existent user - this will generate WARN logs
    log::info!("Attempting to delete non-existent user...");
    match server
        .delete_resource("User", "non-existent-id", &single_context)
        .await
    {
        Ok(_) => log::info!("Delete succeeded (unexpected)"),
        Err(e) => log::warn!("Delete failed as expected: {}", e),
    }

    // Clean up - delete the created users
    log::info!("\n🧹 Cleaning up...");

    server
        .delete_resource("User", &user_id, &single_context)
        .await?;
    log::info!("Deleted single-tenant user");

    server
        .delete_resource("User", &tenant_user_id, &multi_context)
        .await?;
    log::info!("Deleted multi-tenant user");

    log::info!("\n✨ **LOGGING FEATURES DEMONSTRATED**");
    log::info!("==================================");
    log::info!("✓ Structured logging with request IDs");
    log::info!("✓ Multi-tenant context in logs");
    log::info!("✓ Different log levels (TRACE, DEBUG, INFO, WARN)");
    log::info!("✓ Operation-specific logging");
    log::info!("✓ Error condition logging");
    log::info!("✓ Resource lifecycle tracking");

    log::info!("\n📋 **LOGGING CONFIGURATION OPTIONS**");
    log::info!("===================================");
    log::info!("Environment Variables:");
    log::info!("  RUST_LOG=debug                    # Enable debug logging");
    log::info!("  RUST_LOG=scim_server=trace        # Trace SCIM operations");
    log::info!("  RUST_LOG=scim_server::providers=debug  # Debug provider operations");
    log::info!("");
    log::info!("Programmatic Configuration:");
    log::info!("  env_logger::Builder::from_env(...)");
    log::info!("  tracing_subscriber::fmt::init()");
    log::info!("  Custom filtering by module/level");

    log::info!("\n🎯 **PRODUCTION RECOMMENDATIONS**");
    log::info!("=================================");
    log::info!("✓ Use structured logging (JSON format)");
    log::info!("✓ Set appropriate log levels (INFO+ for production)");
    log::info!("✓ Include request tracing for debugging");
    log::info!("✓ Monitor error rates and patterns");
    log::info!("✓ Use log aggregation systems (ELK, Splunk, etc.)");

    Ok(())
}
