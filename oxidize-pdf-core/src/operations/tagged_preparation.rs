//! Explicit lossless preparation of author-supplied tagged-PDF metadata.
use crate::error::{PdfError, Result};
use crate::parser::{
    objects::{PdfObject, PdfString},
    PdfReader,
};
use crate::verification::tagged_pdf::{
    validate_tagged_pdf, TaggedPdfFindingCode, TaggedPdfObjectRef, TaggedPdfValidationReport,
};
use crate::writer::IncrementalUpdate;
use std::{
    collections::{BTreeMap, BTreeSet},
    io::Cursor,
};

/// Values supplied by the author for missing accessibility metadata.
/// Existing nonempty values cannot be overwritten through this preparation API.
#[derive(Debug, Clone, Default)]
pub struct TaggedPdfMetadata {
    /// Explicit catalog language; never inferred from page text.
    pub language: Option<String>,
    /// Explicit descriptions keyed by the structure-element identities from preflight.
    pub alternate_text: BTreeMap<TaggedPdfObjectRef, String>,
}

/// Validated prepared bytes; the original source is an exact prefix.
/// Preparation appends derived-index recovery and explicit metadata revisions.
/// It never writes a source or destination file.
#[derive(Debug)]
pub struct PreparedTaggedPdf {
    /// Complete prepared document, ready for the preserving split API.
    pub pdf_bytes: Vec<u8>,
    /// Full tagged-PDF validation of the prepared bytes (not PDF/UA certification).
    pub validation: TaggedPdfValidationReport,
}

/// Inspect all required metadata after safe recovery of derived indexes/keys.
/// The source is unchanged. Findings identify the original structure elements.
///
/// # Errors
/// Rejects encrypted/certified inputs, ambiguous ownership, malformed graphs or
/// indexes, unsupported structures and resource-limit violations. Recoverable
/// missing metadata is returned together in the report, never fabricated.
pub fn preflight_tagged_pdf(base: &[u8]) -> Result<TaggedPdfValidationReport> {
    let recovered = super::reorder::recover_tagged_structure(base)?;
    validate_tagged_pdf(&recovered, &Default::default())
}

fn invalid(message: impl Into<String>) -> PdfError {
    PdfError::InvalidStructure(message.into())
}

fn text_string(value: &str) -> Result<PdfObject> {
    if value.trim().is_empty() {
        return Err(invalid("explicit metadata values must not be blank"));
    }
    let mut bytes = vec![0xfe, 0xff];
    for unit in value.encode_utf16() {
        bytes.extend_from_slice(&unit.to_be_bytes());
    }
    Ok(PdfObject::String(PdfString::new(bytes)))
}

/// Prepare a separate, fully validated revision using explicit missing values.
/// No page-tree flattening, content rewriting, inferred values, or overwriting
/// existing descriptions is performed. Keep the original bytes for provenance.
///
/// # Errors
/// Returns preflight errors, invalid/unknown author requests, or **all** remaining
/// validation findings. No prepared bytes are returned while validation fails.
pub fn prepare_tagged_pdf(base: &[u8], metadata: &TaggedPdfMetadata) -> Result<PreparedTaggedPdf> {
    let recovered = super::reorder::recover_tagged_structure(base)?;
    let before = validate_tagged_pdf(&recovered, &Default::default())?;
    let missing_descriptions: BTreeSet<_> = before
        .findings
        .iter()
        .filter(|f| f.code == TaggedPdfFindingCode::MissingAlternateText)
        .filter_map(|f| f.object)
        .collect();
    let mut reader = PdfReader::new(Cursor::new(&recovered))?;
    let mut update = IncrementalUpdate::from_base(&recovered)?;
    if let Some(language) = &metadata.language {
        if !before
            .findings
            .iter()
            .any(|f| f.code == TaggedPdfFindingCode::MissingDocumentLanguage)
        {
            return Err(invalid(
                "catalog language is already present; preparation cannot overwrite it",
            ));
        }
        let root = reader.trailer().root()?;
        let mut catalog = reader.catalog()?.clone();
        catalog.insert("Lang".to_string(), text_string(language)?);
        update.replace(root, PdfObject::Dictionary(catalog))?;
    }
    for (element, description) in &metadata.alternate_text {
        if !missing_descriptions.contains(element) {
            return Err(invalid(format!(
                "{} {} R is not a structure element requiring alternate text",
                element.object_number, element.generation
            )));
        }
        let id = (element.object_number, element.generation);
        let mut dictionary = reader
            .get_object(id.0, id.1)?
            .as_dict()
            .cloned()
            .ok_or_else(|| invalid("metadata target is not a structure dictionary"))?;
        dictionary.insert("Alt".to_string(), text_string(description)?);
        update.replace(id, PdfObject::Dictionary(dictionary))?;
    }
    let bytes = if metadata.language.is_none() && metadata.alternate_text.is_empty() {
        recovered
    } else {
        update.finish()?
    };
    let validation = validate_tagged_pdf(&bytes, &Default::default())?;
    if !validation.valid {
        return Err(invalid(format!(
            "tagged source requires explicit metadata or structural correction: {:?}",
            validation.findings
        )));
    }
    Ok(PreparedTaggedPdf {
        pdf_bytes: bytes,
        validation,
    })
}

// PreserveBase planning and publication both diagnose the entire source first.
// This deliberately does not accept a defective source merely because the
// requested first output omits the page carrying a missing description.
pub(super) fn ensure_tagged_source_ready(base: &[u8]) -> Result<()> {
    let mut reader = PdfReader::new(Cursor::new(base))?;
    if reader.catalog()?.contains_key("StructTreeRoot") {
        let report = preflight_tagged_pdf(base)?;
        if !report.valid {
            return Err(invalid(format!(
                "projected tagged structure is invalid; source preflight: {:?}",
                report.findings
            )));
        }
    }
    Ok(())
}
