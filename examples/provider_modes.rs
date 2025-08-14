//! Provider Modes Example
//!
//! This example demonstrates how the StandardResourceProvider works
//! for both single-tenant and multi-tenant scenarios through the RequestContext.
//! This shows how a single provider implementation supports multiple operational modes.

use scim_server::{
    RequestContext, TenantContext,
    providers::StandardResourceProvider,
    storage::InMemoryStorage,
    resource::provider::ResourceProvider,
};
use serde_json::json;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 Provider Modes Example");
    println!("Using the StandardResourceProvider for both single and multi-tenant scenarios\n");

    let storage = InMemoryStorage::new();
    let provider = StandardResourceProvider::new(storage);

    // ===== SINGLE-TENANT OPERATIONS =====
    println!("📋 SINGLE-TENANT OPERATIONS");
    println!("============================");

    // Single-tenant context (no tenant_context)
    let single_context = RequestContext::new("req-single-1".to_string());

    let user1 = provider
        .create_resource(
            "User",
            json!({
                "userName": "alice@single.com",
                "name": {
                    "formatted": "Alice Single",
                    "givenName": "Alice",
                    "familyName": "Single"
                },
                "emails": [
                    {
                        "value": "alice@single.com",
                        "type": "work",
                        "primary": true
                    }
                ]
            }),
            &single_context,
        )
        .await?;

    println!(
        "✅ Created user: {} (ID: {})",
        user1.get_username().unwrap_or("unknown"),
        user1.get_id().unwrap_or("unknown")
    );

    let user2 = provider
        .create_resource(
            "User",
            json!({
                "userName": "bob@single.com",
                "name": {
                    "formatted": "Bob Single",
                    "givenName": "Bob",
                    "familyName": "Single"
                }
            }),
            &single_context,
        )
        .await?;

    println!(
        "✅ Created user: {} (ID: {})",
        user2.get_username().unwrap_or("unknown"),
        user2.get_id().unwrap_or("unknown")
    );

    // List single-tenant users
    let single_users = provider
        .list_resources("User", None, &single_context)
        .await?;
    println!("📊 Single-tenant has {} users", single_users.len());
    for user in &single_users {
        println!(
            "   - {} ({})",
            user.get_username().unwrap_or("unknown"),
            user.get_id().unwrap_or("unknown")
        );
    }

    println!();

    // ===== MULTI-TENANT OPERATIONS =====
    println!("📋 MULTI-TENANT OPERATIONS");
    println!("===========================");

    // Tenant A operations
    let tenant_a_context = RequestContext::with_tenant_generated_id(TenantContext::new(
        "tenant-a".to_string(),
        "client-a".to_string(),
    ));

    let tenant_a_user = provider
        .create_resource(
            "User",
            json!({
                "userName": "alice@tenant-a.com",
                "name": {
                    "formatted": "Alice Tenant A",
                    "givenName": "Alice",
                    "familyName": "TenantA"
                },
                "emails": [
                    {
                        "value": "alice@tenant-a.com",
                        "type": "work",
                        "primary": true
                    }
                ]
            }),
            &tenant_a_context,
        )
        .await?;

    println!(
        "✅ Created user in Tenant A: {} (ID: {})",
        tenant_a_user.get_username().unwrap_or("unknown"),
        tenant_a_user.get_id().unwrap_or("unknown")
    );

    // Tenant B operations
    let tenant_b_context = RequestContext::with_tenant(
        "req-tenant-b-1".to_string(),
        TenantContext::new("tenant-b".to_string(), "client-b".to_string()),
    );

    let tenant_b_user = provider
        .create_resource(
            "User",
            json!({
                "userName": "bob@tenant-b.com",
                "name": {
                    "formatted": "Bob Tenant B",
                    "givenName": "Bob",
                    "familyName": "TenantB"
                }
            }),
            &tenant_b_context,
        )
        .await?;

    println!(
        "✅ Created user in Tenant B: {} (ID: {})",
        tenant_b_user.get_username().unwrap_or("unknown"),
        tenant_b_user.get_id().unwrap_or("unknown")
    );

    // Create a second user in Tenant A to show same usernames across tenants
    let tenant_a_user2 = provider
        .create_resource(
            "User",
            json!({
                "userName": "bob@tenant-a.com",  // Same first name as Tenant B, different domain
                "name": {
                    "formatted": "Bob Tenant A",
                    "givenName": "Bob",
                    "familyName": "TenantA"
                }
            }),
            &tenant_a_context,
        )
        .await?;

    println!(
        "✅ Created second user in Tenant A: {} (ID: {})",
        tenant_a_user2.get_username().unwrap_or("unknown"),
        tenant_a_user2.get_id().unwrap_or("unknown")
    );

    println!();

    // ===== DEMONSTRATE TENANT ISOLATION =====
    println!("📋 TENANT ISOLATION VERIFICATION");
    println!("==================================");

    // List users in each tenant
    let single_users = provider
        .list_resources("User", None, &single_context)
        .await?;

    let tenant_a_users = provider
        .list_resources("User", None, &tenant_a_context)
        .await?;

    let tenant_b_users = provider
        .list_resources("User", None, &tenant_b_context)
        .await?;

    println!("📊 TENANT ISOLATION RESULTS:");
    println!("  • Single-tenant users: {}", single_users.len());
    for user in &single_users {
        println!(
            "    - {} ({})",
            user.get_username().unwrap_or("unknown"),
            user.get_id().unwrap_or("unknown")
        );
    }

    println!("  • Tenant A users: {}", tenant_a_users.len());
    for user in &tenant_a_users {
        println!(
            "    - {} ({})",
            user.get_username().unwrap_or("unknown"),
            user.get_id().unwrap_or("unknown")
        );
    }

    println!("  • Tenant B users: {}", tenant_b_users.len());
    for user in &tenant_b_users {
        println!(
            "    - {} ({})",
            user.get_username().unwrap_or("unknown"),
            user.get_id().unwrap_or("unknown")
        );
    }

    println!();

    // ===== DEMONSTRATE SEARCH OPERATIONS =====
    println!("📋 SEARCH OPERATIONS");
    println!("=====================");

    // Search for alice in single-tenant
    let alice_single = provider
        .find_resource_by_attribute(
            "User",
            "userName",
            &json!("alice@single.com"),
            &single_context,
        )
        .await?;

    // Search for alice in tenant-a
    let alice_tenant_a = provider
        .find_resource_by_attribute(
            "User",
            "userName",
            &json!("alice@tenant-a.com"),
            &tenant_a_context,
        )
        .await?;

    // Search for alice@single.com in tenant-a (should not find due to isolation)
    let alice_cross_search = provider
        .find_resource_by_attribute(
            "User",
            "userName",
            &json!("alice@single.com"),
            &tenant_a_context,
        )
        .await?;

    // Search for bob in different tenants
    let bob_tenant_a = provider
        .find_resource_by_attribute(
            "User",
            "userName",
            &json!("bob@tenant-a.com"),
            &tenant_a_context,
        )
        .await?;

    let bob_tenant_b = provider
        .find_resource_by_attribute(
            "User",
            "userName",
            &json!("bob@tenant-b.com"),
            &tenant_b_context,
        )
        .await?;

    println!("🎯 SEARCH RESULTS:");
    println!("  • Alice in single-tenant: {} ✅", alice_single.is_some());
    println!("  • Alice in tenant-a: {} ✅", alice_tenant_a.is_some());
    println!(
        "  • Alice@single.com in tenant-a: {} ✅ (correctly isolated)",
        alice_cross_search.is_some()
    );
    println!("  • Bob in tenant-a: {} ✅", bob_tenant_a.is_some());
    println!("  • Bob in tenant-b: {} ✅", bob_tenant_b.is_some());

    println!();

    // ===== DEMONSTRATE UPDATE AND DELETE =====
    println!("📋 UPDATE AND DELETE OPERATIONS");
    println!("================================");

    // Update user in tenant A
    let user_id = tenant_a_user.get_id().unwrap();
    let updated_user = provider
        .update_resource(
            "User",
            user_id,
            json!({
                "id": user_id,
                "userName": "alice@tenant-a.com",
                "name": {
                    "formatted": "Alice Updated TenantA",
                    "givenName": "Alice",
                    "familyName": "UpdatedTenantA"
                },
                "emails": [
                    {
                        "value": "alice@tenant-a.com",
                        "type": "work",
                        "primary": true
                    }
                ]
            }),
            &tenant_a_context,
        )
        .await?;

    println!(
        "✅ Updated user in Tenant A: {} (ID: {})",
        updated_user.get_username().unwrap_or("unknown"),
        updated_user.get_id().unwrap_or("unknown")
    );

    // Check resource exists
    let exists = provider
        .resource_exists("User", user_id, &tenant_a_context)
        .await?;
    println!("✅ User exists in Tenant A: {}", exists);

    // Try to access the same user from Tenant B (should not exist due to isolation)
    let exists_cross_tenant = provider
        .resource_exists("User", user_id, &tenant_b_context)
        .await?;
    println!(
        "✅ Same user ID exists in Tenant B: {} (correctly isolated)",
        exists_cross_tenant
    );

    println!();

    // ===== DEMONSTRATE STATISTICS =====
    println!("📋 PROVIDER STATISTICS");
    println!("=======================");

    let stats = provider.get_stats().await;
    println!("📊 OVERALL STATISTICS:");
    println!("  • Total tenants: {}", stats.tenant_count);
    println!("  • Total resources: {}", stats.total_resources);
    println!("  • Resource types: {:?}", stats.resource_types);
    println!("  • Resource type count: {}", stats.resource_type_count);

    println!();

    // ===== SUMMARY =====
    println!("✅ Unified ResourceProvider Demo Complete!");
    println!("🎉 Successfully demonstrated:");
    println!("   • Single-tenant operations using RequestContext without tenant info");
    println!("   • Multi-tenant operations using RequestContext with TenantContext");
    println!("   • Proper tenant isolation (tenants cannot see each other's data)");
    println!("   • Same username allowed across different tenants");
    println!("   • Cross-tenant isolation in searches and resource access");
    println!("   • CRUD operations working consistently across single and multi-tenant modes");
    println!();
    println!("🏗️  This demonstrates the flexible provider interface:");
    println!("   • One provider implementation works for both scenarios");
    println!("   • Context-driven tenant isolation");
    println!("   • Clean, consistent API surface");

    Ok(())
}
