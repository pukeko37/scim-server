# Validation Overview

SCIM Server provides built-in schema validation to ensure resources conform to SCIM 2.0 specifications. This validation is automatically applied when creating, updating, or patching resources.

## What is SCIM Validation?

SCIM Server validates resources against their defined schemas to ensure:
- **Required attributes are present** - userName for Users (displayName is optional for Groups)
- **Data types are correct** - strings, numbers, booleans, arrays as expected
- **Schema compliance** - resources match their SCIM schema definitions
- **Attribute constraints** - uniqueness, canonical values, and format requirements

## Validation Architecture

### Schema-Based Validation

All validation in SCIM Server is performed through the `SchemaRegistry`, which validates resources against SCIM 2.0 schemas:

```rust
use scim_server::{SchemaRegistry, schema::OperationContext};
use serde_json::json;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create schema registry
    let schema_registry = SchemaRegistry::new()?;
    
    // Validate user data
    let user_data = json!({
        "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
        "userName": "alice@example.com",
        "name": {
            "givenName": "Alice",
            "familyName": "Smith"
        }
    });
    
    // Perform validation
    schema_registry.validate_json_resource_with_context(
        "User",                    // Resource type
        &user_data,               // Resource data
        OperationContext::Create  // Operation being performed
    )?;
    
    println!("User data is valid!");
    Ok(())
}
```

### Operation Contexts

Validation behavior differs based on the operation being performed:

```rust
use scim_server::schema::OperationContext;

// Different validation rules apply for different operations
let operations = [
    OperationContext::Create,  // Requires all mandatory fields, no 'id' allowed
    OperationContext::Update,  // Requires 'id' field, full resource replacement
    OperationContext::Patch,   // Requires 'id' field, partial updates
];
```

## Validation Errors

When validation fails, SCIM Server returns detailed error information:

### ValidationError Types

```rust
use scim_server::ValidationError;

// Common validation errors you might encounter:

// Missing required attribute
let error = ValidationError::MissingRequiredAttribute {
    attribute: "userName".to_string()
};

// Wrong data type
let error = ValidationError::InvalidAttributeType {
    attribute: "active".to_string(),
    expected: "boolean".to_string(),
    actual: "string".to_string(),
};

// Custom validation message
let error = ValidationError::Custom {
    message: "Email domain not allowed".to_string()
};
```

### Handling Validation Errors

```rust
use scim_server::{SchemaRegistry, schema::OperationContext, ValidationError};

let schema_registry = SchemaRegistry::new()?;

match schema_registry.validate_json_resource_with_context(
    "User",
    &invalid_data,
    OperationContext::Create
) {
    Ok(_) => println!("Validation passed!"),
    Err(validation_error) => {
        match validation_error {
            ValidationError::MissingRequiredAttribute { attribute } => {
                println!("Missing required field: {}", attribute);
            },
            ValidationError::InvalidAttributeType { attribute, expected, actual } => {
                println!("Wrong type for {}: expected {}, got {}", attribute, expected, actual);
            },
            ValidationError::Custom { message } => {
                println!("Validation failed: {}", message);
            },
            _ => {
                println!("Validation error: {}", validation_error);
            }
        }
    }
}
```

## Integration with Providers

Resource providers automatically validate data during operations:

```rust
use scim_server::{StandardResourceProvider, InMemoryStorage, RequestContext};
use serde_json::json;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let storage = InMemoryStorage::new();
    let provider = StandardResourceProvider::new(storage);
    let context = RequestContext::new("validation-example".to_string());
    
    // This will automatically validate the user data
    let invalid_user = json!({
        "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
        // Missing required userName field
        "name": {
            "givenName": "Bob"
        }
    });
    
    // This will fail with a validation error
    match provider.create_resource("User", invalid_user, &context).await {
        Ok(_) => println!("User created successfully"),
        Err(e) => println!("Failed to create user: {}", e),
    }
    
    Ok(())
}
```

## Common Validation Scenarios

### User Validation

```rust
// Valid user - will pass validation
let valid_user = json!({
    "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
    "userName": "alice@example.com",    // Required
    "name": {
        "givenName": "Alice",
        "familyName": "Smith"
    },
    "emails": [
        {
            "value": "alice@example.com",
            "primary": true
        }
    ],
    "active": true
});

// Invalid user - missing userName
let invalid_user = json!({
    "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
    "name": {
        "givenName": "Bob"
    }
    // Missing required userName field
});
```

### Group Validation

```rust
// Valid group with displayName - will pass validation
let group_with_display_name = json!({
    "schemas": ["urn:ietf:params:scim:schemas:core:2.0:Group"],
    "displayName": "Engineering Team",  // Optional but recommended
    "members": [
        {
            "value": "user-id-123",
            "display": "Alice Smith",
            "type": "User"
        }
    ]
});

// Minimal group - also valid (displayName is optional in the schema)
let minimal_group = json!({
    "schemas": ["urn:ietf:params:scim:schemas:core:2.0:Group"]
    // displayName is optional per the Group schema
});
```

## Validation Best Practices

### 1. Always Handle Validation Errors

```rust
// Don't ignore validation errors
match provider.create_resource("User", user_data, &context).await {
    Ok(user) => {
        println!("User created: {}", user.get_id().unwrap_or("unknown"));
    },
    Err(e) => {
        eprintln!("Failed to create user: {}", e);
        // Handle the error appropriately
        return Err(e.into());
    }
}
```

### 2. Validate Before Operations

```rust
// Validate data before attempting operations
let schema_registry = SchemaRegistry::new()?;

// Pre-validate to catch errors early
schema_registry.validate_json_resource_with_context(
    "User",
    &user_data,
    OperationContext::Create
)?;

// Now attempt the operation
let user = provider.create_resource("User", user_data, &context).await?;
```

### 3. Provide Clear Error Messages

```rust
match validation_result {
    Err(ValidationError::MissingRequiredAttribute { attribute }) => {
        return Err(format!("Required field '{}' is missing. Please provide this field and try again.", attribute));
    },
    Err(ValidationError::InvalidAttributeType { attribute, expected, .. }) => {
        return Err(format!("Field '{}' must be of type '{}'. Please check your data format.", attribute, expected));
    },
    _ => {}
}
```

## Current Limitations

SCIM Server 0.3.7 provides comprehensive schema validation but has some limitations:

- **No custom validation rules** - Only SCIM schema validation is supported
- **No tenant-specific validation** - Same validation rules apply to all tenants  
- **No validation hooks** - Cannot add custom business logic validation
- **No validation configuration** - Validation rules are fixed by SCIM schemas

## Future Extensibility

Custom validation pipelines, business rule validation, and tenant-specific validation rules are planned for future releases. The current schema validation provides a solid foundation that will be extended with additional validation capabilities.

## Next Steps

- [Basic Validation](./basic.md) - Working with validation in practice
- [Field-Level Validation](./field-level.md) - Understanding attribute-specific validation
- [Configuration](./configuration.md) - Validation configuration options