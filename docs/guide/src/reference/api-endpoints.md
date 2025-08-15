# API Endpoints

This reference documents all HTTP endpoints provided by the SCIM Server, including request/response formats, status codes, and example usage.

## Base URL Structure

All SCIM endpoints follow this pattern:
```
https://your-server.com/scim/v2/{tenant-id}/{resource-type}
```

**Components**:
- `{tenant-id}`: Unique identifier for the tenant/organization
- `{resource-type}`: Resource type (Users, Groups, or custom resources)

## Standard Resource Endpoints

### Users

#### Create User
```http
POST /scim/v2/{tenant-id}/Users
Content-Type: application/scim+json
```

**Request Body**:
```json
{
  "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
  "userName": "alice@example.com",
  "name": {
    "givenName": "Alice",
    "familyName": "Johnson"
  },
  "emails": [
    {
      "value": "alice@example.com",
      "type": "work",
      "primary": true
    }
  ],
  "active": true
}
```

**Response** (201 Created):
```json
{
  "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
  "id": "2819c223-7f76-453a-919d-413861904646",
  "userName": "alice@example.com",
  "name": {
    "givenName": "Alice",
    "familyName": "Johnson"
  },
  "emails": [
    {
      "value": "alice@example.com",
      "type": "work",
      "primary": true
    }
  ],
  "active": true,
  "meta": {
    "resourceType": "User",
    "created": "2023-12-01T10:30:00Z",
    "lastModified": "2023-12-01T10:30:00Z",
    "version": "W/\"1\"",
    "location": "https://api.example.com/scim/v2/tenant-1/Users/2819c223-7f76-453a-919d-413861904646"
  }
}
```

#### Get User
```http
GET /scim/v2/{tenant-id}/Users/{id}
```

**Response** (200 OK):
```json
{
  "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
  "id": "2819c223-7f76-453a-919d-413861904646",
  "userName": "alice@example.com",
  "name": {
    "givenName": "Alice", 
    "familyName": "Johnson"
  },
  "emails": [
    {
      "value": "alice@example.com",
      "type": "work",
      "primary": true
    }
  ],
  "active": true,
  "meta": {
    "resourceType": "User",
    "created": "2023-12-01T10:30:00Z",
    "lastModified": "2023-12-01T15:45:00Z",
    "version": "W/\"3\"",
    "location": "https://api.example.com/scim/v2/tenant-1/Users/2819c223-7f76-453a-919d-413861904646"
  }
}
```

#### Update User (Replace)
```http
PUT /scim/v2/{tenant-id}/Users/{id}
Content-Type: application/scim+json
If-Match: W/"3"
```

**Request Body**:
```json
{
  "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
  "id": "2819c223-7f76-453a-919d-413861904646",
  "userName": "alice@example.com",
  "name": {
    "givenName": "Alice",
    "familyName": "Smith"
  },
  "emails": [
    {
      "value": "alice@example.com",
      "type": "work",
      "primary": true
    }
  ],
  "active": true,
  "meta": {
    "version": "W/\"3\""
  }
}
```

**Response** (200 OK): Updated user resource with new version.

#### Update User (Partial)
```http
PATCH /scim/v2/{tenant-id}/Users/{id}
Content-Type: application/scim+json
If-Match: W/"3"
```

**Request Body**:
```json
{
  "schemas": ["urn:ietf:params:scim:api:messages:2.0:PatchOp"],
  "Operations": [
    {
      "op": "replace",
      "path": "name.familyName",
      "value": "Smith"
    },
    {
      "op": "add",
      "path": "emails",
      "value": {
        "value": "alice.personal@example.com",
        "type": "home"
      }
    }
  ]
}
```

**Response** (200 OK): Updated user resource.

#### Delete User
```http
DELETE /scim/v2/{tenant-id}/Users/{id}
If-Match: W/"3"
```

**Response** (204 No Content): Empty body.

#### List Users
```http
GET /scim/v2/{tenant-id}/Users
```

**Query Parameters**:
- `filter`: SCIM filter expression
- `sortBy`: Attribute to sort by
- `sortOrder`: `ascending` or `descending`
- `startIndex`: 1-based index (default: 1)
- `count`: Number of results (default: 100, max: 1000)
- `attributes`: Comma-separated list of attributes to return
- `excludedAttributes`: Comma-separated list of attributes to exclude

**Examples**:
```http
# Filter by email
GET /scim/v2/tenant-1/Users?filter=emails.value eq "alice@example.com"

# Sort by last modified
GET /scim/v2/tenant-1/Users?sortBy=meta.lastModified&sortOrder=descending

# Pagination
GET /scim/v2/tenant-1/Users?startIndex=51&count=50

# Select specific attributes
GET /scim/v2/tenant-1/Users?attributes=userName,emails,active
```

