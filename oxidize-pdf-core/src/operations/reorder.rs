//! PDF page reordering functionality
//!
//! This module provides functionality to reorder pages within a PDF document.

use super::{OperationError, OperationResult};
use crate::error::PdfError;
use crate::parser::objects::{PdfArray, PdfDictionary, PdfObject};
use crate::parser::page_tree::ParsedPage;
use crate::parser::{PdfDocument, PdfReader};
use crate::signatures::{ensure_modification_allowed, IncrementalModification};
use crate::writer::IncrementalUpdate;
use crate::{Document, Page};
use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::io::{Cursor, Read, Seek, SeekFrom, Write};
use std::path::Path;

const INHERITABLE_PAGE_KEYS: [&str; 4] = ["Resources", "MediaBox", "CropBox", "Rotate"];
const MAX_PAGE_TREE_DEPTH: usize = 256;
const MAX_PAGE_COUNT: usize = 100_000;
const MAX_CLONED_OBJECTS_PER_PAGE: usize = 100_000;
const MAX_OBJECT_GRAPH_DEPTH: usize = 256;

#[derive(Debug)]
struct LosslessPage {
    reference: (u32, u16),
    dictionary: PdfDictionary,
    effective_inherited: HashMap<&'static str, PdfObject>,
}

#[derive(Debug)]
struct LosslessValidation {
    root_reference: (u32, u16),
    catalog: PdfDictionary,
    pages: Vec<((u32, u16), HashMap<&'static str, PdfObject>)>,
}

/// One operation in an atomic lossless page-tree mutation batch.
///
/// Page indexes are zero based and are resolved against the result of all
/// preceding operations in the batch.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PageMutation {
    /// Move a page to a new position.
    Move { from: usize, to: usize },
    /// Rotate a page clockwise by a multiple of 90 degrees.
    Rotate { page: usize, degrees: i32 },
    /// Remove a page from the page tree.
    Delete { page: usize },
    /// Duplicate a page, inserting the independent clone at `at`.
    Duplicate { page: usize, at: usize },
    /// Import one page from another PDF file and insert it at `at`.
    Insert {
        source: std::path::PathBuf,
        page: usize,
        at: usize,
    },
}

/// A sequence of page mutations committed as one incremental revision.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PageMutationBatch {
    /// Operations to apply in order.
    pub operations: Vec<PageMutation>,
}

impl PageMutationBatch {
    /// Create an empty batch.
    pub fn new() -> Self {
        Self::default()
    }

    /// Append an operation.
    pub fn push(&mut self, operation: PageMutation) -> &mut Self {
        self.operations.push(operation);
        self
    }
}

/// Objects affected by a planned or completed page-tree mutation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PageMutationReport {
    /// Existing indirect objects that will receive a new definition.
    pub replaced_objects: Vec<(u32, u16)>,
    /// New indirect objects allocated by the revision.
    pub added_objects: Vec<(u32, u16)>,
    /// Source objects reachable from the old catalog but not the new catalog.
    pub unreachable_objects: Vec<(u32, u16)>,
    /// Number of pages after applying the batch.
    pub page_count: usize,
}

#[derive(Debug, Clone)]
enum PlannedPage {
    Existing {
        source_index: usize,
        rotation: Option<i32>,
    },
    Clone {
        source_index: usize,
        rotation: Option<i32>,
    },
    Import {
        source: std::path::PathBuf,
        source_index: usize,
        rotation: Option<i32>,
    },
}

struct ImportedDocument {
    reader: PdfReader<Cursor<Vec<u8>>>,
    pages: Vec<LosslessPage>,
}

/// Options for page reordering
#[derive(Debug, Clone)]
pub struct ReorderOptions {
    /// The new order of pages (0-based indices)
    pub page_order: Vec<usize>,
    /// Whether to preserve document metadata
    pub preserve_metadata: bool,
    /// Whether to optimize the output
    pub optimize: bool,
}

impl Default for ReorderOptions {
    fn default() -> Self {
        Self {
            page_order: Vec::new(),
            preserve_metadata: true,
            optimize: false,
        }
    }
}

/// Page reorderer
pub struct PageReorderer {
    document: PdfDocument<File>,
    options: ReorderOptions,
}

impl PageReorderer {
    /// Create a new page reorderer
    pub fn new(document: PdfDocument<File>, options: ReorderOptions) -> Self {
        Self { document, options }
    }

    /// Reorder pages according to the specified order
    pub fn reorder(&self) -> OperationResult<Document> {
        let total_pages =
            self.document
                .page_count()
                .map_err(|e| OperationError::ParseError(e.to_string()))? as usize;

        if total_pages == 0 {
            return Err(OperationError::NoPagesToProcess);
        }

        // Validate page order
        self.validate_page_order(total_pages)?;

        // Create new document
        let mut output_doc = Document::new();

        // Copy metadata if requested
        if self.options.preserve_metadata {
            self.copy_metadata(&mut output_doc)?;
        }

        // Add pages in the new order
        for &page_idx in &self.options.page_order {
            let parsed_page = self
                .document
                .get_page(page_idx as u32)
                .map_err(|e| OperationError::ParseError(e.to_string()))?;

            let page = self.convert_page(&parsed_page)?;
            output_doc.add_page(page);
        }

        Ok(output_doc)
    }

    /// Reorder pages and save to file
    pub fn reorder_to_file<P: AsRef<Path>>(&self, output_path: P) -> OperationResult<()> {
        let mut doc = self.reorder()?;
        doc.save(output_path)?;
        Ok(())
    }

    /// Validate that the page order is valid
    fn validate_page_order(&self, total_pages: usize) -> OperationResult<()> {
        if self.options.page_order.is_empty() {
            return Err(OperationError::InvalidPageRange(
                "Page order cannot be empty".to_string(),
            ));
        }

        // Check for out of bounds indices
        for &idx in &self.options.page_order {
            if idx >= total_pages {
                return Err(OperationError::InvalidPageRange(format!(
                    "Page index {idx} is out of bounds (document has {total_pages} pages)"
                )));
            }
        }

        Ok(())
    }

    /// Copy metadata from source to destination document
    fn copy_metadata(&self, doc: &mut Document) -> OperationResult<()> {
        if let Ok(metadata) = self.document.metadata() {
            if let Some(title) = metadata.title {
                doc.set_title(&title);
            }
            if let Some(author) = metadata.author {
                doc.set_author(&author);
            }
            if let Some(subject) = metadata.subject {
                doc.set_subject(&subject);
            }
            if let Some(keywords) = metadata.keywords {
                doc.set_keywords(&keywords);
            }
        }
        Ok(())
    }

    /// Convert a parsed page to a new page, preserving its content verbatim.
    ///
    /// Copies the original content streams and resources unchanged via
    /// [`Page::from_parsed_with_content`] (the path `merge` uses) instead of
    /// re-emitting the page through the high-level API, which mapped every font
    /// to one of the standard 14, decoded bytes with `from_utf8_lossy`, and
    /// dropped images, XObjects and unrecognized operators (#453).
    fn convert_page(&self, parsed_page: &ParsedPage) -> OperationResult<Page> {
        Page::from_parsed_with_content(parsed_page, &self.document)
            .map_err(|e| OperationError::ParseError(e.to_string()))
    }
}

/// Convenience function to reorder pages in a PDF
pub fn reorder_pdf_pages<P: AsRef<Path>, Q: AsRef<Path>>(
    input_path: P,
    output_path: Q,
    page_order: Vec<usize>,
) -> OperationResult<()> {
    let document = PdfReader::open_document(input_path)
        .map_err(|e| OperationError::ParseError(e.to_string()))?;

    let options = ReorderOptions {
        page_order,
        preserve_metadata: true,
        optimize: false,
    };

    let reorderer = PageReorderer::new(document, options);
    reorderer.reorder_to_file(output_path)
}

/// Inspect an atomic page-tree mutation without writing an output file.
///
/// This is an exact dry run: imported object graphs and reachability are
/// analyzed in memory, but the prospective revision is neither serialized nor
/// reopened. It returns the same object report as [`mutate_pdf_pages_lossless`]
/// without creating or replacing a destination file.
///
/// # Errors
///
/// Returns an error for malformed or encrypted PDFs, forbidden DocMDP edits,
/// invalid indexes, dangling semantic references, unsupported page labels,
/// or imports that require catalog-level AcroForm, tagged-PDF, named-
/// destination, or optional-content remapping.
pub fn plan_pdf_page_mutations<P: AsRef<Path>>(
    input_path: P,
    batch: &PageMutationBatch,
) -> OperationResult<PageMutationReport> {
    let base = std::fs::read(input_path)?;
    let (_, _, report) = mutate_pdf_bytes_lossless(&base, batch, false)?;
    Ok(report)
}

