//! Incremental editing of standard `/Ink` annotations in existing PDFs.

use super::incremental_annotations::{subtype, AnnotationContainer, AnnotationSnapshot};
use super::incremental_update::IncrementalUpdate;
use crate::error::{PdfError, Result};
use crate::geometry::Point;
use crate::parser::objects::{PdfArray, PdfDictionary, PdfName, PdfObject, PdfStream};
use crate::parser::PdfReader;
use crate::signatures::{ensure_modification_allowed, IncrementalModification};
use std::collections::{HashMap, HashSet};
use std::io::Cursor;

const MAX_INK_STROKES: usize = 4_096;
const MAX_INK_POINTS: usize = 1_000_000;
const MAX_INK_APPEARANCE_BYTES: usize = 64 * 1024 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
/// Stable indirect-object identity of an Ink annotation.
pub struct InkId {
    /// PDF object number.
    pub object_number: u32,
    /// PDF generation number.
    pub generation_number: u16,
}

impl InkId {
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

#[derive(Debug, Clone, PartialEq)]
/// A non-empty, finite sequence of points forming one ink stroke.
pub struct InkStroke(Vec<Point>);

impl InkStroke {
    /// Validate and construct a stroke.
    ///
    /// # Errors
    ///
    /// Returns an error when no points are supplied or a coordinate is non-finite.
    pub fn new(points: Vec<Point>) -> Result<Self> {
        if points.is_empty() {
            return Err(invalid("ink stroke must contain at least one point"));
        }
        if points
            .iter()
            .any(|point| !point.x.is_finite() || !point.y.is_finite())
        {
            return Err(invalid("ink stroke coordinates must be finite"));
        }
        Ok(Self(points))
    }
    /// Return the stroke's points in drawing order.
    pub fn points(&self) -> &[Point] {
        &self.0
    }
}

/// A validated PDF annotation color.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum InkColor {
    /// DeviceGray color with one component.
    Gray([f64; 1]),
    /// DeviceRGB color with three components.
    Rgb([f64; 3]),
    /// DeviceCMYK color with four components.
    Cmyk([f64; 4]),
}

impl InkColor {
    /// Construct a DeviceRGB color whose components are in `0..=1`.
    ///
    /// # Errors
    ///
    /// Returns an error for non-finite or out-of-range components.
    pub fn new(components: [f64; 3]) -> Result<Self> {
        validate_unit(&components, "ink RGB color")?;
        Ok(Self::Rgb(components))
    }

    /// Construct a DeviceGray color.
    ///
    /// # Errors
    ///
    /// Returns an error for a non-finite or out-of-range component.
    pub fn gray(component: f64) -> Result<Self> {
        validate_unit(&[component], "ink grayscale color")?;
        Ok(Self::Gray([component]))
    }

    /// Construct a DeviceCMYK color.
    ///
    /// # Errors
    ///
    /// Returns an error for non-finite or out-of-range components.
    pub fn cmyk(components: [f64; 4]) -> Result<Self> {
        validate_unit(&components, "ink CMYK color")?;
        Ok(Self::Cmyk(components))
    }

