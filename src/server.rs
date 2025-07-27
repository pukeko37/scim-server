//! Type-safe SCIM server implementation with state machine design.
//!
//! This module implements the core SCIM server using a type-parameterized state machine
//! to ensure compile-time safety and prevent invalid operations. The design follows
//! the builder pattern for configuration and provides async operations for all SCIM endpoints.

use crate::error::{BuildError, BuildResult, ScimError, ScimResult};
use crate::resource::{RequestContext, Resource, ResourceProvider};
use crate::schema::{Schema, SchemaRegistry};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::marker::PhantomData;

/// State marker for uninitialized server.
///
/// This state prevents any SCIM operations until the server is properly configured.
#[derive(Debug)]
pub struct Uninitialized;

/// State marker for fully configured and ready server.
///
/// Only servers in this state can perform SCIM operations.
#[derive(Debug)]
pub struct Ready;

/// Type-safe SCIM server with state machine design.
///
/// The server uses phantom types to encode its configuration state at compile time,
/// preventing invalid operations and ensuring proper initialization sequence.
///
/// # Type Parameters
/// * `State` - The current state of the server (Uninitialized or Ready)
///
/// # Example
/// ```rust,no_run
/// use scim_server::{ScimServer, ResourceProvider, Resource, RequestContext};
/// use async_trait::async_trait;
///
/// struct MyProvider;
///
/// #[derive(Debug, thiserror::Error)]
/// #[error("Provider error")]
/// struct MyError;
///
/// #[async_trait]
/// impl ResourceProvider for MyProvider {
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
///     let provider = MyProvider;
///
///     let server = ScimServer::builder()
///         .with_resource_provider(provider)
///         .build()?;
///
///     // Server is now ready for SCIM operations
///     let schemas = server.get_schemas().await?;
///     Ok(())
/// }
/// ```
pub struct ScimServer<State, P = ()> {
    inner: Option<ServerInner<P>>,
    _state: PhantomData<State>,
}

impl<State, P> std::fmt::Debug for ScimServer<State, P> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ScimServer")
            .field("state", &std::any::type_name::<State>())
            .field("configured", &self.inner.is_some())
            .finish()
    }
}

/// Internal server state shared across all server instances.
struct ServerInner<P> {
    schema_registry: SchemaRegistry,
    resource_provider: P,
    service_config: ServiceProviderConfig,
}

impl<P> std::fmt::Debug for ServerInner<P> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ServerInner")
            .field("schema_registry", &"SchemaRegistry")
            .field("resource_provider", &"ResourceProvider")
            .field("service_config", &self.service_config)
            .finish()
    }
}

impl ScimServer<Uninitialized> {
    /// Create a new server builder.
    ///
    /// This is the entry point for creating a new SCIM server. The builder pattern
    /// ensures all required components are configured before the server becomes operational.
    pub fn builder() -> ScimServerBuilder {
        ScimServerBuilder::new()
    }
}

