//! Embedded core SCIM schemas for schema discovery functionality.
//!
//! This module provides the core SCIM schemas (User, Group, ServiceProviderConfig)
//! embedded as static strings, eliminating the need for external schema files
//! for basic schema discovery functionality.

/// Returns the core User schema as a JSON string.
///
/// This is the standard SCIM 2.0 User schema as defined in RFC 7643.
pub fn core_user_schema() -> &'static str {
    r#"{
  "id": "urn:ietf:params:scim:schemas:core:2.0:User",
  "name": "User",
  "description": "User Account",
  "attributes": [
    {
      "name": "id",
      "type": "string",
      "multiValued": false,
      "required": false,
      "caseExact": true,
      "mutability": "readOnly",
      "returned": "always",
      "uniqueness": "server"
    },
    {
      "name": "userName",
      "type": "string",
      "multiValued": false,
      "required": true,
      "caseExact": false,
      "mutability": "readWrite",
      "returned": "default",
      "uniqueness": "server"
    },
    {
      "name": "externalId",
      "type": "string",
      "multiValued": false,
      "required": false,
      "caseExact": true,
      "mutability": "readWrite",
      "returned": "default",
      "uniqueness": "none"
    },
    {
      "name": "displayName",
      "type": "string",
      "multiValued": false,
      "required": false,
      "caseExact": false,
      "mutability": "readWrite",
      "returned": "default",
      "uniqueness": "none"
    },
    {
      "name": "nickName",
      "type": "string",
      "multiValued": false,
      "required": false,
      "caseExact": false,
      "mutability": "readWrite",
      "returned": "default",
      "uniqueness": "none"
    },
    {
      "name": "profileUrl",
      "type": "reference",
      "multiValued": false,
      "required": false,
      "caseExact": false,
      "mutability": "readWrite",
      "returned": "default",
      "uniqueness": "none"
    },
    {
      "name": "title",
      "type": "string",
      "multiValued": false,
      "required": false,
      "caseExact": false,
      "mutability": "readWrite",
      "returned": "default",
      "uniqueness": "none"
    },
    {
      "name": "userType",
      "type": "string",
      "multiValued": false,
      "required": false,
      "caseExact": false,
      "mutability": "readWrite",
      "returned": "default",
      "uniqueness": "none"
    },
    {
      "name": "preferredLanguage",
      "type": "string",
      "multiValued": false,
      "required": false,
      "caseExact": false,
      "mutability": "readWrite",
      "returned": "default",
      "uniqueness": "none"
    },
    {
      "name": "locale",
      "type": "string",
      "multiValued": false,
      "required": false,
      "caseExact": false,
      "mutability": "readWrite",
      "returned": "default",
      "uniqueness": "none"
    },
    {
      "name": "timezone",
      "type": "string",
      "multiValued": false,
      "required": false,
      "caseExact": false,
      "mutability": "readWrite",
      "returned": "default",
      "uniqueness": "none"
    },
    {
      "name": "name",
      "type": "complex",
      "multiValued": false,
      "required": false,
      "caseExact": false,
      "mutability": "readWrite",
      "returned": "default",
      "uniqueness": "none",
      "subAttributes": [
        {
          "name": "formatted",
          "type": "string",
          "multiValued": false,
          "required": false,
          "caseExact": false,
          "mutability": "readWrite",
          "returned": "default",
          "uniqueness": "none"
        },
        {
          "name": "familyName",
          "type": "string",
          "multiValued": false,
          "required": false,
          "caseExact": false,
          "mutability": "readWrite",
          "returned": "default",
          "uniqueness": "none"
        },
        {
          "name": "givenName",
          "type": "string",
          "multiValued": false,
          "required": false,
          "caseExact": false,
          "mutability": "readWrite",
          "returned": "default",
          "uniqueness": "none"
        },
        {
          "name": "middleName",
          "type": "string",
          "multiValued": false,
          "required": false,
          "caseExact": false,
          "mutability": "readWrite",
          "returned": "default",
          "uniqueness": "none"
        },
        {
          "name": "honorificPrefix",
          "type": "string",
          "multiValued": false,
          "required": false,
          "caseExact": false,
          "mutability": "readWrite",
          "returned": "default",
          "uniqueness": "none"
        },
        {
          "name": "honorificSuffix",
          "type": "string",
          "multiValued": false,
          "required": false,
          "caseExact": false,
          "mutability": "readWrite",
          "returned": "default",
          "uniqueness": "none"
        }
      ]
    },
    {
      "name": "emails",
      "type": "complex",
      "multiValued": true,
      "required": false,
      "caseExact": false,
      "mutability": "readWrite",
      "returned": "default",
      "uniqueness": "none",
      "subAttributes": [
        {
          "name": "value",
          "type": "string",
          "multiValued": false,
          "required": true,
          "caseExact": false,
          "mutability": "readWrite",
          "returned": "default",
          "uniqueness": "none"
        },
        {
          "name": "type",
          "type": "string",
          "multiValued": false,
          "required": false,
          "caseExact": false,
          "mutability": "readWrite",
          "returned": "default",
          "uniqueness": "none",
          "canonicalValues": ["work", "home", "other"]
        },
        {
          "name": "primary",
          "type": "boolean",
          "multiValued": false,
          "required": false,
          "caseExact": false,
          "mutability": "readWrite",
          "returned": "default",
          "uniqueness": "none"
        },
        {
          "name": "display",
          "type": "string",
          "multiValued": false,
          "required": false,
          "caseExact": false,
          "mutability": "readWrite",
          "returned": "default",
          "uniqueness": "none"
        }
      ]
    },
    {
      "name": "phoneNumbers",
      "type": "complex",
      "multiValued": true,
      "required": false,
      "caseExact": false,
      "mutability": "readWrite",
      "returned": "default",
      "uniqueness": "none",
      "subAttributes": [
        {
          "name": "value",
          "type": "string",
          "multiValued": false,
          "required": true,
          "caseExact": false,
          "mutability": "readWrite",
          "returned": "default",
          "uniqueness": "none"
        },
        {
          "name": "type",
          "type": "string",
          "multiValued": false,
          "required": false,
          "caseExact": false,
          "mutability": "readWrite",
          "returned": "default",
          "uniqueness": "none",
          "canonicalValues": ["work", "home", "mobile", "fax", "pager", "other"]
        },
        {
          "name": "primary",
          "type": "boolean",
          "multiValued": false,
          "required": false,
          "caseExact": false,
          "mutability": "readWrite",
          "returned": "default",
          "uniqueness": "none"
        },
        {
          "name": "display",
          "type": "string",
          "multiValued": false,
          "required": false,
          "caseExact": false,
          "mutability": "readWrite",
          "returned": "default",
          "uniqueness": "none"
        }
      ]
    },
    {
      "name": "addresses",
      "type": "complex",
      "multiValued": true,
      "required": false,
      "caseExact": false,
      "mutability": "readWrite",
      "returned": "default",
      "uniqueness": "none",
      "subAttributes": [
        {
          "name": "formatted",
          "type": "string",
          "multiValued": false,
          "required": false,
          "caseExact": false,
          "mutability": "readWrite",
          "returned": "default",
          "uniqueness": "none"
        },
        {
          "name": "streetAddress",
          "type": "string",
          "multiValued": false,
          "required": false,
          "caseExact": false,
          "mutability": "readWrite",
          "returned": "default",
          "uniqueness": "none"
        },
        {
          "name": "locality",
          "type": "string",
          "multiValued": false,
          "required": false,
          "caseExact": false,
          "mutability": "readWrite",
          "returned": "default",
          "uniqueness": "none"
        },
        {
          "name": "region",
          "type": "string",
          "multiValued": false,
          "required": false,
          "caseExact": false,
          "mutability": "readWrite",
          "returned": "default",
          "uniqueness": "none"
        },
        {
          "name": "postalCode",
          "type": "string",
          "multiValued": false,
          "required": false,
          "caseExact": false,
          "mutability": "readWrite",
          "returned": "default",
          "uniqueness": "none"
        },
        {
          "name": "country",
          "type": "string",
          "multiValued": false,
          "required": false,
          "caseExact": false,
          "mutability": "readWrite",
          "returned": "default",
          "uniqueness": "none"
        },
        {
          "name": "type",
          "type": "string",
          "multiValued": false,
          "required": false,
          "caseExact": false,
          "mutability": "readWrite",
          "returned": "default",
          "uniqueness": "none",
          "canonicalValues": ["work", "home", "other"]
        },
        {
          "name": "primary",
          "type": "boolean",
          "multiValued": false,
          "required": false,
          "caseExact": false,
          "mutability": "readWrite",
          "returned": "default",
          "uniqueness": "none"
        }
      ]
    },
    {
      "name": "active",
      "type": "boolean",
      "multiValued": false,
      "required": false,
      "caseExact": false,
      "mutability": "readWrite",
      "returned": "default",
      "uniqueness": "none"
    },
    {
      "name": "meta",
      "type": "complex",
      "multiValued": false,
      "required": false,
      "caseExact": false,
      "mutability": "readOnly",
      "returned": "default",
      "uniqueness": "none",
      "subAttributes": [
        {
          "name": "resourceType",
          "type": "string",
          "multiValued": false,
          "required": false,
          "caseExact": true,
          "mutability": "readOnly",
          "returned": "default",
          "uniqueness": "none"
        },
        {
          "name": "created",
          "type": "dateTime",
          "multiValued": false,
          "required": false,
          "caseExact": false,
          "mutability": "readOnly",
          "returned": "default",
          "uniqueness": "none"
        },
        {
          "name": "lastModified",
          "type": "dateTime",
          "multiValued": false,
          "required": false,
          "caseExact": false,
          "mutability": "readOnly",
          "returned": "default",
          "uniqueness": "none"
        },
        {
          "name": "location",
          "type": "reference",
          "multiValued": false,
          "required": false,
          "caseExact": false,
          "mutability": "readOnly",
          "returned": "default",
          "uniqueness": "none"
        },
        {
          "name": "version",
          "type": "string",
          "multiValued": false,
          "required": false,
          "caseExact": true,
          "mutability": "readOnly",
          "returned": "default",
          "uniqueness": "none"
        }
      ]
    }
  ]
}"#
}

