//! Integration tests for SCIM version control functionality.
//!
//! These tests verify the complete version control system including:
//! - ScimVersion creation and manipulation
//! - ConditionalResult handling
//! - HTTP ETag integration
//! - Cross-provider compatibility
//! - Concurrency scenarios

use scim_server::resource::version::{
    ConditionalResult, ScimVersion, VersionConflict, VersionError,
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
    let content_version = ScimVersion::from_content(content);

    // Hash should be deterministic
    let content_version2 = ScimVersion::from_content(content);
    assert_eq!(content_version, content_version2);

    // Different content should produce different versions
    let different_content = br#"{"id":"123","userName":"jane.doe","active":true}"#;
    let different_version = ScimVersion::from_content(different_content);
    assert_ne!(content_version, different_version);

    // Pre-computed hash versioning
    let hash_version = ScimVersion::from_hash("abc123def456");
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
        let version = ScimVersion::parse_http_header(etag_header)
            .expect(&format!("Failed to parse: {}", etag_header));

        assert_eq!(version.as_str(), expected_opaque);

        // Round-trip test: version -> etag -> version
        let generated_etag = version.to_http_header();
        let round_trip = ScimVersion::parse_http_header(&generated_etag).unwrap();
        assert_eq!(version, round_trip);
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
        let result = ScimVersion::parse_http_header(invalid_etag);
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
    let v1 = ScimVersion::from_hash("version-123");
    let v2 = ScimVersion::from_hash("version-123");
    assert!(v1.matches(&v2));
    assert!(v2.matches(&v1));

    // Different hash versions should not match
    let v3 = ScimVersion::from_hash("version-456");
    assert!(!v1.matches(&v3));
    assert!(!v3.matches(&v1));

    // Content-based versions with same content should match
    let content = b"same content";
    let h1 = ScimVersion::from_content(content);
    let h2 = ScimVersion::from_content(content);
    assert!(h1.matches(&h2));

    // Mixed version types with same opaque value should match
    let hash_v = ScimVersion::from_hash("test-123");
    let etag_v = ScimVersion::parse_http_header("\"test-123\"").unwrap();
    assert!(hash_v.matches(&etag_v));
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
    let expected_v = ScimVersion::from_hash("version1");
    let current_v = ScimVersion::from_hash("version2");
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
        ScimVersion::from_hash("version1"),
        ScimVersion::from_hash("version2"),
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
    let expected = ScimVersion::from_hash("old-version");
    let current = ScimVersion::from_hash("new-version");

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
    assert!(display_output.contains("new-version"));
    assert!(display_output.contains("Custom conflict message"));
}

/// Test version serialization and deserialization
#[tokio::test]
async fn test_version_serialization() {
    let content = br#"{"id":"123","test":"serialization"}"#;
    let original_version = ScimVersion::from_content(content);

    // JSON serialization
    let json = serde_json::to_string(&original_version).unwrap();
    let deserialized: ScimVersion = serde_json::from_str(&json).unwrap();
    assert_eq!(original_version, deserialized);

    // Test version conflict serialization
    let conflict = VersionConflict::new(
        ScimVersion::from_hash("version-v1"),
        ScimVersion::from_hash("version-v2"),
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
        version: ScimVersion,
    }

    let store: Arc<Mutex<HashMap<String, VersionedResource>>> =
        Arc::new(Mutex::new(HashMap::new()));

    // Initial resource creation
    let initial_data = json!({"id": "test-123", "userName": "initial"});
    let initial_version = ScimVersion::from_content(initial_data.to_string().as_bytes());

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
            if resource.version.matches(&expected_version) {
                let new_version = ScimVersion::from_content(update_data.to_string().as_bytes());
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
            if resource.version.matches(&old_version) {
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
        let version = ScimVersion::from_content(input);
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
    let empty_version = ScimVersion::from_content(b"");
    assert!(!empty_version.as_str().is_empty());

    // Very long content
    let long_content = "a".repeat(1000);
    let long_version = ScimVersion::from_content(long_content.as_bytes());
    assert!(!long_version.as_str().is_empty());

    // Special characters in hash string
    let special_version = ScimVersion::from_hash("version-with-special-chars!@#$%^&*()");
    let etag = special_version.to_http_header();
    let parsed = ScimVersion::parse_http_header(&etag).unwrap();
    assert_eq!(special_version, parsed);

    // Unicode content
    let unicode_content = "Hello, 世界! 🌍".as_bytes();
    let unicode_version = ScimVersion::from_content(unicode_content);
    assert!(!unicode_version.as_str().is_empty());
}

/// Test version compatibility across different creation methods
#[tokio::test]
async fn test_cross_method_compatibility() {
    let test_value = "cross-method-test-123";

    // Create version using different methods but same underlying value
    let hash_version = ScimVersion::from_hash(test_value);
    let etag_version = ScimVersion::parse_http_header(&format!("\"{}\"", test_value)).unwrap();

    // They should be equivalent
    assert!(hash_version.matches(&etag_version));
    assert!(etag_version.matches(&hash_version));
    assert_eq!(hash_version, etag_version);

    // ETag round-trip should be identical
    let etag_header = hash_version.to_http_header();
    let round_trip = ScimVersion::parse_http_header(&etag_header).unwrap();
    assert_eq!(hash_version, round_trip);
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
        let _version = ScimVersion::from_content(content.as_bytes());
    }
    let content_duration = start.elapsed();

    // Test hash string version creation performance
    let start = Instant::now();
    for i in 0..iterations {
        let hash = format!("hash-{}", i);
        let _version = ScimVersion::from_hash(&hash);
    }
    let hash_duration = start.elapsed();

    // Test ETag parsing performance
    let start = Instant::now();
    for i in 0..iterations {
        let etag = format!("\"etag-value-{}\"", i);
        let _version = ScimVersion::parse_http_header(&etag).unwrap();
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
