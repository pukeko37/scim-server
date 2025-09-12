# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.5.1] - 2025-09-12

### Added
- **Comprehensive Concept Documentation**: Added 4 major concept guides totaling 2,100+ lines
  - **Operation Handlers**: Framework-agnostic integration layer for HTTP, MCP, CLI, and custom protocols
  - **MCP Integration (AI Agent Support)**: AI-native interface with tool discovery and schema-driven operations
  - **SCIM Server**: Central orchestration layer with dynamic resource management and multi-tenant support
  - **Multi-Tenant Architecture**: Complete tenant isolation with flexible deployment patterns
- **Enhanced Documentation Structure**: Reorganized concept guides by integration importance
  - Operation Handlers positioned as primary integration point
  - Logical flow from integration patterns to core architecture concepts

### Changed  
- **MCP Version Handling Documentation**: Clarified that MCP integration uses raw version format (`"abc123def"`) instead of HTTP ETag format for better AI agent programmatic access
- **Feature Flag Documentation**: Added comprehensive documentation for MCP feature flag usage
- **Concurrency Documentation**: Updated to ensure consistency with MCP integration patterns

### Documentation
- **Operation Handlers**: Complete guide covering framework-agnostic SCIM integration patterns
- **AI Agent Support**: Detailed MCP integration guide with conversational identity management examples
- **Multi-Tenant Scenarios**: Enterprise deployment patterns including SaaS, compliance, and geographic isolation
- **SCIM Server Architecture**: Deep dive into dynamic resource management and capability discovery

## [0.5.0] - 2025-01-29

### ⚠️ BREAKING CHANGES

- **Provider Interface Refactored**: Major simplification of `StandardResourceProvider` through helper traits
  - Conditional operations methods renamed: `conditional_update()` → `conditional_update_resource()`, `conditional_delete()` → `conditional_delete_resource()`
  - Helper traits now provide metadata management, patch operations, and multi-tenant functionality
  - `StandardResourceProvider` reduced by ~500 lines through trait composition
- **Error Handling Enhanced**: Added `String` → `ProviderError` conversion for improved error ergonomics

### Added
- **Helper Trait System**: New modular trait architecture for provider functionality
  - `ScimMetadataManager` for SCIM metadata operations
  - `ScimPatchOperations` for PATCH request handling  
  - `MultiTenantProvider` for tenant-aware operations
  - `ConditionalOperations` for version-based concurrency control
- **Comprehensive Architecture Documentation**: 1,160+ lines of detailed concept guides
  - Resources concept guide covering type safety and extensibility patterns
  - Resource Providers guide explaining business logic layer and SCIM compliance
  - Storage Providers guide documenting data persistence abstraction

### Changed
- **Code Organization**: Simplified `StandardResourceProvider` implementation through trait composition
- **Method Consistency**: Standardized conditional operation method naming across the codebase
- **Documentation Structure**: Reorganized concept guides in user documentation for better navigation

### Migration Guide
Update conditional operation method calls:
```rust
// Before v0.5.0
provider.conditional_update(resource_type, id, data, version, context).await?;
provider.conditional_delete(resource_type, id, version, context).await?;

// v0.5.0+
provider.conditional_update_resource(resource_type, id, data, version, context).await?;
provider.conditional_delete_resource(resource_type, id, version, context).await?;
```

## [0.4.1] - 2025-09-04

### Added
- **Phantom Type Version System**: Implemented compile-time format safety for SCIM resource versioning
  - `HttpVersion` and `RawVersion` type aliases with phantom types prevent format confusion
  - Cross-format equality support with `PartialEq<ScimVersion<F2>>` implementation
  - Automatic format conversion through standard Rust traits (`FromStr`, `Display`, `From`/`Into`)
  - Type-safe version handling eliminates entire class of runtime errors
- **Comprehensive Concurrency Documentation**: New concepts page covering SCIM concurrency control
  - When to use version-based concurrency (multi-client) vs when not to (single-client)
  - HTTP ETag vs MCP version handling differences and integration patterns
  - Content-based versioning with optional storage (versions can be computed on-demand)
  - Implementation patterns, best practices, and performance considerations

