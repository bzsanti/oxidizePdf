#\!/usr/bin/env python3
"""Debug width issues in detail"""

import re

def analyze_pdf_internals(pdf_path):
    with open(pdf_path, 'rb') as f:
        content = f.read()
    
    text = content.decode('latin-1', errors='ignore')
    
    print(f"Analyzing: {pdf_path}")
    print("=" * 60)
    
    # Find the actual text rendering commands
    print("\nText rendering commands (first 5):")
    tj_commands = re.findall(r'<([0-9a-fA-F]+)>\s*Tj', text)
    for i, hex_str in enumerate(tj_commands[:5]):
        print(f"\n{i+1}. Hex: {hex_str}")
        # Decode as UTF-16BE
        if len(hex_str) % 4 == 0:
            chars = []
            for j in range(0, len(hex_str), 4):
                code = int(hex_str[j:j+4], 16)
                try:
                    char = chr(code)
                    chars.append(f"{char} (U+{code:04X})")
                except:
                    chars.append(f"? (U+{code:04X})")
            print(f"   Chars: {' '.join(chars[:10])}")
    
    # Find font selections
    print("\n\nFont selections (Tf commands):")
    tf_commands = re.findall(r'/F(\d+)\s+([\d.]+)\s+Tf', text)
    for font_id, size in tf_commands[:5]:
        print(f"  /F{font_id} {size} Tf")
    
    # Check if there are TJ commands (array of text)
    print("\n\nTJ commands (text arrays):")
    tj_arrays = re.findall(r'\[(.*?)\]\s*TJ', text)
    if tj_arrays:
        print(f"Found {len(tj_arrays)} TJ commands")
        for i, array in enumerate(tj_arrays[:2]):
            print(f"\n{i+1}. Array content (first 100 chars): {array[:100]}...")

if __name__ == "__main__":
    analyze_pdf_internals("oxidize-pdf-core/test-pdfs/simple_custom.pdf")
