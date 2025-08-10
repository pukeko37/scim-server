//! Extension value objects for custom SCIM schema attributes.
//!
//! This module provides support for SCIM extension attributes that are defined
//! by custom schemas. Extension attributes allow SCIM implementations to extend
//! the core schema with additional attributes while maintaining type safety
//! and validation.
//!
//! ## Design Principles
//!
//! - **Schema-Driven**: Extensions are defined by schema URIs and definitions
//! - **Type-Safe**: Extension values are validated against their schema definitions
//! - **Flexible**: Support for all SCIM attribute types in extensions
//! - **Namespace-Aware**: Extensions are grouped by schema URI namespaces

use super::value_object_trait::{ExtensionAttribute, ValueObject};
use crate::error::{ValidationError, ValidationResult};
use crate::resource::value_objects::SchemaUri;
use crate::schema::types::{AttributeDefinition, AttributeType};
use base64::Engine;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::any::Any;
use std::collections::HashMap;

/// A single extension attribute with its value and metadata.
///
/// Extension attributes are defined by custom schemas and can contain
/// any valid SCIM attribute type. They maintain a reference to their
/// schema definition for validation purposes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtensionAttributeValue {
    /// The schema URI that defines this extension
    schema_uri: SchemaUri,
    /// The name of the attribute within the extension schema
    attribute_name: String,
    /// The JSON value of the attribute
    value: Value,
    /// The schema definition for validation
    #[serde(skip)]
    definition: Option<AttributeDefinition>,
}

impl ExtensionAttributeValue {
    /// Create a new extension attribute value.
    pub fn new(
        schema_uri: SchemaUri,
        attribute_name: String,
        value: Value,
        definition: Option<AttributeDefinition>,
    ) -> ValidationResult<Self> {
        let ext_attr = Self {
            schema_uri,
            attribute_name,
            value,
            definition,
        };

        // Validate the value against the definition if available
        if let Some(ref def) = ext_attr.definition {
            ext_attr.validate_value_against_definition(def)?;
        }

        Ok(ext_attr)
    }

    /// Create an extension attribute without immediate validation.
    ///
    /// This is useful when the schema definition is not available at construction time
    /// but will be provided later for validation.
    pub fn new_unchecked(schema_uri: SchemaUri, attribute_name: String, value: Value) -> Self {
        Self {
            schema_uri,
            attribute_name,
            value,
            definition: None,
        }
    }

    /// Set the schema definition for this extension attribute.
    pub fn with_definition(mut self, definition: AttributeDefinition) -> ValidationResult<Self> {
        self.validate_value_against_definition(&definition)?;
        self.definition = Some(definition);
        Ok(self)
    }

    /// Get the schema URI for this extension.
    pub fn schema_uri(&self) -> &SchemaUri {
        &self.schema_uri
    }

    /// Get the attribute name.
    pub fn attribute_name(&self) -> &str {
        &self.attribute_name
    }

    /// Get the JSON value.
    pub fn value(&self) -> &Value {
        &self.value
    }

    /// Get the schema definition if available.
    pub fn definition(&self) -> Option<&AttributeDefinition> {
        self.definition.as_ref()
    }

    /// Extract the extension namespace from the schema URI.
    pub fn extension_namespace(&self) -> String {
        // Extract namespace from URN format like "urn:ietf:params:scim:schemas:extension:enterprise:2.0:User"
        if let Some(parts) = self.schema_uri.as_str().split(':').last() {
            if parts.contains(':') {
                // More complex namespace extraction if needed
                parts.to_string()
            } else {
                parts.to_string()
            }
        } else {
            // Fallback to using the entire URI as namespace
            self.schema_uri.as_str().to_string()
        }
    }

    /// Validate the value against the schema definition.
    fn validate_value_against_definition(
        &self,
        definition: &AttributeDefinition,
    ) -> ValidationResult<()> {
        // Check attribute name matches
        if definition.name != self.attribute_name {
            return Err(ValidationError::InvalidAttributeName {
                actual: self.attribute_name.clone(),
                expected: definition.name.clone(),
            });
        }

        // Validate value type against definition
        self.validate_value_type(definition)?;

        // Validate required constraints
        if definition.required && matches!(self.value, Value::Null) {
            return Err(ValidationError::RequiredAttributeMissing(
                self.attribute_name.clone(),
            ));
        }

        // Validate canonical values if specified
        if !definition.canonical_values.is_empty() {
            self.validate_canonical_values(definition)?;
        }

        Ok(())
    }

