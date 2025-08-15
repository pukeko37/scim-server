# Advanced Validation

This guide covers complex validation scenarios including external system integration, conditional validation, and sophisticated business logic that requires asynchronous operations or external dependencies.

## External System Integration

### HR System Validation

Validate users against external HR systems to ensure data consistency:

```rust
use scim_server::validation::{CustomValidator, ValidationContext, ValidationError};
use reqwest::Client;
use serde_json::json;
use std::time::Duration;
use async_trait::async_trait;

pub struct ExternalValidationService {
    http_client: Client,
    hr_system_url: String,
    compliance_service_url: String,
    api_key: String,
}

impl ExternalValidationService {
    pub fn new(hr_system_url: String, compliance_service_url: String, api_key: String) -> Self {
        Self {
            http_client: Client::new(),
            hr_system_url,
            compliance_service_url,
            api_key,
        }
    }
}

#[async_trait]
impl CustomValidator for ExternalValidationService {
    async fn validate_user(
        &self,
        user: &User,
        context: &ValidationContext,
    ) -> Result<(), ValidationError> {
        // Validate against HR system
        self.validate_against_hr_system(user).await?;
        
        // Validate compliance requirements
        self.validate_compliance_requirements(user, context).await?;
        
        // Validate security clearance if present
        if let Some(security_clearance) = self.extract_security_clearance(user) {
            self.validate_security_clearance(&security_clearance, user).await?;
        }
        
        Ok(())
    }

    async fn validate_group(
        &self,
        group: &Group,
        context: &ValidationContext,
    ) -> Result<(), ValidationError> {
        // Validate group against organizational structure
        self.validate_group_structure(group, context).await?;
        Ok(())
    }
}

impl ExternalValidationService {
    async fn validate_against_hr_system(&self, user: &User) -> Result<(), ValidationError> {
        if let Some(employee_number) = self.extract_employee_number(user) {
            let response = self.http_client
                .get(&format!("{}/employees/{}", self.hr_system_url, employee_number))
                .header("Authorization", format!("Bearer {}", self.api_key))
                .timeout(Duration::from_secs(5))
                .send()
                .await
                .map_err(|e| ValidationError::new(
                    "HR_SYSTEM_ERROR",
                    &format!("Failed to validate employee: {}", e),
                ))?;
            
            if response.status() == 404 {
                return Err(ValidationError::new(
                    "EMPLOYEE_NOT_FOUND",
                    "Employee not found in HR system",
                ).with_field("enterpriseUser:employeeNumber"));
            }
            
            if !response.status().is_success() {
                return Err(ValidationError::new(
                    "HR_SYSTEM_ERROR",
                    &format!("HR system returned status: {}", response.status()),
                ));
            }
            
            let hr_employee: HrEmployee = response.json().await
                .map_err(|e| ValidationError::new(
                    "HR_SYSTEM_ERROR",
                    &format!("Failed to parse HR response: {}", e),
                ))?;
            
            // Validate employee status
            if hr_employee.status != "ACTIVE" {
                return Err(ValidationError::new(
                    "EMPLOYEE_INACTIVE",
                    &format!("Employee status in HR system is: {}", hr_employee.status),
                ).with_field("active"));
            }
            
            // Validate department consistency
            if let Some(department) = self.extract_department(user) {
                if hr_employee.department != department {
                    return Err(ValidationError::new(
                        "DEPARTMENT_MISMATCH",
                        "Department does not match HR system",
                    ).with_field("enterpriseUser:department"));
                }
            }

            // Validate manager hierarchy
            if let Some(manager_id) = self.extract_manager_id(user) {
                if let Some(hr_manager_id) = hr_employee.manager_id {
                    if manager_id != hr_manager_id {
                        return Err(ValidationError::new(
                            "MANAGER_MISMATCH",
                            "Manager does not match HR system",
                        ).with_field("enterpriseUser:manager"));
                    }
                }
            }
        }
        
        Ok(())
    }
    
    async fn validate_compliance_requirements(
        &self,
        user: &User,
        context: &ValidationContext,
    ) -> Result<(), ValidationError> {
        let compliance_request = json!({
            "user_data": {
                "name": user.name,
                "emails": user.emails,
                "phone_numbers": user.phone_numbers,
                "addresses": user.addresses,
                "country": self.extract_country(user),
            },
            "tenant_id": context.tenant_id,
            "operation": context.operation,
        });
        
        let response = self.http_client
            .post(&format!("{}/validate", self.compliance_service_url))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&compliance_request)
            .timeout(Duration::from_secs(10))
            .send()
            .await
            .map_err(|e| ValidationError::new(
                "COMPLIANCE_SYSTEM_ERROR",
                &format!("Failed to validate compliance: {}", e),
            ))?;
        
        if !response.status().is_success() {
            return Err(ValidationError::new(
                "COMPLIANCE_SYSTEM_ERROR",
                &format!("Compliance service returned status: {}", response.status()),
            ));
        }
        
        let compliance_result: ComplianceValidationResult = response.json().await
            .map_err(|e| ValidationError::new(
                "COMPLIANCE_SYSTEM_ERROR",
                &format!("Failed to parse compliance response: {}", e),
            ))?;
        
        if !compliance_result.is_compliant {
            let violations = compliance_result.violations.join(", ");
            return Err(ValidationError::new(
                "COMPLIANCE_VIOLATION",
                &format!("Compliance violations: {}", violations),
            ));
        }
        
        Ok(())
    }
    
    async fn validate_security_clearance(
        &self,
        security_clearance: &str,
        user: &User,
    ) -> Result<(), ValidationError> {
        // Validate security clearance levels
        let valid_clearances = ["PUBLIC", "CONFIDENTIAL", "SECRET", "TOP_SECRET"];
        if !valid_clearances.contains(&security_clearance) {
            return Err(ValidationError::new(
                "INVALID_SECURITY_CLEARANCE",
                &format!("Invalid security clearance level: {}", security_clearance),
            ).with_field("enterpriseUser:securityClearance"));
        }
        
        // Validate clearance requirements based on department
        if let Some(department) = self.extract_department(user) {
            match department.as_str() {
                "Defense" | "Intelligence" => {
                    if security_clearance == "PUBLIC" {
                        return Err(ValidationError::new(
                            "INSUFFICIENT_CLEARANCE",
                            "Department requires minimum CONFIDENTIAL clearance",
                        ).with_field("enterpriseUser:securityClearance"));
                    }
                }
                "Research" => {
                    if !["CONFIDENTIAL", "SECRET", "TOP_SECRET"].contains(&security_clearance) {
                        return Err(ValidationError::new(
                            "INSUFFICIENT_CLEARANCE",
                            "Research department requires minimum CONFIDENTIAL clearance",
                        ).with_field("enterpriseUser:securityClearance"));
                    }
                }
                _ => {} // No special requirements
            }
        }
        
        Ok(())
    }

    async fn validate_group_structure(
        &self,
        group: &Group,
        context: &ValidationContext,
    ) -> Result<(), ValidationError> {
        // Validate against organizational chart
        let org_response = self.http_client
            .get(&format!("{}/organizational-chart/{}", self.hr_system_url, context.tenant_id))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .send()
            .await
            .map_err(|e| ValidationError::new(
                "ORG_CHART_ERROR",
                &format!("Failed to fetch organizational chart: {}", e),
            ))?;

        let org_chart: OrganizationalChart = org_response.json().await
            .map_err(|e| ValidationError::new(
                "ORG_CHART_ERROR", 
                &format!("Failed to parse org chart: {}", e),
            ))?;

        // Validate group exists in org chart
        if !org_chart.groups.iter().any(|g| g.name == group.display_name) {
            return Err(ValidationError::new(
                "GROUP_NOT_IN_ORG_CHART",
                "Group does not exist in organizational chart",
            ).with_field("displayName"));
        }

        Ok(())
    }

    // Helper methods for extracting user attributes
    fn extract_employee_number(&self, user: &User) -> Option<String> {
        user.extension_attributes
            .as_ref()
            .and_then(|attrs| attrs.get("employeeNumber"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
    }

    fn extract_department(&self, user: &User) -> Option<String> {
        user.extension_attributes
            .as_ref()
            .and_then(|attrs| attrs.get("department"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
    }

    fn extract_manager_id(&self, user: &User) -> Option<String> {
        user.extension_attributes
            .as_ref()
            .and_then(|attrs| attrs.get("manager"))
            .and_then(|v| v.get("value"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
    }

    fn extract_security_clearance(&self, user: &User) -> Option<String> {
        user.extension_attributes
            .as_ref()
            .and_then(|attrs| attrs.get("securityClearance"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
    }

    fn extract_country(&self, user: &User) -> Option<String> {
        user.addresses
            .as_ref()
            .and_then(|addrs| addrs.first())
            .map(|addr| addr.country.clone())
            .unwrap_or_default()
    }
}

// Supporting types
#[derive(serde::Deserialize)]
struct HrEmployee {
    employee_id: String,
    status: String,
    department: String,
    manager_id: Option<String>,
    hire_date: String,
    termination_date: Option<String>,
}

#[derive(serde::Deserialize)]
struct ComplianceValidationResult {
    is_compliant: bool,
    violations: Vec<String>,
    severity: String,
}

#[derive(serde::Deserialize)]
struct OrganizationalChart {
    groups: Vec<OrgGroup>,
}

#[derive(serde::Deserialize)]
struct OrgGroup {
    name: String,
    parent: Option<String>,
    level: u32,
}
```

