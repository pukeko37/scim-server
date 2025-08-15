# Security Considerations

This guide provides comprehensive security guidance for deploying and operating SCIM servers in production environments. Security is paramount in identity management systems, and this document covers threats, mitigations, and best practices.

## Security Overview

SCIM servers handle sensitive identity data and must be secured against various attack vectors. This document addresses:

- **Authentication and Authorization**
- **Data Protection**
- **Network Security**
- **Input Validation**
- **Audit and Monitoring**
- **Deployment Security**
- **Compliance Considerations**

## Authentication Security

### Token-Based Authentication

#### JWT Token Validation

**Best Practices**:
```rust
use scim_server::auth::{AuthConfig, JwtConfig};

let jwt_config = JwtConfig::builder()
    .issuer("https://trusted-auth-provider.com")
    .audience("scim-api")
    .public_key_url("https://trusted-auth-provider.com/.well-known/jwks.json")
    .algorithm("RS256") // Use asymmetric algorithms
    .clock_skew_seconds(60) // Allow for clock drift
    .cache_public_keys(true)
    .key_refresh_interval_seconds(3600) // Refresh keys hourly
    .validate_not_before(true)
    .validate_expiration(true)
    .build()?;

let auth_config = AuthConfig::builder()
    .jwt_config(jwt_config)
    .require_scope("scim:write") // Require specific scopes
    .build()?;
```

**Security Measures**:
- Always validate JWT signatures using public keys
- Verify issuer, audience, and expiration claims
- Use short-lived tokens (15-60 minutes)
- Implement token refresh mechanisms
- Cache public keys with regular rotation

#### Bearer Token Security

```rust
use scim_server::auth::{BearerTokenValidator, TokenValidationError};

#[async_trait]
impl BearerTokenValidator for CustomTokenValidator {
    async fn validate_token(&self, token: &str) -> Result<AuthContext, TokenValidationError> {
        // Implement secure token validation
        let validation_result = self.introspect_token(token).await?;
        
        if !validation_result.active {
            return Err(TokenValidationError::TokenInactive);
        }
        
        if validation_result.expires_at < Utc::now() {
            return Err(TokenValidationError::TokenExpired);
        }
        
        // Validate required scopes
        if !validation_result.scopes.contains(&"scim:read".to_string()) {
            return Err(TokenValidationError::InsufficientScope);
        }
        
        Ok(AuthContext {
            user_id: validation_result.user_id,
            tenant_id: validation_result.tenant_id,
            scopes: validation_result.scopes,
            roles: validation_result.roles,
        })
    }
}
```

### OAuth 2.0 Integration

#### Scope-Based Authorization

```rust
use scim_server::auth::{AuthMiddleware, RequiredScope};

// Define fine-grained scopes
const SCIM_READ: &str = "scim:read";
const SCIM_WRITE: &str = "scim:write";
const SCIM_DELETE: &str = "scim:delete";
const SCIM_ADMIN: &str = "scim:admin";

// Apply scope requirements to endpoints
app.route("/Users", get(list_users).with(RequiredScope::new(SCIM_READ)))
   .route("/Users", post(create_user).with(RequiredScope::new(SCIM_WRITE)))
   .route("/Users/:id", delete(delete_user).with(RequiredScope::new(SCIM_DELETE)));
```

#### Token Introspection

```rust
use oauth2::introspection::{IntrospectionRequest, IntrospectionResponse};

async fn introspect_token(token: &str) -> Result<IntrospectionResponse, AuthError> {
    let client = reqwest::Client::new();
    let response = client
        .post("https://auth-server.com/oauth2/introspect")
        .form(&[
            ("token", token),
            ("token_type_hint", "access_token"),
        ])
        .basic_auth(&client_id, Some(&client_secret))
        .timeout(Duration::from_secs(5)) // Short timeout
        .send()
        .await?;
    
    let introspection: IntrospectionResponse = response.json().await?;
    Ok(introspection)
}
```

## Authorization Security

### Role-Based Access Control (RBAC)

```rust
use scim_server::auth::{Role, Permission, AuthContext};

#[derive(Debug, Clone)]
pub enum Permission {
    ReadUsers,
    WriteUsers,
    DeleteUsers,
    ReadGroups,
    WriteGroups,
    DeleteGroups,
    ManageTenants,
    ViewAuditLogs,
}

#[derive(Debug, Clone)]
pub struct Role {
    pub name: String,
    pub permissions: Vec<Permission>,
    pub tenant_scope: Option<String>, // None for global roles
}

// Define roles
let user_reader = Role {
    name: "user_reader".to_string(),
    permissions: vec![Permission::ReadUsers],
    tenant_scope: Some("tenant_123".to_string()),
};

let admin = Role {
    name: "admin".to_string(),
    permissions: vec![
        Permission::ReadUsers,
        Permission::WriteUsers,
        Permission::DeleteUsers,
        Permission::ReadGroups,
        Permission::WriteGroups,
        Permission::DeleteGroups,
    ],
    tenant_scope: None, // Global admin
};

// Authorization middleware
async fn authorize_request(
    auth_context: &AuthContext,
    required_permission: Permission,
    resource_tenant: Option<&str>,
) -> Result<(), AuthError> {
    for role in &auth_context.roles {
        if role.permissions.contains(&required_permission) {
            // Check tenant scope
            match (&role.tenant_scope, resource_tenant) {
                (None, _) => return Ok(()), // Global role
                (Some(role_tenant), Some(resource_tenant)) if role_tenant == resource_tenant => {
                    return Ok(());
                }
                _ => continue,
            }
        }
    }
    Err(AuthError::InsufficientPermissions)
}
```

