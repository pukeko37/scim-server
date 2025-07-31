//! Tenant configuration management for multi-tenant SCIM operations.
//!
//! This module provides comprehensive configuration management capabilities for
//! multi-tenant SCIM deployments, allowing each tenant to have customized
//! settings, schema extensions, and operational parameters.
//!
//! # Design Principles
//!
//! * **Type Safety**: Configuration errors caught at compile time where possible
//! * **Immutable Configuration**: Configurations are immutable once created
//! * **Validation**: All configurations validated before application
//! * **Extensibility**: Easy to add new configuration types
//!
//! # Example Usage
//!
//! ```rust,no_run
//! use scim_server::multi_tenant::{TenantConfiguration, SchemaConfiguration, OperationalConfiguration};
//! use serde_json::json;
//! use std::collections::HashMap;
//!
//! // Create schema configuration
//! let schema_config = SchemaConfiguration::builder()
//!     .add_custom_attribute("customField", json!({
//!         "type": "string",
//!         "multiValued": false,
//!         "required": false
//!     }))
//!     .disable_standard_attribute("nickName")
//!     .build();
//!
//! // Create operational configuration
//! let operational_config = OperationalConfiguration::builder()
//!     .with_rate_limit(1000)
//!     .with_max_resource_count(5000)
//!     .enable_audit_logging()
//!     .build();
//!
//! // Create complete tenant configuration
//! let tenant_config = TenantConfiguration::builder("tenant-a")
//!     .with_schema_configuration(schema_config)
//!     .with_operational_configuration(operational_config)
//!     .build()?;
//! ```

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::time::Duration;
use thiserror::Error;

/// Errors that can occur during configuration management.
#[derive(Debug, Error)]
pub enum ConfigurationError {
    /// Configuration validation failed
    #[error("Configuration validation failed: {message}")]
    ValidationError { message: String },
    /// Configuration not found
    #[error("Configuration not found for tenant: {tenant_id}")]
    NotFound { tenant_id: String },
    /// Configuration conflict
    #[error("Configuration conflict: {message}")]
    Conflict { message: String },
    /// Serialization/deserialization error
    #[error("Serialization error: {source}")]
    SerializationError {
        #[from]
        source: serde_json::Error,
    },
    /// Invalid configuration version
    #[error("Invalid configuration version: expected {expected}, got {actual}")]
    VersionMismatch { expected: u64, actual: u64 },
}

/// Complete configuration for a tenant, encompassing all customizable aspects.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TenantConfiguration {
    /// Unique identifier for the tenant
    pub tenant_id: String,
    /// Display name for the tenant
    pub display_name: String,
    /// When this configuration was created
    pub created_at: DateTime<Utc>,
    /// When this configuration was last modified
    pub last_modified: DateTime<Utc>,
    /// Configuration version for optimistic locking
    pub version: u64,
    /// Schema-related configurations
    pub schema: SchemaConfiguration,
    /// Operational settings and limits
    pub operational: OperationalConfiguration,
    /// Compliance and audit settings
    pub compliance: ComplianceConfiguration,
    /// UI and branding customizations
    pub branding: BrandingConfiguration,
}

impl TenantConfiguration {
    /// Create a new builder for tenant configuration.
    pub fn builder(tenant_id: String) -> TenantConfigurationBuilder {
        TenantConfigurationBuilder::new(tenant_id)
    }

