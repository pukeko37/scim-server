//! Group Example - Demonstrating Group schema functionality in SCIM server
//!
//! This example shows how to:
//! 1. Set up a SCIM server with Group support
//! 2. Create, read, update, and delete Group resources
//! 3. Manage Group memberships
//! 4. Validate Group schemas
//!
//! Run with: cargo run --example group_example

use scim_server::{
    RequestContext, providers::StandardResourceProvider, resource::provider::ResourceProvider,
    resource_handlers::create_group_resource_handler, schema::SchemaRegistry,
    scim_server::ScimServer, storage::InMemoryStorage,
};
use serde_json::json;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 SCIM Server Group Example");
    println!("=============================\n");

    // 1. Create StandardResourceProvider with InMemoryStorage
    let storage = InMemoryStorage::new();
    let provider = StandardResourceProvider::new(storage);

    // 2. Create ScimServer
    let mut server = ScimServer::new(provider)?;

    // 3. Load Group schema and register Group resource type
    let registry = SchemaRegistry::new()?;
    let group_schema = registry.get_group_schema().clone();
    let group_handler = create_group_resource_handler(group_schema);

    server.register_resource_type(
        "Group",
        group_handler,
        vec![
            scim_server::multi_tenant::ScimOperation::Create,
            scim_server::multi_tenant::ScimOperation::Read,
            scim_server::multi_tenant::ScimOperation::Update,
            scim_server::multi_tenant::ScimOperation::Delete,
            scim_server::multi_tenant::ScimOperation::List,
            scim_server::multi_tenant::ScimOperation::Search,
        ],
    )?;

    println!("✅ Group resource type registered successfully");

    // 4. Create test context
    let context = RequestContext::new("group-example".to_string());

    // 5. Demonstrate Group creation
    println!("\n📝 Creating Groups...");

    let engineering_group = json!({
        "displayName": "Engineering Team",
        "members": [
            {
                "value": "user-alice",
                "$ref": "https://example.com/v2/Users/user-alice",
                "type": "User"
            },
            {
                "value": "user-bob",
                "$ref": "https://example.com/v2/Users/user-bob",
                "type": "User"
            }
        ]
    });

    let created_engineering = server
        .create_resource("Group", engineering_group, &context)
        .await?;

    println!(
        "✅ Created Engineering Team: {}",
        created_engineering.get_id().unwrap_or("unknown")
    );

    let marketing_group = json!({
        "displayName": "Marketing Team",
        "members": [
            {
                "value": "user-charlie",
                "$ref": "https://example.com/v2/Users/user-charlie",
                "type": "User"
            }
        ]
    });

    let created_marketing = server
        .create_resource("Group", marketing_group, &context)
        .await?;

    println!(
        "✅ Created Marketing Team: {}",
        created_marketing.get_id().unwrap_or("unknown")
    );

    // 6. Demonstrate Group retrieval
    println!("\n🔍 Retrieving Groups...");

    let engineering_id = created_engineering.get_id().unwrap();
    let retrieved_group = server
        .get_resource("Group", &engineering_id, &context)
        .await?;

    if let Some(group) = retrieved_group {
        let display_name = group
            .get_attribute("displayName")
            .and_then(|v| v.as_str())
            .unwrap_or("Unknown");

        let member_count = group
            .get_attribute("members")
            .and_then(|v| v.as_array())
            .map(|arr| arr.len())
            .unwrap_or(0);

        println!(
            "✅ Retrieved group: {} with {} members",
            display_name, member_count
        );
    }

    // 7. Demonstrate Group listing
    println!("\n📋 Listing all Groups...");

    let all_groups = server.list_resources("Group", &context).await?;
    println!("✅ Found {} groups:", all_groups.len());

    for group in &all_groups {
        let display_name = group
            .get_attribute("displayName")
            .and_then(|v| v.as_str())
            .unwrap_or("Unknown");

        let id = group.get_id().unwrap_or("unknown");
        println!("   📁 {}: {}", display_name, id);
    }

    // 8. Demonstrate Group membership updates
    println!("\n👥 Updating Group membership...");

    let engineering_id = created_engineering.get_id().unwrap();
    let updated_engineering_data = json!({
        "id": engineering_id,
        "displayName": "Engineering Team (Updated)",
        "members": [
            {
                "value": "user-alice",
                "$ref": "https://example.com/v2/Users/user-alice",
                "type": "User"
            },
            {
                "value": "user-bob",
                "$ref": "https://example.com/v2/Users/user-bob",
                "type": "User"
            },
            {
                "value": "user-david",
                "$ref": "https://example.com/v2/Users/user-david",
                "type": "User"
            }
        ]
    });

    let updated_group = server
        .update_resource("Group", &engineering_id, updated_engineering_data, &context)
        .await?;

    let updated_display_name = updated_group
        .get_attribute("displayName")
        .and_then(|v| v.as_str())
        .unwrap_or("Unknown");

    let updated_member_count = updated_group
        .get_attribute("members")
        .and_then(|v| v.as_array())
        .map(|arr| arr.len())
        .unwrap_or(0);

    println!(
        "✅ Updated group: {} now has {} members",
        updated_display_name, updated_member_count
    );

    // 9. Demonstrate Group member listing
    println!("\n👤 Group members details...");

    if let Some(members) = updated_group
        .get_attribute("members")
        .and_then(|v| v.as_array())
    {
        println!("📋 Members of {}:", updated_display_name);
        for member in members {
            if let Some(member_obj) = member.as_object() {
                let user_id = member_obj
                    .get("value")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown");
                let user_type = member_obj
                    .get("type")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown");
                let user_ref = member_obj
                    .get("$ref")
                    .and_then(|v| v.as_str())
                    .unwrap_or("no reference");

                println!("   👤 {} ({}): {}", user_id, user_type, user_ref);
            }
        }
    }

    // 10. Demonstrate Group search by display name
    println!("\n🔍 Searching Groups by display name...");

    let found_group = server
        .provider()
        .find_resource_by_attribute("Group", "displayName", &json!("Marketing Team"), &context)
        .await?;

    match found_group {
        Some(group) => {
            let display_name = group
                .get_attribute("displayName")
                .and_then(|v| v.as_str())
                .unwrap_or("Unknown");
            let id = group.get_id().unwrap_or("unknown");
            println!(
                "✅ Found group by display name: {} (ID: {})",
                display_name, id
            );
        }
        None => println!("❌ Group not found by display name"),
    }

    // 11. Demonstrate Group validation
    println!("\n🔍 Group schema validation...");

    let invalid_group = json!({
        "displayName": "", // Invalid: empty display name
        "members": "not-an-array" // Invalid: should be an array
    });

    match server
        .create_resource("Group", invalid_group, &context)
        .await
    {
        Ok(_) => println!("⚠️  Validation should have failed"),
        Err(e) => println!("✅ Validation correctly failed: {}", e),
    }

    // 12. Test resource existence
    println!("\n🔍 Testing resource existence...");

    let marketing_id = created_marketing.get_id().unwrap();
    let exists = server
        .provider()
        .resource_exists("Group", &marketing_id, &context)
        .await?;
    println!("✅ Marketing group exists: {}", exists);

    // 13. Demonstrate Group deletion
    println!("\n🗑️  Deleting Groups...");

    server
        .delete_resource("Group", &marketing_id, &context)
        .await?;
    println!("✅ Deleted Marketing Team");

    // Verify deletion
    let exists_after = server
        .provider()
        .resource_exists("Group", &marketing_id, &context)
        .await?;
    println!("✅ Marketing group exists after deletion: {}", exists_after);

    // List remaining groups
    let remaining_groups = server.list_resources("Group", &context).await?;
    println!("📊 Groups remaining: {}", remaining_groups.len());

    // 14. Provider statistics
    println!("\n📊 Provider Statistics...");

    let stats = server.provider().get_stats().await;
    println!("📈 Provider Statistics:");
    println!("   • Total tenants: {}", stats.tenant_count);
    println!("   • Total resources: {}", stats.total_resources);
    println!("   • Resource types: {:?}", stats.resource_types);
    println!("   • Resource type count: {}", stats.resource_type_count);

    println!("\n✅ Group Example Complete!");
    println!("🎉 Successfully demonstrated:");
    println!("   • Group creation with StandardResourceProvider and InMemoryStorage");
    println!("   • Group retrieval and listing");
    println!("   • Group membership management");
    println!("   • Group search and validation");
    println!("   • Group deletion and resource existence checks");
    println!("   • Provider statistics");

    Ok(())
}
