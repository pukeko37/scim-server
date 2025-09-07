//! Helper traits for composable SCIM ResourceProvider implementations.
//!
//! This module provides reusable traits that implement specific aspects of SCIM protocol
//! functionality. These traits can be mixed and matched by ResourceProvider implementers
//! to add capabilities without reimplementing complex SCIM logic.
//!
//! # Available Traits
//!
//! * [`ScimPatchOperations`] - RFC 7644 compliant PATCH operations
//! * [`ScimMetadataManager`] - SCIM resource metadata management
//! * [`ScimValidator`] - SCIM attribute validation and path parsing
//! * [`ConditionalOperations`] - Version-based optimistic concurrency control
//! * [`MultiTenantProvider`] - Multi-tenant context management
//!
//! # Usage Pattern
//!
//! ```rust,no_run
//! // Helper traits provide reusable functionality for ResourceProvider implementations:
//! //
//! // ScimPatchOperations - RFC-compliant PATCH operation handling
//! // ScimMetadataManager - Automatic metadata management (timestamps, versions, URIs)
//! // ConditionalOperations - Optimistic locking for concurrent updates
//! // MultiTenantProvider - Tenant isolation and scoping
//! // ScimValidator - SCIM path and data validation
//! //
//! // Simply implement ResourceProvider and add trait implementations:
//! // impl<S> ScimPatchOperations for MyProvider<S> {}
//! // impl<S> ScimMetadataManager for MyProvider<S> {}
//! ```
//!
//! # Benefits
//!
//! * **RFC Compliance** - Battle-tested implementations of SCIM specifications
//! * **Composability** - Mix and match only the capabilities you need
//! * **DRY Principle** - Avoid reimplementing complex SCIM protocol logic
//! * **Maintainability** - Bug fixes and improvements benefit all users
//! * **Testing** - Each trait can be tested independently

pub mod conditional;
pub mod metadata;
pub mod patch;
pub mod tenant;
pub mod validation;

// Re-export all traits for convenience
pub use conditional::ConditionalOperations;
pub use metadata::ScimMetadataManager;
pub use patch::ScimPatchOperations;
pub use tenant::MultiTenantProvider;
pub use validation::ScimValidator;
