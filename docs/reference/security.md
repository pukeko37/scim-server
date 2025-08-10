# Security Guide

This guide covers security considerations, best practices, and implementation strategies for running the SCIM Server securely in production environments.

## Table of Contents

- [Security Overview](#security-overview)
- [Authentication](#authentication)
- [Authorization](#authorization)
- [Transport Security](#transport-security)
- [Input Validation](#input-validation)
- [Data Protection](#data-protection)
- [Multi-Tenant Security](#multi-tenant-security)
- [Audit Logging](#audit-logging)
- [Security Headers](#security-headers)
- [Vulnerability Management](#vulnerability-management)
- [Compliance](#compliance)

## Security Overview

The SCIM Server implements defense-in-depth security with multiple layers of protection:

```
┌─────────────────────────────────────────────────────────┐
│                    Network Layer                        │
│              (TLS, Firewalls, CDN)                      │
├─────────────────────────────────────────────────────────┤
│                 Application Layer                       │
│         (Authentication, Authorization)                 │
├─────────────────────────────────────────────────────────┤
│                  Protocol Layer                         │
│             (SCIM Validation, Rate Limiting)            │
├─────────────────────────────────────────────────────────┤
│                   Data Layer                            │
│        (Encryption, Access Controls, Audit)             │
├─────────────────────────────────────────────────────────┤
│                Infrastructure Layer                     │
│         (OS Security, Container Security)               │
└─────────────────────────────────────────────────────────┘
```

### Security Principles

1. **Zero Trust** - Never trust, always verify
2. **Least Privilege** - Minimal necessary permissions
3. **Defense in Depth** - Multiple security layers
4. **Fail Secure** - Secure defaults when systems fail
5. **Audit Everything** - Comprehensive logging and monitoring

## Authentication

### Bearer Token Authentication

The most common authentication method for SCIM servers:

```rust
use scim_server::auth::{BearerTokenAuth, TokenValidator};
use scim_server::error::{Result, ScimError};

// JWT token validation
pub struct JwtTokenValidator {
    jwks_url: String,
    issuer: String,
    audience: String,
}

impl JwtTokenValidator {
    pub fn new(jwks_url: String, issuer: String, audience: String) -> Self {
        Self { jwks_url, issuer, audience }
    }
}

#[async_trait]
impl TokenValidator for JwtTokenValidator {
    async fn validate_token(&self, token: &str) -> Result<AuthContext> {
        // Decode and validate JWT
        let token_data = jsonwebtoken::decode::<Claims>(
            token,
            &self.get_decoding_key().await?,
            &self.get_validation_params(),
        ).map_err(|e| ScimError::Unauthorized {
            realm: Some("SCIM API".to_string()),
            message: format!("Invalid token: {}", e),
        })?;

        // Extract claims
        let claims = token_data.claims;
        
        // Validate issuer and audience
        if claims.iss != self.issuer {
            return Err(ScimError::Unauthorized {
                realm: Some("SCIM API".to_string()),
                message: "Invalid token issuer".to_string(),
            });
        }
        
        if !claims.aud.contains(&self.audience) {
            return Err(ScimError::Unauthorized {
                realm: Some("SCIM API".to_string()),
                message: "Invalid token audience".to_string(),
            });
        }

        Ok(AuthContext {
            user_id: claims.sub,
            tenant_id: claims.tenant_id,
            scopes: claims.scope.split_whitespace().map(String::from).collect(),
            expires_at: claims.exp,
        })
    }
}

#[derive(Debug, Clone)]
pub struct AuthContext {
    pub user_id: String,
    pub tenant_id: Option<String>,
    pub scopes: Vec<String>,
    pub expires_at: i64,
}

#[derive(Debug, Serialize, Deserialize)]
struct Claims {
    sub: String,
    iss: String,
    aud: Vec<String>,
    exp: i64,
    iat: i64,
    scope: String,
    tenant_id: Option<String>,
}
```

### OAuth 2.0 Integration

```rust
use oauth2::{
    AuthorizationCode, ClientId, ClientSecret, CsrfToken, TokenResponse,
    basic::BasicClient, reqwest::async_http_client
};

pub struct OAuth2Authenticator {
    client: BasicClient,
    scopes: Vec<String>,
}

impl OAuth2Authenticator {
    pub fn new(
        client_id: ClientId,
        client_secret: ClientSecret,
        auth_url: String,
        token_url: String,
    ) -> Self {
        let client = BasicClient::new(client_id, Some(client_secret), auth_url, Some(token_url));
        
        Self {
            client,
            scopes: vec!["scim:read".to_string(), "scim:write".to_string()],
        }
    }
    
    pub async fn validate_authorization_code(
        &self,
        code: AuthorizationCode,
    ) -> Result<AuthContext> {
        let token_result = self.client
            .exchange_code(code)
            .request_async(async_http_client)
            .await
            .map_err(|e| ScimError::Unauthorized {
                realm: Some("OAuth2".to_string()),
                message: format!("Token exchange failed: {}", e),
            })?;

        // Validate the access token
        let access_token = token_result.access_token().secret();
        self.validate_access_token(access_token).await
    }
    
    async fn validate_access_token(&self, token: &str) -> Result<AuthContext> {
        // Token introspection or JWT validation
        let response = reqwest::Client::new()
            .post("https://auth.example.com/introspect")
            .form(&[("token", token)])
            .send()
            .await
            .map_err(|e| ScimError::internal_error(format!("Token introspection failed: {}", e)))?;
        
        let introspection: TokenIntrospection = response.json().await
            .map_err(|e| ScimError::internal_error(format!("Invalid introspection response: {}", e)))?;
        
        if !introspection.active {
            return Err(ScimError::Unauthorized {
                realm: Some("OAuth2".to_string()),
                message: "Token is not active".to_string(),
            });
        }
        
        Ok(AuthContext {
            user_id: introspection.sub,
            tenant_id: introspection.tenant_id,
            scopes: introspection.scope.split_whitespace().map(String::from).collect(),
            expires_at: introspection.exp,
        })
    }
}

#[derive(Debug, Deserialize)]
struct TokenIntrospection {
    active: bool,
    sub: String,
    scope: String,
    exp: i64,
    tenant_id: Option<String>,
}
```

### API Key Authentication

```rust
pub struct ApiKeyAuth {
    keys: HashMap<String, ApiKeyInfo>,
    hash_algorithm: HashAlgorithm,
}

impl ApiKeyAuth {
    pub fn new() -> Self {
        Self {
            keys: HashMap::new(),
            hash_algorithm: HashAlgorithm::Sha256,
        }
    }
    
    pub fn add_api_key(&mut self, key: &str, info: ApiKeyInfo) -> Result<()> {
        let hashed_key = self.hash_key(key)?;
        self.keys.insert(hashed_key, info);
        Ok(())
    }
    
    fn hash_key(&self, key: &str) -> Result<String> {
        use sha2::{Sha256, Digest};
        
        let mut hasher = Sha256::new();
        hasher.update(key.as_bytes());
        Ok(format!("{:x}", hasher.finalize()))
    }
}

#[async_trait]
impl TokenValidator for ApiKeyAuth {
    async fn validate_token(&self, token: &str) -> Result<AuthContext> {
        let hashed_token = self.hash_key(token)?;
        
        let key_info = self.keys.get(&hashed_token)
            .ok_or_else(|| ScimError::Unauthorized {
                realm: Some("API Key".to_string()),
                message: "Invalid API key".to_string(),
            })?;
        
        // Check expiration
        if let Some(expires_at) = key_info.expires_at {
            let now = chrono::Utc::now().timestamp();
            if now > expires_at {
                return Err(ScimError::Unauthorized {
                    realm: Some("API Key".to_string()),
                    message: "API key has expired".to_string(),
                });
            }
        }
        
        Ok(AuthContext {
            user_id: key_info.owner.clone(),
            tenant_id: key_info.tenant_id.clone(),
            scopes: key_info.scopes.clone(),
            expires_at: key_info.expires_at.unwrap_or(i64::MAX),
        })
    }
}

#[derive(Debug, Clone)]
pub struct ApiKeyInfo {
    pub owner: String,
    pub tenant_id: Option<String>,
    pub scopes: Vec<String>,
    pub expires_at: Option<i64>,
    pub created_at: i64,
    pub last_used: Option<i64>,
}
```

## Authorization

### Role-Based Access Control (RBAC)

```rust
use std::collections::HashSet;

#[derive(Debug, Clone)]
pub struct RbacAuthorizer {
    roles: HashMap<String, Role>,
    user_roles: HashMap<String, HashSet<String>>,
}

#[derive(Debug, Clone)]
pub struct Role {
    pub name: String,
    pub permissions: HashSet<Permission>,
    pub resource_scopes: Vec<ResourceScope>,
}

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub enum Permission {
    ReadUsers,
    WriteUsers,
    DeleteUsers,
    ReadGroups,
    WriteGroups,
    DeleteGroups,
    ManageSchema,
    ViewMetrics,
    ManageTenants,
}

#[derive(Debug, Clone)]
pub struct ResourceScope {
    pub resource_type: String,
    pub filter: Option<String>,  // SCIM filter expression
}

impl RbacAuthorizer {
    pub fn new() -> Self {
        let mut authorizer = Self {
            roles: HashMap::new(),
            user_roles: HashMap::new(),
        };
        
        // Define default roles
        authorizer.add_role(Role {
            name: "admin".to_string(),
            permissions: vec![
                Permission::ReadUsers, Permission::WriteUsers, Permission::DeleteUsers,
                Permission::ReadGroups, Permission::WriteGroups, Permission::DeleteGroups,
                Permission::ManageSchema, Permission::ViewMetrics, Permission::ManageTenants,
            ].into_iter().collect(),
            resource_scopes: vec![ResourceScope {
                resource_type: "*".to_string(),
                filter: None,
            }],
        });
        
        authorizer.add_role(Role {
            name: "user_manager".to_string(),
            permissions: vec![
                Permission::ReadUsers, Permission::WriteUsers,
                Permission::ReadGroups, Permission::WriteGroups,
            ].into_iter().collect(),
            resource_scopes: vec![
                ResourceScope {
                    resource_type: "User".to_string(),
                    filter: Some("active eq true".to_string()),
                },
                ResourceScope {
                    resource_type: "Group".to_string(),
                    filter: None,
                },
            ],
        });
        
        authorizer.add_role(Role {
            name: "readonly".to_string(),
            permissions: vec![Permission::ReadUsers, Permission::ReadGroups].into_iter().collect(),
            resource_scopes: vec![ResourceScope {
                resource_type: "*".to_string(),
                filter: Some("active eq true".to_string()),
            }],
        });
        
        authorizer
    }
    
    pub fn add_role(&mut self, role: Role) {
        self.roles.insert(role.name.clone(), role);
    }
    
    pub fn assign_role_to_user(&mut self, user_id: &str, role_name: &str) -> Result<()> {
        if !self.roles.contains_key(role_name) {
            return Err(ScimError::bad_request(format!("Unknown role: {}", role_name)));
        }
        
        self.user_roles
            .entry(user_id.to_string())
            .or_insert_with(HashSet::new)
            .insert(role_name.to_string());
        
        Ok(())
    }
    
    pub async fn authorize_operation(
        &self,
        auth_context: &AuthContext,
        operation: Operation,
        resource: Option<&Resource>,
    ) -> Result<()> {
        let user_roles = self.user_roles.get(&auth_context.user_id)
            .ok_or_else(|| ScimError::Forbidden {
                resource: None,
                action: Some(operation.to_string()),
                message: "User has no assigned roles".to_string(),
            })?;
        
        for role_name in user_roles {
            if let Some(role) = self.roles.get(role_name) {
                if self.check_permission(role, &operation, resource).await? {
                    return Ok(()); // Authorized
                }
            }
        }
        
        Err(ScimError::Forbidden {
            resource: resource.map(|r| r.resource_type().to_string()),
            action: Some(operation.to_string()),
            message: "Insufficient permissions for this operation".to_string(),
        })
    }
    
    async fn check_permission(
        &self,
        role: &Role,
        operation: &Operation,
        resource: Option<&Resource>,
    ) -> Result<bool> {
        // Check if role has required permission
        let required_permission = operation.required_permission();
        if !role.permissions.contains(&required_permission) {
            return Ok(false);
        }
        
        // Check resource scope restrictions
        if let Some(resource) = resource {
            for scope in &role.resource_scopes {
                if scope.resource_type == "*" || scope.resource_type == resource.resource_type() {
                    // Check filter constraints
                    if let Some(filter) = &scope.filter {
                        return self.evaluate_filter(filter, resource).await;
                    } else {
                        return Ok(true); // No filter restrictions
                    }
                }
            }
            return Ok(false); // No matching scope found
        }
        
        Ok(true) // Operation doesn't involve a specific resource
    }
    
    async fn evaluate_filter(&self, filter: &str, resource: &Resource) -> Result<bool> {
        let filter_expr = FilterExpression::parse(filter)?;
        Ok(filter_expr.evaluate(resource)?)
    }
}

#[derive(Debug, Clone)]
pub enum Operation {
    CreateUser,
    ReadUser,
    UpdateUser,
    DeleteUser,
    CreateGroup,
    ReadGroup,
    UpdateGroup,
    DeleteGroup,
    SearchResources,
    ViewSchema,
    ManageSchema,
}

impl Operation {
    pub fn required_permission(&self) -> Permission {
        match self {
            Operation::CreateUser | Operation::UpdateUser => Permission::WriteUsers,
            Operation::ReadUser => Permission::ReadUsers,
            Operation::DeleteUser => Permission::DeleteUsers,
            Operation::CreateGroup | Operation::UpdateGroup => Permission::WriteGroups,
            Operation::ReadGroup => Permission::ReadGroups,
            Operation::DeleteGroup => Permission::DeleteGroups,
            Operation::SearchResources => Permission::ReadUsers, // Minimum permission
            Operation::ViewSchema => Permission::ReadUsers,
            Operation::ManageSchema => Permission::ManageSchema,
        }
    }
}
```

### Attribute-Based Access Control (ABAC)

```rust
pub struct AbacAuthorizer {
    policy_engine: PolicyEngine,
}

#[derive(Debug, Clone)]
pub struct PolicyContext {
    pub subject: Subject,
    pub resource: Resource,
    pub action: Action,
    pub environment: Environment,
}

#[derive(Debug, Clone)]
pub struct Subject {
    pub user_id: String,
    pub roles: Vec<String>,
    pub department: Option<String>,
    pub security_clearance: Option<String>,
}

#[derive(Debug, Clone)]
pub struct Action {
    pub operation: String,
    pub resource_type: String,
    pub attributes: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct Environment {
    pub tenant_id: String,
    pub request_time: chrono::DateTime<chrono::Utc>,
    pub source_ip: std::net::IpAddr,
    pub user_agent: Option<String>,
}

impl AbacAuthorizer {
    pub async fn evaluate_policy(&self, context: PolicyContext) -> Result<AuthorizationDecision> {
        let policies = self.policy_engine.get_applicable_policies(&context).await?;
        
        for policy in policies {
            match policy.evaluate(&context).await? {
                PolicyResult::Allow => return Ok(AuthorizationDecision::Allow),
                PolicyResult::Deny => return Ok(AuthorizationDecision::Deny),
                PolicyResult::NotApplicable => continue,
            }
        }
        
        // Default deny
        Ok(AuthorizationDecision::Deny)
    }
}

// Example ABAC policies
impl PolicyEngine {
    fn create_sample_policies() -> Vec<Policy> {
        vec![
            // Policy 1: Department managers can read their department's users
            Policy {
                name: "department_manager_read".to_string(),
                condition: "subject.roles contains 'department_manager' AND resource.type == 'User' AND resource.department == subject.department".to_string(),
                effect: PolicyEffect::Allow,
            },
            
            // Policy 2: HR can read all users but not sensitive attributes
            Policy {
                name: "hr_user_access".to_string(),
                condition: "subject.department == 'HR' AND resource.type == 'User'".to_string(),
                effect: PolicyEffect::AllowWithExclusions(vec!["salary".to_string(), "ssn".to_string()]),
            },
            
            // Policy 3: Deny access outside business hours for non-critical roles
            Policy {
                name: "business_hours_restriction".to_string(),
                condition: "NOT subject.roles contains 'critical_ops' AND (environment.request_time.hour < 8 OR environment.request_time.hour > 18)".to_string(),
                effect: PolicyEffect::Deny,
            },
        ]
    }
}
```

## Transport Security

### TLS Configuration

```rust
use rustls::{Certificate, PrivateKey, ServerConfig as TlsConfig};
use std::fs;

pub struct TlsConfigBuilder {
    cert_path: Option<String>,
    key_path: Option<String>,
    ca_cert_path: Option<String>,
    require_client_cert: bool,
    min_tls_version: TlsVersion,
}

impl TlsConfigBuilder {
    pub fn new() -> Self {
        Self {
            cert_path: None,
            key_path: None,
            ca_cert_path: None,
            require_client_cert: false,
            min_tls_version: TlsVersion::V1_2,
        }
    }
    
    pub fn cert_file<P: AsRef<std::path::Path>>(mut self, path: P) -> Self {
        self.cert_path = Some(path.as_ref().to_string_lossy().to_string());
        self
    }
    
    pub fn key_file<P: AsRef<std::path::Path>>(mut self, path: P) -> Self {
        self.key_path = Some(path.as_ref().to_string_lossy().to_string());
        self
    }
    
    pub fn require_client_certificates(mut self, require: bool) -> Self {
        self.require_client_cert = require;
        self
    }
    
    pub fn min_tls_version(mut self, version: TlsVersion) -> Self {
        self.min_tls_version = version;
        self
    }
    
    pub async fn build(self) -> Result<TlsConfig> {
        let cert_path = self.cert_path.ok_or_else(|| 
            ScimError::bad_request("Certificate file path required"))?;
        let key_path = self.key_path.ok_or_else(|| 
            ScimError::bad_request("Private key file path required"))?;
        
        // Load certificate and key
        let cert_data = fs::read(&cert_path)
            .map_err(|e| ScimError::internal_error(format!("Failed to read certificate: {}", e)))?;
        let key_data = fs::read(&key_path)
            .map_err(|e| ScimError::internal_error(format!("Failed to read private key: {}", e)))?;
        
        let certs = rustls_pemfile::certs(&mut cert_data.as_slice())
            .map_err(|e| ScimError::internal_error(format!("Invalid certificate format: {}", e)))?
            .into_iter()
            .map(Certificate)
            .collect();
        
        let mut keys = rustls_pemfile::pkcs8_private_keys(&mut key_data.as_slice())
            .map_err(|e| ScimError::internal_error(format!("Invalid private key format: {}", e)))?;
        
        if keys.is_empty() {
            return Err(ScimError::internal_error("No private keys found"));
        }
        
        let key = PrivateKey(keys.remove(0));
        
        // Configure TLS
        let mut config = TlsConfig::builder()
            .with_safe_default_cipher_suites()
            .with_safe_default_kx_groups()
            .with_protocol_versions(&[&rustls::version::TLS13, &rustls::version::TLS12])
            .map_err(|e| ScimError::internal_error(format!("TLS configuration error: {}", e)))?
            .with_no_client_auth() // Configure based on requirements
            .with_single_cert(certs, key)
            .map_err(|e| ScimError::internal_error(format!("Certificate configuration error: {}", e)))?;
        
        Ok(config)
    }
}

#[derive(Debug, Clone)]
pub enum TlsVersion {
    V1_2,
    V1_3,
}
```

### Certificate Management

```rust
use x509_parser::{certificate::X509Certificate, prelude::*};

pub struct CertificateManager {
    cert_path: String,
    key_path: String,
    renewal_threshold: Duration,
}

impl CertificateManager {
    pub async fn check_certificate_expiry(&self) -> Result<CertificateStatus> {
        let cert_data = tokio::fs::read(&self.cert_path).await
            .map_err(|e| ScimError::internal_error(format!("Failed to read certificate: {}", e)))?;
        
        let (_, cert) = X509Certificate::from_der(&cert_data)
            .map_err(|e| ScimError::internal_error(format!("Failed to parse certificate: {}", e)))?;
        
        let expiry = cert.validity().not_after.to_datetime();
        let now = chrono::Utc::now();
        let time_until_expiry = expiry.signed_duration_since(now);
        
        if time_until_expiry < chrono::Duration::zero() {
            Ok(CertificateStatus::Expired)
        } else if time_until_expiry < chrono::Duration::from_std(self.renewal_threshold).unwrap() {
            Ok(CertificateStatus::ExpiringSoon {
                expires_at: expiry,
                time_remaining: time_until_expiry.to_std().unwrap(),
            })
        } else {
            Ok(CertificateStatus::Valid {
                expires_at: expiry,
            })
        }
    }
    
    pub async fn auto_renew_certificate(&self) -> Result<()> {
        match self.check_certificate_expiry().await? {
            CertificateStatus::ExpiringLoon { .. } => {
                info!("Certificate expiring soon, attempting renewal...");
                self.renew_certificate().await?;
                info!("Certificate renewed successfully");
            }
            CertificateStatus::Expired => {
                error!("Certificate has expired!");
                return Err(ScimError::internal_error("SSL certificate has expired"));
            }
            CertificateStatus::Valid { .. } => {
                debug!("Certificate is still valid");
            }
        }
        
        Ok(())
    }
    
    async fn renew_certificate(&self) -> Result<()> {
        // Implement certificate renewal logic
        // This could integrate with Let's Encrypt, internal CA, etc.
        todo!("Implement certificate renewal")
    }
}

#[derive(Debug)]
pub enum CertificateStatus {
    Valid { expires_at: chrono::DateTime<chrono::Utc> },
    ExpiringLoon { expires_at: chrono::DateTime<chrono::Utc>, time_remaining: Duration },
    Expired,
}
```

## Input Validation

### Request Sanitization

```rust
use regex::Regex;
use once_cell::sync::Lazy;

static USERNAME_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^[a-zA-Z0-9._-]{3,64}$").unwrap()
});

static EMAIL_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}$").unwrap()
});

pub struct InputSanitizer;

impl InputSanitizer {
    pub fn sanitize_username(username: &str) -> Result<String> {
        // Normalize whitespace
        let normalized = username.trim().to_lowercase();
        
        // Validate format
        if !USERNAME_REGEX.is_match(&normalized) {
            return Err(ScimError::validation_error(
                "userName",
                "Username must be 3-64 characters and contain only letters, numbers, dots, underscores, and hyphens"
            ));
        }
        
        // Check for reserved usernames
        const RESERVED_USERNAMES: &[&str] = &[
            "admin", "administrator", "root", "system", "service",
            "daemon", "nobody", "null", "void", "test"
        ];
        
        if RESERVED_USERNAMES.contains(&normalized.as_str()) {
            return Err(ScimError::validation_error(
                "userName",
                "Username is reserved and cannot be used"
            ));
        }
        
        Ok(normalized)
    }
    
    pub fn sanitize_email(email: &str) -> Result<String> {
        let normalized = email.trim().to_lowercase();
        
        if !EMAIL_REGEX.is_match(&normalized) {
            return Err(ScimError::validation_error(
                "email",
                "Invalid email address format"
            ));
        }
        
        // Check for disposable email domains
        let domain = normalized.split('@').nth(1).unwrap();
        if is_disposable_email_domain(domain) {
            return Err(ScimError::validation_error(
                "email",
                "Disposable email addresses are not allowed"
            ));
        }
        
        Ok(normalized)
    }
    
    pub fn sanitize_display_name(display_name: &str) -> Result<String> {
        // Remove potentially dangerous characters
        let sanitized = display_name
            .chars()
            .filter(|c| c.is_alphanumeric() || " .-_()".contains(*c))
            .collect::<String>();
        
        // Limit length
        if sanitized.len() > 256 {
            return Err(ScimError::validation_error(
                "displayName",
                "Display name too long (maximum 256 characters)"
            ));
        }
        
        Ok(sanitized.trim().to_string())
    }
}

fn is_disposable_email_domain(domain: &str) -> bool {
    const DISPOSABLE_DOMAINS: &[&str] = &[
        "10minutemail.com", "guerrillamail.com", "mailinator.com",
        "tempmail.org", "throwaway.email", "yopmail.com"
    ];
    
    DISPOSABLE_DOMAINS.contains(&domain)
}
```

### SQL Injection Prevention

```rust
use sqlx::{query, query_as};

// Always use parameterized queries
impl DatabaseProvider {
    pub async fn get_user_by_username(&self, username: &str) -> Result<Option<Resource>> {
        // Good: Parameterized query
        let row = query!(
            "SELECT data FROM scim_resources WHERE data->>'userName' = $1 AND resource_type = 'User'",
            username
        )
        .fetch_optional(&self.pool)
        .await?;
        
        // Never do this:
        // let sql = format!("SELECT data FROM scim_resources WHERE data->>'userName' = '{}'", username);
        
        if let Some(row) = row {
            let resource: Resource = serde_json::from_value(row.data)?;
            Ok(Some(resource))
        } else {
            Ok(None)
        }
    }
    
    pub async fn search_with_filter(&self, filter: &FilterExpression) -> Result<Vec<Resource>> {
        // Convert SCIM filter to safe SQL
        let (sql_where, params) = self.filter_to_sql(filter)?;
        
        let query_str = format!(
            "SELECT data FROM scim_resources WHERE {}",
            sql_where
        );
        
        let mut query_builder = sqlx::QueryBuilder::new(&query_str);
        
        // Add parameters safely
        for param in params {
            query_builder.push_bind(param);
        }
        
        let rows = query_builder.build_query_as::<serde_json::Value>()
            .fetch_all(&self.pool)
            .await?;
        
        rows.into_iter()
            .map(|row| serde_json::from_value(row))
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| ScimError::internal_error(format!("Deserialization error: {}", e)))
    }
}
```

## Data Protection

### Encryption at Rest

```rust
use aes_gcm::{Aes256Gcm, Key, Nonce};
use aes_gcm::aead::{Aead, NewAead};

pub struct FieldEncryption {
    cipher: Aes256Gcm,
    sensitive_fields: HashSet<String>,
}

impl FieldEncryption {
    pub fn new(key: &[u8; 32]) -> Self {
        let key = Key::from_slice(key);
        let cipher = Aes256Gcm::new(key);
        
        let sensitive_fields = vec![
            "password".to_string(),
            "ssn".to_string(),
            
