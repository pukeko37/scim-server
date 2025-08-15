# Web Framework Integration

This tutorial shows how to integrate SCIM Server with popular Rust web frameworks. SCIM Server is framework-agnostic, meaning it works with any HTTP library while providing consistent SCIM functionality.

## Overview

SCIM Server follows a layered architecture that separates HTTP handling from SCIM logic:

```
┌─────────────────┐    ┌──────────────────┐    ┌─────────────────┐
│   HTTP Layer    │    │ Resource Provider│    │   Storage       │
│                 │    │                  │    │                 │
│  • Axum         │───▶│  • Validation    │───▶│  • In-Memory    │
│  • Warp         │    │  • Operations    │    │  • Database     │
│  • Actix        │    │  • Type Safety   │    │  • Custom       │
│  • Custom       │    │  • Multi-tenant  │    │                 │
└─────────────────┘    └──────────────────┘    └─────────────────┘
```

This design allows you to use your preferred web framework while leveraging SCIM Server's enterprise-grade capabilities.

## Integration Patterns

### Common Integration Pattern

All framework integrations follow a similar pattern:

1. **Create RequestContext** from request metadata
2. **Parse JSON body** for create/update operations
3. **Handle ETags** for concurrency control
4. **Call StandardResourceProvider** with the extracted data
5. **Return SCIM-compliant responses** with proper headers

## Axum Integration

Axum is a modern, ergonomic web framework built on tokio and hyper.

### Dependencies

```toml
[dependencies]
scim-server = "=0.3.2"
axum = "0.7"
tokio = { version = "1.0", features = ["full"] }
tower = "0.4"
tower-http = { version = "0.5", features = ["cors"] }
serde_json = "1.0"
```

### Basic Server Setup

```rust
use axum::{
    extract::{Path, Query, State},
    http::{StatusCode, HeaderMap, HeaderValue},
    response::Json,
    routing::{get, post, put, delete},
    Router,
};
use scim_server::{
    providers::StandardResourceProvider,
    storage::InMemoryStorage,
    RequestContext,
};
use serde_json::{Value, json};
use std::collections::HashMap;
use tower_http::cors::CorsLayer;
use uuid::Uuid;

type AppState = StandardResourceProvider<InMemoryStorage>;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize SCIM Server with StandardResourceProvider
    let storage = InMemoryStorage::new();
    let provider = StandardResourceProvider::new(storage);

    // Build application with routes
    let app = Router::new()
        // SCIM v2 endpoints
        .route("/scim/v2/Users", post(create_user).get(list_users))
        .route("/scim/v2/Users/:id", 
            get(get_user).put(update_user).delete(delete_user))
        .route("/scim/v2/Groups", post(create_group).get(list_groups))
        .route("/scim/v2/Groups/:id", 
            get(get_group).put(update_group).delete(delete_group))
        // Multi-tenant endpoints
        .route("/tenants/:tenant_id/scim/v2/Users", post(create_user_mt).get(list_users_mt))
        .route("/tenants/:tenant_id/scim/v2/Users/:id", 
            get(get_user_mt).put(update_user_mt).delete(delete_user_mt))
        .with_state(provider)
        .layer(CorsLayer::permissive());

    // Start server
    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000").await?;
    println!("SCIM Server running on http://127.0.0.1:3000");
    axum::serve(listener, app).await?;

    Ok(())
}
```

### CRUD Operations

