//! Builder pattern for configuring SCIM server instances.
//!
//! This module provides a flexible builder pattern for creating SCIM servers
//! with different endpoint URL configurations and tenant handling strategies.
//! This is essential for proper $ref field generation in SCIM responses.

use crate::error::ScimError;
use crate::scim_server::ScimServer;
use crate::resource::ResourceProvider;

/// Strategy for handling tenant information in URLs.
///
/// Different SCIM clients and Identity Providers expect tenant information
/// to be represented in URLs in different ways. This enum supports the
/// most common patterns.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TenantStrategy {
    /// Single tenant mode - no tenant information in URLs.
    /// Example: `https://scim.example.com/v2/Users/123`
    SingleTenant,

    /// Tenant as subdomain.
    /// Example: `https://tenantA.scim.example.com/v2/Users/123`
    Subdomain,

    /// Tenant in URL path before SCIM version.
    /// Example: `https://scim.example.com/tenantA/v2/Users/123`
    PathBased,
}

impl Default for TenantStrategy {
    fn default() -> Self {
        TenantStrategy::SingleTenant
    }
}

/// Configuration for SCIM server endpoint URLs and tenant handling.
///
/// This configuration is used to generate proper $ref fields in SCIM
/// responses by combining the base URL, tenant strategy, and resource
/// information.
#[derive(Debug, Clone)]
pub struct ScimServerConfig {
    /// Base URL for the SCIM server (without tenant or path information).
    /// Examples: "https://scim.example.com", "https://api.company.com"
    pub base_url: String,

    /// Strategy for incorporating tenant information into URLs.
    pub tenant_strategy: TenantStrategy,

    /// SCIM protocol version to use in URLs. Defaults to "v2".
    pub scim_version: String,
}

impl Default for ScimServerConfig {
    fn default() -> Self {
        Self {
            base_url: "https://localhost".to_string(),
            tenant_strategy: TenantStrategy::SingleTenant,
            scim_version: "v2".to_string(),
        }
    }
}

impl ScimServerConfig {
    /// Generate a complete $ref URL for a resource.
    ///
    /// Combines the server configuration with tenant and resource information
    /// to create a properly formatted SCIM $ref URL.
    ///
    /// # Arguments
    ///
    /// * `tenant_id` - Optional tenant identifier from the request context
    /// * `resource_type` - SCIM resource type (e.g., "Users", "Groups")
    /// * `resource_id` - Unique identifier of the resource
    ///
    /// # Returns
    ///
    /// A complete $ref URL following SCIM 2.0 specification
    ///
    /// # Errors
    ///
    /// Returns an error if tenant information is required but missing
    pub fn generate_ref_url(
        &self,
        tenant_id: Option<&str>,
        resource_type: &str,
        resource_id: &str,
    ) -> Result<String, ScimError> {
        match &self.tenant_strategy {
            TenantStrategy::SingleTenant => {
                Ok(format!("{}/{}/{}/{}",
                    self.base_url,
                    self.scim_version,
                    resource_type,
                    resource_id
                ))
            },
            TenantStrategy::Subdomain => {
                let tenant = tenant_id.ok_or_else(|| {
                    ScimError::invalid_request("Tenant ID required for subdomain strategy but not provided")
                })?;

                // Extract domain from base URL and prepend tenant
                let url_without_protocol = self.base_url.strip_prefix("https://")
                    .or_else(|| self.base_url.strip_prefix("http://"))
                    .or_else(|| self.base_url.strip_prefix("mcp://"))
                    .ok_or_else(|| ScimError::internal("Invalid base URL format"))?;

                let protocol = if self.base_url.starts_with("https://") {
                    "https"
                } else if self.base_url.starts_with("http://") {
                    "http"
                } else {
                    "mcp"
                };

                Ok(format!("{}://{}.{}/{}/{}/{}",
                    protocol,
                    tenant,
                    url_without_protocol,
                    self.scim_version,
                    resource_type,
                    resource_id
                ))
            },
            TenantStrategy::PathBased => {
                let tenant = tenant_id.ok_or_else(|| {
                    ScimError::invalid_request("Tenant ID required for path-based strategy but not provided")
                })?;

                Ok(format!("{}/{}/{}/{}/{}",
                    self.base_url,
                    tenant,
                    self.scim_version,
                    resource_type,
                    resource_id
                ))
            },
        }
    }

    /// Validate the configuration.
    ///
    /// Ensures the base URL and other configuration parameters are valid.
    pub fn validate(&self) -> Result<(), ScimError> {
        if self.base_url.is_empty() {
            return Err(ScimError::internal("Base URL cannot be empty"));
        }

        if !self.base_url.starts_with("http://") && !self.base_url.starts_with("https://") && !self.base_url.starts_with("mcp://") {
            return Err(ScimError::internal("Base URL must start with http://, https://, or mcp://"));
        }

        if self.scim_version.is_empty() {
            return Err(ScimError::internal("SCIM version cannot be empty"));
        }

        Ok(())
    }
}

