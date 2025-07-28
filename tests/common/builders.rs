//! Test data builders for creating valid and invalid SCIM resources.
//!
//! This module provides fluent builders for creating test data that can be
//! systematically modified to test specific validation errors.

use super::ValidationErrorCode;
use serde_json::{Value, json};

/// Builder for User resources with fluent API for creating test data
#[derive(Debug, Clone)]
pub struct UserBuilder {
    data: Value,
    expected_errors: Vec<ValidationErrorCode>,
}

impl UserBuilder {
    /// Create a new UserBuilder with minimal valid User data
    pub fn new() -> Self {
        Self {
            data: json!({
                "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
                "id": "2819c223-7f76-453a-919d-413861904646",
                "userName": "bjensen@example.com",
                "meta": {
                    "resourceType": "User",
                    "created": "2010-01-23T04:56:22Z",
                    "lastModified": "2011-05-13T04:42:34Z",
                    "version": "W/\"3694e05e9dff590\"",
                    "location": "https://example.com/v2/Users/2819c223-7f76-453a-919d-413861904646"
                }
            }),
            expected_errors: Vec::new(),
        }
    }

    /// Create a UserBuilder with RFC 7643 Section 8.2 full example
    pub fn new_full() -> Self {
        Self {
            data: json!({
                "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
                "id": "2819c223-7f76-453a-919d-413861904646",
                "externalId": "701984",
                "userName": "bjensen@example.com",
                "name": {
                    "formatted": "Ms. Barbara J Jensen, III",
                    "familyName": "Jensen",
                    "givenName": "Barbara",
                    "middleName": "Jane",
                    "honorificPrefix": "Ms.",
                    "honorificSuffix": "III"
                },
                "displayName": "Babs Jensen",
                "emails": [
                    {
                        "value": "bjensen@example.com",
                        "type": "work",
                        "primary": true
                    },
                    {
                        "value": "babs@jensen.org",
                        "type": "home"
                    }
                ],
                "phoneNumbers": [
                    {
                        "value": "555-555-5555",
                        "type": "work"
                    },
                    {
                        "value": "555-555-4444",
                        "type": "mobile"
                    }
                ],
                "active": true,
                "meta": {
                    "resourceType": "User",
                    "created": "2010-01-23T04:56:22Z",
                    "lastModified": "2011-05-13T04:42:34Z",
                    "version": "W/\"a330bc54f0671c9\"",
                    "location": "https://example.com/v2/Users/2819c223-7f76-453a-919d-413861904646"
                }
            }),
            expected_errors: Vec::new(),
        }
    }

    // Schema structure modifications (Errors 1-8)

    /// Remove the schemas attribute - Error #1: Missing required schemas attribute
    pub fn without_schemas(mut self) -> Self {
        self.data.as_object_mut().unwrap().remove("schemas");
        self.expected_errors
            .push(ValidationErrorCode::MissingSchemas);
        self
    }

    /// Set empty schemas array - Error #2: Empty schemas array
    pub fn with_empty_schemas(mut self) -> Self {
        self.data["schemas"] = json!([]);
        self.expected_errors.push(ValidationErrorCode::EmptySchemas);
        self
    }

    /// Set invalid schema URI - Error #3: Invalid schema URI format
    pub fn with_invalid_schema_uri(mut self) -> Self {
        self.data["schemas"] = json!(["not-a-valid-uri"]);
        self.expected_errors
            .push(ValidationErrorCode::InvalidSchemaUri);
        self
    }

    /// Set unknown schema URI - Error #4: Unknown/unregistered schema URI
    pub fn with_unknown_schema_uri(mut self) -> Self {
        self.data["schemas"] = json!(["urn:example:unknown:schema"]);
        self.expected_errors
            .push(ValidationErrorCode::UnknownSchemaUri);
        self
    }

    /// Set duplicate schema URIs - Error #5: Duplicate schema URIs
    pub fn with_duplicate_schema_uris(mut self) -> Self {
        self.data["schemas"] = json!([
            "urn:ietf:params:scim:schemas:core:2.0:User",
            "urn:ietf:params:scim:schemas:core:2.0:User"
        ]);
        self.expected_errors
            .push(ValidationErrorCode::DuplicateSchemaUri);
        self
    }

