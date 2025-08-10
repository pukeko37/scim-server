//! SCIM-specific tenant configuration for multi-tenant SCIM operations.
//!
//! This module provides SCIM-focused configuration management for multi-tenant
//! deployments. Unlike general-purpose configuration systems, this focuses
//! exclusively on SCIM protocol requirements and multi-tenant orchestration.
//!
//! # Design Principles
//!
//! * **SCIM Protocol Focus**: Only configuration related to SCIM 2.0 specification
//! * **Tenant Isolation**: Configuration for SCIM-level tenant separation
//! * **Client Management**: SCIM client connection and authentication settings
//! * **Protocol Compliance**: Settings that affect SCIM protocol behavior
//!
//! # Scope Boundaries
//!
//! ## ✅ In Scope (SCIM-Specific Configuration)
//! - SCIM endpoint configuration per tenant
//! - SCIM client authentication and connection settings
//! - SCIM protocol rate limiting and throttling
//! - SCIM schema extensions and customizations
//! - SCIM operation audit trails
//! - SCIM filtering and search configuration
//!
//! ## ❌ Out of Scope (General Application Configuration)
//! - UI branding and theming
//! - General performance tuning
//! - Business logic configuration
//! - General session management
//! - Infrastructure encryption settings
//! - General compliance frameworks
//!
//! # Example Usage
//!
//! ```rust
//! use scim_server::multi_tenant::{ScimTenantConfiguration, ScimEndpointConfig};
//! use std::time::Duration;
//!
//! // Create SCIM-specific tenant configuration
//! let config = ScimTenantConfiguration::builder("tenant-a".to_string())
//!     .with_endpoint_path("/scim/v2")
//!     .with_scim_rate_limit(100, Duration::from_secs(60))
//!     .with_scim_client("client-1", "api_key_123")
//!     .enable_scim_audit_log()
//!     .build()
//!     .expect("Valid SCIM configuration");
//! ```

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::time::Duration;
use thiserror::Error;

/// Errors specific to SCIM configuration management.
#[derive(Debug, Error)]
pub enum ScimConfigurationError {
    /// SCIM configuration validation failed
    #[error("SCIM configuration validation failed: {message}")]
    ValidationError { message: String },
    /// SCIM configuration not found for tenant
    #[error("SCIM configuration not found for tenant: {tenant_id}")]
    NotFound { tenant_id: String },
    /// SCIM client configuration conflict
    #[error("SCIM client configuration conflict: {message}")]
    ClientConflict { message: String },
    /// Invalid SCIM endpoint configuration
    #[error("Invalid SCIM endpoint configuration: {message}")]
    InvalidEndpoint { message: String },
    /// SCIM schema extension error
    #[error("SCIM schema extension error: {message}")]
    SchemaExtensionError { message: String },
}

/// Complete SCIM-specific configuration for a tenant.
///
/// This configuration encompasses all SCIM protocol-related settings
/// for a tenant, including endpoint configuration, client connections,
/// schema customizations, and protocol-specific operational settings.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ScimTenantConfiguration {
    /// Unique identifier for the tenant
    pub tenant_id: String,
    /// When this configuration was created
    pub created_at: DateTime<Utc>,
    /// When this configuration was last modified
    pub last_modified: DateTime<Utc>,
    /// Configuration version for optimistic locking
    pub version: u64,
    /// SCIM endpoint configuration
    pub endpoint: ScimEndpointConfig,
    /// SCIM client connection configurations
    pub clients: Vec<ScimClientConfig>,
    /// SCIM protocol-specific rate limiting
    pub rate_limits: ScimRateLimits,
    /// SCIM schema extensions and customizations
    pub schema_config: ScimSchemaConfig,
    /// SCIM operation audit settings
    pub audit_config: ScimAuditConfig,
    /// SCIM filtering and search configuration
    pub search_config: ScimSearchConfig,
}

impl ScimTenantConfiguration {
    /// Create a new builder for SCIM tenant configuration.
    pub fn builder(tenant_id: String) -> ScimTenantConfigurationBuilder {
        ScimTenantConfigurationBuilder::new(tenant_id)
    }

    /// Get SCIM client configuration by client ID.
    pub fn get_client_config(&self, client_id: &str) -> Option<&ScimClientConfig> {
        self.clients.iter().find(|c| c.client_id == client_id)
    }

