//! Basic SCIM Server Usage Example
//!
//! This example demonstrates the basic functionality of a SCIM server
//! using the StandardResourceProvider with in-memory storage.

use scim_server::{
    RequestContext, ResourceProvider, providers::StandardResourceProvider, storage::InMemoryStorage,
};
use serde_json::json;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 Starting SCIM Server Basic Usage Example");

    // Create the StandardResourceProvider with in-memory storage
    let storage = InMemoryStorage::new();
    let provider = StandardResourceProvider::new(storage);

    println!("✅ StandardResourceProvider initialized with in-memory storage");

    // Create a request context for our operations
    let context = RequestContext::new("example-request-1".to_string());

    println!("\n📝 Creating users...");

    // Create first user
    let user1_data = json!({
        "userName": "john.doe@example.com",
        "name": {
            "formatted": "John Doe",
            "familyName": "Doe",
            "givenName": "John"
        },
        "emails": [
            {
                "value": "john.doe@example.com",
                "type": "work",
                "primary": true
            },
            {
                "value": "john.personal@example.com",
                "type": "home",
                "primary": false
            }
        ],
        "phoneNumbers": [
            {
                "value": "+1-555-555-1234",
                "type": "work"
            }
        ],
        "active": true
    });

    let user1 = provider
        .create_resource("User", user1_data, &context)
        .await?;
    println!(
        "✅ Created user: {} (ID: {})",
        user1.get_username().unwrap_or("unknown"),
        user1.get_id().unwrap_or("unknown")
    );

    // Create second user
    let user2_data = json!({
        "userName": "jane.smith@example.com",
        "name": {
            "formatted": "Jane Smith",
            "familyName": "Smith",
            "givenName": "Jane"
        },
        "emails": [
            {
                "value": "jane.smith@example.com",
                "type": "work",
                "primary": true
            }
        ],
        "active": true
    });

    let user2 = provider
        .create_resource("User", user2_data, &context)
        .await?;
    println!(
        "✅ Created user: {} (ID: {})",
        user2.get_username().unwrap_or("unknown"),
        user2.get_id().unwrap_or("unknown")
    );

    println!("\n📋 Listing all users...");

    // List all users
    let users = provider.list_resources("User", None, &context).await?;
    println!("📊 Found {} users:", users.len());
    for user in &users {
        println!(
            "  - {} (ID: {})",
            user.get_username().unwrap_or("unknown"),
            user.get_id().unwrap_or("unknown")
        );
    }

    println!("\n🔍 Finding users by attributes...");

    // Find user by username
    let found_users = provider
        .find_resources_by_attribute("User", "userName", "john.doe@example.com", &context)
        .await?;

    if !found_users.is_empty() {
        let user = &found_users[0];
        println!(
            "✅ Found user by username: {} (ID: {})",
            user.get_username().unwrap_or("unknown"),
            user.get_id().unwrap_or("unknown")
        );
    } else {
        println!("❌ User not found by username");
    }

    // Find user by email
    let found_by_email = provider
        .find_resources_by_attribute("User", "userName", "jane.smith@example.com", &context)
        .await?;

    if !found_by_email.is_empty() {
        let user = &found_by_email[0];
        println!(
            "✅ Found user by email: {} (ID: {})",
            user.get_username().unwrap_or("unknown"),
            user.get_id().unwrap_or("unknown")
        );
    } else {
        println!("❌ User not found by email");
    }

    println!("\n✏️  Updating user...");

    // Update the first user
    let user1_id = user1.get_id().unwrap();
    let updated_data = json!({
        "id": user1_id,
        "userName": "john.doe@example.com",
        "name": {
            "formatted": "John Updated Doe",
            "familyName": "Doe",
            "givenName": "John",
            "middleName": "Updated"
        },
        "emails": [
            {
                "value": "john.doe@example.com",
                "type": "work",
                "primary": true
            },
            {
                "value": "john.personal@example.com",
                "type": "home",
                "primary": false
            }
        ],
        "phoneNumbers": [
            {
                "value": "+1-555-555-1234",
                "type": "work"
            },
            {
                "value": "+1-555-555-5678",
                "type": "mobile"
            }
        ],
        "active": true
    });

    let updated_user = provider
        .update_resource("User", user1_id, updated_data, None, &context)
        .await?;
    println!(
        "✅ Updated user: {} (ID: {})",
        updated_user.get_username().unwrap_or("unknown"),
        updated_user.get_id().unwrap_or("unknown")
    );

    // Show the updated name
    if let Some(name) = updated_user.resource().get_name() {
        if let Some(formatted) = name.formatted.as_ref() {
            println!("   📝 New formatted name: {}", formatted);
        }
    }

    println!("\n📞 Working with phone numbers...");

    // Demonstrate working with phone numbers
    if let Some(phone_numbers) = updated_user.resource().get_phone_numbers() {
        println!("📱 User has {} phone numbers:", phone_numbers.len());
        for phone in phone_numbers {
            let phone_type = phone.phone_type.as_ref().map_or("unknown", |v| v);
            println!("   - {}: {}", phone_type, phone.value);
        }
    } else {
        println!("📱 User has no phone numbers");
    }

    println!("\n📧 Working with email addresses...");

    // Demonstrate working with emails
    if let Some(emails) = updated_user.resource().get_emails() {
        println!("📧 User has {} email addresses:", emails.len());
        for email in emails {
            let email_type = email.email_type.as_ref().map_or("unknown", |v| v);
            let is_primary = email.primary.unwrap_or(false);
            println!(
                "   - {}: {} (primary: {})",
                email_type, email.value, is_primary
            );
        }
    }

    println!("\n🗑️  Testing resource existence and deletion...");

    // Check if user exists
    let user2_id = user2.get_id().unwrap();
    let exists = provider.resource_exists("User", user2_id, &context).await?;
    println!("✅ User {} exists: {}", user2_id, exists);

    // Delete the second user
    provider
        .delete_resource("User", user2_id, None, &context)
        .await?;
    println!("✅ Deleted user");

    // Check if user still exists
    let exists_after = provider.resource_exists("User", user2_id, &context).await?;
    println!(
        "✅ User {} exists after deletion: {}",
        user2_id, exists_after
    );

    // List users again to confirm deletion
    let users_after = provider.list_resources("User", None, &context).await?;
    println!("📊 Users remaining after deletion: {}", users_after.len());

    println!("\n📊 Provider statistics...");

    // Get provider statistics
    let stats = provider.get_stats().await;
    println!("📈 Provider Statistics:");
    println!("   • Total tenants: {}", stats.tenant_count);
    println!("   • Total resources: {}", stats.total_resources);
    println!("   • Resource types: {:?}", stats.resource_types);
    println!("   • Resource type count: {}", stats.resource_type_count);

    println!("\n🧹 Testing clear functionality...");

    // Test clear functionality
    provider.clear().await;
    let stats_after_clear = provider.get_stats().await;
    println!("📈 Statistics after clear:");
    println!("   • Total tenants: {}", stats_after_clear.tenant_count);
    println!(
        "   • Total resources: {}",
        stats_after_clear.total_resources
    );

    println!("\n✅ Basic Usage Example Complete!");
    println!("🎉 Successfully demonstrated:");
    println!("   • Creating resources with StandardResourceProvider");
    println!("   • Listing and searching resources");
    println!("   • Updating and deleting resources");
    println!("   • Working with complex attributes (emails, phone numbers)");
    println!("   • Provider statistics and resource existence checks");
    println!("   • Clear functionality for testing");

    Ok(())
}
