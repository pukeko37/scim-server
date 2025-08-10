# ETag Concurrency Design - SCIM Server Library

## Executive Summary

This document outlines the design and implementation strategy for ETag-based concurrency control in the SCIM server library. The current implementation lacks multi-client concurrency support, representing a **critical gap** for production deployments. This design addresses the gap through a provider-level concurrency strategy that maintains architectural boundaries while enabling robust conflict detection.

## Problem Statement

### Current State: No Concurrency Control

**Critical Issue**: The library currently provides **no protection against concurrent modifications**, making it unsuitable for production scenarios with multiple clients.

**Data Loss Scenario:**
```
Time: T1
Client A: GET /Users/123 → { "displayName": "John Doe", "meta": { "version": "W/\"123-1001\"" } }
Client B: GET /Users/123 → { "displayName": "John Doe", "meta": { "version": "W/\"123-1001\"" } }

Time: T2
Client A: PUT /Users/123 → { "displayName": "John A. Doe" } → Success
                         → { "meta": { "version": "W/\"123-1002\"" } }

Time: T3  
Client B: PUT /Users/123 → { "displayName": "John B. Doe" } → Success (overwrites A's changes)
                         → { "meta": { "version": "W/\"123-1003\"" } }

Result: Client A's changes are lost (last-write-wins)
```

### Missing Components

1. **ETag Value Object**: No type-safe ETag representation
2. **Conditional Operations**: No version-aware update methods
3. **Conflict Detection**: No standardized version mismatch handling
4. **Provider Interface**: No conditional update support in ResourceProvider trait
5. **Operation Integration**: No ETag extraction/validation in operation handlers

## RFC 7644 Requirements

### ETag Specification (from RFC 7643)

```
version: The version of the resource being returned. This value
   must be the same as the entity-tag (ETag) HTTP response header
   (see Sections 2.1 and 2.3 of [RFC7232]). This attribute has
   "caseExact" as "true".
```

### Conditional Request Flow (RFC 7644)

1. **Client reads resource** → receives ETag in `meta.version`
2. **Client modifies resource** → includes `If-Match: {etag}` header
3. **Server validates version** → compares request ETag with current
4. **Success path** → update succeeds, new ETag generated
5. **Conflict path** → returns `412 Precondition Failed`

## Design Strategy: Provider-Level Concurrency

### Architectural Positioning

**Why Provider-Level?**
- **Maintains boundaries**: HTTP layer remains user responsibility
- **Standardizes patterns**: All providers get consistent concurrency support
- **Flexible implementation**: Different storage backends can optimize differently
- **Non-breaking migration**: Can add alongside existing methods

### Layer Responsibilities

#### **Library Provides:**
- ETag value object with RFC 7232 compliance
- Enhanced ResourceProvider trait with conditional methods
- Standard conflict detection patterns
- Operation handler integration

#### **HTTP Layer (User):**
- ETag header extraction (If-Match → ScimOperationRequest)
- ETag header setting (meta.version → ETag response header)
- HTTP status mapping (ConditionalUpdateResult → 412 response)

#### **Provider (User):**
- Version comparison logic (storage-specific)
- Atomic update operations
- Current version retrieval

## Implementation Design

### Phase 1: ETag Value Object

**File**: `src/resource/value_objects/etag.rs`

