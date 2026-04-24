//! Appearance streams for form fields according to ISO 32000-1 Section 12.7.3.3
//!
//! This module provides appearance stream generation for interactive form fields,
//! ensuring visual representation of field content and states.

use crate::error::{PdfError, Result};
use crate::forms::{BorderStyle, DefaultAppearance, FieldType, Widget};
use crate::graphics::Color;
use crate::objects::{Dictionary, Object, Stream};
use crate::text::{escape_pdf_string_literal, Font, TextEncoding};
use std::collections::{HashMap, HashSet};

/// Emit a `(text) Tj` operator for a built-in PDF base-14 font.
///
/// ISO 32000-1 §9.6.2.2 specifies that the standard 14 Type1 fonts use
/// StandardEncoding by default; when a /DA string references one, viewers
/// apply WinAnsi encoding for the subset of base-14 fonts that name Latin
/// glyphs (Helvetica, Times, Courier). The string-literal bytes in the `Tj`
/// operator must therefore be *WinAnsi-encoded*, not UTF-8.
///
/// Fails explicitly (`PdfError::EncodingError`) when `text` contains any
/// codepoint WinAnsi cannot represent — the caller (typically
/// `Document::fill_field`) must propagate this so the user sees the real
/// cause instead of receiving a silently-garbled `/AP` stream.
///
/// Custom (Type0/CID) fonts and symbolic fonts (Symbol, ZapfDingbats) are
/// rejected here — they follow separate content-stream encodings that live
/// outside the WinAnsi path.
fn emit_tj_for_builtin(content: &mut String, text: &str, font: &Font) -> Result<()> {
    if font.is_custom() {
        return Err(PdfError::EncodingError(format!(
            "Custom Type0/CID fonts are not yet supported in form-field appearance \
             streams (font: {:?}). Track: https://github.com/bzsanti/oxidizePdf/issues/212",
            font.pdf_name(),
        )));
    }
    if font.is_symbolic() {
        return Err(PdfError::EncodingError(format!(
            "Symbolic fonts ({:?}) are not supported for form-field text — their \
             encoding depends on glyph names, not Unicode codepoints",
            font.pdf_name(),
        )));
    }

    // Built-in non-symbolic → WinAnsi strict. Any codepoint WinAnsi can't
    // represent fails the operation instead of emitting `?` or raw UTF-8.
    let bytes = TextEncoding::WinAnsiEncoding
        .encode_strict(text)
        .map_err(|ch| {
            PdfError::EncodingError(format!(
                "Value contains character {:?} (U+{:04X}) which cannot be encoded \
                 in WinAnsiEncoding used by built-in PDF font {}. Register a Type0 \
                 font via `Document::add_font_from_bytes` and attach it to the field; \
                 see https://github.com/bzsanti/oxidizePdf/issues/212",
                ch,
                ch as u32,
                font.pdf_name(),
            ))
        })?;

    content.push_str(&format!("({}) Tj\n", escape_pdf_string_literal(&bytes)));
    Ok(())
}

/// Emit a `<HHHH...> Tj` operator for a custom Type0/CID font.
///
/// Each Unicode codepoint is resolved to its glyph index via the font's cmap
/// (`Font::glyph_mapping.char_to_glyph`) and written as a 4-hex-digit code —
/// exactly what a `/Encoding /Identity-H` font expects on the content-stream
/// side. The chars are recorded into `used_chars` so the caller can merge
/// them into `Document::used_characters_by_font` for the subsetter (matches
/// the infrastructure introduced for issue #204).
///
/// Fails with `PdfError::EncodingError` when the font has no glyph for a
/// codepoint — silent fallback to `.notdef` would leave the user with
/// invisible-or-blank glyphs.
fn emit_tj_for_custom(
    content: &mut String,
    text: &str,
    font_name: &str,
    custom_font: &crate::fonts::Font,
    used_chars: &mut HashSet<char>,
) -> Result<()> {
    use std::fmt::Write;
    let mut hex = String::with_capacity(text.len() * 4);
    for ch in text.chars() {
        let gid = custom_font.glyph_mapping.char_to_glyph(ch).ok_or_else(|| {
            PdfError::EncodingError(format!(
                "Custom font {:?} has no glyph for character {:?} (U+{:04X}); \
                 the font's cmap does not cover this codepoint",
                font_name, ch, ch as u32,
            ))
        })?;
        write!(&mut hex, "{:04X}", gid).expect("writing to String cannot fail");
        used_chars.insert(ch);
    }
    content.push_str(&format!("<{}> Tj\n", hex));
    Ok(())
}

/// Appearance states for form fields
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AppearanceState {
    /// Normal appearance (default state)
    Normal,
    /// Rollover appearance (mouse hover)
    Rollover,
    /// Down appearance (mouse pressed)
    Down,
}

impl AppearanceState {
    /// Get the PDF name for this state
    pub fn pdf_name(&self) -> &'static str {
        match self {
            AppearanceState::Normal => "N",
            AppearanceState::Rollover => "R",
            AppearanceState::Down => "D",
        }
    }
}

/// Appearance stream for a form field
#[derive(Debug, Clone)]
pub struct AppearanceStream {
    /// The content stream data
    pub content: Vec<u8>,
    /// Resources dictionary (fonts, colors, etc.)
    pub resources: Dictionary,
    /// Bounding box for the appearance
    pub bbox: [f64; 4],
}

impl AppearanceStream {
    /// Create a new appearance stream
    pub fn new(content: Vec<u8>, bbox: [f64; 4]) -> Self {
        Self {
            content,
            resources: Dictionary::new(),
            bbox,
        }
    }

    /// Set resources dictionary
    pub fn with_resources(mut self, resources: Dictionary) -> Self {
        self.resources = resources;
        self
    }

    /// Convert to a Stream object
    pub fn to_stream(&self) -> Stream {
        let mut dict = Dictionary::new();
        dict.set("Type", Object::Name("XObject".to_string()));
        dict.set("Subtype", Object::Name("Form".to_string()));

        // Set bounding box
        let bbox_array = vec![
            Object::Real(self.bbox[0]),
            Object::Real(self.bbox[1]),
            Object::Real(self.bbox[2]),
            Object::Real(self.bbox[3]),
        ];
        dict.set("BBox", Object::Array(bbox_array));

        // Set resources
        if !self.resources.is_empty() {
            dict.set("Resources", Object::Dictionary(self.resources.clone()));
        }

        // Create stream with dictionary
        Stream::with_dictionary(dict, self.content.clone())
    }
}

/// Outcome of an appearance-stream build — carries the stream plus the
/// per-font character obligations the generator recorded. Callers
/// (typically `Document::fill_field`) merge `used_chars_by_font` into
/// `Document::used_characters_by_font` so the writer's font subsetter
/// covers every codepoint referenced from any `/AP` stream (preserves
/// the #204 invariant).
///
/// Named struct instead of a tuple so adding further outputs (e.g.
/// computed bbox overrides, resource hints) doesn't become a breaking
/// API change. The map is empty for built-in fonts that don't need
/// subsetting.
#[derive(Debug, Clone)]
pub struct FieldAppearanceResult {
    /// The rendered appearance stream ready to attach to a widget.
    pub stream: AppearanceStream,
    /// Characters consumed from each custom Type0 font, keyed by font
    /// name. Empty when every emission went through the built-in path.
    pub used_chars_by_font: HashMap<String, HashSet<char>>,
}

