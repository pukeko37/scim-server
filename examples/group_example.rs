//! Group Example - Demonstrating Group schema functionality in SCIM server
//!
//! This example shows how to:
//! 1. Set up a SCIM server with Group support
//! 2. Create, read, update, and delete Group resources
//! 3. Manage Group memberships
//! 4. Validate Group schemas
//!
//! Run with: cargo run --example group_example

use scim_server::{
    Resource,
    resource::{ListQuery, RequestContext, ResourceProvider, ScimOperation},
    resource_handlers::create_group_resource_handler,
    schema::SchemaRegistry,
    scim_server::ScimServer,
};
use serde_json::{Value, json};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

/// Simple in-memory resource provider for demonstration
#[derive(Debug)]
struct InMemoryProvider {
    resources: Arc<Mutex<HashMap<String, HashMap<String, Resource>>>>,
}

impl InMemoryProvider {
    fn new() -> Self {
        Self {
            resources: Arc::new(Mutex::new(HashMap::new())),
        }
    }
}

#[derive(Debug)]
struct ProviderError(String);

impl std::fmt::Display for ProviderError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "Provider error: {}", self.0)
    }
}

impl std::error::Error for ProviderError {}

impl ResourceProvider for InMemoryProvider {
    type Error = ProviderError;

    async fn create_resource(
        &self,
        resource_type: &str,
        data: Value,
        _context: &RequestContext,
    ) -> Result<Resource, Self::Error> {
        let id = format!("{}-{}", resource_type.to_lowercase(), uuid::Uuid::new_v4());
        let mut resource_data = data;
        resource_data["id"] = json!(id);

        let resource = Resource::new(resource_type.to_string(), resource_data);

        let mut resources = self.resources.lock().unwrap();
        let type_resources = resources
            .entry(resource_type.to_string())
            .or_insert_with(HashMap::new);
        type_resources.insert(id.clone(), resource.clone());

        Ok(resource)
    }

    async fn get_resource(
        &self,
        resource_type: &str,
        id: &str,
        _context: &RequestContext,
    ) -> Result<Option<Resource>, Self::Error> {
        let resources = self.resources.lock().unwrap();
        if let Some(type_resources) = resources.get(resource_type) {
            Ok(type_resources.get(id).cloned())
        } else {
            Ok(None)
        }
    }

    async fn update_resource(
        &self,
        resource_type: &str,
        id: &str,
        data: Value,
        _context: &RequestContext,
    ) -> Result<Resource, Self::Error> {
        let mut resource_data = data;
        resource_data["id"] = json!(id);

        let resource = Resource::new(resource_type.to_string(), resource_data);

        let mut resources = self.resources.lock().unwrap();
        let type_resources = resources
            .entry(resource_type.to_string())
            .or_insert_with(HashMap::new);
        type_resources.insert(id.to_string(), resource.clone());

        Ok(resource)
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
        _query: Option<&ListQuery>,
        _context: &RequestContext,
    ) -> Result<Vec<Resource>, Self::Error> {
        let resources = self.resources.lock().unwrap();
        if let Some(type_resources) = resources.get(resource_type) {
            Ok(type_resources.values().cloned().collect())
        } else {
            Ok(Vec::new())
        }
    }

    async fn find_resource_by_attribute(
        &self,
        resource_type: &str,
        attribute: &str,
        value: &Value,
        _context: &RequestContext,
    ) -> Result<Option<Resource>, Self::Error> {
        let resources = self.resources.lock().unwrap();
        if let Some(type_resources) = resources.get(resource_type) {
            let matching = type_resources
                .values()
                .find(|resource| {
                    if let Some(attr_value) = resource.get_attribute(attribute) {
                        attr_value == value
                    } else {
                        false
                    }
                })
                .cloned();
            Ok(matching)
        } else {
            Ok(None)
        }
    }

    async fn resource_exists(
        &self,
        resource_type: &str,
        id: &str,
        _context: &RequestContext,
    ) -> Result<bool, Self::Error> {
        let resources = self.resources.lock().unwrap();
        if let Some(type_resources) = resources.get(resource_type) {
            Ok(type_resources.contains_key(id))
        } else {
            Ok(false)
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 SCIM Server Group Example");
    println!("=============================\n");

    // 1. Create provider and server
    let provider = InMemoryProvider::new();
    let mut server = ScimServer::new(provider)?;

    // 2. Load Group schema and register Group resource type
    let registry = SchemaRegistry::new()?;
    let group_schema = registry.get_group_schema().clone();
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
            ScimOperation::Search,
        ],
    )?;

    println!("✅ Group resource type registered successfully");

    // 3. Create test context
    let context = RequestContext::new("group-example".to_string());

    // 4. Demonstrate Group creation
    println!("\n📝 Creating Groups...");

    let engineering_group = json!({
        "displayName": "Engineering Team",
        "members": [
            {
                "value": "user-alice",
                "$ref": "https://example.com/v2/Users/user-alice",
                "type": "User"
            },
            {
                "value": "user-bob",
                "$ref": "https://example.com/v2/Users/user-bob",
                "type": "User"
            }
        ]
    });

    let created_engineering = server
        .create_resource("Group", engineering_group, &context)
        .await?;

    println!(
        "✅ Created Engineering Team: {}",
        created_engineering
            .get_attribute("id")
            .unwrap()
            .as_str()
            .unwrap()
    );

    let marketing_group = json!({
        "displayName": "Marketing Team",
        "members": [
            {
                "value": "user-charlie",
                "$ref": "https://example.com/v2/Users/user-charlie",
                "type": "User"
            }
        ]
    });

    let created_marketing = server
        .create_resource("Group", marketing_group, &context)
        .await?;

