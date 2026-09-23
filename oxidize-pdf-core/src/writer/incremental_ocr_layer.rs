//! Lossless incremental OCR text layers for existing PDFs.

use super::incremental_update::IncrementalUpdate;
use crate::error::{PdfError, Result};
use crate::parser::objects::{PdfArray, PdfDictionary, PdfName, PdfObject, PdfStream};
use crate::parser::{PdfDocument, PdfReader};
use crate::signatures::{ensure_modification_allowed, IncrementalModification};
use std::collections::HashSet;
use std::io::Cursor;

const OCR_FONT_NAME: &str = "OxidizeOCR";

/// One OCR word in PDF user-space coordinates.
#[derive(Debug, Clone, PartialEq)]
pub struct OcrLayerFragment {
    /// Recognized Unicode text.
    pub text: String,
    /// Source-image region `[x, y, width, height]` mapped to page coordinates.
    pub region: [f64; 4],
    /// Recognition confidence in the inclusive range 0–1.
    pub confidence: f64,
    /// Zero-based logical reading-order index.
    pub reading_order: u32,
}

/// OCR text to append to one existing page.
#[derive(Debug, Clone, PartialEq)]
pub struct OcrLayerPage {
    /// Zero-based page index.
    pub page_index: u32,
    /// BCP 47 language tag associated with the recognized text.
    pub language: String,
    /// Positioned fragments in deterministic logical order.
    pub fragments: Vec<OcrLayerFragment>,
}

/// Dry-run description of an incremental OCR update.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OcrLayerPlan {
    /// Pages whose dictionaries will be replaced in the new revision.
    pub pages: Vec<u32>,
    /// Number of new content streams.
    pub streams_added: usize,
    /// Number of isolated font resources added.
    pub resources_added: usize,
    /// Pages skipped because an oxidize-pdf OCR layer already exists.
    pub pages_skipped_existing_ocr: Vec<u32>,
}

/// Result of one validated incremental OCR update.
#[derive(Debug, Clone)]
pub struct OcrLayerUpdate {
    /// Complete PDF bytes; the input is an exact prefix.
    pub pdf_bytes: Vec<u8>,
    /// The plan that was applied.
    pub plan: OcrLayerPlan,
}

/// Plans and applies isolated OCR layers without rebuilding the document graph.
pub struct IncrementalOcrLayerEditor<'a> {
    base_bytes: &'a [u8],
}

impl<'a> IncrementalOcrLayerEditor<'a> {
    /// Create an editor over complete existing PDF bytes.
    pub fn new(base_bytes: &'a [u8]) -> Self {
        Self { base_bytes }
    }

    /// Validate the request and report the exact pages and object classes changed.
    ///
    /// # Errors
    ///
    /// Returns an error when the PDF or request is invalid, encrypted, certified
    /// against page-content edits, or uses an unsupported resource layout.
    pub fn plan(&self, pages: &[OcrLayerPage]) -> Result<OcrLayerPlan> {
        validate_policy(self.base_bytes)?;
        let document = parse_document(self.base_bytes)?;
        validate_request(&document, pages)?;
        build_plan(&document, pages)
    }

    /// Append exactly one incremental revision and reopen it for validation.
    ///
    /// # Errors
    ///
    /// Returns an error for any planning failure or when the incremental output
    /// cannot be serialized and reopened with every requested layer present.
    pub fn apply(&self, pages: &[OcrLayerPage]) -> Result<OcrLayerUpdate> {
        validate_policy(self.base_bytes)?;

        let document = parse_document(self.base_bytes)?;
        validate_request(&document, pages)?;
        let plan = build_plan(&document, pages)?;
        if plan.pages.is_empty() {
            return Ok(OcrLayerUpdate {
                pdf_bytes: self.base_bytes.to_vec(),
                plan,
            });
        }

        let mut update = IncrementalUpdate::from_base(self.base_bytes)?;
        for page_index in &plan.pages {
            let requested = pages
                .iter()
                .find(|page| page.page_index == *page_index)
                .ok_or_else(|| invalid("OCR plan references an absent page request"))?;
            let parsed_page = document
                .get_page(requested.page_index)
                .map_err(|error| invalid(format!("read page {}: {error}", requested.page_index)))?;
            let font_id = update.allocate_id()?;
            update.replace(font_id, PdfObject::Dictionary(ocr_font_dictionary()))?;
            let stream_id = update.allocate_id()?;
            let stream = PdfStream {
                dict: ocr_stream_dictionary(&requested.language),
                data: build_ocr_content(requested)?,
            };
            update.replace(stream_id, PdfObject::Stream(stream))?;

            let mut page_dictionary = parsed_page.dict.clone();
            let resources = merged_resources(&parsed_page, &document, font_id)?;
            page_dictionary.insert("Resources".to_string(), PdfObject::Dictionary(resources));
            page_dictionary.insert(
                "Contents".to_string(),
                append_content_reference(parsed_page.get_contents(), stream_id)?,
            );
            update.replace(parsed_page.obj_ref, PdfObject::Dictionary(page_dictionary))?;
        }

        let pdf_bytes = update.finish()?;
        validate_output(self.base_bytes, &pdf_bytes, &plan)?;
        Ok(OcrLayerUpdate { pdf_bytes, plan })
    }
}

