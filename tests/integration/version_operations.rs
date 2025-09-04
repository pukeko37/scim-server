//! Integration tests for SCIM version control functionality.
//!
//! These tests verify the complete version control system including:
//! - ScimVersion creation and manipulation
//! - ConditionalResult handling
//! - HTTP ETag integration
//! - Cross-provider compatibility
//! - Concurrency scenarios

use scim_server::{
    ScimOperationHandler, ScimServer, create_user_resource_handler,
    operation_handler::ScimOperationRequest,
    providers::StandardResourceProvider,
    resource::version::{
        ConditionalResult, HttpVersion, RawVersion, VersionConflict, VersionError,
    },
    storage::InMemoryStorage,
};
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

/// Test ScimVersion creation from content
#[tokio::test]
async fn test_version_creation_methods() {
    // Content-based versioning
    let content = br#"{"id":"123","userName":"john.doe","active":true}"#;
    let content_version = RawVersion::from_content(content);

    // Hash should be deterministic
    let content_version2 = RawVersion::from_content(content);
    assert_eq!(content_version, content_version2);

    // Different content should produce different versions
    let different_content = br#"{"id":"123","userName":"jane.doe","active":true}"#;
    let different_version = RawVersion::from_content(different_content);
    assert_ne!(content_version, different_version);

    // Pre-computed hash versioning
    let hash_version = RawVersion::from_hash("abc123def456");
    assert_eq!(hash_version.as_str(), "abc123def456");
}

/// Test HTTP ETag parsing and generation
#[tokio::test]
async fn test_etag_http_integration() {
    let test_cases = vec![
        // Strong ETags
        ("\"simple-version\"", "simple-version"),
        ("\"v1.2.3\"", "v1.2.3"),
        ("\"abc123def456\"", "abc123def456"),
        // Weak ETags (should be handled the same)
        ("W/\"weak-version\"", "weak-version"),
        ("W/\"another-weak\"", "another-weak"),
    ];

    for (etag_header, expected_opaque) in test_cases {
        let version: HttpVersion = etag_header
            .parse()
            .expect(&format!("Failed to parse: {}", etag_header));

        assert_eq!(version.as_str(), expected_opaque);
    }
}

/// Test invalid ETag formats
#[tokio::test]
async fn test_invalid_etag_formats() {
    let invalid_etags = vec![
        "no-quotes",
        "\"",
        "\"\"", // Empty quotes
        "\"unclosed",
        "unclosed\"",
        "",
        "W/\"\"", // Empty weak ETag
        "W/unclosed\"",
    ];

    for invalid_etag in invalid_etags {
        let result: Result<HttpVersion, _> = invalid_etag.parse();
        assert!(
            result.is_err(),
            "Should reject invalid ETag: {}",
            invalid_etag
        );

        match result.unwrap_err() {
            VersionError::InvalidEtagFormat(msg) => {
                assert_eq!(msg, invalid_etag);
            }
            _ => panic!("Expected InvalidEtagFormat error"),
        }
    }
}

/// Test version matching and comparison
#[tokio::test]
async fn test_version_matching() {
    // Identical hash versions should match
    let v1 = RawVersion::from_hash("version-123");
    let v2 = RawVersion::from_hash("version-123");
    assert!(v1 == v2);
    assert!(v2 == v1);

    // Different hash versions should not match
    let v3 = RawVersion::from_hash("version-456");
    assert!(v1 != v3);
    assert!(v3 != v1);

    // Content-based versions with same content should match
    let content = b"same content";
    let h1 = RawVersion::from_content(content);
    let h2 = RawVersion::from_content(content);
    assert!(h1 == h2);

    // Mixed version types with same opaque value should match
    let hash_v = RawVersion::from_hash("test-123");
    let etag_v: HttpVersion = "\"test-123\"".parse().unwrap();
    assert!(hash_v == etag_v);
}

