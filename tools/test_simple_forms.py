#!/usr/bin/env python3
"""Create a minimal working form with PyPDF2 for comparison"""

from reportlab.pdfgen import canvas
from reportlab.lib.pagesizes import letter
from reportlab.pdfbase import pdfform
from reportlab.lib.colors import black, blue, red

def create_simple_form():
    c = canvas.Canvas("test_simple_form.pdf", pagesize=letter)
    
    # Add title
    c.setFont("Helvetica", 18)
    c.drawString(50, 720, "Test Form with ReportLab")
    
    # Add form fields
    c.setFont("Helvetica", 12)
    c.drawString(50, 650, "Name:")
    c.drawString(50, 600, "Email:")
    
    # Create actual form fields
    c.acroForm.textfield(name='name_field',
                        tooltip='Enter your name',
                        x=150, y=640, borderStyle='inset',
                        borderColor=blue, fillColor=None,
                        width=200, height=20,
                        textColor=black, forceBorder=True)
    
    c.acroForm.textfield(name='email_field',
                        tooltip='Enter your email',
                        x=150, y=590, borderStyle='inset',
                        borderColor=blue, fillColor=None,
                        width=200, height=20,
                        textColor=black, forceBorder=True)
    
    c.acroForm.checkbox(name='subscribe_checkbox',
                       tooltip='Subscribe to newsletter',
                       x=150, y=545, borderStyle='inset',
                       borderColor=black, fillColor=None,
                       size=15,
                       checked=False, forceBorder=True)
    
    c.drawString(175, 550, "Subscribe to newsletter")
    
    c.save()
    print("Created test_simple_form.pdf with ReportLab")

if __name__ == "__main__":
    try:
        create_simple_form()
    except ImportError:
        print("ReportLab not installed. Install with: pip install reportlab")