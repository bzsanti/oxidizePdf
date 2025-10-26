#!/usr/bin/env python3
"""Fix unused variable warnings."""

import sys
from pathlib import Path

# List of fixes: (file, line_number, old_var, new_var)
FIXES = [
    ("oxidize-pdf-core/src/operations/extract_images.rs", 677, "width", "_width"),
    ("oxidize-pdf-core/src/operations/extract_images.rs", 680, "height", "_height"),
    ("oxidize-pdf-core/src/operations/extract_images.rs", 747, "matrix", "_matrix"),
    ("oxidize-pdf-core/src/operations/extract_images.rs", 895, "i", "_i"),
    ("oxidize-pdf-core/src/parser/reader.rs", 397, "e", "_e"),
    ("oxidize-pdf-core/src/parser/reader.rs", 689, "reconstruction_error", "_reconstruction_error"),
    ("oxidize-pdf-core/src/parser/reader.rs", 1193, "obj_num", "_obj_num"),
    ("oxidize-pdf-core/src/parser/reader.rs", 1495, "obj_num", "_obj_num"),
    ("oxidize-pdf-core/src/parser/reader.rs", 1513, "obj_num", "_obj_num"),
    ("oxidize-pdf-core/src/parser/reader.rs", 1527, "obj_num", "_obj_num"),
    ("oxidize-pdf-core/src/parser/reader.rs", 1539, "obj_num", "_obj_num"),
    ("oxidize-pdf-core/src/parser/reader.rs", 1747, "e", "_e"),
    ("oxidize-pdf-core/src/parser/reader.rs", 1783, "e", "_e"),
    ("oxidize-pdf-core/src/parser/reader.rs", 1847, "e", "_e"),
    ("oxidize-pdf-core/src/parser/reader.rs", 1883, "e", "_e"),
    ("oxidize-pdf-core/src/parser/xref.rs", 140, "prev", "_prev"),
    ("oxidize-pdf-core/src/parser/xref.rs", 145, "regular_count", "_regular_count"),
    ("oxidize-pdf-core/src/parser/xref.rs", 146, "extended_count", "_extended_count"),
    ("oxidize-pdf-core/src/parser/xref.rs", 177, "e", "_e"),
]

import re

def fix_file(filepath, fixes_for_file):
    """Apply fixes to a file."""
    try:
        with open(filepath, 'r') as f:
            lines = f.readlines()
    except FileNotFoundError:
        print(f"File not found: {filepath}")
        return False

    modified = False
    for line_num, old_var, new_var in fixes_for_file:
        idx = line_num - 1
        if idx < 0 or idx >= len(lines):
            continue
        
        line = lines[idx]
        
        # Replace the variable name
        # This uses a word boundary to ensure we don't replace parts of words
        new_line = re.sub(rf'\b{re.escape(old_var)}\b', new_var, line)
        
        if new_line != line:
            lines[idx] = new_line
            modified = True
            print(f"  {filepath}:{line_num} - {old_var} → {new_var}")

    if modified:
        with open(filepath, 'w') as f:
            f.writelines(lines)
        return True
    
    return False

# Group fixes by file
fixes_by_file = {}
for filepath, line_num, old_var, new_var in FIXES:
    if filepath not in fixes_by_file:
        fixes_by_file[filepath] = []
    fixes_by_file[filepath].append((line_num, old_var, new_var))

print("Fixing unused variable warnings...")
print("-" * 70)

fixed_count = 0
for filepath, file_fixes in fixes_by_file.items():
    if fix_file(filepath, file_fixes):
        fixed_count += 1

print("-" * 70)
print(f"Fixed {fixed_count} file(s)")
