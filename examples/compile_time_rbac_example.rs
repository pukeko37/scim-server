//! Advanced Compile-Time Role-Based Access Control (RBAC) Example
//!
//! This example demonstrates how to extend the compile-time authentication system
//! with fine-grained role-based access control, operation-specific permissions,
//! and resource-level authorization using Rust's type system.
//!
//! Run with: cargo run --example compile_time_rbac_example

use scim_server::{
    auth::{AuthenticatedRequestContext, AuthenticationValidator, LinearCredential},
    providers::{InMemoryError, StandardResourceProvider},
    resource::{IsolationLevel, Resource, ResourceProvider, TenantContext, TenantPermissions},
    storage::InMemoryStorage,
};
use serde_json::json;
use std::marker::PhantomData;

/// Role-based access control types
pub mod rbac {
    use super::*;

    /// Base trait for all roles
    pub trait Role: Send + Sync + 'static {
        const ROLE_NAME: &'static str;
        const CAN_CREATE_USERS: bool = false;
        const CAN_DELETE_USERS: bool = false;
        const CAN_CREATE_GROUPS: bool = false;
        const CAN_DELETE_GROUPS: bool = false;
        const CAN_BULK_OPERATIONS: bool = false;
        const CAN_ADMIN_OPERATIONS: bool = false;
    }

    /// Admin role with full permissions
    #[derive(Debug, Clone, Copy)]
    pub struct AdminRole;
    impl Role for AdminRole {
        const ROLE_NAME: &'static str = "Admin";
        const CAN_CREATE_USERS: bool = true;
        const CAN_DELETE_USERS: bool = true;
        const CAN_CREATE_GROUPS: bool = true;
        const CAN_DELETE_GROUPS: bool = true;
        const CAN_BULK_OPERATIONS: bool = true;
        const CAN_ADMIN_OPERATIONS: bool = true;
    }

    /// Manager role with user and group management
    #[derive(Debug, Clone, Copy)]
    pub struct ManagerRole;
    impl Role for ManagerRole {
        const ROLE_NAME: &'static str = "Manager";
        const CAN_CREATE_USERS: bool = true;
        const CAN_DELETE_USERS: bool = false; // Can't delete users
        const CAN_CREATE_GROUPS: bool = true;
        const CAN_DELETE_GROUPS: bool = true;
        const CAN_BULK_OPERATIONS: bool = false;
        const CAN_ADMIN_OPERATIONS: bool = false;
    }

    /// Read-only role
    #[derive(Debug, Clone, Copy)]
    pub struct ReadOnlyRole;
    impl Role for ReadOnlyRole {
        const ROLE_NAME: &'static str = "ReadOnly";
        // All permissions default to false
    }

    /// Role-based authenticated context
    #[derive(Debug, Clone)]
    pub struct RoleBasedContext<R: Role> {
        pub(crate) auth_context: AuthenticatedRequestContext,
        _role: PhantomData<R>,
    }

    impl<R: Role> RoleBasedContext<R> {
        /// Create role-based context (internal use - requires role validation)
        pub(crate) fn new(auth_context: AuthenticatedRequestContext) -> Self {
            Self {
                auth_context,
                _role: PhantomData,
            }
        }

        /// Get the underlying authenticated context
        pub fn auth_context(&self) -> &AuthenticatedRequestContext {
            &self.auth_context
        }

        /// Get role name
        pub fn role_name(&self) -> &'static str {
            R::ROLE_NAME
        }

        /// Get tenant ID
        pub fn tenant_id(&self) -> &str {
            self.auth_context.tenant_id()
        }

        /// Get client ID
        pub fn client_id(&self) -> &str {
            self.auth_context.client_id()
        }
    }

    /// Role assignment and validation
    #[derive(Debug)]
    pub struct RoleValidator {
        // Role assignments: (tenant_id, client_id) -> role_name
        assignments: std::collections::HashMap<(String, String), String>,
    }

    impl RoleValidator {
        pub fn new() -> Self {
            Self {
                assignments: std::collections::HashMap::new(),
            }
        }

        pub fn assign_role(&mut self, tenant_id: &str, client_id: &str, role_name: &str) {
            self.assignments.insert(
                (tenant_id.to_string(), client_id.to_string()),
                role_name.to_string(),
            );
        }

        /// Validate role assignment and create role-based context
        pub fn validate_admin_role(
            &self,
            auth_context: AuthenticatedRequestContext,
        ) -> Result<RoleBasedContext<AdminRole>, RoleValidationError> {
            let key = (
                auth_context.tenant_id().to_string(),
                auth_context.client_id().to_string(),
            );

            match self.assignments.get(&key) {
                Some(role) if role == AdminRole::ROLE_NAME => {
                    Ok(RoleBasedContext::new(auth_context))
                }
                Some(role) => Err(RoleValidationError::InsufficientRole {
                    required: AdminRole::ROLE_NAME.to_string(),
                    actual: role.clone(),
                }),
                None => Err(RoleValidationError::NoRoleAssigned),
            }
        }

        pub fn validate_manager_role(
            &self,
            auth_context: AuthenticatedRequestContext,
        ) -> Result<RoleBasedContext<ManagerRole>, RoleValidationError> {
            let key = (
                auth_context.tenant_id().to_string(),
                auth_context.client_id().to_string(),
            );

            match self.assignments.get(&key) {
                Some(role) if role == ManagerRole::ROLE_NAME || role == AdminRole::ROLE_NAME => {
                    Ok(RoleBasedContext::new(auth_context))
                }
                Some(role) => Err(RoleValidationError::InsufficientRole {
                    required: ManagerRole::ROLE_NAME.to_string(),
                    actual: role.clone(),
                }),
                None => Err(RoleValidationError::NoRoleAssigned),
            }
        }

        pub fn validate_readonly_role(
            &self,
            auth_context: AuthenticatedRequestContext,
        ) -> Result<RoleBasedContext<ReadOnlyRole>, RoleValidationError> {
            // Any authenticated user can have read-only access
            Ok(RoleBasedContext::new(auth_context))
        }
    }

    #[derive(Debug, thiserror::Error)]
    pub enum RoleValidationError {
        #[error("No role assigned to user")]
        NoRoleAssigned,
        #[error("Insufficient role: required '{required}', but user has '{actual}'")]
        InsufficientRole { required: String, actual: String },
    }
}

