//! Test Coverage Analyzer
//!
//! This tool analyzes the test suite to categorize tests as either:
//! - Production code tests (testing actual application code)
//! - Test infrastructure tests (testing mocks, utilities, etc.)
//!
//! Usage: cargo run --bin test-coverage-analyzer

use std::collections::HashMap;
use std::fs;
use std::path::Path;

#[derive(Debug, Clone)]
struct TestCategory {
    name: String,
    _description: String,
    production_ready: bool,
}

#[derive(Debug)]
struct TestFile {
    path: String,
    tests: Vec<TestFunction>,
    category: TestCategory,
}

#[derive(Debug)]
struct TestFunction {
    name: String,
    line_number: usize,
    is_production: bool,
}

#[derive(Debug)]
struct CoverageReport {
    total_tests: usize,
    production_tests: usize,
    infrastructure_tests: usize,
    files: Vec<TestFile>,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🔍 SCIM Server Test Coverage Analyzer");
    println!("{}", "=".repeat(50));

    let report = analyze_test_coverage()?;
    print_coverage_report(&report);

    Ok(())
}

fn analyze_test_coverage() -> Result<CoverageReport, Box<dyn std::error::Error>> {
    let mut files = Vec::new();
    let mut total_tests = 0;
    let mut production_tests = 0;
    let mut infrastructure_tests = 0;

    // Analyze src/ tests (production code)
    analyze_directory("src", &mut files, true)?;

    // Analyze tests/ directory (mixed)
    analyze_directory("tests", &mut files, false)?;

    // Count tests and categorize
    for file in &files {
        for test in &file.tests {
            total_tests += 1;
            if test.is_production {
                production_tests += 1;
            } else {
                infrastructure_tests += 1;
            }
        }
    }

    Ok(CoverageReport {
        total_tests,
        production_tests,
        infrastructure_tests,
        files,
    })
}

fn analyze_directory(
    dir: &str,
    files: &mut Vec<TestFile>,
    default_production: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    if !Path::new(dir).exists() {
        return Ok(());
    }

    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_dir() {
            if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                let subdir = format!("{}/{}", dir, name);
                analyze_directory(&subdir, files, default_production)?;
            }
        } else if path.extension().and_then(|s| s.to_str()) == Some("rs") {
            if let Some(file) = analyze_rust_file(&path, default_production)? {
                files.push(file);
            }
        }
    }

    Ok(())
}

fn analyze_rust_file(
    path: &Path,
    _default_production: bool,
) -> Result<Option<TestFile>, Box<dyn std::error::Error>> {
    let content = fs::read_to_string(path)?;
    let path_str = path.to_string_lossy().to_string();

    // Skip if no tests
    if !content.contains("#[test]") && !content.contains("#[tokio::test]") {
        return Ok(None);
    }

    let category = categorize_file(&path_str, &content);
    let tests = extract_tests(&content, &category);

    if tests.is_empty() {
        return Ok(None);
    }

    Ok(Some(TestFile {
        path: path_str,
        tests,
        category,
    }))
}

fn categorize_file(path: &str, _content: &str) -> TestCategory {
    // Production code tests (in src/)
    if path.starts_with("src/") {
        if path.contains("multi_tenant") {
            TestCategory {
                name: "Multi-Tenant Production".to_string(),
                _description: "Tests for production multi-tenant functionality".to_string(),
                production_ready: true,
            }
        } else {
            TestCategory {
                name: "Core Production".to_string(),
                _description: "Tests for core SCIM functionality".to_string(),
                production_ready: true,
            }
        }
    }
    // Phase 1 integration tests
    else if path.contains("phase1_integration") {
        TestCategory {
            name: "Phase 1 Integration".to_string(),
            _description: "End-to-end tests using production APIs".to_string(),
            production_ready: true,
        }
    }
    // Test infrastructure and utilities
    else if path.contains("tests/common") {
        TestCategory {
            name: "Test Infrastructure".to_string(),
            _description: "Test utilities, builders, and fixtures".to_string(),
            production_ready: false,
        }
    }
    // Mock-based integration tests
    else if path.contains("tests/integration") {
        TestCategory {
            name: "Mock Integration".to_string(),
            _description: "Integration tests using mocks and test-only code".to_string(),
            production_ready: false,
        }
    }
    // SCIM validation tests (production schemas, but not multi-tenant)
    else if path.contains("validation") {
        TestCategory {
            name: "SCIM Validation".to_string(),
            _description: "Production SCIM schema and validation tests".to_string(),
            production_ready: true,
        }
    }
    // Default category
    else {
        TestCategory {
            name: "Other".to_string(),
            _description: "Miscellaneous tests".to_string(),
            production_ready: false,
        }
    }
}

