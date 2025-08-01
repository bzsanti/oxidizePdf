#!/usr/bin/env python3
"""
Advanced Commercial PDF Compatibility Testing

This script provides comprehensive testing for PDF compatibility with
commercial readers using multiple Python PDF libraries and external tools.
"""

import sys
import os
import subprocess
import json
import tempfile
from pathlib import Path
from typing import Dict, List, Optional, Tuple
import argparse

# PDF library imports (with fallbacks)
try:
    import PyPDF2
    HAS_PYPDF2 = True
except ImportError:
    HAS_PYPDF2 = False
    print("⚠️  PyPDF2 not available. Install with: pip install PyPDF2")

try:
    import fitz  # PyMuPDF
    HAS_PYMUPDF = True
except ImportError:
    HAS_PYMUPDF = False
    print("⚠️  PyMuPDF not available. Install with: pip install PyMuPDF")

try:
    from reportlab.pdfgen import canvas
    from reportlab.lib.pagesizes import letter
    from reportlab.pdfbase import pdfforms
    HAS_REPORTLAB = True
except ImportError:
    HAS_REPORTLAB = False
    print("⚠️  ReportLab not available. Install with: pip install reportlab")

class CompatibilityTestResult:
    def __init__(self, pdf_path: str):
        self.pdf_path = pdf_path
        self.pypdf2_result = None
        self.pymupdf_result = None
        self.structure_result = None
        self.forms_result = None
        self.annotations_result = None
        self.visual_result = None
        self.errors = []
        
    def success_rate(self) -> float:
        """Calculate overall success rate"""
        tests = [
            self.pypdf2_result,
            self.pymupdf_result, 
            self.structure_result,
            self.forms_result,
            self.annotations_result,
            self.visual_result
        ]
        
        completed_tests = [t for t in tests if t is not None]
        if not completed_tests:
            return 0.0
            
        passed_tests = sum(1 for t in completed_tests if t)
        return (passed_tests / len(completed_tests)) * 100.0
    
    def is_commercial_ready(self) -> bool:
        """Determine if PDF is ready for commercial use"""
        # Critical requirements for commercial compatibility
        critical_passed = (
            (self.pypdf2_result is None or self.pypdf2_result) and
            (self.structure_result is None or self.structure_result) and
            (self.forms_result is None or self.forms_result)
        )
        
        return critical_passed and self.success_rate() >= 75.0

