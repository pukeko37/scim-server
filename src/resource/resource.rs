//! Core SCIM resource representation and validation.
//!
//! This module contains the main Resource struct and its associated methods
//! for creating, validating, and manipulating SCIM resources with type safety
//! for core attributes while maintaining JSON flexibility for extensions.

use crate::error::{ValidationError, ValidationResult};
use crate::resource::value_objects::{
    Address, EmailAddress, ExternalId, GroupMembers, Meta, MultiValuedAddresses, MultiValuedEmails,
    MultiValuedPhoneNumbers, Name, PhoneNumber, ResourceId, SchemaUri, UserName,
};

use serde_json::{Map, Value};

/// Generic SCIM resource representation with type-safe core attributes.
///
/// This hybrid design uses value objects for core validated primitives while
/// maintaining JSON flexibility for extensible attributes. The design ensures
/// compile-time safety for critical fields while preserving SCIM's extensibility.
#[derive(Debug, Clone)]
pub struct Resource {
    /// The type of this resource (e.g., "User", "Group")
    pub resource_type: String,
    /// Validated resource identifier (required for most operations)
    pub id: Option<ResourceId>,
    /// Validated schema URIs
    pub schemas: Vec<SchemaUri>,
    /// Validated external identifier (optional)
    pub external_id: Option<ExternalId>,
    /// Validated username (for User resources)
    pub user_name: Option<UserName>,
    /// Validated meta attributes (optional)
    pub meta: Option<Meta>,
    /// Validated name attributes (for User resources)
    pub name: Option<Name>,
    /// Validated addresses (multi-valued with primary support)
    pub addresses: Option<MultiValuedAddresses>,
    /// Validated phone numbers (multi-valued with primary support)
    pub phone_numbers: Option<MultiValuedPhoneNumbers>,
    /// Validated email addresses (multi-valued with primary support)
    pub emails: Option<MultiValuedEmails>,
    /// Group members (for Group resources)
    pub members: Option<GroupMembers>,
    /// Extended attributes and complex data as JSON
    pub attributes: Map<String, Value>,
}

impl Resource {
    /// Create a new resource from validated JSON data.
    ///
    /// This method extracts and validates core primitives while preserving
    /// other attributes in the flexible JSON structure.
    ///
    /// # Arguments
    /// * `resource_type` - The SCIM resource type identifier
    /// * `data` - The resource data as a JSON value
    ///
    /// # Example
    /// ```rust
    /// use scim_server::Resource;
    /// use serde_json::json;
    ///
    /// fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let user_data = json!({
    ///         "id": "12345",
    ///         "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
    ///         "userName": "jdoe",
    ///         "displayName": "John Doe"
    ///     });
    ///     let resource = Resource::from_json("User".to_string(), user_data)?;
    ///
    ///     Ok(())
    /// }
    /// ```
    pub fn from_json(resource_type: String, data: Value) -> ValidationResult<Self> {
        let obj = data
            .as_object()
            .ok_or_else(|| ValidationError::custom("Resource must be a JSON object"))?;

        // Extract and validate core primitives
        let id = Self::extract_resource_id(obj)?;
        let schemas = Self::extract_schemas(obj, &resource_type)?;
        let external_id = Self::extract_external_id(obj)?;
        let user_name = Self::extract_user_name(obj)?;
        let meta = Self::extract_meta(&data)?;
        let name = Self::extract_name(obj)?;
        let addresses = Self::extract_addresses(obj)?;
        let phone_numbers = Self::extract_phone_numbers(obj)?;
        let emails = Self::extract_emails(obj)?;
        let members = Self::extract_members(obj)?;

        // Collect remaining attributes (excluding core primitives)
        let mut attributes = obj.clone();
        attributes.remove("id");
        attributes.remove("schemas");
        attributes.remove("externalId");
        attributes.remove("userName");
        attributes.remove("meta");
        attributes.remove("name");
        attributes.remove("addresses");
        attributes.remove("phoneNumbers");
        attributes.remove("emails");
        attributes.remove("members");

        Ok(Self {
            resource_type,
            id,
            schemas,
            external_id,
            user_name,
            meta,
            name,
            addresses,
            phone_numbers,
            emails,
            members,
            attributes,
        })
    }

