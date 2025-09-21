# Authentication & Authorization Strategies

This deep dive explores authentication and authorization patterns in SCIM Server, covering different authentication strategies, role-based access control (RBAC), compile-time vs runtime security patterns, and integration with existing identity systems.

## Overview

Authentication and authorization in SCIM Server involves multiple layers working together to provide secure access control. This document shows how to implement robust security patterns that scale from simple API key authentication to complex enterprise identity integration.

**Core Security Flow:**
```text
Client Request → Authentication → Authorization → Tenant Resolution → 
Resource Access Control → Operation Execution → Audit Logging
```

## Authentication Architecture Overview

```text
┌─────────────────────────────────────────────────────────────────────────────┐
│ Authentication Layer                                                        │
│                                                                             │
│ ┌─────────────────┐ ┌─────────────────┐ ┌─────────────────────────────────┐ │
│ │ API Key Auth    │ │ JWT/OAuth2      │ │ Custom Authentication           │ │
│ │                 │ │                 │ │                                 │ │
│ │ • Simple setup  │ │ • Standards     │ │ • Legacy system integration     │ │
│ │ • High perf     │ │   compliant     │ │ • Custom protocols              │ │
│ │ • Tenant scoped │ │ • Token based   │ │ • Advanced security             │ │
│ └─────────────────┘ └─────────────────┘ └─────────────────────────────────┘ │
└─────────────────────────────────────────────────────────────────────────────┘
                                    ↓
┌─────────────────────────────────────────────────────────────────────────────┐
│ Authorization Layer                                                         │
│                                                                             │
│ ┌─────────────────┐ ┌─────────────────┐ ┌─────────────────────────────────┐ │
│ │ Permission-Based│ │ Role-Based      │ │ Attribute-Based (ABAC)          │ │
│ │ (Simple)        │ │ (RBAC)          │ │ (Advanced)                      │ │
│ │                 │ │                 │ │                                 │ │
│ │ • Resource ops  │ │ • Role          │ │ • Context-aware decisions       │ │
│ │ • CRUD perms    │ │   hierarchies   │ │ • Policy engine integration     │ │
│ │ • Tenant limits │ │ • Dynamic perms │ │ • Fine-grained control          │ │
│ └─────────────────┘ └─────────────────┘ └─────────────────────────────────┘ │
└─────────────────────────────────────────────────────────────────────────────┘
                                    ↓
┌─────────────────────────────────────────────────────────────────────────────┐
│ Access Control Enforcement                                                  │
│ • Resource-level filtering • Operation validation • Audit logging          │
└─────────────────────────────────────────────────────────────────────────────┘
```

## Authentication Strategies

### Strategy 1: API Key Authentication

Best for machine-to-machine integrations and simple setups:

```rust
use scim_server::auth::{AuthenticationProvider, AuthenticationResult, Credential};
use std::collections::HashMap;
use tokio::sync::RwLock;

pub struct ApiKeyAuthProvider {
    api_keys: RwLock<HashMap<String, ApiKeyInfo>>,
    key_validator: KeyValidator,
}

#[derive(Clone)]
pub struct ApiKeyInfo {
    pub key_id: String,
    pub tenant_id: String,
    pub client_id: String,
    pub permissions: Vec<String>,
    pub expires_at: Option<chrono::DateTime<chrono::Utc>>,
    pub rate_limit: Option<RateLimit>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub last_used_at: Option<chrono::DateTime<chrono::Utc>>,
}

#[derive(Clone)]
pub struct RateLimit {
    pub requests_per_minute: u32,
    pub burst_capacity: u32,
}

impl ApiKeyAuthProvider {
    pub fn new(key_validator: KeyValidator) -> Self {
        Self {
            api_keys: RwLock::new(HashMap::new()),
            key_validator,
        }
    }
    
    pub async fn create_api_key(
        &self,
        tenant_id: String,
        client_id: String,
        permissions: Vec<String>,
        expires_at: Option<chrono::DateTime<chrono::Utc>>,
    ) -> Result<String, ApiKeyError> {
        let api_key = self.key_validator.generate_secure_key()?;
        let key_info = ApiKeyInfo {
            key_id: uuid::Uuid::new_v4().to_string(),
            tenant_id,
            client_id,
            permissions,
            expires_at,
            rate_limit: Some(RateLimit {
                requests_per_minute: 1000,
                burst_capacity: 100,
            }),
            created_at: chrono::Utc::now(),
            last_used_at: None,
        };
        
        self.api_keys.write().await.insert(api_key.clone(), key_info);
        Ok(api_key)
    }
    
    pub async fn revoke_api_key(&self, api_key: &str) -> Result<bool, ApiKeyError> {
        Ok(self.api_keys.write().await.remove(api_key).is_some())
    }
    
    pub async fn rotate_api_key(&self, old_key: &str) -> Result<String, ApiKeyError> {
        let mut keys = self.api_keys.write().await;
        if let Some(mut key_info) = keys.remove(old_key) {
            let new_key = self.key_validator.generate_secure_key()?;
            key_info.created_at = chrono::Utc::now();
            key_info.last_used_at = None;
            keys.insert(new_key.clone(), key_info);
            Ok(new_key)
        } else {
            Err(ApiKeyError::KeyNotFound)
        }
    }
}

impl AuthenticationProvider for ApiKeyAuthProvider {
    type Error = ApiKeyAuthError;
    
    async fn authenticate(&self, credential: &Credential) -> Result<AuthenticationResult, Self::Error> {
        let api_key = match credential {
            Credential::ApiKey(key) => key,
            Credential::BearerToken(token) => {
                // Support Bearer token format: "Bearer api_key_here"
                token.strip_prefix("Bearer ").unwrap_or(token)
            },
            _ => return Err(ApiKeyAuthError::UnsupportedCredentialType),
        };
        
        let mut keys = self.api_keys.write().await;
        let key_info = keys.get_mut(api_key)
            .ok_or(ApiKeyAuthError::InvalidApiKey)?;
        
        // Check expiration
        if let Some(expires_at) = key_info.expires_at {
            if chrono::Utc::now() > expires_at {
                keys.remove(api_key);
                return Err(ApiKeyAuthError::ApiKeyExpired);
            }
        }
        
        // Update last used timestamp
        key_info.last_used_at = Some(chrono::Utc::now());
        
        // Build authentication result
        Ok(AuthenticationResult {
            authenticated: true,
            tenant_id: Some(key_info.tenant_id.clone()),
            client_id: key_info.client_id.clone(),
            subject: key_info.key_id.clone(),
            permissions: key_info.permissions.clone(),
            metadata: HashMap::from([
                ("auth_method".to_string(), "api_key".to_string()),
                ("key_id".to_string(), key_info.key_id.clone()),
            ]),
        })
    }
    
    async fn validate_permissions(
        &self,
        auth_result: &AuthenticationResult,
        required_permission: &str,
    ) -> Result<bool, Self::Error> {
        Ok(auth_result.permissions.contains(&required_permission.to_string()) ||
           auth_result.permissions.contains(&"*".to_string()))
    }
}

pub struct KeyValidator {
    key_length: usize,
    key_prefix: Option<String>,
}

impl KeyValidator {
    pub fn new(key_length: usize, key_prefix: Option<String>) -> Self {
        Self { key_length, key_prefix }
    }
    
    pub fn generate_secure_key(&self) -> Result<String, ApiKeyError> {
        use rand::Rng;
        const CHARSET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";
        
        let mut rng = rand::thread_rng();
        let key: String = (0..self.key_length)
            .map(|_| {
                let idx = rng.gen_range(0..CHARSET.len());
                CHARSET[idx] as char
            })
            .collect();
            
        match &self.key_prefix {
            Some(prefix) => Ok(format!("{}_{}", prefix, key)),
            None => Ok(key),
        }
    }
}

// Usage example
async fn setup_api_key_auth() -> Result<ApiKeyAuthProvider, Box<dyn std::error::Error>> {
    let key_validator = KeyValidator::new(32, Some("sk".to_string()));
    let auth_provider = ApiKeyAuthProvider::new(key_validator);
    
    // Create API key for tenant
    let api_key = auth_provider.create_api_key(
        "tenant-123".to_string(),
        "client-app".to_string(),
        vec!["scim:read".to_string(), "scim:write".to_string()],
        Some(chrono::Utc::now() + chrono::Duration::days(90)), // 90 day expiry
    ).await?;
    
    println!("Generated API key: {}", api_key);
    Ok(auth_provider)
}
```