    // Common attribute modifications (Errors 9-21)

    /// Remove the id attribute - Error #9: Missing required id attribute
    pub fn without_id(mut self) -> Self {
        self.data.as_object_mut().unwrap().remove("id");
        self.expected_errors.push(ValidationErrorCode::MissingId);
        self
    }

    /// Set empty id value - Error #10: Empty or null id value
    pub fn with_empty_id(mut self) -> Self {
        self.data["id"] = json!("");
        self.expected_errors.push(ValidationErrorCode::EmptyId);
        self
    }

    /// Set reserved bulkId value - Error #11: Invalid id format (reserved keyword)
    pub fn with_reserved_id(mut self) -> Self {
        self.data["id"] = json!("bulkId");
        self.expected_errors
            .push(ValidationErrorCode::InvalidIdFormat);
        self
    }

    /// Remove userName attribute - Error #22: Missing required attribute
    pub fn without_username(mut self) -> Self {
        self.data.as_object_mut().unwrap().remove("userName");
        self.expected_errors
            .push(ValidationErrorCode::MissingRequiredAttribute);
        self
    }

    /// Set invalid data type for userName - Error #23: Invalid data type
    pub fn with_invalid_username_type(mut self) -> Self {
        self.data["userName"] = json!(123); // Number instead of string
        self.expected_errors
            .push(ValidationErrorCode::InvalidDataType);
        self
    }

    /// Set invalid meta structure - Error #14: Invalid meta structure
    pub fn with_invalid_meta_structure(mut self) -> Self {
        self.data["meta"] = json!("not-an-object");
        self.expected_errors
            .push(ValidationErrorCode::InvalidMetaStructure);
        self
    }

    /// Remove meta.resourceType - Error #15: Missing required meta.resourceType
    pub fn without_meta_resource_type(mut self) -> Self {
        if let Some(meta) = self.data["meta"].as_object_mut() {
            meta.remove("resourceType");
        }
        self.expected_errors
            .push(ValidationErrorCode::MissingResourceType);
        self
    }

    /// Set invalid meta.resourceType - Error #16: Invalid meta.resourceType value
    pub fn with_invalid_meta_resource_type(mut self) -> Self {
        self.data["meta"]["resourceType"] = json!("InvalidType");
        self.expected_errors
            .push(ValidationErrorCode::InvalidResourceType);
        self
    }

    /// Set invalid created datetime - Error #18: Invalid meta.created datetime format
    pub fn with_invalid_created_datetime(mut self) -> Self {
        self.data["meta"]["created"] = json!("not-a-datetime");
        self.expected_errors
            .push(ValidationErrorCode::InvalidCreatedDateTime);
        self
    }

    // Attribute type modifications (Errors 22-32)

    /// Set invalid boolean value for active - Error #25: Invalid boolean value
    pub fn with_invalid_boolean_active(mut self) -> Self {
        self.data["active"] = json!("not-boolean"); // Should be true/false
        self.expected_errors
            .push(ValidationErrorCode::InvalidBooleanValue);
        self
    }

    // Multi-valued attribute modifications (Errors 33-38)

    /// Set single value for multi-valued emails - Error #33: Single value for multi-valued
    pub fn with_single_value_emails(mut self) -> Self {
        self.data["emails"] = json!({
            "value": "test@example.com",
            "type": "work"
        });
        self.expected_errors
            .push(ValidationErrorCode::SingleValueForMultiValued);
        self
    }

    /// Set array for single-valued userName - Error #34: Array for single-valued
    pub fn with_array_username(mut self) -> Self {
        self.data["userName"] = json!(["user1", "user2"]);
        self.expected_errors
            .push(ValidationErrorCode::ArrayForSingleValued);
        self
    }

