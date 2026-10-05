# Independent text contract fixtures (#666)

Six TSVs enumerate exactly 256 byte positions each. Columns are hexadecimal byte,
glyph name and Unicode scalar sequence. `-` means an undefined position in that
reference, not permission to reinterpret the byte as Latin-1. Defined characters
are not necessarily supported by oxidize-pdf. Unicode expectations preserve NBSP
and soft hyphen as U+00A0/U+00AD (explicit semantic alias policy).

The data is derived from Apache PDFBox 3.0.5 encoding tables at commit
`804cc824f1a19bcce85a3d0d60a8f10e98188e52`, with Adobe glyphlist.txt and
zapfdingbats.txt at `4036a9ca80a62f64f9de4f7321a9a045ad0ecfd6`. Sources and output
hashes are in provenance.json. No expected output is generated from oxidize-pdf
code. Tables are read offline; no font installation or network is required.

The Standard/MacRoman/WinAnsi named assignments were cross-checked against the
229 rows of PDF 32000-1:2008 Annex D.2, printed pages 653–656. Normative notes
account for MacRoman NBSP, WinAnsi NBSP/soft hyphen and WinAnsi bullet in otherwise
unused positions above octal 040. MacRoman 0xDB is currency, not Euro. Adobe's
published website URLs returned 404 on 2026-10-01; the official GitHub copy was
downloaded and hashed instead. The complete normative document is not vendored.

Symbol (189 assignments) and ZapfDingbats (188 assignments) have also been
reconciled completely against Annex D.5/D.6 and are exercised without an Encoding
entry, using their builtin font encoding. These tables pass; #668 was integrated before the current #666 WIP.
MacExpert is now normatively reconciled (165 names) and has a synthetic Type1C
fixture with actual charstrings. Both explicit ToUnicode and fallback are tested for all assigned codes. Fallback
retains the legacy AGL PUA mappings without semantic normalization; ToUnicode
may supply different semantic text. All six simple encodings recover undefined
positions as U+FFFD and preserve neighboring characters. This recovery is a
product policy, not a normative Unicode assignment.

## Current validation

The 2026-10-04 QR found gaps despite 208 passing contracts. Its counterexamples
are now permanent regressions; current commands, results and scope are recorded in
`docs/reports/2026-10-04-issue-666-qr-fixes.md`. Historical counts below describe
their dated baselines and do not describe the current candidate. The 46-row
matrix remains partial; green tests do not claim complete PDF font support.

## Reproduction

Download the source URLs from provenance.json into one directory under the
`text-contracts-*` filenames in its source hash inventory. Produce
`text-contracts-iso32000.txt` with `pdftotext -layout` from the recorded PDF.
Verify input hashes before regeneration, then run:

```sh
python3 tools/generate_text_encoding_oracles.py SOURCE_DIR OUTPUT_DIR
```

Compare regenerated files with these fixtures. Deliberate reference updates need
review of the changed assignments, provenance and tests; never regenerate at test
runtime or replace expected output with the current decoder's output.

`coverage.json` is the agreed 46-row scope manifest, not a green support matrix.
`executable-partial` means only the documented first slice of that row runs.
Use the plan and validation report to see actual tested contracts and failures.

Historical evidence before the subsequent fixes (2026-10-03): 16 Rust targets, 149 tests, 131 pass / 18 fail /
none ignored on product matching develop d33e7f94. The rows now specify oracle
sources, fixture strategy, closure criterion and remaining work. Validate their
traceability with `python3 tools/text_contracts/validate_coverage.py`; this checks
inventory consistency with the plan, not product conformance.

`type1/` adds original Type1/PFB and Type1C full/subset programs and 16 PDFs.
Generate offline with FontTools 4.60.1 using
`tools/generate_text_type1_contracts.py OUTPUT`. The repository MIT license
applies to the original triangle/rectangle outlines. PFB record headers are
removed from embedded FontFile payloads and Length1/2/3 are preserved.
The intrinsic encoding deliberately maps byte 65 to /B and 66 to /A.
At the original 2026-10-03 baseline, eight new Rust contracts passed and two revealed ignored intrinsic encoding and
missing inheritance under Differences without BaseEncoding. Both external
readers agree on all expected text; MuPDF geometry and qpdf structure also pass.
Run the existing reader validator with `--manifest` pointing to type1/readers.json
and `--root` pointing to this type1 directory. These expectations are immutable
reference values, not outputs learned from oxidize-pdf. See the 2026-10-03 report.

## Licensing and attribution

Encoding assignments derived from Apache PDFBox, Copyright The Apache Software
Foundation, licensed under Apache License 2.0; see LICENSE-Apache-2.0.txt.
Unicode name mappings derived from Adobe Glyph List / ZapfDingbats mappings,
Copyright Adobe; redistribution terms and source notices in LICENSE-Adobe.txt.
The TSVs remain independent test references. Product tables are a fixed Rust
transcription, with redistribution notices in src/text/pdf_encoding_licenses/.
The product does not load these fixtures at runtime.

Second increment: Type3 fixtures contain actual CharProcs and declared metrics;
FontMatrix 0.001/0.002, ToUnicode precedence and bfrange scalar/array destinations
are exercised. This does not claim embedded Type1/CFF/TrueType coverage.


