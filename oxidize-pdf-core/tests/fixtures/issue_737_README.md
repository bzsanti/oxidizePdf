# Issue 737 fixture

Independent handwritten PDF 1.4, assembled from explicit indirect objects and computed xref offsets without the product writer. Catalog 1, Pages 2 with Kids [3,4] and Count 2; page 3/4 reference Helvetica object 5 and content streams 6/7. Streams contain `FIRST SECRET` and `SECOND SECRET` at (72,700). Both pages have MediaBox [0,0,612,792]. Validated with qpdf and Poppler; logs in docs/reports/2026-10-14-issue-737-evidence.

The contract uses 1-indexed pages: 1 and 2 exist; 0, 3 and u32::MAX do not. Visual masks preserve the original extractable strings; this fixture does not claim irreversible redaction.
