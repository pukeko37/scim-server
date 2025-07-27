//! Basic usage example for the SCIM server library.
//!
//! This example demonstrates how to create a simple in-memory SCIM server
//! implementation and perform basic CRUD operations on User resources.

use async_trait::async_trait;
use scim_server::{RequestContext, Resource, ResourceProvider, ScimServer, ServiceProviderConfig};
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Simple in-memory resource provider implementation.
///
/// This provider stores all users in memory using a HashMap. In a real
/// implementation, you would typically use a database or other persistent storage.
#[derive(Debug)]
struct InMemoryProvider {
    users: Arc<RwLock<HashMap<String, Resource>>>,
    next_id: Arc<RwLock<u64>>,
}

impl InMemoryProvider {
    /// Create a new in-memory provider.
    fn new() -> Self {
        Self {
            users: Arc::new(RwLock::new(HashMap::new())),
            next_id: Arc::new(RwLock::new(1)),
        }
    }

    /// Generate a new unique user ID.
    async fn generate_id(&self) -> String {
        let mut next_id = self.next_id.write().await;
        let id = format!("user_{}", *next_id);
        *next_id += 1;
        id
    }

    /// Add server-managed metadata to a user resource.
    async fn add_user_metadata(&self, mut user: Resource) -> Resource {
        let now = chrono::Utc::now().to_rfc3339();

        // Add timestamp metadata
        user.add_metadata("https://example.com/scim", &now, &now);

        // Ensure active field defaults to true if not provided
        if user.get_attribute("active").is_none() {
            user.set_attribute("active".to_string(), json!(true));
        }

        user
    }
}

/// Custom error type for our provider.
#[derive(Debug, thiserror::Error)]
pub enum ProviderError {
    #[error("User not found: {id}")]
    UserNotFound { id: String },
    #[error("Username already exists: {username}")]
    DuplicateUsername { username: String },
    #[error("Invalid user data: {message}")]
    InvalidData { message: String },
    #[error("Internal error: {message}")]
    Internal { message: String },
}

#[async_trait]
impl ResourceProvider for InMemoryProvider {
    type Error = ProviderError;

    async fn create_user(
        &self,
        mut user: Resource,
        context: &RequestContext,
    ) -> Result<Resource, Self::Error> {
        println!("Creating user with request ID: {}", context.request_id);

        // Generate a unique ID for the new user
        let id = self.generate_id().await;
        user.set_attribute("id".to_string(), json!(id.clone()));

        // Check for duplicate usernames
        let username = user
            .get_username()
            .ok_or_else(|| ProviderError::InvalidData {
                message: "userName is required".to_string(),
            })?
            .to_string();

        // Check if username already exists
        let users = self.users.read().await;
        for existing_user in users.values() {
            if existing_user.get_username() == Some(&username) {
                return Err(ProviderError::DuplicateUsername {
                    username: username.clone(),
                });
            }
        }
        drop(users); // Release read lock

        // Add metadata and store the user
        user = self.add_user_metadata(user).await;

        let mut users = self.users.write().await;
        users.insert(id, user.clone());

        println!("Created user: {} ({})", username, user.get_id().unwrap());
        Ok(user)
    }

    async fn get_user(
        &self,
        id: &str,
        context: &RequestContext,
    ) -> Result<Option<Resource>, Self::Error> {
        println!(
            "Getting user {} with request ID: {}",
            id, context.request_id
        );

        let users = self.users.read().await;
        let user = users.get(id).cloned();

        if let Some(ref user) = user {
            println!("Found user: {}", user.get_username().unwrap_or("unknown"));
        } else {
            println!("User {} not found", id);
        }

        Ok(user)
    }

