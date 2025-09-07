//! Integration tests for advanced multi-tenant features.
//!
//! This module contains comprehensive integration tests that validate
//! error scenarios, end-to-end functionality, and documentation of
//! advanced multi-tenant capabilities.

use super::{
    bulk_operations::{BulkOperation, BulkOperationRequest, BulkOperationType},
    config::{AdvancedTenantConfig, ComplianceLevel, CustomValidationRule, ValidationRuleType},
    integration::{AdvancedMultiTenantProvider, TestAdvancedProvider},
};
use scim_server::ResourceProvider;
use scim_server::resource::{RequestContext, TenantContext};
use serde_json::json;

#[cfg(test)]
mod integration_tests {
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
    async fn test_advanced_error_scenarios() {
        let provider = TestAdvancedProvider::new();
        let tenant_id = "error_advanced_tenant";
        let context = create_test_context(tenant_id);

        // Test bulk operation with all failures
        let failing_bulk_request = BulkOperationRequest {
            tenant_id: tenant_id.to_string(),
            operations: vec![
                BulkOperation {
                    operation_type: BulkOperationType::Create,
                    resource_type: "User".to_string(),
                    resource_id: None,
                    data: None, // Missing data
                },
                BulkOperation {
                    operation_type: BulkOperationType::Delete,
                    resource_type: "User".to_string(),
                    resource_id: None, // Missing resource ID
                    data: None,
                },
            ],
            fail_on_errors: false,
            continue_on_error: true,
        };

        let result = provider
            .execute_bulk_operation(failing_bulk_request, &context)
            .await
            .unwrap();

        assert_eq!(result.successful_operations, 0);
        assert_eq!(result.failed_operations, 2);
        assert_eq!(result.results.len(), 2);

        for result_item in &result.results {
            assert!(!result_item.success);
            assert!(result_item.error.is_some());
        }
    }

    #[tokio::test]
    async fn test_bulk_operation_fail_fast() {
        let provider = TestAdvancedProvider::new();
        let tenant_id = "fail_fast_tenant";
        let context = create_test_context(tenant_id);

        let bulk_request = BulkOperationRequest {
            tenant_id: tenant_id.to_string(),
            operations: vec![
                BulkOperation {
                    operation_type: BulkOperationType::Create,
                    resource_type: "User".to_string(),
                    resource_id: None,
                    data: Some(create_test_user("good_user")),
                },
                BulkOperation {
                    operation_type: BulkOperationType::Create,
                    resource_type: "User".to_string(),
                    resource_id: None,
                    data: None, // This should fail
                },
                BulkOperation {
                    operation_type: BulkOperationType::Create,
                    resource_type: "User".to_string(),
                    resource_id: None,
                    data: Some(create_test_user("never_created")),
                },
            ],
            fail_on_errors: true, // Should stop on first error
            continue_on_error: false,
        };

        let result = provider
            .execute_bulk_operation(bulk_request, &context)
            .await
            .unwrap();

        assert_eq!(result.successful_operations, 1);
        assert_eq!(result.failed_operations, 1);
        assert_eq!(result.results.len(), 2); // Should stop after the failure

        // Verify only the first user was created
        let users = provider
            .list_resources("User", None, &context)
            .await
            .unwrap();

        assert_eq!(users.len(), 1);
        assert_eq!(users[0].resource().get_username().unwrap(), "good_user");
    }