class CommercialCompatibilityTester:
    def __init__(self, verbose: bool = False):
        self.verbose = verbose
        self.temp_dir = tempfile.mkdtemp(prefix="pdf_compat_test_")
        
    def log(self, message: str):
        """Log message if verbose mode is enabled"""
        if self.verbose:
            print(f"[DEBUG] {message}")
    
    def test_pypdf2_compatibility(self, pdf_path: str) -> Tuple[bool, List[str]]:
        """Test PDF compatibility with PyPDF2"""
        if not HAS_PYPDF2:
            return True, ["PyPDF2 not available - skipped"]
        
        errors = []
        try:
            self.log(f"Testing PyPDF2 compatibility for {pdf_path}")
            
            with open(pdf_path, 'rb') as file:
                reader = PyPDF2.PdfReader(file)
                
                # Basic structure tests
                page_count = len(reader.pages)
                if page_count == 0:
                    errors.append("No pages found")
                    return False, errors
                
                self.log(f"Found {page_count} pages")
                
                # Test metadata
                if reader.metadata:
                    self.log("Metadata found")
                    if reader.metadata.title:
                        self.log(f"Title: {reader.metadata.title}")
                else:
                    self.log("No metadata found")
                
                # Test first page access
                try:
                    page = reader.pages[0]
                    # Try to extract text (basic functionality test)
                    text = page.extract_text()
                    self.log(f"Extracted {len(text)} characters from first page")
                except Exception as e:
                    errors.append(f"Failed to access first page: {e}")
                
                # Check for forms (AcroForm)
                if hasattr(reader, 'trailer') and reader.trailer:
                    root = reader.trailer.get('/Root', {})
                    if '/AcroForm' in root:
                        self.log("AcroForm dictionary found")
                        try:
                            acroform = root['/AcroForm']
                            if '/Fields' in acroform:
                                fields = acroform['/Fields']
                                self.log(f"Found {len(fields)} form fields")
                        except Exception as e:
                            errors.append(f"Error reading AcroForm: {e}")
                
                # Check annotations on first page
                try:
                    page = reader.pages[0]
                    if '/Annots' in page:
                        annots = page['/Annots']
                        if annots:
                            self.log(f"Found {len(annots)} annotations on first page")
                            
                            # Check annotation types
                            for i, annot_ref in enumerate(annots[:3]):  # Check first 3
                                try:
                                    annot = annot_ref.get_object()
                                    subtype = annot.get('/Subtype', '')
                                    self.log(f"Annotation {i}: {subtype}")
                                except Exception as e:
                                    errors.append(f"Error reading annotation {i}: {e}")
                except Exception as e:
                    errors.append(f"Error checking annotations: {e}")
                
                return True, errors
                
        except Exception as e:
            errors.append(f"PyPDF2 failed to read PDF: {e}")
            return False, errors
    
    def test_pymupdf_compatibility(self, pdf_path: str) -> Tuple[bool, List[str]]:
        """Test PDF compatibility with PyMuPDF"""
        if not HAS_PYMUPDF:
            return True, ["PyMuPDF not available - skipped"]
        
        errors = []
        try:
            self.log(f"Testing PyMuPDF compatibility for {pdf_path}")
            
            doc = fitz.open(pdf_path)
            
            # Basic structure tests
            page_count = doc.page_count
            if page_count == 0:
                errors.append("No pages found")
                return False, errors
            
            self.log(f"Found {page_count} pages")
            
            # Test metadata
            metadata = doc.metadata
            if metadata and metadata.get('title'):
                self.log(f"Title: {metadata['title']}")
            
            # Test first page
            try:
                page = doc[0]
                
                # Test text extraction
                text = page.get_text()
                self.log(f"Extracted {len(text)} characters from first page")
                
                # Test widgets (form fields)
                widgets = page.widgets()
                if widgets:
                    self.log(f"Found {len(widgets)} widgets on first page")
                    for i, widget in enumerate(widgets[:3]):  # Check first 3
                        self.log(f"Widget {i}: type={widget.field_type}, name={widget.field_name}")
                
                # Test annotations
                annots = page.annots()
                if annots:
                    self.log(f"Found {len(annots)} annotations on first page")
                    for i, annot in enumerate(annots[:3]):  # Check first 3
                        self.log(f"Annotation {i}: type={annot.type[1]}")
                
                # Test rendering capability
                try:
                    pix = page.get_pixmap(matrix=fitz.Matrix(0.5, 0.5))  # 50% scale
                    self.log(f"Rendered page to {pix.width}x{pix.height} image")
                except Exception as e:
                    errors.append(f"Failed to render page: {e}")
                
            except Exception as e:
                errors.append(f"Failed to access first page: {e}")
            
            doc.close()
            return True, errors
            
        except Exception as e:
            errors.append(f"PyMuPDF failed to read PDF: {e}")
            return False, errors
    
    def test_pdf_structure(self, pdf_path: str) -> Tuple[bool, List[str]]:
        """Test basic PDF structure requirements"""
        errors = []
        
        try:
            with open(pdf_path, 'rb') as f:
                # Check PDF header
                header = f.read(8)
                if not header.startswith(b'%PDF-'):
                    errors.append("Invalid PDF header")
                    return False, errors
                
                version = header.decode('ascii', errors='ignore')
                self.log(f"PDF version: {version}")
                
                # Check file size
                f.seek(0, 2)  # Seek to end
                size = f.tell()
                self.log(f"PDF file size: {size} bytes")
                
                if size < 100:
                    errors.append("PDF file too small")
                    return False, errors
                
                # Check for EOF marker (basic structure validation)
                f.seek(max(0, size - 100))  # Check last 100 bytes
                tail = f.read()
                if b'%%EOF' not in tail:
                    errors.append("Missing EOF marker")
                    # This is a warning, not necessarily a failure
                
                return True, errors
                
        except Exception as e:
            errors.append(f"Failed to read PDF structure: {e}")
            return False, errors
    
    def test_forms_compatibility(self, pdf_path: str) -> Tuple[bool, List[str]]:
        """Test form field compatibility (the critical commercial compatibility issue)"""
        errors = []
        
        # Test with PyPDF2 if available
        if HAS_PYPDF2:
            try:
                with open(pdf_path, 'rb') as file:
                    reader = PyPDF2.PdfReader(file)
                    
                    # Check for AcroForm
                    has_acroform = False
                    if hasattr(reader, 'trailer') and reader.trailer:
                        root = reader.trailer.get('/Root', {})
                        if '/AcroForm' in root:
                            has_acroform = True
                            self.log("AcroForm found")
                            
                            # Deep check of form structure
                            try:
                                acroform = root['/AcroForm']
                                if '/Fields' in acroform:
                                    fields = acroform['/Fields']
                                    self.log(f"AcroForm has {len(fields)} fields")
                                    
                                    # Check first few fields for critical properties
                                    for i, field_ref in enumerate(fields[:3]):
                                        try:
                                            field = field_ref.get_object()
                                            
                                            # Check for widget annotation properties (critical for commercial compatibility)
                                            has_critical_props = True
                                            critical_checks = []
                                            
                                            # Check for Type = Annot (critical!)
                                            if '/Type' in field and field['/Type'] == '/Annot':
                                                critical_checks.append("Type=Annot ✅")
                                            else:
                                                critical_checks.append("Type=Annot ❌")
                                                has_critical_props = False
                                            
                                            # Check for Subtype = Widget (critical!)
                                            if '/Subtype' in field and field['/Subtype'] == '/Widget':
                                                critical_checks.append("Subtype=Widget ✅")
                                            else:
                                                critical_checks.append("Subtype=Widget ❌")
                                                has_critical_props = False
                                            
                                            # Check for Page reference (critical!)
                                            if '/P' in field:
                                                critical_checks.append("Page ref ✅")
                                            else:
                                                critical_checks.append("Page ref ❌")
                                                has_critical_props = False
                                            
                                            # Check for visibility flags
                                            if '/F' in field:
                                                critical_checks.append("Flags ✅")
                                            else:
                                                critical_checks.append("Flags ❌")
                                            
                                            # Check for default appearance (important for text fields)
                                            if '/DA' in field:
                                                critical_checks.append("Default appearance ✅")
                                            
                                            self.log(f"Field {i} critical properties: {', '.join(critical_checks)}")
                                            
                                            if not has_critical_props:
                                                errors.append(f"Field {i} missing critical widget properties for commercial compatibility")
                                            
                                        except Exception as e:
                                            errors.append(f"Error checking field {i}: {e}")
                                            
                            except Exception as e:
                                errors.append(f"Error analyzing AcroForm structure: {e}")
                    
                    # If no forms found, that's OK
                    if not has_acroform:
                        self.log("No AcroForm found (PDF has no forms)")
                        return True, ["No forms to test"]
                    
                    return len(errors) == 0, errors
                    
            except Exception as e:
                errors.append(f"Failed to check forms with PyPDF2: {e}")
        
        # Test with PyMuPDF if available
        if HAS_PYMUPDF:
            try:
                doc = fitz.open(pdf_path)
                
                total_widgets = 0
                for page_num in range(doc.page_count):
                    page = doc[page_num]
                    widgets = page.widgets()
                    total_widgets += len(widgets)
                    
                    for widget in widgets:
                        # Basic widget validation
                        if not widget.field_name:
                            errors.append(f"Widget on page {page_num} has no field name")
                        
                        if widget.field_type == 0:  # Unknown type
                            errors.append(f"Widget '{widget.field_name}' has unknown field type")
                
                self.log(f"Total widgets found: {total_widgets}")
                doc.close()
                
            except Exception as e:
                errors.append(f"Failed to check forms with PyMuPDF: {e}")
        
        return len(errors) == 0, errors
    
    def test_annotations_compatibility(self, pdf_path: str) -> Tuple[bool, List[str]]:
        """Test annotation compatibility"""
        errors = []
        
        # Test with PyMuPDF if available (better annotation support)
        if HAS_PYMUPDF:
            try:
                doc = fitz.open(pdf_path)
                
                total_annots = 0
                for page_num in range(doc.page_count):
                    page = doc[page_num]
                    annots = page.annots()
                    total_annots += len(annots)
                    
                    for annot in annots:
                        # Check annotation properties
                        annot_type = annot.type[1]  # Get type name
                        self.log(f"Annotation type: {annot_type}")
                        
                        # Check for content (important for visibility)
                        content = annot.info.get('content', '')
                        if not content and annot_type in ['Text', 'FreeText']:
                            errors.append(f"Text annotation on page {page_num} has no content")
                
                self.log(f"Total annotations found: {total_annots}")
                doc.close()
                
            except Exception as e:
                errors.append(f"Failed to check annotations with PyMuPDF: {e}")
        
        return len(errors) == 0, errors
    
    def create_reference_pdf(self, output_path: str) -> bool:
        """Create a reference PDF with ReportLab (known to work with commercial readers)"""
        if not HAS_REPORTLAB:
            print("❌ ReportLab not available for reference PDF creation")
            return False
        
        try:
            c = canvas.Canvas(output_path, pagesize=letter)
            
            # Add title
            c.setFont("Helvetica-Bold", 16)
            c.drawString(50, 750, "Reference PDF - Commercial Compatibility Test")
            
            # Add description
            c.setFont("Helvetica", 12)
            c.drawString(50, 720, "This PDF was created with ReportLab and should work in all commercial readers.")
            
            # Add form fields
            c.setFont("Helvetica-Bold", 12)
            c.drawString(50, 650, "Form Fields:")
            
            # Text field
            c.setFont("Helvetica", 10)
            c.drawString(50, 620, "Name:")
            c.acroForm.textfield(name='name', 
                                tooltip='Enter your name',
                                x=100, y=615, borderStyle='inset',
                                width=200, height=20,
                                textColor=canvas.colors.black,
                                fillColor=canvas.colors.white)
            
            # Checkbox
            c.drawString(50, 590, "Subscribe:")
            c.acroForm.checkbox(name='subscribe',
                               tooltip='Check to subscribe',
                               x=150, y=585,
                               size=15,
                               checked=False)
            
            c.showPage()
            c.save()
            
            print(f"✅ Reference PDF created: {output_path}")
            return True
            
        except Exception as e:
            print(f"❌ Failed to create reference PDF: {e}")
            return False
    
    def test_pdf(self, pdf_path: str) -> CompatibilityTestResult:
        """Run comprehensive compatibility test on a PDF"""
        print(f"\n🔍 Testing PDF: {pdf_path}")
        print("=" * 60)
        
        result = CompatibilityTestResult(pdf_path)
        
        # Test 1: PDF Structure
        print("📋 Testing PDF structure...")
        structure_ok, structure_errors = self.test_pdf_structure(pdf_path)
        result.structure_result = structure_ok
        result.errors.extend([f"Structure: {e}" for e in structure_errors])
        print(f"   Structure: {'✅' if structure_ok else '❌'}")
        
        # Test 2: PyPDF2 Compatibility
        print("🐍 Testing PyPDF2 compatibility...")
        pypdf2_ok, pypdf2_errors = self.test_pypdf2_compatibility(pdf_path)
        result.pypdf2_result = pypdf2_ok
        result.errors.extend([f"PyPDF2: {e}" for e in pypdf2_errors])
        print(f"   PyPDF2: {'✅' if pypdf2_ok else '❌'}")
        
        # Test 3: PyMuPDF Compatibility  
        print("📄 Testing PyMuPDF compatibility...")
        pymupdf_ok, pymupdf_errors = self.test_pymupdf_compatibility(pdf_path)
        result.pymupdf_result = pymupdf_ok
        result.errors.extend([f"PyMuPDF: {e}" for e in pymupdf_errors])
        print(f"   PyMuPDF: {'✅' if pymupdf_ok else '❌'}")
        
        # Test 4: Forms Compatibility (CRITICAL)
        print("📝 Testing forms compatibility...")
        forms_ok, forms_errors = self.test_forms_compatibility(pdf_path)
        result.forms_result = forms_ok
        result.errors.extend([f"Forms: {e}" for e in forms_errors])
        print(f"   Forms: {'✅' if forms_ok else '❌'}")
        
        # Test 5: Annotations Compatibility
        print("📌 Testing annotations compatibility...")
        annots_ok, annots_errors = self.test_annotations_compatibility(pdf_path)
        result.annotations_result = annots_ok
        result.errors.extend([f"Annotations: {e}" for e in annots_errors])
        print(f"   Annotations: {'✅' if annots_ok else '❌'}")
        
        # Summary
        success_rate = result.success_rate()
        commercial_ready = result.is_commercial_ready()
        
        print(f"\n🎯 Results:")
        print(f"   Success Rate: {success_rate:.1f}%")
        print(f"   Commercial Ready: {'✅' if commercial_ready else '❌'}")
        
        if result.errors:
            print(f"\n⚠️  Issues found:")
            for error in result.errors[:10]:  # Show first 10 errors
                print(f"   • {error}")
            if len(result.errors) > 10:
                print(f"   ... and {len(result.errors) - 10} more issues")
        
        return result

