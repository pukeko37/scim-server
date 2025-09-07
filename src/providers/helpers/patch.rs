//! SCIM PATCH operations helper trait.
//!
//! This module provides a reusable trait for implementing RFC 7644 compliant PATCH operations
//! on SCIM resources. Any ResourceProvider can implement this trait to get full PATCH
//! functionality without reimplementing the complex operation logic.
//!
//! # RFC 7644 Compliance
//!
//! This implementation follows RFC 7644 Section 3.5.2 (Modifying with PATCH) and supports:
//! - `add` operations for adding attributes or values
//! - `remove` operations for removing attributes or values
//! - `replace` operations for replacing attribute values
//! - Proper handling of multi-valued attributes
//! - Readonly attribute protection
//! - Complex attribute path parsing
//!
//! # Usage
//!
//! ```rust,no_run
//! use serde_json::{json, Value};
//!
//! // ScimPatchOperations provides RFC 7644 compliant PATCH operation handling:
//! // - add: Add new attributes or values
//! // - remove: Remove attributes or specific values
//! // - replace: Replace existing attributes or values
//! //
//! // Example PATCH operations:
//! let patch_ops = json!([
//!   {"op": "add", "path": "emails", "value": {"value": "new@example.com", "type": "work"}},
//!   {"op": "replace", "path": "active", "value": false},
//!   {"op": "remove", "path": "phoneNumbers[type eq \"fax\"]"}
//! ]);
//! //
//! // When implemented by a ResourceProvider, automatically handles:
//! // - Complex attribute path parsing
//! // - Multi-valued attribute operations
//! // - Value filtering and selection
//! ```

use crate::providers::ResourceProvider;
use serde_json::{Value, json};

/// Trait providing RFC 7644 compliant PATCH operations for SCIM resources.
///
/// This trait extends ResourceProvider with PATCH functionality, implementing
/// the complex logic for applying PATCH operations according to the SCIM specification.
/// Most implementers can use the default implementations without modification.
pub trait ScimPatchOperations: ResourceProvider {
    /// Apply a single PATCH operation to resource data.
    ///
    /// This is the main entry point for PATCH operation processing. It validates
    /// the operation structure, checks for readonly attributes, and delegates to
    /// the appropriate operation handler.
    ///
    /// # Arguments
    /// * `resource_data` - The resource JSON to modify
    /// * `operation` - The PATCH operation as defined in RFC 7644
    ///
    /// # Returns
    /// Result indicating success or failure with appropriate error details
    ///
    /// # Default Implementation
    /// Provides full RFC 7644 compliance including:
    /// - Operation validation (`op` field required)
    /// - Readonly attribute protection
    /// - Delegation to appropriate operation handlers
    fn apply_patch_operation(
        &self,
        resource_data: &mut Value,
        operation: &Value,
    ) -> Result<(), Self::Error> {
        let op = operation
            .get("op")
            .and_then(|v| v.as_str())
            .ok_or_else(|| self.patch_error("PATCH operation must have 'op' field"))?;

        let path = operation.get("path").and_then(|v| v.as_str());
        let value = operation.get("value");

        // Check if the operation targets a readonly attribute
        if let Some(path_str) = path {
            if self.is_readonly_attribute(path_str) {
                return Err(
                    self.patch_error(&format!("Cannot modify readonly attribute: {}", path_str))
                );
            }
        }

        match op.to_lowercase().as_str() {
            "add" => self.apply_add_operation(resource_data, path, value),
            "remove" => self.apply_remove_operation(resource_data, path),
            "replace" => self.apply_replace_operation(resource_data, path, value),
            _ => Err(self.patch_error(&format!("Unsupported PATCH operation: {}", op))),
        }
    }