    /// Validate the configuration for consistency and correctness.
    pub fn validate(&self) -> Result<(), ConfigurationError> {
        // Validate resource limits are reasonable
        if let Some(max_users) = self.operational.resource_limits.max_users {
            if max_users == 0 {
                return Err(ConfigurationError::ValidationError {
                    message: "Maximum users must be greater than 0".to_string(),
                });
            }
        }

        if let Some(max_groups) = self.operational.resource_limits.max_groups {
            if max_groups == 0 {
                return Err(ConfigurationError::ValidationError {
                    message: "Maximum groups must be greater than 0".to_string(),
                });
            }
        }

        if let Some(max_custom) = self.operational.resource_limits.max_custom_resources {
            if max_custom == 0 {
                return Err(ConfigurationError::ValidationError {
                    message: "Maximum custom resources must be greater than 0".to_string(),
                });
            }
        }

        if let Some(max_size) = self.operational.resource_limits.max_resource_size {
            if max_size == 0 {
                return Err(ConfigurationError::ValidationError {
                    message: "Maximum resource size must be greater than 0".to_string(),
                });
            }
        }

        if let Some(max_storage) = self.operational.resource_limits.max_total_storage {
            if max_storage == 0 {
                return Err(ConfigurationError::ValidationError {
                    message: "Maximum total storage must be greater than 0".to_string(),
                });
            }
        }

        // Validate rate limits
        if let Some(rpm) = self.operational.rate_limits.requests_per_minute {
            if rpm == 0 {
                return Err(ConfigurationError::ValidationError {
                    message: "Rate limits must be greater than 0".to_string(),
                });
            }
        }

        if let Some(rph) = self.operational.rate_limits.requests_per_hour {
            if rph == 0 {
                return Err(ConfigurationError::ValidationError {
                    message: "Rate limits must be greater than 0".to_string(),
                });
            }
        }

        if let Some(rpd) = self.operational.rate_limits.requests_per_day {
            if rpd == 0 {
                return Err(ConfigurationError::ValidationError {
                    message: "Rate limits must be greater than 0".to_string(),
                });
            }
        }

        if let Some(burst) = self.operational.rate_limits.burst_allowance {
            if burst == 0 {
                return Err(ConfigurationError::ValidationError {
                    message: "Burst allowance must be greater than 0".to_string(),
                });
            }
        }

        // Validate schema extensions don't conflict
        let mut extension_ids = std::collections::HashSet::new();
        for extension in &self.schema.schema_extensions {
            if !extension_ids.insert(&extension.id) {
                return Err(ConfigurationError::ValidationError {
                    message: format!("Duplicate schema extension ID: {}", extension.id),
                });
            }
        }

        Ok(())
    }

    /// Update the last modified timestamp and increment version.
    pub fn touch(&mut self) {
        self.last_modified = Utc::now();
        self.version += 1;
    }

    /// Check if this configuration allows a specific operation.
    pub fn allows_operation(&self, operation: &str) -> bool {
        // Check if the operation is explicitly disabled
        if let Some(&enabled) = self.operational.feature_flags.get(operation) {
            return enabled;
        }

        // Check with "allow_" prefix for backwards compatibility
        self.operational
            .feature_flags
            .get(&format!("allow_{}", operation))
            .copied()
            .unwrap_or(true)
    }
}

/// Schema customization configuration for a tenant.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SchemaConfiguration {
    /// Custom attributes to add to resources
    pub custom_attributes: HashMap<String, Value>,
    /// Standard attributes to disable/hide
    pub disabled_attributes: Vec<String>,
    /// Required attributes beyond the standard set
    pub additional_required: Vec<String>,
    /// Custom schema extensions
    pub schema_extensions: Vec<SchemaExtension>,
    /// Validation rules for custom attributes
    pub validation_rules: HashMap<String, ValidationRule>,
}

impl SchemaConfiguration {
    /// Create a new builder for schema configuration.
    pub fn builder() -> SchemaConfigurationBuilder {
        SchemaConfigurationBuilder::new()
    }

    /// Check if an attribute is disabled for this tenant.
    pub fn is_attribute_disabled(&self, attribute: &str) -> bool {
        self.disabled_attributes.contains(&attribute.to_string())
    }

    /// Check if an attribute is required for this tenant.
    pub fn is_attribute_required(&self, attribute: &str) -> bool {
        self.additional_required.contains(&attribute.to_string())
    }

