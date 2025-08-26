//! Clipping path support according to ISO 32000-1 Section 8.5
//!
//! This module provides comprehensive support for PDF clipping paths
//! as specified in ISO 32000-1:2008.

use crate::error::Result;
use crate::graphics::{PathCommand, WindingRule};
use std::fmt::Write;

/// Clipping path state
#[derive(Debug, Clone)]
pub struct ClippingPath {
    /// Path commands that define the clipping region
    commands: Vec<PathCommand>,
    /// Winding rule for determining interior/exterior
    winding_rule: WindingRule,
    /// Whether this is a text clipping path
    is_text_clip: bool,
}

impl ClippingPath {
    /// Create a new empty clipping path
    pub fn new() -> Self {
        Self {
            commands: Vec::new(),
            winding_rule: WindingRule::NonZero,
            is_text_clip: false,
        }
    }

    /// Create a rectangular clipping path
    pub fn rect(x: f64, y: f64, width: f64, height: f64) -> Self {
        let mut path = Self::new();
        path.add_rect(x, y, width, height);
        path
    }

    /// Create a circular clipping path
    pub fn circle(cx: f64, cy: f64, radius: f64) -> Self {
        let mut path = Self::new();
        path.add_circle(cx, cy, radius);
        path
    }

    /// Create an elliptical clipping path
    pub fn ellipse(cx: f64, cy: f64, rx: f64, ry: f64) -> Self {
        let mut path = Self::new();
        path.add_ellipse(cx, cy, rx, ry);
        path
    }

    /// Set the winding rule
    pub fn with_winding_rule(mut self, rule: WindingRule) -> Self {
        self.winding_rule = rule;
        self
    }

    /// Mark as text clipping path
    pub fn as_text_clip(mut self) -> Self {
        self.is_text_clip = true;
        self
    }

    /// Add a move command
    pub fn move_to(&mut self, x: f64, y: f64) -> &mut Self {
        self.commands.push(PathCommand::MoveTo { x, y });
        self
    }

    /// Add a line command
    pub fn line_to(&mut self, x: f64, y: f64) -> &mut Self {
        self.commands.push(PathCommand::LineTo { x, y });
        self
    }

    /// Add a cubic Bézier curve
    pub fn curve_to(&mut self, x1: f64, y1: f64, x2: f64, y2: f64, x3: f64, y3: f64) -> &mut Self {
        self.commands.push(PathCommand::CurveTo {
            x1,
            y1,
            x2,
            y2,
            x3,
            y3,
        });
        self
    }

    /// Add a rectangle
    pub fn add_rect(&mut self, x: f64, y: f64, width: f64, height: f64) -> &mut Self {
        self.commands.push(PathCommand::Rectangle {
            x,
            y,
            width,
            height,
        });
        self
    }

    /// Add a circle using Bézier curves
    pub fn add_circle(&mut self, cx: f64, cy: f64, radius: f64) -> &mut Self {
        // Magic constant for approximating circle with cubic Bézier curves
        const KAPPA: f64 = 0.5522847498307933;
        let k = radius * KAPPA;

        // Start at top of circle
        self.move_to(cx, cy + radius);

        // First quarter (top to right)
        self.curve_to(cx + k, cy + radius, cx + radius, cy + k, cx + radius, cy);

        // Second quarter (right to bottom)
        self.curve_to(cx + radius, cy - k, cx + k, cy - radius, cx, cy - radius);

        // Third quarter (bottom to left)
        self.curve_to(cx - k, cy - radius, cx - radius, cy - k, cx - radius, cy);

        // Fourth quarter (left to top)
        self.curve_to(cx - radius, cy + k, cx - k, cy + radius, cx, cy + radius);

        self.close_path();
        self
    }

