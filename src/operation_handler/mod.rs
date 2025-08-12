//! Operation Handler Foundation
//!
//! This module provides a framework-agnostic operation handler that serves as the foundation
//! for both HTTP and MCP integrations. It abstracts SCIM operations into structured
//! request/response types while maintaining type safety and comprehensive error handling.
//!
//! ## ETag Concurrency Control
//!
//! The operation handler provides built-in support for ETag-based conditional operations:
//!
//! - **Automatic Version Management**: All operations include version information in responses
//! - **Conditional Updates**: Support for If-Match style conditional operations
//! - **Conflict Detection**: Automatic detection and handling of version conflicts
//! - **HTTP Compliance**: RFC 7232 compliant ETag headers in metadata
//!
//! ## Module Structure
//!
//! The operation handler is organized into focused modules:
//!
//! - [`core`] - Core types, handler struct, and main dispatcher
//! - [`handlers`] - Operation-specific handlers (CRUD, query, schema, utility)
//! - [`builders`] - Builder utilities for requests and queries
//! - [`errors`] - Error handling utilities
//!
//! ## Example Usage
//!
//! ```rust,no_run
//! use scim_server::operation_handler::{ScimOperationHandler, ScimOperationRequest};
//! use scim_server::resource::version::ScimVersion;
//! use scim_server::{ScimServer, providers::InMemoryProvider};
//! use serde_json::json;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let provider = InMemoryProvider::new();
//! let server = ScimServer::new(provider)?;
//! let handler = ScimOperationHandler::new(server);
//!
//! // Regular update (returns version information)
//! let update_request = ScimOperationRequest::update(
//!     "User", "123", json!({"userName": "new.name", "active": true})
//! );
//! let response = handler.handle_operation(update_request).await;
//! let new_etag = response.metadata.additional.get("etag").unwrap();
//!
//! // Conditional update with version check
//! let version = ScimVersion::parse_http_header(new_etag.as_str().unwrap())?;
//! let conditional_request = ScimOperationRequest::update(
//!     "User", "123", json!({"userName": "newer.name", "active": false})
//! ).with_expected_version(version);
//!
//! let conditional_response = handler.handle_operation(conditional_request).await;
//! if conditional_response.success {
//!     println!("Update succeeded!");
//! } else {
//!     println!("Version conflict: {}", conditional_response.error.unwrap());
//! }
//! # Ok(())
//! # }
//! ```

mod builders;
mod core;
mod errors;
mod handlers;

// Re-export all public types and functions
pub use core::{
    OperationMetadata, ScimOperationHandler, ScimOperationRequest, ScimOperationResponse,
    ScimOperationType, ScimQuery,
};

// Re-export builder utilities
pub use builders::*;

// Re-export error utilities for advanced usage
pub use errors::{create_error_response, create_version_conflict_response};
