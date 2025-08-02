#!/usr/bin/env python3
"""Dump raw PDF content to see what's actually written"""

import sys
import re

def dump_pdf_content(filename):
    print(f"Dumping content of {filename}...\n")
    
    with open(filename, 'rb') as f:
        content = f.read()
    
    # Find all objects
    obj_pattern = rb'(\d+) 0 obj(.*?)endobj'
    objects = re.findall(obj_pattern, content, re.DOTALL)
    
    print(f"Found {len(objects)} objects\n")
    
    for obj_num, obj_content in objects[:10]:  # Show first 10 objects
        print(f"===== Object {obj_num.decode()} =====")
        # Clean up the content for display
        content_str = obj_content.decode('latin-1', errors='ignore')
        # Limit length for readability
        if len(content_str) > 500:
            content_str = content_str[:500] + "..."
        print(content_str.strip())
        print()
    
    # Find page content
    print("\n===== Looking for Page objects =====")
    page_pattern = rb'/Type\s*/Page[^>]*>>'
    pages = re.findall(page_pattern, content)
    print(f"Found {len(pages)} page objects")
    
    # Find content streams
    print("\n===== Looking for Content streams =====")
    stream_pattern = rb'stream\s*\n(.*?)\nendstream'
    streams = re.findall(stream_pattern, content, re.DOTALL)
    print(f"Found {len(streams)} content streams")
    
    for i, stream in enumerate(streams[:3]):
        print(f"\nStream {i+1} (first 200 bytes):")
        print(stream[:200].decode('latin-1', errors='ignore'))

if __name__ == "__main__":
    if len(sys.argv) < 2:
        print("Usage: python dump_pdf_content.py <pdf_file>")
        sys.exit(1)
    
    dump_pdf_content(sys.argv[1])