### Strategy 2: JWT/OAuth2 Authentication

Standards-compliant token-based authentication:

```rust
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation, Algorithm};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct ScimClaims {
    pub sub: String,                    // Subject (user/client ID)
    pub iss: String,                    // Issuer
    pub aud: String,                    // Audience
    pub exp: usize,                     // Expiration time
    pub iat: usize,                     // Issued at
    pub jti: String,                    // JWT ID
    pub tenant_id: Option<String>,      // Tenant identifier
    pub client_id: String,              // Client application
    pub scope: String,                  // OAuth2 scopes
    pub permissions: Vec<String>,       // SCIM-specific permissions
}

pub struct JwtAuthProvider {
    decoding_key: DecodingKey,
    encoding_key: Option<EncodingKey>,
    validation: Validation,
    issuer: String,
    audience: String,
}

impl JwtAuthProvider {
    pub fn new(
        secret: &str,
        issuer: String,
        audience: String,
        algorithm: Algorithm,
    ) -> Self {
        let mut validation = Validation::new(algorithm);
        validation.set_issuer(&[issuer.clone()]);
        validation.set_audience(&[audience.clone()]);
        validation.validate_exp = true;
        validation.validate_nbf = true;
        
        Self {
            decoding_key: DecodingKey::from_secret(secret.as_ref()),
            encoding_key: Some(EncodingKey::from_secret(secret.as_ref())),
            validation,
            issuer,
            audience,
        }
    }
    
    pub fn from_public_key(
        public_key_pem: &str,
        issuer: String,
        audience: String,
        algorithm: Algorithm,
    ) -> Result<Self, JwtAuthError> {
        let mut validation = Validation::new(algorithm);
        validation.set_issuer(&[issuer.clone()]);
        validation.set_audience(&[audience.clone()]);
        
        Ok(Self {
            decoding_key: DecodingKey::from_rsa_pem(public_key_pem.as_bytes())?,
            encoding_key: None,
            validation,
            issuer,
            audience,
        })
    }
    
    pub fn create_token(&self, claims: ScimClaims) -> Result<String, JwtAuthError> {
        let encoding_key = self.encoding_key.as_ref()
            .ok_or(JwtAuthError::TokenCreationNotSupported)?;
            
        encode(&Header::default(), &claims, encoding_key)
            .map_err(JwtAuthError::from)
    }
    
    fn parse_scopes_to_permissions(scope: &str) -> Vec<String> {
        scope.split_whitespace()
            .filter_map(|s| {
                match s {
                    "scim:read" => Some("scim:read".to_string()),
                    "scim:write" => Some("scim:write".to_string()),
                    "scim:admin" => Some("*".to_string()),
                    _ if s.starts_with("scim:") => Some(s.to_string()),
                    _ => None,
                }
            })
            .collect()
    }
}

impl AuthenticationProvider for JwtAuthProvider {
    type Error = JwtAuthError;
    
    async fn authenticate(&self, credential: &Credential) -> Result<AuthenticationResult, Self::Error> {
        let token = match credential {
            Credential::BearerToken(token) => {
                token.strip_prefix("Bearer ").unwrap_or(token)
            },
            Credential::JwtToken(token) => token,
            _ => return Err(JwtAuthError::UnsupportedCredentialType),
        };
        
        let token_data = decode::<ScimClaims>(token, &self.decoding_key, &self.validation)?;
        let claims = token_data.claims;
        
        // Convert OAuth2 scopes to SCIM permissions
        let mut permissions = Self::parse_scopes_to_permissions(&claims.scope);
        permissions.extend(claims.permissions);
        
        Ok(AuthenticationResult {
            authenticated: true,
            tenant_id: claims.tenant_id,
            client_id: claims.client_id,
            subject: claims.sub,
            permissions,
            metadata: HashMap::from([
                ("auth_method".to_string(), "jwt".to_string()),
                ("jti".to_string(), claims.jti),
                ("iss".to_string(), claims.iss),
                ("scope".to_string(), claims.scope),
            ]),
        })
    }
    
    async fn validate_permissions(
        &self,
        auth_result: &AuthenticationResult,
        required_permission: &str,
    ) -> Result<bool, Self::Error> {
        // Check explicit permission or wildcard
        Ok(auth_result.permissions.contains(&required_permission.to_string()) ||
           auth_result.permissions.contains(&"*".to_string()))
    }
}

// OAuth2 integration example
pub struct OAuth2Integration {
    jwt_provider: JwtAuthProvider,
    oauth_client: oauth2::basic::BasicClient,
}

impl OAuth2Integration {
    pub async fn exchange_authorization_code(
        &self,
        code: String,
        redirect_uri: String,
    ) -> Result<String, OAuth2Error> {
        use oauth2::{AuthorizationCode, RedirectUrl, TokenResponse};
        
        let token_result = self.oauth_client
            .exchange_code(AuthorizationCode::new(code))
            .set_redirect_uri(RedirectUrl::new(redirect_uri)?)
            .request_async(oauth2::reqwest::async_http_client)
            .await?;
            
        let access_token = token_result.access_token().secret();
        
        // Convert OAuth2 access token to SCIM JWT
        let claims = ScimClaims {
            sub: "user-from-oauth".to_string(), // Extract from OAuth2 user info
            iss: self.jwt_provider.issuer.clone(),
            aud: self.jwt_provider.audience.clone(),
            exp: (chrono::Utc::now() + chrono::Duration::hours(1)).timestamp() as usize,
            iat: chrono::Utc::now().timestamp() as usize,
            jti: uuid::Uuid::new_v4().to_string(),
            tenant_id: Some("extracted-from-oauth".to_string()),
            client_id: "oauth-client".to_string(),
            scope: token_result.scopes()
                .map(|scopes| scopes.iter().map(|s| s.to_string()).collect::<Vec<_>>().join(" "))
                .unwrap_or_default(),
            permissions: vec!["scim:read".to_string(), "scim:write".to_string()],
        };
        
        self.jwt_provider.create_token(claims)
    }
}
```