fn extract_tests(content: &str, category: &TestCategory) -> Vec<TestFunction> {
    let mut tests = Vec::new();
    let lines: Vec<&str> = content.lines().collect();

    for (i, line) in lines.iter().enumerate() {
        let trimmed = line.trim();

        // Look for test annotations
        if trimmed.starts_with("#[test]") || trimmed.starts_with("#[tokio::test]") {
            // Find the function name on the next line(s)
            for j in (i + 1)..std::cmp::min(i + 5, lines.len()) {
                let func_line = lines[j].trim();
                if func_line.starts_with("fn ") {
                    if let Some(name) = extract_function_name(func_line) {
                        tests.push(TestFunction {
                            name,
                            line_number: i + 1,
                            is_production: category.production_ready,
                        });
                        break;
                    }
                }
            }
        }
    }

    tests
}

fn extract_function_name(line: &str) -> Option<String> {
    // Extract function name from "fn test_name(" or "async fn test_name("
    let parts: Vec<&str> = line.split_whitespace().collect();
    for i in 0..parts.len() {
        if parts[i] == "fn" && i + 1 < parts.len() {
            let name_part = parts[i + 1];
            if let Some(paren_pos) = name_part.find('(') {
                return Some(name_part[..paren_pos].to_string());
            }
        }
    }
    None
}

fn print_coverage_report(report: &CoverageReport) {
    println!("\n📊 Test Coverage Summary");
    println!("{}", "-".repeat(50));
    println!("Total Tests: {}", report.total_tests);
    println!(
        "Production Tests: {} ({:.1}%)",
        report.production_tests,
        (report.production_tests as f64 / report.total_tests as f64) * 100.0
    );
    println!(
        "Infrastructure Tests: {} ({:.1}%)",
        report.infrastructure_tests,
        (report.infrastructure_tests as f64 / report.total_tests as f64) * 100.0
    );

    // Group by category
    let mut category_stats: HashMap<String, (usize, usize)> = HashMap::new();

    for file in &report.files {
        let entry = category_stats
            .entry(file.category.name.clone())
            .or_insert((0, 0));
        for test in &file.tests {
            entry.0 += 1; // total
            if test.is_production {
                entry.1 += 1; // production
            }
        }
    }

    println!("\n📋 By Category");
    println!("{}", "-".repeat(50));

    for (category, (total, production)) in &category_stats {
        let production_pct = if *total > 0 {
            (*production as f64 / *total as f64) * 100.0
        } else {
            0.0
        };

        let status = if *production > 0 {
            "✅ PRODUCTION"
        } else {
            "⚠️  TEST-ONLY"
        };
        println!(
            "{}: {} tests ({} production, {:.1}%) {}",
            category, total, production, production_pct, status
        );
    }

    println!("\n🔍 Detailed Breakdown");
    println!("{}", "-".repeat(50));

    for file in &report.files {
        let prod_count = file.tests.iter().filter(|t| t.is_production).count();
        let total_count = file.tests.len();
        let status = if file.category.production_ready {
            "✅"
        } else {
            "⚠️"
        };

        println!(
            "{} {} ({}/{} production)",
            status, file.path, prod_count, total_count
        );

        if std::env::var("VERBOSE").is_ok() {
            for test in &file.tests {
                let test_status = if test.is_production { "✅" } else { "⚠️" };
                println!(
                    "    {} {}:{} {}",
                    test_status, test.line_number, test.name, test.name
                );
            }
        }
    }

    println!("\n🎯 Production Readiness Assessment");
    println!("{}", "-".repeat(50));

    let production_percentage =
        (report.production_tests as f64 / report.total_tests as f64) * 100.0;

    let readiness_level = match production_percentage {
        p if p >= 80.0 => "🚀 PRODUCTION READY",
        p if p >= 60.0 => "🟡 MOSTLY READY",
        p if p >= 40.0 => "🟠 PARTIALLY READY",
        p if p >= 20.0 => "🔴 EARLY STAGE",
        _ => "⚪ PROTOTYPE",
    };

    println!("Production Test Coverage: {:.1}%", production_percentage);
    println!("Readiness Level: {}", readiness_level);

    println!("\n💡 Recommendations");
    println!("{}", "-".repeat(50));

    if production_percentage < 50.0 {
        println!("• Focus on building more production features with real implementations");
        println!("• Integrate multi-tenant types into main ScimServer");
        println!("• Add real database providers (PostgreSQL, MySQL)");
        println!("• Create HTTP endpoints for multi-tenant operations");
    } else if production_percentage < 80.0 {
        println!("• Good foundation! Focus on production integrations");
        println!("• Add authentication and authorization systems");
        println!("• Create admin APIs for tenant management");
        println!("• Add production deployment examples");
    } else {
        println!("• Excellent production coverage!");
        println!("• Focus on performance optimization and monitoring");
        println!("• Add advanced features and enterprise capabilities");
        println!("• Consider moving some test infrastructure to a separate crate");
    }

    println!("\n💡 Usage Tips");
    println!("{}", "-".repeat(50));
    println!("• Run with VERBOSE=1 for detailed test listings");
    println!("• Production tests validate code that applications can actually use");
    println!("• Infrastructure tests are valuable for development but not end-user features");
    println!("• Focus development on increasing production test percentage");
}
