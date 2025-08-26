//! Schema discovery implementation with state machine design.
//!
//! This module implements a specialized SCIM component for schema discovery and service provider
//! configuration using a type-parameterized state machine to ensure compile-time safety.
//! This component is designed specifically for schema introspection, not for
//! resource CRUD operations. For full SCIM resource management, use ScimServer.

use crate::error::{BuildError, BuildResult, ScimResult};

use crate::schema::{Schema, SchemaRegistry};
use serde::{Deserialize, Serialize};

use std::marker::PhantomData;

/// State marker for uninitialized discovery component.
///
/// This state prevents any SCIM operations until the component is properly configured.
#[derive(Debug)]
pub struct Uninitialized;

/// State marker for fully configured and ready discovery component.
///
/// Only components in this state can perform SCIM operations.
#[derive(Debug)]
pub struct Ready;

/// Schema discovery component with state machine design.
///
/// The component uses phantom types to encode its configuration state at compile time,
/// preventing invalid operations and ensuring proper initialization sequence.
/// This component is specifically designed for schema discovery and service provider
/// configuration, not for resource CRUD operations.
///
/// # Type Parameters
/// * `State` - The current state of the component (Uninitialized or Ready)
///
/// # Example
/// ```rust,no_run
/// use scim_server::SchemaDiscovery;
///
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     // Create a schema discovery component
///     let discovery = SchemaDiscovery::new()?;
///
///     // Get available schemas
///     let schemas = discovery.get_schemas().await?;
///     println!("Available schemas: {}", schemas.len());
///
///     // For resource CRUD operations, use ScimServer instead
///     Ok(())
/// }
/// ```
pub struct SchemaDiscovery<State = Ready> {
    inner: Option<DiscoveryInner>,
    _state: PhantomData<State>,
}

impl<State> std::fmt::Debug for SchemaDiscovery<State> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SchemaDiscovery")
            .field("inner", &self.inner.is_some())
            .field("state", &std::any::type_name::<State>())
            .finish()
    }
}

/// Internal discovery state shared across all component instances.
struct DiscoveryInner {
    schema_registry: SchemaRegistry,
    service_config: ServiceProviderConfig,
}

impl std::fmt::Debug for DiscoveryInner {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DiscoveryInner")
            .field("schema_registry", &"SchemaRegistry")
            .field("service_config", &self.service_config)
            .finish()
    }
}

impl SchemaDiscovery<Uninitialized> {
    /// Create a new schema discovery component.
    ///
    /// This creates a component with default configuration and schema registry
    /// for schema discovery and service provider configuration.
    /// For resource CRUD operations, use ScimServer instead.
    pub fn new() -> BuildResult<SchemaDiscovery<Ready>> {
        let schema_registry =
            SchemaRegistry::with_embedded_schemas().map_err(|_e| BuildError::SchemaLoadError {
                schema_id: "Core".to_string(),
            })?;

        let service_config = ServiceProviderConfig::default();

        let inner = DiscoveryInner {
            schema_registry,
            service_config,
        };

        Ok(SchemaDiscovery {
            inner: Some(inner),
            _state: PhantomData,
        })
    }
}

impl SchemaDiscovery<Ready> {
    // Discovery endpoints

    /// Get all available schemas.
    ///
    /// Returns the complete list of schemas supported by this component instance.
    /// For the MVP, this includes only the core User schema.
    pub async fn get_schemas(&self) -> ScimResult<Vec<Schema>> {
        let inner = self.inner.as_ref().expect("Server should be initialized");
        Ok(inner
            .schema_registry
            .get_schemas()
            .into_iter()
            .cloned()
            .collect())
    }

    /// Get a specific schema by ID.
    ///
    /// # Arguments
    /// * `id` - The schema identifier (URI)
    ///
    /// # Returns
    /// * `Some(Schema)` if the schema exists
    /// * `None` if the schema is not found
    pub async fn get_schema(&self, id: &str) -> ScimResult<Option<Schema>> {
        let inner = self.inner.as_ref().expect("Server should be initialized");
        Ok(inner.schema_registry.get_schema(id).cloned())
    }

    /// Get the service provider configuration.
    ///
    /// Returns the capabilities and configuration of this SCIM service provider
    /// as defined in RFC 7644.
    pub async fn get_service_provider_config(&self) -> ScimResult<ServiceProviderConfig> {
        let inner = self.inner.as_ref().expect("Server should be initialized");
        Ok(inner.service_config.clone())
    }

    /// Get the schema registry for advanced usage.
    ///
    /// This provides access to the underlying schema registry for custom validation
    /// or schema introspection.
    pub fn schema_registry(&self) -> &SchemaRegistry {
        let inner = self
            .inner
            .as_ref()
            .expect("Discovery component should be initialized");
        &inner.schema_registry
    }

