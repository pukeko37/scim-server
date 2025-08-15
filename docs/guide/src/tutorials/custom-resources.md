# Custom Resource Types

This tutorial shows you how to extend the SCIM Server with custom resource types beyond the standard User and Group resources. You'll learn to define schemas, implement type-safe resources, and integrate them with your SCIM server.

## Why Custom Resources?

While SCIM's User and Group resources cover most identity scenarios, enterprise environments often need additional resource types:

- **Projects**: Development projects with team assignments
- **Roles**: Fine-grained permission sets
- **Devices**: Mobile devices and laptops assigned to users
- **Applications**: SaaS applications and their configurations
- **Departments**: Organizational units with hierarchies
- **Locations**: Office locations and room assignments

Custom resources let you manage these entities with the same SCIM operations and guarantees as built-in resources.

## Quick Start Example

Let's start with a simple Device resource:

```rust
use scim_server::{ScimResource, ResourceMeta, Schema, Attribute};
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Device {
    pub id: String,
    pub schemas: Vec<String>,
    pub meta: ResourceMeta,
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
    Other(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DeviceStatus {
    Available,
    Assigned,
    InRepair,
    Retired,
}

impl ScimResource for Device {
    fn id(&self) -> &str {
        &self.id
    }
    
    fn schemas(&self) -> &[String] {
        &self.schemas
    }
    
    fn meta(&self) -> &ResourceMeta {
        &self.meta
    }
    
    fn external_id(&self) -> Option<&str> {
        self.external_id.as_deref()
    }
}
```

## Step 1: Define Your Resource Schema

Every custom resource needs a schema that defines its structure and validation rules:

```rust
use scim_server::{Schema, Attribute, AttributeType, Mutability, Returned, Uniqueness};

pub fn device_schema() -> Schema {
    Schema::builder()
        .id("urn:company:params:scim:schemas:core:2.0:Device")
        .name("Device")
        .description("IT Device Resource")
        .attribute(
            Attribute::builder()
                .name("serialNumber")
                .type_(AttributeType::String)
                .mutability(Mutability::Immutable)  // Can't change after creation
                .returned(Returned::Default)
                .uniqueness(Uniqueness::Server)     // Must be unique
                .required(true)
                .case_exact(true)
                .description("Device serial number")
                .build()
        )
        .attribute(
            Attribute::builder()
                .name("deviceType")
                .type_(AttributeType::String)
                .mutability(Mutability::ReadWrite)
                .returned(Returned::Default)
                .required(true)
                .canonical_values(vec![
                    "Laptop".to_string(),
                    "Desktop".to_string(),
                    "Tablet".to_string(),
                    "Phone".to_string(),
                ])
                .description("Type of device")
                .build()
        )
        .attribute(
            Attribute::builder()
                .name("assignedTo")
                .type_(AttributeType::Reference)
                .mutability(Mutability::ReadWrite)
                .returned(Returned::Default)
                .reference_types(vec!["User".to_string()])
                .description("User this device is assigned to")
                .build()
        )
        .attribute(
            Attribute::builder()
                .name("status")
                .type_(AttributeType::String)
                .mutability(Mutability::ReadWrite)
                .returned(Returned::Default)
                .required(true)
                .canonical_values(vec![
                    "Available".to_string(),
                    "Assigned".to_string(),
                    "InRepair".to_string(),
                    "Retired".to_string(),
                ])
                .build()
        )
        .build()
        .unwrap()
}
```

## Step 2: Implement the Builder Pattern

Provide a convenient builder for creating resources:

