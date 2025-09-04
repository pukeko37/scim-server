//! CRUD operation handlers
//!
//! This module contains handlers for Create, Read, Update, and Delete operations.
//! It includes shared error handling utilities used across all CRUD operations.

use crate::{
    ResourceProvider, ScimError,
    error::ScimResult,
    operation_handler::{
        core::{
            OperationMetadata, ScimOperationHandler, ScimOperationRequest, ScimOperationResponse,
        },
        create_version_conflict_response,
    },
    resource::{
        RequestContext,
        conditional_provider::VersionedResource,
        version::{ConditionalResult, HttpVersion},
    },
};
use std::collections::HashMap;

/// Handle create operations.
pub async fn handle_create<P: ResourceProvider + Sync>(
    handler: &ScimOperationHandler<P>,
    request: ScimOperationRequest,
    context: &RequestContext,
) -> ScimResult<ScimOperationResponse> {
    let data = request.data.ok_or_else(|| {
        ScimError::invalid_request("Missing data for create operation".to_string())
    })?;

    let resource = handler
        .server()
        .create_resource(&request.resource_type, data, context)
        .await?;

    // Include version information in response
    let versioned_resource = VersionedResource::new(resource.clone());
    let mut additional = HashMap::new();
    additional.insert(
        "version".to_string(),
        serde_json::Value::String(versioned_resource.version().as_str().to_string()),
    );
    additional.insert(
        "etag".to_string(),
        serde_json::Value::String(
            HttpVersion::from(versioned_resource.version().clone()).to_string(),
        ),
    );

    Ok(ScimOperationResponse {
        success: true,
        data: Some(
            handler
                .server()
                .serialize_resource_with_refs(&resource, context.tenant_id())?,
        ),
        error: None,
        error_code: None,
        metadata: OperationMetadata {
            resource_type: Some(request.resource_type),
            resource_id: resource.get_id().map(|s| s.to_string()),
            resource_count: Some(1),
            total_results: None,
            request_id: context.request_id.clone(),
            tenant_id: context.tenant_context.as_ref().map(|t| t.tenant_id.clone()),
            schemas: Some(
                resource
                    .schemas
                    .iter()
                    .map(|s| s.as_str().to_string())
                    .collect(),
            ),
            additional,
        },
    })
}

/// Handle get operations.
pub async fn handle_get<P: ResourceProvider + Sync>(
    handler: &ScimOperationHandler<P>,
    request: ScimOperationRequest,
    context: &RequestContext,
) -> ScimResult<ScimOperationResponse> {
    let resource_id = request.resource_id.ok_or_else(|| {
        ScimError::invalid_request("Missing resource_id for get operation".to_string())
    })?;

    let resource = handler
        .server()
        .get_resource(&request.resource_type, &resource_id, context)
        .await?;

    match resource {
        Some(resource) => {
            // Include version information in response
            let versioned_resource = VersionedResource::new(resource.clone());
            let mut additional = HashMap::new();
            additional.insert(
                "version".to_string(),
                serde_json::Value::String(versioned_resource.version().as_str().to_string()),
            );
            additional.insert(
                "etag".to_string(),
                serde_json::Value::String(
                    HttpVersion::from(versioned_resource.version().clone()).to_string(),
                ),
            );

            Ok(ScimOperationResponse {
                success: true,
                data: Some(
                    handler
                        .server()
                        .serialize_resource_with_refs(&resource, context.tenant_id())?,
                ),
                error: None,
                error_code: None,
                metadata: OperationMetadata {
                    resource_type: Some(request.resource_type),
                    resource_id: Some(resource_id),
                    resource_count: Some(1),
                    total_results: None,
                    request_id: context.request_id.clone(),
                    tenant_id: context.tenant_context.as_ref().map(|t| t.tenant_id.clone()),
                    schemas: Some(
                        resource
                            .schemas
                            .iter()
                            .map(|s| s.as_str().to_string())
                            .collect(),
                    ),
                    additional,
                },
            })
        }
        None => Err(ScimError::resource_not_found(
            request.resource_type,
            resource_id,
        )),
    }
}

