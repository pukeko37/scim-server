# Migrate Between Versions

This guide covers migrating your SCIM Server implementation between versions within the 0.3.x series. The library follows semantic versioning and provides clear migration paths for breaking changes.

## Overview

The SCIM Server library started at version 0.3.0 and follows semantic versioning:

- **Patch versions** (0.3.0 → 0.3.1 → 0.3.2): Bug fixes, no breaking changes
- **Minor versions** (0.3.x → 0.4.x): New features, backward compatible
- **Major versions** (0.x.x → 1.x.x): Breaking changes requiring migration

## Current Version History

| Version | Release Date | Type | Key Changes |
|---------|--------------|------|-------------|
| 0.3.0 | 2024-12 | Initial | Initial release with core SCIM 2.0 functionality |
| 0.3.1 | 2024-12 | Patch | Enhanced storage provider architecture |
| 0.3.2 | 2024-12 | Patch | ETag concurrency control, MCP integration |

## Migration Within 0.3.x Series

### From 0.3.0 to 0.3.1

**Breaking Changes**: None - This is a patch release.

**New Features**:
- Enhanced storage provider architecture
- Improved error handling
- Additional validation features

**Migration Steps**:
1. Update `Cargo.toml`:
   ```toml
   [dependencies]
   scim-server = "0.3.1"
   ```

2. Run `cargo update` to update dependencies

3. No code changes required - all existing code continues to work

### From 0.3.1 to 0.3.2

**Breaking Changes**: None - This is a patch release.

**New Features**:
- ETag concurrency control system
- MCP (Model Context Protocol) integration for AI agents
- Enhanced versioning with `VersionedResource`
- Conditional operations support

**Migration Steps**:
1. Update `Cargo.toml`:
   ```toml
   [dependencies]
   scim-server = "0.3.2"
   ```

2. Run `cargo update`

3. **Optional**: Migrate to new concurrency features:
   ```rust
   use scim_server::resource::version::{VersionedResource, ConditionalResult};
   
   // Old approach (still works)
   let resource = provider.update_resource("User", &id, data, &context).await?;
   
   // New approach with version control
   let versioned = provider.conditional_update("User", &id, data, &expected_version, &context).await?;
   match versioned {
       ConditionalResult::Success(resource) => { /* Update succeeded */ },
       ConditionalResult::VersionMismatch(conflict) => { /* Handle conflict */ },
       ConditionalResult::NotFound => { /* Resource not found */ },
   }
   ```

## Best Practices for Migration

### 1. Test Before Upgrading

Always test version upgrades in a development environment:

```bash
# Create a test project
cargo new scim-test
cd scim-test

# Add the new version
cargo add scim-server@0.3.2

# Test your existing code
cargo build
cargo test
```

### 2. Review Release Notes

Check the [CHANGELOG.md](../../../../CHANGELOG.md) for detailed information about each release:

- New features and their usage
- Bug fixes that might affect your code
- Performance improvements
- Deprecation notices

### 3. Incremental Updates

For multiple version updates, upgrade incrementally:

```bash
# Instead of 0.3.0 → 0.3.2 directly
cargo add scim-server@0.3.1  # First step
cargo build && cargo test     # Verify
cargo add scim-server@0.3.2  # Second step
cargo build && cargo test     # Verify
```

### 4. Backup Before Production Updates

Always backup your data before updating production systems:

```rust
// Example backup before migration
async fn backup_before_migration() -> Result<(), Box<dyn std::error::Error>> {
    let provider = get_production_provider().await?;
    let context = RequestContext::new("migration", None);
    
    // Backup all users
    let users = provider.list_resources("User", None, &context).await?;
    let backup_file = format!("users_backup_{}.json", chrono::Utc::now().format("%Y%m%d_%H%M%S"));
    std::fs::write(backup_file, serde_json::to_string_pretty(&users)?)?;
    
    // Backup all groups
    let groups = provider.list_resources("Group", None, &context).await?;
    let backup_file = format!("groups_backup_{}.json", chrono::Utc::now().format("%Y%m%d_%H%M%S"));
    std::fs::write(backup_file, serde_json::to_string_pretty(&groups)?)?;
    
    Ok(())
}
```

## Staying Updated

### 1. Watch for New Releases

Monitor the repository for new releases:
- GitHub releases page
- Cargo.toml dependency updates
- Security advisories

### 2. Dependency Updates

Regularly update dependencies:

```bash
# Check for outdated dependencies
cargo outdated

# Update to latest compatible versions
cargo update

# Update to specific version
cargo add scim-server@latest
```

### 3. Feature Migration

When new features are released, consider migrating gradually:

```rust
// Example: Adopting ETag concurrency control
impl MyApplication {
    // Phase 1: Add version checking without enforcement
    async fn soft_version_check(&self, id: &str, expected_version: Option<&str>) -> bool {
        if let Some(version) = expected_version {
            // Log version mismatches but don't fail
            match self.provider.get_versioned_resource("User", id, &self.context).await {
                Ok(Some(resource)) => {
                    if resource.version().as_str() != version {
                        warn!("Version mismatch detected for user {}: expected {}, got {}", 
                              id, version, resource.version());
                        return false;
                    }
                },
                _ => return false,
            }
        }
        true
    }
    
    // Phase 2: Enforce version checking
    async fn strict_version_check(&self, id: &str, expected_version: &str) -> Result<(), AppError> {
        // Now fail on version mismatches
        // Implementation...
    }
}
```

## Future Migration Planning

### Upcoming Major Version (1.0.x)

The library is working toward a 1.0 release that may include:

- **Potential Breaking Changes**:
  - SCIM filter expression parser (if it changes API)
  - Bulk operations implementation
  - Enhanced multi-tenancy features

- **Migration Planning**:
  - Follow this documentation for migration guides
  - Test pre-release versions in development
  - Plan for potential downtime if storage formats change

### Staying Compatible

To minimize migration effort:

1. **Use the public API**: Avoid relying on internal implementation details
2. **Follow deprecation warnings**: Migrate away from deprecated features early
3. **Use feature flags conservatively**: Only enable features you actively use
4. **Test thoroughly**: Comprehensive tests make migrations safer

## Getting Help

If you encounter issues during migration:

1. **Check the [CHANGELOG.md](../../../../CHANGELOG.md)** for version-specific notes
2. **Review [GitHub Issues](https://github.com/pukeko37/scim-server/issues)** for known problems
3. **Ask questions** in GitHub Discussions
4. **Report bugs** with version information and reproduction steps

## Example: Complete Migration Script

```rust
// migration_helper.rs
use scim_server::*;

pub struct MigrationHelper {
    old_provider: Box<dyn ResourceProvider>,
    new_provider: Box<dyn ResourceProvider>,
}

impl MigrationHelper {
    pub async fn verify_compatibility(&self) -> Result<(), MigrationError> {
        // Verify that existing code still works
        let context = RequestContext::new("migration_test", None);
        
        // Test basic operations
        let test_user = serde_json::json!({
            "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
            "userName": "migration.test@example.com"
        });
        
        // This should work across all 0.3.x versions
        let created = self.new_provider.create_resource("User", test_user, &context).await?;
        let retrieved = self.new_provider.get_resource("User", &created.id().unwrap(), &context).await?;
        self.new_provider.delete_resource("User", &created.id().unwrap(), &context).await?;
        
        println!("✅ Migration compatibility verified");
        Ok(())
    }
}
```

This migration guide focuses on actual version history and real migration scenarios for the SCIM Server library. As the library evolves, this guide will be updated with specific migration instructions for each version.