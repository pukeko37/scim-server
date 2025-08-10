//! Advanced tenant configuration structures and types.
//!
//! This module contains the data structures and enums for configuring
//! advanced multi-tenant features including custom schemas, validation rules,
//! compliance levels, and feature flags.

use scim_server::Schema;
use serde_json::Value;
use std::collections::HashMap;

/// Advanced tenant configuration with custom schemas and rules
#[derive(Debug, Clone)]
pub struct AdvancedTenantConfig {
    pub tenant_id: String,
    pub custom_schemas: Vec<Schema>,
    pub validation_rules: Vec<CustomValidationRule>,
    pub data_retention_days: Option<u32>,
    pub compliance_level: ComplianceLevel,
    pub feature_flags: HashMap<String, bool>,
}

impl AdvancedTenantConfig {
    pub fn new(tenant_id: &str) -> Self {
        Self {
            tenant_id: tenant_id.to_string(),
            custom_schemas: Vec::new(),
            validation_rules: Vec::new(),
            data_retention_days: None,
            compliance_level: ComplianceLevel::Standard,
            feature_flags: HashMap::new(),
        }
    }

    pub fn with_custom_schema(mut self, schema: Schema) -> Self {
        self.custom_schemas.push(schema);
        self
    }

    pub fn with_validation_rule(mut self, rule: CustomValidationRule) -> Self {
        self.validation_rules.push(rule);
        self
    }

    pub fn with_compliance_level(mut self, level: ComplianceLevel) -> Self {
        self.compliance_level = level;
        self
    }

    pub fn with_feature_flag(mut self, feature: &str, enabled: bool) -> Self {
        self.feature_flags.insert(feature.to_string(), enabled);
        self
    }
}

/// Custom validation rules for tenant-specific requirements
#[derive(Debug, Clone)]
pub struct CustomValidationRule {
    pub name: String,
    pub resource_type: String,
    pub attribute: String,
    pub rule_type: ValidationRuleType,
    pub parameters: HashMap<String, Value>,
}

#[derive(Debug, Clone)]
pub enum ValidationRuleType {
    Required,
    Pattern {
        regex: String,
    },
    Length {
        min: Option<usize>,
        max: Option<usize>,
    },
    Custom {
        validator_name: String,
    },
}

/// Compliance levels for different tenant requirements
#[derive(Debug, Clone, PartialEq)]
pub enum ComplianceLevel {
    Basic,
    Standard,
    Enhanced,
    Strict, // GDPR, HIPAA, etc.
}
