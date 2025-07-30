//! Core SCIM server structure and initialization.
//!
//! This module contains the main ScimServer struct definition and its
//! constructor logic, representing the fundamental server structure
//! without specific operational concerns.

use crate::error::ScimError;
use crate::resource::{ResourceHandler, ResourceProvider, ScimOperation};
use crate::schema::SchemaRegistry;
use std::collections::HashMap;
use std::sync::Arc;

/// Completely dynamic SCIM server with no hard-coded resource types
pub struct ScimServer<P> {
    pub(super) provider: P,
    pub(super) schema_registry: SchemaRegistry,
    pub(super) resource_handlers: HashMap<String, Arc<ResourceHandler>>, // resource_type -> handler
    pub(super) supported_operations: HashMap<String, Vec<ScimOperation>>, // resource_type -> supported ops
}

impl<P: ResourceProvider> ScimServer<P> {
    /// Create a new SCIM server
    pub fn new(provider: P) -> Result<Self, ScimError> {
        let schema_registry = SchemaRegistry::new()
            .map_err(|e| ScimError::internal(format!("Failed to create schema registry: {}", e)))?;

        Ok(Self {
            provider,
            schema_registry,
            resource_handlers: HashMap::new(),
            supported_operations: HashMap::new(),
        })
    }
}
