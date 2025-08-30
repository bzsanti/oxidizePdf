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
/// [a c e]   [x]   [ax + cy + e]
/// [b d f] × [y] = [bx + dy + f]
/// [0 0 1]   [1]   [    1      ]
/// ```
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
}
