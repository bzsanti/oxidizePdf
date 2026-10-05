## Problem

Independent public-API contract batteries developed under #666 reproduce document-string decoding defects on develop 48d8b8f8bbf08b2976fd739e6c2462c9552a3cc6 and both #664/#665:

- 38 of 232 defined PDFDocEncoding positions decode incorrectly in Info metadata, ActualText, outline titles and stored AcroForm values. The existing decoder uses WinAnsi semantics.
- PDF 2.0 text strings beginning EF BB BF do not decode UTF-8 in Info metadata and ActualText. Six independently authored vectors cover empty, ASCII, accented, supplementary, combining and internal BOM sequences. MuPDF 1.26.10 confirms all six metadata expectations.

These are document text strings, distinct from Tj/TJ font character codes. MacRoman #662 and tracking #663 do not cover this defect.

## Acceptance

- Preserve the independently fixed #666 PDFDocEncoding oracle (256 positions, explicit undefined inventory) and version-scoped UTF-8 vectors.
- Decode all defined PDFDocEncoding positions correctly through the four public routes.
- Support PDF 2.0 UTF-8 BOM text strings, retaining exact sequences without normalization; preserve UTF-16BE and empty-string behavior.
- Keep UTF-8 without BOM in PDFDocEncoding; do not apply document-string BOM semantics to Tj/TJ. Declare and test pre-2.0 interpretation explicitly.
- Define malformed UTF-16/UTF-8 and undefined-byte behavior before adding recovery, distinguishing normative valid inputs from recovery policy. No silent loss of valid suffix content.
- Meaningful RED/GREEN tests, quality/security review and proportional regression/CI validation; no historical metric recalibration.

References: ISO 32000-1 7.9.2.2 / Annex D.2; ISO 32000-2 7.9.2.2; https://pdfa.org/understanding-utf-8-in-pdf-2-0/.

Local evidence: docs/reports/2026-10-01-issue-666-step3.md, step4.md and step10.md; permanent tests text_document_string_contract_test.rs and text_pdf20_string_contract_test.rs. This issue tracks implementation, not documentation-only completion.