    /// Create a new resource with validated core fields.
    ///
    /// This is the preferred constructor for new resources where core fields
    /// are already validated.
    pub fn new(
        resource_type: String,
        id: Option<ResourceId>,
        schemas: Vec<SchemaUri>,
        external_id: Option<ExternalId>,
        user_name: Option<UserName>,
        attributes: Map<String, Value>,
    ) -> Self {
        Self {
            resource_type,
            id,
            schemas,
            external_id,
            user_name,
            meta: None,
            name: None,
            addresses: None,
            phone_numbers: None,
            emails: None,
            members: None,
            attributes,
        }
    }

    /// Create a new resource with validated core fields including meta.
    ///
    /// Extended constructor that includes meta attributes.
    pub fn new_with_meta(
        resource_type: String,
        id: Option<ResourceId>,
        schemas: Vec<SchemaUri>,
        external_id: Option<ExternalId>,
        user_name: Option<UserName>,
        meta: Option<Meta>,
        attributes: Map<String, Value>,
    ) -> Self {
        Self {
            resource_type,
            id,
            schemas,
            external_id,
            user_name,
            meta,
            name: None,
            addresses: None,
            phone_numbers: None,
            emails: None,
            members: None,
            attributes,
        }
    }

    /// Extract and validate resource ID from JSON
    fn extract_resource_id(obj: &Map<String, Value>) -> ValidationResult<Option<ResourceId>> {
        if let Some(id_value) = obj.get("id") {
            if let Some(id_str) = id_value.as_str() {
                return Ok(Some(ResourceId::new(id_str.to_string())?));
            } else {
                return Err(ValidationError::InvalidIdFormat {
                    id: id_value.to_string(),
                });
            }
        }
        Ok(None)
    }

    /// Extract and validate schemas from JSON
    fn extract_schemas(
        obj: &Map<String, Value>,
        resource_type: &str,
    ) -> ValidationResult<Vec<SchemaUri>> {
        if let Some(schemas_value) = obj.get("schemas") {
            if let Some(schemas_array) = schemas_value.as_array() {
                if schemas_array.is_empty() {
                    return Err(ValidationError::EmptySchemas);
                }

                let mut schemas = Vec::new();
                for schema_value in schemas_array {
                    if let Some(uri_str) = schema_value.as_str() {
                        schemas.push(SchemaUri::new(uri_str.to_string())?);
                    }
                }
                if !schemas.is_empty() {
                    return Ok(schemas);
                }
            }
        }

        // Default schema based on resource type
        let default_uri = match resource_type {
            "User" => "urn:ietf:params:scim:schemas:core:2.0:User",
            "Group" => "urn:ietf:params:scim:schemas:core:2.0:Group",
            _ => return Err(ValidationError::custom("Unknown resource type")),
        };

        Ok(vec![SchemaUri::new(default_uri.to_string())?])
    }

    /// Extract and validate external ID from JSON
    fn extract_external_id(obj: &Map<String, Value>) -> ValidationResult<Option<ExternalId>> {
        if let Some(ext_id_value) = obj.get("externalId") {
            if let Some(ext_id_str) = ext_id_value.as_str() {
                return Ok(Some(ExternalId::new(ext_id_str.to_string())?));
            } else {
                return Err(ValidationError::InvalidExternalId);
            }
        }
        Ok(None)
    }

    /// Extract and validate username from JSON
    fn extract_user_name(obj: &Map<String, Value>) -> ValidationResult<Option<UserName>> {
        if let Some(username_value) = obj.get("userName") {
            if let Some(username_str) = username_value.as_str() {
                return Ok(Some(UserName::new(username_str.to_string())?));
            } else {
                return Err(ValidationError::custom(
                    "userName must be a string".to_string(),
                ));
            }
        }
        Ok(None)
    }

    /// Extract and validate name from JSON
    fn extract_name(obj: &Map<String, Value>) -> ValidationResult<Option<Name>> {
        if let Some(name_value) = obj.get("name") {
            if let Some(_) = name_value.as_object() {
                // Deserialize using serde
                let name: Name = serde_json::from_value(name_value.clone())
                    .map_err(|e| ValidationError::custom(format!("Invalid name format: {}", e)))?;
                return Ok(Some(name));
            } else {
                return Err(ValidationError::custom(
                    "name must be an object".to_string(),
                ));
            }
        }
        Ok(None)
    }

