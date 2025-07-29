//! Dynamic SCIM server implementation with runtime resource type registration.
//!
//! This module provides a completely dynamic SCIM server that can handle any
//! resource type registered at runtime, eliminating hard-coded resource types
//! and enabling true schema-driven operations.

use crate::error::{ScimError, ScimResult};
#[cfg(test)]
use crate::resource::ListQuery;
use crate::resource::{RequestContext, Resource, ResourceHandler, ResourceProvider, ScimOperation};
use crate::schema::{Schema, SchemaRegistry};
use std::collections::HashMap;
use std::sync::Arc;

/// Completely dynamic SCIM server with no hard-coded resource types
pub struct ScimServer<P> {
    provider: P,
    schema_registry: SchemaRegistry,
    resource_handlers: HashMap<String, Arc<ResourceHandler>>, // resource_type -> handler
    supported_operations: HashMap<String, Vec<ScimOperation>>, // resource_type -> supported ops
}

impl<P: ResourceProvider> ScimServer<P> {
    /// Create a new SCIM server
    pub fn new(provider: P) -> Result<Self, ScimError> {
        let schema_registry = SchemaRegistry::new()
            .map_err(|e| ScimError::internal(format!("Failed to create schema registry: {}", e)))?;

        Ok(Self {
            provider,
            schema_registry,
            resource_handlers: HashMap::new(),
            supported_operations: HashMap::new(),
        })
    }

    /// Register a resource type with its handler and supported operations
    pub fn register_resource_type(
        &mut self,
        resource_type: &str,
        handler: ResourceHandler,
        operations: Vec<ScimOperation>,
    ) -> Result<(), ScimError> {
        // Register the schema
        self.schema_registry
            .add_schema(handler.schema.clone())
            .map_err(|e| ScimError::internal(format!("Failed to add schema: {}", e)))?;

        // Register the handler
        self.resource_handlers
            .insert(resource_type.to_string(), Arc::new(handler));

        // Register supported operations
        self.supported_operations
            .insert(resource_type.to_string(), operations);

        Ok(())
    }

    /// Generic create operation for any resource type
    pub async fn create_resource(
        &self,
        resource_type: &str,
        data: serde_json::Value,
        context: &RequestContext,
    ) -> ScimResult<Resource> {
        // Check if resource type is supported
        self.ensure_operation_supported(resource_type, &ScimOperation::Create)?;

        // Get the schema for validation
        let schema = self.get_schema_for_resource_type(resource_type)?;

        // Validate against schema
        self.schema_registry.validate_resource(&schema, &data)?;

        // Delegate to provider
        self.provider
            .create_resource(resource_type, data, context)
            .await
            .map_err(|e| ScimError::ProviderError(e.to_string()))
    }

    /// Generic read operation
    pub async fn get_resource(
        &self,
        resource_type: &str,
        id: &str,
        context: &RequestContext,
    ) -> ScimResult<Option<Resource>> {
        // Check if resource type is supported
        self.ensure_operation_supported(resource_type, &ScimOperation::Read)?;

        self.provider
            .get_resource(resource_type, id, context)
            .await
            .map_err(|e| ScimError::ProviderError(e.to_string()))
    }

    /// Generic update operation
    pub async fn update_resource(
        &self,
        resource_type: &str,
        id: &str,
        data: serde_json::Value,
        context: &RequestContext,
    ) -> ScimResult<Resource> {
        self.ensure_operation_supported(resource_type, &ScimOperation::Update)?;

        // Get the schema for validation
        let schema = self.get_schema_for_resource_type(resource_type)?;

        // Validate against schema
        self.schema_registry.validate_resource(&schema, &data)?;

        self.provider
            .update_resource(resource_type, id, data, context)
            .await
            .map_err(|e| ScimError::ProviderError(e.to_string()))
    }

    /// Generic delete operation
    pub async fn delete_resource(
        &self,
        resource_type: &str,
        id: &str,
        context: &RequestContext,
    ) -> ScimResult<()> {
        self.ensure_operation_supported(resource_type, &ScimOperation::Delete)?;

        self.provider
            .delete_resource(resource_type, id, context)
            .await
            .map_err(|e| ScimError::ProviderError(e.to_string()))
    }