### Strategy 3: Custom Enterprise Authentication

Integration with existing enterprise identity systems:

```rust
use ldap3::{LdapConn, Scope, SearchEntry};
use async_trait::async_trait;

pub struct LdapAuthProvider {
    ldap_url: String,
    base_dn: String,
    bind_dn: String,
    bind_password: String,
    user_search_filter: String,
    group_search_filter: String,
    connection_pool: deadpool::managed::Pool<LdapConnectionManager>,
}

pub struct LdapConnectionManager {
    ldap_url: String,
    bind_dn: String,
    bind_password: String,
}

impl LdapAuthProvider {
    pub async fn new(
        ldap_url: String,
        base_dn: String,
        bind_dn: String,
        bind_password: String,
        pool_size: usize,
    ) -> Result<Self, LdapAuthError> {
        let manager = LdapConnectionManager {
            ldap_url: ldap_url.clone(),
            bind_dn: bind_dn.clone(),
            bind_password: bind_password.clone(),
        };
        
        let pool_config = deadpool::managed::PoolConfig::new(pool_size);
        let connection_pool = deadpool::managed::Pool::builder(manager)
            .config(pool_config)
            .build()?;
            
        Ok(Self {
            ldap_url,
            base_dn,
            bind_dn,
            bind_password,
            user_search_filter: "(uid={})".to_string(),
            group_search_filter: "(member={})".to_string(),
            connection_pool,
        })
    }
    
    async fn authenticate_user(
        &self,
        username: &str,
        password: &str,
    ) -> Result<LdapUserInfo, LdapAuthError> {
        let conn = self.connection_pool.get().await?;
        
        // Search for user
        let user_filter = self.user_search_filter.replace("{}", username);
        let (rs, _res) = conn.search(
            &self.base_dn,
            Scope::Subtree,
            &user_filter,
            vec!["dn", "uid", "mail", "displayName", "memberOf"]
        ).await?;
        
        let user_entry = rs.into_iter()
            .next()
            .ok_or(LdapAuthError::UserNotFound)?;
            
        let user_entry = SearchEntry::construct(user_entry);
        let user_dn = user_entry.dn.clone();
        
        // Authenticate with user credentials
        let mut user_conn = LdapConn::new(&self.ldap_url)?;
        user_conn.simple_bind(&user_dn, password).await?;
        
        // Extract user information
        let user_info = LdapUserInfo {
            dn: user_dn,
            username: user_entry.attrs.get("uid")
                .and_then(|v| v.first())
                .unwrap_or(&username)
                .clone(),
            email: user_entry.attrs.get("mail")
                .and_then(|v| v.first())
                .cloned(),
            display_name: user_entry.attrs.get("displayName")
                .and_then(|v| v.first())
                .cloned(),
            groups: user_entry.attrs.get("memberOf")
                .map(|groups| groups.clone())
                .unwrap_or_default(),
        };
        
        Ok(user_info)
    }
    
    async fn map_groups_to_permissions(&self, groups: &[String]) -> Vec<String> {
        let mut permissions = Vec::new();
        
        for group in groups {
            match group.as_str() {
                g if g.contains("scim-admins") => permissions.push("*".to_string()),
                g if g.contains("scim-users") => {
                    permissions.push("scim:read".to_string());
                    permissions.push("scim:write".to_string());
                },
                g if g.contains("scim-readonly") => permissions.push("scim:read".to_string()),
                _ => {} // No SCIM permissions for other groups
            }
        }
        
        if permissions.is_empty() {
            permissions.push("scim:read".to_string()); // Default permission
        }
        
        permissions
    }
}

#[async_trait]
impl AuthenticationProvider for LdapAuthProvider {
    type Error = LdapAuthError;
    
    async fn authenticate(&self, credential: &Credential) -> Result<AuthenticationResult, Self::Error> {
        let (username, password) = match credential {
            Credential::UsernamePassword { username, password } => (username, password),
            Credential::BasicAuth(encoded) => {
                let decoded = base64::decode(encoded)?;
                let auth_str = String::from_utf8(decoded)?;
                let parts: Vec<&str> = auth_str.splitn(2, ':').collect();
                if parts.len() != 2 {
                    return Err(LdapAuthError::InvalidBasicAuth);
                }
                (parts[0], parts[1])
            },
            _ => return Err(LdapAuthError::UnsupportedCredentialType),
        };
        
        let user_info = self.authenticate_user(username, password).await?;
        let permissions = self.map_groups_to_permissions(&user_info.groups).await;
        
        // Extract tenant from user's organizational unit or group
        let tenant_id = user_info.groups.iter()
            .find(|g| g.contains("ou="))
            .and_then(|g| g.split("ou=").nth(1))
            .and_then(|ou| ou.split(",").next())
            .map(|s| s.to_string());
        
        Ok(AuthenticationResult {
            authenticated: true,
            tenant_id,
            client_id: "ldap-auth".to_string(),
            subject: user_info.username.clone(),
            permissions,
            metadata: HashMap::from([
                ("auth_method".to_string(), "ldap".to_string()),
                ("user_dn".to_string(), user_info.dn),
                ("email".to_string(), user_info.email.unwrap_or_default()),
                ("display_name".to_string(), user_info.display_name.unwrap_or_default()),
            ]),
        })
    }
    
    async fn validate_permissions(
        &self,
        auth_result: &AuthenticationResult,
        required_permission: &str,
    ) -> Result<bool, Self::Error> {
        Ok(auth_result.permissions.contains(&required_permission.to_string()) ||
           auth_result.permissions.contains(&"*".to_string()))
    }
}

#[derive(Debug, Clone)]
struct LdapUserInfo {
    dn: String,
    username: String,
    email: Option<String>,
    display_name: Option<String>,
    groups: Vec<String>,
}
```

