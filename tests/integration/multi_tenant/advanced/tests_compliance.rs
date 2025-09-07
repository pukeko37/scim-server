//! Compliance and audit logging tests for advanced multi-tenant features.
//!
//! This module contains tests for audit logging, compliance metadata,
//! data retention policies, and regulatory compliance features.

use super::{
    compliance::{AuditLogEntry, ComplianceMetadata},
    config::{AdvancedTenantConfig, ComplianceLevel},
    integration::{AdvancedMultiTenantProvider, TestAdvancedProvider},
};
use scim_server::ResourceProvider;
use scim_server::resource::{RequestContext, TenantContext};
use serde_json::json;

#[cfg(test)]
mod compliance_tests {
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
    async fn test_audit_logging_for_tenant_operations() {
        let provider = TestAdvancedProvider::new();
        let tenant_id = "audit_tenant";
        let context = create_test_context(tenant_id);

        // Perform various operations that should be logged
        let user = provider
            .create_resource("User", create_test_user("audit_user"), &context)
            .await
            .unwrap();

        let user_id = user.resource().get_id().unwrap();

        let _retrieved = provider
            .get_resource("User", &user_id, &context)
            .await
            .unwrap();

        let _updated = provider
            .update_resource(
                "User",
                &user_id,
                json!({
                    "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
                    "userName": "audit_user",
                    "displayName": "Updated Audit User",
                    "active": true
                }),
                None,
                &context,
            )
            .await
            .unwrap();

        provider
            .delete_resource("User", &user_id, None, &context)
            .await
            .unwrap();

        // Check audit log
        let audit_entries = provider
            .get_audit_log(tenant_id, None, None, &context)
            .await
            .unwrap();

        assert!(audit_entries.len() >= 4);

        // Verify all operations are logged
        let operations: Vec<&str> = audit_entries
            .iter()
            .map(|entry| entry.operation.as_str())
            .collect();

        assert!(operations.contains(&"create"));
        assert!(operations.contains(&"get"));
        assert!(operations.contains(&"update"));
        assert!(operations.contains(&"delete"));

        // Verify tenant isolation in audit log
        for entry in &audit_entries {
            assert_eq!(entry.tenant_id, tenant_id);
        }
    }

    #[tokio::test]
    async fn test_audit_log_time_filtering() {
        let provider = TestAdvancedProvider::new();
        let tenant_id = "time_filter_tenant";
        let context = create_test_context(tenant_id);

        let _start_time = chrono::Utc::now();

        // Create some resources
        let _user1 = provider
            .create_resource("User", create_test_user("time_user1"), &context)
            .await
            .unwrap();

        // Wait a bit to ensure time difference
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

        let middle_time = chrono::Utc::now();

        let _user2 = provider
            .create_resource("User", create_test_user("time_user2"), &context)
            .await
            .unwrap();

        let _end_time = chrono::Utc::now();

        // Get all entries
        let all_entries = provider
            .get_audit_log(tenant_id, None, None, &context)
            .await
            .unwrap();

        // Get entries from middle time onwards
        let filtered_entries = provider
            .get_audit_log(tenant_id, Some(middle_time), None, &context)
            .await
            .unwrap();

        // Should have fewer entries when filtering by time
        assert!(filtered_entries.len() < all_entries.len());
        assert!(filtered_entries.len() >= 1); // At least the second user creation
    }

    #[tokio::test]
    async fn test_audit_log_tenant_isolation() {
        let provider = TestAdvancedProvider::new();
        let tenant_a = "audit_tenant_a";
        let tenant_b = "audit_tenant_b";
        let context_a = create_test_context(tenant_a);
        let context_b = create_test_context(tenant_b);

        // Perform operations in both tenants
        let _user_a = provider
            .create_resource("User", create_test_user("user_a"), &context_a)
            .await
            .unwrap();

        let _user_b = provider
            .create_resource("User", create_test_user("user_b"), &context_b)
            .await
            .unwrap();

        // Get audit logs for each tenant
        let audit_a = provider
            .get_audit_log(tenant_a, None, None, &context_a)
            .await
            .unwrap();

        let audit_b = provider
            .get_audit_log(tenant_b, None, None, &context_b)
            .await
            .unwrap();

        // Verify each tenant only sees its own audit entries
        for entry in &audit_a {
            assert_eq!(entry.tenant_id, tenant_a);
        }

        for entry in &audit_b {
            assert_eq!(entry.tenant_id, tenant_b);
        }

        // Verify no cross-contamination
        assert!(audit_a.len() >= 1);
        assert!(audit_b.len() >= 1);
    }

