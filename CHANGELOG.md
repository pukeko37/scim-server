# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

### Changed

### Deprecated

### Removed

### Fixed

### Security

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