# Basic Operations

This guide covers the fundamental SCIM operations you'll use most frequently. After reading this, you'll understand how to perform all basic CRUD operations and work with SCIM resources effectively.

## Overview

SCIM (System for Cross-domain Identity Management) defines standard operations for managing identity resources. SCIM Server implements all core operations:

- **Create** - Add new resources
- **Read** - Retrieve existing resources
- **Update** - Modify existing resources
- **Delete** - Remove resources
- **List** - Query multiple resources with filtering and pagination

## Prerequisites

Before starting, ensure you have:
- SCIM Server installed and configured
- Basic understanding of JSON structure
- Familiarity with async/await in Rust

## Basic Setup

All examples assume this basic setup:

```rust
use scim_server::{
    providers::StandardResourceProvider,
    storage::InMemoryStorage,
    RequestContext,
};
use serde_json::json;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let storage = InMemoryStorage::new();
    let provider = StandardResourceProvider::new(storage);
    let context = RequestContext::new("my-request".to_string());
    
    // Operations go here...
    
    Ok(())
}
```

## User Operations

### Creating Users

The most basic operation is creating a user:

```rust
// Minimal user creation
let user_data = json!({
    "userName": "alice@example.com"
});

let user = provider.create_resource("User", user_data, &context).await?;
println!("Created user: {} with ID: {}", 
         user.get_username().unwrap_or("unknown"),
         user.get_id().unwrap_or("unknown"));
```

**Complete user with all common fields:**

```rust
let full_user_data = json!({
    "userName": "alice@example.com",
    "name": {
        "givenName": "Alice",
        "familyName": "Smith",
        "middleName": "M",
        "honorificPrefix": "Ms.",
        "honorificSuffix": "PhD"
    },
    "displayName": "Alice Smith",
    "nickName": "Ally",
    "emails": [
        {
            "value": "alice@example.com",
            "type": "work",
            "primary": true
        },
        {
            "value": "alice.personal@gmail.com",
            "type": "home",
            "primary": false
        }
    ],
    "phoneNumbers": [
        {
            "value": "+1-555-123-4567",
            "type": "work",
            "primary": true
        }
    ],
    "addresses": [
        {
            "type": "work",
            "streetAddress": "123 Business St",
            "locality": "Springfield",
            "region": "IL",
            "postalCode": "62701",
            "country": "US",
            "primary": true
        }
    ],
    "active": true,
    "title": "Senior Developer",
    "userType": "Employee",
    "preferredLanguage": "en-US",
    "locale": "en-US",
    "timezone": "America/Chicago"
});

let full_user = provider.create_resource("User", full_user_data, &context).await?;

// Access user data using typed methods
if let Some(name) = full_user.get_name() {
    println!("Created user: {} {}", 
             name.given_name.as_ref().unwrap_or(&"".to_string()),
             name.family_name.as_ref().unwrap_or(&"".to_string()));
}
```

### Reading Users

**Get a specific user by ID:**

```rust
let user = provider.get_resource("User", &user_id, &context).await?;
println!("User: {}", user.get_username().unwrap_or("unknown"));
println!("Active: {}", user.get_active().unwrap_or(false));
if let Some(meta) = user.get_meta() {
    println!("Created: {}", meta.created);
    println!("Version: {}", meta.version.as_ref().unwrap_or(&"unknown".to_string()));
}
```

**Check if user exists:**

```rust
match provider.get_resource("User", &user_id, &context).await {
    Ok(user) => println!("User exists: {}", user.get_username().unwrap_or("unknown")),
    Err(e) if e.to_string().contains("not found") => println!("User not found"),
    Err(e) => println!("Error: {}", e),
}

// Or use the dedicated exists method
let exists = provider.resource_exists("User", &user_id, &context).await?;
println!("User exists: {}", exists);
```

### Updating Users

**Replace entire user (PUT semantics):**

```rust
let updated_data = json!({
    "userName": "alice@example.com",
    "name": {
        "givenName": "Alice",
        "familyName": "Johnson" // Changed last name
    },
    "active": false // Deactivated user
});

let updated_user = provider.update_resource("User", &user_id, updated_data, &context).await?;
println!("Updated user: {}", updated_user.get_username().unwrap_or("unknown"));
```

