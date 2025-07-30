//! Domain-specific types for SCIM resources.
//!
//! This module contains specialized data structures that represent
//! specific domain concepts used in SCIM resources.

use serde::{Deserialize, Serialize};

/// Email address representation extracted from User resources.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmailAddress {
    pub value: String,
    #[serde(rename = "type")]
    pub email_type: Option<String>,
    pub primary: Option<bool>,
    pub display: Option<String>,
}
