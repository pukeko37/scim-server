//! Operation handler modules
//!
//! This module contains all the specific operation handlers organized by functionality:
//! - CRUD operations (create, read, update, delete)
//! - Query operations (list, search)
//! - Schema operations (get schemas, get schema)
//! - Utility operations (exists check)

pub mod crud;
pub mod query;
pub mod schema;
pub mod utility;

// Handler functions are accessed directly by the core dispatcher
// No re-exports needed since they're called via super::handlers::module::function