/// Handle update operations.
pub async fn handle_update<P: ResourceProvider + Sync>(
    handler: &ScimOperationHandler<P>,
    request: ScimOperationRequest,
    context: &RequestContext,
) -> ScimResult<ScimOperationResponse> {
    let resource_id = request.resource_id.ok_or_else(|| {
        ScimError::invalid_request("Missing resource_id for update operation".to_string())
    })?;

    let data = request.data.ok_or_else(|| {
        ScimError::invalid_request("Missing data for update operation".to_string())
    })?;

    // Check if this is a conditional update request
    if let Some(expected_version) = &request.expected_version {
        // Use conditional update
        match handler
            .server()
            .provider()
            .conditional_update(
                &request.resource_type,
                &resource_id,
                data,
                expected_version,
                context,
            )
            .await
            .map_err(|e| ScimError::ProviderError(e.to_string()))?
        {
            ConditionalResult::Success(versioned_resource) => {
                let mut additional = HashMap::new();
                additional.insert(
                    "version".to_string(),
                    serde_json::Value::String(versioned_resource.version().as_str().to_string()),
                );
                additional.insert(
                    "etag".to_string(),
                    serde_json::Value::String(
                        HttpVersion::from(versioned_resource.version().clone()).to_string(),
                    ),
                );

                Ok(ScimOperationResponse {
                    success: true,
                    data: Some(handler.server().serialize_resource_with_refs(
                        versioned_resource.resource(),
                        context.tenant_id(),
                    )?),
                    error: None,
                    error_code: None,
                    metadata: OperationMetadata {
                        resource_type: Some(request.resource_type),
                        resource_id: Some(resource_id),
                        resource_count: Some(1),
                        total_results: None,
                        request_id: context.request_id.clone(),
                        tenant_id: context.tenant_context.as_ref().map(|t| t.tenant_id.clone()),
                        schemas: Some(
                            versioned_resource
                                .resource()
                                .schemas
                                .iter()
                                .map(|s| s.as_str().to_string())
                                .collect(),
                        ),
                        additional,
                    },
                })
            }
            ConditionalResult::VersionMismatch(conflict) => Ok(create_version_conflict_response(
                conflict,
                context.request_id.clone(),
                Some(request.resource_type),
                Some(resource_id),
            )),
            ConditionalResult::NotFound => Err(ScimError::resource_not_found(
                request.resource_type,
                resource_id,
            )),
        }
    } else {
        // Regular update
        let resource = handler
            .server()
            .update_resource(&request.resource_type, &resource_id, data, context)
            .await?;

        // Include version information in response
        let versioned_resource = VersionedResource::new(resource.clone());
        let mut additional = HashMap::new();
        additional.insert(
            "version".to_string(),
            serde_json::Value::String(versioned_resource.version().as_str().to_string()),
        );
        additional.insert(
            "etag".to_string(),
            serde_json::Value::String(
                HttpVersion::from(versioned_resource.version().clone()).to_string(),
            ),
        );

        Ok(ScimOperationResponse {
            success: true,
            data: Some(
                handler
                    .server()
                    .serialize_resource_with_refs(&resource, context.tenant_id())?,
            ),
            error: None,
            error_code: None,
            metadata: OperationMetadata {
                resource_type: Some(request.resource_type),
                resource_id: Some(resource_id),
                resource_count: Some(1),
                total_results: None,
                request_id: context.request_id.clone(),
                tenant_id: context.tenant_context.as_ref().map(|t| t.tenant_id.clone()),
                schemas: Some(
                    resource
                        .schemas
                        .iter()
                        .map(|s| s.as_str().to_string())
                        .collect(),
                ),
                additional,
            },
        })
    }
}

/// Handle delete operations.
pub async fn handle_delete<P: ResourceProvider + Sync>(
    handler: &ScimOperationHandler<P>,
    request: ScimOperationRequest,
    context: &RequestContext,
) -> ScimResult<ScimOperationResponse> {
    let resource_id = request.resource_id.ok_or_else(|| {
        ScimError::invalid_request("Missing resource_id for delete operation".to_string())
    })?;

    // Check if this is a conditional delete request
    if let Some(expected_version) = &request.expected_version {
        // Use conditional delete
        match handler
            .server()
            .provider()
            .conditional_delete(
                &request.resource_type,
                &resource_id,
                expected_version,
                context,
            )
            .await
            .map_err(|e| ScimError::ProviderError(e.to_string()))?
        {
            ConditionalResult::Success(_) => Ok(ScimOperationResponse {
                success: true,
                data: None,
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
                    additional: HashMap::new(),
                },
            }),
            ConditionalResult::VersionMismatch(conflict) => Ok(create_version_conflict_response(
                conflict,
                context.request_id.clone(),
                Some(request.resource_type),
                Some(resource_id),
            )),
            ConditionalResult::NotFound => Err(ScimError::resource_not_found(
                request.resource_type,
                resource_id,
            )),
        }
    } else {
        // Regular delete
        handler
            .server()
            .delete_resource(&request.resource_type, &resource_id, context)
            .await?;

        Ok(ScimOperationResponse {
            success: true,
            data: None,
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
                additional: HashMap::new(),
            },
        })
    }
}

/// Handle patch operations.
pub async fn handle_patch<P: ResourceProvider + Sync>(
    handler: &ScimOperationHandler<P>,
    request: ScimOperationRequest,
    context: &RequestContext,
) -> ScimResult<ScimOperationResponse> {
    let resource_id = request.resource_id.ok_or_else(|| {
        ScimError::invalid_request("Missing resource_id for patch operation".to_string())
    })?;

    let data = request.data.ok_or_else(|| {
        ScimError::invalid_request("Missing data for patch operation".to_string())
    })?;

    let resource = handler
        .server()
        .patch_resource(&request.resource_type, &resource_id, &data, context)
        .await?;

    // Include version information in response
    let versioned_resource = VersionedResource::new(resource.clone());
    let mut additional = HashMap::new();
    additional.insert(
        "version".to_string(),
        serde_json::Value::String(versioned_resource.version().as_str().to_string()),
    );
    additional.insert(
        "etag".to_string(),
        serde_json::Value::String(
            HttpVersion::from(versioned_resource.version().clone()).to_string(),
        ),
    );

    Ok(ScimOperationResponse {
        success: true,
        data: Some(
            handler
                .server()
                .serialize_resource_with_refs(&resource, context.tenant_id())?,
        ),
        error: None,
        error_code: None,
        metadata: OperationMetadata {
            resource_type: Some(request.resource_type),
            resource_id: Some(resource_id),
            resource_count: Some(1),
            total_results: None,
            request_id: context.request_id.clone(),
            tenant_id: context.tenant_context.as_ref().map(|t| t.tenant_id.clone()),
            schemas: Some(
                resource
                    .schemas
                    .iter()
                    .map(|s| s.as_str().to_string())
                    .collect(),
            ),
            additional,
        },
    })
}