### Changed
- **API Transformation**: Replaced 7 custom methods with standard Rust traits
  - `ScimVersion::parse_http_header()` → `"W/\"abc123\"".parse::<HttpVersion>()`
  - `ScimVersion::to_http_header()` → `HttpVersion::from(version).to_string()`
  - `ScimVersion::matches()` → Standard `PartialEq` comparison (`==`)
  - `ScimVersion::parse_raw()` → `"abc123".parse::<RawVersion>()`
- **Functional Conversions**: Owned value transformations replace method calls
  - Clean bidirectional conversion: `HttpVersion::from(raw)` and `RawVersion::from(http)`
  - Format conversions through standard `From`/`Into` trait implementations
- **Updated All Integration Points**: 11 files updated across operation handlers, providers, examples, and tests
  - All HTTP ETag handling now uses `HttpVersion` type for compile-time safety
  - All MCP integration uses `RawVersion` type for cleaner programmatic access
  - Examples and integration tests updated with new type-safe patterns

### Improved
- **Code Reduction**: 37% reduction in version module (672 → ~420 lines, -252 lines)
- **Type Safety**: Compile-time prevention of HTTP/Raw format mixing
- **Test Suite Cleanup**: Removed 3 obsolete tests now guaranteed by type system
  - Cross-format comparison tests (guaranteed by `PartialEq` implementation)
  - Format conversion round-trip tests (guaranteed by `From`/`Into` traits)
  - Method existence tests (replaced with standard trait implementations)
- **Documentation**: Enhanced mdBook documentation with concurrency concepts
  - Cross-references between schema and concurrency concept pages
  - Updated SUMMARY.md with new concurrency concepts page

### Fixed
- **Version Conflict Logic**: Fixed assertion logic in comprehensive ETag tests
- **RBAC Example**: Added missing trait import for Role associated constants
- **Format Consistency**: All version operations now use consistent type-safe patterns

### Migration Notes
The phantom type system maintains 100% backward compatibility through type aliases:
- Existing `ScimVersion` usage continues to work unchanged
- New type-safe patterns available for enhanced compile-time safety
- No breaking changes to existing ResourceProvider implementations

This release demonstrates using Rust's type system to eliminate runtime errors while reducing code complexity—a textbook example of encoding constraints in types rather than relying on runtime checks.

## [0.4.0] - 2025-01-28

### ⚠️ BREAKING CHANGES

- **InMemoryProvider Removed**: The deprecated `InMemoryProvider` struct has been completely removed
  - **Migration**: Replace `InMemoryProvider::new()` with `StandardResourceProvider::new(InMemoryStorage::new())`
  - **Impact**: External users must update their code, but all functionality is preserved
  - **Benefit**: Eliminates 1200+ lines of duplicate code and provides cleaner API

- **StorageProvider Trait Extended**: Added new discovery methods (breaking change for custom storage implementations)
  - `list_tenants()` - Dynamically discover tenant IDs
  - `list_resource_types(tenant_id)` - Get resource types for specific tenant
  - `list_all_resource_types()` - Get all resource types across tenants
  - `clear()` - Clear all data from storage

### Added

- **Dynamic Tenant Discovery**: Multi-tenant statistics tracking now works with any tenant naming convention
  - No more hardcoded tenant patterns (`"tenant-a"`, `"perf-tenant-0"`, etc.)
  - Automatically discovers tenants and resource types from storage
  - Performance tests and production deployments work without code changes

- **Enhanced StorageProvider Trait**: New discovery capabilities enable robust multi-tenant operations
  - Storage backends can now expose tenant and resource type information
  - Enables accurate statistics gathering across arbitrary tenant structures
  - Future-proof for any storage backend implementation

### Fixed

- **Multi-Tenant Statistics Bug**: `get_stats()` method now correctly counts resources with arbitrary tenant names
- **Performance Test Failures**: Tests using non-hardcoded tenant patterns now pass
- **Architectural Consistency**: Removed code duplication between provider and storage layers

### Changed

- **Simplified API Surface**: Single clear path for in-memory storage via `StandardResourceProvider<InMemoryStorage>`
- **Improved Documentation**: Removed migration guides and outdated examples
- **Cleaner Codebase**: Reduced overall codebase size while maintaining full functionality

