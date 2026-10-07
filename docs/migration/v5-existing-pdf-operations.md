# Existing-PDF operations in v5

This v4 preview is available under `operations::existing_document`. Version 5
will promote it to the primary split, extraction, and
merge APIs. The old reconstruction APIs copied page content into a new
`Document`; callers could therefore lose annotations or document-level
structures without selecting a lossy policy.

The primary functions are now:

- `plan_merge_pdfs` and `merge_pdfs`
- `plan_extract_pdf_pages` and `extract_pdf_pages`
- `plan_split_pdf` and `split_pdf`

Every function requires an `ExistingDocumentPolicy`. There is no default
policy. The policy is an enum whose variants carry only options supported by
their engine, so contradictory combinations cannot be constructed.
`ExistingDocumentPolicy::preserve_base()` keeps the first input as an exact byte
prefix and rejects structures from secondary merge inputs that cannot be
combined safely. It does not claim to merge those structures.

`ExistingDocumentPolicy::reconstruct()` deliberately selects the page-content
reconstruction engine. Planning marks every detected catalog structure as
`Discarded`, and execution returns that same report. This is the explicit lossy
path inside the unified family.
`reconstruct_with_metadata_from_first()` is the separate, explicit choice for
copying trailer `/Info` metadata from the first input.

```rust
use oxidize_pdf::operations::existing_document::{
    merge_pdfs, ExistingDocumentMergeInput, ExistingDocumentPolicy,
};

let inputs = [
    ExistingDocumentMergeInput::new("first.pdf"),
    ExistingDocumentMergeInput::new("second.pdf"),
];

let report = merge_pdfs(
    &inputs,
    "merged.pdf",
    ExistingDocumentPolicy::preserve_base(),
)?;
# Ok::<(), oxidize_pdf::operations::OperationError>(())
```

Planning returns the same machine-readable `SemanticPreservationReport` as
execution. `ExistingDocumentExecutionPlan::Incremental` carries the exact
object mutation, while `ExistingDocumentExecutionPlan::Reconstruct` carries
the reconstructed output page count without pretending that an incremental
mutation occurred. Applications should display or persist this report when users need
to understand which input is the preserved base and which document structures
use a deterministic policy such as `FirstInputWins`.

## Legacy reconstruction

The v4 `PdfMerger`, `PdfSplitter`, `PageExtractor`, `MergeOptions`, and
`SplitOptions` model a different operation: reconstructing a new PDF from page
content. They remain public throughout v4, as do the `*_lossless` entry points.
Version 5 will remove those ambiguous names. The narrower
`operations::reconstruct` namespace is also available for code that wants to
make accepted semantic loss visible at the call site.

The compatibility implementation and its operation-specific module paths stay
public until the major transition. The batch worker already uses the preview
API with an explicit policy.

| v4 entry point | v5-preview replacement |
| --- | --- |
| `merge_pdfs`, `PdfMerger`, `merge_pdf_files` | `existing_document::merge_pdfs` with `ExistingDocumentPolicy::reconstruct()` |
| `split_pdf`, `PdfSplitter`, `split_into_pages` | `existing_document::split_pdf` with `ExistingDocumentPolicy::reconstruct()` |
| `PageExtractor`, `extract_page`, `extract_pages`, `extract_page_range` and their `*_to_file` forms | `existing_document::extract_pdf_pages` with `ExistingDocumentPolicy::reconstruct()` |
| `merge_pdfs_lossless` | `existing_document::merge_pdfs` with `ExistingDocumentPolicy::preserve_base()` |
| `split_pdf_lossless` | `existing_document::split_pdf` with `ExistingDocumentPolicy::preserve_base()` |
| `extract_pdf_pages_lossless` | `existing_document::extract_pdf_pages` with `ExistingDocumentPolicy::preserve_base()` |
| Every `plan_*_lossless` function | Corresponding `existing_document::plan_*` function with `ExistingDocumentPolicy::preserve_base()` |

The v5-preview replacement returns a machine-readable report in both modes;
legacy reconstructive functions return only their historical result types.

## Merge limitation

`ExistingDocumentPolicy::preserve_base()` is base-preserving, not a complete semantic
union of arbitrary PDFs. Forms, outlines, destinations, optional-content
groups, tagged structure, attachments, and page labels found in a secondary
input cause planning to fail. Metadata uses a documented first-input-wins
policy. A future policy may support typed remapping, but it must not silently
change the behavior of this policy.

Digital signatures are inventoried independently from AcroForm. Page
annotations and trailer `/Info` metadata are also inventoried, so reconstructive
loss is visible in the report. Encryption has its own semantic category and
encrypted inputs currently fail during planning
because neither engine accepts credentials through this API; the error names
the selected encryption disposition.


## Preparing tagged PDFs with missing author metadata

For preserving splits, `plan_split_pdf` and `split_pdf` now preflight the entire
source before processing any part. A missing language or description anywhere
in the source prevents publication; selecting a different first range does not
hide the remaining requirements.

Use `writer::IncrementalTaggedPdfEditor::new(&source_bytes).preflight()` to
inspect all residual findings together. This recovers only unambiguous derived
indexes and missing page keys in a private in-memory view. It preserves page
hierarchy, content identities, descriptions and ActualText. Malformed references,
ambiguous ownership, unsupported external-stream MCRs and forbidden edits remain
errors. This is bounded tagged validation, not PDF/UA certification.

Use the same editor for the subsequent revision:

```rust,ignore
use oxidize_pdf::writer::{IncrementalTaggedPdfEditor, TaggedPdfMutation};
use oxidize_pdf::parser::objects::{PdfObject, PdfString};

let editor = IncrementalTaggedPdfEditor::new(&source_bytes)
    .with_recovered_indexes()?
    .with_document_language(&author_supplied_language)?;
let changes = [TaggedPdfMutation::SetElementAttribute {
    element: figure_from_preflight,
    key: "Alt".into(),
    value: Some(PdfObject::String(author_supplied_pdf_string)),
}];
let preparation_objects = editor.preparation_objects();
let plan = editor.plan(&changes)?;
let result = editor.apply(&changes)?;
```

Supply one attribute mutation for **each** missing description. Use PDF string
encoding (UTF-16BE with BOM for arbitrary Unicode), as with other editor
attributes. Omit `with_document_language` when the source already has a language.
No author value is inferred. Blank/unknown descriptions or overwriting existing
descriptions in preparation mode are rejected. Existing ordinary editor edits
retain their previous behavior.

The builder prepares an in-memory baseline without publishing bytes. Its exact
object inventory is `preparation_objects()`; `plan.changed_objects` describes
subsequent mutations relative to that baseline. Both must be shown when reviewing
the complete change. `result.plan` matches `plan`, and the existing
`validation_before`/`validation_after` fields describe the mutation stage.
The public mutation/report enums remain unchanged, preserving exhaustive matches
and existing struct construction in consumers.

After either preparation builder is used, `apply` requires full validation even
for an empty mutation list. If findings remain, it returns an error with those
findings and **no prepared bytes**. Successful `result.pdf_bytes` includes the
preparation and mutation revisions with the original source as an exact prefix.
Store it in a separate file and use that file with the ordinary preserving split
API. The editor performs no filesystem writes; keep the original file unchanged.

Studio/callers must bind author values to the exact snapshot used for preflight.
All split parts materialize and validate before publication; normal rename
failures roll back prior parts. This does not promise crash-atomic multi-file
publication or recovery when the filesystem also refuses rollback writes.
#690 remains open until Studio and the published package pass original-input
acceptance. Synthetic descriptions must never become defaults for real documents.
