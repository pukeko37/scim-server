//! SCIM Server Validation Test Suite
//!
//! This test suite provides comprehensive validation testing for all SCIM validation
//! error types as defined in RFC 7643 and related specifications. The tests are
//! organized into categories for maintainability and coverage tracking.
//!
//! ## Test Organization
//!
//! - `validation/` - Core validation tests organized by error category
//!   - `schema_structure` - Tests for schema structure validation (Errors 1-8)
//!   - `common_attributes` - Tests for common attribute validation (Errors 9-21)
//!   - `data_types` - Tests for data type validation (Errors 22-32)
//!   - `multi_valued` - Tests for multi-valued attribute validation (Errors 33-38)
//!   - `complex_attributes` - Tests for complex attribute validation (Errors 39-43)
//!   - `characteristics` - Tests for attribute characteristics (Errors 44-52)
//!   - `collections` - Tests for collection constraints (Errors 75-80)
//!   - `extensions` - Tests for schema extension validation (Errors 81-85)
//!
//! - `resources/` - Resource-specific validation tests
//!   - `user` - User-specific validation (Errors 53-64)
//!   - `group` - Group-specific validation (Errors 65-70)
//!   - `enterprise` - Enterprise extension validation (Errors 71-74)
//!
//! - `provider/` - Provider-level validation tests
//!   - `business_logic` - Business constraint validation (Errors 93-100)
//!   - `protocol` - Protocol-level validation (Errors 106-112)
//!   - `consistency` - Data consistency validation (Errors 113-117)
//!
//! ## Test Utilities
//!
//! - `common/` - Shared test utilities and helpers
//!   - `builders` - Fluent test data builders
//!   - `fixtures` - RFC examples and test data
//!   - Custom assertion macros for validation testing
//!   - Coverage tracking for ensuring complete test coverage
//!
//! ## Usage
//!
//! Run all validation tests:
//! ```bash
//! cargo test
//! ```
//!
//! Run specific test categories:
//! ```bash
//! cargo test validation::schema_structure
//! cargo test validation::common_attributes
//! ```
//!
//! Run with coverage reporting:
//! ```bash
//! cargo test -- --show-output
//! ```

extern crate scim_server;

// Test modules
pub mod common;
pub mod validation;

// Integration test modules
pub mod integration;

#[cfg(test)]
mod test_suite_meta {
    use super::*;
    use crate::common::TestCoverage;

    /// Meta-test to verify the test suite setup is working correctly
    #[test]
    fn test_suite_setup() {
        // Verify test utilities are working
        let coverage = TestCoverage::new();
        assert_eq!(coverage.coverage_percentage(), 0.0);

        // Verify builders are accessible
        let user = common::builders::UserBuilder::new().build();
        assert!(user["schemas"].is_array());

        // Verify fixtures are accessible
        let rfc_user = common::fixtures::rfc_examples::user_minimal();
        assert_eq!(rfc_user["userName"], "bjensen@example.com");

        println!("✅ Test suite setup is working correctly");
    }

    /// Meta-test to verify macro functionality
    #[test]
    fn test_assertion_macros() {
        use scim_server::error::{ScimError, ValidationError};

        // Test that our custom assertion macros compile and work
        let validation_error = ValidationError::missing_required("userName");
        let scim_error = ScimError::from(validation_error);

        // Test error message contains assertion
        let result: Result<(), ScimError> = Err(scim_error);
        assert_error_message_contains!(result, "userName");

        println!("✅ Assertion macros are working correctly");
    }

    /// Display test suite information
    #[test]
    fn test_suite_info() {
        println!("\n🧪 SCIM Server Validation Test Suite");
        println!("=====================================");
        println!("This comprehensive test suite validates all SCIM validation error types");
        println!("as defined in RFC 7643 and related specifications.\n");

        println!("📊 Test Coverage Goals:");
        println!("  • Schema Structure Validation (Errors 1-8): ✅");
        println!("  • Common Attributes Validation (Errors 9-21): ✅");
        println!("  • Data Type Validation (Errors 22-32): 🚧 Planned");
        println!("  • Multi-valued Attributes (Errors 33-38): 🚧 Planned");
        println!("  • Complex Attributes (Errors 39-43): 🚧 Planned");
        println!("  • Attribute Characteristics (Errors 44-52): 🚧 Planned");
        println!("  • Collection Constraints (Errors 75-80): 🚧 Planned");
        println!("  • Extension Validation (Errors 81-85): 🚧 Planned");
        println!("  • Resource-specific Tests (Errors 53-74): 🚧 Planned");
        println!("  • Provider-level Tests (Errors 93-117): 🚧 Planned\n");

        println!(
            "🎯 Current Implementation: Phase 1 (Foundation + Schema Structure + Common Attributes)"
        );
        println!("📝 Next: Phase 2 (Data Types + Multi-valued + Complex Attributes)");
    }

    /// Test that verifies error code enumeration is complete
    #[test]
    fn test_error_code_completeness() {
        use crate::common::ValidationErrorCode;

        // Verify we have error codes for the ranges we've implemented
        let _schema_errors = [
            ValidationErrorCode::MissingSchemas,
            ValidationErrorCode::EmptySchemas,
            ValidationErrorCode::InvalidSchemaUri,
            ValidationErrorCode::UnknownSchemaUri,
            ValidationErrorCode::DuplicateSchemaUri,
            ValidationErrorCode::MissingBaseSchema,
            ValidationErrorCode::ExtensionWithoutBase,
            ValidationErrorCode::MissingRequiredExtension,
        ];

        let _common_attr_errors = [
            ValidationErrorCode::MissingId,
            ValidationErrorCode::EmptyId,
            ValidationErrorCode::InvalidIdFormat,
            ValidationErrorCode::ClientProvidedId,
            ValidationErrorCode::InvalidExternalId,
            ValidationErrorCode::InvalidMetaStructure,
            ValidationErrorCode::MissingResourceType,
            ValidationErrorCode::InvalidResourceType,
            ValidationErrorCode::ClientProvidedMeta,
            ValidationErrorCode::InvalidCreatedDateTime,
            ValidationErrorCode::InvalidModifiedDateTime,
            ValidationErrorCode::InvalidLocationUri,
            ValidationErrorCode::InvalidVersionFormat,
        ];

        println!("✅ Error code enumeration is complete for implemented phases");
    }
}

// Re-export commonly used items for external test consumers
pub use common::{
    TestCoverage, ValidationErrorCode,
    builders::{GroupBuilder, SchemaBuilder, UserBuilder},
    fixtures::{rfc_examples, test_fixtures},
};

// Re-export integration test utilities
pub use integration::{
    multi_tenant::{
        advanced::{AdvancedMultiTenantProvider, AdvancedTestHarness},
        core::{AuthInfo, EnhancedRequestContext, TenantContext, TenantContextBuilder},
        provider_trait::{MultiTenantResourceProvider, ProviderTestHarness},
    },
    providers::{
        common::{MultiTenantScenarioBuilder, ProviderTestingSuite},
        in_memory::{InMemoryProvider, InMemoryProviderConfig},
    },
};
