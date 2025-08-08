#\!/usr/bin/env python3
"""Check W array in detail"""

import re

def extract_w_array(pdf_path):
    with open(pdf_path, 'rb') as f:
        content = f.read()
    
    # Find W array
    # Look for pattern: /W [...]
    match = re.search(b'/W\\s*\\[([^\\]]+)\\]', content)
    if match:
        w_array_bytes = match.group(1)
        # Parse the array
        w_array_text = w_array_bytes.decode('latin-1', errors='ignore')
        # Extract numbers
        numbers = re.findall(r'(\d+)', w_array_text)
        numbers = [int(n) for n in numbers]
        
        print(f"W array for {pdf_path}:")
        print(f"Total entries: {len(numbers)}")
        
        # Parse the W array format
        # Format can be:
        # c [w] - single character c with width w
        # c_first c_last w - range from c_first to c_last with width w
        
        i = 0
        entries = []
        while i < len(numbers):
            if i + 1 < len(numbers):
                # Check if next element starts with bracket (array)
                # For simplicity, assume alternating pattern
                char_code = numbers[i]
                if i + 2 < len(numbers) and numbers[i+1] > 1000:
                    # Looks like [c w] pair
                    width = numbers[i+1]
                    entries.append((char_code, char_code, width))
                    i += 2
                elif i + 2 < len(numbers):
                    # Could be range [c_first c_last w]
                    if numbers[i+1] - numbers[i] < 100:  # Reasonable range
                        c_last = numbers[i+1]
                        width = numbers[i+2]
                        entries.append((numbers[i], c_last, width))
                        i += 3
                    else:
                        # Assume single entry
                        i += 1
                else:
                    i += 1
            else:
                i += 1
        
        # Show some sample entries
        print("\nSample width mappings:")
        # Common ASCII characters
        test_chars = [
            (65, 'A'), (66, 'B'), (67, 'C'), (68, 'D'), (69, 'E'), (70, 'F'),
            (72, 'H'), (101, 'e'), (108, 'l'), (111, 'o'), (32, 'space')
        ]
        
        for code, name in test_chars:
            width = None
            for start, end, w in entries:
                if start <= code <= end:
                    width = w
                    break
            if width:
                print(f"  U+{code:04X} ({name}): width = {width}")
            else:
                print(f"  U+{code:04X} ({name}): uses default width")
        
        return entries
    else:
        print(f"No W array found in {pdf_path}")
        return []

if __name__ == "__main__":
    entries = extract_w_array("oxidize-pdf-core/test-pdfs/simple_custom.pdf")
