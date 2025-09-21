# Schema System Architecture

This deep dive explores the schema system architecture in SCIM Server, covering how schemas are registered, validated, and extended, plus patterns for creating custom value objects and integrating with dynamic schema requirements.

## Overview

The schema system in SCIM Server provides the foundation for data validation, serialization, and extensibility. It combines SCIM 2.0 compliance with flexible extension mechanisms, allowing you to maintain standards compliance while supporting custom attributes and resource types.

**Core Schema Flow:**
```text
Schema Definition → Registration → Validation → Value Object Creation → 
Extension Support → Dynamic Schema Discovery
```

## Schema System Architecture Overview

```text
┌─────────────────────────────────────────────────────────────────────────────┐
│ Schema Registry                                                             │
│                                                                             │
│ ┌─────────────────┐ ┌─────────────────┐ ┌─────────────────────────────────┐ │
│ │ Core SCIM       │ │ Extension       │ │ Custom Resource                 │ │
│ │ Schemas         │ │ Schemas         │ │ Schemas                         │ │
│ │                 │ │                 │ │                                 │ │
│ │ • User          │ │ • Enterprise    │ │ • Organization                  │ │
│ │ • Group         │ │   User          │ │ • Application                   │ │
│ │ • Schema        │ │ • Custom attrs  │ │ • Custom types                  │ │
│ │ • ServiceConfig │ │ • Tenant exts   │ │ • Domain specific               │ │
│ └─────────────────┘ └─────────────────┘ └─────────────────────────────────┘ │
└─────────────────────────────────────────────────────────────────────────────┘
                                    ↓
┌─────────────────────────────────────────────────────────────────────────────┐
│ Value Object System                                                         │
│                                                                             │
│ ┌─────────────────┐ ┌─────────────────┐ ┌─────────────────────────────────┐ │
│ │ Static Value    │ │ Dynamic Value   │ │ Custom Value                    │ │
│ │ Objects         │ │ Objects         │ │ Objects                         │ │
│ │                 │ │                 │ │                                 │ │
│ │ • Compile-time  │ │ • Runtime       │ │ • Domain-specific validation    │ │
│ │   validation    │ │   creation      │ │ • Complex business rules        │ │
│ │ • Type safety   │ │ • Schema-driven │ │ • Custom serialization          │ │
│ │ • Performance   │ │ • Flexible      │ │ • Integration adapters          │ │
│ └─────────────────┘ └─────────────────┘ └─────────────────────────────────┘ │
└─────────────────────────────────────────────────────────────────────────────┘
                                    ↓
┌─────────────────────────────────────────────────────────────────────────────┐
│ Validation & Extension Engine                                               │
│ • Schema validation • Extension loading • Dynamic discovery • Caching      │
└─────────────────────────────────────────────────────────────────────────────┘
```

## Schema Registry Architecture

### Core Schema Registry

