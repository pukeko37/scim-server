# Basic Validation

This guide covers implementing simple, synchronous validation rules for common business requirements. These validators typically perform local checks without external dependencies.

## Simple Business Rule Validator

Here's a comprehensive example of a basic validator that enforces common organizational policies:

```rust
use scim_server::validation::{CustomValidator, ValidationContext, ValidationError};
use scim_server::models::{User, Group};
use regex::Regex;
use async_trait::async_trait;

pub struct BusinessRuleValidator {
    employee_id_pattern: Regex,
    allowed_domains: Vec<String>,
    password_policy: PasswordPolicy,
}

#[derive(Clone)]
pub struct PasswordPolicy {
    pub min_length: usize,
    pub require_uppercase: bool,
    pub require_lowercase: bool,
    pub require_numbers: bool,
    pub require_special_chars: bool,
    pub forbidden_patterns: Vec<Regex>,
}

impl BusinessRuleValidator {
    pub fn new() -> Self {
        Self {
            employee_id_pattern: Regex::new(r"^EMP\d{6}$").unwrap(),
            allowed_domains: vec![
                "company.com".to_string(),
                "subsidiary.com".to_string(),
            ],
            password_policy: PasswordPolicy {
                min_length: 12,
                require_uppercase: true,
                require_lowercase: true,
                require_numbers: true,
                require_special_chars: true,
                forbidden_patterns: vec![
                    Regex::new(r"password").unwrap(),
                    Regex::new(r"123456").unwrap(),
                    Regex::new(r"qwerty").unwrap(),
                ],
            },
        }
    }
}

#[async_trait]
impl CustomValidator for BusinessRuleValidator {
    async fn validate_user(
        &self,
        user: &User,
        context: &ValidationContext,
    ) -> Result<(), ValidationError> {
        // Validate employee ID format
        if let Some(employee_id) = user.external_id.as_ref() {
            if !self.employee_id_pattern.is_match(employee_id) {
                return Err(ValidationError::new(
                    "INVALID_EMPLOYEE_ID",
                    "Employee ID must follow format EMP123456",
                ).with_field("externalId"));
            }
        }

        // Validate email domain
        if let Some(emails) = &user.emails {
            for (index, email) in emails.iter().enumerate() {
                if let Some(domain) = email.value.split('@').nth(1) {
                    if !self.allowed_domains.contains(&domain.to_lowercase()) {
                        return Err(ValidationError::new(
                            "INVALID_EMAIL_DOMAIN",
                            &format!("Email domain '{}' is not allowed", domain),
                        ).with_field(&format!("emails[{}].value", index)));
                    }
                }
            }
        }

        // Validate password policy (if password is being set)
        if let Some(password) = user.password.as_ref() {
            self.validate_password_policy(password)?;
        }

        // Validate name requirements
        if user.name.is_none() {
            return Err(ValidationError::new(
                "MISSING_NAME",
                "User must have a name",
            ).with_field("name"));
        }

        // Validate username format
        if let Some(username) = &user.username {
            if username.len() < 3 {
                return Err(ValidationError::new(
                    "USERNAME_TOO_SHORT",
                    "Username must be at least 3 characters",
                ).with_field("userName"));
            }
            
            if username.contains(' ') {
                return Err(ValidationError::new(
                    "USERNAME_INVALID_CHARS",
                    "Username cannot contain spaces",
                ).with_field("userName"));
            }
        }

        Ok(())
    }

    async fn validate_group(
        &self,
        group: &Group,
        context: &ValidationContext,
    ) -> Result<(), ValidationError> {
        // Validate group display name
        if group.display_name.is_empty() {
            return Err(ValidationError::new(
                "EMPTY_GROUP_NAME",
                "Group display name cannot be empty",
            ).with_field("displayName"));
        }

        // Validate group name format
        if group.display_name.len() > 64 {
            return Err(ValidationError::new(
                "GROUP_NAME_TOO_LONG",
                "Group display name cannot exceed 64 characters",
            ).with_field("displayName"));
        }

        // Check for reserved group names
        let reserved_names = vec!["admin", "root", "system", "administrator"];
        if reserved_names.contains(&group.display_name.to_lowercase().as_str()) {
            return Err(ValidationError::new(
                "RESERVED_GROUP_NAME",
                &format!("'{}' is a reserved group name", group.display_name),
            ).with_field("displayName"));
        }

        // Validate member limit
        if let Some(members) = &group.members {
            if members.len() > 1000 {
                return Err(ValidationError::new(
                    "TOO_MANY_MEMBERS",
                    "Group cannot have more than 1000 members",
                ).with_field("members"));
            }
        }

        Ok(())
    }
}

impl BusinessRuleValidator {
    fn validate_password_policy(&self, password: &str) -> Result<(), ValidationError> {
        let policy = &self.password_policy;

        // Check minimum length
        if password.len() < policy.min_length {
            return Err(ValidationError::new(
                "PASSWORD_TOO_SHORT",
                &format!("Password must be at least {} characters", policy.min_length),
            ).with_field("password"));
        }

        // Check character requirements
        if policy.require_uppercase && !password.chars().any(|c| c.is_uppercase()) {
            return Err(ValidationError::new(
                "PASSWORD_MISSING_UPPERCASE",
                "Password must contain at least one uppercase letter",
            ).with_field("password"));
        }

        if policy.require_lowercase && !password.chars().any(|c| c.is_lowercase()) {
            return Err(ValidationError::new(
                "PASSWORD_MISSING_LOWERCASE",
                "Password must contain at least one lowercase letter",
            ).with_field("password"));
        }

        if policy.require_numbers && !password.chars().any(|c| c.is_numeric()) {
            return Err(ValidationError::new(
                "PASSWORD_MISSING_NUMBER",
                "Password must contain at least one number",
            ).with_field("password"));
        }

        if policy.require_special_chars && !password.chars().any(|c| "!@#$%^&*()".contains(c)) {
            return Err(ValidationError::new(
                "PASSWORD_MISSING_SPECIAL",
                "Password must contain at least one special character",
            ).with_field("password"));
        }

        // Check forbidden patterns
        for pattern in &policy.forbidden_patterns {
            if pattern.is_match(&password.to_lowercase()) {
                return Err(ValidationError::new(
                    "PASSWORD_FORBIDDEN_PATTERN",
                    "Password contains forbidden pattern",
                ).with_field("password"));
            }
        }

        Ok(())
    }
}
```