    /// Check if a SCIM operation is rate limited for this tenant.
    pub fn is_rate_limited(&self, operation: &str, current_count: u32) -> bool {
        match operation {
            "create" => self.rate_limits.check_create_limit(current_count),
            "read" => self.rate_limits.check_read_limit(current_count),
            "update" => self.rate_limits.check_update_limit(current_count),
            "delete" => self.rate_limits.check_delete_limit(current_count),
            "list" => self.rate_limits.check_list_limit(current_count),
            "search" => self.rate_limits.check_search_limit(current_count),
            _ => false,
        }
    }

    /// Check if a SCIM schema extension is enabled for this tenant.
    pub fn has_schema_extension(&self, extension_uri: &str) -> bool {
        self.schema_config
            .extensions
            .iter()
            .any(|ext| ext.uri == extension_uri && ext.enabled)
    }
}

/// SCIM endpoint configuration for a tenant.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ScimEndpointConfig {
    /// Base path for SCIM endpoints (e.g., "/scim/v2")
    pub base_path: String,
    /// Whether to include tenant ID in the path
    pub include_tenant_in_path: bool,
    /// Custom path pattern if include_tenant_in_path is true
    pub tenant_path_pattern: Option<String>,
    /// Maximum request payload size for SCIM operations
    pub max_payload_size: usize,
    /// SCIM protocol version (typically "2.0")
    pub scim_version: String,
    /// Supported SCIM authentication schemes
    pub supported_auth_schemes: Vec<ScimAuthScheme>,
}

impl Default for ScimEndpointConfig {
    fn default() -> Self {
        Self {
            base_path: "/scim/v2".to_string(),
            include_tenant_in_path: false,
            tenant_path_pattern: None,
            max_payload_size: 1024 * 1024, // 1MB
            scim_version: "2.0".to_string(),
            supported_auth_schemes: vec![ScimAuthScheme::Bearer, ScimAuthScheme::ApiKey],
        }
    }
}

/// SCIM authentication schemes supported by the endpoint.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ScimAuthScheme {
    /// Bearer token authentication
    Bearer,
    /// API key authentication
    ApiKey,
    /// HTTP Basic authentication
    Basic,
    /// OAuth 2.0 authentication
    OAuth2,
    /// Custom authentication scheme
    Custom(String),
}

/// SCIM client connection configuration.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ScimClientConfig {
    /// Unique identifier for this SCIM client
    pub client_id: String,
    /// Human-readable name for the client
    pub client_name: String,
    /// Authentication credentials for this client
    pub auth_config: ScimClientAuth,
    /// Client-specific rate limits (overrides tenant defaults)
    pub rate_limits: Option<ScimRateLimits>,
    /// SCIM operations this client is allowed to perform
    pub allowed_operations: Vec<ScimOperation>,
    /// Resource types this client can access
    pub allowed_resource_types: Vec<String>,
    /// Whether audit logging is enabled for this client
    pub audit_enabled: bool,
    /// Client-specific configuration metadata
    pub metadata: HashMap<String, Value>,
}

/// SCIM client authentication configuration.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ScimClientAuth {
    /// Authentication scheme for this client
    pub scheme: ScimAuthScheme,
    /// Authentication credentials (hashed/encrypted)
    pub credentials: HashMap<String, String>,
    /// Token expiration settings
    pub token_expiration: Option<Duration>,
    /// Whether to validate client IP restrictions
    pub ip_restrictions: Vec<String>,
}

/// SCIM operations that can be performed.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ScimOperation {
    /// Create new resources
    Create,
    /// Read existing resources
    Read,
    /// Update existing resources
    Update,
    /// Delete resources
    Delete,
    /// List resources with pagination
    List,
    /// Search resources with filtering
    Search,
    /// Bulk operations
    Bulk,
    /// Schema discovery
    Schema,
}

/// SCIM protocol-specific rate limiting configuration.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ScimRateLimits {
    /// Rate limit for SCIM create operations
    pub create_operations: Option<RateLimit>,
    /// Rate limit for SCIM read operations
    pub read_operations: Option<RateLimit>,
    /// Rate limit for SCIM update operations
    pub update_operations: Option<RateLimit>,
    /// Rate limit for SCIM delete operations
    pub delete_operations: Option<RateLimit>,
    /// Rate limit for SCIM list operations
    pub list_operations: Option<RateLimit>,
    /// Rate limit for SCIM search operations
    pub search_operations: Option<RateLimit>,
    /// Rate limit for SCIM bulk operations
    pub bulk_operations: Option<RateLimit>,
    /// Global rate limit across all SCIM operations
    pub global_limit: Option<RateLimit>,
}

impl ScimRateLimits {
    pub fn check_create_limit(&self, current_count: u32) -> bool {
        self.create_operations
            .as_ref()
            .map_or(false, |limit| current_count >= limit.max_requests)
    }

