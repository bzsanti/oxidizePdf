//! Incremental editing of standard `/Highlight` annotations in existing PDFs.

use super::incremental_annotations::{subtype, AnnotationContainer, AnnotationSnapshot};
use super::incremental_update::IncrementalUpdate;
use crate::error::{PdfError, Result};
use crate::geometry::Point;
use crate::parser::objects::{PdfArray, PdfDictionary, PdfName, PdfObject, PdfString};
use std::collections::{HashMap, HashSet};

/// Stable indirect-object identity of a highlight annotation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct HighlightId {
    /// PDF object number.
    pub object_number: u32,
    /// PDF generation number.
    pub generation_number: u16,
}

/// A validated highlight quadrilateral in PDF `/QuadPoints` order.
///
/// The points are the start and end of the first edge followed by the start
/// and end of the opposite edge. They must describe a convex, non-degenerate
/// quadrilateral.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HighlightQuad([Point; 4]);

impl HighlightQuad {
    /// Validate and construct a highlight quadrilateral.
    ///
    /// # Errors
    ///
    /// Returns [`PdfError::InvalidStructure`] for non-finite, repeated,
    /// degenerate, crossed, or concave points.
    pub fn new(points: [Point; 4]) -> Result<Self> {
        validate_quad_shape(&points)?;
        Ok(Self(points))
    }

    /// Return the points in PDF `/QuadPoints` order.
    pub const fn points(&self) -> &[Point; 4] {
        &self.0
    }
}

/// A validated DeviceRGB color for a highlight annotation.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HighlightColor([f64; 3]);

impl HighlightColor {
    /// Construct an RGB color whose components are in the inclusive range 0–1.
    ///
    /// # Errors
    ///
    /// Returns [`PdfError::InvalidStructure`] for non-finite or out-of-range components.
    pub fn new(components: [f64; 3]) -> Result<Self> {
        validate_unit_values(&components, "highlight RGB color")?;
        Ok(Self(components))
    }

    /// Return the RGB components.
    pub const fn components(self) -> [f64; 3] {
        self.0
    }
}

/// A validated highlight opacity (`/CA`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HighlightOpacity(f64);

impl HighlightOpacity {
    /// Construct an opacity in the inclusive range 0–1.
    ///
    /// # Errors
    ///
    /// Returns [`PdfError::InvalidStructure`] for a non-finite or out-of-range value.
    pub fn new(value: f64) -> Result<Self> {
        validate_unit_values(&[value], "highlight opacity")?;
        Ok(Self(value))
    }

    /// Return the opacity value.
    pub const fn value(self) -> f64 {
        self.0
    }
}

impl HighlightId {
    /// Construct an identity from a PDF indirect reference.
    pub const fn new(object_number: u32, generation_number: u16) -> Self {
        Self {
            object_number,
            generation_number,
        }
    }

    fn tuple(self) -> (u32, u16) {
        (self.object_number, self.generation_number)
    }
}

/// A standard PDF highlight annotation.
#[derive(Debug, Clone, PartialEq)]
pub struct Highlight {
    /// Stable indirect-object identity.
    pub id: HighlightId,
    /// Zero-based page index.
    pub page_index: u32,
    /// Annotation bounding rectangle `[left, bottom, right, top]`.
    pub rect: [f64; 4],
    /// One or more quadrilaterals, each in PDF `/QuadPoints` order.
    pub quadrilaterals: Vec<HighlightQuad>,
    /// RGB color components in the inclusive range 0–1.
    pub color: HighlightColor,
    /// Optional constant opacity (`/CA`) in the inclusive range 0–1.
    pub opacity: Option<HighlightOpacity>,
    /// Optional annotation contents.
    pub contents: Option<String>,
}

/// A requested change to the document's highlights.
#[derive(Debug, Clone, PartialEq)]
pub enum HighlightMutation {
    /// Add a highlight to a page.
    Add {
        /// Zero-based target page.
        page_index: u32,
        /// Non-empty validated highlight geometry.
        quadrilaterals: Vec<HighlightQuad>,
        /// DeviceRGB highlight color.
        color: HighlightColor,
        /// Optional constant opacity.
        opacity: Option<HighlightOpacity>,
        /// Optional annotation contents.
        contents: Option<String>,
    },
    /// Remove an existing highlight reference from its page.
    Remove {
        /// Stable identity of the highlight to remove.
        id: HighlightId,
    },
}