```rust
/// RFC 7232 compliant ETag representation for SCIM resources.
///
/// ETags provide version information for optimistic concurrency control:
/// - Strong ETags: Byte-for-byte equivalence required
/// - Weak ETags: Semantic equivalence sufficient (SCIM default)
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ETag {
    value: String,
    is_weak: bool,
}

impl ETag {
    /// Create an ETag from HTTP header value, validating RFC 7232 format.
    pub fn new(value: String) -> ValidationResult<Self> {
        Self::validate_format(&value)?;
        let is_weak = value.starts_with("W/");
        Ok(Self { value, is_weak })
    }
    
    /// Create a weak ETag with opaque value (SCIM standard pattern).
    pub fn weak(opaque: String) -> ValidationResult<Self> {
        if opaque.is_empty() {
            return Err(ValidationError::InvalidETagOpaque);
        }
        let value = format!("W/\"{}\"", opaque);
        Ok(Self { value, is_weak: true })
    }
    
    /// Create a strong ETag with opaque value.
    pub fn strong(opaque: String) -> ValidationResult<Self> {
        if opaque.is_empty() {
            return Err(ValidationError::InvalidETagOpaque);
        }
        let value = format!("\"{}\"", opaque);
        Ok(Self { value, is_weak: false })
    }
    
    /// Generate weak ETag from SCIM resource metadata (standard pattern).
    pub fn generate_weak_scim(resource_id: &str, last_modified: DateTime<Utc>) -> Self {
        let timestamp = last_modified.timestamp_millis();
        let opaque = format!("{}-{}", resource_id, timestamp);
        Self::weak(opaque).expect("Generated ETag should be valid")
    }
    
    /// Check if this ETag matches another for conditional requests (RFC 7232).
    pub fn matches(&self, other: &ETag) -> bool {
        if self.is_weak || other.is_weak {
            // Weak comparison: semantic equivalence
            self.opaque_value() == other.opaque_value()
        } else {
            // Strong comparison: exact match required
            self.value == other.value
        }
    }
    
    /// Extract opaque value (content without W/ prefix and quotes).
    pub fn opaque_value(&self) -> &str {
        if self.is_weak {
            &self.value[3..self.value.len()-1]  // Remove W/" and "
        } else {
            &self.value[1..self.value.len()-1]  // Remove " and "
        }
    }
    
    /// Get full ETag value for HTTP headers.
    pub fn as_str(&self) -> &str {
        &self.value
    }
    
    /// Whether this is a weak ETag.
    pub fn is_weak(&self) -> bool {
        self.is_weak
    }
    
    fn validate_format(value: &str) -> ValidationResult<()> {
        if value.is_empty() {
            return Err(ValidationError::InvalidETagFormat);
        }
        
        if value.starts_with("W/\"") {
            if !value.ends_with('"') || value.len() < 4 {
                return Err(ValidationError::InvalidETagFormat);
            }
        } else if value.starts_with('"') {
            if !value.ends_with('"') || value.len() < 2 {
                return Err(ValidationError::InvalidETagFormat);
            }
        } else {
            return Err(ValidationError::InvalidETagFormat);
        }
        
        Ok(())
    }
}

impl ValueObject for ETag {
    type ValidationError = ValidationError;
    
    fn validate(value: &str) -> Result<(), Self::ValidationError> {
        Self::validate_format(value)
    }
}

impl fmt::Display for ETag {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.value)
    }
}

impl TryFrom<String> for ETag {
    type Error = ValidationError;
    
    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl TryFrom<&str> for ETag {
    type Error = ValidationError;
    
    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::new(value.to_string())
    }
}
```

### Phase 2: Enhanced Provider Trait

**File**: `src/resource/provider.rs`

```rust
/// Result of conditional resource operations.
#[derive(Debug, Clone)]
pub enum ConditionalUpdateResult {
    /// Update succeeded with new resource state.
    Updated(Resource),
    
    /// Version mismatch - update rejected.
    VersionMismatch { 
        /// The ETag that was provided in the request
        expected: ETag,
        /// The current ETag of the resource
        current: ETag,
    },
    
    /// Resource not found.
    NotFound,
}

impl ConditionalUpdateResult {
    /// Check version and return appropriate result.
    pub fn check_version(
        expected: Option<&ETag>, 
        current: &ETag, 
        resource: Resource
    ) -> Self {
        match expected {
            Some(expected_etag) if !expected_etag.matches(current) => {
                Self::VersionMismatch { 
                    expected: expected_etag.clone(), 
                    current: current.clone() 
                }
            }
            _ => Self::Updated(resource)
        }
    }
}

/// Enhanced ResourceProvider trait with conditional operation support.
pub trait ResourceProvider {
    type Error: std::error::Error + Send + Sync + 'static;
    
    // === Existing methods remain unchanged ===
    
    fn create_resource(
        &self,
        resource_type: &str,
        data: Value,
        context: &RequestContext,
    ) -> impl Future<Output = Result<Resource, Self::Error>> + Send;
    
    fn get_resource(
        &self,
        resource_type: &str,
        id: &str,
        context: &RequestContext,
    ) -> impl Future<Output = Result<Option<Resource>, Self::Error>> + Send;
    
    fn update_resource(
        &self,
        resource_type: &str,
        id: &str,
        data: Value,
        context: &RequestContext,
    ) -> impl Future<Output = Result<Resource, Self::Error>> + Send;
    
    // === New conditional methods ===
    
    /// Whether this provider supports conditional operations.
    fn supports_conditional_operations(&self) -> bool {
        false  // Default: no conditional support
    }
    
    /// Conditionally update a resource with version checking.
    ///
    /// This method should:
    /// 1. Retrieve current resource and its ETag
    /// 2. Compare expected_version with current version
    /// 3. If versions match, perform update and generate new ETag
    /// 4. If versions don't match, return VersionMismatch
    ///
    /// Providers can optimize this with atomic compare-and-swap operations.
    fn update_resource_conditional(
        &self,
        resource_type: &str,
        id: &str,
        data: Value,
        expected_version: Option<&ETag>,
        context: &RequestContext,
    ) -> impl Future<Output = Result<ConditionalUpdateResult, Self::Error>> + Send {
        // Default implementation: fall back to non-conditional update
        async move {
            match self.update_resource(resource_type, id, data, context).await {
                Ok(resource) => Ok(ConditionalUpdateResult::Updated(resource)),
                Err(e) => Err(e),
            }
        }
    }
    
    /// Conditionally delete a resource with version checking.
    fn delete_resource_conditional(
        &self,
        resource_type: &str,
        id: &str,
        expected_version: Option<&ETag>,
        context: &RequestContext,
    ) -> impl Future<Output = Result<ConditionalDeleteResult, Self::Error>> + Send {
        // Default implementation: fall back to non-conditional delete
        async move {
            match self.delete_resource(resource_type, id, context).await {
                Ok(()) => Ok(ConditionalDeleteResult::Deleted),
                Err(e) => Err(e),
            }
        }
    }
}

/// Result of conditional delete operations.
#[derive(Debug, Clone)]
pub enum ConditionalDeleteResult {
    /// Delete succeeded.
    Deleted,
    
    /// Version mismatch - delete rejected.
    VersionMismatch { 
        expected: ETag,
        current: ETag,
    },
    
    /// Resource not found.
    NotFound,
}
```

