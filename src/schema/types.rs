//! Core schema type definitions for SCIM resources.
//!
//! This module contains the fundamental data structures that define SCIM schemas,
//! attribute definitions, and their characteristics as specified in RFC 7643.

use serde::{Deserialize, Serialize};

/// A SCIM schema definition.
///
/// Represents a complete schema with its metadata and attribute definitions.
/// Each schema defines the structure and validation rules for a specific
/// resource type like User or Group.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Schema {
    /// Unique schema identifier (URI)
    pub id: String,
    /// Human-readable schema name
    pub name: String,
    /// Schema description
    pub description: String,
    /// List of attribute definitions
    pub attributes: Vec<AttributeDefinition>,
}

/// Definition of a SCIM attribute.
///
/// Defines all characteristics of an attribute including type,
/// constraints, and validation rules.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AttributeDefinition {
    /// Attribute name
    pub name: String,
    /// Data type of the attribute
    #[serde(rename = "type")]
    pub data_type: AttributeType,
    /// Whether this attribute can have multiple values
    #[serde(rename = "multiValued")]
    pub multi_valued: bool,
    /// Whether this attribute is required
    pub required: bool,
    /// Whether string comparison is case-sensitive
    #[serde(rename = "caseExact")]
    pub case_exact: bool,
    /// Mutability characteristics
    pub mutability: Mutability,
    /// Uniqueness constraints
    pub uniqueness: Uniqueness,
    /// Allowed values for string attributes
    #[serde(rename = "canonicalValues", default)]
    pub canonical_values: Vec<String>,
    /// Sub-attributes for complex types
    #[serde(rename = "subAttributes", default)]
    pub sub_attributes: Vec<AttributeDefinition>,
    /// How the attribute is returned in responses
    #[serde(default)]
    pub returned: Option<String>,
}

impl Default for AttributeDefinition {
    fn default() -> Self {
        Self {
            name: String::new(),
            data_type: AttributeType::String,
            multi_valued: false,
            required: false,
            case_exact: false,
            mutability: Mutability::ReadWrite,
            uniqueness: Uniqueness::None,
            canonical_values: Vec::new(),
            sub_attributes: Vec::new(),
            returned: None,
        }
    }
}

/// SCIM attribute data types.
///
/// Represents the valid data types for SCIM attributes as defined in RFC 7643.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum AttributeType {
    /// String value
    String,
    /// Boolean value
    Boolean,
    /// Decimal number
    Decimal,
    /// Integer number
    Integer,
    /// DateTime in RFC3339 format
    DateTime,
    /// Binary data (base64 encoded)
    Binary,
    /// URI reference
    Reference,
    /// Complex attribute with sub-attributes
    Complex,
}

impl Default for AttributeType {
    fn default() -> Self {
        Self::String
    }
}

/// Attribute mutability characteristics.
///
/// Defines whether and how an attribute can be modified.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum Mutability {
    /// Read-only attribute (managed by server)
    ReadOnly,
    /// Read-write attribute (can be modified by clients)
    ReadWrite,
    /// Immutable attribute (set once, never modified)
    Immutable,
    /// Write-only attribute (passwords, etc.)
    WriteOnly,
}

impl Default for Mutability {
    fn default() -> Self {
        Self::ReadWrite
    }
}

/// Attribute uniqueness constraints.
///
/// Defines the scope of uniqueness for attribute values.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum Uniqueness {
    /// No uniqueness constraint
    None,
    /// Unique within the server
    Server,
    /// Globally unique
    Global,
}

impl Default for Uniqueness {
    fn default() -> Self {
        Self::None
    }
}
