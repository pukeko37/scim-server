# Authentication Setup

This tutorial shows you how to implement authentication and authorization for your SCIM Server, covering OAuth 2.0, API keys, and custom authentication schemes.

## Overview

SCIM servers typically operate in enterprise environments where security is paramount. The SCIM Server library provides flexible authentication mechanisms that can integrate with existing identity providers and security infrastructure.

### Common Authentication Patterns

1. **OAuth 2.0 Bearer Tokens** - Industry standard for API authentication
2. **API Keys** - Simple shared secrets for service-to-service communication
3. **JWT Tokens** - Self-contained tokens with embedded claims
4. **Basic Authentication** - Username/password for development and testing
5. **Custom Authentication** - Integration with proprietary systems

## Quick Start: Basic Authentication

Let's start with a simple development setup using basic authentication:

```rust
use scim_server::{ScimServer, InMemoryProvider};
use axum::{
    extract::{Request, State},
    http::{StatusCode, HeaderMap},
    middleware::{self, Next},
    response::Response,
    Router,
};
use base64::{Engine as _, engine::general_purpose};

#[derive(Clone)]
struct AppState {
    scim_server: ScimServer,
    admin_credentials: (String, String), // (username, password)
}

async fn basic_auth_middleware(
    State(state): State<AppState>,
    headers: HeaderMap,
    request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    // Get Authorization header
    let auth_header = headers
        .get("Authorization")
        .and_then(|h| h.to_str().ok())
        .ok_or(StatusCode::UNAUTHORIZED)?;

    // Check Basic auth format
    if !auth_header.starts_with("Basic ") {
        return Err(StatusCode::UNAUTHORIZED);
    }

    // Decode credentials
    let encoded = &auth_header[6..];
    let decoded = general_purpose::STANDARD
        .decode(encoded)
        .map_err(|_| StatusCode::UNAUTHORIZED)?;
    
    let credentials = String::from_utf8(decoded)
        .map_err(|_| StatusCode::UNAUTHORIZED)?;
    
    let mut parts = credentials.splitn(2, ':');
    let username = parts.next().ok_or(StatusCode::UNAUTHORIZED)?;
    let password = parts.next().ok_or(StatusCode::UNAUTHORIZED)?;

    // Validate credentials
    if username == state.admin_credentials.0 && password == state.admin_credentials.1 {
        Ok(next.run(request).await)
    } else {
        Err(StatusCode::UNAUTHORIZED)
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let provider = InMemoryProvider::new();
    let scim_server = ScimServer::builder()
        .provider(provider)
        .build();

    let state = AppState {
        scim_server,
        admin_credentials: ("admin".to_string(), "secret123".to_string()),
    };

    let app = Router::new()
        .nest("/scim/v2/:tenant_id", scim_routes())
        .layer(middleware::from_fn_with_state(state.clone(), basic_auth_middleware))
        .with_state(state);

    println!("SCIM server with basic auth running on http://localhost:3000");
    println!("Use credentials: admin:secret123");
    
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await?;
    axum::serve(listener, app).await?;
    Ok(())
}
```

## OAuth 2.0 Bearer Token Authentication

For production deployments, OAuth 2.0 is the recommended approach:

### JWT Token Validation

```rust
use jsonwebtoken::{decode, DecodingKey, Validation, Algorithm};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
struct Claims {
    sub: String,    // Subject (user ID)
    exp: usize,     // Expiration time
    iat: usize,     // Issued at
    iss: String,    // Issuer
    aud: String,    // Audience
    scope: String,  // OAuth scopes
    tenant_id: Option<String>, // Tenant context
}

async fn oauth_middleware(
    State(state): State<AppState>,
    headers: HeaderMap,
    mut request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    // Extract Bearer token
    let auth_header = headers
        .get("Authorization")
        .and_then(|h| h.to_str().ok())
        .ok_or(StatusCode::UNAUTHORIZED)?;

    if !auth_header.starts_with("Bearer ") {
        return Err(StatusCode::UNAUTHORIZED);
    }

    let token = &auth_header[7..];

    // Validate JWT token
    let decoding_key = DecodingKey::from_secret(state.jwt_secret.as_ref());
    let validation = Validation::new(Algorithm::HS256);
    
    let token_data = decode::<Claims>(token, &decoding_key, &validation)
        .map_err(|_| StatusCode::UNAUTHORIZED)?;

    // Check token expiration
    let now = chrono::Utc::now().timestamp() as usize;
    if token_data.claims.exp < now {
        return Err(StatusCode::UNAUTHORIZED);
    }

    // Check required scopes
    let scopes: Vec<&str> = token_data.claims.scope.split(' ').collect();
    if !scopes.contains(&"scim:read") && !scopes.contains(&"scim:write") {
        return Err(StatusCode::FORBIDDEN);
    }

    // Add user context to request
    request.extensions_mut().insert(UserContext {
        user_id: token_data.claims.sub,
        tenant_id: token_data.claims.tenant_id,
        scopes,
    });

    Ok(next.run(request).await)
}

#[derive(Clone)]
struct UserContext {
    user_id: String,
    tenant_id: Option<String>,
    scopes: Vec<String>,
}
```

