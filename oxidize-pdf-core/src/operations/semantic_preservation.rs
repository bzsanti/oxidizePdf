//! Lossless split, extraction, and merge operations for existing PDFs.
//!
//! These APIs use the incremental page-tree mutation engine. The first input
//! remains an exact byte prefix of the result, and unsupported document-level
//! combinations are rejected during planning rather than silently discarded.

use super::reorder::{
    materialize_pdf_page_mutations_from_bytes, plan_pdf_page_mutations_from_bytes,
};
use super::{
    OperationError, OperationResult, PageMutation, PageMutationBatch, PageMutationReport, PageRange,
};
use crate::parser::PdfReader;
use std::collections::BTreeSet;
use std::io::{Cursor, Write};
use std::path::{Path, PathBuf};

const MAX_SELECTION_PLANNING_WORK: usize = 10_000_000;

const DOCUMENT_STRUCTURES: [(&str, DocumentStructure); 8] = [
    ("AcroForm", DocumentStructure::Forms),
    ("Outlines", DocumentStructure::Outlines),
    ("Names", DocumentStructure::NamesAndAttachments),
    ("Dests", DocumentStructure::NamedDestinations),
    ("OCProperties", DocumentStructure::OptionalContent),
    ("StructTreeRoot", DocumentStructure::StructureTree),
    ("Metadata", DocumentStructure::MetadataStream),
    ("PageLabels", DocumentStructure::PageLabels),
];

/// A catalog-level semantic structure considered by lossless operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum DocumentStructure {
    /// Interactive form field tree.
    Forms,
    /// Document outline/bookmark tree.
    Outlines,
    /// Catalog name trees, including embedded files.
    NamesAndAttachments,
    /// Legacy catalog destination dictionary.
    NamedDestinations,
    /// Optional-content configuration and groups.
    OptionalContent,
    /// Tagged-PDF structure tree.
    StructureTree,
    /// Catalog metadata stream.
    MetadataStream,
    /// Page-label number tree.
    PageLabels,
    /// Digital signature fields with a signature value.
    DigitalSignatures,
    /// Trailer-level encryption dictionary.
    Encryption,
    /// Page annotations reachable from selected pages.
    Annotations,
    /// Trailer `/Info` document metadata.
    DocumentInfo,
}

/// Policy applied to a detected document structure.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StructureDisposition {
    /// Retained through the byte-preserved base catalog.
    Preserved,
    /// The first input wins and the secondary value is deliberately ignored.
    FirstInputWins,
    /// The combination cannot be represented safely and planning fails.
    Rejected,
    /// Deliberately omitted by the selected reconstructive engine.
    Discarded,
}

/// Engine selected for an existing-document operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExistingDocumentEngine {
    /// Preserve the first input as an exact byte prefix and mutate incrementally.
    PreserveBase,
    /// Rebuild page appearance/content and explicitly discard catalog semantics.
    Reconstruct,
}

/// One detected structure and its explicit operation policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StructureSemanticReport {
    /// Detected structure.
    pub structure: DocumentStructure,
    /// Policy applied by the operation.
    pub disposition: StructureDisposition,
}

/// How an input document participates in a planned operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputSemanticRole {
    /// The input is the byte-preserved base document.
    PreservedBase,
    /// The input contributes cloned page graphs.
    ImportedPages,
    /// The input contributes page appearance/content through reconstruction.
    ReconstructedPages,
}

/// Semantic inventory for one input.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InputSemanticReport {
    /// Source path.
    pub path: PathBuf,
    /// Role in the output.
    pub role: InputSemanticRole,
    /// Catalog-level structures detected in the source.
    pub structures: Vec<StructureSemanticReport>,
    /// Selected zero-based source page indexes.
    pub selected_pages: Vec<usize>,
}

/// Exact dry-run report returned before a lossless operation writes output.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SemanticPreservationReport {
    /// Engine that will execute, or executed, the operation.
    pub engine: ExistingDocumentEngine,
    /// Per-input semantic inventory and policy.
    pub inputs: Vec<InputSemanticReport>,
    /// Concrete execution plan, whose shape identifies the selected engine.
    pub plan: ExistingDocumentExecutionPlan,
}

/// Engine-specific plan for an existing-document operation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExistingDocumentExecutionPlan {
    /// Exact object-level incremental mutation.
    Incremental(PageMutationReport),
    /// Reconstruct selected page appearance/content into a new document.
    Reconstruct {
        /// Number of pages in the reconstructed output.
        page_count: usize,
    },
}

impl ExistingDocumentExecutionPlan {
    /// Number of pages in the planned output.
    pub const fn page_count(&self) -> usize {
        match self {
            Self::Incremental(report) => report.page_count,
            Self::Reconstruct { page_count } => *page_count,
        }
    }
}

/// Policy for one detected document-level structure.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DocumentStructurePolicy {
    /// Reject the operation before writing output.
    Reject,
    /// Keep the first input's value and deliberately ignore the secondary value.
    FirstInputWins,
}

/// v4-preview compatibility name for [`DocumentStructurePolicy`].
pub type SecondaryStructurePolicy = DocumentStructurePolicy;

/// Policies for structures detected in secondary inputs of a preserving merge.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PreserveBasePolicy {
    /// Interactive forms.
    forms: DocumentStructurePolicy,
    /// Outline tree.
    outlines: DocumentStructurePolicy,
    /// Name trees and attachments.
    names_and_attachments: DocumentStructurePolicy,
    /// Legacy named destinations.
    named_destinations: DocumentStructurePolicy,
    /// Optional-content configuration.
    optional_content: DocumentStructurePolicy,
    /// Tagged-PDF structure tree.
    structure_tree: DocumentStructurePolicy,
    /// Catalog metadata stream.
    metadata: DocumentStructurePolicy,
    /// Page labels.
    page_labels: DocumentStructurePolicy,
    /// Digital signatures.
    digital_signatures: DocumentStructurePolicy,
    /// Encryption.
    encryption: DocumentStructurePolicy,
    /// Page annotations.
    annotations: DocumentStructurePolicy,
    /// Trailer document information.
    document_info: DocumentStructurePolicy,
}

impl PreserveBasePolicy {
    const fn fail_closed() -> Self {
        Self {
            forms: DocumentStructurePolicy::Reject,
            outlines: DocumentStructurePolicy::Reject,
            names_and_attachments: DocumentStructurePolicy::Reject,
            named_destinations: DocumentStructurePolicy::Reject,
            optional_content: DocumentStructurePolicy::Reject,
            structure_tree: DocumentStructurePolicy::Reject,
            metadata: DocumentStructurePolicy::FirstInputWins,
            page_labels: DocumentStructurePolicy::Reject,
            digital_signatures: DocumentStructurePolicy::Reject,
            encryption: DocumentStructurePolicy::Reject,
            annotations: DocumentStructurePolicy::Reject,
            document_info: DocumentStructurePolicy::FirstInputWins,
        }
    }

