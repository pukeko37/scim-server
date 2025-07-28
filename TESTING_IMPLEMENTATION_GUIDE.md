# SCIM Validation Testing Implementation Guide

This guide shows developers exactly how to implement new validation categories following the established pattern.

## Quick Start: Copy This Pattern

### Step 1: Add Error Types to `src/error.rs`

Add your error variants to the `ValidationError` enum:

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

Add validation methods to the `SchemaRegistry` implementation:

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

Follow this exact pattern in your test file (e.g., `tests/validation/data_types.rs`):

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

### Phase Planning
- [ ] Choose your validation category (e.g., Data Types, Multi-valued, etc.)
- [ ] List all error types for your category from `tests/common/mod.rs`
- [ ] Identify which builder methods already exist in `tests/common/builders.rs`

### Error Types Implementation
- [ ] Add all error variants to `ValidationError` enum in `src/error.rs`
- [ ] Follow naming convention: `ErrorName { field: Type }`
- [ ] Include descriptive error messages with `#[error("...")]`
- [ ] Test compilation: `cargo check`

### Validation Logic Implementation  
- [ ] Add main validation function to `SchemaRegistry` in `src/schema.rs`
- [ ] Break down into smaller helper functions for each validation aspect
- [ ] Use existing helper patterns (e.g., `extract_schema_uris`, `is_valid_schema_uri`)
- [ ] Return specific error types, not generic `ValidationError::custom()`
- [ ] Test compilation: `cargo check`

### Integration with Main Validation
- [ ] Update `validate_scim_resource()` to call your validation function
- [ ] Ensure proper error propagation
- [ ] Test basic functionality: `cargo test --lib`

### Test Implementation
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
# Test your category specifically
cargo test validation::your_category --test lib

# Test that existing functionality still works
cargo test validation::schema_structure --test lib

# Run all validation tests
cargo test validation --test lib

# Full test suite
cargo test --test lib
```

## Next Steps After Implementation

1. Update `TESTING_PROGRESS.md` with your completed phase
2. Document any new patterns or helpers you created
3. Consider if your validation functions could be useful for other categories
4. Plan the next category following the roadmap in `TESTING_PROGRESS.md`

## Reference Files

- **Template:** `tests/validation/schema_structure.rs` - Copy this structure
- **Error Types:** `src/error.rs` - Add your errors here
- **Validation Logic:** `src/schema.rs` - Add your functions here  
- **Builders:** `tests/common/builders.rs` - Add missing builders here
- **Progress:** `TESTING_PROGRESS.md` - Update when complete

This pattern has been proven to work with the schema structure validation (Errors 1-8). Following it exactly will ensure your implementation integrates smoothly with the existing codebase.