### Integration with External OAuth Providers

```rust
use reqwest::Client;
use serde_json::Value;

#[derive(Clone)]
struct OAuthConfig {
    introspection_url: String,
    client_id: String,
    client_secret: String,
}

async fn validate_oauth_token(
    config: &OAuthConfig,
    token: &str,
) -> Result<Claims, String> {
    let client = Client::new();
    
    // Call OAuth provider's introspection endpoint
    let response = client
        .post(&config.introspection_url)
        .basic_auth(&config.client_id, Some(&config.client_secret))
        .form(&[("token", token)])
        .send()
        .await
        .map_err(|e| format!("Failed to validate token: {}", e))?;

    let introspection: Value = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse response: {}", e))?;

    // Check if token is active
    if !introspection["active"].as_bool().unwrap_or(false) {
        return Err("Token is not active".to_string());
    }

    // Extract claims
    Ok(Claims {
        sub: introspection["sub"].as_str().unwrap_or("").to_string(),
        exp: introspection["exp"].as_u64().unwrap_or(0) as usize,
        iat: introspection["iat"].as_u64().unwrap_or(0) as usize,
        iss: introspection["iss"].as_str().unwrap_or("").to_string(),
        aud: introspection["aud"].as_str().unwrap_or("").to_string(),
        scope: introspection["scope"].as_str().unwrap_or("").to_string(),
        tenant_id: introspection["tenant_id"].as_str().map(|s| s.to_string()),
    })
}
```

## API Key Authentication

For service-to-service communication, API keys provide a simpler alternative:

```rust
use sha2::{Sha256, Digest};
use std::collections::HashMap;

#[derive(Clone)]
struct ApiKeyStore {
    keys: HashMap<String, ApiKeyInfo>,
}

#[derive(Clone)]
struct ApiKeyInfo {
    name: String,
    tenant_id: String,
    permissions: Vec<String>,
    created_at: chrono::DateTime<chrono::Utc>,
    last_used: Option<chrono::DateTime<chrono::Utc>>,
}

impl ApiKeyStore {
    fn new() -> Self {
        let mut keys = HashMap::new();
        
        // Example API key (in production, store these securely)
        keys.insert(
            "sk_test_1234567890abcdef".to_string(),
            ApiKeyInfo {
                name: "Development Key".to_string(),
                tenant_id: "tenant-1".to_string(),
                permissions: vec!["scim:read".to_string(), "scim:write".to_string()],
                created_at: chrono::Utc::now(),
                last_used: None,
            },
        );
        
        Self { keys }
    }
    
    async fn validate_key(&mut self, api_key: &str) -> Option<&ApiKeyInfo> {
        if let Some(key_info) = self.keys.get_mut(api_key) {
            key_info.last_used = Some(chrono::Utc::now());
            Some(key_info)
        } else {
            None
        }
    }
}

async fn api_key_middleware(
    State(mut state): State<AppState>,
    headers: HeaderMap,
    mut request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    // Extract API key from header
    let api_key = headers
        .get("X-API-Key")
        .or_else(|| headers.get("Authorization").and_then(|h| {
            h.to_str().ok().and_then(|s| {
                if s.starts_with("Bearer ") {
                    Some(&s[7..])
                } else {
                    None
                }
            })
        }))
        .and_then(|h| h.to_str().ok())
        .ok_or(StatusCode::UNAUTHORIZED)?;

    // Validate API key
    let key_info = state.api_keys.validate_key(api_key).await
        .ok_or(StatusCode::UNAUTHORIZED)?;

    // Add context to request
    request.extensions_mut().insert(ApiKeyContext {
        tenant_id: key_info.tenant_id.clone(),
        permissions: key_info.permissions.clone(),
        key_name: key_info.name.clone(),
    });

    Ok(next.run(request).await)
}

#[derive(Clone)]
struct ApiKeyContext {
    tenant_id: String,
    permissions: Vec<String>,
    key_name: String,
}
```

