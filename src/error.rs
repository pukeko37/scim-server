//! Error types for SCIM server operations.
//!
//! This module provides comprehensive error handling for all SCIM operations,
//! following Rust's error handling best practices with detailed error information.

/// Main error type for SCIM server operations.
///
/// This enum covers all possible error conditions that can occur during
/// SCIM server operations, providing detailed context for each error type.
#[derive(Debug, thiserror::Error)]
pub enum ScimError {
    /// Validation errors when resource data doesn't conform to schema
    #[error("Validation error: {0}")]
    Validation(#[from] ValidationError),

    /// Errors from the user-provided resource provider
    #[error("Resource provider error: {0}")]
    Provider(#[source] Box<dyn std::error::Error + Send + Sync>),

    /// JSON serialization/deserialization errors
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    /// Resource not found errors
    #[error("Resource not found: {resource_type} with ID {id}")]
    ResourceNotFound {
        /// The type of resource that was not found
        resource_type: String,
        /// The ID of the resource that was not found
        id: String,
    },

    /// Schema not found errors
    #[error("Schema not found: {schema_id}")]
    SchemaNotFound {
        /// The ID of the schema that was not found
        schema_id: String,
    },

    /// Internal server errors
    #[error("Internal server error: {message}")]
    Internal {
        /// Description of the internal error
        message: String,
    },

    /// Invalid request format or parameters
    #[error("Invalid request: {message}")]
    InvalidRequest {
        /// Description of what makes the request invalid
        message: String,
    },

    /// Unsupported resource type
    #[error("Unsupported resource type: {0}")]
    UnsupportedResourceType(String),

    /// Unsupported operation for resource type
    #[error("Unsupported operation {operation} for resource type {resource_type}")]
    UnsupportedOperation {
        /// The resource type for which the operation is unsupported
        resource_type: String,
        /// The operation that is not supported
        operation: String,
    },

    /// Resource provider error with string message
    #[error("Resource provider error: {0}")]
    ProviderError(String),
}

/// Validation errors for schema compliance checking.
///
/// These errors occur when resource data doesn't conform to the defined schema,
/// providing detailed information about what validation rule was violated.
#[derive(Debug, thiserror::Error)]
pub enum ValidationError {
    /// Required attribute is missing
    #[error("Required attribute '{attribute}' is missing")]
    MissingRequiredAttribute {
        /// The name of the missing required attribute
        attribute: String,
    },

    /// Attribute value doesn't match expected type
    #[error("Attribute '{attribute}' has invalid type, expected {expected}, got {actual}")]
    InvalidAttributeType {
        /// The name of the attribute with invalid type
        attribute: String,
        /// The expected type for this attribute
        expected: String,
        /// The actual type that was provided
        actual: String,
    },

    /// Multi-valued attribute provided as single value
    #[error("Attribute '{attribute}' must be multi-valued (array)")]
    ExpectedMultiValue {
        /// The name of the attribute that should be multi-valued
        attribute: String,
    },

    /// Single-valued attribute provided as array
    #[error("Attribute '{attribute}' must be single-valued (not array)")]
    ExpectedSingleValue {
        /// The name of the attribute that should be single-valued
        attribute: String,
    },

    /// Attribute value violates uniqueness constraint
    #[error("Attribute '{attribute}' violates uniqueness constraint")]
    UniquenesViolation {
        /// The name of the attribute that violates uniqueness
        attribute: String,
    },

    /// Invalid value for attribute with canonical values
    #[error("Attribute '{attribute}' has invalid value '{value}', allowed values: {allowed:?}")]
    InvalidCanonicalValue {
        /// The name of the attribute with invalid canonical value
        attribute: String,
        /// The invalid value that was provided
        value: String,
        /// The list of allowed canonical values
        allowed: Vec<String>,
    },

    /// Complex attribute missing required sub-attributes
    #[error("Complex attribute '{attribute}' missing required sub-attribute '{sub_attribute}'")]
    MissingSubAttribute {
        /// The name of the complex attribute
        attribute: String,
        /// The name of the missing required sub-attribute
        sub_attribute: String,
    },

    /// Unknown attribute in resource
    #[error("Unknown attribute '{attribute}' in schema '{schema_id}'")]
    UnknownAttribute {
        /// The name of the unknown attribute
        attribute: String,
        /// The ID of the schema where the attribute was not found
        schema_id: String,
    },

    /// General validation error with custom message
    #[error("Validation failed: {message}")]
    Custom {
        /// Custom validation error message
        message: String,
    },

    /// Missing schemas attribute
    #[error("Missing required 'schemas' attribute")]
    MissingSchemas,

    /// Empty schemas array
    #[error("'schemas' array cannot be empty")]
    EmptySchemas,

    /// Invalid schema URI format
    #[error("Invalid schema URI format: {uri}")]
    InvalidSchemaUri {
        /// The invalid schema URI
        uri: String,
    },

    /// Unknown schema URI
    #[error("Unknown schema URI: {uri}")]
    UnknownSchemaUri {
        /// The unknown schema URI
        uri: String,
    },

    /// Duplicate schema URI
    #[error("Duplicate schema URI: {uri}")]
    DuplicateSchemaUri {
        /// The duplicated schema URI
        uri: String,
    },

    /// Missing base schema
    #[error("Missing base schema for resource type")]
    MissingBaseSchema,

    /// Extension without base schema
    #[error("Extension schema requires base schema")]
    ExtensionWithoutBase,

    /// Missing required extension
    #[error("Missing required extension schema")]
    MissingRequiredExtension,

    /// Missing id attribute
    #[error("Missing required 'id' attribute")]
    MissingId,

    /// Empty id value
    #[error("'id' attribute cannot be empty")]
    EmptyId,

    /// Invalid id format
    #[error("Invalid 'id' format: {id}")]
    InvalidIdFormat {
        /// The invalid ID value that was provided
        id: String,
    },

    /// Client provided id in creation
    #[error("Client cannot provide 'id' during resource creation")]
    ClientProvidedId,

    /// Invalid external id
    #[error("Invalid 'externalId' format")]
    InvalidExternalId,

    /// Invalid meta structure
    #[error("Invalid 'meta' structure")]
    InvalidMetaStructure,

    /// Missing meta resource type
    #[error("Missing 'meta.resourceType'")]
    MissingResourceType,

    /// Invalid meta resource type
    #[error("Invalid 'meta.resourceType': {resource_type}")]
    InvalidResourceType {
        /// The invalid resource type value
        resource_type: String,
    },

    /// Client provided meta
    #[error("Client cannot provide read-only meta attributes")]
    ClientProvidedMeta,

    /// Invalid created datetime
    #[error("Invalid 'meta.created' datetime format")]
    InvalidCreatedDateTime,

    /// Invalid modified datetime
    #[error("Invalid 'meta.lastModified' datetime format")]
    InvalidModifiedDateTime,

    /// Invalid location URI
    #[error("Invalid 'meta.location' URI format")]
    InvalidLocationUri,

    /// Invalid version format
    #[error("Invalid 'meta.version' format")]
    InvalidVersionFormat,

    /// Invalid data type for attribute
    #[error("Attribute '{attribute}' has invalid type, expected {expected}, got {actual}")]
    InvalidDataType {
        /// The name of the attribute with invalid data type
        attribute: String,
        /// The expected data type
        expected: String,
        /// The actual data type that was provided
        actual: String,
    },

    /// Invalid string format
    #[error("Attribute '{attribute}' has invalid string format: {details}")]
    InvalidStringFormat {
        /// The name of the attribute with invalid string format
        attribute: String,
        /// Details about what makes the format invalid
        details: String,
    },

    /// Invalid boolean value
    #[error("Attribute '{attribute}' has invalid boolean value: {value}")]
    InvalidBooleanValue {
        /// The name of the attribute with invalid boolean value
        attribute: String,
        /// The invalid boolean value that was provided
        value: String,
    },

    /// Invalid decimal format
    #[error("Attribute '{attribute}' has invalid decimal format: {value}")]
    InvalidDecimalFormat {
        /// The name of the attribute with invalid decimal format
        attribute: String,
        /// The invalid decimal value that was provided
        value: String,
    },

    /// Invalid integer value
    #[error("Attribute '{attribute}' has invalid integer value: {value}")]
    InvalidIntegerValue {
        /// The name of the attribute with invalid integer value
        attribute: String,
        /// The invalid integer value that was provided
        value: String,
    },

    /// Invalid datetime format
    #[error("Attribute '{attribute}' has invalid datetime format: {value}")]
    InvalidDateTimeFormat {
        /// The name of the attribute with invalid datetime format
        attribute: String,
        /// The invalid datetime value that was provided
        value: String,
    },

    /// Invalid binary data
    #[error("Attribute '{attribute}' has invalid binary data: {details}")]
    InvalidBinaryData {
        /// The name of the attribute with invalid binary data
        attribute: String,
        /// Details about what makes the binary data invalid
        details: String,
    },

    /// Invalid reference URI
    #[error("Attribute '{attribute}' has invalid reference URI: {uri}")]
    InvalidReferenceUri {
        /// The name of the attribute with invalid reference URI
        attribute: String,
        /// The invalid URI value that was provided
        uri: String,
    },

    /// Invalid reference type
    #[error("Attribute '{attribute}' has invalid reference type: {ref_type}")]
    InvalidReferenceType {
        /// The name of the attribute with invalid reference type
        attribute: String,
        /// The invalid reference type that was provided
        ref_type: String,
    },

    /// Broken reference
    #[error("Attribute '{attribute}' contains broken reference: {reference}")]
    BrokenReference {
        /// The name of the attribute with broken reference
        attribute: String,
        /// The broken reference value
        reference: String,
    },

    // Multi-valued Attribute Validation Errors (33-38)
    /// Single value provided for multi-valued attribute
    #[error("Attribute '{attribute}' must be multi-valued (array)")]
    SingleValueForMultiValued {
        /// The name of the attribute that requires multiple values
        attribute: String,
    },

    /// Array provided for single-valued attribute
    #[error("Attribute '{attribute}' must be single-valued (not array)")]
    ArrayForSingleValued {
        /// The name of the attribute that requires a single value
        attribute: String,
    },

    /// Multiple primary values in multi-valued attribute
    #[error("Attribute '{attribute}' cannot have multiple primary values")]
    MultiplePrimaryValues {
        /// The name of the attribute with multiple primary values
        attribute: String,
    },

    /// Invalid multi-valued structure
    #[error("Attribute '{attribute}' has invalid multi-valued structure: {details}")]
    InvalidMultiValuedStructure {
        /// The name of the attribute with invalid multi-valued structure
        attribute: String,
        /// Details about what makes the structure invalid
        details: String,
    },

    /// Missing required sub-attribute in multi-valued
    #[error("Attribute '{attribute}' missing required sub-attribute '{sub_attribute}'")]
    MissingRequiredSubAttribute {
        /// The name of the multi-valued attribute
        attribute: String,
        /// The name of the missing required sub-attribute
        sub_attribute: String,
    },

    // Complex Attribute Validation Errors (39-43)
    /// Missing required sub-attributes in complex attribute
    #[error("Complex attribute '{attribute}' missing required sub-attributes: {missing:?}")]
    MissingRequiredSubAttributes {
        /// The name of the complex attribute
        attribute: String,
        /// The list of missing required sub-attributes
        missing: Vec<String>,
    },

    /// Invalid sub-attribute type in complex attribute
    #[error(
        "Complex attribute '{attribute}' has invalid sub-attribute '{sub_attribute}' type, expected {expected}, got {actual}"
    )]
    InvalidSubAttributeType {
        attribute: String,
        sub_attribute: String,
        expected: String,
        actual: String,
    },