/// Appearance dictionary for a form field
#[derive(Debug, Clone)]
pub struct AppearanceDictionary {
    /// Appearance streams by state
    appearances: HashMap<AppearanceState, AppearanceStream>,
    /// Down appearances for different values (checkboxes, radio buttons)
    down_appearances: HashMap<String, AppearanceStream>,
}

impl AppearanceDictionary {
    /// Create a new appearance dictionary
    pub fn new() -> Self {
        Self {
            appearances: HashMap::new(),
            down_appearances: HashMap::new(),
        }
    }

    /// Set appearance for a specific state
    pub fn set_appearance(&mut self, state: AppearanceState, stream: AppearanceStream) {
        self.appearances.insert(state, stream);
    }

    /// Set down appearance for a specific value
    pub fn set_down_appearance(&mut self, value: String, stream: AppearanceStream) {
        self.down_appearances.insert(value, stream);
    }

    /// Get appearance for a state
    pub fn get_appearance(&self, state: AppearanceState) -> Option<&AppearanceStream> {
        self.appearances.get(&state)
    }

    /// Convert to PDF dictionary
    pub fn to_dict(&self) -> Dictionary {
        let mut dict = Dictionary::new();

        // Add appearances by state
        for (state, stream) in &self.appearances {
            let stream_obj = stream.to_stream();
            dict.set(
                state.pdf_name(),
                Object::Stream(stream_obj.dictionary().clone(), stream_obj.data().to_vec()),
            );
        }

        // Add down appearances if any
        if !self.down_appearances.is_empty() {
            let mut down_dict = Dictionary::new();
            for (value, stream) in &self.down_appearances {
                let stream_obj = stream.to_stream();
                down_dict.set(
                    value,
                    Object::Stream(stream_obj.dictionary().clone(), stream_obj.data().to_vec()),
                );
            }
            dict.set("D", Object::Dictionary(down_dict));
        }

        dict
    }
}

impl Default for AppearanceDictionary {
    fn default() -> Self {
        Self::new()
    }
}

/// Trait for generating appearance streams for different field types
pub trait AppearanceGenerator {
    /// Generate appearance stream for the field
    fn generate_appearance(
        &self,
        widget: &Widget,
        value: Option<&str>,
        state: AppearanceState,
    ) -> Result<AppearanceStream>;
}

/// Text field appearance generator
pub struct TextFieldAppearance {
    /// Font to use
    pub font: Font,
    /// Font size
    pub font_size: f64,
    /// Text color
    pub text_color: Color,
    /// Justification (0=left, 1=center, 2=right)
    pub justification: i32,
    /// Multiline text
    pub multiline: bool,
}

impl Default for TextFieldAppearance {
    fn default() -> Self {
        Self {
            font: Font::Helvetica,
            font_size: 12.0,
            text_color: Color::black(),
            justification: 0,
            multiline: false,
        }
    }
}

impl AppearanceGenerator for TextFieldAppearance {
    fn generate_appearance(
        &self,
        widget: &Widget,
        value: Option<&str>,
        state: AppearanceState,
    ) -> Result<AppearanceStream> {
        let result = self.generate_appearance_with_font(widget, value, state, None)?;
        Ok(result.stream)
    }
}

impl TextFieldAppearance {
    /// Generate the appearance honouring an optional pre-resolved custom
    /// (Type0/CID) font. Returns a [`FieldAppearanceResult`] carrying both
    /// the stream and any characters that the Type0 path consumed from the
    /// font (empty for the built-in path).
    ///
    /// The caller is responsible for supplying `custom_font` when
    /// `self.font == Font::Custom(_)`. If `self.font` is custom but
    /// `custom_font` is `None`, an explicit `PdfError::EncodingError`
    /// signals "font not registered on the Document" (a common user
    /// mistake distinct from the generator not supporting custom fonts).
    pub fn generate_appearance_with_font(
        &self,
        widget: &Widget,
        value: Option<&str>,
        _state: AppearanceState,
        custom_font: Option<&crate::fonts::Font>,
    ) -> Result<FieldAppearanceResult> {
        let width = widget.rect.upper_right.x - widget.rect.lower_left.x;
        let height = widget.rect.upper_right.y - widget.rect.lower_left.y;

        let mut content = String::new();
        let mut used_chars_per_font: HashMap<String, HashSet<char>> = HashMap::new();

        // Save graphics state
        content.push_str("q\n");

        // Draw background if specified
        if let Some(bg_color) = &widget.appearance.background_color {
            match bg_color {
                Color::Gray(g) => content.push_str(&format!("{g} g\n")),
                Color::Rgb(r, g, b) => content.push_str(&format!("{r} {g} {b} rg\n")),
                Color::Cmyk(c, m, y, k) => content.push_str(&format!("{c} {m} {y} {k} k\n")),
            }
            content.push_str(&format!("0 0 {width} {height} re f\n"));
        }

        // Draw border
        if let Some(border_color) = &widget.appearance.border_color {
            match border_color {
                Color::Gray(g) => content.push_str(&format!("{g} G\n")),
                Color::Rgb(r, g, b) => content.push_str(&format!("{r} {g} {b} RG\n")),
                Color::Cmyk(c, m, y, k) => content.push_str(&format!("{c} {m} {y} {k} K\n")),
            }
            content.push_str(&format!("{} w\n", widget.appearance.border_width));

            match widget.appearance.border_style {
                BorderStyle::Solid => {
                    content.push_str(&format!("0 0 {width} {height} re S\n"));
                }
                BorderStyle::Dashed => {
                    content.push_str("[3 2] 0 d\n");
                    content.push_str(&format!("0 0 {width} {height} re S\n"));
                }
                BorderStyle::Beveled | BorderStyle::Inset => {
                    // Simplified beveled/inset border
                    content.push_str(&format!("0 0 {width} {height} re S\n"));
                }
                BorderStyle::Underline => {
                    content.push_str(&format!("0 0 m {width} 0 l S\n"));
                }
            }
        }

        // Draw text if value is provided
        if let Some(text) = value {
            // Set text color
            match self.text_color {
                Color::Gray(g) => content.push_str(&format!("{g} g\n")),
                Color::Rgb(r, g, b) => content.push_str(&format!("{r} {g} {b} rg\n")),
                Color::Cmyk(c, m, y, k) => content.push_str(&format!("{c} {m} {y} {k} k\n")),
            }

            // Begin text
            content.push_str("BT\n");
            content.push_str(&format!(
                "/{} {} Tf\n",
                self.font.pdf_name(),
                self.font_size
            ));

            // Calculate text position
            let padding = 2.0;
            let text_y = (height - self.font_size) / 2.0 + self.font_size * 0.3;

            let text_x = match self.justification {
                1 => width / 2.0,     // Center (would need text width calculation)
                2 => width - padding, // Right
                _ => padding,         // Left
            };

            content.push_str(&format!("{text_x} {text_y} Td\n"));

            // Dispatch on the font kind.
            //
            // `(true, Some)`  → Custom Type0/CID font with a resolved font
            //                   handle. Emits hex-CID Tj and records used
            //                   chars (#204 subsetter invariant).
            // `(true, None)`  → `/DA` names a custom font the Document does
            //                   not have registered. Fail fast with a
            //                   diagnostic that points the caller at the
            //                   likely mistake; silently falling through to
            //                   `emit_tj_for_builtin` would produce an
            //                   opaque "Custom fonts not supported" error
            //                   from a different code path, confusing
            //                   anyone who mistyped a font name.
            // `(false, _)`    → Built-in Type1 path, WinAnsi strict.
            match (self.font.is_custom(), custom_font) {
                (true, Some(cf)) => {
                    let font_name = self.font.pdf_name();
                    let entry = used_chars_per_font.entry(font_name.clone()).or_default();
                    emit_tj_for_custom(&mut content, text, &font_name, cf, entry)?;
                }
                (true, None) => {
                    return Err(PdfError::EncodingError(format!(
                        "Font {:?} is marked as Custom but was not found in the \
                         document registry; call Document::add_font_from_bytes with \
                         this name before fill_field/save. See issue #212.",
                        self.font.pdf_name(),
                    )));
                }
                (false, _) => {
                    emit_tj_for_builtin(&mut content, text, &self.font)?;
                }
            }

            // End text
            content.push_str("ET\n");
        }

        // Restore graphics state
        content.push_str("Q\n");

        // Create resources dictionary — the /Font entry differs by path:
        // - Built-in: emit the current Type1 inline dict (correct as-is).
        // - Custom Type0: emit a placeholder entry whose key matches the
        //   font name; the writer's /AP externalisation pass rewrites it
        //   to an indirect Reference to the document-level Type0 object
        //   (see issue #212 Phase 3).
        let mut resources = Dictionary::new();
        let mut font_dict = Dictionary::new();
        if self.font.is_custom() {
            // Placeholder — real reference is wired in at write-time.
            let mut placeholder = Dictionary::new();
            placeholder.set("Type", Object::Name("Font".to_string()));
            placeholder.set("Subtype", Object::Name("Type0".to_string()));
            placeholder.set("BaseFont", Object::Name(self.font.pdf_name()));
            placeholder.set("Encoding", Object::Name("Identity-H".to_string()));
            font_dict.set(self.font.pdf_name(), Object::Dictionary(placeholder));
        } else {
            let mut font_res = Dictionary::new();
            font_res.set("Type", Object::Name("Font".to_string()));
            font_res.set("Subtype", Object::Name("Type1".to_string()));
            font_res.set("BaseFont", Object::Name(self.font.pdf_name()));
            font_dict.set(self.font.pdf_name(), Object::Dictionary(font_res));
        }
        resources.set("Font", Object::Dictionary(font_dict));

        let stream = AppearanceStream::new(content.into_bytes(), [0.0, 0.0, width, height])
            .with_resources(resources);

        Ok(FieldAppearanceResult {
            stream,
            used_chars_by_font: used_chars_per_font,
        })
    }
}

