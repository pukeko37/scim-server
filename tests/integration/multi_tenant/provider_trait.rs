//! Stage 2: Provider Trait Multi-Tenancy Tests
//!
//! This module contains tests for the enhanced ResourceProvider trait with tenant support.
//! These tests drive the development of:
//! - Updated ResourceProvider trait with tenant parameters
//! - Tenant-scoped resource operations
//! - Provider-agnostic multi-tenant behavior
//! - Resource isolation verification at the provider level
//! - Cross-tenant access prevention in provider implementations
//!
//! ## Test Strategy
//!
//! These tests define the contract that all ResourceProvider implementations must follow
//! for proper tenant isolation. They test the provider interface without being tied to
//! specific provider implementations.
//!
//! ## Security Requirements
//!
//! Every provider implementation must guarantee:
//! 1. Resources are scoped to tenants - no cross-tenant access
//! 2. Operations fail securely when tenant context is invalid
//! 3. Resource IDs are unique within tenant scope
//! 4. List/search operations only return tenant-scoped resources

use super::core::{EnhancedRequestContext, TenantContextBuilder};
use crate::common::{create_test_context, create_test_user};
use scim_server::Resource;
use serde_json::{Value, json};
use std::collections::HashMap;
use std::future::Future;

// ============================================================================
// Enhanced ResourceProvider Trait Definition
// ============================================================================

/// Enhanced ResourceProvider trait with tenant support
///
/// This trait defines the contract that all multi-tenant resource providers must implement.
/// All operations are scoped to a specific tenant to ensure data isolation.
pub trait MultiTenantResourceProvider: Send + Sync {
    type Error: std::error::Error + Send + Sync + 'static;

    /// Create a resource for a specific tenant
    fn create_resource(
        &self,
        tenant_id: &str,
        resource_type: &str,
        data: Value,
        context: &EnhancedRequestContext,
    ) -> impl Future<Output = Result<Resource, Self::Error>> + Send;

    /// Get a resource by ID within tenant scope
    fn get_resource(
        &self,
        tenant_id: &str,
        resource_type: &str,
        id: &str,
        context: &EnhancedRequestContext,
    ) -> impl Future<Output = Result<Option<Resource>, Self::Error>> + Send;

    /// Update a resource within tenant scope
    fn update_resource(
        &self,
        tenant_id: &str,
        resource_type: &str,
        id: &str,
        data: Value,
        context: &EnhancedRequestContext,
    ) -> impl Future<Output = Result<Resource, Self::Error>> + Send;

    /// Delete a resource within tenant scope
    fn delete_resource(
        &self,
        tenant_id: &str,
        resource_type: &str,
        id: &str,
        context: &EnhancedRequestContext,
    ) -> impl Future<Output = Result<(), Self::Error>> + Send;

    /// List resources within tenant scope
    fn list_resources(
        &self,
        tenant_id: &str,
        resource_type: &str,
        query: Option<&ListQuery>,
        context: &EnhancedRequestContext,
    ) -> impl Future<Output = Result<Vec<Resource>, Self::Error>> + Send;

    /// Find resource by attribute within tenant scope
    fn find_resource_by_attribute(
        &self,
        tenant_id: &str,
        resource_type: &str,
        attribute: &str,
        value: &Value,
        context: &EnhancedRequestContext,
    ) -> impl Future<Output = Result<Option<Resource>, Self::Error>> + Send;

    /// Check if resource exists within tenant scope
    fn resource_exists(
        &self,
        tenant_id: &str,
        resource_type: &str,
        id: &str,
        context: &EnhancedRequestContext,
    ) -> impl Future<Output = Result<bool, Self::Error>> + Send;
}

/// Query parameters for list operations
#[derive(Debug, Clone)]
pub struct ListQuery {
    pub count: Option<i32>,
    pub start_index: Option<i32>,
    pub filter: Option<String>,
    pub attributes: Option<Vec<String>>,
    pub excluded_attributes: Option<Vec<String>>,
}

impl ListQuery {
    pub fn new() -> Self {
        Self {
            count: None,
            start_index: None,
            filter: None,
            attributes: None,
            excluded_attributes: None,
        }
    }

    pub fn with_count(mut self, count: i32) -> Self {
        self.count = Some(count);
        self
    }

    pub fn with_filter(mut self, filter: String) -> Self {
        self.filter = Some(filter);
        self
    }
}

// ============================================================================
// Test Provider Implementation
// ============================================================================

/// Test provider implementation for multi-tenant testing
pub struct TestMultiTenantProvider {
    // Tenant-scoped storage: tenant_id -> resource_type -> resource_id -> resource
    resources: tokio::sync::RwLock<HashMap<String, HashMap<String, HashMap<String, Resource>>>>,
    next_id: tokio::sync::RwLock<u64>,
}

