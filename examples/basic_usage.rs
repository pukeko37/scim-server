//! # Basic Usage Example
//!
//! This example demonstrates the core functionality of the SCIM server
//! with an in-memory resource provider implementation.

use scim_server::resource::value_objects::{EmailAddress, ValueObject};
use scim_server::{
    RequestContext, Resource, ResourceProvider, ScimServer, create_user_resource_handler,
};
use serde_json::{Value, json};
use std::collections::HashMap;
use std::future::Future;
use std::sync::Arc;
use tokio::sync::RwLock;

/// In-memory implementation of ResourceProvider
struct InMemoryProvider {
    resources: Arc<RwLock<HashMap<String, HashMap<String, Resource>>>>, // resource_type -> id -> resource
    next_id: Arc<RwLock<u64>>,
}

impl InMemoryProvider {
    fn new() -> Self {
        Self {
            resources: Arc::new(RwLock::new(HashMap::new())),
            next_id: Arc::new(RwLock::new(1)),
        }
    }
}

/// Custom error type for the provider
#[derive(Debug, thiserror::Error)]
pub enum ProviderError {
    #[error("Resource not found: {resource_type} with id {id}")]
    ResourceNotFound { resource_type: String, id: String },
    #[error("Duplicate attribute {attribute} with value {value} for {resource_type}")]
    DuplicateAttribute {
        resource_type: String,
        attribute: String,
        value: String,
    },
    #[error("Invalid data: {message}")]
    InvalidData { message: String },
    #[error("Internal error: {message}")]
    Internal { message: String },
}

impl ResourceProvider for InMemoryProvider {
    type Error = ProviderError;

    fn create_resource(
        &self,
        resource_type: &str,
        mut data: Value,
        context: &RequestContext,
    ) -> impl Future<Output = Result<Resource, Self::Error>> + Send {
        let resource_type = resource_type.to_string();
        let request_id = context.request_id.clone();
        let resources = self.resources.clone();
        let next_id = self.next_id.clone();

        async move {
            println!(
                "Creating {} resource with request ID: {}",
                resource_type, request_id
            );

            // Generate a unique ID for the new resource
            let mut counter = next_id.write().await;
            let id = counter.to_string();
            *counter += 1;

            if let Some(obj) = data.as_object_mut() {
                obj.insert("id".to_string(), json!(id.clone()));
            }

            // Create resource using new API
            let resource = Resource::from_json(resource_type.clone(), data).map_err(|e| {
                ProviderError::InvalidData {
                    message: format!("Failed to create resource: {}", e),
                }
            })?;

            // Check for duplicate userName for User resources
            if resource_type == "User" {
                if let Some(username) = resource.get_username() {
                    let resources_guard = resources.read().await;
                    if let Some(users) = resources_guard.get("User") {
                        for existing_user in users.values() {
                            if let Some(existing_username) = existing_user.get_username() {
                                if existing_username == username {
                                    return Err(ProviderError::DuplicateAttribute {
                                        resource_type: resource_type.clone(),
                                        attribute: "userName".to_string(),
                                        value: username.to_string(),
                                    });
                                }
                            }
                        }
                    }
                }
            }

            // Add SCIM metadata
            let now = chrono::Utc::now().to_rfc3339();
            let mut resource_with_meta = resource;
            resource_with_meta.add_metadata("/scim/v2", &now, &now);

            // Store resource
            let mut resources_guard = resources.write().await;
            resources_guard
                .entry(resource_type)
                .or_insert_with(HashMap::new)
                .insert(id, resource_with_meta.clone());

            println!(
                "Resource created successfully with ID: {}",
                resource_with_meta.get_id().unwrap_or("unknown")
            );
            Ok(resource_with_meta)
        }
    }

    fn get_resource(
        &self,
        resource_type: &str,
        id: &str,
        _context: &RequestContext,
    ) -> impl Future<Output = Result<Option<Resource>, Self::Error>> + Send {
        let resource_type = resource_type.to_string();
        let id = id.to_string();
        let resources = self.resources.clone();

        async move {
            println!("Getting {} resource with ID: {}", resource_type, id);

            let resources_guard = resources.read().await;
            if let Some(type_resources) = resources_guard.get(&resource_type) {
                Ok(type_resources.get(&id).cloned())
            } else {
                Ok(None)
            }
        }
    }

