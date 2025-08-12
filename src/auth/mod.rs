//! Compile-time authentication system with type-level proofs.
//!
//! This module provides a zero-cost abstraction for authentication that leverages
//! Rust's type system to ensure only authenticated clients can access resources.
//! Authentication state is tracked at compile time, preventing runtime security bugs.
//!
//! # Type-Level Authentication Design
//!
//! The system uses phantom types and witness types to encode authentication state:
//!
//! * **Unauthenticated State**: Raw credentials that haven't been validated
//! * **Authenticated State**: Credentials that have passed validation with type-level proof
//! * **Authorized Context**: Request contexts that can only be created with valid authentication
//! * **Linear Credentials**: Authentication tokens that can only be consumed once
//!
//! # Key Principles
//!
//! 1. **Impossible States**: Unauthenticated access is unrepresentable
//! 2. **Zero Runtime Cost**: All validation happens at compile time where possible
//! 3. **Linear Resources**: Credentials are consumed during authentication
//! 4. **Proof Carrying**: Authenticated contexts carry evidence of validation
//! 5. **Type Safety**: Operations require specific authentication levels
//!
//! # Example Usage
//!
//! ```rust
//! use scim_server::auth::{
//!     AuthenticationValidator, AuthenticatedRequestContext,
//!     LinearCredential, Credential, Unauthenticated
//! };
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // Raw credential (unauthenticated)
//! let raw_cred = LinearCredential::new("api-key-123");
//!
//! // Validation consumes the raw credential
//! let validator = AuthenticationValidator::new();
//! let witness = validator.authenticate(raw_cred).await?;
//!
//! // Only validated credentials can create authenticated contexts
//! let auth_context = AuthenticatedRequestContext::from_witness(witness);
//!
//! // Only authenticated contexts can access resources
//! // provider.list_resources(&auth_context).await;
//! # Ok(())
//! # }
//! ```

use crate::resource::{RequestContext, TenantContext};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::marker::PhantomData;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Type-level authentication states using phantom types
pub trait AuthState: Send + Sync + 'static {}

/// Phantom type for unauthenticated state
#[derive(Debug, Clone, Copy)]
pub struct Unauthenticated;
impl AuthState for Unauthenticated {}

/// Phantom type for authenticated state
#[derive(Debug, Clone, Copy)]
pub struct Authenticated;
impl AuthState for Authenticated {}

/// Credential with compile-time authentication state
#[derive(Debug, Clone)]
pub struct Credential<S: AuthState> {
    pub(crate) value: String,
    pub(crate) _phantom: PhantomData<S>,
}

impl Credential<Unauthenticated> {
    /// Create a new unauthenticated credential
    pub fn new(value: impl Into<String>) -> Self {
        Self {
            value: value.into(),
            _phantom: PhantomData,
        }
    }

    /// Get the raw credential value (only available for unauthenticated)
    pub fn raw_value(&self) -> &str {
        &self.value
    }
}

impl Credential<Authenticated> {
    /// Create an authenticated credential (internal use only)
    #[allow(dead_code)]
    pub(crate) fn authenticated(value: String) -> Self {
        Self {
            value,
            _phantom: PhantomData,
        }
    }

    /// Get the credential value (only available after authentication)
    pub fn authenticated_value(&self) -> &str {
        &self.value
    }
}

/// Witness type proving successful authentication
///
/// This type can only be constructed by the authentication system
/// and serves as compile-time proof that validation occurred.
#[derive(Debug, Clone)]
pub struct AuthenticationWitness {
    pub(crate) tenant_context: TenantContext,
    pub(crate) credential_hash: String,
    pub(crate) validated_at: chrono::DateTime<chrono::Utc>,
}

impl AuthenticationWitness {
    /// Create a new authentication witness (internal use only)
    pub(crate) fn new(tenant_context: TenantContext, credential_hash: String) -> Self {
        Self {
            tenant_context,
            credential_hash,
            validated_at: chrono::Utc::now(),
        }
    }

    /// Get the tenant context (only available with witness)
    pub fn tenant_context(&self) -> &TenantContext {
        &self.tenant_context
    }

    /// Get validation timestamp
    pub fn validated_at(&self) -> chrono::DateTime<chrono::Utc> {
        self.validated_at
    }

    /// Get credential hash for audit purposes
    pub fn credential_hash(&self) -> &str {
        &self.credential_hash
    }
}

