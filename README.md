# SCIM Server

A Rust implementation of a SCIM (System for Cross-domain Identity Management) 2.0 server with comprehensive validation and resource management capabilities.

## Features

- **RFC 7643 Compliant**: Implements SCIM 2.0 Core Schema specification
- **Comprehensive Validation**: 52 distinct validation error types for complete SCIM compliance
- **Flexible Resource Handlers**: Dynamic resource management with customizable operations
- **Schema Registry**: Extensible schema system supporting core and custom schemas
- **Type-Safe Design**: Leverages Rust's type system for compile-time guarantees

## Quick Start

### Running Tests

```bash
# Run all tests
cargo test

# Run integration tests
cargo test --test lib

# Run specific validation category
cargo test validation::schema_structure --test lib

# Run with output
cargo test --test lib -- --nocapture
```

### Basic Usage

```rust
use scim_server::schema::SchemaRegistry;
use serde_json::json;

// Create schema registry
let registry = SchemaRegistry::new()?;

// Validate a SCIM resource
let user = json!({
    "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
    "id": "123",
    "userName": "johndoe@example.com",
    "meta": {
        "resourceType": "User"
    }
});

// This will validate schemas array, ID, external ID, meta structure, and attribute compliance
let result = registry.validate_scim_resource(&user)?;
```

## Testing Status

### ✅ Implemented and Working (40/52 validation errors)
- **Schema Structure Validation (Errors 1-8)**: Complete with 14 passing tests
  - Missing/empty schemas arrays
  - Invalid/unknown schema URIs  
  - Duplicate schemas and extension validation

- **Common Attributes Validation (Errors 9-21)**: Complete with 22 passing tests
  - ✅ ID validation: Missing, empty, invalid format (Errors 9-11)
  - ✅ External ID validation: Type and format checking (Error 13)
  - ✅ Meta validation: Structure, resource type, timestamps, location, version (Errors 14-21)
  - ✅ Integration tests transformed to use actual validation logic
  - 🔲 3 errors deferred: 2 need operation context, 1 currently optional

- **Data Type Validation (Errors 22-32)**: Complete with 22 passing tests
  - ✅ Basic type validation: String, boolean, integer, decimal (Errors 23-27)
  - ✅ Format validation: DateTime, binary data, URI references (Errors 28-30)
  - ✅ Reference validation: Type checking and broken references (Errors 31-32)
  - ✅ Enhanced error messages with specific validation details
  - ✅ Missing required attribute detection (Error 22)

- **Multi-valued Attributes (Errors 33-38)**: Complete with 22 passing tests
  - ✅ Single/multi-valued type checking (Errors 33-34)
  - ✅ Primary value constraints (Error 35)  
  - ✅ Array structure validation (Error 36)
  - ✅ Required sub-attribute checking (Error 37)
  - ✅ Canonical value validation (Error 38)

- **Complex Attributes (Errors 39-43)**: Complete with 21 passing tests
  - ✅ Missing required sub-attributes detection (Error 39)
  - ✅ Sub-attribute type validation (Error 40)
  - ✅ Unknown sub-attribute detection (Error 41)
  - ✅ Nested complex attribute prevention (Error 42)
  - ✅ Malformed complex structure validation (Error 43)
  - ✅ Schema-driven validation using actual SCIM schema definitions
  - ✅ Integration tests transformed to use actual validation logic

### 🔲 Ready for Implementation (12/52 validation errors)
- **Characteristics (Errors 44-52)**: Mutability, uniqueness constraints

## Documentation

### For Developers
- **[Testing Progress](TESTING_PROGRESS.md)**: Complete status and roadmap
- **[Implementation Guide](TESTING_IMPLEMENTATION_GUIDE.md)**: Step-by-step development pattern
- **[Validation Testing](tests/VALIDATION_TESTING.md)**: Original design documentation

### Architecture
- **[Architecture Overview](Architecture.md)**: System design and components

## Project Structure

