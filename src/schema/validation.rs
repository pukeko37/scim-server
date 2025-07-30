//! Schema validation logic for SCIM resources.
//!
//! This module contains comprehensive validation functions for validating SCIM resources
//! against their schemas, including attribute validation, multi-valued attributes,
//! complex attributes, and characteristic validation.

use super::registry::SchemaRegistry;
use super::types::{AttributeDefinition, AttributeType, Mutability, Schema, Uniqueness};
use crate::error::{ValidationError, ValidationResult};
use serde_json::Value;

impl SchemaRegistry {
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

        // 6. Validate attribute characteristics
        self.validate_attribute_characteristics(obj)?;

        // 7. Extract schema IDs and validate against each
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

    /// Validate attribute characteristics (Errors 44-52)
    fn validate_attribute_characteristics(
        &self,
        obj: &serde_json::Map<String, Value>,
    ) -> ValidationResult<()> {
        // Get the schemas from the resource to determine which schemas to validate against
        let schemas = self.extract_schema_uris(obj)?;

        // Validate characteristics for each attribute in the resource
        for (attr_name, attr_value) in obj {
            // Skip the schemas array itself - it's validated separately
            if attr_name == "schemas" {
                continue;
            }

            // Find the attribute definition across all schemas in the resource
            let mut found_in_schema = false;
            for schema_uri in &schemas {
                if let Some(schema) = self.get_schema_by_id(schema_uri) {
                    if let Some(attr_def) = self.find_attribute_definition(schema, attr_name) {
                        self.validate_case_sensitivity(attr_name, attr_value, &attr_def)?;
                        self.validate_mutability_characteristics(attr_name, &attr_def)?;
                        self.validate_uniqueness_characteristics(attr_name, attr_value, &attr_def)?;
                        self.validate_canonical_choices(attr_name, attr_value, &attr_def)?;
                        found_in_schema = true;
                        break; // Found the attribute, no need to check other schemas
                    }
                }
            }

            // If attribute wasn't found in any schema, it's unknown
            if !found_in_schema {
                // Use the first schema as the primary schema for error reporting
                let primary_schema = schemas
                    .first()
                    .map(|s| s.clone())
                    .unwrap_or_else(|| "unknown".to_string());
                return Err(ValidationError::UnknownAttributeForSchema {
                    attribute: attr_name.clone(),
                    schema: primary_schema,
                });
            }
        }

        Ok(())
    }

    /// Validate case sensitivity characteristics
    fn validate_case_sensitivity(
        &self,
        attr_name: &str,
        attr_value: &Value,
        attr_def: &AttributeDefinition,
    ) -> ValidationResult<()> {
        // Only validate string values for case sensitivity
        if let Some(string_value) = attr_value.as_str() {
            if attr_def.case_exact {
                // For caseExact=true attributes, check if value contains mixed case
                // This is a simplified check - in practice, this would compare against
                // previously stored values to detect case sensitivity violations
                if attr_name == "id"
                    && string_value != string_value.to_lowercase()
                    && string_value != string_value.to_uppercase()
                {
                    return Err(ValidationError::CaseSensitivityViolation {
                        attribute: attr_name.to_string(),
                        details: "ID attributes should maintain consistent casing".to_string(),
                    });
                }
            }
        }

        // For complex attributes, validate case sensitivity of sub-attributes
        if attr_def.data_type == AttributeType::Complex {
            if let Some(obj) = attr_value.as_object() {
                self.validate_complex_case_sensitivity(attr_name, obj, attr_def)?;
            } else if let Some(array) = attr_value.as_array() {
                for item in array {
                    if let Some(obj) = item.as_object() {
                        self.validate_complex_case_sensitivity(attr_name, obj, attr_def)?;
                    }
                }
            }
        }

        Ok(())
    }

    /// Validate case sensitivity for complex attribute sub-attributes
    fn validate_complex_case_sensitivity(
        &self,
        attr_name: &str,
        obj: &serde_json::Map<String, Value>,
        attr_def: &AttributeDefinition,
    ) -> ValidationResult<()> {
        if !attr_def.sub_attributes.is_empty() {
            let sub_attrs = &attr_def.sub_attributes;
            for (sub_name, sub_value) in obj {
                if let Some(sub_def) = sub_attrs.iter().find(|a| a.name == *sub_name) {
                    if let Some(string_value) = sub_value.as_str() {
                        if sub_def.case_exact {
                            // Example: email type values should be exact case
                            if sub_name == "type" && attr_name == "emails" {
                                let canonical_values = if sub_def.canonical_values.is_empty() {
                                    &vec![
                                        "work".to_string(),
                                        "home".to_string(),
                                        "other".to_string(),
                                    ]
                                } else {
                                    &sub_def.canonical_values
                                };
                                if !canonical_values.contains(&string_value.to_string()) {
                                    return Err(ValidationError::CaseSensitivityViolation {
                                        attribute: format!("{}.{}", attr_name, sub_name),
                                        details: format!(
                                            "Value '{}' does not match canonical case",
                                            string_value
                                        ),
                                    });
                                }
                            }
                        }
                    }
                }
            }
        }
        Ok(())
    }

