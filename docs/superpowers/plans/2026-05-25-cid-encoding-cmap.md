# Non-Identity CID Encoding CMap Extraction — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Decode text from Type0/CID fonts whose `/Encoding` is not Identity (embedded stream CMaps, predefined CJK names, and `usecmap`-to-predefined in `/ToUnicode`), eliminating glyph-code garbage on the affected corpora.

**Architecture:** Add the missing `code → CID` step to the extraction pipeline. A new `EncodingCMap` (parsed by the existing hardened `tokenize_cmap`) maps content-stream codes to CIDs; the existing `cid_to_unicode` tables map CID → Unicode. `Uni*` predefined encodings are decoded algorithmically as UTF-16BE (no data). Non-Unicode predefined CMaps (GBK-EUC-H, RKSJ, KSCms-UHC) are vendored as Adobe CMap resource files and parsed lazily. External `usecmap` in `/ToUnicode` resolves to the matching `cid_to_unicode` table.

**Tech Stack:** Rust (MSRV 1.77, no new deps), `cargo test`, Adobe cmap-resources (BSD-3-Clause) for vendored data.

**Spec:** `docs/superpowers/specs/2026-05-25-cid-encoding-cmap-design.md`

**Refinement vs spec (components 4–5):** the spec proposed generating `encoding_cmap_data.rs` via `generate_cid_tables.py`. This plan instead vendors the Adobe CMap source files verbatim and parses them lazily with the same `EncodingCMap` parser we build and test (DRY — no second representation, no codegen). Same data, same license, simpler. Flagged for the user.

---

## File Structure

| File | Responsibility | Action |
|---|---|---|
| `oxidize-pdf-core/src/text/cmap.rs` | `tokenize_cmap`/`Token` visibility; `inherited_ordering()` for usecmap→ordering | Modify |
| `oxidize-pdf-core/src/text/extraction_cmap.rs` | `FontInfo.cid_encoding`; `/Encoding` extraction; decode precedence; usecmap fallback in `decode_with_cmap` | Modify |
| `oxidize-pdf-core/src/text/encoding_cmap.rs` | `EncodingCMap` (code→CID parse, `code_len_at`, `map_code_to_cid`), `CidEncoding` enum, UTF-16BE decoder, predefined-name resolution | Create |
| `oxidize-pdf-core/src/text/cmap_resources/*` | Vendored Adobe CMap files + LICENSE/attribution | Create |
| `oxidize-pdf-core/src/text/mod.rs` | Register `encoding_cmap` module | Modify |
| `oxidize-pdf-core/src/text/extraction.rs` | `FontInfo` construction sites get `cid_encoding: None` | Modify |
| `oxidize-pdf-core/tests/encoding_cmap_*.rs` | Integration tests with real fixtures | Create |
| `oxidize-pdf-core/tests/fixtures/issue_272_*.pdf` | issue5010 (Korean usecmap), GBK-EUC-H sample | Create |

All tests assert real content. No `is_ok`/`is_empty`/byte-count/`%PDF` smoke assertions (CLAUDE.md). Bug-first: failing reproduction before the fix.

Run all `cargo` commands with `source ~/.cargo/env &&` prefixed and `nice` for heavy builds.

---

## Task 1: External `usecmap` resolution in `/ToUnicode` (fixes issue5010)

**Files:**
- Modify: `oxidize-pdf-core/src/text/cmap.rs` (add `inherited_ordering`)
- Modify: `oxidize-pdf-core/src/text/extraction_cmap.rs:603-630` (`decode_with_cmap`)
- Create: `oxidize-pdf-core/tests/fixtures/issue_272_issue5010_korean_usecmap.pdf`
- Test: `oxidize-pdf-core/tests/encoding_usecmap_external_test.rs`

- [ ] **Step 1: Add the fixture**

```bash
cp test-corpus/t1-spec/pdfjs/issue5010.pdf \
   oxidize-pdf-core/tests/fixtures/issue_272_issue5010_korean_usecmap.pdf
git add oxidize-pdf-core/tests/fixtures/issue_272_issue5010_korean_usecmap.pdf
```

- [ ] **Step 2: Write the failing integration test**

`oxidize-pdf-core/tests/encoding_usecmap_external_test.rs`:

```rust
use oxidize_pdf::parser::{ParseOptions, PdfDocument, PdfReader};
use oxidize_pdf::text::TextExtractor;
use std::fs::File;

/// issue5010: a `/ToUnicode` CMap that does `/Adobe-Korea1-UCS2 usecmap`
/// plus a few explicit bf* overrides. Before the fix, unmapped codes fall
/// through to nothing and the page extracts replacement-char garbage.
#[test]
fn issue5010_usecmap_korea1_resolves_real_hangul() {
    let path = "tests/fixtures/issue_272_issue5010_korean_usecmap.pdf";
    let doc = PdfDocument::new(
        PdfReader::new_with_options(File::open(path).unwrap(), ParseOptions::tolerant()).unwrap(),
    );
    let mut ext = TextExtractor::default();
    let text = ext.extract_from_page(&doc, 0).unwrap().text;

    let hangul = text.chars().filter(|&c| ('\u{AC00}'..='\u{D7A3}').contains(&c)).count();
    let replacement = text.chars().filter(|&c| c == '\u{FFFD}').count();

    assert!(hangul > 0, "expected real hangul, got: {text:?}");
    assert_eq!(replacement, 0, "no replacement chars expected, got: {text:?}");
}
```

- [ ] **Step 3: Run it to confirm it fails**

Run: `source ~/.cargo/env && cargo test -p oxidize-pdf --test encoding_usecmap_external_test 2>&1 | tail -15`
Expected: FAIL (`hangul > 0` fails; current output is `"\nK - \u{FFFD}..."`).

- [ ] **Step 4: Add `inherited_ordering` to `CMap`** in `cmap.rs` (inside `impl CMap`, near `inherited_predefined_is`):

