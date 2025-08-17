# Field-Level Validation

SCIM Server validates individual fields and attributes according to SCIM 2.0 schema specifications. This guide explains how field-level validation works and what validation rules are applied to different types of attributes.

## How Field Validation Works

Field validation is performed automatically by the `SchemaRegistry` when validating resources. Each attribute in a SCIM schema has specific validation rules that are enforced.

### Basic Field Validation Example

```rust
use scim_server::{SchemaRegistry, ValidationError, schema::OperationContext};
use serde_json::json;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let registry = SchemaRegistry::new()?;
    
    // This will validate each field according to its schema definition
    let user_data = json!({
        "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
        "userName": "alice@example.com",    // String, required
        "active": true,                     // Boolean, optional
        "emails": [                         // Array of complex objects
            {
                "value": "alice@example.com",  // String, required in email object
                "primary": true                // Boolean, optional in email object
            }
        ]
    });
    
    registry.validate_json_resource_with_context(
        "User",
        &user_data,
        OperationContext::Create
    )?;
    
    println!("All fields validated successfully!");
    Ok(())
}
```

## Core User Fields Validation

### userName (Required)

```rust
// ✅ Valid userName examples
"userName": "alice@example.com"      // Email format
"userName": "alice.smith"            // Dot notation
"userName": "asmith"                 // Simple username
"userName": "alice123"               // Alphanumeric

// ❌ Invalid userName examples
"userName": ""                       // Empty string
"userName": null                     // Null value
// Missing userName entirely          // Required field missing
```

**Validation Rules:**
- Must be present (required field)
- Must be a non-empty string
- No specific format requirements (can be email, username, etc.)

### active (Optional)

```rust
// ✅ Valid active examples
"active": true                       // Boolean true
"active": false                      // Boolean false
// Missing active entirely           // Optional field

// ❌ Invalid active examples
"active": "true"                     // String instead of boolean
"active": 1                         // Number instead of boolean
"active": null                      // Null value (should be omitted)
```

**Validation Rules:**
- Must be a boolean when present
- Defaults to true if not specified

### name (Optional Complex Object)

```rust
// ✅ Valid name examples
"name": {
    "givenName": "Alice",
    "familyName": "Smith",
    "formatted": "Alice Smith"
}

"name": {
    "givenName": "Alice"            // Only givenName provided
}

"name": {
    "familyName": "Smith",
    "givenName": "Alice",
    "middleName": "Marie",
    "honorificPrefix": "Dr.",
    "honorificSuffix": "PhD"
}

// ❌ Invalid name examples
"name": "Alice Smith"               // String instead of object
"name": {}                         // Empty object (should be omitted)
"name": {
    "givenName": 123               // Number instead of string
}
```

**Validation Rules:**
- Must be an object when present
- All sub-attributes must be strings
- No required sub-attributes
- Common sub-attributes: `givenName`, `familyName`, `formatted`, `middleName`, `honorificPrefix`, `honorificSuffix`

### emails (Optional Multi-Valued)

```rust
// ✅ Valid emails examples
"emails": [
    {
        "value": "alice@example.com",
        "primary": true
    }
]

"emails": [
    {
        "value": "alice@work.com",
        "type": "work",
        "primary": true
    },
    {
        "value": "alice@personal.com",
        "type": "home",
        "primary": false
    }
]

// ❌ Invalid emails examples
"emails": "alice@example.com"       // String instead of array
"emails": [
    {
        "primary": true             // Missing required 'value'
    }
]
"emails": [
    {
        "value": "invalid-email",   // Invalid email format
        "primary": "true"           // String instead of boolean
    }
]
```

**Validation Rules:**
- Must be an array when present
- Each email object must have a `value` field (required)
- `value` must be a valid email address string
- `type` must be a string when present
- `primary` must be a boolean when present
- Only one email can have `primary: true`

### phoneNumbers (Optional Multi-Valued)

