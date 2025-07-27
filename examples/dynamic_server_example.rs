//! Complete example demonstrating the new dynamic SCIM server approach.
//!
//! This example shows how to:
//! 1. Create a dynamic SCIM server with no hard-coded resource types
//! 2. Register multiple resource types (User, Group, CustomResource)
//! 3. Use schema-driven operations
//! 4. Map between SCIM and implementation schemas
//! 5. Perform CRUD operations generically
//!
//! Run with: cargo run --example dynamic_server_example

use async_trait::async_trait;
use scim_server::resource::ListQuery;
use scim_server::{
    AttributeDefinition, AttributeType, DynamicResource, DynamicResourceProvider,
    DynamicScimServer, Mutability, RequestContext, Schema, SchemaResourceBuilder, ScimOperation,
    Uniqueness, create_group_resource_handler, create_user_resource_handler,
};
use serde_json::{Value, json};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tokio;

// Example provider implementation using in-memory storage
#[derive(Debug)]
struct InMemoryDynamicProvider {
    // storage: resource_type -> id -> resource
    storage: Arc<Mutex<HashMap<String, HashMap<String, DynamicResource>>>>,
    next_id: Arc<Mutex<u64>>,
}

impl InMemoryDynamicProvider {
    fn new() -> Self {
        Self {
            storage: Arc::new(Mutex::new(HashMap::new())),
            next_id: Arc::new(Mutex::new(1)),
        }
    }

    fn generate_id(&self) -> String {
        let mut id = self.next_id.lock().unwrap();
        let current = *id;
        *id += 1;
        current.to_string()
    }
}

#[derive(Debug, thiserror::Error)]
#[error("In-memory provider error: {message}")]
struct ProviderError {
    message: String,
}

impl ProviderError {
    fn new(message: &str) -> Self {
        Self {
            message: message.to_string(),
        }
    }
}

#[async_trait]
impl DynamicResourceProvider for InMemoryDynamicProvider {
    type Error = ProviderError;

    async fn create_resource(
        &self,
        resource_type: &str,
        mut resource: DynamicResource,
        context: &RequestContext,
    ) -> Result<DynamicResource, Self::Error> {
        println!(
            "Creating {} with request ID: {}",
            resource_type, context.request_id
        );

        // Generate ID
        let id = self.generate_id();
        resource
            .set_attribute_dynamic("id", Value::String(id.clone()))
            .map_err(|e| ProviderError::new(&e.to_string()))?;

        // Add metadata using custom method
        if let Ok(meta) = resource.call_custom_method("add_metadata") {
            resource
                .set_attribute_dynamic("meta", meta)
                .map_err(|e| ProviderError::new(&e.to_string()))?;
        }

        // Store the resource
        let mut storage = self.storage.lock().unwrap();
        storage
            .entry(resource_type.to_string())
            .or_insert_with(HashMap::new)
            .insert(id, resource.clone());

        println!(
            "Successfully created {} with ID: {}",
            resource_type,
            resource
                .get_attribute_dynamic("id")
                .and_then(|v| v.as_str().map(|s| s.to_string()))
                .unwrap_or_else(|| "unknown".to_string())
        );

        Ok(resource)
    }

    async fn get_resource(
        &self,
        resource_type: &str,
        id: &str,
        context: &RequestContext,
    ) -> Result<Option<DynamicResource>, Self::Error> {
        println!(
            "Getting {} with ID {} (request: {})",
            resource_type, id, context.request_id
        );

        let storage = self.storage.lock().unwrap();
        let result = storage
            .get(resource_type)
            .and_then(|type_storage| type_storage.get(id))
            .cloned();

        if result.is_some() {
            println!("Found {} with ID: {}", resource_type, id);
        } else {
            println!("{} with ID {} not found", resource_type, id);
        }

        Ok(result)
    }

    async fn update_resource(
        &self,
        resource_type: &str,
        id: &str,
        mut resource: DynamicResource,
        context: &RequestContext,
    ) -> Result<DynamicResource, Self::Error> {
        println!(
            "Updating {} with ID {} (request: {})",
            resource_type, id, context.request_id
        );

        // Ensure ID is set
        resource
            .set_attribute_dynamic("id", Value::String(id.to_string()))
            .map_err(|e| ProviderError::new(&e.to_string()))?;

        // Update metadata
        if let Ok(meta) = resource.call_custom_method("add_metadata") {
            resource
                .set_attribute_dynamic("meta", meta)
                .map_err(|e| ProviderError::new(&e.to_string()))?;
        }

        let mut storage = self.storage.lock().unwrap();
        if let Some(type_storage) = storage.get_mut(resource_type) {
            if type_storage.contains_key(id) {
                type_storage.insert(id.to_string(), resource.clone());
                println!("Successfully updated {} with ID: {}", resource_type, id);
                return Ok(resource);
            }
        }

        Err(ProviderError::new(&format!(
            "{} with ID {} not found",
            resource_type, id
        )))
    }

