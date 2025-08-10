//! Address value object for SCIM user address components.
//!
//! This module provides a type-safe wrapper around SCIM address attributes with built-in validation.
//! Address attributes represent physical mailing addresses as defined in RFC 7643 Section 4.1.2.

use crate::error::{ValidationError, ValidationResult};
use serde::{Deserialize, Serialize};
use std::fmt;

/// A validated SCIM address attribute.
///
/// Address represents a physical mailing address as defined in RFC 7643.
/// It enforces validation rules at construction time, ensuring that only valid address
/// attributes can exist in the system.
///
/// ## Validation Rules
///
/// - At least one address component must be provided (not all fields can be empty/None)
/// - Individual address components cannot be empty strings
/// - Country code must be valid ISO 3166-1 "alpha-2" format when provided
/// - Type must be one of canonical values: "work", "home", "other" when provided
/// - Primary can only be true for one address in a collection
///
/// ## Examples
///
/// ```rust
/// use scim_server::resource::value_objects::Address;
///
/// fn main() -> Result<(), Box<dyn std::error::Error>> {
///     // Create with full address components
///     let address = Address::new(
///         Some("100 Universal City Plaza\nHollywood, CA 91608 USA".to_string()),
///         Some("100 Universal City Plaza".to_string()),
///         Some("Hollywood".to_string()),
///         Some("CA".to_string()),
///         Some("91608".to_string()),
///         Some("US".to_string()),
///         Some("work".to_string()),
///         Some(true)
///     )?;
///
///     // Create with minimal components
///     let simple_address = Address::new_simple(
///         "123 Main St".to_string(),
///         "Anytown".to_string(),
///         "US".to_string()
///     )?;
///
///     Ok(())
/// }
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Address {
    pub formatted: Option<String>,
    #[serde(rename = "streetAddress")]
    pub street_address: Option<String>,
    pub locality: Option<String>,
    pub region: Option<String>,
    #[serde(rename = "postalCode")]
    pub postal_code: Option<String>,
    pub country: Option<String>,
    #[serde(rename = "type")]
    pub address_type: Option<String>,
    pub primary: Option<bool>,
}

impl Address {
    /// Create a new Address with all components.
    ///
    /// This is the primary constructor that enforces all validation rules.
    /// Use this method when creating Address instances from untrusted input.
    ///
    /// # Arguments
    ///
    /// * `formatted` - The full mailing address, formatted for display
    /// * `street_address` - The full street address component
    /// * `locality` - The city or locality component
    /// * `region` - The state or region component
    /// * `postal_code` - The zip code or postal code component
    /// * `country` - The country name component (ISO 3166-1 alpha-2)
    /// * `address_type` - The type of address ("work", "home", "other")
    /// * `primary` - Whether this is the primary address
    ///
    /// # Returns
    ///
    /// * `Ok(Address)` - If at least one field is provided and all provided fields are valid
    /// * `Err(ValidationError)` - If all fields are None/empty or any field violates validation rules
    pub fn new(
        formatted: Option<String>,
        street_address: Option<String>,
        locality: Option<String>,
        region: Option<String>,
        postal_code: Option<String>,
        country: Option<String>,
        address_type: Option<String>,
        primary: Option<bool>,
    ) -> ValidationResult<Self> {
        // Validate individual components
        if let Some(ref f) = formatted {
            Self::validate_address_component(f, "formatted")?;
        }
        if let Some(ref sa) = street_address {
            Self::validate_address_component(sa, "streetAddress")?;
        }
        if let Some(ref l) = locality {
            Self::validate_address_component(l, "locality")?;
        }
        if let Some(ref r) = region {
            Self::validate_address_component(r, "region")?;
        }
        if let Some(ref pc) = postal_code {
            Self::validate_address_component(pc, "postalCode")?;
        }
        if let Some(ref c) = country {
            Self::validate_country_code(c)?;
        }
        if let Some(ref at) = address_type {
            Self::validate_address_type(at)?;
        }

        // Ensure at least one meaningful component is provided
        if formatted.is_none()
            && street_address.is_none()
            && locality.is_none()
            && region.is_none()
            && postal_code.is_none()
            && country.is_none()
        {
            return Err(ValidationError::custom(
                "At least one address component must be provided",
            ));
        }

        Ok(Self {
            formatted,
            street_address,
            locality,
            region,
            postal_code,
            country,
            address_type,
            primary,
        })
    }

