//! Configuration system integration tests.
//!
//! This module contains comprehensive tests for the tenant configuration
//! management system, covering all aspects of configuration storage,
//! retrieval, validation, and management.

pub mod performance_tests;
pub mod provider_tests;
pub mod validation_tests;

// Removed unused import
use scim_server::multi_tenant::{
    AuditLevel, BrandingConfiguration, BulkConfigurationOperation, ComplianceConfiguration,
    ComplianceFramework, ConfigurationError, ConfigurationQuery, InMemoryConfigurationProvider,
    OperationalConfiguration, RateLimitPeriod, SchemaConfiguration, TenantConfiguration,
    TenantConfigurationProvider, ValidationContext,
};
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

/// Create a basic test tenant configuration.
pub fn create_basic_test_config(tenant_id: &str) -> TenantConfiguration {
    TenantConfiguration::builder(tenant_id.to_string())
        .with_display_name(format!("Test Tenant {}", tenant_id))
        .build()
        .expect("Should build basic test configuration")
}

/// Create a comprehensive test configuration with all features.
pub fn create_comprehensive_test_config(tenant_id: &str) -> TenantConfiguration {
    let schema_config = SchemaConfiguration::builder()
        .add_custom_attribute(
            "customField".to_string(),
            json!({
                "type": "string",
                "multiValued": false,
                "required": false,
                "description": "Custom field for testing"
            }),
        )
        .disable_standard_attribute("nickName".to_string())
        .require_attribute("department".to_string())
        .build();

    let operational_config = OperationalConfiguration::builder()
        .enable_feature("advanced_search".to_string())
        .enable_feature("bulk_operations".to_string())
        .disable_feature("external_auth".to_string())
        .build();

    let compliance_config = ComplianceConfiguration {
        audit_level: AuditLevel::Full,
        encryption_requirements: Default::default(),
        compliance_frameworks: vec![ComplianceFramework::GDPR, ComplianceFramework::SOC2],
        data_residency: Some("EU".to_string()),
        enable_pii_scrubbing: true,
        data_retention_policies: {
            let mut policies = HashMap::new();
            policies.insert(
                "audit_logs".to_string(),
                Duration::from_secs(365 * 24 * 3600),
            );
            policies.insert(
                "user_data".to_string(),
                Duration::from_secs(7 * 365 * 24 * 3600),
            );
            policies
        },
    };

    let branding_config = BrandingConfiguration {
        display_name: format!("Comprehensive Test Tenant {}", tenant_id),
        logo_url: Some("https://example.com/logo.png".to_string()),
        primary_color: Some("#007bff".to_string()),
        secondary_color: Some("#6c757d".to_string()),
        custom_css: Some(".custom-style { color: blue; }".to_string()),
        favicon_url: Some("https://example.com/favicon.ico".to_string()),
        footer_text: Some("© 2024 Test Company".to_string()),
        support_contact: Some("support@test.com".to_string()),
    };

    TenantConfiguration::builder(tenant_id.to_string())
        .with_display_name(format!("Comprehensive Test Tenant {}", tenant_id))
        .with_schema_configuration(schema_config)
        .with_operational_configuration(operational_config)
        .with_compliance_configuration(compliance_config)
        .with_branding_configuration(branding_config)
        .build()
        .expect("Should build comprehensive test configuration")
}

/// Create a provider with test configurations pre-loaded.
pub async fn create_provider_with_test_data() -> Arc<InMemoryConfigurationProvider> {
    let provider = Arc::new(InMemoryConfigurationProvider::new());

    // Create basic configurations
    for i in 1..=5 {
        let config = create_basic_test_config(&format!("basic-tenant-{}", i));
        provider
            .create_configuration(config)
            .await
            .expect("Should create basic test configuration");
    }

    // Create comprehensive configurations
    for i in 1..=3 {
        let config = create_comprehensive_test_config(&format!("comp-tenant-{}", i));
        provider
            .create_configuration(config)
            .await
            .expect("Should create comprehensive test configuration");
    }

    provider
}

/// Test configuration builder patterns.
#[tokio::test]
async fn test_configuration_builders() {
    // Test basic builder
    let basic_config = TenantConfiguration::builder("test-tenant".to_string())
        .build()
        .expect("Should build basic configuration");

    assert_eq!(basic_config.tenant_id, "test-tenant");
    assert_eq!(basic_config.display_name, "test-tenant");
    assert_eq!(basic_config.version, 1);

    // Test builder with all options
    let full_config = TenantConfiguration::builder("full-tenant".to_string())
        .with_display_name("Full Test Tenant".to_string())
        .with_schema_configuration(SchemaConfiguration::default())
        .with_operational_configuration(OperationalConfiguration::default())
        .with_compliance_configuration(ComplianceConfiguration::default())
        .with_branding_configuration(BrandingConfiguration::default())
        .build()
        .expect("Should build full configuration");

    assert_eq!(full_config.tenant_id, "full-tenant");
    assert_eq!(full_config.display_name, "Full Test Tenant");
}