/// Checkbox appearance generator
pub struct CheckBoxAppearance {
    /// Check mark style
    pub check_style: CheckStyle,
    /// Check color
    pub check_color: Color,
}

/// Style of check mark
#[derive(Debug, Clone, Copy)]
pub enum CheckStyle {
    /// Check mark (✓)
    Check,
    /// Cross (✗)
    Cross,
    /// Square (■)
    Square,
    /// Circle (●)
    Circle,
    /// Star (★)
    Star,
}

impl Default for CheckBoxAppearance {
    fn default() -> Self {
        Self {
            check_style: CheckStyle::Check,
            check_color: Color::black(),
        }
    }
}

impl AppearanceGenerator for CheckBoxAppearance {
    fn generate_appearance(
        &self,
        widget: &Widget,
        value: Option<&str>,
        _state: AppearanceState,
    ) -> Result<AppearanceStream> {
        let width = widget.rect.upper_right.x - widget.rect.lower_left.x;
        let height = widget.rect.upper_right.y - widget.rect.lower_left.y;
        let is_checked = value.is_some_and(|v| v == "Yes" || v == "On" || v == "true");

        let mut content = String::new();

        // Save graphics state
        content.push_str("q\n");

        // Draw background
        if let Some(bg_color) = &widget.appearance.background_color {
            match bg_color {
                Color::Gray(g) => content.push_str(&format!("{g} g\n")),
                Color::Rgb(r, g, b) => content.push_str(&format!("{r} {g} {b} rg\n")),
                Color::Cmyk(c, m, y, k) => content.push_str(&format!("{c} {m} {y} {k} k\n")),
            }
            content.push_str(&format!("0 0 {width} {height} re f\n"));
        }

        // Draw border
        if let Some(border_color) = &widget.appearance.border_color {
            match border_color {
                Color::Gray(g) => content.push_str(&format!("{g} G\n")),
                Color::Rgb(r, g, b) => content.push_str(&format!("{r} {g} {b} RG\n")),
                Color::Cmyk(c, m, y, k) => content.push_str(&format!("{c} {m} {y} {k} K\n")),
            }
            content.push_str(&format!("{} w\n", widget.appearance.border_width));
            content.push_str(&format!("0 0 {width} {height} re S\n"));
        }

        // Draw check mark if checked
        if is_checked {
            // Set check color
            match self.check_color {
                Color::Gray(g) => content.push_str(&format!("{g} g\n")),
                Color::Rgb(r, g, b) => content.push_str(&format!("{r} {g} {b} rg\n")),
                Color::Cmyk(c, m, y, k) => content.push_str(&format!("{c} {m} {y} {k} k\n")),
            }

            let inset = width * 0.2;

            match self.check_style {
                CheckStyle::Check => {
                    // Draw check mark path
                    content.push_str(&format!("{} {} m\n", inset, height * 0.5));
                    content.push_str(&format!("{} {} l\n", width * 0.4, inset));
                    content.push_str(&format!("{} {} l\n", width - inset, height - inset));
                    content.push_str("3 w S\n");
                }
                CheckStyle::Cross => {
                    // Draw X
                    content.push_str(&format!("{inset} {inset} m\n"));
                    content.push_str(&format!("{} {} l\n", width - inset, height - inset));
                    content.push_str(&format!("{} {inset} m\n", width - inset));
                    content.push_str(&format!("{inset} {} l\n", height - inset));
                    content.push_str("2 w S\n");
                }
                CheckStyle::Square => {
                    // Draw filled square
                    content.push_str(&format!(
                        "{inset} {inset} {} {} re f\n",
                        width - 2.0 * inset,
                        height - 2.0 * inset
                    ));
                }
                CheckStyle::Circle => {
                    // Draw filled circle (simplified)
                    let cx = width / 2.0;
                    let cy = height / 2.0;
                    let r = (width.min(height) - 2.0 * inset) / 2.0;

                    // Use Bézier curves to approximate circle
                    let k = 0.552284749831;
                    content.push_str(&format!("{} {} m\n", cx + r, cy));
                    content.push_str(&format!(
                        "{} {} {} {} {} {} c\n",
                        cx + r,
                        cy + k * r,
                        cx + k * r,
                        cy + r,
                        cx,
                        cy + r
                    ));
                    content.push_str(&format!(
                        "{} {} {} {} {} {} c\n",
                        cx - k * r,
                        cy + r,
                        cx - r,
                        cy + k * r,
                        cx - r,
                        cy
                    ));
                    content.push_str(&format!(
                        "{} {} {} {} {} {} c\n",
                        cx - r,
                        cy - k * r,
                        cx - k * r,
                        cy - r,
                        cx,
                        cy - r
                    ));
                    content.push_str(&format!(
                        "{} {} {} {} {} {} c\n",
                        cx + k * r,
                        cy - r,
                        cx + r,
                        cy - k * r,
                        cx + r,
                        cy
                    ));
                    content.push_str("f\n");
                }
                CheckStyle::Star => {
                    // Draw 5-pointed star (simplified)
                    let cx = width / 2.0;
                    let cy = height / 2.0;
                    let r = (width.min(height) - 2.0 * inset) / 2.0;

                    // Star points (simplified)
                    for i in 0..5 {
                        let angle = std::f64::consts::PI * 2.0 * i as f64 / 5.0
                            - std::f64::consts::PI / 2.0;
                        let x = cx + r * angle.cos();
                        let y = cy + r * angle.sin();

                        if i == 0 {
                            content.push_str(&format!("{x} {y} m\n"));
                        } else {
                            content.push_str(&format!("{x} {y} l\n"));
                        }
                    }
                    content.push_str("f\n");
                }
            }
        }

        // Restore graphics state
        content.push_str("Q\n");

        let stream = AppearanceStream::new(content.into_bytes(), [0.0, 0.0, width, height]);

        Ok(stream)
    }
}

