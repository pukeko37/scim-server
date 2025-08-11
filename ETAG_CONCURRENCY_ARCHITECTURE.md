# SCIM ETag Concurrency Control - Architecture Design Document

**Version**: 0.2.0  
**Date**: December 2024  
**Status**: Final Design - Ready for Implementation

## Executive Summary

This document defines the architecture for implementing RFC 7232-compliant ETag-based optimistic concurrency control in the SCIM server library. The design addresses the critical gap of concurrent modification protection while maintaining clean architectural boundaries between HTTP clients, the SCIM server library, and storage providers.

## Problem Statement

### Current State: No Concurrency Control

The SCIM server library currently provides **no protection against concurrent modifications**, making it unsuitable for production scenarios with multiple clients accessing the same resources.

**Data Loss Scenario:**
```
Time: T1
Client A: GET /Users/123 → { "displayName": "John Doe", "meta": { "version": "W/\"abc123\"" } }
Client B: GET /Users/123 → { "displayName": "John Doe", "meta": { "version": "W/\"abc123\"" } }

Time: T2
Client A: PUT /Users/123, If-Match: W/"abc123" → Success
Client B: PUT /Users/123, If-Match: W/"abc123" → Should fail, but currently succeeds

Result: Client A's changes are lost (last-write-wins)
```

## Design Principles

### Architectural Boundaries

**✅ Clear Separation of Responsibilities:**
- **HTTP/MCP Clients**: Handle protocol-specific headers and message formats
- **SCIM Server Library**: Manage SCIM compliance, Resource/Meta construction
- **Providers**: Control version generation and storage-specific concurrency

**✅ Multiple Client Support:**
- HTTP clients use `If-Match`/`ETag` headers
- MCP clients use JSON version fields
- Direct library usage uses native types

**✅ Provider Flexibility:**
- Each provider chooses optimal version generation strategy
- Storage-specific concurrency mechanisms (SQL transactions, atomic operations)

## RFC Compliance Analysis

### RFC 7643 Requirements

```
"version: The version of the resource being returned. This value
must be the same as the entity-tag (ETag) HTTP response header
(see Sections 2.1 and 2.3 of [RFC7232]). This attribute has
"caseExact" as "true"."
```

### RFC 7232 Weak vs Strong Analysis

**Historical Context**: HTTP ETags predate SCIM (1997 vs 2015). SCIM adopted ETags wholesale without filtering for identity management needs.

**Evidence for Weak-Only Design:**
- ✅ All RFC 7643 examples use weak ETags: `W/"3694e05e9dff591"`
- ✅ RFC expects most implementations to use weak: "if... does not satisfy all characteristics of a strong validator... MUST mark as weak"
- ✅ JSON identity data benefits from semantic equivalence, not byte-level matching
- ✅ Strong ETags designed for HTTP caching/range requests, not applicable to SCIM

**Decision**: **Weak-only ETag design** - Strong ETags add complexity without value for identity management.

## Core Architecture

### System Layer Boundaries

```
┌─────────────────────────────────────────────────────────────────┐
│  CLIENT LAYER (Multiple Types)                                 │
├─────────────────────────────────────────────────────────────────┤
│  • HTTP Client          │  • MCP Client         │  • Direct Use  │
│  - If-Match headers     │  - JSON version       │  - Native types │
│  - ETag responses       │  - Structured errors  │  - Direct calls │
└─────────────────────────────────────────────────────────────────┘
                                  │
                                  ▼
┌─────────────────────────────────────────────────────────────────┐
│  SCIM SERVER LAYER (scim-server crate responsibility)          │
├─────────────────────────────────────────────────────────────────┤
│  • Resource/Meta construction from provider data               │
│  • SCIM schema compliance and JSON formatting                  │
│  • Version type conversion (ScimVersion ↔ String)              │
│  • Operation coordination and response formatting              │
└─────────────────────────────────────────────────────────────────┘
                                  │
                                  ▼
┌─────────────────────────────────────────────────────────────────┐
│  PROVIDER LAYER (user-implemented + InMemoryProvider)          │
├─────────────────────────────────────────────────────────────────┤
│  • Version generation using storage-optimal strategy           │
│  • Atomic version validation and resource updates              │
│  • Storage-specific concurrency control                        │
│  • Raw data + version string return                            │
└─────────────────────────────────────────────────────────────────┘
```

