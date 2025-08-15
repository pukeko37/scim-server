# Error Codes

This reference documents all error codes that the SCIM Server library can generate, their meanings, causes, and recommended responses.

## Error Response Format

All SCIM errors follow the standard SCIM 2.0 error response format:

```json
{
  "schemas": ["urn:ietf:params:scim:api:messages:2.0:Error"],
  "status": "400",
  "scimType": "invalidFilter",
  "detail": "The specified filter syntax is invalid: unexpected token 'and' at position 15"
}
```

### Error Response Fields

| Field | Type | Description |
|-------|------|-------------|
| `schemas` | Array | Always contains the SCIM error schema |
| `status` | String | HTTP status code as a string |
| `scimType` | String | SCIM-specific error type (optional) |
| `detail` | String | Human-readable error description |

## HTTP Status Codes

### 400 Bad Request

**When**: The request is malformed or contains invalid data.

**Common Causes**:
- Invalid JSON syntax
- Missing required fields
- Invalid field values
- Malformed filter expressions

**Example**:
```json
{
  "schemas": ["urn:ietf:params:scim:api:messages:2.0:Error"],
  "status": "400",
  "scimType": "invalidSyntax",
  "detail": "Request body contains invalid JSON: expected ',' or '}' at line 3 column 15"
}
```

### 401 Unauthorized

**When**: Authentication is required but missing or invalid.

**Common Causes**:
- Missing Authorization header
- Invalid credentials
- Expired tokens
- Malformed authentication headers

**Example**:
```json
{
  "schemas": ["urn:ietf:params:scim:api:messages:2.0:Error"],
  "status": "401",
  "detail": "Authentication required: missing or invalid Authorization header"
}
```

### 403 Forbidden

**When**: Authentication succeeded but authorization failed.

**Common Causes**:
- Insufficient permissions for the operation
- Tenant isolation violations
- Read-only attribute modification attempts

**Example**:
```json
{
  "schemas": ["urn:ietf:params:scim:api:messages:2.0:Error"],
  "status": "403",
  "detail": "Insufficient permissions: cannot modify users in tenant 'production'"
}
```

### 404 Not Found

**When**: The requested resource doesn't exist.

**Common Causes**:
- Invalid resource ID
- Resource was deleted
- Incorrect endpoint path
- Wrong tenant context

**Example**:
```json
{
  "schemas": ["urn:ietf:params:scim:api:messages:2.0:Error"],
  "status": "404",
  "detail": "User with ID '123e4567-e89b-12d3-a456-426614174000' not found"
}
```

### 409 Conflict

**When**: The operation conflicts with the current state.

**Common Causes**:
- Duplicate unique values (usernames, emails)
- Circular group memberships
- Resource already exists

**Example**:
```json
{
  "schemas": ["urn:ietf:params:scim:api:messages:2.0:Error"],
  "status": "409",
  "scimType": "uniqueness",
  "detail": "User with userName 'jdoe@example.com' already exists"
}
```

### 412 Precondition Failed

**When**: ETag-based concurrency control detects a conflict.

**Common Causes**:
- Resource was modified by another client
- Missing or invalid If-Match header
- Concurrent updates

**Example**:
```json
{
  "schemas": ["urn:ietf:params:scim:api:messages:2.0:Error"],
  "status": "412",
  "detail": "Resource was modified: expected ETag 'W/\"abc123\"' but found 'W/\"def456\"'"
}
```

### 413 Payload Too Large

**When**: The request body exceeds size limits.

**Common Causes**:
- Large bulk operations
- Excessive user attributes
- Large file uploads

**Example**:
```json
{
  "schemas": ["urn:ietf:params:scim:api:messages:2.0:Error"],
  "status": "413",
  "detail": "Request body size 5242880 bytes exceeds maximum allowed size of 1048576 bytes"
}
```

### 429 Too Many Requests

**When**: Rate limiting is triggered.

