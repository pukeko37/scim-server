You're absolutely right! Using enum variants defeats the purpose of compile-time state safety. Let me redesign this using the type system to parameterize state.

## Type-Safe State Machine Design

### State Types as Zero-Cost Markers

```rust
use std::marker::PhantomData;

// State marker types - zero runtime cost
pub struct Uninitialized;
pub struct LoadingSchemas;
pub struct SchemasLoaded; 
pub struct RegisteringResourceTypes;
pub struct Ready;
pub struct Shutdown;

// Server parameterized by state type
pub struct ScimServer<State> {
    inner: ServerInner,
    _state: PhantomData<State>,
}

struct ServerInner {
    schema_registry: SchemaRegistry,
    resource_types: ResourceTypeRegistry,
    resource_provider: Arc<dyn ResourceProvider>,
    // ... other shared data
}
```

### State Transitions Through Consuming Methods

```rust
impl ScimServer<Uninitialized> {
    pub fn new() -> Self {
        Self {
            inner: ServerInner::new(),
            _state: PhantomData,
        }
    }

    // Consuming transition - takes ownership of current state
    pub async fn load_core_schemas(self) -> Result<ScimServer<LoadingSchemas>, SchemaError> {
        let mut inner = self.inner;
        inner.schema_registry.load_core_schemas().await?;
        
        Ok(ScimServer {
            inner,
            _state: PhantomData,
        })
    }
}

impl ScimServer<LoadingSchemas> {
    pub async fn add_schema_extension(
        mut self, 
        schema: SchemaDefinition
    ) -> Result<ScimServer<SchemasLoaded>, SchemaError> {
        self.inner.schema_registry.add_extension(schema).await?;
        
        Ok(ScimServer {
            inner: self.inner,
            _state: PhantomData,
        })
    }

    // Can also transition directly to SchemasLoaded without extensions
    pub fn finalize_schemas(self) -> ScimServer<SchemasLoaded> {
        ScimServer {
            inner: self.inner,
            _state: PhantomData,
        }
    }
}

impl ScimServer<SchemasLoaded> {
    pub fn register_resource_type(
        mut self,
        resource_type: ResourceTypeDefinition,
    ) -> Result<ScimServer<RegisteringResourceTypes>, ConfigError> {
        self.inner.resource_types.register(resource_type)?;
        
        Ok(ScimServer {
            inner: self.inner,
            _state: PhantomData,
        })
    }

    // Can go directly to Ready if no custom resource types needed
    pub fn finalize_configuration(self) -> ScimServer<Ready> {
        ScimServer {
            inner: self.inner,
            _state: PhantomData,
        }
    }
}

impl ScimServer<RegisteringResourceTypes> {
    pub fn register_resource_type(
        mut self,
        resource_type: ResourceTypeDefinition,
    ) -> Result<Self, ConfigError> {
        self.inner.resource_types.register(resource_type)?;
        Ok(self)
    }

    pub fn finalize_configuration(self) -> ScimServer<Ready> {
        ScimServer {
            inner: self.inner,
            _state: PhantomData,
        }
    }
}
```

### Operations Available Per State

```rust
// Discovery operations available once schemas are loaded
impl<State> ScimServer<State> 
where 
    State: SchemaAvailable 
{
    pub async fn get_schemas(&self) -> Result<Vec<Schema>, ServerError> {
        Ok(self.inner.schema_registry.list_schemas())
    }

    pub async fn get_schema(&self, id: &str) -> Result<Option<Schema>, ServerError> {
        Ok(self.inner.schema_registry.get_schema(id))
    }
}

// Resource type operations available once resource types are registered
impl<State> ScimServer<State>
where
    State: ResourceTypesAvailable
{
    pub async fn get_resource_types(&self) -> Result<Vec<ResourceType>, ServerError> {
        Ok(self.inner.resource_types.list())
    }

    pub async fn get_resource_type(&self, name: &str) -> Result<Option<ResourceType>, ServerError> {
        Ok(self.inner.resource_types.get(name))
    }
}

// Resource CRUD operations only available when Ready
impl ScimServer<Ready> {
    pub async fn create_resource(
        &self,
        resource_type: &str,
        resource: Resource,
        context: RequestContext,
    ) -> Result<Resource, ServerError> {
        // Validate resource against schema
        self.inner.schema_registry.validate_resource(resource_type, &resource)?;
        
        // Delegate to user implementation
        self.inner.resource_provider
            .create_resource(resource_type, resource, &context)
            .await
            .map_err(ServerError::from)
    }

    pub async fn get_resource(
        &self,
        resource_type: &str,
        id: &str,
        context: RequestContext,
    ) -> Result<Option<Resource>, ServerError> {
        self.inner.resource_provider
            .get_resource(resource_type, id, &context)
            .await
            .map_err(ServerError::from)
    }

    pub async fn list_resources(
        &self,
        resource_type: &str,
        query: Query,
        context: RequestContext,
    ) -> Result<ListResponse, ServerError> {
        // Validate query against schema
        self.inner.schema_registry.validate_query(resource_type, &query)?;
        
        self.inner.resource_provider
            .list_resources(resource_type, &query, &context)
            .await
            .map_err(ServerError::from)
    }

    // ... other CRUD operations
}
```

