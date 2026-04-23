//! PDF overlay/watermark functionality
//!
//! Implements overlay operations for superimposing pages from one PDF onto another.
//! Common use cases: watermarks ("DRAFT", "CONFIDENTIAL"), logos, stamps.
//!
//! # Technical approach
//!
//! Each overlay page is converted to a Form XObject (ISO 32000-1 §8.10) and
//! injected into the target page's content stream with appropriate CTM
//! (Coordinate Transformation Matrix) for positioning and scaling.

use super::{OperationError, OperationResult, PageRange};
use crate::geometry::{Point, Rectangle};
use crate::graphics::{ExtGState, FormXObject};
use crate::parser::{PdfDocument, PdfReader};
use crate::{Document, Page};
use std::collections::{HashMap, HashSet};
use std::io::{Read, Seek};
use std::path::Path;

/// Position for overlay placement on the target page.
#[derive(Debug, Clone, PartialEq)]
pub enum OverlayPosition {
    /// Centered on the page
    Center,
    /// Top-left corner
    TopLeft,
    /// Top-right corner
    TopRight,
    /// Bottom-left corner
    BottomLeft,
    /// Bottom-right corner
    BottomRight,
    /// Custom position (x, y) in points from bottom-left
    Custom(f64, f64),
}

impl Default for OverlayPosition {
    fn default() -> Self {
        Self::Center
    }
}

/// Options for overlay operations.
#[derive(Debug, Clone)]
pub struct OverlayOptions {
    /// Which pages to apply the overlay to (default: all)
    pub pages: PageRange,
    /// Position of the overlay on the target page
    pub position: OverlayPosition,
    /// Opacity of the overlay (0.0 = transparent, 1.0 = opaque)
    pub opacity: f64,
    /// Scale factor for the overlay (1.0 = original size)
    pub scale: f64,
    /// If true, cycle through overlay pages when base has more pages than overlay
    pub repeat: bool,
}

impl Default for OverlayOptions {
    fn default() -> Self {
        Self {
            pages: PageRange::All,
            position: OverlayPosition::Center,
            opacity: 1.0,
            scale: 1.0,
            repeat: false,
        }
    }
}

impl OverlayOptions {
    /// Validates the options, returning an error if invalid.
    pub fn validate(&self) -> OperationResult<()> {
        if self.scale <= 0.0 {
            return Err(OperationError::ProcessingError(
                "Overlay scale must be greater than 0".to_string(),
            ));
        }
        Ok(())
    }

    /// Returns the opacity clamped to [0.0, 1.0].
    fn clamped_opacity(&self) -> f64 {
        self.opacity.clamp(0.0, 1.0)
    }
}

/// Computes the CTM (Coordinate Transformation Matrix) for positioning the overlay.
///
/// Returns `[sx, 0, 0, sy, tx, ty]` where:
/// - `sx`, `sy` = scale factors
/// - `tx`, `ty` = translation offsets
pub(crate) fn compute_ctm(
    base_w: f64,
    base_h: f64,
    overlay_w: f64,
    overlay_h: f64,
    scale: f64,
    position: &OverlayPosition,
) -> [f64; 6] {
    let scaled_w = overlay_w * scale;
    let scaled_h = overlay_h * scale;

    let (tx, ty) = match position {
        OverlayPosition::Center => ((base_w - scaled_w) / 2.0, (base_h - scaled_h) / 2.0),
        OverlayPosition::TopLeft => (0.0, base_h - scaled_h),
        OverlayPosition::TopRight => (base_w - scaled_w, base_h - scaled_h),
        OverlayPosition::BottomLeft => (0.0, 0.0),
        OverlayPosition::BottomRight => (base_w - scaled_w, 0.0),
        OverlayPosition::Custom(x, y) => (*x, *y),
    };

    [scale, 0.0, 0.0, scale, tx, ty]
}

/// Converts a parser `PdfDictionary` directly to a writer `objects::Dictionary`.
///
/// Used to pass overlay page resources into the Form XObject's resource dictionary.
/// References are resolved against `doc` (the source/overlay document) so that
/// the resulting writer objects contain inline data rather than dangling IDs
/// from the source PDF. See issue #156.
fn convert_parser_dict_to_objects_dict<R: Read + Seek>(
    parser_dict: &crate::parser::objects::PdfDictionary,
    doc: &PdfDocument<R>,
) -> crate::objects::Dictionary {
    let mut result = crate::objects::Dictionary::new();
    for (key, value) in &parser_dict.0 {
        let converted = convert_parser_obj_to_objects_obj(value, doc);
        result.set(key.as_str(), converted);
    }
    result
}

