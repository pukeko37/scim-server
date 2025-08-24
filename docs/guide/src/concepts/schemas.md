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

## Schema Mechanisms in SCIM Server

The SCIM Server library implements these schema concepts through several key mechanisms:

### Schema Registry

The `SchemaRegistry` component manages all schema definitions and provides validation services:

```scim-server/docs/guide/src/concepts/schemas.md#L200-220
use scim_server::schema::{SchemaRegistry, SchemaDefinition};

// Registry comes pre-loaded with RFC 7643 schemas
let registry = SchemaRegistry::default();

// Register custom schema
let custom_schema = SchemaDefinition {
    id: "urn:example:params:scim:schemas:extension:acme:2.0:User".to_string(),
    name: "AcmeUserExtension".to_string(),
    description: "Acme Corp user extensions".to_string(),
    attributes: vec![
        // ... attribute definitions
    ],
};

registry.register_schema(custom_schema).await?;

// Validate data against schema
let validation_result = registry.validate_resource("User", &user_data).await?;
```

**Key Features:**
- Pre-loaded with RFC 7643 core schemas
- Dynamic schema registration at runtime
- Comprehensive validation of resources against schemas
- Support for schema discovery and introspection

### Value Objects for Type Safety

The library uses value objects to provide type-safe handling of SCIM attributes:

```scim-server/docs/guide/src/concepts/schemas.md#L235-255
use scim_server::value_objects::{Email, UserName, DisplayName};

// Type-safe attribute creation
let email = Email::new("user@example.com")?;
let username = UserName::new("john.doe")?; 
let display_name = DisplayName::new("John Doe")?;

// Automatic validation
let invalid_email = Email::new("not-an-email"); // Returns validation error

// Schema-aware serialization
let json_value = email.to_json()?;

// Integration with complex attributes
let name = Name {
    formatted: Some(display_name),
    family_name: Some("Doe".to_string()),
    given_name: Some("John".to_string()),
    // ...
};
```

**Benefits:**
- Compile-time type safety for SCIM attributes
- Automatic validation of attribute values
- Prevention of invalid data construction
- Clear API boundaries and error handling

### Dynamic Schema Construction

For scenarios requiring runtime flexibility, the library supports dynamic value object creation:

```scim-server/docs/guide/src/concepts/schemas.md#L270-290
use scim_server::value_objects::{ValueObjectFactory, SchemaConstructible};

// Create value objects from schema definitions
let factory = ValueObjectFactory::new();

let attribute_def = registry.get_attribute_definition(
    "urn:ietf:params:scim:schemas:core:2.0:User", 
    "emails"
).await?;

let email_value = factory.create_from_schema(
    &attribute_def,
    &json!({"value": "user@example.com", "primary": true})
)?;

// Supports complex and multi-valued attributes
let name_value = factory.create_from_schema(
    &name_attribute_def,
    &json!({"formatted": "John Doe", "familyName": "Doe"})
)?;
```

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

let updated_user = provider.replace_resource("User", &user_id, replacement_data, &context).await?;
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

let result = provider.patch_resource("User", &user_id, patch_ops, &context).await?;
```

#### DELETE Operations - Schema-Informed Cleanup

```scim-server/docs/guide/src/concepts/schemas.md#L410-430
// DELETE /Users/{id} - Schema guides basic deletion processing
let deletion_result = provider.delete_resource("User", &user_id, &context).await?;

// Schema-informed deletion:
// - Validates deletion permissions based on mutability constraints
// - Processes extension schema cleanup requirements
// - Handles tenant isolation requirements

// Soft delete vs hard delete based on configuration
match deletion_result {
    DeletionResult::SoftDelete => {
        // User marked as active: false, preserved for audit
    },
    DeletionResult::HardDelete => {
        // Complete resource removal
    }
}
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

```scim-server/docs/guide/src/concepts/schemas.md#L585-605
// GET /Users?attributes=userName,emails&filter=active eq true
let query = ScimQuery {
    attributes: Some(vec!["userName".to_string(), "emails".to_string()]),
    excluded_attributes: None,
    filter: Some("active eq true".to_string()),
    sort_by: None,
    sort_order: None,
    start_index: Some(1),
    count: Some(10),
};

// Schema processing for queries:
// - Validates attribute names against schema definitions
// - Handles extension schema attributes in projections
// - Enforces "returned" attribute constraints
// - Processes complex attribute path expressions
// - Validates filter expressions against attribute types

let users = server.list_resources("User", &query, &context).await?;
```

