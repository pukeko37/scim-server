//! Test helpers for PATCH integration tests
//!
//! This module provides common setup and helper functions for PATCH operation testing,
//! including proper server configuration with User and Group resource types and
//! patch capabilities enabled.

use scim_server::{
    RequestContext, ScimOperation, ScimServer,
    multi_tenant::TenantContext,
    providers::StandardResourceProvider,
    storage::InMemoryStorage,
    provider_capabilities::{
        AuthenticationCapabilities, BulkCapabilities, ExtendedCapabilities, FilterCapabilities,
        PaginationCapabilities, ProviderCapabilities,
    },
    resource_handlers::{create_group_resource_handler, create_user_resource_handler},
    schema::SchemaRegistry,
};

/// Create a fully configured SCIM server with User and Group support and patch capabilities
pub fn create_test_server_with_patch_support() -> ScimServer<StandardResourceProvider<InMemoryStorage>> {
    let storage = InMemoryStorage::new();
    let provider = StandardResourceProvider::new(storage);
    let mut server = ScimServer::new(provider).expect("Failed to create server");

    // Register User resource type with full operations including Patch
    register_user_resource_type(&mut server).expect("Failed to register User resource type");

    // Register Group resource type with full operations including Patch
    register_group_resource_type(&mut server).expect("Failed to register Group resource type");

    server
}

/// Create a SCIM server with patch capabilities disabled
pub fn create_test_server_without_patch_support() -> ScimServer<StandardResourceProvider<InMemoryStorage>> {
    let storage = InMemoryStorage::new();
    let provider = StandardResourceProvider::new(storage);
    let mut server = ScimServer::new(provider).expect("Failed to create server");

    // Register resource types without Patch operation
    register_user_resource_type_without_patch(&mut server)
        .expect("Failed to register User resource type");
    register_group_resource_type_without_patch(&mut server)
        .expect("Failed to register Group resource type");

    server
}

/// Register User resource type with full operations including Patch
fn register_user_resource_type(
    server: &mut ScimServer<StandardResourceProvider<InMemoryStorage>>,
) -> Result<(), Box<dyn std::error::Error>> {
    let registry = SchemaRegistry::new()?;
    let user_schema = registry.get_user_schema().clone();
    let user_handler = create_user_resource_handler(user_schema);

    server.register_resource_type(
        "User",
        user_handler,
        vec![
            ScimOperation::Create,
            ScimOperation::Read,
            ScimOperation::Update,
            ScimOperation::Patch,
            ScimOperation::Delete,
            ScimOperation::List,
            ScimOperation::Search,
        ],
    )?;

    Ok(())
}

/// Register Group resource type with full operations including Patch
fn register_group_resource_type(
    server: &mut ScimServer<StandardResourceProvider<InMemoryStorage>>,
) -> Result<(), Box<dyn std::error::Error>> {
    let registry = SchemaRegistry::new()?;
    let group_schema = registry.get_group_schema().clone();
    let group_handler = create_group_resource_handler(group_schema);

    server.register_resource_type(
        "Group",
        group_handler,
        vec![
            ScimOperation::Create,
            ScimOperation::Read,
            ScimOperation::Update,
            ScimOperation::Patch,
            ScimOperation::Delete,
            ScimOperation::List,
            ScimOperation::Search,
        ],
    )?;

    Ok(())
}

/// Register User resource type without Patch operation (for testing disabled patch)
fn register_user_resource_type_without_patch(
    server: &mut ScimServer<StandardResourceProvider<InMemoryStorage>>,
) -> Result<(), Box<dyn std::error::Error>> {
    let registry = SchemaRegistry::new()?;
    let user_schema = registry.get_user_schema().clone();
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

    Ok(())
}

/// Register Group resource type without Patch operation (for testing disabled patch)
fn register_group_resource_type_without_patch(
    server: &mut ScimServer<StandardResourceProvider<InMemoryStorage>>,
) -> Result<(), Box<dyn std::error::Error>> {
    let registry = SchemaRegistry::new()?;
    let group_schema = registry.get_group_schema().clone();
    let group_handler = create_group_resource_handler(group_schema);

    server.register_resource_type(
        "Group",
        group_handler,
        vec![
            ScimOperation::Create,
            ScimOperation::Read,
            ScimOperation::Update,
            ScimOperation::Delete,
            ScimOperation::List,
            ScimOperation::Search,
        ],
    )?;

    Ok(())
}

/// Create a test request context
pub fn create_test_context() -> RequestContext {
    RequestContext::with_generated_id()
}