**Response** (200 OK):
```json
{
  "schemas": ["urn:ietf:params:scim:api:messages:2.0:ListResponse"],
  "totalResults": 150,
  "startIndex": 1,
  "itemsPerPage": 50,
  "Resources": [
    {
      "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
      "id": "2819c223-7f76-453a-919d-413861904646",
      "userName": "alice@example.com",
      "name": {
        "givenName": "Alice",
        "familyName": "Johnson"
      },
      "active": true,
      "meta": {
        "resourceType": "User",
        "created": "2023-12-01T10:30:00Z",
        "lastModified": "2023-12-01T15:45:00Z",
        "version": "W/\"3\""
      }
    }
  ]
}
```

### Groups

#### Create Group
```http
POST /scim/v2/{tenant-id}/Groups
Content-Type: application/scim+json
```

**Request Body**:
```json
{
  "schemas": ["urn:ietf:params:scim:schemas:core:2.0:Group"],
  "displayName": "Engineering Team",
  "members": [
    {
      "value": "2819c223-7f76-453a-919d-413861904646",
      "$ref": "../Users/2819c223-7f76-453a-919d-413861904646",
      "type": "User",
      "display": "Alice Johnson"
    }
  ]
}
```

**Response** (201 Created): Group resource with generated ID and metadata.

#### Get Group
```http
GET /scim/v2/{tenant-id}/Groups/{id}
```

#### Update Group
```http
PUT /scim/v2/{tenant-id}/Groups/{id}
PATCH /scim/v2/{tenant-id}/Groups/{id}
```

#### Delete Group
```http
DELETE /scim/v2/{tenant-id}/Groups/{id}
```

#### List Groups
```http
GET /scim/v2/{tenant-id}/Groups
```

Same query parameters and response format as Users.

## Discovery Endpoints

### Service Provider Configuration
```http
GET /scim/v2/{tenant-id}/ServiceProviderConfig
```

**Response** (200 OK):
```json
{
  "schemas": ["urn:ietf:params:scim:schemas:core:2.0:ServiceProviderConfig"],
  "documentationUri": "https://docs.example.com/scim",
  "patch": {
    "supported": true
  },
  "bulk": {
    "supported": true,
    "maxOperations": 1000,
    "maxPayloadSize": 1048576
  },
  "filter": {
    "supported": true,
    "maxResults": 1000
  },
  "changePassword": {
    "supported": false
  },
  "sort": {
    "supported": true
  },
  "etag": {
    "supported": true
  },
  "authenticationSchemes": [
    {
      "name": "OAuth Bearer Token",
      "description": "Authentication scheme using the OAuth Bearer Token Standard",
      "specUri": "http://www.rfc-editor.org/info/rfc6750",
      "documentationUri": "https://docs.example.com/auth",
      "type": "oauthbearertoken",
      "primary": true
    }
  ],
  "meta": {
    "resourceType": "ServiceProviderConfig",
    "created": "2023-12-01T00:00:00Z",
    "lastModified": "2023-12-01T00:00:00Z",
    "version": "W/\"1\""
  }
}
```

### Resource Types
```http
GET /scim/v2/{tenant-id}/ResourceTypes
```

**Response** (200 OK):
```json
{
  "schemas": ["urn:ietf:params:scim:api:messages:2.0:ListResponse"],
  "totalResults": 2,
  "startIndex": 1,
  "itemsPerPage": 2,
  "Resources": [
    {
      "schemas": ["urn:ietf:params:scim:schemas:core:2.0:ResourceType"],
      "id": "User",
      "name": "User",
      "endpoint": "/Users",
      "description": "User Account",
      "schema": "urn:ietf:params:scim:schemas:core:2.0:User",
      "schemaExtensions": [
        {
          "schema": "urn:ietf:params:scim:schemas:extension:enterprise:2.0:User",
          "required": false
        }
      ],
      "meta": {
        "resourceType": "ResourceType",
        "created": "2023-12-01T00:00:00Z",
        "lastModified": "2023-12-01T00:00:00Z",
        "version": "W/\"1\""
      }
    },
    {
      "schemas": ["urn:ietf:params:scim:schemas:core:2.0:ResourceType"],
      "id": "Group",
      "name": "Group",
      "endpoint": "/Groups",
      "description": "Group",
      "schema": "urn:ietf:params:scim:schemas:core:2.0:Group",
      "meta": {
        "resourceType": "ResourceType",
        "created": "2023-12-01T00:00:00Z",
        "lastModified": "2023-12-01T00:00:00Z",
        "version": "W/\"1\""
      }
    }
  ]
}
```

