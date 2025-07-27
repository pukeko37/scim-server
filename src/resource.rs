//! Resource model and provider trait for SCIM resources.
//!
//! This module defines the core resource abstractions that users implement
//! to provide data access for SCIM operations. The design emphasizes
//! type safety and async patterns while keeping the interface simple.

use crate::error::ScimError;
use crate::schema::Schema;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;

/// Generic SCIM resource representation.
///
/// A resource is a structured data object with a type identifier and JSON data.
/// This design provides flexibility while maintaining schema validation through
/// the server layer.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Resource {
    /// The type of this resource (e.g., "User", "Group")
    pub resource_type: String,
    /// The resource data as validated JSON
    pub data: Value,
}

impl Resource {
    /// Create a new resource with the given type and data.
    ///
    /// # Arguments
    /// * `resource_type` - The SCIM resource type identifier
    /// * `data` - The resource data as a JSON value
    ///
    /// # Example
    /// ```rust
    /// use scim_server::Resource;
    /// use serde_json::json;
    ///
    /// let user_data = json!({
    ///     "userName": "jdoe",
    ///     "displayName": "John Doe"
    /// });
    /// let resource = Resource::new("User".to_string(), user_data);
    /// ```
    pub fn new(resource_type: String, data: Value) -> Self {
        Self {
            resource_type,
            data,
        }
    }

    /// Get the unique identifier of this resource.
    ///
    /// Returns the "id" field from the resource data if present.
    pub fn get_id(&self) -> Option<&str> {
        self.data.get("id")?.as_str()
    }

    /// Get the userName field for User resources.
    ///
    /// This is a convenience method for accessing the required userName field.
    pub fn get_username(&self) -> Option<&str> {
        self.data.get("userName")?.as_str()
    }

    /// Get a specific attribute value from the resource data.
    ///
    /// # Arguments
    /// * `attribute_name` - The name of the attribute to retrieve
    pub fn get_attribute(&self, attribute_name: &str) -> Option<&Value> {
        self.data.get(attribute_name)
    }

    /// Set a specific attribute value in the resource data.
    ///
    /// # Arguments
    /// * `attribute_name` - The name of the attribute to set
    /// * `value` - The value to set
    pub fn set_attribute(&mut self, attribute_name: String, value: Value) {
        if let Some(obj) = self.data.as_object_mut() {
            obj.insert(attribute_name, value);
        }
    }

    /// Get the schemas associated with this resource.
    pub fn get_schemas(&self) -> Vec<String> {
        self.data
            .get("schemas")
            .and_then(|s| s.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str())
                    .map(|s| s.to_string())
                    .collect()
            })
            .unwrap_or_else(|| {
                // Default schema based on resource type
                match self.resource_type.as_str() {
                    "User" => vec!["urn:ietf:params:scim:schemas:core:2.0:User".to_string()],
                    "Group" => vec!["urn:ietf:params:scim:schemas:core:2.0:Group".to_string()],
                    _ => vec![],
                }
            })
    }

    /// Add metadata to the resource.
    ///
    /// This method sets common SCIM metadata fields like resourceType,
    /// created, lastModified, and location.
    pub fn add_metadata(&mut self, base_url: &str, created: &str, last_modified: &str) {
        let meta = serde_json::json!({
            "resourceType": self.resource_type,
            "created": created,
            "lastModified": last_modified,
            "location": format!("{}/{}s/{}", base_url, self.resource_type, self.get_id().unwrap_or("")),
            "version": format!("W/\"{}-{}\"", self.get_id().unwrap_or(""), last_modified)
        });

        self.set_attribute("meta".to_string(), meta);
    }

    /// Check if this resource is active.
    ///
    /// Returns the value of the "active" field, defaulting to true if not present.
    pub fn is_active(&self) -> bool {
        self.data
            .get("active")
            .and_then(|v| v.as_bool())
            .unwrap_or(true)
    }

    /// Get all email addresses from the resource.
    pub fn get_emails(&self) -> Vec<EmailAddress> {
        self.data
            .get("emails")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|email| {
                        let value = email.get("value")?.as_str()?;
                        Some(EmailAddress {
                            value: value.to_string(),
                            email_type: email
                                .get("type")
                                .and_then(|t| t.as_str())
                                .map(|s| s.to_string()),
                            primary: email.get("primary").and_then(|p| p.as_bool()),
                            display: email
                                .get("display")
                                .and_then(|d| d.as_str())
                                .map(|s| s.to_string()),
                        })
                    })
                    .collect()
            })
            .unwrap_or_default()
    }
}