/// Test HTTP interface conversion between ETag headers and internal raw versions
#[tokio::test]
async fn test_http_interface_version_conversion() {
    let storage = InMemoryStorage::new();
    let provider = StandardResourceProvider::new(storage);
    let mut server = ScimServer::new(provider).unwrap();

    // Register User resource type
    let user_schema = server
        .get_schema_by_id("urn:ietf:params:scim:schemas:core:2.0:User")
        .unwrap()
        .clone();
    let user_handler = create_user_resource_handler(user_schema);
    server
        .register_resource_type(
            "User",
            user_handler,
            vec![
                scim_server::multi_tenant::ScimOperation::Create,
                scim_server::multi_tenant::ScimOperation::Read,
                scim_server::multi_tenant::ScimOperation::Update,
            ],
        )
        .unwrap();

    let operation_handler = ScimOperationHandler::new(server);

    // Test 1: Create user and verify raw version is returned in response
    let create_request = ScimOperationRequest::create(
        "User".to_string(),
        json!({
            "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
            "userName": "http.test@example.com",
            "active": true
        }),
    );

    let create_response = operation_handler.handle_operation(create_request).await;
    assert!(create_response.success, "Create should succeed");

    let created_user = create_response.data.unwrap();
    let user_id = created_user["id"].as_str().unwrap();
    let raw_version = created_user["meta"]["version"].as_str().unwrap();

    // Verify version is in raw format (no W/" wrapper)
    assert!(
        !raw_version.starts_with("W/\""),
        "Response should contain raw version, not ETag format"
    );
    assert!(
        !raw_version.starts_with("\""),
        "Response should contain raw version, not quoted format"
    );

    // Test 2: Simulate HTTP client sending ETag header for conditional update
    // HTTP client would extract version from response and send as ETag header
    let etag_header = format!("W/\"{}\"", raw_version);

    // Parse the ETag header (simulating HTTP interface layer)
    let parsed_version = etag_header
        .parse::<HttpVersion>()
        .expect("Should parse valid ETag header");

    // Verify the parsed version matches the original raw version
    assert_eq!(
        parsed_version.as_str(),
        raw_version,
        "ETag parsing should extract correct raw version"
    );

    // Test 3: Use parsed version for conditional update
    let update_request = ScimOperationRequest::update(
        "User".to_string(),
        user_id.to_string(),
        json!({
            "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
            "userName": "http.updated@example.com",
            "active": true
        }),
    )
    .with_expected_version(parsed_version);

    let update_response = operation_handler.handle_operation(update_request).await;
    assert!(
        update_response.success,
        "Conditional update with correct ETag should succeed"
    );

    let updated_user = update_response.data.unwrap();
    let new_raw_version = updated_user["meta"]["version"].as_str().unwrap();

    // Verify new version is different but still in raw format
    assert_ne!(
        new_raw_version, raw_version,
        "Version should change after update"
    );
    assert!(
        !new_raw_version.starts_with("W/\""),
        "Updated response should contain raw version"
    );

    // Test 4: Test conversion from raw to ETag for HTTP response
    let new_version_obj: RawVersion = new_raw_version.parse().unwrap();
    let response_etag = HttpVersion::from(new_version_obj.clone()).to_string();

    // Verify ETag generation for HTTP response
    assert!(
        response_etag.starts_with("W/\""),
        "Generated ETag should have weak format"
    );
    assert!(
        response_etag.ends_with("\""),
        "Generated ETag should be properly quoted"
    );
    assert!(
        response_etag.contains(new_raw_version),
        "Generated ETag should contain raw version"
    );

    // Test 5: Round-trip conversion should be stable
    let round_trip_version: HttpVersion = response_etag.parse().unwrap();
    assert_eq!(
        round_trip_version.as_str(),
        new_raw_version,
        "Round-trip conversion should preserve version"
    );
}

