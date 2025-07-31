#!/usr/bin/env python3
"""Analyze PDF internal structure"""

import sys
import re

def analyze_pdf(filename):
    print(f"Analyzing {filename}...\n")
    
    with open(filename, 'rb') as f:
        content = f.read()
    
    # Find all objects
    obj_pattern = rb'(\d+) 0 obj'
    objects = re.findall(obj_pattern, content)
    
    print(f"Objects found: {len(objects)}")
    print(f"Object numbers: {[int(o) for o in objects]}")
    print(f"Min object: {min(int(o) for o in objects)}, Max object: {max(int(o) for o in objects)}")
    
    # Check if objects are sequential
    obj_nums = sorted([int(o) for o in objects])
    missing = []
    for i in range(1, max(obj_nums)):
        if i not in obj_nums:
            missing.append(i)
    
    if missing:
        print(f"⚠️  Missing object numbers: {missing}")
    else:
        print("✓ All objects are sequential")
    
    # Check object order
    actual_order = [int(o) for o in objects]
    if actual_order == sorted(actual_order):
        print("✓ Objects are written in sequential order")
    else:
        print("⚠️  Objects are NOT in sequential order")
        print(f"   Actual order: {actual_order[:10]}..." if len(actual_order) > 10 else f"   Actual order: {actual_order}")
    
    # Find catalog
    catalog_match = re.search(rb'/Type\s*/Catalog', content)
    if catalog_match:
        print("\n✓ Found Catalog")
        
        # Check for AcroForm
        acroform_match = re.search(rb'/AcroForm\s+(\d+)\s+0\s+R', content)
        if acroform_match:
            print(f"  ✓ Has AcroForm reference: {acroform_match.group(1).decode()} 0 R")
        
        # Check for Outlines
        outlines_match = re.search(rb'/Outlines\s+(\d+)\s+0\s+R', content)
        if outlines_match:
            print(f"  ✓ Has Outlines reference: {outlines_match.group(1).decode()} 0 R")
    
    # Check xref
    xref_match = re.search(rb'xref\s*\n\s*(\d+)\s+(\d+)', content)
    if xref_match:
        start = int(xref_match.group(1))
        count = int(xref_match.group(2))
        print(f"\n✓ Found xref table: {count} entries starting at {start}")
        
        # Verify xref entries
        xref_section = content[xref_match.end():]
        xref_entries = re.findall(rb'(\d{10})\s+(\d{5})\s+([fn])', xref_section[:count*20])
        print(f"  Found {len(xref_entries)} xref entries")

if __name__ == "__main__":
    if len(sys.argv) < 2:
        print("Usage: python analyze_pdf_structure.py <pdf_file>")
        sys.exit(1)
    
    analyze_pdf(sys.argv[1])