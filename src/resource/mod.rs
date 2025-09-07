//! SCIM resource model with type-safe value objects and clean architecture.
//!
//! This module provides the core resource abstractions for SCIM operations,
//! emphasizing type safety, immutable value objects, and clean separation
//! of concerns between protocol logic and storage.
//!
//! # Architecture
//!
//! The resource module follows a hybrid approach:
//! - **Core attributes** as validated value objects (ResourceId, UserName, etc.)
//! - **Extension attributes** as flexible JSON for extensibility
//! - **Schema handlers** for resource type definitions
//! - **Version control** for optimistic concurrency
//!
//! # Key Components
//!
//! * [`Resource`] - Core SCIM resource with type-safe attributes
//! * [`ResourceHandler`] - Schema-based resource type definitions
//! * [`RequestContext`] - Request tracking with optional tenant context
//! * [`VersionedResource`] - Resources with automatic version control
//! * [`value_objects`] - Validated domain primitives (ResourceId, UserName, etc.)
//! * [`mapper`] - Schema mapping infrastructure (for future storage-level mapping)

pub mod builder;
pub mod context;
pub mod handlers;
pub mod mapper;
pub mod versioned;

pub mod resource;
pub mod serialization;
pub mod tenant;

pub mod value_objects;
pub mod version;

