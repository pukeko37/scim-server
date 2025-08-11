//! Core trait system for schema-driven value objects.
//!
//! This module defines the fundamental traits that enable dynamic value object
//! creation, validation, and schema-driven operations. These traits form the
//! foundation for the schema-driven value object system.
//!
//! ## Design Principles
//!
//! - **Schema-Driven**: Value objects can be created from schema definitions
//! - **Type-Safe**: Compile-time guarantees where possible, runtime validation where needed
//! - **Extensible**: Support for custom and extension attributes
//! - **Composable**: Enable validation across multiple value objects

use crate::error::{ValidationError, ValidationResult};
use crate::schema::types::{AttributeDefinition, AttributeType};
use serde_json::Value;
use std::any::Any;
use std::fmt::Debug;

/// Core trait for all SCIM value objects.
///
/// This trait enables dynamic value object operations while maintaining
/// type safety through the use of Any for downcasting when needed.
pub trait ValueObject: Debug + Send + Sync {
    /// Get the SCIM attribute type this value object represents
    fn attribute_type(&self) -> AttributeType;

    /// Get the schema attribute name this value object corresponds to
    fn attribute_name(&self) -> &str;

    /// Serialize the value object to JSON
    fn to_json(&self) -> ValidationResult<Value>;

    /// Validate the value object against a schema definition
    fn validate_against_schema(&self, definition: &AttributeDefinition) -> ValidationResult<()>;

    /// Get the raw value as a JSON Value for schema-agnostic operations
    fn as_json_value(&self) -> Value;

    /// Check if this value object supports the given attribute definition
    fn supports_definition(&self, definition: &AttributeDefinition) -> bool;

    /// Clone the value object as a boxed trait object
    fn clone_boxed(&self) -> Box<dyn ValueObject>;

    /// Get type information for downcasting
    fn as_any(&self) -> &dyn Any;
}

/// Trait for value objects that can be created from schema definitions.
///
/// This trait enables the dynamic factory pattern where value objects
/// are constructed based on schema attribute definitions and JSON values.
pub trait SchemaConstructible: ValueObject + Sized {
    /// Create a value object from a JSON value and schema definition
    fn from_schema_and_value(
        definition: &AttributeDefinition,
        value: &Value,
    ) -> ValidationResult<Self>;

    /// Check if this type can handle the given attribute definition
    fn can_construct_from(definition: &AttributeDefinition) -> bool;

    /// Get the priority for this constructor (higher = preferred)
    /// Used when multiple constructors might handle the same definition
    fn constructor_priority() -> u8 {
        50 // Default priority
    }
}

/// Trait for value objects that represent extension attributes.
///
/// Extension attributes are defined by custom schemas and may have
/// different validation rules and behaviors than core SCIM attributes.
pub trait ExtensionAttribute: ValueObject {
    /// Get the schema URI that defines this extension attribute
    fn schema_uri(&self) -> &str;

    /// Get the extension namespace (usually derived from schema URI)
    fn extension_namespace(&self) -> &str;

    /// Validate against extension-specific rules
    fn validate_extension_rules(&self) -> ValidationResult<()>;
}

/// Trait for composite validation across multiple value objects.
///
/// This enables validation rules that span multiple attributes or
/// require context from other value objects to validate properly.
pub trait CompositeValidator {
    /// Validate relationships between multiple value objects
    fn validate_composite(&self, objects: &[Box<dyn ValueObject>]) -> ValidationResult<()>;

    /// Get the names of attributes this validator depends on
    fn dependent_attributes(&self) -> Vec<String>;

    /// Check if this validator applies to the given set of attributes
    fn applies_to(&self, attribute_names: &[String]) -> bool;
}

/// Registry for value object constructors.
///
/// This registry maintains a mapping of attribute types and names to
/// constructor functions, enabling dynamic value object creation.
#[derive(Default)]
pub struct ValueObjectRegistry {
    constructors: Vec<Box<dyn ValueObjectConstructor>>,
    composite_validators: Vec<Box<dyn CompositeValidator>>,
}

/// Trait for value object constructors that can be registered.
pub trait ValueObjectConstructor: Send + Sync {
    /// Attempt to construct a value object from the given definition and value
    fn try_construct(
        &self,
        definition: &AttributeDefinition,
        value: &Value,
    ) -> Option<ValidationResult<Box<dyn ValueObject>>>;

    /// Get the priority of this constructor
    fn priority(&self) -> u8;

    /// Get a description of what this constructor handles
    fn description(&self) -> &str;
}

impl ValueObjectRegistry {
    /// Create a new registry with default constructors
    pub fn new() -> Self {
        let mut registry = Self::default();
        registry.register_default_constructors();
        registry
    }

    /// Register a value object constructor
    pub fn register_constructor(&mut self, constructor: Box<dyn ValueObjectConstructor>) {
        self.constructors.push(constructor);
        // Sort by priority (highest first)
        self.constructors
            .sort_by(|a, b| b.priority().cmp(&a.priority()));
    }

    /// Register a composite validator
    pub fn register_composite_validator(&mut self, validator: Box<dyn CompositeValidator>) {
        self.composite_validators.push(validator);
    }

    /// Create a value object from schema definition and JSON value
    pub fn create_value_object(
        &self,
        definition: &AttributeDefinition,
        value: &Value,
    ) -> ValidationResult<Box<dyn ValueObject>> {
        // Try each constructor in priority order
        for constructor in &self.constructors {
            if let Some(result) = constructor.try_construct(definition, value) {
                return result;
            }
        }

        Err(ValidationError::UnsupportedAttributeType {
            attribute: definition.name.clone(),
            type_name: format!("{:?}", definition.data_type),
        })
    }