    /// Set multiple primary=true in emails - Error #35: Multiple primary=true values
    pub fn with_multiple_primary_emails(mut self) -> Self {
        self.data["emails"] = json!([
            {
                "value": "primary1@example.com",
                "type": "work",
                "primary": true
            },
            {
                "value": "primary2@example.com",
                "type": "home",
                "primary": true
            }
        ]);
        self.expected_errors
            .push(ValidationErrorCode::MultiplePrimaryValues);
        self
    }

    /// Set invalid canonical value - Error #38: Invalid canonical value
    pub fn with_invalid_email_type(mut self) -> Self {
        self.data["emails"] = json!([
            {
                "value": "test@example.com",
                "type": "invalid-type"
            }
        ]);
        self.expected_errors
            .push(ValidationErrorCode::InvalidCanonicalValue);
        self
    }

    // Complex attribute modifications (Errors 39-43)

    /// Set invalid sub-attribute type in name - Error #40: Invalid sub-attribute type
    pub fn with_invalid_name_sub_attribute_type(mut self) -> Self {
        self.data["name"] = json!({
            "familyName": "Jensen",
            "givenName": 123 // Should be string, not number
        });
        self.expected_errors
            .push(ValidationErrorCode::InvalidSubAttributeType);
        self
    }

    /// Add unknown sub-attribute to name - Error #41: Unknown sub-attribute
    pub fn with_unknown_name_sub_attribute(mut self) -> Self {
        self.data["name"] = json!({
            "familyName": "Jensen",
            "givenName": "Barbara",
            "unknownAttribute": "unknown value"
        });
        self.expected_errors
            .push(ValidationErrorCode::UnknownSubAttribute);
        self
    }

    // Builder methods for valid variations

    /// Set username
    pub fn with_username(mut self, username: &str) -> Self {
        self.data["userName"] = json!(username);
        self
    }

    /// Set display name
    pub fn with_display_name(mut self, display_name: &str) -> Self {
        self.data["displayName"] = json!(display_name);
        self
    }

    /// Add email address
    pub fn with_email(mut self, email: &str, email_type: &str, primary: bool) -> Self {
        let emails = self.data["emails"].as_array().cloned().unwrap_or_default();
        let mut new_emails = emails;
        new_emails.push(json!({
            "value": email,
            "type": email_type,
            "primary": primary
        }));
        self.data["emails"] = json!(new_emails);
        self
    }

    /// Set active status
    pub fn with_active(mut self, active: bool) -> Self {
        self.data["active"] = json!(active);
        self
    }

    /// Build the final JSON value
    pub fn build(self) -> Value {
        self.data
    }

    /// Get expected validation errors for this builder configuration
    pub fn expected_errors(&self) -> &[ValidationErrorCode] {
        &self.expected_errors
    }
}

impl Default for UserBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Builder for Group resources
#[derive(Debug, Clone)]
pub struct GroupBuilder {
    data: Value,
    expected_errors: Vec<ValidationErrorCode>,
}

impl GroupBuilder {
    /// Create a new GroupBuilder with minimal valid Group data
    pub fn new() -> Self {
        Self {
            data: json!({
                "schemas": ["urn:ietf:params:scim:schemas:core:2.0:Group"],
                "id": "e9e30dba-f08f-4109-8486-d5c6a331660a",
                "displayName": "Tour Guides",
                "meta": {
                    "resourceType": "Group",
                    "created": "2010-01-23T04:56:22Z",
                    "lastModified": "2011-05-13T04:42:34Z",
                    "version": "W/\"3694e05e9dff592\"",
                    "location": "https://example.com/v2/Groups/e9e30dba-f08f-4109-8486-d5c6a331660a"
                }
            }),
            expected_errors: Vec::new(),
        }
    }

    /// Remove displayName - Group-specific required attribute
    pub fn without_display_name(mut self) -> Self {
        self.data.as_object_mut().unwrap().remove("displayName");
        self.expected_errors
            .push(ValidationErrorCode::MissingRequiredAttribute);
        self
    }

    /// Set empty displayName
    pub fn with_empty_display_name(mut self) -> Self {
        self.data["displayName"] = json!("");
        self.expected_errors
            .push(ValidationErrorCode::MissingRequiredAttribute);
        self
    }