/// Returns the core Group schema as a JSON string.
///
/// This is the standard SCIM 2.0 Group schema as defined in RFC 7643.
pub fn core_group_schema() -> &'static str {
    r#"{
  "id": "urn:ietf:params:scim:schemas:core:2.0:Group",
  "name": "Group",
  "description": "Group",
  "attributes": [
    {
      "name": "id",
      "type": "string",
      "multiValued": false,
      "description": "Unique identifier for the SCIM resource as defined by the Service Provider.",
      "required": false,
      "caseExact": true,
      "mutability": "readOnly",
      "returned": "always",
      "uniqueness": "server"
    },
    {
      "name": "externalId",
      "type": "string",
      "multiValued": false,
      "description": "A String that is an identifier for the resource as defined by the provisioning client.",
      "required": false,
      "caseExact": false,
      "mutability": "readWrite",
      "returned": "default",
      "uniqueness": "none"
    },
    {
      "name": "meta",
      "type": "complex",
      "multiValued": false,
      "description": "A complex attribute that contains resource metadata.",
      "required": false,
      "caseExact": false,
      "mutability": "readOnly",
      "returned": "default",
      "uniqueness": "none",
      "subAttributes": [
        {
          "name": "resourceType",
          "type": "string",
          "multiValued": false,
          "description": "The name of the resource type of the resource.",
          "required": false,
          "caseExact": false,
          "mutability": "readOnly",
          "returned": "default",
          "uniqueness": "none"
        },
        {
          "name": "created",
          "type": "dateTime",
          "multiValued": false,
          "description": "The DateTime the Resource was added to the Service Provider.",
          "required": false,
          "caseExact": false,
          "mutability": "readOnly",
          "returned": "default",
          "uniqueness": "none"
        },
        {
          "name": "lastModified",
          "type": "dateTime",
          "multiValued": false,
          "description": "The most recent DateTime that the details of this resource were updated at the Service Provider.",
          "required": false,
          "caseExact": false,
          "mutability": "readOnly",
          "returned": "default",
          "uniqueness": "none"
        },
        {
          "name": "location",
          "type": "string",
          "multiValued": false,
          "description": "The URI of the resource being returned.",
          "required": false,
          "caseExact": false,
          "mutability": "readOnly",
          "returned": "default",
          "uniqueness": "none"
        },
        {
          "name": "version",
          "type": "string",
          "multiValued": false,
          "description": "The version of the resource being returned.",
          "required": false,
          "caseExact": false,
          "mutability": "readOnly",
          "returned": "default",
          "uniqueness": "none"
        }
      ]
    },
    {
      "name": "displayName",
      "type": "string",
      "multiValued": false,
      "description": "A human-readable name for the Group. REQUIRED.",
      "required": false,
      "caseExact": false,
      "mutability": "readWrite",
      "returned": "default",
      "uniqueness": "none"
    },
    {
      "name": "members",
      "type": "complex",
      "multiValued": true,
      "description": "A list of members of the Group.",
      "required": false,
      "caseExact": false,
      "uniqueness": "none",
      "subAttributes": [
        {
          "name": "value",
          "type": "string",
          "multiValued": false,
          "description": "Identifier of the member of this Group.",
          "required": false,
          "caseExact": false,
          "mutability": "immutable",
          "returned": "default",
          "uniqueness": "none"
        },
        {
          "name": "$ref",
          "type": "reference",
          "referenceTypes": ["User", "Group"],
          "multiValued": false,
          "description": "The URI corresponding to a SCIM resource that is a member of this Group.",
          "required": false,
          "caseExact": false,
          "mutability": "immutable",
          "returned": "default",
          "uniqueness": "none"
        },
        {
          "name": "type",
          "type": "string",
          "multiValued": false,
          "description": "A label indicating the type of resource, e.g., 'User' or 'Group'.",
          "required": false,
          "caseExact": false,
          "canonicalValues": ["User", "Group"],
          "mutability": "immutable",
          "returned": "default",
          "uniqueness": "none"
        },
        {
          "name": "display",
          "type": "string",
          "multiValued": false,
          "description": "A human-readable name, primarily used for display purposes. READ-ONLY.",
          "required": false,
          "caseExact": false,
          "mutability": "readOnly",
          "returned": "default",
          "uniqueness": "none"
        }
      ],
      "mutability": "readWrite",
      "returned": "default"
    }
  ]
}"#
}

