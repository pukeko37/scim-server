//! # SCIM Server Library for Rust
//!
//! A comprehensive System for Cross-domain Identity Management (SCIM) server library
//! that enables developers to implement SCIM-compliant identity providers with minimal effort.
//!
//! ## Features
//!
//! - Type-safe state machine preventing invalid operations at compile time
//! - Trait-based architecture for flexible data access patterns
//! - Full RFC 7643/7644 compliance for core User schema
//! - Async-first design with functional programming patterns
//!
//! ## Quick Start
//!
/// ```rust,no_run
/// use scim_server::{ScimServer, ResourceProvider, Resource, RequestContext};
/// use async_trait::async_trait;
/// use std::collections::HashMap;
/// use tokio::sync::RwLock;
/// use std::sync::Arc;
///
/// struct MyResourceProvider {
///     users: Arc<RwLock<HashMap<String, Resource>>>,
/// }
///
/// impl MyResourceProvider {
///     fn new() -> Self {
///         Self {
///             users: Arc::new(RwLock::new(HashMap::new())),
///         }
///     }
/// }
///
/// #[derive(Debug, thiserror::Error)]
/// #[error("Provider error")]
/// struct MyError;
///
/// #[async_trait]
/// impl ResourceProvider for MyResourceProvider {
///     type Error = MyError;
///
///     async fn create_user(&self, user: Resource, _context: &RequestContext) -> Result<Resource, Self::Error> {
///         Ok(user)
///     }
///
///     async fn get_user(&self, _id: &str, _context: &RequestContext) -> Result<Option<Resource>, Self::Error> {
///         Ok(None)
///     }
///
///     async fn update_user(&self, _id: &str, user: Resource, _context: &RequestContext) -> Result<Resource, Self::Error> {
///         Ok(user)
///     }
///
///     async fn delete_user(&self, _id: &str, _context: &RequestContext) -> Result<(), Self::Error> {
///         Ok(())
///     }
///
///     async fn list_users(&self, _context: &RequestContext) -> Result<Vec<Resource>, Self::Error> {
///         Ok(vec![])
///     }
/// }
///
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     // Implement your data access layer
///     let provider = MyResourceProvider::new();
///
///     // Build the SCIM server (schemas loaded from JSON files)
///     let server = ScimServer::builder()
///         .with_resource_provider(provider)
///         .with_schema_dir(".") // Load schemas from current directory
///         .build()?;
///
///     // Use server for SCIM operations
///     let schemas = server.get_schemas().await?;
///     println!("Available schemas: {}", schemas.len());
///
///     Ok(())
/// }
/// ```
pub mod dynamic_server;
pub mod error;
pub mod resource;
pub mod schema;
pub mod server;
pub mod user_handler;

// Core re-exports for library users
pub use dynamic_server::DynamicScimServer;
pub use error::{BuildError, ScimError, ValidationError};
pub use resource::{
    DatabaseMapper, DynamicResource, DynamicResourceProvider, ListQuery, RequestContext, Resource,
    ResourceProvider, SchemaResourceBuilder, ScimOperation,
};
pub use schema::{
    AttributeDefinition, AttributeType, Mutability, Schema, SchemaRegistry, Uniqueness,
};
pub use server::{ScimServer, ScimServerBuilder, ServiceProviderConfig};
pub use user_handler::{create_group_resource_handler, create_user_resource_handler};

// State types (re-exported for advanced usage)
pub use server::{Ready, Uninitialized};
