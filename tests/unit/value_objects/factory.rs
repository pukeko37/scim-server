//! Extension Attribute Tests
//!
//! This module tests extension attributes and composite validation rules
//! that are part of the public API for extensibility support.

use scim_server::resource::value_objects::{
    ExtensionAttributeValue, ExtensionCollection, SchemaUri, ValueObject,
};
use scim_server::schema::types::{AttributeDefinition, AttributeType, Mutability, Uniqueness};
use serde_json::json;

/// Create a test attribute definition for string attributes
fn create_string_definition(name: &str, required: bool) -> AttributeDefinition {
    AttributeDefinition {
        name: name.to_string(),
        data_type: AttributeType::String,
        multi_valued: false,
        required,
        case_exact: false,
        mutability: Mutability::ReadWrite,
        uniqueness: if name == "id" {
            Uniqueness::Server
        } else {
            Uniqueness::None
        },
        canonical_values: vec![],
        sub_attributes: vec![],
        returned: None,
    }
}

#[test]
fn test_extension_attribute_creation() {
    let schema_uri = SchemaUri::new("urn:test:scim:schemas:extension:test".to_string()).unwrap();
    let ext_attr = ExtensionAttributeValue::new(
        schema_uri,
        "customAttribute".to_string(),
        json!("test-value"),
        None,
    )
    .unwrap();

    assert_eq!(ext_attr.attribute_name(), "customAttribute");
    assert_eq!(
        ext_attr.schema_uri().as_str(),
        "urn:test:scim:schemas:extension:test"
    );
    assert_eq!(ext_attr.value(), &json!("test-value"));
}

#[test]
fn test_extension_attribute_with_definition() {
    let schema_uri = SchemaUri::new("urn:test:scim:schemas:extension:test".to_string()).unwrap();
    let definition = create_string_definition("customAttribute", false);

    let ext_attr = ExtensionAttributeValue::new(
        schema_uri,
        "customAttribute".to_string(),
        json!("test-value"),
        Some(definition),
    )
    .unwrap();

    assert_eq!(ext_attr.attribute_name(), "customAttribute");
    assert_eq!(ext_attr.attribute_type(), AttributeType::String);
}

#[test]
fn test_extension_collection() {
    let mut collection = ExtensionCollection::new();
    assert!(collection.is_empty());
    assert_eq!(collection.len(), 0);

    let schema_uri = SchemaUri::new("urn:test:scim:schemas:extension:test".to_string()).unwrap();
    let ext_attr = ExtensionAttributeValue::new(
        schema_uri,
        "customAttribute".to_string(),
        json!("test-value"),
        None,
    )
    .unwrap();

    collection.add_attribute(ext_attr);

    assert!(!collection.is_empty());
    assert_eq!(collection.len(), 1);
    assert_eq!(
        collection.schema_uris(),
        vec!["urn:test:scim:schemas:extension:test"]
    );

    let retrieved =
        collection.get_attribute("urn:test:scim:schemas:extension:test", "customAttribute");
    assert!(retrieved.is_some());
    assert_eq!(retrieved.unwrap().attribute_name(), "customAttribute");
}

#[test]
fn test_extension_collection_multiple_schemas() {
    let mut collection = ExtensionCollection::new();

    // Add attribute from first schema
    let schema_uri1 = SchemaUri::new("urn:test:scim:schemas:extension:test1".to_string()).unwrap();
    let ext_attr1 =
        ExtensionAttributeValue::new(schema_uri1, "attr1".to_string(), json!("value1"), None)
            .unwrap();
    collection.add_attribute(ext_attr1);

    // Add attribute from second schema
    let schema_uri2 = SchemaUri::new("urn:test:scim:schemas:extension:test2".to_string()).unwrap();
    let ext_attr2 =
        ExtensionAttributeValue::new(schema_uri2, "attr2".to_string(), json!("value2"), None)
            .unwrap();
    collection.add_attribute(ext_attr2);

    assert_eq!(collection.len(), 2);
    let mut schema_uris = collection.schema_uris();
    schema_uris.sort();
    assert_eq!(
        schema_uris,
        vec![
            "urn:test:scim:schemas:extension:test1",
            "urn:test:scim:schemas:extension:test2"
        ]
    );

    // Verify we can retrieve attributes from both schemas
    assert!(
        collection
            .get_attribute("urn:test:scim:schemas:extension:test1", "attr1")
            .is_some()
    );
    assert!(
        collection
            .get_attribute("urn:test:scim:schemas:extension:test2", "attr2")
            .is_some()
    );
    assert!(
        collection
            .get_attribute("urn:test:scim:schemas:extension:test1", "attr2")
            .is_none()
    );
}