## Authorization Patterns

### Role-Based Access Control (RBAC)

```rust
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Role {
    pub name: String,
    pub description: String,
    pub permissions: HashSet<Permission>,
    pub parent_roles: Vec<String>,
    pub resource_constraints: Vec<ResourceConstraint>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Permission {
    // SCIM resource permissions
    CreateUser,
    ReadUser,
    UpdateUser,
    DeleteUser,
    ListUsers,
    CreateGroup,
    ReadGroup,
    UpdateGroup,
    DeleteGroup,
    ListGroups,
    
    // Administrative permissions
    ManageSchema,
    ManageTenants,
    ViewMetrics,
    ManageApiKeys,
    
    // Wildcard permissions
    All,
    AllUsers,
    AllGroups,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceConstraint {
    pub resource_type: String,
    pub constraint_type: ConstraintType,
    pub constraint_value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConstraintType {
    AttributeEquals,
    AttributeContains,
    AttributeStartsWith,
    TenantEquals,
    GroupMember,
}

pub struct RbacAuthProvider {
    roles: RwLock<HashMap<String, Role>>,
    user_roles: RwLock<HashMap<String, Vec<String>>>,
    role_hierarchy: RwLock<HashMap<String, HashSet<String>>>,
}

impl RbacAuthProvider {
    pub fn new() -> Self {
        let mut provider = Self {
            roles: RwLock::new(HashMap::new()),
            user_roles: RwLock::new(HashMap::new()),
            role_hierarchy: RwLock::new(HashMap::new()),
        };
        
        // Initialize default roles
        provider.setup_default_roles();
        provider
    }
    
    fn setup_default_roles(&mut self) {
        let roles = vec![
            Role {
                name: "scim_admin".to_string(),
                description: "Full SCIM administration access".to_string(),
                permissions: [Permission::All].into_iter().collect(),
                parent_roles: vec![],
                resource_constraints: vec![],
            },
            Role {
                name: "user_manager".to_string(),
                description: "User resource management".to_string(),
                permissions: [
                    Permission::CreateUser, Permission::ReadUser, 
                    Permission::UpdateUser, Permission::DeleteUser, Permission::ListUsers
                ].into_iter().collect(),
                parent_roles: vec![],
                resource_constraints: vec![],
            },
            Role {
                name: "group_manager".to_string(),
                description: "Group resource management".to_string(),
                permissions: [
                    Permission::CreateGroup, Permission::ReadGroup,
                    Permission::UpdateGroup, Permission::DeleteGroup, Permission::ListGroups,
                    Permission::ReadUser, Permission::ListUsers // Needed for group membership
                ].into_iter().collect(),
                parent_roles: vec![],
                resource_constraints: vec![],
            },
            Role {
                name: "readonly".to_string(),
                description: "Read-only access to all resources".to_string(),
                permissions: [
                    Permission::ReadUser, Permission::ListUsers,
                    Permission::ReadGroup, Permission::ListGroups
                ].into_iter().collect(),
                parent_roles: vec![],
                resource_constraints: vec![],
            },
            Role {
                name: "tenant_admin".to_string(),
                description: "Administrative access within tenant".to_string(),
                permissions: [Permission::AllUsers, Permission::AllGroups].into_iter().collect(),
                parent_roles: vec![],
                resource_constraints: vec![
                    ResourceConstraint {
                        resource_type: "*".to_string(),
                        constraint_type: ConstraintType::TenantEquals,
                        constraint_value: "${user.tenant_id}".to_string(),
                    }
                ],
            },
        ];
        
        let mut role_map = self.roles.get_mut().unwrap();
        for role in roles {
            role_map.insert(role.name.clone(), role);
        }
    }
    
    pub async fn assign_role_to_user(&self, user_id: &str, role_name: &str) -> Result<(), RbacError> {
        // Verify role exists
        {
            let roles = self.roles.read().await;
            if !roles.contains_key(role_name) {
                return Err(RbacError::RoleNotFound(role_name.to_string()));
            }
        }
        
        let mut user_roles = self.user_roles.write().await;
        user_roles.entry(user_id.to_string())
            .or_insert_with(Vec::new)
            .push(role_name.to_string());
            
        Ok(())
    }
    
    pub async fn get_effective_permissions(
        &self,
        user_id: &str,
        context: &RequestContext,
    ) -> Result<HashSet<Permission>, RbacError> {
        let user_roles_guard = self.user_roles.read().await;
        let user_roles = user_roles_guard.get(user_id)
            .ok_or(RbacError::UserNotFound(user_id.to_string()))?;
            
        let roles_guard = self.roles.read().await;
        let mut effective_permissions = HashSet::new();
        
        for role_name in user_roles {
            if let Some(role) = roles_guard.get(role_name) {
                // Check if role's resource constraints are satisfied
                if self.check_resource_constraints(&role.resource_constraints, context).await? {
                    effective_permissions.extend(role.permissions.iter().cloned());
                    
                    // Include permissions from parent roles
                    effective_permissions.extend(
                        self.get_inherited_permissions(&role.parent_roles, context, &roles_guard).await?
                    );
                }
            }
        }
        
        Ok(effective_permissions)
    }
    
    async fn check_resource_constraints(
        &self,
        constraints: &[ResourceConstraint],
        context: &RequestContext,
    ) -> Result<bool, RbacError> {
        for constraint in constraints {
            match &constraint.constraint_type {
                ConstraintType::TenantEquals => {
                    let expected_tenant = constraint.constraint_value
                        .replace("${user.tenant_id}", context.tenant_id().unwrap_or(""));
                    if context.tenant_id() != Some(&expected_tenant) {
                        return Ok(false);
                    }
                },
                ConstraintType::AttributeEquals => {
                    // Implementation depends on where user attributes are stored
                    // This is a placeholder for more complex attribute-based constraints
                },
                _ => {} // Other constraint types
            }
        }
        
        Ok(true)
    }
    
    async fn get_inherited_permissions(
        &self,
        parent_roles: &[String],
        context: &RequestContext,
        roles: &HashMap<String, Role>,
    ) -> Result<HashSet<Permission>, RbacError> {
        let mut inherited = HashSet::new();
        
        for parent_role_name in parent_roles {
            if let Some(parent_role) = roles.get(parent_role_name) {
                if self.check_resource_constraints(&parent_role.resource_constraints, context).await? {
                    inherited.extend(parent_role.permissions.iter().cloned());
                    // Recursive inheritance
                    inherited.extend(
                        self.get_inherited_permissions(&parent_role.parent_roles, context, roles).await?
                    );
                }
            }
        }
        
        Ok(inherited)
    }
}

impl AuthenticationProvider for RbacAuthProvider {
    type Error = RbacAuthError;
    
    async fn authenticate(&self, credential: &Credential) -> Result<AuthenticationResult, Self::Error> {
        // RBAC provider typically works as a wrapper around another auth provider
        // This is a simplified example
        match credential {
            Credential::UserId(user_id) => {
                // For demo purposes - in practice this would delegate to another provider
                Ok(AuthenticationResult {
                    authenticated: true,
                    tenant_id: None,
                    client_id: "rbac-system".to_string(),
                    subject: user_id.clone(),
                    permissions: vec![], // Will be populated by authorization check
                    metadata: HashMap::from([
                        ("auth_method".to_string(), "rbac".to_string()),
                    ]),
                })
            },
            _ => Err(RbacAuthError::UnsupportedCredentialType),
        }
    }
    
    async fn authorize_operation(
        &self,
        auth_result: &AuthenticationResult,
        operation: &str,
        resource_type: &str,
        context: &RequestContext,
    ) -> Result<bool, Self::Error> {
        let effective_permissions = self.get_effective_permissions(&auth_result.subject, context).await?;
        
        let required_permission = match (operation, resource_type) {
            ("create", "User") => Permission::CreateUser,
            ("read", "User") => Permission::ReadUser,
            ("update", "User") => Permission::UpdateUser,
            ("delete", "User") => Permission::DeleteUser,
            ("list", "User") => Permission::ListUsers,
            ("create", "Group") => Permission::CreateGroup,
            ("read", "Group") => Permission::ReadGroup,
            ("update", "Group") => Permission::UpdateGroup,
            ("delete", "Group") => Permission::DeleteGroup,
            ("list", "Group") => Permission::ListGroups,
            _ => return Ok(false),
        };
        
        Ok(effective_permissions.contains(&required_permission) ||
           effective_permissions.contains(&Permission::All) ||
           (resource_type == "User" && effective_permissions.contains(&Permission::AllUsers)) ||
           (resource_type == "Group" && effective_permissions.contains(&Permission::AllGroups)))
    }
}
```