/// Email address representation extracted from User resources.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmailAddress {
    pub value: String,
    #[serde(rename = "type")]
    pub email_type: Option<String>,
    pub primary: Option<bool>,
    pub display: Option<String>,
}

/// Request context for SCIM operations.
///
/// Provides contextual information for each SCIM request, enabling
/// providers to implement proper auditing, logging, and request tracking.
#[derive(Debug, Clone)]
pub struct RequestContext {
    /// Unique identifier for this request
    pub request_id: String,
    /// Optional user context for authorization
    pub user_context: Option<UserContext>,
    /// Additional metadata for the request
    pub metadata: HashMap<String, String>,
}

impl RequestContext {
    /// Create a new request context with a specific request ID.
    pub fn new(request_id: String) -> Self {
        Self {
            request_id,
            user_context: None,
            metadata: HashMap::new(),
        }
    }

    /// Create a new request context with a generated request ID.
    pub fn with_generated_id() -> Self {
        Self {
            request_id: uuid::Uuid::new_v4().to_string(),
            user_context: None,
            metadata: HashMap::new(),
        }
    }

    /// Add user context for authorization.
    pub fn with_user_context(mut self, user_context: UserContext) -> Self {
        self.user_context = Some(user_context);
        self
    }

    /// Add metadata to the request context.
    pub fn with_metadata(mut self, key: String, value: String) -> Self {
        self.metadata.insert(key, value);
        self
    }

    /// Get metadata value by key.
    pub fn get_metadata(&self, key: &str) -> Option<&String> {
        self.metadata.get(key)
    }
}

impl Default for RequestContext {
    fn default() -> Self {
        Self::with_generated_id()
    }
}

/// Dynamic attribute handler for schema-driven operations
#[derive(Clone)]
pub enum AttributeHandler {
    Getter(Arc<dyn Fn(&Value) -> Option<Value> + Send + Sync>),
    Setter(Arc<dyn Fn(&mut Value, Value) -> Result<(), ScimError> + Send + Sync>),
    Transformer(Arc<dyn Fn(&Value, &str) -> Option<Value> + Send + Sync>),
}

/// Trait for mapping between SCIM schema and implementation schema (e.g., database)
pub trait SchemaMapper: Send + Sync {
    fn to_implementation(&self, scim_data: &Value) -> Result<Value, ScimError>;
    fn from_implementation(&self, impl_data: &Value) -> Result<Value, ScimError>;
}

/// Database schema mapper for converting between SCIM and database formats
pub struct DatabaseMapper {
    pub table_name: String,
    pub column_mappings: HashMap<String, String>, // SCIM attribute -> DB column
}

impl DatabaseMapper {
    pub fn new(table_name: &str, mappings: HashMap<String, String>) -> Self {
        Self {
            table_name: table_name.to_string(),
            column_mappings: mappings,
        }
    }
}

impl SchemaMapper for DatabaseMapper {
    fn to_implementation(&self, scim_data: &Value) -> Result<Value, ScimError> {
        let mut db_data = serde_json::Map::new();

        if let Some(obj) = scim_data.as_object() {
            for (scim_attr, db_column) in &self.column_mappings {
                if let Some(value) = obj.get(scim_attr) {
                    db_data.insert(db_column.clone(), value.clone());
                }
            }
        }

        Ok(Value::Object(db_data))
    }