**Common Causes**:
- Exceeding API rate limits
- Too many concurrent requests
- Bulk operation limits

**Example**:
```json
{
  "schemas": ["urn:ietf:params:scim:api:messages:2.0:Error"],
  "status": "429",
  "detail": "Rate limit exceeded: maximum 100 requests per minute, retry after 60 seconds"
}
```

### 500 Internal Server Error

**When**: An unexpected server error occurs.

**Common Causes**:
- Database connection failures
- Unhandled exceptions
- Configuration errors
- Provider implementation bugs

**Example**:
```json
{
  "schemas": ["urn:ietf:params:scim:api:messages:2.0:Error"],
  "status": "500",
  "detail": "Internal server error: database connection timeout"
}
```

### 501 Not Implemented

**When**: The requested feature is not supported.

**Common Causes**:
- Optional SCIM features not implemented
- Unsupported operations
- Missing provider functionality

**Example**:
```json
{
  "schemas": ["urn:ietf:params:scim:api:messages:2.0:Error"],
  "status": "501",
  "detail": "Bulk operations are not supported by this provider"
}
```

### 503 Service Unavailable

**When**: The service is temporarily unavailable.

**Common Causes**:
- Database maintenance
- System overload
- Temporary outages

**Example**:
```json
{
  "schemas": ["urn:ietf:params:scim:api:messages:2.0:Error"],
  "status": "503",
  "detail": "Service temporarily unavailable: scheduled maintenance in progress"
}
```

## SCIM Error Types

SCIM defines specific error types in the `scimType` field for more precise error categorization:

### invalidFilter

**Description**: Filter expression syntax is invalid.

**HTTP Status**: 400

**Common Causes**:
- Malformed filter syntax
- Unknown attributes in filter
- Invalid operators
- Missing quotes or parentheses

**Examples**:
```
# Invalid operator
filter=userName xyz "john"

# Missing quotes
filter=userName eq john@example.com

# Unknown attribute
filter=unknownField eq "value"
```

**Response**:
```json
{
  "schemas": ["urn:ietf:params:scim:api:messages:2.0:Error"],
  "status": "400",
  "scimType": "invalidFilter",
  "detail": "Invalid filter expression: unknown operator 'xyz' at position 9"
}
```

### tooMany

**Description**: Query returned too many results.

**HTTP Status**: 400

**Common Causes**:
- Query without sufficient filtering
- Missing pagination parameters
- Large datasets without limits

**Response**:
```json
{
  "schemas": ["urn:ietf:params:scim:api:messages:2.0:Error"],
  "status": "400",
  "scimType": "tooMany",
  "detail": "Query returned 50000 results, maximum allowed is 10000. Use pagination or add filters."
}
```

### uniqueness

**Description**: Unique constraint violation.

**HTTP Status**: 409

**Common Causes**:
- Duplicate usernames
- Duplicate email addresses
- Duplicate external IDs

**Response**:
```json
{
  "schemas": ["urn:ietf:params:scim:api:messages:2.0:Error"],
  "status": "409",
  "scimType": "uniqueness",
  "detail": "Email address 'user@example.com' is already in use by another user"
}
```

### mutability

**Description**: Attempt to modify a read-only attribute.

**HTTP Status**: 400

**Common Causes**:
- Modifying `id` field
- Changing `meta.created` timestamp
- Updating immutable custom attributes

**Response**:
```json
{
  "schemas": ["urn:ietf:params:scim:api:messages:2.0:Error"],
  "status": "400",
  "scimType": "mutability",
  "detail": "Attribute 'id' is immutable and cannot be modified"
}
```

### invalidSyntax

**Description**: Request syntax is invalid.

**HTTP Status**: 400

**Common Causes**:
- Malformed JSON
- Invalid attribute names
- Wrong data types

**Response**:
```json
{
  "schemas": ["urn:ietf:params:scim:api:messages:2.0:Error"],
  "status": "400",
  "scimType": "invalidSyntax",
  "detail": "Invalid JSON syntax: unexpected token '}' at line 5 column 1"
}
```

