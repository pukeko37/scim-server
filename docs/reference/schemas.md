# Schema Reference

This reference provides comprehensive documentation for all SCIM schemas supported by the server, including their structure, attributes, and validation rules.

## Table of Contents

- [Overview](#overview)
- [Core Schemas](#core-schemas)
  - [User Schema](#user-schema)
  - [Group Schema](#group-schema)
  - [Service Provider Configuration Schema](#service-provider-configuration-schema)
- [Schema Structure](#schema-structure)
- [Attribute Types](#attribute-types)
- [Attribute Properties](#attribute-properties)
- [Custom Schemas](#custom-schemas)
- [Schema Validation](#schema-validation)

## Overview

SCIM schemas define the structure and validation rules for SCIM resources. Each schema consists of:

- **Schema ID**: A unique URI identifier for the schema
- **Name**: Human-readable name for the schema
- **Description**: Brief description of the schema's purpose
- **Attributes**: Collection of attribute definitions that define the resource structure

The SCIM server supports the core SCIM 2.0 schemas defined in RFC 7643, with extensibility for custom schemas.

## Core Schemas

### User Schema

**Schema ID**: `urn:ietf:params:scim:schemas:core:2.0:User`

The User schema represents individual user accounts with comprehensive profile information.

#### Core Attributes

| Attribute | Type | Required | Description |
|-----------|------|----------|-------------|
| `id` | string | No | Unique identifier for the user (server-generated) |
| `userName` | string | Yes | Unique identifier for the user (user-provided) |
| `externalId` | string | No | External system identifier |
| `name` | complex | No | User's full name components |
| `displayName` | string | No | Name for display purposes |
| `nickName` | string | No | Casual way to address the user |
| `profileUrl` | reference | No | URL to user's profile page |
| `title` | string | No | User's title |
| `userType` | string | No | Type of user (e.g., "Employee", "Contractor") |
| `preferredLanguage` | string | No | User's preferred language |
| `locale` | string | No | User's locale |
| `timezone` | string | No | User's timezone |
| `active` | boolean | No | Whether the user account is active |
| `password` | string | No | User's password (write-only) |
| `emails` | complex (multi-valued) | No | Email addresses |
| `phoneNumbers` | complex (multi-valued) | No | Phone numbers |
| `ims` | complex (multi-valued) | No | Instant messaging addresses |
| `photos` | complex (multi-valued) | No | Photo URLs |
| `addresses` | complex (multi-valued) | No | Physical addresses |
| `groups` | complex (multi-valued) | No | Group memberships (read-only) |
| `entitlements` | complex (multi-valued) | No | User entitlements |
| `roles` | complex (multi-valued) | No | User roles |
| `x509Certificates` | complex (multi-valued) | No | X.509 certificates |

#### Complex Type Structures

**Name Complex Type**:
- `formatted` (string): Full name for display
- `familyName` (string): Family/last name
- `givenName` (string): Given/first name
- `middleName` (string): Middle name
- `honorificPrefix` (string): Honorific prefix (e.g., "Dr.", "Mr.")
- `honorificSuffix` (string): Honorific suffix (e.g., "Jr.", "III")

**Multi-Valued Attribute Structure** (emails, phoneNumbers, etc.):
- `value` (string): The actual value
- `display` (string): Human-readable label
- `type` (string): Type of value (e.g., "work", "home")
- `primary` (boolean): Whether this is the primary value

**Address Complex Type**:
- `formatted` (string): Full address for display
- `streetAddress` (string): Street address
- `locality` (string): City or locality
- `region` (string): State or region
- `postalCode` (string): Postal/ZIP code
- `country` (string): Country
- `type` (string): Address type (e.g., "work", "home")
- `primary` (boolean): Whether this is the primary address

### Group Schema

**Schema ID**: `urn:ietf:params:scim:schemas:core:2.0:Group`

The Group schema represents collections of users and other groups.

#### Core Attributes

| Attribute | Type | Required | Description |
|-----------|------|----------|-------------|
| `id` | string | No | Unique identifier for the group (server-generated) |
| `displayName` | string | Yes | Human-readable name for the group |
| `externalId` | string | No | External system identifier |
| `members` | complex (multi-valued) | No | Group members (users and groups) |

#### Member Complex Type

**Members Structure**:
- `value` (string): Unique identifier of the member
- `$ref` (reference): URI reference to the member resource
- `type` (string): Type of member ("User" or "Group")
- `display` (string): Human-readable name of the member

### Service Provider Configuration Schema

**Schema ID**: `urn:ietf:params:scim:schemas:core:2.0:ServiceProviderConfig`

Defines the SCIM server's capabilities and configuration.

#### Core Attributes

| Attribute | Type | Required | Description |
|-----------|------|----------|-------------|
| `documentationUri` | reference | No | URI to service documentation |
| `patch` | complex | Yes | PATCH operation support configuration |
| `bulk` | complex | Yes | Bulk operation support configuration |
| `filter` | complex | Yes | Filtering support configuration |
| `changePassword` | complex | Yes | Password change support configuration |
| `sort` | complex | Yes | Sorting support configuration |
| `etag` | complex | Yes | ETag support configuration |
| `authenticationSchemes` | complex (multi-valued) | Yes | Supported authentication schemes |

## Schema Structure

Every schema follows this basic structure:

```json
{
  "id": "urn:ietf:params:scim:schemas:core:2.0:ResourceType",
  "name": "ResourceType",
  "description": "Description of the resource type",
  "attributes": [
    {
      "name": "attributeName",
      "type": "string|boolean|decimal|integer|datetime|reference|complex",
      "multiValued": false,
      "required": false,
      "caseExact": false,
      "mutability": "readWrite|readOnly|writeOnly|immutable",
      "returned": "always|never|default|request",
      "uniqueness": "none|server|global",
      "description": "Attribute description",
      "canonicalValues": [],
      "subAttributes": []
    }
  ]
}
```

## Attribute Types

### Simple Types

- **string**: Text data (UTF-8)
- **boolean**: True/false values
- **decimal**: Decimal numbers with arbitrary precision
- **integer**: Whole numbers
- **dateTime**: ISO 8601 date and time values
- **reference**: URI references to other resources

### Complex Types

- **complex**: Structured attributes with sub-attributes
- Can contain nested simple or complex types
- Defined by the `subAttributes` array

## Attribute Properties

### Mutability

Controls how attributes can be modified:

- **readWrite**: Can be read and written by clients
- **readOnly**: Can only be read, server-managed
- **writeOnly**: Can only be written, never returned (e.g., passwords)
- **immutable**: Can be set once, then becomes read-only

### Returned

Controls when attributes are returned in responses:

- **always**: Always returned in responses
- **never**: Never returned in responses
- **default**: Returned by default, can be excluded with attributes parameter
- **request**: Only returned when explicitly requested

### Uniqueness

Defines uniqueness constraints:

- **none**: No uniqueness constraint
- **server**: Unique within the server
- **global**: Globally unique across all SCIM servers

### Case Sensitivity

- **caseExact**: `true` for case-sensitive attributes
- **caseExact**: `false` for case-insensitive attributes

## Custom Schemas

The SCIM server supports custom schemas through schema extensions:

### Extension Schema Structure

Extension schemas follow the same structure as core schemas but use different schema IDs:

```json
{
  "id": "urn:ietf:params:scim:schemas:extension:enterprise:2.0:User",
  "name": "EnterpriseUser",
  "description": "Enterprise User Extension",
  "attributes": [
    {
      "name": "employeeNumber",
      "type": "string",
      "multiValued": false,
      "required": false,
      "caseExact": false,
      "mutability": "readWrite",
      "returned": "default",
      "uniqueness": "none"
    }
  ]
}
```

### Schema Loading

Schemas are loaded from the `schemas/` directory at server startup. Each schema file must:

1. Be a valid JSON file with `.json` extension
2. Contain a valid schema definition
3. Have a unique schema ID
4. Pass validation checks

### Schema Registry

The `SchemaRegistry` manages all loaded schemas and provides:

- Schema lookup by ID or resource type
- Resource validation against schemas
- Schema metadata access
- Validation context management

## Schema Validation

### Resource Validation

Resources are validated against their schemas during:

- **CREATE operations**: Full validation of all required attributes
- **UPDATE operations**: Validation of provided attributes
- **PATCH operations**: Validation of modified attributes

### Validation Rules

1. **Required Attributes**: Must be present for CREATE operations
2. **Type Validation**: Values must match the declared attribute type
3. **Mutability**: Write-only attributes cannot be read, read-only cannot be written
4. **Uniqueness**: Unique attributes must not conflict with existing resources
5. **Canonical Values**: String attributes with canonical values must use valid values
6. **Complex Type Structure**: Complex attributes must match sub-attribute definitions

### Validation Contexts

Different operations have different validation requirements:

- **Create**: All required attributes must be present
- **Replace**: Replaces entire resource, requires all required attributes
- **Modify**: Only validates provided attributes
- **Read**: No validation required

### Error Handling

Schema validation errors include detailed information:

- **Attribute Path**: Exact location of the validation error
- **Error Type**: Specific validation rule that failed
- **Expected Value**: What was expected vs. what was provided
- **Schema Context**: Which schema and attribute definition was used

## Example Usage

### Loading Schemas

```rust
use scim_server::schema::SchemaRegistry;

// Load schemas from directory
let registry = SchemaRegistry::from_schema_dir("schemas/")?;

// Get a specific schema
let user_schema = registry.get_schema("urn:ietf:params:scim:schemas:core:2.0:User")?;
```

### Validating Resources

```rust
use scim_server::schema::{SchemaRegistry, OperationContext};
use serde_json::json;

let registry = SchemaRegistry::new()?;

let user_data = json!({
    "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
    "userName": "jdoe@example.com",
    "name": {
        "givenName": "John",
        "familyName": "Doe"
    },
    "emails": [
        {
            "value": "john.doe@example.com",
            "type": "work",
            "primary": true
        }
    ]
});

// Validate for create operation
registry.validate_json_resource_with_context(
    "User", 
    &user_data, 
    OperationContext::Create
)?;
```

### Working with Attributes

```rust
use scim_server::schema::{AttributeType, Mutability};

let schema = registry.get_schema("urn:ietf:params:scim:schemas:core:2.0:User")?;

// Find a specific attribute
let username_attr = schema.attributes.iter()
    .find(|attr| attr.name == "userName")
    .unwrap();

// Check attribute properties
assert_eq!(username_attr.data_type, AttributeType::String);
assert_eq!(username_attr.mutability, Mutability::ReadWrite);
assert!(username_attr.required);
```

## Best Practices

### Schema Design

1. **Use Descriptive Names**: Choose clear, self-documenting attribute names
2. **Appropriate Types**: Select the most specific type that fits your data
3. **Mutability Planning**: Carefully consider which attributes should be read-only
4. **Uniqueness Constraints**: Apply uniqueness where business logic requires it
5. **Required vs Optional**: Mark attributes as required only when truly necessary

### Validation Strategy

1. **Early Validation**: Validate at the schema level before business logic
2. **Context-Aware**: Use appropriate operation contexts for validation
3. **Error Clarity**: Provide clear, actionable error messages
4. **Performance**: Cache schema lookups for high-frequency operations

### Extension Development

1. **Namespace URIs**: Use proper URN format for custom schema IDs
2. **Backward Compatibility**: Design extensions to be backward-compatible
3. **Documentation**: Document custom attributes thoroughly
4. **Testing**: Validate custom schemas extensively before deployment

## Schema Files Location

Schema definition files are located in the `schemas/` directory:

- `User.json` - Core User schema (RFC 7643)
- `Group.json` - Core Group schema (RFC 7643)
- `ServiceProviderConfig.json` - Service provider capabilities

## Related Documentation

- [API Reference - Core Types](../api/core-types.md) - Core type implementations
- [Configuration Guide](../guides/configuration.md) - Schema configuration options
- [Developer Guide](../guides/developer-guide.md) - Working with schemas in code
- [SCIM Compliance](scim-compliance.md) - SCIM 2.0 compliance details