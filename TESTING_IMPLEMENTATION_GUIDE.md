# SCIM Validation Testing Implementation Guide

This guide shows developers exactly how to implement new validation categories following the established pattern.

**Current Status:** Phase 2 Step 1 Complete - Validation logic implemented for common attributes (11/13 errors working). Next: Transform integration tests to use actual validation.

## Quick Start: Copy This Pattern

**Note:** Phase 2 validation logic is already implemented. For Phase 2 Step 2, skip to "Step 3: Update Your Test File" below. For Phase 3+, follow the complete pattern.

### Step 1: Add Error Types to `src/error.rs`

✅ **Phase 2 Complete** - All error types already added and working.

For future phases, add your error variants to the `ValidationError` enum:

```rust
/// Add to ValidationError enum
#[derive(Debug, thiserror::Error)]
pub enum ValidationError {
    // ... existing errors ...
    
    // Your new error category (e.g., Data Type Errors 22-32)
    /// Missing required attribute
    #[error("Required attribute '{attribute}' is missing")]
    MissingRequiredAttribute { attribute: String },  // Error #22
    
    /// Invalid data type for attribute  
    #[error("Attribute '{attribute}' has invalid type, expected {expected}, got {actual}")]
    InvalidDataType {
        attribute: String,
        expected: String, 
        actual: String,
    }, // Error #23
    
    /// Invalid string format
    #[error("Attribute '{attribute}' has invalid string format: {details}")]
    InvalidStringFormat { 
        attribute: String,
        details: String,
    }, // Error #24
    
    // ... add all errors for your category
}
```

### Step 2: Add Validation Functions to `src/schema.rs`

✅ **Phase 2 Complete** - All validation functions implemented and working:
- `validate_id_attribute()` - ID validation (Errors 9-12)
- `validate_external_id()` - External ID validation (Error 13)  
- Enhanced `validate_meta_attribute()` - Meta validation (Errors 14-21)

For future phases, add validation methods to the `SchemaRegistry` implementation:

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

🔲 **Phase 2 Step 2 NEXT** - Transform `tests/validation/common_attributes.rs` to use actual validation.

✅ **Template Available** - Follow the exact pattern from `tests/validation/schema_structure.rs`.

Follow this exact pattern in your test file (e.g., `tests/validation/common_attributes.rs`):

```rust
//! Data type validation tests.
//!
//! This module tests validation errors related to SCIM data types (Errors 22-32).

use serde_json::json;

// Import test utilities
use crate::common::{ValidationErrorCode, builders::UserBuilder, fixtures::rfc_examples};

// Import SCIM server types
use scim_server::error::{ValidationError};
use scim_server::schema::SchemaRegistry;

/// Test Error #22: Missing required attribute
#[test]
fn test_missing_required_attribute() {
    let registry = SchemaRegistry::new().expect("Failed to create registry");

    // Create a User resource missing required attribute
    let invalid_user = UserBuilder::new().without_username().build();

    // Verify the test data is constructed correctly
    assert!(!invalid_user.as_object().unwrap().contains_key("userName"));

    // Actually validate the resource
    let result = registry.validate_scim_resource(&invalid_user);

    // Assert that validation fails with the expected error
    assert!(result.is_err());
    match result {
        Err(ValidationError::MissingRequiredAttribute { attribute }) => {
            assert_eq!(attribute, "userName");
        }
        Err(other) => panic!("Expected MissingRequiredAttribute error, got {:?}", other),
        Ok(_) => panic!("Expected validation to fail, but it passed"),
    }
}

/// Test Error #23: Invalid data type
#[test] 
fn test_invalid_data_type() {
    let registry = SchemaRegistry::new().expect("Failed to create registry");

    // Create a User resource with wrong data type
    let invalid_user = UserBuilder::new().with_array_username().build();

    // Verify the test data has wrong type
    assert!(invalid_user["userName"].is_array());

    // Actually validate the resource  
    let result = registry.validate_scim_resource(&invalid_user);

    // Assert that validation fails with the expected error
    assert!(result.is_err());
    match result {
        Err(ValidationError::InvalidDataType { attribute, expected, actual }) => {
            assert_eq!(attribute, "userName");
            assert_eq!(expected, "string");
            assert_eq!(actual, "array");
        }
        Err(other) => panic!("Expected InvalidDataType error, got {:?}", other),
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

### Phase 2 Step 2: Transform Common Attributes Tests
- [x] ✅ Choose validation category: Common Attributes (Errors 9-21)
- [x] ✅ Error types already added to `ValidationError` enum in `src/error.rs`
- [x] ✅ Validation logic implemented in `src/schema.rs`
- [x] ✅ Integration with main validation complete
- [ ] 🔲 **NEXT:** Transform test file `tests/validation/common_attributes.rs`
- [ ] 🔲 Update 17 tests to use `registry.validate_scim_resource()` instead of builders
- [ ] 🔲 Follow pattern from `tests/validation/schema_structure.rs`
- [ ] 🔲 Verify tests pass: `cargo test validation::common_attributes --test lib`
- [ ] 🔲 Update documentation when complete

### Phase 3+ Implementation (Future)
- [ ] Choose your validation category (e.g., Data Types, Multi-valued, etc.)
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
# Phase 2 Step 2: Test common attributes specifically  
cargo test validation::common_attributes --test lib

# Verify Phase 1 still works
cargo test validation::schema_structure --test lib

# Run all validation tests
cargo test validation --test lib

# Full test suite
cargo test --test lib
```

