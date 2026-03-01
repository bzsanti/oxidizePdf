#!/usr/bin/env bash
# T6 — Adversarial Corpus Download
#
# Sources:
#   - Qiqqa corpus: Malformed PDFs from Qiqqa's test collection
#   - SafeDocs malicious subset: Deliberately crafted edge cases
#
# Total: ~200 PDFs
# Size: ~200 MB
#
# These files are designed to break parsers. They include:
#   - Invalid structure (missing headers, broken XRef)
#   - Deliberately malicious (zip bombs, infinite loops, deep nesting)
#   - Truncated files
#   - Files with wrong magic bytes
#   - Enormous objects designed to OOM parsers

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

echo "=== T6 Adversarial Corpus Download ==="

MALFORMED_DIR="${SCRIPT_DIR}/malformed"
MALICIOUS_DIR="${SCRIPT_DIR}/malicious"

if [ -d "${MALFORMED_DIR}" ] && [ -n "$(find "${MALFORMED_DIR}" -name '*.pdf' 2>/dev/null | head -1)" ]; then
    echo "T6 malformed corpus already present ($(find "${MALFORMED_DIR}" -name '*.pdf' | wc -l) PDFs)"
fi

if [ -d "${MALICIOUS_DIR}" ] && [ -n "$(find "${MALICIOUS_DIR}" -name '*.pdf' 2>/dev/null | head -1)" ]; then
    echo "T6 malicious corpus already present ($(find "${MALICIOUS_DIR}" -name '*.pdf' | wc -l) PDFs)"
fi

EXISTING=$(find "${SCRIPT_DIR}" -name '*.pdf' 2>/dev/null | wc -l)
if [ "${EXISTING}" -gt 0 ]; then
    echo "T6 corpus has ${EXISTING} PDFs total"
    exit 0
fi

echo "T6 adversarial corpus requires manual curation."
echo ""
echo "To populate this corpus:"
echo "  1. Malformed PDFs: Collect from Qiqqa, fuzzing outputs, etc."
echo "  2. Malicious PDFs: Curate from SafeDocs malicious subset"
echo "  3. Place in:"
echo "     - Malformed: ${MALFORMED_DIR}"
echo "     - Malicious: ${MALICIOUS_DIR}"
echo ""
echo "Expected: ~200 PDFs total"

mkdir -p "${MALFORMED_DIR}" "${MALICIOUS_DIR}"

echo "=== T6 download complete (manual curation needed) ==="
