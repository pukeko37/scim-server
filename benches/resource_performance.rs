//! Resource Performance Benchmarks
//!
//! This benchmark suite measures the performance characteristics of
//! Resource creation and validation operations.

use criterion::{BenchmarkId, Criterion, Throughput, black_box, criterion_group, criterion_main};
use scim_server::resource::core::Resource;
use scim_server::schema::registry::SchemaRegistry;
use serde_json::{Value, json};
// use std::collections::HashMap; // Unused import

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

/// Create a minimal test user for baseline comparison
fn create_minimal_user_data(id: usize) -> Value {
    json!({
        "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
        "id": format!("user-{}", id),
        "userName": format!("user{}@example.com", id)
    })
}

/// Create test data with invalid values for error path testing
fn create_invalid_user_data(_id: usize) -> Value {
    json!({
        "schemas": [""],  // Invalid schema
        "id": "",         // Invalid ID
        "userName": "",   // Invalid username
        "externalId": ""  // Invalid external ID
    })
}

/// Benchmark Resource creation with value object validation
fn bench_resource_creation(c: &mut Criterion) {
    let mut group = c.benchmark_group("resource_creation");

    // Test with different data sizes
    for size in [1, 10, 100].iter() {
        group.throughput(Throughput::Elements(*size as u64));

        // Benchmark successful resource creation
        group.bench_with_input(
            BenchmarkId::new("success_full_data", size),
            size,
            |b, &size| {
                let test_data: Vec<Value> = (0..size).map(create_test_user_data).collect();

                b.iter(|| {
                    for data in &test_data {
                        let result =
                            Resource::from_json("User".to_string(), black_box(data.clone()));
                        let _ = black_box(result);
                    }
                });
            },
        );

        // Benchmark with minimal data
        group.bench_with_input(
            BenchmarkId::new("success_minimal_data", size),
            size,
            |b, &size| {
                let test_data: Vec<Value> = (0..size).map(create_minimal_user_data).collect();

                b.iter(|| {
                    for data in &test_data {
                        let result =
                            Resource::from_json("User".to_string(), black_box(data.clone()));
                        let _ = black_box(result);
                    }
                });
            },
        );

        // Benchmark validation failures
        group.bench_with_input(
            BenchmarkId::new("validation_failures", size),
            size,
            |b, &size| {
                let test_data: Vec<Value> = (0..size).map(create_invalid_user_data).collect();

                b.iter(|| {
                    for data in &test_data {
                        let result =
                            Resource::from_json("User".to_string(), black_box(data.clone()));
                        let _ = black_box(result);
                    }
                });
            },
        );
    }

    group.finish();
}

/// Benchmark value object extraction and validation
fn bench_value_object_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("value_object_operations");

    // Create test resources
    let resources: Vec<Resource> = (0..100)
        .map(|i| {
            let data = create_test_user_data(i);
            Resource::from_json("User".to_string(), data).unwrap()
        })
        .collect();

    // Benchmark ID access
    group.bench_function("get_id_access", |b| {
        b.iter(|| {
            for resource in &resources {
                black_box(resource.get_id());
            }
        });
    });

    // Benchmark username access
    group.bench_function("get_username_access", |b| {
        b.iter(|| {
            for resource in &resources {
                black_box(resource.get_username());
            }
        });
    });

    // Benchmark external ID access
    group.bench_function("get_external_id_access", |b| {
        b.iter(|| {
            for resource in &resources {
                black_box(resource.get_external_id());
            }
        });
    });

    // Benchmark email extraction
    group.bench_function("get_emails_extraction", |b| {
        b.iter(|| {
            for resource in &resources {
                black_box(resource.get_emails());
            }
        });
    });

    // Benchmark attribute access
    group.bench_function("get_attribute_access", |b| {
        b.iter(|| {
            for resource in &resources {
                black_box(resource.get_attribute("title"));
                black_box(resource.get_attribute("department"));
            }
        });
    });

    group.finish();
}

/// Benchmark schema validation integration
fn bench_schema_validation(c: &mut Criterion) {
    let mut group = c.benchmark_group("schema_validation");

    let registry = SchemaRegistry::new().expect("Failed to create schema registry");

    // Create test resources
    let resources: Vec<Resource> = (0..100)
        .map(|i| {
            let data = create_test_user_data(i);
            Resource::from_json("User".to_string(), data).unwrap()
        })
        .collect();

    // Benchmark hybrid validation
    group.bench_function("hybrid_validation", |b| {
        b.iter(|| {
            for resource in &resources {
                let result = registry.validate_resource_hybrid(black_box(resource));
                let _ = black_box(result);
            }
        });
    });

    group.finish();
}