```rust
impl Device {
    pub fn builder() -> DeviceBuilder {
        DeviceBuilder::new()
    }
}

pub struct DeviceBuilder {
    device: Device,
}

impl DeviceBuilder {
    pub fn new() -> Self {
        Self {
            device: Device {
                id: uuid::Uuid::new_v4().to_string(),
                schemas: vec!["urn:company:params:scim:schemas:core:2.0:Device".to_string()],
                meta: ResourceMeta::new("Device"),
                external_id: None,
                serial_number: String::new(),
                device_type: DeviceType::Laptop,
                manufacturer: String::new(),
                model: String::new(),
                assigned_to: None,
                status: DeviceStatus::Available,
                purchase_date: None,
                warranty_expires: None,
            },
        }
    }
    
    pub fn serial_number(mut self, serial: impl Into<String>) -> Self {
        self.device.serial_number = serial.into();
        self
    }
    
    pub fn device_type(mut self, device_type: DeviceType) -> Self {
        self.device.device_type = device_type;
        self
    }
    
    pub fn manufacturer(mut self, manufacturer: impl Into<String>) -> Self {
        self.device.manufacturer = manufacturer.into();
        self
    }
    
    pub fn model(mut self, model: impl Into<String>) -> Self {
        self.device.model = model.into();
        self
    }
    
    pub fn assigned_to(mut self, user_id: Option<impl Into<String>>) -> Self {
        self.device.assigned_to = user_id.map(|id| id.into());
        self
    }
    
    pub fn status(mut self, status: DeviceStatus) -> Self {
        self.device.status = status;
        self
    }
    
    pub fn purchase_date(mut self, date: DateTime<Utc>) -> Self {
        self.device.purchase_date = Some(date);
        self
    }
    
    pub fn warranty_expires(mut self, date: DateTime<Utc>) -> Self {
        self.device.warranty_expires = Some(date);
        self
    }
    
    pub fn build(self) -> Result<Device, ValidationError> {
        // Validate required fields
        if self.device.serial_number.is_empty() {
            return Err(ValidationError::RequiredField("serialNumber"));
        }
        
        if self.device.manufacturer.is_empty() {
            return Err(ValidationError::RequiredField("manufacturer"));
        }
        
        if self.device.model.is_empty() {
            return Err(ValidationError::RequiredField("model"));
        }
        
        Ok(self.device)
    }
}
```

## Step 3: Extend Your Provider

Add support for your custom resource to your storage provider:

```rust
use async_trait::async_trait;
use scim_server::{Provider, ProviderError, ListOptions, ListResponse};

#[async_trait]
pub trait DeviceProvider: Provider {
    async fn create_device(&self, tenant_id: &str, device: Device) -> Result<Device, ProviderError>;
    async fn get_device(&self, tenant_id: &str, device_id: &str) -> Result<Option<Device>, ProviderError>;
    async fn update_device(&self, tenant_id: &str, device: Device) -> Result<Device, ProviderError>;
    async fn delete_device(&self, tenant_id: &str, device_id: &str) -> Result<(), ProviderError>;
    async fn list_devices(&self, tenant_id: &str, options: &ListOptions) -> Result<ListResponse<Device>, ProviderError>;
}

// Implement for InMemoryProvider
#[async_trait]
impl DeviceProvider for InMemoryProvider {
    async fn create_device(&self, tenant_id: &str, mut device: Device) -> Result<Device, ProviderError> {
        // Update metadata
        device.meta.created = Utc::now();
        device.meta.last_modified = device.meta.created;
        device.meta.version = "1".to_string();
        
        let mut devices = self.devices.write().await;
        let tenant_devices = devices.entry(tenant_id.to_string()).or_insert_with(HashMap::new);
        
        // Check for duplicate serial number
        for existing_device in tenant_devices.values() {
            if existing_device.serial_number == device.serial_number {
                return Err(ProviderError::Conflict(
                    format!("Device with serial number {} already exists", device.serial_number)
                ));
            }
        }
        
        tenant_devices.insert(device.id.clone(), device.clone());
        Ok(device)
    }
    
    async fn get_device(&self, tenant_id: &str, device_id: &str) -> Result<Option<Device>, ProviderError> {
        let devices = self.devices.read().await;
        let result = devices
            .get(tenant_id)
            .and_then(|tenant_devices| tenant_devices.get(device_id))
            .cloned();
        Ok(result)
    }
    
    async fn update_device(&self, tenant_id: &str, mut device: Device) -> Result<Device, ProviderError> {
        let mut devices = self.devices.write().await;
        let tenant_devices = devices.entry(tenant_id.to_string()).or_insert_with(HashMap::new);
        
        // Check if device exists
        let existing = tenant_devices.get(&device.id)
            .ok_or_else(|| ProviderError::NotFound {
                resource_type: "Device".to_string(),
                id: device.id.clone(),
            })?;
        
        // Version check for concurrency control
        if existing.meta.version != device.meta.version {
            return Err(ProviderError::VersionConflict {
                current_version: existing.meta.version.clone(),
                provided_version: device.meta.version.clone(),
            });
        }
        
        // Update metadata
        device.meta.last_modified = Utc::now();
        device.meta.version = (existing.meta.version.parse::<u64>().unwrap_or(0) + 1).to_string();
        
        tenant_devices.insert(device.id.clone(), device.clone());
        Ok(device)
    }
    
    async fn delete_device(&self, tenant_id: &str, device_id: &str) -> Result<(), ProviderError> {
        let mut devices = self.devices.write().await;
        let tenant_devices = devices.entry(tenant_id.to_string()).or_insert_with(HashMap::new);
        
        tenant_devices.remove(device_id)
            .ok_or_else(|| ProviderError::NotFound {
                resource_type: "Device".to_string(),
                id: device_id.to_string(),
            })?;
        
        Ok(())
    }
    
    async fn list_devices(&self, tenant_id: &str, options: &ListOptions) -> Result<ListResponse<Device>, ProviderError> {
        let devices = self.devices.read().await;
        let tenant_devices = devices.get(tenant_id).map(|d| d.values().cloned().collect::<Vec<_>>())
            .unwrap_or_default();
        
        // Apply filtering
        let filtered: Vec<Device> = if let Some(ref filter) = options.filter {
            tenant_devices.into_iter()
                .filter(|device| self.matches_filter(device, filter))
                .collect()
        } else {
            tenant_devices
        };
        
        // Apply sorting
        let mut sorted = filtered;
        if let Some(ref sort_by) = options.sort_by {
            sorted.sort_by(|a, b| self.compare_devices(a, b, sort_by, &options.sort_order));
        }
        
        // Apply pagination
        let total_results = sorted.len();
        let start_index = options.start_index.unwrap_or(1).max(1) - 1;
        let count = options.count.unwrap_or(100).min(1000);
        
        let page: Vec<Device> = sorted
            .into_iter()
            .skip(start_index)
            .take(count)
            .collect();
        
        Ok(ListResponse {
            schemas: vec!["urn:ietf:params:scim:api:messages:2.0:ListResponse".to_string()],
            total_results,
            start_index: start_index + 1,
            items_per_page: page.len(),
            resources: page,
        })
    }
}
```

