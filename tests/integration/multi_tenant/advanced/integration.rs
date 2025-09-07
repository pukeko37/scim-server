//! Integration test utilities and provider implementation for advanced multi-tenant features.
//!
//! This module contains the provider trait definition for advanced multi-tenant
//! functionality and its test implementation, along with integration test utilities
//! and comprehensive test suites.

use super::{
    bulk_operations::{
        BulkOperationItemResult, BulkOperationRequest, BulkOperationResult, BulkOperationType,
        TenantMigrationRequest,
    },
    compliance::AuditLogEntry,
    config::{AdvancedTenantConfig, ComplianceLevel, CustomValidationRule},
    performance::TenantStatistics,
};
use scim_server::Resource;
use scim_server::ResourceProvider;
use scim_server::resource::{
    RequestContext, TenantContext, version::RawVersion, versioned::VersionedResource,
};
use serde_json::{Value, json};
use std::collections::HashMap;

// ============================================================================
// Advanced Multi-Tenant Provider Trait
// ============================================================================

/// Extended provider trait for advanced multi-tenant features
pub trait AdvancedMultiTenantProvider: ResourceProvider {
    /// Execute bulk operations within tenant scope
    fn execute_bulk_operation(
        &self,
        request: BulkOperationRequest,
        context: &RequestContext,
    ) -> impl std::future::Future<
        Output = Result<BulkOperationResult, <Self as ResourceProvider>::Error>,
    > + Send;

    /// Migrate tenant data between tenants
    fn migrate_tenant_data(
        &self,
        request: TenantMigrationRequest,
        context: &RequestContext,
    ) -> impl std::future::Future<Output = Result<(), Self::Error>> + Send;

    /// Get audit log entries for a tenant
    fn get_audit_log(
        &self,
        tenant_id: &str,
        start_time: Option<chrono::DateTime<chrono::Utc>>,
        end_time: Option<chrono::DateTime<chrono::Utc>>,
        context: &RequestContext,
    ) -> impl std::future::Future<Output = Result<Vec<AuditLogEntry>, Self::Error>> + Send;

    /// Validate tenant-specific custom rules
    fn validate_custom_rules(
        &self,
        tenant_id: &str,
        resource_type: &str,
        data: &Value,
        rules: &[CustomValidationRule],
        context: &RequestContext,
    ) -> impl std::future::Future<Output = Result<Vec<String>, Self::Error>> + Send;

    /// Get tenant usage statistics
    fn get_tenant_statistics(
        &self,
        tenant_id: &str,
        context: &RequestContext,
    ) -> impl std::future::Future<Output = Result<TenantStatistics, Self::Error>> + Send;
}

// ============================================================================
// Test Implementation of Advanced Provider
// ============================================================================

/// Test implementation of advanced multi-tenant provider
pub struct TestAdvancedProvider {
    base_provider: crate::integration::multi_tenant::provider_trait::TestMultiTenantProvider,
    pub tenant_configs: tokio::sync::RwLock<HashMap<String, AdvancedTenantConfig>>,
    audit_log: tokio::sync::RwLock<Vec<AuditLogEntry>>,
}

impl TestAdvancedProvider {
    pub fn new() -> Self {
        Self {
            base_provider:
                crate::integration::multi_tenant::provider_trait::TestMultiTenantProvider::new(),
            tenant_configs: tokio::sync::RwLock::new(HashMap::new()),
            audit_log: tokio::sync::RwLock::new(Vec::new()),
        }
    }

    pub async fn configure_tenant(&self, config: AdvancedTenantConfig) {
        let mut configs = self.tenant_configs.write().await;
        configs.insert(config.tenant_id.clone(), config);
    }

    async fn log_operation(
        &self,
        tenant_id: &str,
        operation: &str,
        resource_type: &str,
        resource_id: Option<&str>,
        context: &RequestContext,
    ) {
        let entry = AuditLogEntry {
            timestamp: chrono::Utc::now(),
            tenant_id: tenant_id.to_string(),
            user_id: context
                .tenant_context
                .as_ref()
                .map(|tc| tc.client_id.clone()),
            operation: operation.to_string(),
            resource_type: resource_type.to_string(),
            resource_id: resource_id.map(|s| s.to_string()),
            details: HashMap::new(),
            compliance_metadata: None,
        };

        let mut audit_log = self.audit_log.write().await;
        audit_log.push(entry);
    }
}

impl ResourceProvider for TestAdvancedProvider {
    type Error = crate::integration::multi_tenant::provider_trait::TestProviderError;