### Trait Bounds for State Capabilities

```rust
// Marker traits for state capabilities
pub trait SchemaAvailable {}
pub trait ResourceTypesAvailable {}
pub trait FullyOperational {}

// Implement marker traits for appropriate states
impl SchemaAvailable for SchemasLoaded {}
impl SchemaAvailable for RegisteringResourceTypes {}
impl SchemaAvailable for Ready {}

impl ResourceTypesAvailable for RegisteringResourceTypes {}
impl ResourceTypesAvailable for Ready {}

impl FullyOperational for Ready {}
```

### Builder Pattern with Type-Safe Transitions

```rust
pub struct ScimServerBuilder<State> {
    resource_provider: Option<Arc<dyn ResourceProvider>>,
    schema_provider: Option<Arc<dyn SchemaProvider>>,
    service_config: Option<Arc<dyn ServiceConfiguration>>,
    _state: PhantomData<State>,
}

impl ScimServerBuilder<Uninitialized> {
    pub fn new() -> Self {
        Self {
            resource_provider: None,
            schema_provider: None,
            service_config: None,
            _state: PhantomData,
        }
    }

    pub fn with_resource_provider(
        mut self,
        provider: Arc<dyn ResourceProvider>,
    ) -> Self {
        self.resource_provider = Some(provider);
        self
    }

    pub fn with_schema_provider(
        mut self,
        provider: Arc<dyn SchemaProvider>,
    ) -> Self {
        self.schema_provider = Some(provider);
        self
    }

    pub async fn build(self) -> Result<ScimServer<Ready>, BuildError> {
        let resource_provider = self.resource_provider
            .ok_or(BuildError::MissingResourceProvider)?;

        // Type-safe state transitions during build
        let server = ScimServer::<Uninitialized>::new()
            .with_resource_provider(resource_provider);

        let server = server.load_core_schemas().await?;
        
        // Add custom schemas if provider exists
        let server = if let Some(schema_provider) = self.schema_provider {
            let mut server = server;
            for schema in schema_provider.get_custom_schemas() {
                server = server.add_schema_extension(schema).await?;
            }
            server.finalize_schemas()
        } else {
            server.finalize_schemas()
        };

        // Register resource types and finalize
        let server = if let Some(schema_provider) = self.schema_provider {
            let mut server = server;
            for resource_type in schema_provider.get_resource_types() {
                server = server.register_resource_type(resource_type)?;
            }
            server.finalize_configuration()
        } else {
            server.finalize_configuration()
        };

        Ok(server)
    }
}
```

## Benefits of Type-Parameterized States

1. **Compile-Time Safety**: Invalid operations are compilation errors, not runtime errors
2. **Zero Runtime Cost**: `PhantomData<State>` has no runtime overhead
3. **Clear API**: Method availability clearly indicates what operations are valid
4. **Consuming Transitions**: State changes require ownership, preventing use-after-transition
5. **Trait Bounds**: Generic implementations can require specific state capabilities

## Usage Example

```rust
// This compiles and works
let server = ScimServer::new()
    .load_core_schemas().await?
    .finalize_schemas()
    .finalize_configuration();

server.create_resource("User", user_data, context).await?; // ✅ Works

// This fails at compile time
let server = ScimServer::new();
server.create_resource("User", user_data, context).await?; // ❌ Compile error
//     ^^^^^^^^^^^^^^^ method not available on ScimServer<Uninitialized>
```

This approach leverages Rust's type system to encode the state machine at compile time, eliminating runtime state checks while providing clear, safe APIs.