    async fn update_user(
        &self,
        id: &str,
        mut user: Resource,
        context: &RequestContext,
    ) -> Result<Resource, Self::Error> {
        println!(
            "Updating user {} with request ID: {}",
            id, context.request_id
        );

        // Ensure the ID matches
        user.set_attribute("id".to_string(), json!(id));

        // Check if user exists
        {
            let users = self.users.read().await;
            if !users.contains_key(id) {
                return Err(ProviderError::UserNotFound { id: id.to_string() });
            }
        }

        // Update metadata
        let now = chrono::Utc::now().to_rfc3339();
        let created_time = user
            .get_attribute("meta")
            .and_then(|m| m.get("created"))
            .and_then(|c| c.as_str())
            .unwrap_or(&now)
            .to_string();

        user.add_metadata("https://example.com/scim", &created_time, &now);

        // Store the updated user
        let mut users = self.users.write().await;
        users.insert(id.to_string(), user.clone());

        println!("Updated user: {}", user.get_username().unwrap_or("unknown"));
        Ok(user)
    }

    async fn delete_user(&self, id: &str, context: &RequestContext) -> Result<(), Self::Error> {
        println!(
            "Deleting user {} with request ID: {}",
            id, context.request_id
        );

        let mut users = self.users.write().await;
        let removed = users.remove(id);

        if let Some(user) = removed {
            println!("Deleted user: {}", user.get_username().unwrap_or("unknown"));
        } else {
            println!("User {} was already deleted or didn't exist", id);
        }

        Ok(()) // Idempotent operation
    }