### Phase 3: Operation Handler Integration

**File**: `src/operation_handler.rs`

```rust
/// Enhanced operation request with conditional support.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScimOperationRequest {
    /// The type of operation to perform
    pub operation: ScimOperationType,
    /// The resource type (e.g., "User", "Group")
    pub resource_type: String,
    /// Resource ID for operations that target a specific resource
    pub resource_id: Option<String>,
    /// Data payload for create/update operations
    pub data: Option<Value>,
    /// Query parameters for list/search operations
    pub query: Option<ScimQuery>,
    /// Tenant context for multi-tenant scenarios
    pub tenant_context: Option<TenantContext>,
    /// Request ID for tracing
    pub request_id: Option<String>,
    
    // === New conditional fields ===
    
    /// Expected version for conditional updates (from If-Match header)
    pub expected_version: Option<ETag>,
    /// If-None-Match value for conditional creates
    pub if_none_match: Option<ETag>,
}

/// Enhanced operation response with version information.
#[derive(Debug, Serialize, Deserialize)]
pub struct ScimOperationResponse {
    /// Whether the operation succeeded
    pub success: bool,
    /// Response data (resource or list of resources)
    pub data: Option<Value>,
    /// Error message if operation failed
    pub error: Option<String>,
    /// Standardized error code for programmatic handling
    pub error_code: Option<String>,
    /// Operation metadata
    pub metadata: OperationMetadata,
    
    // === New version fields ===
    
    /// Current version of the resource (for ETag response header)
    pub current_version: Option<ETag>,
    /// Version conflict details (for 412 responses)
    pub version_conflict: Option<VersionConflict>,
}

/// Version conflict information for 412 responses.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionConflict {
    /// The ETag that was expected
    pub expected: ETag,
    /// The current ETag of the resource
    pub current: ETag,
    /// Human-readable conflict message
    pub message: String,
}

impl<P: ResourceProvider> ScimOperationHandler<P> {
    /// Handle update operations with conditional support.
    async fn handle_update(
        &self,
        request: ScimOperationRequest,
        context: &RequestContext,
    ) -> ScimResult<ScimOperationResponse> {
        let resource_id = request.resource_id.ok_or_else(|| {
            ScimError::invalid_request("Missing resource_id for update operation".to_string())
        })?;
        
        let data = request.data.ok_or_else(|| {
            ScimError::invalid_request("Missing data for update operation".to_string())
        })?;
        
        // Check if provider supports conditional operations and client provided ETag
        if self.server.provider().supports_conditional_operations() && request.expected_version.is_some() {
            // Use conditional update
            let result = self.server.provider()
                .update_resource_conditional(
                    &request.resource_type,
                    &resource_id,
                    data,
                    request.expected_version.as_ref(),
                    context,
                )
                .await?;
                
            match result {
                ConditionalUpdateResult::Updated(resource) => {
                    Ok(ScimOperationResponse {
                        success: true,
                        data: Some(resource.to_json()?),
                        error: None,
                        error_code: None,
                        metadata: self.build_metadata(&request, &resource, 1),
                        current_version: resource.meta().and_then(|m| m.version().map(|v| v.clone())),
                        version_conflict: None,
                    })
                }
                ConditionalUpdateResult::VersionMismatch { expected, current } => {
                    Ok(ScimOperationResponse {
                        success: false,
                        data: None,
                        error: Some("Version mismatch - resource has been modified".to_string()),
                        error_code: Some("PRECONDITION_FAILED".to_string()),
                        metadata: self.build_metadata(&request, None, 0),
                        current_version: Some(current.clone()),
                        version_conflict: Some(VersionConflict {
                            expected,
                            current,
                            message: "Resource version does not match expected version".to_string(),
                        }),
                    })
                }
                ConditionalUpdateResult::NotFound => {
                    Ok(ScimOperationResponse {
                        success: false,
                        data: None,
                        error: Some("Resource not found".to_string()),
                        error_code: Some("RESOURCE_NOT_FOUND".to_string()),
                        metadata: self.build_metadata(&request, None, 0),
                        current_version: None,
                        version_conflict: None,
                    })
                }
            }
        } else {
            // Fall back to regular update
            let resource = self.server
                .update_resource(&request.resource_type, &resource_id, data, context)
                .await?;
                
            Ok(ScimOperationResponse {
                success: true,
                data: Some(resource.to_json()?),
                error: None,
                error_code: None,
                metadata: self.build_metadata(&request, &resource, 1),
                current_version: resource.meta().and_then(|m| m.version().map(|v| v.clone())),
                version_conflict: None,
            })
        }
    }
}

impl ScimOperationRequest {
    /// Create an update request with conditional version.
    pub fn update_conditional(
        resource_type: impl Into<String>,
        resource_id: impl Into<String>,
        data: Value,
        expected_version: ETag,
    ) -> Self {
        Self {
            operation: ScimOperationType::Update,
            resource_type: resource_type.into(),
            resource_id: Some(resource_id.into()),
            data: Some(data),
            query: None,
            tenant_context: None,
            request_id: None,
            expected_version: Some(expected_version),
            if_none_match: None,
        }
    }
}
```

