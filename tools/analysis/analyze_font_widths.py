#\!/usr/bin/env python3
"""Check default width and if W array uses it correctly"""

import re

def check_dw_usage(pdf_path):
    with open(pdf_path, 'rb') as f:
        content = f.read()
    
    text = content.decode('latin-1', errors='ignore')
    
    print(f"Analyzing: {pdf_path}")
    print("=" * 60)
    
    # Find DW (default width) - should be 1000 for most fonts
    dw_matches = re.findall(r'/DW\s+(\d+)', text)
    if dw_matches:
        dw = int(dw_matches[0])
        print(f"DW (default width): {dw}")
        if dw == 600:
            print("⚠️ DW is 600, but should typically be 1000 for TrueType fonts")
    else:
        print("No DW found")
        return
    
    # Check text stream for character codes
    print("\nChecking text content streams...")
    
    # Find BT...ET blocks
    bt_et_blocks = re.findall(r'BT(.*?)ET', text, re.DOTALL)
    
    if bt_et_blocks:
        print(f"Found {len(bt_et_blocks)} text blocks")
        for i, block in enumerate(bt_et_blocks[:3]):  # First 3 blocks
            # Look for hex strings in Tj
            hex_strings = re.findall(r'<([0-9a-fA-F]+)>\s*Tj', block)
            if hex_strings:
                print(f"\nText block {i+1} hex strings:")
                for hex_str in hex_strings[:2]:  # First 2 in each block
                    print(f"  {hex_str}")
                    # Convert to character codes
                    if len(hex_str) % 4 == 0:  # Should be pairs of 2 bytes for UTF-16BE
                        codes = []
                        for j in range(0, len(hex_str), 4):
                            code = int(hex_str[j:j+4], 16)
                            codes.append(code)
                        print(f"    Character codes: {codes[:10]}")  # First 10 codes

if __name__ == "__main__":
    check_dw_usage("oxidize-pdf-core/test-pdfs/spacing_test.pdf")
    print("\n" + "=" * 60 + "\n")
    check_dw_usage("oxidize-pdf-core/test-pdfs/simple_custom.pdf")
