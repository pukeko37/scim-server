# SCIM Schema Files Documentation

This document describes the file-based schema system implemented in the SCIM server library. Schemas are now defined in external JSON files, making them easily customizable and maintainable.

## Overview

The SCIM server library loads schema definitions from JSON files at startup. This approach provides several benefits:

- **Easy Customization**: Modify schemas without changing code
- **Version Control**: Track schema changes alongside code
- **Validation**: Schemas can be validated independently
- **Extensibility**: Add custom attributes and resource types
- **Standards Compliance**: JSON format follows SCIM specification

## Schema File Format

Schema files are JSON documents that follow the SCIM specification format (RFC 7643). Each file defines a complete resource schema with metadata and attribute definitions.

### Basic Structure

```json
{
  "id": "urn:ietf:params:scim:schemas:core:2.0:User",
  "name": "User",
  "description": "User Account",
  "attributes": [
    {
      "name": "userName",
      "type": "string",
      "multiValued": false,
      "required": true,
      "caseExact": false,
      "mutability": "readWrite",
      "returned": "default",
      "uniqueness": "server"
    }
  ]
}
```

### Required Fields

#### Schema Level
- `id`: Unique schema identifier (URI format)
- `name`: Human-readable schema name
- `description`: Schema description
- `attributes`: Array of attribute definitions

#### Attribute Level
- `name`: Attribute name
- `type`: Data type (see supported types below)
- `multiValued`: Boolean indicating if attribute can have multiple values
- `required`: Boolean indicating if attribute is required
- `caseExact`: Boolean for case-sensitive string comparisons
- `mutability`: How the attribute can be modified
- `returned`: When the attribute is returned in responses
- `uniqueness`: Uniqueness constraint level

### Supported Data Types

| Type | Description | Example |
|------|-------------|---------|
| `string` | Text value | `"John Doe"` |
| `boolean` | True/false value | `true` |
| `integer` | Whole number | `42` |
| `decimal` | Floating point number | `3.14` |
| `dateTime` | RFC3339 timestamp | `"2023-01-01T00:00:00Z"` |
| `binary` | Base64 encoded data | `"SGVsbG8gV29ybGQ="` |
| `reference` | URI reference | `"https://example.com/Users/123"` |
| `complex` | Object with sub-attributes | `{"value": "...", "type": "..."}` |

### Mutability Values

- `readOnly`: Managed by server, cannot be modified by clients
- `readWrite`: Can be modified by clients
- `immutable`: Set once during creation, cannot be changed
- `writeOnly`: Can be written but not read (e.g., passwords)

### Uniqueness Values

- `none`: No uniqueness constraint
- `server`: Unique within the server
- `global`: Globally unique

### Returned Values

- `always`: Always returned in responses
- `never`: Never returned in responses
- `default`: Returned by default
- `request`: Returned only when explicitly requested

## File Naming Convention

Schema files should be named after the schema name with a `.json` extension:

- `User.json` - Core User schema
- `Group.json` - Core Group schema
- `CustomUser.json` - Custom User extension
- `ServiceProviderConfig.json` - Service provider configuration

## Standard Schema Files

### User.json (Required)

The core User schema as defined in RFC 7643. Contains standard attributes like:

- `userName` (required)
- `displayName`
- `name` (complex with givenName, familyName, etc.)
- `emails` (multi-valued complex)
- `phoneNumbers` (multi-valued complex)
- `active`
- `meta` (system metadata)

### ServiceProviderConfig.json (Optional)

Defines the service provider configuration schema with capabilities like:

- `patch` - PATCH operation support
- `bulk` - Bulk operation support
- `filter` - Filtering support
- `changePassword` - Password change support
- `sort` - Sorting support
- `etag` - ETag support
- `authenticationSchemes` - Supported authentication methods

## Loading Schemas

### Default Behavior

```rust
// Loads schemas from current directory
let server = ScimServer::builder()
    .with_resource_provider(provider)
    .build()?;
```

### Custom Schema Directory

```rust
// Loads schemas from custom directory
let server = ScimServer::builder()
    .with_resource_provider(provider)
    .with_schema_dir("./schemas")
    .build()?;
```

### Error Handling

Schema loading errors are reported during server building:

```rust
match ScimServer::builder()
    .with_resource_provider(provider)
    .build() 
{
    Ok(server) => {
        // Server ready with loaded schemas
    }
    Err(BuildError::SchemaLoadError { schema_id }) => {
        eprintln!("Failed to load schema: {}", schema_id);
    }
}
```

