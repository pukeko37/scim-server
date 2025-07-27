//! # Basic Usage Example
//!
//! This example demonstrates the core functionality of the SCIM server
//! with an in-memory resource provider implementation.

use scim_server::{
    RequestContext, Resource, ResourceProvider, ScimOperation, ScimServer,
    create_user_resource_handler,
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

            // Check for duplicate userName for User resources
            if resource_type == "User" {
                if let Some(username) = data.get("userName").and_then(|v| v.as_str()) {
                    let resources_guard = resources.read().await;
                    if let Some(users) = resources_guard.get("User") {
                        for existing_user in users.values() {
                            if let Some(existing_username) = existing_user
                                .get_attribute("userName")
                                .and_then(|v| v.as_str())
                            {
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

            let mut resource = Resource::new(resource_type.clone(), data);

            // Add SCIM metadata
            let now = chrono::Utc::now().to_rfc3339();
            if let Some(data) = resource.data.as_object_mut() {
                let meta = json!({
                    "resourceType": resource_type,
                    "created": now,
                    "lastModified": now,
                    "version": format!("W/\"{}\"", uuid::Uuid::new_v4()),
                    "location": format!("/scim/v2/{}/{}", resource_type, id)
                });
                data.insert("meta".to_string(), meta);
            }

            // Store resource
            let mut resources_guard = resources.write().await;
            resources_guard
                .entry(resource_type)
                .or_insert_with(HashMap::new)
                .insert(id, resource.clone());

            println!(
                "Resource created successfully with ID: {}",
                resource.get_id().unwrap_or("unknown")
            );
            Ok(resource)
        }
    }

    fn get_resource(
        &self,
        resource_type: &str,
        id: &str,
        context: &RequestContext,
    ) -> impl Future<Output = Result<Option<Resource>, Self::Error>> + Send {
        let resource_type = resource_type.to_string();
        let id = id.to_string();
        let request_id = context.request_id.clone();
        let resources = self.resources.clone();

        async move {
            println!(
                "Getting {} resource {} with request ID: {}",
                resource_type, id, request_id
            );

            let resources = resources.read().await;
            let resource = resources
                .get(&resource_type)
                .and_then(|type_resources| type_resources.get(&id))
                .cloned();

            if resource.is_some() {
                println!("Found {} resource: {}", resource_type, id);
            } else {
                println!("Resource not found: {} {}", resource_type, id);
            }

            Ok(resource)
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
                "Updating {} resource {} with request ID: {}",
                resource_type, id, request_id
            );

            // Ensure the ID is in the data
            if let Some(obj) = data.as_object_mut() {
                obj.insert("id".to_string(), json!(id.clone()));
            }

            // Check if resource exists
            {
                let resources_guard = resources.read().await;
                if !resources_guard
                    .get(&resource_type)
                    .map(|type_resources| type_resources.contains_key(&id))
                    .unwrap_or(false)
                {
                    return Err(ProviderError::ResourceNotFound {
                        resource_type: resource_type.clone(),
                        id: id.clone(),
                    });
                }
            }

            let mut resource = Resource::new(resource_type.clone(), data);

            // Add SCIM metadata
            let now = chrono::Utc::now().to_rfc3339();
            if let Some(data) = resource.data.as_object_mut() {
                let meta = json!({
                    "resourceType": resource_type,
                    "lastModified": now,
                    "version": format!("W/\"{}\"", uuid::Uuid::new_v4()),
                    "location": format!("/scim/v2/{}/{}", resource_type, id)
                });
                data.insert("meta".to_string(), meta);
            }

            // Update resource
            let mut resources_guard = resources.write().await;
            resources_guard
                .entry(resource_type)
                .or_insert_with(HashMap::new)
                .insert(id, resource.clone());

            println!("Resource updated successfully");
            Ok(resource)
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
                "Deleting {} resource {} with request ID: {}",
                resource_type, id, request_id
            );

            let mut resources_guard = resources.write().await;
            let removed = resources_guard
                .get_mut(&resource_type)
                .and_then(|type_resources| type_resources.remove(&id))
                .is_some();

            if removed {
                println!("Resource deleted successfully: {} {}", resource_type, id);
                Ok(())
            } else {
                Err(ProviderError::ResourceNotFound { resource_type, id })
            }
        }
    }

    fn list_resources(
        &self,
        resource_type: &str,
        _query: Option<&scim_server::ListQuery>,
        context: &RequestContext,
    ) -> impl Future<Output = Result<Vec<Resource>, Self::Error>> + Send {
        let resource_type = resource_type.to_string();
        let request_id = context.request_id.clone();
        let resources = self.resources.clone();

        async move {
            println!(
                "Listing {} resources with request ID: {}",
                resource_type, request_id
            );

            let resources = resources.read().await;
            let resource_list: Vec<Resource> = resources
                .get(&resource_type)
                .map(|type_resources| type_resources.values().cloned().collect())
                .unwrap_or_default();

            println!("Found {} {} resources", resource_list.len(), resource_type);
            Ok(resource_list)
        }
    }

    fn find_resource_by_attribute(
        &self,
        resource_type: &str,
        attribute: &str,
        value: &Value,
        context: &RequestContext,
    ) -> impl Future<Output = Result<Option<Resource>, Self::Error>> + Send {
        let resource_type = resource_type.to_string();
        let attribute = attribute.to_string();
        let value = value.clone();
        let request_id = context.request_id.clone();
        let resources = self.resources.clone();

        async move {
            println!(
                "Finding {} resource by {}={} with request ID: {}",
                resource_type, attribute, value, request_id
            );

            let resources = resources.read().await;
            let found_resource = resources
                .get(&resource_type)
                .and_then(|type_resources| {
                    type_resources
                        .values()
                        .find(|resource| resource.get_attribute(&attribute) == Some(&value))
                })
                .cloned();

            if found_resource.is_some() {
                println!("Found resource by attribute: {}={}", attribute, value);
            } else {
                println!("No resource found by attribute: {}={}", attribute, value);
            }

            Ok(found_resource)
        }
    }

    fn resource_exists(
        &self,
        resource_type: &str,
        id: &str,
        context: &RequestContext,
    ) -> impl Future<Output = Result<bool, Self::Error>> + Send {
        let resource_type = resource_type.to_string();
        let id = id.to_string();
        let request_id = context.request_id.clone();
        let resources = self.resources.clone();

        async move {
            println!(
                "Checking if {} resource {} exists with request ID: {}",
                resource_type, id, request_id
            );

            let resources = resources.read().await;
            let exists = resources
                .get(&resource_type)
                .map(|type_resources| type_resources.contains_key(&id))
                .unwrap_or(false);

            println!("{} resource {} exists: {}", resource_type, id, exists);
            Ok(exists)
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 Starting SCIM Server Example");

    // Create our in-memory provider
    let provider = InMemoryProvider::new();

    // Create the SCIM server with our provider
    let mut server = ScimServer::new(provider)?;

    // Get the User schema from the server's registry
    let user_schema = server
        .get_schema_by_id("urn:ietf:params:scim:schemas:core:2.0:User")
        .expect("User schema should be available")
        .clone();

    // Register User resource type with CRUD operations
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

    println!("✅ Server initialized with User resource support");

    // Create some example users
    let context = RequestContext::new("example-request".to_string());

    // Create first user
    let user1_data = json!({
        "userName": "john.doe",
        "name": {
            "givenName": "John",
            "familyName": "Doe",
            "formatted": "John Doe"
        },
        "emails": [
            {
                "value": "john.doe@example.com",
                "primary": true
            }
        ]
    });

    println!("\n📝 Creating user john.doe...");
    let created_user1 = server.create_resource("User", user1_data, &context).await?;
    println!(
        "Created user: {}",
        serde_json::to_string_pretty(&created_user1.data)?
    );

    // Create second user
    let user2_data = json!({
        "userName": "jane.smith",
        "name": {
            "givenName": "Jane",
            "familyName": "Smith",
            "formatted": "Jane Smith"
        },
        "emails": [
            {
                "value": "jane.smith@example.com",
                "primary": true
            }
        ]
    });

    println!("\n📝 Creating user jane.smith...");
    let created_user2 = server.create_resource("User", user2_data, &context).await?;
    println!(
        "Created user: {}",
        serde_json::to_string_pretty(&created_user2.data)?
    );

    // Get user by ID
    if let Some(user_id) = created_user1.get_id() {
        println!("\n🔍 Getting user by ID: {}", user_id);
        if let Some(retrieved_user) = server.get_resource("User", user_id, &context).await? {
            println!(
                "Retrieved user: {}",
                serde_json::to_string_pretty(&retrieved_user.data)?
            );
        }
    }

    // Update user
    if let Some(user_id) = created_user2.get_id() {
        let update_data = json!({
            "userName": "jane.smith",
            "name": {
                "givenName": "Jane",
                "familyName": "Smith-Johnson", // Changed last name
                "formatted": "Jane Smith-Johnson"
            },
            "emails": [
                {
                    "value": "jane.smith@example.com",
                    "primary": true
                },
                {
                    "value": "jane.johnson@example.com",
                    "primary": false
                }
            ]
        });

        println!("\n✏️ Updating user...");
        let updated_user = server
            .update_resource("User", user_id, update_data, &context)
            .await?;
        println!(
            "Updated user: {}",
            serde_json::to_string_pretty(&updated_user.data)?
        );
    }

    // List all users
    println!("\n📋 Listing all users...");
    let all_users = server.list_resources("User", &context).await?;
    println!("Found {} users:", all_users.len());
    for user in &all_users {
        if let Some(username) = user.get_attribute("userName") {
            println!("  - {}", username);
        }
    }

    // Find user by attribute
    println!("\n🔍 Finding user by userName=john.doe...");
    if let Some(found_user) = server
        .find_resource_by_attribute("User", "userName", &json!("john.doe"), &context)
        .await?
    {
        println!(
            "Found user: {}",
            serde_json::to_string_pretty(&found_user.data)?
        );
    }

    // Check if user exists
    if let Some(user_id) = created_user1.get_id() {
        println!("\n❓ Checking if user exists: {}", user_id);
        let exists = server.resource_exists("User", user_id, &context).await?;
        println!("User exists: {}", exists);
    }

    // Delete user
    if let Some(user_id) = created_user1.get_id() {
        println!("\n🗑️ Deleting user: {}", user_id);
        server.delete_resource("User", user_id, &context).await?;
        println!("User deleted successfully");
    }

    // List users after deletion
    println!("\n📋 Listing users after deletion...");
    let remaining_users = server.list_resources("User", &context).await?;
    println!("Found {} users:", remaining_users.len());
    for user in &remaining_users {
        if let Some(username) = user.get_attribute("userName") {
            println!("  - {}", username);
        }
    }

    // Show server capabilities
    println!("\n🎯 Server capabilities:");
    println!(
        "Supported resource types: {:?}",
        server.get_supported_resource_types()
    );

    if let Ok(user_schema) = server.get_resource_schema("User") {
        println!("User schema attributes: {}", user_schema.attributes.len());
    }

    println!("\n✅ Example completed successfully!");
    Ok(())
}
