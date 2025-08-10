//! SCIM-focused multi-tenant integration tests.
//!
//! This module tests the new SCIM-specific multi-tenant configuration and
//! orchestration system. Unlike the previous general-purpose configuration
//! tests, these focus specifically on SCIM protocol compliance across
//! tenant boundaries.

use scim_server::{
    ListQuery, RequestContext, Resource, ResourceProvider, ScimTenantConfiguration,
    StaticTenantResolver, TenantContext, TenantResolver,
};
use serde_json::{Value, json};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;

/// Test provider for SCIM multi-tenant operations
struct TestScimProvider {
    // Tenant-isolated resource storage
    tenant_resources: Arc<RwLock<HashMap<String, HashMap<String, Resource>>>>,
    // SCIM configurations per tenant
    scim_configs: Arc<RwLock<HashMap<String, ScimTenantConfiguration>>>,
}

impl TestScimProvider {
    fn new() -> Self {
        Self {
            tenant_resources: Arc::new(RwLock::new(HashMap::new())),
            scim_configs: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    async fn add_scim_config(&self, config: ScimTenantConfiguration) {
        let mut configs = self.scim_configs.write().await;
        configs.insert(config.tenant_id.clone(), config);
    }

    async fn get_scim_config(&self, tenant_id: &str) -> Option<ScimTenantConfiguration> {
        let configs = self.scim_configs.read().await;
        configs.get(tenant_id).cloned()
    }
}

#[derive(Debug, thiserror::Error)]
#[error("Test provider error")]
struct TestProviderError;

impl ResourceProvider for TestScimProvider {
    type Error = TestProviderError;

    async fn create_resource(
        &self,
        resource_type: &str,
        data: Value,
        context: &RequestContext,
    ) -> Result<Resource, Self::Error> {
        let tenant_id = context.tenant_id().ok_or(TestProviderError)?;

        // Check SCIM configuration for rate limiting
        if let Some(config) = self.get_scim_config(tenant_id).await {
            // In a real implementation, you'd track request counts and check rate limits
            let current_create_count = 0; // Mock value
            if config.is_rate_limited("create", current_create_count) {
                // In real implementation, this would be a proper SCIM error
                return Err(TestProviderError);
            }
        }

        let resource = Resource::from_json(resource_type.to_string(), data)
            .expect("Failed to create resource");

        // Store in tenant-isolated storage
        let mut resources = self.tenant_resources.write().await;
        let tenant_store = resources
            .entry(tenant_id.to_string())
            .or_insert_with(HashMap::new);
        if let Some(id) = resource.get_id() {
            tenant_store.insert(id.to_string(), resource.clone());
        }

        Ok(resource)
    }

    async fn get_resource(
        &self,
        _resource_type: &str,
        id: &str,
        context: &RequestContext,
    ) -> Result<Option<Resource>, Self::Error> {
        let tenant_id = context.tenant_id().ok_or(TestProviderError)?;

        let resources = self.tenant_resources.read().await;
        if let Some(tenant_store) = resources.get(tenant_id) {
            Ok(tenant_store.get(id).cloned())
        } else {
            Ok(None)
        }
    }

    async fn update_resource(
        &self,
        resource_type: &str,
        _id: &str,
        data: Value,
        context: &RequestContext,
    ) -> Result<Resource, Self::Error> {
        let tenant_id = context.tenant_id().ok_or(TestProviderError)?;

        // Check SCIM configuration for update permissions
        if let Some(config) = self.get_scim_config(tenant_id).await {
            let current_update_count = 0; // Mock value
            if config.is_rate_limited("update", current_update_count) {
                return Err(TestProviderError);
            }
        }

        let resource = Resource::from_json(resource_type.to_string(), data)
            .expect("Failed to create resource");

        let mut resources = self.tenant_resources.write().await;
        if let Some(tenant_store) = resources.get_mut(tenant_id) {
            if let Some(id) = resource.get_id() {
                tenant_store.insert(id.to_string(), resource.clone());
            }
        }

        Ok(resource)
    }

    async fn delete_resource(
        &self,
        _resource_type: &str,
        id: &str,
        context: &RequestContext,
    ) -> Result<(), Self::Error> {
        let tenant_id = context.tenant_id().ok_or(TestProviderError)?;

        let mut resources = self.tenant_resources.write().await;
        if let Some(tenant_store) = resources.get_mut(tenant_id) {
            tenant_store.remove(id);
        }
        Ok(())
    }

    async fn list_resources(
        &self,
        _resource_type: &str,
        _query: Option<&ListQuery>,
        context: &RequestContext,
    ) -> Result<Vec<Resource>, Self::Error> {
        let tenant_id = context.tenant_id().ok_or(TestProviderError)?;

        let resources = self.tenant_resources.read().await;
        if let Some(tenant_store) = resources.get(tenant_id) {
            Ok(tenant_store.values().cloned().collect())
        } else {
            Ok(vec![])
        }
    }

    async fn find_resource_by_attribute(
        &self,
        _resource_type: &str,
        attribute: &str,
        value: &Value,
        context: &RequestContext,
    ) -> Result<Option<Resource>, Self::Error> {
        let tenant_id = context.tenant_id().ok_or(TestProviderError)?;

        let resources = self.tenant_resources.read().await;
        if let Some(tenant_store) = resources.get(tenant_id) {
            for resource in tenant_store.values() {
                if let Some(attr_value) = resource.get_attribute(attribute) {
                    if attr_value == value {
                        return Ok(Some(resource.clone()));
                    }
                }
            }
        }
        Ok(None)
    }

    async fn resource_exists(
        &self,
        _resource_type: &str,
        id: &str,
        context: &RequestContext,
    ) -> Result<bool, Self::Error> {
        let tenant_id = context.tenant_id().ok_or(TestProviderError)?;

        let resources = self.tenant_resources.read().await;
        if let Some(tenant_store) = resources.get(tenant_id) {
            Ok(tenant_store.contains_key(id))
        } else {
            Ok(false)
        }
    }
}

/// Test SCIM-specific tenant configuration
#[tokio::test]
async fn test_scim_tenant_configuration() {
    // Create SCIM-focused tenant configuration
    let scim_config = ScimTenantConfiguration::builder("tenant-a".to_string())
        .with_endpoint_path("/scim/v2/tenant-a")
        .with_scim_rate_limit(100, Duration::from_secs(60))
        .with_scim_client("okta-client", "api_key_okta_123")
        .with_scim_client("azure-client", "api_key_azure_456")
        .enable_scim_audit_log()
        .with_schema_extension(
            "urn:ietf:params:scim:schemas:extension:enterprise:2.0:User",
            false,
        )
        .build()
        .expect("Valid SCIM configuration");

    // Verify SCIM-specific configuration
    assert_eq!(scim_config.tenant_id, "tenant-a");
    assert_eq!(scim_config.endpoint.base_path, "/scim/v2/tenant-a");
    assert_eq!(scim_config.clients.len(), 2);

    // Verify SCIM client configurations
    let okta_client = scim_config.get_client_config("okta-client").unwrap();
    assert_eq!(okta_client.client_id, "okta-client");
    assert!(
        okta_client
            .allowed_operations
            .contains(&scim_server::ScimOperation::Create)
    );

    // Verify SCIM schema extension
    assert!(
        scim_config
            .has_schema_extension("urn:ietf:params:scim:schemas:extension:enterprise:2.0:User")
    );

    // Verify SCIM audit configuration
    assert!(scim_config.audit_config.enabled);
}

/// Test SCIM multi-tenant resource operations with tenant isolation
#[tokio::test]
async fn test_scim_multi_tenant_resource_operations() {
    let provider = TestScimProvider::new();

    // Set up SCIM configurations for multiple tenants
    let tenant_a_config = ScimTenantConfiguration::builder("tenant-a".to_string())
        .with_scim_client("client-a", "api_key_a")
        .build()
        .expect("Valid config");

    let tenant_b_config = ScimTenantConfiguration::builder("tenant-b".to_string())
        .with_scim_client("client-b", "api_key_b")
        .build()
        .expect("Valid config");

    provider.add_scim_config(tenant_a_config).await;
    provider.add_scim_config(tenant_b_config).await;

    // Create tenant contexts
    let tenant_a_context = TenantContext::new("tenant-a".to_string(), "client-a".to_string());
    let tenant_b_context = TenantContext::new("tenant-b".to_string(), "client-b".to_string());

    let context_a = RequestContext::with_tenant_generated_id(tenant_a_context);
    let context_b = RequestContext::with_tenant_generated_id(tenant_b_context);

    // Create SCIM User resources in different tenants
    let user_a_data = json!({
        "id": "user-a-1",
        "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
        "userName": "john.doe@tenant-a.com",
        "displayName": "John Doe (Tenant A)",
        "emails": [{"value": "john.doe@tenant-a.com", "primary": true}]
    });

    let user_b_data = json!({
        "id": "user-b-1",
        "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
        "userName": "jane.smith@tenant-b.com",
        "displayName": "Jane Smith (Tenant B)",
        "emails": [{"value": "jane.smith@tenant-b.com", "primary": true}]
    });

    // Create resources in their respective tenants
    let user_a = provider
        .create_resource("User", user_a_data, &context_a)
        .await
        .expect("Should create user in tenant A");

    let user_b = provider
        .create_resource("User", user_b_data, &context_b)
        .await
        .expect("Should create user in tenant B");

    // Verify tenant isolation - tenant A cannot access tenant B's resources
    let user_b_id = user_b.get_id().expect("User B should have ID");
    let tenant_a_cannot_access_b = provider
        .get_resource("User", user_b_id, &context_a)
        .await
        .expect("Should not error");
    assert!(
        tenant_a_cannot_access_b.is_none(),
        "Tenant A should not access Tenant B's resources"
    );

    // Verify tenant isolation - tenant B cannot access tenant A's resources
    let user_a_id = user_a.get_id().expect("User A should have ID");
    let tenant_b_cannot_access_a = provider
        .get_resource("User", user_a_id, &context_b)
        .await
        .expect("Should not error");
    assert!(
        tenant_b_cannot_access_a.is_none(),
        "Tenant B should not access Tenant A's resources"
    );

    // Verify each tenant can access their own resources
    let tenant_a_user = provider
        .get_resource("User", user_a_id, &context_a)
        .await
        .expect("Should not error")
        .expect("Should find user in tenant A");
    assert_eq!(tenant_a_user.get_id(), user_a.get_id());

    let tenant_b_user = provider
        .get_resource("User", user_b_id, &context_b)
        .await
        .expect("Should not error")
        .expect("Should find user in tenant B");
    assert_eq!(tenant_b_user.get_id(), user_b.get_id());
}

/// Test SCIM client authentication and authorization per tenant
#[tokio::test]
async fn test_scim_client_authentication() {
    let resolver = StaticTenantResolver::new();

    // Set up tenant contexts for different SCIM clients
    let tenant_a_context = TenantContext::new("tenant-a".to_string(), "okta-client".to_string());
    let tenant_b_context = TenantContext::new("tenant-b".to_string(), "azure-client".to_string());

    resolver
        .add_tenant("api_key_okta_123", tenant_a_context.clone())
        .await;
    resolver
        .add_tenant("api_key_azure_456", tenant_b_context.clone())
        .await;

    // Test successful tenant resolution for SCIM clients
    let resolved_a = resolver
        .resolve_tenant("api_key_okta_123")
        .await
        .expect("Should resolve Okta client to tenant A");
    assert_eq!(resolved_a.tenant_id, "tenant-a");
    assert_eq!(resolved_a.client_id, "okta-client");

    let resolved_b = resolver
        .resolve_tenant("api_key_azure_456")
        .await
        .expect("Should resolve Azure client to tenant B");
    assert_eq!(resolved_b.tenant_id, "tenant-b");
    assert_eq!(resolved_b.client_id, "azure-client");

    // Test that invalid API keys are rejected
    let invalid_resolution = resolver.resolve_tenant("invalid_api_key").await;
    assert!(
        invalid_resolution.is_err(),
        "Invalid API key should be rejected"
    );
}

/// Test SCIM rate limiting per tenant
#[tokio::test]
async fn test_scim_rate_limiting() {
    let config = ScimTenantConfiguration::builder("tenant-a".to_string())
        .with_scim_rate_limit(10, Duration::from_secs(60)) // Very low limit for testing
        .build()
        .expect("Valid config");

    // Test rate limit checking
    assert!(
        !config.is_rate_limited("create", 5),
        "Should not be rate limited at 5 requests"
    );
    assert!(
        config.is_rate_limited("create", 10),
        "Should be rate limited at 10 requests"
    );
    assert!(
        config.is_rate_limited("create", 15),
        "Should be rate limited at 15 requests"
    );
}

/// Test SCIM schema extensions per tenant
#[tokio::test]
async fn test_scim_schema_extensions() {
    let config = ScimTenantConfiguration::builder("enterprise-tenant".to_string())
        .with_schema_extension(
            "urn:ietf:params:scim:schemas:extension:enterprise:2.0:User",
            true,
        )
        .with_schema_extension("urn:example:custom:extension", false)
        .build()
        .expect("Valid config");

    // Test schema extension checking
    assert!(
        config.has_schema_extension("urn:ietf:params:scim:schemas:extension:enterprise:2.0:User"),
        "Should have enterprise extension"
    );
    assert!(
        config.has_schema_extension("urn:example:custom:extension"),
        "Should have custom extension"
    );
    assert!(
        !config.has_schema_extension("urn:missing:extension"),
        "Should not have missing extension"
    );
}

/// Test SCIM audit configuration per tenant
#[tokio::test]
async fn test_scim_audit_configuration() {
    let config = ScimTenantConfiguration::builder("audit-tenant".to_string())
        .enable_scim_audit_log()
        .build()
        .expect("Valid config");

    // Verify SCIM audit settings
    assert!(config.audit_config.enabled, "Audit should be enabled");
    assert!(
        config
            .audit_config
            .audited_operations
            .contains(&scim_server::ScimOperation::Create),
        "Should audit create operations"
    );
    assert!(
        config
            .audit_config
            .audited_operations
            .contains(&scim_server::ScimOperation::Update),
        "Should audit update operations"
    );
    assert!(
        config
            .audit_config
            .audited_operations
            .contains(&scim_server::ScimOperation::Delete),
        "Should audit delete operations"
    );

    // Verify retention period (should be 90 days by default)
    assert_eq!(
        config.audit_config.retention_period,
        Duration::from_secs(90 * 24 * 60 * 60),
        "Should have 90-day retention period"
    );
}