    /// Extract and validate addresses from JSON
    fn extract_addresses(
        obj: &Map<String, Value>,
    ) -> ValidationResult<Option<MultiValuedAddresses>> {
        if let Some(addresses_value) = obj.get("addresses") {
            if let Some(_) = addresses_value.as_array() {
                // Deserialize using serde
                let addresses: Vec<Address> = serde_json::from_value(addresses_value.clone())
                    .map_err(|e| {
                        ValidationError::custom(format!("Invalid addresses format: {}", e))
                    })?;
                if !addresses.is_empty() {
                    let multi_addresses = MultiValuedAddresses::new(addresses)?;
                    return Ok(Some(multi_addresses));
                }
            } else {
                return Err(ValidationError::custom(
                    "addresses must be an array".to_string(),
                ));
            }
        }
        Ok(None)
    }

    /// Extract and validate phone numbers from JSON
    fn extract_phone_numbers(
        obj: &Map<String, Value>,
    ) -> ValidationResult<Option<MultiValuedPhoneNumbers>> {
        if let Some(phones_value) = obj.get("phoneNumbers") {
            if let Some(_) = phones_value.as_array() {
                // Deserialize using serde
                let phone_numbers: Vec<PhoneNumber> = serde_json::from_value(phones_value.clone())
                    .map_err(|e| {
                        ValidationError::custom(format!("Invalid phoneNumbers format: {}", e))
                    })?;
                if !phone_numbers.is_empty() {
                    let multi_phones = MultiValuedPhoneNumbers::new(phone_numbers)?;
                    return Ok(Some(multi_phones));
                }
            } else {
                return Err(ValidationError::custom(
                    "phoneNumbers must be an array".to_string(),
                ));
            }
        }
        Ok(None)
    }

    /// Extract and validate email addresses from JSON
    fn extract_emails(obj: &Map<String, Value>) -> ValidationResult<Option<MultiValuedEmails>> {
        if let Some(emails_value) = obj.get("emails") {
            if let Some(_) = emails_value.as_array() {
                // Deserialize using serde
                let emails: Vec<EmailAddress> = serde_json::from_value(emails_value.clone())
                    .map_err(|e| {
                        ValidationError::custom(format!("Invalid emails format: {}", e))
                    })?;
                if !emails.is_empty() {
                    let multi_emails = MultiValuedEmails::new(emails)?;
                    return Ok(Some(multi_emails));
                }
            } else {
                return Err(ValidationError::custom(
                    "emails must be an array".to_string(),
                ));
            }
        }
        Ok(None)
    }

    /// Extract and validate group members from JSON
    fn extract_members(obj: &Map<String, Value>) -> ValidationResult<Option<GroupMembers>> {
        if let Some(members_value) = obj.get("members") {
            if let Some(_) = members_value.as_array() {
                // Deserialize using serde to get the raw member data
                let members_data: Vec<serde_json::Value> =
                    serde_json::from_value(members_value.clone()).map_err(|e| {
                        ValidationError::custom(format!("Invalid members format: {}", e))
                    })?;

                let mut members = Vec::new();
                for member_data in members_data {
                    if let Some(obj) = member_data.as_object() {
                        if let Some(value_str) = obj.get("value").and_then(|v| v.as_str()) {
                            let resource_id = ResourceId::new(value_str.to_string())?;
                            let display = obj
                                .get("display")
                                .and_then(|v| v.as_str())
                                .map(|s| s.to_string());
                            let member_type = obj
                                .get("type")
                                .and_then(|v| v.as_str())
                                .map(|s| s.to_string());

                            let member = crate::resource::value_objects::GroupMember::new(
                                resource_id,
                                display,
                                member_type,
                            )?;
                            members.push(member);
                        }
                    }
                }

                if !members.is_empty() {
                    let group_members = GroupMembers::new(members)?;
                    return Ok(Some(group_members));
                }
            } else {
                return Err(ValidationError::custom(
                    "members must be an array".to_string(),
                ));
            }
        }
        Ok(None)
    }