```rust
use scim_server::schema::{Schema, SchemaRegistry, AttributeDefinition, AttributeType};
use serde_json::{Value, json};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

pub struct ExtendedSchemaRegistry {
    core_registry: SchemaRegistry,
    extension_schemas: RwLock<HashMap<String, Schema>>,
    custom_validators: RwLock<HashMap<String, Box<dyn CustomValidator>>>,
    schema_cache: RwLock<HashMap<String, CachedSchema>>,
    tenant_extensions: RwLock<HashMap<String, Vec<String>>>,
}

#[derive(Clone)]
struct CachedSchema {
    schema: Schema,
    cached_at: std::time::Instant,
    compiled_validator: Option<CompiledValidator>,
}

pub trait CustomValidator: Send + Sync {
    fn validate(&self, value: &Value, context: &ValidationContext) -> ValidationResult<()>;
    fn attribute_type(&self) -> AttributeType;
}

impl ExtendedSchemaRegistry {
    pub fn new() -> Result<Self, SchemaError> {
        let core_registry = SchemaRegistry::default();
        
        Ok(Self {
            core_registry,
            extension_schemas: RwLock::new(HashMap::new()),
            custom_validators: RwLock::new(HashMap::new()),
            schema_cache: RwLock::new(HashMap::new()),
            tenant_extensions: RwLock::new(HashMap::new()),
        })
    }
    
    pub fn register_extension_schema(&self, schema: Schema) -> Result<(), SchemaError> {
        let schema_id = schema.id().to_string();
        
        // Validate schema format
        self.validate_schema_format(&schema)?;
        
        // Check for conflicts with existing schemas
        self.check_schema_conflicts(&schema)?;
        
        // Register the schema
        let mut extensions = self.extension_schemas.write().unwrap();
        extensions.insert(schema_id.clone(), schema.clone());
        
        // Clear related caches
        self.invalidate_cache(&schema_id);
        
        // Notify listeners of schema registration
        self.notify_schema_registered(&schema);
        
        Ok(())
    }
    
    pub fn register_custom_validator<V: CustomValidator + 'static>(
        &self,
        attribute_name: &str,
        validator: V,
    ) -> Result<(), SchemaError> {
        let mut validators = self.custom_validators.write().unwrap();
        validators.insert(attribute_name.to_string(), Box::new(validator));
        Ok(())
    }
    
    pub fn add_tenant_extension(
        &self,
        tenant_id: &str,
        schema_id: &str,
    ) -> Result<(), SchemaError> {
        // Verify schema exists
        if !self.schema_exists(schema_id)? {
            return Err(SchemaError::SchemaNotFound(schema_id.to_string()));
        }
        
        let mut tenant_extensions = self.tenant_extensions.write().unwrap();
        tenant_extensions.entry(tenant_id.to_string())
            .or_insert_with(Vec::new)
            .push(schema_id.to_string());
            
        Ok(())
    }
    
    pub fn get_effective_schema(
        &self,
        base_schema_id: &str,
        tenant_id: Option<&str>,
    ) -> Result<CompositeSchema, SchemaError> {
        let cache_key = format!("{}:{}", base_schema_id, tenant_id.unwrap_or("global"));
        
        // Check cache first
        {
            let cache = self.schema_cache.read().unwrap();
            if let Some(cached) = cache.get(&cache_key) {
                if cached.cached_at.elapsed() < std::time::Duration::from_mins(5) {
                    return Ok(CompositeSchema::from_cached(&cached.schema));
                }
            }
        }
        
        // Build composite schema
        let mut composite = CompositeSchema::new();
        
        // Add base schema
        let base_schema = self.core_registry.get_schema(base_schema_id)?;
        composite.add_schema(base_schema.clone());
        
        // Add tenant-specific extensions
        if let Some(tenant_id) = tenant_id {
            let tenant_extensions = self.tenant_extensions.read().unwrap();
            if let Some(extension_ids) = tenant_extensions.get(tenant_id) {
                for extension_id in extension_ids {
                    let extension_schema = self.get_extension_schema(extension_id)?;
                    composite.add_extension(extension_schema);
                }
            }
        }
        
        // Cache the result
        let compiled_schema = composite.compile()?;
        {
            let mut cache = self.schema_cache.write().unwrap();
            cache.insert(cache_key, CachedSchema {
                schema: compiled_schema.clone(),
                cached_at: std::time::Instant::now(),
                compiled_validator: Some(compiled_schema.create_validator()?),
            });
        }
        
        Ok(composite)
    }
    
    pub fn validate_resource(
        &self,
        resource_type: &str,
        resource_data: &Value,
        tenant_id: Option<&str>,
    ) -> ValidationResult<ValidatedResource> {
        let composite_schema = self.get_effective_schema(resource_type, tenant_id)?;
        
        // Perform comprehensive validation
        let validation_context = ValidationContext {
            resource_type: resource_type.to_string(),
            tenant_id: tenant_id.map(|s| s.to_string()),
            schema_registry: self,
            custom_context: HashMap::new(),
        };
        
        composite_schema.validate(resource_data, &validation_context)
    }
    
    fn validate_schema_format(&self, schema: &Schema) -> Result<(), SchemaError> {
        // Validate required fields
        if schema.id().is_empty() {
            return Err(SchemaError::InvalidSchema("Schema ID cannot be empty".into()));
        }
        
        if schema.name().is_empty() {
            return Err(SchemaError::InvalidSchema("Schema name cannot be empty".into()));
        }
        
        // Validate attributes
        for attribute in schema.attributes() {
            self.validate_attribute_definition(attribute)?;
        }
        
        Ok(())
    }
    
    fn validate_attribute_definition(&self, attr: &AttributeDefinition) -> Result<(), SchemaError> {
        // Check attribute name format
        if !attr.name().chars().all(|c| c.is_alphanumeric() || c == '_' || c == '.') {
            return Err(SchemaError::InvalidAttributeName(attr.name().to_string()));
        }
        
        // Validate attribute type consistency
        match attr.attribute_type() {
            AttributeType::Complex => {
                if attr.sub_attributes().is_empty() {
                    return Err(SchemaError::InvalidSchema(
                        format!("Complex attribute '{}' must have sub-attributes", attr.name())
                    ));
                }
                
                // Recursively validate sub-attributes
                for sub_attr in attr.sub_attributes() {
                    self.validate_attribute_definition(sub_attr)?;
                }
            },
            AttributeType::Reference => {
                if attr.reference_types().is_empty() {
                    return Err(SchemaError::InvalidSchema(
                        format!("Reference attribute '{}' must specify reference types", attr.name())
                    ));
                }
            },
            _ => {} // Other types are valid as-is
        }
        
        Ok(())
    }
    
    fn check_schema_conflicts(&self, new_schema: &Schema) -> Result<(), SchemaError> {
        // Check for ID conflicts
        if self.core_registry.has_schema(new_schema.id())? {
            return Err(SchemaError::SchemaConflict(
                format!("Schema ID '{}' already exists in core registry", new_schema.id())
            ));
        }
        
        let extensions = self.extension_schemas.read().unwrap();
        if extensions.contains_key(new_schema.id()) {
            return Err(SchemaError::SchemaConflict(
                format!("Schema ID '{}' already exists in extensions", new_schema.id())
            ));
        }
        
        // Check for attribute conflicts in same namespace
        self.check_attribute_conflicts(new_schema)?;
        
        Ok(())
    }
    
    fn check_attribute_conflicts(&self, schema: &Schema) -> Result<(), SchemaError> {
        let mut seen_attributes = HashMap::new();
        
        for attribute in schema.attributes() {
            let attr_name = attribute.name();
            if let Some(existing_type) = seen_attributes.get(attr_name) {
                if existing_type != &attribute.attribute_type() {
                    return Err(SchemaError::AttributeConflict(
                        format!(
                            "Attribute '{}' defined with conflicting types: {:?} vs {:?}",
                            attr_name, existing_type, attribute.attribute_type()
                        )
                    ));
                }
            }
            seen_attributes.insert(attr_name.to_string(), attribute.attribute_type());
        }
        
        Ok(())
    }
}

#[derive(Debug)]
pub struct CompositeSchema {
    base_schema: Option<Schema>,
    extensions: Vec<Schema>,
    merged_attributes: HashMap<String, AttributeDefinition>,
}

impl CompositeSchema {
    pub fn new() -> Self {
        Self {
            base_schema: None,
            extensions: Vec::new(),
            merged_attributes: HashMap::new(),
        }
    }
    
    pub fn add_schema(&mut self, schema: Schema) {
        self.base_schema = Some(schema);
        self.rebuild_attribute_map();
    }
    
    pub fn add_extension(&mut self, extension: Schema) {
        self.extensions.push(extension);
        self.rebuild_attribute_map();
    }
    
    fn rebuild_attribute_map(&mut self) {
        self.merged_attributes.clear();
        
        // Add base schema attributes
        if let Some(ref base) = self.base_schema {
            for attr in base.attributes() {
                self.merged_attributes.insert(attr.name().to_string(), attr.clone());
            }
        }
        
        // Add extension attributes (extensions can override base attributes)
        for extension in &self.extensions {
            for attr in extension.attributes() {
                self.merged_attributes.insert(attr.name().to_string(), attr.clone());
            }
        }
    }
    
    pub fn validate(
        &self,
        data: &Value,
        context: &ValidationContext,
    ) -> ValidationResult<ValidatedResource> {
        let mut validated_resource = ValidatedResource::new();
        
        // Validate each field in the data
        if let Value::Object(obj) = data {
            for (field_name, field_value) in obj {
                if let Some(attr_def) = self.merged_attributes.get(field_name) {
                    let validated_value = self.validate_attribute(
                        attr_def,
                        field_value,
                        context,
                    )?;
                    validated_resource.add_attribute(field_name.clone(), validated_value);
                } else if !self.is_core_attribute(field_name) {
                    return Err(ValidationError::UnknownAttribute(field_name.clone()));
                }
            }
        }
        
        // Check required attributes
        self.validate_required_attributes(data, context)?;
        
        Ok(validated_resource)
    }
    
    fn validate_attribute(
        &self,
        attr_def: &AttributeDefinition,
        value: &Value,
        context: &ValidationContext,
    ) -> ValidationResult<ValidatedValue> {
        // Check multiplicity
        if attr_def.is_multi_valued() {
            if !value.is_array() {
                return Err(ValidationError::InvalidType {
                    attribute: attr_def.name().to_string(),
                    expected: "array".to_string(),
                    actual: value_type_name(value).to_string(),
                });
            }
            
            let array = value.as_array().unwrap();
            let mut validated_items = Vec::new();
            
            for item in array {
                validated_items.push(self.validate_single_value(attr_def, item, context)?);
            }
            
            Ok(ValidatedValue::MultiValue(validated_items))
        } else {
            let validated = self.validate_single_value(attr_def, value, context)?;
            Ok(ValidatedValue::SingleValue(Box::new(validated)))
        }
    }
    
    fn validate_single_value(
        &self,
        attr_def: &AttributeDefinition,
        value: &Value,
        context: &ValidationContext,
    ) -> ValidationResult<ValidatedValue> {
        match attr_def.attribute_type() {
            AttributeType::String => {
                let str_value = value.as_str()
                    .ok_or_else(|| ValidationError::InvalidType {
                        attribute: attr_def.name().to_string(),
                        expected: "string".to_string(),
                        actual: value_type_name(value).to_string(),
                    })?;
                
                // Apply string constraints
                if let Some(max_length) = attr_def.max_length() {
                    if str_value.len() > max_length {
                        return Err(ValidationError::StringTooLong {
                            attribute: attr_def.name().to_string(),
                            max_length,
                            actual_length: str_value.len(),
                        });
                    }
                }
                
                // Apply custom validation
                if let Some(pattern) = attr_def.pattern() {
                    if !pattern.is_match(str_value) {
                        return Err(ValidationError::PatternMismatch {
                            attribute: attr_def.name().to_string(),
                            pattern: pattern.to_string(),
                            value: str_value.to_string(),
                        });
                    }
                }
                
                Ok(ValidatedValue::String(str_value.to_string()))
            },
            
            AttributeType::Integer => {
                let int_value = value.as_i64()
                    .ok_or_else(|| ValidationError::InvalidType {
                        attribute: attr_def.name().to_string(),
                        expected: "integer".to_string(),
                        actual: value_type_name(value).to_string(),
                    })?;
                
                // Apply numeric constraints
                if let Some(min) = attr_def.min_value() {
                    if int_value < min {
                        return Err(ValidationError::ValueTooSmall {
                            attribute: attr_def.name().to_string(),
                            min_value: min,
                            actual_value: int_value,
                        });
                    }
                }
                
                if let Some(max) = attr_def.max_value() {
                    if int_value > max {
                        return Err(ValidationError::ValueTooLarge {
                            attribute: attr_def.name().to_string(),
                            max_value: max,
                            actual_value: int_value,
                        });
                    }
                }
                
                Ok(ValidatedValue::Integer(int_value))
            },
            
            AttributeType::Boolean => {
                let bool_value = value.as_bool()
                    .ok_or_else(|| ValidationError::InvalidType {
                        attribute: attr_def.name().to_string(),
                        expected: "boolean".to_string(),
                        actual: value_type_name(value).to_string(),
                    })?;
                
                Ok(ValidatedValue::Boolean(bool_value))
            },
            
            AttributeType::DateTime => {
                let date_str = value.as_str()
                    .ok_or_else(|| ValidationError::InvalidType {
                        attribute: attr_def.name().to_string(),
                        expected: "string (ISO 8601 datetime)".to_string(),
                        actual: value_type_name(value).to_string(),
                    })?;
                
                let parsed_date = chrono::DateTime::parse_from_rfc3339(date_str)
                    .map_err(|_| ValidationError::InvalidDateTime {
                        attribute: attr_def.name().to_string(),
                        value: date_str.to_string(),
                    })?;
                
                Ok(ValidatedValue::DateTime(parsed_date.with_timezone(&chrono::Utc)))
            },
            
            AttributeType::Complex => {
                if !value.is_object() {
                    return Err(ValidationError::InvalidType {
                        attribute: attr_def.name().to_string(),
                        expected: "object".to_string(),
                        actual: value_type_name(value).to_string(),
                    });
                }
                
                let mut validated_complex = HashMap::new();
                let obj = value.as_object().unwrap();
                
                // Validate each sub-attribute
                for sub_attr in attr_def.sub_attributes() {
                    if let Some(sub_value) = obj.get(sub_attr.name()) {
                        let validated_sub = self.validate_single_value(sub_attr, sub_value, context)?;
                        validated_complex.insert(sub_attr.name().to_string(), validated_sub);
                    } else if sub_attr.is_required() {
                        return Err(ValidationError::MissingRequiredAttribute {
                            parent: attr_def.name().to_string(),
                            attribute: sub_attr.name().to_string(),
                        });
                    }
                }
                
                Ok(ValidatedValue::Complex(validated_complex))
            },
            
            AttributeType::Reference => {
                let ref_str = value.as_str()
                    .ok_or_else(|| ValidationError::InvalidType {
                        attribute: attr_def.name().to_string(),
                        expected: "string (reference)".to_string(),
                        actual: value_type_name(value).to_string(),
                    })?;
                
                // Validate reference format and type
                let reference = ResourceReference::parse(ref_str)
                    .map_err(|_| ValidationError::InvalidReference {
                        attribute: attr_def.name().to_string(),
                        value: ref_str.to_string(),
                    })?;
                
                // Check if reference type is allowed
                if !attr_def.reference_types().contains(&reference.resource_type) {
                    return Err(ValidationError::InvalidReferenceType {
                        attribute: attr_def.name().to_string(),
                        allowed_types: attr_def.reference_types().clone(),
                        actual_type: reference.resource_type,
                    });
                }
                
                Ok(ValidatedValue::Reference(reference))
            },
        }
    }
    
    fn validate_required_attributes(
        &self,
        data: &Value,
        _context: &ValidationContext,
    ) -> ValidationResult<()> {
        if let Value::Object(obj) = data {
            for attr_def in self.merged_attributes.values() {
                if attr_def.is_required() && !obj.contains_key(attr_def.name()) {
                    return Err(ValidationError::MissingRequiredAttribute {
                        parent: "root".to_string(),
                        attribute: attr_def.name().to_string(),
                    });
                }
            }
        }
        Ok(())
    }
    
    fn is_core_attribute(&self, name: &str) -> bool {
        matches!(name, "id" | "schemas" | "meta" | "externalId")
    }
}
```

