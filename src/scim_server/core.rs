//! Core SCIM server structure and initialization.
//!
//! This module contains the main ScimServer struct definition and its
//! constructor logic, representing the fundamental server structure
//! without specific operational concerns.

use crate::error::ScimError;
use crate::provider_capabilities::{
    CapabilityDiscovery, CapabilityIntrospectable, ProviderCapabilities,
};
use crate::resource::{ResourceHandler, ResourceProvider, ScimOperation};
use crate::schema::SchemaRegistry;
use crate::schema_discovery::ServiceProviderConfig;
use std::collections::HashMap;
use std::sync::Arc;

/// Dynamic SCIM server for handling SCIM protocol operations.
///
/// The server coordinates between storage providers and SCIM protocol requirements,
/// handling schema validation, resource lifecycle, and multi-tenant isolation.
/// Resource types are registered at runtime, allowing for flexible configurations.
///
/// # Type Parameters
///
/// * `P` - The resource provider type that implements [`ResourceProvider`]
///
/// # Examples
///
/// ```rust
/// use scim_server::{ScimServer, providers::StandardResourceProvider};
/// use scim_server::storage::InMemoryStorage;
/// use scim_server::resource::ScimOperation;
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let storage = InMemoryStorage::new();
/// let provider = StandardResourceProvider::new(storage);
/// let mut server = ScimServer::new(provider)?;
///
/// // Register resource types dynamically
/// // server.register_resource_type("User", handler, vec![ScimOperation::Create])?;
/// # Ok(())
/// # }
/// ```
pub struct ScimServer<P> {
    pub(super) provider: P,
    pub(super) schema_registry: SchemaRegistry,
    pub(super) resource_handlers: HashMap<String, Arc<ResourceHandler>>, // resource_type -> handler
    pub(super) supported_operations: HashMap<String, Vec<ScimOperation>>, // resource_type -> supported ops
}

impl<P: ResourceProvider> ScimServer<P> {
    /// Creates a new SCIM server with the given resource provider.
    ///
    /// Initializes the server with a schema registry containing standard SCIM schemas.
    /// Resource types must be registered separately using [`register_resource_type`].
    ///
    /// # Arguments
    ///
    /// * `provider` - The resource provider for storage operations
    ///
    /// # Errors
    ///
    /// Returns [`ScimError::Internal`] if the schema registry cannot be initialized.
    ///
    /// [`register_resource_type`]: Self::register_resource_type
    pub fn new(provider: P) -> Result<Self, ScimError> {
        let schema_registry = SchemaRegistry::new()
            .map_err(|e| ScimError::internal(format!("Failed to create schema registry: {}", e)))?;

        Ok(Self {
            provider,
            schema_registry,
            resource_handlers: HashMap::new(),
            supported_operations: HashMap::new(),
        })
    }

    /// Automatically discover provider capabilities from current server configuration
    ///
    /// This method introspects the registered resource types, schemas, and provider
    /// implementation to determine what capabilities are currently supported.
    pub fn discover_capabilities(&self) -> Result<ProviderCapabilities, ScimError> {
        CapabilityDiscovery::discover_capabilities(
            &self.schema_registry,
            &self.resource_handlers,
            &self.supported_operations,
            &self.provider,
        )
    }

    /// Discover capabilities with provider introspection
    ///
    /// This version works with providers that implement CapabilityIntrospectable
    /// to get provider-specific capability information like bulk limits and
    /// authentication schemes.
    pub fn discover_capabilities_with_introspection(
        &self,
    ) -> Result<ProviderCapabilities, ScimError>
    where
        P: CapabilityIntrospectable,
    {
        CapabilityDiscovery::discover_capabilities_with_introspection(
            &self.schema_registry,
            &self.resource_handlers,
            &self.supported_operations,
            &self.provider,
        )
    }

    /// Generate SCIM ServiceProviderConfig from discovered capabilities
    ///
    /// This automatically creates an RFC 7644 compliant ServiceProviderConfig
    /// that accurately reflects the current server capabilities.
    pub fn get_service_provider_config(&self) -> Result<ServiceProviderConfig, ScimError> {
        let capabilities = self.discover_capabilities()?;
        Ok(CapabilityDiscovery::generate_service_provider_config(
            &capabilities,
        ))
    }

    /// Generate ServiceProviderConfig with provider introspection
    ///
    /// This version works with providers that implement CapabilityIntrospectable
    /// for more detailed capability information.
    pub fn get_service_provider_config_with_introspection(
        &self,
    ) -> Result<ServiceProviderConfig, ScimError>
    where
        P: CapabilityIntrospectable,
    {
        let capabilities = self.discover_capabilities_with_introspection()?;
        Ok(CapabilityDiscovery::generate_service_provider_config(
            &capabilities,
        ))
    }

    /// Check if a specific operation is supported for a resource type
    pub fn supports_operation(&self, resource_type: &str, operation: &ScimOperation) -> bool {
        self.supported_operations
            .get(resource_type)
            .map(|ops| ops.contains(operation))
            .unwrap_or(false)
    }

    /// Get a reference to the underlying provider.
    ///
    /// This allows access to provider-specific functionality like conditional operations.
    pub fn provider(&self) -> &P {
        &self.provider
    }
}