## Department-Based Validation

Validate users based on their department or role:

```rust
use scim_server::validation::{CustomValidator, ValidationContext, ValidationError};
use std::collections::HashMap;

pub struct DepartmentValidator {
    department_rules: HashMap<String, DepartmentRules>,
}

#[derive(Clone)]
pub struct DepartmentRules {
    pub required_attributes: Vec<String>,
    pub allowed_email_domains: Vec<String>,
    pub max_group_memberships: usize,
    pub requires_manager: bool,
}

impl DepartmentValidator {
    pub fn new() -> Self {
        let mut department_rules = HashMap::new();
        
        // IT Department rules
        department_rules.insert("IT".to_string(), DepartmentRules {
            required_attributes: vec!["employeeNumber".to_string(), "title".to_string()],
            allowed_email_domains: vec!["company.com".to_string()],
            max_group_memberships: 20,
            requires_manager: true,
        });
        
        // HR Department rules  
        department_rules.insert("HR".to_string(), DepartmentRules {
            required_attributes: vec!["employeeNumber".to_string(), "title".to_string(), "phoneNumber".to_string()],
            allowed_email_domains: vec!["company.com".to_string()],
            max_group_memberships: 10,
            requires_manager: true,
        });
        
        // Contractor rules
        department_rules.insert("CONTRACTOR".to_string(), DepartmentRules {
            required_attributes: vec!["contractEndDate".to_string()],
            allowed_email_domains: vec!["contractor.company.com".to_string()],
            max_group_memberships: 5,
            requires_manager: false,
        });

        Self { department_rules }
    }
}

#[async_trait]
impl CustomValidator for DepartmentValidator {
    async fn validate_user(
        &self,
        user: &User,
        context: &ValidationContext,
    ) -> Result<(), ValidationError> {
        // Get user's department from custom attributes
        let department = user.extension_attributes
            .as_ref()
            .and_then(|attrs| attrs.get("department"))
            .and_then(|v| v.as_str())
            .unwrap_or("UNKNOWN");

        if let Some(rules) = self.department_rules.get(department) {
            // Check required attributes
            for required_attr in &rules.required_attributes {
                if !user.extension_attributes
                    .as_ref()
                    .map(|attrs| attrs.contains_key(required_attr))
                    .unwrap_or(false) {
                    return Err(ValidationError::new(
                        "MISSING_REQUIRED_ATTRIBUTE",
                        &format!("Department {} requires attribute '{}'", department, required_attr),
                    ).with_field(&format!("enterpriseUser:{}", required_attr)));
                }
            }

            // Validate email domain for department
            if let Some(emails) = &user.emails {
                for (index, email) in emails.iter().enumerate() {
                    if let Some(domain) = email.value.split('@').nth(1) {
                        if !rules.allowed_email_domains.contains(&domain.to_lowercase()) {
                            return Err(ValidationError::new(
                                "INVALID_DEPARTMENT_EMAIL_DOMAIN",
                                &format!("Department {} does not allow email domain '{}'", department, domain),
                            ).with_field(&format!("emails[{}].value", index)));
                        }
                    }
                }
            }

            // Check manager requirement
            if rules.requires_manager {
                let has_manager = user.extension_attributes
                    .as_ref()
                    .and_then(|attrs| attrs.get("manager"))
                    .is_some();
                    
                if !has_manager {
                    return Err(ValidationError::new(
                        "MISSING_MANAGER",
                        &format!("Department {} requires a manager to be assigned", department),
                    ).with_field("enterpriseUser:manager"));
                }
            }
        }

        Ok(())
    }

    async fn validate_group(
        &self,
        group: &Group,
        _context: &ValidationContext,
    ) -> Result<(), ValidationError> {
        // Basic group validation for department-based rules
        Ok(())
    }
}
```