/// Converts a single parser `PdfObject` to a writer `objects::Object`.
///
/// `PdfObject::Reference` values are resolved against `doc` (the source document)
/// and recursively converted, so the returned writer object tree contains only
/// inline data — no references to foreign object IDs. This prevents dangling
/// references when the writer assigns new IDs in the destination PDF (issue #156).
fn convert_parser_obj_to_objects_obj<R: Read + Seek>(
    obj: &crate::parser::objects::PdfObject,
    doc: &PdfDocument<R>,
) -> crate::objects::Object {
    use crate::objects::Object as WObj;
    use crate::parser::objects::PdfObject as PObj;

    match obj {
        PObj::Null => WObj::Null,
        PObj::Boolean(b) => WObj::Boolean(*b),
        PObj::Integer(i) => WObj::Integer(*i),
        PObj::Real(r) => WObj::Real(*r),
        PObj::String(s) => WObj::String(String::from_utf8_lossy(s.as_bytes()).to_string()),
        PObj::Name(n) => WObj::Name(n.as_str().to_string()),
        PObj::Array(arr) => {
            let items: Vec<WObj> = arr
                .0
                .iter()
                .map(|item| convert_parser_obj_to_objects_obj(item, doc))
                .collect();
            WObj::Array(items)
        }
        PObj::Dictionary(dict) => WObj::Dictionary(convert_parser_dict_to_objects_dict(dict, doc)),
        PObj::Stream(stream) => {
            let dict = convert_parser_dict_to_objects_dict(&stream.dict, doc);
            WObj::Stream(dict, stream.data.clone())
        }
        PObj::Reference(num, gen) => {
            // Resolve the reference against the SOURCE document so we get the
            // actual object data instead of a raw ID that belongs to the overlay
            // PDF. The writer will later externalize any inline streams with
            // fresh IDs valid in the destination PDF.
            match doc.get_object(*num, *gen as u16) {
                Ok(resolved) => convert_parser_obj_to_objects_obj(&resolved, doc),
                Err(_) => {
                    tracing::warn!(
                        "Could not resolve reference {} {} R from overlay; replacing with Null",
                        num,
                        gen
                    );
                    WObj::Null
                }
            }
        }
    }
}

/// Applies overlay pages onto a base document.
pub struct PdfOverlay<R: Read + Seek> {
    base_doc: PdfDocument<R>,
    overlay_doc: PdfDocument<R>,
}

impl<R: Read + Seek> PdfOverlay<R> {
    /// Creates a new overlay applicator.
    pub fn new(base_doc: PdfDocument<R>, overlay_doc: PdfDocument<R>) -> Self {
        Self {
            base_doc,
            overlay_doc,
        }
    }

    /// Applies the overlay and returns the resulting document.
    pub fn apply(&self, options: &OverlayOptions) -> OperationResult<Document> {
        options.validate()?;

        let base_count =
            self.base_doc
                .page_count()
                .map_err(|e| OperationError::ParseError(e.to_string()))? as usize;

        if base_count == 0 {
            return Err(OperationError::NoPagesToProcess);
        }

        let overlay_count =
            self.overlay_doc
                .page_count()
                .map_err(|e| OperationError::ParseError(e.to_string()))? as usize;

        if overlay_count == 0 {
            return Err(OperationError::ProcessingError(
                "Overlay PDF has no pages".to_string(),
            ));
        }

        let target_indices = options.pages.get_indices(base_count)?;
        let clamped_opacity = options.clamped_opacity();

        let mut output_doc = Document::new();

        for page_idx in 0..base_count {
            let parsed_base = self
                .base_doc
                .get_page(page_idx as u32)
                .map_err(|e| OperationError::ParseError(e.to_string()))?;

            let mut page = Page::from_parsed_with_content(&parsed_base, &self.base_doc)
                .map_err(OperationError::PdfError)?;

            if target_indices.contains(&page_idx) {
                // Determine which overlay page to use
                let target_pos = target_indices
                    .iter()
                    .position(|&i| i == page_idx)
                    .unwrap_or(0);

                let overlay_page_idx = if options.repeat || overlay_count == 1 {
                    target_pos % overlay_count
                } else if target_pos < overlay_count {
                    target_pos
                } else {
                    // No overlay page available for this target, skip overlay
                    output_doc.add_page(page);
                    continue;
                };

                self.apply_overlay_to_page(
                    &mut page,
                    overlay_page_idx,
                    &parsed_base,
                    clamped_opacity,
                    options.scale,
                    &options.position,
                )?;
            }

            output_doc.add_page(page);
        }

        Ok(output_doc)
    }