## Auto Schema Discovery

SCIM Server provides automatic schema discovery capabilities that integrate with the SCIM protocol's introspection endpoints:

### Schema Endpoint Implementation

```scim-server/docs/guide/src/concepts/schemas.md#L620-640
let server = ScimServer::new(provider)?;

// GET /Schemas returns all registered schemas
let schemas = server.list_schemas(&context).await?;

// GET /Schemas/{schema_id} returns specific schema
let user_schema = server.get_schema(
    "urn:ietf:params:scim:schemas:core:2.0:User",
    &context
).await?;

// Automatic schema discovery for AI agents and clients
let schema_definitions = server.discover_schemas(&context).await?;
```

### Resource Type Discovery

The library also supports resource type introspection as defined in RFC 7644:

```scim-server/docs/guide/src/concepts/schemas.md#L650-670
// GET /ResourceTypes returns supported resource types
let resource_types = server.list_resource_types(&context).await?;

// Returns information like:
// {
//   "schemas": ["urn:ietf:params:scim:schemas:core:2.0:ResourceType"],
//   "id": "User",
//   "name": "User", 
//   "endpoint": "/Users",
//   "description": "User Account",
//   "schema": "urn:ietf:params:scim:schemas:core:2.0:User",
//   "schemaExtensions": [{
//     "schema": "urn:ietf:params:scim:schemas:extension:enterprise:2.0:User",
//     "required": false
//   }]
// }
```

## Dynamic Data Validation

The schema system enables sophisticated validation that goes beyond simple type checking:

### Multi-Level Validation

SCIM Server performs validation at multiple levels:

1. **HTTP Method Validation**: Operation-specific constraints
2. **Syntax Validation**: JSON structure and basic type checking
3. **Schema Validation**: Compliance with schema definitions
4. **Business Rule Validation**: Custom validation logic

```scim-server/docs/guide/src/concepts/schemas.md#L765-785
use scim_server::validation::{ValidationContext, ValidationResult, HttpOperation};

let validation_context = ValidationContext {
    schema_registry: &registry,
    resource_provider: &provider,
    request_context: &context,
    operation: HttpOperation::Post,  // Method-specific validation
};

// Comprehensive validation
let result = validation_context.validate_create_request(
    "User",
    &user_data
).await?;

match result {
    ValidationResult::Valid(normalized_data) => {
        // Data passed all validation checks
        let user = provider.create_resource("User", normalized_data, &context).await?;
    },
    ValidationResult::Invalid(errors) => {
        // Handle validation errors - includes HTTP status codes
        for error in errors {
            println!("Validation error: {} (HTTP {})", error.message, error.status_code);
        }
    }
}
```

### Operation-Specific Validation

Each HTTP operation has specific validation requirements:

```scim-server/docs/guide/src/concepts/schemas.md#L800-820
// POST (Create) - All required attributes must be present
{
  "name": "userName",
  "type": "string", 
  "required": true,                 // MUST be provided in POST
  "mutability": "readWrite"
}

// PUT (Replace) - Complete resource validation
{
  "name": "userName",
  "mutability": "immutable"         // Cannot be changed in PUT after creation
}

// PATCH (Update) - Path and operation validation  
{
  "name": "id",
  "mutability": "readOnly"          // Cannot be target of PATCH operations
}

// GET (Read) - Response filtering
{
  "name": "password",
  "returned": "never"               // Never included in GET responses
}
```

### HTTP Status Code Mapping

Schema validation errors map to specific HTTP status codes:

```scim-server/docs/guide/src/concepts/schemas.md#L835-855
use scim_server::error::{ScimError, ScimErrorType};

// Schema validation error examples
let validation_errors = vec![
    ScimError {
        error_type: ScimErrorType::InvalidValue,
        detail: "userName is required".to_string(),
        status: 400,  // Bad Request
    },
    ScimError {
        error_type: ScimErrorType::Uniqueness, 
        detail: "userName already exists".to_string(),
        status: 409,  // Conflict
    },
    ScimError {
        error_type: ScimErrorType::Mutability,
        detail: "id attribute is readOnly".to_string(), 
        status: 400,  // Bad Request
    },
    ScimError {
        error_type: ScimErrorType::InvalidPath,
        detail: "Invalid PATCH path: invalidAttribute".to_string(),
        status: 400,  // Bad Request
    }
];
```

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