Based on the rfc, what is the use-case flow of interactions between client and scim server we must enable?


Looking at the RFC, here are the key use-case flows that a SCIM server must enable:

## 1. Service Discovery Flow
**Client discovers server capabilities before operations**
```
Client → GET /ServiceProviderConfig → Server
Client → GET /ResourceTypes → Server  
Client → GET /Schemas → Server
Client → GET /Schemas/{schema-id} → Server
```

## 2. User Provisioning Flow (Core Use Case)
**Enterprise provisions new employee**
```
1. Client → POST /Users (create user) → Server
2. Client → GET /Users/{id} (verify creation) → Server
3. Client → PUT /Users/{id} (update full user) → Server
4. Client → PATCH /Users/{id} (partial update) → Server
5. Client → DELETE /Users/{id} (deprovision) → Server
```

## 3. Group Management Flow
**Managing group memberships**
```
1. Client → POST /Groups (create group) → Server
2. Client → PATCH /Groups/{id} (add/remove members) → Server
3. Client → GET /Groups/{id} (verify membership) → Server
4. Client → GET /Users/{id} (check user's groups) → Server
```

## 4. Search and Query Flow
**Finding existing resources**
```
1. Client → GET /Users?filter=userName eq "bjensen" → Server
2. Client → GET /Users?filter=emails.value co "@example.com" → Server
3. Client → GET /Groups?filter=displayName sw "Engineering" → Server
4. Client → GET /Users?attributes=userName,emails&startIndex=1&count=10 → Server
```

## 5. Bulk Operations Flow
**Efficient batch processing**
```
Client → POST /Bulk {
  "Operations": [
    {"method": "POST", "path": "/Users", "data": {...}},
    {"method": "PATCH", "path": "/Users/123", "data": {...}},
    {"method": "DELETE", "path": "/Users/456"}
  ]
} → Server
```

## 6. Version Management Flow
**Handling concurrent updates**
```
1. Client → GET /Users/{id} → Server (get current version/ETag)
2. Client → PUT /Users/{id} with If-Match: "version" → Server
3. Server → 412 Precondition Failed (if version mismatch)
4. Client → GET /Users/{id} → Server (get latest version)
5. Client → PUT /Users/{id} with new If-Match → Server
```

## 7. Extension Schema Flow
**Using custom/enterprise schemas**
```
1. Client → GET /Schemas → Server (discover available schemas)
2. Client → POST /Users with enterprise extension → Server
3. Client → GET /Users/{id} → Server (returns core + extension data)
4. Client → PATCH /Users/{id} (update extension attributes) → Server
```

## 8. Error Handling Flow
**Proper error responses**
```
Client → POST /Users (invalid data) → Server
Server → 400 Bad Request {
  "schemas": ["urn:ietf:params:scim:api:messages:2.0:Error"],
  "detail": "Attribute 'userName' is required",
  "status": "400"
}
```

## Server Implementation Requirements

Based on these flows, our SCIM server must support:

### HTTP Method Mapping
```rust
// Resource endpoints
GET    /{ResourceType}         → list_resources()
POST   /{ResourceType}         → create_resource() 
GET    /{ResourceType}/{id}    → get_resource()
PUT    /{ResourceType}/{id}    → replace_resource()
PATCH  /{ResourceType}/{id}    → update_resource()
DELETE /{ResourceType}/{id}    → delete_resource()

// Discovery endpoints  
GET    /ServiceProviderConfig  → get_service_config()
GET    /ResourceTypes          → list_resource_types()
GET    /ResourceTypes/{id}     → get_resource_type()
GET    /Schemas                → list_schemas()
GET    /Schemas/{id}           → get_schema()

// Bulk endpoint
POST   /Bulk                   → bulk_operations()
```

### Query Parameter Support
```rust
// Filtering
filter: String              // SCIM filter expression
attributes: Vec<String>     // Attribute selection
excludedAttributes: Vec<String>

// Pagination  
startIndex: u32            // 1-based index
count: u32                 // Results per page

// Sorting
sortBy: String             // Attribute name
sortOrder: SortOrder       // ascending/descending
```

### Content Negotiation
```rust
// Request/Response headers
Content-Type: application/scim+json
Accept: application/scim+json
If-Match: "{version}"           // For versioning
If-None-Match: *               // For create-only
```