/// Apply page-tree mutations as one lossless incremental revision.
///
/// The source bytes remain an exact prefix. The completed temporary file is
/// reopened and checked before it atomically replaces `output_path`.
///
/// # Errors
///
/// Returns an error under the same conditions as
/// [`plan_pdf_page_mutations`], or when temporary-file creation, validation,
/// syncing, or atomic publication fails. The destination is not replaced when
/// validation fails.
pub fn mutate_pdf_pages_lossless<P: AsRef<Path>, Q: AsRef<Path>>(
    input_path: P,
    output_path: Q,
    batch: &PageMutationBatch,
) -> OperationResult<PageMutationReport> {
    let base = std::fs::read(input_path)?;
    let (updated, expected, report) = mutate_pdf_bytes_lossless(&base, batch, true)?;
    let updated = updated.expect("materialized page mutation must return bytes");
    validate_lossless_output(&base, &updated, &expected)?;

    let output_path = output_path.as_ref();
    let parent = output_path
        .parent()
        .filter(|path| !path.as_os_str().is_empty());
    let mut temporary = tempfile::NamedTempFile::new_in(parent.unwrap_or_else(|| Path::new(".")))?;
    temporary.write_all(&updated)?;
    temporary.flush()?;
    temporary.as_file().sync_all()?;
    validate_lossless_file(&base, temporary.path(), &expected)?;
    temporary
        .persist(output_path)
        .map_err(|error| OperationError::Io(error.error))?;
    Ok(report)
}

fn mutate_pdf_bytes_lossless(
    base: &[u8],
    batch: &PageMutationBatch,
    materialize: bool,
) -> Result<(Option<Vec<u8>>, LosslessValidation, PageMutationReport), PdfError> {
    if batch.operations.is_empty() {
        return Err(invalid_lossless("page mutation batch cannot be empty"));
    }
    let mut reader = PdfReader::new(Cursor::new(base))
        .map_err(|error| invalid_lossless(format!("parse source PDF: {error}")))?;
    if reader.is_encrypted() {
        return Err(PdfError::PermissionDenied(
            "lossless page-tree mutation does not support encrypted PDFs".to_string(),
        ));
    }
    let catalog = reader
        .catalog()
        .map_err(|error| invalid_lossless(format!("read document catalog: {error}")))?
        .clone();
    if catalog.contains_key("PageLabels")
        && batch
            .operations
            .iter()
            .any(|operation| !matches!(operation, PageMutation::Rotate { .. }))
    {
        return Err(invalid_lossless(
            "page reordering, insertion, duplication, or deletion with catalog /PageLabels is unsupported because label indexes require remapping",
        ));
    }
    ensure_modification_allowed(
        &mut reader,
        &catalog,
        IncrementalModification::PageTreeMutation,
    )?;
    let root_reference = catalog
        .get("Pages")
        .and_then(PdfObject::as_reference)
        .ok_or_else(|| invalid_lossless("catalog /Pages must be an indirect reference"))?;
    let root_dictionary = object_dictionary(&mut reader, root_reference, "page-tree root")?;
    let mut source_pages = Vec::new();
    let mut visited = HashSet::new();
    walk_page_tree(
        &mut reader,
        root_reference,
        None,
        HashMap::new(),
        &mut visited,
        &mut source_pages,
        0,
    )?;

    let mut planned: Vec<_> = (0..source_pages.len())
        .map(|source_index| PlannedPage::Existing {
            source_index,
            rotation: None,
        })
        .collect();
    for operation in &batch.operations {
        apply_page_mutation(&mut planned, operation)?;
    }
    if planned.is_empty() {
        return Err(invalid_lossless(
            "a page-tree mutation cannot remove every page",
        ));
    }
    if planned.len() > MAX_PAGE_COUNT {
        return Err(invalid_lossless(
            "page mutation exceeds the supported page count",
        ));
    }

    let retained: HashSet<_> = planned
        .iter()
        .filter_map(|page| match page {
            PlannedPage::Existing { source_index, .. } => Some(*source_index),
            _ => None,
        })
        .collect();
    let deleted_refs: HashSet<_> = source_pages
        .iter()
        .enumerate()
        .filter(|(index, _)| !retained.contains(index))
        .map(|(_, page)| page.reference)
        .collect();
    ensure_catalog_does_not_reference_deleted_pages(
        &mut reader,
        &catalog,
        root_reference,
        &deleted_refs,
    )?;
    ensure_retained_pages_do_not_reference_deleted_pages(
        &mut reader,
        &source_pages,
        &retained,
        root_reference,
        &deleted_refs,
    )?;
    let source_reachable = reachable_from_catalog(&mut reader, &catalog)?;

    let root_inherited = inherited_values(&root_dictionary);
    let root_preserved_values: Vec<_> = root_dictionary
        .0
        .iter()
        .filter(|(key, _)| !matches!(key.0.as_str(), "Kids" | "Count"))
        .map(|(_, value)| value.clone())
        .collect();
    let mut update = IncrementalUpdate::from_base(base)?;
    let mut added_objects = Vec::new();
    let mut final_pages = Vec::with_capacity(planned.len());
    let mut validation_pages = Vec::with_capacity(planned.len());
    let mut replacements = HashSet::new();
    let mut preserved_source_refs = HashSet::new();
    replacements.insert(root_reference);
    let mut imported_documents = HashMap::new();
    for source in planned.iter().filter_map(|page| match page {
        PlannedPage::Import { source, .. } => Some(source),
        _ => None,
    }) {
        if !imported_documents.contains_key(source) {
            imported_documents.insert(source.clone(), load_imported_document(source)?);
        }
    }

    for plan in planned {
        match plan {
            PlannedPage::Existing {
                source_index,
                rotation,
            } => {
                let page = &source_pages[source_index];
                let mut dictionary = page.dictionary.clone();
                let mut changed = dictionary.get("Parent").and_then(PdfObject::as_reference)
                    != Some(root_reference);
                dictionary.insert(
                    "Parent".to_string(),
                    PdfObject::Reference(root_reference.0, root_reference.1),
                );
                materialize_inherited(
                    &mut dictionary,
                    &page.effective_inherited,
                    &root_inherited,
                    &mut changed,
                );
                apply_rotation(
                    &mut dictionary,
                    &page.effective_inherited,
                    rotation,
                    &mut changed,
                )?;
                let effective = effective_for_flat_page(&dictionary, &root_inherited);
                if changed {
                    update.replace(page.reference, PdfObject::Dictionary(dictionary))?;
                    replacements.insert(page.reference);
                }
                final_pages.push(page.reference);
                validation_pages.push((page.reference, effective));
            }
            PlannedPage::Clone {
                source_index,
                rotation,
            } => {
                let page = &source_pages[source_index];
                let id = clone_page_into_update(
                    &mut reader,
                    page,
                    root_reference,
                    rotation,
                    false,
                    &mut update,
                    &mut added_objects,
                    &mut preserved_source_refs,
                )?;
                let object = object_from_pending(&added_objects, id);
                final_pages.push(id);
                validation_pages.push((id, effective_for_flat_page(&object, &root_inherited)));
            }
            PlannedPage::Import {
                source,
                source_index,
                rotation,
            } => {
                let imported = imported_documents
                    .get_mut(&source)
                    .ok_or_else(|| invalid_lossless("planned import source was not loaded"))?;
                let page = imported.pages.get(source_index).ok_or_else(|| {
                    invalid_lossless(format!(
                        "imported page index {source_index} is out of bounds for {} pages",
                        imported.pages.len()
                    ))
                })?;
                let id = clone_page_into_update(
                    &mut imported.reader,
                    page,
                    root_reference,
                    rotation,
                    true,
                    &mut update,
                    &mut added_objects,
                    &mut preserved_source_refs,
                )?;
                let object = object_from_pending(&added_objects, id);
                final_pages.push(id);
                validation_pages.push((id, effective_for_flat_page(&object, &root_inherited)));
            }
        }
    }

    let mut root_replacement = root_dictionary;
    root_replacement.insert(
        "Kids".to_string(),
        PdfObject::Array(PdfArray(
            final_pages
                .iter()
                .map(|id| PdfObject::Reference(id.0, id.1))
                .collect(),
        )),
    );
    root_replacement.insert(
        "Count".to_string(),
        PdfObject::Integer(final_pages.len() as i64),
    );
    update.replace(root_reference, PdfObject::Dictionary(root_replacement))?;
    for (id, object) in &added_objects {
        update.replace(*id, object.clone())?;
    }
    let output_reachable = prospective_source_references(
        &mut reader,
        &catalog,
        root_reference,
        &root_preserved_values,
        &source_pages,
        &retained,
        &preserved_source_refs,
    )?;
    let mut unreachable_objects: Vec<_> = source_reachable
        .difference(&output_reachable)
        .copied()
        .collect();
    unreachable_objects.sort_unstable();
    let mut replaced_objects: Vec<_> = replacements.into_iter().collect();
    replaced_objects.sort_unstable();
    let mut added_ids: Vec<_> = added_objects.iter().map(|(id, _)| *id).collect();
    added_ids.sort_unstable();
    let report = PageMutationReport {
        replaced_objects,
        added_objects: added_ids,
        unreachable_objects,
        page_count: final_pages.len(),
    };
    let updated = if materialize {
        let bytes = update.finish()?;
        let mut output_reader = PdfReader::new(Cursor::new(&bytes))
            .map_err(|error| invalid_lossless(format!("reopen page mutation: {error}")))?;
        let output_catalog = output_reader
            .catalog()
            .map_err(|error| invalid_lossless(format!("read output catalog: {error}")))?
            .clone();
        let actual_reachable = reachable_from_catalog(&mut output_reader, &output_catalog)?;
        let mut actual_unreachable: Vec<_> = source_reachable
            .difference(&actual_reachable)
            .copied()
            .collect();
        actual_unreachable.sort_unstable();
        if actual_unreachable != report.unreachable_objects {
            return Err(invalid_lossless(
                "dry-run reachability report differs from the materialized revision",
            ));
        }
        Some(bytes)
    } else {
        None
    };
    Ok((
        updated,
        LosslessValidation {
            root_reference,
            catalog,
            pages: validation_pages,
        },
        report,
    ))
}

