# Resource Model

The SCIM Server's resource model provides a type-safe, extensible foundation for identity management. This chapter explains how resources work, how to customize them, and how the type system prevents common errors.

## Overview

In SCIM, everything is a **resource** - users, groups, and custom objects all follow the same fundamental patterns. The SCIM Server library models these as Rust types that provide compile-time safety while maintaining runtime flexibility.

```rust
use scim_server::{ScimUser, ScimGroup, ScimResource};

// Type-safe resource creation
let user = ScimUser::builder()
    .username("alice@example.com")
    .given_name("Alice")
    .family_name("Johnson")
    .email("alice@example.com")
    .build()?;

// Compile-time guarantees
let id = user.id(); // Always returns a valid UUID
let version = user.version(); // ETag for concurrency control
```

This design provides the flexibility of JSON with the safety of Rust's type system.

## Core Resource Structure

### Base Resource Traits

All SCIM resources implement the `ScimResource` trait:

```rust
pub trait ScimResource {
    fn id(&self) -> &str;
    fn schemas(&self) -> &[String];
    fn meta(&self) -> &ResourceMeta;
    fn external_id(&self) -> Option<&str>;
}
```

### Resource Metadata

Every resource includes metadata for versioning and auditing:

```rust
pub struct ResourceMeta {
    pub resource_type: String,
    pub created: DateTime<Utc>,
    pub last_modified: DateTime<Utc>,
    pub version: String,  // ETag for concurrency control
    pub location: Option<String>,
}
```

The metadata is automatically managed by the SCIM Server:

```json
{
  "meta": {
    "resourceType": "User",
    "created": "2023-12-01T10:30:00Z",
    "lastModified": "2023-12-01T15:45:00Z",
    "version": "W/\"3694e05e9dff590\"",
    "location": "https://api.example.com/scim/v2/Users/123"
  }
}
```

## User Resources

### Core User Attributes

The `ScimUser` type models the standard SCIM user schema:

```rust
use scim_server::{ScimUser, Name, Email, PhoneNumber};

let user = ScimUser::builder()
    .username("bjensen@example.com")
    .name(Name {
        formatted: Some("Ms. Barbara J Jensen III".to_string()),
        family_name: Some("Jensen".to_string()),
        given_name: Some("Barbara".to_string()),
        middle_name: Some("Jane".to_string()),
        honorific_prefix: Some("Ms.".to_string()),
        honorific_suffix: Some("III".to_string()),
    })
    .display_name("Babs Jensen")
    .nick_name("Babs")
    .profile_url("https://login.example.com/bjensen")
    .email(Email {
        value: "bjensen@example.com".to_string(),
        type_: Some("work".to_string()),
        primary: Some(true),
    })
    .phone_number(PhoneNumber {
        value: "+1-555-555-8377".to_string(),
        type_: Some("work".to_string()),
        primary: Some(true),
    })
    .active(true)
    .build()?;
```

### Multi-Value Attributes

SCIM supports multi-value attributes for emails, phone numbers, and addresses:

```rust
let user = ScimUser::builder()
    .username("alice@example.com")
    .emails(vec![
        Email {
            value: "alice@work.com".to_string(),
            type_: Some("work".to_string()),
            primary: Some(true),
        },
        Email {
            value: "alice@personal.com".to_string(),
            type_: Some("home".to_string()),
            primary: Some(false),
        },
    ])
    .phone_numbers(vec![
        PhoneNumber {
            value: "+1-555-555-1234".to_string(),
            type_: Some("work".to_string()),
            primary: Some(true),
        },
        PhoneNumber {
            value: "+1-555-555-5678".to_string(),
            type_: Some("mobile".to_string()),
            primary: Some(false),
        },
    ])
    .build()?;
```

### Enterprise Extensions

For enterprise environments, SCIM provides additional attributes:

```rust
use scim_server::{ScimUser, EnterpriseUser};

let user = ScimUser::builder()
    .username("alice@example.com")
    .given_name("Alice")
    .family_name("Johnson")
    .enterprise(EnterpriseUser {
        employee_number: Some("12345".to_string()),
        cost_center: Some("Engineering".to_string()),
        organization: Some("ACME Corp".to_string()),
        division: Some("Technology".to_string()),
        department: Some("Software Development".to_string()),
        manager: Some(Manager {
            value: "manager-id-456".to_string(),
            ref_: Some("../Users/manager-id-456".to_string()),
            display_name: Some("Bob Smith".to_string()),
        }),
    })
    .build()?;
```

## Group Resources

### Basic Group Structure

Groups represent collections of users with optional hierarchical relationships:

```rust
use scim_server::{ScimGroup, GroupMember};

let group = ScimGroup::builder()
    .display_name("Engineering Team")
    .members(vec![
        GroupMember {
            value: "user-id-123".to_string(),
            ref_: Some("../Users/user-id-123".to_string()),
            type_: Some("User".to_string()),
            display: Some("Alice Johnson".to_string()),
        },
        GroupMember {
            value: "user-id-456".to_string(),
            ref_: Some("../Users/user-id-456".to_string()),
            type_: Some("User".to_string()),
            display: Some("Bob Smith".to_string()),
        },
    ])
    .build()?;
```

### Nested Groups

Groups can contain other groups for hierarchical organization:

```rust
let parent_group = ScimGroup::builder()
    .display_name("All Engineering")
    .members(vec![
        GroupMember {
            value: "group-frontend".to_string(),
            ref_: Some("../Groups/group-frontend".to_string()),
            type_: Some("Group".to_string()),
            display: Some("Frontend Team".to_string()),
        },
        GroupMember {
            value: "group-backend".to_string(),
            ref_: Some("../Groups/group-backend".to_string()),
            type_: Some("Group".to_string()),
            display: Some("Backend Team".to_string()),
        },
    ])
    .build()?;
```

## Schema System

### Schema Definition

Schemas define the structure and validation rules for resources:

```rust
use scim_server::{Schema, Attribute, AttributeType, Mutability, Returned};

let user_schema = Schema::builder()
    .id("urn:ietf:params:scim:schemas:core:2.0:User")
    .name("User")
    .description("User Account")
    .attribute(
        Attribute::builder()
            .name("userName")
            .type_(AttributeType::String)
            .mutability(Mutability::ReadWrite)
            .returned(Returned::Default)
            .uniqueness(true)
            .required(true)
            .case_exact(false)
            .build()
    )
    .attribute(
        Attribute::builder()
            .name("name")
            .type_(AttributeType::Complex)
            .mutability(Mutability::ReadWrite)
            .returned(Returned::Default)
            .sub_attribute(
                Attribute::builder()
                    .name("givenName")
                    .type_(AttributeType::String)
                    .mutability(Mutability::ReadWrite)
                    .build()
            )
            .build()
    )
    .build()?;
```

### Dynamic Schema Registry

The SCIM Server maintains a registry of available schemas:

```rust
use scim_server::{SchemaRegistry, CoreSchemas};

let mut registry = SchemaRegistry::new();

// Register core schemas
registry.register(CoreSchemas::user());
registry.register(CoreSchemas::group());
registry.register(CoreSchemas::enterprise_user());

// Register custom schemas
registry.register(custom_department_schema());

// Validate resources against schemas
let validation_result = registry.validate_user(&user)?;
```

## Custom Resources

### Defining Custom Resource Types

You can extend SCIM with custom resource types:

```rust
use scim_server::{ScimResource, ResourceMeta};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    pub id: String,
    pub schemas: Vec<String>,
    pub meta: ResourceMeta,
    pub external_id: Option<String>,
    
    // Custom attributes
    pub name: String,
    pub description: Option<String>,
    pub owner: String,
    pub status: ProjectStatus,
    pub created_date: DateTime<Utc>,
    pub budget: Option<f64>,
    pub team_members: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ProjectStatus {
    Planning,
    Active,
    OnHold,
    Completed,
    Cancelled,
}

impl ScimResource for Project {
    fn id(&self) -> &str {
        &self.id
    }
    
    fn schemas(&self) -> &[String] {
        &self.schemas
    }
    
    fn meta(&self) -> &ResourceMeta {
        &self.meta
    }
    
    fn external_id(&self) -> Option<&str> {
        self.external_id.as_deref()
    }
}
```

### Custom Schema Definition

Define the schema for your custom resource:

```rust
fn project_schema() -> Schema {
    Schema::builder()
        .id("urn:company:params:scim:schemas:core:2.0:Project")
        .name("Project")
        .description("Project Management Resource")
        .attribute(
            Attribute::builder()
                .name("name")
                .type_(AttributeType::String)
                .mutability(Mutability::ReadWrite)
                .returned(Returned::Default)
                .required(true)
                .case_exact(false)
                .build()
        )
        .attribute(
            Attribute::builder()
                .name("status")
                .type_(AttributeType::String)
                .mutability(Mutability::ReadWrite)
                .returned(Returned::Default)
                .canonical_values(vec![
                    "Planning".to_string(),
                    "Active".to_string(),
                    "OnHold".to_string(),
                    "Completed".to_string(),
                    "Cancelled".to_string(),
                ])
                .build()
        )
        .build()
        .unwrap()
}
```

## Type Safety Features

### Compile-Time Validation

The type system prevents many common errors:

```rust
// ✅ This compiles - valid email
let user = ScimUser::builder()
    .email("alice@example.com")
    .build()?;

// ❌ This won't compile - wrong type
let user = ScimUser::builder()
    .email(123)  // Error: expected String, found integer
    .build()?;
```

### Builder Pattern Safety

The builder pattern ensures required fields are provided:

```rust
// ✅ This compiles - username is required and provided
let user = ScimUser::builder()
    .username("alice@example.com")
    .build()?;

// ❌ This won't compile - missing required username
let user = ScimUser::builder()
    .given_name("Alice")
    .build()?;  // Error: username is required
```

### Option Types for Optional Fields

Optional fields use Rust's `Option` type:

```rust
let user = ScimUser::builder()
    .username("alice@example.com")
    .given_name("Alice")  // Option<String> - automatically wrapped
    .middle_name(None)    // Explicitly no middle name
    .family_name(Some("Johnson".to_string()))  // Explicitly provided
    .build()?;

// Safe access to optional fields
match user.middle_name() {
    Some(middle) => println!("Middle name: {}", middle),
    None => println!("No middle name provided"),
}
```

## Validation and Constraints

### Built-in Validation

The SCIM Server provides automatic validation:

```rust
use scim_server::{ScimUser, ValidationError};

// Email format validation
let result = ScimUser::builder()
    .username("invalid-email")  // Missing @ symbol
    .build();

match result {
    Ok(user) => println!("User created: {}", user.username()),
    Err(ValidationError::InvalidEmail(email)) => {
        println!("Invalid email format: {}", email);
    },
    Err(e) => println!("Other validation error: {}", e),
}
```

### Custom Validation Rules

Add your own validation logic:

```rust
impl ScimUser {
    pub fn validate_business_rules(&self) -> Result<(), ValidationError> {
        // Custom rule: work emails must be from company domain
        if let Some(work_email) = self.work_email() {
            if !work_email.ends_with("@company.com") {
                return Err(ValidationError::InvalidWorkEmail);
            }
        }
        
        // Custom rule: employee number format
        if let Some(employee_number) = self.employee_number() {
            if !employee_number.starts_with("EMP") {
                return Err(ValidationError::InvalidEmployeeNumber);
            }
        }
        
        Ok(())
    }
}
```

## Serialization and JSON

### Automatic JSON Serialization

Resources automatically serialize to SCIM-compliant JSON:

```rust
use serde_json;

let user = ScimUser::builder()
    .username("alice@example.com")
    .given_name("Alice")
    .family_name("Johnson")
    .build()?;

let json = serde_json::to_string_pretty(&user)?;
println!("{}", json);
```

Output:
```json
{
  "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
  "id": "2819c223-7f76-453a-919d-413861904646",
  "userName": "alice@example.com",
  "name": {
    "givenName": "Alice",
    "familyName": "Johnson"
  },
  "meta": {
    "resourceType": "User",
    "created": "2023-12-01T10:30:00Z",
    "lastModified": "2023-12-01T10:30:00Z",
    "version": "W/\"1\""
  }
}
```

### JSON Deserialization

Parse JSON into type-safe resources:

```rust
let json = r#"
{
  "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
  "userName": "alice@example.com",
  "name": {
    "givenName": "Alice",
    "familyName": "Johnson"
  }
}
"#;

let user: ScimUser = serde_json::from_str(json)?;
println!("User: {} {}", user.given_name(), user.family_name());
```

## Performance Considerations

### Memory Efficiency

The resource model is designed for efficiency:

```rust
// Zero-copy string references where possible
impl ScimUser {
    pub fn username(&self) -> &str {  // Returns reference, not owned String
        &self.username
    }
    
    pub fn display_name(&self) -> Option<&str> {  // Optional reference
        self.display_name.as_deref()
    }
}
```

### Lazy Loading

Complex attributes can be loaded on demand:

```rust
// Only load enterprise attributes when needed
impl ScimUser {
    pub fn enterprise(&self) -> Option<&EnterpriseUser> {
        self.enterprise.as_ref()
    }
    
    pub fn load_enterprise(&mut self, provider: &impl Provider) -> Result<(), Error> {
        if self.enterprise.is_none() {
            self.enterprise = provider.load_enterprise_data(&self.id)?;
        }
        Ok(())
    }
}
```

## Best Practices

### Resource Creation

**Use builders for complex resources**:
```rust
let user = ScimUser::builder()
    .username("alice@example.com")
    .given_name("Alice")
    .family_name("Johnson")
    .email("alice@example.com")
    .active(true)
    .build()?;
```

**Validate early and often**:
```rust
// Validate during creation
let user = ScimUser::builder()
    .username("alice@example.com")
    .validate()  // Explicit validation
    .build()?;

// Validate before persistence
user.validate_business_rules()?;
provider.create_user(user)?;
```

### Schema Management

**Register schemas at startup**:
```rust
fn setup_schemas(registry: &mut SchemaRegistry) {
    registry.register(CoreSchemas::user());
    registry.register(CoreSchemas::group());
    registry.register(custom_project_schema());
}
```

**Version your custom schemas**:
```rust
const PROJECT_SCHEMA_V1: &str = "urn:company:scim:schemas:project:1.0";
const PROJECT_SCHEMA_V2: &str = "urn:company:scim:schemas:project:2.0";
```

### Error Handling

**Handle validation errors gracefully**:
```rust
match ScimUser::builder().username("invalid").build() {
    Ok(user) => process_user(user),
    Err(ValidationError::InvalidUsername(username)) => {
        log::warn!("Invalid username format: {}", username);
        return_error_response("Invalid username format");
    },
    Err(e) => {
        log::error!("Unexpected validation error: {}", e);
        return_error_response("Internal validation error");
    }
}
```

## Next Steps

Now that you understand the resource model, you're ready to:

1. **[Learn about multi-tenancy](./multi-tenancy.md)** for isolating resources
2. **[Explore storage providers](./providers.md)** for persistence
3. **[Understand ETag concurrency](./etag-concurrency.md)** for safe updates
4. **[Build custom resources](../tutorials/custom-resources.md)** for your domain

The resource model provides the foundation for type-safe SCIM operations. Combined with Rust's ownership system, it prevents many classes of runtime errors while maintaining the flexibility needed for diverse identity management scenarios.