    /// Set the policy for page labels found in secondary merge inputs.
    pub const fn with_page_labels(mut self, policy: DocumentStructurePolicy) -> Self {
        self.page_labels = policy;
        self
    }

    fn disposition(self, structure: DocumentStructure) -> StructureDisposition {
        let policy = match structure {
            DocumentStructure::Forms => self.forms,
            DocumentStructure::Outlines => self.outlines,
            DocumentStructure::NamesAndAttachments => self.names_and_attachments,
            DocumentStructure::NamedDestinations => self.named_destinations,
            DocumentStructure::OptionalContent => self.optional_content,
            DocumentStructure::StructureTree => self.structure_tree,
            DocumentStructure::MetadataStream => self.metadata,
            DocumentStructure::PageLabels => self.page_labels,
            DocumentStructure::DigitalSignatures => self.digital_signatures,
            DocumentStructure::Encryption => self.encryption,
            DocumentStructure::Annotations => self.annotations,
            DocumentStructure::DocumentInfo => self.document_info,
        };
        match policy {
            DocumentStructurePolicy::Reject => StructureDisposition::Rejected,
            DocumentStructurePolicy::FirstInputWins => StructureDisposition::FirstInputWins,
        }
    }
}

/// Metadata handling supported by the reconstructive engine.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReconstructMetadataPolicy {
    /// Discard document information metadata.
    Discard,
    /// Copy document information metadata from the first input.
    FirstInputWins,
}

/// Valid policy for the reconstructive engine.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReconstructPolicy {
    metadata: ReconstructMetadataPolicy,
}

impl ReconstructPolicy {
    /// Metadata policy selected for reconstruction.
    pub const fn metadata(self) -> ReconstructMetadataPolicy {
        self.metadata
    }
}

/// Explicit, type-safe policy for existing-document operations.
///
/// There is deliberately no [`Default`] implementation: every call site must
/// choose preservation or reconstruction. Engine-specific policy values also
/// cannot be assembled from arbitrary structure dispositions.
///
/// ```compile_fail
/// use oxidize_pdf::operations::ExistingDocumentPolicy;
///
/// let policy = ExistingDocumentPolicy::default();
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExistingDocumentPolicy {
    /// Preserve the first input byte-for-byte and control secondary structures.
    PreserveBase(PreserveBasePolicy),
    /// Reconstruct page appearance/content and deliberately lose other semantics.
    Reconstruct(ReconstructPolicy),
}

impl ExistingDocumentPolicy {
    /// Preserve the base and reject ambiguous secondary structures.
    pub const fn preserve_base() -> Self {
        Self::PreserveBase(PreserveBasePolicy::fail_closed())
    }

    /// Reconstruct page appearance/content and explicitly discard every
    /// detected document-level structure.
    pub const fn reconstruct() -> Self {
        Self::Reconstruct(ReconstructPolicy {
            metadata: ReconstructMetadataPolicy::Discard,
        })
    }

    /// Reconstruct pages while copying document information from the first input.
    pub const fn reconstruct_with_metadata_from_first() -> Self {
        Self::Reconstruct(ReconstructPolicy {
            metadata: ReconstructMetadataPolicy::FirstInputWins,
        })
    }

    /// Return the selected execution engine.
    pub const fn engine(self) -> ExistingDocumentEngine {
        match self {
            Self::PreserveBase(_) => ExistingDocumentEngine::PreserveBase,
            Self::Reconstruct(_) => ExistingDocumentEngine::Reconstruct,
        }
    }

    /// Configure page labels in secondary inputs of a preserving merge.
    /// Reconstructive policies are unchanged because they always discard them.
    pub const fn with_page_labels(self, policy: DocumentStructurePolicy) -> Self {
        match self {
            Self::PreserveBase(preserve) => Self::PreserveBase(preserve.with_page_labels(policy)),
            reconstruct @ Self::Reconstruct(_) => reconstruct,
        }
    }

    fn disposition(
        self,
        role: InputSemanticRole,
        structure: DocumentStructure,
    ) -> StructureDisposition {
        match self {
            Self::PreserveBase(_) if role == InputSemanticRole::PreservedBase => {
                StructureDisposition::Preserved
            }
            Self::PreserveBase(policy) => policy.disposition(structure),
            Self::Reconstruct(policy) => match structure {
                DocumentStructure::DocumentInfo
                    if policy.metadata == ReconstructMetadataPolicy::FirstInputWins =>
                {
                    StructureDisposition::FirstInputWins
                }
                _ => StructureDisposition::Discarded,
            },
        }
    }
}

/// Input to a semantic merge of existing PDF documents.
#[derive(Debug, Clone)]
pub struct ExistingDocumentMergeInput {
    /// PDF path.
    pub path: PathBuf,
    /// Pages to include, or all pages when omitted.
    pub pages: Option<PageRange>,
}

impl ExistingDocumentMergeInput {
    /// Include every page from `path`.
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self {
            path: path.into(),
            pages: None,
        }
    }

    /// Include the selected pages from `path`.
    pub fn with_pages(path: impl Into<PathBuf>, pages: PageRange) -> Self {
        Self {
            path: path.into(),
            pages: Some(pages),
        }
    }
}

/// v4 compatibility name for a preservation-first merge input.
pub type LosslessMergeInput = ExistingDocumentMergeInput;

