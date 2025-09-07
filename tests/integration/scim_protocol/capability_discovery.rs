//! Integration tests for automated provider capability discovery.
//!
//! These tests verify that the capability discovery system correctly introspects
//! the server configuration and generates accurate capability information.

use scim_server::{
    BulkCapabilities, CapabilityIntrospectable, ExtendedCapabilities, ListQuery,
    PaginationCapabilities, RequestContext, Resource, ResourceProvider, ScimOperation, ScimServer,
    create_user_resource_handler,
    resource::{version::RawVersion, versioned::VersionedResource},
};
use serde_json::{Value, json};
use std::collections::HashMap;
use std::future::Future;

/// Test provider that implements capability introspection
struct TestProvider;

impl TestProvider {
    fn new() -> Self {
        Self
    }
}

#[derive(Debug, thiserror::Error)]
#[error("Test provider error")]
struct TestError;

impl ResourceProvider for TestProvider {
    type Error = TestError;

    fn create_resource(
        &self,
        resource_type: &str,
        data: Value,
        _context: &RequestContext,
    ) -> impl Future<Output = Result<VersionedResource, Self::Error>> + Send {
        let resource_type = resource_type.to_string();
        async move {
            let resource =
                Resource::from_json(resource_type, data).expect("Failed to create resource");
            Ok(VersionedResource::new(resource))
        }
    }

    fn get_resource(
        &self,
        _resource_type: &str,
        _id: &str,
        _context: &RequestContext,
    ) -> impl Future<Output = Result<Option<VersionedResource>, Self::Error>> + Send {
        async move { Ok(None) }
    }

    fn update_resource(
        &self,
        resource_type: &str,
        _id: &str,
        data: Value,
        _expected_version: Option<&RawVersion>,
        _context: &RequestContext,
    ) -> impl Future<Output = Result<VersionedResource, Self::Error>> + Send {
        let resource_type = resource_type.to_string();
        async move {
            let resource =
                Resource::from_json(resource_type, data).expect("Failed to create resource");
            Ok(VersionedResource::new(resource))
        }
    }

    fn delete_resource(
        &self,
        _resource_type: &str,
        _id: &str,
        _expected_version: Option<&RawVersion>,
        _context: &RequestContext,
    ) -> impl Future<Output = Result<(), Self::Error>> + Send {
        async move { Ok(()) }
    }

    fn list_resources(
        &self,
        _resource_type: &str,
        _query: Option<&ListQuery>,
        _context: &RequestContext,
    ) -> impl Future<Output = Result<Vec<VersionedResource>, Self::Error>> + Send {
        async move { Ok(vec![]) }
    }

    fn find_resources_by_attribute(
        &self,
        _resource_type: &str,
        _attribute: &str,
        _value: &str,
        _context: &RequestContext,
    ) -> impl Future<Output = Result<Vec<VersionedResource>, Self::Error>> + Send {
        async move { Ok(vec![]) }
    }

    fn patch_resource(
        &self,
        _resource_type: &str,
        _id: &str,
        _patch_request: &Value,
        _expected_version: Option<&RawVersion>,
        _context: &RequestContext,
    ) -> impl Future<Output = Result<VersionedResource, Self::Error>> + Send {
        async move { Err(TestError) }
    }

    fn resource_exists(
        &self,
        _resource_type: &str,
        _id: &str,
        _context: &RequestContext,
    ) -> impl Future<Output = Result<bool, Self::Error>> + Send {
        async move { Ok(false) }
    }
}

impl CapabilityIntrospectable for TestProvider {
    fn get_provider_specific_capabilities(&self) -> ExtendedCapabilities {
        ExtendedCapabilities {
            etag_supported: true,
            patch_supported: true,
            change_password_supported: false,
            sort_supported: true,
            custom_capabilities: {
                let mut custom = HashMap::new();
                custom.insert("test_feature".to_string(), json!(true));
                custom
            },
        }
    }

    fn get_bulk_limits(&self) -> Option<BulkCapabilities> {
        Some(BulkCapabilities {
            supported: true,
            max_operations: Some(50),
            max_payload_size: Some(512 * 1024), // 512KB
            fail_on_errors_supported: true,
        })
    }

    fn get_pagination_limits(&self) -> Option<PaginationCapabilities> {
        Some(PaginationCapabilities {
            supported: true,
            default_page_size: Some(25),
            max_page_size: Some(100),
            cursor_based_supported: false,
        })
    }
}