    /// Create a simple Address with basic components.
    ///
    /// Convenience constructor for creating basic address structures.
    ///
    /// # Arguments
    ///
    /// * `street_address` - The street address
    /// * `locality` - The city or locality
    /// * `country` - The country code (ISO 3166-1 alpha-2)
    ///
    /// # Returns
    ///
    /// * `Ok(Address)` - If the address components are valid
    /// * `Err(ValidationError)` - If any component violates validation rules
    pub fn new_simple(
        street_address: String,
        locality: String,
        country: String,
    ) -> ValidationResult<Self> {
        Self::new(
            None,
            Some(street_address),
            Some(locality),
            None,
            None,
            Some(country),
            None,
            None,
        )
    }

    /// Create a work Address.
    ///
    /// Convenience constructor for work addresses.
    ///
    /// # Arguments
    ///
    /// * `street_address` - The street address
    /// * `locality` - The city or locality
    /// * `region` - The state or region
    /// * `postal_code` - The postal code
    /// * `country` - The country code (ISO 3166-1 alpha-2)
    ///
    /// # Returns
    ///
    /// * `Ok(Address)` - If the address components are valid
    /// * `Err(ValidationError)` - If any component violates validation rules
    pub fn new_work(
        street_address: String,
        locality: String,
        region: String,
        postal_code: String,
        country: String,
    ) -> ValidationResult<Self> {
        Self::new(
            None,
            Some(street_address),
            Some(locality),
            Some(region),
            Some(postal_code),
            Some(country),
            Some("work".to_string()),
            None,
        )
    }

    /// Create an Address instance without validation for internal use.
    ///
    /// This method bypasses validation and should only be used when the data
    /// is known to be valid, such as when deserializing from trusted sources.
    ///
    /// # Safety
    ///
    /// The caller must ensure that the provided values are valid according to
    /// SCIM address validation rules.
    #[allow(dead_code)]
    pub(crate) fn new_unchecked(
        formatted: Option<String>,
        street_address: Option<String>,
        locality: Option<String>,
        region: Option<String>,
        postal_code: Option<String>,
        country: Option<String>,
        address_type: Option<String>,
        primary: Option<bool>,
    ) -> Self {
        Self {
            formatted,
            street_address,
            locality,
            region,
            postal_code,
            country,
            address_type,
            primary,
        }
    }

    /// Get the formatted address.
    pub fn formatted(&self) -> Option<&str> {
        self.formatted.as_deref()
    }

    /// Get the street address.
    pub fn street_address(&self) -> Option<&str> {
        self.street_address.as_deref()
    }

    /// Get the locality.
    pub fn locality(&self) -> Option<&str> {
        self.locality.as_deref()
    }

    /// Get the region.
    pub fn region(&self) -> Option<&str> {
        self.region.as_deref()
    }

    /// Get the postal code.
    pub fn postal_code(&self) -> Option<&str> {
        self.postal_code.as_deref()
    }

    /// Get the country code.
    pub fn country(&self) -> Option<&str> {
        self.country.as_deref()
    }

    /// Get the address type.
    pub fn address_type(&self) -> Option<&str> {
        self.address_type.as_deref()
    }

    /// Get whether this is the primary address.
    pub fn is_primary(&self) -> bool {
        self.primary.unwrap_or(false)
    }