fn inspect_input_bytes(
    path: &Path,
    bytes: &[u8],
    role: InputSemanticRole,
    range: Option<&PageRange>,
    policy: ExistingDocumentPolicy,
) -> OperationResult<InputSemanticReport> {
    let mut reader = PdfReader::new(Cursor::new(bytes)).map_err(|error| {
        OperationError::ParseError(format!("open snapshot of {}: {error}", path.display()))
    })?;
    if reader.is_encrypted() {
        let disposition = policy.disposition(role, DocumentStructure::Encryption);
        return Err(OperationError::PdfError(crate::PdfError::PermissionDenied(
            format!(
                "existing-document operation cannot process encrypted PDF {} (policy disposition: {:?})",
                path.display(), disposition
            ),
        )));
    }
    let page_count = reader.page_count().map_err(|error| {
        OperationError::ParseError(format!("count pages in {}: {error}", path.display()))
    })? as usize;
    let selected_pages = range.unwrap_or(&PageRange::All).get_indices(page_count)?;
    if selected_pages.is_empty() {
        return Err(OperationError::NoPagesToProcess);
    }
    let catalog = reader.catalog().map_err(|error| {
        OperationError::ParseError(format!("read catalog in {}: {error}", path.display()))
    })?;
    let mut structures: Vec<_> = DOCUMENT_STRUCTURES
        .iter()
        .filter_map(|(key, structure)| {
            catalog
                .contains_key(key)
                .then_some(StructureSemanticReport {
                    structure: *structure,
                    disposition: policy.disposition(role, *structure),
                })
        })
        .collect();
    let signatures = crate::signatures::detect_signature_fields(&mut reader).map_err(|error| {
        OperationError::ParseError(format!("inspect signatures in {}: {error}", path.display()))
    })?;
    if !signatures.is_empty() {
        structures.push(StructureSemanticReport {
            structure: DocumentStructure::DigitalSignatures,
            disposition: policy.disposition(role, DocumentStructure::DigitalSignatures),
        });
    }
    if reader.trailer().info().is_some() {
        structures.push(StructureSemanticReport {
            structure: DocumentStructure::DocumentInfo,
            disposition: policy.disposition(role, DocumentStructure::DocumentInfo),
        });
    }
    let document = PdfReader::new(Cursor::new(bytes))
        .map_err(|error| OperationError::ParseError(error.to_string()))?
        .into_document();
    let has_annotations = selected_pages.iter().try_fold(false, |found, page| {
        if found {
            return Ok(true);
        }
        document
            .get_page(*page as u32)
            .map(|page| {
                page.get_annotations()
                    .is_some_and(|annots| !annots.is_empty())
            })
            .map_err(|error| OperationError::ParseError(error.to_string()))
    })?;
    if has_annotations {
        structures.push(StructureSemanticReport {
            structure: DocumentStructure::Annotations,
            disposition: policy.disposition(role, DocumentStructure::Annotations),
        });
    }
    Ok(InputSemanticReport {
        path: path.to_path_buf(),
        role,
        structures,
        selected_pages,
    })
}

fn reject_secondary_document_structures(report: &InputSemanticReport) -> OperationResult<()> {
    // Metadata has a deterministic first-input-wins policy. Other catalog
    // structures require cross-document identity/name remapping and therefore
    // fail closed until that mapping is representable.
    let unsupported: Vec<_> = report
        .structures
        .iter()
        .filter(|entry| entry.disposition == StructureDisposition::Rejected)
        .map(|entry| entry.structure)
        .collect();
    if unsupported.is_empty() {
        return Ok(());
    }
    Err(OperationError::ProcessingError(format!(
        "cannot merge document-level structures from secondary input {}: {:?}; the operation was not written",
        report.path.display(), unsupported
    )))
}

fn selection_batch(page_count: usize, selected: &[usize]) -> OperationResult<PageMutationBatch> {
    let selected_set: BTreeSet<_> = selected.iter().copied().collect();
    let mut operations: Vec<_> = (0..page_count)
        .rev()
        .filter(|index| !selected_set.contains(index))
        .map(|page| PageMutation::Delete { page })
        .collect();
    let mut current: Vec<_> = (0..page_count)
        .filter(|index| selected_set.contains(index))
        .collect();
    let estimated_work = current.len().checked_mul(selected.len()).ok_or_else(|| {
        OperationError::ProcessingError("page selection work overflow".to_string())
    })?;
    let already_ordered = current.as_slice() == selected;
    if !already_ordered && estimated_work > MAX_SELECTION_PLANNING_WORK {
        return Err(OperationError::ProcessingError(format!(
            "page selection would require excessive reorder work ({estimated_work} position checks)"
        )));
    }
    for (target, &source_page) in selected.iter().enumerate() {
        if current.get(target) == Some(&source_page) {
            continue;
        }
        if let Some(from) = current
            .iter()
            .enumerate()
            .skip(target + 1)
            .find_map(|(index, page)| (*page == source_page).then_some(index))
        {
            operations.push(PageMutation::Move { from, to: target });
            let page = current.remove(from);
            current.insert(target, page);
        } else if let Some(from) = current.iter().position(|page| *page == source_page) {
            operations.push(PageMutation::Duplicate {
                page: from,
                at: target,
            });
            current.insert(target, source_page);
        }
    }
    Ok(PageMutationBatch { operations })
}

fn read_snapshot(path: &Path) -> OperationResult<Vec<u8>> {
    std::fs::read(path).map_err(OperationError::Io)
}

fn ensure_snapshot_unchanged(path: &Path, expected: &[u8]) -> OperationResult<()> {
    if read_snapshot(path)? != expected {
        return Err(OperationError::ProcessingError(format!(
            "source {} changed after planning; no output was published",
            path.display()
        )));
    }
    Ok(())
}

fn stage_output(path: &Path, bytes: &[u8]) -> OperationResult<tempfile::NamedTempFile> {
    let parent = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty());
    let mut temporary = tempfile::NamedTempFile::new_in(parent.unwrap_or_else(|| Path::new(".")))?;
    temporary.write_all(bytes)?;
    temporary.flush()?;
    temporary.as_file().sync_all()?;
    Ok(temporary)
}

fn normalized_path(path: &Path) -> OperationResult<PathBuf> {
    if path.exists() {
        return path.canonicalize().map_err(OperationError::Io);
    }
    let parent = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."))
        .canonicalize()?;
    let name = path
        .file_name()
        .ok_or_else(|| OperationError::InvalidPath {
            reason: format!("output path {} has no file name", path.display()),
        })?;
    Ok(parent.join(name))
}

fn plan_batch(
    base: &[u8],
    batch: &PageMutationBatch,
    page_count: usize,
) -> OperationResult<PageMutationReport> {
    if batch.operations.is_empty() {
        return Ok(PageMutationReport {
            replaced_objects: Vec::new(),
            added_objects: Vec::new(),
            unreachable_objects: Vec::new(),
            page_count,
        });
    }
    plan_pdf_page_mutations_from_bytes(base, batch)
}

fn materialize_batch(
    base: &[u8],
    batch: &PageMutationBatch,
    page_count: usize,
) -> OperationResult<(Vec<u8>, PageMutationReport)> {
    if batch.operations.is_empty() {
        return Ok((base.to_vec(), plan_batch(base, batch, page_count)?));
    }
    materialize_pdf_page_mutations_from_bytes(base, batch)
}

