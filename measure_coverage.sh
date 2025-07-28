#!/bin/bash
# Simple coverage measurement script

echo "Running code coverage analysis..."

# Run tarpaulin with basic settings
cargo tarpaulin \
    --packages oxidize-pdf \
    --lib \
    --timeout 30 \
    --out Html \
    --output-dir target/coverage \
    --exclude-files "*/tests/*" \
    --exclude-files "*/examples/*" \
    --exclude-files "*/benches/*" \
    2>&1 | grep -E "(Coverage|Tested|Uncovered|lines)" || echo "Coverage run completed. Check target/coverage/tarpaulin-report.html"

# Alternative: use llvm-cov if available
if command -v cargo-llvm-cov &> /dev/null; then
    echo ""
    echo "Running llvm-cov as alternative..."
    cargo llvm-cov --lib --no-report
    cargo llvm-cov report --summary-only
fi