/// Radio button appearance generator
pub struct RadioButtonAppearance {
    /// Button color when selected
    pub selected_color: Color,
}

impl Default for RadioButtonAppearance {
    fn default() -> Self {
        Self {
            selected_color: Color::black(),
        }
    }
}

impl AppearanceGenerator for RadioButtonAppearance {
    fn generate_appearance(
        &self,
        widget: &Widget,
        value: Option<&str>,
        _state: AppearanceState,
    ) -> Result<AppearanceStream> {
        let width = widget.rect.upper_right.x - widget.rect.lower_left.x;
        let height = widget.rect.upper_right.y - widget.rect.lower_left.y;
        let is_selected = value.is_some_and(|v| v == "Yes" || v == "On" || v == "true");

        let mut content = String::new();

        // Save graphics state
        content.push_str("q\n");

        // Draw background circle
        if let Some(bg_color) = &widget.appearance.background_color {
            match bg_color {
                Color::Gray(g) => content.push_str(&format!("{g} g\n")),
                Color::Rgb(r, g, b) => content.push_str(&format!("{r} {g} {b} rg\n")),
                Color::Cmyk(c, m, y, k) => content.push_str(&format!("{c} {m} {y} {k} k\n")),
            }
        } else {
            content.push_str("1 g\n"); // White background
        }

        let cx = width / 2.0;
        let cy = height / 2.0;
        let r = width.min(height) / 2.0 - widget.appearance.border_width;

        // Draw outer circle
        let k = 0.552284749831;
        content.push_str(&format!("{} {} m\n", cx + r, cy));
        content.push_str(&format!(
            "{} {} {} {} {} {} c\n",
            cx + r,
            cy + k * r,
            cx + k * r,
            cy + r,
            cx,
            cy + r
        ));
        content.push_str(&format!(
            "{} {} {} {} {} {} c\n",
            cx - k * r,
            cy + r,
            cx - r,
            cy + k * r,
            cx - r,
            cy
        ));
        content.push_str(&format!(
            "{} {} {} {} {} {} c\n",
            cx - r,
            cy - k * r,
            cx - k * r,
            cy - r,
            cx,
            cy - r
        ));
        content.push_str(&format!(
            "{} {} {} {} {} {} c\n",
            cx + k * r,
            cy - r,
            cx + r,
            cy - k * r,
            cx + r,
            cy
        ));
        content.push_str("f\n");

        // Draw border
        if let Some(border_color) = &widget.appearance.border_color {
            match border_color {
                Color::Gray(g) => content.push_str(&format!("{g} G\n")),
                Color::Rgb(r, g, b) => content.push_str(&format!("{r} {g} {b} RG\n")),
                Color::Cmyk(c, m, y, k) => content.push_str(&format!("{c} {m} {y} {k} K\n")),
            }
            content.push_str(&format!("{} w\n", widget.appearance.border_width));

            content.push_str(&format!("{} {} m\n", cx + r, cy));
            content.push_str(&format!(
                "{} {} {} {} {} {} c\n",
                cx + r,
                cy + k * r,
                cx + k * r,
                cy + r,
                cx,
                cy + r
            ));
            content.push_str(&format!(
                "{} {} {} {} {} {} c\n",
                cx - k * r,
                cy + r,
                cx - r,
                cy + k * r,
                cx - r,
                cy
            ));
            content.push_str(&format!(
                "{} {} {} {} {} {} c\n",
                cx - r,
                cy - k * r,
                cx - k * r,
                cy - r,
                cx,
                cy - r
            ));
            content.push_str(&format!(
                "{} {} {} {} {} {} c\n",
                cx + k * r,
                cy - r,
                cx + r,
                cy - k * r,
                cx + r,
                cy
            ));
            content.push_str("S\n");
        }

        // Draw inner dot if selected
        if is_selected {
            match self.selected_color {
                Color::Gray(g) => content.push_str(&format!("{g} g\n")),
                Color::Rgb(r, g, b) => content.push_str(&format!("{r} {g} {b} rg\n")),
                Color::Cmyk(c, m, y, k) => content.push_str(&format!("{c} {m} {y} {k} k\n")),
            }

            let inner_r = r * 0.4;
            content.push_str(&format!("{} {} m\n", cx + inner_r, cy));
            content.push_str(&format!(
                "{} {} {} {} {} {} c\n",
                cx + inner_r,
                cy + k * inner_r,
                cx + k * inner_r,
                cy + inner_r,
                cx,
                cy + inner_r
            ));
            content.push_str(&format!(
                "{} {} {} {} {} {} c\n",
                cx - k * inner_r,
                cy + inner_r,
                cx - inner_r,
                cy + k * inner_r,
                cx - inner_r,
                cy
            ));
            content.push_str(&format!(
                "{} {} {} {} {} {} c\n",
                cx - inner_r,
                cy - k * inner_r,
                cx - k * inner_r,
                cy - inner_r,
                cx,
                cy - inner_r
            ));
            content.push_str(&format!(
                "{} {} {} {} {} {} c\n",
                cx + k * inner_r,
                cy - inner_r,
                cx + inner_r,
                cy - k * inner_r,
                cx + inner_r,
                cy
            ));
            content.push_str("f\n");
        }

        // Restore graphics state
        content.push_str("Q\n");

        let stream = AppearanceStream::new(content.into_bytes(), [0.0, 0.0, width, height]);

        Ok(stream)
    }
}

