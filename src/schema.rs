//! Schema definitions and validation for SCIM resources.
//!
//! This module provides the schema registry and validation engine for SCIM resources,
//! implementing the core User schema as defined in RFC 7643 with comprehensive
//! validation capabilities.

use crate::error::{ValidationError, ValidationResult};
use chrono::{DateTime, FixedOffset};
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
                    return Err(ValidationError::InvalidDataType {
                        attribute: attr_def.name.clone(),
                        expected: "string".to_string(),
                        actual: Self::get_value_type(value).to_string(),
                    });
                }

                // Validate string format constraints
                if let Some(str_val) = value.as_str() {
                    // Check for empty strings when not allowed
                    if str_val.is_empty() && attr_def.required {
                        return Err(ValidationError::InvalidStringFormat {
                            attribute: attr_def.name.clone(),
                            details: "String cannot be empty for required attribute".to_string(),
                        });
                    }

                    // Validate canonical values if specified
                    if !attr_def.canonical_values.is_empty() {
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
                    return Err(ValidationError::InvalidDataType {
                        attribute: attr_def.name.clone(),
                        expected: "boolean".to_string(),
                        actual: Self::get_value_type(value).to_string(),
                    });
                }

                // Additional boolean validation for string representations
                if value.is_string() {
                    if let Some(str_val) = value.as_str() {
                        if !["true", "false"].contains(&str_val.to_lowercase().as_str()) {
                            return Err(ValidationError::InvalidBooleanValue {
                                attribute: attr_def.name.clone(),
                                value: str_val.to_string(),
                            });
                        }
                    }
                }
            }
            AttributeType::Integer => {
                if !value.is_i64() {
                    return Err(ValidationError::InvalidDataType {
                        attribute: attr_def.name.clone(),
                        expected: "integer".to_string(),
                        actual: Self::get_value_type(value).to_string(),
                    });
                }

                // Validate integer range
                if let Some(num) = value.as_i64() {
                    if num < i32::MIN as i64 || num > i32::MAX as i64 {
                        return Err(ValidationError::InvalidIntegerValue {
                            attribute: attr_def.name.clone(),
                            value: num.to_string(),
                        });
                    }
                }
            }
            AttributeType::Decimal => {
                if !value.is_f64() && !value.is_i64() {
                    return Err(ValidationError::InvalidDataType {
                        attribute: attr_def.name.clone(),
                        expected: "decimal".to_string(),
                        actual: Self::get_value_type(value).to_string(),
                    });
                }

                // Validate decimal format for string representations
                if value.is_string() {
                    if let Some(str_val) = value.as_str() {
                        if str_val.parse::<f64>().is_err() {
                            return Err(ValidationError::InvalidDecimalFormat {
                                attribute: attr_def.name.clone(),
                                value: str_val.to_string(),
                            });
                        }
                    }
                }
            }
            AttributeType::DateTime => {
                if !value.is_string() {
                    return Err(ValidationError::InvalidDataType {
                        attribute: attr_def.name.clone(),
                        expected: "dateTime".to_string(),
                        actual: Self::get_value_type(value).to_string(),
                    });
                }

                // Basic datetime format validation
                if let Some(str_val) = value.as_str() {
                    if !self.is_valid_datetime_format(str_val) {
                        return Err(ValidationError::InvalidDateTimeFormat {
                            attribute: attr_def.name.clone(),
                            value: str_val.to_string(),
                        });
                    }
                }
            }
            AttributeType::Binary => {
                if !value.is_string() {
                    return Err(ValidationError::InvalidDataType {
                        attribute: attr_def.name.clone(),
                        expected: "binary".to_string(),
                        actual: Self::get_value_type(value).to_string(),
                    });
                }

                // Basic base64 validation
                if let Some(str_val) = value.as_str() {
                    if !self.is_valid_base64(str_val) {
                        return Err(ValidationError::InvalidBinaryData {
                            attribute: attr_def.name.clone(),
                            details: "Invalid base64 encoding".to_string(),
                        });
                    }
                }
            }
            AttributeType::Reference => {
                if !value.is_string() {
                    return Err(ValidationError::InvalidDataType {
                        attribute: attr_def.name.clone(),
                        expected: "reference".to_string(),
                        actual: Self::get_value_type(value).to_string(),
                    });
                }

                // Basic URI format validation
                if let Some(str_val) = value.as_str() {
                    if !self.is_valid_uri_format(str_val) {
                        return Err(ValidationError::InvalidReferenceUri {
                            attribute: attr_def.name.clone(),
                            uri: str_val.to_string(),
                        });
                    }
                }
            }
            AttributeType::Complex => {
                if !value.is_object() {
                    return Err(ValidationError::InvalidDataType {
                        attribute: attr_def.name.clone(),
                        expected: "complex".to_string(),
                        actual: Self::get_value_type(value).to_string(),
                    });
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
    fn is_valid_datetime_format(&self, value: &str) -> bool {
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
    fn is_valid_base64(&self, value: &str) -> bool {
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
    fn is_valid_uri_format(&self, value: &str) -> bool {
        if value.is_empty() {
            return false;
        }

        // Basic scheme validation - sufficient for SCIM reference URIs
        // Accepts HTTP(S) URLs and URN schemes commonly used in SCIM
        value.contains("://") || value.starts_with("urn:")
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

    /// Validate a complete SCIM resource including schemas array and meta attributes.
    pub fn validate_scim_resource(&self, resource: &Value) -> ValidationResult<()> {
        let obj = resource
            .as_object()
            .ok_or_else(|| ValidationError::custom("Resource must be a JSON object"))?;

        // 1. Validate schemas array
        self.validate_schemas_attribute(obj)?;

        // 2. Validate common attributes
        self.validate_id_attribute(obj)?;
        self.validate_external_id(obj)?;

        // 3. Validate meta attributes
        self.validate_meta_attribute(obj)?;

        // 4. Validate multi-valued attributes
        self.validate_multi_valued_attributes(obj)?;

        // 5. Validate complex attributes
        self.validate_complex_attributes(obj)?;

        // 6. Extract schema IDs and validate against each
        let schemas = self.extract_schema_uris(obj)?;
        for schema_uri in &schemas {
            if let Some(schema) = self.get_schema_by_id(schema_uri) {
                self.validate_resource(schema, resource)?;
            } else {
                return Err(ValidationError::UnknownSchemaUri {
                    uri: schema_uri.to_string(),
                });
            }
        }

        Ok(())
    }

    /// Validate multi-valued attributes (Errors 33-38)
    fn validate_multi_valued_attributes(
        &self,
        obj: &serde_json::Map<String, Value>,
    ) -> ValidationResult<()> {
        // Define multi-valued attributes that should be arrays
        let multi_valued_attrs = [
            "emails",
            "phoneNumbers",
            "ims",
            "photos",
            "addresses",
            "groups",
            "entitlements",
            "roles",
            "x509Certificates",
        ];

        // Define single-valued attributes that should NOT be arrays
        let single_valued_attrs = [
            "userName",
            "displayName",
            "nickName",
            "profileUrl",
            "title",
            "userType",
            "preferredLanguage",
            "locale",
            "timezone",
            "active",
            "password",
        ];

        // Check multi-valued attributes
        for attr_name in multi_valued_attrs {
            if let Some(value) = obj.get(attr_name) {
                // Error #33: Single value for multi-valued attribute
                if !value.is_array() {
                    return Err(ValidationError::SingleValueForMultiValued {
                        attribute: attr_name.to_string(),
                    });
                }

                // Validate array structure and contents
                if let Some(array) = value.as_array() {
                    self.validate_multi_valued_array(attr_name, array)?;
                }
            }
        }

        // Check single-valued attributes
        for attr_name in single_valued_attrs {
            if let Some(value) = obj.get(attr_name) {
                // Error #34: Array for single-valued attribute
                if value.is_array() {
                    return Err(ValidationError::ArrayForSingleValued {
                        attribute: attr_name.to_string(),
                    });
                }
            }
        }

        Ok(())
    }

    /// Validate the structure and contents of a multi-valued array
    fn validate_multi_valued_array(
        &self,
        attr_name: &str,
        array: &[Value],
    ) -> ValidationResult<()> {
        let mut primary_count = 0;

        for (index, item) in array.iter().enumerate() {
            // Error #36: Invalid multi-valued structure - items should be objects for complex multi-valued attributes
            if matches!(
                attr_name,
                "emails" | "phoneNumbers" | "ims" | "photos" | "addresses"
            ) {
                if !item.is_object() {
                    return Err(ValidationError::InvalidMultiValuedStructure {
                        attribute: attr_name.to_string(),
                        details: format!("Item at index {} is not an object", index),
                    });
                }

                if let Some(obj) = item.as_object() {
                    // Error #35: Multiple primary values
                    if let Some(primary) = obj.get("primary") {
                        if primary.as_bool() == Some(true) {
                            primary_count += 1;
                            if primary_count > 1 {
                                return Err(ValidationError::MultiplePrimaryValues {
                                    attribute: attr_name.to_string(),
                                });
                            }
                        }
                    }

                    // Error #37: Missing required sub-attribute
                    self.validate_required_sub_attributes(attr_name, obj)?;

                    // Error #38: Invalid canonical value
                    self.validate_canonical_values(attr_name, obj)?;
                }
            }
        }

        Ok(())
    }

    /// Validate required sub-attributes in multi-valued complex attributes
    fn validate_required_sub_attributes(
        &self,
        attr_name: &str,
        obj: &serde_json::Map<String, Value>,
    ) -> ValidationResult<()> {
        match attr_name {
            "emails" => {
                // Email requires 'value' sub-attribute
                if !obj.contains_key("value") {
                    return Err(ValidationError::MissingRequiredSubAttribute {
                        attribute: attr_name.to_string(),
                        sub_attribute: "value".to_string(),
                    });
                }
            }
            "phoneNumbers" => {
                // Phone number requires 'value' sub-attribute
                if !obj.contains_key("value") {
                    return Err(ValidationError::MissingRequiredSubAttribute {
                        attribute: attr_name.to_string(),
                        sub_attribute: "value".to_string(),
                    });
                }
            }
            "addresses" => {
                // Address requires at least one of the core fields
                let required_fields = [
                    "formatted",
                    "streetAddress",
                    "locality",
                    "region",
                    "postalCode",
                    "country",
                ];
                if !required_fields.iter().any(|field| obj.contains_key(*field)) {
                    return Err(ValidationError::MissingRequiredSubAttribute {
                        attribute: attr_name.to_string(),
                        sub_attribute: "formatted or address components".to_string(),
                    });
                }
            }
            _ => {} // Other multi-valued attributes may not have strict requirements
        }
        Ok(())
    }

    /// Validate canonical values in multi-valued attributes
    fn validate_canonical_values(
        &self,
        attr_name: &str,
        obj: &serde_json::Map<String, Value>,
    ) -> ValidationResult<()> {
        if let Some(type_value) = obj.get("type") {
            if let Some(type_str) = type_value.as_str() {
                let allowed_values = match attr_name {
                    "emails" => vec!["work", "home", "other"],
                    "phoneNumbers" => vec!["work", "home", "mobile", "fax", "pager", "other"],
                    "ims" => vec!["aim", "gtalk", "icq", "xmpp", "msn", "skype", "qq", "yahoo"],
                    "photos" => vec!["photo", "thumbnail"],
                    "addresses" => vec!["work", "home", "other"],
                    _ => return Ok(()), // No canonical values defined for this attribute
                };

                if !allowed_values.contains(&type_str) {
                    return Err(ValidationError::InvalidCanonicalValue {
                        attribute: attr_name.to_string(),
                        value: type_str.to_string(),
                        allowed: allowed_values.into_iter().map(String::from).collect(),
                    });
                }
            }
        }
        Ok(())
    }

    /// Validate complex attributes (Errors 39-43)
    fn validate_complex_attributes(
        &self,
        obj: &serde_json::Map<String, Value>,
    ) -> ValidationResult<()> {
        // Define complex attributes that need validation
        let complex_attrs = ["name", "addresses"];

        for attr_name in complex_attrs {
            if let Some(attr_value) = obj.get(attr_name) {
                // Skip if null
                if attr_value.is_null() {
                    continue;
                }

                // Error #43: Malformed complex structure - must be object
                if !attr_value.is_object() {
                    return Err(ValidationError::MalformedComplexStructure {
                        attribute: attr_name.to_string(),
                        details: "Complex attribute must be an object".to_string(),
                    });
                }

                if let Some(obj) = attr_value.as_object() {
                    self.validate_complex_attribute_structure(attr_name, obj)?;
                }
            }
        }

        Ok(())
    }

    /// Validate the structure of a specific complex attribute
    fn validate_complex_attribute_structure(
        &self,
        attr_name: &str,
        attr_obj: &serde_json::Map<String, Value>,
    ) -> ValidationResult<()> {
        // Get attribute definition from schema
        if let Some(attr_def) = self.get_complex_attribute_definition(attr_name) {
            if !attr_def.sub_attributes.is_empty() {
                let sub_attrs = &attr_def.sub_attributes;

                // Check for unknown sub-attributes (Error #41)
                self.validate_known_sub_attributes(attr_name, attr_obj, sub_attrs)?;

                // Check sub-attribute types (Error #40)
                self.validate_sub_attribute_types(attr_name, attr_obj, sub_attrs)?;

                // Check for nested complex attributes (Error #42)
                self.validate_no_nested_complex(attr_name, attr_obj, sub_attrs)?;

                // Check required sub-attributes (Error #39)
                self.validate_required_sub_attributes_complex(attr_name, attr_obj, sub_attrs)?;
            }
        }

        Ok(())
    }

    /// Get attribute definition for a complex attribute
    fn get_complex_attribute_definition(&self, attr_name: &str) -> Option<&AttributeDefinition> {
        // Look in core user schema for the attribute
        self.core_user_schema
            .attributes
            .iter()
            .find(|attr| attr.name == attr_name && matches!(attr.data_type, AttributeType::Complex))
    }

    /// Validate that all sub-attributes are known/allowed
    fn validate_known_sub_attributes(
        &self,
        attr_name: &str,
        attr_obj: &serde_json::Map<String, Value>,
        sub_attrs: &[AttributeDefinition],
    ) -> ValidationResult<()> {
        let valid_sub_attr_names: std::collections::HashSet<&str> =
            sub_attrs.iter().map(|attr| attr.name.as_str()).collect();

        for sub_attr_name in attr_obj.keys() {
            if !valid_sub_attr_names.contains(sub_attr_name.as_str()) {
                return Err(ValidationError::UnknownSubAttribute {
                    attribute: attr_name.to_string(),
                    sub_attribute: sub_attr_name.to_string(),
                });
            }
        }

        Ok(())
    }

    /// Validate sub-attribute data types
    fn validate_sub_attribute_types(
        &self,
        attr_name: &str,
        attr_obj: &serde_json::Map<String, Value>,
        sub_attrs: &[AttributeDefinition],
    ) -> ValidationResult<()> {
        for sub_attr_def in sub_attrs {
            if let Some(sub_attr_value) = attr_obj.get(&sub_attr_def.name) {
                // Skip null values
                if sub_attr_value.is_null() {
                    continue;
                }

                let expected_type = match sub_attr_def.data_type {
                    AttributeType::String => "string",
                    AttributeType::Boolean => "boolean",
                    AttributeType::Integer => "integer",
                    AttributeType::Decimal => "number",
                    AttributeType::DateTime => "string",
                    AttributeType::Binary => "string",
                    AttributeType::Reference => "string",
                    AttributeType::Complex => "object",
                };

                let actual_type = Self::get_value_type(sub_attr_value);

                if expected_type != actual_type {
                    return Err(ValidationError::InvalidSubAttributeType {
                        attribute: attr_name.to_string(),
                        sub_attribute: sub_attr_def.name.clone(),
                        expected: expected_type.to_string(),
                        actual: actual_type.to_string(),
                    });
                }
            }
        }

        Ok(())
    }

    /// Validate that no complex attributes are nested within this complex attribute
    fn validate_no_nested_complex(
        &self,
        attr_name: &str,
        attr_obj: &serde_json::Map<String, Value>,
        sub_attrs: &[AttributeDefinition],
    ) -> ValidationResult<()> {
        for sub_attr_def in sub_attrs {
            if matches!(sub_attr_def.data_type, AttributeType::Complex) {
                if attr_obj.contains_key(&sub_attr_def.name) {
                    return Err(ValidationError::NestedComplexAttributes {
                        attribute: format!("{}.{}", attr_name, sub_attr_def.name),
                    });
                }
            }
        }

        Ok(())
    }

    /// Validate required sub-attributes for complex attributes
    fn validate_required_sub_attributes_complex(
        &self,
        attr_name: &str,
        attr_obj: &serde_json::Map<String, Value>,
        sub_attrs: &[AttributeDefinition],
    ) -> ValidationResult<()> {
        let missing: Vec<String> = sub_attrs
            .iter()
            .filter(|attr| attr.required)
            .filter(|attr| !attr_obj.contains_key(&attr.name) || attr_obj[&attr.name].is_null())
            .map(|attr| attr.name.clone())
            .collect();

        if !missing.is_empty() {
            return Err(ValidationError::MissingRequiredSubAttributes {
                attribute: attr_name.to_string(),
                missing,
            });
        }

        Ok(())
    }

    /// Validate the schemas attribute according to SCIM requirements.
    fn validate_schemas_attribute(
        &self,
        obj: &serde_json::Map<String, Value>,
    ) -> ValidationResult<()> {
        // Check if schemas attribute exists
        let schemas_value = obj
            .get("schemas")
            .ok_or_else(|| ValidationError::MissingSchemas)?;

        // Check if schemas is an array
        let schemas_array = schemas_value
            .as_array()
            .ok_or_else(|| ValidationError::InvalidMetaStructure)?;

        // Check if schemas array is not empty
        if schemas_array.is_empty() {
            return Err(ValidationError::EmptySchemas);
        }

        // Validate each schema URI
        let mut seen_uris = std::collections::HashSet::new();
        for schema_value in schemas_array {
            let schema_uri = schema_value
                .as_str()
                .ok_or_else(|| ValidationError::InvalidMetaStructure)?;

            // Check for duplicates
            if !seen_uris.insert(schema_uri) {
                return Err(ValidationError::DuplicateSchemaUri {
                    uri: schema_uri.to_string(),
                });
            }

            // Validate URI format first
            if !self.is_valid_schema_uri(schema_uri) {
                return Err(ValidationError::InvalidSchemaUri {
                    uri: schema_uri.to_string(),
                });
            }

            // Then check if the URI is registered (only after format validation passes)
            if !self.schemas.contains_key(schema_uri) {
                return Err(ValidationError::UnknownSchemaUri {
                    uri: schema_uri.to_string(),
                });
            }
        }

        // Validate schema URI combinations
        self.validate_schema_combinations(&seen_uris)?;

        Ok(())
    }

    /// Validate the meta attribute structure.
    /// Validate the id attribute (Errors 9-12)
    fn validate_id_attribute(&self, obj: &serde_json::Map<String, Value>) -> ValidationResult<()> {
        // Check if id attribute exists
        let id_value = obj.get("id").ok_or_else(|| ValidationError::MissingId)?;

        // Check if id is a string
        let id_str = id_value
            .as_str()
            .ok_or_else(|| ValidationError::InvalidIdFormat {
                id: format!("{:?}", id_value),
            })?;

        // Check if id is empty
        if id_str.is_empty() {
            return Err(ValidationError::EmptyId);
        }

        // TODO: Add more sophisticated ID format validation if needed
        // For now, we accept any non-empty string as a valid ID

        Ok(())
    }

    /// Validate the externalId attribute (Error 13)
    fn validate_external_id(&self, obj: &serde_json::Map<String, Value>) -> ValidationResult<()> {
        // externalId is optional, so only validate if present
        if let Some(external_id_value) = obj.get("externalId") {
            // If present, it must be a string (null is also acceptable)
            if !external_id_value.is_string() && !external_id_value.is_null() {
                return Err(ValidationError::InvalidExternalId);
            }

            // If it's a string, it should not be empty
            if let Some(external_id_str) = external_id_value.as_str() {
                if external_id_str.is_empty() {
                    return Err(ValidationError::InvalidExternalId);
                }
            }
        }

        Ok(())
    }

    fn validate_meta_attribute(
        &self,
        obj: &serde_json::Map<String, Value>,
    ) -> ValidationResult<()> {
        if let Some(meta_value) = obj.get("meta") {
            let meta_obj = meta_value
                .as_object()
                .ok_or_else(|| ValidationError::InvalidMetaStructure)?;

            // Validate resourceType if present
            if let Some(resource_type) = meta_obj.get("resourceType") {
                let resource_type_str = resource_type
                    .as_str()
                    .ok_or_else(|| ValidationError::InvalidMetaStructure)?;

                if resource_type_str.is_empty() {
                    return Err(ValidationError::MissingResourceType);
                }

                // Validate that resourceType matches expected values
                if !["User", "Group"].contains(&resource_type_str) {
                    return Err(ValidationError::InvalidResourceType {
                        resource_type: resource_type_str.to_string(),
                    });
                }
            }

            // Validate datetime fields
            for field in &["created", "lastModified"] {
                if let Some(datetime_value) = meta_obj.get(*field) {
                    if !datetime_value.is_string() {
                        match *field {
                            "created" => return Err(ValidationError::InvalidCreatedDateTime),
                            "lastModified" => return Err(ValidationError::InvalidModifiedDateTime),
                            _ => return Err(ValidationError::InvalidMetaStructure),
                        }
                    }
                    // TODO: Add RFC3339 datetime format validation
                    // For now, we just check that it's a string
                }
            }

            // Validate location URI
            if let Some(location_value) = meta_obj.get("location") {
                if !location_value.is_string() {
                    return Err(ValidationError::InvalidLocationUri);
                }
                // TODO: Add URI format validation
                // For now, we just check that it's a string
            }

            // Validate version
            if let Some(version_value) = meta_obj.get("version") {
                if !version_value.is_string() {
                    return Err(ValidationError::InvalidVersionFormat);
                }
            }
        }

        Ok(())
    }

    /// Extract schema URIs from the schemas array.
    fn extract_schema_uris(
        &self,
        obj: &serde_json::Map<String, Value>,
    ) -> ValidationResult<Vec<String>> {
        let schemas_value = obj
            .get("schemas")
            .ok_or_else(|| ValidationError::MissingSchemas)?;

        let schemas_array = schemas_value
            .as_array()
            .ok_or_else(|| ValidationError::InvalidMetaStructure)?;

        let mut uris = Vec::new();
        for schema_value in schemas_array {
            let uri = schema_value
                .as_str()
                .ok_or_else(|| ValidationError::InvalidMetaStructure)?;
            uris.push(uri.to_string());
        }

        Ok(uris)
    }

    /// Check if a schema URI has valid format.
    fn is_valid_schema_uri(&self, uri: &str) -> bool {
        // Basic validation: must be a URN that starts with correct prefix
        uri.starts_with("urn:") && uri.contains("scim:schemas")
    }

    /// Validate schema URI combinations (base vs extension schemas).
    fn validate_schema_combinations(
        &self,
        uris: &std::collections::HashSet<&str>,
    ) -> ValidationResult<()> {
        let has_user_base = uris.contains("urn:ietf:params:scim:schemas:core:2.0:User");
        let has_group_base = uris.contains("urn:ietf:params:scim:schemas:core:2.0:Group");

        let user_extensions: Vec<_> = uris
            .iter()
            .filter(|uri| uri.contains("extension") && uri.contains("User"))
            .collect();

        let group_extensions: Vec<_> = uris
            .iter()
            .filter(|uri| uri.contains("extension") && uri.contains("Group"))
            .collect();

        // If there are User extensions, there must be a User base schema
        if !user_extensions.is_empty() && !has_user_base {
            return Err(ValidationError::ExtensionWithoutBase);
        }

        // If there are Group extensions, there must be a Group base schema
        if !group_extensions.is_empty() && !has_group_base {
            return Err(ValidationError::ExtensionWithoutBase);
        }

        // Cannot have both User and Group schemas in the same resource
        if has_user_base && has_group_base {
            return Err(ValidationError::InvalidMetaStructure);
        }

        Ok(())
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

        assert!(
            registry
                .validate_resource(&registry.core_user_schema, &user)
                .is_ok()
        );
    }

    #[test]
    fn test_missing_required_attribute() {
        let registry = SchemaRegistry::new().expect("Failed to create registry");
        let user = json!({
            "displayName": "Test User"
            // Missing required userName
        });

        let result = registry.validate_resource(&registry.core_user_schema, &user);
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

        let result = registry.validate_resource(&registry.core_user_schema, &user);
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

        let result = registry.validate_resource(&registry.core_user_schema, &user);
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

        assert!(
            registry
                .validate_resource(&registry.core_user_schema, &user)
                .is_ok()
        );
    }

    #[test]
    fn test_id_validation() {
        let registry = SchemaRegistry::new().expect("Failed to create registry");

        // Test valid resource with ID
        let valid_user = json!({
            "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
            "id": "12345",
            "userName": "testuser@example.com",
            "meta": {
                "resourceType": "User"
            }
        });

        assert!(registry.validate_scim_resource(&valid_user).is_ok());

        // Test missing ID
        let missing_id_user = json!({
            "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
            "userName": "testuser@example.com",
            "meta": {
                "resourceType": "User"
            }
        });

        match registry.validate_scim_resource(&missing_id_user) {
            Err(ValidationError::MissingId) => {
                // Expected error
            }
            other => panic!("Expected MissingId error, got {:?}", other),
        }

        // Test empty ID
        let empty_id_user = json!({
            "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
            "id": "",
            "userName": "testuser@example.com",
            "meta": {
                "resourceType": "User"
            }
        });

        match registry.validate_scim_resource(&empty_id_user) {
            Err(ValidationError::EmptyId) => {
                // Expected error
            }
            other => panic!("Expected EmptyId error, got {:?}", other),
        }

        // Test invalid ID type
        let invalid_id_user = json!({
            "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
            "id": 12345,
            "userName": "testuser@example.com",
            "meta": {
                "resourceType": "User"
            }
        });

        match registry.validate_scim_resource(&invalid_id_user) {
            Err(ValidationError::InvalidIdFormat { .. }) => {
                // Expected error
            }
            other => panic!("Expected InvalidIdFormat error, got {:?}", other),
        }
    }

    #[test]
    fn test_external_id_validation() {
        let registry = SchemaRegistry::new().expect("Failed to create registry");

        // Test valid external ID
        let valid_user = json!({
            "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
            "id": "12345",
            "userName": "testuser@example.com",
            "externalId": "ext123",
            "meta": {
                "resourceType": "User"
            }
        });

        assert!(registry.validate_scim_resource(&valid_user).is_ok());

        // Test invalid external ID type
        let invalid_external_id_user = json!({
            "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
            "id": "12345",
            "userName": "testuser@example.com",
            "externalId": 999,
            "meta": {
                "resourceType": "User"
            }
        });

        match registry.validate_scim_resource(&invalid_external_id_user) {
            Err(ValidationError::InvalidExternalId) => {
                // Expected error
            }
            other => panic!("Expected InvalidExternalId error, got {:?}", other),
        }

        // Test empty external ID
        let empty_external_id_user = json!({
            "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
            "id": "12345",
            "userName": "testuser@example.com",
            "externalId": "",
            "meta": {
                "resourceType": "User"
            }
        });

        match registry.validate_scim_resource(&empty_external_id_user) {
            Err(ValidationError::InvalidExternalId) => {
                // Expected error
            }
            other => panic!("Expected InvalidExternalId error, got {:?}", other),
        }
    }

    #[test]
    fn test_phase_2_integration() {
        let registry = SchemaRegistry::new().expect("Failed to create registry");

        // Test that Phase 2 validation is actually being called in the main validation flow

        // Test 1: Comprehensive valid resource passes all Phase 2 validations
        let valid_user = json!({
            "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
            "id": "valid-id-123",
            "userName": "testuser@example.com",
            "externalId": "ext-valid-123",
            "meta": {
                "resourceType": "User",
                "created": "2023-01-01T00:00:00Z",
                "lastModified": "2023-01-01T00:00:00Z",
                "location": "https://example.com/Users/valid-id-123",
                "version": "v1.0"
            }
        });

        assert!(
            registry.validate_scim_resource(&valid_user).is_ok(),
            "Valid user should pass all Phase 2 validations"
        );

        // Test 2: Multiple Phase 2 errors are caught correctly
        let invalid_user_missing_id = json!({
            "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
            // Missing id
            "userName": "testuser@example.com",
            "meta": {
                "resourceType": "User"
            }
        });

        match registry.validate_scim_resource(&invalid_user_missing_id) {
            Err(ValidationError::MissingId) => {
                // Expected - ID validation caught the missing ID
            }
            other => panic!(
                "Expected MissingId error from Phase 2 validation, got {:?}",
                other
            ),
        }

        // Test 3: External ID validation is integrated
        let invalid_external_id = json!({
            "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
            "id": "valid-id",
            "userName": "testuser@example.com",
            "externalId": false, // Invalid type
            "meta": {
                "resourceType": "User"
            }
        });

        match registry.validate_scim_resource(&invalid_external_id) {
            Err(ValidationError::InvalidExternalId) => {
                // Expected - External ID validation caught the invalid type
            }
            other => panic!(
                "Expected InvalidExternalId error from Phase 2 validation, got {:?}",
                other
            ),
        }

        // Test 4: Meta validation enhancements are working
        let invalid_resource_type = json!({
            "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
            "id": "valid-id",
            "userName": "testuser@example.com",
            "meta": {
                "resourceType": "InvalidType" // Should fail our enhanced validation
            }
        });

        match registry.validate_scim_resource(&invalid_resource_type) {
            Err(ValidationError::InvalidResourceType { resource_type }) => {
                assert_eq!(resource_type, "InvalidType");
            }
            other => panic!(
                "Expected InvalidResourceType error from Phase 2 validation, got {:?}",
                other
            ),
        }

        // Test 5: Validation order - ID validation happens before schema validation
        let missing_id_and_schemas = json!({
            // Missing schemas array AND missing id
            "userName": "testuser@example.com",
            "meta": {
                "resourceType": "User"
            }
        });

        match registry.validate_scim_resource(&missing_id_and_schemas) {
            Err(ValidationError::MissingSchemas) => {
                // Schema validation happens first, so this is expected
            }
            other => panic!(
                "Expected MissingSchemas error (schema validation first), got {:?}",
                other
            ),
        }
    }
}