    /// Generate a formatted display address from components.
    ///
    /// Creates a formatted address string from the available address components
    /// if no explicit formatted address is provided.
    ///
    /// # Returns
    ///
    /// The formatted address if available, otherwise a constructed address from components,
    /// or None if no meaningful components are available.
    pub fn display_address(&self) -> Option<String> {
        if let Some(ref formatted) = self.formatted {
            return Some(formatted.clone());
        }

        let mut parts = Vec::new();

        if let Some(ref street) = self.street_address {
            parts.push(street.as_str());
        }

        let mut city_line = Vec::new();
        if let Some(ref locality) = self.locality {
            city_line.push(locality.as_str());
        }
        if let Some(ref region) = self.region {
            city_line.push(region.as_str());
        }
        if let Some(ref postal) = self.postal_code {
            city_line.push(postal.as_str());
        }

        let city_line_str = if !city_line.is_empty() {
            Some(city_line.join(", "))
        } else {
            None
        };

        if let Some(ref city_str) = city_line_str {
            parts.push(city_str.as_str());
        }

        if let Some(ref country) = self.country {
            parts.push(country.as_str());
        }

        if parts.is_empty() {
            None
        } else {
            Some(parts.join("\n"))
        }
    }

    /// Check if the address has any meaningful content.
    pub fn is_empty(&self) -> bool {
        self.formatted.is_none()
            && self.street_address.is_none()
            && self.locality.is_none()
            && self.region.is_none()
            && self.postal_code.is_none()
            && self.country.is_none()
    }

    /// Validate an address component.
    fn validate_address_component(value: &str, field_name: &str) -> ValidationResult<()> {
        if value.trim().is_empty() {
            return Err(ValidationError::custom(format!(
                "{}: Address component cannot be empty or contain only whitespace",
                field_name
            )));
        }

        // Check for reasonable length
        if value.len() > 1024 {
            return Err(ValidationError::custom(format!(
                "{}: Address component exceeds maximum length of 1024 characters",
                field_name
            )));
        }

        Ok(())
    }

    /// Validate country code according to ISO 3166-1 alpha-2.
    fn validate_country_code(country: &str) -> ValidationResult<()> {
        if country.trim().is_empty() {
            return Err(ValidationError::custom(
                "country: Country code cannot be empty",
            ));
        }

        // Must be exactly 2 characters for ISO 3166-1 alpha-2
        if country.len() != 2 {
            return Err(ValidationError::custom(
                "country: Country code must be exactly 2 characters (ISO 3166-1 alpha-2 format)",
            ));
        }

        // Must be alphabetic
        if !country.chars().all(|c| c.is_ascii_alphabetic()) {
            return Err(ValidationError::custom(
                "country: Country code must contain only alphabetic characters",
            ));
        }

        // Convert to uppercase for validation (we'll store as provided)
        let country_upper = country.to_uppercase();

        // Validate against common ISO 3166-1 alpha-2 codes
        // This is a subset of the most common codes - in practice you might want a complete list
        let valid_codes = [
            "AD", "AE", "AF", "AG", "AI", "AL", "AM", "AO", "AQ", "AR", "AS", "AT", "AU", "AW",
            "AX", "AZ", "BA", "BB", "BD", "BE", "BF", "BG", "BH", "BI", "BJ", "BL", "BM", "BN",
            "BO", "BQ", "BR", "BS", "BT", "BV", "BW", "BY", "BZ", "CA", "CC", "CD", "CF", "CG",
            "CH", "CI", "CK", "CL", "CM", "CN", "CO", "CR", "CU", "CV", "CW", "CX", "CY", "CZ",
            "DE", "DJ", "DK", "DM", "DO", "DZ", "EC", "EE", "EG", "EH", "ER", "ES", "ET", "FI",
            "FJ", "FK", "FM", "FO", "FR", "GA", "GB", "GD", "GE", "GF", "GG", "GH", "GI", "GL",
            "GM", "GN", "GP", "GQ", "GR", "GS", "GT", "GU", "GW", "GY", "HK", "HM", "HN", "HR",
            "HT", "HU", "ID", "IE", "IL", "IM", "IN", "IO", "IQ", "IR", "IS", "IT", "JE", "JM",
            "JO", "JP", "KE", "KG", "KH", "KI", "KM", "KN", "KP", "KR", "KW", "KY", "KZ", "LA",
            "LB", "LC", "LI", "LK", "LR", "LS", "LT", "LU", "LV", "LY", "MA", "MC", "MD", "ME",
            "MF", "MG", "MH", "MK", "ML", "MM", "MN", "MO", "MP", "MQ", "MR", "MS", "MT", "MU",
            "MV", "MW", "MX", "MY", "MZ", "NA", "NC", "NE", "NF", "NG", "NI", "NL", "NO", "NP",
            "NR", "NU", "NZ", "OM", "PA", "PE", "PF", "PG", "PH", "PK", "PL", "PM", "PN", "PR",
            "PS", "PT", "PW", "PY", "QA", "RE", "RO", "RS", "RU", "RW", "SA", "SB", "SC", "SD",
            "SE", "SG", "SH", "SI", "SJ", "SK", "SL", "SM", "SN", "SO", "SR", "SS", "ST", "SV",
            "SX", "SY", "SZ", "TC", "TD", "TF", "TG", "TH", "TJ", "TK", "TL", "TM", "TN", "TO",
            "TR", "TT", "TV", "TW", "TZ", "UA", "UG", "UM", "US", "UY", "UZ", "VA", "VC", "VE",
            "VG", "VI", "VN", "VU", "WF", "WS", "YE", "YT", "ZA", "ZM", "ZW",
        ];

        if !valid_codes.contains(&country_upper.as_str()) {
            return Err(ValidationError::custom(format!(
                "country: '{}' is not a valid ISO 3166-1 alpha-2 country code",
                country
            )));
        }

        Ok(())
    }