/// Plan a sparse lossless extraction without writing output.
///
/// # Errors
///
/// Returns an error for invalid page indexes, encrypted or restricted inputs,
/// and catalog/page structures that reference pages being removed.
fn plan_extract_pdf_pages_preserving(
    input: impl AsRef<Path>,
    pages: &[usize],
    policy: ExistingDocumentPolicy,
) -> OperationResult<SemanticPreservationReport> {
    let input = input.as_ref();
    if pages.is_empty() {
        return Err(OperationError::NoPagesToProcess);
    }
    let range = PageRange::List(pages.to_vec());
    let base = read_snapshot(input)?;
    let report = inspect_input_bytes(
        input,
        &base,
        InputSemanticRole::PreservedBase,
        Some(&range),
        policy,
    )?;
    let page_count = PdfReader::new(Cursor::new(&base))
        .map_err(|error| OperationError::ParseError(error.to_string()))?
        .page_count()
        .map_err(|error| OperationError::ParseError(error.to_string()))?
        as usize;
    let batch = selection_batch(page_count, &report.selected_pages)?;
    let mutation = plan_batch(&base, &batch, page_count)?;
    Ok(SemanticPreservationReport {
        engine: ExistingDocumentEngine::PreserveBase,
        inputs: vec![report],
        plan: ExistingDocumentExecutionPlan::Incremental(mutation),
    })
}

/// Extract pages while retaining the source bytes and complete reachable semantics.
///
/// Planning is repeated immediately before the atomic write. The output is
/// reopened and its reachability is checked by the mutation engine.
///
/// # Errors
///
/// Returns an error under the same conditions as
/// [`plan_extract_pdf_pages_preserving`], or when atomic publication fails.
fn extract_pdf_pages_preserving(
    input: impl AsRef<Path>,
    output: impl AsRef<Path>,
    pages: &[usize],
    policy: ExistingDocumentPolicy,
) -> OperationResult<SemanticPreservationReport> {
    let input = input.as_ref();
    let output = output.as_ref();
    let base = read_snapshot(input)?;
    let range = PageRange::List(pages.to_vec());
    let input_report = inspect_input_bytes(
        input,
        &base,
        InputSemanticRole::PreservedBase,
        Some(&range),
        policy,
    )?;
    let actual_count = PdfReader::new(Cursor::new(&base))
        .map_err(|error| OperationError::ParseError(error.to_string()))?
        .page_count()
        .map_err(|error| OperationError::ParseError(error.to_string()))?
        as usize;
    let batch = selection_batch(actual_count, &input_report.selected_pages)?;
    let planned = plan_batch(&base, &batch, actual_count)?;
    let (updated, written) = materialize_batch(&base, &batch, actual_count)?;
    if written != planned {
        return Err(OperationError::ProcessingError(
            "dry-run report differed from materialized extraction; no output was published"
                .to_string(),
        ));
    }
    ensure_snapshot_unchanged(input, &base)?;
    stage_output(output, &updated)?
        .persist(output)
        .map_err(|error| OperationError::Io(error.error))?;
    let report = SemanticPreservationReport {
        engine: ExistingDocumentEngine::PreserveBase,
        inputs: vec![input_report],
        plan: ExistingDocumentExecutionPlan::Incremental(planned),
    };
    Ok(report)
}

/// Plan every output of a lossless split without writing files.
///
/// # Errors
///
/// Returns an error when a range is empty or cannot be represented while
/// preserving the source's reachable document semantics.
fn plan_split_pdf_preserving(
    input: impl AsRef<Path>,
    ranges: &[PageRange],
    policy: ExistingDocumentPolicy,
) -> OperationResult<Vec<SemanticPreservationReport>> {
    let input = input.as_ref();
    if ranges.is_empty() {
        return Err(OperationError::NoPagesToProcess);
    }
    let base = read_snapshot(input)?;
    let mut reader = PdfReader::new(Cursor::new(&base))
        .map_err(|error| OperationError::ParseError(error.to_string()))?;
    let page_count = reader
        .page_count()
        .map_err(|error| OperationError::ParseError(error.to_string()))?
        as usize;
    ranges
        .iter()
        .map(|range| {
            let pages = range.get_indices(page_count)?;
            let report = inspect_input_bytes(
                input,
                &base,
                InputSemanticRole::PreservedBase,
                Some(range),
                policy,
            )?;
            let batch = selection_batch(page_count, &pages)?;
            let mutation = plan_batch(&base, &batch, page_count)?;
            Ok(SemanticPreservationReport {
                engine: ExistingDocumentEngine::PreserveBase,
                inputs: vec![report],
                plan: ExistingDocumentExecutionPlan::Incremental(mutation),
            })
        })
        .collect()
}

/// Split a PDF into caller-selected outputs using lossless atomic extraction.
///
/// Every output is planned, materialized, and staged before the first
/// destination is replaced. Each replacement is atomic; if a later filesystem
/// replacement fails, previously replaced destinations are restored from
/// snapshots.
///
/// # Errors
///
/// Returns an error when the range/output counts differ, paths alias, any dry
/// run fails, or an output cannot be validated and transactionally published.
fn split_pdf_preserving(
    input: impl AsRef<Path>,
    ranges: &[PageRange],
    outputs: &[PathBuf],
    policy: ExistingDocumentPolicy,
) -> OperationResult<Vec<SemanticPreservationReport>> {
    if ranges.len() != outputs.len() {
        return Err(OperationError::InvalidPath {
            reason: format!(
                "lossless split has {} ranges but {} output paths",
                ranges.len(),
                outputs.len()
            ),
        });
    }
    let input = input.as_ref();
    let normalized_input = normalized_path(input)?;
    let mut normalized_outputs = BTreeSet::new();
    for output in outputs {
        let normalized = normalized_path(output)?;
        if normalized == normalized_input {
            return Err(OperationError::InvalidPath {
                reason: format!("split output {} aliases the input", output.display()),
            });
        }
        if !normalized_outputs.insert(normalized) {
            return Err(OperationError::InvalidPath {
                reason: format!("duplicate split output path {}", output.display()),
            });
        }
    }
    let base = read_snapshot(input)?;
    let mut reader = PdfReader::new(Cursor::new(&base))
        .map_err(|error| OperationError::ParseError(error.to_string()))?;
    let page_count = reader
        .page_count()
        .map_err(|error| OperationError::ParseError(error.to_string()))?
        as usize;
    let mut reports = Vec::with_capacity(ranges.len());
    let mut materialized = Vec::with_capacity(ranges.len());
    for range in ranges {
        let pages = range.get_indices(page_count)?;
        let input_report = inspect_input_bytes(
            input,
            &base,
            InputSemanticRole::PreservedBase,
            Some(range),
            policy,
        )?;
        let batch = selection_batch(page_count, &pages)?;
        let planned = plan_batch(&base, &batch, page_count)?;
        let (bytes, written) = materialize_batch(&base, &batch, page_count)?;
        if written != planned {
            return Err(OperationError::ProcessingError(
                "dry-run report differed from a materialized split output; no output was published"
                    .to_string(),
            ));
        }
        reports.push(SemanticPreservationReport {
            engine: ExistingDocumentEngine::PreserveBase,
            inputs: vec![input_report],
            plan: ExistingDocumentExecutionPlan::Incremental(planned),
        });
        materialized.push(bytes);
    }
    ensure_snapshot_unchanged(input, &base)?;
    let mut staged = outputs
        .iter()
        .zip(&materialized)
        .map(|(path, bytes)| stage_output(path, bytes))
        .collect::<OperationResult<Vec<_>>>()?;
    let backups = outputs
        .iter()
        .map(|path| {
            if path.exists() {
                std::fs::read(path).map(Some)
            } else {
                Ok(None)
            }
        })
        .collect::<Result<Vec<_>, _>>()?;
    for index in 0..staged.len() {
        let temporary = staged.remove(0);
        if let Err(error) = temporary.persist(&outputs[index]) {
            for rollback in 0..index {
                match &backups[rollback] {
                    Some(bytes) => {
                        stage_output(&outputs[rollback], bytes)?
                            .persist(&outputs[rollback])
                            .map_err(|persist| OperationError::Io(persist.error))?;
                    }
                    None => {
                        let _ = std::fs::remove_file(&outputs[rollback]);
                    }
                }
            }
            return Err(OperationError::Io(error.error));
        }
    }
    Ok(reports)
}