```rust
// Helper to create RequestContext from HTTP request
fn create_request_context() -> RequestContext {
    RequestContext::new(Uuid::new_v4().to_string())
}

// Create User
async fn create_user(
    State(provider): State<AppState>,
    Json(user_data): Json<Value>,
) -> Result<(StatusCode, HeaderMap, Json<Value>), AppError> {
    let context = create_request_context();
    let user = provider.create_resource("User", user_data, &context).await?;
    
    let mut headers = HeaderMap::new();
    headers.insert("Location", format!("/scim/v2/Users/{}", 
        user.get_id().unwrap_or("unknown")).parse()?);
    
    if let Some(meta) = user.get_meta() {
        if let Some(version) = &meta.version {
            headers.insert("ETag", format!("\"{}\"", version).parse()?);
        }
    }
    
    Ok((StatusCode::CREATED, headers, Json(user.data)))
}

// Get User
async fn get_user(
    State(provider): State<AppState>,
    Path(user_id): Path<String>,
) -> Result<(HeaderMap, Json<Value>), AppError> {
    let context = create_request_context();
    let user = provider.get_resource("User", &user_id, &context).await?;
    
    let mut headers = HeaderMap::new();
    if let Some(meta) = user.get_meta() {
        if let Some(version) = &meta.version {
            headers.insert("ETag", format!("\"{}\"", version).parse()?);
        }
    }
    
    Ok((headers, Json(user.data)))
}

// List Users with Filtering
async fn list_users(
    State(provider): State<AppState>,
    Query(params): Query<HashMap<String, String>>,
) -> Result<Json<Value>, AppError> {
    let context = create_request_context();
    
    // Get all users (provider handles this internally)
    let users = provider.list_resources("User", None, &context).await?;
    
    // Apply client-side filtering if filter parameter provided
    let filtered_users = if let Some(filter_str) = params.get("filter") {
        // Simple filtering example - extend for full SCIM filter syntax
        users.into_iter()
            .filter(|user| {
                if filter_str.contains("active eq true") {
                    user.get_active().unwrap_or(true)
                } else {
                    true
                }
            })
            .collect::<Vec<_>>()
    } else {
        users
    };
    
    // Apply pagination
    let start_index = params.get("startIndex")
        .and_then(|s| s.parse::<usize>().ok())
        .unwrap_or(1);
    let count = params.get("count")
        .and_then(|s| s.parse::<usize>().ok())
        .unwrap_or(20);
    
    let start = (start_index - 1).min(filtered_users.len());
    let end = (start + count).min(filtered_users.len());
    let page_users = &filtered_users[start..end];
    
    // Create SCIM list response
    let response = json!({
        "schemas": ["urn:ietf:params:scim:api:messages:2.0:ListResponse"],
        "totalResults": filtered_users.len(),
        "startIndex": start_index,
        "itemsPerPage": page_users.len(),
        "Resources": page_users.iter().map(|u| &u.data).collect::<Vec<_>>()
    });
    
    Ok(Json(response))
}

// Update User with ETag support
async fn update_user(
    State(provider): State<AppState>,
    Path(user_id): Path<String>,
    headers: HeaderMap,
    Json(user_data): Json<Value>,
) -> Result<(HeaderMap, Json<Value>), AppError> {
    let context = create_request_context();
    
    // Extract If-Match header for conditional updates
    let if_match = headers.get("if-match")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.trim_matches('"'));
    
    // For ETag support, you could verify the version matches current resource
    if let Some(_expected_version) = if_match {
        // Get current user to check version
        let current_user = provider.get_resource("User", &user_id, &context).await?;
        if let Some(meta) = current_user.get_meta() {
            if let Some(current_version) = &meta.version {
                if current_version != _expected_version {
                    return Err(AppError::VersionMismatch);
                }
            }
        }
    }
    
    let user = provider.update_resource("User", &user_id, user_data, &context).await?;
    
    let mut response_headers = HeaderMap::new();
    if let Some(meta) = user.get_meta() {
        if let Some(version) = &meta.version {
            response_headers.insert("ETag", format!("\"{}\"", version).parse()?);
        }
    }
    
    Ok((response_headers, Json(user.data)))
}

// Delete User
async fn delete_user(
    State(provider): State<AppState>,
    Path(user_id): Path<String>,
) -> Result<StatusCode, AppError> {
    let context = create_request_context();
    provider.delete_resource("User", &user_id, &context).await?;
    Ok(StatusCode::NO_CONTENT)
}
```

### Multi-Tenant Endpoints

