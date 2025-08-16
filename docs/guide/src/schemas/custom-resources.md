# Custom Resources

This guide covers creating entirely new resource types in the SCIM Server library. While User and Group are the standard SCIM resources, you can define custom resource types to model organization-specific entities like devices, applications, roles, or any other business objects.

## Overview

Custom resources allow you to:

- **Model business entities** - Devices, applications, roles, projects, locations
- **Extend beyond identity** - Any organizational resource that needs management
- **Maintain SCIM compliance** - Follow SCIM patterns and conventions
- **Integrate with existing flows** - Use standard SCIM operations (Create, Read, Update, Delete, List)
- **Support multi-tenancy** - Different resource types per tenant

## Complete Example: Device Resource

Let's walk through creating a complete Device resource type from schema definition to server registration.

### Step 1: Define the Resource Structure

First, define your custom resource as a Rust struct:

```rust
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use scim_server::Meta;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Device {
    pub id: String,
    pub schemas: Vec<String>,
    pub meta: Meta,
    pub external_id: Option<String>,

    // Custom attributes
    pub serial_number: String,
    pub device_type: DeviceType,
    pub manufacturer: String,
    pub model: String,
    pub assigned_to: Option<String>,  // User ID
    pub status: DeviceStatus,
    pub purchase_date: Option<DateTime<Utc>>,
    pub warranty_expires: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DeviceType {
    Laptop,
    Desktop,
    Tablet,
    Phone,
    Server,
    Printer,
    Other(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DeviceStatus {
    Available,
    Assigned,
    InRepair,
    Retired,
}

impl Device {
    pub fn new(serial_number: String, device_type: DeviceType, manufacturer: String, model: String) -> Self {
        let now = chrono::Utc::now();
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            schemas: vec!["urn:company:params:scim:schemas:core:2.0:Device".to_string()],
            meta: Meta {
                resource_type: "Device".to_string(),
                created: now,
                last_modified: now,
                version: Some("1".to_string()),
                location: None,
            },
            external_id: None,
            serial_number,
            device_type,
            manufacturer,
            model,
            assigned_to: None,
            status: DeviceStatus::Available,
            purchase_date: None,
            warranty_expires: None,
        }
    }
}
```

### Step 2: Create the Schema Definition

Define the SCIM schema for your custom resource:

```rust
use scim_server::schema::{Schema, AttributeDefinition, AttributeType, Mutability, Uniqueness};

fn create_device_schema() -> Schema {
    Schema {
        id: "urn:company:params:scim:schemas:core:2.0:Device".to_string(),
        name: "Device".to_string(),
        description: "Device resource schema for IT asset management".to_string(),
        attributes: vec![
            // Serial number - required, unique identifier
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
            // Device type - required, with canonical values
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
                    "Server".to_string(),
                    "Printer".to_string(),
                    "Other".to_string(),
                ],
                sub_attributes: vec![],
                returned: Some("always".to_string()),
            },
            // Manufacturer
            AttributeDefinition {
                name: "manufacturer".to_string(),
                data_type: AttributeType::String,
                multi_valued: false,
                required: true,
                case_exact: false,
                mutability: Mutability::ReadWrite,
                uniqueness: Uniqueness::None,
                canonical_values: vec![],
                sub_attributes: vec![],
                returned: Some("always".to_string()),
            },
            // Model
            AttributeDefinition {
                name: "model".to_string(),
                data_type: AttributeType::String,
                multi_valued: false,
                required: true,
                case_exact: false,
                mutability: Mutability::ReadWrite,
                uniqueness: Uniqueness::None,
                canonical_values: vec![],
                sub_attributes: vec![],
                returned: Some("always".to_string()),
            },
            // Status - optional, with canonical values
            AttributeDefinition {
                name: "status".to_string(),
                data_type: AttributeType::String,
                multi_valued: false,
                required: false,
                case_exact: false,
                mutability: Mutability::ReadWrite,
                uniqueness: Uniqueness::None,
                canonical_values: vec![
                    "Available".to_string(),
                    "Assigned".to_string(),
                    "InRepair".to_string(),
                    "Retired".to_string(),
                ],
                sub_attributes: vec![],
                returned: Some("default".to_string()),
            },
            // Assigned to - reference to user
            AttributeDefinition {
                name: "assignedTo".to_string(),
                data_type: AttributeType::Reference,
                multi_valued: false,
                required: false,
                case_exact: false,
                mutability: Mutability::ReadWrite,
                uniqueness: Uniqueness::None,
                canonical_values: vec![],
                sub_attributes: vec![],
                returned: Some("default".to_string()),
            },
            // Purchase date
            AttributeDefinition {
                name: "purchaseDate".to_string(),
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

### Step 3: Create the Resource Handler

Use `SchemaResourceBuilder` to create a resource handler:

```rust
use scim_server::SchemaResourceBuilder;
use serde_json::Value;

