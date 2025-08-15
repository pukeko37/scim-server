# Extensions

> **TODO**: This section is under development. Basic schema extension patterns are outlined below.

## Overview

SCIM schema extensions allow you to add custom attributes to standard resource types like User and Group. This enables organizations to capture additional data specific to their needs while maintaining SCIM compliance.

## Standard Extensions

### Enterprise User Extension

```rust
use scim_server::{Schema, Attribute, AttributeType};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnterpriseUser {
    #[serde(rename = "employeeNumber")]
    pub employee_number: Option<String>,
    
    #[serde(rename = "costCenter")]
    pub cost_center: Option<String>,
    
    pub organization: Option<String>,
    pub division: Option<String>,
    pub department: Option<String>,
    pub manager: Option<Manager>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Manager {
    pub value: String,           // Manager's user ID
    #[serde(rename = "$ref")]
    pub reference: Option<String>, // URI reference to manager
    #[serde(rename = "displayName")]
    pub display_name: Option<String>,
}
```

## Custom Extensions

### Creating Custom Schema Extensions

```rust
use scim_server::schema::{SchemaBuilder, AttributeBuilder};

pub fn create_custom_extension() -> Schema {
    SchemaBuilder::new()
        .id("urn:company:scim:schemas:extension:employee:2.0")
        .name("Employee Extension")
        .description("Custom attributes for employee data")
        .add_attribute(
            AttributeBuilder::new()
                .name("badgeNumber")
                .type_(AttributeType::String)
                .description("Employee badge number")
                .required(false)
                .case_exact(true)
                .build()
        )
        .add_attribute(
            AttributeBuilder::new()
                .name("startDate")
                .type_(AttributeType::DateTime)
                .description("Employee start date")
                .required(false)
                .build()
        )
        .add_attribute(
            AttributeBuilder::new()
                .name("clearanceLevel")
                .type_(AttributeType::String)
                .description("Security clearance level")
                .required(false)
                .canonical_values(vec!["PUBLIC", "CONFIDENTIAL", "SECRET", "TOP_SECRET"])
                .build()
        )
        .build()
}
```

## Usage in Resources

### Extended User Resource

```rust
use serde_json::json;

let extended_user = json!({
    "schemas": [
        "urn:ietf:params:scim:schemas:core:2.0:User",
        "urn:ietf:params:scim:schemas:extension:enterprise:2.0:User",
        "urn:company:scim:schemas:extension:employee:2.0"
    ],
    "userName": "john.doe@company.com",
    "name": {
        "givenName": "John",
        "familyName": "Doe"
    },
    "urn:ietf:params:scim:schemas:extension:enterprise:2.0:User": {
        "employeeNumber": "12345",
        "department": "Engineering",
        "manager": {
            "value": "manager-uuid",
            "displayName": "Jane Smith"
        }
    },
    "urn:company:scim:schemas:extension:employee:2.0": {
        "badgeNumber": "BADGE-12345",
        "startDate": "2024-01-15T00:00:00Z",
        "clearanceLevel": "CONFIDENTIAL"
    }
});
```

## Validation

### Extension Validation

> **TODO**: Implement comprehensive extension validation patterns.

### Schema Registration

```rust
use scim_server::SchemaRegistry;

fn register_extensions(registry: &mut SchemaRegistry) {
    // Register enterprise extension
    registry.register_extension(
        "urn:ietf:params:scim:schemas:extension:enterprise:2.0:User",
        create_enterprise_extension()
    );
    
    // Register custom extension
    registry.register_extension(
        "urn:company:scim:schemas:extension:employee:2.0",
        create_custom_extension()
    );
}
```

## Best Practices

1. **Use descriptive URNs** for extension schemas
2. **Follow naming conventions** (camelCase for attributes)
3. **Document all extensions** thoroughly
4. **Version your extensions** appropriately
5. **Test extension compatibility** with SCIM clients

> **TODO**: Add more advanced extension patterns and examples.
