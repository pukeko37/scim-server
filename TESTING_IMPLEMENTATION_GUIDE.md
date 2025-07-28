# SCIM Validation Testing Implementation Guide

This guide shows developers exactly how to implement new validation categories following the established pattern.

**Current Status:** Phase 3 Complete - Data type validation fully implemented and tested (29/52 errors total). Next: Phase 4 - Multi-valued attribute validation implementation.

## Quick Start: Copy This Pattern

**Note:** Phases 1-3 are complete. For Phase 4+, follow the complete pattern below.

### Step 1: Add Error Types to `src/error.rs`

✅ **Phases 1-3 Complete** - Schema structure, common attributes, and data type error types implemented and working.

For Phase 4+ (multi-valued attributes, complex attributes, characteristics), add your error variants to the `ValidationError` enum:

```rust
/// Add to ValidationError enum
#[derive(Debug, thiserror::Error)]
pub enum ValidationError {
    // ... existing errors ...
    
    // Your new error category (e.g., Multi-valued Attribute Errors 33-38)
    /// Single value provided for multi-valued attribute
    #[error("Attribute '{attribute}' must be multi-valued (array)")]
    SingleValueForMultiValued { attribute: String },  // Error #33
    
    /// Array provided for single-valued attribute  
    #[error("Attribute '{attribute}' must be single-valued (not array)")]
    ArrayForSingleValued { attribute: String }, // Error #34
    
    /// Multiple primary values in multi-valued attribute
    #[error("Attribute '{attribute}' cannot have multiple primary values")]
    MultiplePrimaryValues { attribute: String }, // Error #35
    
    // ... add all errors for your category
}
```

### Step 2: Add Validation Functions to `src/schema.rs`

✅ **Phases 1-3 Complete** - All validation functions implemented and working:
- `validate_schemas_attribute()` - Schema structure validation (Errors 1-8)
- `validate_id_attribute()` - ID validation (Errors 9-12)
- `validate_external_id()` - External ID validation (Error 13)  
- Enhanced `validate_meta_attribute()` - Meta validation (Errors 14-21)
- Enhanced `validate_attribute_value()` - Data type validation (Errors 22-32)

For Phase 4+, add validation methods to the `SchemaRegistry` implementation:

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

🔲 **Phase 4 NEXT** - Implement multi-valued attribute validation (Errors 33-38).

✅ **Templates Available** - Follow the exact pattern from `tests/validation/schema_structure.rs`, `tests/validation/common_attributes.rs`, or `tests/validation/data_types.rs`.

Follow this exact pattern in your test file (e.g., `tests/validation/multi_valued.rs`):

