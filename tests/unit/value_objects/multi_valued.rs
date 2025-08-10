//! Integration tests for Phase 3.3: Multi-Valued Attribute Systems
//!
//! This test file demonstrates the complete functionality of the multi-valued
//! attribute system including:
//! - Generic MultiValuedAttribute container
//! - Specific multi-valued types (addresses, phones, emails)
//! - Group membership value objects
//! - Primary value designation and validation
//! - Resource integration and JSON serialization

use scim_server::error::ValidationResult;
use scim_server::resource::core::{Resource, ResourceBuilder};
use scim_server::resource::value_objects::{
    Address, EmailAddress, GroupMember, GroupMembers, MultiValuedAddresses, MultiValuedEmails,
    MultiValuedPhoneNumbers, Name, PhoneNumber, ResourceId, SchemaUri, UserName,
};
use serde_json::json;

#[test]
fn test_multi_valued_attribute_basic_functionality() -> ValidationResult<()> {
    // Create test addresses
    let work_address = Address::new_work(
        "123 Corporate Blvd".to_string(),
        "Business City".to_string(),
        "NY".to_string(),
        "10001".to_string(),
        "US".to_string(),
    )?;
    let home_address = Address::new(
        Some("456 Residential St\nApt 3B".to_string()),
        Some("456 Residential St".to_string()),
        Some("Hometown".to_string()),
        Some("CA".to_string()),
        Some("90210".to_string()),
        Some("US".to_string()),
        Some("home".to_string()),
        Some(false),
    )?;

    // Create multi-valued addresses
    let addresses = vec![work_address.clone(), home_address.clone()];
    let multi_addresses = MultiValuedAddresses::new(addresses)?;

    // Test basic functionality
    assert_eq!(multi_addresses.len(), 2);
    assert!(!multi_addresses.is_empty());
    assert!(multi_addresses.primary().is_none());

    // Test setting primary
    let with_primary = multi_addresses.with_primary(0)?;
    assert_eq!(with_primary.primary(), Some(&work_address));
    assert_eq!(with_primary.primary_index(), Some(0));

    // Test adding values
    let additional_address = Address::new_simple(
        "789 Vacation Ave".to_string(),
        "Resort Town".to_string(),
        "US".to_string(),
    )?;
    let expanded = with_primary.with_value(additional_address.clone());
    assert_eq!(expanded.len(), 3);
    assert_eq!(expanded.get(2), Some(&additional_address));

    // Test primary value addition
    let with_new_primary = expanded.with_primary_value(additional_address.clone());
    assert_eq!(with_new_primary.len(), 4);
    assert_eq!(with_new_primary.primary(), Some(&additional_address));
    assert_eq!(with_new_primary.primary_index(), Some(3));

    Ok(())
}

#[test]
fn test_multi_valued_phones_with_primary() -> ValidationResult<()> {
    // Create different types of phone numbers
    let work_phone = PhoneNumber::new_work("tel:+1-555-123-4567".to_string())?;
    let mobile_phone = PhoneNumber::new_mobile("tel:+1-555-987-6543".to_string())?;
    let home_phone = PhoneNumber::new(
        "tel:+1-555-555-5555".to_string(),
        Some("Home Line".to_string()),
        Some("home".to_string()),
        Some(false),
    )?;

    let phones = vec![work_phone.clone(), mobile_phone.clone(), home_phone];
    let multi_phones = MultiValuedPhoneNumbers::new(phones)?.with_primary(1)?;

    // Verify primary is the mobile phone
    assert_eq!(multi_phones.primary(), Some(&mobile_phone));
    assert_eq!(multi_phones.primary_index(), Some(1));

    // Test finding specific phone types
    let work_phone_found = multi_phones.find(|p| p.phone_type() == Some("work"));
    assert_eq!(work_phone_found, Some(&work_phone));

    // Test filtering by type
    let non_work_phones = multi_phones.filter(|p| p.phone_type() != Some("work"));
    assert_eq!(non_work_phones.len(), 2);

    Ok(())
}

