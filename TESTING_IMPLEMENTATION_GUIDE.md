# SCIM Validation Testing Implementation Guide

This guide shows developers exactly how to implement new validation categories following the established pattern.

**Current Status:** Phase 4 Complete - Multi-valued attribute validation fully implemented and tested (35/52 errors total). Next: Phase 5 - Complex attribute validation implementation.

## Quick Start: Copy This Pattern

**Note:** Phases 1-4 are complete. For Phase 5+, follow the complete pattern below.

### Step 1: Add Error Types to `src/error.rs`

✅ **Phases 1-3 Complete** - Schema structure, common attributes, and data type error types implemented and working.

For Phase 5+ (complex attributes, characteristics), add your error variants to the `ValidationError` enum:

```rust
/// Add to ValidationError enum
#[derive(Debug, thiserror::Error)]
pub enum ValidationError {
    // ... existing errors ...
    
    // Your new error category (e.g., Complex Attribute Errors 39-43)
    /// Missing required sub-attributes in complex attribute
    #[error("Complex attribute '{attribute}' missing required sub-attributes: {missing:?}")]
    MissingRequiredSubAttributes { attribute: String, missing: Vec<String> },  // Error #39
    
    /// Invalid sub-attribute type in complex attribute
    #[error("Sub-attribute '{sub_attribute}' in '{attribute}' has invalid type, expected {expected}, got {actual}")]
    InvalidSubAttributeType { attribute: String, sub_attribute: String, expected: String, actual: String }, // Error #40
    
    /// Unknown sub-attribute in complex attribute
    #[error("Unknown sub-attribute '{sub_attribute}' in complex attribute '{attribute}'")]
    UnknownSubAttribute { attribute: String, sub_attribute: String }, // Error #41
    
    // ... add all errors for your category
}
```

### Step 2: Add Validation Functions to `src/schema.rs`

✅ **Phases 1-4 Complete** - All validation functions implemented and working:
- `validate_schemas_attribute()` - Schema structure validation (Errors 1-8)
- `validate_id_attribute()` - ID validation (Errors 9-12)
- `validate_external_id()` - External ID validation (Error 13)  
- Enhanced `validate_meta_attribute()` - Meta validation (Errors 14-21)
- Enhanced `validate_attribute_value()` - Data type validation (Errors 22-32)
- `validate_multi_valued_attributes()` - Multi-valued validation (Errors 33-38)

For Phase 5+, add validation methods to the `SchemaRegistry` implementation:

```rust
impl SchemaRegistry {
    /// Your main validation function
    pub fn validate_your_category(&self, resource: &Value) -> ValidationResult<()> {
        let obj = resource
            .as_object()
            .ok_or_else(|| ValidationError::custom("Resource must be a JSON object"))?;

        // Call specific validation functions
        self.validate_specific_aspect(obj)?;
        self.validate_another_aspect(obj)?;
        
        Ok(())
    }

    /// Specific validation function
    fn validate_specific_aspect(
        &self,
        obj: &serde_json::Map<String, Value>,
    ) -> ValidationResult<()> {
        // Implementation here
        if let Some(value) = obj.get("attributeName") {
            // Validate the attribute
            if !self.is_valid_format(value) {
                return Err(ValidationError::YourSpecificError {
                    attribute: "attributeName".to_string(),
                    // ... other fields
                });
            }
        }
        
        Ok(())
    }
    
    /// Helper function
    fn is_valid_format(&self, value: &Value) -> bool {
        // Your validation logic
        true
    }
}
```

### Step 3: Update Your Test File

🔲 **Phase 5 NEXT** - Implement complex attribute validation (Errors 39-43).

✅ **Templates Available** - Follow the exact pattern from `tests/validation/schema_structure.rs`, `tests/validation/common_attributes.rs`, `tests/validation/data_types.rs`, or `tests/validation/multi_valued.rs`.

Follow this exact pattern in your test file (e.g., `tests/validation/complex_attributes.rs`):

