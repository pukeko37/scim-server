//! Automated Provider Capability Discovery System
//!
//! This module provides automatic discovery of SCIM provider capabilities by introspecting
//! the current server configuration, registered resource types, schemas, and provider
//! implementation. This eliminates manual capability configuration and ensures that
//! the ServiceProviderConfig always accurately reflects the actual server capabilities.
//!
//! # Key Features
//!
//! * **Automatic Discovery**: Capabilities are derived from registered components
//! * **SCIM Compliance**: Generates RFC 7644 compliant ServiceProviderConfig
//! * **Type Safety**: Leverages Rust's type system for capability constraints
//! * **Real-time Updates**: Capabilities reflect current server state
//! * **Mandatory ETag Support**: All providers automatically support conditional operations
//!
//! # Discovery Sources
//!
//! * **Schemas**: From SchemaRegistry - determines supported resource types
//! * **Operations**: From registered resource handlers - determines CRUD capabilities
//! * **Provider Type**: From ResourceProvider implementation - determines advanced features
//! * **Attribute Metadata**: From schema definitions - determines filtering capabilities
//! * **ETag Versioning**: Always enabled - conditional operations are mandatory for all providers

use crate::error::ScimError;
use crate::providers::ResourceProvider;
use crate::resource::ScimOperation;
use crate::schema::{AttributeDefinition, SchemaRegistry};
use crate::schema_discovery::{AuthenticationScheme, ServiceProviderConfig};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

/// Comprehensive provider capability information automatically discovered
/// from the current server configuration and registered components.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderCapabilities {
    /// SCIM operations supported per resource type
    pub supported_operations: HashMap<String, Vec<ScimOperation>>,

    /// All schemas currently registered and available
    pub supported_schemas: Vec<String>,

    /// Resource types that can be managed
    pub supported_resource_types: Vec<String>,

    /// Bulk operation capabilities
    pub bulk_capabilities: BulkCapabilities,

    /// Filtering and query capabilities
    pub filter_capabilities: FilterCapabilities,

    /// Pagination support information
    pub pagination_capabilities: PaginationCapabilities,

    /// Authentication schemes available
    pub authentication_capabilities: AuthenticationCapabilities,

    /// Provider-specific extended capabilities
    pub extended_capabilities: ExtendedCapabilities,
}

/// Bulk operation support information discovered from provider implementation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BulkCapabilities {
    /// Whether bulk operations are supported at all
    pub supported: bool,

    /// Maximum number of operations in a single bulk request
    pub max_operations: Option<usize>,

    /// Maximum payload size for bulk requests in bytes
    pub max_payload_size: Option<usize>,

    /// Whether bulk operations support failOnErrors
    pub fail_on_errors_supported: bool,
}

/// Filtering capabilities discovered from schema attribute definitions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilterCapabilities {
    /// Whether filtering is supported
    pub supported: bool,

    /// Maximum number of results that can be returned
    pub max_results: Option<usize>,

    /// Attributes that support filtering (derived from schema)
    pub filterable_attributes: HashMap<String, Vec<String>>, // resource_type -> [attribute_names]

    /// Supported filter operators
    pub supported_operators: Vec<FilterOperator>,

    /// Whether complex filters with AND/OR are supported
    pub complex_filters_supported: bool,
}

/// Pagination support capabilities
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaginationCapabilities {
    /// Whether pagination is supported
    pub supported: bool,

    /// Default page size
    pub default_page_size: Option<usize>,

    /// Maximum page size allowed
    pub max_page_size: Option<usize>,

    /// Whether cursor-based pagination is supported
    pub cursor_based_supported: bool,
}

/// Authentication capabilities (typically configured rather than discovered)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthenticationCapabilities {
    /// Supported authentication schemes
    pub schemes: Vec<AuthenticationScheme>,

    /// Whether multi-factor authentication is supported
    pub mfa_supported: bool,

    /// Whether token refresh is supported
    pub token_refresh_supported: bool,
}

