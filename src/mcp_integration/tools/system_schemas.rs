//! System tool schema definitions for MCP integration
//!
//! This module contains the JSON schema definitions for system-related MCP tools.
//! These schemas enable AI agents to discover and understand server metadata
//! and capability operations.

use serde_json::{Value, json};

/// Schema definition for schemas retrieval tool
///
/// Defines the tool for fetching all available SCIM schemas that the server supports.
/// This helps AI agents understand the data structures they can work with.
pub fn get_schemas_tool() -> Value {
    json!({
        "name": "scim_get_schemas",
        "description": "Get all available SCIM schemas for AI agent understanding",
        "input_schema": {
            "type": "object",
            "properties": {}
        }
    })
}

/// Schema definition for server information tool
///
/// Defines the tool for fetching server capabilities, version, and metadata.
/// This helps AI agents understand what the server can do and how to interact with it.
pub fn get_server_info_tool() -> Value {
    json!({
        "name": "scim_server_info",
        "description": "Get SCIM server information and capabilities",
        "input_schema": {
            "type": "object",
            "properties": {}
        }
    })
}
