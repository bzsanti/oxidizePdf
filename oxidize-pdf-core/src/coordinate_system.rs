//! Coordinate system management for PDF rendering
//!
//! This module provides a flexible and extensible coordinate system framework
//! for PDF generation, supporting multiple coordinate conventions while maintaining
//! PDF standard compliance.
//!
//! # Coordinate Systems
//!
//! ## PDF Standard (Default)
//! - Origin (0,0) at bottom-left corner
//! - Y-axis increases upward (mathematical convention)
//! - X-axis increases rightward
//! - Units in points (1/72 inch)
//!
//! ## Screen Space
//! - Origin (0,0) at top-left corner
//! - Y-axis increases downward (screen convention)
//! - X-axis increases rightward
//! - Useful for developers familiar with web/screen graphics
//!
//! ## Custom Transform
//! - User-defined transformation matrix
//! - Allows arbitrary coordinate systems
//! - Full control over scaling, rotation, and translation

use crate::geometry::Point;

/// Coordinate system types supported for rendering
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CoordinateSystem {
    /// PDF standard: origin (0,0) at bottom-left, Y increases upward
    /// This is the native PDF coordinate system per ISO 32000-1:2008
    PdfStandard,

    /// Screen-like: origin (0,0) at top-left, Y increases downward
    /// Familiar to web developers and screen graphics programmers
    ScreenSpace,

    /// Custom transformation matrix for advanced use cases
    Custom(TransformMatrix),
}

/// 2D transformation matrix in homogeneous coordinates
///
/// Represents a 3x3 matrix in the form:
/// ```text
/// [a c e]   [x]     [ax + cy + e]
/// [b d f] × [y]  =  [bx + dy + f]
/// [0 0 1]   [1]     [    1      ]
/// ```
///
/// Where `[x]`, `[y]`, and `[1]` represent the input vector.
///
/// Common transformations:
/// - Identity: `a=1, b=0, c=0, d=1, e=0, f=0`
/// - Translation: `a=1, b=0, c=0, d=1, e=tx, f=ty`
/// - Scale: `a=sx, b=0, c=0, d=sy, e=0, f=0`
/// - Y-flip: `a=1, b=0, c=0, d=-1, e=0, f=page_height`
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TransformMatrix {
    /// Scale/rotation X component
    pub a: f64,
    /// Skew Y component  
    pub b: f64,
    /// Skew X component
    pub c: f64,
    /// Scale/rotation Y component
    pub d: f64,
    /// Translation X component
    pub e: f64,
    /// Translation Y component
    pub f: f64,
}

impl Default for CoordinateSystem {
    fn default() -> Self {
        Self::PdfStandard
    }
}

impl TransformMatrix {
    /// Identity transformation (no change)
    pub const IDENTITY: Self = Self {
        a: 1.0,
        b: 0.0,
        c: 0.0,
        d: 1.0,
        e: 0.0,
        f: 0.0,
    };

    /// Create a new transformation matrix
    pub fn new(a: f64, b: f64, c: f64, d: f64, e: f64, f: f64) -> Self {
        Self { a, b, c, d, e, f }
    }

    /// Create translation matrix
    pub fn translate(tx: f64, ty: f64) -> Self {
        Self {
            a: 1.0,
            b: 0.0,
            c: 0.0,
            d: 1.0,
            e: tx,
            f: ty,
        }
    }

    /// Create scaling matrix
    pub fn scale(sx: f64, sy: f64) -> Self {
        Self {
            a: sx,
            b: 0.0,
            c: 0.0,
            d: sy,
            e: 0.0,
            f: 0.0,
        }
    }

    /// Create rotation matrix (angle in radians)
    pub fn rotate(angle: f64) -> Self {
        let cos = angle.cos();
        let sin = angle.sin();
        Self {
            a: cos,
            b: sin,
            c: -sin,
            d: cos,
            e: 0.0,
            f: 0.0,
        }
    }