#[tokio::test]
async fn test_basic_capability_discovery() {
    let provider = TestProvider::new();
    let mut server = ScimServer::new(provider).expect("Failed to create server");

    // Register User resource type
    let user_schema = server
        .get_schema_by_id("urn:ietf:params:scim:schemas:core:2.0:User")
        .unwrap()
        .clone();
    let user_handler = create_user_resource_handler(user_schema);

    server
        .register_resource_type(
            "User",
            user_handler,
            vec![ScimOperation::Create, ScimOperation::Read],
        )
        .expect("Failed to register User resource type");

    // Discover capabilities
    let capabilities = server
        .discover_capabilities()
        .expect("Failed to discover capabilities");

    // Verify supported schemas
    assert_eq!(capabilities.supported_schemas.len(), 2);
    assert!(
        capabilities
            .supported_schemas
            .contains(&"urn:ietf:params:scim:schemas:core:2.0:User".to_string())
    );
    assert!(
        capabilities
            .supported_schemas
            .contains(&"urn:ietf:params:scim:schemas:core:2.0:Group".to_string())
    );

    // Verify supported resource types
    assert_eq!(capabilities.supported_resource_types.len(), 1);
    assert!(
        capabilities
            .supported_resource_types
            .contains(&"User".to_string())
    );

    // Verify supported operations
    let user_operations = capabilities.supported_operations.get("User").unwrap();
    assert_eq!(user_operations.len(), 2);
    assert!(user_operations.contains(&ScimOperation::Create));
    assert!(user_operations.contains(&ScimOperation::Read));

    // Verify filter capabilities
    assert!(capabilities.filter_capabilities.supported);
    assert_eq!(capabilities.filter_capabilities.max_results, Some(200));
    assert!(capabilities.filter_capabilities.complex_filters_supported);

    // Verify filterable attributes include sub-attributes
    let user_filterable = capabilities
        .filter_capabilities
        .filterable_attributes
        .get("User")
        .unwrap();
    assert!(user_filterable.contains(&"userName".to_string()));
    assert!(user_filterable.contains(&"meta.created".to_string()));
    assert!(user_filterable.contains(&"emails.value".to_string()));

    // Verify basic capabilities (without introspection)
    assert!(!capabilities.bulk_capabilities.supported); // Default is false
    assert!(capabilities.pagination_capabilities.supported); // Default is true
}

#[tokio::test]
async fn test_capability_discovery_with_introspection() {
    let provider = TestProvider::new();
    let mut server = ScimServer::new(provider).expect("Failed to create server");

    // Register User resource type
    let user_schema = server
        .get_schema_by_id("urn:ietf:params:scim:schemas:core:2.0:User")
        .unwrap()
        .clone();
    let user_handler = create_user_resource_handler(user_schema);

    server
        .register_resource_type(
            "User",
            user_handler,
            vec![
                ScimOperation::Create,
                ScimOperation::Read,
                ScimOperation::Update,
                ScimOperation::Delete,
            ],
        )
        .expect("Failed to register User resource type");

    // Discover capabilities with introspection
    let capabilities = server
        .discover_capabilities_with_introspection()
        .expect("Failed to discover capabilities with introspection");

    // Verify provider-specific bulk capabilities
    assert!(capabilities.bulk_capabilities.supported);
    assert_eq!(capabilities.bulk_capabilities.max_operations, Some(50));
    assert_eq!(
        capabilities.bulk_capabilities.max_payload_size,
        Some(512 * 1024)
    );
    assert!(capabilities.bulk_capabilities.fail_on_errors_supported);

    // Verify provider-specific pagination capabilities
    assert!(capabilities.pagination_capabilities.supported);
    assert_eq!(
        capabilities.pagination_capabilities.default_page_size,
        Some(25)
    );
    assert_eq!(
        capabilities.pagination_capabilities.max_page_size,
        Some(100)
    );
    assert!(!capabilities.pagination_capabilities.cursor_based_supported);

    // Verify extended capabilities
    assert!(capabilities.extended_capabilities.etag_supported);
    assert!(capabilities.extended_capabilities.patch_supported);
    assert!(!capabilities.extended_capabilities.change_password_supported);
    assert!(capabilities.extended_capabilities.sort_supported);

    // Verify custom capabilities
    assert_eq!(
        capabilities
            .extended_capabilities
            .custom_capabilities
            .get("test_feature"),
        Some(&json!(true))
    );
}

#[tokio::test]
async fn test_service_provider_config_generation() {
    let provider = TestProvider::new();
    let mut server = ScimServer::new(provider).expect("Failed to create server");

    // Register User resource type with multiple operations
    let user_schema = server
        .get_schema_by_id("urn:ietf:params:scim:schemas:core:2.0:User")
        .unwrap()
        .clone();
    let user_handler = create_user_resource_handler(user_schema);

    server
        .register_resource_type(
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
        )
        .expect("Failed to register User resource type");

    // Generate service provider config with introspection
    let config = server
        .get_service_provider_config_with_introspection()
        .expect("Failed to generate service provider config");

    // Verify RFC 7644 compliance
    assert!(config.patch_supported); // From provider introspection
    assert!(config.bulk_supported); // From provider introspection
    assert!(config.filter_supported); // Auto-discovered from schemas
    assert!(!config.change_password_supported); // From provider introspection
    assert!(config.sort_supported); // From provider introspection
    assert!(config.etag_supported); // From provider introspection

    // Verify bulk limits
    assert_eq!(config.bulk_max_operations, Some(50));
    assert_eq!(config.bulk_max_payload_size, Some(512 * 1024));

    // Verify filter limits
    assert_eq!(config.filter_max_results, Some(200));

    // Verify authentication schemes (empty by default)
    assert!(config.authentication_schemes.is_empty());
}

