//! # Logging Backends Comparison Example
//!
//! This example demonstrates how to configure different logging backends
//! with the SCIM server, showing the flexibility of the log facade approach.

use scim_server::{
    RequestContext, providers::StandardResourceProvider, resource::provider::ResourceProvider,
    storage::InMemoryStorage,
};
use serde_json::json;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🔧 SCIM Server Logging Backends Comparison");
    println!("==========================================\n");

    // We'll demonstrate different backends by running the same operations
    // with different logging configurations.

    println!("📝 Setting up SCIM server...");

    // Create StandardResourceProvider with InMemoryStorage
    let storage = InMemoryStorage::new();
    let provider = StandardResourceProvider::new(storage);

    // We'll create request contexts as needed for our operations

    println!("✅ SCIM server configured\n");

    // Initialize different logging backends
    // Note: In a real application, you would choose ONE of these approaches

    println!("🚀 **BACKEND 1: env_logger (Simple)**");
    println!("====================================");

    // Backend 1: Simple env_logger
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
        .format_timestamp_secs()
        .format_module_path(false)
        .init();

    println!("Configuration:");
    println!("  - Backend: env_logger");
    println!("  - Format: Simple text with timestamps");
    println!("  - Good for: Development, simple applications");
    println!("  - Setup: env_logger::init()");
    println!("\nExample operations with env_logger:");

    // Perform some operations to show logging
    let context = RequestContext::with_generated_id();

    let user_data = json!({
        "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
        "userName": "alice.env",
        "displayName": "Alice (env_logger)",
        "active": true
    });

    // This will produce env_logger output
    let user = provider
        .create_resource("User", user_data, &context)
        .await?;
    let user_id = user.get_id().unwrap();

    let _retrieved = provider.get_resource("User", user_id, &context).await?;
    provider.delete_resource("User", user_id, &context).await?;

    println!("\n{}", "=".repeat(50));

    // Note: In a real application, you can't reinitialize logging
    // This is just for demonstration purposes
    println!("\n📊 **BACKEND 2: Structured Logging Concept**");
    println!("============================================");

    println!("Configuration for tracing (structured logging):");
    println!("  - Backend: tracing-subscriber");
    println!("  - Format: JSON for log aggregation");
    println!("  - Good for: Production, monitoring systems");
    println!("  - Features: Spans, fields, correlation IDs");
    println!("\nExample setup code:");
    println!(
        r#"
use tracing_subscriber::{{layer::SubscriberExt, util::SubscriberInitExt}};

tracing_subscriber::registry()
    .with(tracing_subscriber::EnvFilter::new("info"))
    .with(tracing_subscriber::fmt::layer().json())
    .init();
"#
    );

    println!("Example JSON output would look like:");
    println!(
        r#"
{{
  "timestamp": "2024-01-15T10:30:45.123Z",
  "level": "INFO",
  "target": "scim_server::providers::in_memory",
  "message": "Creating User resource for tenant 'default'",
  "fields": {{
    "request_id": "abc-123-def",
    "resource_type": "User",
    "tenant_id": "default"
  }}
}}
"#
    );

    println!("\n{}", "=".repeat(50));

    println!("\n🎯 **BACKEND 3: Custom Configuration**");
    println!("=====================================");

    println!("Advanced env_logger configuration:");
    println!(
        r#"
env_logger::Builder::new()
    .filter_level(log::LevelFilter::Info)
    .filter_module("scim_server::providers", log::LevelFilter::Debug)
    .filter_module("scim_server::resource", log::LevelFilter::Trace)
    .format(|buf, record| {{
        writeln!(buf,
            "{{}} [{{}}] {{}}: {{}}",
            chrono::Utc::now().format("%Y-%m-%d %H:%M:%S"),
            record.level(),
            record.target(),
            record.args()
        )
    }})
    .init();
"#
    );

    println!("Benefits:");
    println!("  ✓ Module-specific log levels");
    println!("  ✓ Custom formatting");
    println!("  ✓ Performance optimization");
    println!("  ✓ Environment variable support");

    println!("\n{}", "=".repeat(50));

    println!("\n🏭 **PRODUCTION RECOMMENDATIONS**");
    println!("=================================");

    println!("For Development:");
    println!("  • Use env_logger with DEBUG level");
    println!("  • Enable colored output");
    println!("  • Use module filtering to focus on relevant components");
    println!("  • Example: RUST_LOG=debug,scim_server=trace");

    println!("\nFor Production:");
    println!("  • Use structured logging (JSON format)");
    println!("  • Set INFO level or higher");
    println!("  • Include request correlation IDs");
    println!("  • Ship logs to centralized system (ELK, Splunk, etc.)");
    println!("  • Monitor error rates and response times");

    println!("\nFor High-Performance:");
    println!("  • Use async logging backends");
    println!("  • Buffer log writes");
    println!("  • Consider sampling for high-volume DEBUG logs");
    println!("  • Use WARN level for error paths only");

    println!("\n📋 **ENVIRONMENT VARIABLE EXAMPLES**");
    println!("===================================");

    println!("Basic configurations:");
    println!("  RUST_LOG=info                     # INFO level everywhere");
    println!("  RUST_LOG=debug                    # DEBUG level everywhere");
    println!("  RUST_LOG=scim_server=trace        # TRACE for SCIM server only");
    println!("  RUST_LOG=warn,scim_server=debug   # WARN default, DEBUG for SCIM");

    println!("\nModule-specific configurations:");
    println!("  RUST_LOG=scim_server::providers=debug     # Debug provider operations");
    println!("  RUST_LOG=scim_server::resource=trace      # Trace resource operations");
    println!("  RUST_LOG=scim_server::scim_server=info    # Info for server operations");

    println!("\nCombined configurations:");
    println!("  RUST_LOG=info,scim_server::providers=debug,my_app=trace");

    println!("\n🔧 **INTEGRATION EXAMPLES**");
    println!("===========================");

    println!("With Actix Web:");
    println!(
        r#"
use actix_web::middleware::Logger;

App::new()
    .wrap(Logger::default())
    .service(scim_endpoints)
"#
    );

    println!("With Axum:");
    println!(
        r#"
use tower_http::trace::TraceLayer;

Router::new()
    .layer(TraceLayer::new_for_http())
    .nest("/scim/v2", scim_routes)
"#
    );

    println!("With Request ID middleware:");
    println!(
        r#"
// Custom middleware to add request IDs
let request_id = uuid::Uuid::new_v4().to_string();
let context = RequestContext::new(request_id, None);
"#
    );

    println!("\n✨ **SUMMARY**");
    println!("=============");
    println!("✓ SCIM server uses standard log facade");
    println!("✓ Choose any logging backend (env_logger, tracing, slog)");
    println!("✓ Structured logging with request IDs and tenant context");
    println!("✓ Configurable per-module log levels");
    println!("✓ Production-ready logging patterns");
    println!("✓ Easy integration with web frameworks");

    println!("\n🎯 **NEXT STEPS**");
    println!("================");
    println!("1. Choose a logging backend based on your needs");
    println!("2. Configure appropriate log levels for your environment");
    println!("3. Set up log aggregation for production");
    println!("4. Add monitoring and alerting based on log patterns");
    println!("5. Test logging configuration in staging environment");

    Ok(())
}