    #[tokio::test]
    async fn test_advanced_multi_tenant_integration() {
        println!("\n🚀 Advanced Multi-Tenant Integration Test");
        println!("=========================================");

        let provider = TestAdvancedProvider::new();

        // Configure multiple tenants with different requirements
        let enterprise_config = AdvancedTenantConfig::new("enterprise_corp")
            .with_compliance_level(ComplianceLevel::Enhanced)
            .with_feature_flag("bulk_operations", true)
            .with_feature_flag("audit_logging", true)
            .with_feature_flag("custom_schemas", true);

        let startup_config = AdvancedTenantConfig::new("startup_inc")
            .with_compliance_level(ComplianceLevel::Basic)
            .with_feature_flag("bulk_operations", false)
            .with_feature_flag("audit_logging", false);

        provider.configure_tenant(enterprise_config).await;
        provider.configure_tenant(startup_config).await;

        let enterprise_context = create_test_context("enterprise_corp");
        let startup_context = create_test_context("startup_inc");

        // Test enterprise features
        let enterprise_bulk = BulkOperationRequest {
            tenant_id: "enterprise_corp".to_string(),
            operations: vec![
                BulkOperation {
                    operation_type: BulkOperationType::Create,
                    resource_type: "User".to_string(),
                    resource_id: None,
                    data: Some(create_test_user("enterprise_user1")),
                },
                BulkOperation {
                    operation_type: BulkOperationType::Create,
                    resource_type: "User".to_string(),
                    resource_id: None,
                    data: Some(create_test_user("enterprise_user2")),
                },
            ],
            fail_on_errors: false,
            continue_on_error: true,
        };

        let enterprise_result = provider
            .execute_bulk_operation(enterprise_bulk, &enterprise_context)
            .await
            .unwrap();

        assert_eq!(enterprise_result.successful_operations, 2);

        // Test startup simple operations
        let _startup_user = provider
            .create_resource("User", create_test_user("startup_user"), &startup_context)
            .await
            .unwrap();

        // Verify tenant isolation and statistics
        let enterprise_stats = provider
            .get_tenant_statistics("enterprise_corp", &enterprise_context)
            .await
            .unwrap();

        let startup_stats = provider
            .get_tenant_statistics("startup_inc", &startup_context)
            .await
            .unwrap();

        assert_eq!(enterprise_stats.total_resources, 2);
        assert_eq!(startup_stats.total_resources, 1);

        // Verify audit logging works for enterprise tenant
        let enterprise_audit = provider
            .get_audit_log("enterprise_corp", None, None, &enterprise_context)
            .await
            .unwrap();

        assert!(enterprise_audit.len() >= 2); // At least the bulk operations

        println!(
            "✅ Enterprise tenant: {} resources, {} audit entries",
            enterprise_stats.total_resources,
            enterprise_audit.len()
        );
        println!(
            "✅ Startup tenant: {} resources",
            startup_stats.total_resources
        );
        println!("✅ All tenants properly isolated and configured");
    }

    #[tokio::test]
    async fn test_advanced_features_documentation() {
        println!("\n🎯 Advanced Multi-Tenant Features Test Documentation");
        println!("===================================================");
        println!("This comprehensive test suite validates advanced multi-tenant");
        println!("functionality for enterprise SaaS applications.\n");

        println!("✅ Advanced Features Tested:");
        println!("  • Tenant-specific schema customization");
        println!("  • Compliance level enforcement");
        println!("  • Bulk operations with tenant isolation");
        println!("  • Comprehensive audit logging");
        println!("  • Tenant statistics and monitoring");
        println!("  • Performance optimization");
        println!("  • Advanced error handling\n");

        println!("🔒 Enterprise Security Features:");
        println!("  • Multi-level compliance support (Basic → Strict)");
        println!("  • Complete audit trail with time filtering");
        println!("  • Tenant-scoped bulk operations");
        println!("  • Cross-tenant access prevention");
        println!("  • Custom validation rules per tenant\n");

        println!("⚡ Performance & Scalability:");
        println!("  • Efficient bulk operations");
        println!("  • Concurrent multi-tenant access");
        println!("  • Resource usage monitoring");
        println!("  • Optimized audit log queries\n");

        println!("🏢 Enterprise Use Cases:");
        println!("  • Large-scale user provisioning");
        println!("  • Compliance-driven access control");
        println!("  • Multi-organization data isolation");
        println!("  • Audit trail for regulatory compliance");
        println!("  • Performance monitoring and optimization\n");

        println!("🎯 Production Readiness:");
        println!("  • Complete error handling and recovery");
        println!("  • Fail-fast and continue-on-error strategies");
        println!("  • Resource lifecycle management");
        println!("  • Advanced monitoring and statistics");
        println!("  • Enterprise-grade security and compliance");
    }

