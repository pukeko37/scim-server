//! Group membership value objects for SCIM resources.
//!
//! This module provides value objects for handling group membership relationships
//! in SCIM resources, following SCIM 2.0 specifications for group membership.
//!
//! ## Design Principles
//!
//! - **Resource Relationships**: Type-safe representation of group member relationships
//! - **SCIM Compliance**: Follows SCIM 2.0 group membership attribute patterns
//! - **Multi-Valued Support**: Integrates with MultiValuedAttribute for collections
//! - **Type Safety**: Ensures valid member references and display names
//!
//! ## Usage Pattern
//!
//! ```rust
//! use scim_server::resource::value_objects::{GroupMember, GroupMembers, ResourceId};
//! use scim_server::error::ValidationResult;
//!
//! fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // Create individual group member
//!     let member_id = ResourceId::new("user-123".to_string())?;
//!     let member = GroupMember::new(member_id, Some("John Doe".to_string()), Some("User".to_string()))?;
//!
//!     // Create collection of group members
//!     let members = vec![member];
//!     let group_members = GroupMembers::new(members)?;
//!
//!     // Access members
//!     for member in group_members.iter() {
//!         println!("Member: {} ({})", member.display_name().unwrap_or("Unknown"), member.value().as_str());
//!     }
//!
//!     Ok(())
//! }
//! ```

use crate::error::{ValidationError, ValidationResult};
use crate::resource::value_objects::{MultiValuedAttribute, ResourceId};
use serde::{Deserialize, Serialize};
use std::fmt;

/// A value object representing a single group member in SCIM.
///
/// This type encapsulates the relationship between a group and its member,
/// including the member's resource ID, display name, and member type.
///
/// # Examples
///
/// ```rust
/// use scim_server::resource::value_objects::{GroupMember, ResourceId};
///
/// fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let member_id = ResourceId::new("user-123".to_string())?;
///     let member = GroupMember::new(
///         member_id,
///         Some("John Doe".to_string()),
///         Some("User".to_string())
///     )?;
///
///     assert_eq!(member.display_name(), Some("John Doe"));
///     assert_eq!(member.member_type(), Some("User"));
///
///     Ok(())
/// }
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GroupMember {
    /// The unique identifier of the member resource
    value: ResourceId,
    /// Human-readable display name for the member
    #[serde(skip_serializing_if = "Option::is_none")]
    display: Option<String>,
    /// The type of member (e.g., "User", "Group")
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "type")]
    member_type: Option<String>,
}

impl GroupMember {
    /// Creates a new group member with validation.
    ///
    /// # Arguments
    ///
    /// * `value` - The resource ID of the member
    /// * `display` - Optional display name for the member
    /// * `member_type` - Optional type of the member (e.g., "User", "Group")
    ///
    /// # Returns
    ///
    /// * `Ok(GroupMember)` - Successfully created group member
    /// * `Err(ValidationError)` - If validation fails
    ///
    /// # Examples
    ///
    /// ```rust
    /// use scim_server::resource::value_objects::{GroupMember, ResourceId};
    ///
    /// fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let member_id = ResourceId::new("user-123".to_string())?;
    ///     let member = GroupMember::new(
    ///         member_id,
    ///         Some("John Doe".to_string()),
    ///         Some("User".to_string())
    ///     )?;
    ///
    ///     Ok(())
    /// }
    /// ```
    pub fn new(
        value: ResourceId,
        display: Option<String>,
        member_type: Option<String>,
    ) -> ValidationResult<Self> {
        // Validate display name if provided
        if let Some(ref display_name) = display {
            Self::validate_display_name(display_name)?;
        }

        // Validate member type if provided
        if let Some(ref mtype) = member_type {
            Self::validate_member_type(mtype)?;
        }

        Ok(Self {
            value,
            display,
            member_type,
        })
    }

    /// Creates a new group member for a User resource.
    ///
    /// # Arguments
    ///
    /// * `value` - The resource ID of the user
    /// * `display` - Optional display name for the user
    ///
    /// # Returns
    ///
    /// A group member with member_type set to "User"
    ///
    /// # Examples
    ///
    /// ```rust
    /// use scim_server::resource::value_objects::{GroupMember, ResourceId};
    ///
    /// fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let user_id = ResourceId::new("user-123".to_string())?;
    ///     let member = GroupMember::new_user(user_id, Some("John Doe".to_string()))?;
    ///     assert_eq!(member.member_type(), Some("User"));
    ///
    ///     Ok(())
    /// }
    /// ```
    pub fn new_user(value: ResourceId, display: Option<String>) -> ValidationResult<Self> {
        Self::new(value, display, Some("User".to_string()))
    }

