# Issue #690: inherited zero-offset cross-reference recovery

## Executive summary

Issue: https://github.com/bzsanti/oxidizePdf/issues/690

Candidate: `fix/issue-690-unused-xref`, based on develop
`592f771734d15163f6a61467cbb4e994dca5db95`. Exact reviewed source hashes are in
[the evidence directory](2026-10-08-issue-690-xref-evidence/source-hashes.json).
The existing clean checkout was reused; unrelated root-workspace changes were
preserved. This report covers the correction requested after published 5.4.0
passed the original content-preserving workflow but left four invalid xref
entries in the prepared and split outputs.

The editor now retires only unreadable, uncompressed in-use entries at offset
zero that are unreachable from the complete current trailer graph. Derived
ParentTree/IDTree roots still receive their existing exact-generation recovery;
they are never retired. Catalog, trailer, nested and wrong-generation aliases
remain errors. Missing object-stream containers are rejected rather than
silently retiring their members. The source prefix is unchanged.

Retirement is an incremental free xref entry at generation 65535 pointing to
object zero. This uses the second mechanism in ISO 32000-1 §7.5.4, which permits
permanently free entries outside the reusable free list; the existing head and
links remain untouched. See [PDF 32000-1:2008](https://vanillapdf.github.io/PDF32000_2008.pdf).
Both classic tables and xref streams are supported. No public types, enum
variants, dependencies or validation exceptions were added. Preparation and
page-mutation inventories include the retired source identities; their docs
explain the xref-state change.

## Contracts and evidence

| Contract / source | Producer and consumer | Counterexample / evidence | Status |
| --- | --- | --- | --- |
| Clean xref after preserving preparation, #690 follow-up | Reader current xref → recovery → incremental writer → qpdf | Public 938-byte fixture has absent roots 6/7 and unused 10; baseline qpdf exit 3. Candidate explicitly frees only 10 and reconstructs 6/7. | Demonstrated on synthetic and original outputs |
| Preserve source bytes, content, tags and source page hierarchy | Public editor → public split → reader/Poppler | Native object identity checks; synthetic 553 associations/12 pages and 3/3/6 outputs, prefix and text checks; external byte-identical text/pixels at 72 dpi for direct and prepared splits. | Demonstrated on synthetic and original outputs |
| Reject reachable corruption and preserve exact root-edge exemptions | Complete trailer graph plus existing catalog walk | Initial RED accepted `/Info 10 0 R`; candidate rejects direct/nested Info, catalog references, aliases to recovered roots, and wrong generations. | Demonstrated |
| Preserve latest revision and free list | Merged reader xref → incremental table/stream writer | Tables, streams, hybrids, repeated preparation, latest valid nonzero-generation definition, existing free list and saturated retirement generations. | Demonstrated |
| Do not hide unrelated corruption or reinterpret compressed objects | Storage-kind check and unchanged permission policy | Missing object-stream container rejected; unrelated invalid nonzero offset still fails the existing DocMDP inspection. | Demonstrated |
| Prevent two xref states for one object number | Shared incremental writer | Private unit regression rejects retirement/replacement conflicts in both orders, duplicate retirement, object zero and out-of-range IDs; allocation skips retired identities. | Demonstrated in full suite |
| Existing permission, metadata and publication guarantees | Existing tagged preparation/split contracts | Prior 39 tagged tests pass, including 48 structural/metadata combinations; no permissions or validation gates weakened. | Full workspace passed |
| Original private input | Existing external public-API consumer with previously authorized values | Existing pre-fix prepared file has four qpdf warnings. Candidate diagnoses all three missing values, uses previously authorized metadata and produces 3/3/6 pages; four outputs pass qpdf and 12/12 text/raster comparisons pass. | Demonstrated locally |
| Final workspace, lint, platforms and delivery | Cargo/Clippy/CI | Full workspace: 10,430 passed, 0 failed, 71 existing ignored across 418 summaries; Clippy all-targets/internal-testing with warnings denied and format pass; candidate CI not yet run. | Local checks passed; CI pending |

The initial three-test run was RED (1 pass, 2 failures): absent retirement
inventory and acceptance of a missing `/Info` object. The same three tests then
passed. The expanded suite has 10 passing public tests. During test expansion,
a negative test initially expected nonzero corrupt offsets to be left unchanged;
the actual existing permission inspection already rejects them. The test was
corrected to require that rejection, without weakening product behavior.

Public fixture provenance: [consumer reproduction](https://github.com/bzsanti/oxidizePdf/issues/690#issuecomment-6057868240).
No private PDF, text, descriptions or rendered pages are included in this change.
External artifacts stay under ignored `target`; the verifier records only
commands/results and equality outcomes. Historical benchmark results and
thresholds are unchanged; this is not a performance measurement.

## Findings

1. **Inherited live zero-offset entries survived recovery** —
   `operations/reorder.rs:377` and `writer/incremental_update.rs:101`: only
   derived indexes were rewritten, so independent readers still diagnosed
   unrelated absent objects. Corrected with bounded candidate selection,
   proven unreachability and explicit permanent free entries in the appended
   xref, including direct tagged split materialization.
2. **Catalog-only reachability cannot establish safe retirement** —
   `operations/reorder.rs:1302`: a missing object may be required through
   `/Info` or another trailer value. Corrected by walking the entire current
   trailer before retirement; exact root-edge exemptions remain narrow.
3. **Offset zero alone does not identify an uncompressed absent object** —
   `parser/reader.rs:177` and `operations/reorder.rs:1324`: compressed members
   can inherit their container's storage offset. The candidate checks storage
   kind and rejects a missing container instead of freeing its members.
4. **Retirement must not conflict with a replacement** —
   `writer/incremental_update.rs:81`: the shared writer now rejects conflicting
   states for the same object number, even across generations. Explicit unit
   coverage exercises both call orders and allocation boundaries.

All four points belong to safe implementation of this issue's requested xref
correction. No confirmed unresolved code finding remains in the reviewed scope;
Local review, Clippy and original-input validation are complete; delivery remains pending. Published-candidate Studio acceptance is still separate.

## Test quality (Kripteia)

Both required analyses ran through the quality-review skill script. Results:
reorder 97/100 (10 inline tests), incremental writer 100/100 (1), reader 95/100
(93), new public contracts 98/100 (10). The new helper's coverage comes from the
public contracts, not the pre-existing inline reorder score. Scanner comments
about low assertion/setup ratios, multiple assertions and `unwrap` were checked:
they are fixture setup and explicit failure expectations, and unwrap retains
Rust's error/panic information. Fixed object numbers and free-list pointers are
independent oracles, not values copied from the implementation. The external
Python verifier reports 100/100 with 0 tests/0 files recognized by the scanner;
its security scan reports no issues. It is supplementary executable evidence,
not a unit-test suite; its successful full run is recorded in external-final.json.
See `kripteia-*.log` in the evidence directory.

## Security analysis (Kripteia Security)

The analyses of reorder, incremental writer, reader and public tests each report
`No security issues found`. Manual review also traced certification/encryption
checks before recovery, complete trailer reachability, exact generations,
compressed-object ownership, existing graph limits and transactional publication.
No crypto, unsafe, FFI or subprocess invocation was added to product code. The
external verifier invokes fixed tools with argument arrays and never publishes
private contents. Clean scanner output does not establish PDF/UA certification.

## Metrics and remaining validation

- Product files changed: 4; one is rustdoc-only and the reader change is a private accessor.
- Public contract files: 1 added, 1 updated for the expanded inventory; one private writer regression added.
- Public synthetic fixture: 1; dependency/lockfile changes: 0.
- Independent results: 19 clean qpdf outputs (15 synthetic plus original preparation and three parts); 12/12 texts and rasters identical in each synthetic workflow and the original-input workflow.
- Full workspace: 10,430 passed, 0 failed, 71 existing ignored; format check passed.
- Clippy workspace/all-targets/internal-testing with warnings denied: passed.
- Delivery, candidate CI and published-candidate Studio acceptance: pending.