## Value Object System

### Static Value Objects

```rust
use serde::{Deserialize, Serialize};
use std::fmt;

// Compile-time validated value objects
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Email {
    value: String,
    email_type: Option<String>,
    primary: Option<bool>,
    display: Option<String>,
}

impl Email {
    pub fn new(value: String) -> ValidationResult<Self> {
        Self::validate_email_format(&value)?;
        
        Ok(Self {
            value,
            email_type: None,
            primary: None,
            display: None,
        })
    }
    
    pub fn with_type(mut self, email_type: String) -> Self {
        self.email_type = Some(email_type);
        self
    }
    
    pub fn with_primary(mut self, primary: bool) -> Self {
        self.primary = Some(primary);
        self
    }
    
    pub fn with_display(mut self, display: String) -> Self {
        self.display = Some(display);
        self
    }
    
    fn validate_email_format(email: &str) -> ValidationResult<()> {
        if !email.contains('@') {
            return Err(ValidationError::InvalidEmailFormat(email.to_string()));
        }
        
        let parts: Vec<&str> = email.split('@').collect();
        if parts.len() != 2 || parts[0].is_empty() || parts[1].is_empty() {
            return Err(ValidationError::InvalidEmailFormat(email.to_string()));
        }
        
        // More comprehensive email validation
        if parts[1].contains("..") || parts[0].contains("..") {
            return Err(ValidationError::InvalidEmailFormat(email.to_string()));
        }
        
        Ok(())
    }
    
    pub fn value(&self) -> &str {
        &self.value
    }
    
    pub fn email_type(&self) -> Option<&str> {
        self.email_type.as_deref()
    }
    
    pub fn is_primary(&self) -> bool {
        self.primary.unwrap_or(false)
    }
}

impl fmt::Display for Email {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.value)
    }
}

impl ValueObject for Email {
    fn attribute_type(&self) -> AttributeType {
        AttributeType::Complex
    }
    
    fn to_json(&self) -> ValidationResult<Value> {
        Ok(json!({
            "value": self.value,
            "type": self.email_type.as_deref().unwrap_or("work"),
            "primary": self.primary.unwrap_or(false),
            "display": self.display.as_deref().unwrap_or(&self.value)
        }))
    }
    
    fn validate_against_schema(&self, definition: &AttributeDefinition) -> ValidationResult<()> {
        // Email-specific schema validation
        if definition.attribute_type() != AttributeType::Complex {
            return Err(ValidationError::TypeMismatch {
                expected: AttributeType::Complex,
                actual: definition.attribute_type(),
            });
        }
        
        // Validate against sub-attributes
        for sub_attr in definition.sub_attributes() {
            match sub_attr.name() {
                "value" => {
                    if sub_attr.attribute_type() != AttributeType::String {
                        return Err(ValidationError::SubAttributeTypeMismatch {
                            attribute: "email.value".to_string(),
                            expected: AttributeType::String,
                            actual: sub_attr.attribute_type(),
                        });
                    }
                },
                "type" | "display" => {
                    if sub_attr.attribute_type() != AttributeType::String {
                        return Err(ValidationError::SubAttributeTypeMismatch {
                            attribute: format!("email.{}", sub_attr.name()),
                            expected: AttributeType::String,
                            actual: sub_attr.attribute_type(),
                        });
                    }
                },
                "primary" => {
                    if sub_attr.attribute_type() != AttributeType::Boolean {
                        return Err(ValidationError::SubAttributeTypeMismatch {
                            attribute: "email.primary".to_string(),
                            expected: AttributeType::Boolean,
                            actual: sub_attr.attribute_type(),
                        });
                    }
                },
                _ => {} // Allow unknown sub-attributes for extensibility
            }
        }
        
        Ok(())
    }
}

impl SchemaConstructible for Email {
    fn from_schema_and_value(
        definition: &AttributeDefinition,
        value: &Value,
    ) -> ValidationResult<Self> {
        if definition.attribute_type() != AttributeType::Complex {
            return Err(ValidationError::TypeMismatch {
                expected: AttributeType::Complex,
                actual: definition.attribute_type(),
            });
        }
        
        let obj = value.as_object()
            .ok_or(ValidationError::InvalidStructure("Expected object for email".to_string()))?;
        
        let email_value = obj.get("value")
            .and_then(|v| v.as_str())
            .ok_or(ValidationError::MissingRequiredField("email.value".to_string()))?;
        
        let mut email = Email::new(email_value.to_string())?;
        
        if let Some(email_type) = obj.get("type").and_then(|v| v.as_str()) {
            email = email.with_type(email_type.to_string());
        }
        
        if let Some(primary) = obj.get("primary").and_then(|v| v.as_bool()) {
            email = email.with_primary(primary);
        }
        
        if let Some(display) = obj.get("display").and_then(|v| v.as_str()) {
            email = email.with_display(display.to_string());
        }
        
        Ok(email)
    }
}

// Custom value object for business-specific data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmployeeId {
    value: String,
    department_code: String,
    hire_year: u16,
}

impl EmployeeId {
    pub fn new(value: String) -> ValidationResult<Self> {
        Self::parse_employee_id(&value)
    }
    
    fn parse_employee_id(id: &str) -> ValidationResult<Self> {
        // Format: DEPT-YYYY-NNNN (e.g., ENG-2023-0001)
        let parts: Vec<&str> = id.split('-').collect();
        if parts.len() != 3 {
            return Err(ValidationError::InvalidEmployeeIdFormat(id.to_string()));
        }
        
        let department_code = parts[0].to_string();
        let hire_year: u16 = parts[1].parse()
            .map_err(|_| ValidationError::InvalidEmployeeIdFormat(id.to_string()))?;
        let sequence: u16 = parts[2].parse()
            .map_err(|_| ValidationError::InvalidEmployeeIdFormat(id.to_string()))?;
        
        // Validate department code
        if !matches!(department_code.as_str(), "ENG" | "SAL" | "MKT" | "HR" | "FIN") {
            return Err(ValidationError::InvalidDepartmentCode(department_code));
        }
        
        // Validate year range
        let current_year = chrono::Utc::now().year() as u16;
        if hire_year < 2000 || hire_year > current_year + 1 {
            return Err(ValidationError::InvalidHireYear(hire_year));
        }
        
        Ok(Self {
            value: id.to_string(),
            department_code,
            hire_year,
        })
    }
    
    pub fn department(&self) -> &str {
        &self.department_code
    }
    
    pub fn hire_year(&self) -> u16 {
        self.hire_year
    }
}

impl ValueObject for EmployeeId {
    fn attribute_type(&self) -> AttributeType {
        AttributeType::String
    }
    
    fn to_json(&self) -> ValidationResult<Value> {
        Ok(json!(self.value))
    }
    
    fn validate_against_schema(&self, definition: &AttributeDefinition) -> ValidationResult<()> {
        if definition.attribute_type() != AttributeType::String {
            return Err(ValidationError::TypeMismatch {
                expected: AttributeType::String,
                actual: definition.attribute_type(),
            });
        }
        
        // Additional business rule validation
        if let Some(pattern) = definition.pattern() {
            if !pattern.is_match(&self.value) {
                return Err(ValidationError::PatternMismatch {
                    attribute: "employeeId".to_string(),
                    pattern: pattern.to_string(),
                    value: self.value.clone(),
                });
            }
        }
        
        Ok(())
    }
}

impl SchemaConstructible for EmployeeId {
    fn from_schema_and_value(
        _definition: &AttributeDefinition,
        value: &Value,
    ) -> ValidationResult<Self> {
        let id_str = value.as_str()
            .ok_or(ValidationError::InvalidType {
                attribute: "employeeId".to_string(),
                expected: "string".to_string(),
                actual: value_type_name(value).to_string(),
            })?;
        
        Self::new(id_str.to_string())
    }
}
```