/// Plan a mutation against an immutable caller-owned source snapshot.
pub(crate) fn plan_pdf_page_mutations_from_bytes(
    base: &[u8],
    batch: &PageMutationBatch,
) -> OperationResult<PageMutationReport> {
    let (_, _, report) = mutate_pdf_bytes_lossless(base, batch, false)?;
    Ok(report)
}

/// Materialize and validate a mutation without publishing a destination.
pub(crate) fn materialize_pdf_page_mutations_from_bytes(
    base: &[u8],
    batch: &PageMutationBatch,
) -> OperationResult<(Vec<u8>, PageMutationReport)> {
    let (updated, expected, report) = mutate_pdf_bytes_lossless(base, batch, true)?;
    let updated =
        updated.ok_or_else(|| invalid_lossless("materialized mutation returned no bytes"))?;
    validate_lossless_output(base, &updated, &expected)?;
    Ok((updated, report))
}

fn load_imported_document(path: &Path) -> Result<ImportedDocument, PdfError> {
    let bytes = std::fs::read(path).map_err(|error| {
        invalid_lossless(format!("read imported PDF {}: {error}", path.display()))
    })?;
    let mut reader = PdfReader::new(Cursor::new(bytes)).map_err(|error| {
        invalid_lossless(format!("parse imported PDF {}: {error}", path.display()))
    })?;
    if reader.is_encrypted() {
        return Err(PdfError::PermissionDenied(format!(
            "cannot import a page from encrypted PDF {}",
            path.display()
        )));
    }
    let catalog = reader
        .catalog()
        .map_err(|error| invalid_lossless(format!("read imported catalog: {error}")))?
        .clone();
    let root = catalog
        .get("Pages")
        .and_then(PdfObject::as_reference)
        .ok_or_else(|| invalid_lossless("imported catalog /Pages is not indirect"))?;
    let mut pages = Vec::new();
    walk_page_tree(
        &mut reader,
        root,
        None,
        HashMap::new(),
        &mut HashSet::new(),
        &mut pages,
        0,
    )?;
    Ok(ImportedDocument { reader, pages })
}

fn apply_page_mutation(
    pages: &mut Vec<PlannedPage>,
    operation: &PageMutation,
) -> Result<(), PdfError> {
    let check_page = |index: usize, len: usize, role: &str| {
        if index < len {
            Ok(())
        } else {
            Err(invalid_lossless(format!(
                "{role} page index {index} is out of bounds for {len} pages"
            )))
        }
    };
    match operation {
        PageMutation::Move { from, to } => {
            check_page(*from, pages.len(), "move source")?;
            check_page(*to, pages.len(), "move destination")?;
            let page = pages.remove(*from);
            pages.insert(*to, page);
        }
        PageMutation::Rotate { page, degrees } => {
            check_page(*page, pages.len(), "rotation")?;
            if degrees.rem_euclid(90) != 0 {
                return Err(invalid_lossless(format!(
                    "page rotation {degrees} must be a multiple of 90 degrees"
                )));
            }
            let rotation = match &mut pages[*page] {
                PlannedPage::Existing { rotation, .. }
                | PlannedPage::Clone { rotation, .. }
                | PlannedPage::Import { rotation, .. } => rotation,
            };
            *rotation = Some(
                rotation
                    .unwrap_or(0)
                    .checked_add(*degrees)
                    .ok_or_else(|| invalid_lossless("accumulated page rotation overflows i32"))?,
            );
        }
        PageMutation::Delete { page } => {
            check_page(*page, pages.len(), "delete")?;
            pages.remove(*page);
        }
        PageMutation::Duplicate { page, at } => {
            check_page(*page, pages.len(), "duplicate source")?;
            if *at > pages.len() {
                return Err(invalid_lossless(format!(
                    "duplicate insertion index {at} is out of bounds for {} pages",
                    pages.len()
                )));
            }
            let clone = match &pages[*page] {
                PlannedPage::Existing {
                    source_index,
                    rotation,
                }
                | PlannedPage::Clone {
                    source_index,
                    rotation,
                } => PlannedPage::Clone {
                    source_index: *source_index,
                    rotation: *rotation,
                },
                PlannedPage::Import {
                    source,
                    source_index,
                    rotation,
                } => PlannedPage::Import {
                    source: source.clone(),
                    source_index: *source_index,
                    rotation: *rotation,
                },
            };
            pages.insert(*at, clone);
        }
        PageMutation::Insert { source, page, at } => {
            if *at > pages.len() {
                return Err(invalid_lossless(format!(
                    "import insertion index {at} is out of bounds for {} pages",
                    pages.len()
                )));
            }
            pages.insert(
                *at,
                PlannedPage::Import {
                    source: source.clone(),
                    source_index: *page,
                    rotation: None,
                },
            );
        }
    }
    Ok(())
}

fn materialize_inherited(
    dictionary: &mut PdfDictionary,
    effective: &HashMap<&'static str, PdfObject>,
    root_effective: &HashMap<&'static str, PdfObject>,
    changed: &mut bool,
) {
    for (key, value) in effective {
        if dictionary.contains_key(key) || root_effective.get(key) == Some(value) {
            continue;
        }
        dictionary.insert((*key).to_string(), value.clone());
        *changed = true;
    }
}

fn apply_rotation(
    dictionary: &mut PdfDictionary,
    effective: &HashMap<&'static str, PdfObject>,
    delta: Option<i32>,
    changed: &mut bool,
) -> Result<(), PdfError> {
    let Some(delta) = delta else { return Ok(()) };
    let current = effective
        .get("Rotate")
        .and_then(PdfObject::as_integer)
        .unwrap_or(0);
    let current = i32::try_from(current)
        .map_err(|_| invalid_lossless("effective page /Rotate does not fit in i32"))?;
    let rotation = current
        .checked_add(delta)
        .ok_or_else(|| invalid_lossless("effective page rotation overflows i32"))?
        .rem_euclid(360);
    dictionary.insert("Rotate".to_string(), PdfObject::Integer(rotation as i64));
    *changed = true;
    Ok(())
}