**Partial update:**

```rust
// Get current user first
let current_user = provider.get_resource("User", &user_id, &context).await?;

// Create updated data with changes
let mut updated_data = current_user.data.clone();
updated_data["active"] = json!(false);
updated_data["title"] = json!("Lead Developer");

let patched_user = provider.update_resource("User", &user_id, updated_data, &context).await?;
```

**Working with ETags for concurrency control:**

```rust
// Get current user to check ETag
let current_user = provider.get_resource("User", &user_id, &context).await?;
if let Some(meta) = current_user.get_meta() {
    println!("Current ETag: {}", meta.version.as_ref().unwrap_or(&"none".to_string()));
}

// Update with the current version
let mut update_data = current_user.data.clone();
update_data["active"] = json!(false);

let updated_user = provider.update_resource("User", &user_id, update_data, &context).await?;

// Check new ETag
if let Some(meta) = updated_user.get_meta() {
    println!("New ETag: {}", meta.version.as_ref().unwrap_or(&"none".to_string()));
}
```

### Deleting Users

**Simple deletion:**

```rust
provider.delete_resource("User", &user_id, &context).await?;
println!("User deleted successfully");
```

**Verify deletion:**

```rust
// Check if user still exists
let exists = provider.resource_exists("User", &user_id, &context).await?;
println!("User exists after deletion: {}", exists);
```

**Safe deletion with existence check:**

```rust
// Check if user exists before deleting
if provider.resource_exists("User", &user_id, &context).await? {
    provider.delete_resource("User", &user_id, &context).await?;
    println!("User deleted successfully");
} else {
    println!("User does not exist");
}
```

## Group Operations

### Creating Groups

**Basic group:**

```rust
let group_data = json!({
    "displayName": "Developers"
});

let group = provider.create_resource("Group", group_data, &context).await?;
println!("Created group: {}", group.get_display_name().unwrap_or("unknown"));
```

**Group with members:**

```rust
// Assuming you have user IDs from previous operations
let group_with_members = json!({
    "displayName": "Engineering Team",
    "members": [
        {
            "value": user1_id,
            "display": "Alice Smith"
        },
        {
            "value": user2_id,
            "display": "Bob Johnson"
        }
    ]
});

let group = provider.create_resource("Group", group_with_members, &context).await?;
println!("Created group with {} members", 
         group.get_members().map(|m| m.len()).unwrap_or(0));
```

### Managing Group Membership

**Add user to group:**

```rust
// Get current group
let mut group = provider.get_resource("Group", &group_id, &context).await?;

// Add member to the group data
let mut group_data = group.data.clone();
let mut members = group_data.get("members").unwrap_or(&json!([])).as_array().unwrap().clone();
members.push(json!({
    "value": user_id,
    "display": "User Display Name"
}));
group_data["members"] = json!(members);

// Update the group
let updated_group = provider.update_resource("Group", &group_id, group_data, &context).await?;
```

**Remove user from group:**

```rust
// Get current group
let mut group = provider.get_resource("Group", &group_id, &context).await?;

// Remove member from the group data
let mut group_data = group.data.clone();
if let Some(members) = group_data.get_mut("members") {
    if let Some(members_array) = members.as_array_mut() {
        members_array.retain(|member| {
            member.get("value").and_then(|v| v.as_str()) != Some(&user_id)
        });
        group_data["members"] = json!(members_array);
    }
}

// Update the group
let updated_group = provider.update_resource("Group", &group_id, group_data, &context).await?;
```

**Get group members:**

```rust
let group = provider.get_resource("Group", &group_id, &context).await?;
if let Some(members) = group.get_members() {
    println!("Group has {} members:", members.len());
    for member in members {
        println!("  - {} ({})", 
                 member.display.as_ref().unwrap_or(&"unknown".to_string()), 
                 member.value);
    }
}
```

## Listing and Querying

