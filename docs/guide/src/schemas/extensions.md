# Extensions

Schema extensions in SCIM allow adding custom attributes to existing resource types like User and Group. This section covers the current capabilities and limitations of schema extensions in the SCIM Server library.

## Current Status

⚠️ **Limited Extension Support**: The SCIM Server library currently has basic schema support but lacks a comprehensive extension system. Extensions can be implemented through custom schemas and resource handlers, but there is no automated extension registration or validation system.

## What Works Today

### Custom Schemas for Existing Resources

You can create custom schemas that extend standard SCIM resources by following these patterns:

#### Example: Extended User with Company Attributes

```rust
use scim_server::schema::{Schema, AttributeDefinition, AttributeType, Mutability, Uniqueness};
use serde_json::json;

// Create a schema for company-specific user attributes
fn create_company_user_extension() -> Schema {
    Schema {
        id: "urn:company:params:scim:schemas:extension:employee:2.0:User".to_string(),
        name: "Employee Extension".to_string(),
        description: "Company-specific employee attributes".to_string(),
        attributes: vec![
            AttributeDefinition {
                name: "employeeNumber".to_string(),
                data_type: AttributeType::String,
                multi_valued: false,
                required: false,
                case_exact: true,
                mutability: Mutability::ReadWrite,
                uniqueness: Uniqueness::Server,
                canonical_values: vec![],
                sub_attributes: vec![],
                returned: Some("default".to_string()),
            },
            AttributeDefinition {
                name: "department".to_string(),
                data_type: AttributeType::String,
                multi_valued: false,
                required: false,
                case_exact: false,
                mutability: Mutability::ReadWrite,
                uniqueness: Uniqueness::None,
                canonical_values: vec![
                    "Engineering".to_string(),
                    "Sales".to_string(),
                    "Marketing".to_string(),
                    "HR".to_string(),
                ],
                sub_attributes: vec![],
                returned: Some("default".to_string()),
            },
            AttributeDefinition {
                name: "startDate".to_string(),
                data_type: AttributeType::DateTime,
                multi_valued: false,
                required: false,
                case_exact: false,
                mutability: Mutability::ReadWrite,
                uniqueness: Uniqueness::None,
                canonical_values: vec![],
                sub_attributes: vec![],
                returned: Some("default".to_string()),
            },
        ],
    }
}
```

#### Using Extended Attributes in Resources

```rust
use serde_json::json;

// User resource with extension attributes
let extended_user = json!({
    "schemas": [
        "urn:ietf:params:scim:schemas:core:2.0:User",
        "urn:company:params:scim:schemas:extension:employee:2.0:User"
    ],
    "userName": "john.doe@company.com",
    "name": {
        "givenName": "John",
        "familyName": "Doe"
    },
    "emails": [
        {
            "value": "john.doe@company.com",
            "type": "work",
            "primary": true
        }
    ],
    // Extension attributes (typically namespaced in real SCIM)
    "employeeNumber": "EMP12345",
    "department": "Engineering",
    "startDate": "2024-01-15T00:00:00Z"
});
```

#### Manual Schema Registration

```rust
use scim_server::schema::SchemaRegistry;

async fn register_extension_schema() -> Result<(), Box<dyn std::error::Error>> {
    let mut registry = SchemaRegistry::new()?;
    
    // Create and register the extension schema
    let extension_schema = create_company_user_extension();
    registry.add_schema(extension_schema)?;
    
    println!("Extension schema registered successfully");
    Ok(())
}
```

## Current Limitations

### No Automatic Extension System

- ❌ No `register_extension()` method
- ❌ No automatic validation of extension attributes
- ❌ No built-in support for SCIM extension namespacing
- ❌ No extension discovery mechanisms

### Manual Implementation Required

Extensions currently require manual implementation:

1. **Schema Creation**: Manually define extension schemas
2. **Resource Handlers**: Create custom handlers that understand extensions
3. **Validation**: Implement custom validation logic
4. **Serialization**: Handle extension attribute serialization/deserialization

### Example: Manual Extension Handling

