//! Validation-focused tests for tenant configuration management.
//!
//! This module tests the validation logic for tenant configurations,
//! including schema validation, business rule validation, and
//! constraint checking.

// Removed unused import
use scim_server::multi_tenant::{
    AuditLevel, ComplianceFramework, ConfigurationError, InMemoryConfigurationProvider,
    OperationalConfiguration, RateLimitConfiguration, ResourceLimits, SchemaConfiguration,
    SchemaExtension, TenantConfiguration, TenantConfigurationProvider, ValidationContext,
    ValidationRule, ValidationType,
};
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

/// Test basic configuration validation.
#[tokio::test]
async fn test_basic_configuration_validation() {
    let provider = Arc::new(InMemoryConfigurationProvider::new());

    // Valid configuration should pass
    let valid_config = TenantConfiguration::builder("valid-tenant".to_string())
        .with_display_name("Valid Tenant".to_string())
        .build()
        .expect("Should build valid configuration");

    let context = ValidationContext {
        is_create: true,
        previous_configuration: None,
        validation_params: HashMap::new(),
    };

    provider
        .validate_configuration(&valid_config, &context)
        .await
        .expect("Should validate valid configuration");

    // Test validation during creation
    provider
        .create_configuration(valid_config)
        .await
        .expect("Should create valid configuration");
}

/// Test tenant ID validation.
#[tokio::test]
async fn test_tenant_id_validation() {
    let provider = Arc::new(InMemoryConfigurationProvider::new());
    let context = ValidationContext {
        is_create: true,
        previous_configuration: None,
        validation_params: HashMap::new(),
    };

    // Test empty tenant ID
    let result = provider.get_configuration("").await;
    assert!(result.is_err());
    assert!(matches!(
        result.unwrap_err(),
        ConfigurationError::ValidationError { .. }
    ));

    // Test tenant ID with invalid characters
    let invalid_chars = vec![
        "tenant@test",
        "tenant with spaces",
        "tenant/path",
        "tenant$money",
    ];
    for invalid_id in invalid_chars {
        let result = provider.get_configuration(invalid_id).await;
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("alphanumeric characters")
        );
    }

    // Test overly long tenant ID
    let long_id = "a".repeat(300);
    let result = provider.get_configuration(&long_id).await;
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("255 characters"));

    // Test valid tenant IDs
    let valid_ids = vec![
        "tenant-1",
        "tenant_2",
        "TenantABC",
        "tenant123",
        "a",
        "A-B_C-123",
    ];
    for valid_id in valid_ids {
        let config = TenantConfiguration::builder(valid_id.to_string())
            .build()
            .expect("Should build configuration with valid ID");

        provider
            .validate_configuration(&config, &context)
            .await
            .expect("Should validate configuration with valid tenant ID");
    }
}

/// Test resource limits validation.
#[tokio::test]
async fn test_resource_limits_validation() {
    // Test invalid resource limits
    let invalid_limits = vec![
        (Some(0), None, "max_users cannot be 0"),
        (None, Some(0), "max_groups cannot be 0"),
        (Some(0), Some(0), "both limits cannot be 0"),
    ];

    for (max_users, max_groups, _description) in invalid_limits {
        let resource_limits = ResourceLimits {
            max_users,
            max_groups,
            max_custom_resources: Some(1000),
            max_resource_size: Some(1024),
            max_total_storage: Some(1024 * 1024),
        };

        let operational_config = OperationalConfiguration {
            resource_limits,
            ..Default::default()
        };

        let build_result = TenantConfiguration::builder("test-tenant".to_string())
            .with_operational_configuration(operational_config)
            .build();

        // Build should fail for invalid limits
        assert!(build_result.is_err());
    }

    // Test valid resource limits
    let valid_limits = ResourceLimits {
        max_users: Some(10000),
        max_groups: Some(1000),
        max_custom_resources: Some(5000),
        max_resource_size: Some(1024 * 1024),       // 1MB
        max_total_storage: Some(100 * 1024 * 1024), // 100MB
    };

    let operational_config = OperationalConfiguration {
        resource_limits: valid_limits,
        ..Default::default()
    };

    let config = TenantConfiguration::builder("test-tenant".to_string())
        .with_operational_configuration(operational_config)
        .build()
        .expect("Should build configuration with valid limits");

    assert!(config.validate().is_ok());
}

