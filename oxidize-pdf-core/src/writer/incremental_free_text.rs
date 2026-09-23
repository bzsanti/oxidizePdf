//! Incremental editing of standard `/FreeText` annotations in existing PDFs.

use super::incremental_annotations::{subtype, AnnotationContainer, AnnotationSnapshot, PageState};
use super::incremental_update::IncrementalUpdate;
use crate::error::{PdfError, Result};
use crate::parser::objects::{PdfArray, PdfDictionary, PdfName, PdfObject, PdfStream, PdfString};
use crate::parser::PdfReader;
use crate::signatures::{ensure_modification_allowed, IncrementalModification};
use crate::text::{escape_pdf_string_literal, TextEncoding};
use std::collections::{HashMap, HashSet};
use std::io::Cursor;

/// Stable indirect-object identity of a free-text annotation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FreeTextId {
    /// PDF object number.
    pub object_number: u32,
    /// PDF generation number.
    pub generation_number: u16,
}

impl FreeTextId {
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

/// Horizontal alignment stored in a free-text annotation's `/Q` entry.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum FreeTextAlignment {
    /// Left justified (`/Q 0`).
    #[default]
    Left,
    /// Centered (`/Q 1`).
    Center,
    /// Right justified (`/Q 2`).
    Right,
}

impl FreeTextAlignment {
    fn from_pdf(value: i64) -> Result<Self> {
        match value {
            0 => Ok(Self::Left),
            1 => Ok(Self::Center),
            2 => Ok(Self::Right),
            _ => Err(PdfError::InvalidStructure(format!(
                "/FreeText annotation /Q must be 0, 1, or 2, found {value}"
            ))),
        }
    }

    const fn pdf_value(self) -> i64 {
        match self {
            Self::Left => 0,
            Self::Center => 1,
            Self::Right => 2,
        }
    }
}

/// A standard PDF free-text annotation.
#[derive(Debug, Clone, PartialEq)]
pub struct FreeText {
    /// Stable indirect-object identity.
    pub id: FreeTextId,
    /// Zero-based page index.
    pub page_index: u32,
    /// Annotation rectangle `[left, bottom, right, top]`.
    pub rect: [f64; 4],
    /// Annotation contents decoded as Unicode.
    pub contents: String,
    /// Default appearance string (`/DA`).
    pub default_appearance: String,
    /// Horizontal text alignment.
    pub alignment: FreeTextAlignment,
}

/// A requested change to the document's free-text annotations.
#[derive(Debug, Clone, PartialEq)]
pub enum FreeTextMutation {
    /// Add a free-text annotation to a page.
    Add {
        /// Zero-based target page.
        page_index: u32,
        /// Annotation rectangle `[left, bottom, right, top]`.
        rect: [f64; 4],
        /// Non-empty annotation contents.
        contents: String,
        /// Non-empty ASCII PDF default appearance string, for example `/Helv 12 Tf 0 g`.
        default_appearance: String,
        /// Horizontal text alignment.
        alignment: FreeTextAlignment,
    },
    /// Update an existing annotation while preserving all unrelated keys.
    Update {
        /// Existing free-text annotation identity.
        id: FreeTextId,
        /// New annotation rectangle `[left, bottom, right, top]`.
        rect: [f64; 4],
        /// New non-empty annotation contents.
        contents: String,
        /// New non-empty ASCII PDF default appearance string.
        default_appearance: String,
        /// New horizontal text alignment.
        alignment: FreeTextAlignment,
    },
    /// Remove an existing annotation reference from its page.
    Remove {
        /// Existing free-text annotation identity.
        id: FreeTextId,
    },
}

/// Result of one atomic incremental free-text update.
#[derive(Debug, Clone)]
pub struct FreeTextUpdate {
    /// Complete PDF bytes. The original document is an exact prefix.
    pub pdf_bytes: Vec<u8>,
    /// Complete current free-text set after the update.
    pub annotations: Vec<FreeText>,
}

/// Reads and atomically edits standard free-text annotations in an existing PDF.
pub struct IncrementalFreeTextEditor<'a> {
    base_bytes: &'a [u8],
}