/// Plan a deterministic lossless merge without writing output.
///
/// The first input is the preserved base. Secondary inputs may contribute
/// self-contained page graphs; catalog-level structures in secondary inputs
/// are rejected because combining their policies would be ambiguous.
///
/// # Errors
///
/// Returns an error for fewer than one input, invalid selections, encrypted or
/// restricted PDFs, unsupported imported page graphs, or secondary document
/// structures that cannot be combined safely.
fn plan_merge_pdfs_preserving(
    inputs: &[ExistingDocumentMergeInput],
    policy: ExistingDocumentPolicy,
) -> OperationResult<SemanticPreservationReport> {
    let Some(first) = inputs.first() else {
        return Err(OperationError::NoPagesToProcess);
    };
    let snapshots = inputs
        .iter()
        .map(|input| read_snapshot(&input.path))
        .collect::<OperationResult<Vec<_>>>()?;
    let snapshot_files = snapshot_files(&snapshots)?;
    let base = inspect_input_bytes(
        &first.path,
        &snapshots[0],
        InputSemanticRole::PreservedBase,
        first.pages.as_ref(),
        policy,
    )?;
    let base_count = PdfReader::new(Cursor::new(&snapshots[0]))
        .map_err(|error| OperationError::ParseError(error.to_string()))?
        .page_count()
        .map_err(|error| OperationError::ParseError(error.to_string()))?
        as usize;
    let mut batch = selection_batch(base_count, &base.selected_pages)?;
    let mut reports = vec![base];
    let mut insertion_index = reports[0].selected_pages.len();
    for (index, (input, snapshot)) in inputs.iter().zip(&snapshots).enumerate().skip(1) {
        let report = inspect_input_bytes(
            &input.path,
            snapshot,
            InputSemanticRole::ImportedPages,
            input.pages.as_ref(),
            policy,
        )?;
        reject_secondary_document_structures(&report)?;
        for &page in &report.selected_pages {
            batch.operations.push(PageMutation::Insert {
                source: snapshot_files[index].path().to_path_buf(),
                page,
                at: insertion_index,
            });
            insertion_index += 1;
        }
        reports.push(report);
    }
    let mutation = plan_batch(&snapshots[0], &batch, base_count)?;
    for (input, snapshot) in inputs.iter().zip(&snapshots) {
        ensure_snapshot_unchanged(&input.path, snapshot)?;
    }
    Ok(SemanticPreservationReport {
        engine: ExistingDocumentEngine::PreserveBase,
        inputs: reports,
        plan: ExistingDocumentExecutionPlan::Incremental(mutation),
    })
}

/// Merge PDFs losslessly and publish the validated output atomically.
///
/// # Errors
///
/// Returns an error under the same conditions as [`plan_merge_pdfs_preserving`],
/// or when atomic publication or output validation fails.
fn merge_pdfs_preserving(
    inputs: &[ExistingDocumentMergeInput],
    output: impl AsRef<Path>,
    policy: ExistingDocumentPolicy,
) -> OperationResult<SemanticPreservationReport> {
    let Some(first) = inputs.first() else {
        return Err(OperationError::NoPagesToProcess);
    };
    let snapshots = inputs
        .iter()
        .map(|input| read_snapshot(&input.path))
        .collect::<OperationResult<Vec<_>>>()?;
    let snapshot_files = snapshot_files(&snapshots)?;
    let base_report = inspect_input_bytes(
        &first.path,
        &snapshots[0],
        InputSemanticRole::PreservedBase,
        first.pages.as_ref(),
        policy,
    )?;
    let base_count = PdfReader::new(Cursor::new(&snapshots[0]))
        .map_err(|error| OperationError::ParseError(error.to_string()))?
        .page_count()
        .map_err(|error| OperationError::ParseError(error.to_string()))?
        as usize;
    let mut batch = selection_batch(base_count, &base_report.selected_pages)?;
    let mut reports = vec![base_report];
    let mut at = reports[0].selected_pages.len();
    for (index, (input, snapshot)) in inputs.iter().zip(&snapshots).enumerate().skip(1) {
        let input_report = inspect_input_bytes(
            &input.path,
            snapshot,
            InputSemanticRole::ImportedPages,
            input.pages.as_ref(),
            policy,
        )?;
        reject_secondary_document_structures(&input_report)?;
        for &page in &input_report.selected_pages {
            batch.operations.push(PageMutation::Insert {
                source: snapshot_files[index].path().to_path_buf(),
                page,
                at,
            });
            at += 1;
        }
        reports.push(input_report);
    }
    let final_count = at;
    let planned = plan_batch(&snapshots[0], &batch, final_count)?;
    let (updated, written) = materialize_batch(&snapshots[0], &batch, final_count)?;
    if written != planned {
        return Err(OperationError::ProcessingError(
            "dry-run report differed from materialized merge; no output was published".to_string(),
        ));
    }
    for (input, snapshot) in inputs.iter().zip(&snapshots) {
        ensure_snapshot_unchanged(&input.path, snapshot)?;
    }
    let output = output.as_ref();
    stage_output(output, &updated)?
        .persist(output)
        .map_err(|error| OperationError::Io(error.error))?;
    let report = SemanticPreservationReport {
        engine: ExistingDocumentEngine::PreserveBase,
        inputs: reports,
        plan: ExistingDocumentExecutionPlan::Incremental(planned),
    };
    Ok(report)
}

fn reconstructive_report(
    inputs: &[ExistingDocumentMergeInput],
    policy: ExistingDocumentPolicy,
) -> OperationResult<SemanticPreservationReport> {
    let snapshots = inputs
        .iter()
        .map(|input| read_snapshot(&input.path))
        .collect::<OperationResult<Vec<_>>>()?;
    reconstructive_report_from_snapshots(inputs, &snapshots, policy)
}