/// Test rate limiting validation.
#[tokio::test]
async fn test_rate_limiting_validation() {
    // Test invalid rate limits
    let invalid_rate_configs = vec![
        RateLimitConfiguration {
            requests_per_minute: Some(0),
            requests_per_hour: Some(1000),
            requests_per_day: Some(10000),
            burst_allowance: Some(100),
            window_duration: Duration::from_secs(60),
        },
        RateLimitConfiguration {
            requests_per_minute: Some(100),
            requests_per_hour: Some(0),
            requests_per_day: Some(10000),
            burst_allowance: Some(100),
            window_duration: Duration::from_secs(60),
        },
        RateLimitConfiguration {
            requests_per_minute: Some(100),
            requests_per_hour: Some(1000),
            requests_per_day: Some(0),
            burst_allowance: Some(100),
            window_duration: Duration::from_secs(60),
        },
    ];

    for rate_config in invalid_rate_configs {
        let operational_config = OperationalConfiguration {
            rate_limits: rate_config,
            ..Default::default()
        };

        let build_result = TenantConfiguration::builder("test-tenant".to_string())
            .with_operational_configuration(operational_config)
            .build();

        // Build should fail for invalid rate limits
        assert!(build_result.is_err());
    }

    // Test valid rate limits
    let valid_rate_config = RateLimitConfiguration {
        requests_per_minute: Some(1000),
        requests_per_hour: Some(10000),
        requests_per_day: Some(100000),
        burst_allowance: Some(500),
        window_duration: Duration::from_secs(60),
    };

    let operational_config = OperationalConfiguration {
        rate_limits: valid_rate_config,
        ..Default::default()
    };

    let config = TenantConfiguration::builder("test-tenant".to_string())
        .with_operational_configuration(operational_config)
        .build()
        .expect("Should build configuration with valid rate limits");

    assert!(config.validate().is_ok());
}

/// Test schema extension validation.
#[tokio::test]
async fn test_schema_extension_validation() {
    // Test duplicate schema extension IDs
    let duplicate_extensions = vec![
        SchemaExtension {
            id: "duplicate".to_string(),
            name: "First Extension".to_string(),
            description: "First duplicate extension".to_string(),
            schema: json!({"type": "object"}),
            required: false,
        },
        SchemaExtension {
            id: "duplicate".to_string(),
            name: "Second Extension".to_string(),
            description: "Second duplicate extension".to_string(),
            schema: json!({"type": "string"}),
            required: true,
        },
    ];

    let schema_config = SchemaConfiguration {
        custom_attributes: HashMap::new(),
        disabled_attributes: Vec::new(),
        additional_required: Vec::new(),
        schema_extensions: duplicate_extensions,
        validation_rules: HashMap::new(),
    };

    let build_result = TenantConfiguration::builder("test-tenant".to_string())
        .with_schema_configuration(schema_config)
        .build();

    // Build should fail for duplicate schema extension IDs
    assert!(build_result.is_err());
    assert!(
        build_result
            .unwrap_err()
            .to_string()
            .contains("Duplicate schema extension ID")
    );

    // Test valid schema extensions
    let valid_extensions = vec![
        SchemaExtension {
            id: "extension1".to_string(),
            name: "First Extension".to_string(),
            description: "First extension".to_string(),
            schema: json!({"type": "object", "properties": {"field1": {"type": "string"}}}),
            required: false,
        },
        SchemaExtension {
            id: "extension2".to_string(),
            name: "Second Extension".to_string(),
            description: "Second extension".to_string(),
            schema: json!({"type": "array", "items": {"type": "string"}}),
            required: true,
        },
    ];

    let valid_schema_config = SchemaConfiguration {
        custom_attributes: HashMap::new(),
        disabled_attributes: Vec::new(),
        additional_required: Vec::new(),
        schema_extensions: valid_extensions,
        validation_rules: HashMap::new(),
    };

    let valid_config = TenantConfiguration::builder("test-tenant".to_string())
        .with_schema_configuration(valid_schema_config)
        .build()
        .expect("Should build configuration");

    assert!(valid_config.validate().is_ok());
}