impl<'a> IncrementalFreeTextEditor<'a> {
    /// Create an editor over an existing PDF's bytes.
    pub fn new(base_bytes: &'a [u8]) -> Self {
        Self { base_bytes }
    }

    /// Read every indirect `/FreeText` annotation with its stable identity.
    ///
    /// # Errors
    ///
    /// Returns an error when the PDF is malformed or encrypted, or when a
    /// free-text annotation has no stable identity or invalid properties.
    pub fn annotations(&self) -> Result<Vec<FreeText>> {
        Snapshot::parse(self.base_bytes)?.annotations()
    }

    /// Validate and apply a batch as exactly one incremental revision.
    ///
    /// # Errors
    ///
    /// Returns an error for malformed or encrypted PDFs, invalid pages or
    /// identities, conflicting mutations, invalid annotation properties, or
    /// exhausted indirect-object numbers.
    pub fn apply(&self, mutations: &[FreeTextMutation]) -> Result<FreeTextUpdate> {
        let mut snapshot = Snapshot::parse(self.base_bytes)?;
        if mutations.is_empty() {
            return Ok(FreeTextUpdate {
                pdf_bytes: self.base_bytes.to_vec(),
                annotations: snapshot.annotations()?,
            });
        }
        validate_batch(&snapshot, mutations)?;
        validate_policy(self.base_bytes)?;

        let mut update = IncrementalUpdate::from_base(self.base_bytes)?;
        let mut changed_pages = HashSet::new();
        let mut rewritten = HashMap::new();

        for mutation in mutations {
            match mutation {
                FreeTextMutation::Add {
                    page_index,
                    rect,
                    contents,
                    default_appearance,
                    alignment,
                } => {
                    let id = update.allocate_id()?;
                    let appearance_id = update.allocate_id()?;
                    let appearance =
                        build_appearance(*rect, contents, default_appearance, *alignment)?;
                    update.replace(appearance_id, PdfObject::Stream(appearance))?;
                    let dictionary = new_dictionary(
                        *rect,
                        contents,
                        default_appearance,
                        *alignment,
                        appearance_id,
                    );
                    update.replace(id, PdfObject::Dictionary(dictionary.clone()))?;
                    snapshot.pages[*page_index as usize]
                        .annotations
                        .push(PdfObject::Reference(id.0, id.1));
                    snapshot.annotations.insert(
                        FreeTextId::new(id.0, id.1),
                        AnnotationState {
                            page_index: *page_index,
                            dictionary,
                            is_free_text: true,
                        },
                    );
                    changed_pages.insert(*page_index);
                }
                FreeTextMutation::Update {
                    id,
                    rect,
                    contents,
                    default_appearance,
                    alignment,
                } => {
                    let state = target(&snapshot, *id)?;
                    let mut dictionary = state.dictionary.clone();
                    let appearance_id = update.allocate_id()?;
                    let appearance =
                        build_appearance(*rect, contents, default_appearance, *alignment)?;
                    update.replace(appearance_id, PdfObject::Stream(appearance))?;
                    set_properties(
                        &mut dictionary,
                        *rect,
                        contents,
                        default_appearance,
                        *alignment,
                        appearance_id,
                    );
                    rewritten.insert(*id, dictionary.clone());
                    snapshot
                        .annotations
                        .get_mut(id)
                        .ok_or_else(|| {
                            PdfError::InvalidStructure(format!(
                                "annotation {} {} disappeared during batch application",
                                id.object_number, id.generation_number
                            ))
                        })?
                        .dictionary = dictionary;
                }
                FreeTextMutation::Remove { id } => {
                    let page_index = target(&snapshot, *id)?.page_index;
                    snapshot.pages[page_index as usize]
                        .annotations
                        .retain(|value| value.as_reference() != Some(id.tuple()));
                    snapshot.annotations.remove(id);
                    changed_pages.insert(page_index);
                }
            }
        }

        for (id, dictionary) in rewritten {
            update.replace(id.tuple(), PdfObject::Dictionary(dictionary))?;
        }
        rewrite_pages(&mut update, &mut snapshot.pages, &changed_pages)?;
        let annotations = snapshot.annotations()?;
        Ok(FreeTextUpdate {
            pdf_bytes: update.finish()?,
            annotations,
        })
    }
}

