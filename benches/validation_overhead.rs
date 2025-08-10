//! Validation Overhead Benchmarks
//!
//! This module provides simple implementations without value object validation
//! to compare against the full implementation and measure the overhead
//! of the validation approach.

use criterion::{BenchmarkId, Criterion, Throughput, black_box, criterion_group, criterion_main};
use serde_json::{Value, json};
use std::collections::HashMap;

/// Simple resource structure without value object validation
#[derive(Debug, Clone)]
pub struct SimpleResource {
    pub resource_type: String,
    pub data: Value,
}

impl SimpleResource {
    /// Create a simple resource with minimal validation
    pub fn from_json(resource_type: String, data: Value) -> Result<Self, String> {
        // Only check that it's a valid JSON object
        if !data.is_object() {
            return Err("Data must be a JSON object".to_string());
        }

        Ok(SimpleResource {
            resource_type,
            data,
        })
    }

    /// Get the ID from the resource data
    pub fn get_id(&self) -> Option<&str> {
        self.data.get("id").and_then(|v| v.as_str())
    }

    /// Get the username from the resource data
    pub fn get_username(&self) -> Option<&str> {
        self.data.get("userName").and_then(|v| v.as_str())
    }

    /// Get the external ID from the resource data
    pub fn get_external_id(&self) -> Option<&str> {
        self.data.get("externalId").and_then(|v| v.as_str())
    }

    /// Get emails from the resource data
    pub fn get_emails(&self) -> Vec<String> {
        self.data
            .get("emails")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|email| email.get("value"))
                    .filter_map(|v| v.as_str())
                    .map(|s| s.to_string())
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Get all attributes as a reference to the internal data
    pub fn get_all_attributes(&self) -> &Value {
        &self.data
    }
}

/// Create test data for benchmarking
fn create_test_user_data(id: usize) -> Value {
    json!({
        "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
        "id": format!("user-{}", id),
        "userName": format!("user{}@example.com", id),
        "externalId": format!("ext-{}", id),
        "name": {
            "givenName": format!("User{}", id),
            "familyName": "Test",
            "formatted": format!("User{} Test", id)
        },
        "emails": [
            {
                "value": format!("user{}@example.com", id),
                "type": "work",
                "primary": true
            },
            {
                "value": format!("user{}.personal@gmail.com", id),
                "type": "personal",
                "primary": false
            }
        ],
        "phoneNumbers": [
            {
                "value": format!("+1-555-{:04}", id % 10000),
                "type": "work"
            }
        ],
        "active": true,
        "title": "Software Engineer",
        "department": "Engineering"
    })
}

/// Create a minimal test user for simple comparison
fn create_minimal_user_data(id: usize) -> Value {
    json!({
        "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
        "id": format!("user-{}", id),
        "userName": format!("user{}@example.com", id)
    })
}

/// Benchmark simple resource creation without validation
fn bench_simple_resource_creation(c: &mut Criterion) {
    let mut group = c.benchmark_group("simple_resource_creation");

    // Test with different data sizes
    for size in [1, 10, 100].iter() {
        group.throughput(Throughput::Elements(*size as u64));

        // Benchmark simple resource creation with full data
        group.bench_with_input(
            BenchmarkId::new("simple_full_data", size),
            size,
            |b, &size| {
                let test_data: Vec<Value> = (0..size).map(create_test_user_data).collect();

                b.iter(|| {
                    for data in &test_data {
                        let result =
                            SimpleResource::from_json("User".to_string(), black_box(data.clone()));
                        let _ = black_box(result);
                    }
                });
            },
        );

        // Benchmark simple resource creation with minimal data
        group.bench_with_input(
            BenchmarkId::new("simple_minimal_data", size),
            size,
            |b, &size| {
                let test_data: Vec<Value> = (0..size).map(create_minimal_user_data).collect();

                b.iter(|| {
                    for data in &test_data {
                        let result =
                            SimpleResource::from_json("User".to_string(), black_box(data.clone()));
                        let _ = black_box(result);
                    }
                });
            },
        );
    }

    group.finish();
}

