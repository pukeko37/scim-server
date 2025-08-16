//! Schema registry for loading, managing, and accessing SCIM schemas.
//!
//! This module provides the SchemaRegistry which handles schema loading from files,
//! schema management, and provides access to registered schemas for validation.

use super::{embedded, types::{AttributeDefinition, AttributeType, Schema}};

use chrono::{DateTime, FixedOffset};
use serde_json::Value;
use std::collections::HashMap;
use std::fs;
use std::path::Path;

/// Registry for SCIM schemas with validation capabilities.
///
/// The schema registry manages all available schemas and provides validation
/// services for resources. For the MVP, it contains only the hardcoded core User schema.
#[derive(Debug, Clone)]
pub struct SchemaRegistry {
    core_user_schema: Schema,
    core_group_schema: Schema,
    schemas: HashMap<String, Schema>,
}

impl SchemaRegistry {
    /// Create a new schema registry with embedded core schemas.
    ///
    /// This method uses the schemas embedded in the library and doesn't require
    /// external schema files. For loading schemas from files, use `from_schema_dir()`.
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        Self::with_embedded_schemas()
    }

    /// Create a new schema registry with embedded core schemas.
    ///
    /// This method uses the schemas embedded in the library and doesn't require
    /// external schema files. This is the recommended method for schema discovery
    /// functionality as it works without any file dependencies.
    pub fn with_embedded_schemas() -> Result<Self, Box<dyn std::error::Error>> {
        let core_user_schema = Self::load_schema_from_str(embedded::core_user_schema())?;
        let core_group_schema = Self::load_schema_from_str(embedded::core_group_schema())?;

        let mut schemas = HashMap::new();
        schemas.insert(core_user_schema.id.clone(), core_user_schema.clone());
        schemas.insert(core_group_schema.id.clone(), core_group_schema.clone());

        Ok(Self {
            core_user_schema,
            core_group_schema,
            schemas,
        })
    }

    /// Create a schema registry by loading schemas from a directory.
    pub fn from_schema_dir<P: AsRef<Path>>(
        schema_dir: P,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let user_schema_path = schema_dir.as_ref().join("User.json");
        let core_user_schema = Self::load_schema_from_file(&user_schema_path)?;

        let group_schema_path = schema_dir.as_ref().join("Group.json");
        let core_group_schema = Self::load_schema_from_file(&group_schema_path)?;

        let mut schemas = HashMap::new();
        schemas.insert(core_user_schema.id.clone(), core_user_schema.clone());
        schemas.insert(core_group_schema.id.clone(), core_group_schema.clone());

        Ok(Self {
            core_user_schema,
            core_group_schema,
            schemas,
        })
    }

    /// Load a schema from a JSON file.
    fn load_schema_from_file<P: AsRef<Path>>(
        path: P,
    ) -> Result<Schema, Box<dyn std::error::Error>> {
        let content = fs::read_to_string(&path)?;
        Self::load_schema_from_str(&content)
    }

    /// Load a schema from a JSON string.
    fn load_schema_from_str(content: &str) -> Result<Schema, Box<dyn std::error::Error>> {
        let mut schema: Schema = serde_json::from_str(content)?;

        // Convert JSON schema format to internal format
        Self::convert_json_schema(&mut schema);

        Ok(schema)
    }

    /// Convert JSON schema format to internal AttributeDefinition format.
    fn convert_json_schema(schema: &mut Schema) {
        for attr in &mut schema.attributes {
            Self::convert_attribute_definition(attr);
        }
    }

    /// Convert a single attribute definition from JSON format.
    fn convert_attribute_definition(attr: &mut AttributeDefinition) {
        // Convert data type from string to enum
        // This is handled by serde deserialization

        // Process sub-attributes recursively
        for sub_attr in &mut attr.sub_attributes {
            Self::convert_attribute_definition(sub_attr);
        }
    }

    /// Get all available schemas.
    pub fn get_schemas(&self) -> Vec<&Schema> {
        self.schemas.values().collect()
    }

    /// Get a specific schema by ID.
    pub fn get_schema(&self, id: &str) -> Option<&Schema> {
        self.schemas.get(id)
    }

    /// Get the core User schema.
    pub fn get_user_schema(&self) -> &Schema {
        &self.core_user_schema
    }

    pub fn get_group_schema(&self) -> &Schema {
        &self.core_group_schema
    }

    /// Add a schema to the registry.
    pub fn add_schema(&mut self, schema: Schema) -> Result<(), Box<dyn std::error::Error>> {
        self.schemas.insert(schema.id.clone(), schema);
        Ok(())
    }

    /// Get a schema by ID.
    pub fn get_schema_by_id(&self, schema_id: &str) -> Option<&Schema> {
        self.schemas.get(schema_id)
    }

    /// Validate datetime format using chrono for full RFC3339 compliance
    ///
    /// This leverages chrono's well-tested RFC3339 parser, which provides:
    /// - Full semantic validation (no invalid dates like Feb 30th)
    /// - Proper timezone handling (+/-HH:MM, Z)
    /// - Millisecond precision support
    /// - Leap second awareness
    ///
    /// By using chrono, we avoid reimplementing complex datetime validation
    /// and get specification-compliant parsing for free.
    pub(super) fn is_valid_datetime_format(&self, value: &str) -> bool {
        if value.is_empty() {
            return false;
        }

        // Delegate to chrono's RFC3339 parser - it's well-tested and handles all edge cases
        DateTime::<FixedOffset>::parse_from_rfc3339(value).is_ok()
    }

    /// Validate base64 encoding (basic character set validation)
    ///
    /// This performs basic character set validation for base64 data.
    /// For production use, consider using a dedicated base64 crate like `base64`
    /// for proper padding validation and decode verification.
    pub(super) fn is_valid_base64(&self, value: &str) -> bool {
        if value.is_empty() {
            return false;
        }

        // Basic character set validation - sufficient for type checking
        // Note: Doesn't validate padding rules or decode correctness
        let base64_chars = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/=";
        value.chars().all(|c| base64_chars.contains(c))
    }

    /// Validate URI format (basic scheme validation)
    ///
    /// This performs basic URI scheme validation sufficient for SCIM reference checking.
    /// For comprehensive URI validation, consider using the `url` crate.
    pub(super) fn is_valid_uri_format(&self, value: &str) -> bool {
        if value.is_empty() {
            return false;
        }

        // Basic scheme validation - sufficient for SCIM reference URIs
        // Accepts HTTP(S) URLs and URN schemes commonly used in SCIM
        value.contains("://") || value.starts_with("urn:")
    }

    /// Get the type name of a JSON value for error messages.
    pub(super) fn get_value_type(value: &Value) -> &'static str {
        match value {
            Value::Null => "null",
            Value::Bool(_) => "boolean",
            Value::Number(n) if n.is_i64() => "integer",
            Value::Number(_) => "decimal",
            Value::String(_) => "string",
            Value::Array(_) => "array",
            Value::Object(_) => "object",
        }
    }

    /// Get attribute definition for a complex attribute
    pub(super) fn get_complex_attribute_definition(
        &self,
        attr_name: &str,
    ) -> Option<&AttributeDefinition> {
        // Look in core user schema for the attribute
        self.core_user_schema
            .attributes
            .iter()
            .find(|attr| attr.name == attr_name && matches!(attr.data_type, AttributeType::Complex))
    }
}

impl Default for SchemaRegistry {
    fn default() -> Self {
        Self::new().expect("Failed to load default schemas")
    }
}