### Version Lifecycle Ownership

**Provider Responsibilities:**
- Generate unique version identifiers using optimal strategy for their storage
- Perform atomic version validation during updates
- Return raw resource data + version string
- Handle storage-specific concurrency mechanisms

**SCIM Server Responsibilities:**
- Construct Resource objects from provider data
- Create Meta objects with provider's version string
- Handle SCIM protocol compliance and response formatting
- Convert between HTTP headers and internal types

## Technical Implementation

### 1. ScimVersion Value Object

```rust
/// SCIM weak ETag version identifier
/// 
/// Always generates RFC 7232 compliant weak ETags: W/"opaque-value"
/// The opaque value is provider-controlled for optimal uniqueness
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ScimVersion {
    opaque: String,  // The unique identifier part (without W/" wrapper)
}

impl ScimVersion {
    /// Create version from provider's unique identifier
    pub fn from_provider_id(provider_unique_id: &str) -> ValidationResult<Self>;
    
    /// Helper: Create version from hash of any hashable data
    /// Uses Rust's built-in DefaultHasher for simplicity and performance
    pub fn from_hash<T: Hash>(hashable_data: &T) -> Self;
    
    /// Parse from HTTP ETag header: W/"abc123" -> ScimVersion
    pub fn parse_http_header(etag_header: &str) -> ValidationResult<Self>;
    
    /// Convert to HTTP ETag header: ScimVersion -> W/"abc123"
    pub fn to_http_header(&self) -> String;
    
    /// SCIM weak comparison (opaque value equality)
    pub fn matches(&self, other: &ScimVersion) -> bool;
}
```

**Design Rationale:**
- **Weak-only**: Simplifies implementation, matches RFC examples
- **Provider flexibility**: Accepts any unique string (UUIDs, counters, hashes)
- **Rust hash helper**: Zero dependencies, optimal performance
- **Type safety**: Validates ETag format, prevents invalid versions

### 2. Enhanced Provider Interface

```rust
trait ResourceProvider {
    type Error: std::error::Error + Send + Sync + 'static;
    
    /// Create resource - provider returns data + generated version
    async fn create_resource(
        &self,
        resource_type: &str,
        data: Value,
        context: &RequestContext,
    ) -> Result<(Value, ScimVersion), Self::Error>;
    
    /// Update with optional version checking
    async fn update_resource(
        &self,
        resource_type: &str,
        id: &str,
        data: Value,
        expected_version: Option<&ScimVersion>,  // None = unconditional
        context: &RequestContext,
    ) -> Result<ConditionalResult<(Value, ScimVersion)>, Self::Error>;
    
    /// Get resource with current version
    async fn get_resource(
        &self,
        resource_type: &str,
        id: &str,
        context: &RequestContext,
    ) -> Result<Option<(Value, ScimVersion)>, Self::Error>;
    
    /// Delete with optional version checking  
    async fn delete_resource(
        &self,
        resource_type: &str,
        id: &str,
        expected_version: Option<&ScimVersion>,
        context: &RequestContext,
    ) -> Result<ConditionalResult<()>, Self::Error>;
}

#[derive(Debug, Clone)]
pub enum ConditionalResult<T> {
    Success(T),
    VersionMismatch {
        expected: ScimVersion,
        current: ScimVersion,
        message: String,
    },
    NotFound,
}
```

**Design Rationale:**
- **Unified interface**: Single method handles conditional/unconditional operations
- **Provider returns versions**: Full control over version generation strategy
- **Raw data return**: Provider doesn't need to know SCIM Resource/Meta structure
- **Atomic operations**: Provider can ensure version check + update atomicity

### 3. Provider Implementation Strategies