/// Test configuration validation.
#[tokio::test]
async fn test_configuration_validation() {
    // Valid configuration should pass
    let valid_config = create_basic_test_config("valid-tenant");
    assert!(valid_config.validate().is_ok());

    // Test invalid resource limits
    let mut invalid_config = valid_config.clone();
    invalid_config.operational.resource_limits.max_users = Some(0);
    assert!(invalid_config.validate().is_err());

    // Test invalid rate limits
    let mut invalid_config = valid_config.clone();
    invalid_config.operational.rate_limits.requests_per_minute = Some(0);
    assert!(invalid_config.validate().is_err());

    // Test duplicate schema extension IDs
    let mut invalid_config = valid_config.clone();
    invalid_config.schema.schema_extensions = vec![
        scim_server::multi_tenant::SchemaExtension {
            id: "duplicate".to_string(),
            name: "First".to_string(),
            description: "First extension".to_string(),
            schema: json!({}),
            required: false,
        },
        scim_server::multi_tenant::SchemaExtension {
            id: "duplicate".to_string(),
            name: "Second".to_string(),
            description: "Second extension".to_string(),
            schema: json!({}),
            required: false,
        },
    ];
    assert!(invalid_config.validate().is_err());
}

/// Test configuration serialization and deserialization.
#[tokio::test]
async fn test_configuration_serialization() {
    let original = create_comprehensive_test_config("serialization-test");

    // Test JSON serialization
    let json_str = serde_json::to_string(&original).expect("Should serialize to JSON");
    let deserialized: TenantConfiguration =
        serde_json::from_str(&json_str).expect("Should deserialize from JSON");

    assert_eq!(original, deserialized);

    // Test pretty JSON serialization
    let pretty_json =
        serde_json::to_string_pretty(&original).expect("Should serialize to pretty JSON");
    assert!(pretty_json.contains("tenant_id"));
    assert!(pretty_json.contains("display_name"));

    let from_pretty: TenantConfiguration =
        serde_json::from_str(&pretty_json).expect("Should deserialize from pretty JSON");
    assert_eq!(original, from_pretty);
}

/// Test configuration utility methods.
#[tokio::test]
async fn test_configuration_utility_methods() {
    let mut config = create_comprehensive_test_config("utility-test");

    // Test touch method
    let original_version = config.version;
    let original_modified = config.last_modified;

    // Wait a bit to ensure timestamp difference
    tokio::time::sleep(tokio::time::Duration::from_millis(1)).await;
    config.touch();

    assert_eq!(config.version, original_version + 1);
    assert!(config.last_modified > original_modified);

    // Test allows_operation method
    assert!(config.allows_operation("create")); // Default should allow

    // Add a feature flag to test
    config
        .operational
        .feature_flags
        .insert("allow_test_operation".to_string(), false);
    assert!(!config.allows_operation("test_operation"));

    config
        .operational
        .feature_flags
        .insert("allow_test_operation".to_string(), true);
    assert!(config.allows_operation("test_operation"));
}

/// Test schema configuration methods.
#[tokio::test]
async fn test_schema_configuration_methods() {
    let schema_config = SchemaConfiguration::builder()
        .add_custom_attribute("customField".to_string(), json!({"type": "string"}))
        .disable_standard_attribute("nickName".to_string())
        .require_attribute("department".to_string())
        .add_validation_rule(
            "customField".to_string(),
            scim_server::multi_tenant::ValidationRule {
                rule_type: scim_server::multi_tenant::ValidationType::Regex,
                parameters: {
                    let mut params = HashMap::new();
                    params.insert("pattern".to_string(), json!("^[A-Z]+$"));
                    params
                },
                error_message: "Must be uppercase letters only".to_string(),
            },
        )
        .build();

    // Test attribute checking methods
    assert!(schema_config.is_attribute_disabled("nickName"));
    assert!(!schema_config.is_attribute_disabled("userName"));

    assert!(schema_config.is_attribute_required("department"));
    assert!(!schema_config.is_attribute_required("nickName"));

    // Test validation rule retrieval
    let rule = schema_config.get_validation_rule("customField");
    assert!(rule.is_some());
    assert!(matches!(
        rule.unwrap().rule_type,
        scim_server::multi_tenant::ValidationType::Regex
    ));

    assert!(schema_config.get_validation_rule("nonexistent").is_none());
}

