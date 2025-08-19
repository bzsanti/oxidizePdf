pub mod calibrated_color;
pub mod clipping;
mod color;
mod color_profiles;
pub mod devicen_color;
pub mod form_xobject;
mod indexed_color;
pub mod lab_color;
mod path;
mod patterns;
mod pdf_image;
mod png_decoder;
pub mod separation_color;
mod shadings;
pub mod soft_mask;
pub mod state;
pub mod transparency;

pub use calibrated_color::{CalGrayColorSpace, CalRgbColorSpace, CalibratedColor};
pub use clipping::{ClippingPath, ClippingRegion};
pub use color::Color;
pub use color_profiles::{IccColorSpace, IccProfile, IccProfileManager, StandardIccProfile};
pub use devicen_color::{
    AlternateColorSpace as DeviceNAlternateColorSpace, ColorantDefinition, ColorantType,
    DeviceNAttributes, DeviceNColorSpace, LinearTransform, SampledFunction, TintTransformFunction,
};
pub use form_xobject::{
    FormTemplates, FormXObject, FormXObjectBuilder, FormXObjectManager,
    TransparencyGroup as FormTransparencyGroup,
};
pub use indexed_color::{BaseColorSpace, ColorLookupTable, IndexedColorManager, IndexedColorSpace};
pub use lab_color::{LabColor, LabColorSpace};
pub use path::{LineCap, LineJoin, PathBuilder, PathCommand, WindingRule};
pub use patterns::{
    PaintType, PatternGraphicsContext, PatternManager, PatternMatrix, PatternType, TilingPattern,
    TilingType,
};
pub use pdf_image::{ColorSpace, Image, ImageFormat, MaskType};
pub use separation_color::{
    AlternateColorSpace, SeparationColor, SeparationColorSpace, SpotColors, TintTransform,
};
pub use shadings::{
    AxialShading, ColorStop, FunctionBasedShading, Point, RadialShading, ShadingDefinition,
    ShadingManager, ShadingPattern, ShadingType,
};
pub use soft_mask::{SoftMask, SoftMaskState, SoftMaskType};
pub use state::{
    BlendMode, ExtGState, ExtGStateFont, ExtGStateManager, Halftone, LineDashPattern,
    RenderingIntent, TransferFunction,
};
pub use transparency::TransparencyGroup;
use transparency::TransparencyGroupState;

use crate::error::Result;
use crate::text::{ColumnContent, ColumnLayout, Font, FontManager, ListElement, Table};
use std::collections::{HashMap, HashSet};
use std::fmt::Write;
use std::sync::Arc;

#[derive(Clone)]
pub struct GraphicsContext {
    operations: String,
    current_color: Color,
    stroke_color: Color,
    line_width: f64,
    fill_opacity: f64,
    stroke_opacity: f64,
    // Extended Graphics State support
    extgstate_manager: ExtGStateManager,
    pending_extgstate: Option<ExtGState>,
    current_dash_pattern: Option<LineDashPattern>,
    current_miter_limit: f64,
    current_line_cap: LineCap,
    current_line_join: LineJoin,
    current_rendering_intent: RenderingIntent,
    current_flatness: f64,
    current_smoothness: f64,
    // Clipping support
    clipping_region: ClippingRegion,
    // Font management
    font_manager: Option<Arc<FontManager>>,
    // State stack for save/restore
    state_stack: Vec<(Color, Color)>,
    current_font_name: Option<String>,
    current_font_size: f64,
    // Character tracking for font subsetting
    used_characters: HashSet<char>,
    // Glyph mapping for Unicode fonts (Unicode code point -> Glyph ID)
    glyph_mapping: Option<HashMap<u32, u16>>,
    // Transparency group stack for nested groups
    transparency_stack: Vec<TransparencyGroupState>,
}

impl Default for GraphicsContext {
    fn default() -> Self {
        Self::new()
    }
}

impl GraphicsContext {
    pub fn new() -> Self {
        Self {
            operations: String::new(),
            current_color: Color::black(),
            stroke_color: Color::black(),
            line_width: 1.0,
            fill_opacity: 1.0,
            stroke_opacity: 1.0,
            // Extended Graphics State defaults
            extgstate_manager: ExtGStateManager::new(),
            pending_extgstate: None,
            current_dash_pattern: None,
            current_miter_limit: 10.0,
            current_line_cap: LineCap::Butt,
            current_line_join: LineJoin::Miter,
            current_rendering_intent: RenderingIntent::RelativeColorimetric,
            current_flatness: 1.0,
            current_smoothness: 0.0,
            // Clipping defaults
            clipping_region: ClippingRegion::new(),
            // Font defaults
            font_manager: None,
            state_stack: Vec::new(),
            current_font_name: None,
            current_font_size: 12.0,
            used_characters: HashSet::new(),
            glyph_mapping: None,
            transparency_stack: Vec::new(),
        }
    }

    pub fn move_to(&mut self, x: f64, y: f64) -> &mut Self {
        writeln!(&mut self.operations, "{x:.2} {y:.2} m").unwrap();
        self
    }

    pub fn line_to(&mut self, x: f64, y: f64) -> &mut Self {
        writeln!(&mut self.operations, "{x:.2} {y:.2} l").unwrap();
        self
    }

    pub fn curve_to(&mut self, x1: f64, y1: f64, x2: f64, y2: f64, x3: f64, y3: f64) -> &mut Self {
        writeln!(
            &mut self.operations,
            "{x1:.2} {y1:.2} {x2:.2} {y2:.2} {x3:.2} {y3:.2} c"
        )
        .unwrap();
        self
    }

    pub fn rect(&mut self, x: f64, y: f64, width: f64, height: f64) -> &mut Self {
        writeln!(
            &mut self.operations,
            "{x:.2} {y:.2} {width:.2} {height:.2} re"
        )
        .unwrap();
        self
    }

    pub fn circle(&mut self, cx: f64, cy: f64, radius: f64) -> &mut Self {
        let k = 0.552284749831;
        let r = radius;

        self.move_to(cx + r, cy);
        self.curve_to(cx + r, cy + k * r, cx + k * r, cy + r, cx, cy + r);
        self.curve_to(cx - k * r, cy + r, cx - r, cy + k * r, cx - r, cy);
        self.curve_to(cx - r, cy - k * r, cx - k * r, cy - r, cx, cy - r);
        self.curve_to(cx + k * r, cy - r, cx + r, cy - k * r, cx + r, cy);
        self.close_path()
    }

    pub fn close_path(&mut self) -> &mut Self {
        self.operations.push_str("h\n");
        self
    }

    pub fn stroke(&mut self) -> &mut Self {
        self.apply_pending_extgstate().unwrap_or_default();
        self.apply_stroke_color();
        self.operations.push_str("S\n");
        self
    }

    pub fn fill(&mut self) -> &mut Self {
        self.apply_pending_extgstate().unwrap_or_default();
        self.apply_fill_color();
        self.operations.push_str("f\n");
        self
    }

    pub fn fill_stroke(&mut self) -> &mut Self {
        self.apply_pending_extgstate().unwrap_or_default();
        self.apply_fill_color();
        self.apply_stroke_color();
        self.operations.push_str("B\n");
        self
    }

    pub fn set_stroke_color(&mut self, color: Color) -> &mut Self {
        self.stroke_color = color;
        self
    }

    pub fn set_fill_color(&mut self, color: Color) -> &mut Self {
        self.current_color = color;
        self
    }

    /// Set fill color using calibrated color space
    pub fn set_fill_color_calibrated(&mut self, color: CalibratedColor) -> &mut Self {
        // Generate a unique color space name
        let cs_name = match &color {
            CalibratedColor::Gray(_, _) => "CalGray1",
            CalibratedColor::Rgb(_, _) => "CalRGB1",
        };

        // Set the color space (this would need to be registered in the PDF resources)
        writeln!(&mut self.operations, "/{} cs", cs_name).unwrap();

        // Set color values
        let values = color.values();
        for value in &values {
            write!(&mut self.operations, "{:.4} ", value).unwrap();
        }
        writeln!(&mut self.operations, "sc").unwrap();

        self
    }

    /// Set stroke color using calibrated color space
    pub fn set_stroke_color_calibrated(&mut self, color: CalibratedColor) -> &mut Self {
        // Generate a unique color space name
        let cs_name = match &color {
            CalibratedColor::Gray(_, _) => "CalGray1",
            CalibratedColor::Rgb(_, _) => "CalRGB1",
        };

        // Set the color space (this would need to be registered in the PDF resources)
        writeln!(&mut self.operations, "/{} CS", cs_name).unwrap();

        // Set color values
        let values = color.values();
        for value in &values {
            write!(&mut self.operations, "{:.4} ", value).unwrap();
        }
        writeln!(&mut self.operations, "SC").unwrap();

        self
    }

    /// Set fill color using Lab color space
    pub fn set_fill_color_lab(&mut self, color: LabColor) -> &mut Self {
        // Set the color space (this would need to be registered in the PDF resources)
        writeln!(&mut self.operations, "/Lab1 cs").unwrap();

        // Set color values (normalized for PDF)
        let values = color.values();
        for value in &values {
            write!(&mut self.operations, "{:.4} ", value).unwrap();
        }
        writeln!(&mut self.operations, "sc").unwrap();

        self
    }

    /// Set stroke color using Lab color space
    pub fn set_stroke_color_lab(&mut self, color: LabColor) -> &mut Self {
        // Set the color space (this would need to be registered in the PDF resources)
        writeln!(&mut self.operations, "/Lab1 CS").unwrap();

        // Set color values (normalized for PDF)
        let values = color.values();
        for value in &values {
            write!(&mut self.operations, "{:.4} ", value).unwrap();
        }
        writeln!(&mut self.operations, "SC").unwrap();

        self
    }

    pub fn set_line_width(&mut self, width: f64) -> &mut Self {
        self.line_width = width;
        writeln!(&mut self.operations, "{width:.2} w").unwrap();
        self
    }

    pub fn set_line_cap(&mut self, cap: LineCap) -> &mut Self {
        self.current_line_cap = cap;
        writeln!(&mut self.operations, "{} J", cap as u8).unwrap();
        self
    }