### Migration Guide

#### For InMemoryProvider Users

```rust
// Before v0.4.0 (removed)
use scim_server::providers::InMemoryProvider;
let provider = InMemoryProvider::new();

// v0.4.0+ (current)  
use scim_server::{providers::StandardResourceProvider, storage::InMemoryStorage};
let storage = InMemoryStorage::new();
let provider = StandardResourceProvider::new(storage);
```

#### For Custom Storage Implementations

Custom `StorageProvider` implementations must now implement the new discovery methods:

```rust
impl StorageProvider for MyCustomStorage {
    // ... existing methods ...
    
    // New required methods:
    async fn list_tenants(&self) -> Result<Vec<String>, Self::Error> { /* ... */ }
    async fn list_resource_types(&self, tenant_id: &str) -> Result<Vec<String>, Self::Error> { /* ... */ }  
    async fn list_all_resource_types(&self) -> Result<Vec<String>, Self::Error> { /* ... */ }
    async fn clear(&self) -> Result<(), Self::Error> { /* ... */ }
}
```

## [0.3.11] - 2025-08-26

### Added
- **MCP Server Setup Guide**: Comprehensive user guide for setting up Model Context Protocol servers
- **Complete MCP Integration**: Full documentation for AI agent integration with SCIM servers
- **Production MCP Examples**: 30-line minimal and production-ready MCP server configurations
- **AI-Friendly Interface**: Structured tool discovery and execution for AI agents

### Fixed
- **MCP stdio server**: Added missing List operation support for User resources
- **Group Resource Support**: Fixed MCP server to properly register Group resource types
- **Documentation Examples**: Updated all deprecated `InMemoryProvider` usage to `StandardResourceProvider<InMemoryStorage>`
- **MCP Documentation Compliance**: Fixed critical issues in MCP integration documentation
- **API Consistency**: All documentation examples now use current API patterns

### Documentation
- **User Guide Enhancement**: Added comprehensive MCP server setup guide to mdBook documentation
- **Schema Documentation**: Split and improved SCIM schema concepts documentation for better focus
- **Getting Started Recovery**: Restored lost installation and first-server documentation
- **Example Modernization**: Updated all examples to use current `StandardResourceProvider` patterns
- **Code Quality**: All documentation tests now pass with current API

### Changed
- **Version Updates**: Updated all documentation references to v0.3.11
- **API Migration**: Completed transition from deprecated `InMemoryProvider` to `StandardResourceProvider<InMemoryStorage>`
- **Documentation Structure**: Enhanced mdBook structure with MCP integration guide

This release focuses on MCP integration maturity and documentation excellence, making the SCIM server fully AI-ready with comprehensive setup guides.

## [0.3.10] - 2025-01-12

### Fixed
- **docs.rs Build**: Fixed documentation build failure by removing unstable `#[doc(cfg)]` attributes
- **Stable Rust Compatibility**: Removed `#[cfg_attr(docsrs, doc(cfg(feature = "mcp")))]` that requires nightly features
- **Documentation Accessibility**: Ensured documentation builds successfully on docs.rs with stable Rust

This patch release fixes the remaining docs.rs build failure from v0.3.9 by removing all unstable documentation features.

## [0.3.9] - 2025-01-12

### Fixed
- **docs.rs Build**: Fixed documentation build failure on docs.rs by removing unstable rustdoc flags
- **Cargo.toml**: Simplified docs.rs configuration to use only stable `--cfg docsrs` flag
- **Documentation Accessibility**: Ensured documentation builds successfully on docs.rs for all users

This is a patch release that fixes the docs.rs build failure in v0.3.8 without any functional changes.

## [0.3.8] - 2025-01-12

### Documentation Excellence 📚
- **Major API Documentation Restructuring**: Transformed overwhelming 700+ line lib.rs documentation into focused 25-line API reference
- **Enhanced Module Documentation**: Reduced all module docs to focused 10-15 lines following Rust standards
- **Improved Discoverability**: Traits, methods, and types now prominent and easy to find
- **Professional Structure**: Clear separation between API reference (rustdoc) and comprehensive guides (mdbook)
- **Advanced Documentation Features**: 
  - Added feature-gated documentation with `#[cfg_attr(docsrs, doc(cfg(feature = "mcp")))]`
  - Enhanced docs.rs configuration with link-to-definition and index page generation
  - Fixed import paths and re-exports for better API accessibility
