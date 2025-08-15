#!/usr/bin/env bash
set -euo pipefail

# Documentation Quality Check Script for SCIM Server
# This script validates documentation completeness and quality

echo "🔍 SCIM Server Documentation Quality Check"
echo "=========================================="

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Track overall success
OVERALL_SUCCESS=true

# Function to print status
print_status() {
    local status=$1
    local message=$2
    if [ "$status" = "PASS" ]; then
        echo -e "${GREEN}✓ PASS${NC}: $message"
    elif [ "$status" = "FAIL" ]; then
        echo -e "${RED}✗ FAIL${NC}: $message"
        OVERALL_SUCCESS=false
    elif [ "$status" = "WARN" ]; then
        echo -e "${YELLOW}⚠ WARN${NC}: $message"
    else
        echo -e "${BLUE}ℹ INFO${NC}: $message"
    fi
}

# Function to check if command exists
command_exists() {
    command -v "$1" >/dev/null 2>&1
}

echo
echo "📋 Checking prerequisites..."

# Check for required tools
if command_exists cargo; then
    print_status "PASS" "cargo is available"
else
    print_status "FAIL" "cargo is required but not found"
fi

if command_exists mdbook; then
    print_status "PASS" "mdbook is available"
else
    print_status "WARN" "mdbook not found - install with 'cargo install mdbook'"
fi

echo
echo "📚 Checking Rust documentation..."

# Check for missing docs warnings
echo "Running cargo doc with missing_docs lint..."
if cargo doc --document-private-items 2>&1 | grep -q "warning.*missing documentation"; then
    print_status "WARN" "Some public items are missing documentation"
    echo "Run 'cargo doc --document-private-items' to see details"
else
    print_status "PASS" "No missing documentation warnings"
fi

# Check that docs build successfully
if cargo doc --no-deps --quiet 2>/dev/null; then
    print_status "PASS" "Rust documentation builds successfully"
else
    print_status "FAIL" "Rust documentation failed to build"
fi

echo
echo "📖 Checking guide documentation..."

# Check if mdbook guide builds
if [ -f "docs/guide/book.toml" ]; then
    if command_exists mdbook; then
        if mdbook build docs/guide --dest-dir ../../target/doc-guide 2>/dev/null; then
            print_status "PASS" "mdbook guide builds successfully"
        else
            print_status "FAIL" "mdbook guide failed to build"
        fi
    else
        print_status "WARN" "Cannot check mdbook guide - mdbook not installed"
    fi
else
    print_status "WARN" "No mdbook guide found at docs/guide/book.toml"
fi

echo
echo "📄 Checking core documentation files..."

# Check README length (should be concise)
if [ -f "README.md" ]; then
    README_LINES=$(wc -l < README.md)
    if [ "$README_LINES" -le 300 ]; then
        print_status "PASS" "README.md is appropriately sized ($README_LINES lines)"
    elif [ "$README_LINES" -le 500 ]; then
        print_status "WARN" "README.md is getting long ($README_LINES lines) - consider moving content to guide"
    else
        print_status "FAIL" "README.md is too long ($README_LINES lines) - should be under 300 lines"
    fi
else
    print_status "FAIL" "README.md not found"
fi

# Check for CHANGELOG
if [ -f "CHANGELOG.md" ]; then
    print_status "PASS" "CHANGELOG.md exists"
else
    print_status "WARN" "CHANGELOG.md not found"
fi

# Check for documentation strategy
if [ -f "ReferenceNotes/documentation-strategy.md" ]; then
    print_status "PASS" "Documentation strategy exists"
else
    print_status "WARN" "Documentation strategy not found"
fi

echo
echo "🔗 Checking examples..."

# Check that examples compile
EXAMPLE_COUNT=0
EXAMPLE_FAILURES=0

if [ -d "examples" ]; then
    for example in examples/*.rs; do
        if [ -f "$example" ]; then
            EXAMPLE_COUNT=$((EXAMPLE_COUNT + 1))
            example_name=$(basename "$example" .rs)
            if cargo check --example "$example_name" 2>/dev/null; then
                : # Success, do nothing
            else
                EXAMPLE_FAILURES=$((EXAMPLE_FAILURES + 1))
            fi
        fi
    done

    if [ "$EXAMPLE_FAILURES" -eq 0 ]; then
        print_status "PASS" "All $EXAMPLE_COUNT examples compile successfully"
    else
        print_status "FAIL" "$EXAMPLE_FAILURES out of $EXAMPLE_COUNT examples failed to compile"
    fi
else
    print_status "WARN" "No examples directory found"
fi

echo
echo "📊 Documentation structure check..."

# Check for recommended directory structure
EXPECTED_DIRS=("docs/guide" "examples" "ReferenceNotes")
for dir in "${EXPECTED_DIRS[@]}"; do
    if [ -d "$dir" ]; then
        print_status "PASS" "Directory $dir exists"
    else
        print_status "WARN" "Recommended directory $dir not found"
    fi
done

echo
echo "=========================================="

# Final summary
if [ "$OVERALL_SUCCESS" = true ]; then
    echo -e "${GREEN}🎉 Documentation quality check PASSED${NC}"
    exit 0
else
    echo -e "${RED}❌ Documentation quality check FAILED${NC}"
    echo "Please fix the issues above before proceeding."
    exit 1
fi
