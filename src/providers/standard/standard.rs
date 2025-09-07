//! Standard resource provider implementation with pluggable storage.
//!
//! This module provides a production-ready implementation of the ResourceProvider
//! trait that separates SCIM protocol logic from storage concerns through the
//! StorageProvider interface.
//!
//! # Features
//!
//! * Pluggable storage backends through the StorageProvider trait
//! * Complete SCIM protocol logic preservation
//! * Automatic tenant isolation when tenant context is provided
//! * Fallback to "default" tenant for single-tenant operations
//! * Comprehensive error handling
//! * Resource metadata tracking (created/updated timestamps)
//! * Duplicate detection for userName attributes
//!
//! # Example Usage
//!
//! ```rust
//! use scim_server::providers::StandardResourceProvider;
//! use scim_server::storage::InMemoryStorage;
//! use scim_server::resource::{RequestContext, TenantContext};
//! use scim_server::providers::ResourceProvider;
//! use serde_json::json;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let storage = InMemoryStorage::new();
//! let provider = StandardResourceProvider::new(storage);
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

use crate::providers::ProviderError;
use crate::providers::ResourceProvider;
use crate::providers::helpers::{
    metadata::ScimMetadataManager, patch::ScimPatchOperations, tenant::MultiTenantProvider,
};
use crate::resource::{
    ListQuery, RequestContext, Resource, version::RawVersion, versioned::VersionedResource,
};
use crate::storage::ProviderStats;
use crate::storage::{StorageKey, StorageProvider};
use log::{debug, info, trace, warn};
use serde_json::{Value, json};

/// Standard resource provider with pluggable storage backend.
///
/// This provider separates SCIM protocol logic from storage concerns by delegating
/// data persistence to a StorageProvider implementation while handling all SCIM-specific
/// business logic, validation, and metadata management.
#[derive(Debug, Clone)]
pub struct StandardResourceProvider<S: StorageProvider> {
    // Pluggable storage backend
    storage: S,
}

impl<S: StorageProvider> StandardResourceProvider<S> {
    /// Create a new standard provider with the given storage backend.
    pub fn new(storage: S) -> Self {
        Self { storage }
    }

    /// Check for duplicate userName in User resources within the same tenant.
    async fn check_username_duplicate(
        &self,
        tenant_id: &str,
        username: &str,
        exclude_id: Option<&str>,
    ) -> Result<(), ProviderError> {
        let prefix = StorageKey::prefix(tenant_id, "User");
        let matches = self
            .storage
            .find_by_attribute(prefix, "userName", username)
            .await
            .map_err(|e| ProviderError::Internal {
                message: format!("Storage error during username check: {}", e),
            })?;

        for (key, _data) in matches {
            // Skip the resource we're updating
            if Some(key.resource_id()) != exclude_id {
                return Err(ProviderError::DuplicateAttribute {
                    resource_type: "User".to_string(),
                    attribute: "userName".to_string(),
                    value: username.to_string(),
                    tenant_id: tenant_id.to_string(),
                });
            }
        }

        Ok(())
    }

    /// Clear all data from storage.
    ///
    /// Removes all resources from all tenants by delegating to the storage backend's
    /// clear operation. This method provides a consistent interface for clearing data
    /// regardless of the underlying storage implementation.
    ///
    /// # Behavior
    ///
    /// - Delegates to [`StorageProvider::clear`] for actual data removal
    /// - Logs warnings if the clear operation fails
    /// - Primarily intended for testing scenarios
    /// - After successful clearing, [`get_stats`] should report zero resources
    ///
    /// # Examples
    ///
    /// ```rust
    /// use scim_server::providers::StandardResourceProvider;
    /// use scim_server::storage::InMemoryStorage;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let storage = InMemoryStorage::new();
    /// let provider = StandardResourceProvider::new(storage);
    ///
    /// // ... create some resources ...
    /// provider.clear().await;
    ///
    /// let stats = provider.get_stats().await;
    /// assert_eq!(stats.total_resources, 0);
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// [`StorageProvider::clear`]: crate::storage::StorageProvider::clear
    /// [`get_stats`]: Self::get_stats
    pub async fn clear(&self) {
        // Delegate to storage backend for proper clearing
        if let Err(e) = self.storage.clear().await {
            warn!("Failed to clear storage: {:?}", e);
        }
    }