/// Push button appearance generator
pub struct PushButtonAppearance {
    /// Button label
    pub label: String,
    /// Label font
    pub font: Font,
    /// Font size
    pub font_size: f64,
    /// Text color
    pub text_color: Color,
}

impl Default for PushButtonAppearance {
    fn default() -> Self {
        Self {
            label: String::new(),
            font: Font::Helvetica,
            font_size: 12.0,
            text_color: Color::black(),
        }
    }
}

impl AppearanceGenerator for PushButtonAppearance {
    fn generate_appearance(
        &self,
        widget: &Widget,
        _value: Option<&str>,
        state: AppearanceState,
    ) -> Result<AppearanceStream> {
        let width = widget.rect.upper_right.x - widget.rect.lower_left.x;
        let height = widget.rect.upper_right.y - widget.rect.lower_left.y;

        let mut content = String::new();

        // Save graphics state
        content.push_str("q\n");

        // Draw background with different colors for different states
        let bg_color = match state {
            AppearanceState::Down => Color::gray(0.8),
            AppearanceState::Rollover => Color::gray(0.95),
            AppearanceState::Normal => widget
                .appearance
                .background_color
                .unwrap_or(Color::gray(0.9)),
        };

        match bg_color {
            Color::Gray(g) => content.push_str(&format!("{g} g\n")),
            Color::Rgb(r, g, b) => content.push_str(&format!("{r} {g} {b} rg\n")),
            Color::Cmyk(c, m, y, k) => content.push_str(&format!("{c} {m} {y} {k} k\n")),
        }
        content.push_str(&format!("0 0 {width} {height} re f\n"));

        // Draw beveled border for button appearance
        if matches!(widget.appearance.border_style, BorderStyle::Beveled) {
            // Light edge (top and left)
            content.push_str("0.9 G\n");
            content.push_str("2 w\n");
            content.push_str(&format!("0 {height} m {width} {height} l\n"));
            content.push_str(&format!("{width} {height} l {width} 0 l S\n"));

            // Dark edge (bottom and right)
            content.push_str("0.3 G\n");
            content.push_str(&format!("0 0 m {width} 0 l\n"));
            content.push_str(&format!("0 0 l 0 {height} l S\n"));
        } else {
            // Regular border
            if let Some(border_color) = &widget.appearance.border_color {
                match border_color {
                    Color::Gray(g) => content.push_str(&format!("{g} G\n")),
                    Color::Rgb(r, g, b) => content.push_str(&format!("{r} {g} {b} RG\n")),
                    Color::Cmyk(c, m, y, k) => content.push_str(&format!("{c} {m} {y} {k} K\n")),
                }
                content.push_str(&format!("{} w\n", widget.appearance.border_width));
                content.push_str(&format!("0 0 {width} {height} re S\n"));
            }
        }

        // Draw label text
        if !self.label.is_empty() {
            match self.text_color {
                Color::Gray(g) => content.push_str(&format!("{g} g\n")),
                Color::Rgb(r, g, b) => content.push_str(&format!("{r} {g} {b} rg\n")),
                Color::Cmyk(c, m, y, k) => content.push_str(&format!("{c} {m} {y} {k} k\n")),
            }

            content.push_str("BT\n");
            content.push_str(&format!(
                "/{} {} Tf\n",
                self.font.pdf_name(),
                self.font_size
            ));

            // Center text (simplified - would need actual text width calculation)
            let text_x = width / 4.0; // Approximate centering
            let text_y = (height - self.font_size) / 2.0 + self.font_size * 0.3;

            content.push_str(&format!("{text_x} {text_y} Td\n"));

            // Same contract as TextFieldAppearance: built-in Type1 → WinAnsi
            // strict encoding, explicit error for anything outside the
            // repertoire. Applies to the button label.
            emit_tj_for_builtin(&mut content, &self.label, &self.font)?;

            content.push_str("ET\n");
        }

        // Restore graphics state
        content.push_str("Q\n");

        // Create resources dictionary
        let mut resources = Dictionary::new();

        // Add font resource
        let mut font_dict = Dictionary::new();
        let mut font_res = Dictionary::new();
        font_res.set("Type", Object::Name("Font".to_string()));
        font_res.set("Subtype", Object::Name("Type1".to_string()));
        font_res.set("BaseFont", Object::Name(self.font.pdf_name()));
        font_dict.set(self.font.pdf_name(), Object::Dictionary(font_res));
        resources.set("Font", Object::Dictionary(font_dict));

        let stream = AppearanceStream::new(content.into_bytes(), [0.0, 0.0, width, height])
            .with_resources(resources);

        Ok(stream)
    }
}

/// Appearance generator for ComboBox fields
#[derive(Debug, Clone)]
pub struct ComboBoxAppearance {
    /// Font for text
    pub font: Font,
    /// Font size
    pub font_size: f64,
    /// Text color
    pub text_color: Color,
    /// Selected option
    pub selected_text: Option<String>,
    /// Show dropdown arrow
    pub show_arrow: bool,
}

impl Default for ComboBoxAppearance {
    fn default() -> Self {
        Self {
            font: Font::Helvetica,
            font_size: 12.0,
            text_color: Color::black(),
            selected_text: None,
            show_arrow: true,
        }
    }
}

impl AppearanceGenerator for ComboBoxAppearance {
    fn generate_appearance(
        &self,
        widget: &Widget,
        value: Option<&str>,
        state: AppearanceState,
    ) -> Result<AppearanceStream> {
        let result = self.generate_appearance_with_font(widget, value, state, None)?;
        Ok(result.stream)
    }
}