    async fn create_resource(
        &self,
        resource_type: &str,
        data: Value,
        context: &RequestContext,
    ) -> Result<VersionedResource, Self::Error> {
        let tenant_id = context.tenant_id().unwrap_or("default");

        let result = self
            .base_provider
            .create_resource(resource_type, data, context)
            .await?;

        self.log_operation(tenant_id, "create", resource_type, result.get_id(), context)
            .await;

        Ok(result)
    }

    async fn get_resource(
        &self,
        resource_type: &str,
        id: &str,
        context: &RequestContext,
    ) -> Result<Option<VersionedResource>, Self::Error> {
        let tenant_id = context.tenant_id().unwrap_or("default");

        let result = self
            .base_provider
            .get_resource(resource_type, id, context)
            .await?;

        self.log_operation(tenant_id, "get", resource_type, Some(id), context)
            .await;

        Ok(result)
    }

    async fn update_resource(
        &self,
        resource_type: &str,
        id: &str,
        data: Value,
        expected_version: Option<&RawVersion>,
        context: &RequestContext,
    ) -> Result<VersionedResource, Self::Error> {
        let tenant_id = context.tenant_id().unwrap_or("default");

        let result = self
            .base_provider
            .update_resource(resource_type, id, data, expected_version, context)
            .await?;

        self.log_operation(tenant_id, "update", resource_type, Some(id), context)
            .await;

        Ok(result)
    }

    async fn delete_resource(
        &self,
        resource_type: &str,
        id: &str,
        expected_version: Option<&RawVersion>,
        context: &RequestContext,
    ) -> Result<(), Self::Error> {
        let tenant_id = context.tenant_id().unwrap_or("default");

        self.base_provider
            .delete_resource(resource_type, id, expected_version, context)
            .await?;

        self.log_operation(tenant_id, "delete", resource_type, Some(id), context)
            .await;

        Ok(())
    }

    async fn list_resources(
        &self,
        resource_type: &str,
        query: Option<&scim_server::resource::ListQuery>,
        context: &RequestContext,
    ) -> Result<Vec<VersionedResource>, Self::Error> {
        let tenant_id = context.tenant_id().unwrap_or("default");

        let result = self
            .base_provider
            .list_resources(resource_type, query, context)
            .await?;

        self.log_operation(tenant_id, "list", resource_type, None, context)
            .await;

        Ok(result)
    }

    async fn find_resources_by_attribute(
        &self,
        resource_type: &str,
        attribute: &str,
        value: &str,
        context: &RequestContext,
    ) -> Result<Vec<VersionedResource>, Self::Error> {
        let tenant_id = context.tenant_id().unwrap_or("default");

        let result = self
            .base_provider
            .find_resources_by_attribute(resource_type, attribute, value, context)
            .await?;

        self.log_operation(tenant_id, "find", resource_type, None, context)
            .await;

        Ok(result)
    }

    async fn patch_resource(
        &self,
        resource_type: &str,
        id: &str,
        patch_request: &Value,
        expected_version: Option<&RawVersion>,
        context: &RequestContext,
    ) -> Result<VersionedResource, Self::Error> {
        let tenant_id = context.tenant_id().unwrap_or("default");

        let result = self
            .base_provider
            .patch_resource(resource_type, id, patch_request, expected_version, context)
            .await?;

        self.log_operation(tenant_id, "patch", resource_type, Some(id), context)
            .await;

        Ok(result)
    }

    async fn resource_exists(
        &self,
        resource_type: &str,
        id: &str,
        context: &RequestContext,
    ) -> Result<bool, Self::Error> {
        let tenant_id = context.tenant_id().unwrap_or("default");

        let result = self
            .base_provider
            .resource_exists(resource_type, id, context)
            .await?;

        self.log_operation(tenant_id, "exists", resource_type, Some(id), context)
            .await;

        Ok(result)
    }
}

