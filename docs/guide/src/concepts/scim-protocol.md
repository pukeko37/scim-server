# SCIM Protocol Overview

Welcome to the SCIM (System for Cross-domain Identity Management) protocol! This chapter provides a comprehensive introduction to SCIM 2.0 and explains why it's essential for modern identity provisioning.

## What is SCIM?

SCIM is an open standard for automating the exchange of user identity information between identity domains or IT systems. Think of it as "REST for identity management" - it provides a standardized way to create, read, update, and delete user and group information across different systems.

### The Problem SCIM Solves

Before SCIM, organizations faced these challenges:

**Manual Provisioning**:
- IT administrators manually creating accounts in each system
- Human errors leading to security gaps
- Slow onboarding and offboarding processes

**Custom Integrations**:
- Each system had its own API for user management
- Expensive custom integration development
- Maintenance nightmares when systems changed

**Security Risks**:
- Orphaned accounts when employees left
- Inconsistent access control across systems
- No centralized audit trail

**SCIM's Solution**: A standardized protocol that enables automatic, secure, and consistent identity provisioning across all systems.

## SCIM 2.0 Core Concepts

### Resources

SCIM models identity information as **resources**. The two primary resource types are:

#### Users
Represent individual people with attributes like:
```json
{
  "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
  "id": "2819c223-7f76-453a-919d-413861904646",
  "userName": "bjensen@example.com",
  "name": {
    "formatted": "Ms. Barbara J Jensen III",
    "familyName": "Jensen",
    "givenName": "Barbara"
  },
  "emails": [
    {
      "value": "bjensen@example.com",
      "type": "work",
      "primary": true
    }
  ],
  "active": true
}
```

#### Groups
Represent collections of users:
```json
{
  "schemas": ["urn:ietf:params:scim:schemas:core:2.0:Group"],
  "id": "e9e30dba-f08f-4109-8486-d5c6a331660a",
  "displayName": "Tour Guides",
  "members": [
    {
      "value": "2819c223-7f76-453a-919d-413861904646",
      "$ref": "../Users/2819c223-7f76-453a-919d-413861904646",
      "display": "Barbara Jensen"
    }
  ]
}
```

### Schemas

SCIM uses **schemas** to define the structure and validation rules for resources. Every resource must declare which schemas it conforms to.

**Core Schemas**:
- `urn:ietf:params:scim:schemas:core:2.0:User` - Standard user attributes
- `urn:ietf:params:scim:schemas:core:2.0:Group` - Standard group attributes

**Extension Schemas**:
- `urn:ietf:params:scim:schemas:extension:enterprise:2.0:User` - Enterprise attributes (employee ID, manager, etc.)
- Custom schemas for organization-specific attributes

### Operations

SCIM defines standard HTTP operations for resource management:

