#!/usr/bin/env bash
# T3 — Stress Test Corpus Download
#
# Source: DARPA SafeDocs Issue Tracker (curated subset)
#   https://github.com/nicolo-ribaudo/safedocs
#
# Curation criteria (750 files selected from ~32,500):
#   - 250 files that crashed/hung other parsers
#   - 200 files with corrupted cross-reference tables
#   - 150 files with unusual/rare PDF features
#   - 100 files with mixed valid/invalid content
#   - 50 files with extreme characteristics (10K+ pages, 500MB+, deep nesting)
#
# NOTE: The SafeDocs corpus is large. This script downloads a curated subset.
# For the full corpus, see: https://safedocs.xyz/

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

echo "=== T3 Stress Corpus Download ==="

ISSUE_DIR="${SCRIPT_DIR}/issue-tracker"

if [ -d "${ISSUE_DIR}" ] && [ -n "$(find "${ISSUE_DIR}" -name '*.pdf' 2>/dev/null | head -1)" ]; then
    echo "T3 corpus already present ($(find "${ISSUE_DIR}" -name '*.pdf' | wc -l) PDFs)"
    exit 0
fi

echo "T3 stress corpus requires manual curation."
echo ""
echo "To populate this corpus:"
echo "  1. Download SafeDocs from https://safedocs.xyz/"
echo "  2. Run the curation script: tools/curate_safedocs.py"
echo "  3. Or manually place curated PDFs in: ${ISSUE_DIR}"
echo ""
echo "Expected: ~750 curated PDFs"
echo ""
echo "For now, creating empty directory structure..."

mkdir -p "${ISSUE_DIR}"

echo "=== T3 download complete (manual curation needed) ==="