    /// Get the unique identifier of this resource.
    pub fn get_id(&self) -> Option<&str> {
        self.id.as_ref().map(|id| id.as_str())
    }

    /// Set the unique identifier of this resource.
    pub fn set_id(&mut self, id: &str) -> ValidationResult<()> {
        self.id = Some(ResourceId::new(id.to_string())?);
        Ok(())
    }

    /// Get an attribute value from the resource.
    pub fn get(&self, key: &str) -> Option<&Value> {
        self.attributes.get(key)
    }

    /// Get the userName field for User resources.
    pub fn get_username(&self) -> Option<&str> {
        self.user_name.as_ref().map(|name| name.as_str())
    }

    /// Get the name field for User resources.
    pub fn get_name(&self) -> Option<&Name> {
        self.name.as_ref()
    }

    /// Get all addresses for the resource.
    pub fn get_addresses(&self) -> Option<&MultiValuedAddresses> {
        self.addresses.as_ref()
    }

    /// Get all phone numbers for the resource.
    pub fn get_phone_numbers(&self) -> Option<&MultiValuedPhoneNumbers> {
        self.phone_numbers.as_ref()
    }

    /// Get all emails for the resource.
    pub fn get_emails(&self) -> Option<&MultiValuedEmails> {
        self.emails.as_ref()
    }

    /// Get all group members for the resource.
    pub fn get_members(&self) -> Option<&GroupMembers> {
        self.members.as_ref()
    }

    /// Set the name for the resource.
    pub fn set_name(&mut self, name: Name) {
        self.name = Some(name);
    }

    /// Set addresses for the resource.
    pub fn set_addresses(&mut self, addresses: MultiValuedAddresses) {
        self.addresses = Some(addresses);
    }

    /// Set phone numbers for the resource.
    pub fn set_phone_numbers(&mut self, phone_numbers: MultiValuedPhoneNumbers) {
        self.phone_numbers = Some(phone_numbers);
    }

    /// Set emails for the resource.
    pub fn set_emails(&mut self, emails: MultiValuedEmails) {
        self.emails = Some(emails);
    }

    /// Set group members for the resource.
    pub fn set_members(&mut self, members: GroupMembers) {
        self.members = Some(members);
    }

    /// Add an address to the resource.
    pub fn add_address(&mut self, address: Address) -> ValidationResult<()> {
        match &self.addresses {
            Some(existing) => {
                let new_addresses = existing.clone().with_value(address);
                self.addresses = Some(new_addresses);
            }
            None => {
                let new_addresses = MultiValuedAddresses::single(address);
                self.addresses = Some(new_addresses);
            }
        }
        Ok(())
    }

    /// Add a phone number to the resource.
    pub fn add_phone_number(&mut self, phone_number: PhoneNumber) -> ValidationResult<()> {
        match &self.phone_numbers {
            Some(existing) => {
                let new_phones = existing.clone().with_value(phone_number);
                self.phone_numbers = Some(new_phones);
            }
            None => {
                let new_phones = MultiValuedPhoneNumbers::single(phone_number);
                self.phone_numbers = Some(new_phones);
            }
        }
        Ok(())
    }

    /// Add an email to the resource.
    pub fn add_email(&mut self, email: EmailAddress) -> ValidationResult<()> {
        match &self.emails {
            Some(existing) => {
                let new_emails = existing.clone().with_value(email);
                self.emails = Some(new_emails);
            }
            None => {
                let new_emails = MultiValuedEmails::single(email);
                self.emails = Some(new_emails);
            }
        }
        Ok(())
    }

    /// Get a specific attribute value from the extended attributes.
    ///
    /// # Arguments
    /// * `attribute_name` - The name of the attribute to retrieve
    pub fn get_attribute(&self, attribute_name: &str) -> Option<&Value> {
        self.attributes.get(attribute_name)
    }

    /// Set a specific attribute value in the extended attributes.
    ///
    /// # Arguments
    /// * `attribute_name` - The name of the attribute to set
    /// * `value` - The value to set
    pub fn set_attribute(&mut self, attribute_name: String, value: Value) {
        self.attributes.insert(attribute_name, value);
    }

