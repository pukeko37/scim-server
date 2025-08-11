//! Standard in-memory resource provider implementation.
//!
//! This module provides a production-ready in-memory implementation of the
//! ResourceProvider trait that supports both single-tenant and multi-tenant
//! operations through the unified RequestContext interface.
//!
//! # Features
//!
//! * Thread-safe concurrent access with RwLock
//! * Automatic tenant isolation when tenant context is provided
//! * Fallback to "default" tenant for single-tenant operations
//! * Comprehensive error handling
//! * Resource metadata tracking (created/updated timestamps)
//! * Duplicate detection for userName attributes
//!
//! # Example Usage
//!
//! ```rust
//! use scim_server::providers::InMemoryProvider;
//! use scim_server::resource::{RequestContext, TenantContext, ResourceProvider};
//! use serde_json::json;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let provider = InMemoryProvider::new();
//!
//! // Single-tenant operation
//! let single_context = RequestContext::with_generated_id();
//! let user_data = json!({
//!     "userName": "john.doe",
//!     "displayName": "John Doe"
//! });
//! let user = provider.create_resource("User", user_data.clone(), &single_context).await?;
//!
//! // Multi-tenant operation
//! let tenant_context = TenantContext::new("tenant1".to_string(), "client1".to_string());
//! let multi_context = RequestContext::with_tenant_generated_id(tenant_context);
//! let tenant_user = provider.create_resource("User", user_data, &multi_context).await?;
//! # Ok(())
//! # }
//! ```

use crate::resource::{
    ListQuery, RequestContext, Resource, ResourceProvider,
    conditional_provider::VersionedResource,
    version::{ConditionalResult, ScimVersion, VersionConflict},
};
use log::{debug, info, trace, warn};
use serde_json::{Value, json};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Thread-safe in-memory resource provider supporting both single and multi-tenant operations.
///
/// This provider organizes data as: tenant_id -> resource_type -> resource_id -> resource
/// For single-tenant operations, it uses "default" as the tenant_id.
#[derive(Debug, Clone)]
pub struct InMemoryProvider {
    // Structure: tenant_id -> resource_type -> resource_id -> resource
    data: Arc<RwLock<HashMap<String, HashMap<String, HashMap<String, Resource>>>>>,
    // Track next ID per tenant and resource type for ID generation
    next_ids: Arc<RwLock<HashMap<String, HashMap<String, u64>>>>,
}