impl ComboBoxAppearance {
    /// Parallel to [`TextFieldAppearance::generate_appearance_with_font`].
    /// When `self.font == Font::Custom(name)` AND `custom_font` is supplied,
    /// emits a hex-CID `<HHHH...> Tj` with a Type0 placeholder resource; the
    /// writer rewrites the placeholder into an indirect Reference to the
    /// document-level font object. Same contract on the
    /// `Custom + None` case: fails fast with a "font not registered"
    /// `PdfError::EncodingError` instead of silently falling back.
    pub fn generate_appearance_with_font(
        &self,
        widget: &Widget,
        value: Option<&str>,
        _state: AppearanceState,
        custom_font: Option<&crate::fonts::Font>,
    ) -> Result<FieldAppearanceResult> {
        let width = widget.rect.upper_right.x - widget.rect.lower_left.x;
        let height = widget.rect.upper_right.y - widget.rect.lower_left.y;

        let mut content = String::new();
        let mut used_chars_per_font: HashMap<String, HashSet<char>> = HashMap::new();

        // Draw background
        content.push_str("1 1 1 rg\n"); // White background
        content.push_str(&format!("0 0 {} {} re\n", width, height));
        content.push_str("f\n");

        // Draw border
        if let Some(ref border_color) = widget.appearance.border_color {
            match border_color {
                Color::Gray(g) => content.push_str(&format!("{} G\n", g)),
                Color::Rgb(r, g, b) => content.push_str(&format!("{} {} {} RG\n", r, g, b)),
                Color::Cmyk(c, m, y, k) => {
                    content.push_str(&format!("{} {} {} {} K\n", c, m, y, k))
                }
            }
            content.push_str(&format!("{} w\n", widget.appearance.border_width));
            content.push_str(&format!("0 0 {} {} re\n", width, height));
            content.push_str("S\n");
        }

        // Draw dropdown arrow if enabled
        if self.show_arrow {
            let arrow_x = width - 15.0;
            let arrow_y = height / 2.0;
            content.push_str("0.5 0.5 0.5 rg\n"); // Gray arrow
            content.push_str(&format!("{} {} m\n", arrow_x, arrow_y + 3.0));
            content.push_str(&format!("{} {} l\n", arrow_x + 8.0, arrow_y + 3.0));
            content.push_str(&format!("{} {} l\n", arrow_x + 4.0, arrow_y - 3.0));
            content.push_str("f\n");
        }

        // Draw selected text
        let text_to_show = value.or(self.selected_text.as_deref());
        if let Some(text) = text_to_show {
            content.push_str("BT\n");
            content.push_str(&format!(
                "/{} {} Tf\n",
                self.font.pdf_name(),
                self.font_size
            ));
            match self.text_color {
                Color::Gray(g) => content.push_str(&format!("{} g\n", g)),
                Color::Rgb(r, g, b) => content.push_str(&format!("{} {} {} rg\n", r, g, b)),
                Color::Cmyk(c, m, y, k) => {
                    content.push_str(&format!("{} {} {} {} k\n", c, m, y, k))
                }
            }
            content.push_str(&format!("5 {} Td\n", (height - self.font_size) / 2.0));

            // Same dispatch as `TextFieldAppearance::generate_appearance_with_font`
            // — see that function for the rationale on the explicit
            // `(true, None)` arm. Combo boxes reach this path when the field's
            // `/DA` picks a custom font and the user renders the selected value.
            match (self.font.is_custom(), custom_font) {
                (true, Some(cf)) => {
                    let font_name = self.font.pdf_name();
                    let entry = used_chars_per_font.entry(font_name.clone()).or_default();
                    emit_tj_for_custom(&mut content, text, &font_name, cf, entry)?;
                }
                (true, None) => {
                    return Err(PdfError::EncodingError(format!(
                        "Font {:?} is marked as Custom but was not found in the \
                         document registry; call Document::add_font_from_bytes with \
                         this name before fill_field/save. See issue #212.",
                        self.font.pdf_name(),
                    )));
                }
                (false, _) => emit_tj_for_builtin(&mut content, text, &self.font)?,
            }
            content.push_str("ET\n");
        }

        // Resources dict parallels the TextField path: custom → Type0
        // placeholder (writer-rewritten), builtin → Type1 inline.
        let mut resources = Dictionary::new();
        let mut font_dict = Dictionary::new();
        if self.font.is_custom() {
            let mut placeholder = Dictionary::new();
            placeholder.set("Type", Object::Name("Font".to_string()));
            placeholder.set("Subtype", Object::Name("Type0".to_string()));
            placeholder.set("BaseFont", Object::Name(self.font.pdf_name()));
            placeholder.set("Encoding", Object::Name("Identity-H".to_string()));
            font_dict.set(self.font.pdf_name(), Object::Dictionary(placeholder));
        } else {
            let mut font_res = Dictionary::new();
            font_res.set("Type", Object::Name("Font".to_string()));
            font_res.set("Subtype", Object::Name("Type1".to_string()));
            font_res.set("BaseFont", Object::Name(self.font.pdf_name()));
            font_dict.set(self.font.pdf_name(), Object::Dictionary(font_res));
        }
        resources.set("Font", Object::Dictionary(font_dict));

        let bbox = [0.0, 0.0, width, height];
        let stream = AppearanceStream::new(content.into_bytes(), bbox).with_resources(resources);
        Ok(FieldAppearanceResult {
            stream,
            used_chars_by_font: used_chars_per_font,
        })
    }
}

/// Appearance generator for ListBox fields
#[derive(Debug, Clone)]
pub struct ListBoxAppearance {
    /// Font for text
    pub font: Font,
    /// Font size
    pub font_size: f64,
    /// Text color
    pub text_color: Color,
    /// Background color for selected items
    pub selection_color: Color,
    /// Options to display
    pub options: Vec<String>,
    /// Selected indices
    pub selected: Vec<usize>,
    /// Item height
    pub item_height: f64,
}

impl Default for ListBoxAppearance {
    fn default() -> Self {
        Self {
            font: Font::Helvetica,
            font_size: 12.0,
            text_color: Color::black(),
            selection_color: Color::rgb(0.2, 0.4, 0.8),
            options: Vec::new(),
            selected: Vec::new(),
            item_height: 16.0,
        }
    }
}

impl AppearanceGenerator for ListBoxAppearance {
    fn generate_appearance(
        &self,
        widget: &Widget,
        _value: Option<&str>,
        _state: AppearanceState,
    ) -> Result<AppearanceStream> {
        let width = widget.rect.upper_right.x - widget.rect.lower_left.x;
        let height = widget.rect.upper_right.y - widget.rect.lower_left.y;

        let mut content = String::new();

        // Draw background
        content.push_str("1 1 1 rg\n"); // White background
        content.push_str(&format!("0 0 {} {} re\n", width, height));
        content.push_str("f\n");

        // Draw border
        if let Some(ref border_color) = widget.appearance.border_color {
            match border_color {
                Color::Gray(g) => content.push_str(&format!("{} G\n", g)),
                Color::Rgb(r, g, b) => content.push_str(&format!("{} {} {} RG\n", r, g, b)),
                Color::Cmyk(c, m, y, k) => {
                    content.push_str(&format!("{} {} {} {} K\n", c, m, y, k))
                }
            }
            content.push_str(&format!("{} w\n", widget.appearance.border_width));
            content.push_str(&format!("0 0 {} {} re\n", width, height));
            content.push_str("S\n");
        }

        // Draw list items
        let mut y = height - self.item_height;
        for (index, option) in self.options.iter().enumerate() {
            if y < 0.0 {
                break; // Stop if we've filled the visible area
            }

            // Draw selection background if selected
            if self.selected.contains(&index) {
                match self.selection_color {
                    Color::Gray(g) => content.push_str(&format!("{} g\n", g)),
                    Color::Rgb(r, g, b) => content.push_str(&format!("{} {} {} rg\n", r, g, b)),
                    Color::Cmyk(c, m, y_val, k) => {
                        content.push_str(&format!("{} {} {} {} k\n", c, m, y_val, k))
                    }
                }
                content.push_str(&format!("0 {} {} {} re\n", y, width, self.item_height));
                content.push_str("f\n");
            }

            // Draw text
            content.push_str("BT\n");
            content.push_str(&format!(
                "/{} {} Tf\n",
                self.font.pdf_name(),
                self.font_size
            ));

            // Use white text for selected items, black for others
            if self.selected.contains(&index) {
                content.push_str("1 1 1 rg\n");
            } else {
                match self.text_color {
                    Color::Gray(g) => content.push_str(&format!("{} g\n", g)),
                    Color::Rgb(r, g, b) => content.push_str(&format!("{} {} {} rg\n", r, g, b)),
                    Color::Cmyk(c, m, y_val, k) => {
                        content.push_str(&format!("{} {} {} {} k\n", c, m, y_val, k))
                    }
                }
            }

            content.push_str(&format!("5 {} Td\n", y + 2.0));

            emit_tj_for_builtin(&mut content, option, &self.font)?;
            content.push_str("ET\n");

            y -= self.item_height;
        }

        let bbox = [0.0, 0.0, width, height];
        Ok(AppearanceStream::new(content.into_bytes(), bbox))
    }
}

