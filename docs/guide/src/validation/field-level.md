# Field-Level Validation

This guide covers granular validation at the field and attribute level, allowing you to implement custom validation logic for specific user attributes, custom extensions, and complex data types.

## Custom Attribute Validators

The field-level validation system allows you to register validators for specific attributes:

```rust
use scim_server::validation::{FieldValidator, ValidationContext, ValidationError};
use std::collections::HashMap;
use regex::Regex;
use async_trait::async_trait;

pub struct CustomAttributeValidator {
    validators: HashMap<String, Box<dyn FieldValidator + Send + Sync>>,
}

#[async_trait]
pub trait FieldValidator {
    async fn validate(
        &self,
        field_name: &str,
        value: &serde_json::Value,
        context: &ValidationContext,
    ) -> Result<(), ValidationError>;
}

impl CustomAttributeValidator {
    pub fn new() -> Self {
        let mut validators = HashMap::new();
        
        // Phone number validator
        validators.insert(
            "phoneNumbers".to_string(),
            Box::new(PhoneNumberValidator::new()) as Box<dyn FieldValidator + Send + Sync>
        );
        
        // Social Security Number validator
        validators.insert(
            "enterpriseUser:ssn".to_string(),
            Box::new(SsnValidator::new()) as Box<dyn FieldValidator + Send + Sync>
        );
        
        // Employee ID validator
        validators.insert(
            "enterpriseUser:employeeNumber".to_string(),
            Box::new(EmployeeIdValidator::new()) as Box<dyn FieldValidator + Send + Sync>
        );
        
        // Custom business identifier validator
        validators.insert(
            "enterpriseUser:businessId".to_string(),
            Box::new(BusinessIdentifierValidator::new()) as Box<dyn FieldValidator + Send + Sync>
        );
        
        Self { validators }
    }
    
    pub async fn validate_field(
        &self,
        field_name: &str,
        value: &serde_json::Value,
        context: &ValidationContext,
    ) -> Result<(), ValidationError> {
        if let Some(validator) = self.validators.get(field_name) {
            validator.validate(field_name, value, context).await?;
        }
        
        Ok(())
    }
}
```

## Phone Number Validation

Comprehensive phone number validation with international format support:

```rust
pub struct PhoneNumberValidator {
    allowed_countries: Vec<String>,
    phone_regex: Regex,
    external_validation_enabled: bool,
}

impl PhoneNumberValidator {
    pub fn new() -> Self {
        Self {
            allowed_countries: vec![
                "US".to_string(), 
                "CA".to_string(), 
                "GB".to_string(),
                "DE".to_string(),
                "FR".to_string(),
            ],
            phone_regex: Regex::new(r"^\+[1-9]\d{1,14}$").unwrap(),
            external_validation_enabled: true,
        }
    }
    
    pub fn with_allowed_countries(mut self, countries: Vec<String>) -> Self {
        self.allowed_countries = countries;
        self
    }
    
    pub fn disable_external_validation(mut self) -> Self {
        self.external_validation_enabled = false;
        self
    }
}

#[async_trait]
impl FieldValidator for PhoneNumberValidator {
    async fn validate(
        &self,
        field_name: &str,
        value: &serde_json::Value,
        context: &ValidationContext,
    ) -> Result<(), ValidationError> {
        if let Some(phone_numbers) = value.as_array() {
            for (index, phone_obj) in phone_numbers.iter().enumerate() {
                if let Some(phone_value) = phone_obj.get("value").and_then(|v| v.as_str()) {
                    self.validate_single_phone(phone_value, field_name, index).await?;
                }
            }
        } else if let Some(phone_value) = value.as_str() {
            // Handle direct string value
            self.validate_single_phone(phone_value, field_name, 0).await?;
        }
        
        Ok(())
    }
}

impl PhoneNumberValidator {
    async fn validate_single_phone(
        &self,
        phone_value: &str,
        field_name: &str,
        index: usize,
    ) -> Result<(), ValidationError> {
        let field_path = if field_name.contains("phoneNumbers") {
            format!("{}[{}].value", field_name, index)
        } else {
            field_name.to_string()
        };
        
        // Basic format validation
        if !self.phone_regex.is_match(phone_value) {
            return Err(ValidationError::new(
                "INVALID_PHONE_FORMAT",
                "Phone number must be in international format (+1234567890)",
            ).with_field(&field_path));
        }
        
        // Extract and validate country code
        let country_code = self.extract_country_code(phone_value)?;
        if !self.is_allowed_country_code(&country_code) {
            return Err(ValidationError::new(
                "INVALID_COUNTRY_CODE",
                &format!("Phone number country code '{}' is not allowed", country_code),
            ).with_field(&field_path));
        }
        
        // Validate phone number length for specific countries
        self.validate_country_specific_length(phone_value, &country_code, &field_path)?;
        
        // External validation if enabled
        if self.external_validation_enabled {
            self.validate_with_external_service(phone_value, &field_path).await?;
        }
        
        Ok(())
    }
    
    fn extract_country_code(&self, phone_value: &str) -> Result<String, ValidationError> {
        if phone_value.len() < 2 {
            return Err(ValidationError::new(
                "INVALID_PHONE_LENGTH",
                "Phone number too short",
            ));
        }
        
        // Common country codes
        for length in [1, 2, 3] {
            if phone_value.len() > length {
                let potential_code = &phone_value[1..=length];
                if self.is_valid_country_code(potential_code) {
                    return Ok(potential_code.to_string());
                }
            }
        }
        
        Err(ValidationError::new(
            "UNKNOWN_COUNTRY_CODE",
            "Unable to determine country code",
        ))
    }
    
    fn is_valid_country_code(&self, code: &str) -> bool {
        // Common country codes
        let valid_codes = [
            "1", "7", "20", "27", "30", "31", "32", "33", "34", "36", "39", "40", "41", 
            "43", "44", "45", "46", "47", "48", "49", "51", "52", "53", "54", "55", "56", 
            "57", "58", "60", "61", "62", "63", "64", "65", "66", "81", "82", "84", "86", 
            "90", "91", "92", "93", "94", "95", "98", "212", "213", "216", "218", "220", 
            "221", "222", "223", "224", "225", "226", "227", "228", "229", "230", "231", 
            "232", "233", "234", "235", "236", "237", "238", "239", "240", "241", "242", 
            "243", "244", "245", "246", "247", "248", "249", "250", "251", "252", "253", 
            "254", "255", "256", "257", "258", "260", "261", "262", "263", "264", "265", 
            "266", "267", "268", "269", "290", "291", "297", "298", "299", "350", "351", 
            "352", "353", "354", "355", "356", "357", "358", "359", "370", "371", "372", 
            "373", "374", "375", "376", "377", "378", "380", "381", "382", "383", "385", 
            "386", "387", "389", "420", "421", "423", "500", "501", "502", "503", "504", 
            "505", "506", "507", "508", "509", "590", "591", "592", "593", "594", "595", 
            "596", "597", "598", "599", "670", "672", "673", "674", "675", "676", "677", 
            "678", "679", "680", "681", "682", "683", "684", "685", "686", "687", "688", 
            "689", "690", "691", "692", "850", "852", "853", "855", "856", "880", "886", 
            "960", "961", "962", "963", "964", "965", "966", "967", "968", "970", "971", 
            "972", "973", "974", "975", "976", "977", "992", "993", "994", "995", "996", 
            "998"
        ];
        
        valid_codes.contains(&code)
    }
    
    fn is_allowed_country_code(&self, country_code: &str) -> bool {
        match country_code {
            "1" => self.allowed_countries.contains(&"US".to_string()) || 
                   self.allowed_countries.contains(&"CA".to_string()),
            "44" => self.allowed_countries.contains(&"GB".to_string()),
            "49" => self.allowed_countries.contains(&"DE".to_string()),
            "33" => self.allowed_countries.contains(&"FR".to_string()),
            _ => true, // Allow other countries by default
        }
    }
    
    fn validate_country_specific_length(
        &self,
        phone_value: &str,
        country_code: &str,
        field_path: &str,
    ) -> Result<(), ValidationError> {
        let expected_lengths = match country_code {
            "1" => vec![11], // US/Canada: +1 + 10 digits
            "44" => vec![13], // UK: +44 + 10-11 digits
            "49" => vec![12, 13], // Germany: +49 + 10-11 digits
            "33" => vec![12], // France: +33 + 9 digits
            _ => return Ok(()), // Skip validation for other countries
        };
        
        if !expected_lengths.contains(&phone_value.len()) {
            return Err(ValidationError::new(
                "INVALID_PHONE_LENGTH",
                &format!("Invalid phone number length for country code {}", country_code),
            ).with_field(field_path));
        }
        
        Ok(())
    }
    
    async fn validate_with_external_service(
        &self,
        phone_number: &str,
        field_path: &str,
    ) -> Result<(), ValidationError> {
        let client = reqwest::Client::new();
        let response = client
            .get(&format!("https://api.phonevalidation.com/validate/{}", phone_number))
            .timeout(std::time::Duration::from_secs(3))
            .send()
            .await
            .map_err(|_| ValidationError::new(
                "PHONE_VALIDATION_SERVICE_ERROR",
                "Unable to validate phone number with external service",
            ).with_field(field_path))?;
        
        if !response.status().is_success() {
            return Err(ValidationError::new(
                "INVALID_PHONE_NUMBER",
                "Phone number validation failed",
            ).with_field(field_path));
        }
        
        // Parse validation response
        let validation_result: PhoneValidationResult = response.json().await
            .map_err(|_| ValidationError::new(
                "PHONE_VALIDATION_PARSE_ERROR",
                "Failed to parse phone validation response",
            ).with_field(field_path))?;
        
        if !validation_result.is_valid {
            return Err(ValidationError::new(
                "INVALID_PHONE_NUMBER",
                &validation_result.reason.unwrap_or_else(|| "Phone number is invalid".to_string()),
            ).with_field(field_path));
        }
        
        Ok(())
    }
}

#[derive(serde::Deserialize)]
struct PhoneValidationResult {
    is_valid: bool,
    reason: Option<String>,
    carrier: Option<String>,
    line_type: Option<String>,
}
```

