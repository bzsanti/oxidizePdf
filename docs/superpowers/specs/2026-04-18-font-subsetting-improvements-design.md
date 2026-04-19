# Font Subsetting Improvements — Design Spec

**Date**: 2026-04-18
**Issue**: #165 (font subsetter doesn't work)
**Version target**: v2.6.0

## Problem Statement

Font subsetting in oxidize-pdf produces output that is significantly larger than comparable libraries and has rendering bugs with CJK fonts:

| Metric | oxidize-pdf | krilla | Gap |
|--------|-------------|--------|-----|
| OTF (CFF) ~67 chars from 15.7MB font | 187KB | 59KB | 3.2x |
| TTF ~67 chars from 12.7MB font | 666KB | 48KB | 13.9x |
| CJK character rendering | 3 missing, 1 incorrect | Correct | Bug |

Root causes identified through analysis of `typst/subsetter` and `LaurenzV/krilla`:

1. **TTF**: Unnecessary tables embedded (cmap, OS/2, name) that PDF doesn't need
2. **CFF non-CID**: Full OTF wrapper embedded instead of raw CFF bytes
3. **CFF**: Local/Global Subrs preserved with stubs instead of desubroutinized
4. **CFF SID-keyed**: Separate code path with OTF wrapper, instead of converting to CID-keyed

## Design Principles

- **No new dependencies** — learn from subsetter/krilla algorithms, implement in our codebase
- **MSRV 1.77 preserved** — no Rust version bump needed
- **Single implementation** — no runtime fallbacks or dual code paths
- **Guardrails via tests** — structural validation and outline verification in CI, not runtime

## Section 1: Table Stripping

### TTF Subsetting

**Current tables** (in `build_font_file`, `truetype_subsetter.rs:756-781`):
cmap, glyf, head, hhea, hmtx, loca, maxp, name, post, OS/2

**New tables**:
glyf, head, hhea, hmtx, loca, maxp, post

**Removed**:
- `cmap` — PDF uses its own ToUnicode CMap (generated in `generate_tounicode_cmap_from_font`). The font-embedded cmap is never read by PDF viewers for CID fonts with Identity-H encoding.
- `OS/2` — Not required by ISO 32000-1 for embedded fonts. Font descriptor fields (Ascent, Descent, CapHeight, StemV) are already written separately in the PDF.
- `name` — Not required for rendering. krilla strips it entirely; subsetter keeps only copyright records. We strip it entirely — simpler, and the font name is already in the PDF font dictionary.

**Impact**: For CJK fonts, cmap alone can be 50-100KB+. Combined savings estimated at 30-60% of current TTF subset size.

### CFF Subsetting

**Current behavior** (`cff_subsetter.rs:316-331`):
- CID-keyed: returns raw CFF bytes (`is_raw_cff: true`) — correct
- Non-CID (SID-keyed): wraps in OTF with tables: CFF, head, maxp, hhea, hmtx, cmap

**New behavior**:
All CFF fonts return raw CFF bytes with `is_raw_cff: true`. SID-keyed fonts are converted to CID-keyed (see Section 3). The `rebuild_subset()` method and its OTF wrapper construction are removed.

### Changes to `build_font_file` (TTF)

File: `oxidize-pdf-core/src/text/fonts/truetype_subsetter.rs`

Remove these lines from `build_font_file()`:
```rust
// REMOVE:
let name_table = self.get_table_data(b"name")?;
let os2_table = self.get_table_data(b"OS/2").ok();
tables_to_write.push((b"cmap", cmap));
tables_to_write.push((b"name", name_table));
if let Some(os2) = os2_table {
    tables_to_write.push((b"OS/2", os2));
}
```

The `cmap` parameter to `build_font_file()` is also removed, along with `build_cmap_table()`, `build_cmap_format4()`, and `build_cmap_format12()` methods (~200 lines eliminated).

The `build_subset_font()` method no longer calls `self.build_cmap_table(new_cmap)`.

## Section 2: CFF Desubroutinization

### Current approach

`cff_subsetter.rs` preserves Local Subr INDEXes, replacing unused subroutines with 1-byte `endchar` stubs. Global Subr INDEX is filtered transitively. This was the v2.5.1 fix that reduced CID CFF from ~1MB to ~150KB.

### New approach: Full desubroutinization

For each charstring in the subset:
1. Parse the charstring byte stream
2. When encountering `callsubr` (10) or `callgsubr` (29): pop the biased index from the argument stack, look up the subroutine body, recursively inline its instructions
3. When encountering `return` (11): stop processing current subroutine (return to caller)
4. All other operators and operands: copy to output
5. Maximum recursion depth: 64 (CFF spec limit)

