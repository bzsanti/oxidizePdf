#!/usr/bin/env python3
"""Test script to check if circular reference errors have been fixed"""

import subprocess
import os
import sys

def test_pdf(pdf_path):
    """Test if a PDF can be opened without circular reference errors"""
    test_code = f'''
use oxidize_pdf::parser::PdfReader;

fn main() {{
    let path = "{pdf_path}";
    match PdfReader::open(path) {{
        Ok(reader) => {{
            let document = reader.into_document();
            match document.page_count() {{
                Ok(count) => {{
                    println!("✓ PDF has {{}} pages", count);
                    match document.get_page(0) {{
                        Ok(_) => println!("✓ First page accessible"),
                        Err(e) => {{
                            println!("✗ First page error: {{}}", e);
                            std::process::exit(1);
                        }}
                    }}
                }}
                Err(e) => {{
                    println!("✗ Page count error: {{}}", e);
                    std::process::exit(1);
                }}
            }}
        }}
        Err(e) => {{
            println!("✗ Open error: {{}}", e);
            std::process::exit(1);
        }}
    }}
}}
'''
    
    # Write test code to temporary file
    with open('/tmp/test_pdf.rs', 'w') as f:
        f.write(test_code)
    
    # Compile and run
    try:
        # Compile
        result = subprocess.run(
            ['rustc', '/tmp/test_pdf.rs', '-o', '/tmp/test_pdf', 
             '--extern', 'oxidize_pdf=target/release/liboxide_pdf.rlib',
             '-L', 'target/release/deps'],
            capture_output=True,
            text=True,
            cwd='oxidize-pdf-core'
        )
        
        if result.returncode != 0:
            print(f"Compilation failed: {result.stderr}")
            return False
            
        # Run
        result = subprocess.run(
            ['/tmp/test_pdf'],
            capture_output=True,
            text=True
        )
        
        print(result.stdout)
        return result.returncode == 0
        
    except Exception as e:
        print(f"Error: {e}")
        return False
    finally:
        # Cleanup
        for f in ['/tmp/test_pdf.rs', '/tmp/test_pdf']:
            if os.path.exists(f):
                os.remove(f)

# Test PDFs that were failing with circular references
test_pdfs = [
    "tests/fixtures/Course_Glossary_SUPPLY_LIST.pdf",
    "tests/fixtures/04.ANNEX 2 PQQ rev.01.pdf",
    "tests/fixtures/1002579.pdf",
]

print("Testing PDFs that were failing with circular reference errors...")
print("=" * 60)

success_count = 0
for pdf in test_pdfs:
    if os.path.exists(pdf):
        print(f"\nTesting: {os.path.basename(pdf)}")
        if test_pdf(pdf):
            success_count += 1
    else:
        print(f"\nSkipping: {pdf} (not found)")

print("\n" + "=" * 60)
print(f"Results: {success_count}/{len(test_pdfs)} PDFs passed")