    /// Get validation rule for an attribute.
    pub fn get_validation_rule(&self, attribute: &str) -> Option<&ValidationRule> {
        self.validation_rules.get(attribute)
    }
}

impl Default for SchemaConfiguration {
    fn default() -> Self {
        Self {
            custom_attributes: HashMap::new(),
            disabled_attributes: Vec::new(),
            additional_required: Vec::new(),
            schema_extensions: Vec::new(),
            validation_rules: HashMap::new(),
        }
    }
}

/// A custom schema extension for tenant-specific needs.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SchemaExtension {
    /// Unique identifier for this extension
    pub id: String,
    /// Human-readable name
    pub name: String,
    /// Description of the extension
    pub description: String,
    /// The schema definition
    pub schema: Value,
    /// Whether this extension is required
    pub required: bool,
}

/// Validation rule for custom attributes.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ValidationRule {
    /// Type of validation to perform
    pub rule_type: ValidationType,
    /// Parameters for the validation rule
    pub parameters: HashMap<String, Value>,
    /// Error message to display on validation failure
    pub error_message: String,
}

/// Types of validation that can be applied to attributes.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ValidationType {
    /// Regular expression validation
    Regex,
    /// Minimum/maximum length validation
    Length,
    /// Numeric range validation
    Range,
    /// Enum value validation
    Enum,
    /// Custom validation function
    Custom,
}

/// Operational configuration controlling tenant behavior and limits.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct OperationalConfiguration {
    /// Rate limiting configuration
    pub rate_limits: RateLimitConfiguration,
    /// Resource count limits
    pub resource_limits: ResourceLimits,
    /// Feature flags for this tenant
    pub feature_flags: HashMap<String, bool>,
    /// Session and token settings
    pub session_settings: SessionConfiguration,
    /// Backup and retention settings
    pub retention_settings: RetentionConfiguration,
    /// Performance optimization settings
    pub performance_settings: PerformanceConfiguration,
}

impl OperationalConfiguration {
    /// Create a new builder for operational configuration.
    pub fn builder() -> OperationalConfigurationBuilder {
        OperationalConfigurationBuilder::new()
    }

    /// Check if a feature flag is enabled.
    pub fn is_feature_enabled(&self, feature: &str) -> bool {
        self.feature_flags.get(feature).copied().unwrap_or(false)
    }

    /// Get the effective rate limit for a time period.
    pub fn get_rate_limit(&self, period: RateLimitPeriod) -> Option<u32> {
        match period {
            RateLimitPeriod::Minute => self.rate_limits.requests_per_minute,
            RateLimitPeriod::Hour => self.rate_limits.requests_per_hour,
            RateLimitPeriod::Day => self.rate_limits.requests_per_day,
        }
    }
}

impl Default for OperationalConfiguration {
    fn default() -> Self {
        Self {
            rate_limits: RateLimitConfiguration::default(),
            resource_limits: ResourceLimits::default(),
            feature_flags: HashMap::new(),
            session_settings: SessionConfiguration::default(),
            retention_settings: RetentionConfiguration::default(),
            performance_settings: PerformanceConfiguration::default(),
        }
    }
}

/// Rate limit time periods.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RateLimitPeriod {
    Minute,
    Hour,
    Day,
}

/// Rate limiting configuration for tenant operations.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RateLimitConfiguration {
    /// Requests per minute limit
    pub requests_per_minute: Option<u32>,
    /// Requests per hour limit
    pub requests_per_hour: Option<u32>,
    /// Requests per day limit
    pub requests_per_day: Option<u32>,
    /// Burst allowance for short-term spikes
    pub burst_allowance: Option<u32>,
    /// Rate limit window duration
    #[serde(with = "duration_serde")]
    pub window_duration: Duration,
}

impl Default for RateLimitConfiguration {
    fn default() -> Self {
        Self {
            requests_per_minute: Some(1000),
            requests_per_hour: Some(10000),
            requests_per_day: Some(100000),
            burst_allowance: Some(100),
            window_duration: Duration::from_secs(60),
        }
    }
}

