pub mod calibrated_color;
pub mod clipping;
pub(crate) mod color;
mod color_profiles;
pub mod devicen_color;
pub mod extraction;
pub mod form_xobject;
mod indexed_color;
pub mod lab_color;
pub(crate) mod ops;
pub mod page_color_space;
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
pub use page_color_space::{DeviceColorSpace, PageColorSpace, ParameterisedFamily};
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

/// Saved graphics state for save/restore operations.
/// Using `Arc<str>` for `font_name` makes `Clone` O(1) — only increments the reference count.
#[derive(Clone)]
struct GraphicsState {
    fill_color: Color,
    stroke_color: Color,
    font_name: Option<Arc<str>>,
    font_size: f64,
    is_custom_font: bool,
}

#[derive(Clone)]
pub struct GraphicsContext {
    operations: Vec<ops::Op>,
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
    state_stack: Vec<GraphicsState>,
    current_font_name: Option<Arc<str>>,
    current_font_size: f64,
    // Whether the current font is a custom (Type0/CID) font requiring Unicode encoding
    is_custom_font: bool,
    // Character tracking for font subsetting, bucketed by custom-font name
    // (issue #204 — builtin fonts are not tracked because they don't need
    // subsetting; a single global set across all fonts caused every font's
    // subset to include chars drawn with a different font, doubling emitted
    // size when two fonts in the same family were registered).
    used_characters_by_font: HashMap<String, HashSet<char>>,
    // Glyph mapping for Unicode fonts (Unicode code point -> Glyph ID)
    glyph_mapping: Option<HashMap<u32, u16>>,
    // Transparency group stack for nested groups
    transparency_stack: Vec<TransparencyGroupState>,
}

/// Encode a Unicode character as a CID hex value for Type0/Identity-H fonts.
/// BMP characters (U+0000..U+FFFF) are written as 4-hex-digit values.
/// Supplementary plane characters (U+10000..U+10FFFF) are written as UTF-16BE surrogate pairs.
fn encode_char_as_cid(ch: char, buf: &mut String) {
    let code = ch as u32;
    if code <= 0xFFFF {
        write!(buf, "{:04X}", code).expect("Writing to string should never fail");
    } else {
        // UTF-16BE surrogate pair for supplementary planes
        let adjusted = code - 0x10000;
        let high = ((adjusted >> 10) & 0x3FF) + 0xD800;
        let low = (adjusted & 0x3FF) + 0xDC00;
        write!(buf, "{:04X}{:04X}", high, low).expect("Writing to string should never fail");
    }
}

impl Default for GraphicsContext {
    fn default() -> Self {
        Self::new()
    }
}

impl GraphicsContext {
    pub fn new() -> Self {
        Self {
            operations: Vec::new(),
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
            is_custom_font: false,
            used_characters_by_font: HashMap::new(),
            glyph_mapping: None,
            transparency_stack: Vec::new(),
        }
    }

    pub fn move_to(&mut self, x: f64, y: f64) -> &mut Self {
        self.operations.push(ops::Op::MoveTo { x, y });
        self
    }

    pub fn line_to(&mut self, x: f64, y: f64) -> &mut Self {
        self.operations.push(ops::Op::LineTo { x, y });
        self
    }

    pub fn curve_to(&mut self, x1: f64, y1: f64, x2: f64, y2: f64, x3: f64, y3: f64) -> &mut Self {
        self.operations.push(ops::Op::CurveTo {
            x1,
            y1,
            x2,
            y2,
            x3,
            y3,
        });
        self
    }

    pub fn rect(&mut self, x: f64, y: f64, width: f64, height: f64) -> &mut Self {
        self.operations.push(ops::Op::Rect {
            x,
            y,
            w: width,
            h: height,
        });
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
        self.operations.push(ops::Op::ClosePath);
        self
    }

    pub fn stroke(&mut self) -> &mut Self {
        self.apply_pending_extgstate().unwrap_or_default();
        self.apply_stroke_color();
        self.operations.push(ops::Op::Stroke);
        self
    }

    pub fn fill(&mut self) -> &mut Self {
        self.apply_pending_extgstate().unwrap_or_default();
        self.apply_fill_color();
        self.operations.push(ops::Op::FillNonZero);
        self
    }