/// Benchmark simple field access operations
fn bench_simple_field_access(c: &mut Criterion) {
    let mut group = c.benchmark_group("simple_field_access");

    // Create test resources
    let resources: Vec<SimpleResource> = (0..100)
        .map(|i| {
            let data = create_test_user_data(i);
            SimpleResource::from_json("User".to_string(), data).unwrap()
        })
        .collect();

    // Benchmark ID access
    group.bench_function("simple_get_id", |b| {
        b.iter(|| {
            for resource in &resources {
                let _ = black_box(resource.get_id());
            }
        });
    });

    // Benchmark username access
    group.bench_function("simple_get_username", |b| {
        b.iter(|| {
            for resource in &resources {
                let _ = black_box(resource.get_username());
            }
        });
    });

    // Benchmark external ID access
    group.bench_function("simple_get_external_id", |b| {
        b.iter(|| {
            for resource in &resources {
                let _ = black_box(resource.get_external_id());
            }
        });
    });

    // Benchmark email extraction
    group.bench_function("simple_get_emails", |b| {
        b.iter(|| {
            for resource in &resources {
                let _ = black_box(resource.get_emails());
            }
        });
    });

    // Benchmark attribute access
    group.bench_function("simple_get_attributes", |b| {
        b.iter(|| {
            for resource in &resources {
                let _ = black_box(resource.get_all_attributes());
            }
        });
    });

    group.finish();
}

/// Benchmark simple serialization
fn bench_simple_serialization(c: &mut Criterion) {
    let mut group = c.benchmark_group("simple_serialization");

    // Create test resources
    let resources: Vec<SimpleResource> = (0..100)
        .map(|i| {
            let data = create_test_user_data(i);
            SimpleResource::from_json("User".to_string(), data).unwrap()
        })
        .collect();

    // Benchmark JSON serialization
    group.bench_function("simple_to_json", |b| {
        b.iter(|| {
            for resource in &resources {
                let _ = black_box(serde_json::to_string(&resource.data));
            }
        });
    });

    group.finish();
}

/// Benchmark simple memory operations
fn bench_simple_memory(c: &mut Criterion) {
    let mut group = c.benchmark_group("simple_memory");

    // Benchmark resource cloning
    group.bench_function("simple_clone", |b| {
        let resource = {
            let data = create_test_user_data(1);
            SimpleResource::from_json("User".to_string(), data).unwrap()
        };

        b.iter(|| {
            let _ = black_box(resource.clone());
        });
    });

    // Benchmark large batch creation
    group.bench_function("simple_batch_1000", |b| {
        b.iter(|| {
            let _resources: Vec<SimpleResource> = (0..1000)
                .map(|i| {
                    let data = create_minimal_user_data(i);
                    SimpleResource::from_json("User".to_string(), data).unwrap()
                })
                .collect();
            // Results automatically consumed by iterator
        });
    });

    group.finish();
}

/// Benchmark raw JSON operations as absolute simple comparison
fn bench_raw_json_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("raw_json_operations");

    let test_data: Vec<Value> = (0..100).map(create_test_user_data).collect();

    // Benchmark pure JSON parsing and access
    group.bench_function("pure_json_access", |b| {
        b.iter(|| {
            for data in &test_data {
                if let Some(obj) = data.as_object() {
                    black_box(obj.get("id"));
                    black_box(obj.get("userName"));
                    black_box(obj.get("externalId"));
                    black_box(obj.get("emails"));
                    black_box(obj.get("title"));
                    black_box(obj.get("department"));
                }
            }
        });
    });

    // Benchmark JSON cloning
    group.bench_function("pure_json_clone", |b| {
        let data = create_test_user_data(1);
        b.iter(|| {
            black_box(data.clone());
        });
    });

    // Benchmark JSON serialization
    group.bench_function("pure_json_serialize", |b| {
        b.iter(|| {
            for data in &test_data {
                let result = serde_json::to_string(data);
                let _ = black_box(result);
            }
        });
    });

    group.finish();
}