## Social Security Number Validation

Comprehensive SSN validation with format and uniqueness checks:

```rust
pub struct SsnValidator {
    allow_itin: bool,
    check_uniqueness: bool,
}

impl SsnValidator {
    pub fn new() -> Self {
        Self {
            allow_itin: false,
            check_uniqueness: true,
        }
    }
    
    pub fn allow_itin(mut self, allow: bool) -> Self {
        self.allow_itin = allow;
        self
    }
    
    pub fn check_uniqueness(mut self, check: bool) -> Self {
        self.check_uniqueness = check;
        self
    }
    
    fn validate_ssn_format(&self, ssn: &str) -> Result<(), ValidationError> {
        // Remove hyphens and spaces
        let cleaned = ssn.replace(['-', ' '], "");
        
        // Must be exactly 9 digits
        if cleaned.len() != 9 || !cleaned.chars().all(|c| c.is_ascii_digit()) {
            return Err(ValidationError::new(
                "INVALID_SSN_FORMAT",
                "SSN must be 9 digits (XXX-XX-XXXX format)",
            ));
        }
        
        // Extract area, group, and serial numbers
        let area = &cleaned[0..3];
        let group = &cleaned[3..5];
        let serial = &cleaned[5..9];
        
        // Validate area number
        self.validate_area_number(area)?;
        
        // Validate group number
        self.validate_group_number(group)?;
        
        // Validate serial number
        self.validate_serial_number(serial)?;
        
        // Check for invalid patterns
        self.validate_patterns(&cleaned)?;
        
        Ok(())
    }
    
    fn validate_area_number(&self, area: &str) -> Result<(), ValidationError> {
        let area_num: u32 = area.parse().unwrap_or(0);
        
        // Invalid area numbers
        if area_num == 0 || area_num == 666 || area_num >= 900 {
            return Err(ValidationError::new(
                "INVALID_SSN_AREA",
                "Invalid SSN area number",
            ));
        }
        
        // Check if it's an ITIN (Individual Taxpayer Identification Number)
        if area_num >= 900 && area_num <= 999 {
            if !self.allow_itin {
                return Err(ValidationError::new(
                    "ITIN_NOT_ALLOWED",
                    "Individual Taxpayer Identification Numbers (ITIN) are not allowed",
                ));
            }
        }
        
        Ok(())
    }
    
    fn validate_group_number(&self, group: &str) -> Result<(), ValidationError> {
        let group_num: u32 = group.parse().unwrap_or(0);
        
        if group_num == 0 {
            return Err(ValidationError::new(
                "INVALID_SSN_GROUP",
                "Invalid SSN group number (cannot be 00)",
            ));
        }
        
        Ok(())
    }
    
    fn validate_serial_number(&self, serial: &str) -> Result<(), ValidationError> {
        let serial_num: u32 = serial.parse().unwrap_or(0);
        
        if serial_num == 0 {
            return Err(ValidationError::new(
                "INVALID_SSN_SERIAL",
                "Invalid SSN serial number (cannot be 0000)",
            ));
        }
        
        Ok(())
    }
    
    fn validate_patterns(&self, ssn: &str) -> Result<(), ValidationError> {
        // Invalid SSN patterns
        let invalid_patterns = [
            "000000000", "111111111", "222222222", "333333333",
            "444444444", "555555555", "666666666", "777777777",
            "888888888", "999999999", "123456789", "987654321"
        ];
        
        if invalid_patterns.contains(&ssn) {
            return Err(ValidationError::new(
                "INVALID_SSN_PATTERN",
                "SSN contains an invalid pattern",
            ));
        }
        
        // Check for consecutive digits
        if self.has_consecutive_pattern(ssn) {
            return Err(ValidationError::new(
                "INVALID_SSN_PATTERN",
                "SSN cannot contain repetitive patterns",
            ));
        }
        
        Ok(())
    }
    
    fn has_consecutive_pattern(&self, ssn: &str) -> bool {
        let chars: Vec<char> = ssn.chars().collect();
        
        // Check for all same digits
        if chars.iter().all(|&c| c == chars[0]) {
            return true;
        }
        
        // Check for ascending/descending sequences
        for window in chars.windows(3) {
            if window[0] as u8 + 1 == window[1] as u8 && window[1] as u8 + 1 == window[2] as u8 {
                return true; // Ascending sequence
            }
            if window[0] as u8 == window[1] as u8 + 1 && window[1] as u8 == window[2] as u8 + 1 {
                return true; // Descending sequence
            }
        }
        
        false
    }
}

#[async_trait]
impl FieldValidator for SsnValidator {
    async fn validate(
        &self,
        field_name: &str,
        value: &serde_json::Value,
        context: &ValidationContext,
    ) -> Result<(), ValidationError> {
        if let Some(ssn) = value.as_str() {
            // Validate format
            self.validate_ssn_format(ssn)
                .map_err(|mut e| {
                    e.field_path = Some(field_name.to_string());
                    e
                })?;
            
            // Check uniqueness if enabled
            if self.check_uniqueness {
                // Note: This would require access to the storage provider
                // In practice, you'd inject the storage provider into the validator
                if let Some(storage) = context.storage.as_ref() {
                    if storage.ssn_exists(&context.tenant_id, ssn).await.unwrap_or(false) {
                        return Err(ValidationError::new(
                            "SSN_ALREADY_EXISTS",
                            "Social Security Number is already in use",
                        ).with_field(field_name));
                    }
                }
            }
        }
        
        Ok(())
    }
}
```

