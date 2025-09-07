//! Conditional operations helper trait for SCIM resources.
//!
//! This module provides reusable functionality for implementing version-based optimistic
//! concurrency control in SCIM ResourceProvider implementations. It handles version
//! computation, conflict detection, and conditional operation patterns.
//!
//! # Optimistic Concurrency Control
//!
//! This implementation follows SCIM and HTTP ETag patterns for:
//! - Version-based conditional updates and deletes
//! - Conflict detection and resolution
//! - Version computation using content hashing
//! - ConditionalResult handling for operation outcomes
//!
//! # Usage
//!
//! ```rust,no_run
//! use scim_server::resource::version::{RawVersion, ConditionalResult};
//!
//! // ConditionalOperations provides methods like conditional_update_resource
//! // that prevent lost updates through optimistic locking
//! let expected_version = RawVersion::from_hash("abc123");
//!
//! // The trait automatically implements conditional operations
//! // for any type that implements ResourceProvider
//! ```

use crate::providers::ResourceProvider;
use crate::resource::RequestContext;
use crate::resource::version::{ConditionalResult, RawVersion, VersionConflict};
use crate::resource::versioned::VersionedResource;
use serde_json::Value;
use std::future::Future;

/// Trait providing version-based conditional operations for SCIM resources.
///
/// This trait extends ResourceProvider with optimistic concurrency control capabilities
/// including conditional updates, deletes, and version management. Most implementers
/// can use the default implementations which provide standard conditional operation patterns.
pub trait ConditionalOperations: ResourceProvider {
    /// Perform a conditional update operation.
    ///
    /// Updates a resource only if the current version matches the expected version,
    /// preventing lost updates in concurrent scenarios. Uses optimistic locking
    /// based on resource content versioning.
    ///
    /// # Arguments
    /// * `resource_type` - The type of resource to update
    /// * `id` - The unique identifier of the resource
    /// * `data` - The updated resource data
    /// * `expected_version` - The version the client expects (for conflict detection)
    /// * `context` - Request context containing tenant information
    ///
    /// # Returns
    /// * `Success(VersionedResource)` - Update succeeded with new resource and version
    /// * `VersionMismatch(VersionConflict)` - Resource was modified by another client
    /// * `NotFound` - Resource doesn't exist
    ///
    /// # Example
    /// ```rust,no_run
    /// use scim_server::resource::version::{RawVersion, ConditionalResult};
    /// use serde_json::json;
    ///
    /// let expected_version = RawVersion::from_hash("abc123");
    /// let update_data = json!({"userName": "newname", "active": false});
    ///
    /// // ConditionalOperations automatically available on ResourceProvider implementations
    /// // match provider.conditional_update_resource("Users", "123", update_data, &expected_version, &context).await? {
    /// //     ConditionalResult::Success(versioned) => println!("Update successful"),
    /// //     ConditionalResult::VersionMismatch(conflict) => println!("Conflict detected"),
    /// //     ConditionalResult::NotFound => println!("Resource not found"),
    /// // }
    /// ```
    fn conditional_update_resource(
        &self,
        resource_type: &str,
        id: &str,
        data: Value,
        expected_version: &RawVersion,
        context: &RequestContext,
    ) -> impl Future<Output = Result<ConditionalResult<VersionedResource>, Self::Error>> + Send
    where
        Self: Sync,
    {
        async move {
            // Get current resource
            match self.get_resource(resource_type, id, context).await? {
                Some(current_resource) => {
                    // Create versioned resource to get current version
                    let versioned_current = current_resource;
                    let current_version = versioned_current.version();

                    // Check if versions match
                    if current_version != expected_version {
                        return Ok(ConditionalResult::VersionMismatch(
                            VersionConflict::standard_message(
                                expected_version.clone(),
                                current_version.clone(),
                            ),
                        ));
                    }

                    // Version matches, proceed with update
                    match self
                        .update_resource(resource_type, id, data, None, context)
                        .await
                    {
                        Ok(updated_resource) => {
                            let versioned_result = updated_resource;
                            Ok(ConditionalResult::Success(versioned_result))
                        }
                        Err(e) => Err(e),
                    }
                }
                None => Ok(ConditionalResult::NotFound),
            }
        }
    }