struct AnnotationState {
    page_index: u32,
    dictionary: PdfDictionary,
    is_free_text: bool,
}

struct Snapshot {
    pages: Vec<PageState>,
    annotations: HashMap<FreeTextId, AnnotationState>,
}

impl Snapshot {
    fn parse(bytes: &[u8]) -> Result<Self> {
        let shared = AnnotationSnapshot::parse(bytes, "FreeText", "free-text annotation")?;
        let annotations = shared
            .annotations
            .into_iter()
            .map(|((number, generation), state)| {
                (
                    FreeTextId::new(number, generation),
                    AnnotationState {
                        page_index: state.page_index,
                        is_free_text: subtype(&state.dictionary) == Some("FreeText"),
                        dictionary: state.dictionary,
                    },
                )
            })
            .collect();
        let snapshot = Self {
            pages: shared.pages,
            annotations,
        };
        snapshot.annotations()?;
        Ok(snapshot)
    }

    fn annotations(&self) -> Result<Vec<FreeText>> {
        let mut result = Vec::new();
        for (id, state) in &self.annotations {
            if state.is_free_text {
                result.push(parse_annotation(
                    *id,
                    state,
                    self.pages[state.page_index as usize].bounds,
                )?);
            }
        }
        result.sort_by_key(|annotation| {
            (
                annotation.page_index,
                annotation.id.object_number,
                annotation.id.generation_number,
            )
        });
        Ok(result)
    }
}

fn parse_annotation(
    id: FreeTextId,
    state: &AnnotationState,
    page_bounds: [f64; 4],
) -> Result<FreeText> {
    let rect = parse_rectangle(&state.dictionary)?;
    validate_rectangle(rect, page_bounds)?;
    let contents = required_text(&state.dictionary, "Contents")?;
    let default_appearance = required_text(&state.dictionary, "DA")?;
    validate_contents(&contents)?;
    validate_default_appearance(&default_appearance)?;
    let alignment = match state.dictionary.get("Q") {
        None => Ok(FreeTextAlignment::Left),
        Some(PdfObject::Integer(value)) => FreeTextAlignment::from_pdf(*value),
        Some(_) => Err(PdfError::InvalidStructure(
            "/FreeText annotation /Q must be an integer".to_string(),
        )),
    }?;
    Ok(FreeText {
        id,
        page_index: state.page_index,
        rect,
        contents,
        default_appearance,
        alignment,
    })
}

fn validate_batch(snapshot: &Snapshot, mutations: &[FreeTextMutation]) -> Result<()> {
    let mut targeted = HashSet::new();
    for mutation in mutations {
        match mutation {
            FreeTextMutation::Add {
                page_index,
                rect,
                contents,
                default_appearance,
                ..
            } => {
                let page = snapshot.pages.get(*page_index as usize).ok_or_else(|| {
                    PdfError::InvalidStructure(format!("page {page_index} does not exist"))
                })?;
                validate_properties(*rect, contents, default_appearance, page.bounds)?;
            }
            FreeTextMutation::Update {
                id,
                rect,
                contents,
                default_appearance,
                ..
            } => {
                ensure_unique_target(&mut targeted, *id)?;
                let state = target(snapshot, *id)?;
                validate_properties(
                    *rect,
                    contents,
                    default_appearance,
                    snapshot.pages[state.page_index as usize].bounds,
                )?;
            }
            FreeTextMutation::Remove { id } => {
                ensure_unique_target(&mut targeted, *id)?;
                target(snapshot, *id)?;
            }
        }
    }
    Ok(())
}

fn ensure_unique_target(targeted: &mut HashSet<FreeTextId>, id: FreeTextId) -> Result<()> {
    if !targeted.insert(id) {
        return Err(PdfError::InvalidStructure(format!(
            "free-text annotation {} {} is targeted more than once",
            id.object_number, id.generation_number
        )));
    }
    Ok(())
}

fn target(snapshot: &Snapshot, id: FreeTextId) -> Result<&AnnotationState> {
    let state = snapshot.annotations.get(&id).ok_or_else(|| {
        PdfError::InvalidStructure(format!(
            "annotation {} {} does not exist",
            id.object_number, id.generation_number
        ))
    })?;
    if !state.is_free_text {
        return Err(PdfError::InvalidStructure(format!(
            "annotation {} {} is not a /FreeText annotation",
            id.object_number, id.generation_number
        )));
    }
    Ok(state)
}