- **Documentation Quality**: Added comprehensive error type documentation and fixed missing docs warnings
- **Test Coverage**: Most documentation examples now compile successfully

### Module Documentation Improvements
- **ResourceProvider**: Reduced from 50+ to 15 lines, added Error type documentation
- **Schema**: Reduced from 35 to 15 lines, focused on key types with clean examples  
- **Providers**: Reduced from 40 to 15 lines, comprehensive error field documentation
- **OperationHandler**: Reduced from 59 to 15 lines, framework-agnostic focus
- **InMemoryProvider**: Added full documentation for all error variants and statistics fields

### Enhanced Developer Experience
- **Clean Navigation**: Module structure makes API approachable without information overload
- **Standards Compliance**: Follows Rust documentation conventions throughout
- **Better Re-exports**: Streamlined public API with commonly used types easily accessible
- **Example Compatibility**: Maintained backward compatibility for all examples and advanced usage

### Internal Improvements
- **Removed Legacy Content**: Cleaned up outdated mdbook files stored elsewhere
- **Consistent Structure**: All modules follow same focused template (purpose, key types, example)
- **Ready for Scale**: Documentation structure supports comprehensive guides development

## [0.3.7] - 2025-08-16

### Documentation Overhaul 📚
- **Major Documentation Cleanup**: Removed 73,000+ lines of outdated and fictional content
- **Removed Fictional Content**: Deleted 6 tutorial files (181KB) containing non-existent API references
- **Streamlined Structure**: Simplified to essential Getting Started content only
- **Professional Framework**: Established comprehensive library structure (User Guide, Advanced, Examples, Reference)
- **Link Verification**: All documentation links verified working - zero broken references
- **Content Accuracy**: Removed misleading claims about unimplemented features

### Critical Bug Fixes 🐛
- **CRITICAL: Schema Discovery Runtime Failure** - Fixed `SchemaDiscovery::new()` failing with `SchemaLoadError` for "Core" schema
  - Embedded core SCIM schemas (User, Group, ServiceProviderConfig) directly in the library
  - `SchemaDiscovery::new()` now uses embedded schemas by default, eliminating external file dependencies
  - `SchemaRegistry::new()` updated to use embedded schemas for better reliability
  - Added `SchemaRegistry::with_embedded_schemas()` method for explicit embedded schema usage
  - `SchemaRegistry::from_schema_dir()` still available for loading custom schemas from files

### Changed
- **Documentation**: Completely restructured for accuracy and clarity
- **Documentation**: Removed IDE setup, troubleshooting, and next steps sections from Getting Started
- **Documentation**: Updated Schema Discovery tutorial examples to use proper error handling instead of `.unwrap()`
- **Documentation**: Fixed schema-validator examples to use generic paths instead of hardcoded "schemas/" directory
- **BREAKING**: `SchemaRegistry::new()` now uses embedded schemas instead of loading from "schemas/" directory
  - This fixes the critical runtime failure but changes the default behavior
  - Users who need file-based schema loading should use `SchemaRegistry::from_schema_dir("path")` explicitly

### Added
- New embedded schemas module (`schema::embedded`) with hardcoded core SCIM schemas
- `SchemaRegistry::with_embedded_schemas()` method for explicit embedded schema initialization
- Clean documentation baseline for future API-accurate content development

### Removed
- Fictional tutorial content (authentication-setup.md, custom-resources.md, framework-integration.md, mcp-integration.md, multi-tenant-deployment.md, performance-optimization.md)
- Misleading advanced topics (monitoring.md, production-deployment.md, security.md)
- Inaccurate how-to guides (migrate-versions.md, troubleshooting.md)
- Broken reference documentation (api-endpoints.md, configuration.md, scim-compliance.md)
- External schema files (Group.json, User.json, ServiceProviderConfig.json)

## [Unreleased]

### Added

### Changed

### Deprecated

### Removed

### Fixed