/// Resource count limits for different resource types.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ResourceLimits {
    /// Maximum number of users
    pub max_users: Option<usize>,
    /// Maximum number of groups
    pub max_groups: Option<usize>,
    /// Maximum number of custom resources
    pub max_custom_resources: Option<usize>,
    /// Maximum size of a single resource in bytes
    pub max_resource_size: Option<usize>,
    /// Maximum total storage for the tenant in bytes
    pub max_total_storage: Option<usize>,
}

impl Default for ResourceLimits {
    fn default() -> Self {
        Self {
            max_users: Some(10000),
            max_groups: Some(1000),
            max_custom_resources: Some(1000),
            max_resource_size: Some(1024 * 1024),       // 1MB
            max_total_storage: Some(100 * 1024 * 1024), // 100MB
        }
    }
}

/// Session and authentication configuration.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SessionConfiguration {
    /// Session timeout duration
    #[serde(with = "duration_serde")]
    pub session_timeout: Duration,
    /// Token expiration duration
    #[serde(with = "duration_serde")]
    pub token_expiration: Duration,
    /// Whether to allow concurrent sessions
    pub allow_concurrent_sessions: bool,
    /// Maximum number of active sessions
    pub max_active_sessions: Option<u32>,
    /// Whether to require multi-factor authentication
    pub require_mfa: bool,
}

impl Default for SessionConfiguration {
    fn default() -> Self {
        Self {
            session_timeout: Duration::from_secs(3600),   // 1 hour
            token_expiration: Duration::from_secs(86400), // 24 hours
            allow_concurrent_sessions: true,
            max_active_sessions: Some(10),
            require_mfa: false,
        }
    }
}

/// Data retention and backup configuration.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RetentionConfiguration {
    /// How long to retain audit logs
    #[serde(with = "duration_serde")]
    pub audit_log_retention: Duration,
    /// How long to retain deleted resources
    #[serde(with = "duration_serde")]
    pub deleted_resource_retention: Duration,
    /// Whether to enable automatic backups
    pub enable_automatic_backup: bool,
    /// Backup frequency
    #[serde(with = "duration_serde")]
    pub backup_frequency: Duration,
    /// How long to retain backups
    #[serde(with = "duration_serde")]
    pub backup_retention: Duration,
}

impl Default for RetentionConfiguration {
    fn default() -> Self {
        Self {
            audit_log_retention: Duration::from_secs(365 * 24 * 3600), // 1 year
            deleted_resource_retention: Duration::from_secs(30 * 24 * 3600), // 30 days
            enable_automatic_backup: true,
            backup_frequency: Duration::from_secs(24 * 3600), // Daily
            backup_retention: Duration::from_secs(90 * 24 * 3600), // 90 days
        }
    }
}

/// Performance optimization settings.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PerformanceConfiguration {
    /// Enable caching for frequently accessed resources
    pub enable_caching: bool,
    /// Cache TTL duration
    #[serde(with = "duration_serde")]
    pub cache_ttl: Duration,
    /// Maximum cache size in MB
    pub max_cache_size: Option<usize>,
    /// Enable query optimization
    pub enable_query_optimization: bool,
    /// Connection pool size for database operations
    pub connection_pool_size: Option<u32>,
}

impl Default for PerformanceConfiguration {
    fn default() -> Self {
        Self {
            enable_caching: true,
            cache_ttl: Duration::from_secs(300), // 5 minutes
            max_cache_size: Some(100),           // 100MB
            enable_query_optimization: true,
            connection_pool_size: Some(10),
        }
    }
}

/// Compliance and audit configuration.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ComplianceConfiguration {
    /// Audit logging level
    pub audit_level: AuditLevel,
    /// Data encryption requirements
    pub encryption_requirements: EncryptionConfiguration,
    /// Compliance frameworks this tenant must adhere to
    pub compliance_frameworks: Vec<ComplianceFramework>,
    /// Data residency requirements
    pub data_residency: Option<String>,
    /// Whether PII scrubbing is enabled
    pub enable_pii_scrubbing: bool,
    /// Retention policies for different data types
    #[serde(with = "duration_map_serde")]
    pub data_retention_policies: HashMap<String, Duration>,
}

