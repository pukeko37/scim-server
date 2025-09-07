//! Dynamic value object factory for schema-driven construction.
//!
//! This module provides a factory system that can dynamically create value objects
//! based on SCIM schema definitions and JSON values. It supports both core SCIM
//! attributes and extension attributes, enabling flexible resource construction
//! while maintaining type safety and validation.
//!
//! ## Design Principles
//!
//! - **Schema-Driven**: Factory decisions based on attribute definitions
//! - **Extensible**: Support for registering new value object types
//! - **Type-Safe**: Compile-time guarantees where possible
//! - **Performance**: Efficient lookup and construction mechanisms

#![allow(dead_code)]

use super::extension::ExtensionAttributeValue;
use super::value_object_trait::{ValueObject, ValueObjectConstructor, ValueObjectRegistry};
use super::{EmailAddress, ExternalId, Name, ResourceId, SchemaUri, UserName};
use crate::error::{ValidationError, ValidationResult};
use crate::schema::types::{AttributeDefinition, AttributeType};
use serde_json::Value;
use std::collections::HashMap;

/// Factory for creating value objects from schema definitions and JSON values.
///
/// The factory maintains a registry of constructors and can dynamically
/// create appropriate value objects based on attribute definitions.
pub struct ValueObjectFactory {
    registry: ValueObjectRegistry,
    type_mappings: HashMap<String, AttributeType>,
}

impl ValueObjectFactory {
    /// Create a new factory with default constructors registered.
    pub fn new() -> Self {
        let mut factory = Self {
            registry: ValueObjectRegistry::new(),
            type_mappings: HashMap::new(),
        };

        factory.register_default_constructors();
        factory.setup_type_mappings();
        factory
    }

    /// Create a value object from a schema definition and JSON value.
    pub fn create_value_object(
        &self,
        definition: &AttributeDefinition,
        value: &Value,
    ) -> ValidationResult<Box<dyn ValueObject>> {
        // Handle null values for optional attributes
        if matches!(value, Value::Null) && !definition.required {
            return Err(ValidationError::NullValueForOptionalAttribute(
                definition.name.clone(),
            ));
        }

        // Handle multi-valued attributes
        if definition.multi_valued {
            return self.create_multi_valued_object(definition, value);
        }

        // Try registered constructors first
        match self.registry.create_value_object(definition, value) {
            Ok(obj) => Ok(obj),
            Err(_) => {
                // Fall back to extension attribute if no specific constructor found
                self.create_extension_attribute(definition, value)
            }
        }
    }

    /// Create a multi-valued attribute object.
    fn create_multi_valued_object(
        &self,
        definition: &AttributeDefinition,
        value: &Value,
    ) -> ValidationResult<Box<dyn ValueObject>> {
        let array = value
            .as_array()
            .ok_or_else(|| ValidationError::ExpectedArray(definition.name.clone()))?;

        // Create individual value objects for each array element
        let mut objects = Vec::new();
        for item_value in array {
            // Create a single-valued definition for each item
            let item_definition = AttributeDefinition {
                multi_valued: false,
                ..definition.clone()
            };

            let obj = self.create_value_object(&item_definition, item_value)?;
            objects.push(obj);
        }

        // For now, we'll create a generic multi-valued container
        // In a more sophisticated implementation, we could have specific
        // multi-valued types for different attribute types
        Ok(Box::new(GenericMultiValuedAttribute::new(
            definition.name.clone(),
            objects,
        )))
    }

    /// Create an extension attribute value object.
    fn create_extension_attribute(
        &self,
        definition: &AttributeDefinition,
        value: &Value,
    ) -> ValidationResult<Box<dyn ValueObject>> {
        // Create a default schema URI for unknown extensions
        let schema_uri = SchemaUri::new(format!(
            "urn:ietf:params:scim:schemas:extension:unknown:{}",
            definition.name
        ))?;

        let ext_attr = ExtensionAttributeValue::new(
            schema_uri,
            definition.name.clone(),
            value.clone(),
            Some(definition.clone()),
        )?;

        Ok(Box::new(ext_attr))
    }