## Multi-Tenant Authentication

Handle different authentication schemes per tenant:

```rust
#[derive(Clone)]
enum AuthScheme {
    OAuth {
        jwks_url: String,
        audience: String,
        issuer: String,
    },
    ApiKey {
        keys: HashMap<String, String>, // key -> permissions
    },
    Basic {
        username: String,
        password_hash: String,
    },
}

#[derive(Clone)]
struct TenantAuthConfig {
    tenant_configs: HashMap<String, AuthScheme>,
}

impl TenantAuthConfig {
    async fn authenticate(
        &self,
        tenant_id: &str,
        headers: &HeaderMap,
    ) -> Result<AuthContext, StatusCode> {
        let auth_scheme = self.tenant_configs
            .get(tenant_id)
            .ok_or(StatusCode::NOT_FOUND)?;

        match auth_scheme {
            AuthScheme::OAuth { jwks_url, audience, issuer } => {
                self.validate_oauth(headers, jwks_url, audience, issuer).await
            },
            AuthScheme::ApiKey { keys } => {
                self.validate_api_key(headers, keys).await
            },
            AuthScheme::Basic { username, password_hash } => {
                self.validate_basic(headers, username, password_hash).await
            },
        }
    }
    
    async fn validate_oauth(
        &self,
        headers: &HeaderMap,
        jwks_url: &str,
        audience: &str,
        issuer: &str,
    ) -> Result<AuthContext, StatusCode> {
        // OAuth validation logic
        todo!("Implement OAuth validation")
    }
    
    async fn validate_api_key(
        &self,
        headers: &HeaderMap,
        keys: &HashMap<String, String>,
    ) -> Result<AuthContext, StatusCode> {
        // API key validation logic
        todo!("Implement API key validation")
    }
    
    async fn validate_basic(
        &self,
        headers: &HeaderMap,
        username: &str,
        password_hash: &str,
    ) -> Result<AuthContext, StatusCode> {
        // Basic auth validation logic
        todo!("Implement basic auth validation")
    }
}

#[derive(Clone)]
struct AuthContext {
    tenant_id: String,
    user_id: Option<String>,
    permissions: Vec<String>,
    auth_type: String,
}
```

## Authorization and Permissions

Implement fine-grained access control:

```rust
#[derive(Clone)]
struct PermissionChecker {
    // Define permission patterns
}

impl PermissionChecker {
    fn can_access_resource(
        &self,
        context: &AuthContext,
        resource_type: &str,
        operation: &str,
        resource_id: Option<&str>,
    ) -> bool {
        // Check if user has required permissions
        let required_permission = format!("scim:{}:{}", resource_type, operation);
        
        if context.permissions.contains(&required_permission) {
            return true;
        }
        
        // Check wildcard permissions
        let wildcard_permission = format!("scim:{}:*", resource_type);
        if context.permissions.contains(&wildcard_permission) {
            return true;
        }
        
        // Check admin permission
        if context.permissions.contains(&"scim:admin".to_string()) {
            return true;
        }
        
        // Resource-specific checks
        if let Some(id) = resource_id {
            let specific_permission = format!("scim:{}:{}:{}", resource_type, operation, id);
            if context.permissions.contains(&specific_permission) {
                return true;
            }
        }
        
        false
    }
}

// Usage in handlers
async fn get_user_handler(
    State(state): State<AppState>,
    Extension(auth_context): Extension<AuthContext>,
    Path((tenant_id, user_id)): Path<(String, String)>,
) -> Result<Json<ScimUser>, StatusCode> {
    // Check permissions
    if !state.permissions.can_access_resource(
        &auth_context,
        "users",
        "read",
        Some(&user_id),
    ) {
        return Err(StatusCode::FORBIDDEN);
    }
    
    // Proceed with operation
    let user = state.scim_server
        .get_user(&tenant_id, &user_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;
    
    Ok(Json(user))
}
```

## Production Security Considerations

### Rate Limiting

