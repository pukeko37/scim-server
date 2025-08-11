//! Schema validation logic for SCIM resources.
//!
//! This module provides comprehensive validation for SCIM resources using a hybrid approach:
//! - Type-safe validation for core primitives via value objects
//! - Schema-driven validation for complex attributes and business rules
//! - JSON flexibility for extensible attributes

use super::registry::SchemaRegistry;
use super::types::{AttributeDefinition, AttributeType, Uniqueness};
use crate::error::{ValidationError, ValidationResult};
use crate::resource::core::{RequestContext, Resource};
use crate::resource::provider::ResourceProvider;
use crate::resource::value_objects::SchemaUri;
use serde_json::{Map, Value};

/// Operation context for SCIM resource validation.
///
/// Different SCIM operations have different validation requirements:
/// - CREATE: Server generates ID, readonly attributes forbidden
/// - UPDATE: ID required, readonly attributes ignored/forbidden
/// - PATCH: ID required, partial updates allowed
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OperationContext {
    /// Resource creation operation - server generates ID and metadata
    Create,
    /// Resource replacement operation - full resource update
    Update,
    /// Resource modification operation - partial resource update
    Patch,
}

impl SchemaRegistry {
    /// Validate a SCIM resource with async provider integration for uniqueness checks.
    ///
    /// This method performs both synchronous schema validation and async provider-based
    /// uniqueness validation when required by the schema.
    ///
    /// # Arguments
    /// * `resource_type` - The type of resource to validate (e.g., "User", "Group")
    /// * `resource_json` - The JSON resource data to validate
    /// * `context` - The operation context (Create, Update, or Patch)
    /// * `provider` - The resource provider for uniqueness validation
    /// * `request_context` - The request context for tenant/scope information
    ///
    /// # Returns
    /// * `Ok(())` if validation passes
    /// * `Err(ValidationError)` if validation fails
    pub async fn validate_json_resource_with_provider<P>(
        &self,
        resource_type: &str,
        resource_json: &Value,
        context: OperationContext,
        provider: &P,
        request_context: &RequestContext,
    ) -> ValidationResult<()>
    where
        P: ResourceProvider,
    {
        // 1. First perform all synchronous validation
        self.validate_json_resource_with_context(resource_type, resource_json, context)?;

        // 2. Perform async uniqueness validation if needed
        self.validate_uniqueness_constraints(
            resource_type,
            resource_json,
            context,
            provider,
            request_context,
        )
        .await?;

        Ok(())
    }

    /// Validate uniqueness constraints by checking with the provider.
    async fn validate_uniqueness_constraints<P>(
        &self,
        resource_type: &str,
        resource_json: &Value,
        context: OperationContext,
        provider: &P,
        request_context: &RequestContext,
    ) -> ValidationResult<()>
    where
        P: ResourceProvider,
    {
        // Get the schema for this resource type
        let schema = match resource_type {
            "User" => self.get_user_schema(),
            "Group" => self.get_group_schema(),
            _ => return Ok(()), // Unknown resource type, no uniqueness constraints
        };

        // Check each attribute marked as server unique
        for attr in &schema.attributes {
            if attr.uniqueness == Uniqueness::Server {
                if let Some(value) = resource_json.get(&attr.name) {
                    // For updates, exclude the current resource from uniqueness check
                    let exclude_id = match context {
                        OperationContext::Update | OperationContext::Patch => {
                            resource_json.get("id").and_then(|v| v.as_str())
                        }
                        OperationContext::Create => None,
                    };

                    // Check if this value already exists
                    let existing = provider
                        .find_resource_by_attribute(
                            resource_type,
                            &attr.name,
                            value,
                            request_context,
                        )
                        .await
                        .map_err(|e| ValidationError::Custom {
                            message: format!("Failed to check uniqueness: {}", e),
                        })?;

                    if let Some(existing_resource) = existing {
                        // If we found a resource, check if it's the same one (for updates)
                        let is_same_resource = exclude_id
                            .map(|current_id| {
                                existing_resource.id.as_ref().map(|id| id.as_str())
                                    == Some(current_id)
                            })
                            .unwrap_or(false);

                        if !is_same_resource {
                            return Err(ValidationError::ServerUniquenessViolation {
                                attribute: attr.name.clone(),
                                value: value.to_string(),
                            });
                        }
                    }
                }
            }
        }

        Ok(())
    }