fn effective_for_flat_page(
    dictionary: &PdfDictionary,
    root: &HashMap<&'static str, PdfObject>,
) -> HashMap<&'static str, PdfObject> {
    let mut effective = root.clone();
    for key in INHERITABLE_PAGE_KEYS {
        if let Some(value) = dictionary.get(key) {
            effective.insert(key, value.clone());
        }
    }
    effective
}

fn clone_page_into_update<R: Read + Seek>(
    reader: &mut PdfReader<R>,
    page: &LosslessPage,
    destination_root: (u32, u16),
    rotation: Option<i32>,
    external: bool,
    update: &mut IncrementalUpdate<'_>,
    added: &mut Vec<((u32, u16), PdfObject)>,
    preserved_source_refs: &mut HashSet<(u32, u16)>,
) -> Result<(u32, u16), PdfError> {
    ensure_page_can_be_cloned(reader, page, external)?;
    let page_id = update.allocate_id()?;
    let mut mapping = HashMap::new();
    mapping.insert(page.reference, page_id);
    let mut dictionary = page.dictionary.clone();
    dictionary.0.retain(|key, _| key.0 != "Parent");
    for (key, value) in &page.effective_inherited {
        if !dictionary.contains_key(key) {
            dictionary.insert((*key).to_string(), value.clone());
        }
    }
    let mut keys: Vec<_> = dictionary.0.keys().cloned().collect();
    keys.sort_by(|left, right| left.0.cmp(&right.0));
    for key in keys {
        let value = dictionary.0[&key].clone();
        let cloned = clone_page_object(
            reader,
            &value,
            page.reference,
            external,
            update,
            added,
            &mut mapping,
            preserved_source_refs,
            0,
        )?;
        dictionary.0.insert(key, cloned);
    }
    dictionary.insert(
        "Parent".to_string(),
        PdfObject::Reference(destination_root.0, destination_root.1),
    );
    let mut changed = false;
    apply_rotation(
        &mut dictionary,
        &page.effective_inherited,
        rotation,
        &mut changed,
    )?;
    added.push((page_id, PdfObject::Dictionary(dictionary)));
    Ok(page_id)
}

fn ensure_page_can_be_cloned<R: Read + Seek>(
    reader: &mut PdfReader<R>,
    page: &LosslessPage,
    external: bool,
) -> Result<(), PdfError> {
    let action = if external { "import" } else { "duplicate" };
    if page.dictionary.contains_key("StructParents") {
        return Err(invalid_lossless(format!(
            "cannot {action} a tagged page without remapping its parent-tree entry"
        )));
    }
    if external {
        ensure_import_graph_supported(reader, page)?;
    }
    let Some(annots) = page.dictionary.get("Annots") else {
        return Ok(());
    };
    let annotations = match annots {
        PdfObject::Array(array) => array.clone(),
        PdfObject::Reference(number, generation) => reader
            .get_object(*number, *generation)
            .map_err(|error| invalid_lossless(format!("resolve page /Annots: {error}")))?
            .as_array()
            .cloned()
            .ok_or_else(|| invalid_lossless("indirect page /Annots is not an array"))?,
        _ => {
            return Err(invalid_lossless(
                "page /Annots must be an array or reference",
            ))
        }
    };
    for annotation in annotations.0 {
        let dictionary = match annotation {
            PdfObject::Reference(number, generation) => reader
                .get_object(number, generation)
                .map_err(|error| invalid_lossless(format!("resolve page annotation: {error}")))?
                .as_dict(),
            PdfObject::Dictionary(ref dictionary) => Some(dictionary),
            _ => None,
        };
        if dictionary.is_some_and(|dictionary| {
            dictionary
                .get("Subtype")
                .and_then(PdfObject::as_name)
                .is_some_and(|name| name.0 == "Widget")
        }) {
            return Err(invalid_lossless(format!(
                "cannot {action} a widget page without remapping its AcroForm field tree"
            )));
        }
    }
    Ok(())
}

fn ensure_import_graph_supported<R: Read + Seek>(
    reader: &mut PdfReader<R>,
    page: &LosslessPage,
) -> Result<(), PdfError> {
    let mut pending: Vec<_> = page
        .dictionary
        .0
        .iter()
        .filter(|(key, _)| key.0 != "Parent")
        .map(|(_, value)| value.clone())
        .collect();
    let mut visited = HashSet::new();
    while let Some(object) = pending.pop() {
        match object {
            PdfObject::Reference(number, generation) => {
                let id = (number, generation);
                if !visited.insert(id) {
                    continue;
                }
                if visited.len() > MAX_CLONED_OBJECTS_PER_PAGE {
                    return Err(invalid_lossless(
                        "imported page graph exceeds the supported object count",
                    ));
                }
                pending.push(
                    reader
                        .get_object(number, generation)
                        .map_err(|error| {
                            invalid_lossless(format!("inspect imported page graph: {error}"))
                        })?
                        .clone(),
                );
            }
            PdfObject::Array(array) => pending.extend(array.0),
            PdfObject::Dictionary(dictionary) => {
                reject_unsupported_import_dictionary(&dictionary)?;
                pending.extend(dictionary.0.into_values());
            }
            PdfObject::Stream(stream) => {
                reject_unsupported_import_dictionary(&stream.dict)?;
                pending.extend(stream.dict.0.into_values());
            }
            _ => {}
        }
    }
    Ok(())
}

fn reject_unsupported_import_dictionary(dictionary: &PdfDictionary) -> Result<(), PdfError> {
    if dictionary
        .get_type()
        .is_some_and(|kind| matches!(kind, "OCG" | "OCMD"))
    {
        return Err(invalid_lossless(
            "cannot import optional-content groups without remapping catalog /OCProperties",
        ));
    }
    let named_destination = dictionary
        .get("Dest")
        .is_some_and(|value| matches!(value, PdfObject::Name(_) | PdfObject::String(_)))
        || (dictionary
            .get("S")
            .and_then(PdfObject::as_name)
            .is_some_and(|name| name.0 == "GoTo")
            && dictionary
                .get("D")
                .is_some_and(|value| matches!(value, PdfObject::Name(_) | PdfObject::String(_))));
    if named_destination {
        return Err(invalid_lossless(
            "cannot import a named destination without remapping the destination name tree",
        ));
    }
    Ok(())
}

fn clone_page_object<R: Read + Seek>(
    reader: &mut PdfReader<R>,
    object: &PdfObject,
    source_page: (u32, u16),
    external: bool,
    update: &mut IncrementalUpdate<'_>,
    added: &mut Vec<((u32, u16), PdfObject)>,
    mapping: &mut HashMap<(u32, u16), (u32, u16)>,
    preserved_source_refs: &mut HashSet<(u32, u16)>,
    depth: usize,
) -> Result<PdfObject, PdfError> {
    if depth > MAX_OBJECT_GRAPH_DEPTH {
        return Err(invalid_lossless(
            "page object graph exceeds the supported nesting depth",
        ));
    }
    match object {
        PdfObject::Reference(number, generation) => {
            let source_id = (*number, *generation);
            if let Some(id) = mapping.get(&source_id) {
                return Ok(PdfObject::Reference(id.0, id.1));
            }
            let source_object = reader
                .get_object(*number, *generation)
                .map_err(|error| {
                    invalid_lossless(format!(
                        "clone page object {number} {generation} R: {error}"
                    ))
                })?
                .clone();
            let object_type = source_object.as_dict().and_then(PdfDictionary::get_type);
            if object_type.is_some_and(|kind| matches!(kind, "Page" | "Pages" | "Catalog")) {
                if external {
                    return Err(invalid_lossless(format!(
                        "imported page graph references foreign structural object {number} {generation} R"
                    )));
                }
                preserved_source_refs.insert(source_id);
                return Ok(object.clone());
            }
            if object_type.is_some_and(|kind| matches!(kind, "OCG" | "OCMD")) {
                if external {
                    return Err(invalid_lossless(
                        "cannot import optional-content groups without remapping catalog /OCProperties",
                    ));
                }
                preserved_source_refs.insert(source_id);
                return Ok(object.clone());
            }
            if mapping.len() >= MAX_CLONED_OBJECTS_PER_PAGE {
                return Err(invalid_lossless(
                    "page object graph exceeds the supported object count",
                ));
            }
            let id = update.allocate_id()?;
            mapping.insert(source_id, id);
            let cloned = clone_page_object(
                reader,
                &source_object,
                source_page,
                external,
                update,
                added,
                mapping,
                preserved_source_refs,
                depth + 1,
            )?;
            added.push((id, cloned));
            Ok(PdfObject::Reference(id.0, id.1))
        }
        PdfObject::Array(array) => Ok(PdfObject::Array(PdfArray(
            array
                .0
                .iter()
                .map(|value| {
                    clone_page_object(
                        reader,
                        value,
                        source_page,
                        external,
                        update,
                        added,
                        mapping,
                        preserved_source_refs,
                        depth + 1,
                    )
                })
                .collect::<Result<Vec<_>, _>>()?,
        ))),
        PdfObject::Dictionary(dictionary) => {
            let mut dictionary = dictionary.clone();
            let mut keys: Vec<_> = dictionary.0.keys().cloned().collect();
            keys.sort_by(|left, right| left.0.cmp(&right.0));
            for key in keys {
                let value = dictionary.0[&key].clone();
                let cloned = clone_page_object(
                    reader,
                    &value,
                    source_page,
                    external,
                    update,
                    added,
                    mapping,
                    preserved_source_refs,
                    depth + 1,
                )?;
                dictionary.0.insert(key, cloned);
            }
            Ok(PdfObject::Dictionary(dictionary))
        }
        PdfObject::Stream(stream) => {
            let mut stream = stream.clone();
            let mut keys: Vec<_> = stream.dict.0.keys().cloned().collect();
            keys.sort_by(|left, right| left.0.cmp(&right.0));
            for key in keys {
                let value = stream.dict.0[&key].clone();
                let cloned = clone_page_object(
                    reader,
                    &value,
                    source_page,
                    external,
                    update,
                    added,
                    mapping,
                    preserved_source_refs,
                    depth + 1,
                )?;
                stream.dict.0.insert(key, cloned);
            }
            Ok(PdfObject::Stream(stream))
        }
        _ => Ok(object.clone()),
    }
}