```rust
use tower_governor::{GovernorLayer, governor::GovernorConfig};
use std::time::Duration;

// Add rate limiting middleware
let governor_conf = GovernorConfig::default()
    .per_second(10)
    .burst_size(20)
    .period(Duration::from_secs(60));

let app = Router::new()
    .nest("/scim/v2", scim_routes())
    .layer(GovernorLayer::new(&governor_conf))
    .layer(middleware::from_fn(auth_middleware));
```

### Request Logging and Audit

```rust
async fn audit_middleware(
    Extension(auth_context): Extension<AuthContext>,
    request: Request,
    next: Next,
) -> Response {
    let method = request.method().clone();
    let uri = request.uri().clone();
    let start_time = std::time::Instant::now();
    
    let response = next.run(request).await;
    
    let duration = start_time.elapsed();
    let status = response.status();
    
    // Log the request
    tracing::info!(
        user_id = auth_context.user_id,
        tenant_id = auth_context.tenant_id,
        method = %method,
        uri = %uri,
        status = %status,
        duration_ms = duration.as_millis(),
        "SCIM API request"
    );
    
    response
}
```

### HTTPS and Security Headers

```rust
use tower_http::{
    set_header::SetResponseHeaderLayer,
    cors::CorsLayer,
};

let app = Router::new()
    .nest("/scim/v2", scim_routes())
    .layer(SetResponseHeaderLayer::overriding(
        http::header::STRICT_TRANSPORT_SECURITY,
        http::HeaderValue::from_static("max-age=31536000; includeSubDomains"),
    ))
    .layer(SetResponseHeaderLayer::overriding(
        http::header::X_CONTENT_TYPE_OPTIONS,
        http::HeaderValue::from_static("nosniff"),
    ))
    .layer(SetResponseHeaderLayer::overriding(
        http::header::X_FRAME_OPTIONS,
        http::HeaderValue::from_static("DENY"),
    ))
    .layer(CorsLayer::permissive()) // Configure CORS appropriately
    .layer(middleware::from_fn(auth_middleware));
```

## Testing Authentication

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use axum_test::TestServer;
    
    #[tokio::test]
    async fn test_basic_auth_success() {
        let app = create_test_app().await;
        let server = TestServer::new(app).unwrap();
        
        let response = server
            .get("/scim/v2/tenant-1/Users")
            .add_header("Authorization", "Basic YWRtaW46c2VjcmV0MTIz") // admin:secret123
            .await;
        
        assert_eq!(response.status_code(), 200);
    }
    
    #[tokio::test]
    async fn test_basic_auth_failure() {
        let app = create_test_app().await;
        let server = TestServer::new(app).unwrap();
        
        let response = server
            .get("/scim/v2/tenant-1/Users")
            .add_header("Authorization", "Basic aW52YWxpZA==") // invalid
            .await;
        
        assert_eq!(response.status_code(), 401);
    }
    
    #[tokio::test]
    async fn test_api_key_auth() {
        let app = create_test_app().await;
        let server = TestServer::new(app).unwrap();
        
        let response = server
            .get("/scim/v2/tenant-1/Users")
            .add_header("X-API-Key", "sk_test_1234567890abcdef")
            .await;
        
        assert_eq!(response.status_code(), 200);
    }
}
```

## Configuration

Create a configuration system for different environments:

```rust
#[derive(serde::Deserialize)]
struct AuthConfig {
    #[serde(default)]
    basic_auth: Option<BasicAuthConfig>,
    #[serde(default)]
    oauth: Option<OAuthConfig>,
    #[serde(default)]
    api_keys: Option<ApiKeyConfig>,
}

#[derive(serde::Deserialize)]
struct BasicAuthConfig {
    username: String,
    password: String, // In production, use password hash
}

#[derive(serde::Deserialize)]
struct OAuthConfig {
    jwks_url: String,
    audience: String,
    issuer: String,
}

#[derive(serde::Deserialize)]
struct ApiKeyConfig {
    keys_file: String, // Path to API keys file
}

// Load from environment or config file
fn load_auth_config() -> AuthConfig {
    let config_str = std::fs::read_to_string("auth_config.toml")
        .expect("Failed to read auth config");
    
    toml::from_str(&config_str)
        .expect("Failed to parse auth config")
}
```

This comprehensive authentication setup provides enterprise-grade security for your SCIM Server while maintaining flexibility for different deployment scenarios.

## Next Steps

- [Implement custom validation](../advanced/custom-validation.md) for additional security
- [Set up monitoring](../advanced/monitoring.md) for security events
- [Configure production deployment](../advanced/production-deployment.md) with proper security