```
schemas/                # SCIM schema definitions
├── User.json          # Core User schema (RFC 7643)
├── Group.json         # Core Group schema (RFC 7643)
└── ServiceProviderConfig.json # Service provider capabilities

src/
├── lib.rs              # Public API
├── error.rs            # Error types and handling
├── schema.rs           # Schema validation (✅ Phase 1 complete)
├── resource.rs         # Resource management
└── resource_handlers.rs # Dynamic resource operations

tests/
├── validation/         # Validation test suites
│   ├── schema_structure.rs  # ✅ Complete (14 tests)
│   ├── common_attributes.rs # ✅ Complete (22 tests)
│   ├── data_types.rs        # ✅ Complete (22 tests)
│   ├── multi_valued.rs      # ✅ Complete (22 tests)
│   ├── complex_attributes.rs # ✅ Complete (21 tests)
│   └── characteristics.rs   # 🔲 Phase 6
└── common/             # Test utilities and builders
```

## Contributing

### Adding New Validation Categories

The project follows a systematic approach to implementing validation:

1. **Follow the Pattern**: Use `tests/validation/schema_structure.rs` as your template
2. **Read the Guide**: See `TESTING_IMPLEMENTATION_GUIDE.md` for exact steps
3. **Check Progress**: Review `TESTING_PROGRESS.md` for current status

**Current Focus**: Phase 6 - Attribute characteristics validation implementation (Errors 44-52).

### Development Workflow

```bash
# 1. Add error types to src/error.rs
# 2. Implement validation in src/schema.rs  
# 3. Update test file to call real validation
# 4. Run tests and verify
cargo test validation::your_category --test lib

# Phase 5 (Complex attributes) example
cargo test validation::complex_attributes --test lib
```

## Key Principles

- **YAGNI Compliance**: Only implement what's currently needed
- **Functional Style**: Idiomatic Rust with iterator combinators
- **Type Safety**: Leverage compile-time guarantees where possible
- **Code Reuse**: Follow the established hierarchy for dependencies

## Error Handling

The server provides detailed error information for all validation failures:

```rust
match registry.validate_scim_resource(&invalid_resource) {
    // Schema validation errors (Phase 1)
    Err(ValidationError::MissingSchemas) => {
        // Handle missing schemas array
    }
    Err(ValidationError::InvalidSchemaUri { uri }) => {
        // Handle malformed schema URI
        println!("Invalid URI: {}", uri);
    }
    Err(ValidationError::UnknownSchemaUri { uri }) => {
        // Handle unregistered schema
        println!("Unknown schema: {}", uri);
    }
    
    // Common attribute validation errors (Phase 2)
    Err(ValidationError::MissingId) => {
        // Handle missing ID attribute
    }
    Err(ValidationError::EmptyId) => {
        // Handle empty ID value
    }
    Err(ValidationError::InvalidIdFormat { id }) => {
        // Handle invalid ID format
        println!("Invalid ID format: {}", id);
    }
    Err(ValidationError::InvalidExternalId) => {
        // Handle invalid external ID
    }
    Err(ValidationError::InvalidResourceType { resource_type }) => {
        // Handle invalid meta.resourceType
        println!("Invalid resource type: {}", resource_type);
    }
    
    // Data type validation errors (Phase 3)
    Err(ValidationError::InvalidDataType { attribute, expected, actual }) => {
        // Handle wrong data type
        println!("Attribute '{}' expected {}, got {}", attribute, expected, actual);
    }
    Err(ValidationError::InvalidStringFormat { attribute, details }) => {
        // Handle string format issues
        println!("String format error in '{}': {}", attribute, details);
    }
    Err(ValidationError::InvalidDateTimeFormat { attribute, value }) => {
        // Handle invalid datetime format
        println!("Invalid datetime in '{}': {}", attribute, value);
    }
    Err(ValidationError::InvalidBinaryData { attribute, details }) => {
        // Handle invalid binary data
        println!("Binary data error in '{}': {}", attribute, details);
    }
    
    // Multi-valued attribute validation errors (Phase 4)
    Err(ValidationError::SingleValueForMultiValued { attribute }) => {
        // Handle single value for multi-valued attribute
        println!("Attribute '{}' must be an array", attribute);
    }
    Err(ValidationError::ArrayForSingleValued { attribute }) => {
        // Handle array for single-valued attribute
        println!("Attribute '{}' must not be an array", attribute);
    }
    Err(ValidationError::MultiplePrimaryValues { attribute }) => {
        // Handle multiple primary values
        println!("Attribute '{}' cannot have multiple primary values", attribute);
    }
    Err(ValidationError::InvalidMultiValuedStructure { attribute, details }) => {
        // Handle invalid multi-valued structure
        println!("Multi-valued structure error in '{}': {}", attribute, details);
    }
    Err(ValidationError::MissingRequiredSubAttribute { attribute, sub_attribute }) => {
        // Handle missing required sub-attribute
        println!("Missing required sub-attribute '{}' in '{}'", sub_attribute, attribute);
    }
    
    // Complex attribute validation errors (Phase 5)
    Err(ValidationError::MissingRequiredSubAttributes { attribute, missing }) => {
        // Handle missing required sub-attributes
        println!("Complex attribute '{}' missing required sub-attributes: {:?}", attribute, missing);
    }
    Err(ValidationError::InvalidSubAttributeType { attribute, sub_attribute, expected, actual }) => {
        // Handle invalid sub-attribute type
        println!("Complex attribute '{}' has invalid sub-attribute '{}' type, expected {}, got {}", attribute, sub_attribute, expected, actual);
    }
    Err(ValidationError::UnknownSubAttribute { attribute, sub_attribute }) => {
        // Handle unknown sub-attribute
        println!("Complex attribute '{}' contains unknown sub-attribute '{}'", attribute, sub_attribute);
    }
    Err(ValidationError::NestedComplexAttributes { attribute }) => {
        // Handle nested complex attributes
        println!("Nested complex attributes are not allowed: '{}'", attribute);
    }
    Err(ValidationError::MalformedComplexStructure { attribute, details }) => {
        // Handle malformed complex structure
        println!("Complex attribute '{}' has malformed structure: {}", attribute, details);
    }
    
    Ok(_) => {
        // Resource is valid
    }
}
```