| Operation | HTTP Method | Purpose | Status |
|-----------|-------------|---------|---------|
| **Create** | POST | Add new users or groups | ✅ Implemented ([`create_resource`](https://docs.rs/scim-server/latest/scim_server/trait.ResourceProvider.html#tymethod.create_resource)) |
| **Read** | GET | Retrieve specific resources | ✅ Implemented ([`get_resource`](https://docs.rs/scim-server/latest/scim_server/trait.ResourceProvider.html#tymethod.get_resource)) |
| **Replace/Update** | PUT/PATCH | Replace or modify resources | ✅ Implemented ([`update_resource`](https://docs.rs/scim-server/latest/scim_server/trait.ResourceProvider.html#tymethod.update_resource)) |
| **Delete** | DELETE | Remove resources | ✅ Implemented ([`delete_resource`](https://docs.rs/scim-server/latest/scim_server/trait.ResourceProvider.html#tymethod.delete_resource)) |
| **List** | GET | Query resources with filters | ✅ Implemented ([`list_resources`](https://docs.rs/scim-server/latest/scim_server/trait.ResourceProvider.html#tymethod.list_resources)) |
| **Search** | GET | Find by attribute | ✅ Implemented ([`find_resource_by_attribute`](https://docs.rs/scim-server/latest/scim_server/trait.ResourceProvider.html#tymethod.find_resource_by_attribute)) |
| **Bulk** | POST | Perform multiple operations | ❌ Not implemented (use individual calls) |

## SCIM Endpoints

The SCIM specification defines these standard endpoints. This library provides the business logic for these operations through the [`ResourceProvider`](https://docs.rs/scim-server/latest/scim_server/trait.ResourceProvider.html) trait - you'll need to map them to HTTP endpoints in your web framework:

### Resource Endpoints
```
GET    /Users                    # List all users (list_resources)
POST   /Users                    # Create new user (create_resource)
GET    /Users/{id}               # Get specific user (get_resource)
PUT    /Users/{id}               # Replace user (update_resource)
PATCH  /Users/{id}              # Update user (update_resource)
DELETE /Users/{id}              # Delete user (delete_resource)
```

> **Note**: This library provides the SCIM business logic layer. You'll need to integrate it with a web framework like [Axum](https://github.com/tokio-rs/axum), [Warp](https://github.com/seanmonstar/warp), or [Actix Web](https://actix.rs/) to handle HTTP requests and responses.

GET    /Groups                   # List all groups
POST   /Groups                   # Create new group
GET    /Groups/{id}              # Get specific group
PUT    /Groups/{id}              # Replace group
PATCH  /Groups/{id}              # Update group
DELETE /Groups/{id}              # Delete group
```

### Special Endpoints
```
GET    /ServiceProviderConfig    # Server capabilities
GET    /ResourceTypes            # Available resource types
GET    /Schemas                  # Schema definitions
POST   /Bulk                     # Bulk operations (not yet implemented)
```

## Filtering and Querying

SCIM provides powerful filtering capabilities for finding specific resources:

### Basic Filters
```
# Find users by email
GET /Users?filter=emails.value eq "bjensen@example.com"

# Find active users
GET /Users?filter=active eq true

# Find users by department
GET /Users?filter=department eq "Engineering"
```

### Complex Filters
```
# Multiple conditions
GET /Users?filter=active eq true and emails.type eq "work"

# Pattern matching
GET /Users?filter=userName sw "john"

# Date comparisons
GET /Users?filter=meta.lastModified gt "2023-01-01T00:00:00Z"
```

### Pagination
```
# Paginated results
GET /Users?startIndex=1&count=50

# Sorted results
GET /Users?sortBy=meta.lastModified&sortOrder=descending
```

## Versioning and Concurrency

SCIM uses **ETags** for optimistic concurrency control:

### Version Detection
```http
GET /Users/123
Response Headers:
ETag: "W/\"3694e05e9dff590\""
```

### Conditional Updates
```http
PUT /Users/123
If-Match: "W/\"3694e05e9dff590\""
```

If the resource was modified by someone else, the server returns `412 Precondition Failed`.

## Error Handling

SCIM defines standard error responses:

```json
{
  "schemas": ["urn:ietf:params:scim:api:messages:2.0:Error"],
  "status": "400",
  "scimType": "invalidFilter",
  "detail": "The specified filter syntax is invalid"
}
```

Common error types:
- `invalidFilter` - Malformed filter expression
- `tooMany` - Query returned too many results
- `uniqueness` - Unique constraint violation
- `mutability` - Attempt to modify read-only attribute

## Bulk Operations

> **⚠️ Implementation Status**: Bulk operations are **not yet implemented** in this library.

While the SCIM 2.0 specification includes bulk operations for efficiency, this library currently requires individual API calls for each operation:

```rust
use scim_server::{ResourceProvider, RequestContext};

// Current approach: Individual operations
async fn create_multiple_users(
    provider: &impl ResourceProvider,
    _tenant_id: &str,
    users: Vec<serde_json::Value>
) -> Result<Vec<serde_json::Value>, Box<dyn std::error::Error>> {
    let context = RequestContext::new("batch-create".to_string());
    let mut created_users = Vec::new();
    
    for user_data in users {
        let user = provider.create_resource("User", user_data, &context).await?;
        created_users.push(user);
    }
    
    Ok(created_users)
}
```

**Future Implementation**: The SCIM bulk endpoint specification would look like:

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

## Benefits of SCIM

### For Organizations
- **Automated Provisioning**: Eliminate manual account management
- **Security**: Consistent access control and rapid deprovisioning
- **Compliance**: Centralized audit trails and access reviews
- **Cost Reduction**: Reduce IT overhead and integration costs

### For Developers
- **Standardization**: One API to learn instead of dozens
- **Interoperability**: Works with existing identity providers
- **Type Safety**: Well-defined schemas prevent errors
- **Scalability**: Pagination support and planned bulk operations

### For Users
- **Faster Onboarding**: Immediate access to necessary systems
- **Self-Service**: Update profile information in one place
- **Better Experience**: Consistent identity across applications

## SCIM in Practice

### Common Use Cases

**Employee Lifecycle Management**:
- Automatically create accounts when employees join
- Update access when roles change
- Remove access when employees leave

**Application Integration**:
- Sync users from Active Directory to SaaS applications
- Provision groups based on organizational structure
- Maintain consistent user profiles across systems

**Compliance and Auditing**:
- Track all identity changes with timestamps
- Generate access reports for compliance reviews
- Ensure timely deprovisioning for security

### Integration Patterns

**Identity Provider → Applications**:
```
[Active Directory] → [SCIM Server] → [Slack, GitHub, Salesforce]
```

**HR System → Everything**:
```
[HR System] → [SCIM Server] → [All IT Systems]
```

**Federated Identity**:
```
[Company A SCIM] ←→ [Company B SCIM] (via secure federation)
```

## Why Choose SCIM Server Library?

While SCIM standardizes the protocol, implementation quality varies widely. This library provides:

### Enterprise-Grade Features
- **Multi-tenancy**: Isolate different organizations
- **Type Safety**: Prevent runtime errors with Rust's type system
- **Performance**: Async-first design with minimal overhead
- **Extensibility**: Easy schema customization and validation

### Developer Experience
- **Framework Agnostic**: Works with Axum, Warp, Actix, or custom HTTP
- **Rich Filtering**: Full SCIM filter expression support
- **Comprehensive Testing**: Battle-tested with extensive test suites
- **Clear Documentation**: This guide plus API documentation

### Production Ready
- **Concurrency Control**: Automatic ETag handling
- **Error Handling**: Comprehensive error types and messages
- **Monitoring**: Built-in observability hooks
- **Security**: Input validation and sanitization

## Next Steps

Now that you understand SCIM fundamentals, you're ready to:

1. **[Set up your development environment](../getting-started/installation.md)**
2. **[Build your first SCIM server](../getting-started/first-server.md)**
3. **[Learn the architecture](./architecture.md)** behind this library
4. **[Explore multi-tenancy](./multi-tenancy.md)** for enterprise deployments

The SCIM protocol provides the foundation for modern identity management. With this library, you can focus on your business logic while we handle the complex protocol details.

Ready to provision some identities? Let's get started! 🚀