/// Benchmark HashMap-based attribute storage as alternative simple comparison
fn bench_hashmap_storage(c: &mut Criterion) {
    let mut group = c.benchmark_group("hashmap_storage");

    // Create HashMap-based storage
    let mut hashmap_resources: Vec<HashMap<String, Value>> = Vec::new();
    for i in 0..100 {
        let data = create_test_user_data(i);
        if let Some(obj) = data.as_object() {
            let mut map = HashMap::new();
            for (k, v) in obj.iter() {
                map.insert(k.clone(), v.clone());
            }
            hashmap_resources.push(map);
        }
    }

    // Benchmark HashMap field access
    group.bench_function("hashmap_field_access", |b| {
        b.iter(|| {
            for resource in &hashmap_resources {
                black_box(resource.get("id"));
                black_box(resource.get("userName"));
                black_box(resource.get("externalId"));
                black_box(resource.get("emails"));
                black_box(resource.get("title"));
                black_box(resource.get("department"));
            }
        });
    });

    // Benchmark HashMap creation
    group.bench_function("hashmap_creation", |b| {
        let test_data: Vec<Value> = (0..100).map(create_test_user_data).collect();

        b.iter(|| {
            let mut resources: Vec<HashMap<String, Value>> = Vec::new();
            for data in &test_data {
                if let Some(obj) = data.as_object() {
                    let mut map = HashMap::new();
                    for (k, v) in obj.iter() {
                        map.insert(k.clone(), v.clone());
                    }
                    resources.push(map);
                }
            }
            black_box(resources);
        });
    });

    group.finish();
}

/// Benchmark validation overhead by comparing validated vs unvalidated operations
fn bench_validation_overhead(c: &mut Criterion) {
    let mut group = c.benchmark_group("validation_overhead");

    let test_data: Vec<Value> = (0..100).map(create_test_user_data).collect();

    // Benchmark no validation - just wrap the JSON
    group.bench_function("no_validation", |b| {
        b.iter(|| {
            let mut resources: Vec<SimpleResource> = Vec::new();
            for data in &test_data {
                // Skip validation entirely
                resources.push(SimpleResource {
                    resource_type: "User".to_string(),
                    data: data.clone(),
                });
            }
            black_box(resources);
        });
    });

    // Benchmark minimal validation - just check it's an object
    group.bench_function("minimal_validation", |b| {
        b.iter(|| {
            let mut resources: Vec<SimpleResource> = Vec::new();
            for data in &test_data {
                if data.is_object() {
                    resources.push(SimpleResource {
                        resource_type: "User".to_string(),
                        data: data.clone(),
                    });
                }
            }
            black_box(resources);
        });
    });

    // Benchmark field existence validation
    group.bench_function("field_existence_validation", |b| {
        b.iter(|| {
            let mut resources: Vec<SimpleResource> = Vec::new();
            for data in &test_data {
                if let Some(obj) = data.as_object() {
                    // Check required fields exist
                    if obj.contains_key("id") && obj.contains_key("userName") {
                        resources.push(SimpleResource {
                            resource_type: "User".to_string(),
                            data: data.clone(),
                        });
                    }
                }
            }
            black_box(resources);
        });
    });

    // Benchmark full validation
    group.bench_function("full_validation", |b| {
        b.iter(|| {
            let mut resources: Vec<SimpleResource> = Vec::new();
            for data in &test_data {
                if let Some(obj) = data.as_object() {
                    let mut valid = true;

                    // Check ID format
                    if let Some(id) = obj.get("id").and_then(|v| v.as_str()) {
                        if id.is_empty() {
                            valid = false;
                        }
                    } else {
                        valid = false;
                    }

                    // Check username format
                    if let Some(username) = obj.get("userName").and_then(|v| v.as_str()) {
                        if !username.contains('@') {
                            valid = false;
                        }
                    } else {
                        valid = false;
                    }

                    if valid {
                        resources.push(SimpleResource {
                            resource_type: "User".to_string(),
                            data: data.clone(),
                        });
                    }
                }
            }
            black_box(resources);
        });
    });

    group.finish();
}

criterion_group!(
    validation_overhead_benches,
    bench_simple_resource_creation,
    bench_simple_field_access,
    bench_simple_serialization,
    bench_simple_memory,
    bench_raw_json_operations,
    bench_hashmap_storage,
    bench_validation_overhead
);

criterion_main!(validation_overhead_benches);
