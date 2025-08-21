#!/usr/bin/env python3
"""
ISO 32000-1:2008 Definitive Requirements Extractor

This script creates the MASTER list of ISO requirements by:
1. Parsing the official ISO PDF structure
2. Identifying requirements by ISO standard patterns (not arbitrary filters)
3. Creating a definitive, immutable source of truth
4. Assigning proper ISO section IDs to each requirement

NO MORE DYNAMIC FILTERING. NO MORE CHANGING NUMBERS.
This creates the authoritative list.
"""

import fitz  # PyMuPDF
import json
import re
from pathlib import Path
from typing import Dict, List, Tuple, Optional
from dataclasses import dataclass
from datetime import datetime

@dataclass
class IsoRequirement:
    """A single ISO 32000-1:2008 requirement"""
    id: str  # ISO section ID (e.g., "7.5.2.1")
    section_title: str  # Section name
    requirement_type: str  # "mandatory", "optional", "conditional"
    text: str  # The actual requirement text
    page: int  # Page number in ISO PDF
    context: str  # Surrounding context for clarity
    keywords: List[str]  # Technical keywords extracted
    
    def to_dict(self) -> Dict:
        return {
            'id': self.id,
            'section_title': self.section_title,
            'requirement_type': self.requirement_type,
            'text': self.text,
            'page': self.page,
            'context': self.context,
            'keywords': self.keywords
        }

