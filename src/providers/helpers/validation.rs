//! SCIM validation helper trait.
//!
//! This module provides reusable validation functionality for SCIM attribute paths,
//! attribute types, and schema compliance. Any ResourceProvider can implement this
//! trait to get comprehensive SCIM validation without reimplementing the logic.
//!
//! # RFC 7644 Compliance
//!
//! This implementation follows RFC 7644 and RFC 7643 specifications for:
//! - Attribute name validation
//! - Path expression parsing
//! - Multi-valued attribute detection
//! - Schema URN handling
//! - Standard SCIM attribute recognition
//!
//! # Usage
//!
//! ```rust,no_run
//! // ScimValidator provides SCIM-compliant validation methods:
//! // - is_valid_scim_path(): Validate SCIM attribute paths
//! // - validate_schema_compliance(): Check resource against schemas
//! // - validate_required_attributes(): Ensure required fields present
//! //
//! // Example SCIM paths:
//! // "name.givenName" - Valid nested attribute path
//! // "emails[type eq \"work\"].value" - Valid filtered multi-valued path
//! // "phoneNumbers[primary eq true]" - Valid filtered array path
//! //
//! // When implemented by a ResourceProvider, automatically provides
//! // validation for PATCH operations, resource creation, and updates
//! ```

use crate::providers::ResourceProvider;

/// Trait providing SCIM attribute validation and path parsing functionality.
///
/// This trait extends ResourceProvider with validation capabilities for SCIM
/// attributes, paths, and schema compliance. Most implementers can use the
/// default implementations which provide RFC-compliant behavior.
pub trait ScimValidator: ResourceProvider {
    /// Validate if a path represents a valid SCIM attribute.
    ///
    /// Validates SCIM attribute paths according to RFC specifications:
    /// - Simple attributes: "userName", "active"
    /// - Complex attributes: "name.givenName", "name.familyName"
    /// - Schema URN prefixed: "urn:ietf:params:scim:schemas:core:2.0:User:userName"
    /// - Multi-valued with filters: "emails[type eq \"work\"].value"
    ///
    /// # Arguments
    /// * `path` - The attribute path to validate
    ///
    /// # Returns
    /// `true` if the path is valid according to SCIM specifications
    fn is_valid_scim_path(&self, path: &str) -> bool {
        if path.is_empty() {
            return false;
        }

        // Handle schema URN prefixed paths
        let actual_path = if path.contains(':') && path.contains("urn:ietf:params:scim:schemas:") {
            // Extract the attribute name after the schema URN
            // e.g., "urn:ietf:params:scim:schemas:core:2.0:User:userName" -> "userName"
            if let Some(colon_pos) = path.rfind(':') {
                &path[colon_pos + 1..]
            } else {
                path
            }
        } else {
            path
        };

        // Handle filter expressions in brackets (e.g., emails[type eq "work"])
        let clean_path = if actual_path.contains('[') {
            if let Some(bracket_pos) = actual_path.find('[') {
                &actual_path[..bracket_pos]
            } else {
                actual_path
            }
        } else {
            actual_path
        };

        // Check if it's a simple or complex path
        if clean_path.contains('.') {
            self.is_valid_complex_path(clean_path)
        } else {
            self.is_valid_simple_path(clean_path)
        }
    }

    /// Validate a complex SCIM path (containing dots).
    ///
    /// Examples: "name.givenName", "addresses.streetAddress", "meta.created"
    ///
    /// # Arguments
    /// * `path` - The complex path to validate
    fn is_valid_complex_path(&self, path: &str) -> bool {
        let parts: Vec<&str> = path.split('.').collect();

        if parts.len() < 2 {
            return false;
        }

        // All parts must be valid simple attributes
        parts.iter().all(|part| {
            !part.is_empty()
                && part.chars().all(|c| c.is_alphanumeric() || c == '_')
                && part.chars().next().map_or(false, |c| c.is_alphabetic())
        })
    }

