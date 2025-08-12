//! Unit tests for the multi-tenant provider trait.

use scim_server::resource::{ListQuery, RequestContext, Resource, ResourceProvider, TenantContext};
use serde_json::{Value, json};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid;

/// Test error type for the test provider
#[derive(Debug, thiserror::Error)]
pub enum TestProviderError {
    #[error("Resource not found in tenant {tenant_id}: {resource_type} with ID {id}")]
    ResourceNotFound {
        tenant_id: String,
        resource_type: String,
        id: String,
    },
    #[error("Duplicate resource in tenant {tenant_id}: {attribute}={value} for {resource_type}")]
    DuplicateResource {
        tenant_id: String,
        resource_type: String,
        attribute: String,
        value: String,
    },
    #[error("Internal error: {message}")]
    Internal { message: String },
}

/// Test multi-tenant provider implementation
pub struct TestMultiTenantProvider {
    data: Arc<RwLock<HashMap<String, HashMap<String, HashMap<String, Resource>>>>>,
    // Structure: tenant_id -> resource_type -> resource_id -> Resource
}

impl TestMultiTenantProvider {
    pub fn new() -> Self {
        Self {
            data: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

impl ResourceProvider for TestMultiTenantProvider {
    type Error = TestProviderError;

    async fn create_resource(
        &self,
        resource_type: &str,
        data: Value,
        context: &RequestContext,
    ) -> Result<Resource, Self::Error> {
        let tenant_id = context.tenant_id().unwrap_or("default");
        let mut storage = self.data.write().await;

        // Generate unique ID
        let resource_id = format!("test-{}", uuid::Uuid::new_v4());

        // Create resource from JSON data
        let mut resource_data = data;
        resource_data["id"] = json!(resource_id);

        let resource =
            Resource::from_json(resource_type.to_string(), resource_data).map_err(|e| {
                TestProviderError::Internal {
                    message: format!("Failed to create resource: {}", e),
                }
            })?;

        // Check for duplicates within tenant
        let tenant_data = storage.entry(tenant_id.to_string()).or_default();
        let resource_type_data = tenant_data.entry(resource_type.to_string()).or_default();

        // Check for duplicate userName within tenant
        if let Some(username) = resource.get_username() {
            for existing_resource in resource_type_data.values() {
                if existing_resource.get_username() == Some(username) {
                    return Err(TestProviderError::DuplicateResource {
                        tenant_id: tenant_id.to_string(),
                        resource_type: resource_type.to_string(),
                        attribute: "userName".to_string(),
                        value: username.to_string(),
                    });
                }
            }
        }

        resource_type_data.insert(resource_id, resource.clone());
        Ok(resource)
    }

    async fn get_resource(
        &self,
        resource_type: &str,
        id: &str,
        context: &RequestContext,
    ) -> Result<Option<Resource>, Self::Error> {
        let tenant_id = context.tenant_id().unwrap_or("default");
        let storage = self.data.read().await;

        Ok(storage
            .get(tenant_id)
            .and_then(|tenant_data| tenant_data.get(resource_type))
            .and_then(|resource_type_data| resource_type_data.get(id))
            .cloned())
    }

    async fn update_resource(
        &self,
        resource_type: &str,
        id: &str,
        data: Value,
        context: &RequestContext,
    ) -> Result<Resource, Self::Error> {
        let tenant_id = context.tenant_id().unwrap_or("default");
        let mut storage = self.data.write().await;

        let tenant_data =
            storage
                .get_mut(tenant_id)
                .ok_or_else(|| TestProviderError::ResourceNotFound {
                    tenant_id: tenant_id.to_string(),
                    resource_type: resource_type.to_string(),
                    id: id.to_string(),
                })?;

        let resource_type_data = tenant_data.get_mut(resource_type).ok_or_else(|| {
            TestProviderError::ResourceNotFound {
                tenant_id: tenant_id.to_string(),
                resource_type: resource_type.to_string(),
                id: id.to_string(),
            }
        })?;

        let existing_resource =
            resource_type_data
                .get_mut(id)
                .ok_or_else(|| TestProviderError::ResourceNotFound {
                    tenant_id: tenant_id.to_string(),
                    resource_type: resource_type.to_string(),
                    id: id.to_string(),
                })?;

        // Merge update data with existing resource
        let mut resource_data =
            existing_resource
                .to_json()
                .map_err(|e| TestProviderError::Internal {
                    message: format!("Failed to serialize existing resource: {}", e),
                })?;

        if let Value::Object(ref mut existing_obj) = resource_data {
            if let Value::Object(update_obj) = data {
                for (key, value) in update_obj {
                    existing_obj.insert(key, value);
                }
            }
        }

        let updated_resource = Resource::from_json(resource_type.to_string(), resource_data)
            .map_err(|e| TestProviderError::Internal {
                message: format!("Failed to create updated resource: {}", e),
            })?;

        *existing_resource = updated_resource.clone();
        Ok(updated_resource)
    }

    async fn delete_resource(
        &self,
        resource_type: &str,
        id: &str,
        context: &RequestContext,
    ) -> Result<(), Self::Error> {
        let tenant_id = context.tenant_id().unwrap_or("default");
        let mut storage = self.data.write().await;

        let tenant_data =
            storage
                .get_mut(tenant_id)
                .ok_or_else(|| TestProviderError::ResourceNotFound {
                    tenant_id: tenant_id.to_string(),
                    resource_type: resource_type.to_string(),
                    id: id.to_string(),
                })?;

        let resource_type_data = tenant_data.get_mut(resource_type).ok_or_else(|| {
            TestProviderError::ResourceNotFound {
                tenant_id: tenant_id.to_string(),
                resource_type: resource_type.to_string(),
                id: id.to_string(),
            }
        })?;

        resource_type_data
            .remove(id)
            .ok_or_else(|| TestProviderError::ResourceNotFound {
                tenant_id: tenant_id.to_string(),
                resource_type: resource_type.to_string(),
                id: id.to_string(),
            })?;

        Ok(())
    }

    async fn list_resources(
        &self,
        resource_type: &str,
        _query: Option<&ListQuery>,
        context: &RequestContext,
    ) -> Result<Vec<Resource>, Self::Error> {
        let tenant_id = context.tenant_id().unwrap_or("default");
        let storage = self.data.read().await;

        Ok(storage
            .get(tenant_id)
            .and_then(|tenant_data| tenant_data.get(resource_type))
            .map(|resource_type_data| resource_type_data.values().cloned().collect())
            .unwrap_or_default())
    }

    async fn find_resource_by_attribute(
        &self,
        resource_type: &str,
        attribute: &str,
        value: &Value,
        context: &RequestContext,
    ) -> Result<Option<Resource>, Self::Error> {
        let tenant_id = context.tenant_id().unwrap_or("default");
        let storage = self.data.read().await;

        if let Some(tenant_data) = storage.get(tenant_id) {
            if let Some(resource_type_data) = tenant_data.get(resource_type) {
                for resource in resource_type_data.values() {
                    // Check core fields first
                    let found = match attribute {
                        "userName" => {
                            if let Some(username) = resource.get_username() {
                                value.as_str() == Some(username)
                            } else {
                                false
                            }
                        }
                        "id" => {
                            if let Some(id) = resource.get_id() {
                                value.as_str() == Some(id)
                            } else {
                                false
                            }
                        }
                        "externalId" => {
                            if let Some(external_id) = resource.get_external_id() {
                                value.as_str() == Some(external_id)
                            } else {
                                false
                            }
                        }
                        // Fall back to attributes map for other fields
                        _ => {
                            if let Some(attr_value) = resource.get_attribute(attribute) {
                                attr_value == value
                            } else {
                                false
                            }
                        }
                    };

                    if found {
                        return Ok(Some(resource.clone()));
                    }
                }
            }
        }

        Ok(None)
    }

    async fn resource_exists(
        &self,
        resource_type: &str,
        id: &str,
        context: &RequestContext,
    ) -> Result<bool, Self::Error> {
        let tenant_id = context.tenant_id().unwrap_or("default");
        let storage = self.data.read().await;

        Ok(storage
            .get(tenant_id)
            .and_then(|tenant_data| tenant_data.get(resource_type))
            .and_then(|resource_type_data| resource_type_data.get(id))
            .is_some())
    }
}

// Helper functions for test data creation
fn create_test_context(tenant_id: &str) -> RequestContext {
    let tenant_context = TenantContext::new(tenant_id.to_string(), format!("client-{}", tenant_id));
    RequestContext::with_tenant_generated_id(tenant_context)
}

fn create_test_user(username: &str) -> Value {
    json!({
        "userName": username,
        "name": {
            "familyName": "Doe",
            "givenName": "John"
        },
        "active": true
    })
}

#[tokio::test]
async fn test_create_resource_with_tenant_scope() {
    let provider = TestMultiTenantProvider::new();
    let context = create_test_context("tenant1");
    let user_data = create_test_user("testuser");

    let result = provider.create_resource("User", user_data, &context).await;
    assert!(result.is_ok());

    let resource = result.unwrap();
    assert_eq!(resource.resource_type, "User");
    assert!(
        resource
            .id
            .as_ref()
            .map(|id| id.as_str().starts_with("test-"))
            .unwrap_or(false)
    );
}

#[tokio::test]
async fn test_get_resource_with_tenant_scope() {
    let provider = TestMultiTenantProvider::new();
    let context = create_test_context("tenant1");
    let user_data = create_test_user("testuser");

    // Create resource
    let created = provider
        .create_resource("User", user_data, &context)
        .await
        .unwrap();

    // Get resource
    let id_str = created.id.as_ref().unwrap().as_str();
    let result = provider.get_resource("User", id_str, &context).await;
    assert!(result.is_ok());

    let retrieved = result.unwrap();
    assert!(retrieved.is_some());
    let resource = retrieved.unwrap();
    assert_eq!(resource.id, created.id);
    assert_eq!(resource.resource_type, "User");
}

#[tokio::test]
async fn test_update_resource_with_tenant_scope() {
    let provider = TestMultiTenantProvider::new();
    let context = create_test_context("tenant1");
    let user_data = create_test_user("testuser");

    // Create resource
    let created = provider
        .create_resource("User", user_data, &context)
        .await
        .unwrap();

    // Update resource
    let updated_data = json!({
        "userName": "updateduser",
        "name": {
            "familyName": "Smith",
            "givenName": "Jane"
        }
    });

    let id_str = created.id.as_ref().unwrap().as_str();
    let result = provider
        .update_resource("User", id_str, updated_data, &context)
        .await;
    assert!(result.is_ok());

    let updated = result.unwrap();
    assert_eq!(updated.user_name.as_ref().unwrap().as_str(), "updateduser");
    assert_eq!(
        updated.name.as_ref().unwrap().family_name.as_ref().unwrap(),
        "Smith"
    );
}

#[tokio::test]
async fn test_delete_resource_with_tenant_scope() {
    let provider = TestMultiTenantProvider::new();
    let context = create_test_context("tenant1");
    let user_data = create_test_user("testuser");

    // Create resource
    let created = provider
        .create_resource("User", user_data, &context)
        .await
        .unwrap();

    // Delete resource
    let id_str = created.id.as_ref().unwrap().as_str();
    let result = provider.delete_resource("User", id_str, &context).await;
    assert!(result.is_ok());

    // Verify deletion
    let get_result = provider
        .get_resource("User", id_str, &context)
        .await
        .unwrap();
    assert!(get_result.is_none());
}

#[tokio::test]
async fn test_list_resources_with_tenant_scope() {
    let provider = TestMultiTenantProvider::new();
    let context = create_test_context("tenant1");

    // Create multiple resources
    let user1 = create_test_user("user1");
    let user2 = create_test_user("user2");

    provider
        .create_resource("User", user1, &context)
        .await
        .unwrap();
    provider
        .create_resource("User", user2, &context)
        .await
        .unwrap();

    // List resources
    let result = provider.list_resources("User", None, &context).await;
    assert!(result.is_ok());

    let resources = result.unwrap();
    assert_eq!(resources.len(), 2);
}

#[tokio::test]
async fn test_tenant_isolation_in_create_and_get() {
    let provider = TestMultiTenantProvider::new();
    let context1 = create_test_context("tenant1");
    let context2 = create_test_context("tenant2");

    let user_data = create_test_user("testuser");

    // Create resource in tenant1
    let created = provider
        .create_resource("User", user_data, &context1)
        .await
        .unwrap();

    // Try to get resource from tenant2 (should not find it)
    let id_str = created.id.as_ref().unwrap().as_str();
    let result = provider.get_resource("User", id_str, &context2).await;
    assert!(result.is_ok());
    assert!(result.unwrap().is_none());

    // Verify resource exists in tenant1
    let result = provider.get_resource("User", id_str, &context1).await;
    assert!(result.is_ok());
    assert!(result.unwrap().is_some());
}

#[tokio::test]
async fn test_tenant_isolation_in_list_operations() {
    let provider = TestMultiTenantProvider::new();
    let context1 = create_test_context("tenant1");
    let context2 = create_test_context("tenant2");

    // Create resources in both tenants
    let user1 = create_test_user("user1");
    let user2 = create_test_user("user2");
    let user3 = create_test_user("user3");

    provider
        .create_resource("User", user1, &context1)
        .await
        .unwrap();
    provider
        .create_resource("User", user2, &context1)
        .await
        .unwrap();
    provider
        .create_resource("User", user3, &context2)
        .await
        .unwrap();

    // List resources for tenant1
    let result1 = provider
        .list_resources("User", None, &context1)
        .await
        .unwrap();
    assert_eq!(result1.len(), 2);

    // List resources for tenant2
    let result2 = provider
        .list_resources("User", None, &context2)
        .await
        .unwrap();
    assert_eq!(result2.len(), 1);
}

#[tokio::test]
async fn test_duplicate_prevention_within_tenant() {
    let provider = TestMultiTenantProvider::new();
    let context = create_test_context("tenant1");

    let user_data = create_test_user("testuser");

    // Create first resource
    let result1 = provider
        .create_resource("User", user_data.clone(), &context)
        .await;
    assert!(result1.is_ok());

    // Try to create duplicate resource (should fail)
    let result2 = provider.create_resource("User", user_data, &context).await;
    assert!(result2.is_err());

    if let Err(TestProviderError::DuplicateResource {
        tenant_id,
        attribute,
        value,
        ..
    }) = result2
    {
        assert_eq!(tenant_id, "tenant1");
        assert_eq!(attribute, "userName");
        assert_eq!(value, "testuser");
    } else {
        panic!("Expected DuplicateResource error");
    }
}

#[tokio::test]
async fn test_same_username_allowed_across_tenants() {
    let provider = TestMultiTenantProvider::new();
    let context1 = create_test_context("tenant1");
    let context2 = create_test_context("tenant2");

    let user_data = create_test_user("testuser");

    // Create resource in tenant1
    let result1 = provider
        .create_resource("User", user_data.clone(), &context1)
        .await;
    assert!(result1.is_ok());

    // Create resource with same username in tenant2 (should succeed)
    let result2 = provider.create_resource("User", user_data, &context2).await;
    assert!(result2.is_ok());

    let user1 = result1.unwrap();
    let user2 = result2.unwrap();

    assert_ne!(user1.id, user2.id);
    assert_eq!(
        user1.user_name.as_ref().unwrap().as_str(),
        user2.user_name.as_ref().unwrap().as_str()
    );
}

#[tokio::test]
async fn test_resource_ids_unique_within_tenant() {
    let provider = TestMultiTenantProvider::new();
    let context = create_test_context("tenant1");

    let user1 = create_test_user("user1");
    let user2 = create_test_user("user2");

    let result1 = provider
        .create_resource("User", user1, &context)
        .await
        .unwrap();
    let result2 = provider
        .create_resource("User", user2, &context)
        .await
        .unwrap();

    assert_ne!(result1.id, result2.id);
}

#[tokio::test]
async fn test_resource_exists_tenant_scoped() {
    let provider = TestMultiTenantProvider::new();
    let context1 = create_test_context("tenant1");
    let context2 = create_test_context("tenant2");

    let user_data = create_test_user("testuser");

    // Create resource in tenant1
    let created = provider
        .create_resource("User", user_data, &context1)
        .await
        .unwrap();

    // Check existence in tenant1 (should exist)
    let id_str = created.id.as_ref().unwrap().as_str();
    let exists1 = provider
        .resource_exists("User", id_str, &context1)
        .await
        .unwrap();
    assert!(exists1);

    // Check existence in tenant2 (should not exist)
    let exists2 = provider
        .resource_exists("User", id_str, &context2)
        .await
        .unwrap();
    assert!(!exists2);
}

#[tokio::test]
async fn test_find_resource_by_attribute_tenant_scoped() {
    let provider = TestMultiTenantProvider::new();
    let context1 = create_test_context("tenant1");
    let context2 = create_test_context("tenant2");

    let user_data = create_test_user("testuser");

    // Create resource in tenant1
    provider
        .create_resource("User", user_data, &context1)
        .await
        .unwrap();

    // Find by attribute in tenant1 (should find)
    let result1 = provider
        .find_resource_by_attribute("User", "userName", &json!("testuser"), &context1)
        .await
        .unwrap();
    assert!(result1.is_some());

    // Find by attribute in tenant2 (should not find)
    let result2 = provider
        .find_resource_by_attribute("User", "userName", &json!("testuser"), &context2)
        .await
        .unwrap();
    assert!(result2.is_none());
}

#[tokio::test]
async fn test_find_resource_by_attribute_not_found_in_tenant() {
    let provider = TestMultiTenantProvider::new();
    let context = create_test_context("tenant1");

    // Search for non-existent resource
    let result = provider
        .find_resource_by_attribute("User", "userName", &json!("nonexistent"), &context)
        .await
        .unwrap();
    assert!(result.is_none());
}

#[tokio::test]
async fn test_get_nonexistent_resource() {
    let provider = TestMultiTenantProvider::new();
    let context = create_test_context("tenant1");

    let result = provider
        .get_resource("User", "nonexistent", &context)
        .await
        .unwrap();
    assert!(result.is_none());
}

#[tokio::test]
async fn test_update_nonexistent_resource() {
    let provider = TestMultiTenantProvider::new();
    let context = create_test_context("tenant1");

    let updated_data = json!({
        "userName": "updateduser"
    });

    let result = provider
        .update_resource("User", "nonexistent", updated_data, &context)
        .await;
    assert!(result.is_err());

    if let Err(TestProviderError::ResourceNotFound {
        tenant_id,
        resource_type,
        id,
    }) = result
    {
        assert_eq!(tenant_id, "tenant1");
        assert_eq!(resource_type, "User");
        assert_eq!(id, "nonexistent");
    } else {
        panic!("Expected ResourceNotFound error");
    }
}

#[tokio::test]
async fn test_delete_nonexistent_resource() {
    let provider = TestMultiTenantProvider::new();
    let context = create_test_context("tenant1");

    let result = provider
        .delete_resource("User", "nonexistent", &context)
        .await;
    assert!(result.is_err());

    if let Err(TestProviderError::ResourceNotFound {
        tenant_id,
        resource_type,
        id,
    }) = result
    {
        assert_eq!(tenant_id, "tenant1");
        assert_eq!(resource_type, "User");
        assert_eq!(id, "nonexistent");
    } else {
        panic!("Expected ResourceNotFound error");
    }
}

#[tokio::test]
async fn test_empty_tenant_list_resources() {
    let provider = TestMultiTenantProvider::new();
    let context = create_test_context("tenant1");

    let result = provider
        .list_resources("User", None, &context)
        .await
        .unwrap();
    assert!(result.is_empty());
}

#[tokio::test]
async fn test_multiple_tenants_concurrent_operations() {
    let provider = Arc::new(TestMultiTenantProvider::new());
    let context1 = create_test_context("tenant1");
    let context2 = create_test_context("tenant2");
    let context3 = create_test_context("tenant3");

    let mut handles = vec![];

    // Spawn concurrent operations for different tenants
    for i in 1..=3 {
        let provider_clone = Arc::clone(&provider);
        let context = match i {
            1 => context1.clone(),
            2 => context2.clone(),
            _ => context3.clone(),
        };

        let handle = tokio::spawn(async move {
            let user_data = create_test_user(&format!("user{}", i));
            let result = provider_clone
                .create_resource("User", user_data, &context)
                .await;
            assert!(result.is_ok());
            result.unwrap()
        });

        handles.push(handle);
    }

    // Wait for all operations to complete
    let mut results = Vec::new();
    for handle in handles {
        results.push(handle.await.unwrap());
    }

    // Verify each tenant has exactly one user
    let count1 = provider
        .list_resources("User", None, &context1)
        .await
        .unwrap()
        .len();
    let count2 = provider
        .list_resources("User", None, &context2)
        .await
        .unwrap()
        .len();

    assert_eq!(count1, 1);
    assert_eq!(count2, 1);
}

#[tokio::test]
async fn test_provider_trait_documentation() {
    let provider = TestMultiTenantProvider::new();
    let context = create_test_context("tenant1");

    let user_data = create_test_user("testuser");

    // Create resource
    let created = provider
        .create_resource("User", user_data, &context)
        .await
        .unwrap();
    assert!(created.get_id().is_some());

    // Get resource
    let retrieved = provider
        .get_resource("User", created.get_id().unwrap(), &context)
        .await
        .unwrap();
    assert!(retrieved.is_some());

    // List resources
    let list = provider
        .list_resources("User", None, &context)
        .await
        .unwrap();
    assert_eq!(list.len(), 1);

    // Check existence
    let exists = provider
        .resource_exists("User", created.get_id().unwrap(), &context)
        .await
        .unwrap();
    assert!(exists);

    // Delete resource
    provider
        .delete_resource("User", created.get_id().unwrap(), &context)
        .await
        .unwrap();

    // Verify deletion
    let deleted = provider
        .get_resource("User", created.get_id().unwrap(), &context)
        .await
        .unwrap();
    assert!(deleted.is_none());
}

/// Test harness for provider trait validation
pub struct ProviderTestHarness;

impl ProviderTestHarness {
    /// Create a provider with pre-populated test data for comprehensive testing
    pub async fn create_populated_provider() -> TestMultiTenantProvider {
        let provider = TestMultiTenantProvider::new();

        // Create test data for multiple tenants
        let tenant1_context = create_test_context("tenant1");
        let tenant2_context = create_test_context("tenant2");

        // Add some test users
        for i in 1..=3 {
            let user_data = create_test_user(&format!("user{}", i));
            provider
                .create_resource("User", user_data, &tenant1_context)
                .await
                .unwrap();
        }

        for i in 1..=2 {
            let user_data = create_test_user(&format!("tenant2_user{}", i));
            provider
                .create_resource("User", user_data, &tenant2_context)
                .await
                .unwrap();
        }

        provider
    }

    /// Verify complete tenant isolation across all operations
    pub async fn verify_complete_tenant_isolation(provider: &TestMultiTenantProvider) {
        let tenant1_context = create_test_context("tenant1");
        let tenant2_context = create_test_context("tenant2");

        // Verify list isolation
        let tenant1_users = provider
            .list_resources("User", None, &tenant1_context)
            .await
            .unwrap();
        let tenant2_users = provider
            .list_resources("User", None, &tenant2_context)
            .await
            .unwrap();

        assert_eq!(tenant1_users.len(), 3);
        assert_eq!(tenant2_users.len(), 2);

        // Verify cross-tenant access fails
        if let Some(user) = tenant1_users.first() {
            let id_str = user.id.as_ref().unwrap().as_str();
            let cross_access = provider
                .get_resource("User", id_str, &tenant2_context)
                .await
                .unwrap();
            assert!(
                cross_access.is_none(),
                "Cross-tenant access should be blocked"
            );
        }
    }
}
