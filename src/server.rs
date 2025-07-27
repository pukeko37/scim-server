//! Type-safe SCIM server implementation with state machine design.
//!
//! This module implements the core SCIM server using a type-parameterized state machine
//! to ensure compile-time safety and prevent invalid operations. The design follows
//! the builder pattern for configuration and provides async operations for all SCIM endpoints.

use crate::error::{BuildError, BuildResult, ScimResult};

use crate::schema::{Schema, SchemaRegistry};
use serde::{Deserialize, Serialize};

use std::marker::PhantomData;

/// State marker for uninitialized server.
///
/// This state prevents any SCIM operations until the server is properly configured.
#[derive(Debug)]
pub struct Uninitialized;

/// State marker for fully configured and ready server.
///
/// Only servers in this state can perform SCIM operations.
#[derive(Debug)]
pub struct Ready;

/// Type-safe SCIM server with state machine design.
///
/// The server uses phantom types to encode its configuration state at compile time,
/// preventing invalid operations and ensuring proper initialization sequence.
///
/// # Type Parameters
/// * `State` - The current state of the server (Uninitialized or Ready)
///
/// # Example
/// ```rust,no_run
/// use scim_server::TypeSafeScimServer;
///
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     // Create a basic type-safe SCIM server for schema discovery
///     let server = TypeSafeScimServer::new()?;
///
///     // Get available schemas
///     let schemas = server.get_schemas().await?;
///     println!("Available schemas: {}", schemas.len());
///
///     // For dynamic resource operations, use ScimServer instead
///     Ok(())
/// }
/// ```
pub struct ScimServer<State = Ready> {
    inner: Option<ServerInner>,
    _state: PhantomData<State>,
}

impl<State> std::fmt::Debug for ScimServer<State> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ScimServer")
            .field("inner", &self.inner.is_some())
            .field("state", &std::any::type_name::<State>())
            .finish()
    }
}

/// Internal server state shared across all server instances.
struct ServerInner {
    schema_registry: SchemaRegistry,
    service_config: ServiceProviderConfig,
}

impl std::fmt::Debug for ServerInner {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ServerInner")
            .field("schema_registry", &"SchemaRegistry")
            .field("service_config", &self.service_config)
            .finish()
    }
}

impl ScimServer<Uninitialized> {
    /// Create a new basic SCIM server.
    ///
    /// This creates a server with default configuration and schema registry.
    /// For dynamic resource operations, use DynamicScimServer.
    pub fn new() -> BuildResult<ScimServer<Ready>> {
        let schema_registry = SchemaRegistry::new().map_err(|_e| BuildError::SchemaLoadError {
            schema_id: "Core".to_string(),
        })?;

        let service_config = ServiceProviderConfig::default();

        let inner = ServerInner {
            schema_registry,
            service_config,
        };

        Ok(ScimServer {
            inner: Some(inner),
            _state: PhantomData,
        })
    }
}

impl ScimServer<Ready> {
    // Discovery endpoints

    /// Get all available schemas.
    ///
    /// Returns the complete list of schemas supported by this server instance.
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

    // User CRUD operations

    /// Get the schema registry for advanced usage.
    ///
    /// This provides access to the underlying schema registry for custom validation
    /// or schema introspection.
    pub fn schema_registry(&self) -> &SchemaRegistry {
        let inner = self.inner.as_ref().expect("Server should be initialized");
        &inner.schema_registry
    }

    /// Get the service provider configuration.
    pub fn service_config(&self) -> &ServiceProviderConfig {
        let inner = self.inner.as_ref().expect("Server should be initialized");
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
    async fn test_server_creation() {
        let server = ScimServer::new().expect("Failed to create server");

        // Test that the server can access schemas
        let schemas = server.get_schemas().await.expect("Failed to get schemas");
        assert!(!schemas.is_empty());
    }

    #[tokio::test]
    async fn test_schema_access() {
        let server = ScimServer::new().expect("Failed to create server");

        // Test schema retrieval
        let user_schema = server
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
}
