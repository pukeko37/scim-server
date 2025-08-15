# Troubleshooting

This guide helps you diagnose and resolve common issues when working with the SCIM Server library. It covers typical problems, debugging techniques, and performance optimization.

## Common Issues

### Connection and Network Problems

#### "Connection refused" or "Unable to connect"

**Symptoms:**
- Server fails to start
- Client connections are rejected
- Network timeouts

**Causes & Solutions:**

```rust
// Check if port is already in use
use tokio::net::TcpListener;

// This will fail if port is occupied
let listener = TcpListener::bind("0.0.0.0:3000").await?;
```

**Solution 1: Change port**
```rust
// Try a different port
let listener = TcpListener::bind("0.0.0.0:3001").await?;
```

**Solution 2: Find and kill process using port**
```bash
# Find process using port 3000
lsof -i :3000
# Kill the process
kill -9 <PID>
```

**Solution 3: Bind to correct interface**
```rust
// For local development
let listener = TcpListener::bind("127.0.0.1:3000").await?;

// For production (all interfaces)
let listener = TcpListener::bind("0.0.0.0:3000").await?;
```

#### Database Connection Issues

**Error: "Failed to connect to database"**

```rust
use scim_server::DatabaseProvider;

// Add connection retry logic
async fn connect_with_retry(url: &str, max_retries: u32) -> Result<DatabaseProvider, Error> {
    let mut attempts = 0;
    
    loop {
        match DatabaseProvider::new(url).await {
            Ok(provider) => return Ok(provider),
            Err(e) if attempts < max_retries => {
                attempts += 1;
                println!("Connection attempt {} failed: {}. Retrying in 5 seconds...", attempts, e);
                tokio::time::sleep(Duration::from_secs(5)).await;
            },
            Err(e) => return Err(e.into()),
        }
    }
}
```

**Common database URL formats:**
```rust
// PostgreSQL
"postgresql://username:password@localhost:5432/scim_db"

// SQLite
"sqlite:./scim.db"
"sqlite::memory:" // For testing

// MySQL
"mysql://username:password@localhost:3306/scim_db"
```

### Authentication and Authorization Issues

#### "401 Unauthorized" responses

**Check authentication configuration:**
```rust
async fn debug_auth_middleware(
    headers: HeaderMap,
    request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    // Log all headers for debugging
    for (name, value) in headers.iter() {
        println!("Header: {} = {:?}", name, value);
    }
    
    // Check for Authorization header
    if let Some(auth_header) = headers.get("Authorization") {
        println!("Auth header found: {:?}", auth_header);
    } else {
        println!("No Authorization header found");
        return Err(StatusCode::UNAUTHORIZED);
    }
    
    Ok(next.run(request).await)
}
```

**Common authentication problems:**
```rust
// Problem: Missing Bearer prefix
// Wrong: "abc123def456"
// Correct: "Bearer abc123def456"

// Problem: Expired tokens
let now = chrono::Utc::now().timestamp() as usize;
if token_claims.exp < now {
    return Err(StatusCode::UNAUTHORIZED);
}

// Problem: Incorrect Base64 encoding for Basic auth
use base64::{Engine as _, engine::general_purpose};
let encoded = general_purpose::STANDARD.encode("username:password");
println!("Correct Basic auth: Basic {}", encoded);
```

#### "403 Forbidden" responses

**Debug permission checking:**
```rust
fn debug_permissions(user_perms: &[String], required_perm: &str) -> bool {
    println!("User permissions: {:?}", user_perms);
    println!("Required permission: {}", required_perm);
    
    let has_permission = user_perms.contains(&required_perm.to_string());
    println!("Permission granted: {}", has_permission);
    
    has_permission
}
```

### Resource and Data Issues

#### "404 Not Found" for existing resources

**Check tenant isolation:**
```rust
// Make sure you're using the correct tenant ID
let user = provider.get_user("correct-tenant-id", &user_id).await?;

// Debug tenant data
let all_tenants = provider.list_all_tenants().await?;
println!("Available tenants: {:?}", all_tenants);
```