impl AdvancedMultiTenantProvider for TestAdvancedProvider {
    async fn execute_bulk_operation(
        &self,
        request: BulkOperationRequest,
        context: &RequestContext,
    ) -> Result<BulkOperationResult, Self::Error> {
        let start_time = std::time::Instant::now();
        let mut results = Vec::new();
        let mut successful = 0;
        let mut failed = 0;

        for (index, operation) in request.operations.iter().enumerate() {
            let operation_result = match &operation.operation_type {
                BulkOperationType::Create => {
                    if let Some(data) = &operation.data {
                        match self
                            .create_resource(&operation.resource_type, data.clone(), context)
                            .await
                        {
                            Ok(resource) => {
                                successful += 1;
                                BulkOperationItemResult {
                                    operation_index: index,
                                    success: true,
                                    resource: Some(resource.into_resource()),
                                    error: None,
                                }
                            }
                            Err(e) => {
                                failed += 1;
                                BulkOperationItemResult {
                                    operation_index: index,
                                    success: false,
                                    resource: None,
                                    error: Some(format!("{:?}", e)),
                                }
                            }
                        }
                    } else {
                        failed += 1;
                        BulkOperationItemResult {
                            operation_index: index,
                            success: false,
                            resource: None,
                            error: Some("No data provided for create operation".to_string()),
                        }
                    }
                }
                BulkOperationType::Update => {
                    if let (Some(id), Some(data)) = (&operation.resource_id, &operation.data) {
                        match self
                            .update_resource(
                                &operation.resource_type,
                                id,
                                data.clone(),
                                None,
                                context,
                            )
                            .await
                        {
                            Ok(resource) => {
                                successful += 1;
                                BulkOperationItemResult {
                                    operation_index: index,
                                    success: true,
                                    resource: Some(resource.into_resource()),
                                    error: None,
                                }
                            }
                            Err(e) => {
                                failed += 1;
                                BulkOperationItemResult {
                                    operation_index: index,
                                    success: false,
                                    resource: None,
                                    error: Some(format!("{:?}", e)),
                                }
                            }
                        }
                    } else {
                        failed += 1;
                        BulkOperationItemResult {
                            operation_index: index,
                            success: false,
                            resource: None,
                            error: Some(
                                "Missing resource ID or data for update operation".to_string(),
                            ),
                        }
                    }
                }
                BulkOperationType::Delete => {
                    if let Some(id) = &operation.resource_id {
                        match self
                            .delete_resource(&operation.resource_type, id, None, context)
                            .await
                        {
                            Ok(_) => {
                                successful += 1;
                                BulkOperationItemResult {
                                    operation_index: index,
                                    success: true,
                                    resource: None,
                                    error: None,
                                }
                            }
                            Err(e) => {
                                failed += 1;
                                BulkOperationItemResult {
                                    operation_index: index,
                                    success: false,
                                    resource: None,
                                    error: Some(format!("{:?}", e)),
                                }
                            }
                        }
                    } else {
                        failed += 1;
                        BulkOperationItemResult {
                            operation_index: index,
                            success: false,
                            resource: None,
                            error: Some("Missing resource ID for delete operation".to_string()),
                        }
                    }
                }
                BulkOperationType::Patch => {
                    // Simplified patch implementation for testing
                    failed += 1;
                    BulkOperationItemResult {
                        operation_index: index,
                        success: false,
                        resource: None,
                        error: Some(
                            "Patch operations not implemented in test provider".to_string(),
                        ),
                    }
                }
            };

            results.push(operation_result);

            // Check fail-fast behavior
            if request.fail_on_errors && failed > 0 {
                break;
            }
        }

        let duration = start_time.elapsed();

        Ok(BulkOperationResult {
            total_operations: request.operations.len(),
            successful_operations: successful,
            failed_operations: failed,
            results,
            duration,
        })
    }

    async fn migrate_tenant_data(
        &self,
        _request: TenantMigrationRequest,
        _context: &RequestContext,
    ) -> Result<(), Self::Error> {
        // Simplified implementation for testing
        Ok(())
    }

    async fn get_audit_log(
        &self,
        tenant_id: &str,
        start_time: Option<chrono::DateTime<chrono::Utc>>,
        end_time: Option<chrono::DateTime<chrono::Utc>>,
        _context: &RequestContext,
    ) -> Result<Vec<AuditLogEntry>, Self::Error> {
        let audit_log = self.audit_log.read().await;
        let mut filtered_entries: Vec<AuditLogEntry> = audit_log
            .iter()
            .filter(|entry| entry.tenant_id == tenant_id)
            .cloned()
            .collect();

        if let Some(start) = start_time {
            filtered_entries.retain(|entry| entry.timestamp >= start);
        }

        if let Some(end) = end_time {
            filtered_entries.retain(|entry| entry.timestamp <= end);
        }

        Ok(filtered_entries)
    }