    /// Validate a SCIM resource using the hybrid value object approach.
    ///
    /// This method validates both the type-safe core attributes and the
    /// schema-driven complex attributes, providing comprehensive validation
    /// while maintaining performance and flexibility.
    pub fn validate_resource_hybrid(&self, resource: &Resource) -> ValidationResult<()> {
        // 1. Core primitive validation is already done during Resource construction
        // via value objects, so we focus on schema-driven validation

        // 2. Validate against each registered schema
        for schema_uri in &resource.schemas {
            if let Some(schema) = self.get_schema_by_id(schema_uri.as_str()) {
                self.validate_against_schema(resource, schema)?;
            } else {
                return Err(ValidationError::UnknownSchemaUri {
                    uri: schema_uri.as_str().to_string(),
                });
            }
        }

        // 3. Validate schema combinations
        self.validate_schema_combinations(&resource.schemas)?;

        // 4. Validate multi-valued attributes in extended attributes
        self.validate_multi_valued_attributes(&resource.attributes)?;

        // 5. Validate complex attributes in extended attributes
        self.validate_complex_attributes(&resource.attributes)?;

        // 6. Validate attribute characteristics for extended attributes
        self.validate_attribute_characteristics(&resource.attributes)?;

        Ok(())
    }

    /// Validate resource against a specific schema.
    fn validate_against_schema(
        &self,
        resource: &Resource,
        schema: &super::types::Schema,
    ) -> ValidationResult<()> {
        // Convert resource to JSON for schema validation
        let resource_json = resource.to_json()?;

        // Use existing resource validation logic
        self.validate_resource(schema, &resource_json)
    }

    /// Validate a raw JSON resource (legacy support).
    ///
    /// This method first constructs a Resource from JSON (which validates
    /// core primitives) and then performs schema validation.
    /// Validate a SCIM resource with operation context awareness.
    ///
    /// This method performs context-aware validation that varies based on the operation:
    /// - CREATE: Rejects client-provided IDs and readonly attributes
    /// - UPDATE/PATCH: Requires IDs and handles readonly attributes appropriately
    ///
    /// # Arguments
    /// * `resource_type` - The type of resource to validate (e.g., "User", "Group")
    /// * `resource_json` - The JSON resource data to validate
    /// * `context` - The operation context (Create, Update, or Patch)
    ///
    /// # Returns
    /// * `Ok(())` if validation passes
    /// * `Err(ValidationError)` if validation fails with specific error details
    pub fn validate_json_resource_with_context(
        &self,
        resource_type: &str,
        resource_json: &Value,
        context: OperationContext,
    ) -> ValidationResult<()> {
        // First validate schemas are present and not empty
        if let Some(schemas_value) = resource_json.get("schemas") {
            if let Some(schemas_array) = schemas_value.as_array() {
                if schemas_array.is_empty() {
                    return Err(ValidationError::EmptySchemas);
                }

                // Check for duplicate schema URIs
                let mut seen_schemas = std::collections::HashSet::new();
                for schema_value in schemas_array {
                    if let Some(schema_uri) = schema_value.as_str() {
                        if !seen_schemas.insert(schema_uri) {
                            return Err(ValidationError::DuplicateSchemaUri {
                                uri: schema_uri.to_string(),
                            });
                        }
                    }
                }
            } else {
                return Err(ValidationError::MissingSchemas);
            }
        } else {
            return Err(ValidationError::MissingSchemas);
        }

        // Validate meta.resourceType requirement - only if meta object exists
        if let Some(meta_value) = resource_json.get("meta") {
            if let Some(meta_obj) = meta_value.as_object() {
                if !meta_obj.contains_key("resourceType") {
                    return Err(ValidationError::MissingResourceType);
                }
            }
        }

        // Context-aware ID validation
        match context {
            OperationContext::Create => {
                // CREATE: Client should NOT provide ID
                if resource_json
                    .as_object()
                    .map(|obj| obj.contains_key("id"))
                    .unwrap_or(false)
                {
                    return Err(ValidationError::ClientProvidedId);
                }
            }
            OperationContext::Update | OperationContext::Patch => {
                // UPDATE/PATCH: ID is required
                if !resource_json
                    .as_object()
                    .map(|obj| obj.contains_key("id"))
                    .unwrap_or(false)
                {
                    return Err(ValidationError::MissingId);
                }
            }
        }

        // Context-aware readonly attribute validation
        if let Some(meta_value) = resource_json.get("meta") {
            if let Some(meta_obj) = meta_value.as_object() {
                let readonly_fields = ["created", "lastModified", "location", "version"];
                let has_readonly = readonly_fields
                    .iter()
                    .any(|field| meta_obj.contains_key(*field));

                if has_readonly {
                    match context {
                        OperationContext::Create => {
                            // CREATE: Readonly attributes should not be provided by client
                            return Err(ValidationError::ClientProvidedMeta);
                        }
                        OperationContext::Update | OperationContext::Patch => {
                            // UPDATE/PATCH: Readonly attributes are allowed (server will ignore them)
                            // This is compliant with SCIM specification
                        }
                    }
                }
            }
        }

        // Preliminary validation for specific SCIM errors before resource construction
        self.validate_multi_valued_attributes_preliminary(resource_type, resource_json)?;

        // Then convert to Resource (validates core primitives)
        let resource = Resource::from_json(resource_type.to_string(), resource_json.clone())?;

        // Finally validate using hybrid approach
        self.validate_resource_hybrid(&resource)
    }

