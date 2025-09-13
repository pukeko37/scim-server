# Configuration Guide

This guide walks you through configuring a SCIM server application from the ground up. By the end of this guide, you'll understand the essential components needed to build a working SCIM server and the configuration options available at each stage.

## What This Library Does (And Doesn't Do)

The `scim-server` library provides the core SCIM protocol implementation, resource management, and storage abstraction. It handles:

- **SCIM 2.0 protocol compliance** - Resource validation, schema management, and SCIM operations
- **Multi-tenant support** - Tenant isolation and context management
- **Resource lifecycle** - Create, read, update, delete, and list operations
- **Schema validation** - Built-in User and Group schemas with extension support
- **Concurrency control** - ETag-based optimistic locking

**What the library does NOT provide:**

- **HTTP server and routing** - You need a web framework like Axum, Actix-web, or Warp
- **Authentication and authorization** - Implement JWT, OAuth2, or API key validation yourself
- **Database connections** - Choose and configure your own database or storage solution
- **Process/thread management** - Handle async runtimes and server lifecycle in your application
- **Logging configuration** - Set up structured logging with your preferred framework
- **Deployment infrastructure** - Docker, Kubernetes, or cloud deployment is your responsibility

## Architecture Overview

The SCIM server follows a layered architecture where each component builds upon the previous one:

```
┌─────────────────────────────────────────────────────────┐
│                 Your Application                        │
│            (HTTP routes, auth, logging)                 │
├─────────────────────────────────────────────────────────┤
│                   SCIM Server                           │
│            (Protocol, validation, operations)           │
├─────────────────────────────────────────────────────────┤
│                Resource Provider                        │
│              (Business logic, SCIM semantics)           │
├─────────────────────────────────────────────────────────┤
│                 Storage Provider                        │
│              (Data persistence, queries)                │
└─────────────────────────────────────────────────────────┘
```

You configure these components from the bottom up, with each layer depending on the one below it.

## Configuration Steps

### Stage 1: Storage Provider (Foundation)

**Purpose:** The storage provider is the foundation layer that handles all data persistence and retrieval operations for SCIM resources. It provides a protocol-agnostic interface that abstracts away the specific storage implementation details, allowing the higher layers to work with any storage backend. This layer is responsible for storing, retrieving, updating, and deleting JSON resource data, as well as supporting queries and searches across resources within tenant boundaries.

**Default Options:**
- `InMemoryStorage::new()` - For development and testing
- `SqliteStorage::new()` - For file-based persistence (requires `sqlite` feature)

**Configuration:**

```rust
use scim_server::storage::InMemoryStorage;

// Development/testing storage
let storage = InMemoryStorage::new();

// Or for file-based storage
// let storage = SqliteStorage::new().await?;
```

**When to implement custom storage:**
- Database integration (PostgreSQL, MySQL, MongoDB)
- Cloud storage (DynamoDB, CosmosDB) 
- Distributed systems (Redis, Cassandra)
- Search integration (Elasticsearch)
- User backend integrations (AWS Cognito User Pools, Microsoft Entra External ID)
- Legacy system integration (existing user directories, LDAP systems)

**Custom storage example:**
```rust
struct DatabaseStorage {
    pool: sqlx::PgPool,
}

impl StorageProvider for DatabaseStorage {
    type Error = sqlx::Error;
    
    async fn put(&self, key: StorageKey, data: Value) -> Result<Value, Self::Error> {
        // Your database logic here
    }
    // ... implement other required methods
}
```

### Stage 2: Resource Provider (Business Logic)

**Purpose:** The resource provider acts as the business logic layer that implements SCIM 2.0 protocol semantics and organizational rules on top of the storage layer. It handles resource validation, enforces SCIM compliance, manages resource relationships (like group memberships), implements concurrency control through ETags, and provides tenant-aware operations. This layer transforms generic storage operations into SCIM-compliant resource management, ensuring that all operations follow the SCIM specification while allowing for custom business logic integration.

**Default Option:**
- `StandardResourceProvider::new(storage)` - Recommended for most use cases

**Configuration:**

```rust
use scim_server::providers::StandardResourceProvider;

let provider = StandardResourceProvider::new(storage);
```