    /// Creates a new group member for a Group resource.
    ///
    /// # Arguments
    ///
    /// * `value` - The resource ID of the group
    /// * `display` - Optional display name for the group
    ///
    /// # Returns
    ///
    /// A group member with member_type set to "Group"
    ///
    /// # Examples
    ///
    /// ```rust
    /// use scim_server::resource::value_objects::{GroupMember, ResourceId};
    ///
    /// fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let group_id = ResourceId::new("group-456".to_string())?;
    ///     let member = GroupMember::new_group(group_id, Some("Admin Group".to_string()))?;
    ///     assert_eq!(member.member_type(), Some("Group"));
    ///
    ///     Ok(())
    /// }
    /// ```
    pub fn new_group(value: ResourceId, display: Option<String>) -> ValidationResult<Self> {
        Self::new(value, display, Some("Group".to_string()))
    }

    /// Creates a new group member without validation for internal use.
    ///
    /// This method bypasses validation and should only be used internally
    /// where the inputs are already known to be valid.
    ///
    /// # Arguments

    /// Returns the resource ID of the member.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use scim_server::resource::value_objects::{GroupMember, ResourceId};
    ///
    /// fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let user_id = ResourceId::new("user-123".to_string())?;
    ///     let member = GroupMember::new_user(user_id, None)?;
    ///     let id = member.value();
    ///     println!("Member ID: {}", id.as_str());
    ///     Ok(())
    /// }
    /// ```
    pub fn value(&self) -> &ResourceId {
        &self.value
    }

    /// Returns the display name of the member, if set.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use scim_server::resource::value_objects::{GroupMember, ResourceId};
    ///
    /// fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let user_id = ResourceId::new("user-123".to_string())?;
    ///     let member = GroupMember::new_user(user_id, Some("John Doe".to_string()))?;
    ///     if let Some(name) = member.display_name() {
    ///         println!("Member name: {}", name);
    ///     }
    ///     Ok(())
    /// }
    /// ```
    pub fn display_name(&self) -> Option<&str> {
        self.display.as_deref()
    }

    /// Returns the member type, if set.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use scim_server::resource::value_objects::{GroupMember, ResourceId};
    ///
    /// fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let user_id = ResourceId::new("user-123".to_string())?;
    ///     let member = GroupMember::new_user(user_id, None)?;
    ///     if let Some(mtype) = member.member_type() {
    ///         println!("Member type: {}", mtype);
    ///     }
    ///     Ok(())
    /// }
    /// ```
    pub fn member_type(&self) -> Option<&str> {
        self.member_type.as_deref()
    }

    /// Returns true if this member represents a User resource.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use scim_server::resource::value_objects::{GroupMember, ResourceId};
    ///
    /// fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let user_id = ResourceId::new("user-123".to_string())?;
    ///     let user_member = GroupMember::new_user(user_id, None)?;
    ///     assert!(user_member.is_user());
    ///     Ok(())
    /// }
    /// ```
    pub fn is_user(&self) -> bool {
        self.member_type.as_deref() == Some("User")
    }

    /// Returns true if this member represents a Group resource.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use scim_server::resource::value_objects::{GroupMember, ResourceId};
    ///
    /// fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let group_id = ResourceId::new("group-456".to_string())?;
    ///     let group_member = GroupMember::new_group(group_id, None)?;
    ///     assert!(group_member.is_group());
    ///     Ok(())
    /// }
    /// ```
    pub fn is_group(&self) -> bool {
        self.member_type.as_deref() == Some("Group")
    }

    /// Returns the effective display name for the member.
    ///
    /// This returns the display name if set, otherwise falls back to the resource ID.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use scim_server::resource::value_objects::{GroupMember, ResourceId};
    ///
    /// fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let user_id = ResourceId::new("user-123".to_string())?;
    ///     let member_with_name = GroupMember::new_user(user_id.clone(), Some("John".to_string()))?;
    ///     assert_eq!(member_with_name.effective_display_name(), "John");
    ///
    ///     let member_without_name = GroupMember::new_user(user_id, None)?;
    ///     assert_eq!(member_without_name.effective_display_name(), "user-123");
    ///     Ok(())
    /// }
    /// ```
    pub fn effective_display_name(&self) -> &str {
        self.display.as_deref().unwrap_or(self.value.as_str())
    }

    /// Validates a display name.
    ///
    /// # Arguments
    ///
    /// * `display_name` - The display name to validate
    ///
    /// # Returns
    ///
    /// * `Ok(())` - Display name is valid
    /// * `Err(ValidationError)` - Display name is invalid
    fn validate_display_name(display_name: &str) -> ValidationResult<()> {
        if display_name.is_empty() {
            return Err(ValidationError::custom("Display name cannot be empty"));
        }

        if display_name.len() > 256 {
            return Err(ValidationError::custom(
                "Display name cannot exceed 256 characters",
            ));
        }

        // Check for control characters
        if display_name.chars().any(|c| c.is_control() && c != '\t') {
            return Err(ValidationError::custom(
                "Display name cannot contain control characters",
            ));
        }

        Ok(())
    }