```rust
//! Complex attribute validation tests.
//!
//! This module tests validation errors related to complex attributes (Errors 39-43).

use serde_json::json;

// Import test utilities
use crate::common::{ValidationErrorCode, builders::UserBuilder, fixtures::rfc_examples};

// Import SCIM server types
use scim_server::error::{ValidationError};
use scim_server::schema::SchemaRegistry;

/// Test Error #39: Missing required sub-attributes in complex attribute
#[test]
fn test_missing_required_sub_attributes() {
    let registry = SchemaRegistry::new().expect("Failed to create registry");

    // Create a User resource with incomplete name complex attribute
    let invalid_user = UserBuilder::new().with_incomplete_name().build();

    // Verify the test data is missing required sub-attributes
    assert!(invalid_user["name"].is_object());
    assert!(!invalid_user["name"].as_object().unwrap().contains_key("familyName"));

    // Actually validate the resource
    let result = registry.validate_scim_resource(&invalid_user);

    // Assert that validation fails with the expected error
    assert!(result.is_err());
    match result {
        Err(ValidationError::MissingRequiredSubAttributes { attribute, missing }) => {
            assert_eq!(attribute, "name");
            assert!(missing.contains(&"familyName".to_string()));
        }
        Err(other) => panic!("Expected MissingRequiredSubAttributes error, got {:?}", other),
        Ok(_) => panic!("Expected validation to fail, but it passed"),
    }
}

/// Test Error #40: Invalid sub-attribute type
#[test] 
fn test_invalid_sub_attribute_type() {
    let registry = SchemaRegistry::new().expect("Failed to create registry");

    // Create a User resource with wrong type for name sub-attribute
    let invalid_user = UserBuilder::new().with_invalid_name_sub_attribute_type().build();

    // Verify the test data has wrong type for sub-attribute
    assert!(invalid_user["name"]["givenName"].is_number());

    // Actually validate the resource  
    let result = registry.validate_scim_resource(&invalid_user);

    // Assert that validation fails with the expected error
    assert!(result.is_err());
    match result {
        Err(ValidationError::InvalidSubAttributeType { attribute, sub_attribute, expected, actual }) => {
            assert_eq!(attribute, "name");
            assert_eq!(sub_attribute, "givenName");
            assert_eq!(expected, "string");
            assert_eq!(actual, "number");
        }
        Err(other) => panic!("Expected InvalidSubAttributeType error, got {:?}", other),
        Ok(_) => panic!("Expected validation to fail, but it passed"),
    }
}

// ... continue this pattern for all your error types

/// Test valid cases to ensure no false positives
#[test]
fn test_valid_data_types() {
    let registry = SchemaRegistry::new().expect("Failed to create registry");

    let valid_user = rfc_examples::user_minimal();
    
    // This should pass validation
    let result = registry.validate_scim_resource(&valid_user);
    assert!(result.is_ok(), "Valid user should pass validation: {:?}", result);
}
```

## Step-by-Step Implementation Checklist

### Phase 4 Implementation: Multi-valued Attributes ✅ COMPLETE
- [x] ✅ **COMPLETE:** Multi-valued Attributes (Errors 33-38)
- [x] ✅ Listed all error types for category from `tests/common/mod.rs`
- [x] ✅ Identified existing builder methods in `tests/common/builders.rs`
- [x] ✅ Added missing error types to `ValidationError` enum in `src/error.rs`
- [x] ✅ Implemented validation logic in `src/schema.rs`
- [x] ✅ Integration with main validation flow
- [x] ✅ Transformed test file `tests/validation/multi_valued.rs`
- [x] ✅ Followed pattern from completed phases
- [x] ✅ All tests pass: `cargo test validation::multi_valued --test lib` (22 tests)
- [x] ✅ Documentation updated

### Phase 5 Implementation: Complex Attributes (Next)
- [ ] 🔲 **NEXT:** Choose validation category: Complex Attributes (Errors 39-43)
- [ ] 🔲 List all error types for category from `tests/common/mod.rs`
- [ ] 🔲 Identify which builder methods already exist in `tests/common/builders.rs`
- [ ] 🔲 Add missing error types to `ValidationError` enum in `src/error.rs`
- [ ] 🔲 Implement validation logic in `src/schema.rs`
- [ ] 🔲 Integration with main validation flow
- [ ] 🔲 Transform test file `tests/validation/complex_attributes.rs`
- [ ] 🔲 Follow pattern from completed phases
- [ ] 🔲 Verify tests pass: `cargo test validation::complex_attributes --test lib`
- [ ] 🔲 Update documentation when complete

### Phase 5+ Implementation (Future)
- [ ] Choose your validation category (Complex Attributes, Characteristics, etc.)
- [ ] List all error types for your category from `tests/common/mod.rs`
- [ ] Identify which builder methods already exist in `tests/common/builders.rs`