impl<P> ScimServer<Ready, P>
where
    P: ResourceProvider,
{
    // Discovery endpoints

    /// Get all available schemas.
    ///
    /// Returns the complete list of schemas supported by this server instance.
    /// For the MVP, this includes only the core User schema.
    pub async fn get_schemas(&self) -> ScimResult<Vec<Schema>> {
        let inner = self.inner.as_ref().expect("Server should be initialized");
        Ok(inner
            .schema_registry
            .get_schemas()
            .into_iter()
            .cloned()
            .collect())
    }

    /// Get a specific schema by ID.
    ///
    /// # Arguments
    /// * `id` - The schema identifier (URI)
    ///
    /// # Returns
    /// * `Some(Schema)` if the schema exists
    /// * `None` if the schema is not found
    pub async fn get_schema(&self, id: &str) -> ScimResult<Option<Schema>> {
        let inner = self.inner.as_ref().expect("Server should be initialized");
        Ok(inner.schema_registry.get_schema(id).cloned())
    }

    /// Get the service provider configuration.
    ///
    /// Returns the capabilities and configuration of this SCIM service provider
    /// as defined in RFC 7644.
    pub async fn get_service_provider_config(&self) -> ScimResult<ServiceProviderConfig> {
        let inner = self.inner.as_ref().expect("Server should be initialized");
        Ok(inner.service_config.clone())
    }

    // User CRUD operations

    /// Create a new user resource.
    ///
    /// Validates the user data against the core User schema and delegates
    /// to the configured resource provider for persistence.
    ///
    /// # Arguments
    /// * `user_data` - The user data as JSON
    /// * `context` - Request context for auditing and authorization
    ///
    /// # Returns
    /// The created user resource with server-generated attributes
    ///
    /// # Errors
    /// * `ScimError::Validation` if the user data doesn't conform to schema
    /// * `ScimError::Provider` if the underlying storage operation fails
    pub async fn create_user(
        &self,
        user_data: Value,
        context: RequestContext,
    ) -> ScimResult<Resource> {
        let inner = self.inner.as_ref().expect("Server should be initialized");

        // Validate against schema before creating
        inner.schema_registry.validate_user(&user_data)?;

        // Ensure schemas field is present
        let mut validated_data = user_data;
        if !validated_data.as_object().unwrap().contains_key("schemas") {
            if let Some(obj) = validated_data.as_object_mut() {
                obj.insert(
                    "schemas".to_string(),
                    serde_json::json!(["urn:ietf:params:scim:schemas:core:2.0:User"]),
                );
            }
        }

        let resource = Resource::new("User".to_string(), validated_data);

        // Delegate to user implementation
        inner
            .resource_provider
            .create_user(resource, &context)
            .await
            .map_err(|e| ScimError::Provider(Box::new(e)))
    }

    /// Retrieve a user resource by ID.
    ///
    /// # Arguments
    /// * `id` - The unique identifier of the user
    /// * `context` - Request context for auditing and authorization
    ///
    /// # Returns
    /// * `Some(Resource)` if the user exists
    /// * `None` if the user is not found
    pub async fn get_user(
        &self,
        id: &str,
        context: RequestContext,
    ) -> ScimResult<Option<Resource>> {
        let inner = self.inner.as_ref().expect("Server should be initialized");
        inner
            .resource_provider
            .get_user(id, &context)
            .await
            .map_err(|e| ScimError::Provider(Box::new(e)))
    }

    /// Update an existing user resource.
    ///
    /// Validates the updated user data against the schema and performs
    /// a complete replacement of the user resource.
    ///
    /// # Arguments
    /// * `id` - The unique identifier of the user to update
    /// * `user_data` - The updated user data as JSON
    /// * `context` - Request context for auditing and authorization
    ///
    /// # Returns
    /// The updated user resource
    ///
    /// # Errors
    /// * `ScimError::Validation` if the user data doesn't conform to schema
    /// * `ScimError::Provider` if the underlying storage operation fails
    pub async fn update_user(
        &self,
        id: &str,
        user_data: Value,
        context: RequestContext,
    ) -> ScimResult<Resource> {
        let inner = self.inner.as_ref().expect("Server should be initialized");

        // Validate against schema before updating
        inner.schema_registry.validate_user(&user_data)?;

        // Ensure schemas field is present
        let mut validated_data = user_data;
        if !validated_data.as_object().unwrap().contains_key("schemas") {
            if let Some(obj) = validated_data.as_object_mut() {
                obj.insert(
                    "schemas".to_string(),
                    serde_json::json!(["urn:ietf:params:scim:schemas:core:2.0:User"]),
                );
            }
        }

        let resource = Resource::new("User".to_string(), validated_data);

        inner
            .resource_provider
            .update_user(id, resource, &context)
            .await
            .map_err(|e| ScimError::Provider(Box::new(e)))
    }

    /// Delete a user resource by ID.
    ///
    /// This operation is idempotent - it succeeds even if the user doesn't exist.
    ///
    /// # Arguments
    /// * `id` - The unique identifier of the user to delete
    /// * `context` - Request context for auditing and authorization
    pub async fn delete_user(&self, id: &str, context: RequestContext) -> ScimResult<()> {
        let inner = self.inner.as_ref().expect("Server should be initialized");
        inner
            .resource_provider
            .delete_user(id, &context)
            .await
            .map_err(|e| ScimError::Provider(Box::new(e)))
    }

    /// List all user resources.
    ///
    /// For the MVP, this returns all users without pagination or filtering.
    /// Future versions will add support for query parameters.
    ///
    /// # Arguments
    /// * `context` - Request context for auditing and authorization
    ///
    /// # Returns
    /// A vector of all user resources
    pub async fn list_users(&self, context: RequestContext) -> ScimResult<Vec<Resource>> {
        let inner = self.inner.as_ref().expect("Server should be initialized");
        inner
            .resource_provider
            .list_users(&context)
            .await
            .map_err(|e| ScimError::Provider(Box::new(e)))
    }

    /// Search for a user by username.
    ///
    /// This is a convenience method that searches for users with a specific userName.
    ///
    /// # Arguments
    /// * `username` - The username to search for
    /// * `context` - Request context for auditing and authorization
    ///
    /// # Returns
    /// * `Some(Resource)` if a user with the username exists
    /// * `None` if no user with the username is found
    pub async fn find_user_by_username(
        &self,
        username: &str,
        context: RequestContext,
    ) -> ScimResult<Option<Resource>> {
        let inner = self.inner.as_ref().expect("Server should be initialized");
        inner
            .resource_provider
            .find_user_by_username(username, &context)
            .await
            .map_err(|e| ScimError::Provider(Box::new(e)))
    }

    /// Check if a user exists by ID.
    ///
    /// This is a convenience method for existence checks without retrieving the full resource.
    ///
    /// # Arguments
    /// * `id` - The unique identifier of the user
    /// * `context` - Request context for auditing and authorization
    ///
    /// # Returns
    /// `true` if the user exists, `false` otherwise
    pub async fn user_exists(&self, id: &str, context: RequestContext) -> ScimResult<bool> {
        let inner = self.inner.as_ref().expect("Server should be initialized");
        inner
            .resource_provider
            .user_exists(id, &context)
            .await
            .map_err(|e| ScimError::Provider(Box::new(e)))
    }

    /// Get the schema registry for advanced usage.
    ///
    /// This provides access to the underlying schema registry for custom validation
    /// or schema introspection.
    pub fn schema_registry(&self) -> &SchemaRegistry {
        let inner = self.inner.as_ref().expect("Server should be initialized");
        &inner.schema_registry
    }

    /// Get the service provider configuration.
    pub fn service_config(&self) -> &ServiceProviderConfig {
        let inner = self.inner.as_ref().expect("Server should be initialized");
        &inner.service_config
    }
}