### invalidPath

**Description**: PATCH operation path is invalid.

**HTTP Status**: 400

**Common Causes**:
- Malformed JSON Path expressions
- Paths to non-existent attributes
- Invalid array indices

**Response**:
```json
{
  "schemas": ["urn:ietf:params:scim:api:messages:2.0:Error"],
  "status": "400",
  "scimType": "invalidPath",
  "detail": "Invalid patch path: 'emails[type eq \"work\"].value' - array filter not supported"
}
```

### invalidValue

**Description**: Attribute value doesn't meet validation requirements.

**HTTP Status**: 400

**Common Causes**:
- Invalid email format
- Password doesn't meet complexity requirements
- Enum values not in allowed list

**Response**:
```json
{
  "schemas": ["urn:ietf:params:scim:api:messages:2.0:Error"],
  "status": "400",
  "scimType": "invalidValue",
  "detail": "Invalid email format: 'not-an-email' is not a valid email address"
}
```

### invalidVers

**Description**: Version-related error in bulk operations.

**HTTP Status**: 412

**Common Causes**:
- ETag mismatches in bulk operations
- Version conflicts

**Response**:
```json
{
  "schemas": ["urn:ietf:params:scim:api:messages:2.0:Error"],
  "status": "412",
  "scimType": "invalidVers",
  "detail": "Version conflict in bulk operation: resource was modified"
}
```

### sensitive

**Description**: Request contains sensitive information that cannot be processed.

**HTTP Status**: 403

**Common Causes**:
- Accessing password attributes
- Retrieving sensitive security information

**Response**:
```json
{
  "schemas": ["urn:ietf:params:scim:api:messages:2.0:Error"],
  "status": "403",
  "scimType": "sensitive",
  "detail": "Cannot retrieve sensitive attribute: password"
}
```

## Library-Specific Error Codes

These errors are specific to the SCIM Server library implementation:

### TenantNotFound

**Description**: Specified tenant does not exist.

**HTTP Status**: 404

**Rust Error**: `ScimError::TenantNotFound`

**Response**:
```json
{
  "schemas": ["urn:ietf:params:scim:api:messages:2.0:Error"],
  "status": "404",
  "detail": "Tenant 'acme-corp' not found"
}
```

### ProviderError

**Description**: Storage provider encountered an error.

**HTTP Status**: 500

**Rust Error**: `ScimError::ProviderError`

**Response**:
```json
{
  "schemas": ["urn:ietf:params:scim:api:messages:2.0:Error"],
  "status": "500",
  "detail": "Provider error: database connection failed"
}
```

### ValidationError

**Description**: Custom validation failed.

**HTTP Status**: 400

**Rust Error**: `ScimError::ValidationError`

**Response**:
```json
{
  "schemas": ["urn:ietf:params:scim:api:messages:2.0:Error"],
  "status": "400",
  "scimType": "invalidValue",
  "detail": "Custom validation failed: employee ID must be 6 digits"
}
```

### SerializationError

**Description**: JSON serialization/deserialization failed.

**HTTP Status**: 400

**Rust Error**: `ScimError::SerializationError`

**Response**:
```json
{
  "schemas": ["urn:ietf:params:scim:api:messages:2.0:Error"],
  "status": "400",
  "scimType": "invalidSyntax",
  "detail": "JSON parsing error: missing field 'userName' at line 3"
}
```

## Error Handling Best Practices

### For API Clients

1. **Always check HTTP status codes** before processing responses
2. **Parse the `scimType` field** for specific error handling
3. **Log the `detail` field** for debugging purposes
4. **Implement retry logic** for 429, 500, 502, 503 status codes
5. **Handle 412 errors** by refetching the resource and retrying

### For Server Implementations