    pub fn check_read_limit(&self, current_count: u32) -> bool {
        self.read_operations
            .as_ref()
            .map_or(false, |limit| current_count >= limit.max_requests)
    }

    pub fn check_update_limit(&self, current_count: u32) -> bool {
        self.update_operations
            .as_ref()
            .map_or(false, |limit| current_count >= limit.max_requests)
    }

    pub fn check_delete_limit(&self, current_count: u32) -> bool {
        self.delete_operations
            .as_ref()
            .map_or(false, |limit| current_count >= limit.max_requests)
    }

    pub fn check_list_limit(&self, current_count: u32) -> bool {
        self.list_operations
            .as_ref()
            .map_or(false, |limit| current_count >= limit.max_requests)
    }

    pub fn check_search_limit(&self, current_count: u32) -> bool {
        self.search_operations
            .as_ref()
            .map_or(false, |limit| current_count >= limit.max_requests)
    }
}

impl Default for ScimRateLimits {
    fn default() -> Self {
        Self {
            create_operations: Some(RateLimit::new(100, Duration::from_secs(60))),
            read_operations: Some(RateLimit::new(1000, Duration::from_secs(60))),
            update_operations: Some(RateLimit::new(100, Duration::from_secs(60))),
            delete_operations: Some(RateLimit::new(50, Duration::from_secs(60))),
            list_operations: Some(RateLimit::new(200, Duration::from_secs(60))),
            search_operations: Some(RateLimit::new(100, Duration::from_secs(60))),
            bulk_operations: Some(RateLimit::new(10, Duration::from_secs(60))),
            global_limit: Some(RateLimit::new(2000, Duration::from_secs(60))),
        }
    }
}

/// Rate limiting configuration for specific operations.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RateLimit {
    /// Maximum number of requests allowed
    pub max_requests: u32,
    /// Time window for the rate limit
    #[serde(with = "duration_serde")]
    pub window: Duration,
    /// Burst allowance for short-term spikes
    pub burst_allowance: Option<u32>,
}

impl RateLimit {
    pub fn new(max_requests: u32, window: Duration) -> Self {
        Self {
            max_requests,
            window,
            burst_allowance: None,
        }
    }

    pub fn with_burst(mut self, burst: u32) -> Self {
        self.burst_allowance = Some(burst);
        self
    }
}

/// SCIM schema customization configuration.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ScimSchemaConfig {
    /// SCIM schema extensions enabled for this tenant
    pub extensions: Vec<ScimSchemaExtension>,
    /// Custom attributes added to standard SCIM schemas
    pub custom_attributes: HashMap<String, ScimCustomAttribute>,
    /// Standard SCIM attributes disabled for this tenant
    pub disabled_attributes: Vec<String>,
    /// Additional required attributes for this tenant
    pub additional_required: Vec<String>,
}

impl Default for ScimSchemaConfig {
    fn default() -> Self {
        Self {
            extensions: Vec::new(),
            custom_attributes: HashMap::new(),
            disabled_attributes: Vec::new(),
            additional_required: Vec::new(),
        }
    }
}

/// SCIM schema extension configuration.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ScimSchemaExtension {
    /// SCIM extension URI (e.g., "urn:ietf:params:scim:schemas:extension:enterprise:2.0:User")
    pub uri: String,
    /// Whether this extension is enabled for the tenant
    pub enabled: bool,
    /// Whether this extension is required for resources
    pub required: bool,
    /// Custom attributes defined in this extension
    pub attributes: HashMap<String, ScimCustomAttribute>,
}

/// Custom SCIM attribute definition.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ScimCustomAttribute {
    /// Attribute name
    pub name: String,
    /// SCIM attribute type (string, boolean, decimal, integer, dateTime, reference, complex)
    pub attribute_type: String,
    /// Whether the attribute supports multiple values
    pub multi_valued: bool,
    /// Whether the attribute is required
    pub required: bool,
    /// Whether the attribute is case-sensitive
    pub case_exact: bool,
    /// Mutability of the attribute (readOnly, readWrite, immutable, writeOnly)
    pub mutability: String,
    /// When the attribute is returned (always, never, default, request)
    pub returned: String,
    /// Uniqueness constraint (none, server, global)
    pub uniqueness: String,
    /// Description of the attribute
    pub description: Option<String>,
    /// Canonical values for the attribute
    pub canonical_values: Option<Vec<String>>,
}