fn object_from_pending(added: &[((u32, u16), PdfObject)], id: (u32, u16)) -> PdfDictionary {
    added
        .iter()
        .find(|(candidate, _)| *candidate == id)
        .and_then(|(_, object)| object.as_dict())
        .cloned()
        .expect("new page dictionary was just queued")
}

fn reachable_from_catalog<R: Read + Seek>(
    reader: &mut PdfReader<R>,
    catalog: &PdfDictionary,
) -> Result<HashSet<(u32, u16)>, PdfError> {
    let mut pending: Vec<_> = catalog.0.values().cloned().collect();
    let mut reachable = HashSet::new();
    while let Some(object) = pending.pop() {
        match object {
            PdfObject::Reference(number, generation) => {
                let id = (number, generation);
                if !reachable.insert(id) {
                    continue;
                }
                if reachable.len() > MAX_CLONED_OBJECTS_PER_PAGE {
                    return Err(invalid_lossless(
                        "document object graph exceeds the supported object count",
                    ));
                }
                pending.push(
                    reader
                        .get_object(number, generation)
                        .map_err(|error| {
                            invalid_lossless(format!("walk document object graph: {error}"))
                        })?
                        .clone(),
                );
            }
            PdfObject::Array(array) => pending.extend(array.0),
            PdfObject::Dictionary(dictionary) => pending.extend(dictionary.0.into_values()),
            PdfObject::Stream(stream) => pending.extend(stream.dict.0.into_values()),
            _ => {}
        }
    }
    Ok(reachable)
}

fn prospective_source_references<R: Read + Seek>(
    reader: &mut PdfReader<R>,
    catalog: &PdfDictionary,
    page_root: (u32, u16),
    root_preserved_values: &[PdfObject],
    pages: &[LosslessPage],
    retained: &HashSet<usize>,
    preserved: &HashSet<(u32, u16)>,
) -> Result<HashSet<(u32, u16)>, PdfError> {
    let mut pending: Vec<_> = catalog
        .0
        .iter()
        .filter(|(key, _)| key.0 != "Pages")
        .map(|(_, value)| value.clone())
        .collect();
    pending.extend(root_preserved_values.iter().cloned());
    pending.extend(
        pages
            .iter()
            .enumerate()
            .filter(|(index, _)| retained.contains(index))
            .flat_map(|(_, page)| {
                page.dictionary
                    .0
                    .iter()
                    .filter(|(key, _)| key.0 != "Parent")
                    .map(|(_, value)| value.clone())
            }),
    );
    pending.extend(preserved.iter().map(|id| PdfObject::Reference(id.0, id.1)));

    let mut reachable = HashSet::from([page_root]);
    reachable.extend(
        pages
            .iter()
            .enumerate()
            .filter(|(index, _)| retained.contains(index))
            .map(|(_, page)| page.reference),
    );
    while let Some(object) = pending.pop() {
        match object {
            PdfObject::Reference(number, generation) => {
                let id = (number, generation);
                if !reachable.insert(id) || id == page_root {
                    continue;
                }
                if reachable.len() > MAX_CLONED_OBJECTS_PER_PAGE {
                    return Err(invalid_lossless(
                        "prospective document graph exceeds the supported object count",
                    ));
                }
                let object = reader
                    .get_object(number, generation)
                    .map_err(|error| {
                        invalid_lossless(format!("plan prospective object graph: {error}"))
                    })?
                    .clone();
                match object {
                    PdfObject::Dictionary(dictionary)
                        if dictionary
                            .get_type()
                            .is_some_and(|kind| matches!(kind, "Pages" | "Catalog")) => {}
                    PdfObject::Dictionary(dictionary) if dictionary.get_type() == Some("Page") => {
                        pending.extend(
                            dictionary
                                .0
                                .into_iter()
                                .filter(|(key, _)| key.0 != "Parent")
                                .map(|(_, value)| value),
                        );
                    }
                    object => pending.push(object),
                }
            }
            PdfObject::Array(array) => pending.extend(array.0),
            PdfObject::Dictionary(dictionary) => pending.extend(dictionary.0.into_values()),
            PdfObject::Stream(stream) => pending.extend(stream.dict.0.into_values()),
            _ => {}
        }
    }
    Ok(reachable)
}

fn ensure_catalog_does_not_reference_deleted_pages<R: Read + Seek>(
    reader: &mut PdfReader<R>,
    catalog: &PdfDictionary,
    page_root: (u32, u16),
    deleted: &HashSet<(u32, u16)>,
) -> Result<(), PdfError> {
    if deleted.is_empty() {
        return Ok(());
    }
    let mut pending: Vec<_> = catalog
        .0
        .iter()
        .filter(|(key, _)| key.0 != "Pages")
        .map(|(_, value)| value.clone())
        .collect();
    let mut visited = HashSet::new();
    while let Some(object) = pending.pop() {
        match object {
            PdfObject::Reference(number, generation) => {
                let id = (number, generation);
                if deleted.contains(&id) {
                    return Err(invalid_lossless(format!(
                        "cannot delete page {number} {generation} R because a catalog-level structure references it"
                    )));
                }
                if id == page_root || !visited.insert(id) {
                    continue;
                }
                pending.push(
                    reader
                        .get_object(number, generation)
                        .map_err(|error| {
                            invalid_lossless(format!("inspect catalog references: {error}"))
                        })?
                        .clone(),
                );
            }
            PdfObject::Array(array) => pending.extend(array.0),
            PdfObject::Dictionary(dictionary) => pending.extend(dictionary.0.into_values()),
            PdfObject::Stream(stream) => pending.extend(stream.dict.0.into_values()),
            _ => {}
        }
    }
    Ok(())
}

