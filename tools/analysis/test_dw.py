#!/usr/bin/env python3
"""Check DW and W array in PDFs"""

import re

def check_dw_and_w(pdf_path):
    with open(pdf_path, 'rb') as f:
        content = f.read()
    
    text = content.decode('latin-1', errors='ignore')
    
    print(f"Analyzing: {pdf_path}")
    print("=" * 60)
    
    # Find DW (default width)
    dw_matches = re.findall(r'/DW\s+(\d+)', text)
    if dw_matches:
        print(f"DW (default width) values found: {dw_matches}")
    else:
        print("No DW found")
    
    # Find Font references
    font_refs = re.findall(r'/F(\d+)\s+(\d+)\s+\d+\s+R', text)
    if font_refs:
        print(f"Font references: {font_refs[:5]}")
    
    # Check for Type0
    if '/Type0' in text:
        print("✓ Type0 font present")
    
    # Check for standard fonts
    standard_fonts = ['Helvetica', 'Times', 'Courier']
    for font in standard_fonts:
        if f'/{font}' in text:
            print(f"✓ Standard font: {font}")
    
    # Look for Font dictionaries
    font_dicts = re.findall(r'/Type\s*/Font.*?/BaseFont\s*/([^\s/>]+)', text, re.DOTALL)
    if font_dicts:
        print(f"Fonts found: {font_dicts}")

if __name__ == "__main__":
    check_dw_and_w("oxidize-pdf-core/test-pdfs/spacing_test.pdf")