#!/usr/bin/env python3

import os
import re

def is_in_test_block_improved(lines, line_num):
    """Improved version - check if a line is inside a #[cfg(test)] block"""
    in_test = False
    brace_count = 0
    
    for i, line in enumerate(lines):
        stripped = line.strip()
        
        # Start of test block
        if stripped == "#[cfg(test)]":
            # Check if next few lines contain mod tests or similar
            next_lines = lines[i:i+5] if i+5 < len(lines) else lines[i:]
            for next_line in next_lines:
                if 'mod' in next_line and ('{' in next_line or any('{' in l for l in lines[i:i+10] if i+10 < len(lines))):
                    in_test = True
                    brace_count = 0
                    break
            continue
        
        if in_test:
            # Count braces to track scope
            brace_count += line.count('{') - line.count('}')
            
            # End of test block when we close all braces
            if brace_count <= 0 and i > line_num:
                in_test = False
        
        if i == line_num:
            return in_test
    
    return in_test

def find_production_unwraps_improved():
    """Find .unwrap() calls in production code (not tests) - improved version"""
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
                            if not is_in_test_block_improved(lines, i):
                                unwraps.append((filepath, i+1, line.strip()))
                
                except Exception as e:
                    print(f"Error reading {filepath}: {e}")
    
    return unwraps

if __name__ == "__main__":
    unwraps = find_production_unwraps_improved()
    
    # Group by file
    by_file = {}
    for filepath, line_num, line_content in unwraps:
        if filepath not in by_file:
            by_file[filepath] = []
        by_file[filepath].append((line_num, line_content))
    
    # Sort by count
    sorted_files = sorted(by_file.items(), key=lambda x: len(x[1]), reverse=True)
    
    print("Files with most unwraps (IMPROVED):")
    for filepath, unwrap_list in sorted_files[:10]:
        print(f"{len(unwrap_list):3d} unwraps in {filepath}")
    
    print(f"\nTotal production unwraps (IMPROVED): {len(unwraps)}")