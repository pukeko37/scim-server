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
    ResourceNotFound { resource_type: String, id: String },

    /// Schema not found errors
    #[error("Schema not found: {schema_id}")]
    SchemaNotFound { schema_id: String },

    /// Internal server errors
    #[error("Internal server error: {message}")]
    Internal { message: String },

    /// Invalid request format or parameters
    #[error("Invalid request: {message}")]
    InvalidRequest { message: String },

    /// Unsupported resource type
    #[error("Unsupported resource type: {0}")]
    UnsupportedResourceType(String),

    /// Unsupported operation for resource type
    #[error("Unsupported operation '{operation}' for resource type '{resource_type}'")]
    UnsupportedOperation {
        resource_type: String,
        operation: String,
    },

    /// Method not found on resource
    #[error("Method '{0}' not found")]
    MethodNotFound(String),

    /// Schema mapper not found
    #[error("Schema mapper at index {0} not found")]
    MapperNotFound(usize),

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
    MissingRequiredAttribute { attribute: String },

    /// Attribute value doesn't match expected type
    #[error("Attribute '{attribute}' has invalid type, expected {expected}, got {actual}")]
    InvalidAttributeType {
        attribute: String,
        expected: String,
        actual: String,
    },

    /// Multi-valued attribute provided as single value
    #[error("Attribute '{attribute}' must be multi-valued (array)")]
    ExpectedMultiValue { attribute: String },

    /// Single-valued attribute provided as array
    #[error("Attribute '{attribute}' must be single-valued (not array)")]
    ExpectedSingleValue { attribute: String },

    /// Attribute value violates uniqueness constraint
    #[error("Attribute '{attribute}' violates uniqueness constraint")]
    UniquenesViolation { attribute: String },

    /// Invalid value for attribute with canonical values
    #[error("Attribute '{attribute}' has invalid value '{value}', allowed values: {allowed:?}")]
    InvalidCanonicalValue {
        attribute: String,
        value: String,
        allowed: Vec<String>,
    },

    /// Complex attribute missing required sub-attributes
    #[error("Complex attribute '{attribute}' missing required sub-attribute '{sub_attribute}'")]
    MissingSubAttribute {
        attribute: String,
        sub_attribute: String,
    },

    /// Unknown attribute in resource
    #[error("Unknown attribute '{attribute}' in schema '{schema_id}'")]
    UnknownAttribute {
        attribute: String,
        schema_id: String,
    },

    /// General validation error with custom message
    #[error("Validation failed: {message}")]
    Custom { message: String },
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
