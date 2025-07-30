//! Multi-Tenant SCIM Server Demo
//!
//! This example demonstrates the complete Phase 1 multi-tenant integration,
//! showing how to use all the major components together in a realistic scenario.
//!
//! Run with: cargo run --example multi_tenant_demo

use scim_server::{
    DatabaseResourceProvider, EnhancedRequestContext, IsolationLevel, MultiTenantResourceProvider,
    RequestContext, SingleTenantAdapter, StaticTenantResolver, TenantContext, TenantPermissions,
    TenantResolver,
};
use serde_json::json;
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 Multi-Tenant SCIM Server Demo - Phase 1 Integration");
    println!("{}", "=".repeat(60));

    // Step 1: Set up tenant resolver with multiple tenants
    println!("\n📋 Step 1: Setting up tenant resolver");
    let resolver = setup_tenant_resolver().await;

    // Step 2: Create multi-tenant database provider
    println!("\n🗄️  Step 2: Creating database provider");
    let provider = DatabaseResourceProvider::new_in_memory().await?;

    // Step 3: Demonstrate multi-tenant operations
    println!("\n👥 Step 3: Multi-tenant operations");
    demo_multi_tenant_operations(&resolver, &provider).await?;

    // Step 4: Show tenant isolation
    println!("\n🔒 Step 4: Tenant isolation validation");
    demo_tenant_isolation(&resolver, &provider).await?;

    // Step 5: Demonstrate permission system
    println!("\n🛡️  Step 5: Permission system");
    demo_permission_system(&resolver, &provider).await?;

    // Step 6: Show backward compatibility
    println!("\n🔄 Step 6: Backward compatibility");
    demo_backward_compatibility().await?;

    // Step 7: Performance demonstration
    println!("\n⚡ Step 7: Performance demonstration");
    demo_performance(&provider).await?;

    println!("\n✅ Demo completed successfully!");
    println!("All Phase 1 multi-tenant features are working correctly.");

    Ok(())
}

/// Set up tenant resolver with different tenant configurations
async fn setup_tenant_resolver() -> StaticTenantResolver {
    let resolver = StaticTenantResolver::new();

    // Enterprise tenant with strict isolation
    let enterprise_perms = TenantPermissions {
        can_create: true,
        can_read: true,
        can_update: true,
        can_delete: true,
        can_list: true,
        max_users: Some(1000),
        max_groups: Some(100),
    };

    let enterprise_tenant = TenantContext::new(
        "enterprise-corp".to_string(),
        "enterprise-client-123".to_string(),
    )
    .with_isolation_level(IsolationLevel::Strict)
    .with_permissions(enterprise_perms);

    resolver
        .add_tenant("ent-api-key-secure-123", enterprise_tenant)
        .await;

    // Startup tenant with limited permissions
    let startup_perms = TenantPermissions {
        can_create: true,
        can_read: true,
        can_update: true,
        can_delete: false, // Read-only deletion policy
        can_list: true,
        max_users: Some(50),
        max_groups: Some(10),
    };

    let startup_tenant =
        TenantContext::new("startup-inc".to_string(), "startup-client-456".to_string())
            .with_isolation_level(IsolationLevel::Standard)
            .with_permissions(startup_perms);

    resolver
        .add_tenant("startup-api-key-789", startup_tenant)
        .await;

    // Development tenant with shared resources
    let dev_tenant = TenantContext::new("dev-sandbox".to_string(), "dev-client-dev".to_string())
        .with_isolation_level(IsolationLevel::Shared);

    resolver.add_tenant("dev-api-key-test", dev_tenant).await;

    println!("✅ Configured 3 tenants:");
    println!("   - enterprise-corp: Strict isolation, 1000 user limit");
    println!("   - startup-inc: Standard isolation, 50 user limit, no delete");
    println!("   - dev-sandbox: Shared isolation, unlimited");

    resolver
}