/// Test configuration size validation.
#[tokio::test]
async fn test_configuration_size_validation() {
    let provider = Arc::new(InMemoryConfigurationProvider::new());
    let context = ValidationContext {
        is_create: true,
        previous_configuration: None,
        validation_params: HashMap::new(),
    };

    // Create a configuration with a very large custom attribute
    let mut large_custom_attributes = HashMap::new();
    large_custom_attributes.insert(
        "huge_field".to_string(),
        json!("x".repeat(2 * 1024 * 1024)), // 2MB string
    );

    let schema_config = SchemaConfiguration {
        custom_attributes: large_custom_attributes,
        disabled_attributes: Vec::new(),
        additional_required: Vec::new(),
        schema_extensions: Vec::new(),
        validation_rules: HashMap::new(),
    };

    let oversized_config = TenantConfiguration::builder("oversized-tenant".to_string())
        .with_schema_configuration(schema_config)
        .build()
        .expect("Should build oversized configuration");

    // Should fail validation due to size
    let validation_result = provider
        .validate_configuration(&oversized_config, &context)
        .await;
    assert!(validation_result.is_err());
    assert!(
        validation_result
            .unwrap_err()
            .to_string()
            .contains("exceeds maximum allowed size")
    );

    // Should also fail during creation
    let creation_result = provider.create_configuration(oversized_config).await;
    assert!(creation_result.is_err());
}

/// Test version validation.
#[tokio::test]
async fn test_version_validation() {
    let provider = Arc::new(InMemoryConfigurationProvider::new());

    // Create a configuration with valid version
    let valid_config = TenantConfiguration::builder("version-test".to_string())
        .build()
        .expect("Should build configuration");

    assert_eq!(valid_config.version, 1);

    let created = provider
        .create_configuration(valid_config.clone())
        .await
        .expect("Should create configuration");

    // Test updating with correct version
    let mut update_config = created.clone();
    update_config.display_name = "Updated Name".to_string();

    provider
        .update_configuration(update_config)
        .await
        .expect("Should update with correct version");

    // Test updating with incorrect version
    let mut stale_config = created.clone();
    stale_config.version = 999; // Wrong version
    stale_config.display_name = "Stale Update".to_string();

    let stale_result = provider.update_configuration(stale_config).await;
    assert!(stale_result.is_err());
    assert!(matches!(
        stale_result.unwrap_err(),
        ConfigurationError::VersionMismatch { .. }
    ));

    // Test updating with version 0 (invalid)
    let mut zero_version_config = created.clone();
    zero_version_config.version = 0;

    let zero_result = provider.update_configuration(zero_version_config).await;
    assert!(zero_result.is_err());
}

/// Test validation context usage.
#[tokio::test]
async fn test_validation_context() {
    let provider = Arc::new(InMemoryConfigurationProvider::new());

    // Create initial configuration
    let initial_config = TenantConfiguration::builder("context-test".to_string())
        .build()
        .expect("Should build initial configuration");

    let created = provider
        .create_configuration(initial_config)
        .await
        .expect("Should create initial configuration");

    // Test create context
    let create_context = ValidationContext {
        is_create: true,
        previous_configuration: None,
        validation_params: HashMap::new(),
    };

    let new_config = TenantConfiguration::builder("new-context-test".to_string())
        .build()
        .expect("Should build new configuration");

    provider
        .validate_configuration(&new_config, &create_context)
        .await
        .expect("Should validate with create context");

    // Test update context
    let update_context = ValidationContext {
        is_create: false,
        previous_configuration: Some(created.clone()),
        validation_params: {
            let mut params = HashMap::new();
            params.insert("check_schema_changes".to_string(), "true".to_string());
            params.insert("allow_breaking_changes".to_string(), "false".to_string());
            params
        },
    };

    let mut updated_config = created.clone();
    updated_config.display_name = "Updated via Context".to_string();

    provider
        .validate_configuration(&updated_config, &update_context)
        .await
        .expect("Should validate with update context");
}

