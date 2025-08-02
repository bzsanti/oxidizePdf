#!/usr/bin/env python3
"""
Quick test to verify our generated PDF can be parsed by other tools
"""

import subprocess
import os

def test_pdf_with_external_tools():
    """Test that our generated PDF can be parsed by external tools"""
    
    # Generate a test PDF
    print("Generating test PDF...")
    result = subprocess.run([
        "cargo", "run", "--example", "hello_world", "-p", "oxidize-pdf-core"
    ], capture_output=True, text=True, cwd="../")
    
    if result.returncode != 0:
        print("Failed to generate PDF:")
        print(result.stderr)
        return False
    
    # Check if PDF was created
    pdf_path = "../hello_world.pdf"
    if not os.path.exists(pdf_path):
        print("PDF file was not created")
        return False
    
    print(f"PDF generated successfully: {pdf_path}")
    
    # Try to validate with qpdf if available
    try:
        result = subprocess.run([
            "qpdf", "--check", pdf_path
        ], capture_output=True, text=True)
        
        if result.returncode == 0:
            print("✓ PDF validation with qpdf: PASSED")
            return True
        else:
            print("⚠ PDF validation with qpdf: FAILED")
            print(result.stderr)
            return False
    except FileNotFoundError:
        print("⚠ qpdf not available, skipping external validation")
        return True

if __name__ == "__main__":
    success = test_pdf_with_external_tools()
    exit(0 if success else 1)