fn reconstructive_report_from_snapshots(
    inputs: &[ExistingDocumentMergeInput],
    snapshots: &[Vec<u8>],
    policy: ExistingDocumentPolicy,
) -> OperationResult<SemanticPreservationReport> {
    let mut reports = Vec::with_capacity(inputs.len());
    let mut page_count = 0usize;
    for (input, bytes) in inputs.iter().zip(snapshots) {
        let report = inspect_input_bytes(
            &input.path,
            bytes,
            InputSemanticRole::ReconstructedPages,
            input.pages.as_ref(),
            policy,
        )?;
        page_count = page_count
            .checked_add(report.selected_pages.len())
            .ok_or_else(|| OperationError::ProcessingError("page count overflow".to_string()))?;
        reports.push(report);
    }
    if reports.is_empty() || page_count == 0 {
        return Err(OperationError::NoPagesToProcess);
    }
    Ok(SemanticPreservationReport {
        engine: ExistingDocumentEngine::Reconstruct,
        inputs: reports,
        plan: ExistingDocumentExecutionPlan::Reconstruct { page_count },
    })
}

fn reject_output_aliases(output: &Path, inputs: &[&Path]) -> OperationResult<()> {
    let normalized_output = normalized_path(output)?;
    for input in inputs {
        if normalized_output == normalized_path(input)? {
            return Err(OperationError::InvalidPath {
                reason: format!(
                    "output {} aliases input {}",
                    output.display(),
                    input.display()
                ),
            });
        }
    }
    Ok(())
}

fn snapshot_files(snapshots: &[Vec<u8>]) -> OperationResult<Vec<tempfile::NamedTempFile>> {
    snapshots
        .iter()
        .map(|bytes| stage_output(Path::new("snapshot.pdf"), bytes))
        .collect()
}

fn reconstructive_extract_report(
    input: &Path,
    pages: &[usize],
    policy: ExistingDocumentPolicy,
) -> OperationResult<SemanticPreservationReport> {
    if pages.is_empty() {
        return Err(OperationError::NoPagesToProcess);
    }
    reconstructive_report(
        &[ExistingDocumentMergeInput::with_pages(
            input,
            PageRange::List(pages.to_vec()),
        )],
        policy,
    )
}

/// Plan a semantic merge using an explicit preservation policy.
///
/// Unlike the legacy reconstruction API, this operation cannot silently
/// discard document semantics.
///
/// # Errors
///
/// Returns an error for missing inputs, invalid selections, encrypted or
/// restricted PDFs, unsafe imported graphs, or policy-rejected structures.
pub fn plan_merge_pdfs(
    inputs: &[ExistingDocumentMergeInput],
    policy: ExistingDocumentPolicy,
) -> OperationResult<SemanticPreservationReport> {
    match policy {
        ExistingDocumentPolicy::PreserveBase(_) => plan_merge_pdfs_preserving(inputs, policy),
        ExistingDocumentPolicy::Reconstruct(_) => reconstructive_report(inputs, policy),
    }
}

/// Merge existing PDFs using an explicit preservation policy.
///
/// # Errors
///
/// Returns the planning errors described by [`plan_merge_pdfs`], or an error
/// if validation or atomic publication fails. No output is published on error.
pub fn merge_pdfs(
    inputs: &[ExistingDocumentMergeInput],
    output: impl AsRef<Path>,
    policy: ExistingDocumentPolicy,
) -> OperationResult<SemanticPreservationReport> {
    let output = output.as_ref();
    let input_paths: Vec<_> = inputs.iter().map(|input| input.path.as_path()).collect();
    reject_output_aliases(output, &input_paths)?;
    match policy {
        ExistingDocumentPolicy::PreserveBase(_) => merge_pdfs_preserving(inputs, output, policy),
        ExistingDocumentPolicy::Reconstruct(reconstruct) => {
            let snapshots = inputs
                .iter()
                .map(|input| read_snapshot(&input.path))
                .collect::<OperationResult<Vec<_>>>()?;
            let report = reconstructive_report_from_snapshots(inputs, &snapshots, policy)?;
            let snapshot_files = snapshot_files(&snapshots)?;
            let legacy_inputs = inputs
                .iter()
                .zip(&snapshot_files)
                .map(|(input, snapshot)| super::merge::MergeInput {
                    path: snapshot.path().to_path_buf(),
                    pages: input.pages.clone(),
                })
                .collect();
            let metadata_mode = match reconstruct.metadata() {
                ReconstructMetadataPolicy::Discard => super::merge::MetadataMode::None,
                ReconstructMetadataPolicy::FirstInputWins => super::merge::MetadataMode::FromFirst,
            };
            let temporary = tempfile::NamedTempFile::new_in(
                output
                    .parent()
                    .filter(|parent| !parent.as_os_str().is_empty())
                    .unwrap_or_else(|| Path::new(".")),
            )?;
            super::merge::merge_pdfs(
                legacy_inputs,
                temporary.path(),
                super::merge::MergeOptions {
                    metadata_mode,
                    ..super::merge::MergeOptions::default()
                },
            )?;
            temporary.as_file().sync_all()?;
            temporary
                .persist(output)
                .map_err(|error| OperationError::Io(error.error))?;
            Ok(report)
        }
    }
}

/// Plan sparse extraction using an explicit preservation policy.
///
/// # Errors
///
/// Returns an error for an empty or invalid page selection, encrypted or
/// restricted input, or references that make removing pages unsafe.
pub fn plan_extract_pdf_pages(
    input: impl AsRef<Path>,
    pages: &[usize],
    policy: ExistingDocumentPolicy,
) -> OperationResult<SemanticPreservationReport> {
    match policy {
        ExistingDocumentPolicy::PreserveBase(_) => {
            plan_extract_pdf_pages_preserving(input, pages, policy)
        }
        ExistingDocumentPolicy::Reconstruct(_) => {
            reconstructive_extract_report(input.as_ref(), pages, policy)
        }
    }
}