fn validate_properties(
    rect: [f64; 4],
    contents: &str,
    default_appearance: &str,
    bounds: [f64; 4],
) -> Result<()> {
    validate_rectangle(rect, bounds)?;
    validate_contents(contents)?;
    validate_default_appearance(default_appearance)
}

fn validate_rectangle(rect: [f64; 4], bounds: [f64; 4]) -> Result<()> {
    if rect.iter().any(|value| !value.is_finite()) || rect[0] >= rect[2] || rect[1] >= rect[3] {
        return Err(PdfError::InvalidStructure(
            "free-text rectangle must contain finite, ordered coordinates".to_string(),
        ));
    }
    if rect[0] < bounds[0] || rect[1] < bounds[1] || rect[2] > bounds[2] || rect[3] > bounds[3] {
        return Err(PdfError::InvalidStructure(
            "free-text rectangle is outside the page bounds".to_string(),
        ));
    }
    Ok(())
}

fn validate_contents(contents: &str) -> Result<()> {
    if contents.trim().is_empty() {
        return Err(PdfError::InvalidStructure(
            "free-text contents must not be empty".to_string(),
        ));
    }
    Ok(())
}

fn validate_default_appearance(value: &str) -> Result<()> {
    AppearanceSpec::parse(value).map(|_| ())
}

fn parse_rectangle(dictionary: &PdfDictionary) -> Result<[f64; 4]> {
    let array = dictionary
        .get("Rect")
        .and_then(PdfObject::as_array)
        .ok_or_else(|| {
            PdfError::InvalidStructure("/FreeText annotation has no /Rect".to_string())
        })?;
    if array.0.len() != 4 {
        return Err(PdfError::InvalidStructure(
            "/FreeText annotation /Rect must have four numbers".to_string(),
        ));
    }
    let mut rect = [0.0; 4];
    for (index, value) in array.0.iter().enumerate() {
        rect[index] = match value {
            PdfObject::Integer(value) => *value as f64,
            PdfObject::Real(value) if value.is_finite() => *value,
            _ => {
                return Err(PdfError::InvalidStructure(
                    "/FreeText annotation /Rect contains a non-finite or non-numeric value"
                        .to_string(),
                ))
            }
        };
    }
    if rect[0] >= rect[2] || rect[1] >= rect[3] {
        return Err(PdfError::InvalidStructure(
            "/FreeText annotation /Rect coordinates are not ordered".to_string(),
        ));
    }
    Ok(rect)
}

fn required_text(dictionary: &PdfDictionary, key: &str) -> Result<String> {
    dictionary
        .get(key)
        .and_then(PdfObject::as_string)
        .map(PdfString::to_text)
        .ok_or_else(|| {
            PdfError::InvalidStructure(format!("/FreeText annotation /{key} must be a string"))
        })
}

struct AppearanceSpec {
    resource_name: String,
    base_font: &'static str,
    font_size: f64,
}

impl AppearanceSpec {
    fn parse(value: &str) -> Result<Self> {
        if value.trim().is_empty() || !value.is_ascii() || value.contains('\0') {
            return Err(PdfError::InvalidStructure(
                "free-text default appearance must be non-empty ASCII and contain no NUL"
                    .to_string(),
            ));
        }
        let tokens: Vec<_> = value.split_whitespace().collect();
        let font_at = tokens
            .windows(3)
            .position(|window| window[0].starts_with('/') && window[2] == "Tf")
            .ok_or_else(|| {
                PdfError::InvalidStructure(
                    "free-text default appearance must contain '/Font size Tf'".to_string(),
                )
            })?;
        let resource_name = &tokens[font_at][1..];
        let base_font = base_font(resource_name).ok_or_else(|| {
            PdfError::InvalidStructure(format!(
                "free-text default appearance font /{resource_name} is not a supported non-symbolic base-14 font"
            ))
        })?;
        let font_size = parse_finite(tokens[font_at + 1], "font size")?;
        if font_size <= 0.0 {
            return Err(PdfError::InvalidStructure(
                "free-text default appearance font size must be positive".to_string(),
            ));
        }

        let remaining: Vec<_> = tokens
            .iter()
            .enumerate()
            .filter_map(|(index, token)| {
                (!(font_at..font_at + 3).contains(&index)).then_some(*token)
            })
            .collect();
        validate_color_tokens(&remaining)?;
        Ok(Self {
            resource_name: resource_name.to_string(),
            base_font,
            font_size,
        })
    }
}

