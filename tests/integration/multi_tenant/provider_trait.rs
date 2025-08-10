//! Multi-tenant resource provider integration tests.
//!
//! This module contains comprehensive tests for multi-tenant resource providers
//! using the unified ResourceProvider trait. The tests verify tenant isolation,
//! proper scoping, and all CRUD operations within multi-tenant contexts.

use scim_server::resource::core::{
    ListQuery, RequestContext, Resource, ResourceBuilder, TenantContext,
};
use scim_server::resource::provider::ResourceProvider;
use scim_server::resource::value_objects::{ExternalId, ResourceId, UserName};
use serde_json::{Map, Value, json};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Test query structure for list operations
#[derive(Debug, Clone)]
pub struct TestListQuery {
    pub count: Option<usize>,
    pub start_index: Option<usize>,
    pub filter: Option<String>,
    pub attributes: Option<Vec<String>>,
    pub excluded_attributes: Option<Vec<String>>,
}

impl TestListQuery {
    pub fn new() -> Self {
        Self {
            count: None,
            start_index: None,
            filter: None,
            attributes: None,
            excluded_attributes: None,
        }
    }

    pub fn with_count(mut self, count: usize) -> Self {
        self.count = Some(count);
        self
    }

    pub fn with_filter(mut self, filter: String) -> Self {
        self.filter = Some(filter);
        self
    }
}

/// Test multi-tenant provider implementation using the unified ResourceProvider trait
#[derive(Debug)]
pub struct TestMultiTenantProvider {
    /// Resources organized by tenant_id -> resource_type -> resource_id -> resource
    resources: Arc<RwLock<HashMap<String, HashMap<String, HashMap<String, Resource>>>>>,
    next_id: Arc<RwLock<u64>>,
}

impl TestMultiTenantProvider {
    pub fn new() -> Self {
        Self {
            resources: Arc::new(RwLock::new(HashMap::new())),
            next_id: Arc::new(RwLock::new(1)),
        }
    }

    async fn generate_id(&self) -> String {
        let mut counter = self.next_id.write().await;
        let id = *counter;
        *counter += 1;
        format!("test-{:06}", id)
    }

    async fn ensure_tenant_exists(&self, tenant_id: &str) {
        let mut resources = self.resources.write().await;
        resources
            .entry(tenant_id.to_string())
            .or_insert_with(HashMap::new);
    }

    async fn ensure_resource_type_exists(&self, tenant_id: &str, resource_type: &str) {
        self.ensure_tenant_exists(tenant_id).await;
        let mut resources = self.resources.write().await;
        resources
            .get_mut(tenant_id)
            .unwrap()
            .entry(resource_type.to_string())
            .or_insert_with(HashMap::new);
    }

