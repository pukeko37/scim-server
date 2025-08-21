//! Standard resource provider implementations.
//!
//! This module provides production-ready implementations of the ResourceProvider
//! trait that can be used directly or as reference implementations for custom
//! providers.
//!
//! # Available Providers
//!
//! * [`InMemoryProvider`] - ⚠️ **DEPRECATED** Thread-safe in-memory provider for testing and development
//!   - Use `StandardResourceProvider<InMemoryStorage>` instead for better separation of concerns
//! * [`StandardResourceProvider`] - **RECOMMENDED** Production-ready provider with pluggable storage backends
//!
//! All providers in this module implement the unified ResourceProvider trait,
//! supporting both single-tenant and multi-tenant operations through the
//! RequestContext interface.

pub mod in_memory;
pub mod standard;

pub use in_memory::{InMemoryError, InMemoryProvider, InMemoryStats};
pub use standard::StandardResourceProvider;
