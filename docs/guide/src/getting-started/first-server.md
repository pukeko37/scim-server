# Your First SCIM Server

This tutorial walks you through creating your first SCIM server from scratch. By the end, you'll have a working SCIM server that can manage users and groups with full CRUD operations.

## What We'll Build

We'll create a simple SCIM server that:
- Manages users and groups
- Supports basic CRUD operations
- Uses in-memory storage for simplicity
- Includes proper error handling
- Demonstrates multi-tenant capabilities

## Step 1: Project Setup

First, create a new Rust project:

```bash
cargo new my-scim-server
cd my-scim-server
```

Add the required dependencies to your `Cargo.toml`:

```toml
[dependencies]
scim-server = "0.3.2"
tokio = { version = "1.0", features = ["full"] }
serde_json = "1.0"
anyhow = "1.0"
```

## Step 2: Basic Server Setup

Create your first SCIM server in `src/main.rs`:

```rust
use scim_server::{
    ScimServer,
    providers::StandardResourceProvider,
    storage::InMemoryStorage,
    resource::{RequestContext, ResourceProvider},
    resource_handlers::{create_user_resource_handler, create_group_resource_handler},
    schema::SchemaRegistry,
    resource::ScimOperation,
};
use serde_json::json;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 Starting SCIM Server...");
    
    // Create storage backend
    let storage = InMemoryStorage::new();
    
    // Create resource provider with storage
    let provider = StandardResourceProvider::new(storage);
    
    // Create SCIM server with provider
    let mut server = ScimServer::new(provider)?;
    
    // Register User resource type
    let user_schema = SchemaRegistry::new()?.get_core_user_schema()?;
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
    
    // Register Group resource type
    let group_schema = SchemaRegistry::new()?.get_core_group_schema()?;
    let group_handler = create_group_resource_handler(group_schema);
    server.register_resource_type(
        "Group",
        group_handler,
        vec![
            ScimOperation::Create,
            ScimOperation::Read,
            ScimOperation::Update,
            ScimOperation::Delete,
            ScimOperation::List,
            ScimOperation::Search,
        ],
    )?;
    
    println!("✅ SCIM Server initialized successfully!");
    
    // We'll add operations here in the next steps
    
    Ok(())
}
```

Run this to verify everything works:

```bash
cargo run
```

You should see:
```
🚀 Starting SCIM Server...
✅ SCIM Server initialized successfully!
```

## Step 3: Creating Your First User

Now let's create a user. Add this after the server initialization:

```rust
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // ... previous setup code ...
    
    println!("✅ SCIM Server initialized successfully!");
    
    // Create a request context for our operations
    let context = RequestContext::with_generated_id();
    
    println!("\n📝 Creating a user...");
    
    // Define user data
    let user_data = json!({
        "userName": "alice@example.com",
        "name": {
            "formatted": "Alice Smith",
            "familyName": "Smith",
            "givenName": "Alice"
        },
        "displayName": "Alice Smith",
        "emails": [
            {
                "value": "alice@example.com",
                "type": "work",
                "primary": true
            }
        ],
        "phoneNumbers": [
            {
                "value": "+1-555-123-4567",
                "type": "work"
            }
        ],
        "active": true
    });
    
    // Create the user
    let user = server.create_resource("User", user_data, &context).await?;
    
    println!("✅ Created user: {} (ID: {})", 
             user.get_username().unwrap_or("unknown"),
             user.get_id().unwrap_or("unknown"));
    
    Ok(())
}
```

Run this:

```bash
cargo run
```

You should see:
```
🚀 Starting SCIM Server...
✅ SCIM Server initialized successfully!

📝 Creating a user...
✅ Created user: alice@example.com (ID: user_abc123)
```

## Step 4: Reading and Updating Users

Let's add operations to read and update users:

```rust
// ... after creating the user ...

println!("\n📖 Reading the user...");

// Get the user by ID
let user_id = user.get_id().unwrap();
let retrieved_user = server.get_resource("User", user_id, &context).await?;

match retrieved_user {
    Some(user) => {
        println!("✅ Retrieved user: {} ({})", 
                 user.get_username().unwrap_or("unknown"),
                 user.get_display_name().unwrap_or("unknown"));
    }
    None => {
        println!("❌ User not found");
    }
}

println!("\n📝 Updating the user...");

// Update user data
let updated_data = json!({
    "id": user_id,
    "userName": "alice@example.com",
    "name": {
        "formatted": "Alice Johnson",
        "familyName": "Johnson",  // Changed last name
        "givenName": "Alice"
    },
    "displayName": "Alice Johnson",
    "emails": [
        {
            "value": "alice@example.com",
            "type": "work",
            "primary": true
        },
        {
            "value": "alice.johnson@personal.com",  // Added personal email
            "type": "home",
            "primary": false
        }
    ],
    "active": true
});

// Update the user
let updated_user = server.update_resource("User", user_id, updated_data, &context).await?;

println!("✅ Updated user: {} ({})", 
         updated_user.get_username().unwrap_or("unknown"),
         updated_user.get_display_name().unwrap_or("unknown"));
```

## Step 5: Working with Groups

Let's create a group and manage membership:

```rust
// ... after updating the user ...

println!("\n👥 Creating a group...");

let group_data = json!({
    "displayName": "Engineering Team",
    "members": [
        {
            "value": user_id,
            "display": "Alice Johnson",
            "type": "User"
        }
    ]
});

// Create the group
let group = server.create_resource("Group", group_data, &context).await?;

println!("✅ Created group: {} (ID: {})", 
         group.get_display_name().unwrap_or("unknown"),
         group.get_id().unwrap_or("unknown"));
```

## Step 6: Listing Resources

Add functionality to list users and groups:

```rust
// ... after creating the group ...

println!("\n📋 Listing all users...");

// List all users
let users = server.list_resources("User", None, &context).await?;
println!("✅ Found {} users:", users.len());
for user in &users {
    println!("  - {} ({})", 
             user.get_username().unwrap_or("unknown"),
             user.get_display_name().unwrap_or("unknown"));
}

println!("\n📋 Listing all groups...");

// List all groups
let groups = server.list_resources("Group", None, &context).await?;
println!("✅ Found {} groups:", groups.len());
for group in &groups {
    println!("  - {} (ID: {})", 
             group.get_display_name().unwrap_or("unknown"),
             group.get_id().unwrap_or("unknown"));
}
```

## Step 7: Search Operations

Add search functionality:

```rust
// ... after listing resources ...

println!("\n🔍 Searching for users...");

// Search for user by username
let found_user = server.find_resource_by_attribute(
    "User",
    "userName",
    &json!("alice@example.com"),
    &context,
).await?;

match found_user {
    Some(user) => {
        println!("✅ Found user by username: {} ({})", 
                 user.get_username().unwrap_or("unknown"),
                 user.get_display_name().unwrap_or("unknown"));
    }
    None => {
        println!("❌ User not found");
    }
}
```

## Step 8: Multi-Tenant Operations

Finally, let's demonstrate multi-tenant capabilities:

```rust
use scim_server::TenantContext;

// ... after search operations ...

println!("\n🏢 Multi-tenant operations...");

// Create tenant-specific context
let tenant_context = TenantContext::new(
    "company-123".to_string(),
    "app-456".to_string(),
);
let tenant_request_context = RequestContext::with_tenant_generated_id(tenant_context);

// Create user in specific tenant
let tenant_user_data = json!({
    "userName": "bob@company123.com",
    "name": {
        "formatted": "Bob Wilson",
        "familyName": "Wilson",
        "givenName": "Bob"
    },
    "displayName": "Bob Wilson",
    "active": true
});

let tenant_user = server.create_resource("User", tenant_user_data, &tenant_request_context).await?;

println!("✅ Created user in tenant: {} (ID: {})", 
         tenant_user.get_username().unwrap_or("unknown"),
         tenant_user.get_id().unwrap_or("unknown"));

// Show tenant isolation
let default_users = server.list_resources("User", None, &context).await?;
let tenant_users = server.list_resources("User", None, &tenant_request_context).await?;

println!("📊 Default tenant has {} users", default_users.len());
println!("📊 Company-123 tenant has {} users", tenant_users.len());
println!("✅ Tenant isolation working correctly!");
```