/// Extract pages from an existing PDF using an explicit preservation policy.
///
/// # Errors
///
/// Returns the planning errors described by [`plan_extract_pdf_pages`], or an
/// error if the source changes, validation fails, or publication fails.
pub fn extract_pdf_pages(
    input: impl AsRef<Path>,
    output: impl AsRef<Path>,
    pages: &[usize],
    policy: ExistingDocumentPolicy,
) -> OperationResult<SemanticPreservationReport> {
    let input = input.as_ref();
    let output = output.as_ref();
    reject_output_aliases(output, &[input])?;
    match policy {
        ExistingDocumentPolicy::PreserveBase(_) => {
            extract_pdf_pages_preserving(input, output, pages, policy)
        }
        ExistingDocumentPolicy::Reconstruct(_) => {
            let snapshot = read_snapshot(input)?;
            let merge_input =
                ExistingDocumentMergeInput::with_pages(input, PageRange::List(pages.to_vec()));
            let report = reconstructive_report_from_snapshots(
                std::slice::from_ref(&merge_input),
                std::slice::from_ref(&snapshot),
                policy,
            )?;
            let snapshot_file = stage_output(Path::new("snapshot.pdf"), &snapshot)?;
            let temporary = tempfile::NamedTempFile::new_in(
                output
                    .parent()
                    .filter(|parent| !parent.as_os_str().is_empty())
                    .unwrap_or_else(|| Path::new(".")),
            )?;
            super::page_extraction::extract_pages_to_file(
                snapshot_file.path(),
                pages,
                temporary.path(),
            )?;
            temporary.as_file().sync_all()?;
            temporary
                .persist(output)
                .map_err(|error| OperationError::Io(error.error))?;
            Ok(report)
        }
    }
}

/// Plan a split using an explicit preservation policy.
///
/// # Errors
///
/// Returns an error for empty or invalid ranges, encrypted or restricted input,
/// or retained structures that reference pages removed from an output.
pub fn plan_split_pdf(
    input: impl AsRef<Path>,
    ranges: &[PageRange],
    policy: ExistingDocumentPolicy,
) -> OperationResult<Vec<SemanticPreservationReport>> {
    match policy {
        ExistingDocumentPolicy::PreserveBase(_) => plan_split_pdf_preserving(input, ranges, policy),
        ExistingDocumentPolicy::Reconstruct(_) => {
            if ranges.is_empty() {
                return Err(OperationError::NoPagesToProcess);
            }
            let input = input.as_ref();
            let snapshot = read_snapshot(input)?;
            let mut reader = PdfReader::new(Cursor::new(&snapshot))
                .map_err(|error| OperationError::ParseError(error.to_string()))?;
            let page_count = reader
                .page_count()
                .map_err(|error| OperationError::ParseError(error.to_string()))?
                as usize;
            ranges
                .iter()
                .map(|range| {
                    let pages = range.get_indices(page_count)?;
                    let merge_input =
                        ExistingDocumentMergeInput::with_pages(input, PageRange::List(pages));
                    reconstructive_report_from_snapshots(
                        std::slice::from_ref(&merge_input),
                        std::slice::from_ref(&snapshot),
                        policy,
                    )
                })
                .collect()
        }
    }
}

/// Split an existing PDF using an explicit preservation policy.
///
/// # Errors
///
/// Returns the planning errors described by [`plan_split_pdf`], mismatched or
/// aliased paths, validation failures, or transactional publication failures.
pub fn split_pdf(
    input: impl AsRef<Path>,
    ranges: &[PageRange],
    outputs: &[PathBuf],
    policy: ExistingDocumentPolicy,
) -> OperationResult<Vec<SemanticPreservationReport>> {
    if ranges.is_empty() {
        return Err(OperationError::NoPagesToProcess);
    }
    match policy {
        ExistingDocumentPolicy::PreserveBase(_) => {
            split_pdf_preserving(input, ranges, outputs, policy)
        }
        ExistingDocumentPolicy::Reconstruct(_) => {
            if ranges.len() != outputs.len() {
                return Err(OperationError::InvalidPath {
                    reason: format!(
                        "split has {} ranges but {} output paths",
                        ranges.len(),
                        outputs.len()
                    ),
                });
            }
            let input = input.as_ref();
            let normalized_input = normalized_path(input)?;
            let mut normalized_outputs = BTreeSet::new();
            for output in outputs {
                let normalized = normalized_path(output)?;
                if normalized == normalized_input || !normalized_outputs.insert(normalized) {
                    return Err(OperationError::InvalidPath {
                        reason: format!("invalid or duplicate split output {}", output.display()),
                    });
                }
            }
            let snapshot = read_snapshot(input)?;
            let mut reader = PdfReader::new(Cursor::new(&snapshot))
                .map_err(|error| OperationError::ParseError(error.to_string()))?;
            let page_count = reader
                .page_count()
                .map_err(|error| OperationError::ParseError(error.to_string()))?
                as usize;
            let reports = ranges
                .iter()
                .map(|range| {
                    let pages = range.get_indices(page_count)?;
                    let merge_input =
                        ExistingDocumentMergeInput::with_pages(input, PageRange::List(pages));
                    reconstructive_report_from_snapshots(
                        std::slice::from_ref(&merge_input),
                        std::slice::from_ref(&snapshot),
                        policy,
                    )
                })
                .collect::<OperationResult<Vec<_>>>()?;
            let snapshot_file = stage_output(Path::new("snapshot.pdf"), &snapshot)?;
            let mut staged = Vec::with_capacity(outputs.len());
            for (report, output) in reports.iter().zip(outputs) {
                let parent = output
                    .parent()
                    .filter(|parent| !parent.as_os_str().is_empty())
                    .unwrap_or_else(|| Path::new("."));
                let temporary = tempfile::NamedTempFile::new_in(parent)?;
                super::page_extraction::extract_pages_to_file(
                    snapshot_file.path(),
                    &report.inputs[0].selected_pages,
                    temporary.path(),
                )?;
                temporary.as_file().sync_all()?;
                staged.push(temporary);
            }
            let backups = outputs
                .iter()
                .map(|path| {
                    if path.exists() {
                        std::fs::read(path).map(Some)
                    } else {
                        Ok(None)
                    }
                })
                .collect::<Result<Vec<_>, _>>()?;
            for index in 0..staged.len() {
                let temporary = staged.remove(0);
                if let Err(error) = temporary.persist(&outputs[index]) {
                    for rollback in 0..index {
                        match &backups[rollback] {
                            Some(bytes) => {
                                stage_output(&outputs[rollback], bytes)?
                                    .persist(&outputs[rollback])
                                    .map_err(|persist| OperationError::Io(persist.error))?;
                            }
                            None => {
                                let _ = std::fs::remove_file(&outputs[rollback]);
                            }
                        }
                    }
                    return Err(OperationError::Io(error.error));
                }
            }
            Ok(reports)
        }
    }
}

/// Plan a v4 preservation-first extraction.
pub fn plan_extract_pdf_pages_lossless(
    input: impl AsRef<Path>,
    pages: &[usize],
) -> OperationResult<SemanticPreservationReport> {
    plan_extract_pdf_pages(input, pages, ExistingDocumentPolicy::preserve_base())
}

/// Execute a v4 preservation-first extraction.
pub fn extract_pdf_pages_lossless(
    input: impl AsRef<Path>,
    output: impl AsRef<Path>,
    pages: &[usize],
) -> OperationResult<SemanticPreservationReport> {
    extract_pdf_pages(
        input,
        output,
        pages,
        ExistingDocumentPolicy::preserve_base(),
    )
}

