# Advanced Validation

SCIM Server 0.3.7 provides comprehensive schema validation but does not currently support the advanced validation features described in earlier versions of this documentation.

## Current Validation Capabilities

SCIM Server currently provides:

- **Schema-based validation** - Automatic validation against SCIM 2.0 schemas
- **Built-in error types** - Comprehensive ValidationError enum for common failures
- **Operation context awareness** - Different validation for Create/Update/Patch operations
- **Integration with providers** - Automatic validation during resource operations

See [Basic Validation](./basic.md) for working examples of current validation capabilities.

## Advanced Features Not Currently Available

The following validation features are **not implemented** in SCIM Server 0.3.7:

### Custom Validation Pipelines
```rust
// ❌ This API does not exist
let pipeline = ValidationPipeline::builder()
    .add_validator(CustomValidator::new())
    .build();
```

### External System Integration
```rust
// ❌ This API does not exist
struct DatabaseValidator {
    db_pool: DbPool,
}

impl CustomValidator for DatabaseValidator {
    async fn validate_user(&self, user: &User) -> Result<(), ValidationError> {
        // Custom database validation logic
    }
}
```

### Tenant-Specific Validation
```rust
// ❌ This API does not exist
let tenant_validator = TenantValidatorRegistry::new()
    .with_tenant("tenant-1", CustomBusinessRules::new())
    .with_tenant("tenant-2", StrictComplianceRules::new());
```

### Async Validation Hooks
```rust
// ❌ This API does not exist
async fn validate_with_external_api(user: &User) -> Result<(), ValidationError> {
    let response = external_api_client.validate_user(user).await?;
    // Process external validation response
}
```

### Complex Business Rules
```rust
// ❌ This API does not exist
struct BusinessRuleValidator {
    rules: Vec<Box<dyn ValidationRule>>,
}

impl BusinessRuleValidator {
    fn add_rule<R: ValidationRule + 'static>(mut self, rule: R) -> Self {
        self.rules.push(Box::new(rule));
        self
    }
}
```

## What You Can Do Instead

While advanced validation features are not available, you can implement validation logic in your application layer:

### Application-Level Validation

```rust
use scim_server::{SchemaRegistry, ValidationError, schema::OperationContext};
use serde_json::Value;

// Custom validation function in your application
async fn validate_business_rules(
    resource_type: &str,
    data: &Value,
    operation: OperationContext
) -> Result<(), String> {
    let registry = SchemaRegistry::new()
        .map_err(|e| format!("Schema registry error: {}", e))?;
    
    // First, perform standard SCIM validation
    registry.validate_json_resource_with_context(resource_type, data, operation)
        .map_err(|e| format!("SCIM validation failed: {}", e))?;
    
    // Then, add your custom business rules
    validate_custom_rules(data).await?;
    
    Ok(())
}

async fn validate_custom_rules(data: &Value) -> Result<(), String> {
    // Example: Check email domain
    if let Some(emails) = data.get("emails").and_then(|e| e.as_array()) {
        for email in emails {
            if let Some(email_value) = email.get("value").and_then(|v| v.as_str()) {
                if let Some(domain) = email_value.split('@').nth(1) {
                    if !is_allowed_domain(domain) {
                        return Err(format!("Email domain '{}' is not allowed", domain));
                    }
                }
            }
        }
    }
    
    // Example: Check username format
    if let Some(username) = data.get("userName").and_then(|u| u.as_str()) {
        if username.len() < 3 {
            return Err("Username must be at least 3 characters".to_string());
        }
        
        if username.contains(' ') {
            return Err("Username cannot contain spaces".to_string());
        }
    }
    
    Ok(())
}

fn is_allowed_domain(domain: &str) -> bool {
    // Your domain validation logic
    let allowed_domains = ["company.com", "subsidiary.com"];
    allowed_domains.contains(&domain.to_lowercase().as_str())
}
```

### Pre-Processing Pattern

```rust
use scim_server::{StandardResourceProvider, InMemoryStorage, RequestContext, ResourceProvider};
use serde_json::Value;

async fn create_user_with_validation(
    user_data: Value,
    provider: &StandardResourceProvider<InMemoryStorage>,
    context: &RequestContext
) -> Result<scim_server::Resource, Box<dyn std::error::Error>> {
    
    // Step 1: Your custom validation
    validate_business_rules("User", &user_data, scim_server::schema::OperationContext::Create).await?;
    
    // Step 2: Use provider (which will also do SCIM validation)
    let user = provider.create_resource("User", user_data, context).await?;
    
    Ok(user)
}
```

### Wrapper Pattern

```rust
use scim_server::{StandardResourceProvider, InMemoryStorage, RequestContext, ResourceProvider};
use serde_json::Value;

pub struct ValidatingResourceProvider {
    inner: StandardResourceProvider<InMemoryStorage>,
}

impl ValidatingResourceProvider {
    pub fn new(storage: InMemoryStorage) -> Self {
        Self {
            inner: StandardResourceProvider::new(storage),
        }
    }
    
    pub async fn create_user_validated(
        &self,
        user_data: Value,
        context: &RequestContext
    ) -> Result<scim_server::Resource, Box<dyn std::error::Error>> {
        
        // Custom validation before creation
        self.validate_user_business_rules(&user_data).await?;
        
        // Create using inner provider
        let user = self.inner.create_resource("User", user_data, context).await?;
        
        Ok(user)
    }
    
    async fn validate_user_business_rules(&self, data: &Value) -> Result<(), String> {
        // Your custom validation logic here
        Ok(())
    }
}
```

## Future Plans

Advanced validation features including custom validation pipelines, external system integration, and tenant-specific validation rules are planned for future releases of SCIM Server.

These features will include:

- **Custom Validator Trait** - Define your own validation logic
- **Validation Pipeline** - Chain multiple validators together
- **Async Validation Support** - Integrate with external APIs and databases
- **Tenant-Specific Rules** - Different validation per tenant
- **Configurable Validation** - Runtime configuration of validation rules
- **Validation Hooks** - Pre and post validation callbacks

## Migration Path

When advanced validation features become available, you can migrate your application-level validation to the built-in validation system. The patterns shown above will help you organize your validation logic in a way that can be easily migrated.

## Next Steps

- [Field-Level Validation](./field-level.md) - Understand how SCIM validates specific attributes
- [Configuration](./configuration.md) - Learn about current validation configuration options
- [Basic Validation](./basic.md) - See working examples of current validation capabilities

For now, implement custom validation logic in your application layer using the patterns shown above, and watch for future releases that will include advanced validation features.