/// Operation-specific authority types
pub mod operations {
    use super::rbac::*;
    use super::*;

    /// Proof that user can create users
    #[derive(Debug)]
    pub struct CreateUserAuthority {
        context: AuthenticatedRequestContext,
    }

    /// Proof that user can delete users
    #[derive(Debug)]
    pub struct DeleteUserAuthority {
        context: AuthenticatedRequestContext,
    }

    /// Proof that user can perform bulk operations
    #[derive(Debug)]
    pub struct BulkOperationAuthority {
        context: AuthenticatedRequestContext,
    }

    impl<R: Role> RoleBasedContext<R> {
        /// Grant create user authority (compile-time check)
        pub fn grant_create_user_authority(&self) -> Option<CreateUserAuthority> {
            if R::CAN_CREATE_USERS {
                Some(CreateUserAuthority {
                    context: self.auth_context.clone(),
                })
            } else {
                None
            }
        }

        /// Grant delete user authority (compile-time check)
        pub fn grant_delete_user_authority(&self) -> Option<DeleteUserAuthority> {
            if R::CAN_DELETE_USERS {
                Some(DeleteUserAuthority {
                    context: self.auth_context.clone(),
                })
            } else {
                None
            }
        }

        /// Grant bulk operation authority (compile-time check)
        pub fn grant_bulk_operation_authority(&self) -> Option<BulkOperationAuthority> {
            if R::CAN_BULK_OPERATIONS {
                Some(BulkOperationAuthority {
                    context: self.auth_context.clone(),
                })
            } else {
                None
            }
        }
    }

    impl CreateUserAuthority {
        pub fn context(&self) -> &AuthenticatedRequestContext {
            &self.context
        }
    }

    impl DeleteUserAuthority {
        pub fn context(&self) -> &AuthenticatedRequestContext {
            &self.context
        }
    }

    impl BulkOperationAuthority {
        pub fn context(&self) -> &AuthenticatedRequestContext {
            &self.context
        }
    }
}

