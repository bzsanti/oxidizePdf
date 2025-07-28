#!/usr/bin/env python3
"""Comprehensive PDF analysis script that can analyze PDFs from any location"""

import os
import sys
import subprocess
import re
from collections import defaultdict
from pathlib import Path

def analyze_pdf(pdf_path):
    """Analyze a single PDF file and return the result"""
    # Run oxidizepdf info command
    result = subprocess.run(
        ['cargo', 'run', '--bin', 'oxidizepdf', '--', 'info', str(pdf_path)],
        capture_output=True,
        text=True,
        cwd='.'
    )
    
    return result

def categorize_error(stderr):
    """Categorize the error based on stderr output"""
    if "circular reference" in stderr:
        return "PageTreeError: circular reference"
    elif "MissingKey" in stderr:
        match = re.search(r'MissingKey\("([^"]+)"\)', stderr)
        if match:
            key = match.group(1)
            return f"MissingKey: {key}"
        return "MissingKey"
    elif "Invalid header" in stderr or "InvalidHeader" in stderr:
        return "InvalidHeader"
    elif "xref" in stderr.lower() or "XrefError" in stderr:
        if "Invalid xref table" in stderr:
            return "XrefError: Invalid xref table"
        return "XrefError"
    elif "PageCount" in stderr:
        return "PageCount: Other"
    elif "PageTreeError" in stderr:
        return "PageTreeError: Other"
    elif "encrypted" in stderr.lower() or "encryption" in stderr.lower():
        return "ParseError::Other: Encrypted PDF"
    else:
        return "Other"

def main():
    # Get PDF directory from command line or use default
    if len(sys.argv) > 1:
        pdf_dir = sys.argv[1]
    else:
        pdf_dir = "tests/fixtures"
    
    # Find all PDF files
    pdf_path = Path(pdf_dir)
    if pdf_path.is_file() and pdf_path.suffix == '.pdf':
        # Single PDF file specified
        pdf_files = [pdf_path]
        pdf_dir = pdf_path.parent
    elif pdf_path.is_dir():
        # Directory specified, find all PDFs
        pdf_files = list(pdf_path.glob("*.pdf"))
        if not pdf_files:
            print(f"No PDF files found in {pdf_dir}")
            print("\nUsage: python3 analyze_pdfs.py [path_to_pdfs_or_pdf_file]")
            print("Default: tests/fixtures/")
            return
    else:
        print(f"Invalid path: {pdf_dir}")
        print("\nUsage: python3 analyze_pdfs.py [path_to_pdfs_or_pdf_file]")
        return
    
    print(f"Analyzing {len(pdf_files)} PDFs from {pdf_dir}...")
    
    successful = 0
    failed = 0
    error_types = defaultdict(int)
    error_details = defaultdict(list)
    
    for i, pdf_file in enumerate(pdf_files):
        if i % 50 == 0 and i > 0:
            print(f"Progress: {i}/{len(pdf_files)}...")
        
        result = analyze_pdf(pdf_file)
        
        if result.returncode == 0:
            successful += 1
        else:
            failed += 1
            error_type = categorize_error(result.stderr)
            error_types[error_type] += 1
            error_details[error_type].append(pdf_file.name)
    
    # Print results
    print(f"\nPDF Analysis Report")
    print(f"===================\n")
    print(f"Directory: {pdf_dir}")
    print(f"Total PDFs: {len(pdf_files)}")
    print(f"Successful: {successful} ({successful/len(pdf_files)*100:.1f}%)")
    print(f"Failed: {failed} ({failed/len(pdf_files)*100:.1f}%)\n")
    
    if error_types:
        print(f"Error Categories:")
        for error_type, count in sorted(error_types.items(), key=lambda x: x[1], reverse=True):
            print(f"  {error_type}: {count} PDFs ({count/len(pdf_files)*100:.1f}%)")
        
        print(f"\nSample Failed PDFs (up to 5 per category):")
        for error_type in sorted(error_types.keys()):
            print(f"\n{error_type}:")
            for pdf in error_details[error_type][:5]:
                print(f"  - {pdf}")
            if len(error_details[error_type]) > 5:
                print(f"  ... and {len(error_details[error_type])-5} more")
    
    # Summary for improvement focus
    if failed > 0:
        print("\n" + "="*50)
        print("IMPROVEMENT RECOMMENDATIONS:")
        print("="*50)
        
        top_errors = sorted(error_types.items(), key=lambda x: x[1], reverse=True)[:3]
        for error_type, count in top_errors:
            percentage = count/failed*100
            print(f"\n{error_type}: {count} PDFs ({percentage:.1f}% of failures)")
            
            if "circular reference" in error_type:
                print("  → Implement cycle detection in page tree traversal")
            elif "XrefError" in error_type:
                print("  → Improve XRef recovery and stream handling")
            elif "Encrypted" in error_type:
                print("  → Add support for encrypted PDFs")
            elif "MissingKey" in error_type:
                print("  → Add defensive handling for missing dictionary keys")

if __name__ == "__main__":
    main()