mod color;
mod image;
mod path;

pub use color::Color;
pub use image::{ColorSpace as ImageColorSpace, Image, ImageFormat};
pub use path::{LineCap, LineJoin, PathBuilder};

use crate::error::Result;
use std::fmt::Write;

#[derive(Clone)]
pub struct GraphicsContext {
    operations: String,
    current_color: Color,
    stroke_color: Color,
    line_width: f64,
    fill_opacity: f64,
    stroke_opacity: f64,
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
        writeln!(&mut self.operations, "{} J", cap as u8).unwrap();
        self
    }

    pub fn set_line_join(&mut self, join: LineJoin) -> &mut Self {
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
}