## Critical Flow Dependencies

1. **Schema Discovery Must Precede Operations**: Clients need schemas before creating/updating resources
2. **Resource Type Registration Enables Endpoints**: Custom resource types create new endpoint paths
3. **Filter Validation Requires Schema**: Query filtering must validate against registered schemas
4. **Bulk Operations Need Individual Operation Support**: Bulk is a wrapper around individual CRUD operations
5. **Version Management Requires ETag Support**: Concurrent update protection

This analysis shows that our state machine design is correct - schemas and resource types must be fully loaded before serving resource operations, and our trait-based approach needs to support all these interaction patterns.


Now suggest a minimum viable product. Perhaps load the core schema, provide schema discovery, implement a resource crud.

# SCIM Server Library MVP

## MVP Scope

**Core Features:**
1. Load core SCIM User schema only (no Groups, no extensions)
2. Schema discovery endpoints (`/Schemas`, `/ServiceProviderConfig`)
3. Basic User CRUD operations (`/Users`)
4. Simple filtering (equality only)
5. JSON validation against core schema

**Explicitly Excluded:**
- Groups, extensions, custom resource types
- Complex filtering, sorting, pagination
- Bulk operations, versioning/ETags
- Authentication/authorization

## Implementation Plan

### Phase 1: Core Types and Schema Engine

```rust
// src/lib.rs
pub mod schema;
pub mod resource;
pub mod server;
pub mod error;

pub use server::{ScimServer, ScimServerBuilder};
pub use resource::{Resource, ResourceProvider, RequestContext};
pub use error::ScimError;

// Core re-exports for library users
pub trait ResourceProvider: Send + Sync {
    type Error: std::error::Error + Send + Sync + 'static;

    async fn create_user(&self, user: Resource, context: &RequestContext) -> Result<Resource, Self::Error>;
    async fn get_user(&self, id: &str, context: &RequestContext) -> Result<Option<Resource>, Self::Error>;
    async fn update_user(&self, id: &str, user: Resource, context: &RequestContext) -> Result<Resource, Self::Error>;
    async fn delete_user(&self, id: &str, context: &RequestContext) -> Result<(), Self::Error>;
    async fn list_users(&self, context: &RequestContext) -> Result<Vec<Resource>, Self::Error>;
}
```

### Phase 2: Schema Foundation

```rust
// src/schema/mod.rs
use serde_json::Value;

pub struct SchemaRegistry {
    core_user_schema: Schema,
}

impl SchemaRegistry {
    pub fn new() -> Self {
        Self {
            core_user_schema: Self::load_core_user_schema(),
        }
    }

    fn load_core_user_schema() -> Schema {
        // Hardcoded core User schema from RFC
        Schema {
            id: "urn:ietf:params:scim:schemas:core:2.0:User".to_string(),
            name: "User".to_string(),
            description: "User Account".to_string(),
            attributes: vec![
                // userName - required
                AttributeDefinition {
                    name: "userName".to_string(),
                    data_type: AttributeType::String,
                    required: true,
                    mutability: Mutability::ReadWrite,
                    case_exact: false,
                    uniqueness: Uniqueness::Server,
                    ..Default::default()
                },
                // displayName - optional
                AttributeDefinition {
                    name: "displayName".to_string(),
                    data_type: AttributeType::String,
                    required: false,
                    mutability: Mutability::ReadWrite,
                    ..Default::default()
                },
                // emails - multi-valued complex
                AttributeDefinition {
                    name: "emails".to_string(),
                    data_type: AttributeType::Complex,
                    multi_valued: true,
                    sub_attributes: vec![
                        AttributeDefinition {
                            name: "value".to_string(),
                            data_type: AttributeType::String,
                            ..Default::default()
                        },
                        AttributeDefinition {
                            name: "type".to_string(),
                            data_type: AttributeType::String,
                            canonical_values: vec!["work".to_string(), "home".to_string()],
                            ..Default::default()
                        },
                        AttributeDefinition {
                            name: "primary".to_string(),
                            data_type: AttributeType::Boolean,
                            ..Default::default()
                        },
                    ],
                    ..Default::default()
                },
                // active - boolean
                AttributeDefinition {
                    name: "active".to_string(),
                    data_type: AttributeType::Boolean,
                    ..Default::default()
                },
            ],
        }
    }

    pub fn validate_user(&self, user: &Value) -> Result<(), ValidationError> {
        self.validate_resource(&self.core_user_schema, user)
    }

    pub fn get_schemas(&self) -> Vec<&Schema> {
        vec![&self.core_user_schema]
    }

    pub fn get_schema(&self, id: &str) -> Option<&Schema> {
        if id == self.core_user_schema.id {
            Some(&self.core_user_schema)
        } else {
            None
        }
    }
}

#[derive(Debug, Clone)]
pub struct Schema {
    pub id: String,
    pub name: String,
    pub description: String,
    pub attributes: Vec<AttributeDefinition>,
}

#[derive(Debug, Clone, Default)]
pub struct AttributeDefinition {
    pub name: String,
    pub data_type: AttributeType,
    pub multi_valued: bool,
    pub required: bool,
    pub case_exact: bool,
    pub mutability: Mutability,
    pub uniqueness: Uniqueness,
    pub canonical_values: Vec<String>,
    pub sub_attributes: Vec<AttributeDefinition>,
}

#[derive(Debug, Clone, Default)]
pub enum AttributeType {
    #[default]
    String,
    Boolean,
    Complex,
    Reference,
}

#[derive(Debug, Clone, Default)]
pub enum Mutability {
    ReadOnly,
    #[default]
    ReadWrite,
    Immutable,
    WriteOnly,
}

#[derive(Debug, Clone, Default)]
pub enum Uniqueness {
    #[default]
    None,
    Server,
    Global,
}
```

