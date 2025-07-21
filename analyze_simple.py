#!/usr/bin/env python3
"""Simple PDF analysis script to count errors by type"""

import os
import subprocess
import re
from collections import defaultdict

def main():
    fixtures_dir = "tests/fixtures"
    
    # Check if fixtures directory exists and has PDFs
    if os.path.exists(fixtures_dir) and os.path.isdir(fixtures_dir):
        pdf_files = [f for f in os.listdir(fixtures_dir) if f.endswith('.pdf')]
        if pdf_files:
            print(f"Found {len(pdf_files)} PDFs in {fixtures_dir}")
        else:
            print(f"No PDFs found in {fixtures_dir}")
            print("Please add PDF files to tests/fixtures/ directory")
            return
    else:
        print(f"Directory {fixtures_dir} does not exist!")
        print("Creating directory...")
        os.makedirs(fixtures_dir, exist_ok=True)
        print("Please add PDF files to tests/fixtures/ directory")
        return
    
    successful = 0
    failed = 0
    error_types = defaultdict(int)
    error_details = defaultdict(list)
    
    print(f"Analyzing {len(pdf_files)} PDFs...")
    
    for i, pdf_file in enumerate(pdf_files):
        if i % 50 == 0:
            print(f"Progress: {i}/{len(pdf_files)}...")
            
        pdf_path = os.path.join(fixtures_dir, pdf_file)
        
        # Run oxidizepdf info command
        result = subprocess.run(
            ['cargo', 'run', '--bin', 'oxidizepdf', '--', 'info', pdf_path],
            capture_output=True,
            text=True,
            cwd='.'
        )
        
        if result.returncode == 0:
            successful += 1
        else:
            failed += 1
            # Extract error type from stderr
            stderr = result.stderr
            
            # Look for specific error patterns
            if "MissingKey" in stderr:
                # Extract which key is missing
                match = re.search(r'MissingKey\("([^"]+)"\)', stderr)
                if match:
                    key = match.group(1)
                    error_types[f"MissingKey: {key}"] += 1
                    error_details[f"MissingKey: {key}"].append(pdf_file)
                else:
                    error_types["MissingKey"] += 1
                    error_details["MissingKey"].append(pdf_file)
            elif "Invalid header" in stderr or "InvalidHeader" in stderr:
                error_types["InvalidHeader"] += 1
                error_details["InvalidHeader"].append(pdf_file)
            elif "xref" in stderr.lower() or "XrefError" in stderr:
                error_types["XrefError"] += 1
                error_details["XrefError"].append(pdf_file)
            elif "PageCount" in stderr:
                error_types["PageCount: Other"] += 1
                error_details["PageCount: Other"].append(pdf_file)
            else:
                error_types["Other"] += 1
                error_details["Other"].append(pdf_file)
                if len(error_details["Other"]) <= 5:
                    print(f"  Other error in {pdf_file}: {stderr[:200]}")
    
    # Print results
    print(f"\nSimple PDF Analysis Report")
    print(f"=========================\n")
    print(f"Total PDFs: {len(pdf_files)}")
    print(f"Successful: {successful} ({successful/len(pdf_files)*100:.1f}%)")
    print(f"Failed: {failed} ({failed/len(pdf_files)*100:.1f}%)\n")
    
    print(f"Error Categories:")
    for error_type, count in sorted(error_types.items(), key=lambda x: x[1], reverse=True):
        print(f"  {error_type}: {count} PDFs")
    
    print(f"\nFirst 100 Failed PDFs:")
    count = 0
    for error_type in sorted(error_types.keys()):
        for pdf in error_details[error_type][:10]:  # Show up to 10 per category
            print(f"  {pdf}: {error_type}")
            count += 1
            if count >= 100:
                break
        if count >= 100:
            break

if __name__ == "__main__":
    main()