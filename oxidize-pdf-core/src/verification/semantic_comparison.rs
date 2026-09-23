//! Bounded semantic comparison of complete PDF object graphs.

use crate::error::{PdfError, Result};
use crate::parser::objects::{PdfArray, PdfDictionary, PdfObject};
use crate::parser::{ParseOptions, PdfDocument, PdfReader};
use quick_xml::events::{BytesStart, BytesText, Event};
use quick_xml::{Reader as XmlReader, Writer as XmlWriter};
use sha2::{Digest, Sha256};
use std::cell::RefCell;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::io::Cursor;
use std::rc::Rc;

/// Resource limits applied independently to each compared document.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SemanticComparisonLimits {
    /// Maximum number of indirect objects reachable from one comparison root.
    pub max_objects: usize,
    /// Maximum nesting depth of direct and indirect objects.
    pub max_depth: usize,
    /// Maximum total decoded stream bytes visited in one document.
    pub max_decoded_stream_bytes: usize,
    /// Maximum total bytes materialized as canonical representations.
    pub max_canonical_bytes: usize,
    /// Maximum total extracted logical-text bytes.
    pub max_extracted_text_bytes: usize,
    /// Maximum number of latest in-use objects outside `/Root` and `/Info`.
    pub max_unreachable_objects: usize,
    /// Maximum number of physical incremental revisions.
    pub max_revisions: usize,
}

impl Default for SemanticComparisonLimits {
    fn default() -> Self {
        Self {
            max_objects: 100_000,
            max_depth: 256,
            max_decoded_stream_bytes: 256 * 1024 * 1024,
            max_canonical_bytes: 256 * 1024 * 1024,
            max_extracted_text_bytes: 64 * 1024 * 1024,
            max_unreachable_objects: 10_000,
            max_revisions: 1_024,
        }
    }
}

/// Options controlling semantic PDF comparison.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SemanticComparisonOptions {
    /// Hard limits for adversarial object graphs and decoded streams.
    pub limits: SemanticComparisonLimits,
}

/// Independent semantic domain associated with a difference.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SemanticDifferenceClass {
    /// Rendered page appearance changed.
    Visual,
    /// Extracted logical text changed.
    Textual,
    /// Reachable document structure or page-appearance operators changed.
    Structural,
    /// Document information metadata changed.
    Metadata,
    /// Signatures, permissions, encryption, or signed byte ranges changed.
    Security,
    /// Physical serialization differs without changing supported semantics.
    SerializationOnly,
}

/// One stable, machine-readable semantic difference.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SemanticPdfDifference {
    /// Stable logical path, independent of indirect object numbering.
    pub path: String,
    /// Semantic domain affected by the change.
    pub class: SemanticDifferenceClass,
    /// Concise description of the observed change.
    pub description: String,
}

/// How one indirect object changed in a physical PDF revision.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RevisionObjectChangeKind {
    /// The object became in-use for the first time.
    Added,
    /// A later revision supplied a new definition or generation.
    Replaced,
    /// The xref revision marked a previously known object free.
    Freed,
}

/// Revision-attributed indirect-object change.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RevisionObjectChange {
    /// Object number as physically serialized in this document.
    pub object_number: u32,
    /// Generation recorded by the revision's xref entry.
    pub generation: u16,
    /// State transition introduced in this revision.
    pub kind: RevisionObjectChangeKind,
}

/// One physical cross-reference revision, ordered oldest to newest.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PdfRevisionSummary {
    /// Zero-based chronological revision index.
    pub index: usize,
    /// Byte offset of this revision's xref table or xref stream.
    pub xref_offset: u64,
    /// Object changes attributed to this revision.
    pub object_changes: Vec<RevisionObjectChange>,
    /// Hash of the normalized document state at the end of this revision.
    pub semantic_fingerprint: [u8; 32],
}

/// Result of a bounded semantic comparison.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SemanticComparisonResult {
    /// True when every currently supported semantic domain is equivalent.
    pub semantically_equal: bool,
    /// Stable differences sorted by logical path.
    pub differences: Vec<SemanticPdfDifference>,
    /// Physical revision history of the left input.
    pub left_revisions: Vec<PdfRevisionSummary>,
    /// Physical revision history of the right input.
    pub right_revisions: Vec<PdfRevisionSummary>,
    /// Latest in-use indirect objects unreachable from `/Root` and `/Info`.
    pub left_unreachable_objects: Vec<(u32, u16)>,
    /// Latest in-use indirect objects unreachable from `/Root` and `/Info`.
    pub right_unreachable_objects: Vec<(u32, u16)>,
    /// Reachable indirect objects paired by their stable semantic traversal path.
    pub semantically_equivalent_objects: Vec<((u32, u16), (u32, u16))>,
}

/// Compare two PDFs after normalizing object numbers, dictionary order, stream
/// filters, stream lengths, and volatile creation/modification timestamps.
///
/// # Errors
///
/// Returns an error when either PDF is malformed, encrypted, exceeds a
/// configured limit, or contains a stream that cannot be decoded safely.
pub fn compare_pdfs_semantically(
    left: &[u8],
    right: &[u8],
    options: &SemanticComparisonOptions,
) -> Result<SemanticComparisonResult> {
    let left_bytes = left;
    let right_bytes = right;
    let left = SemanticSnapshot::build(left_bytes, options)?;
    let right = SemanticSnapshot::build(right_bytes, options)?;
    let mut differences = Vec::new();
    for (path, left_domain) in &left.domains {
        let right_domain = right.domains.get(path);
        if right_domain != Some(left_domain) {
            differences.push(domain_difference(path));
        }
    }
    for path in right.domains.keys() {
        if !left.domains.contains_key(path) {
            differences.push(domain_difference(path));
        }
    }
    if left.catalog != right.catalog && left.domains == right.domains {
        differences.push(SemanticPdfDifference {
            path: "/Catalog/ReachableGraph".to_string(),
            class: SemanticDifferenceClass::Structural,
            description: "reachable object graph differs outside a specialized domain".to_string(),
        });
    }
    if left.metadata != right.metadata {
        differences.push(SemanticPdfDifference {
            path: "/Trailer/Info".to_string(),
            class: SemanticDifferenceClass::Metadata,
            description: "document information dictionary differs".to_string(),
        });
    }
    if left.text != right.text {
        differences.push(SemanticPdfDifference {
            path: "/Pages/Text".to_string(),
            class: SemanticDifferenceClass::Textual,
            description: "extracted logical text differs".to_string(),
        });
    }
    if left.signatures != right.signatures {
        differences.push(SemanticPdfDifference {
            path: "/Signatures".to_string(),
            class: SemanticDifferenceClass::Security,
            description: "signature byte ranges or post-signing changes differ".to_string(),
        });
    }
    if left.unreachable_semantics != right.unreachable_semantics {
        differences.push(SemanticPdfDifference {
            path: "/UnreachableObjects".to_string(),
            class: SemanticDifferenceClass::Structural,
            description: "unreachable indirect-object content differs".to_string(),
        });
    }
    let mut left_revision_semantics: Vec<_> = left
        .revisions
        .iter()
        .map(|revision| revision.semantic_fingerprint)
        .collect();
    let mut right_revision_semantics: Vec<_> = right
        .revisions
        .iter()
        .map(|revision| revision.semantic_fingerprint)
        .collect();
    left_revision_semantics.dedup();
    right_revision_semantics.dedup();
    if left_revision_semantics != right_revision_semantics {
        differences.push(SemanticPdfDifference {
            path: "/Revisions".to_string(),
            class: SemanticDifferenceClass::Structural,
            description: "semantic document history differs across incremental revisions"
                .to_string(),
        });
    }
    let semantically_equivalent_objects =
        pair_equivalent_objects(&left.object_fingerprints, &right.object_fingerprints);
    if differences.is_empty() && left_bytes != right_bytes {
        differences.push(SemanticPdfDifference {
            path: "/Serialization".to_string(),
            class: SemanticDifferenceClass::SerializationOnly,
            description: "physical serialization differs without a supported semantic change"
                .to_string(),
        });
    }
    differences.sort_by(|left, right| left.path.cmp(&right.path));
    let semantically_equal = differences
        .iter()
        .all(|difference| difference.class == SemanticDifferenceClass::SerializationOnly);
    Ok(SemanticComparisonResult {
        semantically_equal,
        differences,
        left_revisions: left.revisions,
        right_revisions: right.revisions,
        left_unreachable_objects: left.unreachable_objects,
        right_unreachable_objects: right.unreachable_objects,
        semantically_equivalent_objects,
    })
}