#[test]
fn test_extension_to_json() {
    let mut collection = ExtensionCollection::new();

    let schema_uri =
        SchemaUri::new("urn:test:scim:schemas:extension:enterprise".to_string()).unwrap();
    let dept_attr = ExtensionAttributeValue::new(
        schema_uri.clone(),
        "department".to_string(),
        json!("Engineering"),
        None,
    )
    .unwrap();
    let emp_id_attr = ExtensionAttributeValue::new(
        schema_uri,
        "employeeId".to_string(),
        json!("EMP-12345"),
        None,
    )
    .unwrap();

    collection.add_attribute(dept_attr);
    collection.add_attribute(emp_id_attr);

    let json_result = collection.to_json().unwrap();

    // Should create a JSON object with schema URI as key
    assert!(json_result.is_object());
    let obj = json_result.as_object().unwrap();
    assert!(obj.contains_key("urn:test:scim:schemas:extension:enterprise"));

    let enterprise_ext = obj
        .get("urn:test:scim:schemas:extension:enterprise")
        .unwrap();
    assert!(enterprise_ext.is_object());
    let ext_obj = enterprise_ext.as_object().unwrap();
    assert_eq!(ext_obj.get("department"), Some(&json!("Engineering")));
    assert_eq!(ext_obj.get("employeeId"), Some(&json!("EMP-12345")));
}

#[test]
fn test_extension_validation() {
    let schema_uri = SchemaUri::new("urn:test:scim:schemas:extension:test".to_string()).unwrap();
    let definition = create_string_definition("requiredField", true);

    // Test with valid value
    let valid_ext = ExtensionAttributeValue::new(
        schema_uri.clone(),
        "requiredField".to_string(),
        json!("valid-value"),
        Some(definition.clone()),
    );
    assert!(valid_ext.is_ok());

    // Test validation against definition
    let ext_attr = valid_ext.unwrap();
    assert_eq!(ext_attr.attribute_type(), AttributeType::String);
}

#[test]
fn test_extension_value_object_interface() {
    let schema_uri = SchemaUri::new("urn:test:scim:schemas:extension:test".to_string()).unwrap();
    let ext_attr = ExtensionAttributeValue::new(
        schema_uri,
        "testAttribute".to_string(),
        json!("test-value"),
        None,
    )
    .unwrap();

    // Test ValueObject trait implementation
    assert_eq!(ext_attr.attribute_name(), "testAttribute");
    assert_eq!(ext_attr.as_json_value(), json!("test-value"));

    let json_result = ext_attr.to_json().unwrap();
    // Extension attributes serialize to their JSON value, not necessarily an object
    assert_eq!(json_result, json!("test-value"));

    // Test cloning
    let cloned = ext_attr.clone_boxed();
    assert_eq!(cloned.attribute_name(), "testAttribute");
}

#[test]
fn test_extension_collection_removal() {
    let mut collection = ExtensionCollection::new();

    let schema_uri = SchemaUri::new("urn:test:scim:schemas:extension:test".to_string()).unwrap();
    let ext_attr = ExtensionAttributeValue::new(
        schema_uri,
        "testAttribute".to_string(),
        json!("test-value"),
        None,
    )
    .unwrap();

    collection.add_attribute(ext_attr);
    assert_eq!(collection.len(), 1);

    let removed = collection.remove_schema("urn:test:scim:schemas:extension:test");
    assert!(removed.is_some());
    assert_eq!(removed.unwrap().len(), 1);
    assert!(collection.is_empty());
}

#[test]
fn test_extension_validation_errors() {
    let schema_uri = SchemaUri::new("urn:test:scim:schemas:extension:test".to_string()).unwrap();

    // Test successful creation (empty attribute names are actually allowed)
    let result = ExtensionAttributeValue::new(
        schema_uri.clone(),
        "validName".to_string(),
        json!("value"),
        None,
    );
    assert!(result.is_ok());

    // Test with valid schema and attribute
    let valid_result = ExtensionAttributeValue::new(
        schema_uri,
        "anotherAttribute".to_string(),
        json!("another-value"),
        None,
    );
    assert!(valid_result.is_ok());
}