    /// Generic list operation for any resource type
    pub async fn list_resources(
        &self,
        resource_type: &str,
        context: &RequestContext,
    ) -> ScimResult<Vec<Resource>> {
        // Check if resource type is supported
        self.ensure_operation_supported(resource_type, &ScimOperation::List)?;

        self.provider
            .list_resources(resource_type, None, context)
            .await
            .map_err(|e| ScimError::internal(format!("Provider error: {}", e)))
    }

    /// Generic search by attribute (replaces find_user_by_username)
    pub async fn find_resource_by_attribute(
        &self,
        resource_type: &str,
        attribute: &str,
        value: &serde_json::Value,
        context: &RequestContext,
    ) -> ScimResult<Option<Resource>> {
        // Check if resource type is supported
        self.ensure_operation_supported(resource_type, &ScimOperation::Search)?;

        self.provider
            .find_resource_by_attribute(resource_type, attribute, value, context)
            .await
            .map_err(|e| ScimError::ProviderError(e.to_string()))
    }

    /// Check if a resource exists
    pub async fn resource_exists(
        &self,
        resource_type: &str,
        id: &str,
        context: &RequestContext,
    ) -> ScimResult<bool> {
        self.provider
            .resource_exists(resource_type, id, context)
            .await
            .map_err(|e| ScimError::ProviderError(e.to_string()))
    }

    /// Get all registered resource types
    pub fn get_supported_resource_types(&self) -> Vec<&str> {
        self.resource_handlers.keys().map(|s| s.as_str()).collect()
    }

    /// Get schema for any registered resource type
    pub fn get_resource_schema(&self, resource_type: &str) -> ScimResult<Schema> {
        let handler = self.get_handler(resource_type)?;
        Ok(handler.schema.clone())
    }

    /// Get all registered schemas
    pub fn get_all_schemas(&self) -> Vec<&Schema> {
        self.resource_handlers
            .values()
            .map(|handler| &handler.schema)
            .collect()
    }

    /// Get schema from schema registry by ID
    pub fn get_schema_by_id(&self, schema_id: &str) -> Option<&Schema> {
        self.schema_registry.get_schema(schema_id)
    }

    /// Get supported operations for a resource type
    pub fn get_supported_operations(&self, resource_type: &str) -> Option<&Vec<ScimOperation>> {
        self.supported_operations.get(resource_type)
    }

    /// Helper methods
    fn ensure_operation_supported(
        &self,
        resource_type: &str,
        operation: &ScimOperation,
    ) -> ScimResult<()> {
        let operations = self
            .supported_operations
            .get(resource_type)
            .ok_or_else(|| ScimError::UnsupportedResourceType(resource_type.to_string()))?;

        if !operations.contains(operation) {
            return Err(ScimError::UnsupportedOperation {
                resource_type: resource_type.to_string(),
                operation: format!("{:?}", operation),
            });
        }

        Ok(())
    }

    fn get_handler(&self, resource_type: &str) -> ScimResult<Arc<ResourceHandler>> {
        self.resource_handlers
            .get(resource_type)
            .cloned()
            .ok_or_else(|| ScimError::UnsupportedResourceType(resource_type.to_string()))
    }