```rust
    /// If this CMap inherits (via `usecmap`) from a predefined Adobe
    /// `*-UCS2` CMap, return the matching CID collection ordering.
    /// Used by ToUnicode decoding to resolve codes the child CMap did
    /// not map explicitly (the code is treated as a CID into the table).
    pub(crate) fn inherited_ordering(&self) -> Option<&'static str> {
        match self.inherited_predefined.as_deref()? {
            "Adobe-GB1-UCS2" => Some("GB1"),
            "Adobe-CNS1-UCS2" => Some("CNS1"),
            "Adobe-Japan1-UCS2" => Some("Japan1"),
            "Adobe-Korea1-UCS2" | "Adobe-KR-UCS2" => Some("Korea1"),
            _ => None,
        }
    }
```

- [ ] **Step 5: Add the fallback in `decode_with_cmap`** (`extraction_cmap.rs`). Replace the function body:

```rust
/// Decode text using a CMap — free function (no allocations).
fn decode_with_cmap(text_bytes: &[u8], cmap: &CMap) -> ParseResult<String> {
    use crate::text::cid_to_unicode::CidCollection;
    let inherited = cmap.inherited_ordering().and_then(CidCollection::from_ordering);

    let mut result = String::new();
    let mut i = 0;

    while i < text_bytes.len() {
        let mut decoded = false;

        for len in 1..=4.min(text_bytes.len() - i) {
            let code = &text_bytes[i..i + len];
            if let Some(mapped) = cmap.map(code) {
                if let Some(unicode_str) = cmap.to_unicode(&mapped) {
                    result.push_str(&unicode_str);
                    i += len;
                    decoded = true;
                    break;
                }
            }
        }

        if !decoded {
            // External usecmap to a predefined Adobe `*-UCS2` parent: treat an
            // unmapped 2-byte code as a CID and resolve via the inherited
            // collection. Explicit child bf* mappings already won above.
            if let Some(coll) = inherited {
                if text_bytes.len() - i >= 2 {
                    let cid = u16::from_be_bytes([text_bytes[i], text_bytes[i + 1]]);
                    if let Some(ch) = coll.cid_to_unicode(cid) {
                        result.push(ch);
                        i += 2;
                        continue;
                    }
                }
            }
            i += 1;
        }
    }

    Ok(result)
}
```

- [ ] **Step 6: Add a unit test for the mapping** in `cmap.rs` `tests` module:

```rust
    #[test]
    fn usecmap_external_ucs2_parent_maps_to_ordering() {
        let data = b"begincmap\n/Adobe-Korea1-UCS2 usecmap\n\
1 begincodespacerange <0000> <FFFF> endcodespacerange\n\
endcmap";
        let cmap = CMap::parse(data).expect("parse");
        assert_eq!(cmap.inherited_ordering(), Some("Korea1"));
    }
```

- [ ] **Step 7: Run both tests**

Run: `source ~/.cargo/env && cargo test -p oxidize-pdf --test encoding_usecmap_external_test && cargo test -p oxidize-pdf --lib usecmap_external_ucs2 2>&1 | tail -15`
Expected: PASS (hangul present, 0 replacement chars; ordering == Korea1).

- [ ] **Step 8: Commit**

```bash
git add oxidize-pdf-core/src/text/cmap.rs oxidize-pdf-core/src/text/extraction_cmap.rs \
        oxidize-pdf-core/tests/encoding_usecmap_external_test.rs
git commit -m "fix(text-cmap): resolve external usecmap to predefined UCS2 in ToUnicode (addresses #272)

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>"
```

---

## Task 2: `EncodingCMap` skeleton — codespace + variable-width `code_len_at`

**Files:**
- Modify: `oxidize-pdf-core/src/text/cmap.rs` (visibility of `tokenize_cmap`, `Token`)
- Modify: `oxidize-pdf-core/src/text/mod.rs`
- Create: `oxidize-pdf-core/src/text/encoding_cmap.rs`

- [ ] **Step 1: Make the tokenizer reusable.** In `cmap.rs`, change `enum Token {` → `pub(crate) enum Token {` and `fn tokenize_cmap(` → `pub(crate) fn tokenize_cmap(`.

- [ ] **Step 2: Register the module.** In `src/text/mod.rs`, add after `pub mod cmap;`:

```rust
pub(crate) mod encoding_cmap;
```

- [ ] **Step 3: Write the failing test** in `encoding_cmap.rs`:

```rust
//! Non-Identity CID encoding CMap (`code → CID`) for Type0 text extraction.
//! See docs/superpowers/specs/2026-05-25-cid-encoding-cmap-design.md.

use crate::parser::ParseResult;
use crate::text::cmap::{tokenize_cmap, Token};
use crate::text::cmap::CodeRange;

/// A CID encoding CMap: maps character codes (1–2 bytes, variable width per
/// the codespace) to CIDs. Distinct from `CMap` (ToUnicode), whose
/// destinations are Unicode hex strings.
#[derive(Debug, Clone, Default)]
pub(crate) struct EncodingCMap {
    pub codespace_ranges: Vec<CodeRange>,
    pub single_cid: std::collections::HashMap<Vec<u8>, u16>,
    pub cid_ranges: Vec<CidRange>,
    pub notdef_ranges: Vec<CidRange>,
    pub ordering: Option<String>,
    pub usecmap_parent: Option<String>,
    pub wmode: u8,
}

#[derive(Debug, Clone)]
pub(crate) struct CidRange {
    pub lo: Vec<u8>,
    pub hi: Vec<u8>,
    pub base_cid: u16,
}

impl EncodingCMap {
    /// Determine the byte width of the code starting at `pos` by matching the
    /// first byte against codespace ranges (ISO 32000-1 §9.7.6.2). Falls back
    /// to width 1 when no range matches, guaranteeing forward progress.
    pub fn code_len_at(&self, bytes: &[u8], pos: usize) -> usize {
        let b = bytes[pos];
        for r in &self.codespace_ranges {
            if !r.start.is_empty()
                && r.start.len() == r.end.len()
                && b >= r.start[0]
                && b <= r.end[0]
            {
                return r.start.len();
            }
        }
        1
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gbk_codespace_yields_mixed_widths() {
        // GBK-EUC-H codespace: single-byte <00>..<80>, double-byte <8140>..<FEFE>.
        let cmap = EncodingCMap {
            codespace_ranges: vec![
                CodeRange { start: vec![0x00], end: vec![0x80] },
                CodeRange { start: vec![0x81, 0x40], end: vec![0xFE, 0xFE] },
            ],
            ..Default::default()
        };
        assert_eq!(cmap.code_len_at(&[0x41], 0), 1, "ASCII byte is single");
        assert_eq!(cmap.code_len_at(&[0x81, 0x40], 0), 2, "lead byte 0x81 is double");
        assert_eq!(cmap.code_len_at(&[0xFE, 0xFE], 0), 2);
    }
}
```

