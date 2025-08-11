//! ETag Concurrency Control Example
//!
//! This example demonstrates the built-in ETag concurrency control features
//! of the SCIM server library. It shows how to use conditional operations
//! to prevent lost updates and handle version conflicts.

use scim_server::{
    ScimServer,
    multi_tenant::ScimOperation,
    operation_handler::{ScimOperationHandler, ScimOperationRequest},
    providers::InMemoryProvider,
    resource::{
        RequestContext, ResourceProvider,
        conditional_provider::VersionedResource,
        version::{ConditionalResult, ScimVersion},
    },
    resource_handlers::create_user_resource_handler,
};
use serde_json::json;

use tokio::time::{Duration, sleep};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    env_logger::init();

    println!("🏷️  SCIM ETag Concurrency Control Example");
    println!("=========================================\n");

    // 1. Setup server with InMemoryProvider
    let provider = InMemoryProvider::new();
    let mut server = ScimServer::new(provider)?;

    // Register User resource type
    let user_schema = server
        .get_schema_by_id("urn:ietf:params:scim:schemas:core:2.0:User")
        .expect("User schema should be available")
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
        ],
    )?;

    let handler = ScimOperationHandler::new(server);
    let _context = RequestContext::with_generated_id();

    println!("✅ Server initialized with ETag support\n");

    // === BASIC VERSION MANAGEMENT ===
    println!("📋 BASIC VERSION MANAGEMENT");
    println!("===========================");

    // Create a user
    let create_request = ScimOperationRequest::create(
        "User",
        json!({
            "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
            "userName": "alice.smith",
            "name": {
                "familyName": "Smith",
                "givenName": "Alice",
                "formatted": "Alice Smith"
            },
            "emails": [
                {
                    "value": "alice.smith@example.com",
                    "type": "work",
                    "primary": true
                }
            ],
            "active": true
        }),
    );

    let create_response = handler.handle_operation(create_request).await;
    let user_id = if create_response.success {
        let user_id = create_response.metadata.resource_id.clone().unwrap();
        let etag = create_response.metadata.additional.get("etag").unwrap();

        println!("✅ Created user with ID: {}", user_id);
        println!("   ETag (weak): {}", etag.as_str().unwrap());
        user_id
    } else {
        panic!("Failed to create user: {:?}", create_response.error);
    };

    // Get user to see version information
    let get_request = ScimOperationRequest::get("User", &user_id);
    let get_response = handler.handle_operation(get_request).await;

    let current_version = if get_response.success {
        let etag = get_response.metadata.additional.get("etag").unwrap();

        println!("✅ Retrieved user successfully");
        println!("   Current ETag (weak): {}", etag.as_str().unwrap());

        ScimVersion::parse_http_header(etag.as_str().unwrap())?
    } else {
        panic!("Failed to retrieve user: {:?}", get_response.error);
    };

    println!();

    // === CONDITIONAL UPDATE SUCCESS ===
    println!("✅ CONDITIONAL UPDATE SUCCESS");
    println!("=============================");

    // Update with correct version (should succeed)
    let conditional_update_request = ScimOperationRequest::update(
        "User",
        &user_id,
        json!({
            "id": user_id,
            "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
            "userName": "alice.smith",
            "name": {
                "familyName": "Smith",
                "givenName": "Alice",
                "formatted": "Alice M. Smith" // Changed middle initial
            },
            "emails": [
                {
                    "value": "alice.smith@newcompany.com", // Changed email
                    "type": "work",
                    "primary": true
                }
            ],
            "active": true
        }),
    )
    .with_expected_version(current_version.clone());

    let conditional_update_response = handler.handle_operation(conditional_update_request).await;

    let _new_version = if conditional_update_response.success {
        let old_etag = current_version.to_http_header();
        let new_etag = conditional_update_response
            .metadata
            .additional
            .get("etag")
            .unwrap();

        println!("✅ Conditional update succeeded!");
        println!("   Old ETag (weak): {}", old_etag);
        println!("   New ETag (weak): {}", new_etag.as_str().unwrap());

        ScimVersion::parse_http_header(new_etag.as_str().unwrap())?
    } else {
        panic!(
            "Conditional update should have succeeded: {:?}",
            conditional_update_response.error
        );
    };

    println!();

    // === CONDITIONAL UPDATE CONFLICT ===
    println!("⚠️  CONDITIONAL UPDATE CONFLICT");
    println!("==============================");

    // Try to update with old version (should fail)
    let conflicting_update_request = ScimOperationRequest::update(
        "User",
        &user_id,
        json!({
            "id": user_id,
            "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
            "userName": "alice.smith",
            "name": {
                "familyName": "Smith-Jones", // Different change
                "givenName": "Alice",
                "formatted": "Alice Smith-Jones"
            },
            "active": false // Different change
        }),
    )
    .with_expected_version(current_version); // Using old version

    let conflicting_response = handler.handle_operation(conflicting_update_request).await;

    if !conflicting_response.success {
        println!("✅ Version conflict detected correctly!");
        println!("   Error: {}", conflicting_response.error.unwrap());
        println!(
            "   Error Code: {}",
            conflicting_response.error_code.unwrap()
        );
    } else {
        panic!("Conditional update should have failed due to version mismatch");
    }

    println!();

    // === PROVIDER-LEVEL CONDITIONAL OPERATIONS ===
    println!("🔧 PROVIDER-LEVEL CONDITIONAL OPERATIONS");
    println!("=========================================");

    // Create a new provider instance to demonstrate provider-level operations
    let provider = InMemoryProvider::new();
    let mut provider_server = ScimServer::new(provider.clone())?;

    // Register User resource type for the provider demo
    let user_schema = provider_server
        .get_schema_by_id("urn:ietf:params:scim:schemas:core:2.0:User")
        .expect("User schema should be available")
        .clone();
    let user_handler = create_user_resource_handler(user_schema);
    provider_server.register_resource_type(
        "User",
        user_handler,
        vec![
            ScimOperation::Create,
            ScimOperation::Read,
            ScimOperation::Update,
            ScimOperation::Delete,
        ],
    )?;

    // Create a user for provider-level testing
    let provider_user_data = json!({
        "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
        "userName": "provider.test",
        "active": true
    });

    let provider_context = RequestContext::with_generated_id();
    let created_resource = provider
        .create_resource("User", provider_user_data, &provider_context)
        .await?;
    let provider_user_id = created_resource.get_id().unwrap();

    // Get current resource
    let current_resource = provider
        .get_resource("User", &provider_user_id, &provider_context)
        .await?
        .expect("User should exist");

    let versioned_current = VersionedResource::new(current_resource);
    println!(
        "✅ Current resource ETag (weak): {}",
        versioned_current.version().to_http_header()
    );

    // Successful conditional update
    let update_data = json!({
        "id": provider_user_id,
        "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
        "userName": "provider.test",
        "name": {
            "familyName": "Test",
            "givenName": "Provider",
            "formatted": "Dr. Provider Test" // Added title
        },
        "active": true
    });

    match provider
        .conditional_update(
            "User",
            &provider_user_id,
            update_data,
            versioned_current.version(),
            &provider_context,
        )
        .await?
    {
        ConditionalResult::Success(updated_versioned) => {
            println!("✅ Provider conditional update succeeded!");
            println!(
                "   New ETag (weak): {}",
                updated_versioned.version().to_http_header()
            );
        }
        ConditionalResult::VersionMismatch(conflict) => {
            println!("❌ Unexpected version conflict: {}", conflict.message);
        }
        ConditionalResult::NotFound => {
            println!("❌ Resource not found");
        }
    }

    // Try update with wrong version
    let wrong_version = ScimVersion::from_hash("wrong-version");
    let failing_update_data = json!({
        "id": provider_user_id,
        "userName": "should.fail",
        "active": false
    });

    match provider
        .conditional_update(
            "User",
            &provider_user_id,
            failing_update_data,
            &wrong_version,
            &provider_context,
        )
        .await?
    {
        ConditionalResult::Success(_) => {
            println!("❌ Update should have failed!");
        }
        ConditionalResult::VersionMismatch(conflict) => {
            println!("✅ Provider correctly detected version mismatch!");
            println!("   Expected: {}", conflict.expected);
            println!("   Current: {}", conflict.current);
            println!("   Message: {}", conflict.message);
        }
        ConditionalResult::NotFound => {
            println!("❌ Resource not found");
        }
    }

    println!();

    // === CONDITIONAL DELETE ===
    println!("🗑️  CONDITIONAL DELETE");
    println!("======================");

    // Get current version for delete
    let current_resource = provider
        .get_resource("User", &provider_user_id, &provider_context)
        .await?
        .expect("User should exist");

    let versioned_for_delete = VersionedResource::new(current_resource);
    println!(
        "✅ Resource ETag for delete (weak): {}",
        versioned_for_delete.version().to_http_header()
    );

    // Try delete with wrong version first
    let wrong_delete_version = ScimVersion::from_hash("wrong-delete-version");

    match provider
        .conditional_delete(
            "User",
            &provider_user_id,
            &wrong_delete_version,
            &provider_context,
        )
        .await?
    {
        ConditionalResult::Success(()) => {
            println!("❌ Delete should have failed!");
        }
        ConditionalResult::VersionMismatch(conflict) => {
            println!("✅ Delete correctly rejected due to version mismatch!");
            println!("   Conflict: {}", conflict.message);
        }
        ConditionalResult::NotFound => {
            println!("❌ Resource not found");
        }
    }

    // Now delete with correct version
    match provider
        .conditional_delete(
            "User",
            &provider_user_id,
            versioned_for_delete.version(),
            &provider_context,
        )
        .await?
    {
        ConditionalResult::Success(()) => {
            println!("✅ Conditional delete succeeded!");
        }
        ConditionalResult::VersionMismatch(conflict) => {
            println!("❌ Unexpected version conflict: {}", conflict.message);
        }
        ConditionalResult::NotFound => {
            println!("❌ Resource not found");
        }
    }

    // Verify deletion
    let verify_resource = provider
        .get_resource("User", &provider_user_id, &provider_context)
        .await?;
    if verify_resource.is_none() {
        println!("✅ Resource successfully deleted");
    } else {
        println!("❌ Resource still exists after delete");
    }

    println!();

    // === VERSION COMPUTATION EXAMPLES ===
    println!("🔢 VERSION COMPUTATION EXAMPLES");
    println!("===============================");

    // Show how versions are computed from content
    let test_resource1 = json!({
        "id": "test-123",
        "userName": "test.user",
        "active": true
    });

    let test_resource2 = json!({
        "id": "test-123",
        "userName": "test.user",
        "active": false  // Only this field changed
    });

    let version1 = ScimVersion::from_content(test_resource1.to_string().as_bytes());
    let version2 = ScimVersion::from_content(test_resource2.to_string().as_bytes());

    println!("✅ Content-based version computation:");
    println!("   Resource 1 ETag (weak): {}", version1.to_http_header());
    println!("   Resource 2 ETag (weak): {}", version2.to_http_header());
    println!("   Versions match: {}", version1.matches(&version2));

    // Show identical content produces identical versions
    let test_resource1_copy = json!({
        "id": "test-123",
        "userName": "test.user",
        "active": true
    });

    let version1_copy = ScimVersion::from_content(test_resource1_copy.to_string().as_bytes());
    println!(
        "   Identical content versions match: {}",
        version1.matches(&version1_copy)
    );

    println!();

    // === CONCURRENT MODIFICATION SIMULATION ===
    println!("🏃 CONCURRENT MODIFICATION SIMULATION");
    println!("====================================");

    // Create a new user for this demo
    let create_request = ScimOperationRequest::create(
        "User",
        json!({
            "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
            "userName": "concurrent.test",
            "active": true
        }),
    );

    let create_response = handler.handle_operation(create_request).await;
    let concurrent_user_id = create_response.metadata.resource_id.clone().unwrap();

    // Get initial version
    let get_request = ScimOperationRequest::get("User", &concurrent_user_id);
    let get_response = handler.handle_operation(get_request).await;
    let initial_etag = get_response
        .metadata
        .additional
        .get("etag")
        .unwrap()
        .as_str()
        .unwrap();
    let initial_version = ScimVersion::parse_http_header(initial_etag)?;

    println!(
        "✅ Created user for concurrent test: {}",
        concurrent_user_id
    );
    println!("   Initial ETag (weak): {}", initial_etag);

    // Simulate Client A getting the resource
    let client_a_version = initial_version.clone();
    println!(
        "👤 Client A has ETag (weak): {}",
        client_a_version.to_http_header()
    );

    // Simulate Client B getting the resource
    let client_b_version = initial_version.clone();
    println!(
        "👤 Client B has ETag (weak): {}",
        client_b_version.to_http_header()
    );

    // Client A successfully updates
    let client_a_update = ScimOperationRequest::update(
        "User",
        &concurrent_user_id,
        json!({
            "id": concurrent_user_id,
            "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
            "userName": "concurrent.test",
            "name": {
                "givenName": "Client",
                "familyName": "A"
            },
            "active": true
        }),
    )
    .with_expected_version(client_a_version);

    let client_a_response = handler.handle_operation(client_a_update).await;

    if client_a_response.success {
        let new_etag = client_a_response.metadata.additional.get("etag").unwrap();
        println!("✅ Client A update succeeded!");
        println!("   New ETag (weak): {}", new_etag.as_str().unwrap());
    }

    // Client B tries to update with stale version (should fail)
    sleep(Duration::from_millis(100)).await; // Small delay to simulate real timing

    let client_b_update = ScimOperationRequest::update(
        "User",
        &concurrent_user_id,
        json!({
            "id": concurrent_user_id,
            "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
            "userName": "concurrent.test",
            "name": {
                "givenName": "Client",
                "familyName": "B"
            },
            "active": false
        }),
    )
    .with_expected_version(client_b_version); // Using stale version

    let client_b_response = handler.handle_operation(client_b_update).await;

    if !client_b_response.success {
        println!("✅ Client B update correctly rejected!");
        println!("   Error: {}", client_b_response.error.unwrap());
        println!("   Prevented lost update scenario!");
    } else {
        println!("❌ Client B update should have failed!");
    }

    // Client B retrieves current version and retries
    let get_current_request = ScimOperationRequest::get("User", &concurrent_user_id);
    let get_current_response = handler.handle_operation(get_current_request).await;
    let current_etag = get_current_response
        .metadata
        .additional
        .get("etag")
        .unwrap()
        .as_str()
        .unwrap();
    let current_version = ScimVersion::parse_http_header(current_etag)?;

    println!(
        "👤 Client B retrieves current ETag (weak): {}",
        current_etag
    );

    let client_b_retry = ScimOperationRequest::update(
        "User",
        &concurrent_user_id,
        json!({
            "id": concurrent_user_id,
            "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
            "userName": "concurrent.test",
            "name": {
                "givenName": "Client A and B", // Merge changes
                "familyName": "Combined"
            },
            "active": false
        }),
    )
    .with_expected_version(current_version);

    let client_b_retry_response = handler.handle_operation(client_b_retry).await;

    if client_b_retry_response.success {
        println!("✅ Client B retry succeeded with current version!");
        let final_etag = client_b_retry_response
            .metadata
            .additional
            .get("etag")
            .unwrap();
        println!("   Final ETag (weak): {}", final_etag.as_str().unwrap());
    }

    println!();

    // === SUMMARY ===
    println!("🎉 ETAG CONCURRENCY CONTROL EXAMPLE COMPLETED!");
    println!("==============================================");
    println!("✅ Demonstrated automatic version management");
    println!("✅ Showed conditional update success and failure");
    println!("✅ Verified conditional delete operations");
    println!("✅ Illustrated content-based version computation");
    println!("✅ Simulated concurrent modification protection");
    println!("✅ Prevented lost update scenarios");
    println!();
    println!("🔧 KEY BENEFITS:");
    println!("   • Automatic weak ETag generation from resource content");
    println!("   • Built-in optimistic concurrency control");
    println!("   • RFC 7232 compliant HTTP weak ETag support");
    println!("   • Zero-configuration versioning for all providers");
    println!("   • Comprehensive conflict detection and handling");
    println!("   • Provider-agnostic implementation");

    Ok(())
}
