//! Incremental editing of standard geometric annotations in existing PDFs.

use super::incremental_annotations::{subtype, AnnotationContainer, AnnotationSnapshot};
use super::incremental_update::IncrementalUpdate;
use crate::error::{PdfError, Result};
use crate::geometry::Point;
use crate::parser::objects::{PdfArray, PdfDictionary, PdfName, PdfObject, PdfStream};
use crate::parser::PdfReader;
use crate::signatures::{ensure_modification_allowed, IncrementalModification};
use std::collections::{HashMap, HashSet};
use std::io::Cursor;

const MAX_VERTICES: usize = 100_000;
const MAX_APPEARANCE_BYTES: usize = 64 * 1024 * 1024;

/// Stable indirect-object identity of a geometric annotation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct GeometricId {
    /// PDF object number.
    pub object_number: u32,
    /// PDF generation number.
    pub generation_number: u16,
}

impl GeometricId {
    /// Construct an identity from an indirect reference.
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

/// Device color used by geometric annotations.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum GeometricColor {
    /// DeviceGray.
    Gray([f64; 1]),
    /// DeviceRGB.
    Rgb([f64; 3]),
    /// DeviceCMYK.
    Cmyk([f64; 4]),
}

impl GeometricColor {
    /// Construct a DeviceGray color.
    pub fn gray(value: f64) -> Result<Self> {
        validate_unit(&[value], "gray color")?;
        Ok(Self::Gray([value]))
    }
    /// Construct a DeviceRGB color.
    pub fn rgb(values: [f64; 3]) -> Result<Self> {
        validate_unit(&values, "RGB color")?;
        Ok(Self::Rgb(values))
    }
    /// Construct a DeviceCMYK color.
    pub fn cmyk(values: [f64; 4]) -> Result<Self> {
        validate_unit(&values, "CMYK color")?;
        Ok(Self::Cmyk(values))
    }
    fn values(self) -> Vec<f64> {
        match self {
            Self::Gray(v) => v.to_vec(),
            Self::Rgb(v) => v.to_vec(),
            Self::Cmyk(v) => v.to_vec(),
        }
    }
    fn stroke_operator(self) -> String {
        color_operator(self, true)
    }
    fn fill_operator(self) -> String {
        color_operator(self, false)
    }
}

/// Validated positive line width in user-space units.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GeometricWidth(f64);
impl GeometricWidth {
    /// Construct a positive finite width.
    pub fn new(value: f64) -> Result<Self> {
        if !value.is_finite() || value <= 0.0 {
            Err(invalid("geometric width must be finite and positive"))
        } else {
            Ok(Self(value))
        }
    }
    /// Return the width.
    pub const fn value(self) -> f64 {
        self.0
    }
}

/// Validated constant opacity.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GeometricOpacity(f64);
impl GeometricOpacity {
    /// Construct an opacity in `0..=1`.
    pub fn new(value: f64) -> Result<Self> {
        validate_unit(&[value], "geometric opacity")?;
        Ok(Self(value))
    }
    /// Return the opacity.
    pub const fn value(self) -> f64 {
        self.0
    }
}

/// Validated PDF dash pattern.
#[derive(Debug, Clone, PartialEq)]
pub struct GeometricDashPattern(Vec<f64>);
impl GeometricDashPattern {
    /// Construct a non-empty finite pattern containing non-negative lengths and at least one positive length.
    pub fn new(values: Vec<f64>) -> Result<Self> {
        if values.is_empty()
            || values.len() > 256
            || values.iter().any(|v| !v.is_finite() || *v < 0.0)
            || values.iter().all(|v| *v == 0.0)
        {
            return Err(invalid(
                "dash pattern must be non-empty, finite, non-negative, bounded, and not all zero",
            ));
        }
        Ok(Self(values))
    }
    /// Return dash lengths.
    pub fn values(&self) -> &[f64] {
        &self.0
    }
}

/// Standard PDF line-ending style.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum LineEnding {
    /// No decoration.
    #[default]
    None,
    /// Square.
    Square,
    /// Circle.
    Circle,
    /// Diamond.
    Diamond,
    /// Open arrow.
    OpenArrow,
    /// Closed arrow.
    ClosedArrow,
    /// Butt.
    Butt,
    /// Reverse open arrow.
    ROpenArrow,
    /// Reverse closed arrow.
    RClosedArrow,
    /// Slash.
    Slash,
}
impl LineEnding {
    fn parse(value: &str) -> Result<Self> {
        match value {
            "None" => Ok(Self::None),
            "Square" => Ok(Self::Square),
            "Circle" => Ok(Self::Circle),
            "Diamond" => Ok(Self::Diamond),
            "OpenArrow" => Ok(Self::OpenArrow),
            "ClosedArrow" => Ok(Self::ClosedArrow),
            "Butt" => Ok(Self::Butt),
            "ROpenArrow" => Ok(Self::ROpenArrow),
            "RClosedArrow" => Ok(Self::RClosedArrow),
            "Slash" => Ok(Self::Slash),
            _ => Err(invalid("unsupported geometric line ending")),
        }
    }
    fn name(self) -> &'static str {
        match self {
            Self::None => "None",
            Self::Square => "Square",
            Self::Circle => "Circle",
            Self::Diamond => "Diamond",
            Self::OpenArrow => "OpenArrow",
            Self::ClosedArrow => "ClosedArrow",
            Self::Butt => "Butt",
            Self::ROpenArrow => "ROpenArrow",
            Self::RClosedArrow => "RClosedArrow",
            Self::Slash => "Slash",
        }
    }
}

