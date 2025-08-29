#!/usr/bin/env python3

import os
import re

def is_in_test_block(lines, line_num):
    """Check if a line is inside a #[cfg(test)] block"""
    in_test = False
    brace_count = 0
    
    for i, line in enumerate(lines):
        stripped = line.strip()
        
        # Start of test block
        if stripped == "#[cfg(test)]" and i < line_num:
            in_test = True
            brace_count = 0
            continue
        
        if in_test:
            brace_count += line.count('{') - line.count('}')
            
            # End of test block
            if brace_count <= 0 and i < line_num:
                in_test = False
        
        if i == line_num:
            return in_test
    
    return False

def find_production_unwraps():
    """Find .unwrap() calls in production code (not tests)"""
    unwraps = []
    
    for root, dirs, files in os.walk('./src'):
        for file in files:
            if file.endswith('.rs'):
                filepath = os.path.join(root, file)
                try:
                    with open(filepath, 'r', encoding='utf-8') as f:
                        lines = f.readlines()
                    
                    for i, line in enumerate(lines):
                        if '.unwrap()' in line and '///' not in line:  # Skip doc comments
                            if not is_in_test_block(lines, i):
                                unwraps.append((filepath, i+1, line.strip()))
                
                except Exception as e:
                    print(f"Error reading {filepath}: {e}")
    
    return unwraps

if __name__ == "__main__":
    unwraps = find_production_unwraps()
    
    # Group by file
    by_file = {}
    for filepath, line_num, line_content in unwraps:
        if filepath not in by_file:
            by_file[filepath] = []
        by_file[filepath].append((line_num, line_content))
    
    # Sort by count
    sorted_files = sorted(by_file.items(), key=lambda x: len(x[1]), reverse=True)
    
    print("Files with most unwraps:")
    for filepath, unwrap_list in sorted_files[:10]:
        print(f"{len(unwrap_list):3d} unwraps in {filepath}")
    
    print(f"\nTotal production unwraps: {len(unwraps)}")