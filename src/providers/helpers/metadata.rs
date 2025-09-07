//! SCIM metadata management helper trait.
//!
//! This module provides reusable functionality for managing SCIM resource metadata
//! including creation timestamps, versions, locations, and other RFC 7644 compliant
//! metadata fields. Any ResourceProvider can implement this trait to get consistent
//! metadata handling without reimplementing the logic.
//!
//! # RFC 7644 Compliance
//!
//! This implementation follows RFC 7644 specifications for:
//! - Resource metadata structure (`meta` attribute)
//! - Creation and modification timestamps
//! - Version computation using content hashing
//! - Location URI generation
//! - Resource type identification
//!
//! # Usage
//!
//! ```rust,no_run
//! use scim_server::resource::Resource;
//! use serde_json::json;
//!
//! // ScimMetadataManager provides automatic metadata management
//! // for ResourceProvider implementations, including:
//! // - Creation timestamps
//! // - Version computation
//! // - Location URI generation
//!
//! let user_data = json!({"userName": "john"});
//! let mut resource = Resource::from_json("User".to_string(), user_data).unwrap();
//!
//! // When implemented by a provider, adds metadata automatically:
//! // provider.add_creation_metadata(&mut resource, "https://api.example.com/scim/v2");
//! ```

use crate::providers::ResourceProvider;
use crate::resource::Resource;
use crate::resource::value_objects::Meta;
use crate::resource::version::RawVersion;
use chrono::{DateTime, Utc};

/// Trait providing SCIM resource metadata management functionality.
///
/// This trait extends ResourceProvider with metadata capabilities including
/// timestamp management, version computation, and location URI generation.
/// Most implementers can use the default implementations which provide
/// RFC-compliant behavior.
pub trait ScimMetadataManager: ResourceProvider {
    /// Add creation metadata to a new resource.
    ///
    /// Sets the initial metadata for a newly created resource including:
    /// - Creation timestamp (`meta.created`)
    /// - Last modified timestamp (`meta.lastModified`)
    /// - Resource type (`meta.resourceType`)
    /// - Location URI (`meta.location`)
    /// - Initial version (`meta.version`)
    ///
    /// # Arguments
    /// * `resource` - The resource to add metadata to
    /// * `base_url` - The base URL for generating location URIs
    ///
    /// # Example
    /// ```rust,no_run
    /// use scim_server::resource::Resource;
    /// use serde_json::json;
    ///
    /// let mut resource = Resource::from_json("User".to_string(), json!({"userName": "john"})).unwrap();
    ///
    /// // ScimMetadataManager automatically adds creation metadata:
    /// // - meta.created timestamp
    /// // - meta.lastModified timestamp
    /// // - meta.resourceType
    /// // - meta.location URI
    /// // - meta.version
    /// ```
    fn add_creation_metadata(
        &self,
        resource: &mut Resource,
        base_url: &str,
    ) -> Result<(), Self::Error> {
        let now = Utc::now();
        let resource_type = &resource.resource_type;
        let resource_id = resource
            .get_id()
            .map(|id| id.to_string())
            .unwrap_or_else(|| "unknown".to_string());

        // Generate location URI
        let location = format!("{}/{}/{}", base_url, resource_type, resource_id);

        // Compute initial version
        let version = self.compute_resource_version(resource);

        // Create metadata object
        let meta = Meta::new(
            resource_type.clone(),
            now,
            now,
            Some(location),
            Some(version.as_str().to_string()),
        )
        .map_err(|e| self.metadata_error(&format!("Failed to create metadata: {}", e)))?;

        // Set metadata on resource
        resource.set_meta(meta);

        Ok(())
    }

    /// Update modification metadata on an existing resource.
    ///
    /// Updates the metadata for a modified resource:
    /// - Updates last modified timestamp (`meta.lastModified`)
    /// - Recomputes version (`meta.version`)
    /// - Preserves creation metadata
    ///
    /// # Arguments
    /// * `resource` - The resource to update metadata for
    ///
    /// # Example
    /// ```rust,no_run
    /// use scim_server::resource::Resource;
    /// use serde_json::json;
    ///
    /// let mut resource = Resource::from_json("User".to_string(), json!({"userName": "john"})).unwrap();
    ///
    /// // ScimMetadataManager automatically updates modification metadata:
    /// // - Updates meta.lastModified timestamp
    /// // - Recomputes meta.version
    /// // - Preserves creation metadata
    /// ```
    fn update_modification_metadata(&self, resource: &mut Resource) -> Result<(), Self::Error> {
        let now = Utc::now();
        let new_version = self.compute_resource_version(resource);

        // Get existing metadata to preserve creation info
        if let Some(existing_meta) = resource.get_meta() {
            let updated_meta = Meta::new(
                existing_meta.resource_type.clone(),
                existing_meta.created,
                now,
                existing_meta.location.clone(),
                Some(new_version.as_str().to_string()),
            )
            .map_err(|e| self.metadata_error(&format!("Failed to update metadata: {}", e)))?;

            resource.set_meta(updated_meta);
        }

        Ok(())
    }

    /// Compute a version hash for a resource.
    ///
    /// Generates a deterministic version identifier based on the resource content
    /// using SHA-256 hashing. The version changes whenever the resource content
    /// changes, enabling optimistic concurrency control.
    ///
    /// # Arguments
    /// * `resource` - The resource to compute version for
    ///
    /// # Returns
    /// A `RawVersion` containing the computed hash
    fn compute_resource_version(&self, resource: &Resource) -> RawVersion {
        match resource.to_json() {
            Ok(resource_json) => {
                let content = resource_json.to_string();
                RawVersion::from_content(content.as_bytes())
            }
            Err(_) => {
                // Fallback to a timestamp-based version if serialization fails
                let timestamp = Utc::now().timestamp_millis();
                RawVersion::from_hash(&format!("timestamp-{}", timestamp))
            }
        }
    }