    fn get_schema_for_resource_type(&self, resource_type: &str) -> ScimResult<Schema> {
        let handler = self.get_handler(resource_type)?;
        Ok(handler.schema.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::resource::SchemaResourceBuilder;
    use crate::schema::{AttributeDefinition, AttributeType, Mutability, Schema, Uniqueness};

    use serde_json::{Value, json};
    use std::collections::HashMap;
    use std::future::Future;
    use std::sync::Mutex;

    #[derive(Debug, thiserror::Error)]
    #[error("Test error")]
    struct TestError;

    #[derive(Debug)]
    struct TestProvider {
        resources: Arc<Mutex<HashMap<String, HashMap<String, Resource>>>>,
    }

    impl TestProvider {
        fn new() -> Self {
            Self {
                resources: Arc::new(Mutex::new(HashMap::new())),
            }
        }
    }

    impl ResourceProvider for TestProvider {
        type Error = TestError;

        fn create_resource(
            &self,
            resource_type: &str,
            mut data: Value,
            _context: &RequestContext,
        ) -> impl Future<Output = Result<Resource, Self::Error>> + Send {
            let resource_type = resource_type.to_string();
            let resources = Arc::clone(&self.resources);
            async move {
                let id = uuid::Uuid::new_v4().to_string();
                if let Some(obj) = data.as_object_mut() {
                    obj.insert("id".to_string(), Value::String(id.clone()));
                }

                let resource = Resource::new(resource_type.clone(), data);

                let mut resources = resources.lock().unwrap();
                resources
                    .entry(resource_type)
                    .or_insert_with(HashMap::new)
                    .insert(id, resource.clone());

                Ok(resource)
            }
        }

        fn get_resource(
            &self,
            resource_type: &str,
            id: &str,
            _context: &RequestContext,
        ) -> impl Future<Output = Result<Option<Resource>, Self::Error>> + Send {
            let resource_type = resource_type.to_string();
            let id = id.to_string();
            let resources = Arc::clone(&self.resources);
            async move {
                let resources = resources.lock().unwrap();
                Ok(resources
                    .get(&resource_type)
                    .and_then(|type_resources| type_resources.get(&id))
                    .cloned())
            }
        }

        fn update_resource(
            &self,
            resource_type: &str,
            id: &str,
            mut data: Value,
            _context: &RequestContext,
        ) -> impl Future<Output = Result<Resource, Self::Error>> + Send {
            let resource_type = resource_type.to_string();
            let id = id.to_string();
            let resources = Arc::clone(&self.resources);
            async move {
                if let Some(obj) = data.as_object_mut() {
                    obj.insert("id".to_string(), Value::String(id.clone()));
                }

                let resource = Resource::new(resource_type.clone(), data);

                let mut resources = resources.lock().unwrap();
                resources
                    .entry(resource_type)
                    .or_insert_with(HashMap::new)
                    .insert(id, resource.clone());

                Ok(resource)
            }
        }

        fn delete_resource(
            &self,
            resource_type: &str,
            id: &str,
            _context: &RequestContext,
        ) -> impl Future<Output = Result<(), Self::Error>> + Send {
            let resource_type = resource_type.to_string();
            let id = id.to_string();
            let resources = Arc::clone(&self.resources);
            async move {
                let mut resources = resources.lock().unwrap();
                if let Some(type_resources) = resources.get_mut(&resource_type) {
                    type_resources.remove(&id);
                }
                Ok(())
            }
        }

        fn list_resources(
            &self,
            resource_type: &str,
            _query: Option<&ListQuery>,
            _context: &RequestContext,
        ) -> impl Future<Output = Result<Vec<Resource>, Self::Error>> + Send {
            let resource_type = resource_type.to_string();
            let resources = Arc::clone(&self.resources);
            async move {
                let resources = resources.lock().unwrap();
                Ok(resources
                    .get(&resource_type)
                    .map(|type_resources| type_resources.values().cloned().collect())
                    .unwrap_or_default())
            }
        }

        fn find_resource_by_attribute(
            &self,
            resource_type: &str,
            attribute: &str,
            value: &Value,
            _context: &RequestContext,
        ) -> impl Future<Output = Result<Option<Resource>, Self::Error>> + Send {
            let resource_type = resource_type.to_string();
            let attribute = attribute.to_string();
            let value = value.clone();
            let resources = Arc::clone(&self.resources);
            async move {
                let resources = resources.lock().unwrap();
                Ok(resources
                    .get(&resource_type)
                    .and_then(|type_resources| {
                        type_resources
                            .values()
                            .find(|resource| resource.get_attribute(&attribute) == Some(&value))
                    })
                    .cloned())
            }
        }

        fn resource_exists(
            &self,
            resource_type: &str,
            id: &str,
            _context: &RequestContext,
        ) -> impl Future<Output = Result<bool, Self::Error>> + Send {
            let resource_type = resource_type.to_string();
            let id = id.to_string();
            let resources = Arc::clone(&self.resources);
            async move {
                let resources = resources.lock().unwrap();
                Ok(resources
                    .get(&resource_type)
                    .map(|type_resources| type_resources.contains_key(&id))
                    .unwrap_or(false))
            }
        }
    }

    fn create_test_user_schema() -> Schema {
        Schema {
            id: "urn:ietf:params:scim:schemas:core:2.0:User".to_string(),
            name: "User".to_string(),
            description: "User Account".to_string(),
            attributes: vec![
                AttributeDefinition {
                    name: "id".to_string(),
                    data_type: AttributeType::String,
                    required: false,
                    mutability: Mutability::ReadOnly,
                    uniqueness: Uniqueness::Server,
                    ..Default::default()
                },
                AttributeDefinition {
                    name: "userName".to_string(),
                    data_type: AttributeType::String,
                    required: true,
                    mutability: Mutability::ReadWrite,
                    uniqueness: Uniqueness::Server,
                    ..Default::default()
                },
                AttributeDefinition {
                    name: "displayName".to_string(),
                    data_type: AttributeType::String,
                    required: false,
                    mutability: Mutability::ReadWrite,
                    ..Default::default()
                },
            ],
        }
    }

    fn create_user_resource_handler(schema: Schema) -> ResourceHandler {
        SchemaResourceBuilder::new(schema)
            .with_getter("userName", |data| {
                data.get("userName")
                    .and_then(|v| v.as_str())
                    .map(|s| Value::String(s.to_string()))
            })
            .with_custom_method("get_username", |resource| {
                Ok(resource
                    .get_attribute_dynamic("userName")
                    .unwrap_or(Value::Null))
            })
            .with_database_mapping("users", {
                let mut mappings = HashMap::new();
                mappings.insert("userName".to_string(), "username".to_string());
                mappings.insert("displayName".to_string(), "full_name".to_string());
                mappings.insert("id".to_string(), "user_id".to_string());
                mappings
            })
            .build()
    }

    #[tokio::test]
    async fn test_dynamic_server_registration() {
        let provider = TestProvider::new();
        let mut server = ScimServer::new(provider).expect("Failed to create server");

        let user_schema = create_test_user_schema();
        let user_handler = create_user_resource_handler(user_schema);

        server
            .register_resource_type(
                "User",
                user_handler,
                vec![
                    ScimOperation::Create,
                    ScimOperation::Read,
                    ScimOperation::Update,
                    ScimOperation::Delete,
                    ScimOperation::List,
                    ScimOperation::Search,
                ],
            )
            .expect("Failed to register User resource type");

        let resource_types = server.get_supported_resource_types();
        assert_eq!(resource_types.len(), 1);
        assert!(resource_types.contains(&"User"));
    }

    #[tokio::test]
    async fn test_dynamic_create_and_get() {
        let provider = TestProvider::new();
        let mut server = ScimServer::new(provider).expect("Failed to create server");

        let user_schema = create_test_user_schema();
        let user_handler = create_user_resource_handler(user_schema);

        server
            .register_resource_type(
                "User",
                user_handler,
                vec![
                    ScimOperation::Create,
                    ScimOperation::Read,
                    ScimOperation::Search,
                ],
            )
            .expect("Failed to register User resource type");

        let user_data = json!({
            "userName": "testuser",
            "displayName": "Test User"
        });

        let context = RequestContext::new("test-req".to_string());

        // Create user
        let created_user = server
            .create_resource("User", user_data, &context)
            .await
            .expect("Failed to create user");

        let user_id = created_user
            .get_attribute("id")
            .unwrap()
            .as_str()
            .unwrap()
            .to_string();

        // Get user
        let retrieved_user = server
            .get_resource("User", &user_id, &context)
            .await
            .expect("Failed to get user")
            .expect("User not found");

        assert_eq!(
            retrieved_user.get_attribute("userName"),
            Some(&Value::String("testuser".to_string()))
        );
    }

    #[tokio::test]
    async fn test_unsupported_operation() {
        let provider = TestProvider::new();
        let mut server = ScimServer::new(provider).expect("Failed to create server");

        let user_schema = create_test_user_schema();
        let user_handler = create_user_resource_handler(user_schema);

        // Register with limited operations (no Delete)
        server
            .register_resource_type("User", user_handler, vec![ScimOperation::Create])
            .expect("Failed to register User resource type");

        let context = RequestContext::new("test-req".to_string());

        // Try to delete (should fail)
        let result = server.delete_resource("User", "123", &context).await;
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            ScimError::UnsupportedOperation { .. }
        ));
    }

