#!/usr/bin/env python3
"""Quick analysis to check Phase 1 improvements."""

import subprocess
import os
import json
import time
from concurrent.futures import ThreadPoolExecutor, as_completed

def analyze_pdf(pdf_path):
    """Analyze a single PDF."""
    result = {
        'file': os.path.basename(pdf_path),
        'size': os.path.getsize(pdf_path),
        'status': 'unknown',
        'error': None,
        'pdf_version': None
    }
    
    try:
        cmd_result = subprocess.run(
            ["cargo", "run", "--bin", "oxidizepdf", "--", "info", pdf_path],
            capture_output=True,
            text=True,
            timeout=5
        )
        
        if cmd_result.returncode == 0:
            result['status'] = 'success'
            # Extract PDF version from output
            for line in cmd_result.stdout.split('\n'):
                if 'PDF Version:' in line:
                    result['pdf_version'] = line.split(':', 1)[1].strip()
        else:
            result['status'] = 'error'
            result['error'] = cmd_result.stderr.strip()
    except subprocess.TimeoutExpired:
        result['status'] = 'timeout'
        result['error'] = 'Timed out after 5 seconds'
    except Exception as e:
        result['status'] = 'exception'
        result['error'] = str(e)
    
    return result

def main():
    print("Phase 1 PDF Analysis - Lenient Parsing Results")
    print("==============================================\n")
    
    # Get all PDFs
    pdf_files = []
    fixtures_dir = "tests/fixtures"
    
    if os.path.exists(fixtures_dir):
        for f in os.listdir(fixtures_dir):
            if f.endswith('.pdf'):
                pdf_files.append(os.path.join(fixtures_dir, f))
    
    print(f"Found {len(pdf_files)} PDF files to analyze\n")
    
    # Analyze PDFs in parallel
    results = []
    start_time = time.time()
    
    with ThreadPoolExecutor(max_workers=8) as executor:
        # Submit all tasks
        future_to_pdf = {executor.submit(analyze_pdf, pdf): pdf for pdf in pdf_files}
        
        # Process completed tasks
        completed = 0
        for future in as_completed(future_to_pdf):
            completed += 1
            pdf = future_to_pdf[future]
            try:
                result = future.result()
                results.append(result)
                
                # Show progress
                if completed % 50 == 0:
                    print(f"Progress: {completed}/{len(pdf_files)} PDFs analyzed...")
                    
            except Exception as exc:
                print(f"PDF {pdf} generated an exception: {exc}")
    
    total_time = time.time() - start_time
    
    # Analyze results
    success_count = sum(1 for r in results if r['status'] == 'success')
    error_count = sum(1 for r in results if r['status'] == 'error')
    timeout_count = sum(1 for r in results if r['status'] == 'timeout')
    exception_count = sum(1 for r in results if r['status'] == 'exception')
    
    # Group errors by type
    error_types = {}
    for r in results:
        if r['status'] == 'error' and r['error']:
            # Extract main error type
            if 'Encryption not supported' in r['error']:
                error_type = 'EncryptionNotSupported'
            elif 'Invalid header' in r['error']:
                error_type = 'InvalidHeader'
            elif 'Invalid cross-reference' in r['error']:
                error_type = 'InvalidXRef'
            elif 'Character encoding' in r['error']:
                error_type = 'CharacterEncoding'
            elif 'Circular reference' in r['error']:
                error_type = 'CircularReference'
            elif 'Invalid object reference' in r['error']:
                error_type = 'InvalidObjectReference'
            else:
                error_type = 'Other'
            
            error_types[error_type] = error_types.get(error_type, 0) + 1
    
    # Print summary
    print(f"\n\nAnalysis Results (Phase 1 - Lenient Parsing)")
    print("=" * 50)
    print(f"Total PDFs analyzed: {len(results)}")
    print(f"Successful: {success_count} ({success_count/len(results)*100:.2f}%)")
    print(f"Errors: {error_count} ({error_count/len(results)*100:.2f}%)")
    print(f"Timeouts: {timeout_count}")
    print(f"Exceptions: {exception_count}")
    print(f"Total time: {total_time:.2f}s")
    
    print("\nError Breakdown:")
    for error_type, count in sorted(error_types.items(), key=lambda x: x[1], reverse=True):
        print(f"  {error_type}: {count}")
    
    # Show improvements from baseline
    print("\nImprovement from Baseline:")
    print("  Baseline: 550/743 successful (74.0%)")
    print(f"  Phase 1:  {success_count}/{len(results)} successful ({success_count/len(results)*100:.2f}%)")
    improvement = (success_count/len(results)*100) - 74.0
    print(f"  Improvement: {improvement:+.2f}%")
    
    # Save detailed results
    with open('phase1_analysis_results.json', 'w') as f:
        json.dump({
            'total_pdfs': len(results),
            'successful': success_count,
            'errors': error_count,
            'timeouts': timeout_count,
            'exceptions': exception_count,
            'success_rate': success_count/len(results)*100,
            'total_time': total_time,
            'error_types': error_types,
            'results': results
        }, f, indent=2)
    
    print("\nDetailed results saved to phase1_analysis_results.json")
    
    # Show some specific successes
    print("\nNewly Successful PDFs (sample):")
    baseline_failures = [
        "Course_Glossary_SUPPLY_LIST.pdf",
        "liarsandoutliers_enablingthetrustthatsocietyneedstothrive.pdf",
        "cryptography_engineering_design_principles_and_practical_applications.pdf"
    ]
    
    for r in results:
        if r['status'] == 'success' and r['file'] in baseline_failures:
            print(f"  ✓ {r['file']}")

if __name__ == "__main__":
    main()