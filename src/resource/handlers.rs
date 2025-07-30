//! Handler infrastructure for dynamic resource operations.
//!
//! This module provides the infrastructure for creating and managing
//! dynamic resource handlers that can be configured at runtime with
//! custom attribute handlers, mappers, and methods.

use super::mapper::SchemaMapper;
use crate::error::ScimError;
use crate::schema::Schema;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;

/// Dynamic attribute handler for schema-driven operations
#[derive(Clone)]
pub enum AttributeHandler {
    Getter(Arc<dyn Fn(&Value) -> Option<Value> + Send + Sync>),
    Setter(Arc<dyn Fn(&mut Value, Value) -> Result<(), ScimError> + Send + Sync>),
    Transformer(Arc<dyn Fn(&Value, &str) -> Option<Value> + Send + Sync>),
}

/// Handler for a specific resource type containing all its dynamic behaviors
#[derive(Clone)]
pub struct ResourceHandler {
    pub schema: Schema,
    pub handlers: HashMap<String, AttributeHandler>,
    pub mappers: Vec<Arc<dyn SchemaMapper>>,
    pub custom_methods:
        HashMap<String, Arc<dyn Fn(&DynamicResource) -> Result<Value, ScimError> + Send + Sync>>,
}

impl std::fmt::Debug for ResourceHandler {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ResourceHandler")
            .field("schema", &self.schema)
            .field("handlers", &format!("{} handlers", self.handlers.len()))
            .field("mappers", &format!("{} mappers", self.mappers.len()))
            .field(
                "custom_methods",
                &format!("{} custom methods", self.custom_methods.len()),
            )
            .finish()
    }
}

/// Builder for creating resource handlers with fluent API
pub struct SchemaResourceBuilder {
    schema: Schema,
    handlers: HashMap<String, AttributeHandler>,
    mappers: Vec<Arc<dyn SchemaMapper>>,
    custom_methods:
        HashMap<String, Arc<dyn Fn(&DynamicResource) -> Result<Value, ScimError> + Send + Sync>>,
}

impl SchemaResourceBuilder {
    pub fn new(schema: Schema) -> Self {
        Self {
            schema,
            handlers: HashMap::new(),
            mappers: Vec::new(),
            custom_methods: HashMap::new(),
        }
    }

    pub fn with_getter<F>(mut self, attribute: &str, getter: F) -> Self
    where
        F: Fn(&Value) -> Option<Value> + Send + Sync + 'static,
    {
        self.handlers.insert(
            format!("get_{}", attribute),
            AttributeHandler::Getter(Arc::new(getter)),
        );
        self
    }

    pub fn with_setter<F>(mut self, attribute: &str, setter: F) -> Self
    where
        F: Fn(&mut Value, Value) -> Result<(), ScimError> + Send + Sync + 'static,
    {
        self.handlers.insert(
            format!("set_{}", attribute),
            AttributeHandler::Setter(Arc::new(setter)),
        );
        self
    }

    pub fn with_transformer<F>(mut self, attribute: &str, transformer: F) -> Self
    where
        F: Fn(&Value, &str) -> Option<Value> + Send + Sync + 'static,
    {
        self.handlers.insert(
            format!("transform_{}", attribute),
            AttributeHandler::Transformer(Arc::new(transformer)),
        );
        self
    }

    pub fn with_custom_method<F>(mut self, method_name: &str, method: F) -> Self
    where
        F: Fn(&DynamicResource) -> Result<Value, ScimError> + Send + Sync + 'static,
    {
        self.custom_methods
            .insert(method_name.to_string(), Arc::new(method));
        self
    }

    pub fn with_mapper(mut self, mapper: Arc<dyn SchemaMapper>) -> Self {
        self.mappers.push(mapper);
        self
    }

    pub fn with_database_mapping(
        self,
        table_name: &str,
        column_mappings: HashMap<String, String>,
    ) -> Self {
        self.with_mapper(Arc::new(super::mapper::DatabaseMapper::new(
            table_name,
            column_mappings,
        )))
    }

    pub fn build(self) -> ResourceHandler {
        ResourceHandler {
            schema: self.schema,
            handlers: self.handlers,
            mappers: self.mappers,
            custom_methods: self.custom_methods,
        }
    }
}

/// Dynamic resource that uses registered handlers for operations
#[derive(Clone, Debug)]
pub struct DynamicResource {
    pub resource_type: String,
    pub data: Value,
    pub handler: Arc<ResourceHandler>,
}

impl DynamicResource {
    pub fn new(resource_type: String, data: Value, handler: Arc<ResourceHandler>) -> Self {
        Self {
            resource_type,
            data,
            handler,
        }
    }

    pub fn get_attribute_dynamic(&self, attribute: &str) -> Option<Value> {
        let getter_key = format!("get_{}", attribute);
        if let Some(AttributeHandler::Getter(getter)) = self.handler.handlers.get(&getter_key) {
            getter(&self.data)
        } else {
            // Fallback to direct access
            self.data.get(attribute).cloned()
        }
    }

    pub fn set_attribute_dynamic(
        &mut self,
        attribute: &str,
        value: Value,
    ) -> Result<(), ScimError> {
        let setter_key = format!("set_{}", attribute);
        if let Some(AttributeHandler::Setter(setter)) = self.handler.handlers.get(&setter_key) {
            setter(&mut self.data, value)
        } else {
            // Fallback to direct setting
            if let Some(obj) = self.data.as_object_mut() {
                obj.insert(attribute.to_string(), value);
            }
            Ok(())
        }
    }

    pub fn call_custom_method(&self, method_name: &str) -> Result<Value, ScimError> {
        if let Some(method) = self.handler.custom_methods.get(method_name) {
            method(self)
        } else {
            Err(ScimError::MethodNotFound(method_name.to_string()))
        }
    }

    pub fn to_implementation_schema(&self, mapper_index: usize) -> Result<Value, ScimError> {
        if let Some(mapper) = self.handler.mappers.get(mapper_index) {
            mapper.to_implementation(&self.data)
        } else {
            Err(ScimError::MapperNotFound(mapper_index))
        }
    }

    pub fn from_implementation_schema(
        &mut self,
        impl_data: &Value,
        mapper_index: usize,
    ) -> Result<(), ScimError> {
        if let Some(mapper) = self.handler.mappers.get(mapper_index) {
            self.data = mapper.from_implementation(impl_data)?;
            Ok(())
        } else {
            Err(ScimError::MapperNotFound(mapper_index))
        }
    }
}