impl TestMultiTenantProvider {
    pub fn new() -> Self {
        Self {
            resources: tokio::sync::RwLock::new(HashMap::new()),
            next_id: tokio::sync::RwLock::new(1),
        }
    }

    async fn generate_id(&self) -> String {
        let mut counter = self.next_id.write().await;
        let id = counter.to_string();
        *counter += 1;
        id
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
}

#[derive(Debug, thiserror::Error)]
pub enum TestProviderError {
    #[error("Resource not found: {resource_type} with id {id} in tenant {tenant_id}")]
    ResourceNotFound {
        tenant_id: String,
        resource_type: String,
        id: String,
    },
    #[error("Tenant not found: {tenant_id}")]
    TenantNotFound { tenant_id: String },
    #[error(
        "Duplicate resource: {resource_type} with attribute {attribute}={value} in tenant {tenant_id}"
    )]
    DuplicateResource {
        tenant_id: String,
        resource_type: String,
        attribute: String,
        value: String,
    },
    #[error("Invalid tenant context: expected {expected}, got {actual}")]
    InvalidTenantContext { expected: String, actual: String },
}

impl MultiTenantResourceProvider for TestMultiTenantProvider {
    type Error = TestProviderError;

    async fn create_resource(
        &self,
        tenant_id: &str,
        resource_type: &str,
        mut data: Value,
        context: &EnhancedRequestContext,
    ) -> Result<Resource, Self::Error> {
        // Validate tenant context matches
        if context.tenant_context.tenant_id != tenant_id {
            return Err(TestProviderError::InvalidTenantContext {
                expected: tenant_id.to_string(),
                actual: context.tenant_context.tenant_id.clone(),
            });
        }

        self.ensure_resource_type_exists(tenant_id, resource_type)
            .await;

        // Generate unique ID within tenant scope
        let id = self.generate_id().await;

        // Add ID to data
        if let Some(obj) = data.as_object_mut() {
            obj.insert("id".to_string(), json!(id.clone()));
        }

        // Check for duplicates within tenant scope
        if resource_type == "User" {
            if let Some(username) = data.get("userName").and_then(|v| v.as_str()) {
                let existing = self
                    .find_resource_by_attribute(
                        tenant_id,
                        resource_type,
                        "userName",
                        &json!(username),
                        context,
                    )
                    .await?;

                if existing.is_some() {
                    return Err(TestProviderError::DuplicateResource {
                        tenant_id: tenant_id.to_string(),
                        resource_type: resource_type.to_string(),
                        attribute: "userName".to_string(),
                        value: username.to_string(),
                    });
                }
            }
        }

        let resource = Resource::new(resource_type.to_string(), data);

        // Store in tenant-scoped storage
        let mut resources = self.resources.write().await;
        resources
            .get_mut(tenant_id)
            .unwrap()
            .get_mut(resource_type)
            .unwrap()
            .insert(id, resource.clone());

        Ok(resource)
    }

    async fn get_resource(
        &self,
        tenant_id: &str,
        resource_type: &str,
        id: &str,
        context: &EnhancedRequestContext,
    ) -> Result<Option<Resource>, Self::Error> {
        // Validate tenant context
        if context.tenant_context.tenant_id != tenant_id {
            return Err(TestProviderError::InvalidTenantContext {
                expected: tenant_id.to_string(),
                actual: context.tenant_context.tenant_id.clone(),
            });
        }

        let resources = self.resources.read().await;

        let result = resources
            .get(tenant_id)
            .and_then(|tenant_resources| tenant_resources.get(resource_type))
            .and_then(|type_resources| type_resources.get(id))
            .cloned();

        Ok(result)
    }

    async fn update_resource(
        &self,
        tenant_id: &str,
        resource_type: &str,
        id: &str,
        mut data: Value,
        context: &EnhancedRequestContext,
    ) -> Result<Resource, Self::Error> {
        // Validate tenant context
        if context.tenant_context.tenant_id != tenant_id {
            return Err(TestProviderError::InvalidTenantContext {
                expected: tenant_id.to_string(),
                actual: context.tenant_context.tenant_id.clone(),
            });
        }

        // Ensure ID is set
        if let Some(obj) = data.as_object_mut() {
            obj.insert("id".to_string(), json!(id));
        }

        let mut resources = self.resources.write().await;

        let tenant_resources =
            resources
                .get_mut(tenant_id)
                .ok_or_else(|| TestProviderError::TenantNotFound {
                    tenant_id: tenant_id.to_string(),
                })?;

        let type_resources = tenant_resources.get_mut(resource_type).ok_or_else(|| {
            TestProviderError::ResourceNotFound {
                tenant_id: tenant_id.to_string(),
                resource_type: resource_type.to_string(),
                id: id.to_string(),
            }
        })?;

        if !type_resources.contains_key(id) {
            return Err(TestProviderError::ResourceNotFound {
                tenant_id: tenant_id.to_string(),
                resource_type: resource_type.to_string(),
                id: id.to_string(),
            });
        }

        let resource = Resource::new(resource_type.to_string(), data);
        type_resources.insert(id.to_string(), resource.clone());

        Ok(resource)
    }

