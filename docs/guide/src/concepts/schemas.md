# Understanding SCIM Schemas

SCIM (System for Cross-domain Identity Management) uses a sophisticated schema system to define how identity data is structured, validated, and extended across HTTP REST operations. This chapter explores the schema-centric aspects of SCIM and how they're implemented in the SCIM Server library.

## SCIM Protocol Background

SCIM is defined by two key Internet Engineering Task Force (IETF) Request for Comments (RFC) specifications:

- **[RFC 7643: System for Cross-domain Identity Management (SCIM): Core Schema](https://tools.ietf.org/html/rfc7643)** - Defines the core schema and extension model for representing users and groups, published September 2015
- **[RFC 7644: System for Cross-domain Identity Management (SCIM): Protocol](https://tools.ietf.org/html/rfc7644)** - Specifies the REST API protocol for provisioning and managing identity data, published September 2015

These RFCs establish SCIM 2.0 as the industry standard for identity provisioning, providing a specification for automated user lifecycle management between identity providers (like Okta, Azure AD) and service providers (applications). The schema system defined in RFC 7643 forms the foundation for all SCIM operations, ensuring consistent data representation while allowing for extensibility to meet specific organizational requirements.

## What Are SCIM Schemas?

SCIM schemas define the structure and constraints for identity resources like Users and Groups across HTTP REST operations. They serve multiple purposes:

- **Data Structure Definition**: Define what attributes a resource can have
- **Validation Rules**: Specify required fields, data types, and constraints
- **HTTP Operation Context**: Guide validation and processing for GET, POST, PUT, PATCH, DELETE
- **Extensibility Framework**: Allow custom attributes while maintaining interoperability
- **API Contract**: Provide a machine-readable description of resource formats
- **Meta Components**: Define service provider capabilities and resource types

## Schema Structure Progression

SCIM uses a layered approach to schema definition, progressing from meta-schemas to concrete resource schemas.

### 1. Schema for Schemas (Meta-Schema)

At the foundation is the meta-schema that defines how schemas themselves are structured. This is defined in RFC 7643 Section 7 and includes attributes like:

```scim-server/docs/guide/src/concepts/schemas.md#L25-40
{
  "id": "urn:ietf:params:scim:schemas:core:2.0:Schema",
  "name": "Schema",
  "description": "Specifies the schema that describes a SCIM schema",
  "attributes": [
    {
      "name": "id",
      "type": "string",
      "multiValued": false,
      "description": "The unique URI of the schema",
      "required": true,
      "mutability": "readOnly"
    }
    // ... more meta-attributes
  ]
}
```

This meta-schema ensures consistency across all SCIM schema definitions and enables programmatic schema discovery and validation.

### 2. Core Resource Schemas

SCIM defines two core resource schemas that all compliant implementations must support:

#### User Schema (`urn:ietf:params:scim:schemas:core:2.0:User`)

The User schema defines standard attributes for representing people in identity systems:

```scim-server/docs/guide/src/concepts/schemas.md#L45-65
{
  "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
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
      "mutability": "readWrite",
      "returned": "default",
      "uniqueness": "server"
    },
    {
      "name": "name",
      "type": "complex",
      "multiValued": false,
      "description": "The components of the user's real name",
      "required": false,
      "subAttributes": [
        {
          "name": "formatted",
          "type": "string",
          "multiValued": false,
          "description": "The full name",
          "required": false
        },
        {
          "name": "familyName", 
          "type": "string",
          "multiValued": false,
          "description": "The family name",
          "required": false
        }
        // ... more name components
      ]
    }
    // ... more user attributes
  ]
}
```

**Key User Attributes:**
- `userName`: Unique identifier (required)
- `name`: Complex type with formatted, family, and given names
- `emails`: Multi-valued array of email addresses
- `active`: Boolean indicating account status
- `groups`: Multi-valued references to group memberships

#### Group Schema (`urn:ietf:params:scim:schemas:core:2.0:Group`)

The Group schema defines attributes for representing collections of users:

```scim-server/docs/guide/src/concepts/schemas.md#L90-110
{
  "schemas": ["urn:ietf:params:scim:schemas:core:2.0:Group"],
  "id": "urn:ietf:params:scim:schemas:core:2.0:Group", 
  "name": "Group",
  "description": "Group",
  "attributes": [
    {
      "name": "displayName",
      "type": "string",
      "multiValued": false,
      "description": "A human-readable name for the Group",
      "required": true,
      "mutability": "readWrite"
    },
    {
      "name": "members",
      "type": "complex",
      "multiValued": true,
      "description": "A list of members of the Group",
      "required": false,
      "subAttributes": [
        {
          "name": "value",
          "type": "string", 
          "multiValued": false,
          "description": "Identifier of the member",
          "mutability": "immutable"
        },
        {
          "name": "$ref",
          "type": "reference",
          "referenceTypes": ["User", "Group"],
          "multiValued": false,
          "description": "The URI of the member resource"
        }
      ]
    }
  ]
}
```

**Key Group Attributes:**
- `displayName`: Human-readable group name (required)
- `members`: Multi-valued complex attribute containing user/group references

### 3. Schema Specialization and Extensions

SCIM's extensibility model allows organizations to add custom attributes while maintaining core compatibility.

#### Enterprise User Extension

RFC 7643 defines a standard extension for enterprise environments:

```scim-server/docs/guide/src/concepts/schemas.md#L140-160
{
  "schemas": [
    "urn:ietf:params:scim:schemas:core:2.0:User",
    "urn:ietf:params:scim:schemas:extension:enterprise:2.0:User"
  ],
  "userName": "john.doe",
  "emails": [{"value": "john@example.com", "primary": true}],
  "urn:ietf:params:scim:schemas:extension:enterprise:2.0:User": {
    "employeeNumber": "12345",
    "department": "Engineering", 
    "manager": {
      "value": "26118915-6090-4610-87e4-49d8ca9f808d",
      "$ref": "../Users/26118915-6090-4610-87e4-49d8ca9f808d",
      "displayName": "Jane Smith"
    },
    "organization": "Acme Corp"
  }
}
```

#### Custom Schema Extensions

Organizations can define completely custom schemas using proper URN namespacing:

```scim-server/docs/guide/src/concepts/schemas.md#L170-190
{
  "schemas": [
    "urn:ietf:params:scim:schemas:core:2.0:User",
    "urn:example:params:scim:schemas:extension:acme:2.0:User"
  ],
  "userName": "alice.engineer",
  "urn:example:params:scim:schemas:extension:acme:2.0:User": {
    "securityClearance": "SECRET",
    "projectAssignments": [
      {
        "projectId": "PROJ-001",
        "role": "Lead Developer",
        "startDate": "2024-01-15"
      }
    ],
    "skills": ["rust", "scim", "identity-management"]
  }
}
```

## Schema Processing in SCIM Operations

SCIM schemas drive all protocol operations, providing structure and validation rules that ensure consistent data handling across different identity providers and service providers.

## HTTP REST Operations and Schema Processing

SCIM schemas are deeply integrated with HTTP REST operations. Each SCIM command has specific schema processing requirements:

### Schema-Aware HTTP Operations

#### GET Operations - Schema-Driven Response Formation

```scim-server/docs/guide/src/concepts/schemas.md#L300-320
// GET /Users/{id} - Schema determines response structure
let user = provider.get_resource("User", &user_id, &context).await?;

// Schema controls:
// - Which attributes are returned by default
// - Attribute mutability affects response inclusion
// - Extension schemas determine namespace organization
// - "returned" attribute property: "always", "never", "default", "request"

{
  "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
  "id": "2819c223-7f76-453a-919d-413861904646",
  "userName": "bjensen@example.com",  // returned: "default"
  "meta": {                           // Always included per spec
    "resourceType": "User",
    "created": "2010-01-23T04:56:22Z",
    "lastModified": "2011-05-13T04:42:34Z",
    "version": "W/\"3694e05e9dff591\"",
    "location": "https://example.com/v2/Users/2819c223..."
  }
}
```

#### POST Operations - Schema Validation on Creation

```scim-server/docs/guide/src/concepts/schemas.md#L325-345
// POST /Users - Complete resource creation with schema validation
let create_request = json!({
    "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
    "userName": "newuser@example.com",  // Required per schema
    "emails": [{"value": "newuser@example.com", "primary": true}]
});

// Schema validation includes:
// - Required attribute presence check
// - Data type validation
// - Uniqueness constraint enforcement
// - Mutability rules ("readOnly" attributes rejected)
// - Extension schema validation

let user = provider.create_resource("User", create_request, &context).await?;
```

#### PUT Operations - Complete Resource Replacement

```scim-server/docs/guide/src/concepts/schemas.md#L350-370
// PUT /Users/{id} - Schema ensures complete resource validity
let replacement_data = json!({
    "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
    "userName": "bjensen@example.com",  // Must include all required fields
    "emails": [{"value": "bjensen@newdomain.com", "primary": true}],
    "active": true
});

// Schema processing for PUT:
// - Validates complete resource structure
// - Ensures all required attributes present
// - Respects "immutable" and "readOnly" constraints
// - Processes all registered extension schemas
```

#### PATCH Operations - Partial Updates with Schema Context

```scim-server/docs/guide/src/concepts/schemas.md#L375-395
use scim_server::patch::{PatchOperation, PatchOp};

// PATCH /Users/{id} - Schema-aware partial modifications
let patch_ops = vec![
    PatchOperation {
        op: PatchOp::Replace,
        path: Some("emails[primary eq true].value".to_string()),
        value: Some(json!("newemail@example.com")),
    },
    PatchOperation {
        op: PatchOp::Add,
        path: Some("urn:ietf:params:scim:schemas:extension:enterprise:2.0:User:department".to_string()),
        value: Some(json!("Engineering")),
    }
];

// Schema validation for PATCH:
// - Path expression validation against schema structure
// - Target attribute mutability checking
// - Extension schema awareness for namespaced paths
// - Multi-valued attribute handling per schema rules
```

#### DELETE Operations - Schema-Informed Cleanup

```scim-server/docs/guide/src/concepts/schemas.md#L410-430
// DELETE /Users/{id} - Schema guides deletion processing
// Schema-informed deletion:
// - Validates deletion permissions based on mutability constraints
// - Processes extension schema cleanup requirements
// - Determines soft delete vs hard delete approach
```

## SCIM Meta Components and Schema Integration

SCIM defines several meta-schemas that describe the service provider's capabilities and resource structures:

### Service Provider Configuration Schema

The ServiceProviderConfig resource describes server capabilities:

```scim-server/docs/guide/src/concepts/schemas.md#L445-465
{
  "schemas": ["urn:ietf:params:scim:schemas:core:2.0:ServiceProviderConfig"],
  "documentationUri": "https://example.com/help/scim.html",
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
    "maxResults": 200
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
      "type": "oauthbearertoken",
      "name": "OAuth Bearer Token",
      "description": "Authentication scheme using the OAuth Bearer Token",
      "specUri": "http://www.rfc-editor.org/info/rfc6750",
      "documentationUri": "https://example.com/help/oauth.html"
    }
  ]
}
```

### Resource Type Meta-Schema

Resource types describe available SCIM resources and their schemas:

```scim-server/docs/guide/src/concepts/schemas.md#L485-505
// GET /ResourceTypes/User returns:
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
    },
    {
      "schema": "urn:example:params:scim:schemas:extension:acme:2.0:User", 
      "required": true
    }
  ],
  "meta": {
    "location": "https://example.com/v2/ResourceTypes/User",
    "resourceType": "ResourceType"
  }
}
```

### Schema Meta-Attributes

Every SCIM resource includes meta-attributes that support HTTP operations:

```scim-server/docs/guide/src/concepts/schemas.md#L520-540
{
  "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
  "id": "2819c223-7f76-453a-919d-413861904646",
  "userName": "bjensen@example.com",
  "meta": {
    "resourceType": "User",                              // Schema-derived type
    "created": "2010-01-23T04:56:22Z",                  // Creation timestamp
    "lastModified": "2011-05-13T04:42:34Z",             // Modification tracking
    "location": "https://example.com/v2/Users/2819c223-7f76-453a-919d-413861904646",
    "version": "W/\"3694e05e9dff591\""                  // ETag for concurrency control
  }
}

// Meta attributes enable:
// - HTTP caching with ETags
// - Conditional operations (If-Match, If-None-Match) 
// - Audit trail capabilities
// - Resource location for HATEOAS compliance
```

## Schema Processing in SCIM Server Implementation

The SCIM Server library integrates schema processing throughout the HTTP request lifecycle:

### Request Processing Pipeline

```scim-server/docs/guide/src/concepts/schemas.md#L555-575
use scim_server::{ScimServer, RequestContext};

let server = ScimServer::new(provider)?;

// 1. HTTP Request → Schema Identification
let schemas = extract_schemas_from_request(&request_body)?;

// 2. Schema Validation → Resource Processing  
let validation_context = ValidationContext {
    schemas: &schemas,
    operation: HttpOperation::Post,
    resource_type: "User",
};

// 3. Operation Execution with Schema Constraints
let result = match request.method() {
    "GET" => server.get_with_schema_filtering(&resource_type, &id, &query_params, &context).await?,
    "POST" => server.create_with_schema_validation(&resource_type, &request_body, &context).await?,
    "PUT" => server.replace_with_schema_validation(&resource_type, &id, &request_body, &context).await?,
    "PATCH" => server.patch_with_schema_awareness(&resource_type, &id, &patch_ops, &context).await?,
    "DELETE" => server.delete_with_schema_cleanup(&resource_type, &id, &context).await?,
};
```

### Schema-Driven Query Processing

SCIM query parameters interact directly with schema definitions:

```
GET /Users?attributes=userName,emails&filter=active eq true
```

Schema processing for queries:
- Validates attribute names against schema definitions
- Handles extension schema attributes in projections
- Enforces "returned" attribute constraints
- Processes complex attribute path expressions
- Validates filter expressions against attribute types

## Auto Schema Discovery

SCIM Server provides automatic schema discovery capabilities that integrate with the SCIM protocol's introspection endpoints:

### Schema Endpoint Implementation

SCIM servers provide schema introspection endpoints:

- `GET /Schemas` returns all registered schemas
- `GET /Schemas/{schema_id}` returns specific schema details
- Enables automatic schema discovery for clients and tools

### Resource Type Discovery

SCIM servers support resource type introspection as defined in RFC 7644:

`GET /ResourceTypes` returns supported resource types, including:
- Resource endpoint paths
- Associated schema URNs  
- Available schema extensions
- Extension requirement status

## Dynamic Data Validation

The schema system enables sophisticated validation that goes beyond simple type checking:

### Multi-Level Validation

SCIM validation occurs at multiple levels:

1. **HTTP Method Validation**: Operation-specific constraints
2. **Syntax Validation**: JSON structure and basic type checking
3. **Schema Validation**: Compliance with schema definitions
4. **Business Rule Validation**: Custom validation logic

The validation process ensures that data conforms to schema requirements before processing, returning appropriate HTTP status codes for different error types.

### Operation-Specific Validation

Each HTTP operation has specific validation requirements based on schema attribute properties:

- **POST (Create)**: All required attributes must be present
- **PUT (Replace)**: Complete resource validation, immutable attributes cannot change
- **PATCH (Update)**: Path validation, readOnly attributes cannot be targeted
- **GET (Read)**: Response filtering based on "returned" attribute property

### HTTP Status Code Mapping

Schema validation errors map to specific HTTP status codes:

- **400 Bad Request**: Invalid values, missing required attributes, mutability violations
- **409 Conflict**: Uniqueness constraint violations
- **412 Precondition Failed**: ETag version mismatches

## Working with Standard Data Definitions

SCIM defines standard data formats and constraints that the library enforces:

### Attribute Types and Constraints

| Type | Description | Validation Rules |
|------|-------------|------------------|
| `string` | Text data | Length limits, case sensitivity, uniqueness |
| `boolean` | True/false values | Must be valid JSON boolean |
| `decimal` | Numeric data | Precision and scale constraints |
| `integer` | Whole numbers | Range validation |
| `dateTime` | ISO 8601 timestamps | Format and timezone validation |
| `binary` | Base64-encoded data | Encoding validation |
| `reference` | Resource references | Referential integrity checks |
| `complex` | Nested objects | Sub-attribute validation |

### Multi-Valued Attributes

SCIM supports multi-valued attributes with sophisticated handling:

```scim-server/docs/guide/src/concepts/schemas.md#L450-470
{
  "emails": [
    {
      "value": "primary@example.com",
      "type": "work", 
      "primary": true
    },
    {
      "value": "secondary@example.com",
      "type": "personal",
      "primary": false
    }
  ]
}
```

**Multi-Value Rules:**
- At most one `primary` value allowed
- Type values from canonical list (if specified)
- Duplicate detection and handling
- Order preservation for client expectations

### Reference Attributes

References to other SCIM resources are handled with full integrity checking:

```scim-server/docs/guide/src/concepts/schemas.md#L485-505
{
  "groups": [
    {
      "value": "e9e30dba-f08f-4109-8486-d5c6a331660a",
      "$ref": "https://example.com/scim/v2/Groups/e9e30dba-f08f-4109-8486-d5c6a331660a",
      "display": "Administrators"
    }
  ]
}

// The library validates:
// - Reference type matches schema constraints
// - Tenant isolation for multi-tenant systems
```

## HTTP Content Negotiation and Schema Processing

SCIM servers use HTTP headers to negotiate schema processing:

### Content-Type and Schema Validation

SCIM requests must include proper Content-Type headers and schema declarations:

```
POST /Users HTTP/1.1
Content-Type: application/scim+json

{
  "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
  "userName": "newuser@example.com"
}
```

Servers validate:
- Content-Type header matches SCIM specification (`application/scim+json`)
- schemas array matches Content-Type expectations
- Resource structure conforms to declared schemas

### ETag and Schema Versioning

The SCIM protocol uses ETags (entity tags) for optimistic concurrency control, preventing lost updates when multiple clients modify the same resource. Each SCIM resource includes a `meta.version` field containing an ETag value that changes whenever the resource is modified. Clients use HTTP conditional headers (`If-Match`, `If-None-Match`) with these ETags to ensure they're operating on the expected version of a resource.

For implementation details and practical usage patterns, see the [Concurrency Control in SCIM Operations](./concurrency.md) chapter and [Schema Mechanisms in SCIM Server](./schema-mechanisms.md).

## Schema Extensibility Patterns

The SCIM Server supports several patterns for extending schemas while maintaining interoperability:

### Additive Extensions

Add new attributes without modifying core schemas:

```scim-server/docs/guide/src/concepts/schemas.md#L520-540
// Core user data remains unchanged
{
  "schemas": [
    "urn:ietf:params:scim:schemas:core:2.0:User",
    "urn:example:params:scim:schemas:extension:acme:2.0:User"
  ],
  "userName": "alice.engineer",
  "emails": [{"value": "alice@acme.com", "primary": true}],
  
  // Extension data in separate namespace
  "urn:example:params:scim:schemas:extension:acme:2.0:User": {
    "department": "R&D",
    "clearanceLevel": "SECRET",
    "projects": ["moonshot", "widget-2.0"]
  }
}
```

### Schema Composition

Combine multiple extensions for complex scenarios:

```scim-server/docs/guide/src/concepts/schemas.md#L550-570
{
  "schemas": [
    "urn:ietf:params:scim:schemas:core:2.0:User",
    "urn:ietf:params:scim:schemas:extension:enterprise:2.0:User", 
    "urn:example:params:scim:schemas:extension:security:2.0:User",
    "urn:example:params:scim:schemas:extension:hr:2.0:User"
  ],
  "userName": "bob.manager",
  
  // Each extension provides its own attributes
  "urn:ietf:params:scim:schemas:extension:enterprise:2.0:User": {
    "employeeNumber": "E12345"
  },
  "urn:example:params:scim:schemas:extension:security:2.0:User": {
    "lastSecurityReview": "2024-01-15T10:30:00Z"
  },
  "urn:example:params:scim:schemas:extension:hr:2.0:User": {
    "performanceRating": "exceeds-expectations"
  }
}
```

## Best Practices for Schema Design

When designing custom schemas for use with SCIM Server:

### 1. Follow Naming Conventions

- Use proper URN namespacing: `urn:example:params:scim:schemas:extension:company:version:Type`
- Choose descriptive attribute names that clearly indicate purpose
- Use camelCase for attribute names to match SCIM conventions

### 2. Design for Interoperability

- Minimize custom types - prefer standard SCIM types when possible
- Document extensions clearly for integration partners
- Provide sensible defaults and make attributes optional when appropriate

### 3. Consider Performance Implications

- Avoid deeply nested complex attributes that are expensive to validate
- Use appropriate uniqueness constraints to leverage database indexes
- Consider query patterns when designing multi-valued attributes

### 4. Plan for Evolution

- Design extensible schemas that can accommodate future requirements
- Use semantic versioning for schema URNs
- Maintain backwards compatibility when modifying existing schemas

## Integration with AI Systems

The SCIM Server's schema system is designed to work seamlessly with AI agents through the Model Context Protocol (MCP):

### Schema-Aware AI Tools

SCIM schemas enable AI integration by providing structured descriptions of available operations and data formats. AI systems can discover schema capabilities and generate compliant requests automatically.

Schema information that benefits AI systems includes:
- Required and optional attributes for each resource type
- Validation rules and data format constraints
- Available HTTP operations and their requirements
- Extension schemas and custom attribute definitions

For implementation details, see [Schema Mechanisms in SCIM Server](./schema-mechanisms.md) and the [AI Integration Guide](../advanced/ai-integration.md).

## Conclusion

## Conclusion

SCIM schemas provide a powerful foundation for identity data management that balances standardization with extensibility across HTTP REST operations. Understanding these protocol concepts enables you to:

- **Design compliant SCIM resources** that work with enterprise identity providers
- **Implement proper validation** across all HTTP operations
- **Create extensible systems** using schema extension mechanisms
- **Build interoperable solutions** that follow RFC specifications
- **Support diverse client requirements** through schema discovery

Key takeaways include:

- **Schema Structure**: Schemas define both data structure and operational behavior
- **HTTP Integration**: Each REST operation has specific schema processing requirements
- **Extensibility**: Extension schemas enable customization while maintaining compatibility
- **Validation**: Multi-layered validation ensures data integrity and compliance
- **Discovery**: Schema endpoints enable dynamic client capabilities

For practical implementation of these concepts using the SCIM Server library, see [Schema Mechanisms in SCIM Server](./schema-mechanisms.md).

The next chapter will explore hands-on implementation patterns and real-world usage scenarios.