#### InMemoryProvider (Hash-based)
```rust
impl ResourceProvider for InMemoryProvider {
    async fn create_resource(&self, ...) -> Result<(Value, ScimVersion), Self::Error> {
        let resource_id = self.generate_resource_id(context.tenant_id()).await;
        
        let mut resource_data = data;
        resource_data["id"] = json!(resource_id);
        
        // Hash-based versioning for content consistency
        let hash_input = (
            &resource_data,
            resource_type,
            context.tenant_id(),
            Utc::now().timestamp_millis()
        );
        let version = ScimVersion::from_hash(&hash_input);
        
        // Store with atomic write lock
        let mut data_guard = self.data.write().await;
        self.store_with_version(&mut data_guard, resource_type, key, resource_data.clone(), version.clone());
        
        Ok((resource_data, version))
    }
    
    async fn update_resource(&self, ..., expected_version: Option<&ScimVersion>, ...) 
        -> Result<ConditionalResult<(Value, ScimVersion)>, Self::Error> {
        
        let mut data_guard = self.data.write().await;  // Atomic operation
        
        // Get current version
        let current_version = self.get_version(&data_guard, resource_type, &key)
            .ok_or(InMemoryError::ResourceNotFound { ... })?;
        
        // Version check
        if let Some(expected) = expected_version {
            if !expected.matches(&current_version) {
                return Ok(ConditionalResult::VersionMismatch {
                    expected: expected.clone(),
                    current: current_version,
                    message: format!("Resource {} has been modified", id),
                });
            }
        }
        
        // Generate new version + update (atomic under write lock)
        let updated_data = /* ... */;
        let new_version = ScimVersion::from_hash(&(/* updated data + timestamp */));
        self.store_with_version(&mut data_guard, resource_type, key, updated_data.clone(), new_version.clone());
        
        Ok(ConditionalResult::Success((updated_data, new_version)))
    }
}
```

#### DatabaseProvider (Sequential numbers)
```rust
impl ResourceProvider for DatabaseProvider {
    async fn create_resource(&self, ...) -> Result<(Value, ScimVersion), Self::Error> {
        // Database auto-generates sequential revision
        let query = "INSERT INTO resources (tenant_id, resource_type, data) VALUES ($1, $2, $3) RETURNING id, revision";
        let row = sqlx::query(query)
            .bind(context.tenant_id())
            .bind(resource_type)
            .bind(&data)
            .fetch_one(&self.pool)
            .await?;
        
        let resource_id: String = row.get("id");
        let revision: i64 = row.get("revision");
        
        // Use database revision number
        let version = ScimVersion::from_provider_id(&revision.to_string())?;
        
        let mut resource_data = data;
        resource_data["id"] = json!(resource_id);
        
        Ok((resource_data, version))
    }
    
    async fn update_resource(&self, ..., expected_version: Option<&ScimVersion>, ...) 
        -> Result<ConditionalResult<(Value, ScimVersion)>, Self::Error> {
        
        let mut tx = self.pool.begin().await?;
        
        // Get current revision with row lock
        let current_revision: i64 = sqlx::query_scalar(
            "SELECT revision FROM resources WHERE id = $1 FOR UPDATE"
        )
        .bind(id)
        .fetch_one(&mut *tx)
        .await?;
        
        let current_version = ScimVersion::from_provider_id(&current_revision.to_string())?;
        
        // Version check
        if let Some(expected) = expected_version {
            if !expected.matches(&current_version) {
                tx.rollback().await?;
                return Ok(ConditionalResult::VersionMismatch {
                    expected: expected.clone(),
                    current: current_version,
                    message: format!("Resource {} has been modified", id),
                });
            }
        }
        
        // Atomic update with new revision
        let new_revision: i64 = sqlx::query_scalar(
            "UPDATE resources SET data = $1, revision = revision + 1 WHERE id = $2 RETURNING revision"
        )
        .bind(&data)
        .bind(id)
        .fetch_one(&mut *tx)
        .await?;
        
        tx.commit().await?;
        
        let new_version = ScimVersion::from_provider_id(&new_revision.to_string())?;
        let mut updated_data = data;
        updated_data["id"] = json!(id);
        
        Ok(ConditionalResult::Success((updated_data, new_version)))
    }
}
```

### 4. SCIM Server Integration