    pub fn set_line_join(&mut self, join: LineJoin) -> &mut Self {
        self.current_line_join = join;
        writeln!(&mut self.operations, "{} j", join as u8).unwrap();
        self
    }

    /// Set the opacity for both fill and stroke operations (0.0 to 1.0)
    pub fn set_opacity(&mut self, opacity: f64) -> &mut Self {
        let opacity = opacity.clamp(0.0, 1.0);
        self.fill_opacity = opacity;
        self.stroke_opacity = opacity;

        // Create pending ExtGState if opacity is not 1.0
        if opacity < 1.0 {
            let mut state = ExtGState::new();
            state.alpha_fill = Some(opacity);
            state.alpha_stroke = Some(opacity);
            self.pending_extgstate = Some(state);
        }

        self
    }

    /// Set the fill opacity (0.0 to 1.0)
    pub fn set_fill_opacity(&mut self, opacity: f64) -> &mut Self {
        self.fill_opacity = opacity.clamp(0.0, 1.0);

        // Update or create pending ExtGState
        if opacity < 1.0 {
            if let Some(ref mut state) = self.pending_extgstate {
                state.alpha_fill = Some(opacity);
            } else {
                let mut state = ExtGState::new();
                state.alpha_fill = Some(opacity);
                self.pending_extgstate = Some(state);
            }
        }

        self
    }

    /// Set the stroke opacity (0.0 to 1.0)
    pub fn set_stroke_opacity(&mut self, opacity: f64) -> &mut Self {
        self.stroke_opacity = opacity.clamp(0.0, 1.0);

        // Update or create pending ExtGState
        if opacity < 1.0 {
            if let Some(ref mut state) = self.pending_extgstate {
                state.alpha_stroke = Some(opacity);
            } else {
                let mut state = ExtGState::new();
                state.alpha_stroke = Some(opacity);
                self.pending_extgstate = Some(state);
            }
        }

        self
    }

    pub fn save_state(&mut self) -> &mut Self {
        self.operations.push_str("q\n");
        self.save_clipping_state();
        // Save color state
        self.state_stack
            .push((self.current_color, self.stroke_color));
        self
    }

    pub fn restore_state(&mut self) -> &mut Self {
        self.operations.push_str("Q\n");
        self.restore_clipping_state();
        // Restore color state
        if let Some((fill, stroke)) = self.state_stack.pop() {
            self.current_color = fill;
            self.stroke_color = stroke;
        }
        self
    }

    /// Begin a transparency group
    /// ISO 32000-1:2008 Section 11.4
    pub fn begin_transparency_group(&mut self, group: TransparencyGroup) -> &mut Self {
        // Save current state
        self.save_state();

        // Mark beginning of transparency group with special comment
        writeln!(&mut self.operations, "% Begin Transparency Group").unwrap();

        // Apply group settings via ExtGState
        let mut extgstate = ExtGState::new();
        extgstate = extgstate.with_blend_mode(group.blend_mode.clone());
        extgstate.alpha_fill = Some(group.opacity as f64);
        extgstate.alpha_stroke = Some(group.opacity as f64);

        // Apply the ExtGState
        self.pending_extgstate = Some(extgstate);
        let _ = self.apply_pending_extgstate();

        // Create group state and push to stack
        let mut group_state = TransparencyGroupState::new(group);
        // Save current operations state
        group_state.saved_state = self.operations.as_bytes().to_vec();
        self.transparency_stack.push(group_state);

        self
    }

    /// End a transparency group
    pub fn end_transparency_group(&mut self) -> &mut Self {
        if let Some(_group_state) = self.transparency_stack.pop() {
            // Mark end of transparency group
            writeln!(&mut self.operations, "% End Transparency Group").unwrap();

            // Restore state
            self.restore_state();
        }
        self
    }

    /// Check if we're currently inside a transparency group
    pub fn in_transparency_group(&self) -> bool {
        !self.transparency_stack.is_empty()
    }

    /// Get the current transparency group (if any)
    pub fn current_transparency_group(&self) -> Option<&TransparencyGroup> {
        self.transparency_stack.last().map(|state| &state.group)
    }

    pub fn translate(&mut self, tx: f64, ty: f64) -> &mut Self {
        writeln!(&mut self.operations, "1 0 0 1 {tx:.2} {ty:.2} cm").unwrap();
        self
    }

    pub fn scale(&mut self, sx: f64, sy: f64) -> &mut Self {
        writeln!(&mut self.operations, "{sx:.2} 0 0 {sy:.2} 0 0 cm").unwrap();
        self
    }

    pub fn rotate(&mut self, angle: f64) -> &mut Self {
        let cos = angle.cos();
        let sin = angle.sin();
        writeln!(
            &mut self.operations,
            "{:.6} {:.6} {:.6} {:.6} 0 0 cm",
            cos, sin, -sin, cos
        )
        .unwrap();
        self
    }

    pub fn transform(&mut self, a: f64, b: f64, c: f64, d: f64, e: f64, f: f64) -> &mut Self {
        writeln!(
            &mut self.operations,
            "{a:.2} {b:.2} {c:.2} {d:.2} {e:.2} {f:.2} cm"
        )
        .unwrap();
        self
    }

    pub fn rectangle(&mut self, x: f64, y: f64, width: f64, height: f64) -> &mut Self {
        self.rect(x, y, width, height)
    }

    pub fn draw_image(
        &mut self,
        image_name: &str,
        x: f64,
        y: f64,
        width: f64,
        height: f64,
    ) -> &mut Self {
        // Save graphics state
        self.save_state();

        // Set up transformation matrix for image placement
        // PDF coordinate system has origin at bottom-left, so we need to translate and scale
        writeln!(
            &mut self.operations,
            "{width:.2} 0 0 {height:.2} {x:.2} {y:.2} cm"
        )
        .unwrap();

        // Draw the image XObject
        writeln!(&mut self.operations, "/{image_name} Do").unwrap();

        // Restore graphics state
        self.restore_state();

        self
    }

    /// Draw an image with transparency support (soft mask)
    /// This method handles images with alpha channels or soft masks
    pub fn draw_image_with_transparency(
        &mut self,
        image_name: &str,
        x: f64,
        y: f64,
        width: f64,
        height: f64,
        mask_name: Option<&str>,
    ) -> &mut Self {
        // Save graphics state
        self.save_state();

        // If we have a mask, we need to set up an ExtGState with SMask
        if let Some(mask) = mask_name {
            // Create an ExtGState for the soft mask
            let mut extgstate = ExtGState::new();
            extgstate.set_soft_mask_name(mask.to_string());

            // Register and apply the ExtGState
            let gs_name = self
                .extgstate_manager
                .add_state(extgstate)
                .unwrap_or_else(|_| "GS1".to_string());
            writeln!(&mut self.operations, "/{} gs", gs_name).unwrap();
        }

        // Set up transformation matrix for image placement
        writeln!(
            &mut self.operations,
            "{width:.2} 0 0 {height:.2} {x:.2} {y:.2} cm"
        )
        .unwrap();

        // Draw the image XObject
        writeln!(&mut self.operations, "/{image_name} Do").unwrap();

        // If we had a mask, reset the soft mask to None
        if mask_name.is_some() {
            // Create an ExtGState that removes the soft mask
            let mut reset_extgstate = ExtGState::new();
            reset_extgstate.set_soft_mask_none();

            let gs_name = self
                .extgstate_manager
                .add_state(reset_extgstate)
                .unwrap_or_else(|_| "GS2".to_string());
            writeln!(&mut self.operations, "/{} gs", gs_name).unwrap();
        }

        // Restore graphics state
        self.restore_state();

        self
    }

    fn apply_stroke_color(&mut self) {
        match self.stroke_color {
            Color::Rgb(r, g, b) => {
                writeln!(&mut self.operations, "{r:.3} {g:.3} {b:.3} RG").unwrap();
            }
            Color::Gray(g) => {
                writeln!(&mut self.operations, "{g:.3} G").unwrap();
            }
            Color::Cmyk(c, m, y, k) => {
                writeln!(&mut self.operations, "{c:.3} {m:.3} {y:.3} {k:.3} K").unwrap();
            }
        }
    }

    fn apply_fill_color(&mut self) {
        match self.current_color {
            Color::Rgb(r, g, b) => {
                writeln!(&mut self.operations, "{r:.3} {g:.3} {b:.3} rg").unwrap();
            }
            Color::Gray(g) => {
                writeln!(&mut self.operations, "{g:.3} g").unwrap();
            }
            Color::Cmyk(c, m, y, k) => {
                writeln!(&mut self.operations, "{c:.3} {m:.3} {y:.3} {k:.3} k").unwrap();
            }
        }
    }

    pub(crate) fn generate_operations(&self) -> Result<Vec<u8>> {
        Ok(self.operations.as_bytes().to_vec())
    }

    /// Check if transparency is used (opacity != 1.0)
    pub fn uses_transparency(&self) -> bool {
        self.fill_opacity < 1.0 || self.stroke_opacity < 1.0
    }

    /// Generate the graphics state dictionary for transparency
    pub fn generate_graphics_state_dict(&self) -> Option<String> {
        if !self.uses_transparency() {
            return None;
        }

        let mut dict = String::from("<< /Type /ExtGState");

        if self.fill_opacity < 1.0 {
            write!(&mut dict, " /ca {:.3}", self.fill_opacity).unwrap();
        }

        if self.stroke_opacity < 1.0 {
            write!(&mut dict, " /CA {:.3}", self.stroke_opacity).unwrap();
        }

        dict.push_str(" >>");
        Some(dict)
    }

    /// Get the current fill color
    pub fn fill_color(&self) -> Color {
        self.current_color
    }

    /// Get the current stroke color
    pub fn stroke_color(&self) -> Color {
        self.stroke_color
    }

    /// Get the current line width
    pub fn line_width(&self) -> f64 {
        self.line_width
    }

    /// Get the current fill opacity
    pub fn fill_opacity(&self) -> f64 {
        self.fill_opacity
    }

    /// Get the current stroke opacity
    pub fn stroke_opacity(&self) -> f64 {
        self.stroke_opacity
    }

    /// Get the operations string
    pub fn operations(&self) -> &str {
        &self.operations
    }

