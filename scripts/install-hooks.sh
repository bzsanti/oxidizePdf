#!/bin/bash
#
# Install Git hooks for oxidizePdf
# Run this after cloning the repository
#

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
HOOKS_DIR="$REPO_ROOT/.git/hooks"

echo "🔧 Installing Git hooks for oxidizePdf..."

# Check if .git directory exists
if [ ! -d "$REPO_ROOT/.git" ]; then
    echo "❌ Error: Not in a git repository"
    echo "💡 Run this script from inside the oxidize-pdf repository"
    exit 1
fi

# Install pre-commit hook
HOOK_SOURCE="$HOOKS_DIR/pre-commit"
if [ ! -f "$HOOK_SOURCE" ]; then
    echo "❌ Error: pre-commit hook not found at $HOOK_SOURCE"
    echo "💡 The hook should be committed to .git/hooks/"
    exit 1
fi

# Make executable
chmod +x "$HOOK_SOURCE"

echo "✅ Pre-commit hook installed and activated"
echo ""
echo "📋 The hook will check:"
echo "   1. No backup files (*.backup, *_old.rs)"
echo "   2. Code formatting (cargo fmt)"
echo "   3. Clippy lints (including performance checks)"
echo "   4. Build success (library)"
echo "   5. Tests passing (library tests)"
echo ""
echo "💡 To bypass the hook temporarily: git commit --no-verify"
echo "💡 To disable permanently: rm .git/hooks/pre-commit"
echo ""
echo "🎉 Setup complete! Happy coding!"
