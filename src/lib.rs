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
//! use scim_server::{ScimServer, providers::InMemoryProvider};
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let provider = InMemoryProvider::new();
//! let server = ScimServer::new(provider)?;
//! # Ok(())
//! # }
//! ```
//!
//! For detailed usage, see the [SCIM Server Guide](https://docs.rs/scim-server/guide/).

pub mod auth;
pub mod error;
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
pub use resource::{RequestContext, TenantContext, Resource, ListQuery, ScimOperation};
pub use resource::ResourceProvider;
pub use schema::{Schema, SchemaRegistry};
pub use schema_discovery::SchemaDiscovery;
pub use scim_server::ScimServer;

// Re-export additional types needed by examples and advanced usage
pub use resource_handlers::{create_group_resource_handler, create_user_resource_handler};
pub use provider_capabilities::{
    BulkCapabilities, CapabilityIntrospectable, ExtendedCapabilities, PaginationCapabilities,
    ProviderCapabilities,
};
pub use operation_handler::{
    OperationMetadata, ScimOperationHandler, ScimOperationRequest, ScimOperationResponse,
};

// MCP integration re-exports (feature-gated)
#[cfg(feature = "mcp")]
pub use mcp_integration::{McpServerInfo, ScimMcpServer, ScimToolResult};