- [ ] **Step 4: Add codespace parsing** so the struct can be built from CMap text. Add to `impl EncodingCMap` (above the `code_len_at` method):

```rust
    /// Parse the codespace ranges only. Full mapping parse is added in Task 3.
    pub fn parse(data: &[u8]) -> ParseResult<Self> {
        let content = String::from_utf8_lossy(data);
        let tokens = tokenize_cmap(&content);
        let mut cmap = EncodingCMap::default();
        let mut i = 0;
        while i < tokens.len() {
            match &tokens[i] {
                Token::Keyword(k) if k == "begincodespacerange" => {
                    i += 1;
                    while i < tokens.len() {
                        match &tokens[i] {
                            Token::Keyword(k) if k == "endcodespacerange" => { i += 1; break; }
                            Token::Hex(lo) => {
                                if let Some(Token::Hex(hi)) = tokens.get(i + 1) {
                                    cmap.codespace_ranges.push(CodeRange {
                                        start: lo.clone(),
                                        end: hi.clone(),
                                    });
                                    i += 2;
                                } else { i += 1; }
                            }
                            _ => i += 1,
                        }
                    }
                }
                Token::Keyword(k) if k == "usecmap" => {
                    let mut j = i;
                    while j > 0 {
                        j -= 1;
                        if let Token::Name(p) = &tokens[j] {
                            cmap.usecmap_parent = Some(p.clone());
                            break;
                        }
                    }
                    i += 1;
                }
                _ => i += 1,
            }
        }
        Ok(cmap)
    }
```

Add a parse test:

```rust
    #[test]
    fn parse_reads_codespace_and_usecmap_parent() {
        let data = b"begincmap\n/Foo-Base usecmap\n\
2 begincodespacerange <00> <80> <8140> <FEFE> endcodespacerange\n\
endcmap";
        let cmap = EncodingCMap::parse(data).expect("parse");
        assert_eq!(cmap.codespace_ranges.len(), 2);
        assert_eq!(cmap.code_len_at(&[0x81, 0x40], 0), 2);
        assert_eq!(cmap.usecmap_parent.as_deref(), Some("Foo-Base"));
    }
```

- [ ] **Step 5: Run tests**

Run: `source ~/.cargo/env && cargo test -p oxidize-pdf --lib encoding_cmap 2>&1 | tail -15`
Expected: PASS (3 tests).

- [ ] **Step 6: Commit**

```bash
git add oxidize-pdf-core/src/text/cmap.rs oxidize-pdf-core/src/text/mod.rs \
        oxidize-pdf-core/src/text/encoding_cmap.rs
git commit -m "feat(text-cmap): EncodingCMap skeleton with variable-width codespace (addresses #272)

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>"
```

---

## Task 3: `cidchar`/`cidrange` parsing + `map_code_to_cid`

**Files:**
- Modify: `oxidize-pdf-core/src/text/encoding_cmap.rs`

- [ ] **Step 1: Write the failing test** in `encoding_cmap.rs` `tests`:

```rust
    #[test]
    fn cidchar_and_cidrange_map_to_cids() {
        let data = b"begincmap\n\
1 begincodespacerange <0000> <FFFF> endcodespacerange\n\
1 begincidchar <0041> 100 endcidchar\n\
1 begincidrange <0061> <0063> 200 endcidrange\n\
endcmap";
        let cmap = EncodingCMap::parse(data).expect("parse");
        assert_eq!(cmap.map_code_to_cid(&[0x00, 0x41]), Some(100), "cidchar exact");
        assert_eq!(cmap.map_code_to_cid(&[0x00, 0x61]), Some(200), "cidrange base");
        assert_eq!(cmap.map_code_to_cid(&[0x00, 0x62]), Some(201), "cidrange +1");
        assert_eq!(cmap.map_code_to_cid(&[0x00, 0x63]), Some(202), "cidrange end");
        assert_eq!(cmap.map_code_to_cid(&[0x00, 0x64]), None, "outside range");
    }
```

- [ ] **Step 2: Run to confirm failure**

Run: `source ~/.cargo/env && cargo test -p oxidize-pdf --lib cidchar_and_cidrange 2>&1 | tail -15`
Expected: FAIL (`map_code_to_cid` not found).

- [ ] **Step 3: Extend `parse`** to consume cidchar/cidrange. Add these match arms inside the `while i < tokens.len()` loop in `EncodingCMap::parse`, before the `_ => i += 1` arm:

```rust
                Token::Keyword(k) if k == "begincidchar" => {
                    i += 1;
                    while i < tokens.len() {
                        match &tokens[i] {
                            Token::Keyword(k) if k == "endcidchar" => { i += 1; break; }
                            Token::Hex(code) => {
                                if let Some(Token::Integer(cid)) = tokens.get(i + 1) {
                                    cmap.single_cid.insert(code.clone(), *cid as u16);
                                    i += 2;
                                } else { i += 1; }
                            }
                            _ => i += 1,
                        }
                    }
                }
                Token::Keyword(k) if k == "begincidrange" => {
                    i += 1;
                    while i < tokens.len() {
                        match &tokens[i] {
                            Token::Keyword(k) if k == "endcidrange" => { i += 1; break; }
                            Token::Hex(lo) => {
                                match (tokens.get(i + 1), tokens.get(i + 2)) {
                                    (Some(Token::Hex(hi)), Some(Token::Integer(cid))) => {
                                        cmap.cid_ranges.push(CidRange {
                                            lo: lo.clone(),
                                            hi: hi.clone(),
                                            base_cid: *cid as u16,
                                        });
                                        i += 3;
                                    }
                                    _ => i += 1,
                                }
                            }
                            _ => i += 1,
                        }
                    }
                }
```