### Dynamic Value Objects

```rust
use serde_json::{Value, Map};
use std::any::{Any, TypeId};

pub struct DynamicValueObject {
    attribute_name: String,
    attribute_type: AttributeType,
    raw_value: Value,
    validated_value: Option<Box<dyn Any + Send + Sync>>,
    metadata: HashMap<String, String>,
}

impl DynamicValueObject {
    pub fn from_schema_and_value(
        definition: &AttributeDefinition,
        value: &Value,
    ) -> ValidationResult<Self> {
        let mut dynamic_obj = Self {
            attribute_name: definition.name().to_string(),
            attribute_type: definition.attribute_type(),
            raw_value: value.clone(),
            validated_value: None,
            metadata: HashMap::new(),
        };
        
        // Perform type-specific validation and conversion
        dynamic_obj.validate_and_convert(definition)?;
        
        Ok(dynamic_obj)
    }
    
    fn validate_and_convert(&mut self, definition: &AttributeDefinition) -> ValidationResult<()> {
        match self.attribute_type {
            AttributeType::String => {
                let str_value = self.raw_value.as_str()
                    .ok_or_else(|| ValidationError::InvalidType {
                        attribute: self.attribute_name.clone(),
                        expected: "string".to_string(),
                        actual: value_type_name(&self.raw_value).to_string(),
                    })?;
                
                self.validate_string_constraints(str_value, definition)?;
                self.validated_value = Some(Box::new(str_value.to_string()));
            },
            
            AttributeType::Integer => {
                let int_value = self.raw_value.as_i64()
                    .ok_or_else(|| ValidationError::InvalidType {
                        attribute: self.attribute_name.clone(),
                        expected: "integer".to_string(),
                        actual: value_type_name(&self.raw_value).to_string(),
                    })?;
                
                self.validate_numeric_constraints(int_value, definition)?;
                self.validated_value = Some(Box::new(int_value));
            },
            
            AttributeType::Complex => {
                let obj_value = self.raw_value.as_object()
                    .ok_or_else(|| ValidationError::InvalidType {
                        attribute: self.attribute_name.clone(),
                        expected: "object".to_string(),
                        actual: value_type_name(&self.raw_value).to_string(),
                    })?;
                
                let validated_complex = self.validate_complex_object(obj_value, definition)?;
                self.validated_value = Some(Box::new(validated_complex));
            },
            
            // Handle other types...
            _ => {
                return Err(ValidationError::UnsupportedAttributeType(self.attribute_type));
            }
        }
        
        Ok(())
    }
    
    fn validate_string_constraints(
        &mut self,
        value: &str,
        definition: &AttributeDefinition,
    ) -> ValidationResult<()> {
        // Length validation
        if let Some(max_length) = definition.max_length() {
            if value.len() > max_length {
                return Err(ValidationError::StringTooLong {
                    attribute: self.attribute_name.clone(),
                    max_length,
                    actual_length: value.len(),
                });
            }
        }
        
        // Pattern validation
        if let Some(pattern) = definition.pattern() {
            if !pattern.is_match(value) {
                return Err(ValidationError::PatternMismatch {
                    attribute: self.attribute_name.clone(),
                    pattern: pattern.to_string(),
                    value: value.to_string(),
                });
            }
        }
        
        // Enum validation
        if let Some(canonical_values) = definition.canonical_values() {
            if !canonical_values.contains(&value.to_string()) {
                return Err(ValidationError::InvalidEnumValue {
                    attribute: self.attribute_name.clone(),
                    allowed_values: canonical_values.clone(),
                    actual_value: value.to_string(),
                });
            }
        }
        
        Ok(())
    }
    
    fn validate_numeric_constraints(
        &mut self,
        value: i64,
        definition: &AttributeDefinition,
    ) -> ValidationResult<()> {
        if let Some(min_value) = definition.min_value() {
            if value < min_value {
                return Err(ValidationError::ValueTooSmall {
                    attribute: self.attribute_name.clone(),
                    min_value,
                    actual_value: value,
                });
            }
        }
        
        if let Some(max_value) = definition.max_value() {
            if value > max_value {
                return Err(ValidationError::ValueTooLarge {
                    attribute: self.attribute_name.clone(),
                    max_value,
                    actual_value: value,
                });
            }
        }
        
        Ok(())
    }
    
    fn validate_complex_object(
        &mut self,
        obj: &Map<String, Value>,
        definition: &AttributeDefinition,
    ) -> ValidationResult<HashMap<String, DynamicValueObject>> {
        let mut validated_complex = HashMap::new();
        
        // Validate each sub-attribute
        for sub_attr in definition.sub_attributes() {
            if let Some(sub_value) = obj.get(sub_attr.name()) {
                let validated_sub = DynamicValueObject::from_schema_and_value(sub_attr, sub_value)?;
                validated_complex.insert(sub_attr.name().to_string(), validated_sub);
            } else if sub_attr.is_required() {
                return Err(ValidationError::MissingRequiredAttribute {
                    parent: self.attribute_name.clone(),
                    attribute: sub_attr.name().to_string(),
                });
            }
        }
        
        // Check for unknown attributes
        for (field_name, _) in obj {
            if !definition.sub_attributes().iter().any(|attr| attr.name() == field_name) {
                self.metadata.insert(
                    format!("unknown_field_{}", field_name),
                    "present".to_string(),
                );
            }
        }
        
        Ok(validated_complex)
    }
    
    pub fn get_validated_value<T: 'static>(&self) -> Option<&T> {
        self.validated_value.as_ref()
            .and_then(|boxed| boxed.downcast_ref::<T>())
    }
    
    pub fn to_json(&self) -> ValidationResult<Value> {
        Ok(self.raw_value.clone())
    }
    
    pub fn has_metadata(&self, key: &str) -> bool {
        self.metadata.contains_key(key)
    }
}

impl ValueObject for DynamicValueObject {
    fn attribute_type(&self) -> AttributeType {
        self.attribute_type
    }
    
    fn to_json(&self) -> ValidationResult<Value> {
        self.to_json()
    }
    
    fn validate_against_schema(&self, definition: &AttributeDefinition) -> ValidationResult<()> {
        if self.attribute_type != definition.attribute_type() {
            return Err(ValidationError::TypeMismatch {
                expected: definition.attribute_type(),
                actual: self.attribute_type,
            });
        }
        
        // Re-validate with current definition (schema may have changed)
        let mut temp_obj = Self {
            attribute_name: self.attribute_name.clone(),
            attribute_type: self.attribute_type,
            raw_value: self.raw_value.clone(),
            validated_value: None,
            metadata: HashMap::new(),
        };
        
        temp_obj.validate_and_convert(definition)?;
        Ok(())
    }
}
```