1. **Provide detailed error messages** without exposing sensitive information
2. **Use appropriate HTTP status codes** for different error types
3. **Include SCIM error types** when applicable
4. **Log errors server-side** for monitoring and debugging
5. **Sanitize error details** to prevent information leakage

### Example Error Handling

```rust
use scim_server::{ScimError, ScimResult};

async fn handle_user_creation(user_data: Value) -> ScimResult<User> {
    match create_user(user_data).await {
        Ok(user) => Ok(user),
        Err(ScimError::ValidationError { field, message }) => {
            Err(ScimError::BadRequest {
                scim_type: Some("invalidValue".to_string()),
                detail: format!("Validation failed for field '{}': {}", field, message),
            })
        },
        Err(ScimError::UniqueConstraintViolation { field, value }) => {
            Err(ScimError::Conflict {
                scim_type: Some("uniqueness".to_string()),
                detail: format!("Value '{}' for field '{}' already exists", value, field),
            })
        },
        Err(e) => Err(e), // Re-throw other errors
    }
}
```

### Client-Side Error Handling

```javascript
async function createUser(userData) {
    try {
        const response = await fetch('/Users', {
            method: 'POST',
            headers: { 'Content-Type': 'application/scim+json' },
            body: JSON.stringify(userData)
        });
        
        if (!response.ok) {
            const error = await response.json();
            
            switch (response.status) {
                case 400:
                    if (error.scimType === 'invalidValue') {
                        throw new ValidationError(error.detail);
                    }
                    throw new BadRequestError(error.detail);
                    
                case 409:
                    if (error.scimType === 'uniqueness') {
                        throw new DuplicateError(error.detail);
                    }
                    throw new ConflictError(error.detail);
                    
                case 429:
                    // Implement exponential backoff
                    await sleep(getRetryDelay());
                    return createUser(userData);
                    
                default:
                    throw new ScimError(response.status, error.detail);
            }
        }
        
        return response.json();
    } catch (error) {
        console.error('User creation failed:', error);
        throw error;
    }
}
```

## Debugging Error Responses

### Enable Detailed Logging

```rust
use tracing::{error, warn, debug};

// In your error handler
match result {
    Err(ScimError::ValidationError { field, message }) => {
        warn!("Validation error for field '{}': {}", field, message);
        // Return user-friendly error
    },
    Err(ScimError::ProviderError(e)) => {
        error!("Provider error: {:?}", e);
        // Return generic error message
    },
    _ => {}
}
```

### Error Response Testing

```rust
#[tokio::test]
async fn test_error_responses() {
    let server = test_server().await;
    
    // Test validation error
    let response = server
        .post("/Users")
        .json(&json!({"userName": ""})) // Invalid empty username
        .await;
        
    assert_eq!(response.status(), 400);
    
    let error: ScimError = response.json().await;
    assert_eq!(error.scim_type, Some("invalidValue".to_string()));
    assert!(error.detail.contains("userName"));
}
```

## Common Error Scenarios

### Bulk Operation Errors

Bulk operations can contain mixed success/failure results:

```json
{
  "schemas": ["urn:ietf:params:scim:api:messages:2.0:BulkResponse"],
  "Operations": [
    {
      "method": "POST",
      "bulkId": "user1",
      "location": "/Users/123",
      "status": "201"
    },
    {
      "method": "POST", 
      "bulkId": "user2",
      "status": "409",
      "response": {
        "schemas": ["urn:ietf:params:scim:api:messages:2.0:Error"],
        "status": "409",
        "scimType": "uniqueness",
        "detail": "userName 'duplicate@example.com' already exists"
      }
    }
  ]
}
```

### Multi-Tenant Errors

```json
{
  "schemas": ["urn:ietf:params:scim:api:messages:2.0:Error"],
  "status": "404",
  "detail": "Resource not found in tenant 'customer-a'. Verify tenant context and resource ID."
}
```

This comprehensive error reference should help developers understand, handle, and debug all error scenarios in the SCIM Server library.