/// Builder for configuring and creating SCIM servers.
///
/// The builder ensures all required components are provided before
/// allowing server creation, preventing runtime configuration errors.
pub struct ScimServerBuilder<P = ()> {
    resource_provider: Option<P>,
    service_config: Option<ServiceProviderConfig>,
}

impl ScimServerBuilder {
    /// Create a new server builder.
    fn new() -> Self {
        Self {
            resource_provider: None,
            service_config: None,
        }
    }

    /// Build an empty server (for testing purposes).
    /// This will fail at runtime if no provider is set.
    pub fn build_empty(self) -> BuildResult<ScimServer<Ready, ()>> {
        Err(BuildError::MissingResourceProvider)
    }

    /// Configure the resource provider.
    ///
    /// The resource provider is responsible for all data storage and retrieval
    /// operations. This is the only required configuration for basic operation.
    ///
    /// # Arguments
    /// * `provider` - The resource provider implementation
    pub fn with_resource_provider<P>(self, provider: P) -> ScimServerBuilder<P>
    where
        P: ResourceProvider,
    {
        ScimServerBuilder {
            resource_provider: Some(provider),
            service_config: self.service_config,
        }
    }
}

impl<P> ScimServerBuilder<P>
where
    P: ResourceProvider,
{
    /// Configure the service provider capabilities.
    ///
    /// If not provided, a default configuration will be used with minimal capabilities.
    ///
    /// # Arguments
    /// * `config` - The service provider configuration
    pub fn with_service_config(mut self, config: ServiceProviderConfig) -> Self {
        self.service_config = Some(config);
        self
    }

    /// Build the configured SCIM server.
    ///
    /// This method validates the configuration and creates a ready-to-use server instance.
    ///
    /// # Returns
    /// A fully configured server in the Ready state
    ///
    /// # Errors
    /// * `BuildError::MissingResourceProvider` if no resource provider was configured
    pub fn build(self) -> BuildResult<ScimServer<Ready, P>> {
        let resource_provider = self
            .resource_provider
            .ok_or(BuildError::MissingResourceProvider)?;

        let service_config = self.service_config.unwrap_or_default();

        let inner = ServerInner {
            schema_registry: SchemaRegistry::new(),
            resource_provider,
            service_config,
        };

        Ok(ScimServer {
            inner: Some(inner),
            _state: PhantomData,
        })
    }
}