### Attribute-Level Security

```rust
use scim_server::security::{AttributeFilter, SecurityContext};

#[derive(Debug)]
pub struct AttributeFilter {
    readable_attributes: HashSet<String>,
    writable_attributes: HashSet<String>,
    tenant_id: Option<String>,
    user_roles: Vec<String>,
}

impl AttributeFilter {
    pub fn filter_response(&self, mut user: User) -> User {
        // Remove sensitive attributes based on permissions
        if !self.user_roles.contains(&"admin".to_string()) {
            user.password = None;
            user.security_question = None;
        }
        
        // Remove PII for limited roles
        if !self.user_roles.contains(&"pii_reader".to_string()) {
            user.social_security_number = None;
            user.date_of_birth = None;
        }
        
        user
    }
    
    pub fn validate_write_permissions(&self, patch_ops: &[PatchOp]) -> Result<(), AuthError> {
        for op in patch_ops {
            let attribute = extract_attribute_from_path(&op.path);
            if !self.writable_attributes.contains(attribute) {
                return Err(AuthError::AttributeNotWritable(attribute.to_string()));
            }
        }
        Ok(())
    }
}
```

## Data Protection

### Encryption at Rest

```rust
use scim_server::encryption::{EncryptionProvider, AesGcmProvider};

// Configure encryption for sensitive fields
let encryption_config = EncryptionConfig::builder()
    .provider(AesGcmProvider::new(&encryption_key)?)
    .encrypt_fields(vec![
        "password",
        "socialSecurityNumber",
        "bankAccountNumber",
        "personalEmail",
    ])
    .encryption_algorithm("AES-256-GCM")
    .key_rotation_days(90)
    .build()?;

// Automatic encryption/decryption
#[derive(Serialize, Deserialize)]
pub struct User {
    pub id: String,
    pub user_name: String,
    
    #[serde(with = "encrypted_field")]
    pub password: Option<String>,
    
    #[serde(with = "encrypted_field")]
    pub social_security_number: Option<String>,
    
    pub meta: Meta,
}
```

### Encryption in Transit

```rust
use scim_server::tls::{TlsConfig, TlsVersion, CipherSuite};

let tls_config = TlsConfig::builder()
    .certificate_file("/etc/ssl/certs/server.crt")
    .private_key_file("/etc/ssl/private/server.key")
    .ca_certificate_file("/etc/ssl/certs/ca.crt")
    .min_tls_version(TlsVersion::V1_2)
    .max_tls_version(TlsVersion::V1_3)
    .require_client_cert(false)
    .cipher_suites(vec![
        CipherSuite::TLS_AES_256_GCM_SHA384,
        CipherSuite::TLS_CHACHA20_POLY1305_SHA256,
        CipherSuite::TLS_AES_128_GCM_SHA256,
    ])
    .verify_hostname(true)
    .build()?;
```

### Data Masking and Redaction

```rust
use scim_server::privacy::{DataMasker, MaskingRule};

#[derive(Debug)]
pub struct DataMasker {
    rules: Vec<MaskingRule>,
}

impl DataMasker {
    pub fn mask_user_for_logging(&self, user: &User) -> User {
        let mut masked_user = user.clone();
        
        // Mask email
        if let Some(email) = &masked_user.user_name {
            masked_user.user_name = Some(self.mask_email(email));
        }
        
        // Remove sensitive fields entirely
        masked_user.password = None;
        masked_user.social_security_number = None;
        
        // Mask phone numbers
        for phone in &mut masked_user.phone_numbers {
            phone.value = self.mask_phone(&phone.value);
        }
        
        masked_user
    }
    
    fn mask_email(&self, email: &str) -> String {
        if let Some(at_pos) = email.find('@') {
            let (local, domain) = email.split_at(at_pos);
            if local.len() > 2 {
                format!("{}***@{}", &local[..2], &domain[1..])
            } else {
                format!("***@{}", &domain[1..])
            }
        } else {
            "***".to_string()
        }
    }
}
```

## Input Validation Security

### Schema Validation