## Conditional Validation

Implement validation rules that apply only under specific conditions:

```rust
use scim_server::validation::{CustomValidator, ValidationContext, ValidationError};
use std::collections::HashMap;

pub struct ConditionalValidator {
    rules: Vec<ConditionalRule>,
}

pub struct ConditionalRule {
    pub name: String,
    pub condition: fn(&User, &ValidationContext) -> bool,
    pub validator: fn(&User, &ValidationContext) -> Result<(), ValidationError>,
}

impl ConditionalValidator {
    pub fn new() -> Self {
        let mut rules = Vec::new();
        
        // Rule: Contractors must have end date
        rules.push(ConditionalRule {
            name: "contractor_end_date".to_string(),
            condition: |user, _| {
                user.extension_attributes
                    .as_ref()
                    .and_then(|attrs| attrs.get("userType"))
                    .and_then(|v| v.as_str()) == Some("Contractor")
            },
            validator: |user, _| {
                let has_end_date = user.extension_attributes
                    .as_ref()
                    .and_then(|attrs| attrs.get("employmentEndDate"))
                    .is_some();
                    
                if !has_end_date {
                    return Err(ValidationError::new(
                        "MISSING_END_DATE",
                        "Contractors must have an employment end date",
                    ).with_field("enterpriseUser:employmentEndDate"));
                }
                Ok(())
            },
        });
        
        // Rule: VIP users require additional security
        rules.push(ConditionalRule {
            name: "vip_security_requirements".to_string(),
            condition: |user, _| {
                user.extension_attributes
                    .as_ref()
                    .and_then(|attrs| attrs.get("vipStatus"))
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false)
            },
            validator: |user, _| {
                // Check for required security attributes
                let security_attrs = ["securityClearance", "backgroundCheckDate", "securityTraining"];
                
                for attr in &security_attrs {
                    if !user.extension_attributes
                        .as_ref()
                        .map(|attrs| attrs.contains_key(*attr))
                        .unwrap_or(false) {
                        return Err(ValidationError::new(
                            "MISSING_VIP_SECURITY_ATTR",
                            &format!("VIP users must have {} attribute", attr),
                        ).with_field(&format!("enterpriseUser:{}", attr)));
                    }
                }
                Ok(())
            },
        });
        
        // Rule: Remote workers require specific equipment
        rules.push(ConditionalRule {
            name: "remote_worker_equipment".to_string(),
            condition: |user, _| {
                user.extension_attributes
                    .as_ref()
                    .and_then(|attrs| attrs.get("workLocation"))
                    .and_then(|v| v.as_str()) == Some("Remote")
            },
            validator: |user, _| {
                let required_equipment = ["laptop", "vpnAccess", "phoneStipend"];
                
                for equipment in &required_equipment {
                    if !user.extension_attributes
                        .as_ref()
                        .and_then(|attrs| attrs.get("equipment"))
                        .and_then(|v| v.as_array())
                        .map(|arr| arr.iter().any(|item| 
                            item.as_str().map(|s| s == *equipment).unwrap_or(false)
                        ))
                        .unwrap_or(false) {
                        return Err(ValidationError::new(
                            "MISSING_REMOTE_EQUIPMENT",
                            &format!("Remote workers must have {} assigned", equipment),
                        ).with_field("enterpriseUser:equipment"));
                    }
                }
                Ok(())
            },
        });

        Self { rules }
    }
}

#[async_trait]
impl CustomValidator for ConditionalValidator {
    async fn validate_user(
        &self,
        user: &User,
        context: &ValidationContext,
    ) -> Result<(), ValidationError> {
        for rule in &self.rules {
            if (rule.condition)(user, context) {
                (rule.validator)(user, context)?;
            }
        }
        
        Ok(())
    }

    async fn validate_group(
        &self,
        _group: &Group,
        _context: &ValidationContext,
    ) -> Result<(), ValidationError> {
        // Groups don't typically need conditional validation
        Ok(())
    }
}
```

