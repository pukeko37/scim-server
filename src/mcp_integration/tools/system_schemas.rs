//! System tool schema definitions for MCP integration
//!
//! This module contains JSON schema definitions for system-level operations that provide
//! metadata about SCIM server capabilities, configuration, and available schemas.
//! These tools help AI agents understand the server environment before performing operations.
//!
//! # Architecture
//!
//! System tools provide:
//! - **Schema Discovery** - Available SCIM resource schemas and their structure
//! - **Capability Introspection** - Server features, versions, and operational limits
//! - **Configuration Metadata** - Multi-tenant support, resource types, and extensions
//!
//! # Available Tools
//!
//! - [`get_schemas_tool`] - Retrieve all SCIM schemas for data structure understanding
//! - [`get_server_info_tool`] - Get server capabilities, version, and operational metadata
//!
//! # Usage Pattern
//!
//! AI agents typically use these tools in discovery phase:
//! 1. Call `scim_server_info` to understand server capabilities
//! 2. Call `scim_get_schemas` to understand available data structures
//! 3. Proceed with resource operations based on discovered capabilities
//!
//! # No Parameters Required
//!
//! System tools generally require no input parameters as they provide
//! server-wide information that is not tenant or resource specific.

use serde_json::{Value, json};

/// Schema definition for SCIM schemas retrieval tool
pub fn get_schemas_tool() -> Value {
    json!({
        "name": "scim_get_schemas",
        "description": "Get all available SCIM schemas for AI agent understanding",
        "inputSchema": {
            "type": "object",
            "properties": {}
        }
    })
}

/// Schema definition for server information tool
pub fn get_server_info_tool() -> Value {
    json!({
        "name": "scim_server_info",
        "description": "Get SCIM server information and capabilities",
        "inputSchema": {
            "type": "object",
            "properties": {}
        }
    })
}
