//! Unit tests for value object implementations
//!
//! This module contains unit tests for all SCIM value objects, including
//! core types, complex types, multi-valued containers, and the schema-driven
//! factory system. These tests focus on testing value objects in isolation.
//!
//! ## Test Organization
//!
//! - [`core_types`] - Basic value objects (ResourceId, UserName, ExternalId, etc.)
//! - [`complex_types`] - Complex value objects (Meta, Name, Address, etc.)
//! - [`multi_valued`] - Multi-valued attribute container tests
//! - [`multi_valued_validation`] - Multi-valued attribute validation tests
//! - [`factory`] - Schema-driven value object factory tests
//! - [`integration_tests`] - Value object integration scenarios

pub mod complex_types;
pub mod core_types;
pub mod factory;
pub mod integration_tests;
pub mod multi_valued;
pub mod multi_valued_validation;