### Schemas
```http
GET /scim/v2/{tenant-id}/Schemas
```

**Response** (200 OK):
```json
{
  "schemas": ["urn:ietf:params:scim:api:messages:2.0:ListResponse"],
  "totalResults": 3,
  "startIndex": 1,
  "itemsPerPage": 3,
  "Resources": [
    {
      "schemas": ["urn:ietf:params:scim:schemas:core:2.0:Schema"],
      "id": "urn:ietf:params:scim:schemas:core:2.0:User",
      "name": "User",
      "description": "User Account",
      "attributes": [
        {
          "name": "userName",
          "type": "string",
          "multiValued": false,
          "description": "Unique identifier for the User",
          "required": true,
          "caseExact": false,
          "mutability": "readWrite",
          "returned": "default",
          "uniqueness": "server"
        }
      ],
      "meta": {
        "resourceType": "Schema",
        "created": "2023-12-01T00:00:00Z",
        "lastModified": "2023-12-01T00:00:00Z",
        "version": "W/\"1\""
      }
    }
  ]
}
```

## Bulk Operations

> **⚠️ Implementation Status**: Bulk operations are **not yet implemented** in this library.

### Future Bulk Endpoint (Not Available)
```http
POST /scim/v2/{tenant-id}/Bulk
Content-Type: application/scim+json
```

**Current Alternative**: Use individual API calls for each operation.

**Example**: Creating multiple users sequentially:
```http
POST /scim/v2/{tenant-id}/Users
Content-Type: application/scim+json

{
  "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
  "userName": "alice@example.com",
  "name": {
    "givenName": "Alice",
    "familyName": "Johnson"
  }
}
```

```http
POST /scim/v2/{tenant-id}/Users
Content-Type: application/scim+json

{
  "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
  "userName": "bob@example.com",
  "name": {
    "givenName": "Bob",
    "familyName": "Smith"
  }
}
```

**Future Bulk Request Format**:
```json
{
  "schemas": ["urn:ietf:params:scim:api:messages:2.0:BulkRequest"],
  "Operations": [
    {
      "method": "POST",
      "path": "/Users",
      "bulkId": "qwerty",
      "data": {
        "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
        "userName": "alice@example.com"
      }
    }
  ]
}
```

**Response** (200 OK):
```json
{
  "schemas": ["urn:ietf:params:scim:api:messages:2.0:BulkResponse"],
  "Operations": [
    {
      "method": "POST",
      "path": "/Users",
      "bulkId": "qwerty",
      "status": "201",
      "location": "https://api.example.com/scim/v2/tenant-1/Users/92b725cd-9465-4e7d-8c16-01f8e146b87a",
      "response": {
        "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
        "id": "92b725cd-9465-4e7d-8c16-01f8e146b87a",
        "userName": "alice@example.com",
        "name": {
          "givenName": "Alice",
          "familyName": "Johnson"
        },
        "meta": {
          "resourceType": "User",
          "created": "2023-12-01T16:30:00Z",
          "lastModified": "2023-12-01T16:30:00Z",
          "version": "W/\"1\""
        }
      }
    },
    {
      "method": "POST",
      "path": "/Groups",
      "bulkId": "ytrewq",
      "status": "201",
      "location": "https://api.example.com/scim/v2/tenant-1/Groups/e9e30dba-f08f-4109-8486-d5c6a331660a",
      "response": {
        "schemas": ["urn:ietf:params:scim:schemas:core:2.0:Group"],
        "id": "e9e30dba-f08f-4109-8486-d5c6a331660a",
        "displayName": "Administrators",
        "members": [
          {
            "value": "92b725cd-9465-4e7d-8c16-01f8e146b87a",
            "$ref": "../Users/92b725cd-9465-4e7d-8c16-01f8e146b87a",
            "type": "User",
            "display": "Alice Johnson"
          }
        ],
        "meta": {
          "resourceType": "Group",
          "created": "2023-12-01T16:30:00Z",
          "lastModified": "2023-12-01T16:30:00Z",
          "version": "W/\"1\""
        }
      }
    }
  ]
}
```

## Filter Expressions

SCIM supports rich filtering using a SQL-like syntax:

### Basic Operators
- `eq` - Equal
- `ne` - Not equal
- `co` - Contains
- `sw` - Starts with
- `ew` - Ends with
- `gt` - Greater than
- `ge` - Greater than or equal
- `lt` - Less than
- `le` - Less than or equal
- `pr` - Present (has value)

