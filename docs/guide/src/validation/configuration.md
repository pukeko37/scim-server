# Validation Configuration

This guide covers configurable validation rules that can be dynamically managed and applied at runtime. Instead of hardcoding validation logic, you can define rules through configuration that can be updated without code changes.

## Configuration-Driven Validation

### Validation Configuration Structure

```rust
use scim_server::validation::{ValidationConfig, ValidationRule, RuleEngine};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationConfig {
    pub tenant_id: String,
    pub rules: Vec<ValidationRule>,
    pub external_validators: Vec<ExternalValidatorConfig>,
    pub field_validators: HashMap<String, FieldValidatorConfig>,
    pub global_settings: GlobalValidationSettings,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationRule {
    pub id: String,
    pub name: String,
    pub description: String,
    pub enabled: bool,
    pub severity: ValidationSeverity,
    pub resource_types: Vec<String>, // ["User", "Group"]
    pub operations: Vec<String>,     // ["create", "update", "patch"]
    pub conditions: Vec<RuleCondition>,
    pub actions: Vec<ValidationAction>,
    pub priority: u32,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ValidationSeverity {
    Error,   // Blocks the operation
    Warning, // Logs but allows operation
    Info,    // Informational only
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleCondition {
    pub field: String,
    pub operator: ConditionOperator,
    pub value: serde_json::Value,
    pub case_sensitive: bool,
    pub negate: bool, // NOT condition
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConditionOperator {
    Equals,
    NotEquals,
    Contains,
    StartsWith,
    EndsWith,
    Regex,
    Length,
    GreaterThan,
    LessThan,
    In,
    NotIn,
    Exists,
    NotExists,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ValidationAction {
    Block { message: String },
    Warn { message: String },
    Log { level: String, message: String },
    Transform { field: String, transformation: String },
    Notify { recipients: Vec<String>, template: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlobalValidationSettings {
    pub max_validation_time_ms: u64,
    pub fail_fast: bool,
    pub enable_external_validation: bool,
    pub cache_validation_results: bool,
    pub cache_ttl_seconds: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExternalValidatorConfig {
    pub name: String,
    pub url: String,
    pub timeout_ms: u64,
    pub retry_count: u32,
    pub headers: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldValidatorConfig {
    pub validator_type: String,
    pub config: serde_json::Value,
    pub enabled: bool,
}
```

## Rule Engine Implementation

### Core Rule Engine