    #[tokio::test]
    async fn test_cross_tenant_operation_isolation() {
        let provider = TestAdvancedProvider::new();

        // Configure tenants with different compliance levels
        let configs = vec![
            ("tenant_basic", ComplianceLevel::Basic),
            ("tenant_standard", ComplianceLevel::Standard),
            ("tenant_enhanced", ComplianceLevel::Enhanced),
            ("tenant_strict", ComplianceLevel::Strict),
        ];

        for (tenant_id, compliance_level) in &configs {
            let config = AdvancedTenantConfig::new(tenant_id)
                .with_compliance_level(compliance_level.clone())
                .with_feature_flag("audit_logging", true);
            provider.configure_tenant(config).await;
        }

        // Perform operations in each tenant
        let mut tenant_users = std::collections::HashMap::new();
        for (tenant_id, _) in &configs {
            let context = create_test_context(tenant_id);

            // Create users
            let user1 = provider
                .create_resource(
                    "User",
                    create_test_user(&format!("{}_user1", tenant_id)),
                    &context,
                )
                .await
                .unwrap();

            let user2 = provider
                .create_resource(
                    "User",
                    create_test_user(&format!("{}_user2", tenant_id)),
                    &context,
                )
                .await
                .unwrap();

            tenant_users.insert(tenant_id.to_string(), vec![user1, user2]);
        }

        // Verify complete isolation between tenants
        for (tenant_id, _) in &configs {
            let context = create_test_context(tenant_id);

            // Each tenant should only see its own resources
            let users = provider
                .list_resources("User", None, &context)
                .await
                .unwrap();

            assert_eq!(users.len(), 2);

            // Verify usernames match the tenant
            for user in &users {
                let username = user.resource().get_username().unwrap();
                assert!(username.starts_with(tenant_id));
            }

            // Verify statistics are isolated
            let stats = provider
                .get_tenant_statistics(tenant_id, &context)
                .await
                .unwrap();

            assert_eq!(stats.tenant_id, *tenant_id);
            assert_eq!(stats.total_resources, 2);

            // Verify audit logs are isolated
            let audit_entries = provider
                .get_audit_log(tenant_id, None, None, &context)
                .await
                .unwrap();

            for entry in &audit_entries {
                assert_eq!(entry.tenant_id, *tenant_id);
            }
        }
    }

    #[tokio::test]
    async fn test_comprehensive_error_recovery() {
        let provider = TestAdvancedProvider::new();
        let tenant_id = "error_recovery_tenant";
        let context = create_test_context(tenant_id);

        // Test various error conditions and recovery
        let mixed_bulk_request = BulkOperationRequest {
            tenant_id: tenant_id.to_string(),
            operations: vec![
                // Valid operation
                BulkOperation {
                    operation_type: BulkOperationType::Create,
                    resource_type: "User".to_string(),
                    resource_id: None,
                    data: Some(create_test_user("valid_user1")),
                },
                // Invalid operation - missing data
                BulkOperation {
                    operation_type: BulkOperationType::Create,
                    resource_type: "User".to_string(),
                    resource_id: None,
                    data: None,
                },
                // Valid operation
                BulkOperation {
                    operation_type: BulkOperationType::Create,
                    resource_type: "User".to_string(),
                    resource_id: None,
                    data: Some(create_test_user("valid_user2")),
                },
                // Invalid operation - missing resource ID for update
                BulkOperation {
                    operation_type: BulkOperationType::Update,
                    resource_type: "User".to_string(),
                    resource_id: None,
                    data: Some(create_test_user("update_user")),
                },
                // Valid operation
                BulkOperation {
                    operation_type: BulkOperationType::Create,
                    resource_type: "User".to_string(),
                    resource_id: None,
                    data: Some(create_test_user("valid_user3")),
                },
            ],
            fail_on_errors: false,
            continue_on_error: true, // Continue despite errors
        };

        let result = provider
            .execute_bulk_operation(mixed_bulk_request, &context)
            .await
            .unwrap();

        // Should have 3 successful operations and 2 failed ones
        assert_eq!(result.total_operations, 5);
        assert_eq!(result.successful_operations, 3);
        assert_eq!(result.failed_operations, 2);

        // Verify the system continued processing after errors
        assert_eq!(result.results.len(), 5);

        // Check specific operation results
        assert!(result.results[0].success); // valid_user1
        assert!(!result.results[1].success); // missing data
        assert!(result.results[2].success); // valid_user2
        assert!(!result.results[3].success); // missing resource ID
        assert!(result.results[4].success); // valid_user3

        // Verify the successful operations created resources
        let users = provider
            .list_resources("User", None, &context)
            .await
            .unwrap();

        assert_eq!(users.len(), 3);

        let usernames: Vec<&str> = users
            .iter()
            .map(|u| u.resource().get_username().unwrap())
            .collect();

        assert!(usernames.contains(&"valid_user1"));
        assert!(usernames.contains(&"valid_user2"));
        assert!(usernames.contains(&"valid_user3"));
    }