impl Default for ComplianceConfiguration {
    fn default() -> Self {
        Self {
            audit_level: AuditLevel::Basic,
            encryption_requirements: EncryptionConfiguration::default(),
            compliance_frameworks: Vec::new(),
            data_residency: None,
            enable_pii_scrubbing: false,
            data_retention_policies: HashMap::new(),
        }
    }
}

/// Level of audit logging to perform.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum AuditLevel {
    /// No audit logging
    None,
    /// Log only critical operations
    Basic,
    /// Log all operations
    Full,
    /// Log everything including data changes
    Detailed,
}

/// Encryption configuration for tenant data.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EncryptionConfiguration {
    /// Require encryption at rest
    pub encrypt_at_rest: bool,
    /// Require encryption in transit
    pub encrypt_in_transit: bool,
    /// Encryption algorithm to use
    pub encryption_algorithm: Option<String>,
    /// Key rotation frequency
    #[serde(with = "duration_option_serde")]
    pub key_rotation_frequency: Option<Duration>,
    /// Whether to use customer-managed keys
    pub customer_managed_keys: bool,
}

impl Default for EncryptionConfiguration {
    fn default() -> Self {
        Self {
            encrypt_at_rest: true,
            encrypt_in_transit: true,
            encryption_algorithm: Some("AES-256-GCM".to_string()),
            key_rotation_frequency: Some(Duration::from_secs(90 * 24 * 3600)), // 90 days
            customer_managed_keys: false,
        }
    }
}

/// Compliance frameworks that may apply to a tenant.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ComplianceFramework {
    /// General Data Protection Regulation
    GDPR,
    /// Health Insurance Portability and Accountability Act
    HIPAA,
    /// Sarbanes-Oxley Act
    SOX,
    /// Payment Card Industry Data Security Standard
    PCIDSS,
    /// ISO 27001
    ISO27001,
    /// SOC 2
    SOC2,
    /// Custom compliance framework
    Custom(String),
}

/// Branding and UI customization configuration.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BrandingConfiguration {
    /// Tenant display name for UI
    pub display_name: String,
    /// Logo URL or data
    pub logo_url: Option<String>,
    /// Primary brand color
    pub primary_color: Option<String>,
    /// Secondary brand color
    pub secondary_color: Option<String>,
    /// Custom CSS for UI customization
    pub custom_css: Option<String>,
    /// Favicon URL or data
    pub favicon_url: Option<String>,
    /// Custom footer text
    pub footer_text: Option<String>,
    /// Support contact information
    pub support_contact: Option<String>,
}

impl Default for BrandingConfiguration {
    fn default() -> Self {
        Self {
            display_name: "Default Tenant".to_string(),
            logo_url: None,
            primary_color: Some("#007bff".to_string()),
            secondary_color: Some("#6c757d".to_string()),
            custom_css: None,
            favicon_url: None,
            footer_text: None,
            support_contact: None,
        }
    }
}

/// Builder for creating tenant configurations with validation.
pub struct TenantConfigurationBuilder {
    tenant_id: String,
    display_name: Option<String>,
    schema: Option<SchemaConfiguration>,
    operational: Option<OperationalConfiguration>,
    compliance: Option<ComplianceConfiguration>,
    branding: Option<BrandingConfiguration>,
}

impl TenantConfigurationBuilder {
    /// Create a new builder for the specified tenant.
    pub fn new(tenant_id: String) -> Self {
        Self {
            tenant_id,
            display_name: None,
            schema: None,
            operational: None,
            compliance: None,
            branding: None,
        }
    }

    /// Set the display name for the tenant.
    pub fn with_display_name(mut self, display_name: String) -> Self {
        self.display_name = Some(display_name);
        self
    }

