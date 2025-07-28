mod color;
mod image;
mod path;
mod state;

pub use color::Color;
pub use image::{ColorSpace as ImageColorSpace, Image, ImageFormat};
pub use path::{LineCap, LineJoin, PathBuilder};
pub use state::{
    BlendMode, ExtGState, ExtGStateFont, ExtGStateManager, Halftone, LineDashPattern,
    RenderingIntent, SoftMask, TransferFunction,
};

use crate::error::Result;
use crate::text::{ColumnContent, ColumnLayout, Font, ListElement, Table};
use std::fmt::Write;

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
    current_dash_pattern: Option<LineDashPattern>,
    current_miter_limit: f64,
    current_line_cap: LineCap,
    current_line_join: LineJoin,
    current_rendering_intent: RenderingIntent,
    current_flatness: f64,
    current_smoothness: f64,
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
            current_dash_pattern: None,
            current_miter_limit: 10.0,
            current_line_cap: LineCap::Butt,
            current_line_join: LineJoin::Miter,
            current_rendering_intent: RenderingIntent::RelativeColorimetric,
            current_flatness: 1.0,
            current_smoothness: 0.0,
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
        self.apply_stroke_color();
        self.operations.push_str("S\n");
        self
    }

    pub fn fill(&mut self) -> &mut Self {
        self.apply_fill_color();
        self.operations.push_str("f\n");
        self
    }

    pub fn fill_stroke(&mut self) -> &mut Self {
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
        self
    }

    /// Set the fill opacity (0.0 to 1.0)
    pub fn set_fill_opacity(&mut self, opacity: f64) -> &mut Self {
        self.fill_opacity = opacity.clamp(0.0, 1.0);
        self
    }

    /// Set the stroke opacity (0.0 to 1.0)
    pub fn set_stroke_opacity(&mut self, opacity: f64) -> &mut Self {
        self.stroke_opacity = opacity.clamp(0.0, 1.0);
        self
    }

    pub fn save_state(&mut self) -> &mut Self {
        self.operations.push_str("q\n");
        self
    }

    pub fn restore_state(&mut self) -> &mut Self {
        self.operations.push_str("Q\n");
        self
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

    /// Apply an ExtGState dictionary
    pub fn apply_extgstate(&mut self, state: ExtGState) -> Result<&mut Self> {
        let state_name = self.extgstate_manager.add_state(state)?;
        writeln!(&mut self.operations, "/{state_name} gs").unwrap();
        Ok(self)
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
}
