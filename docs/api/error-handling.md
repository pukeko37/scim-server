# Error Handling API Reference

This document provides comprehensive documentation for error handling in the SCIM Server crate. The error system is designed to provide clear, actionable error information while maintaining type safety and proper error propagation.

## Table of Contents

- [Overview](#overview)
- [Error Types](#error-types)
- [Error Hierarchy](#error-hierarchy)
- [HTTP Status Mapping](#http-status-mapping)
- [Error Construction](#error-construction)
- [Error Handling Patterns](#error-handling-patterns)
- [Custom Error Types](#custom-error-types)
- [Error Serialization](#error-serialization)
- [Best Practices](#best-practices)

## Overview

The SCIM Server uses a structured error system that:

- Maps internal errors to appropriate HTTP status codes
- Provides detailed error information for debugging
- Maintains SCIM 2.0 protocol compliance
- Supports error context and chaining
- Enables type-safe error handling

### Error Design Principles

1. **Explicit Error Types** - Each error category has a specific variant
2. **Context Preservation** - Errors carry relevant context information
3. **HTTP Compliance** - Errors map correctly to HTTP status codes
4. **Developer Friendly** - Clear error messages aid debugging
5. **Client Friendly** - Structured error responses for API consumers

## Error Types

### Core Error Enum

```rust
use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", content = "details")]
pub enum ScimError {
    // Client Errors (4xx)
    BadRequest {
        message: String,
        details: Option<String>,
    },
    Unauthorized {
        realm: Option<String>,
        message: String,
    },
    Forbidden {
        resource: Option<String>,
        action: Option<String>,
        message: String,
    },
    NotFound {
        resource_type: String,
        id: String,
    },
    Conflict {
        message: String,
        existing_resource: Option<String>,
    },
    
    // Validation Errors
    Validation {
        field: String,
        message: String,
        value: Option<String>,
    },
    SchemaViolation {
        schema: String,
        violation: String,
        path: Option<String>,
    },
    
    // Server Errors (5xx)
    InternalServerError {
        message: String,
        correlation_id: Option<String>,
    },
    ServiceUnavailable {
        message: String,
        retry_after: Option<std::time::Duration>,
    },
    
    // Provider Errors
    ProviderError {
        provider_type: String,
        message: String,
        source: Option<String>,
    },
    
    // Multi-tenant Errors
    TenantNotFound {
        tenant_id: String,
    },
    TenantResolutionFailed {
        hint: String,
        reason: String,
    },
    
    // Schema Errors
    SchemaNotFound {
        schema_uri: String,
    },
    SchemaLoadError {
        schema_uri: String,
        reason: String,
    },
    
    // Resource Type Errors
    UnsupportedResourceType {
        resource_type: String,
        supported_types: Vec<String>,
    },
    
    // Operation Errors
    OperationNotSupported {
        operation: String,
        resource_type: String,
    },
    
    // Filter/Query Errors
    InvalidFilter {
        filter: String,
        reason: String,
    },
    
    // Bulk Operation Errors
    BulkOperationError {
        operation_index: usize,
        error: Box<ScimError>,
    },
}
```

### Validation Error Details

```rust
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ValidationError {
    pub field: String,
    pub message: String,
    pub value: Option<String>,
    pub constraint: Option<String>,
    pub error_code: Option<String>,
}

impl ValidationError {
    pub fn required_field(field: &str) -> Self {
        Self {
            field: field.to_string(),
            message: format!("Field '{}' is required", field),
            value: None,
            constraint: Some("required".to_string()),
            error_code: Some("REQUIRED_FIELD".to_string()),
        }
    }
    
    pub fn invalid_format(field: &str, value: &str, expected_format: &str) -> Self {
        Self {
            field: field.to_string(),
            message: format!("Field '{}' has invalid format. Expected: {}", field, expected_format),
            value: Some(value.to_string()),
            constraint: Some(expected_format.to_string()),
            error_code: Some("INVALID_FORMAT".to_string()),
        }
    }
    
    pub fn value_too_long(field: &str, value: &str, max_length: usize) -> Self {
        Self {
            field: field.to_string(),
            message: format!("Field '{}' exceeds maximum length of {}", field, max_length),
            value: Some(value.to_string()),
            constraint: Some(format!("maxLength:{}", max_length)),
            error_code: Some("VALUE_TOO_LONG".to_string()),
        }
    }
}
```

## Error Hierarchy

### Client Errors (4xx)

These errors indicate problems with the client request:

#### BadRequest (400)
```rust
ScimError::BadRequest {
    message: "Invalid JSON in request body".to_string(),
    details: Some("Expected object, found array at line 5".to_string()),
}
```

**When to use:**
- Malformed JSON
- Invalid request structure
- Missing required headers
- Invalid parameter values

#### Unauthorized (401)
```rust
ScimError::Unauthorized {
    realm: Some("SCIM API".to_string()),
    message: "Invalid or expired authentication token".to_string(),
}
```

**When to use:**
- Missing authentication credentials
- Invalid authentication tokens
- Expired tokens

#### Forbidden (403)
```rust
ScimError::Forbidden {
    resource: Some("Users".to_string()),
    action: Some("delete".to_string()),
    message: "Insufficient permissions to delete users".to_string(),
}
```

**When to use:**
- Valid authentication but insufficient permissions
- Resource access denied
- Operation not allowed for user role

#### NotFound (404)
```rust
ScimError::NotFound {
    resource_type: "User".to_string(),
    id: "user-123".to_string(),
}
```

**When to use:**
- Resource doesn't exist
- Invalid resource ID
- Resource deleted or moved

#### Conflict (409)
```rust
ScimError::Conflict {
    message: "User with this username already exists".to_string(),
    existing_resource: Some("user-456".to_string()),
}
```

**When to use:**
- Duplicate resource creation
- Conflicting updates
- Unique constraint violations

### Validation Errors

#### Field Validation
```rust
ScimError::Validation {
    field: "emails[0].value".to_string(),
    message: "Invalid email format".to_string(),
    value: Some("invalid-email".to_string()),
}
```

#### Schema Violations
```rust
ScimError::SchemaViolation {
    schema: "urn:ietf:params:scim:schemas:core:2.0:User".to_string(),
    violation: "Missing required attribute 'userName'".to_string(),
    path: Some("/userName".to_string()),
}
```

### Server Errors (5xx)

#### Internal Server Error (500)
```rust
ScimError::InternalServerError {
    message: "Database connection failed".to_string(),
    correlation_id: Some("req-abc123".to_string()),
}
```

#### Service Unavailable (503)
```rust
ScimError::ServiceUnavailable {
    message: "Database is temporarily unavailable".to_string(),
    retry_after: Some(Duration::from_secs(30)),
}
```

## HTTP Status Mapping

The error system automatically maps errors to appropriate HTTP status codes:

```rust
impl ScimError {
    pub fn status_code(&self) -> u16 {
        match self {
            ScimError::BadRequest { .. } => 400,
            ScimError::Unauthorized { .. } => 401,
            ScimError::Forbidden { .. } => 403,
            ScimError::NotFound { .. } => 404,
            ScimError::Conflict { .. } => 409,
            ScimError::Validation { .. } => 400,
            ScimError::SchemaViolation { .. } => 400,
            ScimError::InternalServerError { .. } => 500,
            ScimError::ServiceUnavailable { .. } => 503,
            ScimError::ProviderError { .. } => 500,
            ScimError::TenantNotFound { .. } => 404,
            ScimError::TenantResolutionFailed { .. } => 400,
            ScimError::SchemaNotFound { .. } => 400,
            ScimError::SchemaLoadError { .. } => 500,
            ScimError::UnsupportedResourceType { .. } => 400,
            ScimError::OperationNotSupported { .. } => 501,
            ScimError::InvalidFilter { .. } => 400,
            ScimError::BulkOperationError { error, .. } => error.status_code(),
        }
    }
}
```

### SCIM Error Response Format

Errors are serialized according to SCIM 2.0 specification:

```json
{
    "schemas": ["urn:ietf:params:scim:api:messages:2.0:Error"],
    "status": "400",
    "scimType": "invalidValue",
    "detail": "Invalid email format in emails[0].value",
    "location": "/Users/user-123"
}
```

## Error Construction

### Direct Construction

```rust
// Create specific error types
let not_found = ScimError::NotFound {
    resource_type: "User".to_string(),
    id: user_id.to_string(),
};

let validation_error = ScimError::Validation {
    field: "userName".to_string(),
    message: "Username cannot be empty".to_string(),
    value: Some("".to_string()),
};
```

### Helper Functions

```rust
impl ScimError {
    // Convenience constructors
    pub fn bad_request<T: Into<String>>(message: T) -> Self {
        Self::BadRequest {
            message: message.into(),
            details: None,
        }
    }
    
    pub fn not_found<T: Into<String>>(resource_type: T, id: T) -> Self {
        Self::NotFound {
            resource_type: resource_type.into(),
            id: id.into(),
        }
    }
    
    pub fn validation_error<T: Into<String>>(field: T, message: T) -> Self {
        Self::Validation {
            field: field.into(),
            message: message.into(),
            value: None,
        }
    }
    
    pub fn internal_error<T: Into<String>>(message: T) -> Self {
        Self::InternalServerError {
            message: message.into(),
            correlation_id: None,
        }
    }
}
```

### Error Context

Add context to errors for better debugging:

```rust
impl ScimError {
    pub fn with_context<T: Into<String>>(mut self, context: T) -> Self {
        let context = context.into();
        match &mut self {
            ScimError::BadRequest { message, .. } => {
                *message = format!("{}: {}", context, message);
            }
            ScimError::InternalServerError { message, .. } => {
                *message = format!("{}: {}", context, message);
            }
            ScimError::ProviderError { message, .. } => {
                *message = format!("{}: {}", context, message);
            }
            _ => {} // Other error types maintain their structure
        }
        self
    }
    
    pub fn with_correlation_id<T: Into<String>>(mut self, correlation_id: T) -> Self {
        match &mut self {
            ScimError::InternalServerError { correlation_id: ref mut id, .. } => {
                *id = Some(correlation_id.into());
            }
            _ => {} // Only applicable to server errors
        }
        self
    }
}

// Usage example
fn process_user_creation(user_data: &str) -> Result<Resource> {
    serde_json::from_str(user_data)
        .map_err(|e| ScimError::bad_request("Invalid JSON")
            .with_context("User creation"))
        .and_then(|resource| validate_resource(resource))
}
```

## Error Handling Patterns

### Result Type Usage

All fallible operations return `Result<T, ScimError>`:

```rust
use scim_server::error::{Result, ScimError};

// Function signature
async fn create_user(user_data: serde_json::Value) -> Result<Resource> {
    // Validate input
    let resource = Resource::from_json(user_data)
        .map_err(|e| ScimError::bad_request(format!("Invalid user data: {}", e)))?;
    
    // Validate schema
    validate_user_schema(&resource).await?;
    
    // Store resource
    provider.create_resource(resource).await
}
```

### Error Propagation with Context

```rust
async fn update_user_email(
    user_id: &ResourceId,
    new_email: &str,
) -> Result<Resource> {
    // Get existing user
    let mut user = provider.get_resource(user_id).await?
        .ok_or_else(|| ScimError::not_found("User", user_id.as_str()))?;
    
    // Validate email format
    let email = EmailAddress::new(new_email)
        .map_err(|e| ScimError::validation_error("emails[0].value", e.to_string()))?;
    
    // Update user
    user.set_primary_email(email)
        .map_err(|e| ScimError::bad_request(format!("Failed to set email: {}", e)))?;
    
    // Save changes
    provider.update_resource(user).await
        .map_err(|e| e.with_context("Updating user email"))
}
```

### Error Matching

```rust
match create_user(user_data).await {
    Ok(user) => {
        println!("User created successfully: {}", user.id());
    }
    Err(ScimError::Validation { field, message, .. }) => {
        eprintln!("Validation failed on field '{}': {}", field, message);
    }
    Err(ScimError::Conflict { message, .. }) => {
        eprintln!("User already exists: {}", message);
    }
    Err(ScimError::ProviderError { message, .. }) => {
        eprintln!("Storage error: {}", message);
    }
    Err(e) => {
        eprintln!("Unexpected error: {}", e);
    }
}
```

## Error Construction

### Validation Errors

```rust
// Field validation
pub fn validate_user_name(value: &str) -> Result<()> {
    if value.is_empty() {
        return Err(ScimError::Validation {
            field: "userName".to_string(),
            message: "Username cannot be empty".to_string(),
            value: Some(value.to_string()),
        });
    }
    
    if value.len() > 255 {
        return Err(ScimError::Validation {
            field: "userName".to_string(),
            message: "Username too long (max 255 characters)".to_string(),
            value: Some(format!("{}...", &value[..50])),
        });
    }
    
    if !value.chars().all(|c| c.is_alphanumeric() || c == '.' || c == '_' || c == '-') {
        return Err(ScimError::Validation {
            field: "userName".to_string(),
            message: "Username contains invalid characters".to_string(),
            value: Some(value.to_string()),
        });
    }
    
    Ok(())
}

// Schema validation
pub fn validate_required_attributes(resource: &Resource, schema: &Schema) -> Result<()> {
    for attr in schema.required_attributes() {
        if !resource.has_attribute(&attr.name) {
            return Err(ScimError::SchemaViolation {
                schema: schema.id().to_string(),
                violation: format!("Missing required attribute '{}'", attr.name),
                path: Some(format!("/{}", attr.name)),
            });
        }
    }
    Ok(())
}
```

### Provider Errors

```rust
// Convert provider-specific errors
impl From<sqlx::Error> for ScimError {
    fn from(err: sqlx::Error) -> Self {
        match err {
            sqlx::Error::RowNotFound => ScimError::NotFound {
                resource_type: "Resource".to_string(),
                id: "unknown".to_string(),
            },
            sqlx::Error::Database(db_err) => {
                if db_err.code() == Some(std::borrow::Cow::Borrowed("23505")) {
                    // PostgreSQL unique violation
                    ScimError::Conflict {
                        message: "Resource already exists".to_string(),
                        existing_resource: None,
                    }
                } else {
                    ScimError::ProviderError {
                        provider_type: "Database".to_string(),
                        message: db_err.message().to_string(),
                        source: Some(db_err.code().map(|c| c.to_string()).unwrap_or_default()),
                    }
                }
            }
            _ => ScimError::ProviderError {
                provider_type: "Database".to_string(),
                message: err.to_string(),
                source: None,
            }
        }
    }
}

// HTTP client errors
impl From<reqwest::Error> for ScimError {
    fn from(err: reqwest::Error) -> Self {
        if err.is_timeout() {
            ScimError::ServiceUnavailable {
                message: "Request timeout".to_string(),
                retry_after: Some(Duration::from_secs(30)),
            }
        } else if err.is_connect() {
            ScimError::ServiceUnavailable {
                message: "Cannot connect to external service".to_string(),
                retry_after: Some(Duration::from_secs(60)),
            }
        } else {
            ScimError::ProviderError {
                provider_type: "HTTP".to_string(),
                message: err.to_string(),
                source: None,
            }
        }
    }
}
```

## Custom Error Types

### Domain-Specific Errors

```rust
#[derive(Debug, Clone, PartialEq)]
pub enum UserValidationError {
    InvalidEmailFormat(String),
    DuplicateEmail(String),
    InvalidPhoneNumber(String),
    InvalidDepartmentCode(String),
}

impl fmt::Display for UserValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            UserValidationError::InvalidEmailFormat(email) => {
                write!(f, "Invalid email format: {}", email)
            }
            UserValidationError::DuplicateEmail(email) => {
                write!(f, "Email already in use: {}", email)
            }
            UserValidationError::InvalidPhoneNumber(phone) => {
                write!(f, "Invalid phone number format: {}", phone)
            }
            UserValidationError::InvalidDepartmentCode(code) => {
                write!(f, "Unknown department code: {}", code)
            }
        }
    }
}

impl std::error::Error for UserValidationError {}

// Convert to ScimError
impl From<UserValidationError> for ScimError {
    fn from(err: UserValidationError) -> Self {
        match err {
            UserValidationError::InvalidEmailFormat(email) => ScimError::Validation {
                field: "emails[].value".to_string(),
                message: format!("Invalid email format: {}", email),
                value: Some(email),
            },
            UserValidationError::DuplicateEmail(email) => ScimError::Conflict {
                message: format!("Email already in use: {}", email),
                existing_resource: None,
            },
            UserValidationError::InvalidPhoneNumber(phone) => ScimError::Validation {
                field: "phoneNumbers[].value".to_string(),
                message: format!("Invalid phone number: {}", phone),
                value: Some(phone),
            },
            UserValidationError::InvalidDepartmentCode(code) => ScimError::Validation {
                field: "department".to_string(),
                message: format!("Unknown department code: {}", code),
                value: Some(code),
            },
        }
    }
}
```

### Error Chaining

```rust
use std::error::Error as StdError;

impl ScimError {
    pub fn chain_error<E: StdError + Send + Sync + 'static>(
        mut self,
        source: E,
    ) -> Self {
        match &mut self {
            ScimError::ProviderError { source: ref mut src, .. } => {
                *src = Some(source.to_string());
            }
            ScimError::InternalServerError { message, .. } => {
                *message = format!("{}: {}", message, source);
            }
            _ => {} // Other error types don't support chaining
        }
        self
    }
}

// Usage
fn process_database_operation() -> Result<Resource> {
    perform_database_query()
        .map_err(|db_err| ScimError::provider_error("Database")
            .chain_error(db_err))
}
```

## Error Serialization

### JSON Serialization

Errors are serialized to JSON following SCIM 2.0 error response format:

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct ScimErrorResponse {
    pub schemas: Vec<String>,
    pub status: String,
    #[serde(rename = "scimType", skip_serializing_if = "Option::is_none")]
    pub scim_type: Option<String>,
    pub detail: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub location: Option<String>,
}

impl From<ScimError> for ScimErrorResponse {
    fn from(error: ScimError) -> Self {
        Self {
            schemas: vec!["urn:ietf:params:scim:api:messages:2.0:Error".to_string()],
            status: error.status_code().to_string(),
            scim_type: error.scim_type(),
            detail: error.to_string(),
            location: error.location(),
        }
    }
}

impl ScimError {
    pub fn scim_type(&self) -> Option<String> {
        match self {
            ScimError::BadRequest { .. } => Some("invalidValue".to_string()),
            ScimError::Unauthorized { .. } => Some("invalidCredentials".to_string()),
            ScimError::NotFound { .. } => Some("notFound".to_string()),
            ScimError::Conflict { .. } => Some("uniqueness".to_string()),
            ScimError::Validation { .. } => Some("invalidValue".to_string()),
            ScimError::SchemaViolation { .. } => Some("invalidValue".to_string()),
            _ => None,
        }
    }
    
    pub fn location(&self) -> Option<String> {
        match self {
            ScimError::NotFound { resource_type, id } => {
                Some(format!("/{}/{}", resource_type, id))
            }
            ScimError::Validation { field, .. } => {
                Some(format!("/{}", field))
            }
            _ => None,
        }
    }
}
```

### Error Response Examples

**Validation Error:**
```json
{
    "schemas": ["urn:ietf:params:scim:api:messages:2.0:Error"],
    "status": "400",
    "scimType": "invalidValue",
    "detail": "Invalid email format in emails[0].value",
    "location": "/emails[0].value"
}
```

**Not Found Error:**
```json
{
    "schemas": ["urn:ietf:params:scim:api:messages:2.0:Error"],
    "status": "404",
    "scimType": "notFound",
    "detail": "User with id 'user-123' not found",
    "location": "/Users/user-123"
}
```

**Conflict Error:**
```json
{
    "schemas": ["urn:ietf:params:scim:api:messages:2.0:Error"],
    "status": "409",
    "scimType": "uniqueness",
    "detail": "User with username 'john.doe' already exists"
}
```

## Error Handling Patterns

### Async Error Handling

```rust
use futures::TryFutureExt;

async fn complex_operation(user_id: &ResourceId) -> Result<Resource> {
    // Chain async operations with error handling
    get_user(user_id).await?
        .ok_or_else(|| ScimError::not_found("User", user_id.as_str()))?
        .validate_for_update().await?
        .apply_business_rules().await?
        .save_to_provider().await
        .map_err(|e| e.with_context("Complex operation failed"))
}

// Alternative: use try_* combinators
async fn alternative_complex_operation(user_id: &ResourceId) -> Result<Resource> {
    let user = get_user(user_id)
        .and_then(|opt| async move {
            opt.ok_or_else(|| ScimError::not_found("User", user_id.as_str()))
        })
        .await?;
    
    validate_and_save_user(user)
        .map_err(|e| e.with_context("Failed to validate and save user"))
        .await
}
```

### Error Recovery

```rust
async fn resilient_operation(resource: Resource) -> Result<Resource> {
    // Try primary provider
    match primary_provider.create_resource(resource.clone()).await {
        Ok(created) => Ok(created),
        Err(ScimError::ServiceUnavailable { .. }) => {
            // Fallback to secondary provider
            tracing::warn!("Primary provider unavailable, using fallback");
            fallback_provider.create_resource(resource).await
                .map_err(|e| e.with_context("Both providers failed"))
        }
        Err(e) => Err(e),
    }
}
```

### Batch Error Handling

```rust
async fn process_user_batch(users: Vec<Resource>) -> (Vec<Resource>, Vec<ScimError>) {
    let mut successes = Vec::new();
    let mut errors = Vec::new();
    
    for (index, user) in users.into_iter().enumerate() {
        match provider.create_resource(user).await {
            Ok(created) => successes.push(created),
            Err(e) => errors.push(ScimError::BulkOperationError {
                operation_index: index,
                error: Box::new(e),
            }),
        }
    }
    
    (successes, errors)
}
```

## Provider Error Handling

### Database Provider Error Mapping

```rust
impl DatabaseProvider {
    async fn handle_database_error<T>(
        &self,
        operation: &str,
        result: sqlx::Result<T>,
    ) -> Result<T> {
        result.map_err(|err| {
            tracing::error!("Database operation '{}' failed: {}", operation, err);
            
            match err {
                sqlx::Error::RowNotFound => ScimError::NotFound {
                    resource_type: "Resource".to_string(),
                    id: "unknown".to_string(),
                },
                sqlx::Error::Database(db_err) => {
                    if db_err.is_unique_violation() {
                        ScimError::Conflict {
                            message: "Resource with this identifier already exists".to_string(),
                            existing_resource: None,
                        }
                    } else if db_err.is_foreign_key_violation() {
                        ScimError::BadRequest {
                            message: "Referenced resource does not exist".to_string(),
                            details: Some(db_err.message().to_string()),
                        }
                    } else {
                        ScimError::ProviderError {
                            provider_type: "Database".to_string(),
                            message: db_err.message().to_string(),
                            source: db_err.code().map(|c| c.to_string()),
                        }
                    }
                }
                sqlx::Error::PoolTimedOut => ScimError::ServiceUnavailable {
                    message: "Database connection pool exhausted".to_string(),
                    retry_after: Some(Duration::from_secs(30)),
                },
                _ => ScimError::ProviderError {
                    provider_type: "Database".to_string(),
                    message: err.to_string(),
                    source: None,
                }
            }
        })
    }
}
```

### External API Provider Error Mapping

```rust
impl ApiProvider {
    async fn handle_api_error(&self, response: reqwest::Response) -> ScimError {
        let status = response.status();
        
        match status {
            reqwest::StatusCode::NOT_FOUND => ScimError::NotFound {
                resource_type: "Resource".to_string(),
                id: "unknown".to_string(),
            },
            reqwest::StatusCode::CONFLICT => ScimError::Conflict {
                message: "Resource conflict in external system".to_string(),
                existing_resource: None,
            },
            reqwest::StatusCode::UNAUTHORIZED => ScimError::Unauthorized {
                realm: Some("External API".to_string()),
                message: "API authentication failed".to_string(),
            },
            reqwest::StatusCode::TOO_MANY_REQUESTS => ScimError::ServiceUnavailable {
                message: "Rate limit exceeded".to_string(),
                retry_after: response.headers()
                    .get("retry-after")
                    .and_then(|h| h.to_str().ok())
                    .and_then(|s| s.parse::<u64>().ok())
                    .map(Duration::from_secs),
            },
            _ => ScimError::ProviderError {
                provider_type: "External API".to_string(),
                message: format!("API request failed with status: {}", status),
                source: Some(status.to_string()),
            }
        }
    }
}
```

## Multi-Tenant Error Handling

### Tenant Resolution Errors

```rust
async fn resolve_tenant_with_fallback(hint: &str) -> Result<TenantContext> {
    // Try primary resolution strategy
    match primary_resolver.resolve_tenant(hint).await {
        Ok(tenant) => Ok(tenant),
        Err(ScimError::TenantNotFound { .. }) => {
            // Try secondary resolution strategy
            secondary_resolver.resolve_tenant(hint).await
                .map_err(|_| ScimError::TenantResolutionFailed {
                    hint: hint.to_string(),
                    reason: "No resolver could identify tenant".to_string(),
                })
        }
        Err(e) => Err(e),
    }
}
```

### Tenant-Scoped Error Context

```rust
async fn tenant_operation(
    tenant_id: &TenantId,
    operation: impl Future<Output = Result