    #[tokio::test]
    async fn test_compliance_metadata_creation() {
        let metadata = ComplianceMetadata::new("sensitive".to_string())
            .with_retention_period(365)
            .with_access_justification("Business requirement".to_string());

        assert_eq!(metadata.data_classification, "sensitive");
        assert_eq!(metadata.retention_period, Some(365));
        assert_eq!(
            metadata.access_justification,
            Some("Business requirement".to_string())
        );
    }

    #[tokio::test]
    async fn test_audit_entry_builder_pattern() {
        let entry = AuditLogEntry::new(
            "test_tenant".to_string(),
            "create".to_string(),
            "User".to_string(),
        )
        .with_user_id("user123".to_string())
        .with_resource_id("resource456".to_string())
        .with_detail("attribute".to_string(), json!("value"))
        .with_compliance_metadata(
            ComplianceMetadata::new("public".to_string()).with_retention_period(90),
        );

        assert_eq!(entry.tenant_id, "test_tenant");
        assert_eq!(entry.operation, "create");
        assert_eq!(entry.resource_type, "User");
        assert_eq!(entry.user_id, Some("user123".to_string()));
        assert_eq!(entry.resource_id, Some("resource456".to_string()));
        assert!(entry.details.contains_key("attribute"));
        assert!(entry.compliance_metadata.is_some());

        let compliance = entry.compliance_metadata.unwrap();
        assert_eq!(compliance.data_classification, "public");
        assert_eq!(compliance.retention_period, Some(90));
    }

    #[tokio::test]
    async fn test_gdpr_compliance_logging() {
        let provider = TestAdvancedProvider::new();

        // Configure tenant with GDPR compliance
        let config = AdvancedTenantConfig::new("gdpr_tenant")
            .with_compliance_level(ComplianceLevel::Strict)
            .with_feature_flag("gdpr_compliance", true)
            .with_feature_flag("audit_logging", true);

        provider.configure_tenant(config).await;

        let context = create_test_context("gdpr_tenant");

        // Create a user with PII data
        let pii_user = json!({
            "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
            "userName": "gdpr.user@example.com",
            "displayName": "GDPR Test User",
            "active": true,
            "emails": [{
                "value": "gdpr.user@example.com",
                "type": "work",
                "primary": true
            }],
            "name": {
                "givenName": "John",
                "familyName": "Doe"
            }
        });

        let user = provider
            .create_resource("User", pii_user, &context)
            .await
            .unwrap();

        // Verify audit logging for GDPR compliance
        let audit_entries = provider
            .get_audit_log("gdpr_tenant", None, None, &context)
            .await
            .unwrap();

        assert!(!audit_entries.is_empty());

        // In a real implementation, we would verify:
        // - PII data is properly classified
        // - Retention policies are applied
        // - Access justification is recorded
        let create_entry = audit_entries
            .iter()
            .find(|e| e.operation == "create")
            .unwrap();

        assert_eq!(create_entry.tenant_id, "gdpr_tenant");
        assert_eq!(create_entry.resource_type, "User");
        assert_eq!(
            create_entry.resource_id,
            user.resource().get_id().map(|s| s.to_string())
        );
    }

    #[tokio::test]
    async fn test_data_retention_policy_enforcement() {
        let provider = TestAdvancedProvider::new();

        // Configure tenant with 30-day retention
        let mut config = AdvancedTenantConfig::new("retention_tenant");
        config.data_retention_days = Some(30);
        config.compliance_level = ComplianceLevel::Enhanced;

        provider.configure_tenant(config).await;

        let context = create_test_context("retention_tenant");

        // Create user that should be subject to retention policy
        let user = provider
            .create_resource("User", create_test_user("retention_user"), &context)
            .await
            .unwrap();

        // Verify tenant configuration includes retention policy
        let configs = provider.tenant_configs.read().await;
        let tenant_config = configs.get("retention_tenant").unwrap();
        assert_eq!(tenant_config.data_retention_days, Some(30));

        // In a real implementation, this would trigger retention policy checks
        // For now, just verify the user was created and config is correct
        assert!(user.resource().get_id().is_some());
    }

    #[tokio::test]
    async fn test_audit_log_compliance_levels() {
        let provider = TestAdvancedProvider::new();

        // Test different compliance levels
        let compliance_levels = vec![
            ("basic_compliance", ComplianceLevel::Basic),
            ("standard_compliance", ComplianceLevel::Standard),
            ("enhanced_compliance", ComplianceLevel::Enhanced),
            ("strict_compliance", ComplianceLevel::Strict),
        ];

        for (tenant_id, level) in compliance_levels {
            let config = AdvancedTenantConfig::new(tenant_id)
                .with_compliance_level(level.clone())
                .with_feature_flag("audit_logging", true);

            provider.configure_tenant(config).await;

            let context = create_test_context(tenant_id);

            // Create a user to generate audit entry
            let _user = provider
                .create_resource("User", create_test_user("compliance_user"), &context)
                .await
                .unwrap();

            // Verify audit logging works for all compliance levels
            let audit_entries = provider
                .get_audit_log(tenant_id, None, None, &context)
                .await
                .unwrap();

            assert!(!audit_entries.is_empty());

            // Verify tenant config has correct compliance level
            let configs = provider.tenant_configs.read().await;
            let tenant_config = configs.get(tenant_id).unwrap();
            assert_eq!(tenant_config.compliance_level, level);
        }
    }

