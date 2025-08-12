# Tutorial: Building Your First SCIM Server

This tutorial will guide you through building your first SCIM server from scratch using the SCIM Server crate. By the end of this tutorial, you'll have a working SCIM server that can manage users and groups with a web interface for testing.

## What You'll Build

In this tutorial, you'll create:

- A complete SCIM 2.0 compliant server
- User and Group resource management
- A simple web interface for testing
- Sample data to work with
- Proper error handling and logging

## Prerequisites

Before starting, ensure you have:

- Rust 1.70 or later installed
- Basic knowledge of Rust and async programming
- A text editor or IDE
- curl or a REST client for testing

## Step 1: Project Setup

### Create a New Rust Project

```bash
cargo new my-first-scim-server
cd my-first-scim-server
```

### Add Dependencies

Edit `Cargo.toml` to add the required dependencies:

```toml
[package]
name = "my-first-scim-server"
version = "0.1.0"
edition = "2021"

[dependencies]
scim-server = "0.2.1"
tokio = { version = "1.0", features = ["full"] }
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
uuid = { version = "1.0", features = ["v4"] }
chrono = { version = "0.4", features = ["serde"] }

[dev-dependencies]
reqwest = { version = "0.11", features = ["json"] }
```

## Step 2: Basic Server Implementation

### Create the Main Server File

Replace the contents of `src/main.rs`:

```rust
use scim_server::{ScimServer, ServerConfig};
use scim_server::providers::InMemoryProvider;
use scim_server::error::Result;
use tracing::{info, warn, Level};
use tracing_subscriber::{FmtSubscriber, EnvFilter};

#[tokio::main]
async fn main() -> Result<()> {
    // Step 1: Initialize logging
    init_logging();
    
    info!("🚀 Starting My First SCIM Server");
    
    // Step 2: Create storage provider
    let provider = InMemoryProvider::new();
    info!("📦 Created in-memory storage provider");
    
    // Step 3: Configure the server
    let config = ServerConfig::builder()
        .host("localhost")
        .port(8080)
        .base_url("http://localhost:8080/scim/v2")
        .provider(provider)
        .enable_cors(true)
        .cors_origins(vec!["*"]) // Allow all origins for development
        .enable_request_logging(true)
        .build()?;
    
    info!("⚙️  Server configured:");
    info!("   Host: {}", config.host());
    info!("   Port: {}", config.port());
    info!("   Base URL: {}", config.base_url());
    
    // Step 4: Create and start the server
    let server = ScimServer::new(config);
    
    print_welcome_message();
    
    // This will run forever until interrupted
    server.run().await?;
    
    Ok(())
}

fn init_logging() {
    let subscriber = FmtSubscriber::builder()
        .with_env_filter(EnvFilter::from_default_env().add_directive("my_first_scim_server=info".parse().unwrap()))
        .with_target(false)
        .with_thread_names(true)
        .finish();
    
    tracing::subscriber::set_global_default(subscriber)
        .expect("Failed to set default subscriber");
}

fn print_welcome_message() {
    info!("🎉 SCIM Server is starting!");
    info!("");
    info!("📍 Server URL: http://localhost:8080");
    info!("🔗 API Base: http://localhost:8080/scim/v2");
    info!("");
    info!("📚 Available Endpoints:");
    info!("   GET    /scim/v2/Users              - List all users");
    info!("   POST   /scim/v2/Users              - Create a new user");
    info!("   GET    /scim/v2/Users/{{id}}         - Get specific user");
    info!("   PUT    /scim/v2/Users/{{id}}         - Update user");
    info!("   PATCH  /scim/v2/Users/{{id}}         - Partially update user");
    info!("   DELETE /scim/v2/Users/{{id}}         - Delete user");
    info!("");
    info!("   GET    /scim/v2/Groups             - List all groups");
    info!("   POST   /scim/v2/Groups             - Create a new group");
    info!("   GET    /scim/v2/Groups/{{id}}        - Get specific group");
    info!("   PUT    /scim/v2/Groups/{{id}}        - Update group");
    info!("   PATCH  /scim/v2/Groups/{{id}}        - Partially update group");
    info!("   DELETE /scim/v2/Groups/{{id}}        - Delete group");
    info!("");
    info!("🔍 Discovery Endpoints:");
    info!("   GET    /scim/v2/ServiceProviderConfig");
    info!("   GET    /scim/v2/ResourceTypes");
    info!("   GET    /scim/v2/Schemas");
    info!("");
    info!("✅ Server ready to accept requests!");
    info!("💡 Try: curl http://localhost:8080/scim/v2/Users");
}
```