    fn update_resource(
        &self,
        resource_type: &str,
        id: &str,
        mut data: Value,
        context: &RequestContext,
    ) -> impl Future<Output = Result<Resource, Self::Error>> + Send {
        let resource_type = resource_type.to_string();
        let id = id.to_string();
        let request_id = context.request_id.clone();
        let resources = self.resources.clone();

        async move {
            println!(
                "Updating {} resource with ID: {} (request: {})",
                resource_type, id, request_id
            );

            if let Some(obj) = data.as_object_mut() {
                obj.insert("id".to_string(), json!(id.clone()));
            }

            // Create updated resource using new API
            let resource = Resource::from_json(resource_type.clone(), data).map_err(|e| {
                ProviderError::InvalidData {
                    message: format!("Failed to update resource: {}", e),
                }
            })?;

            // Add SCIM metadata
            let now = chrono::Utc::now().to_rfc3339();
            let mut resource_with_meta = resource;
            resource_with_meta.add_metadata("/scim/v2", &now, &now);

            // Update resource
            let mut resources_guard = resources.write().await;
            if let Some(type_resources) = resources_guard.get_mut(&resource_type) {
                if type_resources.contains_key(&id) {
                    type_resources.insert(id.clone(), resource_with_meta.clone());
                    println!("Resource updated successfully");
                    Ok(resource_with_meta)
                } else {
                    Err(ProviderError::ResourceNotFound { resource_type, id })
                }
            } else {
                Err(ProviderError::ResourceNotFound { resource_type, id })
            }
        }
    }

    fn delete_resource(
        &self,
        resource_type: &str,
        id: &str,
        context: &RequestContext,
    ) -> impl Future<Output = Result<(), Self::Error>> + Send {
        let resource_type = resource_type.to_string();
        let id = id.to_string();
        let request_id = context.request_id.clone();
        let resources = self.resources.clone();

        async move {
            println!(
                "Deleting {} resource with ID: {} (request: {})",
                resource_type, id, request_id
            );

            let mut resources_guard = resources.write().await;
            if let Some(type_resources) = resources_guard.get_mut(&resource_type) {
                if type_resources.remove(&id).is_some() {
                    println!("Resource deleted successfully");
                    Ok(())
                } else {
                    Err(ProviderError::ResourceNotFound { resource_type, id })
                }
            } else {
                Err(ProviderError::ResourceNotFound { resource_type, id })
            }
        }
    }

    fn list_resources(
        &self,
        resource_type: &str,
        _query: Option<&scim_server::ListQuery>,
        _context: &RequestContext,
    ) -> impl Future<Output = Result<Vec<Resource>, Self::Error>> + Send {
        let resource_type = resource_type.to_string();
        let resources = self.resources.clone();

        async move {
            println!("Listing {} resources", resource_type);

            let resources_guard = resources.read().await;
            if let Some(type_resources) = resources_guard.get(&resource_type) {
                let resources: Vec<Resource> = type_resources.values().cloned().collect();
                println!("Found {} resources", resources.len());
                Ok(resources)
            } else {
                Ok(vec![])
            }
        }
    }

    fn find_resource_by_attribute(
        &self,
        resource_type: &str,
        attribute: &str,
        value: &Value,
        _context: &RequestContext,
    ) -> impl Future<Output = Result<Option<Resource>, Self::Error>> + Send {
        let resource_type = resource_type.to_string();
        let attribute = attribute.to_string();
        let value = value.clone();
        let resources = self.resources.clone();

        async move {
            println!(
                "Finding {} resource by {}={}",
                resource_type, attribute, value
            );

            let resources_guard = resources.read().await;
            if let Some(type_resources) = resources_guard.get(&resource_type) {
                for resource in type_resources.values() {
                    if let Some(attr_value) = resource.get_attribute(&attribute) {
                        if attr_value == &value {
                            return Ok(Some(resource.clone()));
                        }
                    }
                }
            }
            Ok(None)
        }
    }