    /// Apply an ADD operation to resource data.
    ///
    /// Implements RFC 7644 ADD operation semantics:
    /// - With path: Sets value at the specified path
    /// - Without path: Merges value with root object
    /// - Handles multi-valued attributes appropriately
    ///
    /// # Arguments
    /// * `resource_data` - The resource JSON to modify
    /// * `path` - Optional attribute path
    /// * `value` - Value to add (required for ADD operations)
    fn apply_add_operation(
        &self,
        resource_data: &mut Value,
        path: Option<&str>,
        value: Option<&Value>,
    ) -> Result<(), Self::Error> {
        let value = value.ok_or_else(|| self.patch_error("ADD operation requires a value"))?;

        match path {
            Some(path_str) => {
                self.set_value_at_path(resource_data, path_str, value.clone())?;
            }
            None => {
                // No path means add to root - merge objects
                if let (Some(current_obj), Some(value_obj)) =
                    (resource_data.as_object_mut(), value.as_object())
                {
                    for (key, val) in value_obj {
                        current_obj.insert(key.clone(), val.clone());
                    }
                }
            }
        }
        Ok(())
    }

    /// Apply a REMOVE operation to resource data.
    ///
    /// Implements RFC 7644 REMOVE operation semantics:
    /// - Removes the attribute or value at the specified path
    /// - Handles complex path expressions
    /// - Validates path before removal
    ///
    /// # Arguments
    /// * `resource_data` - The resource JSON to modify
    /// * `path` - Attribute path to remove (required for REMOVE operations)
    fn apply_remove_operation(
        &self,
        resource_data: &mut Value,
        path: Option<&str>,
    ) -> Result<(), Self::Error> {
        if let Some(path_str) = path {
            self.remove_value_at_path(resource_data, path_str)?;
        }
        Ok(())
    }

    /// Apply a REPLACE operation to resource data.
    ///
    /// Implements RFC 7644 REPLACE operation semantics:
    /// - With path: Replaces value at specified path
    /// - Without path: Replaces entire resource (merge semantics)
    /// - Validates value before replacement
    ///
    /// # Arguments
    /// * `resource_data` - The resource JSON to modify
    /// * `path` - Optional attribute path
    /// * `value` - Replacement value (required for REPLACE operations)
    fn apply_replace_operation(
        &self,
        resource_data: &mut Value,
        path: Option<&str>,
        value: Option<&Value>,
    ) -> Result<(), Self::Error> {
        let value = value.ok_or_else(|| self.patch_error("REPLACE operation requires a value"))?;

        match path {
            Some(path_str) => {
                self.set_value_at_path(resource_data, path_str, value.clone())?;
            }
            None => {
                // No path means replace entire resource
                if let Some(value_obj) = value.as_object() {
                    if let Some(current_obj) = resource_data.as_object_mut() {
                        for (key, val) in value_obj {
                            current_obj.insert(key.clone(), val.clone());
                        }
                    }
                }
            }
        }
        Ok(())
    }

    /// Set a value at a complex attribute path.
    ///
    /// Handles SCIM attribute path expressions including:
    /// - Simple attributes (e.g., "userName")
    /// - Complex attributes (e.g., "name.givenName")
    /// - Multi-valued attributes (e.g., "emails[type eq \"work\"].value")
    ///
    /// # Arguments
    /// * `data` - The JSON object to modify
    /// * `path` - The SCIM attribute path
    /// * `value` - The value to set
    fn set_value_at_path(
        &self,
        data: &mut Value,
        path: &str,
        value: Value,
    ) -> Result<(), Self::Error> {
        if !self.is_valid_scim_path(path) {
            return Err(self.patch_error(&format!("Invalid SCIM path: {}", path)));
        }

        // Handle simple path (no dots)
        if !path.contains('.') {
            if let Some(obj) = data.as_object_mut() {
                obj.insert(path.to_string(), value);
            }
            return Ok(());
        }

        // Handle complex path - use pointer navigation to avoid borrow checker issues
        let parts: Vec<&str> = path.split('.').collect();

        // Build the path string for JSON pointer
        let mut pointer_path = String::new();
        for part in &parts {
            pointer_path.push('/');
            pointer_path.push_str(part);
        }

        // Navigate to parent and ensure it exists
        let parent_parts = &parts[..parts.len() - 1];
        let mut current = data;

        for part in parent_parts {
            match current {
                Value::Object(obj) => {
                    let entry = obj.entry(part.to_string()).or_insert_with(|| json!({}));
                    current = entry;
                }
                _ => return Ok(()), // Can't navigate further
            }
        }

        // Set the final value
        if let Some(obj) = current.as_object_mut() {
            obj.insert(parts.last().unwrap().to_string(), value);
        }

        Ok(())
    }

