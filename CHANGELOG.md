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
- **Thread-Safe InMemoryProvider** - Production-ready conditional operations
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