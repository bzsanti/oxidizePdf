#\!/usr/bin/env python3
"""Analyze the internal structure of a PDF to debug spacing issues"""

import re
import sys

def analyze_pdf(pdf_path):
    with open(pdf_path, 'rb') as f:
        content = f.read()
    
    print(f"Analyzing: {pdf_path}")
    print("=" * 60)
    
    # Look for font references in the PDF
    font_refs = re.findall(rb'/F\d+\s+\d+\s+\d+\s+R', content)
    print(f"Font references found: {len(font_refs)}")
    
    # Look for Type0 fonts
    type0_count = content.count(b'/Type0')
    print(f"Type0 fonts: {type0_count}")
    
    # Look for Identity-H encoding
    identity_h = content.count(b'/Identity-H')
    print(f"Identity-H encoding: {identity_h}")
    
    # Look for DW value
    dw_match = re.search(rb'/DW\s+(\d+)', content)
    if dw_match:
        dw_value = int(dw_match.group(1))
        print(f"DW (default width): {dw_value}")
    
    # Check for compressed streams
    flate_count = content.count(b'/FlateDecode')
    print(f"Compressed streams: {flate_count}")
    
    # Try to find text operations (might be compressed)
    tj_ops = re.findall(rb'<[0-9a-fA-F]+>\s*Tj', content)
    print(f"Tj operations found (uncompressed): {len(tj_ops)}")
    
    # Check for TJ operations (text arrays)
    tj_array_ops = re.findall(rb'\[.*?\]\s*TJ', content)
    print(f"TJ operations found: {len(tj_array_ops)}")
    
    # Check W array format
    w_array_match = re.search(rb'/W\s*\[([^\]]{0,200})', content)
    if w_array_match:
        w_sample = w_array_match.group(1).decode('latin-1', errors='ignore')
        print(f"\nW array sample (first 200 chars):")
        print(f"  {w_sample}")
        
        # Check if 65 (A) is in the W array
        if b'65 66 666' in content or b'65 [666]' in content:
            print("  ✓ Character 'A' (65) found in W array with width 666")
        else:
            print("  ✗ Character 'A' (65) not found in expected format")

if __name__ == "__main__":
    analyze_pdf("oxidize-pdf-core/test-pdfs/simple_custom.pdf")
    print("\n" + "=" * 60 + "\n")
    # Compare with a working PDF if we have one
    import os
    if os.path.exists("oxidize-pdf-core/test-pdfs/spacing_test.pdf"):
        analyze_pdf("oxidize-pdf-core/test-pdfs/spacing_test.pdf")
