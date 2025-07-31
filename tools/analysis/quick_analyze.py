#!/usr/bin/env python3
"""Quick PDF analysis using the compiled binary"""

import os
import subprocess
import sys
from pathlib import Path
from collections import defaultdict

def main():
    fixtures_dir = "tests/fixtures"
    
    # Use the compiled binary directly
    binary_path = "target/release/oxidizepdf"
    if not Path(binary_path).exists():
        print("Error: Binary not found. Please run: cargo build --release")
        sys.exit(1)
    
    # Get all PDF files
    pdf_files = list(Path(fixtures_dir).glob("*.pdf"))
    total_pdfs = len(pdf_files)
    
    print(f"Found {total_pdfs} PDFs in {fixtures_dir}")
    print("Starting quick analysis...")
    
    successful = 0
    failed = 0
    error_types = defaultdict(int)
    error_examples = defaultdict(list)
    
    # Process all PDFs
    for i, pdf_path in enumerate(pdf_files):
        if i % 50 == 0:
            print(f"Progress: {i}/{total_pdfs} ({i/total_pdfs*100:.1f}%)...")
            
        # Run oxidizepdf info command
        result = subprocess.run(
            [binary_path, 'info', str(pdf_path)],
            capture_output=True,
            text=True
        )
        
        if result.returncode == 0:
            successful += 1
        else:
            failed += 1
            # Categorize error
            stderr = result.stderr
            
            if "Circular reference detected" in stderr:
                error_type = "CircularReference"
            elif "PDF is encrypted" in stderr:
                error_type = "Encrypted"
            elif "Invalid xref" in stderr or "XrefError" in stderr:
                error_type = "XrefError"
            elif "Invalid header" in stderr:
                error_type = "InvalidHeader"
            elif "Invalid reference" in stderr:
                error_type = "InvalidReference"
            elif "Parsing timeout" in stderr:
                error_type = "Timeout"
            else:
                error_type = "Other"
            
            error_types[error_type] += 1
            if len(error_examples[error_type]) < 3:
                error_examples[error_type].append(pdf_path.name)
    
    # Print summary
    print(f"\n{'='*60}")
    print(f"PDF ANALYSIS SUMMARY (with improvements)")
    print(f"{'='*60}")
    print(f"Total PDFs: {total_pdfs}")
    print(f"Successful: {successful} ({successful/total_pdfs*100:.1f}%)")
    print(f"Failed: {failed} ({failed/total_pdfs*100:.1f}%)")
    
    if error_types:
        print(f"\nError Breakdown:")
        for error_type, count in sorted(error_types.items(), key=lambda x: x[1], reverse=True):
            percentage = count/failed*100 if failed > 0 else 0
            print(f"  {error_type}: {count} PDFs ({percentage:.1f}% of failures)")
            if error_examples[error_type]:
                print(f"    Examples: {', '.join(error_examples[error_type][:3])}")

if __name__ == "__main__":
    main()