### Attribute-Based Access Control (ABAC)

```rust
use serde_json::{Value, json};

pub struct AbacPolicyEngine {
    policies: RwLock<Vec<AbacPolicy>>,
    attribute_provider: Arc<dyn AttributeProvider>,
    policy_evaluator: PolicyEvaluator,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AbacPolicy {
    pub id: String,
    pub name: String,
    pub description: String,
    pub target: PolicyTarget,
    pub condition: PolicyCondition,
    pub effect: PolicyEffect,
    pub priority: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyTarget {
    pub subjects: Vec<String>,
    pub resources: Vec<String>,
    pub actions: Vec<String>,
    pub environment: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PolicyCondition {
    Always,
    Never,
    AttributeMatch { attribute: String, operator: String, value: Value },
    And(Vec<PolicyCondition>),
    Or(Vec<PolicyCondition>),
    Not(Box<PolicyCondition>),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PolicyEffect {
    Permit,
    Deny,
}

pub trait AttributeProvider: Send + Sync {
    async fn get_subject_attributes(&self, subject_id: &str) -> Result<HashMap<String, Value>, AttributeError>;
    async fn get_resource_attributes(&self, resource_type: &str, resource_id: &str) -> Result<HashMap<String, Value>, AttributeError>;
    async fn get_environment_attributes(&self, context: &RequestContext) -> Result<HashMap<String, Value>, AttributeError>;
}

impl AbacPolicyEngine {
    pub fn new(attribute_provider: Arc<dyn AttributeProvider>) -> Self {
        Self {
            policies: RwLock::new(Vec::new()),
            attribute_provider,
            policy_evaluator: PolicyEvaluator::new(),
        }
    }
    
    pub async fn add_policy(&self, policy: AbacPolicy) {
        let mut policies = self.policies.write().await;
        policies.push(policy);
        policies.sort_by(|a, b| b.priority.cmp(&a.priority)); // Higher priority first
    }
    
    pub async fn evaluate_authorization(
        &self,
        subject_id: &str,
        resource_type: &str,
        resource_id: Option<&str>,
        action: &str,
        context: &RequestContext,
    ) -> Result<bool, AbacError> {
        // Gather attributes
        let subject_attrs = self.attribute_provider.get_subject_attributes(subject_id).await?;
        let resource_attrs = if let Some(rid) = resource_id {
            self.attribute_provider.get_resource_attributes(resource_type, rid).await?
        } else {
            HashMap::new()
        };
        let env_attrs = self.attribute_provider.get_environment_attributes(context).await?;
        
        let evaluation_context = EvaluationContext {
            subject_id: subject_id.to_string(),
            resource_type: resource_type.to_string(),
            resource_id: resource_id.map(|s| s.to_string()),
            action: action.to_string(),
            subject_attributes: subject_attrs,
            resource_attributes: resource_attrs,
            environment_attributes: env_attrs,
        };
        
        let policies = self.policies.read().await;
        
        // Evaluate policies in priority order
        for policy in policies.iter() {
            if self.policy_matches_target(policy, &evaluation_context)? {
                match self.policy_evaluator.evaluate_condition(&policy.condition, &evaluation_context).await? {
                    true => {
                        return Ok(matches!(policy.effect, PolicyEffect::Permit));
                    },
                    false => continue,
                }
            }
        }
        
        // Default deny
        Ok(false)
    }
    
    fn policy_matches_target(&self, policy: &AbacPolicy, context: &EvaluationContext) -> Result<bool, AbacError> {
        // Check if policy target matches the request context
        let target_matches = 
            (policy.target.subjects.is_empty() || 
             policy.target.subjects.contains(&context.subject_id) || 
             policy.target.subjects.contains(&"*".to_string())) &&
            (policy.target.resources.is_empty() || 
             policy.target.resources.contains(&context.resource_type) || 
             policy.target.resources.contains(&"*".to_string())) &&
            (policy.target.actions.is_empty() || 
             policy.target.actions.contains(&context.action) || 
             policy.target.actions.contains(&"*".to_string()));
             
        Ok(target_matches)
    }
}

#[derive(Debug)]
struct EvaluationContext {
    subject_id: String,
    resource_type: String,
    resource_id: Option<String>,
    action: String,
    subject_attributes: HashMap<String, Value>,
    resource_attributes: HashMap<String, Value>,
    environment_attributes: HashMap<String, Value>,
}

struct PolicyEvaluator;

impl PolicyEvaluator {
    fn new() -> Self {
        Self
    }
    
    async fn evaluate_condition(
        &self,
        condition: &PolicyCondition,
        context: &EvaluationContext,
    ) -> Result<bool, AbacError> {
        match condition {
            PolicyCondition::Always => Ok(true),
            PolicyCondition::Never => Ok(false),
            PolicyCondition::AttributeMatch { attribute, operator, value } => {
                self.evaluate_attribute_match(attribute, operator, value, context).await
            },
            PolicyCondition::And(conditions) => {
                for cond in conditions {
                    if !self.evaluate_condition(cond, context).await? {
                        return Ok(false);
                    }
                }
                Ok(true)
            },
            PolicyCondition::Or(conditions) => {
                for cond in conditions {
                    if self.evaluate_condition(cond, context).await? {
                        return Ok(true);
                    }
                }
                Ok(false)
            },
            PolicyCondition::Not(condition) => {
                Ok(!self.evaluate_condition(condition, context).await?)
            },
        }
    }
    
    async fn evaluate_attribute_match(
        &self,
        attribute: &str,
        operator: &str,
        expected_value: &Value,
        context: &EvaluationContext,
    ) -> Result<bool, AbacError> {
        let actual_value = self.get_attribute_value(attribute, context)?;
        
        match operator {
            "equals" => Ok(actual_value == *expected_value),
            "not_equals" => Ok(actual_value != *expected_value),
            "contains" => {
                if let (Value::String(actual), Value::String(expected)) = (&actual_value, expected_value) {
                    Ok(actual.contains(expected))
                } else {
                    Ok(false)
                }
            },
            "starts_with" => {
                if let (Value::String(actual), Value::String(expected)) = (&actual_value, expected_value) {
                    Ok(actual.starts_with(expected))
                } else {
                    Ok(false)
                }
            },
            "greater_than" => {
                if let (Value::Number(actual), Value::Number(expected)) = (&actual_value, expected_value) {
                    Ok(actual.as_f64().unwrap_or(0.0) > expected.as_f64().unwrap_or(0.0))
                } else {
                    Ok(false)
                }
            },
            "in" => {
                if let Value::Array(expected_array) = expected_value {
                    Ok(expected_array.contains(&actual_value))
                } else {
                    Ok(false)
                }
            },
            _ => Err(AbacError::UnsupportedOperator(operator.to_string())),
        }
    }
    
    fn get_attribute_value(&self, attribute: &str, context: &EvaluationContext) -> Result<Value, AbacError> {
        let parts: Vec<&str> = attribute.split('.').collect();
        match parts.get(0) {
            Some(&"subject") => {
                context.subject_attributes.get(parts.get(1).unwrap_or(&""))
                    .cloned()
                    .ok_or_else(|| AbacError::AttributeNotFound(attribute.to_string()))
            },
            Some(&"resource") => {
                context.resource_attributes.get(parts.get(1).unwrap_or(&""))
                    .cloned()
                    .ok_or_else(|| AbacError::AttributeNotFound(attribute.to_string()))
            },
            Some(&"environment") => {
                context.environment_attributes.get(parts.get(1).unwrap_or(&""))
                    .cloned()
                    .ok_or_else(|| AbacError::AttributeNotFound(attribute.to_string()))
            },
            _ => Err(AbacError::InvalidAttributePath(attribute.to_string())),
        }
    }
}

// Example ABAC policy setup
async fn setup_abac_policies() -> Result<AbacPolicyEngine, Box<dyn std::error::Error>> {
    let attribute_provider = Arc::new(DatabaseAttributeProvider::new(/* db connection */));
    let policy_engine = AbacPolicyEngine::new(attribute_provider);
    
    // Policy 1: Managers can manage users in their department
    policy_engine.add_policy(AbacPolicy {
        id: "manager-dept-access".to_string(),
        name: "Manager Department Access".to_string(),
        description: "Managers can manage users in their department".to_string(),
        target: PolicyTarget {
            subjects: vec!["*".to_string()],
            resources: vec!["User".to_string()],
            actions: vec!["create".to_string(), "update".to_string(), "delete".to_string()],
            environment: vec![],
        },
        condition: PolicyCondition::And(vec![
            PolicyCondition::AttributeMatch {
                attribute: "subject.role".to_string(),
                operator: "equals".to_string(),
                value: json!("manager"),
            },
            PolicyCondition::AttributeMatch {
                attribute: "subject.department".to_string(),
                operator: "equals".to_string(),
                value: json!("${resource.department}"),
            },
        ]),
        effect: PolicyEffect::Permit,
        priority: 100,
    }).await;
    
    // Policy 2: Business hours restriction
    policy_engine.add_policy(AbacPolicy {
        id: "business-hours-only".to_string(),
        name: "Business Hours Only".to_string(),
        description: "Some operations only allowed during business hours".to_string(),
        target: PolicyTarget {
            subjects: vec!["*".to_string()],
            resources: vec!["*".to_string()],
            actions: vec!["delete".to_string()],
            environment: vec![],
        },
        condition: PolicyCondition::And(vec![
            PolicyCondition::AttributeMatch {
                attribute: "environment.time_of_day".to_string(),
                operator: "greater_than".to_string(),
                value: json!(9), // 9 AM
            },
            PolicyCondition::AttributeMatch {
                attribute: "environment.time_of_day".to_string(),
                operator: "less_than".to_string(),
                value: json!(17), // 5 PM
            },
            PolicyCondition::AttributeMatch {
                attribute: "environment.day_of_week".to_string(),
                operator: "in".to_string(),
                value: json!(["Monday", "Tuesday", "Wednesday", "Thursday", "Friday"]),
            },
        ]),
        effect: PolicyEffect::Permit,
        priority: 50,
    }).await;
    
    Ok(policy_engine)
}
```