- [ ] **Step 4: Add `map_code_to_cid` + a byte-offset helper** to `impl EncodingCMap`:

```rust
    /// Map a character code to its CID. `single_cid` first, then `cid_ranges`.
    pub fn map_code_to_cid(&self, code: &[u8]) -> Option<u16> {
        if let Some(&cid) = self.single_cid.get(code) {
            return Some(cid);
        }
        for r in &self.cid_ranges {
            if code.len() == r.lo.len()
                && code.len() == r.hi.len()
                && code >= &r.lo[..]
                && code <= &r.hi[..]
            {
                let offset = be_offset(code, &r.lo);
                return Some(r.base_cid.wrapping_add(offset));
            }
        }
        None
    }
```

And a free helper at module scope (below the `impl`):

```rust
/// Big-endian numeric distance `code - lo`, saturating into u16.
fn be_offset(code: &[u8], lo: &[u8]) -> u16 {
    let to_u64 = |b: &[u8]| b.iter().fold(0u64, |acc, &x| (acc << 8) | x as u64);
    (to_u64(code).saturating_sub(to_u64(lo)) & 0xFFFF) as u16
}
```

- [ ] **Step 5: Run the test**

Run: `source ~/.cargo/env && cargo test -p oxidize-pdf --lib cidchar_and_cidrange 2>&1 | tail -15`
Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add oxidize-pdf-core/src/text/encoding_cmap.rs
git commit -m "feat(text-cmap): cidchar/cidrange parse + map_code_to_cid (addresses #272)

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>"
```

---

## Task 4: `notdefchar`/`notdefrange` + adversarial/hang guards

**Files:**
- Modify: `oxidize-pdf-core/src/text/encoding_cmap.rs`

- [ ] **Step 1: Write the failing tests** (notdef + adversarial) in `encoding_cmap.rs` `tests`:

```rust
    #[test]
    fn notdefrange_maps_to_notdef_cid() {
        let data = b"begincmap\n\
1 begincodespacerange <0000> <FFFF> endcodespacerange\n\
1 beginnotdefrange <0000> <001F> 0 endnotdefrange\n\
endcmap";
        let cmap = EncodingCMap::parse(data).expect("parse");
        // notdef ranges resolve to the notdef CID, distinct from "unmapped".
        assert_eq!(cmap.map_notdef(&[0x00, 0x10]), Some(0));
        assert_eq!(cmap.map_notdef(&[0x00, 0x41]), None);
    }

    #[test]
    fn adversarial_input_terminates_without_hang() {
        // Stray close delimiters and a dangling range must not loop forever.
        for data in [
            b">>>".as_slice(),
            b"begincmap\n1 begincidrange <0041>".as_slice(),
            b"]]] endcidchar beginnotdefrange".as_slice(),
        ] {
            let _ = EncodingCMap::parse(data).expect("must terminate, not hang");
        }
    }
```

- [ ] **Step 2: Run to confirm failure**

Run: `source ~/.cargo/env && cargo test -p oxidize-pdf --lib "encoding_cmap::tests::notdefrange" 2>&1 | tail -15`
Expected: FAIL (`map_notdef` not found).

- [ ] **Step 3: Parse notdef ranges.** Add match arms in `parse` (mirror cidchar/cidrange) before `_ => i += 1`:

```rust
                Token::Keyword(k) if k == "beginnotdefchar" => {
                    i += 1;
                    while i < tokens.len() {
                        match &tokens[i] {
                            Token::Keyword(k) if k == "endnotdefchar" => { i += 1; break; }
                            Token::Hex(code) => {
                                if let Some(Token::Integer(cid)) = tokens.get(i + 1) {
                                    cmap.notdef_ranges.push(CidRange {
                                        lo: code.clone(),
                                        hi: code.clone(),
                                        base_cid: *cid as u16,
                                    });
                                    i += 2;
                                } else { i += 1; }
                            }
                            _ => i += 1,
                        }
                    }
                }
                Token::Keyword(k) if k == "beginnotdefrange" => {
                    i += 1;
                    while i < tokens.len() {
                        match &tokens[i] {
                            Token::Keyword(k) if k == "endnotdefrange" => { i += 1; break; }
                            Token::Hex(lo) => {
                                match (tokens.get(i + 1), tokens.get(i + 2)) {
                                    (Some(Token::Hex(hi)), Some(Token::Integer(cid))) => {
                                        cmap.notdef_ranges.push(CidRange {
                                            lo: lo.clone(), hi: hi.clone(), base_cid: *cid as u16,
                                        });
                                        i += 3;
                                    }
                                    _ => i += 1,
                                }
                            }
                            _ => i += 1,
                        }
                    }
                }
```

- [ ] **Step 4: Add `map_notdef`** to `impl EncodingCMap`:

```rust
    /// Resolve a code that falls in a notdef range to its notdef CID.
    pub fn map_notdef(&self, code: &[u8]) -> Option<u16> {
        for r in &self.notdef_ranges {
            if code.len() == r.lo.len()
                && code >= &r.lo[..]
                && code <= &r.hi[..]
            {
                return Some(r.base_cid);
            }
        }
        None
    }
```

> The `parse` loop already advances `i` in every arm (no `break` without a cursor move), so the adversarial test passes once the arms exist — the outer `_ => i += 1` and the inner `_ => i += 1` guarantee progress. Verify this invariant when reviewing.

- [ ] **Step 5: Run tests**

Run: `source ~/.cargo/env && cargo test -p oxidize-pdf --lib encoding_cmap 2>&1 | tail -20`
Expected: PASS (all encoding_cmap unit tests).

- [ ] **Step 6: Commit**

```bash
git add oxidize-pdf-core/src/text/encoding_cmap.rs
git commit -m "feat(text-cmap): notdef ranges + adversarial parse guards (addresses #272)

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>"
```

---

## Task 5: `CidEncoding` enum + UTF-16BE decoder + predefined-name resolution (Uni\*)

**Files:**
- Modify: `oxidize-pdf-core/src/text/encoding_cmap.rs`

- [ ] **Step 1: Write the failing tests** in `encoding_cmap.rs` `tests`:

```rust
    #[test]
    fn utf16be_decodes_bmp_and_surrogates() {
        // U+4E2D (中) then U+1F600 (😀, surrogate pair D83D DE00).
        let bytes = [0x4E, 0x2D, 0xD8, 0x3D, 0xDE, 0x00];
        assert_eq!(decode_utf16be(&bytes), "中😀");
    }

    #[test]
    fn predefined_uni_families_resolve_to_utf16be() {
        assert!(matches!(resolve_predefined("UniGB-UCS2-H"), Some(CidEncoding::Utf16Be)));
        assert!(matches!(resolve_predefined("UniJIS-UTF16-H"), Some(CidEncoding::Utf16Be)));
        assert!(matches!(resolve_predefined("UniKS-UTF16-H"), Some(CidEncoding::Utf16Be)));
        assert!(resolve_predefined("WhateverUnknown-H").is_none());
    }