## Step 4: Add HTTP Endpoints

Create HTTP endpoints for your custom resource:

```rust
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Json,
    routing::{get, post, put, delete},
    Router,
};
use serde_json::Value;

pub fn device_routes() -> Router<AppState> {
    Router::new()
        .route("/Devices", get(list_devices).post(create_device))
        .route("/Devices/:id", get(get_device).put(update_device).delete(delete_device))
}

async fn create_device(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Json(payload): Json<Value>,
) -> Result<Json<Device>, (StatusCode, Json<ScimError>)> {
    // Parse the JSON into a Device
    let device: Device = serde_json::from_value(payload)
        .map_err(|e| (StatusCode::BAD_REQUEST, Json(ScimError::invalid_syntax(e.to_string()))))?;
    
    // Create the device
    let created_device = state.provider.create_device(&tenant_id, device).await
        .map_err(|e| match e {
            ProviderError::Conflict(msg) => (StatusCode::CONFLICT, Json(ScimError::uniqueness(msg))),
            ProviderError::ValidationError { message } => (StatusCode::BAD_REQUEST, Json(ScimError::invalid_value(message))),
            _ => (StatusCode::INTERNAL_SERVER_ERROR, Json(ScimError::internal_error())),
        })?;
    
    Ok(Json(created_device))
}

async fn get_device(
    State(state): State<AppState>,
    Path((tenant_id, device_id)): Path<(String, String)>,
) -> Result<Json<Device>, (StatusCode, Json<ScimError>)> {
    let device = state.provider.get_device(&tenant_id, &device_id).await
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, Json(ScimError::internal_error())))?
        .ok_or_else(|| (StatusCode::NOT_FOUND, Json(ScimError::not_found("Device", &device_id))))?;
    
    Ok(Json(device))
}

async fn update_device(
    State(state): State<AppState>,
    Path((tenant_id, device_id)): Path<(String, String)>,
    Json(mut payload): Json<Device>,
) -> Result<Json<Device>, (StatusCode, Json<ScimError>)> {
    // Ensure the ID matches the path
    payload.id = device_id;
    
    let updated_device = state.provider.update_device(&tenant_id, payload).await
        .map_err(|e| match e {
            ProviderError::NotFound { .. } => (StatusCode::NOT_FOUND, Json(ScimError::not_found("Device", &payload.id))),
            ProviderError::VersionConflict { .. } => (StatusCode::PRECONDITION_FAILED, Json(ScimError::version_conflict())),
            _ => (StatusCode::INTERNAL_SERVER_ERROR, Json(ScimError::internal_error())),
        })?;
    
    Ok(Json(updated_device))
}

async fn delete_device(
    State(state): State<AppState>,
    Path((tenant_id, device_id)): Path<(String, String)>,
) -> Result<StatusCode, (StatusCode, Json<ScimError>)> {
    state.provider.delete_device(&tenant_id, &device_id).await
        .map_err(|e| match e {
            ProviderError::NotFound { .. } => (StatusCode::NOT_FOUND, Json(ScimError::not_found("Device", &device_id))),
            _ => (StatusCode::INTERNAL_SERVER_ERROR, Json(ScimError::internal_error())),
        })?;
    
    Ok(StatusCode::NO_CONTENT)
}

async fn list_devices(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Query(params): Query<ListParameters>,
) -> Result<Json<ListResponse<Device>>, (StatusCode, Json<ScimError>)> {
    let options = ListOptions::from_query_params(params);
    
    let response = state.provider.list_devices(&tenant_id, &options).await
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, Json(ScimError::internal_error())))?;
    
    Ok(Json(response))
}
```

