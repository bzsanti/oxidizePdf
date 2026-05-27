# Marked-Content Extraction (Tagged-PDF Phase 1) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Wire BDC/BMC/EMC marked-content semantics into `text/extraction.rs` so `TextFragment` carries `mcid` + `struct_tag`, two overlaid blocks at the same baseline stay on distinct logical lines, `/ActualText` overrides decoded glyphs, and `/Artifact` content is filtered by default. Closes #269 and the NCSC alphabet-soup regression.

**Architecture:** Replace the parser's lossy `HashMap<String,String>` properties carrier with a typed `MarkedContentValue`/`MarkedContentProps` (preserves UTF-16BE bytes). Extractor pushes/pops a `mc_stack` on BDC/EMC, tags each emitted fragment with the innermost ancestor's `mcid` and `tag`, filters Artifact subtrees by default, collapses ActualText runs to a single fragment with the substituted text, and adds `mcid` to the line/merge grouping keys so same-baseline overlaid blocks no longer interleave.

**Tech Stack:** Rust (workspace MSRV 1.77), `oxidize-pdf-core` crate. Tests are integration-level in `oxidize-pdf-core/tests/`, content-verifying per `CLAUDE.md` (no smoke tests). Synthetic PDFs handcrafted via the `build_pdf_with_content_stream` pattern already used in `tests/issue_235_tj_fragment_emission_test.rs`. Real corpus: `corpus_cache/e0e3ff11371c09c2.pdf` (NCSC CAF v4.0, ~615 KB, already present locally).

**Spec:** `docs/superpowers/specs/2026-05-21-marked-content-extraction-design.md`. **Branch:** `fix/issue-269-marked-content-extraction` (already checked out, 1 commit ahead of develop with spec only).

---

## File Map

**Modify (production):**
- `oxidize-pdf-core/src/parser/content.rs` — add `MarkedContentValue`, `MarkedContentProps`; rewrite `pop_dict_or_name`; update `ContentOperation::BeginMarkedContentWithProps` / `DefineMarkedContentPointWithProps` variants; update internal tests (lines 1681-2750).
- `oxidize-pdf-core/src/text/extraction.rs` — add `mc_stack` + `pending_actualtext` fields to `TextState`; add `MarkedContentEntry`, `PendingActualText` structs; add `decode_pdf_string` + `resolve_props` helpers; handle BMC/BDC/EMC ops in `extract_from_page`; extend `emit_text_fragment` signature; add `mcid` to `merge_into_lines` / `merge_close_fragments` / `merge_into_paragraphs` grouping; add `mcid` + `struct_tag` fields to `TextFragment`; add `include_artifacts` to `ExtractionOptions`.
- `oxidize-pdf-core/src/text/table_detection.rs:656` — update `TextFragment {…}` literal with new fields.
- `oxidize-pdf-core/src/text/structured/detector.rs:155` — same.
- `oxidize-pdf-core/src/text/structured/keyvalue.rs:180` — same.

**Create (tests):**
- `oxidize-pdf-core/tests/common/synthetic_pdf.rs` — shared helper extracted from `issue_235_tj_fragment_emission_test.rs::build_pdf_with_content_stream` (no copies).
- `oxidize-pdf-core/tests/marked_content_props_test.rs` — 3 parser-level tests (UTF-16BE, ResourceRef, Integer MCID).
- `oxidize-pdf-core/tests/extraction_mcid_test.rs` — 2 extract tests (overlaid baselines distinct, nested BDCs).
- `oxidize-pdf-core/tests/extraction_actualtext_test.rs` — 3 actualtext tests (literal, UTF-16BE, multi-Tj run).
- `oxidize-pdf-core/tests/extraction_artifact_test.rs` — 3 artifact tests (filtered default, opt-in, nested inheritance).
- `oxidize-pdf-core/tests/extraction_unbalanced_bdc_test.rs` — 2 defensive tests (extra EMC, dangling BDC).
- `oxidize-pdf-core/tests/marked_content_roundtrip_test.rs` — 1 writer↔extractor integration test.
- `oxidize-pdf-core/tests/ncsc_no_alphabet_soup_test.rs` — 1 NCSC real-corpus content test.

**Modify (test plumbing):**
- `oxidize-pdf-core/tests/issue_235_tj_fragment_emission_test.rs:35` — replace local `build_pdf_with_content_stream` with `mod common; use common::synthetic_pdf::*;` to avoid duplication once the helper is extracted.

---

## Conventions

**Commit message prefix:** `fix(text-extract):` for production changes; `test(text-extract):` for test-only commits; `refactor(parser):` for the `ContentOperation` reshape.

**Each task is one commit.** Run the pre-commit hook (already installed; runs `cargo fmt --check`, `cargo clippy -- -D warnings`, build, lib tests with metadata-aware skipping). If a hook step fails, fix in the *same* working tree and re-run; do **not** amend or skip the hook.

**Workspace commands:**
- Single test: `cargo test --test <file_stem> -p oxidize-pdf -- <test_name> --exact --nocapture`
- All new tests in this PR: `cargo test --test marked_content_props_test --test extraction_mcid_test --test extraction_actualtext_test --test extraction_artifact_test --test extraction_unbalanced_bdc_test --test marked_content_roundtrip_test --test ncsc_no_alphabet_soup_test -p oxidize-pdf`
- Full sanity: `cargo test --workspace --no-fail-fast 2>&1 | tail -40`

**Pre-existing build state:** `develop` baseline = 8561 tests, 0 fail, 0 warnings under `cargo clippy --all -- -D warnings`. Any task that introduces warnings must fix them in the same task.

---

## Task 1: Extract synthetic-PDF helper to shared `tests/common/`

**Files:**
- Create: `oxidize-pdf-core/tests/common/mod.rs`
- Create: `oxidize-pdf-core/tests/common/synthetic_pdf.rs`
- Modify: `oxidize-pdf-core/tests/issue_235_tj_fragment_emission_test.rs:25-90` (replace local helper with `mod common`).

**Rationale:** Phase 1 introduces 6 new test files that each need to handcraft a 1-page PDF whose `/Contents` is a literal byte sequence. Copy-pasting the 50-line helper into every file violates DRY and risks drift. Extract once.

- [ ] **Step 1: Create `tests/common/mod.rs` to expose the submodule**

`oxidize-pdf-core/tests/common/mod.rs`:
```rust
//! Shared test helpers. Each integration test that needs them declares
//! `#[path = "common/mod.rs"] mod common;` at file top.
pub mod synthetic_pdf;
```

- [ ] **Step 2: Move `build_pdf_with_content_stream` + `write_obj` to `common/synthetic_pdf.rs`**

Copy lines 24-90 of `oxidize-pdf-core/tests/issue_235_tj_fragment_emission_test.rs` verbatim into `oxidize-pdf-core/tests/common/synthetic_pdf.rs`, changing fn signatures to `pub fn`. Final file:

```rust
//! Handcrafted PDF builder for content-stream-level tests. Produces a minimal
//! valid 1-page PDF with a Type1 Helvetica font as `/F1` and the supplied
//! bytes as the `/Contents` stream. Identical layout to the helper introduced
//! by issue #235; extracted here so Phase 1 tests can reuse it without copy.

pub fn write_obj(bytes: &mut Vec<u8>, offset: &mut usize, body: &str) {
    *offset = bytes.len();
    bytes.extend_from_slice(body.as_bytes());
}

pub fn build_pdf_with_content_stream(content: &[u8]) -> Vec<u8> {
    let mut bytes: Vec<u8> = Vec::with_capacity(1024 + content.len());
    let mut offsets: Vec<usize> = vec![0; 6];

    bytes.extend_from_slice(b"%PDF-1.4\n%\xE2\xE3\xCF\xD3\n");

    write_obj(
        &mut bytes,
        &mut offsets[1],
        "1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n",
    );
    write_obj(
        &mut bytes,
        &mut offsets[2],
        "2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n",
    );
    write_obj(
        &mut bytes,
        &mut offsets[3],
        "3 0 obj\n<< /Type /Page /Parent 2 0 R /Resources << /Font << /F1 4 0 R >> >> /Contents 5 0 R /MediaBox [0 0 612 792] >>\nendobj\n",
    );
    write_obj(
        &mut bytes,
        &mut offsets[4],
        "4 0 obj\n<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>\nendobj\n",
    );

    offsets[5] = bytes.len();
    bytes.extend_from_slice(
        format!("5 0 obj\n<< /Length {} >>\nstream\n", content.len()).as_bytes(),
    );
    bytes.extend_from_slice(content);
    bytes.extend_from_slice(b"\nendstream\nendobj\n");

    let xref_off = bytes.len();
    bytes.extend_from_slice(b"xref\n0 6\n0000000000 65535 f \n");
    for off in offsets.iter().skip(1) {
        bytes.extend_from_slice(format!("{:010} 00000 n \n", off).as_bytes());
    }
    bytes.extend_from_slice(
        format!(
            "trailer\n<< /Size 6 /Root 1 0 R >>\nstartxref\n{}\n%%EOF\n",
            xref_off
        )
        .as_bytes(),
    );
    bytes
}
```

- [ ] **Step 3: Replace local helper in `issue_235_tj_fragment_emission_test.rs`**

Delete the local `fn write_obj` (lines 24-32) and `fn build_pdf_with_content_stream` (lines 35-90). At file top, after the existing `use` statements, add:

```rust
#[path = "common/mod.rs"]
mod common;
use common::synthetic_pdf::build_pdf_with_content_stream;
```

(`write_obj` is private to the helper; do not re-export.)

- [ ] **Step 4: Run `issue_235` suite to verify it still passes**

Run: `cargo test --test issue_235_tj_fragment_emission_test -p oxidize-pdf -- --nocapture`
Expected: 6 tests, 0 failures. Output ends with `test result: ok. 6 passed; 0 failed`.

- [ ] **Step 5: Run clippy on tests**

Run: `cargo clippy --tests -p oxidize-pdf -- -D warnings 2>&1 | tail -20`
Expected: no warnings, no errors.

- [ ] **Step 6: Commit**

```bash
git add oxidize-pdf-core/tests/common/ oxidize-pdf-core/tests/issue_235_tj_fragment_emission_test.rs
git commit -m "refactor(test): extract build_pdf_with_content_stream to tests/common/"
```

---

## Task 2: Add `MarkedContentValue` and `MarkedContentProps` types (parser, no behavior change yet)

**Files:**
- Modify: `oxidize-pdf-core/src/parser/content.rs:330-333` (variant signatures) + new types near top of file (around line 285, before `BeginMarkedContent`).

**Rationale:** Introduce the typed carrier as a strict superset (no behavior change yet — variants still typed `HashMap<String, String>`). This lets us land the type definitions and exports without breaking call sites; Task 4 swaps the variant types.

Actually, no — this task is misleading as written. We need a single atomic refactor (Task 3) that adds the types AND swaps the variants AND updates `pop_dict_or_name` together, otherwise we'd ship a dead type. **Skip this task — fold into Task 3.** *(Self-review note: the plan does not contain Task 2 as a separate commit. The numbering jumps from Task 1 to Task 3 to keep the rest of the plan stable.)*

---

## Task 3: Swap `ContentOperation::BeginMarkedContentWithProps` to `MarkedContentProps` (parser refactor, internal-only)

**Files:**
- Modify: `oxidize-pdf-core/src/parser/content.rs`
  - Add new types near line 285 (before `ContentOperation` enum).
  - Change variant signatures lines 330-333.
  - Rewrite `pop_dict_or_name` at lines 1315-1380.
  - Update internal parser tests at lines 1881-2615 that match on `BeginMarkedContent*` variants.

**Pre-flight check:** Confirm no production code outside `parser/content.rs` matches on `BeginMarkedContentWithProps(_, _)` or `DefineMarkedContentPointWithProps(_, _)`:

```bash
grep -rn "BeginMarkedContentWithProps\|DefineMarkedContentPointWithProps" --include="*.rs" oxidize-pdf-core/ | grep -v "src/parser/content.rs"
```

Expected output: empty (only `src/parser/content.rs` references these variants today). If non-empty, those call sites get updated in this same task.

- [ ] **Step 1: Write the failing parser test for UTF-16BE ActualText preservation**

Create `oxidize-pdf-core/tests/marked_content_props_test.rs`:

```rust
//! Parser-level tests for the typed marked-content properties carrier
//! (issue #269 Phase 1).

use oxidize_pdf::parser::content::{
    ContentOperation, ContentParser, MarkedContentProps, MarkedContentValue,
};

