#!/usr/bin/env python3
"""Analyze PDF structure with decompression to debug Unicode issues"""

import sys
import re
import zlib

def extract_streams(content):
    """Extract and decompress PDF streams"""
    streams = []
    
    # Find stream objects
    pattern = rb'stream\s*\n(.*?)\nendstream'
    matches = re.findall(pattern, content, re.DOTALL)
    
    for compressed_data in matches:
        try:
            # Try to decompress with zlib (FlateDecode)
            decompressed = zlib.decompress(compressed_data)
            streams.append(decompressed)
        except:
            # If decompression fails, add raw data
            streams.append(compressed_data)
    
    return streams

def analyze_pdf(filename):
    """Analyze PDF file for Unicode-related structures"""
    with open(filename, 'rb') as f:
        content = f.read()
    
    print(f"Analyzing {filename}")
    print("=" * 60)
    
    # Extract and decompress streams
    streams = extract_streams(content)
    print(f"Found {len(streams)} streams")
    
    # Analyze decompressed content
    all_content = b'\n'.join(streams)
    
    # Look for text operators in decompressed content
    print("\nText operators in decompressed streams:")
    
    # Count BT/ET blocks
    bt_count = all_content.count(b'BT')
    et_count = all_content.count(b'ET')
    print(f"  BT/ET blocks: {bt_count}/{et_count}")
    
    # Look for Tf operators (set font)
    tf_matches = re.findall(rb'/([\w-]+)\s+([\d.]+)\s+Tf', all_content)
    if tf_matches:
        print(f"  Tf operators: {len(tf_matches)}")
        for font, size in tf_matches[:5]:  # Show first 5
            print(f"    Font: {font.decode('latin-1')}, Size: {size.decode('latin-1')}")
    
    # Look for Tj operators (show text)
    tj_count = all_content.count(b' Tj')
    print(f"  Tj operators: {tj_count}")
    
    # Look for text content
    print("\nText content preview:")
    for i, stream in enumerate(streams[:3]):  # First 3 streams
        text = stream.decode('latin-1', errors='ignore')
        if 'BT' in text or 'Tj' in text:
            print(f"\n  Stream {i+1}:")
            # Show first 500 chars
            preview = text[:500].replace('\n', '\n    ')
            print(f"    {preview}")
            if len(text) > 500:
                print("    ...")
    
    # Look for hex strings (Unicode text)
    hex_strings = re.findall(rb'<([0-9A-Fa-f]+)>\s*Tj', all_content)
    if hex_strings:
        print(f"\n  Hex strings found: {len(hex_strings)}")
        for hex_str in hex_strings[:5]:  # Show first 5
            hex_text = hex_str.decode('latin-1')
            if len(hex_text) <= 100:
                print(f"    <{hex_text}>")
                # Try to decode as UTF-16BE (for Unicode)
                try:
                    if len(hex_text) % 4 == 0:  # Must be even number of bytes
                        decoded = bytes.fromhex(hex_text).decode('utf-16-be', errors='ignore')
                        print(f"      → \"{decoded}\"")
                except:
                    pass
    
    print("\n")

if __name__ == "__main__":
    pdfs = [
        "oxidize-pdf-core/test-pdfs/unicode_complete.pdf",
    ]
    
    for pdf in pdfs:
        try:
            analyze_pdf(pdf)
        except FileNotFoundError:
            print(f"File not found: {pdf}\n")
        except Exception as e:
            print(f"Error analyzing {pdf}: {e}\n")