```rust
use scim_server::{SchemaResourceBuilder, ScimServer, ScimOperation};
use serde_json::Value;

fn create_extended_user_handler(user_schema: Schema, extension_schema: Schema) -> scim_server::resource::ResourceHandler {
    SchemaResourceBuilder::new(user_schema)
        // Handle standard User attributes...
        
        // Handle extension attributes manually
        .with_getter("employeeNumber", |data| {
            data.get("employeeNumber")?.as_str().map(|s| Value::String(s.to_string()))
        })
        .with_setter("employeeNumber", |data, value| {
            if let Some(obj) = data.as_object_mut() {
                obj.insert("employeeNumber".to_string(), value);
            }
            Ok(())
        })
        .with_getter("department", |data| {
            data.get("department")?.as_str().map(|s| Value::String(s.to_string()))
        })
        .with_setter("department", |data, value| {
            if let Some(obj) = data.as_object_mut() {
                obj.insert("department".to_string(), value);
            }
            Ok(())
        })
        // Add more extension attribute handlers...
        .build()
}

async fn setup_extended_user_resource() -> Result<(), Box<dyn std::error::Error>> {
    let storage = scim_server::InMemoryStorage::new();
    let provider = scim_server::StandardResourceProvider::new(storage);
    let mut server = ScimServer::new(provider)?;
    
    // Get base user schema
    let registry = scim_server::SchemaRegistry::new()?;
    let user_schema = registry.get_user_schema().clone();
    
    // Create extension schema
    let extension_schema = create_company_user_extension();
    
    // Create handler that supports both schemas
    let extended_handler = create_extended_user_handler(user_schema, extension_schema);
    
    // Register with server
    server.register_resource_type(
        "User",
        extended_handler,
        vec![ScimOperation::Create, ScimOperation::Read, ScimOperation::Update],
    )?;
    
    Ok(())
}
```

## Working Patterns

### Pattern 1: Extended Custom Resources

Instead of extending standard resources, create entirely new custom resources:

```rust
// Instead of extending User, create Employee resource
let employee_schema = Schema {
    id: "urn:company:params:scim:schemas:core:2.0:Employee".to_string(),
    name: "Employee".to_string(),
    description: "Company employee resource".to_string(),
    attributes: vec![
        // Include user-like attributes plus extensions
        // This gives you full control over the schema
    ],
};
```

### Pattern 2: Composition Over Extension

Use composition to combine data from multiple sources:

```rust
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ExtendedUser {
    // Standard SCIM User fields
    pub id: String,
    pub user_name: String,
    pub name: Option<UserName>,
    pub emails: Vec<Email>,
    
    // Extension fields
    pub employee_number: Option<String>,
    pub department: Option<String>,
    pub start_date: Option<chrono::DateTime<chrono::Utc>>,
}
```

### Pattern 3: Configuration-Based Extensions

Define extensions through configuration:

```rust
use std::collections::HashMap;

pub struct ExtensionConfig {
    pub schema_id: String,
    pub attributes: HashMap<String, AttributeDefinition>,
}

pub fn load_extensions_from_config() -> Vec<ExtensionConfig> {
    // Load from configuration files or environment
    vec![]
}
```

## Future Considerations

### Planned Improvements

The library may eventually support:

- Automatic extension registration and validation
- Standard SCIM extension namespace handling
- Extension discovery endpoints
- Automatic schema merging for extended resources

### Contributing Extensions

If you need robust extension support, consider:

1. **Contributing to the library**: Help implement a comprehensive extension system
2. **Creating wrapper patterns**: Build abstraction layers that handle extensions
3. **Using custom resources**: Often simpler than trying to extend existing ones

## Best Practices

### Current Recommendations

1. **Use custom resources when possible**: Often cleaner than extensions
2. **Manually validate extension data**: Don't rely on automatic validation
3. **Document your extensions thoroughly**: Include schema definitions and examples
4. **Test extension compatibility**: Verify with SCIM clients that understand extensions
5. **Plan for schema evolution**: Design extensions that can grow over time

### Avoid Common Pitfalls

1. **Don't assume automatic validation**: Extension attributes need manual validation
2. **Handle missing extension schemas gracefully**: Not all clients will understand your extensions
3. **Use proper SCIM URN format**: Follow SCIM standards for schema identifiers
4. **Consider performance impact**: Extensions can complicate queries and operations

## Next Steps

- [Custom Resources](./custom-resources.md) - Often a better alternative to extensions
- [Validation](./validation.md) - Implement validation for extension attributes
- [Schema Overview](./overview.md) - Understand the core schema system

For robust extension support, consider contributing to the library or creating wrapper abstractions that provide the extension semantics you need.