/// Test validation rules for custom attributes.
#[tokio::test]
async fn test_custom_attribute_validation_rules() {
    // Test regex validation rule
    let regex_rule = ValidationRule {
        rule_type: ValidationType::Regex,
        parameters: {
            let mut params = HashMap::new();
            params.insert("pattern".to_string(), json!("^[A-Z][a-z]+$"));
            params
        },
        error_message: "Must start with uppercase letter followed by lowercase".to_string(),
    };

    // Test length validation rule
    let length_rule = ValidationRule {
        rule_type: ValidationType::Length,
        parameters: {
            let mut params = HashMap::new();
            params.insert("min".to_string(), json!(3));
            params.insert("max".to_string(), json!(50));
            params
        },
        error_message: "Must be between 3 and 50 characters".to_string(),
    };

    // Test range validation rule
    let range_rule = ValidationRule {
        rule_type: ValidationType::Range,
        parameters: {
            let mut params = HashMap::new();
            params.insert("min".to_string(), json!(0));
            params.insert("max".to_string(), json!(100));
            params
        },
        error_message: "Must be between 0 and 100".to_string(),
    };

    // Test enum validation rule
    let enum_rule = ValidationRule {
        rule_type: ValidationType::Enum,
        parameters: {
            let mut params = HashMap::new();
            params.insert(
                "values".to_string(),
                json!(["active", "inactive", "pending"]),
            );
            params
        },
        error_message: "Must be one of: active, inactive, pending".to_string(),
    };

    // Test custom validation rule
    let custom_rule = ValidationRule {
        rule_type: ValidationType::Custom,
        parameters: {
            let mut params = HashMap::new();
            params.insert("function_name".to_string(), json!("validateBusinessId"));
            params.insert("context".to_string(), json!("business_rules"));
            params
        },
        error_message: "Business ID validation failed".to_string(),
    };

    let validation_rules = HashMap::from([
        ("name_field".to_string(), regex_rule),
        ("description".to_string(), length_rule),
        ("priority".to_string(), range_rule),
        ("status".to_string(), enum_rule),
        ("business_id".to_string(), custom_rule),
    ]);

    let schema_config = SchemaConfiguration {
        custom_attributes: HashMap::new(),
        disabled_attributes: Vec::new(),
        additional_required: Vec::new(),
        schema_extensions: Vec::new(),
        validation_rules,
    };

    let config = TenantConfiguration::builder("validation-rules-test".to_string())
        .with_schema_configuration(schema_config)
        .build()
        .expect("Should build configuration with validation rules");

    assert!(config.validate().is_ok());

    // Test validation rule retrieval
    assert!(config.schema.get_validation_rule("name_field").is_some());
    assert!(config.schema.get_validation_rule("description").is_some());
    assert!(config.schema.get_validation_rule("priority").is_some());
    assert!(config.schema.get_validation_rule("status").is_some());
    assert!(config.schema.get_validation_rule("business_id").is_some());
    assert!(config.schema.get_validation_rule("nonexistent").is_none());

    // Test validation rule types
    let name_rule = config.schema.get_validation_rule("name_field").unwrap();
    assert!(matches!(name_rule.rule_type, ValidationType::Regex));
    assert_eq!(
        name_rule.error_message,
        "Must start with uppercase letter followed by lowercase"
    );

    let status_rule = config.schema.get_validation_rule("status").unwrap();
    assert!(matches!(status_rule.rule_type, ValidationType::Enum));
    assert_eq!(
        status_rule.parameters.get("values"),
        Some(&json!(["active", "inactive", "pending"]))
    );
}

/// Test compliance configuration validation.
#[tokio::test]
async fn test_compliance_configuration_validation() {
    // Test valid compliance configurations
    let valid_compliance_configs = vec![
        // Basic configuration
        scim_server::multi_tenant::ComplianceConfiguration {
            audit_level: AuditLevel::Basic,
            encryption_requirements: Default::default(),
            compliance_frameworks: vec![],
            data_residency: None,
            enable_pii_scrubbing: false,
            data_retention_policies: HashMap::new(),
        },
        // Full configuration
        scim_server::multi_tenant::ComplianceConfiguration {
            audit_level: AuditLevel::Full,
            encryption_requirements: Default::default(),
            compliance_frameworks: vec![
                ComplianceFramework::GDPR,
                ComplianceFramework::HIPAA,
                ComplianceFramework::SOC2,
            ],
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
        },
        // Custom compliance framework
        scim_server::multi_tenant::ComplianceConfiguration {
            audit_level: AuditLevel::Detailed,
            encryption_requirements: Default::default(),
            compliance_frameworks: vec![ComplianceFramework::Custom(
                "Internal-Security-Policy-v2".to_string(),
            )],
            data_residency: Some("US".to_string()),
            enable_pii_scrubbing: true,
            data_retention_policies: HashMap::new(),
        },
    ];

    for compliance_config in valid_compliance_configs {
        let config = TenantConfiguration::builder("compliance-test".to_string())
            .with_compliance_configuration(compliance_config)
            .build()
            .expect("Should build configuration with compliance settings");

        assert!(config.validate().is_ok());
    }
}