    /// Get comprehensive statistics about stored data across all tenants.
    ///
    /// Dynamically discovers all tenants and resource types from storage to provide
    /// accurate statistics without relying on hardcoded patterns. This method uses
    /// the storage provider's discovery capabilities to enumerate actual data.
    ///
    /// # Returns
    ///
    /// [`ProviderStats`] containing:
    /// - `tenant_count`: Number of tenants with at least one resource
    /// - `total_resources`: Sum of all resources across all tenants and types
    /// - `resource_type_count`: Number of distinct resource types found
    /// - `resource_types`: List of all resource type names
    ///
    /// # Errors
    ///
    /// This method handles storage errors gracefully by using default values
    /// (empty collections) when discovery operations fail.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use scim_server::providers::StandardResourceProvider;
    /// use scim_server::storage::InMemoryStorage;
    /// use scim_server::resource::{RequestContext, TenantContext};
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let storage = InMemoryStorage::new();
    /// let provider = StandardResourceProvider::new(storage);
    ///
    /// // ... create resources in multiple tenants ...
    ///
    /// let stats = provider.get_stats().await;
    /// println!("Total resources: {}", stats.total_resources);
    /// println!("Active tenants: {}", stats.tenant_count);
    /// println!("Resource types: {:?}", stats.resource_types);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn get_stats(&self) -> ProviderStats {
        // Dynamically discover all tenants and resource types from storage
        let tenants = self.storage.list_tenants().await.unwrap_or_default();
        let resource_types = self
            .storage
            .list_all_resource_types()
            .await
            .unwrap_or_default();

        let mut total_resources = 0;

        // Count total resources across all tenants and resource types
        for tenant_id in &tenants {
            for resource_type in &resource_types {
                let prefix = StorageKey::prefix(tenant_id, resource_type);
                if let Ok(count) = self.storage.count(prefix).await {
                    total_resources += count;
                }
            }
        }

        ProviderStats {
            tenant_count: tenants.len(),
            total_resources,
            resource_type_count: resource_types.len(),
            resource_types,
        }
    }

    /// List all resources of a specific type in a tenant.
    pub async fn list_resources_in_tenant(
        &self,
        tenant_id: &str,
        resource_type: &str,
    ) -> Vec<Resource> {
        let prefix = StorageKey::prefix(tenant_id, resource_type);
        match self.storage.list(prefix, 0, usize::MAX).await {
            Ok(storage_results) => {
                let mut resources = Vec::new();
                for (_key, data) in storage_results {
                    match Resource::from_json(resource_type.to_string(), data) {
                        Ok(resource) => resources.push(resource),
                        Err(e) => {
                            warn!(
                                "Failed to deserialize resource in list_resources_in_tenant: {}",
                                e
                            );
                        }
                    }
                }
                resources
            }
            Err(e) => {
                warn!("Storage error in list_resources_in_tenant: {}", e);
                Vec::new()
            }
        }
    }

    /// Count resources of a specific type for a tenant (used for limit checking).
    async fn count_resources_for_tenant(&self, tenant_id: &str, resource_type: &str) -> usize {
        let prefix = StorageKey::prefix(tenant_id, resource_type);
        match self.storage.count(prefix).await {
            Ok(count) => count,
            Err(e) => {
                warn!("Storage error in count_resources_for_tenant: {}", e);
                0
            }
        }
    }
}

// Note: No Default implementation for StandardResourceProvider as it requires storage parameter

// Reuse error and stats types from the in_memory module for compatibility

// Helper traits are automatically implemented via blanket implementations
// since StandardResourceProvider implements ResourceProvider and ProviderError implements From<String>

impl<S: StorageProvider> ResourceProvider for StandardResourceProvider<S> {
    type Error = ProviderError;