```rust
use scim_server::validation::{SchemaValidator, ValidationError};

#[derive(Debug)]
pub struct SecureValidator {
    max_string_length: usize,
    max_array_size: usize,
    allowed_schemas: HashSet<String>,
    dangerous_patterns: Vec<Regex>,
}

impl SecureValidator {
    pub fn validate_user_input(&self, user: &User) -> Result<(), ValidationError> {
        // Check schema allowlist
        for schema in &user.schemas {
            if !self.allowed_schemas.contains(schema) {
                return Err(ValidationError::UnknownSchema(schema.clone()));
            }
        }
        
        // Validate string lengths
        if let Some(username) = &user.user_name {
            if username.len() > self.max_string_length {
                return Err(ValidationError::StringTooLong("userName".to_string()));
            }
            self.check_dangerous_patterns(username, "userName")?;
        }
        
        // Validate array sizes
        if user.emails.len() > self.max_array_size {
            return Err(ValidationError::ArrayTooLarge("emails".to_string()));
        }
        
        // Validate email formats
        for email in &user.emails {
            self.validate_email(&email.value)?;
        }
        
        Ok(())
    }
    
    fn check_dangerous_patterns(&self, input: &str, field: &str) -> Result<(), ValidationError> {
        for pattern in &self.dangerous_patterns {
            if pattern.is_match(input) {
                warn!("Dangerous pattern detected in field {}: {}", field, input);
                return Err(ValidationError::DangerousInput(field.to_string()));
            }
        }
        Ok(())
    }
    
    fn validate_email(&self, email: &str) -> Result<(), ValidationError> {
        // Strict email validation
        let email_regex = Regex::new(r"^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}$").unwrap();
        if !email_regex.is_match(email) {
            return Err(ValidationError::InvalidEmail(email.to_string()));
        }
        
        // Check for suspicious patterns
        if email.contains("..") || email.starts_with('.') || email.ends_with('.') {
            return Err(ValidationError::SuspiciousEmail(email.to_string()));
        }
        
        Ok(())
    }
}
```

### SQL Injection Prevention

```rust
use scim_server::storage::{QueryBuilder, Parameter};

// Always use parameterized queries for safe filtering
pub async fn find_users_by_criteria(
    &self,
    tenant_id: &str,
    department: Option<&str>,
    active: Option<bool>,
) -> Result<Vec<User>, StorageError> {
    let mut query_builder = QueryBuilder::new();
    let mut params = Vec::new();
    
    // Build parameterized query
    query_builder.add("SELECT * FROM users WHERE tenant_id = ?");
    params.push(Parameter::String(tenant_id.to_string()));
    
    // Add safe criteria parameters
    if let Some(dept) = department {
        query_builder.add(" AND department = ?");
        params.push(Parameter::String(dept.to_string()));
    }
    
    if let Some(is_active) = active {
        query_builder.add(" AND active = ?");
        params.push(Parameter::Bool(is_active));
    }
    
    let query = query_builder.build();
    self.execute_query(&query, &params).await
}

// Safe query building with attribute validation
fn build_search_query(&self, criteria: &SearchCriteria) -> Result<(String, Vec<Parameter>), FilterError> {
    let mut query = String::from("SELECT * FROM users WHERE tenant_id = ?");
    let mut params = vec![Parameter::String(criteria.tenant_id.clone())];
    
    // Validate and add safe search criteria
    if let Some(username) = &criteria.username {
        if self.is_valid_attribute("userName") {
            query.push_str(" AND username = ?");
            params.push(Parameter::String(username.clone()));
        } else {
            return Err(FilterError::InvalidAttribute("userName"));
        }
    }
    
    if let Some(email) = &criteria.email {
        if self.is_valid_attribute("emails.value") {
            query.push_str(" AND email = ?");
            params.push(Parameter::String(email.clone()));
        } else {
            return Err(FilterError::InvalidAttribute("emails.value"));
        }
    }
    
    Ok((query, params))
}
}
```

### JSON Injection Prevention

```rust
use scim_server::json::{SafeJsonParser, JsonValidationError};

pub struct SafeJsonParser {
    max_depth: usize,
    max_object_size: usize,
    max_string_length: usize,
    forbidden_keys: HashSet<String>,
}

impl SafeJsonParser {
    pub fn parse_user(&self, json: &str) -> Result<User, JsonValidationError> {
        // Parse with size limits
        let value: serde_json::Value = serde_json::from_str(json)
            .map_err(|e| JsonValidationError::ParseError(e.to_string()))?;
        
        // Validate structure
        self.validate_json_structure(&value, 0)?;
        
        // Convert to User struct with validation
        let user: User = serde_json::from_value(value)
            .map_err(|e| JsonValidationError::DeserializationError(e.to_string()))?;
        
        Ok(user)
    }
    
    fn validate_json_structure(&self, value: &serde_json::Value, depth: usize) -> Result<(), JsonValidationError> {
        if depth > self.max_depth {
            return Err(JsonValidationError::TooDeep);
        }
        
        match value {
            serde_json::Value::Object(obj) => {
                if obj.len() > self.max_object_size {
                    return Err(JsonValidationError::ObjectTooLarge);
                }
                
                for (key, val) in obj {
                    if self.forbidden_keys.contains(key) {
                        return Err(JsonValidationError::ForbiddenKey(key.clone()));
                    }
                    self.validate_json_structure(val, depth + 1)?;
                }
            }
            serde_json::Value::Array(arr) => {
                for item in arr {
                    self.validate_json_structure(item, depth + 1)?;
                }
            }
            serde_json::Value::String(s) => {
                if s.len() > self.max_string_length {
                    return Err(JsonValidationError::StringTooLong);
                }
            }
            _ => {}
        }
        
        Ok(())
    }
}
```