    /// Add an ellipse using Bézier curves
    pub fn add_ellipse(&mut self, cx: f64, cy: f64, rx: f64, ry: f64) -> &mut Self {
        const KAPPA: f64 = 0.5522847498307933;
        let kx = rx * KAPPA;
        let ky = ry * KAPPA;

        // Start at top of ellipse
        self.move_to(cx, cy + ry);

        // First quarter
        self.curve_to(cx + kx, cy + ry, cx + rx, cy + ky, cx + rx, cy);

        // Second quarter
        self.curve_to(cx + rx, cy - ky, cx + kx, cy - ry, cx, cy - ry);

        // Third quarter
        self.curve_to(cx - kx, cy - ry, cx - rx, cy - ky, cx - rx, cy);

        // Fourth quarter
        self.curve_to(cx - rx, cy + ky, cx - kx, cy + ry, cx, cy + ry);

        self.close_path();
        self
    }

    /// Add a rounded rectangle
    pub fn add_rounded_rect(
        &mut self,
        x: f64,
        y: f64,
        width: f64,
        height: f64,
        radius: f64,
    ) -> &mut Self {
        let r = radius.min(width / 2.0).min(height / 2.0);
        const KAPPA: f64 = 0.5522847498307933;
        let k = r * KAPPA;

        // Start at top-left corner (after radius)
        self.move_to(x + r, y);

        // Top edge
        self.line_to(x + width - r, y);

        // Top-right corner
        self.curve_to(x + width - r + k, y, x + width, y + r - k, x + width, y + r);

        // Right edge
        self.line_to(x + width, y + height - r);

        // Bottom-right corner
        self.curve_to(
            x + width,
            y + height - r + k,
            x + width - r + k,
            y + height,
            x + width - r,
            y + height,
        );

        // Bottom edge
        self.line_to(x + r, y + height);

        // Bottom-left corner
        self.curve_to(
            x + r - k,
            y + height,
            x,
            y + height - r + k,
            x,
            y + height - r,
        );

        // Left edge
        self.line_to(x, y + r);

        // Top-left corner
        self.curve_to(x, y + r - k, x + r - k, y, x + r, y);

        self.close_path();
        self
    }

    /// Add a polygon
    pub fn add_polygon(&mut self, points: &[(f64, f64)]) -> &mut Self {
        if let Some((first, rest)) = points.split_first() {
            self.move_to(first.0, first.1);
            for point in rest {
                self.line_to(point.0, point.1);
            }
            self.close_path();
        }
        self
    }

    /// Close the current subpath
    pub fn close_path(&mut self) -> &mut Self {
        self.commands.push(PathCommand::ClosePath);
        self
    }

    /// Check if the path is empty
    pub fn is_empty(&self) -> bool {
        self.commands.is_empty()
    }

    /// Get the path commands
    pub fn commands(&self) -> &[PathCommand] {
        &self.commands
    }

    /// Generate PDF operations for this clipping path
    pub fn to_pdf_operations(&self) -> Result<String> {
        let mut ops = String::new();

        // Generate path construction commands
        for cmd in &self.commands {
            match cmd {
                PathCommand::MoveTo { x, y } => {
                    writeln!(&mut ops, "{:.3} {:.3} m", x, y).unwrap();
                }
                PathCommand::LineTo { x, y } => {
                    writeln!(&mut ops, "{:.3} {:.3} l", x, y).unwrap();
                }
                PathCommand::CurveTo {
                    x1,
                    y1,
                    x2,
                    y2,
                    x3,
                    y3,
                } => {
                    writeln!(
                        &mut ops,
                        "{:.3} {:.3} {:.3} {:.3} {:.3} {:.3} c",
                        x1, y1, x2, y2, x3, y3
                    )
                    .unwrap();
                }
                PathCommand::Rectangle {
                    x,
                    y,
                    width,
                    height,
                } => {
                    writeln!(&mut ops, "{:.3} {:.3} {:.3} {:.3} re", x, y, width, height).unwrap();
                }
                PathCommand::ClosePath => {
                    writeln!(&mut ops, "h").unwrap();
                }
            }
        }

        // Apply clipping based on winding rule
        match self.winding_rule {
            WindingRule::NonZero => writeln!(&mut ops, "W").unwrap(),
            WindingRule::EvenOdd => writeln!(&mut ops, "W*").unwrap(),
        }

        // End path without filling or stroking
        writeln!(&mut ops, "n").unwrap();

        Ok(ops)
    }