    /// Validate a simple SCIM path (single attribute name).
    ///
    /// Checks against known SCIM User and Group attributes from RFC 7643.
    ///
    /// # Arguments
    /// * `attribute` - The simple attribute name to validate
    fn is_valid_simple_path(&self, attribute: &str) -> bool {
        // Standard SCIM User attributes (RFC 7643 Section 4.1)
        let user_attributes = [
            "id",
            "externalId",
            "userName",
            "password",
            "displayName",
            "nickName",
            "profileUrl",
            "title",
            "userType",
            "preferredLanguage",
            "locale",
            "timezone",
            "active",
            "name",
            "emails",
            "phoneNumbers",
            "ims",
            "photos",
            "addresses",
            "groups",
            "entitlements",
            "roles",
            "x509Certificates",
            "meta",
        ];

        // Standard SCIM Group attributes (RFC 7643 Section 4.2)
        let group_attributes = ["id", "externalId", "displayName", "members", "meta"];

        // Core meta attributes
        let meta_attributes = [
            "resourceType",
            "created",
            "lastModified",
            "location",
            "version",
        ];

        // Complex attribute sub-components
        let complex_sub_attributes = [
            // Name sub-attributes
            "formatted",
            "familyName",
            "givenName",
            "middleName",
            "honorificPrefix",
            "honorificSuffix",
            // Multi-valued sub-attributes
            "value",
            "display",
            "type",
            "primary",
            "operation",
            "$ref",
            // Address sub-attributes
            "streetAddress",
            "locality",
            "region",
            "postalCode",
            "country",
            // Enterprise extension commonly used attributes
            "employeeNumber",
            "costCenter",
            "organization",
            "division",
            "department",
            "manager",
        ];

        // Check if attribute matches any known SCIM attribute
        user_attributes.contains(&attribute) ||
        group_attributes.contains(&attribute) ||
        meta_attributes.contains(&attribute) ||
        complex_sub_attributes.contains(&attribute) ||
        // Allow custom extension attributes (starting with letter, containing alphanumeric + underscore)
        (attribute.chars().next().map_or(false, |c| c.is_alphabetic()) &&
         attribute.chars().all(|c| c.is_alphanumeric() || c == '_'))
    }

    /// Check if an attribute is multi-valued according to SCIM specifications.
    ///
    /// Multi-valued attributes can contain arrays of complex objects with
    /// common sub-attributes like "value", "type", "primary", etc.
    ///
    /// # Arguments
    /// * `attribute_name` - The attribute name to check
    ///
    /// # Returns
    /// `true` if the attribute is multi-valued according to SCIM specs
    fn is_multivalued_attribute(&self, attribute_name: &str) -> bool {
        matches!(
            attribute_name,
            "emails"
                | "phoneNumbers"
                | "ims"
                | "photos"
                | "addresses"
                | "groups"
                | "entitlements"
                | "roles"
                | "x509Certificates"
                | "members" // For Group resources
        )
    }

    /// Check if an attribute path refers to a readonly attribute.
    ///
    /// Readonly attributes cannot be modified through PATCH or PUT operations
    /// according to SCIM specifications.
    ///
    /// # Arguments
    /// * `path` - The attribute path to check
    ///
    /// # Returns
    /// `true` if the attribute is readonly and cannot be modified
    fn is_readonly_attribute(&self, path: &str) -> bool {
        match path.to_lowercase().as_str() {
            // Core readonly attributes per RFC 7643
            "id" => true,
            "meta" => true,
            "meta.resourcetype" => true,
            "meta.created" => true,
            "meta.location" => true,
            // Pattern matching for nested meta attributes
            path if path.starts_with("meta.")
                && (path.ends_with(".resourcetype")
                    || path.ends_with(".created")
                    || path.ends_with(".location")) =>
            {
                true
            }
            // Groups members can be readonly in some implementations
            "groups.display" => true,
            "groups.$ref" => true,
            _ => false,
        }
    }