    async fn delete_resource(
        &self,
        tenant_id: &str,
        resource_type: &str,
        id: &str,
        context: &EnhancedRequestContext,
    ) -> Result<(), Self::Error> {
        // Validate tenant context
        if context.tenant_context.tenant_id != tenant_id {
            return Err(TestProviderError::InvalidTenantContext {
                expected: tenant_id.to_string(),
                actual: context.tenant_context.tenant_id.clone(),
            });
        }

        let mut resources = self.resources.write().await;

        let tenant_resources =
            resources
                .get_mut(tenant_id)
                .ok_or_else(|| TestProviderError::TenantNotFound {
                    tenant_id: tenant_id.to_string(),
                })?;

        let type_resources = tenant_resources.get_mut(resource_type).ok_or_else(|| {
            TestProviderError::ResourceNotFound {
                tenant_id: tenant_id.to_string(),
                resource_type: resource_type.to_string(),
                id: id.to_string(),
            }
        })?;

        type_resources
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
        tenant_id: &str,
        resource_type: &str,
        _query: Option<&ListQuery>,
        context: &EnhancedRequestContext,
    ) -> Result<Vec<Resource>, Self::Error> {
        // Validate tenant context
        if context.tenant_context.tenant_id != tenant_id {
            return Err(TestProviderError::InvalidTenantContext {
                expected: tenant_id.to_string(),
                actual: context.tenant_context.tenant_id.clone(),
            });
        }

        let resources = self.resources.read().await;

        let result = resources
            .get(tenant_id)
            .and_then(|tenant_resources| tenant_resources.get(resource_type))
            .map(|type_resources| type_resources.values().cloned().collect())
            .unwrap_or_else(Vec::new);

        Ok(result)
    }

    async fn find_resource_by_attribute(
        &self,
        tenant_id: &str,
        resource_type: &str,
        attribute: &str,
        value: &Value,
        context: &EnhancedRequestContext,
    ) -> Result<Option<Resource>, Self::Error> {
        // Validate tenant context
        if context.tenant_context.tenant_id != tenant_id {
            return Err(TestProviderError::InvalidTenantContext {
                expected: tenant_id.to_string(),
                actual: context.tenant_context.tenant_id.clone(),
            });
        }

        let resources = self.resources.read().await;

        let result = resources
            .get(tenant_id)
            .and_then(|tenant_resources| tenant_resources.get(resource_type))
            .and_then(|type_resources| {
                type_resources
                    .values()
                    .find(|resource| {
                        resource
                            .get_attribute(attribute)
                            .map(|attr_value| attr_value == value)
                            .unwrap_or(false)
                    })
                    .cloned()
            });

        Ok(result)
    }

    async fn resource_exists(
        &self,
        tenant_id: &str,
        resource_type: &str,
        id: &str,
        context: &EnhancedRequestContext,
    ) -> Result<bool, Self::Error> {
        let resource = self
            .get_resource(tenant_id, resource_type, id, context)
            .await?;
        Ok(resource.is_some())
    }
}

// ============================================================================
// Stage 2 Tests: Provider Trait Multi-Tenancy
// ============================================================================

#[cfg(test)]
mod provider_trait_multi_tenant_tests {
    use super::*;

    fn create_test_context(tenant_id: &str) -> EnhancedRequestContext {
        let tenant_context = TenantContextBuilder::new(tenant_id).build();
        EnhancedRequestContext {
            request_id: format!("req_{}", tenant_id),
            tenant_context,
        }
    }

    fn create_test_user(username: &str) -> Value {
        json!({
            "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
            "userName": username,
            "displayName": format!("{} User", username),
            "active": true
        })
    }

    // ------------------------------------------------------------------------
    // Test Group 1: Basic Tenant-Scoped Operations
    // ------------------------------------------------------------------------

    #[tokio::test]
    async fn test_create_resource_with_tenant_scope() {
        let provider = TestMultiTenantProvider::new();
        let context = create_test_context("tenant_a");
        let user_data = create_test_user("alice");

        let result = provider
            .create_resource("tenant_a", "User", user_data, &context)
            .await;

        assert!(result.is_ok());
        let resource = result.unwrap();
        assert_eq!(resource.resource_type, "User");
        assert!(resource.get_id().is_some());
        assert_eq!(resource.get_attribute("userName").unwrap(), &json!("alice"));
    }

