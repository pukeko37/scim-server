//! Framework-agnostic SCIM operation handler.
//!
//! This module provides structured request/response handling for SCIM operations
//! with built-in ETag concurrency control and comprehensive error handling.
//!
//! # Key Types
//!
//! - [`ScimOperationHandler`] - Main handler for processing SCIM operations
//! - [`ScimOperationRequest`] - Structured request wrapper with validation
//! - [`ScimOperationResponse`] - Response with metadata and ETag information
//!
//! # Examples
//!
//! ```rust,no_run
//! use scim_server::operation_handler::{ScimOperationHandler, ScimOperationRequest};
//! use scim_server::{ScimServer, providers::StandardResourceProvider};
//! use scim_server::storage::InMemoryStorage;
//! use serde_json::json;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let storage = InMemoryStorage::new();
//! let provider = StandardResourceProvider::new(storage);
//! let server = ScimServer::new(provider)?;
//! let handler = ScimOperationHandler::new(server);
//!
//! let request = ScimOperationRequest::update("User", "123", json!({"active": true}));
//! let response = handler.handle_operation(request).await;
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
