# Document-string oracle for issue #667

`PDFDocEncoding.tsv` is the independent 256-position oracle used by
`text_document_string_contract_test.rs`: 232 assigned positions and 24
undefined positions. The table predates the decoder correction. Its hash and
pinned upstream sources are recorded in `provenance.json`; that historical
manifest also inventories font-encoding tables outside this change.

PDFDocEncoding follows ISO 32000-1 Annex D, Table D.2. Undefined positions
recover visibly as U+FFFD. UTF-16BE preserves valid units and exposes malformed
or incomplete units as U+FFFD. PDF 2.0 UTF-8 requires the EF BB BF BOM and the
effective header/catalog version; pre-2.0 and BOM-free input keep PDFDocEncoding
semantics. These recovery policies do not certify malformed input as valid.

The PDF 2.0 tests use literal independent Unicode sequences, including empty,
non-BMP, combining, internal BOM and malformed subsequences. Document text
strings remain separate from binary strings and font character codes in Tj/TJ.

The source license texts are retained alongside the oracle. Tests assemble PDF
syntax directly and use public reader/extractor APIs; expected values are not
computed with production encoding tables.
