#!/usr/bin/env bash
# Documentation build script for SCIM Server releases
# This script ensures all documentation is properly generated and ready for publication

set -euo pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Function to print colored output
print_status() {
    echo -e "${GREEN}[INFO]${NC} $1"
}

print_warning() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

print_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Check if mdbook is installed
if ! command -v mdbook &> /dev/null; then
    print_error "mdbook is not installed. Please install it with:"
    echo "  cargo install mdbook"
    exit 1
fi

# Get script directory
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
DOCS_DIR="$PROJECT_ROOT/docs/guide"

print_status "Building SCIM Server documentation..."
print_status "Project root: $PROJECT_ROOT"
print_status "Documentation directory: $DOCS_DIR"

# Check if documentation directory exists
if [ ! -d "$DOCS_DIR" ]; then
    print_error "Documentation directory not found: $DOCS_DIR"
    exit 1
fi

# Change to documentation directory
cd "$DOCS_DIR"

# Clean previous build
print_status "Cleaning previous build..."
if mdbook clean; then
    print_status "Previous build cleaned successfully"
else
    print_warning "Clean command failed, continuing anyway..."
fi

# Build documentation
print_status "Building documentation with mdbook..."
if mdbook build; then
    print_status "Documentation built successfully"
else
    print_error "Documentation build failed"
    exit 1
fi

# Verify build output
BOOK_DIR="$DOCS_DIR/book"
if [ ! -d "$BOOK_DIR" ]; then
    print_error "Build output directory not found: $BOOK_DIR"
    exit 1
fi

if [ ! -f "$BOOK_DIR/index.html" ]; then
    print_error "Main index.html not found in build output"
    exit 1
fi

# Count generated files
FILE_COUNT=$(find "$BOOK_DIR" -name "*.html" | wc -l)
print_status "Generated $FILE_COUNT HTML files"

# Optional: Run basic validation
print_status "Running basic validation..."

# Check for broken internal links (basic check)
if command -v grep &> /dev/null; then
    BROKEN_LINKS=$(grep -r "href.*\.md" "$BOOK_DIR" || true)
    if [ -n "$BROKEN_LINKS" ]; then
        print_warning "Found potential broken links (should be .html):"
        echo "$BROKEN_LINKS"
    fi
fi

print_status "Documentation build completed successfully!"
print_status "Generated files are in: $BOOK_DIR"
print_status ""
print_status "Next steps for release:"
print_status "1. Review the generated documentation at: $BOOK_DIR/index.html"
print_status "2. Commit the generated HTML files: git add docs/guide/book/"
print_status "3. Include in release commit: git commit -m 'Update documentation for release'"
print_status ""
print_status "For local preview: mdbook serve (from $DOCS_DIR)"