    /// Validates a member type.
    ///
    /// # Arguments
    ///
    /// * `member_type` - The member type to validate
    ///
    /// # Returns
    ///
    /// * `Ok(())` - Member type is valid
    /// * `Err(ValidationError)` - Member type is invalid
    fn validate_member_type(member_type: &str) -> ValidationResult<()> {
        if member_type.is_empty() {
            return Err(ValidationError::custom("Member type cannot be empty"));
        }

        match member_type {
            "User" | "Group" => Ok(()),
            _ => Err(ValidationError::custom(format!(
                "Invalid member type '{}'. Must be 'User' or 'Group'",
                member_type
            ))),
        }
    }
}

impl fmt::Display for GroupMember {
    /// Formats the group member for display.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match (&self.display, &self.member_type) {
            (Some(display), Some(mtype)) => {
                write!(f, "{} ({}) [{}]", display, mtype, self.value.as_str())
            }
            (Some(display), None) => write!(f, "{} [{}]", display, self.value.as_str()),
            (None, Some(mtype)) => write!(f, "({}) [{}]", mtype, self.value.as_str()),
            (None, None) => write!(f, "[{}]", self.value.as_str()),
        }
    }
}

/// Type alias for a collection of group members using MultiValuedAttribute.
///
/// This provides a type-safe way to handle multiple group members with
/// support for primary member designation if needed.
///
/// # Examples
///
/// ```rust
/// use scim_server::resource::value_objects::{GroupMembers, GroupMember, ResourceId};
///
/// fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let members = vec![
///         GroupMember::new_user(ResourceId::new("user1".to_string())?, Some("John".to_string()))?,
///         GroupMember::new_user(ResourceId::new("user2".to_string())?, Some("Jane".to_string()))?,
///     ];
///
///     let group_members = GroupMembers::new(members)?;
///     assert_eq!(group_members.len(), 2);
///     Ok(())
/// }
/// ```
pub type GroupMembers = MultiValuedAttribute<GroupMember>;

/// Type alias for a collection of email addresses using MultiValuedAttribute.
///
/// This provides a type-safe way to handle multiple email addresses with
/// support for primary email designation.
pub type MultiValuedEmails = MultiValuedAttribute<crate::resource::value_objects::EmailAddress>;

/// Type alias for a collection of addresses using MultiValuedAttribute.
///
/// This provides a type-safe way to handle multiple addresses with
/// support for primary address designation.
pub type MultiValuedAddresses = MultiValuedAttribute<crate::resource::value_objects::Address>;

