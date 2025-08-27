# MCP Version Management for AI Agents

## Overview

The SCIM server provides version-based optimistic concurrency control for AI agents through simple raw version strings. This prevents lost updates and accidental deletions in concurrent scenarios.

## How Versions Work

### Version Format
- **Raw version strings**: Simple identifiers like `"abc123def"`
- **Content-based**: Versions are generated from resource content (SHA-256 hash)
- **Deterministic**: Same content always produces same version

### Version Sources
Every operation that retrieves or modifies a user returns version information in two places:

1. **Metadata**: `metadata.version` - Primary version field for conditional operations
2. **Content**: `_version` field - Embedded in resource for convenience

## Basic Operations

### Creating Users
```json
{
  "name": "scim_create_user",
  "arguments": {
    "user_data": {
      "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
      "userName": "alice@company.com"
    }
  }
}
```

**Response includes version**:
```json
{
  "success": true,
  "content": {
    "id": "user123",
    "userName": "alice@company.com",
    "_version": "abc123def"
  },
  "metadata": {
    "version": "abc123def",
    "resource_id": "user123"
  }
}
```

### Getting Users
```json
{
  "name": "scim_get_user", 
  "arguments": {
    "user_id": "user123"
  }
}
```

**Response includes current version**:
```json
{
  "content": {
    "id": "user123",
    "userName": "alice@company.com", 
    "_version": "xyz789ghi"
  },
  "metadata": {
    "version": "xyz789ghi"
  }
}
```

## Conditional Operations

### Safe Updates
Use the `expected_version` parameter to prevent lost updates:

```json
{
  "name": "scim_update_user",
  "arguments": {
    "user_id": "user123",
    "user_data": {
      "id": "user123",
      "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
      "userName": "alice.updated@company.com"
    },
    "expected_version": "xyz789ghi"
  }
}
```

**Success**: Update proceeds and returns new version
**Version Mismatch**: Returns error with `VERSION_MISMATCH` code

### Safe Deletions
```json
{
  "name": "scim_delete_user",
  "arguments": {
    "user_id": "user123",
    "expected_version": "xyz789ghi"  
  }
}
```

## Error Handling

### Version Mismatch
When expected version doesn't match current version:

```json
{
  "success": false,
  "content": {
    "error": "Version mismatch: resource was modified",
    "error_code": "VERSION_MISMATCH"
  }
}
```

**Recovery Strategy**:
1. Get current resource state with `scim_get_user`
2. Examine current data and version
3. Apply your changes to current state
4. Retry operation with current version

### Invalid Version Format
```json
{
  "success": false,
  "content": {
    "error": "Invalid expected_version format: 'W/\"abc123\"'. Use raw version (e.g., 'abc123def')",
    "error_code": "INVALID_VERSION_FORMAT"
  }
}
```

## Best Practices for AI Agents

### 1. Always Use Versions for Updates/Deletes
```javascript
// ✅ GOOD - Version-safe update
const user = await scim_get_user({user_id: "user123"});
await scim_update_user({
  user_id: "user123", 
  user_data: {
    ...user.content,
    active: false
  },
  expected_version: user.metadata.version
});

// ❌ BAD - Unsafe update (race conditions possible)
await scim_update_user({
  user_id: "user123",
  user_data: {active: false}
  // No expected_version!
});
```

### 2. Handle Version Conflicts Gracefully
```javascript
try {
  await scim_update_user({
    user_id: userId,
    user_data: updatedData,
    expected_version: expectedVersion
  });
} catch (error) {
  if (error.error_code === "VERSION_MISMATCH") {
    // Get fresh data and retry
    const current = await scim_get_user({user_id: userId});
    // Merge changes and retry with current version
    await scim_update_user({
      user_id: userId,
      user_data: mergeChanges(current.content, updatedData),
      expected_version: current.metadata.version
    });
  }
}
```

### 3. Store Versions for Later Use
```javascript
// Store version when retrieving user
const user = await scim_get_user({user_id: "user123"});
const userVersion = user.metadata.version;

// Use stored version for conditional operations later
await scim_delete_user({
  user_id: "user123",
  expected_version: userVersion
});
```

## Multi-Tenant Considerations

Versions are scoped to tenants - the same user ID in different tenants will have independent versions:

```json
{
  "name": "scim_update_user",
  "arguments": {
    "user_id": "user123",
    "user_data": {...},
    "expected_version": "tenant1version",
    "tenant_id": "tenant1"
  }
}
```

## Technical Details

- **Hash Algorithm**: SHA-256 of complete resource JSON
- **Encoding**: Base64 (first 8 bytes for shorter versions)
- **Thread Safety**: Versions enable optimistic locking across concurrent operations
- **Transport**: Raw strings - no HTTP ETag formatting in MCP protocol

This version system ensures data integrity while maintaining simple, clean semantics for AI agents working over JSON-RPC protocols.