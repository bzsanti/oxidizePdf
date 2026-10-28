# Structural OmniDocBench evaluation

Issue [#581](https://github.com/bzsanti/oxidizePdf/issues/581) measures native
`PdfDocument::partition` output separately from native text and reading order.
It does not add OCR or improve the partitioner to fit the annotations.

The dataset revision is `f5f559bddf50e36f7f9899d842d0006f13ce8afc` (981 pages).
The official evaluator revision is `337cc26965893db3ef53ddc119a6d6bb5bde096f`.
Use the original one-page PDFs in `ori_pdfs`, not rasterized replacements.

## Prediction contracts

`omnidocbench_structural_jobs` reads only image names and dimensions from the
annotations. It resolves each original PDF, sets page index zero, rejects missing
files and duplicate names, and sorts the manifest. Annotation content, categories
and boxes are not supplied to the Rust exporter.

`omnidocbench_structural_export` calls the public partition API. Its Markdown
contains only detected `Element::Table` structures, with escaped cell contents,
header cells and row/column spans. Paragraphs and code blocks cannot introduce
tables merely because they contain HTML or Markdown syntax. A page without a
detected table still has an empty prediction file.

The layout JSON includes every partition element. Titles map to `title`, textual
elements to `plain text`, headers/footers to `abandon`, images to `figure`, and
tables to `table`. Confidence comes from element metadata. Boxes are transformed
from PDF user coordinates through the CropBox (or MediaBox), including its origin
and clockwise page rotation, into the annotated image dimensions. Categories
the API cannot distinguish are not invented. The official ten-category layout
evaluation still includes those categories and penalizes missing detections.

An extraction error produces an empty page prediction and an entry in the report;
it does not remove the page from evaluation. The output directory is published
with a single rename after all files are ready. An existing destination is rejected.

The report includes hashes of the consumed jobs, predictions, layout, executable,
and adapter/lockfile bytes embedded at compilation. `runtime_checkout_git_sha`
describes the checkout at execution, not the binary's build revision. Retain a
clean source commit, the build command and input PDF hashes alongside the report.

## Reproduction

Prepare a clean checkout of this repository and the pinned evaluator with its
requirements installed. The evaluator's Python environment must provide PyYAML.
The commands below write fresh outputs and never modify historical benchmark data.

```bash
set -euo pipefail
test -z "$(git status --porcelain)"
export DATASET=/absolute/path/to/omnidocbench-f5f559bddf50e36f7f9899d842d0006f13ce8afc
export EVALUATOR=/absolute/path/to/OmniDocBench
export EVAL_PYTHON=/absolute/path/to/evaluator-environment/bin/python
export ARTIFACTS=/absolute/path/to/new-structural-run
export SOURCE="$PWD"

test "$(git -C "$DATASET" rev-parse HEAD)" = f5f559bddf50e36f7f9899d842d0006f13ce8afc
test "$(git -C "$EVALUATOR" rev-parse HEAD)" = 337cc26965893db3ef53ddc119a6d6bb5bde096f
git -C "$EVALUATOR" diff --exit-code HEAD
mkdir "$ARTIFACTS"
cargo build --locked -p oxidize-pdf \
  --example omnidocbench_structural_jobs --example omnidocbench_structural_export
cargo run --locked -p oxidize-pdf --example omnidocbench_structural_jobs -- \
  "$DATASET" "$ARTIFACTS/jobs.json"
cargo run --locked -p oxidize-pdf --example omnidocbench_structural_export -- \
  "$ARTIFACTS/jobs.json" "$ARTIFACTS/export"

"$EVAL_PYTHON" - <<'PY'
import os
from pathlib import Path
import yaml

root = Path(os.environ['SOURCE'])
out = Path(os.environ['ARTIFACTS'])
for name, task, prediction in [
    ('structural', 'end2end_eval', out / 'export/predictions'),
    ('layout', 'detection_eval', out / 'export/layout.json'),
]:
    config = yaml.safe_load((root / f'tools/benchmarks/fixtures/omnidocbench-v1-{name}.yaml').read_text())
    dataset = config[task]['dataset']
    dataset['ground_truth']['data_path'] = str(Path(os.environ['DATASET']) / 'OmniDocBench.json')
    dataset['prediction']['data_path'] = str(prediction)
    (out / f'{name}.yaml').write_text(yaml.safe_dump(config, sort_keys=False))
PY

mkdir -p "$ARTIFACTS/evaluation/result"
cd "$ARTIFACTS/evaluation"
PYTHONDONTWRITEBYTECODE=1 "$EVAL_PYTHON" "$EVALUATOR/pdf_validation.py" \
  --config "$ARTIFACTS/structural.yaml" > "$ARTIFACTS/teds.log" 2>&1
PYTHONDONTWRITEBYTECODE=1 "$EVAL_PYTHON" "$EVALUATOR/pdf_validation.py" \
  --config "$ARTIFACTS/layout.yaml" > "$ARTIFACTS/layout.log" 2>&1
"$EVAL_PYTHON" -m pip freeze > "$ARTIFACTS/evaluator-packages.txt"
```

Retain the generated YAML, jobs and input hashes, export bundle, full logs, and
`evaluation/result/`. The official TEDS output contains per-table observations
with page IDs plus aggregate/group results. The official detection task prints
its COCO metrics to `layout.log`; it does not create a per-page AP report.
`layout.json` retains all per-page predictions, and pages without detections
remain in the annotation population.

## Interpreting results

TEDS measures matched annotated tables. The pinned evaluator removes unmatched
extra predictions; its TEDS result is not a false-positive or table-detection
score. Layout COCO mAP separately evaluates detections, including missing
categories. Report both with their populations and protocol revision.

The upstream end-to-end report requires companion text and reading-order metrics.
They operate on table-only predictions here and are **not native-text quality
results**. Do not publish or compare them with the separate native-text benchmark.
The existing native exporter, serialization contract and official configuration
are unchanged.

Formula CDM is not applicable: the public partition API exposes text, tables,
images and typed elements, but no formula recognition or LaTeX representation.
Literal PDF glyphs are not a mathematical-expression representation. No formula
strings are guessed and no CDM score is claimed. This does not establish that
formula recognition is impossible; it records the current API boundary.

## Validation

The OmniDocBench CI contract executes both examples' tests. Local acceptance also
uses hand-built PDFs with displaced CropBoxes and all four rotations, comparing
the transformed rectangle with Poppler raster bounds. A geometry mutation must
fail that independent comparison. Test results do not replace the full official
evaluation or justify excluding pages that fail extraction.

## Recorded run: 2026-10-25

Core base: `6090294f4ab619160107b85f962d5f09efabfc3c` (develop, crate version
5.5.2), with the new adapters identified by their compiled hashes in the
[run manifest](benchmarks/omnidocbench-structural-2026-10-25.json).
This is a development measurement, not a claim about the published 5.5.2 package.

| Metric | Result | Population |
| --- | ---: | --- |
| Official TEDS | 0.0740509749 | 428 annotated tables across 317 pages |
| Official TEDS, structure only | 0.1414761409 | Same 428 tables |
| Official layout COCO mAP | 0.0120838539 | 981 pages, ten categories |
| Official layout AP50 | 0.0246605104 | Same layout population |

The export contains 38,938 boxes on 768 pages; all 213 pages without detections
remain in layout evaluation. Of the 428 annotated tables, 296 have an empty
matched prediction. The official per-table scores, page identifiers and missing
prediction flags are retained in the [table results](benchmarks/omnidocbench-structural-2026-10-25-tables.json).

One PDF, `jiaocaineedrop_chap10.pdf_8.pdf`, returned `Invalid Filter type`.
Its empty prediction is retained. The export error does not change the population
or establish a new parser defect without investigating the input.

The full local evidence is retained under
`docs/reports/2026-10-25-issue-581-evidence/`: `accepted-export/`,
`evaluation/result/`, the generated configurations, logs, environment package
versions, input PDF hashes, and independent geometry controls. The run manifest
records artifact hashes; the commands above regenerate the artifacts elsewhere.
Earlier outputs at that directory's root are explicitly marked diagnostic and
are not the source of these scores.