    /// Get the schemas associated with this resource.
    pub fn get_schemas(&self) -> Vec<String> {
        self.schemas
            .iter()
            .map(|s| s.as_str().to_string())
            .collect()
    }

    /// Get the validated schema URIs.
    pub fn get_schema_uris(&self) -> &[SchemaUri] {
        &self.schemas
    }

    /// Add metadata to the resource.
    ///
    /// This method sets common SCIM metadata fields like resourceType,
    /// created, lastModified, and location using the new Meta value object.
    ///
    /// # Deprecated
    /// This method is deprecated in favor of `create_meta()` which uses type-safe Meta value objects.
    pub fn add_metadata(&mut self, base_url: &str, created: &str, last_modified: &str) {
        // Parse timestamps
        let created_dt = chrono::DateTime::parse_from_rfc3339(created)
            .map(|dt| dt.with_timezone(&chrono::Utc))
            .unwrap_or_else(|_| chrono::Utc::now());

        let last_modified_dt = chrono::DateTime::parse_from_rfc3339(last_modified)
            .map(|dt| dt.with_timezone(&chrono::Utc))
            .unwrap_or_else(|_| chrono::Utc::now());

        let location = if let Some(id) = &self.id {
            Some(Meta::generate_location(
                base_url,
                &self.resource_type,
                id.as_str(),
            ))
        } else {
            None
        };

        let version = if let Some(id) = &self.id {
            Some(Meta::generate_version(id.as_str(), last_modified_dt))
        } else {
            None
        };

        // Create Meta value object (ignore validation errors for backward compatibility)
        if let Ok(meta) = Meta::new(
            self.resource_type.clone(),
            created_dt,
            last_modified_dt,
            location,
            version,
        ) {
            self.set_meta(meta);
        }
    }

    /// Check if this resource is active.
    ///
    /// Returns the value of the "active" field, defaulting to true if not present.
    pub fn is_active(&self) -> bool {
        self.attributes
            .get("active")
            .and_then(|v| v.as_bool())
            .unwrap_or(true)
    }

    /// Convert the resource to JSON format for serialization.
    ///
    /// This combines the type-safe core fields with the extended attributes
    /// into a single JSON object.
    pub fn to_json(&self) -> ValidationResult<Value> {
        let mut result = self.attributes.clone();

        // Add core fields
        if let Some(ref id) = self.id {
            result.insert("id".to_string(), Value::String(id.as_str().to_string()));
        }

        let schemas: Vec<Value> = self
            .schemas
            .iter()
            .map(|s| Value::String(s.as_str().to_string()))
            .collect();
        result.insert("schemas".to_string(), Value::Array(schemas));

        if let Some(ref external_id) = self.external_id {
            result.insert(
                "externalId".to_string(),
                Value::String(external_id.as_str().to_string()),
            );
        }

        if let Some(ref user_name) = self.user_name {
            result.insert(
                "userName".to_string(),
                Value::String(user_name.as_str().to_string()),
            );
        }

        // Add meta field if present (prioritize value object over JSON attributes)
        if let Some(ref meta) = self.meta {
            if let Ok(meta_json) = serde_json::to_value(meta) {
                result.insert("meta".to_string(), meta_json);
            }
        }

        // Add name field if present
        if let Some(ref name) = self.name {
            if let Ok(name_json) = serde_json::to_value(name) {
                result.insert("name".to_string(), name_json);
            }
        }

        // Add addresses if present
        if let Some(ref addresses) = self.addresses {
            let addresses_json = serde_json::to_value(addresses.values())
                .map_err(|e| ValidationError::custom(format!("Serialization error: {}", e)))?;
            result.insert("addresses".to_string(), addresses_json);
        }

        if let Some(ref phone_numbers) = self.phone_numbers {
            let phones_json = serde_json::to_value(phone_numbers.values())
                .map_err(|e| ValidationError::custom(format!("Serialization error: {}", e)))?;
            result.insert("phoneNumbers".to_string(), phones_json);
        }

        if let Some(ref emails) = self.emails {
            let emails_json = serde_json::to_value(emails.values())
                .map_err(|e| ValidationError::custom(format!("Serialization error: {}", e)))?;
            result.insert("emails".to_string(), emails_json);
        }

        if let Some(ref members) = self.members {
            let members_json = serde_json::to_value(members.values())
                .map_err(|e| ValidationError::custom(format!("Serialization error: {}", e)))?;
            result.insert("members".to_string(), members_json);
        }

        Ok(Value::Object(result))
    }