### Error Types Implementation (Phase 3+)
- [ ] Add all error variants to `ValidationError` enum in `src/error.rs`
- [ ] Follow naming convention: `ErrorName { field: Type }`
- [ ] Include descriptive error messages with `#[error("...")]`
- [ ] Test compilation: `cargo check`

### Validation Logic Implementation (Phase 3+)
- [ ] Add main validation function to `SchemaRegistry` in `src/schema.rs`
- [ ] Break down into smaller helper functions for each validation aspect
- [ ] Use existing helper patterns (e.g., `extract_schema_uris`, `is_valid_schema_uri`)
- [ ] Return specific error types, not generic `ValidationError::custom()`
- [ ] Test compilation: `cargo check`

### Integration with Main Validation (Phase 3+)
- [ ] Update `validate_scim_resource()` to call your validation function
- [ ] Ensure proper error propagation
- [ ] Test basic functionality: `cargo test --lib`

### Test Implementation (Phase 3+)
- [ ] Copy test file structure from `tests/validation/schema_structure.rs`
- [ ] Update imports and error types
- [ ] Implement test for each error type following the established pattern
- [ ] Add valid case tests to prevent false positives
- [ ] Run your tests: `cargo test validation::your_category --test lib`

### Builder Updates (if needed)
- [ ] Add missing builder methods to `tests/common/builders.rs`
- [ ] Follow existing naming: `with_invalid_*`, `without_*`, etc.
- [ ] Update `expected_errors` tracking
- [ ] Test builder functionality independently

### Documentation and Coverage
- [ ] Update test documentation and comments
- [ ] Add your phase to `TESTING_PROGRESS.md`
- [ ] Update success metrics
- [ ] Run full test suite: `cargo test --test lib`

## Common Patterns

### Error Handling Pattern
```rust
// Always use specific error types
return Err(ValidationError::SpecificError {
    field1: value1.to_string(),
    field2: value2,
});

// NOT generic errors
return Err(ValidationError::custom("Something went wrong"));
```

### Validation Function Pattern
```rust
fn validate_specific_thing(&self, obj: &Map<String, Value>) -> ValidationResult<()> {
    // 1. Extract value
    let value = obj.get("field").ok_or_else(|| ValidationError::MissingField)?;
    
    // 2. Type check
    let string_value = value.as_str().ok_or_else(|| ValidationError::InvalidType { ... })?;
    
    // 3. Format/content validation
    if !self.is_valid_format(string_value) {
        return Err(ValidationError::InvalidFormat { ... });
    }
    
    // 4. Business logic validation
    if !self.meets_business_rules(string_value) {
        return Err(ValidationError::BusinessRuleViolation { ... });
    }
    
    Ok(())
}
```

### Test Pattern
```rust
#[test]
fn test_specific_error() {
    // 1. Setup
    let registry = SchemaRegistry::new().expect("Failed to create registry");
    
    // 2. Create invalid data using builder
    let invalid_resource = BuilderType::new().with_invalid_condition().build();
    
    // 3. Verify test data is invalid
    assert!(/* condition showing data is invalid */);
    
    // 4. Validate and assert specific error
    let result = registry.validate_scim_resource(&invalid_resource);
    assert!(result.is_err());
    match result {
        Err(ValidationError::ExpectedErrorType { field }) => {
            assert_eq!(field, "expected_value");
        }
        Err(other) => panic!("Expected ExpectedErrorType, got {:?}", other),
        Ok(_) => panic!("Expected validation to fail, but it passed"),
    }
}
```

## Debugging Tips

### Test Failures
```bash
# Run single test with output
cargo test test_your_specific_test --test lib -- --nocapture

# Check what error you're actually getting
RUST_BACKTRACE=1 cargo test test_your_specific_test --test lib
```

### Validation Logic Issues
```rust
// Add debug prints to see what's happening
println!("Validating value: {:?}", value);
println!("Validation result: {:?}", result);

// Use the existing unit tests in src/ files
cargo test --lib  # Runs unit tests in src/
```

### Builder Issues
```rust
// Test your builder in isolation
let built = YourBuilder::new().with_invalid_thing().build();
println!("Built object: {:#}", built);
```

## Integration Testing

After implementing your category, verify integration:

```bash
# Phase 4: Test multi-valued attributes specifically  
cargo test validation::multi_valued --test lib

# Verify all completed phases still work
cargo test validation::schema_structure --test lib
cargo test validation::common_attributes --test lib  
cargo test validation::data_types --test lib

# Run all validation tests
cargo test validation --test lib

# Full test suite
cargo test --test lib
```

