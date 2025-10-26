#!/bin/bash
# Fix unused variable warnings by prefixing with underscore

# Function to fix a variable in a file at a specific line
fix_var() {
    local file="$1"
    local line="$2"
    local var="$3"
    
    # Create backup
    cp "$file" "$file.bak2"
    
    # Use sed to replace the variable
    sed -i.tmp "${line}s/\b${var}\b/_${var}/g" "$file"
    rm -f "$file.tmp"
    
    echo "Fixed $var in $file:$line"
}

cd /Users/santifdezmunoz/Documents/repos/BelowZero/oxidize-pdf

# Fix extract_images.rs
fix_var "oxidize-pdf-core/src/operations/extract_images.rs" "677" "width"
fix_var "oxidize-pdf-core/src/operations/extract_images.rs" "680" "height"
fix_var "oxidize-pdf-core/src/operations/extract_images.rs" "747" "matrix"
fix_var "oxidize-pdf-core/src/operations/extract_images.rs" "895" "i"

# Fix reader.rs
fix_var "oxidize-pdf-core/src/parser/reader.rs" "397" "e"
fix_var "oxidize-pdf-core/src/parser/reader.rs" "689" "reconstruction_error"
fix_var "oxidize-pdf-core/src/parser/reader.rs" "1193" "obj_num"
fix_var "oxidize-pdf-core/src/parser/reader.rs" "1495" "obj_num"
fix_var "oxidize-pdf-core/src/parser/reader.rs" "1513" "obj_num"
fix_var "oxidize-pdf-core/src/parser/reader.rs" "1527" "obj_num"
fix_var "oxidize-pdf-core/src/parser/reader.rs" "1539" "obj_num"
fix_var "oxidize-pdf-core/src/parser/reader.rs" "1747" "e"
fix_var "oxidize-pdf-core/src/parser/reader.rs" "1783" "e"
fix_var "oxidize-pdf-core/src/parser/reader.rs" "1847" "e"
fix_var "oxidize-pdf-core/src/parser/reader.rs" "1883" "e"

# Fix xref.rs
fix_var "oxidize-pdf-core/src/parser/xref.rs" "140" "prev"
fix_var "oxidize-pdf-core/src/parser/xref.rs" "145" "regular_count"
fix_var "oxidize-pdf-core/src/parser/xref.rs" "146" "extended_count"
fix_var "oxidize-pdf-core/src/parser/xref.rs" "177" "e"

echo "Done fixing warnings"
