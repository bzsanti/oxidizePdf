# Design: Non-Identity CID Encoding CMap support for Type0 text extraction (#272 scope B)

**Date**: 2026-05-25
**Branch**: `fix/issue-272-tj-implicit-space`
**Issue**: #272 (CFF/ToUnicode CMap not decoded → glyph-code garbage)
**Status**: design approved, pending spec review

## Problem

`#272` reports that text extracted from certain PDFs is glyph-code garbage. Two
symptoms (TJ kerning run-ons, BOE Latin-1 garbage) were already fixed on this
branch. This design covers the remaining, larger root cause confirmed by
measurement: **non-Identity CID encoding is not decoded**.

The decode pipeline for a Type0/CID font is:

```
content-stream bytes → [Encoding CMap: code → CID] → [CID → Unicode] → text
```

Today the `code → CID` step is missing: the extractor assumes `code == CID`
(Identity). When `/Encoding` is anything other than `Identity-H`/`Identity-V`,
this assumption is wrong and the wrong CID is looked up → garbage.

### Measured prevalence (corpus t1-spec/pdfjs + t3-stress, 2702 PDFs)

`CMap::parse` (the existing parser) runs **only** on `/ToUnicode` streams
(`extraction_cmap.rs:219`). Per ISO 32000-1 §9.10.3, a `/ToUnicode` CMap uses
only `codespacerange`+`bfchar`+`bfrange`; `usecmap`/`cidchar`/`cidrange`/`notdef`
belong to CID *encoding* CMaps, which are never routed through `CMap::parse`.

Non-Identity encoding usage in the corpus:

| Encoding form | PDFs | Collection |
|---|---|---|
| `/Encoding /GBK-EUC-H` | ~133 | Adobe-GB1 |
| `/Encoding /UniGB-*` (UCS2/UTF16) | ~24 | Adobe-GB1 |
| `/Encoding /90ms-RKSJ-*`, `/90pv-RKSJ-H` | ~11 | Adobe-Japan1 |
| `/Encoding /UniJIS-*` | ~6 | Adobe-Japan1 |
| `/Encoding /UniKS-*`, `/KSCms-UHC-H` | ~2 | Adobe-Korea1 |
| `/Encoding /UniCNS-UTF16-H` | ~1 | Adobe-CNS1 |
| embedded stream CMap (`cidchar`/`cidrange`/`notdef`) | ~13 | various |
| `usecmap` external in `/ToUnicode` (`/Adobe-Korea1-UCS2 usecmap`) | 1 (issue5010) | Adobe-Korea1 |

Reproduced: `issue5010.pdf` page 0 extracts `"\nK - �\r\nK\r���\nI "` (4 U+FFFD,
0 hangul) under `ParseOptions::tolerant()` — the `usecmap` parent is not resolved,
so only the local `bf*` overrides apply and they don't cover the glyphs used.

## Existing infrastructure to reuse

- **`src/text/cid_to_unicode.rs`** (auto-generated from Adobe cmap-resources,
  BSD-3-Clause): static CID→Unicode tables for CNS1/GB1/Japan1/Korea1 via
  `CidCollection::from_ordering(ordering)` + `cid_to_unicode(cid)`. This is the
  `CID → Unicode` half of the pipeline — already present.
- **`src/text/cmap.rs`**: `tokenize_cmap` + `Token` (hardened against hangs on
  branch commit `929740b`). `Token::Integer` already produced — exactly what
  `cidchar`/`cidrange` destinations need (a CID integer, not a hex string).
- **`tools/generate_cid_tables.py`**: the generator that produced
  `cid_to_unicode.rs` from `cid2code.txt`.

The new work is entirely the `code → CID` half plus the `usecmap`-external
resolution in the ToUnicode path.

## Scope decision

Approach 1 (approved): unified PostScript `code→CID` parser + vendored Adobe
CMaps for **non-Unicode** encodings only + algorithmic shortcut for the `Uni*`
families.