### Basic Listing

**List all users:**

```rust
let users = provider.list_resources("User", None, &context).await?;
println!("Found {} users", users.len());

for user in users {
    println!("  - {} ({})", 
             user.get_username().unwrap_or("unknown"),
             user.get_id().unwrap_or("unknown"));
}
```

**List all groups:**

```rust
let groups = provider.list_resources("Group", None, &context).await?;
println!("Found {} groups", groups.len());

for group in groups {
    println!("  - {} ({})",
             group.get_display_name().unwrap_or("unknown"),
             group.get_id().unwrap_or("unknown"));
}
```

### Pagination

**Using provider's built-in pagination:**

```rust
// The StandardResourceProvider handles pagination internally
// For large datasets, you can implement pagination by querying in chunks

let all_users = provider.list_resources("User", None, &context).await?;
println!("Total users: {}", all_users.len());

// For manual pagination, you can filter results
let first_10_users: Vec<_> = all_users.into_iter().take(10).collect();
println!("First 10 users:");
for user in first_10_users {
    println!("  - {}", user.get_username().unwrap_or("unknown"));
}
```

**Working with large datasets:**

```rust
// For very large datasets, consider implementing pagination at the storage level
// This example shows conceptual pagination handling

async fn get_users_page(
    provider: &StandardResourceProvider<InMemoryStorage>,
    context: &RequestContext,
    page: usize,
    page_size: usize
) -> Result<Vec<ScimResource>, Box<dyn std::error::Error>> {
    let all_users = provider.list_resources("User", None, context).await?;
    let start = page * page_size;
    let end = std::cmp::min(start + page_size, all_users.len());
    
    if start >= all_users.len() {
        return Ok(Vec::new());
    }
    
    Ok(all_users[start..end].to_vec())
}

// Example usage:
let page_0 = get_users_page(&provider, &context, 0, 10).await?;
let page_1 = get_users_page(&provider, &context, 1, 10).await?;
```

### Filtering

The StandardResourceProvider supports attribute-based filtering:

**Basic attribute filtering:**

```rust
// Find user by username
let user = provider.find_resource_by_attribute(
    "User",
    "userName", 
    &json!("alice@example.com"),
    &context
).await?;

if let Some(user) = user {
    println!("Found user: {}", user.get_username().unwrap_or("unknown"));
}
```

**Find by email:**

```rust
// Find user by email address
let user_by_email = provider.find_resource_by_attribute(
    "User",
    "emails.value",
    &json!("alice@example.com"),
    &context
).await?;
```

**Client-side filtering for complex queries:**

```rust
// Get all users and filter on the client side
let all_users = provider.list_resources("User", None, &context).await?;

// Filter active users
let active_users: Vec<_> = all_users.into_iter()
    .filter(|user| user.get_active().unwrap_or(false))
    .collect();

println!("Found {} active users", active_users.len());
```

**Filter by email domain:**

```rust
let all_users = provider.list_resources("User", None, &context).await?;

let company_users: Vec<_> = all_users.into_iter()
    .filter(|user| {
        if let Some(emails) = user.get_emails() {
            emails.iter().any(|email| email.value.contains("@company.com"))
        } else {
            false
        }
    })
    .collect();

println!("Found {} company users", company_users.len());
```

### Sorting

**Client-side sorting:**

```rust
let all_users = provider.list_resources("User", None, &context).await?;

// Sort by username
let mut sorted_users = all_users;
sorted_users.sort_by(|a, b| {
    let a_name = a.get_username().unwrap_or("");
    let b_name = b.get_username().unwrap_or("");
    a_name.cmp(b_name)
});

println!("Users sorted by username:");
for user in sorted_users.iter().take(5) {
    println!("  - {}", user.get_username().unwrap_or("unknown"));
}
```

**Sort by creation date:**

```rust
let all_users = provider.list_resources("User", None, &context).await?;

// Sort by creation date (newest first)
let mut users_by_date = all_users;
users_by_date.sort_by(|a, b| {
    let a_created = a.get_meta().and_then(|m| m.created.as_ref());
    let b_created = b.get_meta().and_then(|m| m.created.as_ref());
    b_created.cmp(&a_created) // Reverse for newest first
});
```

