# Basic Validation

This guide covers practical validation scenarios using SCIM Server's built-in schema validation capabilities. You'll learn how to validate resources, handle validation errors, and integrate validation into your applications.

## Quick Start

The simplest way to validate SCIM resources is using the `SchemaRegistry`:

```rust
use scim_server::{SchemaRegistry, schema::OperationContext};
use serde_json::json;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let registry = SchemaRegistry::new()?;
    
    let user_data = json!({
        "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
        "userName": "alice@example.com"
    });
    
    // Validate the user data
    registry.validate_json_resource_with_context(
        "User",
        &user_data,
        OperationContext::Create
    )?;
    
    println!("User data is valid!");
    Ok(())
}
```

## Validation Patterns

### 1. Pre-Validation Pattern

Validate data before attempting to create/update resources:

```rust
use scim_server::{SchemaRegistry, StandardResourceProvider, InMemoryStorage, RequestContext};
use scim_server::schema::OperationContext;
use serde_json::json;

async fn create_user_safely(
    user_data: serde_json::Value
) -> Result<scim_server::Resource, Box<dyn std::error::Error>> {
    let registry = SchemaRegistry::new()?;
    
    // Step 1: Pre-validate the data
    registry.validate_json_resource_with_context(
        "User",
        &user_data,
        OperationContext::Create
    )?;
    
    // Step 2: If validation passes, create the resource
    let storage = InMemoryStorage::new();
    let provider = StandardResourceProvider::new(storage);
    let context = RequestContext::new("validation-example".to_string());
    
    let user = provider.create_resource("User", user_data, &context).await?;
    
    Ok(user)
}
```

### 2. Error Handling Pattern

Handle specific validation errors with appropriate responses:

```rust
use scim_server::{SchemaRegistry, ValidationError, schema::OperationContext};
use serde_json::json;

fn validate_and_handle_errors(user_data: serde_json::Value) -> Result<(), String> {
    let registry = SchemaRegistry::new()
        .map_err(|e| format!("Failed to create registry: {}", e))?;
    
    match registry.validate_json_resource_with_context(
        "User",
        &user_data,
        OperationContext::Create
    ) {
        Ok(_) => {
            println!("✅ Validation successful");
            Ok(())
        },
        Err(validation_error) => {
            let error_message = match validation_error {
                ValidationError::MissingRequiredAttribute { attribute } => {
                    format!("❌ Missing required field: '{}'", attribute)
                },
                ValidationError::InvalidAttributeType { attribute, expected, actual } => {
                    format!("❌ Wrong type for '{}': expected {}, got {}", attribute, expected, actual)
                },
                ValidationError::MissingSchemas => {
                    "❌ Missing 'schemas' field - this is required for all SCIM resources".to_string()
                },
                ValidationError::EmptySchemas => {
                    "❌ The 'schemas' array cannot be empty".to_string()
                },
                ValidationError::UnknownSchemaUri { uri } => {
                    format!("❌ Unknown schema URI: '{}'", uri)
                },
                ValidationError::Custom { message } => {
                    format!("❌ Validation failed: {}", message)
                },
                _ => format!("❌ Validation error: {}", validation_error)
            };
            
            eprintln!("{}", error_message);
            Err(error_message)
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Test with valid data
    let valid_user = json!({
        "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
        "userName": "alice@example.com"
    });
    
    validate_and_handle_errors(valid_user)?;
    
    // Test with invalid data
    let invalid_user = json!({
        "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"]
        // Missing required userName
    });
    
    let _ = validate_and_handle_errors(invalid_user); // Will print error message
    
    Ok(())
}
```

## Common Validation Scenarios

### User Validation Examples

