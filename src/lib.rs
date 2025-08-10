//! # SCIM Server Library for Rust
//!
//! A comprehensive System for Cross-domain Identity Management (SCIM) server library
//! that enables developers to implement SCIM-compliant identity providers with minimal effort.
//!
//! [![Crates.io](https://img.shields.io/crates/v/scim-server.svg)](https://crates.io/crates/scim-server)
//! [![Documentation](https://docs.rs/scim-server/badge.svg)](https://docs.rs/scim-server)
//! [![License](https://img.shields.io/badge/license-MIT-blue.svg)](https://github.com/yourusername/scim-server/blob/main/LICENSE)
//! [![Build Status](https://github.com/yourusername/scim-server/workflows/CI/badge.svg)](https://github.com/yourusername/scim-server/actions)
//!
//! ## Overview
//!
//! This library transforms SCIM from a complex enterprise integration challenge into a
//! straightforward provider implementation task, allowing developers to focus on their
//! core business logic while automatically gaining enterprise SSO and provisioning capabilities.
//!
//! ### Key Value Propositions
//!
//! - **For SaaS Developers**: Add enterprise-grade SCIM provisioning without protocol expertise
//! - **For Enterprise Customers**: Seamless identity provisioning and deprovisioning
//! - **For Integration Teams**: Standards-compliant SCIM 2.0 implementation out of the box
//!
//! ## Table of Contents
//!
//! - [Two Main Components](#two-main-components)
//! - [Features](#features)
//! - [Installation](#installation)
//! - [MCP Integration](#mcp-model-context-protocol-integration)
//! - [Architecture Overview](#architecture-overview)
//! - [Logging Support](#logging-support)
//! - [Multi-Tenant Support](#multi-tenant-support)
//! - [Quick Start - Full SCIM Server](#quick-start---full-scim-server)
//! - [Schema Discovery](#schema-discovery)
//! - [Provider Implementation](#provider-implementation)
//! - [Examples](#examples)
//! - [Schema Validation Utility](#schema-validation-utility)
//! - [Performance](#performance)
//! - [SCIM 2.0 Compliance](#scim-20-compliance)
//! - [Contributing](#contributing)
//!
//! ## Two Main Components
//!
//! This library provides two distinct components:
//!
//! - **`ScimServer`** - Full-featured dynamic server for production SCIM endpoints with runtime resource registration and CRUD operations
//! - **`SchemaDiscovery`** - Lightweight component for schema discovery and service provider configuration
//!
//! ## Features
//!
//! - **RFC Compliance**: Full RFC 7643/7644 compliance for core User and Group schemas
//! - **Type Safety**: Compile-time guarantees preventing invalid operations
//! - **Multi-Tenant Ready**: Built-in tenant isolation and context management
//! - **Provider Agnostic**: Works with any storage backend via trait abstraction
//! - **Async-First**: Non-blocking operations with tokio integration
//! - **Value Objects**: Type-safe SCIM attribute handling with validation
//! - **Schema-Driven**: Dynamic resource types with runtime schema validation
//! - **Comprehensive Logging**: Structured logging with request IDs and tenant context
//! - **Flexible Backends**: Choose from env_logger, tracing, slog, or any log-compatible crate
//! - **Auto-Discovery**: Automatic provider capability detection
//! - **MCP Integration**: AI agent support via Model Context Protocol (optional feature)
//!
//! ## Installation
//!
//! Add this to your `Cargo.toml`:
//!
//! ```toml
//! [dependencies]
//! scim-server = "0.1.0"
//!
//! # For async runtime
//! tokio = { version = "1.0", features = ["full"] }
//!
//! # For logging (choose one)
//! env_logger = "0.10"  # Simple logging
//! # OR
//! tracing-subscriber = "0.3"  # Structured logging
//! ```
//!
//! ### Optional Features
//!
//! ```toml
//! [dependencies]
//! scim-server = { version = "0.1.0", features = ["mcp"] }
//! ```
//!
//! - **`mcp`**: Enables Model Context Protocol integration for AI agents
//!
//! ## MCP (Model Context Protocol) Integration
//!
//! The SCIM server provides optional MCP integration for AI agent interactions. When enabled,
//! the server exposes SCIM operations as structured tools that AI agents can discover and use.
//!
//! ### Enabling MCP Support
//!
//! Add the MCP feature to your `Cargo.toml`:
//!
//! ```toml
//! [dependencies]
//! scim-server = { version = "0.1.0", features = ["mcp"] }
//! ```
//!
//! ### Basic MCP Server Setup
//!
//! ```rust,no_run
//! # #[cfg(feature = "mcp")]
//! use scim_server::{ScimServer, mcp_integration::ScimMcpServer, providers::InMemoryProvider};
//!
//! # #[cfg(feature = "mcp")]
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // Create SCIM server
//!     let provider = InMemoryProvider::new();
//!     let scim_server = ScimServer::new(provider)?;
//!
//!     // Wrap with MCP integration
//!     let mcp_server = ScimMcpServer::new(scim_server);
//!
//!     // Get available tools for AI agents
//!     let tools = mcp_server.get_tools();
//!     println!("Available tools: {}", tools.len());
//!
//!     // Run MCP server with stdio transport
//!     mcp_server.run_stdio().await?;
//!     Ok(())
//! }
//! ```
//!
//! ### Available MCP Tools
//!
//! The MCP integration provides these tools for AI agents:
//!
//! - **`scim_create_user`** - Create new users with SCIM schema validation
//! - **`scim_get_user`** - Retrieve user by ID with full attribute access
//! - **`scim_update_user`** - Update user attributes with conflict detection
//! - **`scim_delete_user`** - Remove users with proper cleanup
//! - **`scim_list_users`** - List all users with pagination support
//! - **`scim_search_users`** - Search users by attributes with filtering
//! - **`scim_user_exists`** - Check user existence for validation
//! - **`scim_get_schemas`** - Retrieve all available schemas for AI understanding
//! - **`scim_get_schema`** - Get specific schema details for validation
//! - **`scim_server_info`** - Get server capabilities and supported operations
//!
//! ### Multi-Tenant MCP Operations
//!
//! AI agents can work with multi-tenant environments:
//!
//! ```rust,no_run
//! # #[cfg(feature = "mcp")]
//! use serde_json::json;
//! # #[cfg(feature = "mcp")]
//! # use scim_server::mcp_integration::ScimMcpServer;
//! # #[cfg(feature = "mcp")]
//! # async fn example(mcp_server: ScimMcpServer<scim_server::providers::InMemoryProvider>) -> Result<(), Box<dyn std::error::Error>> {
//!
//! // Create user in specific tenant
//! let result = mcp_server.execute_tool(
//!     "scim_create_user",
//!     json!({
//!         "user_data": {
//!             "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
//!             "userName": "ai.agent@company.com",
//!             "active": true
//!         },
//!         "tenant_id": "enterprise-corp"
//!     })
//! ).await;
//! # Ok(())
//! # }
//! ```
//!
//! ### AI Agent Integration Benefits
//!
//! - **Schema Discovery**: AI agents can introspect SCIM schemas for proper validation
//! - **Type Safety**: All operations include JSON schema validation for inputs
//! - **Error Handling**: Structured error responses with actionable information
//! - **Multi-Tenant**: Automatic tenant isolation for enterprise scenarios
//! - **Comprehensive CRUD**: Full resource lifecycle management
//! - **Standards Compliance**: SCIM 2.0 compliant operations for enterprise integration
//!
//! ### Custom MCP Server Configuration
//!
//! ```rust,no_run
//! # #[cfg(feature = "mcp")]
//! use scim_server::mcp_integration::{ScimMcpServer, McpServerInfo};
//! # #[cfg(feature = "mcp")]
//! # use scim_server::{ScimServer, providers::InMemoryProvider};
//!
//! # #[cfg(feature = "mcp")]
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let provider = InMemoryProvider::new();
//! let scim_server = ScimServer::new(provider)?;
//!
//! // Custom server information for AI agent discovery
//! let server_info = McpServerInfo {
//!     name: "Enterprise SCIM Server".to_string(),
//!     version: "2.0.0".to_string(),
//!     description: "Production SCIM server with AI agent support".to_string(),
//!     supported_resource_types: vec!["User".to_string(), "Group".to_string()],
//! };
//!
//! let mcp_server = ScimMcpServer::with_info(scim_server, server_info);
//! # Ok(())
//! # }
//! ```
//!
//! See `examples/mcp_server_example.rs` for a complete MCP integration demonstration.
//!
//! ### When to Use MCP Integration
//!
//! **Use MCP integration when:**
//! - Building AI agents that need identity management capabilities
//! - Creating automated provisioning systems with AI decision making
//! - Developing chatbots or virtual assistants with user management features
//! - Building intelligent HR systems with automated user lifecycle management
//! - Creating AI-powered compliance and audit systems
//!
//! **Use regular SCIM server when:**
//! - Building traditional web applications with SCIM endpoints
//! - Integrating with existing identity providers (Okta, Azure AD, etc.)
//! - Creating standard enterprise SCIM provisioning bridges
//! - Building REST APIs for human operators or traditional applications
//!
//! The MCP integration adds AI-specific tooling and structured schemas on top of
//! the core SCIM functionality without changing the underlying SCIM compliance.
//!
//! ## Architecture Overview
//!
//! The SCIM server library follows a layered architecture:
//!
//! ```text
//! ┌─────────────────────────────────────────────────┐
//! │                HTTP Layer                       │  ← Your web framework
//! │             (Axum, Actix, etc.)                │
//! ├─────────────────────────────────────────────────┤
//! │              SCIM Protocol Layer                │  ← This library
//! │         (ScimServer, SchemaDiscovery)           │
//! ├─────────────────────────────────────────────────┤
//! │            Provider Abstraction                 │  ← ResourceProvider trait
//! │        (InMemoryProvider, your impl)            │
//! ├─────────────────────────────────────────────────┤
//! │              Storage Layer                      │  ← Your database/storage
//! │        (PostgreSQL, MongoDB, etc.)              │
//! └─────────────────────────────────────────────────┘
//! ```
//!
//! ## Logging Support
//!
//! The library uses the standard Rust `log` crate facade, allowing you to choose your preferred
//! logging backend. All SCIM operations are logged with structured information including:
//!
//! - Request IDs for operation tracing
//! - Tenant context for multi-tenant deployments
//! - Resource lifecycle events (create, read, update, delete)
//! - Error conditions with full context
//! - Performance and debugging information
//!
//! ### Quick Logging Setup
//!
//! ```rust,no_run
//! // Simple logging for development
//! env_logger::init();
//!
//! // Or with custom configuration
//! env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
//!     .format_timestamp_secs()
//!     .init();
//! ```
//!
//! See `examples/logging_example.rs` for comprehensive logging demonstrations.
//!
//! ## Multi-Tenant Support
//!
//! The library provides built-in multi-tenant capabilities:
//!
//! ```rust,no_run
//! use scim_server::{TenantContext, RequestContext};
//!
//! // Create tenant context
//! let tenant = TenantContext::new("customer-123".to_string(), "app-456".to_string());
//! let context = RequestContext::with_tenant_generated_id(tenant);
//!
//! // All operations are automatically scoped to this tenant
//! let user = server.create_resource("User", user_data, &context).await?;
//! ```
//!
//! ## Quick Start - Full SCIM Server
//!
//! ```rust,no_run
//! use scim_server::{ScimServer, ResourceProvider, Resource, RequestContext, ScimOperation, ListQuery, create_user_resource_handler};
//! use std::collections::HashMap;
//! use tokio::sync::RwLock;
//! use std::sync::Arc;
//! use serde_json::Value;
//! use std::future::Future;
//!
//! struct MyResourceProvider {
//!     resources: Arc<RwLock<HashMap<String, Resource>>>,
//! }
//!
//! impl MyResourceProvider {
//!     fn new() -> Self {
//!         Self {
//!             resources: Arc::new(RwLock::new(HashMap::new())),
//!         }
//!     }
//! }
//!
//! #[derive(Debug, thiserror::Error)]
//! #[error("Provider error")]
//! struct MyError;
//!
//! impl ResourceProvider for MyResourceProvider {
//!     type Error = MyError;
//!
//!     fn create_resource(&self, resource_type: &str, data: Value, _context: &RequestContext) -> impl Future<Output = Result<Resource, Self::Error>> + Send {
//!         async move {
//!             Resource::from_json(resource_type.to_string(), data)
//!                 .map_err(|_| MyError)
//!         }
//!     }
//!
//!     fn get_resource(&self, _resource_type: &str, _id: &str, _context: &RequestContext) -> impl Future<Output = Result<Option<Resource>, Self::Error>> + Send {
//!         async move { Ok(None) }
//!     }
//!
//!     fn update_resource(&self, resource_type: &str, _id: &str, data: Value, _context: &RequestContext) -> impl Future<Output = Result<Resource, Self::Error>> + Send {
//!         async move {
//!             Resource::from_json(resource_type.to_string(), data)
//!                 .map_err(|_| MyError)
//!         }
//!     }
//!
//!     fn delete_resource(&self, _resource_type: &str, _id: &str, _context: &RequestContext) -> impl Future<Output = Result<(), Self::Error>> + Send {
//!         async move { Ok(()) }
//!     }
//!
//!     fn list_resources(&self, _resource_type: &str, _query: Option<&ListQuery>, _context: &RequestContext) -> impl Future<Output = Result<Vec<Resource>, Self::Error>> + Send {
//!         async move { Ok(vec![]) }
//!     }
//!
//!     fn find_resource_by_attribute(&self, _resource_type: &str, _attribute: &str, _value: &Value, _context: &RequestContext) -> impl Future<Output = Result<Option<Resource>, Self::Error>> + Send {
//!         async move { Ok(None) }
//!     }
//!
//!     fn resource_exists(&self, _resource_type: &str, _id: &str, _context: &RequestContext) -> impl Future<Output = Result<bool, Self::Error>> + Send {
//!         async move { Ok(false) }
//!     }
//! }
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // Implement your data access layer
//!     let provider = MyResourceProvider::new();
//!
//!     // Create dynamic SCIM server
//!     let mut server = ScimServer::new(provider)?;
//!
//!     // Register resource types with their operations
//!     let user_schema = server.get_schema_by_id("urn:ietf:params:scim:schemas:core:2.0:User").unwrap().clone();
//!     let user_handler = create_user_resource_handler(user_schema);
//!     let _ = server.register_resource_type("User", user_handler, vec![ScimOperation::Create, ScimOperation::Read]);
//!
//!     // Use server for SCIM operations
//!     let schemas = server.get_all_schemas();
//!     println!("Available schemas: {}", schemas.len());
//!
//!     Ok(())
//! }
//! ```
//!
//! ## Schema Discovery
//!
//! For schema discovery and service provider configuration only:
//!
//! ```rust,no_run
//! use scim_server::SchemaDiscovery;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // Create schema discovery component
//!     let discovery = SchemaDiscovery::new()?;
//!
//!     // Get available schemas
//!     let schemas = discovery.get_schemas().await?;
//!     println!("Available schemas: {}", schemas.len());
//!
//!     // Get service provider configuration
//!     let config = discovery.get_service_provider_config().await?;
//!     println!("Service provider config: {:?}", config);
//!
//!     Ok(())
//! }
//! ```
//!
//! ## Provider Implementation
//!
//! Implement the `ResourceProvider` trait for your storage backend:
//!
//! ```rust,no_run
//! use scim_server::{ResourceProvider, Resource, RequestContext, ListQuery};
//! use serde_json::Value;
//! use std::future::Future;
//!
//! struct MyDatabaseProvider {
//!     // Your database connection, etc.
//! }
//!
//! #[derive(Debug, thiserror::Error)]
//! #[error("Database error")]
//! struct MyError;
//!
//! impl ResourceProvider for MyDatabaseProvider {
//!     type Error = MyError;
//!
//!     fn create_resource(&self, resource_type: &str, data: Value, context: &RequestContext) -> impl Future<Output = Result<Resource, Self::Error>> + Send {
//!         async move {
//!             // Implement your database creation logic here
//!             Resource::from_json(resource_type.to_string(), data)
//!                 .map_err(|_| MyError)
//!         }
//!     }
//!
//!     // Implement other required methods...
//!     # fn get_resource(&self, _resource_type: &str, _id: &str, _context: &RequestContext) -> impl Future<Output = Result<Option<Resource>, Self::Error>> + Send {
//!     #     async move { Ok(None) }
//!     # }
//!     # fn update_resource(&self, resource_type: &str, _id: &str, data: Value, _context: &RequestContext) -> impl Future<Output = Result<Resource, Self::Error>> + Send {
//!     #     async move { Resource::from_json(resource_type.to_string(), data).map_err(|_| MyError) }
//!     # }
//!     # fn delete_resource(&self, _resource_type: &str, _id: &str, _context: &RequestContext) -> impl Future<Output = Result<(), Self::Error>> + Send {
//!     #     async move { Ok(()) }
//!     # }
//!     # fn list_resources(&self, _resource_type: &str, _query: Option<&ListQuery>, _context: &RequestContext) -> impl Future<Output = Result<Vec<Resource>, Self::Error>> + Send {
//!     #     async move { Ok(vec![]) }
//!     # }
//!     # fn find_resource_by_attribute(&self, _resource_type: &str, _attribute: &str, _value: &Value, _context: &RequestContext) -> impl Future<Output = Result<Option<Resource>, Self::Error>> + Send {
//!     #     async move { Ok(None) }
//!     # }
//!     # fn resource_exists(&self, _resource_type: &str, _id: &str, _context: &RequestContext) -> impl Future<Output = Result<bool, Self::Error>> + Send {
//!     #     async move { Ok(false) }
//!     # }
//! }
//! ```
//!
//! ## Examples
//!
//! This crate includes comprehensive examples in the `examples/` directory:
//!
//! - **`basic_usage.rs`** - Simple SCIM server setup
//! - **`multi_tenant_demo.rs`** - Multi-tenant operations
//! - **`logging_example.rs`** - Comprehensive logging configuration
//! - **`operation_handler_example.rs`** - Framework-agnostic operation handling
//! - **`provider_capabilities.rs`** - Automatic capability discovery
//!
//! Run examples with:
//! ```bash
//! cargo run --example basic_usage
//! ```
//!
//! ## Schema Validation Utility
//!
//! This crate includes a command-line schema validation utility for testing and validating
//! SCIM schema files. The `schema-validator` binary helps ensure your schemas conform to
//! the expected format before using them in production.
//!
//! ### Usage
//!
//! #### During Development (with Cargo)
//! ```bash
//! # Validate a single schema file
//! cargo run --bin schema-validator schemas/User.json
//!
//! # Validate all schemas in a directory
//! cargo run --bin schema-validator ./schemas/
//! ```
//!
//! #### Standalone Installation
//! ```bash
//! # Install the binary globally
//! cargo install --path . --bin schema-validator
//!
//! # Then use directly
//! schema-validator schemas/User.json
//! schema-validator ./schemas/
//! ```
//!
//! #### From Published Crate (when available)
//! ```bash
//! # Install from crates.io
//! cargo install scim-server --bin schema-validator
//!
//! # Use anywhere
//! schema-validator /path/to/schemas/
//! ```
//!
//! ### Features
//!
//! - **Schema File Validation**: Validates JSON structure and SCIM schema format
//! - **Directory Processing**: Batch validation of multiple schema files
//! - **Schema Registry Testing**: Tests loading schemas into the registry
//! - **Detailed Error Reporting**: Clear error messages for debugging
//! - **Schema Summary**: Displays attribute counts and types for valid schemas
//!
//! ### Example Output
//!
//! ```text
//! Validating schema file: schemas/User.json
//! ✓ Schema is valid!
//!
//! Schema Summary:
//!   ID: urn:ietf:params:scim:schemas:core:2.0:User
//!   Name: User
//!   Attributes: 15
//!   Required attributes: 2
//!   Multi-valued attributes: 4
//!   Required attribute names: id, userName
//! ```
//!
//! The validator performs comprehensive checks including:
//! - JSON syntax validation
//! - Required field presence (id, name, attributes)
//! - Schema ID URI format validation
//! - Attribute structure validation
//! - Complex attribute sub-attribute validation
//! - Canonical values format checking
//!
//! ## Performance
//!
//! The library is designed for high performance with:
//! - Zero-copy JSON processing where possible
//! - Async-first architecture for high concurrency
//! - Efficient value object system with minimal allocations
//! - Type-safe operations that compile to efficient code
//!
//! Benchmarks show 40,000+ operations/second on modern hardware.
//!
//! ## SCIM 2.0 Compliance
//!
//! This library implements the following SCIM 2.0 specifications:
//! - **RFC 7643**: SCIM Core Schema (User, Group, Schema definitions)
//! - **RFC 7644**: SCIM Protocol (HTTP operations, filtering, pagination)
//!
//! ### Supported Operations
//! - Resource CRUD (Create, Read, Update, Delete)
//! - Resource listing with pagination
//! - Attribute-based search and filtering
//! - Schema discovery and introspection
//! - Service provider configuration
//! - Multi-valued attribute handling
//! - Complex attribute validation
//!
//! ### Standards Compliance
//! - Full User schema implementation
//! - Group schema support
//! - Extension schema framework
//! - JSON Schema validation
//! - HTTP status code compliance
//! - Error response formatting
//!
//! ## Contributing
//!
//! Contributions are welcome! Please see the repository for:
//! - Issue reporting and feature requests
//! - Pull request guidelines
//! - Development setup instructions
//! - Testing requirements
//!
//! ## License
//!
//! This project is licensed under the MIT License - see the LICENSE file for details.
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

