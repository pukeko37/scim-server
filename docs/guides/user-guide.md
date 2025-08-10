# SCIM Server User Guide

A comprehensive guide to using the SCIM Server crate for building production-ready SCIM 2.0 compliant identity management systems.

## Table of Contents

- [Introduction](#introduction)
- [Core Concepts](#core-concepts)
- [Getting Started](#getting-started)
- [Resource Management](#resource-management)
- [Multi-tenancy](#multi-tenancy)
- [Schema Validation](#schema-validation)
- [Storage Providers](#storage-providers)
- [Error Handling](#error-handling)
- [Performance Optimization](#performance-optimization)
- [Production Deployment](#production-deployment)
- [Troubleshooting](#troubleshooting)

## Introduction

The SCIM Server crate provides a complete, type-safe implementation of SCIM (System for Cross-domain Identity Management) 2.0 specification. It's designed for building identity management systems that need to:

- **Manage users and groups** with standardized schemas
- **Support multi-tenant architectures** for SaaS applications  
- **Integrate with existing databases** through custom providers
- **Ensure data consistency** with comprehensive validation
- **Scale to production workloads** with async, high-performance operations

## Core Concepts

### SCIM Resources

SCIM resources represent identity objects like users and groups. Each resource has:

- **Core attributes**: id, externalId, schemas, meta
- **Resource-specific attributes**: userName for users, displayName for groups
- **Multi-valued attributes**: emails, phoneNumbers, addresses
- **Extension attributes**: Custom attributes for specific needs

### Resource Providers

Resource providers handle the actual storage and retrieval of SCIM resources. The crate provides:

- **InMemoryProvider**: For testing and development
- **Custom providers**: Implement the `ResourceProvider` trait for databases, APIs, etc.

### Request Context

Every SCIM operation includes a `RequestContext` that provides:

- **Operation tracking**: Unique ID for each operation
- **Tenant isolation**: Multi-tenant context when needed
- **Audit trail**: Information for logging and monitoring

### Value Objects

Type-safe wrappers around SCIM attributes that enforce validation:

- **Compile-time safety**: Invalid values cannot be constructed
- **Runtime validation**: Clear error messages for validation failures
- **Immutable design**: Values cannot be accidentally modified

## Getting Started

### Basic Server Setup

Here's a minimal SCIM server that handles user resources:

```rust
use scim_server::{
    ScimServer, ResourceProvider, Resource, RequestContext,
    providers::InMemoryProvider, create_user_resource_handler
};
use serde_json::json;
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging for development
    env_logger::init();
    
    // Create an in-memory provider
    let provider = InMemoryProvider::new();
    let mut server = ScimServer::new(provider);
    
    // Register the built-in user resource handler
    server.register_resource_handler("User", create_user_resource_handler());
    
    // Create a sample user
    let user_data = json!({
        "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
        "userName": "john.doe@example.com",
        "name": {
            "givenName": "John",
            "familyName": "Doe",
            "formatted": "John Doe"
        },
        "emails": [{
            "value": "john.doe@example.com",
            "type": "work",
            "primary": true
        }],
        "active": true
    });
    
    let context = RequestContext::with_generated_id();
    let user = server.create_resource("User", user_data, &context).await?;
    
    println!("Created user: {}", user.id().unwrap().as_str());
    
    Ok(())
+}
```

### Adding Web Framework Integration

Integrate with Axum for HTTP endpoints:

```rust
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Json,
    routing::{get, post, put, delete},
    Router,
};
use std::sync::Arc;

type SharedServer = Arc<ScimServer<InMemoryProvider>>;

async fn create_user(
    State(server): State<SharedServer>,
    Json(payload): Json<Value>,
) -> Result<Json<Value>, StatusCode> {
    let context = RequestContext::with_generated_id();
    
    match server.create_resource("User", payload, &context).await {
        Ok(resource) => {
            match resource.to_json() {
                Ok(json) => Ok(Json(json)),
                Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
            }
        }
        Err(_) => Err(StatusCode::BAD_REQUEST),
    }
}

async fn get_user(
    State(server): State<SharedServer>,
    Path(id): Path<String>,
) -> Result<Json<Value>, StatusCode> {
    let context = RequestContext::with_generated_id();
    
    match server.get_resource("User", &id, &context).await {
        Ok(Some(resource)) => {
            match resource.to_json() {
                Ok(json) => Ok(Json(json)),
                Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
            }
        }
        Ok(None) => Err(StatusCode::NOT_FOUND),
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

#[tokio::main]
async fn main() {
    let provider = InMemoryProvider::new();
    let mut server = ScimServer::new(provider);
    server.register_resource_handler("User", create_user_resource_handler());
    
    let server = Arc::new(server);
    
    let app = Router::new()
        .route("/Users", post(create_user))
        .route("/Users/:id", get(get_user))
        .with_state(server);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
```

## Resource Management

### Creating Resources

#### From JSON (Recommended)

```rust
use scim_server::resource::Resource;
use serde_json::json;

fn create_from_json() -> Result<(), Box<dyn std::error::Error>> {
    let user_data = json!({
        "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
        "userName": "jane.smith@example.com",
        "name": {
            "givenName": "Jane",
            "familyName": "Smith"
        },
        "emails": [{
            "value": "jane.smith@example.com",
            "type": "work",
            "primary": true
        }],
        "phoneNumbers": [{
            "value": "555-1234",
            "type": "work"
        }],
        "addresses": [{
            "streetAddress": "123 Main St",
            "locality": "Anytown",
            "region": "CA",
            "postalCode": "12345",
            "country": "US",
            "type": "work",
            "primary": true
        }],
        "active": true
    });
    
    let resource = Resource::from_json("User".to_string(), user_data)?;
    Ok(())
}
```

#### Using Builder Pattern (Type-Safe)

```rust
use scim_server::resource::{ResourceBuilder, value_objects::*};

fn create_with_builder() -> Result<(), Box<dyn std::error::Error>> {
    // Create individual components
    let username = UserName::new("jane.smith@example.com".to_string())?;
    let name = Name::new_simple("Jane".to_string(), "Smith".to_string())?;
    
    // Create multi-valued attributes
    let work_email = EmailAddress::new_simple("jane.smith@example.com".to_string())?;
    let emails = MultiValuedAttribute::single_primary(work_email);
    
    let work_phone = PhoneNumber::new_simple("555-1234".to_string())?;
    let phones = MultiValuedAttribute::single(work_phone);
    
    // Build the resource
    let resource = ResourceBuilder::new("User")
        .user_name(username)?
        .name(name)?
        .emails(emails)?
        .phone_numbers(phones)?
        .active(true)
        .build()?;
    
    println!("Created user: {:?}", resource.user_name());
    Ok(())
}
```

### Reading Resources

```rust
use scim_server::{ResourceProvider, RequestContext};

async fn read_operations<P: ResourceProvider>(
    provider: &P,
    context: &RequestContext
) -> Result<(), P::Error> {
    // Get specific resource
    if let Some(user) = provider.get_resource("User", "12345", context).await? {
        println!("Found user: {}", user.user_name().unwrap().as_str());
        
        // Access attributes safely
        if let Some(name) = user.name() {
            println!("Name: {}", name.formatted().unwrap_or("N/A"));
        }
        
        if let Some(emails) = user.emails() {
            println!("Email count: {}", emails.len());
            if let Some(primary_email) = emails.primary() {
                println!("Primary email: {}", primary_email.value());
            }
        }
    }
    
    // List all resources
    let all_users = provider.list_resources("User", None, context).await?;
    println!("Total users: {}", all_users.len());
    
    // Find by attribute
    let user_by_email = provider.find_resource_by_attribute(
        "User",
        "emails.value", 
        &json!("john@example.com"),
        context
    ).await?;
    
    Ok(())
}
```

### Updating Resources

```rust
async fn update_operations<P: ResourceProvider>(
    provider: &P,
    context: &RequestContext
) -> Result<(), P::Error> {
    let user_id = "12345";
    
    // Full update (replace)
    let update_data = json!({
        "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
        "userName": "updated.user@example.com",
        "name": {
            "givenName": "Updated",
            "familyName": "User"
        },
        "active": false
    });
    
    let updated = provider.update_resource("User", user_id, update_data, context).await?;
    println!("Updated user: {}", updated.user_name().unwrap().as_str());
    
    Ok(())
}
```

### Deleting Resources

```rust
async fn delete_operations<P: ResourceProvider>(
    provider: &P,
    context: &RequestContext
) -> Result<(), P::Error> {
    let user_id = "12345";
    
    // Delete the resource
    provider.delete_resource("User", user_id, context).await?;
    println!("Deleted user: {}", user_id);
    
    // Verify deletion
    let deleted_user = provider.get_resource("User", user_id, context).await?;
    assert!(deleted_user.is_none());
    
    Ok(())
}
```

## Multi-tenancy

### Setting Up Multi-tenant Context

```rust
use scim_server::multi_tenant::{TenantContext, StaticTenantResolver};
use scim_server::resource::RequestContext;

fn setup_multi_tenancy() -> Result<(), Box<dyn std::error::Error>> {
    // Create tenant resolver
    let resolver = StaticTenantResolver::builder()
        .add_tenant("company-a", "client-123")
        .add_tenant("company-b", "client-456")
        .build();
    
    // Create tenant-specific contexts
    let tenant_a = TenantContext::new("company-a".to_string(), "client-123".to_string());
    let context_a = RequestContext::with_tenant_generated_id(tenant_a);
    
    let tenant_b = TenantContext::new("company-b".to_string(), "client-456".to_string());
    let context_b = RequestContext::with_tenant_generated_id(tenant_b);
    
    // Operations are now isolated by tenant
    println!("Tenant A context: {:?}", context_a.tenant_context());
    println!("Tenant B context: {:?}", context_b.tenant_context());
    
    Ok(())
}
```

### Tenant-Isolated Operations

```rust
async fn tenant_operations<P: ResourceProvider>(
    provider: &P
) -> Result<(), P::Error> {
    // Create contexts for different tenants
    let tenant_a = TenantContext::new("tenant-a".to_string(), "client-a".to_string());
    let context_a = RequestContext::with_tenant_generated_id(tenant_a);
    
    let tenant_b = TenantContext::new("tenant-b".to_string(), "client-b".to_string());
    let context_b = RequestContext::with_tenant_generated_id(tenant_b);
    
    // Create users in different tenants
    let user_data = json!({
        "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
        "userName": "user@example.com"
    });
    
    let user_a = provider.create_resource("User", user_data.clone(), &context_a).await?;
    let user_b = provider.create_resource("User", user_data, &context_b).await?;
    
    // Users are isolated by tenant
    let tenant_a_users = provider.list_resources("User", None, &context_a).await?;
    let tenant_b_users = provider.list_resources("User", None, &context_b).await?;
    
    println!("Tenant A users: {}", tenant_a_users.len());
    println!("Tenant B users: {}", tenant_b_users.len());
    
    Ok(())
}
```

## Schema Validation

### Understanding SCIM Schemas

SCIM resources are validated against schemas that define:

- **Required attributes**: Must be present
- **Optional attributes**: May be present
- **Data types**: String, boolean, integer, complex, multi-valued
- **Validation rules**: Format, uniqueness, mutability

### Built-in Schema Validation

```rust
use scim_server::schema::{SchemaRegistry, OperationContext};
use serde_json::json;

async fn schema_validation_example() -> Result<(), Box<dyn std::error::Error>> {
    let registry = SchemaRegistry::new()?;
    
    // Valid user - passes validation
    let valid_user = json!({
        "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
        "userName": "valid.user@example.com",
        "name": {
            "givenName": "Valid",
            "familyName": "User"
        }
    });
    
    registry.validate_json_resource_with_context(
        "User", 
        &valid_user, 
        OperationContext::Create
    )?;
    println!("User is valid!");
    
    // Invalid user - fails validation
    let invalid_user = json!({
        "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
        // Missing required userName
        "name": {
            "givenName": "Invalid",
            "familyName": "User"
        }
    });
    
    match registry.validate_json_resource_with_context(
        "User", 
        &invalid_user, 
        OperationContext::Create
    ) {
        Ok(()) => println!("Unexpected: invalid user passed validation"),
        Err(e) => println!("Expected validation error: {}", e),
    }
    
    Ok(())
}
```

### Custom Validation Rules

```rust
use scim_server::resource::Resource;
use scim_server::error::{ValidationError, ValidationResult};

fn custom_business_validation(resource: &Resource) -> ValidationResult<()> {
    // Example: Enforce company email domain
    if let Some(emails) = resource.emails() {
        for email in emails.iter() {
            if !email.value().ends_with("@company.com") {
                return Err(ValidationError::custom(
                    "Email must be from company.com domain"
                ));
            }
        }
    }
    
    // Example: Require display name for active users
    if resource.active().unwrap_or(false) && resource.display_name().is_none() {
        return Err(ValidationError::custom(
            "Active users must have a display name"
        ));
    }
    
    Ok(())
}
```

## Storage Providers

### Using the In-Memory Provider

Perfect for development, testing, and small applications:

```rust
use scim_server::providers::InMemoryProvider;

async fn in_memory_example() -> Result<(), Box<dyn std::error::Error>> {
    let provider = InMemoryProvider::new();
    let context = RequestContext::with_generated_id();
    
    // Create a user
    let user_data = json!({
        "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
        "userName": "memory.user@example.com"
    });
    
    let user = provider.create_resource("User", user_data, &context).await
        .map_err(|e| format!("Create failed: {:?}", e))?;
    
    // Retrieve the user
    let retrieved = provider.get_resource("User", user.id().unwrap().as_str(), &context).await
        .map_err(|e| format!("Get failed: {:?}", e))?;
    
    assert!(retrieved.is_some());
    println!("In-memory storage working correctly!");
    
    Ok(())
}
```

### Implementing Custom Providers

For production use, implement the `ResourceProvider` trait:

```rust
use scim_server::{ResourceProvider, Resource, RequestContext, ListQuery};
use serde_json::Value;
use std::future::Future;
use async_trait::async_trait;

// Example: Database-backed provider
pub struct DatabaseProvider {
    connection_pool: sqlx::PgPool,
}

impl DatabaseProvider {
    pub fn new(database_url: &str) -> Result<Self, sqlx::Error> {
        let pool = sqlx::PgPool::connect(database_url).await?;
        Ok(Self { connection_pool: pool })
    }
}

#[derive(Debug, thiserror::Error)]
enum DatabaseError {
    #[error("Database error: {0}")]
    Sqlx(#[from] sqlx::Error),
    #[error("Validation error: {0}")]
    Validation(#[from] ValidationError),
    #[error("Resource not found")]
    NotFound,
}

impl ResourceProvider for DatabaseProvider {
    type Error = DatabaseError;

    fn create_resource(
        &self, 
        resource_type: &str, 
        data: Value, 
        context: &RequestContext
    ) -> impl Future<Output = Result<Resource, Self::Error>> + Send {
        async move {
            // Validate the resource
            let resource = Resource::from_json(resource_type.to_string(), data)?;
            
            // Store in database
            let query = "INSERT INTO scim_resources (id, resource_type, data, tenant_id) VALUES ($1, $2, $3, $4)";
            sqlx::query(query)
                .bind(resource.id().unwrap().as_str())
                .bind(resource_type)
                .bind(serde_json::to_string(&resource.to_json()?)?)
                .bind(context.tenant_context().map(|t| t.tenant_id()))
                .execute(&self.connection_pool)
                .await?;
            
            Ok(resource)
        }
    }

    fn get_resource(
        &self, 
        resource_type: &str, 
        id: &str, 
        context: &RequestContext
    ) -> impl Future<Output = Result<Option<Resource>, Self::Error>> + Send {
        async move {
            let query = "SELECT data FROM scim_resources WHERE id = $1 AND resource_type = $2 AND tenant_id = $3";
            let row: Option<(String,)> = sqlx::query_as(query)
                .bind(id)
                .bind(resource_type)
                .bind(context.tenant_context().map(|t| t.tenant_id()))
                .fetch_optional(&self.connection_pool)
                .await?;
            
            match row {
                Some((data,)) => {
                    let json: Value = serde_json::from_str(&data)?;
                    let resource = Resource::from_json(resource_type.to_string(), json)?;
                    Ok(Some(resource))
                }
                None => Ok(None),
            }
        }
    }

    // Implement other required methods...
}
```

## Error Handling

### Understanding Error Types

The SCIM server uses structured error handling with specific error types:

```rust
use scim_server::error::{ValidationError, ValidationResult};

fn error_handling_examples() {
    // ValidationError covers all input validation scenarios
    match ResourceId::new("invalid-uuid".to_string()) {
        Ok(id) => println!("Valid ID: {}", id.as_str()),
        Err(ValidationError::InvalidResourceId { value }) => {
            println!("Invalid resource ID format: {}", value);
        }
        Err(ValidationError::MissingRequiredAttribute { attribute }) => {
            println!("Missing required attribute: {}", attribute);
        }
        Err(ValidationError::InvalidAttributeValue { attribute, value }) => {
            println!("Invalid value '{}' for attribute '{}'", value, attribute);
        }
        Err(e) => println!("Other validation error: {}", e),
    }
}
```

### Error Recovery Patterns

```rust
use scim_server::error::ValidationError;

fn error_recovery_example() -> ValidationResult<Resource> {
    // Try to create resource, handle validation errors gracefully
    match Resource::from_json("User".to_string(), user_data) {
        Ok(resource) => Ok(resource),
        Err(ValidationError::MissingRequiredAttribute { attribute }) => {
            println!("Warning: Missing required attribute: {}", attribute);
            // Provide default value or prompt user
            create_resource_with_defaults()
        }
        Err(ValidationError::InvalidAttributeValue { attribute, value }) => {
            println!("Invalid value '{}' for '{}', using default", value, attribute);
            create_resource_with_corrected_value(attribute, value)
        }
        Err(e) => {
            // Unrecoverable errors
            Err(e)
        }
    }
}
```

### Provider Error Integration

```rust
// Custom provider errors can wrap ValidationError
#[derive(Debug, thiserror::Error)]
pub enum MyProviderError {
    #[error("Database connection failed")]
    DatabaseConnection,
    
    #[error("Resource validation failed: {0}")]
    Validation(#[from] ValidationError),
    
    #[error("Tenant not authorized")]
    Unauthorized,
}

// This allows seamless error propagation
async fn provider_operation() -> Result<Resource, MyProviderError> {
    let resource = Resource::from_json(...)?;  // ValidationError auto-converts
    // ... database operations that might fail
    Ok(resource)
}
```

## Performance Optimization

### Efficient Resource Creation

```rust
// Prefer from_json for external data
let resource = Resource::from_json(resource_type, json_data)?;

// Use builder for programmatic construction
let resource = ResourceBuilder::new("User")
    .user_name(username)?
    .build()?;
```

### Batch Operations

```rust
async fn batch_operations<P: ResourceProvider>(
    provider: &P,
    context: &RequestContext
) -> Result<Vec<Resource>, P::Error> {
    let users_data = vec![
        json!({"userName": "user1@example.com"}),
        json!({"userName": "user2@example.com"}),
        json!({"userName": "user3@example.com"}),
    ];
    
    // Process in parallel for performance
    let futures: Vec<_> = users_data
        .into_iter()
        .map(|data| provider.create_resource("User", data, context))
        .collect();
    
    // Wait for all to complete
    let results = futures::future::try_join_all(futures).await?;
    
    println!("Created {} users in batch", results.len());
    Ok(results)
}
```

### Memory-Efficient Listing

```rust
use scim_server::providers::ListQuery;

async fn efficient_listing<P: ResourceProvider>(
    provider: &P,
    context: &RequestContext
) -> Result<(), P::Error> {
    let page_size = 100;
    let mut start_index = 1;
    
    loop {
        let query = ListQuery::new()
            .with_start_index(start_index)
            .with_count(page_size);
        
        let page = provider.list_resources("User", Some(&query), context).await?;
        
        if page.is_empty() {
            break;
        }
        
        // Process page
        for user in page {
            println!("Processing user: {}", user.user_name().unwrap().as_str());
        }
        
        start_index += page_size;
    }
    
    Ok(())
}
```

## Production Deployment

### Logging Configuration

```rust
use log::{info, error, debug};

fn setup_production_logging() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
        .format_timestamp_secs()
        .format_module_path(false)
        .format_target(false)
        .init();
    
    info!("SCIM server starting up");
}
```

### Health Checks

```rust
use scim_server::{ScimServer, ResourceProvider};

impl<P: ResourceProvider> ScimServer<P> {
    pub async fn health_check(&self) -> bool {
        // Verify provider connectivity
        let context = RequestContext::with_generated_id();
        
        match self.provider.list_resources("User", None, &context).await {
            Ok(_) => {
                debug!("Health check passed");
                true
            }
            Err(e) => {
                error!("Health check failed: {:?}", e);
                false
            }
        }
    }
}
```

### Configuration Management

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub struct ScimServerConfig {
    pub database_url: String,
    pub bind_address: String,
    pub port: u16,
    pub log_level: String,
    pub max_connections: u32,
    pub enable_multi_tenancy: bool,
}

impl ScimServerConfig {
    pub fn from_env() -> Result<Self, config::ConfigError> {
        let mut cfg = config::Config::builder()
            .add_source(config::Environment::with_prefix("SCIM"))
            .build()?;
        
        cfg.try_deserialize()
    }
}
```

## Troubleshooting

### Common Issues

#### "Resource validation failed"
- **Cause**: Input JSON doesn't match SCIM schema requirements
- **Solution**: Check required attributes and data types
- **Debug**: Use `ValidationError` details to identify specific issues

#### "Tenant not found"
- **Cause**: Request context references non-existent tenant
- **Solution**: Verify tenant resolver configuration
- **Debug**: Log tenant resolution attempts

#### "Provider operation failed"  
- **Cause**: Storage backend is unavailable or misconfigured
- **Solution**: Check database connections, credentials, and network
- **Debug**: Enable provider-specific logging

### Debugging Techniques

#### Enable Comprehensive Logging

```rust
fn enable_debug_logging() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("debug"))
        .format_timestamp_millis()
        .format_module_path(true)
        .format_target(true)
        .init();
}
```

#### Validate Resources Manually

```rust
use scim_server::schema::SchemaRegistry;

async fn debug_validation(resource_data: &Value) -> Result<(), Box<dyn std::error::Error>> {
    let registry = SchemaRegistry::new()?;
    
    // Try validation step by step
    println!("Validating resource: {}", serde_json::to_string_pretty(resource_data)?);
    
    match registry.validate_json_resource_with_context("User", resource_data, OperationContext::Create) {
        Ok(()) => println!("✅ Validation passed"),
        Err(e) => {
            println!("❌ Validation failed: {}", e);
            println!("Error details: {:?}", e);
        }
    }
    
    Ok(())
}
```

#### Test Provider Operations

```rust
async fn debug_provider<P: ResourceProvider>(provider: &P) -> Result<(), P::Error> {
    let context = RequestContext::with_generated_id();
    
    // Test basic connectivity
    println!("Testing provider connectivity...");
    let empty_list = provider.list_resources("User", None, &context).await?;
    println!("✅ Provider connected, found {} users", empty_list.len());
    
    // Test create operation
    println!("Testing resource creation...");
    let test_data = json!({
        "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
        "userName": "test.user@example.com"
    });
    
    let created = provider.create_resource("User", test_data, &context).await?;
    println!("✅ Resource created with ID: {}", created.id().unwrap().as_str());
    
    // Test retrieval
    println!("Testing resource retrieval...");
    let retrieved = provider.get_resource("User", created.id().unwrap().as_str(), &context).await?;
    match retrieved {
        Some(_) => println!("✅ Resource retrieved successfully"),
        None => println!("❌ Resource not found after creation"),
    }
    
    Ok(())
}
```

### Performance Monitoring

```rust
use std::time::Instant;

async fn monitor_operation_performance<P: ResourceProvider>(
    provider: &P,
    context: &RequestContext
) -> Result<(), P::Error> {
    let start = Instant::now();
    
    let user_data = json!({
        "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
        "userName": "perf.test@example.com"
    });
    
    let resource = provider.create_resource("User", user_data, context).await?;
    
    let duration = start.elapsed();
    println!("Resource creation took: {:?}", duration);
    
    if duration.as_millis() > 100 {
        println!("⚠️  Slow operation detected");
    }
    
    Ok(())
}
```

## Best Practices

### Resource Design

1. **Use value objects** for type safety and validation
2. **Validate early** at resource construction time
3. **Handle errors gracefully** with specific error types
4. **Use builders** for complex resource construction
5. **Leverage schemas** for automatic validation

### Provider Implementation

1. **Implement all methods** of the ResourceProvider trait
2. **Handle tenant isolation** properly in multi-tenant scenarios
3. **Use connection pooling** for database providers
4. **Implement proper error mapping** from storage errors
5. **Add comprehensive logging** for debugging

### Performance

1. **Use async operations** throughout your application
2. **Batch