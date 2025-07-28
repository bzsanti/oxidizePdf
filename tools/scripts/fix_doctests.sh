#!/bin/bash

# Script to fix all oxidize_pdf_core references in doctests

echo "Fixing doctest references from oxidize_pdf_core to oxidize_pdf..."

# Find all Rust files in oxidize-pdf-core/src
find oxidize-pdf-core/src -name "*.rs" -type f | while read -r file; do
    echo "Processing: $file"
    
    # Replace all occurrences of oxidize_pdf_core with oxidize_pdf in the file
    sed -i '' 's/use oxidize_pdf_core::/use oxidize_pdf::/g' "$file"
done

echo "Done! All references have been updated."