impl InMemoryProvider {
    /// Create a new in-memory provider.
    pub fn new() -> Self {
        Self {
            data: Arc::new(RwLock::new(HashMap::new())),
            next_ids: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Get the effective tenant ID for the operation.
    ///
    /// Returns the tenant ID from the context, or "default" for single-tenant operations.
    fn effective_tenant_id(&self, context: &RequestContext) -> String {
        context.tenant_id().unwrap_or("default").to_string()
    }

    /// Generate a unique resource ID for the given tenant and resource type.
    async fn generate_resource_id(&self, tenant_id: &str, resource_type: &str) -> String {
        let mut next_ids_guard = self.next_ids.write().await;
        let tenant_ids = next_ids_guard
            .entry(tenant_id.to_string())
            .or_insert_with(HashMap::new);
        let next_id = tenant_ids.entry(resource_type.to_string()).or_insert(1);

        let id = next_id.to_string();
        *next_id += 1;
        id
    }

    /// Check for duplicate userName in User resources within the same tenant.
    async fn check_username_duplicate(
        &self,
        tenant_id: &str,
        username: &str,
        exclude_id: Option<&str>,
    ) -> Result<(), InMemoryError> {
        let data_guard = self.data.read().await;

        if let Some(tenant_data) = data_guard.get(tenant_id) {
            if let Some(users) = tenant_data.get("User") {
                for (existing_id, existing_user) in users {
                    // Skip the resource we're updating
                    if Some(existing_id.as_str()) == exclude_id {
                        continue;
                    }

                    if let Some(existing_username) = existing_user.get_username() {
                        if existing_username == username {
                            return Err(InMemoryError::DuplicateAttribute {
                                resource_type: "User".to_string(),
                                attribute: "userName".to_string(),
                                value: username.to_string(),
                                tenant_id: tenant_id.to_string(),
                            });
                        }
                    }
                }
            }
        }

        Ok(())
    }

    /// Add SCIM metadata to a resource.
    fn add_scim_metadata(&self, mut resource: Resource) -> Resource {
        let now = chrono::Utc::now().to_rfc3339();
        resource.add_metadata("/scim/v2", &now, &now);
        resource
    }

    /// Clear all data (useful for testing).
    pub async fn clear(&self) {
        let mut data_guard = self.data.write().await;
        let mut ids_guard = self.next_ids.write().await;
        data_guard.clear();
        ids_guard.clear();
    }

    /// Get statistics about stored data.
    pub async fn get_stats(&self) -> InMemoryStats {
        let data_guard = self.data.read().await;

        let mut tenant_count = 0;
        let mut total_resources = 0;
        let mut resource_types = std::collections::HashSet::new();

        for (_tenant_id, tenant_data) in data_guard.iter() {
            tenant_count += 1;
            for (resource_type, resources) in tenant_data.iter() {
                resource_types.insert(resource_type.clone());
                total_resources += resources.len();
            }
        }

        InMemoryStats {
            tenant_count,
            total_resources,
            resource_type_count: resource_types.len(),
            resource_types: resource_types.into_iter().collect(),
        }
    }

    /// List all resources of a specific type in a tenant.
    pub async fn list_resources_in_tenant(
        &self,
        tenant_id: &str,
        resource_type: &str,
    ) -> Vec<Resource> {
        let data_guard = self.data.read().await;

        data_guard
            .get(tenant_id)
            .and_then(|tenant_data| tenant_data.get(resource_type))
            .map(|resources| resources.values().cloned().collect())
            .unwrap_or_default()
    }

    /// Count resources of a specific type for a tenant (used for limit checking).
    async fn count_resources_for_tenant(&self, tenant_id: &str, resource_type: &str) -> usize {
        let data_guard = self.data.read().await;
        data_guard
            .get(tenant_id)
            .and_then(|tenant_data| tenant_data.get(resource_type))
            .map(|resources| resources.len())
            .unwrap_or(0)
    }
}

impl Default for InMemoryProvider {
    fn default() -> Self {
        Self::new()
    }
}

/// Error types for the in-memory provider.
#[derive(Debug, thiserror::Error)]
pub enum InMemoryError {
    #[error("Resource not found: {resource_type} with id '{id}' in tenant '{tenant_id}'")]
    ResourceNotFound {
        resource_type: String,
        id: String,
        tenant_id: String,
    },

    #[error(
        "Duplicate attribute '{attribute}' with value '{value}' for {resource_type} in tenant '{tenant_id}'"
    )]
    DuplicateAttribute {
        resource_type: String,
        attribute: String,
        value: String,
        tenant_id: String,
    },

    #[error("Invalid resource data: {message}")]
    InvalidData { message: String },

    #[error("Query error: {message}")]
    QueryError { message: String },

    #[error("Internal error: {message}")]
    Internal { message: String },
}

/// Statistics about the in-memory provider state.
#[derive(Debug, Clone)]
pub struct InMemoryStats {
    pub tenant_count: usize,
    pub total_resources: usize,
    pub resource_type_count: usize,
    pub resource_types: Vec<String>,
}

impl ResourceProvider for InMemoryProvider {
    type Error = InMemoryError;

    async fn create_resource(
        &self,
        resource_type: &str,
        mut data: Value,
        context: &RequestContext,
    ) -> Result<Resource, Self::Error> {
        let tenant_id = self.effective_tenant_id(context);

        info!(
            "Creating {} resource for tenant '{}' (request: '{}')",
            resource_type, tenant_id, context.request_id
        );
        trace!(
            "Create data: {}",
            serde_json::to_string(&data).unwrap_or_else(|_| "invalid json".to_string())
        );

        // Check permissions first
        context
            .validate_operation("create")
            .map_err(|e| InMemoryError::Internal { message: e })?;

        // Check resource limits if this is a multi-tenant context
        if let Some(tenant_context) = &context.tenant_context {
            if resource_type == "User" {
                if let Some(max_users) = tenant_context.permissions.max_users {
                    let current_count = self.count_resources_for_tenant(&tenant_id, "User").await;
                    if current_count >= max_users {
                        return Err(InMemoryError::Internal {
                            message: format!(
                                "User limit exceeded: {}/{}",
                                current_count, max_users
                            ),
                        });
                    }
                }
            } else if resource_type == "Group" {
                if let Some(max_groups) = tenant_context.permissions.max_groups {
                    let current_count = self.count_resources_for_tenant(&tenant_id, "Group").await;
                    if current_count >= max_groups {
                        return Err(InMemoryError::Internal {
                            message: format!(
                                "Group limit exceeded: {}/{}",
                                current_count, max_groups
                            ),
                        });
                    }
                }
            }
        }

        // Generate ID if not provided
        if data.get("id").is_none() {
            let id = self.generate_resource_id(&tenant_id, resource_type).await;
            if let Some(obj) = data.as_object_mut() {
                obj.insert("id".to_string(), json!(id));
            }
        }

        // Create resource
        let resource = Resource::from_json(resource_type.to_string(), data).map_err(|e| {
            InMemoryError::InvalidData {
                message: format!("Failed to create resource: {}", e),
            }
        })?;

        // Check for duplicate userName if this is a User resource
        if resource_type == "User" {
            if let Some(username) = resource.get_username() {
                self.check_username_duplicate(&tenant_id, username, None)
                    .await?;
            }
        }

        // Add metadata
        let resource_with_meta = self.add_scim_metadata(resource);
        let resource_id = resource_with_meta.get_id().unwrap_or("unknown").to_string();

        // Store resource
        let mut data_guard = self.data.write().await;
        data_guard
            .entry(tenant_id.clone())
            .or_insert_with(HashMap::new)
            .entry(resource_type.to_string())
            .or_insert_with(HashMap::new)
            .insert(resource_id.clone(), resource_with_meta.clone());

        Ok(resource_with_meta)
    }