/// Extended capabilities specific to the provider implementation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtendedCapabilities {
    /// Whether ETag versioning is supported (always true - conditional operations are mandatory)
    pub etag_supported: bool,

    /// Whether PATCH operations are supported
    pub patch_supported: bool,

    /// Whether password change operations are supported
    pub change_password_supported: bool,

    /// Whether sorting is supported
    pub sort_supported: bool,

    /// Custom provider-specific capabilities
    pub custom_capabilities: HashMap<String, serde_json::Value>,
}

/// SCIM filter operators that can be supported
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum FilterOperator {
    /// Equal comparison
    #[serde(rename = "eq")]
    Equal,

    /// Not equal comparison
    #[serde(rename = "ne")]
    NotEqual,

    /// Contains operation for strings
    #[serde(rename = "co")]
    Contains,

    /// Starts with operation for strings
    #[serde(rename = "sw")]
    StartsWith,

    /// Ends with operation for strings
    #[serde(rename = "ew")]
    EndsWith,

    /// Present (attribute exists)
    #[serde(rename = "pr")]
    Present,

    /// Greater than
    #[serde(rename = "gt")]
    GreaterThan,

    /// Greater than or equal
    #[serde(rename = "ge")]
    GreaterThanOrEqual,

    /// Less than
    #[serde(rename = "lt")]
    LessThan,

    /// Less than or equal
    #[serde(rename = "le")]
    LessThanOrEqual,
}

/// Trait for providers that support capability introspection
pub trait CapabilityIntrospectable {
    /// Get provider-specific capability information that cannot be auto-discovered
    fn get_provider_specific_capabilities(&self) -> ExtendedCapabilities {
        ExtendedCapabilities::default()
    }

    /// Get bulk operation limits from the provider
    fn get_bulk_limits(&self) -> Option<BulkCapabilities> {
        None
    }

    /// Get pagination limits from the provider
    fn get_pagination_limits(&self) -> Option<PaginationCapabilities> {
        None
    }

    /// Get authentication capabilities (usually configured)
    fn get_authentication_capabilities(&self) -> Option<AuthenticationCapabilities> {
        None
    }
}

/// Automatic capability discovery engine that introspects server configuration
pub struct CapabilityDiscovery;

impl CapabilityDiscovery {
    /// Discover all provider capabilities from the current server state
    ///
    /// This method introspects the registered resource types, schemas, and provider
    /// implementation to automatically determine what capabilities are supported.
    pub fn discover_capabilities<P>(
        schema_registry: &SchemaRegistry,
        resource_handlers: &HashMap<String, std::sync::Arc<crate::resource::ResourceHandler>>,
        supported_operations: &HashMap<String, Vec<ScimOperation>>,
        _provider: &P,
    ) -> Result<ProviderCapabilities, ScimError>
    where
        P: ResourceProvider,
    {
        // Discover supported schemas from registry
        let supported_schemas = Self::discover_schemas(schema_registry);

        // Discover resource types from registered handlers
        let supported_resource_types = Self::discover_resource_types(resource_handlers);

        // Copy operation support directly from registration
        let supported_operations_map = supported_operations.clone();

        // Discover filtering capabilities from schema attributes
        let filter_capabilities =
            Self::discover_filter_capabilities(schema_registry, resource_handlers)?;

        // Use default capabilities for basic providers
        let bulk_capabilities = Self::default_bulk_capabilities();
        let pagination_capabilities = Self::default_pagination_capabilities();
        let authentication_capabilities = Self::default_authentication_capabilities();
        let mut extended_capabilities = ExtendedCapabilities::default();

        // Ensure ETag support is always enabled (conditional operations are mandatory)
        extended_capabilities.etag_supported = true;

        // Detect patch support from registered operations
        extended_capabilities.patch_supported = supported_operations
            .values()
            .any(|ops| ops.contains(&ScimOperation::Patch));

        Ok(ProviderCapabilities {
            supported_operations: supported_operations_map,
            supported_schemas,
            supported_resource_types,
            bulk_capabilities,
            filter_capabilities,
            pagination_capabilities,
            authentication_capabilities,
            extended_capabilities,
        })
    }