### Test Your Basic Server

Run your server:

```bash
cargo run
```

You should see output similar to:

```
🚀 Starting My First SCIM Server
📦 Created in-memory storage provider
⚙️  Server configured:
   Host: localhost
   Port: 8080
   Base URL: http://localhost:8080/scim/v2
🎉 SCIM Server is starting!
✅ Server ready to accept requests!
```

Test it with curl:

```bash
# In another terminal
curl http://localhost:8080/scim/v2/Users
```

You should get an empty user list:

```json
{
  "schemas": ["urn:ietf:params:scim:api:messages:2.0:ListResponse"],
  "totalResults": 0,
  "Resources": []
}
```

Congratulations! Your SCIM server is working! 🎉

## Step 3: Adding Sample Data

Let's add some sample data to make the server more interesting.

### Create Sample Data Module

Create `src/sample_data.rs`:

```rust
use scim_server::resource::{ResourceBuilder};
use scim_server::resource::value_objects::{
    ResourceId, UserName, EmailAddress, Name, Address, PhoneNumber, GroupMember
};
use scim_server::providers::ResourceProvider;
use scim_server::error::Result;
use tracing::info;

pub async fn populate_sample_data<P: ResourceProvider>(provider: &P) -> Result<()> {
    info!("📊 Populating sample data...");
    
    // Create sample users
    create_sample_users(provider).await?;
    
    // Create sample groups
    create_sample_groups(provider).await?;
    
    info!("✨ Sample data populated successfully!");
    Ok(())
}

async fn create_sample_users<P: ResourceProvider>(provider: &P) -> Result<()> {
    let users = vec![
        // User 1: John Doe (Administrator)
        create_user(
            "user-001",
            "john.doe",
            "John",
            "Doe",
            "john.doe@example.com",
            "Administrator",
            true
        )?,
        
        // User 2: Jane Smith (Developer)  
        create_user(
            "user-002",
            "jane.smith",
            "Jane",
            "Smith",
            "jane.smith@example.com",
            "Senior Developer",
            true
        )?,
        
        // User 3: Bob Wilson (Support)
        create_user(
            "user-003",
            "bob.wilson",
            "Bob",
            "Wilson",
            "bob.wilson@example.com",
            "Support Specialist",
            true
        )?,
        
        // User 4: Alice Brown (Inactive)
        create_user(
            "user-004",
            "alice.brown",
            "Alice",
            "Brown",
            "alice.brown@example.com",
            "Former Employee",
            false
        )?,
    ];

    for user in users {
        provider.create_resource(user).await?;
    }
    
    info!("👥 Created {} sample users", 4);
    Ok(())
}

async fn create_sample_groups<P: ResourceProvider>(provider: &P) -> Result<()> {
    // Administrators group
    let admin_group = ResourceBuilder::new()
        .id(ResourceId::new("group-001")?)
        .display_name("Administrators")
        .add_group_member(
            GroupMember::new_user(ResourceId::new("user-001")?)
                .with_display_name("John Doe")
        )
        .build()?;
    
    // Developers group
    let dev_group = ResourceBuilder::new()
        .id(ResourceId::new("group-002")?)
        .display_name("Developers")
        .add_group_member(
            GroupMember::new_user(ResourceId::new("user-001")?)
                .with_display_name("John Doe")
        )
        .add_group_member(
            GroupMember::new_user(ResourceId::new("user-002")?)
                .with_display_name("Jane Smith")
        )
        .build()?;
    
    // Support Team group
    let support_group = ResourceBuilder::new()
        .id(ResourceId::new("group-003")?)
        .display_name("Support Team")
        .add_group_member(
            GroupMember::new_user(ResourceId::new("user-003")?)
                .with_display_name("Bob Wilson")
        )
        .build()?;

    provider.create_resource(admin_group).await?;
    provider.create_resource(dev_group).await?;
    provider.create_resource(support_group).await?;
    
    info!("👨‍👩‍👧‍👦 Created {} sample groups", 3);
    Ok(())
}

fn create_user(
    id: &str,
    username: &str,
    given_name: &str,
    family_name: &str,
    email: &str,
    title: &str,
    active: bool,
) -> Result<Resource> {
    let name = Name::builder()
        .given_name(given_name)
        .family_name(family_name)
        .formatted(&format!("{} {}", given_name, family_name))
        .build();

    ResourceBuilder::new()
        .id(ResourceId::new(id)?)
        .user_name(UserName::new(username)?)
        .name(name)
        .display_name(&format!("{} {}", given_name, family_name))
        .title(title)
        .add_email(
            EmailAddress::new(email)?
                .with_type("work")
                .with_primary(true)
        )
        .add_phone(
            PhoneNumber::new("+1-555-0100")?
                .with_type("work")
        )
        .add_address(
            Address::builder()
                .street_address("123 Main Street")
                .locality("Anytown")
                .region("CA")
                .postal_code("90210")
                .country("US")
                .type_("work")
                .build()
        )
        .active(active)
        .build()
}
```