## Attribute Format Validation

Validate specific attribute formats beyond basic schema validation:

```rust
pub struct AttributeFormatValidator {
    phone_regex: Regex,
    ssn_regex: Regex,
    employee_id_regex: Regex,
}

impl AttributeFormatValidator {
    pub fn new() -> Self {
        Self {
            phone_regex: Regex::new(r"^\+1-\d{3}-\d{3}-\d{4}$").unwrap(),
            ssn_regex: Regex::new(r"^\d{3}-\d{2}-\d{4}$").unwrap(), 
            employee_id_regex: Regex::new(r"^[A-Z]{2}\d{6}$").unwrap(),
        }
    }

    fn validate_phone_number(&self, phone: &str) -> Result<(), ValidationError> {
        if !self.phone_regex.is_match(phone) {
            return Err(ValidationError::new(
                "INVALID_PHONE_FORMAT",
                "Phone number must be in format +1-XXX-XXX-XXXX",
            ));
        }
        Ok(())
    }

    fn validate_ssn(&self, ssn: &str) -> Result<(), ValidationError> {
        if !self.ssn_regex.is_match(ssn) {
            return Err(ValidationError::new(
                "INVALID_SSN_FORMAT", 
                "SSN must be in format XXX-XX-XXXX",
            ));
        }

        // Additional SSN validation rules
        let parts: Vec<&str> = ssn.split('-').collect();
        if parts.len() == 3 {
            // Check for invalid area numbers
            if let Ok(area) = parts[0].parse::<u32>() {
                if area == 0 || area == 666 || area >= 900 {
                    return Err(ValidationError::new(
                        "INVALID_SSN_AREA",
                        "Invalid SSN area number",
                    ));
                }
            }
        }
        Ok(())
    }
}

#[async_trait]
impl CustomValidator for AttributeFormatValidator {
    async fn validate_user(
        &self,
        user: &User,
        _context: &ValidationContext,
    ) -> Result<(), ValidationError> {
        // Validate phone numbers
        if let Some(phone_numbers) = &user.phone_numbers {
            for (index, phone) in phone_numbers.iter().enumerate() {
                self.validate_phone_number(&phone.value)
                    .map_err(|mut e| {
                        e.field_path = Some(format!("phoneNumbers[{}].value", index));
                        e
                    })?;
            }
        }

        // Validate custom attributes
        if let Some(attrs) = &user.extension_attributes {
            // Validate SSN if present
            if let Some(ssn_value) = attrs.get("ssn") {
                if let Some(ssn_str) = ssn_value.as_str() {
                    self.validate_ssn(ssn_str)
                        .map_err(|mut e| {
                            e.field_path = Some("enterpriseUser:ssn".to_string());
                            e
                        })?;
                }
            }

            // Validate employee ID
            if let Some(emp_id_value) = attrs.get("employeeNumber") {
                if let Some(emp_id_str) = emp_id_value.as_str() {
                    if !self.employee_id_regex.is_match(emp_id_str) {
                        return Err(ValidationError::new(
                            "INVALID_EMPLOYEE_ID_FORMAT",
                            "Employee ID must be in format XX123456",
                        ).with_field("enterpriseUser:employeeNumber"));
                    }
                }
            }
        }

        Ok(())
    }

    async fn validate_group(
        &self,
        _group: &Group,
        _context: &ValidationContext,
    ) -> Result<(), ValidationError> {
        Ok(())
    }
}
```