    /// Unknown sub-attribute in complex attribute
    #[error("Complex attribute '{attribute}' contains unknown sub-attribute '{sub_attribute}'")]
    UnknownSubAttribute {
        attribute: String,
        sub_attribute: String,
    },

    /// Nested complex attributes (not allowed)
    #[error("Nested complex attributes are not allowed: '{attribute}'")]
    NestedComplexAttributes { attribute: String },

    /// Malformed complex attribute structure
    #[error("Complex attribute '{attribute}' has malformed structure: {details}")]
    MalformedComplexStructure { attribute: String, details: String },

    // Attribute Characteristics Validation Errors (44-52)
    /// Case sensitivity violation
    #[error("Attribute '{attribute}' violates case sensitivity rules: {details}")]
    CaseSensitivityViolation { attribute: String, details: String },

    /// Read-only mutability violation
    #[error("Attribute '{attribute}' is read-only and cannot be modified")]
    ReadOnlyMutabilityViolation { attribute: String },

    /// Immutable mutability violation
    #[error("Attribute '{attribute}' is immutable and cannot be modified after creation")]
    ImmutableMutabilityViolation { attribute: String },

    /// Write-only attribute returned
    #[error("Attribute '{attribute}' is write-only and should not be returned")]
    WriteOnlyAttributeReturned { attribute: String },

