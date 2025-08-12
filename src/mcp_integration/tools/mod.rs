//! MCP tool schema definitions
//!
//! This module contains all the JSON schema definitions for MCP tools that
//! AI agents can discover and execute. The schemas are organized by functional
//! area to maintain clear separation of concerns.

pub mod system_schemas;
pub mod user_schemas;

// Re-export commonly used schema functions for convenience
pub use system_schemas::*;
pub use user_schemas::*;