/// Type alias for a collection of phone numbers using MultiValuedAttribute.
///
/// This provides a type-safe way to handle multiple phone numbers with
/// support for primary phone number designation.
pub type MultiValuedPhoneNumbers =
    MultiValuedAttribute<crate::resource::value_objects::PhoneNumber>;

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_resource_id(id: &str) -> ResourceId {
        ResourceId::new(id.to_string()).unwrap()
    }

    #[test]
    fn test_group_member_new_valid() {
        let member_id = create_test_resource_id("user-123");
        let member = GroupMember::new(
            member_id.clone(),
            Some("John Doe".to_string()),
            Some("User".to_string()),
        )
        .unwrap();

        assert_eq!(member.value(), &member_id);
        assert_eq!(member.display_name(), Some("John Doe"));
        assert_eq!(member.member_type(), Some("User"));
        assert!(member.is_user());
        assert!(!member.is_group());
    }

    #[test]
    fn test_group_member_new_user() {
        let member_id = create_test_resource_id("user-123");
        let member =
            GroupMember::new_user(member_id.clone(), Some("John Doe".to_string())).unwrap();

        assert_eq!(member.value(), &member_id);
        assert_eq!(member.display_name(), Some("John Doe"));
        assert_eq!(member.member_type(), Some("User"));
        assert!(member.is_user());
    }

    #[test]
    fn test_group_member_new_group() {
        let group_id = create_test_resource_id("group-456");
        let member =
            GroupMember::new_group(group_id.clone(), Some("Admin Group".to_string())).unwrap();

        assert_eq!(member.value(), &group_id);
        assert_eq!(member.display_name(), Some("Admin Group"));
        assert_eq!(member.member_type(), Some("Group"));
        assert!(member.is_group());
    }

    #[test]
    fn test_group_member_minimal() {
        let member_id = create_test_resource_id("user-123");
        let member = GroupMember::new(member_id.clone(), None, None).unwrap();

        assert_eq!(member.value(), &member_id);
        assert_eq!(member.display_name(), None);
        assert_eq!(member.member_type(), None);
        assert!(!member.is_user());
        assert!(!member.is_group());
    }

    #[test]
    fn test_group_member_invalid_display_name() {
        let member_id = create_test_resource_id("user-123");

        // Empty display name
        let result = GroupMember::new(member_id.clone(), Some("".to_string()), None);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("cannot be empty"));

        // Too long display name
        let long_name = "a".repeat(257);
        let result = GroupMember::new(member_id.clone(), Some(long_name), None);
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("cannot exceed 256")
        );

        // Control characters
        let result = GroupMember::new(member_id, Some("John\x00Doe".to_string()), None);
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("control characters")
        );
    }

    #[test]
    fn test_group_member_invalid_member_type() {
        let member_id = create_test_resource_id("user-123");

        // Empty member type
        let result = GroupMember::new(member_id.clone(), None, Some("".to_string()));
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("cannot be empty"));

        // Invalid member type
        let result = GroupMember::new(member_id, None, Some("Invalid".to_string()));
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Invalid member type")
        );
    }

    #[test]
    fn test_group_member_effective_display_name() {
        let member_id = create_test_resource_id("user-123");

        // With display name
        let member_with_name =
            GroupMember::new_user(member_id.clone(), Some("John Doe".to_string())).unwrap();
        assert_eq!(member_with_name.effective_display_name(), "John Doe");

        // Without display name
        let member_without_name = GroupMember::new_user(member_id.clone(), None).unwrap();
        assert_eq!(member_without_name.effective_display_name(), "user-123");
    }

    #[test]
    fn test_group_member_display() {
        let member_id = create_test_resource_id("user-123");

        // Full member
        let full_member = GroupMember::new(
            member_id.clone(),
            Some("John Doe".to_string()),
            Some("User".to_string()),
        )
        .unwrap();
        let display_str = format!("{}", full_member);
        assert!(display_str.contains("John Doe"));
        assert!(display_str.contains("User"));
        assert!(display_str.contains("user-123"));

        // Display name only
        let display_only =
            GroupMember::new(member_id.clone(), Some("John Doe".to_string()), None).unwrap();
        let display_str = format!("{}", display_only);
        assert!(display_str.contains("John Doe"));
        assert!(display_str.contains("user-123"));

        // Type only
        let type_only =
            GroupMember::new(member_id.clone(), None, Some("User".to_string())).unwrap();
        let display_str = format!("{}", type_only);
        assert!(display_str.contains("User"));
        assert!(display_str.contains("user-123"));

        // Minimal
        let minimal = GroupMember::new(member_id.clone(), None, None).unwrap();
        let display_str = format!("{}", minimal);
        assert_eq!(display_str, "[user-123]");
    }

    #[test]
    fn test_group_members_collection() {
        let member1 = GroupMember::new_user(
            create_test_resource_id("user-1"),
            Some("John Doe".to_string()),
        )
        .unwrap();
        let member2 = GroupMember::new_user(
            create_test_resource_id("user-2"),
            Some("Jane Smith".to_string()),
        )
        .unwrap();

        let members = vec![member1.clone(), member2.clone()];
        let group_members = GroupMembers::new(members).unwrap();

        assert_eq!(group_members.len(), 2);
        assert_eq!(group_members.get(0), Some(&member1));
        assert_eq!(group_members.get(1), Some(&member2));
    }

    #[test]
    fn test_group_members_with_primary() {
        let member1 = GroupMember::new_user(
            create_test_resource_id("user-1"),
            Some("John Doe".to_string()),
        )
        .unwrap();
        let member2 = GroupMember::new_user(
            create_test_resource_id("user-2"),
            Some("Jane Smith".to_string()),
        )
        .unwrap();

        let members = vec![member1.clone(), member2.clone()];
        let group_members = GroupMembers::new(members).unwrap().with_primary(1).unwrap();

        assert_eq!(group_members.primary(), Some(&member2));
        assert_eq!(group_members.primary_index(), Some(1));
    }

    #[test]
    fn test_serialization() {
        let member_id = create_test_resource_id("user-123");
        let member = GroupMember::new(
            member_id,
            Some("John Doe".to_string()),
            Some("User".to_string()),
        )
        .unwrap();

        let json = serde_json::to_string(&member).unwrap();
        let deserialized: GroupMember = serde_json::from_str(&json).unwrap();

        assert_eq!(member, deserialized);
    }

    #[test]
    fn test_serialization_optional_fields() {
        let member_id = create_test_resource_id("user-123");
        let member = GroupMember::new(member_id, None, None).unwrap();

        let json = serde_json::to_string(&member).unwrap();

        // Optional fields should not be present in JSON when None
        assert!(!json.contains("display"));
        assert!(!json.contains("type"));

        let deserialized: GroupMember = serde_json::from_str(&json).unwrap();
        assert_eq!(member, deserialized);
    }
}