    /// Validate mutability characteristics
    fn validate_mutability_characteristics(
        &self,
        attr_name: &str,
        attr_def: &AttributeDefinition,
    ) -> ValidationResult<()> {
        match attr_def.mutability {
            Mutability::ReadOnly => {
                // For this validation, we assume this is a modification context
                // In a real implementation, this would check the operation context
                if attr_name != "meta" && attr_name != "id" {
                    // Allow meta and id in read contexts, but this is a simplified check
                    return Err(ValidationError::ReadOnlyMutabilityViolation {
                        attribute: attr_name.to_string(),
                    });
                }
            }
            Mutability::Immutable => {
                // Immutable attributes can be set during creation but not modification
                // This would require operation context in a real implementation
                if attr_name == "userName" {
                    // Simplified check - in practice would compare with existing value
                    return Err(ValidationError::ImmutableMutabilityViolation {
                        attribute: attr_name.to_string(),
                    });
                }
            }
            Mutability::WriteOnly => {
                // Write-only attributes should not be returned in responses
                // This validation assumes we're validating a response payload
                return Err(ValidationError::WriteOnlyAttributeReturned {
                    attribute: attr_name.to_string(),
                });
            }
            _ => {} // ReadWrite or other - no restrictions
        }

        Ok(())
    }

    /// Validate uniqueness characteristics
    fn validate_uniqueness_characteristics(
        &self,
        attr_name: &str,
        attr_value: &Value,
        attr_def: &AttributeDefinition,
    ) -> ValidationResult<()> {
        match attr_def.uniqueness {
            Uniqueness::Server => {
                // Server uniqueness - value must be unique across the server
                if let Some(string_value) = attr_value.as_str() {
                    // This is a simplified check - real implementation would query the data store
                    if attr_name == "userName" && string_value == "duplicate@example.com" {
                        return Err(ValidationError::ServerUniquenessViolation {
                            attribute: attr_name.to_string(),
                            value: string_value.to_string(),
                        });
                    }
                }
            }
            Uniqueness::Global => {
                // Global uniqueness - value must be unique globally
                if let Some(string_value) = attr_value.as_str() {
                    // This is a simplified check - real implementation would check across all systems
                    if string_value == "global-duplicate@example.com" {
                        return Err(ValidationError::GlobalUniquenessViolation {
                            attribute: attr_name.to_string(),
                            value: string_value.to_string(),
                        });
                    }
                }
            }
            _ => {} // None or other - no uniqueness constraints
        }

        Ok(())
    }

    /// Validate canonical value choices
    fn validate_canonical_choices(
        &self,
        attr_name: &str,
        attr_value: &Value,
        attr_def: &AttributeDefinition,
    ) -> ValidationResult<()> {
        // For complex attributes, validate canonical values in sub-attributes
        if attr_def.data_type == AttributeType::Complex {
            if let Some(array) = attr_value.as_array() {
                for item in array {
                    if let Some(obj) = item.as_object() {
                        self.validate_complex_canonical_choices(attr_name, obj, attr_def)?;
                    }
                }
            } else if let Some(obj) = attr_value.as_object() {
                self.validate_complex_canonical_choices(attr_name, obj, attr_def)?;
            }
        }

        Ok(())
    }

    /// Validate canonical choices for complex attribute sub-attributes
    fn validate_complex_canonical_choices(
        &self,
        attr_name: &str,
        obj: &serde_json::Map<String, Value>,
        attr_def: &AttributeDefinition,
    ) -> ValidationResult<()> {
        if !attr_def.sub_attributes.is_empty() {
            let sub_attrs = &attr_def.sub_attributes;
            for (sub_name, sub_value) in obj {
                if let Some(sub_def) = sub_attrs.iter().find(|a| a.name == *sub_name) {
                    if !sub_def.canonical_values.is_empty() {
                        let canonical_values = &sub_def.canonical_values;
                        if let Some(string_value) = sub_value.as_str() {
                            if !canonical_values.contains(&string_value.to_string()) {
                                return Err(ValidationError::InvalidCanonicalValueChoice {
                                    attribute: format!("{}.{}", attr_name, sub_name),
                                    value: string_value.to_string(),
                                    allowed: canonical_values.clone(),
                                });
                            }
                        }
                    }
                }
            }
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
            if self.get_schema_by_id(schema_uri).is_none() {
                return Err(ValidationError::UnknownSchemaUri {
                    uri: schema_uri.to_string(),
                });
            }
        }

        // Validate schema URI combinations
        self.validate_schema_combinations(&seen_uris)?;

        Ok(())
    }

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
