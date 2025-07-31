#!/usr/bin/env python3
"""Validate PDF structure and check for common issues"""

import sys
import PyPDF2
from PyPDF2 import PdfReader

def validate_pdf(filename):
    print(f"Validating {filename}...")
    
    try:
        with open(filename, 'rb') as f:
            reader = PdfReader(f)
            
            print(f"✓ PDF is readable")
            print(f"  Pages: {len(reader.pages)}")
            
            # Check metadata
            if reader.metadata:
                print("  Metadata:")
                if reader.metadata.title:
                    print(f"    Title: {reader.metadata.title}")
                if reader.metadata.author:
                    print(f"    Author: {reader.metadata.author}")
            
            # Check for outlines (bookmarks)
            try:
                outlines = reader.outline
                if outlines:
                    print(f"  ✓ Has outlines/bookmarks")
                    
                    def count_outlines(outline_list, level=0):
                        count = 0
                        for item in outline_list:
                            if isinstance(item, list):
                                count += count_outlines(item, level + 1)
                            else:
                                count += 1
                                if level < 2:  # Only show first 2 levels
                                    print(f"    {'  ' * level}- {item.title}")
                        return count
                    
                    total = count_outlines(outlines)
                    print(f"    Total bookmarks: {total}")
                else:
                    print("  ✗ No outlines/bookmarks found")
            except Exception as e:
                print(f"  ✗ Error reading outlines: {e}")
            
            # Check for forms
            try:
                if hasattr(reader, 'get_form_text_fields'):
                    form_fields = reader.get_form_text_fields()
                    if form_fields:
                        print(f"  ✓ Has form fields: {len(form_fields)}")
                        for name, value in list(form_fields.items())[:3]:
                            print(f"    - {name}: {value}")
                    else:
                        print("  ✗ No form fields found")
                
                # Check for AcroForm
                if hasattr(reader, '_root_object') and '/AcroForm' in reader._root_object:
                    print("  ✓ Has AcroForm dictionary")
            except Exception as e:
                print(f"  ✗ Error reading forms: {e}")
            
            # Check pages for annotations
            for i, page in enumerate(reader.pages):
                if '/Annots' in page:
                    annots = page['/Annots']
                    if annots:
                        print(f"  Page {i+1} has {len(annots)} annotations")
                        
    except Exception as e:
        print(f"✗ Error reading PDF: {e}")
        print(f"  Error type: {type(e).__name__}")
        return False
    
    return True

if __name__ == "__main__":
    if len(sys.argv) < 2:
        print("Usage: python validate_pdf.py <pdf_file>")
        sys.exit(1)
    
    validate_pdf(sys.argv[1])