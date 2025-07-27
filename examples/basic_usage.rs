//! Basic usage example for the SCIM server library using the dynamic approach.
//!
//! This example demonstrates how to create a dynamic SCIM server implementation
//! that can handle any resource type without hard-coding and perform CRUD operations.

use async_trait::async_trait;
use scim_server::{
    DynamicResourceProvider, DynamicScimServer, RequestContext, Resource, ScimOperation,
    create_user_resource_handler,
};
use serde_json::{Value, json};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Dynamic in-memory resource provider implementation.
///
/// This provider stores all resources in memory using a HashMap. In a real
/// implementation, you would typically use a database or other persistent storage.
#[derive(Debug)]
struct InMemoryProvider {
    resources: Arc<RwLock<HashMap<String, HashMap<String, Resource>>>>, // resource_type -> id -> resource
    next_id: Arc<RwLock<u64>>,
}

impl InMemoryProvider {
    /// Create a new in-memory provider.
    fn new() -> Self {
        Self {
            resources: Arc::new(RwLock::new(HashMap::new())),
            next_id: Arc::new(RwLock::new(1)),
        }
    }

    /// Generate a new unique resource ID.
    async fn generate_id(&self) -> String {
        let mut next_id = self.next_id.write().await;
        let id = format!("resource_{}", *next_id);
        *next_id += 1;
        id
    }

    /// Add server-managed metadata to a resource.
    async fn add_metadata(&self, mut data: Value, resource_type: &str) -> Value {
        let now = chrono::Utc::now().to_rfc3339();

        // Add timestamp metadata
        if let Some(obj) = data.as_object_mut() {
            obj.insert(
                "meta".to_string(),
                json!({
                    "resourceType": resource_type,
                    "created": now,
                    "lastModified": now,
                    "location": format!("/{}/{}", resource_type, obj.get("id").and_then(|v| v.as_str()).unwrap_or("unknown"))
                }),
            );

            // Ensure active field defaults to true for User resources if not provided
            if resource_type == "User" && !obj.contains_key("active") {
                obj.insert("active".to_string(), json!(true));
            }
        }

        data
    }
}

/// Custom error type for our provider.
#[derive(Debug, thiserror::Error)]
pub enum ProviderError {
    #[error("Resource not found: {resource_type}/{id}")]
    ResourceNotFound { resource_type: String, id: String },
    #[error("Duplicate attribute in {resource_type}: {attribute}={value}")]
    DuplicateAttribute {
        resource_type: String,
        attribute: String,
        value: String,
    },
    #[error("Invalid resource data: {message}")]
    InvalidData { message: String },
    #[error("Internal error: {message}")]
    Internal { message: String },
}

#[async_trait]
impl DynamicResourceProvider for InMemoryProvider {
    type Error = ProviderError;

    async fn create_resource(
        &self,
        resource_type: &str,
        mut data: Value,
        context: &RequestContext,
    ) -> Result<Resource, Self::Error> {
        println!(
            "Creating {} resource with request ID: {}",
            resource_type, context.request_id
        );

        // Generate a unique ID for the new resource
        let id = self.generate_id().await;
        if let Some(obj) = data.as_object_mut() {
            obj.insert("id".to_string(), json!(id.clone()));
        }

        // Check for duplicate userName for User resources
        if resource_type == "User" {
            if let Some(username) = data.get("userName").and_then(|v| v.as_str()) {
                let resources = self.resources.read().await;
                if let Some(users) = resources.get("User") {
                    for existing_user in users.values() {
                        if existing_user
                            .get_attribute("userName")
                            .and_then(|v| v.as_str())
                            == Some(username)
                        {
                            return Err(ProviderError::DuplicateAttribute {
                                resource_type: resource_type.to_string(),
                                attribute: "userName".to_string(),
                                value: username.to_string(),
                            });
                        }
                    }
                }
            }
        }

        // Add metadata
        data = self.add_metadata(data, resource_type).await;

        let resource = Resource::new(resource_type.to_string(), data);

        // Store the resource
        let mut resources = self.resources.write().await;
        resources
            .entry(resource_type.to_string())
            .or_insert_with(HashMap::new)
            .insert(id, resource.clone());

        println!(
            "Created {} resource with ID: {}",
            resource_type,
            resource.get_id().unwrap_or("unknown")
        );
        Ok(resource)
    }