    /// Get the operations string (alias for testing)
    pub fn get_operations(&self) -> &str {
        &self.operations
    }

    /// Clear all operations
    pub fn clear(&mut self) {
        self.operations.clear();
    }

    /// Begin a text object
    pub fn begin_text(&mut self) -> &mut Self {
        self.operations.push_str("BT\n");
        self
    }

    /// End a text object
    pub fn end_text(&mut self) -> &mut Self {
        self.operations.push_str("ET\n");
        self
    }

    /// Set font and size
    pub fn set_font(&mut self, font: Font, size: f64) -> &mut Self {
        writeln!(&mut self.operations, "/{} {} Tf", font.pdf_name(), size).unwrap();

        // Track font name and size for Unicode detection and proper font handling
        match &font {
            Font::Custom(name) => {
                self.current_font_name = Some(name.clone());
                self.current_font_size = size;
            }
            _ => {
                self.current_font_name = Some(font.pdf_name());
                self.current_font_size = size;
            }
        }

        self
    }

    /// Set text position
    pub fn set_text_position(&mut self, x: f64, y: f64) -> &mut Self {
        writeln!(&mut self.operations, "{x:.2} {y:.2} Td").unwrap();
        self
    }

    /// Show text
    pub fn show_text(&mut self, text: &str) -> Result<&mut Self> {
        // Escape special characters in PDF string
        self.operations.push('(');
        for ch in text.chars() {
            match ch {
                '(' => self.operations.push_str("\\("),
                ')' => self.operations.push_str("\\)"),
                '\\' => self.operations.push_str("\\\\"),
                '\n' => self.operations.push_str("\\n"),
                '\r' => self.operations.push_str("\\r"),
                '\t' => self.operations.push_str("\\t"),
                _ => self.operations.push(ch),
            }
        }
        self.operations.push_str(") Tj\n");
        Ok(self)
    }

    /// Render a table
    pub fn render_table(&mut self, table: &Table) -> Result<()> {
        table.render(self)
    }

    /// Render a list
    pub fn render_list(&mut self, list: &ListElement) -> Result<()> {
        match list {
            ListElement::Ordered(ordered) => ordered.render(self),
            ListElement::Unordered(unordered) => unordered.render(self),
        }
    }

    /// Render column layout
    pub fn render_column_layout(
        &mut self,
        layout: &ColumnLayout,
        content: &ColumnContent,
        x: f64,
        y: f64,
        height: f64,
    ) -> Result<()> {
        layout.render(self, content, x, y, height)
    }

    // Extended Graphics State methods

    /// Set line dash pattern
    pub fn set_line_dash_pattern(&mut self, pattern: LineDashPattern) -> &mut Self {
        self.current_dash_pattern = Some(pattern.clone());
        writeln!(&mut self.operations, "{} d", pattern.to_pdf_string()).unwrap();
        self
    }

    /// Set line dash pattern to solid (no dashes)
    pub fn set_line_solid(&mut self) -> &mut Self {
        self.current_dash_pattern = None;
        self.operations.push_str("[] 0 d\n");
        self
    }

    /// Set miter limit
    pub fn set_miter_limit(&mut self, limit: f64) -> &mut Self {
        self.current_miter_limit = limit.max(1.0);
        writeln!(&mut self.operations, "{:.2} M", self.current_miter_limit).unwrap();
        self
    }

    /// Set rendering intent
    pub fn set_rendering_intent(&mut self, intent: RenderingIntent) -> &mut Self {
        self.current_rendering_intent = intent;
        writeln!(&mut self.operations, "/{} ri", intent.pdf_name()).unwrap();
        self
    }

    /// Set flatness tolerance
    pub fn set_flatness(&mut self, flatness: f64) -> &mut Self {
        self.current_flatness = flatness.clamp(0.0, 100.0);
        writeln!(&mut self.operations, "{:.2} i", self.current_flatness).unwrap();
        self
    }

    /// Apply an ExtGState dictionary immediately
    pub fn apply_extgstate(&mut self, state: ExtGState) -> Result<&mut Self> {
        let state_name = self.extgstate_manager.add_state(state)?;
        writeln!(&mut self.operations, "/{state_name} gs").unwrap();
        Ok(self)
    }

    /// Store an ExtGState to be applied before the next drawing operation
    fn set_pending_extgstate(&mut self, state: ExtGState) {
        self.pending_extgstate = Some(state);
    }

    /// Apply any pending ExtGState before drawing
    fn apply_pending_extgstate(&mut self) -> Result<()> {
        if let Some(state) = self.pending_extgstate.take() {
            let state_name = self.extgstate_manager.add_state(state)?;
            writeln!(&mut self.operations, "/{state_name} gs").unwrap();
        }
        Ok(())
    }

    /// Create and apply a custom ExtGState
    pub fn with_extgstate<F>(&mut self, builder: F) -> Result<&mut Self>
    where
        F: FnOnce(ExtGState) -> ExtGState,
    {
        let state = builder(ExtGState::new());
        self.apply_extgstate(state)
    }

    /// Set blend mode for transparency
    pub fn set_blend_mode(&mut self, mode: BlendMode) -> Result<&mut Self> {
        let state = ExtGState::new().with_blend_mode(mode);
        self.apply_extgstate(state)
    }

    /// Set alpha for both stroke and fill operations
    pub fn set_alpha(&mut self, alpha: f64) -> Result<&mut Self> {
        let state = ExtGState::new().with_alpha(alpha);
        self.set_pending_extgstate(state);
        Ok(self)
    }

    /// Set alpha for stroke operations only
    pub fn set_alpha_stroke(&mut self, alpha: f64) -> Result<&mut Self> {
        let state = ExtGState::new().with_alpha_stroke(alpha);
        self.set_pending_extgstate(state);
        Ok(self)
    }

    /// Set alpha for fill operations only
    pub fn set_alpha_fill(&mut self, alpha: f64) -> Result<&mut Self> {
        let state = ExtGState::new().with_alpha_fill(alpha);
        self.set_pending_extgstate(state);
        Ok(self)
    }

    /// Set overprint for stroke operations
    pub fn set_overprint_stroke(&mut self, overprint: bool) -> Result<&mut Self> {
        let state = ExtGState::new().with_overprint_stroke(overprint);
        self.apply_extgstate(state)
    }

    /// Set overprint for fill operations
    pub fn set_overprint_fill(&mut self, overprint: bool) -> Result<&mut Self> {
        let state = ExtGState::new().with_overprint_fill(overprint);
        self.apply_extgstate(state)
    }

    /// Set stroke adjustment
    pub fn set_stroke_adjustment(&mut self, adjustment: bool) -> Result<&mut Self> {
        let state = ExtGState::new().with_stroke_adjustment(adjustment);
        self.apply_extgstate(state)
    }

    /// Set smoothness tolerance
    pub fn set_smoothness(&mut self, smoothness: f64) -> Result<&mut Self> {
        self.current_smoothness = smoothness.clamp(0.0, 1.0);
        let state = ExtGState::new().with_smoothness(self.current_smoothness);
        self.apply_extgstate(state)
    }

    // Getters for extended graphics state

    /// Get current line dash pattern
    pub fn line_dash_pattern(&self) -> Option<&LineDashPattern> {
        self.current_dash_pattern.as_ref()
    }

    /// Get current miter limit
    pub fn miter_limit(&self) -> f64 {
        self.current_miter_limit
    }

    /// Get current line cap
    pub fn line_cap(&self) -> LineCap {
        self.current_line_cap
    }

    /// Get current line join
    pub fn line_join(&self) -> LineJoin {
        self.current_line_join
    }

    /// Get current rendering intent
    pub fn rendering_intent(&self) -> RenderingIntent {
        self.current_rendering_intent
    }

    /// Get current flatness tolerance
    pub fn flatness(&self) -> f64 {
        self.current_flatness
    }

    /// Get current smoothness tolerance
    pub fn smoothness(&self) -> f64 {
        self.current_smoothness
    }

    /// Get the ExtGState manager (for advanced usage)
    pub fn extgstate_manager(&self) -> &ExtGStateManager {
        &self.extgstate_manager
    }

    /// Get mutable ExtGState manager (for advanced usage)
    pub fn extgstate_manager_mut(&mut self) -> &mut ExtGStateManager {
        &mut self.extgstate_manager
    }

    /// Generate ExtGState resource dictionary for PDF
    pub fn generate_extgstate_resources(&self) -> Result<String> {
        self.extgstate_manager.to_resource_dictionary()
    }

    /// Check if any extended graphics states are defined
    pub fn has_extgstates(&self) -> bool {
        self.extgstate_manager.count() > 0
    }

    /// Add a command to the operations
    pub fn add_command(&mut self, command: &str) {
        self.operations.push_str(command);
        self.operations.push('\n');
    }

    /// Create clipping path from current path using non-zero winding rule
    pub fn clip(&mut self) -> &mut Self {
        self.operations.push_str("W\n");
        self
    }

    /// Create clipping path from current path using even-odd rule
    pub fn clip_even_odd(&mut self) -> &mut Self {
        self.operations.push_str("W*\n");
        self
    }

    /// Create clipping path and stroke it
    pub fn clip_stroke(&mut self) -> &mut Self {
        self.apply_stroke_color();
        self.operations.push_str("W S\n");
        self
    }

    /// Set a custom clipping path
    pub fn set_clipping_path(&mut self, path: ClippingPath) -> Result<&mut Self> {
        let ops = path.to_pdf_operations()?;
        self.operations.push_str(&ops);
        self.clipping_region.set_clip(path);
        Ok(self)
    }

    /// Clear the current clipping path
    pub fn clear_clipping(&mut self) -> &mut Self {
        self.clipping_region.clear_clip();
        self
    }

    /// Save the current clipping state (called automatically by save_state)
    fn save_clipping_state(&mut self) {
        self.clipping_region.save();
    }

    /// Restore the previous clipping state (called automatically by restore_state)
    fn restore_clipping_state(&mut self) {
        self.clipping_region.restore();
    }

    /// Create a rectangular clipping region
    pub fn clip_rect(&mut self, x: f64, y: f64, width: f64, height: f64) -> Result<&mut Self> {
        let path = ClippingPath::rect(x, y, width, height);
        self.set_clipping_path(path)
    }