## Compile-Time vs Runtime Security

### Compile-Time Security Patterns

```rust
use std::marker::PhantomData;

// Type-safe permission system
pub struct Authenticated;
pub struct Unauthenticated;

pub struct Authorized<P>(PhantomData<P>);
pub struct Unauthorized;

// Permission types
pub struct ReadPermission;
pub struct WritePermission;
pub struct AdminPermission;

// Context with type-level authentication state
pub struct TypedRequestContext<A, Z> {
    inner: RequestContext,
    _auth_state: PhantomData<A>,
    _authz_state: PhantomData<Z>,
}

impl TypedRequestContext<Unauthenticated, Unauthorized> {
    pub fn new(request_id: String) -> Self {
        Self {
            inner: RequestContext::new(request_id),
            _auth_state: PhantomData,
            _authz_state: PhantomData,
        }
    }
    
    pub fn authenticate<P: AuthenticationProvider>(
        self,
        provider: &P,
        credential: &Credential,
    ) -> impl Future<Output = Result<TypedRequestContext<Authenticated, Unauthorized>, P::Error>> {
        async move {
            provider.authenticate(credential).await?;
            Ok(TypedRequestContext {
                inner: self.inner,
                _auth_state: PhantomData,
                _authz_state: PhantomData,
            })
        }
    }
}

impl TypedRequestContext<Authenticated, Unauthorized> {
    pub fn authorize<P>(
        self,
        _permission_proof: P,
    ) -> TypedRequestContext<Authenticated, Authorized<P>> {
        TypedRequestContext {
            inner: self.inner,
            _auth_state: PhantomData,
            _authz_state: PhantomData,
        }
    }
}

// Only authorized contexts can perform operations
impl<P> TypedRequestContext<Authenticated, Authorized<P>> {
    pub fn into_inner(self) -> RequestContext {
        self.inner
    }
}

// Permission proofs - only created when authorization succeeds
pub struct PermissionProof<P> {
    _permission: PhantomData<P>,
}

impl<P> PermissionProof<P> {
    // Private constructor - only created by authorization system
    fn new() -> Self {
        Self { _permission: PhantomData }
    }
}

// Type-safe SCIM server operations
pub struct TypeSafeScimServer<R: ResourceProvider> {
    inner: ScimServer<R>,
}

impl<R: ResourceProvider> TypeSafeScimServer<R> {
    pub fn new(provider: R) -> Result<Self, ScimServerError> {
        Ok(Self {
            inner: ScimServer::new(provider)?,
        })
    }
    
    // Only accept authorized contexts
    pub async fn create_resource<P>(
        &self,
        resource_type: &str,
        data: Value,
        context: TypedRequestContext<Authenticated, Authorized<WritePermission>>,
    ) -> Result<Resource, ScimError> {
        self.inner.create_resource(resource_type, data, &context.into_inner()).await
    }
    
    pub async fn get_resource<P>(
        &self,
        resource_type: &str,
        id: &str,
        context: TypedRequestContext<Authenticated, Authorized<ReadPermission>>,
    ) -> Result<Option<Resource>, ScimError> {
        self.inner.get_resource(resource_type, id, &context.into_inner()).await
    }
    
    pub async fn delete_resource<P>(
        &self,
        resource_type: &str,
        id: &str,
        context: TypedRequestContext<Authenticated, Authorized<AdminPermission>>,
    ) -> Result<bool, ScimError> {
        self.inner.delete_resource(resource_type, id, &context.into_inner()).await
    }
}

// Authorization service that produces permission proofs
pub struct TypeSafeAuthorizationService<A: AuthenticationProvider> {
    auth_provider: A,
    rbac_engine: RbacAuthProvider,
}

impl<A: AuthenticationProvider> TypeSafeAuthorizationService<A> {
    pub async fn check_read_permission(
        &self,
        auth_result: &AuthenticationResult,
        resource_type: &str,
        context: &RequestContext,
    ) -> Result<PermissionProof<ReadPermission>, AuthorizationError> {
        let required_permission = format!("read_{}", resource_type.to_lowercase());
        
        if self.auth_provider.validate_permissions(auth_result, &required_permission).await? {
            Ok(PermissionProof::new())
        } else {
            Err(AuthorizationError::InsufficientPermissions)
        }
    }
    
    pub async fn check_write_permission(
        &self,
        auth_result: &AuthenticationResult,
        resource_type: &str,
        context: &RequestContext,
    ) -> Result<PermissionProof<WritePermission>, AuthorizationError> {
        let required_permission = format!("write_{}", resource_type.to_lowercase());
        
        if self.auth_provider.validate_permissions(auth_result, &required_permission).await? {
            Ok(PermissionProof::new())
        } else {
            Err(AuthorizationError::InsufficientPermissions)
        }
    }
    
    pub async fn check_admin_permission(
        &self,
        auth_result: &AuthenticationResult,
        context: &RequestContext,
    ) -> Result<PermissionProof<AdminPermission>, AuthorizationError> {
        if self.auth_provider.validate_permissions(auth_result, "*").await? {
            Ok(PermissionProof::new())
        } else {
            Err(AuthorizationError::InsufficientPermissions)
        }
    }
}

// Usage example
async fn compile_time_safe_example() -> Result<(), Box<dyn std::error::Error>> {
    let provider = StandardResourceProvider::new(InMemoryStorage::new());
    let server = TypeSafeScimServer::new(provider)?;
    let auth_service = TypeSafeAuthorizationService::new(/* auth provider */);
    
    // Create unauthenticated context
    let context = TypedRequestContext::new("req-123".to_string());
    
    // Authenticate - compile error if credential is invalid
    let auth_context = context.authenticate(&auth_service.auth_provider, &credential).await?;
    
    // Try to authorize for read permission
    let read_proof = auth_service.check_read_permission(&auth_result, "User", &context).await?;
    let authorized_context = auth_context.authorize(read_proof);
    
    // This compiles - we have read permission
    let user = server.get_resource("User", "123", authorized_context).await?;
    
    // This would be a compile error - we don't have write permission
    // server.create_resource("User", user_data, authorized_context).await?; // ❌ Compile error
    
    Ok(())
}
```

