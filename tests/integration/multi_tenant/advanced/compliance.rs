//! Compliance and audit logging functionality for multi-tenant SCIM operations.
//!
//! This module contains the data structures and functionality for tracking
//! and auditing tenant operations for compliance purposes, including audit
//! logs, compliance metadata, and regulatory compliance features.

use serde_json::Value;
use std::collections::HashMap;

/// Audit log entry for compliance tracking
#[derive(Debug, Clone)]
pub struct AuditLogEntry {
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub tenant_id: String,
    pub user_id: Option<String>,
    pub operation: String,
    pub resource_type: String,
    pub resource_id: Option<String>,
    pub details: HashMap<String, Value>,
    pub compliance_metadata: Option<ComplianceMetadata>,
}

#[derive(Debug, Clone)]
pub struct ComplianceMetadata {
    pub data_classification: String,
    pub retention_period: Option<u32>,
    pub access_justification: Option<String>,
}

impl AuditLogEntry {
    pub fn new(tenant_id: String, operation: String, resource_type: String) -> Self {
        Self {
            timestamp: chrono::Utc::now(),
            tenant_id,
            user_id: None,
            operation,
            resource_type,
            resource_id: None,
            details: HashMap::new(),
            compliance_metadata: None,
        }
    }

    pub fn with_user_id(mut self, user_id: String) -> Self {
        self.user_id = Some(user_id);
        self
    }

    pub fn with_resource_id(mut self, resource_id: String) -> Self {
        self.resource_id = Some(resource_id);
        self
    }

    pub fn with_detail(mut self, key: String, value: Value) -> Self {
        self.details.insert(key, value);
        self
    }

    pub fn with_compliance_metadata(mut self, metadata: ComplianceMetadata) -> Self {
        self.compliance_metadata = Some(metadata);
        self
    }
}

impl ComplianceMetadata {
    pub fn new(data_classification: String) -> Self {
        Self {
            data_classification,
            retention_period: None,
            access_justification: None,
        }
    }

    pub fn with_retention_period(mut self, days: u32) -> Self {
        self.retention_period = Some(days);
        self
    }

    pub fn with_access_justification(mut self, justification: String) -> Self {
        self.access_justification = Some(justification);
        self
    }
}