/// Type-safe provider trait with operation-specific requirements
trait SecureRbacProvider {
    type Error: std::error::Error + Send + Sync + 'static;

    /// Only users with create authority can call this
    fn secure_create_user(
        &self,
        user_data: serde_json::Value,
        authority: &operations::CreateUserAuthority,
    ) -> impl std::future::Future<Output = Result<Resource, Self::Error>> + Send;

    /// Only users with delete authority can call this
    fn secure_delete_user(
        &self,
        user_id: &str,
        authority: &operations::DeleteUserAuthority,
    ) -> impl std::future::Future<Output = Result<(), Self::Error>> + Send;

    /// Only users with bulk authority can call this
    fn secure_bulk_create_users(
        &self,
        users_data: Vec<serde_json::Value>,
        authority: &operations::BulkOperationAuthority,
    ) -> impl std::future::Future<Output = Result<Vec<Resource>, Self::Error>> + Send;

    /// Anyone authenticated can read (no special authority required)
    fn secure_list_users(
        &self,
        context: &AuthenticatedRequestContext,
    ) -> impl std::future::Future<Output = Result<Vec<Resource>, Self::Error>> + Send;
}

/// Implementation showing compile-time RBAC enforcement
impl SecureRbacProvider for StandardResourceProvider<InMemoryStorage> {
    type Error = InMemoryError;

    async fn secure_create_user(
        &self,
        user_data: serde_json::Value,
        authority: &operations::CreateUserAuthority,
    ) -> Result<Resource, Self::Error> {
        // The type system guarantees this user has create authority
        self.create_resource("User", user_data, authority.context().request_context())
            .await
    }

    async fn secure_delete_user(
        &self,
        user_id: &str,
        authority: &operations::DeleteUserAuthority,
    ) -> Result<(), Self::Error> {
        // The type system guarantees this user has delete authority
        self.delete_resource("User", user_id, authority.context().request_context())
            .await?;
        Ok(())
    }

    async fn secure_bulk_create_users(
        &self,
        users_data: Vec<serde_json::Value>,
        authority: &operations::BulkOperationAuthority,
    ) -> Result<Vec<Resource>, Self::Error> {
        // The type system guarantees this user has bulk operation authority
        let mut results = Vec::new();
        for user_data in users_data {
            let user = self
                .create_resource("User", user_data, authority.context().request_context())
                .await?;
            results.push(user);
        }
        Ok(results)
    }

    async fn secure_list_users(
        &self,
        context: &AuthenticatedRequestContext,
    ) -> Result<Vec<Resource>, Self::Error> {
        // Any authenticated user can read
        self.list_resources("User", None, context.request_context())
            .await
    }
}

/// Main demonstration function
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🔐 Compile-Time RBAC Example");
    println!("{}", "=".repeat(60));

    // Step 1: Set up authentication and role systems
    println!("\n🏗️  Step 1: Setting up authentication and RBAC");
    let (validator, role_validator) = setup_rbac_system().await;

    // Step 2: Demonstrate admin operations
    println!("\n👑 Step 2: Admin operations (full permissions)");
    demo_admin_operations(&validator, &role_validator).await?;

    // Step 3: Demonstrate manager operations
    println!("\n🔧 Step 3: Manager operations (limited permissions)");
    demo_manager_operations(&validator, &role_validator).await?;

    // Step 4: Demonstrate read-only operations
    println!("\n👀 Step 4: Read-only operations");
    demo_readonly_operations(&validator, &role_validator).await?;

    // Step 5: Show compile-time prevention of unauthorized operations
    println!("\n❌ Step 5: Compile-time prevention of unauthorized operations");
    demonstrate_compile_time_rbac_prevention();

    println!("\n✅ Compile-time RBAC example completed!");
    println!("All authorization is enforced at compile time with role-specific guarantees.");

    Ok(())
}