/// Test that stale ETag headers are properly rejected
#[tokio::test]
async fn test_http_stale_etag_rejection() {
    let storage = InMemoryStorage::new();
    let provider = StandardResourceProvider::new(storage);
    let mut server = ScimServer::new(provider).unwrap();

    // Register User resource type
    let user_schema = server
        .get_schema_by_id("urn:ietf:params:scim:schemas:core:2.0:User")
        .unwrap()
        .clone();
    let user_handler = create_user_resource_handler(user_schema);
    server
        .register_resource_type(
            "User",
            user_handler,
            vec![
                scim_server::multi_tenant::ScimOperation::Create,
                scim_server::multi_tenant::ScimOperation::Update,
            ],
        )
        .unwrap();

    let operation_handler = ScimOperationHandler::new(server);

    // Create user
    let create_request = ScimOperationRequest::create(
        "User".to_string(),
        json!({
            "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
            "userName": "stale.test@example.com",
            "active": true
        }),
    );

    let create_response = operation_handler.handle_operation(create_request).await;
    let created_user = create_response.data.unwrap();
    let user_id = created_user["id"].as_str().unwrap();
    let original_version = created_user["meta"]["version"].as_str().unwrap();

    // First update to change the version
    let first_update = ScimOperationRequest::update(
        "User".to_string(),
        user_id.to_string(),
        json!({
            "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
            "userName": "first.update@example.com",
            "active": true
        }),
    );

    let first_response = operation_handler.handle_operation(first_update).await;
    assert!(first_response.success);

    // Now try to use the original (stale) version in ETag format
    let stale_etag = format!("W/\"{}\"", original_version);
    let stale_version: HttpVersion = stale_etag.parse().unwrap();

    let stale_update = ScimOperationRequest::update(
        "User".to_string(),
        user_id.to_string(),
        json!({
            "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
            "userName": "should.fail@example.com",
            "active": false
        }),
    )
    .with_expected_version(stale_version);

    let stale_response = operation_handler.handle_operation(stale_update).await;

    // Should fail due to version mismatch
    assert!(
        !stale_response.success,
        "Update with stale ETag should fail"
    );
    assert!(
        stale_response
            .error
            .unwrap()
            .contains("modified by another client"),
        "Should indicate version conflict"
    );
}

/// Test various ETag formats that HTTP clients might send
#[tokio::test]
async fn test_http_etag_format_compatibility() {
    let test_cases = vec![
        // Standard weak ETags
        ("W/\"abc123\"", "abc123"),
        ("W/\"version-1.0\"", "version-1.0"),
        ("W/\"2023-01-01T00:00:00Z\"", "2023-01-01T00:00:00Z"),
        // Strong ETags (should be handled the same)
        ("\"strong-etag\"", "strong-etag"),
        ("\"base64+encoded/value=\"", "base64+encoded/value="),
        // Base64-style versions (common in SCIM)
        ("W/\"dGVzdC12ZXJzaW9u\"", "dGVzdC12ZXJzaW9u"),
        ("W/\"SGVsbG8gV29ybGQ=\"", "SGVsbG8gV29ybGQ="),
    ];

    for (etag_header, expected_raw) in test_cases {
        // Test ETag parsing
        let version: HttpVersion = etag_header
            .parse()
            .expect(&format!("Should parse ETag: {}", etag_header));

        assert_eq!(
            version.as_str(),
            expected_raw,
            "ETag {} should extract raw version {}",
            etag_header,
            expected_raw
        );

        // Test round-trip: raw -> ETag -> raw
        let raw_version: RawVersion = expected_raw.parse().unwrap();
        let generated_etag = HttpVersion::from(raw_version.clone()).to_string();
        let round_trip: HttpVersion = generated_etag.parse().unwrap();

        assert_eq!(
            round_trip.as_str(),
            expected_raw,
            "Round-trip should preserve raw version for {}",
            etag_header
        );
        assert!(
            version == round_trip,
            "Original and round-trip versions should match for {}",
            etag_header
        );
    }
}

/// Test ConditionalResult operations
#[tokio::test]
async fn test_conditional_result_operations() {
    // Success case
    let success_data = json!({"id": "123", "userName": "john.doe"});
    let success: ConditionalResult<serde_json::Value> =
        ConditionalResult::Success(success_data.clone());

    assert!(success.is_success());
    assert!(!success.is_version_mismatch());
    assert!(!success.is_not_found());
    assert_eq!(success.clone().into_success(), Some(success_data));

    // Version mismatch case
    let expected_v = RawVersion::from_hash("version1");
    let current_v = RawVersion::from_hash("version2");
    let conflict = VersionConflict::standard_message(expected_v.clone(), current_v.clone());
    let mismatch: ConditionalResult<serde_json::Value> =
        ConditionalResult::VersionMismatch(conflict.clone());

    assert!(!mismatch.is_success());
    assert!(mismatch.is_version_mismatch());
    assert!(!mismatch.is_not_found());
    assert_eq!(mismatch.into_version_conflict(), Some(conflict));

    // Not found case
    let not_found: ConditionalResult<serde_json::Value> = ConditionalResult::NotFound;
    assert!(!not_found.is_success());
    assert!(!not_found.is_version_mismatch());
    assert!(not_found.is_not_found());
}