## Usage Example

Here's how to register and use these basic validators:

```rust
use scim_server::ScimServerBuilder;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let server = ScimServerBuilder::new()
        .with_provider(my_provider)
        .add_validator(BusinessRuleValidator::new())
        .add_validator(DepartmentValidator::new())
        .add_validator(AttributeFormatValidator::new())
        .build();

    // Start server
    server.run().await?;
    Ok(())
}
```

## Testing Basic Validators

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use scim_server::models::{User, Name, Email};

    #[tokio::test]
    async fn test_business_rule_validation() {
        let validator = BusinessRuleValidator::new();
        let context = ValidationContext::default();

        // Test valid user
        let mut user = User::default();
        user.username = Some("john.doe".to_string());
        user.external_id = Some("EMP123456".to_string());
        user.name = Some(Name {
            formatted: Some("John Doe".to_string()),
            family_name: Some("Doe".to_string()),
            given_name: Some("John".to_string()),
            ..Default::default()
        });
        user.emails = Some(vec![Email {
            value: "john.doe@company.com".to_string(),
            primary: Some(true),
            ..Default::default()
        }]);

        assert!(validator.validate_user(&user, &context).await.is_ok());

        // Test invalid employee ID
        user.external_id = Some("INVALID123".to_string());
        let result = validator.validate_user(&user, &context).await;
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().code, "INVALID_EMPLOYEE_ID");
    }

    #[tokio::test]
    async fn test_password_policy() {
        let validator = BusinessRuleValidator::new();
        
        // Test weak password
        let weak_password = "password123";
        let result = validator.validate_password_policy(weak_password);
        assert!(result.is_err());

        // Test strong password
        let strong_password = "MySecure123!Password";
        let result = validator.validate_password_policy(strong_password);
        assert!(result.is_ok());
    }
}
```

## Next Steps

- [Advanced Validation](./advanced.md) - External system integration and complex logic
- [Field-Level Validation](./field-level.md) - Granular attribute validation
- [Configuration](./configuration.md) - Dynamic validation rules