    /// Set the schema configuration.
    pub fn with_schema_configuration(mut self, schema: SchemaConfiguration) -> Self {
        self.schema = Some(schema);
        self
    }

    /// Set the operational configuration.
    pub fn with_operational_configuration(mut self, operational: OperationalConfiguration) -> Self {
        self.operational = Some(operational);
        self
    }

    /// Set the compliance configuration.
    pub fn with_compliance_configuration(mut self, compliance: ComplianceConfiguration) -> Self {
        self.compliance = Some(compliance);
        self
    }

    /// Set the branding configuration.
    pub fn with_branding_configuration(mut self, branding: BrandingConfiguration) -> Self {
        self.branding = Some(branding);
        self
    }

    /// Build the tenant configuration with validation.
    pub fn build(self) -> Result<TenantConfiguration, ConfigurationError> {
        let now = Utc::now();
        let display_name = self.display_name.unwrap_or_else(|| self.tenant_id.clone());

        let config = TenantConfiguration {
            tenant_id: self.tenant_id,
            display_name,
            created_at: now,
            last_modified: now,
            version: 1,
            schema: self.schema.unwrap_or_default(),
            operational: self.operational.unwrap_or_default(),
            compliance: self.compliance.unwrap_or_default(),
            branding: self.branding.unwrap_or_default(),
        };

        // Validate the configuration
        config.validate()?;
        Ok(config)
    }
}

/// Builder for schema configurations.
pub struct SchemaConfigurationBuilder {
    custom_attributes: HashMap<String, Value>,
    disabled_attributes: Vec<String>,
    additional_required: Vec<String>,
    schema_extensions: Vec<SchemaExtension>,
    validation_rules: HashMap<String, ValidationRule>,
}

impl SchemaConfigurationBuilder {
    /// Create a new schema configuration builder.
    pub fn new() -> Self {
        Self {
            custom_attributes: HashMap::new(),
            disabled_attributes: Vec::new(),
            additional_required: Vec::new(),
            schema_extensions: Vec::new(),
            validation_rules: HashMap::new(),
        }
    }

    /// Add a custom attribute definition.
    pub fn add_custom_attribute(mut self, name: String, definition: Value) -> Self {
        self.custom_attributes.insert(name, definition);
        self
    }

    /// Disable a standard SCIM attribute.
    pub fn disable_standard_attribute(mut self, attribute: String) -> Self {
        self.disabled_attributes.push(attribute);
        self
    }

    /// Make an attribute required beyond the standard requirements.
    pub fn require_attribute(mut self, attribute: String) -> Self {
        self.additional_required.push(attribute);
        self
    }

    /// Add a schema extension.
    pub fn add_schema_extension(mut self, extension: SchemaExtension) -> Self {
        self.schema_extensions.push(extension);
        self
    }

    /// Add a validation rule for an attribute.
    pub fn add_validation_rule(mut self, attribute: String, rule: ValidationRule) -> Self {
        self.validation_rules.insert(attribute, rule);
        self
    }

    /// Build the schema configuration.
    pub fn build(self) -> SchemaConfiguration {
        SchemaConfiguration {
            custom_attributes: self.custom_attributes,
            disabled_attributes: self.disabled_attributes,
            additional_required: self.additional_required,
            schema_extensions: self.schema_extensions,
            validation_rules: self.validation_rules,
        }
    }
}

/// Builder for operational configurations.
pub struct OperationalConfigurationBuilder {
    rate_limits: Option<RateLimitConfiguration>,
    resource_limits: Option<ResourceLimits>,
    feature_flags: HashMap<String, bool>,
    session_settings: Option<SessionConfiguration>,
    retention_settings: Option<RetentionConfiguration>,
    performance_settings: Option<PerformanceConfiguration>,
}