    /// Create a circular clipping region
    pub fn clip_circle(&mut self, cx: f64, cy: f64, radius: f64) -> Result<&mut Self> {
        let path = ClippingPath::circle(cx, cy, radius);
        self.set_clipping_path(path)
    }

    /// Create an elliptical clipping region
    pub fn clip_ellipse(&mut self, cx: f64, cy: f64, rx: f64, ry: f64) -> Result<&mut Self> {
        let path = ClippingPath::ellipse(cx, cy, rx, ry);
        self.set_clipping_path(path)
    }

    /// Check if a clipping path is active
    pub fn has_clipping(&self) -> bool {
        self.clipping_region.has_clip()
    }

    /// Get the current clipping path
    pub fn clipping_path(&self) -> Option<&ClippingPath> {
        self.clipping_region.current()
    }

    /// Set the font manager for custom fonts
    pub fn set_font_manager(&mut self, font_manager: Arc<FontManager>) -> &mut Self {
        self.font_manager = Some(font_manager);
        self
    }

    /// Set the current font to a custom font
    pub fn set_custom_font(&mut self, font_name: &str, size: f64) -> &mut Self {
        self.current_font_name = Some(font_name.to_string());
        self.current_font_size = size;

        // Try to get the glyph mapping from the font manager
        if let Some(ref font_manager) = self.font_manager {
            if let Some(mapping) = font_manager.get_font_glyph_mapping(font_name) {
                self.glyph_mapping = Some(mapping);
            }
        }

        self
    }

    /// Set the glyph mapping for Unicode fonts (Unicode -> GlyphID)
    pub fn set_glyph_mapping(&mut self, mapping: HashMap<u32, u16>) -> &mut Self {
        self.glyph_mapping = Some(mapping);
        self
    }

    /// Draw text at the specified position with automatic encoding detection
    pub fn draw_text(&mut self, text: &str, x: f64, y: f64) -> Result<&mut Self> {
        // Track used characters for font subsetting
        self.used_characters.extend(text.chars());

        // Check if we're using a custom font (which will be Type0/Unicode)
        // Custom fonts are those that are not the standard PDF fonts (Helvetica, Times, etc.)
        let using_custom_font = if let Some(ref font_name) = self.current_font_name {
            // If font name doesn't start with standard PDF font names, it's custom
            !matches!(
                font_name.as_str(),
                "Helvetica"
                    | "Times"
                    | "Courier"
                    | "Symbol"
                    | "ZapfDingbats"
                    | "Helvetica-Bold"
                    | "Helvetica-Oblique"
                    | "Helvetica-BoldOblique"
                    | "Times-Roman"
                    | "Times-Bold"
                    | "Times-Italic"
                    | "Times-BoldItalic"
                    | "Courier-Bold"
                    | "Courier-Oblique"
                    | "Courier-BoldOblique"
            )
        } else {
            false
        };

        // Detect if text needs Unicode encoding
        let needs_unicode = text.chars().any(|c| c as u32 > 255) || using_custom_font;

        // Use appropriate encoding based on content and font type
        if needs_unicode {
            self.draw_with_unicode_encoding(text, x, y)
        } else {
            self.draw_with_simple_encoding(text, x, y)
        }
    }

    /// Internal: Draw text with simple encoding (WinAnsiEncoding for standard fonts)
    fn draw_with_simple_encoding(&mut self, text: &str, x: f64, y: f64) -> Result<&mut Self> {
        // Check if text contains characters outside Latin-1
        let has_unicode = text.chars().any(|c| c as u32 > 255);

        if has_unicode {
            // Warning: Text contains Unicode characters but no Unicode font is set
            eprintln!("Warning: Text contains Unicode characters but using Latin-1 font. Characters will be replaced with '?'");
        }

        // Begin text object
        self.operations.push_str("BT\n");

        // Set font if available
        if let Some(font_name) = &self.current_font_name {
            writeln!(
                &mut self.operations,
                "/{} {} Tf",
                font_name, self.current_font_size
            )
            .unwrap();
        } else {
            writeln!(
                &mut self.operations,
                "/Helvetica {} Tf",
                self.current_font_size
            )
            .unwrap();
        }

        // Set text position
        writeln!(&mut self.operations, "{:.2} {:.2} Td", x, y).unwrap();

        // Use parentheses encoding for Latin-1 text (standard PDF fonts use WinAnsiEncoding)
        // This allows proper rendering of accented characters
        self.operations.push('(');
        for ch in text.chars() {
            let code = ch as u32;
            if code <= 127 {
                // ASCII characters - handle special characters that need escaping
                match ch {
                    '(' => self.operations.push_str("\\("),
                    ')' => self.operations.push_str("\\)"),
                    '\\' => self.operations.push_str("\\\\"),
                    '\n' => self.operations.push_str("\\n"),
                    '\r' => self.operations.push_str("\\r"),
                    '\t' => self.operations.push_str("\\t"),
                    _ => self.operations.push(ch),
                }
            } else if code <= 255 {
                // Latin-1 characters (128-255)
                // For WinAnsiEncoding, we can use octal notation for high-bit characters
                write!(&mut self.operations, "\\{:03o}", code).unwrap();
            } else {
                // Characters outside Latin-1 - replace with '?'
                self.operations.push('?');
            }
        }
        self.operations.push_str(") Tj\n");

        // End text object
        self.operations.push_str("ET\n");

        Ok(self)
    }

    /// Internal: Draw text with Unicode encoding (Type0/CID)
    fn draw_with_unicode_encoding(&mut self, text: &str, x: f64, y: f64) -> Result<&mut Self> {
        // Begin text object
        self.operations.push_str("BT\n");

        // Set font - ensure it's a Type0 font for Unicode
        if let Some(font_name) = &self.current_font_name {
            // The font should be converted to Type0 by FontManager if needed
            writeln!(
                &mut self.operations,
                "/{} {} Tf",
                font_name, self.current_font_size
            )
            .unwrap();
        } else {
            writeln!(
                &mut self.operations,
                "/Helvetica {} Tf",
                self.current_font_size
            )
            .unwrap();
        }

        // Set text position
        writeln!(&mut self.operations, "{:.2} {:.2} Td", x, y).unwrap();

        // IMPORTANT: For Type0 fonts with Identity-H encoding, we write CIDs (Character IDs),
        // NOT GlyphIDs. The CIDToGIDMap in the font handles the CID -> GlyphID conversion.
        // In our case, we use Unicode code points as CIDs.
        self.operations.push('<');

        for ch in text.chars() {
            let code = ch as u32;

            // For Type0 fonts with Identity-H encoding, write the Unicode code point as CID
            // The CIDToGIDMap will handle the conversion to the actual glyph ID
            if code <= 0xFFFF {
                // Write the Unicode code point as a 2-byte hex value (CID)
                write!(&mut self.operations, "{:04X}", code).unwrap();
            } else {
                // Characters outside BMP - use replacement character
                // Most PDF viewers don't handle supplementary planes well
                write!(&mut self.operations, "FFFD").unwrap(); // Unicode replacement character
            }
        }
        self.operations.push_str("> Tj\n");

        // End text object
        self.operations.push_str("ET\n");

        Ok(self)
    }

    /// Legacy: Draw text with hex encoding (kept for compatibility)
    #[deprecated(note = "Use draw_text() which automatically detects encoding")]
    pub fn draw_text_hex(&mut self, text: &str, x: f64, y: f64) -> Result<&mut Self> {
        // Begin text object
        self.operations.push_str("BT\n");

        // Set font if available
        if let Some(font_name) = &self.current_font_name {
            writeln!(
                &mut self.operations,
                "/{} {} Tf",
                font_name, self.current_font_size
            )
            .unwrap();
        } else {
            // Fallback to Helvetica if no font is set
            writeln!(
                &mut self.operations,
                "/Helvetica {} Tf",
                self.current_font_size
            )
            .unwrap();
        }

        // Set text position
        writeln!(&mut self.operations, "{:.2} {:.2} Td", x, y).unwrap();

        // Encode text as hex string
        // For TrueType fonts with Identity-H encoding, we need UTF-16BE
        // But we'll use single-byte encoding for now to fix spacing
        self.operations.push('<');
        for ch in text.chars() {
            if ch as u32 <= 255 {
                // For characters in the Latin-1 range, use single byte
                write!(&mut self.operations, "{:02X}", ch as u8).unwrap();
            } else {
                // For characters outside Latin-1, we need proper glyph mapping
                // For now, use a placeholder
                write!(&mut self.operations, "3F").unwrap(); // '?' character
            }
        }
        self.operations.push_str("> Tj\n");

        // End text object
        self.operations.push_str("ET\n");

        Ok(self)
    }

    /// Legacy: Draw text with Type0 font encoding (kept for compatibility)
    #[deprecated(note = "Use draw_text() which automatically detects encoding")]
    pub fn draw_text_cid(&mut self, text: &str, x: f64, y: f64) -> Result<&mut Self> {
        use crate::fonts::needs_type0_font;

        // Begin text object
        self.operations.push_str("BT\n");

        // Set font if available
        if let Some(font_name) = &self.current_font_name {
            writeln!(
                &mut self.operations,
                "/{} {} Tf",
                font_name, self.current_font_size
            )
            .unwrap();
        } else {
            writeln!(
                &mut self.operations,
                "/Helvetica {} Tf",
                self.current_font_size
            )
            .unwrap();
        }

        // Set text position
        writeln!(&mut self.operations, "{:.2} {:.2} Td", x, y).unwrap();

        // Check if text needs Type0 encoding
        if needs_type0_font(text) {
            // Use 2-byte hex encoding for CIDs with identity mapping
            self.operations.push('<');
            for ch in text.chars() {
                let code = ch as u32;

                // Handle all Unicode characters
                if code <= 0xFFFF {
                    // Direct identity mapping for BMP characters
                    write!(&mut self.operations, "{:04X}", code).unwrap();
                } else if code <= 0x10FFFF {
                    // For characters outside BMP - use surrogate pairs
                    let code = code - 0x10000;
                    let high = ((code >> 10) & 0x3FF) + 0xD800;
                    let low = (code & 0x3FF) + 0xDC00;
                    write!(&mut self.operations, "{:04X}{:04X}", high, low).unwrap();
                } else {
                    // Invalid Unicode - use replacement character
                    write!(&mut self.operations, "FFFD").unwrap();
                }
            }
            self.operations.push_str("> Tj\n");
        } else {
            // Use regular single-byte encoding for Latin-1
            self.operations.push('<');
            for ch in text.chars() {
                if ch as u32 <= 255 {
                    write!(&mut self.operations, "{:02X}", ch as u8).unwrap();
                } else {
                    write!(&mut self.operations, "3F").unwrap();
                }
            }
            self.operations.push_str("> Tj\n");
        }

        // End text object
        self.operations.push_str("ET\n");
        Ok(self)
    }