```rust
use async_trait::async_trait;
use std::time::{Duration, Instant};
use tokio::time::timeout;

pub struct RuleEngine {
    config: ValidationConfig,
    cache: Option<ValidationCache>,
}

impl RuleEngine {
    pub fn new(config: ValidationConfig) -> Self {
        let cache = if config.global_settings.cache_validation_results {
            Some(ValidationCache::new(
                Duration::from_secs(config.global_settings.cache_ttl_seconds)
            ))
        } else {
            None
        };

        Self { config, cache }
    }

    pub async fn validate_resource(
        &self,
        resource: &dyn ScimResource,
        context: &ValidationContext,
    ) -> Result<ValidationResult, ValidationError> {
        let start_time = Instant::now();
        let max_duration = Duration::from_millis(self.config.global_settings.max_validation_time_ms);

        // Check cache first
        if let Some(cache) = &self.cache {
            if let Some(cached_result) = cache.get(resource, context).await {
                return Ok(cached_result);
            }
        }

        // Run validation with timeout
        let validation_future = self.validate_internal(resource, context);
        let result = timeout(max_duration, validation_future)
            .await
            .map_err(|_| ValidationError::new(
                "VALIDATION_TIMEOUT",
                "Validation exceeded maximum allowed time",
            ))??;

        // Cache successful results
        if let Some(cache) = &self.cache {
            if result.is_valid() {
                cache.put(resource, context, &result).await;
            }
        }

        Ok(result)
    }

    async fn validate_internal(
        &self,
        resource: &dyn ScimResource,
        context: &ValidationContext,
    ) -> Result<ValidationResult, ValidationError> {
        let mut validation_result = ValidationResult::new();
        let applicable_rules = self.get_applicable_rules(resource, context);

        for rule in applicable_rules {
            if !rule.enabled {
                continue;
            }

            match self.evaluate_rule(rule, resource, context).await {
                Ok(rule_result) => {
                    validation_result.merge(rule_result);
                    
                    // Fail fast if enabled and we have errors
                    if self.config.global_settings.fail_fast && !validation_result.errors.is_empty() {
                        break;
                    }
                }
                Err(e) => {
                    validation_result.add_error(e);
                    if self.config.global_settings.fail_fast {
                        break;
                    }
                }
            }
        }

        Ok(validation_result)
    }

    fn get_applicable_rules(
        &self,
        resource: &dyn ScimResource,
        context: &ValidationContext,
    ) -> Vec<&ValidationRule> {
        let mut applicable_rules: Vec<&ValidationRule> = self.config.rules
            .iter()
            .filter(|rule| {
                // Filter by resource type
                rule.resource_types.is_empty() || 
                rule.resource_types.contains(&resource.resource_type())
            })
            .filter(|rule| {
                // Filter by operation
                rule.operations.is_empty() || 
                rule.operations.contains(&context.operation.to_string())
            })
            .collect();

        // Sort by priority (higher priority first)
        applicable_rules.sort_by(|a, b| b.priority.cmp(&a.priority));
        applicable_rules
    }

    async fn evaluate_rule(
        &self,
        rule: &ValidationRule,
        resource: &dyn ScimResource,
        context: &ValidationContext,
    ) -> Result<ValidationResult, ValidationError> {
        let mut rule_result = ValidationResult::new();

        // Evaluate all conditions (AND logic)
        let conditions_met = self.evaluate_conditions(&rule.conditions, resource, context).await?;

        if conditions_met {
            // Execute actions
            for action in &rule.actions {
                self.execute_action(action, rule, resource, context, &mut rule_result).await?;
            }
        }

        Ok(rule_result)
    }

    async fn evaluate_conditions(
        &self,
        conditions: &[RuleCondition],
        resource: &dyn ScimResource,
        context: &ValidationContext,
    ) -> Result<bool, ValidationError> {
        for condition in conditions {
            let condition_met = self.evaluate_condition(condition, resource, context).await?;
            let final_result = if condition.negate { !condition_met } else { condition_met };
            
            if !final_result {
                return Ok(false); // AND logic - all conditions must be true
            }
        }
        
        Ok(true)
    }

    async fn evaluate_condition(
        &self,
        condition: &RuleCondition,
        resource: &dyn ScimResource,
        _context: &ValidationContext,
    ) -> Result<bool, ValidationError> {
        let field_value = self.extract_field_value(&condition.field, resource)?;

        match &condition.operator {
            ConditionOperator::Equals => {
                Ok(self.compare_values(&field_value, &condition.value, condition.case_sensitive))
            }
            ConditionOperator::NotEquals => {
                Ok(!self.compare_values(&field_value, &condition.value, condition.case_sensitive))
            }
            ConditionOperator::Contains => {
                self.evaluate_contains(&field_value, &condition.value, condition.case_sensitive)
            }
            ConditionOperator::StartsWith => {
                self.evaluate_starts_with(&field_value, &condition.value, condition.case_sensitive)
            }
            ConditionOperator::EndsWith => {
                self.evaluate_ends_with(&field_value, &condition.value, condition.case_sensitive)
            }
            ConditionOperator::Regex => {
                self.evaluate_regex(&field_value, &condition.value)
            }
            ConditionOperator::Length => {
                self.evaluate_length(&field_value, &condition.value)
            }
            ConditionOperator::GreaterThan => {
                self.evaluate_greater_than(&field_value, &condition.value)
            }
            ConditionOperator::LessThan => {
                self.evaluate_less_than(&field_value, &condition.value)
            }
            ConditionOperator::In => {
                self.evaluate_in(&field_value, &condition.value, condition.case_sensitive)
            }
            ConditionOperator::NotIn => {
                Ok(!self.evaluate_in(&field_value, &condition.value, condition.case_sensitive)?)
            }
            ConditionOperator::Exists => {
                Ok(!field_value.is_null())
            }
            ConditionOperator::NotExists => {
                Ok(field_value.is_null())
            }
        }
    }

    fn extract_field_value(
        &self,
        field_path: &str,
        resource: &dyn ScimResource,
    ) -> Result<serde_json::Value, ValidationError> {
        let resource_json = serde_json::to_value(resource)
            .map_err(|e| ValidationError::new(
                "FIELD_EXTRACTION_ERROR",
                &format!("Failed to serialize resource: {}", e),
            ))?;

        self.extract_nested_value(&resource_json, field_path)
    }

    fn extract_nested_value(
        &self,
        value: &serde_json::Value,
        path: &str,
    ) -> Result<serde_json::Value, ValidationError> {
        let parts: Vec<&str> = path.split('.').collect();
        let mut current = value;

        for part in parts {
            // Handle array access like "emails[0].value"
            if let Some(bracket_pos) = part.find('[') {
                let field_name = &part[..bracket_pos];
                let index_part = &part[bracket_pos + 1..part.len() - 1];
                let index: usize = index_part.parse()
                    .map_err(|_| ValidationError::new(
                        "INVALID_ARRAY_INDEX",
                        &format!("Invalid array index: {}", index_part),
                    ))?;

                current = current.get(field_name)
                    .and_then(|v| v.as_array())
                    .and_then(|arr| arr.get(index))
                    .unwrap_or(&serde_json::Value::Null);
            } else {
                current = current.get(part).unwrap_or(&serde_json::Value::Null);
            }
        }

        Ok(current.clone())
    }

    // Condition evaluation helper methods
    fn compare_values(
        &self,
        field_value: &serde_json::Value,
        condition_value: &serde_json::Value,
        case_sensitive: bool,
    ) -> bool {
        if !case_sensitive {
            if let (Some(field_str), Some(condition_str)) = (field_value.as_str(), condition_value.as_str()) {
                return field_str.to_lowercase() == condition_str.to_lowercase();
            }
        }
        field_value == condition_value
    }

    fn evaluate_contains(
        &self,
        field_value: &serde_json::Value,
        condition_value: &serde_json::Value,
        case_sensitive: bool,
    ) -> Result<bool, ValidationError> {
        if let (Some(field_str), Some(condition_str)) = (field_value.as_str(), condition_value.as_str()) {
            if case_sensitive {
                Ok(field_str.contains(condition_str))
            } else {
                Ok(field_str.to_lowercase().contains(&condition_str.to_lowercase()))
            }
        } else {
            Ok(false)
        }
    }

    fn evaluate_starts_with(
        &self,
        field_value: &serde_json::Value,
        condition_value: &serde_json::Value,
        case_sensitive: bool,
    ) -> Result<bool, ValidationError> {
        if let (Some(field_str), Some(condition_str)) = (field_value.as_str(), condition_value.as_str()) {
            if case_sensitive {
                Ok(field_str.starts_with(condition_str))
            } else {
                Ok(field_str.to_lowercase().starts_with(&condition_str.to_lowercase()))
            }
        } else {
            Ok(false)
        }
    }

    fn evaluate_ends_with(
        &self,
        field_value: &serde_json::Value,
        condition_value: &serde_json::Value,
        case_sensitive: bool,
    ) -> Result<bool, ValidationError> {
        if let (Some(field_str), Some(condition_str)) = (field_value.as_str(), condition_value.as_str()) {
            if case_sensitive {
                Ok(field_str.ends_with(condition_str))
            } else {
                Ok(field_str.to_lowercase().ends_with(&condition_str.to_lowercase()))
            }
        } else {
            Ok(false)
        }
    }

    fn evaluate_regex(
        &self,
        field_value: &serde_json::Value,
        condition_value: &serde_json::Value,
    ) -> Result<bool, ValidationError> {
        if let (Some(field_str), Some(pattern_str)) = (field_value.as_str(), condition_value.as_str()) {
            let regex = regex::Regex::new(pattern_str)
                .map_err(|e| ValidationError::new(
                    "INVALID_REGEX",
                    &format!("Invalid regex pattern: {}", e),
                ))?;
            Ok(regex.is_match(field_str))
        } else {
            Ok(false)
        }
    }

    fn evaluate_length(
        &self,
        field_value: &serde_json::Value,
        condition_value: &serde_json::Value,
    ) -> Result<bool, ValidationError> {
        let field_length = match field_value {
            serde_json::Value::String(s) => s.len(),
            serde_json::Value::Array(arr) => arr.len(),
            _ => return Ok(false),
        };

        if let Some(expected_length) = condition_value.as_u64() {
            Ok(field_length == expected_length as usize)
        } else {
            Ok(false)
        }
    }

    fn evaluate_greater_than(
        &self,
        field_value: &serde_json::Value,
        condition_value: &serde_json::Value,
    ) -> Result<bool, ValidationError> {
        match (field_value.as_f64(), condition_value.as_f64()) {
            (Some(field_num), Some(condition_num)) => Ok(field_num > condition_num),
            _ => Ok(false),
        }
    }

    fn evaluate_less_than(
        &self,
        field_value: &serde_json::Value,
        condition_value: &serde_json::Value,
    ) -> Result<bool, ValidationError> {
        match (field_value.as_f64(), condition_value.as_f64()) {
            (Some(field_num), Some(condition_num)) => Ok(field_num < condition_num),
            _ => Ok(false),
        }
    }

    fn evaluate_in(
        &self,
        field_value: &serde_json::Value,
        condition_value: &serde_json::Value,
        case_sensitive: bool,
    ) -> Result<bool, ValidationError> {
        if let Some(values_array) = condition_value.as_array() {
            for value in values_array {
                if self.compare_values(field_value, value, case_sensitive) {
                    return Ok(true);
                }
            }
        }
        Ok(false)
    }

    async fn execute_action(
        &self,
        action: &ValidationAction,
        rule: &ValidationRule,
        resource: &dyn ScimResource,
        context: &ValidationContext,
        result: &mut ValidationResult,
    ) -> Result<(), ValidationError> {
        match action {
            ValidationAction::Block { message } => {
                let error = ValidationError::new(
                    &format!("RULE_VIOLATION_{}", rule.id.to_uppercase()),
                    message,
                ).with_severity(ValidationSeverity::Error);
                result.add_error(error);
            }
            ValidationAction::Warn { message } => {
                let warning = ValidationError::new(
                    &format!("RULE_WARNING_{}", rule.id.to_uppercase()),
                    message,
                ).with_severity(ValidationSeverity::Warning);
                result.add_warning(warning);
            }
            ValidationAction::Log { level, message } => {
                self.log_validation_event(level, message, rule, resource, context).await;
            }
            ValidationAction::Transform { field, transformation } => {
                // Transform actions would modify the resource
                // This is advanced functionality that requires careful implementation
                self.apply_transformation(field, transformation, resource, result).await?;
            }
            ValidationAction::Notify { recipients, template } => {
                self.send_notification(recipients, template, rule, resource, context).await?;
            }
        }
        Ok(())
    }

    async fn log_validation_event(
        &self,
        level: &str,
        message: &str,
        rule: &ValidationRule,
        resource: &dyn ScimResource,
        context: &ValidationContext,
    ) {
        // Implementation would depend on your logging system
        match level {
            "error" => log::error!("Validation rule '{}': {} (resource: {}, tenant: {})", 
                rule.name, message, resource.id().unwrap_or("unknown"), context.tenant_id),
            "warn" => log::warn!("Validation rule '{}': {} (resource: {}, tenant: {})", 
                rule.name, message, resource.id().unwrap_or("unknown"), context.tenant_id),
            "info" => log::info!("Validation rule '{}': {} (resource: {}, tenant: {})", 
                rule.name, message, resource.id().unwrap_or("unknown"), context.tenant_id),
            _ => log::debug!("Validation rule '{}': {} (resource: {}, tenant: {})", 
                rule.name, message, resource.id().unwrap_or("unknown"), context.tenant_id),
        }
    }

    async fn apply_transformation(
        &self,
        _field: &str,
        _transformation: &str,
        _resource: &dyn ScimResource,
        _result: &mut ValidationResult,
    ) -> Result<(), ValidationError> {
        // Transformation implementation would be complex and resource-specific
        // For now, we'll just log that a transformation was requested
        log::info!("Transformation requested but not implemented");
        Ok(())
    }

    async fn send_notification(
        &self,
        _recipients: &[String],
        _template: &str,
        _rule: &ValidationRule,
        _resource: &dyn ScimResource,
        _context: &ValidationContext,
    ) -> Result<(), ValidationError> {
        // Notification implementation would depend on your notification system
        log::info!("Notification requested but not implemented");
        Ok(())
    }
}
```