    /// Get the service provider configuration.
    pub fn service_config(&self) -> &ServiceProviderConfig {
        let inner = self
            .inner
            .as_ref()
            .expect("Discovery component should be initialized");
        &inner.service_config
    }
}

/// Service provider configuration as defined in RFC 7644.
///
/// This structure describes the capabilities and configuration of the SCIM service provider,
/// allowing clients to discover what features are supported.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ServiceProviderConfig {
    /// Whether PATCH operations are supported
    #[serde(rename = "patch")]
    pub patch_supported: bool,

    /// Whether bulk operations are supported
    #[serde(rename = "bulk")]
    pub bulk_supported: bool,

    /// Whether filtering is supported
    #[serde(rename = "filter")]
    pub filter_supported: bool,

    /// Whether password change operations are supported
    #[serde(rename = "changePassword")]
    pub change_password_supported: bool,

    /// Whether sorting is supported
    #[serde(rename = "sort")]
    pub sort_supported: bool,

    /// Whether ETags are supported for versioning
    #[serde(rename = "etag")]
    pub etag_supported: bool,

    /// Authentication schemes supported
    #[serde(rename = "authenticationSchemes")]
    pub authentication_schemes: Vec<AuthenticationScheme>,

    /// Maximum number of operations in a bulk request
    #[serde(rename = "bulk.maxOperations")]
    pub bulk_max_operations: Option<u32>,

    /// Maximum payload size for bulk operations
    #[serde(rename = "bulk.maxPayloadSize")]
    pub bulk_max_payload_size: Option<u64>,

    /// Maximum number of resources returned in a query
    #[serde(rename = "filter.maxResults")]
    pub filter_max_results: Option<u32>,
}

impl Default for ServiceProviderConfig {
    fn default() -> Self {
        Self {
            patch_supported: false,
            bulk_supported: false,
            filter_supported: false,
            change_password_supported: false,
            sort_supported: false,
            etag_supported: false,
            authentication_schemes: vec![],
            bulk_max_operations: None,
            bulk_max_payload_size: None,
            filter_max_results: Some(200),
        }
    }
}

/// Authentication scheme definition for service provider config.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AuthenticationScheme {
    /// Authentication scheme name
    pub name: String,
    /// Human-readable description
    pub description: String,
    /// URI for more information
    #[serde(rename = "specUri")]
    pub spec_uri: Option<String>,
    /// URI for documentation
    #[serde(rename = "documentationUri")]
    pub documentation_uri: Option<String>,
    /// Authentication type (e.g., "oauth2", "httpbasic")
    #[serde(rename = "type")]
    pub auth_type: String,
    /// Whether this scheme is the primary authentication method
    pub primary: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_discovery_creation() {
        let discovery = SchemaDiscovery::new().expect("Failed to create discovery component");

        // Test that the component can access schemas
        let schemas = discovery
            .get_schemas()
            .await
            .expect("Failed to get schemas");
        assert!(!schemas.is_empty());
    }

    #[tokio::test]
    async fn test_schema_access() {
        let discovery = SchemaDiscovery::new().expect("Failed to create discovery component");

        // Test schema retrieval
        let user_schema = discovery
            .get_schema("urn:ietf:params:scim:schemas:core:2.0:User")
            .await
            .expect("Failed to get schema");

        assert!(user_schema.is_some());
        if let Some(schema) = user_schema {
            assert_eq!(schema.name, "User");
        }
    }

    #[test]
    fn test_service_provider_config() {
        let config = ServiceProviderConfig::default();
        assert!(!config.patch_supported);
        assert!(!config.bulk_supported);
        assert!(!config.filter_supported);
    }

    #[tokio::test]
    async fn test_tutorial_example_works() {
        // This test verifies that the exact tutorial example from the documentation works
        // and addresses the critical issue found in schema-discovery-test-2025-08-15.md

        // The tutorial example should work without panicking
        let discovery = SchemaDiscovery::new()
            .expect("SchemaDiscovery::new() should work with embedded schemas");

        // Get available schemas
        let schemas = discovery
            .get_schemas()
            .await
            .expect("get_schemas() should work");
        assert!(
            !schemas.is_empty(),
            "Should have at least one schema available"
        );
        println!("Available schemas: {}", schemas.len());

        // Get service provider configuration
        let config = discovery
            .get_service_provider_config()
            .await
            .expect("get_service_provider_config() should work");
        println!("Bulk operations supported: {}", config.bulk_supported);

        // Verify we can access specific schemas
        let user_schema = discovery
            .get_schema("urn:ietf:params:scim:schemas:core:2.0:User")
            .await
            .expect("Should be able to get User schema");
        assert!(user_schema.is_some(), "User schema should be available");

        // This test confirms the fix for the critical SchemaLoadError issue
        // identified in the test results where SchemaDiscovery::new() was failing
    }
}
