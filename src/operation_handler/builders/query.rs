//! Query builder utilities for ScimQuery
//!
//! This module provides convenient builder methods for constructing
//! ScimQuery instances with various filtering and pagination options.

use crate::operation_handler::core::ScimQuery;
use serde_json::Value;

impl ScimQuery {
    /// Create a new empty query.
    pub fn new() -> Self {
        Self {
            count: None,
            start_index: None,
            filter: None,
            attributes: None,
            excluded_attributes: None,
            search_attribute: None,
            search_value: None,
        }
    }

    /// Set pagination parameters.
    pub fn with_pagination(mut self, start_index: usize, count: usize) -> Self {
        self.start_index = Some(start_index);
        self.count = Some(count);
        self
    }

    /// Set filter expression.
    pub fn with_filter(mut self, filter: impl Into<String>) -> Self {
        self.filter = Some(filter.into());
        self
    }

    /// Set search parameters.
    pub fn with_search(mut self, attribute: impl Into<String>, value: Value) -> Self {
        self.search_attribute = Some(attribute.into());
        self.search_value = Some(value);
        self
    }

    /// Set attributes to include.
    pub fn with_attributes(mut self, attributes: Vec<String>) -> Self {
        self.attributes = Some(attributes);
        self
    }

    /// Set attributes to exclude.
    pub fn with_excluded_attributes(mut self, excluded_attributes: Vec<String>) -> Self {
        self.excluded_attributes = Some(excluded_attributes);
        self
    }
}

impl Default for ScimQuery {
    fn default() -> Self {
        Self::new()
    }
}