**When to implement custom providers:**
- Complex business validation rules
- External system integration (LDAP, Active Directory)
- Custom security requirements (field-level encryption)
- Workflow integration or approval processes

**Custom provider pattern:**
```rust
struct EnterpriseProvider<S: StorageProvider> {
    standard: StandardResourceProvider<S>,
    ldap_client: LdapClient,
}

impl<S: StorageProvider> ResourceProvider for EnterpriseProvider<S> {
    async fn create_resource(&self, resource_type: &str, data: Value, context: &RequestContext) 
        -> Result<VersionedResource, Self::Error> {
        // Custom validation
        self.validate_with_ldap(&data).await?;
        // Delegate to standard provider
        self.standard.create_resource(resource_type, data, context).await
    }
}
```

### Stage 3: SCIM Server Configuration (Protocol Layer)

**Purpose:** The SCIM server represents the protocol layer that orchestrates all SCIM operations and provides the main API surface for your application. It manages resource type registration, schema validation against registered schemas, URL generation for resource references ($ref fields), multi-tenant URL strategies, and coordinates between different resource providers. This layer handles the SCIM protocol lifecycle including resource creation, updates, patches, deletions, and search operations while maintaining protocol compliance and providing proper error responses.

**Creation Options:**

**Option A - Simple (Single tenant, default settings):**
```rust
use scim_server::ScimServer;

let server = ScimServer::new(provider)?;
```

**Option B - Builder Pattern (Recommended for production):**
```rust
use scim_server::{ScimServerBuilder, TenantStrategy};

let server = ScimServerBuilder::new(provider)
    .with_base_url("https://api.company.com")
    .with_tenant_strategy(TenantStrategy::PathBased)
    .with_scim_version("v2")
    .build()?;
```

**Builder Configuration Options:**

- **Base URL**: Root URL for $ref generation
  - `"https://api.company.com"` - Production HTTPS
  - `"http://localhost:8080"` - Development
  - `"mcp://scim"` - AI agent integration

- **Tenant Strategy**: How tenant information appears in URLs
  - `TenantStrategy::SingleTenant` - No tenant in URLs: `/v2/Users/123`
  - `TenantStrategy::Subdomain` - Subdomain: `https://tenant.api.com/v2/Users/123`
  - `TenantStrategy::PathBased` - Path: `/tenant/v2/Users/123`

- **SCIM Version**: Protocol version in URLs (default: "v2")

### Stage 4: Resource Type Registration

**Purpose:** Resource type registration defines what types of resources your SCIM server can handle and which operations are supported for each type. This stage connects SCIM schemas (which define the structure and validation rules) with resource handlers (which provide the processing logic) and declares which SCIM operations (Create, Read, Update, Delete, List, Search) are available for each resource type. Without this registration, the SCIM server cannot process requests for specific resource types, making this a critical configuration step that determines your server's capabilities.

**Process:**
1. Get schema from server's built-in registry
2. Create resource handler from schema
3. Register with supported operations

```rust
use scim_server::{resource_handlers::{create_user_resource_handler, create_group_resource_handler}, ScimOperation};

// Register User resource type
let user_schema = server.get_schema_by_id("urn:ietf:params:scim:schemas:core:2.0:User")?.clone();
let user_handler = create_user_resource_handler(user_schema);
server.register_resource_type("User", user_handler, vec![
    ScimOperation::Create,
    ScimOperation::Read,
    ScimOperation::Update,
    ScimOperation::Delete,
    ScimOperation::List,
])?;

// Register Group resource type
let group_schema = server.get_schema_by_id("urn:ietf:params:scim:schemas:core:2.0:Group")?.clone();
let group_handler = create_group_resource_handler(group_schema);
server.register_resource_type("Group", group_handler, vec![
    ScimOperation::Create,
    ScimOperation::Read,
])?;
```

**Built-in Resource Types:**
- **User** - Individual user accounts with standard SCIM User schema
- **Group** - User collections with standard SCIM Group schema

**Custom Resource Types:**
Create custom schemas and handlers for organization-specific resources like Roles, Permissions, or Devices.

### Stage 5: Request Context Configuration

