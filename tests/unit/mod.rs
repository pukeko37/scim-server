//! Unit tests for SCIM server components
//!
//! This module contains unit tests that test individual components in isolation.
//! Unit tests focus on testing specific functions, methods, or small units of code
//! without external dependencies or complex integration scenarios.
//!
//! ## Organization
//!
//! - [`resource`] - Tests for core resource functionality
//! - [`value_objects`] - Tests for value object implementations
//! - [`schema`] - Tests for schema system components

pub mod resource;
pub mod schema;
pub mod value_objects;