    fn from_implementation(&self, impl_data: &Value) -> Result<Value, ScimError> {
        let mut scim_data = serde_json::Map::new();

        if let Some(obj) = impl_data.as_object() {
            for (scim_attr, db_column) in &self.column_mappings {
                if let Some(value) = obj.get(db_column) {
                    scim_data.insert(scim_attr.clone(), value.clone());
                }
            }
        }

        Ok(Value::Object(scim_data))
    }
}

/// Handler for a specific resource type containing all its dynamic behaviors
#[derive(Clone)]
pub struct ResourceHandler {
    pub schema: Schema,
    pub handlers: HashMap<String, AttributeHandler>,
    pub mappers: Vec<Arc<dyn SchemaMapper>>,
    pub custom_methods:
        HashMap<String, Arc<dyn Fn(&DynamicResource) -> Result<Value, ScimError> + Send + Sync>>,
}

impl std::fmt::Debug for ResourceHandler {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ResourceHandler")
            .field("schema", &self.schema)
            .field("handlers", &format!("{} handlers", self.handlers.len()))
            .field("mappers", &format!("{} mappers", self.mappers.len()))
            .field(
                "custom_methods",
                &format!("{} custom methods", self.custom_methods.len()),
            )
            .finish()
    }
}

/// Builder for creating resource handlers with fluent API
pub struct SchemaResourceBuilder {
    schema: Schema,
    handlers: HashMap<String, AttributeHandler>,
    mappers: Vec<Arc<dyn SchemaMapper>>,
    custom_methods:
        HashMap<String, Arc<dyn Fn(&DynamicResource) -> Result<Value, ScimError> + Send + Sync>>,
}

impl SchemaResourceBuilder {
    pub fn new(schema: Schema) -> Self {
        Self {
            schema,
            handlers: HashMap::new(),
            mappers: Vec::new(),
            custom_methods: HashMap::new(),
        }
    }

    pub fn with_getter<F>(mut self, attribute: &str, getter: F) -> Self
    where
        F: Fn(&Value) -> Option<Value> + Send + Sync + 'static,
    {
        self.handlers.insert(
            format!("get_{}", attribute),
            AttributeHandler::Getter(Arc::new(getter)),
        );
        self
    }

    pub fn with_setter<F>(mut self, attribute: &str, setter: F) -> Self
    where
        F: Fn(&mut Value, Value) -> Result<(), ScimError> + Send + Sync + 'static,
    {
        self.handlers.insert(
            format!("set_{}", attribute),
            AttributeHandler::Setter(Arc::new(setter)),
        );
        self
    }

    pub fn with_transformer<F>(mut self, attribute: &str, transformer: F) -> Self
    where
        F: Fn(&Value, &str) -> Option<Value> + Send + Sync + 'static,
    {
        self.handlers.insert(
            format!("transform_{}", attribute),
            AttributeHandler::Transformer(Arc::new(transformer)),
        );
        self
    }

    pub fn with_custom_method<F>(mut self, method_name: &str, method: F) -> Self
    where
        F: Fn(&DynamicResource) -> Result<Value, ScimError> + Send + Sync + 'static,
    {
        self.custom_methods
            .insert(method_name.to_string(), Arc::new(method));
        self
    }

    pub fn with_mapper(mut self, mapper: Arc<dyn SchemaMapper>) -> Self {
        self.mappers.push(mapper);
        self
    }

    pub fn with_database_mapping(
        self,
        table_name: &str,
        column_mappings: HashMap<String, String>,
    ) -> Self {
        self.with_mapper(Arc::new(DatabaseMapper::new(table_name, column_mappings)))
    }

    pub fn build(self) -> ResourceHandler {
        ResourceHandler {
            schema: self.schema,
            handlers: self.handlers,
            mappers: self.mappers,
            custom_methods: self.custom_methods,
        }
    }
}

