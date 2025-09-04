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
scim-server = "0.4.0"
tokio = { version = "1.0", features = ["full"] }
serde_json = "1.0"
```

### 3. Basic Server (20 lines)
```rust
use scim_server::{
    ScimServer,
    providers::StandardResourceProvider,
    storage::InMemoryStorage,
    resource_handlers::create_user_resource_handler,
    multi_tenant::ScimOperation,
    RequestContext,
};
use serde_json::json;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create storage, provider, and SCIM server
    let storage = InMemoryStorage::new();
    let provider = StandardResourceProvider::new(storage);
    let mut server = ScimServer::new(provider)?;

    // Register User resource type with schema validation
    let user_schema = server.get_schema_by_id("urn:ietf:params:scim:schemas:core:2.0:User")?.clone();
    let user_handler = create_user_resource_handler(user_schema);
    server.register_resource_type("User", user_handler, vec![ScimOperation::Create])?;

    // Create request context and user data
    let context = RequestContext::new("demo".to_string());
    let user_data = json!({
        "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
        "userName": "john.doe",
        "emails": [{"value": "john@example.com", "primary": true}],
        "active": true
    });

    let user_json = server.create_resource_with_refs("User", user_data, &context).await?;
    println!("Created user: {}", user_json["userName"]);

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
For the following examples, we'll use this server and context setup:

```rust
use scim_server::{
    ScimServer,
    providers::StandardResourceProvider,
    storage::InMemoryStorage,
    resource_handlers::{create_user_resource_handler, create_group_resource_handler},
    multi_tenant::ScimOperation,
    RequestContext,
};
use serde_json::json;

// Create storage, provider, and SCIM server
let storage = InMemoryStorage::new();
let provider = StandardResourceProvider::new(storage);
let mut server = ScimServer::new(provider)?;

// Register User and Group resource types with schema validation
let user_schema = server.get_schema_by_id("urn:ietf:params:scim:schemas:core:2.0:User")?.clone();
let user_handler = create_user_resource_handler(user_schema);
server.register_resource_type("User", user_handler,
    vec![ScimOperation::Create, ScimOperation::Read, ScimOperation::Update, ScimOperation::Delete])?;

let group_schema = server.get_schema_by_id("urn:ietf:params:scim:schemas:core:2.0:Group")?.clone();
let group_handler = create_group_resource_handler(group_schema);
server.register_resource_type("Group", group_handler,
    vec![ScimOperation::Create, ScimOperation::Read, ScimOperation::Update, ScimOperation::Delete])?;

// Single-tenant RequestContext tracks each operation for logging
let context = RequestContext::new("demo".to_string());
```

All the following examples will use these `server` and `context` variables.

### Creating Resources
```rust
// Use JSON to define user attributes following SCIM 2.0 schema
let user_data = json!({
    "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
    "userName": "alice.smith",
    "name": {
        "givenName": "Alice",
        "familyName": "Smith"
    },
    "emails": [{"value": "alice@company.com", "primary": true}],
    "active": true
});

// Create the user - server handles validation, $ref fields, and metadata
let user_json = server.create_resource_with_refs("User", user_data, &context).await?;
let user_id = user_json["id"].as_str().unwrap();  // Get the auto-generated unique ID
```

### Reading Resources
```rust
// Get user by ID - returns SCIM-compliant JSON with proper $ref fields
let retrieved_user = server.get_resource("User", &user_id, &context).await?;
println!("Found: {}", retrieved_user["userName"]);

// Search by specific attribute value - useful for username lookups
let search_results = server.search_resources("User", "userName", &json!("alice.smith"), &context).await?;
if !search_results.is_empty() {
    println!("Search found: {}", search_results[0]["userName"]);
}
```

### Updating Resources
```rust
// Updates require the full resource data, including the ID and schemas
let update_data = json!({
    "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
    "id": user_id,
    "userName": "alice.smith",
    "name": {
        "givenName": "Alice",
        "familyName": "Johnson"  // Changed surname
    },
    "emails": [{"value": "alice@company.com", "primary": true}],
    "active": false  // Deactivated
});

// Update replaces the entire resource with new data, maintains SCIM compliance
let updated_user = server.update_resource("User", &user_id, update_data, &context).await?;
```

### Listing and Searching
```rust
// List all users with proper SCIM compliance
let all_users = server.list_resources("User", &context).await?;
println!("Total users: {}", all_users.len());

// Check existence
let exists = server.resource_exists("User", &user_id, &context).await?;
println!("User exists: {}", exists);
```