/// Demonstrate basic multi-tenant operations
async fn demo_multi_tenant_operations(
    resolver: &StaticTenantResolver,
    provider: &DatabaseResourceProvider<scim_server::InMemoryDatabase>,
) -> Result<(), Box<dyn std::error::Error>> {
    // Resolve enterprise tenant
    let enterprise_context = resolver.resolve_tenant("ent-api-key-secure-123").await?;
    let ent_context = EnhancedRequestContext::with_generated_id(enterprise_context);

    println!("📝 Creating users for enterprise-corp tenant...");

    // Create enterprise users
    let ceo_data = json!({
        "userName": "john.ceo",
        "displayName": "John Smith (CEO)",
        "emails": [{"value": "john@enterprise-corp.com", "primary": true}],
        "title": "Chief Executive Officer"
    });

    let ceo = provider
        .create_resource("enterprise-corp", "User", ceo_data, &ent_context)
        .await?;
    println!(
        "   ✅ Created CEO: {} (ID: {})",
        ceo.get_username().unwrap(),
        ceo.get_id().unwrap()
    );

    let cto_data = json!({
        "userName": "jane.cto",
        "displayName": "Jane Doe (CTO)",
        "emails": [{"value": "jane@enterprise-corp.com", "primary": true}],
        "title": "Chief Technology Officer"
    });

    let _cto = provider
        .create_resource("enterprise-corp", "User", cto_data, &ent_context)
        .await?;
    println!("   ✅ Created CTO: jane.cto");

    // Create a group
    let exec_group_data = json!({
        "displayName": "Executive Team",
        "description": "C-level executives"
    });

    let _exec_group = provider
        .create_resource("enterprise-corp", "Group", exec_group_data, &ent_context)
        .await?;
    println!("   ✅ Created group: Executive Team");

    // Show resource counts
    let user_count = provider
        .get_resource_count("enterprise-corp", "User", &ent_context)
        .await?;
    let group_count = provider
        .get_resource_count("enterprise-corp", "Group", &ent_context)
        .await?;

    println!(
        "📊 Enterprise tenant stats: {} users, {} groups",
        user_count, group_count
    );

    // Now do the same for startup tenant
    let startup_context = resolver.resolve_tenant("startup-api-key-789").await?;
    let startup_ctx = EnhancedRequestContext::with_generated_id(startup_context);

    println!("\n📝 Creating users for startup-inc tenant...");

    let founder_data = json!({
        "userName": "alice.founder",
        "displayName": "Alice Johnson (Founder)",
        "emails": [{"value": "alice@startup-inc.com", "primary": true}]
    });

    let _founder = provider
        .create_resource("startup-inc", "User", founder_data, &startup_ctx)
        .await?;
    println!("   ✅ Created founder: alice.founder");

    let startup_user_count = provider
        .get_resource_count("startup-inc", "User", &startup_ctx)
        .await?;
    println!("📊 Startup tenant stats: {} users", startup_user_count);

    Ok(())
}

/// Demonstrate tenant isolation - tenants cannot access each other's data
async fn demo_tenant_isolation(
    resolver: &StaticTenantResolver,
    provider: &DatabaseResourceProvider<scim_server::InMemoryDatabase>,
) -> Result<(), Box<dyn std::error::Error>> {
    let enterprise_context = resolver.resolve_tenant("ent-api-key-secure-123").await?;
    let ent_context = EnhancedRequestContext::with_generated_id(enterprise_context);

    let startup_context = resolver.resolve_tenant("startup-api-key-789").await?;
    let startup_ctx = EnhancedRequestContext::with_generated_id(startup_context);

    println!("🔍 Testing cross-tenant access prevention...");

    // Enterprise tenant trying to access startup tenant's data (should fail)
    let cross_access_result = provider
        .list_resources("startup-inc", "User", None, &ent_context)
        .await;

    match cross_access_result {
        Err(_) => println!("   ✅ Cross-tenant access correctly blocked"),
        Ok(_) => println!("   ❌ ERROR: Cross-tenant access was allowed!"),
    }

    // Test invalid tenant access
    let invalid_result = provider
        .get_resource("nonexistent-tenant", "User", "any-id", &ent_context)
        .await;

    match invalid_result {
        Err(_) => println!("   ✅ Invalid tenant access correctly blocked"),
        Ok(_) => println!("   ❌ ERROR: Invalid tenant access was allowed!"),
    }

    // Verify each tenant can only see their own data
    let ent_users = provider
        .list_resources("enterprise-corp", "User", None, &ent_context)
        .await?;
    let startup_users = provider
        .list_resources("startup-inc", "User", None, &startup_ctx)
        .await?;

    println!(
        "   ✅ Enterprise tenant sees {} users (their own)",
        ent_users.len()
    );
    println!(
        "   ✅ Startup tenant sees {} users (their own)",
        startup_users.len()
    );

    Ok(())
}