#[tokio::test]
async fn test_capability_queries() {
    let provider = TestProvider::new();
    let mut server = ScimServer::new(provider).expect("Failed to create server");

    // Register User resource type
    let user_schema = server
        .get_schema_by_id("urn:ietf:params:scim:schemas:core:2.0:User")
        .unwrap()
        .clone();
    let user_handler = create_user_resource_handler(user_schema);

    server
        .register_resource_type(
            "User",
            user_handler,
            vec![
                ScimOperation::Create,
                ScimOperation::Read,
                ScimOperation::Search,
            ],
        )
        .expect("Failed to register User resource type");

    // Test capability queries
    assert!(server.supports_operation("User", &ScimOperation::Create));
    assert!(server.supports_operation("User", &ScimOperation::Read));
    assert!(server.supports_operation("User", &ScimOperation::Search));
    assert!(!server.supports_operation("User", &ScimOperation::Update));
    assert!(!server.supports_operation("User", &ScimOperation::Delete));

    // Test non-existent resource type
    assert!(!server.supports_operation("Group", &ScimOperation::Create));

    // Verify resource types
    let resource_types: Vec<&str> = server.get_supported_resource_types();
    assert_eq!(resource_types.len(), 1);
    assert!(resource_types.contains(&"User"));

    // Verify operations for resource type
    let user_operations = server.get_supported_operations("User").unwrap();
    assert_eq!(user_operations.len(), 3);
    assert!(user_operations.contains(&ScimOperation::Create));
    assert!(user_operations.contains(&ScimOperation::Read));
    assert!(user_operations.contains(&ScimOperation::Search));
}

#[tokio::test]
async fn test_filter_operator_discovery() {
    let provider = TestProvider::new();
    let server = ScimServer::new(provider).expect("Failed to create server");

    let capabilities = server
        .discover_capabilities()
        .expect("Failed to discover capabilities");

    // Verify that all expected operators are discovered
    let operators = &capabilities.filter_capabilities.supported_operators;

    // Basic operators should always be present
    assert!(
        operators
            .iter()
            .any(|op| matches!(op, scim_server::FilterOperator::Equal))
    );
    assert!(
        operators
            .iter()
            .any(|op| matches!(op, scim_server::FilterOperator::NotEqual))
    );
    assert!(
        operators
            .iter()
            .any(|op| matches!(op, scim_server::FilterOperator::Present))
    );

    // String operators should be present (User schema has string attributes)
    assert!(
        operators
            .iter()
            .any(|op| matches!(op, scim_server::FilterOperator::Contains))
    );
    assert!(
        operators
            .iter()
            .any(|op| matches!(op, scim_server::FilterOperator::StartsWith))
    );
    assert!(
        operators
            .iter()
            .any(|op| matches!(op, scim_server::FilterOperator::EndsWith))
    );

    // Comparison operators should be present (User schema has dateTime in meta sub-attributes)
    assert!(
        operators
            .iter()
            .any(|op| matches!(op, scim_server::FilterOperator::GreaterThan))
    );
    assert!(
        operators
            .iter()
            .any(|op| matches!(op, scim_server::FilterOperator::LessThan))
    );
}

#[tokio::test]
async fn test_dynamic_capability_updates() {
    let provider = TestProvider::new();
    let mut server = ScimServer::new(provider).expect("Failed to create server");

    // Initially no resource types
    let initial_types: Vec<&str> = server.get_supported_resource_types();
    assert!(initial_types.is_empty());

    // Discover initial capabilities
    let initial_capabilities = server
        .discover_capabilities()
        .expect("Failed to discover initial capabilities");
    assert!(initial_capabilities.supported_resource_types.is_empty());
    assert!(initial_capabilities.supported_operations.is_empty());

    // Register User resource type
    let user_schema = server
        .get_schema_by_id("urn:ietf:params:scim:schemas:core:2.0:User")
        .unwrap()
        .clone();
    let user_handler = create_user_resource_handler(user_schema);

    server
        .register_resource_type("User", user_handler, vec![ScimOperation::Create])
        .expect("Failed to register User resource type");

    // Capabilities should now reflect the registered resource type
    let updated_capabilities = server
        .discover_capabilities()
        .expect("Failed to discover updated capabilities");

    assert_eq!(updated_capabilities.supported_resource_types.len(), 1);
    assert!(
        updated_capabilities
            .supported_resource_types
            .contains(&"User".to_string())
    );

    let user_operations = updated_capabilities
        .supported_operations
        .get("User")
        .unwrap();
    assert_eq!(user_operations.len(), 1);
    assert!(user_operations.contains(&ScimOperation::Create));

    // Filter capabilities should include User attributes
    let user_filterable = updated_capabilities
        .filter_capabilities
        .filterable_attributes
        .get("User")
        .unwrap();
    assert!(!user_filterable.is_empty());
    assert!(user_filterable.contains(&"userName".to_string()));
}