**Verify resource IDs:**
```rust
// UUIDs are case-sensitive
let user_id = "2819c223-7f76-453a-919d-413861904646"; // Correct
let user_id = "2819C223-7F76-453A-919D-413861904646"; // Wrong case

// Check if resource exists
if let Some(user) = provider.get_user(tenant_id, user_id).await? {
    println!("User found: {}", user.username());
} else {
    println!("User not found with ID: {}", user_id);
    
    // List all users to debug
    let all_users = provider.list_users(tenant_id, &ListOptions::default()).await?;
    for user in all_users.resources {
        println!("Existing user: {} ({})", user.username(), user.id());
    }
}
```

#### "409 Conflict" on resource creation

**Check for duplicate constraints:**
```rust
// Debug uniqueness violations
match provider.create_user(tenant_id, user).await {
    Err(ProviderError::Conflict(msg)) => {
        println!("Conflict detected: {}", msg);
        
        // Check for existing username
        if let Some(existing) = provider.find_user_by_username(tenant_id, &user.username()).await? {
            println!("User with username '{}' already exists: {}", user.username(), existing.id());
        }
        
        // Check for existing email
        if let Some(email) = user.primary_email() {
            if let Some(existing) = provider.find_user_by_email(tenant_id, email).await? {
                println!("User with email '{}' already exists: {}", email, existing.id());
            }
        }
    },
    Ok(created_user) => println!("User created successfully: {}", created_user.id()),
    Err(e) => println!("Unexpected error: {}", e),
}
```

#### "412 Precondition Failed" on updates

**ETag version conflicts:**
```rust
// Always fetch latest version before update
let mut user = provider.get_user(tenant_id, user_id).await?
    .ok_or("User not found")?;

println!("Current user version: {}", user.meta().version);

// Make your changes
user.set_given_name("Updated Name");

// Update with current version
match provider.update_user(tenant_id, user).await {
    Ok(updated_user) => {
        println!("Update successful. New version: {}", updated_user.meta().version);
    },
    Err(ProviderError::VersionConflict { current_version, provided_version }) => {
        println!("Version conflict: expected {}, got {}", current_version, provided_version);
        
        // Retry with fresh data
        let fresh_user = provider.get_user(tenant_id, user_id).await?;
        // Apply changes to fresh user and retry
    },
    Err(e) => println!("Update failed: {}", e),
}
```

### Validation and Schema Issues

#### "400 Bad Request" with validation errors

**Debug JSON parsing:**
```rust
use serde_json;

// Test JSON deserialization
let json_str = r#"{"invalid": "json"}"#;
match serde_json::from_str::<ScimUser>(json_str) {
    Ok(user) => println!("User parsed successfully"),
    Err(e) => {
        println!("JSON parsing failed: {}", e);
        println!("Error location: line {}, column {}", e.line(), e.column());
    }
}
```

**Validate required fields:**
```rust
// Check for missing required fields
match ScimUser::builder()
    .username("test@example.com")
    // Missing other required fields
    .build() 
{
    Ok(user) => println!("User created: {}", user.id()),
    Err(ValidationError::RequiredField(field)) => {
        println!("Missing required field: {}", field);
    },
    Err(e) => println!("Validation error: {}", e),
}
```

**Schema validation debugging:**
```rust
// Check schema compliance
let user = ScimUser::builder()
    .username("test@example.com")
    .given_name("Test")
    .family_name("User")
    .build()?;

// Validate against schema
match schema_registry.validate_user(&user) {
    Ok(_) => println!("User is valid according to schema"),
    Err(ValidationError::SchemaViolation { field, message }) => {
        println!("Schema violation in field '{}': {}", field, message);
    },
    Err(e) => println!("Validation error: {}", e),
}
```

## Debugging Techniques

### Enable Detailed Logging