fn create_device_resource_handler(schema: Schema) -> scim_server::resource::ResourceHandler {
    SchemaResourceBuilder::new(schema)
        // Handle serial number field
        .with_getter("serialNumber", |data| {
            data.get("serialNumber")?.as_str().map(|s| Value::String(s.to_string()))
        })
        .with_setter("serialNumber", |data, value| {
            if let Some(obj) = data.as_object_mut() {
                obj.insert("serialNumber".to_string(), value);
            }
            Ok(())
        })
        // Handle device type field
        .with_getter("deviceType", |data| {
            data.get("deviceType")?.as_str().map(|s| Value::String(s.to_string()))
        })
        .with_setter("deviceType", |data, value| {
            if let Some(obj) = data.as_object_mut() {
                obj.insert("deviceType".to_string(), value);
            }
            Ok(())
        })
        // Handle manufacturer field
        .with_getter("manufacturer", |data| {
            data.get("manufacturer")?.as_str().map(|s| Value::String(s.to_string()))
        })
        .with_setter("manufacturer", |data, value| {
            if let Some(obj) = data.as_object_mut() {
                obj.insert("manufacturer".to_string(), value);
            }
            Ok(())
        })
        // Handle model field
        .with_getter("model", |data| {
            data.get("model")?.as_str().map(|s| Value::String(s.to_string()))
        })
        .with_setter("model", |data, value| {
            if let Some(obj) = data.as_object_mut() {
                obj.insert("model".to_string(), value);
            }
            Ok(())
        })
        // Handle status field
        .with_getter("status", |data| {
            data.get("status")?.as_str().map(|s| Value::String(s.to_string()))
        })
        .with_setter("status", |data, value| {
            if let Some(obj) = data.as_object_mut() {
                obj.insert("status".to_string(), value);
            }
            Ok(())
        })
        // Handle assigned to field
        .with_getter("assignedTo", |data| {
            data.get("assignedTo")?.as_str().map(|s| Value::String(s.to_string()))
        })
        .with_setter("assignedTo", |data, value| {
            if let Some(obj) = data.as_object_mut() {
                obj.insert("assignedTo".to_string(), value);
            }
            Ok(())
        })
        .build()
}
```

### Step 4: Register with SCIM Server

Register your custom resource type with the SCIM server:

```rust
use scim_server::{ScimServer, ScimOperation, StandardResourceProvider, InMemoryStorage};