    /// Discover capabilities with provider introspection
    ///
    /// This version works with providers that implement CapabilityIntrospectable
    /// to get provider-specific capability information.
    pub fn discover_capabilities_with_introspection<P>(
        schema_registry: &SchemaRegistry,
        resource_handlers: &HashMap<String, std::sync::Arc<crate::resource::ResourceHandler>>,
        supported_operations: &HashMap<String, Vec<ScimOperation>>,
        provider: &P,
    ) -> Result<ProviderCapabilities, ScimError>
    where
        P: ResourceProvider + CapabilityIntrospectable,
    {
        // Discover supported schemas from registry
        let supported_schemas = Self::discover_schemas(schema_registry);

        // Discover resource types from registered handlers
        let supported_resource_types = Self::discover_resource_types(resource_handlers);

        // Copy operation support directly from registration
        let supported_operations_map = supported_operations.clone();

        // Discover filtering capabilities from schema attributes
        let filter_capabilities =
            Self::discover_filter_capabilities(schema_registry, resource_handlers)?;

        // Get provider-specific capabilities
        let bulk_capabilities = provider
            .get_bulk_limits()
            .unwrap_or_else(|| Self::default_bulk_capabilities());

        let pagination_capabilities = provider
            .get_pagination_limits()
            .unwrap_or_else(|| Self::default_pagination_capabilities());

        let authentication_capabilities = provider
            .get_authentication_capabilities()
            .unwrap_or_else(|| Self::default_authentication_capabilities());

        let extended_capabilities = provider.get_provider_specific_capabilities();

        Ok(ProviderCapabilities {
            supported_operations: supported_operations_map,
            supported_schemas,
            supported_resource_types,
            bulk_capabilities,
            filter_capabilities,
            pagination_capabilities,
            authentication_capabilities,
            extended_capabilities,
        })
    }

    /// Discover all registered schemas
    fn discover_schemas(schema_registry: &SchemaRegistry) -> Vec<String> {
        schema_registry
            .get_schemas()
            .iter()
            .map(|schema| schema.id.clone())
            .collect()
    }

    /// Discover registered resource types
    fn discover_resource_types(
        resource_handlers: &HashMap<String, std::sync::Arc<crate::resource::ResourceHandler>>,
    ) -> Vec<String> {
        resource_handlers.keys().cloned().collect()
    }

    /// Discover filtering capabilities from schema attribute definitions
    fn discover_filter_capabilities(
        schema_registry: &SchemaRegistry,
        resource_handlers: &HashMap<String, std::sync::Arc<crate::resource::ResourceHandler>>,
    ) -> Result<FilterCapabilities, ScimError> {
        let mut filterable_attributes = HashMap::new();

        // For each resource type, discover which attributes can be filtered
        for (resource_type, handler) in resource_handlers {
            // Get schema for this resource type
            if let Some(schema) = schema_registry.get_schema(&handler.schema.id) {
                // Recursively collect all filterable attributes including sub-attributes
                let attrs = Self::collect_filterable_attributes(&schema.attributes, "");
                filterable_attributes.insert(resource_type.clone(), attrs);
            }
        }

        // Determine supported operators based on attribute types
        let supported_operators = Self::determine_supported_operators(schema_registry);

        Ok(FilterCapabilities {
            supported: !filterable_attributes.is_empty(),
            max_results: Some(200), // Default SCIM recommendation
            filterable_attributes,
            supported_operators,
            complex_filters_supported: true, // Most implementations support AND/OR
        })
    }

    /// Determine if an attribute can be used in filters
    fn is_attribute_filterable(attr: &AttributeDefinition) -> bool {
        // Most simple attributes are filterable
        // Complex attributes and some special cases may not be
        match attr.data_type {
            crate::schema::AttributeType::Complex => false, // Complex attributes typically not directly filterable
            _ => true, // String, boolean, integer, decimal, dateTime, binary, reference are filterable
        }
    }

    /// Recursively collect filterable attributes from a schema
    fn collect_filterable_attributes(
        attributes: &[AttributeDefinition],
        prefix: &str,
    ) -> Vec<String> {
        let mut filterable = Vec::new();

        for attr in attributes {
            let attr_name = if prefix.is_empty() {
                attr.name.clone()
            } else {
                format!("{}.{}", prefix, attr.name)
            };

            if Self::is_attribute_filterable(attr) {
                filterable.push(attr_name.clone());
            }

            // Recursively check sub-attributes
            if !attr.sub_attributes.is_empty() {
                filterable.extend(Self::collect_filterable_attributes(
                    &attr.sub_attributes,
                    &attr_name,
                ));
            }
        }

        filterable
    }