```rust
// Multi-tenant user creation
async fn create_user_mt(
    State(provider): State<AppState>,
    Path(tenant_id): Path<String>,
    Json(user_data): Json<Value>,
) -> Result<(StatusCode, HeaderMap, Json<Value>), AppError> {
    // Create context with tenant information
    let context = RequestContext::new(format!("tenant-{}-{}", tenant_id, Uuid::new_v4()));
    let user = provider.create_resource("User", user_data, &context).await?;
    
    let mut headers = HeaderMap::new();
    headers.insert("Location", 
        format!("/tenants/{}/scim/v2/Users/{}", tenant_id, 
        user.get_id().unwrap_or("unknown")).parse()?);
    
    if let Some(meta) = user.get_meta() {
        if let Some(version) = &meta.version {
            headers.insert("ETag", format!("\"{}\"", version).parse()?);
        }
    }
    
    Ok((StatusCode::CREATED, headers, Json(user.data)))
}

// Multi-tenant user retrieval
async fn get_user_mt(
    State(provider): State<AppState>,
    Path((tenant_id, user_id)): Path<(String, String)>,
) -> Result<(HeaderMap, Json<Value>), AppError> {
    let context = RequestContext::new(format!("tenant-{}-{}", tenant_id, Uuid::new_v4()));
    let user = provider.get_resource("User", &user_id, &context).await?;
    
    let mut headers = HeaderMap::new();
    if let Some(meta) = user.get_meta() {
        if let Some(version) = &meta.version {
            headers.insert("ETag", format!("\"{}\"", version).parse()?);
        }
    }
    
    Ok((headers, Json(user.data)))
}

// Multi-tenant user listing
async fn list_users_mt(
    State(provider): State<AppState>,
    Path(tenant_id): Path<String>,
    Query(params): Query<HashMap<String, String>>,
) -> Result<Json<Value>, AppError> {
    let context = RequestContext::new(format!("tenant-{}-{}", tenant_id, Uuid::new_v4()));
    let users = provider.list_resources("User", None, &context).await?;
    
    // Apply pagination
    let start_index = params.get("startIndex")
        .and_then(|s| s.parse::<usize>().ok())
        .unwrap_or(1);
    let count = params.get("count")
        .and_then(|s| s.parse::<usize>().ok())
        .unwrap_or(20);
    
    let start = (start_index - 1).min(users.len());
    let end = (start + count).min(users.len());
    let page_users = &users[start..end];
    
    let response = json!({
        "schemas": ["urn:ietf:params:scim:api:messages:2.0:ListResponse"],
        "totalResults": users.len(),
        "startIndex": start_index,
        "itemsPerPage": page_users.len(),
        "Resources": page_users.iter().map(|u| &u.data).collect::<Vec<_>>()
    });
    
    Ok(Json(response))
}

// Multi-tenant user update
async fn update_user_mt(
    State(provider): State<AppState>,
    Path((tenant_id, user_id)): Path<(String, String)>,
    headers: HeaderMap,
    Json(user_data): Json<Value>,
) -> Result<(HeaderMap, Json<Value>), AppError> {
    let context = RequestContext::new(format!("tenant-{}-{}", tenant_id, Uuid::new_v4()));
    
    // Handle ETag if present
    if let Some(if_match) = headers.get("if-match").and_then(|v| v.to_str().ok()) {
        let expected_version = if_match.trim_matches('"');
        let current_user = provider.get_resource("User", &user_id, &context).await?;
        if let Some(meta) = current_user.get_meta() {
            if let Some(current_version) = &meta.version {
                if current_version != expected_version {
                    return Err(AppError::VersionMismatch);
                }
            }
        }
    }
    
    let user = provider.update_resource("User", &user_id, user_data, &context).await?;
    
    let mut response_headers = HeaderMap::new();
    if let Some(meta) = user.get_meta() {
        if let Some(version) = &meta.version {
            response_headers.insert("ETag", format!("\"{}\"", version).parse()?);
        }
    }
    
    Ok((response_headers, Json(user.data)))
}

// Multi-tenant user deletion
async fn delete_user_mt(
    State(provider): State<AppState>,
    Path((tenant_id, user_id)): Path<(String, String)>,
) -> Result<StatusCode, AppError> {
    let context = RequestContext::new(format!("tenant-{}-{}", tenant_id, Uuid::new_v4()));
    provider.delete_resource("User", &user_id, &context).await?;
    Ok(StatusCode::NO_CONTENT)
}
```

### Error Handling

```rust
use axum::{response::{Response, IntoResponse}, http::StatusCode};
use serde_json::json;

#[derive(Debug)]
enum AppError {
    NotFound,
    VersionMismatch,
    ValidationError(String),
    InternalError(String),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, error_code, message) = match self {
            AppError::NotFound => (
                StatusCode::NOT_FOUND,
                "resourceNotFound",
                "The specified resource was not found",
            ),
            AppError::VersionMismatch => (
                StatusCode::PRECONDITION_FAILED,
                "versionMismatch", 
                "The resource version does not match",
            ),
            AppError::ValidationError(msg) => (
                StatusCode::BAD_REQUEST,
                "invalidData",
                &msg,
            ),
            AppError::InternalError(msg) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "internalError",
                &msg,
            ),
        };

        let body = json!({
            "schemas": ["urn:ietf:params:scim:api:messages:2.0:Error"],
            "status": status.as_u16().to_string(),
            "scimType": error_code,
            "detail": message
        });

        (status, axum::Json(body)).into_response()
    }
}

impl<E> From<E> for AppError
where
    E: std::error::Error + Send + Sync + 'static,
{
    fn from(err: E) -> Self {
        let error_str = err.to_string();
        if error_str.contains("not found") {
            AppError::NotFound
        } else if error_str.contains("validation") {
            AppError::ValidationError(error_str)
        } else {
            AppError::InternalError(error_str)
        }
    }
}

```

### Group Operations