    /// Register default constructors for built-in value objects.
    fn register_default_constructors(&mut self) {
        // Register constructors for core value objects
        self.registry
            .register_constructor(Box::new(ResourceIdConstructor::new()));
        self.registry
            .register_constructor(Box::new(UserNameConstructor::new()));
        self.registry
            .register_constructor(Box::new(ExternalIdConstructor::new()));
        self.registry
            .register_constructor(Box::new(EmailAddressConstructor::new()));
        self.registry
            .register_constructor(Box::new(SchemaUriConstructor::new()));
        self.registry
            .register_constructor(Box::new(NameConstructor::new()));
        // TODO: Add constructors for Address, PhoneNumber, and Meta when from_json methods are implemented
    }

    /// Setup mappings from attribute names to expected types.
    fn setup_type_mappings(&mut self) {
        self.type_mappings
            .insert("id".to_string(), AttributeType::String);
        self.type_mappings
            .insert("userName".to_string(), AttributeType::String);
        self.type_mappings
            .insert("externalId".to_string(), AttributeType::String);
        self.type_mappings
            .insert("schemas".to_string(), AttributeType::Reference);
        self.type_mappings
            .insert("name".to_string(), AttributeType::Complex);
        self.type_mappings
            .insert("emails".to_string(), AttributeType::Complex);
        self.type_mappings
            .insert("phoneNumbers".to_string(), AttributeType::Complex);
        self.type_mappings
            .insert("addresses".to_string(), AttributeType::Complex);
        self.type_mappings
            .insert("meta".to_string(), AttributeType::Complex);
    }

    /// Get the expected attribute type for a given attribute name.
    pub fn get_expected_type(&self, attribute_name: &str) -> Option<AttributeType> {
        self.type_mappings.get(attribute_name).cloned()
    }

    /// Register a custom constructor.
    pub fn register_constructor(&mut self, constructor: Box<dyn ValueObjectConstructor>) {
        self.registry.register_constructor(constructor);
    }

    /// Validate composite rules across multiple value objects.
    pub fn validate_composite_rules(
        &self,
        objects: &[Box<dyn ValueObject>],
    ) -> ValidationResult<()> {
        self.registry.validate_composite_rules(objects)
    }

    /// Check if the factory has any constructors registered.
    pub fn has_constructors(&self) -> bool {
        self.registry.has_constructors()
    }

    /// Create a collection of value objects from a JSON object.
    pub fn create_value_objects_from_json(
        &self,
        definitions: &[AttributeDefinition],
        json_obj: &serde_json::Map<String, Value>,
    ) -> ValidationResult<Vec<Box<dyn ValueObject>>> {
        let mut objects = Vec::new();

        for definition in definitions {
            if let Some(value) = json_obj.get(&definition.name) {
                let obj = self.create_value_object(definition, value)?;
                objects.push(obj);
            } else if definition.required {
                return Err(ValidationError::RequiredAttributeMissing(
                    definition.name.clone(),
                ));
            }
        }

        Ok(objects)
    }
}

impl Default for ValueObjectFactory {
    fn default() -> Self {
        Self::new()
    }
}

/// Generic multi-valued attribute container for factory-created objects.
#[derive(Debug)]
pub struct GenericMultiValuedAttribute {
    attribute_name: String,
    values: Vec<Box<dyn ValueObject>>,
    primary_index: Option<usize>,
}

impl GenericMultiValuedAttribute {
    pub fn new(attribute_name: String, values: Vec<Box<dyn ValueObject>>) -> Self {
        Self {
            attribute_name,
            values,
            primary_index: None,
        }
    }

    pub fn values(&self) -> &[Box<dyn ValueObject>] {
        &self.values
    }

    pub fn primary(&self) -> Option<&Box<dyn ValueObject>> {
        self.primary_index.map(|idx| &self.values[idx])
    }

    pub fn set_primary(&mut self, index: usize) -> ValidationResult<()> {
        if index >= self.values.len() {
            return Err(ValidationError::InvalidPrimaryIndex {
                attribute: self.attribute_name.clone(),
                index,
                length: self.values.len(),
            });
        }
        self.primary_index = Some(index);
        Ok(())
    }
}

