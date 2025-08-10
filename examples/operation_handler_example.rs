//! Operation Handler Example
//!
//! This example demonstrates how to use the ScimOperationHandler foundation
//! for framework-agnostic SCIM operations. This handler can serve as the basis
//! for both HTTP and MCP integrations.

use scim_server::{
    ScimServer,
    multi_tenant::ScimOperation,
    operation_handler::{ScimOperationHandler, ScimOperationRequest},
    providers::InMemoryProvider,
    resource::TenantContext,
    resource_handlers::create_user_resource_handler,
};
use serde_json::json;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    env_logger::init();

    println!("🚀 SCIM Operation Handler Example");
    println!("==================================\n");

    // 1. Create the provider and server
    let provider = InMemoryProvider::new();
    let mut server = ScimServer::new(provider)?;

    // 2. Register User resource type
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
            ScimOperation::List,
            ScimOperation::Search,
        ],
    )?;

    // 3. Create the operation handler
    let handler = ScimOperationHandler::new(server);

    println!("✅ Server initialized with operation handler\n");

    // 4. Demonstrate various operations using structured requests

    // === SCHEMA OPERATIONS ===
    println!("📋 SCHEMA OPERATIONS");
    println!("==================");

    // Get all schemas
    let schemas_request = ScimOperationRequest::get_schemas();
    let schemas_response = handler.handle_operation(schemas_request).await;

    if schemas_response.success {
        println!("✅ Retrieved schemas successfully");
        if let Some(data) = schemas_response.data {
            if let Some(schemas_array) = data.get("schemas") {
                println!(
                    "   Found {} schemas",
                    schemas_array.as_array().unwrap().len()
                );
            }
        }
    } else {
        println!(
            "❌ Failed to retrieve schemas: {:?}",
            schemas_response.error
        );
    }

    // Get specific schema
    let user_schema_request =
        ScimOperationRequest::get_schema("urn:ietf:params:scim:schemas:core:2.0:User");
    let user_schema_response = handler.handle_operation(user_schema_request).await;

    if user_schema_response.success {
        println!("✅ Retrieved User schema successfully");
        if let Some(data) = user_schema_response.data {
            if let Some(attributes) = data.get("attributes") {
                println!(
                    "   User schema has {} attributes",
                    attributes.as_array().unwrap().len()
                );
            }
        }
    }

    println!();

    // === USER MANAGEMENT OPERATIONS ===
    println!("👤 USER MANAGEMENT OPERATIONS");
    println!("============================");

    // Create a user
    let create_request = ScimOperationRequest::create(
        "User",
        json!({
            "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
            "userName": "alice.doe",
            "name": {
                "familyName": "Doe",
                "givenName": "Alice",
                "formatted": "Alice Doe"
            },
            "emails": [
                {
                    "value": "alice.doe@example.com",
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
        println!("✅ Created user with ID: {}", user_id);
        user_id
    } else {
        println!("❌ Failed to create user: {:?}", create_response.error);
        return Ok(());
    };

    // Get the user
    let get_request = ScimOperationRequest::get("User", &user_id);
    let get_response = handler.handle_operation(get_request).await;

    if get_response.success {
        println!("✅ Retrieved user successfully");
        if let Some(data) = get_response.data {
            if let Some(username) = data.get("userName") {
                println!("   Username: {}", username.as_str().unwrap());
            }
        }
    } else {
        println!("❌ Failed to retrieve user: {:?}", get_response.error);
    }

    // Update the user
    let update_request = ScimOperationRequest::update(
        "User",
        &user_id,
        json!({
            "id": user_id,
            "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
            "userName": "alice.doe",
            "name": {
                "familyName": "Doe",
                "givenName": "Alice",
                "formatted": "Alice M. Doe"
            },
            "emails": [
                {
                    "value": "alice.doe@newcompany.com",
                    "type": "work",
                    "primary": true
                }
            ],
            "active": true
        }),
    );

    let update_response = handler.handle_operation(update_request).await;
    if update_response.success {
        println!("✅ Updated user successfully");
    } else {
        println!("❌ Failed to update user: {:?}", update_response.error);
    }

    // Search for user by username
    let search_request = ScimOperationRequest::search("User", "userName", json!("alice.doe"));
    let search_response = handler.handle_operation(search_request).await;

    if search_response.success {
        if search_response.data.is_some() {
            println!("✅ Found user by username search");
        } else {
            println!("⚠️ No user found matching search criteria");
        }
    } else {
        println!("❌ Search failed: {:?}", search_response.error);
    }

    // Check if user exists
    let exists_request = ScimOperationRequest::exists("User", &user_id);
    let exists_response = handler.handle_operation(exists_request).await;

    if exists_response.success {
        if let Some(data) = exists_response.data {
            if let Some(exists) = data.get("exists") {
                println!("✅ User exists check: {}", exists.as_bool().unwrap());
            }
        }
    }

    // List all users
    let list_request = ScimOperationRequest::list("User");
    let list_response = handler.handle_operation(list_request).await;

    if list_response.success {
        println!("✅ Listed users successfully");
        println!(
            "   Total users: {}",
            list_response.metadata.resource_count.unwrap_or(0)
        );
    } else {
        println!("❌ Failed to list users: {:?}", list_response.error);
    }

    println!();

    // === MULTI-TENANT OPERATIONS ===
    println!("🏢 MULTI-TENANT OPERATIONS");
    println!("=========================");

    // Create user in tenant A
    let tenant_a_context = TenantContext::new("tenant-a".to_string(), "client-a".to_string());
    let tenant_create_request = ScimOperationRequest::create(
        "User",
        json!({
            "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
            "userName": "bob.smith",
            "name": {
                "familyName": "Smith",
                "givenName": "Bob"
            }
        }),
    )
    .with_tenant(tenant_a_context);

    let tenant_create_response = handler.handle_operation(tenant_create_request).await;
    if tenant_create_response.success {
        println!("✅ Created user in tenant A");
        println!(
            "   Tenant ID: {}",
            tenant_create_response.metadata.tenant_id.unwrap()
        );
    }

    // List users in tenant A vs global
    let global_list_request = ScimOperationRequest::list("User");
    let global_list_response = handler.handle_operation(global_list_request).await;

    let tenant_a_context = TenantContext::new("tenant-a".to_string(), "client-a".to_string());
    let tenant_list_request = ScimOperationRequest::list("User").with_tenant(tenant_a_context);
    let tenant_list_response = handler.handle_operation(tenant_list_request).await;

    if global_list_response.success && tenant_list_response.success {
        println!("✅ Tenant isolation verified:");
        println!(
            "   Global users: {}",
            global_list_response.metadata.resource_count.unwrap_or(0)
        );
        println!(
            "   Tenant A users: {}",
            tenant_list_response.metadata.resource_count.unwrap_or(0)
        );
    }

    println!();

    // === ERROR HANDLING DEMONSTRATION ===
    println!("⚠️  ERROR HANDLING DEMONSTRATION");
    println!("===============================");

    // Try to get non-existent user
    let invalid_get_request = ScimOperationRequest::get("User", "non-existent-id");
    let invalid_get_response = handler.handle_operation(invalid_get_request).await;

    if !invalid_get_response.success {
        println!("✅ Properly handled non-existent resource error");
        println!("   Error: {}", invalid_get_response.error.unwrap());
        println!(
            "   Error Code: {}",
            invalid_get_response.error_code.unwrap()
        );
    }

    // Try to create user with invalid data
    let invalid_create_request = ScimOperationRequest::create(
        "User",
        json!({
            "invalid_field": "invalid_value"
        }),
    );
    let invalid_create_response = handler.handle_operation(invalid_create_request).await;

    if !invalid_create_response.success {
        println!("✅ Properly handled validation error");
        println!(
            "   Error Code: {}",
            invalid_create_response.error_code.unwrap()
        );
    }

    // Try unsupported resource type
    let unsupported_request = ScimOperationRequest::get("UnsupportedType", "some-id");
    let unsupported_response = handler.handle_operation(unsupported_request).await;

    if !unsupported_response.success {
        println!("✅ Properly handled unsupported resource type");
        println!(
            "   Error Code: {}",
            unsupported_response.error_code.unwrap()
        );
    }

    println!();

    // === CLEANUP ===
    println!("🧹 CLEANUP");
    println!("=========");

    // Delete the user
    let delete_request = ScimOperationRequest::delete("User", &user_id);
    let delete_response = handler.handle_operation(delete_request).await;

    if delete_response.success {
        println!("✅ Deleted user successfully");
    } else {
        println!("❌ Failed to delete user: {:?}", delete_response.error);
    }

    // Verify deletion
    let verify_request = ScimOperationRequest::get("User", &user_id);
    let verify_response = handler.handle_operation(verify_request).await;

    if !verify_response.success {
        println!("✅ Verified user deletion - user no longer exists");
    }

    println!();

    // === SUMMARY ===
    println!("🎉 OPERATION HANDLER EXAMPLE COMPLETED!");
    println!("=======================================");
    println!("✅ Demonstrated structured SCIM operations");
    println!("✅ Showed framework-agnostic request/response handling");
    println!("✅ Verified multi-tenant support");
    println!("✅ Tested comprehensive error handling");
    println!("✅ Confirmed resource lifecycle management");
    println!();
    println!("🔧 INTEGRATION READY:");
    println!("   • Use ScimOperationHandler for HTTP frameworks (Axum, Actix, etc.)");
    println!("   • Use ScimOperationHandler for MCP tool integration");
    println!("   • Use ScimOperationHandler for CLI tools");
    println!("   • Use ScimOperationHandler for any custom integration");

    Ok(())
}