/// Test operational configuration feature flags validation.
#[tokio::test]
async fn test_operational_feature_flags_validation() {
    let operational_config = OperationalConfiguration::builder()
        .enable_feature("advanced_search".to_string())
        .enable_feature("bulk_operations".to_string())
        .enable_feature("custom_schemas".to_string())
        .disable_feature("external_auth".to_string())
        .disable_feature("legacy_api".to_string())
        .build();

    let config = TenantConfiguration::builder("feature-flags-test".to_string())
        .with_operational_configuration(operational_config)
        .build()
        .expect("Should build configuration with feature flags");

    assert!(config.validate().is_ok());

    // Test feature flag queries
    assert!(config.operational.is_feature_enabled("advanced_search"));
    assert!(config.operational.is_feature_enabled("bulk_operations"));
    assert!(config.operational.is_feature_enabled("custom_schemas"));
    assert!(!config.operational.is_feature_enabled("external_auth"));
    assert!(!config.operational.is_feature_enabled("legacy_api"));
    assert!(!config.operational.is_feature_enabled("nonexistent_feature"));

    // Test allows_operation method
    assert!(config.allows_operation("create")); // Default should allow
    assert!(!config.allows_operation("external_auth")); // Explicitly disabled
    assert!(!config.allows_operation("legacy_api")); // Explicitly disabled
}

/// Test branding configuration validation.
#[tokio::test]
async fn test_branding_configuration_validation() {
    // Test various branding configurations
    let branding_configs = vec![
        // Minimal branding
        scim_server::multi_tenant::BrandingConfiguration {
            display_name: "Minimal Tenant".to_string(),
            logo_url: None,
            primary_color: None,
            secondary_color: None,
            custom_css: None,
            favicon_url: None,
            footer_text: None,
            support_contact: None,
        },
        // Full branding
        scim_server::multi_tenant::BrandingConfiguration {
            display_name: "Full Branded Tenant".to_string(),
            logo_url: Some("https://example.com/logo.png".to_string()),
            primary_color: Some("#007bff".to_string()),
            secondary_color: Some("#6c757d".to_string()),
            custom_css: Some(
                ".header { background: var(--primary-color); } .footer { color: var(--secondary-color); }".to_string()
            ),
            favicon_url: Some("https://example.com/favicon.ico".to_string()),
            footer_text: Some("© 2024 Example Company. All rights reserved.".to_string()),
            support_contact: Some("support@example.com".to_string()),
        },
        // Data URI branding
        scim_server::multi_tenant::BrandingConfiguration {
            display_name: "Data URI Tenant".to_string(),
            logo_url: Some("data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mP8/5+hHgAHggJ/PchI7wAAAABJRU5ErkJggg==".to_string()),
            primary_color: Some("#ff6b35".to_string()),
            secondary_color: Some("#004e89".to_string()),
            custom_css: None,
            favicon_url: Some("data:image/x-icon;base64,AAABAAEAEBAQAAEABAAoAQAAFgAAACgAAAAQAAAAIAAAAAEABAAAAAAAgAAAAAAAAAAAAAAAEAAAAAAAAAAAAAAAEAAQAAIAAgAAAAAgACAAIAAgAAgAAIAAgAAIAAgAAAACAAIAAgAAIABAAEAAQABAAEAAIABAAEAAQABAAAAAEABAAEAAQABAAEAAQABAAAQAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA".to_string()),
            footer_text: Some("Contact us: help@company.com".to_string()),
            support_contact: Some("help@company.com".to_string()),
        },
    ];

    for branding_config in branding_configs {
        let config = TenantConfiguration::builder("branding-test".to_string())
            .with_branding_configuration(branding_config)
            .build()
            .expect("Should build configuration with branding");

        assert!(config.validate().is_ok());
    }
}