/// Test operational configuration methods.
#[tokio::test]
async fn test_operational_configuration_methods() {
    let operational_config = OperationalConfiguration::builder()
        .enable_feature("advanced_search".to_string())
        .disable_feature("bulk_operations".to_string())
        .build();

    // Test feature flag checking
    assert!(operational_config.is_feature_enabled("advanced_search"));
    assert!(!operational_config.is_feature_enabled("bulk_operations"));
    assert!(!operational_config.is_feature_enabled("nonexistent_feature"));

    // Test rate limit retrieval
    assert!(
        operational_config
            .get_rate_limit(RateLimitPeriod::Minute)
            .is_some()
    );
    assert!(
        operational_config
            .get_rate_limit(RateLimitPeriod::Hour)
            .is_some()
    );
    assert!(
        operational_config
            .get_rate_limit(RateLimitPeriod::Day)
            .is_some()
    );
}

/// Test default configurations.
#[tokio::test]
async fn test_default_configurations() {
    let schema_default = SchemaConfiguration::default();
    assert!(schema_default.custom_attributes.is_empty());
    assert!(schema_default.disabled_attributes.is_empty());
    assert!(schema_default.additional_required.is_empty());
    assert!(schema_default.schema_extensions.is_empty());
    assert!(schema_default.validation_rules.is_empty());

    let operational_default = OperationalConfiguration::default();
    assert!(
        operational_default
            .rate_limits
            .requests_per_minute
            .is_some()
    );
    assert!(operational_default.resource_limits.max_users.is_some());
    assert!(operational_default.performance_settings.enable_caching);

    let compliance_default = ComplianceConfiguration::default();
    assert_eq!(compliance_default.audit_level, AuditLevel::Basic);
    assert!(compliance_default.encryption_requirements.encrypt_at_rest);
    assert!(compliance_default.compliance_frameworks.is_empty());

    let branding_default = BrandingConfiguration::default();
    assert_eq!(branding_default.display_name, "Default Tenant");
    assert!(branding_default.primary_color.is_some());
}

/// Test configuration error types.
#[tokio::test]
async fn test_configuration_errors() {
    // Test validation error
    let validation_error = ConfigurationError::ValidationError {
        message: "Test validation error".to_string(),
    };
    assert!(validation_error.to_string().contains("validation"));

    // Test not found error
    let not_found_error = ConfigurationError::NotFound {
        tenant_id: "missing-tenant".to_string(),
    };
    assert!(not_found_error.to_string().contains("not found"));

    // Test conflict error
    let conflict_error = ConfigurationError::Conflict {
        message: "Test conflict".to_string(),
    };
    assert!(conflict_error.to_string().contains("conflict"));

    // Test version mismatch error
    let version_error = ConfigurationError::VersionMismatch {
        expected: 1,
        actual: 2,
    };
    assert!(version_error.to_string().contains("version"));
}

/// Test bulk configuration operations.
#[tokio::test]
async fn test_bulk_operations_structure() {
    let config1 = create_basic_test_config("bulk-1");
    let config2 = create_basic_test_config("bulk-2");

    let operations = vec![
        BulkConfigurationOperation::Create(config1.clone()),
        BulkConfigurationOperation::Update(config2.clone()),
        BulkConfigurationOperation::Delete {
            tenant_id: "bulk-3".to_string(),
            expected_version: Some(1),
        },
        BulkConfigurationOperation::Validate(config1.clone()),
    ];

    assert_eq!(operations.len(), 4);

    // Test operation pattern matching
    for operation in &operations {
        match operation {
            BulkConfigurationOperation::Create(config) => {
                assert_eq!(config.tenant_id, "bulk-1");
            }
            BulkConfigurationOperation::Update(config) => {
                assert_eq!(config.tenant_id, "bulk-2");
            }
            BulkConfigurationOperation::Delete {
                tenant_id,
                expected_version,
            } => {
                assert_eq!(tenant_id, "bulk-3");
                assert_eq!(*expected_version, Some(1));
            }
            BulkConfigurationOperation::Validate(config) => {
                assert_eq!(config.tenant_id, "bulk-1");
            }
        }
    }
}