```rust
// ✅ Valid phoneNumbers examples
"phoneNumbers": [
    {
        "value": "+1-555-123-4567",
        "type": "work"
    }
]

"phoneNumbers": [
    {
        "value": "+1-555-123-4567",
        "type": "work",
        "primary": true
    },
    {
        "value": "+1-555-987-6543", 
        "type": "mobile",
        "primary": false
    }
]

// ❌ Invalid phoneNumbers examples
"phoneNumbers": "+1-555-123-4567"   // String instead of array
"phoneNumbers": [
    {
        "type": "work"              // Missing required 'value'
    }
]
```

**Validation Rules:**
- Must be an array when present
- Each phone object must have a `value` field (required)
- `value` must be a string (format validation varies)
- `type` must be a string when present
- `primary` must be a boolean when present

## Core Group Fields Validation

### displayName (Optional)

```rust
// ✅ Valid displayName examples
"displayName": "Engineering Team"
"displayName": "HR Department"
"displayName": "Project Alpha"

// ❌ Invalid displayName examples (when provided)
"displayName": ""                   // Empty string not recommended
"displayName": null                 // Null value not recommended
// Missing displayName entirely is allowed per schema
```

**Validation Rules:**
- Optional field (though recommended for clarity)
- When provided, must be a non-empty string

### members (Optional Multi-Valued)

```rust
// ✅ Valid members examples
"members": []                       // Empty array is valid

"members": [
    {
        "value": "user-123",
        "display": "Alice Smith",
        "type": "User"
    }
]

"members": [
    {
        "value": "user-123",
        "display": "Alice Smith"
    },
    {
        "value": "user-456",
        "display": "Bob Jones",
        "type": "User"
    }
]

// ❌ Invalid members examples
"members": "user-123"               // String instead of array
"members": [
    {
        "display": "Alice Smith"    // Missing required 'value'
    }
]
"members": [
    {
        "value": 123,               // Number instead of string
        "display": "Alice Smith"
    }
]
```

**Validation Rules:**
- Must be an array when present
- Each member object must have a `value` field (required)
- `value` must be a string (typically a user ID)
- `display` must be a string when present
- `type` must be a string when present (typically "User")

## Common Validation Patterns

### Required vs Optional Fields

```rust
use scim_server::{SchemaRegistry, ValidationError, schema::OperationContext};
use serde_json::json;

async fn test_required_fields() -> Result<(), Box<dyn std::error::Error>> {
    let registry = SchemaRegistry::new()?;
    
    // Test missing required field
    let user_missing_username = json!({
        "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
        "displayName": "Alice Smith"
        // Missing required userName
    });
    
    match registry.validate_json_resource_with_context(
        "User",
        &user_missing_username,
        OperationContext::Create
    ) {
        Ok(_) => println!("❌ Should have failed!"),
        Err(ValidationError::MissingRequiredAttribute { attribute }) => {
            println!("✅ Correctly caught missing field: {}", attribute);
        }
        Err(e) => println!("❌ Unexpected error: {}", e),
    }
    
    Ok(())
}
```

### Data Type Validation

```rust
async fn test_data_types() -> Result<(), Box<dyn std::error::Error>> {
    let registry = SchemaRegistry::new()?;
    
    // Test wrong data type
    let user_wrong_type = json!({
        "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
        "userName": "alice@example.com",
        "active": "true"  // String instead of boolean
    });
    
    match registry.validate_json_resource_with_context(
        "User",
        &user_wrong_type,
        OperationContext::Create
    ) {
        Ok(_) => println!("❌ Should have failed!"),
        Err(ValidationError::InvalidAttributeType { attribute, expected, actual }) => {
            println!("✅ Correctly caught type error:");
            println!("   Field: {}", attribute);
            println!("   Expected: {}", expected);
            println!("   Got: {}", actual);
        }
        Err(e) => println!("❌ Unexpected error: {}", e),
    }
    
    Ok(())
}
```

### Multi-Valued Attribute Validation