- `Uni*-UCS2-*` / `Uni*-UTF16-*`: the content-stream code IS a UCS-2 / UTF-16
  value → decode the 2-byte code as UTF-16BE directly to Unicode. **No vendored
  data, no CID step.** Covers ~33 PDFs at zero bytes.
- `GBK-EUC-H`, `GBKp-EUC-H`, `90ms-RKSJ-H`, `90ms-RKSJ-V`, `90pv-RKSJ-H`,
  `KSCms-UHC-H`: vendored `code→CID` tables (generated), then CID→Unicode via the
  existing tables. Covers ~144 PDFs (GBK-EUC-H ~133 dominant).
- Embedded `/Encoding` stream CMaps: parsed with the same `code→CID` parser
  (`cidchar`/`cidrange`/`notdefchar`/`notdefrange`). Covers ~13 PDFs.

Rejected: (2) vendoring `Uni*` as full-BMP tables — huge for zero benefit over
the algorithmic shortcut; (3) adding `encoding_rs` — new dependency against
project philosophy, bypasses the CID architecture, and does not solve the
embedded-stream case which is core scope.

## Architecture

### Components

1. **`src/text/cmap.rs`** (modify)
   - Make `tokenize_cmap` and `Token` `pub(crate)`.
   - **Change A — external `usecmap` resolution in `/ToUnicode`**: `inherited_predefined`
     already records the parent name. When it matches `Adobe-{GB1,Japan1,Korea1,CNS1}-UCS2`,
     a 2-byte code that is in codespace but unmapped by explicit `bf*` falls back to
     `cid_to_unicode` for the corresponding ordering (the code is treated as the CID).
     Explicit child `bf*` mappings always win. A name→ordering map
     (`Adobe-Korea1-UCS2` → `"Korea1"`, etc.) lives next to this logic.

2. **`src/text/encoding_cmap.rs`** (new) — `EncodingCMap` (`code → CID`)
   - Fields: `codespace_ranges: Vec<CodeRange>` (variable byte-width),
     `single_cid: HashMap<Vec<u8>, u16>` (from `cidchar`),
     `cid_ranges: Vec<CidRange { lo: Vec<u8>, hi: Vec<u8>, base_cid: u16 }>` (from `cidrange`),
     `notdef_ranges: Vec<NotdefRange>` (from `notdefchar`/`notdefrange`),
     `ordering: Option<String>` (from `/CIDSystemInfo /Ordering`),
     `wmode: u8`.
   - `parse(&[u8]) -> ParseResult<Self>`: token state machine reusing `tokenize_cmap`,
     consuming `begincidchar`/`endcidchar`, `begincidrange`/`endcidrange`,
     `beginnotdefchar`/`endnotdefchar`, `beginnotdefrange`/`endnotdefrange`,
     `begincodespacerange`/`endcodespacerange`, `usecmap`, `/CIDSystemInfo`. Same
     progress invariant as `tokenize_cmap` (every iteration advances or terminates).
   - `code_len_at(&self, bytes: &[u8], pos: usize) -> usize`: determine the code
     byte-width at `pos` by matching codespace ranges (ISO 32000-1 §9.7.6.2 — a
     lead byte selects the matching range and thus the width). Default to 1 if no
     range matches (progress guarantee).
   - `map_code_to_cid(&self, code: &[u8]) -> Option<u16>`: `single_cid` first, then
     `cid_ranges` (offset within range, big-endian), then `notdef_ranges` (→ notdef CID).

3. **`src/text/predefined_cmap.rs`** (new) — resolves `/Encoding` by name
   - `enum PredefinedEncoding { Utf16(Utf16Variant), Cid(&'static EncodingCMap) }` (or
     equivalent). `Uni*-UCS2-*`/`Uni*-UTF16-*` → `Utf16` (algorithmic decoder).
     Vendored multibyte names → `Cid` backed by `encoding_cmap_data.rs`.
   - `resolve(name: &str) -> Option<PredefinedEncoding>`; unknown → `None` (graceful fallback).