    /// Map resource type to schema URI.
    fn resource_type_to_schema_uri(resource_type: &str) -> Option<&'static str> {
        match resource_type {
            "User" => Some("urn:ietf:params:scim:schemas:core:2.0:User"),
            "Group" => Some("urn:ietf:params:scim:schemas:core:2.0:Group"),
            _ => None,
        }
    }

    /// Preliminary validation for multi-valued attributes to catch specific SCIM errors
    /// before resource construction.
    fn validate_multi_valued_attributes_preliminary(
        &self,
        resource_type: &str,
        resource_json: &Value,
    ) -> ValidationResult<()> {
        let obj = resource_json
            .as_object()
            .ok_or_else(|| ValidationError::custom("Resource must be a JSON object"))?;

        // Get the schema URI for this resource type
        let schema_uri = Self::resource_type_to_schema_uri(resource_type)
            .ok_or_else(|| ValidationError::custom("Unknown resource type"))?;

        // Get the schema for this resource type
        let schema = self
            .get_schema(schema_uri)
            .ok_or_else(|| ValidationError::custom("Schema not found"))?;

        // Check emails attribute for required sub-attributes
        if let Some(emails_value) = obj.get("emails") {
            if let Some(emails_array) = emails_value.as_array() {
                // Find the emails attribute definition
                if let Some(emails_attr) =
                    schema.attributes.iter().find(|attr| attr.name == "emails")
                {
                    self.validate_required_sub_attributes(emails_attr, emails_array)?;
                }
            }
        }

        // Check other multi-valued attributes if needed
        // Add more preliminary validations here as required

        Ok(())
    }

    /// Validate schema URI combinations.
    fn validate_schema_combinations(&self, schemas: &[SchemaUri]) -> ValidationResult<()> {
        if schemas.is_empty() {
            return Err(ValidationError::MissingSchemas);
        }

        // Check for conflicting schemas (basic validation)
        let schema_strings: Vec<String> = schemas.iter().map(|s| s.as_str().to_string()).collect();

        // Ensure we have at least one core schema
        let has_user_schema = schema_strings.iter().any(|s| s.contains("User"));
        let has_group_schema = schema_strings.iter().any(|s| s.contains("Group"));

        if !has_user_schema && !has_group_schema {
            return Err(ValidationError::custom(
                "Resource must have at least one core schema",
            ));
        }

        // Don't allow both User and Group schemas
        if has_user_schema && has_group_schema {
            return Err(ValidationError::custom(
                "Resource cannot have both User and Group schemas",
            ));
        }

        Ok(())
    }

    /// Validate a resource attribute against its schema definition.
    fn validate_attribute(
        &self,
        attr_def: &AttributeDefinition,
        value: &Value,
    ) -> ValidationResult<()> {
        // Check if attribute is required but missing
        if attr_def.required && value.is_null() {
            return Err(ValidationError::MissingRequiredAttribute {
                attribute: attr_def.name.clone(),
            });
        }

        // Skip validation for null optional attributes
        if value.is_null() {
            return Ok(());
        }

        // Validate data type
        self.validate_attribute_value(attr_def, value)?;

        // Validate mutability if this is an update operation
        // (This would need request context to determine operation type)

        // Validate uniqueness constraints
        // (This would need external data to check uniqueness)

        Ok(())
    }

    /// Validate an attribute value against its expected data type.
    fn validate_attribute_value(
        &self,
        attr_def: &AttributeDefinition,
        value: &Value,
    ) -> ValidationResult<()> {
        self.validate_attribute_value_with_context(attr_def, value, None)
    }

    fn validate_attribute_value_with_context(
        &self,
        attr_def: &AttributeDefinition,
        value: &Value,
        parent_attr: Option<&str>,
    ) -> ValidationResult<()> {
        // Skip validation for null optional attributes
        if value.is_null() && !attr_def.required {
            return Ok(());
        }

        // Check if required attribute is null
        if value.is_null() && attr_def.required {
            return Err(ValidationError::MissingRequiredAttribute {
                attribute: attr_def.name.clone(),
            });
        }

        match attr_def.data_type {
            AttributeType::String => {
                if !value.is_string() {
                    return Err(ValidationError::InvalidAttributeType {
                        attribute: attr_def.name.clone(),
                        expected: "string".to_string(),
                        actual: Self::get_value_type(value).to_string(),
                    });
                }

                // Validate case sensitivity for string attributes
                let str_value = value.as_str().unwrap();
                if attr_def.case_exact {
                    // For case-exact attributes, check for mixed case patterns
                    self.validate_case_exact_string(&attr_def.name, str_value)?;
                }

                // Validate canonical values if defined
                if !attr_def.canonical_values.is_empty() {
                    self.validate_canonical_value_with_context(attr_def, str_value, parent_attr)?;
                }
            }
            AttributeType::Boolean => {
                if !value.is_boolean() {
                    return Err(ValidationError::InvalidAttributeType {
                        attribute: attr_def.name.clone(),
                        expected: "boolean".to_string(),
                        actual: Self::get_value_type(value).to_string(),
                    });
                }
            }
            AttributeType::Decimal => {
                if !value.is_number() {
                    return Err(ValidationError::InvalidAttributeType {
                        attribute: attr_def.name.clone(),
                        expected: "decimal".to_string(),
                        actual: Self::get_value_type(value).to_string(),
                    });
                }
            }
            AttributeType::Integer => {
                if !value.is_i64() {
                    return Err(ValidationError::InvalidAttributeType {
                        attribute: attr_def.name.clone(),
                        expected: "integer".to_string(),
                        actual: Self::get_value_type(value).to_string(),
                    });
                }
            }
            AttributeType::DateTime => {
                if let Some(date_str) = value.as_str() {
                    if !self.is_valid_datetime_format(date_str) {
                        return Err(ValidationError::InvalidDateTimeFormat {
                            attribute: attr_def.name.clone(),
                            value: date_str.to_string(),
                        });
                    }
                } else {
                    return Err(ValidationError::InvalidAttributeType {
                        attribute: attr_def.name.clone(),
                        expected: "string (datetime)".to_string(),
                        actual: Self::get_value_type(value).to_string(),
                    });
                }
            }
            AttributeType::Binary => {
                if let Some(binary_str) = value.as_str() {
                    if !self.is_valid_base64(binary_str) {
                        return Err(ValidationError::InvalidBinaryData {
                            attribute: attr_def.name.clone(),
                            details: "Invalid base64 encoding".to_string(),
                        });
                    }
                } else {
                    return Err(ValidationError::InvalidAttributeType {
                        attribute: attr_def.name.clone(),
                        expected: "string (base64)".to_string(),
                        actual: Self::get_value_type(value).to_string(),
                    });
                }
            }
            AttributeType::Reference => {
                if let Some(ref_str) = value.as_str() {
                    if !self.is_valid_uri_format(ref_str) {
                        return Err(ValidationError::InvalidReferenceUri {
                            attribute: attr_def.name.clone(),
                            uri: ref_str.to_string(),
                        });
                    }
                } else {
                    return Err(ValidationError::InvalidAttributeType {
                        attribute: attr_def.name.clone(),
                        expected: "string (URI)".to_string(),
                        actual: Self::get_value_type(value).to_string(),
                    });
                }
            }
            AttributeType::Complex => {
                if value.is_array() {
                    // Multi-valued complex attribute
                    self.validate_multi_valued_array(attr_def, value)?;
                } else if value.is_object() {
                    // Single complex attribute
                    self.validate_complex_attribute_structure(attr_def, value)?;
                } else {
                    return Err(ValidationError::InvalidAttributeType {
                        attribute: attr_def.name.clone(),
                        expected: "object or array".to_string(),
                        actual: Self::get_value_type(value).to_string(),
                    });
                }
            }
        }

        Ok(())
    }

    /// Validate multi-valued attributes in the resource.
    fn validate_multi_valued_attributes(
        &self,
        attributes: &Map<String, Value>,
    ) -> ValidationResult<()> {
        for (attr_name, attr_value) in attributes {
            if let Some(_attr_value_array) = attr_value.as_array() {
                // Find attribute definition
                if let Some(attr_def) = self.get_complex_attribute_definition(attr_name) {
                    if attr_def.multi_valued {
                        self.validate_multi_valued_array(attr_def, attr_value)?;
                    }
                }
            }
        }
        Ok(())
    }

    /// Validate a multi-valued attribute array.
    fn validate_multi_valued_array(
        &self,
        attr_def: &AttributeDefinition,
        value: &Value,
    ) -> ValidationResult<()> {
        let array = value
            .as_array()
            .ok_or_else(|| ValidationError::InvalidAttributeType {
                attribute: attr_def.name.clone(),
                expected: "array".to_string(),
                actual: Self::get_value_type(value).to_string(),
            })?;

        for item in array {
            match attr_def.data_type {
                AttributeType::Complex => {
                    self.validate_complex_attribute_structure(attr_def, item)?;
                }
                _ => {
                    self.validate_attribute_value(attr_def, item)?;
                }
            }
        }

        // Validate required sub-attributes for complex multi-valued attributes
        if matches!(attr_def.data_type, AttributeType::Complex) {
            self.validate_required_sub_attributes(attr_def, array)?;

            // Check for multiple primary values
            let primary_count = array
                .iter()
                .filter(|item| {
                    item.get("primary")
                        .and_then(|p| p.as_bool())
                        .unwrap_or(false)
                })
                .count();

            if primary_count > 1 {
                return Err(ValidationError::MultiplePrimaryValues {
                    attribute: attr_def.name.clone(),
                });
            }
        }

        Ok(())
    }

    /// Validate required sub-attributes in multi-valued complex attributes.
    fn validate_required_sub_attributes(
        &self,
        attr_def: &AttributeDefinition,
        array: &[Value],
    ) -> ValidationResult<()> {
        for item in array {
            if let Some(obj) = item.as_object() {
                for sub_attr in &attr_def.sub_attributes {
                    if sub_attr.required && !obj.contains_key(&sub_attr.name) {
                        return Err(ValidationError::MissingRequiredSubAttribute {
                            attribute: attr_def.name.clone(),
                            sub_attribute: sub_attr.name.clone(),
                        });
                    }
                }
            }
        }
        Ok(())
    }

    /// Validate complex attributes in the resource.
    fn validate_complex_attributes(&self, attributes: &Map<String, Value>) -> ValidationResult<()> {
        for (attr_name, attr_value) in attributes {
            if let Some(attr_def) = self.get_complex_attribute_definition(attr_name) {
                if matches!(attr_def.data_type, AttributeType::Complex) {
                    if attr_def.multi_valued {
                        // Multi-valued complex attribute (handled elsewhere)
                        continue;
                    } else {
                        // Single complex attribute
                        self.validate_complex_attribute_structure(attr_def, attr_value)?;
                    }
                }
            }
        }
        Ok(())
    }

    /// Validate the structure of a complex attribute.
    fn validate_complex_attribute_structure(
        &self,
        attr_def: &AttributeDefinition,
        value: &Value,
    ) -> ValidationResult<()> {
        let obj = value
            .as_object()
            .ok_or_else(|| ValidationError::InvalidAttributeType {
                attribute: attr_def.name.clone(),
                expected: "object".to_string(),
                actual: Self::get_value_type(value).to_string(),
            })?;

        // Validate known sub-attributes
        self.validate_known_sub_attributes(attr_def, obj)?;

        // Validate sub-attribute types
        self.validate_sub_attribute_types(attr_def, obj)?;

        // Validate no nested complex attributes
        self.validate_no_nested_complex(attr_def, obj)?;

        // Validate required sub-attributes
        self.validate_required_sub_attributes_complex(attr_def, obj)?;

        Ok(())
    }

    /// Validate that only known sub-attributes are present.
    fn validate_known_sub_attributes(
        &self,
        attr_def: &AttributeDefinition,
        obj: &Map<String, Value>,
    ) -> ValidationResult<()> {
        let known_sub_attrs: Vec<&str> = attr_def
            .sub_attributes
            .iter()
            .map(|sa| sa.name.as_str())
            .collect();

        for key in obj.keys() {
            if !known_sub_attrs.contains(&key.as_str()) {
                return Err(ValidationError::UnknownSubAttribute {
                    attribute: attr_def.name.clone(),
                    sub_attribute: key.clone(),
                });
            }
        }

        Ok(())
    }

    /// Validate sub-attribute data types.
    fn validate_sub_attribute_types(
        &self,
        attr_def: &AttributeDefinition,
        obj: &Map<String, Value>,
    ) -> ValidationResult<()> {
        for (key, value) in obj {
            if let Some(sub_attr_def) = attr_def.sub_attributes.iter().find(|sa| sa.name == *key) {
                self.validate_attribute_value_with_context(
                    sub_attr_def,
                    value,
                    Some(&attr_def.name),
                )?;
            }
        }
        Ok(())
    }

    /// Validate that complex attributes don't contain nested complex attributes.
    fn validate_no_nested_complex(
        &self,
        attr_def: &AttributeDefinition,
        _obj: &Map<String, Value>,
    ) -> ValidationResult<()> {
        for sub_attr in &attr_def.sub_attributes {
            if matches!(sub_attr.data_type, AttributeType::Complex) {
                return Err(ValidationError::NestedComplexAttributes {
                    attribute: format!("{}.{}", attr_def.name, sub_attr.name),
                });
            }
        }
        Ok(())
    }

    /// Validate required sub-attributes in complex attributes.
    fn validate_required_sub_attributes_complex(
        &self,
        attr_def: &AttributeDefinition,
        obj: &Map<String, Value>,
    ) -> ValidationResult<()> {
        for sub_attr in &attr_def.sub_attributes {
            if sub_attr.required && !obj.contains_key(&sub_attr.name) {
                return Err(ValidationError::MissingRequiredSubAttribute {
                    attribute: attr_def.name.clone(),
                    sub_attribute: sub_attr.name.clone(),
                });
            }
        }
        Ok(())
    }

    /// Validate attribute characteristics (mutability, case sensitivity, etc.).
    fn validate_attribute_characteristics(
        &self,
        attributes: &Map<String, Value>,
    ) -> ValidationResult<()> {
        // This would typically require request context to determine operation type
        // For now, we'll implement basic characteristic validation

        for (attr_name, attr_value) in attributes {
            // Validate case sensitivity for attributes that require it
            self.validate_case_sensitivity(attr_name, attr_value)?;

            // Additional characteristic validation can be added here
        }

        Ok(())
    }

    /// Validate case sensitivity requirements for attributes.
    fn validate_case_sensitivity(
        &self,
        attr_name: &str,
        attr_value: &Value,
    ) -> ValidationResult<()> {
        // Special validation for resourceType data type
        if attr_name == "resourceType" && !attr_value.is_string() {
            return Err(ValidationError::InvalidMetaStructure);
        }

        // Find attribute definition to check case sensitivity
        if let Some(attr_def) = self
            .get_user_schema()
            .attributes
            .iter()
            .find(|attr| attr.name == attr_name)
        {
            if attr_def.case_exact && attr_value.is_string() {
                let str_value = attr_value.as_str().unwrap();
                self.validate_case_exact_string(attr_name, str_value)?;
            }
        }

        // Validate case sensitivity for complex attributes
        if attr_value.is_array() {
            if let Some(array) = attr_value.as_array() {
                for item in array {
                    self.validate_complex_case_sensitivity(attr_name, item)?;
                }
            }
        }

        Ok(())
    }

    /// Validate that a case-exact string follows proper casing rules.
    fn validate_case_exact_string(&self, attr_name: &str, value: &str) -> ValidationResult<()> {
        // Special handling for resourceType - validate against allowed values first
        if attr_name == "resourceType" {
            let allowed_types = ["User", "Group"];
            if !allowed_types.contains(&value) {
                return Err(ValidationError::InvalidResourceType {
                    resource_type: value.to_string(),
                });
            }
            return Ok(());
        }

        // For SCIM, case-exact typically means consistent casing
        // Check for problematic mixed case patterns that suggest inconsistency
        if self.has_inconsistent_casing(value) {
            return Err(ValidationError::CaseSensitivityViolation {
                attribute: attr_name.to_string(),
                details: format!(
                    "Attribute '{}' requires consistent casing but found mixed case in '{}'",
                    attr_name, value
                ),
            });
        }
        Ok(())
    }

    /// Check if a string has inconsistent casing patterns.
    fn has_inconsistent_casing(&self, value: &str) -> bool {
        // For ID attributes, mixed case like "MixedCase123" is problematic
        // This is a heuristic - in practice this would be configurable
        if value.len() > 1 {
            let has_upper = value.chars().any(|c| c.is_uppercase());
            let has_lower = value.chars().any(|c| c.is_lowercase());

            // If it has both upper and lower case letters, it's mixed case
            if has_upper && has_lower {
                // Allow common patterns like camelCase or PascalCase
                // But flag obvious mixed patterns
                let first_char = value.chars().next().unwrap();
                let rest = &value[1..];

                // Simple heuristic: if first char is uppercase and rest has mixed case
                // or if it looks like random mixed case, flag it
                if first_char.is_uppercase()
                    && rest.chars().any(|c| c.is_uppercase())
                    && rest.chars().any(|c| c.is_lowercase())
                {
                    return true;
                }
            }
        }
        false
    }

    /// Validate canonical values considering case sensitivity.

    fn validate_canonical_value_with_context(
        &self,
        attr_def: &AttributeDefinition,
        value: &str,
        parent_attr: Option<&str>,
    ) -> ValidationResult<()> {
        // For SCIM 2.0, canonical values must match exactly as defined in the schema
        // regardless of the caseExact setting. The caseExact setting affects how
        // the server handles submitted values for storage/comparison, but canonical
        // values are predefined constants that must be matched exactly.
        if !attr_def.canonical_values.contains(&value.to_string()) {
            let attribute_name = if let Some(parent) = parent_attr {
                format!("{}.{}", parent, attr_def.name)
            } else {
                attr_def.name.clone()
            };

            return Err(ValidationError::InvalidCanonicalValue {
                attribute: attribute_name,
                value: value.to_string(),
                allowed: attr_def.canonical_values.clone(),
            });
        }
        Ok(())
    }

    /// Validate case sensitivity for complex multi-valued attributes.
    fn validate_complex_case_sensitivity(
        &self,
        attr_name: &str,
        value: &Value,
    ) -> ValidationResult<()> {
        if let Some(obj) = value.as_object() {
            if let Some(attr_def) = self.get_complex_attribute_definition(attr_name) {
                for (sub_attr_name, sub_attr_value) in obj {
                    if let Some(sub_attr_def) = attr_def
                        .sub_attributes
                        .iter()
                        .find(|sa| sa.name == *sub_attr_name)
                    {
                        if !sub_attr_def.case_exact && sub_attr_value.is_string() {
                            // Case-insensitive validation/normalization
                            // Implementation would depend on specific requirements
                        }
                    }
                }
            }
        }
        Ok(())
    }

    /// Validate a resource against a specific schema (legacy method).
    ///
    /// This method validates a JSON resource against a schema definition,
    /// checking attributes, types, and constraints.
    pub fn validate_resource(
        &self,
        schema: &super::types::Schema,
        resource: &Value,
    ) -> ValidationResult<()> {
        let obj = resource
            .as_object()
            .ok_or_else(|| ValidationError::custom("Resource must be a JSON object"))?;

        // Validate each defined attribute in the schema
        for attr_def in &schema.attributes {
            if let Some(value) = obj.get(&attr_def.name) {
                self.validate_attribute(attr_def, value)?;
            } else if attr_def.required {
                return Err(ValidationError::MissingRequiredAttribute {
                    attribute: attr_def.name.clone(),
                });
            }
        }

        // Check for unknown attributes (strict validation)
        for (field_name, _) in obj {
            if !schema
                .attributes
                .iter()
                .any(|attr| attr.name == *field_name)
            {
                // Allow standard SCIM attributes
                if !["schemas", "id", "externalId", "meta"].contains(&field_name.as_str()) {
                    return Err(ValidationError::UnknownAttributeForSchema {
                        attribute: field_name.clone(),
                        schema: schema.id.clone(),
                    });
                }
            }
        }

        Ok(())
    }
}
