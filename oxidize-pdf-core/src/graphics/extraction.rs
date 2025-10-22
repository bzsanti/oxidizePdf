//! Vector graphics extraction for table detection.
//!
//! This module extracts vector line segments from PDF content streams,
//! which are used for detecting table borders and structure.
//!
//! # Overview
//!
//! PDF graphics are defined using path construction operators:
//! - `m` (moveto) - Start new subpath
//! - `l` (lineto) - Append straight line
//! - `re` (rectangle) - Append rectangle (decomposes to 4 lines)
//! - `h` (closepath) - Close current subpath
//!
//! Path painting operators:
//! - `S` - Stroke path
//! - `s` - Close and stroke
//! - `f` - Fill with nonzero winding rule
//! - `F` - Fill with nonzero winding (deprecated)
//! - `f*` - Fill with even-odd rule
//! - `B` - Fill and stroke (nonzero winding)
//! - `b` - Close, fill, and stroke
//!
//! # Coordinate System
//!
//! PDF uses a coordinate system where (0,0) is at the bottom-left corner.
//! The Current Transformation Matrix (CTM) transforms user space to device space.
//!
//! # Example
//!
//! ```rust,no_run
//! use oxidize_pdf::graphics::extraction::{GraphicsExtractor, ExtractionConfig};
//! use oxidize_pdf::parser::{PdfReader, PdfDocument};
//! use std::fs::File;
//!
//! let file = File::open("table.pdf")?;
//! let reader = PdfReader::new(file)?;
//! let doc = PdfDocument::new(reader);
//!
//! let config = ExtractionConfig::default();
//! let mut extractor = GraphicsExtractor::new(config);
//! let graphics = extractor.extract_from_page(&doc, 0)?;
//!
//! for line in &graphics.lines {
//!     println!("Line: ({}, {}) -> ({}, {})",
//!         line.x1, line.y1, line.x2, line.y2);
//! }
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```

use crate::parser::content::{ContentOperation, ContentParser};
use crate::parser::{PdfDocument, ParseError};
use std::fmt;

/// Orientation of a line segment.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LineOrientation {
    /// Horizontal line (y1 == y2)
    Horizontal,
    /// Vertical line (x1 == x2)
    Vertical,
    /// Diagonal line (neither horizontal nor vertical)
    Diagonal,
}

/// A vector line segment extracted from PDF graphics.
#[derive(Debug, Clone, PartialEq)]
pub struct VectorLine {
    /// Start X coordinate
    pub x1: f64,
    /// Start Y coordinate
    pub y1: f64,
    /// End X coordinate
    pub x2: f64,
    /// End Y coordinate
    pub y2: f64,
    /// Line orientation
    pub orientation: LineOrientation,
    /// Stroke width (line thickness)
    pub stroke_width: f64,
    /// Whether this line was stroked (visible)
    pub is_stroked: bool,
}

impl VectorLine {
    /// Creates a new vector line.
    ///
    /// # Arguments
    ///
    /// * `x1`, `y1` - Start coordinates
    /// * `x2`, `y2` - End coordinates
    /// * `stroke_width` - Line thickness
    /// * `is_stroked` - Whether line is visible (stroked)
    ///
    /// # Returns
    ///
    /// A new `VectorLine` with computed orientation.
    pub fn new(x1: f64, y1: f64, x2: f64, y2: f64, stroke_width: f64, is_stroked: bool) -> Self {
        let orientation = Self::compute_orientation(x1, y1, x2, y2);
        Self {
            x1,
            y1,
            x2,
            y2,
            orientation,
            stroke_width,
            is_stroked,
        }
    }

    /// Computes the orientation of a line segment.
    ///
    /// Uses a tolerance of 0.1 points to handle floating-point imprecision.
    fn compute_orientation(x1: f64, y1: f64, x2: f64, y2: f64) -> LineOrientation {
        const TOLERANCE: f64 = 0.1;

        let dx = (x2 - x1).abs();
        let dy = (y2 - y1).abs();

        if dy < TOLERANCE {
            LineOrientation::Horizontal
        } else if dx < TOLERANCE {
            LineOrientation::Vertical
        } else {
            LineOrientation::Diagonal
        }
    }

    /// Returns the length of the line segment.
    pub fn length(&self) -> f64 {
        let dx = self.x2 - self.x1;
        let dy = self.y2 - self.y1;
        (dx * dx + dy * dy).sqrt()
    }

    /// Returns the midpoint of the line segment.
    pub fn midpoint(&self) -> (f64, f64) {
        ((self.x1 + self.x2) / 2.0, (self.y1 + self.y2) / 2.0)
    }
}

/// Container for extracted graphics elements.
#[derive(Debug, Clone, Default)]
pub struct ExtractedGraphics {
    /// Extracted line segments
    pub lines: Vec<VectorLine>,
    /// Number of horizontal lines
    pub horizontal_count: usize,
    /// Number of vertical lines
    pub vertical_count: usize,
}

impl ExtractedGraphics {
    /// Creates a new empty graphics container.
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds a line segment and updates counts.
    pub fn add_line(&mut self, line: VectorLine) {
        match line.orientation {
            LineOrientation::Horizontal => self.horizontal_count += 1,
            LineOrientation::Vertical => self.vertical_count += 1,
            LineOrientation::Diagonal => {} // Don't count diagonals for tables
        }
        self.lines.push(line);
    }

    /// Returns only horizontal lines.
    pub fn horizontal_lines(&self) -> impl Iterator<Item = &VectorLine> {
        self.lines
            .iter()
            .filter(|l| l.orientation == LineOrientation::Horizontal)
    }

    /// Returns only vertical lines.
    pub fn vertical_lines(&self) -> impl Iterator<Item = &VectorLine> {
        self.lines
            .iter()
            .filter(|l| l.orientation == LineOrientation::Vertical)
    }

    /// Checks if there are enough lines for table detection.
    ///
    /// A basic table requires at least 2 horizontal and 2 vertical lines.
    pub fn has_table_structure(&self) -> bool {
        self.horizontal_count >= 2 && self.vertical_count >= 2
    }
}

/// Configuration for graphics extraction.
#[derive(Debug, Clone)]
pub struct ExtractionConfig {
    /// Minimum line length to consider (in points)
    pub min_line_length: f64,
    /// Whether to extract diagonal lines
    pub extract_diagonals: bool,
    /// Whether to extract only stroked lines
    pub stroked_only: bool,
}

impl Default for ExtractionConfig {
    fn default() -> Self {
        Self {
            min_line_length: 1.0, // Ignore very short lines
            extract_diagonals: false, // Tables use only H/V lines
            stroked_only: true, // Only visible lines
        }
    }
}

/// Graphics extractor for parsing PDF content streams.
pub struct GraphicsExtractor {
    config: ExtractionConfig,
}

impl GraphicsExtractor {
    /// Creates a new graphics extractor with the given configuration.
    pub fn new(config: ExtractionConfig) -> Self {
        Self { config }
    }

    /// Creates a graphics extractor with default configuration.
    pub fn default() -> Self {
        Self::new(ExtractionConfig::default())
    }

    /// Gets the current configuration.
    pub fn config(&self) -> &ExtractionConfig {
        &self.config
    }

    /// Extracts vector graphics from a PDF page.
    ///
    /// # Arguments
    ///
    /// * `document` - The PDF document
    /// * `page_index` - Zero-based page index
    ///
    /// # Returns
    ///
    /// An `ExtractedGraphics` containing all extracted line segments.
    ///
    /// # Errors
    ///
    /// Returns an error if the page cannot be accessed or parsed.
    pub fn extract_from_page<R: std::io::Read + std::io::Seek>(
        &mut self,
        document: &PdfDocument<R>,
        page_index: usize,
    ) -> Result<ExtractedGraphics, ExtractionError> {
        // Get page
        let page = document
            .get_page(page_index as u32)
            .map_err(|e| ExtractionError::ParseError(format!("Failed to get page: {}", e)))?;

        // Get content streams
        let streams = document
            .get_page_content_streams(&page)
            .map_err(|e| ExtractionError::ParseError(format!("Failed to get content: {}", e)))?;

        let mut graphics = ExtractedGraphics::new();
        let mut state = GraphicsState::new();

        // Process each content stream
        for stream in streams {
            let operations = ContentParser::parse(&stream)
                .map_err(|e| ExtractionError::ParseError(format!("Failed to parse content: {}", e)))?;

            self.process_operations(&operations, &mut state, &mut graphics)?;
        }

        Ok(graphics)
    }

    /// Processes a sequence of content stream operations.
    fn process_operations(
        &self,
        operations: &[ContentOperation],
        state: &mut GraphicsState,
        graphics: &mut ExtractedGraphics,
    ) -> Result<(), ExtractionError> {
        for op in operations {
            match op {
                // Graphics state management
                ContentOperation::SaveGraphicsState => state.save(),
                ContentOperation::RestoreGraphicsState => state.restore(),
                ContentOperation::SetLineWidth(w) => state.stroke_width = *w as f64,
                ContentOperation::SetTransformMatrix(a, b, c, d, e, f) => {
                    state.apply_transform(*a as f64, *b as f64, *c as f64, *d as f64, *e as f64, *f as f64);
                }

                // Path construction
                ContentOperation::MoveTo(x, y) => {
                    let (tx, ty) = state.transform_point(*x as f64, *y as f64);
                    state.move_to(tx, ty);
                }
                ContentOperation::LineTo(x, y) => {
                    let (tx, ty) = state.transform_point(*x as f64, *y as f64);
                    state.line_to(tx, ty);
                }
                ContentOperation::Rectangle(x, y, width, height) => {
                    self.extract_rectangle_lines(*x as f64, *y as f64, *width as f64, *height as f64, state, graphics);
                }
                ContentOperation::ClosePath => {
                    state.close_path();
                }

                // Path painting (triggers line extraction)
                ContentOperation::Stroke | ContentOperation::CloseStroke => {
                    self.extract_path_lines(state, graphics, true);
                    state.clear_path();
                }
                ContentOperation::Fill | ContentOperation::FillEvenOdd => {
                    if !self.config.stroked_only {
                        self.extract_path_lines(state, graphics, false);
                    }
                    state.clear_path();
                }

                _ => {} // Ignore other operators
            }
        }

        Ok(())
    }

    /// Extracts lines from a rectangle operation.
    ///
    /// Transforms all 4 corners using the current CTM to handle rotations and scaling.
    fn extract_rectangle_lines(
        &self,
        x: f64,
        y: f64,
        width: f64,
        height: f64,
        state: &GraphicsState,
        graphics: &mut ExtractedGraphics,
    ) {
        let stroke_width = state.stroke_width;

        // Transform all 4 corners
        let (x1, y1) = state.transform_point(x, y);                    // Bottom-left
        let (x2, y2) = state.transform_point(x + width, y);            // Bottom-right
        let (x3, y3) = state.transform_point(x + width, y + height);   // Top-right
        let (x4, y4) = state.transform_point(x, y + height);           // Top-left

        // Bottom edge
        graphics.add_line(VectorLine::new(x1, y1, x2, y2, stroke_width, true));

        // Right edge
        graphics.add_line(VectorLine::new(x2, y2, x3, y3, stroke_width, true));

        // Top edge
        graphics.add_line(VectorLine::new(x3, y3, x4, y4, stroke_width, true));

        // Left edge
        graphics.add_line(VectorLine::new(x4, y4, x1, y1, stroke_width, true));
    }

    /// Extracts lines from the current path.
    fn extract_path_lines(
        &self,
        state: &GraphicsState,
        graphics: &mut ExtractedGraphics,
        is_stroked: bool,
    ) {
        let stroke_width = state.stroke_width;

        for segment in &state.path {
            let PathSegment::Line { x1, y1, x2, y2 } = segment;
            let line = VectorLine::new(*x1, *y1, *x2, *y2, stroke_width, is_stroked);

            // Apply filters
            if self.config.stroked_only && !is_stroked {
                continue;
            }

            if line.length() < self.config.min_line_length {
                continue;
            }

            if !self.config.extract_diagonals && line.orientation == LineOrientation::Diagonal {
                continue;
            }

            graphics.add_line(line);
        }
    }
}

/// Graphics state for tracking PDF drawing state.
struct GraphicsState {
    /// Current transformation matrix [a, b, c, d, e, f]
    ctm: [f64; 6],
    /// Current stroke width
    stroke_width: f64,
    /// Current path being constructed
    path: Vec<PathSegment>,
    /// Current pen position
    current_point: Option<(f64, f64)>,
    /// Saved graphics states (for q/Q operators)
    state_stack: Vec<SavedState>,
}

/// Saved graphics state for q/Q operators.
#[derive(Clone)]
struct SavedState {
    ctm: [f64; 6],
    stroke_width: f64,
}

/// Path segment types.
#[derive(Debug, Clone)]
enum PathSegment {
    Line { x1: f64, y1: f64, x2: f64, y2: f64 },
}

impl GraphicsState {
    fn new() -> Self {
        Self {
            ctm: [1.0, 0.0, 0.0, 1.0, 0.0, 0.0], // Identity matrix
            stroke_width: 1.0,
            path: Vec::new(),
            current_point: None,
            state_stack: Vec::new(),
        }
    }

    fn save(&mut self) {
        self.state_stack.push(SavedState {
            ctm: self.ctm,
            stroke_width: self.stroke_width,
        });
    }

    fn restore(&mut self) {
        if let Some(saved) = self.state_stack.pop() {
            self.ctm = saved.ctm;
            self.stroke_width = saved.stroke_width;
        }
    }

    fn apply_transform(&mut self, a: f64, b: f64, c: f64, d: f64, e: f64, f: f64) {
        let [a0, b0, c0, d0, e0, f0] = self.ctm;
        self.ctm = [
            a * a0 + b * c0,
            a * b0 + b * d0,
            c * a0 + d * c0,
            c * b0 + d * d0,
            e * a0 + f * c0 + e0,
            e * b0 + f * d0 + f0,
        ];
    }

    /// Transforms a point using the current transformation matrix.
    ///
    /// Applies the CTM to convert user space coordinates to device space.
    fn transform_point(&self, x: f64, y: f64) -> (f64, f64) {
        let [a, b, c, d, e, f] = self.ctm;
        let tx = a * x + c * y + e;
        let ty = b * x + d * y + f;
        (tx, ty)
    }

    fn move_to(&mut self, x: f64, y: f64) {
        self.current_point = Some((x, y));
    }

    fn line_to(&mut self, x: f64, y: f64) {
        if let Some((x1, y1)) = self.current_point {
            self.path.push(PathSegment::Line { x1, y1, x2: x, y2: y });
            self.current_point = Some((x, y));
        }
    }

    fn close_path(&mut self) {
        // Close path by adding line from current point back to the start
        if let Some((start_x, start_y)) = self.path.first().map(|seg| match seg {
            PathSegment::Line { x1, y1, .. } => (*x1, *y1),
        }) {
            if let Some((x, y)) = self.current_point {
                // Only add closing line if current point is different from start
                const EPSILON: f64 = 0.01;
                if (x - start_x).abs() > EPSILON || (y - start_y).abs() > EPSILON {
                    self.path.push(PathSegment::Line {
                        x1: x,
                        y1: y,
                        x2: start_x,
                        y2: start_y,
                    });
                    self.current_point = Some((start_x, start_y));
                }
            }
        }
    }

    fn clear_path(&mut self) {
        self.path.clear();
        self.current_point = None;
    }
}

/// Error type for graphics extraction.
#[derive(Debug)]
pub enum ExtractionError {
    /// Invalid graphics operator
    InvalidOperator(String),
    /// Malformed operand
    InvalidOperand(String),
    /// I/O error
    IoError(std::io::Error),
    /// Parser error
    ParseError(String),
}

impl fmt::Display for ExtractionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidOperator(op) => write!(f, "Invalid graphics operator: {}", op),
            Self::InvalidOperand(msg) => write!(f, "Invalid operand: {}", msg),
            Self::IoError(e) => write!(f, "I/O error: {}", e),
            Self::ParseError(msg) => write!(f, "Parse error: {}", msg),
        }
    }
}

impl std::error::Error for ExtractionError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::IoError(e) => Some(e),
            _ => None,
        }
    }
}

impl From<std::io::Error> for ExtractionError {
    fn from(err: std::io::Error) -> Self {
        Self::IoError(err)
    }
}

impl From<ParseError> for ExtractionError {
    fn from(err: ParseError) -> Self {
        Self::ParseError(format!("{}", err))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_line_orientation_horizontal() {
        let line = VectorLine::new(100.0, 200.0, 300.0, 200.0, 1.0, true);
        assert_eq!(line.orientation, LineOrientation::Horizontal);
    }

    #[test]
    fn test_line_orientation_vertical() {
        let line = VectorLine::new(100.0, 200.0, 100.0, 400.0, 1.0, true);
        assert_eq!(line.orientation, LineOrientation::Vertical);
    }

    #[test]
    fn test_line_orientation_diagonal() {
        let line = VectorLine::new(100.0, 200.0, 300.0, 400.0, 1.0, true);
        assert_eq!(line.orientation, LineOrientation::Diagonal);
    }

    #[test]
    fn test_line_orientation_tolerance() {
        // Almost horizontal (within tolerance)
        let line = VectorLine::new(100.0, 200.0, 300.0, 200.05, 1.0, true);
        assert_eq!(line.orientation, LineOrientation::Horizontal);

        // Almost vertical (within tolerance)
        let line = VectorLine::new(100.0, 200.0, 100.05, 400.0, 1.0, true);
        assert_eq!(line.orientation, LineOrientation::Vertical);
    }

    #[test]
    fn test_line_length() {
        let line = VectorLine::new(0.0, 0.0, 3.0, 4.0, 1.0, true);
        assert!((line.length() - 5.0).abs() < 0.001); // 3-4-5 triangle
    }

    #[test]
    fn test_line_midpoint() {
        let line = VectorLine::new(100.0, 200.0, 300.0, 400.0, 1.0, true);
        let (mx, my) = line.midpoint();
        assert!((mx - 200.0).abs() < 0.001);
        assert!((my - 300.0).abs() < 0.001);
    }

    #[test]
    fn test_extracted_graphics_add_line() {
        let mut graphics = ExtractedGraphics::new();

        graphics.add_line(VectorLine::new(0.0, 0.0, 100.0, 0.0, 1.0, true)); // H
        graphics.add_line(VectorLine::new(0.0, 0.0, 0.0, 100.0, 1.0, true)); // V
        graphics.add_line(VectorLine::new(0.0, 0.0, 100.0, 100.0, 1.0, true)); // D

        assert_eq!(graphics.horizontal_count, 1);
        assert_eq!(graphics.vertical_count, 1);
        assert_eq!(graphics.lines.len(), 3);
    }

    #[test]
    fn test_extracted_graphics_iterators() {
        let mut graphics = ExtractedGraphics::new();

        graphics.add_line(VectorLine::new(0.0, 0.0, 100.0, 0.0, 1.0, true)); // H
        graphics.add_line(VectorLine::new(0.0, 0.0, 0.0, 100.0, 1.0, true)); // V
        graphics.add_line(VectorLine::new(0.0, 100.0, 100.0, 100.0, 1.0, true)); // H

        assert_eq!(graphics.horizontal_lines().count(), 2);
        assert_eq!(graphics.vertical_lines().count(), 1);
    }

    #[test]
    fn test_has_table_structure() {
        let mut graphics = ExtractedGraphics::new();

        // Not enough lines
        assert!(!graphics.has_table_structure());

        // Add 2 horizontal, 1 vertical (insufficient)
        graphics.add_line(VectorLine::new(0.0, 0.0, 100.0, 0.0, 1.0, true));
        graphics.add_line(VectorLine::new(0.0, 100.0, 100.0, 100.0, 1.0, true));
        graphics.add_line(VectorLine::new(0.0, 0.0, 0.0, 100.0, 1.0, true));
        assert!(!graphics.has_table_structure());

        // Add 2nd vertical (sufficient)
        graphics.add_line(VectorLine::new(100.0, 0.0, 100.0, 100.0, 1.0, true));
        assert!(graphics.has_table_structure());
    }

    #[test]
    fn test_extraction_config_default() {
        let config = ExtractionConfig::default();
        assert_eq!(config.min_line_length, 1.0);
        assert!(!config.extract_diagonals);
        assert!(config.stroked_only);
    }

    // CTM (Current Transformation Matrix) tests
    #[test]
    fn test_ctm_transform_point_identity() {
        let state = GraphicsState::new();
        let (tx, ty) = state.transform_point(100.0, 200.0);
        assert!((tx - 100.0).abs() < 0.001);
        assert!((ty - 200.0).abs() < 0.001);
    }

    #[test]
    fn test_ctm_transform_point_translation() {
        let mut state = GraphicsState::new();
        // Translate by (50, 75)
        state.apply_transform(1.0, 0.0, 0.0, 1.0, 50.0, 75.0);

        let (tx, ty) = state.transform_point(100.0, 200.0);
        assert!((tx - 150.0).abs() < 0.001); // 100 + 50
        assert!((ty - 275.0).abs() < 0.001); // 200 + 75
    }

    #[test]
    fn test_ctm_transform_point_scale() {
        let mut state = GraphicsState::new();
        // Scale by 2x
        state.apply_transform(2.0, 0.0, 0.0, 2.0, 0.0, 0.0);

        let (tx, ty) = state.transform_point(100.0, 200.0);
        assert!((tx - 200.0).abs() < 0.001); // 100 * 2
        assert!((ty - 400.0).abs() < 0.001); // 200 * 2
    }

    #[test]
    fn test_ctm_transform_point_combined() {
        let mut state = GraphicsState::new();
        // Scale 2x + translate (10, 20)
        state.apply_transform(2.0, 0.0, 0.0, 2.0, 10.0, 20.0);

        let (tx, ty) = state.transform_point(5.0, 5.0);
        assert!((tx - 20.0).abs() < 0.001); // 5*2 + 10
        assert!((ty - 30.0).abs() < 0.001); // 5*2 + 20
    }

    #[test]
    fn test_graphics_state_save_restore() {
        let mut state = GraphicsState::new();
        state.stroke_width = 2.0;
        state.apply_transform(2.0, 0.0, 0.0, 2.0, 10.0, 20.0);

        state.save();
        state.stroke_width = 5.0;
        state.apply_transform(1.0, 0.0, 0.0, 1.0, 50.0, 50.0);

        state.restore();
        assert_eq!(state.stroke_width, 2.0);

        // Verify CTM was restored
        let (tx, ty) = state.transform_point(5.0, 5.0);
        assert!((tx - 20.0).abs() < 0.001);
        assert!((ty - 30.0).abs() < 0.001);
    }

    #[test]
    fn test_graphics_state_nested_save_restore() {
        let mut state = GraphicsState::new();
        state.stroke_width = 2.0;

        state.save();
        state.stroke_width = 5.0;

        state.save();
        state.stroke_width = 10.0;

        state.restore();
        assert_eq!(state.stroke_width, 5.0);

        state.restore();
        assert_eq!(state.stroke_width, 2.0);

        // Restore on empty stack should be no-op
        state.restore();
        assert_eq!(state.stroke_width, 2.0);
    }

    #[test]
    fn test_close_path_creates_closing_line() {
        let mut state = GraphicsState::new();

        // Create a triangle path
        state.move_to(100.0, 100.0);
        state.line_to(200.0, 100.0);
        state.line_to(200.0, 200.0);
        state.close_path();

        // Should have 3 lines: 2 explicit + 1 from closepath
        assert_eq!(state.path.len(), 3);

        // Last line should close back to start
        if let PathSegment::Line { x1, y1, x2, y2 } = &state.path[2] {
            assert!((*x1 - 200.0).abs() < 0.01);
            assert!((*y1 - 200.0).abs() < 0.01);
            assert!((*x2 - 100.0).abs() < 0.01);
            assert!((*y2 - 100.0).abs() < 0.01);
        } else {
            panic!("Expected Line segment");
        }
    }

    #[test]
    fn test_close_path_no_duplicate_if_already_closed() {
        let mut state = GraphicsState::new();

        // Create a closed square manually
        state.move_to(100.0, 100.0);
        state.line_to(200.0, 100.0);
        state.line_to(200.0, 200.0);
        state.line_to(100.0, 200.0);
        state.line_to(100.0, 100.0); // Manually close
        state.close_path(); // Should not add duplicate

        // Should have 4 lines (not 5)
        assert_eq!(state.path.len(), 4);
    }
}