**Purpose:** Request contexts provide essential tracking and tenant isolation capabilities for every SCIM operation. The RequestContext carries a unique request identifier for logging and debugging purposes, while the optional TenantContext provides tenant isolation in multi-tenant deployments. These contexts flow through every layer of the system, ensuring that operations are properly attributed, logged, and isolated to the correct tenant boundaries. The context system enables audit trails, tenant-specific customizations, and secure multi-tenant operations.

**Context Types:**

**Single-tenant contexts:**
Single-tenant contexts are used when your application serves only one organization or when you don't need tenant isolation. The RequestContext carries only a request ID for operation tracking.

```rust
use scim_server::RequestContext;

// Simple context with custom request ID for tracking
let context = RequestContext::new("request-123".to_string());

// Auto-generated UUID request ID for convenience
let context = RequestContext::with_generated_id();
```

**Multi-tenant contexts:**
Multi-tenant contexts include both request tracking and tenant isolation information. The TenantContext contains the tenant identifier (for resource isolation) and client identifier (for API access tracking).

```rust
use scim_server::{RequestContext, TenantContext};

// Create tenant context with tenant ID and client ID
let tenant = TenantContext::new("tenant-id".to_string(), "client-id".to_string());
let context = RequestContext::with_tenant("request-123".to_string(), tenant);

// Auto-generated request ID with tenant context
let context = RequestContext::with_tenant_generated_id(tenant);
```

The tenant ID determines resource isolation boundaries (ensuring tenants can only access their own resources), while the client ID tracks which API client is making requests (useful for API key management and rate limiting).

### Stage 6: Multi-Tenant Configuration (Optional)

**Purpose:** Multi-tenant configuration enables secure resource isolation between different organizations or customer instances within a single SCIM server deployment. This stage defines tenant-specific permissions (what operations each tenant can perform), resource quotas (maximum users/groups per tenant), isolation levels (how strictly tenants are separated), and tenant-specific customizations. Multi-tenancy is essential for SaaS applications, enterprise deployments, and any scenario where multiple distinct organizations need to share the same SCIM infrastructure while maintaining complete data separation.

**When to skip:** Single-tenant applications where all users share the same namespace.

**When to use:** SaaS applications, enterprise deployments with multiple organizations.

**Tenant Context Configuration:**

```rust
use scim_server::{TenantContext, TenantPermissions, IsolationLevel};

// Configure tenant permissions
let permissions = TenantPermissions {
    can_create: true,
    can_read: true,
    can_update: true,
    can_delete: false,  // Read-only deletion policy
    can_list: true,
    max_users: Some(1000),
    max_groups: Some(100),
};

// Create tenant with isolation settings
let tenant = TenantContext::new("enterprise-corp".to_string(), "client-123".to_string())
    .with_isolation_level(IsolationLevel::Strict)
    .with_permissions(permissions);
```

**Isolation Levels:**
- `IsolationLevel::Strict` - Complete separation, highest security
- `IsolationLevel::Standard` - Normal isolation with some resource sharing
- `IsolationLevel::Shared` - Minimal isolation for development/testing

### Stage 7: Schema Extensions (Optional)

**Purpose:** Schema extensions allow you to add custom attributes, validation rules, and resource types beyond the standard SCIM User and Group schemas. This stage is crucial for organizations that need to store additional information like employee IDs, cost centers, custom roles, or industry-specific data fields. Extensions can be defined at the server level (affecting all tenants) or per-tenant (for multi-tenant deployments), enabling flexible customization while maintaining SCIM protocol compliance. This extensibility ensures your SCIM server can adapt to diverse organizational requirements and integrate with existing identity systems.

**When to use:**
- Industry-specific requirements (healthcare, finance)
- Enterprise attributes (employee ID, cost center)
- Integration with existing systems

**Extension Options:**

**Option A - Custom Schema Files:**
Create JSON schema files with your custom attributes:

```json
{
  "id": "urn:company:params:scim:schemas:extension:2.0:Employee",
  "name": "Employee",
  "attributes": [
    {
      "name": "employeeId",
      "type": "string",
      "required": true,
      "uniqueness": "server"
    }
  ]
}
```

