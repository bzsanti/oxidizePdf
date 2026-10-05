# Content differential protocol — anchored-content-v1

This is a per-page disagreement measurement against Poppler, not a normative
PDF oracle or a character-accuracy score. It complements the existing fusion
and order gates without reading or changing their baselines.

The extractor uses lenient parsing and the default flat TextExtractor, with an
explicit output budget. An error, panic, timeout or truncated output is never
reported as successful empty text. Successful empty text is comparable and
counts as deletion of all reference content. Page errors stay in the report.
If page counts disagree, the entire file's page alignment is unproven: none of
its pages contribute to character totals. No attempt is made to shift indices.

Content consists of Unicode scalars excluding Python `str.isspace()` characters.
Whitespace counts are reported separately; this protocol does not measure
inserted spaces. Case, punctuation, Unicode normalization, combining marks and
replacement characters are preserved. For a non-equal SequenceMatcher span,
`min(reference_length, candidate_length)` counts as substitutions; the remaining
reference/candidate lengths count as deletions/insertions. `autojunk=False` is
required. This is an anchored alignment, **not minimum Levenshtein distance**.
Reordered material can appear as deletions and insertions. The additional
per-page scalar-multiset deficit is order-independent, but can hide a missing
word if the same characters occur elsewhere. Neither signal alone proves loss
of PDF glyphs. Use both alongside the existing order gate and page diagnostics.

All rates use reference content on comparable pages as their denominator. A
zero denominator yields null. File/page statuses remain separate; excluding
more files or pages is never evidence of improvement. A manifest/hash change
or absence of comparable pages makes the command fail. Nonzero discrepancies
do not automatically fail this observational report: it has no calibrated
accuracy threshold. Introducing a future ratchet requires a separately
reviewed population/coverage policy, not a change to historical gates.

## Reproduce from the repository root

Use Python 3.11+, Linux with GNU `timeout`, and Poppler `pdftotext`. No Python
package or font installed on the host is used by the candidate. Capture Python,
Unicode database, Poppler, Rust versions and source/binary hashes with evidence;
the first execution's environment is recorded in
`docs/reports/2026-10-05-t4-content-evidence/environment.json`.

```sh
cargo build --locked --offline -p oxidize-pdf --example text_content_probe
python3 -m unittest discover -s tools/tests -p test_content_differential.py -v
python3 tools/text_contracts/content_differential.py freeze test-corpus/t3-stress --manifest target/content-population.json
python3 tools/text_contracts/content_differential.py run test-corpus/t3-stress --manifest target/content-population.json --probe target/debug/examples/text_content_probe --output target/content-pages.jsonl --work target/content-work --jobs 4
```

Population/output files are created exclusively: use new output names when
repeating a measurement and reuse the frozen manifest. Keep both attempts when
diagnosing a defect. Every selected PDF has a relative path and SHA-256 checked
before and after processing. Duplicate/escaping paths are rejected. The runner,
probe and manifest must remain unchanged throughout the measurement.

Each JSONL record identifies a file and its page statuses. Compared pages carry
counts and up to 32 edit previews, with scalar offsets in the whitespace-filtered
strings. Previews are truncated to 48 scalars; counts are complete. The companion
`.summary.json` records population/status totals, hashes and comparator version.
Warnings are retained even when a comparator returns success.

Resource policy: 128 MiB input PDFs, 10,000 pages per candidate, 32 MiB candidate
text per file, 64 MiB per worker output file, 2 GiB worker address space, 20 seconds
per extraction command, 45 seconds per complete worker (including alignment),
100,000 content scalars per compared page. Worker timeouts kill the process group.
Limit failures are explicit exclusions, never zero measurements. The command
uses file-backed temporary output under the requested work directory.

The reusable copied-source mutation runner is
`docs/reports/2026-10-05-t4-content-evidence/run-mutations.py`: the control must
pass and all six variants must fail through test assertions. It changes only
disposable copies below `target/`, not the reviewed candidate or corpus.