/// Dynamic resource that uses registered handlers for operations
#[derive(Clone, Debug)]
pub struct DynamicResource {
    pub resource_type: String,
    pub data: Value,
    pub handler: Arc<ResourceHandler>,
}

impl DynamicResource {
    pub fn new(resource_type: String, data: Value, handler: Arc<ResourceHandler>) -> Self {
        Self {
            resource_type,
            data,
            handler,
        }
    }

    pub fn get_attribute_dynamic(&self, attribute: &str) -> Option<Value> {
        let getter_key = format!("get_{}", attribute);
        if let Some(AttributeHandler::Getter(getter)) = self.handler.handlers.get(&getter_key) {
            getter(&self.data)
        } else {
            // Fallback to direct access
            self.data.get(attribute).cloned()
        }
    }

    pub fn set_attribute_dynamic(
        &mut self,
        attribute: &str,
        value: Value,
    ) -> Result<(), ScimError> {
        let setter_key = format!("set_{}", attribute);
        if let Some(AttributeHandler::Setter(setter)) = self.handler.handlers.get(&setter_key) {
            setter(&mut self.data, value)
        } else {
            // Fallback to direct setting
            if let Some(obj) = self.data.as_object_mut() {
                obj.insert(attribute.to_string(), value);
            }
            Ok(())
        }
    }

    pub fn call_custom_method(&self, method_name: &str) -> Result<Value, ScimError> {
        if let Some(method) = self.handler.custom_methods.get(method_name) {
            method(self)
        } else {
            Err(ScimError::MethodNotFound(method_name.to_string()))
        }
    }

    pub fn to_implementation_schema(&self, mapper_index: usize) -> Result<Value, ScimError> {
        if let Some(mapper) = self.handler.mappers.get(mapper_index) {
            mapper.to_implementation(&self.data)
        } else {
            Err(ScimError::MapperNotFound(mapper_index))
        }
    }

    pub fn from_implementation_schema(
        &mut self,
        impl_data: &Value,
        mapper_index: usize,
    ) -> Result<(), ScimError> {
        if let Some(mapper) = self.handler.mappers.get(mapper_index) {
            self.data = mapper.from_implementation(impl_data)?;
            Ok(())
        } else {
            Err(ScimError::MapperNotFound(mapper_index))
        }
    }
}

/// Supported SCIM operations for resource types
#[derive(Debug, Clone, PartialEq)]
pub enum ScimOperation {
    Create,
    Read,
    Update,
    Delete,
    List,
    Search,
}

/// Dynamic resource provider trait for generic SCIM operations
#[async_trait]
pub trait DynamicResourceProvider {
    type Error: std::error::Error + Send + Sync + 'static;

    /// Generic create operation for any resource type
    async fn create_resource(
        &self,
        resource_type: &str,
        data: Value,
        context: &RequestContext,
    ) -> Result<Resource, Self::Error>;

    /// Generic read operation for any resource type
    async fn get_resource(
        &self,
        resource_type: &str,
        id: &str,
        context: &RequestContext,
    ) -> Result<Option<Resource>, Self::Error>;

    /// Generic update operation for any resource type
    async fn update_resource(
        &self,
        resource_type: &str,
        id: &str,
        data: Value,
        context: &RequestContext,
    ) -> Result<Resource, Self::Error>;

    /// Generic delete operation for any resource type
    async fn delete_resource(
        &self,
        resource_type: &str,
        id: &str,
        context: &RequestContext,
    ) -> Result<(), Self::Error>;

    /// Generic list operation for any resource type
    async fn list_resources(
        &self,
        resource_type: &str,
        context: &RequestContext,
    ) -> Result<Vec<Resource>, Self::Error>;

    /// Generic search by attribute for any resource type
    async fn find_resource_by_attribute(
        &self,
        resource_type: &str,
        attribute: &str,
        value: &Value,
        context: &RequestContext,
    ) -> Result<Option<Resource>, Self::Error>;

