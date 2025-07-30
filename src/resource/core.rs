//! Core types for SCIM resource operations.
//!
//! This module contains the fundamental data structures used throughout
//! the SCIM server for representing resources and operation contexts.

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Generic SCIM resource representation.
///
/// A resource is a structured data object with a type identifier and JSON data.
/// This design provides flexibility while maintaining schema validation through
/// the server layer.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Resource {
    /// The type of this resource (e.g., "User", "Group")
    pub resource_type: String,
    /// The resource data as validated JSON
    pub data: Value,
}

impl Resource {
    /// Create a new resource with the given type and data.
    ///
    /// # Arguments
    /// * `resource_type` - The SCIM resource type identifier
    /// * `data` - The resource data as a JSON value
    ///
    /// # Example
    /// ```rust
    /// use scim_server::Resource;
    /// use serde_json::json;
    ///
    /// let user_data = json!({
    ///     "userName": "jdoe",
    ///     "displayName": "John Doe"
    /// });
    /// let resource = Resource::new("User".to_string(), user_data);
    /// ```
    pub fn new(resource_type: String, data: Value) -> Self {
        Self {
            resource_type,
            data,
        }
    }

    /// Get the unique identifier of this resource.
    ///
    /// Returns the "id" field from the resource data if present.
    pub fn get_id(&self) -> Option<&str> {
        self.data.get("id")?.as_str()
    }

    /// Get the userName field for User resources.
    ///
    /// This is a convenience method for accessing the required userName field.
    pub fn get_username(&self) -> Option<&str> {
        self.data.get("userName")?.as_str()
    }

    /// Get a specific attribute value from the resource data.
    ///
    /// # Arguments
    /// * `attribute_name` - The name of the attribute to retrieve
    pub fn get_attribute(&self, attribute_name: &str) -> Option<&Value> {
        self.data.get(attribute_name)
    }

    /// Set a specific attribute value in the resource data.
    ///
    /// # Arguments
    /// * `attribute_name` - The name of the attribute to set
    /// * `value` - The value to set
    pub fn set_attribute(&mut self, attribute_name: String, value: Value) {
        if let Some(obj) = self.data.as_object_mut() {
            obj.insert(attribute_name, value);
        }
    }

    /// Get the schemas associated with this resource.
    pub fn get_schemas(&self) -> Vec<String> {
        self.data
            .get("schemas")
            .and_then(|s| s.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str())
                    .map(|s| s.to_string())
                    .collect()
            })
            .unwrap_or_else(|| {
                // Default schema based on resource type
                match self.resource_type.as_str() {
                    "User" => vec!["urn:ietf:params:scim:schemas:core:2.0:User".to_string()],
                    "Group" => vec!["urn:ietf:params:scim:schemas:core:2.0:Group".to_string()],
                    _ => vec![],
                }
            })
    }

    /// Add metadata to the resource.
    ///
    /// This method sets common SCIM metadata fields like resourceType,
    /// created, lastModified, and location.
    pub fn add_metadata(&mut self, base_url: &str, created: &str, last_modified: &str) {
        let meta = serde_json::json!({
            "resourceType": self.resource_type,
            "created": created,
            "lastModified": last_modified,
            "location": format!("{}/{}s/{}", base_url, self.resource_type, self.get_id().unwrap_or("")),
            "version": format!("W/\"{}-{}\"", self.get_id().unwrap_or(""), last_modified)
        });

        self.set_attribute("meta".to_string(), meta);
    }

    /// Check if this resource is active.
    ///
    /// Returns the value of the "active" field, defaulting to true if not present.
    pub fn is_active(&self) -> bool {
        self.data
            .get("active")
            .and_then(|v| v.as_bool())
            .unwrap_or(true)
    }

    /// Get all email addresses from the resource.
    pub fn get_emails(&self) -> Vec<super::types::EmailAddress> {
        self.data
            .get("emails")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|email| {
                        let value = email.get("value")?.as_str()?;
                        Some(super::types::EmailAddress {
                            value: value.to_string(),
                            email_type: email
                                .get("type")
                                .and_then(|t| t.as_str())
                                .map(|s| s.to_string()),
                            primary: email.get("primary").and_then(|p| p.as_bool()),
                            display: email
                                .get("display")
                                .and_then(|d| d.as_str())
                                .map(|s| s.to_string()),
                        })
                    })
                    .collect()
            })
            .unwrap_or_default()
    }
}

/// Request context for SCIM operations.
///
/// Provides request tracking for logging and auditing purposes.
#[derive(Debug, Clone)]
pub struct RequestContext {
    /// Unique identifier for this request
    pub request_id: String,
}

impl RequestContext {
    /// Create a new request context with a specific request ID.
    pub fn new(request_id: String) -> Self {
        Self { request_id }
    }

    /// Create a new request context with a generated request ID.
    pub fn with_generated_id() -> Self {
        Self {
            request_id: uuid::Uuid::new_v4().to_string(),
        }
    }
}

impl Default for RequestContext {
    fn default() -> Self {
        Self::with_generated_id()
    }
}

/// Supported SCIM operations for resource types
#[derive(Debug, Clone, PartialEq)]
pub enum ScimOperation {
    Create,
    Read,
    Update,
    Delete,
    List,
    Search,
}

/// Query parameters for listing resources (future extension).
///
/// This structure is prepared for future pagination and filtering support
/// but is not used in the MVP implementation.
#[derive(Debug, Clone, Default)]
pub struct ListQuery {
    /// Maximum number of results to return
    pub count: Option<usize>,
    /// Starting index for pagination
    pub start_index: Option<usize>,
    /// Filter expression
    pub filter: Option<String>,
    /// Attributes to include in results
    pub attributes: Vec<String>,
    /// Attributes to exclude from results
    pub excluded_attributes: Vec<String>,
}

impl ListQuery {
    /// Create a new empty query.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the maximum count.
    pub fn with_count(mut self, count: usize) -> Self {
        self.count = Some(count);
        self
    }

    /// Set the starting index.
    pub fn with_start_index(mut self, start_index: usize) -> Self {
        self.start_index = Some(start_index);
        self
    }

    /// Set a filter expression.
    pub fn with_filter(mut self, filter: String) -> Self {
        self.filter = Some(filter);
        self
    }
}
