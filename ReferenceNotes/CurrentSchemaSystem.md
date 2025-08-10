# Current Schema System Documentation

This document provides a comprehensive overview of the SCIM server's current schema handling and validation system.

## Overview

The SCIM server implements a robust schema-driven validation system that ensures all resources conform to their defined schemas. The system follows the SCIM RFC 7643 specification and provides comprehensive validation at multiple levels.

## Schema Storage & Loading

### File-Based Schema Storage
- Schemas are stored as JSON files in the `schemas/` directory
- Core schemas provided: `User.json` and `Group.json`
- Schemas follow the SCIM RFC 7643 JSON schema format

### Schema Loading Process
The schema loading occurs during application startup:

1. **Initialization**: `SchemaRegistry::new()` is called during server startup
2. **Directory Loading**: `from_schema_dir("schemas")` loads all schemas from the filesystem
3. **File Processing**: Each `.json` file is loaded and validated
4. **Registry Population**: Valid schemas are stored in a `HashMap<String, Schema>` indexed by schema ID

```rust
// Schema loading flow
SchemaRegistry::new() 
  -> from_schema_dir("schemas")
    -> load_schema_from_file("User.json")
    -> load_schema_from_file("Group.json")
    -> convert_json_schema() // Transform to internal format
```

### Schema Structure
Each schema contains:
- **id**: Unique identifier (e.g., `urn:ietf:params:scim:schemas:core:2.0:User`)
- **name**: Human-readable name (e.g., "User")
- **description**: Schema description
- **attributes**: Array of attribute definitions

#### Attribute Definitions
Each attribute specifies:
- **name**: Attribute name
- **type**: Data type (string, boolean, integer, decimal, dateTime, binary, reference, complex)
- **multiValued**: Whether the attribute can contain multiple values
- **required**: Whether the attribute is mandatory
- **caseExact**: Case sensitivity for string comparisons
- **mutability**: Write permissions (readOnly, writeOnly, readWrite, immutable)
- **returned**: When the attribute is returned (always, default, request, never)
- **uniqueness**: Uniqueness constraints (none, server, global)
- **canonicalValues**: Allowed values for enumerated attributes
- **subAttributes**: For complex types, nested attribute definitions

## Schema Validation Timing

### Schema Definition Validation
Schema validation occurs at two distinct times:

1. **Startup Validation**: During `SchemaRegistry` initialization
   - Validates JSON syntax and structure
   - Ensures required schema fields are present
   - Converts external JSON format to internal representation
   - Fails fast if schemas are malformed

2. **Runtime Schema Addition**: When `add_schema()` is called
   - Validates the schema definition before adding to registry
   - Ensures schema ID uniqueness
   - Performs structural validation

## Resource Data Validation

### Validation Trigger Points
Resource validation occurs at multiple points in the request lifecycle:

1. **Resource Creation**: Before `create_resource()` persists data
2. **Resource Updates**: Before `update_resource()` applies changes
3. **Explicit Validation**: When `validate_scim_resource()` is called directly
4. **Resource Type Registration**: When handlers validate schema compatibility

### Multi-Phase Validation Architecture

The validation system implements a comprehensive multi-phase approach:

#### Phase 1: Basic Structure Validation
**Timing**: First phase of any resource validation
**Scope**: Fundamental SCIM resource structure

- **Schemas Array Validation**: Ensures `schemas` field contains valid schema URIs
- **Data Type Validation**: Verifies each attribute matches its defined type
- **Required Field Validation**: Confirms all required attributes are present
- **Multi-valued Constraints**: Validates array vs single-value expectations
- **Unknown Attribute Detection**: Flags attributes not defined in schema

```rust
// Phase 1 validation flow
validate_scim_resource()
  -> validate_schemas_attribute()    // Check schemas array
  -> validate_resource()             // Basic structure
    -> validate_attribute()          // Per-attribute validation
      -> validate_attribute_value()  // Type and constraint checking
```

#### Phase 2: SCIM-Specific Validation
**Timing**: After Phase 1 passes
**Scope**: SCIM protocol compliance and semantic validation

- **ID Format Validation**: Ensures resource IDs meet format requirements
- **External ID Validation**: Validates external identifier constraints
- **Meta Attribute Validation**: Checks meta object structure and values
- **Complex Attribute Validation**: Validates nested object structures
- **Canonical Value Validation**: Ensures enumerated values are valid
- **Sub-attribute Validation**: Validates nested attributes in complex types

```rust
// Phase 2 validation components
validate_scim_resource()
  -> validate_id_attribute()
  -> validate_external_id()
  -> validate_meta_attribute()
  -> validate_multi_valued_attributes()
  -> validate_complex_attributes()
  -> validate_attribute_characteristics()
```

