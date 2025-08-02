#!/usr/bin/env python3
"""Extract and decompress PDF page content"""

import sys
import zlib
import re

def extract_page_content(filename):
    print(f"Extracting page content from {filename}...\n")
    
    with open(filename, 'rb') as f:
        content = f.read()
    
    # Find all streams
    stream_pattern = rb'<<([^>]*?)>>\s*stream\s*\n(.*?)\nendstream'
    streams = re.findall(stream_pattern, content, re.DOTALL)
    
    print(f"Found {len(streams)} streams\n")
    
    for i, (header, stream_data) in enumerate(streams):
        print(f"=== Stream {i+1} ===")
        header_str = header.decode('latin-1', errors='ignore')
        print(f"Header: {header_str[:200]}...")
        
        # Check if it's compressed
        if b'/Filter' in header and b'/FlateDecode' in header:
            try:
                decompressed = zlib.decompress(stream_data)
                print(f"Decompressed content ({len(decompressed)} bytes):")
                print(decompressed.decode('latin-1', errors='ignore')[:500])
            except Exception as e:
                print(f"Failed to decompress: {e}")
                print(f"Raw data ({len(stream_data)} bytes):")
                print(stream_data[:100])
        else:
            print(f"Uncompressed content ({len(stream_data)} bytes):")
            print(stream_data.decode('latin-1', errors='ignore')[:500])
        print()

if __name__ == "__main__":
    if len(sys.argv) < 2:
        print("Usage: python test_page_content.py <pdf_file>")
        sys.exit(1)
    
    extract_page_content(sys.argv[1])