After desubroutinization:
- Write Global Subr INDEX with count=0 (just the 2-byte header `00 00`)
- Write Private DICTs without `Subrs` offset operator (opcode 19)
- Each charstring is self-contained — no external references

### Desubroutinizer design

New struct `CharstringDesubroutinizer` (or integrated into existing charstring processing):

```
Input:
  - charstring: &[u8]           — raw charstring bytes
  - global_subrs: &CffIndex     — parsed Global Subr INDEX
  - local_subrs: &CffIndex      — parsed Local Subr INDEX for this font dict
  - depth: u8                   — current recursion depth (start at 0)

Output:
  - Vec<u8>                     — desubroutinized charstring bytes

Algorithm:
  offset = 0
  while offset < charstring.len():
    byte = charstring[offset]

    if byte is operand (number encoding):
      copy raw encoded bytes to output (do NOT re-encode — preserve original byte representation)
      push decoded value to arg stack (needed for callsubr/callgsubr bias calculation)
      advance offset past the encoded number

    else if byte == 10 (callsubr):
      if depth >= 64: error
      biased_index = pop arg stack
      REMOVE the last encoded number from output (it was the subr index, not glyph data)
      actual_index = biased_index + bias(local_subrs.count)
      subr_bytes = local_subrs.get(actual_index)
      recursively desubroutinize(subr_bytes, depth + 1)
      append result to output
      advance offset by 1

    else if byte == 29 (callgsubr):
      if depth >= 64: error
      biased_index = pop arg stack
      REMOVE the last encoded number from output (it was the subr index, not glyph data)
      actual_index = biased_index + bias(global_subrs.count)
      subr_bytes = global_subrs.get(actual_index)
      recursively desubroutinize(subr_bytes, depth + 1)
      append result to output
      advance offset by 1

    else if byte == 11 (return):
      stop (do not copy return op)

    else:
      copy operator byte(s) to output
      clear arg stack (operators consume args)
      advance offset

  Bias calculation (per CFF spec):
    count < 1240:   bias = 107
    count < 33900:  bias = 1131
    else:           bias = 32768
```

### Why desubroutinization is better for small subsets

For a font with 65K glyphs subsetted to 67 characters:
- Most subroutines are referenced by only 1-2 of the 67 kept charstrings
- The subroutine INDEX overhead (count + offsets + call/return ops) exceeds the byte savings from sharing
- Desubroutinization eliminates: entire Global Subr INDEX, all Local Subr INDEXes, all call/return operators
- Net result: larger individual charstrings but dramatically smaller total because thousands of unreferenced subroutines disappear

### Files affected

- `cff_subsetter.rs`: Add desubroutinizer, remove Local Subr stub-replacement logic and Global Subr transitive filtering
- The existing CFF INDEX parser (`parse_cff_index`) is reused to read subroutine INDEXes

## Section 3: SID-keyed to CID-keyed Conversion

### Current behavior

`cff_subsetter.rs:316-331` bifurcates:
- CID-keyed (has FDArray/FDSelect): returns raw CFF
- SID-keyed (no FDArray/FDSelect): wraps in OTF via `rebuild_subset()`

### New behavior: Always CID-keyed

For SID-keyed fonts, generate CID-keyed structure:

1. **FDArray**: Single font dict (index 0) containing:
   - Private DICT with the original font's private dict values (defaultWidthX, nominalWidthX, BlueValues, etc.)
   - No Subrs offset (desubroutinized)

2. **FDSelect**: Format 0 — one byte per glyph, all mapping to FD index 0
   - Format: `[format=0] [fd_index=0] * num_glyphs`
   - Simplest format, one byte per glyph

3. **Charset**: Format 2 (range-based) with identity mapping
   - `[format=2] [first_sid=1] [n_left=num_glyphs-2]`
   - Maps GID n to CID n (identity)

