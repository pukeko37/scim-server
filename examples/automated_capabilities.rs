//! Automated Provider Capability Discovery Example
//!
//! This example demonstrates how the SCIM server automatically discovers and publishes
//! provider capabilities based on registered resource types, schemas, and provider
//! implementation. This eliminates the need for manual capability configuration.

use scim_server::{
    BulkCapabilities, CapabilityIntrospectable, ExtendedCapabilities, PaginationCapabilities,
    RequestContext, Resource, ResourceProvider, ScimOperation, ScimServer,
    create_user_resource_handler,
};
use serde_json::{Value, json};
use std::collections::HashMap;
use std::future::Future;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Example provider that implements capability introspection
struct AdvancedProvider {
    resources: Arc<RwLock<HashMap<String, Resource>>>,
    bulk_support: bool,
    max_page_size: usize,
}

impl AdvancedProvider {
    fn new() -> Self {
        Self {
            resources: Arc::new(RwLock::new(HashMap::new())),
            bulk_support: true,
            max_page_size: 500,
        }
    }
}

#[derive(Debug, thiserror::Error)]
#[error("Provider error: {message}")]
struct ProviderError {
    message: String,
}

impl ResourceProvider for AdvancedProvider {
    type Error = ProviderError;

    fn create_resource(
        &self,
        resource_type: &str,
        data: Value,
        _context: &RequestContext,
    ) -> impl Future<Output = Result<Resource, Self::Error>> + Send {
        let resource_type = resource_type.to_string();
        let resources = self.resources.clone();

        async move {
            let resource = Resource::from_json(resource_type, data).map_err(|e| ProviderError {
                message: format!("Failed to create resource: {}", e),
            })?;
            let id = resource.get_id().unwrap_or("unknown").to_string();

            resources.write().await.insert(id, resource.clone());
            Ok(resource)
        }
    }

    fn get_resource(
        &self,
        _resource_type: &str,
        id: &str,
        _context: &RequestContext,
    ) -> impl Future<Output = Result<Option<Resource>, Self::Error>> + Send {
        let id = id.to_string();
        let resources = self.resources.clone();

        async move { Ok(resources.read().await.get(&id).cloned()) }
    }

    fn update_resource(
        &self,
        resource_type: &str,
        id: &str,
        data: Value,
        _context: &RequestContext,
    ) -> impl Future<Output = Result<Resource, Self::Error>> + Send {
        let resource_type = resource_type.to_string();
        let id = id.to_string();
        let resources = self.resources.clone();

        async move {
            let resource = Resource::from_json(resource_type, data).map_err(|e| ProviderError {
                message: format!("Failed to update resource: {}", e),
            })?;
            resources.write().await.insert(id, resource.clone());
            Ok(resource)
        }
    }

    fn delete_resource(
        &self,
        _resource_type: &str,
        id: &str,
        _context: &RequestContext,
    ) -> impl Future<Output = Result<(), Self::Error>> + Send {
        let id = id.to_string();
        let resources = self.resources.clone();

        async move {
            resources.write().await.remove(&id);
            Ok(())
        }
    }

    fn list_resources(
        &self,
        _resource_type: &str,
        _query: Option<&scim_server::ListQuery>,
        _context: &RequestContext,
    ) -> impl Future<Output = Result<Vec<Resource>, Self::Error>> + Send {
        let resources = self.resources.clone();

        async move { Ok(resources.read().await.values().cloned().collect::<Vec<_>>()) }
    }

    fn find_resource_by_attribute(
        &self,
        _resource_type: &str,
        _attribute: &str,
        _value: &Value,
        _context: &RequestContext,
    ) -> impl Future<Output = Result<Option<Resource>, Self::Error>> + Send {
        async move { Ok(None) }
    }

    fn resource_exists(
        &self,
        _resource_type: &str,
        id: &str,
        _context: &RequestContext,
    ) -> impl Future<Output = Result<bool, Self::Error>> + Send {
        let id = id.to_string();
        let resources = self.resources.clone();

        async move { Ok(resources.read().await.contains_key(&id)) }
    }
}

