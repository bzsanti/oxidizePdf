#!/usr/bin/env python3
"""Test the 3 specific PDFs for circular reference errors."""

import sys
sys.path.append('oxidize-pdf-core')

try:
    import oxidize_pdf
    
    pdfs = [
        "tests/fixtures/Course_Glossary_SUPPLY_LIST.pdf",
        "tests/fixtures/liarsandoutliers_enablingthetrustthatsocietyneedstothrive.pdf", 
        "tests/fixtures/cryptography_engineering_design_principles_and_practical_applications.pdf"
    ]
    
    for pdf_path in pdfs:
        print(f"\n{'='*60}")
        print(f"Testing: {pdf_path}")
        print(f"{'='*60}")
        
        try:
            doc = oxidize_pdf.Document.from_path(pdf_path)
            print(f"✓ Successfully opened!")
            print(f"  Pages: {doc.page_count()}")
            print(f"  Version: {doc.pdf_version()}")
            
            # Try to access first page
            try:
                page = doc.get_page(0)
                print(f"  First page accessed successfully")
            except Exception as e:
                print(f"  Error accessing first page: {e}")
                
        except Exception as e:
            print(f"✗ Failed to open: {type(e).__name__}: {e}")
            
except ImportError:
    print("oxidize_pdf module not available, using subprocess...")
    import subprocess
    import os
    
    pdfs = [
        "tests/fixtures/Course_Glossary_SUPPLY_LIST.pdf",
        "tests/fixtures/liarsandoutliers_enablingthetrustthatsocietyneedstothrive.pdf", 
        "tests/fixtures/cryptography_engineering_design_principles_and_practical_applications.pdf"
    ]
    
    for pdf_path in pdfs:
        print(f"\n{'='*60}")
        print(f"Testing: {os.path.basename(pdf_path)}")
        print(f"File size: {os.path.getsize(pdf_path):,} bytes")
        print(f"{'='*60}")
        
        # Try with oxidizepdf CLI
        try:
            result = subprocess.run(
                ["cargo", "run", "--bin", "oxidizepdf", "--", "info", pdf_path],
                capture_output=True,
                text=True,
                timeout=30
            )
            
            if result.returncode == 0:
                print("✓ SUCCESS with CLI")
                print(result.stdout)
            else:
                print("✗ FAILED with CLI")
                print("STDERR:")
                print(result.stderr)
                
        except subprocess.TimeoutExpired:
            print("✗ TIMEOUT after 30 seconds")
        except Exception as e:
            print(f"✗ Exception: {e}")