fn validate_policy(bytes: &[u8]) -> Result<()> {
    let mut reader = PdfReader::new(Cursor::new(bytes))
        .map_err(|error| invalid(format!("parse base PDF: {error}")))?;
    if reader.is_encrypted() {
        return Err(PdfError::PermissionDenied(
            "incremental OCR is not supported on encrypted PDFs".to_string(),
        ));
    }
    let catalog = reader
        .catalog()
        .map_err(|error| invalid(format!("read catalog: {error}")))?
        .clone();
    ensure_modification_allowed(&mut reader, &catalog, IncrementalModification::OcrTextLayer)
}

fn parse_document(bytes: &[u8]) -> Result<PdfDocument<Cursor<&[u8]>>> {
    let reader = PdfReader::new(Cursor::new(bytes))
        .map_err(|error| invalid(format!("parse base PDF: {error}")))?;
    if reader.is_encrypted() {
        return Err(PdfError::PermissionDenied(
            "incremental OCR is not supported on encrypted PDFs".to_string(),
        ));
    }
    Ok(PdfDocument::new(reader))
}

fn validate_request(document: &PdfDocument<Cursor<&[u8]>>, pages: &[OcrLayerPage]) -> Result<()> {
    let page_count = document
        .page_count()
        .map_err(|error| invalid(format!("read page tree: {error}")))?;
    let mut seen = HashSet::new();
    for page in pages {
        if page.page_index >= page_count {
            return Err(invalid(format!(
                "OCR page {} is outside the document",
                page.page_index
            )));
        }
        if !seen.insert(page.page_index) {
            return Err(invalid(format!(
                "OCR page {} is requested more than once",
                page.page_index
            )));
        }
        if page.language.trim().is_empty() || !page.language.is_ascii() {
            return Err(invalid("OCR language must be a non-empty ASCII BCP 47 tag"));
        }
        let mut previous_order = None;
        for fragment in &page.fragments {
            if fragment.text.is_empty()
                || !fragment.confidence.is_finite()
                || !(0.0..=1.0).contains(&fragment.confidence)
                || fragment.region.iter().any(|value| !value.is_finite())
                || fragment.region[2] <= 0.0
                || fragment.region[3] <= 0.0
            {
                return Err(invalid(
                    "OCR fragment contains invalid text, confidence, or region",
                ));
            }
            if previous_order.is_some_and(|value| fragment.reading_order <= value) {
                return Err(invalid(
                    "OCR fragments must have strictly increasing logical reading order",
                ));
            }
            previous_order = Some(fragment.reading_order);
        }
    }
    Ok(())
}

fn build_plan(
    document: &PdfDocument<Cursor<&[u8]>>,
    pages: &[OcrLayerPage],
) -> Result<OcrLayerPlan> {
    let mut changed = Vec::new();
    let mut skipped = Vec::new();
    for requested in pages {
        let page = document
            .get_page(requested.page_index)
            .map_err(|error| invalid(format!("read page {}: {error}", requested.page_index)))?;
        if has_existing_ocr_layer(&page, document)? {
            skipped.push(requested.page_index);
        } else if !requested.fragments.is_empty() {
            merged_resources(&page, document, (u32::MAX, 0))?;
            append_content_reference(page.get_contents(), (u32::MAX, 0))?;
            changed.push(requested.page_index);
        }
    }
    changed.sort_unstable();
    skipped.sort_unstable();
    Ok(OcrLayerPlan {
        streams_added: changed.len(),
        resources_added: changed.len(),
        pages: changed,
        pages_skipped_existing_ocr: skipped,
    })
}