fn pair_equivalent_objects(
    left: &[((u32, u16), [u8; 32])],
    right: &[((u32, u16), [u8; 32])],
) -> Vec<((u32, u16), (u32, u16))> {
    let mut left_by_fingerprint: BTreeMap<[u8; 32], Vec<(u32, u16)>> = BTreeMap::new();
    let mut right_by_fingerprint: BTreeMap<[u8; 32], Vec<(u32, u16)>> = BTreeMap::new();
    for (reference, fingerprint) in left {
        left_by_fingerprint
            .entry(*fingerprint)
            .or_default()
            .push(*reference);
    }
    for (reference, fingerprint) in right {
        right_by_fingerprint
            .entry(*fingerprint)
            .or_default()
            .push(*reference);
    }
    let mut pairs = Vec::new();
    for (fingerprint, mut left_references) in left_by_fingerprint {
        let Some(mut right_references) = right_by_fingerprint.remove(&fingerprint) else {
            continue;
        };
        left_references.sort_unstable();
        right_references.sort_unstable();
        pairs.extend(left_references.into_iter().zip(right_references));
    }
    pairs.sort_unstable();
    pairs
}

fn domain_difference(path: &str) -> SemanticPdfDifference {
    let (class, description) = if path.starts_with("/Pages/") {
        if path.ends_with("/Annotations") {
            (
                SemanticDifferenceClass::Structural,
                "page annotations differ",
            )
        } else {
            (SemanticDifferenceClass::Visual, "page appearance differs")
        }
    } else {
        match path {
            "/Catalog/OCProperties" => (
                SemanticDifferenceClass::Visual,
                "optional content configuration differs",
            ),
            "/Catalog/Metadata" => (SemanticDifferenceClass::Metadata, "XMP metadata differs"),
            "/Catalog/Perms" | "/Catalog/DSS" => (
                SemanticDifferenceClass::Security,
                "signature permissions or validation data differs",
            ),
            "/Catalog/AcroForm" => (
                SemanticDifferenceClass::Structural,
                "interactive forms differ",
            ),
            "/Catalog/Names" => (
                SemanticDifferenceClass::Structural,
                "names or attachments differ",
            ),
            "/Catalog/Outlines" => (SemanticDifferenceClass::Structural, "outlines differ"),
            "/Catalog/StructTreeRoot" | "/Catalog/MarkInfo" => {
                (SemanticDifferenceClass::Structural, "tag structure differs")
            }
            _ => (
                SemanticDifferenceClass::Structural,
                "reachable structure differs",
            ),
        }
    };
    SemanticPdfDifference {
        path: path.to_string(),
        class,
        description: description.to_string(),
    }
}

#[derive(Clone)]
struct ComparisonBudget {
    limits: SemanticComparisonLimits,
    usage: Rc<RefCell<ComparisonUsage>>,
}

#[derive(Default)]
struct ComparisonUsage {
    decoded_stream_bytes: usize,
    canonical_bytes: usize,
    indirect_objects: HashSet<(u32, u16)>,
}

impl ComparisonBudget {
    fn new(limits: &SemanticComparisonLimits) -> Self {
        Self {
            limits: limits.clone(),
            usage: Rc::new(RefCell::new(ComparisonUsage::default())),
        }
    }

    fn charge_decoded(&self, bytes: usize) -> Result<()> {
        let mut usage = self.usage.borrow_mut();
        usage.decoded_stream_bytes = usage
            .decoded_stream_bytes
            .checked_add(bytes)
            .ok_or_else(|| limit("decoded stream bytes", self.limits.max_decoded_stream_bytes))?;
        if usage.decoded_stream_bytes > self.limits.max_decoded_stream_bytes {
            return Err(limit(
                "decoded stream bytes",
                self.limits.max_decoded_stream_bytes,
            ));
        }
        Ok(())
    }

    fn charge_canonical(&self, bytes: usize) -> Result<()> {
        let mut usage = self.usage.borrow_mut();
        usage.canonical_bytes = usage
            .canonical_bytes
            .checked_add(bytes)
            .ok_or_else(|| limit("canonical bytes", self.limits.max_canonical_bytes))?;
        if usage.canonical_bytes > self.limits.max_canonical_bytes {
            return Err(limit("canonical bytes", self.limits.max_canonical_bytes));
        }
        Ok(())
    }

    fn charge_reference(&self, reference: (u32, u16)) -> Result<()> {
        let mut usage = self.usage.borrow_mut();
        usage.indirect_objects.insert(reference);
        if usage.indirect_objects.len() > self.limits.max_objects {
            return Err(limit("indirect objects", self.limits.max_objects));
        }
        Ok(())
    }

    fn ensure_local_output(&self, current: usize, additional: usize) -> Result<()> {
        let local = current
            .checked_add(additional)
            .ok_or_else(|| limit("canonical bytes", self.limits.max_canonical_bytes))?;
        let committed = self.usage.borrow().canonical_bytes;
        if committed
            .checked_add(local)
            .is_none_or(|total| total > self.limits.max_canonical_bytes)
        {
            return Err(limit("canonical bytes", self.limits.max_canonical_bytes));
        }
        Ok(())
    }
}

struct SemanticSnapshot {
    catalog: Vec<u8>,
    object_fingerprints: Vec<((u32, u16), [u8; 32])>,
    domains: BTreeMap<String, Vec<u8>>,
    text: Vec<String>,
    signatures: Vec<Vec<u8>>,
    unreachable_semantics: Vec<Vec<u8>>,
    metadata: Option<Vec<u8>>,
    revisions: Vec<PdfRevisionSummary>,
    unreachable_objects: Vec<(u32, u16)>,
}