    async fn validate_custom_rules(
        &self,
        tenant_id: &str,
        resource_type: &str,
        data: &Value,
        rules: &[CustomValidationRule],
        _context: &RequestContext,
    ) -> Result<Vec<String>, Self::Error> {
        let configs = self.tenant_configs.read().await;
        let _tenant_config = configs.get(tenant_id);

        let mut errors = Vec::new();

        for rule in rules {
            if rule.resource_type != resource_type {
                continue;
            }

            // Simplified validation implementation for testing
            match &rule.rule_type {
                super::config::ValidationRuleType::Required => {
                    if data.get(&rule.attribute).is_none() {
                        errors.push(format!(
                            "Required attribute '{}' is missing",
                            rule.attribute
                        ));
                    }
                }
                super::config::ValidationRuleType::Pattern { regex: _ } => {
                    // Pattern validation would be implemented here
                }
                super::config::ValidationRuleType::Length { min, max } => {
                    if let Some(value) = data.get(&rule.attribute) {
                        if let Some(string_value) = value.as_str() {
                            if let Some(min_len) = min {
                                if string_value.len() < *min_len {
                                    errors.push(format!(
                                        "Attribute '{}' is too short (minimum {} characters)",
                                        rule.attribute, min_len
                                    ));
                                }
                            }
                            if let Some(max_len) = max {
                                if string_value.len() > *max_len {
                                    errors.push(format!(
                                        "Attribute '{}' is too long (maximum {} characters)",
                                        rule.attribute, max_len
                                    ));
                                }
                            }
                        }
                    }
                }
                super::config::ValidationRuleType::Custom { validator_name: _ } => {
                    // Custom validation would be implemented here
                }
            }
        }

        Ok(errors)
    }

    async fn get_tenant_statistics(
        &self,
        _tenant_id: &str,
        context: &RequestContext,
    ) -> Result<TenantStatistics, Self::Error> {
        let users = self.list_resources("User", None, context).await?;
        let groups = self.list_resources("Group", None, context).await?;

        let mut resources_by_type = HashMap::new();
        resources_by_type.insert("User".to_string(), users.len());
        resources_by_type.insert("Group".to_string(), groups.len());

        Ok(TenantStatistics {
            tenant_id: context.tenant_id().unwrap_or("default").to_string(),
            total_resources: users.len() + groups.len(),
            resources_by_type,
            storage_usage_bytes: 0, // Would be calculated in real implementation
            last_activity: Some(chrono::Utc::now()),
            operations_count: 0, // Would be tracked in real implementation
        })
    }
}

// ============================================================================
// Test Utilities for Advanced Features
// ============================================================================

/// Test harness for advanced multi-tenant scenarios
pub struct TestHarness {
    provider: TestAdvancedProvider,
}

impl TestHarness {
    pub fn new() -> Self {
        Self {
            provider: TestAdvancedProvider::new(),
        }
    }

    pub fn provider(&self) -> &TestAdvancedProvider {
        &self.provider
    }

    pub fn create_test_context(tenant_id: &str) -> RequestContext {
        let tenant_context = TenantContext::new(tenant_id.to_string(), "test-client".to_string());
        RequestContext::with_tenant(format!("req_{}", tenant_id), tenant_context)
    }

    pub fn create_test_user(username: &str) -> Value {
        json!({
            "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
            "userName": username,
            "displayName": format!("{} User", username),
            "active": true
        })
    }

    pub fn create_test_group(name: &str) -> Value {
        json!({
            "schemas": ["urn:ietf:params:scim:schemas:core:2.0:Group"],
            "displayName": name,
            "members": []
        })
    }

    pub async fn setup_test_tenant(&self, tenant_id: &str, compliance_level: ComplianceLevel) {
        let config = AdvancedTenantConfig::new(tenant_id)
            .with_compliance_level(compliance_level)
            .with_feature_flag("audit_logging", true)
            .with_feature_flag("custom_validation", true);

        self.provider.configure_tenant(config).await;
    }

    pub async fn create_test_data(
        &self,
        tenant_id: &str,
    ) -> Result<(Resource, Resource), Box<dyn std::error::Error>> {
        let context = Self::create_test_context(tenant_id);

        let user_data = Self::create_test_user("testuser");
        let group_data = Self::create_test_group("testgroup");

        let user = self
            .provider
            .create_resource("User", user_data, &context)
            .await?;

        let group = self
            .provider
            .create_resource("Group", group_data, &context)
            .await?;

        Ok((user.into_resource(), group.into_resource()))
    }
}

impl Default for TestHarness {
    fn default() -> Self {
        Self::new()
    }
}