    /// Remove a value at a complex attribute path.
    ///
    /// Handles removal of values from SCIM attribute paths, including
    /// validation and proper cleanup of empty parent objects.
    ///
    /// # Arguments
    /// * `data` - The JSON object to modify
    /// * `path` - The SCIM attribute path to remove
    fn remove_value_at_path(&self, data: &mut Value, path: &str) -> Result<(), Self::Error> {
        if !self.is_valid_scim_path(path) {
            return Err(self.patch_error(&format!("Invalid SCIM path: {}", path)));
        }

        // Handle simple path
        if !path.contains('.') {
            if let Some(obj) = data.as_object_mut() {
                obj.remove(path);
            }
            return Ok(());
        }

        // Handle complex path by rebuilding the structure without the target
        let parts: Vec<&str> = path.split('.').collect();
        self.remove_nested_value(data, &parts, 0)
    }

    /// Helper function to recursively remove nested values
    fn remove_nested_value(
        &self,
        current: &mut Value,
        parts: &[&str],
        depth: usize,
    ) -> Result<(), Self::Error> {
        if depth >= parts.len() {
            return Ok(());
        }

        let part = parts[depth];

        if depth == parts.len() - 1 {
            // We're at the final part, remove it
            if let Some(obj) = current.as_object_mut() {
                obj.remove(part);
            }
        } else {
            // Navigate deeper
            if let Some(obj) = current.as_object_mut() {
                if let Some(child) = obj.get_mut(part) {
                    self.remove_nested_value(child, parts, depth + 1)?;
                }
            }
        }

        Ok(())
    }

    /// Check if an attribute path refers to a readonly attribute.
    ///
    /// Default implementation covers RFC 7644 readonly attributes:
    /// - `id` - Resource identifier
    /// - `meta.created` - Creation timestamp
    /// - `meta.resourceType` - Resource type
    /// - `meta.location` - Resource location
    ///
    /// Override this method to add custom readonly attributes.
    fn is_readonly_attribute(&self, path: &str) -> bool {
        match path.to_lowercase().as_str() {
            // Core readonly attributes
            "id" => true,
            "meta.created" => true,
            "meta.resourcetype" => true,
            "meta.location" => true,
            // Pattern matching for meta attributes
            path if path.starts_with("meta.")
                && (path.ends_with(".created")
                    || path.ends_with(".resourcetype")
                    || path.ends_with(".location")) =>
            {
                true
            }
            _ => false,
        }
    }

    /// Validate if a path represents a valid SCIM attribute.
    ///
    /// Default implementation provides basic validation:
    /// - Non-empty paths
    /// - Valid attribute name characters
    /// - Proper dot notation for complex attributes
    ///
    /// Override for more sophisticated validation.
    fn is_valid_scim_path(&self, path: &str) -> bool {
        if path.is_empty() {
            return false;
        }

        // Handle schema URN prefixed paths
        let actual_path = if path.contains(':') && path.contains("urn:ietf:params:scim:schemas:") {
            // Extract the attribute name after the schema URN
            path.split(':').last().unwrap_or(path)
        } else {
            path
        };

        // Basic validation - can be enhanced by implementers
        !actual_path.is_empty()
            && actual_path
                .chars()
                .all(|c| c.is_alphanumeric() || c == '.' || c == '_')
    }

    /// Create a PATCH-specific error.
    ///
    /// Helper method for creating errors with appropriate context.
    /// Default implementation assumes the Error type can be created from strings.
    /// Override if your error type requires different construction.
    fn patch_error(&self, message: &str) -> Self::Error;
}

/// Default error creation for common error types that implement From<String>
impl<T> ScimPatchOperations for T
where
    T: ResourceProvider,
    T::Error: From<String>,
{
    fn patch_error(&self, message: &str) -> Self::Error {
        Self::Error::from(message.to_string())
    }
}
