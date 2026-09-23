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