4. **`src/text/encoding_cmap_data.rs`** (new, generated) — static `code→CID` data
   for the 6 vendored names, with a constructor that builds the `&'static EncodingCMap`
   (or the lookup arrays directly). Header documents provenance + BSD-3-Clause.

5. **`tools/generate_cid_tables.py`** (modify) — emit `encoding_cmap_data.rs` from
   Adobe cmap-resources (`*-H`/`*-V` CMap files), alongside the existing
   `cid_to_unicode.rs` generation.

### Data flow

`extract_font_info` (`extraction_cmap.rs`), on `/Encoding`:

- Name `Identity-H`/`Identity-V` → unchanged (`cid_encoding = None`).
- Name predefined CJK → `font_info.cid_encoding = predefined_cmap::resolve(name)`
  (`Some(Utf16Be)` for `Uni*`, `Some(Cmap(..))` for vendored multibyte, `None` if unknown).
- **Stream** (currently ignored — falls in `_ => {}`) → decode + `EncodingCMap::parse`
  → `cid_encoding = Some(Cmap(..))`.
- WinAnsi/MacRoman/Standard → unchanged.

`decode_text_with_font` (Type0 branch) precedence — ToUnicode still preferred so
current good behavior never regresses:

1. ToUnicode present (parent or descendant) → `decode_with_cmap` (now with Change A applied).
2. No ToUnicode + non-Identity encoding present:
   - algorithmic `Uni*` → decode code as UTF-16BE → Unicode.
   - `EncodingCMap` (embedded or vendored) → `code → CID` (variable width via
     `code_len_at`), then `cid_to_unicode` keyed by the descendant/encoding ordering.
3. Identity + `cid_to_unicode` (existing path) — unchanged.
4. Fallback to encoding-based decode — unchanged.

`FontInfo` gains a single field `cid_encoding: Option<CidEncoding>` where:

```rust
enum CidEncoding {
    Utf16Be,             // Uni*-UCS2-* / Uni*-UTF16-* (algorithmic, no data)
    Cmap(EncodingCMap),  // embedded stream CMap, or a vendored predefined CMap
}
```

`None` preserves today's behavior (Identity / non-Type0). This one enum removes the
need for a separate discriminator: the `Utf16Be` variant and the `Cmap` variant are
mutually exclusive resolved forms of a non-Identity encoding. `Identity-H`/`Identity-V`
map to `None` (handled by the existing Identity path).

## Error handling

- Malformed embedded CMap → best-effort partial map; **never panic, never hang**
  (hardened tokenizer + progress invariant). Adversarial-input test is mandatory
  (error-log lesson 2026-05-24: a full parser rewrite must be validated against
  malformed input — stray close delimiters, EOF mid-token, unexpected bytes — not
  only hand-written happy paths).
- Variable-width decode with no codespace match at a position → advance 1 byte
  (same defensive stance as `decode_with_cmap`), guaranteeing forward progress.
- Unmapped code / notdef → `U+FFFD` (aligned with existing `decode_with_cid_table`,
  which emits `U+FFFD` for `cid > 0`; preserves position).
- Unknown predefined name → fall back to current behavior (Identity assumption),
  not an error.

## Testing (TDD, content-verifying — no smoke tests)

Every test asserts real content, never `is_ok`/`is_empty`/byte-count/`%PDF`
(CLAUDE.md + `feedback_no_smoke_tests.md`). Bug-first: write the failing
reproduction before the fix.

- **Unit (`EncodingCMap::parse`)**: `cidchar` single, `cidrange` with offset,
  `notdefrange`; mixed 1+2-byte codespace (GBK pattern `<00><80>` + `<8140><FEFE>`);
  `code_len_at` width determination; adversarial inputs (stray close delimiters,
  unterminated range, EOF mid-token) must terminate and not hang.
