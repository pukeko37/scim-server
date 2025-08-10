//! In-Memory Resource Provider Implementation and Tests
//!
//! This module provides a comprehensive in-memory implementation of the
//! unified ResourceProvider interface for testing purposes. It includes:
//! - Full CRUD operations with proper tenant isolation
//! - Concurrent access support with proper locking
//! - Comprehensive test suite demonstrating functionality
//! - Performance testing utilities
//! - Configuration and validation testing

use crate::common::{create_multi_tenant_context, create_single_tenant_context};
use scim_server::resource::core::{ListQuery, RequestContext, Resource, ResourceBuilder};
use scim_server::resource::provider::ResourceProvider;
use scim_server::resource::value_objects::{ResourceId, UserName};
use serde_json::{Map, Value, json};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

// ============================================================================
// Production In-Memory Provider Implementation
// ============================================================================

/// High-performance in-memory resource provider with tenant isolation
#[derive(Debug)]
pub struct InMemoryProvider {
    /// Resources organized by tenant_id -> resource_type -> resource_id -> resource
    /// Single-tenant uses "default" as tenant_id
    resources: Arc<RwLock<HashMap<String, HashMap<String, HashMap<String, Resource>>>>>,
    next_id: Arc<RwLock<u64>>,
    config: InMemoryProviderConfig,
}

/// Configuration for the in-memory provider
#[derive(Debug, Clone)]
pub struct InMemoryProviderConfig {
    pub enable_metrics: bool,
    pub max_resources_per_tenant: Option<usize>,
    pub max_tenants: Option<usize>,
    pub enable_validation: bool,
}

impl Default for InMemoryProviderConfig {
    fn default() -> Self {
        Self {
            enable_metrics: false,
            max_resources_per_tenant: Some(10000),
            max_tenants: Some(1000),
            enable_validation: true,
        }
    }
}

impl InMemoryProviderConfig {
    /// Create configuration for testing (no limits)
    pub fn for_testing() -> Self {
        Self {
            enable_metrics: false,
            max_resources_per_tenant: None,
            max_tenants: None,
            enable_validation: true,
        }
    }

    /// Create configuration for performance testing
    pub fn for_performance() -> Self {
        Self {
            enable_metrics: true,
            max_resources_per_tenant: Some(100000),
            max_tenants: Some(10000),
            enable_validation: false,
        }
    }
}

/// Errors that can occur in the in-memory provider
#[derive(Debug, thiserror::Error)]
pub enum InMemoryProviderError {
    #[error("Resource not found: tenant={tenant_id}, type={resource_type}, id={id}")]
    ResourceNotFound {
        tenant_id: String,
        resource_type: String,
        id: String,
    },

    #[error("Tenant not found: {tenant_id}")]
    TenantNotFound { tenant_id: String },

    #[error(
        "Duplicate resource: tenant={tenant_id}, type={resource_type}, attribute={attribute}, value={value}"
    )]
    DuplicateResource {
        tenant_id: String,
        resource_type: String,
        attribute: String,
        value: String,
    },

    #[error("Tenant limit exceeded: max={max_tenants}")]
    TenantLimitExceeded { max_tenants: usize },

    #[error("Resource limit exceeded for tenant {tenant_id}: max={max_resources}")]
    ResourceLimitExceeded {
        tenant_id: String,
        max_resources: usize,
    },

    #[error("Validation error: {message}")]
    ValidationError { message: String },

    #[error("Concurrent access error: {message}")]
    ConcurrencyError { message: String },
}

impl InMemoryProvider {
    /// Create a new in-memory provider with default configuration
    pub fn new() -> Self {
        Self::with_config(InMemoryProviderConfig::default())
    }

    /// Create a new in-memory provider with custom configuration
    pub fn with_config(config: InMemoryProviderConfig) -> Self {
        Self {
            resources: Arc::new(RwLock::new(HashMap::new())),
            next_id: Arc::new(RwLock::new(1)),
            config,
        }
    }

    /// Create a provider optimized for testing
    pub fn for_testing() -> Self {
        Self::with_config(InMemoryProviderConfig::for_testing())
    }

    /// Generate a unique resource ID
    async fn generate_id(&self) -> String {
        let mut counter = self.next_id.write().await;
        let id = *counter;
        *counter += 1;
        format!("inmem-{:08}", id)
    }

