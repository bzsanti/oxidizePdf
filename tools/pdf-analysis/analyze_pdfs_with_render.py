#!/usr/bin/env python3
"""
Comprehensive PDF analysis script that tests both parsing and rendering capabilities.
This script uses both oxidize-pdf parser and oxidize-pdf-render to identify compatibility issues.
"""

import os
import sys
import subprocess
import re
import json
import time
from collections import defaultdict
from pathlib import Path
from datetime import datetime

def analyze_pdf_parsing(pdf_path):
    """Analyze a single PDF file using oxidize-pdf parser"""
    # Run oxidizepdf info command
    result = subprocess.run(
        ['cargo', 'run', '--bin', 'oxidizepdf', '--', 'info', str(pdf_path)],
        capture_output=True,
        text=True,
        cwd='.'
    )
    
    return result

def analyze_pdf_rendering(pdf_path):
    """Analyze a single PDF file using oxidize-pdf-render"""
    # Create temporary output path
    output_path = f"/tmp/render_test_{os.path.basename(pdf_path)}.png"
    
    # Try to render the PDF using oxidize-pdf-render example
    result = subprocess.run(
        ['cargo', 'run', '--manifest-path', '../oxidize-pdf-render/Cargo.toml', 
         '--example', 'test_render', '--', str(pdf_path), output_path],
        capture_output=True,
        text=True,
        timeout=10  # 10 second timeout per PDF
    )
    
    # Clean up output file if it exists
    if os.path.exists(output_path):
        os.remove(output_path)
    
    return result

def categorize_parse_error(stderr):
    """Categorize the parsing error based on stderr output"""
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
        return "EncryptionNotSupported"
    elif "EmptyFile" in stderr:
        return "EmptyFile"
    else:
        return "Other"