## Schema Extension Patterns

### Tenant-Specific Extensions

```rust
pub struct TenantSchemaManager {
    base_registry: Arc<ExtendedSchemaRegistry>,
    tenant_schemas: RwLock<HashMap<String, TenantSchemaSet>>,
    extension_loader: Box<dyn SchemaLoader>,
}

#[derive(Clone)]
struct TenantSchemaSet {
    tenant_id: String,
    extensions: Vec<Schema>,
    compiled_schemas: HashMap<String, CompositeSchema>,
    last_updated: chrono::DateTime<chrono::Utc>,
}

pub trait SchemaLoader: Send + Sync {
    async fn load_tenant_extensions(&self, tenant_id: &str) -> Result<Vec<Schema>, SchemaLoadError>;
    async fn watch_schema_changes(&self, callback: Box<dyn Fn(&str, &Schema) + Send + Sync>);
}

impl TenantSchemaManager {
    pub fn new(
        base_registry: Arc<ExtendedSchemaRegistry>,
        extension_loader: Box<dyn SchemaLoader>,
    ) -> Self {
        Self {
            base_registry,
            tenant_schemas: RwLock::new(HashMap::new()),
            extension_loader,
        }
    }
    
    pub async fn load_tenant_schemas(&self, tenant_id: &str) -> Result<(), SchemaError> {
        let extensions = self.extension_loader.load_tenant_extensions(tenant_id).await?;
        
        let tenant_set = TenantSchemaSet {
            tenant_id: tenant_id.to_string(),
            extensions: extensions.clone(),
            compiled_schemas: HashMap::new(),
            last_updated: chrono::Utc::now(),
        };
        
        // Validate and register extensions
        for extension in &extensions {
            self.base_registry.register_extension_schema(extension.clone())?;
        }
        
        let mut tenant_schemas = self.tenant_schemas.write().unwrap();
        tenant_schemas.insert(tenant_id.to_string(), tenant_set);
        
        Ok(())
    }
    
    pub async fn get_tenant_schema(
        &self,
        tenant_id: &str,
        resource_type: &str,
    ) -> Result<CompositeSchema, SchemaError> {
        // Check if tenant schemas are loaded
        {
            let tenant_schemas = self.tenant_schemas.read().unwrap();
            if !tenant_schemas.contains_key(tenant_id) {
                drop(tenant_schemas);
                self.load_tenant_schemas(tenant_id).await?;
            }
        }
        
        let mut tenant_schemas = self.tenant_schemas.write().unwrap();
        let tenant_set = tenant_schemas.get_mut(tenant_id)
            .ok_or_else(|| SchemaError::TenantNotFound(tenant_id.to_string()))?;
        
        // Check if we have a compiled schema cached
        if let Some(compiled) = tenant_set.compiled_schemas.get(resource_type) {
            return Ok(compiled.clone());
        }
        
        // Build composite schema
        let base_schema = self.base_registry.get_schema(resource_type)?;
        let mut composite = CompositeSchema::new();
        composite.add_schema(base_schema);
        
        // Add relevant tenant extensions
        for extension in &tenant_set.extensions {
            if extension.applies_to_resource_type(resource_type) {
                composite.add_extension(extension.clone());
            }
        }
        
        let compiled = composite.compile()?;
        tenant_set.compiled_schemas.insert(resource_type.to_string(), compiled.clone());
        
        Ok(compiled)
    }
    
    pub async fn validate_tenant_resource(
        &self,
        tenant_id: &str,
        resource_type: &str,
        resource_data: &Value,
    ) -> ValidationResult<ValidatedResource> {
        let schema = self.get_tenant_schema(tenant_id, resource_type).await?;
        
        let validation_context = ValidationContext {
            resource_type: resource_type.to_string(),
            tenant_id: Some(tenant_id.to_string()),
            schema_registry: self.base_registry.as_ref(),
            custom_context: HashMap::new(),
        };
        
        schema.validate(resource_data, &validation_context)
    }
}

// Database-backed schema loader
pub struct DatabaseSchemaLoader {
    db_pool: PgPool,
    schema_cache: RwLock<HashMap<String, CachedTenantSchemas>>,
}

#[derive(Clone)]
struct CachedTenantSchemas {
    schemas: Vec<Schema>,
    cached_at: chrono::DateTime<chrono::Utc>,
}

impl DatabaseSchemaLoader {
    pub fn new(db_pool: PgPool) -> Self {
        Self {
            db_pool,
            schema_cache: RwLock::new(HashMap::new()),
        }
    }
}

impl SchemaLoader for DatabaseSchemaLoader {
    async fn load_tenant_extensions(&self, tenant_id: &str) -> Result<Vec<Schema>, SchemaLoadError> {
        // Check cache first
        {
            let cache = self.schema_cache.read().unwrap();
            if let Some(cached) = cache.get(tenant_id) {
                if cached.cached_at.signed_duration_since(chrono::Utc::now()).num_minutes().abs() < 5 {
                    return Ok(cached.schemas.clone());
                }
            }
        }
        
        // Query database for tenant extensions
        let rows = sqlx::query!(
            r#"
            SELECT schema_id, schema_name, schema_definition, resource_types
            FROM tenant_schema_extensions
            WHERE tenant_id = $1 AND active = true
            ORDER BY priority DESC
            "#,
            tenant_id
        )
        .fetch_all(&self.db_pool)
        .await?;
        
        let mut schemas = Vec::new();
        
        for row in rows {
            let schema_def: Value = serde_json::from_str(&row.schema_definition)?;
            let resource_types: Vec<String> = serde_json::from_str(&row.resource_types)?;
            
            let schema = Schema::from_json(schema_def)
                .with_id(row.schema_id)
                .with_name(row.schema_name)
                .with_resource_types(resource_types)
                .build()?;
                
            schemas.push(schema);
        }
        
        // Update cache
        {
            let mut cache = self.schema_cache.write().unwrap();
            cache.insert(tenant_id.to_string(), CachedTenantSchemas {
                schemas: schemas.clone(),
                cached_at: chrono::Utc::now(),
            });
        }
        
        Ok(schemas)
    }
    
    async fn watch_schema_changes(&self, callback: Box<dyn Fn(&str, &Schema) + Send + Sync>) {
        // Implementation would use database change streams or polling
        // This is a simplified example
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(30));
            loop {
                interval.tick().await;
                // Check for schema changes and call callback
                // callback(&tenant_id, &changed_schema);
            }
        });
    }
}
```