    /// Extract tenant ID from context, defaulting to "default" for single-tenant
    fn get_tenant_id_from_context(context: &RequestContext) -> String {
        context.tenant_id().unwrap_or("default").to_string()
    }

    /// Ensure tenant exists in the data structure
    async fn ensure_tenant_exists(&self, tenant_id: &str) -> Result<(), InMemoryProviderError> {
        if let Some(max_tenants) = self.config.max_tenants {
            let resources = self.resources.read().await;
            if !resources.contains_key(tenant_id) && resources.len() >= max_tenants {
                return Err(InMemoryProviderError::TenantLimitExceeded { max_tenants });
            }
        }

        let mut resources = self.resources.write().await;
        resources
            .entry(tenant_id.to_string())
            .or_insert_with(HashMap::new);
        Ok(())
    }

    /// Ensure resource type exists for a tenant
    async fn ensure_resource_type_exists(
        &self,
        tenant_id: &str,
        resource_type: &str,
    ) -> Result<(), InMemoryProviderError> {
        self.ensure_tenant_exists(tenant_id).await?;

        let mut resources = self.resources.write().await;
        resources
            .get_mut(tenant_id)
            .unwrap()
            .entry(resource_type.to_string())
            .or_insert_with(HashMap::new);
        Ok(())
    }

    /// Check resource limits for a tenant
    async fn check_resource_limits(&self, tenant_id: &str) -> Result<(), InMemoryProviderError> {
        if let Some(max_resources) = self.config.max_resources_per_tenant {
            let resources = self.resources.read().await;
            if let Some(tenant_resources) = resources.get(tenant_id) {
                let total_resources: usize = tenant_resources.values().map(|r| r.len()).sum();
                if total_resources >= max_resources {
                    return Err(InMemoryProviderError::ResourceLimitExceeded {
                        tenant_id: tenant_id.to_string(),
                        max_resources,
                    });
                }
            }
        }
        Ok(())
    }

    /// Check for duplicate usernames within a tenant
    async fn check_username_uniqueness(
        &self,
        tenant_id: &str,
        username: &str,
        exclude_id: Option<&str>,
    ) -> Result<(), InMemoryProviderError> {
        let resources = self.resources.read().await;
        if let Some(tenant_resources) = resources.get(tenant_id) {
            if let Some(user_resources) = tenant_resources.get("User") {
                for (id, resource) in user_resources {
                    if Some(id.as_str()) == exclude_id {
                        continue;
                    }
                    if let Some(existing_username) = &resource.user_name {
                        if existing_username.as_str() == username {
                            return Err(InMemoryProviderError::DuplicateResource {
                                tenant_id: tenant_id.to_string(),
                                resource_type: "User".to_string(),
                                attribute: "userName".to_string(),
                                value: username.to_string(),
                            });
                        }
                    }
                }
            }
        }
        Ok(())
    }

    /// Get resource statistics for a tenant
    pub async fn get_tenant_statistics(&self, tenant_id: &str) -> TenantStatistics {
        let resources = self.resources.read().await;

        if let Some(tenant_resources) = resources.get(tenant_id) {
            let mut stats = TenantStatistics {
                tenant_id: tenant_id.to_string(),
                resource_counts: HashMap::new(),
                total_resources: 0,
            };

            for (resource_type, type_resources) in tenant_resources {
                let count = type_resources.len();
                stats.resource_counts.insert(resource_type.clone(), count);
                stats.total_resources += count;
            }

            stats
        } else {
            TenantStatistics {
                tenant_id: tenant_id.to_string(),
                resource_counts: HashMap::new(),
                total_resources: 0,
            }
        }
    }

    /// Clear all data (useful for testing)
    pub async fn clear_all_data(&self) {
        let mut resources = self.resources.write().await;
        resources.clear();

        let mut counter = self.next_id.write().await;
        *counter = 1;
    }
}

impl Default for InMemoryProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl ResourceProvider for InMemoryProvider {
    type Error = InMemoryProviderError;