## Integration Patterns

### Middleware Integration

```rust
use axum::{extract::Request, middleware::Next, response::Response};

pub async fn auth_middleware(
    mut request: Request,
    next: Next,
) -> Result<Response, AuthMiddlewareError> {
    // Extract authentication credential
    let credential = extract_credential_from_request(&request)?;
    
    // Get authentication provider from request extensions
    let auth_provider = request.extensions()
        .get::<Arc<dyn AuthenticationProvider>>()
        .ok_or(AuthMiddlewareError::MissingAuthProvider)?;
    
    // Authenticate
    let auth_result = auth_provider.authenticate(&credential).await
        .map_err(AuthMiddlewareError::AuthenticationFailed)?;
    
    if !auth_result.authenticated {
        return Err(AuthMiddlewareError::Unauthenticated);
    }
    
    // Add authentication result to request extensions
    request.extensions_mut().insert(auth_result);
    
    Ok(next.run(request).await)
}

pub async fn authz_middleware(
    mut request: Request,
    next: Next,
) -> Result<Response, AuthzMiddlewareError> {
    let auth_result = request.extensions()
        .get::<AuthenticationResult>()
        .ok_or(AuthzMiddlewareError::MissingAuthResult)?;
    
    // Extract operation details from request
    let (operation, resource_type) = extract_operation_details(&request)?;
    
    // Get authorization provider
    let authz_provider = request.extensions()
        .get::<Arc<dyn AuthorizationProvider>>()
        .ok_or(AuthzMiddlewareError::MissingAuthzProvider)?;
    
    // Check authorization
    let authorized = authz_provider.authorize_operation(
        auth_result,
        &operation,
        &resource_type,
        &RequestContext::from_request(&request),
    ).await
    .map_err(AuthzMiddlewareError::AuthorizationFailed)?;
    
    if !authorized {
        return Err(AuthzMiddlewareError::Forbidden);
    }
    
    Ok(next.run(request).await)
}

fn extract_credential_from_request(request: &Request) -> Result<Credential, CredentialExtractionError> {
    // Try Authorization header first
    if let Some(auth_header) = request.headers().get("authorization") {
        if let Ok(auth_str) = auth_header.to_str() {
            if auth_str.starts_with("Bearer ") {
                return Ok(Credential::BearerToken(auth_str[7..].to_string()));
            } else if auth_str.starts_with("Basic ") {
                return Ok(Credential::BasicAuth(auth_str[6..].to_string()));
            }
        }
    }
    
    // Try API key header
    if let Some(api_key) = request.headers().get("x-api-key") {
        if let Ok(key_str) = api_key.to_str() {
            return Ok(Credential::ApiKey(key_str.to_string()));
        }
    }
    
    Err(CredentialExtractionError::NoCredentialFound)
}
```