def categorize_render_error(stderr):
    """Categorize the rendering error based on stderr output"""
    if "NotImplemented" in stderr:
        return "NotImplemented: Missing feature"
    elif "InvalidColorSpace" in stderr:
        return "InvalidColorSpace"
    elif "InvalidFont" in stderr or "FontError" in stderr:
        return "FontError"
    elif "ImageDecoding" in stderr:
        return "ImageDecoding: Unsupported format"
    elif "ContentStream" in stderr:
        return "ContentStream: Invalid operations"
    elif "circular reference" in stderr:
        return "CircularReference"
    elif "encrypted" in stderr.lower():
        return "EncryptionNotSupported"
    elif "timeout" in stderr.lower():
        return "Timeout"
    else:
        return "Other rendering error"

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
            print("\nUsage: python3 analyze_pdfs_with_render.py [path_to_pdfs_or_pdf_file]")
            print("Default: tests/fixtures/")
            return
    else:
        print(f"Invalid path: {pdf_dir}")
        print("\nUsage: python3 analyze_pdfs_with_render.py [path_to_pdfs_or_pdf_file]")
        return
    
    print(f"🔍 PDF Compatibility Analysis with Rendering")
    print(f"===========================================")
    print(f"Analyzing {len(pdf_files)} PDFs from {pdf_dir}...")
    print(f"Testing both parsing (oxidize-pdf) and rendering (oxidize-pdf-render)\n")
    
    # Statistics
    parse_successful = 0
    parse_failed = 0
    render_successful = 0
    render_failed = 0
    both_successful = 0
    parse_only_success = 0
    render_only_success = 0
    both_failed = 0
    
    parse_error_types = defaultdict(int)
    render_error_types = defaultdict(int)
    error_details = defaultdict(list)
    compatibility_issues = []
    
    start_time = time.time()
    
    for i, pdf_file in enumerate(pdf_files):
        if i % 50 == 0 and i > 0:
            print(f"Progress: {i}/{len(pdf_files)}...")
        
        # Test parsing
        parse_result = analyze_pdf_parsing(pdf_file)
        parse_success = parse_result.returncode == 0
        
        # Test rendering
        render_success = False
        render_error = None
        
        try:
            render_result = analyze_pdf_rendering(pdf_file)
            render_success = render_result.returncode == 0
            if not render_success:
                render_error = categorize_render_error(render_result.stderr)
        except subprocess.TimeoutExpired:
            render_error = "Timeout"
        except Exception as e:
            render_error = f"Exception: {str(e)}"
        
        # Update statistics
        if parse_success:
            parse_successful += 1
        else:
            parse_failed += 1
            parse_error = categorize_parse_error(parse_result.stderr)
            parse_error_types[parse_error] += 1
        
        if render_success:
            render_successful += 1
        else:
            render_failed += 1
            if render_error:
                render_error_types[render_error] += 1
        
        # Categorize combined results
        if parse_success and render_success:
            both_successful += 1
        elif parse_success and not render_success:
            parse_only_success += 1
            compatibility_issues.append({
                'file': pdf_file.name,
                'issue': 'Parses but fails to render',
                'render_error': render_error
            })
        elif not parse_success and render_success:
            render_only_success += 1
            compatibility_issues.append({
                'file': pdf_file.name,
                'issue': 'Renders but fails to parse',
                'parse_error': parse_error if 'parse_error' in locals() else 'Unknown'
            })
        else:
            both_failed += 1
    
    end_time = time.time()
    duration = end_time - start_time
    
    # Print results
    print(f"\n📊 PDF Compatibility Analysis Report")
    print(f"====================================\n")
    print(f"Directory: {pdf_dir}")
    print(f"Total PDFs: {len(pdf_files)}")
    print(f"Analysis duration: {duration:.2f} seconds ({len(pdf_files)/duration:.1f} PDFs/sec)\n")
    
    print(f"📈 Parsing Results (oxidize-pdf):")
    print(f"  ✅ Successful: {parse_successful} ({parse_successful/len(pdf_files)*100:.1f}%)")
    print(f"  ❌ Failed: {parse_failed} ({parse_failed/len(pdf_files)*100:.1f}%)")
    
    print(f"\n🎨 Rendering Results (oxidize-pdf-render):")
    print(f"  ✅ Successful: {render_successful} ({render_successful/len(pdf_files)*100:.1f}%)")
    print(f"  ❌ Failed: {render_failed} ({render_failed/len(pdf_files)*100:.1f}%)")
    
    print(f"\n🔄 Combined Analysis:")
    print(f"  ✅✅ Both successful: {both_successful} ({both_successful/len(pdf_files)*100:.1f}%)")
    print(f"  ✅❌ Parse only: {parse_only_success} ({parse_only_success/len(pdf_files)*100:.1f}%)")
    print(f"  ❌✅ Render only: {render_only_success} ({render_only_success/len(pdf_files)*100:.1f}%)")
    print(f"  ❌❌ Both failed: {both_failed} ({both_failed/len(pdf_files)*100:.1f}%)")
    
    if parse_error_types:
        print(f"\n📋 Parse Error Categories:")
        for error_type, count in sorted(parse_error_types.items(), key=lambda x: x[1], reverse=True):
            print(f"  {error_type}: {count} PDFs ({count/parse_failed*100:.1f}% of failures)")
    
    if render_error_types:
        print(f"\n🎨 Render Error Categories:")
        for error_type, count in sorted(render_error_types.items(), key=lambda x: x[1], reverse=True):
            print(f"  {error_type}: {count} PDFs ({count/render_failed*100:.1f}% of failures)")
    
    # Compatibility issues
    if compatibility_issues:
        print(f"\n⚠️  Compatibility Issues Found: {len(compatibility_issues)}")
        print("PDFs that parse successfully but fail to render (top 10):")
        for issue in compatibility_issues[:10]:
            if issue['issue'] == 'Parses but fails to render':
                print(f"  - {issue['file']}: {issue['render_error']}")
    
    # Save detailed report
    report_data = {
        'analysis_date': datetime.now().isoformat(),
        'total_pdfs': len(pdf_files),
        'duration_seconds': duration,
        'parsing': {
            'successful': parse_successful,
            'failed': parse_failed,
            'success_rate': parse_successful/len(pdf_files)*100
        },
        'rendering': {
            'successful': render_successful,
            'failed': render_failed,
            'success_rate': render_successful/len(pdf_files)*100
        },
        'combined': {
            'both_successful': both_successful,
            'parse_only': parse_only_success,
            'render_only': render_only_success,
            'both_failed': both_failed
        },
        'parse_errors': dict(parse_error_types),
        'render_errors': dict(render_error_types),
        'compatibility_issues': compatibility_issues[:50]  # Save top 50 issues
    }
    
    report_filename = f"pdf_compatibility_report_{datetime.now().strftime('%Y%m%d_%H%M%S')}.json"
    with open(report_filename, 'w') as f:
        json.dump(report_data, f, indent=2)
    
    print(f"\n💾 Detailed report saved to: {report_filename}")
    
    # Improvement recommendations
    if parse_only_success > 0:
        print("\n" + "="*50)
        print("🔧 RENDERING IMPROVEMENT RECOMMENDATIONS:")
        print("="*50)
        print(f"\n{parse_only_success} PDFs parse correctly but fail to render.")
        print("This indicates missing features in the renderer:")
        
        top_render_errors = sorted(render_error_types.items(), key=lambda x: x[1], reverse=True)[:3]
        for error_type, count in top_render_errors:
            percentage = count/render_failed*100
            print(f"\n{error_type}: {count} PDFs ({percentage:.1f}% of render failures)")
            
            if "NotImplemented" in error_type:
                print("  → Implement missing PDF operations/features")
            elif "FontError" in error_type:
                print("  → Improve font handling and embedding support")
            elif "ImageDecoding" in error_type:
                print("  → Add support for more image formats")
            elif "ContentStream" in error_type:
                print("  → Fix content stream parsing issues")

if __name__ == "__main__":
    main()