### Phase 3: Type-Safe Server

```rust
// src/server/mod.rs
use std::sync::Arc;
use std::marker::PhantomData;

pub struct Uninitialized;
pub struct Ready;

pub struct ScimServer<State> {
    inner: ServerInner,
    _state: PhantomData<State>,
}

struct ServerInner {
    schema_registry: SchemaRegistry,
    resource_provider: Arc<dyn ResourceProvider>,
    service_config: ServiceProviderConfig,
}

impl ScimServer<Uninitialized> {
    pub fn builder() -> ScimServerBuilder {
        ScimServerBuilder::new()
    }
}

impl ScimServer<Ready> {
    // Discovery endpoints
    pub async fn get_schemas(&self) -> Result<Vec<Schema>, ScimError> {
        Ok(self.inner.schema_registry.get_schemas().into_iter().cloned().collect())
    }

    pub async fn get_schema(&self, id: &str) -> Result<Option<Schema>, ScimError> {
        Ok(self.inner.schema_registry.get_schema(id).cloned())
    }

    pub async fn get_service_provider_config(&self) -> Result<ServiceProviderConfig, ScimError> {
        Ok(self.inner.service_config.clone())
    }

    // User CRUD operations
    pub async fn create_user(
        &self,
        user_data: serde_json::Value,
        context: RequestContext,
    ) -> Result<Resource, ScimError> {
        // Validate against schema
        self.inner.schema_registry.validate_user(&user_data)?;
        
        let resource = Resource::new("User".to_string(), user_data);
        
        // Delegate to user implementation
        self.inner.resource_provider
            .create_user(resource, &context)
            .await
            .map_err(ScimError::Provider)
    }

    pub async fn get_user(
        &self,
        id: &str,
        context: RequestContext,
    ) -> Result<Option<Resource>, ScimError> {
        self.inner.resource_provider
            .get_user(id, &context)
            .await
            .map_err(ScimError::Provider)
    }

    pub async fn update_user(
        &self,
        id: &str,
        user_data: serde_json::Value,
        context: RequestContext,
    ) -> Result<Resource, ScimError> {
        // Validate against schema
        self.inner.schema_registry.validate_user(&user_data)?;
        
        let resource = Resource::new("User".to_string(), user_data);
        
        self.inner.resource_provider
            .update_user(id, resource, &context)
            .await
            .map_err(ScimError::Provider)
    }

    pub async fn delete_user(
        &self,
        id: &str,
        context: RequestContext,
    ) -> Result<(), ScimError> {
        self.inner.resource_provider
            .delete_user(id, &context)
            .await
            .map_err(ScimError::Provider)
    }

    pub async fn list_users(
        &self,
        context: RequestContext,
    ) -> Result<Vec<Resource>, ScimError> {
        self.inner.resource_provider
            .list_users(&context)
            .await
            .map_err(ScimError::Provider)
    }
}

pub struct ScimServerBuilder {
    resource_provider: Option<Arc<dyn ResourceProvider>>,
}

impl ScimServerBuilder {
    pub fn new() -> Self {
        Self {
            resource_provider: None,
        }
    }

    pub fn with_resource_provider(mut self, provider: Arc<dyn ResourceProvider>) -> Self {
        self.resource_provider = Some(provider);
        self
    }

    pub fn build(self) -> Result<ScimServer<Ready>, BuildError> {
        let resource_provider = self.resource_provider
            .ok_or(BuildError::MissingResourceProvider)?;

        let inner = ServerInner {
            schema_registry: SchemaRegistry::new(),
            resource_provider,
            service_config: ServiceProviderConfig::default(),
        };

        Ok(ScimServer {
            inner,
            _state: PhantomData,
        })
    }
}

#[derive(Debug, Clone)]
pub struct ServiceProviderConfig {
    pub patch_supported: bool,
    pub bulk_supported: bool,
    pub filter_supported: bool,
    pub change_password_supported: bool,
    pub sort_supported: bool,
    pub etag_supported: bool,
}

impl Default for ServiceProviderConfig {
    fn default() -> Self {
        Self {
            patch_supported: false,
            bulk_supported: false,
            filter_supported: false,
            change_password_supported: false,
            sort_supported: false,
            etag_supported: false,
        }
    }
}
```