## Network Security

### Rate Limiting

```rust
use scim_server::rate_limit::{RateLimiter, RateLimitConfig, RateLimitError};

#[derive(Debug)]
pub struct SecurityRateLimiter {
    global_limiter: RateLimiter,
    tenant_limiters: DashMap<String, RateLimiter>,
    ip_limiters: DashMap<IpAddr, RateLimiter>,
}

impl SecurityRateLimiter {
    pub async fn check_rate_limits(
        &self,
        ip: IpAddr,
        tenant_id: Option<&str>,
        endpoint: &str,
    ) -> Result<(), RateLimitError> {
        // Check global rate limit (most permissive)
        self.global_limiter.check_rate_limit("global", 10000, 3600).await?;
        
        // Check IP-based rate limit (stricter)
        let ip_key = ip.to_string();
        if let Some(ip_limiter) = self.ip_limiters.get(&ip) {
            ip_limiter.check_rate_limit(&ip_key, 1000, 3600).await?;
        }
        
        // Check tenant-specific rate limit
        if let Some(tenant_id) = tenant_id {
            if let Some(tenant_limiter) = self.tenant_limiters.get(tenant_id) {
                tenant_limiter.check_rate_limit(tenant_id, 5000, 3600).await?;
            }
        }
        
        // Check endpoint-specific limits
        match endpoint {
            "/Users" => self.check_user_endpoint_limits(ip, tenant_id).await?,
            "/Bulk" => self.check_bulk_endpoint_limits(ip, tenant_id).await?,
            _ => {}
        }
        
        Ok(())
    }
    
    async fn check_bulk_endpoint_limits(
        &self,
        ip: IpAddr,
        tenant_id: Option<&str>,
    ) -> Result<(), RateLimitError> {
        // Bulk operations are more expensive, so stricter limits
        let bulk_limiter = self.ip_limiters.entry(ip).or_insert_with(|| {
            RateLimiter::new(RateLimitConfig::new(10, 3600)) // 10 per hour
        });
        
        bulk_limiter.check_rate_limit(&format!("bulk:{}", ip), 10, 3600).await
    }
}
```

### CORS Security

```rust
use scim_server::cors::{CorsConfig, CorsMiddleware};

let cors_config = CorsConfig::builder()
    .allowed_origins(vec![
        "https://app.company.com".to_string(),
        "https://admin.company.com".to_string(),
    ]) // Never use "*" in production
    .allowed_methods(vec!["GET", "POST", "PUT", "PATCH", "DELETE"])
    .allowed_headers(vec![
        "Authorization",
        "Content-Type",
        "X-Tenant-ID",
        "If-Match",
        "If-None-Match",
    ])
    .expose_headers(vec!["ETag", "Location"])
    .allow_credentials(true)
    .max_age(3600)
    .vary_header(true) // Important for caching security
    .build()?;
```

### IP Allowlisting

```rust
use scim_server::network::{IpFilter, IpFilterConfig};

let ip_filter_config = IpFilterConfig::builder()
    .allowed_cidrs(vec![
        "10.0.0.0/8".parse()?,      // Internal network
        "172.16.0.0/12".parse()?,   // Private network
        "203.0.113.0/24".parse()?,  // Specific public range
    ])
    .blocked_cidrs(vec![
        "192.168.1.100/32".parse()?, // Known malicious IP
    ])
    .enable_geoblocking(true)
    .allowed_countries(vec!["US", "CA", "GB"])
    .build()?;

// Apply IP filtering middleware
async fn ip_filter_middleware(
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    req: Request<Body>,
    next: Next<Body>,
) -> Result<Response<Body>, StatusCode> {
    let ip = addr.ip();
    
    if !ip_filter_config.is_allowed(ip).await? {
        warn!("Blocked request from IP: {}", ip);
        return Err(StatusCode::FORBIDDEN);
    }
    
    Ok(next.run(req).await)
}
```

## Audit and Monitoring

### Comprehensive Audit Logging