```rust
//! Multi-valued attribute validation tests.
//!
//! This module tests validation errors related to multi-valued attributes (Errors 33-38).

use serde_json::json;

// Import test utilities
use crate::common::{ValidationErrorCode, builders::UserBuilder, fixtures::rfc_examples};

// Import SCIM server types
use scim_server::error::{ValidationError};
use scim_server::schema::SchemaRegistry;

/// Test Error #33: Single value for multi-valued attribute
#[test]
fn test_single_value_for_multi_valued() {
    let registry = SchemaRegistry::new().expect("Failed to create registry");

    // Create a User resource with single value for multi-valued emails
    let invalid_user = UserBuilder::new().with_single_value_emails().build();

    // Verify the test data is constructed correctly
    assert!(!invalid_user["emails"].is_array());

    // Actually validate the resource
    let result = registry.validate_scim_resource(&invalid_user);

    // Assert that validation fails with the expected error
    assert!(result.is_err());
    match result {
        Err(ValidationError::SingleValueForMultiValued { attribute }) => {
            assert_eq!(attribute, "emails");
        }
        Err(other) => panic!("Expected SingleValueForMultiValued error, got {:?}", other),
        Ok(_) => panic!("Expected validation to fail, but it passed"),
    }
}

/// Test Error #34: Array for single-valued attribute
#[test] 
fn test_array_for_single_valued() {
    let registry = SchemaRegistry::new().expect("Failed to create registry");

    // Create a User resource with array for single-valued userName
    let invalid_user = UserBuilder::new().with_array_username().build();

    // Verify the test data has wrong type
    assert!(invalid_user["userName"].is_array());

    // Actually validate the resource  
    let result = registry.validate_scim_resource(&invalid_user);

    // Assert that validation fails with the expected error
    assert!(result.is_err());
    match result {
        Err(ValidationError::ArrayForSingleValued { attribute }) => {
            assert_eq!(attribute, "userName");
        }
        Err(other) => panic!("Expected ArrayForSingleValued error, got {:?}", other),
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

### Phase 4 Implementation: Multi-valued Attributes (Next)
- [ ] 🔲 **NEXT:** Choose validation category: Multi-valued Attributes (Errors 33-38)
- [ ] 🔲 List all error types for category from `tests/common/mod.rs`
- [ ] 🔲 Identify which builder methods already exist in `tests/common/builders.rs`
- [ ] 🔲 Add missing error types to `ValidationError` enum in `src/error.rs`
- [ ] 🔲 Implement validation logic in `src/schema.rs`
- [ ] 🔲 Integration with main validation flow
- [ ] 🔲 Transform test file `tests/validation/multi_valued.rs`
- [ ] 🔲 Follow pattern from completed phases
- [ ] 🔲 Verify tests pass: `cargo test validation::multi_valued --test lib`
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

3. **Tests to Transform (17 total):**
   - `test_single_value_for_multi_valued` → `ValidationError::SingleValueForMultiValued`
   - `test_array_for_single_valued` → `ValidationError::ArrayForSingleValued`  
   - `test_multiple_primary_values` → `ValidationError::MultiplePrimaryValues`
   - `test_invalid_multi_valued_structure` → `ValidationError::InvalidMultiValuedStructure`
   - `test_missing_required_sub_attribute` → `ValidationError::MissingRequiredSubAttribute`
   - `test_invalid_canonical_value` → `ValidationError::InvalidCanonicalValue`
   - Plus valid case tests for multi-valued attributes

## Next Steps After Implementation

1. Update `TESTING_PROGRESS.md` with Phase 4 completion  
2. Verify all integration tests still pass (currently 147 tests)
3. Document any issues or patterns discovered
4. Begin Phase 5 planning (Complex Attributes validation)

## Reference Files

- **Template:** `tests/validation/schema_structure.rs` - ✅ Copy this structure exactly
- **Error Types:** `src/error.rs` - ✅ Phase 2 errors already added
- **Validation Logic:** `src/schema.rs` - ✅ Phase 2 functions already implemented
- **Builders:** `tests/common/builders.rs` - ✅ All Phase 2 builders already exist
- **Progress:** `TESTING_PROGRESS.md` - 🔲 Update when Phase 2 Step 2 complete

## Current Implementation Status

**Phase 1:** ✅ **COMPLETE** - Schema structure validation (8/52 errors) fully working
**Phase 2:** ✅ **COMPLETE** - Common attributes validation (10/13 testable errors working)  
**Phase 3:** ✅ **COMPLETE** - Data type validation (11/11 errors working)
**Phase 4:** 🔲 **NEXT** - Multi-valued attribute validation implementation

**Validation Functions Working:**
```rust
// These are already implemented and working in src/schema.rs
validate_schemas_attribute()    // Errors 1-8: Schema structure validation
validate_id_attribute()        // Errors 9-11: ID validation  
validate_external_id()         // Error 13: External ID validation
validate_meta_attribute()      // Errors 14-21: Meta validation (enhanced)
validate_attribute_value()     // Errors 22-32: Data type validation (enhanced)
```

**Test Files Status:**
- `tests/validation/schema_structure.rs` - ✅ **COMPLETE** (14 tests)
- `tests/validation/common_attributes.rs` - ✅ **COMPLETE** (22 tests)
- `tests/validation/data_types.rs` - ✅ **COMPLETE** (22 tests)
- `tests/validation/multi_valued.rs` - 🔲 **PHASE 4 TARGET**
- `tests/validation/complex_attributes.rs` - 🔲 **PHASE 5**
- `tests/validation/characteristics.rs` - 🔲 **PHASE 6**

This pattern has been proven to work across three completed phases. The established pattern and infrastructure are ready for Phase 4 multi-valued attribute validation.