//! Standard resource provider implementations.
//!
//! This module provides production-ready implementations of the ResourceProvider
//! trait that can be used directly or as reference implementations for custom
//! providers.
//!
//! # Available Providers
//!
//! * [`StandardResourceProvider`] - **RECOMMENDED** Production-ready provider with pluggable storage backends
//! * [`InMemoryProvider`] - ⚠️ **DEPRECATED** Thread-safe in-memory provider for testing and development
//!   - Use `StandardResourceProvider<InMemoryStorage>` instead for better separation of concerns
//!
//! All providers in this module implement the unified ResourceProvider trait,
//! supporting both single-tenant and multi-tenant operations through the
//! RequestContext interface.
//!
//! # Quick Start
//!
//! ```rust
//! use scim_server::providers::StandardResourceProvider;
//! use scim_server::storage::InMemoryStorage;
//!
//! // Recommended approach
//! let storage = InMemoryStorage::new();
//! let provider = StandardResourceProvider::new(storage);
//! ```

pub mod in_memory;
pub mod standard;

// Re-export the recommended types
pub use crate::storage::{InMemoryStorage, StorageProvider};
pub use standard::StandardResourceProvider;

// Legacy deprecated exports - will be removed in future version
#[deprecated(
    since = "0.3.10",
    note = "Use `StandardResourceProvider<InMemoryStorage>` instead. InMemoryProvider will be removed in a future version."
)]
pub use in_memory::{InMemoryError, InMemoryProvider, InMemoryStats};