fn ensure_retained_pages_do_not_reference_deleted_pages<R: Read + Seek>(
    reader: &mut PdfReader<R>,
    pages: &[LosslessPage],
    retained: &HashSet<usize>,
    page_root: (u32, u16),
    deleted: &HashSet<(u32, u16)>,
) -> Result<(), PdfError> {
    if deleted.is_empty() {
        return Ok(());
    }
    let mut pending: Vec<_> = pages
        .iter()
        .enumerate()
        .filter(|(index, _)| retained.contains(index))
        .flat_map(|(_, page)| {
            page.dictionary
                .0
                .iter()
                .filter(|(key, _)| key.0 != "Parent")
                .map(|(_, value)| value.clone())
        })
        .collect();
    let mut visited = HashSet::new();
    while let Some(object) = pending.pop() {
        match object {
            PdfObject::Reference(number, generation) => {
                let id = (number, generation);
                if deleted.contains(&id) {
                    return Err(invalid_lossless(format!(
                        "cannot delete page {number} {generation} R because a retained page structure references it"
                    )));
                }
                if id == page_root || !visited.insert(id) {
                    continue;
                }
                pending.push(
                    reader
                        .get_object(number, generation)
                        .map_err(|error| {
                            invalid_lossless(format!("inspect retained page references: {error}"))
                        })?
                        .clone(),
                );
            }
            PdfObject::Array(array) => pending.extend(array.0),
            PdfObject::Dictionary(dictionary) => pending.extend(dictionary.0.into_values()),
            PdfObject::Stream(stream) => pending.extend(stream.dict.0.into_values()),
            _ => {}
        }
    }
    Ok(())
}

/// Reorder every page as one lossless incremental revision.
///
/// Unlike [`reorder_pdf_pages`], this API retains the original bytes and
/// indirect page identities. The order must be an exact permutation of all
/// source pages. Encrypted and signed inputs are rejected until their security
/// policies can be enforced safely.
pub fn reorder_pdf_pages_lossless<P: AsRef<Path>, Q: AsRef<Path>>(
    input_path: P,
    output_path: Q,
    page_order: &[usize],
) -> OperationResult<()> {
    let base = std::fs::read(input_path)?;
    let (updated, expected) = reorder_pdf_bytes_lossless(&base, page_order)?;
    validate_lossless_output(&base, &updated, &expected)?;

    let output_path = output_path.as_ref();
    let parent = output_path
        .parent()
        .filter(|path| !path.as_os_str().is_empty());
    let mut temporary = tempfile::NamedTempFile::new_in(parent.unwrap_or_else(|| Path::new(".")))?;
    temporary.write_all(&updated)?;
    temporary.flush()?;
    temporary.as_file().sync_all()?;

    validate_lossless_file(&base, temporary.path(), &expected)?;

    temporary
        .persist(output_path)
        .map_err(|error| OperationError::Io(error.error))?;
    Ok(())
}

fn reorder_pdf_bytes_lossless(
    base: &[u8],
    page_order: &[usize],
) -> Result<(Vec<u8>, LosslessValidation), PdfError> {
    let mut reader = PdfReader::new(Cursor::new(base))
        .map_err(|error| invalid_lossless(format!("parse source PDF: {error}")))?;
    if reader.is_encrypted() {
        return Err(PdfError::PermissionDenied(
            "lossless page reordering does not support encrypted PDFs".to_string(),
        ));
    }

    let catalog = reader
        .catalog()
        .map_err(|error| invalid_lossless(format!("read document catalog: {error}")))?
        .clone();
    ensure_modification_allowed(
        &mut reader,
        &catalog,
        IncrementalModification::PageTreeReorder,
    )?;

    let root_reference = catalog
        .get("Pages")
        .and_then(PdfObject::as_reference)
        .ok_or_else(|| invalid_lossless("catalog /Pages must be an indirect reference"))?;
    let root_dictionary = object_dictionary(&mut reader, root_reference, "page-tree root")?;

    let mut pages = Vec::new();
    let mut visited = HashSet::new();
    let inherited = HashMap::new();
    walk_page_tree(
        &mut reader,
        root_reference,
        None,
        inherited,
        &mut visited,
        &mut pages,
        0,
    )?;
    validate_exact_permutation(page_order, pages.len())?;

    let expected_order: Vec<_> = page_order
        .iter()
        .map(|index| pages[*index].reference)
        .collect();
    let root_inherited = inherited_values(&root_dictionary);
    let mut root_replacement = root_dictionary;
    root_replacement.insert(
        "Kids".to_string(),
        PdfObject::Array(PdfArray(
            expected_order
                .iter()
                .map(|(number, generation)| PdfObject::Reference(*number, *generation))
                .collect(),
        )),
    );
    root_replacement.insert("Count".to_string(), PdfObject::Integer(pages.len() as i64));

    let mut update = IncrementalUpdate::from_base(base)?;
    update.replace(root_reference, PdfObject::Dictionary(root_replacement))?;
    let mut validation_pages = Vec::with_capacity(pages.len());
    for page in pages {
        let mut dictionary = page.dictionary;
        let mut changed =
            dictionary.get("Parent").and_then(PdfObject::as_reference) != Some(root_reference);
        if changed {
            dictionary.insert(
                "Parent".to_string(),
                PdfObject::Reference(root_reference.0, root_reference.1),
            );
        }
        for (key, value) in &page.effective_inherited {
            if dictionary.contains_key(key) {
                continue;
            }
            if root_inherited.get(key) != Some(value) {
                dictionary.insert((*key).to_string(), value.clone());
                changed = true;
            }
        }
        validation_pages.push((page.reference, page.effective_inherited));
        if changed {
            update.replace(page.reference, PdfObject::Dictionary(dictionary))?;
        }
    }
    let updated = update.finish()?;
    let ordered_pages = page_order
        .iter()
        .map(|index| validation_pages[*index].clone())
        .collect();
    Ok((
        updated,
        LosslessValidation {
            root_reference,
            catalog,
            pages: ordered_pages,
        },
    ))
}

fn inherited_values(dictionary: &PdfDictionary) -> HashMap<&'static str, PdfObject> {
    INHERITABLE_PAGE_KEYS
        .into_iter()
        .filter_map(|key| dictionary.get(key).cloned().map(|value| (key, value)))
        .collect()
}

fn walk_page_tree<R: Read + std::io::Seek>(
    reader: &mut PdfReader<R>,
    node_reference: (u32, u16),
    expected_parent: Option<(u32, u16)>,
    inherited: HashMap<&'static str, PdfObject>,
    visited: &mut HashSet<(u32, u16)>,
    pages: &mut Vec<LosslessPage>,
    depth: usize,
) -> Result<usize, PdfError> {
    if depth > MAX_PAGE_TREE_DEPTH {
        return Err(invalid_lossless("page tree exceeds the supported depth"));
    }
    if !visited.insert(node_reference) {
        return Err(invalid_lossless(format!(
            "page tree contains a cycle or duplicate reference at {} {} R",
            node_reference.0, node_reference.1
        )));
    }
    let dictionary = object_dictionary(reader, node_reference, "page-tree node")?;
    if let Some(parent) = expected_parent {
        if dictionary.get("Parent").and_then(PdfObject::as_reference) != Some(parent) {
            return Err(invalid_lossless(format!(
                "page-tree node {} {} R has an inconsistent /Parent",
                node_reference.0, node_reference.1
            )));
        }
    } else if dictionary.contains_key("Parent") {
        return Err(invalid_lossless(
            "the root /Pages node must not have a /Parent",
        ));
    }

    match dictionary.get_type() {
        Some("Page") => {
            if pages.len() >= MAX_PAGE_COUNT {
                return Err(invalid_lossless(
                    "page tree exceeds the supported page count",
                ));
            }
            let mut effective_inherited = inherited;
            for key in INHERITABLE_PAGE_KEYS {
                if let Some(value) = dictionary.get(key) {
                    effective_inherited.insert(key, value.clone());
                }
            }
            if !effective_inherited.contains_key("MediaBox") {
                return Err(invalid_lossless(format!(
                    "page {} {} R has no effective /MediaBox",
                    node_reference.0, node_reference.1
                )));
            }
            pages.push(LosslessPage {
                reference: node_reference,
                dictionary,
                effective_inherited,
            });
            Ok(1)
        }
        Some("Pages") => {
            let mut child_inherited = inherited;
            for key in INHERITABLE_PAGE_KEYS {
                if let Some(value) = dictionary.get(key) {
                    child_inherited.insert(key, value.clone());
                }
            }
            let kids = resolve_reference_array(reader, dictionary.get("Kids"), "/Kids")?;
            let declared_count = dictionary
                .get("Count")
                .and_then(PdfObject::as_integer)
                .ok_or_else(|| invalid_lossless("every /Pages node must have an integer /Count"))?;
            if declared_count < 0 {
                return Err(invalid_lossless("a /Pages /Count cannot be negative"));
            }
            let mut actual_count = 0usize;
            for child in kids {
                actual_count = actual_count
                    .checked_add(walk_page_tree(
                        reader,
                        child,
                        Some(node_reference),
                        child_inherited.clone(),
                        visited,
                        pages,
                        depth + 1,
                    )?)
                    .ok_or_else(|| invalid_lossless("page count overflow"))?;
            }
            let declared_count = usize::try_from(declared_count)
                .map_err(|_| invalid_lossless("a /Pages /Count does not fit in memory"))?;
            if declared_count != actual_count {
                return Err(invalid_lossless(format!(
                    "/Pages node {} {} R declares /Count {} but contains {} pages",
                    node_reference.0, node_reference.1, declared_count, actual_count
                )));
            }
            Ok(actual_count)
        }
        other => Err(invalid_lossless(format!(
            "page-tree node {} {} R has unsupported /Type {:?}",
            node_reference.0, node_reference.1, other
        ))),
    }
}

