# Custom Resources

This guide covers creating entirely new resource types in the SCIM Server library. While User and Group are the standard SCIM resources, you can define custom resource types to model organization-specific entities.

## Overview

Custom resources allow you to:

- **Model business entities** - Devices, applications, roles, projects
- **Extend beyond identity** - Any organizational resource
- **Maintain SCIM compliance** - Follow SCIM patterns and conventions
- **Integrate with existing flows** - Use standard SCIM operations
- **Support multi-tenancy** - Different resource types per tenant

## Creating a Device Resource

### Resource Definition

```rust
use scim_server::models::{Meta, Resource};
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Device {
    pub id: Option<String>,
    pub schemas: Vec<String>,
    pub device_name: String,
    pub device_type: DeviceType,
    pub serial_number: Option<String>,
    pub manufacturer: Option<String>,
    pub model: Option<String>,
    pub operating_system: Option<OperatingSystem>,
    pub owner: Option<DeviceOwner>,
    pub location: Option<DeviceLocation>,
    pub network_info: Option<NetworkInfo>,
    pub security_info: Option<DeviceSecurityInfo>,
    pub active: Option<bool>,
    pub meta: Option<Meta>,
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
pub struct OperatingSystem {
    pub name: String,
    pub version: String,
    pub architecture: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceOwner {
    pub user_id: String,
    pub user_name: Option<String>,
    pub assignment_date: Option<DateTime<Utc>>,
    pub assignment_type: AssignmentType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AssignmentType {
    Personal,
    Shared,
    Pool,
}
```

### Schema Definition

```rust
use scim_server::schema::{SchemaDefinition, AttributeDefinition, AttributeType};

pub fn create_device_schema() -> SchemaDefinition {
    SchemaDefinition {
        id: "urn:company:schemas:core:2.0:Device".to_string(),
        name: "Device".to_string(),
        description: "Device resource schema".to_string(),
        attributes: vec![
            AttributeDefinition {
                name: "deviceName".to_string(),
                attribute_type: AttributeType::String,
                multi_valued: false,
                description: "The name of the device".to_string(),
                required: true,
                case_exact: false,
                mutability: Mutability::ReadWrite,
                returned: Returned::Always,
                uniqueness: Uniqueness::None,
                reference_types: None,
                canonical_values: None,
                sub_attributes: None,
            },
            AttributeDefinition {
                name: "deviceType".to_string(),
                attribute_type: AttributeType::String,
                multi_valued: false,
                description: "The type of device".to_string(),
                required: true,
                case_exact: false,
                mutability: Mutability::ReadWrite,
                returned: Returned::Always,
                uniqueness: Uniqueness::None,
                reference_types: None,
                canonical_values: Some(vec![
                    "Laptop".to_string(),
                    "Desktop".to_string(),
                    "Tablet".to_string(),
                    "Phone".to_string(),
                    "Server".to_string(),
                    "Printer".to_string(),
                    "Other".to_string(),
                ]),
                sub_attributes: None,
            },
            // Add more attributes...
        ],
        meta: SchemaMeta {
            resource_type: "Schema".to_string(),
            location: "/Schemas/urn:company:schemas:core:2.0:Device".to_string(),
            created: Utc::now(),
            last_modified: Utc::now(),
            version: "1.0".to_string(),
        },
    }
}
```

## Resource Provider Implementation

### Storage Provider Extension

```rust
use scim_server::storage::StorageProvider;
use async_trait::async_trait;

#[async_trait]
pub trait DeviceStorageProvider: StorageProvider {
    async fn create_device(
        &self,
        tenant_id: &str,
        device: Device,
    ) -> Result<Device, StorageError>;
    
    async fn get_device(
        &self,
        tenant_id: &str,
        device_id: &str,
    ) -> Result<Device, StorageError>;
    
    async fn update_device(
        &self,
        tenant_id: &str,
        device_id: &str,
        device: Device,
        version: Option<&str>,
    ) -> Result<Device, StorageError>;
    
    async fn delete_device(
        &self,
        tenant_id: &str,
        device_id: &str,
        version: Option<&str>,
    ) -> Result<(), StorageError>;
    
    async fn list_devices(
        &self,
        tenant_id: &str,
        sort_by: Option<&str>,
        sort_order: Option<SortOrder>,
        start_index: Option<usize>,
        count: Option<usize>,
    ) -> Result<ListResponse<Device>, StorageError>;
    
    // Helper methods for common filtering patterns
    async fn find_devices_by_type(
        &self,
        tenant_id: &str,
        device_type: &DeviceType,
    ) -> Result<Vec<Device>, StorageError>;
    
    async fn find_devices_by_status(
        &self,
        tenant_id: &str,
        status: &DeviceStatus,
    ) -> Result<Vec<Device>, StorageError>;
}
```