    #[tokio::test]
    async fn test_get_resource_with_tenant_scope() {
        let provider = TestMultiTenantProvider::new();
        let context = create_test_context("tenant_a");
        let user_data = create_test_user("bob");

        // Create resource
        let created = provider
            .create_resource("tenant_a", "User", user_data, &context)
            .await
            .unwrap();

        let resource_id = created.get_id().unwrap();

        // Get resource
        let result = provider
            .get_resource("tenant_a", "User", &resource_id, &context)
            .await;

        assert!(result.is_ok());
        let resource = result.unwrap();
        assert!(resource.is_some());
        assert_eq!(
            resource.unwrap().get_attribute("userName").unwrap(),
            &json!("bob")
        );
    }

    #[tokio::test]
    async fn test_update_resource_with_tenant_scope() {
        let provider = TestMultiTenantProvider::new();
        let context = create_test_context("tenant_a");
        let user_data = create_test_user("charlie");

        // Create resource
        let created = provider
            .create_resource("tenant_a", "User", user_data, &context)
            .await
            .unwrap();

        let resource_id = created.get_id().unwrap();

        // Update resource
        let updated_data = json!({
            "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
            "userName": "charlie_updated",
            "displayName": "Charlie Updated User",
            "active": false
        });

        let result = provider
            .update_resource("tenant_a", "User", &resource_id, updated_data, &context)
            .await;

        assert!(result.is_ok());
        let resource = result.unwrap();
        assert_eq!(
            resource.get_attribute("userName").unwrap(),
            &json!("charlie_updated")
        );
        assert_eq!(resource.get_attribute("active").unwrap(), &json!(false));
    }