### Phase 4: Meta Integration

**File**: `src/resource/value_objects/meta.rs`

```rust
/// Enhanced Meta with ETag integration.
pub struct Meta {
    #[serde(rename = "resourceType")]
    pub resource_type: String,
    pub created: DateTime<Utc>,
    #[serde(rename = "lastModified")]
    pub last_modified: DateTime<Utc>,
    pub location: Option<String>,
    
    // Changed from Option<String> to Option<ETag>
    pub version: Option<ETag>,
}

impl Meta {
    /// Create Meta with ETag version.
    pub fn new(
        resource_type: String,
        created: DateTime<Utc>,
        last_modified: DateTime<Utc>,
        location: Option<String>,
        version: Option<ETag>,
    ) -> ValidationResult<Self> {
        Self::validate_resource_type(&resource_type)?;
        Self::validate_timestamps(created, last_modified)?;
        if let Some(ref location_val) = location {
            Self::validate_location(location_val)?;
        }
        
        Ok(Self {
            resource_type,
            created,
            last_modified,
            location,
            version,
        })
    }
    
    /// Generate new Meta with weak ETag for resource.
    pub fn with_generated_etag(
        resource_type: String,
        resource_id: &str,
        created: DateTime<Utc>,
        last_modified: DateTime<Utc>,
        location: Option<String>,
    ) -> ValidationResult<Self> {
        let etag = ETag::generate_weak_scim(resource_id, last_modified);
        Self::new(resource_type, created, last_modified, location, Some(etag))
    }
    
    /// Get the ETag version.
    pub fn version(&self) -> Option<&ETag> {
        self.version.as_ref()
    }
    
    /// Create new Meta with updated timestamp and ETag.
    pub fn with_update(self, resource_id: &str) -> Self {
        let now = Utc::now();
        let new_etag = ETag::generate_weak_scim(resource_id, now);
        
        Self {
            resource_type: self.resource_type,
            created: self.created,
            last_modified: now,
            location: self.location,
            version: Some(new_etag),
        }
    }
}
```

## HTTP Integration Pattern

### Client Request Pattern

