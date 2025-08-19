# Core Types API Reference

This document provides detailed documentation for the core types in the SCIM server crate. These types form the foundation of the SCIM 2.0 implementation and provide type-safe abstractions for SCIM resources.

## Table of Contents

- [Resource](#resource)
- [ResourceBuilder](#resourcebuilder)
- [RequestContext](#requestcontext)
- [ScimServer](#scimserver)
- [Value Objects](#value-objects)
- [Common Patterns](#common-patterns)

## Resource

The `Resource` struct is the core representation of a SCIM resource (User, Group, etc.).

### Definition

```rust
pub struct Resource {
    resource_type: String,
    id: Option<ResourceId>,
    schemas: Vec<SchemaUri>,
    external_id: Option<ExternalId>,
    user_name: Option<UserName>,
    meta: Option<Meta>,
    name: Option<Name>,
    // ... additional fields
}
```

### Key Methods

#### `Resource::from_json`

Creates a resource from JSON data with validation.

```rust
pub fn from_json(resource_type: String, data: Value) -> ValidationResult<Self>
```

**Example:**
```rust
use scim_server::resource::Resource;
use serde_json::json;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let user_data = json!({
        "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
        "userName": "john.doe@example.com",
        "name": {
            "givenName": "John",
            "familyName": "Doe"
        }
    });
    
    let resource = Resource::from_json("User".to_string(), user_data)?;
    println!("Created resource with ID: {:?}", resource.id());
    
    Ok(())
}
```

#### `Resource::new`

Creates a resource with validated core fields.

```rust
pub fn new(
    resource_type: String,
    id: Option<ResourceId>,
    schemas: Vec<SchemaUri>,
    external_id: Option<ExternalId>,
    user_name: Option<UserName>,
    attributes: Map<String, Value>,
) -> Self
```

#### `Resource::to_json`

Serializes the resource to JSON.

```rust
pub fn to_json(&self) -> ValidationResult<Value>
```

#### Accessor Methods

```rust
// Core identifiers
pub fn id(&self) -> Option<&ResourceId>
pub fn external_id(&self) -> Option<&ExternalId>
pub fn user_name(&self) -> Option<&UserName>

// Resource metadata
pub fn resource_type(&self) -> &str
pub fn schemas(&self) -> &[SchemaUri]
pub fn meta(&self) -> Option<&Meta>

// User-specific attributes
pub fn name(&self) -> Option<&Name>
pub fn display_name(&self) -> Option<&str>
pub fn nick_name(&self) -> Option<&str>
pub fn emails(&self) -> Option<&MultiValuedAttribute<EmailAddress>>
pub fn phone_numbers(&self) -> Option<&MultiValuedAttribute<PhoneNumber>>
pub fn addresses(&self) -> Option<&MultiValuedAttribute<Address>>

// Group-specific attributes  
pub fn members(&self) -> Option<&GroupMembers>

// Extension attributes
pub fn get_extension_attribute(&self, schema_uri: &str, attribute_name: &str) -> Option<&Value>
```

## ResourceBuilder

A builder pattern for constructing SCIM resources with validation.

### Definition

```rust
pub struct ResourceBuilder {
    // Internal fields for building resources
}
```

### Usage Pattern

```rust
use scim_server::resource::ResourceBuilder;
use scim_server::resource::value_objects::{ResourceId, UserName, Name};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let resource = ResourceBuilder::new("User")
        .id(ResourceId::new("123".to_string())?)?
        .user_name(UserName::new("jdoe".to_string())?)?
        .name(Name::new_simple("John".to_string(), "Doe".to_string())?)?
        .display_name("John Doe".to_string())
        .active(true)
        .build()?;
    
    println!("Built resource: {:?}", resource.id());
    Ok(())
}
```

### Key Methods

#### `ResourceBuilder::new`

Creates a new builder for the specified resource type.

```rust
pub fn new(resource_type: &str) -> Self
```

#### Core Field Setters

```rust
// Identity fields
pub fn id(self, id: ResourceId) -> ValidationResult<Self>
pub fn external_id(self, external_id: ExternalId) -> ValidationResult<Self>
pub fn user_name(self, user_name: UserName) -> ValidationResult<Self>

// User attributes
pub fn name(self, name: Name) -> ValidationResult<Self>
pub fn display_name(self, display_name: String) -> Self
pub fn nick_name(self, nick_name: String) -> Self
pub fn profile_url(self, profile_url: String) -> Self
pub fn title(self, title: String) -> Self
pub fn user_type(self, user_type: String) -> Self
pub fn preferred_language(self, preferred_language: String) -> Self
pub fn locale(self, locale: String) -> Self
pub fn timezone(self, timezone: String) -> Self
pub fn active(self, active: bool) -> Self
pub fn password(self, password: String) -> Self

// Multi-valued attributes
pub fn emails(self, emails: MultiValuedAttribute<EmailAddress>) -> ValidationResult<Self>
pub fn phone_numbers(self, phone_numbers: MultiValuedAttribute<PhoneNumber>) -> ValidationResult<Self>
pub fn addresses(self, addresses: MultiValuedAttribute<Address>) -> ValidationResult<Self>

// Group attributes
pub fn members(self, members: GroupMembers) -> ValidationResult<Self>

// Extension attributes
pub fn extension_attribute(self, schema_uri: String, attribute_name: String, value: Value) -> Self
```

#### `ResourceBuilder::build`

Finalizes the builder and creates the resource.

```rust
pub fn build(self) -> ValidationResult<Resource>
```

## RequestContext

Provides context information for SCIM operations, including tenant information and operation metadata.

### Definition

```rust
pub struct RequestContext {
    tenant_context: Option<TenantContext>,
    operation_id: String,
    // Additional context fields
}
```

### Factory Methods

#### Single-tenant Operations

```rust
// Generate a random operation ID
pub fn with_generated_id() -> Self

// Use a specific operation ID
pub fn new(operation_id: String) -> Self
```

#### Multi-tenant Operations

```rust
// Generate ID with tenant context
pub fn with_tenant_generated_id(tenant_context: TenantContext) -> Self

// Use specific ID with tenant context
pub fn with_tenant(operation_id: String, tenant_context: TenantContext) -> Self
```

### Usage Examples

```rust
use scim_server::resource::RequestContext;
use scim_server::multi_tenant::TenantContext;

fn main() {
    // Single-tenant context
    let context = RequestContext::with_generated_id();
    
    // Multi-tenant context
    let tenant_context = TenantContext::new("tenant1".to_string(), "client1".to_string());
    let multi_context = RequestContext::with_tenant_generated_id(tenant_context);
    
    println!("Operation ID: {}", context.operation_id());
    if let Some(tenant) = multi_context.tenant_context() {
        println!("Tenant: {}", tenant.tenant_id());
    }
}
```

### Accessor Methods

```rust
pub fn operation_id(&self) -> &str
pub fn tenant_context(&self) -> Option<&TenantContext>
pub fn is_multi_tenant(&self) -> bool
```

## ScimServer

The main server struct that orchestrates SCIM operations.

### Definition

```rust
pub struct ScimServer<P: ResourceProvider> {
    provider: P,
    resource_handlers: HashMap<String, Box<dyn ResourceHandler>>,
    schema_registry: SchemaRegistry,
    // Additional server state
}
```

### Construction

```rust
use scim_server::{
    providers::StandardResourceProvider,
    storage::InMemoryStorage,
    RequestContext,
};

fn main() {
    let storage = InMemoryStorage::new();
    let provider = StandardResourceProvider::new(storage);
    let context = RequestContext::new("example".to_string());
}
```

### Resource Handler Registration

```rust
use scim_server::{create_user_resource_handler, create_group_resource_handler};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let storage = InMemoryStorage::new();
    let provider = StandardResourceProvider::new(storage);
    let mut server = ScimServer::new(provider)?;
    
    // Register schemas for resource types
    server.register_schema(user_schema).await?;
    server.register_schema(group_schema).await?;
    
    // Register custom schema
    server.register_schema(custom_schema).await?;
    
    Ok(())
}
```

### Core Operations

The server provides high-level operations that coordinate between providers, handlers, and validation:

```rust
impl<P: ResourceProvider> ScimServer<P> {
    // Resource CRUD operations
    pub async fn create_resource(&self, resource_type: &str, data: Value, context: &RequestContext) -> Result<Resource, P::Error>
    pub async fn get_resource(&self, resource_type: &str, id: &str, context: &RequestContext) -> Result<Option<Resource>, P::Error>
    pub async fn update_resource(&self, resource_type: &str, id: &str, data: Value, context: &RequestContext) -> Result<Resource, P::Error>
    pub async fn delete_resource(&self, resource_type: &str, id: &str, context: &RequestContext) -> Result<(), P::Error>
    pub async fn list_resources(&self, resource_type: &str, query: Option<&ListQuery>, context: &RequestContext) -> Result<Vec<Resource>, P::Error>
    
    // Schema operations
    pub fn get_schemas(&self) -> &SchemaRegistry
    pub fn validate_resource(&self, resource: &Resource) -> ValidationResult<()>
}
```

## Value Objects

Type-safe representations of SCIM attributes that enforce validation at compile time.

### Core Identity Types

#### ResourceId
- **Purpose**: Unique identifier for SCIM resources
- **Validation**: Must be a valid UUID format
- **Usage**: `ResourceId::new(uuid_string)?`

#### UserName  
- **Purpose**: Unique user identifier (often email)
- **Validation**: Must not be empty, typically email format
- **Usage**: `UserName::new("user@example.com".to_string())?`

#### ExternalId
- **Purpose**: External system identifier
- **Validation**: Must not be empty if provided
- **Usage**: `ExternalId::new("ext123".to_string())?`

### Complex Types

#### Name
- **Purpose**: Structured name information
- **Components**: givenName, familyName, middleName, honorificPrefix, honorificSuffix, formatted
- **Usage**: `Name::new_simple("John".to_string(), "Doe".to_string())?`

#### Meta
- **Purpose**: Resource metadata (created, lastModified, version, location)
- **Managed by**: Server automatically
- **Usage**: Generally not created by client code

### Multi-valued Attribute Types

#### EmailAddress
- **Purpose**: Email address with type and primary designation
- **Validation**: Valid email format
- **Usage**: `EmailAddress::new_simple("user@example.com".to_string())?`

#### PhoneNumber
- **Purpose**: Phone number with type and primary designation  
- **Validation**: Non-empty string
- **Usage**: `PhoneNumber::new_simple("555-1234".to_string())?`

#### Address
- **Purpose**: Structured postal address
- **Components**: formatted, streetAddress, locality, region, postalCode, country, type, primary
- **Usage**: `Address::new_simple(street, city, state, zip, country)?`

### Multi-valued Container

#### MultiValuedAttribute<T>
- **Purpose**: Container for multiple values with primary designation
- **Constraint**: At most one primary value
- **Usage**: `MultiValuedAttribute::new(vec![email1, email2])?.with_primary(0)?`

## Common Patterns

### Resource Creation Pattern

```rust
use scim_server::resource::{Resource, ResourceBuilder};
use scim_server::resource::value_objects::*;

fn create_user_pattern() -> Result<Resource, Box<dyn std::error::Error>> {
    // Pattern 1: From JSON (most common)
    let user_data = json!({
        "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
        "userName": "john.doe@example.com"
    });
    let resource = Resource::from_json("User".to_string(), user_data)?;
    
    // Pattern 2: Using builder (type-safe)
    let resource = ResourceBuilder::new("User")
        .user_name(UserName::new("john.doe@example.com".to_string())?)?
        .name(Name::new_simple("John".to_string(), "Doe".to_string())?)?
        .active(true)
        .build()?;
    
    Ok(resource)
}
```

### Validation Pattern

```rust
use scim_server::error::{ValidationError, ValidationResult};

fn validation_pattern() -> ValidationResult<()> {
    // All value objects validate at construction
    let id = ResourceId::new("invalid".to_string())?; // Returns Err if invalid
    
    // Chain validations
    let name = Name::new_simple("John".to_string(), "Doe".to_string())?;
    let username = UserName::new("john.doe@example.com".to_string())?;
    
    // Combine in builder
    let resource = ResourceBuilder::new("User")
        .user_name(username)?  // ? operator propagates validation errors
        .name(name)?
        .build()?;
    
    Ok(())
}
```

### Multi-valued Attribute Pattern

```rust
use scim_server::resource::value_objects::{MultiValuedAttribute, EmailAddress};

fn multi_valued_pattern() -> Result<(), Box<dyn std::error::Error>> {
    // Create individual values
    let work_email = EmailAddress::new_simple("work@example.com".to_string())?;
    let personal_email = EmailAddress::new_simple("personal@example.com".to_string())?;
    
    // Combine into multi-valued attribute
    let emails = MultiValuedAttribute::new(vec![work_email, personal_email])?
        .with_primary(0)?;  // Set first email as primary
    
    // Access patterns
    println!("Total emails: {}", emails.len());
    if let Some(primary) = emails.primary() {
        println!("Primary email: {}", primary.value());
    }
    
    // Iteration
    for email in emails.iter() {
        println!("Email: {}", email.value());
    }
    
    // Filtering
    let work_emails = emails.filter(|e| e.email_type().map_or(false, |t| t == "work"));
    
    Ok(())
}
```

### Error Handling Pattern

```rust
use scim_server::error::{ValidationError, ValidationResult};

fn error_handling_pattern() -> ValidationResult<()> {
    match ResourceId::new("invalid-uuid".to_string()) {
        Ok(id) => {
            println!("Valid ID: {}", id.as_str());
        }
        Err(ValidationError::InvalidResourceId { value }) => {
            println!("Invalid resource ID format: {}", value);
            // Handle gracefully
        }
        Err(ValidationError::MissingRequiredAttribute { attribute }) => {
            println!("Missing required attribute: {}", attribute);
        }
        Err(e) => {
            println!("Other validation error: {}", e);
        }
    }
    
    Ok(())
}
```

### Async Operation Pattern

```rust
use scim_server::{ResourceProvider, RequestContext};
use serde_json::json;

async fn async_operation_pattern<P: ResourceProvider>(
    provider: &P,
    context: &RequestContext
) -> Result<(), P::Error> {
    // Create operation
    let user_data = json!({
        "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
        "userName": "async.user@example.com"
    });
    
    let created = provider.create_resource("User", user_data, context).await?;
    let user_id = created.id().unwrap().as_str();
    
    // Read operation
    let retrieved = provider.get_resource("User", user_id, context).await?;
    
    // Update operation
    let update_data = json!({
        "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
        "userName": "updated.user@example.com",
        "active": false
    });
    
    let updated = provider.update_resource("User", user_id, update_data, context).await?;
    
    // Delete operation
    provider.delete_resource("User", user_id, context).await?;
    
    Ok(())
}
```

## Type Relationships

### Ownership and Lifetimes

```rust
// Resources own their data
let resource: Resource = Resource::from_json(...)?;

// Value objects are owned by resources
let name: &Name = resource.name().unwrap();  // Borrowed from resource

// Multi-valued attributes contain owned values
let emails: &MultiValuedAttribute<EmailAddress> = resource.emails().unwrap();
let email: &EmailAddress = emails.primary().unwrap();  // Borrowed from collection
```

### Conversion Patterns

```rust
// String to value object
let id = ResourceId::new(uuid_string)?;
let username = UserName::new(email_string)?;

// Value object to string
let id_str: &str = id.as_str();
let id_owned: String = id.into_string();

// JSON to Resource
let resource = Resource::from_json(resource_type, json_data)?;

// Resource to JSON
let json_data = resource.to_json()?;
```

### Builder to Resource Conversion

```rust
// Builder accumulates validated components
let builder = ResourceBuilder::new("User")
    .user_name(validated_username)?
    .name(validated_name)?;

// Build validates the complete resource
let resource: Resource = builder.build()?;
```

## Performance Characteristics

### Memory Efficiency
- **Zero-copy deserialization** where possible
- **Borrowed data** in accessor methods
- **Efficient string handling** with owned vs borrowed patterns

### Validation Costs
- **Compile-time guarantees** eliminate runtime validation where possible
- **Early validation** catches errors at construction time
- **Incremental validation** in builders reduces redundant checks

### Async Performance
- **Non-blocking operations** throughout the API
- **Efficient futures** with minimal allocation
- **Concurrent resource operations** supported

## Thread Safety

### Send + Sync Types
Most types implement `Send + Sync` for multi-threaded usage:

```rust
// These can be safely shared between threads
let resource: Resource = ...;  // Send + Sync
let context: RequestContext = ...;  // Send + Sync
let provider: StandardResourceProvider<InMemoryStorage> = ...;  // Send + Sync
```

### Shared State Patterns

```rust
use std::sync::Arc;
use tokio::sync::RwLock;

// Shared provider pattern
type SharedProvider = Arc<RwLock<StandardResourceProvider<InMemoryStorage>>>;

// Shared server pattern  
type SharedServer = Arc<ScimServer<StandardResourceProvider<InMemoryStorage>>>;
```

## See Also

- **[Providers](providers.md)** - Storage backend implementations
- **[Value Objects](value-objects.md)** - Detailed value object documentation
- **[Multi-tenancy](multi-tenancy.md)** - Multi-tenant context and resolution
- **[Error Handling](error-handling.md)** - Comprehensive error handling guide
- **[Schemas](schemas.md)** - Schema validation and management

---

*This documentation reflects the current API as of the latest version. All examples are tested and guaranteed to compile.*