/// Test ConditionalResult mapping
#[tokio::test]
async fn test_conditional_result_mapping() {
    // Map success value
    let success: ConditionalResult<i32> = ConditionalResult::Success(42);
    let mapped = success.map(|x| x.to_string());
    assert_eq!(mapped.into_success(), Some("42".to_string()));

    // Map preserves non-success variants
    let conflict = ConditionalResult::<i32>::VersionMismatch(VersionConflict::new(
        RawVersion::from_hash("version1"),
        RawVersion::from_hash("version2"),
        "test conflict",
    ));
    let mapped_conflict = conflict.map(|x| x.to_string());
    assert!(mapped_conflict.is_version_mismatch());

    let not_found: ConditionalResult<i32> = ConditionalResult::NotFound;
    let mapped_not_found = not_found.map(|x| x.to_string());
    assert!(mapped_not_found.is_not_found());
}

/// Test VersionConflict creation and formatting
#[tokio::test]
async fn test_version_conflict() {
    let expected = RawVersion::from_hash("old-version");
    let current = RawVersion::from_hash("current-version");

    // Custom conflict message
    let custom_conflict =
        VersionConflict::new(expected.clone(), current.clone(), "Custom conflict message");

    assert_eq!(custom_conflict.expected, expected);
    assert_eq!(custom_conflict.current, current);
    assert_eq!(custom_conflict.message, "Custom conflict message");

    // Standard conflict message
    let standard_conflict = VersionConflict::standard_message(expected.clone(), current.clone());
    assert!(!standard_conflict.message.is_empty());
    assert!(standard_conflict.message.contains("modified"));

    // Display formatting
    let display_output = format!("{}", custom_conflict);
    assert!(display_output.contains("old-version"));
    assert!(display_output.contains("current-version"));
    assert!(display_output.contains("Custom conflict message"));
}

/// Test version serialization and deserialization
#[tokio::test]
async fn test_version_serialization() {
    let content = br#"{"id":"123","test":"serialization"}"#;
    let original_version = RawVersion::from_content(content);

    // JSON serialization
    let json = serde_json::to_string(&original_version).unwrap();
    let deserialized: RawVersion = serde_json::from_str(&json).unwrap();
    assert_eq!(original_version, deserialized);

    // Test version conflict serialization
    let conflict = VersionConflict::new(
        RawVersion::from_hash("version-v1"),
        RawVersion::from_hash("version-v2"),
        "Serialization test conflict",
    );

    let conflict_json = serde_json::to_string(&conflict).unwrap();
    let deserialized_conflict: VersionConflict = serde_json::from_str(&conflict_json).unwrap();
    assert_eq!(conflict, deserialized_conflict);
}

/// Test concurrent version operations simulation
#[tokio::test]
async fn test_concurrent_version_scenarios() {
    // Simulate a simple in-memory store with version tracking
    #[derive(Clone)]
    struct VersionedResource {
        version: RawVersion,
    }

    let store: Arc<Mutex<HashMap<String, VersionedResource>>> =
        Arc::new(Mutex::new(HashMap::new()));

    // Initial resource creation
    let initial_data = json!({"id": "test-123", "userName": "initial"});
    let initial_version = RawVersion::from_content(initial_data.to_string().as_bytes());

    {
        let mut store_guard = store.lock().await;
        store_guard.insert(
            "test-123".to_string(),
            VersionedResource {
                version: initial_version.clone(),
            },
        );
    }

    // Simulate successful conditional update
    let update_data = json!({"id": "test-123", "userName": "updated"});
    let expected_version = initial_version.clone();

    let update_result = {
        let mut store_guard = store.lock().await;
        if let Some(resource) = store_guard.get("test-123") {
            if resource.version == expected_version {
                let new_version = RawVersion::from_content(update_data.to_string().as_bytes());
                store_guard.insert(
                    "test-123".to_string(),
                    VersionedResource {
                        version: new_version.clone(),
                    },
                );
                ConditionalResult::Success(update_data.clone())
            } else {
                ConditionalResult::VersionMismatch(VersionConflict::standard_message(
                    expected_version,
                    resource.version.clone(),
                ))
            }
        } else {
            ConditionalResult::NotFound
        }
    };

    assert!(update_result.is_success());

    // Simulate version conflict (using old version)
    let conflict_data = json!({"id": "test-123", "userName": "conflicted"});
    let old_version = initial_version; // This should cause a conflict

    let conflict_result = {
        let store_guard = store.lock().await;
        if let Some(resource) = store_guard.get("test-123") {
            if resource.version == old_version {
                ConditionalResult::Success(conflict_data.clone())
            } else {
                ConditionalResult::VersionMismatch(VersionConflict::standard_message(
                    old_version,
                    resource.version.clone(),
                ))
            }
        } else {
            ConditionalResult::NotFound
        }
    };

    assert!(conflict_result.is_version_mismatch());
    if let Some(conflict) = conflict_result.into_version_conflict() {
        assert!(conflict.message.contains("modified"));
    }
}

