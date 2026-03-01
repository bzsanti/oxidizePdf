#!/usr/bin/env bash
# T1 — Spec Compliance Corpus Download
#
# Sources:
#   - veraPDF corpus: https://github.com/veraPDF/veraPDF-corpus
#   - Mozilla pdf.js test suite: https://github.com/nicolo-ribaudo/pdfjs-test
#
# Total: ~2,000 PDFs
# Size: ~500 MB

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

echo "=== T1 Spec Compliance Corpus Download ==="

# ── veraPDF corpus ──
VERAPDF_DIR="${SCRIPT_DIR}/verapdf"
if [ -d "${VERAPDF_DIR}" ] && [ -n "$(find "${VERAPDF_DIR}" -name '*.pdf' 2>/dev/null | head -1)" ]; then
    echo "veraPDF corpus already present ($(find "${VERAPDF_DIR}" -name '*.pdf' | wc -l) PDFs)"
else
    echo "Downloading veraPDF corpus..."
    mkdir -p "${VERAPDF_DIR}"

    # Clone the veraPDF corpus repository (shallow clone for speed)
    TEMP_DIR=$(mktemp -d)
    git clone --depth 1 https://github.com/veraPDF/veraPDF-corpus.git "${TEMP_DIR}/verapdf" 2>/dev/null || {
        echo "WARNING: Failed to clone veraPDF corpus. Skipping."
        rm -rf "${TEMP_DIR}"
        exit 0
    }

    # Copy only PDF files
    find "${TEMP_DIR}/verapdf" -name '*.pdf' -exec cp {} "${VERAPDF_DIR}/" \;
    rm -rf "${TEMP_DIR}"

    PDF_COUNT=$(find "${VERAPDF_DIR}" -name '*.pdf' | wc -l)
    echo "veraPDF: downloaded ${PDF_COUNT} PDFs"
fi

# ── pdf.js test suite ──
PDFJS_DIR="${SCRIPT_DIR}/pdfjs"
if [ -d "${PDFJS_DIR}" ] && [ -n "$(find "${PDFJS_DIR}" -name '*.pdf' 2>/dev/null | head -1)" ]; then
    echo "pdf.js corpus already present ($(find "${PDFJS_DIR}" -name '*.pdf' | wc -l) PDFs)"
else
    echo "Downloading pdf.js test suite..."
    mkdir -p "${PDFJS_DIR}"

    TEMP_DIR=$(mktemp -d)
    # pdf.js test PDFs are in the main mozilla/pdf.js repository under test/pdfs/
    # We use sparse checkout to only fetch the test/pdfs directory
    git clone --depth 1 --filter=blob:none --sparse https://github.com/mozilla/pdf.js.git "${TEMP_DIR}/pdfjs" 2>/dev/null || {
        echo "WARNING: Failed to clone pdf.js repository. Skipping."
        rm -rf "${TEMP_DIR}"
        exit 0
    }

    (cd "${TEMP_DIR}/pdfjs" && git sparse-checkout set test/pdfs 2>/dev/null) || {
        echo "WARNING: Sparse checkout failed. Trying full clone fallback..."
    }

    find "${TEMP_DIR}/pdfjs" -name '*.pdf' -exec cp {} "${PDFJS_DIR}/" \;
    rm -rf "${TEMP_DIR}"

    PDF_COUNT=$(find "${PDFJS_DIR}" -name '*.pdf' | wc -l)
    echo "pdf.js: downloaded ${PDF_COUNT} PDFs"
fi

TOTAL=$(find "${SCRIPT_DIR}" -name '*.pdf' | wc -l)
echo "=== T1 complete: ${TOTAL} total PDFs ==="