```scim-server/docs/guide/src/concepts/schemas.md#L890-910
// POST /Users HTTP/1.1
// Content-Type: application/scim+json
// 
// {
//   "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
//   "userName": "newuser@example.com"
// }

// Server validates:
// - Content-Type header matches SCIM specification
// - schemas array matches Content-Type expectations
// - Resource structure conforms to declared schemas

let content_type = request.headers().get("content-type");
if content_type != Some("application/scim+json") {
    return Err(ScimError::invalid_syntax("Invalid Content-Type header"));
}
```

### ETag and Schema Versioning

The SCIM protocol uses ETags (entity tags) for optimistic concurrency control, preventing lost updates when multiple clients modify the same resource. Each SCIM resource includes a `meta.version` field containing an ETag value that changes whenever the resource is modified. Clients use HTTP conditional headers (`If-Match`, `If-None-Match`) with these ETags to ensure they're operating on the expected version of a resource. This mechanism integrates seamlessly with schema validation, as the ETag is updated only after successful schema validation and resource modification.

For a comprehensive explanation of ETag implementation, concurrency scenarios, and best practices, see the [ETag Concurrency Control](./etag-concurrency.md) chapter.

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

```scim-server/docs/guide/src/concepts/schemas.md#L975-995
// AI agents receive structured schema information
let schema_tools = server.generate_mcp_tools(&context).await?;

// Each tool includes:
// - HTTP operation specifications (GET, POST, PUT, PATCH, DELETE)
// - Input schema validation rules
// - Output format specifications  
// - Business rule constraints
// - Example usage patterns with proper HTTP methods

// AI can discover available schemas and their capabilities
let user_schema_info = schema_tools.get_schema_info("User")?;
// Returns: required fields, optional fields, validation rules, HTTP examples

// Example AI tool description:
{
  "name": "create_scim_user",
  "description": "Create a new SCIM user via POST /Users",
  "inputSchema": {
    "type": "object", 
    "properties": {
      "schemas": {"type": "array", "items": {"type": "string"}},
      "userName": {"type": "string", "description": "Required unique identifier"},
      "emails": {"type": "array", "description": "Email addresses"}
    },
    "required": ["schemas", "userName"]
  }
}
```

This enables AI systems to:
- Understand valid data formats automatically
- Generate compliant SCIM resources with proper HTTP methods
- Handle validation errors intelligently
- Discover available extensions and capabilities
- Execute proper HTTP operations (GET, POST, PUT, PATCH, DELETE)
- Process schema-aware query parameters and filters

## Conclusion

SCIM schemas provide a powerful foundation for identity data management that balances standardization with extensibility across HTTP REST operations. The SCIM Server library implements these concepts through:

- **Complete RFC 7643/7644 compliance** with pre-loaded core schemas and meta components
- **HTTP method-aware processing** that validates operations against schema constraints
- **Type-safe value objects** that prevent invalid data construction  
- **Flexible extension mechanisms** supporting custom business requirements
- **Comprehensive validation** at HTTP, syntax, schema, and business rule levels
- **Meta-schema support** for service provider configuration and resource type discovery
- **Dynamic discovery capabilities** for AI integration and client tooling with HTTP operation awareness
- **Production-ready performance** with efficient validation, caching, and ETag concurrency control

Key integration points include:

- **REST Operation Schema Processing**: Each HTTP method (GET, POST, PUT, PATCH, DELETE) has specific schema validation and processing requirements
- **Meta Components**: Service provider configuration, resource types, and schema introspection endpoints
- **Content Negotiation**: HTTP header processing for schema-aware content handling
- **Concurrency Control**: ETag-based versioning integrated with schema validation
- **Query Processing**: Schema-driven attribute filtering and search capabilities

By understanding these schema concepts and their implementation across HTTP operations, you can build robust identity provisioning systems that integrate seamlessly with enterprise identity providers while maintaining the flexibility to meet specific business requirements.

The next chapter will explore how to implement these schema concepts in practice through hands-on examples and common integration patterns.