/// Benchmark serialization performance
fn bench_serialization(c: &mut Criterion) {
    let mut group = c.benchmark_group("serialization");

    // Create test resources
    let resources: Vec<Resource> = (0..100)
        .map(|i| {
            let data = create_test_user_data(i);
            Resource::from_json("User".to_string(), data).unwrap()
        })
        .collect();

    // Benchmark JSON serialization
    group.bench_function("to_json", |b| {
        b.iter(|| {
            for resource in &resources {
                let _ = black_box(resource.to_json());
            }
        });
    });

    // Benchmark serde serialization
    group.bench_function("serde_serialize", |b| {
        b.iter(|| {
            for resource in &resources {
                let result = serde_json::to_string(black_box(resource));
                let _ = black_box(result);
            }
        });
    });

    group.finish();
}

/// Benchmark memory usage patterns
fn bench_memory_patterns(c: &mut Criterion) {
    let mut group = c.benchmark_group("memory_patterns");

    // Benchmark resource cloning
    group.bench_function("resource_clone", |b| {
        let resource = {
            let data = create_test_user_data(1);
            Resource::from_json("User".to_string(), data).unwrap()
        };

        b.iter(|| {
            black_box(resource.clone());
        });
    });

    // Benchmark large batch creation
    group.bench_function("batch_creation_1000", |b| {
        b.iter(|| {
            let _resources: Vec<Resource> = (0..1000)
                .map(|i| {
                    let data = create_minimal_user_data(i);
                    Resource::from_json("User".to_string(), data).unwrap()
                })
                .collect();
        });
    });

    group.finish();
}

/// Benchmark comparison with raw JSON operations
fn bench_raw_json_comparison(c: &mut Criterion) {
    let mut group = c.benchmark_group("raw_json_comparison");

    let test_data: Vec<Value> = (0..100).map(create_test_user_data).collect();

    // Benchmark raw JSON parsing (baseline)
    group.bench_function("raw_json_parse_only", |b| {
        b.iter(|| {
            for data in &test_data {
                // Just parse and access fields without validation
                if let Some(obj) = data.as_object() {
                    black_box(obj.get("id"));
                    black_box(obj.get("userName"));
                    black_box(obj.get("externalId"));
                }
            }
        });
    });

    // Benchmark Resource creation (with validation)
    group.bench_function("resource_creation_with_validation", |b| {
        b.iter(|| {
            for data in &test_data {
                let result = Resource::from_json("User".to_string(), data.clone());
                if let Ok(resource) = result {
                    black_box(resource.get_id());
                    black_box(resource.get_username());
                    black_box(resource.get_external_id());
                }
            }
        });
    });

    group.finish();
}

/// Benchmark concurrent operations
fn bench_concurrent_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("concurrent_operations");

    // use std::sync::Arc; // Unused import
    use std::thread;

    // Benchmark concurrent resource creation
    group.bench_function("concurrent_creation", |b| {
        b.iter(|| {
            let handles: Vec<_> = (0..4)
                .map(|thread_id| {
                    thread::spawn(move || {
                        let mut results = Vec::new();
                        for i in 0..25 {
                            let data = create_test_user_data(thread_id * 25 + i);
                            let result = Resource::from_json("User".to_string(), data);
                            results.push(result);
                        }
                        results
                    })
                })
                .collect();

            for handle in handles {
                black_box(handle.join().unwrap());
            }
        });
    });

    group.finish();
}

/// Benchmark error handling overhead
fn bench_error_handling(c: &mut Criterion) {
    let mut group = c.benchmark_group("error_handling");

    // Create mixed valid/invalid data
    let mixed_data: Vec<Value> = (0..100)
        .map(|i| {
            if i % 2 == 0 {
                create_test_user_data(i)
            } else {
                create_invalid_user_data(i)
            }
        })
        .collect();

    // Benchmark error handling patterns
    group.bench_function("mixed_success_failure", |b| {
        b.iter(|| {
            let mut success_count = 0;
            let mut error_count = 0;

            for data in &mixed_data {
                match Resource::from_json("User".to_string(), data.clone()) {
                    Ok(_) => success_count += 1,
                    Err(_) => error_count += 1,
                }
            }

            black_box((success_count, error_count));
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_resource_creation,
    bench_value_object_operations,
    bench_schema_validation,
    bench_serialization,
    bench_memory_patterns,
    bench_raw_json_comparison,
    bench_concurrent_operations,
    bench_error_handling
);

criterion_main!(benches);