    /// Applies a single overlay page onto a base page.
    fn apply_overlay_to_page(
        &self,
        page: &mut Page,
        overlay_page_idx: usize,
        parsed_base: &crate::parser::page_tree::ParsedPage,
        opacity: f64,
        scale: f64,
        position: &OverlayPosition,
    ) -> OperationResult<()> {
        let parsed_overlay = self
            .overlay_doc
            .get_page(overlay_page_idx as u32)
            .map_err(|e| OperationError::ParseError(e.to_string()))?;

        // Extract overlay content streams
        let overlay_streams = self
            .overlay_doc
            .get_page_content_streams(&parsed_overlay)
            .map_err(|e| OperationError::ParseError(e.to_string()))?;

        let mut overlay_content = Vec::new();
        for stream in &overlay_streams {
            overlay_content.extend_from_slice(stream);
            overlay_content.push(b'\n');
        }

        // Build Form XObject from overlay content
        let ov_w = parsed_overlay.width();
        let ov_h = parsed_overlay.height();
        let bbox = Rectangle::new(Point::new(0.0, 0.0), Point::new(ov_w, ov_h));

        let mut form = FormXObject::new(bbox).with_content(overlay_content);

        // Preserve overlay page resources in the Form XObject so fonts, images, etc. are available
        if let Some(resources) = parsed_overlay.get_resources() {
            let writer_dict = convert_parser_dict_to_objects_dict(resources, &self.overlay_doc);
            form = form.with_resources(writer_dict);
        }

        let xobj_name = format!("Overlay{}", overlay_page_idx);
        // Overlay-generated names are under our control (`Overlay{n}`)
        // and always valid per ISO 32000-1 §7.3.5, so `?` is defensive
        // here rather than a practical failure mode.
        page.add_form_xobject(&xobj_name, form)?;

        // Calculate CTM for positioning and scaling
        let base_w = parsed_base.width();
        let base_h = parsed_base.height();
        let ctm = compute_ctm(base_w, base_h, ov_w, ov_h, scale, position);

        // Build overlay operators: q [gs] cm Do Q
        let mut ops = String::new();
        ops.push_str("q\n");

        // Apply opacity via ExtGState if opacity is less than 1.0
        if (opacity - 1.0).abs() > f64::EPSILON {
            let mut state = ExtGState::new();
            state.alpha_fill = Some(opacity);
            state.alpha_stroke = Some(opacity);

            let registered_name = page
                .graphics()
                .extgstate_manager_mut()
                .add_state(state)
                .map_err(|e| OperationError::ProcessingError(format!("ExtGState error: {e}")))?;

            ops.push_str(&format!("/{} gs\n", registered_name));
        }

        // Apply CTM for positioning and scaling
        ops.push_str(&format!(
            "{} {} {} {} {} {} cm\n",
            ctm[0], ctm[1], ctm[2], ctm[3], ctm[4], ctm[5]
        ));

        // Invoke the Form XObject
        ops.push_str(&format!("/{} Do\n", xobj_name));
        ops.push_str("Q\n");

        // Append overlay operators to page content (renders on top of
        // existing content).
        //
        // The overlay path composes a `cm` matrix + `/<xobj> Do` — it
        // does NOT emit `Tj` operators directly. The XObject invoked
        // carries its own font references and character data (those
        // live in the source PDF's resources, independent of this
        // Document's `custom_fonts` registry). Consequently there are
        // no fonts OF THE TARGET DOCUMENT referenced inside `ops`, and
        // the issue-#204 font-usage map is correctly empty here. If a
        // future overlay variant starts embedding inline `Tj` against
        // target-document fonts, it must populate this map.
        let font_usage: HashMap<String, HashSet<char>> = HashMap::new();
        page.append_raw_content(ops.as_bytes(), &font_usage);

        Ok(())
    }
}