```rust
use scim_server::audit::{AuditLogger, AuditEvent, AuditLevel};

#[derive(Debug, Serialize)]
pub struct AuditEvent {
    pub timestamp: DateTime<Utc>,
    pub event_type: String,
    pub actor: ActorInfo,
    pub resource: ResourceInfo,
    pub tenant_id: Option<String>,
    pub ip_address: IpAddr,
    pub user_agent: Option<String>,
    pub result: OperationResult,
    pub details: serde_json::Value,
}

impl AuditLogger {
    pub async fn log_user_access(&self, event: UserAccessEvent) {
        let audit_event = AuditEvent {
            timestamp: Utc::now(),
            event_type: "user_access".to_string(),
            actor: ActorInfo {
                user_id: event.actor_id,
                auth_method: event.auth_method,
                scopes: event.scopes,
            },
            resource: ResourceInfo {
                resource_type: "User".to_string(),
                resource_id: event.user_id.clone(),
                operation: event.operation,
            },
            tenant_id: event.tenant_id,
            ip_address: event.ip_address,
            user_agent: event.user_agent,
            result: event.result,
            details: json!({
                "user_id": event.user_id,
                "fields_accessed": event.fields_accessed,
                "response_size": event.response_size,
            }),
        };
        
        // Log to multiple destinations
        self.log_to_file(&audit_event).await;
        self.log_to_siem(&audit_event).await;
        self.log_to_database(&audit_event).await;
        
        // Trigger alerts for sensitive operations
        if event.operation == "delete" || event.fields_accessed.contains(&"password".to_string()) {
            self.trigger_security_alert(&audit_event).await;
        }
    }
    
    pub async fn log_authentication_event(&self, event: AuthEvent) {
        let audit_event = AuditEvent {
            timestamp: Utc::now(),
            event_type: match event.result {
                AuthResult::Success => "auth_success".to_string(),
                AuthResult::Failure => "auth_failure".to_string(),
            },
            actor: ActorInfo {
                user_id: event.user_id.clone(),
                auth_method: event.auth_method,
                scopes: vec![],
            },
            resource: ResourceInfo {
                resource_type: "Authentication".to_string(),
                resource_id: "system".to_string(),
                operation: "authenticate".to_string(),
            },
            tenant_id: event.tenant_id,
            ip_address: event.ip_address,
            user_agent: event.user_agent,
            result: match event.result {
                AuthResult::Success => OperationResult::Success,
                AuthResult::Failure => OperationResult::Failure,
            },
            details: json!({
                "auth_method": event.auth_method,
                "failure_reason": event.failure_reason,
                "token_claims": event.token_claims,
            }),
        };
        
        self.log_to_security_log(&audit_event).await;
        
        // Detect brute force attacks
        if matches!(event.result, AuthResult::Failure) {
            self.check_for_brute_force(event.ip_address, event.user_id).await;
        }
    }
}
```

### Security Monitoring

```rust
use scim_server::monitoring::{SecurityMonitor, ThreatDetector};

pub struct SecurityMonitor {
    threat_detector: ThreatDetector,
    alert_manager: AlertManager,
    metrics_collector: MetricsCollector,
}

impl SecurityMonitor {
    pub async fn analyze_request_patterns(&self) {
        // Detect unusual access patterns
        let patterns = self.threat_detector.analyze_recent_requests().await;
        
        for pattern in patterns {
            match pattern.threat_level {
                ThreatLevel::High => {
                    self.alert_manager.send_immediate_alert(pattern).await;
                    self.auto_block_suspicious_ips(pattern.source_ips).await;
                }
                ThreatLevel::Medium => {
                    self.alert_manager.send_alert(pattern).await;
                    self.increase_monitoring(pattern.source_ips).await;
                }
                ThreatLevel::Low => {
                    self.log_suspicious_activity(pattern).await;
                }
            }
        }
    }
    
    pub async fn detect_data_exfiltration(&self) {
        // Monitor for unusual data access patterns
        let access_patterns = self.metrics_collector
            .get_recent_access_patterns(Duration::hours(1))
            .await;
        
        for pattern in access_patterns {
            // Large number of user records accessed by single user
            if pattern.resources_accessed > 1000 && pattern.time_span < Duration::minutes(10) {
                self.alert_manager.send_data_exfiltration_alert(&pattern).await;
            }
            
            // Access to sensitive fields by non-admin users
            if pattern.sensitive_fields_accessed > 0 && !pattern.is_admin_user {
                self.alert_manager.send_privilege_escalation_alert(&pattern).await;
            }
        }
    }
}
```

## Deployment Security

### Container Security

```dockerfile
# Use minimal base image
FROM gcr.io/distroless/cc-debian12:latest

# Don't run as root
USER 65534:65534

# Copy only necessary files
COPY --from=builder /app/target/release/scim-server /scim-server
COPY --from=builder /app/config/ /config/

# Set secure file permissions
USER root
RUN chmod 500 /scim-server && \
    chmod -R 400 /config/
USER 65534:65534

# Health check
HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 \
    CMD ["/scim-server", "health-check"]

# Expose only necessary port
EXPOSE 8080

ENTRYPOINT ["/scim-server"]
```

### Kubernetes Security

```yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: scim-server
spec:
  template:
    spec:
      securityContext:
        runAsNonRoot: true
        runAsUser: 65534
        runAsGroup: 65534
        fsGroup: 65534
        seccompProfile:
          type: RuntimeDefault
      containers:
      - name: scim-server
        image: scim-server:latest
        securityContext:
          allowPrivilegeEscalation: false
          readOnlyRootFilesystem: true
          runAsNonRoot: true
          runAsUser: 65534
          capabilities:
            drop:
            - ALL
        resources:
          limits:
            memory: "1Gi"
            cpu: "500m"
          requests:
            memory: "512Mi"
            cpu: "250m"
        env:
        - name: SCIM_DB_PASSWORD
          valueFrom:
            secretKeyRef:
              name: scim-secrets
              key: db-password
        - name: SCIM_JWT_SECRET
          valueFrom:
            secretKeyRef:
              name: scim-secrets
              key: jwt-secret
        volumeMounts:
        - name: config
          mountPath: /config
          readOnly: true
        - name: tmp
          mountPath: /tmp
        livenessProbe:
          httpGet:
            path: /health/live
            port: 8080
          initialDelaySeconds: 30
          periodSeconds: 10
        readinessProbe:
          httpGet:
            path: /health/ready
            port: 8080
          initialDelaySeconds: 5
          periodSeconds: 5
      volumes:
      - name: config
        configMap:
          name: scim-config
          defaultMode: 0400
      - name: tmp
        emptyDir: {}
---
apiVersion: v1
kind: NetworkPolicy
metadata:
  name: scim-server-netpol
spec:
  podSelector:
    matchLabels:
      app: scim-server
  policyTypes:
  - Ingress
  - Egress
  ingress:
  - from:
    - namespaceSelector:
        matchLabels:
          name: ingress-system
    ports:
    - protocol: TCP
      port: 8080
  egress:
  - to:
    - namespaceSelector:
        matchLabels:
          name: database
    ports:
    - protocol: TCP
      port: 5432
  - to: []
    ports:
    - protocol: TCP
      port: 443  # HTTPS only
```

### Environment Hardening

```rust
use scim_server::config::SecurityConfig;

fn load_secure_config() -> Result<SecurityConfig, ConfigError> {
    let config = SecurityConfig::builder()
        // Disable debug features in production
        .debug_mode(false)
        .detailed_errors(false)
        
        // Enable all security features
        .require_https(true)
        .strict_transport_security(true)
        .content_security_policy(true)
        .x_frame_options("DENY")
        .x_content_type_options(true)
        .referrer_policy("strict-origin-when-cross-origin")
        
        // Security headers
        .hsts_max_age(31536000) // 1 year
        .hsts_include_subdomains(true)
        .hsts_preload(true)
        
        // Rate limiting
        .enable_rate_limiting(true)
        .default_rate_limit(1000) // per hour
        .burst_rate_limit(100)
        
        // Input validation
        .max_request_size(1048576) // 1MB
        .max_json_depth(10)
        .enable_strict_validation(true)
        
        // Audit logging
        .enable_audit_logging(true)
        .audit_log_level("INFO")
        .audit_destinations(vec!["file", "syslog", "webhook"])
        
        .build()
}
```

## Compliance and Standards

### GDPR Compliance

```rust
use scim_server::privacy::{GdprCompliance, DataSubjectRequest, LegalBasis};

#[derive(Debug)]
pub struct GdprCompliance {
    data_controller: String,
    data_processor: Option<String>,
    legal_basis: LegalBasis,
    retention_policy: RetentionPolicy,
}

impl GdprCompliance {
    pub async fn handle_data_subject_request(
        &self,
        request: DataSubjectRequest,
    ) -> Result<DataSubjectResponse, GdprError> {
        match request.request_type {
            RequestType::Access => self.handle_access_request(request).await,
            RequestType::Rectification => self.handle_rectification_request(request).await,
            RequestType::Erasure => self.handle_erasure_request(request).await,
            RequestType::Portability => self.handle_portability_request(request).await,
            RequestType::Restriction => self.handle_restriction_request(request).await,
        }
    }
    
    async fn handle_erasure_request(
        &self,
        request: DataSubjectRequest,
    ) -> Result<DataSubjectResponse, GdprError> {
        // Verify identity
        self.verify_data_subject_identity(&request).await?;
        
        // Check for legal obligations that prevent erasure
        if self.has_legal_obligation_to_retain(&request.subject_id).await? {
            return Err(GdprError::ErasureNotPermitted(
                "Data retention required by law".to_string()
            ));
        }
        
        // Perform cascading deletion
        self.delete_user_data(&request.subject_id).await?;
        self.delete_audit_logs(&request.subject_id).await?;
        self.notify_third_parties(&request.subject_id).await?;
        
        // Log the erasure
        self.audit_logger.log_erasure(&request).await;
        
        Ok(DataSubjectResponse {
            request_id: request.id,
            status: "completed".to_string(),
            completion_date: Utc::now(),
        })
    }
}
```

### SOC 2 Compliance