    /// Perform a conditional delete operation.
    ///
    /// Deletes a resource only if the current version matches the expected version,
    /// preventing accidental deletion of modified resources.
    ///
    /// # Arguments
    /// * `resource_type` - The type of resource to delete
    /// * `id` - The unique identifier of the resource
    /// * `expected_version` - The version the client expects (for conflict detection)
    /// * `context` - Request context containing tenant information
    ///
    /// # Returns
    /// * `Success(())` - Delete succeeded
    /// * `VersionMismatch(VersionConflict)` - Resource was modified by another client
    /// * `NotFound` - Resource doesn't exist
    fn conditional_delete_resource(
        &self,
        resource_type: &str,
        id: &str,
        expected_version: &RawVersion,
        context: &RequestContext,
    ) -> impl Future<Output = Result<ConditionalResult<()>, Self::Error>> + Send
    where
        Self: Sync,
    {
        async move {
            // Get current resource
            match self.get_resource(resource_type, id, context).await? {
                Some(current_resource) => {
                    // Create versioned resource to get current version
                    let versioned_current = current_resource;
                    let current_version = versioned_current.version();

                    // Check if versions match
                    if current_version != expected_version {
                        return Ok(ConditionalResult::VersionMismatch(
                            VersionConflict::standard_message(
                                expected_version.clone(),
                                current_version.clone(),
                            ),
                        ));
                    }

                    // Version matches, proceed with delete
                    match self.delete_resource(resource_type, id, None, context).await {
                        Ok(_) => Ok(ConditionalResult::Success(())),
                        Err(e) => Err(e),
                    }
                }
                None => Ok(ConditionalResult::NotFound),
            }
        }
    }

    /// Perform a conditional PATCH operation.
    ///
    /// Applies PATCH operations to a resource only if the current version matches
    /// the expected version, combining version control with incremental updates.
    ///
    /// # Arguments
    /// * `resource_type` - The type of resource to patch
    /// * `id` - The unique identifier of the resource
    /// * `patch_request` - The PATCH operations to apply
    /// * `expected_version` - The version the client expects (for conflict detection)
    /// * `context` - Request context containing tenant information
    ///
    /// # Returns
    /// * `Success(VersionedResource)` - PATCH succeeded with updated resource and version
    /// * `VersionMismatch(VersionConflict)` - Resource was modified by another client
    /// * `NotFound` - Resource doesn't exist
    fn conditional_patch_resource(
        &self,
        resource_type: &str,
        id: &str,
        patch_request: &Value,
        expected_version: &RawVersion,
        context: &RequestContext,
    ) -> impl Future<Output = Result<ConditionalResult<VersionedResource>, Self::Error>> + Send
    where
        Self: Sync,
    {
        async move {
            // Get current resource
            match self.get_resource(resource_type, id, context).await? {
                Some(current_resource) => {
                    // Create versioned resource to get current version
                    let versioned_current = current_resource;
                    let current_version = versioned_current.version();

                    // Check if versions match
                    if current_version != expected_version {
                        return Ok(ConditionalResult::VersionMismatch(
                            VersionConflict::standard_message(
                                expected_version.clone(),
                                current_version.clone(),
                            ),
                        ));
                    }

                    // Version matches, proceed with patch
                    match self
                        .patch_resource(resource_type, id, patch_request, None, context)
                        .await
                    {
                        Ok(patched_resource) => {
                            let versioned_result = patched_resource;
                            Ok(ConditionalResult::Success(versioned_result))
                        }
                        Err(e) => Err(e),
                    }
                }
                None => Ok(ConditionalResult::NotFound),
            }
        }
    }

    /// Get a resource with its version information.
    ///
    /// Retrieves a resource wrapped in a VersionedResource container that includes
    /// both the resource data and its computed version for use in conditional operations.
    ///
    /// # Arguments
    /// * `resource_type` - The type of resource to retrieve
    /// * `id` - The unique identifier of the resource
    /// * `context` - Request context containing tenant information
    ///
    /// # Returns
    /// The versioned resource if found, None if not found
    ///
    /// # Example
    /// ```rust,no_run
    /// // ConditionalOperations provides get_versioned_resource for getting resources with version info
    /// // if let Some(versioned_resource) = provider.get_versioned_resource("Users", "123", &context).await? {
    /// //     let current_version = versioned_resource.version().clone();
    /// //     // Use current_version for subsequent conditional operations
    /// // }
    /// ```
    fn get_versioned_resource(
        &self,
        resource_type: &str,
        id: &str,
        context: &RequestContext,
    ) -> impl Future<Output = Result<Option<VersionedResource>, Self::Error>> + Send
    where
        Self: Sync,
    {
        async move {
            match self.get_resource(resource_type, id, context).await? {
                Some(versioned_resource) => Ok(Some(versioned_resource)),
                None => Ok(None),
            }
        }
    }