## Step 5: Register Your Resource

Register your custom resource and schema with the SCIM server:

```rust
use scim_server::{ScimServer, SchemaRegistry};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create provider
    let provider = InMemoryProvider::new();
    
    // Create schema registry and register schemas
    let mut schema_registry = SchemaRegistry::new();
    schema_registry.register(CoreSchemas::user());
    schema_registry.register(CoreSchemas::group());
    schema_registry.register(device_schema());  // Register our custom schema
    
    // Create SCIM server
    let scim_server = ScimServer::builder()
        .provider(provider)
        .schema_registry(schema_registry)
        .build();
    
    // Create HTTP router
    let app = Router::new()
        .nest("/scim/v2/:tenant_id", user_routes())
        .nest("/scim/v2/:tenant_id", group_routes())
        .nest("/scim/v2/:tenant_id", device_routes())  // Add device routes
        .route("/scim/v2/:tenant_id/Schemas", get(get_schemas))
        .route("/scim/v2/:tenant_id/ResourceTypes", get(get_resource_types))
        .with_state(AppState { provider: scim_server });
    
    // Start server
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await?;
    println!("SCIM server running on http://localhost:3000");
    axum::serve(listener, app).await?;
    
    Ok(())
}
```

## Step 6: Add Resource Type Configuration

Expose your custom resource through the `/ResourceTypes` endpoint:

```rust
async fn get_resource_types(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
) -> Json<ListResponse<ResourceType>> {
    let resource_types = vec![
        ResourceType {
            schemas: vec!["urn:ietf:params:scim:schemas:core:2.0:ResourceType".to_string()],
            id: "User".to_string(),
            name: "User".to_string(),
            endpoint: "/Users".to_string(),
            description: Some("User Account".to_string()),
            schema: "urn:ietf:params:scim:schemas:core:2.0:User".to_string(),
            schema_extensions: vec![
                SchemaExtension {
                    schema: "urn:ietf:params:scim:schemas:extension:enterprise:2.0:User".to_string(),
                    required: false,
                }
            ],
            meta: ResourceMeta::new("ResourceType"),
        },
        ResourceType {
            schemas: vec!["urn:ietf:params:scim:schemas:core:2.0:ResourceType".to_string()],
            id: "Group".to_string(),
            name: "Group".to_string(),
            endpoint: "/Groups".to_string(),
            description: Some("Group".to_string()),
            schema: "urn:ietf:params:scim:schemas:core:2.0:Group".to_string(),
            schema_extensions: vec![],
            meta: ResourceMeta::new("ResourceType"),
        },
        ResourceType {
            schemas: vec!["urn:ietf:params:scim:schemas:core:2.0:ResourceType".to_string()],
            id: "Device".to_string(),
            name: "Device".to_string(),
            endpoint: "/Devices".to_string(),
            description: Some("IT Device".to_string()),
            schema: "urn:company:params:scim:schemas:core:2.0:Device".to_string(),
            schema_extensions: vec![],
            meta: ResourceMeta::new("ResourceType"),
        },
    ];
    
    Json(ListResponse {
        schemas: vec!["urn:ietf:params:scim:api:messages:2.0:ListResponse".to_string()],
        total_results: resource_types.len(),
        start_index: 1,
        items_per_page: resource_types.len(),
        resources: resource_types,
    })
}
```

