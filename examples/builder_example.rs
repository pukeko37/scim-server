//! SCIM Server Builder Pattern Example
//!
//! This example demonstrates how to use the new ScimServerBuilder to configure
//! a SCIM server with different endpoint URLs and tenant handling strategies.
//! It also shows how $ref fields are automatically generated based on the
//! server configuration.

use scim_server::{
    ScimServerBuilder, TenantStrategy, RequestContext, TenantContext,
    providers::StandardResourceProvider, storage::InMemoryStorage,
    resource_handlers::{create_user_resource_handler, create_group_resource_handler},
    resource::ScimOperation,
};
use serde_json::json;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🏗️  SCIM Server Builder Pattern Example");
    println!("========================================\n");

    // Example 1: Single Tenant Server
    println!("1. Single Tenant Configuration");
    println!("   Base URL: https://scim.company.com");
    println!("   Strategy: Single tenant (no tenant in URLs)");

    let storage1 = InMemoryStorage::new();
    let provider1 = StandardResourceProvider::new(storage1);

    let mut single_tenant_server = ScimServerBuilder::new(provider1)
        .with_base_url("https://scim.company.com")
        .with_tenant_strategy(TenantStrategy::SingleTenant)
        .build()?;

    // Register User and Group resource types
    let user_schema = single_tenant_server
        .get_schema_by_id("urn:ietf:params:scim:schemas:core:2.0:User")
        .expect("User schema should exist").clone();
    let user_handler = create_user_resource_handler(user_schema);
    single_tenant_server.register_resource_type("User", user_handler,
        vec![ScimOperation::Create, ScimOperation::Read])?;

    let group_schema = single_tenant_server
        .get_schema_by_id("urn:ietf:params:scim:schemas:core:2.0:Group")
        .expect("Group schema should exist").clone();
    let group_handler = create_group_resource_handler(group_schema);
    single_tenant_server.register_resource_type("Group", group_handler,
        vec![ScimOperation::Create, ScimOperation::Read])?;

    // Create a context (no tenant needed for single tenant)
    let single_tenant_context = RequestContext::with_generated_id();

    // Create a user
    let user_data = json!({
        "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
        "userName": "john.doe@company.com",
        "name": {"givenName": "John", "familyName": "Doe"}
    });

    let user = single_tenant_server.create_resource("User", user_data, &single_tenant_context).await?;
    let user_id = user.get_id().unwrap();

    // Create a group with the user as a member
    let group_data = json!({
        "schemas": ["urn:ietf:params:scim:schemas:core:2.0:Group"],
        "displayName": "Engineering Team",
        "members": [{
            "value": user_id,
            "type": "User",
            "display": "John Doe"
        }]
    });

    let group_json = single_tenant_server
        .create_resource_with_refs("Group", group_data, &single_tenant_context)
        .await?;

    let ref_url = group_json["members"][0]["$ref"].as_str().unwrap();
    println!("   ✅ Generated $ref: {}", ref_url);
    assert_eq!(ref_url, format!("https://scim.company.com/v2/Users/{}", user_id));

    println!();

    // Example 2: Multi-Tenant with Subdomain Strategy
    println!("2. Multi-Tenant Subdomain Configuration");
    println!("   Base URL: https://scim.example.com");
    println!("   Strategy: Subdomain-based (tenant.scim.example.com)");

    let storage2 = InMemoryStorage::new();
    let provider2 = StandardResourceProvider::new(storage2);

    let mut subdomain_server = ScimServerBuilder::new(provider2)
        .with_base_url("https://scim.example.com")
        .with_tenant_strategy(TenantStrategy::Subdomain)
        .build()?;

    // Register resource types
    let user_schema = subdomain_server
        .get_schema_by_id("urn:ietf:params:scim:schemas:core:2.0:User")
        .expect("User schema should exist").clone();
    let user_handler = create_user_resource_handler(user_schema);
    subdomain_server.register_resource_type("User", user_handler,
        vec![ScimOperation::Create, ScimOperation::Read])?;

    let group_schema = subdomain_server
        .get_schema_by_id("urn:ietf:params:scim:schemas:core:2.0:Group")
        .expect("Group schema should exist").clone();
    let group_handler = create_group_resource_handler(group_schema);
    subdomain_server.register_resource_type("Group", group_handler,
        vec![ScimOperation::Create, ScimOperation::Read])?;

    // Create tenant context
    let tenant_context = TenantContext::new("acme-corp".to_string(), "client-123".to_string());
    let subdomain_context = RequestContext::with_tenant_generated_id(tenant_context);

    // Create user and group for subdomain tenant
    let tenant_user_data = json!({
        "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
        "userName": "alice@acme-corp.com",
        "name": {"givenName": "Alice", "familyName": "Smith"}
    });

    let tenant_user = subdomain_server.create_resource("User", tenant_user_data, &subdomain_context).await?;
    let tenant_user_id = tenant_user.get_id().unwrap();

    let tenant_group_data = json!({
        "schemas": ["urn:ietf:params:scim:schemas:core:2.0:Group"],
        "displayName": "ACME Development Team",
        "members": [{
            "value": tenant_user_id,
            "type": "User",
            "display": "Alice Smith"
        }]
    });

    let tenant_group_json = subdomain_server
        .create_resource_with_refs("Group", tenant_group_data, &subdomain_context)
        .await?;

    let tenant_ref_url = tenant_group_json["members"][0]["$ref"].as_str().unwrap();
    println!("   ✅ Generated $ref: {}", tenant_ref_url);
    assert_eq!(tenant_ref_url, format!("https://acme-corp.scim.example.com/v2/Users/{}", tenant_user_id));

    println!();

    // Example 3: Multi-Tenant with Path-Based Strategy
    println!("3. Multi-Tenant Path-Based Configuration");
    println!("   Base URL: https://api.company.com");
    println!("   Strategy: Path-based (/tenant-id/v2)");

    let storage3 = InMemoryStorage::new();
    let provider3 = StandardResourceProvider::new(storage3);

    let mut path_server = ScimServerBuilder::new(provider3)
        .with_base_url("https://api.company.com")
        .with_tenant_strategy(TenantStrategy::PathBased)
        .build()?;

    // Register resource types
    let user_schema = path_server
        .get_schema_by_id("urn:ietf:params:scim:schemas:core:2.0:User")
        .expect("User schema should exist").clone();
    let user_handler = create_user_resource_handler(user_schema);
    path_server.register_resource_type("User", user_handler,
        vec![ScimOperation::Create, ScimOperation::Read])?;

    let group_schema = path_server
        .get_schema_by_id("urn:ietf:params:scim:schemas:core:2.0:Group")
        .expect("Group schema should exist").clone();
    let group_handler = create_group_resource_handler(group_schema);
    path_server.register_resource_type("Group", group_handler,
        vec![ScimOperation::Create, ScimOperation::Read])?;

    // Create tenant context for path-based tenant
    let path_tenant_context = TenantContext::new("enterprise".to_string(), "ent-client-456".to_string());
    let path_context = RequestContext::with_tenant_generated_id(path_tenant_context);

    // Create user and group for path-based tenant
    let path_user_data = json!({
        "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
        "userName": "bob@enterprise.com",
        "name": {"givenName": "Bob", "familyName": "Johnson"}
    });

    let path_user = path_server.create_resource("User", path_user_data, &path_context).await?;
    let path_user_id = path_user.get_id().unwrap();

    let path_group_data = json!({
        "schemas": ["urn:ietf:params:scim:schemas:core:2.0:Group"],
        "displayName": "Enterprise Administrators",
        "members": [{
            "value": path_user_id,
            "type": "User",
            "display": "Bob Johnson"
        }]
    });

    let path_group_json = path_server
        .create_resource_with_refs("Group", path_group_data, &path_context)
        .await?;

    let path_ref_url = path_group_json["members"][0]["$ref"].as_str().unwrap();
    println!("   ✅ Generated $ref: {}", path_ref_url);
    assert_eq!(path_ref_url, format!("https://api.company.com/enterprise/v2/Users/{}", path_user_id));

    println!();

    // Example 4: Error Handling - Missing Tenant
    println!("4. Error Handling Example");
    println!("   Demonstrating what happens when tenant is required but missing");

    // Try to create a group without tenant context on a multi-tenant server
    let no_tenant_context = RequestContext::with_generated_id();
    let error_group_data = json!({
        "schemas": ["urn:ietf:params:scim:schemas:core:2.0:Group"],
        "displayName": "This Should Fail",
        "members": [{
            "value": "some-user-id",
            "type": "User",
            "display": "Test User"
        }]
    });

    let error_result = path_server
        .create_resource_with_refs("Group", error_group_data, &no_tenant_context)
        .await;

    match error_result {
        Err(e) => {
            println!("   ✅ Expected error occurred: {}", e);
            assert!(e.to_string().contains("Tenant ID required"));
        }
        Ok(_) => {
            panic!("Expected an error when tenant is required but missing");
        }
    }

    println!("\n🎉 All examples completed successfully!");
    println!("\nKey Takeaways:");
    println!("• Use ScimServerBuilder for flexible server configuration");
    println!("• Choose the right TenantStrategy for your deployment model");
    println!("• $ref fields are automatically generated based on configuration");
    println!("• Proper error handling when tenant information is missing");
    println!("• Single codebase supports multiple deployment patterns");

    Ok(())
}