fn has_existing_ocr_layer(
    page: &crate::parser::page_tree::ParsedPage,
    document: &PdfDocument<Cursor<&[u8]>>,
) -> Result<bool> {
    let Some(contents) = page.get_contents() else {
        return Ok(false);
    };
    let values: Vec<&PdfObject> = match contents {
        PdfObject::Array(array) => array.0.iter().collect(),
        value => vec![value],
    };
    for value in values {
        let object =
            match value {
                PdfObject::Reference(number, generation) => document
                    .get_object(*number, *generation)
                    .map_err(|error| invalid(format!("resolve page content: {error}")))?,
                object => object.clone(),
            };
        if object
            .as_stream()
            .and_then(|stream| stream.dict.get("OxidizeOCR"))
            == Some(&PdfObject::Boolean(true))
        {
            return Ok(true);
        }
    }
    Ok(false)
}

fn merged_resources(
    page: &crate::parser::page_tree::ParsedPage,
    document: &PdfDocument<Cursor<&[u8]>>,
    font_id: (u32, u16),
) -> Result<PdfDictionary> {
    let mut resources = page.get_resources().cloned().unwrap_or_default();
    let mut fonts = match resources.get("Font") {
        None => PdfDictionary::new(),
        Some(PdfObject::Dictionary(fonts)) => fonts.clone(),
        Some(PdfObject::Reference(number, generation)) => document
            .get_object(*number, *generation)
            .map_err(|error| invalid(format!("resolve page fonts: {error}")))?
            .as_dict()
            .cloned()
            .ok_or_else(|| invalid("page /Font resource is not a dictionary"))?,
        Some(_) => return Err(invalid("page /Font resource is not a dictionary")),
    };
    if fonts.get(OCR_FONT_NAME).is_some() {
        return Err(invalid("page already reserves the /OxidizeOCR font name"));
    }
    fonts.insert(
        OCR_FONT_NAME.to_string(),
        PdfObject::Reference(font_id.0, font_id.1),
    );
    resources.insert("Font".to_string(), PdfObject::Dictionary(fonts));
    Ok(resources)
}

fn append_content_reference(
    contents: Option<&PdfObject>,
    stream_id: (u32, u16),
) -> Result<PdfObject> {
    let new_stream = PdfObject::Reference(stream_id.0, stream_id.1);
    match contents {
        None => Ok(new_stream),
        Some(PdfObject::Reference(number, generation)) => Ok(PdfObject::Array(PdfArray(vec![
            PdfObject::Reference(*number, *generation),
            new_stream,
        ]))),
        Some(PdfObject::Array(array))
            if array
                .0
                .iter()
                .all(|value| matches!(value, PdfObject::Reference(_, _))) =>
        {
            let mut values = array.0.clone();
            values.push(new_stream);
            Ok(PdfObject::Array(PdfArray(values)))
        }
        Some(_) => Err(invalid(
            "incremental OCR requires indirect page content streams",
        )),
    }
}

fn ocr_font_dictionary() -> PdfDictionary {
    let mut font = PdfDictionary::new();
    font.insert("Type".to_string(), name("Font"));
    font.insert("Subtype".to_string(), name("Type1"));
    font.insert("BaseFont".to_string(), name("Helvetica"));
    font.insert("Encoding".to_string(), name("WinAnsiEncoding"));
    font
}

fn ocr_stream_dictionary(language: &str) -> PdfDictionary {
    let mut dictionary = PdfDictionary::new();
    dictionary.insert("OxidizeOCR".to_string(), PdfObject::Boolean(true));
    dictionary.insert(
        "Lang".to_string(),
        PdfObject::String(crate::parser::objects::PdfString(
            language.as_bytes().to_vec(),
        )),
    );
    dictionary
}