Third increment adds PDFDocEncoding.tsv, extracted directly from normative Table
D.2 in Annex D.3. Its columns are byte/status/scalar, distinct from glyph-name
tables. All 256 positions are inventoried: 232 defined and 24 marked U/undefined
by the standard. Unicode values printed beside U rows are not treated as defined.
Two public routes exercise the 232 defined positions: Info Title/Author and
ActualText. UTF-16BE tests cover valid BOM, empty payload, surrogate pair,
combining sequence, interior U+FEFF and multiple characters. Malformed sequences
and undefined-byte recovery remain pending policy, not silently assumed passing.

fonts/ contains the complete official SourceSans3-Regular 3.052 OpenType/CFF font,
its SIL OFL 1.1 license and source/hash/metrics provenance. No subset or outline
modification is made. The PDF fixture embeds FontFile3 /OpenType in PDF 1.7.
This exercises full CFF/OpenType, not raw Type1C/PFB, TrueType or subset behavior.


Fourth increment adds the official SourceSans3-Regular 3.052 TrueType font and
ContractSansSubset-Regular.ttf, renamed under OFL after subsetting. Generate with
FontTools 4.60.1 using tools/generate_text_font_subset.py (offline); the input hash
is enforced. The subset retains space/A/B/eacute and composite dependencies,
seven glyphs versus 2478. License and truetype-provenance.json are included.
Tests use only these vendored artifacts, not Python or FontTools at runtime.

Document string routes now include outline titles and AcroForm stored V values.
The latter follows Catalog/AcroForm/Fields through public reader APIs; it does
not test form filling, appearance generation or viewer interaction.


Fifth increment: ContractExpert.cff is an original generated font with 165 expert
names plus .notdef. Outlines are rectangles, not expert typography. FontTools
4.60.1 generator and provenance are retained; it reproduces byte-for-byte.
The source MacExpert assignments are checked completely against Annex D.4.

CID tests embed the complete pinned TrueType font and map source codes to CIDs
17/18/19/23/29, then to GIDs 2/3/371/2; ToUnicode is keyed by source codes.
Identity-H, cidchar and an incrementing cidrange have matching raster controls.
Swapped GIDs change the raster while explicit Unicode stays unchanged. Poppler
warns about long-source ToUnicode entries and omits final text; those Unicode
cases are not claimed independently validated by Poppler.


Actualización posterior — discrepancia Poppler resuelta: los límites numéricos de
Poppler 24.02.0 explican avisos/pérdida de texto. MuPDF 1.26.10 confirma los
fixtures originales; nueve casos mínimos verifican los umbrales. Se conservan
expectativas y fixtures. Informe: docs/reports/2026-10-01-issue-666-poppler-discrepancy.md.


ContractCID.cff is an original CID-keyed CFF fixture, generated offline with
FontTools 4.60.1 by tools/generate_text_cid_cff.py. CID charset 0/17/29 maps to
GIDs 0/1/2; CIDs 17/29 have widths 400/700 and distinct triangle/rectangle outlines.
The generator reopens the binary and verifies ROS, FDSelect, charset and widths.
One FDArray entry is used. No glyphs are copied from external fonts. See
cid-cff-provenance.json for hash and scope; generation is byte-reproducible.
This tests CIDFontType0 with explicit Unicode, not a standard CJK collection.

### Incremento simbólico / UseCMap (2026-10-03)

`symbolic/`: ocho programas glyf originales completos/subconjuntos, cmap 0/4/12,
12 PDFs y provenance; generar con FontTools 4.60.1 mediante
`tools/generate_text_symbolic_contracts.py OUTPUT`. Los 24 textos externos coinciden.
`usecmap/`: once PDFs sobre ContractCID.cff, generar con
`tools/generate_text_usecmap_contracts.py OUTPUT`. Nueve casos tienen oráculo MuPDF
de texto/geometría; notdef-char y notdef-range son probes exploratorios pendientes.
El manifiesto externo UseCMap **no pasa**: cinco discrepancias Poppler y dos MuPDF;
no se han añadido excepciones. Detalle en
`docs/reports/2026-10-03-issue-666-symbolic-usecmap-review.md`.
Total actual: 167 contratos, 143 PASS/24 FAIL/0 omitidos; matriz aún parcial.


## CID collection API compatibility

`CidCollection` retains its four original variants, so existing exhaustive
matches compile unchanged. `from_ordering("KR")` now returns None instead of
incorrectly treating KR as Korea1. Use the new non-exhaustive
`AdobeCidCollection` for all five collections and include a wildcard arm when
matching it. Both APIs preserve the scalar method; sequence destinations are
available through `cid_to_unicode_sequence`. Internal PDF consumers use the
five-collection API.

Both Adobe generators verify `tools/adobe_cjk_source_pins.json` before writing
output. Updating an upstream reference requires a separate review of that pin
inventory and the changed mappings; a freshly computed output hash is not proof
of source revision.

`kr_extended/` adds 43 independently sampled controls for the twelve remaining
KR resources in the same pinned revision: `Adobe-KR-0` through `Adobe-KR-9`,
`UniAKR-UTF8-H` and `UniAKR-UTF32-H`. The original `cjk/` oracles stay unchanged.
Regenerate with `tools/generate_text_kr_extended.py CMAP_ARCHIVE UNICODE_ARCHIVE OUTPUT`;
it verifies the shared source pins before writing. Together with the original
UTF-16 resource these cover all thirteen KR CMaps present in that revision;
there is no vertical KR resource there. Tests check both Unicode authority and
code/CID/GID identity, including rejection of an unrelated Korea1 collection.
