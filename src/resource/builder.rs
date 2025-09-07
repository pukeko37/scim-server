//! Resource builder for type-safe SCIM resource construction.
//!
//! This module provides a fluent API for constructing SCIM resources with
//! compile-time validation and type safety for all value objects.

use crate::error::{ValidationError, ValidationResult};
use crate::resource::resource::Resource;
use crate::resource::value_objects::{
    Address, EmailAddress, ExternalId, GroupMembers, Meta, MultiValuedAddresses, MultiValuedEmails,
    MultiValuedPhoneNumbers, Name, PhoneNumber, ResourceId, SchemaUri, UserName,
};
use serde_json::{Map, Value};

/// Enhanced Resource Builder for type-safe resource construction.
///
/// This builder provides a fluent API for constructing SCIM resources with
/// compile-time validation and type safety for all value objects.
///
/// # Example
/// ```rust
/// use scim_server::Resource;
/// use serde_json::json;
///
/// fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let user_data = json!({
///         "id": "123",
///         "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
///         "userName": "jdoe",
///         "name": {
///             "givenName": "John",
///             "familyName": "Doe"
///         },
///         "displayName": "John Doe"
///     });
///     let resource = Resource::from_json("User".to_string(), user_data)?;
///
///     Ok(())
/// }
/// ```
#[derive(Debug, Clone)]
pub struct ResourceBuilder {
    resource_type: String,
    id: Option<ResourceId>,
    schemas: Vec<SchemaUri>,
    external_id: Option<ExternalId>,
    user_name: Option<UserName>,
    meta: Option<Meta>,
    name: Option<Name>,
    addresses: Option<MultiValuedAddresses>,
    phone_numbers: Option<MultiValuedPhoneNumbers>,
    emails: Option<MultiValuedEmails>,
    members: Option<GroupMembers>,
    attributes: Map<String, Value>,
}

impl ResourceBuilder {
    /// Create a new ResourceBuilder with the specified resource type.
    pub fn new(resource_type: String) -> Self {
        let mut schemas = Vec::new();

        // Add default schema based on resource type
        if resource_type == "User" {
            if let Ok(schema) =
                SchemaUri::new("urn:ietf:params:scim:schemas:core:2.0:User".to_string())
            {
                schemas.push(schema);
            }
        } else if resource_type == "Group" {
            if let Ok(schema) =
                SchemaUri::new("urn:ietf:params:scim:schemas:core:2.0:Group".to_string())
            {
                schemas.push(schema);
            }
        }

        Self {
            resource_type,
            id: None,
            schemas,
            external_id: None,
            user_name: None,
            meta: None,
            name: None,
            addresses: None,
            phone_numbers: None,
            emails: None,
            members: None,
            attributes: Map::new(),
        }
    }

    /// Set the resource ID.
    pub fn with_id(mut self, id: ResourceId) -> Self {
        self.id = Some(id);
        self
    }

    /// Set the external ID.
    pub fn with_external_id(mut self, external_id: ExternalId) -> Self {
        self.external_id = Some(external_id);
        self
    }

    /// Set the username (for User resources).
    pub fn with_username(mut self, username: UserName) -> Self {
        self.user_name = Some(username);
        self
    }

    /// Set the meta attributes.
    pub fn with_meta(mut self, meta: Meta) -> Self {
        self.meta = Some(meta);
        self
    }

    /// Set the name (for User resources).
    pub fn with_name(mut self, name: Name) -> Self {
        self.name = Some(name);
        self
    }

    /// Set addresses for the resource.
    pub fn with_addresses(mut self, addresses: MultiValuedAddresses) -> Self {
        self.addresses = Some(addresses);
        self
    }

    /// Set phone numbers for the resource.
    pub fn with_phone_numbers(mut self, phone_numbers: MultiValuedPhoneNumbers) -> Self {
        self.phone_numbers = Some(phone_numbers);
        self
    }

    /// Set emails for the resource.
    pub fn with_emails(mut self, emails: MultiValuedEmails) -> Self {
        self.emails = Some(emails);
        self
    }

    /// Set group members for the resource.
    pub fn with_members(mut self, members: GroupMembers) -> Self {
        self.members = Some(members);
        self
    }

    /// Add a single address to the resource.
    pub fn add_address(mut self, address: Address) -> Self {
        match self.addresses {
            Some(existing) => {
                let new_addresses = existing.with_value(address);
                self.addresses = Some(new_addresses);
            }
            None => {
                let new_addresses = MultiValuedAddresses::single(address);
                self.addresses = Some(new_addresses);
            }
        }
        self
    }

    /// Add a single phone number to the resource.
    pub fn add_phone_number(mut self, phone_number: PhoneNumber) -> Self {
        match self.phone_numbers {
            Some(existing) => {
                let new_phones = existing.with_value(phone_number);
                self.phone_numbers = Some(new_phones);
            }
            None => {
                let new_phones = MultiValuedPhoneNumbers::single(phone_number);
                self.phone_numbers = Some(new_phones);
            }
        }
        self
    }

    /// Add a single email to the resource.
    pub fn add_email(mut self, email: EmailAddress) -> Self {
        match self.emails {
            Some(existing) => {
                let new_emails = existing.with_value(email);
                self.emails = Some(new_emails);
            }
            None => {
                let new_emails = MultiValuedEmails::single(email);
                self.emails = Some(new_emails);
            }
        }
        self
    }

    /// Add a schema URI.
    pub fn add_schema(mut self, schema: SchemaUri) -> Self {
        self.schemas.push(schema);
        self
    }

    /// Set all schema URIs.
    pub fn with_schemas(mut self, schemas: Vec<SchemaUri>) -> Self {
        self.schemas = schemas;
        self
    }

    /// Add an extended attribute.
    pub fn with_attribute<S: Into<String>>(mut self, name: S, value: Value) -> Self {
        self.attributes.insert(name.into(), value);
        self
    }

    /// Add multiple extended attributes from a map.
    pub fn with_attributes(mut self, attributes: Map<String, Value>) -> Self {
        for (key, value) in attributes {
            self.attributes.insert(key, value);
        }
        self
    }

    /// Build the Resource.
    pub fn build(self) -> ValidationResult<Resource> {
        // Validate that required fields are present
        if self.schemas.is_empty() {
            return Err(ValidationError::custom("At least one schema is required"));
        }

        Ok(Resource {
            resource_type: self.resource_type,
            id: self.id,
            schemas: self.schemas,
            external_id: self.external_id,
            user_name: self.user_name,
            meta: self.meta,
            name: self.name,
            addresses: self.addresses,
            phone_numbers: self.phone_numbers,
            emails: self.emails,
            members: self.members,
            attributes: self.attributes,
        })
    }

    /// Build the Resource and create meta attributes for a new resource.
    pub fn build_with_meta(mut self, base_url: &str) -> ValidationResult<Resource> {
        // Create meta if not already set
        if self.meta.is_none() {
            let meta = Meta::new_for_creation(self.resource_type.clone())?;
            let meta_with_location = if let Some(ref id) = self.id {
                let location = Meta::generate_location(base_url, &self.resource_type, id.as_str());
                meta.with_location(location)?
            } else {
                meta
            };
            self.meta = Some(meta_with_location);
        }

        self.build()
    }
}
