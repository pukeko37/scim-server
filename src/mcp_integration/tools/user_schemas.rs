//! User tool schema definitions for MCP integration
//!
//! This module contains the JSON schema definitions for all user-related MCP tools.
//! These schemas enable AI agents to discover and understand the available user
//! operations and their required parameters.

use serde_json::{Value, json};

/// Schema definition for user creation tool
///
/// Defines the parameters and validation rules for creating new users.
pub fn create_user_tool() -> Value {
    json!({
        "name": "scim_create_user",
        "description": "Create a new user in the SCIM server",
        "input_schema": {
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
///
/// Defines the parameters for fetching a user by ID.
pub fn get_user_tool() -> Value {
    json!({
        "name": "scim_get_user",
        "description": "Retrieve a user by ID from the SCIM server",
        "input_schema": {
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
///
/// Defines the parameters for updating existing users with optional ETag support.
pub fn update_user_tool() -> Value {
    json!({
        "name": "scim_update_user",
        "description": "Update an existing user in the SCIM server with optional ETag versioning for optimistic locking",
        "input_schema": {
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
                    "description": "Optional ETag version for conditional update (e.g., 'W/\"abc123\"'). If provided, update only succeeds if current version matches. Prevents lost updates in concurrent scenarios."
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
///
/// Defines the parameters for deleting users with optional ETag support.
pub fn delete_user_tool() -> Value {
    json!({
        "name": "scim_delete_user",
        "description": "Delete a user from the SCIM server with optional ETag versioning for safe deletion",
        "input_schema": {
            "type": "object",
            "properties": {
                "user_id": {
                    "type": "string",
                    "description": "The unique identifier of the user to delete"
                },
                "expected_version": {
                    "type": "string",
                    "description": "Optional ETag version for conditional delete (e.g., 'W/\"abc123\"'). If provided, delete only succeeds if current version matches. Prevents accidental deletion of modified resources."
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
///
/// Defines the parameters for listing users with pagination and filtering.
pub fn list_users_tool() -> Value {
    json!({
        "name": "scim_list_users",
        "description": "List users with optional pagination and sorting",
        "input_schema": {
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
///
/// Defines the parameters for searching users by attributes.
pub fn search_users_tool() -> Value {
    json!({
        "name": "scim_search_users",
        "description": "Search for users by attribute value",
        "input_schema": {
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
///
/// Defines the parameters for checking if a user exists.
pub fn user_exists_tool() -> Value {
    json!({
        "name": "scim_user_exists",
        "description": "Check if a user exists in the SCIM server",
        "input_schema": {
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
