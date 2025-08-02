#!/usr/bin/env python3
"""Quick sample analysis to check Phase 1 improvements."""

import subprocess
import os
import random

# Specific PDFs we know had circular reference issues
known_problematic = [
    "Course_Glossary_SUPPLY_LIST.pdf",
    "liarsandoutliers_enablingthetrustthatsocietyneedstothrive.pdf", 
    "cryptography_engineering_design_principles_and_practical_applications.pdf",
    "04.ANNEX 2 PQQ rev.01.pdf",
    "1002579.pdf",
    "20220603 QE Biometrical Consent signed SFM.pdf",
    "240043_Quintas Energy.pdf",
    "ANEXO I Tecnicas de Poda.pdf",
    "BRUC_Intersnack_PPA_(Execution_Version_-_2024-08-13).pdf",
    "CertificadoPrimas-2021.pdf"
]

def analyze_pdf(pdf_path):
    """Analyze a single PDF."""
    try:
        result = subprocess.run(
            ["cargo", "run", "--release", "--bin", "oxidizepdf", "--", "info", pdf_path],
            capture_output=True,
            text=True,
            timeout=5
        )
        
        if result.returncode == 0:
            return "SUCCESS"
        else:
            # Extract error type
            stderr = result.stderr.strip()
            if "Circular reference" in stderr:
                return "CIRCULAR_REF"
            elif "Encryption not supported" in stderr:
                return "ENCRYPTED"
            elif "Invalid header" in stderr:
                return "INVALID_HEADER"
            elif "Character encoding" in stderr:
                return "CHAR_ENCODING"
            else:
                return "OTHER_ERROR"
    except subprocess.TimeoutExpired:
        return "TIMEOUT"
    except Exception:
        return "EXCEPTION"

def main():
    print("Phase 1 Sample Analysis - Testing Known Problematic PDFs")
    print("========================================================\n")
    
    fixtures_dir = "tests/fixtures"
    results = {}
    
    # Test known problematic PDFs
    print("Testing PDFs that had circular reference errors:\n")
    for pdf_name in known_problematic:
        pdf_path = os.path.join(fixtures_dir, pdf_name)
        if os.path.exists(pdf_path):
            print(f"Testing: {pdf_name}...", end=" ", flush=True)
            status = analyze_pdf(pdf_path)
            results[pdf_name] = status
            
            if status == "SUCCESS":
                print("✓ SUCCESS!")
            else:
                print(f"✗ {status}")
        else:
            print(f"Not found: {pdf_name}")
    
    # Test a random sample of other PDFs
    print("\n\nTesting random sample of other PDFs:\n")
    all_pdfs = [f for f in os.listdir(fixtures_dir) if f.endswith('.pdf')]
    other_pdfs = [f for f in all_pdfs if f not in known_problematic]
    sample_pdfs = random.sample(other_pdfs, min(20, len(other_pdfs)))
    
    for pdf_name in sample_pdfs:
        pdf_path = os.path.join(fixtures_dir, pdf_name)
        print(f"Testing: {pdf_name[:50]}...", end=" ", flush=True)
        status = analyze_pdf(pdf_path)
        results[pdf_name] = status
        
        if status == "SUCCESS":
            print("✓")
        else:
            print(f"✗ {status}")
    
    # Summary
    print("\n\nSummary:")
    print("========")
    total = len(results)
    success = sum(1 for s in results.values() if s == "SUCCESS")
    circular_ref = sum(1 for s in results.values() if s == "CIRCULAR_REF")
    encrypted = sum(1 for s in results.values() if s == "ENCRYPTED")
    
    print(f"Total tested: {total}")
    print(f"Successful: {success} ({success/total*100:.1f}%)")
    print(f"Circular reference errors: {circular_ref}")
    print(f"Encrypted PDFs: {encrypted}")
    print(f"Other errors: {total - success - circular_ref - encrypted}")
    
    # Check if known problematic PDFs are fixed
    print("\n\nCircular Reference Fix Status:")
    print("==============================")
    fixed_count = 0
    for pdf in known_problematic:
        if pdf in results:
            if results[pdf] == "SUCCESS":
                print(f"✓ FIXED: {pdf}")
                fixed_count += 1
            else:
                print(f"✗ Still failing ({results[pdf]}): {pdf}")
    
    print(f"\nFixed {fixed_count}/{len([p for p in known_problematic if p in results])} known circular reference issues")

if __name__ == "__main__":
    main()