fn parse_finite(token: &str, description: &str) -> Result<f64> {
    let value = token.parse::<f64>().map_err(|_| {
        PdfError::InvalidStructure(format!(
            "free-text default appearance {description} is not numeric"
        ))
    })?;
    if !value.is_finite() {
        return Err(PdfError::InvalidStructure(format!(
            "free-text default appearance {description} must be finite"
        )));
    }
    Ok(value)
}

fn validate_color_tokens(tokens: &[&str]) -> Result<()> {
    let component_count = match tokens.last().copied() {
        None => return Ok(()),
        Some("g") if tokens.len() == 2 => 1,
        Some("rg") if tokens.len() == 4 => 3,
        Some("k") if tokens.len() == 5 => 4,
        _ => {
            return Err(PdfError::InvalidStructure(
                "free-text default appearance may contain only one base-14 font and an optional g, rg, or k color"
                    .to_string(),
            ))
        }
    };
    for token in &tokens[..component_count] {
        let component = parse_finite(token, "color component")?;
        if !(0.0..=1.0).contains(&component) {
            return Err(PdfError::InvalidStructure(
                "free-text default appearance color components must be in the range 0..=1"
                    .to_string(),
            ));
        }
    }
    Ok(())
}

fn base_font(resource_name: &str) -> Option<&'static str> {
    match resource_name {
        "Helv" | "Helvetica" => Some("Helvetica"),
        "Helvetica-Bold" => Some("Helvetica-Bold"),
        "Helvetica-Oblique" => Some("Helvetica-Oblique"),
        "Helvetica-BoldOblique" => Some("Helvetica-BoldOblique"),
        "Times-Roman" => Some("Times-Roman"),
        "Times-Bold" => Some("Times-Bold"),
        "Times-Italic" => Some("Times-Italic"),
        "Times-BoldItalic" => Some("Times-BoldItalic"),
        "Courier" => Some("Courier"),
        "Courier-Bold" => Some("Courier-Bold"),
        "Courier-Oblique" => Some("Courier-Oblique"),
        "Courier-BoldOblique" => Some("Courier-BoldOblique"),
        _ => None,
    }
}

fn build_appearance(
    rect: [f64; 4],
    contents: &str,
    default_appearance: &str,
    alignment: FreeTextAlignment,
) -> Result<PdfStream> {
    let spec = AppearanceSpec::parse(default_appearance)?;
    let width = rect[2] - rect[0];
    let height = rect[3] - rect[1];
    let padding = 2.0;
    let line_height = spec.font_size * 1.2;
    let mut content = format!(
        "q\n0 0 {} {} re W n\nBT\n",
        pdf_number(width),
        pdf_number(height)
    );
    content.push_str(default_appearance);
    content.push('\n');
    for (line_index, line) in contents.lines().enumerate() {
        let encoded = TextEncoding::WinAnsiEncoding.encode(line);
        let estimated_width = encoded.len() as f64 * spec.font_size * 0.5;
        let x = match alignment {
            FreeTextAlignment::Left => padding,
            FreeTextAlignment::Center => ((width - estimated_width) / 2.0).max(padding),
            FreeTextAlignment::Right => (width - estimated_width - padding).max(padding),
        };
        let y = height - padding - spec.font_size - line_index as f64 * line_height;
        if y < 0.0 {
            break;
        }
        content.push_str(&format!(
            "1 0 0 1 {} {} Tm\n({}) Tj\n",
            pdf_number(x),
            pdf_number(y),
            escape_pdf_string_literal(&encoded)
        ));
    }
    content.push_str("ET\nQ");

    let mut font = PdfDictionary::new();
    font.insert("Type".to_string(), name("Font"));
    font.insert("Subtype".to_string(), name("Type1"));
    font.insert("BaseFont".to_string(), name(spec.base_font));
    font.insert("Encoding".to_string(), name("WinAnsiEncoding"));
    let mut fonts = PdfDictionary::new();
    fonts.insert(spec.resource_name, PdfObject::Dictionary(font));
    let mut resources = PdfDictionary::new();
    resources.insert("Font".to_string(), PdfObject::Dictionary(fonts));

    let mut dictionary = PdfDictionary::new();
    dictionary.insert("Type".to_string(), name("XObject"));
    dictionary.insert("Subtype".to_string(), name("Form"));
    dictionary.insert("FormType".to_string(), PdfObject::Integer(1));
    dictionary.insert("BBox".to_string(), rectangle([0.0, 0.0, width, height]));
    dictionary.insert("Resources".to_string(), PdfObject::Dictionary(resources));
    Ok(PdfStream {
        dict: dictionary,
        data: content.into_bytes(),
    })
}