    async fn get_resource(
        &self,
        resource_type: &str,
        id: &str,
        context: &RequestContext,
    ) -> Result<Option<Resource>, Self::Error> {
        let tenant_id = self.effective_tenant_id(context);

        debug!(
            "Getting {} resource with ID '{}' for tenant '{}' (request: '{}')",
            resource_type, id, tenant_id, context.request_id
        );

        // Check permissions first
        context
            .validate_operation("read")
            .map_err(|e| InMemoryError::Internal { message: e })?;

        let data_guard = self.data.read().await;
        let resource = data_guard
            .get(&tenant_id)
            .and_then(|tenant_data| tenant_data.get(resource_type))
            .and_then(|type_data| type_data.get(id))
            .cloned();

        if resource.is_some() {
            trace!("Resource found and returned");
        } else {
            debug!("Resource not found");
        }

        Ok(resource)
    }

    async fn update_resource(
        &self,
        resource_type: &str,
        id: &str,
        mut data: Value,
        context: &RequestContext,
    ) -> Result<Resource, Self::Error> {
        let tenant_id = self.effective_tenant_id(context);

        info!(
            "Updating {} resource with ID '{}' for tenant '{}' (request: '{}')",
            resource_type, id, tenant_id, context.request_id
        );
        trace!(
            "Update data: {}",
            serde_json::to_string(&data).unwrap_or_else(|_| "invalid json".to_string())
        );

        // Check permissions first
        context
            .validate_operation("update")
            .map_err(|e| InMemoryError::Internal { message: e })?;

        // Ensure ID is set correctly
        if let Some(obj) = data.as_object_mut() {
            obj.insert("id".to_string(), json!(id));
        }

        // Create updated resource
        let resource = Resource::from_json(resource_type.to_string(), data).map_err(|e| {
            InMemoryError::InvalidData {
                message: format!("Failed to update resource: {}", e),
            }
        })?;

        // Check for duplicate userName if this is a User resource
        if resource_type == "User" {
            if let Some(username) = resource.get_username() {
                self.check_username_duplicate(&tenant_id, username, Some(id))
                    .await?;
            }
        }

        // Verify resource exists
        {
            let data_guard = self.data.read().await;
            let exists = data_guard
                .get(&tenant_id)
                .and_then(|tenant_data| tenant_data.get(resource_type))
                .and_then(|type_data| type_data.get(id))
                .is_some();

            if !exists {
                return Err(InMemoryError::ResourceNotFound {
                    resource_type: resource_type.to_string(),
                    id: id.to_string(),
                    tenant_id,
                });
            }
        }

        // Add metadata (preserve created time, update modified time)
        let resource_with_meta = self.add_scim_metadata(resource);

        // Store updated resource
        let mut data_guard = self.data.write().await;
        data_guard
            .get_mut(&tenant_id)
            .and_then(|tenant_data| tenant_data.get_mut(resource_type))
            .and_then(|type_data| type_data.insert(id.to_string(), resource_with_meta.clone()));

        Ok(resource_with_meta)
    }