#[test]
fn test_multi_valued_emails() -> ValidationResult<()> {
    // Create email addresses
    let personal_email = EmailAddress::new_simple("personal@example.com".to_string())?;
    let work_email = EmailAddress::new_simple("work@company.com".to_string())?;
    let backup_email = EmailAddress::new_simple("backup@backup.com".to_string())?;

    // Test single email creation
    let single_email = MultiValuedEmails::single_primary(work_email.clone());
    assert_eq!(single_email.len(), 1);
    assert_eq!(single_email.primary(), Some(&work_email));

    // Test multiple emails
    let emails = vec![personal_email.clone(), work_email.clone(), backup_email];
    let multi_emails = MultiValuedEmails::new(emails)?;

    // Test without primary
    let without_primary = multi_emails.clone().without_primary();
    assert!(without_primary.primary().is_none());

    // Test iteration
    let collected_emails: Vec<&EmailAddress> = multi_emails.iter().collect();
    assert_eq!(collected_emails.len(), 3);
    assert_eq!(collected_emails[0], &personal_email);

    Ok(())
}

#[test]
fn test_group_members_functionality() -> ValidationResult<()> {
    // Create individual group members
    let user1_id = ResourceId::new("user-123".to_string())?;
    let user1_member = GroupMember::new_user(user1_id, Some("John Doe".to_string()))?;

    let user2_id = ResourceId::new("user-456".to_string())?;
    let user2_member = GroupMember::new_user(user2_id, Some("Jane Smith".to_string()))?;

    let group_id = ResourceId::new("group-789".to_string())?;
    let group_member = GroupMember::new_group(group_id, Some("Admin Group".to_string()))?;

    // Test individual member properties
    assert!(user1_member.is_user());
    assert!(!user1_member.is_group());
    assert_eq!(user1_member.display_name(), Some("John Doe"));
    assert_eq!(user1_member.effective_display_name(), "John Doe");

    assert!(group_member.is_group());
    assert!(!group_member.is_user());

    // Create group members collection
    let members = vec![user1_member.clone(), user2_member.clone(), group_member];
    let group_members = GroupMembers::new(members)?.with_primary(0)?;

    // Test collection functionality
    assert_eq!(group_members.len(), 3);
    assert_eq!(group_members.primary(), Some(&user1_member));

    // Test finding users vs groups
    let user_members = group_members.filter(|m| m.is_user());
    assert_eq!(user_members.len(), 2);

    let group_members_filtered = group_members.filter(|m| m.is_group());
    assert_eq!(group_members_filtered.len(), 1);

    Ok(())
}

#[test]
fn test_resource_integration_with_multi_valued_attributes() -> ValidationResult<()> {
    // Create complex user resource with all multi-valued attributes
    let user_name = UserName::new("jdoe".to_string())?;
    let name = Name::new(
        Some("John Doe".to_string()),
        Some("Doe".to_string()),
        Some("John".to_string()),
        Some("Q".to_string()),
        Some("Mr".to_string()),
        Some("Jr".to_string()),
    )?;

    // Create addresses
    let work_address = Address::new_work(
        "123 Business Ave".to_string(),
        "Corporate City".to_string(),
        "NY".to_string(),
        "10001".to_string(),
        "US".to_string(),
    )?;
    let home_address = Address::new(
        Some("456 Home St".to_string()),
        Some("456 Home St".to_string()),
        Some("Hometown".to_string()),
        Some("CA".to_string()),
        Some("90210".to_string()),
        Some("US".to_string()),
        Some("home".to_string()),
        Some(true), // This is primary
    )?;

    let multi_addresses = MultiValuedAddresses::new(vec![work_address, home_address])?;

    // Create phone numbers
    let work_phone = PhoneNumber::new_work("tel:+1-555-123-4567".to_string())?;
    let mobile_phone = PhoneNumber::new_mobile("tel:+1-555-987-6543".to_string())?;
    let multi_phones =
        MultiValuedPhoneNumbers::new(vec![work_phone, mobile_phone])?.with_primary(1)?;

    // Create emails
    let work_email = EmailAddress::new_simple("jdoe@company.com".to_string())?;
    let personal_email = EmailAddress::new_simple("john.doe@personal.com".to_string())?;
    let multi_emails = MultiValuedEmails::new(vec![work_email, personal_email])?.with_primary(0)?;

    // Build resource using ResourceBuilder
    let user_schema = SchemaUri::new("urn:ietf:params:scim:schemas:core:2.0:User".to_string())?;
    let resource = ResourceBuilder::new("User".to_string())
        .with_username(user_name)
        .with_name(name)
        .add_schema(user_schema)
        .with_addresses(multi_addresses)
        .with_phone_numbers(multi_phones)
        .with_emails(multi_emails)
        .build()?;

    // Test resource access methods
    assert!(resource.get_username().is_some());
    assert!(resource.get_name().is_some());

    let addresses = resource.get_addresses().expect("Should have addresses");
    assert_eq!(addresses.len(), 2);
    // Work address should not be primary, home address should be primary
    // (based on the Address value object's primary field)

    let phones = resource.get_phone_numbers().expect("Should have phones");
    assert_eq!(phones.len(), 2);
    assert_eq!(phones.primary_index(), Some(1)); // Mobile is primary

    let emails = resource.get_emails().expect("Should have emails");
    assert_eq!(emails.len(), 2);
    assert_eq!(emails.primary_index(), Some(0)); // Work email is primary

    // Test JSON serialization
    let json_output = resource.to_json()?;
    assert!(json_output.get("addresses").is_some());
    assert!(json_output.get("phoneNumbers").is_some());
    assert!(json_output.get("emails").is_some());

    Ok(())
}