## Best Practices Summary

### Security Implementation Guidelines

1. **Layer Security Appropriately**
   - Authentication verifies identity
   - Authorization controls access
   - Audit logging tracks all operations

2. **Choose the Right Strategy**
   - API keys for simple machine-to-machine
   - JWT/OAuth2 for standards compliance
   - Custom auth for legacy integration
   - RBAC for role-based organizations
   - ABAC for fine-grained control

3. **Implement Defense in Depth**
   - Multiple authentication factors
   - Authorization at multiple layers
   - Rate limiting and DDoS protection
   - Input validation and sanitization

4. **Use Compile-Time Safety When Possible**
   - Type-safe permission systems
   - Phantom types for state tracking
   - Zero-cost abstractions

5. **Monitor and Audit**
   - Log all authentication attempts
   - Track authorization decisions
   - Monitor for unusual patterns
   - Implement alerting for security events

## Related Topics

- **[Request Lifecycle & Context Management](./request-lifecycle.md)** - How authentication integrates with request flow
- **[Multi-Tenant Architecture Patterns](./multi-tenant-patterns.md)** - Tenant-scoped authentication and authorization
- **[Compile-Time Auth Example](../examples/compile-time-auth.md)** - Practical compile-time security patterns
- **[Role-Based Access Control Example](../examples/rbac.md)** - RBAC implementation details

## Next Steps

Now that you understand authentication and authorization strategies:

1. **Choose your authentication strategy** based on your integration requirements
2. **Implement appropriate authorization** (permissions, RBAC, or ABAC)
3. **Set up proper middleware integration** for your web framework
4. **Add comprehensive audit logging** for security monitoring
5. **Consider compile-time safety patterns** for critical applications
        
