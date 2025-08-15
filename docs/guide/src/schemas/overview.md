# Schema Overview

This guide provides an overview of SCIM schemas and how they work within the SCIM Server library. Understanding schemas is essential for extending the server with custom attributes and resource types.

## What are SCIM Schemas?

SCIM (System for Cross-domain Identity Management) schemas define the structure, attributes, and validation rules for resources like Users and Groups. They provide a standardized way to describe what data can be stored and how it should be formatted.

### Core Concepts

- **Schema URI**: Unique identifier for each schema (e.g., `urn:ietf:params:scim:schemas:core:2.0:User`)
- **Resource Types**: The types of objects that can be managed (User, Group, custom types)
- **Attributes**: The fields that make up a resource (name, email, etc.)
- **Extensions**: Additional schemas that add custom attributes to existing resource types

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

### Enterprise User Extension
```
urn:ietf:params:scim:schemas:extension:enterprise:2.0:User
```

Adds enterprise-specific attributes:
- `employeeNumber` - Employee identifier
- `department` - Department name
- `manager` - Reference to manager
- `organization` - Organization name

## Schema Architecture

### Schema Definition Structure

```rust
use scim_server::schema::{SchemaDefinition, AttributeDefinition};
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchemaDefinition {
    pub id: String,                              // Schema URI
    pub name: String,                            // Human-readable name
    pub description: String,                     // Schema description
    pub attributes: Vec<AttributeDefinition>,    // Attribute definitions
    pub meta: SchemaMeta,                        // Metadata
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchemaMeta {
    pub resource_type: String,
    pub location: String,
    pub created: DateTime<Utc>,
    pub last_modified: DateTime<Utc>,
    pub version: String,
}
```

### Attribute Definition

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttributeDefinition {
    pub name: String,                    // Attribute name
    pub attribute_type: AttributeType,   // Data type
    pub multi_valued: bool,              // Can have multiple values
    pub description: String,             // Human-readable description
    pub required: bool,                  // Must be present
    pub case_exact: bool,                // Case-sensitive comparison
    pub mutability: Mutability,          // When attribute can be modified
    pub returned: Returned,              // When attribute is returned
    pub uniqueness: Uniqueness,          // Uniqueness constraint
    pub reference_types: Option<Vec<String>>, // For reference attributes
    pub canonical_values: Option<Vec<String>>, // Predefined valid values
    pub sub_attributes: Option<Vec<AttributeDefinition>>, // For complex types
}
```

### Attribute Types

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
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
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Mutability {
    ReadOnly,   // Cannot be modified by client
    ReadWrite,  // Can be read and written
    Immutable,  // Can only be set during creation
    WriteOnly,  // Can be written but not read (e.g., passwords)
}
```

### Return Behavior

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Returned {
    Always,  // Always returned
    Never,   // Never returned (e.g., passwords)
    Default, // Returned by default
    Request, // Only returned when explicitly requested
}
```

### Uniqueness Constraints

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Uniqueness {
    None,   // No uniqueness constraint
    Server, // Unique within the server
    Global, // Globally unique
}
```

## Schema Registry

The schema registry manages all available schemas and provides validation services:

```rust
use scim_server::schema::SchemaRegistry;
use std::collections::HashMap;

pub struct SchemaRegistry {
    schemas: HashMap<String, SchemaDefinition>,
    resource_schemas: HashMap<String, Vec<String>>, // resource_type -> schema_uris
}

impl SchemaRegistry {
    pub fn new() -> Self {
        let mut registry = Self {
            schemas: HashMap::new(),
            resource_schemas: HashMap::new(),
        };
        
        // Register core schemas
        registry.register_core_schemas();
        registry
    }
    
    pub fn register_schema(&mut self, schema: SchemaDefinition) -> Result<(), SchemaError> {
        // Validate schema definition
        self.validate_schema(&schema)?;
        
        // Store schema
        self.schemas.insert(schema.id.clone(), schema);
        Ok(())
    }
    
    pub fn get_schema(&self, schema_uri: &str) -> Option<&SchemaDefinition> {
        self.schemas.get(schema_uri)
    }
    
    pub fn get_resource_schemas(&self, resource_type: &str) -> Vec<&SchemaDefinition> {
        if let Some(schema_uris) = self.resource_schemas.get(resource_type) {
            schema_uris.iter()
                .filter_map(|uri| self.schemas.get(uri))
                .collect()
        } else {
            Vec::new()
        }
    }
    
    pub fn validate_resource(
        &self,
        resource: &serde_json::Value,
        schemas: &[String],
    ) -> Result<(), ValidationError> {
        for schema_uri in schemas {
            if let Some(schema) = self.get_schema(schema_uri) {
                self.validate_against_schema(resource, schema)?;
            } else {
                return Err(ValidationError::UnknownSchema(schema_uri.clone()));
            }
        }
        Ok(())
    }
    
    fn register_core_schemas(&mut self) {
        // Register User schema
        let user_schema = self.create_user_schema();
        self.schemas.insert(user_schema.id.clone(), user_schema);
        
        // Register Group schema
        let group_schema = self.create_group_schema();
        self.schemas.insert(group_schema.id.clone(), group_schema);
        
        // Register Enterprise User extension
        let enterprise_schema = self.create_enterprise_user_schema();
        self.schemas.insert(enterprise_schema.id.clone(), enterprise_schema);
        
        // Map resource types to schemas
        self.resource_schemas.insert(
            "User".to_string(),
            vec![
                "urn:ietf:params:scim:schemas:core:2.0:User".to_string(),
                "urn:ietf:params:scim:schemas:extension:enterprise:2.0:User".to_string(),
            ]
        );
        
        self.resource_schemas.insert(
            "Group".to_string(),
            vec!["urn:ietf:params:scim:schemas:core:2.0:Group".to_string()]
        );
    }
}
```

