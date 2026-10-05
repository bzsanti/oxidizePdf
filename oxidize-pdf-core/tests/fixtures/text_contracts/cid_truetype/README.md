# F10 — CIDFontType2

Two original TrueType programs (full/subset), 40 PDFs and a frozen manifest.
Generate with FontTools4.60.1:

```
PYTHONPATH=target/issue666-fonttools python3 tools/generate_text_cid_truetype_contracts.py OUTPUT
```

The matrix covers Identity/stream CIDToGIDMap, remapped source codes, H/V,
private and Adobe-Japan1 collections, and explicit/absent ToUnicode. Original
GIDs2/3 draw distinct triangle/rectangle outlines. PDF widths450/750 deliberately
differ from hmtx400/700; W2 advances are -1200/-900. Explicit Unicode is fi/😀,
separate from glyph shapes and Adobe collection A/B. Private unknown Unicode
has no scalar oracle: the project recovery contract emits U+FFFD per source code.

The subset retains the GIDs of A/B and removes C. Files are original repository
MIT fixtures; no host font or network dependency. `manifest.json` freezes42
SHA256 hashes. Independent FontTools/MuPDF/qpdf checks and mutations are in
`docs/reports/2026-10-05-f10-evidence/`. Rust tests assert source text and pen
positions; layout-inserted line separators are outside this font-selection row.