```rust
use scim_server::compliance::{Soc2Controls, ControlObjective};

pub struct Soc2Controls {
    security_controls: Vec<SecurityControl>,
    availability_controls: Vec<AvailabilityControl>,
    confidentiality_controls: Vec<ConfidentialityControl>,
}

impl Soc2Controls {
    pub fn implement_cc6_1_logical_access(&self) -> Result<(), ComplianceError> {
        // CC6.1: Logical and physical access controls
        
        // Multi-factor authentication
        self.enforce_mfa_for_admin_access()?;
        
        // Principle of least privilege
        self.implement_rbac_controls()?;
        
        // Access reviews
        self.schedule_quarterly_access_reviews()?;
        
        // Segregation of duties
        self.enforce_segregation_of_duties()?;
        
        Ok(())
    }
    
    pub fn implement_cc7_1_system_monitoring(&self) -> Result<(), ComplianceError> {
        // CC7.1: System monitoring
        
        // Comprehensive logging
        self.enable_comprehensive_audit_logging()?;
        
        // Real-time monitoring
        self.implement_real_time_alerting()?;
        
        // Log integrity
        self.implement_log_integrity_controls()?;
        
        // Incident response
        self.implement_incident_response_procedures()?;
        
        Ok(())
    }
}
```

## Incident Response

### Security Incident Detection

```rust
use scim_server::security::{IncidentDetector, SecurityIncident, IncidentSeverity};

pub struct IncidentDetector {
    anomaly_detector: AnomalyDetector,
    threat_intelligence: ThreatIntelligence,
    correlation_engine: CorrelationEngine,
}

impl IncidentDetector {
    pub async fn analyze_security_events(&self) -> Vec<SecurityIncident> {
        let mut incidents = Vec::new();
        
        // Detect authentication anomalies
        let auth_anomalies = self.detect_authentication_anomalies().await;
        for anomaly in auth_anomalies {
            incidents.push(SecurityIncident {
                id: Uuid::new_v4(),
                incident_type: IncidentType::AuthenticationAnomaly,
                severity: self.calculate_severity(&anomaly),
                description: anomaly.description,
                affected_resources: anomaly.affected_resources,
                indicators: anomaly.indicators,
                timestamp: Utc::now(),
            });
        }
        
        // Detect data access anomalies
        let data_anomalies = self.detect_data_access_anomalies().await;
        for anomaly in data_anomalies {
            incidents.push(SecurityIncident {
                id: Uuid::new_v4(),
                incident_type: IncidentType::DataAccessAnomaly,
                severity: IncidentSeverity::High,
                description: format!("Unusual data access pattern: {}", anomaly.pattern),
                affected_resources: anomaly.resources,
                indicators: anomaly.indicators,
                timestamp: Utc::now(),
            });
        }
        
        incidents
    }
    
    async fn detect_authentication_anomalies(&self) -> Vec<AuthenticationAnomaly> {
        let mut anomalies = Vec::new();
        
        // Detect brute force attacks
        let failed_logins = self.get_recent_failed_logins(Duration::hours(1)).await;
        let grouped_by_ip = self.group_by_ip(failed_logins);
        
        for (ip, attempts) in grouped_by_ip {
            if attempts.len() > 50 {
                anomalies.push(AuthenticationAnomaly {
                    anomaly_type: AnomalyType::BruteForce,
                    source_ip: ip,
                    description: format!("Brute force attack detected: {} failed attempts", attempts.len()),
                    affected_resources: attempts.into_iter().map(|a| a.target_user).collect(),
                    indicators: vec![
                        format!("source_ip: {}", ip),
                        format!("attempt_count: {}", attempts.len()),
                    ],
                });
            }
        }
        
        // Detect impossible travel
        let successful_logins = self.get_recent_successful_logins(Duration::hours(24)).await;
        let travel_anomalies = self.detect_impossible_travel(successful_logins).await;
        anomalies.extend(travel_anomalies);
        
        anomalies
    }
}
```

### Automated Response

```rust
use scim_server::security::{AutomatedResponse, ResponseAction};

pub struct AutomatedResponse {
    action_executor: ActionExecutor,
    notification_service: NotificationService,
    escalation_rules: Vec<EscalationRule>,
}

impl AutomatedResponse {
    pub async fn respond_to_incident(&self, incident: &SecurityIncident) {
        match incident.severity {
            IncidentSeverity::Critical => {
                self.execute_critical_response(incident).await;
            }
            IncidentSeverity::High => {
                self.execute_high_severity_response(incident).await;
            }
            IncidentSeverity::Medium => {
                self.execute_medium_severity_response(incident).await;
            }
            IncidentSeverity::Low => {
                self.execute_low_severity_response(incident).await;
            }
        }
    }
    
    async fn execute_critical_response(&self, incident: &SecurityIncident) {
        // Immediate blocking
        if let Some(source_ip) = self.extract_source_ip(incident) {
            self.action_executor.block_ip_address(source_ip).await;
        }
        
        // Disable compromised accounts
        for resource in &incident.affected_resources {
            if resource.resource_type == "User" {
                self.action_executor.disable_user_account(&resource.id).await;
            }
        }
        
        // Immediate notifications
        self.notification_service.send_critical_alert(incident).await;
        self.notification_service.notify_security_team(incident).await;
        self.notification_service.notify_management(incident).await;
        
        // Initiate incident response process
        self.initiate_incident_response_process(incident).await;
    }
    
    async fn initiate_incident_response_process(&self, incident: &SecurityIncident) {
        // Create incident ticket
        let ticket = self.create_incident_ticket(incident).await;
        
        // Preserve evidence
        self.preserve_digital_evidence(incident).await;
        
        // Notify external parties if required
        if self.requires_external_notification(incident) {
            self.notify_authorities(incident).await;
            self.notify_customers(incident).await;
        }
        
        // Start forensic analysis
        self.start_forensic_analysis(incident).await;
    }
}
```