## Performance Optimization

### Schema Compilation and Caching

```rust
use std::sync::Arc;
use tokio::sync::RwLock as AsyncRwLock;

pub struct OptimizedSchemaRegistry {
    inner: ExtendedSchemaRegistry,
    compiled_validators: AsyncRwLock<HashMap<String, Arc<CompiledValidator>>>,
    validation_cache: AsyncRwLock<lru::LruCache<String, ValidationResult<ValidatedResource>>>,
    compilation_stats: Arc<CompilationStats>,
}

pub struct CompiledValidator {
    schema_id: String,
    validator_fn: Box<dyn Fn(&Value, &ValidationContext) -> ValidationResult<ValidatedResource> + Send + Sync>,
    attribute_validators: HashMap<String, AttributeValidator>,
    required_attributes: HashSet<String>,
    compilation_metadata: CompilationMetadata,
}

#[derive(Debug)]
struct CompilationMetadata {
    compiled_at: chrono::DateTime<chrono::Utc>,
    optimization_level: OptimizationLevel,
    validation_stats: ValidationStats,
}

#[derive(Debug, Clone)]
enum OptimizationLevel {
    None,
    Basic,
    Aggressive,
}

impl OptimizedSchemaRegistry {
    pub fn new(optimization_level: OptimizationLevel) -> Result<Self, SchemaError> {
        Ok(Self {
            inner: ExtendedSchemaRegistry::new()?,
            compiled_validators: AsyncRwLock::new(HashMap::new()),
            validation_cache: AsyncRwLock::new(lru::LruCache::new(1000)),
            compilation_stats: Arc::new(CompilationStats::new()),
        })
    }
    
    pub async fn register_optimized_schema(&self, schema: Schema) -> Result<(), SchemaError> {
        // Register with inner registry
        self.inner.register_extension_schema(schema.clone())?;
        
        // Compile optimized validator
        let compiled_validator = self.compile_schema_validator(&schema).await?;
        
        let mut validators = self.compiled_validators.write().await;
        validators.insert(schema.id().to_string(), Arc::new(compiled_validator));
        
        // Clear validation cache for this schema
        let mut cache = self.validation_cache.write().await;
        cache.clear();
        
        Ok(())
    }
    
    async fn compile_schema_validator(&self, schema: &Schema) -> Result<CompiledValidator, SchemaError> {
        let start_time = std::time::Instant::now();
        
        let mut attribute_validators = HashMap::new();
        let mut required_attributes = HashSet::new();
        
        // Pre-compile attribute validators
        for attribute in schema.attributes() {
            let attr_validator = self.compile_attribute_validator(attribute).await?;
            attribute_validators.insert(attribute.name().to_string(), attr_validator);
            
            if attribute.is_required() {
                required_attributes.insert(attribute.name().to_string());
            }
        }
        
        // Create optimized validation function
        let schema_id = schema.id().to_string();
        let validator_fn = self.create_validation_function(
            schema_id.clone(),
            attribute_validators.clone(),
            required_attributes.clone(),
        );
        
        let compilation_time = start_time.elapsed();
        self.compilation_stats.record_compilation(schema.id(), compilation_time);
        
        Ok(CompiledValidator {
            schema_id,
            validator_fn,
            attribute_validators,
            required_attributes,
            compilation_metadata: CompilationMetadata {
                compiled_at: chrono::Utc::now(),
                optimization_level: OptimizationLevel::Aggressive,
                validation_stats: ValidationStats::new(),
            },
        })
    }
    
    async fn compile_attribute_validator(
        &self,
        attribute: &AttributeDefinition,
    ) -> Result<AttributeValidator, SchemaError> {
        match attribute.attribute_type() {
            AttributeType::String => {
                let mut constraints = Vec::new();
                
                if let Some(max_len) = attribute.max_length() {
                    constraints.push(StringConstraint::MaxLength(max_len));
                }
                
                if let Some(pattern) = attribute.pattern() {
                    constraints.push(StringConstraint::Pattern(pattern.clone()));
                }
                
                if let Some(canonical_values) = attribute.canonical_values() {
                    constraints.push(StringConstraint::Enum(canonical_values.clone()));
                }
                
                Ok(AttributeValidator::String(StringValidator { constraints }))
            },
            
            AttributeType::Integer => {
                let mut constraints = Vec::new();
                
                if let Some(min) = attribute.min_value() {
                    constraints.push(IntegerConstraint::MinValue(min));
                }
                
                if let Some(max) = attribute.max_value() {
                    constraints.push(IntegerConstraint::MaxValue(max));
                }
                
                Ok(AttributeValidator::Integer(IntegerValidator { constraints }))
            },
            
            AttributeType::Complex => {
                let mut sub_validators = HashMap::new();
                
                for sub_attr in attribute.sub_attributes() {
                    let sub_validator = Box::pin(self.compile_attribute_validator(sub_attr)).await?;
                    sub_validators.insert(sub_attr.name().to_string(), sub_validator);
                }
                
                Ok(AttributeValidator::Complex(ComplexValidator { 
                    sub_validators,
                    required_sub_attributes: attribute.sub_attributes()
                        .iter()
                        .filter(|attr| attr.is_required())
                        .map(|attr| attr.name().to_string())
                        .collect(),
                }))
            },
            
            // Handle other types...
            _ => Ok(AttributeValidator::Generic),
        }
    }
    
    fn create_validation_function(
        &self,
        schema_id: String,
        attribute_validators: HashMap<String, AttributeValidator>,
        required_attributes: HashSet<String>,
    ) -> Box<dyn Fn(&Value, &ValidationContext) -> ValidationResult<ValidatedResource> + Send + Sync> {
        Box::new(move |value: &Value, context: &ValidationContext| -> ValidationResult<ValidatedResource> {
            let start_time = std::time::Instant::now();
            
            let mut validated_resource = ValidatedResource::new();
            validated_resource.set_schema_id(schema_id.clone());
            
            // Fast path: validate object structure first
            let obj = value.as_object()
                .ok_or_else(|| ValidationError::InvalidStructure("Expected object".to_string()))?;
            
            // Check required attributes (optimized with pre-computed set)
            for required_attr in &required_attributes {
                if !obj.contains_key(required_attr) {
                    return Err(ValidationError::MissingRequiredAttribute {
                        parent: "root".to_string(),
                        attribute: required_attr.clone(),
                    });
                }
            }
            
            // Validate each attribute using compiled validators
            for (attr_name, attr_value) in obj {
                if let Some(validator) = attribute_validators.get(attr_name) {
                    let validated_value = validator.validate_fast(attr_value, context)?;
                    validated_resource.add_attribute(attr_name.clone(), validated_value);
                } else if !is_core_attribute(attr_name) {
                    return Err(ValidationError::UnknownAttribute(attr_name.clone()));
                }
            }
            
            let validation_time = start_time.elapsed();
            validated_resource.set_validation_time(validation_time);
            
            Ok(validated_resource)
        })
    }
    
    pub async fn validate_with_cache(
        &self,
        resource_type: &str,
        resource_data: &Value,
        context: &ValidationContext,
    ) -> ValidationResult<ValidatedResource> {
        // Create cache key from resource data hash and context
        let cache_key = self.create_cache_key(resource_type, resource_data, context);
        
        // Check validation cache
        {
            let mut cache = self.validation_cache.write().await;
            if let Some(cached_result) = cache.get(&cache_key) {
                return cached_result.clone();
            }
        }
        
        // Perform validation
        let result = self.validate_optimized(resource_type, resource_data, context).await;
        
        // Cache successful validations
        if result.is_ok() {
            let mut cache = self.validation_cache.write().await;
            cache.put(cache_key, result.clone());
        }
        
        result
    }
    
    async fn validate_optimized(
        &self,
        resource_type: &str,
        resource_data: &Value,
        context: &ValidationContext,
    ) -> ValidationResult<ValidatedResource> {
        let validators = self.compiled_validators.read().await;
        
        if let Some(compiled_validator) = validators.get(resource_type) {
            // Use compiled validator for optimal performance
            (compiled_validator.validator_fn)(resource_data, context)
        } else {
            // Fall back to standard validation
            self.inner.validate_resource(resource_type, resource_data, context.tenant_id.as_deref())
        }
    }
    
    fn create_cache_key(
        &self,
        resource_type: &str,
        resource_data: &Value,
        context: &ValidationContext,
    ) -> String {
        use std::hash::{Hash, Hasher};
        use std::collections::hash_map::DefaultHasher;
        
        let mut hasher = DefaultHasher::new();
        resource_type.hash(&mut hasher);
        resource_data.to_string().hash(&mut hasher);
        context.tenant_id.hash(&mut hasher);
        
        format!("validation_{}_{}", resource_type, hasher.finish())
    }
}

#[derive(Debug, Clone)]
enum AttributeValidator {
    String(StringValidator),
    Integer(IntegerValidator),
    Boolean,
    DateTime,
    Complex(ComplexValidator),
    Reference(ReferenceValidator),
    Generic,
}

impl AttributeValidator {
    fn validate_fast(&self, value: &Value, context: &ValidationContext) -> ValidationResult<ValidatedValue> {
        match self {
            AttributeValidator::String(validator) => validator.validate_fast(value, context),
            AttributeValidator::Integer(validator) => validator.validate_fast(value, context),
            AttributeValidator::Complex(validator) => validator.validate_fast(value, context),
            // ... other validators
            _ => Err(ValidationError::UnsupportedValidationType),
        }
    }
}
```

