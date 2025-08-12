//! Schema operation handlers
//!
//! This module contains handlers for schema-related operations such as retrieving
//! all schemas or a specific schema by ID.

use crate::{
    ResourceProvider, ScimError,
    error::ScimResult,
    operation_handler::core::{
        OperationMetadata, ScimOperationHandler, ScimOperationRequest, ScimOperationResponse,
    },
    resource::RequestContext,
};
use std::collections::HashMap;

/// Handle get schemas operations.
pub async fn handle_get_schemas<P: ResourceProvider + Sync>(
    handler: &ScimOperationHandler<P>,
    _request: ScimOperationRequest,
    context: &RequestContext,
) -> ScimResult<ScimOperationResponse> {
    let schemas = handler.server().get_all_schemas();

    let schemas_json: Result<Vec<_>, _> = schemas
        .iter()
        .map(|schema| {
            serde_json::to_value(schema)
                .map_err(|e| ScimError::internal(format!("Failed to serialize schema: {}", e)))
        })
        .collect();

    let schemas_json = schemas_json?;
    let schema_count = schemas_json.len();

    let response_data = serde_json::json!({
        "schemas": schemas_json
    });

    Ok(ScimOperationResponse {
        success: true,
        data: Some(response_data),
        error: None,
        error_code: None,
        metadata: OperationMetadata {
            resource_type: Some("Schema".to_string()),
            resource_id: None,
            resource_count: Some(schema_count),
            total_results: Some(schema_count),
            request_id: context.request_id.clone(),
            tenant_id: context.tenant_context.as_ref().map(|t| t.tenant_id.clone()),
            schemas: None,
            additional: HashMap::new(),
        },
    })
}

/// Handle single schema retrieval.
pub async fn handle_get_schema<P: ResourceProvider + Sync>(
    handler: &ScimOperationHandler<P>,
    request: ScimOperationRequest,
    context: &RequestContext,
) -> ScimResult<ScimOperationResponse> {
    let schema_id = request.resource_id.ok_or_else(|| {
        ScimError::invalid_request("Missing schema_id for get schema operation".to_string())
    })?;

    let schema = handler.server().get_schema_by_id(&schema_id);

    match schema {
        Some(schema) => {
            let schema_json = serde_json::to_value(schema)
                .map_err(|e| ScimError::internal(format!("Failed to serialize schema: {}", e)))?;

            Ok(ScimOperationResponse {
                success: true,
                data: Some(schema_json),
                error: None,
                error_code: None,
                metadata: OperationMetadata {
                    resource_type: Some("Schema".to_string()),
                    resource_id: Some(schema_id),
                    resource_count: Some(1),
                    total_results: None,
                    request_id: context.request_id.clone(),
                    tenant_id: context.tenant_context.as_ref().map(|t| t.tenant_id.clone()),
                    schemas: None,
                    additional: HashMap::new(),
                },
            })
        }
        None => Err(ScimError::schema_not_found(schema_id)),
    }
}
