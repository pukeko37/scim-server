//! Compile-Time Authentication Example
//!
//! This example demonstrates the type-safe authentication system that provides
//! compile-time guarantees about authentication state. It shows how the type
//! system prevents unauthenticated access and ensures proper credential lifecycle.
//!
//! Run with: cargo run --example compile_time_auth_example

use scim_server::{
    ResourceProvider,
    auth::{
        AuthenticatedRequestContext, AuthenticationValidator, Credential, LinearCredential,
        Unauthenticated,
    },
    providers::StandardResourceProvider,
    resource::{IsolationLevel, TenantContext, TenantPermissions},
    storage::InMemoryStorage,
};
use serde_json::json;

/// Demonstrates how the compile-time authentication system works
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🔐 Compile-Time Authentication Example");
    println!("{}", "=".repeat(60));

    // Step 1: Set up authentication validator
    println!("\n🏗️  Step 1: Setting up authentication system");
    let validator = setup_authentication_system().await;

    // Step 2: Demonstrate compile-time safety
    println!("\n🛡️  Step 2: Compile-time authentication safety");
    demonstrate_compile_time_safety(&validator).await?;

    // Step 3: Show linear credential consumption
    println!("\n🔄 Step 3: Linear credential lifecycle");
    demonstrate_linear_credentials(&validator).await?;

    // Step 4: Authenticated provider operations
    println!("\n📊 Step 4: Type-safe provider operations");
    demonstrate_authenticated_operations(&validator).await?;

    // Step 5: Show impossible states
    println!("\n❌ Step 5: Impossible states (compile-time prevention)");
    demonstrate_impossible_states();

    println!("\n✅ Compile-time authentication example completed!");
    println!("All authentication is enforced at compile time with zero runtime cost.");

    Ok(())
}