/// Test comprehensive validation scenarios.
#[tokio::test]
async fn test_comprehensive_validation_scenarios() {
    let provider = Arc::new(InMemoryConfigurationProvider::new());

    // Create a complex configuration that should pass validation
    let valid_complex_config = create_comprehensive_test_configuration();

    let context = ValidationContext {
        is_create: true,
        previous_configuration: None,
        validation_params: HashMap::new(),
    };

    provider
        .validate_configuration(&valid_complex_config, &context)
        .await
        .expect("Should validate comprehensive configuration");

    let created = provider
        .create_configuration(valid_complex_config)
        .await
        .expect("Should create comprehensive configuration");

    // Test update validation with previous configuration
    let mut updated_config = created.clone();
    updated_config.display_name = "Updated Complex Configuration".to_string();
    updated_config
        .operational
        .feature_flags
        .insert("new_feature".to_string(), true);

    let update_context = ValidationContext {
        is_create: false,
        previous_configuration: Some(created.clone()),
        validation_params: {
            let mut params = HashMap::new();
            params.insert("validate_feature_changes".to_string(), "true".to_string());
            params
        },
    };

    provider
        .validate_configuration(&updated_config, &update_context)
        .await
        .expect("Should validate updated comprehensive configuration");

    provider
        .update_configuration(updated_config)
        .await
        .expect("Should update comprehensive configuration");
}

/// Helper function to create a comprehensive test configuration.
fn create_comprehensive_test_configuration() -> TenantConfiguration {
    let schema_config = SchemaConfiguration::builder()
        .add_custom_attribute(
            "departmentCode".to_string(),
            json!({
                "type": "string",
                "multiValued": false,
                "required": true,
                "description": "Department code for organizational structure"
            }),
        )
        .add_custom_attribute(
            "employeeId".to_string(),
            json!({
                "type": "string",
                "multiValued": false,
                "required": true,
                "description": "Unique employee identifier"
            }),
        )
        .disable_standard_attribute("nickName".to_string())
        .require_attribute("department".to_string())
        .add_validation_rule(
            "departmentCode".to_string(),
            ValidationRule {
                rule_type: ValidationType::Regex,
                parameters: {
                    let mut params = HashMap::new();
                    params.insert("pattern".to_string(), json!("^[A-Z]{2,4}$"));
                    params
                },
                error_message: "Department code must be 2-4 uppercase letters".to_string(),
            },
        )
        .add_validation_rule(
            "employeeId".to_string(),
            ValidationRule {
                rule_type: ValidationType::Regex,
                parameters: {
                    let mut params = HashMap::new();
                    params.insert("pattern".to_string(), json!("^EMP\\d{6}$"));
                    params
                },
                error_message: "Employee ID must be in format EMP followed by 6 digits".to_string(),
            },
        )
        .add_schema_extension(SchemaExtension {
            id: "enterprise_extension".to_string(),
            name: "Enterprise Extension".to_string(),
            description: "Additional fields for enterprise customers".to_string(),
            schema: json!({
                "type": "object",
                "properties": {
                    "costCenter": {"type": "string"},
                    "manager": {"type": "string"},
                    "division": {"type": "string"}
                }
            }),
            required: false,
        })
        .build();

    let operational_config = OperationalConfiguration::builder()
        .with_rate_limits(RateLimitConfiguration {
            requests_per_minute: Some(500),
            requests_per_hour: Some(5000),
            requests_per_day: Some(50000),
            burst_allowance: Some(100),
            window_duration: Duration::from_secs(60),
        })
        .with_resource_limits(ResourceLimits {
            max_users: Some(5000),
            max_groups: Some(500),
            max_custom_resources: Some(1000),
            max_resource_size: Some(512 * 1024),       // 512KB
            max_total_storage: Some(50 * 1024 * 1024), // 50MB
        })
        .enable_feature("advanced_search".to_string())
        .enable_feature("bulk_operations".to_string())
        .enable_feature("audit_logging".to_string())
        .disable_feature("beta_features".to_string())
        .build();

    TenantConfiguration::builder("comprehensive-validation-test".to_string())
        .with_display_name("Comprehensive Validation Test Tenant".to_string())
        .with_schema_configuration(schema_config)
        .with_operational_configuration(operational_config)
        .build()
        .expect("Should build comprehensive test configuration")
}