/// Demonstrate permission system and limits
async fn demo_permission_system(
    resolver: &StaticTenantResolver,
    provider: &DatabaseResourceProvider<scim_server::InMemoryDatabase>,
) -> Result<(), Box<dyn std::error::Error>> {
    let startup_context = resolver.resolve_tenant("startup-api-key-789").await?;
    let startup_ctx = EnhancedRequestContext::with_generated_id(startup_context);

    println!("🚫 Testing permission restrictions...");

    // Create a test user first
    let test_user_data = json!({
        "id": "test-user-123",
        "userName": "test.user",
        "displayName": "Test User"
    });

    let test_user = provider
        .create_resource("startup-inc", "User", test_user_data, &startup_ctx)
        .await?;
    println!("   ✅ Created test user for permission testing");

    // Try to delete (startup tenant has delete disabled)
    let delete_result = provider
        .delete_resource(
            "startup-inc",
            "User",
            test_user.get_id().unwrap(),
            &startup_ctx,
        )
        .await;

    match delete_result {
        Err(_) => println!("   ✅ Delete operation correctly blocked by permissions"),
        Ok(_) => println!("   ❌ ERROR: Delete operation was allowed despite restrictions!"),
    }

    // Test user limit (startup has 50 user limit)
    println!("   Testing user creation limits...");
    let mut created_count = provider
        .get_resource_count("startup-inc", "User", &startup_ctx)
        .await?;

    // Try to create users up to the limit
    for i in created_count..std::cmp::min(created_count + 5, 50) {
        let user_data = json!({
            "userName": format!("test.user.{}", i),
            "displayName": format!("Test User {}", i)
        });

        let result = provider
            .create_resource("startup-inc", "User", user_data, &startup_ctx)
            .await;
        match result {
            Ok(_) => created_count += 1,
            Err(_) => break,
        }
    }

    println!("   ✅ Created {} users within limit", created_count);

    Ok(())
}

/// Demonstrate backward compatibility with existing single-tenant code
async fn demo_backward_compatibility() -> Result<(), Box<dyn std::error::Error>> {
    println!("🔄 Testing backward compatibility...");

    // Old-style RequestContext still works
    let old_context = RequestContext::new("legacy-request-123".to_string());
    println!("   ✅ Legacy RequestContext: {}", old_context.request_id);
    println!("   ✅ Is multi-tenant: {}", old_context.is_multi_tenant());

    // Can enhance existing context with tenant information
    let tenant_context = TenantContext::new("compat-tenant".to_string(), "client".to_string());
    let enhanced_context =
        RequestContext::with_tenant("enhanced-request".to_string(), tenant_context);

    println!(
        "   ✅ Enhanced context tenant: {}",
        enhanced_context.tenant_id().unwrap()
    );

    // Conversion between context types
    let converted: Result<EnhancedRequestContext, _> = enhanced_context.try_into();
    match converted {
        Ok(ctx) => println!("   ✅ Context conversion successful: {}", ctx.tenant_id()),
        Err(_) => println!("   ❌ Context conversion failed"),
    }

    Ok(())
}

/// Demonstrate performance with multiple tenants and resources
async fn demo_performance(
    provider: &DatabaseResourceProvider<scim_server::InMemoryDatabase>,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("⚡ Performance demonstration with multiple tenants...");

    let tenant_count = 3;
    let users_per_tenant = 10;

    let start_time = std::time::Instant::now();

    // Create multiple tenants with users sequentially (since we can't clone the provider)
    let mut total_created = 0;

    for tenant_idx in 0..tenant_count {
        let tenant_id = format!("perf-tenant-{}", tenant_idx);
        let tenant_context = TenantContext::new(tenant_id.clone(), "perf-client".to_string());
        let context = EnhancedRequestContext::with_generated_id(tenant_context);

        let mut created = 0;
        for user_idx in 0..users_per_tenant {
            let user_data = json!({
                "userName": format!("perfuser{}_{}", tenant_idx, user_idx),
                "displayName": format!("Performance User {} from Tenant {}", user_idx, tenant_idx),
                "emails": [{"value": format!("user{}@perf-tenant-{}.com", user_idx, tenant_idx), "primary": true}]
            });

            match provider
                .create_resource(&tenant_id, "User", user_data, &context)
                .await
            {
                Ok(_) => created += 1,
                Err(e) => eprintln!("Failed to create user: {}", e),
            }
        }

        total_created += created;
        println!("   ✅ {}: {} users created", tenant_id, created);
    }

    let duration = start_time.elapsed();

    println!("📊 Performance Results:");
    println!("   - Total tenants: {}", tenant_count);
    println!("   - Total users created: {}", total_created);
    println!("   - Time taken: {:?}", duration);
    println!(
        "   - Users per second: {:.2}",
        total_created as f64 / duration.as_secs_f64()
    );

    // Get overall statistics
    let stats = provider.get_stats().await;
    println!("📈 Database Statistics:");
    println!("   - Active tenants: {}", stats.tenant_count);
    println!("   - Total resources: {}", stats.total_resources);
    println!(
        "   - Users: {}",
        stats.resource_counts.get("User").unwrap_or(&0)
    );
    println!(
        "   - Groups: {}",
        stats.resource_counts.get("Group").unwrap_or(&0)
    );

    Ok(())
}