```rust
// Create Group
async fn create_group(
    State(provider): State<AppState>,
    Json(group_data): Json<Value>,
) -> Result<(StatusCode, HeaderMap, Json<Value>), AppError> {
    let context = create_request_context();
    let group = provider.create_resource("Group", group_data, &context).await?;
    
    let mut headers = HeaderMap::new();
    headers.insert("Location", format!("/scim/v2/Groups/{}", 
        group.get_id().unwrap_or("unknown")).parse()?);
    
    if let Some(meta) = group.get_meta() {
        if let Some(version) = &meta.version {
            headers.insert("ETag", format!("\"{}\"", version).parse()?);
        }
    }
    
    Ok((StatusCode::CREATED, headers, Json(group.data)))
}

// Get Group
async fn get_group(
    State(provider): State<AppState>,
    Path(group_id): Path<String>,
) -> Result<(HeaderMap, Json<Value>), AppError> {
    let context = create_request_context();
    let group = provider.get_resource("Group", &group_id, &context).await?;
    
    let mut headers = HeaderMap::new();
    if let Some(meta) = group.get_meta() {
        if let Some(version) = &meta.version {
            headers.insert("ETag", format!("\"{}\"", version).parse()?);
        }
    }
    
    Ok((headers, Json(group.data)))
}

// List Groups
async fn list_groups(
    State(provider): State<AppState>,
    Query(params): Query<HashMap<String, String>>,
) -> Result<Json<Value>, AppError> {
    let context = create_request_context();
    let groups = provider.list_resources("Group", None, &context).await?;
    
    // Apply pagination
    let start_index = params.get("startIndex")
        .and_then(|s| s.parse::<usize>().ok())
        .unwrap_or(1);
    let count = params.get("count")
        .and_then(|s| s.parse::<usize>().ok())
        .unwrap_or(20);
    
    let start = (start_index - 1).min(groups.len());
    let end = (start + count).min(groups.len());
    let page_groups = &groups[start..end];
    
    let response = json!({
        "schemas": ["urn:ietf:params:scim:api:messages:2.0:ListResponse"],
        "totalResults": groups.len(),
        "startIndex": start_index,
        "itemsPerPage": page_groups.len(),
        "Resources": page_groups.iter().map(|g| &g.data).collect::<Vec<_>>()
    });
    
    Ok(Json(response))
}

// Update Group
async fn update_group(
    State(provider): State<AppState>,
    Path(group_id): Path<String>,
    headers: HeaderMap,
    Json(group_data): Json<Value>,
) -> Result<(HeaderMap, Json<Value>), AppError> {
    let context = create_request_context();
    
    // Handle ETag if present
    if let Some(if_match) = headers.get("if-match").and_then(|v| v.to_str().ok()) {
        let expected_version = if_match.trim_matches('"');
        let current_group = provider.get_resource("Group", &group_id, &context).await?;
        if let Some(meta) = current_group.get_meta() {
            if let Some(current_version) = &meta.version {
                if current_version != expected_version {
                    return Err(AppError::VersionMismatch);
                }
            }
        }
    }
    
    let group = provider.update_resource("Group", &group_id, group_data, &context).await?;
    
    let mut response_headers = HeaderMap::new();
    if let Some(meta) = group.get_meta() {
        if let Some(version) = &meta.version {
            response_headers.insert("ETag", format!("\"{}\"", version).parse()?);
        }
    }
    
    Ok((response_headers, Json(group.data)))
}

// Delete Group
async fn delete_group(
    State(provider): State<AppState>,
    Path(group_id): Path<String>,
) -> Result<StatusCode, AppError> {
    let context = create_request_context();
    provider.delete_resource("Group", &group_id, &context).await?;
    Ok(StatusCode::NO_CONTENT)
}
```

## Warp Integration

Warp is a composable web framework focusing on filters and type safety.

### Dependencies

```toml
[dependencies]
scim-server = "=0.3.2"
warp = "0.3"
tokio = { version = "1.0", features = ["full"] }
serde_json = "1.0"
uuid = { version = "1.0", features = ["v4"] }
```

### Basic Server Setup