impl ValueObject for GenericMultiValuedAttribute {
    fn attribute_type(&self) -> AttributeType {
        AttributeType::Complex // Multi-valued attributes are complex
    }

    fn attribute_name(&self) -> &str {
        &self.attribute_name
    }

    fn to_json(&self) -> ValidationResult<Value> {
        let mut array = Vec::new();
        for value_obj in &self.values {
            array.push(value_obj.to_json()?);
        }
        Ok(Value::Array(array))
    }

    fn validate_against_schema(&self, definition: &AttributeDefinition) -> ValidationResult<()> {
        if !definition.multi_valued {
            return Err(ValidationError::NotMultiValued(definition.name.clone()));
        }

        // Validate each value object
        let single_def = AttributeDefinition {
            multi_valued: false,
            ..definition.clone()
        };

        for value_obj in &self.values {
            value_obj.validate_against_schema(&single_def)?;
        }

        Ok(())
    }

    fn as_json_value(&self) -> Value {
        self.to_json().unwrap_or(Value::Null)
    }

    fn supports_definition(&self, definition: &AttributeDefinition) -> bool {
        definition.multi_valued && definition.name == self.attribute_name
    }

    fn clone_boxed(&self) -> Box<dyn ValueObject> {
        Box::new(GenericMultiValuedAttribute {
            attribute_name: self.attribute_name.clone(),
            values: self.values.iter().map(|v| v.clone_boxed()).collect(),
            primary_index: self.primary_index,
        })
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

// Specific constructors for built-in value objects

pub struct ResourceIdConstructor;

impl ResourceIdConstructor {
    pub fn new() -> Self {
        Self
    }
}

impl ValueObjectConstructor for ResourceIdConstructor {
    fn try_construct(
        &self,
        definition: &AttributeDefinition,
        value: &Value,
    ) -> Option<ValidationResult<Box<dyn ValueObject>>> {
        if definition.name == "id" && definition.data_type == AttributeType::String {
            if let Some(id_str) = value.as_str() {
                Some(
                    ResourceId::new(id_str.to_string())
                        .map(|id| Box::new(id) as Box<dyn ValueObject>),
                )
            } else {
                Some(Err(ValidationError::InvalidAttributeType {
                    attribute: definition.name.clone(),
                    expected: "string".to_string(),
                    actual: "non-string".to_string(),
                }))
            }
        } else {
            None
        }
    }

    fn priority(&self) -> u8 {
        100 // High priority for exact matches
    }

    fn description(&self) -> &str {
        "ResourceId constructor for 'id' attributes"
    }
}

pub struct UserNameConstructor;

impl UserNameConstructor {
    pub fn new() -> Self {
        Self
    }
}

impl ValueObjectConstructor for UserNameConstructor {
    fn try_construct(
        &self,
        definition: &AttributeDefinition,
        value: &Value,
    ) -> Option<ValidationResult<Box<dyn ValueObject>>> {
        if definition.name == "userName" && definition.data_type == AttributeType::String {
            if let Some(username_str) = value.as_str() {
                Some(
                    UserName::new(username_str.to_string())
                        .map(|username| Box::new(username) as Box<dyn ValueObject>),
                )
            } else {
                Some(Err(ValidationError::InvalidAttributeType {
                    attribute: definition.name.clone(),
                    expected: "string".to_string(),
                    actual: "non-string".to_string(),
                }))
            }
        } else {
            None
        }
    }

    fn priority(&self) -> u8 {
        100
    }

    fn description(&self) -> &str {
        "UserName constructor for 'userName' attributes"
    }
}

pub struct ExternalIdConstructor;

impl ExternalIdConstructor {
    pub fn new() -> Self {
        Self
    }
}

impl ValueObjectConstructor for ExternalIdConstructor {
    fn try_construct(
        &self,
        definition: &AttributeDefinition,
        value: &Value,
    ) -> Option<ValidationResult<Box<dyn ValueObject>>> {
        if definition.name == "externalId" && definition.data_type == AttributeType::String {
            if let Some(ext_id_str) = value.as_str() {
                Some(
                    ExternalId::new(ext_id_str.to_string())
                        .map(|ext_id| Box::new(ext_id) as Box<dyn ValueObject>),
                )
            } else {
                Some(Err(ValidationError::InvalidAttributeType {
                    attribute: definition.name.clone(),
                    expected: "string".to_string(),
                    actual: "non-string".to_string(),
                }))
            }
        } else {
            None
        }
    }

    fn priority(&self) -> u8 {
        100
    }

    fn description(&self) -> &str {
        "ExternalId constructor for 'externalId' attributes"
    }
}

pub struct EmailAddressConstructor;

impl EmailAddressConstructor {
    pub fn new() -> Self {
        Self
    }
}

impl ValueObjectConstructor for EmailAddressConstructor {
    fn try_construct(
        &self,
        definition: &AttributeDefinition,
        value: &Value,
    ) -> Option<ValidationResult<Box<dyn ValueObject>>> {
        if (definition.name == "value" || definition.name.contains("email"))
            && definition.data_type == AttributeType::String
        {
            if let Some(email_str) = value.as_str() {
                Some(
                    EmailAddress::new(email_str.to_string(), None, None, None)
                        .map(|email| Box::new(email) as Box<dyn ValueObject>),
                )
            } else {
                Some(Err(ValidationError::InvalidAttributeType {
                    attribute: definition.name.clone(),
                    expected: "string".to_string(),
                    actual: "non-string".to_string(),
                }))
            }
        } else {
            None
        }
    }

    fn priority(&self) -> u8 {
        80 // Lower priority than exact name matches
    }

    fn description(&self) -> &str {
        "EmailAddress constructor for email-related attributes"
    }
}

pub struct SchemaUriConstructor;

impl SchemaUriConstructor {
    pub fn new() -> Self {
        Self
    }
}

impl ValueObjectConstructor for SchemaUriConstructor {
    fn try_construct(
        &self,
        _definition: &AttributeDefinition,
        _value: &Value,
    ) -> Option<ValidationResult<Box<dyn ValueObject>>> {
        // TODO: Implement when SchemaUri implements ValueObject trait
        None
    }

    fn priority(&self) -> u8 {
        100
    }

    fn description(&self) -> &str {
        "SchemaUri constructor for 'schemas' attributes"
    }
}

pub struct NameConstructor;

impl NameConstructor {
    pub fn new() -> Self {
        Self
    }
}

impl ValueObjectConstructor for NameConstructor {
    fn try_construct(
        &self,
        definition: &AttributeDefinition,
        value: &Value,
    ) -> Option<ValidationResult<Box<dyn ValueObject>>> {
        if definition.name == "name" && definition.data_type == AttributeType::Complex {
            Some(Name::from_json(value).map(|name| Box::new(name) as Box<dyn ValueObject>))
        } else {
            None
        }
    }

    fn priority(&self) -> u8 {
        100
    }

    fn description(&self) -> &str {
        "Name constructor for 'name' complex attributes"
    }
}

// TODO: Implement constructors for Address, PhoneNumber, and Meta when from_json methods are available

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::types::{Mutability, Uniqueness};

    fn create_string_definition(name: &str) -> AttributeDefinition {
        AttributeDefinition {
            name: name.to_string(),
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
    fn test_factory_creation() {
        let factory = ValueObjectFactory::new();
        assert!(factory.has_constructors());
    }

    #[test]
    fn test_resource_id_construction() {
        let factory = ValueObjectFactory::new();
        let definition = create_string_definition("id");
        let value = Value::String("test-id".to_string());

        let result = factory.create_value_object(&definition, &value);
        assert!(result.is_ok());

        let obj = result.unwrap();
        assert_eq!(obj.attribute_name(), "id");
        assert_eq!(obj.attribute_type(), AttributeType::String);
    }

    #[test]
    fn test_username_construction() {
        let factory = ValueObjectFactory::new();
        let definition = create_string_definition("userName");
        let value = Value::String("testuser".to_string());

        let result = factory.create_value_object(&definition, &value);
        assert!(result.is_ok());

        let obj = result.unwrap();
        assert_eq!(obj.attribute_name(), "userName");
    }
}
