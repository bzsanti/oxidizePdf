#!/usr/bin/env bash
# T5 — Quality Benchmark Corpus Download
#
# Source: OmniDocBench
#   https://github.com/opendatalab/OmniDocBench
#
# ~900 pages across 9 document types:
#   academic papers, textbooks, slides, financial reports,
#   newspapers, handwritten notes, magazines, books, notes
#
# Includes ground-truth annotations for:
#   text, tables, formulas, reading order

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

echo "=== T5 Quality Benchmark Corpus Download ==="

BENCH_DIR="${SCRIPT_DIR}/omnidocbench"
ANNOT_DIR="${SCRIPT_DIR}/annotations"

if [ -d "${BENCH_DIR}" ] && [ -n "$(find "${BENCH_DIR}" -name '*.pdf' 2>/dev/null | head -1)" ]; then
    echo "T5 corpus already present ($(find "${BENCH_DIR}" -name '*.pdf' | wc -l) PDFs)"
    exit 0
fi

echo "Downloading OmniDocBench..."
mkdir -p "${BENCH_DIR}" "${ANNOT_DIR}"

TEMP_DIR=$(mktemp -d)
git clone --depth 1 https://github.com/opendatalab/OmniDocBench.git "${TEMP_DIR}/omnidocbench" 2>/dev/null || {
    echo "WARNING: Failed to clone OmniDocBench. Skipping."
    echo ""
    echo "To manually populate:"
    echo "  1. Clone https://github.com/opendatalab/OmniDocBench"
    echo "  2. Copy PDF files to: ${BENCH_DIR}"
    echo "  3. Copy annotation files to: ${ANNOT_DIR}"
    rm -rf "${TEMP_DIR}"
    exit 0
}

# Copy PDFs and annotations
find "${TEMP_DIR}/omnidocbench" -name '*.pdf' -exec cp {} "${BENCH_DIR}/" \;
find "${TEMP_DIR}/omnidocbench" -name '*.json' -exec cp {} "${ANNOT_DIR}/" \;
rm -rf "${TEMP_DIR}"

PDF_COUNT=$(find "${BENCH_DIR}" -name '*.pdf' | wc -l)
echo "=== T5 complete: ${PDF_COUNT} PDFs downloaded ==="