### Update Main Function

Update `src/main.rs` to use the sample data:

```rust
mod sample_data;

// Add this after creating the provider and before creating the config
let provider = InMemoryProvider::new();

// Populate with sample data
sample_data::populate_sample_data(&provider).await?;
info!("📊 Sample data loaded");

// Continue with config creation...
```

### Test with Sample Data

Restart your server and test:

```bash
# List users (should now show 4 users)
curl http://localhost:8080/scim/v2/Users

# Get a specific user
curl http://localhost:8080/scim/v2/Users/user-001

# List groups (should show 3 groups)
curl http://localhost:8080/scim/v2/Groups
```

## Step 4: Creating Your First User

Let's create a new user through the API:

```bash
curl -X POST http://localhost:8080/scim/v2/Users \
  -H "Content-Type: application/json" \
  -d '{
    "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
    "userName": "tutorial.user",
    "name": {
      "givenName": "Tutorial",
      "familyName": "User",
      "formatted": "Tutorial User"
    },
    "displayName": "Tutorial User",
    "emails": [{
      "value": "tutorial@example.com",
      "type": "work",
      "primary": true
    }],
    "phoneNumbers": [{
      "value": "+1-555-0123",
      "type": "mobile"
    }],
    "active": true
  }'
```

The server should respond with the created user including generated metadata:

```json
{
  "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
  "id": "user-005",
  "userName": "tutorial.user",
  "name": {
    "givenName": "Tutorial",
    "familyName": "User",
    "formatted": "Tutorial User"
  },
  "displayName": "Tutorial User",
  "emails": [{
    "value": "tutorial@example.com",
    "type": "work",
    "primary": true
  }],
  "phoneNumbers": [{
    "value": "+1-555-0123",
    "type": "mobile"
  }],
  "active": true,
  "meta": {
    "resourceType": "User",
    "created": "2024-01-15T10:30:00.000Z",
    "lastModified": "2024-01-15T10:30:00.000Z",
    "location": "http://localhost:8080/scim/v2/Users/user-005",
    "version": "W/\"abc123\""
  }
}
```

## Step 5: Updating Resources

### Full Update (PUT)

Update the entire user resource:

```bash
curl -X PUT http://localhost:8080/scim/v2/Users/user-005 \
  -H "Content-Type: application/json" \
  -d '{
    "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
    "id": "user-005",
    "userName": "tutorial.user",
    "displayName": "Tutorial User (Updated)",
    "active": true
  }'
```

### Partial Update (PATCH)

Update just the display name:

```bash
curl -X PATCH http://localhost:8080/scim/v2/Users/user-005 \
  -H "Content-Type: application/json" \
  -d '{
    "schemas": ["urn:ietf:params:scim:api:messages:2.0:PatchOp"],
    "Operations": [{
      "op": "replace",
      "path": "displayName",
      "value": "Tutorial User (Patched)"
    }]
  }'
```