/// Result of one atomic incremental highlight update.
#[derive(Debug, Clone)]
pub struct HighlightUpdate {
    /// Complete PDF bytes. The original document is an exact prefix.
    pub pdf_bytes: Vec<u8>,
    /// Complete current highlight set after the update.
    pub highlights: Vec<Highlight>,
}

/// Reads and atomically edits standard highlights in an existing PDF.
pub struct IncrementalHighlightEditor<'a> {
    base_bytes: &'a [u8],
}

impl<'a> IncrementalHighlightEditor<'a> {
    /// Create an editor over an existing PDF's bytes.
    pub fn new(base_bytes: &'a [u8]) -> Self {
        Self { base_bytes }
    }

    /// Read every indirect `/Highlight` annotation with its stable identity.
    ///
    /// # Errors
    ///
    /// Returns an error when the PDF is malformed or encrypted, or when a
    /// highlight has no stable identity or contains invalid properties.
    pub fn highlights(&self) -> Result<Vec<Highlight>> {
        let snapshot = AnnotationSnapshot::parse(self.base_bytes, "Highlight", "highlight")?;
        read_highlights(&snapshot)
    }

    /// Validate and apply a batch as exactly one incremental revision.
    ///
    /// # Errors
    ///
    /// Returns an error for malformed or encrypted PDFs, invalid pages or
    /// identities, conflicting mutations, invalid geometry, and exhausted
    /// indirect-object numbers.
    pub fn apply(&self, mutations: &[HighlightMutation]) -> Result<HighlightUpdate> {
        let mut snapshot = AnnotationSnapshot::parse(self.base_bytes, "Highlight", "highlight")?;
        let existing = read_highlights(&snapshot)?;
        if mutations.is_empty() {
            return Ok(HighlightUpdate {
                pdf_bytes: self.base_bytes.to_vec(),
                highlights: existing,
            });
        }
        let existing_pages = validate_batch(&snapshot, &existing, mutations)?;
        let mut update = IncrementalUpdate::from_base(self.base_bytes)?;
        let mut changed_pages = HashSet::new();

        for mutation in mutations {
            match mutation {
                HighlightMutation::Add {
                    page_index,
                    quadrilaterals,
                    color,
                    opacity,
                    contents,
                } => {
                    let id = update.allocate_id()?;
                    update.replace(
                        id,
                        PdfObject::Dictionary(new_dictionary(
                            quadrilaterals,
                            *color,
                            *opacity,
                            contents.as_deref(),
                        )),
                    )?;
                    snapshot.pages[*page_index as usize]
                        .annotations
                        .push(PdfObject::Reference(id.0, id.1));
                    changed_pages.insert(*page_index);
                }
                HighlightMutation::Remove { id } => {
                    let page_index = *existing_pages.get(id).ok_or_else(|| {
                        invalid(&format!(
                            "highlight {} {} does not exist",
                            id.object_number, id.generation_number
                        ))
                    })?;
                    snapshot.pages[page_index as usize]
                        .annotations
                        .retain(|value| value.as_reference() != Some(id.tuple()));
                    changed_pages.insert(page_index);
                }
            }
        }

        for page_index in changed_pages {
            let page = &mut snapshot.pages[page_index as usize];
            match page.container {
                AnnotationContainer::Page => {
                    page.dictionary.insert(
                        "Annots".to_string(),
                        PdfObject::Array(PdfArray(page.annotations.clone())),
                    );
                    update.replace(
                        page.reference,
                        PdfObject::Dictionary(page.dictionary.clone()),
                    )?;
                }
                AnnotationContainer::Indirect(id) => {
                    update.replace(id, PdfObject::Array(PdfArray(page.annotations.clone())))?;
                }
            }
        }
        let pdf_bytes = update.finish()?;
        let highlights = IncrementalHighlightEditor::new(&pdf_bytes).highlights()?;
        Ok(HighlightUpdate {
            pdf_bytes,
            highlights,
        })
    }
}

