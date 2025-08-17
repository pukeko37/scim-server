# Schema Overview

This guide provides an overview of SCIM schemas and how they work within the SCIM Server library. Understanding schemas is essential for working with SCIM resources and creating custom resource types.

## What are SCIM Schemas?

SCIM (System for Cross-domain Identity Management) schemas define the structure, attributes, and validation rules for resources like Users and Groups. They provide a standardized way to describe what data can be stored and how it should be formatted.

### Core Concepts

- **Schema URI**: Unique identifier for each schema (e.g., `urn:ietf:params:scim:schemas:core:2.0:User`)
- **Resource Types**: The types of objects that can be managed (User, Group, custom types)
- **Attributes**: The fields that make up a resource (name, email, etc.)
- **Schema Registry**: Central repository for managing available schemas

## Standard SCIM Schemas

### Core User Schema
```
urn:ietf:params:scim:schemas:core:2.0:User
```

Defines essential user attributes:
- `id` - Unique identifier
- `userName` - Primary identifier for authentication
- `name` - User's full name (complex attribute)
- `displayName` - Name for display purposes
- `emails` - Email addresses (multi-valued)
- `phoneNumbers` - Phone numbers (multi-valued)
- `active` - Whether the user account is active

### Core Group Schema
```
urn:ietf:params:scim:schemas:core:2.0:Group
```

Defines group attributes:
- `id` - Unique identifier
- `displayName` - Group's display name
- `members` - Group members (multi-valued complex)

## Schema Structure

### Schema Definition

```rust
use scim_server::schema::Schema;

// The main schema structure
pub struct Schema {
    pub id: String,                              // Schema URI
    pub name: String,                            // Human-readable name
    pub description: String,                     // Schema description
    pub attributes: Vec<AttributeDefinition>,    // Attribute definitions
}
```

### Attribute Definition

```rust
use scim_server::schema::{AttributeDefinition, AttributeType, Mutability, Uniqueness};

pub struct AttributeDefinition {
    pub name: String,                    // Attribute name
    pub data_type: AttributeType,        // Data type
    pub multi_valued: bool,              // Can have multiple values
    pub required: bool,                  // Must be present
    pub case_exact: bool,                // Case-sensitive comparison
    pub mutability: Mutability,          // When attribute can be modified
    pub uniqueness: Uniqueness,          // Uniqueness constraint
    pub canonical_values: Vec<String>,   // Predefined valid values
    pub sub_attributes: Vec<AttributeDefinition>, // For complex types
    pub returned: Option<String>,        // When attribute is returned
}
```

### Attribute Types

```rust
pub enum AttributeType {
    String,    // Text data
    Boolean,   // True/false values
    Decimal,   // Floating-point numbers
    Integer,   // Whole numbers
    DateTime,  // ISO 8601 date-time
    Binary,    // Base64-encoded binary data
    Reference, // Reference to another resource
    Complex,   // Nested object with sub-attributes
}
```

### Mutability Levels

```rust
pub enum Mutability {
    ReadOnly,   // Cannot be modified by client
    ReadWrite,  // Can be read and written
    Immutable,  // Can only be set during creation
    WriteOnly,  // Can be written but not read (e.g., passwords)
}
```

### Uniqueness Constraints

```rust
pub enum Uniqueness {
    None,   // No uniqueness constraint
    Server, // Unique within the server
    Global, // Globally unique
}
```

## Schema Registry

The schema registry manages all available schemas and provides access to schema information:

```rust
use scim_server::schema::SchemaRegistry;

// Create a registry with embedded core schemas
let registry = SchemaRegistry::new()?;

// Alternative: explicitly use embedded schemas
let registry = SchemaRegistry::with_embedded_schemas()?;

// Load schemas from a directory (if you have custom schema files)
let registry = SchemaRegistry::from_schema_dir("path/to/schemas")?;
```

### Working with Schemas

#### Retrieving Schema Information

```rust
use scim_server::schema::SchemaRegistry;

let registry = SchemaRegistry::new()?;

// Get schema by URI
let user_schema = registry.get_schema("urn:ietf:params:scim:schemas:core:2.0:User");
if let Some(schema) = user_schema {
    println!("Schema: {} with {} attributes", schema.name, schema.attributes.len());
}

// Convenience methods for core schemas
let user_schema = registry.get_user_schema();
let group_schema = registry.get_group_schema();

// Get all available schemas
let all_schemas = registry.get_schemas();
for schema in all_schemas {
    println!("Available schema: {}", schema.id);
}
```

#### Adding Custom Schemas

```rust
use scim_server::schema::{Schema, AttributeDefinition, AttributeType, Mutability, Uniqueness};

// Create a custom schema
let custom_schema = Schema {
    id: "urn:company:params:scim:schemas:core:2.0:Device".to_string(),
    name: "Device".to_string(),
    description: "Device resource schema".to_string(),
    attributes: vec![
        AttributeDefinition {
            name: "serialNumber".to_string(),
            data_type: AttributeType::String,
            multi_valued: false,
            required: true,
            case_exact: true,
            mutability: Mutability::ReadWrite,
            uniqueness: Uniqueness::Server,
            canonical_values: vec![],
            sub_attributes: vec![],
            returned: Some("always".to_string()),
        },
        AttributeDefinition {
            name: "deviceType".to_string(),
            data_type: AttributeType::String,
            multi_valued: false,
            required: true,
            case_exact: false,
            mutability: Mutability::ReadWrite,
            uniqueness: Uniqueness::None,
            canonical_values: vec![
                "Laptop".to_string(),
                "Desktop".to_string(),
                "Tablet".to_string(),
                "Phone".to_string(),
            ],
            sub_attributes: vec![],
            returned: Some("always".to_string()),
        },
    ],
};

// Add to registry
let mut registry = SchemaRegistry::new()?;
registry.add_schema(custom_schema)?;
```