/// Returns the ServiceProviderConfig schema as a JSON string.
///
/// This is the standard SCIM 2.0 ServiceProviderConfig schema as defined in RFC 7643.
pub fn service_provider_config_schema() -> &'static str {
    r#"{
  "id": "urn:ietf:params:scim:schemas:core:2.0:ServiceProviderConfig",
  "name": "ServiceProviderConfig",
  "description": "Schema for representing the service provider's configuration",
  "attributes": [
    {
      "name": "documentationUri",
      "type": "reference",
      "multiValued": false,
      "required": false,
      "caseExact": false,
      "mutability": "readOnly",
      "returned": "default",
      "uniqueness": "none"
    },
    {
      "name": "patch",
      "type": "complex",
      "multiValued": false,
      "required": true,
      "caseExact": false,
      "mutability": "readOnly",
      "returned": "default",
      "uniqueness": "none",
      "subAttributes": [
        {
          "name": "supported",
          "type": "boolean",
          "multiValued": false,
          "required": true,
          "caseExact": false,
          "mutability": "readOnly",
          "returned": "default",
          "uniqueness": "none"
        }
      ]
    },
    {
      "name": "bulk",
      "type": "complex",
      "multiValued": false,
      "required": true,
      "caseExact": false,
      "mutability": "readOnly",
      "returned": "default",
      "uniqueness": "none",
      "subAttributes": [
        {
          "name": "supported",
          "type": "boolean",
          "multiValued": false,
          "required": true,
          "caseExact": false,
          "mutability": "readOnly",
          "returned": "default",
          "uniqueness": "none"
        },
        {
          "name": "maxOperations",
          "type": "integer",
          "multiValued": false,
          "required": true,
          "caseExact": false,
          "mutability": "readOnly",
          "returned": "default",
          "uniqueness": "none"
        },
        {
          "name": "maxPayloadSize",
          "type": "integer",
          "multiValued": false,
          "required": true,
          "caseExact": false,
          "mutability": "readOnly",
          "returned": "default",
          "uniqueness": "none"
        }
      ]
    },
    {
      "name": "filter",
      "type": "complex",
      "multiValued": false,
      "required": true,
      "caseExact": false,
      "mutability": "readOnly",
      "returned": "default",
      "uniqueness": "none",
      "subAttributes": [
        {
          "name": "supported",
          "type": "boolean",
          "multiValued": false,
          "required": true,
          "caseExact": false,
          "mutability": "readOnly",
          "returned": "default",
          "uniqueness": "none"
        },
        {
          "name": "maxResults",
          "type": "integer",
          "multiValued": false,
          "required": true,
          "caseExact": false,
          "mutability": "readOnly",
          "returned": "default",
          "uniqueness": "none"
        }
      ]
    },
    {
      "name": "changePassword",
      "type": "complex",
      "multiValued": false,
      "required": true,
      "caseExact": false,
      "mutability": "readOnly",
      "returned": "default",
      "uniqueness": "none",
      "subAttributes": [
        {
          "name": "supported",
          "type": "boolean",
          "multiValued": false,
          "required": true,
          "caseExact": false,
          "mutability": "readOnly",
          "returned": "default",
          "uniqueness": "none"
        }
      ]
    },
    {
      "name": "sort",
      "type": "complex",
      "multiValued": false,
      "required": true,
      "caseExact": false,
      "mutability": "readOnly",
      "returned": "default",
      "uniqueness": "none",
      "subAttributes": [
        {
          "name": "supported",
          "type": "boolean",
          "multiValued": false,
          "required": true,
          "caseExact": false,
          "mutability": "readOnly",
          "returned": "default",
          "uniqueness": "none"
        }
      ]
    },
    {
      "name": "etag",
      "type": "complex",
      "multiValued": false,
      "required": true,
      "caseExact": false,
      "mutability": "readOnly",
      "returned": "default",
      "uniqueness": "none",
      "subAttributes": [
        {
          "name": "supported",
          "type": "boolean",
          "multiValued": false,
          "required": true,
          "caseExact": false,
          "mutability": "readOnly",
          "returned": "default",
          "uniqueness": "none"
        }
      ]
    },
    {
      "name": "authenticationSchemes",
      "type": "complex",
      "multiValued": true,
      "required": true,
      "caseExact": false,
      "mutability": "readOnly",
      "returned": "default",
      "uniqueness": "none",
      "subAttributes": [
        {
          "name": "name",
          "type": "string",
          "multiValued": false,
          "required": true,
          "caseExact": false,
          "mutability": "readOnly",
          "returned": "default",
          "uniqueness": "none"
        },
        {
          "name": "description",
          "type": "string",
          "multiValued": false,
          "required": true,
          "caseExact": false,
          "mutability": "readOnly",
          "returned": "default",
          "uniqueness": "none"
        },
        {
          "name": "specUri",
          "type": "reference",
          "multiValued": false,
          "required": false,
          "caseExact": false,
          "mutability": "readOnly",
          "returned": "default",
          "uniqueness": "none"
        },
        {
          "name": "documentationUri",
          "type": "reference",
          "multiValued": false,
          "required": false,
          "caseExact": false,
          "mutability": "readOnly",
          "returned": "default",
          "uniqueness": "none"
        },
        {
          "name": "type",
          "type": "string",
          "multiValued": false,
          "required": true,
          "caseExact": false,
          "mutability": "readOnly",
          "returned": "default",
          "uniqueness": "none",
          "canonicalValues": ["oauth", "oauth2", "oauthbearertoken", "httpbasic", "httpdigest"]
        },
        {
          "name": "primary",
          "type": "boolean",
          "multiValued": false,
          "required": false,
          "caseExact": false,
          "mutability": "readOnly",
          "returned": "default",
          "uniqueness": "none"
        }
      ]
    }
  ]
}"#
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    #[test]
    fn test_core_user_schema_parses() {
        let schema_json = core_user_schema();
        let parsed: Result<Value, _> = serde_json::from_str(schema_json);
        assert!(parsed.is_ok(), "User schema should parse as valid JSON");

        let schema = parsed.unwrap();
        assert_eq!(schema["id"], "urn:ietf:params:scim:schemas:core:2.0:User");
        assert_eq!(schema["name"], "User");
    }

    #[test]
    fn test_core_group_schema_parses() {
        let schema_json = core_group_schema();
        let parsed: Result<Value, _> = serde_json::from_str(schema_json);
        assert!(parsed.is_ok(), "Group schema should parse as valid JSON");

        let schema = parsed.unwrap();
        assert_eq!(schema["id"], "urn:ietf:params:scim:schemas:core:2.0:Group");
        assert_eq!(schema["name"], "Group");
    }

    #[test]
    fn test_service_provider_config_schema_parses() {
        let schema_json = service_provider_config_schema();
        let parsed: Result<Value, _> = serde_json::from_str(schema_json);
        assert!(parsed.is_ok(), "ServiceProviderConfig schema should parse as valid JSON");

        let schema = parsed.unwrap();
        assert_eq!(schema["id"], "urn:ietf:params:scim:schemas:core:2.0:ServiceProviderConfig");
        assert_eq!(schema["name"], "ServiceProviderConfig");
    }
}