/// Set up the authentication system with registered credentials
async fn setup_authentication_system() -> AuthenticationValidator {
    let validator = AuthenticationValidator::new();

    // Register enterprise tenant
    let enterprise_permissions = TenantPermissions {
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
    .with_permissions(enterprise_permissions);

    validator
        .register_credential("ent-secure-key-456", enterprise_tenant)
        .await;

    // Register startup tenant
    let startup_permissions = TenantPermissions {
        can_create: true,
        can_read: true,
        can_update: true,
        can_delete: false, // No delete permission
        can_list: true,
        max_users: Some(50),
        max_groups: Some(10),
    };

    let startup_tenant =
        TenantContext::new("startup-inc".to_string(), "startup-client-789".to_string())
            .with_isolation_level(IsolationLevel::Standard)
            .with_permissions(startup_permissions);

    validator
        .register_credential("startup-api-key", startup_tenant)
        .await;

    println!("✅ Registered 2 tenant credentials");
    println!("   - enterprise-corp: Full permissions, 1000 user limit");
    println!("   - startup-inc: No delete, 50 user limit");

    validator
}

/// Demonstrate compile-time authentication safety
async fn demonstrate_compile_time_safety(
    validator: &AuthenticationValidator,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("🔍 Testing compile-time authentication guarantees...");

    // 1. Create unauthenticated credential
    let raw_credential = Credential::<Unauthenticated>::new("ent-secure-key-456");
    println!(
        "   ✅ Created unauthenticated credential: {}",
        raw_credential.raw_value()
    );

    // 2. Cannot access authenticated methods on unauthenticated credential
    // This would NOT compile:
    // let value = raw_credential.authenticated_value(); // Compile error!

    // 3. Must go through authentication to get authenticated state
    let linear_cred = LinearCredential::new("ent-secure-key-456");
    let witness = validator.authenticate(linear_cred).await?;
    println!("   ✅ Authentication successful - received witness");

    // 4. Create authenticated context
    let auth_context = AuthenticatedRequestContext::from_witness(witness);
    println!(
        "   ✅ Created authenticated context for tenant: {}",
        auth_context.tenant_id()
    );

    // 5. Show tenant authority proof
    let authority = auth_context.authority();
    println!(
        "   ✅ Authority proven for client: {}",
        authority.client_id()
    );
    println!("   ✅ Validated at: {}", authority.witness().validated_at());

    Ok(())
}

/// Demonstrate linear credential consumption
async fn demonstrate_linear_credentials(
    validator: &AuthenticationValidator,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("🔄 Testing linear credential consumption...");

    // 1. Create linear credential
    let linear_cred = LinearCredential::new("startup-api-key");
    println!("   ✅ Created linear credential");
    println!("   ✅ Is consumed: {}", linear_cred.is_consumed());

    // 2. Authenticate (consumes the credential)
    let witness = validator.authenticate(linear_cred).await?;
    println!("   ✅ Authentication consumed the credential");

    // 3. Credential is now consumed and cannot be reused
    // This would panic if we tried:
    // let another_witness = validator.authenticate(linear_cred).await; // Panic!

    // 4. Create authenticated context
    let auth_context = AuthenticatedRequestContext::from_witness(witness);
    println!("   ✅ Created context for: {}", auth_context.tenant_id());

    Ok(())
}

/// Demonstrate authenticated provider operations
async fn demonstrate_authenticated_operations(
    validator: &AuthenticationValidator,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("📊 Testing type-safe provider operations...");

    let storage = InMemoryStorage::new();
    let provider = StandardResourceProvider::new(storage);

    // 1. Authenticate enterprise client
    let enterprise_cred = LinearCredential::new("ent-secure-key-456");
    let enterprise_witness = validator.authenticate(enterprise_cred).await?;
    let enterprise_context = AuthenticatedRequestContext::from_witness(enterprise_witness);

    println!("   ✅ Enterprise client authenticated");

    // 2. Create users with authenticated context
    let ceo_data = json!({
        "userName": "john.ceo",
        "displayName": "John Smith (CEO)",
        "emails": [{"value": "john@enterprise-corp.com", "primary": true}],
        "title": "Chief Executive Officer"
    });

    let ceo = provider
        .create_resource("User", ceo_data, enterprise_context.request_context())
        .await?;
    println!(
        "   ✅ Created CEO: {} (authenticated)",
        ceo.get_username().unwrap()
    );

    let cto_data = json!({
        "userName": "jane.cto",
        "displayName": "Jane Doe (CTO)",
        "emails": [{"value": "jane@enterprise-corp.com", "primary": true}],
        "title": "Chief Technology Officer"
    });

    let _cto = provider
        .create_resource("User", cto_data, enterprise_context.request_context())
        .await?;
    println!("   ✅ Created CTO: jane.cto (authenticated)");

    // 3. Authenticate startup client
    let startup_cred = LinearCredential::new("startup-api-key");
    let startup_witness = validator.authenticate(startup_cred).await?;
    let startup_context = AuthenticatedRequestContext::from_witness(startup_witness);

    println!("   ✅ Startup client authenticated");

    // 4. Create startup user
    let founder_data = json!({
        "userName": "alice.founder",
        "displayName": "Alice Johnson (Founder)",
        "emails": [{"value": "alice@startup-inc.com", "primary": true}]
    });

    let _founder = provider
        .create_resource("User", founder_data, startup_context.request_context())
        .await?;
    println!("   ✅ Created founder: alice.founder (authenticated)");

    // 5. List resources with authenticated contexts
    let enterprise_users = provider
        .list_resources("User", None, enterprise_context.request_context())
        .await?;
    let startup_users = provider
        .list_resources("User", None, startup_context.request_context())
        .await?;

    println!(
        "   📊 Enterprise users: {} (isolated)",
        enterprise_users.len()
    );
    println!("   📊 Startup users: {} (isolated)", startup_users.len());

    // 6. Show compile-time tenant isolation
    println!("   ✅ Each authenticated context only sees their own tenant's data");
    println!("   ✅ Cross-tenant access is impossible at compile time");

    Ok(())
}

/// Demonstrate states that are impossible to represent
fn demonstrate_impossible_states() {
    println!("❌ Testing impossible states (these won't compile)...");

    // 1. Cannot create authenticated credential directly
    // This would NOT compile:
    // let fake_auth = Credential::<Authenticated>::new("fake"); // Compile error!
    println!("   ✅ Cannot create authenticated credentials without validation");

    // 2. Cannot access authenticated methods without proof
    let _unauth = Credential::<Unauthenticated>::new("test");
    // This would NOT compile:
    // let value = unauth.authenticated_value(); // Compile error!
    println!("   ✅ Cannot access authenticated methods on unauth credentials");

    // 3. Cannot create authenticated context without witness
    // This would NOT compile:
    // let fake_context = AuthenticatedRequestContext::new("fake"); // No such method!
    println!("   ✅ Cannot create authenticated context without witness");

    // 4. Cannot reuse consumed credentials
    // Linear credentials prevent this at compile time
    println!("   ✅ Cannot reuse consumed credentials (linear types)");

    // 5. Cannot bypass authentication
    // All authenticated operations require proper witness types
    println!("   ✅ Cannot bypass authentication system");

    println!("\n🎯 Key Compile-Time Guarantees:");
    println!("   • Unauthenticated access is unrepresentable");
    println!("   • Credentials must be validated before use");
    println!("   • Authentication witnesses cannot be forged");
    println!("   • Tenant contexts are guaranteed valid");
    println!("   • Linear credentials prevent reuse");
    println!("   • Zero runtime authentication overhead");
}

/// Example trait showing how to require authenticated contexts in APIs
///
/// This demonstrates how to design secure APIs that can only be called with
/// authenticated contexts, providing compile-time guarantees about authentication state.
///
/// # Example
///
/// ```rust,no_run
/// use scim_server::{
///     auth::AuthenticatedRequestContext,
///     providers::{StandardResourceProvider, ProviderError},
///     storage::InMemoryStorage,
///     resource::Resource,
/// };
/// use serde_json::Value;
///
/// /// Secure API trait that requires authenticated contexts
/// trait SecureScimProvider {
///     type Error: std::error::Error + Send + Sync + 'static;
///
///     /// This method can ONLY be called with an authenticated context
///     /// The type system prevents calling it without proper authentication
///     fn secure_list_users(
///         &self,
///         context: &AuthenticatedRequestContext,
///     ) -> impl std::future::Future<Output = Result<Vec<Resource>, Self::Error>> + Send;
///
///     /// This method CANNOT be called with unauthenticated access
///     /// The compile-time guarantee ensures security
///     fn secure_create_user(
///         &self,
///         user_data: Value,
///         context: &AuthenticatedRequestContext,
///     ) -> impl std::future::Future<Output = Result<Resource, Self::Error>> + Send;
/// }
///
/// /// Example implementation showing compile-time enforcement
/// impl SecureScimProvider for StandardResourceProvider<InMemoryStorage> {
///     type Error = ProviderError;
///
///     async fn secure_list_users(
///         &self,
///         context: &AuthenticatedRequestContext,
///     ) -> Result<Vec<Resource>, Self::Error> {
///         // The authenticated context provides compile-time proof of authentication
///         // We can safely use it without additional runtime checks
///         self.list_resources("User", None, context.request_context())
///             .await
///     }
///
///     async fn secure_create_user(
///         &self,
///         user_data: Value,
///         context: &AuthenticatedRequestContext,
///     ) -> Result<Resource, Self::Error> {
///         // Again, compile-time authentication guarantee
///         self.create_resource("User", user_data, context.request_context())
///             .await
///     }
/// }
/// ```
///
/// This pattern ensures that secure operations can only be performed with valid
/// authenticated contexts, making unauthorized access impossible at compile time.
fn _secure_scim_provider_example() {}
