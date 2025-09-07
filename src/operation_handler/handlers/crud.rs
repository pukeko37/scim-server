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
    resource::{RequestContext, version::HttpVersion, versioned::VersionedResource},
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

    // Update the resource's meta field with the new version
    let mut updated_resource = resource.clone();
    if let Some(meta) = updated_resource.get_meta() {
        if let Ok(updated_meta) = meta
            .clone()
            .with_version(versioned_resource.version().as_str().to_string())
        {
            updated_resource.set_meta(updated_meta);
        }
    } else {
        // Create meta field if it doesn't exist
        use crate::resource::value_objects::Meta;
        let now = chrono::Utc::now();
        if let Ok(meta) = Meta::new(
            updated_resource.resource_type.clone(),
            now,
            now,
            None,
            Some(versioned_resource.version().as_str().to_string()),
        ) {
            updated_resource.set_meta(meta);
        }
    }

    Ok(ScimOperationResponse {
        success: true,
        data: Some(
            handler
                .server()
                .serialize_resource_with_refs(&updated_resource, context.tenant_id())?,
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
            .update_resource(
                &request.resource_type,
                &resource_id,
                data,
                Some(expected_version),
                context,
            )
            .await
        {
            Ok(versioned_resource) => {
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

                // Update the resource's meta field with the new version
                let mut updated_resource = versioned_resource.resource().clone();
                if let Some(meta) = updated_resource.get_meta() {
                    if let Ok(updated_meta) = meta
                        .clone()
                        .with_version(versioned_resource.version().as_str().to_string())
                    {
                        updated_resource.set_meta(updated_meta);
                    }
                } else {
                    // Create meta field if it doesn't exist
                    use crate::resource::value_objects::Meta;
                    let now = chrono::Utc::now();
                    if let Ok(meta) = Meta::new(
                        updated_resource.resource_type.clone(),
                        now,
                        now,
                        None,
                        Some(versioned_resource.version().as_str().to_string()),
                    ) {
                        updated_resource.set_meta(meta);
                    }
                }

                Ok(ScimOperationResponse {
                    success: true,
                    data: Some(
                        handler
                            .server()
                            .serialize_resource_with_refs(&updated_resource, context.tenant_id())?,
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
            Err(e) => match &e {
                e if e.to_string().contains("Version conflict") => {
                    // Extract VersionConflict from ProviderError::VersionConflict
                    if let Some(conflict_start) = e.to_string().find("Version conflict: ") {
                        let _conflict_msg =
                            &e.to_string()[conflict_start + "Version conflict: ".len()..];
                        Ok(create_version_conflict_response(
                            crate::resource::version::VersionConflict::standard_message(
                                crate::resource::version::RawVersion::from_hash("unknown"),
                                crate::resource::version::RawVersion::from_hash("unknown"),
                            ),
                            context.request_id.clone(),
                            Some(request.resource_type),
                            Some(resource_id),
                        ))
                    } else {
                        Err(ScimError::ProviderError(e.to_string()))
                    }
                }
                e if e.to_string().contains("Precondition failed") => {
                    Ok(create_version_conflict_response(
                        crate::resource::version::VersionConflict::standard_message(
                            crate::resource::version::RawVersion::from_hash("unknown"),
                            crate::resource::version::RawVersion::from_hash("unknown"),
                        ),
                        context.request_id.clone(),
                        Some(request.resource_type),
                        Some(resource_id),
                    ))
                }
                e if e.to_string().contains("not found") => Err(ScimError::resource_not_found(
                    request.resource_type,
                    resource_id,
                )),
                _ => Err(ScimError::ProviderError(e.to_string())),
            },
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

        // Update the resource's meta field with the new version
        let mut updated_resource = resource.clone();
        if let Some(meta) = updated_resource.get_meta() {
            if let Ok(updated_meta) = meta
                .clone()
                .with_version(versioned_resource.version().as_str().to_string())
            {
                updated_resource.set_meta(updated_meta);
            }
        } else {
            // Create meta field if it doesn't exist
            use crate::resource::value_objects::Meta;
            let now = chrono::Utc::now();
            if let Ok(meta) = Meta::new(
                updated_resource.resource_type.clone(),
                now,
                now,
                None,
                Some(versioned_resource.version().as_str().to_string()),
            ) {
                updated_resource.set_meta(meta);
            }
        }

        Ok(ScimOperationResponse {
            success: true,
            data: Some(
                handler
                    .server()
                    .serialize_resource_with_refs(&updated_resource, context.tenant_id())?,
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
            .delete_resource(
                &request.resource_type,
                &resource_id,
                Some(expected_version),
                context,
            )
            .await
        {
            Ok(_) => Ok(ScimOperationResponse {
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
            Err(e) => match &e {
                e if e.to_string().contains("Version conflict") => {
                    Ok(create_version_conflict_response(
                        crate::resource::version::VersionConflict::standard_message(
                            crate::resource::version::RawVersion::from_hash("unknown"),
                            crate::resource::version::RawVersion::from_hash("unknown"),
                        ),
                        context.request_id.clone(),
                        Some(request.resource_type),
                        Some(resource_id),
                    ))
                }
                e if e.to_string().contains("Precondition failed") => {
                    Ok(create_version_conflict_response(
                        crate::resource::version::VersionConflict::standard_message(
                            crate::resource::version::RawVersion::from_hash("unknown"),
                            crate::resource::version::RawVersion::from_hash("unknown"),
                        ),
                        context.request_id.clone(),
                        Some(request.resource_type),
                        Some(resource_id),
                    ))
                }
                e if e.to_string().contains("not found") => Err(ScimError::resource_not_found(
                    request.resource_type,
                    resource_id,
                )),
                _ => Err(ScimError::ProviderError(e.to_string())),
            },
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

    // Update the resource's meta field with the new version
    let mut updated_resource = resource.clone();
    if let Some(meta) = updated_resource.get_meta() {
        if let Ok(updated_meta) = meta
            .clone()
            .with_version(versioned_resource.version().as_str().to_string())
        {
            updated_resource.set_meta(updated_meta);
        }
    } else {
        // Create meta field if it doesn't exist
        use crate::resource::value_objects::Meta;
        let now = chrono::Utc::now();
        if let Ok(meta) = Meta::new(
            updated_resource.resource_type.clone(),
            now,
            now,
            None,
            Some(versioned_resource.version().as_str().to_string()),
        ) {
            updated_resource.set_meta(meta);
        }
    }

    Ok(ScimOperationResponse {
        success: true,
        data: Some(
            handler
                .server()
                .serialize_resource_with_refs(&updated_resource, context.tenant_id())?,
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