### Validation and Error Handling
```rust
// The server automatically validates data against SCIM schemas
let invalid_user = json!({
    "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
    "userName": "",  // Empty username - violates SCIM requirements
    "emails": [{"value": "not-an-email"}],  // Invalid email format
});

// Always handle validation errors gracefully
match server.create_resource_with_refs("User", invalid_user, &context).await {
    Ok(user_json) => println!("User created: {}", user_json["id"]),
    Err(e) => println!("Validation failed: {}", e),  // Detailed error message
}
```

### Deleting Resources
```rust
// Delete a resource by ID
server.delete_resource("User", &user_id, &context).await?;

// Verify deletion
let exists = server.resource_exists("User", &user_id, &context).await?;
println!("User still exists: {}", exists); // Should be false
```

## Working with Groups

```rust
// Groups can contain users as members - useful for access control
// Create a group (assuming you have a user_id from previous examples)
let group_data = json!({
    "schemas": ["urn:ietf:params:scim:schemas:core:2.0:Group"],
    "displayName": "Engineering Team",  // Required: human-readable name
    "members": [  // Optional: list of member references
        {
            "value": user_id,  // Reference to the user's ID
            "type": "User"  // Type of the referenced resource
            // Note: $ref field will be automatically generated by the server
        }
    ]
});

// Create group with full SCIM compliance - server will inject proper $ref fields
let group_json = server.create_resource_with_refs("Group", group_data, &context).await?;
println!("Created group: {}", group_json["displayName"]);
println!("Member $ref: {}", group_json["members"][0]["$ref"]);  // Auto-generated!
```

## Multi-Tenant Support

For multi-tenant scenarios, you create explicit tenant contexts instead of using the default single-tenant setup:

```rust
// Import TenantContext for multi-tenant operations
use scim_server::{ScimServerBuilder, TenantStrategy, multi_tenant::TenantContext};

// Create multi-tenant server with proper configuration
let storage = InMemoryStorage::new();
let provider = StandardResourceProvider::new(storage);
let mut server = ScimServerBuilder::new(provider)
    .with_base_url("https://api.company.com")
    .with_tenant_strategy(TenantStrategy::PathBased)
    .build()?;

// Register resource types (same as before)
let user_schema = server.get_schema_by_id("urn:ietf:params:scim:schemas:core:2.0:User")?.clone();
let user_handler = create_user_resource_handler(user_schema);
server.register_resource_type("User", user_handler, vec![ScimOperation::Create])?;

// Multi-tenant contexts - each gets isolated data space
let tenant_a = TenantContext::new("company-a".to_string(), "client-123".to_string());
let tenant_a_context = RequestContext::with_tenant("req-a".to_string(), tenant_a);

let tenant_b = TenantContext::new("company-b".to_string(), "client-456".to_string());
let tenant_b_context = RequestContext::with_tenant("req-b".to_string(), tenant_b);

// Same server, different tenants - data is completely isolated
let user_data = json!({
    "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
    "userName": "john.doe",
    "emails": [{"value": "john@company.com", "primary": true}]
});
server.create_resource_with_refs("User", user_data.clone(), &tenant_a_context).await?;
server.create_resource_with_refs("User", user_data, &tenant_b_context).await?;

// Each tenant sees only their own data
let tenant_a_users = server.list_resources("User", &tenant_a_context).await?;
let tenant_b_users = server.list_resources("User", &tenant_b_context).await?;

println!("Company A users: {}", tenant_a_users.len());
println!("Company B users: {}", tenant_b_users.len());
```

## Provider Statistics

```rust
// Get server information and capabilities
let server_info = server.get_server_info();
println!("Supported resource types: {:?}", server_info.supported_resource_types);
println!("SCIM version: {}", server_info.scim_version);
println!("Server capabilities: {:?}", server_info.capabilities);
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

- **`ScimServer`** - Main interface providing full SCIM 2.0 compliance
- **`StandardResourceProvider`** - Storage abstraction layer
- **`InMemoryStorage`** - Simple storage backend for development
- **`RequestContext`** - Request tracking and tenant isolation
- **Resource Handlers** - Schema validation and business logic
- **Resource Types** - "User", "Group", or custom types registered with schemas
- **JSON Data** - All resource data uses `serde_json::Value`
- **Auto-generated Fields** - Server automatically adds `$ref`, `meta.location`, and other SCIM compliance fields

You now have a working SCIM server! The examples above demonstrate all core functionality needed for SCIM 2.0 compliant implementations.
