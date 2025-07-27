//! Schema definitions and validation for SCIM resources.
//!
//! This module provides the schema registry and validation engine for SCIM resources,
//! implementing the core User schema as defined in RFC 7643 with comprehensive
//! validation capabilities.

use crate::error::{ValidationError, ValidationResult};
use serde::{Deserialize, Serialize};
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
    schemas: HashMap<String, Schema>,
}

impl SchemaRegistry {
    /// Create a new schema registry with core User schema loaded from file.
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        Self::from_schema_dir(".")
    }

    /// Create a schema registry by loading schemas from a directory.
    pub fn from_schema_dir<P: AsRef<Path>>(
        schema_dir: P,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let user_schema_path = schema_dir.as_ref().join("User.json");
        let core_user_schema = Self::load_schema_from_file(&user_schema_path)?;

        let mut schemas = HashMap::new();
        schemas.insert(core_user_schema.id.clone(), core_user_schema.clone());

        Ok(Self {
            core_user_schema,
            schemas,
        })
    }

    /// Load a schema from a JSON file.
    fn load_schema_from_file<P: AsRef<Path>>(
        path: P,
    ) -> Result<Schema, Box<dyn std::error::Error>> {
        let content = fs::read_to_string(&path)?;
        let mut schema: Schema = serde_json::from_str(&content)?;

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

    /// Validate a user resource against the core User schema.
    pub fn validate_user(&self, user: &Value) -> ValidationResult<()> {
        self.validate_resource(&self.core_user_schema, user)
    }

    /// Validate a resource against a specific schema.
    pub fn validate_resource(&self, schema: &Schema, resource: &Value) -> ValidationResult<()> {
        let obj = resource
            .as_object()
            .ok_or_else(|| ValidationError::custom("Resource must be a JSON object"))?;

        // Validate each defined attribute
        for attr_def in &schema.attributes {
            self.validate_attribute(attr_def, obj, &schema.id)?;
        }

        // Check for unknown attributes (strict validation)
        for (field_name, _) in obj {
            if !schema
                .attributes
                .iter()
                .any(|attr| attr.name == *field_name)
            {
                // Allow schemas field for SCIM resources
                if field_name != "schemas" {
                    return Err(ValidationError::UnknownAttribute {
                        attribute: field_name.clone(),
                        schema_id: schema.id.clone(),
                    });
                }
            }
        }

        Ok(())
    }

    /// Validate a single attribute against its definition.
    fn validate_attribute(
        &self,
        attr_def: &AttributeDefinition,
        obj: &serde_json::Map<String, Value>,
        _schema_id: &str,
    ) -> ValidationResult<()> {
        let value = obj.get(&attr_def.name);

        // Check required attributes
        if attr_def.required && value.is_none() {
            return Err(ValidationError::missing_required(&attr_def.name));
        }

        // If value is None and not required, validation passes
        let Some(value) = value else {
            return Ok(());
        };

        // Check null values
        if value.is_null() {
            if attr_def.required {
                return Err(ValidationError::missing_required(&attr_def.name));
            }
            return Ok(());
        }

        // Validate multi-valued vs single-valued
        if attr_def.multi_valued {
            if !value.is_array() {
                return Err(ValidationError::ExpectedMultiValue {
                    attribute: attr_def.name.clone(),
                });
            }
            // Validate each item in the array
            if let Some(array) = value.as_array() {
                for item in array {
                    self.validate_attribute_value(attr_def, item)?;
                }
            }
        } else {
            if value.is_array() {
                return Err(ValidationError::ExpectedSingleValue {
                    attribute: attr_def.name.clone(),
                });
            }
            self.validate_attribute_value(attr_def, value)?;
        }

        Ok(())
    }

    /// Validate the value of an attribute against its type and constraints.
    fn validate_attribute_value(
        &self,
        attr_def: &AttributeDefinition,
        value: &Value,
    ) -> ValidationResult<()> {
        // Validate data type
        match attr_def.data_type {
            AttributeType::String => {
                if !value.is_string() {
                    return Err(ValidationError::invalid_type(
                        &attr_def.name,
                        "string",
                        Self::get_value_type(value),
                    ));
                }

                // Validate canonical values if specified
                if !attr_def.canonical_values.is_empty() {
                    if let Some(str_val) = value.as_str() {
                        if !attr_def.canonical_values.contains(&str_val.to_string()) {
                            return Err(ValidationError::InvalidCanonicalValue {
                                attribute: attr_def.name.clone(),
                                value: str_val.to_string(),
                                allowed: attr_def.canonical_values.clone(),
                            });
                        }
                    }
                }
            }
            AttributeType::Boolean => {
                if !value.is_boolean() {
                    return Err(ValidationError::invalid_type(
                        &attr_def.name,
                        "boolean",
                        Self::get_value_type(value),
                    ));
                }
            }
            AttributeType::Integer => {
                if !value.is_i64() {
                    return Err(ValidationError::invalid_type(
                        &attr_def.name,
                        "integer",
                        Self::get_value_type(value),
                    ));
                }
            }
            AttributeType::Decimal => {
                if !value.is_f64() && !value.is_i64() {
                    return Err(ValidationError::invalid_type(
                        &attr_def.name,
                        "decimal",
                        Self::get_value_type(value),
                    ));
                }
            }
            AttributeType::DateTime => {
                if !value.is_string() {
                    return Err(ValidationError::invalid_type(
                        &attr_def.name,
                        "dateTime",
                        Self::get_value_type(value),
                    ));
                }
                // TODO: Add RFC3339 datetime format validation
            }
            AttributeType::Binary => {
                if !value.is_string() {
                    return Err(ValidationError::invalid_type(
                        &attr_def.name,
                        "binary",
                        Self::get_value_type(value),
                    ));
                }
                // TODO: Add base64 format validation
            }
            AttributeType::Reference => {
                if !value.is_string() {
                    return Err(ValidationError::invalid_type(
                        &attr_def.name,
                        "reference",
                        Self::get_value_type(value),
                    ));
                }
                // TODO: Add URI format validation
            }
            AttributeType::Complex => {
                if !value.is_object() {
                    return Err(ValidationError::invalid_type(
                        &attr_def.name,
                        "complex",
                        Self::get_value_type(value),
                    ));
                }

                // Validate sub-attributes
                if let Some(obj) = value.as_object() {
                    for sub_attr in &attr_def.sub_attributes {
                        if sub_attr.required && !obj.contains_key(&sub_attr.name) {
                            return Err(ValidationError::MissingSubAttribute {
                                attribute: attr_def.name.clone(),
                                sub_attribute: sub_attr.name.clone(),
                            });
                        }

                        if let Some(sub_value) = obj.get(&sub_attr.name) {
                            self.validate_attribute_value(sub_attr, sub_value)?;
                        }
                    }
                }
            }
        }

        Ok(())
    }

    /// Get the type name of a JSON value for error messages.
    fn get_value_type(value: &Value) -> &'static str {
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

    /// Add a schema to the registry.
    pub fn add_schema(&mut self, schema: Schema) -> Result<(), Box<dyn std::error::Error>> {
        self.schemas.insert(schema.id.clone(), schema);
        Ok(())
    }

    /// Get a schema by ID.
    pub fn get_schema_by_id(&self, schema_id: &str) -> Option<&Schema> {
        self.schemas.get(schema_id)
    }
}