    #[tokio::test]
    async fn test_tenant_lifecycle_management() {
        let provider = TestAdvancedProvider::new();

        // Simulate complete tenant lifecycle
        let tenant_id = "lifecycle_tenant";
        let context = create_test_context(tenant_id);

        // 1. Configure new tenant
        let config = AdvancedTenantConfig::new(tenant_id)
            .with_compliance_level(ComplianceLevel::Enhanced)
            .with_feature_flag("bulk_operations", true)
            .with_feature_flag("audit_logging", true);

        provider.configure_tenant(config).await;

        // 2. Populate with initial data
        let bulk_request = BulkOperationRequest {
            tenant_id: tenant_id.to_string(),
            operations: (0..10)
                .map(|i| BulkOperation {
                    operation_type: BulkOperationType::Create,
                    resource_type: "User".to_string(),
                    resource_id: None,
                    data: Some(create_test_user(&format!("lifecycle_user_{}", i))),
                })
                .collect(),
            fail_on_errors: false,
            continue_on_error: true,
        };

        let bulk_result = provider
            .execute_bulk_operation(bulk_request, &context)
            .await
            .unwrap();

        assert_eq!(bulk_result.successful_operations, 10);

        // 3. Verify tenant is operational
        let stats = provider
            .get_tenant_statistics(tenant_id, &context)
            .await
            .unwrap();

        assert_eq!(stats.total_resources, 10);

        // 4. Verify audit trail
        let audit_entries = provider
            .get_audit_log(tenant_id, None, None, &context)
            .await
            .unwrap();

        assert!(audit_entries.len() >= 10); // At least one entry per user creation

        // 5. Test tenant configuration changes
        let updated_config = AdvancedTenantConfig::new(tenant_id)
            .with_compliance_level(ComplianceLevel::Strict)
            .with_feature_flag("bulk_operations", false)
            .with_feature_flag("audit_logging", true)
            .with_feature_flag("enhanced_security", true);

        provider.configure_tenant(updated_config).await;

        // Verify configuration was updated
        let configs = provider.tenant_configs.read().await;
        let tenant_config = configs.get(tenant_id).unwrap();
        assert_eq!(tenant_config.compliance_level, ComplianceLevel::Strict);
        assert_eq!(
            *tenant_config.feature_flags.get("bulk_operations").unwrap(),
            false
        );
        assert_eq!(
            *tenant_config
                .feature_flags
                .get("enhanced_security")
                .unwrap(),
            true
        );

        // 6. Test data operations still work with new config
        let new_user = provider
            .create_resource("User", create_test_user("post_update_user"), &context)
            .await
            .unwrap();

        assert!(new_user.resource().get_id().is_some());

        // Final verification
        let final_stats = provider
            .get_tenant_statistics(tenant_id, &context)
            .await
            .unwrap();

        assert_eq!(final_stats.total_resources, 11); // 10 + 1 new user
    }

    #[tokio::test]
    async fn test_advanced_validation_rules() {
        let provider = TestAdvancedProvider::new();
        let tenant_id = "validation_tenant";
        let context = create_test_context(tenant_id);

        // Create custom validation rules
        let validation_rules = vec![
            CustomValidationRule {
                name: "username_length".to_string(),
                resource_type: "User".to_string(),
                attribute: "userName".to_string(),
                rule_type: ValidationRuleType::Length {
                    min: Some(5),
                    max: Some(50),
                },
                parameters: std::collections::HashMap::new(),
            },
            CustomValidationRule {
                name: "display_name_required".to_string(),
                resource_type: "User".to_string(),
                attribute: "displayName".to_string(),
                rule_type: ValidationRuleType::Required,
                parameters: std::collections::HashMap::new(),
            },
        ];

        // Test validation with valid data
        let valid_user_data = json!({
            "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
            "userName": "valid_user_name",
            "displayName": "Valid User",
            "active": true
        });

        let validation_result = provider
            .validate_custom_rules(
                tenant_id,
                "User",
                &valid_user_data,
                &validation_rules,
                &context,
            )
            .await
            .unwrap();

        assert!(validation_result.is_empty()); // No validation errors

        // Test validation with invalid data
        let invalid_user_data = json!({
            "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
            "userName": "usr", // Too short
            "active": true
            // Missing displayName
        });

        let validation_errors = provider
            .validate_custom_rules(
                tenant_id,
                "User",
                &invalid_user_data,
                &validation_rules,
                &context,
            )
            .await
            .unwrap();

        assert!(!validation_errors.is_empty());
        assert!(validation_errors.iter().any(|e| e.contains("displayName")));
        assert!(validation_errors.iter().any(|e| e.contains("too short")));
    }