    /// Check if resource exists
    async fn resource_exists(
        &self,
        resource_type: &str,
        id: &str,
        context: &RequestContext,
    ) -> Result<bool, Self::Error>;
}

/// User context for authorization and auditing.
#[derive(Debug, Clone)]
pub struct UserContext {
    /// User identifier
    pub user_id: String,
    /// User roles or permissions
    pub roles: Vec<String>,
    /// Additional user attributes
    pub attributes: HashMap<String, String>,
}

impl UserContext {
    /// Create a new user context.
    pub fn new(user_id: String, roles: Vec<String>) -> Self {
        Self {
            user_id,
            roles,
            attributes: HashMap::new(),
        }
    }

    /// Check if the user has a specific role.
    pub fn has_role(&self, role: &str) -> bool {
        self.roles.contains(&role.to_string())
    }
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
/// use scim_server::{DynamicResourceProvider, Resource, RequestContext};
/// use async_trait::async_trait;
/// use serde_json::Value;
/// use std::collections::HashMap;
/// use std::sync::Arc;
/// use tokio::sync::RwLock;
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
/// #[async_trait]
/// impl DynamicResourceProvider for InMemoryProvider {
///     type Error = ProviderError;
///
///     async fn create_resource(
///         &self,
///         resource_type: &str,
///         data: Value,
///         _context: &RequestContext,
///     ) -> Result<Resource, Self::Error> {
///         let resource = Resource::new(resource_type.to_string(), data);
///         let id = resource.get_id().unwrap_or_default().to_string();
///
///         let mut resources = self.resources.write().await;
///         resources.entry(resource_type.to_string())
///             .or_insert_with(HashMap::new)
///             .insert(id, resource.clone());
///         Ok(resource)
///     }
///
///     async fn get_resource(
///         &self,
///         resource_type: &str,
///         id: &str,
///         _context: &RequestContext,
///     ) -> Result<Option<Resource>, Self::Error> {
///         let resources = self.resources.read().await;
///         Ok(resources.get(resource_type)
///             .and_then(|type_resources| type_resources.get(id))
///             .cloned())
///     }
///
///     async fn update_resource(
///         &self,
///         resource_type: &str,
///         id: &str,
///         data: Value,
///         _context: &RequestContext,
///     ) -> Result<Resource, Self::Error> {
///         let resource = Resource::new(resource_type.to_string(), data);
///         let mut resources = self.resources.write().await;
///         resources.entry(resource_type.to_string())
///             .or_insert_with(HashMap::new)
///             .insert(id.to_string(), resource.clone());
///         Ok(resource)
///     }
///
///     async fn delete_resource(
///         &self,
///         resource_type: &str,
///         id: &str,
///         _context: &RequestContext,
///     ) -> Result<(), Self::Error> {
///         let mut resources = self.resources.write().await;
///         if let Some(type_resources) = resources.get_mut(resource_type) {
///             type_resources.remove(id);
///         }
///         Ok(())
///     }
///
///     async fn list_resources(
///         &self,
///         resource_type: &str,
///         _context: &RequestContext,
///     ) -> Result<Vec<Resource>, Self::Error> {
///         let resources = self.resources.read().await;
///         Ok(resources.get(resource_type)
///             .map(|type_resources| type_resources.values().cloned().collect())
///             .unwrap_or_default())
///     }
///
///     async fn find_resource_by_attribute(
///         &self,
///         resource_type: &str,
///         attribute: &str,
///         value: &Value,
///         _context: &RequestContext,
///     ) -> Result<Option<Resource>, Self::Error> {
///         let resources = self.resources.read().await;
///         Ok(resources.get(resource_type)
///             .and_then(|type_resources| {
///                 type_resources.values().find(|resource| {
///                     resource.get_attribute(attribute) == Some(value)
///                 })
///             })
///             .cloned())
///     }
///
///     async fn resource_exists(
///         &self,
///         resource_type: &str,
///         id: &str,
///         _context: &RequestContext,
///     ) -> Result<bool, Self::Error> {
///         let resources = self.resources.read().await;
///         Ok(resources.get(resource_type)
///             .map(|type_resources| type_resources.contains_key(id))
///             .unwrap_or(false))
///     }
/// }
/// ```

/// Query parameters for listing resources (future extension).
///
/// This structure is prepared for future pagination and filtering support
/// but is not used in the MVP implementation.
#[derive(Debug, Clone, Default)]
pub struct ListQuery {
    /// Maximum number of results to return
    pub count: Option<usize>,
    /// Starting index for pagination
    pub start_index: Option<usize>,
    /// Filter expression
    pub filter: Option<String>,
    /// Attributes to include in results
    pub attributes: Vec<String>,
    /// Attributes to exclude from results
    pub excluded_attributes: Vec<String>,
}

impl ListQuery {
    /// Create a new empty query.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the maximum count.
    pub fn with_count(mut self, count: usize) -> Self {
        self.count = Some(count);
        self
    }

