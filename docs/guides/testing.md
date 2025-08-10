# Testing Guide

This guide covers testing strategies, best practices, and test organization for the SCIM Server crate. It explains how to write effective tests for different components and ensure high code quality.

## Table of Contents

- [Testing Philosophy](#testing-philosophy)
- [Test Organization](#test-organization)
- [Unit Testing](#unit-testing)
- [Integration Testing](#integration-testing)
- [Documentation Testing](#documentation-testing)
- [Property-Based Testing](#property-based-testing)
- [Performance Testing](#performance-testing)
- [Multi-Tenant Testing](#multi-tenant-testing)
- [Provider Testing](#provider-testing)
- [Testing Utilities](#testing-utilities)
- [Best Practices](#best-practices)

## Testing Philosophy

The SCIM Server follows a comprehensive testing strategy with multiple layers:

1. **Unit Tests** - Test individual components in isolation
2. **Integration Tests** - Test component interactions
3. **Documentation Tests** - Ensure examples in docs work
4. **Property Tests** - Verify properties hold across input ranges
5. **Performance Tests** - Validate performance characteristics
6. **Contract Tests** - Ensure SCIM protocol compliance

### Testing Pyramid

```
                    ┌─────────────────┐
                    │   E2E Tests     │ (Few, Slow, High Value)
                    └─────────────────┘
                  ┌─────────────────────┐
                  │ Integration Tests   │ (Some, Medium Speed)
                  └─────────────────────┘
              ┌─────────────────────────────┐
              │      Unit Tests             │ (Many, Fast, Focused)
              └─────────────────────────────┘
```

## Test Organization

### Directory Structure

```
tests/
├── integration/          # Integration tests
│   ├── api_tests.rs     # Full API testing
│   ├── multi_tenant.rs  # Multi-tenant scenarios
│   └── providers.rs     # Provider integration tests
├── property/            # Property-based tests
│   ├── resource_props.rs
│   └── schema_props.rs
├── performance/         # Performance benchmarks
│   ├── resource_ops.rs
│   └── concurrent_access.rs
└── fixtures/            # Test data and utilities
    ├── resources.json
    ├── schemas.json
    └── test_helpers.rs
```

### Test Module Organization

Each source module should have corresponding tests:

```rust
// src/resource/value_objects/email_address.rs
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_valid_email_creation() {
        // Unit tests here
    }
    
    #[test]
    fn test_invalid_email_rejection() {
        // More unit tests
    }
}
```

## Unit Testing

### Testing Value Objects

Value objects should be thoroughly tested for validation logic:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::ValidationError;

    #[test]
    fn test_resource_id_creation() {
        // Valid cases
        assert!(ResourceId::new("user-123").is_ok());
        assert!(ResourceId::new("group_456").is_ok());
        assert!(ResourceId::new("a").is_ok()); // Minimum length
        
        // Invalid cases
        assert!(ResourceId::new("").is_err());
        assert!(ResourceId::new(" ").is_err());
        assert!(ResourceId::new("user with spaces").is_err());
    }

    #[test]
    fn test_email_address_validation() {
        // Valid emails
        let valid_emails = vec![
            "user@example.com",
            "test.email+tag@domain.co.uk",
            "user123@sub.domain.org",
        ];
        
        for email in valid_emails {
            assert!(EmailAddress::new(email).is_ok(), "Failed on: {}", email);
        }
        
        // Invalid emails
        let invalid_emails = vec![
            "",
            "not-an-email",
            "@domain.com",
            "user@",
            "user name@domain.com",
        ];
        
        for email in invalid_emails {
            assert!(EmailAddress::new(email).is_err(), "Should fail on: {}", email);
        }
    }

    #[test]
    fn test_multi_valued_attribute_operations() {
        let emails = MultiValuedAttribute::new()
            .with_value(EmailAddress::new("primary@example.com").unwrap())
            .with_primary_value(EmailAddress::new("work@company.com").unwrap());
        
        assert_eq!(emails.len(), 2);
        assert!(emails.primary().is_some());
        assert_eq!(emails.primary().unwrap().value(), "work@company.com");
    }
}
```

### Testing Resource Operations

```rust
#[cfg(test)]
mod resource_tests {
    use super::*;
    use crate::resource::{Resource, ResourceBuilder};
    use serde_json::json;

    #[test]
    fn test_resource_creation() {
        let resource = ResourceBuilder::new()
            .id(ResourceId::new("test-user").unwrap())
            .user_name(UserName::new("testuser").unwrap())
            .display_name("Test User")
            .build()
            .unwrap();
        
        assert_eq!(resource.id().as_str(), "test-user");
        assert_eq!(resource.user_name().unwrap().as_str(), "testuser");
        assert_eq!(resource.display_name(), Some("Test User"));
    }

    #[test]
    fn test_resource_serialization() {
        let resource = create_test_resource();
        
        // Test JSON serialization roundtrip
        let json = serde_json::to_value(&resource).unwrap();
        let deserialized: Resource = serde_json::from_value(json).unwrap();
        
        assert_eq!(resource.id(), deserialized.id());
        assert_eq!(resource.user_name(), deserialized.user_name());
    }

    #[test]
    fn test_resource_validation() {
        let invalid_resource = Resource::from_json(json!({
            "id": "",  // Invalid empty ID
            "userName": "testuser",
            "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"]
        }));
        
        assert!(invalid_resource.is_err());
    }
}
```

### Testing Schema Validation

```rust
#[cfg(test)]
mod schema_tests {
    use super::*;
    use crate::schema::validation::SchemaValidator;

    #[tokio::test]
    async fn test_user_schema_validation() {
        let validator = SchemaValidator::new();
        
        // Valid user
        let valid_user = create_valid_test_user();
        assert!(validator.validate(&valid_user).await.is_ok());
        
        // Invalid user - missing required field
        let invalid_user = Resource::from_json(json!({
            "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
            // Missing required userName
            "displayName": "Test User"
        })).unwrap();
        
        let result = validator.validate(&invalid_user).await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), ScimError::Validation { .. }));
    }

    #[tokio::test]
    async fn test_schema_extension_validation() {
        let validator = SchemaValidator::with_extensions(vec![
            Box::new(EnterpriseUserExtension::new())
        ]);
        
        let user_with_extension = create_user_with_enterprise_extension();
        assert!(validator.validate(&user_with_extension).await.is_ok());
    }
}
```

## Integration Testing

### Full API Testing

Integration tests validate the complete request/response flow:

```rust
// tests/integration/api_tests.rs
use scim_server::{ScimServer, ServerConfig};
use scim_server::providers::InMemoryProvider;
use reqwest::Client;
use serde_json::json;

async fn setup_test_server() -> (ScimServer, String) {
    let provider = InMemoryProvider::new();
    let config = ServerConfig::builder()
        .host("localhost")
        .port(0) // Random available port
        .provider(provider)
        .build()
        .unwrap();
    
    let server = ScimServer::new(config);
    let addr = server.local_addr();
    
    tokio::spawn(async move {
        server.run().await.unwrap();
    });
    
    // Wait for server to start
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    
    (server, format!("http://{}", addr))
}

#[tokio::test]
async fn test_user_crud_operations() {
    let (_server, base_url) = setup_test_server().await;
    let client = Client::new();
    
    // Create user
    let user_data = json!({
        "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
        "userName": "testuser",
        "name": {
            "givenName": "Test",
            "familyName": "User"
        },
        "emails": [{
            "value": "test@example.com",
            "type": "work",
            "primary": true
        }]
    });
    
    let response = client
        .post(&format!("{}/Users", base_url))
        .json(&user_data)
        .send()
        .await
        .unwrap();
    
    assert_eq!(response.status(), 201);
    let created_user: serde_json::Value = response.json().await.unwrap();
    let user_id = created_user["id"].as_str().unwrap();
    
    // Get user
    let response = client
        .get(&format!("{}/Users/{}", base_url, user_id))
        .send()
        .await
        .unwrap();
    
    assert_eq!(response.status(), 200);
    let retrieved_user: serde_json::Value = response.json().await.unwrap();
    assert_eq!(retrieved_user["userName"], "testuser");
    
    // Update user
    let update_data = json!({
        "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
        "id": user_id,
        "userName": "testuser",
        "displayName": "Updated Test User"
    });
    
    let response = client
        .put(&format!("{}/Users/{}", base_url, user_id))
        .json(&update_data)
        .send()
        .await
        .unwrap();
    
    assert_eq!(response.status(), 200);
    
    // Delete user
    let response = client
        .delete(&format!("{}/Users/{}", base_url, user_id))
        .send()
        .await
        .unwrap();
    
    assert_eq!(response.status(), 204);
    
    // Verify deletion
    let response = client
        .get(&format!("{}/Users/{}", base_url, user_id))
        .send()
        .await
        .unwrap();
    
    assert_eq!(response.status(), 404);
}

#[tokio::test]
async fn test_group_membership() {
    let (_server, base_url) = setup_test_server().await;
    let client = Client::new();
    
    // Create user first
    let user = create_test_user(&client, &base_url).await;
    let user_id = user["id"].as_str().unwrap();
    
    // Create group with user as member
    let group_data = json!({
        "schemas": ["urn:ietf:params:scim:schemas:core:2.0:Group"],
        "displayName": "Test Group",
        "members": [{
            "value": user_id,
            "type": "User",
            "display": "Test User"
        }]
    });
    
    let response = client
        .post(&format!("{}/Groups", base_url))
        .json(&group_data)
        .send()
        .await
        .unwrap();
    
    assert_eq!(response.status(), 201);
    let group: serde_json::Value = response.json().await.unwrap();
    
    // Verify membership
    assert_eq!(group["members"].as_array().unwrap().len(), 1);
    assert_eq!(group["members"][0]["value"], user_id);
}
```

### Provider Integration Testing

Test provider implementations with a common test suite:

```rust
// tests/integration/providers.rs
use scim_server::providers::{ResourceProvider, InMemoryProvider};
use scim_server::resource::{Resource, ResourceBuilder};

async fn test_provider_crud<P: ResourceProvider + 'static>(provider: P) {
    // Test creation
    let resource = create_test_resource();
    let created = provider.create_resource(resource).await.unwrap();
    assert!(created.id().as_str().len() > 0);
    
    // Test retrieval
    let retrieved = provider.get_resource(created.id()).await.unwrap();
    assert!(retrieved.is_some());
    assert_eq!(retrieved.unwrap().id(), created.id());
    
    // Test update
    let mut updated = created.clone();
    updated.set_display_name(Some("Updated Name"));
    let updated_result = provider.update_resource(updated).await.unwrap();
    assert_eq!(updated_result.display_name(), Some("Updated Name"));
    
    // Test deletion
    provider.delete_resource(created.id()).await.unwrap();
    let deleted = provider.get_resource(created.id()).await.unwrap();
    assert!(deleted.is_none());
}

#[tokio::test]
async fn test_in_memory_provider() {
    let provider = InMemoryProvider::new();
    test_provider_crud(provider).await;
}

// This pattern allows testing any provider implementation
// #[tokio::test]
// async fn test_database_provider() {
//     let provider = DatabaseProvider::new(&test_db_url()).await.unwrap();
//     test_provider_crud(provider).await;
// }
```

## Documentation Testing

### Running Documentation Tests

Documentation tests ensure that all code examples in documentation actually work:

```bash
# Run all documentation tests
cargo test --doc

# Run doc tests for specific module
cargo test --doc --package scim-server resource::value_objects

# Run with verbose output
cargo test --doc -- --nocapture
```

### Writing Good Documentation Tests

```rust
/// EmailAddress represents a validated email address.
/// 
/// # Examples
/// 
/// Creating a valid email address:
/// 
/// ```
/// # use scim_server::resource::value_objects::EmailAddress;
/// # use scim_server::error::Result;
/// # fn main() -> Result<()> {
/// let email = EmailAddress::new("user@example.com")?;
/// assert_eq!(email.value(), "user@example.com");
/// # Ok(())
/// # }
/// ```
/// 
/// Invalid email addresses are rejected:
/// 
/// ```should_panic
/// # use scim_server::resource::value_objects::EmailAddress;
/// // This will panic because the email is invalid
/// let email = EmailAddress::new("invalid-email").unwrap();
/// ```
/// 
/// Working with optional type information:
/// 
/// ```
/// # use scim_server::resource::value_objects::EmailAddress;
/// # use scim_server::error::Result;
/// # fn main() -> Result<()> {
/// let work_email = EmailAddress::new("work@company.com")?
///     .with_type("work")
///     .with_primary(true);
/// 
/// assert_eq!(work_email.type_(), Some("work"));
/// assert_eq!(work_email.primary(), Some(true));
/// # Ok(())
/// # }
/// ```
pub struct EmailAddress {
    // Implementation...
}
```

### Documentation Test Patterns

1. **Happy Path Examples** - Show normal usage
2. **Error Cases** - Demonstrate error handling (use `should_panic` or `Result`)
3. **Edge Cases** - Cover boundary conditions
4. **Integration Examples** - Show how components work together

## Property-Based Testing

Use property-based testing to verify invariants across large input spaces:

```rust
// tests/property/resource_props.rs
use proptest::prelude::*;
use scim_server::resource::{Resource, ResourceBuilder};
use scim_server::resource::value_objects::{ResourceId, UserName};

// Property: Resource ID roundtrip serialization
proptest! {
    #[test]
    fn test_resource_id_roundtrip(id_str in "[a-zA-Z0-9\\-_]{1,100}") {
        let resource_id = ResourceId::new(&id_str)?;
        let serialized = serde_json::to_value(&resource_id)?;
        let deserialized: ResourceId = serde_json::from_value(serialized)?;
        
        prop_assert_eq!(resource_id.as_str(), deserialized.as_str());
    }
}

// Property: Email validation consistency
proptest! {
    #[test]
    fn test_email_validation_consistency(
        local in "[a-zA-Z0-9]{1,20}",
        domain in "[a-zA-Z0-9]{1,20}",
        tld in "[a-zA-Z]{2,10}"
    ) {
        let email_str = format!("{}@{}.{}", local, domain, tld);
        let email = EmailAddress::new(&email_str);
        
        // Property: valid format should always create valid EmailAddress
        prop_assert!(email.is_ok());
        prop_assert_eq!(email.unwrap().value(), email_str);
    }
}

// Property: Multi-valued attribute invariants
proptest! {
    #[test]
    fn test_multi_valued_invariants(
        values in prop::collection::vec(any::<String>(), 0..10),
        primary_idx in any::<Option<usize>>()
    ) {
        let emails: Result<Vec<_>, _> = values.iter()
            .filter(|v| !v.is_empty() && v.contains('@'))
            .map(|v| EmailAddress::new(v))
            .collect();
        
        if let Ok(email_vec) = emails {
            if !email_vec.is_empty() {
                let mut attr = MultiValuedAttribute::new();
                for email in email_vec {
                    attr = attr.with_value(email);
                }
                
                // Property: length should match added values
                prop_assert!(attr.len() > 0);
                
                // Property: primary should be valid if set
                if let Some(primary) = attr.primary() {
                    prop_assert!(attr.values().contains(primary));
                }
            }
        }
    }
}
```

## Performance Testing

### Benchmark Tests

Use criterion for performance benchmarks:

```rust
// benches/resource_operations.rs
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use scim_server::resource::{Resource, ResourceBuilder};
use scim_server::providers::InMemoryProvider;

fn bench_resource_creation(c: &mut Criterion) {
    c.bench_function("resource_creation", |b| {
        b.iter(|| {
            ResourceBuilder::new()
                .id(ResourceId::new("bench-user").unwrap())
                .user_name(UserName::new("benchuser").unwrap())
                .display_name("Benchmark User")
                .build()
                .unwrap()
        })
    });
}

fn bench_provider_operations(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let provider = InMemoryProvider::new();
    
    c.bench_function("provider_create", |b| {
        b.to_async(&rt).iter(|| async {
            let resource = create_test_resource();
            black_box(provider.create_resource(resource).await.unwrap())
        })
    });
}

criterion_group!(benches, bench_resource_creation, bench_provider_operations);
criterion_main!(benches);
```

### Load Testing

```rust
// tests/performance/concurrent_access.rs
use tokio::task::JoinSet;
use scim_server::providers::InMemoryProvider;

#[tokio::test]
async fn test_concurrent_resource_creation() {
    let provider = Arc::new(InMemoryProvider::new());
    let mut join_set = JoinSet::new();
    
    // Spawn 100 concurrent resource creation tasks
    for i in 0..100 {
        let provider = Arc::clone(&provider);
        join_set.spawn(async move {
            let resource = ResourceBuilder::new()
                .id(ResourceId::new(&format!("user-{}", i)).unwrap())
                .user_name(UserName::new(&format!("user{}", i)).unwrap())
                .build()
                .unwrap();
            
            provider.create_resource(resource).await
        });
    }
    
    // Wait for all tasks to complete
    let mut results = Vec::new();
    while let Some(result) = join_set.join_next().await {
        results.push(result.unwrap().unwrap());
    }
    
    assert_eq!(results.len(), 100);
    
    // Verify all resources were created
    let all_resources = provider.list_resources(ResourceType::User).await.unwrap();
    assert_eq!(all_resources.len(), 100);
}

#[tokio::test]
async fn test_read_write_contention() {
    let provider = Arc::new(InMemoryProvider::new());
    
    // Pre-populate with test data
    for i in 0..50 {
        let resource = create_test_user_with_id(&format!("user-{}", i));
        provider.create_resource(resource).await.unwrap();
    }
    
    let mut join_set = JoinSet::new();
    
    // Spawn readers
    for _ in 0..20 {
        let provider = Arc::clone(&provider);
        join_set.spawn(async move {
            for i in 0..50 {
                let id = ResourceId::new(&format!("user-{}", i)).unwrap();
                let _resource = provider.get_resource(&id).await.unwrap();
            }
        });
    }
    
    // Spawn writers
    for i in 50..70 {
        let provider = Arc::clone(&provider);
        join_set.spawn(async move {
            let resource = create_test_user_with_id(&format!("user-{}", i));
            provider.create_resource(resource).await.unwrap();
        });
    }
    
    // Wait for completion
    while let Some(result) = join_set.join_next().await {
        result.unwrap();
    }
    
    // Verify final state
    let final_count = provider.list_resources(ResourceType::User).await.unwrap().len();
    assert_eq!(final_count, 70);
}
```

## Multi-Tenant Testing

### Tenant Isolation Testing

```rust
// tests/integration/multi_tenant.rs
use scim_server::multi_tenant::{StaticTenantResolver, TenantId, TenantContext};

#[tokio::test]
async fn test_tenant_isolation() {
    let mut resolver = StaticTenantResolver::new();
    
    // Set up two tenants with separate providers
    let tenant_a_provider = InMemoryProvider::new();
    let tenant_b_provider = InMemoryProvider::new();
    
    resolver.add_tenant(
        TenantId::new("tenant-a").unwrap(),
        TenantContext::new(ScimConfig::builder()
            .provider(tenant_a_provider)
            .build().unwrap())
    ).unwrap();
    
    resolver.add_tenant(
        TenantId::new("tenant-b").unwrap(),
        TenantContext::new(ScimConfig::builder()
            .provider(tenant_b_provider)
            .build().unwrap())
    ).unwrap();
    
    // Create user in tenant A
    let tenant_a = resolver.resolve_tenant("tenant-a").await.unwrap();
    let user_a = create_test_user();
    tenant_a.resource_provider().create_resource(user_a).await.unwrap();
    
    // Create user in tenant B
    let tenant_b = resolver.resolve_tenant("tenant-b").await.unwrap();
    let user_b = create_test_user();
    tenant_b.resource_provider().create_resource(user_b).await.unwrap();
    
    // Verify isolation - tenant A can't see tenant B's users
    let tenant_a_users = tenant_a.resource_provider()
        .list_resources(ResourceType::User).await.unwrap();
    let tenant_b_users = tenant_b.resource_provider()
        .list_resources(ResourceType::User).await.unwrap();
    
    assert_eq!(tenant_a_users.len(), 1);
    assert_eq!(tenant_b_users.len(), 1);
    assert_ne!(tenant_a_users[0].id(), tenant_b_users[0].id());
}

#[tokio::test]
async fn test_tenant_configuration_isolation() {
    let mut resolver = StaticTenantResolver::new();
    
    // Tenant A: Strict validation
    let config_a = ScimConfig::builder()
        .strict_validation(true)
        .provider(InMemoryProvider::new())
        .build().unwrap();
    
    // Tenant B: Relaxed validation
    let config_b = ScimConfig::builder()
        .strict_validation(false)
        .allow_unknown_attributes(true)
        .provider(InMemoryProvider::new())
        .build().unwrap();
    
    resolver.add_tenant(TenantId::new("strict").unwrap(), TenantContext::new(config_a)).unwrap();
    resolver.add_tenant(TenantId::new("relaxed").unwrap(), TenantContext::new(config_b)).unwrap();
    
    // Test that tenant configurations are isolated
    let strict_tenant = resolver.resolve_tenant("strict").await.unwrap();
    let relaxed_tenant = resolver.resolve_tenant("relaxed").await.unwrap();
    
    assert!(strict_tenant.config().strict_validation());
    assert!(!relaxed_tenant.config().strict_validation());
}
```

## Testing Utilities

### Test Fixtures

Create reusable test data and helpers:

```rust
// tests/fixtures/test_helpers.rs
use scim_server::resource::{Resource, ResourceBuilder};
use scim_server::resource::value_objects::*;
use serde_json::json;

pub fn create_test_user() -> Resource {
    ResourceBuilder::new()
        .id(ResourceId::new("test-user-123").unwrap())
        .user_name(UserName::new("testuser").unwrap())
        .display_name("Test User")
        .add_email(EmailAddress::new("test@example.com").unwrap())
        .active(true)
        .build()
        .unwrap()
}

pub fn create_test_group() -> Resource {
    ResourceBuilder::new()
        .id(ResourceId::new("test-group-456").unwrap())
        .display_name("Test Group")
        .group_type("security")
        .build()
        .unwrap()
}

pub fn create_test_user_json() -> serde_json::Value {
    json!({
        "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
        "id": "test-user-789",
        "userName": "testuser",
        "name": {
            "givenName": "Test",
            "familyName": "User"
        },
        "emails": [{
            "value": "test@example.com",
            "type": "work",
            "primary": true
        }],
        "active": true
    })
}

// Test server setup
pub async fn create_test_server() -> TestServer {
    TestServer::new(
        ServerConfig::builder()
            .host("localhost")
            .port(0)
            .provider(InMemoryProvider::new())
            .build()
            .unwrap()
    ).await
}

// Common assertions
pub fn assert_valid_user_resource(resource: &Resource) {
    assert!(resource.id().as_str().len() > 0);
    assert!(resource.user_name().is_some());
    assert!(resource.schemas().contains(&SchemaUri::core_user()));
}

pub fn assert_scim_error_type(error: &ScimError, expected_type: &str) {
    match error {
        ScimError::BadRequest { .. } => assert_eq!(expected_type, "BadRequest"),
        ScimError::NotFound { .. } => assert_eq!(expected_type, "NotFound"),
        ScimError::Validation { .. } => assert_eq!(expected_type, "Validation"),
        _ => panic!("Unexpected error type: {:?}", error),
    }
}
```

### Mock Providers for Testing

```rust
// Test-specific mock provider
pub struct MockProvider {
    resources: HashMap<ResourceId, Resource>,
    should_fail: bool,
    latency: Duration,
}

impl MockProvider {
    pub fn new() -> Self {
        Self {
            resources: HashMap::new(),
            should_fail: false,
            latency: Duration::from_millis(0),
        }
    }
    
    pub fn with_failure(mut self) -> Self {
        self.should_fail = true;
        self
    }
    
    pub fn with_latency(mut self, latency: Duration) -> Self {
        self.latency = latency;
        self
    }
}

#[async_trait]
impl ResourceProvider for MockProvider {
    async fn create_resource(&self, resource: Resource) -> Result<Resource> {
        // Simulate latency
        if self.latency > Duration::from_millis(0) {
            tokio::time::sleep(self.latency).await;
        }
        
        // Simulate failure
        if self.should_fail {
            return Err(ScimError::InternalServerError {
                message: "Mock failure".to_string()
            });
        }
        
        // Normal operation
        Ok(resource)
    }
    
    // ... implement other methods
}
```

### Test Database Setup

For database provider testing:

```rust
// Database test utilities
pub async fn setup_test_database() -> DatabaseProvider {
    let test_db_url = env::var("TEST_DATABASE_URL")
        .unwrap_or_else(|_| "sqlite::memory:".to_string());
    
    let provider = DatabaseProvider::new(&test_db_url).await.unwrap();
    
    // Run test migrations
    provider.run_migrations().await.unwrap();
    
    provider
}

pub async fn cleanup_test_database(provider: &DatabaseProvider) {
    // Clean up test data
    provider.truncate_all_tables().await.unwrap();
}

// Test with database cleanup
#[tokio::test]
async fn test_database_operations() {
    let provider = setup_test_database().await;
    
    // Your test logic here
    test_provider_crud(provider.clone()).await;
    
    // Cleanup
    cleanup_test_database(&provider).await;
}
```

## Testing Error Scenarios

### Error Path Testing

```rust
#[cfg(test)]
mod error_tests {
    use super::*;

    #[tokio::test]
    async fn test_resource_not_found() {
        let provider = InMemoryProvider::new();
        let non_existent_id = ResourceId::new("does-not-exist").unwrap();
        
        let result = provider.get_resource(&non_existent_id).await.unwrap();
        assert!(result.is_none());
    }

    #[