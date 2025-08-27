//! User tool schema definitions for MCP integration
//!
//! This module contains JSON schema definitions that enable AI agents to discover
//! and understand available user operations. The schemas define parameter validation
//! and provide structured metadata for tool execution.
//!
//! # Architecture
//!
//! Each user tool schema includes:
//! - **Tool name and description** for AI agent discovery
//! - **Input parameter validation** using JSON Schema format
//! - **Required vs optional parameters** clearly defined
//! - **Multi-tenant support** through optional tenant_id parameter
//! - **SCIM compliance** ensuring all operations follow SCIM 2.0 standards
//!
//! # Tool Categories
//!
//! **CRUD Operations**:
//! - [`create_user_tool`] - User creation with schema validation
//! - [`get_user_tool`] - User retrieval by ID
//! - [`update_user_tool`] - User modification with version support
//! - [`delete_user_tool`] - User deletion with conditional operation
//!
//! **Query Operations**:
//! - [`list_users_tool`] - Paginated user listing
//! - [`search_users_tool`] - Attribute-based user search
//! - [`user_exists_tool`] - User existence checking
//!
//! # Usage
//!
//! These schemas are consumed by the MCP protocol layer to provide tool discovery
//! to AI agents. They are not intended for direct use by application developers.
//! The schemas are automatically registered and exposed when the MCP server starts.
//!
//! # Version Support
//!
//! Many operations support version-based optimistic concurrency control through
//! the optional `expected_version` parameter, helping prevent lost updates in
//! concurrent scenarios.

use serde_json::{Value, json};

/// Schema definition for user creation tool
pub fn create_user_tool() -> Value {
    json!({
        "name": "scim_create_user",
        "description": "Create a new user in the SCIM server",
        "inputSchema": {
            "type": "object",
            "properties": {
                "user_data": {
                    "type": "object",
                    "description": "User data conforming to SCIM User schema",
                    "properties": {
                        "schemas": {
                            "type": "array",
                            "items": {"type": "string"},
                            "description": "SCIM schemas for the user"
                        },
                        "userName": {
                            "type": "string",
                            "description": "Unique identifier for the user"
                        },
                        "name": {
                            "type": "object",
                            "description": "User's name components"
                        },
                        "emails": {
                            "type": "array",
                            "description": "User's email addresses"
                        },
                        "active": {
                            "type": "boolean",
                            "description": "Whether the user is active"
                        }
                    },
                    "required": ["schemas", "userName"]
                },
                "tenant_id": {
                    "type": "string",
                    "description": "Optional tenant identifier"
                }
            },
            "required": ["user_data"]
        }
    })
}

/// Schema definition for user retrieval tool
pub fn get_user_tool() -> Value {
    json!({
        "name": "scim_get_user",
        "description": "Retrieve a user by ID from the SCIM server",
        "inputSchema": {
            "type": "object",
            "properties": {
                "user_id": {
                    "type": "string",
                    "description": "The unique identifier of the user to retrieve"
                },
                "tenant_id": {
                    "type": "string",
                    "description": "Optional tenant identifier"
                }
            },
            "required": ["user_id"]
        }
    })
}

/// Schema definition for user update tool
pub fn update_user_tool() -> Value {
    json!({
        "name": "scim_update_user",
        "description": "Update an existing user in the SCIM server with optional versioning for optimistic locking",
        "inputSchema": {
            "type": "object",
            "properties": {
                "user_id": {
                    "type": "string",
                    "description": "The unique identifier of the user to update"
                },
                "user_data": {
                    "type": "object",
                    "description": "Updated user data conforming to SCIM User schema"
                },
                "expected_version": {
                    "type": "string",
                    "description": "Optional version for conditional update (e.g., 'abc123def'). If provided, update only succeeds if current version matches. Prevents lost updates in concurrent scenarios."
                },
                "tenant_id": {
                    "type": "string",
                    "description": "Optional tenant identifier"
                }
            },
            "required": ["user_id", "user_data"]
        }
    })
}

/// Schema definition for user deletion tool
pub fn delete_user_tool() -> Value {
    json!({
        "name": "scim_delete_user",
        "description": "Delete a user from the SCIM server with optional versioning for safe deletion",
        "inputSchema": {
            "type": "object",
            "properties": {
                "user_id": {
                    "type": "string",
                    "description": "The unique identifier of the user to delete"
                },
                "expected_version": {
                    "type": "string",
                    "description": "Optional version for conditional delete (e.g., 'abc123def'). If provided, delete only succeeds if current version matches. Prevents accidental deletion of modified resources."
                },
                "tenant_id": {
                    "type": "string",
                    "description": "Optional tenant identifier"
                }
            },
            "required": ["user_id"]
        }
    })
}

/// Schema definition for user listing tool
pub fn list_users_tool() -> Value {
    json!({
        "name": "scim_list_users",
        "description": "List users with optional pagination and sorting",
        "inputSchema": {
            "type": "object",
            "properties": {
                "start_index": {
                    "type": "integer",
                    "minimum": 1,
                    "description": "1-based index of the first result to return"
                },
                "count": {
                    "type": "integer",
                    "minimum": 0,
                    "description": "Maximum number of results to return"
                },
                "tenant_id": {
                    "type": "string",
                    "description": "Optional tenant identifier"
                }
            }
        }
    })
}

/// Schema definition for user search tool
pub fn search_users_tool() -> Value {
    json!({
        "name": "scim_search_users",
        "description": "Search for users by attribute value",
        "inputSchema": {
            "type": "object",
            "properties": {
                "attribute": {
                    "type": "string",
                    "description": "The attribute name to search by (e.g., 'userName', 'email')"
                },
                "value": {
                    "description": "The value to search for"
                },
                "tenant_id": {
                    "type": "string",
                    "description": "Optional tenant identifier"
                }
            },
            "required": ["attribute", "value"]
        }
    })
}

/// Schema definition for user existence check tool
pub fn user_exists_tool() -> Value {
    json!({
        "name": "scim_user_exists",
        "description": "Check if a user exists in the SCIM server",
        "inputSchema": {
            "type": "object",
            "properties": {
                "user_id": {
                    "type": "string",
                    "description": "The unique identifier of the user to check"
                },
                "tenant_id": {
                    "type": "string",
                    "description": "Optional tenant identifier"
                }
            },
            "required": ["user_id"]
        }
    })
}