#[test]
fn test_resource_json_round_trip_with_multi_valued() -> ValidationResult<()> {
    // Create JSON data with multi-valued attributes
    let json_data = json!({
        "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
        "id": "user-123",
        "userName": "testuser",
        "name": {
            "formatted": "Test User",
            "familyName": "User",
            "givenName": "Test"
        },
        "addresses": [
            {
                "formatted": "123 Main St\nAnytown, CA 12345\nUS",
                "streetAddress": "123 Main St",
                "locality": "Anytown",
                "region": "CA",
                "postalCode": "12345",
                "country": "US",
                "type": "work",
                "primary": true
            },
            {
                "streetAddress": "456 Home Ave",
                "locality": "Hometown",
                "region": "NY",
                "postalCode": "54321",
                "country": "US",
                "type": "home",
                "primary": false
            }
        ],
        "phoneNumbers": [
            {
                "value": "tel:+1-555-555-5555",
                "type": "work",
                "primary": false
            },
            {
                "value": "tel:+1-555-123-4567",
                "type": "mobile",
                "primary": true
            }
        ],
        "emails": [
            {
                "value": "work@example.com",
                "type": "work",
                "primary": true
            },
            {
                "value": "personal@example.com",
                "type": "personal",
                "primary": false
            }
        ]
    });

    // Parse from JSON
    let resource = Resource::from_json("User".to_string(), json_data)?;

    // Verify multi-valued attributes were parsed correctly
    let addresses = resource.get_addresses().expect("Should have addresses");
    assert_eq!(addresses.len(), 2);
    let work_addr = addresses.get(0).expect("Should have work address");
    assert_eq!(work_addr.address_type(), Some("work"));
    assert_eq!(work_addr.is_primary(), true);

    let phones = resource.get_phone_numbers().expect("Should have phones");
    assert_eq!(phones.len(), 2);
    let mobile_phone = phones.get(1).expect("Should have mobile phone");
    assert_eq!(mobile_phone.phone_type(), Some("mobile"));
    assert_eq!(mobile_phone.is_primary(), true);

    let emails = resource.get_emails().expect("Should have emails");
    assert_eq!(emails.len(), 2);
    let work_email = emails.get(0).expect("Should have work email");
    assert_eq!(work_email.value(), "work@example.com");

    // Convert back to JSON
    let output_json = resource.to_json()?;

    // Verify key fields are preserved
    assert!(output_json.get("addresses").is_some());
    assert!(output_json.get("phoneNumbers").is_some());
    assert!(output_json.get("emails").is_some());

    let output_addresses = output_json.get("addresses").unwrap().as_array().unwrap();
    assert_eq!(output_addresses.len(), 2);

    Ok(())
}

#[test]
fn test_group_resource_with_members() -> ValidationResult<()> {
    // Create group members
    let user1_id = ResourceId::new("user-001".to_string())?;
    let user1_member = GroupMember::new_user(user1_id, Some("Alice Johnson".to_string()))?;

    let user2_id = ResourceId::new("user-002".to_string())?;
    let user2_member = GroupMember::new_user(user2_id, Some("Bob Wilson".to_string()))?;

    let subgroup_id = ResourceId::new("group-sub".to_string())?;
    let subgroup_member = GroupMember::new_group(subgroup_id, Some("Sub Admins".to_string()))?;

    // Create group members collection
    let members = vec![user1_member, user2_member, subgroup_member];
    let group_members = GroupMembers::new(members)?;

    // Create group resource
    let group_schema = SchemaUri::new("urn:ietf:params:scim:schemas:core:2.0:Group".to_string())?;
    let group_resource = ResourceBuilder::new("Group".to_string())
        .add_schema(group_schema)
        .with_members(group_members)
        .build()?;

    // Verify group functionality
    let members = group_resource.get_members().expect("Should have members");
    assert_eq!(members.len(), 3);

    // Count users vs groups
    let user_count = members.filter(|m| m.is_user()).len();
    let group_count = members.filter(|m| m.is_group()).len();
    assert_eq!(user_count, 2);
    assert_eq!(group_count, 1);

    // Test JSON serialization includes members
    let json_output = group_resource.to_json()?;
    assert!(json_output.get("members").is_some());

    let members_json = json_output.get("members").unwrap().as_array().unwrap();
    assert_eq!(members_json.len(), 3);

    Ok(())
}

