//! Resource model and provider trait for SCIM resources.
//!
//! This module defines the core resource abstractions that users implement
//! to provide data access for SCIM operations. The design emphasizes
//! type safety and async patterns while keeping the interface simple.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

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
    /// Create a new request context with a generated ID.
    pub fn new() -> Self {
        Self {
            request_id: uuid::Uuid::new_v4().to_string(),
            user_context: None,
            metadata: HashMap::new(),
        }
    }

    /// Create a request context with a specific ID.
    pub fn with_id(request_id: String) -> Self {
        Self {
            request_id,
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
        Self::new()
    }
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
/// use scim_server::{ResourceProvider, Resource, RequestContext};
/// use async_trait::async_trait;
/// use std::collections::HashMap;
/// use std::sync::Arc;
/// use tokio::sync::RwLock;
///
/// struct InMemoryProvider {
///     users: Arc<RwLock<HashMap<String, Resource>>>,
/// }
///
/// impl InMemoryProvider {
///     fn new() -> Self {
///         Self {
///             users: Arc::new(RwLock::new(HashMap::new())),
///         }
///     }
/// }
///
/// #[derive(Debug, thiserror::Error)]
/// #[error("Provider error")]
/// struct ProviderError;
///
/// #[async_trait]
/// impl ResourceProvider for InMemoryProvider {
///     type Error = ProviderError;
///
///     async fn create_user(
///         &self,
///         user: Resource,
///         _context: &RequestContext,
///     ) -> Result<Resource, Self::Error> {
///         let mut users = self.users.write().await;
///         let id = uuid::Uuid::new_v4().to_string();
///         let mut user_with_id = user;
///         user_with_id.set_attribute("id".to_string(), serde_json::Value::String(id.clone()));
///         users.insert(id, user_with_id.clone());
///         Ok(user_with_id)
///     }
///
///     async fn get_user(
///         &self,
///         id: &str,
///         _context: &RequestContext,
///     ) -> Result<Option<Resource>, Self::Error> {
///         let users = self.users.read().await;
///         Ok(users.get(id).cloned())
///     }
///
///     async fn update_user(
///         &self,
///         id: &str,
///         user: Resource,
///         _context: &RequestContext,
///     ) -> Result<Resource, Self::Error> {
///         let mut users = self.users.write().await;
///         let mut updated_user = user;
///         updated_user.set_attribute("id".to_string(), serde_json::Value::String(id.to_string()));
///         users.insert(id.to_string(), updated_user.clone());
///         Ok(updated_user)
///     }
///
///     async fn delete_user(
///         &self,
///         id: &str,
///         _context: &RequestContext,
///     ) -> Result<(), Self::Error> {
///         let mut users = self.users.write().await;
///         users.remove(id);
///         Ok(())
///     }
///
///     async fn list_users(
///         &self,
///         _context: &RequestContext,
///     ) -> Result<Vec<Resource>, Self::Error> {
///         let users = self.users.read().await;
///         Ok(users.values().cloned().collect())
///     }
/// }
/// ```
#[async_trait]
pub trait ResourceProvider: Send + Sync {
    /// Error type for provider operations
    type Error: std::error::Error + Send + Sync + 'static;

    /// Create a new user resource.
    ///
    /// The provider should generate a unique ID for the user and return
    /// the created resource with all server-managed attributes populated.
    ///
    /// # Arguments
    /// * `user` - The user resource to create
    /// * `context` - Request context for auditing and authorization
    async fn create_user(
        &self,
        user: Resource,
        context: &RequestContext,
    ) -> Result<Resource, Self::Error>;

    /// Retrieve a user resource by ID.
    ///
    /// Returns `None` if the user doesn't exist.
    ///
    /// # Arguments
    /// * `id` - The unique identifier of the user
    /// * `context` - Request context for auditing and authorization
    async fn get_user(
        &self,
        id: &str,
        context: &RequestContext,
    ) -> Result<Option<Resource>, Self::Error>;

    /// Update an existing user resource.
    ///
    /// The provider should update the resource and return the updated version
    /// with any server-managed attributes refreshed.
    ///
    /// # Arguments
    /// * `id` - The unique identifier of the user to update
    /// * `user` - The updated user resource data
    /// * `context` - Request context for auditing and authorization
    async fn update_user(
        &self,
        id: &str,
        user: Resource,
        context: &RequestContext,
    ) -> Result<Resource, Self::Error>;

    /// Delete a user resource by ID.
    ///
    /// Should succeed even if the user doesn't exist (idempotent operation).
    ///
    /// # Arguments
    /// * `id` - The unique identifier of the user to delete
    /// * `context` - Request context for auditing and authorization
    async fn delete_user(&self, id: &str, context: &RequestContext) -> Result<(), Self::Error>;

    /// List all user resources.
    ///
    /// For the MVP, this returns all users without pagination or filtering.
    /// Future versions may add support for query parameters.
    ///
    /// # Arguments
    /// * `context` - Request context for auditing and authorization
    async fn list_users(&self, context: &RequestContext) -> Result<Vec<Resource>, Self::Error>;

    /// Search users by username (convenience method).
    ///
    /// Default implementation lists all users and filters by username.
    /// Providers can override this for more efficient implementation.
    async fn find_user_by_username(
        &self,
        username: &str,
        context: &RequestContext,
    ) -> Result<Option<Resource>, Self::Error> {
        let users = self.list_users(context).await?;
        Ok(users
            .into_iter()
            .find(|user| user.get_username() == Some(username)))
    }

    /// Check if a user exists by ID.
    ///
    /// Default implementation uses get_user but providers can optimize this.
    async fn user_exists(&self, id: &str, context: &RequestContext) -> Result<bool, Self::Error> {
        let user = self.get_user(id, context).await?;
        Ok(user.is_some())
    }
}

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
        let context = RequestContext::new();
        assert!(!context.request_id.is_empty());

        let context_with_id = RequestContext::with_id("test-123".to_string());
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
