# Validation Overview

This section covers the validation architecture and concepts in the SCIM Server library. While the library provides comprehensive built-in validation based on SCIM schemas, you can extend it with custom validation for business rules, compliance requirements, and organization-specific constraints.

## What is Custom Validation?

Custom validation in SCIM Server allows you to:

- **Enforce business rules** - Complex validation logic beyond schema constraints
- **Implement compliance requirements** - GDPR, HIPAA, or industry-specific rules
- **Add organization-specific constraints** - Custom attribute validation
- **Integrate with external systems** - Real-time validation against external APIs
- **Implement cross-field validation** - Dependencies between multiple attributes

## Validation Architecture

### Validation Pipeline

The SCIM Server validation pipeline processes requests in this order:

1. **Schema Validation** - Built-in SCIM schema compliance
2. **Type Validation** - Data type checking and format validation
3. **Custom Validation** - Your business logic
4. **Storage Validation** - Database constraints and uniqueness checks

```rust
use scim_server::validation::{
    ValidationPipeline, 
    ValidatorChain, 
    SchemaValidator,
    CustomValidator,
    ValidationResult
};

let validation_pipeline = ValidationPipeline::builder()
    .add_validator(SchemaValidator::new())
    .add_validator(TypeValidator::new())
    .add_validator(CustomBusinessRuleValidator::new())
    .add_validator(ComplianceValidator::new())
    .build();
```

### Core Components

The validation system consists of several key components:

#### ValidationContext
Provides context information during validation:

```rust
pub struct ValidationContext {
    pub tenant_id: String,
    pub operation: Operation,
    pub resource_type: ResourceType,
    pub authenticated_user: Option<String>,
    pub client_info: ClientInfo,
    pub timestamp: DateTime<Utc>,
}

pub enum Operation {
    Create,
    Update,
    Patch,
    Delete,
    BulkCreate,
    BulkUpdate,
}
```

#### ValidationError
Represents validation failures with detailed information:

```rust
pub struct ValidationError {
    pub code: String,
    pub message: String,
    pub field_path: Option<String>,
    pub severity: ValidationSeverity,
    pub details: Option<serde_json::Value>,
}

impl ValidationError {
    pub fn new(code: &str, message: &str) -> Self {
        Self {
            code: code.to_string(),
            message: message.to_string(),
            field_path: None,
            severity: ValidationSeverity::Error,
            details: None,
        }
    }

    pub fn with_field(mut self, field_path: &str) -> Self {
        self.field_path = Some(field_path.to_string());
        self
    }

    pub fn with_severity(mut self, severity: ValidationSeverity) -> Self {
        self.severity = severity;
        self
    }
}
```

### Custom Validator Trait

Implement the `CustomValidator` trait for your validation logic:

```rust
use scim_server::validation::{CustomValidator, ValidationContext, ValidationError};
use scim_server::models::{User, Group};
use async_trait::async_trait;

#[async_trait]
pub trait CustomValidator: Send + Sync {
    /// Validate a user during creation or update
    async fn validate_user(
        &self,
        user: &User,
        context: &ValidationContext,
    ) -> Result<(), ValidationError>;
    
    /// Validate a group during creation or update
    async fn validate_group(
        &self,
        group: &Group,
        context: &ValidationContext,
    ) -> Result<(), ValidationError>;
    
    /// Validate patch operations before applying
    async fn validate_patch_operations(
        &self,
        resource_type: &str,
        resource_id: &str,
        operations: &[PatchOperation],
        context: &ValidationContext,
    ) -> Result<(), ValidationError>;
    
    /// Custom validation for batch operations (individual operations in sequence)
    async fn validate_batch_operation(
        &self,
        resource_type: &str,
        operation_type: &str,
        data: &serde_json::Value,
        context: &ValidationContext,
    ) -> Result<(), ValidationError> {
        // Default implementation - override if needed
        // Validate each operation individually since bulk operations aren't implemented
        match operation_type {
            "CREATE" => self.validate_create(resource_type, data, context).await,
            "UPDATE" => self.validate_update(resource_type, data, context).await,
            "PATCH" => {
                // For patch operations, extract patch operations from data
                if let Ok(operations) = serde_json::from_value::<Vec<PatchOperation>>(data.clone()) {
                    self.validate_patch(resource_type, &operations, context).await
                } else {
                    Err(ValidationError::InvalidData("Invalid patch operations".to_string()))
                }
            },
            _ => Ok(())
        }
    }
}
```

## Validation Strategies

### 1. Synchronous vs Asynchronous

- **Synchronous validation** - Fast, local checks (regex, length, format)
- **Asynchronous validation** - External API calls, database lookups

### 2. Fail-Fast vs Collect-All

- **Fail-Fast** - Stop on first validation error
- **Collect-All** - Gather all validation errors before failing

### 3. Severity Levels

```rust
pub enum ValidationSeverity {
    Error,   // Blocks the operation
    Warning, // Logs but allows operation  
    Info,    // Informational only
}
```

## Integration Points

### Server Configuration

Register validators during server startup:

```rust
use scim_server::ScimServerBuilder;

let server = ScimServerBuilder::new()
    .with_provider(my_provider)
    .add_validator(BusinessRuleValidator::new())
    .add_validator(ComplianceValidator::new())
    .build();
```

### Tenant-Specific Validation

Different validation rules per tenant:

```rust
pub struct TenantValidatorRegistry {
    validators: HashMap<String, Vec<Box<dyn CustomValidator>>>,
}

impl TenantValidatorRegistry {
    pub async fn validate_for_tenant(
        &self,
        tenant_id: &str,
        user: &User,
        context: &ValidationContext,
    ) -> Result<(), ValidationError> {
        if let Some(validators) = self.validators.get(tenant_id) {
            for validator in validators {
                validator.validate_user(user, context).await?;
            }
        }
        Ok(())
    }
}
```

## Error Handling

### Validation Error Aggregation

```rust
pub struct ValidationResult {
    pub errors: Vec<ValidationError>,
    pub warnings: Vec<ValidationError>,
}

impl ValidationResult {
    pub fn is_valid(&self) -> bool {
        self.errors.is_empty()
    }

    pub fn add_error(&mut self, error: ValidationError) {
        match error.severity {
            ValidationSeverity::Error => self.errors.push(error),
            ValidationSeverity::Warning => self.warnings.push(error),
            ValidationSeverity::Info => { /* Log only */ }
        }
    }
}
```

### Client Error Response

Validation errors are returned as SCIM-compliant error responses:

```json
{
  "schemas": ["urn:ietf:params:scim:api:messages:2.0:Error"],
  "status": "400",
  "scimType": "invalidValue",
  "detail": "Validation failed",
  "errors": [
    {
      "code": "INVALID_EMAIL_DOMAIN",
      "message": "Email domain 'example.com' is not allowed",
      "field": "emails[0].value"
    }
  ]
}
```

## Next Steps

- [Basic Validation](./basic.md) - Simple business rule validators
- [Advanced Validation](./advanced.md) - External integrations and complex logic
- [Field-Level Validation](./field-level.md) - Custom attribute validators
- [Configuration](./configuration.md) - Configurable validation rules