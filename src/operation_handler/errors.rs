//! Error handling utilities for operation handlers
//!
//! This module contains shared error response creation utilities used across
//! all operation handlers.

use crate::{
    ScimError,
    operation_handler::core::{OperationMetadata, ScimOperationResponse},
    resource::version::{HttpVersion, VersionConflict},
};
use serde_json::Value;
use std::collections::HashMap;

/// Create an error response from a ScimError.
pub fn create_error_response(error: ScimError, request_id: String) -> ScimOperationResponse {
    let (error_message, error_code) = match &error {
        ScimError::Validation(ve) => (
            format!("Validation error: {}", ve),
            Some("VALIDATION_ERROR"),
        ),
        ScimError::ResourceNotFound { resource_type, id } => (
            format!("Resource not found: {} with ID {}", resource_type, id),
            Some("RESOURCE_NOT_FOUND"),
        ),
        ScimError::SchemaNotFound { schema_id } => (
            format!("Schema not found: {}", schema_id),
            Some("SCHEMA_NOT_FOUND"),
        ),
        ScimError::UnsupportedResourceType(resource_type) => (
            format!("Unsupported resource type: {}", resource_type),
            Some("UNSUPPORTED_RESOURCE_TYPE"),
        ),
        ScimError::UnsupportedOperation {
            resource_type,
            operation,
        } => (
            format!(
                "Unsupported operation {} for resource type {}",
                operation, resource_type
            ),
            Some("UNSUPPORTED_OPERATION"),
        ),
        ScimError::InvalidRequest { message } => (
            format!("Invalid request: {}", message),
            Some("INVALID_REQUEST"),
        ),
        ScimError::Provider(provider_error) => (
            format!("Provider error: {}", provider_error),
            Some("PROVIDER_ERROR"),
        ),
        ScimError::Internal { message } => (
            format!("Internal error: {}", message),
            Some("INTERNAL_ERROR"),
        ),
        _ => (error.to_string(), Some("UNKNOWN_ERROR")),
    };

    ScimOperationResponse {
        success: false,
        data: None,
        error: Some(error_message),
        error_code: error_code.map(|s| s.to_string()),
        metadata: OperationMetadata {
            resource_type: None,
            resource_id: None,
            resource_count: None,
            total_results: None,
            request_id,
            tenant_id: None,
            schemas: None,
            additional: HashMap::new(),
        },
    }
}

/// Create a response for version conflicts.
pub fn create_version_conflict_response(
    conflict: VersionConflict,
    request_id: String,
    resource_type: Option<String>,
    resource_id: Option<String>,
) -> ScimOperationResponse {
    let mut additional = HashMap::new();
    additional.insert(
        "expected_version".to_string(),
        Value::String(conflict.expected.as_str().to_string()),
    );
    additional.insert(
        "current_version".to_string(),
        Value::String(conflict.current.as_str().to_string()),
    );
    additional.insert(
        "expected_etag".to_string(),
        Value::String(HttpVersion::from(conflict.expected.clone()).to_string()),
    );
    additional.insert(
        "current_etag".to_string(),
        Value::String(HttpVersion::from(conflict.current.clone()).to_string()),
    );

    ScimOperationResponse {
        success: false,
        data: None,
        error: Some(conflict.message),
        error_code: Some("version_mismatch".to_string()),
        metadata: OperationMetadata {
            resource_type,
            resource_id,
            resource_count: None,
            total_results: None,
            request_id,
            tenant_id: None,
            schemas: None,
            additional,
        },
    }
}