fn read_highlights(snapshot: &AnnotationSnapshot) -> Result<Vec<Highlight>> {
    let mut result = Vec::new();
    for (&(number, generation), state) in &snapshot.annotations {
        if subtype(&state.dictionary) != Some("Highlight") {
            continue;
        }
        result.push(parse_highlight(
            HighlightId::new(number, generation),
            state.page_index,
            &state.dictionary,
        )?);
    }
    result.sort_by_key(|item| {
        (
            item.page_index,
            item.id.object_number,
            item.id.generation_number,
        )
    });
    Ok(result)
}

fn parse_highlight(id: HighlightId, page_index: u32, dict: &PdfDictionary) -> Result<Highlight> {
    let rect_values = numeric_array(dict, "Rect", Some(4))?;
    let rect = [
        rect_values[0],
        rect_values[1],
        rect_values[2],
        rect_values[3],
    ];
    if rect[2] <= rect[0] || rect[3] <= rect[1] {
        return Err(invalid(
            "/Highlight /Rect must have positive width and height",
        ));
    }
    let values = numeric_array(dict, "QuadPoints", None)?;
    if values.is_empty() || values.len() % 8 != 0 {
        return Err(invalid(
            "/Highlight /QuadPoints must contain one or more groups of eight numbers",
        ));
    }
    let quadrilaterals = values
        .chunks_exact(8)
        .map(|v| {
            HighlightQuad::new([
                Point::new(v[0], v[1]),
                Point::new(v[2], v[3]),
                Point::new(v[4], v[5]),
                Point::new(v[6], v[7]),
            ])
        })
        .collect::<Result<Vec<_>>>()?;
    let color_values = numeric_array(dict, "C", Some(3))?;
    let color = HighlightColor::new([color_values[0], color_values[1], color_values[2]])?;
    let opacity = match dict.get("CA") {
        None => None,
        Some(value) => {
            Some(HighlightOpacity::new(number(value).ok_or_else(|| {
                invalid("/Highlight /CA must be a finite number")
            })?)?)
        }
    };
    let contents = match dict.get("Contents") {
        None => None,
        Some(value) => Some(
            value
                .as_string()
                .ok_or_else(|| invalid("/Highlight /Contents must be a string"))?
                .to_text(),
        ),
    };
    Ok(Highlight {
        id,
        page_index,
        rect,
        quadrilaterals,
        color,
        opacity,
        contents,
    })
}

fn validate_batch(
    snapshot: &AnnotationSnapshot,
    existing: &[Highlight],
    mutations: &[HighlightMutation],
) -> Result<HashMap<HighlightId, u32>> {
    let existing_pages: HashMap<_, _> = existing
        .iter()
        .map(|item| (item.id, item.page_index))
        .collect();
    let mut targeted = HashSet::new();
    for mutation in mutations {
        match mutation {
            HighlightMutation::Add {
                page_index,
                quadrilaterals,
                ..
            } => {
                let page = snapshot
                    .pages
                    .get(*page_index as usize)
                    .ok_or_else(|| invalid(&format!("page {page_index} does not exist")))?;
                validate_quads(quadrilaterals, page.bounds)?;
            }
            HighlightMutation::Remove { id } => {
                if !targeted.insert(*id) {
                    return Err(invalid("highlight is targeted more than once"));
                }
                if !existing_pages.contains_key(id) {
                    return Err(invalid(&format!(
                        "highlight {} {} does not exist",
                        id.object_number, id.generation_number
                    )));
                }
            }
        }
    }
    Ok(existing_pages)
}

fn validate_quads(quads: &[HighlightQuad], bounds: [f64; 4]) -> Result<()> {
    if quads.is_empty() {
        return Err(invalid("highlight must contain at least one quadrilateral"));
    }
    for point in quads.iter().flat_map(|quad| quad.points()) {
        if point.x < bounds[0] || point.x > bounds[2] || point.y < bounds[1] || point.y > bounds[3]
        {
            return Err(invalid("highlight coordinates are outside the page bounds"));
        }
    }
    Ok(())
}