fn pdf_number(value: f64) -> String {
    let mut formatted = format!("{value:.6}");
    while formatted.contains('.') && formatted.ends_with('0') {
        formatted.pop();
    }
    if formatted.ends_with('.') {
        formatted.pop();
    }
    if formatted == "-0" {
        "0".to_string()
    } else {
        formatted
    }
}

fn name(value: &str) -> PdfObject {
    PdfObject::Name(PdfName(value.to_string()))
}

fn validate_policy(bytes: &[u8]) -> Result<()> {
    let mut reader = PdfReader::new(Cursor::new(bytes))
        .map_err(|error| PdfError::InvalidStructure(format!("parse base PDF: {error}")))?;
    let catalog = reader
        .catalog()
        .map_err(|error| PdfError::InvalidStructure(format!("read catalog: {error}")))?
        .clone();
    ensure_modification_allowed(
        &mut reader,
        &catalog,
        IncrementalModification::AddAnnotation,
    )
}

fn new_dictionary(
    rect: [f64; 4],
    contents: &str,
    default_appearance: &str,
    alignment: FreeTextAlignment,
    appearance_id: (u32, u16),
) -> PdfDictionary {
    let mut dictionary = PdfDictionary::new();
    dictionary.insert(
        "Type".to_string(),
        PdfObject::Name(PdfName("Annot".to_string())),
    );
    dictionary.insert(
        "Subtype".to_string(),
        PdfObject::Name(PdfName("FreeText".to_string())),
    );
    set_properties(
        &mut dictionary,
        rect,
        contents,
        default_appearance,
        alignment,
        appearance_id,
    );
    dictionary
}

fn set_properties(
    dictionary: &mut PdfDictionary,
    rect: [f64; 4],
    contents: &str,
    default_appearance: &str,
    alignment: FreeTextAlignment,
    appearance_id: (u32, u16),
) {
    dictionary.insert("Rect".to_string(), rectangle(rect));
    dictionary.insert("Contents".to_string(), unicode_text(contents));
    dictionary.insert(
        "DA".to_string(),
        PdfObject::String(PdfString(default_appearance.as_bytes().to_vec())),
    );
    dictionary.insert("Q".to_string(), PdfObject::Integer(alignment.pdf_value()));
    let mut appearance = PdfDictionary::new();
    appearance.insert(
        "N".to_string(),
        PdfObject::Reference(appearance_id.0, appearance_id.1),
    );
    dictionary.insert("AP".to_string(), PdfObject::Dictionary(appearance));
}

fn rectangle(rect: [f64; 4]) -> PdfObject {
    PdfObject::Array(PdfArray(rect.into_iter().map(PdfObject::Real).collect()))
}

fn unicode_text(contents: &str) -> PdfObject {
    let mut bytes = vec![0xFE, 0xFF];
    for unit in contents.encode_utf16() {
        bytes.extend_from_slice(&unit.to_be_bytes());
    }
    PdfObject::String(PdfString(bytes))
}

fn rewrite_pages(
    update: &mut IncrementalUpdate,
    pages: &mut [PageState],
    changed_pages: &HashSet<u32>,
) -> Result<()> {
    for page_index in changed_pages {
        let page = &mut pages[*page_index as usize];
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
    Ok(())
}
