#!/bin/bash

# Run ISO 32000 Compliance Tests
echo "Running ISO 32000-1:2008 Compliance Tests..."
echo "=========================================="

# Run the specific compliance test
echo "Running comprehensive compliance test..."
cargo test --package oxidize-pdf-test-suite test_comprehensive_iso_compliance -- --nocapture

# Generate the full PDF report
echo ""
echo "Generating PDF compliance report..."
cargo run --package oxidize-pdf-test-suite --bin iso-compliance-report

# Display results
echo ""
echo "Compliance test complete!"
echo "Reports generated:"
echo "  - ISO_32000_COMPLIANCE_REPORT.pdf"
echo "  - ISO_32000_COMPLIANCE_SUMMARY.md"