    async fn create_resource(
        &self,
        resource_type: &str,
        data: Value,
        context: &RequestContext,
    ) -> Result<Resource, Self::Error> {
        let tenant_id = Self::get_tenant_id_from_context(context);

        // Check limits
        self.check_resource_limits(&tenant_id).await?;
        self.ensure_resource_type_exists(&tenant_id, resource_type)
            .await?;

        // Check for duplicate usernames if creating a User
        if resource_type == "User" {
            if let Some(username) = data.get("userName").and_then(|v| v.as_str()) {
                self.check_username_uniqueness(&tenant_id, username, None)
                    .await?;
            }
        }

        // Generate ID and build resource
        let id = self.generate_id().await;
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
            if let Ok(ext_id) =
                scim_server::resource::value_objects::ExternalId::new(external_id.to_string())
            {
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
            .map_err(|e| InMemoryProviderError::ValidationError {
                message: format!("Failed to build resource: {}", e),
            })?;

        // Store the resource
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
        let tenant_id = Self::get_tenant_id_from_context(context);

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
        let tenant_id = Self::get_tenant_id_from_context(context);

        // Check for duplicate usernames if updating a User
        if resource_type == "User" {
            if let Some(username) = data.get("userName").and_then(|v| v.as_str()) {
                self.check_username_uniqueness(&tenant_id, username, Some(id))
                    .await?;
            }
        }

        let mut resources = self.resources.write().await;
        let tenant_resources =
            resources
                .get_mut(&tenant_id)
                .ok_or_else(|| InMemoryProviderError::TenantNotFound {
                    tenant_id: tenant_id.clone(),
                })?;

        let type_resources = tenant_resources.get_mut(resource_type).ok_or_else(|| {
            InMemoryProviderError::ResourceNotFound {
                tenant_id: tenant_id.clone(),
                resource_type: resource_type.to_string(),
                id: id.to_string(),
            }
        })?;

        let resource =
            type_resources
                .get_mut(id)
                .ok_or_else(|| InMemoryProviderError::ResourceNotFound {
                    tenant_id: tenant_id.clone(),
                    resource_type: resource_type.to_string(),
                    id: id.to_string(),
                })?;

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
                builder = builder.with_username(existing_username.clone());
            }
        }