    /// Validate address type against canonical values.
    fn validate_address_type(address_type: &str) -> ValidationResult<()> {
        if address_type.trim().is_empty() {
            return Err(ValidationError::custom(
                "type: Address type cannot be empty",
            ));
        }

        // SCIM canonical values for address type
        let valid_types = ["work", "home", "other"];
        if !valid_types.contains(&address_type) {
            return Err(ValidationError::custom(format!(
                "type: '{}' is not a valid address type. Valid types are: {:?}",
                address_type, valid_types
            )));
        }

        Ok(())
    }
}

impl fmt::Display for Address {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.display_address() {
            Some(address) => {
                if let Some(address_type) = &self.address_type {
                    write!(f, "{} ({})", address, address_type)
                } else {
                    write!(f, "{}", address)
                }
            }
            None => write!(f, "[Empty Address]"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_address_full() {
        let address = Address::new(
            Some("100 Universal City Plaza\nHollywood, CA 91608 USA".to_string()),
            Some("100 Universal City Plaza".to_string()),
            Some("Hollywood".to_string()),
            Some("CA".to_string()),
            Some("91608".to_string()),
            Some("US".to_string()),
            Some("work".to_string()),
            Some(true),
        );

        assert!(address.is_ok());
        let address = address.unwrap();
        assert_eq!(
            address.formatted(),
            Some("100 Universal City Plaza\nHollywood, CA 91608 USA")
        );
        assert_eq!(address.street_address(), Some("100 Universal City Plaza"));
        assert_eq!(address.locality(), Some("Hollywood"));
        assert_eq!(address.region(), Some("CA"));
        assert_eq!(address.postal_code(), Some("91608"));
        assert_eq!(address.country(), Some("US"));
        assert_eq!(address.address_type(), Some("work"));
        assert!(address.is_primary());
    }

    #[test]
    fn test_valid_address_simple() {
        let address = Address::new_simple(
            "123 Main St".to_string(),
            "Anytown".to_string(),
            "US".to_string(),
        );

        assert!(address.is_ok());
        let address = address.unwrap();
        assert_eq!(address.street_address(), Some("123 Main St"));
        assert_eq!(address.locality(), Some("Anytown"));
        assert_eq!(address.country(), Some("US"));
        assert!(!address.is_primary());
    }

    #[test]
    fn test_valid_address_work() {
        let address = Address::new_work(
            "456 Business Ave".to_string(),
            "Corporate City".to_string(),
            "NY".to_string(),
            "10001".to_string(),
            "US".to_string(),
        );

        assert!(address.is_ok());
        let address = address.unwrap();
        assert_eq!(address.address_type(), Some("work"));
        assert_eq!(address.region(), Some("NY"));
        assert_eq!(address.postal_code(), Some("10001"));
    }

    #[test]
    fn test_empty_address_components() {
        let result = Address::new(
            Some("".to_string()),
            None,
            None,
            None,
            None,
            None,
            None,
            None,
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_all_none_components() {
        let result = Address::new(None, None, None, None, None, None, None, None);
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("At least one address component")
        );
    }

    #[test]
    fn test_invalid_country_code() {
        let result = Address::new_simple(
            "123 Main St".to_string(),
            "Anytown".to_string(),
            "USA".to_string(), // Should be US
        );
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("must be exactly 2 characters")
        );
    }

    #[test]
    fn test_invalid_country_code_non_alphabetic() {
        let result = Address::new_simple(
            "123 Main St".to_string(),
            "Anytown".to_string(),
            "U1".to_string(),
        );
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("must contain only alphabetic")
        );
    }