/// Test configuration query structure.
#[tokio::test]
async fn test_configuration_query() {
    use chrono::Utc;
    use scim_server::multi_tenant::config_provider::SortOrder;

    let query = ConfigurationQuery {
        tenant_ids: Some(vec!["tenant-1".to_string(), "tenant-2".to_string()]),
        display_name_filter: Some("Test".to_string()),
        modified_after: Some(Utc::now() - chrono::Duration::days(7)),
        modified_before: Some(Utc::now()),
        offset: Some(10),
        limit: Some(50),
        sort_order: SortOrder::DisplayNameAsc,
    };

    assert_eq!(query.tenant_ids.as_ref().unwrap().len(), 2);
    assert!(query.display_name_filter.is_some());
    assert!(query.modified_after.is_some());
    assert!(query.modified_before.is_some());
    assert_eq!(query.offset, Some(10));
    assert_eq!(query.limit, Some(50));
    assert!(matches!(query.sort_order, SortOrder::DisplayNameAsc));

    // Test default query
    let default_query = ConfigurationQuery::default();
    assert!(default_query.tenant_ids.is_none());
    assert!(default_query.display_name_filter.is_none());
    assert!(default_query.offset.is_none());
    assert!(default_query.limit.is_none());
    assert!(matches!(default_query.sort_order, SortOrder::TenantIdAsc));
}

/// Test validation context structure.
#[tokio::test]
async fn test_validation_context() {
    let previous_config = create_basic_test_config("previous");

    let create_context = ValidationContext {
        is_create: true,
        previous_configuration: None,
        validation_params: HashMap::new(),
    };

    let update_context = ValidationContext {
        is_create: false,
        previous_configuration: Some(previous_config.clone()),
        validation_params: {
            let mut params = HashMap::new();
            params.insert("check_schema_changes".to_string(), "true".to_string());
            params
        },
    };

    assert!(create_context.is_create);
    assert!(create_context.previous_configuration.is_none());
    assert!(create_context.validation_params.is_empty());

    assert!(!update_context.is_create);
    assert!(update_context.previous_configuration.is_some());
    assert_eq!(
        update_context
            .previous_configuration
            .as_ref()
            .unwrap()
            .tenant_id,
        "previous"
    );
    assert_eq!(
        update_context.validation_params.get("check_schema_changes"),
        Some(&"true".to_string())
    );
}

/// Integration test combining all configuration features.
#[tokio::test]
async fn test_end_to_end_configuration_workflow() {
    let provider = Arc::new(InMemoryConfigurationProvider::new());

    // Step 1: Create comprehensive configuration
    let original_config = create_comprehensive_test_config("e2e-tenant");
    let created_config = provider
        .create_configuration(original_config.clone())
        .await
        .expect("Should create configuration");

    assert_eq!(created_config.tenant_id, "e2e-tenant");
    assert_eq!(created_config.version, 1);

    // Step 2: Retrieve and verify
    let retrieved_config = provider
        .get_configuration("e2e-tenant")
        .await
        .expect("Should retrieve configuration")
        .expect("Configuration should exist");

    assert_eq!(retrieved_config.tenant_id, created_config.tenant_id);
    assert_eq!(retrieved_config.version, created_config.version);

    // Step 3: Update configuration
    let mut updated_config = retrieved_config.clone();
    updated_config.display_name = "Updated E2E Tenant".to_string();
    updated_config
        .operational
        .feature_flags
        .insert("new_feature".to_string(), true);

    let final_config = provider
        .update_configuration(updated_config)
        .await
        .expect("Should update configuration");

    assert_eq!(final_config.display_name, "Updated E2E Tenant");
    assert_eq!(final_config.version, 2);
    assert!(final_config.operational.is_feature_enabled("new_feature"));

    // Step 4: Test validation
    let validation_context = ValidationContext {
        is_create: false,
        previous_configuration: Some(retrieved_config),
        validation_params: HashMap::new(),
    };

    provider
        .validate_configuration(&final_config, &validation_context)
        .await
        .expect("Should validate updated configuration");

    // Step 5: List configurations
    let query = ConfigurationQuery::default();
    let list_result = provider
        .list_configurations(&query)
        .await
        .expect("Should list configurations");

    assert_eq!(list_result.configurations.len(), 1);
    assert_eq!(list_result.total_count, 1);
    assert!(!list_result.has_more);

    // Step 6: Get statistics
    let stats = provider
        .get_configuration_stats()
        .await
        .expect("Should get statistics");

    assert_eq!(stats.total_configurations, 1);
    assert!(stats.average_size > 0);
    assert!(stats.newest_configuration.is_some());

    // Step 7: Delete configuration
    provider
        .delete_configuration("e2e-tenant", Some(final_config.version))
        .await
        .expect("Should delete configuration");

    let after_delete = provider
        .get_configuration("e2e-tenant")
        .await
        .expect("Should not error after delete");
    assert!(after_delete.is_none());

    // Step 8: Verify empty state
    let final_count = provider
        .count_configurations()
        .await
        .expect("Should count configurations");
    assert_eq!(final_count, 0);
}