/// `BDC <</ActualText <FEFF00660069>>` MUST preserve the raw 6 bytes
/// `FE FF 00 66 00 69` (UTF-16BE BOM + "fi"). The old `HashMap<String,String>`
/// carrier ran the bytes through `String::from_utf8_lossy`, mangling the BOM
/// and producing `\u{FFFD}\u{FFFD}\0f\0i`.
#[test]
fn utf16be_actualtext_preserved_as_raw_bytes() {
    let stream = b"/Span <</ActualText <FEFF00660069>>> BDC EMC";
    let ops = ContentParser::parse_content(stream).expect("parse");

    let (tag, props) = match &ops[0] {
        ContentOperation::BeginMarkedContentWithProps(t, p) => (t, p),
        other => panic!("expected BeginMarkedContentWithProps, got {:?}", other),
    };
    assert_eq!(tag, "Span");

    let inline = match props {
        MarkedContentProps::Inline(map) => map,
        MarkedContentProps::ResourceRef(name) => {
            panic!("expected Inline, got ResourceRef({})", name)
        }
    };
    let actual = inline.get("ActualText").expect("/ActualText key present");
    let bytes = match actual {
        MarkedContentValue::String(b) => b,
        other => panic!("expected MarkedContentValue::String, got {:?}", other),
    };
    assert_eq!(
        bytes.as_slice(),
        &[0xFE, 0xFF, 0x00, 0x66, 0x00, 0x69],
        "UTF-16BE bytes must be preserved verbatim"
    );
}

/// `BDC /PropsName` (single name operand) produces `ResourceRef("PropsName")`,
/// not an `Inline` map with a `__resource_ref` magic key.
#[test]
fn resource_ref_props_parsed_as_resource_ref_variant() {
    let stream = b"/P /PropsName BDC EMC";
    let ops = ContentParser::parse_content(stream).expect("parse");

    let props = match &ops[0] {
        ContentOperation::BeginMarkedContentWithProps(_, p) => p,
        other => panic!("expected BeginMarkedContentWithProps, got {:?}", other),
    };
    match props {
        MarkedContentProps::ResourceRef(name) => assert_eq!(name, "PropsName"),
        MarkedContentProps::Inline(_) => panic!("expected ResourceRef, got Inline"),
    }
}

/// `BDC <</MCID 0>>` produces `MarkedContentValue::Integer(0)` for the
/// `MCID` key — never `String` or `Name`. MCID is the *only* required
/// integer-typed key for tagged PDFs.
#[test]
fn mcid_integer_value_preserved_as_integer_variant() {
    let stream = b"/P <</MCID 42>> BDC EMC";
    let ops = ContentParser::parse_content(stream).expect("parse");

    let inline = match &ops[0] {
        ContentOperation::BeginMarkedContentWithProps(_, MarkedContentProps::Inline(m)) => m,
        other => panic!("expected Inline props, got {:?}", other),
    };
    let mcid = inline.get("MCID").expect("/MCID key present");
    match mcid {
        MarkedContentValue::Integer(n) => assert_eq!(*n, 42),
        other => panic!("expected Integer(42), got {:?}", other),
    }
}
```

- [ ] **Step 2: Run the test to verify it fails to compile (types do not exist yet)**

Run: `cargo test --test marked_content_props_test -p oxidize-pdf 2>&1 | tail -20`
Expected: build failure — `error[E0432]: unresolved import oxidize_pdf::parser::content::MarkedContentProps` (and `MarkedContentValue`).

- [ ] **Step 3: Add the new types in `parser/content.rs`**

Insert before the `ContentOperation` enum definition (currently around line 285). Pick a spot just after the `TextElement` enum (which is at line 358) or before the enum — anywhere above `ContentOperation` works. Recommended placement: directly above `ContentOperation`, near line 285.

```rust
/// A single value inside a marked-content properties dictionary or array.
///
/// PDF marked-content properties (BDC, DP) carry typed values: strings,
/// integers, real numbers, names, arrays, and nested dictionaries. The
/// previous `HashMap<String, String>` carrier was lossy for `/ActualText`
/// (UTF-16BE bytes mangled by `String::from_utf8_lossy`) and for `/MCID`
/// (integer values stored as their decimal string representation). This
/// enum preserves the original token type and bytes; decoding happens
/// lazily at the extractor level (e.g. UTF-16BE detection via BOM).
///
/// Hex strings (`<FEFF00660069>`) and literal strings (`(text)`) both
/// land here as `MarkedContentValue::String(Vec<u8>)` because both are
/// raw byte sequences at the PDF tokenizer level.
#[derive(Debug, Clone, PartialEq)]
pub enum MarkedContentValue {
    /// Raw PDF string bytes (from either `Token::String` or `Token::HexString`).
    /// Decoded lazily by consumers — UTF-16BE detection via BOM happens in the
    /// extractor's `decode_pdf_string` helper.
    String(Vec<u8>),
    /// PDF integer (e.g. `/MCID 0`).
    Integer(i64),
    /// PDF real number.
    Real(f64),
    /// PDF name token (e.g. `/Pagination`).
    Name(String),
    /// PDF array; nested values are themselves `MarkedContentValue`.
    Array(Vec<MarkedContentValue>),
    /// Nested dictionary; keys are PDF name strings (the leading `/` is stripped).
    Dict(HashMap<String, MarkedContentValue>),
}

/// Properties operand of a BDC/DP operator. Two shapes per ISO 32000-1
/// §14.6.2:
///
/// - **Inline**: the second BDC operand is an inline dictionary literal
///   (`<< /MCID 0 /ActualText (fi) >>`). Keys map to `MarkedContentValue`.
/// - **ResourceRef**: the second BDC operand is a name (`/PropsName`) that
///   references the page's `/Resources /Properties /<name>` dictionary.
///   Resolution against the page's resource tree happens in the extractor
///   (parser does not have access to the page object).
#[derive(Debug, Clone, PartialEq)]
pub enum MarkedContentProps {
    Inline(HashMap<String, MarkedContentValue>),
    ResourceRef(String),
}
```

- [ ] **Step 4: Update `ContentOperation` variants**

In `parser/content.rs`, replace lines 330 and 333:

```rust
// Marked content operators
BeginMarkedContent(String),                              // BMC
BeginMarkedContentWithProps(String, MarkedContentProps), // BDC
EndMarkedContent,                                        // EMC
DefineMarkedContentPoint(String),                        // MP
DefineMarkedContentPointWithProps(String, MarkedContentProps), // DP
```

- [ ] **Step 5: Rewrite `pop_dict_or_name`**

Replace `oxidize-pdf-core/src/parser/content.rs:1315-1380` (current body) with the new typed implementation. The new signature returns `MarkedContentProps`:

```rust
fn pop_dict_or_name(&self, operands: &mut Vec<Token>) -> ParseResult<MarkedContentProps> {
    let token = operands.pop().ok_or_else(|| ParseError::SyntaxError {
        position: self.position,
        message: "Expected dict or name operand for BDC/DP".to_string(),
    })?;

    match token {
        Token::Name(name) => Ok(MarkedContentProps::ResourceRef(name)),
        Token::DictEnd => {
            // Inline dictionary. Stack layout (newest on top):
            //   ... DictStart Name(k1) Value(v1) Name(k2) Value(v2) DictEnd
            // We pop value-then-key pairs in reverse until we hit DictStart.
            let mut map: HashMap<String, MarkedContentValue> = HashMap::new();
            loop {
                let next = operands.pop().ok_or_else(|| ParseError::SyntaxError {
                    position: self.position,
                    message: "Unterminated inline dict in BDC/DP".to_string(),
                })?;
                if matches!(next, Token::DictStart) {
                    break;
                }
                let value = Self::token_to_mc_value(next, operands)?;
                let key = match operands.pop() {
                    Some(Token::Name(k)) => k,
                    Some(other) => {
                        return Err(ParseError::SyntaxError {
                            position: self.position,
                            message: format!(
                                "Expected Name as inline dict key, got {:?}",
                                other
                            ),
                        });
                    }
                    None => {
                        return Err(ParseError::SyntaxError {
                            position: self.position,
                            message: "Unterminated inline dict (missing key)".to_string(),
                        });
                    }
                };
                map.insert(key, value);
            }
            Ok(MarkedContentProps::Inline(map))
        }
        other => Err(ParseError::SyntaxError {
            position: self.position,
            message: format!(
                "Expected name or inline dict for BDC/DP, got {:?}",
                other
            ),
        }),
    }
}

/// Convert a popped token to a `MarkedContentValue`. For `ArrayEnd` and
/// `DictEnd` tokens we recursively collect the matching container; all
/// other tokens map to leaf variants.
fn token_to_mc_value(
    token: Token,
    operands: &mut Vec<Token>,
) -> ParseResult<MarkedContentValue> {
    match token {
        Token::String(b) | Token::HexString(b) => Ok(MarkedContentValue::String(b)),
        Token::Integer(i) => Ok(MarkedContentValue::Integer(i as i64)),
        Token::Number(f) => Ok(MarkedContentValue::Real(f as f64)),
        Token::Name(n) => Ok(MarkedContentValue::Name(n)),
        Token::ArrayEnd => {
            // Collect until matching ArrayStart.
            let mut items: Vec<MarkedContentValue> = Vec::new();
            loop {
                let next = operands.pop().ok_or_else(|| ParseError::SyntaxError {
                    position: 0, // not on `self`; static method
                    message: "Unterminated array in marked-content props".to_string(),
                })?;
                if matches!(next, Token::ArrayStart) {
                    break;
                }
                items.push(Self::token_to_mc_value(next, operands)?);
            }
            items.reverse();
            Ok(MarkedContentValue::Array(items))
        }
        Token::DictEnd => {
            let mut nested: HashMap<String, MarkedContentValue> = HashMap::new();
            loop {
                let next = operands.pop().ok_or_else(|| ParseError::SyntaxError {
                    position: 0,
                    message: "Unterminated nested dict in marked-content props".to_string(),
                })?;
                if matches!(next, Token::DictStart) {
                    break;
                }
                let value = Self::token_to_mc_value(next, operands)?;
                let key = match operands.pop() {
                    Some(Token::Name(k)) => k,
                    _ => {
                        return Err(ParseError::SyntaxError {
                            position: 0,
                            message: "Expected name key in nested dict".to_string(),
                        });
                    }
                };
                nested.insert(key, value);
            }
            Ok(MarkedContentValue::Dict(nested))
        }
        other => Err(ParseError::SyntaxError {
            position: 0,
            message: format!(
                "Unexpected token type in marked-content value: {:?}",
                other
            ),
        }),
    }
}
```

Note `token_to_mc_value` is an associated function (no `&self`) because of borrow constraints (it must recursively pop from `operands`). The `position: 0` placeholder is acceptable here because the outer caller wraps the result and the syntax-error path is exercised only by malformed PDFs.

- [ ] **Step 6: Update the two BDC/DP call sites in the same file**

At lines ~1198-1202 (BDC) and ~1209-1212 (DP) the existing call sites already call `self.pop_dict_or_name(operands)?` — they continue to compile since the signature change only changes the return type, and the result is forwarded directly into the variant constructor. Verify by reading the current code:

```rust
"BDC" => {
    let props = self.pop_dict_or_name(operands)?;
    let tag = self.pop_name(operands)?;
    ContentOperation::BeginMarkedContentWithProps(tag, props)
}
```

This compiles unchanged. No edit needed at the call sites if the signature change above is complete.

- [ ] **Step 7: Update internal parser tests in `parser/content.rs`**

Inspect lines 1881-2615 (and any other test that destructures or compares `BeginMarkedContentWithProps`). The grep at the top of this task confirmed there are matches at lines 1884, 1906, 2070, 2071, 2375, 2559, 2606, 2609, 2750. Most of these only match `BeginMarkedContent(_)` (BMC, not BDC) — those don't need changes. Run:

```bash
grep -n "BeginMarkedContentWithProps\|DefineMarkedContentPointWithProps" oxidize-pdf-core/src/parser/content.rs
```

Expected: only the two production-code lines (variant declaration + call site). If any test references these variants with the old `HashMap<String,String>` signature, update it. Likely zero changes here.

- [ ] **Step 8: Update `pop_dict_or_name` unit tests in `parser/content.rs` if present**

```bash
grep -n "pop_dict_or_name\|__resource_ref" oxidize-pdf-core/src/parser/content.rs
```

Expected matches: the function definition and possibly tests that asserted the old `__resource_ref` magic-key shape. If any test exercises the old shape, replace it with assertions on `MarkedContentProps::ResourceRef(...)`. If a test no longer makes sense after the refactor, rewrite it — do not delete or `#[ignore]`.

- [ ] **Step 9: Build the crate to flush compilation errors**

Run: `cargo build -p oxidize-pdf 2>&1 | tail -30`
Expected: builds clean. If errors mention removed `__resource_ref` magic key consumers outside parser, fix them.

- [ ] **Step 10: Run the new parser test**

Run: `cargo test --test marked_content_props_test -p oxidize-pdf 2>&1 | tail -20`
Expected: 3 tests, 0 failures.

- [ ] **Step 11: Run the full parser internal test suite**

Run: `cargo test --lib -p oxidize-pdf parser::content 2>&1 | tail -10`
Expected: all green.

- [ ] **Step 12: Run clippy**

Run: `cargo clippy --lib --tests -p oxidize-pdf -- -D warnings 2>&1 | tail -20`
Expected: clean.

- [ ] **Step 13: Commit**

