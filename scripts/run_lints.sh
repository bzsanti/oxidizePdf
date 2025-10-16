#!/bin/bash
#
# Run custom oxidize-pdf lints using dylint
#
# This script builds the custom lints and runs them against the workspace.
# It reports any anti-patterns found in the code.
#
# Usage:
#   ./scripts/run_lints.sh              # Run all lints
#   ./scripts/run_lints.sh --fix        # Run with auto-fix (when supported)
#   ./scripts/run_lints.sh --verbose    # Verbose output

set -e  # Exit on error

# Color codes for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Script directory
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
LINTS_DIR="$PROJECT_ROOT/lints"

# Parse arguments
VERBOSE=false
FIX_MODE=false

for arg in "$@"; do
    case $arg in
        --verbose)
            VERBOSE=true
            shift
            ;;
        --fix)
            FIX_MODE=true
            shift
            ;;
        --help)
            echo "Usage: $0 [OPTIONS]"
            echo ""
            echo "Options:"
            echo "  --verbose    Enable verbose output"
            echo "  --fix        Auto-fix issues where possible"
            echo "  --help       Show this help message"
            exit 0
            ;;
        *)
            echo -e "${RED}Unknown option: $arg${NC}"
            exit 1
            ;;
    esac
done

echo -e "${BLUE}========================================${NC}"
echo -e "${BLUE}  oxidize-pdf Custom Lints${NC}"
echo -e "${BLUE}========================================${NC}"
echo ""

# Check if dylint is installed
if ! command -v cargo-dylint &> /dev/null; then
    echo -e "${RED}❌ cargo-dylint not found!${NC}"
    echo ""
    echo "Please install it with:"
    echo "  cargo +nightly install cargo-dylint dylint-link"
    exit 1
fi

# Check if nightly toolchain is installed
if ! rustup toolchain list | grep -q "nightly"; then
    echo -e "${RED}❌ Rust nightly toolchain not found!${NC}"
    echo ""
    echo "Please install it with:"
    echo "  rustup toolchain install nightly --component rustc-dev"
    exit 1
fi

# Build the lints
echo -e "${YELLOW}🔨 Building custom lints...${NC}"
cd "$LINTS_DIR"

if [ "$VERBOSE" = true ]; then
    cargo +nightly build --release
else
    cargo +nightly build --release 2>&1 | grep -E "(Compiling oxidize-pdf-lints|Finished|error|warning:)" || true
fi

if [ ${PIPESTATUS[0]} -ne 0 ]; then
    echo -e "${RED}❌ Failed to build lints${NC}"
    exit 1
fi

echo -e "${GREEN}✅ Lints built successfully${NC}"
echo ""

# Run the lints
cd "$PROJECT_ROOT"

echo -e "${YELLOW}🔍 Running lints on workspace...${NC}"
echo ""

# Build dylint command
DYLINT_CMD="cargo +nightly dylint --all --workspace"

if [ "$FIX_MODE" = true ]; then
    DYLINT_CMD="$DYLINT_CMD --fix"
fi

if [ "$VERBOSE" = true ]; then
    DYLINT_CMD="$DYLINT_CMD -- --verbose"
fi

# Run dylint
if eval "$DYLINT_CMD"; then
    echo ""
    echo -e "${GREEN}========================================${NC}"
    echo -e "${GREEN}✅ All lints passed!${NC}"
    echo -e "${GREEN}========================================${NC}"
    exit 0
else
    LINT_EXIT_CODE=$?
    echo ""
    echo -e "${RED}========================================${NC}"
    echo -e "${RED}❌ Lints found issues${NC}"
    echo -e "${RED}========================================${NC}"
    echo ""
    echo "Fix the issues above and run again."
    echo ""
    echo -e "${YELLOW}Lint categories:${NC}"
    echo "  • ${BLUE}BOOL_OPTION_PATTERN${NC}  - Using bool + Option<Error> instead of Result"
    echo "  • ${BLUE}STRING_ERRORS${NC}         - Using String for errors instead of error types"
    echo "  • ${BLUE}MISSING_ERROR_CONTEXT${NC} - Creating errors without context"
    echo "  • ${BLUE}LIBRARY_UNWRAPS${NC}       - Using unwrap() in library code"
    echo "  • ${BLUE}DURATION_PRIMITIVES${NC}   - Using primitives for durations instead of Duration"
    echo ""
    exit $LINT_EXIT_CODE
fi