    #[tokio::test]
    async fn test_delete_resource_with_tenant_scope() {
        let provider = TestMultiTenantProvider::new();
        let context = create_test_context("tenant_a");
        let user_data = create_test_user("diana");

        // Create resource
        let created = provider
            .create_resource("tenant_a", "User", user_data, &context)
            .await
            .unwrap();

        let resource_id = created.get_id().unwrap();

        // Delete resource
        let delete_result = provider
            .delete_resource("tenant_a", "User", &resource_id, &context)
            .await;

        assert!(delete_result.is_ok());

        // Verify resource is deleted
        let get_result = provider
            .get_resource("tenant_a", "User", &resource_id, &context)
            .await;

        assert!(get_result.is_ok());
        assert!(get_result.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_list_resources_with_tenant_scope() {
        let provider = TestMultiTenantProvider::new();
        let context = create_test_context("tenant_a");

        // Create multiple resources
        let _user1 = provider
            .create_resource("tenant_a", "User", create_test_user("eve"), &context)
            .await
            .unwrap();

        let _user2 = provider
            .create_resource("tenant_a", "User", create_test_user("frank"), &context)
            .await
            .unwrap();

        // List resources
        let result = provider
            .list_resources("tenant_a", "User", None, &context)
            .await;

        assert!(result.is_ok());
        let resources = result.unwrap();
        assert_eq!(resources.len(), 2);

        let usernames: Vec<&str> = resources
            .iter()
            .map(|r| r.get_attribute("userName").unwrap().as_str().unwrap())
            .collect();
        assert!(usernames.contains(&"eve"));
        assert!(usernames.contains(&"frank"));
    }

    // ------------------------------------------------------------------------
    // Test Group 2: Cross-Tenant Isolation
    // ------------------------------------------------------------------------

    #[tokio::test]
    async fn test_tenant_isolation_in_create_and_get() {
        let provider = TestMultiTenantProvider::new();
        let context_a = create_test_context("tenant_a");
        let context_b = create_test_context("tenant_b");

        // Create resource in tenant A
        let user_a = provider
            .create_resource("tenant_a", "User", create_test_user("alice_a"), &context_a)
            .await
            .unwrap();

        // Create resource in tenant B
        let user_b = provider
            .create_resource("tenant_b", "User", create_test_user("alice_b"), &context_b)
            .await
            .unwrap();

        let resource_id_a = user_a.get_id().unwrap();
        let resource_id_b = user_b.get_id().unwrap();

        // Verify tenant A can only see its resource
        let result_a = provider
            .get_resource("tenant_a", "User", &resource_id_a, &context_a)
            .await
            .unwrap();
        assert!(result_a.is_some());

        let result_a_cross = provider
            .get_resource("tenant_a", "User", &resource_id_b, &context_a)
            .await
            .unwrap();
        assert!(result_a_cross.is_none()); // Cannot see tenant B's resource

        // Verify tenant B can only see its resource
        let result_b = provider
            .get_resource("tenant_b", "User", &resource_id_b, &context_b)
            .await
            .unwrap();
        assert!(result_b.is_some());

        let result_b_cross = provider
            .get_resource("tenant_b", "User", &resource_id_a, &context_b)
            .await
            .unwrap();
        assert!(result_b_cross.is_none()); // Cannot see tenant A's resource
    }

    #[tokio::test]
    async fn test_tenant_isolation_in_list_operations() {
        let provider = TestMultiTenantProvider::new();
        let context_a = create_test_context("tenant_a");
        let context_b = create_test_context("tenant_b");

        // Create resources in both tenants
        let _user_a1 = provider
            .create_resource("tenant_a", "User", create_test_user("alice_a"), &context_a)
            .await
            .unwrap();

        let _user_a2 = provider
            .create_resource("tenant_a", "User", create_test_user("bob_a"), &context_a)
            .await
            .unwrap();

        let _user_b1 = provider
            .create_resource("tenant_b", "User", create_test_user("alice_b"), &context_b)
            .await
            .unwrap();

        // List resources for tenant A
        let list_a = provider
            .list_resources("tenant_a", "User", None, &context_a)
            .await
            .unwrap();

        // List resources for tenant B
        let list_b = provider
            .list_resources("tenant_b", "User", None, &context_b)
            .await
            .unwrap();

        // Verify isolation
        assert_eq!(list_a.len(), 2);
        assert_eq!(list_b.len(), 1);

        let usernames_a: Vec<&str> = list_a
            .iter()
            .map(|r| r.get_attribute("userName").unwrap().as_str().unwrap())
            .collect();
        assert!(usernames_a.contains(&"alice_a"));
        assert!(usernames_a.contains(&"bob_a"));
        assert!(!usernames_a.contains(&"alice_b"));

        let usernames_b: Vec<&str> = list_b
            .iter()
            .map(|r| r.get_attribute("userName").unwrap().as_str().unwrap())
            .collect();
        assert!(usernames_b.contains(&"alice_b"));
        assert!(!usernames_b.contains(&"alice_a"));
        assert!(!usernames_b.contains(&"bob_a"));
    }

    // ------------------------------------------------------------------------
    // Test Group 3: Tenant Context Validation
    // ------------------------------------------------------------------------

    #[tokio::test]
    async fn test_tenant_context_mismatch_in_create() {
        let provider = TestMultiTenantProvider::new();
        let context_a = create_test_context("tenant_a");
        let user_data = create_test_user("mismatch_user");

        // Try to create resource with mismatched tenant context
        let result = provider
            .create_resource("tenant_b", "User", user_data, &context_a)
            .await;

        assert!(result.is_err());
        match result.unwrap_err() {
            TestProviderError::InvalidTenantContext { expected, actual } => {
                assert_eq!(expected, "tenant_b");
                assert_eq!(actual, "tenant_a");
            }
            other => panic!("Expected InvalidTenantContext, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_tenant_context_mismatch_in_get() {
        let provider = TestMultiTenantProvider::new();
        let context_a = create_test_context("tenant_a");
        let context_b = create_test_context("tenant_b");

        // Create resource in tenant A
        let user = provider
            .create_resource(
                "tenant_a",
                "User",
                create_test_user("test_user"),
                &context_a,
            )
            .await
            .unwrap();

        let resource_id = user.get_id().unwrap();

        // Try to get resource with wrong tenant context
        let result = provider
            .get_resource("tenant_b", "User", &resource_id, &context_a)
            .await;

        assert!(result.is_err());
        match result.unwrap_err() {
            TestProviderError::InvalidTenantContext { expected, actual } => {
                assert_eq!(expected, "tenant_b");
                assert_eq!(actual, "tenant_a");
            }
            other => panic!("Expected InvalidTenantContext, got {:?}", other),
        }
    }

    // ------------------------------------------------------------------------
    // Test Group 4: Duplicate Prevention Within Tenant Scope
    // ------------------------------------------------------------------------

    #[tokio::test]
    async fn test_duplicate_prevention_within_tenant() {
        let provider = TestMultiTenantProvider::new();
        let context = create_test_context("tenant_a");

        // Create first user
        let result1 = provider
            .create_resource(
                "tenant_a",
                "User",
                create_test_user("duplicate_test"),
                &context,
            )
            .await;
        assert!(result1.is_ok());

        // Try to create duplicate user in same tenant
        let result2 = provider
            .create_resource(
                "tenant_a",
                "User",
                create_test_user("duplicate_test"),
                &context,
            )
            .await;

        assert!(result2.is_err());
        match result2.unwrap_err() {
            TestProviderError::DuplicateResource {
                tenant_id,
                attribute,
                value,
                ..
            } => {
                assert_eq!(tenant_id, "tenant_a");
                assert_eq!(attribute, "userName");
                assert_eq!(value, "duplicate_test");
            }
            other => panic!("Expected DuplicateResource, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_same_username_allowed_across_tenants() {
        let provider = TestMultiTenantProvider::new();
        let context_a = create_test_context("tenant_a");
        let context_b = create_test_context("tenant_b");

        // Create user with same username in both tenants
        let result_a = provider
            .create_resource(
                "tenant_a",
                "User",
                create_test_user("shared_username"),
                &context_a,
            )
            .await;
        assert!(result_a.is_ok());

        let result_b = provider
            .create_resource(
                "tenant_b",
                "User",
                create_test_user("shared_username"),
                &context_b,
            )
            .await;
        assert!(result_b.is_ok()); // Should succeed - different tenants

        // Verify both resources exist in their respective tenants
        let found_a = provider
            .find_resource_by_attribute(
                "tenant_a",
                "User",
                "userName",
                &json!("shared_username"),
                &context_a,
            )
            .await
            .unwrap();
        assert!(found_a.is_some());

        let found_b = provider
            .find_resource_by_attribute(
                "tenant_b",
                "User",
                "userName",
                &json!("shared_username"),
                &context_b,
            )
            .await
            .unwrap();
        assert!(found_b.is_some());
    }

    // ------------------------------------------------------------------------
    // Test Group 5: Resource ID Uniqueness and Scoping
    // ------------------------------------------------------------------------

    #[tokio::test]
    async fn test_resource_ids_unique_within_tenant() {
        let provider = TestMultiTenantProvider::new();
        let context = create_test_context("tenant_a");

        // Create multiple resources
        let user1 = provider
            .create_resource("tenant_a", "User", create_test_user("user1"), &context)
            .await
            .unwrap();

        let user2 = provider
            .create_resource("tenant_a", "User", create_test_user("user2"), &context)
            .await
            .unwrap();

        // Verify IDs are different
        let id1 = user1.get_id().unwrap();
        let id2 = user2.get_id().unwrap();
        assert_ne!(id1, id2);
    }

    #[tokio::test]
    async fn test_resource_exists_tenant_scoped() {
        let provider = TestMultiTenantProvider::new();
        let context_a = create_test_context("tenant_a");
        let context_b = create_test_context("tenant_b");

        // Create resource in tenant A
        let user = provider
            .create_resource(
                "tenant_a",
                "User",
                create_test_user("exists_test"),
                &context_a,
            )
            .await
            .unwrap();

        let resource_id = user.get_id().unwrap();

        // Check existence in tenant A
        let exists_a = provider
            .resource_exists("tenant_a", "User", &resource_id, &context_a)
            .await
            .unwrap();
        assert!(exists_a);

        // Check existence in tenant B (should not exist)
        let exists_b = provider
            .resource_exists("tenant_b", "User", &resource_id, &context_b)
            .await
            .unwrap();
        assert!(!exists_b);
    }

    // ------------------------------------------------------------------------
    // Test Group 6: Find Resource by Attribute with Tenant Scoping
    // ------------------------------------------------------------------------

    #[tokio::test]
    async fn test_find_resource_by_attribute_tenant_scoped() {
        let provider = TestMultiTenantProvider::new();
        let context_a = create_test_context("tenant_a");
        let context_b = create_test_context("tenant_b");

        // Create users with same attribute value in different tenants
        let _user_a = provider
            .create_resource("tenant_a", "User", create_test_user("findme"), &context_a)
            .await
            .unwrap();

        let _user_b = provider
            .create_resource("tenant_b", "User", create_test_user("findme"), &context_b)
            .await
            .unwrap();

        // Find in tenant A
        let found_a = provider
            .find_resource_by_attribute(
                "tenant_a",
                "User",
                "userName",
                &json!("findme"),
                &context_a,
            )
            .await
            .unwrap();
        assert!(found_a.is_some());

        // Find in tenant B
        let found_b = provider
            .find_resource_by_attribute(
                "tenant_b",
                "User",
                "userName",
                &json!("findme"),
                &context_b,
            )
            .await
            .unwrap();
        assert!(found_b.is_some());

        // Verify they are different resources (different IDs)
        let resource_a = found_a.unwrap();
        let resource_b = found_b.unwrap();
        let id_a = resource_a.get_id().unwrap();
        let id_b = resource_b.get_id().unwrap();
        assert_ne!(id_a, id_b);
    }

    #[tokio::test]
    async fn test_find_resource_by_attribute_not_found_in_tenant() {
        let provider = TestMultiTenantProvider::new();
        let context_a = create_test_context("tenant_a");
        let context_b = create_test_context("tenant_b");

        // Create user in tenant A only
        let _user_a = provider
            .create_resource("tenant_a", "User", create_test_user("onlyina"), &context_a)
            .await
            .unwrap();

        // Try to find in tenant B
        let found_b = provider
            .find_resource_by_attribute(
                "tenant_b",
                "User",
                "userName",
                &json!("onlyina"),
                &context_b,
            )
            .await
            .unwrap();
        assert!(found_b.is_none());
    }

    // ------------------------------------------------------------------------
    // Test Group 7: Error Scenarios and Edge Cases
    // ------------------------------------------------------------------------

    #[tokio::test]
    async fn test_get_nonexistent_resource() {
        let provider = TestMultiTenantProvider::new();
        let context = create_test_context("tenant_a");

        let result = provider
            .get_resource("tenant_a", "User", "nonexistent_id", &context)
            .await;

        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_update_nonexistent_resource() {
        let provider = TestMultiTenantProvider::new();
        let context = create_test_context("tenant_a");

        // First establish the tenant by creating and deleting a resource
        let temp_user_data = create_test_user("temp_user");
        let temp_resource = provider
            .create_resource("tenant_a", "User", temp_user_data, &context)
            .await
            .unwrap();
        let temp_id = temp_resource.get_id().unwrap();
        provider
            .delete_resource("tenant_a", "User", temp_id, &context)
            .await
            .unwrap();

        // Now test update on nonexistent resource in existing tenant
        let user_data = create_test_user("nonexistent");
        let result = provider
            .update_resource("tenant_a", "User", "nonexistent_id", user_data, &context)
            .await;

        assert!(result.is_err());
        match result.unwrap_err() {
            TestProviderError::ResourceNotFound {
                tenant_id,
                resource_type,
                id,
            } => {
                assert_eq!(tenant_id, "tenant_a");
                assert_eq!(resource_type, "User");
                assert_eq!(id, "nonexistent_id");
            }
            other => panic!("Expected ResourceNotFound, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_delete_nonexistent_resource() {
        let provider = TestMultiTenantProvider::new();
        let context = create_test_context("tenant_a");

        // First establish the tenant by creating and deleting a resource
        let temp_user_data = create_test_user("temp_user");
        let temp_resource = provider
            .create_resource("tenant_a", "User", temp_user_data, &context)
            .await
            .unwrap();
        let temp_id = temp_resource.get_id().unwrap();
        provider
            .delete_resource("tenant_a", "User", temp_id, &context)
            .await
            .unwrap();

        // Now test delete on nonexistent resource in existing tenant
        let result = provider
            .delete_resource("tenant_a", "User", "nonexistent_id", &context)
            .await;

        assert!(result.is_err());
        match result.unwrap_err() {
            TestProviderError::ResourceNotFound {
                tenant_id,
                resource_type,
                id,
            } => {
                assert_eq!(tenant_id, "tenant_a");
                assert_eq!(resource_type, "User");
                assert_eq!(id, "nonexistent_id");
            }
            other => panic!("Expected ResourceNotFound, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_empty_tenant_list_resources() {
        let provider = TestMultiTenantProvider::new();
        let context = create_test_context("empty_tenant");

        let result = provider
            .list_resources("empty_tenant", "User", None, &context)
            .await;

        assert!(result.is_ok());
        let resources = result.unwrap();
        assert!(resources.is_empty());
    }

    // ------------------------------------------------------------------------
    // Test Group 8: Performance and Stress Testing
    // ------------------------------------------------------------------------

    #[tokio::test]
    async fn test_multiple_tenants_concurrent_operations() {
        let provider = std::sync::Arc::new(TestMultiTenantProvider::new());

        // Create multiple tenants
        let tenant_ids = vec!["tenant_1", "tenant_2", "tenant_3", "tenant_4", "tenant_5"];
        let mut handles = Vec::new();

        for tenant_id in tenant_ids {
            let provider_clone = provider.clone();
            let context = create_test_context(tenant_id);

            let handle = tokio::spawn(async move {
                // Create multiple users per tenant
                for i in 0..5 {
                    let username = format!("user_{}_{}", tenant_id, i);
                    let user_data = create_test_user(&username);

                    let result = provider_clone
                        .create_resource(tenant_id, "User", user_data, &context)
                        .await;

                    assert!(result.is_ok());
                }

                // List resources for this tenant
                let list_result = provider_clone
                    .list_resources(tenant_id, "User", None, &context)
                    .await;

                assert!(list_result.is_ok());
                let resources = list_result.unwrap();
                assert_eq!(resources.len(), 5);

                tenant_id
            });

            handles.push(handle);
        }

        // Wait for all operations to complete
        for handle in handles {
            let tenant_id = handle.await.unwrap();
            println!("Completed operations for tenant: {}", tenant_id);
        }

        // Verify total isolation - each tenant should have exactly 5 users
        for tenant_id in ["tenant_1", "tenant_2", "tenant_3", "tenant_4", "tenant_5"] {
            let context = create_test_context(tenant_id);
            let resources = provider
                .list_resources(tenant_id, "User", None, &context)
                .await
                .unwrap();

            assert_eq!(
                resources.len(),
                5,
                "Tenant {} should have exactly 5 users",
                tenant_id
            );
        }
    }

    // ------------------------------------------------------------------------
    // Test Group 9: Integration Test Documentation
    // ------------------------------------------------------------------------

    #[tokio::test]
    async fn test_provider_trait_documentation() {
        println!("\n🔧 Provider Trait Multi-Tenancy Test Documentation");
        println!("===================================================");
        println!("This test suite validates the MultiTenantResourceProvider trait");
        println!("implementation and ensures proper tenant isolation.\n");

        println!("🔒 Security Guarantees Tested:");
        println!("  • All operations are scoped to tenant ID");
        println!("  • Cross-tenant data access is prevented");
        println!("  • Tenant context validation is enforced");
        println!("  • Resource IDs are unique within tenant scope");
        println!("  • List operations only return tenant-scoped resources\n");

        println!("✅ Test Categories:");
        println!("  • Basic tenant-scoped CRUD operations");
        println!("  • Cross-tenant isolation verification");
        println!("  • Tenant context validation");
        println!("  • Duplicate prevention within tenant scope");
        println!("  • Resource ID uniqueness and scoping");
        println!("  • Find operations with tenant scoping");
        println!("  • Error scenarios and edge cases");
        println!("  • Performance and concurrent operations\n");

        println!("🎯 Provider Contract Requirements:");
        println!("  • Implement MultiTenantResourceProvider trait");
        println!("  • Validate tenant context in all operations");
        println!("  • Ensure data isolation between tenants");
        println!("  • Handle tenant-specific errors appropriately");
        println!("  • Support concurrent multi-tenant operations");
    }
}

// ============================================================================
// Test Utilities and Helpers
// ============================================================================

/// Utility functions for provider trait testing
pub struct ProviderTestHarness;

impl ProviderTestHarness {
    /// Create a test provider with pre-populated data for multiple tenants
    pub async fn create_populated_provider() -> TestMultiTenantProvider {
        let provider = TestMultiTenantProvider::new();

        // Populate tenant A
        let context_a = create_test_context("tenant_a");
        for i in 1..=3 {
            let username = format!("user_a_{}", i);
            let _ = provider
                .create_resource("tenant_a", "User", create_test_user(&username), &context_a)
                .await;
        }

        // Populate tenant B
        let context_b = create_test_context("tenant_b");
        for i in 1..=2 {
            let username = format!("user_b_{}", i);
            let _ = provider
                .create_resource("tenant_b", "User", create_test_user(&username), &context_b)
                .await;
        }

        provider
    }

    /// Verify tenant isolation across all operations
    pub async fn verify_complete_tenant_isolation(
        provider: &TestMultiTenantProvider,
        tenant_a_id: &str,
        tenant_b_id: &str,
    ) {
        let context_a = create_test_context(tenant_a_id);
        let context_b = create_test_context(tenant_b_id);

        // Get counts for each tenant
        let resources_a = provider
            .list_resources(tenant_a_id, "User", None, &context_a)
            .await
            .unwrap();

        let resources_b = provider
            .list_resources(tenant_b_id, "User", None, &context_b)
            .await
            .unwrap();

        println!("Tenant {} has {} resources", tenant_a_id, resources_a.len());
        println!("Tenant {} has {} resources", tenant_b_id, resources_b.len());

        // Verify no resource IDs overlap
        let ids_a: std::collections::HashSet<String> = resources_a
            .iter()
            .map(|r| r.get_id().unwrap().to_string())
            .collect();

        let ids_b: std::collections::HashSet<String> = resources_b
            .iter()
            .map(|r| r.get_id().unwrap().to_string())
            .collect();

        let intersection: Vec<_> = ids_a.intersection(&ids_b).collect();
        assert!(
            intersection.is_empty(),
            "Found overlapping resource IDs: {:?}",
            intersection
        );

        println!("✅ Complete tenant isolation verified");
    }
}
