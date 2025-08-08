#!/usr/bin/env python3
"""Check if PDFs have visible text content"""

import PyPDF2
import sys
import os

def check_pdf(filepath):
    """Check if a PDF has text content"""
    try:
        with open(filepath, 'rb') as f:
            reader = PyPDF2.PdfReader(f)
            num_pages = len(reader.pages)
            
            print(f"\n{'='*60}")
            print(f"File: {os.path.basename(filepath)}")
            print(f"Pages: {num_pages}")
            print(f"{'='*60}")
            
            has_text = False
            for i, page in enumerate(reader.pages):
                text = page.extract_text()
                if text and text.strip():
                    has_text = True
                    print(f"\nPage {i+1} text preview:")
                    # Show first 200 chars
                    preview = text[:200].replace('\n', ' ').strip()
                    print(f"  {preview}...")
                else:
                    print(f"\nPage {i+1}: NO TEXT FOUND")
            
            return has_text
    except Exception as e:
        print(f"Error reading {filepath}: {e}")
        return False

def main():
    pdf_files = [
        "simple_text_test.pdf",
        "unicode_comprehensive.pdf", 
        "unicode_exhaustive.pdf",
        "debug_standard.pdf"
    ]
    
    print("Checking PDF files for visible text content...")
    
    results = {}
    for pdf_file in pdf_files:
        if os.path.exists(pdf_file):
            results[pdf_file] = check_pdf(pdf_file)
        else:
            print(f"File not found: {pdf_file}")
            results[pdf_file] = None
    
    print(f"\n{'='*60}")
    print("SUMMARY:")
    print(f"{'='*60}")
    for pdf_file, has_text in results.items():
        if has_text is None:
            status = "NOT FOUND"
        elif has_text:
            status = "✅ HAS TEXT"
        else:
            status = "❌ NO TEXT"
        print(f"  {pdf_file}: {status}")

if __name__ == "__main__":
    main()