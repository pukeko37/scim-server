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
/// use scim_server::{DynamicScimServer, DynamicResourceProvider, Resource, RequestContext, ScimOperation};
/// use async_trait::async_trait;
/// use std::collections::HashMap;
/// use tokio::sync::RwLock;
/// use std::sync::Arc;
/// use serde_json::Value;
///
/// struct MyResourceProvider {
///     resources: Arc<RwLock<HashMap<String, Resource>>>,
/// }
///
/// impl MyResourceProvider {
///     fn new() -> Self {
///         Self {
///             resources: Arc::new(RwLock::new(HashMap::new())),
///         }
///     }
/// }
///
/// #[derive(Debug, thiserror::Error)]
/// #[error("Provider error")]
/// struct MyError;
///
/// #[async_trait]
/// impl DynamicResourceProvider for MyResourceProvider {
///     type Error = MyError;
///
///     async fn create_resource(&self, resource_type: &str, data: Value, _context: &RequestContext) -> Result<Resource, Self::Error> {
///         Ok(Resource::new(resource_type.to_string(), data))
///     }
///
///     async fn get_resource(&self, resource_type: &str, _id: &str, _context: &RequestContext) -> Result<Option<Resource>, Self::Error> {
///         Ok(None)
///     }
///
///     async fn update_resource(&self, resource_type: &str, _id: &str, data: Value, _context: &RequestContext) -> Result<Resource, Self::Error> {
///         Ok(Resource::new(resource_type.to_string(), data))
///     }
///
///     async fn delete_resource(&self, _resource_type: &str, _id: &str, _context: &RequestContext) -> Result<(), Self::Error> {
///         Ok(())
///     }
///
///     async fn list_resources(&self, _resource_type: &str, _context: &RequestContext) -> Result<Vec<Resource>, Self::Error> {
///         Ok(vec![])
///     }
///
///     async fn find_resource_by_attribute(&self, _resource_type: &str, _attribute: &str, _value: &Value, _context: &RequestContext) -> Result<Option<Resource>, Self::Error> {
///         Ok(None)
///     }
///
///     async fn resource_exists(&self, _resource_type: &str, _id: &str, _context: &RequestContext) -> Result<bool, Self::Error> {
///         Ok(false)
///     }
/// }
///
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     // Implement your data access layer
///     let provider = MyResourceProvider::new();
///
///     // Create dynamic SCIM server
///     let mut server = DynamicScimServer::new(provider)?;
///
///     // Register resource types with their operations
///     let user_schema = server.get_schema_by_id("urn:ietf:params:scim:schemas:core:2.0:User").unwrap().clone();
///     let user_handler = scim_server::create_user_resource_handler(user_schema);
///     let _ = server.register_resource_type("User", user_handler, vec![ScimOperation::Create, ScimOperation::Read]);
///
///     // Use server for SCIM operations
///     let schemas = server.get_all_schemas();
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
    SchemaResourceBuilder, ScimOperation,
};
pub use schema::{
    AttributeDefinition, AttributeType, Mutability, Schema, SchemaRegistry, Uniqueness,
};
pub use server::{ScimServer, ServiceProviderConfig};
pub use user_handler::{create_group_resource_handler, create_user_resource_handler};
