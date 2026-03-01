#!/usr/bin/env bash
# T4 — AI/RAG Target Corpus Download
#
# Source: PubMed Central Open Access (PMC OA)
#   https://www.ncbi.nlm.nih.gov/pmc/tools/ftp/
#
# Selection: 500 papers with diverse characteristics:
#   - 200 native text PDFs (clean multi-column academic papers)
#   - 150 mixed content (some scanned figures, native text)
#   - 100 with complex tables
#   - 50 with equations/formulas
#
# Ground truth: PubMed Central provides full-text XML alongside PDFs
# for accuracy comparison.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

echo "=== T4 AI/RAG Target Corpus Download ==="

PAPERS_DIR="${SCRIPT_DIR}/papers"
GT_DIR="${SCRIPT_DIR}/ground-truth"

if [ -d "${PAPERS_DIR}" ] && [ -n "$(find "${PAPERS_DIR}" -name '*.pdf' 2>/dev/null | head -1)" ]; then
    echo "T4 corpus already present ($(find "${PAPERS_DIR}" -name '*.pdf' | wc -l) PDFs)"
    exit 0
fi

echo "T4 corpus requires PubMed Central OA subset selection."
echo ""
echo "To populate this corpus:"
echo "  1. Download PMC OA bulk files from: https://ftp.ncbi.nlm.nih.gov/pub/pmc/oa_bulk/"
echo "  2. Select 500 papers matching the criteria above"
echo "  3. Place PDFs in: ${PAPERS_DIR}"
echo "  4. Extract XML full-text as .txt files in: ${GT_DIR}"
echo ""
echo "Expected: 500 PDFs + matching ground truth files"

mkdir -p "${PAPERS_DIR}" "${GT_DIR}"

echo "=== T4 download complete (manual selection needed) ==="