## Security Testing

### Penetration Testing Integration

```rust
use scim_server::testing::{PenetrationTest, VulnerabilityScanner};

pub struct SecurityTestSuite {
    vulnerability_scanner: VulnerabilityScanner,
    penetration_tester: PenetrationTest,
    compliance_checker: ComplianceChecker,
}

impl SecurityTestSuite {
    pub async fn run_security_tests(&self) -> SecurityTestReport {
        let mut report = SecurityTestReport::new();
        
        // Vulnerability scanning
        let vulnerabilities = self.vulnerability_scanner.scan().await;
        report.add_vulnerabilities(vulnerabilities);
        
        // Authentication testing
        let auth_tests = self.test_authentication_security().await;
        report.add_test_results("authentication", auth_tests);
        
        // Authorization testing
        let authz_tests = self.test_authorization_security().await;
        report.add_test_results("authorization", authz_tests);
        
        // Input validation testing
        let input_tests = self.test_input_validation().await;
        report.add_test_results("input_validation", input_tests);
        
        // Network security testing
        let network_tests = self.test_network_security().await;
        report.add_test_results("network_security", network_tests);
        
        report
    }
    
    async fn test_authentication_security(&self) -> Vec<TestResult> {
        vec![
            self.test_jwt_validation().await,
            self.test_token_expiration().await,
            self.test_brute_force_protection().await,
            self.test_session_management().await,
        ]
    }
    
    async fn test_jwt_validation(&self) -> TestResult {
        let test_cases = vec![
            ("Invalid signature", "invalid_jwt_token"),
            ("Expired token", self.create_expired_jwt()),
            ("Wrong audience", self.create_wrong_audience_jwt()),
            ("Missing required claims", self.create_incomplete_jwt()),
        ];
        
        for (test_name, token) in test_cases {
            let result = self.make_authenticated_request(token).await;
            if result.status() != 401 {
                return TestResult::Failed(format!("JWT validation failed for: {}", test_name));
            }
        }
        
        TestResult::Passed("JWT validation working correctly".to_string())
    }
}
```

## Best Practices Summary

### Authentication Best Practices

1. **Use Strong Authentication Methods**
   - Implement JWT with RS256 or ES256 algorithms
   - Require multi-factor authentication for admin access
   - Use short-lived tokens (15-60 minutes)
   - Implement proper token refresh mechanisms

2. **Secure Token Handling**
   - Never store tokens in local storage
   - Use secure, httpOnly cookies when possible
   - Implement proper token revocation
   - Cache public keys securely with rotation

3. **Session Management**
   - Implement session timeout
   - Regenerate session IDs after authentication
   - Use secure session storage
   - Implement concurrent session limits

### Authorization Best Practices

1. **Principle of Least Privilege**
   - Grant minimum necessary permissions
   - Implement role-based access control
   - Use attribute-based access control for complex scenarios
   - Regular access reviews and cleanup

2. **Resource-Level Security**
   - Implement tenant isolation
   - Validate resource ownership
   - Use resource-specific permissions
   - Implement field-level access control

### Data Protection Best Practices

1. **Encryption**
   - Encrypt sensitive data at rest
   - Use TLS 1.2+ for data in transit
   - Implement key rotation policies
   - Use envelope encryption for large datasets

2. **Data Handling**
   - Implement data classification
   - Use data masking for non-production environments
   - Implement secure data deletion
   - Monitor data access patterns

### Network Security Best Practices

1. **Network Controls**
   - Implement IP allowlisting
   - Use rate limiting aggressively
   - Configure CORS properly
   - Implement DDoS protection

2. **Monitoring**
   - Implement comprehensive logging
   - Use real-time alerting
   - Monitor for anomalous patterns
   - Implement automated response

### Deployment Security Best Practices

1. **Infrastructure Security**
   - Use minimal container images
   - Run as non-root user
   - Implement network policies
   - Use secrets management

2. **Configuration Security**
   - Never hardcode secrets
   - Use environment-specific configurations
   - Implement configuration validation
   - Regular security assessments

This comprehensive security guide provides the foundation for deploying and operating secure SCIM servers in production environments. Regular security reviews and updates are essential for maintaining security posture.