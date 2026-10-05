#!/usr/bin/env python3
"""Compatibility entry point for the pinned, sequence-preserving CID generator.
Usage: python3 tools/generate_cid_tables.py <mapping-resources-pdf-root> <output.rs>
The legacy unpinned cid2code-column generator is intentionally retired: it lost
surrogate pairs and aliased Adobe-KR to Korea1. See issue676.
"""
import runpy
from pathlib import Path

if __name__ == '__main__':
    runpy.run_path(str(Path(__file__).with_name('generate_adobe_pdf_cid_tables.py')), run_name='__main__')
