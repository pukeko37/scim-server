//! Unit tests for resource functionality
//!
//! This module contains unit tests for the core Resource type and related
//! functionality. These tests focus on testing resource creation, manipulation,
//! serialization, and validation in isolation.
//!
//! ## Test Organization
//!
//! - [`core_functionality`] - Core resource creation and manipulation
//! - [`common_attributes`] - Common SCIM attribute validation
//! - [`attribute_characteristics`] - Attribute characteristic validation

pub mod attribute_characteristics;
pub mod common_attributes;
pub mod core_functionality;