## Async Workflow Integration

Integrate with approval workflows and external processes:

```rust
use scim_server::validation::{CustomValidator, ValidationContext, ValidationError};
use tokio::time::{sleep, Duration};

pub struct WorkflowValidator {
    workflow_client: WorkflowClient,
    approval_timeout: Duration,
}

impl WorkflowValidator {
    pub fn new(workflow_url: String, api_key: String) -> Self {
        Self {
            workflow_client: WorkflowClient::new(workflow_url, api_key),
            approval_timeout: Duration::from_secs(30),
        }
    }
}

#[async_trait]
impl CustomValidator for WorkflowValidator {
    async fn validate_user(
        &self,
        user: &User,
        context: &ValidationContext,
    ) -> Result<(), ValidationError> {
        match context.operation {
            Operation::Create => {
                // Check if user creation requires approval
                if self.requires_approval(user, context).await? {
                    self.validate_approval_exists(user, context).await?;
                }
            }
            Operation::Update => {
                // Check for sensitive attribute changes
                if self.has_sensitive_changes(user, context).await? {
                    self.validate_change_approval(user, context).await?;
                }
            }
            _ => {}
        }
        
        Ok(())
    }

    async fn validate_group(
        &self,
        group: &Group,
        context: &ValidationContext,
    ) -> Result<(), ValidationError> {
        // Group creation/modification might require approval for certain types
        if self.is_privileged_group(group) {
            self.validate_group_approval(group, context).await?;
        }
        
        Ok(())
    }
}

impl WorkflowValidator {
    async fn requires_approval(&self, user: &User, context: &ValidationContext) -> Result<bool, ValidationError> {
        // External users always require approval
        if self.is_external_user(user) {
            return Ok(true);
        }
        
        // High-privilege roles require approval
        if let Some(roles) = &user.roles {
            let privileged_roles = ["Admin", "Security", "HR"];
            if roles.iter().any(|role| privileged_roles.contains(&role.value.as_str())) {
                return Ok(true);
            }
        }
        
        // Users with high security clearance require approval
        if let Some(clearance) = user.extension_attributes
            .as_ref()
            .and_then(|attrs| attrs.get("securityClearance"))
            .and_then(|v| v.as_str()) {
            if ["SECRET", "TOP_SECRET"].contains(&clearance) {
                return Ok(true);
            }
        }
        
        Ok(false)
    }

    async fn validate_approval_exists(
        &self,
        user: &User,
        context: &ValidationContext,
    ) -> Result<(), ValidationError> {
        let approval_request = ApprovalRequest {
            request_type: "user_creation".to_string(),
            tenant_id: context.tenant_id.clone(),
            requester: context.authenticated_user.clone().unwrap_or_default(),
            subject: user.username.clone().unwrap_or_default(),
            details: serde_json::to_value(user).unwrap_or_default(),
        };

        let approval_status = self.workflow_client
            .check_approval_status(&approval_request)
            .await
            .map_err(|e| ValidationError::new(
                "WORKFLOW_ERROR",
                &format!("Failed to check approval status: {}", e),
            ))?;

        match approval_status.status.as_str() {
            "approved" => Ok(()),
            "pending" => {
                // Wait for approval with timeout
                self.wait_for_approval(&approval_request).await
            }
            "rejected" => {
                Err(ValidationError::new(
                    "APPROVAL_REJECTED",
                    &format!("User creation was rejected: {}", approval_status.reason.unwrap_or_default()),
                ))
            }
            _ => {
                // Create new approval request
                self.workflow_client
                    .create_approval_request(&approval_request)
                    .await
                    .map_err(|e| ValidationError::new(
                        "WORKFLOW_ERROR",
                        &format!("Failed to create approval request: {}", e),
                    ))?;
                
                Err(ValidationError::new(
                    "APPROVAL_PENDING",
                    "User creation requires approval. Request has been submitted.",
                ))
            }
        }
    }

    async fn wait_for_approval(&self, request: &ApprovalRequest) -> Result<(), ValidationError> {
        let mut attempts = 0;
        let max_attempts = (self.approval_timeout.as_secs() / 5) as usize; // Check every 5 seconds

        while attempts < max_attempts {
            sleep(Duration::from_secs(5)).await;
            
            let status = self.workflow_client
                .check_approval_status(request)
                .await
                .map_err(|e| ValidationError::new(
                    "WORKFLOW_ERROR",
                    &format!("Failed to check approval status: {}", e),
                ))?;

            match status.status.as_str() {
                "approved" => return Ok(()),
                "rejected" => return Err(ValidationError::new(
                    "APPROVAL_REJECTED",
                    &format!("Request was rejected: {}", status.reason.unwrap_or_default()),
                )),
                "pending" => {
                    attempts += 1;
                    continue;
                }
                _ => return Err(ValidationError::new(
                    "WORKFLOW_ERROR",
                    "Unexpected approval status",
                )),
            }
        }

        Err(ValidationError::new(
            "APPROVAL_TIMEOUT",
            "Approval request timed out",
        ))
    }

    async fn has_sensitive_changes(&self, user: &User, context: &ValidationContext) -> Result<bool, ValidationError> {
        // This would typically compare with the existing user record
        // For brevity, we'll assume sensitive attributes are being checked
        let sensitive_attributes = [
            "roles", "permissions", "securityClearance", 
            "department", "manager", "salary"
        ];
        
        // In a real implementation, you would fetch the existing user
        // and compare the attributes to detect changes
        Ok(true) // Simplified for example
    }

    async fn validate_change_approval(
        &self,
        user: &User,
        context: &ValidationContext,
    ) -> Result<(), ValidationError> {
        // Similar to validate_approval_exists but for user changes
        let approval_request = ApprovalRequest {
            request_type: "user_modification".to_string(),
            tenant_id: context.tenant_id.clone(),
            requester: context.authenticated_user.clone().unwrap_or_default(),
            subject: user.username.clone().unwrap_or_default(),
            details: serde_json::to_value(user).unwrap_or_default(),
        };

        // Check for existing approval or create new request
        self.validate_approval_exists(user, context).await
    }

    async fn validate_group_approval(
        &self,
        group: &Group,
        context: &ValidationContext,
    ) -> Result<(), ValidationError> {
        let approval_request = ApprovalRequest {
            request_type: "privileged_group_creation".to_string(),
            tenant_id: context.tenant_id.clone(),
            requester: context.authenticated_user.clone().unwrap_or_default(),
            subject: group.display_name.clone(),
            details: serde_json::to_value(group).unwrap_or_default(),
        };

        self.validate_approval_exists_for_group(group, &approval_request).await
    }

    async fn validate_approval_exists_for_group(
        &self,
        group: &Group,
        approval_request: &ApprovalRequest,
    ) -> Result<(), ValidationError> {
        // Similar logic to user approval validation
        let approval_status = self.workflow_client
            .check_approval_status(approval_request)
            .await
            .map_err(|e| ValidationError::new(
                "WORKFLOW_ERROR",
                &format!("Failed to check group approval status: {}", e),
            ))?;

        match approval_status.status.as_str() {
            "approved" => Ok(()),
            "pending" => self.wait_for_approval(approval_request).await,
            "rejected" => Err(ValidationError::new(
                "GROUP_APPROVAL_REJECTED",
                &format!("Group creation was rejected: {}", approval_status.reason.unwrap_or_default()),
            )),
            _ => {
                self.workflow_client
                    .create_approval_request(approval_request)
                    .await
                    .map_err(|e| ValidationError::new(
                        "WORKFLOW_ERROR",
                        &format!("Failed to create group approval request: {}", e),
                    ))?;
                
                Err(ValidationError::new(
                    "GROUP_APPROVAL_PENDING",
                    "Privileged group creation requires approval. Request has been submitted.",
                ))
            }
        }
    }

    fn is_external_user(&self, user: &User) -> bool {
        if let Some(emails) = &user.emails {
            return emails.iter().any(|email| {
                !email.value.ends_with("@company.com") && 
                !email.value.ends_with("@subsidiary.com")
            });
        }
        false
    }

    fn is_privileged_group(&self, group: &Group) -> bool {
        let privileged_patterns = ["admin", "security", "hr", "finance", "executive"];
        privileged_patterns.iter().any(|pattern| {
            group.display_name.to_lowercase().contains(pattern)
        })
    }
}

// Supporting types and client
struct WorkflowClient {
    base_url: String,
    api_key: String,
    client: reqwest::Client,
}

impl WorkflowClient {
    fn new(base_url: String, api_key: String) -> Self {
        Self {
            base_url,
            api_key,
            client: reqwest::Client::new(),
        }
    }

    async fn check_approval_status(&self, request: &ApprovalRequest) -> Result<ApprovalStatus, Box<dyn std::error::Error>> {
        let response = self.client
            .get(&format!("{}/approvals/{}", self.base_url, request.get_id()))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .send()
            .await?;
        
        Ok(response.json().await?)
    }

    async fn create_approval_request(&self, request: &ApprovalRequest) -> Result<ApprovalStatus, Box<dyn std::error::Error>> {
        let response = self.client
            .post(&format!("{}/approvals", self.base_url))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(request)
            .send()
            .await?;
        
        Ok(response.json().await?)
    }
}

#[derive(serde::Serialize, serde::Deserialize)]
struct ApprovalRequest {
    request_type: String,
    tenant_id: String,
    requester: String,
    subject: String,
    details: serde_json::Value,
}

impl ApprovalRequest {
    fn get_id(&self) -> String {
        // Generate ID based on request content
        format!("{}_{}_{}_{}", 
            self.request_type, 
            self.tenant_id, 
            self.requester, 
            self.subject
        )
    }
}

#[derive(serde::Deserialize)]
struct ApprovalStatus {
    status: String,
    reason: Option<String>,
    approved_by: Option<String>,
    approved_at: Option<String>,
}
```