    async fn delete_resource(
        &self,
        resource_type: &str,
        id: &str,
        context: &RequestContext,
    ) -> Result<(), Self::Error> {
        println!(
            "Deleting {} with ID {} (request: {})",
            resource_type, id, context.request_id
        );

        let mut storage = self.storage.lock().unwrap();
        if let Some(type_storage) = storage.get_mut(resource_type) {
            if type_storage.remove(id).is_some() {
                println!("Successfully deleted {} with ID: {}", resource_type, id);
                return Ok(());
            }
        }

        Err(ProviderError::new(&format!(
            "{} with ID {} not found",
            resource_type, id
        )))
    }

    async fn list_resources(
        &self,
        resource_type: &str,
        _query: &ListQuery,
        context: &RequestContext,
    ) -> Result<Vec<DynamicResource>, Self::Error> {
        println!(
            "Listing {}s (request: {})",
            resource_type, context.request_id
        );

        let storage = self.storage.lock().unwrap();
        let resources: Vec<DynamicResource> = storage
            .get(resource_type)
            .map(|type_storage| type_storage.values().cloned().collect())
            .unwrap_or_default();

        println!("Found {} {}s", resources.len(), resource_type);
        Ok(resources)
    }

    async fn find_resource_by_attribute(
        &self,
        resource_type: &str,
        attribute: &str,
        value: &str,
        context: &RequestContext,
    ) -> Result<Option<DynamicResource>, Self::Error> {
        println!(
            "Searching for {} by {}={} (request: {})",
            resource_type, attribute, value, context.request_id
        );

        let storage = self.storage.lock().unwrap();
        if let Some(type_storage) = storage.get(resource_type) {
            for resource in type_storage.values() {
                if let Some(attr_value) = resource.get_attribute_dynamic(attribute) {
                    if attr_value.as_str() == Some(value) {
                        println!("Found {} by {}={}", resource_type, attribute, value);
                        return Ok(Some(resource.clone()));
                    }
                }
            }
        }

        println!("No {} found with {}={}", resource_type, attribute, value);
        Ok(None)
    }

    async fn resource_exists(
        &self,
        resource_type: &str,
        id: &str,
        _context: &RequestContext,
    ) -> Result<bool, Self::Error> {
        let storage = self.storage.lock().unwrap();
        let exists = storage
            .get(resource_type)
            .map(|type_storage| type_storage.contains_key(id))
            .unwrap_or(false);

        Ok(exists)
    }
}

// Create a custom resource schema for demonstration
fn create_custom_resource_schema() -> Schema {
    Schema {
        id: "urn:example:scim:schemas:CustomResource".to_string(),
        name: "CustomResource".to_string(),
        description: "Custom resource for demonstration".to_string(),
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
                name: "customName".to_string(),
                data_type: AttributeType::String,
                required: true,
                mutability: Mutability::ReadWrite,
                ..Default::default()
            },
            AttributeDefinition {
                name: "customValue".to_string(),
                data_type: AttributeType::String,
                required: false,
                mutability: Mutability::ReadWrite,
                ..Default::default()
            },
            AttributeDefinition {
                name: "isEnabled".to_string(),
                data_type: AttributeType::Boolean,
                required: false,
                mutability: Mutability::ReadWrite,
                ..Default::default()
            },
        ],
    }
}