    /// Create Y-axis flip transformation for given page height
    pub fn flip_y(page_height: f64) -> Self {
        Self {
            a: 1.0,
            b: 0.0,
            c: 0.0,
            d: -1.0,
            e: 0.0,
            f: page_height,
        }
    }

    /// Matrix multiplication: self * other
    pub fn multiply(&self, other: &TransformMatrix) -> Self {
        Self {
            a: self.a * other.a + self.c * other.b,
            b: self.b * other.a + self.d * other.b,
            c: self.a * other.c + self.c * other.d,
            d: self.b * other.c + self.d * other.d,
            e: self.a * other.e + self.c * other.f + self.e,
            f: self.b * other.e + self.d * other.f + self.f,
        }
    }

    /// Transform a point using this matrix
    pub fn transform_point(&self, point: Point) -> Point {
        Point::new(
            self.a * point.x + self.c * point.y + self.e,
            self.b * point.x + self.d * point.y + self.f,
        )
    }

    /// Convert to PDF CTM (Current Transformation Matrix) string
    pub fn to_pdf_ctm(&self) -> String {
        format!(
            "{:.6} {:.6} {:.6} {:.6} {:.6} {:.6} cm",
            self.a, self.b, self.c, self.d, self.e, self.f
        )
    }
}

impl CoordinateSystem {
    /// Get transformation matrix to convert from this system to PDF standard
    pub fn to_pdf_standard_matrix(&self, page_height: f64) -> TransformMatrix {
        match *self {
            Self::PdfStandard => TransformMatrix::IDENTITY,
            Self::ScreenSpace => TransformMatrix::flip_y(page_height),
            Self::Custom(matrix) => matrix,
        }
    }

    /// Convert a point from this coordinate system to PDF standard
    pub fn to_pdf_standard(&self, point: Point, page_height: f64) -> Point {
        let matrix = self.to_pdf_standard_matrix(page_height);
        matrix.transform_point(point)
    }

    /// Convert a Y coordinate specifically (common operation)
    pub fn y_to_pdf_standard(&self, y: f64, page_height: f64) -> f64 {
        match *self {
            Self::PdfStandard => y,
            Self::ScreenSpace => page_height - y,
            Self::Custom(matrix) => {
                // For custom matrices, we need to transform a point
                let transformed = matrix.transform_point(Point::new(0.0, y));
                transformed.y
            }
        }
    }

    /// Check if this coordinate system grows upward (like PDF standard)
    pub fn grows_upward(&self) -> bool {
        match *self {
            Self::PdfStandard => true,
            Self::ScreenSpace => false,
            Self::Custom(matrix) => matrix.d > 0.0, // Positive Y scaling
        }
    }
}

/// Rendering context that maintains coordinate system state
#[derive(Debug)]
pub struct RenderContext {
    /// Active coordinate system
    pub coordinate_system: CoordinateSystem,
    /// Page dimensions
    pub page_width: f64,
    pub page_height: f64,
    /// Current transformation matrix
    pub current_transform: TransformMatrix,
}

impl RenderContext {
    /// Create a new render context
    pub fn new(coordinate_system: CoordinateSystem, page_width: f64, page_height: f64) -> Self {
        let current_transform = coordinate_system.to_pdf_standard_matrix(page_height);

        Self {
            coordinate_system,
            page_width,
            page_height,
            current_transform,
        }
    }

    /// Transform a point to PDF standard coordinates
    pub fn to_pdf_standard(&self, point: Point) -> Point {
        self.coordinate_system
            .to_pdf_standard(point, self.page_height)
    }