```

- [ ] **Step 2: Run to confirm failure**

Run: `source ~/.cargo/env && cargo test -p oxidize-pdf --lib "encoding_cmap::tests::utf16be" 2>&1 | tail -15`
Expected: FAIL (`decode_utf16be`/`CidEncoding`/`resolve_predefined` not found).

- [ ] **Step 3: Add the enum + decoder + resolver** at module scope in `encoding_cmap.rs`:

```rust
/// The resolved, non-Identity encoding of a Type0 font, as carried on `FontInfo`.
#[derive(Debug, Clone)]
pub(crate) enum CidEncoding {
    /// `Uni*-UCS2-*` / `Uni*-UTF16-*`: the code IS a UTF-16BE value.
    Utf16Be,
    /// An embedded stream CMap or a vendored predefined CMap (code → CID).
    Cmap(EncodingCMap),
}

/// Decode a byte string as UTF-16BE, replacing malformed units with U+FFFD.
pub(crate) fn decode_utf16be(bytes: &[u8]) -> String {
    let units: Vec<u16> = bytes
        .chunks(2)
        .filter(|c| c.len() == 2)
        .map(|c| u16::from_be_bytes([c[0], c[1]]))
        .collect();
    char::decode_utf16(units)
        .map(|r| r.unwrap_or('\u{FFFD}'))
        .collect()
}

/// Resolve a predefined `/Encoding` name. `Uni*-UCS2-*`/`Uni*-UTF16-*` are
/// algorithmic UTF-16BE. Vendored multibyte names are added in Task 7.
/// Unknown names return `None` (caller falls back to current behavior).
pub(crate) fn resolve_predefined(name: &str) -> Option<CidEncoding> {
    if name.starts_with("Uni") && (name.contains("UCS2") || name.contains("UTF16")) {
        return Some(CidEncoding::Utf16Be);
    }
    None
}
```

- [ ] **Step 4: Run the tests**

Run: `source ~/.cargo/env && cargo test -p oxidize-pdf --lib encoding_cmap 2>&1 | tail -20`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add oxidize-pdf-core/src/text/encoding_cmap.rs
git commit -m "feat(text-cmap): CidEncoding enum + UTF-16BE decode + Uni* resolution (addresses #272)

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>"
```

---

## Task 6: Wire `cid_encoding` into `FontInfo`, extraction, and decode precedence

**Files:**
- Modify: `oxidize-pdf-core/src/text/extraction_cmap.rs` (`FontInfo`, `extract_font_info`, `decode_text_with_font`)
- Modify: `oxidize-pdf-core/src/text/extraction.rs` (FontInfo construction sites)
- Test: `oxidize-pdf-core/tests/encoding_embedded_stream_test.rs`

- [ ] **Step 1: Write the failing integration test** with a synthetic embedded-stream CID font. `oxidize-pdf-core/tests/encoding_embedded_stream_test.rs`:

```rust
use oxidize_pdf::text::extraction_cmap::{decode_text_with_font, FontInfo, FontMetrics};
use oxidize_pdf::text::encoding_cmap::{CidEncoding, EncodingCMap};

/// A Type0 font with an embedded code→CID CMap and a GB1 descendant.
/// Code <0041> → CID via cidrange → GB1 CID→Unicode. We assert the decoded
/// char equals the GB1 table's entry for that CID (real content, not shape).
#[test]
fn embedded_encoding_cmap_decodes_via_cid_table() {
    use oxidize_pdf::text::cid_to_unicode::CidCollection;
    // Pick a CID present in GB1 and find a code that maps to it.
    let coll = CidCollection::from_ordering("GB1").unwrap();
    let cid: u16 = 1; // CID 1 is mapped in every Adobe collection (space-ish)
    let expected = coll.cid_to_unicode(cid).unwrap();

    let enc = EncodingCMap::parse(
        format!(
            "begincmap\n1 begincodespacerange <0000> <FFFF> endcodespacerange\n\
1 begincidchar <0041> {cid} endcidchar\nendcmap"
        )
        .as_bytes(),
    )
    .unwrap();

    let descendant = FontInfo {
        name: "Desc".into(), font_type: "CIDFontType0".into(),
        encoding: None, to_unicode: None, differences: None, descendant_font: None,
        cid_to_gid_map: None, cid_ordering: Some("GB1".into()),
        metrics: FontMetrics::default(), cid_encoding: None,
    };
    let parent = FontInfo {
        name: "Type0".into(), font_type: "Type0".into(),
        encoding: None, to_unicode: None, differences: None,
        descendant_font: Some(Box::new(descendant)),
        cid_to_gid_map: None, cid_ordering: None,
        metrics: FontMetrics::default(),
        cid_encoding: Some(CidEncoding::Cmap(enc)),
    };

    let out = decode_text_with_font(&[0x00, 0x41], &parent).unwrap();
    assert_eq!(out, expected.to_string());
}
```

- [ ] **Step 2: Run to confirm failure**

Run: `source ~/.cargo/env && cargo test -p oxidize-pdf --test encoding_embedded_stream_test 2>&1 | tail -15`
Expected: FAIL to compile (`cid_encoding` field missing; `encoding_cmap` not `pub`).