impl SemanticSnapshot {
    fn build(bytes: &[u8], options: &SemanticComparisonOptions) -> Result<Self> {
        let budget = ComparisonBudget::new(&options.limits);
        let text_reader = PdfReader::new(Cursor::new(bytes))
            .map_err(|error| invalid(format!("parse PDF for text extraction: {error}")))?;
        if text_reader.is_encrypted() {
            return Err(PdfError::PermissionDenied(
                "semantic comparison requires decrypted PDF input".to_string(),
            ));
        }
        let text_document = PdfDocument::new(text_reader);
        let page_count = text_document
            .page_count()
            .map_err(|error| invalid(format!("count pages: {error}")))?;
        let pages: Vec<_> = (0..page_count)
            .map(|index| {
                text_document
                    .get_page(index)
                    .map_err(|error| invalid(format!("read page {index}: {error}")))
            })
            .collect::<Result<_>>()?;
        let extracted = text_document
            .extract_text_with_options(crate::text::ExtractionOptions {
                max_extracted_bytes: Some(options.limits.max_extracted_text_bytes),
                ..crate::text::ExtractionOptions::default()
            })
            .map_err(|error| invalid(format!("extract logical text: {error}")))?;
        let mut extracted_text_bytes = 0usize;
        let mut text = Vec::with_capacity(extracted.len());
        for page in extracted {
            if page.truncated {
                return Err(limit(
                    "extracted text bytes",
                    options.limits.max_extracted_text_bytes,
                ));
            }
            extracted_text_bytes = extracted_text_bytes
                .checked_add(page.text.len())
                .ok_or_else(|| {
                    limit(
                        "extracted text bytes",
                        options.limits.max_extracted_text_bytes,
                    )
                })?;
            if extracted_text_bytes > options.limits.max_extracted_text_bytes {
                return Err(limit(
                    "extracted text bytes",
                    options.limits.max_extracted_text_bytes,
                ));
            }
            text.push(page.text);
        }
        let mut reader = PdfReader::new(Cursor::new(bytes))
            .map_err(|error| invalid(format!("parse PDF: {error}")))?;
        if reader.is_encrypted() {
            return Err(PdfError::PermissionDenied(
                "semantic comparison requires decrypted PDF input".to_string(),
            ));
        }
        let mut signatures: Vec<_> = reader
            .signatures()
            .map_err(|error| invalid(format!("read signatures: {error}")))?
            .into_iter()
            .map(|signature| {
                let mut value = format!(
                    "{:?}|{}|{}|{:?}|{:?}|{:?}|{:?}|{:?}",
                    signature.name,
                    signature.byte_range,
                    crate::signatures::has_incremental_update(bytes, &signature.byte_range),
                    signature.filter,
                    signature.sub_filter,
                    signature.reason,
                    signature.location,
                    signature.signing_time,
                )
                .into_bytes();
                value.extend_from_slice(&signature.contents);
                value
            })
            .collect();
        signatures.sort();
        let mut revisions = summarize_revisions(&reader);
        if revisions.len() > options.limits.max_revisions {
            return Err(limit("physical revisions", options.limits.max_revisions));
        }
        attach_revision_fingerprints(bytes, &mut revisions, &budget)?;
        let root = reader
            .trailer()
            .root()
            .map_err(|error| invalid(format!("read trailer root: {error}")))?;
        let catalog_dictionary = reader
            .catalog()
            .map_err(|error| invalid(format!("read catalog: {error}")))?
            .clone();
        let mut domains = BTreeMap::new();
        for key in [
            "OCProperties",
            "Metadata",
            "AcroForm",
            "Perms",
            "DSS",
            "Names",
            "Outlines",
            "StructTreeRoot",
            "MarkInfo",
        ] {
            if let Some(value) = catalog_dictionary.get(key) {
                let (canonical, _, _) = Canonicalizer::new(&mut reader, &options.limits, &budget)
                    .canonicalize(value, &[])?;
                domains.insert(format!("/Catalog/{key}"), canonical);
            }
        }
        let pages_root = catalog_dictionary
            .get("Pages")
            .ok_or_else(|| invalid("catalog is missing /Pages"))?;
        let (page_tree, _, _) = Canonicalizer::new(&mut reader, &options.limits, &budget)
            .canonicalize(pages_root, &["Kids"])?;
        domains.insert("/Pages/Tree".to_string(), page_tree);
        for (index, parsed_page) in pages.iter().enumerate() {
            let (number, generation) = parsed_page.obj_ref;
            let page = reader
                .get_object(number, generation)
                .map_err(|error| invalid(format!("resolve page {index}: {error}")))?
                .clone();
            let mut appearance_page = page
                .as_dict()
                .cloned()
                .ok_or_else(|| invalid(format!("page {index} is not a dictionary")))?;
            if !appearance_page.contains_key("Resources") {
                if let Some(resources) = &parsed_page.inherited_resources {
                    appearance_page.insert(
                        "Resources".to_string(),
                        PdfObject::Dictionary(resources.clone()),
                    );
                }
            }
            if !appearance_page.contains_key("MediaBox") {
                appearance_page.insert(
                    "MediaBox".to_string(),
                    PdfObject::Array(PdfArray(
                        parsed_page
                            .media_box
                            .iter()
                            .copied()
                            .map(PdfObject::Real)
                            .collect(),
                    )),
                );
            }
            if !appearance_page.contains_key("CropBox") {
                if let Some(crop_box) = parsed_page.crop_box {
                    appearance_page.insert(
                        "CropBox".to_string(),
                        PdfObject::Array(PdfArray(
                            crop_box.iter().copied().map(PdfObject::Real).collect(),
                        )),
                    );
                }
            }
            if !appearance_page.contains_key("Rotate") && parsed_page.rotation != 0 {
                appearance_page.insert(
                    "Rotate".to_string(),
                    PdfObject::Integer(i64::from(parsed_page.rotation)),
                );
            }
            let (appearance, _, _) = Canonicalizer::new(&mut reader, &options.limits, &budget)
                .canonicalize(
                    &PdfObject::Dictionary(appearance_page),
                    &["Parent", "Annots"],
                )?;
            domains.insert(format!("/Pages/{index}/Appearance"), appearance);
            if let Some(annotations) = page.as_dict().and_then(|page| page.get("Annots")) {
                let (annotations, _, _) = Canonicalizer::new(&mut reader, &options.limits, &budget)
                    .canonicalize(annotations, &["P"])?;
                budget.charge_canonical(annotations.len())?;
                domains.insert(
                    format!("/Pages/{index}/AnnotationAppearance"),
                    annotations.clone(),
                );
                domains.insert(format!("/Pages/{index}/Annotations"), annotations);
            }
        }
        let domain_keys: HashSet<_> = [
            "Pages",
            "OCProperties",
            "Metadata",
            "AcroForm",
            "Perms",
            "DSS",
            "Names",
            "Outlines",
            "StructTreeRoot",
            "MarkInfo",
        ]
        .into_iter()
        .collect();
        let residual_catalog = PdfDictionary(
            catalog_dictionary
                .0
                .iter()
                .filter(|(key, _)| !domain_keys.contains(key.0.as_str()))
                .map(|(key, value)| (key.clone(), value.clone()))
                .collect(),
        );
        let (residual, _, _) = Canonicalizer::new(&mut reader, &options.limits, &budget)
            .canonicalize(&PdfObject::Dictionary(residual_catalog), &[])?;
        domains.insert("/Catalog".to_string(), residual);
        let (catalog, mut reachable, _) = Canonicalizer::new(&mut reader, &options.limits, &budget)
            .canonicalize(&PdfObject::Reference(root.0, root.1), &[])?;
        let metadata = reader.trailer().info().map(|reference| {
            Canonicalizer::new(&mut reader, &options.limits, &budget).canonicalize(
                &PdfObject::Reference(reference.0, reference.1),
                &[
                    "CreationDate",
                    "ModDate",
                    "oxidize-pdf-build",
                    "oxidize-pdf-features",
                ],
            )
        });
        let metadata = metadata.transpose()?;
        if let Some((_, metadata_references, _)) = &metadata {
            reachable.extend(metadata_references);
        }
        let object_references = reader.object_references();
        let mut unreachable_objects: Vec<_> = object_references
            .iter()
            .copied()
            .into_iter()
            .filter(|reference| !reachable.contains(reference))
            .collect();
        unreachable_objects.sort_unstable();
        if unreachable_objects.len() > options.limits.max_unreachable_objects {
            return Err(limit(
                "unreachable objects",
                options.limits.max_unreachable_objects,
            ));
        }
        let unreachable_set: HashSet<_> = unreachable_objects.iter().copied().collect();
        let mut unreachable_semantics = Vec::with_capacity(unreachable_objects.len());
        let mut object_fingerprints = Vec::with_capacity(object_references.len());
        for (number, generation) in object_references {
            let (canonical, _, _) = Canonicalizer::new(&mut reader, &options.limits, &budget)
                .canonicalize(&PdfObject::Reference(number, generation), &[])?;
            let fingerprint: [u8; 32] = Sha256::digest(&canonical).into();
            object_fingerprints.push(((number, generation), fingerprint));
            if unreachable_set.contains(&(number, generation)) {
                unreachable_semantics.push(canonical);
            }
        }
        unreachable_semantics.sort();
        Ok(Self {
            catalog,
            object_fingerprints,
            domains,
            text,
            signatures,
            unreachable_semantics,
            metadata: metadata.map(|(bytes, _, _)| bytes),
            revisions,
            unreachable_objects,
        })
    }
}