**Option B - Runtime Schema Addition:**
```rust
// Load custom schema and add to registry
let mut schema_registry = server.schema_registry_mut();
schema_registry.add_schema(custom_schema)?;
```

### Stage 8: Request/Response Handler Integration

**Purpose:** The final integration stage connects your configured SCIM server with HTTP frameworks and client applications. This stage involves creating HTTP route handlers that translate REST API requests into SCIM server operations, implementing authentication middleware, handling request/response serialization, and managing error responses. This is where your SCIM server becomes accessible to client applications, identity providers, and administrative tools. The request/response handlers serve as the bridge between the HTTP protocol layer and your SCIM server's internal operations.

**Integration Options:**

**HTTP Framework Integration:**
```rust
// Example with Axum web framework
use axum::{extract::Extension, http::StatusCode, response::Json, routing::post, Router};

async fn create_user_handler(
    Extension(server): Extension<ScimServer<YourProvider>>,
    Json(user_data): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let context = RequestContext::with_generated_id();
    
    match server.create_resource("User", user_data, &context).await {
        Ok(user) => Ok(Json(user.into_json())),
        Err(_) => Err(StatusCode::BAD_REQUEST),
    }
}

let app = Router::new()
    .route("/scim/v2/Users", post(create_user_handler))
    .layer(Extension(server));
```

**Authentication Integration:**
```rust
// Middleware for JWT/API key validation
async fn auth_middleware(request: Request, next: Next) -> Response {
    // Extract and validate authentication token
    let auth_header = request.headers().get("authorization");
    if let Some(token) = extract_and_validate_token(auth_header) {
        // Add authenticated context to request
        request.extensions_mut().insert(AuthenticatedUser::from(token));
        next.run(request).await
    } else {
        Response::builder()
            .status(401)
            .body("Unauthorized".into())
            .unwrap()
    }
}
```

**What you need to implement:**
- HTTP route handlers for each SCIM endpoint (Users, Groups, etc.)
- Authentication and authorization middleware
- Request validation and error handling
- Response formatting and status code management
- CORS policies for web applications
- Rate limiting and request throttling
- Audit logging and monitoring integration

## Complete Configuration Example

Here's a minimal but complete configuration for a production-ready SCIM server:

```rust
use scim_server::{
    ScimServerBuilder, TenantStrategy, RequestContext,
    providers::StandardResourceProvider,
    storage::InMemoryStorage,
    resource_handlers::{create_user_resource_handler, create_group_resource_handler},
    ScimOperation,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Stage 1: Storage Provider
    let storage = InMemoryStorage::new();
    
    // Stage 2: Resource Provider  
    let provider = StandardResourceProvider::new(storage);
    
    // Stage 3: SCIM Server Configuration
    let mut server = ScimServerBuilder::new(provider)
        .with_base_url("https://api.company.com")
        .with_tenant_strategy(TenantStrategy::PathBased)
        .build()?;
    
    // Stage 4: Resource Type Registration
    let user_schema = server.get_schema_by_id("urn:ietf:params:scim:schemas:core:2.0:User")?.clone();
    let user_handler = create_user_resource_handler(user_schema);
    server.register_resource_type("User", user_handler, vec![
        ScimOperation::Create,
        ScimOperation::Read,
        ScimOperation::Update,
        ScimOperation::Delete,
    ])?;
    
    // Stage 5: Request Context (configured per operation)
    let context = RequestContext::with_generated_id();
    
    // Stage 6: Ready for request/response handler integration
    // server is now ready to handle SCIM operations through HTTP handlers
    // Next: create HTTP routes and authentication middleware
    
    Ok(())
}
```

## Next Steps

With your SCIM server configured, you'll need to:

1. **Request/Response Handlers** - Create HTTP route handlers for SCIM endpoints
2. **Authentication Middleware** - Implement JWT, OAuth2, or API key validation
3. **Error Handling** - Set up structured SCIM-compliant error responses
4. **CORS and Security** - Configure cross-origin policies and security headers
5. **Logging and Monitoring** - Set up request/response logging and metrics
6. **Testing** - Create integration tests for your complete SCIM API

See the [examples directory](https://github.com/pukeko37/scim-server/tree/main/examples) for complete integration examples with popular web frameworks.