/// SCIM operation audit configuration.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ScimAuditConfig {
    /// Whether SCIM operation auditing is enabled
    pub enabled: bool,
    /// SCIM operations to audit
    pub audited_operations: Vec<ScimOperation>,
    /// Whether to include request/response payloads in audit logs
    pub include_payloads: bool,
    /// Whether to include sensitive attributes in audit logs
    pub include_sensitive_data: bool,
    /// How long to retain SCIM audit logs
    #[serde(with = "duration_serde")]
    pub retention_period: Duration,
    /// Additional metadata to include in audit logs
    pub additional_metadata: HashMap<String, String>,
}

impl Default for ScimAuditConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            audited_operations: vec![
                ScimOperation::Create,
                ScimOperation::Update,
                ScimOperation::Delete,
            ],
            include_payloads: false,
            include_sensitive_data: false,
            retention_period: Duration::from_secs(90 * 24 * 60 * 60), // 90 days
            additional_metadata: HashMap::new(),
        }
    }
}

/// SCIM search and filtering configuration.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ScimSearchConfig {
    /// Maximum number of resources returned in a single search
    pub max_results: u32,
    /// Default number of resources returned if not specified
    pub default_count: u32,
    /// Maximum allowed depth for complex filter expressions
    pub max_filter_depth: u32,
    /// Attributes that support filtering
    pub filterable_attributes: Vec<String>,
    /// Attributes that support sorting
    pub sortable_attributes: Vec<String>,
    /// Whether case-insensitive filtering is supported
    pub case_insensitive_filtering: bool,
    /// Custom search operators supported
    pub custom_operators: Vec<String>,
}

impl Default for ScimSearchConfig {
    fn default() -> Self {
        Self {
            max_results: 200,
            default_count: 20,
            max_filter_depth: 10,
            filterable_attributes: vec![
                "userName".to_string(),
                "displayName".to_string(),
                "emails.value".to_string(),
                "active".to_string(),
                "meta.created".to_string(),
                "meta.lastModified".to_string(),
            ],
            sortable_attributes: vec![
                "userName".to_string(),
                "displayName".to_string(),
                "meta.created".to_string(),
                "meta.lastModified".to_string(),
            ],
            case_insensitive_filtering: true,
            custom_operators: Vec::new(),
        }
    }
}

/// Builder for creating SCIM tenant configurations.
pub struct ScimTenantConfigurationBuilder {
    tenant_id: String,
    endpoint: Option<ScimEndpointConfig>,
    clients: Vec<ScimClientConfig>,
    rate_limits: Option<ScimRateLimits>,
    schema_config: Option<ScimSchemaConfig>,
    audit_config: Option<ScimAuditConfig>,
    search_config: Option<ScimSearchConfig>,
}

impl ScimTenantConfigurationBuilder {
    pub fn new(tenant_id: String) -> Self {
        Self {
            tenant_id,
            endpoint: None,
            clients: Vec::new(),
            rate_limits: None,
            schema_config: None,
            audit_config: None,
            search_config: None,
        }
    }

    pub fn with_endpoint_path(mut self, path: &str) -> Self {
        let mut endpoint = self.endpoint.unwrap_or_default();
        endpoint.base_path = path.to_string();
        self.endpoint = Some(endpoint);
        self
    }

    pub fn with_scim_rate_limit(mut self, max_requests: u32, window: Duration) -> Self {
        let rate_limit = RateLimit::new(max_requests, window);
        let mut rate_limits = self.rate_limits.unwrap_or_default();

        // Set the global limit and all operation-specific limits to the same value
        rate_limits.global_limit = Some(rate_limit.clone());
        rate_limits.create_operations = Some(rate_limit.clone());
        rate_limits.read_operations = Some(rate_limit.clone());
        rate_limits.update_operations = Some(rate_limit.clone());
        rate_limits.delete_operations = Some(rate_limit.clone());
        rate_limits.list_operations = Some(rate_limit.clone());
        rate_limits.search_operations = Some(rate_limit.clone());
        rate_limits.bulk_operations = Some(rate_limit);

        self.rate_limits = Some(rate_limits);
        self
    }

    pub fn with_scim_client(mut self, client_id: &str, api_key: &str) -> Self {
        let mut credentials = HashMap::new();
        credentials.insert("api_key".to_string(), api_key.to_string());

        let client = ScimClientConfig {
            client_id: client_id.to_string(),
            client_name: client_id.to_string(),
            auth_config: ScimClientAuth {
                scheme: ScimAuthScheme::ApiKey,
                credentials,
                token_expiration: None,
                ip_restrictions: Vec::new(),
            },
            rate_limits: None,
            allowed_operations: vec![
                ScimOperation::Create,
                ScimOperation::Read,
                ScimOperation::Update,
                ScimOperation::Delete,
                ScimOperation::List,
                ScimOperation::Search,
            ],
            allowed_resource_types: vec!["User".to_string(), "Group".to_string()],
            audit_enabled: true,
            metadata: HashMap::new(),
        };

        self.clients.push(client);
        self
    }

