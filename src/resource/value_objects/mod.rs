//! Value objects for SCIM resource domain primitives.
//!
//! This module contains immutable value objects that encapsulate validation logic
//! for core SCIM domain concepts. Each value object enforces invariants at construction
//! time, making invalid states unrepresentable.
//!
//! ## Design Principles
//!
//! - **Immutable**: Once created, value objects cannot be modified
//! - **Self-validating**: Validation happens at construction time via explicit constructors
//! - **Type-safe**: Invalid states are unrepresentable at compile time
//! - **Domain-focused**: Each type represents a specific business concept
//!
//! ## Usage Pattern
//!
/// ```rust
/// use scim_server::resource::value_objects::ResourceId;
///
/// fn main() -> Result<(), Box<dyn std::error::Error>> {
///     // Explicit validation at construction
///     let id = ResourceId::new("2819c223-7f76-453a-919d-413861904646".to_string())?;
///
///     // Safe access to validated value
///     println!("Resource ID: {}", id.as_str());
///     Ok(())
/// }
/// ```
mod address;
mod composite_validation;
mod email_address;
mod extension;
mod external_id;
mod factory;
mod group_member;
mod meta;
mod multi_valued;
mod name;
mod phone_number;
mod resource_id;
mod schema_uri;
mod user_name;
mod value_object_trait;

pub use address::Address;
pub use composite_validation::{
    CompositeValidatorChain, EmailConsistencyValidator, IdentityConsistencyValidator,
    NameConsistencyValidator, UniquePrimaryValidator, UserNameUniquenessValidator,
};
pub use email_address::EmailAddress;
pub use extension::{ExtensionAttributeValue, ExtensionCollection};
pub use external_id::ExternalId;
pub use factory::GenericMultiValuedAttribute;
pub use group_member::{
    GroupMember, GroupMembers, MultiValuedAddresses, MultiValuedEmails, MultiValuedPhoneNumbers,
};
pub use meta::Meta;
pub use multi_valued::MultiValuedAttribute;
pub use name::Name;
pub use phone_number::PhoneNumber;
pub use resource_id::ResourceId;
pub use schema_uri::SchemaUri;
pub use user_name::UserName;
pub use value_object_trait::{
    CompositeValidator, ExtensionAttribute, SchemaConstructible, ValueObject,
    ValueObjectConstructor, ValueObjectRegistry,
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_value_objects_are_immutable() {
        // Once created, value objects cannot be modified
        let id = ResourceId::new("test-id".to_string()).unwrap();
        let id_str = id.as_str();

        // The only way to get a different value is to create a new instance
        let new_id = ResourceId::new("different-id".to_string()).unwrap();
        assert_ne!(id_str, new_id.as_str());
    }

    #[test]
    fn test_value_objects_enforce_invariants() {
        // Invalid values cannot be constructed
        assert!(ResourceId::new("".to_string()).is_err());

        // Valid values can be constructed
        assert!(ResourceId::new("valid-id".to_string()).is_ok());
    }
}