fn summarize_revisions<R: std::io::Read + std::io::Seek>(
    reader: &PdfReader<R>,
) -> Vec<PdfRevisionSummary> {
    let mut known = HashSet::new();
    reader
        .xref_revisions()
        .into_iter()
        .enumerate()
        .map(|(index, revision)| {
            let mut object_changes = Vec::new();
            for entry in revision.entries {
                if entry.object_number == 0 {
                    continue;
                }
                let kind = if entry.in_use {
                    if known.insert(entry.object_number) {
                        RevisionObjectChangeKind::Added
                    } else {
                        RevisionObjectChangeKind::Replaced
                    }
                } else if known.remove(&entry.object_number) {
                    RevisionObjectChangeKind::Freed
                } else {
                    continue;
                };
                object_changes.push(RevisionObjectChange {
                    object_number: entry.object_number,
                    generation: entry.generation,
                    kind,
                });
            }
            PdfRevisionSummary {
                index,
                xref_offset: revision.xref_offset,
                object_changes,
                semantic_fingerprint: [0; 32],
            }
        })
        .collect()
}

fn attach_revision_fingerprints(
    bytes: &[u8],
    revisions: &mut [PdfRevisionSummary],
    budget: &ComparisonBudget,
) -> Result<()> {
    for revision in revisions {
        let xref_offset = usize::try_from(revision.xref_offset)
            .ok()
            .and_then(|offset| bytes.get(offset..).map(|tail| (offset, tail)))
            .ok_or_else(|| invalid("revision xref offset exceeds input size"))?;
        let eof_relative = find_bytes(xref_offset.1, b"%%EOF")
            .ok_or_else(|| invalid("incremental revision is missing %%EOF"))?;
        let revision_end = xref_offset
            .0
            .checked_add(eof_relative)
            .and_then(|offset| offset.checked_add(b"%%EOF".len()))
            .ok_or_else(|| invalid("incremental revision boundary overflows input size"))?;
        let prefix = bytes
            .get(..revision_end)
            .ok_or_else(|| invalid("incremental revision boundary exceeds input size"))?;
        let mut reader = PdfReader::new(Cursor::new(prefix))
            .map_err(|error| invalid(format!("parse incremental revision: {error}")))?;
        let root = reader
            .trailer()
            .root()
            .map_err(|error| invalid(format!("read incremental revision root: {error}")))?;
        let (catalog, mut reachable, _) =
            Canonicalizer::new(&mut reader, &budget.limits, budget)
                .canonicalize(&PdfObject::Reference(root.0, root.1), &[])?;
        let metadata = reader.trailer().info().map(|reference| {
            Canonicalizer::new(&mut reader, &budget.limits, budget).canonicalize(
                &PdfObject::Reference(reference.0, reference.1),
                &[
                    "CreationDate",
                    "ModDate",
                    "oxidize-pdf-build",
                    "oxidize-pdf-features",
                ],
            )
        });
        let metadata = metadata.transpose()?;
        let mut digest = Sha256::new();
        digest.update((catalog.len() as u64).to_be_bytes());
        digest.update(catalog);
        if let Some((metadata, metadata_references, _)) = metadata {
            reachable.extend(metadata_references);
            digest.update((metadata.len() as u64).to_be_bytes());
            digest.update(metadata);
        }
        let mut unreachable = Vec::new();
        for reference in reader
            .object_references()
            .into_iter()
            .filter(|reference| !reachable.contains(reference))
        {
            let (canonical, _, _) = Canonicalizer::new(&mut reader, &budget.limits, budget)
                .canonicalize(&PdfObject::Reference(reference.0, reference.1), &[])?;
            unreachable.push(canonical);
        }
        if unreachable.len() > budget.limits.max_unreachable_objects {
            return Err(limit(
                "unreachable objects",
                budget.limits.max_unreachable_objects,
            ));
        }
        unreachable.sort();
        for canonical in unreachable {
            digest.update((canonical.len() as u64).to_be_bytes());
            digest.update(canonical);
        }
        revision.semantic_fingerprint = digest.finalize().into();
    }
    Ok(())
}

struct Canonicalizer<'a, R: std::io::Read + std::io::Seek> {
    reader: &'a mut PdfReader<R>,
    limits: &'a SemanticComparisonLimits,
    budget: ComparisonBudget,
    references: HashMap<(u32, u16), usize>,
}

impl<'a, R: std::io::Read + std::io::Seek> Canonicalizer<'a, R> {
    fn new(
        reader: &'a mut PdfReader<R>,
        limits: &'a SemanticComparisonLimits,
        budget: &ComparisonBudget,
    ) -> Self {
        Self {
            reader,
            limits,
            budget: budget.clone(),
            references: HashMap::new(),
        }
    }

    fn canonicalize(
        mut self,
        root: &PdfObject,
        ignored_keys: &[&str],
    ) -> Result<(Vec<u8>, HashSet<(u32, u16)>, Vec<(u32, u16)>)> {
        let mut output = Vec::new();
        self.write_object(root, 0, ignored_keys, &mut output)?;
        self.budget.charge_canonical(output.len())?;
        let mut ordered_references: Vec<_> = self
            .references
            .iter()
            .map(|(reference, identifier)| (*identifier, *reference))
            .collect();
        ordered_references.sort_unstable_by_key(|(identifier, _)| *identifier);
        Ok((
            output,
            self.references.keys().copied().collect(),
            ordered_references
                .into_iter()
                .map(|(_, reference)| reference)
                .collect(),
        ))
    }

    fn write_object(
        &mut self,
        object: &PdfObject,
        depth: usize,
        ignored_keys: &[&str],
        output: &mut Vec<u8>,
    ) -> Result<()> {
        if depth > self.limits.max_depth {
            return Err(limit("object nesting depth", self.limits.max_depth));
        }
        match object {
            PdfObject::Null => output.extend_from_slice(b"null"),
            PdfObject::Boolean(value) => {
                output.extend_from_slice(if *value { b"true" } else { b"false" })
            }
            PdfObject::Integer(value) => output.extend_from_slice(format!("i{value};").as_bytes()),
            PdfObject::Real(value) => {
                let normalized = if *value == 0.0 { 0.0 } else { *value };
                output.extend_from_slice(format!("r{:016x};", normalized.to_bits()).as_bytes());
            }
            PdfObject::String(value) => self.write_bytes(b's', &value.0, output)?,
            PdfObject::Name(value) => self.write_bytes(b'n', value.0.as_bytes(), output)?,
            PdfObject::Array(array) => {
                output.push(b'[');
                for value in &array.0 {
                    self.write_object(value, depth + 1, ignored_keys, output)?;
                }
                output.push(b']');
            }
            PdfObject::Dictionary(dictionary) => {
                self.write_dictionary(dictionary, depth, ignored_keys, output)?;
            }
            PdfObject::Stream(stream) => {
                output.extend_from_slice(b"stream");
                self.write_dictionary_filtered(
                    &stream.dict,
                    depth,
                    ignored_keys,
                    &["Length", "Filter", "DecodeParms"],
                    output,
                )?;
                let decoded = stream
                    .decode(&ParseOptions::default())
                    .map_err(|error| invalid(format!("decode stream: {error}")))?;
                self.budget.charge_decoded(decoded.len())?;
                let normalized = if stream.dict.get_type() == Some("Metadata")
                    && stream
                        .dict
                        .get("Subtype")
                        .and_then(PdfObject::as_name)
                        .is_some_and(|name| name.0 == "XML")
                {
                    normalize_xmp_noise(decoded)
                } else {
                    decoded
                };
                self.write_bytes(b'd', &normalized, output)?;
            }
            PdfObject::Reference(number, generation) => {
                let reference = (*number, *generation);
                if let Some(identifier) = self.references.get(&reference) {
                    output.extend_from_slice(format!("@{identifier};").as_bytes());
                    return Ok(());
                }
                if self.references.len() >= self.limits.max_objects {
                    return Err(limit("reachable indirect objects", self.limits.max_objects));
                }
                self.budget.charge_reference(reference)?;
                let identifier = self.references.len();
                self.references.insert(reference, identifier);
                output.extend_from_slice(format!("&{identifier}=").as_bytes());
                let resolved = self
                    .reader
                    .get_object(*number, *generation)
                    .map_err(|error| invalid(format!("resolve indirect object: {error}")))?
                    .clone();
                self.write_object(&resolved, depth + 1, ignored_keys, output)?;
            }
        }
        Ok(())
    }