/// Test hash collision resistance (basic test)
#[tokio::test]
async fn test_hash_collision_resistance() {
    let mut versions = std::collections::HashSet::new();

    // Generate versions from different inputs
    let test_inputs = vec![
        b"user1".as_slice(),
        b"user2".as_slice(),
        b"user3".as_slice(),
        br#"{"id":"1","name":"Alice"}"#.as_slice(),
        br#"{"id":"2","name":"Bob"}"#.as_slice(),
        br#"{"id":"1","name":"Bob"}"#.as_slice(), // Same ID, different name
        br#"{"id":"2","name":"Alice"}"#.as_slice(), // Different ID, same name
    ];

    for input in test_inputs {
        let version = RawVersion::from_content(input);
        assert!(
            versions.insert(version.as_str().to_string()),
            "Hash collision detected for input: {:?}",
            std::str::from_utf8(input).unwrap_or("(invalid utf8)")
        );
    }

    assert_eq!(versions.len(), 7, "All versions should be unique");
}

/// Test version operations with edge cases
#[tokio::test]
async fn test_version_edge_cases() {
    // Empty content hash
    let empty_version = RawVersion::from_content(b"");
    assert!(!empty_version.as_str().is_empty());

    // Very long content
    let long_content = "a".repeat(1000);
    let long_version = RawVersion::from_content(long_content.as_bytes());
    assert!(!long_version.as_str().is_empty());

    // Special characters in hash string
    let special_version = RawVersion::from_hash("version-with-special-chars!@#$%^&*()");
    let etag = HttpVersion::from(special_version.clone()).to_string();
    let parsed: HttpVersion = etag.parse().unwrap();
    assert_eq!(special_version, parsed);

    // Unicode content
    let unicode_content = "Hello, 世界! 🌍".as_bytes();
    let unicode_version = RawVersion::from_content(unicode_content);
    assert!(!unicode_version.as_str().is_empty());
}

/// Benchmark-style test for version operation performance
#[tokio::test]
async fn test_version_performance_characteristics() {
    use std::time::Instant;

    let iterations = 1000;

    // Test content-based version creation performance
    let start = Instant::now();
    for i in 0..iterations {
        let content = format!("test-content-{}", i);
        let _version = RawVersion::from_content(content.as_bytes());
    }
    let content_duration = start.elapsed();

    // Test hash string version creation performance
    let start = Instant::now();
    for i in 0..iterations {
        let hash = format!("hash-{}", i);
        let _version = RawVersion::from_hash(&hash);
    }
    let hash_duration = start.elapsed();

    // Test ETag parsing performance
    let start = Instant::now();
    for i in 0..iterations {
        let etag = format!("\"etag-value-{}\"", i);
        let _version: HttpVersion = etag.parse().unwrap();
    }
    let parse_duration = start.elapsed();

    // Performance should be reasonable (adjust thresholds as needed)
    println!(
        "Content hashing: {:?} for {} iterations",
        content_duration, iterations
    );
    println!(
        "Hash string creation: {:?} for {} iterations",
        hash_duration, iterations
    );
    println!(
        "ETag parsing: {:?} for {} iterations",
        parse_duration, iterations
    );

    // Basic performance assertions (very generous thresholds)
    assert!(
        content_duration.as_millis() < 1000,
        "Content hashing should be fast"
    );
    assert!(
        hash_duration.as_millis() < 100,
        "Hash string creation should be very fast"
    );
    assert!(
        parse_duration.as_millis() < 200,
        "ETag parsing should be fast"
    );
}
