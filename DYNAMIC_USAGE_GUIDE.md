# Dynamic SCIM Server Usage Guide

This guide shows you how to use the new dynamic SCIM server to build flexible, schema-driven identity management systems.

## Quick Start

### 1. Basic Setup

```rust
use scim_server::{
    ScimServer, ResourceProvider, Resource,
    RequestContext, ScimOperation, create_user_resource_handler
};

// 1. Create your provider
let provider = MyProvider::new();

// 2. Create the dynamic server
let mut server = ScimServer::new(provider)?;

// 3. Register resource types
let user_schema = load_schema_from_file("User.json")?;
let user_handler = create_user_resource_handler(user_schema);

server.register_resource_type(
    "User",
    user_handler,
    vec![ScimOperation::Create, ScimOperation::Read, ScimOperation::Update, ScimOperation::Delete, ScimOperation::List, ScimOperation::Search],
)?;

// 4. Use the server
let context = RequestContext::new("request-123".to_string());
let user = server.create_resource("User", user_data, &context).await?;
```

## Creating Custom Resource Types

### Step 1: Define Your Schema

Create a schema file or define it programmatically:

```rust
use scim_server::{Schema, AttributeDefinition, AttributeType, Mutability};

let project_schema = Schema {
    id: "urn:company:scim:schemas:Project".to_string(),
    name: "Project".to_string(),
    description: "Project Management Resource".to_string(),
    attributes: vec![
        AttributeDefinition {
            name: "id".to_string(),
            data_type: AttributeType::String,
            required: false,
            mutability: Mutability::ReadOnly,
            ..Default::default()
        },
        AttributeDefinition {
            name: "name".to_string(),
            data_type: AttributeType::String,
            required: true,
            mutability: Mutability::ReadWrite,
            ..Default::default()
        },
        AttributeDefinition {
            name: "status".to_string(),
            data_type: AttributeType::String,
            required: false,
            mutability: Mutability::ReadWrite,
            canonical_values: vec!["active".to_string(), "inactive".to_string(), "archived".to_string()],
            ..Default::default()
        },
        AttributeDefinition {
            name: "members".to_string(),
            data_type: AttributeType::Complex,
            multi_valued: true,
            required: false,
            mutability: Mutability::ReadWrite,
            ..Default::default()
        },
    ],
};
```

### Step 2: Create a Resource Handler

```rust
use scim_server::{SchemaResourceBuilder, DatabaseMapper};
use std::collections::HashMap;

fn create_project_resource_handler(schema: Schema) -> ResourceHandler {
    SchemaResourceBuilder::new(schema)
        // Basic attribute handlers
        .with_getter("name", |data| {
            data.get("name")?.as_str().map(|s| Value::String(s.to_string()))
        })
        .with_setter("name", |data, value| {
            if let Some(obj) = data.as_object_mut() {
                obj.insert("name".to_string(), value);
            }
            Ok(())
        })
        
        // Status with validation
        .with_setter("status", |data, value| {
            let valid_statuses = ["active", "inactive", "archived"];
            if let Some(status) = value.as_str() {
                if !valid_statuses.contains(&status) {
                    return Err(ScimError::invalid_request(
                        format!("Invalid status: {}", status)
                    ));
                }
            }
            if let Some(obj) = data.as_object_mut() {
                obj.insert("status".to_string(), value);
            }
            Ok(())
        })
        
        // Custom business logic methods
        .with_custom_method("get_project_name", |resource| {
            Ok(resource.get_attribute_dynamic("name").unwrap_or(Value::Null))
        })
        
        .with_custom_method("is_active", |resource| {
            let status = resource.get_attribute_dynamic("status")
                .and_then(|v| v.as_str())
                .unwrap_or("active");
            Ok(Value::Bool(status == "active"))
        })
        
        .with_custom_method("add_member", |resource| {
            // In practice, this would take parameters
            // This is just a demonstration
            Ok(Value::Bool(true))
        })
        
        .with_custom_method("get_member_count", |resource| {
            let count = resource.get_attribute_dynamic("members")
                .and_then(|v| v.as_array())
                .map(|arr| arr.len())
                .unwrap_or(0);
            Ok(Value::Number(count.into()))
        })
        
        // Database mapping
        .with_database_mapping("projects", {
            let mut mappings = HashMap::new();
            mappings.insert("name".to_string(), "project_name".to_string());
            mappings.insert("status".to_string(), "project_status".to_string());
            mappings.insert("members".to_string(), "member_data".to_string());
            mappings.insert("id".to_string(), "project_id".to_string());
            mappings
        })
        
        .build()
}
```

### Step 3: Register the Resource Type

```rust
let project_handler = create_project_resource_handler(project_schema);

server.register_resource_type(
    "Project",
    project_handler,
    vec![
        ScimOperation::Create,
        ScimOperation::Read,
        ScimOperation::Update,
        ScimOperation::Delete,
        ScimOperation::List,
        // Note: Search not included - operation restrictions
    ],
)?;
```