## Bulk Operations

The StandardResourceProvider processes operations individually. For bulk efficiency, use async iteration:

**Create multiple users:**

```rust
let bulk_users = vec![
    json!({
        "userName": "user1@example.com",
        "active": true
    }),
    json!({
        "userName": "user2@example.com", 
        "active": true
    }),
    json!({
        "userName": "user3@example.com",
        "active": true
    })
];

// Create users concurrently
let mut created_users = Vec::new();
for user_data in bulk_users {
    match provider.create_resource("User", user_data, &context).await {
        Ok(user) => {
            println!("Created user: {}", user.get_username().unwrap_or("unknown"));
            created_users.push(user);
        },
        Err(e) => {
            println!("Failed to create user: {}", e);
        }
    }
}

println!("Successfully created {} users", created_users.len());
```

**Bulk update multiple users:**

```rust
let user_ids = vec!["user1", "user2", "user3"];

for user_id in user_ids {
    // Get current user
    if let Ok(mut user) = provider.get_resource("User", user_id, &context).await {
        // Update the user data
        let mut user_data = user.data.clone();
        user_data["active"] = json!(false);
        
        match provider.update_resource("User", user_id, user_data, &context).await {
            Ok(_) => println!("Updated user: {}", user_id),
            Err(e) => println!("Failed to update user {}: {}", user_id, e),
        }
    }
}
```

## Error Handling Patterns

### Comprehensive Error Handling

```rust
async fn safe_user_operation(
    provider: &StandardResourceProvider<InMemoryStorage>,
    context: &RequestContext,
    user_id: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    match provider.get_resource("User", user_id, context).await {
        Ok(user) => {
            println!("Found user: {}", user.get_username().unwrap_or("unknown"));
            Ok(())
        },
        Err(e) if e.to_string().contains("not found") => {
            println!("User {} not found", user_id);
            Err("User not found".into())
        },
        Err(e) if e.to_string().contains("validation") => {
            println!("Validation failed: {}", e);
            Err("Invalid data".into())
        },
        Err(e) if e.to_string().contains("conflict") => {
            println!("Conflict detected for resource: {}", user_id);
            Err("Resource conflict".into())
        },
        Err(e) if e.to_string().contains("storage") => {
            println!("Storage error: {}", e);
            Err("Storage issue".into())
        },
        Err(e) => {
            println!("Unexpected error: {}", e);
            Err(e.into())
        }
    }
}
```

### Retry Logic for Conflicts

```rust
use tokio::time::{sleep, Duration};

async fn retry_update_user(
    provider: &StandardResourceProvider<InMemoryStorage>,
    context: &RequestContext,
    user_id: &str,
    update_data: serde_json::Value,
    max_retries: u32,
) -> Result<ScimResource, Box<dyn std::error::Error>> {
    for attempt in 0..max_retries {
        // Get current user and version
        let current_user = provider.get_resource("User", user_id, context).await?;
        let current_version = current_user.get_meta()
            .and_then(|m| m.version.as_ref())
            .cloned();
        
        // Attempt update
        match provider.update_resource("User", user_id, update_data.clone(), context).await {
            Ok(updated_user) => {
                println!("Successfully updated user after {} attempt(s)", attempt + 1);
                return Ok(updated_user);
            },
            Err(e) if e.to_string().contains("conflict") => {
                if attempt < max_retries - 1 {
                    // Exponential backoff
                    let delay = Duration::from_millis(100 * 2_u64.pow(attempt));
                    println!("Conflict detected, retrying in {:?}...", delay);
                    sleep(delay).await;
                    continue;
                } else {
                    return Err(format!("Max retries ({}) exceeded due to conflicts", max_retries).into());
                }
            },
            Err(e) => return Err(e.into()),
        }
    }
    
    unreachable!()
}

// Example usage:
let update_data = json!({ "active": false });
match retry_update_user(&provider, &context, "user123", update_data, 3).await {
    Ok(user) => println!("Updated user successfully"),
    Err(e) => println!("Failed to update user: {}", e),
}
```

