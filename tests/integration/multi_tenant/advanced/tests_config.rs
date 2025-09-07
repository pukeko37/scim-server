//! Configuration tests for advanced multi-tenant features.
//!
//! This module contains tests for tenant-specific schema customization,
//! compliance level enforcement, and feature flag functionality.

use super::{
    config::{AdvancedTenantConfig, ComplianceLevel},
    integration::TestAdvancedProvider,
};
use scim_server::ResourceProvider;
use scim_server::resource::{RequestContext, TenantContext};
use serde_json::json;

#[cfg(test)]
mod config_tests {
    use super::*;

    fn create_test_context(tenant_id: &str) -> RequestContext {
        let tenant_context = TenantContext::new(tenant_id.to_string(), "test-client".to_string());
        RequestContext::with_tenant(format!("req_{}", tenant_id), tenant_context)
    }

    fn create_test_user(username: &str) -> serde_json::Value {
        json!({
            "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
            "userName": username,
            "displayName": format!("{} User", username),
            "active": true
        })
    }

    #[tokio::test]
    async fn test_tenant_specific_schema_configuration() {
        let provider = TestAdvancedProvider::new();

        // Configure tenant with custom schema requirements
        let config = AdvancedTenantConfig::new("enterprise_tenant")
            .with_compliance_level(ComplianceLevel::Enhanced)
            .with_feature_flag("custom_attributes", true)
            .with_feature_flag("audit_logging", true);

        provider.configure_tenant(config).await;

        let context = create_test_context("enterprise_tenant");

        // Create user with enterprise-specific requirements
        let enterprise_user = json!({
            "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
            "userName": "enterprise.user@company.com",
            "displayName": "Enterprise User",
            "active": true,
            "employeeNumber": "EMP001",
            "department": "Engineering",
            "complianceLevel": "Enhanced"
        });

        let result = provider
            .create_resource("User", enterprise_user, &context)
            .await;

        assert!(result.is_ok());
        let created_user = result.unwrap();
        assert_eq!(
            created_user
                .resource()
                .get_attribute("employeeNumber")
                .unwrap(),
            &json!("EMP001")
        );
    }

    #[tokio::test]
    async fn test_compliance_level_enforcement() {
        let provider = TestAdvancedProvider::new();

        // Configure different tenants with different compliance levels
        let basic_config =
            AdvancedTenantConfig::new("basic_tenant").with_compliance_level(ComplianceLevel::Basic);

        let strict_config = AdvancedTenantConfig::new("strict_tenant")
            .with_compliance_level(ComplianceLevel::Strict);

        provider.configure_tenant(basic_config).await;
        provider.configure_tenant(strict_config).await;

        let basic_context = create_test_context("basic_tenant");
        let strict_context = create_test_context("strict_tenant");

        // Both tenants should support basic operations
        let basic_user = provider
            .create_resource("User", create_test_user("basic_user"), &basic_context)
            .await;
        assert!(basic_user.is_ok());

        let strict_user = provider
            .create_resource("User", create_test_user("strict_user"), &strict_context)
            .await;
        assert!(strict_user.is_ok());

        // Verify compliance levels are properly configured
        let configs = provider.tenant_configs.read().await;
        assert_eq!(
            configs.get("basic_tenant").unwrap().compliance_level,
            ComplianceLevel::Basic
        );
        assert_eq!(
            configs.get("strict_tenant").unwrap().compliance_level,
            ComplianceLevel::Strict
        );
    }

    #[tokio::test]
    async fn test_feature_flag_configuration() {
        let provider = TestAdvancedProvider::new();

        // Configure tenant with specific feature flags
        let config = AdvancedTenantConfig::new("feature_test_tenant")
            .with_feature_flag("bulk_operations", true)
            .with_feature_flag("audit_logging", false)
            .with_feature_flag("custom_schemas", true);

        provider.configure_tenant(config).await;

        // Verify feature flags are set correctly
        let configs = provider.tenant_configs.read().await;
        let tenant_config = configs.get("feature_test_tenant").unwrap();

        assert_eq!(
            *tenant_config.feature_flags.get("bulk_operations").unwrap(),
            true
        );
        assert_eq!(
            *tenant_config.feature_flags.get("audit_logging").unwrap(),
            false
        );
        assert_eq!(
            *tenant_config.feature_flags.get("custom_schemas").unwrap(),
            true
        );
    }

    #[tokio::test]
    async fn test_tenant_isolation_in_configuration() {
        let provider = TestAdvancedProvider::new();

        // Configure two different tenants
        let tenant_a_config = AdvancedTenantConfig::new("tenant_a")
            .with_compliance_level(ComplianceLevel::Basic)
            .with_feature_flag("feature_x", true);

        let tenant_b_config = AdvancedTenantConfig::new("tenant_b")
            .with_compliance_level(ComplianceLevel::Enhanced)
            .with_feature_flag("feature_x", false);

        provider.configure_tenant(tenant_a_config).await;
        provider.configure_tenant(tenant_b_config).await;

        // Verify each tenant has its own isolated configuration
        let configs = provider.tenant_configs.read().await;

        let config_a = configs.get("tenant_a").unwrap();
        let config_b = configs.get("tenant_b").unwrap();

        assert_eq!(config_a.compliance_level, ComplianceLevel::Basic);
        assert_eq!(config_b.compliance_level, ComplianceLevel::Enhanced);

        assert_eq!(*config_a.feature_flags.get("feature_x").unwrap(), true);
        assert_eq!(*config_b.feature_flags.get("feature_x").unwrap(), false);
    }

    #[tokio::test]
    async fn test_data_retention_configuration() {
        let provider = TestAdvancedProvider::new();

        // Configure tenant with data retention policy
        let mut config = AdvancedTenantConfig::new("retention_tenant");
        config.data_retention_days = Some(90);

        provider.configure_tenant(config).await;

        // Verify data retention is configured
        let configs = provider.tenant_configs.read().await;
        let tenant_config = configs.get("retention_tenant").unwrap();

        assert_eq!(tenant_config.data_retention_days, Some(90));
    }
}
