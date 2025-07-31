#!/usr/bin/env python3
"""
Generate minimal test PDFs for the test suite.
This is a temporary solution until the Rust code compiles.
"""

import os
import json

def create_minimal_pdf():
    """Create the absolute minimal valid PDF."""
    return b"""%PDF-1.4
%\xe2\xe3\xcf\xd3
1 0 obj
<< /Type /Catalog /Pages 2 0 R >>
endobj
2 0 obj
<< /Type /Pages /Kids [3 0 R] /Count 1 >>
endobj
3 0 obj
<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] >>
endobj
xref
0 4
0000000000 65535 f 
0000000015 00000 n 
0000000068 00000 n 
0000000125 00000 n 
trailer
<< /Size 4 /Root 1 0 R >>
startxref
203
%%EOF"""

def create_pdf_with_info():
    """Create a minimal PDF with metadata."""
    return b"""%PDF-1.4
%\xe2\xe3\xcf\xd3
1 0 obj
<< /Type /Catalog /Pages 2 0 R >>
endobj
2 0 obj
<< /Type /Pages /Kids [3 0 R] /Count 1 >>
endobj
3 0 obj
<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] >>
endobj
4 0 obj
<< /Title (Test Document) /Author (John Doe) /Creator (oxidizePdf Test Suite) >>
endobj
xref
0 5
0000000000 65535 f 
0000000015 00000 n 
0000000068 00000 n 
0000000125 00000 n 
0000000203 00000 n 
trailer
<< /Size 5 /Root 1 0 R /Info 4 0 R >>
startxref
306
%%EOF"""

def create_pdf_with_text():
    """Create a PDF with simple text."""
    return b"""%PDF-1.4
1 0 obj
<< /Type /Catalog /Pages 2 0 R >>
endobj
2 0 obj
<< /Type /Pages /Kids [3 0 R] /Count 1 >>
endobj
3 0 obj
<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Contents 4 0 R /Resources << /Font << /F1 << /Type /Font /Subtype /Type1 /BaseFont /Helvetica >> >> >> >>
endobj
4 0 obj
<< /Length 44 >>
stream
BT
/F1 12 Tf
100 700 Td
(Hello, World!) Tj
ET
endstream
endobj
xref
0 5
0000000000 65535 f 
0000000009 00000 n 
0000000062 00000 n 
0000000119 00000 n 
0000000297 00000 n 
trailer
<< /Size 5 /Root 1 0 R >>
startxref
389
%%EOF"""

def create_corrupted_pdf():
    """Create a corrupted PDF (truncated)."""
    return b"""%PDF-1.4
1 0 obj
<< /Type /Catalog /Pages 2 0 R >>
endobj
2 0 obj
<< /Type /Pages /Kids [3 0 R] /Count 1 >>
endobj
xref
0 3
0000000000 655"""

def create_invalid_header_pdf():
    """Create a PDF with invalid header."""
    return b"""%PDF-9.9
1 0 obj
<< /Type /Catalog /Pages 2 0 R >>
endobj
%%EOF"""

def main():
    # Create directories
    base_dir = "test-suite/fixtures"
    dirs = [
        f"{base_dir}/valid/minimal",
        f"{base_dir}/valid/standard",
        f"{base_dir}/invalid/corrupted",
        f"{base_dir}/invalid/malformed",
    ]
    
    for dir_path in dirs:
        os.makedirs(dir_path, exist_ok=True)
    
    # Generate valid PDFs
    test_pdfs = [
        ("valid/minimal/minimal_empty.pdf", create_minimal_pdf(), {
            "name": "minimal_empty",
            "description": "Absolute minimal valid PDF with one empty page",
            "pdf_version": "1.4",
            "features": [],
            "compliance": ["Pdf17"]
        }),
        ("valid/minimal/minimal_with_info.pdf", create_pdf_with_info(), {
            "name": "minimal_with_info",
            "description": "Minimal PDF with info dictionary",
            "pdf_version": "1.4",
            "features": [],
            "compliance": ["Pdf17"]
        }),
        ("valid/minimal/minimal_text.pdf", create_pdf_with_text(), {
            "name": "minimal_text",
            "description": "Minimal PDF with text content",
            "pdf_version": "1.4",
            "features": ["Text"],
            "compliance": ["Pdf17"]
        }),
    ]
    
    # Generate invalid PDFs
    invalid_pdfs = [
        ("invalid/corrupted/truncated.pdf", create_corrupted_pdf(), {
            "name": "truncated",
            "description": "PDF file truncated in the middle",
            "pdf_version": "1.4",
            "features": [],
            "compliance": []
        }),
        ("invalid/malformed/invalid_header.pdf", create_invalid_header_pdf(), {
            "name": "invalid_header",
            "description": "PDF with invalid version in header",
            "pdf_version": "9.9",
            "features": [],
            "compliance": []
        }),
    ]
    
    # Write PDFs and metadata
    for path, content, metadata in test_pdfs + invalid_pdfs:
        pdf_path = f"{base_dir}/{path}"
        json_path = pdf_path.replace('.pdf', '.json')
        
        # Write PDF
        with open(pdf_path, 'wb') as f:
            f.write(content)
        
        # Write metadata
        json_data = {
            "metadata": metadata,
            "expected_behavior": {
                "ParseSuccess": {"page_count": 1}
            } if "valid" in path else {
                "ParseError": {
                    "error_type": "InvalidHeader" if "header" in path else "UnexpectedEof",
                    "error_pattern": "invalid.*header" if "header" in path else "unexpected end"
                }
            }
        }
        
        with open(json_path, 'w') as f:
            json.dump(json_data, f, indent=2)
        
        print(f"Created: {pdf_path}")

if __name__ == "__main__":
    main()
    print("\nTest PDFs generated successfully!")