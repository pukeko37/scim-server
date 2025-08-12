//! Utility operation handlers
//!
//! This module contains handlers for utility operations such as checking
//! if a resource exists.

use crate::{
    ResourceProvider, ScimError,
    error::ScimResult,
    operation_handler::core::{
        OperationMetadata, ScimOperationHandler, ScimOperationRequest, ScimOperationResponse,
    },
    resource::RequestContext,
};
use std::collections::HashMap;

/// Handle resource existence check.
pub async fn handle_exists<P: ResourceProvider + Sync>(
    handler: &ScimOperationHandler<P>,
    request: ScimOperationRequest,
    context: &RequestContext,
) -> ScimResult<ScimOperationResponse> {
    let resource_id = request.resource_id.ok_or_else(|| {
        ScimError::invalid_request("Missing resource_id for exists operation".to_string())
    })?;

    let exists = handler
        .server()
        .provider()
        .resource_exists(&request.resource_type, &resource_id, context)
        .await
        .map_err(|e| ScimError::ProviderError(e.to_string()))?;

    let mut additional = HashMap::new();
    additional.insert("exists".to_string(), serde_json::Value::Bool(exists));

    Ok(ScimOperationResponse {
        success: true,
        data: Some(serde_json::Value::Bool(exists)),
        error: None,
        error_code: None,
        metadata: OperationMetadata {
            resource_type: Some(request.resource_type),
            resource_id: Some(resource_id),
            resource_count: None,
            total_results: None,
            request_id: context.request_id.clone(),
            tenant_id: context.tenant_context.as_ref().map(|t| t.tenant_id.clone()),
            schemas: None,
            additional,
        },
    })
}
