#!/bin/bash

# oxidize-pdf Coverage Measurement Script
# Version: 1.0.0
# This script implements the official coverage methodology defined in docs/COVERAGE_METHODOLOGY.md

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Configuration
TIMEOUT=600
OUTPUT_DIR="target/coverage"
HISTORY_FILE="docs/coverage_history.csv"

# Print header
echo -e "${BLUE}═══════════════════════════════════════════════════════════════${NC}"
echo -e "${BLUE}           oxidize-pdf Coverage Measurement Tool              ${NC}"
echo -e "${BLUE}═══════════════════════════════════════════════════════════════${NC}"
echo ""

# Check if cargo-tarpaulin is installed
if ! command -v cargo-tarpaulin &> /dev/null; then
    echo -e "${YELLOW}cargo-tarpaulin not found. Installing...${NC}"
    cargo install cargo-tarpaulin
fi

# Create output directory
mkdir -p "$OUTPUT_DIR"

# Create history file if it doesn't exist
if [ ! -f "$HISTORY_FILE" ]; then
    echo "Date,Version,Line Coverage,Branch Coverage,Function Coverage,Total Tests,Notes" > "$HISTORY_FILE"
fi

# Get current version
VERSION=$(grep "^version" Cargo.toml | head -1 | cut -d'"' -f2)
DATE=$(date +"%Y-%m-%d %H:%M:%S")

echo -e "${GREEN}Starting coverage analysis...${NC}"
echo "Version: $VERSION"
echo "Date: $DATE"
echo ""

# Run tarpaulin with standard configuration
echo -e "${BLUE}Running cargo-tarpaulin...${NC}"

# Create temporary file for raw output
TEMP_OUTPUT=$(mktemp)

# Run tarpaulin and capture output
cargo tarpaulin \
    --workspace \
    --lib \
    --timeout "$TIMEOUT" \
    --exclude-files "*/tests/*" \
    --exclude-files "*/examples/*" \
    --exclude-files "*/benches/*" \
    --exclude-files "*/build.rs" \
    --exclude-files "**/mod.rs" \
    --ignore-panics \
    --skip-clean \
    --out Html \
    --out Json \
    --output-dir "$OUTPUT_DIR" \
    2>&1 | tee "$TEMP_OUTPUT"

# Extract coverage percentages
LINE_COVERAGE=$(grep -E "^\s*[0-9]+\.[0-9]+%" "$TEMP_OUTPUT" | head -1 | grep -oE "[0-9]+\.[0-9]+" || echo "0.0")

# Try to get more detailed stats from JSON if available
if [ -f "$OUTPUT_DIR/tarpaulin-report.json" ]; then
    # Parse JSON for more detailed metrics (requires jq)
    if command -v jq &> /dev/null; then
        TOTAL_LINES=$(jq '.files[].covered_lines | add' "$OUTPUT_DIR/tarpaulin-report.json" 2>/dev/null || echo "0")
        COVERED_LINES=$(jq '.files[].uncovered_lines | add' "$OUTPUT_DIR/tarpaulin-report.json" 2>/dev/null || echo "0")
    fi
fi

# Count total tests
TOTAL_TESTS=$(cargo test --lib -- --list 2>/dev/null | grep -E "^test " | wc -l | xargs)

# Generate summary report
REPORT_FILE="$OUTPUT_DIR/coverage_report_${DATE// /_}.txt"

cat > "$REPORT_FILE" << EOF
oxidize-pdf Coverage Report
Generated: $DATE
Version: $VERSION
Tool: cargo-tarpaulin

Summary:
- Line Coverage: ${LINE_COVERAGE}%
- Total Tests: $TOTAL_TESTS

Coverage Classification:
EOF

# Determine coverage level and add emoji
if (( $(echo "$LINE_COVERAGE < 40" | bc -l) )); then
    LEVEL="🔴 Critical - Immediate intervention required"
elif (( $(echo "$LINE_COVERAGE < 55" | bc -l) )); then
    LEVEL="🟠 Low - Priority improvement needed"
elif (( $(echo "$LINE_COVERAGE < 70" | bc -l) )); then
    LEVEL="🟡 Acceptable - Planned improvements"
elif (( $(echo "$LINE_COVERAGE < 85" | bc -l) )); then
    LEVEL="🟢 Good - Maintain and enhance"
else
    LEVEL="💚 Excellent - Best practice achieved"
fi

echo "$LEVEL" >> "$REPORT_FILE"

# Module-level analysis
echo -e "\nModule Coverage Analysis:" >> "$REPORT_FILE"
echo "| Module | Lines | Tests | Density |" >> "$REPORT_FILE"
echo "|--------|-------|-------|---------|" >> "$REPORT_FILE"

for dir in src/*; do
    if [ -d "$dir" ]; then
        MODULE=$(basename "$dir")
        LINES=$(find "$dir" -name "*.rs" -exec wc -l {} \; 2>/dev/null | awk '{sum+=$1} END {print sum}')
        TESTS=$(grep -r "#\[test\]" "$dir" --include="*.rs" 2>/dev/null | wc -l | xargs)
        if [ "$LINES" -gt 0 ]; then
            DENSITY=$(echo "scale=1; $TESTS * 100 / $LINES" | bc)
        else
            DENSITY="0.0"
        fi
        printf "| %-14s | %6d | %5d | %6.1f |\n" "$MODULE" "$LINES" "$TESTS" "$DENSITY" >> "$REPORT_FILE"
    fi
done

# Add to history
echo "$DATE,$VERSION,$LINE_COVERAGE,N/A,N/A,$TOTAL_TESTS,Automated measurement" >> "$HISTORY_FILE"

# Clean up
rm -f "$TEMP_OUTPUT"

# Print results
echo ""
echo -e "${GREEN}═══════════════════════════════════════════════════════════════${NC}"
echo -e "${GREEN}                    Coverage Analysis Complete                 ${NC}"
echo -e "${GREEN}═══════════════════════════════════════════════════════════════${NC}"
echo ""
echo -e "Line Coverage: ${YELLOW}${LINE_COVERAGE}%${NC}"
echo -e "Total Tests: ${YELLOW}${TOTAL_TESTS}${NC}"
echo -e "Status: $LEVEL"
echo ""
echo "Reports generated:"
echo "  - HTML Report: $OUTPUT_DIR/tarpaulin-report.html"
echo "  - JSON Report: $OUTPUT_DIR/tarpaulin-report.json"
echo "  - Text Report: $REPORT_FILE"
echo "  - History Updated: $HISTORY_FILE"
echo ""

# Generate README badge snippet
BADGE_COLOR="red"
if (( $(echo "$LINE_COVERAGE >= 70" | bc -l) )); then
    BADGE_COLOR="green"
elif (( $(echo "$LINE_COVERAGE >= 55" | bc -l) )); then
    BADGE_COLOR="yellow"
elif (( $(echo "$LINE_COVERAGE >= 40" | bc -l) )); then
    BADGE_COLOR="orange"
fi

echo "README Badge:"
echo "![Coverage](https://img.shields.io/badge/coverage-${LINE_COVERAGE}%25-${BADGE_COLOR})"
echo ""

# Check if coverage meets minimum requirement
MIN_COVERAGE=55
if (( $(echo "$LINE_COVERAGE < $MIN_COVERAGE" | bc -l) )); then
    echo -e "${RED}⚠️  WARNING: Coverage is below minimum requirement of ${MIN_COVERAGE}%${NC}"
    exit 1
else
    echo -e "${GREEN}✅ Coverage meets minimum requirement${NC}"
fi