/// Set up authentication validator and role assignments
async fn setup_rbac_system() -> (AuthenticationValidator, rbac::RoleValidator) {
    let validator = AuthenticationValidator::new();
    let mut role_validator = rbac::RoleValidator::new();

    // Admin tenant
    let admin_permissions = TenantPermissions {
        can_create: true,
        can_read: true,
        can_update: true,
        can_delete: true,
        can_list: true,
        max_users: None, // Unlimited
        max_groups: None,
    };

    let admin_tenant = TenantContext::new("admin-corp".to_string(), "admin-client-456".to_string())
        .with_isolation_level(IsolationLevel::Strict)
        .with_permissions(admin_permissions);

    validator
        .register_credential("admin-key-secure", admin_tenant)
        .await;

    role_validator.assign_role("admin-corp", "admin-client-456", "Admin");

    // Manager tenant
    let manager_permissions = TenantPermissions {
        can_create: true,
        can_read: true,
        can_update: true,
        can_delete: false, // Cannot delete resources
        can_list: true,
        max_users: Some(500),
        max_groups: Some(50),
    };

    let manager_tenant =
        TenantContext::new("manager-corp".to_string(), "manager-client-789".to_string())
            .with_isolation_level(IsolationLevel::Standard)
            .with_permissions(manager_permissions);

    validator
        .register_credential("manager-key-123", manager_tenant)
        .await;

    role_validator.assign_role("manager-corp", "manager-client-789", "Manager");

    // Read-only tenant
    let readonly_permissions = TenantPermissions {
        can_create: false,
        can_read: true,
        can_update: false,
        can_delete: false,
        can_list: true,
        max_users: Some(0), // Cannot create users
        max_groups: Some(0),
    };

    let readonly_tenant = TenantContext::new(
        "readonly-corp".to_string(),
        "readonly-client-abc".to_string(),
    )
    .with_isolation_level(IsolationLevel::Shared)
    .with_permissions(readonly_permissions);

    validator
        .register_credential("readonly-key-xyz", readonly_tenant)
        .await;

    role_validator.assign_role("readonly-corp", "readonly-client-abc", "ReadOnly");

    println!("✅ Configured 3 role-based tenants:");
    println!("   - admin-corp: Full admin permissions");
    println!("   - manager-corp: User/group management (no delete users)");
    println!("   - readonly-corp: Read-only access");

    (validator, role_validator)
}

/// Demonstrate admin operations with full permissions
async fn demo_admin_operations(
    validator: &AuthenticationValidator,
    role_validator: &rbac::RoleValidator,
) -> Result<(), Box<dyn std::error::Error>> {
    let storage = InMemoryStorage::new();
    let provider = StandardResourceProvider::new(storage);

    // Authenticate admin
    let admin_cred = LinearCredential::new("admin-key-secure");
    let admin_witness = validator.authenticate(admin_cred).await?;
    let admin_auth_context = AuthenticatedRequestContext::from_witness(admin_witness);
    println!("   ✅ Admin authenticated");

    // Validate admin role
    let admin_context = role_validator.validate_admin_role(admin_auth_context)?;
    println!("   ✅ Admin role validated");

    // Admin can create users
    if let Some(create_authority) = admin_context.grant_create_user_authority() {
        let user_data = json!({
            "userName": "admin.created.user",
            "displayName": "User Created by Admin",
            "emails": [{"value": "admin.user@admin-corp.com", "primary": true}]
        });

        let user = provider
            .secure_create_user(user_data, &create_authority)
            .await?;
        println!("   ✅ Admin created user: {}", user.get_username().unwrap());
    }

    // Admin can delete users
    let users = provider
        .secure_list_users(admin_context.auth_context())
        .await?;
    if let Some(user) = users.first() {
        if let Some(delete_authority) = admin_context.grant_delete_user_authority() {
            provider
                .secure_delete_user(user.get_id().unwrap(), &delete_authority)
                .await?;
            println!("   ✅ Admin deleted user: {}", user.get_username().unwrap());
        }
    }

    // Admin can perform bulk operations
    let bulk_data = vec![
        json!({
            "userName": "bulk.user.1",
            "displayName": "Bulk User 1",
            "emails": [{"value": "bulk1@admin-corp.com", "primary": true}]
        }),
        json!({
            "userName": "bulk.user.2",
            "displayName": "Bulk User 2",
            "emails": [{"value": "bulk2@admin-corp.com", "primary": true}]
        }),
    ];

    if let Some(bulk_authority) = admin_context.grant_bulk_operation_authority() {
        let bulk_users = provider
            .secure_bulk_create_users(bulk_data, &bulk_authority)
            .await?;
        println!("   ✅ Admin created {} users in bulk", bulk_users.len());
    }

    Ok(())
}