## Testing Advanced Validators

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use tokio_test;
    use wiremock::{MockServer, Mock, ResponseTemplate};

    #[tokio::test]
    async fn test_external_validation_service() {
        // Setup mock HR system
        let mock_server = MockServer::start().await;
        
        Mock::given(wiremock::matchers::method("GET"))
            .and(wiremock::matchers::path("/employees/EMP123456"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "employee_id": "EMP123456",
                "status": "ACTIVE",
                "department": "Engineering",
                "manager_id": "MGR789"
            })))
            .mount(&mock_server)
            .await;

        let validator = ExternalValidationService::new(
            mock_server.uri(),
            "http://compliance.test".to_string(),
            "test-api-key".to_string(),
        );

        let mut user = User::default();
        user.extension_attributes = Some(serde_json::json!({
            "employeeNumber": "EMP123456",
            "department": "Engineering"
        }).as_object().unwrap().clone());

        let context = ValidationContext::default();
        let result = validator.validate_user(&user, &context).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_conditional_validation() {
        let validator = ConditionalValidator::new();
        
        // Test contractor without end date
        let mut contractor = User::default();
        contractor.extension_attributes = Some(serde_json::json!({
            "userType": "Contractor"
        }).as_object().unwrap().clone());

        let context = ValidationContext::default();
        let result = validator.validate_user(&contractor, &context).await;
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().code, "MISSING_END_DATE");

        // Test contractor with end date
        contractor.extension_attributes = Some(serde_json::json!({
            "userType": "Contractor",
            "employmentEndDate": "2024-12-31"
        }).as_object().unwrap().clone());

        let result = validator.validate_user(&contractor, &context).await;
        assert!(result.is_ok());
    }
}
```

## Usage Examples

```rust
use scim_server::ScimServerBuilder;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let server = ScimServerBuilder::new()
        .with_provider(my_provider)
        .add_validator(ExternalValidationService::new(
            "https://hr.company.com/api".to_string(),
            "https://compliance.company.com/api".to_string(),
            std::env::var("API_KEY")?,
        ))
        .add_validator(ConditionalValidator::new())
        .add_validator(WorkflowValidator::new(
            "https://workflow.company.com/api".to_string(),
            std::env::var("WORKFLOW_API_KEY")?,
        ))
        .build();

    server.run().await?;
    Ok(())
}
```

## Next Steps

- [Field-