    fn get_tenant_id_from_context(context: &RequestContext) -> Result<String, TestProviderError> {
        match &context.tenant_context {
            Some(tenant_context) => Ok(tenant_context.tenant_id.clone()),
            None => Err(TestProviderError::InvalidTenantContext {
                expected: "Some(tenant_context)".to_string(),
                actual: "None".to_string(),
            }),
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum TestProviderError {
    #[error("Resource not found: tenant={tenant_id}, type={resource_type}, id={id}")]
    ResourceNotFound {
        tenant_id: String,
        resource_type: String,
        id: String,
    },
    #[error("Tenant not found: {tenant_id}")]
    TenantNotFound { tenant_id: String },

    #[error("Duplicate resource: tenant={tenant_id}, type={resource_type}, {attribute}={value}")]
    DuplicateResource {
        tenant_id: String,
        resource_type: String,
        attribute: String,
        value: String,
    },
    #[error("Invalid tenant context: expected {expected}, found {actual}")]
    InvalidTenantContext { expected: String, actual: String },

    #[error("Validation error: {message}")]
    ValidationError { message: String },
}

impl ResourceProvider for TestMultiTenantProvider {
    type Error = TestProviderError;

    async fn create_resource(
        &self,
        resource_type: &str,
        data: Value,
        context: &RequestContext,
    ) -> Result<Resource, Self::Error> {
        let tenant_id = Self::get_tenant_id_from_context(context)?;

        self.ensure_resource_type_exists(&tenant_id, resource_type)
            .await;

        // Check for duplicate usernames within tenant
        if resource_type == "User" {
            if let Some(username) = data.get("userName").and_then(|v| v.as_str()) {
                let resources = self.resources.read().await;
                if let Some(tenant_resources) = resources.get(&tenant_id) {
                    if let Some(user_resources) = tenant_resources.get("User") {
                        for resource in user_resources.values() {
                            if let Some(existing_username) = &resource.user_name {
                                if existing_username.as_str() == username {
                                    return Err(TestProviderError::DuplicateResource {
                                        tenant_id: tenant_id.clone(),
                                        resource_type: resource_type.to_string(),
                                        attribute: "userName".to_string(),
                                        value: username.to_string(),
                                    });
                                }
                            }
                        }
                    }
                }
            }
        }

        let id = self.generate_id().await;

        // Build the resource using ResourceBuilder
        let mut builder = ResourceBuilder::new(resource_type.to_string());

        // Set ID
        if let Ok(resource_id) = ResourceId::new(id.clone()) {
            builder = builder.with_id(resource_id);
        }

        // Set username for User resources
        if resource_type == "User" {
            if let Some(username) = data.get("userName").and_then(|v| v.as_str()) {
                if let Ok(user_name) = UserName::new(username.to_string()) {
                    builder = builder.with_username(user_name);
                }
            }
        }

        // Set external ID if provided
        if let Some(external_id) = data.get("externalId").and_then(|v| v.as_str()) {
            if let Ok(ext_id) = ExternalId::new(external_id.to_string()) {
                builder = builder.with_external_id(ext_id);
            }
        }

        // Add remaining attributes
        let mut attributes = Map::new();
        for (key, value) in data.as_object().unwrap_or(&Map::new()) {
            match key.as_str() {
                "userName" | "externalId" | "id" => {
                    // These are handled by value objects, skip
                }
                _ => {
                    attributes.insert(key.clone(), value.clone());
                }
            }
        }
        builder = builder.with_attributes(attributes);

        let resource = builder
            .build_with_meta("https://example.com/scim/v2")
            .map_err(|e| TestProviderError::ValidationError {
                message: format!("Failed to build resource: {}", e),
            })?;

        let mut resources = self.resources.write().await;
        resources
            .get_mut(&tenant_id)
            .unwrap()
            .get_mut(resource_type)
            .unwrap()
            .insert(id, resource.clone());

        Ok(resource)
    }

    async fn get_resource(
        &self,
        resource_type: &str,
        id: &str,
        context: &RequestContext,
    ) -> Result<Option<Resource>, Self::Error> {
        let tenant_id = Self::get_tenant_id_from_context(context)?;

        let resources = self.resources.read().await;
        let result = resources
            .get(&tenant_id)
            .and_then(|tenant| tenant.get(resource_type))
            .and_then(|resources| resources.get(id))
            .cloned();

        Ok(result)
    }

    async fn update_resource(
        &self,
        resource_type: &str,
        id: &str,
        data: Value,
        context: &RequestContext,
    ) -> Result<Resource, Self::Error> {
        let tenant_id = Self::get_tenant_id_from_context(context)?;

        let mut resources = self.resources.write().await;

        // Check if resource exists - return ResourceNotFound if tenant, type, or id doesn't exist
        let resource = match resources
            .get_mut(&tenant_id)
            .and_then(|tenant_resources| tenant_resources.get_mut(resource_type))
            .and_then(|type_resources| type_resources.get_mut(id))
        {
            Some(resource) => resource,
            None => {
                return Err(TestProviderError::ResourceNotFound {
                    tenant_id: tenant_id.clone(),
                    resource_type: resource_type.to_string(),
                    id: id.to_string(),
                });
            }
        };

        // Update the resource using ResourceBuilder
        let mut builder = ResourceBuilder::new(resource_type.to_string());

        // Preserve existing ID
        if let Some(existing_id) = &resource.id {
            builder = builder.with_id(existing_id.clone());
        }

        // Update username for User resources
        if resource_type == "User" {
            if let Some(username) = data.get("userName").and_then(|v| v.as_str()) {
                if let Ok(user_name) = UserName::new(username.to_string()) {
                    builder = builder.with_username(user_name);
                }
            } else if let Some(existing_username) = &resource.user_name {
                // Preserve existing username if not in update data
                builder = builder.with_username(existing_username.clone());
            }
        }

        // Update external ID if provided, otherwise preserve existing
        if let Some(external_id) = data.get("externalId").and_then(|v| v.as_str()) {
            if let Ok(ext_id) = ExternalId::new(external_id.to_string()) {
                builder = builder.with_external_id(ext_id);
            }
        } else if let Some(existing_external_id) = &resource.external_id {
            builder = builder.with_external_id(existing_external_id.clone());
        }

        // Update other attributes
        let mut attributes = resource.attributes.clone();
        for (key, value) in data.as_object().unwrap_or(&Map::new()) {
            match key.as_str() {
                "userName" | "externalId" | "id" => {
                    // These are handled by value objects, skip
                }
                _ => {
                    attributes.insert(key.clone(), value.clone());
                }
            }
        }
        builder = builder.with_attributes(attributes);

        let updated_resource = builder
            .build_with_meta("https://example.com/scim/v2")
            .map_err(|e| TestProviderError::ValidationError {
                message: format!("Failed to update resource: {}", e),
            })?;

        *resource = updated_resource.clone();
        Ok(updated_resource)
    }

    async fn delete_resource(
        &self,
        resource_type: &str,
        id: &str,
        context: &RequestContext,
    ) -> Result<(), Self::Error> {
        let tenant_id = Self::get_tenant_id_from_context(context)?;

        let mut resources = self.resources.write().await;

        // If tenant doesn't exist or resource type doesn't exist, return ResourceNotFound
        if let Some(tenant_resources) = resources.get_mut(&tenant_id) {
            if let Some(type_resources) = tenant_resources.get_mut(resource_type) {
                if type_resources.remove(id).is_some() {
                    return Ok(());
                }
            }
        }

        // Resource not found - either tenant, type, or id doesn't exist
        Err(TestProviderError::ResourceNotFound {
            tenant_id: tenant_id.clone(),
            resource_type: resource_type.to_string(),
            id: id.to_string(),
        })
    }

    async fn list_resources(
        &self,
        resource_type: &str,
        _query: Option<&ListQuery>,
        context: &RequestContext,
    ) -> Result<Vec<Resource>, Self::Error> {
        let tenant_id = Self::get_tenant_id_from_context(context)?;

        let resources = self.resources.read().await;
        let result = resources
            .get(&tenant_id)
            .and_then(|tenant| tenant.get(resource_type))
            .map(|resources| resources.values().cloned().collect())
            .unwrap_or_default();

        Ok(result)
    }

    async fn find_resource_by_attribute(
        &self,
        resource_type: &str,
        attribute: &str,
        value: &Value,
        context: &RequestContext,
    ) -> Result<Option<Resource>, Self::Error> {
        let tenant_id = Self::get_tenant_id_from_context(context)?;

        let resources = self.resources.read().await;
        if let Some(tenant_resources) = resources.get(&tenant_id) {
            if let Some(type_resources) = tenant_resources.get(resource_type) {
                for resource in type_resources.values() {
                    // Check value object fields first
                    match attribute {
                        "userName" => {
                            if let Some(username) = &resource.user_name {
                                if let Some(search_str) = value.as_str() {
                                    if username.as_str() == search_str {
                                        return Ok(Some(resource.clone()));
                                    }
                                }
                            }
                        }
                        "id" => {
                            if let Some(id) = &resource.id {
                                if let Some(search_str) = value.as_str() {
                                    if id.as_str() == search_str {
                                        return Ok(Some(resource.clone()));
                                    }
                                }
                            }
                        }
                        "externalId" => {
                            if let Some(external_id) = &resource.external_id {
                                if let Some(search_str) = value.as_str() {
                                    if external_id.as_str() == search_str {
                                        return Ok(Some(resource.clone()));
                                    }
                                }
                            }
                        }
                        _ => {
                            // Check in extended attributes
                            if let Some(attr_value) = resource.attributes.get(attribute) {
                                if attr_value == value {
                                    return Ok(Some(resource.clone()));
                                }
                            }
                        }
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
        let tenant_id = Self::get_tenant_id_from_context(context)?;

        let resources = self.resources.read().await;
        let exists = resources
            .get(&tenant_id)
            .and_then(|tenant| tenant.get(resource_type))
            .map(|resources| resources.contains_key(id))
            .unwrap_or(false);

        Ok(exists)
    }
}

fn create_test_context(tenant_id: &str) -> RequestContext {
    let tenant_context = TenantContext::new(tenant_id.to_string(), "test-client".to_string());
    RequestContext::with_tenant("test-request".to_string(), tenant_context)
}

fn create_test_user(username: &str) -> Value {
    json!({
        "userName": username,
        "name": {
            "familyName": "Doe",
            "givenName": "John"
        },
        "emails": [{
            "value": format!("{}@example.com", username),
            "primary": true
        }]
    })
}

#[cfg(test)]
mod provider_trait_multi_tenant_tests {
    use super::*;

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
        assert_eq!(updated.attributes["name"]["familyName"], "Smith");
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