// Create handler for custom resource
fn create_custom_resource_handler(schema: Schema) -> scim_server::resource::ResourceHandler {
    SchemaResourceBuilder::new(schema)
        .with_getter("customName", |data| {
            data.get("customName")?
                .as_str()
                .map(|s| Value::String(s.to_string()))
        })
        .with_custom_method("get_custom_name", |resource| {
            Ok(resource
                .get_attribute_dynamic("customName")
                .unwrap_or(Value::Null))
        })
        .with_custom_method("add_metadata", |resource| {
            let base_url = "https://example.com/scim";
            let now = chrono::Utc::now().to_rfc3339();

            let meta = json!({
                "resourceType": resource.resource_type,
                "created": now,
                "lastModified": now,
                "location": format!("{}/{}s/{}",
                    base_url,
                    resource.resource_type,
                    resource.get_attribute_dynamic("id")
                        .and_then(|v| v.as_str().map(|s| s.to_string()))
                        .unwrap_or_else(|| "".to_string())
                )
            });

            Ok(meta)
        })
        .with_database_mapping("custom_resources", {
            let mut mappings = HashMap::new();
            mappings.insert("customName".to_string(), "name".to_string());
            mappings.insert("customValue".to_string(), "value".to_string());
            mappings.insert("isEnabled".to_string(), "enabled".to_string());
            mappings.insert("id".to_string(), "resource_id".to_string());
            mappings
        })
        .build()
}

// Load schemas from JSON files (in a real app, you'd load from actual files)
fn load_user_schema() -> Schema {
    // In practice, you'd load this from User.json
    // For this example, we'll create it programmatically
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
            AttributeDefinition {
                name: "emails".to_string(),
                data_type: AttributeType::Complex,
                multi_valued: true,
                required: false,
                mutability: Mutability::ReadWrite,
                ..Default::default()
            },
            AttributeDefinition {
                name: "active".to_string(),
                data_type: AttributeType::Boolean,
                required: false,
                mutability: Mutability::ReadWrite,
                ..Default::default()
            },
        ],
    }
}

