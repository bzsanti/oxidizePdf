#\!/usr/bin/env python3
"""Test if the issue is with text positioning calculation"""

import PyPDF2
import sys

def analyze_text_spacing(pdf_path):
    try:
        with open(pdf_path, 'rb') as f:
            reader = PyPDF2.PdfReader(f)
            if len(reader.pages) > 0:
                page = reader.pages[0]
                text = page.extract_text()
                print(f"Extracted text from {pdf_path}:")
                print(text)
                print("\nCharacter analysis:")
                lines = text.split('\n')
                for i, line in enumerate(lines[:5]):
                    if line.strip():
                        print(f"Line {i+1}: '{line}'")
                        print(f"  Length: {len(line)} chars")
                        if len(line) > 0:
                            # Check spacing between chars
                            spaces = line.count(' ')
                            print(f"  Spaces: {spaces}")
    except Exception as e:
        print(f"Error: {e}")

if __name__ == "__main__":
    analyze_text_spacing("oxidize-pdf-core/test-pdfs/simple_custom.pdf")
    print("\n" + "="*60 + "\n")
    analyze_text_spacing("oxidize-pdf-core/test-pdfs/spacing_test.pdf")