    fn write_dictionary(
        &mut self,
        dictionary: &PdfDictionary,
        depth: usize,
        ignored_keys: &[&str],
        output: &mut Vec<u8>,
    ) -> Result<()> {
        self.write_dictionary_filtered(dictionary, depth, ignored_keys, &[], output)
    }

    fn write_dictionary_filtered(
        &mut self,
        dictionary: &PdfDictionary,
        depth: usize,
        ignored_keys: &[&str],
        additional_ignored: &[&str],
        output: &mut Vec<u8>,
    ) -> Result<()> {
        output.push(b'<');
        let mut entries: Vec<_> = dictionary
            .0
            .iter()
            .filter(|(key, _)| {
                !ignored_keys.contains(&key.0.as_str())
                    && !additional_ignored.contains(&key.0.as_str())
            })
            .collect();
        entries.sort_by(|left, right| left.0 .0.cmp(&right.0 .0));
        for (key, value) in entries {
            self.write_bytes(b'k', key.0.as_bytes(), output)?;
            self.write_object(value, depth + 1, ignored_keys, output)?;
        }
        output.push(b'>');
        Ok(())
    }

    fn write_bytes(&self, prefix: u8, bytes: &[u8], output: &mut Vec<u8>) -> Result<()> {
        let length_digits = bytes.len().max(1).ilog10() as usize + 1;
        self.budget.ensure_local_output(
            output.len(),
            bytes.len().saturating_add(length_digits).saturating_add(3),
        )?;
        output.push(prefix);
        output.extend_from_slice(format!("{}:", bytes.len()).as_bytes());
        output.extend_from_slice(bytes);
        output.push(b';');
        Ok(())
    }
}

fn normalize_xmp_noise(bytes: Vec<u8>) -> Vec<u8> {
    let mut reader = XmlReader::from_reader(bytes.as_slice());
    reader.config_mut().trim_text(false);
    let mut writer = XmlWriter::new(Vec::with_capacity(bytes.len()));
    let mut buffer = Vec::new();
    let mut date_elements = Vec::new();
    let mut date_text_written = Vec::new();

    loop {
        let event = match reader.read_event_into(&mut buffer) {
            Ok(Event::Eof) => break,
            Ok(event) => event,
            Err(_) => return bytes,
        };
        let write_result = match event {
            Event::Start(start) => {
                let is_date = is_xmp_date_name(start.local_name().as_ref());
                date_elements.push(is_date);
                date_text_written.push(false);
                writer.write_event(Event::Start(normalize_xmp_attributes(&start)))
            }
            Event::Empty(start) => {
                writer.write_event(Event::Empty(normalize_xmp_attributes(&start)))
            }
            Event::Text(_) if date_elements.last() == Some(&true) => {
                if date_text_written.last() == Some(&false) {
                    if let Some(written) = date_text_written.last_mut() {
                        *written = true;
                    }
                    writer.write_event(Event::Text(BytesText::new("normalized")))
                } else {
                    Ok(())
                }
            }
            Event::CData(_) if date_elements.last() == Some(&true) => {
                if date_text_written.last() == Some(&false) {
                    if let Some(written) = date_text_written.last_mut() {
                        *written = true;
                    }
                    writer.write_event(Event::Text(BytesText::new("normalized")))
                } else {
                    Ok(())
                }
            }
            Event::End(end) => {
                date_elements.pop();
                date_text_written.pop();
                writer.write_event(Event::End(end.into_owned()))
            }
            event => writer.write_event(event.into_owned()),
        };
        if write_result.is_err() {
            return bytes;
        }
        buffer.clear();
    }
    writer.into_inner()
}

