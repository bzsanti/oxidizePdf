# Development Tools

This directory contains development and debugging scripts for the oxidizePdf project.

## Scripts

### test_parsing.py
**Purpose**: Validates generated PDFs using external tools (qpdf)
**Usage**: `python test_parsing.py`
**Description**: Generates a test PDF using the hello_world example and validates it with qpdf to ensure compatibility with external PDF processors.

### debug_xref.py
**Purpose**: Generates minimal PDF files for debugging xref table issues
**Usage**: `python debug_xref.py`
**Description**: Creates a debug PDF with manually calculated offsets to test xref table generation and parsing. Outputs `debug_manual.pdf`.

### test_pdf_samples.sh
**Purpose**: Comprehensive test suite for different PDF complexity levels
**Usage**: `./test_pdf_samples.sh`
**Description**: Runs the external PDF test suite against simple, medium, and complex PDF samples to validate parser robustness.

## Security Note

These scripts are development tools and should not be included in production builds. They are excluded from the repository via `.gitignore` to prevent accidental commits of debug code.

## Requirements

- Python 3.x (for Python scripts)
- qpdf (for PDF validation)
- Rust/Cargo (for building examples)

## Development Workflow

1. Make changes to the PDF library
2. Run validation scripts to ensure compatibility
3. Test against various PDF samples
4. Debug issues using the debug scripts