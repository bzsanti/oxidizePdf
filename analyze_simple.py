#!/usr/bin/env python3
"""Simple script to analyze the 3 specific PDFs with circular reference issues."""

import subprocess
import os

problematic_pdfs = [
    "tests/fixtures/Course_Glossary_SUPPLY_LIST.pdf",
    "tests/fixtures/liarsandoutliers_enablingthetrustthatsocietyneedstothrive.pdf",
    "tests/fixtures/cryptography_engineering_design_principles_and_practical_applications.pdf"
]

print("=== Analyzing Specific Problematic PDFs ===\n")

for pdf_path in problematic_pdfs:
    print(f"\n{'='*60}")
    print(f"Analyzing: {os.path.basename(pdf_path)}")
    print(f"File size: {os.path.getsize(pdf_path):,} bytes")
    print(f"{'='*60}\n")
    
    # Test with oxidize-pdf CLI
    print("Testing with oxidize-pdf CLI:")
    try:
        result = subprocess.run(
            ["cargo", "run", "--bin", "oxidizepdf", "--", "info", pdf_path],
            capture_output=True,
            text=True,
            timeout=10
        )
        if result.returncode == 0:
            print("SUCCESS!")
            print(result.stdout)
        else:
            print("FAILED!")
            print("STDERR:")
            print(result.stderr)
    except subprocess.TimeoutExpired:
        print("TIMEOUT after 10 seconds")
    except Exception as e:
        print(f"EXCEPTION: {str(e)}")
    
    # Test with pdfinfo
    print("\n\nTesting with pdfinfo:")
    try:
        result = subprocess.run(
            ["pdfinfo", pdf_path],
            capture_output=True,
            text=True,
            timeout=5
        )
        if result.returncode == 0:
            print("SUCCESS!")
            print(result.stdout[:500] + "..." if len(result.stdout) > 500 else result.stdout)
        else:
            print("FAILED!")
            print(result.stderr)
    except:
        print("pdfinfo not available")
    
    print("\n")