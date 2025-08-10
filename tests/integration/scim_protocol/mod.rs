//! Integration tests for SCIM protocol compliance
//!
//! This module contains integration tests that validate compliance with the
//! SCIM 2.0 protocol (RFC 7643 and RFC 7644). These tests focus on end-to-end
//! SCIM protocol behavior, including resource lifecycle operations, schema
//! discovery, and error handling.
//!
//! ## Test Organization
//!
//! - [`capability_discovery`] - Service provider capability discovery tests
//! - [`group_lifecycle`] - Group resource lifecycle operations
//!
//! ## SCIM Compliance Areas
//!
//! These tests validate:
//! - Resource creation, retrieval, update, and deletion (CRUD)
//! - Schema discovery and validation
//! - Error response formats and status codes
//! - Query and filtering operations
//! - Bulk operations
//! - Resource versioning and meta attributes
//!
//! ## Test Data
//!
//! Tests use RFC 7643 compliant test data and validate against official
//! SCIM schemas to ensure full protocol compliance.

pub mod capability_discovery;
pub mod group_lifecycle;