    /// Validate composite rules across multiple value objects
    pub fn validate_composite_rules(
        &self,
        objects: &[Box<dyn ValueObject>],
    ) -> ValidationResult<()> {
        let attribute_names: Vec<String> = objects
            .iter()
            .map(|obj| obj.attribute_name().to_string())
            .collect();

        for validator in &self.composite_validators {
            if validator.applies_to(&attribute_names) {
                validator.validate_composite(objects)?;
            }
        }

        Ok(())
    }

    /// Check if the registry has any constructors registered
    pub fn has_constructors(&self) -> bool {
        !self.constructors.is_empty()
    }

    /// Register default constructors for built-in value objects
    fn register_default_constructors(&mut self) {
        // These will be implemented as we add support for each type
        // For now, we'll register placeholder constructors
    }
}

/// Helper macro for implementing ValueObject trait for existing value objects
#[macro_export]
macro_rules! impl_value_object {
    (
        $type:ty,
        attribute_type: $attr_type:expr,
        attribute_name: $attr_name:expr
    ) => {
        impl $crate::resource::value_objects::value_object_trait::ValueObject for $type {
            fn attribute_type(&self) -> $crate::schema::types::AttributeType {
                $attr_type
            }

            fn attribute_name(&self) -> &str {
                $attr_name
            }

            fn to_json(&self) -> $crate::error::ValidationResult<serde_json::Value> {
                Ok(serde_json::to_value(self)?)
            }

            fn validate_against_schema(
                &self,
                definition: &$crate::schema::types::AttributeDefinition,
            ) -> $crate::error::ValidationResult<()> {
                // Basic type checking
                if definition.data_type != self.attribute_type() {
                    return Err($crate::error::ValidationError::InvalidAttributeType(
                        definition.name.clone(),
                        format!("{:?}", definition.data_type),
                        format!("{:?}", self.attribute_type()),
                    ));
                }

                // Additional validation can be added here based on the specific type
                Ok(())
            }

            fn as_json_value(&self) -> serde_json::Value {
                self.to_json().unwrap_or(serde_json::Value::Null)
            }

            fn supports_definition(
                &self,
                definition: &$crate::schema::types::AttributeDefinition,
            ) -> bool {
                definition.data_type == self.attribute_type()
                    && definition.name == self.attribute_name()
            }

            fn clone_boxed(
                &self,
            ) -> Box<dyn $crate::resource::value_objects::value_object_trait::ValueObject> {
                Box::new(self.clone())
            }

            fn as_any(&self) -> &dyn std::any::Any {
                self
            }
        }
    };
}

/// Generic constructor for simple value objects
pub struct GenericValueObjectConstructor<T> {
    _phantom: std::marker::PhantomData<T>,
}

impl<T> GenericValueObjectConstructor<T> where T: SchemaConstructible + 'static {}

impl<T> ValueObjectConstructor for GenericValueObjectConstructor<T>
where
    T: SchemaConstructible + 'static,
{
    fn try_construct(
        &self,
        definition: &AttributeDefinition,
        value: &Value,
    ) -> Option<ValidationResult<Box<dyn ValueObject>>> {
        if T::can_construct_from(definition) {
            Some(
                T::from_schema_and_value(definition, value)
                    .map(|obj| Box::new(obj) as Box<dyn ValueObject>),
            )
        } else {
            None
        }
    }

    fn priority(&self) -> u8 {
        T::constructor_priority()
    }

    fn description(&self) -> &str {
        std::any::type_name::<T>()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::types::{AttributeType, Mutability, Uniqueness};

    // Mock value object for testing
    #[derive(Debug, Clone)]
    struct MockValueObject {
        name: String,
        value: String,
    }

    impl ValueObject for MockValueObject {
        fn attribute_type(&self) -> AttributeType {
            AttributeType::String
        }

        fn attribute_name(&self) -> &str {
            &self.name
        }

        fn to_json(&self) -> ValidationResult<Value> {
            Ok(Value::String(self.value.clone()))
        }

        fn validate_against_schema(
            &self,
            _definition: &AttributeDefinition,
        ) -> ValidationResult<()> {
            Ok(())
        }

        fn as_json_value(&self) -> Value {
            Value::String(self.value.clone())
        }

        fn supports_definition(&self, definition: &AttributeDefinition) -> bool {
            definition.data_type == AttributeType::String
        }

        fn clone_boxed(&self) -> Box<dyn ValueObject> {
            Box::new(self.clone())
        }

        fn as_any(&self) -> &dyn Any {
            self
        }
    }

    #[test]
    fn test_value_object_trait() {
        let obj = MockValueObject {
            name: "test".to_string(),
            value: "value".to_string(),
        };

        assert_eq!(obj.attribute_type(), AttributeType::String);
        assert_eq!(obj.attribute_name(), "test");
        assert_eq!(obj.as_json_value(), Value::String("value".to_string()));
    }

    #[test]
    fn test_value_object_registry() {
        let registry = ValueObjectRegistry::new();

        // Registry should be created successfully
        assert_eq!(registry.constructors.len(), 0); // No default constructors yet
        assert_eq!(registry.composite_validators.len(), 0);
    }

    #[test]
    fn test_validate_against_schema() {
        let obj = MockValueObject {
            name: "test".to_string(),
            value: "value".to_string(),
        };

        let definition = AttributeDefinition {
            name: "test".to_string(),
            data_type: AttributeType::String,
            multi_valued: false,
            required: false,
            case_exact: false,
            mutability: Mutability::ReadWrite,
            uniqueness: Uniqueness::None,
            canonical_values: vec![],
            sub_attributes: vec![],
            returned: None,
        };

        assert!(obj.validate_against_schema(&definition).is_ok());
    }
}