    pub fn enable_scim_audit_log(mut self) -> Self {
        let mut audit_config = self.audit_config.unwrap_or_default();
        audit_config.enabled = true;
        self.audit_config = Some(audit_config);
        self
    }

    pub fn with_schema_extension(mut self, uri: &str, required: bool) -> Self {
        let mut schema_config = self.schema_config.unwrap_or_default();
        schema_config.extensions.push(ScimSchemaExtension {
            uri: uri.to_string(),
            enabled: true,
            required,
            attributes: HashMap::new(),
        });
        self.schema_config = Some(schema_config);
        self
    }

    pub fn build(self) -> Result<ScimTenantConfiguration, ScimConfigurationError> {
        let now = Utc::now();

        Ok(ScimTenantConfiguration {
            tenant_id: self.tenant_id,
            created_at: now,
            last_modified: now,
            version: 1,
            endpoint: self.endpoint.unwrap_or_default(),
            clients: self.clients,
            rate_limits: self.rate_limits.unwrap_or_default(),
            schema_config: self.schema_config.unwrap_or_default(),
            audit_config: self.audit_config.unwrap_or_default(),
            search_config: self.search_config.unwrap_or_default(),
        })
    }
}

// Custom serialization for Duration fields
mod duration_serde {
    use serde::{Deserialize, Deserializer, Serializer};
    use std::time::Duration;

    pub fn serialize<S>(duration: &Duration, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_u64(duration.as_secs())
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Duration, D::Error>
    where
        D: Deserializer<'de>,
    {
        let secs = u64::deserialize(deserializer)?;
        Ok(Duration::from_secs(secs))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scim_tenant_configuration_builder() {
        let config = ScimTenantConfiguration::builder("test-tenant".to_string())
            .with_endpoint_path("/scim/v2")
            .with_scim_rate_limit(100, Duration::from_secs(60))
            .with_scim_client("client-1", "api_key_123")
            .enable_scim_audit_log()
            .with_schema_extension(
                "urn:ietf:params:scim:schemas:extension:enterprise:2.0:User",
                false,
            )
            .build()
            .expect("Valid SCIM configuration");

        assert_eq!(config.tenant_id, "test-tenant");
        assert_eq!(config.endpoint.base_path, "/scim/v2");
        assert_eq!(config.clients.len(), 1);
        assert_eq!(config.clients[0].client_id, "client-1");
        assert!(config.audit_config.enabled);
        assert_eq!(config.schema_config.extensions.len(), 1);
    }

    #[test]
    fn test_rate_limit_checking() {
        let rate_limits = ScimRateLimits::default();

        // Test with default create limit (100 per minute)
        assert!(!rate_limits.check_create_limit(50));
        assert!(rate_limits.check_create_limit(100));
        assert!(rate_limits.check_create_limit(150));
    }

    #[test]
    fn test_client_config_lookup() {
        let config = ScimTenantConfiguration::builder("test-tenant".to_string())
            .with_scim_client("client-1", "api_key_123")
            .with_scim_client("client-2", "api_key_456")
            .build()
            .expect("Valid configuration");

        assert!(config.get_client_config("client-1").is_some());
        assert!(config.get_client_config("client-2").is_some());
        assert!(config.get_client_config("client-3").is_none());
    }

    #[test]
    fn test_schema_extension_checking() {
        let config = ScimTenantConfiguration::builder("test-tenant".to_string())
            .with_schema_extension(
                "urn:ietf:params:scim:schemas:extension:enterprise:2.0:User",
                true,
            )
            .build()
            .expect("Valid configuration");

        assert!(
            config
                .has_schema_extension("urn:ietf:params:scim:schemas:extension:enterprise:2.0:User")
        );
        assert!(!config.has_schema_extension("urn:example:custom:extension"));
    }

    #[test]
    fn test_default_configurations() {
        let endpoint = ScimEndpointConfig::default();
        assert_eq!(endpoint.base_path, "/scim/v2");
        assert_eq!(endpoint.scim_version, "2.0");

        let rate_limits = ScimRateLimits::default();
        assert!(rate_limits.create_operations.is_some());
        assert!(rate_limits.global_limit.is_some());

        let audit_config = ScimAuditConfig::default();
        assert!(audit_config.enabled);
        assert_eq!(audit_config.audited_operations.len(), 3);
    }
}