## Working with Schemas

### Retrieving Schema Information

```rust
use scim_server::ScimServer;

async fn get_user_schema(server: &ScimServer) -> Result<SchemaDefinition, ScimError> {
    let registry = server.schema_registry();
    let schema = registry.get_schema("urn:ietf:params:scim:schemas:core:2.0:User")
        .ok_or(ScimError::SchemaNotFound)?;
    Ok(schema.clone())
}

async fn list_all_schemas(server: &ScimServer) -> Vec<SchemaDefinition> {
    let registry = server.schema_registry();
    registry.list_schemas()
}
```

### Validating Resources

```rust
use scim_server::models::User;
use serde_json;

async fn validate_user(
    registry: &SchemaRegistry,
    user: &User,
) -> Result<(), ValidationError> {
    let user_json = serde_json::to_value(user)?;
    
    registry.validate_resource(
        &user_json,
        &user.schemas,
    )
}
```

## Schema Versioning

### Version Management

Schemas should be versioned to handle evolution over time:

```rust
pub struct SchemaVersion {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
}

impl SchemaVersion {
    pub fn new(major: u32, minor: u32, patch: u32) -> Self {
        Self { major, minor, patch }
    }
    
    pub fn is_compatible_with(&self, other: &SchemaVersion) -> bool {
        // Same major version is compatible
        self.major == other.major
    }
}

// Schema URI with version
// urn:company:schemas:extension:employee:1.0:User
```

### Migration Support

```rust
pub trait SchemaMigration {
    fn migrate(&self, from: &SchemaVersion, to: &SchemaVersion, data: &mut serde_json::Value) -> Result<(), MigrationError>;
    fn supports_migration(&self, from: &SchemaVersion, to: &SchemaVersion) -> bool;
}

pub struct SchemaEvolutionManager {
    migrations: Vec<Box<dyn SchemaMigration>>,
}

impl SchemaEvolutionManager {
    pub fn migrate_data(
        &self,
        data: &mut serde_json::Value,
        from_version: &SchemaVersion,
        to_version: &SchemaVersion,
    ) -> Result<(), MigrationError> {
        for migration in &self.migrations {
            if migration.supports_migration(from_version, to_version) {
                migration.migrate(from_version, to_version, data)?;
                break;
            }
        }
        Ok(())
    }
}
```

## Error Handling

### Schema-Related Errors

```rust
#[derive(Debug, thiserror::Error)]
pub enum SchemaError {
    #[error("Schema validation failed: {0}")]
    ValidationFailed(String),
    
    #[error("Unknown schema: {0}")]
    UnknownSchema(String),
    
    #[error("Schema conflict: {0}")]
    SchemaConflict(String),
    
    #[error("Invalid attribute definition: {0}")]
    InvalidAttribute(String),
    
    #[error("Schema version incompatible: {0}")]
    VersionIncompatible(String),
}
```

## Best Practices

### Schema Design Guidelines

1. **Use meaningful names**: Attribute names should be descriptive and follow camelCase convention
2. **Choose appropriate types**: Select the most specific type that fits your data
3. **Set proper constraints**: Use `required`, `uniqueness`, and `mutability` appropriately
4. **Document thoroughly**: Provide clear descriptions for schemas and attributes
5. **Version strategically**: Plan for schema evolution from the beginning

### Performance Considerations

1. **Index unique attributes**: Ensure database indexes exist for attributes with uniqueness constraints
2. **Minimize complex attributes**: Deeply nested structures can impact performance
3. **Cache schema definitions**: Avoid repeated schema lookups during validation
4. **Batch validation**: Validate multiple resources together when possible

## Next Steps

- [Custom Resources](./custom-resources.md) - Learn to create entirely new resource types
- [Extensions](./extensions.md) - Add custom attributes to existing resources
- [Validation](./validation.md) - Implement custom validation rules for schemas