// Implement capability introspection for our advanced provider
// Note: We provide our own implementation rather than using the default
impl CapabilityIntrospectable for AdvancedProvider {
    fn get_provider_specific_capabilities(&self) -> ExtendedCapabilities {
        ExtendedCapabilities {
            etag_supported: true,
            patch_supported: true,
            change_password_supported: true,
            sort_supported: true,
            custom_capabilities: {
                let mut custom = HashMap::new();
                custom.insert(
                    "advanced_filtering".to_string(),
                    json!({"regex_supported": true, "case_insensitive": true}),
                );
                custom.insert("transaction_support".to_string(), json!(true));
                custom
            },
        }
    }

    fn get_bulk_limits(&self) -> Option<BulkCapabilities> {
        Some(BulkCapabilities {
            supported: self.bulk_support,
            max_operations: Some(100),
            max_payload_size: Some(1024 * 1024), // 1MB
            fail_on_errors_supported: true,
        })
    }

    fn get_pagination_limits(&self) -> Option<scim_server::PaginationCapabilities> {
        Some(PaginationCapabilities {
            supported: true,
            default_page_size: Some(20),
            max_page_size: Some(self.max_page_size),
            cursor_based_supported: true,
        })
    }

    fn get_authentication_capabilities(&self) -> Option<scim_server::AuthenticationCapabilities> {
        Some(scim_server::AuthenticationCapabilities {
            schemes: vec![scim_server::schema_discovery::AuthenticationScheme {
                name: "OAuth 2.0 Bearer Token".to_string(),
                description: "OAuth 2.0 Bearer Token authentication".to_string(),
                spec_uri: Some("https://tools.ietf.org/html/rfc6750".to_string()),
                documentation_uri: Some("https://example.com/auth-docs".to_string()),
                auth_type: "oauth2".to_string(),
                primary: true,
            }],
            mfa_supported: true,
            token_refresh_supported: true,
        })
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 SCIM Server Automated Capability Discovery Example");
    println!("====================================================\n");

    // Create an advanced provider with specific capabilities
    let provider = AdvancedProvider::new();

    // Create SCIM server
    let mut server = ScimServer::new(provider)?;

    // Register User resource type with full CRUD operations
    let user_schema = server
        .get_schema_by_id("urn:ietf:params:scim:schemas:core:2.0:User")
        .unwrap()
        .clone();
    let user_handler = create_user_resource_handler(user_schema);

    server.register_resource_type(
        "User",
        user_handler,
        vec![
            ScimOperation::Create,
            ScimOperation::Read,
            ScimOperation::Update,
            ScimOperation::Delete,
            ScimOperation::List,
            ScimOperation::Search,
        ],
    )?;

    println!("📋 Server Configuration:");
    println!(
        "- Registered resource types: {:?}",
        server
            .get_supported_resource_types()
            .iter()
            .collect::<Vec<_>>()
    );
    println!(
        "- User operations: {:?}",
        server.get_supported_operations("User")
    );
    println!();

    // 🎯 **AUTOMATIC CAPABILITY DISCOVERY** - This is the key feature!
    println!("🔍 Discovering Capabilities Automatically...");
    let capabilities = server.discover_capabilities_with_introspection()?;

    println!("\n📊 **DISCOVERED CAPABILITIES**");
    println!("==============================");

    // Schema capabilities (auto-discovered from SchemaRegistry)
    println!("📋 Schemas:");
    for schema in &capabilities.supported_schemas {
        println!("  ✓ {}", schema);
    }

    // Resource type capabilities (auto-discovered from registered handlers)
    println!("\n🎯 Resource Types & Operations:");
    for (resource_type, operations) in &capabilities.supported_operations {
        println!("  📁 {}:", resource_type);
        for op in operations {
            println!("    ✓ {:?}", op);
        }
    }

    // Filtering capabilities (auto-discovered from schema attributes)
    println!("\n🔍 Filter Capabilities:");
    println!(
        "  Supported: {}",
        capabilities.filter_capabilities.supported
    );
    println!(
        "  Max Results: {:?}",
        capabilities.filter_capabilities.max_results
    );
    println!(
        "  Complex Filters: {}",
        capabilities.filter_capabilities.complex_filters_supported
    );

    println!("\n  📋 Filterable Attributes:");
    for (resource_type, attributes) in &capabilities.filter_capabilities.filterable_attributes {
        println!("    {} -> {:?}", resource_type, attributes);
    }

    println!("\n  🎛️ Supported Operators:");
    for operator in &capabilities.filter_capabilities.supported_operators {
        println!("    ✓ {:?}", operator);
    }

    // Bulk capabilities (from provider introspection)
    println!("\n📦 Bulk Operations:");
    println!("  Supported: {}", capabilities.bulk_capabilities.supported);
    if let Some(max_ops) = capabilities.bulk_capabilities.max_operations {
        println!("  Max Operations: {}", max_ops);
    }
    if let Some(max_size) = capabilities.bulk_capabilities.max_payload_size {
        println!("  Max Payload: {} bytes", max_size);
    }
    println!(
        "  Fail on Errors: {}",
        capabilities.bulk_capabilities.fail_on_errors_supported
    );

    // Pagination capabilities (from provider introspection)
    println!("\n📄 Pagination:");
    println!(
        "  Supported: {}",
        capabilities.pagination_capabilities.supported
    );
    if let Some(default_size) = capabilities.pagination_capabilities.default_page_size {
        println!("  Default Page Size: {}", default_size);
    }
    if let Some(max_size) = capabilities.pagination_capabilities.max_page_size {
        println!("  Max Page Size: {}", max_size);
    }
    println!(
        "  Cursor-based: {}",
        capabilities.pagination_capabilities.cursor_based_supported
    );

    // Authentication capabilities (from provider configuration)
    println!("\n🔐 Authentication:");
    for scheme in &capabilities.authentication_capabilities.schemes {
        println!("  ✓ {} ({})", scheme.name, scheme.auth_type);
        if scheme.primary {
            println!("    [PRIMARY]");
        }
    }
    println!(
        "  MFA Supported: {}",
        capabilities.authentication_capabilities.mfa_supported
    );
    println!(
        "  Token Refresh: {}",
        capabilities
            .authentication_capabilities
            .token_refresh_supported
    );

    // Extended capabilities (from provider introspection)
    println!("\n⚡ Extended Features:");
    println!(
        "  ETag Support: {}",
        capabilities.extended_capabilities.etag_supported
    );
    println!(
        "  PATCH Support: {}",
        capabilities.extended_capabilities.patch_supported
    );
    println!(
        "  Password Change: {}",
        capabilities.extended_capabilities.change_password_supported
    );
    println!(
        "  Sorting: {}",
        capabilities.extended_capabilities.sort_supported
    );

    println!("\n  🎛️ Custom Capabilities:");
    for (key, value) in &capabilities.extended_capabilities.custom_capabilities {
        println!("    {} -> {}", key, value);
    }

    // 🎯 **AUTO-GENERATED SERVICE PROVIDER CONFIG** - RFC 7644 compliant!
    println!("\n🌐 **RFC 7644 SERVICE PROVIDER CONFIG**");
    println!("======================================");

    let service_config = server.get_service_provider_config_with_introspection()?;

    println!("📋 SCIM ServiceProviderConfig (auto-generated):");
    println!("{}", serde_json::to_string_pretty(&service_config)?);

    // Demonstrate capability queries
    println!("\n🔍 **CAPABILITY QUERIES**");
    println!("=========================");

    println!(
        "Can create Users? {}",
        server.supports_operation("User", &ScimOperation::Create)
    );
    println!(
        "Can search Users? {}",
        server.supports_operation("User", &ScimOperation::Search)
    );
    println!(
        "Can create Groups? {}",
        server.supports_operation("Group", &ScimOperation::Create)
    );

    // Show that capabilities reflect actual server state
    println!("\n🎯 **DYNAMIC CAPABILITY UPDATES**");
    println!("=================================");

    println!(
        "Before: Supported resource types = {:?}",
        server
            .get_supported_resource_types()
            .iter()
            .collect::<Vec<_>>()
    );

    // Capabilities automatically update when we register new resource types
    // (This would be demonstrated if we had a Group handler)
    println!("Note: Capabilities automatically reflect current server state!");
    println!("When you register new resource types or change provider settings,");
    println!("the discovered capabilities update automatically.");

    println!("\n✨ **KEY BENEFITS**");
    println!("==================");
    println!("✓ No manual capability configuration required");
    println!("✓ Capabilities always match actual server state");
    println!("✓ RFC 7644 compliant ServiceProviderConfig");
    println!("✓ Real-time capability introspection");
    println!("✓ Type-safe capability constraints");
    println!("✓ Automatic schema-based filter capability discovery");

    Ok(())
}