impl<P> std::fmt::Debug for ScimServerBuilder<P> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ScimServerBuilder")
            .field("has_resource_provider", &self.resource_provider.is_some())
            .field("has_service_config", &self.service_config.is_some())
            .finish()
    }
}

/// Service provider configuration as defined in RFC 7644.
///
/// This structure describes the capabilities and configuration of the SCIM service provider,
/// allowing clients to discover what features are supported.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ServiceProviderConfig {
    /// Whether PATCH operations are supported
    #[serde(rename = "patch")]
    pub patch_supported: bool,

    /// Whether bulk operations are supported
    #[serde(rename = "bulk")]
    pub bulk_supported: bool,

    /// Whether filtering is supported
    #[serde(rename = "filter")]
    pub filter_supported: bool,

    /// Whether password change operations are supported
    #[serde(rename = "changePassword")]
    pub change_password_supported: bool,

    /// Whether sorting is supported
    #[serde(rename = "sort")]
    pub sort_supported: bool,

    /// Whether ETags are supported for versioning
    #[serde(rename = "etag")]
    pub etag_supported: bool,

    /// Authentication schemes supported
    #[serde(rename = "authenticationSchemes")]
    pub authentication_schemes: Vec<AuthenticationScheme>,

    /// Maximum number of operations in a bulk request
    #[serde(rename = "bulk.maxOperations")]
    pub bulk_max_operations: Option<u32>,

    /// Maximum payload size for bulk operations
    #[serde(rename = "bulk.maxPayloadSize")]
    pub bulk_max_payload_size: Option<u64>,

    /// Maximum number of resources returned in a query
    #[serde(rename = "filter.maxResults")]
    pub filter_max_results: Option<u32>,
}

impl Default for ServiceProviderConfig {
    fn default() -> Self {
        Self {
            patch_supported: false,
            bulk_supported: false,
            filter_supported: false,
            change_password_supported: false,
            sort_supported: false,
            etag_supported: false,
            authentication_schemes: vec![],
            bulk_max_operations: None,
            bulk_max_payload_size: None,
            filter_max_results: Some(200),
        }
    }
}

