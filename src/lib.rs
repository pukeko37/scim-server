//! SCIM 2.0 server library for Rust.
//!
//! Provides type-safe, async-first SCIM protocol implementation with
//! multi-tenant support and pluggable storage backends.
//!
//! # Core Components
//!
//! - [`ScimServer`] - Main server for handling SCIM operations
//! - [`ResourceProvider`] - Trait for implementing storage backends
//! - [`SchemaDiscovery`] - Schema introspection and service configuration
//!
//! # Quick Start
//!
//! ```rust,no_run
//! use scim_server::{ScimServer, providers::StandardResourceProvider};
//! use scim_server::storage::InMemoryStorage;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let storage = InMemoryStorage::new();
//! let provider = StandardResourceProvider::new(storage);
//! let server = ScimServer::new(provider)?;
//! # Ok(())
//! # }
//! ```
//!
//! For detailed usage, see the [SCIM Server Guide](https://docs.rs/scim-server/guide/).

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
pub use resource::{IsolationLevel, ResourceProvider, TenantPermissions};
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
