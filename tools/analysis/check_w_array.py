#\!/usr/bin/env python3
"""Check W array structure in detail"""

import re

def analyze_w_array(pdf_path):
    with open(pdf_path, 'rb') as f:
        content = f.read()
    
    text = content.decode('latin-1', errors='ignore')
    
    print(f"Analyzing: {pdf_path}")
    print("=" * 60)
    
    # Find W arrays with context
    w_arrays = re.findall(r'/W\s*\[(.*?)\]', text, re.DOTALL)
    
    if w_arrays:
        for i, w_array in enumerate(w_arrays[:2]):  # First 2 W arrays
            print(f"\nW array {i+1}:")
            # Clean up whitespace
            cleaned = ' '.join(w_array.split())
            # Show first 200 chars
            if len(cleaned) > 200:
                print(f"First 200 chars: {cleaned[:200]}...")
            else:
                print(f"Content: {cleaned}")
            
            # Count entries
            numbers = re.findall(r'\d+', cleaned)
            print(f"Total numbers in array: {len(numbers)}")
            
            # Check if it's CID->Width or has ranges
            if len(numbers) >= 6:
                print(f"First 6 numbers: {numbers[:6]}")
                # Check if it looks like ranges (CID start end width)
                if len(numbers) >= 3:
                    first_three = [int(x) for x in numbers[:3]]
                    if first_three[1] == first_three[0] + 1:
                        print("Looks like: [CID CID+1 width] format")
                    elif first_three[1] > 1000:
                        print("Looks like: [CID width] pairs")
    else:
        print("No W array found")
    
    # Check CIDToGIDMap
    if '/CIDToGIDMap' in text:
        print("\n✓ CIDToGIDMap present")
        # Find its stream
        cidmap_match = re.search(r'/CIDToGIDMap\s+(\d+)\s+\d+\s+R', text)
        if cidmap_match:
            print(f"CIDToGIDMap reference: {cidmap_match.group(1)}")
    
    # Check for subsetting indicators
    if re.search(r'/BaseFont\s*/[A-Z]{6}\+', text):
        print("\n✓ Font appears to be subset (has prefix like ABCDEF+FontName)")

if __name__ == "__main__":
    analyze_w_array("oxidize-pdf-core/test-pdfs/spacing_test.pdf")
    print("\n" + "=" * 60 + "\n")
    analyze_w_array("oxidize-pdf-core/test-pdfs/simple_custom.pdf")