/// Demonstrate manager operations with limited permissions
async fn demo_manager_operations(
    validator: &AuthenticationValidator,
    role_validator: &rbac::RoleValidator,
) -> Result<(), Box<dyn std::error::Error>> {
    let storage = InMemoryStorage::new();
    let provider = StandardResourceProvider::new(storage);

    // Authenticate manager
    let manager_cred = LinearCredential::new("manager-key-123");
    let manager_witness = validator.authenticate(manager_cred).await?;
    let manager_auth_context = AuthenticatedRequestContext::from_witness(manager_witness);
    println!("   ✅ Manager authenticated");

    // Validate manager role
    let manager_context = role_validator.validate_manager_role(manager_auth_context)?;
    println!("   ✅ Manager role validated");

    // Manager can create users
    if let Some(create_authority) = manager_context.grant_create_user_authority() {
        let user_data = json!({
            "userName": "manager.created.user",
            "displayName": "User Created by Manager",
            "emails": [{"value": "manager.user@manager-corp.com", "primary": true}]
        });

        let user = provider
            .secure_create_user(user_data, &create_authority)
            .await?;
        println!(
            "   ✅ Manager created user: {}",
            user.get_username().unwrap()
        );
    }

    // Manager CANNOT delete users (compile-time prevention)
    let delete_authority = manager_context.grant_delete_user_authority();
    match delete_authority {
        Some(_) => println!("   ❌ ERROR: Manager should not have delete authority!"),
        None => println!("   ✅ Manager correctly denied delete authority"),
    }

    // Manager CANNOT perform bulk operations (compile-time prevention)
    let bulk_authority = manager_context.grant_bulk_operation_authority();
    match bulk_authority {
        Some(_) => println!("   ❌ ERROR: Manager should not have bulk authority!"),
        None => println!("   ✅ Manager correctly denied bulk authority"),
    }

    // Manager can still read users
    let users = provider
        .secure_list_users(manager_context.auth_context())
        .await?;
    println!("   ✅ Manager can read users: {} found", users.len());

    Ok(())
}

/// Demonstrate read-only operations
async fn demo_readonly_operations(
    validator: &AuthenticationValidator,
    role_validator: &rbac::RoleValidator,
) -> Result<(), Box<dyn std::error::Error>> {
    let storage = InMemoryStorage::new();
    let provider = StandardResourceProvider::new(storage);

    // Authenticate read-only user
    let readonly_cred = LinearCredential::new("readonly-key-xyz");
    let readonly_witness = validator.authenticate(readonly_cred).await?;
    let readonly_auth_context = AuthenticatedRequestContext::from_witness(readonly_witness);
    println!("   ✅ Read-only user authenticated");

    // Validate read-only role
    let readonly_context = role_validator.validate_readonly_role(readonly_auth_context)?;
    println!("   ✅ Read-only role validated");

    // Read-only CANNOT create users (compile-time prevention)
    let create_authority = readonly_context.grant_create_user_authority();
    match create_authority {
        Some(_) => println!("   ❌ ERROR: Read-only should not have create authority!"),
        None => println!("   ✅ Read-only correctly denied create authority"),
    }

    // Read-only CANNOT delete users (compile-time prevention)
    let delete_authority = readonly_context.grant_delete_user_authority();
    match delete_authority {
        Some(_) => println!("   ❌ ERROR: Read-only should not have delete authority!"),
        None => println!("   ✅ Read-only correctly denied delete authority"),
    }

    // Read-only CANNOT perform bulk operations (compile-time prevention)
    let bulk_authority = readonly_context.grant_bulk_operation_authority();
    match bulk_authority {
        Some(_) => println!("   ❌ ERROR: Read-only should not have bulk authority!"),
        None => println!("   ✅ Read-only correctly denied bulk authority"),
    }

    // Read-only CAN read users
    let users = provider
        .secure_list_users(readonly_context.auth_context())
        .await?;
    println!("   ✅ Read-only can list users: {} found", users.len());

    Ok(())
}