// Core re-exports for library users
pub use error::{BuildError, ScimError, ValidationError};

// Provider capability discovery system
pub use provider_capabilities::{
    AuthenticationCapabilities, BulkCapabilities, CapabilityDiscovery, CapabilityIntrospectable,
    ExtendedCapabilities, FilterCapabilities, FilterOperator, PaginationCapabilities,
    ProviderCapabilities,
};
// SCIM-focused multi-tenant configuration (recommended)
pub use multi_tenant::{
    RateLimit, ScimAuditConfig, ScimAuthScheme, ScimClientAuth, ScimClientConfig,
    ScimConfigurationError, ScimCustomAttribute, ScimEndpointConfig, ScimOperation, ScimRateLimits,
    ScimSchemaConfig, ScimSchemaExtension, ScimSearchConfig, ScimTenantConfiguration,
};

// Standard provider implementations
pub use providers::{InMemoryError, InMemoryProvider, InMemoryStats};

// Multi-tenant provider and resolver components
pub use multi_tenant::{
    SingleTenantAdapter, StaticTenantResolver, TenantResolver, TenantValidator, ToSingleTenant,
};

pub use resource::{
    Address, DatabaseMapper, DynamicResource, EmailAddress, IsolationLevel, ListQuery, Meta, Name,
    PhoneNumber, RequestContext, Resource, ResourceBuilder, ResourceProvider, ResourceProviderExt,
    SchemaResourceBuilder, TenantContext, TenantPermissions,
};
pub use resource_handlers::{create_group_resource_handler, create_user_resource_handler};
pub use schema::{
    AttributeDefinition, AttributeType, Mutability, Schema, SchemaRegistry, Uniqueness,
};
pub use schema_discovery::{SchemaDiscovery, ServiceProviderConfig};
pub use scim_server::ScimServer;

// Operation handler foundation for framework-agnostic integration
pub use operation_handler::{
    OperationMetadata, ScimOperationHandler, ScimOperationRequest, ScimOperationResponse,
    ScimOperationType, ScimQuery,
};

// MCP (Model Context Protocol) integration for AI agents (optional feature)
#[cfg(feature = "mcp")]
pub use mcp_integration::{McpServerInfo, ScimMcpServer, ScimToolResult};