impl Default for SchemaRegistry {
    fn default() -> Self {
        Self::new().expect("Failed to load default schemas")
    }
}

/// SCIM schema definition.
///
/// Represents a complete schema as defined in RFC 7643, including
/// metadata and attribute definitions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Schema {
    /// Unique schema identifier (URI)
    pub id: String,
    /// Human-readable schema name
    pub name: String,
    /// Schema description
    pub description: String,
    /// List of attribute definitions
    pub attributes: Vec<AttributeDefinition>,
}

/// Definition of a SCIM attribute.
///
/// Defines all characteristics of an attribute including type,
/// constraints, and validation rules.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AttributeDefinition {
    /// Attribute name
    pub name: String,
    /// Data type of the attribute
    #[serde(rename = "type")]
    pub data_type: AttributeType,
    /// Whether this attribute can have multiple values
    #[serde(rename = "multiValued")]
    pub multi_valued: bool,
    /// Whether this attribute is required
    pub required: bool,
    /// Whether string comparison is case-sensitive
    #[serde(rename = "caseExact")]
    pub case_exact: bool,
    /// Mutability characteristics
    pub mutability: Mutability,
    /// Uniqueness constraints
    pub uniqueness: Uniqueness,
    /// Allowed values for string attributes
    #[serde(rename = "canonicalValues", default)]
    pub canonical_values: Vec<String>,
    /// Sub-attributes for complex types
    #[serde(rename = "subAttributes", default)]
    pub sub_attributes: Vec<AttributeDefinition>,
    /// How the attribute is returned in responses
    #[serde(default)]
    pub returned: Option<String>,
}