## Employee ID Validation

Custom business identifier validation:

```rust
pub struct EmployeeIdValidator {
    format_regex: Regex,
    department_prefixes: HashMap<String, String>,
    check_uniqueness: bool,
}

impl EmployeeIdValidator {
    pub fn new() -> Self {
        let mut department_prefixes = HashMap::new();
        department_prefixes.insert("Engineering".to_string(), "ENG".to_string());
        department_prefixes.insert("Sales".to_string(), "SAL".to_string());
        department_prefixes.insert("Marketing".to_string(), "MKT".to_string());
        department_prefixes.insert("HR".to_string(), "HRS".to_string());
        department_prefixes.insert("Finance".to_string(), "FIN".to_string());
        
        Self {
            format_regex: Regex::new(r"^[A-Z]{3}\d{5}$").unwrap(),
            department_prefixes,
            check_uniqueness: true,
        }
    }
    
    pub fn with_custom_format(mut self, regex: &str) -> Result<Self, regex::Error> {
        self.format_regex = Regex::new(regex)?;
        Ok(self)
    }
    
    fn validate_format(&self, employee_id: &str) -> Result<(), ValidationError> {
        if !self.format_regex.is_match(employee_id) {
            return Err(ValidationError::new(
                "INVALID_EMPLOYEE_ID_FORMAT",
                "Employee ID must follow format: 3 letters + 5 digits (e.g., ENG12345)",
            ));
        }
        
        Ok(())
    }
    
    fn validate_department_prefix(
        &self,
        employee_id: &str,
        user_department: Option<&str>,
    ) -> Result<(), ValidationError> {
        if let Some(department) = user_department {
            if let Some(expected_prefix) = self.department_prefixes.get(department) {
                let prefix = &employee_id[0..3];
                if prefix != expected_prefix {
                    return Err(ValidationError::new(
                        "EMPLOYEE_ID_DEPARTMENT_MISMATCH",
                        &format!(
                            "Employee ID prefix '{}' does not match department '{}' (expected '{}')",
                            prefix, department, expected_prefix
                        ),
                    ));
                }
            }
        }
        
        Ok(())
    }
    
    fn validate_sequence_number(&self, employee_id: &str) -> Result<(), ValidationError> {
        let sequence = &employee_id[3..8];
        let sequence_num: u32 = sequence.parse().unwrap_or(0);
        
        // Sequence number cannot be 00000
        if sequence_num == 0 {
            return Err(ValidationError::new(
                "INVALID_EMPLOYEE_ID_SEQUENCE",
                "Employee ID sequence number cannot be 00000",
            ));
        }
        
        // Validate reasonable range (e.g., 00001-99999)
        if sequence_num > 99999 {
            return Err(ValidationError::new(
                "INVALID_EMPLOYEE_ID_SEQUENCE",
                "Employee ID sequence number must be between 00001 and 99999",
            ));
        }
        
        Ok(())
    }
}

#[async_trait]
impl FieldValidator for EmployeeIdValidator {
    async fn validate(
        &self,
        field_name: &str,
        value: &serde_json::Value,
        context: &ValidationContext,
    ) -> Result<(), ValidationError> {
        if let Some(employee_id) = value.as_str() {
            // Validate format
            self.validate_format(employee_id)
                .map_err(|mut e| {
                    e.field_path = Some(field_name.to_string());
                    e
                })?;
            
            // Validate sequence number
            self.validate_sequence_number(employee_id)
                .map_err(|mut e| {
                    e.field_path = Some(field_name.to_string());
                    e
                })?;
            
            // Validate department prefix if user has department info
            if let Some(user_data) = context.additional_data.get("user") {
                if let Some(department) = user_data.get("department").and_then(|v| v.as_str()) {
                    self.validate_department_prefix(employee_id, Some(department))
                        .map_err(|mut e| {
                            e.field_path = Some(field_name.to_string());
                            e
                        })?;
                }
            }
            
            // Check uniqueness
            if self.check_uniqueness {
                if let Some(storage) = context.storage.as_ref() {
                    if storage.employee_id_exists(&context.tenant_id, employee_id).await.unwrap_or(false) {
                        return Err(ValidationError::new(
                            "EMPLOYEE_ID_ALREADY_EXISTS",
                            "Employee ID is already in use",
                        ).with_field(field_name));
                    }
                }
            }
        }
        
        Ok(())
    }
}
```