impl OperationalConfigurationBuilder {
    /// Create a new operational configuration builder.
    pub fn new() -> Self {
        Self {
            rate_limits: None,
            resource_limits: None,
            feature_flags: HashMap::new(),
            session_settings: None,
            retention_settings: None,
            performance_settings: None,
        }
    }

    /// Set rate limiting configuration.
    pub fn with_rate_limits(mut self, rate_limits: RateLimitConfiguration) -> Self {
        self.rate_limits = Some(rate_limits);
        self
    }

    /// Set resource limits.
    pub fn with_resource_limits(mut self, resource_limits: ResourceLimits) -> Self {
        self.resource_limits = Some(resource_limits);
        self
    }

    /// Enable a feature flag.
    pub fn enable_feature(mut self, feature: String) -> Self {
        self.feature_flags.insert(feature, true);
        self
    }

    /// Disable a feature flag.
    pub fn disable_feature(mut self, feature: String) -> Self {
        self.feature_flags.insert(feature, false);
        self
    }

    /// Set session configuration.
    pub fn with_session_settings(mut self, session_settings: SessionConfiguration) -> Self {
        self.session_settings = Some(session_settings);
        self
    }

    /// Set retention configuration.
    pub fn with_retention_settings(mut self, retention_settings: RetentionConfiguration) -> Self {
        self.retention_settings = Some(retention_settings);
        self
    }

    /// Set performance configuration.
    pub fn with_performance_settings(
        mut self,
        performance_settings: PerformanceConfiguration,
    ) -> Self {
        self.performance_settings = Some(performance_settings);
        self
    }

    /// Build the operational configuration.
    pub fn build(self) -> OperationalConfiguration {
        OperationalConfiguration {
            rate_limits: self.rate_limits.unwrap_or_default(),
            resource_limits: self.resource_limits.unwrap_or_default(),
            feature_flags: self.feature_flags,
            session_settings: self.session_settings.unwrap_or_default(),
            retention_settings: self.retention_settings.unwrap_or_default(),
            performance_settings: self.performance_settings.unwrap_or_default(),
        }
    }
}

/// Serde module for Duration serialization.
mod duration_serde {
    use serde::{Deserialize, Deserializer, Serialize, Serializer};
    use std::time::Duration;

    pub fn serialize<S>(duration: &Duration, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        duration.as_secs().serialize(serializer)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Duration, D::Error>
    where
        D: Deserializer<'de>,
    {
        let secs = u64::deserialize(deserializer)?;
        Ok(Duration::from_secs(secs))
    }
}

/// Serde module for Option<Duration> serialization.
mod duration_option_serde {
    use serde::{Deserialize, Deserializer, Serialize, Serializer};
    use std::time::Duration;

    pub fn serialize<S>(duration: &Option<Duration>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match duration {
            Some(d) => Some(d.as_secs()).serialize(serializer),
            None => None::<u64>.serialize(serializer),
        }
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Option<Duration>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let secs = Option::<u64>::deserialize(deserializer)?;
        Ok(secs.map(Duration::from_secs))
    }
}

/// Serde module for HashMap<String, Duration> serialization.
mod duration_map_serde {
    use serde::{Deserialize, Deserializer, Serialize, Serializer};
    use std::collections::HashMap;
    use std::time::Duration;