/// Plan a v4 preservation-first split.
pub fn plan_split_pdf_lossless(
    input: impl AsRef<Path>,
    ranges: &[PageRange],
) -> OperationResult<Vec<SemanticPreservationReport>> {
    plan_split_pdf(input, ranges, ExistingDocumentPolicy::preserve_base())
}

/// Execute a v4 preservation-first split.
pub fn split_pdf_lossless(
    input: impl AsRef<Path>,
    ranges: &[PageRange],
    outputs: &[PathBuf],
) -> OperationResult<Vec<SemanticPreservationReport>> {
    split_pdf(
        input,
        ranges,
        outputs,
        ExistingDocumentPolicy::preserve_base(),
    )
}

/// Plan a v4 preservation-first merge.
pub fn plan_merge_pdfs_lossless(
    inputs: &[LosslessMergeInput],
) -> OperationResult<SemanticPreservationReport> {
    plan_merge_pdfs(inputs, ExistingDocumentPolicy::preserve_base())
}

/// Execute a v4 preservation-first merge.
pub fn merge_pdfs_lossless(
    inputs: &[LosslessMergeInput],
    output: impl AsRef<Path>,
) -> OperationResult<SemanticPreservationReport> {
    merge_pdfs(inputs, output, ExistingDocumentPolicy::preserve_base())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn semantic_fixture() -> Vec<u8> {
        semantic_fixture_with_field(
            b"<< /Type /Annot /Subtype /Widget /FT /Tx /T (field) /P 3 0 R /Rect [0 0 10 10] >>",
        )
    }

    fn semantic_fixture_with_field(field: &[u8]) -> Vec<u8> {
        let objects = [
            b"<< /Type /Catalog /Pages 2 0 R /AcroForm << /Fields [6 0 R] >> /Outlines << /Type /Outlines /Count 0 >> /Names << /EmbeddedFiles << /Names [(attachment.txt) << /F (attachment.txt) >>] >> >> /Dests << >> /OCProperties << /OCGs [] /D << >> >> /StructTreeRoot << /Type /StructTreeRoot /K [] >> /Metadata 4 0 R /PageLabels << /Nums [] >> >>".as_slice(),
            b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>".as_slice(),
            b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] /Resources << >> /StructParents 0 /Annots [5 0 R 6 0 R] >>".as_slice(),
            b"<< /Type /Metadata /Subtype /XML /Length 0 >>\nstream\n\nendstream".as_slice(),
            b"<< /Type /Annot /Subtype /Link /Rect [0 0 10 10] /Dest [3 0 R /Fit] >>".as_slice(),
            field,
        ];
        let mut bytes = b"%PDF-1.7\n".to_vec();
        let mut offsets = Vec::new();
        for (index, object) in objects.iter().enumerate() {
            offsets.push(bytes.len());
            bytes.extend_from_slice(format!("{} 0 obj\n", index + 1).as_bytes());
            bytes.extend_from_slice(object);
            bytes.extend_from_slice(b"\nendobj\n");
        }
        let xref = bytes.len();
        bytes.extend_from_slice(b"xref\n0 7\n0000000000 65535 f \n");
        for offset in offsets {
            bytes.extend_from_slice(format!("{offset:010} 00000 n \n").as_bytes());
        }
        bytes.extend_from_slice(
            format!("trailer\n<< /Size 7 /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n").as_bytes(),
        );
        bytes
    }

    #[test]
    fn inventories_digital_signatures_separately_from_forms() {
        let bytes = semantic_fixture_with_field(
            b"<< /Type /Annot /Subtype /Widget /FT /Sig /T (signature) /P 3 0 R /Rect [0 0 10 10] /V << /Type /Sig /Filter /Adobe.PPKLite /ByteRange [0 0 0 0] /Contents () >> >>",
        );
        let report = inspect_input_bytes(
            Path::new("signed.pdf"),
            &bytes,
            InputSemanticRole::PreservedBase,
            None,
            ExistingDocumentPolicy::preserve_base(),
        )
        .unwrap();

        assert!(report.structures.iter().any(|entry| {
            entry.structure == DocumentStructure::DigitalSignatures
                && entry.disposition == StructureDisposition::Preserved
        }));
    }

    #[test]
    fn empty_annotations_array_is_not_reported_as_document_semantics() {
        let fixture = String::from_utf8(semantic_fixture()).unwrap();
        let bytes = fixture
            .replace("/Annots [5 0 R 6 0 R]", "/Annots []")
            .into_bytes();
        let report = inspect_input_bytes(
            Path::new("empty-annots.pdf"),
            &bytes,
            InputSemanticRole::ImportedPages,
            None,
            ExistingDocumentPolicy::preserve_base(),
        )
        .unwrap();

        assert!(!report
            .structures
            .iter()
            .any(|entry| entry.structure == DocumentStructure::Annotations));
    }

    #[test]
    fn inventories_every_required_catalog_structure_with_explicit_dispositions() {
        let bytes = semantic_fixture();
        let path = Path::new("semantic-fixture.pdf");
        let base = inspect_input_bytes(
            path,
            &bytes,
            InputSemanticRole::PreservedBase,
            None,
            ExistingDocumentPolicy::preserve_base(),
        )
        .unwrap();
        assert_eq!(base.structures.len(), DOCUMENT_STRUCTURES.len() + 1);
        assert!(base
            .structures
            .iter()
            .all(|entry| entry.disposition == StructureDisposition::Preserved));

        let imported = inspect_input_bytes(
            path,
            &bytes,
            InputSemanticRole::ImportedPages,
            None,
            ExistingDocumentPolicy::preserve_base(),
        )
        .unwrap();
        assert_eq!(imported.structures.len(), DOCUMENT_STRUCTURES.len() + 1);
        assert!(imported.structures.iter().any(|entry| {
            entry.structure == DocumentStructure::Annotations
                && entry.disposition == StructureDisposition::Rejected
        }));
        assert!(imported
            .structures
            .iter()
            .any(|entry| entry.structure == DocumentStructure::MetadataStream
                && entry.disposition == StructureDisposition::FirstInputWins));
        assert_eq!(
            imported
                .structures
                .iter()
                .filter(|entry| entry.disposition == StructureDisposition::Rejected)
                .count(),
            DOCUMENT_STRUCTURES.len()
        );
    }

    #[test]
    fn all_page_selection_preserves_annotations_forms_links_and_catalog_bytes_exactly() {
        let bytes = semantic_fixture();
        let batch = selection_batch(1, &[0]).unwrap();
        let (output, report) = materialize_batch(&bytes, &batch, 1).unwrap();
        assert_eq!(output, bytes);
        assert_eq!(report.page_count, 1);
        assert!(report.replaced_objects.is_empty());
    }
}
