#\!/usr/bin/env python3
"""Debug how CIDs are being used"""

import re

def analyze_cid_usage(pdf_path):
    with open(pdf_path, 'rb') as f:
        content = f.read()
    
    print(f"Analyzing CID usage in: {pdf_path}")
    print("=" * 60)
    
    # Find text with hex strings
    hex_pattern = rb'<([0-9a-fA-F]+)>\s*Tj'
    hex_matches = re.findall(hex_pattern, content)
    
    if hex_matches:
        print(f"Found {len(hex_matches)} hex strings in Tj operators")
        
        # Analyze first few
        for i, hex_bytes in enumerate(hex_matches[:3]):
            hex_str = hex_bytes.decode('ascii')
            print(f"\n{i+1}. Hex string: {hex_str}")
            
            # Parse as UTF-16BE (2 bytes per character)
            if len(hex_str) % 4 == 0:
                chars = []
                cids = []
                for j in range(0, len(hex_str), 4):
                    cid = int(hex_str[j:j+4], 16)
                    cids.append(cid)
                    try:
                        char = chr(cid)
                        chars.append(char)
                    except:
                        chars.append('?')
                
                print(f"   Text: {''.join(chars)}")
                print(f"   CIDs: {cids[:10]}")  # First 10
                
                # Check if these CIDs are in expected ASCII range
                ascii_cids = [c for c in cids if 32 <= c <= 126]
                if ascii_cids:
                    print(f"   ASCII CIDs found: {ascii_cids[:5]}")

if __name__ == "__main__":
    analyze_cid_usage("oxidize-pdf-core/test-pdfs/simple_custom.pdf")
