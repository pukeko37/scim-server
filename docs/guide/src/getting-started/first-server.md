# Your First SCIM Server

Learn to build a working SCIM server in 10 minutes using this library.

## Quick Start

### 1. Create a New Project
```bash
cargo new my-scim-server
cd my-scim-server
```

### 2. Add Dependencies
```toml
[dependencies]
scim-server = "0.3.11"
tokio = { version = "1.0", features = ["full"] }
serde_json = "1.0"
```

### 3. Basic Server (15 lines)
```rust
use scim_server::{
    providers::StandardResourceProvider,
    storage::InMemoryStorage,
    RequestContext,
};
use serde_json::json;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create storage and provider - the foundation of your SCIM server
    let storage = InMemoryStorage::new();  // Simple storage for development
    let provider = StandardResourceProvider::new(storage);  // Main SCIM interface

    // Create a single-tenant request context - tracks this operation for logging
    let context = RequestContext::new("demo".to_string());
    let user_data = json!({
        "userName": "john.doe",
        "emails": [{"value": "john@example.com", "primary": true}],
        "active": true
    });

    let user = provider.create_resource("User", user_data, &context).await?;
    println!("Created user: {}", user.get_username().unwrap());

    Ok(())
}
```

### 4. Run It
```bash
cargo run
# Output: Created user: john.doe
```

## Core Operations

### Setup
For the following examples, we'll use this provider and context setup:

```rust
use scim_server::{
    providers::StandardResourceProvider,
    storage::InMemoryStorage,
    RequestContext,
};
use serde_json::json;

// Create storage and provider - the foundation of your SCIM server
let storage = InMemoryStorage::new();  // Simple storage for development
let provider = StandardResourceProvider::new(storage);  // Main SCIM interface

// Single-tenant RequestContext tracks each operation for logging
let context = RequestContext::new("demo".to_string());
```

All the following examples will use these `provider` and `context` variables.

### Creating Resources
```rust
// Use JSON to define user attributes following SCIM 2.0 schema
let user_data = json!({
    "userName": "alice.smith",
    "name": {
        "givenName": "Alice",
        "familyName": "Smith"
    },
    "emails": [{"value": "alice@company.com", "primary": true}],
    "active": true
});

// Create the user - provider handles validation and storage
let user = provider.create_resource("User", user_data, &context).await?;
let user_id = user.get_id().unwrap();  // Get the auto-generated unique ID
```

### Reading Resources
```rust
// Get user by ID - returns Option<Resource> (None if not found)
let retrieved_user = provider.get_resource("User", &user_id, &context).await?;

if let Some(user) = retrieved_user {
    println!("Found: {}", user.get_username().unwrap());
}

// Search by specific attribute value - useful for username lookups
let found_user = provider
    .find_resource_by_attribute("User", "userName", &json!("alice.smith"), &context)
    .await?;
```

### Updating Resources
```rust
// Updates require the full resource data, including the ID
let update_data = json!({
    "id": user_id,
    "userName": "alice.smith",
    "name": {
        "givenName": "Alice",
        "familyName": "Johnson"  // Changed surname
    },
    "emails": [{"value": "alice@company.com", "primary": true}],
    "active": false  // Deactivated
});

// Update replaces the entire resource with new data
let updated_user = provider
    .update_resource("User", &user_id, update_data, &context)
    .await?;
```

### Listing and Searching
```rust
// List all users - None means no pagination/filtering
let all_users = provider.list_resources("User", None, &context).await?;
println!("Total users: {}", all_users.len());

// Efficiently check existence without retrieving full data
let exists = provider.resource_exists("User", &user_id, &context).await?;
println!("User exists: {}", exists);
```

### Validation and Error Handling
```rust
// The provider automatically validates data against SCIM schemas
let invalid_user = json!({
    "userName": "",  // Empty username - violates SCIM requirements
    "emails": [{"value": "not-an-email"}],  // Invalid email format
});

// Always handle validation errors gracefully
match provider.create_resource("User", invalid_user, &context).await {
    Ok(user) => println!("User created: {}", user.get_id().unwrap()),
    Err(e) => println!("Validation failed: {}", e),  // Detailed error message
}
```