- **Algorithmic `Uni*`**: UCS-2 BMP decode; UTF-16 with a surrogate pair.
- **`usecmap` external (Change A)**: `issue5010.pdf` added as fixture; extract with
  `ParseOptions::tolerant()` → assert hangul code points present and U+FFFD count = 0
  (today: `"\nK - �..."`, 4× U+FFFD, 0 hangul).
- **Integration corpus (GBK-EUC-H)**: a real GBK-EUC-H PDF (candidate `issue14438.pdf`)
  added as fixture → assert CJK Unified Ideographs extracted, no U+FFFD garbage run.
- **Regression**: `rag_realworld` 5/5 unchanged; `t1_spec::t1_pdfjs_corpus` and
  `t3_stress::t3_zero_panics_on_stress_corpus` run **individually** (not via summary
  grep) — 0 new panics/hangs/timeouts.
- **Bench**: `text_extraction_*` vs `v2.0.0-profiling` within the project's ±10%
  band; the `code→CID` step only runs for non-Identity Type0 fonts, so Latin PDFs
  are unaffected.

## Scope boundaries (YAGNI)

- Extraction only. No writer changes (CID writing already uses Identity-H).
- No CIDToGIDMap-based glyph rendering — not needed for text content.
- Vendored set limited to the measured high-prevalence names; others fall back
  gracefully rather than erroring. New names can be added by re-running the generator.
- `WMode`/vertical is parsed but vertical positioning is out of scope (affects glyph
  layout, not extracted text content).
- `usecmap` chaining to *arbitrary* external CMap resources in the document's
  resource dict remains out (the parser has no document handle); only resolution to
  the bundled predefined `*-UCS2` parents is implemented, which covers the measured case.

## Open risks

- **GBK 1/2-byte boundary**: correct codespace-driven width determination is the
  main correctness risk; covered by dedicated unit tests with the real GBK codespace.
- **Vendored data size**: 6 `code→CID` tables add to crate size. GBK-EUC-H is the
  largest. Mitigated by excluding the `Uni*` families (algorithmic) and CNS1-EUC
  (not measured). Measure the generated byte size during implementation; if it is
  disproportionate, reconsider lazy/feature-gated inclusion (decision deferred to
  implementation with real numbers, not guessed now).

## Known gaps after implementation (2026-05-26)

Implemented across 13 commits (`3b0fb95..69028cc`). Verified: lib 6441/0, t1/t3
0 panics/0 timeouts, rag_realworld 5/5 unchanged, bench within ±10%, issue5010
Korean + GBK-EUC-H Chinese extract real content. Two non-blocking gaps remain
(flagged by final review, do not block merge to develop):

- **Embedded-stream `/Encoding` Reference branch is integration-untested.** The
  `PdfObject::Reference → stream.decode → EncodingCMap::parse → CidEncoding::Cmap`
  wiring in `extract_font_info` is exercised only by a unit test that constructs the
  `EncodingCMap` directly; no end-to-end test drives a real embedded-stream encoding
  CMap through document loading (a corpus fixture needs both an embedded CMapType-1
  `/Encoding` stream AND a recognised Adobe ordering — GB1/Japan1/Korea1/CNS1 — which
  the available corpus lacks: e.g. `issue20232.pdf` uses ordering `Modern1`, unmapped).
  Recommended before tagging a release.
- **Vendored set is `-H` only.** Shipped: GBK-EUC-H, GBKp-EUC-H, 90ms-RKSJ-H,
  90pv-RKSJ-H, KSCms-UHC-H. The spec text above listed `90ms-RKSJ-V`; vertical (`-V`)
  variants are out of scope and fall through to graceful Identity fallback.
- The `EncodingCMap.usecmap_parent` field is parsed but not followed (external-base
  chaining for embedded streams remains out of scope, as stated above); the 5 vendored
  CMaps contain no `usecmap`, so this does not affect them.