async fn setup_server_with_device_resource() -> Result<ScimServer<StandardResourceProvider<InMemoryStorage>>, Box<dyn std::error::Error>> {
    // Create storage and provider
    let storage = InMemoryStorage::new();
    let provider = StandardResourceProvider::new(storage);
    let mut server = ScimServer::new(provider)?;

    // Create and register the Device schema
    let device_schema = create_device_schema();
    let device_handler = create_device_resource_handler(device_schema);

    // Register the custom resource type
    server.register_resource_type(
        "Device",
        device_handler,
        vec![
            ScimOperation::Create,
            ScimOperation::Read,
            ScimOperation::Update,
            ScimOperation::Delete,
            ScimOperation::List,
        ],
    )?;

    println!("Device resource type registered successfully!");
    
    // Verify registration
    let supported_types = server.get_supported_resource_types();
    println!("Supported resource types: {:?}", supported_types);

    Ok(server)
}
```

### Step 5: Using the Custom Resource

Once registered, you can work with your custom resource using standard SCIM operations:

```rust
use serde_json::json;

// Create a device resource
let device_data = json!({
    "schemas": ["urn:company:params:scim:schemas:core:2.0:Device"],
    "serialNumber": "SN123456789",
    "deviceType": "Laptop",
    "manufacturer": "Dell",
    "model": "XPS 13",
    "status": "Available"
});

