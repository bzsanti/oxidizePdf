# Text contracts (#666)

## Deterministic suite and CI

From the repository root, run Python 3.11+ with an installed Rust toolchain:

```sh
python3 tools/text_contracts/run_fast.py --profile default --output target/contracts-default
python3 tools/text_contracts/run_fast.py --profile minimal --output target/contracts-minimal
python3 tools/text_contracts/run_fast.py --profile spi --output target/contracts-spi
python3 tools/text_contracts/run_fast.py --profile default --toolchain 1.88 --output target/contracts-msrv
```

Use a fresh output directory for each invocation; `--offline` is available when
Cargo dependencies are cached. The runner executes traceability and tooling
checks, builds the actual library, then runs all manifest-declared targets and
`text_*_contract_test` targets. SPI also runs `analysis_spi_test`. A failed step
fails the invocation; `result.json` records commands, exit codes and completion.
No external reader, source download or host font is needed by this fast suite.

`minimal` means `--no-default-features --features compression`, the supported
minimum for this text suite, not an assertion that every arbitrary feature
combination works. `spi` enables `unstable-spi,semantic` with defaults. Library
builds precede tests to avoid hiding missing dependencies behind dev-dependencies.
The Text contracts workflow runs all three profiles on Linux, Windows and macOS,
plus all three on Linux/MSRV1.88. Configuration alone is not a passing remote run.

The Corpus Tests workflow measures fusion/order and the separate content signal
nightly and by `workflow_dispatch`. It preserves the run's frozen population,
per-page diagnostics and summary as artifacts. The observational content signal
has no calibrated acceptance threshold; historical gates keep their own ratchets.

## External readers

This is an offline validation tool, separate from the library and Rust tests.
It compares 29 pinned, single-page reference PDFs from the durable
investigation evidence. It is not the complete #666 matrix or corpus gate.

- Expectations come from explicit fixture ToUnicode and the pinned MacExpert/AGL
  references. They are written in cases.json, never generated from reader output.
- MuPDF is the primary independent extraction comparison: PyMuPDF 1.26.5,
  embedded MuPDF 1.26.10. Both versions are checked at runtime.
- Poppler is a secondary comparison. Known behavior is accepted only for the
  exact case, version, exit code, stdout and stderr recorded in the manifest.
- qpdf checks structure. Its success does not certify text correctness.

## Setup and execution

Install the validation dependencies into a separate environment; no changes to
Cargo.toml or product dependencies are needed. Install qpdf and pdftotext with
your environment's package manager, then:

```sh
uv venv /tmp/oxidize-text-readers
uv pip install --python /tmp/oxidize-text-readers/bin/python -r tools/text_contracts/requirements.txt
/tmp/oxidize-text-readers/bin/python tools/text_contracts/validate_readers.py \
  --output /tmp/text-readers-report.json
/tmp/oxidize-text-readers/bin/python -m unittest discover -s tools/text_contracts/tests -v
```

The already prepared investigation environment can also run it with
`PYTHONPATH=/tmp/issue666-pymupdf python3 ...`. This path is optional, not embedded
in the validator. All manifest PDF paths are relative to the repository; fixtures
must match their SHA256 before any reader is invoked. The files are referenced
in place under docs/reports to avoid duplicating embedded font binaries.

## Exact comparisons and exit codes

The manifest contains single-page PDFs. `expected_text` explicitly includes all
internal line breaks, including vertical A/B/A fixtures.
MuPDF's expected transport is `expected_text + LF`; pdftotext's is
`expected_text + LF + FF`. The tool compares complete outputs without strip,
Unicode normalization, whitespace folding, ligature expansion or replacement
character suppression. Multi-page cases need an explicit schema extension,
not a generic trim. MuPDF warnings make its comparison fail.

Reports retain versions, fixture hashes, expected text, reader outputs, warnings,
process exit codes, per-reader status and known-limit reasons. Exit 0 means all
cases matched or had only the exact documented secondary-reader limitation, and
all structural checks passed. Exit 1 means unexpected content, changed/missing
fixture, missing reader, unreviewed MuPDF version or another failure.
`accepted` does not mean every reader agreed. Inspect `known_limitation` separately.

Poppler 24.02.0's numeric ToUnicode limits and ligature decomposition are recorded
in cases.json; see docs/reports/2026-10-01-issue-666-poppler-discrepancy.md. An
updated Poppler that returns the exact expected text passes without an exception;
old divergences from a different version require investigation. There is no
option to learn/rebaseline expectations or automatically broaden exceptions.

## Extending the manifest

Add an independently specified fixture, hash, expected text and its source.
Investigate every new disagreement before recording any known limitation.
The primary MuPDF comparison cannot be bypassed by a Poppler exception. Review
reader upgrades against all cases and adversarial tests before updating pins.
This describes the initial RED baseline. Subsequent fixes and current local validation are recorded in `docs/reports/2026-10-04-issue-666-qr-fixes.md`; this tooling does not certify CI or completion of the contract matrix.


## Vertical geometry

Cases can include `expected_trace_origins`: an ordered list of Unicode character
and [x,y] glyph origins from an independent metric calculation, with 0.001-point
tolerance. These coordinates use MuPDF page space (y downward). The validator
compares `get_texttrace()` origins, not rawdict's text-layout origins, which differ
in vertical mode. No geometry is claimed when this optional field is absent.

For vertical metrics without horizontal scaling, glyph origin0 is
`(pen_x - vx * font_size / 1000, page_height - pen_y + vy * font_size / 1000)`.
The Rust tests assert text pen positions separately. Nine vertical fixtures check
these glyph origins; two Tz fixtures compare text externally and pen positions
in Rust, without claiming external validation of their scaled glyph offsets.
A correct text result with an incorrect geometry result fails the gate.


CIDFontType0 fixtures use an original, independently generated CID-keyed CFF,
not a name-keyed Type1C or a renamed TrueType font. ROS, FDArray, FDSelect and
charset are round-trip verified by FontTools. Explicit CIDs differ from GIDs.
Their four expected text/geometry cases are declared in the manifest; no reader
exception is needed. This does not yet cover Adobe CJK collections or multiple FDs.

## Per-page corpus content measurement

The separate [anchored-content-v1 protocol](CONTENT_DIFFERENTIAL.md) measures
substitutions, deletions, insertions and multiset deficits against Poppler on a
hash-frozen corpus. It preserves errors/exclusions and does not recalibrate the
existing fusion/order gates.