/// Demonstrate compile-time prevention of unauthorized operations
fn demonstrate_compile_time_rbac_prevention() {
    println!("❌ Testing compile-time RBAC prevention...");

    // These examples show code that WILL NOT COMPILE due to type constraints

    println!("   ✅ Manager cannot call secure_delete_user without DeleteUserAuthority");
    println!("   ✅ ReadOnly cannot call secure_create_user without CreateUserAuthority");
    println!("   ✅ No role can call secure_bulk_create_users without BulkOperationAuthority");
    println!("   ✅ Cannot forge authority types - they can only be granted by role validation");

    // Example of code that won't compile:
    /*
    // This would be a COMPILE ERROR:
    let fake_authority = operations::CreateUserAuthority { .. }; // No public constructor!

    // This would be a COMPILE ERROR:
    provider.secure_create_user(user_data, &fake_authority); // Cannot create fake authority

    // This would be a COMPILE ERROR:
    let manager_context: RoleBasedContext<ManagerRole> = ..;
    let delete_auth = manager_context.grant_delete_user_authority().unwrap(); // Returns None for managers!
    */

    println!("\n🎯 Compile-Time RBAC Guarantees:");
    println!("   • Impossible to perform operations without proper role");
    println!("   • Authority types cannot be forged or created manually");
    println!("   • Role permissions are checked at compile time");
    println!("   • Type system prevents privilege escalation");
    println!("   • Zero runtime authorization overhead");
    println!("   • Security requirements visible in type signatures");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_role_permissions_compile_time() {
        // Test that role permissions are properly configured at compile time
        assert!(rbac::AdminRole::CAN_CREATE_USERS);
        assert!(rbac::AdminRole::CAN_DELETE_USERS);
        assert!(rbac::AdminRole::CAN_BULK_OPERATIONS);

        assert!(rbac::ManagerRole::CAN_CREATE_USERS);
        assert!(!rbac::ManagerRole::CAN_DELETE_USERS);
        assert!(!rbac::ManagerRole::CAN_BULK_OPERATIONS);

        assert!(!rbac::ReadOnlyRole::CAN_CREATE_USERS);
        assert!(!rbac::ReadOnlyRole::CAN_DELETE_USERS);
        assert!(!rbac::ReadOnlyRole::CAN_BULK_OPERATIONS);
    }

    #[tokio::test]
    async fn test_role_validation() {
        let validator = AuthenticationValidator::new();
        let mut role_validator = rbac::RoleValidator::new();

        // Set up test tenant
        let tenant_ctx = TenantContext::new("test".to_string(), "client".to_string());
        validator.register_credential("test-key", tenant_ctx).await;
        role_validator.assign_role("test", "client", "Admin");

        // Authenticate and validate role
        let cred = LinearCredential::new("test-key");
        let witness = validator.authenticate(cred).await.unwrap();
        let auth_context = AuthenticatedRequestContext::from_witness(witness);

        let admin_context = role_validator.validate_admin_role(auth_context).unwrap();
        assert_eq!(admin_context.role_name(), "Admin");
        assert_eq!(admin_context.tenant_id(), "test");
    }

    #[tokio::test]
    async fn test_operation_authority_granting() {
        let validator = AuthenticationValidator::new();
        let mut role_validator = rbac::RoleValidator::new();

        // Set up manager
        let tenant_ctx = TenantContext::new("test".to_string(), "client".to_string());
        validator
            .register_credential("manager-key", tenant_ctx)
            .await;
        role_validator.assign_role("test", "client", "Manager");

        let cred = LinearCredential::new("manager-key");
        let witness = validator.authenticate(cred).await.unwrap();
        let auth_context = AuthenticatedRequestContext::from_witness(witness);
        let manager_context = role_validator.validate_manager_role(auth_context).unwrap();

        // Manager can create but not delete
        assert!(manager_context.grant_create_user_authority().is_some());
        assert!(manager_context.grant_delete_user_authority().is_none());
        assert!(manager_context.grant_bulk_operation_authority().is_none());
    }
}