fn object_dictionary<R: Read + std::io::Seek>(
    reader: &mut PdfReader<R>,
    reference: (u32, u16),
    role: &str,
) -> Result<PdfDictionary, PdfError> {
    reader
        .get_object(reference.0, reference.1)
        .map_err(|error| {
            invalid_lossless(format!(
                "resolve {role} {} {} R: {error}",
                reference.0, reference.1
            ))
        })?
        .as_dict()
        .cloned()
        .ok_or_else(|| {
            invalid_lossless(format!(
                "{role} {} {} R must be a dictionary",
                reference.0, reference.1
            ))
        })
}

fn resolve_reference_array<R: Read + std::io::Seek>(
    reader: &mut PdfReader<R>,
    value: Option<&PdfObject>,
    role: &str,
) -> Result<Vec<(u32, u16)>, PdfError> {
    let value = value.ok_or_else(|| invalid_lossless(format!("missing {role}")))?;
    let resolved = match value {
        PdfObject::Array(array) => PdfObject::Array(array.clone()),
        PdfObject::Reference(number, generation) => reader
            .get_object(*number, *generation)
            .map_err(|error| invalid_lossless(format!("resolve indirect {role}: {error}")))?
            .clone(),
        _ => return Err(invalid_lossless(format!("{role} must be an array"))),
    };
    let array = match resolved {
        PdfObject::Array(array) => array,
        _ => {
            return Err(invalid_lossless(format!(
                "indirect {role} must resolve to an array"
            )))
        }
    };
    array
        .0
        .iter()
        .map(|item| {
            item.as_reference()
                .ok_or_else(|| invalid_lossless(format!("{role} must contain only references")))
        })
        .collect()
}

fn validate_exact_permutation(page_order: &[usize], page_count: usize) -> Result<(), PdfError> {
    if page_order.len() != page_count {
        return Err(invalid_lossless(format!(
            "page order must contain exactly {page_count} entries, got {}",
            page_order.len()
        )));
    }
    let mut seen = vec![false; page_count];
    for &index in page_order {
        if index >= page_count {
            return Err(invalid_lossless(format!(
                "page index {index} is out of bounds for {page_count} pages"
            )));
        }
        if std::mem::replace(&mut seen[index], true) {
            return Err(invalid_lossless(format!(
                "page index {index} is duplicated"
            )));
        }
    }
    Ok(())
}

fn validate_lossless_output(
    base: &[u8],
    updated: &[u8],
    expected: &LosslessValidation,
) -> OperationResult<()> {
    if !updated.starts_with(base) {
        return Err(OperationError::ProcessingError(
            "incremental output does not preserve the source bytes as an exact prefix".to_string(),
        ));
    }
    let reader = PdfReader::new(Cursor::new(updated))
        .map_err(|error| OperationError::ParseError(format!("reopen output PDF: {error}")))?;
    validate_lossless_reader(reader, expected)
}

fn validate_lossless_file(
    base: &[u8],
    path: &Path,
    expected: &LosslessValidation,
) -> OperationResult<()> {
    let mut file = File::open(path)?;
    let mut chunk = [0u8; 64 * 1024];
    for expected_chunk in base.chunks(chunk.len()) {
        file.read_exact(&mut chunk[..expected_chunk.len()])?;
        if &chunk[..expected_chunk.len()] != expected_chunk {
            return Err(OperationError::ProcessingError(
                "temporary output does not preserve the source bytes as an exact prefix"
                    .to_string(),
            ));
        }
    }
    file.seek(SeekFrom::Start(0))?;
    let reader = PdfReader::new(file)
        .map_err(|error| OperationError::ParseError(format!("reopen temporary PDF: {error}")))?;
    validate_lossless_reader(reader, expected)
}

fn validate_lossless_reader<R: Read + Seek>(
    mut reader: PdfReader<R>,
    expected: &LosslessValidation,
) -> OperationResult<()> {
    let catalog = reader
        .catalog()
        .map_err(|error| OperationError::ParseError(format!("validate output catalog: {error}")))?;
    if catalog != &expected.catalog {
        return Err(OperationError::ProcessingError(
            "output catalog differs from the source catalog".to_string(),
        ));
    }

    let mut actual_pages = Vec::new();
    let mut visited = HashSet::new();
    walk_page_tree(
        &mut reader,
        expected.root_reference,
        None,
        HashMap::new(),
        &mut visited,
        &mut actual_pages,
        0,
    )
    .map_err(|error| OperationError::ParseError(format!("validate output page tree: {error}")))?;
    if actual_pages.len() != expected.pages.len() {
        return Err(OperationError::ProcessingError(format!(
            "output contains {} pages, expected {}",
            actual_pages.len(),
            expected.pages.len()
        )));
    }
    for (index, (actual, (expected_reference, expected_inherited))) in
        actual_pages.iter().zip(&expected.pages).enumerate()
    {
        if actual.reference != *expected_reference {
            return Err(OperationError::ProcessingError(format!(
                "output page {index} references {} {} R, expected {} {} R",
                actual.reference.0, actual.reference.1, expected_reference.0, expected_reference.1
            )));
        }
        if &actual.effective_inherited != expected_inherited {
            return Err(OperationError::ProcessingError(format!(
                "output page {index} does not preserve its effective inherited attributes"
            )));
        }
    }
    Ok(())
}

fn invalid_lossless(message: impl Into<String>) -> PdfError {
    PdfError::InvalidStructure(message.into())
}

/// Reverse all pages in a PDF
pub fn reverse_pdf_pages<P: AsRef<Path>, Q: AsRef<Path>>(
    input_path: P,
    output_path: Q,
) -> OperationResult<()> {
    let document = PdfReader::open_document(&input_path)
        .map_err(|e| OperationError::ParseError(e.to_string()))?;

    let page_count = document
        .page_count()
        .map_err(|e| OperationError::ParseError(e.to_string()))? as usize;

    // Create reverse order
    let page_order: Vec<usize> = (0..page_count).rev().collect();

    reorder_pdf_pages(input_path, output_path, page_order)
}

/// Move a page to a new position
pub fn move_pdf_page<P: AsRef<Path>, Q: AsRef<Path>>(
    input_path: P,
    output_path: Q,
    from_index: usize,
    to_index: usize,
) -> OperationResult<()> {
    let document = PdfReader::open_document(&input_path)
        .map_err(|e| OperationError::ParseError(e.to_string()))?;

    let page_count = document
        .page_count()
        .map_err(|e| OperationError::ParseError(e.to_string()))? as usize;

    if from_index >= page_count || to_index >= page_count {
        return Err(OperationError::InvalidPageRange(
            "Page index out of bounds".to_string(),
        ));
    }

    // Create new order
    let mut page_order: Vec<usize> = (0..page_count).collect();
    let page = page_order.remove(from_index);
    page_order.insert(to_index, page);

    reorder_pdf_pages(input_path, output_path, page_order)
}