/// Create a test request context with specific tenant
pub fn create_test_context_with_tenant(tenant_id: &str, client_id: &str) -> RequestContext {
    let tenant_context = TenantContext::new(tenant_id.to_string(), client_id.to_string());
    RequestContext::with_tenant_generated_id(tenant_context)
}

/// Create an InMemoryProvider with patch capabilities enabled
pub fn create_provider_with_patch_enabled() -> StandardResourceProvider<InMemoryStorage> {
    // For now, return default provider
    // In a real implementation, this would configure the provider to advertise patch support
    let storage = InMemoryStorage::new();
    StandardResourceProvider::new(storage)
}

/// Create an InMemoryProvider with patch capabilities disabled
pub fn create_provider_with_patch_disabled() -> StandardResourceProvider<InMemoryStorage> {
    // For now, return default provider
    // In a real implementation, this would configure the provider to not advertise patch support
    let storage = InMemoryStorage::new();
    StandardResourceProvider::new(storage)
}

/// Helper to create a user resource for testing
pub async fn create_test_user(
    server: &ScimServer<StandardResourceProvider<InMemoryStorage>>,
    context: &RequestContext,
) -> Result<scim_server::Resource, Box<dyn std::error::Error>> {
    let user_data = serde_json::json!({
        "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
        "userName": "test.user",
        "displayName": "Test User",
        "emails": [
            {
                "value": "test@example.com",
                "primary": true,
                "type": "work"
            }
        ],
        "active": true
    });

    let created = server.create_resource("User", user_data, context).await?;
    Ok(created)
}

/// Helper to create a group resource for testing
pub async fn create_test_group(
    server: &ScimServer<StandardResourceProvider<InMemoryStorage>>,
    context: &RequestContext,
) -> Result<scim_server::Resource, Box<dyn std::error::Error>> {
    let group_data = serde_json::json!({
        "schemas": ["urn:ietf:params:scim:schemas:core:2.0:Group"],
        "displayName": "Test Group",
        "members": []
    });

    let created = server.create_resource("Group", group_data, context).await?;
    Ok(created)
}

/// Assert that a server has patch support properly configured
pub fn assert_patch_support_enabled(server: &ScimServer<StandardResourceProvider<InMemoryStorage>>) {
    let config = server
        .get_service_provider_config()
        .expect("Should be able to get service provider config");

    assert!(config.patch_supported, "Patch support should be enabled");
}

/// Assert that a server has patch support disabled
pub fn assert_patch_support_disabled(server: &ScimServer<StandardResourceProvider<InMemoryStorage>>) {
    let config = server
        .get_service_provider_config()
        .expect("Should be able to get service provider config");

    assert!(!config.patch_supported, "Patch support should be disabled");
}

/// Helper to check if a resource type is supported
pub fn is_resource_type_supported(
    server: &ScimServer<StandardResourceProvider<InMemoryStorage>>,
    resource_type: &str,
) -> bool {
    server
        .get_supported_resource_types()
        .contains(&resource_type)
}

/// Assert that required resource types are registered
pub fn assert_required_resource_types_registered(server: &ScimServer<StandardResourceProvider<InMemoryStorage>>) {
    assert!(
        is_resource_type_supported(server, "User"),
        "User resource type should be registered"
    );
    assert!(
        is_resource_type_supported(server, "Group"),
        "Group resource type should be registered"
    );
}

/// Create provider capabilities with patch enabled
pub fn create_patch_enabled_capabilities() -> ProviderCapabilities {
    use std::collections::HashMap;

    let mut supported_operations = HashMap::new();
    supported_operations.insert(
        "User".to_string(),
        vec![
            scim_server::multi_tenant::ScimOperation::Create,
            scim_server::multi_tenant::ScimOperation::Read,
            scim_server::multi_tenant::ScimOperation::Update,
            scim_server::multi_tenant::ScimOperation::Patch,
            scim_server::multi_tenant::ScimOperation::Delete,
            scim_server::multi_tenant::ScimOperation::List,
        ],
    );
    supported_operations.insert(
        "Group".to_string(),
        vec![
            scim_server::multi_tenant::ScimOperation::Create,
            scim_server::multi_tenant::ScimOperation::Read,
            scim_server::multi_tenant::ScimOperation::Update,
            scim_server::multi_tenant::ScimOperation::Patch,
            scim_server::multi_tenant::ScimOperation::Delete,
            scim_server::multi_tenant::ScimOperation::List,
        ],
    );

    ProviderCapabilities {
        supported_operations,
        supported_schemas: vec![
            "urn:ietf:params:scim:schemas:core:2.0:User".to_string(),
            "urn:ietf:params:scim:schemas:core:2.0:Group".to_string(),
        ],
        supported_resource_types: vec!["User".to_string(), "Group".to_string()],
        bulk_capabilities: BulkCapabilities::default(),
        filter_capabilities: FilterCapabilities::default(),
        pagination_capabilities: PaginationCapabilities::default(),
        authentication_capabilities: AuthenticationCapabilities::default(),
        extended_capabilities: ExtendedCapabilities {
            patch_supported: true,
            etag_supported: true,
            change_password_supported: false,
            sort_supported: true,
            custom_capabilities: std::collections::HashMap::new(),
        },
    }
}