## Implementing the Provider

### Basic Provider Implementation

```rust
use async_trait::async_trait;
use scim_server::{DynamicResourceProvider, DynamicResource, RequestContext, ListQuery};

struct MyDynamicProvider {
    // Your storage implementation
    database: DatabaseConnection,
}

#[derive(Debug, thiserror::Error)]
#[error("Provider error: {message}")]
struct MyProviderError {
    message: String,
}

#[async_trait]
impl DynamicResourceProvider for MyDynamicProvider {
    type Error = MyProviderError;

    async fn create_resource(
        &self,
        resource_type: &str,
        mut resource: DynamicResource,
        context: &RequestContext,
    ) -> Result<DynamicResource, Self::Error> {
        // Generate ID
        let id = self.generate_id().await?;
        resource.set_attribute_dynamic("id", Value::String(id.clone()))?;
        
        // Convert to database format using the registered mapper
        let db_data = resource.to_implementation_schema(0)?;
        
        // Store in database
        match resource_type {
            "User" => self.database.insert_user(db_data).await?,
            "Project" => self.database.insert_project(db_data).await?,
            _ => return Err(MyProviderError { message: format!("Unsupported resource type: {}", resource_type) }),
        }
        
        // Add metadata
        if let Ok(meta) = resource.call_custom_method("add_metadata") {
            resource.set_attribute_dynamic("meta", meta)?;
        }
        
        Ok(resource)
    }

    async fn get_resource(
        &self,
        resource_type: &str,
        id: &str,
        _context: &RequestContext,
    ) -> Result<Option<DynamicResource>, Self::Error> {
        let db_data = match resource_type {
            "User" => self.database.get_user(id).await?,
            "Project" => self.database.get_project(id).await?,
            _ => return Ok(None),
        };
        
        if let Some(data) = db_data {
            // Get the handler for this resource type
            let handler = self.get_handler(resource_type)?;
            
            // Create resource from database data
            let mut resource = DynamicResource::new(resource_type.to_string(), Value::Null, handler);
            
            // Convert from database format
            resource.from_implementation_schema(&data, 0)?;
            
            Ok(Some(resource))
        } else {
            Ok(None)
        }
    }

    // ... implement other methods similarly
}
```

## Advanced Usage Patterns

### 1. Complex Attribute Handling

```rust
.with_transformer("emails", |data, operation| {
    match operation {
        "get_primary" => {
            data.get("emails")
                .and_then(|v| v.as_array())
                .and_then(|arr| {
                    arr.iter().find(|email| {
                        email.get("primary").and_then(|p| p.as_bool()).unwrap_or(false)
                    })
                })
                .and_then(|email| email.get("value"))
                .cloned()
        }
        "get_work_emails" => {
            let work_emails: Vec<Value> = data.get("emails")
                .and_then(|v| v.as_array())
                .unwrap_or(&vec![])
                .iter()
                .filter(|email| {
                    email.get("type").and_then(|t| t.as_str()) == Some("work")
                })
                .cloned()
                .collect();
            Some(Value::Array(work_emails))
        }
        _ => None
    }
})
```

### 2. Multi-Schema Resources

```rust
// Register extension schema
let enterprise_user_schema = load_schema_from_file("EnterpriseUser.json")?;
let enterprise_handler = create_enterprise_user_handler(enterprise_user_schema);

server.register_resource_type(
    "EnterpriseUser",
    enterprise_handler,
    vec![ScimOperation::Create, ScimOperation::Read, ScimOperation::Update],
)?;

// Handle resources with multiple schemas
.with_custom_method("get_schemas", |resource| {
    let mut schemas = vec![
        "urn:ietf:params:scim:schemas:core:2.0:User".to_string()
    ];
    
    if resource.get_attribute_dynamic("employeeNumber").is_some() {
        schemas.push("urn:ietf:params:scim:schemas:extension:enterprise:2.0:User".to_string());
    }
    
    Ok(Value::Array(schemas.into_iter().map(Value::String).collect()))
})
```

### 3. Custom Validation

```rust
.with_setter("email", |data, value| {
    if let Some(email_str) = value.as_str() {
        if !email_str.contains('@') || !email_str.contains('.') {
            return Err(ScimError::invalid_request("Invalid email format"));
        }
    }
    
    if let Some(obj) = data.as_object_mut() {
        obj.insert("email".to_string(), value);
    }
    Ok(())
})
```

### 4. Audit and Logging