    async fn get_resource(
        &self,
        resource_type: &str,
        id: &str,
        context: &RequestContext,
    ) -> Result<Option<Resource>, Self::Error> {
        println!(
            "Getting {} resource {} with request ID: {}",
            resource_type, id, context.request_id
        );

        let resources = self.resources.read().await;
        let resource = resources
            .get(resource_type)
            .and_then(|type_resources| type_resources.get(id))
            .cloned();

        if resource.is_some() {
            println!("Found {} resource: {}", resource_type, id);
        } else {
            println!("{} resource {} not found", resource_type, id);
        }

        Ok(resource)
    }

    async fn update_resource(
        &self,
        resource_type: &str,
        id: &str,
        mut data: Value,
        context: &RequestContext,
    ) -> Result<Resource, Self::Error> {
        println!(
            "Updating {} resource {} with request ID: {}",
            resource_type, id, context.request_id
        );

        // Ensure the ID matches
        if let Some(obj) = data.as_object_mut() {
            obj.insert("id".to_string(), json!(id));
        }

        // Check if resource exists
        {
            let resources = self.resources.read().await;
            if !resources
                .get(resource_type)
                .map(|type_resources| type_resources.contains_key(id))
                .unwrap_or(false)
            {
                return Err(ProviderError::ResourceNotFound {
                    resource_type: resource_type.to_string(),
                    id: id.to_string(),
                });
            }
        }

        // Update metadata
        let now = chrono::Utc::now().to_rfc3339();
        if let Some(obj) = data.as_object_mut() {
            let created_time = obj
                .get("meta")
                .and_then(|m| m.get("created"))
                .and_then(|c| c.as_str())
                .unwrap_or(&now);

            obj.insert(
                "meta".to_string(),
                json!({
                    "resourceType": resource_type,
                    "created": created_time,
                    "lastModified": now,
                    "location": format!("/{}/{}", resource_type, id)
                }),
            );
        }

        let resource = Resource::new(resource_type.to_string(), data);

        // Store the updated resource
        let mut resources = self.resources.write().await;
        resources
            .entry(resource_type.to_string())
            .or_insert_with(HashMap::new)
            .insert(id.to_string(), resource.clone());

        println!("Updated {} resource: {}", resource_type, id);
        Ok(resource)
    }

    async fn delete_resource(
        &self,
        resource_type: &str,
        id: &str,
        context: &RequestContext,
    ) -> Result<(), Self::Error> {
        println!(
            "Deleting {} resource {} with request ID: {}",
            resource_type, id, context.request_id
        );

        let mut resources = self.resources.write().await;
        let removed = resources
            .get_mut(resource_type)
            .and_then(|type_resources| type_resources.remove(id));

        if removed.is_some() {
            println!("Deleted {} resource: {}", resource_type, id);
        } else {
            println!(
                "{} resource {} was already deleted or didn't exist",
                resource_type, id
            );
        }

        Ok(()) // Idempotent operation
    }

    async fn list_resources(
        &self,
        resource_type: &str,
        context: &RequestContext,
    ) -> Result<Vec<Resource>, Self::Error> {
        println!(
            "Listing {} resources with request ID: {}",
            resource_type, context.request_id
        );

        let resources = self.resources.read().await;
        let resource_list: Vec<Resource> = resources
            .get(resource_type)
            .map(|type_resources| type_resources.values().cloned().collect())
            .unwrap_or_default();

        println!("Found {} {} resources", resource_list.len(), resource_type);
        Ok(resource_list)
    }