#[test]
fn test_multi_valued_attribute_error_cases() {
    // Test empty collection fails validation
    let empty_addresses: Vec<Address> = vec![];
    let result = MultiValuedAddresses::new(empty_addresses);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("cannot be empty"));

    // Test invalid primary index
    let address = Address::new_simple(
        "123 Test St".to_string(),
        "TestCity".to_string(),
        "US".to_string(),
    )
    .unwrap();
    let addresses = vec![address];
    let multi_addresses = MultiValuedAddresses::new(addresses).unwrap();
    let result = multi_addresses.with_primary(5);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("out of bounds"));
}

#[test]
fn test_group_member_validation() {
    // Test invalid display name
    let user_id = ResourceId::new("user-123".to_string()).unwrap();

    // Empty display name
    let result = GroupMember::new(
        user_id.clone(),
        Some("".to_string()),
        Some("User".to_string()),
    );
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("cannot be empty"));

    // Invalid member type
    let result = GroupMember::new(
        user_id.clone(),
        Some("Valid Name".to_string()),
        Some("Invalid".to_string()),
    );
    assert!(result.is_err());
    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("Invalid member type")
    );

    // Valid member creation
    let result = GroupMember::new_user(user_id, Some("Valid User".to_string()));
    assert!(result.is_ok());
}