## Best Practices

### 1. Always Use Request Context

```rust
// Good: Always provide request context
let context = RequestContext::new("operation-123".to_string());
let user = provider.get_resource("User", &user_id, &context).await?;

// The request context provides operation tracking and audit trails
```

### 2. Handle Versions for Updates

```rust
// Good: Check versions for safe updates
let current_user = provider.get_resource("User", &user_id, &context).await?;
if let Some(meta) = current_user.get_meta() {
    println!("Current version: {}", meta.version.as_ref().unwrap_or(&"none".to_string()));
}

// Update with current data
let mut update_data = current_user.data.clone();
update_data["active"] = json!(false);
let result = provider.update_resource("User", &user_id, update_data, &context).await?;

// Check new version
if let Some(meta) = result.get_meta() {
    println!("New version: {}", meta.version.as_ref().unwrap_or(&"none".to_string()));
}
```

### 3. Use Appropriate Operations

```rust
// For creating new resources
let user = provider.create_resource("User", user_data, &context).await?;

// For updating existing resources
let updated_user = provider.update_resource("User", &user_id, updated_data, &context).await?;

// For retrieving resources
let user = provider.get_resource("User", &user_id, &context).await?;

// For deleting resources
provider.delete_resource("User", &user_id, &context).await?;
```

### 4. Validate Data Before Operations

```rust
fn validate_email(email: &str) -> bool {
    email.contains('@') && email.contains('.')
}

async fn create_user_safely(
    provider: &StandardResourceProvider<InMemoryStorage>,
    context: &RequestContext,
    user_data: serde_json::Value,
) -> Result<ScimResource, Box<dyn std::error::Error>> {
    // Validate email if present
    if let Some(username) = user_data.get("userName").and_then(|v| v.as_str()) {
        if !validate_email(username) {
            return Err("Invalid email format".into());
        }
    }
    
    // Validate required fields
    if user_data.get("userName").is_none() {
        return Err("userName is required".into());
    }
    
    // Create user if validation passes
    provider.create_resource("User", user_data, context).await
        .map_err(|e| e.into())
}
```

### 5. Use Pagination for Large Results

```rust
async fn process_all_users(
    provider: &StandardResourceProvider<InMemoryStorage>,
    context: &RequestContext,
) -> Result<(), Box<dyn std::error::Error>> {
    let all_users = provider.list_resources("User", None, context).await?;
    
    // Process in chunks for memory efficiency
    for chunk in all_users.chunks(100) {
        for user in chunk {
            // Process each user
            println!("Processing user: {}", user.get_username().unwrap_or("unknown"));
        }
        
        // Optional: Add delay between chunks to avoid overwhelming the system
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    }
    
    Ok(())
}
```

## Summary

This guide covered all fundamental SCIM operations using the StandardResourceProvider:

**CRUD Operations**:
- ✅ **Create** resources with `create_resource()`
- ✅ **Read** resources with `get_resource()` and `list_resources()`  
- ✅ **Update** resources with `update_resource()`
- ✅ **Delete** resources with `delete_resource()`

**Advanced Features**:
- ✅ **Attribute search** with `find_resource_by_attribute()`
- ✅ **Resource existence** checks with `resource_exists()`
- ✅ **Version management** with ETag support
- ✅ **Client-side filtering** and sorting
- ✅ **Bulk operations** with async iteration
- ✅ **Error handling** patterns and retry logic

**Key Takeaways**:
1. Always use `RequestContext` for operation tracking
2. Leverage typed methods like `get_username()`, `get_emails()` for safe data access
3. Handle errors gracefully with proper pattern matching
4. Use ETags for concurrency control in multi-client scenarios
5. Implement client-side filtering for complex queries

You're now ready to build robust SCIM applications! Next, explore [Custom Resource Types](../tutorials/custom-resources.md) or [Multi-Tenant Deployment](../tutorials/multi-tenant-deployment.md) for advanced scenarios.