    /// Server uniqueness violation
    #[error("Attribute '{attribute}' violates server uniqueness constraint with value '{value}'")]
    ServerUniquenessViolation { attribute: String, value: String },

    /// Global uniqueness violation
    #[error("Attribute '{attribute}' violates global uniqueness constraint with value '{value}'")]
    GlobalUniquenessViolation { attribute: String, value: String },

    /// Invalid canonical value choice
    #[error(
        "Attribute '{attribute}' has invalid canonical value '{value}', allowed values: {allowed:?}"
    )]
    InvalidCanonicalValueChoice {
        attribute: String,
        value: String,
        allowed: Vec<String>,
    },

    /// Unknown attribute for schema
    #[error("Unknown attribute '{attribute}' for schema '{schema}'")]
    UnknownAttributeForSchema { attribute: String, schema: String },

    /// Required characteristic violation
    #[error("Attribute '{attribute}' violates required characteristic '{characteristic}'")]
    RequiredCharacteristicViolation {
        attribute: String,
        characteristic: String,
    },

    // Schema-Driven Value Object Errors
    /// Unsupported attribute type for value object creation
    #[error("Unsupported attribute type '{type_name}' for attribute '{attribute}'")]
    UnsupportedAttributeType {
        attribute: String,
        type_name: String,
    },

    /// Invalid attribute name mismatch
    #[error("Invalid attribute name '{actual}', expected '{expected}'")]
    InvalidAttributeName { actual: String, expected: String },

    /// Required attribute missing
    #[error("Required attribute '{0}' is missing")]
    RequiredAttributeMissing(String),

    /// Null value for optional attribute (used in factory)
    #[error("Null value provided for optional attribute '{0}'")]
    NullValueForOptionalAttribute(String),

    /// Expected array for multi-valued attribute
    #[error("Expected array for multi-valued attribute '{0}'")]
    ExpectedArray(String),

    /// Invalid primary index for multi-valued attribute
    #[error("Invalid primary index {index} for attribute '{attribute}' (length: {length})")]
    InvalidPrimaryIndex {
        attribute: String,
        index: usize,
        length: usize,
    },

    /// Attribute is not multi-valued
    #[error("Attribute '{0}' is not multi-valued")]
    NotMultiValued(String),

    /// Reserved username error
    #[error("Username '{0}' is reserved and cannot be used")]
    ReservedUsername(String),

    /// Username too short
    #[error("Username '{0}' is too short (minimum 3 characters)")]
    UsernameTooShort(String),

    /// Username too long
    #[error("Username '{0}' is too long (maximum 64 characters)")]
    UsernameTooLong(String),

    /// Invalid username format
    #[error("Username '{0}' has invalid format")]
    InvalidUsernameFormat(String),

    /// Invalid email domain
    #[error("Email '{email}' has invalid domain, allowed domains: {allowed_domains:?}")]
    InvalidEmailDomain {
        email: String,
        allowed_domains: Vec<String>,
    },

    /// Work email required
    #[error("A work email address is required")]
    WorkEmailRequired,

    /// External ID required
    #[error("External ID is required")]
    ExternalIdRequired,

    /// Name component required
    #[error("At least one name component (given, family, or formatted) is required")]
    NameComponentRequired,

    /// Empty formatted name
    #[error("Formatted name cannot be empty")]
    EmptyFormattedName,
}

