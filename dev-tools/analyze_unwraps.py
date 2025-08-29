#!/usr/bin/env python3
"""
Analyze unwrap() usage in Rust source files to identify production vs test code.
"""
import os
import re
from pathlib import Path

def find_test_markers(content):
    """Find line numbers where test sections start."""
    test_markers = []
    lines = content.split('\n')
    for i, line in enumerate(lines, 1):
        if '#[cfg(test)]' in line:
            test_markers.append(i)
    return test_markers

def is_in_test_section(line_num, test_markers):
    """Check if a line number is in a test section."""
    for marker in test_markers:
        if line_num >= marker:
            return True
    return False

def analyze_file(filepath):
    """Analyze a single Rust file for unwrap usage."""
    try:
        with open(filepath, 'r', encoding='utf-8') as f:
            content = f.read()
        
        test_markers = find_test_markers(content)
        lines = content.split('\n')
        
        production_unwraps = []
        test_unwraps = []
        
        for i, line in enumerate(lines, 1):
            if '.unwrap()' in line:
                if is_in_test_section(i, test_markers):
                    test_unwraps.append(i)
                else:
                    production_unwraps.append(i)
        
        return {
            'filepath': filepath,
            'production': production_unwraps,
            'test': test_unwraps,
            'total': len(production_unwraps) + len(test_unwraps)
        }
    except Exception as e:
        return {
            'filepath': filepath,
            'error': str(e),
            'production': [],
            'test': [],
            'total': 0
        }

def main():
    src_dir = Path('oxidize-pdf-core/src')
    results = []
    
    # Find all Rust files
    for rust_file in src_dir.rglob('*.rs'):
        result = analyze_file(rust_file)
        if result['total'] > 0:
            results.append(result)
    
    # Sort by production unwraps (most critical first)
    results.sort(key=lambda x: len(x.get('production', [])), reverse=True)
    
    print("=== UNWRAP ANALYSIS RESULTS ===\n")
    
    total_production = sum(len(r.get('production', [])) for r in results)
    total_test = sum(len(r.get('test', [])) for r in results)
    
    print(f"SUMMARY:")
    print(f"  Production unwraps: {total_production}")
    print(f"  Test unwraps: {total_test}")
    print(f"  Total files with unwraps: {len(results)}\n")
    
    print("TOP PRIORITY (Production Unwraps):")
    production_files = [r for r in results if len(r.get('production', [])) > 0]
    for i, result in enumerate(production_files[:10], 1):
        rel_path = str(result['filepath']).replace('oxidize-pdf-core/src/', '')
        prod_count = len(result['production'])
        test_count = len(result['test'])
        print(f"  {i:2d}. {rel_path:<40} {prod_count:3d} prod, {test_count:3d} test")
    
    print(f"\n(Showing top 10 of {len(production_files)} files with production unwraps)")

if __name__ == '__main__':
    main()