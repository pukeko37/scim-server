# SCIM PATCH Operations Guide

This guide provides comprehensive documentation for SCIM PATCH operations in the scim-server library, implementing RFC 7644 Section 3.5.2.

## Overview

SCIM PATCH operations allow you to make granular updates to resources without needing to send the entire resource representation. This is especially useful for:

- Updating specific attributes without affecting others
- Adding values to multi-valued attributes (emails, phone numbers)
- Removing specific values from arrays
- Atomic updates that either succeed completely or fail without partial changes

## Supported Operations

The scim-server library supports all three RFC 7644 PATCH operations:

### 1. `add` - Add New Values

Adds new attribute values or appends to existing multi-valued attributes.

### 2. `remove` - Remove Values

Removes attributes or specific values from multi-valued attributes.

### 3. `replace` - Replace Values

Replaces existing attribute values with new ones.

## Basic Usage

```rust
use scim_server::{ScimServer, providers::InMemoryProvider, resource::RequestContext};
use serde_json::json;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let provider = InMemoryProvider::new();
    let server = ScimServer::new(provider)?;
    let context = RequestContext::with_generated_id();

    // Create a user first
    let user_data = json!({
        "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
        "userName": "alice@example.com",
        "active": true,
        "emails": [{"value": "alice@example.com", "primary": true}]
    });
    
    let user = server.provider()
        .create_resource("User", user_data, &context)
        .await?;

    // Apply PATCH operations
    let patch_request = json!({
        "Operations": [
            {
                "op": "replace",
                "path": "active",
                "value": false
            },
            {
                "op": "add", 
                "path": "emails",
                "value": {"value": "alice.work@example.com", "type": "work"}
            }
        ]
    });

    let patched_user = server.provider()
        .patch_resource("User", &user.id, patch_request, &context)
        .await?;
    
    Ok(())
}
```

## Operation Examples

### Replace Operations

Replace the value of an existing attribute:

```json
{
  "Operations": [
    {
      "op": "replace",
      "path": "active",
      "value": false
    },
    {
      "op": "replace", 
      "path": "displayName",
      "value": "Alice Smith-Jones"
    },
    {
      "op": "replace",
      "path": "name.familyName", 
      "value": "Smith-Jones"
    }
  ]
}
```

### Add Operations

Add new values to attributes:

```json
{
  "Operations": [
    {
      "op": "add",
      "path": "emails",
      "value": {
        "value": "alice.work@company.com",
        "type": "work",
        "primary": false
      }
    },
    {
      "op": "add",
      "path": "phoneNumbers",
      "value": [
        {"value": "+1-555-123-4567", "type": "work"},
        {"value": "+1-555-987-6543", "type": "mobile"}
      ]
    },
    {
      "op": "add",
      "path": "department",
      "value": "Engineering"
    }
  ]
}
```

### Remove Operations

Remove attributes or specific values:

```json
{
  "Operations": [
    {
      "op": "remove",
      "path": "department"
    },
    {
      "op": "remove",
      "path": "emails[type eq \"work\"]"
    },
    {
      "op": "remove",
      "path": "phoneNumbers[value eq \"+1-555-123-4567\"]"
    }
  ]
}
```

## Path Expressions

SCIM PATCH operations support sophisticated path expressions for targeting specific attributes:

### Simple Paths

```json
"path": "active"              // Top-level attribute
"path": "name.givenName"      // Nested attribute
"path": "emails"              // Multi-valued attribute
```

### Filter Expressions

Target specific values in multi-valued attributes:

```json
"path": "emails[primary eq true]"                    // Primary email
"path": "emails[type eq \"work\"]"                   // Work emails
"path": "phoneNumbers[value eq \"+1-555-123-4567\"]" // Specific phone number
"path": "addresses[type eq \"work\"].streetAddress"  // Work address street
```

### Supported Filter Operators

- `eq` - Equals
- `ne` - Not equals  
- `sw` - Starts with
- `ew` - Ends with
- `co` - Contains
- `pr` - Present (attribute has a value)

## Multi-Valued Attributes

Special handling for arrays and complex multi-valued attributes:

### Adding to Arrays

```rust
let patch_add_emails = json!({
    "Operations": [
        {
            "op": "add",
            "path": "emails",
            "value": [
                {"value": "alice.personal@gmail.com", "type": "personal"},
                {"value": "alice.backup@yahoo.com", "type": "backup"}
            ]
        }
    ]
});
```

### Replacing Array Values

```rust
let patch_replace_primary_email = json!({
    "Operations": [
        {
            "op": "replace",
            "path": "emails[primary eq true].value",
            "value": "alice.new.primary@example.com"
        }
    ]
});
```

### Removing Array Items