fn build_ocr_content(page: &OcrLayerPage) -> Result<Vec<u8>> {
    let mut content = Vec::new();
    content.extend_from_slice(b"q\nBT\n/OxidizeOCR 1 Tf\n3 Tr\n");
    for (index, fragment) in page.fragments.iter().enumerate() {
        let [x, y, width, height] = fragment.region;
        let separator = (index + 1 < page.fragments.len())
            .then_some(" ")
            .unwrap_or("");
        let logical_text = format!("{}{separator}", fragment.text);
        let actual_text = encode_actual_text(&logical_text);
        let glyphs = encode_invisible_glyphs(&logical_text);
        let escaped_glyphs = escape_literal(&glyphs);
        let estimated_width = (fragment.text.chars().count().max(1) as f64) * 0.5;
        let horizontal_scale = (width / estimated_width * 100.0).clamp(1.0, 1000.0);
        content.extend_from_slice(
            format!(
                "/OCR << /ActualText {} /Lang ({}) /OxidizeConfidence {:.6} /OxidizeRegion [{:.6} {:.6} {:.6} {:.6}] /OxidizeReadingOrder {} >> BDC\n/OxidizeOCR {:.6} Tf\n{:.6} Tz\n1 0 0 1 {:.6} {:.6} Tm\n({}) Tj\nEMC\n",
                actual_text,
                escape_literal(page.language.as_bytes()),
                fragment.confidence,
                x,
                y,
                width,
                height,
                fragment.reading_order,
                height,
                horizontal_scale,
                x,
                y,
                escaped_glyphs,
            )
            .as_bytes(),
        );
    }
    content.extend_from_slice(b"ET\nQ\n");
    Ok(content)
}

fn encode_actual_text(text: &str) -> String {
    let mut output = String::from("<FEFF");
    for unit in text.encode_utf16() {
        output.push_str(&format!("{unit:04X}"));
    }
    output.push('>');
    output
}

fn encode_invisible_glyphs(text: &str) -> Vec<u8> {
    text.chars()
        .map(|character| {
            if character.is_ascii() {
                character as u8
            } else {
                b'?'
            }
        })
        .collect()
}

fn escape_literal(bytes: &[u8]) -> String {
    let mut output = String::new();
    for byte in bytes {
        match byte {
            b'(' | b')' | b'\\' => {
                output.push('\\');
                output.push(*byte as char);
            }
            b'\n' => output.push_str("\\n"),
            b'\r' => output.push_str("\\r"),
            byte => output.push(*byte as char),
        }
    }
    output
}

fn validate_output(base: &[u8], output: &[u8], plan: &OcrLayerPlan) -> Result<()> {
    if !output.starts_with(base) {
        return Err(invalid(
            "incremental OCR output does not preserve the input prefix",
        ));
    }
    let document = parse_document(output)?;
    for page_index in &plan.pages {
        let page = document
            .get_page(*page_index)
            .map_err(|error| invalid(format!("reopen page {page_index}: {error}")))?;
        if !has_existing_ocr_layer(&page, &document)? {
            return Err(invalid(format!(
                "reopened page {page_index} has no validated OCR layer"
            )));
        }
    }
    Ok(())
}

fn name(value: &str) -> PdfObject {
    PdfObject::Name(PdfName(value.to_string()))
}