### Add an Email Address

```bash
curl -X PATCH http://localhost:8080/scim/v2/Users/user-005 \
  -H "Content-Type: application/json" \
  -d '{
    "schemas": ["urn:ietf:params:scim:api:messages:2.0:PatchOp"],
    "Operations": [{
      "op": "add",
      "path": "emails",
      "value": {
        "value": "personal@example.com",
        "type": "home",
        "primary": false
      }
    }]
  }'
```

## Step 6: Working with Groups

### Create a New Group

```bash
curl -X POST http://localhost:8080/scim/v2/Groups \
  -H "Content-Type: application/json" \
  -d '{
    "schemas": ["urn:ietf:params:scim:schemas:core:2.0:Group"],
    "displayName": "Tutorial Group",
    "members": [{
      "value": "user-005",
      "type": "User",
      "display": "Tutorial User"
    }]
  }'
```

### Add a Member to Existing Group

```bash
curl -X PATCH http://localhost:8080/scim/v2/Groups/group-002 \
  -H "Content-Type: application/json" \
  -d '{
    "schemas": ["urn:ietf:params:scim:api:messages:2.0:PatchOp"],
    "Operations": [{
      "op": "add",
      "path": "members",
      "value": {
        "value": "user-005",
        "type": "User",
        "display": "Tutorial User"
      }
    }]
  }'
```

## Step 7: Searching and Filtering

### Basic Search

Search for users by username:

```bash
# URL-encoded: filter=userName eq "john.doe"
curl "http://localhost:8080/scim/v2/Users?filter=userName%20eq%20%22john.doe%22"
```

### Complex Filtering

Search for active users with work emails:

```bash
# URL-encoded: filter=active eq true and emails.type eq "work"
curl "http://localhost:8080/scim/v2/Users?filter=active%20eq%20true%20and%20emails.type%20eq%20%22work%22"
```

### Pagination

Get users with pagination:

```bash
curl "http://localhost:8080/scim/v2/Users?startIndex=1&count=2"
```

### Sorting

Get users sorted by family name:

```bash
# URL-encoded: sortBy=name.familyName&sortOrder=ascending
curl "http://localhost:8080/scim/v2/Users?sortBy=name.familyName&sortOrder=ascending"
```

## Step 8: Error Handling

Let's test error scenarios:

### Test Validation Errors

Try to create a user without required userName:

```bash
curl -X POST http://localhost:8080/scim/v2/Users \
  -H "Content-Type: application/json" \
  -d '{
    "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
    "displayName": "Invalid User"
  }'
```

You should get a validation error:

```json
{
  "schemas": ["urn:ietf:params:scim:api:messages:2.0:Error"],
  "status": "400",
  "scimType": "invalidValue",
  "detail": "Missing required attribute 'userName'"
}
```

### Test Not Found Errors

Try to get a non-existent user:

```bash
curl http://localhost:8080/scim/v2/Users/does-not-exist
```

You should get:

```json
{
  "schemas": ["urn:ietf:params:scim:api:messages:2.0:Error"],
  "status": "404",
  "scimType": "notFound",
  "detail": "User with id 'does-not-exist' not found",
  "location": "/Users/does-not-exist"
}
```

## Step 9: Adding Graceful Shutdown

### Update Main Function for Graceful Shutdown

Update your `src/main.rs`:

```rust
use tokio::signal;

#[tokio::main]
async fn main() -> Result<()> {
    // ... previous initialization code ...
    
    let server = ScimServer::new(config);
    
    print_welcome_message();
    
    // Set up graceful shutdown
    let shutdown_signal = async {
        match signal::ctrl_c().await {
            Ok(()) => {
                info!("🛑 Shutdown signal received, starting graceful shutdown...");
            }
            Err(err) => {
                warn!("⚠️  Failed to listen for shutdown signal: {}", err);
            }
        }
    };

    // Run server with graceful shutdown
    tokio::select! {
        result = server.run() => {
            match result {
                Ok(()) => info!("✅ Server stopped successfully"),
                Err(e) => warn!("❌ Server error: {}", e),
            }
        }
        _ = shutdown_signal => {
            info!("🔄 Graceful shutdown initiated");
        }
    }

    info!("👋 Server shutdown complete");
    Ok(())
}
```

