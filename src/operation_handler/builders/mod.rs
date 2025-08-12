//! Builder utilities for operation handler types
//!
//! This module provides convenient builder methods for constructing
//! operation handler types such as ScimOperationRequest and ScimQuery.

pub mod query;
pub mod request;

// Builder implementations are available through impl blocks on core types
// No re-exports needed since modules only contain trait implementations