    /// Validate that a username meets SCIM requirements.
    ///
    /// SCIM usernames should be unique within a tenant and follow
    /// reasonable formatting rules for identifiers.
    ///
    /// # Arguments
    /// * `username` - The username to validate
    ///
    /// # Returns
    /// `true` if the username is valid according to SCIM guidelines
    fn is_valid_username(&self, username: &str) -> bool {
        if username.is_empty() || username.len() > 256 {
            return false;
        }

        // Username should not contain control characters or be only whitespace
        if username.trim().is_empty() || username.chars().any(|c| c.is_control()) {
            return false;
        }

        // Allow common username formats:
        // - email addresses: user@domain.com
        // - alphanumeric with common separators: john.doe, john_doe, john-doe
        // - numbers: user123
        username.chars().all(|c| {
            c.is_alphanumeric() || c == '.' || c == '_' || c == '-' || c == '@' || c == '+'
        })
    }

    /// Validate an external ID format.
    ///
    /// External IDs are used to correlate SCIM resources with external systems
    /// and should follow reasonable identifier patterns.
    ///
    /// # Arguments
    /// * `external_id` - The external ID to validate
    ///
    /// # Returns
    /// `true` if the external ID format is acceptable
    fn is_valid_external_id(&self, external_id: &str) -> bool {
        if external_id.is_empty() || external_id.len() > 512 {
            return false;
        }

        // External ID should not be only whitespace or contain control characters
        !external_id.trim().is_empty() && !external_id.chars().any(|c| c.is_control())
    }

    /// Check if a schema URI is valid for SCIM.
    ///
    /// SCIM uses URNs to identify schemas and extensions.
    /// Common patterns: "urn:ietf:params:scim:schemas:core:2.0:User"
    ///
    /// # Arguments
    /// * `schema_uri` - The schema URI to validate
    ///
    /// # Returns
    /// `true` if the schema URI follows SCIM URN conventions
    fn is_valid_schema_uri(&self, schema_uri: &str) -> bool {
        if schema_uri.is_empty() {
            return false;
        }

        // Must start with urn: and contain scim schemas
        schema_uri.starts_with("urn:") &&
        schema_uri.contains("scim:schemas") &&
        // Should have reasonable length
        schema_uri.len() <= 512 &&
        // Should not contain control characters
        !schema_uri.chars().any(|c| c.is_control())
    }

    /// Extract the attribute name from a potentially complex path.
    ///
    /// Handles various SCIM path formats and extracts the base attribute name:
    /// - "userName" -> "userName"
    /// - "name.givenName" -> "name"
    /// - "emails[type eq \"work\"].value" -> "emails"
    /// - "urn:ietf:params:scim:schemas:core:2.0:User:userName" -> "userName"
    ///
    /// # Arguments
    /// * `path` - The SCIM path to parse
    ///
    /// # Returns
    /// The base attribute name, or the original path if parsing fails
    fn extract_base_attribute<'a>(&self, path: &'a str) -> &'a str {
        // Handle schema URN prefixed paths first
        let clean_path = if path.contains(':') && path.contains("urn:ietf:params:scim:schemas:") {
            if let Some(colon_pos) = path.rfind(':') {
                &path[colon_pos + 1..]
            } else {
                path
            }
        } else {
            path
        };

        // Handle filter expressions in brackets
        let without_filter = if let Some(bracket_pos) = clean_path.find('[') {
            &clean_path[..bracket_pos]
        } else {
            clean_path
        };

        // Handle complex paths (take the first part before dot)
        if let Some(dot_pos) = without_filter.find('.') {
            &without_filter[..dot_pos]
        } else {
            without_filter
        }
    }
}

/// Default implementation for any ResourceProvider
impl<T: ResourceProvider> ScimValidator for T {}
