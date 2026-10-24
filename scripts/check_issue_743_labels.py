#!/usr/bin/env python3
"""Check an actual emitted PDF with pypdf, independently of Rust formatting.

Generate with:
  OXIDIZE_ISSUE743_PDF=/tmp/labels.pdf cargo test -p oxidize-pdf \
    --test issue_743_repeated_page_labels
Then run (pypdf is a verification-only dependency):
  uv run --with pypdf==6.21.2 --no-project python \
    scripts/check_issue_743_labels.py /tmp/labels.pdf
"""
import sys

import pypdf

reader = pypdf.PdfReader(sys.argv[1], strict=True)
expected = ["App-Z", "App-AA", "App-BB", "ann-zz", "ann-aaa"]
assert reader.page_labels == expected, (reader.page_labels, expected)
print(f"pypdf {pypdf.__version__}: {reader.page_labels}")