/// Typed geometry for a standard geometric annotation.
#[derive(Debug, Clone, PartialEq)]
pub enum GeometricGeometry {
    /// `/Line` geometry.
    Line {
        start: Point,
        end: Point,
        start_ending: LineEnding,
        end_ending: LineEnding,
    },
    /// `/Square` bounds.
    Square { rect: [f64; 4] },
    /// `/Circle` bounds.
    Circle { rect: [f64; 4] },
    /// Closed `/Polygon` vertices.
    Polygon { vertices: Vec<Point> },
    /// Open `/PolyLine` vertices and endings.
    PolyLine {
        vertices: Vec<Point>,
        start_ending: LineEnding,
        end_ending: LineEnding,
    },
}

/// Shared stroke and fill style.
#[derive(Debug, Clone, PartialEq)]
pub struct GeometricStyle {
    /// Stroke color.
    pub stroke_color: GeometricColor,
    /// Optional fill or line-ending interior color.
    pub fill_color: Option<GeometricColor>,
    /// Stroke width.
    pub width: GeometricWidth,
    /// Optional dash pattern.
    pub dash_pattern: Option<GeometricDashPattern>,
    /// Optional constant opacity.
    pub opacity: Option<GeometricOpacity>,
}

/// A geometric annotation read from a PDF.
#[derive(Debug, Clone, PartialEq)]
pub struct GeometricAnnotation {
    /// Stable identity.
    pub id: GeometricId,
    /// Zero-based page index.
    pub page_index: u32,
    /// Annotation geometry.
    pub geometry: GeometricGeometry,
    /// Annotation style.
    pub style: GeometricStyle,
}

/// Atomic geometric annotation mutation.
#[derive(Debug, Clone, PartialEq)]
pub enum GeometricMutation {
    /// Add an annotation.
    Add {
        page_index: u32,
        geometry: GeometricGeometry,
        style: GeometricStyle,
    },
    /// Update an annotation, preserving unrelated dictionary keys.
    Update {
        id: GeometricId,
        geometry: GeometricGeometry,
        style: GeometricStyle,
    },
    /// Remove an annotation reference.
    Remove { id: GeometricId },
}

/// Result of an atomic incremental update.
#[derive(Debug, Clone)]
pub struct GeometricUpdate {
    /// Complete PDF bytes; the input is an exact prefix.
    pub pdf_bytes: Vec<u8>,
    /// Current geometric annotations.
    pub annotations: Vec<GeometricAnnotation>,
}

