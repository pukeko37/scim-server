//! SCIM 2.0 server library for Rust.
//!
//! A modern, type-safe implementation of the SCIM 2.0 protocol with clean architecture,
//! multi-tenant support, and pluggable storage backends.
//!
//! # Core Architecture
//!
//! This library follows a clean, layered architecture:
//!
//! - **SCIM Protocol Layer**: [`ScimServer`] handles SCIM HTTP operations
//! - **Resource Layer**: [`Resource`] provides type-safe resource representation
//! - **Storage Layer**: [`ResourceProvider`] trait for pluggable backends
//! - **Schema Layer**: [`Schema`] definitions with validation
//! - **Multi-tenancy**: Built-in support via [`TenantContext`]
//!
//! # Quick Start
//!
//! ```rust,no_run
//! use scim_server::ScimServer;
//! use scim_server::providers::StandardResourceProvider;
//! use scim_server::storage::InMemoryStorage;
//! use std::sync::Arc;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // 1. Set up storage and provider
//! let storage = InMemoryStorage::new();
//! let provider = StandardResourceProvider::new(storage);
//!
//! // 2. Create SCIM server
//! let server = ScimServer::new(provider)?;
//!
//! // 3. Server is ready for HTTP integration
//! # Ok(())
//! # }
//! ```
//!
//! # Key Features
//!
//! - **Type Safety**: Value objects with compile-time validation
//! - **Multi-tenant**: Full tenant isolation with configurable strategies
//! - **Async First**: Built on async/await for high performance
//! - **Pluggable Storage**: Bring your own database via [`ResourceProvider`]
//! - **Schema Validation**: Automatic validation against SCIM schemas
//! - **Version Control**: ETag-based optimistic concurrency control
//! - **Extensible**: Support for custom schemas and value objects
//!
//! # Architecture Overview
//!
//! ```text
//! ┌─────────────────┐    ┌──────────────────┐    ┌─────────────────┐
//! │   HTTP Layer    │───▶│   ScimServer     │───▶│ Operation       │
//! │   (Axum/etc)    │    │   (Protocol)     │    │ Handler         │
//! └─────────────────┘    └──────────────────┘    └─────────────────┘
//!                                 │                        │
//!                                 ▼                        ▼
//! ┌─────────────────┐    ┌──────────────────┐    ┌─────────────────┐
//! │     Schema      │    │    Resource      │    │ ResourceProvider│
//! │   Validation    │    │  (Value Objects) │    │   (Storage)     │
//! └─────────────────┘    └──────────────────┘    └─────────────────┘
//! ```

pub mod auth;
pub mod error;
/// Model Context Protocol integration for AI agents.
///
/// This module is only available when the `mcp` feature is enabled.
/// Add `features = ["mcp"]` to your Cargo.toml dependency to use this module.
#[cfg(feature = "mcp")]
pub mod mcp_integration;
pub mod multi_tenant;
pub mod operation_handler;
pub mod provider_capabilities;
pub mod providers;
pub mod resource;
pub mod resource_handlers;
pub mod schema;
pub mod schema_discovery;
pub mod scim_server;
pub mod storage;

// Re-export commonly used types for convenience
pub use error::{ScimError, ScimResult};
pub use providers::ResourceProvider;
pub use resource::{IsolationLevel, TenantPermissions};
pub use resource::{ListQuery, RequestContext, Resource, ScimOperation, TenantContext};
pub use schema::{Schema, SchemaRegistry};
pub use schema_discovery::SchemaDiscovery;
pub use scim_server::{ScimServer, ScimServerBuilder, ScimServerConfig, TenantStrategy};

// Re-export additional types needed by examples and advanced usage
pub use operation_handler::{
    OperationMetadata, ScimOperationHandler, ScimOperationRequest, ScimOperationResponse,
};
pub use provider_capabilities::{
    AuthenticationCapabilities, BulkCapabilities, CapabilityIntrospectable, ExtendedCapabilities,
    FilterOperator, PaginationCapabilities, ProviderCapabilities,
};
pub use resource_handlers::{create_group_resource_handler, create_user_resource_handler};
pub use schema_discovery::AuthenticationScheme;

// Multi-tenant types
pub use multi_tenant::{ScimTenantConfiguration, StaticTenantResolver, TenantResolver};

// MCP integration re-exports (feature-gated)
/// Model Context Protocol integration types.
///
/// These types are only available when the `mcp` feature is enabled.
/// Add `features = ["mcp"]` to your Cargo.toml dependency to use these types.
#[cfg(feature = "mcp")]
pub use mcp_integration::{McpServerInfo, ScimMcpServer, ScimToolResult};