```rust
use warp::{Filter, Reply, Rejection};
use scim_server::{
    providers::StandardResourceProvider,
    storage::InMemoryStorage,
    RequestContext,
};
use serde_json::{Value, json};
use std::convert::Infallible;
use std::sync::Arc;
use uuid::Uuid;

type SharedProvider = Arc<StandardResourceProvider<InMemoryStorage>>;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize SCIM Server with StandardResourceProvider
    let storage = InMemoryStorage::new();
    let provider = Arc::new(StandardResourceProvider::new(storage));

    // CORS configuration
    let cors = warp::cors()
        .allow_any_origin()
        .allow_headers(vec!["content-type", "authorization", "if-match"])
        .allow_methods(vec!["GET", "POST", "PUT", "DELETE"]);

    // Base path filter
    let scim_v2 = warp::path("scim").and(warp::path("v2"));
    
    // Provider state filter
    let with_provider = warp::any().map(move || provider.clone());

    // User routes
    let users = scim_v2
        .and(warp::path("Users"))
        .and(with_provider.clone())
        .and(
            // POST /scim/v2/Users
            warp::post()
                .and(warp::body::json())
                .and_then(create_user_handler)
            .or(
                // GET /scim/v2/Users
                warp::get()
                    .and(warp::query::query())
                    .and_then(list_users_handler)
            )
        );

    let user_by_id = scim_v2
        .and(warp::path("Users"))
        .and(warp::path::param::<String>())
        .and(with_provider.clone())
        .and(
            // GET /scim/v2/Users/{id}
            warp::get()
                .and_then(get_user_handler)
            .or(
                // PUT /scim/v2/Users/{id}
                warp::put()
                    .and(warp::header::optional::<String>("if-match"))
                    .and(warp::body::json())
                    .and_then(update_user_handler)
            )
            .or(
                // DELETE /scim/v2/Users/{id}
                warp::delete()
                    .and_then(delete_user_handler)
            )
        );

    let routes = users
        .or(user_by_id)
        .with(cors)
        .recover(handle_rejection);
    // Start server
    println!("SCIM Server running on http://127.0.0.1:3030");
    warp::serve(routes)
        .run(([127, 0, 0, 1], 3030))
        .await;

    Ok(())
}

// Helper to create RequestContext
fn create_request_context() -> RequestContext {
    RequestContext::new(Uuid::new_v4().to_string())
}

// Warp handlers
async fn create_user_handler(
    provider: SharedProvider,
    user_data: Value,
) -> Result<impl Reply, Rejection> {
    let context = create_request_context();
    
    match provider.create_resource("User", user_data, &context).await {
        Ok(user) => {
            let mut response = warp::reply::with_status(
                warp::reply::json(&user.data), 
                warp::http::StatusCode::CREATED
            );
            
            if let Some(meta) = user.get_meta() {
                if let Some(version) = &meta.version {
                    response = warp::reply::with_header(
                        response,
                        "ETag",
                        format!("\"{}\"", version)
                    );
                }
            }
            
            Ok(response)
        },
        Err(e) => Err(warp::reject::custom(WarpError::from(e))),
    }
}

async fn list_users_handler(
    provider: SharedProvider,
    params: std::collections::HashMap<String, String>,
) -> Result<impl Reply, Rejection> {
    let context = create_request_context();
    
    match provider.list_resources("User", None, &context).await {
        Ok(users) => {
            let start_index = params.get("startIndex")
                .and_then(|s| s.parse::<usize>().ok())
                .unwrap_or(1);
            let count = params.get("count")
                .and_then(|s| s.parse::<usize>().ok())
                .unwrap_or(20);
            
            let start = (start_index - 1).min(users.len());
            let end = (start + count).min(users.len());
            let page_users = &users[start..end];
            
            let response = json!({
                "schemas": ["urn:ietf:params:scim:api:messages:2.0:ListResponse"],
                "totalResults": users.len(),
                "startIndex": start_index,
                "itemsPerPage": page_users.len(),
                "Resources": page_users.iter().map(|u| &u.data).collect::<Vec<_>>()
            });
            
            Ok(warp::reply::json(&response))
        },
        Err(e) => Err(warp::reject::custom(WarpError::from(e))),
    }
}

// Error handling for Warp
#[derive(Debug)]
struct WarpError {
    message: String,
    status: warp::http::StatusCode,
}

impl warp::reject::Reject for WarpError {}

impl<E> From<E> for WarpError
where
    E: std::error::Error + Send + Sync + 'static,
{
    fn from(err: E) -> Self {
        let error_str = err.to_string();
        let status = if error_str.contains("not found") {
            warp::http::StatusCode::NOT_FOUND
        } else if error_str.contains("validation") {
            warp::http::StatusCode::BAD_REQUEST
        } else {
            warp::http::StatusCode::INTERNAL_SERVER_ERROR
        };
        
        Self { message: error_str, status }
    }
}

async fn handle_rejection(err: Rejection) -> Result<impl Reply, std::convert::Infallible> {
    if let Some(warp_error) = err.find::<WarpError>() {
        let body = json!({
            "schemas": ["urn:ietf:params:scim:api:messages:2.0:Error"],
            "status": warp_error.status.as_u16().to_string(),
            "detail": warp_error.message
        });
        
        Ok(warp::reply::with_status(
            warp::reply::json(&body),
            warp_error.status
        ))
    } else {
        let body = json!({
            "schemas": ["urn:ietf:params:scim:api:messages:2.0:Error"],
            "status": "500",
            "detail": "Internal server error"
        });
        
        Ok(warp::reply::with_status(
            warp::reply::json(&body),
            warp::http::StatusCode::INTERNAL_SERVER_ERROR
        ))
    }
}
```