/// Create provider capabilities with patch disabled
pub fn create_patch_disabled_capabilities() -> ProviderCapabilities {
    use std::collections::HashMap;

    let mut supported_operations = HashMap::new();
    supported_operations.insert(
        "User".to_string(),
        vec![
            scim_server::multi_tenant::ScimOperation::Create,
            scim_server::multi_tenant::ScimOperation::Read,
            scim_server::multi_tenant::ScimOperation::Update,
            scim_server::multi_tenant::ScimOperation::Delete,
            scim_server::multi_tenant::ScimOperation::List,
        ],
    );
    supported_operations.insert(
        "Group".to_string(),
        vec![
            scim_server::multi_tenant::ScimOperation::Create,
            scim_server::multi_tenant::ScimOperation::Read,
            scim_server::multi_tenant::ScimOperation::Update,
            scim_server::multi_tenant::ScimOperation::Delete,
            scim_server::multi_tenant::ScimOperation::List,
        ],
    );

    ProviderCapabilities {
        supported_operations,
        supported_schemas: vec![
            "urn:ietf:params:scim:schemas:core:2.0:User".to_string(),
            "urn:ietf:params:scim:schemas:core:2.0:Group".to_string(),
        ],
        supported_resource_types: vec!["User".to_string(), "Group".to_string()],
        bulk_capabilities: BulkCapabilities::default(),
        filter_capabilities: FilterCapabilities::default(),
        pagination_capabilities: PaginationCapabilities::default(),
        authentication_capabilities: AuthenticationCapabilities::default(),
        extended_capabilities: ExtendedCapabilities {
            patch_supported: false,
            etag_supported: true,
            change_password_supported: false,
            sort_supported: true,
            custom_capabilities: std::collections::HashMap::new(),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_server_with_patch_support() {
        let server = create_test_server_with_patch_support();
        assert_required_resource_types_registered(&server);

        // Check that Patch operation is supported for User
        let user_operations = server.get_supported_operations("User").unwrap();
        assert!(user_operations.contains(&ScimOperation::Patch));

        // Check that Patch operation is supported for Group
        let group_operations = server.get_supported_operations("Group").unwrap();
        assert!(group_operations.contains(&ScimOperation::Patch));
    }

    #[test]
    fn test_create_server_without_patch_support() {
        let server = create_test_server_without_patch_support();
        assert_required_resource_types_registered(&server);

        // Check that Patch operation is NOT supported for User
        let user_operations = server.get_supported_operations("User").unwrap();
        assert!(!user_operations.contains(&ScimOperation::Patch));

        // Check that Patch operation is NOT supported for Group
        let group_operations = server.get_supported_operations("Group").unwrap();
        assert!(!group_operations.contains(&ScimOperation::Patch));
    }

    #[test]
    fn test_context_creation() {
        let context = create_test_context();
        assert!(!context.request_id.is_empty());

        let tenant_context = create_test_context_with_tenant("test-tenant", "test-client");
        assert_eq!(tenant_context.tenant_id(), Some("test-tenant"));
        assert_eq!(tenant_context.client_id(), Some("test-client"));
    }

    #[tokio::test]
    async fn test_resource_creation_helpers() {
        let server = create_test_server_with_patch_support();
        let context = create_test_context();

        // Test user creation
        let user = create_test_user(&server, &context).await;
        assert!(user.is_ok(), "Should be able to create test user");
        let user_resource = user.unwrap();
        assert!(user_resource.get_id().is_some(), "User should have ID");

        // Test group creation
        let group = create_test_group(&server, &context).await;
        assert!(group.is_ok(), "Should be able to create test group");
        let group_resource = group.unwrap();
        assert!(group_resource.get_id().is_some(), "Group should have ID");
    }
}