        // Update external ID if provided, otherwise preserve existing
        if let Some(external_id) = data.get("externalId").and_then(|v| v.as_str()) {
            if let Ok(ext_id) =
                scim_server::resource::value_objects::ExternalId::new(external_id.to_string())
            {
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
            .map_err(|e| InMemoryProviderError::ValidationError {
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
        let tenant_id = Self::get_tenant_id_from_context(context);

        let mut resources = self.resources.write().await;
        let tenant_resources =
            resources
                .get_mut(&tenant_id)
                .ok_or_else(|| InMemoryProviderError::TenantNotFound {
                    tenant_id: tenant_id.clone(),
                })?;

        let type_resources = tenant_resources.get_mut(resource_type).ok_or_else(|| {
            InMemoryProviderError::ResourceNotFound {
                tenant_id: tenant_id.clone(),
                resource_type: resource_type.to_string(),
                id: id.to_string(),
            }
        })?;

        if type_resources.remove(id).is_none() {
            return Err(InMemoryProviderError::ResourceNotFound {
                tenant_id: tenant_id.clone(),
                resource_type: resource_type.to_string(),
                id: id.to_string(),
            });
        }

        Ok(())
    }

    async fn list_resources(
        &self,
        resource_type: &str,
        _query: Option<&ListQuery>,
        context: &RequestContext,
    ) -> Result<Vec<Resource>, Self::Error> {
        let tenant_id = Self::get_tenant_id_from_context(context);

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
        let tenant_id = Self::get_tenant_id_from_context(context);

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
        let tenant_id = Self::get_tenant_id_from_context(context);

        let resources = self.resources.read().await;
        let exists = resources
            .get(&tenant_id)
            .and_then(|tenant| tenant.get(resource_type))
            .map(|resources| resources.contains_key(id))
            .unwrap_or(false);

        Ok(exists)
    }
}

// ============================================================================
// Supporting Types and Utilities
// ============================================================================

/// Statistics for a tenant's resource usage
#[derive(Debug, Clone)]
pub struct TenantStatistics {
    pub tenant_id: String,
    pub resource_counts: HashMap<String, usize>,
    pub total_resources: usize,
}

// ============================================================================
// Comprehensive Test Suite
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_user_data(username: &str) -> Value {
        json!({
            "userName": username,
            "displayName": format!("{} User", username),
            "active": true,
            "emails": [{
                "value": format!("{}@example.com", username),
                "type": "work",
                "primary": true
            }]
        })
    }

    fn create_test_group_data(display_name: &str) -> Value {
        json!({
            "displayName": display_name,
            "description": format!("{} group for testing", display_name),
            "members": []
        })
    }

    #[tokio::test]
    async fn test_basic_single_tenant_operations() {
        let provider = InMemoryProvider::for_testing();
        let context = create_single_tenant_context();

        // Create user
        let user_data = create_test_user_data("testuser");
        let created = provider
            .create_resource("User", user_data, &context)
            .await
            .unwrap();

        assert_eq!(created.resource_type, "User");
        assert!(created.id.is_some());
        assert_eq!(created.user_name.as_ref().unwrap().as_str(), "testuser");

        // Get user
        let id = created.id.as_ref().unwrap().as_str();
        let retrieved = provider
            .get_resource("User", id, &context)
            .await
            .unwrap()
            .unwrap();

        assert_eq!(retrieved.id, created.id);

        // Update user
        let update_data = json!({
            "userName": "updateduser",
            "active": false
        });

        let updated = provider
            .update_resource("User", id, update_data, &context)
            .await
            .unwrap();

        assert_eq!(updated.user_name.as_ref().unwrap().as_str(), "updateduser");

        // List users
        let users = provider
            .list_resources("User", None, &context)
            .await
            .unwrap();

        assert_eq!(users.len(), 1);

        // Delete user
        provider
            .delete_resource("User", id, &context)
            .await
            .unwrap();

        let deleted = provider.get_resource("User", id, &context).await.unwrap();

        assert!(deleted.is_none());
    }

    #[tokio::test]
    async fn test_multi_tenant_isolation() {
        let provider = InMemoryProvider::for_testing();
        let context_a = create_multi_tenant_context("tenant_a");
        let context_b = create_multi_tenant_context("tenant_b");

        // Create users in different tenants
        let user_a = provider
            .create_resource("User", create_test_user_data("user_a"), &context_a)
            .await
            .unwrap();

        let user_b = provider
            .create_resource("User", create_test_user_data("user_b"), &context_b)
            .await
            .unwrap();

        let id_a = user_a.id.as_ref().unwrap().as_str();
        let id_b = user_b.id.as_ref().unwrap().as_str();

        // Verify tenant A can access its own user but not tenant B's
        let get_a_own = provider
            .get_resource("User", id_a, &context_a)
            .await
            .unwrap();
        assert!(get_a_own.is_some());

        let get_a_cross = provider
            .get_resource("User", id_b, &context_a)
            .await
            .unwrap();
        assert!(get_a_cross.is_none());

        // Verify tenant B can access its own user but not tenant A's
        let get_b_own = provider
            .get_resource("User", id_b, &context_b)
            .await
            .unwrap();
        assert!(get_b_own.is_some());

        let get_b_cross = provider
            .get_resource("User", id_a, &context_b)
            .await
            .unwrap();
        assert!(get_b_cross.is_none());

        // Verify list isolation
        let list_a = provider
            .list_resources("User", None, &context_a)
            .await
            .unwrap();
        assert_eq!(list_a.len(), 1);

        let list_b = provider
            .list_resources("User", None, &context_b)
            .await
            .unwrap();
        assert_eq!(list_b.len(), 1);
    }

    #[tokio::test]
    async fn test_username_uniqueness_within_tenant() {
        let provider = InMemoryProvider::for_testing();
        let context = create_multi_tenant_context("test_tenant");

        // Create first user
        let user_data = create_test_user_data("testuser");
        let _created = provider
            .create_resource("User", user_data.clone(), &context)
            .await
            .unwrap();

        // Try to create duplicate user (should fail)
        let result = provider.create_resource("User", user_data, &context).await;

        assert!(result.is_err());
        match result.err().unwrap() {
            InMemoryProviderError::DuplicateResource {
                attribute, value, ..
            } => {
                assert_eq!(attribute, "userName");
                assert_eq!(value, "testuser");
            }
            _ => panic!("Expected DuplicateResource error"),
        }
    }

    #[tokio::test]
    async fn test_username_allowed_across_tenants() {
        let provider = InMemoryProvider::for_testing();
        let context_a = create_multi_tenant_context("tenant_a");
        let context_b = create_multi_tenant_context("tenant_b");

        let user_data = create_test_user_data("testuser");

        // Create user in tenant A
        let user_a = provider
            .create_resource("User", user_data.clone(), &context_a)
            .await
            .unwrap();

        // Create user with same username in tenant B (should succeed)
        let user_b = provider
            .create_resource("User", user_data, &context_b)
            .await
            .unwrap();

        assert_ne!(user_a.id, user_b.id);
        assert_eq!(
            user_a.user_name.as_ref().unwrap().as_str(),
            user_b.user_name.as_ref().unwrap().as_str()
        );
    }

    #[tokio::test]
    async fn test_find_resource_by_attribute() {
        let provider = InMemoryProvider::for_testing();
        let context = create_single_tenant_context();

        // Create user
        let user_data = create_test_user_data("searchuser");
        let created = provider
            .create_resource("User", user_data, &context)
            .await
            .unwrap();

        // Find by username
        let found = provider
            .find_resource_by_attribute("User", "userName", &json!("searchuser"), &context)
            .await
            .unwrap()
            .unwrap();

        assert_eq!(found.id, created.id);

        // Find by non-existent username
        let not_found = provider
            .find_resource_by_attribute("User", "userName", &json!("nonexistent"), &context)
            .await
            .unwrap();

        assert!(not_found.is_none());
    }

    #[tokio::test]
    async fn test_resource_limits() {
        let config = InMemoryProviderConfig {
            enable_metrics: false,
            max_resources_per_tenant: Some(2),
            max_tenants: Some(10),
            enable_validation: true,
        };
        let provider = InMemoryProvider::with_config(config);
        let context = create_single_tenant_context();

        // Create resources up to limit
        provider
            .create_resource("User", create_test_user_data("user1"), &context)
            .await
            .unwrap();

        provider
            .create_resource("User", create_test_user_data("user2"), &context)
            .await
            .unwrap();

        // Try to create beyond limit (should fail)
        let result = provider
            .create_resource("User", create_test_user_data("user3"), &context)
            .await;

        assert!(result.is_err());
        match result.err().unwrap() {
            InMemoryProviderError::ResourceLimitExceeded { max_resources, .. } => {
                assert_eq!(max_resources, 2);
            }
            _ => panic!("Expected ResourceLimitExceeded error"),
        }
    }

    #[tokio::test]
    async fn test_statistics() {
        let provider = InMemoryProvider::for_testing();
        let context = create_single_tenant_context();

        // Create some resources
        provider
            .create_resource("User", create_test_user_data("user1"), &context)
            .await
            .unwrap();

        provider
            .create_resource("User", create_test_user_data("user2"), &context)
            .await
            .unwrap();

        provider
            .create_resource("Group", create_test_group_data("group1"), &context)
            .await
            .unwrap();

        // Get statistics
        let stats = provider.get_tenant_statistics("default").await;

        assert_eq!(stats.tenant_id, "default");
        assert_eq!(stats.total_resources, 3);
        assert_eq!(stats.resource_counts.get("User"), Some(&2));
        assert_eq!(stats.resource_counts.get("Group"), Some(&1));
    }

    #[tokio::test]
    async fn test_concurrent_operations() {
        let provider = Arc::new(InMemoryProvider::for_testing());
        let context = create_single_tenant_context();

        // Create concurrent tasks
        let tasks: Vec<_> = (0..10)
            .map(|i| {
                let provider = Arc::clone(&provider);
                let context = context.clone();
                tokio::spawn(async move {
                    let username = format!("user{}", i);
                    provider
                        .create_resource("User", create_test_user_data(&username), &context)
                        .await
                        .unwrap()
                })
            })
            .collect();

        // Wait for all tasks to complete
        let mut results = Vec::new();
        for task in tasks {
            results.push(task.await.unwrap());
        }

        // Verify all resources were created
        let resources = provider
            .list_resources("User", None, &context)
            .await
            .unwrap();

        assert_eq!(resources.len(), 10);
    }
}
