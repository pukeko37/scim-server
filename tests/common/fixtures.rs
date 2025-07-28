//! Test fixtures for loading and managing test data files.
//!
//! This module provides utilities for loading test schemas and resources
//! from JSON files, with caching for performance.

use serde_json::Value;
use std::collections::HashMap;
use std::sync::OnceLock;

/// Cache for loaded fixtures to avoid repeated file I/O
static FIXTURE_CACHE: OnceLock<HashMap<String, Value>> = OnceLock::new();

/// Load a fixture from the fixtures directory with caching
pub fn load_fixture(path: &str) -> Value {
    let cache = FIXTURE_CACHE.get_or_init(|| HashMap::new());

    // Check cache first
    if let Some(cached) = cache.get(path) {
        return cached.clone();
    }

    // Load from file if not cached
    let fixture_path = format!("tests/fixtures/{}", path);
    let content = std::fs::read_to_string(&fixture_path)
        .unwrap_or_else(|_| panic!("Failed to load fixture: {}", fixture_path));

    serde_json::from_str(&content)
        .unwrap_or_else(|_| panic!("Failed to parse JSON fixture: {}", fixture_path))
}

/// Load a schema fixture
pub fn load_schema(name: &str) -> Value {
    load_fixture(&format!("schemas/{}.json", name))
}

/// Load a valid resource fixture
pub fn load_valid_resource(name: &str) -> Value {
    load_fixture(&format!("resources/valid/{}.json", name))
}

/// Load an invalid resource fixture
pub fn load_invalid_resource(name: &str) -> Value {
    load_fixture(&format!("resources/invalid/{}.json", name))
}

/// RFC 7643 examples as constants for easy reference
pub mod rfc_examples {
    use serde_json::{Value, json};

    /// RFC 7643 Section 8.1 - Minimal User representation
    pub fn user_minimal() -> Value {
        json!({
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
        })
    }