    /// Create a resource with version information.
    ///
    /// Creates a new resource and returns it wrapped in a VersionedResource container
    /// with its initial computed version for immediate use in conditional operations.
    ///
    /// # Arguments
    /// * `resource_type` - The type of resource to create
    /// * `data` - The resource data as JSON
    /// * `context` - Request context containing tenant information
    ///
    /// # Returns
    /// The newly created versioned resource
    ///
    /// # Example
    /// ```rust,no_run
    /// use serde_json::json;
    ///
    /// let user_data = json!({"userName": "john.doe", "active": true});
    /// // ConditionalOperations provides create_versioned_resource for creating resources with version info
    /// // let versioned_user = provider.create_versioned_resource("Users", user_data, &context).await?;
    /// // let initial_version = versioned_user.version().clone();
    /// ```
    fn create_versioned_resource(
        &self,
        resource_type: &str,
        data: Value,
        context: &RequestContext,
    ) -> impl Future<Output = Result<VersionedResource, Self::Error>> + Send
    where
        Self: Sync,
    {
        async move {
            let versioned_resource = self.create_resource(resource_type, data, context).await?;
            Ok(versioned_resource)
        }
    }

    /// Check if a resource version matches the expected version.
    ///
    /// Utility method for comparing resource versions without performing operations,
    /// useful for validation or pre-flight checks.
    ///
    /// # Arguments
    /// * `resource_type` - The type of resource to check
    /// * `id` - The unique identifier of the resource
    /// * `expected_version` - The version to compare against
    /// * `context` - Request context containing tenant information
    ///
    /// # Returns
    /// * `Some(true)` - Resource exists and version matches
    /// * `Some(false)` - Resource exists but version doesn't match
    /// * `None` - Resource doesn't exist
    fn check_resource_version(
        &self,
        resource_type: &str,
        id: &str,
        expected_version: &RawVersion,
        context: &RequestContext,
    ) -> impl Future<Output = Result<Option<bool>, Self::Error>> + Send
    where
        Self: Sync,
    {
        async move {
            match self.get_resource(resource_type, id, context).await? {
                Some(versioned_resource) => {
                    Ok(Some(versioned_resource.version() == expected_version))
                }
                None => Ok(None),
            }
        }
    }

    /// Get the current version of a resource without retrieving the full resource.
    ///
    /// Optimized method for retrieving just the version information, useful for
    /// version checks without the overhead of full resource retrieval.
    ///
    /// # Arguments
    /// * `resource_type` - The type of resource to check
    /// * `id` - The unique identifier of the resource
    /// * `context` - Request context containing tenant information
    ///
    /// # Returns
    /// The current version if the resource exists, None if not found
    ///
    /// # Default Implementation
    /// The default implementation retrieves the full resource and computes the version.
    /// Implementers may override this for more efficient version-only retrieval.
    fn get_resource_version(
        &self,
        resource_type: &str,
        id: &str,
        context: &RequestContext,
    ) -> impl Future<Output = Result<Option<RawVersion>, Self::Error>> + Send
    where
        Self: Sync,
    {
        async move {
            match self.get_resource(resource_type, id, context).await? {
                Some(versioned_resource) => Ok(Some(versioned_resource.version().clone())),
                None => Ok(None),
            }
        }
    }

    /// Validate that a version is in the expected format.
    ///
    /// Checks that a version string or RawVersion follows expected patterns,
    /// useful for input validation before conditional operations.
    ///
    /// # Arguments
    /// * `version` - The version to validate
    ///
    /// # Returns
    /// `true` if the version format is acceptable
    fn is_valid_version(&self, version: &RawVersion) -> bool {
        // Basic validation - version should not be empty
        !version.as_str().trim().is_empty()
    }

    /// Create a version conflict error with standard messaging.
    ///
    /// Helper method for creating consistent version conflict errors across
    /// conditional operations.
    ///
    /// # Arguments
    /// * `expected` - The version the client expected
    /// * `current` - The actual current version on the server
    /// * `resource_info` - Optional additional context about the resource
    ///
    /// # Returns
    /// A VersionConflict with appropriate error messaging
    fn create_version_conflict(
        &self,
        expected: RawVersion,
        current: RawVersion,
        resource_info: Option<&str>,
    ) -> VersionConflict {
        let message = match resource_info {
            Some(info) => format!(
                "Resource {} was modified by another client. Expected version {}, current version {}",
                info,
                expected.as_str(),
                current.as_str()
            ),
            None => format!(
                "Resource was modified by another client. Expected version {}, current version {}",
                expected.as_str(),
                current.as_str()
            ),
        };
        VersionConflict::new(expected, current, message)
    }
}

/// Default implementation for any ResourceProvider
impl<T: ResourceProvider> ConditionalOperations for T {}