    println!(
        "✅ Created Marketing Team: {}",
        created_marketing
            .get_attribute("id")
            .unwrap()
            .as_str()
            .unwrap()
    );

    // 5. Demonstrate Group retrieval
    println!("\n🔍 Retrieving Groups...");

    let engineering_id = created_engineering
        .get_attribute("id")
        .unwrap()
        .as_str()
        .unwrap();
    let retrieved_group = server
        .get_resource("Group", engineering_id, &context)
        .await?;

    if let Some(group) = retrieved_group {
        println!(
            "✅ Retrieved group: {} with {} members",
            group
                .get_attribute("displayName")
                .unwrap()
                .as_str()
                .unwrap(),
            group
                .get_attribute("members")
                .unwrap()
                .as_array()
                .unwrap()
                .len()
        );
    }

    // 6. Demonstrate Group listing
    println!("\n📋 Listing all Groups...");

    let all_groups = server.list_resources("Group", &context).await?;
    println!("✅ Found {} groups:", all_groups.len());

    for group in &all_groups {
        let display_name = group
            .get_attribute("displayName")
            .unwrap()
            .as_str()
            .unwrap();
        let member_count = group
            .get_attribute("members")
            .map(|m| m.as_array().map(|a| a.len()).unwrap_or(0))
            .unwrap_or(0);
        println!("   • {} ({} members)", display_name, member_count);
    }

    // 7. Demonstrate Group member addition (update)
    println!("\n✏️  Updating Group membership...");

    let updated_engineering = json!({
        "displayName": "Engineering Team",
        "members": [
            {
                "value": "user-alice",
                "$ref": "https://example.com/v2/Users/user-alice",
                "type": "User"
            },
            {
                "value": "user-bob",
                "$ref": "https://example.com/v2/Users/user-bob",
                "type": "User"
            },
            {
                "value": "user-david",
                "$ref": "https://example.com/v2/Users/user-david",
                "type": "User"
            }
        ]
    });

    let updated_group = server
        .update_resource("Group", engineering_id, updated_engineering, &context)
        .await?;

    println!(
        "✅ Updated Engineering Team: now has {} members",
        updated_group
            .get_attribute("members")
            .unwrap()
            .as_array()
            .unwrap()
            .len()
    );

    // 8. Demonstrate nested group membership
    println!("\n🎯 Creating nested Groups...");

    let management_group = json!({
        "displayName": "Management",
        "members": [
            {
                "value": engineering_id,
                "$ref": format!("https://example.com/v2/Groups/{}", engineering_id),
                "type": "Group"
            },
            {
                "value": created_marketing.get_attribute("id").unwrap().as_str().unwrap(),
                "$ref": format!("https://example.com/v2/Groups/{}",
                    created_marketing.get_attribute("id").unwrap().as_str().unwrap()),
                "type": "Group"
            }
        ]
    });

    let created_management = server
        .create_resource("Group", management_group, &context)
        .await?;

    println!(
        "✅ Created Management group with nested groups: {}",
        created_management
            .get_attribute("id")
            .unwrap()
            .as_str()
            .unwrap()
    );

    // 9. Demonstrate Group search
    println!("\n🔎 Searching for Groups...");

    let search_result = server
        .find_resource_by_attribute("Group", "displayName", &json!("Engineering Team"), &context)
        .await?;

    match search_result {
        Some(group) => println!(
            "✅ Found group matching 'Engineering Team': {}",
            group.get_attribute("id").unwrap().as_str().unwrap()
        ),
        None => println!("❌ No groups found matching 'Engineering Team'"),
    }

    // 10. Demonstrate schema validation
    println!("\n🛡️  Testing Group validation...");

    // Test invalid group (this should fail validation)
    let invalid_group = json!({
        "displayName": "Invalid Group",
        "members": [
            {
                "value": "user-eve",
                "$ref": "https://example.com/v2/Users/user-eve",
                "type": "InvalidType"  // Invalid member type
            }
        ]
    });

    match server
        .create_resource("Group", invalid_group, &context)
        .await
    {
        Ok(_) => println!("⚠️  Invalid group was unexpectedly accepted"),
        Err(e) => println!("✅ Validation correctly rejected invalid group: {}", e),
    }

    // 11. Demonstrate Group deletion
    println!("\n🗑️  Cleaning up Groups...");

    let management_id = created_management
        .get_attribute("id")
        .unwrap()
        .as_str()
        .unwrap();
    server
        .delete_resource("Group", management_id, &context)
        .await?;
    println!("✅ Deleted Management group");

    server
        .delete_resource("Group", engineering_id, &context)
        .await?;
    println!("✅ Deleted Engineering Team");

    let marketing_id = created_marketing
        .get_attribute("id")
        .unwrap()
        .as_str()
        .unwrap();
    server
        .delete_resource("Group", marketing_id, &context)
        .await?;
    println!("✅ Deleted Marketing Team");

    // 12. Final verification
    let final_groups = server.list_resources("Group", &context).await?;
    println!(
        "\n📊 Final verification: {} groups remaining",
        final_groups.len()
    );

    println!("\n🎉 Group example completed successfully!");
    println!("\nThis example demonstrated:");
    println!("   ✅ Group schema loading and validation");
    println!("   ✅ Group resource registration");
    println!("   ✅ Creating Groups with members");
    println!("   ✅ Retrieving individual Groups");
    println!("   ✅ Listing all Groups");
    println!("   ✅ Updating Group membership");
    println!("   ✅ Nested Group membership (Groups containing Groups)");
    println!("   ✅ Searching Groups by attribute");
    println!("   ✅ Schema validation for invalid Groups");
    println!("   ✅ Deleting Groups");

    Ok(())
}