    async fn delete_resource(
        &self,
        resource_type: &str,
        id: &str,
        context: &RequestContext,
    ) -> Result<(), Self::Error> {
        let tenant_id = self.effective_tenant_id(context);

        info!(
            "Deleting {} resource with ID '{}' for tenant '{}' (request: '{}')",
            resource_type, id, tenant_id, context.request_id
        );

        // Check permissions first
        context
            .validate_operation("delete")
            .map_err(|e| InMemoryError::Internal { message: e })?;

        let mut data_guard = self.data.write().await;
        let removed = data_guard
            .get_mut(&tenant_id)
            .and_then(|tenant_data| tenant_data.get_mut(resource_type))
            .and_then(|type_data| type_data.remove(id))
            .is_some();

        if !removed {
            warn!(
                "Attempted to delete non-existent {} resource with ID '{}' for tenant '{}'",
                resource_type, id, tenant_id
            );
            return Err(InMemoryError::ResourceNotFound {
                resource_type: resource_type.to_string(),
                id: id.to_string(),
                tenant_id,
            });
        }

        debug!(
            "Successfully deleted {} resource with ID '{}' for tenant '{}'",
            resource_type, id, tenant_id
        );
        Ok(())
    }

    async fn list_resources(
        &self,
        resource_type: &str,
        query: Option<&ListQuery>,
        context: &RequestContext,
    ) -> Result<Vec<Resource>, Self::Error> {
        let tenant_id = self.effective_tenant_id(context);

        debug!(
            "Listing {} resources for tenant '{}' (request: '{}')",
            resource_type, tenant_id, context.request_id
        );

        // Check permissions first
        context
            .validate_operation("list")
            .map_err(|e| InMemoryError::Internal { message: e })?;

        let data_guard = self.data.read().await;
        let resources: Vec<Resource> = data_guard
            .get(&tenant_id)
            .and_then(|tenant_data| tenant_data.get(resource_type))
            .map(|type_data| type_data.values().cloned().collect())
            .unwrap_or_default();

        // Apply simple filtering and pagination if query is provided
        let mut filtered_resources = resources;

        if let Some(q) = query {
            // Apply start_index and count for pagination
            if let Some(start_index) = q.start_index {
                let start = (start_index.saturating_sub(1)) as usize; // SCIM uses 1-based indexing
                if start < filtered_resources.len() {
                    filtered_resources = filtered_resources.into_iter().skip(start).collect();
                } else {
                    filtered_resources = Vec::new();
                }
            }

            if let Some(count) = q.count {
                filtered_resources.truncate(count as usize);
            }
        }

        debug!(
            "Found {} {} resources for tenant '{}' (after filtering)",
            filtered_resources.len(),
            resource_type,
            tenant_id
        );

        Ok(filtered_resources)
    }

