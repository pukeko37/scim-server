//! MCP tool schema definitions
//!
//! This module contains all the JSON schema definitions for MCP tools that
//! AI agents can discover and execute. The schemas provide structured metadata
//! that enables automatic tool discovery and parameter validation.
//!
//! # Architecture
//!
//! Tool schemas are organized by functional area:
//! - [`user_schemas`] - User lifecycle and query operations
//! - [`group_schemas`] - Group lifecycle and query operations
//! - [`system_schemas`] - Server introspection and metadata operations
//!
//! Each schema defines:
//! - Tool name for AI agent discovery
//! - Human-readable description of functionality
//! - JSON Schema validation for input parameters
//! - Multi-tenant support configuration
//!
//! # Usage
//!
//! These schemas are consumed by the MCP protocol layer and are not intended
//! for direct use by application developers. They are automatically registered
//! when the MCP server initializes and provide the foundation for AI agent
//! tool discovery and execution.

pub mod group_schemas;
pub mod system_schemas;
pub mod user_schemas;

// Re-export commonly used schema functions for convenience
pub use group_schemas::*;
pub use system_schemas::*;
pub use user_schemas::*;