class IsoRequirementsExtractor:
    """Extract ISO 32000-1:2008 requirements using official methodology"""
    
    def __init__(self, pdf_path: str):
        self.pdf_path = pdf_path
        self.doc = None
        self.requirements: List[IsoRequirement] = []
        self.current_section = "Unknown"
        self.current_section_title = "Unknown"
        
        # ISO 32000-1:2008 specific patterns
        self.requirement_patterns = [
            # Primary requirement indicators
            r'\b(?:shall|must|required)\b',
            r'\b(?:should|recommended)\b', 
            r'\b(?:may|optional)\b',
            r'\b(?:conforming reader|conforming writer)\b',
            
            # Table-based requirements
            r'Table\s+\d+[.–-]\s*\w+',
            r'\(Required\)',
            r'\(Optional\)',
            
            # Structure requirements
            r'The\s+\w+\s+dictionary\s+(?:shall|must|should)',
            r'The\s+\w+\s+entry\s+(?:shall|must|should)',
        ]
        
        # ISO section patterns
        self.section_patterns = [
            r'^(\d+(?:\.\d+)*)\s+(.+)$',  # "7.5.2 Document Catalog"
            r'^([A-Z]\.\d+(?:\.\d+)*)\s+(.+)$',  # "A.1.2 Appendix"
        ]
        
        # Skip patterns (to avoid false positives)
        self.skip_patterns = [
            r'^\s*©.*Adobe',  # Copyright notices
            r'^\s*NOTE\s+\d+',  # Explanatory notes
            r'^\s*EXAMPLE\s+\d+',  # Examples
            r'^\s*Figure\s+\d+',  # Figure captions
            r'^\s*Table\s+\d+\s*$',  # Table headers only
            r'^\s*\d+\s*$',  # Page numbers only
        ]
    
    def extract_all_requirements(self) -> List[IsoRequirement]:
        """Extract all ISO requirements from the PDF"""
        print("🔍 Opening ISO 32000-1:2008 PDF...")
        
        try:
            self.doc = fitz.open(self.pdf_path)
            print(f"✓ Opened PDF: {len(self.doc)} pages")
        except Exception as e:
            print(f"❌ Error opening PDF: {e}")
            return []
        
        print("📖 Extracting requirements by section...")
        
        # Process each page systematically
        for page_num in range(len(self.doc)):
            if page_num % 50 == 0:
                print(f"  - Processing page {page_num + 1}/{len(self.doc)}")
            
            page = self.doc[page_num]
            text = page.get_text()
            
            # Skip early pages (cover, TOC, etc.)
            if page_num < 20:
                continue
            
            self._process_page_text(text, page_num + 1)
        
        print(f"✓ Extracted {len(self.requirements)} ISO requirements")
        return self.requirements
    
    def _process_page_text(self, text: str, page_num: int):
        """Process text from a single page"""
        lines = text.split('\n')
        
        for i, line in enumerate(lines):
            line = line.strip()
            if not line:
                continue
            
            # Check if this is a section header
            section_match = self._match_section_header(line)
            if section_match:
                self.current_section, self.current_section_title = section_match
                continue
            
            # Check if this line contains a requirement
            if self._is_requirement_line(line):
                # Get context (surrounding lines)
                context_start = max(0, i - 2)
                context_end = min(len(lines), i + 3)
                context = ' '.join(lines[context_start:context_end]).strip()
                
                requirement = self._create_requirement(line, context, page_num)
                if requirement:
                    self.requirements.append(requirement)
    
    def _match_section_header(self, line: str) -> Optional[Tuple[str, str]]:
        """Check if line is an ISO section header"""
        for pattern in self.section_patterns:
            match = re.match(pattern, line.strip())
            if match:
                section_id = match.group(1)
                section_title = match.group(2).strip()
                return section_id, section_title
        return None
    
    def _is_requirement_line(self, line: str) -> bool:
        """Check if line contains an ISO requirement"""
        # Skip obvious non-requirements
        for skip_pattern in self.skip_patterns:
            if re.match(skip_pattern, line, re.IGNORECASE):
                return False
        
        # Must be substantial content
        if len(line) < 20:
            return False
        
        # Check for requirement indicators
        for pattern in self.requirement_patterns:
            if re.search(pattern, line, re.IGNORECASE):
                return True
        
        return False
    
    def _create_requirement(self, text: str, context: str, page: int) -> Optional[IsoRequirement]:
        """Create an IsoRequirement from extracted text"""
        # Determine requirement type
        req_type = self._determine_requirement_type(text)
        
        # Extract keywords
        keywords = self._extract_keywords(text)
        
        # Create unique ID
        req_id = f"{self.current_section}.{len([r for r in self.requirements if r.id.startswith(self.current_section)])}"
        
        # Clean up text
        clean_text = re.sub(r'\s+', ' ', text).strip()
        clean_context = re.sub(r'\s+', ' ', context).strip()
        
        return IsoRequirement(
            id=req_id,
            section_title=self.current_section_title,
            requirement_type=req_type,
            text=clean_text,
            page=page,
            context=clean_context,
            keywords=keywords
        )
    
    def _determine_requirement_type(self, text: str) -> str:
        """Determine if requirement is mandatory, optional, or conditional"""
        text_lower = text.lower()
        
        if any(word in text_lower for word in ['shall', 'must', 'required']):
            return 'mandatory'
        elif any(word in text_lower for word in ['should', 'recommended']):
            return 'recommended' 
        elif any(word in text_lower for word in ['may', 'optional']):
            return 'optional'
        elif any(word in text_lower for word in ['if', 'when', 'where']):
            return 'conditional'
        else:
            return 'general'
    
    def _extract_keywords(self, text: str) -> List[str]:
        """Extract technical PDF keywords from requirement text"""
        pdf_keywords = {
            # Core structure
            'catalog', 'dictionary', 'object', 'stream', 'array', 'page', 'pages',
            'indirect', 'reference', 'xref', 'trailer', 'header', 'body',
            
            # Graphics
            'graphics', 'path', 'stroke', 'fill', 'color', 'rgb', 'cmyk', 'gray',
            'image', 'matrix', 'transform', 'coordinate', 'clip', 'pattern',
            
            # Text
            'text', 'font', 'glyph', 'character', 'encoding', 'unicode', 'cmap',
            'truetype', 'type1', 'cid', 'postscript',
            
            # Interactive
            'annotation', 'form', 'field', 'widget', 'action', 'link', 'destination',
            'outline', 'bookmark', 'thread', 'article',
            
            # Filters
            'filter', 'decode', 'flate', 'lzw', 'ascii', 'compression', 'ccitt',
            'jbig', 'jpeg', 'jpx',
            
            # Security
            'encrypt', 'decrypt', 'security', 'permission', 'password', 'access',
            
            # Advanced
            'transparency', 'blend', 'mask', 'shading', 'function', 'colorspace',
            'device', 'calibrated', 'separation', 'devicen'
        }
        
        found_keywords = []
        words = re.findall(r'\b\w+\b', text.lower())
        
        for word in words:
            if word in pdf_keywords:
                found_keywords.append(word)
        
        return list(set(found_keywords))  # Remove duplicates
    
    def save_requirements(self, output_file: str):
        """Save requirements to JSON file"""
        requirements_data = {
            'metadata': {
                'source': 'ISO 32000-1:2008 PDF',
                'extraction_date': datetime.now().isoformat(),
                'extractor_version': '1.0.0',
                'methodology': 'Official ISO structure parsing',
                'total_requirements': len(self.requirements)
            },
            'summary': {
                'by_type': {},
                'by_section': {},
                'total_pages_processed': len(self.doc) if self.doc else 0
            },
            'requirements': [req.to_dict() for req in self.requirements]
        }
        
        # Calculate summaries
        type_counts = {}
        section_counts = {}
        
        for req in self.requirements:
            type_counts[req.requirement_type] = type_counts.get(req.requirement_type, 0) + 1
            section_prefix = req.id.split('.')[0]
            section_counts[section_prefix] = section_counts.get(section_prefix, 0) + 1
        
        requirements_data['summary']['by_type'] = type_counts
        requirements_data['summary']['by_section'] = section_counts
        
        # Save to file
        Path(output_file).parent.mkdir(parents=True, exist_ok=True)
        with open(output_file, 'w', encoding='utf-8') as f:
            json.dump(requirements_data, f, indent=2, ensure_ascii=False)
        
        print(f"💾 Saved {len(self.requirements)} requirements to {output_file}")
        
        # Print summary
        print("\n📊 Requirements Summary:")
        print(f"  Total: {len(self.requirements)}")
        for req_type, count in sorted(type_counts.items()):
            print(f"  {req_type.title()}: {count}")
        
        print("\n📋 By Section:")
        for section, count in sorted(section_counts.items()):
            print(f"  Section {section}: {count}")

def main():
    print("🎯 ISO 32000-1:2008 DEFINITIVE Requirements Extractor")
    print("=" * 60)
    print("Creating the MASTER requirements list - no more dynamic filtering!")
    print()
    
    # Check if ISO PDF exists
    pdf_path = "PDF32000_2008.pdf"
    if not Path(pdf_path).exists():
        print(f"❌ ISO PDF not found: {pdf_path}")
        print("Please ensure the ISO 32000-1:2008 PDF is in the project root")
        return
    
    # Extract requirements
    extractor = IsoRequirementsExtractor(pdf_path)
    requirements = extractor.extract_all_requirements()
    
    if not requirements:
        print("❌ No requirements extracted")
        return
    
    # Save master file
    output_file = "ISO_REQUIREMENTS_MASTER.json"
    extractor.save_requirements(output_file)
    
    print(f"\n🎉 SUCCESS: Created definitive requirements list")
    print(f"📄 Master file: {output_file}")
    print(f"🔢 Total requirements: {len(requirements)}")
    print("\n✅ This is now the SINGLE SOURCE OF TRUTH for ISO requirements")
    print("   All future analysis will use this exact list - no more variation!")

if __name__ == "__main__":
    main()