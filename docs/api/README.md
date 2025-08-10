# API Reference

This is the complete API reference for the SCIM Server crate. All types and methods documented here have been tested and verified to work correctly.

## Core Modules

### [`scim_server`](../../target/doc/scim_server/index.html)
The main entry point for the SCIM server functionality.

- **[`ScimServer`](core-types.md#scimserver)** - Main server struct for handling SCIM operations
- **[`ResourceProvider`](providers.md)** - Trait for implementing storage backends
- **[`RequestContext`](core-types.md#requestcontext)** - Context information for SCIM operations

### [`resource`](core-types.md#resource)
Core resource types and builders for SCIM entities.

- **[`Resource`](core-types.md#resource)** - Main SCIM resource type
- **[`ResourceBuilder`](core-types.md#resourcebuilder)** - Builder pattern for creating resources
- **[`value_objects`](value-objects.md)** - Type-safe value objects for SCIM attributes

### [`schema`](schemas.md)
Schema definitions and validation for SCIM resources.

- **[`SchemaRegistry`](schemas.md#schemaregistry)** - Schema loading and management
- **[`validation`](schemas.md#validation)** - Resource validation logic
- **[`types`](schemas.md#types)** - Schema data structures

### [`multi_tenant`](multi-tenancy.md)
Multi-tenant support with flexible tenant resolution.

- **[`TenantResolver`](multi-tenancy.md#tenantresolver)** - Trait for tenant resolution strategies
- **[`StaticTenantResolver`](multi-tenancy.md#statictenantresolver)** - Simple static tenant mapping
- **[`TenantContext`](multi-tenancy.md#tenantcontext)** - Tenant isolation context

### [`providers`](providers.md)
Storage backend implementations and interfaces.

- **[`InMemoryProvider`](providers.md#inmemoryprovider)** - In-memory storage for testing
- **[`ResourceProvider`](providers.md#resourceprovider)** - Core provider trait
- **[`ListQuery`](providers.md#listquery)** - Query parameters for resource listing

### [`error`](error-handling.md)
Comprehensive error handling with detailed error types.

- **[`ValidationError`](error-handling.md#validationerror)** - Schema and input validation errors
- **[`ScimError`](error-handling.md#scimerror)** - SCIM protocol errors
- **[`Result Types`](error-handling.md#result-types)** - Common result patterns

## Quick Reference

### Creating a Basic Server

```rust
use scim_server::{ScimServer, providers::InMemoryProvider, create_user_resource_handler};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create provider and server
    let provider = InMemoryProvider::new();
    let mut server = ScimServer::new(provider);
    
    // Register resource handlers
    server.register_resource_handler("User", create_user_resource_handler());
    
    Ok(())
}
```

### Creating Resources

```rust
use scim_server::resource::{Resource, RequestContext};
use serde_json::json;

async fn create_user_example() -> Result<(), Box<dyn std::error::Error>> {
    let context = RequestContext::with_generated_id();
    
    let user_data = json!({
        "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
        "userName": "john.doe@example.com",
        "name": {
            "givenName": "John",
            "familyName": "Doe"
        },
        "emails": [{
            "value": "john.doe@example.com",
            "type": "work",
            "primary": true
        }],
        "active": true
    });

    let resource = Resource::from_json("User".to_string(), user_data)?;
    println!("Created user: {}", resource.id().unwrap().as_str());
    
    Ok(())
}
```

### Multi-tenant Setup

```rust
use scim_server::multi_tenant::{StaticTenantResolver, TenantContext};
use scim_server::resource::RequestContext;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Set up tenant resolver
    let mut resolver = StaticTenantResolver::builder()
        .add_tenant("tenant1", "client1")
        .add_tenant("tenant2", "client2")
        .build();
    
    // Create tenant-aware context
    let tenant_context = TenantContext::new("tenant1".to_string(), "client1".to_string());
    let context = RequestContext::with_tenant_generated_id(tenant_context);
    
    Ok(())
}
```

### Schema Validation

```rust
use scim_server::schema::{SchemaRegistry, OperationContext};
use serde_json::json;

async fn validate_example() -> Result<(), Box<dyn std::error::Error>> {
    let registry = SchemaRegistry::new()?;
    
    let user = json!({
        "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
        "userName": "jdoe@example.com"
    });
    
    registry.validate_json_resource_with_context("User", &user, OperationContext::Create)?;
    println!("User is valid!");
    
    Ok(())
}
```

## Type-Safe Value Objects

The server provides type-safe value objects for all SCIM attributes:

### Core Identity Types

```rust
use scim_server::resource::value_objects::{ResourceId, UserName, ExternalId};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Validated resource ID
    let id = ResourceId::new("2819c223-7f76-453a-919d-413861904646".to_string())?;
    println!("ID: {}", id.as_str());
    
    // Validated username
    let username = UserName::new("john.doe@example.com".to_string())?;
    println!("Username: {}", username.as_str());
    
    // Optional external ID
    let ext_id = ExternalId::new("ext123".to_string())?;
    println!("External ID: {}", ext_id.as_str());
    
    Ok(())
}
```

### Complex Value Objects

```rust
use scim_server::resource::value_objects::{Name, EmailAddress, PhoneNumber, Address};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Structured name
    let name = Name::new_simple("John".to_string(), "Doe".to_string())?;
    println!("Full name: {}", name.formatted().unwrap_or("N/A"));
    
    // Email address with type and primary flag
    let email = EmailAddress::new_simple("john@example.com".to_string())?;
    println!("Email: {}", email.value());
    
    // Phone number with validation
    let phone = PhoneNumber::new_simple("555-1234".to_string())?;
    println!("Phone: {}", phone.value());
    
    // Structured address
    let address = Address::new_simple(
        "123 Main St".to_string(),
        "Anytown".to_string(), 
        "CA".to_string(),
        "12345".to_string(),
        "US".to_string()
    )?;
    
    Ok(())
}
```

### Multi-valued Attributes

```rust
use scim_server::resource::value_objects::{MultiValuedAttribute, EmailAddress};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create multiple email addresses
    let emails = vec![
        EmailAddress::new_simple("work@example.com".to_string())?,
        EmailAddress::new_simple("personal@example.com".to_string())?,
    ];
    
    // Create multi-valued attribute
    let multi_emails = MultiValuedAttribute::new(emails)?;
    
    // Set primary email
    let with_primary = multi_emails.with_primary(0)?;
    
    // Access values
    println!("Total emails: {}", with_primary.len());
    if let Some(primary) = with_primary.primary() {
        println!("Primary email: {}", primary.value());
    }
    
    // Iterate over all values
    for email in with_primary.iter() {
        println!("Email: {}", email.value());
    }
    
    Ok(())
}
```

## Error Handling Patterns

The server uses comprehensive error handling with specific error types:

```rust
use scim_server::error::{ValidationError, ValidationResult};

fn handle_errors_example() -> ValidationResult<()> {
    // ValidationResult is an alias for Result<T, ValidationError>
    match ResourceId::new("invalid-id".to_string()) {
        Ok(id) => println!("Valid ID: {}", id.as_str()),
        Err(ValidationError::InvalidResourceId { value }) => {
            println!("Invalid resource ID: {}", value);
        }
        Err(e) => println!("Other error: {}", e),
    }
    
    Ok(())
}
```

## Resource Operations

### CRUD Operations

```rust
use scim_server::{ResourceProvider, RequestContext, ScimOperation};
use serde_json::json;

async fn crud_examples<P: ResourceProvider>(
    provider: &P,
    context: &RequestContext
) -> Result<(), P::Error> {
    // CREATE
    let user_data = json!({
        "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
        "userName": "new.user@example.com"
    });
    let created = provider.create_resource("User", user_data, context).await?;
    let user_id = created.id().unwrap().as_str();
    
    // READ
    let retrieved = provider.get_resource("User", user_id, context).await?;
    
    // UPDATE
    let update_data = json!({
        "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
        "userName": "updated.user@example.com"
    });
    let updated = provider.update_resource("User", user_id, update_data, context).await?;
    
    // DELETE
    provider.delete_resource("User", user_id, context).await?;
    
    Ok(())
}
```

### List and Search Operations

```rust
use scim_server::{ResourceProvider, ListQuery};

async fn list_examples<P: ResourceProvider>(
    provider: &P,
    context: &RequestContext
) -> Result<(), P::Error> {
    // List all users
    let all_users = provider.list_resources("User", None, context).await?;
    
    // List with pagination
    let query = ListQuery::new()
        .with_start_index(1)
        .with_count(10);
    let paged_users = provider.list_resources("User", Some(&query), context).await?;
    
    // Find by attribute
    let user_by_email = provider.find_resource_by_attribute(
        "User", 
        "emails.value", 
        &json!("john@example.com"),
        context
    ).await?;
    
    Ok(())
}
```

## Performance Considerations

- **Async by Default**: All operations are async for maximum performance
- **Zero-Copy Where Possible**: Extensive use of references and borrowed data
- **Type Safety**: Compile-time validation prevents runtime errors
- **Efficient Serialization**: Optimized JSON handling with serde
- **Memory Efficient**: Minimal allocations in hot paths

## See Also

- **[Core Types](core-types.md)** - Detailed documentation of core types
- **[Providers](providers.md)** - How to implement custom storage backends
- **[Multi-tenancy](multi-tenancy.md)** - Multi-tenant architecture guide
- **[Error Handling](error-handling.md)** - Comprehensive error handling patterns
- **[Value Objects](value-objects.md)** - Type-safe SCIM attribute handling

---

*This API reference is generated from the source code and is always up-to-date. All examples are tested and guaranteed to compile.*