## Actix Web Integration

Actix Web is a powerful, pragmatic web framework for Rust.

### Dependencies

```toml
[dependencies]
scim-server = "=0.3.2"
actix-web = "4"
tokio = { version = "1.0", features = ["full"] }
serde_json = "1.0"
```

### Basic Server Setup

```rust
use actix_web::{
    web, App, HttpServer, HttpRequest, HttpResponse, Result,
    middleware::Logger,
};
use scim_server::{
    providers::StandardResourceProvider,
    storage::InMemoryStorage,
    RequestContext,
};
use serde_json::{Value, json};
use std::sync::Arc;
use uuid::Uuid;

type AppData = web::Data<StandardResourceProvider<InMemoryStorage>>;

// Helper to create RequestContext
fn create_request_context() -> RequestContext {
    RequestContext::new(Uuid::new_v4().to_string())
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    env_logger::init();

    // Initialize SCIM Server with StandardResourceProvider
    let storage = InMemoryStorage::new();
    let provider = StandardResourceProvider::new(storage);
    let app_data = web::Data::new(provider);

    HttpServer::new(move || {
        App::new()
            .app_data(app_data.clone())
            .wrap(Logger::default())
            .service(
                web::scope("/scim/v2")
                    .service(
                        web::resource("/Users")
                            .route(web::post().to(create_user))
                            .route(web::get().to(list_users))
                    )
                    .service(
                        web::resource("/Users/{id}")
                            .route(web::get().to(get_user))
                            .route(web::put().to(update_user))
                            .route(web::delete().to(delete_user))
                    )
                    .service(
                        web::resource("/Groups")
                            .route(web::post().to(create_group))
                            .route(web::get().to(list_groups))
                    )
            )
    })
    .bind("127.0.0.1:8080")?
    .run()
    .await
}
```

### Handler Functions

