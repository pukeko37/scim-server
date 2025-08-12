//! Query operation handlers
//!
//! This module contains handlers for List and Search operations that involve
//! querying multiple resources with optional filtering, pagination, and sorting.

use crate::{
    ResourceProvider, ScimError,
    error::ScimResult,
    operation_handler::core::{
        OperationMetadata, ScimOperationHandler, ScimOperationRequest, ScimOperationResponse,
    },
    resource::RequestContext,
};
use std::collections::HashMap;

/// Handle list operations.
pub async fn handle_list<P: ResourceProvider + Sync>(
    handler: &ScimOperationHandler<P>,
    request: ScimOperationRequest,
    context: &RequestContext,
) -> ScimResult<ScimOperationResponse> {
    let resources = handler
        .server()
        .list_resources(&request.resource_type, context)
        .await?;

    let resource_count = resources.len();
    let resources_json: Result<Vec<_>, _> = resources.iter().map(|r| r.to_json()).collect();

    let resources_json = resources_json?;

    Ok(ScimOperationResponse {
        success: true,
        data: Some(serde_json::Value::Array(resources_json)),
        error: None,
        error_code: None,
        metadata: OperationMetadata {
            resource_type: Some(request.resource_type),
            resource_id: None,
            resource_count: Some(resource_count),
            total_results: Some(resource_count),
            request_id: context.request_id.clone(),
            tenant_id: context.tenant_context.as_ref().map(|t| t.tenant_id.clone()),
            schemas: None,
            additional: HashMap::new(),
        },
    })
}

/// Handle search operations.
pub async fn handle_search<P: ResourceProvider + Sync>(
    handler: &ScimOperationHandler<P>,
    request: ScimOperationRequest,
    context: &RequestContext,
) -> ScimResult<ScimOperationResponse> {
    let query = request.query.ok_or_else(|| {
        ScimError::invalid_request("Missing query parameters for search operation".to_string())
    })?;

    let search_attribute = query.search_attribute.ok_or_else(|| {
        ScimError::invalid_request("Missing search_attribute for search operation".to_string())
    })?;

    let search_value = query.search_value.ok_or_else(|| {
        ScimError::invalid_request("Missing search_value for search operation".to_string())
    })?;

    let resources = handler
        .server()
        .list_resources(&request.resource_type, context)
        .await?
        .into_iter()
        .filter(|resource| {
            // Simple attribute-based filtering for now
            if let Ok(json) = resource.to_json() {
                if let Some(value) = json.get(&search_attribute) {
                    return value == &search_value;
                }
            }
            false
        })
        .collect::<Vec<_>>();

    let resource_count = resources.len();
    let resources_json: Result<Vec<_>, _> = resources.iter().map(|r| r.to_json()).collect();

    let resources_json = resources_json?;

    Ok(ScimOperationResponse {
        success: true,
        data: Some(serde_json::Value::Array(resources_json)),
        error: None,
        error_code: None,
        metadata: OperationMetadata {
            resource_type: Some(request.resource_type),
            resource_id: None,
            resource_count: Some(resource_count),
            total_results: Some(resource_count),
            request_id: context.request_id.clone(),
            tenant_id: context.tenant_context.as_ref().map(|t| t.tenant_id.clone()),
            schemas: None,
            additional: HashMap::new(),
        },
    })
}