    fn components(self) -> Vec<f64> {
        match self {
            Self::Gray(values) => values.to_vec(),
            Self::Rgb(values) => values.to_vec(),
            Self::Cmyk(values) => values.to_vec(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
/// A validated positive stroke width in PDF user-space units.
pub struct InkWidth(f64);

impl InkWidth {
    /// Construct a finite, positive stroke width.
    ///
    /// # Errors
    ///
    /// Returns an error when `value` is non-finite or not positive.
    pub fn new(value: f64) -> Result<Self> {
        if !value.is_finite() || value <= 0.0 {
            return Err(invalid("ink width must be finite and positive"));
        }
        Ok(Self(value))
    }
    /// Return the stroke width in user-space units.
    pub const fn value(self) -> f64 {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
/// A validated constant opacity in the inclusive range `0..=1`.
pub struct InkOpacity(f64);

impl InkOpacity {
    /// Construct an opacity.
    ///
    /// # Errors
    ///
    /// Returns an error when `value` is non-finite or outside `0..=1`.
    pub fn new(value: f64) -> Result<Self> {
        validate_unit(&[value], "ink opacity")?;
        Ok(Self(value))
    }
    /// Return the opacity value.
    pub const fn value(self) -> f64 {
        self.0
    }
}

#[derive(Debug, Clone, PartialEq)]
/// A standard PDF `/Ink` annotation read from an existing document.
pub struct Ink {
    /// Stable indirect-object identity.
    pub id: InkId,
    /// Zero-based page index.
    pub page_index: u32,
    /// Bounding rectangle including half the stroke width.
    pub rect: [f64; 4],
    /// One or more strokes in page coordinates.
    pub strokes: Vec<InkStroke>,
    /// DeviceGray, DeviceRGB, or DeviceCMYK stroke color.
    pub color: InkColor,
    /// Stroke width in PDF user-space units.
    pub width: InkWidth,
    /// Optional constant opacity.
    pub opacity: Option<InkOpacity>,
}

#[derive(Debug, Clone, PartialEq)]
/// A requested change to the document's Ink annotations.
pub enum InkMutation {
    /// Add an Ink annotation to a page.
    Add {
        /// Zero-based target page.
        page_index: u32,
        /// Non-empty validated strokes.
        strokes: Vec<InkStroke>,
        /// Stroke color.
        color: InkColor,
        /// Positive stroke width.
        width: InkWidth,
        /// Optional constant opacity.
        opacity: Option<InkOpacity>,
    },
    /// Update an existing annotation while preserving unrelated keys.
    Update {
        /// Existing Ink annotation identity.
        id: InkId,
        /// Replacement strokes.
        strokes: Vec<InkStroke>,
        /// Replacement color.
        color: InkColor,
        /// Replacement stroke width.
        width: InkWidth,
        /// Replacement optional opacity.
        opacity: Option<InkOpacity>,
    },
    /// Remove an existing annotation reference from its page.
    Remove {
        /// Existing Ink annotation identity.
        id: InkId,
    },
}

#[derive(Debug, Clone)]
/// Result of one atomic incremental Ink update.
pub struct InkUpdate {
    /// Complete PDF bytes; the original document is an exact prefix.
    pub pdf_bytes: Vec<u8>,
    /// Complete current Ink annotation set after the update.
    pub annotations: Vec<Ink>,
}

/// Reads and atomically edits standard Ink annotations in an existing PDF.
pub struct IncrementalInkEditor<'a> {
    base_bytes: &'a [u8],
}

impl<'a> IncrementalInkEditor<'a> {
    /// Create an editor over an existing PDF's bytes.
    pub fn new(base_bytes: &'a [u8]) -> Self {
        Self { base_bytes }
    }

    /// Read every indirect `/Ink` annotation with its stable identity.
    ///
    /// # Errors
    ///
    /// Returns an error for malformed, encrypted, inline, or resource-excessive input.
    pub fn annotations(&self) -> Result<Vec<Ink>> {
        let snapshot = AnnotationSnapshot::parse(self.base_bytes, "Ink", "ink annotation")?;
        read_inks(&snapshot)
    }

    /// Validate and apply all mutations as exactly one incremental revision.
    ///
    /// # Errors
    ///
    /// Returns an error for invalid geometry, pages, identities, conflicting
    /// mutations, encryption, forbidden signature policies, or resource limits.
    pub fn apply(&self, mutations: &[InkMutation]) -> Result<InkUpdate> {
        let mut snapshot = AnnotationSnapshot::parse(self.base_bytes, "Ink", "ink annotation")?;
        let existing = read_inks(&snapshot)?;
        if mutations.is_empty() {
            return Ok(InkUpdate {
                pdf_bytes: self.base_bytes.to_vec(),
                annotations: existing,
            });
        }
        let pages = validate_batch(&snapshot, &existing, mutations)?;
        validate_policy(self.base_bytes)?;
        let mut update = IncrementalUpdate::from_base(self.base_bytes)?;
        let mut changed_pages = HashSet::new();
        let mut rewritten = HashMap::new();

        for mutation in mutations {
            match mutation {
                InkMutation::Add {
                    page_index,
                    strokes,
                    color,
                    width,
                    opacity,
                } => {
                    let id = update.allocate_id()?;
                    let appearance_id = update.allocate_id()?;
                    let rect = bounds(strokes, *width);
                    update.replace(
                        appearance_id,
                        PdfObject::Stream(appearance(strokes, *color, *width, *opacity, rect)?),
                    )?;
                    update.replace(
                        id,
                        PdfObject::Dictionary(new_dictionary(
                            strokes,
                            *color,
                            *width,
                            *opacity,
                            rect,
                            appearance_id,
                        )),
                    )?;
                    snapshot.pages[*page_index as usize]
                        .annotations
                        .push(PdfObject::Reference(id.0, id.1));
                    changed_pages.insert(*page_index);
                }
                InkMutation::Update {
                    id,
                    strokes,
                    color,
                    width,
                    opacity,
                } => {
                    let state = snapshot
                        .annotations
                        .get(&id.tuple())
                        .ok_or_else(|| missing(*id))?;
                    if subtype(&state.dictionary) != Some("Ink") {
                        return Err(missing(*id));
                    }
                    let mut dictionary = state.dictionary.clone();
                    let appearance_id = update.allocate_id()?;
                    let rect = bounds(strokes, *width);
                    update.replace(
                        appearance_id,
                        PdfObject::Stream(appearance(strokes, *color, *width, *opacity, rect)?),
                    )?;
                    set_properties(
                        &mut dictionary,
                        strokes,
                        *color,
                        *width,
                        *opacity,
                        rect,
                        appearance_id,
                    );
                    rewritten.insert(id.tuple(), dictionary);
                }
                InkMutation::Remove { id } => {
                    let page_index = *pages.get(id).ok_or_else(|| missing(*id))?;
                    snapshot.pages[page_index as usize]
                        .annotations
                        .retain(|value| value.as_reference() != Some(id.tuple()));
                    changed_pages.insert(page_index);
                }
            }
        }
        for (id, dictionary) in rewritten {
            update.replace(id, PdfObject::Dictionary(dictionary))?;
        }
        rewrite_pages(&mut update, &mut snapshot, &changed_pages)?;
        let pdf_bytes = update.finish()?;
        let annotations = IncrementalInkEditor::new(&pdf_bytes).annotations()?;
        Ok(InkUpdate {
            pdf_bytes,
            annotations,
        })
    }
}

fn read_inks(snapshot: &AnnotationSnapshot) -> Result<Vec<Ink>> {
    let mut result = Vec::new();
    for (&(number, generation), state) in &snapshot.annotations {
        if subtype(&state.dictionary) == Some("Ink") {
            let page = &snapshot.pages[state.page_index as usize];
            result.push(parse_ink(
                InkId::new(number, generation),
                state.page_index,
                &state.dictionary,
                page.bounds,
            )?);
        }
    }
    result.sort_by_key(|ink| {
        (
            ink.page_index,
            ink.id.object_number,
            ink.id.generation_number,
        )
    });
    Ok(result)
}

fn parse_ink(
    id: InkId,
    page_index: u32,
    dict: &PdfDictionary,
    page_bounds: [f64; 4],
) -> Result<Ink> {
    let rect_values = numeric_array(dict, "Rect", Some(4))?;
    let rect = [
        rect_values[0],
        rect_values[1],
        rect_values[2],
        rect_values[3],
    ];
    if rect[0] >= rect[2] || rect[1] >= rect[3] {
        return Err(invalid("/Ink /Rect must have positive width and height"));
    }
    if rect[0] < page_bounds[0]
        || rect[1] < page_bounds[1]
        || rect[2] > page_bounds[2]
        || rect[3] > page_bounds[3]
    {
        return Err(invalid("/Ink /Rect is outside the page bounds"));
    }
    let lists = dict
        .get("InkList")
        .and_then(PdfObject::as_array)
        .ok_or_else(|| invalid("/Ink /InkList is missing or is not an array"))?;
    if lists.0.is_empty() {
        return Err(invalid("/Ink /InkList must contain at least one stroke"));
    }
    if lists.0.len() > MAX_INK_STROKES {
        return Err(invalid("/Ink /InkList exceeds the stroke limit"));
    }
    let mut point_count = 0usize;
    let strokes = lists
        .0
        .iter()
        .map(|value| {
            let array = value
                .as_array()
                .ok_or_else(|| invalid("/Ink /InkList stroke is not an array"))?;
            if array.0.len() < 2 || array.0.len() % 2 != 0 {
                return Err(invalid(
                    "/Ink /InkList stroke must contain coordinate pairs",
                ));
            }
            point_count = point_count
                .checked_add(array.0.len() / 2)
                .ok_or_else(|| invalid("/Ink /InkList point count overflows"))?;
            if point_count > MAX_INK_POINTS {
                return Err(invalid("/Ink /InkList exceeds the point limit"));
            }
            let values = array
                .0
                .iter()
                .map(|value| {
                    number(value).ok_or_else(|| {
                        invalid("/Ink /InkList contains a non-finite or non-numeric value")
                    })
                })
                .collect::<Result<Vec<_>>>()?;
            InkStroke::new(
                values
                    .chunks_exact(2)
                    .map(|p| Point::new(p[0], p[1]))
                    .collect(),
            )
        })
        .collect::<Result<Vec<_>>>()?;
    for point in strokes.iter().flat_map(InkStroke::points) {
        if point.x < page_bounds[0]
            || point.x > page_bounds[2]
            || point.y < page_bounds[1]
            || point.y > page_bounds[3]
        {
            return Err(invalid(
                "/Ink /InkList coordinates are outside the page bounds",
            ));
        }
    }
    let color = match dict.get("C") {
        None => InkColor::Gray([0.0]),
        Some(_) => {
            let colors = numeric_array(dict, "C", None)?;
            match colors.as_slice() {
                [gray] => InkColor::gray(*gray)?,
                [red, green, blue] => InkColor::new([*red, *green, *blue])?,
                [cyan, magenta, yellow, black] => {
                    InkColor::cmyk([*cyan, *magenta, *yellow, *black])?
                }
                _ => return Err(invalid("/Ink /C must have one, three, or four components")),
            }
        }
    };
    let width_value = match dict.get("BS") {
        Some(PdfObject::Dictionary(bs)) => match bs.get("W") {
            None => 1.0,
            Some(value) => {
                number(value).ok_or_else(|| invalid("/Ink /BS /W must be a finite number"))?
            }
        },
        Some(_) => return Err(invalid("/Ink /BS must be a dictionary")),
        None => border_width(dict)?.unwrap_or(1.0),
    };
    let width = InkWidth::new(width_value)?;
    let opacity = dict
        .get("CA")
        .map(|value| {
            number(value)
                .ok_or_else(|| invalid("/Ink /CA must be a finite number"))
                .and_then(InkOpacity::new)
        })
        .transpose()?;
    Ok(Ink {
        id,
        page_index,
        rect,
        strokes,
        color,
        width,
        opacity,
    })
}

fn validate_batch(
    snapshot: &AnnotationSnapshot,
    existing: &[Ink],
    mutations: &[InkMutation],
) -> Result<HashMap<InkId, u32>> {
    let pages: HashMap<_, _> = existing
        .iter()
        .map(|ink| (ink.id, ink.page_index))
        .collect();
    let mut targeted = HashSet::new();
    for mutation in mutations {
        match mutation {
            InkMutation::Add {
                page_index,
                strokes,
                width,
                ..
            } => {
                let page = snapshot
                    .pages
                    .get(*page_index as usize)
                    .ok_or_else(|| invalid(&format!("page {page_index} does not exist")))?;
                validate_geometry(strokes, *width, page.bounds)?;
            }
            InkMutation::Update {
                id, strokes, width, ..
            } => {
                if !targeted.insert(*id) {
                    return Err(invalid("ink annotation is targeted more than once"));
                }
                let page_index = *pages.get(id).ok_or_else(|| missing(*id))?;
                validate_geometry(strokes, *width, snapshot.pages[page_index as usize].bounds)?;
            }
            InkMutation::Remove { id } => {
                if !targeted.insert(*id) {
                    return Err(invalid("ink annotation is targeted more than once"));
                }
                if !pages.contains_key(id) {
                    return Err(missing(*id));
                }
            }
        }
    }
    Ok(pages)
}

fn validate_geometry(strokes: &[InkStroke], width: InkWidth, page: [f64; 4]) -> Result<()> {
    if strokes.is_empty() {
        return Err(invalid("ink annotation must contain at least one stroke"));
    }
    if strokes.len() > MAX_INK_STROKES {
        return Err(invalid("ink annotation exceeds the stroke limit"));
    }
    let point_count = strokes.iter().try_fold(0usize, |count, stroke| {
        count
            .checked_add(stroke.points().len())
            .ok_or_else(|| invalid("ink point count overflows"))
    })?;
    if point_count > MAX_INK_POINTS {
        return Err(invalid("ink annotation exceeds the point limit"));
    }
    let rect = bounds(strokes, width);
    if rect[0] < page[0] || rect[1] < page[1] || rect[2] > page[2] || rect[3] > page[3] {
        return Err(invalid(
            "ink stroke and width extend outside the page bounds",
        ));
    }
    Ok(())
}

fn bounds(strokes: &[InkStroke], width: InkWidth) -> [f64; 4] {
    let half = width.value() / 2.0;
    let mut rect = [
        f64::INFINITY,
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::NEG_INFINITY,
    ];
    for point in strokes.iter().flat_map(InkStroke::points) {
        rect[0] = rect[0].min(point.x - half);
        rect[1] = rect[1].min(point.y - half);
        rect[2] = rect[2].max(point.x + half);
        rect[3] = rect[3].max(point.y + half);
    }
    rect
}

fn new_dictionary(
    strokes: &[InkStroke],
    color: InkColor,
    width: InkWidth,
    opacity: Option<InkOpacity>,
    rect: [f64; 4],
    appearance: (u32, u16),
) -> PdfDictionary {
    let mut dict = PdfDictionary::new();
    dict.insert("Type".into(), name("Annot"));
    dict.insert("Subtype".into(), name("Ink"));
    set_properties(&mut dict, strokes, color, width, opacity, rect, appearance);
    dict
}

fn set_properties(
    dict: &mut PdfDictionary,
    strokes: &[InkStroke],
    color: InkColor,
    width: InkWidth,
    opacity: Option<InkOpacity>,
    rect: [f64; 4],
    appearance: (u32, u16),
) {
    dict.insert("Rect".into(), numbers(&rect));
    dict.insert(
        "InkList".into(),
        PdfObject::Array(PdfArray(
            strokes
                .iter()
                .map(|stroke| {
                    numbers(
                        &stroke
                            .points()
                            .iter()
                            .flat_map(|p| [p.x, p.y])
                            .collect::<Vec<_>>(),
                    )
                })
                .collect(),
        )),
    );
    dict.insert("C".into(), numbers(&color.components()));
    let mut bs = match dict.get("BS") {
        Some(PdfObject::Dictionary(existing)) => existing.clone(),
        _ => PdfDictionary::new(),
    };
    bs.insert("W".into(), PdfObject::Real(width.value()));
    dict.insert("BS".into(), PdfObject::Dictionary(bs));
    match opacity {
        Some(value) => {
            dict.insert("CA".into(), PdfObject::Real(value.value()));
        }
        None => {
            dict.0.remove(&PdfName("CA".into()));
        }
    }
    let mut ap = match dict.get("AP") {
        Some(PdfObject::Dictionary(existing)) => existing.clone(),
        _ => PdfDictionary::new(),
    };
    ap.insert("N".into(), PdfObject::Reference(appearance.0, appearance.1));
    dict.insert("AP".into(), PdfObject::Dictionary(ap));
}

fn appearance(
    strokes: &[InkStroke],
    color: InkColor,
    width: InkWidth,
    opacity: Option<InkOpacity>,
    rect: [f64; 4],
) -> Result<PdfStream> {
    let color_operator = match color {
        InkColor::Gray([gray]) => format!("{} G", pdf_number(gray)),
        InkColor::Rgb([red, green, blue]) => format!(
            "{} {} {} RG",
            pdf_number(red),
            pdf_number(green),
            pdf_number(blue)
        ),
        InkColor::Cmyk([cyan, magenta, yellow, black]) => format!(
            "{} {} {} {} K",
            pdf_number(cyan),
            pdf_number(magenta),
            pdf_number(yellow),
            pdf_number(black)
        ),
    };
    let mut data = format!(
        "q\n{color_operator}\n{} w\n1 J\n1 j\n",
        pdf_number(width.value())
    );
    if let Some(value) = opacity {
        data.push_str("/GS0 gs\n");
        let _ = value;
    }
    for stroke in strokes {
        let points = stroke.points();
        data.push_str(&format!(
            "{} {} m\n",
            pdf_number(points[0].x - rect[0]),
            pdf_number(points[0].y - rect[1])
        ));
        for point in &points[1..] {
            data.push_str(&format!(
                "{} {} l\n",
                pdf_number(point.x - rect[0]),
                pdf_number(point.y - rect[1])
            ));
            if data.len() > MAX_INK_APPEARANCE_BYTES {
                return Err(invalid("ink appearance exceeds the byte limit"));
            }
        }
        if points.len() == 1 {
            data.push_str(&format!(
                "{} {} l\n",
                pdf_number(points[0].x - rect[0]),
                pdf_number(points[0].y - rect[1])
            ));
        }
        data.push_str("S\n");
        if data.len() > MAX_INK_APPEARANCE_BYTES {
            return Err(invalid("ink appearance exceeds the byte limit"));
        }
    }
    data.push_str("Q");
    let mut dict = PdfDictionary::new();
    dict.insert("Type".into(), name("XObject"));
    dict.insert("Subtype".into(), name("Form"));
    dict.insert("FormType".into(), PdfObject::Integer(1));
    dict.insert(
        "BBox".into(),
        numbers(&[0.0, 0.0, rect[2] - rect[0], rect[3] - rect[1]]),
    );
    if let Some(value) = opacity {
        let mut gs = PdfDictionary::new();
        gs.insert("CA".into(), PdfObject::Real(value.value()));
        gs.insert("ca".into(), PdfObject::Real(value.value()));
        let mut resources = PdfDictionary::new();
        resources.insert("GS0".into(), PdfObject::Dictionary(gs));
        let mut root = PdfDictionary::new();
        root.insert("ExtGState".into(), PdfObject::Dictionary(resources));
        dict.insert("Resources".into(), PdfObject::Dictionary(root));
    }
    Ok(PdfStream {
        dict,
        data: data.into_bytes(),
    })
}

fn rewrite_pages(
    update: &mut IncrementalUpdate,
    snapshot: &mut AnnotationSnapshot,
    changed: &HashSet<u32>,
) -> Result<()> {
    for page_index in changed {
        let page = &mut snapshot.pages[*page_index as usize];
        match page.container {
            AnnotationContainer::Page => {
                page.dictionary.insert(
                    "Annots".into(),
                    PdfObject::Array(PdfArray(page.annotations.clone())),
                );
                update.replace(
                    page.reference,
                    PdfObject::Dictionary(page.dictionary.clone()),
                )?;
            }
            AnnotationContainer::Indirect(id) => {
                update.replace(id, PdfObject::Array(PdfArray(page.annotations.clone())))?
            }
        }
    }
    Ok(())
}

fn validate_policy(bytes: &[u8]) -> Result<()> {
    let mut reader =
        PdfReader::new(Cursor::new(bytes)).map_err(|e| invalid(&format!("parse base PDF: {e}")))?;
    let catalog = reader
        .catalog()
        .map_err(|e| invalid(&format!("read catalog: {e}")))?
        .clone();
    ensure_modification_allowed(
        &mut reader,
        &catalog,
        IncrementalModification::AddAnnotation,
    )
}

fn numeric_array(dict: &PdfDictionary, key: &str, length: Option<usize>) -> Result<Vec<f64>> {
    let array = dict
        .get(key)
        .and_then(PdfObject::as_array)
        .ok_or_else(|| invalid(&format!("/Ink /{key} is missing or is not an array")))?;
    if length.is_some_and(|len| array.0.len() != len) {
        return Err(invalid(&format!(
            "/Ink /{key} has the wrong number of values"
        )));
    }
    array
        .0
        .iter()
        .map(|value| {
            number(value).ok_or_else(|| {
                invalid(&format!(
                    "/Ink /{key} contains a non-finite or non-numeric value"
                ))
            })
        })
        .collect()
}

fn border_width(dict: &PdfDictionary) -> Result<Option<f64>> {
    let Some(value) = dict.get("Border") else {
        return Ok(None);
    };
    let array = value
        .as_array()
        .ok_or_else(|| invalid("/Ink /Border must be an array"))?;
    if array.0.len() < 3 {
        return Err(invalid("/Ink /Border must contain at least three values"));
    }
    number(&array.0[2])
        .ok_or_else(|| invalid("/Ink /Border width must be a finite number"))
        .map(Some)
}

fn number(value: &PdfObject) -> Option<f64> {
    match value {
        PdfObject::Integer(v) => Some(*v as f64),
        PdfObject::Real(v) if v.is_finite() => Some(*v),
        _ => None,
    }
}
fn numbers(values: &[f64]) -> PdfObject {
    PdfObject::Array(PdfArray(
        values.iter().copied().map(PdfObject::Real).collect(),
    ))
}
fn name(value: &str) -> PdfObject {
    PdfObject::Name(PdfName(value.into()))
}
fn validate_unit(values: &[f64], label: &str) -> Result<()> {
    if values
        .iter()
        .any(|v| !v.is_finite() || !(0.0..=1.0).contains(v))
    {
        Err(invalid(&format!(
            "{label} components must be finite and between 0 and 1"
        )))
    } else {
        Ok(())
    }
}
fn missing(id: InkId) -> PdfError {
    invalid(&format!(
        "ink annotation {} {} does not exist",
        id.object_number, id.generation_number
    ))
}
fn invalid(message: &str) -> PdfError {
    PdfError::InvalidStructure(message.into())
}
fn pdf_number(value: f64) -> String {
    let mut s = format!("{value:.6}");
    while s.contains('.') && s.ends_with('0') {
        s.pop();
    }
    if s.ends_with('.') {
        s.pop();
    }
    if s == "-0" {
        "0".into()
    } else {
        s
    }
}