```rust
impl ScimServer<P> {
    pub async fn create_resource(
        &self,
        resource_type: &str,
        data: Value,
        context: &RequestContext,
    ) -> ScimResult<Resource> {
        // 1. Provider generates data + version
        let (resource_data, version) = self.provider
            .create_resource(resource_type, data, context)
            .await?;
        
        // 2. SCIM Server constructs Resource from provider data
        let mut resource = Resource::from_json(resource_type.to_string(), resource_data)?;
        
        // 3. SCIM Server creates Meta with provider's version
        let now = Utc::now();
        let meta = Meta::new(
            resource_type.to_string(),
            now, now,
            self.generate_location(resource_type, &resource.id.as_ref().unwrap().to_string()),
            Some(version.to_http_header()),  // ScimVersion -> String
        )?;
        resource.set_meta(meta);
        
        Ok(resource)
    }
    
    pub async fn update_resource(
        &self,
        resource_type: &str,
        id: &str,
        data: Value,
        if_match_header: Option<&str>,  // From HTTP If-Match header
        context: &RequestContext,
    ) -> ScimResult<ConditionalResult<Resource>> {
        // Parse version from HTTP header if provided
        let expected_version = if let Some(header) = if_match_header {
            Some(ScimVersion::parse_http_header(header)?)
        } else {
            None
        };
        
        let result = self.provider
            .update_resource(resource_type, id, data, expected_version.as_ref(), context)
            .await?;
        
        match result {
            ConditionalResult::Success((updated_data, new_version)) => {
                let mut resource = Resource::from_json(resource_type.to_string(), updated_data)?;
                
                // SCIM Server updates Meta with new version
                let updated_meta = Meta::new(
                    resource_type.to_string(),
                    resource.meta.as_ref().unwrap().created,  // preserve created
                    Utc::now(),
                    resource.meta.as_ref().and_then(|m| m.location.clone()),
                    Some(new_version.to_http_header()),  // ScimVersion -> String
                )?;
                resource.set_meta(updated_meta);
                
                Ok(ConditionalResult::Success(resource))
            },
            ConditionalResult::VersionMismatch { expected, current, message } => {
                Ok(ConditionalResult::VersionMismatch { expected, current, message })
            },
            ConditionalResult::NotFound => {
                Ok(ConditionalResult::NotFound)
            }
        }
    }
}
```

### 5. Operation Handler Integration

```rust
/// Enhanced operation request with conditional support
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
    /// Expected version for conditional operations (from If-Match header)
    pub expected_version: Option<ScimVersion>,
}

/// Enhanced operation response with version information
#[derive(Debug, Serialize, Deserialize)]
pub struct ScimOperationResponse {
    /// Whether the operation succeeded
    pub success: bool,
    /// Response data (resource or list of resources)
    pub data: Option<Value>,
    /// Error message if operation failed
    pub error: Option<String>,
    /// Error code for programmatic handling
    pub error_code: Option<String>,
    /// Operation metadata
    pub metadata: OperationMetadata,
    /// Current resource version (for conflict resolution)
    pub current_version: Option<ScimVersion>,
    /// Version conflict information
    pub version_conflict: Option<VersionConflict>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionConflict {
    pub expected: ScimVersion,
    pub current: ScimVersion,
    pub message: String,
}
```

## End-to-End Flow Examples

### HTTP Client Conditional Update
```
1. Client reads resource:
   GET /Users/123
   ← 200 OK, ETag: W/"abc123", { "meta": { "version": "W/\"abc123\"" } }

2. Client modifies resource:
   PUT /Users/123, If-Match: W/"abc123", { updated data }

3. HTTP Layer → SCIM Server:
   ScimOperationRequest {
     operation: Update,
     resource_id: "123",
     data: { updated data },
     expected_version: Some(ScimVersion::parse_http_header("W/\"abc123\"")),
   }

4. SCIM Server → Provider:
   provider.update_resource("User", "123", data, Some(&version), context)

5. Provider validates version atomically:
   - Current version: W/"abc123" → matches expected
   - Update data + generate new version: W/"def456"
   - Return: ConditionalResult::Success((data, new_version))

6. SCIM Server → HTTP Layer:
   ScimOperationResponse {
     success: true,
     data: { updated resource },
     current_version: Some(ScimVersion("def456")),
   }

7. HTTP Layer → Client:
   200 OK, ETag: W/"def456", { updated resource with meta.version: "W/\"def456\"" }
```