    async fn create_resource(
        &self,
        resource_type: &str,
        mut data: Value,
        context: &RequestContext,
    ) -> Result<VersionedResource, Self::Error> {
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
            .map_err(|e| ProviderError::Internal { message: e })?;

        // Check resource limits if this is a multi-tenant context
        if let Some(tenant_context) = &context.tenant_context {
            if resource_type == "User" {
                if let Some(max_users) = tenant_context.permissions.max_users {
                    let current_count = self.count_resources_for_tenant(&tenant_id, "User").await;
                    if current_count >= max_users {
                        return Err(ProviderError::Internal {
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
                        return Err(ProviderError::Internal {
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
            let id = self.generate_tenant_resource_id(&tenant_id, resource_type);
            if let Some(obj) = data.as_object_mut() {
                obj.insert("id".to_string(), json!(id));
            }
        }

        // Create resource
        let resource = Resource::from_json(resource_type.to_string(), data).map_err(|e| {
            ProviderError::InvalidData {
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

        // Add metadata using ScimMetadataManager trait
        let mut resource_with_meta = resource;
        self.add_creation_metadata(&mut resource_with_meta, "https://example.com/scim/v2")
            .map_err(|e| ProviderError::Internal {
                message: format!("Failed to add metadata: {}", e),
            })?;
        let resource_id = resource_with_meta.get_id().unwrap_or("unknown").to_string();

        // Store resource using storage provider
        let key = StorageKey::new(&tenant_id, resource_type, &resource_id);
        let stored_data = self
            .storage
            .put(
                key,
                resource_with_meta
                    .to_json()
                    .map_err(|e| ProviderError::Internal {
                        message: format!("Failed to serialize resource: {}", e),
                    })?,
            )
            .await
            .map_err(|e| ProviderError::Internal {
                message: format!("Storage error during create: {}", e),
            })?;

        // Return the resource as stored, wrapped in VersionedResource
        let resource =
            Resource::from_json(resource_type.to_string(), stored_data).map_err(|e| {
                ProviderError::InvalidData {
                    message: format!("Failed to deserialize stored resource: {}", e),
                }
            })?;

        Ok(VersionedResource::new(resource))
    }

    async fn get_resource(
        &self,
        resource_type: &str,
        id: &str,
        context: &RequestContext,
    ) -> Result<Option<VersionedResource>, Self::Error> {
        let tenant_id = self.effective_tenant_id(context);

        debug!(
            "Getting {} resource with ID '{}' for tenant '{}' (request: '{}')",
            resource_type, id, tenant_id, context.request_id
        );

        // Check permissions first
        context
            .validate_operation("read")
            .map_err(|e| ProviderError::Internal { message: e })?;

        let key = StorageKey::new(&tenant_id, resource_type, id);
        let resource_data = self
            .storage
            .get(key)
            .await
            .map_err(|e| ProviderError::Internal {
                message: format!("Storage error during get: {}", e),
            })?;

        let resource = match resource_data {
            Some(data) => {
                let resource =
                    Resource::from_json(resource_type.to_string(), data).map_err(|e| {
                        ProviderError::InvalidData {
                            message: format!("Failed to deserialize resource: {}", e),
                        }
                    })?;
                trace!("Resource found and returned");
                Some(VersionedResource::new(resource))
            }
            None => {
                debug!("Resource not found");
                None
            }
        };

        Ok(resource)
    }

    async fn update_resource(
        &self,
        resource_type: &str,
        id: &str,
        mut data: Value,
        expected_version: Option<&RawVersion>,
        context: &RequestContext,
    ) -> Result<VersionedResource, Self::Error> {
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
            .map_err(|e| ProviderError::Internal { message: e })?;

        // Handle version checking if expected_version is provided
        if let Some(expected_version) = expected_version {
            // Get current resource to check version
            let key = StorageKey::new(&tenant_id, resource_type, id);
            match self.storage.get(key.clone()).await {
                Ok(Some(current_data)) => {
                    // Parse current resource to extract version
                    let current_resource =
                        Resource::from_json(resource_type.to_string(), current_data.clone())
                            .map_err(|e| ProviderError::InvalidInput {
                                message: format!("Failed to deserialize stored resource: {}", e),
                            })?;

                    // Check if version matches
                    let current_version = VersionedResource::new(current_resource.clone())
                        .version()
                        .clone();

                    if &current_version != expected_version {
                        return Err(ProviderError::PreconditionFailed {
                            message: format!(
                                "Version mismatch: expected {}, got {}",
                                expected_version.as_str(),
                                current_version.as_str()
                            ),
                        });
                    }
                }
                Ok(None) => {
                    return Err(ProviderError::NotFound {
                        resource_type: resource_type.to_string(),
                        id: id.to_string(),
                    });
                }
                Err(_) => {
                    return Err(ProviderError::Internal {
                        message: "Failed to retrieve resource for version check".to_string(),
                    });
                }
            }
        }

        // Ensure ID is set correctly
        if let Some(obj) = data.as_object_mut() {
            obj.insert("id".to_string(), json!(id));
        }

        // Create updated resource
        let resource = Resource::from_json(resource_type.to_string(), data).map_err(|e| {
            ProviderError::InvalidData {
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

        // Verify resource exists using storage provider
        let key = StorageKey::new(&tenant_id, resource_type, id);
        let exists =
            self.storage
                .exists(key.clone())
                .await
                .map_err(|e| ProviderError::Internal {
                    message: format!("Storage error during existence check: {}", e),
                })?;

        if !exists {
            return Err(ProviderError::ResourceNotFound {
                resource_type: resource_type.to_string(),
                id: id.to_string(),
                tenant_id,
            });
        }

        // Add metadata using ScimMetadataManager trait (preserve created time, update modified time)
        let mut resource_with_meta = resource;
        self.update_modification_metadata(&mut resource_with_meta)
            .map_err(|e| ProviderError::Internal {
                message: format!("Failed to update metadata: {}", e),
            })?;

        // Store updated resource using storage provider
        let stored_data = self
            .storage
            .put(
                key,
                resource_with_meta
                    .to_json()
                    .map_err(|e| ProviderError::Internal {
                        message: format!("Failed to serialize resource: {}", e),
                    })?,
            )
            .await
            .map_err(|e| ProviderError::Internal {
                message: format!("Storage error during update: {}", e),
            })?;

        // Return the updated resource as stored, wrapped in VersionedResource
        let resource =
            Resource::from_json(resource_type.to_string(), stored_data).map_err(|e| {
                ProviderError::InvalidData {
                    message: format!("Failed to deserialize updated resource: {}", e),
                }
            })?;

        Ok(VersionedResource::new(resource))
    }

    async fn delete_resource(
        &self,
        resource_type: &str,
        id: &str,
        expected_version: Option<&RawVersion>,
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
            .map_err(|e| ProviderError::Internal { message: e })?;

        // Handle version checking if expected_version is provided
        if let Some(expected_version) = expected_version {
            // Get current resource to check version
            let key = StorageKey::new(&tenant_id, resource_type, id);
            match self.storage.get(key.clone()).await {
                Ok(Some(current_data)) => {
                    // Parse current resource to extract version
                    let current_resource =
                        Resource::from_json(resource_type.to_string(), current_data.clone())
                            .map_err(|e| ProviderError::InvalidInput {
                                message: format!("Failed to deserialize stored resource: {}", e),
                            })?;

                    // Check if version matches
                    let current_version = VersionedResource::new(current_resource.clone())
                        .version()
                        .clone();

                    if &current_version != expected_version {
                        return Err(ProviderError::PreconditionFailed {
                            message: format!(
                                "Version mismatch: expected {}, got {}",
                                expected_version.as_str(),
                                current_version.as_str()
                            ),
                        });
                    }
                }
                Ok(None) => {
                    return Err(ProviderError::NotFound {
                        resource_type: resource_type.to_string(),
                        id: id.to_string(),
                    });
                }
                Err(_) => {
                    return Err(ProviderError::Internal {
                        message: "Failed to retrieve resource for version check".to_string(),
                    });
                }
            }
        }

        // Delete resource using storage provider
        let key = StorageKey::new(&tenant_id, resource_type, id);
        let removed = self
            .storage
            .delete(key)
            .await
            .map_err(|e| ProviderError::Internal {
                message: format!("Storage error during delete: {}", e),
            })?;

        if !removed {
            warn!(
                "Attempted to delete non-existent {} resource with ID '{}' for tenant '{}'",
                resource_type, id, tenant_id
            );
            return Err(ProviderError::ResourceNotFound {
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
    ) -> Result<Vec<VersionedResource>, Self::Error> {
        let tenant_id = self.effective_tenant_id(context);

        debug!(
            "Listing {} resources for tenant '{}' (request: '{}')",
            resource_type, tenant_id, context.request_id
        );

        // Check permissions first
        context
            .validate_operation("list")
            .map_err(|e| ProviderError::Internal { message: e })?;

        // List resources using storage provider
        let prefix = StorageKey::prefix(&tenant_id, resource_type);
        let storage_results = self
            .storage
            .list(prefix, 0, usize::MAX) // Get all resources for now, apply pagination later
            .await
            .map_err(|e| ProviderError::Internal {
                message: format!("Storage error during list: {}", e),
            })?;

        // Convert storage results to VersionedResource objects
        let mut resources = Vec::new();
        for (_key, data) in storage_results {
            match Resource::from_json(resource_type.to_string(), data) {
                Ok(resource) => resources.push(VersionedResource::new(resource)),
                Err(e) => {
                    warn!("Failed to deserialize resource during list: {}", e);
                    // Continue with other resources instead of failing entirely
                }
            }
        }

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

    async fn find_resources_by_attribute(
        &self,
        resource_type: &str,
        attribute_name: &str,
        attribute_value: &str,
        context: &RequestContext,
    ) -> Result<Vec<VersionedResource>, Self::Error> {
        let tenant_id = self.effective_tenant_id(context);

        // Find resource by attribute using storage provider
        let prefix = StorageKey::prefix(&tenant_id, resource_type);

        let matches = self
            .storage
            .find_by_attribute(prefix, attribute_name, attribute_value)
            .await
            .map_err(|e| ProviderError::Internal {
                message: format!("Storage error during find by attribute: {}", e),
            })?;

        // Return all matches as VersionedResources
        let mut results = Vec::new();
        for (_key, data) in matches {
            match Resource::from_json(resource_type.to_string(), data) {
                Ok(resource) => results.push(VersionedResource::new(resource)),
                Err(e) => {
                    warn!("Failed to deserialize resource during find: {}", e);
                    continue;
                }
            }
        }

        Ok(results)
    }

    async fn patch_resource(
        &self,
        resource_type: &str,
        id: &str,
        patch_request: &Value,
        expected_version: Option<&RawVersion>,
        context: &RequestContext,
    ) -> Result<VersionedResource, Self::Error> {
        let tenant_id = self.effective_tenant_id(context);

        // Handle version checking if expected_version is provided
        if let Some(expected_version) = expected_version {
            // Get current resource to check version
            let key = StorageKey::new(&tenant_id, resource_type, id);
            match self.storage.get(key.clone()).await {
                Ok(Some(current_data)) => {
                    // Parse current resource to extract version
                    let current_resource =
                        Resource::from_json(resource_type.to_string(), current_data.clone())
                            .map_err(|e| ProviderError::InvalidInput {
                                message: format!("Failed to deserialize stored resource: {}", e),
                            })?;

                    // Check if version matches
                    let current_version = VersionedResource::new(current_resource.clone())
                        .version()
                        .clone();

                    if &current_version != expected_version {
                        return Err(ProviderError::PreconditionFailed {
                            message: format!(
                                "Version mismatch: expected {}, got {}",
                                expected_version.as_str(),
                                current_version.as_str()
                            ),
                        });
                    }
                }
                Ok(None) => {
                    return Err(ProviderError::NotFound {
                        resource_type: resource_type.to_string(),
                        id: id.to_string(),
                    });
                }
                Err(_) => {
                    return Err(ProviderError::Internal {
                        message: "Failed to retrieve resource for version check".to_string(),
                    });
                }
            }
        }

        // Regular patch without version checking - get current resource
        let current_resource = self
            .get_resource(resource_type, id, context)
            .await?
            .ok_or_else(|| ProviderError::NotFound {
                resource_type: resource_type.to_string(),
                id: id.to_string(),
            })?;

        // Convert to JSON for patching
        let mut resource_data =
            current_resource
                .resource()
                .to_json()
                .map_err(|e| ProviderError::Internal {
                    message: format!("Failed to serialize resource for patching: {}", e),
                })?;

        // Apply patch operations using helper trait
        if let Some(operations) = patch_request.get("Operations") {
            if let Some(ops_array) = operations.as_array() {
                for operation in ops_array {
                    self.apply_patch_operation(&mut resource_data, operation)?;
                }
            }
        }

        // Parse back to Resource
        let patched_resource = Resource::from_json(resource_type.to_string(), resource_data)
            .map_err(|e| ProviderError::InvalidData {
                message: format!("Failed to create patched resource: {}", e),
            })?;

        // Store the patched resource
        let key = StorageKey::new(&tenant_id, resource_type, id);
        let patched_json = patched_resource
            .to_json()
            .map_err(|e| ProviderError::Internal {
                message: format!("Failed to serialize patched resource: {}", e),
            })?;

        self.storage
            .put(key, patched_json)
            .await
            .map_err(|e| ProviderError::Internal {
                message: format!("Storage error during patch: {}", e),
            })?;

        Ok(VersionedResource::new(patched_resource))
    }

    async fn resource_exists(
        &self,
        resource_type: &str,
        id: &str,
        context: &RequestContext,
    ) -> Result<bool, Self::Error> {
        let tenant_id = self.effective_tenant_id(context);

        let key = StorageKey::new(&tenant_id, resource_type, id);
        self.storage
            .exists(key)
            .await
            .map_err(|e| ProviderError::Internal {
                message: format!("Storage error during exists check: {}", e),
            })
    }
}