## Advanced Examples

### Complex Resource with Relationships

Here's a more complex example - a Project resource that references users and groups:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    pub id: String,
    pub schemas: Vec<String>,
    pub meta: ResourceMeta,
    pub external_id: Option<String>,
    
    // Basic attributes
    pub name: String,
    pub description: Option<String>,
    pub status: ProjectStatus,
    pub priority: Priority,
    
    // Relationships
    pub owner: Reference,
    pub team_members: Vec<Reference>,
    pub stakeholder_groups: Vec<Reference>,
    
    // Dates
    pub start_date: DateTime<Utc>,
    pub end_date: Option<DateTime<Utc>>,
    pub created_date: DateTime<Utc>,
    
    // Business attributes
    pub budget: Option<Money>,
    pub tags: Vec<String>,
    pub custom_fields: HashMap<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Reference {
    pub value: String,
    #[serde(rename = "$ref")]
    pub ref_: Option<String>,
    #[serde(rename = "type")]
    pub type_: Option<String>,
    pub display: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Money {
    pub amount: f64,
    pub currency: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ProjectStatus {
    Planning,
    Active,
    OnHold,
    Completed,
    Cancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Priority {
    Low,
    Medium,
    High,
    Critical,
}
```

### Resource with Validation Rules

Add complex validation logic:

```rust
impl Project {
    pub fn validate(&self) -> Result<(), ValidationError> {
        // Name validation
        if self.name.trim().is_empty() {
            return Err(ValidationError::RequiredField("name"));
        }
        
        if self.name.len() > 100 {
            return Err(ValidationError::ValueTooLong("name", 100));
        }
        
        // Date validation
        if let Some(end_date) = self.end_date {
            if end_date <= self.start_date {
                return Err(ValidationError::InvalidDateRange);
            }
        }
        
        // Budget validation
        if let Some(ref budget) = self.budget {
            if budget.amount < 0.0 {
                return Err(ValidationError::InvalidValue("budget.amount", "must be non-negative"));
            }
        }
        
        // Team size validation
        if self.team_members.len() > 50 {
            return Err(ValidationError::ValueTooLong("teamMembers", 50));
        }
        
        // Custom business rules
        if self.status == ProjectStatus::Active && self.team_members.is_empty() {
            return Err(ValidationError::BusinessRule("Active projects must have team members"));
        }
        
        Ok(())
    }
}
```

## Testing Your Custom Resource

Create comprehensive tests for your custom resource:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    
    #[tokio::test]
    async fn test_device_lifecycle() {
        let provider = InMemoryProvider::new();
        let tenant_id = "test-tenant";
        
        // Create device
        let device = Device::builder()
            .serial_number("ABC123")
            .device_type(DeviceType::Laptop)
            .manufacturer("Dell")
            .model("XPS 13")
            .status(DeviceStatus::Available)
            .build()
            .unwrap();
        
        let created = provider.create_device(tenant_id, device.clone()).await.unwrap();
        assert_eq!(created.serial_number, "ABC123");
        assert_eq!(created.status, DeviceStatus::Available);
        
        // Assign device to user
        let mut assigned = created.clone();
        assigned.assigned_to = Some("user-123".to_string());
        assigned.status = DeviceStatus::Assigned;
        
        let updated = provider.update_device(tenant_id, assigned).await.unwrap();
        assert_eq!(updated.assigned_to, Some("user-123".to_string()));
        assert_eq!(updated.status, DeviceStatus::Assigned);
        
        // List devices
        let list_response = provider.list_devices(tenant_id, &ListOptions::default()).await.unwrap();
        assert_eq!(list_response.total_results, 1);
        assert_eq!(list_response.resources[0].id, created.id);
        
        // Delete device
        provider.delete_device(tenant_id, &created.id).await.unwrap();
        let deleted = provider.get_device(tenant_id, &created.id).await.unwrap();
        assert!(deleted.is_none());
    }
    
    #[tokio::test]
    async fn test_device_validation() {
        // Test missing serial number
        let result = Device::builder()
            .manufacturer("Dell")
            .model("XPS 13")
            .build();
        
        assert!(result.is_err());
        
        // Test valid device
        let result = Device::builder()
            .serial_number("ABC123")
            .manufacturer("Dell")
            .model("XPS 13")
            .build();
        
        assert!(result.is_ok());
    }
    
    #[tokio::test]
    async fn test_device_filtering() {
        let provider = InMemoryProvider::new();
        let tenant_id = "test-tenant";
        
        // Create test devices
        let devices = vec![
            Device::builder().serial_number("LAP001").device_type(DeviceType::Laptop).build().unwrap(),
            Device::builder().serial_number("PHN001").device_type(DeviceType::Phone).build().unwrap(),
            Device::builder().serial_number("LAP002").device_type(DeviceType::Laptop).build().unwrap(),
        ];
        
        for device in devices {
            provider.create_device(tenant_id, device).await.unwrap();
        }
        
        // List all devices and filter in memory (database filtering not yet implemented)
        let options = ListOptions::default();
        let response = provider.list_devices(tenant_id, &options).await.unwrap();
        
        // Filter by device type in memory
        let laptops: Vec<_> = response.resources.into_iter()
            .filter(|device| device.device_type == DeviceType::Laptop)
            .collect();
        
        assert_eq!(laptops.len(), 2);
        for device in laptops {
            assert_eq!(device.device_type, DeviceType::Laptop);
        }
    }
}
```

## Best Practices

### Schema Design

**Use meaningful schema IDs**:
```rust
// Good: Company-specific with versioning
"urn:company:params:scim:schemas:core:2.0:Device"

// Bad: Generic or unversioned
"device"
"urn:scim:schemas:device"
```

**Define appropriate constraints**:
```rust
.attribute(
    Attribute::builder()
        .name("serialNumber")
        .mutability(Mutability::Immutable)  // Can't change after creation
        .uniqueness(Uniqueness::Server)     // Must be unique across tenant
        .required(true)                     // Must be provided
        .case_exact(true)                   // Exact case matching
        .build()
)
```

### Type Safety

**Use enums for constrained values**:
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DeviceStatus {
    Available,
    Assigned,
    InRepair,
    Retired,
}

// Instead of just String
```

**Implement strong validation**:
```rust
impl DeviceBuilder {
    pub fn serial_number(mut self, serial: impl Into<String>) -> Self {
        let serial = serial.into();
        
        // Validate format (example: must be alphanumeric, 6-20 chars)
        if !serial.chars().all(|c| c.is_alphanumeric()) {
            panic!("Serial number must be alphanumeric");
        }
        
        if serial.len() < 6 || serial.len() > 20 {
            panic!("Serial number must be 6-20 characters");
        }
        
        self.device.serial_number = serial;
        self
    }
}
```

### Error Handling

**Provide meaningful error messages**:
```rust
#[derive(Debug, thiserror::Error)]
pub enum DeviceError {
    #[error("Serial number {0} is already in use")]
    DuplicateSerial(String),
    
    #[error("Device {device_id} is currently assigned to user {user_id}")]
    DeviceInUse { device_id: String, user_id: String },
    
    #[error("Cannot assign retired device {0}")]
    RetiredDevice(String),
}
```

### Performance

**Add appropriate indexes for your provider**:
```sql
-- For database providers
CREATE INDEX idx_devices_serial_number ON devices(tenant_id, serial_number);
CREATE INDEX idx_devices_assigned_to ON devices(tenant_id, assigned_to);
CREATE INDEX idx_devices_status ON devices(tenant_id, status);
```

**Implement efficient querying**:
```rust
// For now, implement pagination and in-memory filtering
impl DatabaseProvider {
    async fn list_devices_paginated(&self, tenant_id: &str, start_index: Option<usize>, count: Option<usize>) -> Result<Vec<Device>, ProviderError> {
        let skip = start_index.unwrap_or(1).saturating_sub(1);
        let limit = count.unwrap_or(50).min(1000); // Cap at 1000 for performance
        
        // Database query with pagination
        let devices = sqlx::query_as!(
            Device,
            "SELECT * FROM devices WHERE tenant_id = $1 ORDER BY created_at LIMIT $2 OFFSET $3",
            tenant_id, limit as i64, skip as i64
        )
        .fetch_all(&self.pool)
        .await?;
        
        Ok(devices)
    }
    
    // Helper method for common filtering patterns
    async fn find_devices_by_type(&self, tenant_id: &str, device_type: &DeviceType) -> Result<Vec<Device>, ProviderError> {
        let devices = sqlx::query_as!(
            Device,
            "SELECT * FROM devices WHERE tenant_id = $1 AND device_type = $2",
            tenant_id, device_type.to_string()
        )
        .fetch_all(&self.pool)
        .await?;
        
        Ok(devices)