impl Default for AttributeDefinition {
    fn default() -> Self {
        Self {
            name: String::new(),
            data_type: AttributeType::String,
            multi_valued: false,
            required: false,
            case_exact: false,
            mutability: Mutability::ReadWrite,
            uniqueness: Uniqueness::None,
            canonical_values: Vec::new(),
            sub_attributes: Vec::new(),
            returned: None,
        }
    }
}

/// SCIM attribute data types.
///
/// Represents the valid data types for SCIM attributes as defined in RFC 7643.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum AttributeType {
    /// String value
    String,
    /// Boolean value
    Boolean,
    /// Decimal number
    Decimal,
    /// Integer number
    Integer,
    /// DateTime in RFC3339 format
    DateTime,
    /// Binary data (base64 encoded)
    Binary,
    /// URI reference
    Reference,
    /// Complex attribute with sub-attributes
    Complex,
}

impl Default for AttributeType {
    fn default() -> Self {
        Self::String
    }
}

/// Attribute mutability characteristics.
///
/// Defines whether and how an attribute can be modified.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum Mutability {
    /// Read-only attribute (managed by server)
    ReadOnly,
    /// Read-write attribute (can be modified by clients)
    ReadWrite,
    /// Immutable attribute (set once, never modified)
    Immutable,
    /// Write-only attribute (passwords, etc.)
    WriteOnly,
}

impl Default for Mutability {
    fn default() -> Self {
        Self::ReadWrite
    }
}

/// Attribute uniqueness constraints.
///
/// Defines the scope of uniqueness for attribute values.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum Uniqueness {
    /// No uniqueness constraint
    None,
    /// Unique within the server
    Server,
    /// Globally unique
    Global,
}

impl Default for Uniqueness {
    fn default() -> Self {
        Self::None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_schema_registry_creation() {
        let registry = SchemaRegistry::new().expect("Failed to create registry");
        assert_eq!(registry.get_schemas().len(), 1);
        assert!(
            registry
                .get_schema("urn:ietf:params:scim:schemas:core:2.0:User")
                .is_some()
        );
    }

    #[test]
    fn test_valid_user_validation() {
        let registry = SchemaRegistry::new().expect("Failed to create registry");
        let user = json!({
            "userName": "testuser",
            "displayName": "Test User",
            "active": true,
            "emails": [
                {
                    "value": "test@example.com",
                    "type": "work",
                    "primary": true
                }
            ]
        });

        assert!(registry.validate_user(&user).is_ok());
    }

    #[test]
    fn test_missing_required_attribute() {
        let registry = SchemaRegistry::new().expect("Failed to create registry");
        let user = json!({
            "displayName": "Test User"
            // Missing required userName
        });

        let result = registry.validate_user(&user);
        assert!(result.is_err());
        if let Err(ValidationError::MissingRequiredAttribute { attribute }) = result {
            assert_eq!(attribute, "userName");
        } else {
            panic!("Expected MissingRequiredAttribute error");
        }
    }

    #[test]
    fn test_invalid_type_validation() {
        let registry = SchemaRegistry::new().expect("Failed to create registry");
        let user = json!({
            "userName": "testuser",
            "active": "not_a_boolean"
        });

        let result = registry.validate_user(&user);
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_canonical_value() {
        let registry = SchemaRegistry::new().expect("Failed to create registry");
        let user = json!({
            "userName": "testuser",
            "emails": [
                {
                    "value": "test@example.com",
                    "type": "invalid_type"
                }
            ]
        });

        let result = registry.validate_user(&user);
        assert!(result.is_err());
    }

    #[test]
    fn test_complex_attribute_validation() {
        let registry = SchemaRegistry::new().expect("Failed to create registry");
        let user = json!({
            "userName": "testuser",
            "name": {
                "givenName": "John",
                "familyName": "Doe",
                "formatted": "John Doe"
            }
        });

        assert!(registry.validate_user(&user).is_ok());
    }
}