    fn resource_exists(
        &self,
        resource_type: &str,
        id: &str,
        _context: &RequestContext,
    ) -> impl Future<Output = Result<bool, Self::Error>> + Send {
        let resource_type = resource_type.to_string();
        let id = id.to_string();
        let resources = self.resources.clone();

        async move {
            let resources_guard = resources.read().await;
            if let Some(type_resources) = resources_guard.get(&resource_type) {
                Ok(type_resources.contains_key(&id))
            } else {
                Ok(false)
            }
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 Starting SCIM Server Basic Usage Example");

    // Create the in-memory provider
    let provider = InMemoryProvider::new();

    // Create the SCIM server
    let mut server = ScimServer::new(provider)?;

    // Get the User schema from the server's registry
    let user_schema = server
        .get_schema_by_id("urn:ietf:params:scim:schemas:core:2.0:User")
        .expect("User schema should be available")
        .clone();

    // Register User resource handler
    let user_handler = create_user_resource_handler(user_schema);
    server.register_resource_type(
        "User",
        user_handler,
        vec![
            scim_server::ScimOperation::Create,
            scim_server::ScimOperation::Read,
            scim_server::ScimOperation::Update,
            scim_server::ScimOperation::Delete,
            scim_server::ScimOperation::List,
        ],
    )?;

    println!("✅ SCIM Server initialized with User resource handler");

    // Demonstrate basic operations
    let context = RequestContext::new("example-request-1".to_string());

    // 1. Create a user
    println!("\n📝 Creating a new user...");
    let user_data = json!({
        "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
        "userName": "jdoe",
        "name": {
            "familyName": "Doe",
            "givenName": "John",
            "formatted": "John Doe"
        },
        "emails": [
            {
                "value": "john.doe@example.com",
                "type": "work",
                "primary": true
            }
        ],
        "active": true
    });

    let created_user = server.create_resource("User", user_data, &context).await?;
    println!(
        "✅ User created with ID: {}",
        created_user.get_id().unwrap()
    );
    println!("   Username: {}", created_user.get_username().unwrap());

    // Display emails using the new API
    if let Some(emails) = created_user.get_emails() {
        for email in emails.values() {
            // Access the underlying EmailAddress value object
            if let Some(email_obj) = email.as_any().downcast_ref::<EmailAddress>() {
                println!("   Email: {}", email_obj.value);
            }
        }
    }

    // 2. Get the user
    println!("\n🔍 Retrieving the user...");
    let user_id = created_user.get_id().unwrap();
    if let Some(retrieved_user) = server.get_resource("User", &user_id, &context).await? {
        println!(
            "✅ User retrieved: {}",
            retrieved_user.get_username().unwrap()
        );
        println!("   Active: {}", retrieved_user.is_active());
    }

    // 3. Update the user
    println!("\n📝 Updating the user...");
    let update_data = json!({
        "id": user_id,
        "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
        "userName": "jdoe",
        "name": {
            "familyName": "Doe",
            "givenName": "Jane",
            "formatted": "Jane Doe"
        },
        "emails": [
            {
                "value": "jane.doe@example.com",
                "type": "work",
                "primary": true
            }
        ],
        "active": false
    });

    let updated_user = server
        .update_resource("User", &user_id, update_data, &context)
        .await?;
    println!("✅ User updated");
    println!("   New given name: Jane");
    println!("   Active: {}", updated_user.is_active());

    // 4. List users
    println!("\n📋 Listing all users...");
    let users = server.list_resources("User", &context).await?;
    println!("✅ Found {} users", users.len());
    for user in &users {
        println!(
            "   - {} ({})",
            user.get_username().unwrap(),
            user.get_id().unwrap()
        );
    }

    // 5. Create another user to demonstrate uniqueness validation
    println!("\n📝 Attempting to create duplicate user...");
    let duplicate_user_data = json!({
        "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
        "userName": "jdoe",  // Same username
        "name": {
            "familyName": "Smith",
            "givenName": "Bob"
        }
    });

    match server
        .create_resource("User", duplicate_user_data, &context)
        .await
    {
        Ok(_) => println!("❌ Duplicate user creation should have failed!"),
        Err(e) => println!("✅ Correctly rejected duplicate: {}", e),
    }

    // 6. Find user by attribute
    println!("\n🔍 Finding user by userName...");
    if let Some(found_user) = server
        .find_resource_by_attribute("User", "userName", &json!("jdoe"), &context)
        .await?
    {
        println!("✅ Found user: {}", found_user.get_username().unwrap());
    }

    // 7. Check if user exists
    println!("\n❓ Checking if user exists...");
    let exists = server.resource_exists("User", &user_id, &context).await?;
    println!("✅ User exists: {}", exists);

    // 8. Delete the user
    println!("\n🗑️ Deleting the user...");
    server.delete_resource("User", &user_id, &context).await?;
    println!("✅ User deleted");

    // 9. Verify deletion
    println!("\n🔍 Verifying deletion...");
    if server
        .get_resource("User", &user_id, &context)
        .await?
        .is_none()
    {
        println!("✅ User successfully deleted");
    } else {
        println!("❌ User still exists after deletion!");
    }

    // 10. Final resource count
    let final_users = server.list_resources("User", &context).await?;
    println!("✅ Final user count: {}", final_users.len());

    // 11. Show server capabilities
    println!("\n🎯 Server capabilities:");
    println!(
        "Supported resource types: {:?}",
        server.get_supported_resource_types()
    );

    if let Ok(user_schema) = server.get_resource_schema("User") {
        println!("User schema attributes: {}", user_schema.attributes.len());
    }

    println!("\n🎉 Basic usage example completed successfully!");
    Ok(())
}