    #[tokio::test]
    async fn test_end_to_end_multi_tenant_workflow() {
        println!("\n🔄 End-to-End Multi-Tenant Workflow Test");
        println!("==========================================");

        let provider = TestAdvancedProvider::new();

        // Simulate a complete enterprise workflow
        let enterprise_tenant = "enterprise_workflow";
        let dev_tenant = "dev_workflow";

        // 1. Setup enterprise tenant with strict compliance
        let enterprise_config = AdvancedTenantConfig::new(enterprise_tenant)
            .with_compliance_level(ComplianceLevel::Strict)
            .with_feature_flag("audit_logging", true)
            .with_feature_flag("bulk_operations", true)
            .with_feature_flag("data_encryption", true);

        // 2. Setup development tenant with basic compliance
        let dev_config = AdvancedTenantConfig::new(dev_tenant)
            .with_compliance_level(ComplianceLevel::Basic)
            .with_feature_flag("audit_logging", false)
            .with_feature_flag("bulk_operations", true);

        provider.configure_tenant(enterprise_config).await;
        provider.configure_tenant(dev_config).await;

        let enterprise_context = create_test_context(enterprise_tenant);
        let dev_context = create_test_context(dev_tenant);

        // 3. Bulk import users to enterprise tenant
        let enterprise_bulk = BulkOperationRequest {
            tenant_id: enterprise_tenant.to_string(),
            operations: (0..25)
                .map(|i| BulkOperation {
                    operation_type: BulkOperationType::Create,
                    resource_type: "User".to_string(),
                    resource_id: None,
                    data: Some(json!({
                        "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
                        "userName": format!("enterprise.user.{}@company.com", i),
                        "displayName": format!("Enterprise User {}", i),
                        "active": true,
                        "department": "Engineering",
                        "title": if i < 5 { "Manager" } else { "Developer" }
                    })),
                })
                .collect(),
            fail_on_errors: false,
            continue_on_error: true,
        };

        let enterprise_result = provider
            .execute_bulk_operation(enterprise_bulk, &enterprise_context)
            .await
            .unwrap();

        // 4. Create test users in dev tenant
        let dev_bulk = BulkOperationRequest {
            tenant_id: dev_tenant.to_string(),
            operations: (0..10)
                .map(|i| BulkOperation {
                    operation_type: BulkOperationType::Create,
                    resource_type: "User".to_string(),
                    resource_id: None,
                    data: Some(json!({
                        "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
                        "userName": format!("dev.user.{}", i),
                        "displayName": format!("Dev User {}", i),
                        "active": true
                    })),
                })
                .collect(),
            fail_on_errors: false,
            continue_on_error: true,
        };

        let dev_result = provider
            .execute_bulk_operation(dev_bulk, &dev_context)
            .await
            .unwrap();

        // 5. Verify operations succeeded
        assert_eq!(enterprise_result.successful_operations, 25);
        assert_eq!(dev_result.successful_operations, 10);

        // 6. Collect and verify statistics
        let enterprise_stats = provider
            .get_tenant_statistics(enterprise_tenant, &enterprise_context)
            .await
            .unwrap();

        let dev_stats = provider
            .get_tenant_statistics(dev_tenant, &dev_context)
            .await
            .unwrap();

        assert_eq!(enterprise_stats.total_resources, 25);
        assert_eq!(dev_stats.total_resources, 10);

        // 7. Verify audit logging (only for enterprise tenant)
        let enterprise_audit = provider
            .get_audit_log(enterprise_tenant, None, None, &enterprise_context)
            .await
            .unwrap();

        assert!(enterprise_audit.len() >= 25);

        // 8. Test cross-tenant isolation
        let enterprise_users = provider
            .list_resources("User", None, &enterprise_context)
            .await
            .unwrap();

        let dev_users = provider
            .list_resources("User", None, &dev_context)
            .await
            .unwrap();

        // Verify no cross-contamination
        for user in &enterprise_users {
            assert!(
                user.resource()
                    .get_username()
                    .unwrap()
                    .contains("enterprise")
            );
        }

        for user in &dev_users {
            assert!(user.resource().get_username().unwrap().contains("dev"));
        }

        println!(
            "✅ Enterprise tenant: {} users created",
            enterprise_stats.total_resources
        );
        println!(
            "✅ Development tenant: {} users created",
            dev_stats.total_resources
        );
        println!(
            "✅ Audit entries for enterprise: {}",
            enterprise_audit.len()
        );
        println!("✅ Complete tenant isolation verified");
        println!("✅ End-to-end workflow completed successfully");
    }
}