- [ ] **Step 3: Make `encoding_cmap` module + items reachable from tests.** In `src/text/mod.rs`, change `pub(crate) mod encoding_cmap;` → `pub mod encoding_cmap;`. In `encoding_cmap.rs`, change `pub(crate) struct EncodingCMap` → `pub struct EncodingCMap`, `pub(crate) enum CidEncoding` → `pub enum CidEncoding`, and `pub(crate) fn decode_utf16be`/`resolve_predefined`/`tokenize`-users as needed → `pub`. (Keep `tokenize_cmap`/`Token` at `pub(crate)`.)

- [ ] **Step 4: Add the field to `FontInfo`** (`extraction_cmap.rs`, struct at line ~45). Add after `metrics`:

```rust
    /// Resolved non-Identity CID encoding (code→CID), if any.
    pub cid_encoding: Option<crate::text::encoding_cmap::CidEncoding>,
```

And in the `FontInfo` literal inside `extract_font_info` (line ~106), add `cid_encoding: None,`. In the test-module `FontInfo` literal at line ~820, add `cid_encoding: None,`.

- [ ] **Step 5: Update every other `FontInfo` construction site.** In `extraction.rs`, sites at lines ~2209, 2250, 2302, 2342, 2377, 2411 each construct `FontInfo { ... }`. Add `cid_encoding: None,` to each.

Run to find any missed: `source ~/.cargo/env && cargo build -p oxidize-pdf 2>&1 | grep -A2 "missing field" | head`
Add `cid_encoding: None,` wherever the compiler reports a missing field.

- [ ] **Step 6: Populate `cid_encoding` in `extract_font_info`** from `/Encoding`. Replace the encoding `match` block (lines ~135-153) so the `Name` and stream cases also resolve `cid_encoding`:

```rust
        // Extract encoding
        if let Some(encoding_obj) = font_dict.get("Encoding") {
            match encoding_obj {
                PdfObject::Name(enc_name) => {
                    font_info.encoding = Some(enc_name.0.clone());
                    if enc_name.0 != "Identity-H" && enc_name.0 != "Identity-V" {
                        font_info.cid_encoding =
                            crate::text::encoding_cmap::resolve_predefined(&enc_name.0);
                    }
                }
                PdfObject::Dictionary(enc_dict) => {
                    if let Some(base_enc) = enc_dict.get("BaseEncoding").and_then(|o| o.as_name()) {
                        font_info.encoding = Some(base_enc.0.clone());
                    }
                    if let Some(PdfObject::Array(differences)) = enc_dict.get("Differences") {
                        font_info.differences =
                            Some(self.parse_encoding_differences(&differences.0)?);
                    }
                }
                PdfObject::Reference(num, gen) => {
                    // Embedded CMap stream referenced indirectly.
                    if let Ok(PdfObject::Stream(stream)) = document.get_object(*num, *gen) {
                        if let Ok(data) = stream.decode(&ParseOptions::default()) {
                            if let Ok(enc) = crate::text::encoding_cmap::EncodingCMap::parse(&data) {
                                font_info.cid_encoding =
                                    Some(crate::text::encoding_cmap::CidEncoding::Cmap(enc));
                            }
                        }
                    }
                }
                _ => {}
            }
        }
```

- [ ] **Step 7: Insert the decode step** in `decode_text_with_font` (`extraction_cmap.rs`). In the `Type0` branch, after the descendant-ToUnicode check and before the existing `cid_ordering` block (around line ~552), add:

```rust
            // Non-Identity encoding: map code→CID (or UTF-16BE) before CID→Unicode.
            match &font_info.cid_encoding {
                Some(crate::text::encoding_cmap::CidEncoding::Utf16Be) => {
                    return Ok(crate::text::encoding_cmap::decode_utf16be(text_bytes));
                }
                Some(crate::text::encoding_cmap::CidEncoding::Cmap(enc)) => {
                    let ordering = descendant
                        .cid_ordering
                        .as_deref()
                        .or(font_info.cid_ordering.as_deref());
                    if let Some(coll) = ordering
                        .and_then(crate::text::cid_to_unicode::CidCollection::from_ordering)
                    {
                        return Ok(decode_via_encoding_cmap(text_bytes, enc, &coll));
                    }
                }
                None => {}
            }
```

- [ ] **Step 8: Add the `decode_via_encoding_cmap` free function** to `extraction_cmap.rs` (next to `decode_with_cid_table`):

```rust
/// Decode using an embedded/predefined encoding CMap (code→CID) followed by a
/// CID→Unicode collection. Walks variable-width codes per the CMap codespace.
fn decode_via_encoding_cmap(
    text_bytes: &[u8],
    enc: &crate::text::encoding_cmap::EncodingCMap,
    collection: &crate::text::cid_to_unicode::CidCollection,
) -> String {
    let mut result = String::new();
    let mut i = 0;
    while i < text_bytes.len() {
        let len = enc.code_len_at(text_bytes, i).max(1).min(text_bytes.len() - i);
        let code = &text_bytes[i..i + len];
        if let Some(cid) = enc.map_code_to_cid(code) {
            match collection.cid_to_unicode(cid) {
                Some(ch) => result.push(ch),
                None if cid > 0 => result.push('\u{FFFD}'),
                None => {}
            }
        } else if enc.map_notdef(code).is_some() {
            result.push('\u{FFFD}');
        } else {
            result.push('\u{FFFD}');
        }
        i += len;
    }
    result
}
```

- [ ] **Step 9: Run the integration test + lib**

Run: `source ~/.cargo/env && cargo test -p oxidize-pdf --test encoding_embedded_stream_test && cargo test -p oxidize-pdf --lib 2>&1 | tail -5`
Expected: PASS; lib test count unchanged-or-higher, 0 failures.

- [ ] **Step 10: Commit**

```bash
git add oxidize-pdf-core/src/text/extraction_cmap.rs oxidize-pdf-core/src/text/extraction.rs \
        oxidize-pdf-core/src/text/mod.rs oxidize-pdf-core/src/text/encoding_cmap.rs \
        oxidize-pdf-core/tests/encoding_embedded_stream_test.rs
git commit -m "feat(text-extract): wire code→CID encoding CMap into Type0 decode (addresses #272)

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>"
```

---

## Task 7: Vendored Adobe predefined CMaps (GBK-EUC-H et al.) + GBK-EUC-H corpus test