    async fn find_resource_by_attribute(
        &self,
        resource_type: &str,
        attribute: &str,
        value: &Value,
        context: &RequestContext,
    ) -> Result<Option<Resource>, Self::Error> {
        let tenant_id = self.effective_tenant_id(context);

        let data_guard = self.data.read().await;
        if let Some(tenant_data) = data_guard.get(&tenant_id) {
            if let Some(type_data) = tenant_data.get(resource_type) {
                for resource in type_data.values() {
                    // Handle special structured fields
                    let found_match = match attribute {
                        "userName" => {
                            if let Some(username) = resource.get_username() {
                                &Value::String(username.to_string()) == value
                            } else {
                                false
                            }
                        }
                        "id" => {
                            if let Some(id) = resource.get_id() {
                                &Value::String(id.to_string()) == value
                            } else {
                                false
                            }
                        }
                        // For other attributes, check the attributes map
                        _ => {
                            if let Some(attr_value) = resource.get_attribute(attribute) {
                                attr_value == value
                            } else {
                                false
                            }
                        }
                    };

                    if found_match {
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
        let tenant_id = self.effective_tenant_id(context);

        let data_guard = self.data.read().await;
        let exists = data_guard
            .get(&tenant_id)
            .and_then(|tenant_data| tenant_data.get(resource_type))
            .and_then(|type_data| type_data.get(id))
            .is_some();

        Ok(exists)
    }
}

// Essential conditional operations for testing
impl InMemoryProvider {
    pub async fn conditional_update(
        &self,
        resource_type: &str,
        id: &str,
        data: Value,
        expected_version: &ScimVersion,
        context: &RequestContext,
    ) -> Result<ConditionalResult<VersionedResource>, InMemoryError> {
        let tenant_id = context.tenant_id().unwrap_or("default");

        let mut store = self.data.write().await;
        let tenant_data = store
            .entry(tenant_id.to_string())
            .or_insert_with(HashMap::new);
        let type_data = tenant_data
            .entry(resource_type.to_string())
            .or_insert_with(HashMap::new);

        // Check if resource exists
        let existing_resource = match type_data.get(id) {
            Some(resource) => resource,
            None => return Ok(ConditionalResult::NotFound),
        };

        // Compute current version
        let current_version = VersionedResource::new(existing_resource.clone())
            .version()
            .clone();

        // Check version match
        if !current_version.matches(expected_version) {
            let conflict = VersionConflict::new(
                expected_version.clone(),
                current_version,
                format!(
                    "Resource {}/{} was modified by another client",
                    resource_type, id
                ),
            );
            return Ok(ConditionalResult::VersionMismatch(conflict));
        }

        // Create updated resource
        let mut updated_resource =
            Resource::from_json(resource_type.to_string(), data).map_err(|e| {
                InMemoryError::InvalidData {
                    message: format!("Failed to update resource: {}", e),
                }
            })?;

        // Preserve ID
        if let Some(original_id) = existing_resource.get_id() {
            updated_resource
                .set_id(original_id)
                .map_err(|e| InMemoryError::InvalidData {
                    message: format!("Failed to set ID: {}", e),
                })?;
        }

        type_data.insert(id.to_string(), updated_resource.clone());
        Ok(ConditionalResult::Success(VersionedResource::new(
            updated_resource,
        )))
    }

    pub async fn conditional_delete(
        &self,
        resource_type: &str,
        id: &str,
        expected_version: &ScimVersion,
        context: &RequestContext,
    ) -> Result<ConditionalResult<()>, InMemoryError> {
        let tenant_id = context.tenant_id().unwrap_or("default");

        let mut store = self.data.write().await;
        let tenant_data = store
            .entry(tenant_id.to_string())
            .or_insert_with(HashMap::new);
        let type_data = tenant_data
            .entry(resource_type.to_string())
            .or_insert_with(HashMap::new);

        // Check if resource exists
        let existing_resource = match type_data.get(id) {
            Some(resource) => resource,
            None => return Ok(ConditionalResult::NotFound),
        };

        // Compute current version
        let current_version = VersionedResource::new(existing_resource.clone())
            .version()
            .clone();

        // Check version match
        if !current_version.matches(expected_version) {
            let conflict = VersionConflict::new(
                expected_version.clone(),
                current_version,
                format!(
                    "Resource {}/{} was modified by another client",
                    resource_type, id
                ),
            );
            return Ok(ConditionalResult::VersionMismatch(conflict));
        }

        // Delete resource
        type_data.remove(id);
        Ok(ConditionalResult::Success(()))
    }

    pub async fn get_versioned_resource(
        &self,
        resource_type: &str,
        id: &str,
        context: &RequestContext,
    ) -> Result<Option<VersionedResource>, InMemoryError> {
        match self.get_resource(resource_type, id, context).await? {
            Some(resource) => Ok(Some(VersionedResource::new(resource))),
            None => Ok(None),
        }
    }

    pub async fn create_versioned_resource(
        &self,
        resource_type: &str,
        data: Value,
        context: &RequestContext,
    ) -> Result<VersionedResource, InMemoryError> {
        let resource = self.create_resource(resource_type, data, context).await?;
        Ok(VersionedResource::new(resource))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::resource::{ListQuery, TenantContext};
    use serde_json::json;

    fn create_test_user_data(username: &str) -> Value {
        json!({
            "userName": username,
            "displayName": format!("User {}", username),
            "active": true
        })
    }

    #[tokio::test]
    async fn test_single_tenant_operations() {
        let provider = InMemoryProvider::new();
        let context = RequestContext::with_generated_id();

        // Create user
        let user_data = create_test_user_data("john.doe");
        let user = provider
            .create_resource("User", user_data, &context)
            .await
            .unwrap();
        let user_id = user.get_id().unwrap();

        // Get user
        let retrieved = provider
            .get_resource("User", user_id, &context)
            .await
            .unwrap();
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().get_username(), Some("john.doe"));

        // Update user
        let update_data = json!({
            "userName": "john.doe",
            "displayName": "John Updated",
            "active": false
        });
        let _updated = provider
            .update_resource("User", user_id, update_data, &context)
            .await
            .unwrap();
        // Check that the resource was updated (we'll verify via getting it back)
        let verified = provider
            .get_resource("User", user_id, &context)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(
            verified.get_attribute("displayName"),
            Some(&json!("John Updated"))
        );

        // List users
        let query = ListQuery::default();
        let users = provider
            .list_resources("User", Some(&query), &context)
            .await
            .unwrap();
        assert_eq!(users.len(), 1);

        // Delete user
        provider
            .delete_resource("User", user_id, &context)
            .await
            .unwrap();
        let deleted = provider
            .get_resource("User", user_id, &context)
            .await
            .unwrap();
        assert!(deleted.is_none());
    }

    #[tokio::test]
    async fn test_multi_tenant_isolation() {
        let provider = InMemoryProvider::new();

        // Create users in different tenants
        let tenant_a_context = TenantContext::new("tenant-a".to_string(), "client-a".to_string());
        let context_a = RequestContext::with_tenant_generated_id(tenant_a_context);

        let tenant_b_context = TenantContext::new("tenant-b".to_string(), "client-b".to_string());
        let context_b = RequestContext::with_tenant_generated_id(tenant_b_context);

        // Create user in tenant A
        let user_a_data = create_test_user_data("alice.tenant.a");
        let user_a = provider
            .create_resource("User", user_a_data, &context_a)
            .await
            .unwrap();
        let _user_a_id = user_a.get_id().unwrap();

        // Create user in tenant B (different username)
        let user_b_data = create_test_user_data("alice.tenant.b");
        let user_b = provider
            .create_resource("User", user_b_data, &context_b)
            .await
            .unwrap();
        let _user_b_id = user_b.get_id().unwrap();

        // Verify isolation using username search - tenant B should not find tenant A's user
        let alice_a_from_b = provider
            .find_resource_by_attribute("User", "userName", &json!("alice.tenant.a"), &context_b)
            .await
            .unwrap();
        assert!(
            alice_a_from_b.is_none(),
            "Tenant B should not find tenant A's user by username"
        );

        // Verify tenant A should not find tenant B's user
        let alice_b_from_a = provider
            .find_resource_by_attribute("User", "userName", &json!("alice.tenant.b"), &context_a)
            .await
            .unwrap();
        assert!(
            alice_b_from_a.is_none(),
            "Tenant A should not find tenant B's user by username"
        );

        // Each tenant should find their own user
        let alice_a_from_a = provider
            .find_resource_by_attribute("User", "userName", &json!("alice.tenant.a"), &context_a)
            .await
            .unwrap();
        assert!(
            alice_a_from_a.is_some(),
            "Tenant A should find its own user"
        );

        let alice_b_from_b = provider
            .find_resource_by_attribute("User", "userName", &json!("alice.tenant.b"), &context_b)
            .await
            .unwrap();
        assert!(
            alice_b_from_b.is_some(),
            "Tenant B should find its own user"
        );

        // Each tenant should see only their own user
        let query = ListQuery::default();
        let users_a = provider
            .list_resources("User", Some(&query), &context_a)
            .await
            .unwrap();
        let users_b = provider
            .list_resources("User", Some(&query), &context_b)
            .await
            .unwrap();

        assert_eq!(users_a.len(), 1);
        assert_eq!(users_b.len(), 1);
        assert_eq!(users_a[0].get_username(), Some("alice.tenant.a"));
        assert_eq!(users_b[0].get_username(), Some("alice.tenant.b"));
    }

    #[tokio::test]
    async fn test_username_duplicate_detection() {
        let provider = InMemoryProvider::new();
        let context = RequestContext::with_generated_id();

        // Create first user
        let user1_data = create_test_user_data("duplicate");
        let _user1 = provider
            .create_resource("User", user1_data, &context)
            .await
            .unwrap();

        // Attempt to create second user with same username
        let user2_data = create_test_user_data("duplicate");
        let result = provider.create_resource("User", user2_data, &context).await;

        assert!(result.is_err());
        match result.unwrap_err() {
            InMemoryError::DuplicateAttribute {
                attribute, value, ..
            } => {
                assert_eq!(attribute, "userName");
                assert_eq!(value, "duplicate");
            }
            _ => panic!("Expected DuplicateAttribute error"),
        }
    }

    #[tokio::test]
    async fn test_cross_tenant_username_allowed() {
        let provider = InMemoryProvider::new();

        let tenant_a_context = TenantContext::new("tenant-a".to_string(), "client-a".to_string());
        let context_a = RequestContext::with_tenant_generated_id(tenant_a_context);

        let tenant_b_context = TenantContext::new("tenant-b".to_string(), "client-b".to_string());
        let context_b = RequestContext::with_tenant_generated_id(tenant_b_context);

        // Create user with same username in both tenants - should succeed
        let user_data = create_test_user_data("shared.name");

        let user_a = provider
            .create_resource("User", user_data.clone(), &context_a)
            .await
            .unwrap();
        let user_b = provider
            .create_resource("User", user_data, &context_b)
            .await
            .unwrap();

        assert_eq!(user_a.get_username(), Some("shared.name"));
        assert_eq!(user_b.get_username(), Some("shared.name"));
    }

    #[tokio::test]
    async fn test_find_resource_by_attribute() {
        let provider = InMemoryProvider::new();
        let context = RequestContext::with_generated_id();

        // Create a user
        let user_data = create_test_user_data("john.doe");
        let _user = provider
            .create_resource("User", user_data, &context)
            .await
            .unwrap();

        // Find by userName
        let found = provider
            .find_resource_by_attribute("User", "userName", &json!("john.doe"), &context)
            .await
            .unwrap();

        assert!(found.is_some());
        assert_eq!(found.unwrap().get_username(), Some("john.doe"));

        // Find by non-existent attribute value
        let not_found = provider
            .find_resource_by_attribute("User", "userName", &json!("nonexistent"), &context)
            .await
            .unwrap();

        assert!(not_found.is_none());
    }

    #[tokio::test]
    async fn test_resource_exists() {
        let provider = InMemoryProvider::new();
        let context = RequestContext::with_generated_id();

        // Create a user
        let user_data = create_test_user_data("test.user");
        let user = provider
            .create_resource("User", user_data, &context)
            .await
            .unwrap();
        let user_id = user.get_id().unwrap();

        // Check existence
        let exists = provider
            .resource_exists("User", user_id, &context)
            .await
            .unwrap();
        assert!(exists);

        // Check non-existent resource
        let not_exists = provider
            .resource_exists("User", "nonexistent-id", &context)
            .await
            .unwrap();
        assert!(!not_exists);

        // Delete and check again
        provider
            .delete_resource("User", user_id, &context)
            .await
            .unwrap();

        let exists_after_delete = provider
            .resource_exists("User", user_id, &context)
            .await
            .unwrap();
        assert!(!exists_after_delete);
    }

    #[tokio::test]
    async fn test_provider_stats() {
        let provider = InMemoryProvider::new();

        let tenant_a_context = TenantContext::new("tenant-a".to_string(), "client-a".to_string());
        let context_a = RequestContext::with_tenant_generated_id(tenant_a_context);

        let single_context = RequestContext::with_generated_id();

        // Create resources in different tenants and types
        let _user1 = provider
            .create_resource("User", create_test_user_data("user1"), &context_a)
            .await
            .unwrap();
        let _user2 = provider
            .create_resource("User", create_test_user_data("user2"), &single_context)
            .await
            .unwrap();
        let _group = provider
            .create_resource("Group", json!({"displayName": "Test Group"}), &context_a)
            .await
            .unwrap();

        let final_stats = provider.get_stats().await;
        assert_eq!(final_stats.tenant_count, 2); // tenant-a and default
        assert_eq!(final_stats.total_resources, 3);
        assert_eq!(final_stats.resource_type_count, 2); // User and Group
        assert!(final_stats.resource_types.contains(&"User".to_string()));
        assert!(final_stats.resource_types.contains(&"Group".to_string()));
    }

    #[tokio::test]
    async fn test_clear_functionality() {
        let provider = InMemoryProvider::new();
        let context = RequestContext::with_generated_id();

        // Create some data
        let _user = provider
            .create_resource("User", create_test_user_data("test"), &context)
            .await
            .unwrap();

        let stats_before = provider.get_stats().await;
        assert!(stats_before.total_resources > 0);

        // Clear all data
        provider.clear().await;

        let stats_after = provider.get_stats().await;
        assert_eq!(stats_after.total_resources, 0);
        assert_eq!(stats_after.tenant_count, 0);
    }

    #[tokio::test]
    async fn test_conditional_operations_via_resource_provider() {
        // This test ensures InMemoryProvider implements conditional operations via ResourceProvider trait
        // Using static dispatch with generic function to enforce trait bounds
        async fn test_provider<P>(provider: &P, context: &RequestContext)
        where
            P: ResourceProvider<Error = InMemoryError> + Sync,
        {
            // Create a user first using regular provider
            let user_data = create_test_user_data("jane.doe");
            let user = provider
                .create_resource("User", user_data, context)
                .await
                .unwrap();
            let user_id = user.get_id().unwrap();

            // Get versioned resource using trait method
            let versioned = provider
                .get_versioned_resource("User", user_id, context)
                .await
                .unwrap()
                .unwrap();

            // Test conditional update using trait method
            let update_data = json!({
                "userName": "jane.doe",
                "displayName": "Jane Updated",
                "active": false
            });

            let result = provider
                .conditional_update("User", user_id, update_data, versioned.version(), context)
                .await
                .unwrap();

            // Should succeed since version matches
            assert!(matches!(result, ConditionalResult::Success(_)));
        }

        let provider = InMemoryProvider::new();
        let context = RequestContext::with_generated_id();

        // This tests that conditional operations work via ResourceProvider trait
        test_provider(&provider, &context).await;
    }

    #[tokio::test]
    async fn test_conditional_provider_concurrent_updates() {
        use tokio::task::JoinSet;

        let provider = Arc::new(InMemoryProvider::new());
        let context = RequestContext::with_generated_id();

        // Create a user first
        let user_data = create_test_user_data("concurrent.user");
        let user = provider
            .create_resource("User", user_data, &context)
            .await
            .unwrap();
        let user_id = user.get_id().unwrap().to_string();

        // Get initial version
        let initial_versioned = provider
            .get_versioned_resource("User", &user_id, &context)
            .await
            .unwrap()
            .unwrap();

        // Launch concurrent updates with the same version
        let mut tasks = JoinSet::new();
        let num_concurrent = 10;

        for i in 0..num_concurrent {
            let provider_clone = Arc::clone(&provider);
            let context_clone = context.clone();
            let user_id_clone = user_id.clone();
            let version_clone = initial_versioned.version().clone();

            tasks.spawn(async move {
                let update_data = json!({
                    "userName": "concurrent.user",
                    "displayName": format!("Update {}", i),
                    "active": true
                });

                provider_clone
                    .conditional_update(
                        &"User",
                        &user_id_clone,
                        update_data,
                        &version_clone,
                        &context_clone,
                    )
                    .await
            });
        }

        // Collect results
        let mut success_count = 0;
        let mut conflict_count = 0;

        while let Some(result) = tasks.join_next().await {
            match result.unwrap().unwrap() {
                ConditionalResult::Success(_) => success_count += 1,
                ConditionalResult::VersionMismatch(_) => conflict_count += 1,
                ConditionalResult::NotFound => panic!("Resource should exist"),
            }
        }

        // Only one update should succeed, others should get version conflicts
        assert_eq!(success_count, 1, "Exactly one update should succeed");
        assert_eq!(
            conflict_count,
            num_concurrent - 1,
            "Other updates should conflict"
        );
    }

    #[tokio::test]
    async fn test_conditional_provider_delete_version_conflict() {
        let provider = InMemoryProvider::new();
        let context = RequestContext::with_generated_id();

        // Create a user
        let user_data = create_test_user_data("delete.user");
        let user = provider
            .create_resource("User", user_data, &context)
            .await
            .unwrap();
        let user_id = user.get_id().unwrap();

        // Get initial version
        let initial_versioned = provider
            .get_versioned_resource("User", user_id, &context)
            .await
            .unwrap()
            .unwrap();

        // Update the resource to change its version
        let update_data = json!({
            "userName": "delete.user",
            "displayName": "Updated Before Delete",
            "active": false
        });
        provider
            .update_resource("User", user_id, update_data, &context)
            .await
            .unwrap();

        // Try to delete with old version - should fail
        let delete_result = provider
            .conditional_delete("User", user_id, initial_versioned.version(), &context)
            .await
            .unwrap();

        // Should get version conflict
        assert!(matches!(
            delete_result,
            ConditionalResult::VersionMismatch(_)
        ));

        // Resource should still exist
        let still_exists = provider
            .resource_exists("User", user_id, &context)
            .await
            .unwrap();
        assert!(still_exists);
    }

    #[tokio::test]
    async fn test_conditional_provider_successful_delete() {
        let provider = InMemoryProvider::new();
        let context = RequestContext::with_generated_id();

        // Create a user
        let user_data = create_test_user_data("delete.success");
        let user = provider
            .create_resource("User", user_data, &context)
            .await
            .unwrap();
        let user_id = user.get_id().unwrap();

        // Get current version
        let current_versioned = provider
            .get_versioned_resource("User", user_id, &context)
            .await
            .unwrap()
            .unwrap();

        // Delete with correct version - should succeed
        let delete_result = provider
            .conditional_delete("User", user_id, current_versioned.version(), &context)
            .await
            .unwrap();

        // Should succeed
        assert!(matches!(delete_result, ConditionalResult::Success(())));

        // Resource should no longer exist
        let exists = provider
            .resource_exists("User", user_id, &context)
            .await
            .unwrap();
        assert!(!exists);
    }
}