    #[tokio::test]
    async fn test_cross_tenant_audit_isolation() {
        let provider = TestAdvancedProvider::new();

        // Set up multiple tenants with different compliance requirements
        let tenants = vec![
            ("finance_tenant", ComplianceLevel::Strict),
            ("marketing_tenant", ComplianceLevel::Standard),
            ("dev_tenant", ComplianceLevel::Basic),
        ];

        for (tenant_id, compliance_level) in &tenants {
            let config = AdvancedTenantConfig::new(tenant_id)
                .with_compliance_level(compliance_level.clone())
                .with_feature_flag("audit_logging", true);

            provider.configure_tenant(config).await;
        }

        // Perform operations in each tenant
        for (tenant_id, _) in &tenants {
            let context = create_test_context(tenant_id);
            let _user = provider
                .create_resource(
                    "User",
                    create_test_user(&format!("{}_user", tenant_id)),
                    &context,
                )
                .await
                .unwrap();
        }

        // Verify each tenant only sees its own audit entries
        for (tenant_id, _) in &tenants {
            let context = create_test_context(tenant_id);
            let audit_entries = provider
                .get_audit_log(tenant_id, None, None, &context)
                .await
                .unwrap();

            // Each tenant should have exactly one audit entry (the create operation)
            assert_eq!(audit_entries.len(), 1);
            assert_eq!(audit_entries[0].tenant_id, *tenant_id);
            assert_eq!(audit_entries[0].operation, "create");

            // Verify no other tenant's data is visible
            for entry in &audit_entries {
                assert_eq!(entry.tenant_id, *tenant_id);
            }
        }
    }

    #[tokio::test]
    async fn test_audit_log_pagination_and_filtering() {
        let provider = TestAdvancedProvider::new();
        let tenant_id = "pagination_tenant";
        let context = create_test_context(tenant_id);

        // Create multiple operations to test pagination
        for i in 0..5 {
            let _user = provider
                .create_resource(
                    "User",
                    create_test_user(&format!("pagination_user_{}", i)),
                    &context,
                )
                .await
                .unwrap();
        }

        // Get all audit entries
        let all_entries = provider
            .get_audit_log(tenant_id, None, None, &context)
            .await
            .unwrap();

        assert_eq!(all_entries.len(), 5);

        // Test time-based filtering
        let start_time = all_entries[2].timestamp;
        let filtered_entries = provider
            .get_audit_log(tenant_id, Some(start_time), None, &context)
            .await
            .unwrap();

        // Should include entries from index 2 onwards
        assert!(filtered_entries.len() <= 3);
        assert!(filtered_entries.len() >= 1);

        // Verify all filtered entries are after start_time
        for entry in &filtered_entries {
            assert!(entry.timestamp >= start_time);
        }
    }

    #[tokio::test]
    async fn test_sensitive_data_masking_in_audit_logs() {
        let provider = TestAdvancedProvider::new();

        // Configure tenant for sensitive data handling
        let config = AdvancedTenantConfig::new("sensitive_tenant")
            .with_compliance_level(ComplianceLevel::Strict)
            .with_feature_flag("data_masking", true)
            .with_feature_flag("audit_logging", true);

        provider.configure_tenant(config).await;

        let context = create_test_context("sensitive_tenant");

        // Create user with sensitive information
        let sensitive_user = json!({
            "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
            "userName": "sensitive.user@example.com",
            "displayName": "Sensitive User",
            "active": true,
            "phoneNumbers": [{
                "value": "+1-555-123-4567",
                "type": "work"
            }],
            "addresses": [{
                "streetAddress": "123 Sensitive St",
                "locality": "Privacy City",
                "region": "Secure State",
                "postalCode": "12345",
                "country": "US"
            }]
        });

        let _user = provider
            .create_resource("User", sensitive_user, &context)
            .await
            .unwrap();

        // Verify audit entry was created
        let audit_entries = provider
            .get_audit_log("sensitive_tenant", None, None, &context)
            .await
            .unwrap();

        assert!(!audit_entries.is_empty());

        // In a production implementation, sensitive data in audit logs would be masked
        // For now, just verify the audit entry exists and contains operation details
        let create_entry = audit_entries
            .iter()
            .find(|e| e.operation == "create")
            .unwrap();

        assert_eq!(create_entry.tenant_id, "sensitive_tenant");
        assert_eq!(create_entry.resource_type, "User");
    }
}
