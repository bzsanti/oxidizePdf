#!/bin/bash
# Generate R5 and R6 encrypted PDFs using qpdf 11.0+
# Phase 4.1 of TDD_PLAN_AES256_ENCRYPTION.md

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

# Check qpdf version
QPDF_VERSION=$(qpdf --version | head -1 | cut -d' ' -f3)
echo "Using qpdf version: $QPDF_VERSION"

# Version check (needs 11.0.0+)
MAJOR_VERSION=$(echo "$QPDF_VERSION" | cut -d'.' -f1)
if [ "$MAJOR_VERSION" -lt 11 ]; then
    echo "ERROR: qpdf 11.0.0+ required for R5/R6 support"
    exit 1
fi

echo "Generating test PDFs in: $SCRIPT_DIR"

# Use existing Cold_Email_Hacks.pdf as base (it's unencrypted)
BASE_PDF="$SCRIPT_DIR/Cold_Email_Hacks.pdf"

if [ ! -f "$BASE_PDF" ]; then
    echo "ERROR: Base PDF not found at $BASE_PDF"
    echo "Please ensure Cold_Email_Hacks.pdf exists in the fixtures directory"
    exit 1
fi

echo "Using base PDF: $BASE_PDF"

# Encrypt with R5 (AES-256, revision 5 - deprecated but supported)
# --force-R5 option goes INSIDE --encrypt ... --
echo "Generating R5 encrypted PDFs..."
qpdf --encrypt user5 owner5 256 --force-R5 -- "$BASE_PDF" encrypted_aes256_r5_user.pdf
qpdf --encrypt "" owner5_empty 256 --force-R5 -- "$BASE_PDF" encrypted_aes256_r5_empty_user.pdf
qpdf --encrypt "unicode_contraseña" owner5_unicode 256 --force-R5 -- "$BASE_PDF" encrypted_aes256_r5_unicode.pdf

# Encrypt with R6 (AES-256, revision 6 - default for 256-bit)
# 256-bit encryption defaults to R6, no extra flags needed
echo "Generating R6 encrypted PDFs..."
qpdf --encrypt user6 owner6 256 -- "$BASE_PDF" encrypted_aes256_r6_user.pdf
qpdf --encrypt "" owner6_empty 256 -- "$BASE_PDF" encrypted_aes256_r6_empty_user.pdf
qpdf --encrypt "café🔒" owner6_unicode 256 -- "$BASE_PDF" encrypted_aes256_r6_unicode.pdf

echo ""
echo "=== Generated R5/R6 test PDFs ==="
ls -lh encrypted_aes256_*.pdf

echo ""
echo "=== Verifying encryption metadata ==="
for pdf in encrypted_aes256_*.pdf; do
    echo "--- $pdf ---"
    qpdf --show-encryption "$pdf" | head -8
    echo ""
done

echo "=== Generation complete ==="
echo ""
echo "Passwords for testing:"
echo "  R5 PDFs:"
echo "    encrypted_aes256_r5_user.pdf: user='user5', owner='owner5'"
echo "    encrypted_aes256_r5_empty_user.pdf: user='', owner='owner5_empty'"
echo "    encrypted_aes256_r5_unicode.pdf: user='unicode_contraseña', owner='owner5_unicode'"
echo "  R6 PDFs:"
echo "    encrypted_aes256_r6_user.pdf: user='user6', owner='owner6'"
echo "    encrypted_aes256_r6_empty_user.pdf: user='', owner='owner6_empty'"
echo "    encrypted_aes256_r6_unicode.pdf: user='café🔒', owner='owner6_unicode'"
