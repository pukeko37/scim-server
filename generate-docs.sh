#!/bin/bash

# SCIM Server Documentation Generation Script
# This script generates comprehensive documentation for the scim-server crate

set -e

echo "🔧 SCIM Server Documentation Generator"
echo "======================================"

# Check if cargo is available
if ! command -v cargo &> /dev/null; then
    echo "❌ Error: cargo is not installed or not in PATH"
    exit 1
fi

# Check if we're in the right directory
if [ ! -f "Cargo.toml" ]; then
    echo "❌ Error: This script must be run from the scim-server crate root directory"
    echo "   (The directory containing Cargo.toml)"
    exit 1
fi

# Check if this is the scim-server crate
if ! grep -q "name = \"scim-server\"" Cargo.toml; then
    echo "❌ Error: This doesn't appear to be the scim-server crate directory"
    exit 1
fi

echo "📁 Working directory: $(pwd)"
echo ""

# Clean previous documentation
echo "🧹 Cleaning previous documentation..."
cargo clean --doc

# Generate documentation for this crate only (no dependencies)
echo "📚 Generating documentation for scim-server crate..."
cargo doc --no-deps --document-private-items

# Check if generation was successful
if [ ! -f "target/doc/scim_server/index.html" ]; then
    echo "❌ Error: Documentation generation failed"
    exit 1
fi

echo "✅ Documentation generated successfully!"
echo ""

# Display information about generated docs
echo "📖 Generated Documentation:"
echo "   Main page: target/doc/scim_server/index.html"
echo "   All items: target/doc/scim_server/all.html"
echo ""

# List all modules
echo "📋 Available modules:"
for module in target/doc/scim_server/*/; do
    if [ -d "$module" ]; then
        module_name=$(basename "$module")
        echo "   - $module_name"
    fi
done

echo ""

# Check if we can open the docs automatically
if command -v xdg-open &> /dev/null; then
    echo "🌐 Opening documentation in your default browser..."
    xdg-open target/doc/scim_server/index.html
elif command -v open &> /dev/null; then
    echo "🌐 Opening documentation in your default browser..."
    open target/doc/scim_server/index.html
elif command -v start &> /dev/null; then
    echo "🌐 Opening documentation in your default browser..."
    start target/doc/scim_server/index.html
else
    echo "💡 To view the documentation, open this file in your browser:"
    echo "   file://$(pwd)/target/doc/scim_server/index.html"
fi

echo ""
echo "🎉 Documentation generation complete!"
echo ""
echo "📚 Quick links:"
echo "   • Main documentation: target/doc/scim_server/index.html"
echo "   • Dynamic server:     target/doc/scim_server/dynamic_server/index.html"
echo "   • Resource types:     target/doc/scim_server/resource/index.html"
echo "   • Schema system:      target/doc/scim_server/schema/index.html"
echo "   • Error handling:     target/doc/scim_server/error/index.html"
echo ""
echo "💡 Tip: Run 'cargo doc --no-deps --open' to regenerate and open docs"