## Phase 4: Specific Instructions

**Current Task:** Implement multi-valued attribute validation (Errors 33-38).

**Pattern to Follow:** Copy exactly from completed phases (`tests/validation/schema_structure.rs`, `tests/validation/common_attributes.rs`, or `tests/validation/data_types.rs`):

1. **Import the validation types:**
   ```rust
   use scim_server::error::ValidationError;
   use scim_server::schema::SchemaRegistry;
   ```

2. **Transform each test:**
   ```rust
   // OLD PATTERN (testing builders)
   let builder = UserBuilder::new().without_id();
   let expected_errors = builder.expected_errors();
   assert_eq!(expected_errors, &[ValidationErrorCode::MissingId]);

   // NEW PATTERN (testing validation)
   let registry = SchemaRegistry::new().expect("Failed to create registry");
   let invalid_user = UserBuilder::new().without_id().build();
   let result = registry.validate_scim_resource(&invalid_user);
   assert!(result.is_err());
   match result {
       Err(ValidationError::MissingId) => {
           // Expected error occurred
       }
       Err(other) => panic!("Expected MissingId error, got {:?}", other),
       Ok(_) => panic!("Expected validation to fail, but it passed"),
   }
   ```

3. **Tests to Transform (estimated 15 total):**
   - `test_missing_required_sub_attributes` → `ValidationError::MissingRequiredSubAttributes`
   - `test_invalid_sub_attribute_type` → `ValidationError::InvalidSubAttributeType`  
   - `test_unknown_sub_attribute` → `ValidationError::UnknownSubAttribute`
   - `test_nested_complex_attributes` → `ValidationError::NestedComplexAttributes`
   - `test_malformed_complex_structure` → `ValidationError::MalformedComplexStructure`
   - Plus valid case tests for complex attributes

## Next Steps After Implementation

1. Update `TESTING_PROGRESS.md` with Phase 5 completion  
2. Verify all integration tests still pass (currently 144 tests passing, 3 ignored)
3. Document any issues or patterns discovered
4. Begin Phase 6 planning (Characteristics validation)

## Reference Files

- **Template:** `tests/validation/multi_valued.rs` - ✅ Copy this structure exactly
- **Error Types:** `src/error.rs` - 🔲 Phase 5 errors need to be added
- **Validation Logic:** `src/schema.rs` - 🔲 Phase 5 functions need implementation
- **Builders:** `tests/common/builders.rs` - ✅ Some Phase 5 builders already exist
- **Progress:** `TESTING_PROGRESS.md` - 🔲 Update when Phase 5 complete

## Current Implementation Status

**Phase 1:** ✅ **COMPLETE** - Schema structure validation (8/52 errors) fully working
**Phase 2:** ✅ **COMPLETE** - Common attributes validation (10/13 testable errors working)  
**Phase 3:** ✅ **COMPLETE** - Data type validation (11/11 errors working)
**Phase 4:** ✅ **COMPLETE** - Multi-valued attribute validation (6/6 errors working, 22 tests passing)
**Phase 5:** 🔲 **NEXT** - Complex attribute validation implementation

**Validation Functions Working:**
```rust
// These are already implemented and working in src/schema.rs
validate_schemas_attribute()    // Errors 1-8: Schema structure validation
validate_id_attribute()        // Errors 9-11: ID validation  
validate_external_id()         // Error 13: External ID validation
validate_meta_attribute()         // Errors 14-21: Meta validation (enhanced)
validate_attribute_value()        // Errors 22-32: Data type validation (enhanced)
validate_multi_valued_attributes() // Errors 33-38: Multi-valued validation (complete)
```

**Test Files Status:**
- `tests/validation/schema_structure.rs` - ✅ **COMPLETE** (14 tests)
- `tests/validation/common_attributes.rs` - ✅ **COMPLETE** (22 tests)
- `tests/validation/data_types.rs` - ✅ **COMPLETE** (22 tests)
- `tests/validation/multi_valued.rs` - ✅ **COMPLETE** (22 tests)
- `tests/validation/complex_attributes.rs` - 🔲 **PHASE 5 TARGET**
- `tests/validation/characteristics.rs` - 🔲 **PHASE 6**

This pattern has been proven to work across four completed phases with 144 total tests passing. The established pattern and infrastructure are ready for Phase 5 complex attribute validation.