//! Schema definitions and validation for SCIM resources.
//!
//! This module provides the schema registry and validation engine implementing
//! RFC 7643 SCIM core schemas with comprehensive validation capabilities.
//!
//! # Key Types
//!
//! - [`Schema`] - SCIM schema definition with attributes and metadata
//! - [`SchemaRegistry`] - Registry for managing and accessing schemas
//! - [`AttributeDefinition`] - Individual attribute specifications and constraints
//!
//! # Examples
//!
//! ```rust
//! use scim_server::schema::SchemaRegistry;
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let registry = SchemaRegistry::new()?;
//! let user_schema = registry.get_user_schema();
//! # Ok(())
//! # }
//! ```

pub mod embedded;
pub mod registry;
pub mod types;
pub mod validation;

#[cfg(test)]
mod tests;

// Re-export the main types for convenience
pub use registry::SchemaRegistry;
pub use types::{AttributeDefinition, AttributeType, Mutability, Schema, Uniqueness};
pub use validation::OperationContext;