```rust
.with_custom_method("add_metadata", |resource| {
    let now = chrono::Utc::now().to_rfc3339();
    
    // Log the operation
    log::info!("Resource {} accessed at {}", 
        resource.get_attribute_dynamic("id")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown"),
        now
    );
    
    let meta = json!({
        "resourceType": resource.resource_type,
        "created": now,
        "lastModified": now,
        "location": format!("/scim/v2/{}s/{}", 
            resource.resource_type,
            resource.get_attribute_dynamic("id")
                .and_then(|v| v.as_str())
                .unwrap_or("")
        )
    });
    
    Ok(meta)
})
```

## HTTP Integration Example

```rust
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::Json,
    routing::{get, post, put, delete},
    Router,
};

#[derive(Clone)]
struct AppState {
    scim_server: Arc<DynamicScimServer<MyProvider>>,
}

async fn create_resource(
    Path(resource_type): Path<String>,
    State(state): State<AppState>,
    Json(data): Json<Value>,
) -> Result<Json<Value>, StatusCode> {
    let context = RequestContext::new(uuid::Uuid::new_v4().to_string());
    
    match state.scim_server.create_resource(&resource_type, data, &context).await {
        Ok(resource) => Ok(Json(resource.data)),
        Err(e) => {
            log::error!("Failed to create {}: {}", resource_type, e);
            Err(StatusCode::BAD_REQUEST)
        }
    }
}

async fn get_resource(
    Path((resource_type, id)): Path<(String, String)>,
    State(state): State<AppState>,
) -> Result<Json<Value>, StatusCode> {
    let context = RequestContext::new(uuid::Uuid::new_v4().to_string());
    
    match state.scim_server.get_resource(&resource_type, &id, &context).await {
        Ok(Some(resource)) => Ok(Json(resource.data)),
        Ok(None) => Err(StatusCode::NOT_FOUND),
        Err(e) => {
            log::error!("Failed to get {} {}: {}", resource_type, id, e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

fn create_router(state: AppState) -> Router {
    Router::new()
        .route("/scim/v2/:resource_type", post(create_resource))
        .route("/scim/v2/:resource_type/:id", get(get_resource))
        .route("/scim/v2/:resource_type/:id", put(update_resource))
        .route("/scim/v2/:resource_type/:id", delete(delete_resource))
        .route("/scim/v2/:resource_type", get(list_resources))
        .with_state(state)
}
```

## Best Practices

### 1. Schema Design
- **Use meaningful IDs**: Schema IDs should follow URN format
- **Validate early**: Put validation in setters, not just at the schema level
- **Document attributes**: Use clear descriptions in schema definitions
- **Version schemas**: Include version information in schema IDs

### 2. Handler Organization
- **One handler per schema**: Don't try to handle multiple schemas in one handler
- **Separate concerns**: Keep business logic in custom methods, not in getters/setters
- **Use transformers**: For complex data transformations that don't fit getter/setter model
- **Database mapping**: Always provide database mapping for persistence layers

### 3. Error Handling
- **Specific errors**: Use appropriate ScimError types
- **Validation messages**: Provide clear, actionable error messages
- **Logging**: Log errors with sufficient context for debugging
- **Recovery**: Design handlers to be resilient to partial failures

### 4. Performance
- **Lazy loading**: Only load data when needed
- **Caching**: Cache frequently accessed schemas and handlers
- **Batch operations**: Implement batch processing for list operations
- **Database efficiency**: Use efficient queries in your provider implementation

### 5. Security
- **Input validation**: Always validate input data
- **Authorization**: Implement proper authorization in your provider
- **Audit trails**: Log all operations for security auditing
- **Data sanitization**: Sanitize data before storage and retrieval

## Troubleshooting

### Common Issues

1. **Schema validation errors**
   - Ensure required attributes are present
   - Check data types match schema definitions
   - Verify canonical values are respected

2. **Handler registration failures**
   - Check for duplicate resource type registrations
   - Ensure schema IDs are unique
   - Verify all required operations are supported

3. **Provider errors**
   - Implement proper error handling in your provider
   - Check database connection and permissions
   - Verify data mapping between SCIM and database schemas

4. **Performance issues**
   - Profile your provider implementation
   - Check for N+1 query problems
   - Consider implementing caching

### Debug Tips

- Enable debug logging: `RUST_LOG=debug cargo run`
- Use the provided examples as reference implementations
- Test with simple data first, then add complexity
- Use the schema validation tools to verify your schemas

## Migration from Static Implementation

If you're migrating from the old static implementation:

1. **Identify hard-coded methods**: Look for methods like `get_username()`, `is_active()`
2. **Convert to custom methods**: Register these as custom methods in your handler
3. **Update provider**: Migrate from `ResourceProvider` to `DynamicResourceProvider`
4. **Test incrementally**: Migrate one resource type at a time
5. **Maintain compatibility**: Both approaches can coexist during migration

This dynamic approach provides unlimited flexibility while maintaining type safety and performance. Start with the provided User handler example and gradually build out your custom resource types as needed.