fn normalize_xmp_attributes(start: &BytesStart<'_>) -> BytesStart<'static> {
    let name = String::from_utf8_lossy(start.name().as_ref()).into_owned();
    let mut normalized = BytesStart::new(name);
    for attribute in start.attributes().with_checks(false).flatten() {
        let key = String::from_utf8_lossy(attribute.key.as_ref()).into_owned();
        let value = if is_xmp_date_name(attribute.key.local_name().as_ref()) {
            "normalized".to_string()
        } else {
            String::from_utf8_lossy(attribute.value.as_ref()).into_owned()
        };
        normalized.push_attribute((key.as_str(), value.as_str()));
    }
    normalized.into_owned()
}

fn is_xmp_date_name(name: &[u8]) -> bool {
    matches!(name, b"CreateDate" | b"ModifyDate" | b"MetadataDate")
}

fn find_bytes(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
}

fn limit(resource: &str, maximum: usize) -> PdfError {
    invalid(format!(
        "semantic comparison {resource} limit exceeded ({maximum})"
    ))
}

fn invalid(message: impl Into<String>) -> PdfError {
    PdfError::InvalidStructure(message.into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::writer::{IncrementalOcrLayerEditor, OcrLayerFragment, OcrLayerPage};
    use crate::{Document, Font, Page};

    fn document(compress: bool) -> Vec<u8> {
        let mut document = Document::new();
        document.set_compress(compress);
        let mut page = Page::a4();
        page.text()
            .set_font(Font::Helvetica, 12.0)
            .at(20.0, 800.0)
            .write("semantic content")
            .unwrap();
        document.add_page(page);
        document.to_bytes().unwrap()
    }

    fn numbered_pdf(numbers: [u32; 4], definition_order: [usize; 4]) -> Vec<u8> {
        let [catalog, pages, page, contents] = numbers;
        let bodies = [
            format!("<< /Type /Catalog /Pages {pages} 0 R >>"),
            format!("<< /Type /Pages /Kids [{page} 0 R] /Count 1 >>"),
            format!("<< /Type /Page /Parent {pages} 0 R /MediaBox [0 0 300 300] /Contents {contents} 0 R >>"),
            "<< /Length 16 >>\nstream\nBT (same) Tj ET\nendstream".to_string(),
        ];
        let mut output = b"%PDF-1.7\n".to_vec();
        let size = numbers.iter().copied().max().unwrap() + 1;
        let mut offsets = vec![None; size as usize];
        for index in definition_order {
            let number = numbers[index];
            offsets[number as usize] = Some(output.len());
            output.extend_from_slice(
                format!("{number} 0 obj\n{}\nendobj\n", bodies[index]).as_bytes(),
            );
        }
        let xref = output.len();
        output.extend_from_slice(format!("xref\n0 {size}\n0000000000 65535 f \n").as_bytes());
        for offset in offsets.into_iter().skip(1) {
            match offset {
                Some(offset) => {
                    output.extend_from_slice(format!("{offset:010} 00000 n \n").as_bytes())
                }
                None => output.extend_from_slice(b"0000000000 00000 f \n"),
            }
        }
        output.extend_from_slice(
            format!("trailer\n<< /Size {size} /Root {catalog} 0 R >>\nstartxref\n{xref}\n%%EOF\n")
                .as_bytes(),
        );
        output
    }

    fn raw_pdf(objects: &[(u32, &str)], root: u32, trailer_extra: &str) -> Vec<u8> {
        let mut output = b"%PDF-1.7\n".to_vec();
        let size = objects.iter().map(|(number, _)| *number).max().unwrap() + 1;
        let mut offsets = vec![None; size as usize];
        for (number, body) in objects {
            offsets[*number as usize] = Some(output.len());
            output.extend_from_slice(format!("{number} 0 obj\n{body}\nendobj\n").as_bytes());
        }
        let xref = output.len();
        output.extend_from_slice(format!("xref\n0 {size}\n0000000000 65535 f \n").as_bytes());
        for offset in offsets.into_iter().skip(1) {
            if let Some(offset) = offset {
                output.extend_from_slice(format!("{offset:010} 00000 n \n").as_bytes());
            } else {
                output.extend_from_slice(b"0000000000 00000 f \n");
            }
        }
        output.extend_from_slice(
            format!(
                "trailer\n<< /Size {size} /Root {root} 0 R {trailer_extra} >>\nstartxref\n{xref}\n%%EOF\n"
            )
            .as_bytes(),
        );
        output
    }

    fn minimal_objects(catalog_suffix: &str, page_suffix: &str) -> Vec<(u32, String)> {
        vec![
            (
                1,
                format!("<< /Type /Catalog /Pages 2 0 R {catalog_suffix} >>"),
            ),
            (2, "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_string()),
            (
                3,
                format!(
                    "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 300 300] /Contents 4 0 R {page_suffix} >>"
                ),
            ),
            (
                4,
                "<< /Length 16 >>\nstream\nBT (base) Tj ET\nendstream".to_string(),
            ),
        ]
    }

    fn append_replacement(mut base: Vec<u8>, object_number: u32, body: &str) -> Vec<u8> {
        let marker = b"startxref\n";
        let start = base
            .windows(marker.len())
            .rposition(|window| window == marker)
            .unwrap()
            + marker.len();
        let end = base[start..]
            .iter()
            .position(|byte| *byte == b'\n')
            .unwrap()
            + start;
        let previous_xref = std::str::from_utf8(&base[start..end])
            .unwrap()
            .parse::<usize>()
            .unwrap();
        let object_offset = base.len();
        base.extend_from_slice(format!("{object_number} 0 obj\n{body}\nendobj\n").as_bytes());
        let xref = base.len();
        base.extend_from_slice(
            format!(
                "xref\n{object_number} 1\n{object_offset:010} 00000 n \ntrailer\n<< /Size 5 /Root 1 0 R /Prev {previous_xref} >>\nstartxref\n{xref}\n%%EOF\n"
            )
            .as_bytes(),
        );
        base
    }

    fn append_free(mut base: Vec<u8>, object_number: u32, generation: u16) -> Vec<u8> {
        let marker = b"startxref\n";
        let start = base
            .windows(marker.len())
            .rposition(|window| window == marker)
            .unwrap()
            + marker.len();
        let end = base[start..]
            .iter()
            .position(|byte| *byte == b'\n')
            .unwrap()
            + start;
        let previous_xref = std::str::from_utf8(&base[start..end])
            .unwrap()
            .parse::<usize>()
            .unwrap();
        let xref = base.len();
        base.extend_from_slice(
            format!(
                "xref\n{object_number} 1\n0000000000 {generation:05} f \ntrailer\n<< /Size 6 /Root 1 0 R /Prev {previous_xref} >>\nstartxref\n{xref}\n%%EOF\n"
            )
            .as_bytes(),
        );
        base
    }

    #[test]
    fn stream_compression_is_serialization_only() {
        let left_bytes = document(false);
        let right_bytes = document(true);
        let result = compare_pdfs_semantically(
            &left_bytes,
            &right_bytes,
            &SemanticComparisonOptions::default(),
        )
        .unwrap();
        assert!(result.semantically_equal, "{:?}", result.differences);
        assert!(result
            .differences
            .iter()
            .any(|difference| { difference.class == SemanticDifferenceClass::SerializationOnly }));
    }

    #[test]
    fn object_numbers_and_definition_order_are_serialization_only() {
        let left = numbered_pdf([1, 2, 3, 4], [0, 1, 2, 3]);
        let right = numbered_pdf([11, 7, 19, 5], [3, 1, 0, 2]);
        let result =
            compare_pdfs_semantically(&left, &right, &SemanticComparisonOptions::default())
                .unwrap();
        assert!(result.semantically_equal, "{:?}", result.differences);
    }

    #[test]
    fn visible_content_change_is_visual_and_textual() {
        let left = document(false);
        let mut right = left.clone();
        let position = right
            .windows(b"semantic content".len())
            .position(|window| window == b"semantic content")
            .unwrap();
        right[position..position + b"semantic content".len()].copy_from_slice(b"different text!!");
        let result =
            compare_pdfs_semantically(&left, &right, &SemanticComparisonOptions::default())
                .unwrap();
        assert!(!result.semantically_equal);
        assert!(result.differences.iter().any(|difference| {
            difference.path == "/Pages/0/Appearance"
                && difference.class == SemanticDifferenceClass::Visual
        }));
        assert!(result.differences.iter().any(|difference| {
            difference.path == "/Pages/Text" && difference.class == SemanticDifferenceClass::Textual
        }));
    }

    #[test]
    fn content_outside_page_bounds_is_still_reported() {
        let left_content = "BT 10000 10000 Td (hidden-a) Tj ET\n";
        let right_content = "BT 10000 10000 Td (hidden-b) Tj ET\n";
        let mut left_objects = minimal_objects("", "");
        left_objects[3].1 = format!(
            "<< /Length {} >>\nstream\n{left_content}endstream",
            left_content.len()
        );
        let mut right_objects = minimal_objects("", "");
        right_objects[3].1 = format!(
            "<< /Length {} >>\nstream\n{right_content}endstream",
            right_content.len()
        );
        let left_refs: Vec<_> = left_objects
            .iter()
            .map(|(number, body)| (*number, body.as_str()))
            .collect();
        let right_refs: Vec<_> = right_objects
            .iter()
            .map(|(number, body)| (*number, body.as_str()))
            .collect();
        let result = compare_pdfs_semantically(
            &raw_pdf(&left_refs, 1, ""),
            &raw_pdf(&right_refs, 1, ""),
            &SemanticComparisonOptions::default(),
        )
        .unwrap();
        assert!(result.differences.iter().any(|difference| {
            difference.path == "/Pages/0/Appearance"
                && difference.class == SemanticDifferenceClass::Visual
        }));
    }

    #[test]
    fn reachable_object_budget_fails_closed() {
        let options = SemanticComparisonOptions {
            limits: SemanticComparisonLimits {
                max_objects: 1,
                ..SemanticComparisonLimits::default()
            },
        };
        let pdf = document(false);
        let error = compare_pdfs_semantically(&pdf, &pdf, &options).unwrap_err();
        assert!(
            matches!(error, PdfError::InvalidStructure(message) if message.contains("limit exceeded"))
        );
    }

    #[test]
    fn incremental_changes_are_attributed_to_their_revision() {
        let base = document(false);
        let updated = IncrementalOcrLayerEditor::new(&base)
            .apply(&[OcrLayerPage {
                page_index: 0,
                language: "en".to_string(),
                fragments: vec![OcrLayerFragment {
                    text: "revision".to_string(),
                    region: [20.0, 700.0, 60.0, 12.0],
                    confidence: 0.9,
                    reading_order: 0,
                }],
            }])
            .unwrap();
        let result = compare_pdfs_semantically(
            &base,
            &updated.pdf_bytes,
            &SemanticComparisonOptions::default(),
        )
        .unwrap();
        assert_eq!(result.left_revisions.len(), 1);
        assert_eq!(result.right_revisions.len(), 2);
        let latest = &result.right_revisions[1];
        assert!(latest
            .object_changes
            .iter()
            .any(|change| change.kind == RevisionObjectChangeKind::Replaced));
        assert!(latest
            .object_changes
            .iter()
            .any(|change| change.kind == RevisionObjectChangeKind::Added));

        let mut objects = minimal_objects("", "");
        objects.push((5, "(temporary)".to_string()));
        let refs: Vec<_> = objects
            .iter()
            .map(|(number, body)| (*number, body.as_str()))
            .collect();
        let freed = append_free(raw_pdf(&refs, 1, ""), 5, 1);
        let freed_result =
            compare_pdfs_semantically(&freed, &freed, &SemanticComparisonOptions::default())
                .unwrap();
        assert!(freed_result.right_revisions[1]
            .object_changes
            .iter()
            .any(|change| change.kind == RevisionObjectChangeKind::Freed));
    }

    #[test]
    fn semantically_redundant_incremental_revision_is_serialization_only() {
        let objects = minimal_objects("", "");
        let refs: Vec<_> = objects
            .iter()
            .map(|(number, body)| (*number, body.as_str()))
            .collect();
        let base = raw_pdf(&refs, 1, "");
        let updated = append_replacement(base.clone(), 4, &objects[3].1);

        let result =
            compare_pdfs_semantically(&base, &updated, &SemanticComparisonOptions::default())
                .unwrap();

        assert!(result.semantically_equal, "{:?}", result.differences);
        assert_eq!(result.right_revisions.len(), 2);
        assert!(result
            .differences
            .iter()
            .all(|difference| difference.class == SemanticDifferenceClass::SerializationOnly));
    }

    #[test]
    fn signatures_and_post_signing_updates_are_security_differences() {
        let signed = include_bytes!("../../tests/fixtures/signatures/signed_rsa.pdf");
        let incrementally_updated =
            include_bytes!("../../tests/fixtures/signatures/signed_rsa_incremental.pdf");
        let result = compare_pdfs_semantically(
            signed,
            incrementally_updated,
            &SemanticComparisonOptions::default(),
        )
        .unwrap();

        assert!(result.differences.iter().any(|difference| {
            difference.path == "/Signatures"
                && difference.class == SemanticDifferenceClass::Security
        }));
    }

    #[test]
    fn optional_content_annotations_metadata_and_attachments_have_stable_paths() {
        let base_objects = minimal_objects("", "");
        let base_refs: Vec<_> = base_objects
            .iter()
            .map(|(number, body)| (*number, body.as_str()))
            .collect();
        let base = raw_pdf(&base_refs, 1, "");

        let optional_objects = minimal_objects("/OCProperties << /D << /Name (Layer) >> >>", "");
        let optional_refs: Vec<_> = optional_objects
            .iter()
            .map(|(number, body)| (*number, body.as_str()))
            .collect();
        let optional = raw_pdf(&optional_refs, 1, "");
        let optional_result =
            compare_pdfs_semantically(&base, &optional, &SemanticComparisonOptions::default())
                .unwrap();
        assert!(optional_result.differences.iter().any(|difference| {
            difference.path == "/Catalog/OCProperties"
                && difference.class == SemanticDifferenceClass::Visual
        }));

        let mut annotation_objects = minimal_objects("", "/Annots [5 0 R]");
        annotation_objects.push((
            5,
            "<< /Type /Annot /Subtype /Text /Rect [10 10 20 20] /Contents (note) >>".to_string(),
        ));
        let annotation_refs: Vec<_> = annotation_objects
            .iter()
            .map(|(number, body)| (*number, body.as_str()))
            .collect();
        let annotation = raw_pdf(&annotation_refs, 1, "");
        let annotation_result =
            compare_pdfs_semantically(&base, &annotation, &SemanticComparisonOptions::default())
                .unwrap();
        assert!(annotation_result.differences.iter().any(|difference| {
            difference.path == "/Pages/0/Annotations"
                && difference.class == SemanticDifferenceClass::Structural
        }));
        assert!(annotation_result.differences.iter().any(|difference| {
            difference.path == "/Pages/0/AnnotationAppearance"
                && difference.class == SemanticDifferenceClass::Visual
        }));

        let mut metadata_objects = minimal_objects("", "");
        metadata_objects.push((5, "<< /Title (changed) >>".to_string()));
        let metadata_refs: Vec<_> = metadata_objects
            .iter()
            .map(|(number, body)| (*number, body.as_str()))
            .collect();
        let metadata = raw_pdf(&metadata_refs, 1, "/Info 5 0 R");
        let metadata_result =
            compare_pdfs_semantically(&base, &metadata, &SemanticComparisonOptions::default())
                .unwrap();
        assert!(metadata_result.differences.iter().any(|difference| {
            difference.path == "/Trailer/Info"
                && difference.class == SemanticDifferenceClass::Metadata
        }));

        let mut attachment_objects = minimal_objects(
            "/Names << /EmbeddedFiles << /Names [(data.txt) 5 0 R] >> >>",
            "",
        );
        attachment_objects.push((
            5,
            "<< /Type /Filespec /F (data.txt) /EF << /F 6 0 R >> >>".to_string(),
        ));
        attachment_objects.push((
            6,
            "<< /Type /EmbeddedFile /Length 5 >>\nstream\ndata\nendstream".to_string(),
        ));
        let attachment_refs: Vec<_> = attachment_objects
            .iter()
            .map(|(number, body)| (*number, body.as_str()))
            .collect();
        let attachment = raw_pdf(&attachment_refs, 1, "");
        let attachment_result =
            compare_pdfs_semantically(&base, &attachment, &SemanticComparisonOptions::default())
                .unwrap();
        assert!(attachment_result.differences.iter().any(|difference| {
            difference.path == "/Catalog/Names"
                && difference.class == SemanticDifferenceClass::Structural
        }));
    }

    #[test]
    fn unreachable_and_historical_changes_are_reported() {
        let mut left_objects = minimal_objects("", "");
        left_objects.push((5, "(orphan-left)".to_string()));
        let left_refs: Vec<_> = left_objects
            .iter()
            .map(|(number, body)| (*number, body.as_str()))
            .collect();
        let left = raw_pdf(&left_refs, 1, "");
        let mut right_objects = minimal_objects("", "");
        right_objects.push((5, "(orphan-right)".to_string()));
        let right_refs: Vec<_> = right_objects
            .iter()
            .map(|(number, body)| (*number, body.as_str()))
            .collect();
        let right = raw_pdf(&right_refs, 1, "");
        let unreachable_result =
            compare_pdfs_semantically(&left, &right, &SemanticComparisonOptions::default())
                .unwrap();
        assert!(unreachable_result
            .differences
            .iter()
            .any(|difference| { difference.path == "/UnreachableObjects" }));

        let left_base_objects = minimal_objects("", "");
        let mut right_base_objects = minimal_objects("", "");
        right_base_objects[3].1 =
            "<< /Length 16 >>\nstream\nBT (past) Tj ET\nendstream".to_string();
        let left_base_refs: Vec<_> = left_base_objects
            .iter()
            .map(|(number, body)| (*number, body.as_str()))
            .collect();
        let right_base_refs: Vec<_> = right_base_objects
            .iter()
            .map(|(number, body)| (*number, body.as_str()))
            .collect();
        let final_body = "<< /Length 17 >>\nstream\nBT (final) Tj ET\nendstream";
        let historical_left = append_replacement(raw_pdf(&left_base_refs, 1, ""), 4, final_body);
        let historical_right = append_replacement(raw_pdf(&right_base_refs, 1, ""), 4, final_body);
        let history_result = compare_pdfs_semantically(
            &historical_left,
            &historical_right,
            &SemanticComparisonOptions::default(),
        )
        .unwrap();
        assert!(history_result
            .differences
            .iter()
            .any(|difference| { difference.path == "/Revisions" }));
        assert_eq!(history_result.left_revisions.len(), 2);
        assert_eq!(history_result.right_revisions.len(), 2);

        let historical_orphan_left = append_free(left, 5, 1);
        let historical_orphan_right = append_free(right, 5, 1);
        let orphan_history_result = compare_pdfs_semantically(
            &historical_orphan_left,
            &historical_orphan_right,
            &SemanticComparisonOptions::default(),
        )
        .unwrap();
        assert!(orphan_history_result.left_unreachable_objects.is_empty());
        assert!(orphan_history_result.right_unreachable_objects.is_empty());
        assert!(orphan_history_result
            .differences
            .iter()
            .any(|difference| difference.path == "/Revisions"));
    }

    #[test]
    fn cyclic_graphs_terminate_and_respect_object_limits() {
        let mut objects = minimal_objects("/Custom 5 0 R", "");
        objects.push((5, "<< /Next 6 0 R >>".to_string()));
        objects.push((6, "<< /Next 5 0 R >>".to_string()));
        let refs: Vec<_> = objects
            .iter()
            .map(|(number, body)| (*number, body.as_str()))
            .collect();
        let pdf = raw_pdf(&refs, 1, "");
        let equal =
            compare_pdfs_semantically(&pdf, &pdf, &SemanticComparisonOptions::default()).unwrap();
        assert!(equal.semantically_equal);

        let options = SemanticComparisonOptions {
            limits: SemanticComparisonLimits {
                max_objects: 1,
                ..SemanticComparisonLimits::default()
            },
        };
        assert!(compare_pdfs_semantically(&pdf, &pdf, &options).is_err());
    }

    #[test]
    fn forms_outlines_and_tags_are_independent_structural_domains() {
        let base_objects = minimal_objects("", "");
        let base_refs: Vec<_> = base_objects
            .iter()
            .map(|(number, body)| (*number, body.as_str()))
            .collect();
        let base = raw_pdf(&base_refs, 1, "");

        for (suffix, expected_path) in [
            ("/AcroForm << /Fields [] >>", "/Catalog/AcroForm"),
            ("/Outlines << /Count 0 >>", "/Catalog/Outlines"),
            (
                "/MarkInfo << /Marked true >> /StructTreeRoot << /Type /StructTreeRoot /K [] >>",
                "/Catalog/StructTreeRoot",
            ),
        ] {
            let objects = minimal_objects(suffix, "");
            let refs: Vec<_> = objects
                .iter()
                .map(|(number, body)| (*number, body.as_str()))
                .collect();
            let changed = raw_pdf(&refs, 1, "");
            let result =
                compare_pdfs_semantically(&base, &changed, &SemanticComparisonOptions::default())
                    .unwrap();
            assert!(result.differences.iter().any(|difference| {
                difference.path == expected_path
                    && difference.class == SemanticDifferenceClass::Structural
            }));
        }
    }

    #[test]
    fn every_configured_budget_fails_closed() {
        let objects = minimal_objects("", "");
        let refs: Vec<_> = objects
            .iter()
            .map(|(number, body)| (*number, body.as_str()))
            .collect();
        let pdf = raw_pdf(&refs, 1, "");
        for limits in [
            SemanticComparisonLimits {
                max_depth: 1,
                ..SemanticComparisonLimits::default()
            },
            SemanticComparisonLimits {
                max_decoded_stream_bytes: 1,
                ..SemanticComparisonLimits::default()
            },
            SemanticComparisonLimits {
                max_canonical_bytes: 1,
                ..SemanticComparisonLimits::default()
            },
            SemanticComparisonLimits {
                max_extracted_text_bytes: 1,
                ..SemanticComparisonLimits::default()
            },
        ] {
            let error =
                compare_pdfs_semantically(&pdf, &pdf, &SemanticComparisonOptions { limits })
                    .unwrap_err();
            assert!(
                matches!(error, PdfError::InvalidStructure(message) if message.contains("limit exceeded"))
            );
        }

        let updated = append_replacement(
            pdf,
            4,
            "<< /Length 17 >>\nstream\nBT (final) Tj ET\nendstream",
        );
        let revision_error = compare_pdfs_semantically(
            &updated,
            &updated,
            &SemanticComparisonOptions {
                limits: SemanticComparisonLimits {
                    max_revisions: 1,
                    ..SemanticComparisonLimits::default()
                },
            },
        )
        .unwrap_err();
        assert!(
            matches!(revision_error, PdfError::InvalidStructure(message) if message.contains("physical revisions limit exceeded"))
        );

        let mut orphan_objects = minimal_objects("", "");
        orphan_objects.push((5, "(orphan)".to_string()));
        let orphan_refs: Vec<_> = orphan_objects
            .iter()
            .map(|(number, body)| (*number, body.as_str()))
            .collect();
        let orphan_pdf = raw_pdf(&orphan_refs, 1, "");
        let unreachable_error = compare_pdfs_semantically(
            &orphan_pdf,
            &orphan_pdf,
            &SemanticComparisonOptions {
                limits: SemanticComparisonLimits {
                    max_unreachable_objects: 0,
                    ..SemanticComparisonLimits::default()
                },
            },
        )
        .unwrap_err();
        assert!(
            matches!(unreachable_error, PdfError::InvalidStructure(message) if message.contains("unreachable objects limit exceeded"))
        );
    }

    #[test]
    fn equivalent_objects_are_reported_even_when_the_documents_differ() {
        let left = document(false);
        let mut right = left.clone();
        let position = right
            .windows(b"semantic content".len())
            .position(|window| window == b"semantic content")
            .unwrap();
        right[position..position + b"semantic content".len()].copy_from_slice(b"different text!!");

        let result =
            compare_pdfs_semantically(&left, &right, &SemanticComparisonOptions::default())
                .unwrap();

        assert!(!result.semantically_equal);
        assert!(!result.semantically_equivalent_objects.is_empty());
    }

    #[test]
    fn all_xmp_timestamp_forms_are_normalized() {
        let left = br#"<rdf:Description xmp:CreateDate="2020" xmp:ModifyDate="2021"><xmp:MetadataDate><![CDATA[2022]]></xmp:MetadataDate><xmp:CreateDate>2023</xmp:CreateDate></rdf:Description>"#;
        let right = br#"<rdf:Description xmp:CreateDate="2030" xmp:ModifyDate="2031"><xmp:MetadataDate>2032</xmp:MetadataDate><xmp:CreateDate>2033</xmp:CreateDate></rdf:Description>"#;

        assert_eq!(
            normalize_xmp_noise(left.to_vec()),
            normalize_xmp_noise(right.to_vec())
        );
    }

    #[test]
    fn permitted_info_and_xmp_timestamp_noise_is_ignored() {
        let mut left_objects = minimal_objects("/Metadata 5 0 R", "");
        left_objects.push((
            5,
            "<< /Type /Metadata /Subtype /XML /Length 44 >>\nstream\n<xmp:CreateDate>2020-01-01</xmp:CreateDate>\nendstream".to_string(),
        ));
        left_objects.push((
            6,
            "<< /CreationDate (D:20200101000000Z) /ModDate (D:20200101000000Z) >>".to_string(),
        ));
        let left_refs: Vec<_> = left_objects
            .iter()
            .map(|(number, body)| (*number, body.as_str()))
            .collect();
        let left = raw_pdf(&left_refs, 1, "/Info 6 0 R");

        let mut right_objects = minimal_objects("/Metadata 5 0 R", "");
        right_objects.push((
            5,
            "<< /Type /Metadata /Subtype /XML /Length 44 >>\nstream\n<xmp:CreateDate>2030-12-31</xmp:CreateDate>\nendstream".to_string(),
        ));
        right_objects.push((
            6,
            "<< /CreationDate (D:20301231000000Z) /ModDate (D:20301231000000Z) >>".to_string(),
        ));
        let right_refs: Vec<_> = right_objects
            .iter()
            .map(|(number, body)| (*number, body.as_str()))
            .collect();
        let right = raw_pdf(&right_refs, 1, "/Info 6 0 R");

        let result =
            compare_pdfs_semantically(&left, &right, &SemanticComparisonOptions::default())
                .unwrap();
        assert!(result.semantically_equal, "{:?}", result.differences);
        assert!(result
            .differences
            .iter()
            .all(|difference| difference.class == SemanticDifferenceClass::SerializationOnly));
    }
}