#[test]
fn test_phase_3_3_complete_functionality() -> ValidationResult<()> {
    // This test demonstrates the complete Phase 3.3 functionality:
    // 1. Generic MultiValuedAttribute container
    // 2. Specific multi-valued types
    // 3. Group membership objects
    // 4. Resource integration
    // 5. Primary value support
    // 6. JSON serialization/deserialization

    println!("Phase 3.3: Multi-Valued Attribute Systems - Complete Test");

    // 1. Create a comprehensive user with all multi-valued attributes
    let user_name = UserName::new("comprehensive_user".to_string())?;
    let name = Name::new(
        Some("Comprehensive User".to_string()),
        Some("User".to_string()),
        Some("Comprehensive".to_string()),
        Some("T".to_string()),
        Some("Dr".to_string()),
        Some("PhD".to_string()),
    )?;

    // Multiple addresses with primary
    let work_addr = Address::new_work(
        "123 Corporate Plaza".to_string(),
        "Business City".to_string(),
        "NY".to_string(),
        "10001".to_string(),
        "US".to_string(),
    )?;
    let home_addr = Address::new(
        Some("456 Residential Lane".to_string()),
        Some("456 Residential Lane".to_string()),
        Some("Hometown".to_string()),
        Some("CA".to_string()),
        Some("90210".to_string()),
        Some("US".to_string()),
        Some("home".to_string()),
        Some(true), // Primary address
    )?;
    let vacation_addr = Address::new_simple(
        "789 Beach Road".to_string(),
        "Resort City".to_string(),
        "US".to_string(),
    )?;

    let addresses = MultiValuedAddresses::new(vec![work_addr, home_addr, vacation_addr])?;

    // Multiple phone numbers with primary
    let work_phone = PhoneNumber::new_work("tel:+1-555-WORK-123".to_string())?;
    let mobile_phone = PhoneNumber::new_mobile("tel:+1-555-MOBILE-1".to_string())?;
    let home_phone = PhoneNumber::new(
        "tel:+1-555-HOME-456".to_string(),
        Some("Home Landline".to_string()),
        Some("home".to_string()),
        Some(false),
    )?;

    let phones = MultiValuedPhoneNumbers::new(vec![work_phone, mobile_phone, home_phone])?
        .with_primary(1)?; // Mobile is primary

    // Multiple emails with primary
    let work_email = EmailAddress::new_simple("work@company.com".to_string())?;
    let personal_email = EmailAddress::new_simple("personal@email.com".to_string())?;
    let backup_email = EmailAddress::new_simple("backup@backup.com".to_string())?;

    let emails =
        MultiValuedEmails::new(vec![work_email, personal_email, backup_email])?.with_primary(0)?; // Work is primary

    // 2. Create user resource
    let user_schema = SchemaUri::new("urn:ietf:params:scim:schemas:core:2.0:User".to_string())?;
    let user_resource = ResourceBuilder::new("User".to_string())
        .with_username(user_name)
        .with_name(name)
        .add_schema(user_schema)
        .with_addresses(addresses)
        .with_phone_numbers(phones)
        .with_emails(emails)
        .build()?;

    // 3. Create a group with the user as a member
    let user_id = ResourceId::new("comprehensive-user-id".to_string())?;
    let admin_id = ResourceId::new("admin-user-id".to_string())?;
    let subgroup_id = ResourceId::new("subgroup-id".to_string())?;

    let user_member = GroupMember::new_user(user_id, Some("Comprehensive User".to_string()))?;
    let admin_member = GroupMember::new_user(admin_id, Some("Admin User".to_string()))?;
    let subgroup_member = GroupMember::new_group(subgroup_id, Some("Nested Group".to_string()))?;

    let group_members =
        GroupMembers::new(vec![user_member, admin_member, subgroup_member])?.with_primary(0)?;

    let group_schema = SchemaUri::new("urn:ietf:params:scim:schemas:core:2.0:Group".to_string())?;
    let group_resource = ResourceBuilder::new("Group".to_string())
        .add_schema(group_schema)
        .with_members(group_members)
        .build()?;

    // 4. Verify all functionality

    // User resource verification
    let user_addresses = user_resource
        .get_addresses()
        .expect("User should have addresses");
    assert_eq!(user_addresses.len(), 3);
    println!("✓ User has {} addresses", user_addresses.len());

    let user_phones = user_resource
        .get_phone_numbers()
        .expect("User should have phones");
    assert_eq!(user_phones.len(), 3);
    assert_eq!(user_phones.primary_index(), Some(1)); // Mobile is primary
    println!("✓ User has {} phones, mobile is primary", user_phones.len());

    let user_emails = user_resource.get_emails().expect("User should have emails");
    assert_eq!(user_emails.len(), 3);
    assert_eq!(user_emails.primary_index(), Some(0)); // Work is primary
    println!("✓ User has {} emails, work is primary", user_emails.len());

    // Group resource verification
    let members = group_resource
        .get_members()
        .expect("Group should have members");
    assert_eq!(members.len(), 3);
    assert_eq!(members.primary_index(), Some(0)); // First user is primary

    let user_members_count = members.filter(|m| m.is_user()).len();
    let group_members_count = members.filter(|m| m.is_group()).len();
    assert_eq!(user_members_count, 2);
    assert_eq!(group_members_count, 1);
    println!(
        "✓ Group has {} members ({} users, {} groups)",
        members.len(),
        user_members_count,
        group_members_count
    );

    // 5. Test JSON round-trip
    let user_json = user_resource.to_json()?;
    let group_json = group_resource.to_json()?;

    // Verify JSON structure
    assert!(user_json.get("addresses").is_some());
    assert!(user_json.get("phoneNumbers").is_some());
    assert!(user_json.get("emails").is_some());
    assert!(group_json.get("members").is_some());
    println!("✓ JSON serialization includes all multi-valued attributes");

    // Parse back from JSON
    let user_from_json = Resource::from_json("User".to_string(), user_json)?;
    let group_from_json = Resource::from_json("Group".to_string(), group_json)?;

    // Verify round-trip preservation
    assert_eq!(user_from_json.get_addresses().unwrap().len(), 3);
    assert_eq!(user_from_json.get_phone_numbers().unwrap().len(), 3);
    assert_eq!(user_from_json.get_emails().unwrap().len(), 3);
    assert_eq!(group_from_json.get_members().unwrap().len(), 3);
    println!("✓ JSON round-trip preserves all multi-valued attributes");

    println!("\n🎉 Phase 3.3: Multi-Valued Attribute Systems - All tests passed!");
    println!("✅ Generic MultiValuedAttribute container");
    println!("✅ Specific multi-valued types (addresses, phones, emails)");
    println!("✅ Group membership value objects");
    println!("✅ Primary value designation and validation");
    println!("✅ Complete Resource integration");
    println!("✅ JSON serialization/deserialization");

    Ok(())
}
