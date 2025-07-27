//! Dynamic SCIM server implementation with runtime resource type registration.
//!
//! This module provides a completely dynamic SCIM server that can handle any
//! resource type registered at runtime, eliminating hard-coded resource types
//! and enabling true schema-driven operations.

use crate::error::{ScimError, ScimResult};
use crate::resource::{
    DynamicResource, DynamicResourceProvider, ListQuery, RequestContext, ResourceHandler,
    ScimOperation,
};
use crate::schema::{Schema, SchemaRegistry};
use std::collections::HashMap;
use std::sync::Arc;

/// Completely dynamic SCIM server with no hard-coded resource types
pub struct DynamicScimServer<P> {
    provider: P,
    schema_registry: SchemaRegistry,
    resource_handlers: HashMap<String, Arc<ResourceHandler>>, // resource_type -> handler
    supported_operations: HashMap<String, Vec<ScimOperation>>, // resource_type -> supported ops
}

impl<P: DynamicResourceProvider> DynamicScimServer<P> {
    /// Create a new dynamic SCIM server
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
    ) -> ScimResult<DynamicResource> {
        // Check if resource type is supported
        self.ensure_operation_supported(resource_type, &ScimOperation::Create)?;

        // Get the handler for this resource type
        let handler = self.get_handler(resource_type)?;

        // Get the schema for validation
        let schema = self.get_schema_for_resource_type(resource_type)?;

        // Validate against schema
        self.schema_registry.validate_resource(&schema, &data)?;

        // Create dynamic resource
        let resource = DynamicResource::new(resource_type.to_string(), data, handler.clone());

        // Delegate to provider
        self.provider
            .create_resource(resource_type, resource, context)
            .await
            .map_err(|e| ScimError::ProviderError(e.to_string()))
    }

    /// Generic read operation
    pub async fn get_resource(
        &self,
        resource_type: &str,
        id: &str,
        context: &RequestContext,
    ) -> ScimResult<Option<DynamicResource>> {
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
    ) -> ScimResult<DynamicResource> {
        self.ensure_operation_supported(resource_type, &ScimOperation::Update)?;

        let handler = self.get_handler(resource_type)?;
        let schema = self.get_schema_for_resource_type(resource_type)?;

        // Validate the update data
        self.schema_registry.validate_resource(&schema, &data)?;

        let resource = DynamicResource::new(resource_type.to_string(), data, handler.clone());

        self.provider
            .update_resource(resource_type, id, resource, context)
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

    /// Generic list operation
    pub async fn list_resources(
        &self,
        resource_type: &str,
        query: &ListQuery,
        context: &RequestContext,
    ) -> ScimResult<Vec<DynamicResource>> {
        self.ensure_operation_supported(resource_type, &ScimOperation::List)?;

        self.provider
            .list_resources(resource_type, query, context)
            .await
            .map_err(|e| ScimError::ProviderError(e.to_string()))
    }

    /// Generic search by attribute (replaces find_user_by_username)
    pub async fn find_resource_by_attribute(
        &self,
        resource_type: &str,
        attribute: &str,
        value: &str,
        context: &RequestContext,
    ) -> ScimResult<Option<DynamicResource>> {
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
    use async_trait::async_trait;
    use serde_json::{Value, json};
    use std::collections::HashMap;

    #[derive(Debug, thiserror::Error)]
    #[error("Test provider error")]
    struct TestError;

    struct TestProvider {
        resources: std::sync::Mutex<HashMap<String, HashMap<String, DynamicResource>>>,
    }

    impl TestProvider {
        fn new() -> Self {
            Self {
                resources: std::sync::Mutex::new(HashMap::new()),
            }
        }
    }

    #[async_trait]
    impl DynamicResourceProvider for TestProvider {
        type Error = TestError;

        async fn create_resource(
            &self,
            resource_type: &str,
            mut data: DynamicResource,
            _context: &RequestContext,
        ) -> Result<DynamicResource, Self::Error> {
            let id = uuid::Uuid::new_v4().to_string();
            data.set_attribute_dynamic("id", Value::String(id.clone()))
                .map_err(|_| TestError)?;

            let mut resources = self.resources.lock().unwrap();
            resources
                .entry(resource_type.to_string())
                .or_insert_with(HashMap::new)
                .insert(id, data.clone());

            Ok(data)
        }

        async fn get_resource(
            &self,
            resource_type: &str,
            id: &str,
            _context: &RequestContext,
        ) -> Result<Option<DynamicResource>, Self::Error> {
            let resources = self.resources.lock().unwrap();
            Ok(resources
                .get(resource_type)
                .and_then(|type_resources| type_resources.get(id))
                .cloned())
        }

        async fn update_resource(
            &self,
            resource_type: &str,
            id: &str,
            mut data: DynamicResource,
            _context: &RequestContext,
        ) -> Result<DynamicResource, Self::Error> {
            data.set_attribute_dynamic("id", Value::String(id.to_string()))
                .map_err(|_| TestError)?;

            let mut resources = self.resources.lock().unwrap();
            if let Some(type_resources) = resources.get_mut(resource_type) {
                type_resources.insert(id.to_string(), data.clone());
            }

            Ok(data)
        }

        async fn delete_resource(
            &self,
            resource_type: &str,
            id: &str,
            _context: &RequestContext,
        ) -> Result<(), Self::Error> {
            let mut resources = self.resources.lock().unwrap();
            if let Some(type_resources) = resources.get_mut(resource_type) {
                type_resources.remove(id);
            }
            Ok(())
        }

        async fn list_resources(
            &self,
            resource_type: &str,
            _query: &ListQuery,
            _context: &RequestContext,
        ) -> Result<Vec<DynamicResource>, Self::Error> {
            let resources = self.resources.lock().unwrap();
            Ok(resources
                .get(resource_type)
                .map(|type_resources| type_resources.values().cloned().collect())
                .unwrap_or_default())
        }

        async fn find_resource_by_attribute(
            &self,
            resource_type: &str,
            attribute: &str,
            value: &str,
            _context: &RequestContext,
        ) -> Result<Option<DynamicResource>, Self::Error> {
            let resources = self.resources.lock().unwrap();
            if let Some(type_resources) = resources.get(resource_type) {
                for resource in type_resources.values() {
                    if let Some(attr_value) = resource.get_attribute_dynamic(attribute) {
                        if attr_value.as_str() == Some(value) {
                            return Ok(Some(resource.clone()));
                        }
                    }
                }
            }
            Ok(None)
        }

        async fn resource_exists(
            &self,
            resource_type: &str,
            id: &str,
            _context: &RequestContext,
        ) -> Result<bool, Self::Error> {
            let resources = self.resources.lock().unwrap();
            Ok(resources
                .get(resource_type)
                .map(|type_resources| type_resources.contains_key(id))
                .unwrap_or(false))
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
        let mut server = DynamicScimServer::new(provider).expect("Failed to create server");

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
        let mut server = DynamicScimServer::new(provider).expect("Failed to create server");

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
            .get_attribute_dynamic("id")
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
            retrieved_user.get_attribute_dynamic("userName"),
            Some(Value::String("testuser".to_string()))
        );
    }

    #[tokio::test]
    async fn test_unsupported_operation() {
        let provider = TestProvider::new();
        let mut server = DynamicScimServer::new(provider).expect("Failed to create server");

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
}