**Files:**
- Create: `oxidize-pdf-core/src/text/cmap_resources/{GBK-EUC-H,GBKp-EUC-H,90ms-RKSJ-H,90pv-RKSJ-H,KSCms-UHC-H}` (+ `LICENSE`)
- Modify: `oxidize-pdf-core/src/text/encoding_cmap.rs` (vendored resolution, lazy cache)
- Create: `oxidize-pdf-core/tests/fixtures/issue_272_gbk_euc_h.pdf`
- Test: `oxidize-pdf-core/tests/encoding_gbk_euc_h_test.rs`

- [ ] **Step 1: Vendor the CMap files.** Clone Adobe cmap-resources and copy the five `*-H` files (BSD-3-Clause):

```bash
git clone --depth 1 https://github.com/adobe-type-tools/cmap-resources /tmp/cmap-resources
mkdir -p oxidize-pdf-core/src/text/cmap_resources
cp /tmp/cmap-resources/Adobe-GB1-7/CMap/GBK-EUC-H   oxidize-pdf-core/src/text/cmap_resources/
cp /tmp/cmap-resources/Adobe-GB1-7/CMap/GBKp-EUC-H  oxidize-pdf-core/src/text/cmap_resources/
cp /tmp/cmap-resources/Adobe-Japan1-7/CMap/90ms-RKSJ-H oxidize-pdf-core/src/text/cmap_resources/
cp /tmp/cmap-resources/Adobe-Japan1-7/CMap/90pv-RKSJ-H oxidize-pdf-core/src/text/cmap_resources/
cp /tmp/cmap-resources/Adobe-Korea1-2/CMap/KSCms-UHC-H oxidize-pdf-core/src/text/cmap_resources/
cp /tmp/cmap-resources/LICENSE oxidize-pdf-core/src/text/cmap_resources/LICENSE
```

(If a directory version differs, list `/tmp/cmap-resources/*/CMap/` and use the actual path. If any of these `*-H` files contains a `usecmap` of another vendored file, also copy that base file — the resolver in Step 4 merges it.)

- [ ] **Step 2: Verify the GBK fixture parses and reproduce the bug.** Choose a GBK-EUC-H PDF that oxidize can open:

```bash
for f in issue14438 issue20453; do
  echo "== $f =="; \
  test -f test-corpus/t1-spec/pdfjs/$f.pdf && echo present;
done
cp test-corpus/t1-spec/pdfjs/issue14438.pdf \
   oxidize-pdf-core/tests/fixtures/issue_272_gbk_euc_h.pdf
git add oxidize-pdf-core/tests/fixtures/issue_272_gbk_euc_h.pdf
```

If `issue14438` does not open even with `ParseOptions::tolerant()` (verify in Step 3), pick another from the GBK-EUC-H list (`issue20453`, or any from the scan) and use it instead.

- [ ] **Step 3: Write the failing corpus test.** `oxidize-pdf-core/tests/encoding_gbk_euc_h_test.rs`:

```rust
use oxidize_pdf::parser::{ParseOptions, PdfDocument, PdfReader};
use oxidize_pdf::text::TextExtractor;
use std::fs::File;

/// A real GBK-EUC-H PDF. Before the fix, the GBK codes are treated as CIDs
/// (Identity) and decode to garbage. After: real CJK Unified Ideographs.
#[test]
fn gbk_euc_h_extracts_real_cjk() {
    let path = "tests/fixtures/issue_272_gbk_euc_h.pdf";
    let doc = PdfDocument::new(
        PdfReader::new_with_options(File::open(path).unwrap(), ParseOptions::tolerant()).unwrap(),
    );
    let mut ext = TextExtractor::default();
    let n = doc.page_count().unwrap();
    let mut text = String::new();
    for p in 0..n.min(3) {
        if let Ok(r) = ext.extract_from_page(&doc, p) { text.push_str(&r.text); }
    }
    let cjk = text.chars().filter(|&c| ('\u{4E00}'..='\u{9FFF}').contains(&c)).count();
    assert!(cjk >= 5, "expected real CJK ideographs, got: {:?}", &text.chars().take(80).collect::<String>());
}
```

- [ ] **Step 4: Add vendored resolution with a lazy cache** in `encoding_cmap.rs`. Add `use std::sync::OnceLock;` at the top and:

```rust
macro_rules! vendored {
    ($name:literal, $file:literal) => {{
        static CELL: OnceLock<Option<EncodingCMap>> = OnceLock::new();
        CELL.get_or_init(|| {
            let mut cmap = EncodingCMap::parse(include_bytes!(concat!("cmap_resources/", $file)))
                .ok()?;
            // Resolve a usecmap to another vendored base by merging (base first).
            if let Some(parent) = cmap.usecmap_parent.clone() {
                if let Some(CidEncoding::Cmap(base)) = resolve_predefined(&parent) {
                    let mut merged = base;
                    merged.single_cid.extend(cmap.single_cid.drain());
                    merged.cid_ranges.append(&mut cmap.cid_ranges);
                    merged.notdef_ranges.append(&mut cmap.notdef_ranges);
                    if !cmap.codespace_ranges.is_empty() {
                        merged.codespace_ranges = cmap.codespace_ranges;
                    }
                    cmap = merged;
                }
            }
            Some(cmap)
        })
        .clone()
        .map(CidEncoding::Cmap)
    }};
}
```

Then extend `resolve_predefined` (before the final `None`):

```rust
    match name {
        "GBK-EUC-H"  => return vendored!("GBK-EUC-H",  "GBK-EUC-H"),
        "GBKp-EUC-H" => return vendored!("GBKp-EUC-H", "GBKp-EUC-H"),
        "90ms-RKSJ-H" => return vendored!("90ms-RKSJ-H", "90ms-RKSJ-H"),
        "90pv-RKSJ-H" => return vendored!("90pv-RKSJ-H", "90pv-RKSJ-H"),
        "KSCms-UHC-H" => return vendored!("KSCms-UHC-H", "KSCms-UHC-H"),
        _ => {}
    }
```

