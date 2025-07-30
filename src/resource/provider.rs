//! Resource provider trait for implementing SCIM data access.
//!
//! This module defines the core trait that users must implement to provide
//! data storage and retrieval for SCIM resources. The design is async-first
//! and provides comprehensive error handling.

use super::core::{ListQuery, RequestContext, Resource};
use serde_json::Value;
use std::future::Future;

/// Resource provider trait for generic SCIM operations
pub trait ResourceProvider {
    type Error: std::error::Error + Send + Sync + 'static;

    /// Generic create operation for any resource type
    fn create_resource(
        &self,
        resource_type: &str,
        data: Value,
        context: &RequestContext,
    ) -> impl Future<Output = Result<Resource, Self::Error>> + Send;

    /// Generic read operation for any resource type
    fn get_resource(
        &self,
        resource_type: &str,
        id: &str,
        context: &RequestContext,
    ) -> impl Future<Output = Result<Option<Resource>, Self::Error>> + Send;

    /// Generic update operation for any resource type
    fn update_resource(
        &self,
        resource_type: &str,
        id: &str,
        data: Value,
        context: &RequestContext,
    ) -> impl Future<Output = Result<Resource, Self::Error>> + Send;

    /// Generic delete operation for any resource type
    fn delete_resource(
        &self,
        resource_type: &str,
        id: &str,
        context: &RequestContext,
    ) -> impl Future<Output = Result<(), Self::Error>> + Send;

    /// Generic list operation for any resource type
    fn list_resources(
        &self,
        resource_type: &str,
        query: Option<&ListQuery>,
        context: &RequestContext,
    ) -> impl Future<Output = Result<Vec<Resource>, Self::Error>> + Send;

    /// Find a resource by attribute value
    fn find_resource_by_attribute(
        &self,
        resource_type: &str,
        attribute: &str,
        value: &Value,
        context: &RequestContext,
    ) -> impl Future<Output = Result<Option<Resource>, Self::Error>> + Send;

    /// Check if resource exists
    fn resource_exists(
        &self,
        resource_type: &str,
        id: &str,
        context: &RequestContext,
    ) -> impl Future<Output = Result<bool, Self::Error>> + Send;
}

/// Trait for implementing SCIM resource data access.
///
/// This trait defines the interface that users must implement to provide
/// data storage and retrieval for SCIM resources. The design is async-first
/// and provides comprehensive error handling.
///
/// # Example Implementation
///
/// ```rust,no_run
/// use scim_server::{ResourceProvider, Resource, RequestContext, ListQuery};
/// use serde_json::Value;
/// use std::collections::HashMap;
/// use std::sync::Arc;
/// use tokio::sync::RwLock;
/// use std::future::Future;
///
/// struct InMemoryProvider {
///     resources: Arc<RwLock<HashMap<String, HashMap<String, Resource>>>>,
/// }
///
/// impl InMemoryProvider {
///     fn new() -> Self {
///         Self {
///             resources: Arc::new(RwLock::new(HashMap::new())),
///         }
///     }
/// }
///
/// #[derive(Debug, thiserror::Error)]
/// #[error("Provider error")]
/// struct ProviderError;
///
/// impl ResourceProvider for InMemoryProvider {
///     type Error = ProviderError;
///
///     fn create_resource(
///         &self,
///         resource_type: &str,
///         data: Value,
///         _context: &RequestContext,
///     ) -> impl Future<Output = Result<Resource, Self::Error>> + Send {
///         async move {
///             let resource = Resource::new(resource_type.to_string(), data);
///             let id = resource.get_id().unwrap_or_default().to_string();
///
///             let mut resources = self.resources.write().await;
///             resources.entry(resource_type.to_string())
///                 .or_insert_with(HashMap::new)
///                 .insert(id, resource.clone());
///             Ok(resource)
///         }
///     }
///
///     fn get_resource(
///         &self,
///         resource_type: &str,
///         id: &str,
///         _context: &RequestContext,
///     ) -> impl Future<Output = Result<Option<Resource>, Self::Error>> + Send {
///         async move {
///             let resources = self.resources.read().await;
///             Ok(resources.get(resource_type)
///                 .and_then(|type_resources| type_resources.get(id))
///                 .cloned())
///         }
///     }
///
///     fn update_resource(
///         &self,
///         resource_type: &str,
///         id: &str,
///         data: Value,
///         _context: &RequestContext,
///     ) -> impl Future<Output = Result<Resource, Self::Error>> + Send {
///         async move {
///             let resource = Resource::new(resource_type.to_string(), data);
///             let mut resources = self.resources.write().await;
///             resources.entry(resource_type.to_string())
///                 .or_insert_with(HashMap::new)
///                 .insert(id.to_string(), resource.clone());
///             Ok(resource)
///         }
///     }
///
///     fn delete_resource(
///         &self,
///         resource_type: &str,
///         id: &str,
///         _context: &RequestContext,
///     ) -> impl Future<Output = Result<(), Self::Error>> + Send {
///         async move {
///             let mut resources = self.resources.write().await;
///             if let Some(type_resources) = resources.get_mut(resource_type) {
///                 type_resources.remove(id);
///             }
///             Ok(())
///         }
///     }
///
///     fn list_resources(
///         &self,
///         resource_type: &str,
///         _query: Option<&ListQuery>,
///         _context: &RequestContext,
///     ) -> impl Future<Output = Result<Vec<Resource>, Self::Error>> + Send {
///         async move {
///             let resources = self.resources.read().await;
///             Ok(resources.get(resource_type)
///                 .map(|type_resources| type_resources.values().cloned().collect())
///                 .unwrap_or_default())
///         }
///     }
///
///     fn find_resource_by_attribute(
///         &self,
///         resource_type: &str,
///         attribute: &str,
///         value: &Value,
///         _context: &RequestContext,
///     ) -> impl Future<Output = Result<Option<Resource>, Self::Error>> + Send {
///         async move {
///             let resources = self.resources.read().await;
///             Ok(resources.get(resource_type)
///                 .and_then(|type_resources| {
///                     type_resources.values().find(|resource| {
///                         resource.get_attribute(attribute) == Some(value)
///                     })
///                 })
///                 .cloned())
///         }
///     }
///
///     fn resource_exists(
///         &self,
///         resource_type: &str,
///         id: &str,
///         _context: &RequestContext,
///     ) -> impl Future<Output = Result<bool, Self::Error>> + Send {
///         async move {
///             let resources = self.resources.read().await;
///             Ok(resources.get(resource_type)
///                 .map(|type_resources| type_resources.contains_key(id))
///                 .unwrap_or(false))
///         }
///     }
/// }
/// ```
pub struct _ExampleDocumentation;