/// Builder for configuring and creating SCIM server instances.
///
/// Provides a fluent API for setting up endpoint URLs and tenant handling
/// strategies before creating the final `ScimServer` instance.
///
/// # Examples
///
/// ```rust
/// use scim_server::ScimServerBuilder;
/// use scim_server::TenantStrategy;
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// # let provider = scim_server::providers::StandardResourceProvider::new(
/// #     scim_server::storage::InMemoryStorage::new()
/// # );
///
/// // Single tenant server
/// let server = ScimServerBuilder::new(provider.clone())
///     .with_base_url("https://scim.company.com")
///     .build()?;
///
/// // Multi-tenant with subdomains
/// let server = ScimServerBuilder::new(provider.clone())
///     .with_base_url("https://scim.company.com")
///     .with_tenant_strategy(TenantStrategy::Subdomain)
///     .build()?;
///
/// // Multi-tenant with path-based tenants
/// let server = ScimServerBuilder::new(provider)
///     .with_base_url("https://api.company.com")
///     .with_tenant_strategy(TenantStrategy::PathBased)
///     .with_scim_version("v2.1")
///     .build()?;
/// # Ok(())
/// # }
/// ```
pub struct ScimServerBuilder<P> {
    provider: P,
    config: ScimServerConfig,
}

impl<P: ResourceProvider> ScimServerBuilder<P> {
    /// Create a new SCIM server builder with a resource provider.
    ///
    /// Starts with default configuration (single tenant, localhost base URL).
    pub fn new(provider: P) -> Self {
        Self {
            provider,
            config: ScimServerConfig::default(),
        }
    }

    /// Set the base URL for the SCIM server.
    ///
    /// This should be the root URL without any tenant or SCIM path information.
    ///
    /// # Examples
    ///
    /// - `"https://scim.company.com"`
    /// - `"https://api.company.com"`
    /// - `"http://localhost:8080"`
    pub fn with_base_url(mut self, base_url: impl Into<String>) -> Self {
        self.config.base_url = base_url.into();
        self
    }

    /// Set the tenant handling strategy.
    ///
    /// Determines how tenant information from request contexts is incorporated
    /// into generated $ref URLs.
    pub fn with_tenant_strategy(mut self, strategy: TenantStrategy) -> Self {
        self.config.tenant_strategy = strategy;
        self
    }

    /// Set the SCIM protocol version to use in URLs.
    ///
    /// Defaults to "v2" if not specified.
    pub fn with_scim_version(mut self, version: impl Into<String>) -> Self {
        self.config.scim_version = version.into();
        self
    }

    /// Build the configured SCIM server.
    ///
    /// Validates the configuration and creates the final `ScimServer` instance.
    ///
    /// # Errors
    ///
    /// Returns a `ScimError` if the configuration is invalid or if server
    /// initialization fails.
    pub fn build(self) -> Result<ScimServer<P>, ScimError> {
        self.config.validate()?;
        ScimServer::with_config(self.provider, self.config)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_single_tenant_ref_url_generation() {
        let config = ScimServerConfig {
            base_url: "https://scim.example.com".to_string(),
            tenant_strategy: TenantStrategy::SingleTenant,
            scim_version: "v2".to_string(),
        };

        let url = config.generate_ref_url(None, "Users", "12345").unwrap();
        assert_eq!(url, "https://scim.example.com/v2/Users/12345");
    }

    #[test]
    fn test_subdomain_tenant_ref_url_generation() {
        let config = ScimServerConfig {
            base_url: "https://scim.example.com".to_string(),
            tenant_strategy: TenantStrategy::Subdomain,
            scim_version: "v2".to_string(),
        };

        let url = config.generate_ref_url(Some("acme"), "Groups", "67890").unwrap();
        assert_eq!(url, "https://acme.scim.example.com/v2/Groups/67890");
    }

    #[test]
    fn test_path_based_tenant_ref_url_generation() {
        let config = ScimServerConfig {
            base_url: "https://api.company.com".to_string(),
            tenant_strategy: TenantStrategy::PathBased,
            scim_version: "v2".to_string(),
        };

        let url = config.generate_ref_url(Some("tenant1"), "Users", "abc123").unwrap();
        assert_eq!(url, "https://api.company.com/tenant1/v2/Users/abc123");
    }

    #[test]
    fn test_missing_tenant_error() {
        let config = ScimServerConfig {
            base_url: "https://scim.example.com".to_string(),
            tenant_strategy: TenantStrategy::Subdomain,
            scim_version: "v2".to_string(),
        };

        let result = config.generate_ref_url(None, "Users", "12345");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Tenant ID required"));
    }

    #[test]
    fn test_config_validation() {
        let mut config = ScimServerConfig::default();
        assert!(config.validate().is_ok());

        config.base_url = "".to_string();
        assert!(config.validate().is_err());

        config.base_url = "invalid-url".to_string();
        assert!(config.validate().is_err());

        config.base_url = "https://valid.com".to_string();
        config.scim_version = "".to_string();
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_builder_pattern() {
        // This is a compile-time test to ensure the builder API works
        fn _test_builder_compiles() {
            use crate::providers::StandardResourceProvider;
            use crate::storage::InMemoryStorage;

            let storage = InMemoryStorage::new();
            let provider = StandardResourceProvider::new(storage);

            let _builder = ScimServerBuilder::new(provider)
                .with_base_url("https://test.com")
                .with_tenant_strategy(TenantStrategy::PathBased)
                .with_scim_version("v2.1");
        }
    }
}