// Re-export all public types to maintain API compatibility
pub use context::{ListQuery, RequestContext};
pub use resource::Resource;
pub use tenant::{IsolationLevel, TenantContext, TenantPermissions};
// Re-export ScimOperation from multi_tenant module for backward compatibility
pub use crate::multi_tenant::ScimOperation;
pub use handlers::{ResourceHandler, SchemaResourceBuilder};
pub use mapper::{DatabaseMapper, SchemaMapper};
pub use value_objects::{
    Address, EmailAddress, ExternalId, Meta, Name, PhoneNumber, ResourceId, SchemaUri, UserName,
};
pub use version::{
    ConditionalResult, HttpVersion, RawVersion, ScimVersion, VersionConflict, VersionError,
};
pub use versioned::VersionedResource;

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
        let resource = Resource::from_json("User".to_string(), data).unwrap();

        assert_eq!(resource.resource_type, "User");
        assert_eq!(resource.get_username(), Some("testuser"));
    }

    #[test]
    fn test_resource_id_extraction() {
        let data = json!({
            "id": "12345",
            "userName": "testuser"
        });
        let resource = Resource::from_json("User".to_string(), data).unwrap();

        assert_eq!(resource.get_id(), Some("12345"));
    }

    #[test]
    fn test_resource_schemas() {
        let data = json!({
            "userName": "testuser"
        });
        let resource = Resource::from_json("User".to_string(), data).unwrap();

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
        let resource = Resource::from_json("User".to_string(), data).unwrap();

        let emails = resource.get_emails().expect("Should have emails");
        assert_eq!(emails.len(), 1);
        let email = emails.get(0).expect("Should have first email");
        assert_eq!(email.value(), "test@example.com");
    }

    #[test]
    fn test_request_context_creation() {
        let context = RequestContext::new("test-request".to_string());
        assert!(!context.request_id.is_empty());

        let context_with_id = RequestContext::new("test-123".to_string());
        assert_eq!(context_with_id.request_id, "test-123");
    }

    #[test]
    fn test_resource_active_status() {
        let active_data = json!({
            "userName": "testuser",
            "active": true
        });
        let active_resource = Resource::from_json("User".to_string(), active_data).unwrap();
        assert!(active_resource.is_active());

        let inactive_data = json!({
            "userName": "testuser",
            "active": false
        });
        let inactive_resource = Resource::from_json("User".to_string(), inactive_data).unwrap();
        assert!(!inactive_resource.is_active());

        let no_active_data = json!({
            "userName": "testuser"
        });
        let default_resource = Resource::from_json("User".to_string(), no_active_data).unwrap();
        assert!(default_resource.is_active()); // Default to true
    }

    #[test]
    fn test_meta_extraction_from_json() {
        use chrono::{TimeZone, Utc};

        // Test resource with valid meta
        let data_with_meta = json!({
            "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
            "id": "12345",
            "userName": "testuser",
            "meta": {
                "resourceType": "User",
                "created": "2023-01-01T12:00:00Z",
                "lastModified": "2023-01-02T12:00:00Z",
                "location": "https://example.com/Users/12345",
                "version": "12345-1672574400000"
            }
        });

        let resource = Resource::from_json("User".to_string(), data_with_meta).unwrap();
        let meta = resource.get_meta().unwrap();

        assert_eq!(meta.resource_type(), "User");
        assert_eq!(
            meta.created(),
            Utc.with_ymd_and_hms(2023, 1, 1, 12, 0, 0).unwrap()
        );
        assert_eq!(
            meta.last_modified(),
            Utc.with_ymd_and_hms(2023, 1, 2, 12, 0, 0).unwrap()
        );
        assert_eq!(meta.location(), Some("https://example.com/Users/12345"));
        assert_eq!(meta.version(), Some("12345-1672574400000"));
    }

    #[test]
    fn test_meta_extraction_minimal() {
        // Test resource with minimal meta
        let data_minimal_meta = json!({
            "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
            "userName": "testuser",
            "meta": {
                "resourceType": "User",
                "created": "2023-01-01T12:00:00Z",
                "lastModified": "2023-01-01T12:00:00Z"
            }
        });

        let resource = Resource::from_json("User".to_string(), data_minimal_meta).unwrap();
        let meta = resource.get_meta().unwrap();

        assert_eq!(meta.resource_type(), "User");
        assert_eq!(meta.location(), None);
        assert_eq!(meta.version(), None);
    }

    #[test]
    fn test_meta_extraction_invalid_datetime_returns_error() {
        // Test resource with invalid datetime format returns validation error
        let data_invalid_meta = json!({
            "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
            "userName": "testuser",
            "meta": {
                "resourceType": "User",
                "created": "invalid-date",
                "lastModified": "2023-01-01T12:00:00Z"
            }
        });

        let result = Resource::from_json("User".to_string(), data_invalid_meta);
        assert!(result.is_err());
        match result.unwrap_err() {
            crate::error::ValidationError::InvalidCreatedDateTime => {
                // Expected error
            }
            other => panic!("Expected InvalidCreatedDateTime, got {:?}", other),
        }
    }

    #[test]
    fn test_meta_extraction_incomplete_is_ignored() {
        // Test resource with incomplete meta is ignored (for backward compatibility)
        let data_incomplete_meta = json!({
            "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
            "userName": "testuser",
            "meta": {
                "resourceType": "User"
                // Missing created and lastModified
            }
        });

        let resource = Resource::from_json("User".to_string(), data_incomplete_meta).unwrap();
        assert!(resource.get_meta().is_none());
    }

    #[test]
    fn test_meta_extraction_missing() {
        // Test resource without meta
        let data_no_meta = json!({
            "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
            "userName": "testuser"
        });

        let resource = Resource::from_json("User".to_string(), data_no_meta).unwrap();
        assert!(resource.get_meta().is_none());
    }

    #[test]
    fn test_set_meta() {
        use crate::resource::value_objects::Meta;
        use chrono::Utc;

        let mut resource = Resource::from_json(
            "User".to_string(),
            json!({
                "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
                "userName": "testuser"
            }),
        )
        .unwrap();

        let now = Utc::now();
        let meta = Meta::new_simple("User".to_string(), now, now).unwrap();
        resource.set_meta(meta.clone());

        assert!(resource.get_meta().is_some());
        assert_eq!(resource.get_meta().unwrap().resource_type(), "User");

        // Test that meta is also in JSON representation
        let json_output = resource.to_json().unwrap();
        assert!(json_output.get("meta").is_some());
    }

    #[test]
    fn test_create_meta() {
        let mut resource = Resource::from_json(
            "User".to_string(),
            json!({
                "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
                "id": "12345",
                "userName": "testuser"
            }),
        )
        .unwrap();

        resource.create_meta("https://example.com").unwrap();

        let meta = resource.get_meta().unwrap();
        assert_eq!(meta.resource_type(), "User");
        assert_eq!(meta.created(), meta.last_modified());
        assert_eq!(meta.location(), Some("https://example.com/Users/12345"));
    }

    #[test]
    fn test_update_meta() {
        use crate::resource::value_objects::Meta;
        use chrono::Utc;
        use std::thread;
        use std::time::Duration;

        let mut resource = Resource::from_json(
            "User".to_string(),
            json!({
                "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
                "userName": "testuser"
            }),
        )
        .unwrap();

        let now = Utc::now();
        let meta = Meta::new_simple("User".to_string(), now, now).unwrap();
        resource.set_meta(meta);

        let original_modified = resource.get_meta().unwrap().last_modified();

        // Wait a bit to ensure timestamp difference
        thread::sleep(Duration::from_millis(10));

        resource.update_meta();

        let updated_modified = resource.get_meta().unwrap().last_modified();
        assert!(updated_modified > original_modified);
        assert_eq!(resource.get_meta().unwrap().created(), now);
    }

    #[test]
    fn test_meta_serialization_in_to_json() {
        use crate::resource::value_objects::Meta;
        use chrono::{TimeZone, Utc};

        let mut resource = Resource::from_json(
            "User".to_string(),
            json!({
                "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
                "userName": "testuser"
            }),
        )
        .unwrap();

        let created = Utc.with_ymd_and_hms(2023, 1, 1, 12, 0, 0).unwrap();
        let modified = Utc.with_ymd_and_hms(2023, 1, 2, 12, 0, 0).unwrap();
        let meta = Meta::new(
            "User".to_string(),
            created,
            modified,
            Some("https://example.com/Users/123".to_string()),
            Some("123-456".to_string()),
        )
        .unwrap();

        resource.set_meta(meta);

        let json_output = resource.to_json().unwrap();
        let meta_json = json_output.get("meta").unwrap();

        assert_eq!(
            meta_json.get("resourceType").unwrap().as_str().unwrap(),
            "User"
        );
        assert!(
            meta_json
                .get("created")
                .unwrap()
                .as_str()
                .unwrap()
                .starts_with("2023-01-01T12:00:00")
        );
        assert!(
            meta_json
                .get("lastModified")
                .unwrap()
                .as_str()
                .unwrap()
                .starts_with("2023-01-02T12:00:00")
        );
        assert_eq!(
            meta_json.get("location").unwrap().as_str().unwrap(),
            "https://example.com/Users/123"
        );
        assert_eq!(
            meta_json.get("version").unwrap().as_str().unwrap(),
            "123-456"
        );
    }

    #[test]
    fn test_resource_with_name_extraction() {
        let data = json!({
            "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
            "userName": "testuser",
            "name": {
                "formatted": "John Doe",
                "familyName": "Doe",
                "givenName": "John"
            }
        });

        let resource = Resource::from_json("User".to_string(), data).unwrap();

        assert!(resource.get_name().is_some());
        let name = resource.get_name().unwrap();
        assert_eq!(name.formatted(), Some("John Doe"));
        assert_eq!(name.family_name(), Some("Doe"));
        assert_eq!(name.given_name(), Some("John"));
    }

    #[test]
    fn test_resource_with_addresses_extraction() {
        let data = json!({
            "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
            "userName": "testuser",
            "addresses": [
                {
                    "type": "work",
                    "streetAddress": "123 Main St",
                    "locality": "Anytown",
                    "region": "CA",
                    "postalCode": "12345",
                    "country": "US",
                    "primary": true
                }
            ]
        });

        let resource = Resource::from_json("User".to_string(), data).unwrap();

        let addresses = resource.get_addresses().expect("Should have addresses");
        assert_eq!(addresses.len(), 1);
        let address = addresses.get(0).expect("Should have first address");
        assert_eq!(address.address_type(), Some("work"));
        assert_eq!(address.street_address(), Some("123 Main St"));
        assert_eq!(address.locality(), Some("Anytown"));
        assert_eq!(address.region(), Some("CA"));
        assert_eq!(address.postal_code(), Some("12345"));
        assert_eq!(address.country(), Some("US"));
        assert_eq!(address.is_primary(), true);
    }

    #[test]
    fn test_resource_with_phone_numbers_extraction() {
        let data = json!({
            "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
            "userName": "testuser",
            "phoneNumbers": [
                {
                    "value": "tel:+1-555-555-5555",
                    "type": "work",
                    "primary": true
                }
            ]
        });

        let resource = Resource::from_json("User".to_string(), data).unwrap();

        let phones = resource
            .get_phone_numbers()
            .expect("Should have phone numbers");
        assert_eq!(phones.len(), 1);
        let phone = phones.get(0).expect("Should have first phone");
        assert_eq!(phone.value(), "tel:+1-555-555-5555");
        assert_eq!(phone.phone_type(), Some("work"));
        assert_eq!(phone.is_primary(), true);
    }

    #[test]
    fn test_resource_builder_basic() {
        use crate::resource::builder::ResourceBuilder;
        use crate::resource::value_objects::{ResourceId, UserName};

        let resource = ResourceBuilder::new("User".to_string())
            .with_id(ResourceId::new("123".to_string()).unwrap())
            .with_username(UserName::new("jdoe".to_string()).unwrap())
            .with_attribute("displayName", json!("John Doe"))
            .build()
            .unwrap();

        assert_eq!(resource.resource_type, "User");
        assert_eq!(resource.get_id(), Some("123"));
        assert_eq!(resource.get_username(), Some("jdoe"));
        assert_eq!(
            resource.get_attribute("displayName"),
            Some(&json!("John Doe"))
        );
        assert_eq!(resource.schemas.len(), 1);
        assert_eq!(
            resource.schemas[0].as_str(),
            "urn:ietf:params:scim:schemas:core:2.0:User"
        );
    }

    #[test]
    fn test_resource_builder_with_complex_attributes() {
        use crate::resource::value_objects::{Address, Name, PhoneNumber};

        let name = Name::new_simple("John".to_string(), "Doe".to_string()).unwrap();
        let address = Address::new_work(
            "123 Main St".to_string(),
            "Anytown".to_string(),
            "CA".to_string(),
            "12345".to_string(),
            "US".to_string(),
        )
        .unwrap();
        let phone = PhoneNumber::new_work("tel:+1-555-555-5555".to_string()).unwrap();

        use crate::resource::builder::ResourceBuilder;

        let resource = ResourceBuilder::new("User".to_string())
            .with_name(name)
            .add_address(address)
            .add_phone_number(phone)
            .build()
            .unwrap();

        assert!(resource.get_name().is_some());
        assert_eq!(resource.get_addresses().unwrap().len(), 1);
        assert_eq!(resource.get_phone_numbers().unwrap().len(), 1);

        // Test serialization includes all fields
        let json_output = resource.to_json().unwrap();
        assert!(json_output.get("name").is_some());
        assert!(json_output.get("addresses").is_some());
        assert!(json_output.get("phoneNumbers").is_some());
    }

    #[test]
    fn test_resource_builder_with_meta() {
        use crate::resource::value_objects::ResourceId;

        use crate::resource::builder::ResourceBuilder;

        let resource = ResourceBuilder::new("User".to_string())
            .with_id(ResourceId::new("123".to_string()).unwrap())
            .build_with_meta("https://example.com")
            .unwrap();

        assert!(resource.get_meta().is_some());
        let meta = resource.get_meta().unwrap();
        assert_eq!(meta.resource_type(), "User");
        assert_eq!(meta.location(), Some("https://example.com/Users/123"));
    }

    #[test]
    fn test_resource_builder_validation() {
        // Test that builder validates required fields
        use crate::resource::builder::ResourceBuilder;

        let builder = ResourceBuilder::new("User".to_string());
        // Remove default schema to test validation
        let builder_no_schema = builder.with_schemas(vec![]);
        let result = builder_no_schema.build();
        assert!(result.is_err());
    }

    #[test]
    fn test_resource_setter_methods() {
        use crate::resource::value_objects::{Address, Name, PhoneNumber};

        let mut resource = Resource::from_json(
            "User".to_string(),
            json!({
                "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
                "userName": "testuser"
            }),
        )
        .unwrap();

        // Test name setter
        let name = Name::new_simple("Jane".to_string(), "Smith".to_string()).unwrap();
        resource.set_name(name);
        assert!(resource.get_name().is_some());
        assert_eq!(resource.get_name().unwrap().given_name(), Some("Jane"));

        // Test address setter
        let address = Address::new(
            None,
            Some("456 Oak Ave".to_string()),
            Some("Hometown".to_string()),
            Some("NY".to_string()),
            Some("67890".to_string()),
            Some("US".to_string()),
            Some("home".to_string()),
            Some(false),
        )
        .unwrap();
        resource.add_address(address).unwrap();
        let addresses = resource.get_addresses().expect("Should have addresses");
        assert_eq!(addresses.len(), 1);
        let address = addresses.get(0).expect("Should have first address");
        assert_eq!(address.address_type(), Some("home"));

        // Test phone number setter
        let phone = PhoneNumber::new_mobile("tel:+1-555-123-4567".to_string()).unwrap();
        resource.add_phone_number(phone).unwrap();
        let phones = resource
            .get_phone_numbers()
            .expect("Should have phone numbers");
        assert_eq!(phones.len(), 1);
        let phone = phones.get(0).expect("Should have first phone");
        assert_eq!(phone.phone_type(), Some("mobile"));
    }

    #[test]
    fn test_resource_json_round_trip_with_complex_attributes() {
        let data = json!({
            "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
            "id": "123",
            "userName": "jdoe",
            "name": {
                "formatted": "John Doe",
                "familyName": "Doe",
                "givenName": "John"
            },
            "addresses": [
                {
                    "type": "work",
                    "streetAddress": "123 Main St",
                    "locality": "Anytown",
                    "region": "CA",
                    "postalCode": "12345",
                    "country": "US",
                    "primary": true
                }
            ],
            "phoneNumbers": [
                {
                    "value": "tel:+1-555-555-5555",
                    "type": "work",
                    "primary": true
                }
            ]
        });

        let resource = Resource::from_json("User".to_string(), data).unwrap();
        let json_output = resource.to_json().unwrap();

        // Verify all complex attributes are preserved
        assert!(json_output.get("name").is_some());
        assert!(json_output.get("addresses").is_some());
        assert!(json_output.get("phoneNumbers").is_some());

        // Verify the structured data is correct
        let name_json = json_output.get("name").unwrap();
        assert_eq!(
            name_json.get("formatted").unwrap().as_str().unwrap(),
            "John Doe"
        );

        let addresses_json = json_output.get("addresses").unwrap().as_array().unwrap();
        assert_eq!(addresses_json.len(), 1);
        assert_eq!(
            addresses_json[0].get("type").unwrap().as_str().unwrap(),
            "work"
        );

        let phones_json = json_output.get("phoneNumbers").unwrap().as_array().unwrap();
        assert_eq!(phones_json.len(), 1);
        assert_eq!(
            phones_json[0].get("value").unwrap().as_str().unwrap(),
            "tel:+1-555-555-5555"
        );
    }

    #[test]
    fn test_resource_invalid_complex_attributes() {
        // Test invalid name structure
        let invalid_name_data = json!({
            "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
            "userName": "testuser",
            "name": "should be object not string"
        });
        let result = Resource::from_json("User".to_string(), invalid_name_data);
        assert!(result.is_err());

        // Test invalid addresses structure
        let invalid_addresses_data = json!({
            "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
            "userName": "testuser",
            "addresses": "should be array not string"
        });
        let result = Resource::from_json("User".to_string(), invalid_addresses_data);
        assert!(result.is_err());

        // Test invalid phone numbers structure
        let invalid_phones_data = json!({
            "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
            "userName": "testuser",
            "phoneNumbers": "should be array not string"
        });
        let result = Resource::from_json("User".to_string(), invalid_phones_data);
        assert!(result.is_err());
    }
}
