#\!/usr/bin/env python3
"""Create a simple PDF with custom font to compare"""

try:
    from reportlab.pdfgen import canvas
    from reportlab.pdfbase import pdfmetrics
    from reportlab.pdfbase.ttfonts import TTFont
    
    # Register Arial Unicode font
    pdfmetrics.registerFont(TTFont('ArialUnicode', '/System/Library/Fonts/Supplemental/Arial Unicode.ttf'))
    
    # Create PDF
    c = canvas.Canvas("test_reference.pdf")
    
    # Use Arial Unicode
    c.setFont("ArialUnicode", 24)
    c.drawString(50, 700, "ABCDEF")
    c.drawString(50, 650, "Hello World")
    
    c.save()
    print("Created test_reference.pdf with correct spacing")
    
    # Now analyze it
    with open("test_reference.pdf", 'rb') as f:
        content = f.read()
    
    # Check for Type0
    if b'/Type0' in content:
        print("✓ Uses Type0 font")
    
    # Check DW
    import re
    dw_match = re.search(rb'/DW\s+(\d+)', content)
    if dw_match:
        print(f"DW value: {dw_match.group(1).decode()}")
        
except ImportError:
    print("reportlab not available - can't create reference PDF")