/// Helper function to create a demo single-tenant provider for adapter testing
#[allow(dead_code)]
async fn demo_single_tenant_adapter() -> Result<(), Box<dyn std::error::Error>> {
    use scim_server::{Resource, ResourceProvider};
    use std::collections::HashMap;
    use tokio::sync::RwLock;

    // Mock single-tenant provider
    struct MockProvider {
        resources: Arc<RwLock<HashMap<String, HashMap<String, Resource>>>>,
    }

    impl MockProvider {
        fn new() -> Self {
            Self {
                resources: Arc::new(RwLock::new(HashMap::new())),
            }
        }
    }

    #[derive(Debug, thiserror::Error)]
    #[error("Mock error")]
    struct MockError;

    impl ResourceProvider for MockProvider {
        type Error = MockError;

        async fn create_resource(
            &self,
            resource_type: &str,
            data: serde_json::Value,
            _context: &RequestContext,
        ) -> Result<Resource, Self::Error> {
            let resource = Resource::new(resource_type.to_string(), data);
            let id = resource.get_id().unwrap_or("generated").to_string();

            let mut resources = self.resources.write().await;
            resources
                .entry(resource_type.to_string())
                .or_insert_with(HashMap::new)
                .insert(id, resource.clone());

            Ok(resource)
        }

        async fn get_resource(
            &self,
            resource_type: &str,
            id: &str,
            _context: &RequestContext,
        ) -> Result<Option<Resource>, Self::Error> {
            let resources = self.resources.read().await;
            Ok(resources
                .get(resource_type)
                .and_then(|r| r.get(id))
                .cloned())
        }

        async fn update_resource(
            &self,
            resource_type: &str,
            id: &str,
            data: serde_json::Value,
            _context: &RequestContext,
        ) -> Result<Resource, Self::Error> {
            let resource = Resource::new(resource_type.to_string(), data);
            let mut resources = self.resources.write().await;
            resources
                .entry(resource_type.to_string())
                .or_insert_with(HashMap::new)
                .insert(id.to_string(), resource.clone());
            Ok(resource)
        }

        async fn delete_resource(
            &self,
            resource_type: &str,
            id: &str,
            _context: &RequestContext,
        ) -> Result<(), Self::Error> {
            let mut resources = self.resources.write().await;
            if let Some(type_resources) = resources.get_mut(resource_type) {
                type_resources.remove(id);
            }
            Ok(())
        }

        async fn list_resources(
            &self,
            resource_type: &str,
            _query: Option<&scim_server::ListQuery>,
            _context: &RequestContext,
        ) -> Result<Vec<Resource>, Self::Error> {
            let resources = self.resources.read().await;
            Ok(resources
                .get(resource_type)
                .map(|r| r.values().cloned().collect())
                .unwrap_or_default())
        }

        async fn find_resource_by_attribute(
            &self,
            resource_type: &str,
            attribute: &str,
            value: &serde_json::Value,
            _context: &RequestContext,
        ) -> Result<Option<Resource>, Self::Error> {
            let resources = self.resources.read().await;
            Ok(resources.get(resource_type).and_then(|type_resources| {
                type_resources
                    .values()
                    .find(|resource| resource.get_attribute(attribute) == Some(value))
                    .cloned()
            }))
        }

        async fn resource_exists(
            &self,
            resource_type: &str,
            id: &str,
            _context: &RequestContext,
        ) -> Result<bool, Self::Error> {
            let resources = self.resources.read().await;
            Ok(resources
                .get(resource_type)
                .map(|r| r.contains_key(id))
                .unwrap_or(false))
        }
    }

    println!("🔄 Testing single-tenant adapter...");

    // Create single-tenant provider
    let single_provider = Arc::new(MockProvider::new());

    // Wrap it with multi-tenant adapter
    let multi_provider = SingleTenantAdapter::new(single_provider);

    // Use it with multi-tenant context
    let tenant_context = TenantContext::new("adapted-tenant".to_string(), "client".to_string());
    let context = EnhancedRequestContext::with_generated_id(tenant_context);

    let user_data = json!({
        "id": "adapter-user",
        "userName": "adapted.user",
        "displayName": "Adapted User"
    });

    let _user = multi_provider
        .create_resource("adapted-tenant", "User", user_data, &context)
        .await?;
    println!("   ✅ Single-tenant provider successfully adapted for multi-tenant use");

    Ok(())
}