    pub fn serialize<S>(map: &HashMap<String, Duration>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let secs_map: HashMap<String, u64> =
            map.iter().map(|(k, v)| (k.clone(), v.as_secs())).collect();
        secs_map.serialize(serializer)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<HashMap<String, Duration>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let secs_map = HashMap::<String, u64>::deserialize(deserializer)?;
        Ok(secs_map
            .into_iter()
            .map(|(k, v)| (k, Duration::from_secs(v)))
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_tenant_configuration_builder() {
        let config = TenantConfiguration::builder("test-tenant".to_string())
            .with_display_name("Test Tenant".to_string())
            .build()
            .expect("Should build valid configuration");

        assert_eq!(config.tenant_id, "test-tenant");
        assert_eq!(config.display_name, "Test Tenant");
        assert_eq!(config.version, 1);
    }

    #[test]
    fn test_schema_configuration_builder() {
        let schema_config = SchemaConfiguration::builder()
            .add_custom_attribute(
                "customField".to_string(),
                json!({
                    "type": "string",
                    "required": false
                }),
            )
            .disable_standard_attribute("nickName".to_string())
            .require_attribute("department".to_string())
            .build();

        assert!(schema_config.custom_attributes.contains_key("customField"));
        assert!(
            schema_config
                .disabled_attributes
                .contains(&"nickName".to_string())
        );
        assert!(
            schema_config
                .additional_required
                .contains(&"department".to_string())
        );
    }

    #[test]
    fn test_operational_configuration_builder() {
        let op_config = OperationalConfiguration::builder()
            .enable_feature("advanced_search".to_string())
            .disable_feature("bulk_operations".to_string())
            .build();

        assert_eq!(op_config.feature_flags.get("advanced_search"), Some(&true));
        assert_eq!(op_config.feature_flags.get("bulk_operations"), Some(&false));
    }

    #[test]
    fn test_configuration_validation() {
        // Valid configuration should pass
        let valid_config = TenantConfiguration::builder("test-tenant".to_string())
            .build()
            .expect("Should build valid configuration");

        assert!(valid_config.validate().is_ok());

        // Invalid configuration should fail
        let mut invalid_config = valid_config.clone();
        invalid_config.operational.resource_limits.max_users = Some(0);
        assert!(invalid_config.validate().is_err());
    }

    #[test]
    fn test_configuration_touch() {
        let mut config = TenantConfiguration::builder("test-tenant".to_string())
            .build()
            .expect("Should build valid configuration");

        let original_version = config.version;
        let original_modified = config.last_modified;

        // Wait a bit to ensure timestamp difference
        std::thread::sleep(std::time::Duration::from_millis(1));
        config.touch();

        assert_eq!(config.version, original_version + 1);
        assert!(config.last_modified > original_modified);
    }

    #[test]
    fn test_schema_configuration_methods() {
        let config = SchemaConfiguration::builder()
            .disable_standard_attribute("nickName".to_string())
            .require_attribute("department".to_string())
            .build();

        assert!(config.is_attribute_disabled("nickName"));
        assert!(!config.is_attribute_disabled("userName"));
        assert!(config.is_attribute_required("department"));
        assert!(!config.is_attribute_required("nickName"));
    }

    #[test]
    fn test_operational_configuration_methods() {
        let config = OperationalConfiguration::builder()
            .enable_feature("advanced_search".to_string())
            .build();

        assert!(config.is_feature_enabled("advanced_search"));
        assert!(!config.is_feature_enabled("nonexistent_feature"));

        assert!(config.get_rate_limit(RateLimitPeriod::Minute).is_some());
    }

    #[test]
    fn test_serialization() {
        let config = TenantConfiguration::builder("test-tenant".to_string())
            .with_display_name("Test Tenant".to_string())
            .build()
            .expect("Should build valid configuration");

        let json = serde_json::to_string(&config).expect("Should serialize");
        let deserialized: TenantConfiguration =
            serde_json::from_str(&json).expect("Should deserialize");

        assert_eq!(config, deserialized);
    }

    #[test]
    fn test_default_configurations() {
        let schema_default = SchemaConfiguration::default();
        assert!(schema_default.custom_attributes.is_empty());
        assert!(schema_default.disabled_attributes.is_empty());

        let operational_default = OperationalConfiguration::default();
        assert!(
            operational_default
                .rate_limits
                .requests_per_minute
                .is_some()
        );
        assert!(operational_default.performance_settings.enable_caching);

        let compliance_default = ComplianceConfiguration::default();
        assert_eq!(compliance_default.audit_level, AuditLevel::Basic);
        assert!(compliance_default.encryption_requirements.encrypt_at_rest);
    }
}