## Complete Example

Here's the complete working example:

```rust
use scim_server::{
    ScimServer,
    providers::StandardResourceProvider,
    storage::InMemoryStorage,
    resource::{RequestContext, TenantContext, ResourceProvider},
    resource_handlers::{create_user_resource_handler, create_group_resource_handler},
    schema::SchemaRegistry,
    resource::ScimOperation,
};
use serde_json::json;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 Starting SCIM Server...");
    
    // Create storage backend
    let storage = InMemoryStorage::new();
    
    // Create resource provider with storage
    let provider = StandardResourceProvider::new(storage);
    
    // Create SCIM server with provider
    let mut server = ScimServer::new(provider)?;
    
    // Register User resource type
    let user_schema = SchemaRegistry::new()?.get_core_user_schema()?;
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
    
    // Register Group resource type
    let group_schema = SchemaRegistry::new()?.get_core_group_schema()?;
    let group_handler = create_group_resource_handler(group_schema);
    server.register_resource_type(
        "Group",
        group_handler,
        vec![
            ScimOperation::Create,
            ScimOperation::Read,
            ScimOperation::Update,
            ScimOperation::Delete,
            ScimOperation::List,
            ScimOperation::Search,
        ],
    )?;
    
    println!("✅ SCIM Server initialized successfully!");
    
    // Create request context
    let context = RequestContext::with_generated_id();
    
    // Create a user
    let user_data = json!({
        "userName": "alice@example.com",
        "name": {
            "formatted": "Alice Smith",
            "familyName": "Smith",
            "givenName": "Alice"
        },
        "displayName": "Alice Smith",
        "emails": [
            {
                "value": "alice@example.com",
                "type": "work",
                "primary": true
            }
        ],
        "active": true
    });
    
    let user = server.create_resource("User", user_data, &context).await?;
    let user_id = user.get_id().unwrap();
    
    println!("✅ Created user: {} (ID: {})", 
             user.get_username().unwrap_or("unknown"), user_id);
    
    // Create a group
    let group_data = json!({
        "displayName": "Engineering Team",
        "members": [
            {
                "value": user_id,
                "display": "Alice Smith",
                "type": "User"
            }
        ]
    });
    
    let group = server.create_resource("Group", group_data, &context).await?;
    
    println!("✅ Created group: {} (ID: {})", 
             group.get_display_name().unwrap_or("unknown"),
             group.get_id().unwrap_or("unknown"));
    
    // List resources
    let users = server.list_resources("User", None, &context).await?;
    let groups = server.list_resources("Group", None, &context).await?;
    
    println!("📊 Total: {} users, {} groups", users.len(), groups.len());
    
    println!("🎉 SCIM Server example completed successfully!");
    
    Ok(())
}
```

## Next Steps

Now that you have a working SCIM server, you can:

1. **[Add Authentication](../tutorials/authentication-setup.md)** - Secure your SCIM endpoints
2. **[Implement Custom Resources](../tutorials/custom-resources.md)** - Extend beyond Users and Groups
3. **[Deploy for Production](../advanced/production-deployment.md)** - Scale your SCIM server
4. **[Add Database Storage](../providers/basic.md)** - Replace in-memory storage with persistence
5. **[Set up Multi-Tenancy](../tutorials/multi-tenant-deployment.md)** - Support multiple customers

## Troubleshooting

### Common Issues

**Compilation Errors**
- Make sure you're using the correct version: `scim-server = "0.3.2"`
- Ensure all required features are enabled

**Runtime Errors**
- Check that all resource types are registered before use
- Verify request contexts are properly created

**Resource Not Found**
- Ensure you're using the correct tenant context
- Check that the resource was created successfully

For more help, see the [Troubleshooting Guide](../how-to/troubleshooting.md).