//! Schema definitions and validation for SCIM resources.
//!
//! This module provides the schema registry and validation engine for SCIM resources,
//! implementing the core User schema as defined in RFC 7643 with comprehensive
//! validation capabilities.
//!
//! ## Organization
//!
//! The schema module is organized into several sub-modules:
//!
//! - [`types`] - Core schema data structures (Schema, AttributeDefinition, etc.)
//! - [`registry`] - Schema registry for loading and managing schemas
//! - [`validation`] - Comprehensive validation logic for SCIM resources
//! - `tests` - Test cases for schema functionality
//!
//! ## Usage
//!
//! ```rust
//! use scim_server::schema::{SchemaRegistry, OperationContext};
//! use serde_json::json;
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! // Create a schema registry
//! let registry = SchemaRegistry::new()?;
//!
//! // Validate a SCIM resource
//! let user = json!({
//!     "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
//!     "userName": "jdoe@example.com"
//! });
//!
//! registry.validate_json_resource_with_context("User", &user, OperationContext::Create)?;
//! # Ok(())
//! # }
//! ```

pub mod registry;
pub mod types;
pub mod validation;

#[cfg(test)]
mod tests;

// Re-export the main types for convenience
pub use registry::SchemaRegistry;
pub use types::{AttributeDefinition, AttributeType, Mutability, Schema, Uniqueness};
pub use validation::OperationContext;