/// Generate default appearance stream for a field type.
///
/// Kept for API stability — delegates to [`generate_field_appearance`] with
/// no `/DA` override and no custom-font context, which is the only behaviour
/// this function could ever offer. Prefer the richer signature of
/// [`generate_field_appearance`] when you need non-WinAnsi values or a
/// per-field font.
pub fn generate_default_appearance(
    field_type: FieldType,
    widget: &Widget,
    value: Option<&str>,
) -> Result<AppearanceStream> {
    Ok(generate_field_appearance(field_type, widget, value, None, None)?.stream)
}

/// Generate an appearance stream honouring an optional typed `/DA` and an
/// optional resolved custom font.
///
/// Returns both the stream and the set of characters consumed from the
/// custom font (empty for built-in fonts) — callers (typically
/// `Document::fill_field`) merge the latter into
/// `Document::used_characters_by_font` so the font subsetter emits a subset
/// that covers the appearance content (same invariant as issue #204).
///
/// Dispatch:
/// - `default_appearance.font == Font::Custom(name)` AND `custom_font` is
///   `Some(...)` → **Type0/CID path**. Content stream uses hex glyph-index
///   Tj, resources dict carries a placeholder `/Type0` entry that the
///   writer rewrites to an indirect Reference to the document-level font
///   object (see [`writer::pdf_writer`]).
/// - Anything else → **built-in / WinAnsi path** (existing behaviour, now
///   with strict encoding so non-WinAnsi values fail explicitly instead of
///   being silently corrupted).
pub fn generate_field_appearance(
    field_type: FieldType,
    widget: &Widget,
    value: Option<&str>,
    default_appearance: Option<&DefaultAppearance>,
    custom_font: Option<&crate::fonts::Font>,
) -> Result<FieldAppearanceResult> {
    match field_type {
        FieldType::Text => {
            let mut generator = TextFieldAppearance::default();
            if let Some(da) = default_appearance {
                generator.font = da.font.clone();
                generator.font_size = da.font_size;
                generator.text_color = da.color.clone();
            }
            generator.generate_appearance_with_font(
                widget,
                value,
                AppearanceState::Normal,
                custom_font,
            )
        }
        FieldType::Button => {
            // Default button appearance (checkbox-style) does not consume a
            // user-supplied `/DA` today — the button glyphs are synthesised.
            let generator = CheckBoxAppearance::default();
            let stream = generator.generate_appearance(widget, value, AppearanceState::Normal)?;
            Ok(FieldAppearanceResult {
                stream,
                used_chars_by_font: HashMap::new(),
            })
        }
        FieldType::Choice => {
            let mut generator = ComboBoxAppearance::default();
            if let Some(da) = default_appearance {
                generator.font = da.font.clone();
                generator.font_size = da.font_size;
                generator.text_color = da.color.clone();
            }
            generator.generate_appearance_with_font(
                widget,
                value,
                AppearanceState::Normal,
                custom_font,
            )
        }
        FieldType::Signature => {
            let width = widget.rect.upper_right.x - widget.rect.lower_left.x;
            let height = widget.rect.upper_right.y - widget.rect.lower_left.y;
            Ok(FieldAppearanceResult {
                stream: AppearanceStream::new(b"q\nQ\n".to_vec(), [0.0, 0.0, width, height]),
                used_chars_by_font: HashMap::new(),
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry::{Point, Rectangle};

    #[test]
    fn test_appearance_state_names() {
        assert_eq!(AppearanceState::Normal.pdf_name(), "N");
        assert_eq!(AppearanceState::Rollover.pdf_name(), "R");
        assert_eq!(AppearanceState::Down.pdf_name(), "D");
    }

    #[test]
    fn test_appearance_stream_creation() {
        let content = b"q\n1 0 0 RG\n0 0 100 50 re S\nQ\n";
        let stream = AppearanceStream::new(content.to_vec(), [0.0, 0.0, 100.0, 50.0]);

        assert_eq!(stream.content, content);
        assert_eq!(stream.bbox, [0.0, 0.0, 100.0, 50.0]);
        assert!(stream.resources.is_empty());
    }

    #[test]
    fn test_appearance_stream_with_resources() {
        let mut resources = Dictionary::new();
        resources.set("Font", Object::Name("F1".to_string()));

        let content = b"BT\n/F1 12 Tf\n(Test) Tj\nET\n";
        let stream = AppearanceStream::new(content.to_vec(), [0.0, 0.0, 100.0, 50.0])
            .with_resources(resources.clone());

        assert_eq!(stream.resources, resources);
    }

    #[test]
    fn test_appearance_dictionary() {
        let mut app_dict = AppearanceDictionary::new();

        let normal_stream = AppearanceStream::new(b"normal".to_vec(), [0.0, 0.0, 10.0, 10.0]);
        let down_stream = AppearanceStream::new(b"down".to_vec(), [0.0, 0.0, 10.0, 10.0]);

        app_dict.set_appearance(AppearanceState::Normal, normal_stream);
        app_dict.set_appearance(AppearanceState::Down, down_stream);

        assert!(app_dict.get_appearance(AppearanceState::Normal).is_some());
        assert!(app_dict.get_appearance(AppearanceState::Down).is_some());
        assert!(app_dict.get_appearance(AppearanceState::Rollover).is_none());
    }

    #[test]
    fn test_text_field_appearance() {
        let widget = Widget::new(Rectangle {
            lower_left: Point { x: 0.0, y: 0.0 },
            upper_right: Point { x: 200.0, y: 30.0 },
        });

        let generator = TextFieldAppearance::default();
        let result =
            generator.generate_appearance(&widget, Some("Test Text"), AppearanceState::Normal);

        assert!(result.is_ok());
        let stream = result.unwrap();
        assert_eq!(stream.bbox, [0.0, 0.0, 200.0, 30.0]);

        let content = String::from_utf8_lossy(&stream.content);
        assert!(content.contains("BT"));
        assert!(content.contains("(Test Text) Tj"));
        assert!(content.contains("ET"));
    }

    #[test]
    fn test_checkbox_appearance_checked() {
        let widget = Widget::new(Rectangle {
            lower_left: Point { x: 0.0, y: 0.0 },
            upper_right: Point { x: 20.0, y: 20.0 },
        });

        let generator = CheckBoxAppearance::default();
        let result = generator.generate_appearance(&widget, Some("Yes"), AppearanceState::Normal);

        assert!(result.is_ok());
        let stream = result.unwrap();
        let content = String::from_utf8_lossy(&stream.content);

        // Should contain check mark drawing commands
        assert!(content.contains(" m"));
        assert!(content.contains(" l"));
        assert!(content.contains(" S"));
    }

    #[test]
    fn test_checkbox_appearance_unchecked() {
        let widget = Widget::new(Rectangle {
            lower_left: Point { x: 0.0, y: 0.0 },
            upper_right: Point { x: 20.0, y: 20.0 },
        });

        let generator = CheckBoxAppearance::default();
        let result = generator.generate_appearance(&widget, Some("No"), AppearanceState::Normal);

        assert!(result.is_ok());
        let stream = result.unwrap();
        let content = String::from_utf8_lossy(&stream.content);

        // Should not contain complex drawing for check mark
        assert!(content.contains("q"));
        assert!(content.contains("Q"));
    }

    #[test]
    fn test_radio_button_appearance() {
        let widget = Widget::new(Rectangle {
            lower_left: Point { x: 0.0, y: 0.0 },
            upper_right: Point { x: 20.0, y: 20.0 },
        });

        let generator = RadioButtonAppearance::default();
        let result = generator.generate_appearance(&widget, Some("Yes"), AppearanceState::Normal);

        assert!(result.is_ok());
        let stream = result.unwrap();
        let content = String::from_utf8_lossy(&stream.content);

        // Should contain circle drawing commands (Bézier curves)
        assert!(
            content.contains(" c"),
            "Content should contain curve commands"
        );
        assert!(
            content.contains("f\n"),
            "Content should contain fill commands"
        );
    }

    #[test]
    fn test_push_button_appearance() {
        let mut generator = PushButtonAppearance::default();
        generator.label = "Click Me".to_string();

        let widget = Widget::new(Rectangle {
            lower_left: Point { x: 0.0, y: 0.0 },
            upper_right: Point { x: 100.0, y: 30.0 },
        });

        let result = generator.generate_appearance(&widget, None, AppearanceState::Normal);

        assert!(result.is_ok());
        let stream = result.unwrap();
        let content = String::from_utf8_lossy(&stream.content);

        assert!(content.contains("(Click Me) Tj"));
        assert!(!stream.resources.is_empty());
    }

    #[test]
    fn test_push_button_states() {
        let generator = PushButtonAppearance::default();
        let widget = Widget::new(Rectangle {
            lower_left: Point { x: 0.0, y: 0.0 },
            upper_right: Point { x: 100.0, y: 30.0 },
        });

        // Test different states produce different appearances
        let normal = generator
            .generate_appearance(&widget, None, AppearanceState::Normal)
            .unwrap();
        let down = generator
            .generate_appearance(&widget, None, AppearanceState::Down)
            .unwrap();
        let rollover = generator
            .generate_appearance(&widget, None, AppearanceState::Rollover)
            .unwrap();

        // Content should be different for different states (different background colors)
        assert_ne!(normal.content, down.content);
        assert_ne!(normal.content, rollover.content);
        assert_ne!(down.content, rollover.content);
    }

    #[test]
    fn test_check_styles() {
        let widget = Widget::new(Rectangle {
            lower_left: Point { x: 0.0, y: 0.0 },
            upper_right: Point { x: 20.0, y: 20.0 },
        });

        // Test different check styles
        for style in [
            CheckStyle::Check,
            CheckStyle::Cross,
            CheckStyle::Square,
            CheckStyle::Circle,
            CheckStyle::Star,
        ] {
            let mut generator = CheckBoxAppearance::default();
            generator.check_style = style;

            let result =
                generator.generate_appearance(&widget, Some("Yes"), AppearanceState::Normal);

            assert!(result.is_ok(), "Failed for style {:?}", style);
        }
    }

    #[test]
    fn test_appearance_state_pdf_names() {
        assert_eq!(AppearanceState::Normal.pdf_name(), "N");
        assert_eq!(AppearanceState::Rollover.pdf_name(), "R");
        assert_eq!(AppearanceState::Down.pdf_name(), "D");
    }

    #[test]
    fn test_appearance_stream_creation_advanced() {
        let content = b"q 1 0 0 1 0 0 cm Q".to_vec();
        let bbox = [0.0, 0.0, 100.0, 50.0];
        let stream = AppearanceStream::new(content.clone(), bbox);

        assert_eq!(stream.content, content);
        assert_eq!(stream.bbox, bbox);
        assert!(stream.resources.is_empty());
    }

    #[test]
    fn test_appearance_stream_with_resources_advanced() {
        let mut resources = Dictionary::new();
        resources.set("Font", Object::Dictionary(Dictionary::new()));

        let stream =
            AppearanceStream::new(vec![], [0.0, 0.0, 10.0, 10.0]).with_resources(resources.clone());

        assert_eq!(stream.resources, resources);
    }

    #[test]
    fn test_appearance_dictionary_new() {
        let dict = AppearanceDictionary::new();
        assert!(dict.appearances.is_empty());
        assert!(dict.down_appearances.is_empty());
    }

    #[test]
    fn test_appearance_dictionary_set_get() {
        let mut dict = AppearanceDictionary::new();
        let stream = AppearanceStream::new(vec![1, 2, 3], [0.0, 0.0, 10.0, 10.0]);

        dict.set_appearance(AppearanceState::Normal, stream);
        assert!(dict.get_appearance(AppearanceState::Normal).is_some());
        assert!(dict.get_appearance(AppearanceState::Down).is_none());
    }

    #[test]
    fn test_text_field_multiline() {
        let mut generator = TextFieldAppearance::default();
        generator.multiline = true;

        let widget = Widget::new(Rectangle {
            lower_left: Point { x: 0.0, y: 0.0 },
            upper_right: Point { x: 200.0, y: 100.0 },
        });

        let text = "Line 1\nLine 2\nLine 3";
        let result = generator.generate_appearance(&widget, Some(text), AppearanceState::Normal);
        assert!(result.is_ok());
    }

    #[test]
    fn test_appearance_with_custom_colors() {
        let mut generator = TextFieldAppearance::default();
        generator.text_color = Color::rgb(1.0, 0.0, 0.0); // Red text
        generator.font_size = 14.0;
        generator.justification = 1; // center

        let widget = Widget::new(Rectangle {
            lower_left: Point { x: 0.0, y: 0.0 },
            upper_right: Point { x: 100.0, y: 30.0 },
        });

        let result =
            generator.generate_appearance(&widget, Some("Colored"), AppearanceState::Normal);
        assert!(result.is_ok());
    }
}