    /// Generate a location URI for a resource.
    ///
    /// Creates the canonical location URI for a SCIM resource following
    /// the pattern: `{base_url}/{resource_type}/{id}`
    ///
    /// # Arguments
    /// * `base_url` - The base URL of the SCIM service
    /// * `resource_type` - The type of resource (e.g., "Users", "Groups")
    /// * `id` - The unique identifier of the resource
    ///
    /// # Returns
    /// The complete location URI for the resource
    ///
    /// # Example
    /// ```rust,no_run
    /// // ScimMetadataManager generates location URIs following the pattern:
    /// // "{base_url}/{resource_type}/{id}"
    ///
    /// // Example result:
    /// // "https://api.example.com/scim/v2/Users/123e4567-e89b-12d3-a456-426614174000"
    /// ```
    fn generate_location_uri(&self, base_url: &str, resource_type: &str, id: &str) -> String {
        format!(
            "{}/{}/{}",
            base_url.trim_end_matches('/'),
            resource_type,
            id
        )
    }

    /// Extract version from resource metadata.
    ///
    /// Retrieves the current version from a resource's metadata, if present.
    /// Returns None if the resource has no metadata or version information.
    ///
    /// # Arguments
    /// * `resource` - The resource to extract version from
    ///
    /// # Returns
    /// The version as a string, or None if not present
    fn extract_resource_version(&self, resource: &Resource) -> Option<String> {
        resource
            .get_meta()
            .and_then(|meta| meta.version.as_ref())
            .map(|s| s.to_string())
    }

    /// Check if a resource has valid metadata.
    ///
    /// Validates that a resource contains the required SCIM metadata fields:
    /// - `resourceType` - matches the resource type
    /// - `created` - valid RFC 3339 timestamp
    /// - `lastModified` - valid RFC 3339 timestamp
    /// - `location` - non-empty location URI
    ///
    /// # Arguments
    /// * `resource` - The resource to validate
    ///
    /// # Returns
    /// `true` if the resource has valid metadata structure
    fn has_valid_metadata(&self, resource: &Resource) -> bool {
        if let Some(meta) = resource.get_meta() {
            // Check required fields exist
            let has_resource_type = !meta.resource_type.is_empty();
            let has_location = meta
                .location
                .as_ref()
                .map(|s| !s.is_empty())
                .unwrap_or(false);

            has_resource_type && has_location
        } else {
            false
        }
    }

    /// Refresh all metadata fields for a resource.
    ///
    /// Completely regenerates metadata for a resource, useful when:
    /// - Migrating resources between systems
    /// - Fixing corrupted metadata
    /// - Updating metadata format
    ///
    /// # Arguments
    /// * `resource` - The resource to refresh metadata for
    /// * `base_url` - The base URL for location generation
    /// * `preserve_created` - Whether to preserve the original creation timestamp
    fn refresh_metadata(
        &self,
        resource: &mut Resource,
        base_url: &str,
        preserve_created: bool,
    ) -> Result<(), Self::Error> {
        let now = Utc::now();
        let resource_type = &resource.resource_type;
        let resource_id = resource
            .get_id()
            .map(|id| id.to_string())
            .unwrap_or_else(|| "unknown".to_string());

        // Preserve creation timestamp if requested
        let created = if preserve_created {
            resource.get_meta().map(|meta| meta.created).unwrap_or(now)
        } else {
            now
        };

        let location = self.generate_location_uri(base_url, resource_type, &resource_id);
        let version = self.compute_resource_version(resource);

        let meta = Meta::new(
            resource_type.clone(),
            created,
            now,
            Some(location),
            Some(version.as_str().to_string()),
        )
        .map_err(|e| self.metadata_error(&format!("Failed to refresh metadata: {}", e)))?;

        resource.set_meta(meta);

        Ok(())
    }

    /// Strip metadata from a resource.
    ///
    /// Removes all metadata from a resource, useful for:
    /// - Creating clean templates
    /// - Preparing resources for export
    /// - Testing without metadata noise
    ///
    /// # Arguments
    /// * `resource` - The resource to strip metadata from
    fn strip_metadata(&self, resource: &mut Resource) {
        resource.meta = None;
    }

    /// Get the creation timestamp from a resource.
    ///
    /// Extracts the creation timestamp from resource metadata.
    ///
    /// # Arguments
    /// * `resource` - The resource to get creation time for
    ///
    /// # Returns
    /// The creation timestamp, or None if not present or invalid
    fn get_creation_time(&self, resource: &Resource) -> Option<DateTime<Utc>> {
        resource.get_meta().map(|meta| meta.created)
    }

    /// Get the last modification timestamp from a resource.
    ///
    /// Extracts the last modification timestamp from resource metadata.
    ///
    /// # Arguments
    /// * `resource` - The resource to get modification time for
    ///
    /// # Returns
    /// The last modification timestamp, or None if not present or invalid
    fn get_modification_time(&self, resource: &Resource) -> Option<DateTime<Utc>> {
        resource.get_meta().map(|meta| meta.last_modified)
    }

    /// Create a metadata-specific error.
    ///
    /// Helper method for creating errors with metadata context.
    /// Default implementation assumes the Error type can be created from strings.
    /// Override if your error type requires different construction.
    fn metadata_error(&self, message: &str) -> Self::Error;
}

/// Default error creation for common error types that implement From<String>
impl<T> ScimMetadataManager for T
where
    T: ResourceProvider,
    T::Error: From<String>,
{
    fn metadata_error(&self, message: &str) -> Self::Error {
        Self::Error::from(message.to_string())
    }
}