```rust
async fn test_multi_valued_attributes() -> Result<(), Box<dyn std::error::Error>> {
    let registry = SchemaRegistry::new()?;
    
    // Test emails as single value instead of array
    let user_wrong_emails = json!({
        "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
        "userName": "alice@example.com",
        "emails": "alice@example.com"  // Should be array
    });
    
    match registry.validate_json_resource_with_context(
        "User",
        &user_wrong_emails,
        OperationContext::Create
    ) {
        Ok(_) => println!("❌ Should have failed!"),
        Err(ValidationError::ExpectedMultiValue { attribute }) => {
            println!("✅ Correctly caught multi-value error: {}", attribute);
        }
        Err(e) => println!("❌ Unexpected error: {}", e),
    }
    
    Ok(())
}
```

## Field Validation in Different Contexts

### Create Operation

```rust
// Create operations validate:
// - All required fields are present
// - No 'id' field is provided (server generates this)
// - No 'meta' fields are provided (server generates these)

let create_data = json!({
    "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
    "userName": "alice@example.com"  // Required field present
    // No 'id' field - good for create
});
```

### Update Operation

```rust
// Update operations validate:
// - 'id' field is present and valid
// - All provided fields are valid
// - Complete resource replacement

let update_data = json!({
    "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
    "id": "user-123",              // Required for update
    "userName": "alice@example.com",
    "displayName": "Alice Smith"
});
```

### Patch Operation

```rust
// Patch operations validate:
// - 'id' field is present
// - Partial updates are allowed
// - Patch operations themselves have validation

let patch_data = json!({
    "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
    "id": "user-123",              // Required for patch
    "displayName": "Alice Johnson" // Only updating this field
});
```

## Complex Attribute Validation

### Nested Object Validation

```rust
// Name object validation
let user_with_name = json!({
    "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
    "userName": "alice@example.com",
    "name": {
        "givenName": "Alice",      // String validation
        "familyName": "Smith",     // String validation
        "formatted": "Alice Smith" // String validation
    }
});

// Each field in the name object is validated individually
```

### Array of Objects Validation

```rust
// Emails array validation
let user_with_emails = json!({
    "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
    "userName": "alice@example.com",
    "emails": [
        {
            "value": "alice@work.com",     // Required in each email object
            "type": "work",                // Optional string
            "primary": true                // Optional boolean
        },
        {
            "value": "alice@personal.com", // Required in each email object
            "type": "home",                // Optional string
            "primary": false               // Optional boolean
        }
    ]
});

// Each object in the emails array is validated individually
```

## Error Messages and Field Paths

Field validation errors include specific field paths to help identify exactly which field failed validation:

```rust
use scim_server::ValidationError;

// Examples of field paths in validation errors:

// "userName" - top level field
ValidationError::MissingRequiredAttribute { 
    attribute: "userName".to_string() 
};

// "name.givenName" - nested object field
ValidationError::InvalidAttributeType {
    attribute: "name.givenName".to_string(),
    expected: "string".to_string(),
    actual: "number".to_string(),
};

// "emails[0].value" - array element field
ValidationError::MissingRequiredAttribute {
    attribute: "emails[0].value".to_string()
};

// "phoneNumbers[1].primary" - nested array element field
ValidationError::InvalidAttributeType {
    attribute: "phoneNumbers[1].primary".to_string(),
    expected: "boolean".to_string(),
    actual: "string".to_string(),
};
```

## Custom Validation Integration

While SCIM Server handles all schema-based field validation automatically, you can add custom field validation in your application:

```rust
use scim_server::{SchemaRegistry, ValidationError, schema::OperationContext};
use serde_json::Value;

async fn validate_with_custom_field_rules(
    resource_type: &str,
    data: &Value,
    operation: OperationContext
) -> Result<(), String> {
    let registry = SchemaRegistry::new()
        .map_err(|e| format!("Schema registry error: {}", e))?;
    
    // First: Standard SCIM field validation
    registry.validate_json_resource_with_context(resource_type, data, operation)
        .map_err(|e| format!("SCIM field validation failed: {}", e))?;
    
    // Then: Your custom field validation
    validate_custom_field_rules(data).await?;
    
    Ok(())
}

async fn validate_custom_field_rules(data: &Value) -> Result<(), String> {
    // Example: Custom email domain validation
    if let Some(emails) = data.get("emails").and_then(|e| e.as_array()) {
        for (index, email) in emails.iter().enumerate() {
            if let Some(email_value) = email.get("value").and_then(|v| v.as_str()) {
                if let Some(domain) = email_value.split('@').nth(1) {
                    if !is_corporate_domain(domain) {
                        return Err(format!(
                            "emails[{}].value: Domain '{}' is not allowed", 
                            index, domain
                        ));
                    }
                }
            }
        }
    }
    
    // Example: Custom username format validation
    if let Some(username) = data.get("userName").and_then(|u| u.as_str()) {
        if username.len() < 3 {
            return Err("userName: Must be at least 3 characters long".to_string());
        }
        
        if !username.chars().all(|c| c.is_alphanumeric() || c == '.' || c == '@' || c == '_') {
            return Err("userName: Contains invalid characters".to_string());
        }
    }
    
    Ok(())
}

fn is_corporate_domain(domain: &str) -> bool {
    let allowed_domains = ["company.com", "subsidiary.com"];
    allowed_domains.contains(&domain.to_lowercase().as_str())
}
```

## Best Practices

### 1. Understand Required vs Optional

```rust
// Always provide required fields
let minimal_user = json!({
    "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
    "userName": "alice@example.com"  // userName is required
});

let minimal_group = json!({
    "schemas": ["urn:ietf:params:scim:schemas:core:2.0:Group"],
    "displayName": "Engineering Team"  // displayName is optional but recommended
});
```

### 2. Use Correct Data Types

```rust
// Use proper data types for each field
let properly_typed_user = json!({
    "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
    "userName": "alice@example.com",   // String
    "active": true,                    // Boolean, not "true"
    "emails": [                        // Array, not single object
        {
            "value": "alice@example.com",  // String
            "primary": true                // Boolean, not "true"
        }
    ]
});
```

### 3. Handle Validation Errors Gracefully

```rust
use scim_server::ValidationError;

fn handle_field_validation_error(error: ValidationError) -> String {
    match error {
        ValidationError::MissingRequiredAttribute { attribute } => {
            format!("Required field '{}' is missing. Please provide this field.", attribute)
        },
        ValidationError::InvalidAttributeType { attribute, expected, actual } => {
            format!("Field '{}' has wrong type. Expected {}, but got {}.", 
                   attribute, expected, actual)
        },
        ValidationError::ExpectedMultiValue { attribute } => {
            format!("Field '{}' must be an array, not a single value.", attribute)
        },
        ValidationError::ExpectedSingleValue { attribute } => {
            format!("Field '{}' must be a single value, not an array.", attribute)
        },
        _ => format!("Field validation error: {}", error)
    }
}
```

## Common Field Validation Errors

### Missing Required Fields
```
Error: Required attribute 'userName' is missing
Note: displayName is optional for Groups per the SCIM schema
```

### Wrong Data Types
```
Error: Attribute 'active' has invalid type, expected boolean, got string
Error: Attribute 'emails[0].primary' has invalid type, expected boolean, got string
```

### Multi-Value vs Single-Value
```
Error: Attribute 'emails' must be multi-valued (array)
Error: Attribute 'userName' must be single-valued (not array)
```

### Schema Issues
```
Error: Missing required 'schemas' attribute
Error: 'schemas' array cannot be empty
Error: Unknown schema URI: 'invalid:schema:uri'
```

Field-level validation in SCIM Server ensures that all attributes conform to the SCIM 2.0 specification, providing a solid foundation for data integrity and interoperability.

## Next Steps

- [Configuration](./configuration.md) - Learn about validation configuration options
- [Basic Validation](./basic.md) - See complete validation examples in practice
- [Overview](./overview.md) - Understand the overall validation architecture