### Security

## [0.3.7] - 2025-08-16

### Fixed
- **Documentation**: Fixed critical issues identified in getting started guide
  - Updated version references from 0.3.2/0.3.6 to 0.3.7 throughout documentation
  - Added missing `ResourceProvider` trait import in installation verification example
  - Corrected feature flags documentation to show only actual features (`mcp` only)
  - Removed references to non-existent `auth` and `logging` features
  - Fixed ASCII art diagram in introduction to prevent mdbook test failures
  - Cleaned up example code to eliminate compilation warnings
- **Documentation**: Fixed critical issues identified in core concepts guide
  - Fixed `RequestContext::new()` API inconsistencies across all code examples
  - Corrected implementation status table with accurate method links to docs.rs
  - Updated architecture documentation to reflect actual storage provider implementations
  - Removed misleading claims about PostgreSQL, MySQL, DynamoDB support (only InMemory currently implemented)
  - Added proper API documentation links throughout concepts section
  - Fixed compilation errors in bulk operation examples
- **Documentation**: Fixed critical issues identified in custom resources tutorial
  - Replaced non-existent `ScimResource` trait usage with correct `Resource` type
  - Fixed schema definition to use direct struct initialization instead of non-existent builder pattern
  - Corrected `ResourceMeta` to `Meta` and updated import statements
  - Rewrote provider implementation to use actual `ResourceProvider` trait API
  - Fixed `AttributeDefinition` API structure (`type_` → `data_type`, removed non-existent fields)
  - Added complete working example with proper field types and comprehensive test coverage
  - Resolved all 28 compilation errors identified in independent testing
  - Tutorial now provides end-to-end implementation path from schema to working provider
- **Documentation**: Fixed critical issues identified in authentication tutorial
  - Replaced deprecated `InMemoryProvider` with current `StandardResourceProvider<InMemoryStorage>` API
  - Fixed `ScimServer::builder()` to correct `ScimServer::new()` constructor
  - Added comprehensive dependencies section with required Cargo.toml entries
  - Added missing `scim_routes()` function definition for complete working example
  - Updated all import statements to use correct current API
- **Code Quality**: All documentation examples now compile and run without warnings

### Changed
- **Documentation**: Switched from exact version pinning to flexible versioning in examples
- **Examples**: Improved installation verification test with minimal, clean imports

## [0.3.6] - 2025-08-16

### Fixed
- Successfully deployed documentation to GitHub Pages with working CSS and navigation
- Manual gh-pages branch creation to resolve GitHub Actions deployment issues
- All documentation links now fully functional with proper styling

### Changed
- Documentation now hosted at https://pukeko37.github.io/scim-server/ with complete mdbook functionality

## [0.3.5] - 2025-08-16

### Fixed
- Updated GitHub Pages workflow to properly create gh-pages branch for documentation deployment
- Fixed workflow compatibility with default GitHub Pages settings

## [0.3.4] - 2025-08-16

### Added
- GitHub Actions workflow for automatic documentation deployment to GitHub Pages
- Professional documentation hosting with proper CSS/JS loading and navigation

