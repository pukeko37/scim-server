//! Multi-tenant resource provider integration tests.
//!
//! This module contains comprehensive tests for multi-tenant resource providers
//! using the unified ResourceProvider trait. The tests verify tenant isolation,
//! proper scoping, and all CRUD operations within multi-tenant contexts.

use scim_server::ResourceProvider;
use scim_server::resource::value_objects::{ExternalId, ResourceId, UserName};
use scim_server::resource::{ListQuery, RequestContext, Resource, builder::ResourceBuilder};
use scim_server::resource::{version::RawVersion, versioned::VersionedResource};
use serde_json::{Map, Value};
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
    ) -> Result<VersionedResource, Self::Error> {
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

        Ok(VersionedResource::new(resource))
    }

    async fn get_resource(
        &self,
        resource_type: &str,
        id: &str,
        context: &RequestContext,
    ) -> Result<Option<VersionedResource>, Self::Error> {
        let tenant_id = Self::get_tenant_id_from_context(context)?;

        let resources = self.resources.read().await;
        let result = resources
            .get(&tenant_id)
            .and_then(|tenant| tenant.get(resource_type))
            .and_then(|resources| resources.get(id))
            .map(|resource| VersionedResource::new(resource.clone()));

        Ok(result)
    }

    async fn update_resource(
        &self,
        resource_type: &str,
        id: &str,
        data: Value,
        _expected_version: Option<&RawVersion>,
        context: &RequestContext,
    ) -> Result<VersionedResource, Self::Error> {
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
        Ok(VersionedResource::new(updated_resource))
    }

    async fn delete_resource(
        &self,
        resource_type: &str,
        id: &str,
        _expected_version: Option<&RawVersion>,
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
    ) -> Result<Vec<VersionedResource>, Self::Error> {
        let tenant_id = Self::get_tenant_id_from_context(context)?;

        let resources = self.resources.read().await;
        let result = resources
            .get(&tenant_id)
            .and_then(|tenant| tenant.get(resource_type))
            .map(|resources| {
                resources
                    .values()
                    .map(|resource| VersionedResource::new(resource.clone()))
                    .collect()
            })
            .unwrap_or_default();

        Ok(result)
    }

    async fn find_resources_by_attribute(
        &self,
        resource_type: &str,
        attribute: &str,
        value: &str,
        context: &RequestContext,
    ) -> Result<Vec<VersionedResource>, Self::Error> {
        let tenant_id = Self::get_tenant_id_from_context(context)?;

        let resources = self.resources.read().await;
        let mut results = Vec::new();
        if let Some(tenant_resources) = resources.get(&tenant_id) {
            if let Some(type_resources) = tenant_resources.get(resource_type) {
                for resource in type_resources.values() {
                    let matches = match attribute {
                        "userName" => resource
                            .user_name
                            .as_ref()
                            .map(|username| username.as_str() == value)
                            .unwrap_or(false),
                        "id" => resource
                            .id
                            .as_ref()
                            .map(|id| id.as_str() == value)
                            .unwrap_or(false),
                        "externalId" => resource
                            .external_id
                            .as_ref()
                            .map(|external_id| external_id.as_str() == value)
                            .unwrap_or(false),
                        _ => {
                            // Check in extended attributes
                            resource
                                .attributes
                                .get(attribute)
                                .and_then(|attr_value| attr_value.as_str())
                                .map(|attr_str| attr_str == value)
                                .unwrap_or(false)
                        }
                    };

                    if matches {
                        results.push(VersionedResource::new(resource.clone()));
                    }
                }
            }
        }
        Ok(results)
    }

    async fn patch_resource(
        &self,
        resource_type: &str,
        id: &str,
        _patch_request: &Value,
        _expected_version: Option<&RawVersion>,
        context: &RequestContext,
    ) -> Result<VersionedResource, Self::Error> {
        // Simple implementation: just return the existing resource
        self.get_resource(resource_type, id, context)
            .await?
            .ok_or_else(|| TestProviderError::ResourceNotFound {
                tenant_id: Self::get_tenant_id_from_context(context).unwrap_or_default(),
                resource_type: resource_type.to_string(),
                id: id.to_string(),
            })
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