    /// Intersect with another clipping path
    pub fn intersect(&mut self, other: &ClippingPath) -> &mut Self {
        // In PDF, intersection is achieved by applying both clips sequentially
        // Here we just append the commands
        self.commands.extend_from_slice(&other.commands);
        self
    }

    /// Clear all commands
    pub fn clear(&mut self) {
        self.commands.clear();
    }
}

impl Default for ClippingPath {
    fn default() -> Self {
        Self::new()
    }
}

/// Clipping region manager for handling multiple clipping paths
#[derive(Debug, Clone)]
pub struct ClippingRegion {
    /// Stack of clipping paths (for save/restore operations)
    stack: Vec<ClippingPath>,
    /// Current active clipping path
    current: Option<ClippingPath>,
}

impl ClippingRegion {
    /// Create a new clipping region manager
    pub fn new() -> Self {
        Self {
            stack: Vec::new(),
            current: None,
        }
    }

    /// Set the current clipping path
    pub fn set_clip(&mut self, path: ClippingPath) {
        self.current = Some(path);
    }

    /// Clear the current clipping path
    pub fn clear_clip(&mut self) {
        self.current = None;
    }

    /// Save the current clipping state
    pub fn save(&mut self) {
        if let Some(ref current) = self.current {
            self.stack.push(current.clone());
        }
    }

    /// Restore the previous clipping state
    pub fn restore(&mut self) {
        if let Some(saved) = self.stack.pop() {
            self.current = Some(saved);
        }
    }

    /// Get the current clipping path
    pub fn current(&self) -> Option<&ClippingPath> {
        self.current.as_ref()
    }

    /// Check if there's an active clipping path
    pub fn has_clip(&self) -> bool {
        self.current.is_some()
    }

    /// Generate PDF operations for the current clipping path
    pub fn to_pdf_operations(&self) -> Result<Option<String>> {
        if let Some(ref clip) = self.current {
            Ok(Some(clip.to_pdf_operations()?))
        } else {
            Ok(None)
        }
    }
}

impl Default for ClippingRegion {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clipping_path_creation() {
        let path = ClippingPath::new();
        assert!(path.is_empty());
        assert!(!path.is_text_clip);
        assert_eq!(path.winding_rule, WindingRule::NonZero);
    }

    #[test]
    fn test_rect_clipping_path() {
        let path = ClippingPath::rect(10.0, 20.0, 100.0, 50.0);
        assert!(!path.is_empty());
        assert_eq!(path.commands.len(), 1);
    }

    #[test]
    fn test_circle_clipping_path() {
        let path = ClippingPath::circle(50.0, 50.0, 25.0);
        assert!(!path.is_empty());
        // Circle is approximated with 4 cubic Bézier curves + move + close
        assert!(path.commands.len() >= 6);
    }

    #[test]
    fn test_ellipse_clipping_path() {
        let path = ClippingPath::ellipse(50.0, 50.0, 30.0, 20.0);
        assert!(!path.is_empty());
        assert!(path.commands.len() >= 6);
    }

    #[test]
    fn test_winding_rule() {
        let path = ClippingPath::new().with_winding_rule(WindingRule::EvenOdd);
        assert_eq!(path.winding_rule, WindingRule::EvenOdd);
    }

    #[test]
    fn test_text_clip() {
        let path = ClippingPath::new().as_text_clip();
        assert!(path.is_text_clip);
    }

    #[test]
    fn test_path_construction() {
        let mut path = ClippingPath::new();
        path.move_to(0.0, 0.0)
            .line_to(100.0, 0.0)
            .line_to(100.0, 100.0)
            .line_to(0.0, 100.0)
            .close_path();

        assert_eq!(path.commands.len(), 5);
    }

    #[test]
    fn test_curve_to() {
        let mut path = ClippingPath::new();
        path.move_to(0.0, 0.0)
            .curve_to(10.0, 10.0, 20.0, 20.0, 30.0, 30.0);

        assert_eq!(path.commands.len(), 2);
    }

    #[test]
    fn test_polygon() {
        let mut path = ClippingPath::new();
        let points = vec![(0.0, 0.0), (50.0, 0.0), (25.0, 50.0)];
        path.add_polygon(&points);

        // move + 2 lines + close
        assert_eq!(path.commands.len(), 4);
    }

