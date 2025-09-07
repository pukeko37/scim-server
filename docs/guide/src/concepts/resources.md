# Resources

Resources are the foundational data structures in SCIM that represent identity objects like users, groups, and custom entities. The SCIM Server library implements a hybrid resource design that combines type safety for core attributes with JSON flexibility for extensions.

## Value Proposition

The Resource system in SCIM Server provides several key benefits:

- **Type Safety**: Core SCIM attributes use validated value objects that make invalid states unrepresentable
- **Extensibility**: Extended attributes remain as flexible JSON, preserving SCIM's extension capabilities
- **Performance**: Compile-time guarantees reduce runtime validation overhead
- **Developer Experience**: Rich type information and validation errors guide correct usage
- **Interoperability**: Full compliance with SCIM 2.0 specification while adding safety

## Architecture Overview

Resources follow a hybrid design pattern:

```text
Resource
├── Type-Safe Core Attributes (Value Objects)
│   ├── ResourceId (validated)
│   ├── UserName (validated)
│   ├── EmailAddress (validated)
│   └── SchemaUri (validated)
└── Extended Attributes (JSON Map)
    ├── Custom fields
    ├── Enterprise extensions
    └── Third-party extensions
```

### Core Validated Attributes

The following attributes use type-safe value objects with compile-time validation:

- **Resource Identity**: `ResourceId`, `ExternalId`, `SchemaUri`
- **User Attributes**: `UserName`, `Name`, `EmailAddress`, `PhoneNumber`, `Address`
- **Group Attributes**: `GroupMembers`
- **Metadata**: `Meta` (timestamps, versions, resource type)

### Extended Attributes

All other attributes are stored as JSON in the `attributes` map, providing:

- Full SCIM schema flexibility
- Support for enterprise extensions
- Custom attribute definitions
- Complex nested structures

## Use Cases

### 1. Standard SCIM Operations

**Creating a User with Core Attributes**
```rust
use scim_server::resource::Resource;
use serde_json::json;

// Resource automatically validates core attributes
let user_data = json!({
    "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
    "userName": "john.doe",
    "name": {
        "givenName": "John",
        "familyName": "Doe"
    },
    "emails": [{
        "value": "john.doe@example.com",
        "primary": true
    }]
});

let resource = Resource::from_json("User".to_string(), user_data)?;
```

**Benefits**: UserName and email validation happens at creation time, preventing invalid data from entering the system.

### 2. Enterprise Extensions

**Adding Custom Attributes**
```rust
let enterprise_user = json!({
    "schemas": [
        "urn:ietf:params:scim:schemas:core:2.0:User",
        "urn:ietf:params:scim:schemas:extension:enterprise:2.0:User"
    ],
    "userName": "jane.smith",
    "urn:ietf:params:scim:schemas:extension:enterprise:2.0:User": {
        "employeeNumber": "12345",
        "department": "Engineering",
        "manager": {
            "value": "boss@example.com",
            "displayName": "Boss Person"
        }
    }
});

let resource = Resource::from_json("User".to_string(), enterprise_user)?;
```

**Benefits**: Core attributes remain type-safe while enterprise extensions use JSON flexibility.

### 3. Custom Resource Types

**Defining Application-Specific Resources**
```rust
let application_resource = json!({
    "schemas": ["urn:example:schemas:Application"],
    "id": "app-123",
    "displayName": "My Application",
    "version": "1.2.3",
    "permissions": ["read", "write", "admin"]
});

let resource = Resource::from_json("Application".to_string(), application_resource)?;
```

**Benefits**: Resources aren't limited to Users and Groups - any JSON structure can be managed.

### 4. Multi-Valued Attributes

**Handling Complex Attribute Collections**
```rust
let user_with_multiple_emails = json!({
    "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
    "userName": "multi.user",
    "emails": [
        {
            "value": "work@example.com",
            "type": "work",
            "primary": true
        },
        {
            "value": "personal@example.com",
            "type": "home",
            "primary": false
        }
    ]
});

let resource = Resource::from_json("User".to_string(), user_with_multiple_emails)?;

// Type-safe access to primary email
if let Some(primary_email) = resource.get_primary_email() {
    println!("Primary email: {}", primary_email);
}
```

**Benefits**: Multi-valued attributes get proper validation while maintaining SCIM semantics.

## Design Patterns

### Value Object Pattern

Core attributes use the value object pattern for domain modeling:

```rust
// UserName is a value object with validation
pub struct UserName(String);

impl UserName {
    pub fn new(value: String) -> Result<Self, ValidationError> {
        if value.trim().is_empty() {
            return Err(ValidationError::EmptyField("userName".to_string()));
        }
        if value.len() > 256 {
            return Err(ValidationError::TooLong {
                field: "userName".to_string(),
                max: 256,
                actual: value.len(),
            });
        }
        Ok(UserName(value))
    }
}
```

This ensures invalid usernames cannot be constructed at compile time.

### Hybrid Serialization

Resources serialize to standard SCIM JSON while maintaining internal type safety:

```rust
let resource = Resource::from_json("User".to_string(), user_data)?;
let scim_json = resource.to_json()?; // Standard SCIM format
```

## When to Use Resources Directly

### Application Scenarios

1. **Custom SCIM Servers**: Building domain-specific identity management
2. **Data Transformation**: Converting between identity formats
3. **Validation Services**: Ensuring SCIM data integrity
4. **Testing Frameworks**: Generating valid test data

### Integration Points

Resources integrate with other SCIM Server components:

- **Storage Providers**: Resources serialize to JSON for storage
- **Resource Providers**: Business logic operates on Resources
- **HTTP Handlers**: Resources convert to/from HTTP payloads
- **Schema Validation**: Resources respect SCIM schema definitions

## Best Practices

### 1. Use Value Objects for Critical Data

Wrap important business identifiers in value objects:

```rust
// Good: Type-safe, validated
let user_id = ResourceId::new(id_string)?;

// Avoid: Stringly-typed, no validation
let user_id = id_string;
```

### 2. Preserve JSON for Flexibility

Keep non-critical attributes as JSON for extensibility:

```rust
// Good: Allows schema evolution
resource.set_attribute("customField", json!("value"));

// Avoid: Over-constraining extensions
struct CustomField(String); // Too rigid for evolving schemas
```

### 3. Handle Validation Errors Gracefully

Resource creation can fail with detailed error information:

```rust
match Resource::from_json("User".to_string(), data) {
    Ok(resource) => process_resource(resource),
    Err(ValidationError::EmptyField(field)) => {
        return_validation_error(&field);
    },
    Err(e) => log_and_handle_error(e),
}
```

### 4. Leverage Type Safety in Business Logic

Use typed accessors for core attributes:

```rust
// Type-safe access with proper error handling
if let Some(username) = resource.get_username() {
    validate_username_policy(username);
}

// Avoid raw JSON access for core attributes
let username = resource.get_attribute("userName"); // Loses type safety
```

## Comparison with Alternative Approaches

| Approach | Type Safety | Flexibility | Performance | Complexity |
|----------|-------------|-------------|-------------|------------|
| **Hybrid Design** | ✅ High (core) | ✅ High (extensions) | ✅ High | Medium |
| Full Value Objects | ✅ Very High | ❌ Low | ✅ Very High | High |
| Pure JSON | ❌ None | ✅ Very High | ⚠️ Medium | Low |
| Schema-Only | ⚠️ Runtime | ✅ High | ⚠️ Medium | Medium |

The hybrid approach provides the best balance for SCIM use cases, offering safety where it matters most while preserving the protocol's inherent flexibility.