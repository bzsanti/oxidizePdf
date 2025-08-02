#!/bin/bash

# Test script for PDF samples
echo "=== OxidizePDF Test Suite ==="
echo "Date: $(date)"
echo

# Test simple PDFs (should all pass)
echo "Testing SIMPLE PDFs (should work):"
echo "=================================="
cargo run --release --example test_external_pdf -p oxidize-pdf-core -- PDF_Samples/simple 2>&1 | grep -E "(Total|Passed|Failed)"
echo

# Test medium PDFs (work in progress)
echo "Testing MEDIUM PDFs (targets):"
echo "=============================="
cargo run --release --example test_external_pdf -p oxidize-pdf-core -- PDF_Samples/medium 2>&1 | grep -E "(Total|Passed|Failed)"
echo

# Test complex PDFs (future goals)
echo "Testing COMPLEX PDFs (future):"
echo "=============================="
cargo run --release --example test_external_pdf -p oxidize-pdf-core -- PDF_Samples/complex 2>&1 | grep -E "(Total|Passed|Failed)"
echo

# Overall statistics
echo "OVERALL Statistics:"
echo "==================="
cargo run --release --example test_external_pdf -p oxidize-pdf-core -- PDF_Samples 2>&1 | grep -E "(Total|Passed|Failed)"

# Error analysis
echo
echo "Most common errors:"
echo "==================="
cargo run --release --example test_external_pdf -p oxidize-pdf-core -- PDF_Samples 2>&1 | grep "✗" | sed 's/.*- //' | sort | uniq -c | sort -nr | head -5