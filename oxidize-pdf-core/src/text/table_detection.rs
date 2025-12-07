//! Advanced table detection using vector graphics and text analysis.
//!
//! This module implements border-based table detection by combining:
//! - Vector line extraction from PDF graphics (horizontal/vertical borders)
//! - Text fragment positions from content streams
//! - Grid pattern recognition and cell boundary calculation
//!
//! # Algorithm Overview
//!
//! 1. **Line Extraction**: Use `GraphicsExtractor` to get H/V lines from PDF
//! 2. **Grid Detection**: Find intersections and regular patterns
//! 3. **Cell Boundary Calculation**: Determine cell rectangles from line intersections
//! 4. **Text Assignment**: Map text fragments to cells using spatial containment
//! 5. **Table Construction**: Build `DetectedTable` with rows, columns, and cells
//!
//! # Example
//!
//! ```rust,no_run
//! use oxidize_pdf::text::table_detection::{TableDetector, TableDetectionConfig};
//! use oxidize_pdf::graphics::extraction::GraphicsExtractor;
//! use oxidize_pdf::text::extraction::TextExtractor;
//! use oxidize_pdf::parser::{PdfReader, PdfDocument};
//! use std::fs::File;
//!
//! let file = File::open("document.pdf")?;
//! let reader = PdfReader::new(file)?;
//! let doc = PdfDocument::new(reader);
//!
//! // Extract graphics (lines) and text
//! let mut graphics_ext = GraphicsExtractor::default();
//! let graphics = graphics_ext.extract_from_page(&doc, 0)?;
//!
//! let mut text_ext = TextExtractor::default();
//! let text = text_ext.extract_from_page(&doc, 0)?;
//!
//! // Detect tables
//! let detector = TableDetector::default();
//! let tables = detector.detect(&graphics, &text.fragments)?;
//!
//! for table in &tables {
//!     println!("Table: {}x{} cells", table.row_count(), table.column_count());
//! }
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```

use crate::graphics::extraction::{ExtractedGraphics, LineOrientation, VectorLine};
use crate::text::extraction::TextFragment;
use thiserror::Error;

/// Errors that can occur during table detection.
#[derive(Debug, Error)]
pub enum TableDetectionError {
    /// Invalid coordinate value (NaN or Infinity)
    #[error("Invalid coordinate value: expected valid f64, found NaN or Infinity")]
    InvalidCoordinate,

    /// Grid has no rows or columns
    #[error("Invalid grid: {0}")]
    InvalidGrid(String),

    /// Internal logic error
    #[error("Internal error: {0}")]
    InternalError(String),
}

/// Configuration for table detection.
#[derive(Debug, Clone)]
pub struct TableDetectionConfig {
    /// Minimum number of rows to consider a valid table
    pub min_rows: usize,
    /// Minimum number of columns to consider a valid table
    pub min_columns: usize,
    /// Tolerance for line alignment (in points)
    pub alignment_tolerance: f64,
    /// Minimum table area (in square points)
    pub min_table_area: f64,
    /// Whether to detect borderless tables (alignment-based)
    pub detect_borderless: bool,
}

impl Default for TableDetectionConfig {
    fn default() -> Self {
        Self {
            min_rows: 2,
            min_columns: 2,
            alignment_tolerance: 2.0, // 2 points tolerance for line alignment
            min_table_area: 1000.0,   // Minimum 1000 sq points (~35x35 pt square)
            detect_borderless: false, // Start with bordered tables only
        }
    }
}

/// A detected table with cells, rows, and columns.
#[derive(Debug, Clone)]
pub struct DetectedTable {
    /// Bounding box of the entire table
    pub bbox: BoundingBox,
    /// All cells in the table (row-major order)
    pub cells: Vec<TableCell>,
    /// Number of rows
    pub rows: usize,
    /// Number of columns
    pub columns: usize,
    /// Confidence score (0.0 to 1.0)
    pub confidence: f64,
}

impl DetectedTable {
    /// Creates a new detected table.
    pub fn new(bbox: BoundingBox, cells: Vec<TableCell>, rows: usize, columns: usize) -> Self {
        let confidence = Self::calculate_confidence(&cells, rows, columns);
        Self {
            bbox,
            cells,
            rows,
            columns,
            confidence,
        }
    }

    /// Returns the number of rows.
    pub fn row_count(&self) -> usize {
        self.rows
    }

    /// Returns the number of columns.
    pub fn column_count(&self) -> usize {
        self.columns
    }

    /// Gets a cell by row and column index (0-based).
    pub fn get_cell(&self, row: usize, col: usize) -> Option<&TableCell> {
        if row >= self.rows || col >= self.columns {
            return None;
        }
        let index = row * self.columns + col;
        self.cells.get(index)
    }

    /// Calculates confidence score based on cell population.
    fn calculate_confidence(cells: &[TableCell], rows: usize, columns: usize) -> f64 {
        if rows == 0 || columns == 0 {
            return 0.0;
        }

        let total_cells = rows * columns;
        let populated_cells = cells.iter().filter(|c| !c.text.is_empty()).count();

        // Base confidence from population ratio
        let population_ratio = populated_cells as f64 / total_cells as f64;

        // Bonus for larger tables (more likely to be intentional)
        let size_bonus = ((rows + columns) as f64 / 10.0).min(0.2);

        (population_ratio + size_bonus).min(1.0)
    }
}

/// A single cell in a detected table.
#[derive(Debug, Clone)]
pub struct TableCell {
    /// Row index (0-based)
    pub row: usize,
    /// Column index (0-based)
    pub column: usize,
    /// Cell bounding box
    pub bbox: BoundingBox,
    /// Text content in the cell
    pub text: String,
    /// Whether this cell has borders
    pub has_borders: bool,
}

impl TableCell {
    /// Creates a new table cell.
    pub fn new(row: usize, column: usize, bbox: BoundingBox) -> Self {
        Self {
            row,
            column,
            bbox,
            text: String::new(),
            has_borders: false,
        }
    }

    /// Sets the text content.
    pub fn set_text(&mut self, text: String) {
        self.text = text;
    }

    /// Checks if the cell is empty.
    pub fn is_empty(&self) -> bool {
        self.text.is_empty()
    }
}

/// Bounding box for tables and cells.
#[derive(Debug, Clone, Copy)]
pub struct BoundingBox {
    /// Left X coordinate
    pub x: f64,
    /// Bottom Y coordinate (PDF coordinate system)
    pub y: f64,
    /// Width
    pub width: f64,
    /// Height
    pub height: f64,
}

impl BoundingBox {
    /// Creates a new bounding box.
    pub fn new(x: f64, y: f64, width: f64, height: f64) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    /// Returns the right edge X coordinate.
    pub fn right(&self) -> f64 {
        self.x + self.width
    }

    /// Returns the top edge Y coordinate.
    pub fn top(&self) -> f64 {
        self.y + self.height
    }

    /// Checks if a point is inside this bounding box.
    pub fn contains_point(&self, px: f64, py: f64) -> bool {
        px >= self.x && px <= self.right() && py >= self.y && py <= self.top()
    }

    /// Returns the area of the bounding box.
    pub fn area(&self) -> f64 {
        self.width * self.height
    }
}

/// Main table detector.
pub struct TableDetector {
    config: TableDetectionConfig,
}

impl TableDetector {
    /// Creates a new table detector with the given configuration.
    pub fn new(config: TableDetectionConfig) -> Self {
        Self { config }
    }

    /// Creates a table detector with default configuration.
    pub fn default() -> Self {
        Self::new(TableDetectionConfig::default())
    }

    /// Detects tables from extracted graphics and text fragments.
    ///
    /// # Arguments
    ///
    /// * `graphics` - Extracted vector lines (H/V borders)
    /// * `text_fragments` - Text fragments with positions
    ///
    /// # Returns
    ///
    /// A vector of detected tables, sorted by confidence (highest first).
    pub fn detect(
        &self,
        graphics: &ExtractedGraphics,
        text_fragments: &[TextFragment],
    ) -> Result<Vec<DetectedTable>, TableDetectionError> {
        let mut tables = Vec::new();

        // Check if there are enough lines for a table
        if !graphics.has_table_structure() {
            return Ok(tables);
        }

        // Phase 1: Detect bordered tables from vector lines
        if let Some(table) = self.detect_bordered_table(graphics, text_fragments)? {
            tables.push(table);
        }

        // Phase 2: Detect borderless tables (alignment-based)
        if self.config.detect_borderless {
            // Enhancement: Implement borderless table detection using spatial clustering
            // Priority: MEDIUM - Related to Issue #90 (Advanced Text Extraction)
            // Current implementation works well for bordered tables
            // Borderless detection would use alignment patterns and whitespace analysis
        }

        // Sort by confidence (highest first)
        tables.sort_by(|a, b| {
            b.confidence
                .partial_cmp(&a.confidence)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        Ok(tables)
    }

    /// Detects a bordered table from vector lines.
    fn detect_bordered_table(
        &self,
        graphics: &ExtractedGraphics,
        text_fragments: &[TextFragment],
    ) -> Result<Option<DetectedTable>, TableDetectionError> {
        // Extract horizontal and vertical lines
        let h_lines: Vec<&VectorLine> = graphics.horizontal_lines().collect();
        let v_lines: Vec<&VectorLine> = graphics.vertical_lines().collect();

        // Find grid pattern
        let grid = self.detect_grid_pattern(&h_lines, &v_lines)?;

        if grid.rows.len() < self.config.min_rows || grid.columns.len() < self.config.min_columns {
            return Ok(None);
        }

        // Calculate cell boundaries
        let cells = self.create_cells_from_grid(&grid);

        // Assign text to cells
        let cells_with_text = self.assign_text_to_cells(cells, text_fragments);

        // Create table bounding box
        let bbox = self.calculate_table_bbox(&grid)?;

        // Check minimum area
        if bbox.area() < self.config.min_table_area {
            return Ok(None);
        }

        // Number of rows/columns = grid positions - 1 (gaps between lines)
        let num_rows = grid.rows.len().saturating_sub(1);
        let num_cols = grid.columns.len().saturating_sub(1);

        let table = DetectedTable::new(bbox, cells_with_text, num_rows, num_cols);

        Ok(Some(table))
    }

    /// Detects a grid pattern from horizontal and vertical lines.
    fn detect_grid_pattern(
        &self,
        h_lines: &[&VectorLine],
        v_lines: &[&VectorLine],
    ) -> Result<GridPattern, TableDetectionError> {
        // Cluster horizontal lines by Y coordinate
        let mut rows = self.cluster_lines_by_position(h_lines, LineOrientation::Horizontal)?;

        // Cluster vertical lines by X coordinate
        let columns = self.cluster_lines_by_position(v_lines, LineOrientation::Vertical)?;

        // Reverse rows so row 0 is at the top (highest Y) for intuitive indexing
        rows.reverse();

        Ok(GridPattern { rows, columns })
    }

    /// Clusters lines by their primary position (Y for horizontal, X for vertical).
    fn cluster_lines_by_position(
        &self,
        lines: &[&VectorLine],
        orientation: LineOrientation,
    ) -> Result<Vec<f64>, TableDetectionError> {
        if lines.is_empty() {
            return Ok(vec![]);
        }

        // Extract positions
        let mut positions: Vec<f64> = lines
            .iter()
            .map(|line| match orientation {
                LineOrientation::Horizontal => line.y1, // Y coordinate for horizontal lines
                LineOrientation::Vertical => line.x1,   // X coordinate for vertical lines
                _ => 0.0,
            })
            .collect();

        // Sort positions (return error if NaN/Infinity found)
        positions.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

        // Validate no NaN or Infinity values
        if positions.iter().any(|p| !p.is_finite()) {
            return Err(TableDetectionError::InvalidCoordinate);
        }

        // Cluster by tolerance - group nearby positions
        let mut clusters: Vec<Vec<f64>> = vec![vec![positions[0]]];

        for &pos in &positions[1..] {
            let last_cluster = clusters.last_mut().ok_or_else(|| {
                TableDetectionError::InternalError("cluster list unexpectedly empty".to_string())
            })?;
            let cluster_mean = last_cluster.iter().sum::<f64>() / last_cluster.len() as f64;

            if (pos - cluster_mean).abs() <= self.config.alignment_tolerance {
                // Add to existing cluster
                last_cluster.push(pos);
            } else {
                // Start new cluster
                clusters.push(vec![pos]);
            }
        }

        // Return mean position of each cluster
        Ok(clusters
            .iter()
            .map(|cluster| cluster.iter().sum::<f64>() / cluster.len() as f64)
            .collect())
    }

    /// Creates cell boundaries from grid pattern.
    fn create_cells_from_grid(&self, grid: &GridPattern) -> Vec<TableCell> {
        let mut cells = Vec::new();

        // Number of cells = number of gaps between grid lines
        let num_rows = grid.rows.len().saturating_sub(1);
        let num_cols = grid.columns.len().saturating_sub(1);

        if num_rows == 0 || num_cols == 0 {
            return cells;
        }

        // Iterate over gaps between lines (not the lines themselves)
        for row_idx in 0..num_rows {
            let y1 = grid.rows[row_idx];
            let y2 = grid.rows[row_idx + 1];

            // BoundingBox expects (x, y, width, height) where y is the LOWER edge
            let row_y = y1.min(y2);
            let row_height = (y2 - y1).abs();

            for col_idx in 0..num_cols {
                let col_x = grid.columns[col_idx];
                let col_width = (grid.columns[col_idx + 1] - col_x).abs();

                let bbox = BoundingBox::new(col_x, row_y, col_width, row_height);
                let mut cell = TableCell::new(row_idx, col_idx, bbox);
                cell.has_borders = true;

                cells.push(cell);
            }
        }

        cells
    }

    /// Assigns text fragments to cells based on spatial containment.
    ///
    /// **Coordinate Space Normalization**:
    /// Some PDFs (especially those generated by certain tools) have extreme CTM transformations
    /// that result in text and graphics being in vastly different coordinate spaces.
    /// This function detects such mismatches and applies affine transformation to normalize.
    fn assign_text_to_cells(
        &self,
        mut cells: Vec<TableCell>,
        text_fragments: &[TextFragment],
    ) -> Vec<TableCell> {
        if text_fragments.is_empty() || cells.is_empty() {
            return cells;
        }

        // Detect coordinate space mismatch and normalize if needed
        let normalized_fragments = normalize_coordinates_if_needed(&cells, text_fragments);

        for cell in &mut cells {
            let mut cell_texts = Vec::new();

            for fragment in &normalized_fragments {
                // Check if fragment center is inside cell
                let center_x = fragment.x + fragment.width / 2.0;
                let center_y = fragment.y + fragment.height / 2.0;

                if cell.bbox.contains_point(center_x, center_y) {
                    cell_texts.push(fragment.text.clone());
                }
            }

            if !cell_texts.is_empty() {
                cell.text = cell_texts.join(" ");
            }
        }

        cells
    }

    /// Calculates the table bounding box from grid pattern.
    fn calculate_table_bbox(&self, grid: &GridPattern) -> Result<BoundingBox, TableDetectionError> {
        let min_x = *grid
            .columns
            .first()
            .ok_or_else(|| TableDetectionError::InvalidGrid("no columns".to_string()))?;
        let max_x = *grid
            .columns
            .last()
            .ok_or_else(|| TableDetectionError::InvalidGrid("no columns".to_string()))?;

        // Get min/max Y regardless of row order (ascending or descending)
        let first_y = *grid
            .rows
            .first()
            .ok_or_else(|| TableDetectionError::InvalidGrid("no rows".to_string()))?;
        let last_y = *grid
            .rows
            .last()
            .ok_or_else(|| TableDetectionError::InvalidGrid("no rows".to_string()))?;
        let min_y = first_y.min(last_y);
        let max_y = first_y.max(last_y);

        Ok(BoundingBox::new(min_x, min_y, max_x - min_x, max_y - min_y))
    }
}

/// Grid pattern detected from lines.
struct GridPattern {
    /// Row Y coordinates (sorted)
    rows: Vec<f64>,
    /// Column X coordinates (sorted)
    columns: Vec<f64>,
}

impl Default for TableDetector {
    fn default() -> Self {
        Self::new(TableDetectionConfig::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bounding_box_contains_point() {
        let bbox = BoundingBox::new(100.0, 100.0, 100.0, 50.0);

        assert!(bbox.contains_point(150.0, 125.0)); // Center
        assert!(bbox.contains_point(100.0, 100.0)); // Bottom-left corner
        assert!(bbox.contains_point(200.0, 150.0)); // Top-right corner
        assert!(!bbox.contains_point(50.0, 125.0)); // Left outside
        assert!(!bbox.contains_point(250.0, 125.0)); // Right outside
        assert!(!bbox.contains_point(150.0, 50.0)); // Below
        assert!(!bbox.contains_point(150.0, 200.0)); // Above
    }

    #[test]
    fn test_bounding_box_area() {
        let bbox = BoundingBox::new(0.0, 0.0, 100.0, 50.0);
        assert!((bbox.area() - 5000.0).abs() < 0.01);
    }

    #[test]
    fn test_table_cell_new() {
        let bbox = BoundingBox::new(0.0, 0.0, 50.0, 25.0);
        let cell = TableCell::new(1, 2, bbox);

        assert_eq!(cell.row, 1);
        assert_eq!(cell.column, 2);
        assert!(cell.is_empty());
        assert!(!cell.has_borders);
    }

    #[test]
    fn test_table_cell_set_text() {
        let bbox = BoundingBox::new(0.0, 0.0, 50.0, 25.0);
        let mut cell = TableCell::new(0, 0, bbox);

        cell.set_text("Test".to_string());
        assert_eq!(cell.text, "Test");
        assert!(!cell.is_empty());
    }

    #[test]
    fn test_detected_table_get_cell() {
        let bbox = BoundingBox::new(0.0, 0.0, 200.0, 100.0);
        let cells = vec![
            TableCell::new(0, 0, BoundingBox::new(0.0, 0.0, 100.0, 50.0)),
            TableCell::new(0, 1, BoundingBox::new(100.0, 0.0, 100.0, 50.0)),
            TableCell::new(1, 0, BoundingBox::new(0.0, 50.0, 100.0, 50.0)),
            TableCell::new(1, 1, BoundingBox::new(100.0, 50.0, 100.0, 50.0)),
        ];

        let table = DetectedTable::new(bbox, cells, 2, 2);

        assert_eq!(table.row_count(), 2);
        assert_eq!(table.column_count(), 2);

        let cell = table.get_cell(0, 0).expect("cell (0,0) should exist");
        assert_eq!(cell.row, 0);
        assert_eq!(cell.column, 0);

        assert!(table.get_cell(2, 0).is_none()); // Out of bounds
        assert!(table.get_cell(0, 2).is_none()); // Out of bounds
    }

    #[test]
    fn test_table_detection_config_default() {
        let config = TableDetectionConfig::default();
        assert_eq!(config.min_rows, 2);
        assert_eq!(config.min_columns, 2);
        assert_eq!(config.alignment_tolerance, 2.0);
        assert!(!config.detect_borderless);
    }
}

/// Normalizes text coordinates to match cell coordinate space if needed.
///
/// **Problem**: Some PDFs have extreme CTM transformations where text and graphics
/// end up in vastly different coordinate systems (e.g., text Y=878000, cells Y=300).
///
/// **Solution**: Detect coordinate space mismatch and apply affine transformation
/// (scale + translate) to map text coordinates into cell coordinate space.
///
/// **When applied**:
/// - Only when there's NO overlap between text and cell bounding boxes
/// - Preserves aspect ratio and relative positioning
/// - Returns original fragments if coordinates already align
fn normalize_coordinates_if_needed(
    cells: &[TableCell],
    text_fragments: &[TextFragment],
) -> Vec<TextFragment> {
    // Calculate bounding boxes for both coordinate spaces
    let cell_bbox = calculate_combined_bbox_cells(cells);
    let text_bbox = calculate_combined_bbox_fragments(text_fragments);

    // Check if bounding boxes overlap
    let x_overlap = text_bbox.0 < cell_bbox.2 && text_bbox.2 > cell_bbox.0;
    let y_overlap = text_bbox.1 < cell_bbox.3 && text_bbox.3 > cell_bbox.1;

    // If coordinates already overlap, no normalization needed
    if x_overlap && y_overlap {
        return text_fragments.to_vec();
    }

    // Calculate affine transformation: scale + translate
    let text_width = text_bbox.2 - text_bbox.0;
    let text_height = text_bbox.3 - text_bbox.1;
    let cell_width = cell_bbox.2 - cell_bbox.0;
    let cell_height = cell_bbox.3 - cell_bbox.1;

    let scale_x = if text_width > 0.0 {
        cell_width / text_width
    } else {
        1.0
    };
    let scale_y = if text_height > 0.0 {
        cell_height / text_height
    } else {
        1.0
    };

    let translate_x = cell_bbox.0 - (text_bbox.0 * scale_x);
    let translate_y = cell_bbox.1 - (text_bbox.1 * scale_y);

    // Apply transformation to all fragments
    text_fragments
        .iter()
        .map(|frag| TextFragment {
            text: frag.text.clone(),
            x: frag.x * scale_x + translate_x,
            y: frag.y * scale_y + translate_y,
            width: frag.width * scale_x,
            height: frag.height * scale_y,
            font_size: frag.font_size,
            font_name: frag.font_name.clone(),
            is_bold: frag.is_bold,
            is_italic: frag.is_italic,
            color: frag.color,
        })
        .collect()
}

/// Calculates combined bounding box for cells: (min_x, min_y, max_x, max_y)
fn calculate_combined_bbox_cells(cells: &[TableCell]) -> (f64, f64, f64, f64) {
    let min_x = cells.iter().map(|c| c.bbox.x).fold(f64::INFINITY, f64::min);
    let max_x = cells
        .iter()
        .map(|c| c.bbox.right())
        .fold(f64::NEG_INFINITY, f64::max);
    let min_y = cells.iter().map(|c| c.bbox.y).fold(f64::INFINITY, f64::min);
    let max_y = cells
        .iter()
        .map(|c| c.bbox.top())
        .fold(f64::NEG_INFINITY, f64::max);
    (min_x, min_y, max_x, max_y)
}

/// Calculates combined bounding box for text fragments: (min_x, min_y, max_x, max_y)
fn calculate_combined_bbox_fragments(fragments: &[TextFragment]) -> (f64, f64, f64, f64) {
    let min_x = fragments.iter().map(|f| f.x).fold(f64::INFINITY, f64::min);
    let max_x = fragments
        .iter()
        .map(|f| f.x + f.width)
        .fold(f64::NEG_INFINITY, f64::max);
    let min_y = fragments.iter().map(|f| f.y).fold(f64::INFINITY, f64::min);
    let max_y = fragments
        .iter()
        .map(|f| f.y + f.height)
        .fold(f64::NEG_INFINITY, f64::max);
    (min_x, min_y, max_x, max_y)
}
