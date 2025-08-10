# Custom Providers Example

This document provides comprehensive examples of implementing custom resource providers for the SCIM Server. You'll learn how to create providers for different storage backends including databases, external APIs, and hybrid solutions.

## Table of Contents

- [Overview](#overview)
- [Provider Interface](#provider-interface)
- [Database Provider Example](#database-provider-example)
- [External API Provider Example](#external-api-provider-example)
- [Redis Provider Example](#redis-provider-example)
- [Hybrid Provider Example](#hybrid-provider-example)
- [Provider Testing](#provider-testing)
- [Performance Optimization](#performance-optimization)
- [Error Handling](#error-handling)
- [Best Practices](#best-practices)

## Overview

Custom providers allow you to integrate the SCIM server with your existing storage infrastructure. The provider system is designed to be:

- **Flexible** - Support any storage backend
- **Async-First** - Non-blocking I/O operations
- **Type-Safe** - Rust's type system prevents runtime errors
- **Testable** - Easy to unit test and mock
- **Performant** - Optimized for high-throughput scenarios

### Provider Architecture

```
┌─────────────────────────────────────────────────────────┐
│                 SCIM Resource Handlers                  │
├─────────────────────────────────────────────────────────┤
│              ResourceProvider Trait                     │
│                 (Your Implementation)                   │
├─────────────────────────────────────────────────────────┤
│              Your Storage Backend                       │
│     (Database, API, File System, etc.)                  │
└─────────────────────────────────────────────────────────┘
```

## Provider Interface

Every custom provider must implement the `ResourceProvider` trait:

```rust
use async_trait::async_trait;
use scim_server::providers::ResourceProvider;
use scim_server::resource::{Resource, ResourceType};
use scim_server::resource::value_objects::ResourceId;
use scim_server::search::{SearchQuery, SearchResult};
use scim_server::bulk::{BulkRequest, BulkResponse};
use scim_server::patch::PatchRequest;
use scim_server::error::Result;

#[async_trait]
pub trait ResourceProvider: Send + Sync + Clone {
    /// Create a new resource
    async fn create_resource(&self, resource: Resource) -> Result<Resource>;
    
    /// Retrieve a resource by ID
    async fn get_resource(&self, id: &ResourceId) -> Result<Option<Resource>>;
    
    /// Update an existing resource (full replacement)
    async fn update_resource(&self, resource: Resource) -> Result<Resource>;
    
    /// Apply PATCH operations to a resource
    async fn patch_resource(&self, id: &ResourceId, patch: PatchRequest) -> Result<Resource>;
    
    /// Delete a resource
    async fn delete_resource(&self, id: &ResourceId) -> Result<()>;
    
    /// List resources of a specific type
    async fn list_resources(&self, resource_type: ResourceType) -> Result<Vec<Resource>>;
    
    /// Search resources with filtering, sorting, and pagination
    async fn search_resources(&self, query: &SearchQuery) -> Result<SearchResult>;
    
    /// Perform bulk operations
    async fn bulk_operations(&self, operations: BulkRequest) -> Result<BulkResponse>;
    
    /// Check provider health
    async fn health_check(&self) -> Result<HealthStatus>;
    
    /// Get provider statistics
    async fn get_statistics(&self) -> Result<ProviderStatistics>;
}
```

## Database Provider Example

This example shows how to implement a PostgreSQL-based provider with connection pooling and optimized queries.

### Dependencies

Add to your `Cargo.toml`:

```toml
[dependencies]
sqlx = { version = "0.7", features = ["runtime-tokio-rustls", "postgres", "uuid", "chrono", "json"] }
uuid = { version = "1.0", features = ["v4"] }
serde_json = "1.0"
```

### Implementation

```rust
use sqlx::{PgPool, Row};
use uuid::Uuid;
use std::collections::HashMap;
use serde_json::Value;

#[derive(Clone)]
pub struct PostgresProvider {
    pool: PgPool,
    table_prefix: String,
    tenant_id: Option<String>,
}

impl PostgresProvider {
    pub async fn new(database_url: &str) -> Result<Self> {
        let pool = PgPool::connect(database_url).await
            .map_err(|e| ScimError::provider_error("PostgreSQL", e.to_string()))?;
        
        let provider = Self {
            pool,
            table_prefix: "scim_".to_string(),
            tenant_id: None,
        };
        
        // Initialize database schema
        provider.initialize_schema().await?;
        
        Ok(provider)
    }
    
    pub fn with_tenant(mut self, tenant_id: String) -> Self {
        self.tenant_id = Some(tenant_id);
        self
    }
    
    async fn initialize_schema(&self) -> Result<()> {
        let resources_table = format!("{}resources", self.table_prefix);
        
        sqlx::query(&format!(r#"
            CREATE TABLE IF NOT EXISTS {} (
                id VARCHAR(255) PRIMARY KEY,
                resource_type VARCHAR(50) NOT NULL,
                tenant_id VARCHAR(255),
                version INTEGER NOT NULL DEFAULT 1,
                data JSONB NOT NULL,
                created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
                updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
                
                CONSTRAINT unique_tenant_resource UNIQUE (tenant_id, id)
            )
        "#, resources_table))
        .execute(&self.pool)
        .await
        .map_err(|e| ScimError::internal_error(format!("Failed to create table: {}", e)))?;
        
        // Create indexes for performance
        self.create_indexes().await?;
        
        Ok(())
    }
    
    async fn create_indexes(&self) -> Result<()> {
        let table_name = format!("{}resources", self.table_prefix);
        
        let indexes = vec![
            format!("CREATE INDEX IF NOT EXISTS idx_{}_resource_type ON {} (resource_type)", self.table_prefix, table_name),
            format!("CREATE INDEX IF NOT EXISTS idx_{}_tenant_id ON {} (tenant_id)", self.table_prefix, table_name),
            format!("CREATE INDEX IF NOT EXISTS idx_{}_username ON {} ((data->>'userName')) WHERE resource_type = 'User'", self.table_prefix, table_name),
            format!("CREATE INDEX IF NOT EXISTS idx_{}_external_id ON {} ((data->>'externalId'))", self.table_prefix, table_name),
            format!("CREATE INDEX IF NOT EXISTS idx_{}_created ON {} (created_at)", self.table_prefix, table_name),
            format!("CREATE INDEX IF NOT EXISTS idx_{}_data_gin ON {} USING GIN (data)", self.table_prefix, table_name),
        ];
        
        for index_sql in indexes {
            sqlx::query(&index_sql)
                .execute(&self.pool)
                .await
                .map_err(|e| ScimError::internal_error(format!("Failed to create index: {}", e)))?;
        }
        
        Ok(())
    }
    
    fn get_tenant_filter(&self) -> String {
        if let Some(tenant_id) = &self.tenant_id {
            format!("tenant_id = '{}'", tenant_id)
        } else {
            "tenant_id IS NULL".to_string()
        }
    }
}

#[async_trait]
impl ResourceProvider for PostgresProvider {
    async fn create_resource(&self, mut resource: Resource) -> Result<Resource> {
        let table_name = format!("{}resources", self.table_prefix);
        
        // Set metadata
        let now = chrono::Utc::now();
        resource.set_created(now);
        resource.set_last_modified(now);
        resource.set_version(1);
        
        // Serialize resource data
        let data = serde_json::to_value(&resource)
            .map_err(|e| ScimError::internal_error(format!("Serialization error: {}", e)))?;
        
        // Insert into database
        sqlx::query(&format!(r#"
            INSERT INTO {} (id, resource_type, tenant_id, data, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6)
        "#, table_name))
        .bind(resource.id().as_str())
        .bind(resource.resource_type().to_string())
        .bind(&self.tenant_id)
        .bind(&data)
        .bind(now)
        .bind(now)
        .execute(&self.pool)
        .await
        .map_err(|e| {
            if e.to_string().contains("unique constraint") {
                ScimError::Conflict {
                    message: format!("Resource with ID '{}' already exists", resource.id()),
                    existing_resource: Some(resource.id().to_string()),
                }
            } else {
                ScimError::provider_error("PostgreSQL", e.to_string())
            }
        })?;
        
        Ok(resource)
    }
    
    async fn get_resource(&self, id: &ResourceId) -> Result<Option<Resource>> {
        let table_name = format!("{}resources", self.table_prefix);
        let tenant_filter = self.get_tenant_filter();
        
        let row = sqlx::query(&format!(r#"
            SELECT data FROM {} WHERE id = $1 AND {}
        "#, table_name, tenant_filter))
        .bind(id.as_str())
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| ScimError::provider_error("PostgreSQL", e.to_string()))?;
        
        if let Some(row) = row {
            let data: Value = row.get("data");
            let resource: Resource = serde_json::from_value(data)
                .map_err(|e| ScimError::internal_error(format!("Deserialization error: {}", e)))?;
            Ok(Some(resource))
        } else {
            Ok(None)
        }
    }
    
    async fn update_resource(&self, mut resource: Resource) -> Result<Resource> {
        let table_name = format!("{}resources", self.table_prefix);
        let tenant_filter = self.get_tenant_filter();
        
        // Update metadata
        resource.set_last_modified(chrono::Utc::now());
        resource.increment_version();
        
        let data = serde_json::to_value(&resource)
            .map_err(|e| ScimError::internal_error(format!("Serialization error: {}", e)))?;
        
        let result = sqlx::query(&format!(r#"
            UPDATE {} 
            SET data = $1, updated_at = $2, version = version + 1
            WHERE id = $3 AND {}
            RETURNING version
        "#, table_name, tenant_filter))
        .bind(&data)
        .bind(chrono::Utc::now())
        .bind(resource.id().as_str())
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| ScimError::provider_error("PostgreSQL", e.to_string()))?;
        
        if result.is_none() {
            return Err(ScimError::NotFound {
                resource_type: resource.resource_type().to_string(),
                id: resource.id().to_string(),
            });
        }
        
        Ok(resource)
    }
    
    async fn delete_resource(&self, id: &ResourceId) -> Result<()> {
        let table_name = format!("{}resources", self.table_prefix);
        let tenant_filter = self.get_tenant_filter();
        
        let result = sqlx::query(&format!(r#"
            DELETE FROM {} WHERE id = $1 AND {}
        "#, table_name, tenant_filter))
        .bind(id.as_str())
        .execute(&self.pool)
        .await
        .map_err(|e| ScimError::provider_error("PostgreSQL", e.to_string()))?;
        
        if result.rows_affected() == 0 {
            return Err(ScimError::NotFound {
                resource_type: "Resource".to_string(),
                id: id.to_string(),
            });
        }
        
        Ok(())
    }
    
    async fn list_resources(&self, resource_type: ResourceType) -> Result<Vec<Resource>> {
        let table_name = format!("{}resources", self.table_prefix);
        let tenant_filter = self.get_tenant_filter();
        
        let rows = sqlx::query(&format!(r#"
            SELECT data FROM {} 
            WHERE resource_type = $1 AND {}
            ORDER BY created_at DESC
        "#, table_name, tenant_filter))
        .bind(resource_type.to_string())
        .fetch_all(&self.pool)
        .await
        .map_err(|e| ScimError::provider_error("PostgreSQL", e.to_string()))?;
        
        let resources = rows.into_iter()
            .map(|row| {
                let data: Value = row.get("data");
                serde_json::from_value(data)
            })
            .collect::<Result<Vec<Resource>, _>>()
            .map_err(|e| ScimError::internal_error(format!("Deserialization error: {}", e)))?;
        
        Ok(resources)
    }
    
    async fn search_resources(&self, query: &SearchQuery) -> Result<SearchResult> {
        let table_name = format!("{}resources", self.table_prefix);
        let tenant_filter = self.get_tenant_filter();
        
        // Build WHERE clause from filter
        let (filter_sql, params) = if let Some(filter) = &query.filter {
            self.filter_to_sql(filter)?
        } else {
            ("TRUE".to_string(), Vec::new())
        };
        
        // Build ORDER BY clause
        let order_by = if let Some(sort_by) = &query.sort_by {
            let direction = match query.sort_order {
                Some(SortOrder::Descending) => "DESC",
                _ => "ASC",
            };
            format!("ORDER BY data->>'{}' {}", sort_by, direction)
        } else {
            "ORDER BY created_at DESC".to_string()
        };
        
        // Calculate pagination
        let offset = query.start_index.saturating_sub(1);
        let limit = query.count;
        
        // Count total results
        let count_sql = format!(r#"
            SELECT COUNT(*) as total
            FROM {} 
            WHERE {} AND ({})
        "#, table_name, tenant_filter, filter_sql);
        
        let total_results: i64 = sqlx::query(&count_sql)
            .bind_all(&params)
            .fetch_one(&self.pool)
            .await
            .map_err(|e| ScimError::provider_error("PostgreSQL", e.to_string()))?
            .get("total");
        
        // Fetch paginated results
        let search_sql = format!(r#"
            SELECT data FROM {}
            WHERE {} AND ({})
            {}
            LIMIT $1 OFFSET $2
        "#, table_name, tenant_filter, filter_sql, order_by);
        
        let mut query_builder = sqlx::query(&search_sql);
        for param in params {
            query_builder = query_builder.bind(param);
        }
        query_builder = query_builder.bind(limit as i64).bind(offset as i64);
        
        let rows = query_builder
            .fetch_all(&self.pool)
            .await
            .map_err(|e| ScimError::provider_error("PostgreSQL", e.to_string()))?;
        
        let resources = rows.into_iter()
            .map(|row| {
                let data: Value = row.get("data");
                serde_json::from_value(data)
            })
            .collect::<Result<Vec<Resource>, _>>()
            .map_err(|e| ScimError::internal_error(format!("Deserialization error: {}", e)))?;
        
        Ok(SearchResult {
            resources,
            total_results: total_results as usize,
            start_index: query.start_index,
            items_per_page: resources.len(),
        })
    }
    
    async fn health_check(&self) -> Result<HealthStatus> {
        match sqlx::query("SELECT 1").execute(&self.pool).await {
            Ok(_) => Ok(HealthStatus::healthy()),
            Err(e) => Ok(HealthStatus::unhealthy(&format!("Database error: {}", e))),
        }
    }
    
    async fn get_statistics(&self) -> Result<ProviderStatistics> {
        let table_name = format!("{}resources", self.table_prefix);
        let tenant_filter = self.get_tenant_filter();
        
        let stats_row = sqlx::query(&format!(r#"
            SELECT 
                COUNT(*) as total_resources,
                COUNT(CASE WHEN resource_type = 'User' THEN 1 END) as user_count,
                COUNT(CASE WHEN resource_type = 'Group' THEN 1 END) as group_count,
                MAX(created_at) as last_created,
                MAX(updated_at) as last_updated
            FROM {} WHERE {}
        "#, table_name, tenant_filter))
        .fetch_one(&self.pool)
        .await
        .map_err(|e| ScimError::provider_error("PostgreSQL", e.to_string()))?;
        
        Ok(ProviderStatistics {
            total_resources: stats_row.get::<i64, _>("total_resources") as usize,
            user_count: stats_row.get::<i64, _>("user_count") as usize,
            group_count: stats_row.get::<i64, _>("group_count") as usize,
            last_created: stats_row.get("last_created"),
            last_updated: stats_row.get("last_updated"),
            provider_type: "PostgreSQL".to_string(),
        })
    }
    
    // Helper method to convert SCIM filters to SQL
    fn filter_to_sql(&self, filter: &FilterExpression) -> Result<(String, Vec<String>)> {
        match filter {
            FilterExpression::Equality { attribute, value } => {
                let sql = format!("data->>'{}' = $1", attribute);
                Ok((sql, vec![value.clone()]))
            }
            FilterExpression::Contains { attribute, value } => {
                let sql = format!("data->>'{}' ILIKE $1", attribute);
                Ok((sql, vec![format!("%{}%", value)]))
            }
            FilterExpression::StartsWith { attribute, value } => {
                let sql = format!("data->>'{}' ILIKE $1", attribute);
                Ok((sql, vec![format!("{}%", value)]))
            }
            FilterExpression::Present { attribute } => {
                let sql = format!("data ? '{}'", attribute);
                Ok((sql, Vec::new()))
            }
            FilterExpression::And { left, right } => {
                let (left_sql, mut left_params) = self.filter_to_sql(left)?;
                let (right_sql, mut right_params) = self.filter_to_sql(right)?;
                
                // Adjust parameter placeholders for right side
                let right_sql_adjusted = self.adjust_parameter_placeholders(&right_sql, left_params.len());
                
                left_params.append(&mut right_params);
                Ok((format!("({}) AND ({})", left_sql, right_sql_adjusted), left_params))
            }
            FilterExpression::Or { left, right } => {
                let (left_sql, mut left_params) = self.filter_to_sql(left)?;
                let (right_sql, mut right_params) = self.filter_to_sql(right)?;
                
                let right_sql_adjusted = self.adjust_parameter_placeholders(&right_sql, left_params.len());
                
                left_params.append(&mut right_params);
                Ok((format!("({}) OR ({})", left_sql, right_sql_adjusted), left_params))
            }
            _ => Err(ScimError::invalid_filter(
                filter.to_string(),
                "Filter operation not supported by PostgreSQL provider"
            )),
        }
    }
    
    fn adjust_parameter_placeholders(&self, sql: &str, offset: usize) -> String {
        let mut result = sql.to_string();
        for i in (1..=10).rev() { // Adjust up to 10 parameters
            let old_placeholder = format!("${}", i);
            let new_placeholder = format!("${}", i + offset);
            result = result.replace(&old_placeholder, &new_placeholder);
        }
        result
    }
}
```

## External API Provider Example

This example shows how to create a provider that proxies requests to an external REST API:

```rust
use reqwest::{Client, header};
use std::time::Duration;

#[derive(Clone)]
pub struct ExternalApiProvider {
    client: Client,
    base_url: String,
    api_key: String,
    timeout: Duration,
    cache: Arc<RwLock<HashMap<ResourceId, (Resource, Instant)>>>,
    cache_ttl: Duration,
}

impl ExternalApiProvider {
    pub fn new(base_url: String, api_key: String) -> Result<Self> {
        let mut headers = header::HeaderMap::new();
        headers.insert(
            header::AUTHORIZATION,
            header::HeaderValue::from_str(&format!("Bearer {}", api_key))
                .map_err(|e| ScimError::bad_request(format!("Invalid API key format: {}", e)))?
        );
        headers.insert(
            header::CONTENT_TYPE,
            header::HeaderValue::from_static("application/scim+json")
        );
        
        let client = Client::builder()
            .default_headers(headers)
            .timeout(Duration::from_secs(30))
            .build()
            .map_err(|e| ScimError::internal_error(format!("Failed to create HTTP client: {}", e)))?;
        
        Ok(Self {
            client,
            base_url,
            api_key,
            timeout: Duration::from_secs(30),
            cache: Arc::new(RwLock::new(HashMap::new())),
            cache_ttl: Duration::from_secs(300), // 5 minutes
        })
    }
    
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }
    
    pub fn with_cache_ttl(mut self, ttl: Duration) -> Self {
        self.cache_ttl = ttl;
        self
    }
    
    async fn make_request<T>(&self, method: &str, path: &str, body: Option<T>) -> Result<Value>
    where
        T: serde::Serialize,
    {
        let url = format!("{}/scim/v2/{}", self.base_url, path);
        
        let mut request = self.client
            .request(method.parse().unwrap(), &url)
            .timeout(self.timeout);
        
        if let Some(body) = body {
            request = request.json(&body);
        }
        
        let response = request.send().await
            .map_err(|e| {
                if e.is_timeout() {
                    ScimError::ServiceUnavailable {
                        message: "External API timeout".to_string(),
                        retry_after: Some(Duration::from_secs(30)),
                    }
                } else if e.is_connect() {
                    ScimError::ServiceUnavailable {
                        message: "Cannot connect to external API".to_string(),
                        retry_after: Some(Duration::from_secs(60)),
                    }
                } else {
                    ScimError::provider_error("External API", e.to_string())
                }
            })?;
        
        let status = response.status();
        if !status.is_success() {
            return Err(self.map_http_error(status, response).await?);
        }
        
        response.json().await
            .map_err(|e| ScimError::provider_error("External API", format!("Invalid JSON response: {}", e)))
    }
    
    async fn map_http_error(&self, status: reqwest::StatusCode, response: reqwest::Response) -> Result<ScimError> {
        let error_body = response.text().await.unwrap_or_default();
        
        let error = match status {
            reqwest::StatusCode::NOT_FOUND => ScimError::NotFound {
                resource_type: "Resource".to_string(),
                id: "unknown".to_string(),
            },
            reqwest::StatusCode::CONFLICT => ScimError::Conflict {
                message: "Resource conflict in external system".to_string(),
                existing_resource: None,
            },
            reqwest::StatusCode::UNAUTHORIZED => ScimError::Unauthorized {
                realm: Some("External API".to_string()),
                message: "External API authentication failed".to_string(),
            },
            reqwest::StatusCode::FORBIDDEN => ScimError::Forbidden {
                resource: None,
                action: None,
                message: "External API access denied".to_string(),
            },
            reqwest::StatusCode::TOO_MANY_REQUESTS => ScimError::ServiceUnavailable {
                message: "External API rate limit exceeded".to_string(),
                retry_after: Some(Duration::from_secs(60)),
            },
            _ => ScimError::provider_error("External API", format!("HTTP {}: {}", status, error_body)),
        };
        
        Ok(error)
    }
    
    async fn get_cached(&self, id: &ResourceId) -> Option<Resource> {
        let cache = self.cache.read().await;
        if let Some((resource, cached_at)) = cache.get(id) {
            if cached_at.elapsed() < self.cache_ttl {
                return Some(resource.clone());
            }
        }
        None
    }
    
    async fn set_cache(&self, id: ResourceId, resource: Resource) {
        let mut cache = self.cache.write().await;
        cache.insert(id, (resource, Instant::now()));
        
        // Clean up expired entries
        cache.retain(|_, (_, cached_at)| cached_at.elapsed() < self.cache_ttl);
    }
}

#[async_trait]
impl ResourceProvider for ExternalApiProvider {
    async fn create_resource(&self, resource: Resource) -> Result<Resource> {
        let resource_type = resource.resource_type().to_string();
        let path = format!("{}s", resource_type); // Users, Groups, etc.
        
        let response_data = self.make_request("POST", &path, Some(&resource)).await?;
        let created_resource: Resource = serde_json::from_value(response_data)
            .map_err(|e| ScimError::internal_error(format!("Invalid resource in response: {}", e)))?;
        
        // Cache the created resource
        self.set_cache(created_resource.id().clone(), created_resource.clone()).await;
        
        Ok(created_resource)
    }
    
    async fn get_resource(&self, id: &ResourceId) -> Result<Option<Resource>> {
        // Check cache first
        if let Some(cached) = self.get_cached(id).await {
            return Ok(Some(cached));
        }
        
        // Make API request
        let path = format!("Resources/{}", id.as_str());
        
        match self.make_request::<()>("GET", &path, None).await {
            Ok(data) => {
                let resource: Resource = serde_json::from_value(data)
                    .map_err(|e| ScimError::internal_error(format!("Invalid resource in response: {}", e)))?;
                
                // Cache the resource
                self.set_cache(id.clone(), resource.clone()).await;
                
                Ok(Some(resource))
            }
            Err(ScimError::NotFound { .. }) => Ok(None),
            Err(e) => Err(e),
        }
    }
    
    async fn update_resource(&self, resource: Resource) -> Result<Resource> {
        let path = format!("Resources/{}", resource.id().as_str());
        
        let response_data = self.make_request("PUT", &path, Some(&resource)).await?;
        let updated_resource: Resource = serde_json::from_value(response_data)
            .map_err(|e| ScimError::internal_error(format!("Invalid resource in response: {}", e)))?;
        
        // Update cache
        self.set_cache(updated_resource.id().clone(), updated_resource.clone()).await;
        
        Ok(updated_resource)
    }
    
    async fn delete_resource(&self, id: &ResourceId) -> Result<()> {
        let path = format!("Resources/{}", id.as_str());
        self.make_request::<()>("DELETE", &path, None).await?;
        
        // Remove from cache
        let mut cache = self.cache.write().await;
        cache.remove(id);
        
        Ok(())
    }
    
    async fn list_resources(&self, resource_type: ResourceType) -> Result<Vec<Resource>> {
        let path = format!("{}s", resource_type.to_string());
        
        let response_data = self.make_request::<()>("GET", &path, None).await?;
        
        let list_response: ScimListResponse = serde_json::from_value(response_data)
            .map_err(|e| ScimError::internal_error(format!("Invalid list response: {}", e)))?;
        
        // Cache all resources
        for resource in &list_response.resources {
            self.set_cache(resource.id().clone(), resource.clone()).await;
        }
        
        Ok(list_response.resources)
    }
    
    async fn search_resources(&self, query: &SearchQuery) -> Result<SearchResult> {
        // Convert SearchQuery to external API format
        let mut params = Vec::new();
        
        if let Some(filter) = &query.filter {
            params.push(("filter", filter.to_string()));
        }
        
        if let Some(sort_by) = &query.sort_by {
            params.push(("sortBy", sort_by.clone()));
            if let Some(sort_order) = &query.sort_order {
                params.push(("sortOrder", sort_order.to_string()));
            }
        }
        
        params.push(("startIndex", query.start_index.to_string()));
        params.push(("count", query.count.to_string()));
        
        let query_string = params.iter()
            .map(|(k, v)| format!("{}={}", k, urlencoding::encode(v)))
            .collect::<Vec<_>>()
            .join("&");
        
        let path = format!("Resources?{}", query_string);
        
        let response_data = self.make_request::<()>("GET", &path, None).await?;
        
        let search_response: ScimSearchResponse = serde_json::from_value(response_data)
            .map_err(|e| ScimError::internal_error(format!("Invalid search response: {}", e)))?;
        
        Ok(SearchResult {
            resources: search_response.resources,
            total_results: search_response.total_results,
            start_index: search_response.start_index,
            items_per_page: search_