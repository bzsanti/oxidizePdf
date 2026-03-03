#!/usr/bin/env bash
# T3 — Stress Test Corpus Download
#
# Aggregates PDFs from multiple parser test suites that contain
# edge cases, corrupted files, and unusual PDF features.
#
# Sources:
#   1. poppler test suite (freedesktop.org GitLab)
#   2. qpdf test suite (GitHub)
#   3. pdf-association/safedocs hand-crafted edge cases (GitHub)
#   4. openpreserve format-corpus / pdfCabinetOfHorrors (GitHub)
#   5. Apache PDFBox test files (GitHub)
#   6. Google PDFium regression tests (googlesource.com)
#
# Expected: 500-1000 PDFs from real parser bug reports and test suites

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
TEMP_DIR=$(mktemp -d)
trap 'rm -rf "${TEMP_DIR}"' EXIT

echo "=== T3 Stress Corpus Download ==="

# Check if already populated
EXISTING=$(find "${SCRIPT_DIR}" -name '*.pdf' 2>/dev/null | wc -l)
if [ "${EXISTING}" -gt 100 ]; then
    echo "T3 corpus already present (${EXISTING} PDFs)"
    exit 0
fi

clone_repo() {
    local name=$1
    local url=$2
    local dest="${TEMP_DIR}/${name}"

    echo "  Cloning ${name}..."
    if git clone --depth 1 --quiet "${url}" "${dest}" 2>/dev/null; then
        return 0
    else
        echo "  WARNING: Failed to clone ${name}. Skipping."
        return 1
    fi
}

collect_pdfs() {
    local source_dir=$1
    local target_dir=$2
    local prefix=$3

    mkdir -p "${target_dir}"
    local count=0
    while IFS= read -r -d '' pdf; do
        local basename
        basename=$(basename "${pdf}")
        # Prefix to avoid name collisions between sources
        cp "${pdf}" "${target_dir}/${prefix}_${basename}" 2>/dev/null || true
        count=$((count + 1))
    done < <(find "${source_dir}" -name '*.pdf' -print0 2>/dev/null)
    echo "    Collected ${count} PDFs from ${prefix}"
}

# --- Source 1: poppler test repository ---
echo "[1/6] Poppler test suite..."
POPPLER_DIR="${SCRIPT_DIR}/poppler"
if [ ! -d "${POPPLER_DIR}" ] || [ -z "$(find "${POPPLER_DIR}" -name '*.pdf' 2>/dev/null | head -1)" ]; then
    if clone_repo "poppler-test" "https://gitlab.freedesktop.org/poppler/test.git"; then
        collect_pdfs "${TEMP_DIR}/poppler-test" "${POPPLER_DIR}" "poppler"
    fi
else
    echo "  Already present ($(find "${POPPLER_DIR}" -name '*.pdf' | wc -l) PDFs)"
fi

# --- Source 2: qpdf test suite ---
echo "[2/6] qpdf test suite..."
QPDF_DIR="${SCRIPT_DIR}/qpdf"
if [ ! -d "${QPDF_DIR}" ] || [ -z "$(find "${QPDF_DIR}" -name '*.pdf' 2>/dev/null | head -1)" ]; then
    if clone_repo "qpdf" "https://github.com/qpdf/qpdf.git"; then
        collect_pdfs "${TEMP_DIR}/qpdf" "${QPDF_DIR}" "qpdf"
    fi
else
    echo "  Already present ($(find "${QPDF_DIR}" -name '*.pdf' | wc -l) PDFs)"
fi

# --- Source 3: pdf-association/safedocs hand-crafted edge cases ---
echo "[3/6] PDF Association SafeDocs targeted tests..."
SAFEDOCS_DIR="${SCRIPT_DIR}/safedocs"
if [ ! -d "${SAFEDOCS_DIR}" ] || [ -z "$(find "${SAFEDOCS_DIR}" -name '*.pdf' 2>/dev/null | head -1)" ]; then
    if clone_repo "safedocs" "https://github.com/pdf-association/safedocs.git"; then
        collect_pdfs "${TEMP_DIR}/safedocs" "${SAFEDOCS_DIR}" "safedocs"
    fi
else
    echo "  Already present ($(find "${SAFEDOCS_DIR}" -name '*.pdf' | wc -l) PDFs)"
fi

# --- Source 4: openpreserve format-corpus (pdfCabinetOfHorrors + others) ---
echo "[4/6] openpreserve format-corpus..."
PRESERVE_DIR="${SCRIPT_DIR}/format-corpus"
if [ ! -d "${PRESERVE_DIR}" ] || [ -z "$(find "${PRESERVE_DIR}" -name '*.pdf' 2>/dev/null | head -1)" ]; then
    if clone_repo "format-corpus" "https://github.com/openpreserve/format-corpus.git"; then
        collect_pdfs "${TEMP_DIR}/format-corpus" "${PRESERVE_DIR}" "preserve"
    fi
else
    echo "  Already present ($(find "${PRESERVE_DIR}" -name '*.pdf' | wc -l) PDFs)"
fi

# --- Source 5: Apache PDFBox test files ---
echo "[5/6] Apache PDFBox test files..."
PDFBOX_DIR="${SCRIPT_DIR}/pdfbox"
if [ ! -d "${PDFBOX_DIR}" ] || [ -z "$(find "${PDFBOX_DIR}" -name '*.pdf' 2>/dev/null | head -1)" ]; then
    if clone_repo "pdfbox-testfiles" "https://github.com/apache/pdfbox-testfiles.git"; then
        collect_pdfs "${TEMP_DIR}/pdfbox-testfiles" "${PDFBOX_DIR}" "pdfbox"
    fi
else
    echo "  Already present ($(find "${PDFBOX_DIR}" -name '*.pdf' | wc -l) PDFs)"
fi

# --- Source 6: Google PDFium regression tests ---
echo "[6/6] Google PDFium regression tests..."
PDFIUM_DIR="${SCRIPT_DIR}/pdfium"
if [ ! -d "${PDFIUM_DIR}" ] || [ -z "$(find "${PDFIUM_DIR}" -name '*.pdf' 2>/dev/null | head -1)" ]; then
    if clone_repo "pdfium_tests" "https://pdfium.googlesource.com/pdfium_tests"; then
        collect_pdfs "${TEMP_DIR}/pdfium_tests" "${PDFIUM_DIR}" "pdfium"
    fi
else
    echo "  Already present ($(find "${PDFIUM_DIR}" -name '*.pdf' | wc -l) PDFs)"
fi

# --- Summary ---
TOTAL=$(find "${SCRIPT_DIR}" -name '*.pdf' | wc -l)
echo ""
echo "=== T3 complete: ${TOTAL} total PDFs ==="
echo "  Sources:"
for subdir in poppler qpdf safedocs format-corpus pdfbox pdfium; do
    if [ -d "${SCRIPT_DIR}/${subdir}" ]; then
        count=$(find "${SCRIPT_DIR}/${subdir}" -name '*.pdf' | wc -l)
        echo "    ${subdir}: ${count} PDFs"
    fi
done