    /// Validate the value type against the attribute definition.
    fn validate_value_type(&self, definition: &AttributeDefinition) -> ValidationResult<()> {
        let matches_type = match (&definition.data_type, &self.value) {
            (AttributeType::String, Value::String(_)) => true,
            (AttributeType::Boolean, Value::Bool(_)) => true,
            (AttributeType::Integer, Value::Number(n)) if n.is_i64() => true,
            (AttributeType::Decimal, Value::Number(_)) => true,
            (AttributeType::DateTime, Value::String(s)) => {
                // Basic datetime format validation
                chrono::DateTime::parse_from_rfc3339(s).is_ok()
            }
            (AttributeType::Binary, Value::String(s)) => {
                // Basic base64 validation
                base64::engine::general_purpose::STANDARD.decode(s).is_ok()
            }
            (AttributeType::Reference, Value::String(_)) => true, // URI validation could be added
            (AttributeType::Complex, Value::Object(_)) => true,
            (_, Value::Null) => !definition.required,
            _ => false,
        };

        if !matches_type {
            return Err(ValidationError::InvalidAttributeType {
                attribute: self.attribute_name.clone(),
                expected: format!("{:?}", definition.data_type),
                actual: self.get_value_type_name().to_string(),
            });
        }

        Ok(())
    }

    /// Validate canonical values constraint.
    fn validate_canonical_values(&self, definition: &AttributeDefinition) -> ValidationResult<()> {
        if let Value::String(value_str) = &self.value {
            if !definition.canonical_values.contains(value_str) {
                return Err(ValidationError::InvalidCanonicalValue {
                    attribute: self.attribute_name.clone(),
                    value: value_str.clone(),
                    allowed: definition.canonical_values.clone(),
                });
            }
        }
        Ok(())
    }

    /// Get the type name of the JSON value for error reporting.
    fn get_value_type_name(&self) -> &'static str {
        match &self.value {
            Value::Null => "null",
            Value::Bool(_) => "boolean",
            Value::Number(n) if n.is_i64() => "integer",
            Value::Number(_) => "decimal",
            Value::String(_) => "string",
            Value::Array(_) => "array",
            Value::Object(_) => "object",
        }
    }
}

impl ValueObject for ExtensionAttributeValue {
    fn attribute_type(&self) -> AttributeType {
        if let Some(ref def) = self.definition {
            def.data_type.clone()
        } else {
            // Infer type from JSON value
            match &self.value {
                Value::String(_) => AttributeType::String,
                Value::Bool(_) => AttributeType::Boolean,
                Value::Number(n) if n.is_i64() => AttributeType::Integer,
                Value::Number(_) => AttributeType::Decimal,
                Value::Object(_) => AttributeType::Complex,
                _ => AttributeType::String, // Default fallback
            }
        }
    }

    fn attribute_name(&self) -> &str {
        &self.attribute_name
    }

    fn to_json(&self) -> ValidationResult<Value> {
        Ok(self.value.clone())
    }

    fn validate_against_schema(&self, definition: &AttributeDefinition) -> ValidationResult<()> {
        self.validate_value_against_definition(definition)
    }

    fn as_json_value(&self) -> Value {
        self.value.clone()
    }

    fn supports_definition(&self, definition: &AttributeDefinition) -> bool {
        definition.name == self.attribute_name
    }