## Credit Card Validation (for Financial Applications)

If your application handles financial data:

```rust
pub struct CreditCardValidator {
    allowed_types: Vec<CreditCardType>,
    validate_luhn: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub enum CreditCardType {
    Visa,
    MasterCard,
    AmericanExpress,
    Discover,
    DinersClub,
    JCB,
}

impl CreditCardValidator {
    pub fn new() -> Self {
        Self {
            allowed_types: vec![
                CreditCardType::Visa,
                CreditCardType::MasterCard,
                CreditCardType::AmericanExpress,
            ],
            validate_luhn: true,
        }
    }
    
    pub fn with_allowed_types(mut self, types: Vec<CreditCardType>) -> Self {
        self.allowed_types = types;
        self
    }
    
    fn detect_card_type(&self, number: &str) -> Option<CreditCardType> {
        match number {
            n if n.starts_with('4') => Some(CreditCardType::Visa),
            n if n.starts_with("51") || n.starts_with("52") || n.starts_with("53") || 
                 n.starts_with("54") || n.starts_with("55") => Some(CreditCardType::MasterCard),
            n if n.starts_with("34") || n.starts_with("37") => Some(CreditCardType::AmericanExpress),
            n if n.starts_with("6011") || n.starts_with("65") => Some(CreditCardType::Discover),
            n if n.starts_with("300") || n.starts_with("301") || n.starts_with("302") ||
                 n.starts_with("303") || n.starts_with("36") || n.starts_with("38") => Some(CreditCardType::DinersClub),
            n if n.starts_with("35") => Some(CreditCardType::JCB),
            _ => None,
        }
    }
    
    fn validate_luhn_algorithm(&self, number: &str) -> bool {
        let digits: Vec<u32> = number.chars()
            .filter_map(|c| c.to_digit(10))
            .collect();
        
        if digits.is_empty() {
            return false;
        }
        
        let checksum = digits.iter()
            .rev()
            .enumerate()
            .map(|(i, &digit)| {
                if i % 2 == 1 {
                    let doubled = digit * 2;
                    if doubled > 9 { doubled - 9 } else { doubled }
                } else {
                    digit
                }
            })
            .sum::<u32>();
        
        checksum % 10 == 0
    }
    
    fn validate_length(&self, number: &str, card_type: &CreditCardType) -> bool {
        match card_type {
            CreditCardType::Visa => number.len() == 13 || number.len() == 16 || number.len() == 19,
            CreditCardType::MasterCard => number.len() == 16,
            CreditCardType::AmericanExpress => number.len() == 15,
            CreditCardType::Discover => number.len() == 16,
            CreditCardType::DinersClub => number.len() == 14,
            CreditCardType::JCB => number.len() == 15 || number.len() == 16,
        }
    }
}

#[async_trait]
impl FieldValidator for CreditCardValidator {
    async fn validate(
        &self,
        field_name: &str,
        value: &serde_json::Value,
        _context: &ValidationContext,
    ) -> Result<(), ValidationError> {
        if let Some(card_number) = value.as_str() {
            // Remove spaces and hyphens
            let cleaned_number = card_number.replace([' ', '-'], "");
            
            // Validate format (digits only)
            if !cleaned_number.chars().all(|c| c.is_ascii_digit()) {
                return Err(ValidationError::new(
                    "INVALID_CREDIT_CARD_FORMAT",
                    "Credit card number must contain only digits",
                ).with_field(field_name));
            }
            
            // Detect card type
            let card_type = self.detect_card_type(&cleaned_number)
                .ok_or_else(|| ValidationError::new(
                    "UNSUPPORTED_CREDIT_CARD_TYPE",
                    "Unsupported credit card type",
                ).with_field(field_name))?;
            
            // Check if card type is allowed
            if !self.allowed_types.contains(&card_type) {
                return Err(ValidationError::new(
                    "CREDIT_CARD_TYPE_NOT_ALLOWED",
                    &format!("Credit card type {:?} is not allowed", card_type),
                ).with_field(field_name));
            }
            
            // Validate length for card type
            if !self.validate_length(&cleaned_number, &card_type) {
                return Err(ValidationError::new(
                    "INVALID_CREDIT_CARD_LENGTH",
                    &format!("Invalid length for {:?} credit card", card_type),
                ).with_field(field_name));
            }
            
            // Validate using Luhn algorithm
            if self.validate_luhn && !self.validate_luhn_algorithm(&cleaned_number) {
                return Err(ValidationError::new(
                    "INVALID_CREDIT_CARD_CHECKSUM",
                    "Credit card number failed checksum validation",
                ).with_field(field_name));
            }
        }
        
        Ok(())
    }
}
```

## Integration with Custom Validators

Use field-level validators within your main custom validators:

```rust
use scim_server::validation::{CustomValidator, ValidationContext, ValidationError};

pub struct ComprehensiveUserValidator {
    field_validator: CustomAttributeValidator,
}

impl ComprehensiveUserValidator {
    pub fn new() -> Self {
        Self {
            field_validator: CustomAttributeValidator::new(),
        }
    }
}

#[async_trait]
impl CustomValidator for ComprehensiveUserValidator {
    async fn validate_user(
        &self,
        user: &User,
        context: &ValidationContext,
    ) -> Result<(), ValidationError> {
        // Validate phone numbers
        if let Some(phone_numbers) = &user.phone_numbers {
            let phone_value = serde_json::to_value(phone_numbers).unwrap();
            self.field_validator
                .validate_field("phoneNumbers", &phone_value, context)
                .await?;
        }
        
        // Validate enterprise extensions
        if let Some(enterprise_ext) = &user.extension_attributes {
            for (key, value) in enterprise_ext {
                let field_name = format!("enterpriseUser:{}", key);
                self.field_validator
                    .validate_field(&field_name, value, context)
                    .await?;