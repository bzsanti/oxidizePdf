#!/usr/bin/env python3
"""Analyze PDF form structure in detail"""

import sys
import re

def analyze_forms(filename):
    print(f"Analyzing form structure in {filename}...\n")
    
    with open(filename, 'rb') as f:
        content = f.read()
    
    # Find AcroForm
    acroform_match = re.search(rb'/AcroForm\s+(\d+)\s+0\s+R', content)
    if acroform_match:
        acroform_ref = acroform_match.group(1).decode()
        print(f"✓ Found AcroForm reference: {acroform_ref} 0 R")
        
        # Find AcroForm object
        acroform_obj_pattern = rb'%s 0 obj\s*<<([^>]+)>>' % acroform_ref.encode()
        acroform_obj = re.search(acroform_obj_pattern, content, re.DOTALL)
        if acroform_obj:
            acroform_content = acroform_obj.group(1).decode('latin-1', errors='ignore')
            print(f"  AcroForm content: {acroform_content[:200]}...")
            
            # Find Fields array
            fields_match = re.search(r'/Fields\s*\[([\d\s+R]+)\]', acroform_content)
            if fields_match:
                fields = fields_match.group(1).strip()
                print(f"  Fields array: [{fields}]")
                
                # Parse field references
                field_refs = re.findall(r'(\d+)\s+\d+\s+R', fields)
                print(f"  Found {len(field_refs)} field(s): {field_refs}")
    else:
        print("✗ No AcroForm found")
    
    # Find all Widget annotations
    print("\n✓ Looking for Widget annotations...")
    widget_pattern = rb'<<[^>]*?/Subtype\s*/Widget[^>]*?>>'
    widgets = re.findall(widget_pattern, content, re.DOTALL)
    print(f"  Found {len(widgets)} widget annotation(s)")
    
    for i, widget in enumerate(widgets[:3]):  # Show first 3
        widget_str = widget.decode('latin-1', errors='ignore')
        print(f"\n  Widget {i+1}:")
        
        # Check for appearance stream
        if b'/AP' in widget:
            print("    ✓ Has appearance stream (/AP)")
        else:
            print("    ✗ No appearance stream (/AP)")
            
        # Check for Parent reference
        parent_match = re.search(rb'/Parent\s+(\d+)\s+0\s+R', widget)
        if parent_match:
            print(f"    ✓ Has Parent reference: {parent_match.group(1).decode()} 0 R")
        else:
            print("    ✗ No Parent reference")
            
        # Check for field type
        ft_match = re.search(rb'/FT\s*/(\w+)', widget)
        if ft_match:
            print(f"    Field type: {ft_match.group(1).decode()}")
            
        # Check for field name
        t_match = re.search(rb'/T\s*\(([^)]+)\)', widget)
        if t_match:
            print(f"    Field name: {t_match.group(1).decode()}")
            
        # Check for rectangle
        rect_match = re.search(rb'/Rect\s*\[([\d\s.]+)\]', widget)
        if rect_match:
            print(f"    Rectangle: [{rect_match.group(1).decode()}]")
    
    # Check form field objects
    print("\n✓ Checking form field objects...")
    field_pattern = rb'<<[^>]*?/FT\s*/(?:Tx|Btn|Ch)[^>]*?>>'
    fields = re.findall(field_pattern, content, re.DOTALL)
    print(f"  Found {len(fields)} field object(s)")
    
    for i, field in enumerate(fields[:3]):
        field_str = field.decode('latin-1', errors='ignore')
        print(f"\n  Field {i+1}:")
        
        # Check for Kids array (widgets)
        kids_match = re.search(rb'/Kids\s*\[([\d\s+R]+)\]', field)
        if kids_match:
            print(f"    ✓ Has Kids array: [{kids_match.group(1).decode()}]")
        else:
            print("    ✗ No Kids array")

if __name__ == "__main__":
    if len(sys.argv) < 2:
        print("Usage: python analyze_form_structure.py <pdf_file>")
        sys.exit(1)
    
    analyze_forms(sys.argv[1])