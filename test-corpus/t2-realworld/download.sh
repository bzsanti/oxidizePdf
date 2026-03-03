#!/usr/bin/env bash
# T2 — Real-World Diversity Corpus Download
#
# Source: GovDocs1 from digitalcorpora.org
#   https://digitalcorpora.org/corpora/file-corpora/files/govdocs1/
#
# We download 2 threads (subset0, subset1) = ~2,000 PDF files
# Total size: ~2 GB
#
# GovDocs1 contains real documents from .gov websites with diverse
# generators (Adobe, Word, LibreOffice, LaTeX, etc.)

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BASE_URL="https://digitalcorpora.s3.amazonaws.com/corpora/files/govdocs1"

echo "=== T2 Real-World Corpus Download ==="

download_thread() {
    local thread_num=$1
    local thread_dir="${SCRIPT_DIR}/govdocs-subset${thread_num}"

    if [ -d "${thread_dir}" ] && [ -n "$(find "${thread_dir}" -name '*.pdf' 2>/dev/null | head -1)" ]; then
        echo "Thread ${thread_num} already present ($(find "${thread_dir}" -name '*.pdf' | wc -l) PDFs)"
        return 0
    fi

    echo "Downloading GovDocs1 thread ${thread_num}..."
    mkdir -p "${thread_dir}"

    # GovDocs1 threads are distributed as zip files
    local zip_name
    printf -v zip_name "%03d" "${thread_num}"
    local zip_url="${BASE_URL}/zipfiles/${zip_name}.zip"
    local zip_file="${thread_dir}/${zip_name}.zip"

    if command -v curl &>/dev/null; then
        curl -fsSL --retry 3 -o "${zip_file}" "${zip_url}" || {
            echo "WARNING: Failed to download thread ${thread_num}. Skipping."
            return 0
        }
    elif command -v wget &>/dev/null; then
        wget -q -O "${zip_file}" "${zip_url}" || {
            echo "WARNING: Failed to download thread ${thread_num}. Skipping."
            return 0
        }
    else
        echo "ERROR: Neither curl nor wget available."
        return 1
    fi

    # Extract only PDF files
    unzip -q -j -o "${zip_file}" '*.pdf' -d "${thread_dir}/" 2>/dev/null || true
    rm -f "${zip_file}"

    local pdf_count
    pdf_count=$(find "${thread_dir}" -name '*.pdf' | wc -l)
    echo "Thread ${thread_num}: extracted ${pdf_count} PDFs"
}

# Download threads 0 and 1
download_thread 0
download_thread 1

TOTAL=$(find "${SCRIPT_DIR}" -name '*.pdf' | wc -l)
echo "=== T2 complete: ${TOTAL} total PDFs ==="
