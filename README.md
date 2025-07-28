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

### ✅ Implemented and Working (19/52 validation errors)
- **Schema Structure Validation (Errors 1-8)**: Complete with 14 passing tests
  - Missing/empty schemas arrays
  - Invalid/unknown schema URIs  
  - Duplicate schemas and extension validation

- **Common Attributes Validation (Errors 9-21)**: Validation logic complete, Step 2 pending
  - ✅ ID validation: Missing, empty, invalid format (Errors 9-11)
  - ✅ External ID validation: Type and format checking (Error 13)
  - ✅ Meta validation: Structure, resource type, timestamps, location, version (Errors 14-21)
  - 🔲 Test transformation needed: 17 tests ready to use actual validation

### 🔲 Ready for Implementation (33/52 validation errors)
- **Data Types (Errors 22-32)**: String, boolean, integer, datetime validation
- **Multi-valued (Errors 33-38)**: Array attribute validation
- **Complex Attributes (Errors 39-43)**: Nested object validation
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
src/
├── lib.rs              # Public API
├── error.rs            # Error types and handling
├── schema.rs           # Schema validation (✅ Phase 1 complete)
├── resource.rs         # Resource management
└── resource_handlers.rs # Dynamic resource operations

tests/
├── validation/         # Validation test suites
│   ├── schema_structure.rs  # ✅ Complete (template for others)
│   ├── common_attributes.rs # 🔲 Next phase
│   ├── data_types.rs        # 🔲 Phase 3
│   ├── multi_valued.rs      # 🔲 Phase 4
│   ├── complex_attributes.rs # 🔲 Phase 5
│   └── characteristics.rs   # 🔲 Phase 6
└── common/             # Test utilities and builders
```

## Contributing

### Adding New Validation Categories

The project follows a systematic approach to implementing validation:

1. **Follow the Pattern**: Use `tests/validation/schema_structure.rs` as your template
2. **Read the Guide**: See `TESTING_IMPLEMENTATION_GUIDE.md` for exact steps
3. **Check Progress**: Review `TESTING_PROGRESS.md` for current status

**Current Focus**: Phase 2 Step 2 - Transform `tests/validation/common_attributes.rs` to use actual validation logic instead of testing builders.

### Development Workflow

```bash
# 1. Add error types to src/error.rs
# 2. Implement validation in src/schema.rs  
# 3. Update test file to call real validation
# 4. Run tests and verify
cargo test validation::your_category --test lib
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
    
    Ok(_) => {
        // Resource is valid
    }
}
```

## Current Limitations

- Only User schema is currently loaded (Group schema planned)
- Extension schemas not yet supported
- Some validation functions marked as TODO (RFC3339 dates, base64, etc.)
- Test suite is in active development (19/52 error types implemented)
- 2 Phase 2 errors deferred (ClientProvidedId, ClientProvidedMeta - need operation context)

## License

This project is licensed under the MIT License - see the LICENSE file for details.

## RFC Compliance

This implementation follows:
- **RFC 7643**: SCIM Core Schema
- **RFC 7644**: SCIM Protocol (planned)
- **RFC 3339**: Date and Time on the Internet (partially implemented)

## Development Status

**Phase 1 Complete**: Foundation established with working schema structure validation (8/52 errors).
**Phase 2 Step 1 Complete**: Validation logic implemented for common attributes (11/13 errors working, 19/52 total).
**Next**: Phase 2 Step 2 - Transform integration tests to use actual validation logic.

### Validation Functions Working
- ✅ Schema structure validation (errors 1-8)
- ✅ ID attribute validation (errors 9-11)  
- ✅ External ID validation (error 13)
- ✅ Meta attribute validation (errors 14-21, enhanced)

### Ready for Testing Transformation
- 🔲 `tests/validation/common_attributes.rs` - 17 tests ready to use validation logic

The project is designed for incremental development with each phase building on the previous foundation.