## Schema Attributes in Detail

### Simple Attributes

Most attributes are simple, single-valued fields:

```rust
AttributeDefinition {
    name: "userName".to_string(),
    data_type: AttributeType::String,
    multi_valued: false,        // Single value
    required: true,             // Must be present
    case_exact: false,          // Case-insensitive
    mutability: Mutability::ReadWrite,
    uniqueness: Uniqueness::Server,  // Must be unique
    canonical_values: vec![],   // No predefined values
    sub_attributes: vec![],     // No sub-attributes
    returned: Some("always".to_string()),
}
```

### Multi-valued Attributes

Some attributes can have multiple values:

```rust
// emails attribute - array of email objects
AttributeDefinition {
    name: "emails".to_string(),
    data_type: AttributeType::Complex,
    multi_valued: true,         // Can have multiple emails
    required: false,
    case_exact: false,
    mutability: Mutability::ReadWrite,
    uniqueness: Uniqueness::None,
    canonical_values: vec![],
    sub_attributes: vec![
        AttributeDefinition {
            name: "value".to_string(),
            data_type: AttributeType::String,
            // ... email value definition
        },
        AttributeDefinition {
            name: "type".to_string(),
            data_type: AttributeType::String,
            canonical_values: vec!["work".to_string(), "home".to_string()],
            // ... email type definition
        },
    ],
    returned: Some("default".to_string()),
}
```

### Complex Attributes

Complex attributes contain sub-attributes:

```rust
// name attribute - structured name object
AttributeDefinition {
    name: "name".to_string(),
    data_type: AttributeType::Complex,
    multi_valued: false,
    required: false,
    case_exact: false,
    mutability: Mutability::ReadWrite,
    uniqueness: Uniqueness::None,
    canonical_values: vec![],
    sub_attributes: vec![
        AttributeDefinition {
            name: "givenName".to_string(),
            data_type: AttributeType::String,
            // ... given name definition
        },
        AttributeDefinition {
            name: "familyName".to_string(),
            data_type: AttributeType::String,
            // ... family name definition
        },
        AttributeDefinition {
            name: "formatted".to_string(),
            data_type: AttributeType::String,
            // ... formatted name definition
        },
    ],
    returned: Some("default".to_string()),
}
```

## Schema Usage Examples

### Inspecting Schema Structure

```rust
use scim_server::schema::SchemaRegistry;

let registry = SchemaRegistry::new()?;
let user_schema = registry.get_user_schema();

println!("Schema: {}", user_schema.name);
println!("Description: {}", user_schema.description);
println!("Attributes:");

for attr in &user_schema.attributes {
    println!("  - {} ({:?})", attr.name, attr.data_type);
    if attr.required {
        println!("    Required: Yes");
    }
    if attr.multi_valued {
        println!("    Multi-valued: Yes");
    }
    if !attr.canonical_values.is_empty() {
        println!("    Allowed values: {:?}", attr.canonical_values);
    }
}
```

### Creating Resource Data

```rust
use serde_json::json;

// Create a user resource following the User schema
let user_data = json!({
    "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
    "userName": "john.doe@example.com",
    "name": {
        "givenName": "John",
        "familyName": "Doe",
        "formatted": "John Doe"
    },
    "emails": [
        {
            "value": "john.doe@example.com",
            "type": "work",
            "primary": true
        }
    ],
    "active": true
});
```

## Best Practices

### Schema Design Guidelines

1. **Use meaningful names**: Attribute names should be descriptive and follow camelCase convention
2. **Choose appropriate types**: Select the most specific type that fits your data
3. **Set proper constraints**: Use `required`, `uniqueness`, and `mutability` appropriately
4. **Document thoroughly**: Provide clear descriptions for schemas and attributes
5. **Follow SCIM conventions**: Use standard SCIM patterns for consistency

### Performance Considerations

1. **Index unique attributes**: Ensure database indexes exist for attributes with uniqueness constraints
2. **Minimize complex attributes**: Deeply nested structures can impact performance
3. **Consider multi-valued implications**: Multi-valued attributes require special handling in queries

### Compatibility

1. **Use standard URIs**: Follow SCIM URI conventions for schema identifiers
2. **Maintain backward compatibility**: Changes to existing schemas should be additive
3. **Test with SCIM clients**: Ensure custom schemas work with existing SCIM implementations

## Error Handling

When working with schemas, you may encounter various errors:

```rust
use scim_server::schema::SchemaRegistry;

let mut registry = SchemaRegistry::new()?;

// Handle potential errors when adding schemas
match registry.add_schema(custom_schema) {
    Ok(()) => println!("Schema added successfully"),
    Err(e) => eprintln!("Failed to add schema: {}", e),
}

// Handle missing schemas
match registry.get_schema("unknown:schema:uri") {
    Some(schema) => println!("Found schema: {}", schema.name),
    None => println!("Schema not found"),
}
```

## Next Steps

- [Custom Resources](./custom-resources.md) - Learn to create entirely new resource types
- [Extensions](./extensions.md) - Add custom attributes to existing resources  
- [Validation](./validation.md) - Implement custom validation rules for schemas