    /// Set the starting index.
    pub fn with_start_index(mut self, start_index: usize) -> Self {
        self.start_index = Some(start_index);
        self
    }

    /// Set a filter expression.
    pub fn with_filter(mut self, filter: String) -> Self {
        self.filter = Some(filter);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_resource_creation() {
        let data = json!({
            "userName": "testuser",
            "displayName": "Test User"
        });
        let resource = Resource::new("User".to_string(), data);

        assert_eq!(resource.resource_type, "User");
        assert_eq!(resource.get_username(), Some("testuser"));
    }

    #[test]
    fn test_resource_id_extraction() {
        let data = json!({
            "id": "12345",
            "userName": "testuser"
        });
        let resource = Resource::new("User".to_string(), data);

        assert_eq!(resource.get_id(), Some("12345"));
    }

    #[test]
    fn test_resource_schemas() {
        let data = json!({
            "userName": "testuser"
        });
        let resource = Resource::new("User".to_string(), data);

        let schemas = resource.get_schemas();
        assert_eq!(schemas.len(), 1);
        assert_eq!(schemas[0], "urn:ietf:params:scim:schemas:core:2.0:User");
    }

    #[test]
    fn test_email_extraction() {
        let data = json!({
            "userName": "testuser",
            "emails": [
                {
                    "value": "test@example.com",
                    "type": "work",
                    "primary": true
                }
            ]
        });
        let resource = Resource::new("User".to_string(), data);

        let emails = resource.get_emails();
        assert_eq!(emails.len(), 1);
        assert_eq!(emails[0].value, "test@example.com");
        assert_eq!(emails[0].email_type, Some("work".to_string()));
        assert_eq!(emails[0].primary, Some(true));
    }

    #[test]
    fn test_request_context_creation() {
        let context = RequestContext::new("test-request".to_string());
        assert!(!context.request_id.is_empty());

        let context_with_id = RequestContext::new("test-123".to_string());
        assert_eq!(context_with_id.request_id, "test-123");
    }

    #[test]
    fn test_user_context() {
        let user_context = UserContext::new(
            "user123".to_string(),
            vec!["admin".to_string(), "user".to_string()],
        );

        assert!(user_context.has_role("admin"));
        assert!(user_context.has_role("user"));
        assert!(!user_context.has_role("superuser"));
    }

    #[test]
    fn test_resource_active_status() {
        let active_data = json!({
            "userName": "testuser",
            "active": true
        });
        let active_resource = Resource::new("User".to_string(), active_data);
        assert!(active_resource.is_active());

        let inactive_data = json!({
            "userName": "testuser",
            "active": false
        });
        let inactive_resource = Resource::new("User".to_string(), inactive_data);
        assert!(!inactive_resource.is_active());

        let no_active_data = json!({
            "userName": "testuser"
        });
        let default_resource = Resource::new("User".to_string(), no_active_data);
        assert!(default_resource.is_active()); // Default to true
    }
}