```rust
// HTTP handler extracting ETag from headers
async fn handle_put_user(
    Path(user_id): Path<String>,
    headers: HeaderMap,
    Json(user_data): Json<Value>,
) -> Result<impl IntoResponse, ScimError> {
    // Extract If-Match header
    let expected_version = headers
        .get("if-match")
        .and_then(|h| h.to_str().ok())
        .and_then(|s| ETag::try_from(s).ok());
    
    // Create conditional request
    let request = if let Some(etag) = expected_version {
        ScimOperationRequest::update_conditional("User", user_id, user_data, etag)
    } else {
        ScimOperationRequest::update("User", user_id, user_data)
    };
    
    // Execute operation
    let response = handler.handle_operation(request).await;
    
    // Map to HTTP response
    match response.success {
        true => {
            let mut http_response = Json(response.data.unwrap()).into_response();
            
            // Set ETag header if available
            if let Some(version) = response.current_version {
                http_response.headers_mut().insert(
                    "etag",
                    HeaderValue::from_str(version.as_str()).unwrap(),
                );
            }
            
            Ok(http_response)
        }
        false if response.error_code == Some("PRECONDITION_FAILED".to_string()) => {
            let mut error_response = (
                StatusCode::PRECONDITION_FAILED,
                Json(json!({
                    "schemas": ["urn:ietf:params:scim:api:messages:2.0:Error"],
                    "status": "412",
                    "scimType": "invalidVersion",
                    "detail": response.error.unwrap_or_default()
                }))
            ).into_response();
            
            // Include current ETag in response
            if let Some(version) = response.current_version {
                error_response.headers_mut().insert(
                    "etag",
                    HeaderValue::from_str(version.as_str()).unwrap(),
                );
            }
            
            Ok(error_response)
        }
        false => {
            // Handle other errors...
        }
    }
}
```

## Provider Implementation Examples

### In-Memory Provider with CAS

```rust
impl ResourceProvider for InMemoryProvider {
    fn supports_conditional_operations(&self) -> bool {
        true
    }
    
    async fn update_resource_conditional(
        &self,
        resource_type: &str,
        id: &str,
        data: Value,
        expected_version: Option<&ETag>,
        context: &RequestContext,
    ) -> Result<ConditionalUpdateResult, Self::Error> {
        let mut resources = self.resources.write().await;
        let tenant_key = self.make_key(resource_type, id, context);
        
        // Get current resource
        let current_resource = resources.get(&tenant_key)
            .ok_or_else(|| ProviderError::NotFound)?;
        
        // Extract current version
        let current_version = current_resource
            .meta()
            .and_then(|m| m.version())
            .ok_or_else(|| ProviderError::NoVersion)?;
        
        // Check version if expected version provided
        if let Some(expected) = expected_version {
            if !expected.matches(current_version) {
                return Ok(ConditionalUpdateResult::VersionMismatch {
                    expected: expected.clone(),
                    current: current_version.clone(),
                });
            }
        }
        
        // Perform update with new ETag
        let mut updated_resource = Resource::from_json(data, resource_type)?;
        let new_meta = current_resource.meta()
            .unwrap()
            .clone()
            .with_update(id);
        updated_resource.set_meta(new_meta);
        
        // Atomic update
        resources.insert(tenant_key, updated_resource.clone());
        
        Ok(ConditionalUpdateResult::Updated(updated_resource))
    }
}
```

### Database Provider with Optimistic Locking

```rust
impl ResourceProvider for DatabaseProvider {
    fn supports_conditional_operations(&self) -> bool {
        true
    }
    
    async fn update_resource_conditional(
        &self,
        resource_type: &str,
        id: &str,
        data: Value,
        expected_version: Option<&ETag>,
        context: &RequestContext,
    ) -> Result<ConditionalUpdateResult, Self::Error> {
        let mut tx = self.pool.begin().await?;
        
        // Get current resource with version
        let current_row = sqlx::query!(
            "SELECT data, version FROM scim_resources WHERE id = $1 AND tenant_id = $2 FOR UPDATE",
            id,
            context.tenant_id().unwrap_or("default")
        )
        .fetch_optional(&mut tx)
        .await?;
        
        let current_row = match current_row {
            Some(row) => row,
            None => return Ok(ConditionalUpdateResult::NotFound),
        };
        
        // Parse current version
        let current_version = ETag::try_from(current_row.version)
            .map_err(|_| ProviderError::InvalidVersion)?;
        
        // Check version if expected version provided
        if let Some(expected) = expected_version {
            if !expected.matches(&current_version) {
                return Ok(ConditionalUpdateResult::VersionMismatch {
                    expected: expected.clone(),
                    current: current_version,
                });
            }
        }
        
        // Generate new version
        let now = Utc::now();
        let new_version = ETag::generate_weak_scim(id, now);
        
        // Update resource with new version
        let updated_resource = self.merge_resource_data(current_row.data, data, &new_version)?;
        
        // Atomic database update
        sqlx::query!(
            "UPDATE scim_resources SET data = $1, version = $2, last_modified = $3 WHERE id = $4 AND tenant_id = $5",
            serde_json::to_string(&updated_resource)?,
            new_version.as_str(),
            now,
            id,
            context.tenant_id().unwrap_or("default")
        )
        .execute(&mut tx)
        .await?;
        
        tx.commit().await?;
        
        Ok(ConditionalUpdateResult::Updated(updated_resource))
    }
}
```