### Changed
- README.md documentation links now use clean GitHub Pages URLs (https://pukeko37.github.io/scim-server/)
- Improved documentation user experience with fully functional mdbook interface

## [0.3.3] - 2025-08-16

### Fixed
- Fixed README.md documentation links to point to actual HTML documentation using htmlpreview.github.io instead of mdbook source files

## [0.3.2] - 2025-08-16

### Added
- Comprehensive documentation overhaul with 10,000+ lines of accurate content
- Single source of truth documentation in mdbook format (`docs/guide/src/`)
- Complete mdbook-based user guide with professional structure
- Placeholder pages filled with substantive content and TODO markers
- Clear implementation status indicators (✅/❌/⚠️) throughout documentation
- Honest SCIM compliance assessment (updated from false 94% to accurate ~65%)
- Working code examples that compile and run successfully
- Documentation refactoring completion report in `docs/DOCUMENTATION_REFACTORING_COMPLETE.md`
- SCIM compliance audit findings in `docs/COMPLIANCE_AUDIT_FINDINGS.md`

### Changed
- **MAJOR**: Updated SCIM compliance claims from misleading 94% to honest ~65% assessment
- **MAJOR**: Replaced all non-existent API examples (FilterExpression, BulkOperation) with working alternatives
- Converted troubleshooting guide to use realistic pagination patterns instead of filter expressions
- Updated security examples to use safe query building instead of non-existent filter parsing
- Modified all tutorials to show only working code patterns
- Enhanced migration guide with accurate version history (0.3.0 → 0.3.2)
- Improved API endpoint documentation with clear implementation status

### Removed
- **21,700+ lines** of outdated, duplicate, and misleading documentation content
- Entire `docs/guides/` directory (75+ redundant files)
- Complete `docs/examples/` directory with outdated patterns
- All references to non-existent FilterExpression::parse() APIs
- All fake BulkOperation and BulkRequest implementation examples
- Misleading bulk operations support claims
- False filter expression parsing capabilities

### Fixed
- All code examples now compile successfully without errors
- Removed references to unimplemented features throughout documentation
- Corrected API endpoint documentation to reflect actual capabilities
- Fixed placeholder pages that were causing broken navigation
- Updated configuration examples to match actual library options
- Corrected troubleshooting patterns to use implemented features only

### Documentation
- **BREAKING**: Consolidated all documentation to single mdbook source
- Created comprehensive tutorial series with working examples only
- Enhanced getting started guides with accurate feature representations
- Added detailed provider implementation patterns
- Improved schema and validation documentation
- Enhanced multi-tenancy and security guides
- Updated all reference documentation for accuracy

## [0.3.1] - 2025-08-14

### Added
- **MAJOR**: New `StorageProvider` trait for pluggable storage backends
- **MAJOR**: `StandardResourceProvider<S>` generic provider with pluggable storage
- `InMemoryStorage` implementation of the new storage interface
- Provider statistics API (`get_stats()` method)
- Clear functionality for testing (`clear()` method)
- Conditional operations support (version-aware updates, deletes, patches)
- Enhanced versioning system with `VersionedResource` support
- Migration guide for upgrading from `InMemoryProvider`
- Updated examples demonstrating new provider architecture

### Changed
- **BREAKING**: Provider architecture now uses generic storage backends
- **BREAKING**: `InMemoryProvider` moved to legacy status (still functional)
- Updated examples to use `StandardResourceProvider` by default
- Improved error handling and type safety in provider operations
- Enhanced documentation for provider patterns and best practices

### Deprecated
- **IMPORTANT**: `InMemoryProvider` is now deprecated in favor of `StandardResourceProvider<InMemoryStorage>`
- Old provider pattern will be removed in v0.4.0
- See migration guide in `docs/migration-v0.4.md` for upgrade instructions

### Security
- Improved tenant isolation in multi-tenant scenarios
- Enhanced validation in conditional operations to prevent race conditions

### Migration Notes
- See `docs/migration-v0.4.md` for detailed migration instructions
- Examples updated to demonstrate new provider patterns
- Backward compatibility maintained for existing `InMemoryProvider` users
- All existing ResourceProvider APIs remain unchanged

### Deprecated
- `InMemoryProvider` - Use `StandardResourceProvider<InMemoryStorage>` instead
- Direct usage of provider implementations - Use the new generic provider pattern

### Migration Guide
To migrate from v0.2.x to v0.3.1:

1. Replace `InMemoryProvider::new()` with:
   ```rust
   let storage = InMemoryStorage::new();
   let provider = StandardResourceProvider::new(storage);
   ```

2. Update imports:
   ```rust
   // Old
   use scim_server::providers::InMemoryProvider;
   
   // New
   use scim_server::storage::InMemoryStorage;
   use scim_server::providers::StandardResourceProvider;
   ```

3. The API remains the same - only the construction changes

## [0.3.0] - 2025-08-14

### Note
This version was published as a patch update before the complete storage provider architecture was ready. Please use v0.3.1 for the full storage provider implementation.

### Added
- **MAJOR**: New `StorageProvider` trait for pluggable storage backends
- **MAJOR**: `StandardResourceProvider<S>` generic provider with pluggable storage
- `InMemoryStorage` implementation of the new storage interface
- Provider statistics API (`get_stats()` method)
- Clear functionality for testing (`clear()` method)
- Conditional operations support (version-aware updates, deletes, patches)
- Enhanced versioning system with `VersionedResource` support
- Migration guide for upgrading from `InMemoryProvider`
- Updated examples demonstrating new provider architecture

### Changed
- **BREAKING**: Provider architecture now uses generic storage backends
- **BREAKING**: `InMemoryProvider` moved to legacy status (still functional)
- Updated examples to use `StandardResourceProvider` by default
- Improved error handling and type safety in provider operations
- Enhanced documentation for provider patterns and best practices

### Deprecated
- **IMPORTANT**: `InMemoryProvider` is now deprecated in favor of `StandardResourceProvider<InMemoryStorage>`
- Old provider pattern will be removed in v0.4.0
- See migration guide in `docs/migration-v0.4.md` for upgrade instructions

### Security
- Improved tenant isolation in multi-tenant scenarios
- Enhanced validation in conditional operations to prevent race conditions

### Migration Notes
- See `docs/migration-v0.4.md` for detailed migration instructions
- Examples updated to demonstrate new provider patterns
- Backward compatibility maintained for existing `InMemoryProvider` users
- All existing ResourceProvider APIs remain unchanged

### Deprecated
- `InMemoryProvider` - Use `StandardResourceProvider<InMemoryStorage>` instead
- Direct usage of provider implementations - Use the new generic provider pattern

### Migration Guide
To migrate from v0.2.x to v0.3.0:

1. Replace `InMemoryProvider::new()` with:
   ```rust
   let storage = InMemoryStorage::new();
   let provider = StandardResourceProvider::new(storage);
   ```

2. Update imports:
   ```rust
   // Old
   use scim_server::providers::InMemoryProvider;
   
   // New
   use scim_server::storage::InMemoryStorage;
   use scim_server::providers::StandardResourceProvider;
   ```

3. The API remains the same - only the construction changes

## [0.2.3] - 2025-01-14

### Added
- **Complete SCIM PATCH Operations** - Full RFC 7644 Section 3.5.2 implementation
  - `add` operation for adding new attributes and array values to resources
  - `remove` operation for deleting specific attributes and values
  - `replace` operation for updating existing attribute values
  - Path expression parsing and validation with comprehensive error handling
  - Multi-valued attribute support (emails, phone numbers, addresses)
  - Nested attribute path handling for complex objects
  - ETag concurrency control integration for safe PATCH operations
  - Comprehensive test coverage with 450+ passing tests including PATCH scenarios

### Changed
- **Enhanced InMemoryProvider** - Added robust PATCH operation support with validation
  - Path validation to prevent modification of read-only attributes
  - Schema-aware attribute existence checking
  - Atomic PATCH operations with rollback on error
- **Improved Error Handling** - More descriptive PATCH-related error messages
- **Documentation Updates** - Added comprehensive PATCH operation examples and guides

### Fixed
- **SCIM Compliance** - Now fully compliant with RFC 7644 PATCH requirements
- **Multi-valued Attribute Handling** - Correct behavior for array operations
- **Path Expression Validation** - Proper handling of complex nested paths
- **Test Coverage** - All PATCH operation edge cases properly covered

## [0.2.2] - 2025-01-13

### Fixed
- **Documentation Build** - Fixed private intra-doc link warnings for docs.rs
- **docs.rs Configuration** - Added metadata for comprehensive feature documentation

## [0.2.1] - 2025-01-12

### Added
- **Compile-Time Authentication System** - Type-safe authorization enforced at compile time
  - `AuthenticationState` phantom types for tracking auth status
  - `LinearCredentials` that can only be consumed once to prevent reuse
  - `AuthenticationWitness` types proving successful authentication
  - `TenantAuthority` for compile-time tenant access validation
  - Zero-cost runtime authentication with compile-time guarantees
- **Type-Safe Request Contexts** - Authentication required for sensitive operations
  - `AuthenticatedContext<T>` wrapper ensuring proper authorization
  - Compile-time prevention of unauthenticated resource access
  - Linear type consumption preventing credential replay attacks
- **Enhanced Documentation** - Comprehensive guides for new authentication system
  - [Compile-Time Authentication Guide](docs/COMPILE_TIME_AUTHENTICATION.md)
  - Working examples demonstrating type-safe auth patterns
  - RBAC (Role-Based Access Control) implementation examples

### Changed
- **Modular Architecture Refactoring** - Improved code organization and maintainability
  - Split `operation_handler.rs` into focused submodules (core, handlers, builders, errors)
  - Refactored MCP integration into clean modular structure with separate concerns
  - Moved unit tests to dedicated test directory structure
  - Enhanced separation between CRUD, query, schema, and utility operations
- **MCP Integration Improvements** - Better organization and maintainability
  - Separated core types, protocol handling, and tool schemas into distinct modules
  - Enhanced error handling with dedicated modules per functional area
  - Improved type safety through modular design
- **Test Organization** - Restructured test suite for better maintainability
  - Consolidated unit tests into logical module groupings
  - Enhanced integration test coverage for new authentication features

### Fixed
- **Code Organization** - Resolved maintainability issues with large monolithic modules
- **Module Dependencies** - Cleaner separation of concerns across the codebase
- **Test Coverage** - Comprehensive testing for new compile-time authentication features

### Security
- **Compile-Time Security Guarantees** - Authentication bugs caught at compile time
  - Impossible to access protected resources without proper authentication
  - Linear credentials prevent authentication bypass through credential reuse
  - Type system enforces tenant isolation at compile time

## [0.2.0] - 2024-12-28

### Added
- **ETag Concurrency Control System** - Complete implementation of weak ETag-based optimistic locking
  - `ScimVersion` type for hash-based deterministic versioning
  - `ConditionalResult` enum for handling version conflicts, success, and not-found scenarios
  - `VersionedResource` wrapper providing automatic version management
  - HTTP ETag header generation and parsing (RFC 7232 compliant)
  - Weak ETag format (`W/"version"`) for semantic equivalence
- **Conditional Provider Operations** - New trait extending `ResourceProvider`
  - `conditional_update()` - Version-checked resource updates
  - `conditional_delete()` - Version-checked resource deletions
  - `get_versioned_resource()` - Retrieve resources with version information
  - `create_versioned_resource()` - Create resources with automatic versioning
- **Enhanced Operation Handler** - Full SCIM server integration with ETag support
  - Automatic ETag inclusion in all operation responses
  - Version conflict detection with structured error responses
  - Backward compatible operation routing (conditional when available, fallback otherwise)
  - Multi-tenant version isolation
- **MCP Integration Enhancements** - AI agent support for concurrent operations
  - ETag metadata in all MCP tool responses
  - Version conflict handling in AI workflows
  - Structured error responses for automated conflict resolution
- **Thread-Safe InMemoryProvider** - Stable conditional operations
  - Atomic version checking and updates using mutex protection
  - Concurrent operation safety validation
  - Performance optimized for high-throughput scenarios

### Changed
- **Resource Provider Interface** - Non-breaking extension with new versioned methods
- **Operation Handler Response Format** - Now includes `etag` field in metadata for all operations
- **Error Handling** - Enhanced with version conflict details and resolution guidance
- **Test Coverage** - Expanded to 827 tests (397 integration + 332 unit + 98 doctests)

### Fixed
- **Documentation Examples** - All doctests now compile and execute correctly
- **Weak ETag Implementation** - Consistent `W/"version"` format across all components
- **Concurrency Safety** - Prevents data loss in multi-client modification scenarios
- **Memory Safety** - Resolved ownership issues in MCP integration examples

## [0.1.0] - 2024-12-19

### Added
- Initial SCIM 2.0 server library implementation
- Multi-tenant support with type-safe operations
- Core SCIM resource types (User, Group, Enterprise User)
- SCIM schema validation and enforcement
- JSON attribute filtering and projection
- Resource versioning with meta attributes
- Comprehensive error handling with SCIM-compliant responses
- Model Context Protocol (MCP) integration for AI agents
- Schema validation utility binary
- Performance benchmarks for resource operations
- Full test coverage with integration tests

### Security
- Type-safe resource operations preventing common vulnerabilities
- Secure tenant isolation mechanisms