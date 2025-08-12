//! MCP integration handlers
//!
//! This module contains all the handler implementations for MCP tool execution.
//! Handlers are organized by functional area to maintain clear separation of
//! concerns and enable focused testing and maintenance.

pub mod system_info;
pub mod user_crud;
pub mod user_queries;

// Re-export handler functions for convenience
pub use system_info::*;
pub use user_crud::*;
pub use user_queries::*;