```bash
git add oxidize-pdf-core/src/parser/content.rs oxidize-pdf-core/tests/marked_content_props_test.rs
git commit -m "refactor(parser): typed MarkedContentProps preserves raw bytes (addresses #269)"
```

---

## Task 4: Add `mcid` + `struct_tag` fields to `TextFragment`, default-populated everywhere

**Files:**
- Modify: `oxidize-pdf-core/src/text/extraction.rs:93-116` (struct definition).
- Modify: `oxidize-pdf-core/src/text/extraction.rs:358-369` (literal at `merge_into_paragraphs`).
- Modify: `oxidize-pdf-core/src/text/extraction.rs:1189-1201` (literal at `emit_text_fragment`).
- Modify: `oxidize-pdf-core/src/text/extraction.rs:1399-1411` (literal at `build_line_fragment`).
- Modify: `oxidize-pdf-core/src/text/extraction.rs:1528, 1552, 1565` (literals inside `#[cfg(test)]` helpers).
- Modify: `oxidize-pdf-core/src/text/extraction.rs:2222-2230` (the `tf` helper).
- Modify: `oxidize-pdf-core/src/text/table_detection.rs:656`.
- Modify: `oxidize-pdf-core/src/text/structured/detector.rs:155`.
- Modify: `oxidize-pdf-core/src/text/structured/keyvalue.rs:180`.

**Rationale:** Adding fields to a `pub` struct is a public-API addition (additive only — non-breaking for consumers who construct via existing fns; **breaking** only for downstream callers that build `TextFragment` with struct-literal syntax). Phase 1 is internal: there are no known downstream literal constructors outside this repo. Update every in-repo literal to default `mcid: None, struct_tag: None`.

- [ ] **Step 1: Enumerate all literal sites**

```bash
grep -rn "TextFragment {" --include="*.rs" oxidize-pdf-core/ | grep -v "//"
```

Expected (current snapshot, 2026-05-21):
- `oxidize-pdf-core/src/text/table_detection.rs:656`
- `oxidize-pdf-core/src/text/structured/detector.rs:155`
- `oxidize-pdf-core/src/text/structured/keyvalue.rs:180`
- `oxidize-pdf-core/src/text/extraction.rs:358`, `:1189`, `:1399`, `:1528`, `:1552`, `:1565`, `:2223`

That's 10 sites. Every one needs `mcid: None, struct_tag: None` added.

- [ ] **Step 2: Update the struct definition**

Replace `oxidize-pdf-core/src/text/extraction.rs:91-116`:

```rust
/// A fragment of text with position information
#[derive(Debug, Clone)]
pub struct TextFragment {
    /// Text content
    pub text: String,
    /// X position in page coordinates
    pub x: f64,
    /// Y position in page coordinates
    pub y: f64,
    /// Width of the text
    pub width: f64,
    /// Height of the text
    pub height: f64,
    /// Font size
    pub font_size: f64,
    /// Font name (if known) - used for kerning-aware text spacing
    pub font_name: Option<String>,
    /// Whether the font is bold (detected from font name)
    pub is_bold: bool,
    /// Whether the font is italic (detected from font name)
    pub is_italic: bool,
    /// Fill color of the text (from graphics state)
    pub color: Option<Color>,
    /// Space insertion decisions (empty unless `track_space_decisions` is true).
    pub space_decisions: Vec<SpaceDecision>,
    /// Marked-content identifier from the innermost ancestor BDC with `/MCID`
    /// (issue #269 Phase 1). `None` for non-tagged PDFs, which preserves the
    /// pre-Phase-1 grouping behavior (`None == None` collapses to legacy keys).
    pub mcid: Option<u32>,
    /// Structural tag of the owning BDC (e.g. `"P"`, `"H1"`, `"Figure"`,
    /// `"Artifact"`). Set on the same ancestor that supplied `mcid`. Phase 3
    /// will consume this for partitioner classification; Phase 1 only carries it.
    pub struct_tag: Option<String>,
}
```

- [ ] **Step 3: Update each literal — extraction.rs**

For each of `:358`, `:1189`, `:1399`, `:1528`, `:1552`, `:1565`, `:2223`, add the two lines just before the closing `}`:

```rust
            mcid: None,
            struct_tag: None,
        });
```

(Or `,` instead of `;` for fn return position.) Concrete diff for `emit_text_fragment` at line ~1189:

```rust
    fragments.push(TextFragment {
        text: decoded.to_owned(),
        x,
        y,
        width: effective_width,
        height: effective_size,
        font_size: effective_size,
        font_name: state.font_name.clone(),
        is_bold,
        is_italic,
        color: state.fill_color,
        space_decisions: Vec::new(),
        mcid: None,        // wired in Task 9
        struct_tag: None,  // wired in Task 9
    });
```

For `build_line_fragment` at ~1399: take the head's mcid/struct_tag (since lines should not span mcid boundaries — Task 8 guarantees this; for now `None` is fine here as well, since this fn runs *after* `merge_into_lines` has grouped):

```rust
    TextFragment {
        text,
        x: x_min,
        y: y_min,
        width: x_max - x_min,
        height: y_max - y_min,
        font_size: head.font_size,
        font_name: head.font_name.clone(),
        is_bold: head.is_bold,
        is_italic: head.is_italic,
        color: head.color,
        space_decisions: Vec::new(),
        mcid: head.mcid,
        struct_tag: head.struct_tag.clone(),
    }
```

For the paragraph join at ~358:

```rust
    current = TextFragment {
        text: joined_text,
        x: x_min,
        y: y_min,
        width: x_max - x_min,
        height: y_max - y_min,
        font_size: current.font_size,
        font_name: current.font_name.clone(),
        is_bold: current.is_bold,
        is_italic: current.is_italic,
        color: current.color,
        space_decisions: Vec::new(),
        mcid: current.mcid,
        struct_tag: current.struct_tag.clone(),
    };
```

For test helpers (`:1528`, `:1552`, `:1565`, `:2223`): plain `mcid: None, struct_tag: None`.

- [ ] **Step 4: Update `table_detection.rs:656`**

Open `oxidize-pdf-core/src/text/table_detection.rs` near line 656 (a `.map(|frag| TextFragment { ... })`). Add `mcid: frag.mcid, struct_tag: frag.struct_tag.clone(),` (or `None, None,` if the surrounding `frag` is some other type — read the closure body first to determine).

```bash
sed -n '650,680p' oxidize-pdf-core/src/text/table_detection.rs
```

Add the appropriate fields based on what's available in the closure.

- [ ] **Step 5: Update `structured/detector.rs:155`**

```bash
sed -n '150,170p' oxidize-pdf-core/src/text/structured/detector.rs
```

Most likely a test fixture — set `mcid: None, struct_tag: None`.

- [ ] **Step 6: Update `structured/keyvalue.rs:180`**

```bash
sed -n '175,200p' oxidize-pdf-core/src/text/structured/keyvalue.rs
```

This is the `create_fragment` test helper. Add the two fields with `None`.

- [ ] **Step 7: Build**

Run: `cargo build -p oxidize-pdf 2>&1 | tail -20`
Expected: clean. Any "missing field" error points to a literal we missed — grep again and fix.

- [ ] **Step 8: Run extraction tests**

Run: `cargo test --lib -p oxidize-pdf text::extraction 2>&1 | tail -20`
Expected: all green (we only added defaulted fields).

- [ ] **Step 9: Run full lib test**

Run: `cargo test --lib -p oxidize-pdf 2>&1 | tail -10`
Expected: 6000+ tests, 0 failures.

- [ ] **Step 10: Clippy**

Run: `cargo clippy --lib --tests -p oxidize-pdf -- -D warnings 2>&1 | tail -10`
Expected: clean.

- [ ] **Step 11: Commit**

```bash
git add oxidize-pdf-core/src/text/extraction.rs oxidize-pdf-core/src/text/table_detection.rs oxidize-pdf-core/src/text/structured/detector.rs oxidize-pdf-core/src/text/structured/keyvalue.rs
git commit -m "feat(text-extract): add mcid/struct_tag fields to TextFragment (addresses #269)"
```

---

## Task 5: Add `include_artifacts` to `ExtractionOptions`

**Files:**
- Modify: `oxidize-pdf-core/src/text/extraction.rs:18-64` (struct + Default impl).

- [ ] **Step 1: Add field**

Replace `ExtractionOptions` (lines 17-64) — add `include_artifacts` field with doc + default `false`:

```rust
/// Text extraction options
#[derive(Debug, Clone)]
pub struct ExtractionOptions {
    /// Preserve the original layout (spacing and positioning)
    pub preserve_layout: bool,
    /// Minimum space width to insert space character (in text space units)
    pub space_threshold: f64,
    /// Minimum vertical distance to insert newline (in text space units)
    pub newline_threshold: f64,
    /// Sort text fragments by position (useful for multi-column layouts)
    pub sort_by_position: bool,
    /// Detect and handle columns
    pub detect_columns: bool,
    /// Column separation threshold (in page units)
    pub column_threshold: f64,
    /// Merge hyphenated words at line ends
    pub merge_hyphenated: bool,
    /// Track space insertion decisions in each TextFragment (default: false).
    pub track_space_decisions: bool,
    /// Reconstruct visual lines and paragraphs (see existing doc above).
    pub reconstruct_paragraphs: bool,
    /// Include content inside `/Artifact` marked-content scopes (page headers,
    /// footers, watermarks, decorative content). Default `false` — Artifact
    /// content is filtered out, as the PDF/UA conformance level recommends
    /// for accessibility tooling and as RAG callers consistently want
    /// (issue #269 Phase 1). Opt-in by setting `true` when extracting
    /// page furniture matters (e.g. forensic auditing, redaction tools).
    pub include_artifacts: bool,
}

impl Default for ExtractionOptions {
    fn default() -> Self {
        Self {
            preserve_layout: false,
            space_threshold: 0.3,
            newline_threshold: 10.0,
            sort_by_position: true,
            detect_columns: false,
            column_threshold: 50.0,
            merge_hyphenated: true,
            track_space_decisions: false,
            reconstruct_paragraphs: false,
            include_artifacts: false,
        }
    }
}
```

- [ ] **Step 2: Build to flush any literal-constructor of `ExtractionOptions`**

```bash
grep -rn "ExtractionOptions {" --include="*.rs" oxidize-pdf-core/ | head
cargo build -p oxidize-pdf 2>&1 | tail -10
```

If any literal-construction site exists, add `include_artifacts: false`. Most call sites use `ExtractionOptions::default()` and need no change.

- [ ] **Step 3: Run lib tests**

Run: `cargo test --lib -p oxidize-pdf 2>&1 | tail -5`
Expected: green.

- [ ] **Step 4: Commit**

```bash
git add oxidize-pdf-core/src/text/extraction.rs
git commit -m "feat(text-extract): add include_artifacts option (default false, addresses #269)"
```

---

## Task 6: Add `mc_stack`, `MarkedContentEntry`, `PendingActualText` to `TextState`

**Files:**
- Modify: `oxidize-pdf-core/src/text/extraction.rs:118-176` (TextState + Default).

**Rationale:** Foundation for BMC/BDC/EMC handling in Task 7. No behavior change yet — fields are populated but not read until Task 7+.

- [ ] **Step 1: Add new types just above `TextState`**

Insert at `oxidize-pdf-core/src/text/extraction.rs` before line 118:

```rust
/// One entry on the marked-content stack maintained by `TextState`.
///
/// PDF marked-content operators (BDC/BMC/EMC) form a balanced LIFO stack
/// per content stream. Each entry remembers the tag (`"P"`, `"H1"`,
/// `"Artifact"`, …), the optional `MCID` for fragment grouping, the
/// optional `/ActualText` substitution string, and a computed
/// `is_artifact` flag that inherits from any ancestor (so nested
/// `/P` inside `/Artifact` is still filtered out).
#[derive(Debug, Clone)]
struct MarkedContentEntry {
    /// The BDC/BMC tag (e.g. `"P"`, `"Figure"`, `"Artifact"`, `"Span"`).
    tag: String,
    /// MCID from `/MCID <int>` if present in the BDC props.
    mcid: Option<u32>,
    /// Decoded ActualText from `/ActualText (...)` if present. Decoded
    /// once at BDC time (UTF-16BE BOM detection in `decode_pdf_string`)
    /// rather than per-fragment.
    actual_text: Option<String>,
    /// True if this entry's tag == `"Artifact"` OR any ancestor on the
    /// stack at push time had `is_artifact == true`. Inheritance lets the
    /// emitter check only the innermost entry to decide filtering.
    is_artifact: bool,
}

/// A pending ActualText run. Created when a BDC pushes an entry with
/// `actual_text == Some(_)`; drained and emitted as a single synthetic
/// `TextFragment` when the matching EMC pops the entry.
///
/// Spec §3a/§4 (collapse-on-EMC): per-`Tj` emission inside an ActualText
/// scope is suppressed; on scope close we emit one fragment whose `text`
/// is the substitution string, `x`/`y` is the first `Tj` origin, and
/// `width` is the sum of suppressed text widths.
#[derive(Debug, Clone)]
struct PendingActualText {
    /// Substitution text from the BDC's `/ActualText` (already decoded).
    text: String,
    /// Pen origin of the first suppressed `Tj` (page-space).
    first_x: f64,
    /// Same for Y.
    first_y: f64,
    /// Accumulated effective width of suppressed `Tj` runs.
    width: f64,
    /// Effective font size at the time the first `Tj` was suppressed.
    font_size: f64,
    /// Font name + style at first `Tj`. Set on first suppression.
    font_name: Option<String>,
    /// Bold/italic from the font name at first suppression.
    is_bold: bool,
    is_italic: bool,
    /// Fill color at first suppression.
    color: Option<Color>,
    /// Depth in `mc_stack` at which this run was opened. When the entry at
    /// this depth is popped, the pending run is flushed.
    stack_depth: usize,
    /// Whether a `Tj`/`TJ`/`'`/`"` has been observed yet inside the scope.
    /// Until the first one fires, the run has no origin to record.
    populated: bool,
}
```

- [ ] **Step 2: Extend `TextState`**

Replace `oxidize-pdf-core/src/text/extraction.rs:118-176`:

```rust
/// Text extraction state
struct TextState {
    /// Current text matrix
    text_matrix: [f64; 6],
    /// Current text line matrix
    text_line_matrix: [f64; 6],
    /// Current transformation matrix (CTM)
    ctm: [f64; 6],
    /// Text leading (line spacing)
    leading: f64,
    /// Character spacing
    char_space: f64,
    /// Word spacing
    word_space: f64,
    /// Horizontal scaling
    horizontal_scale: f64,
    /// Text rise
    text_rise: f64,
    /// Current font size
    font_size: f64,
    /// Current font name
    font_name: Option<String>,
    /// Render mode (0 = fill, 1 = stroke, etc.)
    render_mode: u8,
    /// Fill color (for text rendering)
    fill_color: Option<Color>,
    /// Graphics state stack for `q`/`Q` operators.
    saved_states: Vec<SavedGraphicsState>,
    /// Marked-content stack (issue #269 Phase 1). Pushed on BMC/BDC,
    /// popped on EMC. Empty on entry to each page.
    mc_stack: Vec<MarkedContentEntry>,
    /// Pending ActualText run if any BDC ancestor declared `/ActualText`.
    /// At most one active run at a time — nested ActualText replaces the
    /// outer (innermost wins, per spec §4).
    pending_actualtext: Option<PendingActualText>,
}

impl Default for TextState {
    fn default() -> Self {
        Self {
            text_matrix: [1.0, 0.0, 0.0, 1.0, 0.0, 0.0],
            text_line_matrix: [1.0, 0.0, 0.0, 1.0, 0.0, 0.0],
            ctm: [1.0, 0.0, 0.0, 1.0, 0.0, 0.0],
            leading: 0.0,
            char_space: 0.0,
            word_space: 0.0,
            horizontal_scale: 100.0,
            text_rise: 0.0,
            font_size: 0.0,
            font_name: None,
            render_mode: 0,
            fill_color: None,
            saved_states: Vec::new(),
            mc_stack: Vec::new(),
            pending_actualtext: None,
        }
    }
}
```

- [ ] **Step 3: Build (allow `dead_code` on new types this commit only)**

We are about to introduce unused-field warnings: `MarkedContentEntry::tag`, `mcid`, `actual_text`, `is_artifact` and most `PendingActualText` fields. We treat warnings as errors. Solution: annotate both structs with `#[allow(dead_code)]` for this commit only — Task 7 removes the allow when it actually reads these fields.

Add `#[allow(dead_code)]` above each struct declaration:

```rust
#[allow(dead_code)] // Fields wired in Task 7 (BDC/EMC handler).
#[derive(Debug, Clone)]
struct MarkedContentEntry { ... }

#[allow(dead_code)]
#[derive(Debug, Clone)]
struct PendingActualText { ... }
```

- [ ] **Step 4: Build + lib tests**

Run: `cargo build -p oxidize-pdf 2>&1 | tail -10` then `cargo test --lib -p oxidize-pdf 2>&1 | tail -5`.
Expected: green, no warnings.

- [ ] **Step 5: Commit**

```bash
git add oxidize-pdf-core/src/text/extraction.rs
git commit -m "feat(text-extract): scaffold mc_stack + ActualText state in TextState (addresses #269)"
```

---

## Task 7: Implement `decode_pdf_string` + `resolve_props` helpers (no callers yet)

**Files:**
- Modify: `oxidize-pdf-core/src/text/extraction.rs` — add helpers near the existing `text_origin`/`multiply_matrix` helpers around line 1213.

- [ ] **Step 1: Write failing unit tests for `decode_pdf_string`**

Add at the bottom of `oxidize-pdf-core/src/text/extraction.rs` inside the existing `#[cfg(test)] mod tests { ... }` block:

```rust
    #[test]
    fn decode_pdf_string_utf16be_bom_decodes_fi_ligature() {
        let bytes = [0xFE, 0xFF, 0x00, 0x66, 0x00, 0x69];
        assert_eq!(super::decode_pdf_string(&bytes), "fi");
    }

    #[test]
    fn decode_pdf_string_ascii_pdfdocencoding_passthrough() {
        let bytes = b"page 12";
        assert_eq!(super::decode_pdf_string(bytes), "page 12");
    }

    #[test]
    fn decode_pdf_string_empty_input_returns_empty() {
        assert_eq!(super::decode_pdf_string(&[]), "");
    }

    #[test]
    fn decode_pdf_string_lone_bom_returns_empty() {
        // BOM only, no code units after.
        assert_eq!(super::decode_pdf_string(&[0xFE, 0xFF]), "");
    }
```

- [ ] **Step 2: Run them to verify they fail (fn does not exist)**

Run: `cargo test --lib -p oxidize-pdf -- decode_pdf_string 2>&1 | tail -10`
Expected: build error — `cannot find function decode_pdf_string in this scope`.

- [ ] **Step 3: Implement `decode_pdf_string`**

Add near line 1213 in `oxidize-pdf-core/src/text/extraction.rs` (next to `text_origin`):

```rust
/// Decode a PDF string operand into Rust `String`.
///
/// PDF strings inside marked-content properties (notably `/ActualText`)
/// may be encoded as:
///
/// - **UTF-16BE with BOM**: leading `0xFE 0xFF`, then big-endian 16-bit
///   code units. This is the canonical encoding for non-ASCII ActualText
///   (e.g. `fi` ligature, Greek/math symbols). Decoded via `String::from_utf16_lossy`
///   so invalid surrogate pairs become `U+FFFD` rather than panicking.
/// - **PDFDocEncoding** (the catch-all for non-BOM bytes). For the ASCII
///   subset (0x20-0x7E) PDFDocEncoding is identical to Latin-1. We
///   conservatively map byte-by-byte to `char`. A future revision can
///   plug in the full PDFDocEncoding table if a real PDF emerges with
///   high-bit characters in ActualText *without* a UTF-16BE BOM (rare;
///   most producers emit the BOM when going outside ASCII).
fn decode_pdf_string(bytes: &[u8]) -> String {
    if bytes.len() >= 2 && bytes[0] == 0xFE && bytes[1] == 0xFF {
        let mut code_units: Vec<u16> = Vec::with_capacity((bytes.len() - 2) / 2);
        let mut i = 2;
        while i + 1 < bytes.len() {
            code_units.push(u16::from_be_bytes([bytes[i], bytes[i + 1]]));
            i += 2;
        }
        String::from_utf16_lossy(&code_units)
    } else {
        bytes.iter().map(|&b| b as char).collect()
    }
}
```

- [ ] **Step 4: Run the unit tests — expect pass**

Run: `cargo test --lib -p oxidize-pdf -- decode_pdf_string 2>&1 | tail -10`
Expected: 4 tests, 0 failures.

- [ ] **Step 5: Write failing test for `resolve_props`**

Add to the `#[cfg(test)] mod tests` block:

```rust
    use crate::parser::content::{MarkedContentProps, MarkedContentValue};
    use std::collections::HashMap;

    #[test]
    fn resolve_props_extracts_integer_mcid() {
        let mut map = HashMap::new();
        map.insert("MCID".to_string(), MarkedContentValue::Integer(7));
        let props = MarkedContentProps::Inline(map);

        let (mcid, actual) = super::resolve_props(&props, None);
        assert_eq!(mcid, Some(7));
        assert_eq!(actual, None);
    }

    #[test]
    fn resolve_props_decodes_utf16be_actualtext() {
        let mut map = HashMap::new();
        map.insert(
            "ActualText".to_string(),
            MarkedContentValue::String(vec![0xFE, 0xFF, 0x00, 0x66, 0x00, 0x69]),
        );
        let props = MarkedContentProps::Inline(map);

        let (mcid, actual) = super::resolve_props(&props, None);
        assert_eq!(mcid, None);
        assert_eq!(actual.as_deref(), Some("fi"));
    }

    #[test]
    fn resolve_props_returns_none_for_unresolvable_resource_ref() {
        // No `Properties` available -> graceful (None, None), no panic.
        let props = MarkedContentProps::ResourceRef("PropsName".to_string());
        let (mcid, actual) = super::resolve_props(&props, None);
        assert_eq!((mcid, actual), (None, None));
    }

    #[test]
    fn resolve_props_negative_mcid_rejected() {
        // MCID is unsigned per ISO 32000-1; negative integer is malformed.
        // Return None rather than panicking on the u32 cast.
        let mut map = HashMap::new();
        map.insert("MCID".to_string(), MarkedContentValue::Integer(-1));
        let props = MarkedContentProps::Inline(map);

        let (mcid, _) = super::resolve_props(&props, None);
        assert_eq!(mcid, None);
    }
```

Note: `super::resolve_props` requires the fn to be at module level (not in an impl). The `Properties` lookup parameter is typed as `Option<&PdfDictionary>` so unit tests can pass `None`.

- [ ] **Step 6: Run failing tests**

Run: `cargo test --lib -p oxidize-pdf -- resolve_props 2>&1 | tail -10`
Expected: build error — `cannot find function resolve_props`.

- [ ] **Step 7: Implement `resolve_props`**

Add adjacent to `decode_pdf_string`:

```rust
/// Resolve a `MarkedContentProps` to `(mcid, actual_text)`.
///
/// For `Inline` props, walk the map: `/MCID` (Integer, must fit in `u32`)
/// becomes `mcid`; `/ActualText` (String) is decoded via `decode_pdf_string`.
///
/// For `ResourceRef(name)`, look up `properties.get(name)`. If found and
/// it's a Dictionary, extract `/MCID` and `/ActualText` from there. If
/// not found (or the named entry is not a dict), return `(None, None)`
/// and rely on the caller's logging path — a malformed reference must not
/// abort extraction.
///
/// Per spec §3 / §4: this fn is *not* responsible for surfacing the tag
/// itself (the BDC/BMC operator already supplies the tag); only for the
/// properties dictionary's `/MCID` and `/ActualText`.
fn resolve_props(
    props: &crate::parser::content::MarkedContentProps,
    properties: Option<&crate::parser::objects::PdfDictionary>,
) -> (Option<u32>, Option<String>) {
    use crate::parser::content::{MarkedContentProps, MarkedContentValue};

    let map_mcid_actual = |map: &std::collections::HashMap<String, MarkedContentValue>| {
        let mcid = match map.get("MCID") {
            Some(MarkedContentValue::Integer(n)) if *n >= 0 && *n <= u32::MAX as i64 => {
                Some(*n as u32)
            }
            _ => None,
        };
        let actual = match map.get("ActualText") {
            Some(MarkedContentValue::String(bytes)) => Some(decode_pdf_string(bytes)),
            _ => None,
        };
        (mcid, actual)
    };

    match props {
        MarkedContentProps::Inline(map) => map_mcid_actual(map),
        MarkedContentProps::ResourceRef(name) => {
            let Some(properties) = properties else {
                return (None, None);
            };
            let Some(entry) = properties.get(name) else {
                return (None, None);
            };
            let crate::parser::objects::PdfObject::Dictionary(dict) = entry else {
                return (None, None);
            };
            // Read MCID and ActualText directly off PdfDictionary.
            let mcid = dict.get("MCID").and_then(|o| match o {
                crate::parser::objects::PdfObject::Integer(n) if *n >= 0 => Some(*n as u32),
                _ => None,
            });
            let actual_text = dict.get("ActualText").and_then(|o| match o {
                crate::parser::objects::PdfObject::String(s) => {
                    Some(decode_pdf_string(s.as_bytes()))
                }
                _ => None,
            });
            (mcid, actual_text)
        }
    }
}
```

**Verify before-step:** confirm the import paths used above match real names in the crate. Run:

```bash
grep -n "pub struct PdfDictionary\|pub enum PdfObject\|impl PdfDictionary" oxidize-pdf-core/src/parser/objects.rs | head
grep -n "PdfString\|String(.*Vec<u8>)\|String(.*PdfString)" oxidize-pdf-core/src/parser/objects.rs | head
```