    #[test]
    fn test_rounded_rect() {
        let mut path = ClippingPath::new();
        path.add_rounded_rect(10.0, 10.0, 100.0, 50.0, 5.0);

        // Rounded rect has 4 corners (curves) + 4 edges (lines) + move + close
        assert!(path.commands.len() >= 10);
    }

    #[test]
    fn test_pdf_operations_nonzero() {
        let path = ClippingPath::rect(0.0, 0.0, 100.0, 100.0);
        let ops = path.to_pdf_operations().unwrap();

        assert!(ops.contains("0.000 0.000 100.000 100.000 re"));
        assert!(ops.contains("W")); // Non-zero winding
        assert!(ops.contains("n")); // End path
    }

    #[test]
    fn test_pdf_operations_evenodd() {
        let path =
            ClippingPath::rect(0.0, 0.0, 100.0, 100.0).with_winding_rule(WindingRule::EvenOdd);
        let ops = path.to_pdf_operations().unwrap();

        assert!(ops.contains("W*")); // Even-odd winding
    }

    #[test]
    fn test_intersect_paths() {
        let mut path1 = ClippingPath::rect(0.0, 0.0, 100.0, 100.0);
        let path2 = ClippingPath::rect(50.0, 50.0, 100.0, 100.0);

        path1.intersect(&path2);
        assert_eq!(path1.commands.len(), 2);
    }

    #[test]
    fn test_clear_path() {
        let mut path = ClippingPath::rect(0.0, 0.0, 100.0, 100.0);
        assert!(!path.is_empty());

        path.clear();
        assert!(path.is_empty());
    }

    #[test]
    fn test_clipping_region_creation() {
        let region = ClippingRegion::new();
        assert!(!region.has_clip());
        assert!(region.current().is_none());
    }

    #[test]
    fn test_clipping_region_set_clip() {
        let mut region = ClippingRegion::new();
        let path = ClippingPath::rect(0.0, 0.0, 100.0, 100.0);

        region.set_clip(path);
        assert!(region.has_clip());
        assert!(region.current().is_some());
    }

    #[test]
    fn test_clipping_region_clear() {
        let mut region = ClippingRegion::new();
        region.set_clip(ClippingPath::rect(0.0, 0.0, 100.0, 100.0));
        assert!(region.has_clip());

        region.clear_clip();
        assert!(!region.has_clip());
    }

    #[test]
    fn test_clipping_region_save_restore() {
        let mut region = ClippingRegion::new();
        let path1 = ClippingPath::rect(0.0, 0.0, 100.0, 100.0);
        let path2 = ClippingPath::rect(50.0, 50.0, 50.0, 50.0);

        region.set_clip(path1);
        region.save();
        region.set_clip(path2);

        // Current should be path2
        assert!(region.has_clip());

        region.restore();
        // Should be back to path1
        assert!(region.has_clip());
    }

    #[test]
    fn test_clipping_region_pdf_operations() {
        let mut region = ClippingRegion::new();

        // No clip set
        let ops = region.to_pdf_operations().unwrap();
        assert!(ops.is_none());

        // With clip set
        region.set_clip(ClippingPath::rect(0.0, 0.0, 100.0, 100.0));
        let ops = region.to_pdf_operations().unwrap();
        assert!(ops.is_some());
        assert!(ops.unwrap().contains("re"));
    }

    #[test]
    fn test_complex_clipping_path() {
        let mut path = ClippingPath::new();
        path.move_to(10.0, 10.0)
            .line_to(50.0, 10.0)
            .curve_to(60.0, 10.0, 70.0, 20.0, 70.0, 30.0)
            .line_to(70.0, 50.0)
            .close_path();

        let ops = path.to_pdf_operations().unwrap();
        assert!(ops.contains("10.000 10.000 m"));
        assert!(ops.contains("50.000 10.000 l"));
        assert!(ops.contains("c")); // Curve
        assert!(ops.contains("h")); // Close path
        assert!(ops.contains("W")); // Clip
        assert!(ops.contains("n")); // End path
    }
}