```rust
use tracing::{info, warn, error, debug};
use tracing_subscriber::{EnvFilter, fmt};

// Set up comprehensive logging
fn setup_logging() {
    let filter = EnvFilter::from_default_env()
        .add_directive("scim_server=debug".parse().unwrap())
        .add_directive("sqlx=info".parse().unwrap())
        .add_directive("hyper=info".parse().unwrap());

    fmt()
        .with_env_filter(filter)
        .with_target(true)
        .with_thread_ids(true)
        .with_file(true)
        .with_line_number(true)
        .init();
}

// Use in your application
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    setup_logging();
    
    info!("Starting SCIM server");
    
    // Your application code...
    
    Ok(())
}
```

**Environment variables for logging:**
```bash
# Enable debug logging
export RUST_LOG=scim_server=debug,sqlx=info

# Enable trace-level logging (very verbose)
export RUST_LOG=scim_server=trace

# Log only errors
export RUST_LOG=scim_server=error
```

### Request/Response Debugging

```rust
use axum::{
    body::Body,
    extract::Request,
    middleware::Next,
    response::Response,
};

async fn debug_middleware(
    request: Request,
    next: Next,
) -> Response {
    let method = request.method().clone();
    let uri = request.uri().clone();
    let headers = request.headers().clone();
    
    // Log request
    debug!("Incoming request: {} {}", method, uri);
    for (name, value) in headers.iter() {
        debug!("Request header: {}: {:?}", name, value);
    }
    
    let start = std::time::Instant::now();
    let response = next.run(request).await;
    let duration = start.elapsed();
    
    // Log response
    debug!(
        "Response: {} {} - {} in {:?}",
        method,
        uri,
        response.status(),
        duration
    );
    
    response
}

// Add to your router
let app = Router::new()
    .nest("/scim/v2", scim_routes())
    .layer(middleware::from_fn(debug_middleware));
```

### Database Query Debugging

```rust
// Enable SQL query logging for sqlx
use sqlx::postgres::PgPoolOptions;

let pool = PgPoolOptions::new()
    .max_connections(10)
    .connect(&database_url)
    .await?;

// Queries will be logged when RUST_LOG includes sqlx=debug
```

### Provider State Inspection

```rust
// For InMemoryProvider, add debugging methods
impl InMemoryProvider {
    pub async fn debug_state(&self) {
        let users = self.users.read().await;
        let groups = self.groups.read().await;
        
        println!("=== Provider State Debug ===");
        for (tenant_id, tenant_users) in users.iter() {
            println!("Tenant '{}' has {} users:", tenant_id, tenant_users.len());
            for (user_id, user) in tenant_users.iter() {
                println!("  User: {} ({})", user.username(), user_id);
            }
        }
        
        for (tenant_id, tenant_groups) in groups.iter() {
            println!("Tenant '{}' has {} groups:", tenant_id, tenant_groups.len());
            for (group_id, group) in tenant_groups.iter() {
                println!("  Group: {} ({}) with {} members", 
                         group.display_name(), group_id, group.members().len());
            }
        }
        println!("=== End Debug ===");
    }
}

// Use in your code
provider.debug_state().await;
```

## Performance Issues

### Slow Database Queries

**Add query timing:**
```rust
use std::time::Instant;

async fn timed_query<T, F, Fut>(operation: F) -> Result<T, Error>
where
    F: FnOnce() -> Fut,
    Fut: Future<Output = Result<T, Error>>,
{
    let start = Instant::now();
    let result = operation().await;
    let duration = start.elapsed();
    
    if duration > Duration::from_millis(100) {
        warn!("Slow query detected: {:?}", duration);
    }
    
    result
}

// Usage
let users = timed_query(|| provider.list_users(tenant_id, &options)).await?;
```

**Optimize filtering:**
```rust
// Current approach: Load all users then filter in memory
// Note: Database-level filtering is not yet implemented
let all_users = provider.list_users(tenant_id, &ListOptions::default()).await?;
let filtered: Vec<_> = all_users.resources.into_iter()
    .filter(|u| u.department() == Some("Engineering"))
    .collect();

// For large datasets, consider implementing pagination
let options = ListOptions::builder()
    .count(Some(50))  // Limit results
    .start_index(Some(1))
    .build();
let paginated_users = provider.list_users(tenant_id, &options).await?;
```