Expected: there is a `PdfObject::String(PdfString)` variant whose inner `PdfString` exposes `as_bytes() -> &[u8]`. If the exact API differs (e.g. `PdfObject::String(Vec<u8>)` directly), adjust the match arm — read 10 lines around the variant to confirm. Update the code in this step to match.

- [ ] **Step 8: Run the new tests — expect pass**

Run: `cargo test --lib -p oxidize-pdf -- resolve_props 2>&1 | tail -10`
Expected: 4 tests, 0 failures.

- [ ] **Step 9: Clippy**

Run: `cargo clippy --lib --tests -p oxidize-pdf -- -D warnings 2>&1 | tail -10`
Expected: clean.

- [ ] **Step 10: Commit**

```bash
git add oxidize-pdf-core/src/text/extraction.rs
git commit -m "feat(text-extract): decode_pdf_string + resolve_props helpers (addresses #269)"
```

---

## Task 8: Handle `BeginMarkedContent` / `BeginMarkedContentWithProps` / `EndMarkedContent` in `extract_from_page`

**Files:**
- Modify: `oxidize-pdf-core/src/text/extraction.rs` — add three `match op` arms inside `extract_from_page` (around the existing arms near line 454-700); pass page resources through.

- [ ] **Step 1: Write the failing nested-BDC extraction test**

Create `oxidize-pdf-core/tests/extraction_mcid_test.rs`:

```rust
//! Issue #269 Phase 1 — `TextFragment.mcid` and `struct_tag` carry the
//! innermost BDC ancestor's identity; nested BDCs are resolved to the
//! innermost MCID-bearing entry.

#[path = "common/mod.rs"]
mod common;
use common::synthetic_pdf::build_pdf_with_content_stream;

use oxidize_pdf::parser::{PdfDocument, PdfReader};
use oxidize_pdf::text::{ExtractionOptions, TextExtractor};
use std::io::Cursor;

fn extract_fragments(content: &[u8], options: ExtractionOptions) -> Vec<oxidize_pdf::text::TextFragment> {
    let pdf = build_pdf_with_content_stream(content);
    let reader = PdfReader::new(Cursor::new(pdf)).expect("reader");
    let document = PdfDocument::new(reader);
    let mut extractor = TextExtractor::with_options(options);
    let extracted = extractor
        .extract_from_page(&document, 0)
        .expect("extract page 0");
    extracted.fragments
}

#[test]
fn nested_bdc_innermost_mcid_and_tag_win() {
    // /P <</MCID 0>> BDC /Span BDC (x) Tj EMC EMC
    // Expected: fragment.mcid = 0 (from /P; /Span has no MCID), struct_tag = "P".
    let content = b"BT\n/F1 12 Tf\n100 700 Td\n\
                    /P << /MCID 0 >> BDC\n\
                    /Span BMC\n\
                    (x) Tj\n\
                    EMC\n\
                    EMC\n\
                    ET\n";
    let mut opts = ExtractionOptions::default();
    opts.preserve_layout = true;
    let frags = extract_fragments(content, opts);

    let frag = frags
        .iter()
        .find(|f| f.text == "x")
        .expect("fragment for 'x' present");
    assert_eq!(frag.mcid, Some(0));
    assert_eq!(frag.struct_tag.as_deref(), Some("P"));
}

#[test]
fn overlaid_baselines_distinct_lines_when_mcid_differs() {
    // Two BDC blocks at the same Y (700 pt) but different MCIDs.
    // Before Phase 1: merge_into_lines would interleave them.
    // After Phase 1: two distinct lines.
    let content = b"BT\n/F1 12 Tf\n\
                    /P << /MCID 0 >> BDC\n\
                    100 700 Td (Hello) Tj\n\
                    EMC\n\
                    /P << /MCID 1 >> BDC\n\
                    1 0 0 1 200 700 Tm (World) Tj\n\
                    EMC\n\
                    ET\n";
    let mut opts = ExtractionOptions::default();
    opts.preserve_layout = true;
    opts.reconstruct_paragraphs = true;
    let frags = extract_fragments(content, opts);

    // After reconstruct_paragraphs, the two MCID groups must remain on
    // distinct paragraph fragments (since they have different mcid keys
    // even though Y_bucket is identical).
    let texts: Vec<&str> = frags.iter().map(|f| f.text.as_str()).collect();
    assert!(
        texts.iter().any(|t| *t == "Hello"),
        "MCID 0 fragment 'Hello' must survive as its own group; got {:?}",
        texts
    );
    assert!(
        texts.iter().any(|t| *t == "World"),
        "MCID 1 fragment 'World' must survive as its own group; got {:?}",
        texts
    );
    // Critical: nothing got cross-mcid-interleaved. The pre-Phase-1 bug
    // produced "HWoerllldo" or "Hello World" merged on one line.
    assert!(
        !texts.iter().any(|t| t.contains("HW") || t.contains("ld o")),
        "fragments must not be merged across mcid boundaries; got {:?}",
        texts
    );
}
```

- [ ] **Step 2: Verify the tests fail (mc_stack not yet read)**