    async fn find_resource_by_attribute(
        &self,
        resource_type: &str,
        attribute: &str,
        value: &Value,
        context: &RequestContext,
    ) -> Result<Option<Resource>, Self::Error> {
        println!(
            "Finding {} resource by {}={} with request ID: {}",
            resource_type, attribute, value, context.request_id
        );

        let resources = self.resources.read().await;
        let found_resource = resources
            .get(resource_type)
            .and_then(|type_resources| {
                type_resources.values().find(|resource| {
                    resource
                        .get_attribute(attribute)
                        .map(|attr_value| attr_value == value)
                        .unwrap_or(false)
                })
            })
            .cloned();

        if found_resource.is_some() {
            println!(
                "Found {} resource by {}={}",
                resource_type, attribute, value
            );
        } else {
            println!(
                "No {} resource found with {}={}",
                resource_type, attribute, value
            );
        }

        Ok(found_resource)
    }

    async fn resource_exists(
        &self,
        resource_type: &str,
        id: &str,
        context: &RequestContext,
    ) -> Result<bool, Self::Error> {
        println!(
            "Checking if {} resource {} exists with request ID: {}",
            resource_type, id, context.request_id
        );

        let resources = self.resources.read().await;
        let exists = resources
            .get(resource_type)
            .map(|type_resources| type_resources.contains_key(id))
            .unwrap_or(false);

        println!("{} resource {} exists: {}", resource_type, id, exists);
        Ok(exists)
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("SCIM Server Library - Dynamic Usage Example");
    println!("===========================================\n");

    // Create the resource provider
    let provider = InMemoryProvider::new();

    // Create the dynamic SCIM server
    let mut server = DynamicScimServer::new(provider)?;

    // Register resource types with their handlers and supported operations
    let user_schema = server
        .get_schema_by_id("urn:ietf:params:scim:schemas:core:2.0:User")
        .expect("User schema should be available in schema registry")
        .clone();
    let user_handler = create_user_resource_handler(user_schema);
    let _ = server.register_resource_type(
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
    );

    println!("✓ Dynamic SCIM server created and configured\n");

    // Demonstrate schema discovery
    println!("1. Schema Discovery");
    println!("-------------------");

    let schemas = server.get_all_schemas();
    println!("Available schemas: {}", schemas.len());
    for schema in &schemas {
        println!("  - {} ({})", schema.name, schema.id);
        println!("    Description: {}", schema.description);
        println!("    Attributes: {}", schema.attributes.len());
    }

    let supported_types = server.get_supported_resource_types();
    println!("\nSupported resource types: {:?}", supported_types);

    let supported_ops = server.get_supported_operations("User");
    println!("User operations: {:?}", supported_ops);

    println!("\n2. Dynamic Resource Management");
    println!("------------------------------");

    // Create request context
    let context = RequestContext::new("dynamic-usage-example".to_string())
        .with_metadata("source".to_string(), "example".to_string());

    // Create a sample user using dynamic operations
    let user_data = json!({
        "userName": "jdoe",
        "displayName": "John Doe",
        "name": {
            "givenName": "John",
            "familyName": "Doe",
            "formatted": "John Doe"
        },
        "emails": [
            {
                "value": "john.doe@example.com",
                "type": "work",
                "primary": true
            },
            {
                "value": "john@personal.com",
                "type": "home",
                "primary": false
            }
        ],
        "phoneNumbers": [
            {
                "value": "+1-555-0123",
                "type": "work"
            }
        ],
        "active": true
    });

    println!("Creating User resource...");
    let created_user = server.create_resource("User", user_data, &context).await?;
    let user_id = created_user.get_id().unwrap();
    println!("✓ User resource created with ID: {}", user_id);

    // Create another user
    let user2_data = json!({
        "userName": "asmith",
        "displayName": "Alice Smith",
        "emails": [
            {
                "value": "alice.smith@example.com",
                "type": "work",
                "primary": true
            }
        ]
    });

    println!("\nCreating second User resource...");
    let created_user2 = server.create_resource("User", user2_data, &context).await?;
    let user2_id = created_user2.get_id().unwrap();
    println!("✓ Second User resource created with ID: {}", user2_id);

    // Retrieve the user
    println!("\nRetrieving User resource {}...", user_id);
    let retrieved_user = server.get_resource("User", user_id, &context).await?;
    if let Some(user) = retrieved_user {
        println!(
            "✓ Retrieved user: {}",
            user.get_attribute("userName")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown")
        );
        println!(
            "  Display name: {}",
            user.get_attribute("displayName")
                .and_then(|v| v.as_str())
                .unwrap_or("N/A")
        );

        if let Some(emails) = user.get_attribute("emails").and_then(|v| v.as_array()) {
            println!("  Emails: {}", emails.len());
            for email in emails {
                if let (Some(value), Some(type_)) = (
                    email.get("value").and_then(|v| v.as_str()),
                    email.get("type").and_then(|v| v.as_str()),
                ) {
                    println!("    - {} ({})", value, type_);
                }
            }
        }
    }

    // List all users
    println!("\nListing all User resources...");
    let users = server.list_resources("User", &context).await?;
    println!("✓ Found {} User resources:", users.len());
    for user in &users {
        println!(
            "  - {} ({})",
            user.get_attribute("userName")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown"),
            user.get_id().unwrap_or("no-id")
        );
    }

    // Update a user
    println!("\nUpdating User resource {}...", user_id);
    let updated_data = json!({
        "userName": "jdoe",
        "displayName": "John F. Doe", // Changed display name
        "name": {
            "givenName": "John",
            "middleName": "Francis", // Added middle name
            "familyName": "Doe",
            "formatted": "John Francis Doe"
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

    let updated_user = server
        .update_resource("User", user_id, updated_data, &context)
        .await?;
    println!(
        "✓ User resource updated. New display name: {}",
        updated_user
            .get_attribute("displayName")
            .and_then(|v| v.as_str())
            .unwrap_or("N/A")
    );

    // Search for user by username
    println!("\nSearching for User resource by userName 'asmith'...");
    let found_user = server
        .find_resource_by_attribute("User", "userName", &json!("asmith"), &context)
        .await?;
    if let Some(user) = found_user {
        println!(
            "✓ Found user: {} ({})",
            user.get_attribute("userName")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown"),
            user.get_id().unwrap_or("no-id")
        );
    }

    // Check if user exists
    println!("\nChecking if User resource {} exists...", user_id);
    let exists = server.resource_exists("User", user_id, &context).await?;
    println!("✓ User resource exists: {}", exists);

    // Delete a user
    println!("\nDeleting User resource {}...", user2_id);
    server.delete_resource("User", user2_id, &context).await?;
    println!("✓ User resource deleted");

    // Verify deletion
    println!("\nVerifying deletion...");
    let deleted_user = server.get_resource("User", user2_id, &context).await?;
    if deleted_user.is_none() {
        println!("✓ User resource successfully deleted");
    }

    // Final user count
    let final_users = server.list_resources("User", &context).await?;
    println!("\nFinal User resource count: {}", final_users.len());

    println!("\n3. Error Handling");
    println!("-----------------");

    // Try to create a user with duplicate username
    println!("Attempting to create User with duplicate userName...");
    let duplicate_data = json!({
        "userName": "jdoe", // This username already exists
        "displayName": "Duplicate User"
    });

    match server
        .create_resource("User", duplicate_data, &context)
        .await
    {
        Ok(_) => println!("❌ Should have failed due to duplicate username"),
        Err(e) => println!("✓ Duplicate username prevented: {}", e),
    }

    // Try to get a non-existent user
    println!("\nAttempting to get non-existent User resource...");
    let missing_user = server.get_resource("User", "nonexistent", &context).await?;
    if missing_user.is_none() {
        println!("✓ Non-existent User resource correctly returned None");
    }

    // Try to perform unsupported operation
    println!("\nAttempting unsupported operation on unregistered resource type...");
    match server
        .create_resource("Group", json!({"name": "test"}), &context)
        .await
    {
        Ok(_) => println!("❌ Should have failed for unregistered resource type"),
        Err(e) => println!("✓ Unregistered resource type prevented: {}", e),
    }

    println!("\n✓ Dynamic example completed successfully!");
    println!("\nThis example demonstrated:");
    println!("- Dynamic server creation and resource type registration");
    println!("- Schema discovery for registered types");
    println!("- Dynamic resource CRUD operations");
    println!("- Resource search functionality");
    println!("- Error handling for business rules");
    println!("- Resource type validation");
    println!("- Flexible, schema-driven approach");

    Ok(())
}
