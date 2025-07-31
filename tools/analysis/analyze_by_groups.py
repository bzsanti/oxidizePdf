#!/usr/bin/env python3
"""Analyze PDFs in groups to avoid timeouts"""

import os
import subprocess
import sys
from pathlib import Path

def analyze_group(pdf_files, group_num, total_groups):
    """Analyze a group of PDF files"""
    print(f"\n{'='*60}")
    print(f"Analyzing Group {group_num}/{total_groups} ({len(pdf_files)} PDFs)")
    print(f"{'='*60}")
    
    successful = 0
    failed = 0
    errors = {}
    
    for i, pdf_path in enumerate(pdf_files):
        if i % 10 == 0:
            print(f"Progress: {i}/{len(pdf_files)}...")
            
        # Run oxidizepdf info command
        result = subprocess.run(
            ['cargo', 'run', '--release', '--bin', 'oxidizepdf', '--', 'info', pdf_path],
            capture_output=True,
            text=True
        )
        
        if result.returncode == 0:
            successful += 1
        else:
            failed += 1
            # Categorize error
            if "circular reference" in result.stderr:
                error_type = "CircularReference"
            elif "Encrypted" in result.stderr or "encrypted" in result.stderr:
                error_type = "Encrypted"
            elif "XrefError" in result.stderr or "xref" in result.stderr.lower():
                error_type = "XrefError"
            elif "InvalidHeader" in result.stderr:
                error_type = "InvalidHeader"
            else:
                error_type = "Other"
            
            errors[error_type] = errors.get(error_type, 0) + 1
    
    return successful, failed, errors

def main():
    fixtures_dir = "tests/fixtures"
    
    # Get all PDF files
    pdf_files = list(Path(fixtures_dir).glob("*.pdf"))
    total_pdfs = len(pdf_files)
    
    print(f"Found {total_pdfs} PDFs in {fixtures_dir}")
    
    # Process in groups of 50
    group_size = 50
    total_successful = 0
    total_failed = 0
    all_errors = {}
    
    for i in range(0, total_pdfs, group_size):
        group = pdf_files[i:i+group_size]
        group_num = (i // group_size) + 1
        total_groups = (total_pdfs + group_size - 1) // group_size
        
        successful, failed, errors = analyze_group(group, group_num, total_groups)
        
        total_successful += successful
        total_failed += failed
        
        # Merge errors
        for error_type, count in errors.items():
            all_errors[error_type] = all_errors.get(error_type, 0) + count
    
    # Print final summary
    print(f"\n{'='*60}")
    print(f"FINAL ANALYSIS SUMMARY")
    print(f"{'='*60}")
    print(f"Total PDFs: {total_pdfs}")
    print(f"Successful: {total_successful} ({total_successful/total_pdfs*100:.1f}%)")
    print(f"Failed: {total_failed} ({total_failed/total_pdfs*100:.1f}%)")
    
    if all_errors:
        print(f"\nError Breakdown:")
        for error_type, count in sorted(all_errors.items(), key=lambda x: x[1], reverse=True):
            print(f"  {error_type}: {count} PDFs ({count/total_failed*100:.1f}% of failures)")

if __name__ == "__main__":
    main()