### Deleting Resources
```rust
// Delete a resource by ID
provider.delete_resource("User", &user_id, &context).await?;

// Verify deletion
let exists = provider.resource_exists("User", &user_id, &context).await?;
println!("User still exists: {}", exists); // Should be false
```

## Working with Groups

```rust
// Groups can contain users as members - useful for access control
// Create a group (assuming you have a user_id from previous examples)
let group_data = json!({
    "displayName": "Engineering Team",  // Required: human-readable name
    "members": [  // Optional: list of member references
        {
            "value": user_id,  // Reference to the user's ID
            "$ref": format!("https://example.com/v2/Users/{}", user_id),  // Full URI
            "type": "User"  // Type of the referenced resource
        }
    ]
});

// Using the context and user_id from previous examples
// Create group just like users - same provider interface
let group = provider.create_resource("Group", group_data, &context).await?;
println!("Created group: {}", group.get_attribute("displayName").unwrap());
```

## Multi-Tenant Support

For multi-tenant scenarios, you create explicit tenant contexts instead of using the default single-tenant setup:

```rust
// Import TenantContext for multi-tenant operations
use scim_server::resource::TenantContext;

// Create the same provider as before
let storage = InMemoryStorage::new();
let provider = StandardResourceProvider::new(storage);

// Multi-tenant contexts - each gets isolated data space
let tenant_a = TenantContext::new("company-a".to_string(), "client-123".to_string());
let tenant_a_context = RequestContext::with_tenant("req-a".to_string(), tenant_a);

let tenant_b = TenantContext::new("company-b".to_string(), "client-456".to_string());
let tenant_b_context = RequestContext::with_tenant("req-b".to_string(), tenant_b);

// Same provider, different tenants - data is completely isolated
provider.create_resource("User", user_data.clone(), &tenant_a_context).await?;
provider.create_resource("User", user_data, &tenant_b_context).await?;

// Each tenant sees only their own data
let tenant_a_users = provider.list_resources("User", None, &tenant_a_context).await?;
let tenant_b_users = provider.list_resources("User", None, &tenant_b_context).await?;

println!("Company A users: {}", tenant_a_users.len());
println!("Company B users: {}", tenant_b_users.len());
```

## Provider Statistics

```rust
// Useful for monitoring and debugging your SCIM server
let stats = provider.get_stats().await;
println!("Total tenants: {}", stats.tenant_count);  // Number of active tenants
println!("Total resources: {}", stats.total_resources);  // Users + Groups + etc.
println!("Resource types: {:?}", stats.resource_types);  // ["User", "Group", ...]
```

## Next Steps

- **[HTTP Server Integration](../http/overview.md)** - Add REST endpoints with Axum or Actix
- **[Multi-tenant Setup](../multi-tenant/basics.md)** - Advanced tenant isolation and management
- **[Advanced Features](../advanced/overview.md)** - Groups, custom schemas, bulk operations
- **[Storage Backends](../storage/overview.md)** - PostgreSQL, SQLite, and custom storage

## Complete Examples

See the [examples directory](../../../../examples/) for full working implementations:

- **[basic_usage.rs](../../../../examples/basic_usage.rs)** - Complete CRUD operations
- **[group_example.rs](../../../../examples/group_example.rs)** - Group management with members
- **[multi_tenant_example.rs](../../../../examples/multi_tenant_example.rs)** - Tenant isolation patterns

## Running Examples

```bash
# Run any example to see it in action
cargo run --example basic_usage
cargo run --example group_example
```

## Key Concepts

- **`StandardResourceProvider`** - Main interface for SCIM operations
- **`InMemoryStorage`** - Simple storage backend for development
- **`RequestContext`** - Request tracking and tenant isolation
- **Resource Types** - "User", "Group", or custom types
- **JSON Data** - All resource data uses `serde_json::Value`

You now have a working SCIM server! The examples above demonstrate all core functionality needed for most SCIM implementations.