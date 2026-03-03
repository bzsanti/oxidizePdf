#!/usr/bin/env bash
# T5 — Quality Benchmark Corpus Download
#
# Sources:
#   1. OmniDocBench from Hugging Face (CVPR 2025)
#      https://huggingface.co/datasets/opendatalab/OmniDocBench
#      ~1,355 pages across 9 document types with ground-truth annotations
#      Requires git-lfs for large files
#
#   2. veraPDF test corpus (fallback / supplement)
#      https://github.com/veraPDF/veraPDF-corpus
#      ~2,900 PDFs covering PDF/A and PDF/UA conformance
#
# Total expected: 1,000-4,000+ PDFs with quality benchmarking data

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
TEMP_DIR=$(mktemp -d)
trap 'rm -rf "${TEMP_DIR}"' EXIT

echo "=== T5 Quality Benchmark Corpus Download ==="

# Check if already populated
EXISTING=$(find "${SCRIPT_DIR}" -name '*.pdf' 2>/dev/null | wc -l)
if [ "${EXISTING}" -gt 100 ]; then
    echo "T5 corpus already present (${EXISTING} PDFs)"
    exit 0
fi

# --- Source 1: OmniDocBench from Hugging Face ---
echo "[1/2] OmniDocBench (Hugging Face)..."
OMNIDOC_DIR="${SCRIPT_DIR}/omnidocbench"
ANNOT_DIR="${SCRIPT_DIR}/annotations"

if [ ! -d "${OMNIDOC_DIR}" ] || [ -z "$(find "${OMNIDOC_DIR}" -name '*.pdf' 2>/dev/null | head -1)" ]; then
    mkdir -p "${OMNIDOC_DIR}" "${ANNOT_DIR}"

    # Check for git-lfs
    if command -v git-lfs &>/dev/null || git lfs version &>/dev/null 2>&1; then
        echo "  git-lfs available, cloning with LFS..."
        GIT_LFS_SKIP_SMUDGE=0 git clone --depth 1 \
            "https://huggingface.co/datasets/opendatalab/OmniDocBench" \
            "${TEMP_DIR}/omnidocbench" 2>/dev/null && {

            # Copy PDFs from ori_pdfs/ directory (where the actual PDFs are)
            if [ -d "${TEMP_DIR}/omnidocbench/ori_pdfs" ]; then
                find "${TEMP_DIR}/omnidocbench/ori_pdfs" -name '*.pdf' -exec cp {} "${OMNIDOC_DIR}/" \;
            fi
            # Also check root and other directories
            find "${TEMP_DIR}/omnidocbench" -maxdepth 2 -name '*.pdf' -exec cp {} "${OMNIDOC_DIR}/" \;

            # Copy annotation files
            find "${TEMP_DIR}/omnidocbench" -name '*.json' -maxdepth 1 -exec cp {} "${ANNOT_DIR}/" \;

            PDF_COUNT=$(find "${OMNIDOC_DIR}" -name '*.pdf' | wc -l)
            echo "  OmniDocBench: ${PDF_COUNT} PDFs collected"
        } || {
            echo "  WARNING: Failed to clone OmniDocBench. Trying without LFS..."
        }
    else
        echo "  git-lfs not available. Install with: sudo apt install git-lfs"
        echo "  Skipping OmniDocBench (PDFs are stored with LFS)."
    fi
else
    echo "  Already present ($(find "${OMNIDOC_DIR}" -name '*.pdf' | wc -l) PDFs)"
fi

# --- Source 2: veraPDF test corpus ---
echo "[2/2] veraPDF test corpus..."
VERAPDF_DIR="${SCRIPT_DIR}/verapdf-corpus"

if [ ! -d "${VERAPDF_DIR}" ] || [ -z "$(find "${VERAPDF_DIR}" -name '*.pdf' 2>/dev/null | head -1)" ]; then
    mkdir -p "${VERAPDF_DIR}"

    echo "  Cloning veraPDF-corpus (~160 MB)..."
    if git clone --depth 1 --quiet \
        "https://github.com/veraPDF/veraPDF-corpus.git" \
        "${TEMP_DIR}/verapdf-corpus" 2>/dev/null; then

        # Collect all PDFs, preserving relative path info in filename
        local_count=0
        while IFS= read -r -d '' pdf; do
            # Create a flat name from the relative path
            rel_path="${pdf#${TEMP_DIR}/verapdf-corpus/}"
            flat_name="${rel_path//\//_}"
            cp "${pdf}" "${VERAPDF_DIR}/${flat_name}" 2>/dev/null || true
            local_count=$((local_count + 1))
        done < <(find "${TEMP_DIR}/verapdf-corpus" -name '*.pdf' -print0 2>/dev/null)

        echo "  veraPDF corpus: ${local_count} PDFs collected"
    else
        echo "  WARNING: Failed to clone veraPDF-corpus. Skipping."
    fi
else
    echo "  Already present ($(find "${VERAPDF_DIR}" -name '*.pdf' | wc -l) PDFs)"
fi

# --- Summary ---
TOTAL=$(find "${SCRIPT_DIR}" -name '*.pdf' | wc -l)
echo ""
echo "=== T5 complete: ${TOTAL} total PDFs ==="
echo "  Sources:"
for subdir in omnidocbench verapdf-corpus; do
    if [ -d "${SCRIPT_DIR}/${subdir}" ]; then
        count=$(find "${SCRIPT_DIR}/${subdir}" -name '*.pdf' | wc -l)
        echo "    ${subdir}: ${count} PDFs"
    fi
done