/// Authentication scheme definition for service provider config.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AuthenticationScheme {
    /// Authentication scheme name
    pub name: String,
    /// Human-readable description
    pub description: String,
    /// URI for more information
    #[serde(rename = "specUri")]
    pub spec_uri: Option<String>,
    /// URI for documentation
    #[serde(rename = "documentationUri")]
    pub documentation_uri: Option<String>,
    /// Authentication type (e.g., "oauth2", "httpbasic")
    #[serde(rename = "type")]
    pub auth_type: String,
    /// Whether this scheme is the primary authentication method
    pub primary: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::resource::ResourceProvider;
    use async_trait::async_trait;
    use serde_json::json;
    use std::collections::HashMap;
    use std::sync::Arc;
    use tokio::sync::RwLock;

    // Test implementation of ResourceProvider
    #[derive(Debug)]
    struct TestProvider {
        users: Arc<RwLock<HashMap<String, Resource>>>,
    }

    impl TestProvider {
        fn new() -> Self {
            Self {
                users: Arc::new(RwLock::new(HashMap::new())),
            }
        }
    }

    #[derive(Debug, thiserror::Error)]
    #[error("Test provider error: {message}")]
    struct TestError {
        message: String,
    }

    #[async_trait]
    impl ResourceProvider for TestProvider {
        type Error = TestError;

        async fn create_user(
            &self,
            mut user: Resource,
            _context: &RequestContext,
        ) -> Result<Resource, Self::Error> {
            let id = uuid::Uuid::new_v4().to_string();
            user.set_attribute("id".to_string(), Value::String(id.clone()));

            let mut users = self.users.write().await;
            users.insert(id, user.clone());
            Ok(user)
        }

        async fn get_user(
            &self,
            id: &str,
            _context: &RequestContext,
        ) -> Result<Option<Resource>, Self::Error> {
            let users = self.users.read().await;
            Ok(users.get(id).cloned())
        }

        async fn update_user(
            &self,
            id: &str,
            mut user: Resource,
            _context: &RequestContext,
        ) -> Result<Resource, Self::Error> {
            user.set_attribute("id".to_string(), Value::String(id.to_string()));

            let mut users = self.users.write().await;
            users.insert(id.to_string(), user.clone());
            Ok(user)
        }

        async fn delete_user(
            &self,
            id: &str,
            _context: &RequestContext,
        ) -> Result<(), Self::Error> {
            let mut users = self.users.write().await;
            users.remove(id);
            Ok(())
        }

        async fn list_users(
            &self,
            _context: &RequestContext,
        ) -> Result<Vec<Resource>, Self::Error> {
            let users = self.users.read().await;
            Ok(users.values().cloned().collect())
        }
    }

    #[tokio::test]
    async fn test_server_builder() {
        let provider = TestProvider::new();
        let server = ScimServer::builder()
            .with_resource_provider(provider)
            .build()
            .expect("Failed to build server");

        // Test that the server is in Ready state and can perform operations
        let schemas = server.get_schemas().await.expect("Failed to get schemas");
        assert_eq!(schemas.len(), 1);
        assert_eq!(schemas[0].name, "User");
    }

    #[tokio::test]
    async fn test_server_operations() {
        let provider = TestProvider::new();
        let server = ScimServer::builder()
            .with_resource_provider(provider)
            .build()
            .expect("Failed to build server");

        let context = RequestContext::new();

        // Test user creation
        let user_data = json!({
            "userName": "testuser",
            "displayName": "Test User",
            "active": true
        });

        let created_user = server
            .create_user(user_data, context.clone())
            .await
            .expect("Failed to create user");

        assert!(created_user.get_id().is_some());
        assert_eq!(created_user.get_username(), Some("testuser"));

        // Test user retrieval
        let user_id = created_user.get_id().unwrap();
        let retrieved_user = server
            .get_user(user_id, context.clone())
            .await
            .expect("Failed to get user")
            .expect("User should exist");

        assert_eq!(retrieved_user.get_id(), Some(user_id));
        assert_eq!(retrieved_user.get_username(), Some("testuser"));

        // Test user listing
        let users = server
            .list_users(context.clone())
            .await
            .expect("Failed to list users");

        assert_eq!(users.len(), 1);
        assert_eq!(users[0].get_username(), Some("testuser"));

        // Test user deletion
        server
            .delete_user(user_id, context.clone())
            .await
            .expect("Failed to delete user");

        let deleted_user = server
            .get_user(user_id, context)
            .await
            .expect("Failed to check deleted user");

        assert!(deleted_user.is_none());
    }

    #[tokio::test]
    async fn test_schema_validation() {
        let provider = TestProvider::new();
        let server = ScimServer::builder()
            .with_resource_provider(provider)
            .build()
            .expect("Failed to build server");

        let context = RequestContext::new();

        // Test invalid user (missing required userName)
        let invalid_user = json!({
            "displayName": "Test User"
        });

        let result = server.create_user(invalid_user, context).await;
        assert!(result.is_err());
    }

    #[test]
    fn test_service_provider_config() {
        let config = ServiceProviderConfig::default();
        assert!(!config.patch_supported);
        assert!(!config.bulk_supported);
        assert!(!config.filter_supported);
        assert_eq!(config.filter_max_results, Some(200));
    }

    #[test]
    fn test_builder_missing_provider() {
        let result = ScimServer::builder().build_empty();
        assert!(result.is_err());
        if let Err(BuildError::MissingResourceProvider) = result {
            // Expected error
        } else {
            panic!("Expected MissingResourceProvider error");
        }
    }
}
