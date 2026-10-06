# Simple-font encoding contracts for issue #668

Six independent 256-position tables cover the five encodings corrected here
and the existing PDF MacRoman decoder. MacRoman is a regression control: the
implementation contributed in #664 remains unchanged.

The source revisions, licenses and checksums are pinned in `provenance.json`.
Across the six tables there are 1123 assigned and 413 undefined positions;
the five corrected tables account for 915 assigned and 365 undefined positions.
Undefined positions recover as U+FFFD without losing neighboring text. Legacy
AGL PUA values, NBSP/soft-hyphen duplicates and Unicode sequences are retained.

`ContractExpert.cff` is an original, redistributable name-keyed CFF fixture with
165 MacExpert glyph names and genuine Type2 charstrings. Its provenance records
the distinction between code-to-text correctness and visual expert typography.
`tools/generate_text_expert_font.py` regenerates it with FontTools 4.60.1.

The tests build PDFs independently, exercise public reader/extractor APIs, check
strict/lenient modes, declared widths and following glyph origins, and verify
Differences/ToUnicode precedence. Table inventory alone is not evidence that the
rest of the font matrix in #666 is implemented.

Recovery scope: an unresolved Differences name in a known simple encoding yields
U+FFFD. Custom Type3 fonts and fonts with unknown intrinsic encodings retain the
previous best-effort base-byte recovery; that compatibility behavior is not a
normative Unicode mapping. The corpus exposed text loss when the new replacement
policy was applied to those fonts indiscriminately. Two public-API regressions
cover custom names and known-name/ToUnicode precedence.
