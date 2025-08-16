# Validation Configuration

SCIM Server 0.3.7 provides schema-based validation with limited configuration options. This guide covers the actual validation configuration capabilities available in the current implementation.

## Current Configuration Capabilities

### Schema Registry Configuration

The primary validation configuration in SCIM Server involves choosing how to load SCIM schemas:

```rust
use scim_server::SchemaRegistry;

// Option 1: Use embedded schemas (recommended)
let registry = SchemaRegistry::new()?;

// Option 2: Use embedded schemas explicitly
let registry = SchemaRegistry::with_embedded_schemas()?;

// Option 3: Load schemas from file directory
let registry = SchemaRegistry::from_schema_dir("./schemas")?;
```

### Embedded vs File-Based Schemas

**Embedded Schemas (Default)**
```rust
use scim_server::SchemaRegistry;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Uses built-in SCIM 2.0 core schemas
    let registry = SchemaRegistry::new()?;
    
    // Always includes:
    // - urn:ietf:params:scim:schemas:core:2.0:User
    // - urn:ietf:params:scim:schemas:core:2.0:Group
    
    println!("Available schemas: {}", registry.get_schemas().len());
    Ok(())
}
```

**File-Based Schemas**
```rust
use scim_server::SchemaRegistry;
use std::path::Path;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load schemas from directory containing User.json and Group.json
    let schema_dir = Path::new("./custom-schemas");
    let registry = SchemaRegistry::from_schema_dir(schema_dir)?;
    
    println!("Loaded schemas from directory: {:?}", schema_dir);
    Ok(())
}
```

## Operation Context Configuration

Validation behavior can be configured by specifying the operation context:

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
    
    // Configure validation for different operations
    let contexts = [
        OperationContext::Create,  // No 'id' field allowed
        OperationContext::Update,  // 'id' field required
        OperationContext::Patch,   // 'id' field required, partial updates
    ];
    
    for context in contexts {
        match registry.validate_json_resource_with_context("User", &user_data, context) {
            Ok(_) => println!("✅ Validation passed for {:?}", context),
            Err(e) => println!("❌ Validation failed for {:?}: {}", context, e),
        }
    }
    
    Ok(())
}
```

## Provider Integration Configuration

When using resource providers, validation is automatically configured:

```rust
use scim_server::{StandardResourceProvider, InMemoryStorage, RequestContext};
use serde_json::json;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Providers automatically use embedded schema validation
    let storage = InMemoryStorage::new();
    let provider = StandardResourceProvider::new(storage);
    let context = RequestContext::new("config-example".to_string());
    
    let user_data = json!({
        "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
        "userName": "alice@example.com"
    });
    
    // Validation happens automatically during resource operations
    let user = provider.create_resource("User", user_data, &context).await?;
    println!("User created with automatic validation: {}", 
             user.get_id().unwrap_or("unknown"));
    
    Ok(())
}
```

## Error Handling Configuration

Configure how validation errors are handled in your application:

```rust
use scim_server::{SchemaRegistry, ValidationError, schema::OperationContext};
use serde_json::json;

struct ValidationConfig {
    fail_fast: bool,
    detailed_errors: bool,
    log_validation_errors: bool,
}

impl ValidationConfig {
    fn new() -> Self {
        Self {
            fail_fast: true,
            detailed_errors: true,
            log_validation_errors: false,
        }
    }
}