```rust
let patch_remove_work_email = json!({
    "Operations": [
        {
            "op": "remove",
            "path": "emails[type eq \"work\"]"
        }
    ]
});
```

## ETag Integration

PATCH operations work seamlessly with ETag concurrency control:

```rust
use scim_server::resource::version::ConditionalResult;

// Get current version
let versioned_user = server.provider()
    .get_versioned_resource("User", "123", &context)
    .await?;

let patch_request = json!({
    "Operations": [
        {"op": "replace", "path": "active", "value": false}
    ]
});

// Conditional PATCH with version checking
match server.provider()
    .conditional_patch("User", "123", patch_request, versioned_user.version(), &context)
    .await?
{
    ConditionalResult::Success(updated) => {
        println!("PATCH successful! New version: {}", updated.version().to_http_header());
    },
    ConditionalResult::VersionMismatch(conflict) => {
        println!("Version conflict - resource was modified by another client");
        // Handle conflict: refresh, merge, or retry
    },
    ConditionalResult::NotFound => {
        println!("Resource no longer exists");
    }
}
```

## Error Handling

The library provides comprehensive error handling for PATCH operations:

### Common Errors

```rust
use scim_server::providers::InMemoryError;

match server.provider().patch_resource("User", "123", patch_request, &context).await {
    Ok(updated_user) => {
        println!("PATCH successful");
    },
    Err(InMemoryError::InvalidInput { message }) => {
        println!("Invalid PATCH request: {}", message);
    },
    Err(InMemoryError::InvalidPath { path, message }) => {
        println!("Invalid path '{}': {}", path, message);
    },
    Err(InMemoryError::ValidationError { message }) => {
        println!("Schema validation failed: {}", message);
    },
    Err(err) => {
        println!("Other error: {}", err);
    }
}
```

### Path Validation

The library validates paths to ensure:

- Read-only attributes cannot be modified (`id`, `meta.created`, etc.)
- Paths reference valid schema attributes
- Filter expressions are syntactically correct
- Target attributes exist in the resource schema

## Advanced Examples

### Complex User Profile Update

```rust
let comprehensive_patch = json!({
    "Operations": [
        // Update basic info
        {
            "op": "replace",
            "path": "displayName", 
            "value": "Dr. Alice Johnson-Smith"
        },
        {
            "op": "replace",
            "path": "name.honorificPrefix",
            "value": "Dr."
        },
        // Add new work email
        {
            "op": "add",
            "path": "emails",
            "value": {
                "value": "a.johnson-smith@newcompany.com",
                "type": "work",
                "primary": false
            }
        },
        // Remove old work email
        {
            "op": "remove", 
            "path": "emails[value eq \"alice.work@oldcompany.com\"]"
        },
        // Update work address
        {
            "op": "replace",
            "path": "addresses[type eq \"work\"].locality",
            "value": "San Francisco"
        },
        // Add enterprise attributes
        {
            "op": "add",
            "path": "urn:ietf:params:scim:schemas:extension:enterprise:2.0:User:department",
            "value": "Research & Development"
        }
    ]
});
```

### Group Membership Management

```rust
let group_patch = json!({
    "Operations": [
        // Add new members
        {
            "op": "add",
            "path": "members",
            "value": [
                {"value": "user123", "display": "John Doe"},
                {"value": "user456", "display": "Jane Smith"}
            ]
        },
        // Remove a specific member
        {
            "op": "remove",
            "path": "members[value eq \"user789\"]"
        },
        // Update group metadata
        {
            "op": "replace",
            "path": "displayName",
            "value": "Engineering Team - Backend"
        }
    ]
});
```

## Best Practices

### 1. Atomic Operations

All operations in a PATCH request are applied atomically:

```rust
// All operations succeed or all fail - no partial updates
let atomic_patch = json!({
    "Operations": [
        {"op": "replace", "path": "active", "value": false},
        {"op": "add", "path": "meta.lastModified", "value": "2024-01-15T10:30:00Z"},
        {"op": "remove", "path": "emails[type eq \"temp\"]"}
    ]
});
```

### 2. Path Specificity

Use specific paths to avoid unintended side effects:

```rust
// Good: Specific path
{"op": "replace", "path": "emails[primary eq true].value", "value": "new@example.com"}

// Avoid: Too broad
{"op": "replace", "path": "emails", "value": [{"value": "new@example.com", "primary": true}]}
```

### 3. Validation

Always validate PATCH requests before applying:

```rust
fn validate_patch_request(patch: &Value) -> Result<(), String> {
    let operations = patch.get("Operations")
        .ok_or("Missing Operations array")?
        .as_array()
        .ok_or("Operations must be an array")?;
    
    for (i, op) in operations.iter().enumerate() {
        let op_type = op.get("op")
            .ok_or(format!("Operation {} missing 'op' field", i))?
            .as_str()
            .ok_or(format!("Operation {} 'op' must be string", i))?;
            
        match op_type {
            "add" | "remove" | "replace" => {},
            _ => return Err(format!("Unsupported operation: {}", op_type))
        }
        
        // Validate path exists for remove and replace
        if op_type != "add" && !op.get("path").is_some() {
            return Err(format!("Operation {} missing required 'path' field", i));
        }
    }
    
    Ok(())
}
```

### 4. Error Recovery

Implement proper error handling and recovery:

```rust
async fn safe_patch_user(
    server: &ScimServer<InMemoryProvider>, 
    user_id: &str,
    patch: Value,
    context: &RequestContext
) -> Result<Resource, Box<dyn std::error::Error>> {
    // Validate before applying
    validate_patch_request(&patch)?;
    
    // Get current version for rollback if needed
    let current = server.provider()
        .get_versioned_resource("User", user_id, context)
        .await?;
    
    // Apply patch
    match server.provider().patch_resource("User", user_id, patch, context).await {
        Ok(updated) => Ok(updated),
        Err(err) => {
            // Log error and potentially restore from backup
            eprintln!("PATCH failed for user {}: {}", user_id, err);
            Err(Box::new(err))
        }
    }
}
```

## Performance Considerations

### 1. Batch Operations

Use single PATCH requests with multiple operations instead of multiple requests:

```rust
// Efficient: Single request with multiple operations
let batch_patch = json!({
    "Operations": [
        {"op": "replace", "path": "active", "value": false},
        {"op": "add", "path": "emails", "value": {"value": "new@example.com", "type": "work"}},
        {"op": "remove", "path": "phoneNumbers[type eq \"old\"]"}
    ]
});

// Less efficient: Multiple separate PATCH requests
// let patch1 = json!({"Operations": [{"op": "replace", "path": "active", "value": false}]});
// let patch2 = json!({"Operations": [{"op": "add", "path": "emails", "value": "..."}]});
// let patch3 = json!({"Operations": [{"op": "remove", "path": "phoneNumbers[type eq \"old\"]"}]});
```

### 2. Path Optimization

Use efficient path expressions:

```rust
// Efficient: Direct attribute access
{"op": "replace", "path": "active", "value": false}

// Less efficient: Unnecessary filtering
{"op": "replace", "path": ".[userName eq \"alice@example.com\"].active", "value": false}
```

## Testing PATCH Operations

The library includes comprehensive test coverage for PATCH operations:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use scim_server::providers::InMemoryProvider;
    
    #[tokio::test]
    async fn test_patch_add_email() {
        let provider = InMemoryProvider::new();
        let context = RequestContext::with_generated_id();
        
        // Create user
        let user_data = json!({
            "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
            "userName": "test@example.com",
            "emails": [{"value": "test@example.com", "primary": true}]
        });
        
        let user = provider.create_resource("User", user_data, &context).await.unwrap();
        
        // PATCH: Add new email
        let patch = json!({
            "Operations": [{
                "op": "add",
                "path": "emails",
                "value": {"value": "work@example.com", "type": "work"}
            }]
        });
        
        let updated = provider.patch_resource("User", &user.id, patch, &context).await.unwrap();
        
        // Verify email was added
        let emails = updated.get_attribute("emails").unwrap().as_array().unwrap();
        assert_eq!(emails.len(), 2);
        assert!(emails.iter().any(|e| e["value"] == "work@example.com"));
    }
}
```

## Troubleshooting

### Common Issues

1. **"Path not found" errors**: Ensure the path exists in the resource schema
2. **"Read-only attribute" errors**: Cannot modify `id`, `meta.created`, etc.
3. **"Invalid filter" errors**: Check filter expression syntax
4. **"Type mismatch" errors**: Ensure value types match schema definitions

### Debug Tips

1. Enable detailed logging:
   ```rust
   env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("debug")).init();
   ```

2. Validate paths before applying:
   ```rust
   let path_validator = PathValidator::new(&schema);
   path_validator.validate("emails[type eq \"work\"]")?;
   ```

3. Test with minimal operations first:
   ```rust
   // Start simple
   let simple_patch = json!({
       "Operations": [{"op": "replace", "path": "active", "value": false}]
   });
   ```

## Conclusion

SCIM PATCH operations provide a powerful and efficient way to update resources with atomic, granular changes. The scim-server library's implementation is fully compliant with RFC 7644 and provides comprehensive error handling, validation, and integration with ETag concurrency control.

For more examples and advanced usage patterns, see the test files in `tests/integration/patch/` and the various example applications in the `examples/` directory.