## Configuration Examples

### Sample Validation Configuration

```yaml
# validation-config.yaml
tenant_id: "company-123"

global_settings:
  max_validation_time_ms: 5000
  fail_fast: false
  enable_external_validation: true
  cache_validation_results: true
  cache_ttl_seconds: 300

rules:
  - id: "email_domain_check"
    name: "Email Domain Validation"
    description: "Ensure users have company email domains"
    enabled: true
    severity: "Error"
    resource_types: ["User"]
    operations: ["create", "update"]
    priority: 100
    conditions:
      - field: "emails[0].value"
        operator: "Regex"
        value: ".*@(company\\.com|subsidiary\\.com)$"
        case_sensitive: false
        negate: false
    actions:
      - Block:
          message: "Users must have a company email address (@company.com or @subsidiary.com)"

  - id: "manager_hierarchy_check"
    name: "Manager Hierarchy Validation"
    description: "Prevent circular manager relationships"
    enabled: true
    severity: "Error"
    resource_types: ["User"]
    operations: ["update", "patch"]
    priority: 90
    conditions:
      - field: "enterpriseUser.manager.value"
        operator: "Exists"
        value: null
        case_sensitive: false
        negate: false
    actions:
      - Block:
          message: "Manager assignment would create a circular reference"
      - Log:
          level: "warn"
          message: "Attempted circular manager assignment detected"

  - id: "contractor_end_date"
    name: "Contractor End Date Required"
    description: "Contractors must have employment end date"
    enabled: true
    severity: "Error"
    resource_types: ["User"]
    operations: ["create", "update"]
    priority: 80
    conditions:
      - field: "enterpriseUser.employeeType"
        operator: "Equals"
        value: "Contractor"
        case_sensitive: false
        negate: false
      - field: "enterpriseUser.employmentEndDate"
        operator: "NotExists"
        value: null
        case_sensitive: false
        negate: false
    actions:
      - Block:
          message: "Contractors must have an employment end date specified"

  - id: "vip_user_notification"
    name: "VIP User Notification"
    description: "Notify security team when VIP users are modified"
    enabled: true
    severity: "Info"
    resource_types: ["User"]
    operations: ["create", "update", "delete"]
    priority: 50
    conditions:
      - field: "enterpriseUser.vipStatus"
        operator: "Equals"
        value: true
        case_sensitive: false
        negate: false
    actions:
      - Log:
          level: "info"
          message: "VIP user account modified"
      - Notify:
          recipients: ["security@company.com", "compliance@company.com"]
          template: "vip_user_modification"

  - id: "username_format"
    name: "Username Format Validation"
    description: "Enforce username format standards"
    enabled: true
    severity: "Warning"
    resource_types: ["User"]
    operations: ["create", "update"]
    priority: 70
    conditions:
      - field: "userName"
        operator: "Regex"
        value: "^[a-z]+\\.[a-z]+$"
        case_sensitive: false
        negate: true
    actions:
      - Warn:
          message: "Username should follow format: firstname.lastname (lowercase, no numbers or special characters)"

field_validators:
  phoneNumbers:
    validator_type: "PhoneNumberValidator"
    enabled: true
    config:
      allowed_countries: ["US", "CA", "GB"]
      external_validation: true

  "enterpriseUser:ssn":
    validator_type: "SsnValidator"
    enabled: true
    config:
      allow_itin: false
      check_uniqueness: true

external_validators:
  - name: "hr_system"
    url: "https://hr.company.com/api/validate"
    timeout_ms: 3000
    retry_count: 2
    headers:
      Authorization: "Bearer ${HR_API_TOKEN}"
      Content-Type: "application/json"
```