## Current Limitations

- Only User schema is currently loaded from `schemas/User.json` (Group schema available but not integrated)
- Extension schemas not yet supported
- Enhanced format validation planned (full RFC3339 dates, strict base64, complete URI validation)
- Test suite is in active development (40/52 error types implemented)
- 2 Phase 2 errors deferred (ClientProvidedId, ClientProvidedMeta - need operation context)
- Characteristics validation not yet implemented

## License

This project is licensed under the MIT License - see the LICENSE file for details.

## RFC Compliance

This implementation follows:
- **RFC 7643**: SCIM Core Schema
- **RFC 7644**: SCIM Protocol (planned)
- **RFC 3339**: Date and Time on the Internet (partially implemented)

**Development Status**

**Phase 1:** ✅ **COMPLETE** - Schema structure validation fully implemented (8/52 errors).
**Phase 2:** ✅ **COMPLETE** - Common attributes validation fully implemented and tested (10/13 testable errors working).
**Phase 3:** ✅ **COMPLETE** - Data type validation fully implemented and tested (11/11 errors working).
**Phase 4:** ✅ **COMPLETE** - Multi-valued attribute validation fully implemented and tested (6/6 errors working).
**Phase 5:** ✅ **COMPLETE** - Complex attribute validation fully implemented and tested (5/5 errors working).
**Current Total:** 40/52 errors implemented (77% complete).
**Next**: Phase 6 - Attribute characteristics validation implementation.

### Validation Functions Working
- ✅ Schema structure validation (errors 1-8)
- ✅ ID attribute validation (errors 9-11)  
- ✅ External ID validation (error 13)
- ✅ Meta attribute validation (errors 14-21, enhanced)
- ✅ Data type validation (errors 22-32, comprehensive)
- ✅ Multi-valued attribute validation (errors 33-38, complete)
- ✅ Complex attribute validation (errors 39-43, schema-driven)

### Integration Tests Complete
- ✅ `tests/validation/schema_structure.rs` - 14 tests using validation logic
- ✅ `tests/validation/common_attributes.rs` - 22 tests using validation logic
- ✅ `tests/validation/data_types.rs` - 22 tests using validation logic
- ✅ `tests/validation/multi_valued.rs` - 22 tests using validation logic
- ✅ `tests/validation/complex_attributes.rs` - 21 tests using validation logic

The project is designed for incremental development with each phase building on the previous foundation.