```rust
use scim_server::{SchemaRegistry, schema::OperationContext};
use serde_json::json;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let registry = SchemaRegistry::new()?;
    
    // ✅ Valid minimal user
    let minimal_user = json!({
        "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
        "userName": "alice@example.com"
    });
    
    registry.validate_json_resource_with_context(
        "User", &minimal_user, OperationContext::Create
    )?;
    println!("✅ Minimal user validated");
    
    // ✅ Valid complete user
    let complete_user = json!({
        "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
        "userName": "bob@example.com",
        "name": {
            "givenName": "Bob",
            "familyName": "Smith",
            "formatted": "Bob Smith"
        },
        "displayName": "Bob Smith",
        "emails": [
            {
                "value": "bob@example.com",
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
    
    registry.validate_json_resource_with_context(
        "User", &complete_user, OperationContext::Create
    )?;
    println!("✅ Complete user validated");
    
    // ❌ Invalid user - missing userName
    let invalid_user = json!({
        "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
        "displayName": "Charlie Brown"
        // Missing required userName
    });
    
    match registry.validate_json_resource_with_context(
        "User", &invalid_user, OperationContext::Create
    ) {
        Ok(_) => println!("❌ This should have failed!"),
        Err(e) => println!("✅ Correctly caught error: {}", e),
    }
    
    Ok(())
}
```

### Group Validation Examples

```rust
use scim_server::{SchemaRegistry, schema::OperationContext};
use serde_json::json;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let registry = SchemaRegistry::new()?;
    
    // ✅ Valid minimal group
    let minimal_group = json!({
        "schemas": ["urn:ietf:params:scim:schemas:core:2.0:Group"],
        "displayName": "Engineering Team"
    });
    
    registry.validate_json_resource_with_context(
        "Group", &minimal_group, OperationContext::Create
    )?;
    println!("✅ Minimal group validated");
    
    // ✅ Valid group with members
    let group_with_members = json!({
        "schemas": ["urn:ietf:params:scim:schemas:core:2.0:Group"],
        "displayName": "Development Team",
        "members": [
            {
                "value": "user-123",
                "display": "Alice Smith",
                "type": "User"
            },
            {
                "value": "user-456", 
                "display": "Bob Jones",
                "type": "User"
            }
        ]
    });
    
    registry.validate_json_resource_with_context(
        "Group", &group_with_members, OperationContext::Create
    )?;
    println!("✅ Group with members validated");
    
    // ✅ Minimal group - displayName is optional per the schema
    let minimal_group = json!({
        "schemas": ["urn:ietf:params:scim:schemas:core:2.0:Group"]
        // displayName is optional (though recommended)
    });
    
    match registry.validate_json_resource_with_context(
        "Group", &minimal_group, OperationContext::Create
    ) {
        Ok(_) => println!("✅ Minimal group validation passed"),
        Err(e) => println!("❌ Minimal group validation failed: {}", e),
    }
    
    Ok(())
}
```

## Operation Context Validation

Different operations have different validation requirements:

```rust
use scim_server::{SchemaRegistry, schema::OperationContext};
use serde_json::json;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let registry = SchemaRegistry::new()?;
    
    // Create operation - no 'id' field allowed
    let create_data = json!({
        "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
        "userName": "alice@example.com"
    });
    
    registry.validate_json_resource_with_context(
        "User", &create_data, OperationContext::Create
    )?;
    println!("✅ Create validation passed");
    
    // Update operation - requires 'id' field
    let update_data = json!({
        "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
        "id": "user-123",
        "userName": "alice@example.com",
        "displayName": "Alice Smith"
    });
    
    registry.validate_json_resource_with_context(
        "User", &update_data, OperationContext::Update
    )?;
    println!("✅ Update validation passed");
    
    // Patch operation - also requires 'id' field
    let patch_data = json!({
        "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
        "id": "user-123",
        "displayName": "Alice Johnson" // Only updating display name
    });
    
    registry.validate_json_resource_with_context(
        "User", &patch_data, OperationContext::Patch
    )?;
    println!("✅ Patch validation passed");
    
    Ok(())
}
```

## Integration with Resource Providers

Validation is automatically applied when using resource providers:

```rust
use scim_server::{StandardResourceProvider, InMemoryStorage, RequestContext, ResourceProvider};
use serde_json::json;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let storage = InMemoryStorage::new();
    let provider = StandardResourceProvider::new(storage);
    let context = RequestContext::new("validation-test".to_string());
    
    // This will automatically validate before creating
    let valid_user = json!({
        "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
        "userName": "alice@example.com",
        "displayName": "Alice Smith"
    });
    
    match provider.create_resource("User", valid_user, &context).await {
        Ok(user) => {
            println!("✅ User created successfully: {}", 
                     user.get_username().unwrap_or("unknown"));
        },
        Err(e) => {
            println!("❌ Failed to create user: {}", e);
        }
    }
    
    // This will fail validation
    let invalid_user = json!({
        "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
        "displayName": "Bob Smith"
        // Missing required userName
    });
    
    match provider.create_resource("User", invalid_user, &context).await {
        Ok(_) => {
            println!("❌ This should have failed validation!");
        },
        Err(e) => {
            println!("✅ Validation correctly prevented creation: {}", e);
        }
    }
    
    Ok(())
}
```

## Best Practices

### 1. Validate Early and Often

```rust
// Validate as soon as you receive data
fn process_user_request(request_data: serde_json::Value) -> Result<(), String> {
    let registry = SchemaRegistry::new()
        .map_err(|e| format!("Registry error: {}", e))?;
    
    // Validate immediately
    registry.validate_json_resource_with_context(
        "User", 
        &request_data, 
        OperationContext::Create
    ).map_err(|e| format!("Validation failed: {}", e))?;
    
    // Continue processing knowing data is valid
    Ok(())
}
```

### 2. Provide Helpful Error Messages

```rust
use scim_server::ValidationError;

fn user_friendly_error(error: ValidationError) -> String {
    match error {
        ValidationError::MissingRequiredAttribute { attribute } => {
            match attribute.as_str() {
                "userName" => "Username is required. Please provide a valid username.".to_string(),
                "displayName" => "Display name is missing (though it's optional for groups).".to_string(),
                _ => format!("Required field '{}' is missing.", attribute)
            }
        },
        ValidationError::InvalidAttributeType { attribute, expected, .. } => {
            format!("The field '{}' must be a {}. Please check your data format.", attribute, expected)
        },
        ValidationError::MissingSchemas => {
            "Missing 'schemas' field. All SCIM resources must include a 'schemas' array.".to_string()
        },
        _ => format!("Validation error: {}", error)
    }
}
```

### 3. Handle Different Resource Types

```rust
async fn validate_resource(
    resource_type: &str,
    data: &serde_json::Value,
    operation: OperationContext
) -> Result<(), ValidationError> {
    let registry = SchemaRegistry::new()?;
    
    match resource_type {
        "User" | "Group" => {
            registry.validate_json_resource_with_context(
                resource_type,
                data,
                operation
            )
        },
        _ => Err(ValidationError::Custom {
            message: format!("Unsupported resource type: {}", resource_type)
        })
    }
}
```

## Common Validation Errors

### Missing Required Fields

```rust
// This will fail - missing userName
let user = json!({
    "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
    "displayName": "John Doe"
});
// Error: ValidationError::MissingRequiredAttribute { attribute: "userName" }
```

### Wrong Data Types

```rust
// This will fail - active should be boolean, not string
let user = json!({
    "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
    "userName": "john@example.com",
    "active": "true"  // Should be: true
});
// Error: ValidationError::InvalidAttributeType
```

### Missing or Invalid Schemas

```rust
// This will fail - missing schemas array
let user = json!({
    "userName": "john@example.com"
});
// Error: ValidationError::MissingSchemas

// This will fail - empty schemas array
let user = json!({
    "schemas": [],
    "userName": "john@example.com"
});
// Error: ValidationError::EmptySchemas
```

## Next Steps

- [Field-Level Validation](./field-level.md) - Understand how specific attributes are validated
- [Configuration](./configuration.md) - Learn about validation configuration options