fn new_dictionary(
    quads: &[HighlightQuad],
    color: HighlightColor,
    opacity: Option<HighlightOpacity>,
    contents: Option<&str>,
) -> PdfDictionary {
    let mut dict = PdfDictionary::new();
    dict.insert(
        "Type".to_string(),
        PdfObject::Name(PdfName("Annot".to_string())),
    );
    dict.insert(
        "Subtype".to_string(),
        PdfObject::Name(PdfName("Highlight".to_string())),
    );
    let rect = quad_bounds(quads);
    dict.insert("Rect".to_string(), numbers(&rect));
    let flat: Vec<f64> = quads
        .iter()
        .flat_map(|quad| quad.points().iter().flat_map(|p| [p.x, p.y]))
        .collect();
    dict.insert("QuadPoints".to_string(), numbers(&flat));
    dict.insert("C".to_string(), numbers(&color.components()));
    if let Some(value) = opacity {
        dict.insert("CA".to_string(), PdfObject::Real(value.value()));
    }
    if let Some(value) = contents {
        dict.insert("Contents".to_string(), pdf_text(value));
    }
    dict
}

fn quad_bounds(quads: &[HighlightQuad]) -> [f64; 4] {
    let mut left = f64::INFINITY;
    let mut bottom = f64::INFINITY;
    let mut right = f64::NEG_INFINITY;
    let mut top = f64::NEG_INFINITY;
    for point in quads.iter().flat_map(|quad| quad.points()) {
        left = left.min(point.x);
        bottom = bottom.min(point.y);
        right = right.max(point.x);
        top = top.max(point.y);
    }
    [left, bottom, right, top]
}

fn validate_quad_shape(points: &[Point; 4]) -> Result<()> {
    if points
        .iter()
        .any(|point| !point.x.is_finite() || !point.y.is_finite())
    {
        return Err(invalid("highlight coordinates must be finite"));
    }
    let boundary = [points[0], points[1], points[3], points[2]];
    let mut sign = 0.0;
    for index in 0..4 {
        let a = boundary[index];
        let b = boundary[(index + 1) % 4];
        let c = boundary[(index + 2) % 4];
        let cross = (b.x - a.x) * (c.y - b.y) - (b.y - a.y) * (c.x - b.x);
        if cross.abs() <= f64::EPSILON {
            return Err(invalid(
                "highlight quadrilateral must have four non-collinear corners",
            ));
        }
        if sign == 0.0 {
            sign = cross.signum();
        } else if cross.signum() != sign {
            return Err(invalid(
                "highlight quadrilateral must be convex and not crossed",
            ));
        }
    }
    Ok(())
}

fn numeric_array(dict: &PdfDictionary, key: &str, length: Option<usize>) -> Result<Vec<f64>> {
    let array = dict
        .get(key)
        .and_then(PdfObject::as_array)
        .ok_or_else(|| invalid(&format!("/Highlight /{key} is missing or is not an array")))?;
    if length.is_some_and(|expected| array.0.len() != expected) {
        return Err(invalid(&format!(
            "/Highlight /{key} has the wrong number of values"
        )));
    }
    array
        .0
        .iter()
        .map(|value| {
            number(value).ok_or_else(|| {
                invalid(&format!(
                    "/Highlight /{key} contains a non-finite or non-numeric value"
                ))
            })
        })
        .collect()
}

fn number(value: &PdfObject) -> Option<f64> {
    match value {
        PdfObject::Integer(v) => Some(*v as f64),
        PdfObject::Real(v) if v.is_finite() => Some(*v),
        _ => None,
    }
}

fn validate_unit_values(values: &[f64], label: &str) -> Result<()> {
    if values
        .iter()
        .any(|value| !value.is_finite() || !(0.0..=1.0).contains(value))
    {
        return Err(invalid(&format!(
            "{label} components must be finite and between 0 and 1"
        )));
    }
    Ok(())
}

fn numbers(values: &[f64]) -> PdfObject {
    PdfObject::Array(PdfArray(
        values.iter().copied().map(PdfObject::Real).collect(),
    ))
}

fn pdf_text(contents: &str) -> PdfObject {
    let mut bytes = vec![0xFE, 0xFF];
    for unit in contents.encode_utf16() {
        bytes.extend_from_slice(&unit.to_be_bytes());
    }
    PdfObject::String(PdfString(bytes))
}

fn invalid(message: &str) -> PdfError {
    PdfError::InvalidStructure(message.to_string())
}
