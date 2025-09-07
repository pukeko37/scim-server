//! Standard resource provider module.
//!
//! This module contains the implementation of the StandardResourceProvider
//! and related functionality for SCIM resource management with pluggable
//! storage backends.

mod standard;

pub use standard::StandardResourceProvider;
