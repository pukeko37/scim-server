# Validation

> **TODO**: This section is under development. Basic schema validation patterns are outlined below.

## Overview

Schema validation ensures that SCIM resources conform to their defined schemas before being stored or processed. This includes validating data types, required fields, constraints, and custom business rules.

## Built-in Validation

### Core Schema Validation

```rust
use scim_server::{Schema, ValidationError, ValidationResult};
use serde_json::Value;

pub struct SchemaValidator {
    schemas: HashMap<String, Schema>,
}

impl SchemaValidator {
    pub fn validate_resource(
        &self,
        resource: &Value,
        resource_type: &str,
    ) -> ValidationResult {
        let schemas = self.extract_schemas(resource)?;
        
        for schema_id in schemas {
            let schema = self.schemas.get(&schema_id)
                .ok_or_else(|| ValidationError::UnknownSchema(schema_id.clone()))?;
            
            self.validate_against_schema(resource, schema)?;
        }
        
        Ok(())
    }
    
    fn validate_against_schema(
        &self,
        resource: &Value,
        schema: &Schema,
    ) -> ValidationResult {
        for attribute in schema.attributes() {
            self.validate_attribute(resource, attribute)?;
        }
        Ok(())
    }
}
```

### Attribute Validation

```rust
use scim_server::schema::{Attribute, AttributeType, Mutability};

impl SchemaValidator {
    fn validate_attribute(
        &self,
        resource: &Value,
        attribute: &Attribute,
    ) -> ValidationResult {
        let value = resource.get(attribute.name());
        
        // Check required attributes
        if attribute.required() && value.is_none() {
            return Err(ValidationError::MissingRequired(attribute.name().to_string()));
        }
        
        if let Some(value) = value {
            // Type validation
            self.validate_type(value, attribute.type_())?;
            
            // Constraints validation
            if let Some(canonical_values) = attribute.canonical_values() {
                self.validate_canonical_values(value, canonical_values)?;
            }
            
            // Custom validation rules
            self.validate_custom_rules(value, attribute)?;
        }
        
        Ok(())
    }
    
    fn validate_type(&self, value: &Value, expected_type: AttributeType) -> ValidationResult {
        match expected_type {
            AttributeType::String => {
                if !value.is_string() {
                    return Err(ValidationError::TypeMismatch {
                        expected: "string".to_string(),
                        actual: value.clone(),
                    });
                }
            },
            AttributeType::Boolean => {
                if !value.is_boolean() {
                    return Err(ValidationError::TypeMismatch {
                        expected: "boolean".to_string(),
                        actual: value.clone(),
                    });
                }
            },
            AttributeType::Integer => {
                if !value.is_i64() {
                    return Err(ValidationError::TypeMismatch {
                        expected: "integer".to_string(),
                        actual: value.clone(),
                    });
                }
            },
            // TODO: Add other type validations
        }
        Ok(())
    }
}
```

## Custom Validation Rules

### Business Logic Validation

> **TODO**: Implement custom business rule validation patterns.

### Cross-Field Validation

> **TODO**: Add examples for validating relationships between fields.

## Validation Errors

```rust
#[derive(Debug)]
pub enum ValidationError {
    MissingRequired(String),
    TypeMismatch {
        expected: String,
        actual: Value,
    },
    InvalidValue {
        field: String,
        value: Value,
        reason: String,
    },
    UnknownSchema(String),
    CustomRule {
        rule: String,
        message: String,
    },
}

impl std::fmt::Display for ValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ValidationError::MissingRequired(field) => {
                write!(f, "Missing required field: {}", field)
            },
            ValidationError::TypeMismatch { expected, actual } => {
                write!(f, "Type mismatch: expected {}, got {:?}", expected, actual)
            },
            ValidationError::InvalidValue { field, value, reason } => {
                write!(f, "Invalid value for {}: {:?} ({})", field, value, reason)
            },
            ValidationError::UnknownSchema(schema) => {
                write!(f, "Unknown schema: {}", schema)
            },
            ValidationError::CustomRule { rule, message } => {
                write!(f, "Custom rule '{}' failed: {}", rule, message)
            },
        }
    }
}
```

## Integration with Providers

```rust
impl ResourceProvider for ValidatedProvider {
    async fn create_resource(
        &self,
        resource_type: &str,
        data: Value,
        context: &RequestContext,
    ) -> Result<ScimResource, ProviderError> {
        // Validate before creating
        self.validator.validate_resource(&data, resource_type)
            .map_err(|e| ProviderError::ValidationFailed(e))?;
        
        // Proceed with creation
        self.inner.create_resource(resource_type, data, context).await
    }
}
```

## Testing Validation

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_required_field_validation() {
        // TODO: Test required field validation
    }
    
    #[test]
    fn test_type_validation() {
        // TODO: Test type validation
    }
    
    #[test]
    fn test_custom_rules() {
        // TODO: Test custom validation rules
    }
}
```

> **TODO**: Add more comprehensive validation patterns and examples.
