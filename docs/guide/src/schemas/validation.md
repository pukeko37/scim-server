# Validation

Schema validation in SCIM ensures that resources conform to their defined schemas before being stored or processed. This section covers the current validation capabilities and limitations in the SCIM Server library.

## Current Status

⚠️ **Basic Validation Only**: The SCIM Server library currently provides basic type validation and constraint checking, but lacks a comprehensive validation framework. Most validation is handled internally by the storage providers and resource handlers.

## What Works Today

### Built-in Type Validation

The schema system provides basic type validation for SCIM attributes:

```rust
use scim_server::schema::{AttributeType, SchemaRegistry};
use serde_json::json;

// Basic type validation happens automatically
let registry = SchemaRegistry::new()?;
let user_schema = registry.get_user_schema();

// Valid user data
let valid_user = json!({
    "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
    "userName": "john.doe@example.com",  // String type
    "active": true,                      // Boolean type
    "name": {                           // Complex type
        "givenName": "John",
        "familyName": "Doe"
    }
});

// Invalid data would be caught by JSON parsing or type checking
let invalid_user = json!({
    "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
    "userName": 12345,                   // Wrong type - should be string
    "active": "not_boolean"              // Wrong type - should be boolean
});
```

### Schema Constraint Validation

Basic constraint validation is available through the schema definition:

```rust
use scim_server::schema::{AttributeDefinition, AttributeType, Mutability, Uniqueness};

// Example attribute with constraints
let username_attr = AttributeDefinition {
    name: "userName".to_string(),
    data_type: AttributeType::String,
    multi_valued: false,
    required: true,                      // REQUIRED validation
    case_exact: false,
    mutability: Mutability::ReadWrite,
    uniqueness: Uniqueness::Server,      // UNIQUENESS validation
    canonical_values: vec![],
    sub_attributes: vec![],
    returned: Some("always".to_string()),
};

// Validation happens based on these constraints:
// - required: true means the field must be present
// - uniqueness: Server means values must be unique across the server
// - data_type: String means value must be a string
```

### Canonical Values Validation

Attributes can define allowed values:

```rust
let status_attr = AttributeDefinition {
    name: "status".to_string(),
    data_type: AttributeType::String,
    multi_valued: false,
    required: false,
    case_exact: false,
    mutability: Mutability::ReadWrite,
    uniqueness: Uniqueness::None,
    canonical_values: vec![
        "active".to_string(),
        "inactive".to_string(),
        "suspended".to_string(),
    ],                                   // Only these values allowed
    sub_attributes: vec![],
    returned: Some("default".to_string()),
};
```

### Format Validation Helpers

The schema registry provides some basic format validation:

```rust
use scim_server::schema::SchemaRegistry;

let registry = SchemaRegistry::new()?;

// These methods are internal but show what validation exists:
// registry.is_valid_datetime_format("2023-01-01T00:00:00Z")  // RFC3339 validation
// registry.is_valid_base64("SGVsbG8gV29ybGQ=")               // Base64 validation  
// registry.is_valid_uri_format("https://example.com")       // Basic URI validation

// Note: These are currently private methods (pub(super))
```

## Current Limitations

### No High-Level Validation API

The library lacks a public validation API:

```rust
// ❌ This doesn't exist:
// let validator = SchemaValidator::new();
// validator.validate_resource(&resource_data, &schema)?;

// ❌ This doesn't exist:
// registry.validate_resource(&resource_data, &schema_uris)?;
```

### No Custom Validation Rules

There's no system for defining custom business logic validation:

```rust
// ❌ This doesn't exist:
// schema.add_validation_rule("employeeNumber", |value| {
//     value.len() == 8 && value.chars().all(|c| c.is_numeric())
// });
```

### No Cross-Field Validation

No support for validating relationships between fields:

```rust
// ❌ This doesn't exist:
// schema.add_cross_field_rule(|resource| {
//     if resource.start_date > resource.end_date {
//         return Err("Start date must be before end date");
//     }
//     Ok(())
// });
```

## Working Validation Patterns

### Pattern 1: Provider-Level Validation

Implement validation in your resource provider:

```rust
use scim_server::resource::{ResourceProvider, RequestContext};
use scim_server::error::ScimError;
use serde_json::Value;

pub struct ValidatingProvider<T: ResourceProvider> {
    inner: T,
}

impl<T: ResourceProvider> ValidatingProvider<T> {
    pub fn new(inner: T) -> Self {
        Self { inner }
    }
    
    fn validate_user(&self, data: &Value) -> Result<(), ScimError> {
        // Custom validation logic
        if let Some(username) = data.get("userName").and_then(|v| v.as_str()) {
            if username.is_empty() {
                return Err(ScimError::InvalidData("userName cannot be empty".to_string()));
            }
            if !username.contains('@') {
                return Err(ScimError::InvalidData("userName must be an email".to_string()));
            }
        }
        
        if let Some(emails) = data.get("emails").and_then(|v| v.as_array()) {
            for email in emails {
                if let Some(value) = email.get("value").and_then(|v| v.as_str()) {
                    if !value.contains('@') {
                        return Err(ScimError::InvalidData("Invalid email format".to_string()));
                    }
                }
            }
        }
        
        Ok(())
    }
}

#[async_trait::async_trait]
impl<T: ResourceProvider> ResourceProvider for ValidatingProvider<T> {
    async fn create_resource(
        &self,
        resource_type: &str,
        data: Value,
        context: &RequestContext,
    ) -> Result<Value, ScimError> {
        // Validate before creating
        if resource_type == "User" {
            self.validate_user(&data)?;
        }
        
        // Delegate to inner provider
        self.inner.create_resource(resource_type, data, context).await
    }
    
    async fn update_resource(
        &self,
        resource_type: &str,
        id: &str,
        data: Value,
        context: &RequestContext,
    ) -> Result<Value, ScimError> {
        // Validate before updating
        if resource_type == "User" {
            self.validate_user(&data)?;
        }
        
        self.inner.update_resource(resource_type, id, data, context).await
    }
    
    // Implement other required methods...
}
```

### Pattern 2: Resource Handler Validation

Add validation to resource handlers:

```rust
use scim_server::SchemaResourceBuilder;
use serde_json::Value;

fn create_validated_user_handler(schema: scim_server::schema::Schema) -> scim_server::resource::ResourceHandler {
    SchemaResourceBuilder::new(schema)
        .with_setter("userName", |data, value| {
            // Validate userName when setting
            if let Some(username) = value.as_str() {
                if username.is_empty() {
                    return Err(scim_server::error::ScimError::InvalidData(
                        "userName cannot be empty".to_string()
                    ));
                }
                if !username.contains('@') {
                    return Err(scim_server::error::ScimError::InvalidData(
                        "userName must be an email address".to_string()
                    ));
                }
            }
            
            // Set the value if validation passes
            if let Some(obj) = data.as_object_mut() {
                obj.insert("userName".to_string(), value);
            }
            Ok(())
        })
        .with_setter("emails", |data, value| {
            // Validate emails array
            if let Some(emails) = value.as_array() {
                for email in emails {
                    if let Some(email_value) = email.get("value").and_then(|v| v.as_str()) {
                        if !email_value.contains('@') {
                            return Err(scim_server::error::ScimError::InvalidData(
                                format!("Invalid email format: {}", email_value)
                            ));
                        }
                    }
                }
            }
            
            if let Some(obj) = data.as_object_mut() {
                obj.insert("emails".to_string(), value);
            }
            Ok(())
        })
        .build()
}
```

### Pattern 3: Validation Utilities

Create reusable validation functions:

```rust
use serde_json::Value;
use scim_server::error::ScimError;

pub struct ValidationUtils;

impl ValidationUtils {
    pub fn validate_email(email: &str) -> Result<(), ScimError> {
        if email.is_empty() {
            return Err(ScimError::InvalidData("Email cannot be empty".to_string()));
        }
        
        if !email.contains('@') {
            return Err(ScimError::InvalidData("Email must contain @ symbol".to_string()));
        }
        
        if !email.contains('.') {
            return Err(ScimError::InvalidData("Email must contain domain".to_string()));
        }
        
        Ok(())
    }
    
    pub fn validate_phone_number(phone: &str) -> Result<(), ScimError> {
        if phone.is_empty() {
            return Err(ScimError::InvalidData("Phone number cannot be empty".to_string()));
        }
        
        // Basic phone validation - customize as needed
        let digits_only: String = phone.chars().filter(|c| c.is_numeric()).collect();
        if digits_only.len() < 10 {
            return Err(ScimError::InvalidData("Phone number too short".to_string()));
        }
        
        Ok(())
    }
    
    pub fn validate_date_format(date_str: &str) -> Result<(), ScimError> {
        use chrono::{DateTime, FixedOffset};
        
        DateTime::<FixedOffset>::parse_from_rfc3339(date_str)
            .map_err(|_| ScimError::InvalidData(format!("Invalid date format: {}", date_str)))?;
        
        Ok(())
    }
    
    pub fn validate_user_resource(data: &Value) -> Result<(), ScimError> {
        // Validate required fields
        if data.get("userName").is_none() {
            return Err(ScimError::InvalidData("userName is required".to_string()));
        }
        
        // Validate userName format
        if let Some(username) = data.get("userName").and_then(|v| v.as_str()) {
            Self::validate_email(username)?;
        }
        
        // Validate emails array
        if let Some(emails) = data.get("emails").and_then(|v| v.as_array()) {
            for email in emails {
                if let Some(value) = email.get("value").and_then(|v| v.as_str()) {
                    Self::validate_email(value)?;
                }
            }
        }
        
        // Validate phone numbers
        if let Some(phones) = data.get("phoneNumbers").and_then(|v| v.as_array()) {
            for phone in phones {
                if let Some(value) = phone.get("value").and_then(|v| v.as_str()) {
                    Self::validate_phone_number(value)?;
                }
            }
        }
        
        Ok(())
    }
}

// Usage in your application
fn validate_and_create_user(user_data: Value) -> Result<(), ScimError> {
    // Validate the user data
    ValidationUtils::validate_user_resource(&user_data)?;
    
    // Proceed with creation if validation passes
    println!("User data is valid");
    Ok(())
}
```

### Pattern 4: Schema-Based Validation

Use schema information for validation:

```rust
use scim_server::schema::{SchemaRegistry, AttributeType};
use serde_json::Value;

pub fn validate_against_schema(
    data: &Value,
    schema_id: &str,
    registry: &SchemaRegistry,
) -> Result<(), ScimError> {
    let schema = registry.get_schema(schema_id)
        .ok_or_else(|| ScimError::InvalidData(format!("Unknown schema: {}", schema_id)))?;
    
    for attr in &schema.attributes {
        let value = data.get(&attr.name);
        
        // Check required attributes
        if attr.required && value.is_none() {
            return Err(ScimError::InvalidData(
                format!("Required attribute '{}' is missing", attr.name)
            ));
        }
        
        if let Some(value) = value {
            // Check data type
            let valid_type = match attr.data_type {
                AttributeType::String => value.is_string(),
                AttributeType::Boolean => value.is_boolean(),
                AttributeType::Integer => value.is_i64(),
                AttributeType::Decimal => value.is_f64(),
                AttributeType::DateTime => {
                    // For DateTime, we expect a string in RFC3339 format
                    value.is_string() && value.as_str()
                        .map(|s| chrono::DateTime::parse_from_rfc3339(s).is_ok())
                        .unwrap_or(false)
                },
                AttributeType::Complex => value.is_object(),
                AttributeType::Reference => value.is_string(),
                AttributeType::Binary => {
                    // For Binary, we expect a base64-encoded string
                    value.is_string()
                },
            };
            
            if !valid_type {
                return Err(ScimError::InvalidData(
                    format!("Attribute '{}' has invalid type", attr.name)
                ));
            }
            
            // Check canonical values
            if !attr.canonical_values.is_empty() {
                if let Some(str_value) = value.as_str() {
                    if !attr.canonical_values.contains(&str_value.to_string()) {
                        return Err(ScimError::InvalidData(
                            format!("Attribute '{}' has invalid value. Allowed: {:?}", 
                                   attr.name, attr.canonical_values)
                        ));
                    }
                }
            }
        }
    }
    
    Ok(())
}
```