    /// Get the external id if present.
    pub fn get_external_id(&self) -> Option<&str> {
        self.external_id.as_ref().map(|id| id.as_str())
    }

    /// Extract meta attributes from JSON data.
    fn extract_meta(data: &Value) -> ValidationResult<Option<Meta>> {
        if let Some(meta_value) = data.get("meta") {
            if let Some(meta_obj) = meta_value.as_object() {
                // Check if we have minimal required fields
                let resource_type = if let Some(rt_value) = meta_obj.get("resourceType") {
                    if let Some(rt_str) = rt_value.as_str() {
                        Some(rt_str.to_string())
                    } else {
                        // resourceType exists but is not a string
                        return Err(ValidationError::InvalidMetaStructure);
                    }
                } else {
                    None
                };

                let created = meta_obj.get("created").and_then(|v| v.as_str());
                let last_modified = meta_obj.get("lastModified").and_then(|v| v.as_str());

                // If meta has any non-string types for datetime fields, fail immediately
                if meta_obj.get("created").is_some() && created.is_none() {
                    return Err(ValidationError::InvalidCreatedDateTime);
                }
                if meta_obj.get("lastModified").is_some() && last_modified.is_none() {
                    return Err(ValidationError::InvalidModifiedDateTime);
                }

                // Validate location field data type if present
                if let Some(location_value) = meta_obj.get("location") {
                    if !location_value.is_string() {
                        return Err(ValidationError::InvalidLocationUri);
                    }
                }

                // Validate version field data type if present
                if let Some(version_value) = meta_obj.get("version") {
                    if !version_value.is_string() {
                        return Err(ValidationError::InvalidVersionFormat);
                    }
                }

                // Only proceed if we have both resourceType and timestamps
                if let (Some(resource_type), Some(created), Some(last_modified)) =
                    (resource_type, created, last_modified)
                {
                    let location = meta_obj
                        .get("location")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string());

                    let version = meta_obj
                        .get("version")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string());

                    // Parse DateTime strings with strict validation
                    let created_dt = chrono::DateTime::parse_from_rfc3339(created)
                        .map_err(|_| ValidationError::InvalidCreatedDateTime)?
                        .with_timezone(&chrono::Utc);

                    let last_modified_dt = chrono::DateTime::parse_from_rfc3339(last_modified)
                        .map_err(|_| ValidationError::InvalidModifiedDateTime)?
                        .with_timezone(&chrono::Utc);

                    let meta = Meta::new(
                        resource_type,
                        created_dt,
                        last_modified_dt,
                        location,
                        version,
                    )?;
                    Ok(Some(meta))
                } else {
                    // Meta exists but is incomplete - ignore it for backward compatibility
                    Ok(None)
                }
            } else {
                Err(ValidationError::InvalidMetaStructure)
            }
        } else {
            Ok(None)
        }
    }

    /// Get the meta attributes if present.
    pub fn get_meta(&self) -> Option<&Meta> {
        self.meta.as_ref()
    }

    /// Set meta attributes for the resource.
    pub fn set_meta(&mut self, meta: Meta) {
        // Update the JSON representation before moving
        let meta_json = serde_json::to_value(&meta).unwrap_or(Value::Null);
        self.set_attribute("meta".to_string(), meta_json);
        self.meta = Some(meta);
    }

    /// Create meta attributes for a new resource.
    pub fn create_meta(&mut self, base_url: &str) -> ValidationResult<()> {
        let meta = Meta::new_for_creation(self.resource_type.clone())?;
        let meta_with_location = if let Some(id) = &self.id {
            let location = Meta::generate_location(base_url, &self.resource_type, id.as_str());
            meta.with_location(location)?
        } else {
            meta
        };
        self.set_meta(meta_with_location);
        Ok(())
    }

    /// Update meta attributes with current timestamp.
    pub fn update_meta(&mut self) {
        if let Some(meta) = &self.meta {
            let updated_meta = meta.with_updated_timestamp();
            self.set_meta(updated_meta);
        }
    }
}