## Phase 2 Step 2: Specific Instructions

**Current Task:** Transform `tests/validation/common_attributes.rs` from builder testing to actual validation testing.

**Pattern to Follow:** Copy exactly from `tests/validation/schema_structure.rs`:

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
   - `test_missing_id_attribute` → `ValidationError::MissingId`
   - `test_empty_id_value` → `ValidationError::EmptyId`  
   - `test_invalid_id_format` → `ValidationError::InvalidIdFormat`
   - `test_invalid_external_id_format` → `ValidationError::InvalidExternalId`
   - `test_invalid_meta_structure` → `ValidationError::InvalidMetaStructure`
   - `test_missing_meta_resource_type` → `ValidationError::MissingResourceType`
   - `test_invalid_meta_resource_type` → `ValidationError::InvalidResourceType`
   - `test_invalid_created_datetime` → `ValidationError::InvalidCreatedDateTime`
   - `test_invalid_last_modified_datetime` → `ValidationError::InvalidModifiedDateTime`
   - `test_invalid_location_uri` → `ValidationError::InvalidLocationUri`
   - `test_invalid_version_format` → `ValidationError::InvalidVersionFormat`
   - Plus valid case tests

## Next Steps After Implementation

1. Update `TESTING_PROGRESS.md` with Phase 2 completion
2. Verify all 139 integration tests still pass
3. Document any issues or patterns discovered
4. Begin Phase 3 planning (Data Type validation)

## Reference Files

- **Template:** `tests/validation/schema_structure.rs` - ✅ Copy this structure exactly
- **Error Types:** `src/error.rs` - ✅ Phase 2 errors already added
- **Validation Logic:** `src/schema.rs` - ✅ Phase 2 functions already implemented
- **Builders:** `tests/common/builders.rs` - ✅ All Phase 2 builders already exist
- **Progress:** `TESTING_PROGRESS.md` - 🔲 Update when Phase 2 Step 2 complete

## Current Implementation Status

**Phase 1:** ✅ **COMPLETE** - Schema structure validation (8/52 errors) fully working
**Phase 2 Step 1:** ✅ **COMPLETE** - Validation logic implemented (11/13 errors working)
**Phase 2 Step 2:** 🔲 **NEXT** - Transform integration tests to use validation logic

**Validation Functions Ready:**
```rust
// These are already implemented and working in src/schema.rs
validate_id_attribute()        // Errors 9-11: MissingId, EmptyId, InvalidIdFormat
validate_external_id()         // Error 13: InvalidExternalId  
validate_meta_attribute()      // Errors 14-21: Meta validation (enhanced)
```

**Test Files Status:**
- `tests/validation/schema_structure.rs` - ✅ **COMPLETE** (Pattern template)
- `tests/validation/common_attributes.rs` - 🔲 **READY FOR TRANSFORMATION**
- All other test files - 🔲 **FUTURE PHASES**

This pattern has been proven to work with schema structure validation (Errors 1-8) and the validation logic for common attributes (Errors 9-21). The validation functions are implemented and tested. Following the transformation pattern exactly will complete Phase 2.