    #[test]
    fn test_invalid_country_code_unknown() {
        let result = Address::new_simple(
            "123 Main St".to_string(),
            "Anytown".to_string(),
            "XX".to_string(),
        );
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("not a valid ISO 3166-1")
        );
    }

    #[test]
    fn test_invalid_address_type() {
        let result = Address::new(
            None,
            Some("123 Main St".to_string()),
            Some("Anytown".to_string()),
            None,
            None,
            Some("US".to_string()),
            Some("business".to_string()), // Should be work, home, or other
            None,
        );
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("not a valid address type")
        );
    }

    #[test]
    fn test_too_long_component() {
        let long_street = "a".repeat(1100);
        let result = Address::new_simple(long_street, "Anytown".to_string(), "US".to_string());
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("exceeds maximum length")
        );
    }

    #[test]
    fn test_display_address_with_formatted() {
        let address = Address::new(
            Some("100 Main St\nAnytown, NY 12345\nUSA".to_string()),
            None,
            None,
            None,
            None,
            None,
            None,
            None,
        )
        .unwrap();

        assert_eq!(
            address.display_address(),
            Some("100 Main St\nAnytown, NY 12345\nUSA".to_string())
        );
    }

    #[test]
    fn test_display_address_from_components() {
        let address = Address::new(
            None,
            Some("123 Main St".to_string()),
            Some("Anytown".to_string()),
            Some("NY".to_string()),
            Some("12345".to_string()),
            Some("US".to_string()),
            None,
            None,
        )
        .unwrap();

        assert_eq!(
            address.display_address(),
            Some("123 Main St\nAnytown, NY, 12345\nUS".to_string())
        );
    }

    #[test]
    fn test_display_address_partial_components() {
        let address = Address::new(
            None,
            Some("456 Oak Ave".to_string()),
            Some("Springfield".to_string()),
            None,
            None,
            Some("US".to_string()),
            None,
            None,
        )
        .unwrap();

        assert_eq!(
            address.display_address(),
            Some("456 Oak Ave\nSpringfield\nUS".to_string())
        );
    }

    #[test]
    fn test_is_empty() {
        let empty_address = Address::new_unchecked(None, None, None, None, None, None, None, None);
        assert!(empty_address.is_empty());

        let non_empty_address = Address::new_simple(
            "123 Main St".to_string(),
            "Anytown".to_string(),
            "US".to_string(),
        )
        .unwrap();
        assert!(!non_empty_address.is_empty());
    }

    #[test]
    fn test_new_unchecked() {
        let address = Address::new_unchecked(
            Some("123 Main St".to_string()),
            Some("123 Main St".to_string()),
            Some("Anytown".to_string()),
            Some("NY".to_string()),
            Some("12345".to_string()),
            Some("US".to_string()),
            Some("home".to_string()),
            Some(false),
        );

        assert_eq!(address.street_address(), Some("123 Main St"));
        assert_eq!(address.locality(), Some("Anytown"));
        assert_eq!(address.address_type(), Some("home"));
        assert!(!address.is_primary());
    }

    #[test]
    fn test_display() {
        let address = Address::new_work(
            "100 Business Blvd".to_string(),
            "Corporate City".to_string(),
            "NY".to_string(),
            "10001".to_string(),
            "US".to_string(),
        )
        .unwrap();

        let display = format!("{}", address);
        assert!(display.contains("100 Business Blvd"));
        assert!(display.contains("(work)"));

        let empty_address = Address::new_unchecked(None, None, None, None, None, None, None, None);
        assert_eq!(format!("{}", empty_address), "[Empty Address]");
    }

    #[test]
    fn test_serialization() {
        let address = Address::new_work(
            "100 Business Blvd".to_string(),
            "Corporate City".to_string(),
            "NY".to_string(),
            "10001".to_string(),
            "US".to_string(),
        )
        .unwrap();

        let json = serde_json::to_string(&address).unwrap();
        assert!(json.contains("\"streetAddress\":\"100 Business Blvd\""));
        assert!(json.contains("\"locality\":\"Corporate City\""));
        assert!(json.contains("\"type\":\"work\""));
    }

    #[test]
    fn test_deserialization() {
        let json = r#"{
            "formatted": "100 Universal City Plaza\nHollywood, CA 91608 USA",
            "streetAddress": "100 Universal City Plaza",
            "locality": "Hollywood",
            "region": "CA",
            "postalCode": "91608",
            "country": "US",
            "type": "work",
            "primary": true
        }"#;

        let address: Address = serde_json::from_str(json).unwrap();
        assert_eq!(address.street_address(), Some("100 Universal City Plaza"));
        assert_eq!(address.locality(), Some("Hollywood"));
        assert_eq!(address.country(), Some("US"));
        assert_eq!(address.address_type(), Some("work"));
        assert!(address.is_primary());
    }

    #[test]
    fn test_equality() {
        let addr1 = Address::new_simple(
            "123 Main St".to_string(),
            "Anytown".to_string(),
            "US".to_string(),
        )
        .unwrap();
        let addr2 = Address::new_simple(
            "123 Main St".to_string(),
            "Anytown".to_string(),
            "US".to_string(),
        )
        .unwrap();
        let addr3 = Address::new_simple(
            "456 Oak Ave".to_string(),
            "Anytown".to_string(),
            "US".to_string(),
        )
        .unwrap();

        assert_eq!(addr1, addr2);
        assert_ne!(addr1, addr3);
    }

    #[test]
    fn test_clone() {
        let original = Address::new_work(
            "100 Business Blvd".to_string(),
            "Corporate City".to_string(),
            "NY".to_string(),
            "10001".to_string(),
            "US".to_string(),
        )
        .unwrap();

        let cloned = original.clone();
        assert_eq!(original, cloned);
        assert_eq!(cloned.street_address(), Some("100 Business Blvd"));
        assert_eq!(cloned.address_type(), Some("work"));
    }

    #[test]
    fn test_country_code_case_insensitive() {
        let address = Address::new_simple(
            "123 Main St".to_string(),
            "Anytown".to_string(),
            "us".to_string(), // lowercase
        );
        assert!(address.is_ok());

        let address = Address::new_simple(
            "123 Main St".to_string(),
            "Anytown".to_string(),
            "Us".to_string(), // mixed case
        );
        assert!(address.is_ok());
    }

    #[test]
    fn test_valid_address_types() {
        for addr_type in ["work", "home", "other"] {
            let address = Address::new(
                None,
                Some("123 Main St".to_string()),
                Some("Anytown".to_string()),
                None,
                None,
                Some("US".to_string()),
                Some(addr_type.to_string()),
                None,
            );
            assert!(
                address.is_ok(),
                "Address type '{}' should be valid",
                addr_type
            );
        }
    }
}
