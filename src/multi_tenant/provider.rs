//! Multi-tenant provider utilities and validation helpers.
//!
//! This module provides utilities for working with the unified ResourceProvider
//! in multi-tenant scenarios. The unified ResourceProvider supports both single
//! and multi-tenant operations through the RequestContext.

use crate::resource::RequestContext;
#[cfg(test)]
use crate::resource::TenantContext;

/// Helper trait for validating tenant context in provider operations.
///
/// This trait provides common validation logic that can be reused across
/// different multi-tenant provider implementations.
pub trait TenantValidator {
    /// Validate that the context has the expected tenant.
    fn validate_tenant_context(
        &self,
        expected_tenant_id: &str,
        context: &RequestContext,
    ) -> Result<(), String> {
        match context.tenant_id() {
            Some(actual_tenant_id) if actual_tenant_id == expected_tenant_id => Ok(()),
            Some(actual_tenant_id) => Err(format!(
                "Tenant mismatch: context has '{}', operation requested '{}'",
                actual_tenant_id, expected_tenant_id
            )),
            None => Err(format!(
                "Multi-tenant operation requested '{}' but context has no tenant",
                expected_tenant_id
            )),
        }
    }

    /// Validate that the context is for single-tenant operation.
    fn validate_single_tenant_context(&self, context: &RequestContext) -> Result<(), String> {
        match context.tenant_id() {
            None => Ok(()),
            Some(tenant_id) => Err(format!(
                "Single-tenant operation but context has tenant '{}'",
                tenant_id
            )),
        }
    }

    /// Extract tenant context or return error for multi-tenant operations.
    fn require_tenant_context(&self, context: &RequestContext) -> Result<(), String> {
        match context.tenant_context {
            Some(_) => Ok(()),
            None => Err("Multi-tenant operation requires tenant context".to_string()),
        }
    }
}

/// Default implementation of TenantValidator for any type.
impl<T> TenantValidator for T {}

#[cfg(test)]
mod tests {
    use super::*;

    struct MockValidator;

    #[test]
    fn test_tenant_validator_success() {
        let validator = MockValidator;
        let tenant_context = TenantContext {
            tenant_id: "test-tenant".to_string(),
            client_id: "client".to_string(),
            permissions: Default::default(),
            isolation_level: Default::default(),
        };
        let context = RequestContext::with_tenant_generated_id(tenant_context);

        // Should succeed with matching tenant
        assert!(
            validator
                .validate_tenant_context("test-tenant", &context)
                .is_ok()
        );

        // Should succeed with tenant context requirement
        assert!(validator.require_tenant_context(&context).is_ok());
    }

    #[test]
    fn test_tenant_validator_failure() {
        let validator = MockValidator;
        let tenant_context = TenantContext {
            tenant_id: "test-tenant".to_string(),
            client_id: "client".to_string(),
            permissions: Default::default(),
            isolation_level: Default::default(),
        };
        let context = RequestContext::with_tenant_generated_id(tenant_context);

        // Should fail with mismatched tenant
        let result = validator.validate_tenant_context("different-tenant", &context);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Tenant mismatch"));
    }

    #[test]
    fn test_single_tenant_validation() {
        let validator = MockValidator;

        // Single-tenant context should pass single-tenant validation
        let context = RequestContext::with_generated_id();
        assert!(validator.validate_single_tenant_context(&context).is_ok());

        // Multi-tenant context should fail single-tenant validation
        let tenant_context = TenantContext {
            tenant_id: "test-tenant".to_string(),
            client_id: "client".to_string(),
            permissions: Default::default(),
            isolation_level: Default::default(),
        };
        let multi_context = RequestContext::with_tenant_generated_id(tenant_context);
        assert!(
            validator
                .validate_single_tenant_context(&multi_context)
                .is_err()
        );
    }

    #[test]
    fn test_require_tenant_context() {
        let validator = MockValidator;

        // Should fail for single-tenant context
        let single_context = RequestContext::with_generated_id();
        assert!(validator.require_tenant_context(&single_context).is_err());

        // Should succeed for multi-tenant context
        let tenant_context = TenantContext {
            tenant_id: "test-tenant".to_string(),
            client_id: "client".to_string(),
            permissions: Default::default(),
            isolation_level: Default::default(),
        };
        let multi_context = RequestContext::with_tenant_generated_id(tenant_context);
        assert!(validator.require_tenant_context(&multi_context).is_ok());
    }
}