### Memory Issues

**Monitor memory usage:**
```rust
use sysinfo::{System, SystemExt};

fn log_memory_usage() {
    let mut system = System::new_all();
    system.refresh_memory();
    
    let used = system.used_memory();
    let total = system.total_memory();
    let percentage = (used as f64 / total as f64) * 100.0;
    
    info!("Memory usage: {} MB / {} MB ({:.1}%)", 
          used / 1024 / 1024, 
          total / 1024 / 1024, 
          percentage);
}

// Check periodically
tokio::spawn(async {
    loop {
        log_memory_usage();
        tokio::time::sleep(Duration::from_secs(60)).await;
    }
});
```

**Optimize large result sets:**
```rust
// Use pagination for large datasets
let mut start_index = 1;
let page_size = 100;

loop {
    let options = ListOptions::builder()
        .start_index(start_index)
        .count(page_size)
        .build();
    
    let page = provider.list_users(tenant_id, &options).await?;
    
    // Process page
    for user in page.resources {
        process_user(user).await?;
    }
    
    // Check if we're done
    if page.resources.len() < page_size {
        break;
    }
    
    start_index += page_size;
}
```

## Health Checks and Monitoring

### Implement Health Endpoints

```rust
use axum::{response::Json, http::StatusCode};
use serde_json::json;

async fn health_check(
    State(state): State<AppState>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    // Check provider health
    let provider_health = state.provider.health_check().await
        .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
    
    // Check database connectivity
    let db_status = match state.provider.get_user("health-check", "non-existent").await {
        Ok(None) | Err(ProviderError::NotFound { .. }) => "healthy",
        Err(_) => "unhealthy",
    };
    
    let response = json!({
        "status": "healthy",
        "timestamp": chrono::Utc::now(),
        "version": env!("CARGO_PKG_VERSION"),
        "components": {
            "provider": provider_health.status,
            "database": db_status
        }
    });
    
    Ok(Json(response))
}

// Add to router
let app = Router::new()
    .route("/health", get(health_check))
    .nest("/scim/v2", scim_routes());
```

### Metrics Collection

```rust
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

#[derive(Clone)]
struct Metrics {
    requests_total: Arc<AtomicU64>,
    requests_errors: Arc<AtomicU64>,
    users_created: Arc<AtomicU64>,
    users_updated: Arc<AtomicU64>,
}

impl Metrics {
    fn new() -> Self {
        Self {
            requests_total: Arc::new(AtomicU64::new(0)),
            requests_errors: Arc::new(AtomicU64::new(0)),
            users_created: Arc::new(AtomicU64::new(0)),
            users_updated: Arc::new(AtomicU64::new(0)),
        }
    }
    
    fn increment_requests(&self) {
        self.requests_total.fetch_add(1, Ordering::Relaxed);
    }
    
    fn increment_errors(&self) {
        self.requests_errors.fetch_add(1, Ordering::Relaxed);
    }
}

async fn metrics_endpoint(
    State(metrics): State<Metrics>,
) -> Json<serde_json::Value> {
    Json(json!({
        "requests_total": metrics.requests_total.load(Ordering::Relaxed),
        "requests_errors": metrics.requests_errors.load(Ordering::Relaxed),
        "users_created": metrics.users_created.load(Ordering::Relaxed),
        "users_updated": metrics.users_updated.load(Ordering::Relaxed),
    }))
}
```

## Testing and Validation

