//! MCP integration handlers
//!
//! This module contains all the handler implementations for MCP tool execution.
//! Handlers are organized by functional area to maintain clear separation of
//! concerns and enable focused testing and maintenance.

use crate::resource::version::{HttpVersion, RawVersion};
use serde_json::Value;

pub mod group_crud;
pub mod group_queries;
pub mod system_info;
pub mod user_crud;
pub mod user_queries;

// Re-export handler functions for convenience
pub use group_crud::*;
pub use group_queries::*;
pub use system_info::*;
pub use user_crud::*;
pub use user_queries::*;

/// Shared utility functions for MCP handlers

/// Convert HTTP ETag format to raw format for MCP responses
///
/// MCP clients work better with raw version strings without HTTP ETag escaping.
/// This helper extracts the raw version from either HTTP ETag or raw format.
pub fn etag_to_raw_version(etag_value: &Value) -> Option<String> {
    let etag_str = etag_value.as_str()?;

    // Try parsing as HTTP ETag first
    if let Ok(version) = etag_str.parse::<HttpVersion>() {
        return Some(version.as_str().to_string());
    }

    // Try parsing as raw version
    if let Ok(version) = etag_str.parse::<RawVersion>() {
        return Some(version.as_str().to_string());
    }

    None
}

/// Convert all version fields in resource data from ETag format to raw format
///
/// This processes SCIM resource data (users/groups) and converts any version
/// fields found in meta.version from HTTP ETag format to raw format for
/// consistent MCP client experience.
pub fn convert_resource_versions(mut resource: Value) -> Value {
    if let Some(meta) = resource.get_mut("meta") {
        if let Some(version) = meta.get("version") {
            if let Some(raw_version) = etag_to_raw_version(version) {
                meta["version"] = Value::String(raw_version);
            }
        }
    }
    resource
}

/// Convert version fields in a list of resources from ETag format to raw format
///
/// This processes arrays of SCIM resources (from list/search operations) and
/// converts all version fields to raw format for consistent MCP responses.
pub fn convert_resources_versions(resources: &mut Value) {
    if let Some(resources_array) = resources.as_array_mut() {
        for resource in resources_array {
            *resource = convert_resource_versions(resource.clone());
        }
    }
}