> Ordering for vendored CMaps comes from the descendant font's `/CIDSystemInfo`
> (already read into `cid_ordering`), so the decode path in Task 6 uses the
> right collection without storing ordering on the vendored `EncodingCMap`.

- [ ] **Step 5: Add a vendored-load unit test** in `encoding_cmap.rs` `tests`:

```rust
    #[test]
    fn gbk_euc_h_loads_and_maps_ascii_range() {
        let enc = match resolve_predefined("GBK-EUC-H") {
            Some(CidEncoding::Cmap(c)) => c,
            other => panic!("expected vendored Cmap, got {other:?}"),
        };
        // GBK-EUC-H has a single-byte codespace covering ASCII.
        assert_eq!(enc.code_len_at(&[0x41], 0), 1);
        // Single-byte 0x41 ('A') maps to a CID (GB1 CID for 'A').
        assert!(enc.map_code_to_cid(&[0x41]).is_some(), "ASCII 'A' must map to a CID");
    }
```

- [ ] **Step 6: Run tests**

Run: `source ~/.cargo/env && cargo test -p oxidize-pdf --lib gbk_euc_h_loads && cargo test -p oxidize-pdf --test encoding_gbk_euc_h_test 2>&1 | tail -15`
Expected: PASS (vendored load maps ASCII; real PDF yields ≥5 CJK ideographs).

- [ ] **Step 7: Commit**

```bash
git add oxidize-pdf-core/src/text/cmap_resources oxidize-pdf-core/src/text/encoding_cmap.rs \
        oxidize-pdf-core/tests/encoding_gbk_euc_h_test.rs
git commit -m "feat(text-cmap): vendor Adobe predefined CJK CMaps + GBK-EUC-H decode (addresses #272)

Vendored GBK-EUC-H, GBKp-EUC-H, 90ms/90pv-RKSJ-H, KSCms-UHC-H from
adobe-type-tools/cmap-resources (BSD-3-Clause), parsed lazily.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>"
```

---

## Task 8: Regression gate — corpus, rag_realworld, bench, clippy

**Files:** none (verification only). Fix regressions in the relevant module if found.

- [ ] **Step 1: Lib + integration suite**

Run: `source ~/.cargo/env && nice cargo test -p oxidize-pdf --lib 2>&1 | tail -5`
Expected: 0 failed (count ≥ baseline 6426 + new tests).

- [ ] **Step 2: Corpus tier-tests INDIVIDUALLY** (error-log 2026-05-24 — never declare green from a summary grep):

```bash
source ~/.cargo/env
nice cargo test -p oxidize-pdf --test t1_spec t1_pdfjs_corpus -- --ignored --nocapture 2>&1 | tail -20
nice cargo test -p oxidize-pdf --test t3_stress t3_zero_panics_on_stress_corpus -- --ignored --nocapture 2>&1 | tail -20
```

Expected: t1 PASS (pass-rate ≥ baseline, 0 timeouts); t3 PASS (0 panics, 0 timeouts on 1802 PDFs). If either hangs, the new parser violated the progress invariant — fix before proceeding.

- [ ] **Step 3: rag_realworld content unchanged**

Run: `source ~/.cargo/env && nice cargo run --release --example rag_realworld 2>&1 | tail -8`
Expected: 5/5 documents; BOE chunk 0 still `MINISTERIO DE ECONOMÍA...`; Higgs chunk 0 still `EUROPEAN ORGANISATION...`.

- [ ] **Step 4: Bench within budget**

Run: `source ~/.cargo/env && nice cargo bench -p oxidize-pdf --bench text_extraction 2>&1 | tail -20`
Expected: `text_extraction_*` within ±10% of `v2.0.0-profiling` (the code→CID step runs only for non-Identity Type0 fonts; Latin PDFs unaffected).

- [ ] **Step 5: Clippy clean on modified files**

Run: `source ~/.cargo/env && cargo clippy -p oxidize-pdf --lib -- -D warnings 2>&1 | tail -20`
Expected: no new warnings in `cmap.rs`/`encoding_cmap.rs`/`extraction_cmap.rs` (pre-existing warnings elsewhere are not introduced here).

- [ ] **Step 6: Commit any regression fixes** (if Steps 1-5 required code changes):

```bash
git add -A oxidize-pdf-core/src
git commit -m "fix(text-cmap): address regression-gate findings (addresses #272)

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>"
```

---

## Self-Review (author checklist — completed)

**Spec coverage:**
- Change A (usecmap external in ToUnicode) → Task 1. ✓
- EncodingCMap parse (cidchar/cidrange/notdef) + variable-width codespace → Tasks 2–4. ✓
- `CidEncoding` enum + algorithmic UTF-16BE for `Uni*` → Task 5. ✓
- FontInfo integration + decode precedence (ToUnicode still preferred) → Task 6. ✓
- Vendored Adobe CMaps (GBK-EUC-H, GBKp-EUC-H, 90ms/90pv-RKSJ-H, KSCms-UHC-H) → Task 7. ✓
- Error handling (FFFD, progress invariant, unknown name fallback) → Tasks 4, 6, 7. ✓
- Testing (unit, algorithmic, usecmap fixture, GBK corpus, regression, bench) → Tasks 1–8. ✓
- YAGNI boundaries (extraction only, no CIDToGIDMap, no codegen) → respected. ✓

**Placeholder scan:** No TBD/TODO; every code step shows complete code; commands have expected output. The only conditional is the fixture-selection fallback in Task 7 (explicit alternative given), which is a real environment branch, not a placeholder.

**Type consistency:** `EncodingCMap`, `CidRange { lo, hi, base_cid }`, `CidEncoding::{Utf16Be, Cmap}`, `code_len_at`, `map_code_to_cid`, `map_notdef`, `decode_utf16be`, `resolve_predefined`, `decode_via_encoding_cmap`, `inherited_ordering`, `FontInfo.cid_encoding` — names used identically across Tasks 1–8. `CidCollection::from_ordering`/`cid_to_unicode` match the existing `cid_to_unicode.rs` API.

**Known risk carried from spec:** vendored-data byte size — measure after Task 7 (`ls -l target/.../liboxidize_pdf*`) and report to the user; if disproportionate, consider feature-gating `cmap_resources` includes.