## Complex Attributes

Complex attributes contain sub-attributes and are used for structured data:

```json
{
  "name": "emails",
  "type": "complex",
  "multiValued": true,
  "subAttributes": [
    {
      "name": "value",
      "type": "string",
      "required": true
    },
    {
      "name": "type",
      "type": "string",
      "canonicalValues": ["work", "home", "other"]
    },
    {
      "name": "primary",
      "type": "boolean"
    }
  ]
}
```

## Canonical Values

String attributes can specify allowed values:

```json
{
  "name": "department",
  "type": "string",
  "canonicalValues": ["Engineering", "Marketing", "Sales", "HR"]
}
```

## Custom Schema Example

Here's an example of a custom enterprise User schema:

```json
{
  "id": "urn:example:params:scim:schemas:extension:2.0:EnterpriseUser",
  "name": "EnterpriseUser",
  "description": "Enterprise User with additional attributes",
  "attributes": [
    {
      "name": "employeeId",
      "type": "string",
      "required": true,
      "uniqueness": "server"
    },
    {
      "name": "department",
      "type": "string",
      "canonicalValues": ["Engineering", "Marketing", "Sales"]
    },
    {
      "name": "manager",
      "type": "complex",
      "subAttributes": [
        {
          "name": "value",
          "type": "string",
          "required": true
        },
        {
          "name": "displayName",
          "type": "string"
        }
      ]
    }
  ]
}
```

## Schema Validation

Use the included validation tool to check schema files:

```bash
# Validate a single schema file
cargo run --bin validate-schema User.json

# Validate all schemas in a directory
cargo run --bin validate-schema ./schemas/

# Validate current directory
cargo run --bin validate-schema .
```

The validator checks:

- JSON syntax and structure
- Required fields presence
- Attribute type consistency
- Sub-attribute validation for complex types
- Canonical values for string types
- Schema ID format (should be URI)

## Best Practices

### Schema Design

1. **Use descriptive IDs**: Follow URI format with your organization's namespace
2. **Meaningful names**: Use clear, descriptive attribute names
3. **Required fields**: Only mark truly required attributes as required
4. **Canonical values**: Use for controlled vocabularies
5. **Case sensitivity**: Set `caseExact` appropriately for string attributes

### File Organization

1. **Separate files**: One schema per file
2. **Clear naming**: Use schema name as filename
3. **Version control**: Track schema changes in git
4. **Documentation**: Document custom attributes and their purpose

### Validation

1. **Test schemas**: Use the validation tool before deployment
2. **Validate data**: Ensure existing data conforms to schema changes
3. **Backward compatibility**: Consider compatibility when modifying schemas

### Error Handling

1. **Graceful failures**: Handle schema loading errors appropriately
2. **Clear messages**: Provide helpful error messages for validation failures
3. **Fallback options**: Consider fallback behaviors for missing schemas

## Migration from Hardcoded Schemas

If migrating from hardcoded schemas:

1. **Export existing schemas**: Create JSON files from current definitions
2. **Validate exported schemas**: Use validation tool to ensure correctness
3. **Update configuration**: Use `.with_schema_dir()` if needed
4. **Test thoroughly**: Verify all functionality works with file-based schemas

## Future Extensions

The file-based schema system supports future enhancements:

- **Dynamic reloading**: Reload schemas without server restart
- **Schema versions**: Support multiple schema versions
- **Remote schemas**: Load schemas from URLs
- **Schema inheritance**: Extend base schemas
- **Conditional attributes**: Attributes based on conditions

## Troubleshooting

### Common Issues

1. **File not found**: Ensure schema files are in the correct directory
2. **JSON syntax errors**: Validate JSON syntax with a JSON validator
3. **Missing required fields**: Check that all required fields are present
4. **Type mismatches**: Ensure attribute types are valid
5. **Complex attribute errors**: Verify sub-attributes for complex types

### Debug Tips

1. **Use validation tool**: Always validate schemas before deployment
2. **Check file paths**: Verify schema directory and file names
3. **Review error messages**: Schema loading errors include file details
4. **Test incrementally**: Add attributes one at a time when creating custom schemas

## Examples

See the `examples/` directory for:

- `CustomUser.json` - Extended User schema with enterprise attributes
- Additional example schemas for various use cases

For more information, see the main README.md and API documentation.