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
/// use scim_server::{ScimServer, ResourceProvider, Resource, RequestContext, ScimOperation, ListQuery};
/// use std::collections::HashMap;
/// use tokio::sync::RwLock;
/// use std::sync::Arc;
/// use serde_json::Value;
/// use std::future::Future;
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
/// impl ResourceProvider for MyResourceProvider {
///     type Error = MyError;
///
///     fn create_resource(&self, resource_type: &str, data: Value, _context: &RequestContext) -> impl Future<Output = Result<Resource, Self::Error>> + Send {
///         async move { Ok(Resource::new(resource_type.to_string(), data)) }
///     }
///
///     fn get_resource(&self, _resource_type: &str, _id: &str, _context: &RequestContext) -> impl Future<Output = Result<Option<Resource>, Self::Error>> + Send {
///         async move { Ok(None) }
///     }
///
///     fn update_resource(&self, resource_type: &str, _id: &str, data: Value, _context: &RequestContext) -> impl Future<Output = Result<Resource, Self::Error>> + Send {
///         async move { Ok(Resource::new(resource_type.to_string(), data)) }
///     }
///
///     fn delete_resource(&self, _resource_type: &str, _id: &str, _context: &RequestContext) -> impl Future<Output = Result<(), Self::Error>> + Send {
///         async move { Ok(()) }
///     }
///
///     fn list_resources(&self, _resource_type: &str, _query: Option<&ListQuery>, _context: &RequestContext) -> impl Future<Output = Result<Vec<Resource>, Self::Error>> + Send {
///         async move { Ok(vec![]) }
///     }
///
///     fn find_resource_by_attribute(&self, _resource_type: &str, _attribute: &str, _value: &Value, _context: &RequestContext) -> impl Future<Output = Result<Option<Resource>, Self::Error>> + Send {
///         async move { Ok(None) }
///     }
///
///     fn resource_exists(&self, _resource_type: &str, _id: &str, _context: &RequestContext) -> impl Future<Output = Result<bool, Self::Error>> + Send {
///         async move { Ok(false) }
///     }
/// }
///
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     // Implement your data access layer
///     let provider = MyResourceProvider::new();
///
///     // Create dynamic SCIM server
///     let mut server = ScimServer::new(provider)?;
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
pub use dynamic_server::ScimServer;
pub use error::{BuildError, ScimError, ValidationError};
pub use resource::{
    DatabaseMapper, DynamicResource, ListQuery, RequestContext, Resource, ResourceProvider,
    SchemaResourceBuilder, ScimOperation,
};
pub use schema::{
    AttributeDefinition, AttributeType, Mutability, Schema, SchemaRegistry, Uniqueness,
};
pub use server::{ScimServer as TypeSafeScimServer, ServiceProviderConfig};
pub use user_handler::{create_group_resource_handler, create_user_resource_handler};