fn invalid(message: impl Into<String>) -> PdfError {
    PdfError::InvalidStructure(message.into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::annotations::TextAnnotation;
    use crate::geometry::Point;
    use crate::text::{ExtractionOptions, TextExtractor};
    use crate::{Document, Font, Page};

    fn base_pdf() -> Vec<u8> {
        let mut document = Document::new();
        document.set_title("preserved metadata");
        let mut page = Page::a4();
        page.set_rotation(90);
        page.text()
            .set_font(Font::Helvetica, 12.0)
            .at(20.0, 800.0)
            .write("visible original")
            .unwrap();
        page.add_annotation(
            TextAnnotation::new(Point::new(40.0, 760.0))
                .with_contents("preserved annotation")
                .to_annotation(),
        );
        document.add_page(page);
        document.to_bytes().unwrap()
    }

    fn xref_stream_base_pdf() -> Vec<u8> {
        let mut document = Document::new();
        document.enable_xref_streams(true);
        document.set_title("xref metadata");
        document.add_page(Page::a4());
        document.to_bytes().unwrap()
    }

    fn layer() -> OcrLayerPage {
        OcrLayerPage {
            page_index: 0,
            language: "en-US".to_string(),
            fragments: vec![OcrLayerFragment {
                text: "searchable".to_string(),
                region: [50.0, 700.0, 80.0, 12.0],
                confidence: 0.98,
                reading_order: 0,
            }],
        }
    }

    #[test]
    fn dry_run_reports_only_incremental_ocr_objects() {
        let base = base_pdf();
        let plan = IncrementalOcrLayerEditor::new(&base)
            .plan(&[layer()])
            .unwrap();
        assert_eq!(plan.pages, vec![0]);
        assert_eq!(plan.streams_added, 1);
        assert_eq!(plan.resources_added, 1);
        assert!(plan.pages_skipped_existing_ocr.is_empty());
    }

    #[test]
    fn update_preserves_base_as_exact_prefix_and_reopens() {
        let base = base_pdf();
        let update = IncrementalOcrLayerEditor::new(&base)
            .apply(&[layer()])
            .unwrap();
        assert!(update.pdf_bytes.starts_with(&base));
        assert!(update.pdf_bytes.len() > base.len());
        let document = parse_document(&update.pdf_bytes).unwrap();
        assert_eq!(document.page_count().unwrap(), 1);
        let page = document.get_page(0).unwrap();
        assert_eq!(page.rotation, 90);
        assert!(page.has_annotations());
        assert_eq!(
            document.metadata().unwrap().title.as_deref(),
            Some("preserved metadata")
        );
        let extracted = TextExtractor::with_options(ExtractionOptions::default())
            .extract_from_page(&document, 0)
            .unwrap();
        assert!(extracted.text.contains("visible original"));
        assert!(extracted.text.contains("searchable"));
    }

    #[test]
    fn second_application_detects_existing_ocr_layer() {
        let base = base_pdf();
        let first = IncrementalOcrLayerEditor::new(&base)
            .apply(&[layer()])
            .unwrap();
        let second = IncrementalOcrLayerEditor::new(&first.pdf_bytes)
            .apply(&[layer()])
            .unwrap();
        assert_eq!(second.pdf_bytes, first.pdf_bytes);
        assert_eq!(second.plan.pages_skipped_existing_ocr, vec![0]);
    }

    #[test]
    fn supports_xref_stream_sources() {
        let base = xref_stream_base_pdf();
        assert!(base
            .windows(b"/Type /XRef".len())
            .any(|bytes| bytes == b"/Type /XRef"));
        let update = IncrementalOcrLayerEditor::new(&base)
            .apply(&[layer()])
            .unwrap();
        assert!(update.pdf_bytes.starts_with(&base));
        let reopened = parse_document(&update.pdf_bytes).unwrap();
        assert_eq!(reopened.page_count().unwrap(), 1);
        assert_eq!(
            reopened.metadata().unwrap().title.as_deref(),
            Some("xref metadata")
        );
    }

    #[test]
    fn unicode_actual_text_and_logical_order_are_extractable() {
        let base = base_pdf();
        let mut requested = layer();
        requested.fragments = vec![
            OcrLayerFragment {
                text: "café".to_string(),
                region: [50.0, 700.0, 40.0, 12.0],
                confidence: 0.99,
                reading_order: 0,
            },
            OcrLayerFragment {
                text: "世界".to_string(),
                region: [95.0, 700.0, 30.0, 12.0],
                confidence: 0.97,
                reading_order: 1,
            },
        ];
        let update = IncrementalOcrLayerEditor::new(&base)
            .apply(&[requested])
            .unwrap();
        let document = parse_document(&update.pdf_bytes).unwrap();
        let extracted = TextExtractor::with_options(ExtractionOptions::default())
            .extract_from_page(&document, 0)
            .unwrap();
        let first = extracted.text.find("café").unwrap();
        let second = extracted.text.find("世界").unwrap();
        assert!(first < second, "{}", extracted.text);
    }

    #[test]
    fn docmdp_certification_rejects_ocr_page_content_changes() {
        let base = include_bytes!("../../tests/fixtures/signatures/docmdp_p2_rsa.pdf");
        let error = IncrementalOcrLayerEditor::new(base)
            .apply(&[layer()])
            .unwrap_err();
        assert!(matches!(error, PdfError::PermissionDenied(message) if message.contains("DocMDP")));
    }

    #[test]
    fn dry_run_also_rejects_forbidden_docmdp_changes() {
        let base = include_bytes!("../../tests/fixtures/signatures/docmdp_p2_rsa.pdf");
        let error = IncrementalOcrLayerEditor::new(base)
            .plan(&[layer()])
            .unwrap_err();
        assert!(matches!(error, PdfError::PermissionDenied(message) if message.contains("DocMDP")));
    }

    #[test]
    fn page_request_order_does_not_change_output() {
        let mut document = Document::new();
        document.add_page(Page::a4());
        document.add_page(Page::a4());
        let base = document.to_bytes().unwrap();
        let first = layer();
        let mut second = layer();
        second.page_index = 1;
        let forward = IncrementalOcrLayerEditor::new(&base)
            .apply(&[first.clone(), second.clone()])
            .unwrap();
        let reverse = IncrementalOcrLayerEditor::new(&base)
            .apply(&[second, first])
            .unwrap();
        assert_eq!(forward.pdf_bytes, reverse.pdf_bytes);
    }
}