    /// Legacy: Draw text with UTF-16BE encoding (kept for compatibility)
    #[deprecated(note = "Use draw_text() which automatically detects encoding")]
    pub fn draw_text_unicode(&mut self, text: &str, x: f64, y: f64) -> Result<&mut Self> {
        // Begin text object
        self.operations.push_str("BT\n");

        // Set font if available
        if let Some(font_name) = &self.current_font_name {
            writeln!(
                &mut self.operations,
                "/{} {} Tf",
                font_name, self.current_font_size
            )
            .unwrap();
        } else {
            // Fallback to Helvetica if no font is set
            writeln!(
                &mut self.operations,
                "/Helvetica {} Tf",
                self.current_font_size
            )
            .unwrap();
        }

        // Set text position
        writeln!(&mut self.operations, "{:.2} {:.2} Td", x, y).unwrap();

        // Encode text as UTF-16BE hex string
        self.operations.push('<');
        let mut utf16_buffer = [0u16; 2];
        for ch in text.chars() {
            let encoded = ch.encode_utf16(&mut utf16_buffer);
            for unit in encoded {
                // Write UTF-16BE (big-endian)
                write!(&mut self.operations, "{:04X}", unit).unwrap();
            }
        }
        self.operations.push_str("> Tj\n");

        // End text object
        self.operations.push_str("ET\n");

        Ok(self)
    }