### Detailed Validation Components

#### Attribute Type Validation
For each data type, specific validation rules apply:

- **String**: Character encoding, length constraints, format validation
- **Boolean**: True/false value verification
- **Integer**: Numeric range and format validation
- **Decimal**: Floating-point format and precision validation
- **DateTime**: RFC3339 format compliance using chrono parser
- **Binary**: Base64 encoding validation
- **Reference**: URI format validation for resource references
- **Complex**: Nested object structure and sub-attribute validation

#### Complex Attribute Validation
For complex attributes, the system validates:
- **Sub-attribute Presence**: Required sub-attributes exist
- **Sub-attribute Types**: Each nested field matches its type definition
- **Unknown Sub-attributes**: No undefined nested fields
- **Nested Complexity**: Prevents deeply nested complex structures

#### Multi-valued Attribute Validation
For array-type attributes:
- **Array Structure**: Ensures value is properly formatted array
- **Element Validation**: Each array element validated individually
- **Uniqueness Constraints**: Enforces unique values where required
- **Cardinality**: Validates minimum/maximum element counts

## Schema Registry Architecture

### Core Components

#### SchemaRegistry Structure
```rust
pub struct SchemaRegistry {
    core_user_schema: Schema,      // Cached User schema
    core_group_schema: Schema,     // Cached Group schema  
    schemas: HashMap<String, Schema>, // All schemas by ID
}
```

#### Key Methods
- **`new()`**: Initialize with default schema directory
- **`from_schema_dir()`**: Load schemas from custom directory
- **`validate_resource()`**: Validate against specific schema
- **`validate_scim_resource()`**: Full SCIM resource validation
- **`get_schema()`**: Retrieve schema by ID
- **`add_schema()`**: Runtime schema addition

### Integration with SCIM Server

The `SchemaRegistry` integrates with the main `ScimServer` through:

1. **Resource Type Registration**: Schemas enable resource type handlers
2. **Validation Pipeline**: All resource operations validate against schemas
3. **Capability Advertisement**: Server capabilities derived from available schemas
4. **Error Reporting**: Detailed validation errors reference schema definitions

## Validation Error Handling

### Error Types
The system provides detailed error information for validation failures:

- **MissingRequiredAttribute**: Required field absent
- **InvalidDataType**: Type mismatch between value and schema
- **UnknownAttribute**: Field not defined in schema
- **InvalidCanonicalValue**: Enumerated value not in allowed set
- **InvalidStringFormat**: String format violations
- **InvalidReferenceUri**: Malformed reference URIs
- **MissingSubAttribute**: Required nested field absent
- **InvalidSubAttributeType**: Nested field type mismatch

### Error Context
Each validation error includes:
- **Attribute Path**: Exact location of validation failure
- **Schema Context**: Which schema and rule was violated
- **Expected vs Actual**: Clear description of the mismatch
- **Corrective Guidance**: Hints for fixing the validation issue

## Performance Characteristics

### Schema Loading
- **Startup Cost**: One-time file I/O and parsing during initialization
- **Memory Usage**: Schemas cached in memory for fast access
- **Validation Speed**: In-memory validation with minimal overhead

### Validation Performance
- **Phase 1**: O(n) where n is number of attributes
- **Phase 2**: O(m) where m is number of SCIM-specific validations
- **Complex Attributes**: O(k) where k is depth of nesting
- **Multi-valued**: O(p) where p is array length

## Current Limitations

### Schema Management
- **Static Loading**: Schemas must be present at startup
- **File Dependency**: Requires filesystem access for schema files
- **No Runtime Updates**: Cannot modify schemas after initialization
- **Limited Extensibility**: Adding new schemas requires restart

### Validation Scope
- **Cross-Resource Validation**: No validation across resource boundaries
- **Referential Integrity**: Limited validation of resource references
- **Custom Rules**: No support for business-specific validation rules
- **Schema Evolution**: No versioning or migration support

## Testing and Validation Tools

### Schema Validator Utility
The `schema-validator` binary provides:
- **Single File Validation**: Validate individual schema files
- **Directory Validation**: Validate entire schema directories
- **Registry Testing**: Test schema loading and registry creation
- **Detailed Reporting**: Comprehensive validation reports

### Test Coverage
The validation system includes extensive test coverage:
- **Unit Tests**: Individual validation component testing
- **Integration Tests**: End-to-end validation scenarios
- **Error Case Testing**: Comprehensive error condition coverage
- **Performance Tests**: Validation timing and memory usage tests

This schema system provides a robust foundation for SCIM resource validation while maintaining clear separation of concerns and comprehensive error handling.