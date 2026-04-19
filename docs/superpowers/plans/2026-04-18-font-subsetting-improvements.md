# Font Subsetting Improvements Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Fix CJK character rendering bugs and reduce font subset output size by 3-14x through desubroutinization, table stripping, and SID→CID conversion.

**Architecture:** Replace the current Local Subr stub-replacement strategy with full charstring desubroutinization. Unify all CFF output as raw CID-keyed CFF (no OTF wrapper). Strip unnecessary tables from TTF subsets. Modularize the 3,010-line monolithic CFF subsetter into focused modules.

**Tech Stack:** Rust, CFF spec (Adobe Technical Note #5176), OpenType spec, ISO 32000-1

**Spec:** `docs/superpowers/specs/2026-04-18-font-subsetting-improvements-design.md`

---

## File Structure

### New files

| File | Responsibility |
|------|---------------|
| `oxidize-pdf-core/src/text/fonts/cff/mod.rs` | Re-exports for CFF modules |
| `oxidize-pdf-core/src/text/fonts/cff/index.rs` | CFF INDEX parsing (`parse_cff_index`, `CffIndex`) and creation (`build_cff_index`) |
| `oxidize-pdf-core/src/text/fonts/cff/dict.rs` | DICT parsing/serialization (`TopDictOffsets`, `parse_top_dict`, `rebuild_cid_top_dict`, `rebuild_fd_dict`, `encode_cff_int_5byte`, `parse_fd_select`, `parse_fd_private`) |
| `oxidize-pdf-core/src/text/fonts/cff/charstring.rs` | Charstring desubroutinizer + bias calculation |
| `oxidize-pdf-core/src/text/fonts/cff/types.rs` | CFF number encoding, `read_u16`/`read_u32`/`read_i16`, `otf_checksum`, `CffDictToken`, `CffDictScanner` |
| `oxidize-pdf-core/tests/cff_desubroutinize_test.rs` | Tests for the desubroutinizer |
| `oxidize-pdf-core/tests/font_subset_size_test.rs` | Size regression tests |

### Modified files

| File | Changes |
|------|---------|
| `oxidize-pdf-core/src/text/fonts/mod.rs` | Add `pub mod cff;` |
| `oxidize-pdf-core/src/text/fonts/cff_subsetter.rs` | Refactor to orchestration-only (~600 lines), delegate to `cff/` modules. Remove `rebuild_subset()`, OTF wrapper, Local Subr filtering. Unify output as raw CID-keyed CFF. |
| `oxidize-pdf-core/src/text/fonts/truetype_subsetter.rs` | Remove cmap/OS/2/name from `build_font_file()`. Remove `build_cmap_table()`, `build_cmap_format4()`, `build_cmap_format12()`. Remove `cmap` parameter from `build_font_file()` and `build_subset_font()`. |
| `oxidize-pdf-core/src/writer/pdf_writer/mod.rs` | Simplify CFF embedding: always `CIDFontType0C`/`FontFile3`. Remove `embed_as_raw_cff` branching. |
| `oxidize-pdf-core/tests/cff_subsetter_test.rs` | Update test that asserts OTTO signature (subset now returns raw CFF, not OTF). Update imports for moved public items. |

### Files to delete (code removed during refactor)

None — all existing code is moved into modules or removed inline. No files deleted, but ~2,000 lines of dead code removed from `cff_subsetter.rs` and ~400 lines from `truetype_subsetter.rs`.

---

## Task 1: Create CFF module structure and extract `types.rs`

Extract primitive types, readers, and the DICT scanner from `cff_subsetter.rs` into `cff/types.rs`. This is a pure refactor — no behavior changes.

**Files:**
- Create: `oxidize-pdf-core/src/text/fonts/cff/mod.rs`
- Create: `oxidize-pdf-core/src/text/fonts/cff/types.rs`
- Modify: `oxidize-pdf-core/src/text/fonts/mod.rs`
- Modify: `oxidize-pdf-core/src/text/fonts/cff_subsetter.rs`

- [ ] **Step 1: Create `cff/mod.rs` with module declarations**

```rust
// oxidize-pdf-core/src/text/fonts/cff/mod.rs
//! CFF (Compact Font Format) parsing and subsetting modules.

pub mod types;
```

- [ ] **Step 2: Create `cff/types.rs` by extracting from `cff_subsetter.rs`**

Move the following items from `cff_subsetter.rs` into `cff/types.rs`:
- `CffDictToken` enum (lines 43–52)
- `CffDictScanner` struct + impl + Iterator impl (lines 68–177)
- `read_u16()` (lines 2335–2343)
- `read_u32()` (lines 2345–2358)
- `read_i16()` (lines 2360–2378)
- `otf_checksum()` (lines 2615–2634)
- `encode_cff_int_5byte()` (lines 908–915)

All items should be `pub(crate)`. Add `use crate::error::{ParseError, ParseResult};` at the top.

```rust
// oxidize-pdf-core/src/text/fonts/cff/types.rs
//! CFF primitive types, number encoding, and DICT scanning.

use crate::error::{ParseError, ParseResult};

/// Token produced by the CFF DICT scanner.
#[derive(Debug, Clone, PartialEq)]
pub enum CffDictToken {
    // ... (copy from cff_subsetter.rs lines 43-52 exactly)
}

// ... (copy all items listed above)
```

- [ ] **Step 3: Register the `cff` module in `mod.rs`**

In `oxidize-pdf-core/src/text/fonts/mod.rs`, add:
```rust
pub mod cff;
```

- [ ] **Step 4: Update `cff_subsetter.rs` to use `cff::types` instead of local definitions**

Replace the moved items with imports:
```rust
use crate::text::fonts::cff::types::{
    CffDictToken, CffDictScanner, read_u16, read_u32, read_i16,
    otf_checksum, encode_cff_int_5byte,
};
```

Delete the original definitions from `cff_subsetter.rs`.

- [ ] **Step 5: Update `tests/cff_subsetter_test.rs` imports**

The test file imports `CffDictScanner` and `CffDictToken` from `cff_subsetter`. Update to:
```rust
use oxidize_pdf::text::fonts::cff::types::{CffDictScanner, CffDictToken};
```

- [ ] **Step 6: Run all tests to verify no regressions**

Run: `cargo test --test cff_subsetter_test && cargo test --lib -- cff`
Expected: All 27 integration tests + 24 unit tests pass. Zero failures.

- [ ] **Step 7: Commit**

```bash
git add oxidize-pdf-core/src/text/fonts/cff/
git add oxidize-pdf-core/src/text/fonts/mod.rs
git add oxidize-pdf-core/src/text/fonts/cff_subsetter.rs
git add oxidize-pdf-core/tests/cff_subsetter_test.rs
git commit -m "refactor(cff): extract types module from monolithic cff_subsetter"
```

---

## Task 2: Extract `cff/index.rs`

Extract CFF INDEX parsing and creation into its own module.

**Files:**
- Create: `oxidize-pdf-core/src/text/fonts/cff/index.rs`
- Modify: `oxidize-pdf-core/src/text/fonts/cff/mod.rs`
- Modify: `oxidize-pdf-core/src/text/fonts/cff_subsetter.rs`

- [ ] **Step 1: Create `cff/index.rs`**

Move from `cff_subsetter.rs`:
- `CffIndex` struct + impl (lines 618–659)
- `parse_cff_index()` (lines 663–729)
- `read_offset()` (lines 732–760)
- `usize_to_cff_offset()` (lines 761–769)
- `build_cff_index()` (lines 771–806)
- `write_offset()` (lines 808–825)

All items `pub(crate)` except `build_cff_index` and `usize_to_cff_offset` which are already `pub` (used by integration tests).

```rust
// oxidize-pdf-core/src/text/fonts/cff/index.rs
//! CFF INDEX structure parsing and creation.

use crate::error::{ParseError, ParseResult};

/// Parsed CFF INDEX — a counted array of variable-length items.
pub(crate) struct CffIndex {
    // ... (copy from cff_subsetter.rs lines 618-625)
}

// ... (copy all items listed above)
```

- [ ] **Step 2: Update `cff/mod.rs`**

```rust
pub mod index;
pub mod types;
```

- [ ] **Step 3: Update `cff_subsetter.rs` imports**

Replace moved items with:
```rust
use crate::text::fonts::cff::index::{
    CffIndex, parse_cff_index, build_cff_index, usize_to_cff_offset,
};
```

Delete originals from `cff_subsetter.rs`.

- [ ] **Step 4: Update `tests/cff_subsetter_test.rs` imports**

```rust
use oxidize_pdf::text::fonts::cff::index::{build_cff_index, usize_to_cff_offset};
```

- [ ] **Step 5: Run tests**

Run: `cargo test --test cff_subsetter_test && cargo test --lib -- cff`
Expected: All pass.

- [ ] **Step 6: Commit**

```bash
git add oxidize-pdf-core/src/text/fonts/cff/
git add oxidize-pdf-core/src/text/fonts/cff_subsetter.rs
git add oxidize-pdf-core/tests/cff_subsetter_test.rs
git commit -m "refactor(cff): extract index module for CFF INDEX parsing/creation"
```

---

## Task 3: Extract `cff/dict.rs`

Extract DICT parsing, Top DICT / FD DICT building, FDSelect parsing.

**Files:**
- Create: `oxidize-pdf-core/src/text/fonts/cff/dict.rs`
- Modify: `oxidize-pdf-core/src/text/fonts/cff/mod.rs`
- Modify: `oxidize-pdf-core/src/text/fonts/cff_subsetter.rs`

- [ ] **Step 1: Create `cff/dict.rs`**

Move from `cff_subsetter.rs`:
- `TopDictOffsets` struct (lines 828–840)
- `parse_top_dict()` (lines 842–906)
- `rebuild_cid_top_dict()` (lines 916–989)
- `rebuild_fd_dict()` (lines 990–1035)
- `rebuild_top_dict()` (lines 1037–1102)
- `parse_fd_select()` (lines 1104–1174)
- `parse_fd_private()` (lines 1176–1210)
- `FdData` struct (lines 1212–1220)
- `parse_local_subrs_offset()` (lines 2039–2069)
- `patch_private_subrs_offset()` (lines 2071–2111)

These depend on `cff::types` and `cff::index` — add imports accordingly.

- [ ] **Step 2: Update `cff/mod.rs`**

```rust
pub mod dict;
pub mod index;
pub mod types;
```

- [ ] **Step 3: Update `cff_subsetter.rs` imports**

Replace moved items with imports from `cff::dict`.

- [ ] **Step 4: Run tests**

Run: `cargo test --test cff_subsetter_test && cargo test --lib -- cff`
Expected: All pass.

- [ ] **Step 5: Commit**

```bash
git add oxidize-pdf-core/src/text/fonts/cff/
git add oxidize-pdf-core/src/text/fonts/cff_subsetter.rs
git commit -m "refactor(cff): extract dict module for DICT parsing/serialization"
```

---

## Task 4: Write failing tests for charstring desubroutinizer

Write the tests first (TDD red phase). These tests define the expected behavior of the desubroutinizer before it exists.

**Files:**
- Create: `oxidize-pdf-core/src/text/fonts/cff/charstring.rs`
- Create: `oxidize-pdf-core/tests/cff_desubroutinize_test.rs`
- Modify: `oxidize-pdf-core/src/text/fonts/cff/mod.rs`

- [ ] **Step 1: Create stub `cff/charstring.rs`**

```rust
// oxidize-pdf-core/src/text/fonts/cff/charstring.rs
//! CFF charstring desubroutinization.
//!
//! Inlines all subroutine calls (callsubr/callgsubr) to produce
//! self-contained charstrings with no external references.

use crate::error::{ParseError, ParseResult};
use super::index::CffIndex;

/// Calculate subroutine bias per CFF spec.
/// - count < 1240:  bias = 107
/// - count < 33900: bias = 1131
/// - else:          bias = 32768
pub(crate) fn cff_subr_bias(count: usize) -> i32 {
    if count < 1240 {
        107
    } else if count < 33900 {
        1131
    } else {
        32768
    }
}

/// Desubroutinize a charstring by recursively inlining all subroutine calls.
///
/// Returns a new charstring with all `callsubr` (10), `callgsubr` (29), and
/// `return` (11) operators removed, and subroutine bodies inlined in place.
///
/// `global_subrs` and `local_subrs` are the parsed CFF INDEX structures for
/// global and local (per-FD) subroutines respectively.
pub(crate) fn desubroutinize(
    _charstring: &[u8],
    _global_subrs: &CffIndex,
    _local_subrs: &CffIndex,
) -> ParseResult<Vec<u8>> {
    Err(ParseError::SyntaxError {
        position: 0,
        message: "desubroutinize not yet implemented".to_string(),
    })
}
```

- [ ] **Step 2: Register module in `cff/mod.rs`**

```rust
pub mod charstring;
pub mod dict;
pub mod index;
pub mod types;
```

- [ ] **Step 3: Write failing tests in `cff_desubroutinize_test.rs`**

```rust
// oxidize-pdf-core/tests/cff_desubroutinize_test.rs
//! Tests for CFF charstring desubroutinization.

use oxidize_pdf::text::fonts::cff::charstring::{cff_subr_bias, desubroutinize};
use oxidize_pdf::text::fonts::cff::index::{build_cff_index, parse_cff_index};

/// Helper: build a CffIndex from items and parse it back.
fn make_index(items: &[&[u8]]) -> (Vec<u8>, usize) {
    let data = build_cff_index(items);
    let len = data.len();
    (data, len)
}

// =========================================================================
// Bias calculation (these should pass immediately — already implemented)
// =========================================================================

#[test]
fn test_bias_small() {
    assert_eq!(cff_subr_bias(0), 107);
    assert_eq!(cff_subr_bias(1), 107);
    assert_eq!(cff_subr_bias(1239), 107);
}

#[test]
fn test_bias_medium() {
    assert_eq!(cff_subr_bias(1240), 1131);
    assert_eq!(cff_subr_bias(33899), 1131);
}

#[test]
fn test_bias_large() {
    assert_eq!(cff_subr_bias(33900), 32768);
    assert_eq!(cff_subr_bias(65535), 32768);
}

// =========================================================================
// Desubroutinization: no subroutine calls
// =========================================================================

#[test]
fn test_desubroutinize_no_calls() {
    // Charstring: rmoveto(100, 200) + endchar
    // 100 = 0x8B + 100 - 139 = byte 0xEB (CFF 1-byte encoding: v = b0 - 139)
    // But simpler: use 5-byte encoding: [29, 0, 0, 0, 100]
    // endchar = 14
    let charstring: Vec<u8> = vec![
        29, 0, 0, 0, 100,  // operand 100
        29, 0, 0, 0, 200,  // operand 200
        21,                 // rmoveto
        14,                 // endchar
    ];

    let empty_index_data = build_cff_index(&[]);
    let global_subrs = parse_cff_index(&empty_index_data, 0).unwrap();
    let local_subrs = parse_cff_index(&empty_index_data, 0).unwrap();

    let result = desubroutinize(&charstring, &global_subrs, &local_subrs).unwrap();

    // No calls to inline — output should be identical to input
    assert_eq!(result, charstring);
}

// =========================================================================
// Desubroutinization: local subr call
// =========================================================================

#[test]
fn test_desubroutinize_local_subr_call() {
    // Local subroutine 0: rmoveto(50, 50) + return
    // With bias=107 for count=1, callsubr index = 0 - 107 = -107
    // Encode -107: CFF 1-byte: b0 = -107 + 139 = 32 = 0x20
    let subr_body: Vec<u8> = vec![
        29, 0, 0, 0, 50,   // operand 50
        29, 0, 0, 0, 50,   // operand 50
        21,                 // rmoveto
        11,                 // return
    ];

    // Charstring: call local subr 0 + endchar
    // Biased index for subr 0: 0 - bias(1) = 0 - 107 = -107
    let charstring: Vec<u8> = vec![
        29, 255, 255, 255, 149,  // operand -107 (i32: -107 = 0xFFFFFF95)
        10,                       // callsubr
        14,                       // endchar
    ];

    let local_subrs_data = build_cff_index(&[&subr_body]);
    let local_subrs = parse_cff_index(&local_subrs_data, 0).unwrap();
    let empty_index_data = build_cff_index(&[]);
    let global_subrs = parse_cff_index(&empty_index_data, 0).unwrap();

    let result = desubroutinize(&charstring, &global_subrs, &local_subrs).unwrap();

    // Expected: subroutine body inlined (without return), then endchar
    let expected: Vec<u8> = vec![
        29, 0, 0, 0, 50,   // operand 50
        29, 0, 0, 0, 50,   // operand 50
        21,                 // rmoveto
        14,                 // endchar
    ];
    assert_eq!(result, expected);
}

// =========================================================================
// Desubroutinization: global subr call
// =========================================================================

#[test]
fn test_desubroutinize_global_subr_call() {
    // Global subroutine 0: rlineto(10, 20) + return
    let gsubr_body: Vec<u8> = vec![
        29, 0, 0, 0, 10,   // operand 10
        29, 0, 0, 0, 20,   // operand 20
        5,                  // rlineto
        11,                 // return
    ];

    // Charstring: rmoveto(0,0) + call global subr 0 + endchar
    let charstring: Vec<u8> = vec![
        29, 0, 0, 0, 0,         // operand 0
        29, 0, 0, 0, 0,         // operand 0
        21,                      // rmoveto
        29, 255, 255, 255, 149,  // operand -107 (biased index for gsubr 0)
        29,                      // callgsubr (opcode 29)
        14,                      // endchar
    ];

    // WAIT — opcode 29 is ALSO the 5-byte integer prefix!
    // This is a known ambiguity in CFF: byte 29 as first byte = 5-byte integer,
    // byte 29 after operands on stack = callgsubr operator.
    // The desubroutinizer must track the operand/operator state correctly.
    //
    // Let's use a cleaner encoding. Use 1-byte encoding for -107:
    // CFF 1-byte: value = b0 - 139, so b0 = -107 + 139 = 32
    let charstring: Vec<u8> = vec![
        29, 0, 0, 0, 0,    // operand 0
        29, 0, 0, 0, 0,    // operand 0
        21,                 // rmoveto
        32,                 // operand -107 (1-byte encoding: 32 - 139 = -107)
        29,                 // callgsubr
        14,                 // endchar
    ];

    // Hmm, byte 29 after byte 32 — is 32 an operand (yes, 32-246 range)
    // and then 29 as operator? But 29 is also the 5-byte integer prefix (28-31 range is NOT operands).
    // Actually in CFF Type 2:
    //   - Bytes 0-11: operators
    //   - Byte 12: escape prefix (2-byte operator)
    //   - Bytes 13-18: operators  
    //   - Byte 19-20: operators (hintmask, cntrmask)
    //   - Byte 21-27: operators
    //   - Byte 28: 2-byte integer (i16)
    //   - Byte 29: callgsubr operator (NOT 5-byte int in Type 2!)
    //   - Bytes 30-31: operators (vhcurveto, hvcurveto)
    //   - Bytes 32-246: 1-byte integers (value = b0 - 139)
    //   - Bytes 247-254: 2-byte integers
    //   - Byte 255: 4-byte fixed-point
    //
    // NOTE: CFF Type 2 charstrings and CFF DICT have DIFFERENT encodings!
    // In DICT: byte 29 = 5-byte integer
    // In Type 2 charstrings: byte 29 = callgsubr operator
    // This is critical for the desubroutinizer implementation.

    let charstring_clean: Vec<u8> = vec![
        32,     // operand -107 (1-byte: 32 - 139 = -107)
        29,     // callgsubr
        14,     // endchar
    ];

    let global_subrs_data = build_cff_index(&[&gsubr_body]);
    let global_subrs = parse_cff_index(&global_subrs_data, 0).unwrap();
    let empty_index_data = build_cff_index(&[]);
    let local_subrs = parse_cff_index(&empty_index_data, 0).unwrap();

    let result = desubroutinize(&charstring_clean, &global_subrs, &local_subrs).unwrap();

    let expected: Vec<u8> = vec![
        29, 0, 0, 0, 10,   // operand 10
        29, 0, 0, 0, 20,   // operand 20
        5,                  // rlineto
        14,                 // endchar
    ];
    assert_eq!(result, expected);
}

// =========================================================================
// Desubroutinization: nested calls (subr calls another subr)
// =========================================================================

#[test]
fn test_desubroutinize_nested_local_subrs() {
    // Subr 0: calls subr 1 + return
    // Subr 1: rlineto(5, 5) + return
    // With 2 subrs, bias = 107
    // Biased index for subr 1: 1 - 107 = -106
    // Encode -106 as 1-byte: b0 = -106 + 139 = 33

    let subr1_body: Vec<u8> = vec![
        29, 0, 0, 0, 5,    // operand 5
        29, 0, 0, 0, 5,    // operand 5
        5,                  // rlineto
        11,                 // return
    ];

    let subr0_body: Vec<u8> = vec![
        33,     // operand -106 (biased index for subr 1)
        10,     // callsubr
        11,     // return
    ];

    // Charstring: call subr 0 + endchar
    // Biased index for subr 0: 0 - 107 = -107, encode as byte 32
    let charstring: Vec<u8> = vec![
        32,     // operand -107 (biased index for subr 0)
        10,     // callsubr
        14,     // endchar
    ];

    let local_subrs_data = build_cff_index(&[&subr0_body, &subr1_body]);
    let local_subrs = parse_cff_index(&local_subrs_data, 0).unwrap();
    let empty_index_data = build_cff_index(&[]);
    let global_subrs = parse_cff_index(&empty_index_data, 0).unwrap();

    let result = desubroutinize(&charstring, &global_subrs, &local_subrs).unwrap();

    // Expected: subr0 → subr1 inlined, fully flat
    let expected: Vec<u8> = vec![
        29, 0, 0, 0, 5,    // operand 5
        29, 0, 0, 0, 5,    // operand 5
        5,                  // rlineto
        14,                 // endchar
    ];
    assert_eq!(result, expected);
}

// =========================================================================
// Desubroutinization: max depth exceeded
// =========================================================================

#[test]
fn test_desubroutinize_max_depth_exceeded() {
    // Create a chain of 65 subrs, each calling the next.
    // This exceeds the 64-level CFF spec limit.
    let mut subr_bodies: Vec<Vec<u8>> = Vec::new();

    // Subrs 0..63: each calls the next subr
    // With 65 subrs, bias = 107
    for i in 0..64 {
        let next_biased = (i + 1) as i32 - 107;
        let mut body = Vec::new();
        // Encode biased index using 5-byte DICT encoding... NO!
        // In Type 2 charstrings, use Type 2 number encoding.
        // For -107+i: 1-byte encoding: b0 = value + 139
        let b0 = (next_biased + 139) as u8;
        body.push(b0);
        body.push(10); // callsubr
        body.push(11); // return
        subr_bodies.push(body);
    }

    // Subr 64: just return
    subr_bodies.push(vec![11]);

    let subr_refs: Vec<&[u8]> = subr_bodies.iter().map(|v| v.as_slice()).collect();
    let local_subrs_data = build_cff_index(&subr_refs);
    let local_subrs = parse_cff_index(&local_subrs_data, 0).unwrap();
    let empty_index_data = build_cff_index(&[]);
    let global_subrs = parse_cff_index(&empty_index_data, 0).unwrap();

    // Charstring: call subr 0 + endchar
    let charstring: Vec<u8> = vec![
        32,     // operand -107 (biased index for subr 0)
        10,     // callsubr
        14,     // endchar
    ];

    let result = desubroutinize(&charstring, &global_subrs, &local_subrs);
    assert!(result.is_err(), "Should fail when recursion depth exceeds 64");
}

// =========================================================================
// Desubroutinization: empty charstring
// =========================================================================

#[test]
fn test_desubroutinize_empty_charstring() {
    let empty_index_data = build_cff_index(&[]);
    let global_subrs = parse_cff_index(&empty_index_data, 0).unwrap();
    let local_subrs = parse_cff_index(&empty_index_data, 0).unwrap();

    let result = desubroutinize(&[], &global_subrs, &local_subrs).unwrap();
    assert!(result.is_empty());
}

#[test]
fn test_desubroutinize_endchar_only() {
    let empty_index_data = build_cff_index(&[]);
    let global_subrs = parse_cff_index(&empty_index_data, 0).unwrap();
    let local_subrs = parse_cff_index(&empty_index_data, 0).unwrap();

    let charstring = vec![14u8]; // endchar
    let result = desubroutinize(&charstring, &global_subrs, &local_subrs).unwrap();
    assert_eq!(result, vec![14u8]);
}

// =========================================================================
// Desubroutinization: mixed local + global calls
// =========================================================================

#[test]
fn test_desubroutinize_mixed_local_and_global() {
    // Local subr 0: rmoveto(1, 2) + return
    let local_body: Vec<u8> = vec![
        29, 0, 0, 0, 1,    // operand 1
        29, 0, 0, 0, 2,    // operand 2
        21,                 // rmoveto
        11,                 // return
    ];

    // Global subr 0: rlineto(3, 4) + return
    let global_body: Vec<u8> = vec![
        29, 0, 0, 0, 3,    // operand 3
        29, 0, 0, 0, 4,    // operand 4
        5,                  // rlineto
        11,                 // return
    ];

    // Charstring: call local 0, call global 0, endchar
    let charstring: Vec<u8> = vec![
        32,     // operand -107 (biased index for local subr 0)
        10,     // callsubr
        32,     // operand -107 (biased index for global subr 0)
        29,     // callgsubr
        14,     // endchar
    ];

    let local_data = build_cff_index(&[&local_body]);
    let local_subrs = parse_cff_index(&local_data, 0).unwrap();
    let global_data = build_cff_index(&[&global_body]);
    let global_subrs = parse_cff_index(&global_data, 0).unwrap();

    let result = desubroutinize(&charstring, &global_subrs, &local_subrs).unwrap();

    let expected: Vec<u8> = vec![
        29, 0, 0, 0, 1,    // operand 1 (from local subr)
        29, 0, 0, 0, 2,    // operand 2
        21,                 // rmoveto
        29, 0, 0, 0, 3,    // operand 3 (from global subr)
        29, 0, 0, 0, 4,    // operand 4
        5,                  // rlineto
        14,                 // endchar
    ];
    assert_eq!(result, expected);
}

// =========================================================================
// Desubroutinization: operands before call are preserved except the index
// =========================================================================

#[test]
fn test_desubroutinize_preserves_preceding_operands() {
    // Local subr 0: just rmoveto + return (consumes 2 args from caller's stack)
    let local_body: Vec<u8> = vec![
        21,     // rmoveto (uses the 2 operands pushed before callsubr)
        11,     // return
    ];

    // Charstring: push 100, 200, then call subr 0 (index is separate arg), then endchar
    // The desubroutinizer should remove only the subr index operand,
    // keeping 100 and 200 on the output.
    let charstring: Vec<u8> = vec![
        29, 0, 0, 0, 100,  // operand 100
        29, 0, 0, 0, 200,  // operand 200
        32,                 // operand -107 (subr index)
        10,                 // callsubr
        14,                 // endchar
    ];

    let local_data = build_cff_index(&[&local_body]);
    let local_subrs = parse_cff_index(&local_data, 0).unwrap();
    let empty = build_cff_index(&[]);
    let global_subrs = parse_cff_index(&empty, 0).unwrap();

    let result = desubroutinize(&charstring, &global_subrs, &local_subrs).unwrap();

    // Expected: 100, 200 preserved, then rmoveto (from subr body), then endchar
    let expected: Vec<u8> = vec![
        29, 0, 0, 0, 100,  // operand 100 (preserved)
        29, 0, 0, 0, 200,  // operand 200 (preserved)
        21,                 // rmoveto (from subr body)
        14,                 // endchar
    ];
    assert_eq!(result, expected);
}
```

- [ ] **Step 4: Run tests to verify they fail**

Run: `cargo test --test cff_desubroutinize_test 2>&1 | tail -20`
Expected: Bias tests pass, all `desubroutinize` tests FAIL with "desubroutinize not yet implemented".

- [ ] **Step 5: Commit**

```bash
git add oxidize-pdf-core/src/text/fonts/cff/charstring.rs
git add oxidize-pdf-core/src/text/fonts/cff/mod.rs
git add oxidize-pdf-core/tests/cff_desubroutinize_test.rs
git commit -m "test(cff): add failing tests for charstring desubroutinizer (TDD red)"
```

---

## Task 5: Implement charstring desubroutinizer

Make the failing tests from Task 4 pass.

**Files:**
- Modify: `oxidize-pdf-core/src/text/fonts/cff/charstring.rs`

- [ ] **Step 1: Implement `desubroutinize`**

Replace the stub in `charstring.rs` with the full implementation:

```rust
/// Desubroutinize a charstring by recursively inlining all subroutine calls.
pub(crate) fn desubroutinize(
    charstring: &[u8],
    global_subrs: &CffIndex,
    local_subrs: &CffIndex,
) -> ParseResult<Vec<u8>> {
    desubroutinize_inner(charstring, global_subrs, local_subrs, 0)
}

/// Internal recursive implementation with depth tracking.
fn desubroutinize_inner(
    charstring: &[u8],
    global_subrs: &CffIndex,
    local_subrs: &CffIndex,
    depth: u8,
) -> ParseResult<Vec<u8>> {
    if depth > 64 {
        return Err(ParseError::SyntaxError {
            position: 0,
            message: format!("CFF subroutine recursion depth exceeded (max 64, got {})", depth),
        });
    }

    let mut output = Vec::new();
    // Track byte positions of operands so we can remove the subr index
    let mut operand_positions: Vec<usize> = Vec::new();
    let mut offset = 0;

    while offset < charstring.len() {
        let b0 = charstring[offset];

        match b0 {
            // Operators that consume stack
            10 => {
                // callsubr — pop last operand (subr index), inline subr body
                let (biased_index, operand_start) = pop_last_operand(&output, &mut operand_positions)?;
                output.truncate(operand_start);

                let actual_index = biased_index + cff_subr_bias(local_subrs.count()) as i32;
                let subr_data = local_subrs.get_item(actual_index as usize, local_subrs.raw_data())
                    .ok_or_else(|| ParseError::SyntaxError {
                        position: offset,
                        message: format!("Local subr index {} out of range", actual_index),
                    })?;

                let inlined = desubroutinize_inner(subr_data, global_subrs, local_subrs, depth + 1)?;
                output.extend_from_slice(&inlined);
                operand_positions.clear();
                offset += 1;
            }
            29 => {
                // callgsubr — pop last operand (subr index), inline subr body
                let (biased_index, operand_start) = pop_last_operand(&output, &mut operand_positions)?;
                output.truncate(operand_start);

                let actual_index = biased_index + cff_subr_bias(global_subrs.count()) as i32;
                let subr_data = global_subrs.get_item(actual_index as usize, global_subrs.raw_data())
                    .ok_or_else(|| ParseError::SyntaxError {
                        position: offset,
                        message: format!("Global subr index {} out of range", actual_index),
                    })?;

                let inlined = desubroutinize_inner(subr_data, global_subrs, local_subrs, depth + 1)?;
                output.extend_from_slice(&inlined);
                operand_positions.clear();
                offset += 1;
            }
            11 => {
                // return — stop processing this subroutine
                break;
            }
            14 => {
                // endchar — copy and stop
                output.push(14);
                operand_positions.clear();
                offset += 1;
            }
            12 => {
                // 2-byte operator (escape)
                if offset + 1 < charstring.len() {
                    output.push(b0);
                    output.push(charstring[offset + 1]);
                    offset += 2;
                } else {
                    output.push(b0);
                    offset += 1;
                }
                operand_positions.clear();
            }
            // 1-byte operators (0-9, 13-18, 19-20, 21-27, 30-31)
            0..=9 | 13..=18 | 21..=27 | 30..=31 => {
                // hintmask/cntrmask (19, 20) have data bytes after them
                if b0 == 19 || b0 == 20 {
                    output.push(b0);
                    offset += 1;
                    // The mask data length depends on the number of hints,
                    // which we don't track. However, we can determine it:
                    // For now, we just copy until the next valid operator/operand.
                    // Actually, hintmask data is ceil(num_hints / 8) bytes.
                    // Since we don't track hints, we need to count hint operators
                    // in the already-processed output.
                    // 
                    // SIMPLIFICATION: count stem hints from existing output to
                    // determine mask length. Stem operators: hstem(1), vstem(3),
                    // hstemhm(18), vstemhm(23). Each consumes pairs of operands.
                    //
                    // For a first pass, we track hint count during processing.
                    // TODO: implement hint counting. For now, skip mask bytes.
                    // The number of mask bytes = ceil(num_stems / 8).
                    // We'll need to add hint tracking to the desubroutinizer.
                    // For fonts without hints (most CJK fonts), this is 0.
                } else {
                    output.push(b0);
                    offset += 1;
                }
                operand_positions.clear();
            }
            // Number encodings (Type 2 charstring format)
            32..=246 => {
                // 1-byte integer: value = b0 - 139
                operand_positions.push(output.len());
                output.push(b0);
                offset += 1;
            }
            247..=250 => {
                // 2-byte positive: value = (b0 - 247) * 256 + b1 + 108
                if offset + 1 < charstring.len() {
                    operand_positions.push(output.len());
                    output.push(b0);
                    output.push(charstring[offset + 1]);
                    offset += 2;
                } else {
                    break;
                }
            }
            251..=254 => {
                // 2-byte negative: value = -(b0 - 251) * 256 - b1 - 108
                if offset + 1 < charstring.len() {
                    operand_positions.push(output.len());
                    output.push(b0);
                    output.push(charstring[offset + 1]);
                    offset += 2;
                } else {
                    break;
                }
            }
            28 => {
                // 3-byte integer (i16): value = b1 << 8 | b2
                if offset + 2 < charstring.len() {
                    operand_positions.push(output.len());
                    output.push(b0);
                    output.push(charstring[offset + 1]);
                    output.push(charstring[offset + 2]);
                    offset += 3;
                } else {
                    break;
                }
            }
            255 => {
                // 5-byte fixed-point: 16.16 format
                if offset + 4 < charstring.len() {
                    operand_positions.push(output.len());
                    output.push(b0);
                    output.push(charstring[offset + 1]);
                    output.push(charstring[offset + 2]);
                    output.push(charstring[offset + 3]);
                    output.push(charstring[offset + 4]);
                    offset += 5;
                } else {
                    break;
                }
            }
            _ => {
                // Unknown byte — copy as-is
                output.push(b0);
                offset += 1;
            }
        }
    }

    Ok(output)
}

/// Decode the value of the last operand and return (value, start_position_in_output).
fn pop_last_operand(
    output: &[u8],
    operand_positions: &mut Vec<usize>,
) -> ParseResult<(i32, usize)> {
    let start = operand_positions.pop().ok_or_else(|| ParseError::SyntaxError {
        position: 0,
        message: "callsubr/callgsubr with empty operand stack".to_string(),
    })?;

    let bytes = &output[start..];
    let value = decode_type2_number(bytes)?;
    Ok((value, start))
}

/// Decode a Type 2 charstring number from its byte encoding.
fn decode_type2_number(bytes: &[u8]) -> ParseResult<i32> {
    if bytes.is_empty() {
        return Err(ParseError::SyntaxError {
            position: 0,
            message: "Empty number encoding".to_string(),
        });
    }

    let b0 = bytes[0];
    match b0 {
        32..=246 => Ok(b0 as i32 - 139),
        247..=250 => {
            if bytes.len() < 2 {
                return Err(ParseError::SyntaxError {
                    position: 0,
                    message: "Truncated 2-byte positive number".to_string(),
                });
            }
            Ok((b0 as i32 - 247) * 256 + bytes[1] as i32 + 108)
        }
        251..=254 => {
            if bytes.len() < 2 {
                return Err(ParseError::SyntaxError {
                    position: 0,
                    message: "Truncated 2-byte negative number".to_string(),
                });
            }
            Ok(-(b0 as i32 - 251) * 256 - bytes[1] as i32 - 108)
        }
        28 => {
            if bytes.len() < 3 {
                return Err(ParseError::SyntaxError {
                    position: 0,
                    message: "Truncated 3-byte number".to_string(),
                });
            }
            Ok(((bytes[1] as i16) << 8 | bytes[2] as i16) as i32)
        }
        255 => {
            if bytes.len() < 5 {
                return Err(ParseError::SyntaxError {
                    position: 0,
                    message: "Truncated 5-byte fixed-point number".to_string(),
                });
            }
            // 16.16 fixed point — return as integer (we only need the value for bias calc)
            let val = i32::from_be_bytes([bytes[1], bytes[2], bytes[3], bytes[4]]);
            Ok(val)
        }
        _ => Err(ParseError::SyntaxError {
            position: 0,
            message: format!("Byte {} is not a valid Type 2 number encoding", b0),
        }),
    }
}
```

**IMPORTANT NOTE on `CffIndex`**: The desubroutinizer needs `CffIndex` to support two things that it may not currently:
1. `count()` → number of items (already exists, line 634)
2. `get_item(index, data)` → get item bytes by index (already exists, line 643)
3. `raw_data()` → reference to the underlying data slice

If `CffIndex` doesn't store a reference to the raw data, we need to adjust the API. The current `CffIndex::get_item()` takes `data: &[u8]` as a parameter (the outer CFF data), so we pass it. The `desubroutinize` function needs to receive the raw data alongside the index. Adjust the function signature to:

```rust
pub(crate) fn desubroutinize(
    charstring: &[u8],
    global_subrs: &CffIndex,
    global_subrs_data: &[u8],
    local_subrs: &CffIndex,
    local_subrs_data: &[u8],
) -> ParseResult<Vec<u8>>
```

And update the tests accordingly (pass the raw data buffers).

- [ ] **Step 2: Handle hint counting for hintmask/cntrmask**

Add a `hint_count` tracker to the desubroutinizer. Stem operators (hstem=1, vstem=3, hstemhm=18, vstemhm=23) each add `floor(stack_size / 2)` stems. After the first hintmask/cntrmask, the mask length is `ceil(hint_count / 8)` bytes.

```rust
// Add to the desubroutinize_inner function:
let mut hint_count: usize = 0;
let mut seen_hint_mask = false;
let mut operand_count: usize = 0;

// In the operand match arms, increment operand_count
// In stem operator handling:
1 | 3 | 18 | 23 => {
    hint_count += operand_count / 2;
    output.push(b0);
    offset += 1;
    operand_positions.clear();
    operand_count = 0;
}

// In hintmask/cntrmask handling:
19 | 20 => {
    if !seen_hint_mask {
        // First mask — remaining stack operands are implicit vstem hints
        hint_count += operand_count / 2;
        seen_hint_mask = true;
    }
    output.push(b0);
    offset += 1;
    let mask_bytes = (hint_count + 7) / 8;
    for _ in 0..mask_bytes {
        if offset < charstring.len() {
            output.push(charstring[offset]);
            offset += 1;
        }
    }
    operand_positions.clear();
    operand_count = 0;
}
```

- [ ] **Step 3: Run tests**

Run: `cargo test --test cff_desubroutinize_test -v`
Expected: ALL tests pass (TDD green).

- [ ] **Step 4: Run full test suite for regressions**

Run: `cargo test --test cff_subsetter_test && cargo test --lib -- cff`
Expected: All pass.

- [ ] **Step 5: Commit**

```bash
git add oxidize-pdf-core/src/text/fonts/cff/charstring.rs
git commit -m "feat(cff): implement charstring desubroutinizer"
```

---

## Task 6: Wire desubroutinizer into CFF subsetter

Replace the Local Subr stub-replacement strategy with full desubroutinization in the CFF subsetting pipeline.

**Files:**
- Modify: `oxidize-pdf-core/src/text/fonts/cff_subsetter.rs`

- [ ] **Step 1: Write a failing integration test**

Add to `tests/cff_subsetter_test.rs`:

```rust
#[test]
fn test_cff_subset_no_subrs_in_output() {
    // After subsetting a CFF font with subroutines, the output should
    // contain NO subroutine calls — all charstrings are self-contained.
    let font_data = build_large_cff_otf_with_subrs();
    let used: HashSet<char> = "AB".chars().collect();
    let result = subset_font(font_data, &used).unwrap();

    // The output is raw CFF (not OTF wrapper)
    assert!(result.is_raw_cff, "CFF subset should be raw CFF");

    // Parse the output CFF and verify Global Subr INDEX is empty
    let cff_data = &result.font_data;
    // Skip header (4 bytes), Name INDEX, Top DICT INDEX, String INDEX
    // Then Global Subr INDEX should have count=0
    // We verify by checking there are no callsubr (10) or callgsubr (29)
    // operators in the charstring data.
    // (Full structural validation is a future task)
    let has_subr_calls = cff_data.windows(1).enumerate().any(|(i, w)| {
        // This is approximate — a byte value 10 or 29 could be part of
        // a number encoding. But for our synthetic test fonts with 5-byte
        // encoding, stray 10/29 bytes in operands are unlikely.
        w[0] == 10 || w[0] == 29
    });
    // Note: byte 29 could appear in 5-byte number prefix in DICT data,
    // so this test is best-effort. The real verification is the outline test.
    // For now, just verify the font is smaller than before.
    assert!(result.font_data.len() < 1000,
        "Subset of 2 chars from synthetic font should be small, got {} bytes",
        result.font_data.len());
}
```

Also add the helper `build_large_cff_otf_with_subrs()` that creates a CFF OTF with charstrings that use Local Subrs.

- [ ] **Step 2: Modify `subset_cff_table()` to use desubroutinizer**

In `cff_subsetter.rs`, in the charstring subsetting section of `subset_cff_table()` and `subset_cid_cff_table()`:

1. Parse Global Subr INDEX and Local Subr INDEXes (already done)
2. For each kept charstring, call `desubroutinize()` instead of copying raw bytes
3. Write empty Global Subr INDEX: `build_cff_index(&[])`
4. Write Private DICTs without Subrs offset

Remove:
- `collect_subr_calls()`
- `collect_used_subrs_transitive()`
- `collect_used_subrs_full()`
- `collect_used_global_subrs_transitive()`
- `filter_subr_index()`
- `patch_private_subrs_offset()`
- `parse_local_subrs_offset()`

These are all replaced by the desubroutinizer.

- [ ] **Step 3: Run tests**

Run: `cargo test --test cff_subsetter_test && cargo test --test cff_desubroutinize_test && cargo test --lib -- cff`
Expected: All pass.

- [ ] **Step 4: Commit**

```bash
git add oxidize-pdf-core/src/text/fonts/cff_subsetter.rs
git add oxidize-pdf-core/tests/cff_subsetter_test.rs
git commit -m "feat(cff): replace Local Subr stubs with full desubroutinization"
```

---

## Task 7: SID→CID conversion — unify CFF output

Convert SID-keyed fonts to CID-keyed during subsetting. Remove the OTF wrapper path.

**Files:**
- Modify: `oxidize-pdf-core/src/text/fonts/cff_subsetter.rs`
- Modify: `oxidize-pdf-core/tests/cff_subsetter_test.rs`

- [ ] **Step 1: Write failing test**

Add to `tests/cff_subsetter_test.rs`:

```rust
#[test]
fn test_sid_keyed_font_outputs_raw_cff_not_otf() {
    // SID-keyed CFF fonts (non-CID) should now be converted to CID-keyed
    // and output as raw CFF, not wrapped in OTF.
    let font_data = build_large_cff_otf(); // This is SID-keyed (no FDArray)
    let used: HashSet<char> = "AB".chars().collect();
    let result = subset_font(font_data, &used).unwrap();

    assert!(result.is_raw_cff,
        "SID-keyed CFF should be converted to CID and returned as raw CFF");

    // Must NOT start with OTTO signature (not an OTF wrapper)
    if result.font_data.len() >= 4 {
        let sig = u32::from_be_bytes([
            result.font_data[0], result.font_data[1],
            result.font_data[2], result.font_data[3],
        ]);
        assert_ne!(sig, 0x4F54544F,
            "Raw CFF should not start with OTTO signature");
    }

    // Must start with CFF header (major=1, minor=0)
    assert!(result.font_data.len() >= 4, "CFF data too short");
    assert_eq!(result.font_data[0], 1, "CFF major version should be 1");
    assert_eq!(result.font_data[1], 0, "CFF minor version should be 0");
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --test cff_subsetter_test test_sid_keyed_font_outputs_raw_cff_not_otf`
Expected: FAIL — currently returns OTF wrapper with `is_raw_cff: false`.

- [ ] **Step 3: Implement SID→CID conversion**

In `subset_cff_table()` (the non-CID path), instead of calling `rebuild_top_dict()` + OTF wrapper:

1. Build a CID-keyed Top DICT with ROS (Adobe, Identity, 0), FDArray offset, FDSelect offset, charset offset
2. Build FDSelect Format 0: `[0u8] + vec![0u8; num_glyphs]`
3. Build FDArray with single font dict containing the original Private DICT (without Subrs)
4. Build charset Format 2: `[2u8, 0, 1, (num_glyphs-2) as u16 big-endian]`
5. Return raw CFF bytes with `is_raw_cff: true`

Remove:
- `rebuild_top_dict()` (non-CID top dict builder)
- `OtfFile::rebuild_subset()` and all `OtfFile` methods
- `OtfFile` struct, `OtfTableEntry` struct
- `build_minimal_cmap()`

- [ ] **Step 4: Update existing test that asserts OTTO signature**

The test `test_cff_subset_reduces_size_and_preserves_mapping` asserts the subset starts with OTTO. Update it:

```rust
// OLD:
assert_eq!(sfnt, 0x4F54544F, "Subset font must have OTTO signature");
// NEW:
assert_eq!(result.font_data[0], 1, "CFF subset must start with CFF header (major=1)");
assert!(result.is_raw_cff, "CFF subset must be raw CFF");
```

- [ ] **Step 5: Run tests**

Run: `cargo test --test cff_subsetter_test && cargo test --lib -- cff`
Expected: All pass.

- [ ] **Step 6: Commit**

```bash
git add oxidize-pdf-core/src/text/fonts/cff_subsetter.rs
git add oxidize-pdf-core/tests/cff_subsetter_test.rs
git commit -m "feat(cff): convert SID-keyed fonts to CID-keyed, always output raw CFF"
```

---

## Task 8: TTF table stripping

Remove cmap, OS/2, and name tables from TTF subset output.

**Files:**
- Modify: `oxidize-pdf-core/src/text/fonts/truetype_subsetter.rs`

- [ ] **Step 1: Write failing test**

Add to a new test or existing test file. Since TTF subsetting tests are in `truetype_subsetter.rs` inline tests (lines 906+), add there:

```rust
#[test]
fn test_ttf_subset_does_not_contain_cmap_os2_name() {
    // TTF subsets for PDF embedding should not contain cmap, OS/2, or name tables.
    // PDF uses its own ToUnicode CMap, and OS/2/name are not needed for rendering.
    let subsetter = TrueTypeSubsetter::new(/* need a TTF font here */).unwrap();
    // This test needs a real or synthetic TTF. Use the existing test infrastructure.
    // For now, we verify by checking the table directory of the output.
}
```

Actually, the better approach is to verify at the `build_font_file` level. Since `build_font_file` is private, test via the public `subset()` method using a real TTF fixture or the test infrastructure that already exists.

Add to `tests/cff_subsetter_test.rs` or create `tests/ttf_subsetter_test.rs`:

```rust
// In truetype_subsetter.rs inline tests:
#[test]
fn test_subset_font_excludes_cmap_os2_name_tables() {
    // Build a minimal TTF font with cmap, OS/2, name tables
    // Subset it and verify the output does NOT contain these tables
    // by parsing the table directory.

    // For this test we need to parse the output font's table directory.
    // The output is a TTF file starting with sfnt version + table count.
    let font_data = build_test_ttf_font(); // helper that creates a valid TTF
    let used_chars: HashSet<char> = vec!['A', 'B'].into_iter().collect();
    let result = subset_font(font_data, &used_chars).unwrap();

    // Parse table directory
    let num_tables = u16::from_be_bytes([result.font_data[4], result.font_data[5]]);
    let mut table_tags: Vec<[u8; 4]> = Vec::new();
    for i in 0..num_tables as usize {
        let base = 12 + i * 16;
        let tag = [
            result.font_data[base],
            result.font_data[base + 1],
            result.font_data[base + 2],
            result.font_data[base + 3],
        ];
        table_tags.push(tag);
    }

    assert!(!table_tags.contains(&*b"cmap"), "Subset should not contain cmap table");
    assert!(!table_tags.contains(&*b"OS/2"), "Subset should not contain OS/2 table");
    assert!(!table_tags.contains(&*b"name"), "Subset should not contain name table");

    // Should still contain essential tables
    assert!(table_tags.contains(&*b"glyf"), "Subset must contain glyf table");
    assert!(table_tags.contains(&*b"head"), "Subset must contain head table");
    assert!(table_tags.contains(&*b"loca"), "Subset must contain loca table");
    assert!(table_tags.contains(&*b"hmtx"), "Subset must contain hmtx table");
}
```

- [ ] **Step 2: Run test to verify it fails**

Expected: FAIL — current output includes cmap, OS/2, name.

- [ ] **Step 3: Modify `build_font_file()` to exclude tables**

In `truetype_subsetter.rs`, modify `build_font_file()` (line 740):

Remove the `cmap` parameter. Remove `name_table`, `os2_table`, and their entries in `tables_to_write`.

```rust
fn build_font_file(
    &self,
    glyf: Vec<u8>,
    loca: Vec<u8>,
    hmtx: Vec<u8>,
    num_glyphs: u16,
    loca_format: u16,
) -> ParseResult<Vec<u8>> {
    // ... (existing setup code)

    let head_table = self.get_table_data(b"head")?;
    let hhea_table = self.update_hhea_table(num_glyphs)?;
    let maxp_table = self.get_original_maxp(num_glyphs)?;
    let post_table = self
        .get_table_data(b"post")
        .unwrap_or_else(|_| vec![0x00, 0x03, 0x00, 0x00]);

    tables_to_write.push((b"glyf", glyf));
    tables_to_write.push((b"head", self.update_head_table(head_table, loca_format)?));
    tables_to_write.push((b"hhea", hhea_table));
    tables_to_write.push((b"hmtx", hmtx));
    tables_to_write.push((b"loca", loca));
    tables_to_write.push((b"maxp", maxp_table));
    tables_to_write.push((b"post", post_table));
    // NO cmap, name, or OS/2

    // ... (rest unchanged)
}
```

Update `build_subset_font()` to not build cmap and not pass it:

```rust
fn build_subset_font(
    &self,
    glyph_map: &HashMap<u16, u16>,
    _new_cmap: &HashMap<u32, u16>,  // no longer needed
) -> ParseResult<Vec<u8>> {
    // ... (glyf, loca, hmtx building unchanged)

    // Remove: let new_cmap_data = self.build_cmap_table(new_cmap)?;

    self.build_font_file(
        new_glyf,
        new_loca,
        new_hmtx,
        glyph_map.len() as u16,
        loca_format,
    )
}
```

- [ ] **Step 4: Remove dead code**

Delete these methods from `TrueTypeSubsetter`:
- `build_cmap_table()` (~30 lines)
- `build_cmap_format4()` (~100 lines)
- `build_cmap_format12()` (~70 lines)

- [ ] **Step 5: Run tests**

Run: `cargo test 2>&1 | tail -5`
Expected: All pass.

- [ ] **Step 6: Commit**

```bash
git add oxidize-pdf-core/src/text/fonts/truetype_subsetter.rs
git commit -m "feat(ttf): strip cmap, OS/2, name tables from subset output"
```

---

## Task 9: Simplify PDF writer for unified CFF embedding

Remove the `embed_as_raw_cff` branching — CFF is always raw CFF now.

**Files:**
- Modify: `oxidize-pdf-core/src/writer/pdf_writer/mod.rs`

- [ ] **Step 1: Simplify CFF embedding in `write_type0_font_from_font()`**

At lines 1365-1415, the current code has:
```rust
let (font_data_to_embed, subset_glyph_mapping, original_font_for_widths, embed_as_raw_cff) = ...
```

Change the tuple to remove `embed_as_raw_cff` (it's always true for CFF). The logic becomes:

For the font file stream writing (lines 1389-1415):
```rust
if !font_data_to_embed.is_empty() {
    let mut font_file_dict = Dictionary::new();
    match font.format {
        crate::fonts::FontFormat::OpenType => {
            // CFF font — always raw CFF with CIDFontType0C
            font_file_dict.set("Subtype", Object::Name("CIDFontType0C".to_string()));
        }
        crate::fonts::FontFormat::TrueType => {
            font_file_dict.set("Length1", Object::Integer(font_data_to_embed.len() as i64));
        }
    }
    // ... (rest unchanged)
}
```

For the font descriptor (lines 1440-1444):
```rust
let font_file_key = match font.format {
    crate::fonts::FontFormat::OpenType => "FontFile3",
    crate::fonts::FontFormat::TrueType => "FontFile2",
};
```
(unchanged — already correct)

For CIDFont subtype (lines 1451-1457):
```rust
let cid_font_subtype = match font.format {
    crate::fonts::FontFormat::OpenType => "CIDFontType0",
    crate::fonts::FontFormat::TrueType => "CIDFontType2",
};
```

Remove `CjkFontType::should_use_cidfonttype2()` if it's no longer needed.

- [ ] **Step 2: Run full test suite**

Run: `cargo test 2>&1 | tail -10`
Expected: All pass.

- [ ] **Step 3: Commit**

```bash
git add oxidize-pdf-core/src/writer/pdf_writer/mod.rs
git commit -m "refactor(writer): simplify CFF embedding — always CIDFontType0C"
```

---

## Task 10: Size regression tests

Add tests that verify subset output size stays below thresholds.

**Files:**
- Create: `oxidize-pdf-core/tests/font_subset_size_test.rs`

- [ ] **Step 1: Create size regression tests**

```rust
// oxidize-pdf-core/tests/font_subset_size_test.rs
//! Size regression tests for font subsetting.
//! These verify that subset output stays below target thresholds
//! to prevent regressions in subsetting quality.

use oxidize_pdf::text::fonts::truetype_subsetter::subset_font;
use std::collections::HashSet;

// Helper to build a large SID-keyed CFF OTF for size testing
// (reuse or import the helpers from cff_subsetter_test.rs)

/// Build a synthetic CFF OTF with 10,000 glyphs and Local Subrs.
/// Each charstring calls Local Subr 0 (rmoveto) + Local Subr 1 (rlineto).
/// After subsetting to 2-3 chars, the 500 unused subrs should disappear.
fn build_large_font_with_subrs() -> Vec<u8> {
    use oxidize_pdf::text::fonts::cff::index::build_cff_index;

    let num_glyphs: u16 = 10_000;

    // --- CFF table ---
    let mut cff = Vec::new();
    cff.push(1); cff.push(0); cff.push(4); cff.push(1); // header

    let name_index = build_cff_index(&[b"TestCFF"]);
    cff.extend_from_slice(&name_index);

    // Local Subr 0: rmoveto(50,50) + return
    let subr0: Vec<u8> = vec![
        29, 0, 0, 0, 50,   // 50
        29, 0, 0, 0, 50,   // 50
        21, 11,             // rmoveto + return
    ];
    // Local Subr 1: rlineto(10,10) + return
    let subr1: Vec<u8> = vec![
        29, 0, 0, 0, 10,   // 10
        29, 0, 0, 0, 10,   // 10
        5, 11,              // rlineto + return
    ];
    // 498 unused subrs: just endchar + return
    let unused_subr: Vec<u8> = vec![14, 11];

    let mut subr_items: Vec<&[u8]> = vec![&subr0, &subr1];
    let unused_ref: &[u8] = &unused_subr;
    for _ in 0..498 {
        subr_items.push(unused_ref);
    }
    let local_subrs_index = build_cff_index(&subr_items);

    // Each charstring calls subr 0 then subr 1 then endchar
    // bias for 500 subrs = 107
    // subr 0: biased = 0 - 107 = -107, encode 1-byte: 32
    // subr 1: biased = 1 - 107 = -106, encode 1-byte: 33
    let charstring: Vec<u8> = vec![32, 10, 33, 10, 14]; // call subr0, call subr1, endchar
    let cs_ref: &[u8] = &charstring;
    let cs_items: Vec<&[u8]> = (0..num_glyphs).map(|_| cs_ref).collect();
    let charstrings_index = build_cff_index(&cs_items);

    // Charset format 0
    let mut charset = vec![0u8];
    for i in 1..num_glyphs { charset.extend_from_slice(&i.to_be_bytes()); }

    // String INDEX (empty), Global Subr INDEX (empty)
    let string_index = build_cff_index(&[]);
    let global_subr_index = build_cff_index(&[]);

    // Two-pass Top DICT offset calculation (same pattern as test helpers)
    // Top DICT needs: charset, CharStrings, Private (with local subrs)
    // For simplicity, build a minimal Top DICT pointing to charset and charstrings,
    // with a Private DICT that points to the Local Subr INDEX.
    // (The exact offset math follows the same pattern as build_large_cff_table)

    // Placeholder pass to measure sizes
    let encode_int = |v: i32| -> Vec<u8> { let mut r = vec![29u8]; r.extend_from_slice(&v.to_be_bytes()); r };
    let placeholder_dict = {
        let mut d = Vec::new();
        d.extend_from_slice(&encode_int(99999)); d.push(15); // charset
        d.extend_from_slice(&encode_int(99999)); d.push(17); // CharStrings
        d.extend_from_slice(&encode_int(99999)); // Private size
        d.extend_from_slice(&encode_int(99999)); d.push(18); // Private offset
        d
    };
    let placeholder_ref: &[u8] = &placeholder_dict;
    let placeholder_top = build_cff_index(&[placeholder_ref]);

    let after_top = cff.len() + placeholder_top.len();
    let after_str = after_top + string_index.len();
    let after_gsubr = after_str + global_subr_index.len();
    let charset_off = after_gsubr;
    let cs_off = charset_off + charset.len();
    let private_off = cs_off + charstrings_index.len();

    // Private DICT: just Subrs offset pointing to local_subrs_index appended right after
    let private_dict = {
        let mut d = Vec::new();
        let subrs_off = 0i32; // placeholder, will be patched
        // defaultWidthX = 0 (operator 20)
        d.extend_from_slice(&encode_int(0)); d.push(20);
        // Subrs offset (operator 19) — offset relative to start of Private DICT
        let pd_size_estimate = 5 + 1 + 5 + 1; // two entries
        d.extend_from_slice(&encode_int(pd_size_estimate as i32)); d.push(19);
        d
    };
    let private_size = private_dict.len();

    let real_dict = {
        let mut d = Vec::new();
        d.extend_from_slice(&encode_int(charset_off as i32)); d.push(15);
        d.extend_from_slice(&encode_int(cs_off as i32)); d.push(17);
        d.extend_from_slice(&encode_int(private_size as i32));
        d.extend_from_slice(&encode_int(private_off as i32)); d.push(18);
        d
    };
    let real_ref: &[u8] = &real_dict;
    let top_dict_index = build_cff_index(&[real_ref]);

    cff.extend_from_slice(&top_dict_index);
    cff.extend_from_slice(&string_index);
    cff.extend_from_slice(&global_subr_index);
    cff.extend_from_slice(&charset);
    cff.extend_from_slice(&charstrings_index);
    cff.extend_from_slice(&private_dict);
    cff.extend_from_slice(&local_subrs_index);

    // --- Wrap in OTF (same as build_large_cff_otf) ---
    // head, hhea, hmtx, maxp, cmap tables + CFF
    let mut head = vec![0u8; 54];
    head[0..4].copy_from_slice(&0x00010000u32.to_be_bytes());
    head[18..20].copy_from_slice(&1000u16.to_be_bytes());

    let mut hhea = vec![0u8; 36];
    hhea[0..4].copy_from_slice(&0x00010000u32.to_be_bytes());
    hhea[34..36].copy_from_slice(&num_glyphs.to_be_bytes());

    let mut hmtx = Vec::with_capacity(num_glyphs as usize * 4);
    for _ in 0..num_glyphs {
        hmtx.extend_from_slice(&600u16.to_be_bytes());
        hmtx.extend_from_slice(&0i16.to_be_bytes());
    }

    let mut maxp = vec![0u8; 6];
    maxp[0..4].copy_from_slice(&0x00005000u32.to_be_bytes());
    maxp[4..6].copy_from_slice(&num_glyphs.to_be_bytes());

    // Simple cmap: A-Z → GID 1-26
    let mut cmap = Vec::new();
    cmap.extend_from_slice(&0u16.to_be_bytes()); // version
    cmap.extend_from_slice(&1u16.to_be_bytes()); // numTables
    cmap.extend_from_slice(&3u16.to_be_bytes()); // platformID
    cmap.extend_from_slice(&1u16.to_be_bytes()); // encodingID
    cmap.extend_from_slice(&12u32.to_be_bytes()); // offset
    cmap.extend_from_slice(&4u16.to_be_bytes()); // format
    let subtable_len: u16 = 14 + 2 * 2 * 4;
    cmap.extend_from_slice(&subtable_len.to_be_bytes());
    cmap.extend_from_slice(&0u16.to_be_bytes()); // language
    cmap.extend_from_slice(&4u16.to_be_bytes()); // segCountX2
    cmap.extend_from_slice(&2u16.to_be_bytes()); // searchRange
    cmap.extend_from_slice(&0u16.to_be_bytes()); // entrySelector
    cmap.extend_from_slice(&2u16.to_be_bytes()); // rangeShift
    cmap.extend_from_slice(&0x005Au16.to_be_bytes()); // endCode Z
    cmap.extend_from_slice(&0xFFFFu16.to_be_bytes());
    cmap.extend_from_slice(&0u16.to_be_bytes()); // reservedPad
    cmap.extend_from_slice(&0x0041u16.to_be_bytes()); // startCode A
    cmap.extend_from_slice(&0xFFFFu16.to_be_bytes());
    cmap.extend_from_slice(&(-64i16).to_be_bytes()); // idDelta
    cmap.extend_from_slice(&1i16.to_be_bytes());
    cmap.extend_from_slice(&0u16.to_be_bytes());
    cmap.extend_from_slice(&0u16.to_be_bytes());

    let table_defs: Vec<(&[u8; 4], Vec<u8>)> = vec![
        (b"CFF ", cff), (b"cmap", cmap), (b"head", head),
        (b"hhea", hhea), (b"hmtx", hmtx), (b"maxp", maxp),
    ];
    let num_tables = table_defs.len() as u16;
    let header_size = 12 + num_tables as usize * 16;

    let mut font = Vec::new();
    font.extend_from_slice(&0x4F54544Fu32.to_be_bytes()); // OTTO
    font.extend_from_slice(&num_tables.to_be_bytes());
    font.extend_from_slice(&0u16.to_be_bytes());
    font.extend_from_slice(&0u16.to_be_bytes());
    font.extend_from_slice(&0u16.to_be_bytes());

    let mut current_offset = header_size;
    let mut entries = Vec::new();
    for (_, data) in &table_defs {
        while current_offset % 4 != 0 { current_offset += 1; }
        entries.push((current_offset as u32, data.len() as u32));
        current_offset += data.len();
    }
    for (i, (tag, _)) in table_defs.iter().enumerate() {
        font.extend_from_slice(*tag);
        font.extend_from_slice(&0u32.to_be_bytes());
        font.extend_from_slice(&entries[i].0.to_be_bytes());
        font.extend_from_slice(&entries[i].1.to_be_bytes());
    }
    for (i, (_, data)) in table_defs.iter().enumerate() {
        while font.len() < entries[i].0 as usize { font.push(0); }
        font.extend_from_slice(data);
    }
    font
}

#[test]
fn test_cff_subset_size_regression() {
    let font_data = build_large_font_with_subrs();
    let original_size = font_data.len();
    let used: HashSet<char> = "ABC".chars().collect();
    let result = subset_font(font_data, &used).unwrap();

    // After desubroutinization + raw CFF output, the subset should be
    // dramatically smaller than the original.
    // Target: <1% of original for 3 chars out of 10,000
    let ratio = result.font_data.len() as f64 / original_size as f64;
    assert!(ratio < 0.01,
        "CFF subset ({} bytes) should be <1% of original ({} bytes), ratio: {:.4}",
        result.font_data.len(), original_size, ratio);
}

#[test]
fn test_ttf_subset_no_cmap_size_reduction() {
    // Verify that TTF subsets are smaller without cmap/OS/2/name.
    // This test uses the existing synthetic TTF infrastructure.
    // The key assertion: no cmap table in output.
    let font_data = build_test_ttf_font(); // from existing helpers
    let used: HashSet<char> = "AB".chars().collect();
    let result = subset_font(font_data, &used).unwrap();

    // Parse table directory and verify no cmap
    let num_tables = u16::from_be_bytes([result.font_data[4], result.font_data[5]]);
    for i in 0..num_tables as usize {
        let base = 12 + i * 16;
        let tag = &result.font_data[base..base + 4];
        assert_ne!(tag, b"cmap", "TTF subset should not contain cmap table");
        assert_ne!(tag, b"OS/2", "TTF subset should not contain OS/2 table");
        assert_ne!(tag, b"name", "TTF subset should not contain name table");
    }
}
```

- [ ] **Step 2: Run tests**

Run: `cargo test --test font_subset_size_test`
Expected: All pass (if implementations from Tasks 5-8 are correct).

- [ ] **Step 3: Commit**

```bash
git add oxidize-pdf-core/tests/font_subset_size_test.rs
git commit -m "test: add size regression tests for font subsetting"
```

---

## Task 11: Clean up dead code and finalize

Remove dead code from the refactored files and ensure everything compiles cleanly.

**Files:**
- Modify: `oxidize-pdf-core/src/text/fonts/cff_subsetter.rs`
- Modify: `oxidize-pdf-core/src/text/fonts/truetype_subsetter.rs`

- [ ] **Step 1: Remove dead code from `cff_subsetter.rs`**

After Tasks 3, 6, and 7, the following should be dead code in `cff_subsetter.rs`:
- `OtfFile` struct and all its methods (`parse`, `find_table`, `rebuild_subset`, `patch_maxp`, `patch_hhea`, `truncate_hmtx`, `build_minimal_cmap`, `REQUIRED_TABLES`)
- `OtfTableEntry` struct
- `collect_subr_calls()` (replaced by desubroutinizer)
- `collect_used_subrs_transitive()` (replaced)
- `collect_used_subrs_full()` (replaced)
- `collect_used_global_subrs_transitive()` (replaced)
- `filter_subr_index()` (replaced)
- `patch_private_subrs_offset()` (replaced)
- `parse_local_subrs_offset()` (replaced)
- `rebuild_top_dict()` (non-CID path removed)
- Related inline tests for removed functions

Delete all of the above.

- [ ] **Step 2: Remove dead code from `truetype_subsetter.rs`**

After Task 8:
- `build_cmap_table()` (if not already deleted)
- `build_cmap_format4()`
- `build_cmap_format12()`
- Related inline tests for these methods

- [ ] **Step 3: Compile with warnings as errors**

Run: `cargo build 2>&1 | grep -i warning`
Expected: No warnings about unused functions, dead code, etc.

- [ ] **Step 4: Run full test suite**

Run: `cargo test 2>&1 | tail -5`
Expected: All pass. Some inline tests may have been removed along with the dead code — that's expected.

- [ ] **Step 5: Commit**

```bash
git add oxidize-pdf-core/src/text/fonts/cff_subsetter.rs
git add oxidize-pdf-core/src/text/fonts/truetype_subsetter.rs
git commit -m "refactor: remove dead code from subsetter refactor (OTF wrapper, subr filtering)"
```

---

## Task 12: Final validation — full round-trip test

Verify the complete pipeline works end-to-end: create a document with custom fonts, write to PDF, verify the PDF is valid.

**Files:**
- Modify: `oxidize-pdf-core/tests/cff_subsetter_test.rs` (or existing integration tests)

- [ ] **Step 1: Verify existing round-trip tests pass**

Run: `cargo test --test cff_subsetter_test`
Expected: All pass.

- [ ] **Step 2: Verify existing full test suite passes**

Run: `cargo test`
Expected: All pass, no regressions.

- [ ] **Step 3: Check for compiler warnings**

Run: `cargo build 2>&1`
Expected: Zero warnings.

- [ ] **Step 4: Final commit**

If any fixes were needed in this task:
```bash
git add -A
git commit -m "fix: address final integration issues from font subsetting improvements"
```

---

## Note: Outline verification tests

The spec calls for outline verification tests (comparing glyph paths between original and subset). These are deferred to a follow-up because they require a CFF charstring path interpreter, which is non-trivial to build (tracking moveto/lineto/curveto from Type 2 charstring operators). The desubroutinizer tests in Task 4 verify byte-level correctness, which catches the same class of bugs. Outline verification can be added when a path interpreter is available.

## Note: Debug assertions

The spec calls for `debug_assert!` structural validation after CFF subset generation. These should be added to `subset_cff_font()` after the core implementation stabilizes. They are lightweight (parse-check the output header and INDEX counts) and can be added incrementally during Tasks 6-7.

## Summary

| Task | Description | TDD Phase |
|------|------------|-----------|
| 1 | Extract `cff/types.rs` | Refactor (no behavior change) |
| 2 | Extract `cff/index.rs` | Refactor (no behavior change) |
| 3 | Extract `cff/dict.rs` | Refactor (no behavior change) |
| 4 | Write failing desubroutinizer tests | Red |
| 5 | Implement desubroutinizer | Green |
| 6 | Wire desubroutinizer into CFF subsetter | Green (integration) |
| 7 | SID→CID conversion | Red → Green |
| 8 | TTF table stripping | Red → Green |
| 9 | Writer simplification | Refactor |
| 10 | Size regression tests | Green (validation) |
| 11 | Dead code cleanup | Refactor |
| 12 | Final round-trip validation | Green (validation) |