## Migration Strategy

### Breaking Change Management

**Timeline**: 2-3 weeks implementation + 2 weeks migration window

**Phase 1: Preparation (Week 1)**
- Implement ETag value object
- Add conditional methods to ResourceProvider trait (with default implementations)
- Update Meta struct (breaking change to version field type)

**Phase 2: Integration (Week 2)**
- Update operation handlers
- Add conditional request support
- Update examples and documentation

**Phase 3: Migration Support (Week 3)**
- Release with breaking change warning
- Provide migration guide
- Support existing implementations during transition

**Phase 4: Migration Window (Weeks 4-5)**
- Community migration period
- Support and assistance for provider updates
- Address migration issues

### Provider Migration Guide

**Current Provider:**
```rust
impl ResourceProvider for MyProvider {
    async fn update_resource(&self, resource_type: &str, id: &str, data: Value, context: &RequestContext) -> Result<Resource, Self::Error> {
        // Current implementation
    }
}
```

**Migrated Provider:**
```rust
impl ResourceProvider for MyProvider {
    // Existing method unchanged
    async fn update_resource(&self, resource_type: &str, id: &str, data: Value, context: &RequestContext) -> Result<Resource, Self::Error> {
        // Existing implementation unchanged
    }
    
    // New conditional support (opt-in)
    fn supports_conditional_operations(&self) -> bool {
        true  // Enable conditional operations
    }
    
    async fn update_resource_conditional(
        &self,
        resource_type: &str,
        id: &str,
        data: Value,
        expected_version: Option<&ETag>,
        context: &RequestContext,
    ) -> Result<ConditionalUpdateResult, Self::Error> {
        // New conditional implementation
        // Can reuse existing update_resource logic with version checking
    }
}
```

## Testing Strategy

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_etag_validation() {
        // Valid weak ETag
        assert!(ETag::new("W/\"123-456\"".to_string()).is_ok());
        
        // Valid strong ETag
        assert!(ETag::new("\"exact-match\"".to_string()).is_ok());
        
        // Invalid format
        assert!(ETag::new("invalid".to_string()).is_err());
    }
    
    #[test]
    fn test_etag_matching() {
        let weak1 = ETag::weak("123".to_string()).unwrap();
        let weak2 = ETag::weak("123".to_string()).unwrap();
        let weak3 = ETag::weak("456".to_string()).unwrap();
        
        assert!(weak1.matches(&weak2));  // Same opaque value
        assert!(!weak1.matches(&weak3)); // Different opaque value
    }
    
    #[tokio::test]
    async fn test_conditional_update_success() {
        let provider = TestProvider::new();
        let context = RequestContext::default();
        
        // Create resource
        let resource = provider.create_resource("User", test_user_data(), &context).await.unwrap();
        let etag = resource.meta().unwrap().version().unwrap().clone();
        
        // Conditional update with correct ETag
        let result = provider.update_resource_conditional(
            "User",
            resource.id().unwrap(),
            updated_user_data(),
            Some(&etag),
            &context,
        ).await.unwrap();
        
        assert!(matches!(result, ConditionalUpdateResult::Updated(_)));
    }
    
    #[tokio::test]
    async fn test_conditional_update_conflict() {
        let provider = TestProvider::new();
        let context = RequestContext::default();
        
        // Create resource
        let resource = provider.create_resource("User", test_user_data(), &context).await.unwrap();
        let old_etag = resource.meta().unwrap().version().unwrap().clone();
        
        // Update resource (changes ETag)
        provider.update_resource("User", resource.id().unwrap(), updated_user_data(), &context).await.unwrap();
        
        // Conditional update with old ETag should fail
        let result = provider