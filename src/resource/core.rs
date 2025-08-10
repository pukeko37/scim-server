//! Core types for SCIM resource operations.
//!
//! This module contains the fundamental data structures used throughout
//! the SCIM server for representing resources and operation contexts.
//!
//! The core functionality has been split into focused modules:
//! - `tenant` - Tenant-related types and contexts
//! - `resource` - Core Resource struct and validation
//! - `builder` - ResourceBuilder functionality
//! - `context` - Request contexts and query structures
//! - `serialization` - Serde implementations

// Re-export all types from the split modules for backward compatibility
pub use crate::resource::builder::ResourceBuilder;
pub use crate::resource::context::{ListQuery, RequestContext};
pub use crate::resource::resource::Resource;
pub use crate::resource::tenant::{IsolationLevel, TenantContext, TenantPermissions};

// Re-export ScimOperation from multi_tenant module for backward compatibility
pub use crate::multi_tenant::ScimOperation;