    /// Set displayName
    pub fn with_display_name(mut self, display_name: &str) -> Self {
        self.data["displayName"] = json!(display_name);
        self
    }

    /// Add member to group
    pub fn with_member(
        mut self,
        user_id: &str,
        user_ref: &str,
        display_name: Option<&str>,
    ) -> Self {
        let members = self.data["members"].as_array().cloned().unwrap_or_default();
        let mut new_members = members;
        let mut member = json!({
            "value": user_id,
            "$ref": user_ref,
            "type": "User"
        });
        if let Some(name) = display_name {
            member["display"] = json!(name);
        }
        new_members.push(member);
        self.data["members"] = json!(new_members);
        self
    }

    /// Build the final JSON value
    pub fn build(self) -> Value {
        self.data
    }

    /// Get expected validation errors for this builder configuration
    pub fn expected_errors(&self) -> &[ValidationErrorCode] {
        &self.expected_errors
    }
}

impl Default for GroupBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Builder for Schema resources
#[derive(Debug, Clone)]
pub struct SchemaBuilder {
    data: Value,
}

impl SchemaBuilder {
    /// Create a new SchemaBuilder with basic User schema structure
    pub fn new_user_schema() -> Self {
        Self {
            data: json!({
                "id": "urn:ietf:params:scim:schemas:core:2.0:User",
                "name": "User",
                "description": "User Account",
                "attributes": [
                    {
                        "name": "userName",
                        "type": "string",
                        "multiValued": false,
                        "required": true,
                        "caseExact": false,
                        "mutability": "readWrite",
                        "returned": "default",
                        "uniqueness": "server"
                    }
                ]
            }),
        }
    }

    /// Build the final JSON value
    pub fn build(self) -> Value {
        self.data
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_user_builder_new() {
        let user = UserBuilder::new().build();
        assert_eq!(
            user["schemas"][0],
            "urn:ietf:params:scim:schemas:core:2.0:User"
        );
        assert_eq!(user["userName"], "bjensen@example.com");
        assert!(!user["id"].as_str().unwrap().is_empty());
    }

    #[test]
    fn test_user_builder_without_schemas() {
        let builder = UserBuilder::new().without_schemas();
        let user = builder.build();
        assert!(!user.as_object().unwrap().contains_key("schemas"));

        let builder2 = UserBuilder::new().without_schemas();
        let expected_errors = builder2.expected_errors();
        assert_eq!(expected_errors, &[ValidationErrorCode::MissingSchemas]);
    }

    #[test]
    fn test_user_builder_with_empty_schemas() {
        let builder = UserBuilder::new().with_empty_schemas();
        let user = builder.build();
        assert_eq!(user["schemas"], json!([]));

        let builder2 = UserBuilder::new().with_empty_schemas();
        assert_eq!(
            builder2.expected_errors(),
            &[ValidationErrorCode::EmptySchemas]
        );
    }

    #[test]
    fn test_user_builder_multiple_errors() {
        let builder = UserBuilder::new().without_schemas().without_username();
        let expected_errors = builder.expected_errors();
        assert_eq!(expected_errors.len(), 2);
        assert!(expected_errors.contains(&ValidationErrorCode::MissingSchemas));
        assert!(expected_errors.contains(&ValidationErrorCode::MissingRequiredAttribute));
    }

    #[test]
    fn test_group_builder_new() {
        let group = GroupBuilder::new().build();
        assert_eq!(
            group["schemas"][0],
            "urn:ietf:params:scim:schemas:core:2.0:Group"
        );
        assert_eq!(group["displayName"], "Tour Guides");
    }

    #[test]
    fn test_group_builder_without_display_name() {
        let builder = GroupBuilder::new().without_display_name();
        let group = builder.build();
        assert!(!group.as_object().unwrap().contains_key("displayName"));

        let builder2 = GroupBuilder::new().without_display_name();
        assert_eq!(
            builder2.expected_errors(),
            &[ValidationErrorCode::MissingRequiredAttribute]
        );
    }
}