4. **Top DICT**: Updated with:
   - `ROS` operator: (Adobe, Identity, 0) — marks as CID-keyed
   - `FDArray` offset
   - `FDSelect` offset
   - `charset` offset
   - Remove `Encoding` operator (CID fonts don't use it)
   - Remove individual `Private` offset (it's in FDArray now)

5. **String INDEX**: Minimal — only strings needed for ROS ("Adobe", "Identity") if not already in standard strings. Both "Adobe" (SID 394 → actually standard SID 391? — verify during implementation) and "Identity" need checking against the standard string list.

### Writer changes

File: `oxidize-pdf-core/src/writer/pdf_writer/mod.rs`

The `is_raw_cff` flag is always `true` for CFF fonts after this change. The writer code at lines 1391-1407 simplifies:
- Remove the `if embed_as_raw_cff { ... } else { ... }` branch
- CFF fonts always use `/Subtype /CIDFontType0C` and `FontFile3`
- The `CjkFontType::should_use_cidfonttype2()` check at line 1452 simplifies: CFF → always CIDFontType0, TTF → always CIDFontType2

## Section 4: Guardrails

### Structural validation (debug assertions)

After generating a CFF subset, validate in debug builds:
- Parse the output as CFF: header, Name INDEX, Top DICT INDEX, String INDEX, Global Subr INDEX
- Verify CharStrings INDEX has expected glyph count
- Verify each charstring terminates with `endchar` (14) or valid endpoint
- Verify FDArray/FDSelect present and consistent

These are `debug_assert!` — zero cost in release builds, catch structural bugs during development.

### Outline verification tests

For each font in the test corpus:
1. Parse original font, extract path points for each used glyph (resolve subroutines to get flat path)
2. Subset the font
3. Parse the subset, extract path points for each glyph
4. Compare: same number of path segments, same coordinates (within f32 epsilon)

This catches:
- Desubroutinization errors (wrong subroutine inlined, incorrect bias)
- SID→CID conversion errors (wrong glyph mapped to wrong CID)
- Missing glyphs (the exact bug reported in #165)

### Test corpus

Existing:
- UDHR multilingual PDFs (Chinese, Japanese, Korean, Arabic, Hebrew) in `tests/fixtures/multilingual/`
- Various TTF/OTF fonts already used in existing tests

New:
- SourceHanSansSC-Regular.otf (or subset) — the exact font from the issue reporter
- A SID-keyed CFF font (e.g., LatinModernRoman) to exercise SID→CID conversion
- Targeted test with the specific characters reported as missing/incorrect

### Size regression tests

For key fonts, assert that subset size is below a threshold:
- SourceHanSansSC with 67 chars: assert < 80KB (currently 187KB, target ~60KB)
- TTF equivalent: assert < 80KB (currently 666KB, target ~50KB)

Thresholds are generous (not trying to match krilla exactly) but prevent regressions.

## Code Organization

### Current state (monolithic)

- `truetype_subsetter.rs`: 1,933 lines — TTF subsetting + cmap building + all TTF table manipulation
- `cff_subsetter.rs`: 3,010 lines — CFF parsing + subsetting + OTF wrapper + Local Subr handling

### Proposed modularization

Following subsetter's pattern of focused modules, but adapted to our existing structure:

```
src/text/fonts/
  truetype_subsetter.rs    — TTF subset orchestration (reduced from 1933 to ~800 lines)
  cff_subsetter.rs         — CFF subset orchestration (reduced from 3010 to ~600 lines)
  cff/
    mod.rs                 — Re-exports
    index.rs               — CFF INDEX parsing and creation
    dict.rs                — DICT parsing and serialization (Top, Private, Font)
    charstring.rs          — Charstring desubroutinizer
    sid.rs                 — SID/String INDEX handling
    types.rs               — Number encoding (Integer, Real), operator types
```

The TTF subsetter stays as one file (it's simpler — just table manipulation). The CFF subsetter splits because CFF has more internal structure (INDEXes, DICTs, charstrings, SID system, FD management).

**Note**: Existing code from `cff_subsetter.rs` is refactored into modules, not rewritten from scratch. The parsing logic for CFF INDEXes, DICTs, and charstrings already works — it just needs to be reorganized and the desubroutinizer added.

## Migration Strategy

This is a single feature branch. The changes are interdependent (desubroutinization requires understanding the CFF module split, SID→CID conversion changes what `is_raw_cff` means). Attempting to ship them separately would require intermediate states that add complexity without value.

**Order of implementation within the branch:**

1. **Modularize CFF** — extract existing code into `cff/` modules without changing behavior. All existing tests must pass.
2. **Implement desubroutinizer** — add `cff/charstring.rs` with the inlining algorithm. Wire it in. Verify with outline comparison tests.
3. **SID→CID conversion** — unify the CFF output path. Remove `rebuild_subset()`. All CFF returns `is_raw_cff: true`.
4. **TTF table stripping** — remove cmap/OS/2/name from `build_font_file()`. Remove cmap building methods.
5. **Writer simplification** — remove dead branches in `write_type0_font_from_font()`.
6. **Add size regression tests and outline verification tests.**

## Out of Scope

- **W array optimization** (grouping consecutive same-width glyphs into ranges) — good improvement but independent of subsetting. Can be a separate PR.
- **CIDToGIDMap Identity optimization** (Name instead of stream for TTF) — depends on verifying our remapper produces consecutive GIDs from 0. Separate PR.
- **Compression improvements** (zlib-rs backend) — orthogonal, separate PR.
- **name table reduction** (keep only copyright) vs full removal — we chose full removal for simplicity. If a viewer complains, we can add back minimal records.
- **hmtx advance width deduplication** — subsetter does this, we don't. Minor size impact, separate PR.
