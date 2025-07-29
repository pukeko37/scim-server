# SCIM Schemas Directory

This directory contains the SCIM schema definitions used by the SCIM server for validation and resource management.

## Schema Files

### Core Schemas

- **`User.json`** - Core User schema as defined in RFC 7643
  - Contains all standard User attributes (userName, name, emails, etc.)
  - Defines complex attributes like name, emails, phoneNumbers, and meta
  - Used for validating User resources in SCIM operations

- **`Group.json`** - Core Group schema as defined in RFC 7643
  - Contains standard Group attributes (displayName, members)
  - Defines the members complex attribute for group membership management
  - Used for validating Group resources in SCIM operations

- **`ServiceProviderConfig.json`** - Service Provider Configuration schema
  - Defines the capabilities and configuration of the SCIM service provider
  - Used for the `/ServiceProviderConfig` endpoint
  - Contains authentication schemes, bulk operation limits, etc.

## Schema Format

All schemas follow the SCIM schema definition format with the following structure:

```json
{
  "id": "urn:ietf:params:scim:schemas:core:2.0:SchemaName",
  "name": "SchemaName",
  "description": "Schema description",
  "attributes": [
    {
      "name": "attributeName",
      "type": "string|boolean|decimal|integer|dateTime|binary|reference|complex",
      "multiValued": false,
      "required": false,
      "caseExact": false,
      "mutability": "readOnly|readWrite|immutable|writeOnly",
      "returned": "always|never|default|request",
      "uniqueness": "none|server|global",
      "subAttributes": [
        // For complex types only
      ],
      "canonicalValues": [
        // For string types with predefined values
      ]
    }
  ]
}
```

## Usage

The schemas are automatically loaded by the SCIM server at startup:

```rust
use scim_server::schema::SchemaRegistry;

// Loads all schemas from the schemas/ directory
let registry = SchemaRegistry::new()?;

// Or load from a specific directory
let registry = SchemaRegistry::from_schema_dir("path/to/schemas")?;
```

## Validation

You can validate schema files using the schema validator tool:

```bash
# Validate all schemas in the directory
cargo run --bin schema-validator schemas/

# Validate a specific schema file
cargo run --bin schema-validator schemas/User.json
```

## Adding New Schemas

To add a new schema:

1. Create a new JSON file following the SCIM schema format
2. Ensure the schema ID follows the SCIM URI format
3. Validate the schema using the schema validator tool
4. Update the SchemaRegistry code if needed to load the new schema

## RFC Compliance

These schemas implement:

- **RFC 7643** - SCIM Core Schema
  - Section 4: User Schema
  - Section 5: Group Schema (planned)
  - Section 6: Enterprise User Schema Extension (planned)

## Current Limitations

- Only the User schema is currently loaded and used by the validation engine
- Group schema is defined but not yet integrated into the validation engine
- Extension schemas are not yet supported
- Schema discovery endpoints use these definitions

## File Structure

```
schemas/
├── README.md                    # This file
├── User.json                    # Core User schema (RFC 7643)
├── Group.json                   # Core Group schema (RFC 7643)
└── ServiceProviderConfig.json   # Service provider capabilities
```

Future additions:
- `EnterpriseUser.json` - Enterprise User extension
- Custom extension schemas as needed