/// Reads and atomically edits standard geometric annotations.
pub struct IncrementalGeometricEditor<'a> {
    base_bytes: &'a [u8],
}
impl<'a> IncrementalGeometricEditor<'a> {
    /// Create an editor over PDF bytes.
    pub fn new(base_bytes: &'a [u8]) -> Self {
        Self { base_bytes }
    }
    /// Read all indirect Line, Square, Circle, Polygon, and PolyLine annotations.
    pub fn annotations(&self) -> Result<Vec<GeometricAnnotation>> {
        read_annotations(&parse_snapshot(self.base_bytes)?)
    }
    /// Validate and apply a batch as one incremental revision.
    pub fn apply(&self, mutations: &[GeometricMutation]) -> Result<GeometricUpdate> {
        let mut snapshot = parse_snapshot(self.base_bytes)?;
        let existing = read_annotations(&snapshot)?;
        if mutations.is_empty() {
            return Ok(GeometricUpdate {
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
                GeometricMutation::Add {
                    page_index,
                    geometry,
                    style,
                } => {
                    let id = update.allocate_id()?;
                    let appearance_id = update.allocate_id()?;
                    let rect = geometry_bounds(geometry, style.width);
                    update.replace(
                        appearance_id,
                        PdfObject::Stream(appearance(geometry, style, rect)?),
                    )?;
                    update.replace(
                        id,
                        PdfObject::Dictionary(new_dictionary(geometry, style, rect, appearance_id)),
                    )?;
                    snapshot.pages[*page_index as usize]
                        .annotations
                        .push(PdfObject::Reference(id.0, id.1));
                    changed_pages.insert(*page_index);
                }
                GeometricMutation::Update {
                    id,
                    geometry,
                    style,
                } => {
                    let state = snapshot
                        .annotations
                        .get(&id.tuple())
                        .ok_or_else(|| missing(*id))?;
                    if !is_geometric(subtype(&state.dictionary)) {
                        return Err(missing(*id));
                    }
                    let mut dictionary = state.dictionary.clone();
                    let appearance_id = update.allocate_id()?;
                    let rect = geometry_bounds(geometry, style.width);
                    update.replace(
                        appearance_id,
                        PdfObject::Stream(appearance(geometry, style, rect)?),
                    )?;
                    set_properties(&mut dictionary, geometry, style, rect, appearance_id);
                    rewritten.insert(id.tuple(), dictionary);
                }
                GeometricMutation::Remove { id } => {
                    let page_index = *pages.get(id).ok_or_else(|| missing(*id))?;
                    snapshot.pages[page_index as usize]
                        .annotations
                        .retain(|v| v.as_reference() != Some(id.tuple()));
                    changed_pages.insert(page_index);
                }
            }
        }
        for (id, dictionary) in rewritten {
            update.replace(id, PdfObject::Dictionary(dictionary))?;
        }
        rewrite_pages(&mut update, &mut snapshot, &changed_pages)?;
        let pdf_bytes = update.finish()?;
        let annotations = IncrementalGeometricEditor::new(&pdf_bytes).annotations()?;
        Ok(GeometricUpdate {
            pdf_bytes,
            annotations,
        })
    }
}

fn parse_snapshot(bytes: &[u8]) -> Result<AnnotationSnapshot> {
    AnnotationSnapshot::parse_subtypes(
        bytes,
        &["Line", "Square", "Circle", "Polygon", "PolyLine"],
        "geometric annotation",
    )
}

fn is_geometric(value: Option<&str>) -> bool {
    matches!(
        value,
        Some("Line" | "Square" | "Circle" | "Polygon" | "PolyLine")
    )
}

fn read_annotations(snapshot: &AnnotationSnapshot) -> Result<Vec<GeometricAnnotation>> {
    let mut result = Vec::new();
    for (&(number, generation), state) in &snapshot.annotations {
        if is_geometric(subtype(&state.dictionary)) {
            result.push(parse_annotation(
                GeometricId::new(number, generation),
                state.page_index,
                &state.dictionary,
                snapshot.pages[state.page_index as usize].bounds,
            )?);
        }
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

fn parse_annotation(
    id: GeometricId,
    page_index: u32,
    dict: &PdfDictionary,
    page: [f64; 4],
) -> Result<GeometricAnnotation> {
    let subtype = subtype(dict).ok_or_else(|| invalid("geometric annotation has no subtype"))?;
    let declared_rect = parse_rect(dict)?;
    let geometry = match subtype {
        "Line" => {
            let v = numeric_array(dict, "L", Some(4))?;
            let (a, b) = parse_endings(dict)?;
            GeometricGeometry::Line {
                start: Point::new(v[0], v[1]),
                end: Point::new(v[2], v[3]),
                start_ending: a,
                end_ending: b,
            }
        }
        "Square" => GeometricGeometry::Square {
            rect: parse_rect(dict)?,
        },
        "Circle" => GeometricGeometry::Circle {
            rect: parse_rect(dict)?,
        },
        "Polygon" => GeometricGeometry::Polygon {
            vertices: parse_vertices(dict, 3)?,
        },
        "PolyLine" => {
            let (a, b) = parse_endings(dict)?;
            GeometricGeometry::PolyLine {
                vertices: parse_vertices(dict, 2)?,
                start_ending: a,
                end_ending: b,
            }
        }
        _ => return Err(invalid("unsupported geometric subtype")),
    };
    let style = parse_style(dict)?;
    validate_geometry(&geometry, &style, page)?;
    let required_rect = geometry_bounds(&geometry, style.width);
    if !rect_contains(declared_rect, required_rect) {
        return Err(invalid(
            "geometric /Rect does not contain its geometry and decorations",
        ));
    }
    Ok(GeometricAnnotation {
        id,
        page_index,
        geometry,
        style,
    })
}

fn parse_style(dict: &PdfDictionary) -> Result<GeometricStyle> {
    let stroke_color = parse_color(dict, "C")?.unwrap_or(GeometricColor::Gray([0.0]));
    let fill_color = parse_color(dict, "IC")?;
    let (width, dash_pattern) = match dict.get("BS") {
        None => parse_legacy_border(dict)?,
        Some(PdfObject::Dictionary(bs)) => {
            let width = match bs.get("W") {
                None => 1.0,
                Some(v) => number(v).ok_or_else(|| invalid("geometric /BS /W must be numeric"))?,
            };
            let dash = bs
                .get("D")
                .map(|_| numeric_array(bs, "D", None).and_then(GeometricDashPattern::new))
                .transpose()?;
            (GeometricWidth::new(width)?, dash)
        }
        Some(_) => return Err(invalid("geometric /BS must be a dictionary")),
    };
    let opacity = dict
        .get("CA")
        .map(|v| {
            number(v)
                .ok_or_else(|| invalid("geometric /CA must be numeric"))
                .and_then(GeometricOpacity::new)
        })
        .transpose()?;
    Ok(GeometricStyle {
        stroke_color,
        fill_color,
        width,
        dash_pattern,
        opacity,
    })
}

fn parse_legacy_border(
    dict: &PdfDictionary,
) -> Result<(GeometricWidth, Option<GeometricDashPattern>)> {
    let Some(value) = dict.get("Border") else {
        return Ok((GeometricWidth::new(1.0)?, None));
    };
    let border = value
        .as_array()
        .ok_or_else(|| invalid("geometric /Border must be an array"))?;
    if !(3..=4).contains(&border.0.len()) {
        return Err(invalid(
            "geometric /Border must contain three or four values",
        ));
    }
    for value in &border.0[..3] {
        if number(value).is_none() {
            return Err(invalid("geometric /Border dimensions must be numeric"));
        }
    }
    let width = GeometricWidth::new(number(&border.0[2]).unwrap_or(1.0))?;
    let dash = border
        .0
        .get(3)
        .map(|value| {
            let values = value
                .as_array()
                .ok_or_else(|| invalid("geometric /Border dash must be an array"))?
                .0
                .iter()
                .map(|item| {
                    number(item).ok_or_else(|| invalid("geometric /Border dash must be numeric"))
                })
                .collect::<Result<Vec<_>>>()?;
            if values.is_empty() {
                Ok(None)
            } else {
                GeometricDashPattern::new(values).map(Some)
            }
        })
        .transpose()?
        .flatten();
    Ok((width, dash))
}

fn parse_color(dict: &PdfDictionary, key: &str) -> Result<Option<GeometricColor>> {
    let Some(_) = dict.get(key) else {
        return Ok(None);
    };
    let v = numeric_array(dict, key, None)?;
    match v.as_slice() {
        [g] => Ok(Some(GeometricColor::gray(*g)?)),
        [r, g, b] => Ok(Some(GeometricColor::rgb([*r, *g, *b])?)),
        [c, m, y, k] => Ok(Some(GeometricColor::cmyk([*c, *m, *y, *k])?)),
        _ => Err(invalid(
            "geometric color must have one, three, or four components",
        )),
    }
}

fn parse_endings(dict: &PdfDictionary) -> Result<(LineEnding, LineEnding)> {
    let Some(value) = dict.get("LE") else {
        return Ok((LineEnding::None, LineEnding::None));
    };
    let a = value
        .as_array()
        .ok_or_else(|| invalid("geometric /LE must be an array"))?;
    if a.0.len() != 2 {
        return Err(invalid("geometric /LE must have two names"));
    }
    let first = a.0[0]
        .as_name()
        .ok_or_else(|| invalid("geometric /LE must contain names"))?;
    let second = a.0[1]
        .as_name()
        .ok_or_else(|| invalid("geometric /LE must contain names"))?;
    Ok((LineEnding::parse(&first.0)?, LineEnding::parse(&second.0)?))
}

fn parse_vertices(dict: &PdfDictionary, minimum: usize) -> Result<Vec<Point>> {
    let v = numeric_array(dict, "Vertices", None)?;
    if v.len() % 2 != 0 || v.len() / 2 < minimum || v.len() / 2 > MAX_VERTICES {
        return Err(invalid("geometric vertices have invalid count"));
    }
    Ok(v.chunks_exact(2).map(|p| Point::new(p[0], p[1])).collect())
}

fn validate_batch(
    snapshot: &AnnotationSnapshot,
    existing: &[GeometricAnnotation],
    mutations: &[GeometricMutation],
) -> Result<HashMap<GeometricId, u32>> {
    let pages: HashMap<_, _> = existing.iter().map(|a| (a.id, a.page_index)).collect();
    let mut targeted = HashSet::new();
    for mutation in mutations {
        match mutation {
            GeometricMutation::Add {
                page_index,
                geometry,
                style,
            } => {
                let page = snapshot
                    .pages
                    .get(*page_index as usize)
                    .ok_or_else(|| invalid(&format!("page {page_index} does not exist")))?;
                validate_geometry(geometry, style, page.bounds)?;
            }
            GeometricMutation::Update {
                id,
                geometry,
                style,
            } => {
                if !targeted.insert(*id) {
                    return Err(invalid("geometric annotation is targeted more than once"));
                }
                let p = *pages.get(id).ok_or_else(|| missing(*id))?;
                let original = snapshot
                    .annotations
                    .get(&id.tuple())
                    .and_then(|state| subtype(&state.dictionary));
                if original != Some(geometry_subtype(geometry)) {
                    return Err(invalid(
                        "geometric updates cannot change the annotation subtype",
                    ));
                }
                validate_geometry(geometry, style, snapshot.pages[p as usize].bounds)?;
            }
            GeometricMutation::Remove { id } => {
                if !targeted.insert(*id) {
                    return Err(invalid("geometric annotation is targeted more than once"));
                }
                if !pages.contains_key(id) {
                    return Err(missing(*id));
                }
            }
        }
    }
    Ok(pages)
}

fn validate_geometry(
    geometry: &GeometricGeometry,
    style: &GeometricStyle,
    page: [f64; 4],
) -> Result<()> {
    let points: Vec<Point> = match geometry {
        GeometricGeometry::Line { start, end, .. } => {
            if start == end {
                return Err(invalid("line endpoints must differ"));
            }
            vec![*start, *end]
        }
        GeometricGeometry::Square { rect } | GeometricGeometry::Circle { rect } => {
            validate_rect(*rect)?;
            if style.width.value() >= (rect[2] - rect[0]).min(rect[3] - rect[1]) {
                return Err(invalid("geometric width is too large for the rectangle"));
            }
            vec![Point::new(rect[0], rect[1]), Point::new(rect[2], rect[3])]
        }
        GeometricGeometry::Polygon { vertices } => {
            if vertices.len() < 3 {
                return Err(invalid("polygon requires at least three vertices"));
            }
            if vertices
                .iter()
                .zip(vertices.iter().cycle().skip(1))
                .any(|(first, second)| first == second)
            {
                return Err(invalid("polygon contains a zero-length edge"));
            }
            let twice_area = vertices
                .iter()
                .zip(vertices.iter().cycle().skip(1))
                .map(|(first, second)| first.x * second.y - second.x * first.y)
                .sum::<f64>();
            if !twice_area.is_finite() || twice_area.abs() <= f64::EPSILON {
                return Err(invalid("polygon must have a finite non-zero area"));
            }
            vertices.clone()
        }
        GeometricGeometry::PolyLine { vertices, .. } => {
            if vertices.len() < 2 {
                return Err(invalid("polyline requires at least two vertices"));
            }
            if vertices.windows(2).all(|pair| pair[0] == pair[1]) {
                return Err(invalid("polyline requires a non-zero-length segment"));
            }
            vertices.clone()
        }
    };
    if points.len() > MAX_VERTICES || points.iter().any(|p| !p.x.is_finite() || !p.y.is_finite()) {
        return Err(invalid("geometric coordinates must be finite and bounded"));
    }
    let rect = geometry_bounds(geometry, style.width);
    if rect[0] < page[0] || rect[1] < page[1] || rect[2] > page[2] || rect[3] > page[3] {
        return Err(invalid("geometric annotation is outside page bounds"));
    }
    Ok(())
}

fn geometry_bounds(geometry: &GeometricGeometry, width: GeometricWidth) -> [f64; 4] {
    if let GeometricGeometry::Square { rect } | GeometricGeometry::Circle { rect } = geometry {
        return *rect;
    }
    let mut b = [
        f64::INFINITY,
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::NEG_INFINITY,
    ];
    let mut add = |p: Point| {
        b[0] = b[0].min(p.x);
        b[1] = b[1].min(p.y);
        b[2] = b[2].max(p.x);
        b[3] = b[3].max(p.y);
    };
    match geometry {
        GeometricGeometry::Line { start, end, .. } => {
            add(*start);
            add(*end)
        }
        GeometricGeometry::Polygon { vertices } | GeometricGeometry::PolyLine { vertices, .. } => {
            vertices.iter().for_each(|p| add(*p))
        }
        _ => {}
    }
    let has_endings = match geometry {
        GeometricGeometry::Line {
            start_ending,
            end_ending,
            ..
        }
        | GeometricGeometry::PolyLine {
            start_ending,
            end_ending,
            ..
        } => *start_ending != LineEnding::None || *end_ending != LineEnding::None,
        _ => false,
    };
    let h = if has_endings {
        (width.value() * 4.0).max(6.0)
    } else {
        width.value() / 2.0
    };
    [b[0] - h, b[1] - h, b[2] + h, b[3] + h]
}

fn new_dictionary(
    geometry: &GeometricGeometry,
    style: &GeometricStyle,
    rect: [f64; 4],
    ap: (u32, u16),
) -> PdfDictionary {
    let mut d = PdfDictionary::new();
    d.insert("Type".into(), name("Annot"));
    set_properties(&mut d, geometry, style, rect, ap);
    d
}

fn geometry_subtype(geometry: &GeometricGeometry) -> &'static str {
    match geometry {
        GeometricGeometry::Line { .. } => "Line",
        GeometricGeometry::Square { .. } => "Square",
        GeometricGeometry::Circle { .. } => "Circle",
        GeometricGeometry::Polygon { .. } => "Polygon",
        GeometricGeometry::PolyLine { .. } => "PolyLine",
    }
}

fn set_properties(
    d: &mut PdfDictionary,
    geometry: &GeometricGeometry,
    style: &GeometricStyle,
    rect: [f64; 4],
    appearance: (u32, u16),
) {
    let subtype = geometry_subtype(geometry);
    d.insert("Subtype".into(), name(subtype));
    d.insert("Rect".into(), numbers(&rect));
    d.0.remove(&PdfName("L".into()));
    d.0.remove(&PdfName("Vertices".into()));
    d.0.remove(&PdfName("LE".into()));
    match geometry {
        GeometricGeometry::Line {
            start,
            end,
            start_ending,
            end_ending,
        } => {
            d.insert("L".into(), numbers(&[start.x, start.y, end.x, end.y]));
            d.insert(
                "LE".into(),
                names(&[start_ending.name(), end_ending.name()]),
            );
        }
        GeometricGeometry::PolyLine {
            vertices,
            start_ending,
            end_ending,
        } => {
            d.insert("Vertices".into(), points(vertices));
            d.insert(
                "LE".into(),
                names(&[start_ending.name(), end_ending.name()]),
            );
        }
        GeometricGeometry::Polygon { vertices } => {
            d.insert("Vertices".into(), points(vertices));
        }
        _ => {}
    }
    d.insert("C".into(), numbers(&style.stroke_color.values()));
    match style.fill_color {
        Some(c) => {
            d.insert("IC".into(), numbers(&c.values()));
        }
        None => {
            d.0.remove(&PdfName("IC".into()));
        }
    }
    let mut bs = match d.get("BS") {
        Some(PdfObject::Dictionary(v)) => v.clone(),
        _ => PdfDictionary::new(),
    };
    bs.insert("W".into(), PdfObject::Real(style.width.value()));
    match &style.dash_pattern {
        Some(v) => {
            bs.insert("S".into(), name("D"));
            bs.insert("D".into(), numbers(v.values()));
        }
        None => {
            bs.insert("S".into(), name("S"));
            bs.0.remove(&PdfName("D".into()));
        }
    }
    d.insert("BS".into(), PdfObject::Dictionary(bs));
    match style.opacity {
        Some(v) => {
            d.insert("CA".into(), PdfObject::Real(v.value()));
        }
        None => {
            d.0.remove(&PdfName("CA".into()));
        }
    }
    let mut ap = match d.get("AP") {
        Some(PdfObject::Dictionary(v)) => v.clone(),
        _ => PdfDictionary::new(),
    };
    ap.insert("N".into(), PdfObject::Reference(appearance.0, appearance.1));
    d.insert("AP".into(), PdfObject::Dictionary(ap));
}

fn appearance(
    geometry: &GeometricGeometry,
    style: &GeometricStyle,
    rect: [f64; 4],
) -> Result<PdfStream> {
    if estimate_appearance_bytes(geometry, style, rect)? > MAX_APPEARANCE_BYTES {
        return Err(invalid("geometric appearance exceeds the byte limit"));
    }
    let mut s = format!(
        "q\n{}\n{} w\n",
        style.stroke_color.stroke_operator(),
        pdf_number(style.width.value())
    );
    if let Some(d) = &style.dash_pattern {
        s.push_str(&format!(
            "[{}] 0 d\n",
            d.values()
                .iter()
                .map(|v| pdf_number(*v))
                .collect::<Vec<_>>()
                .join(" ")
        ));
    }
    if style.opacity.is_some() {
        s.push_str("/GS0 gs\n");
    }
    s.push_str(&format!(
        "{}\n",
        style
            .fill_color
            .unwrap_or(style.stroke_color)
            .fill_operator()
    ));
    let x = |v: f64| pdf_number(v - rect[0]);
    let y = |v: f64| pdf_number(v - rect[1]);
    match geometry {
        GeometricGeometry::Line {
            start,
            end,
            start_ending,
            end_ending,
        } => {
            s.push_str(&format!(
                "{} {} m {} {} l S\n",
                x(start.x),
                y(start.y),
                x(end.x),
                y(end.y)
            ));
            draw_ending(&mut s, *start, *end, *start_ending, style.width, rect);
            draw_ending(&mut s, *end, *start, *end_ending, style.width, rect);
        }
        GeometricGeometry::Square { rect: r } => {
            let inset = style.width.value() / 2.0;
            s.push_str(&format!(
                "{} {} {} {} re {}\n",
                x(r[0] + inset),
                y(r[1] + inset),
                pdf_number(r[2] - r[0] - 2.0 * inset),
                pdf_number(r[3] - r[1] - 2.0 * inset),
                if style.fill_color.is_some() { "B" } else { "S" }
            ));
        }
        GeometricGeometry::Circle { rect: r } => {
            let inset = style.width.value() / 2.0;
            let (x0, y0, x1, y1) = (
                r[0] + inset - rect[0],
                r[1] + inset - rect[1],
                r[2] - inset - rect[0],
                r[3] - inset - rect[1],
            );
            let (cx, cy, rx, ry) = (
                (x0 + x1) / 2.0,
                (y0 + y1) / 2.0,
                (x1 - x0) / 2.0,
                (y1 - y0) / 2.0,
            );
            let k = 0.5522847498;
            s.push_str(&format!("{} {} m {} {} {} {} {} {} c {} {} {} {} {} {} c {} {} {} {} {} {} c {} {} {} {} {} {} c {}\n",pdf_number(cx+rx),pdf_number(cy),pdf_number(cx+rx),pdf_number(cy+k*ry),pdf_number(cx+k*rx),pdf_number(cy+ry),pdf_number(cx),pdf_number(cy+ry),pdf_number(cx-k*rx),pdf_number(cy+ry),pdf_number(cx-rx),pdf_number(cy+k*ry),pdf_number(cx-rx),pdf_number(cy),pdf_number(cx-rx),pdf_number(cy-k*ry),pdf_number(cx-k*rx),pdf_number(cy-ry),pdf_number(cx),pdf_number(cy-ry),pdf_number(cx+k*rx),pdf_number(cy-ry),pdf_number(cx+rx),pdf_number(cy-k*ry),pdf_number(cx+rx),pdf_number(cy),if style.fill_color.is_some(){"B"}else{"S"}));
        }
        GeometricGeometry::Polygon { vertices } => {
            draw_vertices(&mut s, vertices, rect, true, style.fill_color.is_some())
        }
        GeometricGeometry::PolyLine {
            vertices,
            start_ending,
            end_ending,
        } => {
            draw_vertices(&mut s, vertices, rect, false, false);
            draw_ending(
                &mut s,
                vertices[0],
                vertices[1],
                *start_ending,
                style.width,
                rect,
            );
            let last = vertices.len() - 1;
            draw_ending(
                &mut s,
                vertices[last],
                vertices[last - 1],
                *end_ending,
                style.width,
                rect,
            );
        }
    }
    s.push('Q');
    if s.len() > MAX_APPEARANCE_BYTES {
        return Err(invalid("geometric appearance exceeds the byte limit"));
    }
    let mut d = PdfDictionary::new();
    d.insert("Type".into(), name("XObject"));
    d.insert("Subtype".into(), name("Form"));
    d.insert("FormType".into(), PdfObject::Integer(1));
    d.insert(
        "BBox".into(),
        numbers(&[0.0, 0.0, rect[2] - rect[0], rect[3] - rect[1]]),
    );
    if let Some(o) = style.opacity {
        let mut gs = PdfDictionary::new();
        gs.insert("CA".into(), PdfObject::Real(o.value()));
        gs.insert("ca".into(), PdfObject::Real(o.value()));
        let mut e = PdfDictionary::new();
        e.insert("GS0".into(), PdfObject::Dictionary(gs));
        let mut r = PdfDictionary::new();
        r.insert("ExtGState".into(), PdfObject::Dictionary(e));
        d.insert("Resources".into(), PdfObject::Dictionary(r));
    }
    Ok(PdfStream {
        dict: d,
        data: s.into_bytes(),
    })
}

fn estimate_appearance_bytes(
    geometry: &GeometricGeometry,
    style: &GeometricStyle,
    rect: [f64; 4],
) -> Result<usize> {
    let mut size = 4096usize;
    let mut add_point = |point: Point| -> Result<()> {
        let coordinate_bytes = pdf_number(point.x - rect[0])
            .len()
            .checked_add(pdf_number(point.y - rect[1]).len())
            .and_then(|value| value.checked_add(16))
            .ok_or_else(|| invalid("geometric appearance size overflow"))?;
        size = size
            .checked_add(coordinate_bytes)
            .ok_or_else(|| invalid("geometric appearance size overflow"))?;
        Ok(())
    };
    match geometry {
        GeometricGeometry::Line { start, end, .. } => {
            add_point(*start)?;
            add_point(*end)?;
            size = size.saturating_add(4096);
        }
        GeometricGeometry::Square { rect } | GeometricGeometry::Circle { rect } => {
            add_point(Point::new(rect[0], rect[1]))?;
            add_point(Point::new(rect[2], rect[3]))?;
        }
        GeometricGeometry::Polygon { vertices } | GeometricGeometry::PolyLine { vertices, .. } => {
            for point in vertices {
                add_point(*point)?;
            }
            size = size.saturating_add(4096);
        }
    }
    for value in style
        .dash_pattern
        .as_ref()
        .into_iter()
        .flat_map(|pattern| pattern.values())
    {
        size = size
            .checked_add(pdf_number(*value).len() + 2)
            .ok_or_else(|| invalid("geometric appearance size overflow"))?;
    }
    Ok(size)
}

fn draw_ending(
    s: &mut String,
    tip: Point,
    neighbor: Point,
    ending: LineEnding,
    width: GeometricWidth,
    rect: [f64; 4],
) {
    if ending == LineEnding::None {
        return;
    }
    let dx = neighbor.x - tip.x;
    let dy = neighbor.y - tip.y;
    let length = dx.hypot(dy);
    if length == 0.0 {
        return;
    }
    let ux = dx / length;
    let uy = dy / length;
    let px = -uy;
    let py = ux;
    let size = (width.value() * 4.0).max(6.0);
    let point = |forward: f64, side: f64| {
        (
            tip.x + ux * forward + px * side - rect[0],
            tip.y + uy * forward + py * side - rect[1],
        )
    };
    let (left_x, left_y) = point(size, size * 0.5);
    let (right_x, right_y) = point(size, -size * 0.5);
    let (tip_x, tip_y) = point(0.0, 0.0);
    match ending {
        LineEnding::OpenArrow => s.push_str(&format!(
            "{} {} m {} {} l {} {} l S\n",
            pdf_number(left_x),
            pdf_number(left_y),
            pdf_number(tip_x),
            pdf_number(tip_y),
            pdf_number(right_x),
            pdf_number(right_y)
        )),
        LineEnding::ClosedArrow => s.push_str(&format!(
            "{} {} m {} {} l {} {} l h B\n",
            pdf_number(left_x),
            pdf_number(left_y),
            pdf_number(tip_x),
            pdf_number(tip_y),
            pdf_number(right_x),
            pdf_number(right_y)
        )),
        LineEnding::ROpenArrow | LineEnding::RClosedArrow => {
            let (apex_x, apex_y) = point(size, 0.0);
            let (base_left_x, base_left_y) = point(0.0, size * 0.5);
            let (base_right_x, base_right_y) = point(0.0, -size * 0.5);
            s.push_str(&format!(
                "{} {} m {} {} l {} {} l {}\n",
                pdf_number(base_left_x),
                pdf_number(base_left_y),
                pdf_number(apex_x),
                pdf_number(apex_y),
                pdf_number(base_right_x),
                pdf_number(base_right_y),
                if ending == LineEnding::RClosedArrow {
                    "h B"
                } else {
                    "S"
                }
            ));
        }
        LineEnding::Square => {
            let half = size * 0.4;
            let corners = [
                point(-half, half),
                point(half, half),
                point(half, -half),
                point(-half, -half),
            ];
            draw_closed_shape(s, &corners);
        }
        LineEnding::Diamond => {
            let half = size * 0.55;
            let corners = [
                point(-half, 0.0),
                point(0.0, half),
                point(half, 0.0),
                point(0.0, -half),
            ];
            draw_closed_shape(s, &corners);
        }
        LineEnding::Circle => {
            let radius = size * 0.4;
            let k = radius * 0.552_284_749_8;
            s.push_str(&format!(
                "{} {} m {} {} {} {} {} {} c {} {} {} {} {} {} c {} {} {} {} {} {} c {} {} {} {} {} {} c B\n",
                pdf_number(tip_x + radius), pdf_number(tip_y),
                pdf_number(tip_x + radius), pdf_number(tip_y + k), pdf_number(tip_x + k), pdf_number(tip_y + radius), pdf_number(tip_x), pdf_number(tip_y + radius),
                pdf_number(tip_x - k), pdf_number(tip_y + radius), pdf_number(tip_x - radius), pdf_number(tip_y + k), pdf_number(tip_x - radius), pdf_number(tip_y),
                pdf_number(tip_x - radius), pdf_number(tip_y - k), pdf_number(tip_x - k), pdf_number(tip_y - radius), pdf_number(tip_x), pdf_number(tip_y - radius),
                pdf_number(tip_x + k), pdf_number(tip_y - radius), pdf_number(tip_x + radius), pdf_number(tip_y - k), pdf_number(tip_x + radius), pdf_number(tip_y)
            ));
        }
        LineEnding::Butt => s.push_str(&format!(
            "{} {} m {} {} l S\n",
            pdf_number(tip_x - px * size * 0.5),
            pdf_number(tip_y - py * size * 0.5),
            pdf_number(tip_x + px * size * 0.5),
            pdf_number(tip_y + py * size * 0.5)
        )),
        LineEnding::Slash => {
            let half = size * 0.6;
            let (first_x, first_y) = point(-half * 0.5, -half);
            let (second_x, second_y) = point(half * 0.5, half);
            s.push_str(&format!(
                "{} {} m {} {} l S\n",
                pdf_number(first_x),
                pdf_number(first_y),
                pdf_number(second_x),
                pdf_number(second_y)
            ));
        }
        LineEnding::None => {}
    }
}

fn draw_closed_shape(s: &mut String, points: &[(f64, f64); 4]) {
    s.push_str(&format!(
        "{} {} m {} {} l {} {} l {} {} l h B\n",
        pdf_number(points[0].0),
        pdf_number(points[0].1),
        pdf_number(points[1].0),
        pdf_number(points[1].1),
        pdf_number(points[2].0),
        pdf_number(points[2].1),
        pdf_number(points[3].0),
        pdf_number(points[3].1)
    ));
}
fn draw_vertices(s: &mut String, v: &[Point], r: [f64; 4], close: bool, fill: bool) {
    s.push_str(&format!(
        "{} {} m\n",
        pdf_number(v[0].x - r[0]),
        pdf_number(v[0].y - r[1])
    ));
    for p in &v[1..] {
        s.push_str(&format!(
            "{} {} l\n",
            pdf_number(p.x - r[0]),
            pdf_number(p.y - r[1])
        ));
    }
    if close {
        s.push_str("h\n");
    }
    s.push_str(if fill { "B\n" } else { "S\n" });
}

fn rewrite_pages(
    update: &mut IncrementalUpdate,
    snapshot: &mut AnnotationSnapshot,
    changed: &HashSet<u32>,
) -> Result<()> {
    for i in changed {
        let p = &mut snapshot.pages[*i as usize];
        match p.container {
            AnnotationContainer::Page => {
                p.dictionary.insert(
                    "Annots".into(),
                    PdfObject::Array(PdfArray(p.annotations.clone())),
                );
                update.replace(p.reference, PdfObject::Dictionary(p.dictionary.clone()))?;
            }
            AnnotationContainer::Indirect(id) => {
                update.replace(id, PdfObject::Array(PdfArray(p.annotations.clone())))?
            }
        }
    }
    Ok(())
}
fn validate_policy(bytes: &[u8]) -> Result<()> {
    let mut r =
        PdfReader::new(Cursor::new(bytes)).map_err(|e| invalid(&format!("parse base PDF: {e}")))?;
    let c = r
        .catalog()
        .map_err(|e| invalid(&format!("read catalog: {e}")))?
        .clone();
    ensure_modification_allowed(&mut r, &c, IncrementalModification::AddAnnotation)
}
fn parse_rect(d: &PdfDictionary) -> Result<[f64; 4]> {
    let v = numeric_array(d, "Rect", Some(4))?;
    let r = [v[0], v[1], v[2], v[3]];
    validate_rect(r)?;
    Ok(r)
}
fn validate_rect(r: [f64; 4]) -> Result<()> {
    if r.iter().any(|v| !v.is_finite()) || r[0] >= r[2] || r[1] >= r[3] {
        Err(invalid("geometric rectangle must be finite and ordered"))
    } else {
        Ok(())
    }
}
fn rect_contains(outer: [f64; 4], inner: [f64; 4]) -> bool {
    outer[0] <= inner[0] && outer[1] <= inner[1] && outer[2] >= inner[2] && outer[3] >= inner[3]
}
fn numeric_array(d: &PdfDictionary, k: &str, len: Option<usize>) -> Result<Vec<f64>> {
    let a = d
        .get(k)
        .and_then(PdfObject::as_array)
        .ok_or_else(|| invalid(&format!("geometric /{k} must be an array")))?;
    if len.is_some_and(|n| a.0.len() != n) {
        return Err(invalid(&format!("geometric /{k} has wrong length")));
    }
    a.0.iter()
        .map(|v| {
            number(v).ok_or_else(|| invalid(&format!("geometric /{k} contains invalid number")))
        })
        .collect()
}
fn number(v: &PdfObject) -> Option<f64> {
    match v {
        PdfObject::Integer(n) => Some(*n as f64),
        PdfObject::Real(n) if n.is_finite() => Some(*n),
        _ => None,
    }
}
fn numbers(v: &[f64]) -> PdfObject {
    PdfObject::Array(PdfArray(v.iter().copied().map(PdfObject::Real).collect()))
}
fn points(v: &[Point]) -> PdfObject {
    numbers(&v.iter().flat_map(|p| [p.x, p.y]).collect::<Vec<_>>())
}
fn names(v: &[&str]) -> PdfObject {
    PdfObject::Array(PdfArray(v.iter().map(|n| name(n)).collect()))
}
fn name(v: &str) -> PdfObject {
    PdfObject::Name(PdfName(v.into()))
}
fn validate_unit(v: &[f64], label: &str) -> Result<()> {
    if v.iter().any(|x| !x.is_finite() || !(0.0..=1.0).contains(x)) {
        Err(invalid(&format!(
            "{label} must be finite and between 0 and 1"
        )))
    } else {
        Ok(())
    }
}
fn color_operator(c: GeometricColor, stroke: bool) -> String {
    let op = match (c, stroke) {
        (GeometricColor::Gray(_), true) => "G",
        (GeometricColor::Gray(_), false) => "g",
        (GeometricColor::Rgb(_), true) => "RG",
        (GeometricColor::Rgb(_), false) => "rg",
        (GeometricColor::Cmyk(_), true) => "K",
        (GeometricColor::Cmyk(_), false) => "k",
    };
    format!(
        "{} {op}",
        c.values()
            .iter()
            .map(|v| pdf_number(*v))
            .collect::<Vec<_>>()
            .join(" ")
    )
}
fn pdf_number(v: f64) -> String {
    let mut s = format!("{v:.6}");
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
fn missing(id: GeometricId) -> PdfError {
    invalid(&format!(
        "geometric annotation {} {} does not exist",
        id.object_number, id.generation_number
    ))
}
fn invalid(m: &str) -> PdfError {
    PdfError::InvalidStructure(m.into())
}
