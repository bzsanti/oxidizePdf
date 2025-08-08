#!/usr/bin/env python3
"""Quick verification that PDFs contain visible text operations"""

import sys

def check_pdf_has_text_operations(pdf_path):
    """Check if PDF contains text drawing operations"""
    try:
        with open(pdf_path, 'rb') as f:
            content = f.read()
            
        # Convert to string for searching (ignore encoding errors)
        pdf_str = content.decode('latin-1', errors='ignore')
        
        # Look for PDF text operations
        has_bt = 'BT' in pdf_str  # Begin Text
        has_et = 'ET' in pdf_str  # End Text
        has_tf = 'Tf' in pdf_str  # Set Font
        has_tj = 'Tj' in pdf_str or 'TJ' in pdf_str  # Show Text
        
        # Look for actual text content (between parentheses)
        has_text = '(' in pdf_str and ')' in pdf_str
        
        return {
            'has_text_begin': has_bt,
            'has_text_end': has_et,
            'has_font_set': has_tf,
            'has_text_show': has_tj,
            'has_text_content': has_text,
            'all_present': all([has_bt, has_et, has_tf, has_tj, has_text])
        }
    except Exception as e:
        return {'error': str(e)}

# Check both PDFs
pdfs = [
    'unicode_comprehensive.pdf',
    'unicode_exhaustive.pdf'
]

for pdf in pdfs:
    print(f"\n📄 Checking {pdf}:")
    result = check_pdf_has_text_operations(pdf)
    
    if 'error' in result:
        print(f"  ❌ Error: {result['error']}")
    else:
        print(f"  BT (Begin Text): {'✅' if result['has_text_begin'] else '❌'}")
        print(f"  ET (End Text): {'✅' if result['has_text_end'] else '❌'}")
        print(f"  Tf (Set Font): {'✅' if result['has_font_set'] else '❌'}")
        print(f"  Tj/TJ (Show Text): {'✅' if result['has_text_show'] else '❌'}")
        print(f"  Text content: {'✅' if result['has_text_content'] else '❌'}")
        print(f"  Overall: {'✅ Has text operations' if result['all_present'] else '❌ Missing text operations'}")

print("\n" + "="*50)
print("If all checks are ✅, the PDFs should display text.")
print("Open them in a PDF viewer to verify visually.")
print("="*50)