    /// Determine which filter operators are supported based on schema attribute types
    fn determine_supported_operators(schema_registry: &SchemaRegistry) -> Vec<FilterOperator> {
        let mut operators = HashSet::new();

        // Basic operators always supported
        operators.insert(FilterOperator::Equal);
        operators.insert(FilterOperator::NotEqual);
        operators.insert(FilterOperator::Present);

        // Check if we have string attributes (enables string operations)
        if Self::has_string_attributes(schema_registry) {
            operators.insert(FilterOperator::Contains);
            operators.insert(FilterOperator::StartsWith);
            operators.insert(FilterOperator::EndsWith);
        }

        // Check if we have numeric/date attributes (enables comparison operations)
        if Self::has_comparable_attributes(schema_registry) {
            operators.insert(FilterOperator::GreaterThan);
            operators.insert(FilterOperator::GreaterThanOrEqual);
            operators.insert(FilterOperator::LessThan);
            operators.insert(FilterOperator::LessThanOrEqual);
        }

        operators.into_iter().collect()
    }

    /// Check if any registered schemas have string attributes
    fn has_string_attributes(schema_registry: &SchemaRegistry) -> bool {
        fn has_string_in_attributes(attributes: &[AttributeDefinition]) -> bool {
            attributes.iter().any(|attr| {
                matches!(attr.data_type, crate::schema::AttributeType::String)
                    || has_string_in_attributes(&attr.sub_attributes)
            })
        }

        schema_registry
            .get_schemas()
            .iter()
            .any(|schema| has_string_in_attributes(&schema.attributes))
    }

    /// Check if any registered schemas have comparable attributes (numeric, date)
    fn has_comparable_attributes(schema_registry: &SchemaRegistry) -> bool {
        fn has_comparable_in_attributes(attributes: &[AttributeDefinition]) -> bool {
            attributes.iter().any(|attr| {
                matches!(
                    attr.data_type,
                    crate::schema::AttributeType::Integer
                        | crate::schema::AttributeType::Decimal
                        | crate::schema::AttributeType::DateTime
                ) || has_comparable_in_attributes(&attr.sub_attributes)
            })
        }

        schema_registry
            .get_schemas()
            .iter()
            .any(|schema| has_comparable_in_attributes(&schema.attributes))
    }

    /// Default bulk capabilities for providers that don't specify them
    fn default_bulk_capabilities() -> BulkCapabilities {
        BulkCapabilities {
            supported: false, // Conservative default
            max_operations: None,
            max_payload_size: None,
            fail_on_errors_supported: false,
        }
    }

    /// Default pagination capabilities
    fn default_pagination_capabilities() -> PaginationCapabilities {
        PaginationCapabilities {
            supported: true, // Most providers support basic pagination
            default_page_size: Some(20),
            max_page_size: Some(200),
            cursor_based_supported: false, // Conservative default
        }
    }

    /// Default authentication capabilities
    fn default_authentication_capabilities() -> AuthenticationCapabilities {
        AuthenticationCapabilities {
            schemes: vec![], // Must be explicitly configured
            mfa_supported: false,
            token_refresh_supported: false,
        }
    }

    /// Generate RFC 7644 compliant ServiceProviderConfig from discovered capabilities
    pub fn generate_service_provider_config(
        capabilities: &ProviderCapabilities,
    ) -> ServiceProviderConfig {
        ServiceProviderConfig {
            patch_supported: capabilities.extended_capabilities.patch_supported,
            bulk_supported: capabilities.bulk_capabilities.supported,
            filter_supported: capabilities.filter_capabilities.supported,
            change_password_supported: capabilities.extended_capabilities.change_password_supported,
            sort_supported: capabilities.extended_capabilities.sort_supported,
            etag_supported: capabilities.extended_capabilities.etag_supported,
            authentication_schemes: capabilities.authentication_capabilities.schemes.clone(),
            bulk_max_operations: capabilities
                .bulk_capabilities
                .max_operations
                .map(|n| n as u32),
            bulk_max_payload_size: capabilities
                .bulk_capabilities
                .max_payload_size
                .map(|n| n as u64),
            filter_max_results: capabilities
                .filter_capabilities
                .max_results
                .map(|n| n as u32),
        }
    }
}