    /// RFC 7643 Section 8.2 - Full User representation
    pub fn user_full() -> Value {
        json!({
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
            "nickName": "Babs",
            "profileUrl": "https://login.example.com/bjensen",
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
            "addresses": [
                {
                    "type": "work",
                    "streetAddress": "100 Universal City Plaza",
                    "locality": "Hollywood",
                    "region": "CA",
                    "postalCode": "91608",
                    "country": "USA",
                    "formatted": "100 Universal City Plaza\nHollywood, CA 91608 USA",
                    "primary": true
                },
                {
                    "type": "home",
                    "streetAddress": "456 Hollywood Blvd",
                    "locality": "Hollywood",
                    "region": "CA",
                    "postalCode": "91608",
                    "country": "USA",
                    "formatted": "456 Hollywood Blvd\nHollywood, CA 91608 USA"
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
            "ims": [
                {
                    "value": "someaimhandle",
                    "type": "aim"
                }
            ],
            "photos": [
                {
                    "value": "https://photos.example.com/profilephoto/72930000000Ccne/F",
                    "type": "photo"
                },
                {
                    "value": "https://photos.example.com/profilephoto/72930000000Ccne/T",
                    "type": "thumbnail"
                }
            ],
            "userType": "Employee",
            "title": "Tour Guide",
            "preferredLanguage": "en-US",
            "locale": "en-US",
            "timezone": "America/Los_Angeles",
            "active": true,
            "password": "t1meMa$heen",
            "groups": [
                {
                    "value": "e9e30dba-f08f-4109-8486-d5c6a331660a",
                    "$ref": "https://example.com/v2/Groups/e9e30dba-f08f-4109-8486-d5c6a331660a",
                    "display": "Tour Guides"
                },
                {
                    "value": "fc348aa8-3835-40eb-a20b-c726e15c55b5",
                    "$ref": "https://example.com/v2/Groups/fc348aa8-3835-40eb-a20b-c726e15c55b5",
                    "display": "Employees"
                },
                {
                    "value": "71ddacd2-a8e7-49b8-a5db-ae50d0a5bfd7",
                    "$ref": "https://example.com/v2/Groups/71ddacd2-a8e7-49b8-a5db-ae50d0a5bfd7",
                    "display": "US Employees"
                }
            ],
            "x509Certificates": [
                {
                    "value": "MIIDQzCCAqygAwIBAgICEAAwDQYJKoZIhvcNAQEFBQAwTjELMAkGA1UEBhMCVVMxEzARBgNVBAgMCkNhbGlmb3JuaWExFDASBgNVBAoMC2V4YW1wbGUuY29tMRQwEgYDVQQDDAtleGFtcGxlLmNvbTAeFw0xMTEwMjIwNjI0MzFaFw0xMjEwMDQwNjI0MzFaMH8xCzAJBgNVBAYTAlVTMRMwEQYDVQQIDApDYWxpZm9ybmlhMRQwEgYDVQQKDAtleGFtcGxlLmNvbTEhMB8GA1UEAwwYTXMuIEJhcmJhcmEgSiBKZW5zZW4gSUlJMSIwIAYJKoZIhvcNAQkBFhNiamVuc2VuQGV4YW1wbGUuY29tMIIBIjANBgkqhkiG9w0BAQEFAAOCAQ8AMIIBCgKCAQEA7Kr+Dcds/JQ5GwejJFcBIP682X3xpjis56AK02bc1FLgzdLI8auoR+cC9/Vrh5t66HkQIOdA4unHh0AaZ4xL5PhVbXIPMB5vAPKpzz5iPSi8xO8SL7I7SDhcBVJhqVqr3HgllEG6UClDdHO7nkLuwXq8HcISKkbT5WFTVfFZzidPl8HZ7DhXkZIRtJwBweq4bvm3hM1Os7UQH05ZS6cVDgweKNwdLLrT51ikSQG3DYrl+ft781UQRIqxgwqCfXEuDiinPh0kkvIi5jivVu1Z9QiwlYEdRbLJ4zJQBmDrSGTMYn4lRc2HgHO4DqB/bnMVorHB0CC6AV1QoFK4GPe1LwIDAQABo3sweTAJBgNVHRMEAjAAMCwGCWCGSAGG+EIBDQQfFh1PcGVuU1NMIEdlbmVyYXRlZCBDZXJ0aWZpY2F0ZTAdBgNVHQ4EFgQU8pD0U0vsZIsaA16lL8En8bx0F/gwHwYDVR0jBBgwFoAUdGeKitcaF7gnzsNwDx708kqaVt0wDQYJKoZIhvcNAQEFBQADgYEAA81SsFnOdYJtNg5Tcq+/ByEDrBgnusx0jloUhByPMEVkoMZ3J7j1ZgI8rAbOkNngX8+pKfTiDz1RC4+dx8oU6Za+4NJXUjlL5CvV6BEYb1+QAEJwitTVvxB/A67g42/vzgAtoRUeDov1+GFiBZ+GNF/cAYKcMtGcrs2i97ZkJMo="
                }
            ],
            "meta": {
                "resourceType": "User",
                "created": "2010-01-23T04:56:22Z",
                "lastModified": "2011-05-13T04:42:34Z",
                "version": "W/\"a330bc54f0671c9\"",
                "location": "https://example.com/v2/Users/2819c223-7f76-453a-919d-413861904646"
            }
        })
    }

    /// RFC 7643 Section 8.3 - Enterprise User extension representation
    pub fn user_enterprise() -> Value {
        let mut user = user_full();

        // Add enterprise extension to schemas
        user["schemas"] = json!([
            "urn:ietf:params:scim:schemas:core:2.0:User",
            "urn:ietf:params:scim:schemas:extension:enterprise:2.0:User"
        ]);

        // Add enterprise extension attributes
        user["urn:ietf:params:scim:schemas:extension:enterprise:2.0:User"] = json!({
            "employeeNumber": "701984",
            "costCenter": "4130",
            "organization": "Universal Studios",
            "division": "Theme Park",
            "department": "Tour Operations",
            "manager": {
                "value": "26118915-6090-4610-87e4-49d8ca9f808d",
                "$ref": "../Users/26118915-6090-4610-87e4-49d8ca9f808d",
                "displayName": "John Smith"
            }
        });

        user
    }

    /// RFC 7643 Section 8.4 - Group representation
    pub fn group_basic() -> Value {
        json!({
            "schemas": ["urn:ietf:params:scim:schemas:core:2.0:Group"],
            "id": "e9e30dba-f08f-4109-8486-d5c6a331660a",
            "displayName": "Tour Guides",
            "members": [
                {
                    "value": "2819c223-7f76-453a-919d-413861904646",
                    "$ref": "https://example.com/v2/Users/2819c223-7f76-453a-919d-413861904646",
                    "display": "Babs Jensen"
                },
                {
                    "value": "902c246b-6245-4190-8e05-00816be7344a",
                    "$ref": "https://example.com/v2/Users/902c246b-6245-4190-8e05-00816be7344a",
                    "display": "Mandy Pepperidge"
                }
            ],
            "meta": {
                "resourceType": "Group",
                "created": "2010-01-23T04:56:22Z",
                "lastModified": "2011-05-13T04:42:34Z",
                "version": "W/\"3694e05e9dff592\"",
                "location": "https://example.com/v2/Groups/e9e30dba-f08f-4109-8486-d5c6a331660a"
            }
        })
    }
}

/// Test-specific fixtures for systematic validation testing
pub mod test_fixtures {
    use serde_json::{Value, json};

    /// Basic test extension schema for testing extension validation
    pub fn test_extension_schema() -> Value {
        json!({
            "id": "urn:example:params:scim:schemas:extension:test:2.0:User",
            "name": "TestExtension",
            "description": "Test extension for validation testing",
            "attributes": [
                {
                    "name": "testString",
                    "type": "string",
                    "multiValued": false,
                    "required": false,
                    "caseExact": false,
                    "mutability": "readWrite",
                    "returned": "default",
                    "uniqueness": "none"
                },
                {
                    "name": "testRequired",
                    "type": "string",
                    "multiValued": false,
                    "required": true,
                    "caseExact": false,
                    "mutability": "readWrite",
                    "returned": "default",
                    "uniqueness": "none"
                },
                {
                    "name": "testReadOnly",
                    "type": "string",
                    "multiValued": false,
                    "required": false,
                    "caseExact": false,
                    "mutability": "readOnly",
                    "returned": "default",
                    "uniqueness": "none"
                }
            ]
        })
    }

    /// Intentionally malformed schema for testing schema validation
    pub fn invalid_schema() -> Value {
        json!({
            "id": "not-a-valid-uri",
            "name": 123, // Should be string
            "attributes": "not-an-array" // Should be array
        })
    }

    /// Minimal valid schemas for testing
    pub fn minimal_user_schema() -> Value {
        json!({
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
        })
    }

    pub fn minimal_group_schema() -> Value {
        json!({
            "id": "urn:ietf:params:scim:schemas:core:2.0:Group",
            "name": "Group",
            "description": "Group",
            "attributes": [
                {
                    "name": "displayName",
                    "type": "string",
                    "multiValued": false,
                    "required": true,
                    "caseExact": false,
                    "mutability": "readWrite",
                    "returned": "default",
                    "uniqueness": "none"
                }
            ]
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rfc_examples_user_minimal() {
        let user = rfc_examples::user_minimal();
        assert_eq!(
            user["schemas"][0],
            "urn:ietf:params:scim:schemas:core:2.0:User"
        );
        assert_eq!(user["userName"], "bjensen@example.com");
        assert!(!user["id"].as_str().unwrap().is_empty());
    }

    #[test]
    fn test_rfc_examples_user_full() {
        let user = rfc_examples::user_full();
        assert_eq!(
            user["schemas"][0],
            "urn:ietf:params:scim:schemas:core:2.0:User"
        );
        assert_eq!(user["displayName"], "Babs Jensen");
        assert!(user["emails"].is_array());
        assert_eq!(user["emails"][0]["primary"], true);
    }

    #[test]
    fn test_rfc_examples_user_enterprise() {
        let user = rfc_examples::user_enterprise();
        assert_eq!(user["schemas"].as_array().unwrap().len(), 2);
        assert!(
            user["schemas"]
                .as_array()
                .unwrap()
                .contains(&serde_json::json!(
                    "urn:ietf:params:scim:schemas:extension:enterprise:2.0:User"
                ))
        );

        let enterprise_ext = &user["urn:ietf:params:scim:schemas:extension:enterprise:2.0:User"];
        assert_eq!(enterprise_ext["employeeNumber"], "701984");
        assert_eq!(enterprise_ext["department"], "Tour Operations");
    }

    #[test]
    fn test_rfc_examples_group_basic() {
        let group = rfc_examples::group_basic();
        assert_eq!(
            group["schemas"][0],
            "urn:ietf:params:scim:schemas:core:2.0:Group"
        );
        assert_eq!(group["displayName"], "Tour Guides");
        assert!(group["members"].is_array());
        assert_eq!(group["members"].as_array().unwrap().len(), 2);
    }

    #[test]
    fn test_test_fixtures_schemas() {
        let schema = test_fixtures::minimal_user_schema();
        assert_eq!(schema["id"], "urn:ietf:params:scim:schemas:core:2.0:User");
        assert_eq!(schema["name"], "User");

        let attrs = schema["attributes"].as_array().unwrap();
        assert_eq!(attrs.len(), 1);
        assert_eq!(attrs[0]["name"], "userName");
        assert_eq!(attrs[0]["required"], true);
    }

    #[test]
    fn test_invalid_schema_fixture() {
        let schema = test_fixtures::invalid_schema();
        assert_eq!(schema["id"], "not-a-valid-uri");
        assert_eq!(schema["name"], 123);
        assert_eq!(schema["attributes"], "not-an-array");
    }
}