    /// Transform Y coordinate to PDF standard
    pub fn y_to_pdf(&self, y: f64) -> f64 {
        self.coordinate_system
            .y_to_pdf_standard(y, self.page_height)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry::Point;

    #[test]
    fn test_transform_matrix_identity() {
        let identity = TransformMatrix::IDENTITY;
        let point = Point::new(10.0, 20.0);
        let transformed = identity.transform_point(point);

        assert_eq!(transformed, point);
    }

    #[test]
    fn test_transform_matrix_translate() {
        let translate = TransformMatrix::translate(5.0, 10.0);
        let point = Point::new(1.0, 2.0);
        let transformed = translate.transform_point(point);

        assert_eq!(transformed, Point::new(6.0, 12.0));
    }

    #[test]
    fn test_transform_matrix_scale() {
        let scale = TransformMatrix::scale(2.0, 3.0);
        let point = Point::new(4.0, 5.0);
        let transformed = scale.transform_point(point);

        assert_eq!(transformed, Point::new(8.0, 15.0));
    }

    #[test]
    fn test_coordinate_system_pdf_standard() {
        let coord_system = CoordinateSystem::PdfStandard;
        let page_height = 842.0;
        let point = Point::new(100.0, 200.0);

        let pdf_point = coord_system.to_pdf_standard(point, page_height);
        assert_eq!(pdf_point, point); // Should be unchanged
    }

    #[test]
    fn test_coordinate_system_screen_space() {
        let coord_system = CoordinateSystem::ScreenSpace;
        let page_height = 842.0;
        let point = Point::new(100.0, 200.0);

        let pdf_point = coord_system.to_pdf_standard(point, page_height);
        assert_eq!(pdf_point, Point::new(100.0, 642.0)); // Y flipped
    }

    #[test]
    fn test_y_flip_matrix() {
        let page_height = 800.0;
        let flip = TransformMatrix::flip_y(page_height);

        // Top of page (screen coords) -> bottom of page (PDF coords)
        let top_screen = Point::new(0.0, 0.0);
        let top_pdf = flip.transform_point(top_screen);
        assert_eq!(top_pdf, Point::new(0.0, 800.0));

        // Bottom of page (screen coords) -> top of page (PDF coords)
        let bottom_screen = Point::new(0.0, 800.0);
        let bottom_pdf = flip.transform_point(bottom_screen);
        assert_eq!(bottom_pdf, Point::new(0.0, 0.0));
    }

    #[test]
    fn test_render_context() {
        let context = RenderContext::new(CoordinateSystem::ScreenSpace, 595.0, 842.0);

        let screen_point = Point::new(100.0, 100.0);
        let pdf_point = context.to_pdf_standard(screen_point);

        assert_eq!(pdf_point, Point::new(100.0, 742.0));
    }

    // =============================================================================
    // RIGOROUS TESTS FOR TransformMatrix
    // =============================================================================

    #[test]
    fn test_transform_matrix_rotate_90_degrees() {
        let rotate = TransformMatrix::rotate(std::f64::consts::FRAC_PI_2); // 90 degrees
        let point = Point::new(1.0, 0.0);
        let transformed = rotate.transform_point(point);

        // (1,0) rotated 90° should be (0,1)
        assert!((transformed.x - 0.0).abs() < 1e-10, "X should be ~0");
        assert!((transformed.y - 1.0).abs() < 1e-10, "Y should be ~1");
    }

    #[test]
    fn test_transform_matrix_rotate_180_degrees() {
        let rotate = TransformMatrix::rotate(std::f64::consts::PI); // 180 degrees
        let point = Point::new(1.0, 0.0);
        let transformed = rotate.transform_point(point);

        // (1,0) rotated 180° should be (-1,0)
        assert!((transformed.x - (-1.0)).abs() < 1e-10, "X should be ~-1");
        assert!((transformed.y - 0.0).abs() < 1e-10, "Y should be ~0");
    }

    #[test]
    fn test_transform_matrix_rotate_270_degrees() {
        let rotate = TransformMatrix::rotate(3.0 * std::f64::consts::FRAC_PI_2); // 270 degrees
        let point = Point::new(1.0, 0.0);
        let transformed = rotate.transform_point(point);

        // (1,0) rotated 270° should be (0,-1)
        assert!((transformed.x - 0.0).abs() < 1e-10, "X should be ~0");
        assert!((transformed.y - (-1.0)).abs() < 1e-10, "Y should be ~-1");
    }

    #[test]
    fn test_transform_matrix_multiply_identity() {
        let matrix = TransformMatrix::new(2.0, 3.0, 4.0, 5.0, 6.0, 7.0);
        let result = matrix.multiply(&TransformMatrix::IDENTITY);

        // Multiplying by identity should return the same matrix
        assert_eq!(result.a, 2.0);
        assert_eq!(result.b, 3.0);
        assert_eq!(result.c, 4.0);
        assert_eq!(result.d, 5.0);
        assert_eq!(result.e, 6.0);
        assert_eq!(result.f, 7.0);
    }

    #[test]
    fn test_transform_matrix_multiply_translate_then_scale() {
        let translate = TransformMatrix::translate(10.0, 20.0);
        let scale = TransformMatrix::scale(2.0, 3.0);

        // Translate then scale: scale is applied first, then translate
        let combined = translate.multiply(&scale);
        let point = Point::new(5.0, 5.0);
        let transformed = combined.transform_point(point);

        // Point (5,5) scaled by (2,3) = (10,15), then translated by (10,20) = (20,35)
        assert_eq!(transformed.x, 20.0);
        assert_eq!(transformed.y, 35.0);
    }

    #[test]
    fn test_transform_matrix_multiply_scale_then_translate() {
        let scale = TransformMatrix::scale(2.0, 3.0);
        let translate = TransformMatrix::translate(10.0, 20.0);

        // Scale then translate: translate is applied first, then scale
        let combined = scale.multiply(&translate);
        let point = Point::new(5.0, 5.0);
        let transformed = combined.transform_point(point);

        // Point (5,5) translated by (10,20) = (15,25), then scaled by (2,3) = (30,75)
        assert_eq!(transformed.x, 30.0);
        assert_eq!(transformed.y, 75.0);
    }

    #[test]
    fn test_transform_matrix_to_pdf_ctm() {
        let matrix = TransformMatrix::new(1.5, 0.5, -0.5, 2.0, 10.0, 20.0);
        let ctm = matrix.to_pdf_ctm();

        assert_eq!(
            ctm,
            "1.500000 0.500000 -0.500000 2.000000 10.000000 20.000000 cm"
        );
    }

    #[test]
    fn test_transform_matrix_to_pdf_ctm_with_precision() {
        let matrix = TransformMatrix::new(
            0.123456789,
            0.987654321,
            -0.111111111,
            0.222222222,
            100.123456,
            200.987654,
        );
        let ctm = matrix.to_pdf_ctm();

        // Should round to 6 decimal places
        assert!(ctm.contains("0.123457")); // Rounded up from 0.123456789
        assert!(ctm.contains("0.987654")); // Rounded down from 0.987654321
        assert!(ctm.contains("-0.111111"));
        assert!(ctm.contains("0.222222"));
        assert!(ctm.contains("100.123456"));
        assert!(ctm.contains("200.987654"));
        assert!(ctm.ends_with(" cm"));
    }

    #[test]
    fn test_transform_matrix_flip_y_zero_height() {
        let flip = TransformMatrix::flip_y(0.0);
        let point = Point::new(100.0, 50.0);
        let transformed = flip.transform_point(point);

        // With page_height=0, flip should map (100,50) to (100,-50)
        assert_eq!(transformed.x, 100.0);
        assert_eq!(transformed.y, -50.0);
    }

    #[test]
    fn test_transform_matrix_flip_y_negative_height() {
        let flip = TransformMatrix::flip_y(-100.0);
        let point = Point::new(50.0, 25.0);
        let transformed = flip.transform_point(point);

        // With page_height=-100, flip should map (50,25) to (50,-125)
        assert_eq!(transformed.x, 50.0);
        assert_eq!(transformed.y, -125.0);
    }

    #[test]
    fn test_transform_matrix_scale_zero() {
        let scale = TransformMatrix::scale(0.0, 0.0);
        let point = Point::new(100.0, 200.0);
        let transformed = scale.transform_point(point);

        // Scaling by zero should collapse point to origin
        assert_eq!(transformed.x, 0.0);
        assert_eq!(transformed.y, 0.0);
    }

    #[test]
    fn test_transform_matrix_scale_negative() {
        let scale = TransformMatrix::scale(-1.0, -2.0);
        let point = Point::new(10.0, 20.0);
        let transformed = scale.transform_point(point);

        // Negative scaling should flip and scale
        assert_eq!(transformed.x, -10.0);
        assert_eq!(transformed.y, -40.0);
    }

    #[test]
    fn test_transform_matrix_translate_zero() {
        let translate = TransformMatrix::translate(0.0, 0.0);
        let point = Point::new(50.0, 75.0);
        let transformed = translate.transform_point(point);

        // Zero translation should not change point
        assert_eq!(transformed, point);
    }

    #[test]
    fn test_transform_matrix_translate_negative() {
        let translate = TransformMatrix::translate(-10.0, -20.0);
        let point = Point::new(100.0, 200.0);
        let transformed = translate.transform_point(point);

        assert_eq!(transformed.x, 90.0);
        assert_eq!(transformed.y, 180.0);
    }

    // =============================================================================
    // RIGOROUS TESTS FOR CoordinateSystem
    // =============================================================================

    #[test]
    fn test_coordinate_system_default() {
        let default_cs = CoordinateSystem::default();
        assert!(
            matches!(default_cs, CoordinateSystem::PdfStandard),
            "Default should be PdfStandard"
        );
    }

    #[test]
    fn test_coordinate_system_pdf_standard_identity() {
        let cs = CoordinateSystem::PdfStandard;
        let matrix = cs.to_pdf_standard_matrix(500.0);

        // PdfStandard should return identity matrix
        assert_eq!(matrix.a, 1.0);
        assert_eq!(matrix.b, 0.0);
        assert_eq!(matrix.c, 0.0);
        assert_eq!(matrix.d, 1.0);
        assert_eq!(matrix.e, 0.0);
        assert_eq!(matrix.f, 0.0);
    }

    #[test]
    fn test_coordinate_system_screen_space_flip() {
        let cs = CoordinateSystem::ScreenSpace;
        let page_height = 600.0;
        let matrix = cs.to_pdf_standard_matrix(page_height);

        // ScreenSpace should flip Y-axis
        assert_eq!(matrix.a, 1.0);
        assert_eq!(matrix.b, 0.0);
        assert_eq!(matrix.c, 0.0);
        assert_eq!(matrix.d, -1.0);
        assert_eq!(matrix.e, 0.0);
        assert_eq!(matrix.f, page_height);
    }

    #[test]
    fn test_coordinate_system_custom_matrix() {
        let custom_matrix = TransformMatrix::new(2.0, 0.0, 0.0, 2.0, 50.0, 100.0);
        let cs = CoordinateSystem::Custom(custom_matrix);

        let retrieved_matrix = cs.to_pdf_standard_matrix(500.0);

        // Custom should return the exact matrix provided
        assert_eq!(retrieved_matrix.a, 2.0);
        assert_eq!(retrieved_matrix.b, 0.0);
        assert_eq!(retrieved_matrix.c, 0.0);
        assert_eq!(retrieved_matrix.d, 2.0);
        assert_eq!(retrieved_matrix.e, 50.0);
        assert_eq!(retrieved_matrix.f, 100.0);
    }

    #[test]
    fn test_coordinate_system_y_to_pdf_standard_pdf_standard() {
        let cs = CoordinateSystem::PdfStandard;
        let y = 200.0;
        let page_height = 842.0;

        let pdf_y = cs.y_to_pdf_standard(y, page_height);

        // PdfStandard should not change Y
        assert_eq!(pdf_y, 200.0);
    }

    #[test]
    fn test_coordinate_system_y_to_pdf_standard_screen_space() {
        let cs = CoordinateSystem::ScreenSpace;
        let y = 200.0;
        let page_height = 842.0;

        let pdf_y = cs.y_to_pdf_standard(y, page_height);

        // ScreenSpace should flip Y: 842 - 200 = 642
        assert_eq!(pdf_y, 642.0);
    }

    #[test]
    fn test_coordinate_system_y_to_pdf_standard_custom() {
        let custom_matrix = TransformMatrix::new(1.0, 0.0, 0.0, -2.0, 0.0, 500.0);
        let cs = CoordinateSystem::Custom(custom_matrix);
        let y = 100.0;
        let page_height = 600.0; // Not used for custom

        let pdf_y = cs.y_to_pdf_standard(y, page_height);

        // Custom matrix transforms (0, 100): y' = b*0 + d*100 + f = 0*0 + (-2)*100 + 500 = 300
        assert_eq!(pdf_y, 300.0);
    }

    #[test]
    fn test_coordinate_system_grows_upward_pdf_standard() {
        let cs = CoordinateSystem::PdfStandard;
        assert!(cs.grows_upward(), "PdfStandard should grow upward");
    }

    #[test]
    fn test_coordinate_system_grows_upward_screen_space() {
        let cs = CoordinateSystem::ScreenSpace;
        assert!(
            !cs.grows_upward(),
            "ScreenSpace should NOT grow upward (Y increases downward)"
        );
    }

    #[test]
    fn test_coordinate_system_grows_upward_custom_positive_d() {
        let custom_matrix = TransformMatrix::new(1.0, 0.0, 0.0, 2.0, 0.0, 0.0);
        let cs = CoordinateSystem::Custom(custom_matrix);
        assert!(
            cs.grows_upward(),
            "Custom with positive d should grow upward"
        );
    }

    #[test]
    fn test_coordinate_system_grows_upward_custom_negative_d() {
        let custom_matrix = TransformMatrix::new(1.0, 0.0, 0.0, -1.0, 0.0, 100.0);
        let cs = CoordinateSystem::Custom(custom_matrix);
        assert!(
            !cs.grows_upward(),
            "Custom with negative d should NOT grow upward"
        );
    }

    #[test]
    fn test_coordinate_system_grows_upward_custom_zero_d() {
        let custom_matrix = TransformMatrix::new(1.0, 0.0, 0.0, 0.0, 0.0, 0.0);
        let cs = CoordinateSystem::Custom(custom_matrix);
        assert!(
            !cs.grows_upward(),
            "Custom with zero d should NOT grow upward"
        );
    }

    // =============================================================================
    // RIGOROUS TESTS FOR RenderContext
    // =============================================================================

    #[test]
    fn test_render_context_new_pdf_standard() {
        let context = RenderContext::new(CoordinateSystem::PdfStandard, 595.0, 842.0);

        assert_eq!(context.page_width, 595.0);
        assert_eq!(context.page_height, 842.0);
        assert!(matches!(
            context.coordinate_system,
            CoordinateSystem::PdfStandard
        ));

        // Transform should be identity for PdfStandard
        assert_eq!(context.current_transform.a, 1.0);
        assert_eq!(context.current_transform.b, 0.0);
        assert_eq!(context.current_transform.c, 0.0);
        assert_eq!(context.current_transform.d, 1.0);
        assert_eq!(context.current_transform.e, 0.0);
        assert_eq!(context.current_transform.f, 0.0);
    }

    #[test]
    fn test_render_context_new_screen_space() {
        let context = RenderContext::new(CoordinateSystem::ScreenSpace, 595.0, 842.0);

        assert_eq!(context.page_width, 595.0);
        assert_eq!(context.page_height, 842.0);
        assert!(matches!(
            context.coordinate_system,
            CoordinateSystem::ScreenSpace
        ));

        // Transform should be Y-flip for ScreenSpace
        assert_eq!(context.current_transform.a, 1.0);
        assert_eq!(context.current_transform.b, 0.0);
        assert_eq!(context.current_transform.c, 0.0);
        assert_eq!(context.current_transform.d, -1.0);
        assert_eq!(context.current_transform.e, 0.0);
        assert_eq!(context.current_transform.f, 842.0);
    }

    #[test]
    fn test_render_context_new_custom() {
        let custom_matrix = TransformMatrix::scale(2.0, 2.0);
        let context = RenderContext::new(CoordinateSystem::Custom(custom_matrix), 595.0, 842.0);

        assert_eq!(context.page_width, 595.0);
        assert_eq!(context.page_height, 842.0);

        // Transform should be the custom matrix
        assert_eq!(context.current_transform.a, 2.0);
        assert_eq!(context.current_transform.d, 2.0);
    }

    #[test]
    fn test_render_context_to_pdf_standard_pdf_standard() {
        let context = RenderContext::new(CoordinateSystem::PdfStandard, 595.0, 842.0);
        let point = Point::new(100.0, 200.0);
        let pdf_point = context.to_pdf_standard(point);

        // PdfStandard should not change point
        assert_eq!(pdf_point, point);
    }

    #[test]
    fn test_render_context_to_pdf_standard_screen_space() {
        let context = RenderContext::new(CoordinateSystem::ScreenSpace, 595.0, 842.0);
        let point = Point::new(100.0, 200.0);
        let pdf_point = context.to_pdf_standard(point);

        // ScreenSpace should flip Y: (100, 842-200) = (100, 642)
        assert_eq!(pdf_point, Point::new(100.0, 642.0));
    }

    #[test]
    fn test_render_context_y_to_pdf_pdf_standard() {
        let context = RenderContext::new(CoordinateSystem::PdfStandard, 595.0, 842.0);
        let y = 300.0;
        let pdf_y = context.y_to_pdf(y);

        // PdfStandard should not change Y
        assert_eq!(pdf_y, 300.0);
    }

    #[test]
    fn test_render_context_y_to_pdf_screen_space() {
        let context = RenderContext::new(CoordinateSystem::ScreenSpace, 595.0, 842.0);
        let y = 300.0;
        let pdf_y = context.y_to_pdf(y);

        // ScreenSpace should flip Y: 842 - 300 = 542
        assert_eq!(pdf_y, 542.0);
    }

    #[test]
    fn test_render_context_edge_case_zero_dimensions() {
        let context = RenderContext::new(CoordinateSystem::PdfStandard, 0.0, 0.0);

        assert_eq!(context.page_width, 0.0);
        assert_eq!(context.page_height, 0.0);

        let point = Point::new(10.0, 20.0);
        let pdf_point = context.to_pdf_standard(point);

        // Should still work with zero dimensions
        assert_eq!(pdf_point, point);
    }

    #[test]
    fn test_render_context_edge_case_negative_dimensions() {
        let context = RenderContext::new(CoordinateSystem::ScreenSpace, 595.0, -842.0);

        assert_eq!(context.page_width, 595.0);
        assert_eq!(context.page_height, -842.0);

        let point = Point::new(100.0, 200.0);
        let pdf_point = context.to_pdf_standard(point);

        // ScreenSpace with negative height: -842 - 200 = -1042
        assert_eq!(pdf_point, Point::new(100.0, -1042.0));
    }

    #[test]
    fn test_coordinate_system_equality() {
        let cs1 = CoordinateSystem::PdfStandard;
        let cs2 = CoordinateSystem::PdfStandard;
        assert_eq!(cs1, cs2);

        let cs3 = CoordinateSystem::ScreenSpace;
        let cs4 = CoordinateSystem::ScreenSpace;
        assert_eq!(cs3, cs4);

        assert_ne!(cs1, cs3);

        let matrix1 = TransformMatrix::IDENTITY;
        let matrix2 = TransformMatrix::IDENTITY;
        let cs5 = CoordinateSystem::Custom(matrix1);
        let cs6 = CoordinateSystem::Custom(matrix2);
        assert_eq!(cs5, cs6);
    }

    #[test]
    fn test_transform_matrix_equality() {
        let m1 = TransformMatrix::new(1.0, 2.0, 3.0, 4.0, 5.0, 6.0);
        let m2 = TransformMatrix::new(1.0, 2.0, 3.0, 4.0, 5.0, 6.0);
        assert_eq!(m1, m2);

        let m3 = TransformMatrix::new(1.0, 2.0, 3.0, 4.0, 5.0, 7.0);
        assert_ne!(m1, m3);
    }
}