    fn clone_boxed(&self) -> Box<dyn ValueObject> {
        Box::new(self.clone())
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl ExtensionAttribute for ExtensionAttributeValue {
    fn schema_uri(&self) -> &str {
        self.schema_uri.as_str()
    }

    fn extension_namespace(&self) -> &str {
        // For now, we'll use the schema URI as the namespace
        // In a more sophisticated implementation, this could be cached
        // or computed more efficiently
        self.schema_uri.as_str()
    }

    fn validate_extension_rules(&self) -> ValidationResult<()> {
        // Extension-specific validation rules can be added here
        // For now, we delegate to the standard schema validation
        if let Some(ref def) = self.definition {
            self.validate_against_schema(def)
        } else {
            Ok(())
        }
    }
}

/// Collection of extension attributes grouped by schema URI.
///
/// This provides an organized way to manage multiple extension attributes
/// and ensures that attributes are properly namespaced by their schema URIs.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtensionCollection {
    /// Map of schema URI to extension attributes
    extensions: HashMap<String, Vec<ExtensionAttributeValue>>,
}

impl ExtensionCollection {
    /// Create a new empty extension collection.
    pub fn new() -> Self {
        Self {
            extensions: HashMap::new(),
        }
    }

    /// Add an extension attribute to the collection.
    pub fn add_attribute(&mut self, attribute: ExtensionAttributeValue) {
        let schema_uri = attribute.schema_uri().as_str().to_string();
        self.extensions
            .entry(schema_uri)
            .or_insert_with(Vec::new)
            .push(attribute);
    }

    /// Get all extension attributes for a specific schema URI.
    pub fn get_by_schema(&self, schema_uri: &str) -> Option<&Vec<ExtensionAttributeValue>> {
        self.extensions.get(schema_uri)
    }

    /// Get a specific extension attribute by schema URI and attribute name.
    pub fn get_attribute(
        &self,
        schema_uri: &str,
        attribute_name: &str,
    ) -> Option<&ExtensionAttributeValue> {
        self.extensions
            .get(schema_uri)?
            .iter()
            .find(|attr| attr.attribute_name() == attribute_name)
    }

    /// Get all schema URIs that have extensions in this collection.
    pub fn schema_uris(&self) -> Vec<&str> {
        self.extensions.keys().map(|s| s.as_str()).collect()
    }

    /// Get all extension attributes across all schemas.
    pub fn all_attributes(&self) -> Vec<&ExtensionAttributeValue> {
        self.extensions
            .values()
            .flat_map(|attrs| attrs.iter())
            .collect()
    }

    /// Remove all extensions for a specific schema URI.
    pub fn remove_schema(&mut self, schema_uri: &str) -> Option<Vec<ExtensionAttributeValue>> {
        self.extensions.remove(schema_uri)
    }

    /// Check if the collection is empty.
    pub fn is_empty(&self) -> bool {
        self.extensions.is_empty()
    }

    /// Get the total number of extension attributes.
    pub fn len(&self) -> usize {
        self.extensions.values().map(|v| v.len()).sum()
    }

    /// Validate all extension attributes in the collection.
    pub fn validate_all(&self) -> ValidationResult<()> {
        for attributes in self.extensions.values() {
            for attribute in attributes {
                attribute.validate_extension_rules()?;
            }
        }
        Ok(())
    }

    /// Convert the extension collection to a JSON object.
    ///
    /// The resulting JSON object has schema URIs as keys and
    /// objects containing the extension attributes as values.
    pub fn to_json(&self) -> ValidationResult<Value> {
        let mut result = serde_json::Map::new();

        for (schema_uri, attributes) in &self.extensions {
            let mut schema_obj = serde_json::Map::new();

            for attribute in attributes {
                schema_obj.insert(attribute.attribute_name().to_string(), attribute.to_json()?);
            }

            result.insert(schema_uri.clone(), Value::Object(schema_obj));
        }

        Ok(Value::Object(result))
    }

    /// Create an extension collection from a JSON object.
    ///
    /// The JSON object should have schema URIs as keys and
    /// objects containing extension attributes as values.
    pub fn from_json(value: &Value) -> ValidationResult<Self> {
        let mut collection = Self::new();

        if let Value::Object(schema_map) = value {
            for (schema_uri_str, schema_value) in schema_map {
                let schema_uri = SchemaUri::new(schema_uri_str.clone())?;

                if let Value::Object(attr_map) = schema_value {
                    for (attr_name, attr_value) in attr_map {
                        let ext_attr = ExtensionAttributeValue::new_unchecked(
                            schema_uri.clone(),
                            attr_name.clone(),
                            attr_value.clone(),
                        );
                        collection.add_attribute(ext_attr);
                    }
                }
            }
        }

        Ok(collection)
    }
}

impl Default for ExtensionCollection {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::types::{Mutability, Uniqueness};