## Testing Validation

### Unit Tests for Validation Logic

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_email_validation() {
        // Valid emails
        assert!(ValidationUtils::validate_email("user@example.com").is_ok());
        assert!(ValidationUtils::validate_email("test.user+tag@domain.co.uk").is_ok());
        
        // Invalid emails
        assert!(ValidationUtils::validate_email("").is_err());
        assert!(ValidationUtils::validate_email("notanemail").is_err());
        assert!(ValidationUtils::validate_email("@domain.com").is_err());
    }

    #[test]
    fn test_user_validation() {
        // Valid user
        let valid_user = json!({
            "userName": "test@example.com",
            "emails": [
                {"value": "test@example.com", "type": "work"}
            ]
        });
        assert!(ValidationUtils::validate_user_resource(&valid_user).is_ok());
        
        // Missing userName
        let invalid_user = json!({
            "emails": [
                {"value": "test@example.com", "type": "work"}
            ]
        });
        assert!(ValidationUtils::validate_user_resource(&invalid_user).is_err());
    }

    #[test]
    fn test_schema_validation() {
        let registry = SchemaRegistry::new().unwrap();
        
        let valid_data = json!({
            "userName": "test@example.com",
            "active": true
        });
        
        let result = validate_against_schema(
            &valid_data,
            "urn:ietf:params:scim:schemas:core:2.0:User",
            &registry
        );
        assert!(result.is_ok());
    }
}
```

## Error Handling

### Validation Error Types

The library uses `ScimError` for validation errors:

```rust
use scim_server::error::ScimError;

// Common validation error patterns
fn handle_validation_error(error: ScimError) {
    match error {
        ScimError::InvalidData(msg) => {
            eprintln!("Validation failed: {}", msg);
            // Return HTTP 400 Bad Request
        },
        ScimError::MissingRequiredAttribute { attribute } => {
            eprintln!("Missing required field: {}", attribute);
            // Return HTTP 400 Bad Request  
        },
        ScimError::InvalidAttributeType { attribute, expected, actual } => {
            eprintln!("Invalid type for {}: expected {}, got {}", attribute, expected, actual);
            // Return HTTP 400 Bad Request
        },
        _ => {
            eprintln!("Other error: {:?}", error);
        }
    }
}
```

## Best Practices

### Current Recommendations

1. **Implement validation early**: Add validation at the provider or handler level
2. **Use schema information**: Leverage existing schema definitions for validation rules
3. **Provide clear error messages**: Help clients understand what went wrong
4. **Test validation thoroughly**: Write comprehensive tests for all validation rules
5. **Be consistent**: Use the same validation patterns across your application

### Performance Considerations

1. **Validate once**: Don't duplicate validation across multiple layers
2. **Cache validation results**: For expensive validations, consider caching
3. **Validate incrementally**: For updates, only validate changed fields when possible
4. **Use efficient algorithms**: For complex validation, optimize for performance

### Security Considerations

1. **Sanitize input**: Validate and sanitize all input data
2. **Prevent injection**: Be careful with dynamic validation that could enable injection attacks
3. **Limit input size**: Validate that input sizes are within reasonable bounds
4. **Handle edge cases**: Test with malformed, oversized, and malicious input

## Future Improvements

### Potential Enhancements

The library may eventually include:

- Built-in validation framework with fluent API
- Automatic validation based on schema definitions
- Custom validation rule registration
- Cross-field validation support
- Integration with external validation libraries

### Contributing

If you need advanced validation features, consider:

1. **Contributing to the library**: Help build a comprehensive validation system
2. **Creating validation middleware**: Build reusable validation components
3. **Sharing validation patterns**: Document patterns that work well

## Next Steps

- [Schema Overview](./overview.md) - Understand the schema system that drives validation
- [Custom Resources](./custom-resources.md) - Learn to create resources with custom validation
- [Extensions](./extensions.md) - Add validation to schema extensions
- [Provider Implementation](../providers/basic.md) - Implement validation in custom providers

For now, validation requires manual implementation, but the patterns shown here provide a solid foundation for building robust validation into your SCIM server.