def main():
    parser = argparse.ArgumentParser(description='Test PDF compatibility with commercial readers')
    parser.add_argument('pdf_files', nargs='+', help='PDF files to test')
    parser.add_argument('--verbose', '-v', action='store_true', help='Verbose output')
    parser.add_argument('--create-reference', '-r', help='Create reference PDF with ReportLab')
    parser.add_argument('--output-json', '-o', help='Output results to JSON file')
    
    args = parser.parse_args()
    
    tester = CommercialCompatibilityTester(verbose=args.verbose)
    
    # Create reference PDF if requested
    if args.create_reference:
        if tester.create_reference_pdf(args.create_reference):
            print(f"Reference PDF created: {args.create_reference}")
        else:
            print("Failed to create reference PDF")
            return 1
    
    # Test PDFs
    results = []
    total_success_rate = 0.0
    commercial_ready_count = 0
    
    for pdf_path in args.pdf_files:
        if not os.path.exists(pdf_path):
            print(f"❌ File not found: {pdf_path}")
            continue
            
        result = tester.test_pdf(pdf_path)
        results.append(result)
        
        total_success_rate += result.success_rate()
        if result.is_commercial_ready():
            commercial_ready_count += 1
    
    # Overall summary
    if results:
        avg_success_rate = total_success_rate / len(results)
        commercial_ready_rate = (commercial_ready_count / len(results)) * 100.0
        
        print(f"\n📊 OVERALL SUMMARY")
        print("=" * 40)
        print(f"Files tested: {len(results)}")
        print(f"Average success rate: {avg_success_rate:.1f}%")
        print(f"Commercial ready: {commercial_ready_count}/{len(results)} ({commercial_ready_rate:.1f}%)")
        
        if commercial_ready_rate >= 90.0:
            print("\n🎉 EXCELLENT! Your PDFs are highly compatible with commercial readers!")
        elif commercial_ready_rate >= 70.0:
            print("\n✅ GOOD! Most PDFs are compatible, minor improvements possible.")
        else:
            print("\n⚠️  NEEDS IMPROVEMENT! Significant compatibility issues detected.")
            print("\nRecommendations:")
            print("• Ensure form fields have Type=Annot and Subtype=Widget")
            print("• Add page references (P) to all form fields")
            print("• Include visibility flags (F) and default appearance (DA)")
            print("• Test with oxidize-pdf's commercial compatibility fixes")
        
        # Output JSON if requested
        if args.output_json:
            json_data = {
                'summary': {
                    'files_tested': len(results),
                    'average_success_rate': avg_success_rate,
                    'commercial_ready_count': commercial_ready_count,
                    'commercial_ready_rate': commercial_ready_rate
                },
                'results': []
            }
            
            for result in results:
                json_data['results'].append({
                    'pdf_path': result.pdf_path,
                    'success_rate': result.success_rate(),
                    'commercial_ready': result.is_commercial_ready(),
                    'pypdf2_result': result.pypdf2_result,
                    'pymupdf_result': result.pymupdf_result,
                    'structure_result': result.structure_result,
                    'forms_result': result.forms_result,
                    'annotations_result': result.annotations_result,
                    'errors': result.errors
                })
            
            with open(args.output_json, 'w') as f:
                json.dump(json_data, f, indent=2)
            print(f"\n📄 Results saved to: {args.output_json}")
    
    return 0

if __name__ == '__main__':
    sys.exit(main())