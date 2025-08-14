#\!/usr/bin/env python3
"""Try to decompress PDF streams to see actual content"""

import zlib
import re

def decompress_pdf(pdf_path):
    with open(pdf_path, 'rb') as f:
        content = f.read()
    
    # Find FlateDecode streams
    pattern = rb'stream\r?\n(.*?)\r?\nendstream'
    matches = re.findall(pattern, content, re.DOTALL)
    
    print(f"Found {len(matches)} streams in {pdf_path}")
    
    for i, stream_data in enumerate(matches[:3]):  # First 3 streams
        try:
            # Try to decompress
            decompressed = zlib.decompress(stream_data)
            text = decompressed.decode('latin-1', errors='ignore')
            
            # Look for text operations
            if 'BT' in text or 'Tj' in text:
                print(f"\nStream {i+1} (text operations):")
                # Show first 500 chars
                print(text[:500])
                
                # Look for hex strings
                hex_matches = re.findall(r'<([0-9a-fA-F]+)>', text)
                if hex_matches:
                    print(f"\nHex strings found: {len(hex_matches)}")
                    for h in hex_matches[:3]:
                        print(f"  {h}")
                        # Decode as UTF-16BE
                        if len(h) % 4 == 0:
                            chars = []
                            for j in range(0, len(h), 4):
                                try:
                                    code = int(h[j:j+4], 16)
                                    chars.append(chr(code))
                                except:
                                    chars.append('?')
                            print(f"    -> {''.join(chars)}")
                
        except:
            pass

if __name__ == "__main__":
    decompress_pdf("oxidize-pdf-core/test-pdfs/simple_custom.pdf")
