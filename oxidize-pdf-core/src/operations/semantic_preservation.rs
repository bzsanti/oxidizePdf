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
    /// Per-input semantic inventory and policy.
    pub inputs: Vec<InputSemanticReport>,
    /// Exact object-level page mutation plan.
    pub mutation: PageMutationReport,
}

/// Input to a lossless merge.
#[derive(Debug, Clone)]
pub struct LosslessMergeInput {
    /// PDF path.
    pub path: PathBuf,
    /// Pages to include, or all pages when omitted.
    pub pages: Option<PageRange>,
}

impl LosslessMergeInput {
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

fn inspect_input_bytes(
    path: &Path,
    bytes: &[u8],
    role: InputSemanticRole,
    range: Option<&PageRange>,
) -> OperationResult<InputSemanticReport> {
    let mut reader = PdfReader::new(Cursor::new(bytes)).map_err(|error| {
        OperationError::ParseError(format!("open snapshot of {}: {error}", path.display()))
    })?;
    if reader.is_encrypted() {
        return Err(OperationError::PdfError(crate::PdfError::PermissionDenied(
            format!(
                "lossless semantic operations do not support encrypted PDF {}",
                path.display()
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
    let structures = DOCUMENT_STRUCTURES
        .iter()
        .filter_map(|(key, structure)| {
            catalog
                .contains_key(key)
                .then_some(StructureSemanticReport {
                    structure: *structure,
                    disposition: match role {
                        InputSemanticRole::PreservedBase => StructureDisposition::Preserved,
                        InputSemanticRole::ImportedPages
                            if *structure == DocumentStructure::MetadataStream =>
                        {
                            StructureDisposition::FirstInputWins
                        }
                        InputSemanticRole::ImportedPages => StructureDisposition::Rejected,
                    },
                })
        })
        .collect();
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
pub fn plan_extract_pdf_pages_lossless(
    input: impl AsRef<Path>,
    pages: &[usize],
) -> OperationResult<SemanticPreservationReport> {
    let input = input.as_ref();
    if pages.is_empty() {
        return Err(OperationError::NoPagesToProcess);
    }
    let range = PageRange::List(pages.to_vec());
    let base = read_snapshot(input)?;
    let report = inspect_input_bytes(input, &base, InputSemanticRole::PreservedBase, Some(&range))?;
    let page_count = PdfReader::new(Cursor::new(&base))
        .map_err(|error| OperationError::ParseError(error.to_string()))?
        .page_count()
        .map_err(|error| OperationError::ParseError(error.to_string()))?
        as usize;
    let batch = selection_batch(page_count, &report.selected_pages)?;
    let mutation = plan_batch(&base, &batch, page_count)?;
    Ok(SemanticPreservationReport {
        inputs: vec![report],
        mutation,
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
/// [`plan_extract_pdf_pages_lossless`], or when atomic publication fails.
pub fn extract_pdf_pages_lossless(
    input: impl AsRef<Path>,
    output: impl AsRef<Path>,
    pages: &[usize],
) -> OperationResult<SemanticPreservationReport> {
    let input = input.as_ref();
    let output = output.as_ref();
    let base = read_snapshot(input)?;
    let range = PageRange::List(pages.to_vec());
    let input_report =
        inspect_input_bytes(input, &base, InputSemanticRole::PreservedBase, Some(&range))?;
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
        inputs: vec![input_report],
        mutation: planned,
    };
    Ok(report)
}

/// Plan every output of a lossless split without writing files.
///
/// # Errors
///
/// Returns an error when a range is empty or cannot be represented while
/// preserving the source's reachable document semantics.
pub fn plan_split_pdf_lossless(
    input: impl AsRef<Path>,
    ranges: &[PageRange],
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
            let report =
                inspect_input_bytes(input, &base, InputSemanticRole::PreservedBase, Some(range))?;
            let batch = selection_batch(page_count, &pages)?;
            let mutation = plan_batch(&base, &batch, page_count)?;
            Ok(SemanticPreservationReport {
                inputs: vec![report],
                mutation,
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
pub fn split_pdf_lossless(
    input: impl AsRef<Path>,
    ranges: &[PageRange],
    outputs: &[PathBuf],
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
        let input_report =
            inspect_input_bytes(input, &base, InputSemanticRole::PreservedBase, Some(range))?;
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
            inputs: vec![input_report],
            mutation: planned,
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
pub fn plan_merge_pdfs_lossless(
    inputs: &[LosslessMergeInput],
) -> OperationResult<SemanticPreservationReport> {
    let Some(first) = inputs.first() else {
        return Err(OperationError::NoPagesToProcess);
    };
    let snapshots = inputs
        .iter()
        .map(|input| read_snapshot(&input.path))
        .collect::<OperationResult<Vec<_>>>()?;
    let base = inspect_input_bytes(
        &first.path,
        &snapshots[0],
        InputSemanticRole::PreservedBase,
        first.pages.as_ref(),
    )?;
    let base_count = PdfReader::new(Cursor::new(&snapshots[0]))
        .map_err(|error| OperationError::ParseError(error.to_string()))?
        .page_count()
        .map_err(|error| OperationError::ParseError(error.to_string()))?
        as usize;
    let mut batch = selection_batch(base_count, &base.selected_pages)?;
    let mut reports = vec![base];
    let mut insertion_index = reports[0].selected_pages.len();
    for (input, snapshot) in inputs.iter().skip(1).zip(snapshots.iter().skip(1)) {
        let report = inspect_input_bytes(
            &input.path,
            snapshot,
            InputSemanticRole::ImportedPages,
            input.pages.as_ref(),
        )?;
        reject_secondary_document_structures(&report)?;
        for &page in &report.selected_pages {
            batch.operations.push(PageMutation::Insert {
                source: input.path.clone(),
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
        inputs: reports,
        mutation,
    })
}

/// Merge PDFs losslessly and publish the validated output atomically.
///
/// # Errors
///
/// Returns an error under the same conditions as [`plan_merge_pdfs_lossless`],
/// or when atomic publication or output validation fails.
pub fn merge_pdfs_lossless(
    inputs: &[LosslessMergeInput],
    output: impl AsRef<Path>,
) -> OperationResult<SemanticPreservationReport> {
    let Some(first) = inputs.first() else {
        return Err(OperationError::NoPagesToProcess);
    };
    let snapshots = inputs
        .iter()
        .map(|input| read_snapshot(&input.path))
        .collect::<OperationResult<Vec<_>>>()?;
    let base_report = inspect_input_bytes(
        &first.path,
        &snapshots[0],
        InputSemanticRole::PreservedBase,
        first.pages.as_ref(),
    )?;
    let base_count = PdfReader::new(Cursor::new(&snapshots[0]))
        .map_err(|error| OperationError::ParseError(error.to_string()))?
        .page_count()
        .map_err(|error| OperationError::ParseError(error.to_string()))?
        as usize;
    let mut batch = selection_batch(base_count, &base_report.selected_pages)?;
    let mut reports = vec![base_report];
    let mut at = reports[0].selected_pages.len();
    for (input, snapshot) in inputs.iter().zip(&snapshots).skip(1) {
        let input_report = inspect_input_bytes(
            &input.path,
            snapshot,
            InputSemanticRole::ImportedPages,
            input.pages.as_ref(),
        )?;
        reject_secondary_document_structures(&input_report)?;
        for &page in &input_report.selected_pages {
            batch.operations.push(PageMutation::Insert {
                source: input.path.clone(),
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
        inputs: reports,
        mutation: planned,
    };
    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn semantic_fixture() -> Vec<u8> {
        let objects = [
            b"<< /Type /Catalog /Pages 2 0 R /AcroForm << /Fields [6 0 R] >> /Outlines << /Type /Outlines /Count 0 >> /Names << /EmbeddedFiles << /Names [(attachment.txt) << /F (attachment.txt) >>] >> >> /Dests << >> /OCProperties << /OCGs [] /D << >> >> /StructTreeRoot << /Type /StructTreeRoot /K [] >> /Metadata 4 0 R /PageLabels << /Nums [] >> >>".as_slice(),
            b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>".as_slice(),
            b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] /Resources << >> /StructParents 0 /Annots [5 0 R 6 0 R] >>".as_slice(),
            b"<< /Type /Metadata /Subtype /XML /Length 0 >>\nstream\n\nendstream".as_slice(),
            b"<< /Type /Annot /Subtype /Link /Rect [0 0 10 10] /Dest [3 0 R /Fit] >>".as_slice(),
            b"<< /Type /Annot /Subtype /Widget /FT /Tx /T (field) /P 3 0 R /Rect [0 0 10 10] >>".as_slice(),
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
    fn inventories_every_required_catalog_structure_with_explicit_dispositions() {
        let bytes = semantic_fixture();
        let path = Path::new("semantic-fixture.pdf");
        let base =
            inspect_input_bytes(path, &bytes, InputSemanticRole::PreservedBase, None).unwrap();
        assert_eq!(base.structures.len(), DOCUMENT_STRUCTURES.len());
        assert!(base
            .structures
            .iter()
            .all(|entry| entry.disposition == StructureDisposition::Preserved));

        let imported =
            inspect_input_bytes(path, &bytes, InputSemanticRole::ImportedPages, None).unwrap();
        assert_eq!(imported.structures.len(), DOCUMENT_STRUCTURES.len());
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
            DOCUMENT_STRUCTURES.len() - 1
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
