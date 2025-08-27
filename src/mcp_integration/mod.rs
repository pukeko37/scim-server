//! MCP (Model Context Protocol) Integration for SCIM Server
//!
//! This module provides comprehensive MCP integration that exposes SCIM operations
//! as structured tools for AI agents. The integration enables AI systems to perform
//! identity management operations through a standardized protocol interface.
//!
//! ## Overview
//!
//! The MCP integration transforms SCIM server operations into discoverable tools
//! that AI agents can understand and execute. This enables:
//!
//! - **Automated Identity Management**: AI agents can provision/deprovision users
//! - **Schema-Driven Operations**: AI agents understand SCIM data structures
//! - **Multi-Tenant Support**: Tenant-aware operations for enterprise scenarios
//! - **Version-Based Concurrency Control**: Built-in optimistic locking prevents lost updates
//! - **Error Handling**: Structured error responses for AI decision making
//! - **Real-time Operations**: Async operations suitable for AI workflows
//!
//! ## Architecture
//!
//! ```text
//! ┌─────────────────┐    ┌──────────────────┐    ┌─────────────────┐
//! │   AI Agent      │───▶│  MCP Protocol    │───▶│  SCIM Server    │
//! │   (Client)      │    │  (This Module)   │    │  (Operations)   │
//! └─────────────────┘    └──────────────────┘    └─────────────────┘
//!          │                        │                       │
//!          ▼                        ▼                       ▼
//!    Tool Discovery          Tool Execution        Resource Management
//!    Schema Learning         JSON Validation        Provider Integration
//!    Error Handling          Tenant Context        Multi-Tenant Isolation
//! ```
//!
//! ## Module Structure
//!
//! - `core` - Core types and infrastructure (McpServerInfo, ScimToolResult, ScimMcpServer)
//! - `protocol` - Tool discovery and dispatch functionality
//! - `tools/` - JSON schema definitions for MCP tool discovery
//!   - `user_schemas` - User operation tool schemas
//!   - `system_schemas` - System information tool schemas
//! - `handlers/` - Tool execution handlers
//!   - `user_crud` - User CRUD operation handlers
//!   - `user_queries` - User query and search handlers
//!   - `system_info` - System metadata handlers
//!
//! ## Usage Example
//!
//! ```rust,no_run
//! # #[cfg(feature = "mcp")]
//! use scim_server::{ScimServer, mcp_integration::ScimMcpServer, providers::StandardResourceProvider};
//! use scim_server::storage::InMemoryStorage;
//! use serde_json::json;
//!
//! # #[cfg(feature = "mcp")]
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // Create SCIM server
//!     let storage = InMemoryStorage::new();
//!     let provider = StandardResourceProvider::new(storage);
//!     let scim_server = ScimServer::new(provider)?;
//!
//!     // Create MCP server
//!     let mcp_server = ScimMcpServer::new(scim_server);
//!
//!     // Execute tool (simulating AI agent)
//!     let result = mcp_server.execute_tool(
//!         "scim_create_user",
//!         json!({
//!             "user_data": {
//!                 "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
//!                 "userName": "ai.agent@company.com",
//!                 "active": true
//!             }
//!         })
//!     ).await;
//!
//!     if result.success {
//!         println!("User created successfully");
//!     }
//!     Ok(())
//! }
//! ```

#[cfg(feature = "mcp")]
pub mod core;
#[cfg(feature = "mcp")]
pub mod handlers;
#[cfg(feature = "mcp")]
pub mod protocol;
#[cfg(feature = "mcp")]
pub mod tools;

#[cfg(all(feature = "mcp", test))]
mod tests;

// Re-export core types for convenience
#[cfg(feature = "mcp")]
pub use core::{McpServerInfo, ScimMcpServer, ScimToolResult};

// Protocol functions are accessed through ScimMcpServer methods
// No need to re-export protocol internals