```rust
use actix_web::{web::Path, web::Json, web::Query, HttpRequest};
use std::collections::HashMap;

async fn create_user(
    provider: AppData,
    user_data: Json<Value>,
) -> Result<HttpResponse> {
    let context = create_request_context();
    
    match provider.create_resource("User", user_data.into_inner(), &context).await {
        Ok(user) => {
            let mut response = HttpResponse::Created().json(&user.data);
            
            if let Some(meta) = user.get_meta() {
                if let Some(version) = &meta.version {
                    response.headers_mut().insert(
                        actix_web::http::header::ETAG,
                        actix_web::http::header::HeaderValue::from_str(&format!("\"{}\"", version)).unwrap()
                    );
                }
            }
            
            Ok(response)
        },
        Err(e) => Ok(HttpResponse::BadRequest().json(json!({
            "schemas": ["urn:ietf:params:scim:api:messages:2.0:Error"],
            "detail": e.to_string(),
            "status": "400"
        }))),
    }
}

async fn get_user(
    provider: AppData,
    path: Path<String>,
) -> Result<HttpResponse> {
    let user_id = path.into_inner();
    let context = create_request_context();
    
    match provider.get_resource("User", &user_id, &context).await {
        Ok(user) => {
            let mut response = HttpResponse::Ok().json(&user.data);
            
            if let Some(meta) = user.get_meta() {
                if let Some(version) = &meta.version {
                    response.headers_mut().insert(
                        actix_web::http::header::ETAG,
                        actix_web::http::header::HeaderValue::from_str(&format!("\"{}\"", version)).unwrap()
                    );
                }
            }
            
            Ok(response)
        },
        Err(e) => Ok(HttpResponse::NotFound().json(json!({
            "schemas": ["urn:ietf:params:scim:api:messages:2.0:Error"],
            "detail": e.to_string(),
            "status": "404"
        }))),
    }
}

async fn list_users(
    provider: AppData,
    query: Query<HashMap<String, String>>,
) -> Result<HttpResponse> {
    let context = create_request_context();
    
    match provider.list_resources("User", None, &context).await {
        Ok(users) => {
            let start_index = query.get("startIndex")
                .and_then(|s| s.parse::<usize>().ok())
                .unwrap_or(1);
            let count = query.get("count")
                .and_then(|s| s.parse::<usize>().ok())
                .unwrap_or(20);
            
            let start = (start_index - 1).min(users.len());
            let end = (start + count).min(users.len());
            let page_users = &users[start..end];
            
            let response = json!({
                "schemas": ["urn:ietf:params:scim:api:messages:2.0:ListResponse"],
                "totalResults": users.len(),
                "startIndex": start_index,
                "itemsPerPage": page_users.len(),
                "Resources": page_users.iter().map(|u| &u.data).collect::<Vec<_>>()
            });
            
            Ok(HttpResponse::Ok().json(response))
        },
        Err(e) => Ok(HttpResponse::InternalServerError().json(json!({
            "schemas": ["urn:ietf:params:scim:api:messages:2.0:Error"],
            "detail": e.to_string(),
            "status": "500"
        }))),
    }
}

async fn update_user(
    provider: AppData,
    path: Path<String>,
    user_data: Json<Value>,
    req: HttpRequest,
) -> Result<HttpResponse> {
    let user_id = path.into_inner();
    let context = create_request_context();
    
    // Handle ETag if present
    if let Some(if_match) = req.headers().get("if-match") {
        if let Ok(expected_version) = if_match.to_str() {
            let expected_version = expected_version.trim_matches('"');
            
            match provider.get_resource("User", &user_id, &context).await {
                Ok(current_user) => {
                    if let Some(meta) = current_user.get_meta() {
                        if let Some(current_version) = &meta.version {
                            if current_version != expected_version {
                                return Ok(HttpResponse::PreconditionFailed().json(json!({
                                    "schemas": ["urn:ietf:params:scim:api:messages:2.0:Error"],
                                    "detail": "Version mismatch",
                                    "status": "412"
                                })));
                            }
                        }
                    }
                },
                Err(_) => return Ok(HttpResponse::NotFound().json(json!({
                    "schemas": ["urn:ietf:params:scim:api:messages:2.0:Error"],
                    "detail": "User not found",
                    "status": "404"
                }))),
            }
        }
    }
    
    match provider.update_resource("User", &user_id, user_data.into_inner(), &context).await {
        Ok(user) => {
            let mut response = HttpResponse::Ok().json(&user.data);
            
            if let Some(meta) = user.get_meta() {
                if let Some(version) = &meta.version {
                    response.headers_mut().insert(
                        actix_web::http::header::ETAG,
                        actix_web::http::header::HeaderValue::from_str(&format!("\"{}\"", version)).unwrap()
                    );
                }
            }
            
            Ok(response)
        },
        Err(e) => Ok(HttpResponse::BadRequest().json(json!({
            "schemas": ["urn:ietf:params:scim:api:messages:2.0:Error"],
            "detail": e.to_string(),
            "status": "400"
        }))),
    }
}

async fn delete_user(
    provider: AppData,
    path: Path<String>,
) -> Result<HttpResponse> {
    let user_id = path.into_inner();
    let context = create_request_context();
    
    match provider.delete_resource("User", &user_id, &context).await {
        Ok(_) => Ok(HttpResponse::NoContent().finish()),
        Err(e) => Ok(HttpResponse::NotFound().json(json!({
            "schemas": ["urn:ietf:params:scim:api:messages:2.0:Error"],
            "detail": e.to_string(),
            "status": "404"
        }))),
    }
}

// Group handlers follow the same pattern
async fn create_group(
    provider: AppData,
    group_data: Json<Value>,
) -> Result<HttpResponse> {
    let context = create_request_context();
    
    match provider.create_resource("Group", group_data.into_inner(), &context).await {
        Ok(group) => Ok(HttpResponse::Created().json(&group.data)),
        Err(e) => Ok(HttpResponse::BadRequest().json(json!({
            "schemas": ["urn:ietf:params:scim:api:messages:2.0:Error"],
            "detail": e.to_string(),
            "status": "400"
        }))),
    }
}

async fn list_groups(
    provider: AppData,
    query: Query<HashMap<String, String>>,
) -> Result<HttpResponse> {
    let context = create_request_context();
    
    match provider.list_resources("Group", None, &context).await {
        Ok(groups) => {
            let response = json!({
                "schemas": ["urn:ietf:params:scim:api:messages:2.0:ListResponse"],
                "totalResults": groups.len(),
                "startIndex": 1,
                "itemsPerPage": groups.len(),
                "Resources": groups.iter().map(|g| &g.data).collect::<Vec<_>>()
            });
            
            Ok(HttpResponse::Ok().json(response))
        },
        Err(e) => Ok(HttpResponse::InternalServerError().json(json!({
            "schemas": ["urn:ietf:params:scim:api:messages:2.0:Error"],
            "detail": e.to_string(),
            "status": "500"
        }))),
    }
}
```

## Framework-Specific Considerations