/// Errors that can occur during server building/configuration.
///
/// These errors are typically programming errors and should be caught
/// during development rather than runtime.
#[derive(Debug, thiserror::Error)]
pub enum BuildError {
    /// Resource provider was not configured
    #[error("Resource provider is required but not provided")]
    MissingResourceProvider,

    /// Invalid configuration provided
    #[error("Invalid configuration: {message}")]
    InvalidConfiguration { message: String },

    /// Schema loading failed
    #[error("Failed to load schema: {schema_id}")]
    SchemaLoadError { schema_id: String },
}

// Convenience methods for creating common errors
impl ScimError {
    /// Create a resource not found error
    pub fn resource_not_found(resource_type: impl Into<String>, id: impl Into<String>) -> Self {
        Self::ResourceNotFound {
            resource_type: resource_type.into(),
            id: id.into(),
        }
    }

    /// Create a schema not found error
    pub fn schema_not_found(schema_id: impl Into<String>) -> Self {
        Self::SchemaNotFound {
            schema_id: schema_id.into(),
        }
    }

    /// Create an internal server error
    pub fn internal(message: impl Into<String>) -> Self {
        Self::Internal {
            message: message.into(),
        }
    }

    /// Create an invalid request error
    pub fn invalid_request(message: impl Into<String>) -> Self {
        Self::InvalidRequest {
            message: message.into(),
        }
    }