fn validate_with_config(
    registry: &SchemaRegistry,
    resource_type: &str,
    data: &serde_json::Value,
    context: OperationContext,
    config: &ValidationConfig,
) -> Result<(), String> {
    match registry.validate_json_resource_with_context(resource_type, data, context) {
        Ok(_) => Ok(()),
        Err(validation_error) => {
            if config.log_validation_errors {
                eprintln!("Validation error logged: {}", validation_error);
            }
            
            let error_message = if config.detailed_errors {
                match validation_error {
                    ValidationError::MissingRequiredAttribute { attribute } => {
                        format!("Required field '{}' is missing", attribute)
                    },
                    ValidationError::InvalidAttributeType { attribute, expected, actual } => {
                        format!("Field '{}' has wrong type: expected {}, got {}", 
                               attribute, expected, actual)
                    },
                    ValidationError::MissingSchemas => {
                        "Missing 'schemas' field".to_string()
                    },
                    ValidationError::EmptySchemas => {
                        "Empty 'schemas' array".to_string()
                    },
                    ValidationError::UnknownSchemaUri { uri } => {
                        format!("Unknown schema URI: {}", uri)
                    },
                    ValidationError::Custom { message } => message,
                    _ => format!("Validation error: {}", validation_error)
                }
            } else {
                "Validation failed".to_string()
            };
            
            if config.fail_fast {
                return Err(error_message);
            }
            
            // Could collect errors instead of failing immediately
            eprintln!("Warning: {}", error_message);
            Ok(())
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let registry = SchemaRegistry::new()?;
    let config = ValidationConfig::new();
    
    let invalid_user = json!({
        "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"]
        // Missing required userName
    });
    
    match validate_with_config(
        &registry, 
        "User", 
        &invalid_user, 
        OperationContext::Create, 
        &config
    ) {
        Ok(_) => println!("Validation passed"),
        Err(e) => println!("Validation failed: {}", e),
    }
    
    Ok(())
}
```

## Schema Directory Structure

When using file-based schema loading, organize schemas in this structure:

```
schemas/
├── User.json          # Core User schema
├── Group.json         # Core Group schema
└── extensions/        # Optional: future schema extensions
    ├── Enterprise.json
    └── Custom.json
```

Example usage:
```rust
use scim_server::SchemaRegistry;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // This expects User.json and Group.json in the schemas directory
    let registry = SchemaRegistry::from_schema_dir("./schemas")?;
    
    println!("Loaded {} schemas from directory", registry.get_schemas().len());
    
    // Verify required schemas are present
    let user_schema = registry.get_user_schema();
    let group_schema = registry.get_group_schema();
    
    println!("User schema: {}", user_schema.name);
    println!("Group schema: {}", group_schema.name);
    
    Ok(())
}
```

## Configuration Best Practices

### 1. Use Embedded Schemas by Default

```rust
// ✅ Recommended: Works reliably without file dependencies
let registry = SchemaRegistry::new()?;

// ❌ Avoid unless you need custom schemas
let registry = SchemaRegistry::from_schema_dir("./schemas")?;
```

### 2. Handle Schema Loading Errors

```rust
use scim_server::SchemaRegistry;

fn create_registry() -> Result<SchemaRegistry, String> {
    SchemaRegistry::new()
        .map_err(|e| format!("Failed to create schema registry: {}", e))
}

// Or with fallback
fn create_registry_with_fallback() -> SchemaRegistry {
    SchemaRegistry::new()
        .or_else(|_| SchemaRegistry::with_embedded_schemas())
        .expect("Failed to create schema registry with any method")
}
```

### 3. Configure Error Handling Early

```rust
use scim_server::{SchemaRegistry, ValidationError};

struct AppConfig {
    registry: SchemaRegistry,
    validation_config: ValidationConfig,
}

impl AppConfig {
    fn new() -> Result<Self, Box<dyn std::error::Error>> {
        Ok(Self {
            registry: SchemaRegistry::new()?,
            validation_config: ValidationConfig::new(),
        })
    }
    
    fn validate_resource(
        &self,
        resource_type: &str,
        data: &serde_json::Value,
    ) -> Result<(), ValidationError> {
        self.registry.validate_json_resource_with_context(
            resource_type,
            data,
            scim_server::schema::OperationContext::Create,
        )
    }
}
```

## Limitations

SCIM Server 0.3.7 validation configuration is limited to:

- **Schema source selection** (embedded vs file-based)
- **Operation context specification** (Create/Update/Patch)
- **Error handling strategy** (application-level configuration)

**Not Available:**
- Custom validation rules
- Tenant-specific validation
- External validator integration
- Dynamic rule configuration
- Validation pipeline customization
- Field-level validation overrides

## Migration from Earlier Versions

If you have code expecting advanced validation configuration APIs that don't exist:

```rust
// ❌ This API does not exist in 0.3.7
// let config = ValidationConfig::builder()
//     .add_rule(ValidationRule::new())
//     .build();

// ✅ Use actual validation capabilities
let registry = SchemaRegistry::new()?;
registry.validate_json_resource_with_context(
    "User",
    &user_data,
    OperationContext::Create
)?;
```

## Future Configuration

Advanced validation configuration features like custom rules, external validators, and tenant-specific validation are planned for future releases. The current schema-based validation provides a solid foundation that will be extended with additional configuration capabilities.

## Next Steps

- [Basic Validation](./basic.md) - Working with validation in practice
- [Field-Level Validation](./field-level.md) - Understanding attribute-specific validation  
- [Advanced Validation](./advanced.md) - Current limitations and future plans