/// High-level function to apply a PDF overlay/watermark.
///
/// Reads the base PDF and overlay PDF from disk, applies the overlay
/// according to the given options, and writes the result to the output path.
///
/// # Arguments
///
/// * `base_path` - Path to the base PDF document
/// * `overlay_path` - Path to the overlay/watermark PDF
/// * `output_path` - Path for the output PDF
/// * `options` - Overlay configuration (position, opacity, scale, etc.)
///
/// # Example
///
/// ```rust,no_run
/// use oxidize_pdf::operations::{overlay_pdf, OverlayOptions, OverlayPosition};
///
/// // Apply a centered watermark at 30% opacity
/// overlay_pdf(
///     "document.pdf",
///     "watermark.pdf",
///     "output.pdf",
///     OverlayOptions {
///         opacity: 0.3,
///         position: OverlayPosition::Center,
///         ..Default::default()
///     },
/// ).unwrap();
/// ```
pub fn overlay_pdf<P, Q, R>(
    base_path: P,
    overlay_path: Q,
    output_path: R,
    options: OverlayOptions,
) -> OperationResult<()>
where
    P: AsRef<Path>,
    Q: AsRef<Path>,
    R: AsRef<Path>,
{
    let base_reader = PdfReader::open(base_path.as_ref())
        .map_err(|e| OperationError::ParseError(format!("Failed to open base PDF: {e}")))?;
    let base_doc = PdfDocument::new(base_reader);

    let overlay_reader = PdfReader::open(overlay_path.as_ref())
        .map_err(|e| OperationError::ParseError(format!("Failed to open overlay PDF: {e}")))?;
    let overlay_doc = PdfDocument::new(overlay_reader);

    let overlay_applicator = PdfOverlay::new(base_doc, overlay_doc);
    let mut doc = overlay_applicator.apply(&options)?;
    doc.save(output_path)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_overlay_options_default() {
        let opts = OverlayOptions::default();
        assert_eq!(opts.opacity, 1.0);
        assert_eq!(opts.scale, 1.0);
        assert!(!opts.repeat);
        assert!(matches!(opts.position, OverlayPosition::Center));
        assert!(matches!(opts.pages, PageRange::All));
    }

    #[test]
    fn test_overlay_options_validate_ok() {
        let opts = OverlayOptions::default();
        assert!(opts.validate().is_ok());
    }

    #[test]
    fn test_overlay_options_validate_zero_scale() {
        let opts = OverlayOptions {
            scale: 0.0,
            ..Default::default()
        };
        assert!(opts.validate().is_err());
    }

    #[test]
    fn test_overlay_options_validate_negative_scale() {
        let opts = OverlayOptions {
            scale: -1.0,
            ..Default::default()
        };
        assert!(opts.validate().is_err());
    }

    #[test]
    fn test_overlay_options_validate_high_opacity_ok() {
        let opts = OverlayOptions {
            opacity: 2.5,
            ..Default::default()
        };
        // opacity > 1.0 is clamped, not rejected
        assert!(opts.validate().is_ok());
        assert_eq!(opts.clamped_opacity(), 1.0);
    }

    #[test]
    fn test_overlay_options_clamped_opacity() {
        assert_eq!(
            OverlayOptions {
                opacity: -0.5,
                ..Default::default()
            }
            .clamped_opacity(),
            0.0
        );
        assert_eq!(
            OverlayOptions {
                opacity: 0.5,
                ..Default::default()
            }
            .clamped_opacity(),
            0.5
        );
        assert_eq!(
            OverlayOptions {
                opacity: 3.0,
                ..Default::default()
            }
            .clamped_opacity(),
            1.0
        );
    }

    #[test]
    fn test_compute_ctm_center_same_size() {
        let ctm = compute_ctm(595.0, 842.0, 595.0, 842.0, 1.0, &OverlayPosition::Center);
        assert_eq!(ctm[0], 1.0);
        assert_eq!(ctm[3], 1.0);
        assert!((ctm[4] - 0.0).abs() < 0.001);
        assert!((ctm[5] - 0.0).abs() < 0.001);
    }

    #[test]
    fn test_compute_ctm_center_different_sizes() {
        let ctm = compute_ctm(595.0, 842.0, 200.0, 200.0, 1.0, &OverlayPosition::Center);
        assert!((ctm[4] - 197.5).abs() < 0.001);
        assert!((ctm[5] - 321.0).abs() < 0.001);
    }

    #[test]
    fn test_compute_ctm_with_scale() {
        let ctm = compute_ctm(595.0, 842.0, 595.0, 842.0, 0.5, &OverlayPosition::Center);
        assert!((ctm[0] - 0.5).abs() < 0.001);
        assert!((ctm[3] - 0.5).abs() < 0.001);
        // Centered: tx = (595 - 595*0.5) / 2 = 148.75
        assert!((ctm[4] - 148.75).abs() < 0.001);
        assert!((ctm[5] - 210.5).abs() < 0.001);
    }

    #[test]
    fn test_compute_ctm_bottom_left() {
        let ctm = compute_ctm(
            595.0,
            842.0,
            200.0,
            200.0,
            1.0,
            &OverlayPosition::BottomLeft,
        );
        assert!((ctm[4]).abs() < 0.001);
        assert!((ctm[5]).abs() < 0.001);
    }

    #[test]
    fn test_compute_ctm_bottom_right() {
        let ctm = compute_ctm(
            595.0,
            842.0,
            200.0,
            200.0,
            1.0,
            &OverlayPosition::BottomRight,
        );
        assert!((ctm[4] - 395.0).abs() < 0.001);
        assert!((ctm[5]).abs() < 0.001);
    }

    #[test]
    fn test_compute_ctm_top_left() {
        let ctm = compute_ctm(595.0, 842.0, 200.0, 200.0, 1.0, &OverlayPosition::TopLeft);
        assert!((ctm[4]).abs() < 0.001);
        assert!((ctm[5] - 642.0).abs() < 0.001);
    }

    #[test]
    fn test_compute_ctm_top_right() {
        let ctm = compute_ctm(595.0, 842.0, 200.0, 200.0, 1.0, &OverlayPosition::TopRight);
        assert!((ctm[4] - 395.0).abs() < 0.001);
        assert!((ctm[5] - 642.0).abs() < 0.001);
    }

    #[test]
    fn test_compute_ctm_custom_position() {
        let ctm = compute_ctm(
            595.0,
            842.0,
            200.0,
            200.0,
            1.0,
            &OverlayPosition::Custom(100.0, 150.0),
        );
        assert!((ctm[4] - 100.0).abs() < 0.001);
        assert!((ctm[5] - 150.0).abs() < 0.001);
    }

    #[test]
    fn test_overlay_position_default() {
        assert_eq!(OverlayPosition::default(), OverlayPosition::Center);
    }

    #[test]
    fn test_overlay_position_equality() {
        assert_eq!(OverlayPosition::Center, OverlayPosition::Center);
        assert_eq!(
            OverlayPosition::Custom(1.0, 2.0),
            OverlayPosition::Custom(1.0, 2.0)
        );
        assert_ne!(OverlayPosition::Center, OverlayPosition::TopLeft);
    }

    /// Issue #156: unresolvable references must degrade to Null, not panic.
    #[test]
    fn test_unresolvable_reference_degrades_to_null() {
        use crate::objects::Object as WObj;
        use crate::parser::objects::{PdfDictionary, PdfName, PdfObject as PObj};

        // Build a PdfDictionary containing a reference to a non-existent object.
        let mut dict = PdfDictionary::new();
        dict.0
            .insert(PdfName::new("SMask".to_string()), PObj::Reference(99999, 0));
        dict.0
            .insert(PdfName::new("Width".to_string()), PObj::Integer(100));

        // Create a minimal in-memory PDF to use as the document for resolution.
        let mut doc_builder = crate::Document::new();
        let page = crate::Page::a4();
        doc_builder.add_page(page);
        let pdf_bytes = doc_builder.to_bytes().unwrap();

        let reader = crate::parser::PdfReader::new(std::io::Cursor::new(pdf_bytes)).unwrap();
        let pdf_doc = crate::parser::PdfDocument::new(reader);

        let result = convert_parser_dict_to_objects_dict(&dict, &pdf_doc);

        // The unresolvable reference (99999 0 R) should become Null.
        let smask_key = "SMask";
        let smask_val = result.get(smask_key);
        assert!(
            matches!(smask_val, Some(WObj::Null)),
            "Unresolvable reference should become Null, got: {:?}",
            smask_val
        );

        // Other values should convert normally.
        let width_val = result.get("Width");
        assert!(
            matches!(width_val, Some(WObj::Integer(100))),
            "Normal integer should convert, got: {:?}",
            width_val
        );
    }
}