    /// Get the characters used in this graphics context
    pub(crate) fn get_used_characters(&self) -> Option<HashSet<char>> {
        if self.used_characters.is_empty() {
            None
        } else {
            Some(self.used_characters.clone())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_graphics_context_new() {
        let ctx = GraphicsContext::new();
        assert_eq!(ctx.fill_color(), Color::black());
        assert_eq!(ctx.stroke_color(), Color::black());
        assert_eq!(ctx.line_width(), 1.0);
        assert_eq!(ctx.fill_opacity(), 1.0);
        assert_eq!(ctx.stroke_opacity(), 1.0);
        assert!(ctx.operations().is_empty());
    }

    #[test]
    fn test_graphics_context_default() {
        let ctx = GraphicsContext::default();
        assert_eq!(ctx.fill_color(), Color::black());
        assert_eq!(ctx.stroke_color(), Color::black());
        assert_eq!(ctx.line_width(), 1.0);
    }

    #[test]
    fn test_move_to() {
        let mut ctx = GraphicsContext::new();
        ctx.move_to(10.0, 20.0);
        assert!(ctx.operations().contains("10.00 20.00 m\n"));
    }

    #[test]
    fn test_line_to() {
        let mut ctx = GraphicsContext::new();
        ctx.line_to(30.0, 40.0);
        assert!(ctx.operations().contains("30.00 40.00 l\n"));
    }

    #[test]
    fn test_curve_to() {
        let mut ctx = GraphicsContext::new();
        ctx.curve_to(10.0, 20.0, 30.0, 40.0, 50.0, 60.0);
        assert!(ctx
            .operations()
            .contains("10.00 20.00 30.00 40.00 50.00 60.00 c\n"));
    }

    #[test]
    fn test_rect() {
        let mut ctx = GraphicsContext::new();
        ctx.rect(10.0, 20.0, 100.0, 50.0);
        assert!(ctx.operations().contains("10.00 20.00 100.00 50.00 re\n"));
    }

    #[test]
    fn test_rectangle_alias() {
        let mut ctx = GraphicsContext::new();
        ctx.rectangle(10.0, 20.0, 100.0, 50.0);
        assert!(ctx.operations().contains("10.00 20.00 100.00 50.00 re\n"));
    }

    #[test]
    fn test_circle() {
        let mut ctx = GraphicsContext::new();
        ctx.circle(50.0, 50.0, 25.0);

        let ops = ctx.operations();
        // Check that it starts with move to radius point
        assert!(ops.contains("75.00 50.00 m\n"));
        // Check that it contains curve operations
        assert!(ops.contains(" c\n"));
        // Check that it closes the path
        assert!(ops.contains("h\n"));
    }

    #[test]
    fn test_close_path() {
        let mut ctx = GraphicsContext::new();
        ctx.close_path();
        assert!(ctx.operations().contains("h\n"));
    }

    #[test]
    fn test_stroke() {
        let mut ctx = GraphicsContext::new();
        ctx.set_stroke_color(Color::red());
        ctx.rect(0.0, 0.0, 10.0, 10.0);
        ctx.stroke();

        let ops = ctx.operations();
        assert!(ops.contains("1.000 0.000 0.000 RG\n"));
        assert!(ops.contains("S\n"));
    }

    #[test]
    fn test_fill() {
        let mut ctx = GraphicsContext::new();
        ctx.set_fill_color(Color::blue());
        ctx.rect(0.0, 0.0, 10.0, 10.0);
        ctx.fill();

        let ops = ctx.operations();
        assert!(ops.contains("0.000 0.000 1.000 rg\n"));
        assert!(ops.contains("f\n"));
    }

    #[test]
    fn test_fill_stroke() {
        let mut ctx = GraphicsContext::new();
        ctx.set_fill_color(Color::green());
        ctx.set_stroke_color(Color::red());
        ctx.rect(0.0, 0.0, 10.0, 10.0);
        ctx.fill_stroke();

        let ops = ctx.operations();
        assert!(ops.contains("0.000 1.000 0.000 rg\n"));
        assert!(ops.contains("1.000 0.000 0.000 RG\n"));
        assert!(ops.contains("B\n"));
    }

    #[test]
    fn test_set_stroke_color() {
        let mut ctx = GraphicsContext::new();
        ctx.set_stroke_color(Color::rgb(0.5, 0.6, 0.7));
        assert_eq!(ctx.stroke_color(), Color::Rgb(0.5, 0.6, 0.7));
    }

    #[test]
    fn test_set_fill_color() {
        let mut ctx = GraphicsContext::new();
        ctx.set_fill_color(Color::gray(0.5));
        assert_eq!(ctx.fill_color(), Color::Gray(0.5));
    }

    #[test]
    fn test_set_line_width() {
        let mut ctx = GraphicsContext::new();
        ctx.set_line_width(2.5);
        assert_eq!(ctx.line_width(), 2.5);
        assert!(ctx.operations().contains("2.50 w\n"));
    }

    #[test]
    fn test_set_line_cap() {
        let mut ctx = GraphicsContext::new();
        ctx.set_line_cap(LineCap::Round);
        assert!(ctx.operations().contains("1 J\n"));

        ctx.set_line_cap(LineCap::Butt);
        assert!(ctx.operations().contains("0 J\n"));

        ctx.set_line_cap(LineCap::Square);
        assert!(ctx.operations().contains("2 J\n"));
    }

    #[test]
    fn test_set_line_join() {
        let mut ctx = GraphicsContext::new();
        ctx.set_line_join(LineJoin::Round);
        assert!(ctx.operations().contains("1 j\n"));

        ctx.set_line_join(LineJoin::Miter);
        assert!(ctx.operations().contains("0 j\n"));

        ctx.set_line_join(LineJoin::Bevel);
        assert!(ctx.operations().contains("2 j\n"));
    }

    #[test]
    fn test_save_restore_state() {
        let mut ctx = GraphicsContext::new();
        ctx.save_state();
        assert!(ctx.operations().contains("q\n"));

        ctx.restore_state();
        assert!(ctx.operations().contains("Q\n"));
    }

    #[test]
    fn test_translate() {
        let mut ctx = GraphicsContext::new();
        ctx.translate(50.0, 100.0);
        assert!(ctx.operations().contains("1 0 0 1 50.00 100.00 cm\n"));
    }

    #[test]
    fn test_scale() {
        let mut ctx = GraphicsContext::new();
        ctx.scale(2.0, 3.0);
        assert!(ctx.operations().contains("2.00 0 0 3.00 0 0 cm\n"));
    }

    #[test]
    fn test_rotate() {
        let mut ctx = GraphicsContext::new();
        let angle = std::f64::consts::PI / 4.0; // 45 degrees
        ctx.rotate(angle);

        let ops = ctx.operations();
        assert!(ops.contains(" cm\n"));
        // Should contain cos and sin values
        assert!(ops.contains("0.707107")); // Approximate cos(45°)
    }

    #[test]
    fn test_transform() {
        let mut ctx = GraphicsContext::new();
        ctx.transform(1.0, 2.0, 3.0, 4.0, 5.0, 6.0);
        assert!(ctx
            .operations()
            .contains("1.00 2.00 3.00 4.00 5.00 6.00 cm\n"));
    }

    #[test]
    fn test_draw_image() {
        let mut ctx = GraphicsContext::new();
        ctx.draw_image("Image1", 10.0, 20.0, 100.0, 150.0);

        let ops = ctx.operations();
        assert!(ops.contains("q\n")); // Save state
        assert!(ops.contains("100.00 0 0 150.00 10.00 20.00 cm\n")); // Transform
        assert!(ops.contains("/Image1 Do\n")); // Draw image
        assert!(ops.contains("Q\n")); // Restore state
    }

    #[test]
    fn test_gray_color_operations() {
        let mut ctx = GraphicsContext::new();
        ctx.set_stroke_color(Color::gray(0.5));
        ctx.set_fill_color(Color::gray(0.7));
        ctx.stroke();
        ctx.fill();

        let ops = ctx.operations();
        assert!(ops.contains("0.500 G\n")); // Stroke gray
        assert!(ops.contains("0.700 g\n")); // Fill gray
    }

    #[test]
    fn test_cmyk_color_operations() {
        let mut ctx = GraphicsContext::new();
        ctx.set_stroke_color(Color::cmyk(0.1, 0.2, 0.3, 0.4));
        ctx.set_fill_color(Color::cmyk(0.5, 0.6, 0.7, 0.8));
        ctx.stroke();
        ctx.fill();

        let ops = ctx.operations();
        assert!(ops.contains("0.100 0.200 0.300 0.400 K\n")); // Stroke CMYK
        assert!(ops.contains("0.500 0.600 0.700 0.800 k\n")); // Fill CMYK
    }

    #[test]
    fn test_method_chaining() {
        let mut ctx = GraphicsContext::new();
        ctx.move_to(0.0, 0.0)
            .line_to(10.0, 0.0)
            .line_to(10.0, 10.0)
            .line_to(0.0, 10.0)
            .close_path()
            .set_fill_color(Color::red())
            .fill();

        let ops = ctx.operations();
        assert!(ops.contains("0.00 0.00 m\n"));
        assert!(ops.contains("10.00 0.00 l\n"));
        assert!(ops.contains("10.00 10.00 l\n"));
        assert!(ops.contains("0.00 10.00 l\n"));
        assert!(ops.contains("h\n"));
        assert!(ops.contains("f\n"));
    }

    #[test]
    fn test_generate_operations() {
        let mut ctx = GraphicsContext::new();
        ctx.rect(0.0, 0.0, 10.0, 10.0);

        let result = ctx.generate_operations();
        assert!(result.is_ok());
        let bytes = result.unwrap();
        let ops_string = String::from_utf8(bytes).unwrap();
        assert!(ops_string.contains("0.00 0.00 10.00 10.00 re"));
    }

    #[test]
    fn test_clear_operations() {
        let mut ctx = GraphicsContext::new();
        ctx.rect(0.0, 0.0, 10.0, 10.0);
        assert!(!ctx.operations().is_empty());

        ctx.clear();
        assert!(ctx.operations().is_empty());
    }

    #[test]
    fn test_complex_path() {
        let mut ctx = GraphicsContext::new();
        ctx.save_state()
            .translate(100.0, 100.0)
            .rotate(std::f64::consts::PI / 6.0)
            .scale(2.0, 2.0)
            .set_line_width(2.0)
            .set_stroke_color(Color::blue())
            .move_to(0.0, 0.0)
            .line_to(50.0, 0.0)
            .curve_to(50.0, 25.0, 25.0, 50.0, 0.0, 50.0)
            .close_path()
            .stroke()
            .restore_state();

        let ops = ctx.operations();
        assert!(ops.contains("q\n"));
        assert!(ops.contains("cm\n"));
        assert!(ops.contains("2.00 w\n"));
        assert!(ops.contains("0.000 0.000 1.000 RG\n"));
        assert!(ops.contains("S\n"));
        assert!(ops.contains("Q\n"));
    }

    #[test]
    fn test_graphics_context_clone() {
        let mut ctx = GraphicsContext::new();
        ctx.set_fill_color(Color::red());
        ctx.set_stroke_color(Color::blue());
        ctx.set_line_width(3.0);
        ctx.set_opacity(0.5);
        ctx.rect(0.0, 0.0, 10.0, 10.0);

        let ctx_clone = ctx.clone();
        assert_eq!(ctx_clone.fill_color(), Color::red());
        assert_eq!(ctx_clone.stroke_color(), Color::blue());
        assert_eq!(ctx_clone.line_width(), 3.0);
        assert_eq!(ctx_clone.fill_opacity(), 0.5);
        assert_eq!(ctx_clone.stroke_opacity(), 0.5);
        assert_eq!(ctx_clone.operations(), ctx.operations());
    }

    #[test]
    fn test_set_opacity() {
        let mut ctx = GraphicsContext::new();

        // Test setting opacity
        ctx.set_opacity(0.5);
        assert_eq!(ctx.fill_opacity(), 0.5);
        assert_eq!(ctx.stroke_opacity(), 0.5);

        // Test clamping to valid range
        ctx.set_opacity(1.5);
        assert_eq!(ctx.fill_opacity(), 1.0);
        assert_eq!(ctx.stroke_opacity(), 1.0);

        ctx.set_opacity(-0.5);
        assert_eq!(ctx.fill_opacity(), 0.0);
        assert_eq!(ctx.stroke_opacity(), 0.0);
    }

    #[test]
    fn test_set_fill_opacity() {
        let mut ctx = GraphicsContext::new();

        ctx.set_fill_opacity(0.3);
        assert_eq!(ctx.fill_opacity(), 0.3);
        assert_eq!(ctx.stroke_opacity(), 1.0); // Should not affect stroke

        // Test clamping
        ctx.set_fill_opacity(2.0);
        assert_eq!(ctx.fill_opacity(), 1.0);
    }

    #[test]
    fn test_set_stroke_opacity() {
        let mut ctx = GraphicsContext::new();

        ctx.set_stroke_opacity(0.7);
        assert_eq!(ctx.stroke_opacity(), 0.7);
        assert_eq!(ctx.fill_opacity(), 1.0); // Should not affect fill

        // Test clamping
        ctx.set_stroke_opacity(-1.0);
        assert_eq!(ctx.stroke_opacity(), 0.0);
    }

    #[test]
    fn test_uses_transparency() {
        let mut ctx = GraphicsContext::new();

        // Initially no transparency
        assert!(!ctx.uses_transparency());

        // With fill transparency
        ctx.set_fill_opacity(0.5);
        assert!(ctx.uses_transparency());

        // Reset and test stroke transparency
        ctx.set_fill_opacity(1.0);
        assert!(!ctx.uses_transparency());
        ctx.set_stroke_opacity(0.8);
        assert!(ctx.uses_transparency());

        // Both transparent
        ctx.set_fill_opacity(0.5);
        assert!(ctx.uses_transparency());
    }

    #[test]
    fn test_generate_graphics_state_dict() {
        let mut ctx = GraphicsContext::new();

        // No transparency
        assert_eq!(ctx.generate_graphics_state_dict(), None);

        // Fill opacity only
        ctx.set_fill_opacity(0.5);
        let dict = ctx.generate_graphics_state_dict().unwrap();
        assert!(dict.contains("/Type /ExtGState"));
        assert!(dict.contains("/ca 0.500"));
        assert!(!dict.contains("/CA"));

        // Stroke opacity only
        ctx.set_fill_opacity(1.0);
        ctx.set_stroke_opacity(0.75);
        let dict = ctx.generate_graphics_state_dict().unwrap();
        assert!(dict.contains("/Type /ExtGState"));
        assert!(dict.contains("/CA 0.750"));
        assert!(!dict.contains("/ca"));

        // Both opacities
        ctx.set_fill_opacity(0.25);
        let dict = ctx.generate_graphics_state_dict().unwrap();
        assert!(dict.contains("/Type /ExtGState"));
        assert!(dict.contains("/ca 0.250"));
        assert!(dict.contains("/CA 0.750"));
    }

    #[test]
    fn test_opacity_with_graphics_operations() {
        let mut ctx = GraphicsContext::new();

        ctx.set_fill_color(Color::red())
            .set_opacity(0.5)
            .rect(10.0, 10.0, 100.0, 100.0)
            .fill();

        assert_eq!(ctx.fill_opacity(), 0.5);
        assert_eq!(ctx.stroke_opacity(), 0.5);

        let ops = ctx.operations();
        assert!(ops.contains("10.00 10.00 100.00 100.00 re"));
        assert!(ops.contains("1.000 0.000 0.000 rg")); // Red color
        assert!(ops.contains("f")); // Fill
    }

    #[test]
    fn test_begin_end_text() {
        let mut ctx = GraphicsContext::new();
        ctx.begin_text();
        assert!(ctx.operations().contains("BT\n"));

        ctx.end_text();
        assert!(ctx.operations().contains("ET\n"));
    }

    #[test]
    fn test_set_font() {
        let mut ctx = GraphicsContext::new();
        ctx.set_font(Font::Helvetica, 12.0);
        assert!(ctx.operations().contains("/Helvetica 12 Tf\n"));

        ctx.set_font(Font::TimesBold, 14.5);
        assert!(ctx.operations().contains("/Times-Bold 14.5 Tf\n"));
    }

    #[test]
    fn test_set_text_position() {
        let mut ctx = GraphicsContext::new();
        ctx.set_text_position(100.0, 200.0);
        assert!(ctx.operations().contains("100.00 200.00 Td\n"));
    }

    #[test]
    fn test_show_text() {
        let mut ctx = GraphicsContext::new();
        ctx.show_text("Hello World").unwrap();
        assert!(ctx.operations().contains("(Hello World) Tj\n"));
    }

    #[test]
    fn test_show_text_with_escaping() {
        let mut ctx = GraphicsContext::new();
        ctx.show_text("Test (parentheses)").unwrap();
        assert!(ctx.operations().contains("(Test \\(parentheses\\)) Tj\n"));

        ctx.clear();
        ctx.show_text("Back\\slash").unwrap();
        assert!(ctx.operations().contains("(Back\\\\slash) Tj\n"));

        ctx.clear();
        ctx.show_text("Line\nBreak").unwrap();
        assert!(ctx.operations().contains("(Line\\nBreak) Tj\n"));
    }

    #[test]
    fn test_text_operations_chaining() {
        let mut ctx = GraphicsContext::new();
        ctx.begin_text()
            .set_font(Font::Courier, 10.0)
            .set_text_position(50.0, 100.0)
            .show_text("Test")
            .unwrap()
            .end_text();

        let ops = ctx.operations();
        assert!(ops.contains("BT\n"));
        assert!(ops.contains("/Courier 10 Tf\n"));
        assert!(ops.contains("50.00 100.00 Td\n"));
        assert!(ops.contains("(Test) Tj\n"));
        assert!(ops.contains("ET\n"));
    }

    #[test]
    fn test_clip() {
        let mut ctx = GraphicsContext::new();
        ctx.clip();
        assert!(ctx.operations().contains("W\n"));
    }

    #[test]
    fn test_clip_even_odd() {
        let mut ctx = GraphicsContext::new();
        ctx.clip_even_odd();
        assert!(ctx.operations().contains("W*\n"));
    }

    #[test]
    fn test_clipping_with_path() {
        let mut ctx = GraphicsContext::new();

        // Create a rectangular clipping path
        ctx.rect(10.0, 10.0, 100.0, 50.0).clip();

        let ops = ctx.operations();
        assert!(ops.contains("10.00 10.00 100.00 50.00 re\n"));
        assert!(ops.contains("W\n"));
    }

    #[test]
    fn test_clipping_even_odd_with_path() {
        let mut ctx = GraphicsContext::new();

        // Create a complex path and clip with even-odd rule
        ctx.move_to(0.0, 0.0)
            .line_to(100.0, 0.0)
            .line_to(100.0, 100.0)
            .line_to(0.0, 100.0)
            .close_path()
            .clip_even_odd();

        let ops = ctx.operations();
        assert!(ops.contains("0.00 0.00 m\n"));
        assert!(ops.contains("100.00 0.00 l\n"));
        assert!(ops.contains("100.00 100.00 l\n"));
        assert!(ops.contains("0.00 100.00 l\n"));
        assert!(ops.contains("h\n"));
        assert!(ops.contains("W*\n"));
    }

    #[test]
    fn test_clipping_chaining() {
        let mut ctx = GraphicsContext::new();

        // Test method chaining with clipping
        ctx.save_state()
            .rect(20.0, 20.0, 60.0, 60.0)
            .clip()
            .set_fill_color(Color::red())
            .rect(0.0, 0.0, 100.0, 100.0)
            .fill()
            .restore_state();

        let ops = ctx.operations();
        assert!(ops.contains("q\n"));
        assert!(ops.contains("20.00 20.00 60.00 60.00 re\n"));
        assert!(ops.contains("W\n"));
        assert!(ops.contains("1.000 0.000 0.000 rg\n"));
        assert!(ops.contains("0.00 0.00 100.00 100.00 re\n"));
        assert!(ops.contains("f\n"));
        assert!(ops.contains("Q\n"));
    }

    #[test]
    fn test_multiple_clipping_regions() {
        let mut ctx = GraphicsContext::new();

        // Test nested clipping regions
        ctx.save_state()
            .rect(0.0, 0.0, 200.0, 200.0)
            .clip()
            .save_state()
            .circle(100.0, 100.0, 50.0)
            .clip_even_odd()
            .set_fill_color(Color::blue())
            .rect(50.0, 50.0, 100.0, 100.0)
            .fill()
            .restore_state()
            .restore_state();

        let ops = ctx.operations();
        // Check for nested save/restore states
        let q_count = ops.matches("q\n").count();
        let q_restore_count = ops.matches("Q\n").count();
        assert_eq!(q_count, 2);
        assert_eq!(q_restore_count, 2);

        // Check for both clipping operations
        assert!(ops.contains("W\n"));
        assert!(ops.contains("W*\n"));
    }

    // ============= Additional Critical Method Tests =============

    #[test]
    fn test_move_to_and_line_to() {
        let mut ctx = GraphicsContext::new();
        ctx.move_to(100.0, 200.0).line_to(300.0, 400.0).stroke();

        let ops = ctx.generate_operations().unwrap();
        let ops_str = String::from_utf8_lossy(&ops);
        assert!(ops_str.contains("100.00 200.00 m"));
        assert!(ops_str.contains("300.00 400.00 l"));
        assert!(ops_str.contains("S"));
    }

    #[test]
    fn test_bezier_curve() {
        let mut ctx = GraphicsContext::new();
        ctx.move_to(0.0, 0.0)
            .curve_to(10.0, 20.0, 30.0, 40.0, 50.0, 60.0)
            .stroke();

        let ops = ctx.generate_operations().unwrap();
        let ops_str = String::from_utf8_lossy(&ops);
        assert!(ops_str.contains("0.00 0.00 m"));
        assert!(ops_str.contains("10.00 20.00 30.00 40.00 50.00 60.00 c"));
        assert!(ops_str.contains("S"));
    }

    #[test]
    fn test_circle_path() {
        let mut ctx = GraphicsContext::new();
        ctx.circle(100.0, 100.0, 50.0).fill();

        let ops = ctx.generate_operations().unwrap();
        let ops_str = String::from_utf8_lossy(&ops);
        // Circle should use bezier curves (c operator)
        assert!(ops_str.contains(" c"));
        assert!(ops_str.contains("f"));
    }

    #[test]
    fn test_path_closing() {
        let mut ctx = GraphicsContext::new();
        ctx.move_to(0.0, 0.0)
            .line_to(100.0, 0.0)
            .line_to(100.0, 100.0)
            .close_path()
            .stroke();

        let ops = ctx.generate_operations().unwrap();
        let ops_str = String::from_utf8_lossy(&ops);
        assert!(ops_str.contains("h")); // close path operator
        assert!(ops_str.contains("S"));
    }

    #[test]
    fn test_fill_and_stroke() {
        let mut ctx = GraphicsContext::new();
        ctx.rect(10.0, 10.0, 50.0, 50.0).fill_stroke();

        let ops = ctx.generate_operations().unwrap();
        let ops_str = String::from_utf8_lossy(&ops);
        assert!(ops_str.contains("10.00 10.00 50.00 50.00 re"));
        assert!(ops_str.contains("B")); // fill and stroke operator
    }

    #[test]
    fn test_color_settings() {
        let mut ctx = GraphicsContext::new();
        ctx.set_fill_color(Color::rgb(1.0, 0.0, 0.0))
            .set_stroke_color(Color::rgb(0.0, 1.0, 0.0))
            .rect(10.0, 10.0, 50.0, 50.0)
            .fill_stroke(); // This will write the colors

        assert_eq!(ctx.fill_color(), Color::rgb(1.0, 0.0, 0.0));
        assert_eq!(ctx.stroke_color(), Color::rgb(0.0, 1.0, 0.0));

        let ops = ctx.generate_operations().unwrap();
        let ops_str = String::from_utf8_lossy(&ops);
        assert!(ops_str.contains("1.000 0.000 0.000 rg")); // red fill
        assert!(ops_str.contains("0.000 1.000 0.000 RG")); // green stroke
    }

    #[test]
    fn test_line_styles() {
        let mut ctx = GraphicsContext::new();
        ctx.set_line_width(2.5)
            .set_line_cap(LineCap::Round)
            .set_line_join(LineJoin::Bevel);

        assert_eq!(ctx.line_width(), 2.5);

        let ops = ctx.generate_operations().unwrap();
        let ops_str = String::from_utf8_lossy(&ops);
        assert!(ops_str.contains("2.50 w")); // line width
        assert!(ops_str.contains("1 J")); // round line cap
        assert!(ops_str.contains("2 j")); // bevel line join
    }

    #[test]
    fn test_opacity_settings() {
        let mut ctx = GraphicsContext::new();
        ctx.set_opacity(0.5);

        assert_eq!(ctx.fill_opacity(), 0.5);
        assert_eq!(ctx.stroke_opacity(), 0.5);
        assert!(ctx.uses_transparency());

        ctx.set_fill_opacity(0.7).set_stroke_opacity(0.3);

        assert_eq!(ctx.fill_opacity(), 0.7);
        assert_eq!(ctx.stroke_opacity(), 0.3);
    }

    #[test]
    fn test_state_save_restore() {
        let mut ctx = GraphicsContext::new();
        ctx.save_state()
            .set_fill_color(Color::rgb(1.0, 0.0, 0.0))
            .restore_state();

        let ops = ctx.generate_operations().unwrap();
        let ops_str = String::from_utf8_lossy(&ops);
        assert!(ops_str.contains("q")); // save state
        assert!(ops_str.contains("Q")); // restore state
    }

    #[test]
    fn test_transformations() {
        let mut ctx = GraphicsContext::new();
        ctx.translate(100.0, 200.0).scale(2.0, 3.0).rotate(45.0);

        let ops = ctx.generate_operations().unwrap();
        let ops_str = String::from_utf8_lossy(&ops);
        assert!(ops_str.contains("1 0 0 1 100.00 200.00 cm")); // translate
        assert!(ops_str.contains("2.00 0 0 3.00 0 0 cm")); // scale
        assert!(ops_str.contains("cm")); // rotate matrix
    }

    #[test]
    fn test_custom_transform() {
        let mut ctx = GraphicsContext::new();
        ctx.transform(1.0, 0.5, 0.5, 1.0, 10.0, 20.0);

        let ops = ctx.generate_operations().unwrap();
        let ops_str = String::from_utf8_lossy(&ops);
        assert!(ops_str.contains("1.00 0.50 0.50 1.00 10.00 20.00 cm"));
    }

    #[test]
    fn test_rectangle_path() {
        let mut ctx = GraphicsContext::new();
        ctx.rectangle(25.0, 25.0, 150.0, 100.0).stroke();

        let ops = ctx.generate_operations().unwrap();
        let ops_str = String::from_utf8_lossy(&ops);
        assert!(ops_str.contains("25.00 25.00 150.00 100.00 re"));
        assert!(ops_str.contains("S"));
    }

    #[test]
    fn test_empty_operations() {
        let ctx = GraphicsContext::new();
        let ops = ctx.generate_operations().unwrap();
        assert!(ops.is_empty());
    }

    #[test]
    fn test_complex_path_operations() {
        let mut ctx = GraphicsContext::new();
        ctx.move_to(50.0, 50.0)
            .line_to(100.0, 50.0)
            .curve_to(125.0, 50.0, 150.0, 75.0, 150.0, 100.0)
            .line_to(150.0, 150.0)
            .close_path()
            .fill();

        let ops = ctx.generate_operations().unwrap();
        let ops_str = String::from_utf8_lossy(&ops);
        assert!(ops_str.contains("50.00 50.00 m"));
        assert!(ops_str.contains("100.00 50.00 l"));
        assert!(ops_str.contains("125.00 50.00 150.00 75.00 150.00 100.00 c"));
        assert!(ops_str.contains("150.00 150.00 l"));
        assert!(ops_str.contains("h"));
        assert!(ops_str.contains("f"));
    }

    #[test]
    fn test_graphics_state_dict_generation() {
        let mut ctx = GraphicsContext::new();

        // Without transparency, should return None
        assert!(ctx.generate_graphics_state_dict().is_none());

        // With transparency, should generate dict
        ctx.set_opacity(0.5);
        let dict = ctx.generate_graphics_state_dict();
        assert!(dict.is_some());
        let dict_str = dict.unwrap();
        assert!(dict_str.contains("/ca 0.5"));
        assert!(dict_str.contains("/CA 0.5"));
    }

    #[test]
    fn test_line_dash_pattern() {
        let mut ctx = GraphicsContext::new();
        let pattern = LineDashPattern {
            array: vec![3.0, 2.0],
            phase: 0.0,
        };
        ctx.set_line_dash_pattern(pattern);

        let ops = ctx.generate_operations().unwrap();
        let ops_str = String::from_utf8_lossy(&ops);
        assert!(ops_str.contains("[3.00 2.00] 0.00 d"));
    }

    #[test]
    fn test_miter_limit_setting() {
        let mut ctx = GraphicsContext::new();
        ctx.set_miter_limit(4.0);

        let ops = ctx.generate_operations().unwrap();
        let ops_str = String::from_utf8_lossy(&ops);
        assert!(ops_str.contains("4.00 M"));
    }

    #[test]
    fn test_line_cap_styles() {
        let mut ctx = GraphicsContext::new();

        ctx.set_line_cap(LineCap::Butt);
        let ops = ctx.generate_operations().unwrap();
        let ops_str = String::from_utf8_lossy(&ops);
        assert!(ops_str.contains("0 J"));

        let mut ctx = GraphicsContext::new();
        ctx.set_line_cap(LineCap::Round);
        let ops = ctx.generate_operations().unwrap();
        let ops_str = String::from_utf8_lossy(&ops);
        assert!(ops_str.contains("1 J"));

        let mut ctx = GraphicsContext::new();
        ctx.set_line_cap(LineCap::Square);
        let ops = ctx.generate_operations().unwrap();
        let ops_str = String::from_utf8_lossy(&ops);
        assert!(ops_str.contains("2 J"));
    }

    #[test]
    fn test_transparency_groups() {
        let mut ctx = GraphicsContext::new();

        // Test basic transparency group
        let group = TransparencyGroup::new()
            .with_isolated(true)
            .with_opacity(0.5);

        ctx.begin_transparency_group(group);
        assert!(ctx.in_transparency_group());

        // Draw something in the group
        ctx.rect(10.0, 10.0, 100.0, 100.0);
        ctx.fill();

        ctx.end_transparency_group();
        assert!(!ctx.in_transparency_group());

        // Check that operations contain transparency markers
        let ops = ctx.operations();
        assert!(ops.contains("% Begin Transparency Group"));
        assert!(ops.contains("% End Transparency Group"));
    }

    #[test]
    fn test_nested_transparency_groups() {
        let mut ctx = GraphicsContext::new();

        // First group
        let group1 = TransparencyGroup::isolated().with_opacity(0.8);
        ctx.begin_transparency_group(group1);
        assert!(ctx.in_transparency_group());

        // Nested group
        let group2 = TransparencyGroup::knockout().with_blend_mode(BlendMode::Multiply);
        ctx.begin_transparency_group(group2);

        // Draw in nested group
        ctx.circle(50.0, 50.0, 25.0);
        ctx.fill();

        // End nested group
        ctx.end_transparency_group();
        assert!(ctx.in_transparency_group()); // Still in first group

        // End first group
        ctx.end_transparency_group();
        assert!(!ctx.in_transparency_group());
    }

    #[test]
    fn test_line_join_styles() {
        let mut ctx = GraphicsContext::new();

        ctx.set_line_join(LineJoin::Miter);
        let ops = ctx.generate_operations().unwrap();
        let ops_str = String::from_utf8_lossy(&ops);
        assert!(ops_str.contains("0 j"));

        let mut ctx = GraphicsContext::new();
        ctx.set_line_join(LineJoin::Round);
        let ops = ctx.generate_operations().unwrap();
        let ops_str = String::from_utf8_lossy(&ops);
        assert!(ops_str.contains("1 j"));

        let mut ctx = GraphicsContext::new();
        ctx.set_line_join(LineJoin::Bevel);
        let ops = ctx.generate_operations().unwrap();
        let ops_str = String::from_utf8_lossy(&ops);
        assert!(ops_str.contains("2 j"));
    }

    #[test]
    fn test_rendering_intent() {
        let mut ctx = GraphicsContext::new();

        ctx.set_rendering_intent(RenderingIntent::AbsoluteColorimetric);
        assert_eq!(
            ctx.rendering_intent(),
            RenderingIntent::AbsoluteColorimetric
        );

        ctx.set_rendering_intent(RenderingIntent::Perceptual);
        assert_eq!(ctx.rendering_intent(), RenderingIntent::Perceptual);

        ctx.set_rendering_intent(RenderingIntent::Saturation);
        assert_eq!(ctx.rendering_intent(), RenderingIntent::Saturation);
    }

    #[test]
    fn test_flatness_tolerance() {
        let mut ctx = GraphicsContext::new();

        ctx.set_flatness(0.5);
        assert_eq!(ctx.flatness(), 0.5);

        let ops = ctx.generate_operations().unwrap();
        let ops_str = String::from_utf8_lossy(&ops);
        assert!(ops_str.contains("0.50 i"));
    }

    #[test]
    fn test_smoothness_tolerance() {
        let mut ctx = GraphicsContext::new();

        ctx.set_smoothness(0.1);
        assert_eq!(ctx.smoothness(), 0.1);
    }

    #[test]
    fn test_bezier_curves() {
        let mut ctx = GraphicsContext::new();

        // Cubic Bezier
        ctx.move_to(10.0, 10.0);
        ctx.curve_to(20.0, 10.0, 30.0, 20.0, 30.0, 30.0);

        let ops = ctx.generate_operations().unwrap();
        let ops_str = String::from_utf8_lossy(&ops);
        assert!(ops_str.contains("10.00 10.00 m"));
        assert!(ops_str.contains("c")); // cubic curve
    }

    #[test]
    fn test_clipping_path() {
        let mut ctx = GraphicsContext::new();

        ctx.rectangle(10.0, 10.0, 100.0, 100.0);
        ctx.clip();

        let ops = ctx.generate_operations().unwrap();
        let ops_str = String::from_utf8_lossy(&ops);
        assert!(ops_str.contains("W"));
    }

    #[test]
    fn test_even_odd_clipping() {
        let mut ctx = GraphicsContext::new();

        ctx.rectangle(10.0, 10.0, 100.0, 100.0);
        ctx.clip_even_odd();

        let ops = ctx.generate_operations().unwrap();
        let ops_str = String::from_utf8_lossy(&ops);
        assert!(ops_str.contains("W*"));
    }

    #[test]
    fn test_color_creation() {
        // Test color creation methods
        let gray = Color::gray(0.5);
        assert_eq!(gray, Color::Gray(0.5));

        let rgb = Color::rgb(0.2, 0.4, 0.6);
        assert_eq!(rgb, Color::Rgb(0.2, 0.4, 0.6));

        let cmyk = Color::cmyk(0.1, 0.2, 0.3, 0.4);
        assert_eq!(cmyk, Color::Cmyk(0.1, 0.2, 0.3, 0.4));

        // Test predefined colors
        assert_eq!(Color::black(), Color::Gray(0.0));
        assert_eq!(Color::white(), Color::Gray(1.0));
        assert_eq!(Color::red(), Color::Rgb(1.0, 0.0, 0.0));
    }

    #[test]
    fn test_extended_graphics_state() {
        let mut ctx = GraphicsContext::new();

        // Test that we can create and use an extended graphics state
        let extgstate = ExtGState::new();

        // We should be able to create the state without errors
        assert!(ctx.generate_operations().is_ok());
    }

    #[test]
    fn test_path_construction_methods() {
        let mut ctx = GraphicsContext::new();

        // Test basic path construction methods that exist
        ctx.move_to(10.0, 10.0);
        ctx.line_to(20.0, 20.0);
        ctx.curve_to(30.0, 30.0, 40.0, 40.0, 50.0, 50.0);
        ctx.rect(60.0, 60.0, 30.0, 30.0);
        ctx.circle(100.0, 100.0, 25.0);
        ctx.close_path();

        let ops = ctx.generate_operations().unwrap();
        assert!(!ops.is_empty());
    }

    #[test]
    fn test_graphics_context_clone_advanced() {
        let mut ctx = GraphicsContext::new();
        ctx.set_fill_color(Color::rgb(1.0, 0.0, 0.0));
        ctx.set_line_width(5.0);

        let cloned = ctx.clone();
        assert_eq!(cloned.fill_color(), Color::rgb(1.0, 0.0, 0.0));
        assert_eq!(cloned.line_width(), 5.0);
    }

    #[test]
    fn test_basic_drawing_operations() {
        let mut ctx = GraphicsContext::new();

        // Test that we can at least create a basic drawing
        ctx.move_to(50.0, 50.0);
        ctx.line_to(100.0, 100.0);
        ctx.stroke();

        let ops = ctx.generate_operations().unwrap();
        let ops_str = String::from_utf8_lossy(&ops);
        assert!(ops_str.contains("m")); // move
        assert!(ops_str.contains("l")); // line
        assert!(ops_str.contains("S")); // stroke
    }

    #[test]
    fn test_graphics_state_stack() {
        let mut ctx = GraphicsContext::new();

        // Initial state
        ctx.set_fill_color(Color::black());

        // Save and change
        ctx.save_state();
        ctx.set_fill_color(Color::red());
        assert_eq!(ctx.fill_color(), Color::red());

        // Save again and change
        ctx.save_state();
        ctx.set_fill_color(Color::blue());
        assert_eq!(ctx.fill_color(), Color::blue());

        // Restore once
        ctx.restore_state();
        assert_eq!(ctx.fill_color(), Color::red());

        // Restore again
        ctx.restore_state();
        assert_eq!(ctx.fill_color(), Color::black());
    }
}