### Unit Test Debugging

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use tokio_test;
    
    #[tokio::test]
    async fn debug_user_creation() {
        // Enable logging in tests
        let _ = tracing_subscriber::fmt::try_init();
        
        let provider = InMemoryProvider::new();
        let tenant_id = "test-tenant";
        
        // Create test user
        let user = ScimUser::builder()
            .username("test@example.com")
            .given_name("Test")
            .family_name("User")
            .build()
            .unwrap();
        
        println!("Creating user: {:?}", user);
        
        let created = provider.create_user(tenant_id, user).await.unwrap();
        
        println!("Created user with ID: {}", created.id());
        
        // Verify creation
        let fetched = provider.get_user(tenant_id, created.id()).await.unwrap();
        assert!(fetched.is_some());
        
        println!("Test passed: user creation and retrieval");
    }
}
```

### Integration Test Setup

```rust
// Create test helper functions
pub async fn setup_test_server() -> (TestServer, String) {
    let provider = InMemoryProvider::new();
    let scim_server = ScimServer::builder()
        .provider(provider)
        .build();
    
    let app = create_app(scim_server);
    let server = TestServer::new(app).unwrap();
    let tenant_id = "test-tenant".to_string();
    
    (server, tenant_id)
}

pub fn create_test_user() -> ScimUser {
    ScimUser::builder()
        .username("test@example.com")
        .given_name("Test")
        .family_name("User")
        .email("test@example.com")
        .active(true)
        .build()
        .unwrap()
}

#[tokio::test]
async fn integration_test_user_lifecycle() {
    let (server, tenant_id) = setup_test_server().await;
    let user = create_test_user();
    
    // Create user
    let response = server
        .post(&format!("/scim/v2/{}/Users", tenant_id))
        .json(&user)
        .await;
    
    assert_eq!(response.status_code(), 201);
    
    let created_user: ScimUser = response.json();
    println!("Created user: {}", created_user.id());
    
    // Get user
    let response = server
        .get(&format!("/scim/v2/{}/Users/{}", tenant_id, created_user.id()))
        .await;
    
    assert_eq!(response.status_code(), 200);
    
    // Update user
    let mut updated_user = created_user.clone();
    updated_user.set_given_name("Updated");
    
    let response = server
        .put(&format!("/scim/v2/{}/Users/{}", tenant_id, created_user.id()))
        .json(&updated_user)
        .await;
    
    assert_eq!(response.status_code(), 200);
    
    // Delete user
    let response = server
        .delete(&format!("/scim/v2/{}/Users/{}", tenant_id, created_user.id()))
        .await;
    
    assert_eq!(response.status_code(), 204);
}
```

## Getting Help

### Collect Diagnostic Information

When reporting issues, include:

```rust
// Version information
println!("SCIM Server version: {}", env!("CARGO_PKG_VERSION"));
println!("Rust version: {}", env!("RUSTC_VERSION"));

// System information
use sysinfo::{System, SystemExt};
let mut system = System::new_all();
system.refresh_all();
println!("OS: {} {}", system.name().unwrap_or("Unknown"), system.os_version().unwrap_or("Unknown"));
println!("Total memory: {} MB", system.total_memory() / 1024 / 1024);

// Configuration
println!("Database URL: {}", env::var("DATABASE_URL").unwrap_or("Not set".to_string()));
println!("Log level: {}", env::var("RUST_LOG").unwrap_or("Not set".to_string()));
```

### Enable Maximum Logging

```bash
# Set maximum verbosity
export RUST_LOG=trace

# Run with backtrace
export RUST_BACKTRACE=full

# Run your application
cargo run
```

### Create Minimal Reproduction

```rust
// Create the smallest possible example that reproduces the issue
use scim_server::{ScimServer, InMemoryProvider, ScimUser};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let provider = InMemoryProvider::new();
    let scim_server = ScimServer::builder().provider(provider).build();
    
    // Minimal reproduction of your issue
    let user = ScimUser::builder()
        .username("test@example.com")
        .build()?;
    
    let result = scim_server.create_user("tenant-1", user).await;
    println!("Result: {:?}", result);
    
    Ok(())
}
```

This troubleshooting guide should help you identify and resolve most common issues with the SCIM Server library. For additional help, check the [GitHub Issues](https://github.com/your-repo/scim-server/issues) or create a new issue with your diagnostic information.