    /// Test Group resource registration and CRUD operations
    #[tokio::test]
    async fn test_group_resource_operations() {
        let provider = TestProvider::new();
        let mut server = ScimServer::new(provider).expect("Failed to create server");

        // Create and register Group resource type
        let registry = SchemaRegistry::new().expect("Failed to create registry");
        let group_schema = registry.get_group_schema().clone();
        let group_handler = crate::resource_handlers::create_group_resource_handler(group_schema);

        server
            .register_resource_type(
                "Group",
                group_handler,
                vec![
                    ScimOperation::Create,
                    ScimOperation::Read,
                    ScimOperation::Update,
                    ScimOperation::Delete,
                    ScimOperation::List,
                ],
            )
            .expect("Failed to register Group resource type");

        let group_data = json!({
            "displayName": "Engineering Team",
            "members": [
                {
                    "value": "user-123",
                    "$ref": "https://example.com/v2/Users/user-123",
                    "type": "User"
                }
            ]
        });

        let context = RequestContext::new("test-req".to_string());

        // Create group
        let created_group = server
            .create_resource("Group", group_data, &context)
            .await
            .expect("Failed to create group");

        assert_eq!(
            created_group
                .get_attribute("displayName")
                .unwrap()
                .as_str()
                .unwrap(),
            "Engineering Team"
        );
        assert!(created_group.get_attribute("members").unwrap().is_array());

        let group_id = created_group
            .get_attribute("id")
            .unwrap()
            .as_str()
            .unwrap()
            .to_string();

        // Get group
        let retrieved_group = server
            .get_resource("Group", &group_id, &context)
            .await
            .expect("Failed to get group");

        let retrieved_group = retrieved_group.expect("Failed to retrieve group");
        assert_eq!(
            retrieved_group
                .get_attribute("displayName")
                .unwrap()
                .as_str()
                .unwrap(),
            "Engineering Team"
        );
        assert_eq!(
            retrieved_group
                .get_attribute("id")
                .unwrap()
                .as_str()
                .unwrap(),
            group_id
        );

        // Update group
        let updated_data = json!({
            "displayName": "Updated Engineering Team"
        });

        let updated_group = server
            .update_resource("Group", &group_id, updated_data, &context)
            .await
            .expect("Failed to update group");

        assert_eq!(
            updated_group
                .get_attribute("displayName")
                .unwrap()
                .as_str()
                .unwrap(),
            "Updated Engineering Team"
        );

        // List groups
        let groups_list = server
            .list_resources("Group", &context)
            .await
            .expect("Failed to list groups");

        assert!(!groups_list.is_empty());

        // Delete group
        server
            .delete_resource("Group", &group_id, &context)
            .await
            .expect("Failed to delete group");

        // Verify deletion - resource should return None
        let deleted_resource = server
            .get_resource("Group", &group_id, &context)
            .await
            .expect("Get operation should succeed even after deletion");

        assert!(
            deleted_resource.is_none(),
            "Resource should be None after deletion"
        );
    }

    /// Test Group schema validation in server context
    #[tokio::test]
    async fn test_group_validation_in_server() {
        let provider = TestProvider::new();
        let mut server = ScimServer::new(provider).expect("Failed to create server");

        let registry = SchemaRegistry::new().expect("Failed to create registry");
        let group_schema = registry.get_group_schema().clone();
        let group_handler = crate::resource_handlers::create_group_resource_handler(group_schema);

        server
            .register_resource_type("Group", group_handler, vec![ScimOperation::Create])
            .expect("Failed to register Group resource type");

        let context = RequestContext::new("test-req".to_string());

        // Test valid group creation
        let valid_group = json!({
            "displayName": "Valid Group",
            "members": []
        });

        let result = server.create_resource("Group", valid_group, &context).await;
        assert!(result.is_ok(), "Valid group should be created successfully");

        // Test invalid group creation (missing schemas will be added automatically)
        let minimal_group = json!({
            "displayName": "Minimal Group"
        });

        let result = server
            .create_resource("Group", minimal_group, &context)
            .await;
        assert!(
            result.is_ok(),
            "Minimal group should be created successfully"
        );
    }
}
