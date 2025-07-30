//! Schema mapping functionality for converting between SCIM and implementation formats.
//!
//! This module provides traits and implementations for mapping between
//! SCIM schema representations and backend implementation schemas,
//! such as database column mappings.

use crate::error::ScimError;
use serde_json::Value;
use std::collections::HashMap;

/// Trait for mapping between SCIM schema and implementation schema (e.g., database)
pub trait SchemaMapper: Send + Sync {
    fn to_implementation(&self, scim_data: &Value) -> Result<Value, ScimError>;
    fn from_implementation(&self, impl_data: &Value) -> Result<Value, ScimError>;
}

/// Database schema mapper for converting between SCIM and database formats
pub struct DatabaseMapper {
    pub table_name: String,
    pub column_mappings: HashMap<String, String>, // SCIM attribute -> DB column
}

impl DatabaseMapper {
    pub fn new(table_name: &str, mappings: HashMap<String, String>) -> Self {
        Self {
            table_name: table_name.to_string(),
            column_mappings: mappings,
        }
    }
}

impl SchemaMapper for DatabaseMapper {
    fn to_implementation(&self, scim_data: &Value) -> Result<Value, ScimError> {
        let mut db_data = serde_json::Map::new();

        if let Some(obj) = scim_data.as_object() {
            for (scim_attr, db_column) in &self.column_mappings {
                if let Some(value) = obj.get(scim_attr) {
                    db_data.insert(db_column.clone(), value.clone());
                }
            }
        }

        Ok(Value::Object(db_data))
    }

    fn from_implementation(&self, impl_data: &Value) -> Result<Value, ScimError> {
        let mut scim_data = serde_json::Map::new();

        if let Some(obj) = impl_data.as_object() {
            for (scim_attr, db_column) in &self.column_mappings {
                if let Some(value) = obj.get(db_column) {
                    scim_data.insert(scim_attr.clone(), value.clone());
                }
            }
        }

        Ok(Value::Object(scim_data))
    }
}