## Step 10: Adding a Simple Web Interface

Let's add a simple HTML interface for easier testing.

### Create Static HTML File

Create `static/index.html`:

```html
<!DOCTYPE html>
<html>
<head>
    <title>My First SCIM Server</title>
    <style>
        body { font-family: Arial, sans-serif; margin: 40px; }
        .container { max-width: 800px; margin: 0 auto; }
        .endpoint { background: #f5f5f5; padding: 15px; margin: 10px 0; border-radius: 5px; }
        .method { font-weight: bold; color: #2196F3; }
        button { background: #4CAF50; color: white; padding: 8px 16px; border: none; border-radius: 4px; cursor: pointer; }
        button:hover { background: #45a049; }
        .response { background: #f9f9f9; border: 1px solid #ddd; padding: 10px; margin-top: 10px; white-space: pre-wrap; font-family: monospace; }
    </style>
</head>
<body>
    <div class="container">
        <h1>🚀 My First SCIM Server</h1>
        <p>Welcome to your SCIM server! Use the buttons below to test the API endpoints.</p>
        
        <div class="endpoint">
            <div class="method">GET /scim/v2/Users</div>
            <p>List all users in the system</p>
            <button onclick="makeRequest('GET', '/scim/v2/Users')">List Users</button>
        </div>
        
        <div class="endpoint">
            <div class="method">GET /scim/v2/Groups</div>
            <p>List all groups in the system</p>
            <button onclick="makeRequest('GET', '/scim/v2/Groups')">List Groups</button>
        </div>
        
        <div class="endpoint">
            <div class="method">GET /scim/v2/Users/user-001</div>
            <p>Get John Doe's user details</p>
            <button onclick="makeRequest('GET', '/scim/v2/Users/user-001')">Get John Doe</button>
        </div>
        
        <div class="endpoint">
            <div class="method">POST /scim/v2/Users</div>
            <p>Create a new test user</p>
            <button onclick="createTestUser()">Create Test User</button>
        </div>
        
        <div class="endpoint">
            <div class="method">GET /scim/v2/ServiceProviderConfig</div>
            <p>Get server capabilities and configuration</p>
            <button onclick="makeRequest('GET', '/scim/v2/ServiceProviderConfig')">Get Config</button>
        </div>
        
        <div id="response" class="response" style="display: none;"></div>
    </div>

    <script>
        async function makeRequest(method, url, body = null) {
            const responseDiv = document.getElementById('response');
            responseDiv.style.display = 'block';
            responseDiv.textContent = 'Loading...';
            
            try {
                const options = {
                    method: method,
                    headers: {
                        'Content-Type': 'application/json'
                    }
                };
                
                if (body) {
                    options.body = JSON.stringify(body);
                }
                
                const response = await fetch(url, options);
                const data = await response.json();
                
                responseDiv.textContent = `Status: ${response.status}\n\n${JSON.stringify(data, null, 2)}`;
            } catch (error) {
                responseDiv.textContent = `Error: ${error.message}`;
            }
        }
        
        async function createTestUser() {
            const userData = {
                schemas: ["urn:ietf:params:scim:schemas:core:2.0:User"],
                userName: "test.user." + Date.now(),
                name: {
                    givenName: "Test",
                    familyName: "User",
                    formatted: "Test User"
                },
                displayName: "Test User",
                emails: [{
                    value: "test" + Date.now() + "@example.com",
                    type: "work",
                    primary: true
                }],
                active: true
            };
            
            await makeRequest('POST', '/scim/v2/Users', userData);
        }
    </script>
</body>
</html>
```

### Serve Static Files

Update your `src/main.rs` to serve static files:

```rust
use axum::{routing::get, Router};
use tower_http::services::ServeDir;

// In your main function, before starting the server:
let app = Router::new()
    .nest_service("/", ServeDir::new("static"))
    .nest("/scim/v2", scim_routes);

// Update server configuration to use the combined app
let server = ScimServer::with_app(config, app);
```

Now you can open `http://localhost:8080` in your browser to use the web interface!

