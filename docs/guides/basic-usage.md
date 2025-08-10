# Basic Usage Guide

This guide covers the fundamental concepts and basic operations of the SCIM Server crate. After reading this guide, you'll understand how to perform common SCIM operations and work with core types.

## Table of Contents

- [Core Concepts](#core-concepts)
- [Basic Resource Operations](#basic-resource-operations)
- [Working with Schemas](#working-with-schemas)
- [Resource Providers](#resource-providers)
- [Error Handling](#error-handling)
- [Common Patterns](#common-patterns)

## Core Concepts

### SCIM Resources

SCIM resources are the core entities managed by a SCIM server. The most common resource types are:

- **Users** - Individual user accounts with attributes like username, email, and name
- **Groups** - Collections of users with membership information
- **Custom Resources** - Domain-specific resources extending the base SCIM schema

```rust
use scim_server::resource::{Resource, ResourceBuilder};
use scim_server::resource::value_objects::{ResourceId, UserName, EmailAddress};
use serde_json::json;

// Create a new user resource
let user = ResourceBuilder::new()
    .id(ResourceId::new("user123")?)
    .user_name(UserName::new("john.doe")?)
    .add_email(EmailAddress::new("john.doe@example.com")?)
    .display_name("John Doe")
    .build()?;
```

### Schemas

Schemas define the structure and validation rules for SCIM resources. The server supports:

- **Core User Schema** - Standard SCIM 2.0 user attributes
- **Core Group Schema** - Standard SCIM 2.0 group attributes
- **Enterprise User Extension** - Extended user attributes for enterprise environments
- **Custom Schemas** - User-defined schemas for specific business needs

```rust
use scim_server::schema::{Schema, SchemaBuilder};

// Working with the core user schema
let user_schema = Schema::core_user();
println!("Schema ID: {}", user_schema.id());
println!("Schema Name: {}", user_schema.name());
```

### Multi-Tenancy

The SCIM server supports multi-tenant architectures where different organizations or contexts can be isolated:

```rust
use scim_server::multi_tenant::{TenantContext, TenantId};

// Create a tenant context
let tenant = TenantContext::new(TenantId::new("acme-corp")?);
```

## Basic Resource Operations

### Creating Resources

Resources can be created using the `ResourceBuilder` for type-safe construction:

```rust
use scim_server::resource::{ResourceBuilder, Resource};
use scim_server::resource::value_objects::{ResourceId, UserName, EmailAddress, Name};
use scim_server::error::Result;

async fn create_user() -> Result<Resource> {
    let name = Name::builder()
        .given_name("John")
        .family_name("Doe")
        .build();

    ResourceBuilder::new()
        .id(ResourceId::new("user-123")?)
        .user_name(UserName::new("john.doe")?)
        .add_email(EmailAddress::new("john.doe@example.com")?)
        .name(name)
        .display_name("John Doe")
        .active(true)
        .build()
}
```

### Reading Resources

Resources can be retrieved using their ID:

```rust
use scim_server::providers::ResourceProvider;
use scim_server::resource::value_objects::ResourceId;

async fn get_user<P: ResourceProvider>(
    provider: &P,
    user_id: &str,
) -> Result<Option<Resource>> {
    let id = ResourceId::new(user_id)?;
    provider.get_resource(&id).await
}
```

### Updating Resources

Resources support both full updates (PUT) and partial updates (PATCH):

```rust
use scim_server::resource::Resource;
use serde_json::json;

// Update user's email
let mut user = get_existing_user().await?;
user.set_attribute("emails", json!([{
    "value": "new.email@example.com",
    "type": "work",
    "primary": true
}]))?;
```

### Deleting Resources

```rust
async fn delete_user<P: ResourceProvider>(
    provider: &P,
    user_id: &str,
) -> Result<()> {
    let id = ResourceId::new(user_id)?;
    provider.delete_resource(&id).await
}
```

## Working with Schemas

### Schema Validation

All resources are automatically validated against their schemas:

```rust
use scim_server::schema::validation::SchemaValidator;
use scim_server::resource::Resource;

async fn validate_resource(resource: &Resource) -> Result<()> {
    let validator = SchemaValidator::new();
    validator.validate(resource).await?;
    println!("Resource is valid!");
    Ok(())
}
```

### Custom Schema Attributes

You can work with custom schema attributes:

```rust
use serde_json::json;

// Add custom attribute to a resource
let mut resource = Resource::new();
resource.set_attribute("customAttribute", json!("custom value"))?;

// Retrieve custom attribute
if let Some(value) = resource.get_attribute("customAttribute") {
    println!("Custom attribute: {}", value);
}
```

## Resource Providers

Resource providers handle the storage and retrieval of SCIM resources. The crate includes an in-memory provider for testing and development:

```rust
use scim_server::providers::InMemoryProvider;
use scim_server::resource::Resource;

#[tokio::main]
async fn main() -> Result<()> {
    // Create an in-memory provider
    let provider = InMemoryProvider::new();
    
    // Store a resource
    let user = create_user_resource()?;
    provider.create_resource(user).await?;
    
    // Retrieve the resource
    let retrieved = provider.get_resource(&ResourceId::new("user-123")?).await?;
    
    Ok(())
}
```

### Custom Providers

You can implement your own storage backend by implementing the `ResourceProvider` trait:

```rust
use scim_server::providers::ResourceProvider;
use scim_server::resource::{Resource, ResourceType};
use scim_server::resource::value_objects::ResourceId;
use scim_server::error::Result;
use async_trait::async_trait;

struct MyCustomProvider {
    // Your storage implementation
}

#[async_trait]
impl ResourceProvider for MyCustomProvider {
    async fn create_resource(&self, resource: Resource) -> Result<Resource> {
        // Implement resource creation
        todo!()
    }
    
    async fn get_resource(&self, id: &ResourceId) -> Result<Option<Resource>> {
        // Implement resource retrieval
        todo!()
    }
    
    // ... implement other required methods
}
```

## Error Handling

The SCIM server uses comprehensive error types for different failure scenarios:

```rust
use scim_server::error::{ScimError, ErrorType};

// Handle validation errors
match create_user_with_invalid_email().await {
    Ok(user) => println!("User created successfully"),
    Err(ScimError::Validation { message, .. }) => {
        eprintln!("Validation failed: {}", message);
    }
    Err(ScimError::NotFound { resource_type, id }) => {
        eprintln!("Resource not found: {} with id {}", resource_type, id);
    }
    Err(e) => {
        eprintln!("Other error: {}", e);
    }
}
```

### Result Type

All operations return a `Result<T, ScimError>`:

```rust
use scim_server::error::Result;

async fn safe_operation() -> Result<()> {
    let user = create_user().await?;  // Propagate errors with ?
    validate_user(&user).await?;
    store_user(user).await?;
    Ok(())
}
```

## Common Patterns

### Building Complex Resources

Use the builder pattern for complex resource creation:

```rust
use scim_server::resource::value_objects::{
    Address, PhoneNumber, MultiValuedAttribute
};

let user = ResourceBuilder::new()
    .id(ResourceId::new("complex-user")?)
    .user_name(UserName::new("jane.smith")?)
    .add_email(EmailAddress::new("jane@work.com")?)
    .add_email(EmailAddress::new("jane@personal.com")?)
    .add_phone(PhoneNumber::new("+1-555-0123")?)
    .add_address(Address::builder()
        .street_address("123 Main St")
        .locality("Anytown")
        .region("CA")
        .postal_code("90210")
        .country("US")
        .type_("work")
        .build())
    .active(true)
    .build()?;
```

### Working with Multi-Valued Attributes

Multi-valued attributes like emails and phone numbers have specialized handling:

```rust
use scim_server::resource::value_objects::MultiValuedAttribute;

// Access primary email
if let Some(primary_email) = user.emails()?.primary() {
    println!("Primary email: {}", primary_email.value());
}

// Find specific email by type
let work_emails: Vec<_> = user.emails()?.filter(|email| {
    email.type_() == Some("work")
}).collect();

// Add a new email
let mut emails = user.emails()?.clone();
emails = emails.with_value(EmailAddress::new("new@example.com")?);
user.set_emails(emails)?;
```

### Resource Querying

Basic filtering and searching:

```rust
use scim_server::providers::ResourceProvider;
use scim_server::resource::ResourceType;

// Simple search by username
async fn find_user_by_username<P: ResourceProvider>(
    provider: &P,
    username: &str,
) -> Result<Option<Resource>> {
    let users = provider.list_resources(ResourceType::User).await?;
    
    for user in users {
        if let Some(user_name) = user.user_name() {
            if user_name.as_str() == username {
                return Ok(Some(user));
            }
        }
    }
    
    Ok(None)
}
```

### Tenant-Aware Operations

When working in multi-tenant environments:

```rust
use scim_server::multi_tenant::{TenantContext, TenantResolver};

async fn tenant_aware_operation<R: TenantResolver>(
    resolver: &R,
    tenant_hint: &str,
) -> Result<()> {
    // Resolve tenant from request context
    let tenant = resolver.resolve_tenant(tenant_hint).await?;
    
    // Perform operations within tenant context
    let provider = tenant.resource_provider();
    let users = provider.list_resources(ResourceType::User).await?;
    
    println!("Found {} users in tenant {}", users.len(), tenant.id());
    Ok(())
}
```

## Next Steps

Now that you understand the basics, you can:

1. **[Set up a complete server](quick-start.md)** - Build your first SCIM server
2. **[Learn about configuration](configuration.md)** - Configure the server for your needs
3. **[Explore the API reference](../api/README.md)** - Dive deeper into specific APIs
4. **[Check out examples](../examples/README.md)** - See complete working examples
5. **[Read the architecture guide](architecture.md)** - Understand the system design

## Best Practices

### Type Safety

Always use the provided value objects for type safety:

```rust
// Good: Type-safe construction
let id = ResourceId::new("user-123")?;
let email = EmailAddress::new("user@example.com")?;

// Avoid: Working with raw strings
// let id = "user-123";  // No validation
```

### Error Propagation

Use the `?` operator for clean error propagation:

```rust
async fn create_and_store_user() -> Result<Resource> {
    let user = ResourceBuilder::new()
        .id(ResourceId::new("user-456")?)           // ? propagates validation errors
        .user_name(UserName::new("jane.doe")?)      // ? propagates validation errors
        .build()?;                                  // ? propagates build errors
    
    provider.create_resource(user).await           // ? propagates storage errors
}
```

### Resource Validation

Always validate resources before storage:

```rust
use scim_server::schema::validation::SchemaValidator;

async fn safe_resource_creation(resource: Resource) -> Result<Resource> {
    // Validate before storing
    SchemaValidator::new().validate(&resource).await?;
    
    // Store the validated resource
    provider.create_resource(resource).await
}
```

This completes the basic usage guide. Continue to the [Configuration Guide](configuration.md) to learn about server setup and configuration options.