### Examples
```http
# Exact match
GET /Users?filter=userName eq "alice@example.com"

# Contains
GET /Users?filter=name.givenName co "Ali"

# Starts with
GET /Users?filter=userName sw "alice"

# Date comparison
GET /Users?filter=meta.lastModified gt "2023-01-01T00:00:00Z"

# Present check
GET /Users?filter=emails pr

# Complex expressions
GET /Users?filter=active eq true and (emails.type eq "work" or emails.type eq "primary")

# Nested attributes
GET /Users?filter=emails[type eq "work" and primary eq true].value eq "alice@work.com"
```

### Logical Operators
- `and` - Logical AND
- `or` - Logical OR
- `not` - Logical NOT

### Grouping
Use parentheses for complex expressions:
```http
GET /Users?filter=(name.givenName eq "Alice" or name.givenName eq "Bob") and active eq true
```

## HTTP Status Codes

### Success Codes
- `200 OK` - Successful GET, PUT, PATCH
- `201 Created` - Successful POST
- `204 No Content` - Successful DELETE
- `304 Not Modified` - Resource not modified (when using If-None-Match)

### Client Error Codes
- `400 Bad Request` - Invalid request syntax
- `401 Unauthorized` - Authentication required
- `403 Forbidden` - Insufficient permissions
- `404 Not Found` - Resource not found
- `409 Conflict` - Resource conflict (duplicate)
- `412 Precondition Failed` - ETag mismatch
- `413 Payload Too Large` - Request body too large
- `422 Unprocessable Entity` - Validation error

### Server Error Codes
- `500 Internal Server Error` - Server error
- `501 Not Implemented` - Feature not supported
- `503 Service Unavailable` - Server temporarily unavailable

## Error Response Format

All errors follow the SCIM error response format:

```json
{
  "schemas": ["urn:ietf:params:scim:api:messages:2.0:Error"],
  "status": "400",
  "scimType": "invalidFilter",
  "detail": "The specified filter syntax is invalid: unexpected token 'eq' at position 5"
}
```

### SCIM Error Types
- `invalidFilter` - Malformed filter expression
- `tooMany` - Query returned too many results
- `uniqueness` - Unique constraint violation
- `mutability` - Attempt to modify read-only attribute
- `invalidSyntax` - Malformed JSON or request
- `invalidPath` - Invalid attribute path in PATCH
- `noTarget` - PATCH target not found
- `invalidValue` - Invalid attribute value
- `invalidVers` - Invalid version in If-Match header
- `sensitive` - Cannot return sensitive attribute

## Headers

### Request Headers
- `Content-Type: application/scim+json` - Required for POST/PUT/PATCH
- `Authorization: Bearer <token>` - Authentication token
- `If-Match: W/"<version>"` - Conditional update
- `If-None-Match: W/"<version>"` - Conditional get

### Response Headers
- `Content-Type: application/scim+json` - SCIM JSON response
- `ETag: W/"<version>"` - Resource version
- `Location: <url>` - URL of created resource (201 responses)

## Custom Resource Endpoints

Custom resources follow the same patterns:

```http
# Custom Device resource
GET /scim/v2/{tenant-id}/Devices
POST /scim/v2/{tenant-id}/Devices
GET /scim/v2/{tenant-id}/Devices/{id}
PUT /scim/v2/{tenant-id}/Devices/{id}
PATCH /scim/v2/{tenant-id}/Devices/{id}
DELETE /scim/v2/{tenant-id}/Devices/{id}
```

## Rate Limiting

The server may implement rate limiting with these headers:

### Response Headers
- `X-RateLimit-Limit` - Requests per time window
- `X-RateLimit-Remaining` - Remaining requests
- `X-RateLimit-Reset` - Unix timestamp when limit resets

### Rate Limit Exceeded
```http
HTTP/1.1 429 Too Many Requests
X-RateLimit-Limit: 1000
X-RateLimit-Remaining: 0
X-RateLimit-Reset: 1699123200

{
  "schemas": ["urn:ietf:params:scim:api:messages:2.0:Error"],
  "status": "429",
  "detail": "Rate limit exceeded. Try again in 60 seconds."
}
```

## Best Practices

### Efficient Queries
- Use specific filters instead of retrieving all resources
- Use pagination for large result sets
- Request only needed attributes with `attributes` parameter
- Use ETag headers for caching and concurrency control

### Bulk Operations
- Use bulk operations for multiple changes
- Use pagination for large result sets with `count` and `startIndex` parameters
- Implement retry logic for individual failed operations

### Error Handling
- Check response status codes
- Parse SCIM error responses for detailed error information
- Implement retry logic for transient errors (5xx codes)
- Use exponential backoff for rate limiting

### Security
- Always use HTTPS in production
- Include proper Authorization headers
- Validate all input data
- Handle authentication errors gracefully

This comprehensive API reference covers all standard SCIM endpoints and patterns. For implementation examples, see the [Framework Integration](../tutorials/framework-integration.md) tutorial.