// This would be used in your SCIM endpoints
// POST /Devices
// GET /Devices/{id}
// PUT /Devices/{id}
// DELETE /Devices/{id}
// GET /Devices (with filtering, sorting, pagination)
```

## Advanced Patterns

### Complex Attributes

You can define complex attributes with sub-attributes:

```rust
// Location attribute with sub-attributes
AttributeDefinition {
    name: "location".to_string(),
    data_type: AttributeType::Complex,
    multi_valued: false,
    required: false,
    case_exact: false,
    mutability: Mutability::ReadWrite,
    uniqueness: Uniqueness::None,
    canonical_values: vec![],
    sub_attributes: vec![
        AttributeDefinition {
            name: "building".to_string(),
            data_type: AttributeType::String,
            multi_valued: false,
            required: false,
            case_exact: false,
            mutability: Mutability::ReadWrite,
            uniqueness: Uniqueness::None,
            canonical_values: vec![],
            sub_attributes: vec![],
            returned: Some("default".to_string()),
        },
        AttributeDefinition {
            name: "floor".to_string(),
            data_type: AttributeType::String,
            multi_valued: false,
            required: false,
            case_exact: false,
            mutability: Mutability::ReadWrite,
            uniqueness: Uniqueness::None,
            canonical_values: vec![],
            sub_attributes: vec![],
            returned: Some("default".to_string()),
        },
        AttributeDefinition {
            name: "room".to_string(),
            data_type: AttributeType::String,
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
    returned: Some("default".to_string()),
},
```

### Multi-valued Attributes

For attributes that can have multiple values:

```rust
// Tags attribute - multi-valued string
AttributeDefinition {
    name: "tags".to_string(),
    data_type: AttributeType::String,
    multi_valued: true,  // Multiple values allowed
    required: false,
    case_exact: false,
    mutability: Mutability::ReadWrite,
    uniqueness: Uniqueness::None,
    canonical_values: vec![],
    sub_attributes: vec![],
    returned: Some("default".to_string()),
},
```

### Validation and Constraints

Use the schema definition to enforce business rules:

```rust
// Employee ID with specific format constraints
AttributeDefinition {
    name: "employeeId".to_string(),
    data_type: AttributeType::String,
    multi_valued: false,
    required: true,
    case_exact: true,           // Exact case matching
    mutability: Mutability::Immutable,  // Can't be changed after creation
    uniqueness: Uniqueness::Server,     // Must be unique across server
    canonical_values: vec![],   // No predefined values, but could add regex validation
    sub_attributes: vec![],
    returned: Some("always".to_string()),
},
```

## Integration with Existing Resources

### References to Other Resources

Custom resources can reference standard SCIM resources:

```rust
// Reference to a User
AttributeDefinition {
    name: "owner".to_string(),
    data_type: AttributeType::Reference,
    multi_valued: false,
    required: false,
    case_exact: false,
    mutability: Mutability::ReadWrite,
    uniqueness: Uniqueness::None,
    canonical_values: vec![],
    sub_attributes: vec![],
    returned: Some("default".to_string()),
},
```

Usage in JSON:

```json
{
    "schemas": ["urn:company:params:scim:schemas:core:2.0:Device"],
    "serialNumber": "SN123456",
    "owner": "48af2f60-2d4a-4d7f-8e7f-3b9c1a2e5f8d",  // User ID
    "deviceType": "Laptop"
}
```

### Multi-Resource Operations

Custom resources work with the same operation patterns as standard resources:

```rust
// In your application logic
async fn assign_device_to_user(
    server: &mut ScimServer<impl ResourceProvider>,
    device_id: &str,
    user_id: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    // Get the device
    let device_data = json!({
        "assignedTo": user_id,
        "status": "Assigned"
    });
    
    // Update would go through your SCIM endpoints
    // PATCH /Devices/{device_id}
    Ok(())
}
```

## Best Practices

### Schema Design

1. **Use descriptive URIs**: Follow the pattern `urn:company:params:scim:schemas:core:2.0:ResourceType`
2. **Plan for evolution**: Design schemas that can be extended without breaking changes
3. **Follow SCIM conventions**: Use camelCase for attribute names, appropriate types
4. **Document thoroughly**: Include clear descriptions for schemas and attributes

### Resource Structure

1. **Include standard fields**: Always include `id`, `schemas`, `meta`, and optionally `externalId`
2. **Use appropriate types**: Choose the most specific `AttributeType` for your data
3. **Consider mutability**: Set appropriate `Mutability` levels for different attributes
4. **Plan for uniqueness**: Use `Uniqueness` constraints where business logic requires it

### Performance

1. **Index unique attributes**: Ensure your storage provider indexes unique fields
2. **Consider query patterns**: Design schemas with expected filtering and sorting in mind
3. **Minimize complex nesting**: Deep attribute structures can impact performance
4. **Use references wisely**: Reference attributes should point to stable identifiers

### Testing

Always test your custom resources thoroughly:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_device_resource_creation() {
        let mut server = setup_server_with_device_resource().await.unwrap();
        
        // Test that Device is in supported types
        let types = server.get_supported_resource_types();
        assert!(types.contains(&"Device"));
        
        // Test schema retrieval
        let schema = server.get_schema_for_resource_type("Device").unwrap();
        assert_eq!(schema.name, "Device");
    }

    #[test]
    fn test_device_serialization() {
        let device = Device::new(
            "TEST123".to_string(),
            DeviceType::Laptop,
            "Dell".to_string(),
            "XPS 13".to_string(),
        );
        
        // Test JSON serialization
        let json = serde_json::to_value(&device).unwrap();
        assert_eq!(json["serialNumber"], "TEST123");
        assert_eq!(json["deviceType"], "Laptop");
    }
}
```

## Error Handling

Handle common scenarios when working with custom resources:

```rust
use scim_server::error::ScimError;

async fn safe_device_registration() -> Result<(), Box<dyn std::error::Error>> {
    let storage = InMemoryStorage::new();
    let provider = StandardResourceProvider::new(storage);
    let mut server = ScimServer::new(provider)?;

    let device_schema = create_device_schema();
    let device_handler = create_device_resource_handler(device_schema);

    // Handle registration errors
    match server.register_resource_type("Device", device_handler, vec![ScimOperation::Create]) {
        Ok(()) => {
            println!("Device resource registered successfully");
        }
        Err(ScimError::Internal(msg)) => {
            eprintln!("Failed to register Device resource: {}", msg);
            return Err(msg.into());
        }
        Err(e) => {
            eprintln!("Unexpected error: {:?}", e);
            return Err(e.into());
        }
    }

    Ok(())
}
```

## Next Steps

- [Extensions](./extensions.md) - Add custom attributes to existing resources like User and Group
- [Validation](./validation.md) - Implement custom validation rules for your schemas
- [Provider Implementation](../providers/basic.md) - Create custom storage providers for your resources