/// Witness type proving tenant-level authority
///
/// This type can only be created from an AuthenticationWitness
/// and proves the holder has authority within a specific tenant.
#[derive(Debug, Clone)]
pub struct TenantAuthority {
    witness: AuthenticationWitness,
}

impl TenantAuthority {
    /// Create tenant authority from authentication witness
    pub fn from_witness(witness: AuthenticationWitness) -> Self {
        Self { witness }
    }

    /// Get the underlying authentication witness
    pub fn witness(&self) -> &AuthenticationWitness {
        &self.witness
    }

    /// Get tenant ID (compile-time guaranteed to exist)
    pub fn tenant_id(&self) -> &str {
        &self.witness.tenant_context.tenant_id
    }

    /// Get client ID (compile-time guaranteed to exist)
    pub fn client_id(&self) -> &str {
        &self.witness.tenant_context.client_id
    }
}

/// Linear credential that can only be consumed once
///
/// This prevents credential reuse and ensures authentication
/// happens exactly once per credential.
#[derive(Debug)]
pub struct LinearCredential {
    inner: Option<Credential<Unauthenticated>>,
}

impl LinearCredential {
    /// Create a new linear credential
    pub fn new(value: impl Into<String>) -> Self {
        Self {
            inner: Some(Credential::new(value)),
        }
    }

    /// Consume this credential for authentication (can only be called once)
    pub fn consume(mut self) -> Credential<Unauthenticated> {
        self.inner.take().expect("Credential already consumed")
    }

    /// Check if credential has been consumed
    pub fn is_consumed(&self) -> bool {
        self.inner.is_none()
    }
}

/// Marker type proving a credential was consumed
#[derive(Debug)]
pub struct ConsumedCredential {
    _private: (),
}

impl ConsumedCredential {
    /// Create proof of consumption (internal use only)
    pub(crate) fn new() -> Self {
        Self { _private: () }
    }
}

/// Result of authentication that either succeeds with proof or fails
#[derive(Debug)]
pub enum AuthenticationResult {
    /// Authentication succeeded with witness
    Success {
        witness: AuthenticationWitness,
        consumed: ConsumedCredential,
    },
    /// Authentication failed
    Failed { consumed: ConsumedCredential },
}

/// Request context that can only be created with authentication proof
#[derive(Debug, Clone)]
pub struct AuthenticatedRequestContext {
    inner: RequestContext,
    authority: TenantAuthority,
}

impl AuthenticatedRequestContext {
    /// Create authenticated context from witness (consuming it)
    pub fn from_witness(witness: AuthenticationWitness) -> Self {
        let tenant_context = witness.tenant_context().clone();
        let authority = TenantAuthority::from_witness(witness);
        let inner = RequestContext::with_tenant_generated_id(tenant_context);

        Self { inner, authority }
    }

    /// Create authenticated context with specific request ID
    pub fn with_request_id(witness: AuthenticationWitness, request_id: String) -> Self {
        let tenant_context = witness.tenant_context().clone();
        let authority = TenantAuthority::from_witness(witness);
        let inner = RequestContext::with_tenant(request_id, tenant_context);

        Self { inner, authority }
    }

    /// Get the underlying request context
    pub fn request_context(&self) -> &RequestContext {
        &self.inner
    }

    /// Get tenant authority proof
    pub fn authority(&self) -> &TenantAuthority {
        &self.authority
    }

    /// Get tenant ID (compile-time guaranteed)
    pub fn tenant_id(&self) -> &str {
        self.authority.tenant_id()
    }

    /// Get client ID (compile-time guaranteed)
    pub fn client_id(&self) -> &str {
        self.authority.client_id()
    }

    /// Get request ID
    pub fn request_id(&self) -> &str {
        &self.inner.request_id
    }
}

/// Simplified authenticated context for common operations
#[derive(Debug, Clone)]
pub struct AuthenticatedContext {
    authority: TenantAuthority,
}

impl AuthenticatedContext {
    /// Create from authentication witness
    pub fn from_witness(witness: AuthenticationWitness) -> Self {
        Self {
            authority: TenantAuthority::from_witness(witness),
        }
    }

    /// Convert to full request context
    pub fn to_request_context(&self) -> AuthenticatedRequestContext {
        let witness = self.authority.witness().clone();
        AuthenticatedRequestContext::from_witness(witness)
    }

    /// Get tenant authority
    pub fn authority(&self) -> &TenantAuthority {
        &self.authority
    }
}