fn load_group_schema() -> Schema {
    Schema {
        id: "urn:ietf:params:scim:schemas:core:2.0:Group".to_string(),
        name: "Group".to_string(),
        description: "Group".to_string(),
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
                name: "displayName".to_string(),
                data_type: AttributeType::String,
                required: true,
                mutability: Mutability::ReadWrite,
                ..Default::default()
            },
            AttributeDefinition {
                name: "members".to_string(),
                data_type: AttributeType::Complex,
                multi_valued: true,
                required: false,
                mutability: Mutability::ReadWrite,
                ..Default::default()
            },
        ],
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 Starting Dynamic SCIM Server Example");
    println!("========================================");

    // 1. Create the provider
    let provider = InMemoryDynamicProvider::new();

    // 2. Create the dynamic server
    let mut server = DynamicScimServer::new(provider)?;

    println!("✅ Created dynamic SCIM server");

    // 3. Load schemas and register resource types

    // Register User resource type
    let user_schema = load_user_schema();
    let user_handler = create_user_resource_handler(user_schema);
    server.register_resource_type(
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
    )?;
    println!("✅ Registered User resource type");

    // Register Group resource type
    let group_schema = load_group_schema();
    let group_handler = create_group_resource_handler(group_schema);
    server.register_resource_type(
        "Group",
        group_handler,
        vec![
            ScimOperation::Create,
            ScimOperation::Read,
            ScimOperation::Update,
            ScimOperation::Delete,
            ScimOperation::List,
        ],
    )?;
    println!("✅ Registered Group resource type");

    // Register custom resource type
    let custom_schema = create_custom_resource_schema();
    let custom_handler = create_custom_resource_handler(custom_schema);
    server.register_resource_type(
        "CustomResource",
        custom_handler,
        vec![ScimOperation::Create, ScimOperation::Read], // Limited operations
    )?;
    println!("✅ Registered CustomResource resource type");

    // 4. Display supported resource types
    let supported_types = server.get_supported_resource_types();
    println!("\n📋 Supported resource types: {:?}", supported_types);

    // 5. Create test data and perform operations
    let context = RequestContext::new("demo-request-123".to_string());

    println!("\n🧪 Testing CRUD Operations");
    println!("==========================");

    // Create a User
    println!("\n👤 Creating User...");
    let user_data = json!({
        "userName": "jdoe",
        "displayName": "John Doe",
        "emails": [
            {
                "value": "john.doe@example.com",
                "type": "work",
                "primary": true
            },
            {
                "value": "john@personal.com",
                "type": "home",
                "primary": false
            }
        ],
        "active": true
    });

    let created_user = server.create_resource("User", user_data, &context).await?;
    let user_id = created_user
        .get_attribute_dynamic("id")
        .and_then(|v| v.as_str().map(|s| s.to_string()))
        .unwrap();

    // Test User custom methods
    println!(
        "  📧 Primary email: {}",
        created_user.call_custom_method("get_primary_email")?
    );
    println!(
        "  👤 Username: {}",
        created_user.call_custom_method("get_username")?
    );
    println!(
        "  ✅ Is active: {}",
        created_user.call_custom_method("is_active")?
    );

    // Test database mapping
    println!(
        "  💾 Database format: {}",
        created_user.to_implementation_schema(0)?
    );

    // Create a Group
    println!("\n👥 Creating Group...");
    let group_data = json!({
        "displayName": "Developers",
        "members": [
            {
                "value": user_id,
                "type": "User"
            }
        ]
    });

    let created_group = server
        .create_resource("Group", group_data, &context)
        .await?;
    let _group_id = created_group
        .get_attribute_dynamic("id")
        .and_then(|v| v.as_str().map(|s| s.to_string()))
        .unwrap();

    println!(
        "  📝 Group name: {}",
        created_group.call_custom_method("get_display_name")?
    );

    // Create a Custom Resource
    println!("\n🔧 Creating CustomResource...");
    let custom_data = json!({
        "customName": "MyCustomThing",
        "customValue": "SomeValue",
        "isEnabled": true
    });

    let created_custom = server
        .create_resource("CustomResource", custom_data, &context)
        .await?;
    println!(
        "  🏷️  Custom name: {}",
        created_custom.call_custom_method("get_custom_name")?
    );

    // Test reading resources
    println!("\n📖 Reading Resources...");
    let retrieved_user = server.get_resource("User", &user_id, &context).await?;
    match retrieved_user {
        Some(user) => println!(
            "  👤 Retrieved user: {}",
            user.call_custom_method("get_username")?
        ),
        None => println!("  ❌ User not found"),
    }

    // Test searching
    println!("\n🔍 Searching Resources...");
    let found_user = server
        .find_resource_by_attribute("User", "userName", "jdoe", &context)
        .await?;

    match found_user {
        Some(user) => println!(
            "  🎯 Found user by username: {}",
            user.call_custom_method("get_display_name")?
        ),
        None => println!("  ❌ User not found by username"),
    }

    // Test listing
    println!("\n📜 Listing Resources...");
    let query = ListQuery::new();
    let users = server.list_resources("User", &query, &context).await?;
    println!("  👥 Total users: {}", users.len());

    let groups = server.list_resources("Group", &query, &context).await?;
    println!("  👥 Total groups: {}", groups.len());

    // Test updating
    println!("\n✏️  Updating User...");
    let updated_user_data = json!({
        "userName": "jdoe",
        "displayName": "John F. Doe", // Changed
        "emails": [
            {
                "value": "john.doe@example.com",
                "type": "work",
                "primary": true
            }
        ],
        "active": true
    });

    let updated_user = server
        .update_resource("User", &user_id, updated_user_data, &context)
        .await?;
    println!(
        "  📝 Updated display name: {}",
        updated_user.call_custom_method("get_display_name")?
    );

    // Test operation restrictions
    println!("\n🚫 Testing Operation Restrictions...");
    // Test deleting custom resource (should fail - operation not supported)
    let custom_id = created_custom
        .get_attribute_dynamic("id")
        .and_then(|v| v.as_str().map(|s| s.to_string()))
        .unwrap();

    // Try to delete custom resource (should fail - operation not supported)
    let delete_result = server
        .delete_resource("CustomResource", &custom_id, &context)
        .await;
    match delete_result {
        Ok(_) => println!("  ❌ Unexpected: Delete succeeded"),
        Err(e) => println!("  ✅ Expected error: {}", e),
    }

    // Test unsupported resource type
    let unsupported_result = server
        .create_resource("UnsupportedType", json!({}), &context)
        .await;
    match unsupported_result {
        Ok(_) => println!("  ❌ Unexpected: Unsupported type succeeded"),
        Err(e) => println!("  ✅ Expected error: {}", e),
    }

    // Show schemas
    println!("\n📊 Available Schemas:");
    for schema in server.get_all_schemas() {
        println!(
            "  🏗️  {}: {} ({} attributes)",
            schema.name,
            schema.id,
            schema.attributes.len()
        );
    }

    println!("\n🎉 Dynamic SCIM Server Example Complete!");
    println!("=========================================");
    println!("✅ Successfully demonstrated:");
    println!("   - Runtime resource type registration");
    println!("   - Schema-driven operations");
    println!("   - Generic CRUD operations");
    println!("   - Custom method invocation");
    println!("   - Database schema mapping");
    println!("   - Operation restrictions");
    println!("   - Error handling");

    Ok(())
}