## Best Practices Summary

### Schema Design Guidelines

1. **Start with SCIM Core Schemas**
   - Extend rather than replace standard schemas
   - Maintain SCIM compliance for interoperability
   - Use standard attribute names where possible

2. **Design Extensible Schemas**
   - Use complex attributes for structured data
   - Plan for multi-valued attributes early
   - Consider tenant-specific extensions

3. **Implement Efficient Validation**
   - Compile schemas for performance
   - Cache validation results appropriately
   - Use type-safe value objects where beneficial

4. **Handle Schema Evolution**
   - Version your custom schemas
   - Plan migration strategies
   - Support backward compatibility

5. **Monitor Schema Performance**
   - Track validation times
   - Monitor cache hit rates
   - Profile schema compilation costs

## Related Topics

- **[Understanding SCIM Schemas](../concepts/schemas.md)** - Core schema concepts
- **[Schema Mechanisms in SCIM Server](../concepts/schema-mechanisms.md)** - Implementation details
- **[Resource Provider Architecture](./resource-provider-architecture.md)** - How schemas integrate with providers
- **[Multi-Tenant Architecture Patterns](./multi-tenant-patterns.md)** - Tenant-specific schema extensions

## Next Steps

Now that you understand schema system architecture:

1. **Design your schema extensions** based on your domain requirements
2. **Implement custom value objects** for complex business data
3. **Set up tenant-specific extensions** for multi-tenant systems
4. **Optimize schema validation performance** for high-throughput scenarios
5. **Plan schema evolution strategies** for long-term maintenance