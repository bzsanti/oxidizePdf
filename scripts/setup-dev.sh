#!/bin/bash
#
# Development setup script for oxidizePdf
# Sets up pre-commit hooks and validates environment
#

set -e

echo "🚀 Setting up oxidizePdf development environment..."

# Check if we're in the right directory
if [ ! -f "Cargo.toml" ]; then
    echo "❌ Error: Run this script from the project root directory"
    exit 1
fi

# Check Rust installation
echo "🦀 Checking Rust installation..."
if ! command -v cargo &> /dev/null; then
    echo "❌ Rust/Cargo not found! Please install Rust from https://rustup.rs/"
    exit 1
fi

RUST_VERSION=$(rustc --version)
echo "✅ Found Rust: $RUST_VERSION"

# Install required components
echo "🔧 Installing required Rust components..."
rustup component add rustfmt clippy

# Set up pre-commit hook
echo "🪝 Setting up pre-commit hook..."
if [ -f ".git/hooks/pre-commit" ]; then
    echo "⚠️  Pre-commit hook already exists, backing up..."
    mv ".git/hooks/pre-commit" ".git/hooks/pre-commit.backup.$(date +%s)"
fi

cp "scripts/pre-commit.template" ".git/hooks/pre-commit"
chmod +x ".git/hooks/pre-commit"
echo "✅ Pre-commit hook installed"

# Validate current state
echo "🔍 Validating current project state..."

echo "📝 Checking formatting..."
if cargo fmt --all -- --check; then
    echo "✅ Code is properly formatted"
else
    echo "⚠️  Code needs formatting - run 'cargo fmt --all'"
fi

echo "🔍 Running clippy..."
if cargo clippy --all -- -D warnings; then
    echo "✅ No clippy warnings"
else
    echo "⚠️  Clippy warnings found - please address them"
fi

echo "🔨 Building workspace..."
if cargo build --workspace; then
    echo "✅ Build successful"
else
    echo "❌ Build failed - please fix compilation errors"
    exit 1
fi

echo "🧪 Running tests..."
if cargo test --workspace; then
    echo "✅ All tests pass"
else
    echo "⚠️  Some tests are failing"
fi

# Create useful aliases file
echo "📝 Creating development aliases..."
cat > .dev-aliases << 'EOF'
# Development aliases for oxidizePdf
# Source this file: source .dev-aliases

alias oxfmt='cargo fmt --all'
alias oxcheck='cargo clippy --all -- -D warnings'
alias oxtest='cargo test --workspace'
alias oxbuild='cargo build --workspace'
alias oxfull='cargo fmt --all && cargo clippy --all -- -D warnings && cargo test --workspace'
alias oxdoc='cargo doc --open --workspace'
alias oxbench='cargo bench --workspace'

echo "🛠️  oxidizePdf development aliases loaded!"
echo "💡 Available commands:"
echo "  oxfmt     - Format code"
echo "  oxcheck   - Run clippy"
echo "  oxtest    - Run tests"
echo "  oxbuild   - Build workspace"
echo "  oxfull    - Run all checks"
echo "  oxdoc     - Open documentation"
echo "  oxbench   - Run benchmarks"
EOF

echo ""
echo "🎉 Development environment setup complete!"
echo ""
echo "📚 Next steps:"
echo "  1. Source aliases: source .dev-aliases"
echo "  2. Read docs/DEVELOPMENT_GUIDELINES.md"
echo "  3. Check CONTRIBUTING.md for workflow"
echo ""
echo "🔧 Available commands:"
echo "  cargo fmt --all              # Format code"
echo "  cargo clippy --all -- -D warnings  # Check lints"
echo "  cargo test --workspace       # Run tests"
echo "  cargo build --workspace      # Build all"
echo ""
echo "Happy coding! 🦀"