Run: `cargo test --test extraction_mcid_test -p oxidize-pdf 2>&1 | tail -20`
Expected: 2 tests, 2 failures. First test fails because `frag.mcid` is `None` (Task 4 default). Second test may pass-by-accident on current main due to existing baseline-tolerance (#265 fix), so the *interesting* failure is the first.

- [ ] **Step 3: Pass page resources into the operations loop**

In `oxidize-pdf-core/src/text/extraction.rs::extract_from_page` (line ~394), bind the page resources before the operations loop:

```rust
// (near line 420, after streams are obtained)
let page_properties: Option<&crate::parser::objects::PdfDictionary> = page
    .get_resources()
    .and_then(|res| match res.get("Properties") {
        Some(crate::parser::objects::PdfObject::Dictionary(d)) => Some(d),
        _ => None,
    });
```

Bind it as `Option<&PdfDictionary>` whose lifetime borrows from `page` (already in scope through the iteration). If borrow-checker complains because `page.get_resources()` returns a transient reference, materialise the lookup eagerly into a `HashMap<String, (Option<u32>, Option<String>)>` keyed by resource-property name and pass that into the BDC handler — but try the direct borrow first; the existing code already borrows `page.dict` for the duration of the loop (line 1011).

- [ ] **Step 4: Add the three match arms in the `for op in operations` loop**

Inside the existing match block in `extract_from_page` (~line 454-700), at the bottom before the final `_` catch-all (or near other MC-related arms), insert:

```rust
ContentOperation::BeginMarkedContent(tag) => {
    let parent_artifact = state.mc_stack.last().is_some_and(|e| e.is_artifact);
    state.mc_stack.push(MarkedContentEntry {
        is_artifact: tag == "Artifact" || parent_artifact,
        tag,
        mcid: None,
        actual_text: None,
    });
}

ContentOperation::BeginMarkedContentWithProps(tag, props) => {
    let parent_artifact = state.mc_stack.last().is_some_and(|e| e.is_artifact);
    let (mcid, actual_text) = resolve_props(&props, page_properties);

    // If this scope declares ActualText, open a pending run that will be
    // flushed on the matching EMC. Suppresses per-Tj emission inside the
    // scope (innermost-ActualText-wins per spec §4).
    if let Some(ref text) = actual_text {
        state.pending_actualtext = Some(PendingActualText {
            text: text.clone(),
            first_x: 0.0,
            first_y: 0.0,
            width: 0.0,
            font_size: state.font_size,
            font_name: state.font_name.clone(),
            is_bold: false,  // overwritten on first Tj (we don't know font yet)
            is_italic: false,
            color: state.fill_color,
            stack_depth: state.mc_stack.len(), // BEFORE the push below
            populated: false,
        });
    }

    state.mc_stack.push(MarkedContentEntry {
        is_artifact: tag == "Artifact" || parent_artifact,
        tag,
        mcid,
        actual_text,
    });
}

ContentOperation::EndMarkedContent => {
    let popped_depth = state.mc_stack.len();
    if state.mc_stack.pop().is_none() {
        // Unbalanced EMC — log and ignore. Real PDFs occasionally emit
        // dangling EMC (e.g. from incremental updates). We must not panic.
        tracing::debug!(
            "extraction: EMC with empty marked-content stack on page {}",
            page_index + 1
        );
    } else if let Some(pending) = state.pending_actualtext.as_ref() {
        // If we just closed the scope that opened the pending run, flush it.
        if pending.stack_depth + 1 == popped_depth {
            let run = state.pending_actualtext.take().unwrap();
            if run.populated && self.options.preserve_layout {
                let (mcid, struct_tag) = innermost_mc_tag(&state.mc_stack);
                let in_artifact = state.mc_stack.iter().any(|e| e.is_artifact);
                if !in_artifact || self.options.include_artifacts {
                    fragments.push(TextFragment {
                        text: run.text,
                        x: run.first_x,
                        y: run.first_y,
                        width: run.width,
                        height: run.font_size,
                        font_size: run.font_size,
                        font_name: run.font_name,
                        is_bold: run.is_bold,
                        is_italic: run.is_italic,
                        color: run.color,
                        space_decisions: Vec::new(),
                        mcid,
                        struct_tag,
                    });
                }
            }
        }
    }
}
```

Helper `innermost_mc_tag` lives at module level:

```rust
/// Walk the stack from innermost (top) outward, returning the first entry's
/// `(mcid, tag)` pair where `mcid` is `Some`. Returns `(None, None)` when no
/// ancestor declared an MCID — typical of non-tagged PDFs.
fn innermost_mc_tag(stack: &[MarkedContentEntry]) -> (Option<u32>, Option<String>) {
    stack
        .iter()
        .rev()
        .find(|e| e.mcid.is_some())
        .map_or((None, None), |e| (e.mcid, Some(e.tag.clone())))
}
```

- [ ] **Step 5: Drop the `#[allow(dead_code)]` from Task 6**

Remove the two `#[allow(dead_code)]` annotations introduced in Task 6 from `MarkedContentEntry` and `PendingActualText`. The fields are now consumed.

- [ ] **Step 6: Build**

Run: `cargo build -p oxidize-pdf 2>&1 | tail -30`
Expected: clean (or borrow-checker errors around `page_properties` — see Step 3 fallback).

- [ ] **Step 7: Run the new tests**

Run: `cargo test --test extraction_mcid_test -p oxidize-pdf 2>&1 | tail -20`
Expected at this point: `nested_bdc_innermost_mcid_and_tag_win` passes (mcid + struct_tag wired). The overlay test still fails — `merge_into_lines` does not yet group by mcid. We address that in Task 11.

- [ ] **Step 8: Lib tests + clippy**

```bash
cargo test --lib -p oxidize-pdf 2>&1 | tail -5
cargo clippy --lib --tests -p oxidize-pdf -- -D warnings 2>&1 | tail -10
```

Expected: green, clean.

- [ ] **Step 9: Commit**

```bash
git add oxidize-pdf-core/src/text/extraction.rs oxidize-pdf-core/tests/extraction_mcid_test.rs
git commit -m "feat(text-extract): consume BDC/BMC/EMC + tag fragments with mcid (addresses #269)"
```

---

## Task 9: Wire `mcid` + `struct_tag` into per-`Tj` emission, suppress under ActualText, filter Artifact

**Files:**
- Modify: `oxidize-pdf-core/src/text/extraction.rs::emit_text_fragment` (lines ~1161-1202) and the four call sites (`ShowText`, `ShowTextArray::Text`, `NextLineShowText`, `SetSpacingNextLineShowText`) at lines ~527, 570, ~620, ~660.

- [ ] **Step 1: Write failing test for Artifact filter (default off)**

Create `oxidize-pdf-core/tests/extraction_artifact_test.rs`:

```rust
//! Issue #269 Phase 1 — `/Artifact` content filtered by default.

#[path = "common/mod.rs"]
mod common;
use common::synthetic_pdf::build_pdf_with_content_stream;

use oxidize_pdf::parser::{PdfDocument, PdfReader};
use oxidize_pdf::text::{ExtractionOptions, TextExtractor};
use std::io::Cursor;

fn extract(content: &[u8], include_artifacts: bool) -> Vec<oxidize_pdf::text::TextFragment> {
    let pdf = build_pdf_with_content_stream(content);
    let reader = PdfReader::new(Cursor::new(pdf)).expect("reader");
    let document = PdfDocument::new(reader);
    let mut opts = ExtractionOptions::default();
    opts.preserve_layout = true;
    opts.include_artifacts = include_artifacts;
    let mut extractor = TextExtractor::with_options(opts);
    extractor
        .extract_from_page(&document, 0)
        .expect("extract")
        .fragments
}

#[test]
fn artifact_content_filtered_by_default() {
    let content = b"BT\n/F1 12 Tf\n100 700 Td\n\
                    /Artifact BMC\n\
                    (page 12) Tj\n\
                    EMC\n\
                    ET\n";
    let frags = extract(content, false);
    assert!(
        frags.iter().all(|f| !f.text.contains("page 12")),
        "Artifact content must be filtered with default options; got {:?}",
        frags.iter().map(|f| &f.text).collect::<Vec<_>>()
    );
    assert!(
        frags.is_empty(),
        "no other fragments expected; got {:?}",
        frags.iter().map(|f| &f.text).collect::<Vec<_>>()
    );
}

#[test]
fn artifact_content_extracted_when_opted_in() {
    let content = b"BT\n/F1 12 Tf\n100 700 Td\n\
                    /Artifact BMC\n\
                    (page 12) Tj\n\
                    EMC\n\
                    ET\n";
    let frags = extract(content, true);
    let texts: Vec<&str> = frags.iter().map(|f| f.text.as_str()).collect();
    assert!(
        texts.iter().any(|t| *t == "page 12"),
        "with include_artifacts=true, 'page 12' must be present; got {:?}",
        texts
    );
}

#[test]
fn nested_artifact_inherited_by_descendants() {
    // /Artifact BMC /P BMC (x) Tj EMC EMC
    // Inner /P must inherit is_artifact=true and be filtered.
    let content = b"BT\n/F1 12 Tf\n100 700 Td\n\
                    /Artifact BMC\n\
                    /P BMC\n\
                    (x) Tj\n\
                    EMC\n\
                    EMC\n\
                    ET\n";
    let frags = extract(content, false);
    assert!(frags.is_empty(), "nested Artifact must inherit filtering");
}
```

- [ ] **Step 2: Run failing tests**

Run: `cargo test --test extraction_artifact_test -p oxidize-pdf 2>&1 | tail -20`
Expected: 2 of 3 fail (the opt-in test passes accidentally because filtering isn't wired yet; the other two fail because `(page 12)` is emitted).

- [ ] **Step 3: Update `emit_text_fragment` signature + body**

Replace `oxidize-pdf-core/src/text/extraction.rs:1161-1202`:

```rust
/// Emit a `TextFragment` for one decoded text-show event under `preserve_layout`.
///
/// Skips emission when an ancestor in the marked-content stack is `/Artifact`
/// and `options.include_artifacts` is false. When a pending ActualText run is
/// active in the current scope, accumulates the text-width contribution and
/// records the first origin instead of pushing a fragment (the run is flushed
/// once on EMC, see Task 8's EndMarkedContent handler).
///
/// `mcid` and `struct_tag` come from the innermost ancestor on the stack that
/// declared `/MCID`; non-tagged content leaves both as `None`.
fn emit_text_fragment(
    fragments: &mut Vec<TextFragment>,
    decoded: &str,
    text_width: f64,
    x: f64,
    y: f64,
    state: &mut TextState,
    include_artifacts: bool,
) {
    if decoded.is_empty() {
        return;
    }

    // Artifact filter (default: skip emission for Artifact subtrees).
    if !include_artifacts && state.mc_stack.iter().any(|e| e.is_artifact) {
        return;
    }

    let (is_bold, is_italic) = state
        .font_name
        .as_ref()
        .map(|name| parse_font_style(name))
        .unwrap_or((false, false));

    let combined = multiply_matrix(&state.text_matrix, &state.ctm);
    let x_scale = (combined[0] * combined[0] + combined[1] * combined[1]).sqrt();
    let y_scale = (combined[2] * combined[2] + combined[3] * combined[3]).sqrt();
    let effective_width = text_width * x_scale;
    let effective_size = state.font_size * y_scale;

    // If a pending ActualText run is active in the current scope, accumulate
    // into it instead of emitting a fragment now.
    if let Some(pending) = state.pending_actualtext.as_mut() {
        if !pending.populated {
            pending.first_x = x;
            pending.first_y = y;
            pending.font_size = effective_size;
            pending.font_name = state.font_name.clone();
            pending.is_bold = is_bold;
            pending.is_italic = is_italic;
            pending.color = state.fill_color;
            pending.populated = true;
        }
        pending.width += effective_width;
        return;
    }

    let (mcid, struct_tag) = innermost_mc_tag(&state.mc_stack);

    fragments.push(TextFragment {
        text: decoded.to_owned(),
        x,
        y,
        width: effective_width,
        height: effective_size,
        font_size: effective_size,
        font_name: state.font_name.clone(),
        is_bold,
        is_italic,
        color: state.fill_color,
        space_decisions: Vec::new(),
        mcid,
        struct_tag,
    });
}
```

Signature changed: `state: &TextState` → `state: &mut TextState`, and `include_artifacts: bool` added.

- [ ] **Step 4: Update all four call sites in `extract_from_page`**

There are four call sites (already enumerated by the issue #235 work). Each currently passes `&state`; update to `&mut state` and add the options flag:

```rust
// Pattern at each call site:
emit_text_fragment(
    &mut fragments,
    &decoded,
    text_width,
    x,
    y,
    &mut state,
    self.options.include_artifacts,
);
```

Locate them with:

```bash
grep -n "emit_text_fragment(" oxidize-pdf-core/src/text/extraction.rs
```

Expected: 5 matches (definition + 4 call sites at ~527, ~570, ~620, ~660). Update each.

- [ ] **Step 5: Build**

Run: `cargo build -p oxidize-pdf 2>&1 | tail -10`
Expected: clean. If borrow-checker complains about `&mut state` while another field is borrowed, restructure to drop the conflicting borrow before the call (or copy the conflicting field by value first).

- [ ] **Step 6: Run the new artifact tests**

Run: `cargo test --test extraction_artifact_test -p oxidize-pdf 2>&1 | tail -10`
Expected: 3 tests, 0 failures.

- [ ] **Step 7: Run the previously-written mcid tests**

Run: `cargo test --test extraction_mcid_test -p oxidize-pdf 2>&1 | tail -10`
Expected: `nested_bdc_innermost_mcid_and_tag_win` still passes. Overlay test may still fail (merge grouping fixed in Task 11).

- [ ] **Step 8: Lib tests + clippy**

```bash
cargo test --lib -p oxidize-pdf 2>&1 | tail -5
cargo clippy --lib --tests -p oxidize-pdf -- -D warnings 2>&1 | tail -10
```

Expected: green.

- [ ] **Step 9: Commit**

```bash
git add oxidize-pdf-core/src/text/extraction.rs oxidize-pdf-core/tests/extraction_artifact_test.rs
git commit -m "feat(text-extract): wire mcid/struct_tag + Artifact filter into emission (addresses #269)"
```

---

## Task 10: ActualText override (literal and UTF-16BE) + multi-`Tj` run collapsing

**Files:**
- Modify: `oxidize-pdf-core/src/text/extraction.rs` — verified working from Tasks 8 + 9. This task only adds tests; if any fail, fix in this commit.

- [ ] **Step 1: Write the three ActualText tests**

Create `oxidize-pdf-core/tests/extraction_actualtext_test.rs`:

```rust
//! Issue #269 Phase 1 — `/ActualText` overrides decoded glyphs at the
//! BDC scope level, with UTF-16BE support and multi-`Tj` collapsing.

#[path = "common/mod.rs"]
mod common;
use common::synthetic_pdf::build_pdf_with_content_stream;

use oxidize_pdf::parser::{PdfDocument, PdfReader};
use oxidize_pdf::text::{ExtractionOptions, TextExtractor};
use std::io::Cursor;

fn extract(content: &[u8]) -> Vec<oxidize_pdf::text::TextFragment> {
    let pdf = build_pdf_with_content_stream(content);
    let reader = PdfReader::new(Cursor::new(pdf)).expect("reader");
    let document = PdfDocument::new(reader);
    let mut opts = ExtractionOptions::default();
    opts.preserve_layout = true;
    let mut extractor = TextExtractor::with_options(opts);
    extractor
        .extract_from_page(&document, 0)
        .expect("extract")
        .fragments
}

#[test]
fn literal_actualtext_overrides_decoded_glyphs() {
    // /Span <</ActualText (fi)>> BDC (xy) Tj EMC -> single fragment "fi"
    let content = b"BT\n/F1 12 Tf\n100 700 Td\n\
                    /Span << /ActualText (fi) >> BDC\n\
                    (xy) Tj\n\
                    EMC\n\
                    ET\n";
    let frags = extract(content);
    let texts: Vec<&str> = frags.iter().map(|f| f.text.as_str()).collect();
    assert!(
        texts.iter().any(|t| *t == "fi"),
        "fragment must be ActualText 'fi', not glyph 'xy'; got {:?}",
        texts
    );
    assert!(
        !texts.iter().any(|t| *t == "xy"),
        "raw glyph 'xy' must not be emitted under ActualText scope"
    );
}

#[test]
fn utf16be_actualtext_overrides_decoded_glyphs() {
    // ActualText <FEFF00660069> = UTF-16BE for "fi"
    let content = b"BT\n/F1 12 Tf\n100 700 Td\n\
                    /Span << /ActualText <FEFF00660069> >> BDC\n\
                    (junk) Tj\n\
                    EMC\n\
                    ET\n";
    let frags = extract(content);
    let texts: Vec<&str> = frags.iter().map(|f| f.text.as_str()).collect();
    assert!(
        texts.iter().any(|t| *t == "fi"),
        "UTF-16BE ActualText must decode to 'fi'; got {:?}",
        texts
    );
    assert!(!texts.iter().any(|t| *t == "junk"));
}

#[test]
fn actualtext_collapses_multi_tj_run_to_single_fragment() {
    // Two separate Tj inside one ActualText scope -> one fragment with "ff"
    let content = b"BT\n/F1 12 Tf\n100 700 Td\n\
                    /Span << /ActualText (ff) >> BDC\n\
                    (f) Tj\n\
                    (i) Tj\n\
                    EMC\n\
                    ET\n";
    let frags = extract(content);
    let texts: Vec<&str> = frags.iter().map(|f| f.text.as_str()).collect();
    let ff_count = texts.iter().filter(|t| **t == "ff").count();
    assert_eq!(
        ff_count, 1,
        "expected exactly one 'ff' fragment, got {:?}",
        texts
    );
    assert!(!texts.iter().any(|t| *t == "f"));
    assert!(!texts.iter().any(|t| *t == "i"));
}
```

- [ ] **Step 2: Run them**

Run: `cargo test --test extraction_actualtext_test -p oxidize-pdf 2>&1 | tail -10`
Expected: green if Tasks 8 + 9 implemented correctly. If any test fails, fix in this commit.

Common cause if `literal_actualtext_overrides_decoded_glyphs` fails: the flush in EMC (Task 8 Step 4) is checking `popped_depth - pending.stack_depth != 1`. Re-derive — at BDC push time `pending.stack_depth = stack.len()` (size *before* the push). At EMC pop time `popped_depth = stack.len()` *before* the pop. If `popped_depth == pending.stack_depth + 1` and `stack[pending.stack_depth].actual_text.is_some()`, flush.

- [ ] **Step 3: Lib tests + clippy**

```bash
cargo test --lib -p oxidize-pdf 2>&1 | tail -5
cargo clippy --lib --tests -p oxidize-pdf -- -D warnings 2>&1 | tail -10
```

Expected: green.

- [ ] **Step 4: Commit**

```bash
git add oxidize-pdf-core/tests/extraction_actualtext_test.rs oxidize-pdf-core/src/text/extraction.rs
git commit -m "test(text-extract): ActualText override + multi-Tj run collapse (addresses #269)"
```

(If only the test file changed, the message stays — it locks in coverage for the Task 8+9 behavior.)

---

## Task 11: Add `mcid` to `merge_into_lines` / `merge_close_fragments` / `merge_into_paragraphs` grouping keys

**Files:**
- Modify: `oxidize-pdf-core/src/text/extraction.rs:282-309` (`merge_into_lines`).
- Modify: `oxidize-pdf-core/src/text/extraction.rs:958-994` (`merge_close_fragments`).
- Modify: `oxidize-pdf-core/src/text/extraction.rs:318-375` (`merge_into_paragraphs`).

- [ ] **Step 1: Run the previously-failing overlay test to confirm it still fails**

Run: `cargo test --test extraction_mcid_test -p oxidize-pdf -- overlaid_baselines_distinct_lines_when_mcid_differs --exact --nocapture 2>&1 | tail -20`
Expected: FAIL (or pass-by-coincidence; if it passes already due to other factors, the assertion is still correct after the change).

- [ ] **Step 2: Update `merge_into_lines` group predicate**

Replace the inner `placed = ...` block (lines 292-303) with an mcid-aware predicate:

```rust
let mut lines: Vec<Vec<&TextFragment>> = Vec::new();
for frag in sorted {
    let placed = lines.last_mut().is_some_and(|line| {
        let head = line[0];
        let tol = (head.height.min(frag.height)) * 0.2;
        // Same Y-bucket AND same mcid. `None == None` collapses to legacy
        // single-key grouping for non-tagged PDFs.
        (head.y - frag.y).abs() < tol && head.mcid == frag.mcid
    });
    if placed {
        lines.last_mut().unwrap().push(frag);
    } else {
        lines.push(vec![frag]);
    }
}
```

- [ ] **Step 3: Update `merge_close_fragments` predicate**

In the should_merge expression at line ~973, add `&& current.mcid == fragment.mcid`:

```rust
let should_merge = y_diff < 1.0
    && x_gap >= 0.0
    && x_gap < fragment.font_size * 0.5
    && current.mcid == fragment.mcid;
```

- [ ] **Step 4: Update `merge_into_paragraphs` join predicate**

At line ~337, when joining `current` and `line` into the same paragraph, require equal mcid. Add the early-out next to the existing `gap` check:

```rust
if gap < 0.0 || gap > max_paragraph_gap || current.mcid != line.mcid {
    paragraphs.push(current);
    current = line.clone();
    continue;
}
```

- [ ] **Step 5: Run all marked-content tests**

```bash
cargo test --test extraction_mcid_test --test extraction_actualtext_test --test extraction_artifact_test -p oxidize-pdf 2>&1 | tail -20
```

Expected: all green. `overlaid_baselines_distinct_lines_when_mcid_differs` now passes.

- [ ] **Step 6: Run the existing extraction unit tests to catch regressions**

```bash
cargo test --lib -p oxidize-pdf text::extraction 2>&1 | tail -20
```

Expected: all green. The existing tests `merge_into_lines_groups_same_baseline_fragments`, `merge_into_paragraphs_groups_consecutive_lines`, etc. build fragments with `mcid: None`, so `None == None` keeps them in the same group — no behavior change.

- [ ] **Step 7: Run the issue #265 regression test (line-interleaving on NCSC)**

Locate any test that exists today for #265 baseline-tolerance:

```bash
grep -rn "issue_265\|fn .*interleav\|alphabet" --include="*.rs" oxidize-pdf-core/tests/
```

Run any that exists to make sure we haven't broken its assertion.

- [ ] **Step 8: Workspace test sweep**

Run: `cargo test --workspace --no-fail-fast 2>&1 | tail -20`
Expected: 8500+ tests, 0 failures. No existing test should regress.

- [ ] **Step 9: Clippy**

Run: `cargo clippy --lib --tests -p oxidize-pdf -- -D warnings 2>&1 | tail -10`
Expected: clean.

- [ ] **Step 10: Commit**

```bash
git add oxidize-pdf-core/src/text/extraction.rs
git commit -m "fix(text-extract): mcid-aware line/paragraph grouping (addresses #269 + #265)"
```

---

## Task 12: Defensive tests for unbalanced BDC/EMC (no panic)

**Files:**
- Create: `oxidize-pdf-core/tests/extraction_unbalanced_bdc_test.rs`.

- [ ] **Step 1: Write tests**

```rust
//! Issue #269 Phase 1 — defensive paths for unbalanced marked-content
//! operators. Real PDFs (especially those produced by buggy generators or
//! after incremental updates) sometimes emit dangling EMC or unmatched
//! BDC. The extractor must not panic.

#[path = "common/mod.rs"]
mod common;
use common::synthetic_pdf::build_pdf_with_content_stream;

use oxidize_pdf::parser::{PdfDocument, PdfReader};
use oxidize_pdf::text::{ExtractionOptions, TextExtractor};
use std::io::Cursor;

fn extract(content: &[u8]) -> Vec<oxidize_pdf::text::TextFragment> {
    let pdf = build_pdf_with_content_stream(content);
    let reader = PdfReader::new(Cursor::new(pdf)).expect("reader");
    let document = PdfDocument::new(reader);
    let mut opts = ExtractionOptions::default();
    opts.preserve_layout = true;
    let mut extractor = TextExtractor::with_options(opts);
    extractor
        .extract_from_page(&document, 0)
        .expect("extract")
        .fragments
}

#[test]
fn extra_emc_does_not_panic_and_text_still_extracts() {
    // Three EMCs but only one BDC. Extractor must extract the text and
    // silently drop the extra EMCs.
    let content = b"BT\n/F1 12 Tf\n100 700 Td\n\
                    EMC\n\
                    EMC\n\
                    /P << /MCID 0 >> BDC\n\
                    (hello) Tj\n\
                    EMC\n\
                    EMC\n\
                    ET\n";
    let frags = extract(content);
    let texts: Vec<&str> = frags.iter().map(|f| f.text.as_str()).collect();
    assert!(
        texts.iter().any(|t| *t == "hello"),
        "text must survive extra EMC; got {:?}",
        texts
    );
}

#[test]
fn dangling_bdc_at_eof_does_not_panic_and_text_still_extracts() {
    // BDC with no EMC. Stack is non-empty at end of stream — must not
    // panic, must flush content (even if mcid attribution is degraded).
    let content = b"BT\n/F1 12 Tf\n100 700 Td\n\
                    /P << /MCID 0 >> BDC\n\
                    (hello) Tj\n\
                    ET\n";
    let frags = extract(content);
    let texts: Vec<&str> = frags.iter().map(|f| f.text.as_str()).collect();
    assert!(
        texts.iter().any(|t| *t == "hello"),
        "text under dangling BDC must still extract; got {:?}",
        texts
    );
    // The single fragment must carry mcid=0 (the BDC was opened).
    let f = frags.iter().find(|f| f.text == "hello").unwrap();
    assert_eq!(f.mcid, Some(0));
}
```

- [ ] **Step 2: Run the tests**

Run: `cargo test --test extraction_unbalanced_bdc_test -p oxidize-pdf 2>&1 | tail -10`
Expected: green. Task 8's EMC arm already handles the empty-stack case via `tracing::debug!`. The dangling BDC at EOF is harmless because we never validate stack-empty after the loop — that's the expected behavior.

If the dangling-BDC test fails because we panic when `pending_actualtext` is set and never flushed, decide policy: silently drop the unflushed run is acceptable for Phase 1; defensive vs. correctness, defensive wins. Document the choice in a one-line comment on `pending_actualtext`.

- [ ] **Step 3: Commit**

```bash
git add oxidize-pdf-core/tests/extraction_unbalanced_bdc_test.rs
git commit -m "test(text-extract): defensive unbalanced BDC/EMC paths (addresses #269)"
```

---

## Task 13: Writer↔extractor roundtrip integration test

**Files:**
- Create: `oxidize-pdf-core/tests/marked_content_roundtrip_test.rs`.

**Rationale:** Closes the loop — uses the existing v1.4.0 tagged-PDF writer (`structure::marked_content`, `structure::tagged`) to produce a PDF with two paragraphs at the same baseline but distinct MCIDs, then extracts and confirms the new grouping logic keeps them separate.

- [ ] **Step 1: Inspect the existing writer API**

```bash
grep -n "pub fn begin_marked_content\|pub fn begin_with_mcid\|pub fn end_marked_content\|fn add_text\|pub fn finalize" oxidize-pdf-core/src/structure/marked_content.rs oxidize-pdf-core/src/structure/tagged.rs | head -30
```

Read 20 lines around the most relevant entry points to confirm the call shape. Adjust the test below if names differ.

- [ ] **Step 2: Write the test**

```rust
//! Issue #269 Phase 1 — writer-to-extractor roundtrip.
//!
//! Produces a tagged 1-page PDF using the public v1.4.0 writer API: two
//! paragraphs at identical baseline (Y=700pt) with distinct MCIDs. The
//! extractor must keep them on distinct logical lines (issue-265 root cause
//! for tagged PDFs) and tag each fragment with the corresponding mcid.

use oxidize_pdf::parser::{PdfDocument, PdfReader};
use oxidize_pdf::text::{ExtractionOptions, TextExtractor};
use oxidize_pdf::{Document, Font, Page};
use std::io::Cursor;

#[test]
fn writer_to_extractor_keeps_overlaid_mcid_blocks_distinct() {
    // Build a 1-page tagged PDF with two paragraphs at Y=700.
    let mut doc = Document::new();
    let mut page = Page::a4();

    // (Pseudocode — replace with the real call shape once Step 1 confirms it.)
    //   page.begin_marked_content_with_mcid("P", 0);
    //   page.text().set_font(Font::Helvetica, 12.0).at(72.0, 700.0).write("Hello");
    //   page.end_marked_content();
    //   page.begin_marked_content_with_mcid("P", 1);
    //   page.text().set_font(Font::Helvetica, 12.0).at(300.0, 700.0).write("World");
    //   page.end_marked_content();
    //
    // If the high-level Page API doesn't yet expose BMC/BDC, write directly
    // into the content stream via `page.graphics().raw_op("/P <</MCID 0>> BDC")`
    // or equivalent. Confirm during Step 1.

    doc.add_page(page);
    let bytes = doc.to_bytes().expect("serialize");

    let reader = PdfReader::new(Cursor::new(bytes)).expect("reader");
    let document = PdfDocument::new(reader);
    let mut opts = ExtractionOptions::default();
    opts.preserve_layout = true;
    opts.reconstruct_paragraphs = true;
    let mut extractor = TextExtractor::with_options(opts);
    let extracted = extractor
        .extract_from_page(&document, 0)
        .expect("extract page 0");

    let texts: Vec<&str> = extracted.fragments.iter().map(|f| f.text.as_str()).collect();
    assert!(
        texts.iter().any(|t| *t == "Hello"),
        "'Hello' must survive as its own group; got {:?}",
        texts
    );
    assert!(
        texts.iter().any(|t| *t == "World"),
        "'World' must survive as its own group; got {:?}",
        texts
    );

    // Critical: each fragment carries the writer's mcid.
    let hello = extracted.fragments.iter().find(|f| f.text == "Hello").unwrap();
    let world = extracted.fragments.iter().find(|f| f.text == "World").unwrap();
    assert_eq!(hello.mcid, Some(0));
    assert_eq!(world.mcid, Some(1));
    assert_eq!(hello.struct_tag.as_deref(), Some("P"));
    assert_eq!(world.struct_tag.as_deref(), Some("P"));
}
```

- [ ] **Step 3: Resolve the real writer API**

The pseudocode above must be replaced before the test compiles. Concrete decision tree based on Step 1's grep:

- If `Page::begin_marked_content_with_mcid` (or equivalent) exists: use it directly.
- If only the lower-level `MarkedContentWriter` from `src/structure/marked_content.rs` exists: build the content stream via that, then attach to the page.
- If neither: write raw operator bytes into the page's content stream (last resort — but the writer is documented as v1.4.0 so a path exists).

Update the test code to call the real API and ensure it compiles.

- [ ] **Step 4: Run the test**

Run: `cargo test --test marked_content_roundtrip_test -p oxidize-pdf 2>&1 | tail -20`
Expected: green.

- [ ] **Step 5: Commit**

```bash
git add oxidize-pdf-core/tests/marked_content_roundtrip_test.rs
git commit -m "test(text-extract): writer→extractor roundtrip for tagged paragraphs (addresses #269)"
```

---

## Task 14: NCSC alphabet-soup real-corpus test

**Files:**
- Create: `oxidize-pdf-core/tests/ncsc_no_alphabet_soup_test.rs`.

**Rationale:** Concrete reproducer for the issue motivating Phase 1 — the NCSC Cyber Assessment Framework v4.0 page 12 produced `"Tahre mere iansag neod s efysftecemtaitivecl yp.roc ess"` before this work. After: each interleaved BDC block extracts to a coherent paragraph.

- [ ] **Step 1: Confirm corpus file is present**

```bash
ls -la corpus_cache/e0e3ff11371c09c2.pdf
```

Expected: ~615 KB. If absent, error and stop — the corpus is provided by the `rag_realworld` example and was downloaded in session 2026-05-21 (see roadmap).

- [ ] **Step 2: Write the test**

```rust
//! Issue #269 Phase 1 — NCSC CAF v4.0 page 12 produced alphabet-soup
//! ("Tahre mere iansag…") before marked-content was wired. This test
//! locks in that the user-visible chunks no longer contain the
//! interleaved garbage strings.
//!
//! Corpus file: `corpus_cache/e0e3ff11371c09c2.pdf` (NCSC CAF v4.0,
//! present locally, provided by the `rag_realworld` example).

use oxidize_pdf::parser::{PdfDocument, PdfReader};
use oxidize_pdf::text::{ExtractionOptions, TextExtractor};
use std::path::PathBuf;

fn corpus_path() -> Option<PathBuf> {
    let p = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("corpus_cache")
        .join("e0e3ff11371c09c2.pdf");
    if p.exists() {
        Some(p)
    } else {
        None
    }
}

#[test]
fn ncsc_page_12_extracts_coherent_text_no_alphabet_soup() {
    let path = match corpus_path() {
        Some(p) => p,
        None => {
            // Corpus not present (e.g. CI minimal checkout). Don't fail —
            // skip with an eprintln so dev runs see the gap.
            eprintln!("ncsc_no_alphabet_soup_test: corpus file missing, skipping");
            return;
        }
    };

    let reader = PdfReader::open(&path).expect("open NCSC corpus");
    let document = PdfDocument::new(reader);

    let mut opts = ExtractionOptions::default();
    opts.preserve_layout = true;
    opts.reconstruct_paragraphs = true;
    let mut extractor = TextExtractor::with_options(opts);

    // Page 12 in 1-based numbering = page_index 11.
    let extracted = extractor
        .extract_from_page(&document, 11)
        .expect("extract page 12");

    let full_text = extracted.text.as_str();

    // Negative assertions — the pre-fix garbage substrings must be absent.
    for garbage in &["Tahre", "iansag", "efysftecemtaitivecl", "neod s ef"] {
        assert!(
            !full_text.contains(garbage),
            "page 12 still contains interleaved garbage substring {:?}; \
             extracted text:\n{}",
            garbage,
            full_text
        );
    }

    // Positive assertions — at least one coherent English fragment survives.
    let coherent_hits: Vec<&&str> = ["There", "systems", "Security", "process"]
        .iter()
        .filter(|needle| full_text.contains(*needle))
        .collect();
    assert!(
        !coherent_hits.is_empty(),
        "page 12 must contain at least one coherent English word from the \
         expected set [There, systems, Security, process]; got text:\n{}",
        full_text
    );
}
```

- [ ] **Step 3: Run the test**

Run: `cargo test --test ncsc_no_alphabet_soup_test -p oxidize-pdf -- --nocapture 2>&1 | tail -30`
Expected: green. If it fails, the alphabet-soup is *still* present — either the page uses a marked-content variant we haven't covered, or another bug. Investigate with `cargo run --example rag_realworld 2>&1 | grep -A2 NCSC` (the example processes the same file).

- [ ] **Step 4: Capture extracted-text sample for the PR body**

```bash
cargo test --test ncsc_no_alphabet_soup_test -p oxidize-pdf -- --nocapture 2>&1 | grep -A50 "page 12 must contain" || true
# If green, run a small driver script that prints the first ~10 lines of
# extracted text from page 12 and pipe it to a file for the PR body:
cat > /tmp/ncsc_dump.rs <<'EOF'
// (one-shot inline binary; do not commit)
use oxidize_pdf::parser::{PdfDocument, PdfReader};
use oxidize_pdf::text::{ExtractionOptions, TextExtractor};
fn main() {
    let p = "corpus_cache/e0e3ff11371c09c2.pdf";
    let r = PdfReader::open(p).unwrap();
    let d = PdfDocument::new(r);
    let mut o = ExtractionOptions::default();
    o.preserve_layout = true;
    o.reconstruct_paragraphs = true;
    let mut e = TextExtractor::with_options(o);
    let x = e.extract_from_page(&d, 11).unwrap();
    println!("{}", x.text);
}
EOF
# Run via cargo's example mechanism if convenient, otherwise capture from
# the test's --nocapture output by adding a temporary `println!` and
# REVERTING it before committing. Save the output to PR body assets.
```

Save the captured "before" (pre-fix) text from the spec/roadmap (already documented as "Tahre mere iansag…") and the "after" (this task's output) into a `.private/ncsc_page12_before_after.txt` for the PR description. **Do not commit** `.private/` files (see `feedback_never_commit_private.md`).

- [ ] **Step 5: Commit**

```bash
git add oxidize-pdf-core/tests/ncsc_no_alphabet_soup_test.rs
git commit -m "test(text-extract): NCSC CAF v4.0 page 12 no alphabet-soup (addresses #269)"
```

---

## Task 15: `rag_realworld` regression sweep + chunk count capture

**Files:**
- No code changes. Manual verification only; output goes into the PR body.

**Rationale:** Success criterion #7 — the 5-PDF showcase must continue at 5/5 with chunk counts in the same ballpark. Roadmap baseline pre-Phase-1: 776 chunks total across 5 documents.

- [ ] **Step 1: Run the example and capture chunk counts**

Run with `nice` (per `CLAUDE.md` for heavy tasks):

```bash
nice cargo run --release --example rag_realworld 2>&1 | tee /tmp/rag_realworld_after.log | tail -80
```

Expected output: 5/5 documents processed, chunk counts per document. Roadmap baseline: 776 total chunks. Acceptable band: ±10% per document; failures or regressions to zero chunks are blockers.

- [ ] **Step 2: Diff against the recorded baseline**

The 2026-05-21 roadmap entry records the per-doc baseline ("776 chunks", "Higgs went from `Unknown keyword: Sendstream` to 94 chunks"). Compare the new log against those numbers:

| Doc | Baseline chunks | New chunks | Δ | Verdict |
|---|---|---|---|---|
| BSI | (from baseline log) | (from new log) | | |
| Higgs | 94 | | | |
| ENS | | | | |
| BOE | | | | |
| NCSC | (was 0 / alphabet-soup) | | | Must be >0 and coherent |

If any document regresses to 0 chunks or balloons >2× baseline, treat as a Phase 1 blocker — investigate with `RUST_LOG=oxidize_pdf::text::extraction=debug`.

- [ ] **Step 3: Save the log under `.private/` for the PR body**

```bash
cp /tmp/rag_realworld_after.log .private/rag_realworld_after_phase1.log
```

Do **not** commit. The PR body will copy-paste the table.

- [ ] **Step 4: No commit (manual verification step)**

This task produces no commit. It produces evidence for the PR body.

---

## Task 16: Benchmark delta (text-extraction baseline)

**Files:**
- No code changes; uses existing `criterion` benchmarks (baseline `v2.0.0-profiling` per roadmap).

- [ ] **Step 1: Identify the relevant bench**

```bash
ls oxidize-pdf-core/benches/ 2>/dev/null
grep -rn "text_extract\|text_extraction" --include="*.rs" oxidize-pdf-core/benches/ 2>/dev/null | head
```

Use the `text_extract` / `text_extract_full` benchmark identified in the roadmap performance baseline.

- [ ] **Step 2: Run with the saved `v2.0.0-profiling` baseline**

```bash
nice cargo bench -p oxidize-pdf --bench <bench_file> -- --baseline v2.0.0-profiling 2>&1 | tee /tmp/bench_after.log | tail -60
```

Expected output: criterion reports `change: [-x% +y%]` per bench. The `mc_stack` push/pop adds one branch + one stack op per BDC/EMC; on non-tagged PDFs the stack stays empty so the cost is bounded.

- [ ] **Step 3: Acceptance**

- Non-tagged PDFs: regression ≤5% on `text_extract_full` is acceptable (the spec calls it out as bounded).
- Tagged PDFs: regression up to 15% is acceptable for Phase 1 (the new code paths are not yet optimized; Phase 2-5 add work but also more opportunity for caching).

If the regression exceeds these bands, identify the hot path with `cargo flamegraph` (or `perf record`) and propose a micro-fix in a follow-up commit *before* opening the PR.

- [ ] **Step 4: Save the bench log**

```bash
cp /tmp/bench_after.log .private/bench_phase1_after.log
```

No commit.

---

## Task 17: Open PR to `develop`

**Files:**
- None (gh CLI only).

**Authorization:** Per `CLAUDE.md` "Release Workflow" and the user's standing policy from error-log 2026-05-09, opening a PR to `develop` requires explicit per-turn authorization. **Do not run the `gh pr create` step until the user authorises it in the current turn.**

- [ ] **Step 1: Verify branch is clean and push it**

```bash
git status
git push -u origin fix/issue-269-marked-content-extraction
```

- [ ] **Step 2: Draft the PR body**

The PR body must contain (per `CLAUDE.md` "External Communication" — no `closes`/`fixes`/`resolves` keywords):

```markdown
## Summary
Phase 1 of the Tagged-PDF initiative. Wires `BDC`/`BMC`/`EMC` semantics into the text extractor so that:

- `TextFragment` carries `mcid: Option<u32>` and `struct_tag: Option<String>` from the innermost ancestor BDC with `/MCID`.
- Two overlaid blocks at the same baseline stay on distinct logical lines (`merge_into_lines` group key is now `(Y_bucket, mcid)`).
- `/ActualText` overrides decoded glyphs at BDC scope; UTF-16BE BOM supported.
- `/Artifact` subtrees filtered by default (`ExtractionOptions::include_artifacts = false`).

Addresses #269. Addresses the NCSC alphabet-soup regression flagged in #265.

## Detailed changes
- Parser: typed `MarkedContentValue` / `MarkedContentProps` (replaces lossy `HashMap<String,String>`). Preserves UTF-16BE bytes and Integer MCIDs.
- Extractor: `TextState.mc_stack` + `PendingActualText`; `decode_pdf_string` + `resolve_props`; `emit_text_fragment` now respects Artifact filter + ActualText scope.
- Merge invariants: `merge_into_lines`, `merge_close_fragments`, `merge_into_paragraphs` add `mcid` equality to the grouping/merge predicate.

Non-tagged PDFs are unchanged (`mcid = None` everywhere → `None == None` collapses to legacy grouping).

## Tests (all content-verifying, no smoke tests)
- 3 parser unit (`marked_content_props_test.rs`)
- 2 extract MCID (`extraction_mcid_test.rs`)
- 3 ActualText (`extraction_actualtext_test.rs`)
- 3 Artifact (`extraction_artifact_test.rs`)
- 2 defensive unbalanced (`extraction_unbalanced_bdc_test.rs`)
- 1 writer↔extractor roundtrip (`marked_content_roundtrip_test.rs`)
- 1 NCSC real corpus (`ncsc_no_alphabet_soup_test.rs`)

Plus regression sweep across the existing 8500+ workspace tests: 0 failures.

## NCSC CAF v4.0 page 12 — before / after

Before (snippet from #269 reproduction):
> Tahre mere iansag neod s efysftecemtaitivecl yp.roc ess

After (this PR):
> *(captured from `cargo test --test ncsc_no_alphabet_soup_test -- --nocapture`; paste here)*

## `rag_realworld` chunk counts
*(table from Task 15)*

## Bench delta
*(criterion summary from Task 16; text_extract_full regression: …%)*

## Scope discipline
Phase 1 only. Phase 2 (structure tree), Phase 3 (partitioner classification), Phase 4 (writer ParentTree), Phase 5 (PDF/UA) are explicitly out of scope.
```

- [ ] **Step 3: Save the draft body locally**

Write the body to `.private/pr_269_body.md` (not committed). Do not run `gh pr create` until authorised.

- [ ] **Step 4: AWAIT AUTHORISATION**

Pause for the user to confirm the PR text and authorise opening it.

- [ ] **Step 5: (After authorisation) open the PR**

```bash
gh pr create --base develop --head fix/issue-269-marked-content-extraction \
  --title "fix(text-extract): wire marked-content semantics for tagged PDFs (addresses #269)" \
  --body "$(cat .private/pr_269_body.md)"
```

- [ ] **Step 6: Confirm CI green**

```bash
gh pr checks
```

Expected: T0+T1 + Test ubuntu/macos/windows-stable all PASS.

---

## Self-Review (per writing-plans skill)

**1. Spec coverage:**
- Success #1 (synthetic overlay distinct lines): Task 8 test + Task 11 fix ✓
- Success #2 (literal ActualText override): Task 10 test ✓
- Success #3 (UTF-16BE ActualText): Task 7 (`decode_pdf_string`) + Task 10 test ✓
- Success #4 (Artifact filter default): Task 9 test ✓
- Success #5 (Artifact opt-in): Task 9 test ✓
- Success #6 (NCSC alphabet-soup): Task 14 ✓
- Success #7 (rag_realworld regression): Task 15 ✓
- Success #8 (non-tagged unchanged): Task 11 (`None == None` invariant) + Task 4 (defaulted fields) + workspace test sweep ✓
- Success #9 (`cargo test --workspace` passes): Task 11 Step 8 ✓
- Parser refactor (§3): Task 3 ✓
- Decoding helper (§3a): Task 7 ✓
- Extractor state (§4): Task 6 ✓
- Merge invariants (§5): Task 11 ✓
- Public API additions (§6): Task 4 + Task 5 ✓
- Risks (rollback, perf): Task 16 ✓

**2. Placeholders:** Task 13 contains pseudocode (the writer API call shape) explicitly flagged in Step 1 as requiring resolution. Acceptable because Step 3 in that task forces resolution before the test compiles, but a strict reading of "no placeholders" warns about it. **Action:** the Step 1 grep is the resolution mechanism — treat it as a sub-step that produces concrete code, not a placeholder.

**3. Type consistency:** `MarkedContentValue` / `MarkedContentProps` / `MarkedContentEntry` / `PendingActualText` / `emit_text_fragment` / `innermost_mc_tag` — names used consistently across Tasks 3, 6, 7, 8, 9. `TextFragment.mcid` / `struct_tag` consistent across Tasks 4, 8, 9, 11. `ExtractionOptions.include_artifacts` consistent across Tasks 5, 9.

**4. Numbering:** Task 2 is intentionally absent (folded into Task 3 — explanation embedded). Tasks run 1, 3, 4, …, 17.

---

## Execution Handoff

Plan complete and saved to `docs/superpowers/plans/2026-05-21-marked-content-extraction.md`. Two execution options:

**1. Subagent-Driven (recommended)** — dispatch a fresh subagent per task, review between tasks, fast iteration.

**2. Inline Execution** — execute tasks in this session via `superpowers:executing-plans`, batch with checkpoints for review.

Which approach?