## Step 11: Adding Custom Validation

Let's add some business-specific validation rules.

### Create Validation Module

Create `src/validation.rs`:

```rust
use scim_server::resource::Resource;
use scim_server::error::{Result, ScimError};
use tracing::debug;

pub async fn validate_business_rules(resource: &Resource) -> Result<()> {
    debug!("🔍 Validating business rules for resource: {}", resource.id());
    
    // Rule 1: Email domain validation
    validate_email_domain(resource).await?;
    
    // Rule 2: Username format validation
    validate_username_format(resource).await?;
    
    // Rule 3: Phone number format validation
    validate_phone_format(resource).await?;
    
    debug!("✅ Business rules validation passed");
    Ok(())
}

async fn validate_email_domain(resource: &Resource) -> Result<()> {
    const ALLOWED_DOMAINS: &[&str] = &["example.com", "mycompany.com", "test.org"];
    
    if let Some(emails) = resource.emails() {
        for email in emails.values() {
            let domain = email.value().split('@').nth(1).unwrap_or("");
            
            if !ALLOWED_DOMAINS.contains(&domain) {
                return Err(ScimError::validation_error(
                    "emails.value",
                    format!("Email domain '{}' not allowed. Allowed domains: {}", 
                           domain, 
                           ALLOWED_DOMAINS.join(", "))
                ));
            }
        }
    }
    
    Ok(())
}

async fn validate_username_format(resource: &Resource) -> Result<()> {
    if let Some(username) = resource.user_name() {
        let username_str = username.as_str();
        
        // Username must be at least 3 characters
        if username_str.len() < 3 {
            return Err(ScimError::validation_error(
                "userName",
                "Username must be at least 3 characters long"
            ));
        }
        
        // Username must contain only allowed characters
        if !username_str.chars().all(|c| c.is_alphanumeric() || c == '.' || c == '_' || c == '-') {
            return Err(ScimError::validation_error(
                "userName",
                "Username can only contain letters, numbers, dots, underscores, and hyphens"
            ));
        }
        
        // Username cannot start or end with special characters
        if username_str.starts_with(|c: char| !c.is_alphanumeric()) || 
           username_str.ends_with(|c: char| !c.is_alphanumeric()) {
            return Err(ScimError::validation_error(
                "userName",
                "Username must start and end with alphanumeric characters"
            ));
        }
    }
    
    Ok(())
}

async fn validate_phone_format(resource: &Resource) -> Result<()> {
    if let Some(phones) = resource.phone_numbers() {
        for phone in phones.values() {
            let phone_str = phone.value();
            
            // Simple phone number format validation
            if !phone_str.starts_with('+') && !phone_str.chars().skip(1).all(|c| c.is_ascii_digit() || c == '-') {
                return Err(ScimError::validation_error(
                    "phoneNumbers.value",
                    format!("Invalid phone number format: {}", phone_str)
                ));
            }
        }
    }
    
    Ok(())
}
```

### Integrate Validation

Update your `src/main.rs` to include validation:

```rust
mod validation;

// Add validation middleware to your server configuration
let config = ServerConfig::builder()
    .host("localhost")
    .port(8080)
    .base_url("http://localhost:8080/scim/v2")
    .provider(provider)
    .enable_cors(true)
    .cors_origins(vec!["*"])
    .enable_request_logging(true)
    .add_validation_hook(validation::validate_business_rules)
    .build()?;
```

### Test Custom Validation

Try creating a user with an invalid email domain:

```bash
curl -X POST http://localhost:8080/scim/v2/Users \
  -H "Content-Type: application/json" \
  -d '{
    "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
    "userName": "invalid.user",
    "emails": [{
      "value": "user@invalid-domain.com",
      "primary": true
    }]
  }'
```

You should get a validation error about the email domain.

## Step 12: Adding Logging and Monitoring

### Enhanced Logging

Update your logging configuration in `src/main.rs`:

```rust
use tracing::{info, warn, error, debug};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

fn init_logging() {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "my_first_scim_server=info,scim_server=info".into())
        )
        .with(
            tracing_subscriber::fmt::layer()
                .with_target(false)
                .with_thread_