/// Swap two pages in a PDF
pub fn swap_pdf_pages<P: AsRef<Path>, Q: AsRef<Path>>(
    input_path: P,
    output_path: Q,
    page1: usize,
    page2: usize,
) -> OperationResult<()> {
    let document = PdfReader::open_document(&input_path)
        .map_err(|e| OperationError::ParseError(e.to_string()))?;

    let page_count = document
        .page_count()
        .map_err(|e| OperationError::ParseError(e.to_string()))? as usize;

    if page1 >= page_count || page2 >= page_count {
        return Err(OperationError::InvalidPageRange(
            "Page index out of bounds".to_string(),
        ));
    }

    // Create new order with swapped pages
    let mut page_order: Vec<usize> = (0..page_count).collect();
    page_order.swap(page1, page2);

    reorder_pdf_pages(input_path, output_path, page_order)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_reorder_options_default() {
        let options = ReorderOptions::default();
        assert!(options.page_order.is_empty());
        assert!(options.preserve_metadata);
        assert!(!options.optimize);
    }

    #[test]
    fn test_reorder_options_custom() {
        let options = ReorderOptions {
            page_order: vec![2, 0, 1],
            preserve_metadata: false,
            optimize: true,
        };
        assert_eq!(options.page_order, vec![2, 0, 1]);
        assert!(!options.preserve_metadata);
        assert!(options.optimize);
    }

    #[test]
    fn test_validate_page_order_empty() {
        use crate::{Document, Page};
        use tempfile::NamedTempFile;

        // Create a test PDF
        let mut doc = Document::new();
        doc.add_page(Page::a4());

        let temp_file = NamedTempFile::new().unwrap();
        doc.save(temp_file.path()).unwrap();

        // Create reorderer with empty page order
        let pdf_doc = PdfReader::open_document(temp_file.path()).unwrap();
        let options = ReorderOptions {
            page_order: vec![],
            preserve_metadata: true,
            optimize: false,
        };

        let reorderer = PageReorderer::new(pdf_doc, options);
        let result = reorderer.reorder();

        assert!(result.is_err());
        if let Err(OperationError::InvalidPageRange(msg)) = result {
            assert!(msg.contains("empty"));
        } else {
            panic!("Expected InvalidPageRange error");
        }
    }

    #[test]
    fn test_validate_page_order_out_of_bounds() {
        use crate::{Document, Page};
        use tempfile::NamedTempFile;

        // Create a test PDF with 2 pages
        let mut doc = Document::new();
        doc.add_page(Page::a4());
        doc.add_page(Page::letter());

        let temp_file = NamedTempFile::new().unwrap();
        doc.save(temp_file.path()).unwrap();

        // Try to reorder with invalid index
        let pdf_doc = PdfReader::open_document(temp_file.path()).unwrap();
        let options = ReorderOptions {
            page_order: vec![0, 5], // Index 5 is out of bounds
            preserve_metadata: true,
            optimize: false,
        };

        let reorderer = PageReorderer::new(pdf_doc, options);
        let result = reorderer.reorder();

        assert!(result.is_err());
        if let Err(OperationError::InvalidPageRange(msg)) = result {
            assert!(msg.contains("out of bounds"));
        } else {
            panic!("Expected InvalidPageRange error");
        }
    }

    #[test]
    fn test_reorder_pages_simple() {
        use crate::{Document, Page};
        use tempfile::NamedTempFile;

        // Create a test PDF with 3 pages
        let mut doc = Document::new();
        let mut page1 = Page::a4();
        page1.graphics().begin_text();
        page1.graphics().set_text_position(100.0, 700.0);
        let _ = page1.graphics().show_text("Page 1");
        page1.graphics().end_text();
        doc.add_page(page1);

        let mut page2 = Page::a4();
        page2.graphics().begin_text();
        page2.graphics().set_text_position(100.0, 700.0);
        let _ = page2.graphics().show_text("Page 2");
        page2.graphics().end_text();
        doc.add_page(page2);

        let mut page3 = Page::a4();
        page3.graphics().begin_text();
        page3.graphics().set_text_position(100.0, 700.0);
        let _ = page3.graphics().show_text("Page 3");
        page3.graphics().end_text();
        doc.add_page(page3);

        let temp_file = NamedTempFile::new().unwrap();
        doc.save(temp_file.path()).unwrap();

        // Reorder pages: [2, 0, 1]
        let pdf_doc = PdfReader::open_document(temp_file.path()).unwrap();
        let options = ReorderOptions {
            page_order: vec![2, 0, 1],
            preserve_metadata: true,
            optimize: false,
        };

        let reorderer = PageReorderer::new(pdf_doc, options);
        let result = reorderer.reorder();

        assert!(result.is_ok());
        let reordered_doc = result.unwrap();
        assert_eq!(reordered_doc.page_count(), 3);
    }

    #[test]
    fn test_reverse_pages() {
        use crate::{Document, Page};
        use tempfile::NamedTempFile;

        // Create a test PDF with 4 pages
        let mut doc = Document::new();
        for i in 1..=4 {
            let mut page = Page::a4();
            page.graphics().begin_text();
            page.graphics().set_text_position(100.0, 700.0);
            let _ = page.graphics().show_text(&format!("Page {}", i));
            page.graphics().end_text();
            doc.add_page(page);
        }

        let temp_input = NamedTempFile::new().unwrap();
        doc.save(temp_input.path()).unwrap();

        let temp_output = NamedTempFile::new().unwrap();

        // Reverse the pages
        let result = reverse_pdf_pages(temp_input.path(), temp_output.path());
        assert!(result.is_ok());

        // Verify the output file exists
        assert!(temp_output.path().exists());
    }

    #[test]
    fn test_swap_pages() {
        use crate::{Document, Page};
        use tempfile::NamedTempFile;

        // Create a test PDF
        let mut doc = Document::new();
        doc.add_page(Page::a4());
        doc.add_page(Page::letter());
        doc.add_page(Page::legal());

        let temp_input = NamedTempFile::new().unwrap();
        doc.save(temp_input.path()).unwrap();

        let temp_output = NamedTempFile::new().unwrap();

        // Swap pages 0 and 2
        let result = swap_pdf_pages(temp_input.path(), temp_output.path(), 0, 2);
        assert!(result.is_ok());

        // Test invalid swap (out of bounds)
        let result = swap_pdf_pages(temp_input.path(), temp_output.path(), 0, 10);
        assert!(result.is_err());
    }

    #[test]
    fn test_move_page() {
        use crate::{Document, Page};
        use tempfile::NamedTempFile;

        // Create a test PDF
        let mut doc = Document::new();
        for _ in 0..5 {
            doc.add_page(Page::a4());
        }

        let temp_input = NamedTempFile::new().unwrap();
        doc.save(temp_input.path()).unwrap();

        let temp_output = NamedTempFile::new().unwrap();

        // Move page from index 0 to index 3
        let result = move_pdf_page(temp_input.path(), temp_output.path(), 0, 3);
        assert!(result.is_ok());

        // Test invalid move (out of bounds)
        let result = move_pdf_page(temp_input.path(), temp_output.path(), 10, 2);
        assert!(result.is_err());
    }

    #[test]
    fn test_duplicate_pages_in_order() {
        use crate::{Document, Page};
        use tempfile::NamedTempFile;

        // Create a test PDF with 2 pages
        let mut doc = Document::new();
        doc.add_page(Page::a4());
        doc.add_page(Page::letter());

        let temp_file = NamedTempFile::new().unwrap();
        doc.save(temp_file.path()).unwrap();

        // Create order with duplicates [0, 1, 0, 1]
        let pdf_doc = PdfReader::open_document(temp_file.path()).unwrap();
        let options = ReorderOptions {
            page_order: vec![0, 1, 0, 1],
            preserve_metadata: true,
            optimize: false,
        };

        let reorderer = PageReorderer::new(pdf_doc, options);
        let result = reorderer.reorder();

        assert!(result.is_ok());
        let reordered_doc = result.unwrap();
        assert_eq!(reordered_doc.page_count(), 4); // Should have 4 pages now
    }

    #[test]
    fn test_single_page_reorder() {
        use crate::{Document, Page};
        use tempfile::NamedTempFile;

        // Create a test PDF with 1 page
        let mut doc = Document::new();
        doc.add_page(Page::a4());

        let temp_file = NamedTempFile::new().unwrap();
        doc.save(temp_file.path()).unwrap();

        // Reorder single page
        let pdf_doc = PdfReader::open_document(temp_file.path()).unwrap();
        let options = ReorderOptions {
            page_order: vec![0],
            preserve_metadata: true,
            optimize: false,
        };

        let reorderer = PageReorderer::new(pdf_doc, options);
        let result = reorderer.reorder();

        assert!(result.is_ok());
        let reordered_doc = result.unwrap();
        assert_eq!(reordered_doc.page_count(), 1);
    }
}

#[cfg(test)]
#[path = "reorder_tests.rs"]
mod reorder_tests;