### Axum
- **Strengths**: Modern async design, excellent type safety, minimal boilerplate
- **Best for**: New projects, type-safe APIs, microservices
- **SCIM Integration**: Clean state management with `State<T>`, built-in JSON handling

### Warp  
- **Strengths**: Functional approach, composable filters, zero-cost abstractions
- **Best for**: High-performance APIs, functional programming enthusiasts
- **SCIM Integration**: Filter composition allows flexible middleware

### Actix Web
- **Strengths**: Mature ecosystem, high performance, extensive middleware
- **Best for**: Production applications, teams familiar with traditional web frameworks
- **SCIM Integration**: Straightforward handler patterns, robust error handling

## Common Patterns Across Frameworks

### Request Context Creation
All integrations use the same pattern for creating request contexts:

```rust
fn create_request_context() -> RequestContext {
    RequestContext::new(Uuid::new_v4().to_string())
}
```

### ETag Handling
Consistent ETag support across frameworks:

```rust
// Extract If-Match header
let if_match = headers.get("if-match")
    .and_then(|v| v.to_str().ok())
    .map(|s| s.trim_matches('"'));

// Set ETag in response
if let Some(version) = &meta.version {
    headers.insert("ETag", format!("\"{}\"", version));
}
```

### Error Response Format
Standard SCIM error responses:

```rust
let error_response = json!({
    "schemas": ["urn:ietf:params:scim:api:messages:2.0:Error"],
    "status": status_code.to_string(),
    "detail": error_message
});
```

## Multi-Tenant URL Patterns

All frameworks support multi-tenant deployments with URL patterns like:

- Single tenant: `/scim/v2/Users`
- Multi-tenant: `/tenants/{tenant_id}/scim/v2/Users`

The key is creating request contexts that include tenant information:

```rust
let context = RequestContext::new(format!("tenant-{}-{}", tenant_id, Uuid::new_v4()));
```

## Best Practices

### 1. Consistent Error Handling
- Always return SCIM-compliant error responses
- Include proper HTTP status codes
- Provide meaningful error messages

### 2. ETag Support
- Implement conditional requests for concurrency control
- Return ETags in response headers
- Handle If-Match headers for updates

### 3. Request Context Management
- Create unique request contexts for operation tracking
- Include tenant information for multi-tenant scenarios
- Use UUIDs for request correlation

### 4. Performance Considerations
- Use async/await throughout the request pipeline
- Leverage framework-specific optimizations
- Consider connection pooling for database storage

### 5. Security
- Validate all input data
- Implement proper authentication middleware
- Use HTTPS in production
- Sanitize error messages to avoid information leakage

## Testing Your Integration

### Unit Tests
Test individual handlers with mock data:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use scim_server::storage::InMemoryStorage;

    #[tokio::test]
    async fn test_create_user() {
        let storage = InMemoryStorage::new();
        let provider = StandardResourceProvider::new(storage);
        let context = RequestContext::new("test".to_string());
        
        let user_data = json!({
            "userName": "test@example.com",
            "active": true
        });
        
        let result = provider.create_resource("User", user_data, &context).await;
        assert!(result.is_ok());
    }
}
```

### Integration Tests
Test complete HTTP request/response cycles:

```rust
#[actix_web::test]
async fn test_user_creation_endpoint() {
    let app = test::init_service(
        App::new()
            .app_data(create_test_app_data())
            .service(web::resource("/Users").route(web::post().to(create_user)))
    ).await;

    let req = test::TestRequest::post()
        .uri("/Users")
        .set_json(&json!({"userName": "test@example.com"}))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 201);
}
```

## Summary

This tutorial demonstrated how to integrate SCIM Server with three popular Rust web frameworks:

✅ **Framework Integration Patterns**:
- Axum: Modern, type-safe with excellent async support
- Warp: Functional, composable filters for flexibility  
- Actix Web: Mature, high-performance with extensive middleware

✅ **Key Implementation Details**:
- StandardResourceProvider usage across all frameworks
- RequestContext creation and management
- ETag support for concurrency control
- SCIM-compliant error handling
- Multi-tenant URL patterns

✅ **Production Considerations**:
- Security best practices
- Performance optimization
- Testing strategies
- Error handling standards

Choose the framework that best fits your team's expertise and project requirements. All three provide excellent foundations for building production SCIM servers with the SCIM Server library.

**Next Steps**: 
- [Authentication Setup](./authentication-setup.md) - Secure your SCIM endpoints
- [Multi-Tenant Deployment](./multi-tenant-deployment.md) - Scale to multiple organizations
- [Custom Resources](./custom-resources.md) - Extend beyond Users and Groups
    let tenant_id = TenantId::default();