### MCP Client Conditional Update
```
1. AI Agent reads resource:
   execute_tool("scim_get_user", {"user_id": "123"})
   ← { "success": true, "data": {...}, "current_version": "W/\"abc123\"" }

2. AI Agent modifies resource:
   execute_tool("scim_update_user", {
     "user_id": "123",
     "user_data": {...},
     "expected_version": "W/\"abc123\""
   })

3. MCP Layer → SCIM Server:
   ScimOperationRequest {
     operation: Update,
     resource_id: "123",
     data: {...},
     expected_version: Some(ScimVersion::parse_http_header("W/\"abc123\"")),
   }

4-5. [Same as HTTP flow]

6. SCIM Server → MCP Layer:
   ScimOperationResponse { success: true, current_version: Some(...) }

7. MCP Layer → AI Agent:
   { "success": true, "data": {...}, "current_version": "W/\"def456\"" }
```

### Version Conflict Scenario
```
1. Two clients read same resource version: W/"abc123"

2. Client A updates first:
   PUT /Users/123, If-Match: W/"abc123" → 200 OK, ETag: W/"def456"

3. Client B attempts update:
   PUT /Users/123, If-Match: W/"abc123"

4. Provider detects version mismatch:
   - Expected: W/"abc123"
   - Current: W/"def456"
   - Return: ConditionalResult::VersionMismatch

5. Client B receives:
   412 Precondition Failed, ETag: W/"def456"
   
6. Client B can retry with current version
```

## Migration Strategy

### Phase 1: Foundation (Non-Breaking)
- ✅ Add `ScimVersion` value object
- ✅ Add conditional methods to `ResourceProvider` trait (with default implementations)
- ✅ Update `Cargo.toml` with minimal dependencies
- ✅ Implement `ScimVersion` comprehensive tests

### Phase 2: Provider Enhancement
- ✅ Implement conditional operations in `InMemoryProvider`
- ✅ Add version generation to resource creation/updates
- ✅ Update existing provider tests
- ✅ Add version conflict test scenarios

### Phase 3: Integration
- ✅ Update `ScimOperationHandler` for version support
- ✅ Extend `ScimOperationRequest`/`ScimOperationResponse`
- ✅ Add HTTP integration examples
- ✅ Add MCP integration examples

### Phase 4: Documentation & Examples
- ✅ Document HTTP integration patterns
- ✅ Document custom provider implementation
- ✅ Add comprehensive examples
- ✅ Performance benchmarks

## Dependencies

### Required Dependencies
```toml
# No additional dependencies required!
# Uses existing dependencies + Rust standard library

[dependencies]
serde = { version = "1.0.215", features = ["derive"] }
serde_json = "1.0.132"
tokio = { version = "1.47.0", features = ["full"] }
uuid = { version = "1.11.0", features = ["v4"] }  # Optional, if provider chooses UUIDs
thiserror = "2.0.9"
chrono = { version = "0.4.38", features = ["serde"] }
base64 = "0.21"
log = "0.4"
```

**Design Decision**: Use Rust's built-in `DefaultHasher` instead of cryptographic hashes for simplicity and performance.

## Testing Strategy

### Unit Tests
- `ScimVersion` creation, parsing, and comparison
- Provider version generation strategies
- Conditional result handling
- Error cases and validation

### Integration Tests
- Complete conditional update flows
- Multi-client conflict scenarios
- Provider-specific concurrency tests
- HTTP/MCP integration examples

### Performance Tests
- Version generation overhead
- Concurrent update throughput
- Memory usage with version storage

## Benefits

### For SCIM Library Users
- ✅ **Production Ready**: Robust concurrent modification protection
- ✅ **Multiple Client Support**: Works with HTTP, MCP, and direct usage
- ✅ **Provider Flexibility**: Optimize versioning for specific storage backends
- ✅ **Standards Compliant**: Full RFC 7232/7643 compliance

### For Library Maintainers
- ✅ **Clean Architecture**: Clear separation of concerns across layers
- ✅ **Minimal Dependencies**: Uses Rust standard library, no crypto overhead
- ✅ **Extensible Design**: Easy to add new client types or provider strategies
- ✅ **Type Safety**: Compile-time prevention of version-related bugs

## Security Considerations

### Version Predictability
- Using hashes/UUIDs prevents version guessing attacks
- Provider can choose appropriate uniqueness strategy for security requirements

### Denial of Service
- Version conflicts are handled gracefully without resource exhaustion
- Failed operations don't consume excessive resources

### Information Disclosure
- Version tokens don't leak internal storage details
- Opaque identifiers prevent inference of resource change patterns

---

**Status**: Ready for implementation  
**Next Steps**: Begin Phase 1 implementation starting with `ScimVersion` value object