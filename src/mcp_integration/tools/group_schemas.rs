//! Group tool schema definitions for MCP integration
//!
//! This module contains JSON schema definitions that enable AI agents to discover
//! and understand available group operations. The schemas define parameter validation
//! and provide structured metadata for tool execution.
//!
//! # Architecture
//!
//! Each group tool schema includes:
//! - **Tool name and description** for AI agent discovery
//! - **Input parameter validation** using JSON Schema format
//! - **Required vs optional parameters** clearly defined
//! - **Multi-tenant support** through optional tenant_id parameter
//! - **SCIM compliance** ensuring all operations follow SCIM 2.0 standards
//!
//! # Tool Categories
//!
//! **CRUD Operations**:
//! - [`create_group_tool`] - Group creation with schema validation
//! - [`get_group_tool`] - Group retrieval by ID
//! - [`update_group_tool`] - Group modification with version support
//! - [`delete_group_tool`] - Group deletion with conditional operation
//!
//! **Query Operations**:
//! - [`list_groups_tool`] - Paginated group listing
//! - [`search_groups_tool`] - Attribute-based group search
//! - [`group_exists_tool`] - Group existence checking
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

/// Schema definition for group creation tool
pub fn create_group_tool() -> Value {
    json!({
        "name": "scim_create_group",
        "description": "Create a new group in the SCIM server",
        "inputSchema": {
            "type": "object",
            "properties": {
                "group_data": {
                    "type": "object",
                    "description": "Group data conforming to SCIM Group schema",
                    "properties": {
                        "schemas": {
                            "type": "array",
                            "items": {"type": "string"},
                            "description": "SCIM schemas for the group"
                        },
                        "displayName": {
                            "type": "string",
                            "description": "Human-readable name for the group"
                        },
                        "members": {
                            "type": "array",
                            "description": "Group members with user references",
                            "items": {
                                "type": "object",
                                "properties": {
                                    "value": {"type": "string"},
                                    "$ref": {"type": "string"},
                                    "type": {"type": "string"}
                                }
                            }
                        },
                        "externalId": {
                            "type": "string",
                            "description": "External identifier for the group"
                        }
                    },
                    "required": ["schemas", "displayName"]
                },
                "tenant_id": {
                    "type": "string",
                    "description": "Optional tenant identifier"
                }
            },
            "required": ["group_data"]
        }
    })
}

/// Schema definition for group retrieval tool
pub fn get_group_tool() -> Value {
    json!({
        "name": "scim_get_group",
        "description": "Retrieve a group by ID from the SCIM server",
        "inputSchema": {
            "type": "object",
            "properties": {
                "group_id": {
                    "type": "string",
                    "description": "The unique identifier of the group to retrieve"
                },
                "tenant_id": {
                    "type": "string",
                    "description": "Optional tenant identifier"
                }
            },
            "required": ["group_id"]
        }
    })
}

/// Schema definition for group update tool
pub fn update_group_tool() -> Value {
    json!({
        "name": "scim_update_group",
        "description": "Update an existing group in the SCIM server with optional versioning for optimistic locking",
        "inputSchema": {
            "type": "object",
            "properties": {
                "group_id": {
                    "type": "string",
                    "description": "The unique identifier of the group to update"
                },
                "group_data": {
                    "type": "object",
                    "description": "Updated group data conforming to SCIM Group schema"
                },
                "expected_version": {
                    "type": "string",
                    "description": "Optional version for conditional update (e.g., 'abc123def' or 'W/\"abc123def\"'). Raw format preferred for simplicity. If provided, update only succeeds if current version matches. Prevents lost updates in concurrent scenarios."
                },
                "tenant_id": {
                    "type": "string",
                    "description": "Optional tenant identifier"
                }
            },
            "required": ["group_id", "group_data"]
        }
    })
}

/// Schema definition for group deletion tool
pub fn delete_group_tool() -> Value {
    json!({
        "name": "scim_delete_group",
        "description": "Delete a group from the SCIM server with optional versioning for safe deletion",
        "inputSchema": {
            "type": "object",
            "properties": {
                "group_id": {
                    "type": "string",
                    "description": "The unique identifier of the group to delete"
                },
                "expected_version": {
                    "type": "string",
                    "description": "Optional version for conditional delete (e.g., 'abc123def' or 'W/\"abc123def\"'). Raw format preferred for simplicity. If provided, delete only succeeds if current version matches. Prevents accidental deletion of modified resources."
                },
                "tenant_id": {
                    "type": "string",
                    "description": "Optional tenant identifier"
                }
            },
            "required": ["group_id"]
        }
    })
}

/// Schema definition for group listing tool
pub fn list_groups_tool() -> Value {
    json!({
        "name": "scim_list_groups",
        "description": "List groups with optional pagination and sorting",
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

/// Schema definition for group search tool
pub fn search_groups_tool() -> Value {
    json!({
        "name": "scim_search_groups",
        "description": "Search for groups by attribute value",
        "inputSchema": {
            "type": "object",
            "properties": {
                "attribute": {
                    "type": "string",
                    "description": "The attribute name to search by (e.g., 'displayName', 'externalId')"
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

/// Schema definition for group existence check tool
pub fn group_exists_tool() -> Value {
    json!({
        "name": "scim_group_exists",
        "description": "Check if a group exists in the SCIM server",
        "inputSchema": {
            "type": "object",
            "properties": {
                "group_id": {
                    "type": "string",
                    "description": "The unique identifier of the group to check"
                },
                "tenant_id": {
                    "type": "string",
                    "description": "Optional tenant identifier"
                }
            },
            "required": ["group_id"]
        }
    })
}