### Loading Configuration

```rust
use serde_yaml;
use std::fs;

pub struct ConfigurationLoader;

impl ConfigurationLoader {
    pub fn load_from_file(file_path: &str) -> Result<ValidationConfig, Box<dyn std::error::Error>> {
        let config_content = fs::read_to_string(file_path)?;
        let config: ValidationConfig = serde_yaml::from_str(&config_content)?;
        Ok(config)
    }

    pub fn load_from_env() -> Result<ValidationConfig, Box<dyn std::error::Error>> {
        let config_path = std::env::var("VALIDATION_CONFIG_PATH")
            .unwrap_or_else(|_| "validation-config.yaml".to_string());
        Self::load_from_file(&config_path)
    }

    pub async fn load_from_database(
        tenant_id: &str,
        storage: &dyn StorageProvider,
    ) -> Result<ValidationConfig, Box<dyn std::error::Error>> {
        // Load configuration from database
        let config_json = storage.get_tenant_config(tenant_id, "validation").await?;
        let config: ValidationConfig = serde_json::from_str(&config_json)?;
        Ok(config)
    }
}
```

## Dynamic Rule Management

### Rule Management API

```rust
use scim_server::validation::{ValidationConfig, ValidationRule, RuleEngine};

pub struct ValidationRuleManager {
    storage: Box<dyn StorageProvider>,
    rule_engines: std::sync::RwLock<HashMap<String, RuleEngine>>, // tenant_id -> RuleEngine
}

impl ValidationRuleManager {
    pub fn new(storage: Box<dyn StorageProvider>) -> Self {
        Self {
            storage,
            rule_engines: std::sync::RwLock::new(HashMap::new()),
        }
    }

    pub async fn add_rule(
        &self,
        tenant_id: &str,
        rule: ValidationRule,
    ) -> Result<(), ValidationError> {
        // Validate rule configuration
        self.validate_rule_config(&rule)?;

        // Save to storage
        self.storage.save_validation_rule(tenant_id, &rule).await?;

        // Reload configuration for tenant
        self.reload_tenant_config(tenant_id).await?;

        Ok(())
    }

    pub async fn update_rule(
        &self,
        tenant_id: &str,
        rule_id: &str,
        updated_rule: ValidationRule,
    ) -> Result<(), ValidationError> {
        if updated_rule.id != rule_id {
            return Err(ValidationError::new(
                "RULE_ID_MISMATCH",
                "Rule ID in path does not match rule ID in body",
            ));
        }

        self.validate_rule_config(&updated_rule)?;
        self.storage.update_validation_rule(tenant_id, &updated_rule).await?;
        self.reload_tenant_config(tenant_id).await?;

        Ok(())
    }

    pub async fn delete_rule(
        &self,
        tenant_id: &str,
        rule_id: &str,
    ) -> Result<(), ValidationError> {
        self.storage.delete_validation_rule(tenant_id, rule_id).await?;
        self.reload_tenant_config(tenant_id).await?;
        Ok(())
    }

    pub async fn toggle_rule(
        &self,
        tenant_id: &str,
        rule_id: &str,
        enabled: bool,
    ) -> Result<(), ValidationError> {
        self.storage.toggle_validation_rule(tenant_id, rule_id, enabled).await?;
        self.reload_tenant_config(tenant_id).await?;
        Ok(())
    }

    pub async fn get_rule_engine(&self, tenant_id: &str) -> Result<RuleEngine, ValidationError> {
        // Check if we have a cached rule engine
        {
            let engines = self.rule_engines.read().unwrap();
            if let Some(engine) = engines.get(tenant_id) {
                return Ok(engine.clone());
            }
        }

        // Load configuration and create new engine
        let config = self.load_tenant_config(tenant_id).await?;
        let engine = RuleEngine::new(config);

        // Cache the engine
        {
            let mut engines = self.rule_engines.write().unwrap();
            engines.insert(tenant_id.to_string(), engine.clone());
        }

        Ok(engine)
    }

    async fn reload_tenant_config(&self, tenant_id: &str) -> Result<(), ValidationError> {
        let config = self.load_tenant_config(tenant_id).await?;
        let engine = RuleEngine::new(config);

        let mut engines = self.rule_engines.write().unwrap();
        engines.insert(tenant_id.to_string(), engine);

        Ok(())
    }

    async fn load_tenant_config(&self, tenant_id: &str) -> Result<ValidationConfig, ValidationError> {
        self.storage.get_validation_config(tenant_id).await
            .map_err(|e| ValidationError::new(
                "CONFIG_LOAD_ERROR",
                &format!("Failed to load validation config: {}", e),
            ))
    }

    fn validate_rule_config(&self, rule: &ValidationRule) -> Result<(), ValidationError> {
        // Validate rule ID
        if rule.id.is_empty() {
            return Err(ValidationError::new(
                "INVALID_RULE_ID",
                "Rule ID cannot be empty",
            ));
        }

        // Validate conditions
        for condition in &rule.conditions {
            if condition.field.is_empty() {
                return Err(ValidationError::new(
                    "INVALID_CONDITION_FIELD",
                    "Condition field cannot be empty",
                ));
            }

            // Validate regex patterns
            if matches!(condition.operator, ConditionOperator::Regex) {
                if let Some(pattern) = condition.value.as_str() {
                    regex::Regex::new(pattern).map_err(|e| ValidationError::new(
                        "INVALID_REGEX_PATTERN",
                        &format!("Invalid regex pattern: {}", e),
                    ))?;
                }
            }
        }

        // Validate actions
        if rule.actions.is_empty() {
            return Err(ValidationError::new(
                "NO_ACTIONS_DEFINED",
                "Rule must have at least one action",
            ));
        }

        Ok(())
    }
}
```

## Usage Examples

### Basic Usage

```rust
use scim_server::validation::{ValidationRuleManager, RuleEngine};

#[tokio::main]
async fn main() -> Result<(), Box