    pub fn fill_stroke(&mut self) -> &mut Self {
        self.apply_pending_extgstate().unwrap_or_default();
        self.apply_fill_color();
        self.apply_stroke_color();
        self.operations.push(ops::Op::FillStroke);
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

    /// Emit a colour-space selection followed by its components.
    ///
    /// Shared by every named colour setter (ICC, calibrated, Lab): each
    /// produces a colour-space operator (`cs`/`CS`) immediately followed by its
    /// component operator (`sc`/`SC`) per ISO 32000-1 §8.6.8.
    fn push_color_space_and_components(
        &mut self,
        space: ops::Op,
        components: ops::Op,
    ) -> &mut Self {
        self.operations.push(space);
        self.operations.push(components);
        self
    }

    /// Set fill color using an ICC-based color space already registered on the
    /// page under `/Resources/ColorSpace/<name>` (see [`crate::page::Page::add_color_space`]).
    /// `components` are the raw color values for the profile's channel count
    /// (1=Gray, 3=RGB/Lab, 4=CMYK).
    ///
    /// ICC profiles are named dynamically by [`IccProfileManager`], so the
    /// resource name is supplied by the caller rather than hardcoded.
    ///
    /// `components` must be non-empty: an empty list would emit a bare `sc`
    /// operator with no operands, invalid per ISO 32000-1 §8.6.8.
    pub fn set_fill_color_icc(
        &mut self,
        name: impl Into<String>,
        components: Vec<f64>,
    ) -> &mut Self {
        debug_assert!(
            !components.is_empty(),
            "ICC fill colour components must not be empty"
        );
        self.push_color_space_and_components(
            ops::Op::SetFillColorSpace(name.into()),
            ops::Op::SetFillColorComponents(components),
        )
    }

    /// Set stroke color using an ICC-based color space already registered on
    /// the page under `/Resources/ColorSpace/<name>` (see
    /// [`crate::page::Page::add_color_space`]). See [`Self::set_fill_color_icc`].
    ///
    /// `components` must be non-empty (see [`Self::set_fill_color_icc`]).
    pub fn set_stroke_color_icc(
        &mut self,
        name: impl Into<String>,
        components: Vec<f64>,
    ) -> &mut Self {
        debug_assert!(
            !components.is_empty(),
            "ICC stroke colour components must not be empty"
        );
        self.push_color_space_and_components(
            ops::Op::SetStrokeColorSpace(name.into()),
            ops::Op::SetStrokeColorComponents(components),
        )
    }

    /// Set fill color using a calibrated color space registered under a
    /// caller-supplied resource name (`/Resources/ColorSpace/<name>`).
    ///
    /// Unlike [`Self::set_fill_color_calibrated`], which always references the
    /// single default `CalGray1`/`CalRGB1` slot, this accepts the name of any
    /// registered calibrated space — removing the one-calibrated-space-per-page
    /// limitation.
    pub fn set_fill_color_calibrated_named(
        &mut self,
        name: impl Into<String>,
        color: CalibratedColor,
    ) -> &mut Self {
        self.push_color_space_and_components(
            ops::Op::SetFillColorSpace(name.into()),
            ops::Op::SetFillColorComponents(color.values()),
        )
    }

    /// Set stroke color using a calibrated color space registered under a
    /// caller-supplied resource name. See [`Self::set_fill_color_calibrated_named`].
    pub fn set_stroke_color_calibrated_named(
        &mut self,
        name: impl Into<String>,
        color: CalibratedColor,
    ) -> &mut Self {
        self.push_color_space_and_components(
            ops::Op::SetStrokeColorSpace(name.into()),
            ops::Op::SetStrokeColorComponents(color.values()),
        )
    }

    /// Set fill color using a Lab color space registered under a
    /// caller-supplied resource name (`/Resources/ColorSpace/<name>`).
    ///
    /// Companion to [`Self::set_fill_color_lab`] (which references the default
    /// `Lab1` slot); accepts any registered Lab space so multiple Lab spaces
    /// can coexist on one page.
    pub fn set_fill_color_lab_named(
        &mut self,
        name: impl Into<String>,
        color: LabColor,
    ) -> &mut Self {
        self.push_color_space_and_components(
            ops::Op::SetFillColorSpace(name.into()),
            ops::Op::SetFillColorComponents(color.values()),
        )
    }

    /// Set stroke color using a Lab color space registered under a
    /// caller-supplied resource name. See [`Self::set_fill_color_lab_named`].
    pub fn set_stroke_color_lab_named(
        &mut self,
        name: impl Into<String>,
        color: LabColor,
    ) -> &mut Self {
        self.push_color_space_and_components(
            ops::Op::SetStrokeColorSpace(name.into()),
            ops::Op::SetStrokeColorComponents(color.values()),
        )
    }

    /// Set fill color using calibrated color space, referencing the default
    /// `CalGray1`/`CalRGB1` resource slot.
    ///
    /// Delegates to [`Self::set_fill_color_calibrated_named`] with the default
    /// name; behaviour is unchanged for existing callers.
    pub fn set_fill_color_calibrated(&mut self, color: CalibratedColor) -> &mut Self {
        let cs_name = match &color {
            CalibratedColor::Gray(_, _) => "CalGray1",
            CalibratedColor::Rgb(_, _) => "CalRGB1",
        };
        self.set_fill_color_calibrated_named(cs_name, color)
    }

    /// Set stroke color using calibrated color space, referencing the default
    /// `CalGray1`/`CalRGB1` resource slot.
    ///
    /// Delegates to [`Self::set_stroke_color_calibrated_named`] with the
    /// default name; behaviour is unchanged for existing callers.
    pub fn set_stroke_color_calibrated(&mut self, color: CalibratedColor) -> &mut Self {
        let cs_name = match &color {
            CalibratedColor::Gray(_, _) => "CalGray1",
            CalibratedColor::Rgb(_, _) => "CalRGB1",
        };
        self.set_stroke_color_calibrated_named(cs_name, color)
    }

    /// Set fill color using Lab color space, referencing the default `Lab1`
    /// resource slot.
    ///
    /// Delegates to [`Self::set_fill_color_lab_named`] with the default name;
    /// behaviour is unchanged for existing callers.
    pub fn set_fill_color_lab(&mut self, color: LabColor) -> &mut Self {
        self.set_fill_color_lab_named("Lab1", color)
    }

    /// Set stroke color using Lab color space, referencing the default `Lab1`
    /// resource slot.
    ///
    /// Delegates to [`Self::set_stroke_color_lab_named`] with the default name;
    /// behaviour is unchanged for existing callers.
    pub fn set_stroke_color_lab(&mut self, color: LabColor) -> &mut Self {
        self.set_stroke_color_lab_named("Lab1", color)
    }

    pub fn set_line_width(&mut self, width: f64) -> &mut Self {
        self.line_width = width;
        self.operations.push(ops::Op::SetLineWidth(width));
        self
    }

    pub fn set_line_cap(&mut self, cap: LineCap) -> &mut Self {
        self.current_line_cap = cap;
        self.operations.push(ops::Op::SetLineCap(cap as u8));
        self
    }

    pub fn set_line_join(&mut self, join: LineJoin) -> &mut Self {
        self.current_line_join = join;
        self.operations.push(ops::Op::SetLineJoin(join as u8));
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
        self.operations.push(ops::Op::SaveState);
        self.save_clipping_state();
        // Save color + font state
        self.state_stack.push(GraphicsState {
            fill_color: self.current_color,
            stroke_color: self.stroke_color,
            font_name: self.current_font_name.clone(),
            font_size: self.current_font_size,
            is_custom_font: self.is_custom_font,
        });
        self
    }

    pub fn restore_state(&mut self) -> &mut Self {
        self.operations.push(ops::Op::RestoreState);
        self.restore_clipping_state();
        // Restore color + font state
        if let Some(state) = self.state_stack.pop() {
            self.current_color = state.fill_color;
            self.stroke_color = state.stroke_color;
            self.current_font_name = state.font_name;
            self.current_font_size = state.font_size;
            self.is_custom_font = state.is_custom_font;
        }
        self
    }

    /// Begin a transparency group
    /// ISO 32000-1:2008 Section 11.4
    pub fn begin_transparency_group(&mut self, group: TransparencyGroup) -> &mut Self {
        // Save current state
        self.save_state();

        // Mark beginning of transparency group with special comment
        self.operations
            .push(ops::Op::Comment("Begin Transparency Group".to_string()));

        // Apply group settings via ExtGState
        let mut extgstate = ExtGState::new();
        extgstate = extgstate.with_blend_mode(group.blend_mode.clone());
        extgstate.alpha_fill = Some(group.opacity as f64);
        extgstate.alpha_stroke = Some(group.opacity as f64);

        // Apply the ExtGState
        self.pending_extgstate = Some(extgstate);
        let _ = self.apply_pending_extgstate();

        // Push group state onto the stack. Pre-2.7.0 we also serialised
        // the entire ops buffer into a `saved_state` snapshot here, but
        // the snapshot was never consumed — both fields were dead code.
        // Removed in v2.7.0 (review finding).
        self.transparency_stack
            .push(TransparencyGroupState::new(group));

        self
    }

    /// End a transparency group
    pub fn end_transparency_group(&mut self) -> &mut Self {
        if let Some(_group_state) = self.transparency_stack.pop() {
            // Mark end of transparency group
            self.operations
                .push(ops::Op::Comment("End Transparency Group".to_string()));

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
        self.operations.push(ops::Op::Cm {
            a: 1.0,
            b: 0.0,
            c: 0.0,
            d: 1.0,
            e: tx,
            f: ty,
        });
        self
    }

    pub fn scale(&mut self, sx: f64, sy: f64) -> &mut Self {
        self.operations.push(ops::Op::Cm {
            a: sx,
            b: 0.0,
            c: 0.0,
            d: sy,
            e: 0.0,
            f: 0.0,
        });
        self
    }

    pub fn rotate(&mut self, angle: f64) -> &mut Self {
        let cos = angle.cos();
        let sin = angle.sin();
        // Rotation historically used `{:.6}` precision; the IR uses `{:.2}`
        // throughout the v2.7.0 refactor for consistency. The behavioural
        // change is documented in CHANGELOG (2.7.0 cm matrix format).
        self.operations.push(ops::Op::Cm {
            a: cos,
            b: sin,
            c: -sin,
            d: cos,
            e: 0.0,
            f: 0.0,
        });
        self
    }

    pub fn transform(&mut self, a: f64, b: f64, c: f64, d: f64, e: f64, f: f64) -> &mut Self {
        self.operations.push(ops::Op::Cm { a, b, c, d, e, f });
        self
    }

    pub fn rectangle(&mut self, x: f64, y: f64, width: f64, height: f64) -> &mut Self {
        self.rect(x, y, width, height)
    }

    pub fn draw_image(
        &mut self,
        image_name: impl Into<String>,
        x: f64,
        y: f64,
        width: f64,
        height: f64,
    ) -> &mut Self {
        // Save graphics state
        self.save_state();

        // Set up transformation matrix for image placement
        // PDF coordinate system has origin at bottom-left, so we need to translate and scale
        self.operations.push(ops::Op::Cm {
            a: width,
            b: 0.0,
            c: 0.0,
            d: height,
            e: x,
            f: y,
        });

        // Draw the image XObject
        self.operations
            .push(ops::Op::InvokeXObject(image_name.into()));

        // Restore graphics state
        self.restore_state();

        self
    }

    /// Draw an image with transparency support (soft mask)
    /// This method handles images with alpha channels or soft masks
    pub fn draw_image_with_transparency(
        &mut self,
        image_name: impl Into<String>,
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
            self.operations.push(ops::Op::SetExtGState(gs_name));
        }

        // Set up transformation matrix for image placement
        self.operations.push(ops::Op::Cm {
            a: width,
            b: 0.0,
            c: 0.0,
            d: height,
            e: x,
            f: y,
        });

        // Draw the image XObject
        self.operations
            .push(ops::Op::InvokeXObject(image_name.into()));

        // If we had a mask, reset the soft mask to None
        if mask_name.is_some() {
            // Create an ExtGState that removes the soft mask
            let mut reset_extgstate = ExtGState::new();
            reset_extgstate.set_soft_mask_none();

            let gs_name = self
                .extgstate_manager
                .add_state(reset_extgstate)
                .unwrap_or_else(|_| "GS2".to_string());
            self.operations.push(ops::Op::SetExtGState(gs_name));
        }

        // Restore graphics state
        self.restore_state();

        self
    }

    fn apply_stroke_color(&mut self) {
        // Single source of truth for stroke-colour emission across
        // `TextContext`, `TextFlowContext`, and `GraphicsContext` — see
        // `graphics::color::write_stroke_color` (issues #220 + #221).
        // After the IR migration the operator is pushed as `Op::SetStrokeColor`
        // and `serialize_ops` delegates to `write_stroke_color_bytes`.
        self.operations
            .push(ops::Op::SetStrokeColor(self.stroke_color));
    }

    fn apply_fill_color(&mut self) {
        // Single source of truth for fill-colour emission. See sibling
        // `apply_stroke_color`. The IR delegates emission to
        // `write_fill_color_bytes`, preserving the NaN/inf sanitisation
        // and device-space selection from 2.6.0.
        self.operations
            .push(ops::Op::SetFillColor(self.current_color));
    }

    pub(crate) fn generate_operations(&self) -> Result<Vec<u8>> {
        let mut buf = Vec::new();
        ops::serialize_ops(&mut buf, &self.operations);
        Ok(buf)
    }

    /// Take ownership of the accumulated `Op` buffer, leaving an empty
    /// `Vec` in its place. Used by `Page` to flush the graphics buffer
    /// into a unified content stream when the caller switches contexts
    /// (issue #227 — preserves PDF painter-model call order across
    /// `Page::graphics()` / `Page::text()` switches).
    ///
    /// State fields (`current_color`, `line_width`, …) are unaffected so
    /// the next chain of calls on this context resumes with the same
    /// graphics state.
    pub(crate) fn drain_ops(&mut self) -> Vec<ops::Op> {
        std::mem::take(&mut self.operations)
    }

    /// Read-only access to the operation list (used by `Page` to peek
    /// at whether a flush is required without taking the buffer).
    pub(crate) fn ops_slice(&self) -> &[ops::Op] {
        &self.operations
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
            write!(&mut dict, " /ca {:.3}", self.fill_opacity)
                .expect("Writing to string should never fail");
        }

        if self.stroke_opacity < 1.0 {
            write!(&mut dict, " /CA {:.3}", self.stroke_opacity)
                .expect("Writing to string should never fail");
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

    /// Get the operations as a serialised PDF content-stream `String`.
    ///
    /// Pre-2.7.0 this returned `&str`. The IR migration replaced the
    /// internal `String` buffer with a typed `Vec<Op>`, so the legacy
    /// borrow is materialised on demand. Internal callers prefer
    /// `generate_operations()` which returns the byte buffer directly.
    pub fn operations(&self) -> String {
        ops::ops_to_string(&self.operations)
    }

    /// Get the operations as a serialised content-stream `String` (alias
    /// retained for legacy tests; mirrors `operations()`).
    pub fn get_operations(&self) -> String {
        ops::ops_to_string(&self.operations)
    }

    /// Clear all operations
    pub fn clear(&mut self) {
        self.operations.clear();
    }

    /// Begin a text object
    pub fn begin_text(&mut self) -> &mut Self {
        self.operations.push(ops::Op::BeginText);
        self
    }

    /// End a text object
    pub fn end_text(&mut self) -> &mut Self {
        self.operations.push(ops::Op::EndText);
        self
    }

    /// Set font and size
    pub fn set_font(&mut self, font: Font, size: f64) -> &mut Self {
        self.operations.push(ops::Op::SetFont {
            name: font.pdf_name(),
            size,
        });

        // Track font name, size, and type for Unicode detection and proper font handling
        match &font {
            Font::Custom(name) => {
                self.current_font_name = Some(Arc::from(name.as_str()));
                self.current_font_size = size;
                self.is_custom_font = true;
            }
            _ => {
                self.current_font_name = Some(Arc::from(font.pdf_name().as_str()));
                self.current_font_size = size;
                self.is_custom_font = false;
            }
        }

        self
    }

    /// Set text position
    pub fn set_text_position(&mut self, x: f64, y: f64) -> &mut Self {
        self.operations.push(ops::Op::SetTextPosition { x, y });
        self
    }

    /// Show text
    ///
    /// For custom (Type0/CID) fonts, text is encoded as Unicode code points (CIDs).
    /// BMP characters (U+0000..U+FFFF) are written as 4-hex-digit values.
    /// Supplementary plane characters (U+10000..U+10FFFF) use UTF-16BE surrogate pairs.
    /// For standard fonts, text is encoded as literal PDF strings.
    pub fn show_text(&mut self, text: &str) -> Result<&mut Self> {
        // Track used characters for font subsetting, bucketed by font name
        // (issue #204). Builtin fonts skip tracking — subsetting only
        // applies to custom Type0/CID fonts.
        self.record_used_chars(text);

        if self.is_custom_font {
            // For custom fonts (CJK/Type0), encode as hex string with Unicode code points as CIDs
            let mut hex = String::new();
            for ch in text.chars() {
                encode_char_as_cid(ch, &mut hex);
            }
            self.operations.push(ops::Op::ShowTextHex(hex.into_bytes()));
        } else {
            // For standard fonts, escape special characters in PDF literal string
            let mut escaped = String::new();
            for ch in text.chars() {
                match ch {
                    '(' => escaped.push_str("\\("),
                    ')' => escaped.push_str("\\)"),
                    '\\' => escaped.push_str("\\\\"),
                    '\n' => escaped.push_str("\\n"),
                    '\r' => escaped.push_str("\\r"),
                    '\t' => escaped.push_str("\\t"),
                    _ => escaped.push(ch),
                }
            }
            self.operations
                .push(ops::Op::ShowText(escaped.into_bytes()));
        }
        Ok(self)
    }

    /// Set word spacing for text justification
    pub fn set_word_spacing(&mut self, spacing: f64) -> &mut Self {
        self.operations.push(ops::Op::SetWordSpacing(spacing));
        self
    }

    /// Set character spacing
    pub fn set_character_spacing(&mut self, spacing: f64) -> &mut Self {
        self.operations.push(ops::Op::SetCharSpacing(spacing));
        self
    }

    /// Show justified text with automatic word spacing calculation
    pub fn show_justified_text(&mut self, text: &str, target_width: f64) -> Result<&mut Self> {
        // Split text into words
        let words: Vec<&str> = text.split_whitespace().collect();
        if words.len() <= 1 {
            // Can't justify single word or empty text
            return self.show_text(text);
        }

        // Calculate natural width of text without extra spacing
        let text_without_spaces = words.join("");
        let natural_text_width = self.estimate_text_width_simple(&text_without_spaces);
        let space_width = self.estimate_text_width_simple(" ");
        let natural_width = natural_text_width + (space_width * (words.len() - 1) as f64);

        // Calculate extra spacing needed per word gap
        let extra_space_needed = target_width - natural_width;
        let word_gaps = (words.len() - 1) as f64;

        if word_gaps > 0.0 && extra_space_needed > 0.0 {
            let extra_word_spacing = extra_space_needed / word_gaps;

            // Set word spacing
            self.set_word_spacing(extra_word_spacing);

            // Show text (spaces will be expanded automatically)
            self.show_text(text)?;

            // Reset word spacing to default
            self.set_word_spacing(0.0);
        } else {
            // Fallback to normal text display
            self.show_text(text)?;
        }

        Ok(self)
    }

    /// Simple text width estimation (placeholder implementation)
    fn estimate_text_width_simple(&self, text: &str) -> f64 {
        // This is a simplified estimation. In a full implementation,
        // you would use actual font metrics.
        let font_size = self.current_font_size;
        text.len() as f64 * font_size * 0.6 // Approximate width factor
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
        self.operations
            .push(ops::Op::SetDashPatternRaw(pattern.to_pdf_string()));
        self
    }

    /// Set line dash pattern to solid (no dashes)
    pub fn set_line_solid(&mut self) -> &mut Self {
        self.current_dash_pattern = None;
        self.operations
            .push(ops::Op::SetDashPatternRaw("[] 0".to_string()));
        self
    }

    /// Set miter limit
    pub fn set_miter_limit(&mut self, limit: f64) -> &mut Self {
        self.current_miter_limit = limit.max(1.0);
        self.operations
            .push(ops::Op::SetMiterLimit(self.current_miter_limit));
        self
    }

    /// Set rendering intent
    pub fn set_rendering_intent(&mut self, intent: RenderingIntent) -> &mut Self {
        self.current_rendering_intent = intent;
        self.operations
            .push(ops::Op::SetRenderingIntent(intent.pdf_name().to_string()));
        self
    }

    /// Set flatness tolerance
    pub fn set_flatness(&mut self, flatness: f64) -> &mut Self {
        self.current_flatness = flatness.clamp(0.0, 100.0);
        self.operations
            .push(ops::Op::SetFlatness(self.current_flatness));
        self
    }

    /// Apply an ExtGState dictionary immediately
    pub fn apply_extgstate(&mut self, state: ExtGState) -> Result<&mut Self> {
        let state_name = self.extgstate_manager.add_state(state)?;
        self.operations.push(ops::Op::SetExtGState(state_name));
        Ok(self)
    }

    /// Store an ExtGState to be applied before the next drawing operation
    #[allow(dead_code)]
    fn set_pending_extgstate(&mut self, state: ExtGState) {
        self.pending_extgstate = Some(state);
    }

    /// Apply any pending ExtGState before drawing
    fn apply_pending_extgstate(&mut self) -> Result<()> {
        if let Some(state) = self.pending_extgstate.take() {
            let state_name = self.extgstate_manager.add_state(state)?;
            self.operations.push(ops::Op::SetExtGState(state_name));
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
        self.apply_extgstate(state)
    }

    /// Set alpha for stroke operations only
    pub fn set_alpha_stroke(&mut self, alpha: f64) -> Result<&mut Self> {
        let state = ExtGState::new().with_alpha_stroke(alpha);
        self.apply_extgstate(state)
    }

    /// Set alpha for fill operations only
    pub fn set_alpha_fill(&mut self, alpha: f64) -> Result<&mut Self> {
        let state = ExtGState::new().with_alpha_fill(alpha);
        self.apply_extgstate(state)
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

    /// Add a command to the operations.
    ///
    /// Untyped escape hatch — bytes are emitted verbatim with a trailing
    /// newline. Used by callers that need to inject operators not yet
    /// modelled as `Op` variants. New code should prefer the typed
    /// methods on `GraphicsContext`.
    pub fn add_command(&mut self, command: &str) {
        let mut bytes = command.as_bytes().to_vec();
        bytes.push(b'\n');
        self.operations.push(ops::Op::Raw(bytes));
    }

    /// Create clipping path from current path using non-zero winding rule
    pub fn clip(&mut self) -> &mut Self {
        self.operations.push(ops::Op::ClipNonZero);
        self
    }

    /// Create clipping path from current path using even-odd rule
    pub fn clip_even_odd(&mut self) -> &mut Self {
        self.operations.push(ops::Op::ClipEvenOdd);
        self
    }

    /// Create clipping path and stroke it
    pub fn clip_stroke(&mut self) -> &mut Self {
        self.apply_stroke_color();
        self.operations.push(ops::Op::ClipStroke);
        self
    }

    /// Set a custom clipping path
    pub fn set_clipping_path(&mut self, path: ClippingPath) -> Result<&mut Self> {
        let ops_str = path.to_pdf_operations()?;
        self.operations.push(ops::Op::Raw(ops_str.into_bytes()));
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
        // Emit Tf operator to the content stream (consistent with set_font)
        self.operations.push(ops::Op::SetFont {
            name: font_name.to_string(),
            size,
        });

        self.current_font_name = Some(Arc::from(font_name));
        self.current_font_size = size;
        self.is_custom_font = true;

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
        // Track used characters for font subsetting, bucketed by font name
        // (issue #204).
        self.record_used_chars(text);

        // Detect if text needs Unicode encoding: custom fonts always use hex,
        // and text with non-Latin-1 characters also needs Unicode encoding
        let needs_unicode = self.is_custom_font || text.chars().any(|c| c as u32 > 255);

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
            tracing::debug!("Warning: Text contains Unicode characters but using Latin-1 font. Characters will be replaced with '?'");
        }

        self.operations.push(ops::Op::BeginText);
        self.apply_fill_color();
        self.push_active_font();
        self.operations.push(ops::Op::SetTextPosition { x, y });

        // Encode text as a literal string (parentheses, WinAnsi octal escapes
        // for 128–255, '?' fallback for code points beyond Latin-1).
        let mut buf = String::new();
        for ch in text.chars() {
            let code = ch as u32;
            if code <= 127 {
                match ch {
                    '(' => buf.push_str("\\("),
                    ')' => buf.push_str("\\)"),
                    '\\' => buf.push_str("\\\\"),
                    '\n' => buf.push_str("\\n"),
                    '\r' => buf.push_str("\\r"),
                    '\t' => buf.push_str("\\t"),
                    _ => buf.push(ch),
                }
            } else if code <= 255 {
                use std::fmt::Write as _;
                write!(&mut buf, "\\{code:03o}").expect("write to String never fails");
            } else {
                buf.push('?');
            }
        }
        self.operations.push(ops::Op::ShowText(buf.into_bytes()));
        self.operations.push(ops::Op::EndText);

        Ok(self)
    }

    /// Internal: Draw text with Unicode encoding (Type0/CID)
    fn draw_with_unicode_encoding(&mut self, text: &str, x: f64, y: f64) -> Result<&mut Self> {
        self.operations.push(ops::Op::BeginText);
        self.apply_fill_color();
        self.push_active_font();
        self.operations.push(ops::Op::SetTextPosition { x, y });

        let mut hex = String::new();
        for ch in text.chars() {
            encode_char_as_cid(ch, &mut hex);
        }
        self.operations.push(ops::Op::ShowTextHex(hex.into_bytes()));
        self.operations.push(ops::Op::EndText);

        Ok(self)
    }

    /// Push a `Tf` operator for the active font, falling back to
    /// `/Helvetica` at the current size when no font has been set.
    /// Shared by `draw_with_simple_encoding`, `draw_with_unicode_encoding`,
    /// and the deprecated `draw_text_*` aliases.
    fn push_active_font(&mut self) {
        let name = self
            .current_font_name
            .as_deref()
            .unwrap_or("Helvetica")
            .to_string();
        self.operations.push(ops::Op::SetFont {
            name,
            size: self.current_font_size,
        });
    }

    /// Legacy: Draw text with hex encoding (kept for compatibility)
    #[deprecated(note = "Use draw_text() which automatically detects encoding")]
    pub fn draw_text_hex(&mut self, text: &str, x: f64, y: f64) -> Result<&mut Self> {
        self.operations.push(ops::Op::BeginText);
        self.apply_fill_color();
        self.push_active_font();
        self.operations.push(ops::Op::SetTextPosition { x, y });

        let mut hex = String::new();
        for ch in text.chars() {
            use std::fmt::Write as _;
            if ch as u32 <= 255 {
                write!(&mut hex, "{:02X}", ch as u8).expect("write to String never fails");
            } else {
                hex.push_str("3F");
            }
        }
        self.operations.push(ops::Op::ShowTextHex(hex.into_bytes()));
        self.operations.push(ops::Op::EndText);

        Ok(self)
    }

    /// Legacy: Draw text with Type0 font encoding (kept for compatibility)
    #[deprecated(note = "Use draw_text() which automatically detects encoding")]
    pub fn draw_text_cid(&mut self, text: &str, x: f64, y: f64) -> Result<&mut Self> {
        use crate::fonts::needs_type0_font;

        self.operations.push(ops::Op::BeginText);
        self.apply_fill_color();
        self.push_active_font();
        self.operations.push(ops::Op::SetTextPosition { x, y });

        let mut hex = String::new();
        if needs_type0_font(text) {
            for ch in text.chars() {
                encode_char_as_cid(ch, &mut hex);
            }
        } else {
            for ch in text.chars() {
                use std::fmt::Write as _;
                if ch as u32 <= 255 {
                    write!(&mut hex, "{:02X}", ch as u8).expect("write to String never fails");
                } else {
                    hex.push_str("3F");
                }
            }
        }
        self.operations.push(ops::Op::ShowTextHex(hex.into_bytes()));
        self.operations.push(ops::Op::EndText);

        Ok(self)
    }

    /// Legacy: Draw text with UTF-16BE encoding (kept for compatibility)
    #[deprecated(note = "Use draw_text() which automatically detects encoding")]
    pub fn draw_text_unicode(&mut self, text: &str, x: f64, y: f64) -> Result<&mut Self> {
        self.operations.push(ops::Op::BeginText);
        self.apply_fill_color();
        self.push_active_font();
        self.operations.push(ops::Op::SetTextPosition { x, y });

        let mut hex = String::new();
        let mut utf16_buffer = [0u16; 2];
        for ch in text.chars() {
            let encoded = ch.encode_utf16(&mut utf16_buffer);
            for unit in encoded {
                use std::fmt::Write as _;
                write!(&mut hex, "{:04X}", unit).expect("write to String never fails");
            }
        }
        self.operations.push(ops::Op::ShowTextHex(hex.into_bytes()));
        self.operations.push(ops::Op::EndText);

        Ok(self)
    }

    /// Record `text` as drawn with the currently-active font.
    ///
    /// Chars are bucketed under the font name (builtin or custom) so
    /// that the writer can subset each custom font with only its own
    /// characters (issue #204). When no font has been set yet the
    /// chars are bucketed under an empty-string sentinel — the writer
    /// iterates `custom_font_names()` when subsetting and that list
    /// never contains an empty name, so the sentinel is ignored by
    /// the writer but keeps the merged [`Self::get_used_characters`]
    /// accessor lossless for diagnostic callers.
    fn record_used_chars(&mut self, text: &str) {
        let bucket = self.current_font_name.as_deref().unwrap_or("").to_string();
        self.used_characters_by_font
            .entry(bucket)
            .or_default()
            .extend(text.chars());
    }

    /// Get the characters used in this graphics context, merged across
    /// fonts. Test-only back-compat accessor; production callers go
    /// through [`GraphicsContext::get_used_characters_by_font`] so the
    /// writer can subset each custom font with only its own characters
    /// (issue #204).
    #[cfg(test)]
    pub(crate) fn get_used_characters(&self) -> Option<HashSet<char>> {
        let merged: HashSet<char> = self
            .used_characters_by_font
            .values()
            .flat_map(|s| s.iter().copied())
            .collect();
        if merged.is_empty() {
            None
        } else {
            Some(merged)
        }
    }

    /// Get the per-font character map for font subsetting (issue #204).
    ///
    /// Keys are the registered custom-font names exactly as passed to
    /// `Document::add_font_from_bytes`. Builtin fonts never appear as
    /// keys because they don't need subsetting. A font name missing
    /// from the map means no content stream in this context drew any
    /// character with that font.
    pub(crate) fn get_used_characters_by_font(&self) -> &HashMap<String, HashSet<char>> {
        &self.used_characters_by_font
    }

    /// Merge a per-font char map produced by an external content-stream
    /// builder (e.g. [`crate::layout::RichText::render_operations`])
    /// into this graphics context's accumulator. Issue #204 — callers
    /// of [`crate::Page::append_raw_content`] MUST report what they
    /// drew so the writer can subset each custom font correctly.
    pub(crate) fn merge_font_usage(&mut self, usage: &HashMap<String, HashSet<char>>) {
        for (name, chars) in usage {
            self.used_characters_by_font
                .entry(name.clone())
                .or_default()
                .extend(chars);
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
        // v2.7.0 IR: every component of the `cm` matrix is emitted as `{:.2}`,
        // including identity-matrix slots (`1 0 0 1` → `1.00 0.00 0.00 1.00`).
        // See CHANGELOG → "cm matrix format" under 2.7.0.
        assert!(ctx
            .operations()
            .contains("1.00 0.00 0.00 1.00 50.00 100.00 cm\n"));
    }

    #[test]
    fn test_scale() {
        let mut ctx = GraphicsContext::new();
        ctx.scale(2.0, 3.0);
        // v2.7.0 IR: see test_translate.
        assert!(ctx
            .operations()
            .contains("2.00 0.00 0.00 3.00 0.00 0.00 cm\n"));
    }

    #[test]
    fn test_rotate() {
        let mut ctx = GraphicsContext::new();
        let angle = std::f64::consts::PI / 4.0; // 45 degrees
        ctx.rotate(angle);

        let ops = ctx.operations();
        assert!(ops.contains(" cm\n"));
        // v2.7.0 IR: rotation matrix emitted at `{:.2}` (was `{:.6}`).
        // 0.707107 truncates to "0.71". See CHANGELOG.
        assert!(ops.contains("0.71"));
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
                                      // v2.7.0 IR: cm matrix slots emitted at `{:.2}` consistently. See CHANGELOG.
        assert!(ops.contains("100.00 0.00 0.00 150.00 10.00 20.00 cm\n"));
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
        let bytes = result.expect("Writing to string should never fail");
        let ops_string = String::from_utf8(bytes).expect("Writing to string should never fail");
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
        let dict = ctx
            .generate_graphics_state_dict()
            .expect("Writing to string should never fail");
        assert!(dict.contains("/Type /ExtGState"));
        assert!(dict.contains("/ca 0.500"));
        assert!(!dict.contains("/CA"));

        // Stroke opacity only
        ctx.set_fill_opacity(1.0);
        ctx.set_stroke_opacity(0.75);
        let dict = ctx
            .generate_graphics_state_dict()
            .expect("Writing to string should never fail");
        assert!(dict.contains("/Type /ExtGState"));
        assert!(dict.contains("/CA 0.750"));
        assert!(!dict.contains("/ca"));

        // Both opacities
        ctx.set_fill_opacity(0.25);
        let dict = ctx
            .generate_graphics_state_dict()
            .expect("Writing to string should never fail");
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
        ctx.show_text("Hello World")
            .expect("Writing to string should never fail");
        assert!(ctx.operations().contains("(Hello World) Tj\n"));
    }

    #[test]
    fn test_show_text_with_escaping() {
        let mut ctx = GraphicsContext::new();
        ctx.show_text("Test (parentheses)")
            .expect("Writing to string should never fail");
        assert!(ctx.operations().contains("(Test \\(parentheses\\)) Tj\n"));

        ctx.clear();
        ctx.show_text("Back\\slash")
            .expect("Writing to string should never fail");
        assert!(ctx.operations().contains("(Back\\\\slash) Tj\n"));

        ctx.clear();
        ctx.show_text("Line\nBreak")
            .expect("Writing to string should never fail");
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

        let ops = ctx
            .generate_operations()
            .expect("Writing to string should never fail");
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

        let ops = ctx
            .generate_operations()
            .expect("Writing to string should never fail");
        let ops_str = String::from_utf8_lossy(&ops);
        assert!(ops_str.contains("0.00 0.00 m"));
        assert!(ops_str.contains("10.00 20.00 30.00 40.00 50.00 60.00 c"));
        assert!(ops_str.contains("S"));
    }

    #[test]
    fn test_circle_path() {
        let mut ctx = GraphicsContext::new();
        ctx.circle(100.0, 100.0, 50.0).fill();

        let ops = ctx
            .generate_operations()
            .expect("Writing to string should never fail");
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

        let ops = ctx
            .generate_operations()
            .expect("Writing to string should never fail");
        let ops_str = String::from_utf8_lossy(&ops);
        assert!(ops_str.contains("h")); // close path operator
        assert!(ops_str.contains("S"));
    }

    #[test]
    fn test_fill_and_stroke() {
        let mut ctx = GraphicsContext::new();
        ctx.rect(10.0, 10.0, 50.0, 50.0).fill_stroke();

        let ops = ctx
            .generate_operations()
            .expect("Writing to string should never fail");
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

        let ops = ctx
            .generate_operations()
            .expect("Writing to string should never fail");
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

        let ops = ctx
            .generate_operations()
            .expect("Writing to string should never fail");
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

        let ops = ctx
            .generate_operations()
            .expect("Writing to string should never fail");
        let ops_str = String::from_utf8_lossy(&ops);
        assert!(ops_str.contains("q")); // save state
        assert!(ops_str.contains("Q")); // restore state
    }

    #[test]
    fn test_transformations() {
        let mut ctx = GraphicsContext::new();
        ctx.translate(100.0, 200.0).scale(2.0, 3.0).rotate(45.0);

        let ops = ctx
            .generate_operations()
            .expect("Writing to string should never fail");
        let ops_str = String::from_utf8_lossy(&ops);
        // v2.7.0 IR: cm matrix slots emitted at `{:.2}` consistently
        // (identity slots are no longer integer literals). See CHANGELOG.
        assert!(ops_str.contains("1.00 0.00 0.00 1.00 100.00 200.00 cm")); // translate
        assert!(ops_str.contains("2.00 0.00 0.00 3.00 0.00 0.00 cm")); // scale
        assert!(ops_str.contains("cm")); // rotate matrix
    }

    #[test]
    fn test_custom_transform() {
        let mut ctx = GraphicsContext::new();
        ctx.transform(1.0, 0.5, 0.5, 1.0, 10.0, 20.0);

        let ops = ctx
            .generate_operations()
            .expect("Writing to string should never fail");
        let ops_str = String::from_utf8_lossy(&ops);
        assert!(ops_str.contains("1.00 0.50 0.50 1.00 10.00 20.00 cm"));
    }

    #[test]
    fn test_rectangle_path() {
        let mut ctx = GraphicsContext::new();
        ctx.rectangle(25.0, 25.0, 150.0, 100.0).stroke();

        let ops = ctx
            .generate_operations()
            .expect("Writing to string should never fail");
        let ops_str = String::from_utf8_lossy(&ops);
        assert!(ops_str.contains("25.00 25.00 150.00 100.00 re"));
        assert!(ops_str.contains("S"));
    }

    #[test]
    fn test_empty_operations() {
        let ctx = GraphicsContext::new();
        let ops = ctx
            .generate_operations()
            .expect("Writing to string should never fail");
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

        let ops = ctx
            .generate_operations()
            .expect("Writing to string should never fail");
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
        let dict_str = dict.expect("Writing to string should never fail");
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

        let ops = ctx
            .generate_operations()
            .expect("Writing to string should never fail");
        let ops_str = String::from_utf8_lossy(&ops);
        assert!(ops_str.contains("[3.00 2.00] 0.00 d"));
    }

    #[test]
    fn test_miter_limit_setting() {
        let mut ctx = GraphicsContext::new();
        ctx.set_miter_limit(4.0);

        let ops = ctx
            .generate_operations()
            .expect("Writing to string should never fail");
        let ops_str = String::from_utf8_lossy(&ops);
        assert!(ops_str.contains("4.00 M"));
    }

    #[test]
    fn test_line_cap_styles() {
        let mut ctx = GraphicsContext::new();

        ctx.set_line_cap(LineCap::Butt);
        let ops = ctx
            .generate_operations()
            .expect("Writing to string should never fail");
        let ops_str = String::from_utf8_lossy(&ops);
        assert!(ops_str.contains("0 J"));

        let mut ctx = GraphicsContext::new();
        ctx.set_line_cap(LineCap::Round);
        let ops = ctx
            .generate_operations()
            .expect("Writing to string should never fail");
        let ops_str = String::from_utf8_lossy(&ops);
        assert!(ops_str.contains("1 J"));

        let mut ctx = GraphicsContext::new();
        ctx.set_line_cap(LineCap::Square);
        let ops = ctx
            .generate_operations()
            .expect("Writing to string should never fail");
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
        let ops = ctx
            .generate_operations()
            .expect("Writing to string should never fail");
        let ops_str = String::from_utf8_lossy(&ops);
        assert!(ops_str.contains("0 j"));

        let mut ctx = GraphicsContext::new();
        ctx.set_line_join(LineJoin::Round);
        let ops = ctx
            .generate_operations()
            .expect("Writing to string should never fail");
        let ops_str = String::from_utf8_lossy(&ops);
        assert!(ops_str.contains("1 j"));

        let mut ctx = GraphicsContext::new();
        ctx.set_line_join(LineJoin::Bevel);
        let ops = ctx
            .generate_operations()
            .expect("Writing to string should never fail");
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

        let ops = ctx
            .generate_operations()
            .expect("Writing to string should never fail");
        let ops_str = String::from_utf8_lossy(&ops);
        assert!(ops_str.contains("0.50 i"));
    }

    #[test]
    fn test_smoothness_tolerance() {
        let mut ctx = GraphicsContext::new();

        let _ = ctx.set_smoothness(0.1);
        assert_eq!(ctx.smoothness(), 0.1);
    }

    #[test]
    fn test_bezier_curves() {
        let mut ctx = GraphicsContext::new();

        // Cubic Bezier
        ctx.move_to(10.0, 10.0);
        ctx.curve_to(20.0, 10.0, 30.0, 20.0, 30.0, 30.0);

        let ops = ctx
            .generate_operations()
            .expect("Writing to string should never fail");
        let ops_str = String::from_utf8_lossy(&ops);
        assert!(ops_str.contains("10.00 10.00 m"));
        assert!(ops_str.contains("c")); // cubic curve
    }

    #[test]
    fn test_clipping_path() {
        let mut ctx = GraphicsContext::new();

        ctx.rectangle(10.0, 10.0, 100.0, 100.0);
        ctx.clip();

        let ops = ctx
            .generate_operations()
            .expect("Writing to string should never fail");
        let ops_str = String::from_utf8_lossy(&ops);
        assert!(ops_str.contains("W"));
    }

    #[test]
    fn test_even_odd_clipping() {
        let mut ctx = GraphicsContext::new();

        ctx.rectangle(10.0, 10.0, 100.0, 100.0);
        ctx.clip_even_odd();

        let ops = ctx
            .generate_operations()
            .expect("Writing to string should never fail");
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
        let ctx = GraphicsContext::new();

        // Test that we can create and use an extended graphics state
        let _extgstate = ExtGState::new();

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

        let ops = ctx
            .generate_operations()
            .expect("Writing to string should never fail");
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

        let ops = ctx
            .generate_operations()
            .expect("Writing to string should never fail");
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

    #[test]
    fn test_word_spacing() {
        let mut ctx = GraphicsContext::new();
        ctx.set_word_spacing(2.5);

        let ops = ctx.generate_operations().unwrap();
        let ops_str = String::from_utf8_lossy(&ops);
        assert!(ops_str.contains("2.50 Tw"));
    }

    #[test]
    fn test_character_spacing() {
        let mut ctx = GraphicsContext::new();
        ctx.set_character_spacing(1.0);

        let ops = ctx.generate_operations().unwrap();
        let ops_str = String::from_utf8_lossy(&ops);
        assert!(ops_str.contains("1.00 Tc"));
    }

    #[test]
    fn test_justified_text() {
        let mut ctx = GraphicsContext::new();
        ctx.begin_text();
        ctx.set_text_position(100.0, 200.0);
        ctx.show_justified_text("Hello world from PDF", 200.0)
            .unwrap();
        ctx.end_text();

        let ops = ctx.generate_operations().unwrap();
        let ops_str = String::from_utf8_lossy(&ops);

        // Should contain text operations
        assert!(ops_str.contains("BT")); // Begin text
        assert!(ops_str.contains("ET")); // End text
        assert!(ops_str.contains("100.00 200.00 Td")); // Text position
        assert!(ops_str.contains("(Hello world from PDF) Tj")); // Show text

        // Should contain word spacing operations
        assert!(ops_str.contains("Tw")); // Word spacing
    }

    #[test]
    fn test_justified_text_single_word() {
        let mut ctx = GraphicsContext::new();
        ctx.begin_text();
        ctx.show_justified_text("Hello", 200.0).unwrap();
        ctx.end_text();

        let ops = ctx.generate_operations().unwrap();
        let ops_str = String::from_utf8_lossy(&ops);

        // Single word should just use normal text display
        assert!(ops_str.contains("(Hello) Tj"));
        // Should not contain word spacing since there's only one word
        assert_eq!(ops_str.matches("Tw").count(), 0);
    }

    #[test]
    fn test_text_width_estimation() {
        let ctx = GraphicsContext::new();
        let width = ctx.estimate_text_width_simple("Hello");

        // Should return reasonable estimation based on font size and character count
        assert!(width > 0.0);
        assert_eq!(width, 5.0 * 12.0 * 0.6); // 5 chars * 12pt font * 0.6 factor
    }

    #[test]
    fn test_set_alpha_methods() {
        let mut ctx = GraphicsContext::new();

        // Test that set_alpha methods don't panic and return correctly
        assert!(ctx.set_alpha(0.5).is_ok());
        assert!(ctx.set_alpha_fill(0.3).is_ok());
        assert!(ctx.set_alpha_stroke(0.7).is_ok());

        // Test edge cases - should handle clamping in ExtGState
        assert!(ctx.set_alpha(1.5).is_ok()); // Should not panic
        assert!(ctx.set_alpha(-0.2).is_ok()); // Should not panic
        assert!(ctx.set_alpha_fill(2.0).is_ok()); // Should not panic
        assert!(ctx.set_alpha_stroke(-1.0).is_ok()); // Should not panic

        // Test that methods return self for chaining
        let result = ctx
            .set_alpha(0.5)
            .and_then(|c| c.set_alpha_fill(0.3))
            .and_then(|c| c.set_alpha_stroke(0.7));
        assert!(result.is_ok());
    }

    #[test]
    fn test_alpha_methods_generate_extgstate() {
        let mut ctx = GraphicsContext::new();

        // Set some transparency
        ctx.set_alpha(0.5).unwrap();

        // Draw something to trigger ExtGState generation
        ctx.rect(10.0, 10.0, 50.0, 50.0).fill();

        let ops = ctx.generate_operations().unwrap();
        let ops_str = String::from_utf8_lossy(&ops);

        // Should contain ExtGState reference
        assert!(ops_str.contains("/GS")); // ExtGState name
        assert!(ops_str.contains(" gs\n")); // ExtGState operator

        // Test separate alpha settings
        ctx.clear();
        ctx.set_alpha_fill(0.3).unwrap();
        ctx.set_alpha_stroke(0.8).unwrap();
        ctx.rect(20.0, 20.0, 60.0, 60.0).fill_stroke();

        let ops2 = ctx.generate_operations().unwrap();
        let ops_str2 = String::from_utf8_lossy(&ops2);

        // Should contain multiple ExtGState references
        assert!(ops_str2.contains("/GS")); // ExtGState names
        assert!(ops_str2.contains(" gs\n")); // ExtGState operators
    }

    #[test]
    fn test_add_command() {
        let mut ctx = GraphicsContext::new();

        // Test normal command
        ctx.add_command("1 0 0 1 100 200 cm");
        let ops = ctx.operations();
        assert!(ops.contains("1 0 0 1 100 200 cm\n"));

        // Test that newline is always added
        ctx.clear();
        ctx.add_command("q");
        assert_eq!(ctx.operations(), "q\n");

        // Test empty string
        ctx.clear();
        ctx.add_command("");
        assert_eq!(ctx.operations(), "\n");

        // Test command with existing newline
        ctx.clear();
        ctx.add_command("Q\n");
        assert_eq!(ctx.operations(), "Q\n\n"); // Double newline

        // Test multiple commands
        ctx.clear();
        ctx.add_command("q");
        ctx.add_command("1 0 0 1 50 50 cm");
        ctx.add_command("Q");
        assert_eq!(ctx.operations(), "q\n1 0 0 1 50 50 cm\nQ\n");
    }

    #[test]
    fn test_get_operations() {
        let mut ctx = GraphicsContext::new();
        ctx.rect(10.0, 10.0, 50.0, 50.0);
        let ops1 = ctx.operations();
        let ops2 = ctx.get_operations();
        assert_eq!(ops1, ops2);
    }

    #[test]
    fn test_set_line_solid() {
        let mut ctx = GraphicsContext::new();
        ctx.set_line_dash_pattern(LineDashPattern::new(vec![5.0, 3.0], 0.0));
        ctx.set_line_solid();
        let ops = ctx.operations();
        assert!(ops.contains("[] 0 d\n"));
    }

    #[test]
    fn test_set_custom_font() {
        let mut ctx = GraphicsContext::new();
        ctx.set_custom_font("CustomFont", 14.0);
        assert_eq!(ctx.current_font_name.as_deref(), Some("CustomFont"));
        assert_eq!(ctx.current_font_size, 14.0);
        assert!(ctx.is_custom_font);
    }

    #[test]
    fn test_show_text_standard_font_uses_literal_string() {
        let mut ctx = GraphicsContext::new();
        ctx.set_font(Font::Helvetica, 12.0);
        assert!(!ctx.is_custom_font);

        ctx.begin_text();
        ctx.set_text_position(10.0, 20.0);
        ctx.show_text("Hello World").unwrap();
        ctx.end_text();

        let ops = ctx.operations();
        assert!(ops.contains("(Hello World) Tj"));
        assert!(!ops.contains("<"));
    }

    #[test]
    fn test_show_text_custom_font_uses_hex_encoding() {
        let mut ctx = GraphicsContext::new();
        ctx.set_font(Font::Custom("NotoSansCJK".to_string()), 12.0);
        assert!(ctx.is_custom_font);

        ctx.begin_text();
        ctx.set_text_position(10.0, 20.0);
        // CJK characters: 你好 (U+4F60 U+597D)
        ctx.show_text("你好").unwrap();
        ctx.end_text();

        let ops = ctx.operations();
        // Must be hex-encoded, not literal
        assert!(
            ops.contains("<4F60597D> Tj"),
            "Expected hex encoding for CJK text, got: {}",
            ops
        );
        assert!(!ops.contains("(你好)"));
    }

    #[test]
    fn test_show_text_custom_font_ascii_still_hex() {
        let mut ctx = GraphicsContext::new();
        ctx.set_font(Font::Custom("MyFont".to_string()), 10.0);

        ctx.begin_text();
        ctx.set_text_position(0.0, 0.0);
        // Even ASCII text should be hex-encoded when using custom font
        ctx.show_text("AB").unwrap();
        ctx.end_text();

        let ops = ctx.operations();
        // A=0x0041, B=0x0042
        assert!(
            ops.contains("<00410042> Tj"),
            "Expected hex encoding for ASCII in custom font, got: {}",
            ops
        );
    }

    #[test]
    fn test_show_text_tracks_used_characters() {
        let mut ctx = GraphicsContext::new();
        ctx.set_font(Font::Custom("CJKFont".to_string()), 12.0);

        ctx.begin_text();
        ctx.show_text("你好A").unwrap();
        ctx.end_text();

        let chars = ctx
            .get_used_characters()
            .expect("show_text with a custom font must record characters");
        assert!(chars.contains(&'你'));
        assert!(chars.contains(&'好'));
        assert!(chars.contains(&'A'));
    }

    #[test]
    fn test_is_custom_font_toggles_correctly() {
        let mut ctx = GraphicsContext::new();
        assert!(!ctx.is_custom_font);

        ctx.set_font(Font::Custom("CJK".to_string()), 12.0);
        assert!(ctx.is_custom_font);

        ctx.set_font(Font::Helvetica, 12.0);
        assert!(!ctx.is_custom_font);

        ctx.set_custom_font("AnotherCJK", 14.0);
        assert!(ctx.is_custom_font);

        ctx.set_font(Font::CourierBold, 10.0);
        assert!(!ctx.is_custom_font);
    }

    #[test]
    fn test_set_glyph_mapping() {
        let mut ctx = GraphicsContext::new();

        // Test initial state
        assert!(ctx.glyph_mapping.is_none());

        // Test normal mapping
        let mut mapping = HashMap::new();
        mapping.insert(65u32, 1u16); // 'A' -> glyph 1
        mapping.insert(66u32, 2u16); // 'B' -> glyph 2
        ctx.set_glyph_mapping(mapping.clone());
        assert!(ctx.glyph_mapping.is_some());
        assert_eq!(ctx.glyph_mapping.as_ref().unwrap().len(), 2);
        assert_eq!(ctx.glyph_mapping.as_ref().unwrap().get(&65), Some(&1));
        assert_eq!(ctx.glyph_mapping.as_ref().unwrap().get(&66), Some(&2));

        // Test empty mapping
        ctx.set_glyph_mapping(HashMap::new());
        assert!(ctx.glyph_mapping.is_some());
        assert_eq!(ctx.glyph_mapping.as_ref().unwrap().len(), 0);

        // Test overwrite existing mapping
        let mut new_mapping = HashMap::new();
        new_mapping.insert(67u32, 3u16); // 'C' -> glyph 3
        ctx.set_glyph_mapping(new_mapping);
        assert_eq!(ctx.glyph_mapping.as_ref().unwrap().len(), 1);
        assert_eq!(ctx.glyph_mapping.as_ref().unwrap().get(&67), Some(&3));
        assert_eq!(ctx.glyph_mapping.as_ref().unwrap().get(&65), None); // Old mapping gone
    }

    #[test]
    fn test_draw_text_basic() {
        let mut ctx = GraphicsContext::new();
        ctx.set_font(Font::Helvetica, 12.0);

        let result = ctx.draw_text("Hello", 100.0, 200.0);
        assert!(result.is_ok());

        let ops = ctx.operations();
        // Verify text block
        assert!(ops.contains("BT\n"));
        assert!(ops.contains("ET\n"));

        // Verify font is set
        assert!(ops.contains("/Helvetica"));
        assert!(ops.contains("12"));
        assert!(ops.contains("Tf\n"));

        // Verify positioning
        assert!(ops.contains("100"));
        assert!(ops.contains("200"));
        assert!(ops.contains("Td\n"));

        // Verify text content
        assert!(ops.contains("(Hello)") || ops.contains("<48656c6c6f>")); // Text or hex
    }

    #[test]
    fn test_draw_text_with_special_characters() {
        let mut ctx = GraphicsContext::new();
        ctx.set_font(Font::Helvetica, 12.0);

        // Test with parentheses (must be escaped in PDF)
        let result = ctx.draw_text("Test (with) parens", 50.0, 100.0);
        assert!(result.is_ok());

        let ops = ctx.operations();
        // Should escape parentheses
        assert!(ops.contains("\\(") || ops.contains("\\)") || ops.contains("<"));
        // Either escaped or hex
    }

    #[test]
    fn test_draw_text_unicode_detection() {
        let mut ctx = GraphicsContext::new();
        ctx.set_font(Font::Helvetica, 12.0);

        // ASCII text should use simple encoding
        ctx.draw_text("ASCII", 0.0, 0.0).unwrap();
        let _ops_ascii = ctx.operations();

        ctx.clear();

        // Unicode text should trigger different encoding
        ctx.set_font(Font::Helvetica, 12.0);
        ctx.draw_text("中文", 0.0, 0.0).unwrap();
        let ops_unicode = ctx.operations();

        // Unicode should produce hex encoding
        assert!(ops_unicode.contains("<") && ops_unicode.contains(">"));
    }

    #[test]
    #[allow(deprecated)]
    fn test_draw_text_hex_encoding() {
        let mut ctx = GraphicsContext::new();
        ctx.set_font(Font::Helvetica, 12.0);
        let result = ctx.draw_text_hex("Test", 50.0, 100.0);
        assert!(result.is_ok());
        let ops = ctx.operations();
        assert!(ops.contains("<"));
        assert!(ops.contains(">"));
    }

    #[test]
    #[allow(deprecated)]
    fn test_draw_text_cid() {
        let mut ctx = GraphicsContext::new();
        ctx.set_custom_font("CustomCIDFont", 12.0);
        let result = ctx.draw_text_cid("Test", 50.0, 100.0);
        assert!(result.is_ok());
        let ops = ctx.operations();
        assert!(ops.contains("BT\n"));
        assert!(ops.contains("ET\n"));
    }

    #[test]
    #[allow(deprecated)]
    fn test_draw_text_unicode() {
        let mut ctx = GraphicsContext::new();
        ctx.set_custom_font("UnicodeFont", 12.0);
        let result = ctx.draw_text_unicode("Test \u{4E2D}\u{6587}", 50.0, 100.0);
        assert!(result.is_ok());
        let ops = ctx.operations();
        assert!(ops.contains("BT\n"));
        assert!(ops.contains("ET\n"));
    }

    #[test]
    fn test_begin_end_transparency_group() {
        let mut ctx = GraphicsContext::new();

        // Initial state - no transparency group
        assert!(!ctx.in_transparency_group());
        assert!(ctx.current_transparency_group().is_none());

        // Begin transparency group
        let group = TransparencyGroup::new();
        ctx.begin_transparency_group(group);
        assert!(ctx.in_transparency_group());
        assert!(ctx.current_transparency_group().is_some());

        // Verify operations contain transparency marker
        let ops = ctx.operations();
        assert!(ops.contains("% Begin Transparency Group"));

        // End transparency group
        ctx.end_transparency_group();
        assert!(!ctx.in_transparency_group());
        assert!(ctx.current_transparency_group().is_none());

        // Verify end marker
        let ops_after = ctx.operations();
        assert!(ops_after.contains("% End Transparency Group"));
    }

    #[test]
    fn test_transparency_group_nesting() {
        let mut ctx = GraphicsContext::new();

        // Nest 3 levels
        let group1 = TransparencyGroup::new();
        let group2 = TransparencyGroup::new();
        let group3 = TransparencyGroup::new();

        ctx.begin_transparency_group(group1);
        assert_eq!(ctx.transparency_stack.len(), 1);

        ctx.begin_transparency_group(group2);
        assert_eq!(ctx.transparency_stack.len(), 2);

        ctx.begin_transparency_group(group3);
        assert_eq!(ctx.transparency_stack.len(), 3);

        // End all
        ctx.end_transparency_group();
        assert_eq!(ctx.transparency_stack.len(), 2);

        ctx.end_transparency_group();
        assert_eq!(ctx.transparency_stack.len(), 1);

        ctx.end_transparency_group();
        assert_eq!(ctx.transparency_stack.len(), 0);
        assert!(!ctx.in_transparency_group());
    }

    #[test]
    fn test_transparency_group_without_begin() {
        let mut ctx = GraphicsContext::new();

        // Try to end without begin - should not panic, just be no-op
        assert!(!ctx.in_transparency_group());
        ctx.end_transparency_group();
        assert!(!ctx.in_transparency_group());
    }

    #[test]
    fn test_extgstate_manager_access() {
        let ctx = GraphicsContext::new();
        let manager = ctx.extgstate_manager();
        assert_eq!(manager.count(), 0);
    }

    #[test]
    fn test_extgstate_manager_mut_access() {
        let mut ctx = GraphicsContext::new();
        let manager = ctx.extgstate_manager_mut();
        assert_eq!(manager.count(), 0);
    }

    #[test]
    fn test_has_extgstates() {
        let mut ctx = GraphicsContext::new();

        // Initially no extgstates
        assert!(!ctx.has_extgstates());
        assert_eq!(ctx.extgstate_manager().count(), 0);

        // Adding transparency creates extgstate
        ctx.set_alpha(0.5).unwrap();
        ctx.rect(10.0, 10.0, 50.0, 50.0).fill();
        let result = ctx.generate_operations().unwrap();

        assert!(ctx.has_extgstates());
        assert!(ctx.extgstate_manager().count() > 0);

        // Verify extgstate is in PDF output
        let output = String::from_utf8_lossy(&result);
        assert!(output.contains("/GS")); // ExtGState reference
        assert!(output.contains(" gs\n")); // ExtGState operator
    }

    #[test]
    fn test_generate_extgstate_resources() {
        let mut ctx = GraphicsContext::new();
        ctx.set_alpha(0.5).unwrap();
        ctx.rect(10.0, 10.0, 50.0, 50.0).fill();
        ctx.generate_operations().unwrap();

        let resources = ctx.generate_extgstate_resources();
        assert!(resources.is_ok());
    }

    #[test]
    fn test_apply_extgstate() {
        let mut ctx = GraphicsContext::new();

        // Create ExtGState with specific values
        let mut state = ExtGState::new();
        state.alpha_fill = Some(0.5);
        state.alpha_stroke = Some(0.8);
        state.blend_mode = Some(BlendMode::Multiply);

        let result = ctx.apply_extgstate(state);
        assert!(result.is_ok());

        // Verify ExtGState was registered
        assert!(ctx.has_extgstates());
        assert_eq!(ctx.extgstate_manager().count(), 1);

        // Apply different ExtGState
        let mut state2 = ExtGState::new();
        state2.alpha_fill = Some(0.3);
        ctx.apply_extgstate(state2).unwrap();

        // Should have 2 different extgstates
        assert_eq!(ctx.extgstate_manager().count(), 2);
    }

    #[test]
    fn test_with_extgstate() {
        let mut ctx = GraphicsContext::new();
        let result = ctx.with_extgstate(|mut state| {
            state.alpha_fill = Some(0.5);
            state.alpha_stroke = Some(0.8);
            state
        });
        assert!(result.is_ok());
    }

    #[test]
    fn test_set_blend_mode() {
        let mut ctx = GraphicsContext::new();

        // Test different blend modes
        let result = ctx.set_blend_mode(BlendMode::Multiply);
        assert!(result.is_ok());
        assert!(ctx.has_extgstates());

        // Test that different blend modes create different extgstates
        ctx.clear();
        ctx.set_blend_mode(BlendMode::Screen).unwrap();
        ctx.rect(0.0, 0.0, 10.0, 10.0).fill();
        let ops = ctx.generate_operations().unwrap();
        let output = String::from_utf8_lossy(&ops);

        // Should contain extgstate reference
        assert!(output.contains("/GS"));
        assert!(output.contains(" gs\n"));
    }

    #[test]
    fn test_render_table() {
        let mut ctx = GraphicsContext::new();
        let table = Table::with_equal_columns(2, 200.0);
        let result = ctx.render_table(&table);
        assert!(result.is_ok());
    }

    #[test]
    fn test_render_list() {
        let mut ctx = GraphicsContext::new();
        use crate::text::{OrderedList, OrderedListStyle};
        let ordered = OrderedList::new(OrderedListStyle::Decimal);
        let list = ListElement::Ordered(ordered);
        let result = ctx.render_list(&list);
        assert!(result.is_ok());
    }

    #[test]
    fn test_render_column_layout() {
        let mut ctx = GraphicsContext::new();
        use crate::text::ColumnContent;
        let layout = ColumnLayout::new(2, 100.0, 200.0);
        let content = ColumnContent::new("Test content");
        let result = ctx.render_column_layout(&layout, &content, 50.0, 50.0, 400.0);
        assert!(result.is_ok());
    }

    #[test]
    fn test_clip_ellipse() {
        let mut ctx = GraphicsContext::new();

        // No clipping initially
        assert!(!ctx.has_clipping());
        assert!(ctx.clipping_path().is_none());

        // Apply ellipse clipping
        let result = ctx.clip_ellipse(100.0, 100.0, 50.0, 30.0);
        assert!(result.is_ok());
        assert!(ctx.has_clipping());
        assert!(ctx.clipping_path().is_some());

        // Verify clipping operations in PDF
        let ops = ctx.operations();
        assert!(ops.contains("W\n") || ops.contains("W*\n")); // Clipping operator

        // Clear clipping
        ctx.clear_clipping();
        assert!(!ctx.has_clipping());
    }

    #[test]
    fn test_clipping_path_access() {
        let mut ctx = GraphicsContext::new();

        // No clipping initially
        assert!(ctx.clipping_path().is_none());

        // Apply rect clipping
        ctx.clip_rect(10.0, 10.0, 50.0, 50.0).unwrap();
        assert!(ctx.clipping_path().is_some());

        // Apply different clipping - should replace
        ctx.clip_circle(100.0, 100.0, 25.0).unwrap();
        assert!(ctx.clipping_path().is_some());

        // Save/restore should preserve clipping
        ctx.save_state();
        ctx.clear_clipping();
        assert!(!ctx.has_clipping());

        ctx.restore_state();
        // After restore, clipping should be back
        assert!(ctx.has_clipping());
    }

    // ====== QUALITY TESTS: EDGE CASES ======

    #[test]
    fn test_edge_case_move_to_negative() {
        let mut ctx = GraphicsContext::new();
        ctx.move_to(-100.5, -200.25);
        assert!(ctx.operations().contains("-100.50 -200.25 m\n"));
    }

    #[test]
    fn test_edge_case_opacity_out_of_range() {
        let mut ctx = GraphicsContext::new();

        // Above 1.0 - should clamp
        let _ = ctx.set_opacity(2.5);
        assert_eq!(ctx.fill_opacity(), 1.0);

        // Below 0.0 - should clamp
        let _ = ctx.set_opacity(-0.5);
        assert_eq!(ctx.fill_opacity(), 0.0);
    }

    #[test]
    fn test_edge_case_line_width_extremes() {
        let mut ctx = GraphicsContext::new();

        ctx.set_line_width(0.0);
        assert_eq!(ctx.line_width(), 0.0);

        ctx.set_line_width(10000.0);
        assert_eq!(ctx.line_width(), 10000.0);
    }

    // ====== QUALITY TESTS: FEATURE INTERACTIONS ======

    #[test]
    fn test_interaction_transparency_plus_clipping() {
        let mut ctx = GraphicsContext::new();

        ctx.set_alpha(0.5).unwrap();
        ctx.clip_rect(10.0, 10.0, 100.0, 100.0).unwrap();
        ctx.rect(20.0, 20.0, 80.0, 80.0).fill();

        let ops = ctx.generate_operations().unwrap();
        let output = String::from_utf8_lossy(&ops);

        // Both features should be in PDF
        assert!(output.contains("W\n") || output.contains("W*\n"));
        assert!(output.contains("/GS"));
    }

    #[test]
    fn test_interaction_extgstate_plus_text() {
        let mut ctx = GraphicsContext::new();

        let mut state = ExtGState::new();
        state.alpha_fill = Some(0.7);
        ctx.apply_extgstate(state).unwrap();

        ctx.set_font(Font::Helvetica, 14.0);
        ctx.draw_text("Test", 100.0, 200.0).unwrap();

        let ops = ctx.generate_operations().unwrap();
        let output = String::from_utf8_lossy(&ops);

        assert!(output.contains("/GS"));
        assert!(output.contains("BT\n"));
    }

    #[test]
    fn test_interaction_chained_transformations() {
        let mut ctx = GraphicsContext::new();

        ctx.translate(50.0, 100.0);
        ctx.rotate(45.0);
        ctx.scale(2.0, 2.0);

        let ops = ctx.operations();
        assert_eq!(ops.matches("cm\n").count(), 3);
    }

    // ====== QUALITY TESTS: END-TO-END ======

    #[test]
    fn test_e2e_complete_page_with_header() {
        use crate::{Document, Page};

        let mut doc = Document::new();
        let mut page = Page::a4();
        let ctx = page.graphics();

        // Header
        ctx.save_state();
        let _ = ctx.set_fill_opacity(0.3);
        ctx.set_fill_color(Color::rgb(200.0, 200.0, 255.0));
        ctx.rect(0.0, 750.0, 595.0, 42.0).fill();
        ctx.restore_state();

        // Content
        ctx.save_state();
        ctx.clip_rect(50.0, 50.0, 495.0, 692.0).unwrap();
        ctx.rect(60.0, 60.0, 100.0, 100.0).fill();
        ctx.restore_state();

        let ops = ctx.generate_operations().unwrap();
        let output = String::from_utf8_lossy(&ops);

        assert!(output.contains("q\n"));
        assert!(output.contains("Q\n"));
        assert!(output.contains("f\n"));

        doc.add_page(page);
        assert!(doc.to_bytes().unwrap().len() > 0);
    }

    #[test]
    fn test_e2e_watermark_workflow() {
        let mut ctx = GraphicsContext::new();

        ctx.save_state();
        let _ = ctx.set_fill_opacity(0.2);
        ctx.translate(300.0, 400.0);
        ctx.rotate(45.0);
        ctx.set_font(Font::HelveticaBold, 72.0);
        ctx.draw_text("DRAFT", 0.0, 0.0).unwrap();
        ctx.restore_state();

        let ops = ctx.generate_operations().unwrap();
        let output = String::from_utf8_lossy(&ops);

        // Verify watermark structure
        assert!(output.contains("q\n")); // save state
        assert!(output.contains("Q\n")); // restore state
        assert!(output.contains("cm\n")); // transformations
        assert!(output.contains("BT\n")); // text begin
        assert!(output.contains("ET\n")); // text end
    }

    // ====== PHASE 5: set_custom_font emits Tf operator ======

    #[test]
    fn test_set_custom_font_emits_tf_operator() {
        let mut ctx = GraphicsContext::new();
        ctx.set_custom_font("NotoSansCJK", 14.0);

        let ops = ctx.operations();
        assert!(
            ops.contains("/NotoSansCJK 14 Tf"),
            "set_custom_font should emit Tf operator, got: {}",
            ops
        );
    }

    // ====== PHASE 3: unified custom font detection in draw_text ======

    #[test]
    fn test_draw_text_uses_is_custom_font_flag() {
        let mut ctx = GraphicsContext::new();
        // Name matches a standard font, but set via set_custom_font → flag is true
        ctx.set_custom_font("Helvetica", 12.0);
        ctx.clear(); // clear the Tf operator from set_custom_font

        ctx.draw_text("A", 10.0, 20.0).unwrap();
        let ops = ctx.operations();
        // Must use hex encoding because is_custom_font=true
        assert!(
            ops.contains("<0041> Tj"),
            "draw_text with is_custom_font=true should use hex, got: {}",
            ops
        );
    }

    #[test]
    fn test_draw_text_standard_font_uses_literal() {
        let mut ctx = GraphicsContext::new();
        ctx.set_font(Font::Helvetica, 12.0);
        ctx.clear();

        ctx.draw_text("Hello", 10.0, 20.0).unwrap();
        let ops = ctx.operations();
        assert!(
            ops.contains("(Hello) Tj"),
            "draw_text with standard font should use literal, got: {}",
            ops
        );
    }

    // ====== PHASE 2: surrogate pairs for SMP characters ======

    #[test]
    fn test_show_text_smp_character_uses_surrogate_pairs() {
        let mut ctx = GraphicsContext::new();
        ctx.set_font(Font::Custom("Emoji".to_string()), 12.0);

        ctx.begin_text();
        ctx.set_text_position(0.0, 0.0);
        // U+1F600 (GRINNING FACE) → surrogate pair: D83D DE00
        ctx.show_text("\u{1F600}").unwrap();
        ctx.end_text();

        let ops = ctx.operations();
        assert!(
            ops.contains("<D83DDE00> Tj"),
            "SMP character should use UTF-16BE surrogate pair, got: {}",
            ops
        );
        assert!(
            !ops.contains("FFFD"),
            "SMP character must NOT be replaced with FFFD"
        );
    }

    // ====== PHASE 1: save/restore font state ======

    #[test]
    fn test_save_restore_preserves_font_state() {
        let mut ctx = GraphicsContext::new();
        ctx.set_font(Font::Custom("CJK".to_string()), 12.0);
        assert!(ctx.is_custom_font);
        assert_eq!(ctx.current_font_name.as_deref(), Some("CJK"));
        assert_eq!(ctx.current_font_size, 12.0);

        ctx.save_state();
        ctx.set_font(Font::Helvetica, 10.0);
        assert!(!ctx.is_custom_font);
        assert_eq!(ctx.current_font_name.as_deref(), Some("Helvetica"));

        ctx.restore_state();
        assert!(
            ctx.is_custom_font,
            "is_custom_font must be restored after restore_state"
        );
        assert_eq!(ctx.current_font_name.as_deref(), Some("CJK"));
        assert_eq!(ctx.current_font_size, 12.0);
    }

    #[test]
    fn test_save_restore_mixed_font_encoding() {
        let mut ctx = GraphicsContext::new();
        ctx.set_font(Font::Custom("CJK".to_string()), 12.0);

        // Simulate table cell pattern: save → change font → text → restore → text
        ctx.save_state();
        ctx.set_font(Font::Helvetica, 10.0);
        ctx.begin_text();
        ctx.show_text("Hello").unwrap();
        ctx.end_text();
        ctx.restore_state();

        // After restore, CJK font should be active again
        ctx.begin_text();
        ctx.show_text("你好").unwrap();
        ctx.end_text();

        let ops = ctx.operations();
        // After restore, text must be hex-encoded (custom font restored)
        assert!(
            ops.contains("<4F60597D> Tj"),
            "After restore_state, CJK text should use hex encoding, got: {}",
            ops
        );
    }

    #[test]
    fn test_graphics_state_arc_str_save_restore() {
        // Verifies that save/restore correctly round-trips font names stored as Arc<str>,
        // and that the clone is O(1) (no String allocation per save).
        let mut ctx = GraphicsContext::new();

        // Set initial font
        ctx.set_font(Font::Custom("TestFont".to_string()), 14.0);
        assert_eq!(ctx.current_font_name.as_deref(), Some("TestFont"));
        assert!(ctx.is_custom_font);

        // Save state, change font
        ctx.save_state();
        ctx.set_font(Font::Custom("Other".to_string()), 10.0);
        assert_eq!(ctx.current_font_name.as_deref(), Some("Other"));

        // Restore: font must revert to "TestFont"
        ctx.restore_state();
        assert_eq!(
            ctx.current_font_name.as_deref(),
            Some("TestFont"),
            "Font name must be restored to TestFont after restore_state"
        );
        assert_eq!(ctx.current_font_size, 14.0);
        assert!(
            ctx.is_custom_font,
            "is_custom_font must be restored to true"
        );

        // Verify the Arc<str> is actually shared (same pointer after clone)
        if let Some(ref arc) = ctx.current_font_name {
            let cloned = arc.clone();
            assert_eq!(arc.as_ref(), cloned.as_ref());
            // Arc::ptr_eq confirms O(1) clone (same backing allocation)
            assert!(Arc::ptr_eq(arc, &cloned));
        }
    }

    /// RED for Phase 1 of the v2.7.0 IR refactor: with the current `String`
    /// emission, `set_line_width(f64::NAN)` writes literally `NaN w` into the
    /// content stream — invalid per ISO 32000-1 §7.3.3, same CWE-20 class as
    /// the colour-emission fix in 2.6.0 (issues #220, #221). Once the
    /// migration routes line width through `serialize_ops`, the helper
    /// `finite_or_zero` clamps non-finite floats to `0.0` at the emission
    /// boundary and the assertion below passes.
    #[test]
    fn nan_line_width_sanitised_at_emission() {
        let mut ctx = GraphicsContext::new();
        ctx.set_line_width(f64::NAN);
        let ops = ctx.operations();
        assert!(
            ops.contains("0.00 w\n"),
            "NaN line width must emit `0.00 w`, got: {ops:?}"
        );
        assert!(
            !ops.contains("NaN"),
            "`NaN` must not appear in any content-stream output, got: {ops:?}"
        );
    }

    #[test]
    fn pos_inf_line_width_sanitised_at_emission() {
        let mut ctx = GraphicsContext::new();
        ctx.set_line_width(f64::INFINITY);
        let ops = ctx.operations();
        assert!(
            ops.contains("0.00 w\n"),
            "+inf line width must emit `0.00 w`, got: {ops:?}"
        );
        assert!(
            !ops.contains("inf"),
            "`inf` must not appear in any content-stream output, got: {ops:?}"
        );
    }

    #[test]
    fn nan_path_coords_sanitised_at_emission() {
        let mut ctx = GraphicsContext::new();
        ctx.move_to(f64::NAN, 20.0);
        ctx.line_to(30.0, f64::NEG_INFINITY);
        ctx.rect(f64::NAN, f64::INFINITY, 100.0, f64::NEG_INFINITY);
        let ops = ctx.operations();
        assert!(
            !ops.contains("NaN") && !ops.contains("inf"),
            "non-finite floats must not appear in path operators, got: {ops:?}"
        );
        assert!(
            ops.contains("0.00 20.00 m\n"),
            "NaN x must clamp to 0.00 in `m` op, got: {ops:?}"
        );
        assert!(
            ops.contains("30.00 0.00 l\n"),
            "-inf y must clamp to 0.00 in `l` op, got: {ops:?}"
        );
        assert!(
            ops.contains("0.00 0.00 100.00 0.00 re\n"),
            "non-finite components must clamp to 0.00 in `re` op, got: {ops:?}"
        );
    }

    // ── GFX-019: ICC + named calibrated/Lab colour-space methods ──
    //
    // The content-stream wire format these assert against is fixed by
    // graphics/ops.rs: `SetFill/StrokeColorSpace(name)` → `/<name> cs\n`
    // (fill) / `/<name> CS\n` (stroke); `SetFill/StrokeColorComponents` →
    // each value as `"{v:.4} "` followed by `sc\n` (fill) / `SC\n` (stroke).
    // A fresh GraphicsContext has no operations, so the full serialised
    // string equals colour-space line + components line — which also proves
    // the colour space is emitted BEFORE the components (correct order).

    /// Format a components line exactly as `ops.rs` does (`"{v:.4} "` per
    /// value, then the painting operator + newline).
    fn expected_components(values: &[f64], op: &str) -> String {
        let mut s = String::new();
        for v in values {
            s.push_str(&format!("{v:.4} "));
        }
        s.push_str(op);
        s.push('\n');
        s
    }

    #[test]
    fn set_fill_color_icc_emits_named_cs_then_components() {
        let mut ctx = GraphicsContext::new();
        ctx.set_fill_color_icc("ICCRGB1", vec![0.25, 0.5, 0.75]);
        let expected = format!(
            "/ICCRGB1 cs\n{}",
            expected_components(&[0.25, 0.5, 0.75], "sc")
        );
        assert_eq!(
            ctx.operations(),
            expected,
            "ICC fill must emit `/ICCRGB1 cs` then `0.2500 0.5000 0.7500 sc`"
        );
    }

    #[test]
    fn set_stroke_color_icc_emits_named_cs_then_components() {
        let mut ctx = GraphicsContext::new();
        ctx.set_stroke_color_icc("ICCRGB1", vec![0.25, 0.5, 0.75]);
        let expected = format!(
            "/ICCRGB1 CS\n{}",
            expected_components(&[0.25, 0.5, 0.75], "SC")
        );
        assert_eq!(
            ctx.operations(),
            expected,
            "ICC stroke must emit `/ICCRGB1 CS` then `0.2500 0.5000 0.7500 SC`"
        );
    }

    #[test]
    fn set_fill_color_icc_single_channel_gray() {
        let mut ctx = GraphicsContext::new();
        ctx.set_fill_color_icc("ICCGray1", vec![0.42]);
        let expected = format!("/ICCGray1 cs\n{}", expected_components(&[0.42], "sc"));
        assert_eq!(ctx.operations(), expected);
    }

    #[test]
    fn set_fill_color_icc_cmyk_four_channels() {
        let mut ctx = GraphicsContext::new();
        ctx.set_fill_color_icc("ICCCmyk1", vec![0.1, 0.2, 0.3, 0.4]);
        let expected = format!(
            "/ICCCmyk1 cs\n{}",
            expected_components(&[0.1, 0.2, 0.3, 0.4], "sc")
        );
        assert_eq!(ctx.operations(), expected);
    }

    #[test]
    fn set_fill_color_calibrated_named_uses_caller_name() {
        let color = CalibratedColor::cal_rgb([0.1, 0.2, 0.3], CalRgbColorSpace::new());
        let mut ctx = GraphicsContext::new();
        ctx.set_fill_color_calibrated_named("MyCalRgb", color.clone());
        let expected = format!(
            "/MyCalRgb cs\n{}",
            expected_components(&color.values(), "sc")
        );
        assert_eq!(ctx.operations(), expected);
    }

    #[test]
    fn set_stroke_color_calibrated_named_uses_caller_name() {
        let color = CalibratedColor::cal_gray(0.6, CalGrayColorSpace::new());
        let mut ctx = GraphicsContext::new();
        ctx.set_stroke_color_calibrated_named("MyCalGray", color.clone());
        let expected = format!(
            "/MyCalGray CS\n{}",
            expected_components(&color.values(), "SC")
        );
        assert_eq!(ctx.operations(), expected);
    }

    #[test]
    fn set_fill_color_lab_named_uses_caller_name() {
        let color = LabColor::with_default(50.0, 12.0, -8.0);
        let mut ctx = GraphicsContext::new();
        ctx.set_fill_color_lab_named("MyLab", color.clone());
        let expected = format!("/MyLab cs\n{}", expected_components(&color.values(), "sc"));
        assert_eq!(ctx.operations(), expected);
    }

    #[test]
    fn set_stroke_color_lab_named_uses_caller_name() {
        let color = LabColor::with_default(50.0, 12.0, -8.0);
        let mut ctx = GraphicsContext::new();
        ctx.set_stroke_color_lab_named("MyLab", color.clone());
        let expected = format!("/MyLab CS\n{}", expected_components(&color.values(), "SC"));
        assert_eq!(ctx.operations(), expected);
    }

    // ── Regression: the four existing methods keep their hardcoded default
    //    resource names after being refactored to delegate to `_named`. ──

    #[test]
    fn legacy_calibrated_rgb_still_emits_calrgb1() {
        let color = CalibratedColor::cal_rgb([0.1, 0.2, 0.3], CalRgbColorSpace::new());
        let mut ctx = GraphicsContext::new();
        ctx.set_fill_color_calibrated(color.clone());
        let expected = format!(
            "/CalRGB1 cs\n{}",
            expected_components(&color.values(), "sc")
        );
        assert_eq!(ctx.operations(), expected);
    }

    #[test]
    fn legacy_calibrated_gray_still_emits_calgray1() {
        let color = CalibratedColor::cal_gray(0.6, CalGrayColorSpace::new());
        let mut ctx = GraphicsContext::new();
        ctx.set_stroke_color_calibrated(color.clone());
        let expected = format!(
            "/CalGray1 CS\n{}",
            expected_components(&color.values(), "SC")
        );
        assert_eq!(ctx.operations(), expected);
    }

    #[test]
    fn legacy_lab_still_emits_lab1() {
        let color = LabColor::with_default(50.0, 12.0, -8.0);
        let mut ctx = GraphicsContext::new();
        ctx.set_fill_color_lab(color.clone());
        let expected = format!("/Lab1 cs\n{}", expected_components(&color.values(), "sc"));
        assert_eq!(ctx.operations(), expected);
    }

    #[test]
    fn legacy_lab_stroke_still_emits_lab1() {
        let color = LabColor::with_default(50.0, 12.0, -8.0);
        let mut ctx = GraphicsContext::new();
        ctx.set_stroke_color_lab(color.clone());
        let expected = format!("/Lab1 CS\n{}", expected_components(&color.values(), "SC"));
        assert_eq!(ctx.operations(), expected);
    }

    // The empty-components guard is a `debug_assert!`, compiled out in release
    // builds (`cargo test --release`, tarpaulin coverage). Gating these tests to
    // `debug_assertions` keeps them honest: they only run where the assert fires.
    // Without the gate they fail in release with "did not panic as expected".
    #[test]
    #[cfg(debug_assertions)]
    #[should_panic(expected = "ICC fill colour components must not be empty")]
    fn set_fill_color_icc_empty_components_panics_in_debug() {
        // An empty component list would emit a bare `sc` operator with no
        // operands, invalid per ISO 32000-1 §8.6.8. The debug_assert guards
        // the caller-supplied ICC path (calibrated/Lab cannot reach this).
        let mut ctx = GraphicsContext::new();
        ctx.set_fill_color_icc("ICCRGB1", vec![]);
    }

    #[test]
    #[cfg(debug_assertions)]
    #[should_panic(expected = "ICC stroke colour components must not be empty")]
    fn set_stroke_color_icc_empty_components_panics_in_debug() {
        let mut ctx = GraphicsContext::new();
        ctx.set_stroke_color_icc("ICCRGB1", vec![]);
    }
}