impl Default for BulkCapabilities {
    fn default() -> Self {
        Self {
            supported: false,
            max_operations: None,
            max_payload_size: None,
            fail_on_errors_supported: false,
        }
    }
}

impl Default for FilterCapabilities {
    fn default() -> Self {
        Self {
            supported: false,
            max_results: Some(200),
            filterable_attributes: HashMap::new(),
            supported_operators: vec![FilterOperator::Equal, FilterOperator::Present],
            complex_filters_supported: false,
        }
    }
}

impl Default for PaginationCapabilities {
    fn default() -> Self {
        Self {
            supported: true,
            default_page_size: Some(20),
            max_page_size: Some(200),
            cursor_based_supported: false,
        }
    }
}

impl Default for AuthenticationCapabilities {
    fn default() -> Self {
        Self {
            schemes: vec![],
            mfa_supported: false,
            token_refresh_supported: false,
        }
    }
}

impl Default for ExtendedCapabilities {
    fn default() -> Self {
        Self {
            etag_supported: true, // Always true - conditional operations are mandatory
            patch_supported: false,
            change_password_supported: false,
            sort_supported: false,
            custom_capabilities: HashMap::new(),
        }
    }
}

// Default implementation can be provided via a blanket impl, but users can override
// by implementing the trait directly on their provider types

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::SchemaRegistry;
    use std::collections::HashMap;

    #[test]
    fn test_discover_schemas() {
        let registry = SchemaRegistry::new().expect("Failed to create schema registry");
        let schemas = CapabilityDiscovery::discover_schemas(&registry);

        assert!(!schemas.is_empty());
        assert!(schemas.contains(&"urn:ietf:params:scim:schemas:core:2.0:User".to_string()));
    }

    #[test]
    fn test_has_string_attributes() {
        let registry = SchemaRegistry::new().expect("Failed to create schema registry");
        assert!(CapabilityDiscovery::has_string_attributes(&registry));
    }

    #[test]
    fn test_has_comparable_attributes() {
        let registry = SchemaRegistry::new().expect("Failed to create schema registry");
        assert!(CapabilityDiscovery::has_comparable_attributes(&registry));
    }

    #[test]
    fn test_service_provider_config_generation() {
        let capabilities = ProviderCapabilities {
            supported_operations: HashMap::new(),
            supported_schemas: vec!["urn:ietf:params:scim:schemas:core:2.0:User".to_string()],
            supported_resource_types: vec!["User".to_string()],
            bulk_capabilities: BulkCapabilities {
                supported: true,
                max_operations: Some(100),
                max_payload_size: Some(1024 * 1024),
                fail_on_errors_supported: true,
            },
            filter_capabilities: FilterCapabilities::default(),
            pagination_capabilities: PaginationCapabilities::default(),
            authentication_capabilities: AuthenticationCapabilities::default(),
            extended_capabilities: ExtendedCapabilities {
                patch_supported: true,
                ..Default::default()
            },
        };

        let config = CapabilityDiscovery::generate_service_provider_config(&capabilities);

        assert!(config.bulk_supported);
        assert!(config.patch_supported);
        assert_eq!(config.bulk_max_operations, Some(100));
        assert_eq!(config.bulk_max_payload_size, Some(1024 * 1024));
    }

    #[test]
    fn test_filter_operators() {
        let registry = SchemaRegistry::new().expect("Failed to create schema registry");
        let operators = CapabilityDiscovery::determine_supported_operators(&registry);

        log::debug!("Discovered filter operators: {:?}", operators);

        // Should have basic operators
        assert!(operators.contains(&FilterOperator::Equal));
        assert!(operators.contains(&FilterOperator::Present));

        // Should have string operators since User schema has string attributes
        assert!(operators.contains(&FilterOperator::Contains));
        assert!(operators.contains(&FilterOperator::StartsWith));

        // Should have comparison operators since User schema has dateTime attributes (in sub-attributes)
        assert!(operators.contains(&FilterOperator::GreaterThan));
        assert!(operators.contains(&FilterOperator::LessThan));
    }
}
