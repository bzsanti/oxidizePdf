#!/usr/bin/env bash
# T6 — Adversarial Corpus Download
#
# Aggregates deliberately malformed, malicious, and broken PDFs from
# multiple security research collections.
#
# Sources:
#   1. PayloadsAllThePDFs — XSS/JavaScript payload PDFs
#   2. pdf-association/safedocs — Spec corner-case PDFs
#   3. openpreserve pdfCabinetOfHorrors — Archival horror scenarios
#   4. pentest-pdf-collection — Simulated malicious PDFs
#   5. Test-PDF-Files — Corrupted/encrypted test files
#   6. malicious-pdf generator — Generates phone-home PDFs
#
# Total expected: 80-150 adversarial PDFs
#
# SAFETY NOTE: These PDFs are for parser robustness testing only.
# They may contain JavaScript, embedded objects, or exploit-like structures.
# They are NOT actual malware — they test parser resilience.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
MALFORMED_DIR="${SCRIPT_DIR}/malformed"
MALICIOUS_DIR="${SCRIPT_DIR}/malicious"
TEMP_DIR=$(mktemp -d)
trap 'rm -rf "${TEMP_DIR}"' EXIT

echo "=== T6 Adversarial Corpus Download ==="

# Check if already populated
EXISTING=$(find "${SCRIPT_DIR}" -name '*.pdf' 2>/dev/null | wc -l)
if [ "${EXISTING}" -gt 50 ]; then
    echo "T6 corpus already present (${EXISTING} PDFs)"
    exit 0
fi

mkdir -p "${MALFORMED_DIR}" "${MALICIOUS_DIR}"

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

# --- Source 1: PayloadsAllThePDFs (XSS/JavaScript payloads) ---
echo "[1/6] PayloadsAllThePDFs..."
if clone_repo "payloads" "https://github.com/nickvdp/PayloadsAllThePDFs.git" || \
   clone_repo "payloads" "https://github.com/luigigubello/PayloadsAllThePDFs.git"; then
    count=0
    while IFS= read -r -d '' pdf; do
        cp "${pdf}" "${MALICIOUS_DIR}/payloads_$(basename "${pdf}")" 2>/dev/null || true
        count=$((count + 1))
    done < <(find "${TEMP_DIR}/payloads" -name '*.pdf' -print0 2>/dev/null)
    echo "    Collected ${count} PDFs"
fi

# --- Source 2: pdf-association/safedocs (spec corner cases) ---
echo "[2/6] PDF Association SafeDocs..."
if clone_repo "safedocs" "https://github.com/pdf-association/safedocs.git"; then
    count=0
    while IFS= read -r -d '' pdf; do
        cp "${pdf}" "${MALFORMED_DIR}/safedocs_$(basename "${pdf}")" 2>/dev/null || true
        count=$((count + 1))
    done < <(find "${TEMP_DIR}/safedocs" -name '*.pdf' -print0 2>/dev/null)
    echo "    Collected ${count} PDFs"
fi

# --- Source 3: openpreserve pdfCabinetOfHorrors ---
echo "[3/6] openpreserve pdfCabinetOfHorrors..."
if clone_repo "format-corpus" "https://github.com/openpreserve/format-corpus.git"; then
    count=0
    # Focus on the Cabinet of Horrors + other PDF directories
    for search_dir in "${TEMP_DIR}/format-corpus/pdfCabinetOfHorrors" "${TEMP_DIR}/format-corpus"; do
        while IFS= read -r -d '' pdf; do
            cp "${pdf}" "${MALFORMED_DIR}/preserve_$(basename "${pdf}")" 2>/dev/null || true
            count=$((count + 1))
        done < <(find "${search_dir}" -maxdepth 2 -name '*.pdf' -print0 2>/dev/null)
    done
    echo "    Collected ${count} PDFs"
fi

# --- Source 4: pentest-pdf-collection ---
echo "[4/6] pentest-pdf-collection..."
if clone_repo "pentest" "https://github.com/klausnitzer/pentest-pdf-collection.git"; then
    count=0
    while IFS= read -r -d '' pdf; do
        cp "${pdf}" "${MALICIOUS_DIR}/pentest_$(basename "${pdf}")" 2>/dev/null || true
        count=$((count + 1))
    done < <(find "${TEMP_DIR}/pentest" -name '*.pdf' -print0 2>/dev/null)
    echo "    Collected ${count} PDFs"
fi

# --- Source 5: Test-PDF-Files (corrupted + encrypted) ---
echo "[5/6] Test-PDF-Files..."
if clone_repo "test-pdf" "https://github.com/ArturT/Test-PDF-Files.git"; then
    count=0
    while IFS= read -r -d '' pdf; do
        cp "${pdf}" "${MALFORMED_DIR}/testpdf_$(basename "${pdf}")" 2>/dev/null || true
        count=$((count + 1))
    done < <(find "${TEMP_DIR}/test-pdf" -name '*.pdf' -print0 2>/dev/null)
    echo "    Collected ${count} PDFs"
fi

# --- Source 6: malicious-pdf generator ---
echo "[6/6] malicious-pdf generator..."
if clone_repo "malicious-pdf" "https://github.com/jonaslejon/malicious-pdf.git"; then
    if command -v python3 &>/dev/null; then
        pushd "${TEMP_DIR}/malicious-pdf" > /dev/null
        python3 malicious-pdf.py 2>/dev/null || true
        popd > /dev/null

        count=0
        while IFS= read -r -d '' pdf; do
            cp "${pdf}" "${MALICIOUS_DIR}/generated_$(basename "${pdf}")" 2>/dev/null || true
            count=$((count + 1))
        done < <(find "${TEMP_DIR}/malicious-pdf" -name '*.pdf' -print0 2>/dev/null)
        echo "    Generated ${count} PDFs"
    else
        echo "    python3 not available — skipping PDF generation"
    fi
fi

# --- Summary ---
echo ""
MALFORMED_COUNT=$(find "${MALFORMED_DIR}" -name '*.pdf' | wc -l)
MALICIOUS_COUNT=$(find "${MALICIOUS_DIR}" -name '*.pdf' | wc -l)
TOTAL=$((MALFORMED_COUNT + MALICIOUS_COUNT))

echo "=== T6 complete: ${TOTAL} total PDFs ==="
echo "  Malformed: ${MALFORMED_COUNT}"
echo "  Malicious: ${MALICIOUS_COUNT}"