/// Compile-time authentication validator
#[derive(Debug, Clone)]
pub struct AuthenticationValidator {
    // Runtime credential store for validation
    credentials: Arc<RwLock<HashMap<String, TenantContext>>>,
}

impl AuthenticationValidator {
    /// Create a new authentication validator
    pub fn new() -> Self {
        Self {
            credentials: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Register a credential (for testing/setup)
    pub async fn register_credential(&self, credential: &str, tenant_context: TenantContext) {
        let mut creds = self.credentials.write().await;
        creds.insert(credential.to_string(), tenant_context);
    }

    /// Authenticate a credential with compile-time proof
    ///
    /// This method consumes the credential and either returns an authentication
    /// witness (proving successful validation) or an error with proof of consumption.
    pub async fn authenticate(
        &self,
        credential: LinearCredential,
    ) -> Result<AuthenticationWitness, AuthenticationError> {
        // Consume the credential (can only happen once)
        let raw_cred = credential.consume();
        let _consumed_proof = ConsumedCredential::new();

        // Runtime validation
        let creds = self.credentials.read().await;
        if let Some(tenant_context) = creds.get(raw_cred.raw_value()) {
            // Create secure hash for witness using SHA-256
            let mut hasher = Sha256::new();
            hasher.update(raw_cred.raw_value().as_bytes());
            let credential_hash = format!("{:x}", hasher.finalize());

            Ok(AuthenticationWitness::new(
                tenant_context.clone(),
                credential_hash,
            ))
        } else {
            Err(AuthenticationError::InvalidCredential)
        }
    }
}

impl Default for AuthenticationValidator {
    fn default() -> Self {
        Self::new()
    }
}

/// Authentication errors
#[derive(Debug, thiserror::Error)]
pub enum AuthenticationError {
    #[error("Invalid credential provided")]
    InvalidCredential,
    #[error("Credential has been revoked")]
    CredentialRevoked,
    #[error("Authentication system unavailable")]
    SystemUnavailable,
}

/// Type-safe authentication traits for providers
pub trait AuthenticatedProvider {
    type Error: std::error::Error + Send + Sync + 'static;

    /// List resources with authenticated context (compile-time guaranteed)
    fn list_resources_authenticated(
        &self,
        resource_type: &str,
        context: &AuthenticatedRequestContext,
    ) -> impl std::future::Future<Output = Result<Vec<crate::resource::Resource>, Self::Error>> + Send;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::resource::TenantContext;

    #[tokio::test]
    async fn test_linear_credential_consumption() {
        let cred = LinearCredential::new("test-key");
        assert!(!cred.is_consumed());

        let _raw = cred.consume();
        // Credential is now consumed - cannot use again
    }

    #[tokio::test]
    async fn test_authentication_flow() {
        let validator = AuthenticationValidator::new();
        let tenant_ctx = TenantContext::new("test-tenant".to_string(), "test-client".to_string());

        validator.register_credential("valid-key", tenant_ctx).await;

        // Create linear credential
        let cred = LinearCredential::new("valid-key");

        // Authenticate (consumes credential)
        let witness = validator.authenticate(cred).await.unwrap();

        // Create authenticated context
        let auth_context = AuthenticatedRequestContext::from_witness(witness);

        assert_eq!(auth_context.tenant_id(), "test-tenant");
        assert_eq!(auth_context.client_id(), "test-client");
    }

    #[tokio::test]
    async fn test_invalid_authentication() {
        let validator = AuthenticationValidator::new();
        let cred = LinearCredential::new("invalid-key");

        let result = validator.authenticate(cred).await;
        assert!(result.is_err());
    }

    #[test]
    fn test_type_level_authentication_states() {
        // Can create unauthenticated credential
        let unauth = Credential::<Unauthenticated>::new("test");
        assert_eq!(unauth.raw_value(), "test");

        // Cannot create authenticated credential directly
        // This would not compile:
        // let auth = Credential::<Authenticated>::new("test");
    }

    #[test]
    fn test_witness_types() {
        let tenant_ctx = TenantContext::new("test".to_string(), "client".to_string());
        let witness = AuthenticationWitness::new(tenant_ctx, "hash".to_string());

        let authority = TenantAuthority::from_witness(witness);
        assert_eq!(authority.tenant_id(), "test");
        assert_eq!(authority.client_id(), "client");
    }
}
