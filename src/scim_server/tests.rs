//! Test infrastructure and test cases for the SCIM server.
//!
//! This module contains all test-related code including the TestProvider
//! implementation and comprehensive test cases for the SCIM server functionality.

#[cfg(test)]
use super::core::ScimServer;
#[cfg(test)]
use crate::resource::{
    ListQuery, RequestContext, Resource, ResourceProvider, SchemaResourceBuilder, ScimOperation,
};
#[cfg(test)]
use crate::schema::{Schema, SchemaRegistry};

#[cfg(test)]
use serde_json::{Value, json};
#[cfg(test)]
use std::collections::HashMap;
#[cfg(test)]
use std::future::Future;
#[cfg(test)]
use std::sync::{Arc, Mutex};

#[cfg(test)]
#[derive(Debug, thiserror::Error)]
pub enum TestError {
    #[error("Test error")]
    Test,
    #[error("Validation error: {0}")]
    ValidationError(String),
}

#[cfg(test)]
#[derive(Debug)]
pub struct TestProvider {
    resources: Arc<Mutex<HashMap<String, HashMap<String, Resource>>>>,
}

#[cfg(test)]
impl TestProvider {
    pub fn new() -> Self {
        Self {
            resources: Arc::new(Mutex::new(HashMap::new())),
        }
    }
}

#[cfg(test)]
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

            let resource = Resource::from_json(resource_type.clone(), data)
                .map_err(|e| TestError::ValidationError(e.to_string()))?;

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

            let resource = Resource::from_json(resource_type.clone(), data)
                .map_err(|e| TestError::ValidationError(e.to_string()))?;

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

#[cfg(test)]
pub fn create_test_user_schema() -> Schema {
    let registry = SchemaRegistry::new().expect("Failed to create registry");
    registry.get_user_schema().clone()
}

#[cfg(test)]
pub fn create_user_resource_handler(schema: Schema) -> crate::resource::ResourceHandler {
    SchemaResourceBuilder::new(schema).build()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_dynamic_server_registration() {
        let provider = TestProvider::new();
        let mut server = ScimServer::new(provider).expect("Failed to create server");

        // Create and register a User resource type
        let user_schema = create_test_user_schema();
        let user_handler = create_user_resource_handler(user_schema);

        let result = server.register_resource_type(
            "User",
            user_handler,
            vec![ScimOperation::Create, ScimOperation::Read],
        );

        assert!(result.is_ok(), "Should register User resource type");

        // Verify registration
        let resource_types = server.get_supported_resource_types();
        assert!(
            resource_types.contains(&"User"),
            "User should be in supported resource types"
        );

        let operations = server.get_supported_operations("User");
        assert!(operations.is_some(), "User operations should be defined");
        assert_eq!(operations.unwrap().len(), 2, "Should have 2 operations");
    }

    #[tokio::test]
    async fn test_dynamic_create_and_get() {
        let provider = TestProvider::new();
        let mut server = ScimServer::new(provider).expect("Failed to create server");

        // Register User resource type
        let user_schema = create_test_user_schema();
        let user_handler = create_user_resource_handler(user_schema);

        server
            .register_resource_type(
                "User",
                user_handler,
                vec![ScimOperation::Create, ScimOperation::Read],
            )
            .expect("Failed to register User resource type");

        let context = RequestContext::new("test-request".to_string());

        // Create a user
        let user_data = json!({
            "userName": "testuser"
        });

        let created_user = server
            .create_resource("User", user_data, &context)
            .await
            .expect("Failed to create user");

        assert_eq!(created_user.resource_type, "User");
        assert_eq!(created_user.get_username(), Some("testuser"));

        // Get the user back
        let user_id = created_user.get_id().expect("User should have an ID");
        let retrieved_user = server
            .get_resource("User", user_id, &context)
            .await
            .expect("Failed to get user")
            .expect("User should exist");

        assert_eq!(retrieved_user.get_id(), Some(user_id));
        assert_eq!(retrieved_user.get_username(), Some("testuser"));
    }

    #[tokio::test]
    async fn test_unsupported_operation() {
        let provider = TestProvider::new();
        let mut server = ScimServer::new(provider).expect("Failed to create server");

        // Register User with only Create operation
        let user_schema = create_test_user_schema();
        let user_handler = create_user_resource_handler(user_schema);

        server
            .register_resource_type("User", user_handler, vec![ScimOperation::Create])
            .expect("Failed to register User resource type");

        let context = RequestContext::new("test-request".to_string());

        // Try to read (unsupported operation)
        let result = server.get_resource("User", "123", &context).await;

        assert!(result.is_err(), "Should fail for unsupported operation");
    }

    /// Test full Group resource lifecycle with dynamic server
    #[tokio::test]
    async fn test_group_resource_operations() {
        let provider = TestProvider::new();
        let mut server = ScimServer::new(provider).expect("Failed to create server");

        // Get Group schema and create handler
        let registry = SchemaRegistry::new().expect("Failed to create registry");
        let group_schema = registry.get_group_schema().clone();
        let group_handler = crate::resource_handlers::create_group_resource_handler(group_schema);

        // Register Group resource type with all operations
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

        let context = RequestContext::new("test-group-ops".to_string());

        // Test Create Group
        let group_data = json!({
            "displayName": "Test Group",
            "members": []
        });

        let created_group = server
            .create_resource("Group", group_data, &context)
            .await
            .expect("Failed to create group");

        assert_eq!(created_group.resource_type, "Group");
        let group_id = created_group
            .get_id()
            .expect("Group should have an ID")
            .to_string();

        // Test Read Group
        let retrieved_group = server
            .get_resource("Group", &group_id, &context)
            .await
            .expect("Failed to get group")
            .expect("Group should exist");

        assert_eq!(retrieved_group.get_id(), Some(group_id.as_str()));

        // Test Update Group
        let updated_data = json!({
            "displayName": "Updated Test Group",
            "members": []
        });

        let updated_group = server
            .update_resource("Group", &group_id, updated_data, &context)
            .await
            .expect("Failed to update group");

        assert_eq!(
            updated_group.get_attribute("displayName"),
            Some(&json!("Updated Test Group"))
        );

        // Test List Groups
        let groups = server
            .list_resources("Group", &context)
            .await
            .expect("Failed to list groups");

        assert!(!groups.is_empty(), "Should have at least one group");
        assert!(
            groups.iter().any(|g| g.get_id() == Some(group_id.as_str())),
            "Should contain our created group"
        );

        // Test Delete Group
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
