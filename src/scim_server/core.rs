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
use crate::scim_server::builder::ScimServerConfig;
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
    pub(super) config: ScimServerConfig,
}

impl<P: ResourceProvider> ScimServer<P> {
    /// Creates a new SCIM server with the given resource provider.
    ///
    /// Initializes the server with a schema registry containing standard SCIM schemas
    /// and default configuration (single tenant, localhost base URL).
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
        Self::with_config(provider, ScimServerConfig::default())
    }

    /// Creates a new SCIM server with the given resource provider and configuration.
    ///
    /// This is the primary constructor used by the builder pattern, but can also
    /// be used directly for more advanced configuration scenarios.
    ///
    /// # Arguments
    ///
    /// * `provider` - The resource provider for storage operations
    /// * `config` - Server configuration for URL generation and tenant handling
    ///
    /// # Errors
    ///
    /// Returns [`ScimError::Internal`] if the schema registry cannot be initialized.
    pub fn with_config(provider: P, config: ScimServerConfig) -> Result<Self, ScimError> {
        let schema_registry = SchemaRegistry::new()
            .map_err(|e| ScimError::internal(format!("Failed to create schema registry: {}", e)))?;

        Ok(Self {
            provider,
            schema_registry,
            resource_handlers: HashMap::new(),
            supported_operations: HashMap::new(),
            config,
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

    /// Get a reference to the server configuration.
    ///
    /// This allows access to URL generation and tenant handling configuration.
    pub fn config(&self) -> &ScimServerConfig {
        &self.config
    }

    /// Generate a $ref URL for a resource.
    ///
    /// Combines server configuration with tenant and resource information
    /// to create properly formatted SCIM $ref URLs.
    ///
    /// # Arguments
    ///
    /// * `tenant_id` - Optional tenant identifier from request context
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
        self.config
            .generate_ref_url(tenant_id, resource_type, resource_id)
    }

    /// Inject $ref fields into resource JSON for SCIM compliance.
    ///
    /// This method post-processes resource JSON to add proper $ref fields
    /// to Group.members and User.groups arrays based on server configuration
    /// and tenant context.
    ///
    /// # Arguments
    ///
    /// * `resource_json` - Mutable JSON object representing the resource
    /// * `tenant_id` - Optional tenant identifier from request context
    ///
    /// # Errors
    ///
    /// Returns an error if $ref URL generation fails due to missing tenant information
    pub fn inject_ref_fields(
        &self,
        resource_json: &mut serde_json::Value,
        tenant_id: Option<&str>,
    ) -> Result<(), ScimError> {
        // Handle Group.members array
        if let Some(members_array) = resource_json.get_mut("members") {
            if let Some(members) = members_array.as_array_mut() {
                for member in members {
                    if let Some(member_obj) = member.as_object_mut() {
                        if let (Some(member_id), Some(member_type)) = (
                            member_obj.get("value").and_then(|v| v.as_str()),
                            member_obj.get("type").and_then(|v| v.as_str()),
                        ) {
                            // Determine resource type for $ref URL
                            let resource_type = match member_type {
                                "User" => "Users",
                                "Group" => "Groups",
                                _ => member_type, // Use as-is for unknown types
                            };

                            let ref_url =
                                self.generate_ref_url(tenant_id, resource_type, member_id)?;
                            member_obj
                                .insert("$ref".to_string(), serde_json::Value::String(ref_url));
                        }
                    }
                }
            }
        }

        // Handle User.groups array
        if let Some(groups_array) = resource_json.get_mut("groups") {
            if let Some(groups) = groups_array.as_array_mut() {
                for group in groups {
                    if let Some(group_obj) = group.as_object_mut() {
                        if let Some(group_id) = group_obj.get("value").and_then(|v| v.as_str()) {
                            let ref_url = self.generate_ref_url(tenant_id, "Groups", group_id)?;
                            group_obj
                                .insert("$ref".to_string(), serde_json::Value::String(ref_url));
                        }
                    }
                }
            }
        }

        Ok(())
    }

    /// Inject proper location field into resource metadata based on server configuration.
    ///
    /// This method updates the meta.location field to use the server's configured
    /// base URL instead of any hardcoded URLs that may have been set during
    /// resource creation by the provider.
    ///
    /// # Arguments
    ///
    /// * `resource_json` - Mutable JSON object representing the resource
    /// * `tenant_id` - Optional tenant identifier from request context
    ///
    /// # Errors
    ///
    /// Returns an error if location URL generation fails due to missing tenant information
    pub fn inject_location_field(
        &self,
        resource_json: &mut serde_json::Value,
        tenant_id: Option<&str>,
    ) -> Result<(), ScimError> {
        // Extract resource_id first to avoid borrowing conflicts
        let resource_id = resource_json
            .get("id")
            .and_then(|id| id.as_str())
            .map(|s| s.to_string());

        if let Some(meta_obj) = resource_json
            .get_mut("meta")
            .and_then(|m| m.as_object_mut())
        {
            if let (Some(resource_type), Some(resource_id)) = (
                meta_obj.get("resourceType").and_then(|rt| rt.as_str()),
                resource_id.as_deref(),
            ) {
                // Generate proper location URL using server configuration
                let resource_type_plural = match resource_type {
                    "User" => "Users",
                    "Group" => "Groups",
                    _ => resource_type, // Use as-is for unknown types
                };

                let location_url =
                    self.generate_ref_url(tenant_id, resource_type_plural, resource_id)?;
                meta_obj.insert(
                    "location".to_string(),
                    serde_json::Value::String(location_url),
                );
            }
        }
        Ok(())
    }

    /// Serialize a resource with proper $ref fields and location URL injected.
    ///
    /// This method combines `Resource::to_json()` with $ref field injection and
    /// location URL correction to ensure SCIM 2.0 compliance for resource references
    /// and proper server configuration usage.
    ///
    /// # Arguments
    ///
    /// * `resource` - The resource to serialize
    /// * `tenant_id` - Optional tenant identifier from request context
    ///
    /// # Returns
    ///
    /// JSON representation of the resource with proper $ref fields and location URL
    pub fn serialize_resource_with_refs(
        &self,
        resource: &crate::resource::Resource,
        tenant_id: Option<&str>,
    ) -> Result<serde_json::Value, ScimError> {
        let mut json = resource
            .to_json()
            .map_err(|e| ScimError::internal(format!("Failed to serialize resource: {}", e)))?;

        self.inject_ref_fields(&mut json, tenant_id)?;
        self.inject_location_field(&mut json, tenant_id)?;
        Ok(json)
    }
}
