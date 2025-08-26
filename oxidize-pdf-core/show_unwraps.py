#!/usr/bin/env python3

import sys

file_to_check = sys.argv[1] if len(sys.argv) > 1 else "./src/text/mod.rs"

try:
    with open(file_to_check, 'r', encoding='utf-8') as f:
        lines = f.readlines()
    
    print(f"Unwraps in {file_to_check}:")
    for i, line in enumerate(lines):
        if '.unwrap()' in line and '///' not in line:
            print(f"{i+1:4d}: {line.strip()}")
            
except Exception as e:
    print(f"Error: {e}")