    /// Wrap a provider error
    pub fn provider_error<E>(error: E) -> Self
    where
        E: std::error::Error + Send + Sync + 'static,
    {
        Self::Provider(Box::new(error))
    }
}

impl ValidationError {
    /// Create a missing required attribute error
    pub fn missing_required(attribute: impl Into<String>) -> Self {
        Self::MissingRequiredAttribute {
            attribute: attribute.into(),
        }
    }

    /// Create an invalid type error
    pub fn invalid_type(
        attribute: impl Into<String>,
        expected: impl Into<String>,
        actual: impl Into<String>,
    ) -> Self {
        Self::InvalidAttributeType {
            attribute: attribute.into(),
            expected: expected.into(),
            actual: actual.into(),
        }
    }

    /// Create a custom validation error
    pub fn custom(message: impl Into<String>) -> Self {
        Self::Custom {
            message: message.into(),
        }
    }
}

// Result type aliases for convenience
pub type ScimResult<T> = Result<T, ScimError>;
pub type ValidationResult<T> = Result<T, ValidationError>;
pub type BuildResult<T> = Result<T, BuildError>;

// Implement From for common error conversions
impl From<serde_json::Error> for ValidationError {
    fn from(error: serde_json::Error) -> Self {
        ValidationError::Custom {
            message: format!("JSON serialization error: {}", error),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_creation() {
        let error = ScimError::resource_not_found("User", "123");
        assert!(error.to_string().contains("User"));
        assert!(error.to_string().contains("123"));
    }

    #[test]
    fn test_validation_error_creation() {
        let error = ValidationError::missing_required("userName");
        assert!(error.to_string().contains("userName"));
    }

    #[test]
    fn test_error_chain() {
        let validation_error = ValidationError::missing_required("userName");
        let scim_error = ScimError::from(validation_error);
        assert!(scim_error.to_string().contains("Validation error"));
    }
}
