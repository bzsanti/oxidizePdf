#!/usr/bin/env python3
"""Debug script to analyze character spacing in PDFs"""

import sys
import re
import zlib

def extract_and_analyze_text_operations(pdf_path):
    """Extract and analyze text positioning operations from PDF"""
    with open(pdf_path, 'rb') as f:
        content = f.read()
    
    # Find compressed streams
    streams = []
    pattern = rb'stream\s*\n(.*?)\nendstream'
    matches = re.findall(pattern, content, re.DOTALL)
    
    for compressed_data in matches:
        try:
            # Decompress stream
            decompressed = zlib.decompress(compressed_data)
            streams.append(decompressed)
        except:
            streams.append(compressed_data)
    
    print(f"Analyzing: {pdf_path}")
    print("=" * 60)
    
    # Look for text operations
    for i, stream in enumerate(streams):
        try:
            text = stream.decode('latin-1', errors='ignore')
            if 'BT' in text and 'Tj' in text:
                print(f"\nStream {i+1} - Text operations:")
                
                # Find all Td (positioning) operations
                td_ops = re.findall(r'([\d.-]+)\s+([\d.-]+)\s+Td', text)
                if td_ops:
                    print(f"  Td operations (text positioning): {len(td_ops)}")
                    for x, y in td_ops[:5]:  # Show first 5
                        print(f"    Move to ({x}, {y})")
                
                # Find hex strings and their lengths
                hex_strings = re.findall(r'<([0-9A-Fa-f]+)>\s*Tj', text)
                if hex_strings:
                    print(f"  Hex strings: {len(hex_strings)}")
                    for hex_str in hex_strings[:3]:
                        # Count characters (4 hex digits = 1 UTF-16 char)
                        char_count = len(hex_str) // 4
                        print(f"    <{hex_str[:40]}...> ({char_count} chars, {len(hex_str)} hex digits)")
                        
                        # Decode first few characters
                        chars = []
                        for j in range(0, min(20, len(hex_str)), 4):
                            if j+4 <= len(hex_str):
                                char_code = int(hex_str[j:j+4], 16)
                                chars.append(chr(char_code))
                        print(f"      Decoded: {''.join(chars)}")
                
                # Check for TJ operations (array of positioned text)
                tj_arrays = re.findall(r'\[(.*?)\]\s*TJ', text)
                if tj_arrays:
                    print(f"  TJ operations (positioned text arrays): {len(tj_arrays)}")
                    for tj in tj_arrays[:2]:
                        print(f"    Array: {tj[:100]}...")
                
                # Look for W array in font definitions
                if '/W [' in text or '/W[' in text:
                    print("\n  Width array found in stream!")
                    w_match = re.search(r'/W\s*\[(.*?)\]', text, re.DOTALL)
                    if w_match:
                        w_content = w_match.group(1).replace('\n', ' ')
                        w_values = w_content.split()[:20]  # First 20 values
                        print(f"    W array preview: {' '.join(w_values)}...")
                
                # Check DW (default width)
                dw_match = re.search(r'/DW\s+(\d+)', text)
                if dw_match:
                    print(f"  Default width (DW): {dw_match.group(1)}")
                    
        except:
            pass
    
    # Check for font info in uncompressed parts
    text = content.decode('latin-1', errors='ignore')
    
    # Look for Type0 fonts
    if '/Type0' in text:
        print("\n✓ Type0 font found")
    
    # Look for CIDToGIDMap
    if b'CIDToGIDMap' in content:
        print("✓ CIDToGIDMap found")
        # Find the actual reference
        cidmap_refs = re.findall(r'/CIDToGIDMap\s+(\S+)', text)
        for ref in cidmap_refs[:2]:
            print(f"  Reference: {ref}")
    
    # Look for width specifications
    print("\nWidth specifications in PDF:")
    w_arrays = re.findall(r'/W\s*\[(.*?)\]', text, re.DOTALL)
    if w_arrays:
        for w_array in w_arrays[:1]:  # First W array
            values = w_array.replace('\n', ' ').split()[:30]
            print(f"  W array: {' '.join(values[:10])}...")
            
            # Analyze the pattern
            if len(values) >= 3:
                # Check if it's using ranges or individual values
                try:
                    # Try to parse as numbers
                    nums = [int(v) if v.isdigit() else v for v in values[:20]]
                    print(f"  Parsed values: {nums[:10]}")
                except:
                    pass

if __name__ == "__main__":
    pdfs = [
        "oxidize-pdf-core/test-pdfs/spacing_test.pdf",
    ]
    
    for pdf in pdfs:
        try:
            extract_and_analyze_text_operations(pdf)
        except Exception as e:
            print(f"Error analyzing {pdf}: {e}")