# Advanced Features Examples

This document demonstrates advanced features and patterns available in the SCIM server crate, including complex resource operations, custom validation, multi-tenancy, and performance optimizations.

## Table of Contents

- [Custom Resource Providers](#custom-resource-providers)
- [Advanced Multi-Tenancy](#advanced-multi-tenancy)
- [Complex Filtering and Sorting](#complex-filtering-and-sorting)
- [Bulk Operations](#bulk-operations)
- [Custom Validation Logic](#custom-validation-logic)
- [Resource Extensions](#resource-extensions)
- [Performance Optimizations](#performance-optimizations)
- [Advanced Error Handling](#advanced-error-handling)
- [Middleware Integration](#middleware-integration)
- [Custom Schema Loading](#custom-schema-loading)

## Custom Resource Providers

### Database-Backed Provider with Connection Pooling

```rust
use scim_server::providers::ResourceProvider;
use scim_server::resource::{Resource, ResourceId};
use scim_server::error::ScimError;
use sqlx::{PgPool, Row};
use async_trait::async_trait;
use serde_json::Value;
use std::collections::HashMap;

pub struct PostgresProvider {
    pool: PgPool,
    table_name: String,
}

impl PostgresProvider {
    pub async fn new(database_url: &str, table_name: String) -> Result<Self, sqlx::Error> {
        let pool = PgPool::connect(database_url).await?;
        Ok(Self { pool, table_name })
    }
}

#[async_trait]
impl ResourceProvider for PostgresProvider {
    async fn create(&self, resource: Resource) -> Result<Resource, ScimError> {
        let id = ResourceId::new();
        let json_data = serde_json::to_value(&resource)?;
        
        sqlx::query(&format!(
            "INSERT INTO {} (id, data, created_at, updated_at) VALUES ($1, $2, NOW(), NOW())",
            self.table_name
        ))
        .bind(id.as_str())
        .bind(&json_data)
        .execute(&self.pool)
        .await
        .map_err(|e| ScimError::internal_server_error(&format!("Database error: {}", e)))?;
        
        let mut created = resource;
        created.id = Some(id);
        created.meta.as_mut().map(|m| {
            m.created = Some(chrono::Utc::now());
            m.last_modified = Some(chrono::Utc::now());
        });
        
        Ok(created)
    }

    async fn get_by_id(&self, id: &ResourceId) -> Result<Option<Resource>, ScimError> {
        let row = sqlx::query(&format!(
            "SELECT data FROM {} WHERE id = $1",
            self.table_name
        ))
        .bind(id.as_str())
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| ScimError::internal_server_error(&format!("Database error: {}", e)))?;
        
        match row {
            Some(row) => {
                let data: Value = row.get("data");
                let resource: Resource = serde_json::from_value(data)?;
                Ok(Some(resource))
            }
            None => Ok(None),
        }
    }

    async fn update(&self, id: &ResourceId, resource: Resource) -> Result<Resource, ScimError> {
        let json_data = serde_json::to_value(&resource)?;
        
        let result = sqlx::query(&format!(
            "UPDATE {} SET data = $1, updated_at = NOW() WHERE id = $2",
            self.table_name
        ))
        .bind(&json_data)
        .bind(id.as_str())
        .execute(&self.pool)
        .await
        .map_err(|e| ScimError::internal_server_error(&format!("Database error: {}", e)))?;
        
        if result.rows_affected() == 0 {
            return Err(ScimError::not_found(&format!("Resource {} not found", id)));
        }
        
        let mut updated = resource;
        updated.meta.as_mut().map(|m| m.last_modified = Some(chrono::Utc::now()));
        Ok(updated)
    }

    async fn delete(&self, id: &ResourceId) -> Result<(), ScimError> {
        let result = sqlx::query(&format!("DELETE FROM {} WHERE id = $1", self.table_name))
            .bind(id.as_str())
            .execute(&self.pool)
            .await
            .map_err(|e| ScimError::internal_server_error(&format!("Database error: {}", e)))?;
        
        if result.rows_affected() == 0 {
            Err(ScimError::not_found(&format!("Resource {} not found", id)))
        } else {
            Ok(())
        }
    }

    async fn list(&self, filter: Option<&str>, start_index: usize, count: usize) 
        -> Result<(Vec<Resource>, usize), ScimError> {
        
        let offset = start_index.saturating_sub(1);
        
        // Simple filter implementation - in production, you'd want more sophisticated parsing
        let (where_clause, params) = if let Some(filter) = filter {
            if filter.contains("userName eq ") {
                let username = filter.split("userName eq ").nth(1)
                    .and_then(|s| s.trim_matches('"').split_whitespace().next())
                    .unwrap_or("");
                ("WHERE data->>'userName' = $3", vec![username])
            } else {
                ("", vec![])
            }
        } else {
            ("", vec![])
        };
        
        let query = format!(
            "SELECT data FROM {} {} ORDER BY id LIMIT $1 OFFSET $2",
            self.table_name, where_clause
        );
        
        let mut query_builder = sqlx::query(&query)
            .bind(count as i64)
            .bind(offset as i64);
            
        for param in params {
            query_builder = query_builder.bind(param);
        }
        
        let rows = query_builder
            .fetch_all(&self.pool)
            .await
            .map_err(|e| ScimError::internal_server_error(&format!("Database error: {}", e)))?;
        
        let resources: Result<Vec<Resource>, _> = rows
            .into_iter()
            .map(|row| {
                let data: Value = row.get("data");
                serde_json::from_value(data)
            })
            .collect();
        
        let resources = resources?;
        
        // Get total count
        let count_query = format!("SELECT COUNT(*) FROM {} {}", self.table_name, where_clause);
        let total_count: i64 = sqlx::query_scalar(&count_query)
            .fetch_one(&self.pool)
            .await
            .map_err(|e| ScimError::internal_server_error(&format!("Database error: {}", e)))?;
        
        Ok((resources, total_count as usize))
    }
}
```

## Advanced Multi-Tenancy

### Dynamic Tenant Resolution with Caching

```rust
use scim_server::multi_tenant::{TenantResolver, TenantContext};
use scim_server::error::ScimError;
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use lru::LruCache;

pub struct CachingTenantResolver {
    cache: Arc<RwLock<LruCache<String, TenantContext>>>,
    database_resolver: DatabaseTenantResolver,
}

impl CachingTenantResolver {
    pub fn new(database_resolver: DatabaseTenantResolver, cache_size: usize) -> Self {
        Self {
            cache: Arc::new(RwLock::new(LruCache::new(cache_size.try_into().unwrap()))),
            database_resolver,
        }
    }
    
    async fn get_from_cache(&self, tenant_id: &str) -> Option<TenantContext> {
        let mut cache = self.cache.write().await;
        cache.get(tenant_id).cloned()
    }
    
    async fn cache_tenant(&self, tenant_id: String, context: TenantContext) {
        let mut cache = self.cache.write().await;
        cache.put(tenant_id, context);
    }
}

#[async_trait]
impl TenantResolver for CachingTenantResolver {
    async fn resolve_tenant(&self, tenant_id: &str) -> Result<TenantContext, ScimError> {
        // Check cache first
        if let Some(context) = self.get_from_cache(tenant_id).await {
            return Ok(context);
        }
        
        // Fallback to database
        let context = self.database_resolver.resolve_tenant(tenant_id).await?;
        
        // Cache the result
        self.cache_tenant(tenant_id.to_string(), context.clone()).await;
        
        Ok(context)
    }
}

pub struct DatabaseTenantResolver {
    pool: PgPool,
}

#[async_trait]
impl TenantResolver for DatabaseTenantResolver {
    async fn resolve_tenant(&self, tenant_id: &str) -> Result<TenantContext, ScimError> {
        let row = sqlx::query(
            "SELECT config, schema_overrides FROM tenants WHERE tenant_id = $1"
        )
        .bind(tenant_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| ScimError::internal_server_error(&format!("Database error: {}", e)))?;
        
        match row {
            Some(row) => {
                let config: Value = row.get("config");
                let schema_overrides: Option<Value> = row.get("schema_overrides");
                
                Ok(TenantContext {
                    tenant_id: tenant_id.to_string(),
                    config,
                    schema_overrides,
                })
            }
            None => Err(ScimError::not_found(&format!("Tenant {} not found", tenant_id))),
        }
    }
}
```

## Complex Filtering and Sorting

### Advanced Filter Implementation

```rust
use scim_server::providers::ResourceProvider;
use scim_server::resource::Resource;
use scim_server::error::ScimError;
use serde_json::Value;

pub struct AdvancedFilterProvider {
    base_provider: Box<dyn ResourceProvider>,
}

impl AdvancedFilterProvider {
    pub fn new(base_provider: Box<dyn ResourceProvider>) -> Self {
        Self { base_provider }
    }
    
    fn apply_complex_filter(&self, resources: Vec<Resource>, filter: &str) -> Result<Vec<Resource>, ScimError> {
        // Parse complex filter expressions
        let parsed_filter = self.parse_filter(filter)?;
        
        let filtered: Vec<Resource> = resources
            .into_iter()
            .filter(|resource| self.evaluate_filter(resource, &parsed_filter))
            .collect();
            
        Ok(filtered)
    }
    
    fn parse_filter(&self, filter: &str) -> Result<FilterExpression, ScimError> {
        // Simplified filter parser - in production, use a proper parser like nom
        if filter.contains(" and ") {
            let parts: Vec<&str> = filter.split(" and ").collect();
            let conditions = parts
                .into_iter()
                .map(|part| self.parse_simple_condition(part))
                .collect::<Result<Vec<_>, _>>()?;
            Ok(FilterExpression::And(conditions))
        } else if filter.contains(" or ") {
            let parts: Vec<&str> = filter.split(" or ").collect();
            let conditions = parts
                .into_iter()
                .map(|part| self.parse_simple_condition(part))
                .collect::<Result<Vec<_>, _>>()?;
            Ok(FilterExpression::Or(conditions))
        } else {
            Ok(FilterExpression::Simple(self.parse_simple_condition(filter)?))
        }
    }
    
    fn parse_simple_condition(&self, condition: &str) -> Result<SimpleCondition, ScimError> {
        let parts: Vec<&str> = condition.trim().split_whitespace().collect();
        if parts.len() != 3 {
            return Err(ScimError::invalid_filter("Invalid filter format"));
        }
        
        let attribute = parts[0].to_string();
        let operator = match parts[1] {
            "eq" => FilterOperator::Equal,
            "ne" => FilterOperator::NotEqual,
            "co" => FilterOperator::Contains,
            "sw" => FilterOperator::StartsWith,
            "ew" => FilterOperator::EndsWith,
            "gt" => FilterOperator::GreaterThan,
            "ge" => FilterOperator::GreaterEqual,
            "lt" => FilterOperator::LessThan,
            "le" => FilterOperator::LessEqual,
            _ => return Err(ScimError::invalid_filter(&format!("Unknown operator: {}", parts[1]))),
        };
        let value = parts[2].trim_matches('"').to_string();
        
        Ok(SimpleCondition { attribute, operator, value })
    }
    
    fn evaluate_filter(&self, resource: &Resource, filter: &FilterExpression) -> bool {
        match filter {
            FilterExpression::Simple(condition) => self.evaluate_simple_condition(resource, condition),
            FilterExpression::And(conditions) => {
                conditions.iter().all(|c| self.evaluate_simple_condition(resource, c))
            }
            FilterExpression::Or(conditions) => {
                conditions.iter().any(|c| self.evaluate_simple_condition(resource, c))
            }
        }
    }
    
    fn evaluate_simple_condition(&self, resource: &Resource, condition: &SimpleCondition) -> bool {
        let resource_value = self.get_attribute_value(resource, &condition.attribute);
        
        match resource_value {
            Some(value) => self.compare_values(value, &condition.operator, &condition.value),
            None => false,
        }
    }
    
    fn get_attribute_value(&self, resource: &Resource, path: &str) -> Option<&Value> {
        // Handle dot notation for nested attributes
        let parts: Vec<&str> = path.split('.').collect();
        let mut current = &resource.attributes;
        
        for part in parts {
            current = current.get(part)?;
        }
        
        Some(current)
    }
    
    fn compare_values(&self, resource_val: &Value, op: &FilterOperator, filter_val: &str) -> bool {
        match op {
            FilterOperator::Equal => self.values_equal(resource_val, filter_val),
            FilterOperator::NotEqual => !self.values_equal(resource_val, filter_val),
            FilterOperator::Contains => self.value_contains(resource_val, filter_val),
            FilterOperator::StartsWith => self.value_starts_with(resource_val, filter_val),
            FilterOperator::EndsWith => self.value_ends_with(resource_val, filter_val),
            FilterOperator::GreaterThan => self.value_greater_than(resource_val, filter_val),
            FilterOperator::GreaterEqual => self.value_greater_equal(resource_val, filter_val),
            FilterOperator::LessThan => self.value_less_than(resource_val, filter_val),
            FilterOperator::LessEqual => self.value_less_equal(resource_val, filter_val),
        }
    }
    
    fn values_equal(&self, resource_val: &Value, filter_val: &str) -> bool {
        match resource_val {
            Value::String(s) => s == filter_val,
            Value::Bool(b) => filter_val == &b.to_string(),
            Value::Number(n) => filter_val.parse::<f64>().map_or(false, |f| *n == f.into()),
            _ => false,
        }
    }
    
    fn value_contains(&self, resource_val: &Value, filter_val: &str) -> bool {
        if let Value::String(s) = resource_val {
            s.to_lowercase().contains(&filter_val.to_lowercase())
        } else {
            false
        }
    }
    
    // Additional comparison methods...
}

#[derive(Debug, Clone)]
enum FilterExpression {
    Simple(SimpleCondition),
    And(Vec<SimpleCondition>),
    Or(Vec<SimpleCondition>),
}

#[derive(Debug, Clone)]
struct SimpleCondition {
    attribute: String,
    operator: FilterOperator,
    value: String,
}

#[derive(Debug, Clone)]
enum FilterOperator {
    Equal, NotEqual, Contains, StartsWith, EndsWith,
    GreaterThan, GreaterEqual, LessThan, LessEqual,
}
```

## Advanced Multi-Tenancy

### Hierarchical Tenant Management

```rust
use scim_server::multi_tenant::{TenantResolver, TenantContext};
use scim_server::error::ScimError;
use async_trait::async_trait;
use std::collections::HashMap;

pub struct HierarchicalTenantResolver {
    tenant_hierarchy: HashMap<String, TenantHierarchy>,
    config_inheritance: ConfigInheritance,
}

#[derive(Clone)]
pub struct TenantHierarchy {
    pub tenant_id: String,
    pub parent_id: Option<String>,
    pub children: Vec<String>,
    pub config: Value,
    pub inherited_config: Value,
}

pub struct ConfigInheritance {
    pub inherit_schemas: bool,
    pub inherit_providers: bool,
    pub inherit_auth: bool,
}

impl HierarchicalTenantResolver {
    pub fn new() -> Self {
        Self {
            tenant_hierarchy: HashMap::new(),
            config_inheritance: ConfigInheritance {
                inherit_schemas: true,
                inherit_providers: false,
                inherit_auth: true,
            },
        }
    }
    
    pub fn add_tenant(&mut self, tenant: TenantHierarchy) -> Result<(), ScimError> {
        // Validate parent exists if specified
        if let Some(parent_id) = &tenant.parent_id {
            if !self.tenant_hierarchy.contains_key(parent_id) {
                return Err(ScimError::invalid_value(&format!(
                    "Parent tenant {} does not exist", parent_id
                )));
            }
        }
        
        // Build inherited configuration
        let inherited_config = self.build_inherited_config(&tenant)?;
        let mut tenant_with_inheritance = tenant;
        tenant_with_inheritance.inherited_config = inherited_config;
        
        self.tenant_hierarchy.insert(tenant_with_inheritance.tenant_id.clone(), tenant_with_inheritance);
        Ok(())
    }
    
    fn build_inherited_config(&self, tenant: &TenantHierarchy) -> Result<Value, ScimError> {
        let mut config = tenant.config.clone();
        
        if let Some(parent_id) = &tenant.parent_id {
            if let Some(parent) = self.tenant_hierarchy.get(parent_id) {
                config = self.merge_configs(&parent.inherited_config, &config)?;
            }
        }
        
        Ok(config)
    }
    
    fn merge_configs(&self, parent_config: &Value, child_config: &Value) -> Result<Value, ScimError> {
        if let (Value::Object(parent), Value::Object(child)) = (parent_config, child_config) {
            let mut merged = parent.clone();
            
            for (key, value) in child {
                if self.should_inherit_config_key(key) {
                    if let (Some(Value::Object(parent_obj)), Value::Object(child_obj)) = 
                        (merged.get(key), value) {
                        // Recursively merge objects
                        let merged_obj = self.merge_configs(
                            &Value::Object(parent_obj.clone()), 
                            &Value::Object(child_obj.clone())
                        )?;
                        merged.insert(key.clone(), merged_obj);
                    } else {
                        merged.insert(key.clone(), value.clone());
                    }
                } else {
                    merged.insert(key.clone(), value.clone());
                }
            }
            
            Ok(Value::Object(merged))
        } else {
            Ok(child_config.clone())
        }
    }
    
    fn should_inherit_config_key(&self, key: &str) -> bool {
        match key {
            "schemas" => self.config_inheritance.inherit_schemas,
            "providers" => self.config_inheritance.inherit_providers,
            "authentication" => self.config_inheritance.inherit_auth,
            _ => true,
        }
    }
}

#[async_trait]
impl TenantResolver for HierarchicalTenantResolver {
    async fn resolve_tenant(&self, tenant_id: &str) -> Result<TenantContext, ScimError> {
        match self.tenant_hierarchy.get(tenant_id) {
            Some(hierarchy) => Ok(TenantContext {
                tenant_id: tenant_id.to_string(),
                config: hierarchy.inherited_config.clone(),
                schema_overrides: None,
            }),
            None => Err(ScimError::not_found(&format!("Tenant {} not found", tenant_id))),
        }
    }
}
```

## Complex Filtering and Sorting

### Advanced Query Builder

```rust
use scim_server::resource::Resource;
use scim_server::error::ScimError;
use serde_json::Value;
use std::cmp::Ordering;

pub struct QueryBuilder {
    filters: Vec<FilterClause>,
    sort_by: Option<String>,
    sort_order: SortOrder,
    limit: Option<usize>,
    offset: Option<usize>,
}

#[derive(Debug, Clone)]
pub struct FilterClause {
    pub path: String,
    pub operator: FilterOperator,
    pub value: FilterValue,
    pub logical_op: Option<LogicalOperator>,
}

#[derive(Debug, Clone)]
pub enum FilterValue {
    String(String),
    Number(f64),
    Boolean(bool),
    Array(Vec<FilterValue>),
}

#[derive(Debug, Clone)]
pub enum LogicalOperator {
    And,
    Or,
    Not,
}

#[derive(Debug, Clone)]
pub enum SortOrder {
    Ascending,
    Descending,
}

impl QueryBuilder {
    pub fn new() -> Self {
        Self {
            filters: Vec::new(),
            sort_by: None,
            sort_order: SortOrder::Ascending,
            limit: None,
            offset: None,
        }
    }
    
    pub fn filter(mut self, path: &str, operator: FilterOperator, value: FilterValue) -> Self {
        self.filters.push(FilterClause {
            path: path.to_string(),
            operator,
            value,
            logical_op: None,
        });
        self
    }
    
    pub fn and_filter(mut self, path: &str, operator: FilterOperator, value: FilterValue) -> Self {
        self.filters.push(FilterClause {
            path: path.to_string(),
            operator,
            value,
            logical_op: Some(LogicalOperator::And),
        });
        self
    }
    
    pub fn or_filter(mut self, path: &str, operator: FilterOperator, value: FilterValue) -> Self {
        self.filters.push(FilterClause {
            path: path.to_string(),
            operator,
            value,
            logical_op: Some(LogicalOperator::Or),
        });
        self
    }
    
    pub fn sort_by(mut self, attribute: &str, order: SortOrder) -> Self {
        self.sort_by = Some(attribute.to_string());
        self.sort_order = order;
        self
    }
    
    pub fn limit(mut self, limit: usize) -> Self {
        self.limit = Some(limit);
        self
    }
    
    pub fn offset(mut self, offset: usize) -> Self {
        self.offset = Some(offset);
        self
    }
    
    pub fn execute(&self, resources: Vec<Resource>) -> Result<Vec<Resource>, ScimError> {
        let mut result = resources;
        
        // Apply filters
        if !self.filters.is_empty() {
            result = self.apply_filters(result)?;
        }
        
        // Apply sorting
        if let Some(sort_attr) = &self.sort_by {
            result.sort_by(|a, b| self.compare_resources(a, b, sort_attr));
            if matches!(self.sort_order, SortOrder::Descending) {
                result.reverse();
            }
        }
        
        // Apply pagination
        if let Some(offset) = self.offset {
            result = result.into_iter().skip(offset).collect();
        }
        
        if let Some(limit) = self.limit {
            result = result.into_iter().take(limit).collect();
        }
        
        Ok(result)
    }
    
    fn apply_filters(&self, resources: Vec<Resource>) -> Result<Vec<Resource>, ScimError> {
        resources
            .into_iter()
            .filter(|resource| self.evaluate_all_filters(resource))
            .collect::<Result<Vec<_>, _>>()
            .map_err(|_| ScimError::internal_server_error("Filter evaluation failed"))
    }
    
    fn evaluate_all_filters(&self, resource: &Resource) -> bool {
        if self.filters.is_empty() {
            return true;
        }
        
        let mut result = true;
        let mut current_logical_op = LogicalOperator::And;
        
        for filter in &self.filters {
            let filter_result = self.evaluate_single_filter(resource, filter);
            
            result = match current_logical_op {
                LogicalOperator::And => result && filter_result,
                LogicalOperator::Or => result || filter_result,
                LogicalOperator::Not => result && !filter_result,
            };
            
            if let Some(next_op) = &filter.logical_op {
                current_logical_op = next_op.clone();
            }
        }
        
        result
    }
    
    fn evaluate_single_filter(&self, resource: &Resource, filter: &FilterClause) -> bool {
        if let Some(resource_value) = self.get_nested_value(&resource.attributes, &filter.path) {
            self.compare_filter_values(resource_value, &filter.operator, &filter.value)
        } else {
            false
        }
    }
    
    fn get_nested_value(&self, obj: &Value, path: &str) -> Option<&Value> {
        let parts: Vec<&str> = path.split('.').collect();
        let mut current = obj;
        
        for part in parts {
            current = current.get(part)?;
        }
        
        Some(current)
    }
    
    fn compare_filter_values(&self, resource_val: &Value, op: &FilterOperator, filter_val: &FilterValue) -> bool {
        match (resource_val, filter_val) {
            (Value::String(r), FilterValue::String(f)) => match op {
                FilterOperator::Equal => r == f,
                FilterOperator::NotEqual => r != f,
                FilterOperator::Contains => r.to_lowercase().contains(&f.to_lowercase()),
                FilterOperator::StartsWith => r.to_lowercase().starts_with(&f.to_lowercase()),
                FilterOperator::EndsWith => r.to_lowercase().ends_with(&f.to_lowercase()),
                _ => false,
            },
            (Value::Number(r), FilterValue::Number(f)) => match op {
                FilterOperator::Equal => (r.as_f64().unwrap() - f).abs() < f64::EPSILON,
                FilterOperator::NotEqual => (r.as_f64().unwrap() - f).abs() >= f64::EPSILON,
                FilterOperator::GreaterThan => r.as_f64().unwrap() > *f,
                FilterOperator::GreaterEqual => r.as_f64().unwrap() >= *f,
                FilterOperator::LessThan => r.as_f64().unwrap() < *f,
                FilterOperator::LessEqual => r.as_f64().unwrap() <= *f,
                _ => false,
            },
            (Value::Bool(r), FilterValue::Boolean(f)) => match op {
                FilterOperator::Equal => r == f,
                FilterOperator::NotEqual => r != f,
                _ => false,
            },
            _ => false,
        }
    }
    
    fn compare_resources(&self, a: &Resource, b: &Resource, sort_attr: &str) -> Ordering {
        let val_a = self.get_nested_value(&a.attributes, sort_attr);
        let val_b = self.get_nested_value(&b.attributes, sort_attr);
        
        match (val_a, val_b) {
            (Some(Value::String(a)), Some(Value::String(b))) => a.cmp(b),
            (Some(Value::Number(a)), Some(Value::Number(b))) => {
                a.as_f64().partial_cmp(&b.as_f64().unwrap()).unwrap_or(Ordering::Equal)
            }
            (Some(Value::Bool(a)), Some(Value::Bool(b))) => a.cmp(b),
            (Some(_), None) => Ordering::Greater,
            (None, Some(_)) => Ordering::Less,
            _ => Ordering::Equal,
        }
    }
}

// Usage example
pub async fn advanced_query_example() -> Result<(), ScimError> {
    // let provider = get_provider();
    
    let query = QueryBuilder::new()
        .filter("active", FilterOperator::Equal, FilterValue::Boolean(true))
        .and_filter("emails.type", FilterOperator::Equal, FilterValue::String("work".to_string()))
        .and_filter("name.familyName", FilterOperator::StartsWith, FilterValue::String("Smith".to_string()))
        .sort_by("userName", SortOrder::Ascending)
        .limit(50)
        .offset(0);
    
    // let resources = provider.list(None, 1, 1000).await?.0;
    // let filtered_resources = query.execute(resources)?;
    
    Ok(())
}
```

## Bulk Operations

### Efficient Bulk Processing

```rust
use scim_server::resource::Resource;
use scim_server::providers::ResourceProvider;
use scim_server::error::ScimError;
use serde::{Deserialize, Serialize};
use futures::future::try_join_all;
use std::sync::Arc;

#[derive(Debug, Serialize, Deserialize)]
pub struct BulkRequest {
    pub operations: Vec<BulkOperation>,
    pub fail_on_errors: Option<usize>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BulkOperation {
    pub method: BulkMethod,
    pub path: String,
    pub bulk_id: Option<String>,
    pub version: Option<String>,
    pub data: Option<Value>,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum BulkMethod {
    POST,
    PUT,
    PATCH,
    DELETE,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BulkResponse {
    pub operations: Vec<BulkOperationResult>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BulkOperationResult {
    pub method: BulkMethod,
    pub path: String,
    pub bulk_id: Option<String>,
    pub location: Option<String>,
    pub version: Option<String>,
    pub status: u16,
    pub response: Option<Value>,
}

pub struct BulkProcessor {
    provider: Arc<dyn ResourceProvider>,
    max_operations: usize,
    max_payload_size: usize,
}

impl BulkProcessor {
    pub fn new(provider: Arc<dyn ResourceProvider>) -> Self {
        Self {
            provider,
            max_operations: 1000,
            max_payload_size: 1024 * 1024, // 1MB
        }
    }
    
    pub async fn process_bulk_request(&self, request: BulkRequest) -> Result<BulkResponse, ScimError> {
        // Validate request limits
        if request.operations.len() > self.max_operations {
            return Err(ScimError::too_many(&format!(
                "Too many operations: {} (max: {})",
                request.operations.len(),
                self.max_operations
            )));
        }
        
        let fail_on_errors = request.fail_on_errors.unwrap_or(0);
        let mut results = Vec::new();
        let mut error_count = 0;
        
        // Process operations in batches for better performance
        let batch_size = 50;
        for chunk in request.operations.chunks(batch_size) {
            if fail_on_errors > 0 && error_count >= fail_on_errors {
                break;
            }
            
            let batch_futures: Vec<_> = chunk
                .iter()
                .map(|op| self.process_single_operation(op.clone()))
                .collect();
            
            let batch_results = try_join_all(batch_futures).await?;
            
            for result in batch_results {
                if result.status >= 400 {
                    error_count += 1;
                }
                results.push(result);
                
                if fail_on_errors > 0 && error_count >= fail_on_errors {
                    break;
                }
            }
        }
        
        Ok(BulkResponse { operations: results })
    }
    
    async fn process_single_operation(&self, operation: BulkOperation) -> Result<BulkOperationResult, ScimError> {
        let result = match operation.method {
            BulkMethod::POST => self.handle_bulk_create(&operation).await,
            BulkMethod::PUT => self.handle_bulk_update(&operation).await,
            BulkMethod::PATCH => self.handle_bulk_patch(&operation).await,
            BulkMethod::DELETE => self.handle_bulk_delete(&operation).await,
        };
        
        match result {
            Ok((status, location, response)) => Ok(BulkOperationResult {
                method: operation.method,
                path: operation.path,
                bulk_id: operation.bulk_id,
                location,
                version: operation.version,
                status,
                response,
            }),
            Err(e) => Ok(BulkOperationResult {
                method: operation.method,
                path: operation.path,
                bulk_id: operation.bulk_id,
                location: None,
                version: operation.version,
                status: e.status_code(),
                response: Some(serde_json::to_value(e)?),
            }),
        }
    }
    
    async fn handle_bulk_create(&self, operation: &BulkOperation) -> Result<(u16, Option<String>, Option<Value>), ScimError> {
        let data = operation.data.as_ref()
            .ok_or_else(|| ScimError::invalid_value("POST operation requires data"))?;
        
        let resource: Resource = serde_json::from_value(data.clone())?;
        let created = self.provider.create(resource).await?;
        let location = format!("{}/{}", operation.path.trim_end_matches('/'), created.id.as_ref().unwrap().as_str());
        
        Ok((201, Some(location), Some(serde_json::to_value(created)?)))
    }
    
    async fn handle_bulk_update(&self, operation: &BulkOperation) -> Result<(u16, Option<String>, Option<Value>), ScimError> {
        let resource_id = self.extract_resource_id_from_path(&operation.path)?;
        let data = operation.data.as_ref()
            .ok_or_else(|| ScimError::invalid_value("PUT operation requires data"))?;
        
        let resource: Resource = serde_json::from_value(data.clone())?;
        let updated = self.provider.update(&resource_id, resource).await?;
        
        Ok((200, None, Some(serde_json::to_value(updated)?)))
    }
    
    async fn handle_bulk_patch(&self, operation: &BulkOperation) -> Result<(u16, Option<String>, Option<Value>), ScimError> {
        let resource_id = self.extract_resource_id_from_path(&operation.path)?;
        let data = operation.data.as_ref()
            .ok_or_else(|| ScimError::invalid_value("PATCH operation requires data"))?;
        
        // Simplified PATCH implementation - in practice, you'd implement RFC 7396 JSON Patch
        let existing = self.provider.get_by_id(&resource_id).await?
            .ok_or_else(|| ScimError::not_found(&format!("Resource {} not found", resource_id)))?;
        
        let mut merged_data = serde_json::to_value(existing)?;
        self.merge_patch(&mut merged_data, data)?;
        
        let patched_resource: Resource = serde_json::from_value(merged_data)?;
        let updated = self.provider.update(&resource_id, patched_resource).await?;
        
        Ok((200, None, Some(serde_json::to_value(updated)?)))
    }
    
    async fn handle_bulk_delete(&self, operation: &BulkOperation) -> Result<(u16, Option<String>, Option<Value>), ScimError> {
        let resource_id = self.extract_resource_id_from_path(&operation.path)?;
        self.provider.delete(&resource_id).await?;
        Ok((204, None, None))
    }
    
    fn extract_resource_id_from_path(&self, path: &str) -> Result<ResourceId, ScimError> {
        let id_str = path.split('/').last()
            .ok_or_else(|| ScimError::invalid_value("Invalid resource path"))?;
        ResourceId::new(id_str).map_err(|e| ScimError::invalid_value(&e.to_string()))
    }
    
    fn merge_patch(&self, target: &mut Value, patch: &Value) -> Result<(), ScimError> {
        if let (Value::Object(target_obj), Value::Object(patch_obj)) = (target, patch) {
            for (key, value) in patch_obj {
                if value.is_null() {
                    target_obj.remove(key);
                } else {
                    target_obj.insert(key.clone(), value.clone());
                }
            }
        }
        Ok(())
    }
}

// Usage example
pub async fn bulk_operations_example() -> Result<(), ScimError> {
    // let provider = Arc::new(PostgresProvider::new("postgresql://...", "users".to_string()).await?);
    // let processor = BulkProcessor::new(provider);
    
    let bulk_request = BulkRequest {
        fail_on_errors: Some(5),
        operations: vec![
            BulkOperation {
                method: BulkMethod::POST,
                path: "/Users".to_string(),
                bulk_id: Some("create-user-1".to_string()),
                version: None,
                data: Some(serde_json::json!({
                    "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
                    "userName": "bulk.user1@example.com",
                    "name": {
                        "givenName": "Bulk",
                        "familyName": "User1"
                    }
                })),
            },
            BulkOperation {
                method: BulkMethod::POST,
                path: "/Users".to_string(),
                bulk_id: Some("create-user-2".to_string()),
                version: None,
                data: Some(serde_json::json!({
                    "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
                    "userName": "bulk.user2@example.com",
                    "name": {
                        "givenName": "Bulk",
                        "familyName": "User2"
                    }
                })),
            },
        ],
    };
    
    // let response = processor.process_bulk_request(bulk_request).await?;
    // println!("Processed {} operations", response.operations.len());
    
    Ok(())
}
```

## Custom Validation Logic

### Business Rule Validation

```rust
use scim_server::resource::Resource;
use scim_server::error::ScimError;
use scim_server::schema::{SchemaRegistry, OperationContext};
use async_trait::async_trait;
use std::collections::HashSet;

#[async_trait]
pub trait BusinessValidator: Send + Sync {
    async fn validate_create(&self, resource: &Resource) -> Result<(), ScimError>;
    async fn validate_update(&self, old: &Resource, new: &Resource) -> Result<(), ScimError>;
    async fn validate_delete(&self, resource: &Resource) -> Result<(), ScimError>;
}

pub struct UserBusinessValidator {
    schema_registry: Arc<SchemaRegistry>,
    blocked_domains: HashSet<String>,
    required_groups: HashSet<String>,
}

impl UserBusinessValidator {
    pub fn new(schema_registry: Arc<SchemaRegistry>) -> Self {
        let mut blocked_domains = HashSet::new();
        blocked_domains.insert("tempmail.com".to_string());
        blocked_domains.insert("10minutemail.com".to_string());
        
        let mut required_groups = HashSet::new();
        required_groups.insert("all-users".to_string());
        
        Self {
            schema_registry,
            blocked_domains,
            required_groups,
        }
    }
    
    fn validate_email_domain(&self, email: &str) -> Result<(), ScimError> {
        let domain = email.split('@').nth(1)
            .ok_or_else(|| ScimError::invalid_value("Invalid email format"))?;
        
        if self.blocked_domains.contains(domain) {
            return Err(ScimError::invalid_value(&format!(
                "Email domain {} is not allowed", domain
            )));
        }
        
        Ok(())
    }
    
    fn validate_username_format(&self, username: &str) -> Result<(), ScimError> {
        // Business rule: usernames must be email format
        if !username.contains('@') {
            return Err(ScimError::invalid_value(
                "Username must be in email format"
            ));
        }
        
        // Business rule: no special characters except @ and .
        if username.chars().any(|c| !c.is_alphanumeric() && c != '@' && c != '.' && c != '-' && c != '_') {
            return Err(ScimError::invalid_value(
                "Username contains invalid characters"
            ));
        }
        
        Ok(())
    }
    
    fn validate_required_attributes(&self, resource: &Resource) -> Result<(), ScimError> {
        // Business rule: active users must have email and display name
        if let Some(Value::Bool(true)) = resource.attributes.get("active") {
            if !resource.attributes.contains_key("emails") {
                return Err(ScimError::invalid_value(
                    "Active users must have at least one email address"
                ));
            }
            
            if !resource.attributes.contains_key("displayName") {
                return Err(ScimError::invalid_value(
                    "Active users must have a display name"
                ));
            }
        }
        
        Ok(())
    }
}

#[async_trait]
impl BusinessValidator for UserBusinessValidator {
    async fn validate_create(&self, resource: &Resource) -> Result<(), ScimError> {
        // First, validate against schema
        self.schema_registry.validate_json_resource_with_context(
            "User",
            &serde_json::to_value(resource)?,
            OperationContext::Create,
        )?;
        
        // Then apply business rules
        if let Some(Value::String(username)) = resource.attributes.get("userName") {
            self.validate_username_format(username)?;
            self.validate_email_domain(username)?;
        }
        
        self.validate_required_attributes(resource)?;
        
        Ok(())
    }
    
    async fn validate_update(&self, old: &Resource, new: &Resource) -> Result<(), ScimError> {
        // Validate schema first
        self.schema_registry.validate_json_resource_with_context(
            "User",
            &serde_json::to_value(new)?,
            OperationContext::Replace,
        )?;
        
        // Business rule: cannot deactivate admin users
        if let (Some(Value::Array(old_groups)), Some(Value::Bool(false))) = 
            (old.attributes.get("groups"), new.attributes.get("active")) {
            
            let is_admin = old_groups.iter().any(|group| {
                group.get("display")
                    .and_then(|v| v.as_str())
                    .map_or(false, |s| s == "Administrators")
            });
            
            if is_admin {
                return Err(ScimError::invalid_value(
                    "Cannot deactivate administrator users"
                ));
            }
        }
        
        self.validate_required_attributes(new)?;
        
        Ok(())
    }
    
    async fn validate_delete(&self, resource: &Resource) -> Result<(), ScimError> {
        // Business rule: cannot delete the last admin user
        if let Some(Value::Array(groups)) = resource.attributes.get("groups") {
            let is_admin = groups.iter().any(|group| {
                group.get("display")
                    .and_then(|v| v.as_str())
                    .map_or(false, |s| s == "Administrators")
            });
            
            if is_admin {
                // In a real implementation, you'd check if this is the last admin
                return Err(ScimError::invalid_value(
                    "Cannot delete administrator users"
                ));
            }
        }
        
        Ok(())
    }
}
```

## Resource Extensions

### Custom Attribute Extensions

```rust
use scim_server::resource::{Resource, ResourceBuilder};
use scim_server::schema::{Schema, AttributeDefinition, AttributeType};
use serde_json::{json, Value};

pub struct ResourceExtensionManager {
    extensions: HashMap<String, Extension>,
}

#[derive(Debug, Clone)]
pub struct Extension {
    pub schema_id: String,
    pub attributes: HashMap<String, AttributeDefinition>,
}

impl ResourceExtensionManager {
    pub fn new() -> Self {
        Self {
            extensions: HashMap::new(),
        }
    }
    
    pub fn register_extension(&mut self, extension: Extension) {
        self.extensions.insert(extension.schema_id.clone(), extension);
    }
    
    pub fn extend_resource(&self, mut resource: Resource, extension_data: Value) -> Result<Resource, ScimError> {
        // Validate extension data against registered extensions
        for (schema_id, data) in extension_data.as_object()
            .ok_or_else(|| ScimError::invalid_value("Extension data must be an object"))? {
            
            if let Some(extension) = self.extensions.get(schema_id) {
                self.validate_extension_data(extension, data)?;
                
                // Add to resource schemas
                if !resource.schemas.contains(schema_id) {
                    resource.schemas.push(schema_id.clone());
                }
                
                // Merge extension attributes
                if let Value::Object(ext_obj) = data {
                    for (key, value) in ext_obj {
                        resource.attributes.insert(
                            format!("{}:{}", schema_id, key),
                            value.clone()
                        );
                    }
                }
            } else {
                return Err(ScimError::invalid_value(&format!(
                    "Unknown extension schema: {}", schema_id
                )));
            }
        }
        
        Ok(resource)
    }
    
    fn validate_extension_data(&self, extension: &Extension, data: &Value) -> Result<(), ScimError> {
        let obj = data.as_object()
            .ok_or_else(|| ScimError::invalid_value("Extension data must be an object"))?;
        
        for (attr_name, attr_def) in &extension.attributes {
            if attr_def.required && !obj.contains_key(attr_name) {
                return Err(ScimError::invalid_value(&format!(
                    "Required extension attribute missing: {}", attr_name
                )));
            }
            
            if let Some(value) = obj.get(attr_name) {
                self.validate_attribute_value(value, attr_def)?;
            }
        }
        
        Ok(())
    }
    
    fn validate_attribute_value(&self, value: &Value, attr_def: &AttributeDefinition) -> Result<(), ScimError> {
        match (&attr_def.data_type, value) {
            (AttributeType::String, Value::String(_)) => Ok(()),
            (AttributeType::Boolean, Value::Bool(_)) => Ok(()),
            (AttributeType::Integer, Value::Number(n)) if n.is_i64() => Ok(()),
            (AttributeType::Decimal, Value::Number(_)) => Ok(()),
            (AttributeType::DateTime, Value::String(s)) => {
                chrono::DateTime::parse_from_rfc3339(s)
                    .map_err(|_| ScimError::invalid_value("Invalid datetime format"))?;
                Ok(())
            }
            _ => Err(ScimError::invalid_value(&format!(
                "Type mismatch for attribute: expected {:?}", attr_def.data_type
            ))),
        }
    }
}

// Example: Enterprise User Extension
pub fn create_enterprise_extension() -> Extension {
    let mut attributes = HashMap::new();
    
    attributes.insert("employeeNumber".to_string(), AttributeDefinition {
        name: "employeeNumber".to_string(),
        data_type: AttributeType::String,
        multi_valued: false,
        required: false,
        case_exact: false,
        mutability: scim_server::schema::Mutability::ReadWrite,
        returned: scim_server::schema::Returned::Default,
        uniqueness: scim_server::schema::Uniqueness::Server,
        description: Some("Employee number".to_string()),
        canonical_values: Vec::new(),
        sub_attributes: Vec::new(),
    });
    
    attributes.insert("department".to_string(), AttributeDefinition {
        name: "department".to_string(),
        data_type: AttributeType::String,
        multi_valued: false,
        required: false,
        case_exact: false,
        mutability: scim_server::schema::Mutability::ReadWrite,
        returned: scim_server::schema::Returned::Default,
        uniqueness: scim_server::schema::Uniqueness::None,
        description: Some("Department name".to_string()),
        canonical_values: Vec::new(),
        sub_attributes: Vec::new(),
    });
    
    Extension {
        schema_id: "urn:ietf:params:scim:schemas:extension:enterprise:2.0:User".to_string(),
        attributes,
    }
}

// Usage example
pub fn extension_usage_example() -> Result<(), ScimError> {
    let mut manager = ResourceExtensionManager::new();
    manager.register_extension(create_enterprise_extension());
    
    let base_resource = ResourceBuilder::new()
        .id("user123")
        .schema("urn:ietf:params:scim:schemas:core:2.0:User")
        .attribute("userName", "john.doe@example.com")
        .attribute("name", json!({
            "givenName": "John",
            "familyName": "Doe"
        }))
        .build()?;
    
    let extension_data = json!({
        "urn:ietf:params:scim:schemas:extension:enterprise:2.0:User": {
            "employeeNumber": "EMP001",
            "department": "Engineering"
        }
    });
    
    let extended_resource = manager.extend_resource(base_resource, extension_data)?;
    
    Ok(())
}
```

## Performance Optimizations

### Resource Caching and Indexing

```rust
use scim_server::resource::{Resource, ResourceId};
use scim_server::providers::ResourceProvider;
use scim_server::error::ScimError;
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use lru::LruCache;
use std::time::{Duration, Instant};

pub struct CachedResourceProvider {
    base_provider: Arc<dyn ResourceProvider>,
    cache: Arc<RwLock<LruCache<ResourceId, CachedResource>>>,
    index_cache: Arc<RwLock<HashMap<String, Vec<ResourceId>>>>, // Attribute -> Resource IDs
    cache_ttl: Duration,
}

#[derive(Clone)]
struct CachedResource {
    resource: Resource,
    cached_at: Instant,
}

impl CachedResourceProvider {
    pub fn new(base_provider: Arc<dyn ResourceProvider>, cache_size: usize, ttl: Duration) -> Self {
        Self {
            base_provider,
            cache: Arc::new(RwLock::new(LruCache::new(cache_size.try_into().unwrap()))),
            index_cache: Arc::new(RwLock::new(HashMap::new())),
            cache_ttl: ttl,
        }
    }
    
    async fn get_from_cache(&self, id: &ResourceId) -> Option<Resource> {
        let mut cache = self.cache.write().await;
        
        if let Some(cached) = cache.get(id) {
            if cached.cached_at.elapsed() < self.cache_ttl {
                return Some(cached.resource.clone());
            } else {
                // Remove expired entry
                cache.pop(id);
            }
        }
        
        None
    }
    
    async fn cache_resource(&self, resource: Resource) {
        let id = resource.id.clone().unwrap();
        let cached = CachedResource {
            resource: resource.clone(),
            cached_at: Instant::now(),
        };
        
        // Cache the resource
        {
            let mut cache = self.cache.write().await;
            cache.put(id.clone(), cached);
        }
        
        // Update indices
        self.update_indices(&id, &resource).await;
    }
    
    async fn update_indices(&self, id: &ResourceId, resource: &Resource) {
        let mut index_cache = self.index_cache.write().await;
        
        // Index by userName
        if let Some(Value::String(username)) = resource.attributes.get("userName") {
            index_cache
                .entry(format!("userName:{}", username))
                .or_insert_with(Vec::new)
                .push(id.clone());
        }
        
        // Index by email
        if let Some(Value::Array(emails)) = resource.attributes.get("emails") {
            for email in emails {
                if let Some(Value::String(email_addr)) = email.get("value") {
                    index_cache
                        .entry(format!("email:{}", email_addr))
                        .or_insert_with(Vec::new)
                        .push(id.clone());
                }
            }
        }
        
        // Index by active status
        if let Some(Value::Bool(active)) = resource.attributes.get("active") {
            index_cache
                .entry(format!("active:{}", active))
                .or_insert_with(Vec::new)
                .push(id.clone());
        }
    }
    
    async fn find_by_index(&self, key: &str) -> Option<Vec<ResourceId>> {
        let index_cache = self.index_cache.read().await;
        index_cache.get(key).cloned()
    }
    
    async fn invalidate_cache(&self, id: &ResourceId) {
        {
            let mut cache = self.cache.write().await;
            cache.pop(id);
        }
        
        // Remove from indices
        {
            let mut index_cache = self.index_cache.write().await;
            index_cache.retain(|_, ids| {
                ids.retain(|cached_id| cached_id != id);
                !ids.is_empty()
            });
        }
    }
}

#[async_trait]
impl ResourceProvider for CachedResourceProvider {
    async fn get_by_id(&self, id: &ResourceId) -> Result<Option<Resource>, ScimError> {
        // Check cache first
        if let Some(resource) = self.get_from_cache(id).await {
            return Ok(Some(resource));
        }
        
        // Fallback to base provider
        if let Some(resource) = self.base_provider.get_by_id(id).await? {
            self.cache_resource(resource.clone()).await;
            Ok(Some(resource))
        } else {
            Ok(None)
        }
    }
    
    async fn create(&self, resource: Resource) -> Result<Resource, ScimError> {
        let created = self.base_provider.create(resource).await?;
        self.cache_resource(created.clone()).await;
        Ok(created)
    }
    
    async fn update(&self, id: &ResourceId, resource: Resource) -> Result<Resource, ScimError> {
        let updated = self.base_provider.update(id, resource).await?;
        self.invalidate_cache(id).await;
        self.cache_resource(updated.clone()).await;
        Ok(updated)
    }
    
    async fn delete(&self, id: &ResourceId) -> Result<(), ScimError> {
        self.base_provider.delete(id).await?;
        self.invalidate_cache(id).await;
        Ok(())
    }
    
    async fn list(&self, filter: Option<&str>, start_index: usize, count: usize) 
        -> Result<(Vec<Resource>, usize), ScimError> {
        
        // Try to use index for simple filters
        if let Some(filter_str) = filter {
            if let Some(index_key) = self.extract_index_key(filter_str) {
                if let Some(ids) = self.find_by_index(&index_key).await {
                    let mut resources = Vec::new();
                    
                    for id in ids.iter().skip(start_index.saturating_sub(1)).take(count) {
                        if let Some(resource) = self.get_by_id(id).await? {
                            resources.push(resource);
                        }
                    }
                    
                    return Ok((resources, ids.len()));
                }
            }
        }
        
        // Fallback to base provider
        self.base_provider.list(filter, start_index, count).await
    }
    
    fn extract_index_key(&self, filter: &str) -> Option<String> {
        // Simple index key extraction - in production, use proper filter parsing
        if let Some(captures) = regex::Regex::new(r"(\w+)\s+eq\s+\"([^\"]+)\"")
            .ok()?
            .captures(filter) {
            
            let attr = captures.get(1)?.as_str();
            let value = captures.get(2)?.as_str();
            
            Some(format!("{}:{}", attr, value))
        } else {
            None
        }
    }
}
```

## Advanced Error Handling

### Structured Error Responses with Context

```rust
use scim_server::error::ScimError;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Serialize, Deserialize)]
pub struct DetailedScimError {
    pub schemas: Vec<String>,
    pub detail: String,
    pub status: String,
    pub scim_type: Option<String>,
    pub context: ErrorContext,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ErrorContext {
    pub operation: String,
    pub resource_type: Option<String>,
    pub resource_id: Option<String>,
    pub tenant_id: Option<String>,
    pub attribute_path: Option<String>,
    pub validation_errors: Vec<ValidationError>,
    pub trace_id: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ValidationError {
    pub field: String,
    pub code: String,
    pub message: String,
    pub rejected_value: Option<Value>,
}

pub struct ErrorEnhancer {
    trace_id_generator: TraceIdGenerator,
}

impl ErrorEnhancer {
    pub fn new() -> Self {
        Self {
            trace_id_generator: TraceIdGenerator::new(),
        }
    }
    
    pub fn enhance_error(&self, error: ScimError, context: ErrorContext) -> DetailedScimError {
        DetailedScimError {
            schemas: vec!["urn:ietf:params:scim:api:messages:2.0:Error".to_string()],
            detail: error.detail().unwrap_or("An error occurred").to_string(),
            status: error.status_code().to_string(),
            scim_type: error.scim_type().map(|s| s.to_string()),
            context: ErrorContext {
                trace_id: self.trace_id_generator.generate(),
                ..context
            },
        }
    }
    
    pub fn create_validation_error(&self, field: &str, message: &str, rejected_value: Option<Value>) -> ValidationError {
        ValidationError {
            field: field.to_string(),
            code: "invalid_value".to_string(),
            message: message.to_string(),
            rejected_value,
        }
    }
}

pub struct TraceIdGenerator {
    counter: std::sync::atomic::AtomicU64,
}

impl TraceIdGenerator {
    pub fn new() -> Self {
        Self {
            counter: std::sync::atomic::AtomicU64::new(0),
        }
    }
    
    pub fn generate(&self) -> String {
        let count = self.counter.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        format!("trace-{}-{}", 
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            count
        )
    }
}
```

## Middleware Integration

### Request/Response Middleware Chain

```rust
use scim_server::resource::Resource;
use scim_server::error::ScimError;
use async_trait::async_trait;
use std::sync::Arc;
use serde_json::Value;

#[async_trait]
pub trait Middleware: Send + Sync {
    async fn before_request(&self, context: &mut RequestContext) -> Result<(), ScimError>;
    async fn after_response(&self, context: &mut ResponseContext) -> Result<(), ScimError>;
}

pub struct RequestContext {
    pub method: String,
    pub path: String,
    pub headers: HashMap<String, String>,
    pub body: Option<Value>,
    pub tenant_id: Option<String>,
    pub user_id: Option<String>,
    pub correlation_id: String,
}

pub struct ResponseContext {
    pub status: u16,
    pub headers: HashMap<String, String>,
    pub body: Option<Value>,
    pub processing_time: Duration,
    pub correlation_id: String,
}

pub struct MiddlewareChain {
    middlewares: Vec<Arc<dyn Middleware>>,
}

impl MiddlewareChain {
    pub fn new() -> Self {
        Self {
            middlewares: Vec::new(),
        }
    }
    
    pub fn add_middleware(mut self, middleware: Arc<dyn Middleware>) -> Self {
        self.middlewares.push(middleware);
        self
    }
    
    pub async fn process_request(&self, mut context: RequestContext) -> Result<RequestContext, ScimError> {
        for middleware in &self.middlewares {
            middleware.before_request(&mut context).await?;
        }
        Ok(context)
    }
    
    pub async fn process_response(&self, mut context: ResponseContext) -> Result<ResponseContext, ScimError> {
        // Process middlewares in reverse order for response
        for middleware in self.middlewares.iter().rev() {
            middleware.after_response(&mut context).await?;
        }
        Ok(context)
    }
}

// Example: Audit Logging Middleware
pub struct AuditMiddleware {
    audit_logger: Arc<dyn AuditLogger>,
}

#[async_trait]
pub trait AuditLogger: Send + Sync {
    async fn log_request(&self, event: AuditEvent) -> Result<(), ScimError>;
}

#[derive(Debug, Serialize)]
pub struct AuditEvent {
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub correlation_id: String,
    pub tenant_id: Option<String>,
    pub user_id: Option<String>,
    pub operation: String,
    pub resource_type: Option<String>,
    pub resource_id: Option<String>,
    pub status: u16,
    pub processing_time_ms: u64,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
}

#[async_trait]
impl Middleware for AuditMiddleware {
    async fn before_request(&self, context: &mut RequestContext) -> Result<(), ScimError> {
        // Could add request validation, rate limiting, etc.
        Ok(())
    }
    
    async fn after_response(&self, context: &mut ResponseContext) -> Result<(), ScimError> {
        let audit_event = AuditEvent {
            timestamp: chrono::Utc::now(),
            correlation_id: context.correlation_id.clone(),
            tenant_id: None, // Would be extracted from request context
            user_id: None,   // Would be extracted from authentication
            operation: "SCIM_OPERATION".to_string(), // Would be derived from method/path
            resource_type: None, // Would be extracted from path
            resource_id: None,   // Would be extracted from path
            status: context.status,
            processing_time_ms: context.processing_time.as_millis() as u64,
            ip_address: context.headers.get("x-forwarded-for").cloned(),
            user_agent: context.headers.get("user-agent").cloned(),
        };
        
        self.audit_logger.log_request(audit_event).await?;
        Ok(())
    }
}

// Example: Rate Limiting Middleware
pub struct RateLimitMiddleware {
    rate_limiter: Arc<dyn RateLimiter>,
}

#[async_trait]
pub trait RateLimiter: Send + Sync {
    async fn check_rate_limit(&self, key: &str) -> Result<bool, ScimError>;
}

#[async_trait]
impl Middleware for RateLimitMiddleware {
    async fn before_request(&self, context: &mut RequestContext) -> Result<(), ScimError> {
        let rate_key = format!("{}:{}", 
            context.tenant_id.as_deref().unwrap_or("default"),
            context.headers.get("x-forwarded-for").unwrap_or(&"unknown".to_string())
        );
        
        if !self.rate_limiter.check_rate_limit(&rate_key).await? {
            return Err(ScimError::too_many("Rate limit exceeded"));
        }
        
        Ok(())
    }
    
    async fn after_response(&self, _context: &mut ResponseContext) -> Result<(), ScimError> {
        Ok(())
    }
}
```

## Custom Schema Loading

### Dynamic Schema Management

```rust
use scim_server::schema::{Schema, SchemaRegistry, AttributeDefinition};
use scim_server::error::ScimError;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

pub struct DynamicSchemaManager {
    base_registry: Arc<SchemaRegistry>,
    custom_schemas: Arc<RwLock<HashMap<String, Schema>>>,
    schema_loader: Arc<dyn SchemaLoader>,
}

#[async_trait]
pub trait SchemaLoader: Send + Sync {
    async fn load_schema(&self, schema_id: &str) -> Result<Schema, ScimError>;
    async fn list_available_schemas(&self) -> Result<Vec<String>, ScimError>;
}

pub struct DatabaseSchemaLoader {
    pool: PgPool,
}

#[async_trait]
impl SchemaLoader for DatabaseSchemaLoader {
    async fn load_schema(&self, schema_id: &str) -> Result<Schema, ScimError> {
        let row = sqlx::query("SELECT schema_definition FROM custom_schemas WHERE schema_id = $1")
            .bind(schema_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| ScimError::internal_server_error(&format!("Database error: {}", e)))?;
        
        match row {
            Some(row) => {
                let schema_json: Value = row.get("schema_definition");
                let schema: Schema = serde_json::from_value(schema_json)
                    .map_err(|e| ScimError::invalid_value(&format!("Invalid schema format: {}", e)))?;
                
                self.validate_custom_schema(&schema)?;
                Ok(schema)
            }
            None => Err(ScimError::not_found(&format!("Schema {} not found", schema_id))),
        }
    }
    
    async fn list_available_schemas(&self) -> Result<Vec<String>, ScimError> {
        let rows = sqlx::query("SELECT schema_id FROM custom_schemas WHERE active = true")
            .fetch_all(&self.pool)
            .await
            .map_err(|e| ScimError::internal_server_error(&format!("Database error: {}", e)))?;
        
        Ok(rows.into_iter()
            .map(|row| row.get::<String, _>("schema_id"))
            .collect())
    }
}

impl DatabaseSchemaLoader {
    fn validate_custom_schema(&self, schema: &Schema) -> Result<(), ScimError> {
        // Validate schema ID format
        if !schema.id.starts_with("urn:") && !schema.id.starts_with("http") {
            return Err(ScimError::invalid_value("Schema ID must be a valid URI"));
        }
        
        // Validate required fields
        if schema.name.is_empty() {
            return Err(ScimError::invalid_value("Schema name cannot be empty"));
        }
        
        if schema.attributes.is_empty() {
            return Err(ScimError::invalid_value("Schema must have at least one attribute"));
        }
        
        // Validate each attribute
        for attr in &schema.attributes {
            self.validate_attribute_definition(attr)?;
        }
        
        Ok(())
    }
    
    fn validate_attribute_definition(&self, attr: &AttributeDefinition) -> Result<(), ScimError> {
        // Validate attribute name
        if attr.name.is_empty() {
            return Err(ScimError::invalid_value("Attribute name cannot be empty"));
        }
        
        // Validate complex attributes have sub-attributes
        if matches!(attr.data_type, scim_server::schema::AttributeType::Complex) && attr.sub_attributes.is_empty() {
            return Err(ScimError::invalid_value(&format!(
                "Complex attribute '{}' must have sub-attributes", attr.name
            )));
        }
        
        // Validate canonical values are only for string types
        if !attr.canonical_values.is_empty() && !matches!(attr.data_type, scim_server::schema::AttributeType::String) {
            return Err(ScimError::invalid_value(&format!(
                "Canonical values only allowed for string attributes: {}", attr.name
            )));
        }
        
        Ok(())
    }
}

impl DynamicSchemaManager {
    pub async fn new(base_registry: Arc<SchemaRegistry>, schema_loader: Arc<dyn SchemaLoader>) -> Self {
        Self {
            base_registry,
            custom_schemas: Arc::new(RwLock::new(HashMap::new())),
            schema_loader,
        }
    }
    
    pub async fn load_custom_schema(&self, schema_id: &str) -> Result<Schema, ScimError> {
        // Check if already loaded
        {
            let schemas = self.custom_schemas.read().await;
            if let Some(schema) = schemas.get(schema_id) {
                return Ok(schema.clone());
            }
        }
        
        // Load from external source
        let schema = self.schema_loader.load_schema(schema_id).await?;
        
        // Cache the loaded schema
        {
            let mut schemas = self.custom_schemas.write().await;
            schemas.insert(schema_id.to_string(), schema.clone());
        }
        
        Ok(schema)
    }
    
    pub async fn get_schema(&self, schema_id: &str) -> Result<Schema, ScimError> {
        // Try base registry first
        if let Ok(schema) = self.base_registry.get_schema(schema_id) {
            return Ok(schema);
        }
        
        // Try custom schemas
        self.load_custom_schema(schema_id).await
    }
    
    pub async fn reload_all_schemas(&self) -> Result<(), ScimError> {
        let available_schemas = self.schema_loader.list_available_schemas().await?;
        
        let mut new_schemas = HashMap::new();
        for schema_id in available_schemas {
            match self.schema_loader.load_schema(&schema_id).await {
                Ok(schema) => {
                    new_schemas.insert(schema_id, schema);
                }
                Err(e) => {
                    eprintln!("Failed to load schema {}: {}", schema_id, e);
                }
            }
        }
        
        // Replace all custom schemas atomically
        {
            let mut schemas = self.custom_schemas.write().await;
            *schemas = new_schemas;
        }
        
        Ok(())
    }
}
```

## Best Practices Summary

### Performance Optimization Patterns

1. **Use Connection Pooling**: For database providers, always use connection pools
2. **Implement Caching**: Cache frequently accessed resources with appropriate TTL
3. **Batch Operations**: Process bulk operations in batches to avoid memory issues
4. **Index Common Queries**: Build indices for frequently filtered attributes
5. **Lazy Loading**: Load schemas and extensions only when needed

### Error Handling Patterns

1. **Structured Errors**: Provide detailed error context for debugging
2. **Correlation IDs**: Track requests across distributed systems
3. **Graceful Degradation**: Handle partial failures in bulk operations
4. **Validation Layers**: Separate schema validation from business rule validation

### Multi-Tenancy Patterns

1. **Configuration Inheritance**: Allow tenant hierarchies with config inheritance
2. **Resource Isolation**: Ensure complete isolation between tenants
3. **Schema Customization**: Support per-tenant schema extensions
4. **Performance Isolation**: Prevent one tenant from affecting others

### Security Patterns

1. **Input Validation**: Always validate and sanitize inputs
2. **Authorization Context**: Carry authorization context through the request pipeline
3. **Audit Logging**: Log all operations for compliance and debugging
4. **Rate Limiting**: Implement rate limiting to prevent abuse

## Related Documentation

- [Basic Server Example](basic-server.md) - Simple server setup
- [Multi-Tenant Server Example](multi-tenant-server.md) - Multi-tenancy basics
- [Custom Providers Example](custom-providers.md) - Provider implementation
- [Performance Guide](../reference/performance.md) - Performance optimization details
- [Security Guide](../reference/security.md) - Security best practices