    async fn list_users(&self, context: &RequestContext) -> Result<Vec<Resource>, Self::Error> {
        println!("Listing users with request ID: {}", context.request_id);

        let users = self.users.read().await;
        let user_list: Vec<Resource> = users.values().cloned().collect();

        println!("Found {} users", user_list.len());
        Ok(user_list)
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("SCIM Server Library - Basic Usage Example");
    println!("=========================================\n");

    // Create the resource provider
    let provider = InMemoryProvider::new();

    // Configure service provider capabilities
    let service_config = ServiceProviderConfig {
        patch_supported: false,
        bulk_supported: false,
        filter_supported: false,
        change_password_supported: false,
        sort_supported: false,
        etag_supported: false,
        authentication_schemes: vec![],
        bulk_max_operations: None,
        bulk_max_payload_size: None,
        filter_max_results: Some(100),
    };

    // Build the SCIM server
    let server = ScimServer::builder()
        .with_resource_provider(provider)
        .with_service_config(service_config)
        .build()?;

    println!("✓ SCIM server created successfully\n");

    // Demonstrate schema discovery
    println!("1. Schema Discovery");
    println!("-------------------");

    let schemas = server.get_schemas().await?;
    println!("Available schemas: {}", schemas.len());
    for schema in &schemas {
        println!("  - {} ({})", schema.name, schema.id);
        println!("    Description: {}", schema.description);
        println!("    Attributes: {}", schema.attributes.len());
    }

    let user_schema = server
        .get_schema("urn:ietf:params:scim:schemas:core:2.0:User")
        .await?;
    if let Some(schema) = user_schema {
        println!("\nUser schema details:");
        for attr in &schema.attributes {
            println!(
                "  - {}: {:?} (required: {})",
                attr.name, attr.data_type, attr.required
            );
        }
    }

    let config = server.get_service_provider_config().await?;
    println!("\nService Provider Configuration:");
    println!("  - PATCH supported: {}", config.patch_supported);
    println!("  - Bulk supported: {}", config.bulk_supported);
    println!("  - Filter supported: {}", config.filter_supported);

    println!("\n2. User Management");
    println!("------------------");

    // Create request context
    let context = RequestContext::new().with_metadata("source".to_string(), "example".to_string());

    // Create a sample user
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

    println!("Creating user...");
    let created_user = server.create_user(user_data, context.clone()).await?;
    let user_id = created_user.get_id().unwrap();
    println!("✓ User created with ID: {}", user_id);

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

    println!("\nCreating second user...");
    let created_user2 = server.create_user(user2_data, context.clone()).await?;
    let user2_id = created_user2.get_id().unwrap();
    println!("✓ Second user created with ID: {}", user2_id);

    // Retrieve the user
    println!("\nRetrieving user {}...", user_id);
    let retrieved_user = server.get_user(user_id, context.clone()).await?;
    if let Some(user) = retrieved_user {
        println!("✓ Retrieved user: {}", user.get_username().unwrap());
        println!(
            "  Display name: {}",
            user.get_attribute("displayName")
                .and_then(|v| v.as_str())
                .unwrap_or("N/A")
        );

        let emails = user.get_emails();
        println!("  Emails: {}", emails.len());
        for email in emails {
            println!(
                "    - {} ({})",
                email.value,
                email.email_type.as_deref().unwrap_or("unspecified")
            );
        }
    }

    // List all users
    println!("\nListing all users...");
    let users = server.list_users(context.clone()).await?;
    println!("✓ Found {} users:", users.len());
    for user in &users {
        println!(
            "  - {} ({})",
            user.get_username().unwrap_or("unknown"),
            user.get_id().unwrap_or("no-id")
        );
    }

    // Update a user
    println!("\nUpdating user {}...", user_id);
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
        .update_user(user_id, updated_data, context.clone())
        .await?;
    println!(
        "✓ User updated. New display name: {}",
        updated_user
            .get_attribute("displayName")
            .and_then(|v| v.as_str())
            .unwrap_or("N/A")
    );

    // Search for user by username
    println!("\nSearching for user by username 'asmith'...");
    let found_user = server
        .find_user_by_username("asmith", context.clone())
        .await?;
    if let Some(user) = found_user {
        println!(
            "✓ Found user: {} ({})",
            user.get_username().unwrap(),
            user.get_id().unwrap()
        );
    }

    // Check if user exists
    println!("\nChecking if user {} exists...", user_id);
    let exists = server.user_exists(user_id, context.clone()).await?;
    println!("✓ User exists: {}", exists);

    // Delete a user
    println!("\nDeleting user {}...", user2_id);
    server.delete_user(user2_id, context.clone()).await?;
    println!("✓ User deleted");

    // Verify deletion
    println!("\nVerifying deletion...");
    let deleted_user = server.get_user(user2_id, context.clone()).await?;
    if deleted_user.is_none() {
        println!("✓ User successfully deleted");
    }

    // Final user count
    let final_users = server.list_users(context.clone()).await?;
    println!("\nFinal user count: {}", final_users.len());

    println!("\n3. Error Handling");
    println!("-----------------");

    // Try to create a user with invalid data
    println!("Attempting to create user with missing userName...");
    let invalid_data = json!({
        "displayName": "Invalid User"
        // Missing required userName
    });

    match server.create_user(invalid_data, context.clone()).await {
        Ok(_) => println!("❌ Should have failed validation"),
        Err(e) => println!("✓ Validation failed as expected: {}", e),
    }

    // Try to get a non-existent user
    println!("\nAttempting to get non-existent user...");
    let missing_user = server.get_user("nonexistent", context.clone()).await?;
    if missing_user.is_none() {
        println!("✓ Non-existent user correctly returned None");
    }

    // Try to create user with duplicate username
    println!("\nAttempting to create user with duplicate username...");
    let duplicate_data = json!({
        "userName": "jdoe", // This username already exists
        "displayName": "Duplicate User"
    });

    match server.create_user(duplicate_data, context).await {
        Ok(_) => println!("❌ Should have failed due to duplicate username"),
        Err(e) => println!("✓ Duplicate username prevented: {}", e),
    }

    println!("\n✓ Example completed successfully!");
    println!("\nThis example demonstrated:");
    println!("- Server creation and configuration");
    println!("- Schema discovery");
    println!("- User CRUD operations");
    println!("- Search functionality");
    println!("- Error handling and validation");
    println!("- Type-safe state management");

    Ok(())
}