    fn create_test_schema_uri() -> SchemaUri {
        SchemaUri::new("urn:ietf:params:scim:schemas:extension:enterprise:2.0:User".to_string())
            .unwrap()
    }

    fn create_test_definition() -> AttributeDefinition {
        AttributeDefinition {
            name: "employeeNumber".to_string(),
            data_type: AttributeType::String,
            multi_valued: false,
            required: false,
            case_exact: false,
            mutability: Mutability::ReadWrite,
            uniqueness: Uniqueness::None,
            canonical_values: vec![],
            sub_attributes: vec![],
            returned: None,
        }
    }

    #[test]
    fn test_extension_attribute_creation() {
        let schema_uri = create_test_schema_uri();
        let definition = create_test_definition();
        let value = Value::String("12345".to_string());

        let ext_attr = ExtensionAttributeValue::new(
            schema_uri.clone(),
            "employeeNumber".to_string(),
            value.clone(),
            Some(definition),
        )
        .unwrap();

        assert_eq!(ext_attr.schema_uri(), &schema_uri);
        assert_eq!(ext_attr.attribute_name(), "employeeNumber");
        assert_eq!(ext_attr.value(), &value);
    }

    #[test]
    fn test_extension_attribute_validation() {
        let schema_uri = create_test_schema_uri();
        let definition = create_test_definition();

        // Valid value
        let valid_value = Value::String("12345".to_string());
        let result = ExtensionAttributeValue::new(
            schema_uri.clone(),
            "employeeNumber".to_string(),
            valid_value,
            Some(definition.clone()),
        );
        assert!(result.is_ok());

        // Invalid type
        let invalid_value = Value::Number(serde_json::Number::from(12345));
        let result = ExtensionAttributeValue::new(
            schema_uri.clone(),
            "employeeNumber".to_string(),
            invalid_value,
            Some(definition),
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_extension_collection() {
        let mut collection = ExtensionCollection::new();

        let schema_uri = create_test_schema_uri();
        let ext_attr = ExtensionAttributeValue::new_unchecked(
            schema_uri.clone(),
            "employeeNumber".to_string(),
            Value::String("12345".to_string()),
        );

        collection.add_attribute(ext_attr);

        assert_eq!(collection.len(), 1);
        assert!(!collection.is_empty());
        assert!(collection.get_by_schema(schema_uri.as_str()).is_some());
        assert!(
            collection
                .get_attribute(schema_uri.as_str(), "employeeNumber")
                .is_some()
        );
    }

    #[test]
    fn test_extension_collection_json_round_trip() {
        let mut collection = ExtensionCollection::new();

        let schema_uri = create_test_schema_uri();
        let ext_attr = ExtensionAttributeValue::new_unchecked(
            schema_uri.clone(),
            "employeeNumber".to_string(),
            Value::String("12345".to_string()),
        );

        collection.add_attribute(ext_attr);

        // Convert to JSON and back
        let json = collection.to_json().unwrap();
        let restored_collection = ExtensionCollection::from_json(&json).unwrap();

        assert_eq!(collection.len(), restored_collection.len());
        assert!(
            restored_collection
                .get_attribute(schema_uri.as_str(), "employeeNumber")
                .is_some()
        );
    }

    #[test]
    fn test_value_object_trait_implementation() {
        let schema_uri = create_test_schema_uri();
        let ext_attr = ExtensionAttributeValue::new_unchecked(
            schema_uri,
            "employeeNumber".to_string(),
            Value::String("12345".to_string()),
        );

        assert_eq!(ext_attr.attribute_type(), AttributeType::String);
        assert_eq!(ext_attr.attribute_name(), "employeeNumber");
        assert_eq!(ext_attr.as_json_value(), Value::String("12345".to_string()));
    }

    #[test]
    fn test_extension_attribute_trait_implementation() {
        let schema_uri = create_test_schema_uri();
        let ext_attr = ExtensionAttributeValue::new_unchecked(
            schema_uri.clone(),
            "employeeNumber".to_string(),
            Value::String("12345".to_string()),
        );

        assert_eq!(ext_attr.schema_uri(), &schema_uri);
        assert!(ext_attr.validate_extension_rules().is_ok());
    }
}