## Resource Handler

### HTTP Endpoint Implementation

```rust
use scim_server::handlers::{ResourceHandler, ScimHandler};
use axum::{Json, Path, Query};

pub struct DeviceHandler<T: DeviceStorageProvider> {
    storage: T,
    validator: DeviceValidator,
}

impl<T: DeviceStorageProvider> DeviceHandler<T> {
    pub fn new(storage: T) -> Self {
        Self {
            storage,
            validator: DeviceValidator::new(),
        }
    }
}

#[async_trait]
impl<T: DeviceStorageProvider> ResourceHandler<Device> for DeviceHandler<T> {
    async fn create(
        &self,
        tenant_id: &str,
        device: Device,
    ) -> Result<Device, ScimError> {
        // Validate device
        self.validator.validate_create(&device)?;
        
        // Create in storage
        let created_device = self.storage.create_device(tenant_id, device).await?;
        
        Ok(created_device)
    }
    
    async fn get(
        &self,
        tenant_id: &str,
        device_id: &str,
    ) -> Result<Device, ScimError> {
        self.storage.get_device(tenant_id, device_id).await
            .map_err(|e| e.into())
    }
    
    async fn update(
        &self,
        tenant_id: &str,
        device_id: &str,
        device: Device,
        version: Option<&str>,
    ) -> Result<Device, ScimError> {
        // Validate update
        self.validator.validate_update(&device)?;
        
        // Update in storage
        let updated_device = self.storage.update_device(
            tenant_id,
            device_id,
            device,
            version,
        ).await?;
        
        Ok(updated_device)
    }
    
    async fn delete(
        &self,
        tenant_id: &str,
        device_id: &str,
        version: Option<&str>,
    ) -> Result<(), ScimError> {
        self.storage.delete_device(tenant_id, device_id, version).await
            .map_err(|e| e.into())
    }
    
    async fn list(
        &self,
        tenant_id: &str,
        sort_by: Option<&str>,
        sort_order: Option<SortOrder>,
        start_index: Option<usize>,
        count: Option<usize>,
    ) -> Result<ListResponse<Device>, ScimError> {
        self.storage.list_devices(
            tenant_id,
            sort_by,
            sort_order,
            start_index,
            count,
        ).await.map_err(|e| e.into())
    }
    
    // Implement specific search methods for common patterns
    async fn search_by_type(
        &self,
        tenant_id: &str,
        device_type: &DeviceType,
    ) -> Result<Vec<Device>, ScimError> {
        self.storage.find_devices_by_type(tenant_id, device_type)
            .await
            .map_err(|e| e.into())
    }
}
```

## Registration and Configuration

### Registering Custom Resources

```rust
use scim_server::ScimServerBuilder;

let server = ScimServerBuilder::new()
    .with_provider(storage_provider)
    .register_resource_type::<Device>("Device", device_handler)
    .register_schema(create_device_schema())
    .add_endpoint("/Devices", device_routes())
    .build();
```

## Best Practices

### Resource Design
- Follow SCIM naming conventions
- Use appropriate attribute types
- Define proper mutability rules
- Include comprehensive metadata

### Performance
- Index frequently queried attributes
- Implement efficient filtering
- Consider caching for read-heavy resources
- Optimize storage provider operations

### Validation
- Implement comprehensive validation rules
- Validate references to other resources
- Check business logic constraints
- Provide meaningful error messages

## Next Steps

- [Extensions](./extensions.md) - Add custom attributes to existing resources
- [Validation](./validation.md) - Implement custom validation rules