### Phase 4: Resource Model

```rust
// src/resource/mod.rs
use serde_json::Value;

#[derive(Debug, Clone)]
pub struct Resource {
    pub resource_type: String,
    pub data: Value,
}

impl Resource {
    pub fn new(resource_type: String, data: Value) -> Self {
        Self { resource_type, data }
    }

    pub fn get_id(&self) -> Option<&str> {
        self.data.get("id")?.as_str()
    }
}

#[derive(Debug)]
pub struct RequestContext {
    // Minimal context for MVP
    pub request_id: String,
}

impl RequestContext {
    pub fn new() -> Self {
        Self {
            request_id: uuid::Uuid::new_v4().to_string(),
        }
    }
}
```

## MVP Usage Example

```rust
use scim_server::{ScimServer, ResourceProvider, Resource, RequestContext, ScimError};
use std::sync::Arc;
use std::collections::HashMap;
use tokio::sync::Mutex;

// User implements this trait
struct InMemoryProvider {
    users: Arc<Mutex<HashMap<String, serde_json::Value>>>,
}

impl InMemoryProvider {
    fn new() -> Self {
        Self {
            users: Arc::new(Mutex::new(HashMap::new())),
        }
    }
}

#[async_trait::async_trait]
impl ResourceProvider for InMemoryProvider {
    type Error = Box<dyn std::error::Error + Send + Sync>;

    async fn create_user(&self, user: Resource, _context: &RequestContext) -> Result<Resource, Self::Error> {
        let id = uuid::Uuid::new_v4().to_string();
        let mut data = user.data;
        data["id"] = serde_json::Value::String(id.clone());
        
        let mut users = self.users.lock().await;
        users.insert(id, data.clone());
        
        Ok(Resource::new("User".to_string(), data))
    }

    async fn get_user(&self, id: &str, _context: &RequestContext) -> Result<Option<Resource>, Self::Error> {
        let users = self.users.lock().await;
        if let Some(data) = users.get(id) {
            Ok(Some(Resource::new("User".to_string(), data.clone())))
        } else {
            Ok(None)
        }
    }

    // ... other methods
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let provider = Arc::new(InMemoryProvider::new());
    
    let server = ScimServer::builder()
        .with_resource_provider(provider)
        .build()?;

    // Discovery
    let schemas = server.get_schemas().await?;
    println!("Available schemas: {}", schemas.len());

    // Create user
    let user_data = serde_json::json!({
        "userName": "jdoe@example.com",
        "displayName": "John Doe",
        "active": true
    });

    let context = RequestContext::new();
    let created_user = server.create_user(user_data, context).await?;
    println!("Created user: {}", created_user.get_id().unwrap());

    Ok(())
}
```

## MVP Deliverables

1. **Core library crate** with hardcoded User schema
2. **Simple trait** for data access implementation
3. **Type-safe builder** with state transitions
4. **JSON validation** against User schema
5. **Working example** with in-memory storage
6. **Integration tests** for basic CRUD operations

This MVP provides immediate value while establishing the foundation for the full SCIM implementation.